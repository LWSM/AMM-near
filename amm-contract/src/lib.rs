use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, log, near_bindgen, require, AccountId, Balance, PanicOnDefault};

#[ext_contract(ext_token)]
trait ExtToken {
    fn get_token_contract_meta_info(&self) -> (String, u8);
    fn account_register(&mut self);
    fn transfer_from(&mut self, sender: AccountId, receiver: AccountId, amount: Balance);
}

#[ext_contract(ext_self)]
trait ExtSelf {
    fn callback_get_info(&mut self, contract_id: AccountId, #[callback] val: (String, u8));
    fn callback_ft_deposit(
        &mut self,
        a_vault_after: Balance,
        b_vault_after: Balance,
        contract_id: AccountId,
        receiver_id: AccountId,
        amount: Balance,
    );
    fn callback_update_vaults(&mut self, a_vault_after: Balance, b_vault_after: Balance);
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Contract {
    owner_id: AccountId,
    ratio: u128,
    a_vault: Balance,
    a_contract_id: AccountId,
    a_contract_name: String,
    a_contract_decimals: u8,
    b_vault: Balance,
    b_contract_id: AccountId,
    b_contract_name: String,
    b_contract_decimals: u8,
}

#[near_bindgen]
impl Contract {
    /// Initialization:
    /// Param:
    /// contract owner ID
    /// Program ID of tokens pairs A and B
    /// Initiate amount of tokens pairs A and B
    #[init]
    pub fn new(owner_id: AccountId, a_contract_id: AccountId, b_contract_id: AccountId, a_init_amount: u128, b_init_amount: u128, ) -> Self {
        assert!(!env::state_exists(), "AMM initialized...");

        let this = Self {
            owner_id: owner_id.clone(),
            ratio: 0,
            a_vault: a_init_amount.clone(),
            a_contract_id,
            a_contract_name: "".into(),
            a_contract_decimals: 1,
            b_vault: b_init_amount.clone(),
            b_contract_id,
            b_contract_name: "".into(),
            b_contract_decimals: 1,
        };
        // get symbol and decimal of token pair A and B
        ext_token::ext(this.a_contract_id.clone()).get_token_contract_meta_info().then(
            ext_self::ext(env::current_account_id()).callback_get_info(this.a_contract_id.clone()),
        );
        ext_token::ext(this.b_contract_id.clone()).get_token_contract_meta_info().then(
            ext_self::ext(env::current_account_id()).callback_get_info(this.b_contract_id.clone()),
        );

        // get amm registered to token pair A and B
        ext_token::ext(this.a_contract_id.clone()).account_register();
        ext_token::ext(this.b_contract_id.clone()).account_register();

        // deposit tokens pair A and B to AMM from owner
        ext_token::ext(this.a_contract_id.clone()).transfer_from(owner_id.clone(), env::current_account_id(), a_init_amount.clone());
        ext_token::ext(this.b_contract_id.clone()).transfer_from(owner_id.clone(), env::current_account_id(), b_init_amount.clone());

        this
    }

    pub fn callback_get_info(&mut self, contract_id: AccountId, #[callback] info: (String, u8)) {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            "Authority Check Failed: Owner function"
        );
        log!("Fill additional info for {}", info.0);
        if contract_id == self.a_contract_id {
            self.a_contract_name = info.0; //symbnol acctually
            self.a_contract_decimals = info.1;
        } else if contract_id == self.b_contract_id {
            self.b_contract_name = info.0;
            self.b_contract_decimals = info.1;
        }
        self.calc_ratio();
    }

    pub fn get_info(
        &self,
    ) -> (
        (AccountId, String, Balance, u8),
        (AccountId, String, Balance, u8),
    ) {
        (
            (
                self.a_contract_id.clone(),
                self.a_contract_name.clone(),
                self.a_vault,
                self.a_contract_decimals,
            ),
            (
                self.b_contract_id.clone(),
                self.b_contract_name.clone(),
                self.b_vault,
                self.b_contract_decimals,
            ),
        )
    }

    pub fn get_ratio(&self) -> u128 {
        self.ratio
    }

    /// calculate CONST K for K = X * Y;
    fn calc_ratio(&mut self) {
        let a_num = self.a_vault;
        let b_num = self.b_vault;
        self.ratio = a_num.checked_mul(b_num).unwrap();
    }

    /// User Swap functions: deposit part, in transfer_from, redeem part in callback_ft_deposit
    #[payable]
    pub fn swap_from_a(&mut self, amount: Balance) {
        let sender_id = env::predecessor_account_id();
        let a_amount = amount.checked_mul(10_u128.checked_pow(self.a_contract_decimals as u32).unwrap()).unwrap();
        // core logic of AMM here
        let a_vault_after = a_amount.checked_add(self.a_vault).unwrap();
        let b_vault_after = self.ratio.checked_div(a_vault_after).unwrap();
        let b_amount = self.b_vault.checked_sub(b_vault_after).unwrap();
        ext_token::ext(self.a_contract_id.clone())
            .transfer_from(sender_id.clone(), env::current_account_id(), a_amount)
            .then(
                ext_self::ext(env::current_account_id()).callback_ft_deposit(
                    a_vault_after,
                    b_vault_after,
                    self.b_contract_id.clone(),
                    sender_id,
                    b_amount,
                ),
            );
    }
    #[payable]
    pub fn swap_from_b(&mut self, amount: Balance) {
        let sender_id = env::predecessor_account_id();
        let b_amount = amount.checked_mul(10_u128.checked_pow(self.b_contract_decimals as u32).unwrap()).unwrap();
        let b_vault_after = b_amount.checked_add(self.b_vault).unwrap();
        let a_vault_after = self.ratio.checked_div(b_vault_after).unwrap();
        let a_amount = self.a_vault.checked_sub(a_vault_after).unwrap();
        ext_token::ext(self.b_contract_id.clone())
            .transfer_from(sender_id.clone(), env::current_account_id(), b_amount)
            .then(
                ext_self::ext(env::current_account_id()).callback_ft_deposit(
                    a_vault_after,
                    b_vault_after,
                    self.a_contract_id.clone(),
                    sender_id,
                    a_amount,
                ),
            );
    }

    /// The owner of the contract can deposit A or B to AMM vault, thereafter change K to maintain the ratio.
    #[payable]
    pub fn deposit_a_by_owner(&mut self, amount: Balance) {
        assert!(
            env::predecessor_account_id() == self.owner_id,
            "Authority Check Failed: Owner function"
        );
        let a_amount = amount.checked_mul(10_u128.checked_pow(self.a_contract_decimals as u32).unwrap()).unwrap();
        let a_vault_after = a_amount.checked_add(self.a_vault).unwrap();
        ext_token::ext(self.a_contract_id.clone())
            .transfer_from(self.owner_id.clone(), env::current_account_id(), a_amount)
            .then(
                ext_self::ext(env::current_account_id())
                    .callback_update_vaults(a_vault_after, self.b_vault),
            );
    }

    #[payable]
    pub fn deposit_b_by_owner(&mut self, amount: Balance) {
        assert!(
            env::predecessor_account_id() == self.owner_id,
            "Authority Check Failed: Owner function"
        );
        let b_amount = amount.checked_mul(10_u128.checked_pow(self.b_contract_decimals as u32).unwrap()).unwrap();
        let b_vault_after = b_amount.checked_add(self.b_vault).unwrap();
        ext_token::ext(self.b_contract_id.clone())
            .transfer_from(self.owner_id.clone(), env::current_account_id(), b_amount)
            .then(
                ext_self::ext(env::current_account_id())
                    .callback_update_vaults(self.a_vault, b_vault_after),
            );
    }

    pub fn callback_ft_deposit(
        &mut self,
        a_vault_after: Balance,
        b_vault_after: Balance,
        contract_id: AccountId,
        receiver_id: AccountId,
        amount: Balance,
    ) {
        assert!(
            env::predecessor_account_id() == env::current_account_id(),
            "Authority Check Failed: self-called function"
        );
        ext_token::ext(contract_id)
            .transfer_from(env::current_account_id(), receiver_id, amount)
            .then(
                ext_self::ext(env::current_account_id())
                    .callback_update_vaults(a_vault_after, b_vault_after),
            );
    }

    pub fn callback_update_vaults(&mut self, a_vault_after: Balance, b_vault_after: Balance) {
        assert!(
            env::predecessor_account_id() == env::current_account_id(),
            "Authority Check Failed: self-called function"
        );
        self.a_vault = a_vault_after;
        self.b_vault = b_vault_after;
        self.calc_ratio();
    }
}
