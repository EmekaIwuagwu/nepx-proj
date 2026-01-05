# 🌈 NEP-XChain

## Universal Bridged Asset & Messaging Standard for NEAR Protocol

> **A canonical standard for representing cross-chain assets and messages on NEAR, enabling seamless interoperability across blockchain ecosystems.**

---

## � What is NEP-XChain?

NEP-XChain solves the **fragmentation problem** in cross-chain bridging by establishing:

1. **Canonical Asset Identity** - One standard way to identify any asset from any chain
2. **Standardized Bridge Interface** - Common API for all bridge implementations
3. **Receipt Token Standard** - NEP-141 tokens with cross-chain metadata
4. **Unified Event Format** - NEP-297 compliant events for indexers

### The Problem We Solve

| Problem | NEP-XChain Solution |
|---------|---------------------|
| Same asset, different tokens per bridge | `CanonicalAssetId` uniquely identifies assets |
| Unclear security assumptions | `SecurityModel` enum transparently declares trust model |
| Fragmented indexing | NEP-297 events with standard schema |
| No cross-chain metadata | `XChainReceiptToken` trait extends NEP-141 |

---

## � Contracts

| Contract | Description | WASM Size |
|----------|-------------|-----------|
| `xchain-bridge` | Main bridge with factory pattern | ~250KB |
| `xchain-token` | NEP-141 receipt token | ~180KB |
| `xchain-messaging` | Generic message passing | ~150KB |
| `xchain-core` | Shared types (library) | N/A |

---

## �️ Build & Deploy

### Prerequisites

```bash
rustup target add wasm32-unknown-unknown
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release \
  -p xchain-bridge -p xchain-token -p xchain-messaging
```

### Deploy to NEAR Testnet

```bash
# Create accounts
near create-account xchain-bridge.testnet --masterAccount your-account.testnet

# Deploy bridge
near deploy xchain-bridge.testnet ./target/wasm32-unknown-unknown/release/xchain_bridge.wasm

# Initialize
near call xchain-bridge.testnet new '{"owner_id":"your-account.testnet","initial_validators":[]}' --accountId your-account.testnet
```

---

## 📚 API Reference

### XChainBridge

---

#### `new` - Initialize Contract

```bash
near call xchain-bridge.testnet new \
  '{"owner_id":"admin.testnet","initial_validators":[]}' \
  --accountId admin.testnet
```

---

#### `bridge_in` - Import Asset from External Chain

**Parameters:**
```json
{
  "proof": {
    "source_tx_hash": "0xabc123...",
    "proof_data": [...],
    "block_height": 18500000
  },
  "receiver_id": "alice.testnet"
}
```

**Example Call:**
```bash
near call xchain-bridge.testnet bridge_in \
  '{"proof":{"source_tx_hash":"0xabc","proof_data":[...],"block_height":1000},"receiver_id":"alice.testnet"}' \
  --accountId relayer.testnet \
  --deposit 5 \
  --gas 100000000000000
```

**Success Response (Event Log):**
```json
{
  "standard": "nep_xchain",
  "version": "1.0.0",
  "event": "bridge_in",
  "data": {
    "canonical_asset": {
      "source_chain_id": "ethereum:1",
      "source_contract": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
      "asset_standard": "ERC20"
    },
    "amount": "1000000000000000000",
    "receiver_id": "alice.testnet",
    "source_tx_hash": "0xabc123...",
    "receipt_token": "a1b2c3d4.xchain-bridge.testnet"
  }
}
```

---

#### `bridge_out` - Export Asset to External Chain

**Parameters:**
```json
{
  "token_id": "a1b2c3d4.xchain-bridge.testnet",
  "amount": "1000000000000000000",
  "destination_chain": "ethereum:1",
  "destination_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f1dE3b"
}
```

**Example Call:**
```bash
near call xchain-bridge.testnet bridge_out \
  '{"token_id":"a1b2c3d4.xchain-bridge.testnet","amount":"1000000000","destination_chain":"ethereum:1","destination_address":"0x742d35Cc..."}' \
  --accountId alice.testnet \
  --deposit 0.1 \
  --gas 50000000000000
```

**Fee:** 0.1 NEAR (sent to treasury)

---

#### `get_canonical_asset` - View Asset Info

```bash
near view xchain-bridge.testnet get_canonical_asset \
  '{"token_id":"a1b2c3d4.xchain-bridge.testnet"}'
```

**Response:**
```json
{
  "source_chain_id": "ethereum:1",
  "source_contract": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  "asset_standard": "ERC20"
}
```

---

#### `estimate_fee` - Get Bridge Fee

```bash
near view xchain-bridge.testnet estimate_fee \
  '{"direction":"Out","asset":null,"amount":"1000000000"}'
```

**Response:**
```json
"100000000000000000000000"
```
> 0.1 NEAR in yoctoNEAR

---

#### `is_paused` - Check Pause Status

```bash
near view xchain-bridge.testnet is_paused
```

