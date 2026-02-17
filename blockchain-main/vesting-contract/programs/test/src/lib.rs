// Import all necessary macros and types from Anchor, including common types like `Context`, `Program`, and attributes like `#[program]`.
use anchor_lang::prelude::*;
// Import SPL Token program interfaces and helper functions:
// - `Mint` represents a token mint (e.g., USDC).
// - `TokenAccount` represents a user's or program's token holding account.
// - `Transfer` is the instruction context for token transfers.
// - `token` provides utility functions like `token::transfer`.
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Import the Associated Token Account interface.
// Used to create or interact with associated token accounts (one per token per wallet).
use anchor_spl::associated_token::AssociatedToken;
// Import `invoke_signed`, which allows programs to make Cross-Program Invocations (CPI) while using PDA signers.
use anchor_lang::solana_program::program::invoke_signed;
// Import Solana's native system instructions (e.g., `create_account`, `transfer` for SOL).
// Useful for operations involving SOL rather than SPL tokens.
use anchor_lang::solana_program::system_instruction;


// Declare the unique program ID for your smart contract on Solana.
// This must match the program ID used when deploying the program with Solana CLI or Anchor.
declare_id!("7V64h32PJnSF9L83FryWCaTf4MuvxFghueo7GwMszmzS");
// The main module for your Anchor program.
// All public functions inside this module are program entrypoints callable from clients.
#[program]
pub mod token_vesting {
    use super::*;
// Initializes the vesting contract with specified parameters.
    // This function sets up the initial data (amount, decimals, start time) in a PDA account.
    //
    // Arguments:
    // - `ctx`: The context includes all required accounts passed in from the client.
    // - `_data_bump`: The bump used to derive the PDA for the data account (usually for signer verification).
    // - `amount`: The total amount of tokens to be vested.
    // - `decimals`: Token precision (usually 6 or 9 for SPL tokens).
    // - `start_timestamp`: The UNIX timestamp at which vesting should begin.    

    
    

pub fn initialize(
    ctx: Context<Initialize>,
    _data_bump: u8,
    amount: u64,
    decimals: u8,
    start_timestamp: i64, // NEW ARG
) -> Result<()> {
    
    // Function logic goes here...
    // Get a mutable reference to the data account (PDA) where vesting configuration will be stored.
       let data_account = &mut ctx.accounts.data_account;
    // Ensure the vesting amount is greater than zero.
// If not, throw a custom error `VestingError::ZeroVestingAmount`.

        require!(amount > 0, VestingError::ZeroVestingAmount);
    // Initialize vesting state variables in the data account:
    // No tokens are available to claim initially; vesting will unlock over time.

        data_account.percent_available = 0;
    // Store the total token amount to be vested.
        data_account.token_amount = amount;
     // Store token precision (e.g., 6 or 9 for SPL tokens).
        data_account.decimals = decimals;
     // Save the initializer's public key (i.e., the user who called `initialize`).
        data_account.initializer = ctx.accounts.sender.key();
     // Save the public key of the escrow wallet where tokens are held.
        data_account.escrow_wallet = ctx.accounts.escrow_wallet.key();
    // Store the token mint address (i.e., the type of SPL token being vested).
        data_account.token_mint = ctx.accounts.token_mint.key();
     // Set the vesting period to 36 months (3 years).
        data_account.vesting_months = 36;
     // Record the UNIX timestamp when vesting should start.
        data_account.start_timestamp = start_timestamp;

    // Create a new SPL token `Transfer` instruction context.
// This struct tells the Anchor SPL Token CPI which accounts to use for the transfer:
//
// - `from`: The token account from which tokens will be withdrawn.
// - `to`: The escrow wallet token account where tokens will be deposited.
// - `authority`: The signer/owner of the `from` token account (must approve the transfer).
        let transfer_instruction = Transfer {
            from: ctx.accounts.wallet_to_withdraw_from.to_account_info(), // Source token account
            to: ctx.accounts.escrow_wallet.to_account_info(),  // Destination escrow token account
            authority: ctx.accounts.sender.to_account_info(), // Owner/signer of the source account
        
        };
    // Create a new Cross-Program Invocation (CPI) context for the SPL Token `transfer` instruction.
//
// This context tells Anchor how to perform the token transfer by specifying:
// - The SPL Token program to invoke (`token_program`).
// - The previously defined `transfer_instruction` which includes `from`, `to`, and `authority`.
//
// This context is later passed to `token::transfer(...)` to execute the actual transfer.

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),// SPL Token Program account
            transfer_instruction,  // Accounts required for the transfer
        );

      // Execute the SPL token transfer using the previously created CPI context.
