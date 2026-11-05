//! Token and collection launches, and deployment failure handling.

mod common;

use anyhow::Result;
use common::*;
use near_workspaces::types::NearToken;
use serde_json::json;

fn token_args(env: &Env, slug: &str) -> serde_json::Value {
    json!({
        "slug": slug,
        "description": "Treasury token",
        "token": {
            "name": "Treasury", "symbol": "TRSY", "decimals": 6, "icon": null,
            "total_supply": "1000000000",
            "allocations": [
                { "account_id": env.creator.id(), "amount": "600000000", "label": "Creator" },
                { "account_id": env.alice.id(), "amount": "400000000", "label": "Community" },
            ],
        },
    })
}

async fn token_quote(env: &Env) -> Result<NearToken> {
    quote(&view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": "token" })).await?)
}

#[tokio::test]
async fn token_launch_distributes_allocations() -> Result<()> {
    let env = setup().await?;
    let result = env
        .creator
        .call(env.factory.id(), "create_token_launch")
        .args_json(token_args(&env, "treasury"))
        .deposit(token_quote(&env).await?)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)?;

    let ft = launch_account(&env, "treasury", "ft");
    assert_eq!(ft_balance(&env.worker, &ft, env.creator.id().as_str()).await?, 600_000_000);
    assert_eq!(ft_balance(&env.worker, &ft, env.alice.id().as_str()).await?, 400_000_000);
    let allocations = view(&env.worker, &ft, "orbit_allocations", json!({})).await?;
    assert_eq!(allocations.as_array().map(Vec::len), Some(2));

    let by_creator = view(
        &env.worker,
        env.factory.id().as_str(),
        "get_launches_by_creator",
        json!({ "account_id": env.creator.id() }),
    )
    .await?;
    assert_eq!(by_creator[0]["status"], "live");
    assert_eq!(by_creator[0]["kind"], "token");

    // NEP-330 source metadata is exposed.
    let source = view(&env.worker, &ft, "contract_source_metadata", json!({})).await?;
    assert!(source["standards"].as_array().is_some_and(|s| s.iter().any(|x| x["standard"] == "nep141")));
    Ok(())
}

#[tokio::test]
async fn collection_launch_enforces_payment() -> Result<()> {
    let env = setup().await?;
    let deposit = quote(
        &view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": "collection" }))
            .await?,
    )?;
    let result = env
        .creator
        .call(env.factory.id(), "create_collection_launch")
        .args_json(json!({
            "slug": "art",
            "collection": {
                "name": "Art", "symbol": "ART", "description": "Open edition",
                "cover_media": "ipfs://bafycover",
                "media": { "kind": "shared", "media": "ipfs://bafymedia", "media_hash": null },
                "supply": { "kind": "unlimited" },
                "price": NearToken::from_near(1).as_yoctonear().to_string(),
                "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": 2, "public_mint": true,
            },
        }))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)?;
    let nft = launch_account(&env, "art", "nft");

    let underpaid = env
        .alice
        .call(&nft.parse()?, "nft_mint")
        .args_json(json!({ "quantity": 1 }))
        .deposit(NearToken::from_millinear(999))
        .transact()
        .await?;
    assert!(failure_text(&underpaid).contains("is less than the required"));
    assert_eq!(view(&env.worker, &nft, "nft_total_supply", json!({})).await?, json!("0"));

    let alice_before = env.alice.view_account().await?.balance;
    let paid = env
        .alice
        .call(&nft.parse()?, "nft_mint")
        .args_json(json!({ "quantity": 2 }))
        .deposit(NearToken::from_near(3))
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&paid)?;
    let spent = alice_before.saturating_sub(env.alice.view_account().await?.balance);
    assert!(spent < NearToken::from_millinear(2_100), "excess deposit refunded, spent {spent}");

    let over_limit = env
        .alice
        .call(&nft.parse()?, "nft_mint")
        .args_json(json!({ "quantity": 1 }))
        .deposit(NearToken::from_near(2))
        .transact()
        .await?;
    assert!(failure_text(&over_limit).contains("may mint at most 2"));

    let owned =
        view(&env.worker, &nft, "nft_tokens_for_owner", json!({ "account_id": env.alice.id() })).await?;
    assert_eq!(owned.as_array().map(Vec::len), Some(2));
    Ok(())
}

