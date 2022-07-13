use near_contract_standards;
use near_contract_standards::fungible_token::{
    events::FtTransfer,
    metadata::{FungibleTokenMetadata, FT_METADATA_SPEC},
    FungibleToken,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::LazyOption,
    env,
    json_types::U128,
    log, near_bindgen, require, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    PromiseOrValue,
};
/// a basic FungibleToken contract for 2 test tokens
/// Framework Borrowed From Near SDK example
#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    FungibleToken,
    Metadata,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(
        owner_id: AccountId,
        name: String,
        symbol: String,
        total_supply: Balance,
        decimals: u8,
    ) -> Self {
        Self::new(
            owner_id,
            total_supply,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name,
                symbol,
                icon: None,
                reference: None,
                reference_hash: None,
                decimals,
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: AccountId,
        total_supply: Balance,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        this
    }

    pub fn get_token_contract_meta_info(self) -> (String, u8) {
        (
            self.metadata.get().unwrap().symbol,
            self.metadata.get().unwrap().decimals,
        )
    }

    #[payable]
    pub fn account_register(&mut self) {
        let wallet_id = env::predecessor_account_id();
        self.token.internal_register_account(&wallet_id);
    }

    #[payable]
    pub fn transfer_from(&mut self, sender: AccountId, receiver: AccountId, amount: Balance) {
        require!(amount > 0, "Attampted to transfer 0 ammount!");
        require!(
            sender != receiver,
            "Attampted to transfer to the same account!"
        );
        require!(
            self.token.internal_unwrap_balance_of(&sender) >= amount,
            "insufficient balance"
        );
        self.token.internal_withdraw(&sender, amount);
        if !self.token.accounts.contains_key(&receiver) {
            log!("Account {} does not exist, creating it", receiver);
            self.token.internal_register_account(&receiver);
        }
        self.token.internal_deposit(&receiver, amount);
        FtTransfer {
            old_owner_id: &sender,
            new_owner_id: &receiver,
            amount: &U128(amount),
            memo: None,
        }
        .emit();
    }
    // need to look into these functions latter
    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;
    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }
    const TOTAL_SUPPLY: Balance = 10_000_000_000_000;
    const DECIMALS: u8 = 6;

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        let name = "TEST Token Contract";
        let symbol = "TEST";

        testing_env!(context.build());
        let contract = Contract::new(
            accounts(1).into(),
            TOTAL_SUPPLY,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: name.into(),
                symbol: symbol.into(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: DECIMALS,
            },
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
        assert_eq!(
            contract.get_token_contract_meta_info(),
            (symbol.into(), DECIMALS)
        );
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(
            accounts(2).into(),
            "TEST Token Contract".to_string(),
            "TEST".to_string(),
            TOTAL_SUPPLY.into(),
            DECIMALS,
        );
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount: Balance = 1_000_000_000_000;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(
            contract.ft_balance_of(accounts(2)).0,
            (TOTAL_SUPPLY - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