//
// The transfer amount is calculated as:
// `data_account.token_amount * 10^decimals`
// This converts the human-readable token amount into base units (smallest denomination of the token),
// e.g., for 6 decimals, 1 token = 1_000_000 base units.
//
// This call will transfer the full vesting amount from the sender's token account to the escrow wallet.

        token::transfer(cpi_ctx, data_account.token_amount * 10u64.pow(decimals as u32))?;

        Ok(())
    }
     // Public instruction to release a certain percentage of the vested tokens.
// This function increases the `percent_available` in the `data_account`,
// making that portion of tokens claimable by the beneficiary.

    pub fn release(ctx: Context<Release>, _data_bump: u8, percent: u8) -> Result<()> {
          // Get mutable access to the on-chain data account storing vesting state.
        let data_account = &mut ctx.accounts.data_account;
          // Ensure that the requested percentage is not more than 100%.
        require!(percent <= 100, VestingError::InvalidPercentage);
         // Increase the `percent_available` by the given `percent`,
    // but cap the result at a maximum of 100% to prevent over-release.
    //
    // `saturating_add` prevents overflow.
    // `std::cmp::min` ensures the cap at 100.

        data_account.percent_available = std::cmp::min(
            data_account.percent_available.saturating_add(percent),
            100,
        );
        // Successfully complete the instruction.

        Ok(())
    }
     // Public instruction to allow a beneficiary to claim their vested tokens.
