#prepare accounts
owner_id=lanceorderly.testnet
FT_A_contact_id=ft_a_contract.lanceorderly.testnet
FT_B_contact_id=ft_b_contract.lanceorderly.testnet
FT_A_issuer_id=ft_a_mint.lanceorderly.testnet
FT_B_issuer_id=ft_b_mint.lanceorderly.testnet
amm_contract_id=amm_contract.lanceorderly.testnet
amm_owner_id=amm_owner.lanceorderly.testnet
user_id=user.lanceorderly.testnet

# test amm contract
near view $amm_contract_id get_info
near view $amm_contract_id get_ratio
near call $amm_contract_id swap_from_a '{"amount":5}' --accountId=$user_id --gas=40000000000000
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$user_id'"}'
near view $FT_B_contact_id ft_balance_of '{"account_id": "'$user_id'"}'
near call $amm_contract_id deposit_b_by_owner '{"amount":30000}' --accountId=$amm_owner_id
near view $amm_contract_id get_info
near view $amm_contract_id get_ratio