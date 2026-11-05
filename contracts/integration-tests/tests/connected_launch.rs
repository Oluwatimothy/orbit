//! The full connected-launch journey: factory → one launch account with token,
//! collection and holder claims → mint → claim → duplicate and non-owner
//! rejection → transfer → claim.

mod common;

use anyhow::Result;
use common::*;
use near_workspaces::types::NearToken;
use serde_json::json;
use sha2::{Digest, Sha256};

const DECIMALS: u32 = 18;
const SUPPLY: u128 = 10_000_000 * 10u128.pow(DECIMALS);
const GENESIS: u128 = 1_000 * 10u128.pow(DECIMALS);
const FOUNDERS: u128 = 5_000 * 10u128.pow(DECIMALS);
// NFT supply 5: IDs 1–4 Genesis, ID 5 Founders.
const RESERVED: u128 = 4 * GENESIS + FOUNDERS;
const MINT_PRICE: NearToken = NearToken::from_millinear(100);

async fn create_connected(env: &Env, slug: &str) -> Result<()> {
    let launch = launch_account(env, slug);
    let result = env
        .creator
        .call(env.factory.id(), "create_connected_launch")
        .args_json(json!({
            "slug": slug,
            "description": "Orbit Genesis connected launch",
            "token": {
                "name": "Orbit Genesis", "symbol": "ORBIT", "decimals": DECIMALS, "icon": null,
                "total_supply": SUPPLY.to_string(),
                "allocations": [
                    { "account_id": env.creator.id(), "amount": (SUPPLY - RESERVED).to_string(), "label": "Creator" },
                    { "account_id": launch, "amount": RESERVED.to_string(), "label": "NFT holder claims" },
                ],
            },
            "collection": {
                "name": "Genesis", "symbol": "GEN", "description": "Genesis pass",
                "cover_media": "ipfs://bafycover",
                "media": { "kind": "shared", "media": "ipfs://bafymedia", "media_hash": null },
                "supply": { "kind": "fixed", "max": 5 },
                "price": MINT_PRICE.as_yoctonear().to_string(),
                "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": null, "public_mint": true,
            },
            "claim": {
                "tiers": [
                    { "name": "Genesis", "from": 1, "to": 4, "amount_per_nft": GENESIS.to_string() },
                    { "name": "Founders", "from": 5, "to": 5, "amount_per_nft": FOUNDERS.to_string() },
                ],
                "starts_at_ns": "0",
            },
            "sale": null,
        }))
        .deposit(quote_for(env, "connected").await?)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)
}

async fn mint(
    who: &near_workspaces::Account,
    nft: &str,
    quantity: u32,
) -> Result<near_workspaces::result::ExecutionFinalResult> {
    let deposit = MINT_PRICE
        .saturating_mul(u128::from(quantity))
        .saturating_add(NearToken::from_millinear(20).saturating_mul(u128::from(quantity)));
    Ok(who
        .call(&nft.parse()?, "nft_mint")
        .args_json(json!({ "quantity": quantity }))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?)
}

/// Attaches 0.005 NEAR, as the web app does; the contract keeps only the storage it adds.
async fn claim(
    who: &near_workspaces::Account,
    launch: &str,
    id: &str,
) -> Result<near_workspaces::result::ExecutionFinalResult> {
    Ok(who
        .call(&launch.parse()?, "claim")
        .args_json(json!({ "nft_token_id": id }))
        .deposit(NearToken::from_millinear(5))
        .gas(MAX_GAS)
        .transact()
        .await?)
}