//
// This function will transfer the currently claimable portion of tokens
// from the escrow wallet to the beneficiary's associated token account (ATA).

    pub fn claim(ctx: Context<Claim>, data_bump: u8, _beneficiary_bump: u8) -> Result<()> {
         // Get a reference to the signer account (beneficiary trying to claim tokens).
        let sender = &ctx.accounts.sender;
         // Get a reference to the escrow wallet holding the vested tokens.
        let escrow_wallet = &ctx.accounts.escrow_wallet;
        // Get a mutable reference to the main vesting state account (PDA).
        let data_account = &mut ctx.accounts.data_account;
         // Get a reference to the SPL Token program account, needed for CPI.
        let token_program = &ctx.accounts.token_program;
         // Get the public key of the token mint used for vesting (e.g., USDC, custom SPL token).
        let token_mint_key = ctx.accounts.token_mint.key();
         // Get the associated token account (ATA) of the beneficiary — this is where tokens will be sent.
        let beneficiary_ata = &ctx.accounts.wallet_to_deposit_to;
        // Get the decimals (precision) used by the token (e.g., 6 decimals means 1 token = 1_000_000 units).
        let decimals = data_account.decimals;
 // Get a mutable reference to the beneficiary's vesting tracking account.
        let beneficiary = &mut ctx.accounts.beneficiary_account;
        
        // Ensure the sender is the actual beneficiary by comparing public keys.
      // If they don't match, return a custom error: `BeneficiaryNotFound`
        require_keys_eq!(beneficiary.key, sender.key(), VestingError::BeneficiaryNotFound);
         // Get the current on-chain UNIX timestamp from the Solana clock sysvar.
        let now = Clock::get()?.unix_timestamp;
         // Check that the vesting has started.
// If current time is before the `start_timestamp`, throw `VestingNotStarted` error.
        require!(now >= data_account.start_timestamp, VestingError::VestingNotStarted);
// Calculate how many seconds have passed since vesting started.
        let elapsed_seconds = now - data_account.start_timestamp;
         // Convert elapsed seconds into months.
// Assumes 1 month = 30 days = 30 * 24 * 60 * 60 seconds.

        let elapsed_months = elapsed_seconds / (30 * 24 * 60 * 60);
        
        // Compute the percentage of the vesting period that has passed.
// Formula: (elapsed_months * 100) / total vesting months
// Clamp the result at 100% to prevent overflow.

// Calculate the percentage of tokens that should be unlocked based on elapsed time.
//
// Formula:
// (elapsed_months * 100) / total_vesting_months
//
// This gives a linear vesting percentage (e.g., 50% after 18 months of a 36-month vesting).
// `std::cmp::min(..., 100)` ensures the value never exceeds 100%, even if extra time has passed.
// The result is cast to `u8` since percentages are stored as 0–100.

        let time_vested_percent = std::cmp::min(
            (elapsed_months as u64 * 100) / data_account.vesting_months as u64,
            100,
        ) as u8;
        // Determine the effective claimable percentage for the beneficiary.
//
// Take the lesser of:
// - `time_vested_percent`: how much has vested over time
// - `data_account.percent_available`: how much has been manually released (e.g., via `release()`)
// This ensures both time-based and manual vesting constraints are respected.

        let effective_claim_percent = std::cmp::min(time_vested_percent, data_account.percent_available);
          // Calculate the total number of tokens the beneficiary is eligible to claim at this point.
// Formula:
// (allocated_tokens * effective_percent) / 100

        let total_eligible = (beneficiary.allocated_tokens * effective_claim_percent as u64) / 100;
        // Calculate the remaining claimable amount by subtracting already claimed tokens.
// `saturating_sub` ensures the result is not negative (prevents underflow).
        let claimable_amount = total_eligible.saturating_sub(beneficiary.claimed_tokens);
         // Prepare the signer seeds for invoking CPI as the data_account PDA.
       // Seeds used to generate the PDA:
// - "data_account": a static string prefix
// - token_mint_key: identifies the specific vesting mint
// - data_bump: bump used in PDA derivation

        let seeds = &[b"data_account", token_mint_key.as_ref(), &[data_bump]];
          // Wrap the seeds in the required nested format for CPI signer support.
        let signer_seeds = &[&seeds[..]];

         // Set up the SPL Token `Transfer` instruction to move claimable tokens from the escrow to the beneficiary.
//
// This instruction defines the required accounts:
// - `from`: The program's escrow wallet holding the vested tokens.
// - `to`: The beneficiary's associated token account (where tokens will be received).
// - `authority`: The signer of the transfer — in this case, the `data_account` PDA,
//                which must sign the transaction using `signer_seeds` and `with_signer`.


        let transfer_instruction = Transfer {
            from: escrow_wallet.to_account_info(), // Source: escrow holding vested tokens
            to: beneficiary_ata.to_account_info(), // Destination: beneficiary's token account
            authority: data_account.to_account_info(), // PDA that authorizes the transfer
        };

        // Create a CPI (Cross-Program Invocation) context for the token transfer,
// allowing the program to sign on behalf of a PDA (`data_account`) using `signer_seeds`.
//
// This is required because the escrow wallet is controlled by a PDA, not a regular user,
// and thus needs to be signed using its derived seeds.
//
// Parameters:
// - `token_program`: The SPL Token program to invoke.
// - `transfer_instruction`: Contains `from`, `to`, and `authority` accounts for the transfer.
// - `signer_seeds`: Seeds used to regenerate the PDA that acts as the signer (i.e., `data_account`).


        let cpi_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(), // The SPL Token program account
            transfer_instruction,  // The transfer instruction with source, destination, and PDA authority
            signer_seeds,  // Seeds needed for PDA signing
        );
 // Convert the human-readable token amount to raw amount by applying the token's decimal places
        let amount_to_transfer_raw = claimable_amount * 10u64.pow(decimals as u32);
         // Ensure that the effective claim percentage is greater than 0 before proceeding

        require!(effective_claim_percent > 0, VestingError::ClaimNotAllowed);
         // Perform the actual token transfer from escrow to the beneficiary
        token::transfer(cpi_ctx, amount_to_transfer_raw)?;
         // Update the beneficiary's claimed amount (in base units)

        beneficiary.claimed_tokens = beneficiary.claimed_tokens.saturating_add(claimable_amount);
        // Update the total claimed amount in the data account (in base units)
        data_account.claimed_total = data_account.claimed_total.saturating_add(claimable_amount);
        


        Ok(())
    }

    pub fn withdraw_unclaimed(ctx: Context<WithdrawUnclaimed>, data_bump: u8, _escrow_bump: u8) -> Result<()> {
         // Get mutable reference to the main vesting data account
        let data_account = &mut ctx.accounts.data_account;
         // Get the current on-chain timestamp
        let now = Clock::get()?.unix_timestamp;
        // Calculate the number of seconds since vesting started
        let elapsed_seconds = now - data_account.start_timestamp;
        // Calculate total vesting duration in seconds (assuming 30-day months)
        let vesting_duration = (data_account.vesting_months as i64) * 30 * 24 * 60 * 60;
        // Ensure vesting period has fully elapsed before allowing withdrawal
        require!(elapsed_seconds >= vesting_duration, VestingError::VestingStillActive);
         // Read total claimed and total vested amounts

        let total_claimed = data_account.claimed_total;
        let total_vested_amount = data_account.token_amount;
        // Calculate how much unclaimed amount remains after deducting claimed and previously withdrawn unclaimed tokens
        let unclaimed = total_vested_amount.saturating_sub(total_claimed + data_account.unclaimed_withdrawn);
        // Ensure there is something to withdraw
        require!(unclaimed > 0, VestingError::NoUnclaimedTokens);

         // Prepare signer seeds for PDA authority
        let token_mint_key = ctx.accounts.token_mint.key();
        let seeds = &[b"data_account", token_mint_key.as_ref(), &[data_bump]];
        let signer_seeds = &[&seeds[..]];

        // Prepare transfer instruction from the escrow wallet to the recipient
        let transfer_instruction = Transfer {
            from: ctx.accounts.escrow_wallet.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
            authority: data_account.to_account_info(),
        };

        // Build CPI context with signer seeds
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds,
        );

        // Calculate amount to withdraw in raw units (based on token decimals)
        let amount_to_withdraw = unclaimed * 10u64.pow(data_account.decimals as u32);
        // Perform token transfer from escrow to recipient
        token::transfer(cpi_ctx, amount_to_withdraw)?;
        // Update the amount of unclaimed tokens that have been withdrawn
        data_account.unclaimed_withdrawn += unclaimed;
        Ok(())
    }

    pub fn cancel_vesting(
    ctx: Context<CancelVesting>,
    data_bump: u8,
    escrow_bump: u8,
) -> Result<()> {
        // Get a mutable reference to the main vesting data account
    let data_account = &mut ctx.accounts.data_account;
         // Get the current on-chain timestamp
    let now = Clock::get()?.unix_timestamp;
// Ensure vesting is still active (i.e., has not yet fully completed)
    require!(now < data_account.start_timestamp + (data_account.vesting_months as i64) * 30 * 24 * 60 * 60, VestingError::VestingAlreadyCompleted);
        
// Total tokens allocated for vesting
    let total_allocated = data_account.token_amount;
        // Total tokens claimed by all beneficiaries so far
    let total_claimed = data_account.claimed_total;
        // Calculate unclaimed tokens still in escrow (excluding previously withdrawn unclaimed tokens)
    let unclaimed = total_allocated
        .saturating_sub(total_claimed + data_account.unclaimed_withdrawn);
// Ensure there are still unclaimed tokens available for transfer
    require!(unclaimed > 0, VestingError::NoUnclaimedTokens);

    // Derive the signer PDA seeds for signing the token transfer
    let token_mint_key = ctx.accounts.token_mint.key();
    let seeds = &[b"data_account", token_mint_key.as_ref(), &[data_bump]];
    let signer_seeds = &[&seeds[..]];

     // Create a transfer instruction to move tokens from the program's escrow wallet to the recipient's account   
    let transfer_instruction = Transfer {
        from: ctx.accounts.escrow_wallet.to_account_info(), // Source escrow token account
        to: ctx.accounts.recipient.to_account_info(),      // Destination recipient token account
        authority: data_account.to_account_info(),     // PDA authority that signs the transfer
    };
        
 // Create a CPI (Cross-Program Invocation) context with signer seeds
// This context is used to authorize the token transfer using the program-derived address (PDA) as the signer
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(), // SPL Token program being invoked
        transfer_instruction,      // Transfer instruction created earlier
        signer_seeds,      // PDA seeds used to sign the CPI on behalf of the program
    );
