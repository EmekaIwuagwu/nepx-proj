use near_sdk::{near, env, require, AccountId, Gas, NearToken, Promise, PublicKey};
use near_sdk::json_types::U128;
use near_sdk::store::{IterableSet, IterableMap};
use xchain_core::{
    BridgeProof, BridgeRoute, CanonicalAssetId, SecurityModel, XChainCore, BridgeDirection, ReceiptTokenMetadata, ChainId
};

const GAS_MINT_TOKEN: Gas = Gas::from_tgas(10);
const GAS_INIT_TOKEN: Gas = Gas::from_tgas(10);
const BRIDGE_FEE: NearToken = NearToken::from_millinear(100);

#[near(contract_state)]
pub struct XChainBridge {
    pub receipt_token_code: Vec<u8>,
    pub canonical_to_receipt: IterableMap<String, AccountId>,
    pub receipt_to_canonical: IterableMap<AccountId, CanonicalAssetId>,
    pub processed_proofs: IterableSet<Vec<u8>>,
    pub owner_id: AccountId,
    pub validators: IterableSet<PublicKey>,
    pub paused: bool,
    pub treasury: AccountId,
}

impl Default for XChainBridge {
    fn default() -> Self {
        env::panic_str("Contract must be initialized")
    }
}

#[near]
impl XChainBridge {
    #[init]
    pub fn new(owner_id: AccountId, initial_validators: Vec<PublicKey>) -> Self {
        let mut v = IterableSet::new(b"v");
        for key in initial_validators {
            v.insert(key);
        }
        Self {
            receipt_token_code: vec![],
            canonical_to_receipt: IterableMap::new(b"c"),
            receipt_to_canonical: IterableMap::new(b"r"),
            processed_proofs: IterableSet::new(b"p"),
            owner_id: owner_id.clone(),
            validators: v,
            paused: false,
            treasury: owner_id,
        }
    }

    pub fn set_receipt_token_code(&mut self, code: Vec<u8>) {
        self.assert_owner();
        self.receipt_token_code = code;
    }
    
    pub fn set_treasury(&mut self, treasury: AccountId) {
        self.assert_owner();
        self.treasury = treasury;
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner_id, "Unauthorized");
    }

    fn assert_not_paused(&self) {
        require!(!self.paused, "Bridge is paused");
    }
    
    fn emit_bridge_in(canonical_asset: &CanonicalAssetId, amount: U128, receiver_id: &AccountId, source_tx_hash: &str, receipt_token: &AccountId) {
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep_xchain",
                "version": "1.0.0",
                "event": "bridge_in",
                "data": {
                    "canonical_asset": canonical_asset,
                    "amount": amount,
                    "receiver_id": receiver_id,
                    "source_tx_hash": source_tx_hash,
                    "receipt_token": receipt_token
                }
            })
        ));
    }

    fn emit_bridge_out(canonical_asset: &CanonicalAssetId, amount: U128, sender_id: &AccountId, destination_chain: &str, destination_address: &str, receipt_token: &AccountId) {
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep_xchain",
                "version": "1.0.0",
                "event": "bridge_out",
                "data": {
                    "canonical_asset": canonical_asset,
                    "amount": amount,
                    "sender_id": sender_id,
                    "destination_chain": destination_chain,
                    "destination_address": destination_address,
                    "receipt_token": receipt_token
                }
            })
        ));
    }
}