#[tokio::test]
async fn failed_deployment_is_recorded_and_refunded() -> Result<()> {
    let env = setup().await?;
    // Point new token launches at code that was never published.
    let config = view(&env.worker, env.factory.id().as_str(), "get_config", json!({})).await?;
    let mut code = config["code"].clone();
    code["token"] = json!("11111111111111111111111111111111");
    let set = env
        .owner
        .call(env.factory.id(), "set_code_hashes")
        .args_json(json!({ "code": code }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    expect_success(&set)?;

    let deposit = token_quote(&env).await?;
    let before = env.creator.view_account().await?.balance;
    let result = env
        .creator
        .call(env.factory.id(), "create_token_launch")
        .args_json(token_args(&env, "broken"))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(!result.receipt_failures().is_empty(), "the deployment receipt fails");

    let launch = view(&env.worker, env.factory.id().as_str(), "get_launch", json!({ "id": 0 })).await?;
    assert_eq!(launch["status"], "failed");
    assert!(
        env.worker.view_account(&launch_account(&env, "broken", "ft").parse()?).await.is_err(),
        "account not created"
    );
    let spent = before.saturating_sub(env.creator.view_account().await?.balance);
    assert!(spent < NearToken::from_millinear(50), "account balance refunded, spent {spent}");
    Ok(())
}

#[tokio::test]
async fn non_owner_cannot_reconfigure_factory() -> Result<()> {
    let env = setup().await?;
    let result = env
        .alice
        .call(env.factory.id(), "set_paused")
        .args_json(json!({ "paused": true }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    assert!(failure_text(&result).contains("only the factory owner"));
    Ok(())
}

/// The web app attaches 0.02 NEAR per NFT as a storage allowance
/// (`MINT_STORAGE_ALLOWANCE`); this keeps that figure honest.
#[tokio::test]
async fn mint_storage_fits_the_web_allowance() -> Result<()> {
    // 0.02 NEAR per NFT, minus the 0.0025 NEAR transfer reserve, at 1e19 yoctoNEAR per byte.
    const WEB_ALLOWANCE_BYTES: u64 = 1_750;
    let env = setup().await?;
    let deposit = quote(
        &view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": "collection" }))
            .await?,
    )?;
    let long_media = format!("ipfs://bafy{}", "a".repeat(400));
    let result = env
        .creator
        .call(env.factory.id(), "create_collection_launch")
        .args_json(json!({
            "slug": "storage",
            "collection": {
                "name": "A collection with a fairly long name for measurement", "symbol": "STORE", "description": "Storage",
                "cover_media": long_media,
                "media": { "kind": "shared", "media": long_media, "media_hash": null },
                "supply": { "kind": "unlimited" }, "price": "0",
                "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": null, "public_mint": true,
            },
        }))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)?;
    let nft: near_workspaces::AccountId = launch_account(&env, "storage", "nft").parse()?;

    let mut previous = env.worker.view_account(&nft).await?.storage_usage;
    for quantity in [1u64, 10] {
        let mint = env
            .alice
            .call(&nft, "nft_mint")
            .args_json(json!({ "quantity": quantity }))
            .deposit(NearToken::from_millinear(20).saturating_mul(u128::from(quantity)))
            .gas(MAX_GAS)
            .transact()
            .await?;
        expect_success(&mint)?;
        let usage = env.worker.view_account(&nft).await?.storage_usage;
        let per_token = (usage - previous) / quantity;
        println!("mint of {quantity}: {per_token} bytes per NFT");
        assert!(per_token < WEB_ALLOWANCE_BYTES, "{per_token} bytes per NFT exceeds the web allowance");
        previous = usage;
    }
    Ok(())
}