// Calculate the actual token amount to transfer by scaling `unclaimed` with the token's decimal precision
    let amount = unclaimed * 10u64.pow(data_account.decimals as u32);
// Perform the token transfer from the escrow wallet to the recipient using the CPI context
    token::transfer(cpi_ctx, amount)?;

    data_account.unclaimed_withdrawn += unclaimed;
    data_account.percent_available = 100; // Optional: to prevent further release
    data_account.vesting_months = 0;      // Effectively ends vesting

    Ok(())
}

pub fn add_beneficiaries<'info>(
    ctx: Context<'_, '_, '_, 'info, AddBeneficiaries<'info>>,
    new_beneficiaries: Vec<NewBeneficiary>,
) -> Result<()> {
     // Get the current program ID, data account, and payer (usually the authority/owner)
    let program_id = ctx.program_id;
    let data_account = &ctx.accounts.data_account;
    let payer = &ctx.accounts.sender;
    // Iterator over remaining accounts (used to receive dynamically generated PDAs for beneficiaries)
    let mut remaining = ctx.remaining_accounts.iter();

     // Loop through each new beneficiary to add
    for new in new_beneficiaries {
        let beneficiary_pubkey = new.key;
        let allocated_tokens = new.allocated_tokens;

        let beneficiary_account_info = remaining
            .next()
            .ok_or(VestingError::MissingRemainingAccount)?;
// Seeds used to generate the PDA for the beneficiary
        let data_account_key = data_account.key();
        let beneficiary_seeds = &[
            b"beneficiary",
            data_account_key.as_ref(),
            beneficiary_pubkey.as_ref(),
        ];
// Derive the PDA and its bump for the beneficiary
        let (beneficiary_pda, bump) =
            Pubkey::find_program_address(beneficiary_seeds, program_id);
         // Ensure the beneficiary account is still owned by the System Program (i.e., not yet initialized)
        require!(
            beneficiary_account_info.owner == &System::id(),
            VestingError::BeneficiaryAlreadyExists
        );

        // Skip creation if already initialized
        if beneficiary_account_info.owner == &System::id() {
            let rent = Rent::get()?;
            let space = std::mem::size_of::<BeneficiaryAccount>() + 8; // add discriminator
            let lamports = rent.minimum_balance(space);

            invoke_signed(
                &system_instruction::create_account(
                    payer.key,
                    &beneficiary_pda,
                    lamports,
                    space as u64,
                    program_id,
                ),
                &[
                    payer.to_account_info(),
                    beneficiary_account_info.clone(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[&[
                    b"beneficiary",
                    data_account_key.as_ref(),
                    beneficiary_pubkey.as_ref(),
                    &[bump],
                ]],
            )?;

            let mut account_data = BeneficiaryAccount {
                key: beneficiary_pubkey,
                allocated_tokens,
                claimed_tokens: 0,
            };
            account_data
                .try_serialize(&mut &mut beneficiary_account_info.data.borrow_mut()[..])?;
        }
    }

    Ok(())
}

    //Removes a list of beneficiary accounts from the vesting program.
///
/// # Arguments
/// * `ctx` - The execution context containing all the necessary accounts.
/// * `data_bump` - The bump seed for the data_account PDA.
/// * `keys` - A vector of public keys representing the beneficiaries to be removed.
///
/// The function starts by:
/// - Fetching the `program_id`, `data_account_key`, and `initializer` (the sender authorized to modify beneficiaries).
/// - Preparing an iterator over `remaining_accounts`, which may be used to dynamically validate or deserialize the accounts
///    corresponding to the beneficiary keys being removed.
///
/// Note: Actual logic for checking authority, validating keys, and updating state




pub fn remove_beneficiaries(
    ctx: Context<RemoveBeneficiaries>,
    data_bump: u8,
    keys: Vec<Pubkey>,
) -> Result<()> {
    let program_id = ctx.program_id;
    let data_account_key = ctx.accounts.data_account.key();
    let initializer = &ctx.accounts.sender;
    let mut remaining = ctx.remaining_accounts.iter();

    for key in keys {
        let beneficiary_info = remaining
            .next()
            .ok_or(VestingError::MissingRemainingAccount)?;

        // Derive the expected PDA for the beneficiary
        let seeds = &[b"beneficiary", data_account_key.as_ref(), key.as_ref()];
        let (expected_pda, bump) = Pubkey::find_program_address(seeds, program_id);

        require_keys_eq!(beneficiary_info.key(), expected_pda, VestingError::InvalidBeneficiaryPDA);

        // Close the account, refunding lamports to initializer
        **initializer.to_account_info().try_borrow_mut_lamports()? += beneficiary_info.lamports();
        **beneficiary_info.try_borrow_mut_lamports()? = 0;
        let mut data = beneficiary_info.try_borrow_mut_data()?;
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }

    Ok(())
}


}

/// Accounts required to initialize the vesting contract.
///
/// This instruction creates and initializes two PDA accounts:
/// 1. `data_account` - A PDA that stores metadata/configuration for the vesting logic.
/// 2. `escrow_wallet` - A PDA token account that will hold the escrowed SPL tokens to be vested.
/// The main data account storing vesting configuration.
    /// 
    /// Seeds: ["data_account", token_mint.key()]
    /// Bump: Auto-calculated
    /// Space: Enough to store the serialized `DataAccount` structure:
    /// - 8   (discriminator)
    /// - 1   (is_initialized: bool)
    /// - 8   (start_time: i64)
    /// - 32  (token_mint: Pubkey)
    /// - 32  (admin: Pubkey)
    /// - 32  (creator: Pubkey)
    /// - 1   (has_cliff: bool)
    /// - 8   (cliff_duration: i64)
    /// - 1   (is_cancellable: bool)
    /// - 8   (total_duration: i64)
    /// - 8   (created_at: i64)

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = sender,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump,
        space = 8 + 1 + 8 + 32 + 32 + 32 + 1 + 8 + 1 + 8 + 8
    )]
    pub data_account: Account<'info, DataAccount>,