#[tokio::test]
async fn connected_launch_end_to_end() -> Result<()> {
    let env = setup().await?;
    create_connected(&env, "genesis").await?;
    let launch_id = launch_account(&env, "genesis");
    let launch_ref = launch_id.as_str();
    let w = &env.worker;

    // Registry, deployed code, metadata, supply and reserve.
    let launch = view(w, env.factory.id().as_str(), "get_launch", json!({ "id": 0 })).await?;
    assert_eq!(launch["status"], "live");
    for part in ["account", "token", "nft", "claim"] {
        assert_eq!(launch[part], launch_ref, "{part}");
    }
    assert_eq!(launch["sale"], json!(null));
    let code = w.view_code(&launch_ref.parse()?).await?;
    assert!(code == wasm("orbit_launch")?, "the launch runs the factory's template");
    let config = view(w, env.factory.id().as_str(), "get_config", json!({})).await?;
    assert_eq!(config["template"]["hash"], bs58::encode(Sha256::digest(&code)).into_string());

    let metadata = view(w, launch_ref, "ft_metadata", json!({})).await?;
    assert_eq!((metadata["symbol"].as_str(), metadata["decimals"].as_u64()), (Some("ORBIT"), Some(18)));
    assert_eq!(view(w, launch_ref, "nft_metadata", json!({})).await?["symbol"], "GEN");
    assert_eq!(view(w, launch_ref, "ft_total_supply", json!({})).await?, json!(SUPPLY.to_string()));
    assert_eq!(ft_balance(w, launch_ref, env.creator.id().as_str()).await?, SUPPLY - RESERVED);
    assert_eq!(ft_balance(w, launch_ref, launch_ref).await?, RESERVED);
    let info = view(w, launch_ref, "claim_info", json!({})).await?;
    assert_eq!(info["total_reserved"], json!(RESERVED.to_string()));

    // The launch account is keyless.
    assert!(w.view_access_keys(&launch_ref.parse()?).await?.is_empty(), "launch account must have no keys");

    // Alice mints 4 (IDs 1–4), Bob mints the last one (ID 5); a sixth is rejected.
    // Let gas refunds from the creator's launch transaction land first.
    w.fast_forward(5).await?;
    let creator_before = env.creator.view_account().await?.balance;
    expect_success(&mint(&env.alice, launch_ref, 4).await?)?;
    expect_success(&mint(&env.bob, launch_ref, 1).await?)?;
    let over = mint(&env.alice, launch_ref, 1).await?;
    assert!(failure_text(&over).contains("only 0 tokens remain"), "{}", failure_text(&over));
    let token = view(w, launch_ref, "nft_token", json!({ "token_id": "5" })).await?;
    assert_eq!(token["owner_id"], env.bob.id().as_str());
    let received = env.creator.view_account().await?.balance.saturating_sub(creator_before);
    let proceeds = MINT_PRICE.saturating_mul(5);
    assert!(
        received >= proceeds && received < proceeds.saturating_add(NearToken::from_millinear(20)),
        "creator received {received}"
    );

    // Alice claims NFT #1 and keeps most of her deposit.
    let alice_near = env.alice.view_account().await?.balance;
    let result = claim(&env.alice, launch_ref, "1").await?;
    expect_success(&result)?;
    assert_eq!(result.json::<String>()?, GENESIS.to_string());
    assert_eq!(ft_balance(w, launch_ref, env.alice.id().as_str()).await?, GENESIS);
    let spent = alice_near.saturating_sub(env.alice.view_account().await?.balance);
    assert!(spent < NearToken::from_millinear(5), "storage charged, excess refunded; spent {spent}");

    // Duplicate claim of #1 is rejected — by Alice and by anyone else.
    let duplicate = claim(&env.alice, launch_ref, "1").await?;
    assert!(failure_text(&duplicate).contains("already been claimed"));
    let by_bob = claim(&env.bob, launch_ref, "1").await?;
    assert!(failure_text(&by_bob).contains("already been claimed"));

    // Bob cannot claim #2, which Alice owns: nothing moves.
    let not_owner = claim(&env.bob, launch_ref, "2").await?;
    assert!(failure_text(&not_owner).contains("you do not own this NFT"), "{}", failure_text(&not_owner));
    assert_eq!(ft_balance(w, launch_ref, env.bob.id().as_str()).await?, 0);
    assert_eq!(view(w, launch_ref, "claim_record", json!({ "nft_token_id": "2" })).await?, json!(null));

    // After Alice transfers #2 to Bob, Bob can claim it.
    let transfer = env
        .alice
        .call(&launch_ref.parse()?, "nft_transfer")
        .args_json(json!({ "receiver_id": env.bob.id(), "token_id": "2" }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    expect_success(&transfer)?;
    expect_success(&claim(&env.bob, launch_ref, "2").await?)?;

    // Bob claims the Founders tier NFT.
    expect_success(&claim(&env.bob, launch_ref, "5").await?)?;

    // Final balances add up exactly.
    let alice = ft_balance(w, launch_ref, env.alice.id().as_str()).await?;
    let bob = ft_balance(w, launch_ref, env.bob.id().as_str()).await?;
    assert_eq!(alice, GENESIS);
    assert_eq!(bob, GENESIS + FOUNDERS);
    assert_eq!(ft_balance(w, launch_ref, launch_ref).await?, RESERVED - alice - bob);
    let info = view(w, launch_ref, "claim_info", json!({})).await?;
    assert_eq!(info["claimed_total"], json!((alice + bob).to_string()));
    assert_eq!(info["claims_count"], json!(3));
    assert_eq!(view(w, launch_ref, "ft_total_supply", json!({})).await?, json!(SUPPLY.to_string()));

    // The account's balance still covers its storage: nobody's activity was paid for by the launch.
    let account = w.view_account(&launch_ref.parse()?).await?;
    let locked = NearToken::from_yoctonear(u128::from(account.storage_usage) * 10u128.pow(19));
    assert!(account.balance >= locked, "balance {} below storage {}", account.balance, locked);
    Ok(())
}

#[tokio::test]
async fn callbacks_are_private() -> Result<()> {
    let env = setup().await?;
    create_connected(&env, "genesis").await?;
    let forged_launch = env
        .alice
        .call(env.factory.id(), "on_launch_deployed")
        .args_json(json!({ "launch_id": 0, "balance": "1000000000000000000000000" }))
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(failure_text(&forged_launch).contains("is private"));

    let forged_resolve = env
        .alice
        .call(&launch_account(&env, "genesis").parse()?, "ft_resolve_transfer")
        .args_json(json!({ "sender_id": env.alice.id(), "receiver_id": env.bob.id(), "amount": "1" }))
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(failure_text(&forged_resolve).contains("is private"));
    Ok(())
}
