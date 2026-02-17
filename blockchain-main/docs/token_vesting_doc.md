
# 🪙 Solana Token Vesting Program

A secure, flexible SPL token vesting contract built with the Anchor framework on the Solana blockchain.

## 🔧 Features

- 🔐 Token escrow via program-controlled PDA
- 📅 Time-based vesting over a 36-month period
- ⏱ Optional start delay using `start_timestamp`
- 💯 Manual release control using percent-based unlocks
- 👥 Per-beneficiary vesting allocations via PDA accounts
- 🚫 Vesting cancellation with unclaimed fund withdrawal
- 💸 Unclaimed token withdrawal after vesting ends
- ♻️ Rent refunds on beneficiary removal

---

## 📁 Program Accounts

### `DataAccount`
Main vesting config account (1 per token mint).
- `percent_available: u8` — % of total vesting released
- `token_amount: u64` — Total tokens deposited for vesting
- `start_timestamp: i64` — Vesting start time (UNIX)
- `vesting_months: u8` — Total vesting duration (default: 36)
- `initializer: Pubkey` — Admin of vesting schedule
- `claimed_total: u64` — Total tokens claimed by all beneficiaries
- `unclaimed_withdrawn: u64` — Unclaimed tokens withdrawn post vesting
- `decimals: u8` — Token mint decimals

### `BeneficiaryAccount`
Individual vesting allocation.
- `key: Pubkey` — Beneficiary wallet
- `allocated_tokens: u64` — Total tokens allocated
- `claimed_tokens: u64` — Claimed portion

---

## 🛠 Instructions

### `initialize`
Create the `DataAccount` + `escrow_wallet`, and deposit tokens.

```ts
initialize(amount: u64, decimals: u8, start_timestamp: i64)
````

### `add_beneficiaries`

Adds one or more beneficiaries and allocates tokens.

```ts
add_beneficiaries([{ key: Pubkey, allocated_tokens: u64 }, ...])
```

### `release`

Allows the initializer to increase the `percent_available`.

```ts
release(percent: u8)
```

### `claim`

Lets a beneficiary claim vested tokens.

```ts
claim()
```

### `cancel_vesting`

Withdraws unclaimed tokens before vesting completion.

```ts
cancel_vesting()
```

### `withdraw_unclaimed`

Allows the initializer to withdraw leftover unclaimed tokens after vesting ends.

```ts
withdraw_unclaimed()
```

### `remove_beneficiaries`

Closes unused beneficiary accounts and refunds rent to initializer.

```ts
remove_beneficiaries([Pubkey, Pubkey, ...])
```

---

## 🧪 Testing

> Powered by [solana-bankrun](https://github.com/anza-xyz/solana-bankrun) + `@coral-xyz/anchor`.

Test cases cover:

* Vesting with cliff + monthly linear release
* Claims before and after start time
* Cancelling vesting early
* Removing unused beneficiaries
* Withdrawing unclaimed tokens

To run tests:

```bash
yarn test
```

---

## 📦 Deployment

```bash
anchor build
anchor deploy
```

Program ID should be updated in:

```rust
declare_id!("YourDeployedProgramIdHere");
```

---

## 📚 PDA Seeds

* `data_account`: `["data_account", token_mint]`
* `escrow_wallet`: `["escrow_wallet", token_mint]`
* `beneficiary_account`: `["beneficiary", data_account, beneficiary_pubkey]`

---

## 🚨 Errors

| Code                       | Meaning                        |
| -------------------------- | ------------------------------ |
| `InvalidSender`            | Caller is not the initializer  |
| `ClaimNotAllowed`          | Tokens not yet claimable       |
| `BeneficiaryNotFound`      | Invalid or missing beneficiary |
| `VestingNotStarted`        | Too early to claim             |
| `ZeroVestingAmount`        | Token amount must be > 0       |
| `InvalidPercentage`        | Release percent > 100          |
| `VestingStillActive`       | Tokens still vesting           |
| `NoUnclaimedTokens`        | Nothing left to withdraw       |
| `BeneficiaryAlreadyExists` | Account already initialized    |
| `VestingAlreadyCompleted`  | Vesting fully over             |

---

## 👨‍💻 Contributors

* \[Your Name / Handle]
* Built using [Anchor](https://book.anchor-lang.com/)

