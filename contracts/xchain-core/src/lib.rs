use near_sdk::near;
use near_sdk::{AccountId, Promise, PublicKey, Timestamp};
use near_sdk::json_types::U128;

pub type ChainId = String;

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalAssetId {
    pub source_chain_id: ChainId,
    pub source_contract: String,
    pub asset_standard: AssetStandard,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub enum AssetStandard {
    Native,
    ERC20,
    ERC721,
    ERC1155,
    SPL,
    Custom(String),
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct BridgeRoute {
    pub bridge_contract: AccountId,
    pub security_model: SecurityModel,
    pub deployed_at: Timestamp,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub enum SecurityModel {
    Optimistic { challenge_period_sec: u64 },
    ZeroKnowledge { proof_system: String },
    Committee { threshold: u8, size: u8 },
    Trusted { operator: AccountId },
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct ReceiptTokenMetadata {
    pub canonical_asset: CanonicalAssetId,
    pub bridge_route: BridgeRoute,
    pub total_locked: U128,
    pub nep141_token: AccountId,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub enum BridgeDirection {
    In,
    Out,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct BridgeProof {
    pub source_tx_hash: String,
    pub proof_data: Vec<u8>,
    pub block_height: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct CommitteeProof {
    pub signatures: Vec<(PublicKey, Vec<u8>)>,
    pub message_hash: Vec<u8>,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct XChainMessage {
    pub nonce: U128,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub source_contract: String,
    pub destination_contract: String,
    pub payload: Vec<u8>,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct BridgePayload {
    pub nonce: U128,
    pub source_chain: ChainId,
    pub target_chain: ChainId,
    pub asset: CanonicalAssetId,
    pub amount: U128,
    pub receiver: AccountId,
    pub source_tx_hash: String,
}

pub trait XChainCore {
    fn bridge_in(
        &mut self,
        proof: BridgeProof,
        receiver_id: AccountId,
    ) -> Promise;

    fn bridge_out(
        &mut self,
        token_id: AccountId,
        amount: U128,
        destination_chain: ChainId,
        destination_address: String,
    ) -> Promise;

    fn get_canonical_asset(&self, token_id: AccountId) -> Option<CanonicalAssetId>;
    fn get_bridge_route(&self) -> BridgeRoute;
    
    fn estimate_fee(
        &self,
        direction: BridgeDirection,
        asset: Option<CanonicalAssetId>,
        amount: U128,
    ) -> U128;

    fn set_paused(&mut self, paused: bool);
    fn is_paused(&self) -> bool;
    
    fn add_validator(&mut self, public_key: PublicKey);
    fn remove_validator(&mut self, public_key: PublicKey);
}

pub trait XChainReceiptToken {
    fn xc_metadata(&self) -> ReceiptTokenMetadata;
    fn xc_is_bridgeable_to(&self, chain_id: ChainId) -> bool;
    fn xc_total_locked(&self) -> U128;
}

pub trait XChainMessaging {
    fn send_message(
        &mut self,
        destination_chain: ChainId,
        destination_contract: String,
        payload: Vec<u8>,
    ) -> Promise;

    fn receive_message(
        &mut self,
        proof: BridgeProof,
    );
}
