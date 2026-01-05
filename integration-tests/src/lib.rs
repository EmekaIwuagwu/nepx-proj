use near_workspaces::types::NearToken;
use serde_json::json;
use xchain_core::{AssetStandard, BridgePayload, CanonicalAssetId};
use near_sdk::json_types::U128;
use near_sdk::borsh::BorshSerialize;

#[tokio::test]
async fn test_full_bridge_flow() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.root_account()?;

    println!("Compiling and deploying bridge...");
    let bridge_wasm = near_workspaces::compile_project("./contracts/xchain-bridge").await?;
    let bridge = worker.dev_deploy(&bridge_wasm).await?;
    
    println!("Initializing bridge...");
    let init_result = bridge.call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "initial_validators": []
        }))
        .transact()
        .await?;
    assert!(init_result.is_success(), "Bridge init failed");

    println!("Compiling token...");
    let token_wasm = near_workspaces::compile_project("./contracts/xchain-token").await?;
    
    println!("Setting receipt token code...");
    let set_code_result = bridge.call("set_receipt_token_code")
        .args_borsh(token_wasm)
        .transact()
        .await?;
    assert!(set_code_result.is_success(), "Set code failed");

    println!("Creating test user...");
    let user = owner.create_subaccount("alice")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;

    println!("Preparing bridge payload...");
    let canonical_asset = CanonicalAssetId {
        source_chain_id: "ethereum:1".to_string(),
        source_contract: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        asset_standard: AssetStandard::ERC20,
    };
    
    let payload = BridgePayload {
        nonce: U128(1),
        source_chain: "ethereum:1".to_string(),
        target_chain: "near:testnet".to_string(),
        asset: canonical_asset,
        amount: U128(1_000_000_000),
        receiver: user.id().to_string().parse().unwrap(),
        source_tx_hash: "0xabc123def456".to_string(),
    };
    
    let proof_data = payload.try_to_vec().expect("Failed to serialize payload");

    println!("Executing bridge_in...");
    let bridge_in_result = bridge.call("bridge_in")
        .args_json(json!({
            "proof": {
                "source_tx_hash": "0xabc123def456",
                "proof_data": proof_data,
                "block_height": 18500000
            },
            "receiver_id": user.id()
        }))
        .deposit(NearToken::from_near(5))
        .max_gas()
        .transact()
        .await?;
    
    println!("Bridge in result: {:?}", bridge_in_result.logs());
    
    let has_bridge_event = bridge_in_result.logs().iter()
        .any(|log| log.contains("nep_xchain") && log.contains("bridge_in"));
    assert!(has_bridge_event, "Expected bridge_in event not found");

    println!("Test passed!");
    Ok(())
}

#[tokio::test]
async fn test_bridge_pause() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.root_account()?;

    let bridge_wasm = near_workspaces::compile_project("./contracts/xchain-bridge").await?;
    let bridge = worker.dev_deploy(&bridge_wasm).await?;
    
    bridge.call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "initial_validators": []
        }))
        .transact()
        .await?;

    let is_paused: bool = bridge.view("is_paused")
        .await?
        .json()?;
    assert!(!is_paused, "Bridge should not be paused initially");

    bridge.call("set_paused")
        .args_json(json!({"paused": true}))
        .transact()
        .await?;

    let is_paused: bool = bridge.view("is_paused")
        .await?
        .json()?;
    assert!(is_paused, "Bridge should be paused after set_paused(true)");

    println!("Pause test passed!");
    Ok(())
}

#[tokio::test]
async fn test_messaging_contract() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.root_account()?;

    let msg_wasm = near_workspaces::compile_project("./contracts/xchain-messaging").await?;
    let messenger = worker.dev_deploy(&msg_wasm).await?;
    
    messenger.call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "initial_validators": []
        }))
        .transact()
        .await?;

    let send_result = messenger.call("send_message")
        .args_json(json!({
            "destination_chain": "ethereum:1",
            "destination_contract": "0x1234567890abcdef",
            "payload": [1, 2, 3, 4, 5]
        }))
        .transact()
        .await?;
    
    let has_send_event = send_result.logs().iter()
        .any(|log| log.contains("nep_xchain_msg") && log.contains("send_message"));
    assert!(has_send_event, "Expected send_message event not found");

    println!("Messaging test passed!");
    Ok(())
}