/// The escrow wallet PDA that holds SPL tokens for vesting.
    ///
    /// Seeds: ["escrow_wallet", token_mint.key()]
    /// Authority: The `data_account` PDA
    /// Token Mint: Must match the `token_mint` passed into the instruction
    
    #[account(
        init,
        payer = sender,
        seeds = [b"escrow_wallet", token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = data_account
    )]
    pub escrow_wallet: Account<'info, TokenAccount>,

    #[account(mut)]
    pub wallet_to_withdraw_from: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(data_bump: u8, beneficiary_bump: u8)]
pub struct Claim<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
    )]
    pub data_account: Account<'info, DataAccount>,

    #[account(
        mut,
        seeds = [b"beneficiary", data_account.key().as_ref(), sender.key().as_ref()],
        bump = beneficiary_bump,
    )]
    pub beneficiary_account: Account<'info, BeneficiaryAccount>,

    #[account(mut)]
    pub escrow_wallet: Account<'info, TokenAccount>,

    #[account(mut)]
    pub sender: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = sender,
        associated_token::mint = token_mint,
        associated_token::authority = sender,
    )]
    pub wallet_to_deposit_to: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_bump: u8)]
pub struct Release<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender
    )]
    pub data_account: Account<'info, DataAccount>,

    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_bump: u8)]