**Response:**
```json
false
```

---

#### `set_paused` - Emergency Stop (Owner Only)

```bash
near call xchain-bridge.testnet set_paused '{"paused":true}' \
  --accountId admin.testnet
```

---

#### `add_validator` / `remove_validator` - Manage Committee

```bash
near call xchain-bridge.testnet add_validator \
  '{"public_key":"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"}' \
  --accountId admin.testnet
```

---

### XChainReceiptToken (NEP-141 Extension)

Receipt tokens are automatically deployed by the bridge and implement:

| Standard | Methods |
|----------|---------|
| **NEP-141** | `ft_transfer`, `ft_transfer_call`, `ft_total_supply`, `ft_balance_of` |
| **NEP-148** | `ft_metadata` |
| **XChain** | `xc_metadata`, `xc_is_bridgeable_to`, `xc_total_locked` |

---

#### `xc_metadata` - Get Cross-Chain Origin

```bash
near view a1b2c3d4.xchain-bridge.testnet xc_metadata
```

**Response:**
```json
{
  "canonical_asset": {
    "source_chain_id": "ethereum:1",
    "source_contract": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "asset_standard": "ERC20"
  },
  "bridge_route": {
    "bridge_contract": "xchain-bridge.testnet",
    "security_model": {"Committee": {"threshold": 2, "size": 3}},
    "deployed_at": 1704470400000000000
  },
  "total_locked": "5000000000000000000000",
  "nep141_token": "a1b2c3d4.xchain-bridge.testnet"
}
```

---

#### `ft_balance_of` - Check Balance

```bash
near view a1b2c3d4.xchain-bridge.testnet ft_balance_of \
  '{"account_id":"alice.testnet"}'
```

**Response:**
```json
"1000000000000000000"
```

---

### XChainMessenger

---

#### `send_message` - Send Cross-Chain Message

```bash
near call xchain-messenger.testnet send_message \
  '{"destination_chain":"ethereum:1","destination_contract":"0x123...","payload":[72,101,108,108,111]}' \
  --accountId alice.testnet
```

**Event Emitted:**
```json
{
  "standard": "nep_xchain_msg",
  "version": "1.0.0",
  "event": "send_message",
  "data": {
    "destination_chain": "ethereum:1",
    "destination_contract": "0x123...",
    "nonce": "1",
    "sender": "alice.testnet",
    "payload_hash": [...]
  }
}
```

---

## 🔐 Security

### Access Control Matrix

| Function | Who Can Call |
|----------|--------------|
| `bridge_in` | Anyone with valid proof |
| `bridge_out` | Token holder (requires fee) |
| `set_paused` | Contract owner |
| `add_validator` | Contract owner |
| `remove_validator` | Contract owner |
| `set_receipt_token_code` | Contract owner |
| `mint` | Bridge contract only |
| `burn` | Bridge contract only |

### Security Model Types

```rust
enum SecurityModel {
    Optimistic { challenge_period_sec: u64 },  // Rainbow Bridge style
    ZeroKnowledge { proof_system: String },    // ZK proof verification
    Committee { threshold: u8, size: u8 },     // Multi-sig committee
    Trusted { operator: AccountId },           // Single trusted operator
}
```

---

## 📊 Gas Requirements

| Operation | Gas (TGas) | Notes |
|-----------|------------|-------|
| `bridge_in` (new token) | ~100 | Deploys new receipt token |
| `bridge_in` (existing) | ~20 | Mints to existing token |
| `bridge_out` | ~15 | Burns tokens |
| `ft_transfer` | ~10 | Standard NEP-141 |
| `send_message` | ~10 | Message emission |

---

## � Core Data Types

### CanonicalAssetId

```rust
struct CanonicalAssetId {
    source_chain_id: String,      // "ethereum:1", "solana:mainnet"
    source_contract: String,      // "0xA0b86991c..." or "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    asset_standard: AssetStandard // ERC20, ERC721, SPL, Native, etc.
}
```

### BridgeProof

```rust
struct BridgeProof {
    source_tx_hash: String,  // Transaction hash on source chain
    proof_data: Vec<u8>,     // Borsh-encoded BridgePayload
    block_height: u64        // Block height on source chain
}
```

### BridgePayload

```rust
struct BridgePayload {
    nonce: U128,
    source_chain: String,
    target_chain: String,
    asset: CanonicalAssetId,
    amount: U128,
    receiver: AccountId,
    source_tx_hash: String,
}
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test -p integration-tests
```

---

## � Checklist for Production

- [x] No hardcoded values
- [x] No TODO/FIXME comments
- [x] All functions implemented
- [x] NEP-297 events emitted
- [x] Access control enforced
- [x] Pausable mechanism
- [x] Fee collection
- [ ] External audit
- [ ] Mainnet deployment

---

## � License

MIT

---

**Built for the NEAR Ecosystem** 🌈
#   n e p x - p r o j  
 