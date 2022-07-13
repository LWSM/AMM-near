#start from res folder
wdir=$(pwd)
rm *.wasm
cd ../amm-contract
cargo build --target wasm32-unknown-unknown --release
cp ./target/wasm32-unknown-unknown/release/amm_contract.wasm $wdir/amm_contract.wasm
cd ../token-contract
cargo build --target wasm32-unknown-unknown --release
cp ./target/wasm32-unknown-unknown/release/token_contract.wasm $wdir/token_contract.wasm
cd $wdir


#prepare accounts
owner_id=lanceorderly.testnet

FT_A_contact_id=ft_a_contract.lanceorderly.testnet
FT_B_contact_id=ft_b_contract.lanceorderly.testnet
FT_A_issuer_id=ft_a_mint.lanceorderly.testnet
FT_B_issuer_id=ft_b_mint.lanceorderly.testnet

amm_contract_id=amm_contract.lanceorderly.testnet
amm_owner_id=amm_owner.lanceorderly.testnet
user_id=user.lanceorderly.testnet

near delete $FT_A_contact_id $owner_id
near delete $FT_B_contact_id $owner_id
near delete $FT_A_issuer_id $owner_id
near delete $FT_B_issuer_id $owner_id
near delete $amm_contract_id $owner_id
near delete $amm_owner_id $owner_id
near delete $user_id $owner_id

near create-account $FT_A_contact_id --masterAccount $owner_id --initialBalance 5
near create-account $FT_B_contact_id --masterAccount $owner_id --initialBalance 5
near create-account $FT_A_issuer_id --masterAccount $owner_id --initialBalance 5
near create-account $FT_B_issuer_id --masterAccount $owner_id --initialBalance 5
near create-account $amm_owner_id --masterAccount $owner_id --initialBalance 5
near create-account $amm_contract_id --masterAccount $owner_id --initialBalance 5
near create-account $user_id --masterAccount $owner_id --initialBalance 5

#deploy token contracts and mint tokens
near deploy $FT_A_contact_id --wasmFile="./token_contract.wasm"
near deploy $FT_B_contact_id --wasmFile="./token_contract.wasm"
near call $FT_A_contact_id new_default_meta '{"owner_id":"'$FT_A_issuer_id'", "name":"Token A Contract", "symbol":"TKA", "total_supply":1000000000000000, "decimals": 6}' --accountId=$FT_A_issuer_id
near call $FT_B_contact_id new_default_meta '{"owner_id":"'$FT_B_issuer_id'", "name":"Token B Contract", "symbol":"TKB", "total_supply":150000000000000000, "decimals": 9}' --accountId=$FT_B_issuer_id

near view $FT_A_contact_id ft_balance_of '{"account_id": "'$FT_A_issuer_id'"}'
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$FT_B_issuer_id'"}'

# transfer tokens to user and amm owner
near call $FT_A_contact_id storage_deposit '{"account_id": "'$amm_owner_id'"}' --accountId=$owner_id --deposit=1
near call $FT_B_contact_id storage_deposit '{"account_id": "'$amm_owner_id'"}' --accountId=$owner_id --deposit=1
near call $FT_A_contact_id ft_transfer '{"receiver_id": "'$amm_owner_id'","amount":"600000000000000"}' --accountId=$FT_A_issuer_id --deposit=0.000000000000000000000001
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$amm_owner_id'"}'
near call $FT_B_contact_id ft_transfer '{"receiver_id": "'$amm_owner_id'","amount":"120000000000000000"}' --accountId=$FT_B_issuer_id --deposit=0.000000000000000000000001
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$amm_owner_id'"}'

near call $FT_A_contact_id storage_deposit '{"account_id": "'$user_id'"}' --accountId=$owner_id --deposit=0.25
near call $FT_B_contact_id storage_deposit '{"account_id": "'$user_id'"}' --accountId=$owner_id --deposit=0.25
near call $FT_A_contact_id ft_transfer '{"receiver_id": "'$user_id'","amount":"100000000"}' --accountId=$FT_A_issuer_id --deposit=0.000000000000000000000001
near call $FT_B_contact_id ft_transfer '{"receiver_id": "'$user_id'","amount":"300000000000"}' --accountId=$FT_B_issuer_id --deposit=0.000000000000000000000001
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$user_id'"}'
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$user_id'"}'

# deploy amm contract and check balance of AMM and owner
near deploy $amm_contract_id --wasmFile="./amm_contract.wasm"
near call $amm_contract_id new '{"owner_id":"'$amm_owner_id'", "a_contract_id":"'$FT_A_contact_id'", "b_contract_id":"'$FT_B_contact_id'", "a_init_amount": 500000000000000, "b_init_amount": 100000000000000000}' --accountId=$amm_owner_id --gas=50000000000000
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$amm_owner_id'"}'
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$amm_owner_id'"}'
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$amm_contract_id'"}'
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$amm_contract_id'"}'