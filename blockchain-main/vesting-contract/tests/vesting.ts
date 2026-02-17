// tests/vesting.ts

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenVesting } from "../target/types/token_vesting";
import {
  createMint,
  getAssociatedTokenAddress,
  mintTo,
  getAccount,
  createAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("token_vesting full suite", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenVesting as Program<TokenVesting>;
  const payer = provider.wallet;

  let tokenMint: anchor.web3.PublicKey;
  let senderAta: anchor.web3.PublicKey;
  let escrowWallet: anchor.web3.PublicKey;
  let dataAccount: anchor.web3.PublicKey;
  let dataBump: number;
  let escrowBump: number;
  let beneficiaryKeypair: anchor.web3.Keypair;
  let beneficiaryPda: anchor.web3.PublicKey;
  let beneficiaryBump: number;

  const vestingAmount = 1000;
  const tokenDecimals = 6;
  const SECONDS_IN_MONTH = 30 * 24 * 60 * 60;
  const pastTimestamp = Math.floor(Date.now() / 1000) - 36 * SECONDS_IN_MONTH;

  function getPDAs(tokenMint: anchor.web3.PublicKey) {
    const [dataAccount, dataBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("data_account"), tokenMint.toBuffer()],
      program.programId
    );
    const [escrowWallet, escrowBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow_wallet"), tokenMint.toBuffer()],
      program.programId
    );
    return { dataAccount, dataBump, escrowWallet, escrowBump };
  }

  it("Initializes vesting contract", async () => {
    tokenMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      tokenDecimals
    );

    senderAta = await getAssociatedTokenAddress(tokenMint, payer.publicKey);
    await createAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      tokenMint,
      payer.publicKey
    );
    await mintTo(
      provider.connection,
      payer.payer,
      tokenMint,
      senderAta,
      payer.payer,
      vestingAmount * 10 ** tokenDecimals
    );

    const pda = getPDAs(tokenMint);
    dataAccount = pda.dataAccount;
    dataBump = pda.dataBump;
    escrowWallet = pda.escrowWallet;
    escrowBump = pda.escrowBump;

    await program.methods
      .initialize(dataBump, new anchor.BN(vestingAmount), tokenDecimals, new anchor.BN(pastTimestamp))
      .accounts({
        dataAccount,
        escrowWallet,
        walletToWithdrawFrom: senderAta,
        tokenMint,
        sender: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();
  });

  it("Initializes data account and adds a beneficiary", async () => {
    beneficiaryKeypair = anchor.web3.Keypair.generate();

    [beneficiaryPda, beneficiaryBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("beneficiary"),
        dataAccount.toBuffer(),
        beneficiaryKeypair.publicKey.toBuffer(),
      ],
      program.programId
    );

    await program.methods
      .addBeneficiaries([
        {
          key: beneficiaryKeypair.publicKey,
          allocatedTokens: new anchor.BN(100),
        },
      ])
      .accounts({
        dataAccount,
        sender: payer.publicKey,
        tokenMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .remainingAccounts([
        {
          pubkey: beneficiaryPda,
          isSigner: false,
          isWritable: true,
        },
      ])
      .rpc();

    const acc = await program.account.beneficiaryAccount.fetch(beneficiaryPda);
    assert.ok(acc.key.equals(beneficiaryKeypair.publicKey));
    assert.ok(acc.allocatedTokens.eq(new anchor.BN(100)));
    assert.ok(acc.claimedTokens.eq(new anchor.BN(0)));
  });

  it("Releases 100% of tokens manually", async () => {
    await program.methods
      .release(dataBump, 100)
      .accounts({
        dataAccount,
        tokenMint,
        sender: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const data = await program.account.dataAccount.fetch(dataAccount);
    assert.equal(data.percentAvailable, 100);
  });

  it("Allows beneficiary to claim available tokens", async () => {
    await provider.connection.requestAirdrop(beneficiaryKeypair.publicKey, 1 * anchor.web3.LAMPORTS_PER_SOL);
    await new Promise(resolve => setTimeout(resolve, 1000));

    const beneficiaryAta = await getAssociatedTokenAddress(tokenMint, beneficiaryKeypair.publicKey);

    await program.methods
      .claim(dataBump, beneficiaryBump)
      .accounts({
        dataAccount,
        beneficiaryAccount: beneficiaryPda,
        escrowWallet,
        sender: beneficiaryKeypair.publicKey,
        tokenMint,
        walletToDepositTo: beneficiaryAta,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([beneficiaryKeypair])
      .rpc();

    const balance = await getAccount(provider.connection, beneficiaryAta);
    assert.isAbove(Number(balance.amount), 0);
  });

  it("Withdraws unclaimed tokens after full vesting", async () => {
    const recipientAta = await getAssociatedTokenAddress(tokenMint, payer.publicKey);

    await program.methods
      .withdrawUnclaimed(dataBump, escrowBump)
      .accounts({
        dataAccount,
        escrowWallet,
        tokenMint,
        recipient: recipientAta,
        sender: payer.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const state = await program.account.dataAccount.fetch(dataAccount);
    assert.ok(state.unclaimedWithdrawn.gt(new anchor.BN(0)));
  });
});
