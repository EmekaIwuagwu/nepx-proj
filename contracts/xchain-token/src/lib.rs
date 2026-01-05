use near_sdk::{near, env, require, AccountId, PromiseOrValue};
use near_sdk::json_types::U128;
use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::storage_management::StorageManagement;
use xchain_core::{ReceiptTokenMetadata, XChainReceiptToken, ChainId};

#[near(contract_state)]
pub struct ReceiptToken {
    token: FungibleToken,
    metadata: FungibleTokenMetadata,
    xc_metadata: ReceiptTokenMetadata,
    owner_id: AccountId,
}

impl Default for ReceiptToken {
    fn default() -> Self {
        env::panic_str("Contract must be initialized")
    }
}

#[near]
impl ReceiptToken {
    #[init]
    pub fn new(
        owner_id: AccountId,
        metadata: FungibleTokenMetadata,
        xc_metadata: ReceiptTokenMetadata,
    ) -> Self {
        Self {
            token: FungibleToken::new(b"t".to_vec()),
            metadata,
            xc_metadata,
            owner_id,
        }
    }

    #[payable]
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> near_contract_standards::storage_management::StorageBalance {
        StorageManagement::storage_deposit(&mut self.token, account_id, registration_only)
    }

    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(env::predecessor_account_id() == self.owner_id, "Only bridge can mint");
        self.token.internal_deposit(&account_id, amount.0);
        
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep141",
                "version": "1.0.0",
                "event": "ft_mint",
                "data": [{
                    "owner_id": account_id,
                    "amount": amount
                }]
            })
        ));
    }

    pub fn burn(&mut self, account_id: AccountId, amount: U128) {
        require!(env::predecessor_account_id() == self.owner_id, "Only bridge can burn");
        self.token.internal_withdraw(&account_id, amount.0);
        
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep141",
                "version": "1.0.0",
                "event": "ft_burn",
                "data": [{
                    "owner_id": account_id,
                    "amount": amount
                }]
            })
        ));
    }
}

#[near]
impl near_contract_standards::fungible_token::core::FungibleTokenCore for ReceiptToken {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for ReceiptToken {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.clone()
    }
}

#[near]
impl XChainReceiptToken for ReceiptToken {
    fn xc_metadata(&self) -> ReceiptTokenMetadata {
        self.xc_metadata.clone()
    }

    fn xc_is_bridgeable_to(&self, chain_id: ChainId) -> bool {
        self.xc_metadata.canonical_asset.source_chain_id == chain_id
    }

    fn xc_total_locked(&self) -> U128 {
        self.xc_metadata.total_locked
    }
}
