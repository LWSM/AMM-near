
## Vanilla AMM on testnet, Near-chain

A simple vanilla AMM on Near-chain. This is a simple example of how AMM works.

  

Two smart contracts are used in this example, token_contract and amm_contract.

  

**token_contract:**

* mainly derived from offical token contract example.

* basic functions like `mint`, `burn`, `transfer` are implemented.

* for AMM testing, only`mint` and `transfer` is applied.

**amm_contract:**

* automatic market making based on a simple market making algorithm.

* $X * Y = K$
 in which $X$ is the amount of the token X, $Y$ is the amount of the token You, and $K$ is a constant.

* each swap garantee that $K$ is unchanged. (i.e. $（X+\delta X) * （Y-\delta Y)  = K$)
in which $\delta X$ is the amount of token $X$, swapped for $\delta Y$, the amount of token $Y$

  
## Prerequisite

Purposeful omission


## Prepare Accounts

NEAR accounts are super readable and the logic is straightforward.
Preparation of accounts (IDs):
```bash
# root account
owner_id=lanceorderly.testnet
# fungible token contact holder
FT_A_contact_id=ft_a_contract.lanceorderly.testnet
FT_B_contact_id=ft_b_contract.lanceorderly.testnet
# fungible token issuer
FT_A_issuer_id=ft_a_mint.lanceorderly.testnet
FT_B_issuer_id=ft_b_mint.lanceorderly.testnet
# amm contact holder
amm_contract_id=amm_contract.lanceorderly.testnet
# amm contact owner
amm_owner_id=amm_owner.lanceorderly.testnet
# amm contact user
user_id=user.lanceorderly.testnet

# an example of account preparation:
near login
near delete $user_id  $owner_id
near create-account $user_id --masterAccount $owner_id --initialBalance 5
```
  

## Build WASM and Deploy
**Build WASM**
```bash

# start from res folder
wdir=$(pwd)
rm *.wasm
cd ../amm-contract
cargo build --target wasm32-unknown-unknown --release
cp ./target/wasm32-unknown-unknown/release/amm_contract.wasm $wdir/amm_contract.wasm
cd ../token-contract
cargo build --target wasm32-unknown-unknown --release
cp ./target/wasm32-unknown-unknown/release/token_contract.wasm $wdir/token_contract.wasm
cd  $wdir

```
**Deploy**
```bash
# fungible token contact
near deploy $FT_A_contact_id --wasmFile="./token_contract.wasm"
near deploy $FT_B_contact_id --wasmFile="./token_contract.wasm"
# amm contact
near deploy $amm_contract_id --wasmFile="./amm_contract.wasm"
```

## AMM preparation
**Mint token and distribute**
```bash
# mint
near call $FT_A_contact_id new_default_meta '{"owner_id":"'$FT_A_issuer_id'", "name":"Token A Contract", "symbol":"TKA", "total_supply":1000000000000000, "decimals": 6}' --accountId=$FT_A_issuer_id

# distribute to amm user and owner, need to storage_deposit first for registration
near call $FT_A_contact_id storage_deposit '{"account_id": "'$amm_owner_id'"}' --accountId=$owner_id --deposit=1
near call $FT_A_contact_id ft_transfer '{"receiver_id": "'$amm_owner_id'","amount":"600000000000000"}' --accountId=$FT_A_issuer_id --deposit=0.000000000000000000000001
```
**Initiate AMM**
```bash
# initiate and deposit and calculate K
near call $amm_contract_id new '{"owner_id":"'$amm_owner_id'", "a_contract_id":"'$FT_A_contact_id'", "b_contract_id":"'$FT_B_contact_id'", "a_init_amount": 500000000000000, "b_init_amount": 100000000000000000}' --accountId=$amm_owner_id --gas=50000000000000
```
## AMM test
```bash
# user swap
near call $amm_contract_id swap_from_a '{"amount":5}' --accountId=$user_id --gas=40000000000000
# owner deposit
near call $amm_contract_id deposit_b_by_owner '{"amount":30000}' --accountId=$amm_owner_id

# view balance and AMM info
near view $amm_contract_id get_info
near view $amm_contract_id get_ratio
near view $FT_A_contact_id ft_balance_of '{"account_id": "'$user_id'"}'
```
## AMM function explains
**Cross Contract Call to Fungible Token contract**
```rust
#[ext_contract(ext_token)]
trait  ExtToken {
fn  get_token_contract_meta_info(&self) -> (String, u8);
fn  account_register(&mut  self);
fn  transfer_from(&mut  self, sender: AccountId, receiver: AccountId, amount: Balance);
}
```
fn for getting token symbol and decimal
```rust
fn  get_token_contract_meta_info(&self) -> (String, u8);
```
fn for registering ID for holding token
```rust
fn  account_register(&mut  self);
```
fn for transfer within Cross-Contract transaction
```rust
fn transfer_from(&mut  self, sender: AccountId, receiver: AccountId, amount: Balance);
```
**Cross Contract Call to AMM self**
```rust
#[ext_contract(ext_self)]
trait  ExtSelf {
fn  callback_get_info(&mut  self, contract_id: AccountId, #[callback] val: (String, u8));
fn  callback_ft_deposit(...);
fn  callback_update_vaults(...);
}
```
fn for getting token symbol and decimal
```rust
fn  callback_get_info(&mut  self, contract_id: AccountId, #[callback] val: (String, u8));
```
CORE fn for transfering token to swap user after user deposit
```rust
fn  callback_ft_deposit(&mut  self,
	a_vault_after: Balance,
	b_vault_after: Balance,
	contract_id: AccountId,
	receiver_id: AccountId,
	amount: Balance,);
```
CORE fn for balance setting and recalculating K
```rust
fn  callback_update_vaults(&mut  self, a_vault_after: Balance, b_vault_after: Balance);
```
**Swap Function** 
decoupled for easier debug. In this function, deposit is first added into pool. The amount of the dependent Token is calculated from  $X * Y = K$. The difference is passed into callback_ft_deposit fn for token settling.
```rust
pub  fn  swap_from_b(&mut  self, amount: Balance) {
let  sender_id = env::predecessor_account_id();
let  b_amount = amount.checked_mul(10_u128.checked_pow(self.b_contract_decimals as  u32).unwrap()).unwrap();
let  b_vault_after = b_amount.checked_add(self.b_vault).unwrap();
let  a_vault_after = self.ratio.checked_div(b_vault_after).unwrap();
let  a_amount = self.a_vault.checked_sub(a_vault_after).unwrap();
ext_token::ext(self.b_contract_id.clone())
.transfer_from(sender_id.clone(), env::current_account_id(), b_amount)
.then(
ext_self::ext(env::current_account_id()).callback_ft_deposit(
a_vault_after,
b_vault_after,
self.a_contract_id.clone(),
sender_id,
a_amount,
),);}

pub  fn  swap_from_b(&mut  self, amount: Balance);
```
**Owner Deposit Function** 
In this process， $X/Y$ ratio is constant and $K$ will be changed.
```rust
#[payable]
pub  fn  deposit_a_by_owner(&mut  self, amount: Balance)

fn  calc_ratio(&mut  self) {
let  a_num = self.a_vault;
let  b_num = self.b_vault;
self.ratio = a_num.checked_mul(b_num).unwrap();
}
```
**Functions for Info Printing** 
```rust
pub  fn  get_info()
pub  fn  get_ratio()
```