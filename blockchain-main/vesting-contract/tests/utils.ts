import * as spl from '@solana/spl-token';
import * as anchor from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

export const createMint = async (provider: anchor.AnchorProvider, decimals: number): Promise<anchor.web3.PublicKey> => {
    const tokenMint = new anchor.web3.Keypair();
    const lamportsForMint = await provider.connection.getMinimumBalanceForRentExemption(spl.MintLayout.span);
    let tx = new anchor.web3.Transaction();
  
    // Allocate mint
    tx.add(
      anchor.web3.SystemProgram.createAccount({
        programId: spl.TOKEN_PROGRAM_ID,
        space: spl.MintLayout.span,
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        lamports: lamportsForMint,
      })
    )
    // Allocate wallet account
    tx.add(
      spl.createInitializeMintInstruction(
        tokenMint.publicKey,
        decimals,
        provider.wallet.publicKey,
        provider.wallet.publicKey,
        spl.TOKEN_PROGRAM_ID,
      )
    );
    const signature = await provider.sendAndConfirm(tx, [tokenMint]);
  
    // console.log(`[${tokenMint.publicKey}] Created new mint account at ${signature}`);
    return tokenMint.publicKey;
  }
  
export const createUserAndATA = async (provider: anchor.AnchorProvider, mint: anchor.web3.PublicKey, ): Promise<[anchor.web3.Keypair, anchor.web3.PublicKey]> => {
    // Create the User, fund with 10 SOL to be able to execute tx's
    const user = anchor.web3.Keypair.generate();
    let token_airdrop = await provider.connection.requestAirdrop(user.publicKey,
      10 * LAMPORTS_PER_SOL);
  
    const latestBlockHash = await provider.connection.getLatestBlockhash();
  
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: token_airdrop,
    });
    // console.log("airdrop successful??");
  
    // Find ATA for the User
    let userATA = await spl.getAssociatedTokenAddress(
      mint,
      user.publicKey,
      false,
      spl.TOKEN_PROGRAM_ID,
      spl.ASSOCIATED_TOKEN_PROGRAM_ID
    );
  
    // console.log("userata finding successful");
    return [user, userATA];
  
  }
export const fundATA = async (provider: anchor.AnchorProvider, mint: anchor.web3.PublicKey, user: anchor.web3.Keypair, userATA: anchor.web3.PublicKey, decimals: number): Promise<anchor.web3.PublicKey> => {
    // Create TX to mint tokens to the User
    const txFundATA = new anchor.web3.Transaction();
  
    txFundATA.add(
      spl.createAssociatedTokenAccountInstruction(
        user.publicKey,
        userATA,
        user.publicKey,
        mint,
        spl.TOKEN_PROGRAM_ID,
        spl.ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
  
    txFundATA.add(
      spl.createMintToInstruction(
        mint,
        userATA,
        provider.wallet.publicKey,
        2000 * 10 ** decimals,
        // 2000000000,
        [],
        spl.TOKEN_PROGRAM_ID
      )
    );
  
    const txFundToken = await provider.sendAndConfirm(txFundATA, [user]);
    return userATA;
  }
  
export async function getTokenBalanceWeb3(tokenAccount: anchor.web3.PublicKey, provider: anchor.AnchorProvider): Promise<number> {
    let info = await provider.connection.getTokenAccountBalance(tokenAccount);
    if (!info.value.uiAmount) throw new Error('No balance found');
    return info.value.uiAmount;
  }
  
export async function createPDA(seeds: Buffer[], programId: anchor.web3.PublicKey): Promise<[anchor.web3.PublicKey, number]>{
    let [PDA, bump] = await anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      programId
    );
    return [PDA, bump];
  }
  