pub struct ModifyBeneficiaries<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender
    )]
    pub data_account: Account<'info, DataAccount>,

    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sender: Signer<'info>,
}

#[derive(Accounts)]
#[instruction()]

#[account(init, seeds = [...], bump, payer = sender, space = ...)]

pub struct AddBeneficiaries<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender,
    )]
    pub data_account: Account<'info, DataAccount>,

    #[account(mut)]
    pub sender: Signer<'info>,

    pub token_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    // BeneficiaryAccount PDAs will be passed dynamically via remaining_accounts
}

#[derive(Accounts)]
#[instruction(data_bump: u8, escrow_bump: u8)]
pub struct WithdrawUnclaimed<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender,
    )]
    pub data_account: Account<'info, DataAccount>,

    #[account(
        mut,
        seeds = [b"escrow_wallet", token_mint.key().as_ref()],
        bump = escrow_bump,
    )]
    pub escrow_wallet: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,

    #[account(mut)]
    pub sender: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct DataAccount {
    pub percent_available: u8,
    pub token_amount: u64,
    pub initializer: Pubkey,
    pub escrow_wallet: Pubkey,
    pub token_mint: Pubkey,
    pub decimals: u8,
    pub start_timestamp: i64,
    pub vesting_months: u8,
    pub claimed_total: u64,
    pub unclaimed_withdrawn: u64,
}

