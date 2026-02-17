

  token_vesting full suite
    ✔ Initializes vesting contract (1425ms)
    ✔ Initializes data account and adds a beneficiary (409ms)
    1) Allows beneficiary to claim available tokens
    ✔ Withdraws unclaimed tokens after full vesting (391ms)

  advanced-token-vesting
    ✔ Prevents claiming before vesting starts (820ms)
    ✔ Allows releasing percent multiple times (796ms)

  token_vesting full suite
    ✔ Initializes vesting contract (1614ms)
    ✔ Initializes data account and adds a beneficiary (404ms)
    ✔ Releases 100% of tokens manually (407ms)
    ✔ Allows beneficiary to claim available tokens (3881ms)
    ✔ Withdraws unclaimed tokens after full vesting (404ms)


  10 passing (14s)
  1 failing

  1) token_vesting full suite
       Allows beneficiary to claim available tokens:
     Error: AnchorError thrown in programs/test/src/lib.rs:109. Error Code: ClaimNotAllowed. Error Number: 6001. Error Message: Not allowed to claim new tokens currently.
      at Function.parse (node_modules/@coral-xyz/anchor/src/error.ts:152:14)
      at translateError (node_modules/@coral-xyz/anchor/src/error.ts:277:35)
      at MethodsBuilder.rpc [as _rpcFn] (node_modules/@coral-xyz/anchor/src/program/namespace/rpc.ts:35:29)
      at processTicksAndRejections (node:internal/process/task_queues:95:5)




