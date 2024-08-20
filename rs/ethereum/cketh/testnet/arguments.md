```shell
dfx canister create --all
```

```shell
dfx deploy ledger --argument '(variant { Init = record { minting_account = record { owner = principal "'"$(dfx canister id ledger)"'" }; feature_flags  = opt record { icrc2 = true }; decimals = opt 18; max_memo_length = opt 80; transfer_fee = 10_000_000_000; token_symbol = "ckSepoliaETH"; token_name = "Chain key Sepolia Ethereum"; metadata = vec {}; initial_balances = vec {}; archive_options = record { num_blocks_to_archive = 1000; trigger_threshold = 2000; max_message_size_bytes = null; cycles_for_archive_creation = opt 1_000_000_000_000; node_max_memory_size_bytes = opt 3_221_225_472; controller_id = principal "mf7xa-laaaa-aaaar-qaaaa-cai"; } }})'
```

```shell
dfx deploy minter --argument '(variant {InitArg = record { ethereum_network = variant {Sepolia} ; ecdsa_key_name = "dfx_test_key"; ethereum_contract_address = opt "0x68Ad36013DA43b8938D0CDAdcee186bf24B97e1C" ; ledger_id = principal "'"$(dfx canister id ledger)"'"; ethereum_block_height = variant {Finalized} ; minimum_withdrawal_amount = 10_000_000_000_000_000; next_transaction_nonce = 0 ; last_scraped_block_number = 6523854; }})'
```

```shell
dfx deploy orchestrator --argument "(variant { InitArg = record { more_controller_ids = vec { principal \"mf7xa-laaaa-aaaar-qaaaa-cai\"; }; minter_id = opt principal \"$(dfx canister id minter)\"; cycles_management = opt record { cycles_for_ledger_creation = 2_000_000_000_000 ; cycles_for_archive_creation = 1_000_000_000_000; cycles_for_index_creation = 1_000_000_000_000; cycles_top_up_increment = 500_000_000_000 } }})"
```

```shell
dfx deploy minter --argument "(variant {UpgradeArg = record {ledger_suite_orchestrator_id = opt principal \"$(dfx canister id orchestrator)\"; erc20_helper_contract_address = opt \"0x68Ad36013DA43b8938D0CDAdcee186bf24B97e1C\"; last_erc20_scraped_block_number = opt 6523854;}})" --upgrade-unchanged
```

```shell
dfx deploy orchestrator --argument "(variant { AddErc20Arg = record { contract = record { chain_id = 11155111; address = \"0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238\" }; ledger_init_arg = record { minting_account = record { owner = principal \"$(dfx canister id minter)\" }; feature_flags = opt record { icrc2 = true }; decimals = 6; max_memo_length = opt 80; transfer_fee = 4_000; token_symbol = \"ckSepoliaUSDC\"; token_name = \"Chain key Sepolia USDC\"; token_logo = \"\"; initial_balances = vec {}; }; git_commit_hash = \"3924e543af04d30a0b601d749721af239a10dff6\"; ledger_compressed_wasm_hash = \"57e2a728f9ffcb1a7d9e101dbd1260f8b9f3246bf5aa2ad3e2c750e125446838\"; index_compressed_wasm_hash = \"6fb62c7e9358ca5c937a5d25f55700459ed09a293d0826c09c631b64ba756594\"; }})"
```