#[account]
#[derive(Default)]
pub struct BeneficiaryAccount {
    pub key: Pubkey,
    pub allocated_tokens: u64,
    pub claimed_tokens: u64,
}

#[error_code]
pub enum VestingError {
    #[msg("Sender is not owner of Data Account")]
    InvalidSender,
    #[msg("Not allowed to claim new tokens currently")]
    ClaimNotAllowed,
    #[msg("Beneficiary does not exist in account")]
    BeneficiaryNotFound,
    #[msg("Vesting period has not started yet")]
    VestingNotStarted,
    #[msg("Invalid percentage provided (must be between 0 and 100)")]
    InvalidPercentage,
    #[msg("Total vesting amount must be greater than 0")]
    ZeroVestingAmount,
    #[msg("Unclaimed tokens are not yet withdrawable")]
    VestingStillActive,
    #[msg("No unclaimed tokens available for withdrawal")]
    NoUnclaimedTokens,
    #[msg("Missing account in remaining_accounts")]
MissingRemainingAccount,
#[msg("Provided account does not match expected beneficiary PDA")]
InvalidBeneficiaryPDA,
#[msg("Provided account does not match expected beneficiary PDA")]
BeneficiaryAlreadyExists,
#[msg("Vesting already completed, cannot cancel")]
VestingAlreadyCompleted,

}
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct NewBeneficiary {
    pub key: Pubkey,
    pub allocated_tokens: u64,
}
#[derive(Accounts)]
#[instruction(data_bump: u8)]
pub struct RemoveBeneficiaries<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender,
    )]
    pub data_account: Account<'info, DataAccount>,

    pub token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,

    // Pass each BeneficiaryAccount in remaining_accounts[]
}
#[derive(Accounts)]
#[instruction(data_bump: u8, escrow_bump: u8)]
pub struct CancelVesting<'info> {
    #[account(
        mut,
        seeds = [b"data_account", token_mint.key().as_ref()],
        bump = data_bump,
        constraint = data_account.initializer == sender.key() @ VestingError::InvalidSender,
    )]
    pub data_account: Account<'info, DataAccount>,

    #[account(
        mut,
        seeds = [b"escrow_wallet", token_mint.key().as_ref()],
        bump = escrow_bump,
    )]
    // The program-owned escrow token account that temporarily holds tokens until conditions are met.
    pub escrow_wallet: Account<'info, TokenAccount>,
    
 // The recipient's token account where tokens will be sent once escrow conditions are fulfilled.
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,

      // The signer (payer/initiator) of the transaction, usually the one depositing tokens into escrow.
    #[account(mut)]
    pub sender: Signer<'info>,
    
    // The SPL token mint for the token being escrowed (e.g., USDC, custom token).
    pub token_mint: Account<'info, Mint>,
    // The SPL Token Program — required to perform token transfers and account operations.
    pub token_program: Program<'info, Token>,
}
}
