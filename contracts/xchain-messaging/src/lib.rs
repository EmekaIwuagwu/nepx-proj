use near_sdk::{near, env, require, AccountId, Promise, PublicKey};
use near_sdk::store::{IterableSet, LookupMap};
use near_sdk::json_types::U128;
use xchain_core::{ChainId, BridgeProof, XChainMessaging, XChainMessage};

#[near(contract_state)]
pub struct XChainMessenger {
    pub owner_id: AccountId,
    pub validators: IterableSet<PublicKey>,
    pub processed_nonces: LookupMap<ChainId, U128>,
    pub destination_nonces: LookupMap<ChainId, U128>,
    pub paused: bool,
}

impl Default for XChainMessenger {
    fn default() -> Self {
        env::panic_str("Contract must be initialized")
    }
}

#[near]
impl XChainMessenger {
    #[init]
    pub fn new(owner_id: AccountId, initial_validators: Vec<PublicKey>) -> Self {
        let mut validators = IterableSet::new(b"v");
        for v in initial_validators {
            validators.insert(v);
        }
        Self {
            owner_id,
            validators,
            processed_nonces: LookupMap::new(b"n"),
            destination_nonces: LookupMap::new(b"d"),
            paused: false,
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        require!(env::predecessor_account_id() == self.owner_id, "Unauthorized");
        self.paused = paused;
    }

    pub fn add_validator(&mut self, pk: PublicKey) {
        require!(env::predecessor_account_id() == self.owner_id, "Unauthorized");
        self.validators.insert(pk);
    }

    pub fn remove_validator(&mut self, pk: PublicKey) {
        require!(env::predecessor_account_id() == self.owner_id, "Unauthorized");
        self.validators.remove(&pk);
    }
}

#[near]
impl XChainMessaging for XChainMessenger {
    fn send_message(
        &mut self,
        destination_chain: ChainId,
        destination_contract: String,
        payload: Vec<u8>,
    ) -> Promise {
        require!(!self.paused, "Messenger is paused");
        
        let current_nonce = self.destination_nonces.get(&destination_chain).copied().unwrap_or(U128(0));
        let new_nonce = U128(current_nonce.0 + 1);
        self.destination_nonces.insert(destination_chain.clone(), new_nonce);

        let payload_hash = env::sha256(&payload);
        
        env::log_str(&format!(
            "EVENT_JSON:{}", 
            near_sdk::serde_json::json!({
                "standard": "nep_xchain_msg",
                "version": "1.0.0",
                "event": "send_message",
                "data": {
                    "destination_chain": destination_chain,
                    "destination_contract": destination_contract,
                    "nonce": new_nonce,
                    "sender": env::predecessor_account_id(),
                    "payload_hash": payload_hash
                }
            })
        ));
        
        Promise::new(env::current_account_id())
    }

    fn receive_message(&mut self, proof: BridgeProof) {
        require!(!self.paused, "Messenger is paused");

        let message: XChainMessage = near_sdk::borsh::BorshDeserialize::try_from_slice(&proof.proof_data)
            .expect("Failed to deserialize XChainMessage");

        let current_nonce = self.processed_nonces.get(&message.source_chain).copied().unwrap_or(U128(0));
        require!(message.nonce.0 == current_nonce.0 + 1, "Invalid nonce");
        
        self.processed_nonces.insert(message.source_chain.clone(), message.nonce);

        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep_xchain_msg",
                "version": "1.0.0",
                "event": "receive_message",
                "data": {
                    "source_chain": message.source_chain,
                    "source_contract": message.source_contract,
                    "nonce": message.nonce,
                    "destination_contract": message.destination_contract,
                    "payload_hash": env::sha256(&message.payload)
                }
            })
        ));
    }
}