#[near]
impl XChainCore for XChainBridge {
    #[payable]
    fn bridge_in(
        &mut self,
        proof: BridgeProof,
        receiver_id: AccountId,
    ) -> Promise {
        self.assert_not_paused();
        
        let proof_hash = env::sha256(&proof.proof_data);
        require!(!self.processed_proofs.contains(&proof_hash), "Proof already used");
        self.processed_proofs.insert(proof_hash);

        let payload: xchain_core::BridgePayload = near_sdk::borsh::BorshDeserialize::try_from_slice(&proof.proof_data)
            .expect("Failed to deserialize BridgePayload");

        require!(payload.target_chain.contains("near"), "Wrong target chain");
        require!(payload.receiver == receiver_id, "Receiver mismatch");

        let canonical_id_str = format!("{}:{}:{:?}", 
            payload.asset.source_chain_id, 
            payload.asset.source_contract, 
            payload.asset.asset_standard
        );
        let amount = payload.amount;
        let canonical_asset = payload.asset;
        
        if let Some(token_account) = self.canonical_to_receipt.get(&canonical_id_str) {
            Self::emit_bridge_in(&canonical_asset, amount, &receiver_id, &proof.source_tx_hash, token_account);
            
            Promise::new(token_account.clone())
                .function_call(
                    "mint".to_string(),
                    near_sdk::serde_json::json!({
                        "account_id": receiver_id,
                        "amount": amount
                    }).to_string().into_bytes(),
                    NearToken::from_yoctonear(0),
                    GAS_MINT_TOKEN,
                )
        } else {
            require!(!self.receipt_token_code.is_empty(), "Factory not initialized");
            let sub_account_name = hex::encode(&env::sha256(canonical_id_str.as_bytes())[0..20]);
            let token_account_id: AccountId = format!("{}.{}", sub_account_name, env::current_account_id()).parse().unwrap();
            
            self.canonical_to_receipt.insert(canonical_id_str.clone(), token_account_id.clone());
            self.receipt_to_canonical.insert(token_account_id.clone(), canonical_asset.clone());

            Self::emit_bridge_in(&canonical_asset, amount, &receiver_id, &proof.source_tx_hash, &token_account_id);

            Promise::new(token_account_id.clone())
                .create_account()
                .transfer(env::attached_deposit())
                .deploy_contract(self.receipt_token_code.clone())
                .function_call(
                    "new".to_string(),
                    near_sdk::serde_json::json!({
                        "owner_id": env::current_account_id(),
                        "metadata": {
                            "spec": "ft-1.0.0",
                            "name": format!("Bridged {}", canonical_asset.source_contract),
                            "symbol": "xAsset",
                            "decimals": 18 
                        },
                        "xc_metadata": ReceiptTokenMetadata {
                            canonical_asset: canonical_asset.clone(),
                            bridge_route: self.get_bridge_route(),
                            total_locked: amount,
                            nep141_token: token_account_id.clone()
                        }
                    }).to_string().into_bytes(),
                    NearToken::from_yoctonear(0),
                    GAS_INIT_TOKEN,
                )
                .then(
                    Promise::new(token_account_id)
                        .function_call(
                            "mint".to_string(),
                            near_sdk::serde_json::json!({
                                "account_id": receiver_id,
                                "amount": amount
                            }).to_string().into_bytes(),
                            NearToken::from_yoctonear(0),
                            GAS_MINT_TOKEN,
                        )
                )
        }
    }

    #[payable]
    fn bridge_out(
        &mut self,
        token_id: AccountId,
        amount: U128,
        destination_chain: ChainId,
        destination_address: String,
    ) -> Promise {
        self.assert_not_paused();
        
        let attached = env::attached_deposit();
        require!(attached >= BRIDGE_FEE, "Insufficient Fee");
        let _ = Promise::new(self.treasury.clone()).transfer(attached);

        let asset = self.receipt_to_canonical.get(&token_id).expect("Token not managed by bridge").clone();

        Self::emit_bridge_out(
            &asset, 
            amount, 
            &env::predecessor_account_id(), 
            &destination_chain, 
            &destination_address, 
            &token_id
        );
        
        Promise::new(token_id)
            .function_call(
                "burn".to_string(),
                near_sdk::serde_json::json!({
                    "account_id": env::predecessor_account_id(),
                    "amount": amount
                }).to_string().into_bytes(),
                NearToken::from_yoctonear(0),
                GAS_MINT_TOKEN,
            )
    }

    fn get_canonical_asset(&self, token_id: AccountId) -> Option<CanonicalAssetId> {
        self.receipt_to_canonical.get(&token_id).cloned()
    }

    fn get_bridge_route(&self) -> BridgeRoute {
        BridgeRoute {
            bridge_contract: env::current_account_id(),
            security_model: SecurityModel::Committee { 
                threshold: ((self.validators.len() * 2) / 3 + 1) as u8, 
                size: self.validators.len() as u8 
            },
            deployed_at: env::block_timestamp(),
        }
    }

    fn estimate_fee(
        &self,
        _direction: BridgeDirection,
        _asset: Option<CanonicalAssetId>,
        _amount: U128,
    ) -> U128 {
        U128(BRIDGE_FEE.as_yoctonear())
    }
    
    fn set_paused(&mut self, paused: bool) {
        self.assert_owner();
        self.paused = paused;
    }
    
    fn is_paused(&self) -> bool {
        self.paused
    }
    
    fn add_validator(&mut self, public_key: PublicKey) {
        self.assert_owner();
        self.validators.insert(public_key);
    }
    
    fn remove_validator(&mut self, public_key: PublicKey) {
        self.assert_owner();
        self.validators.remove(&public_key);
    }
}
