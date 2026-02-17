import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenVesting } from "../target/types/token_vesting";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { BN } from "bn.js";
import {
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";

describe("advanced-token-vesting", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.TokenVesting as Program<TokenVesting>;

  const sender = provider.wallet as anchor.Wallet;
  let mint: PublicKey;
  let senderTokenAccount: PublicKey;
  let dataAccount: PublicKey;
  let escrowWallet: PublicKey;
  let dataBump: number;
  let escrowBump: number;

  const tokenDecimals = 6;
  const amount = new BN(1); // 1 token (multiplied inside program)
  const startTimestamp = Math.floor(Date.now() / 1000) + 60; // 1 minute from now

  before(async () => {
    mint = await createMint(
      provider.connection,
      sender.payer,
      sender.publicKey,
      null,
      tokenDecimals
    );

    const ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      sender.payer,
      mint,
      sender.publicKey
    );

    senderTokenAccount = ata.address;

    await mintTo(
      provider.connection,
      sender.payer,
      mint,
      senderTokenAccount,
      sender.publicKey,
      amount.toNumber() * 10 ** tokenDecimals
    );

    [dataAccount, dataBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("data_account"), mint.toBuffer()],
      program.programId
    );

    [escrowWallet, escrowBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow_wallet"), mint.toBuffer()],
      program.programId
    );

    await program.methods
      .initialize(dataBump, amount, tokenDecimals, new BN(startTimestamp))
      .accounts({
        dataAccount,
        escrowWallet,
        walletToWithdrawFrom: senderTokenAccount,
        tokenMint: mint,
        sender: sender.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();
  });

  it("Prevents claiming before vesting starts", async () => {
    const beneficiary = Keypair.generate();
    const newBeneficiary = {
      key: beneficiary.publicKey,
      allocatedTokens: new BN(1),
    };

    const [beneficiaryAccount, beneficiaryBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("beneficiary"), dataAccount.toBuffer(), beneficiary.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .addBeneficiaries([newBeneficiary])
      .accounts({
        dataAccount,
        tokenMint: mint,
        sender: sender.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        { pubkey: beneficiaryAccount, isSigner: false, isWritable: true },
      ])
      .rpc();

    const beneficiaryATA = (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      sender.payer,
      mint,
      beneficiary.publicKey
    )).address;

    let threw = false;
    try {
      await program.methods
        .claim(dataBump, beneficiaryBump)
        .accounts({
          dataAccount,
          beneficiaryAccount,
          escrowWallet,
          sender: beneficiary.publicKey,
          tokenMint: mint,
          walletToDepositTo: beneficiaryATA,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([beneficiary])
        .rpc();
    } catch (e) {
      threw = true;
      assert.ok(e.message.includes("Vesting period has not started yet"));
    }
    assert.isTrue(threw);
  });

  it("Allows releasing percent multiple times", async () => {
    await program.methods
      .release(dataBump, 10)
      .accounts({
        dataAccount,
        tokenMint: mint,
        sender: sender.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .release(dataBump, 20)
      .accounts({
        dataAccount,
        tokenMint: mint,
        sender: sender.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const acc = await program.account.dataAccount.fetch(dataAccount);
    assert.equal(acc.percentAvailable, 30);
  });
});
