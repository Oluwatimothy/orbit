//! NFT-gated token sale: holder window enforced by live ownership, per-NFT
//! limits, creator paid on delivery, and a public window with per-account limits.

mod common;

use anyhow::Result;
use common::*;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::NearToken;
use near_workspaces::Account;
use serde_json::{json, Value};

const TOKEN: u128 = 10u128.pow(18);
const SUPPLY: u128 = 1_000_000 * TOKEN;
const FOR_SALE: u128 = 100 * TOKEN;
const PRICE: NearToken = NearToken::from_millinear(100); // per whole token
const FAR_FUTURE_NS: u64 = 4_000_000_000_000_000_000;

/// `public_starts_at_ns: Some(..)` with a time in the past puts the sale straight into its public window.
async fn create_sale_launch(env: &Env, slug: &str, public: Option<(u64, u128)>) -> Result<()> {
    let sale_account = launch_account(env, slug, "sale");
    let deposit = quote(
        &view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": "connected" }))
            .await?,
    )?;
    let (holder_start, public_start, public_limit) = match public {
        Some((start, limit)) => (1u64, Value::from(start.to_string()), Value::from(limit.to_string())),
        None => (0, Value::Null, Value::Null),
    };
    let result = env
        .creator
        .call(env.factory.id(), "create_connected_launch")
        .args_json(json!({
            "slug": slug,
            "description": "Gated sale",
            "token": {
                "name": "Gated", "symbol": "GATE", "decimals": 18, "icon": null,
                "total_supply": SUPPLY.to_string(),
                "allocations": [
                    { "account_id": env.creator.id(), "amount": (SUPPLY - FOR_SALE).to_string(), "label": "Creator" },
                    { "account_id": sale_account, "amount": FOR_SALE.to_string(), "label": "Holder sale" },
                ],
            },
            "collection": {
                "name": "Pass", "symbol": "PASS", "description": "Access pass",
                "cover_media": "ipfs://bafycover",
                "media": { "kind": "shared", "media": "ipfs://bafycover", "media_hash": null },
                "supply": { "kind": "fixed", "max": 10 },
                "price": "0", "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": null, "public_mint": true,
            },
            "claim": null,
            "sale": {
                "price_per_token": PRICE.as_yoctonear().to_string(),
                "total_for_sale": FOR_SALE.to_string(),
                "per_nft_limit": (10 * TOKEN).to_string(),
                "holder_starts_at_ns": holder_start.to_string(),
                "public_starts_at_ns": public_start,
                "per_account_public_limit": public_limit,
                "ends_at_ns": FAR_FUTURE_NS.to_string(),
            },
        }))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)
}

async fn buy(who: &Account, sale: &str, amount: u128, nft: Option<&str>) -> Result<ExecutionFinalResult> {
    // Price for `amount` plus a 0.01 NEAR margin covering token registration; the rest is refunded.
    let cost = PRICE.as_yoctonear() * (amount / TOKEN);
    Ok(who
        .call(&sale.parse()?, "buy")
        .args_json(json!({ "amount": amount.to_string(), "nft_token_id": nft }))
        .deposit(NearToken::from_yoctonear(cost).saturating_add(NearToken::from_millinear(10)))
        .gas(MAX_GAS)
        .transact()
        .await?)
}

#[tokio::test]
async fn holder_window_is_enforced_by_ownership() -> Result<()> {
    let env = setup().await?;
    create_sale_launch(&env, "gated", None).await?;
    let (ft, nft, sale) = (
        launch_account(&env, "gated", "ft"),
        launch_account(&env, "gated", "nft"),
        launch_account(&env, "gated", "sale"),
    );
    let w = &env.worker;
    let info = view(w, &sale, "get_info", json!({})).await?;
    assert_eq!((info["status"].as_str(), info["phase"].as_str()), (Some("active"), Some("holders")));

    let mint = env
        .alice
        .call(&nft.parse()?, "nft_mint")
        .args_json(json!({ "quantity": 1 }))
        .deposit(NearToken::from_millinear(50))
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&mint)?;

    // Alice buys 4 tokens against NFT #1; the creator receives exactly the price.
    let creator_before = env.creator.view_account().await?.balance;
    let result = buy(&env.alice, &sale, 4 * TOKEN, Some("1")).await?;
    expect_success(&result)?;
    assert!(result.json::<bool>()?);
    assert_eq!(ft_balance(w, &ft, env.alice.id().as_str()).await?, 4 * TOKEN);
    let received = env.creator.view_account().await?.balance.saturating_sub(creator_before);
    assert!(
        received >= PRICE.saturating_mul(4)
            && received < PRICE.saturating_mul(4).saturating_add(NearToken::from_millinear(20))
    );

    // Bob does not own NFT #1: rolled back, nothing delivered, limit untouched.
    let not_owner = buy(&env.bob, &sale, TOKEN, Some("1")).await?;
    expect_success(&not_owner)?;
    assert!(!not_owner.json::<bool>()?);
    assert_eq!(ft_balance(w, &ft, env.bob.id().as_str()).await?, 0);
    assert_eq!(
        view(w, &sale, "get_purchased_by_nft", json!({ "nft_token_id": "1" })).await?,
        json!((4 * TOKEN).to_string())
    );

    // Per-NFT limit and the NFT requirement.
    let over = buy(&env.alice, &sale, 7 * TOKEN, Some("1")).await?;
    assert!(
        failure_text(&over).contains("can buy at most 6000000000000000000 more"),
        "{}",
        failure_text(&over)
    );
    let no_nft = buy(&env.alice, &sale, TOKEN, None).await?;
    assert!(failure_text(&no_nft).contains("requires an NFT ID"));

    let info = view(w, &sale, "get_info", json!({})).await?;
    assert_eq!((info["sold"].clone(), info["pending"].clone()), (json!((4 * TOKEN).to_string()), json!("0")));
    assert_eq!(ft_balance(w, &ft, &sale).await?, FOR_SALE - 4 * TOKEN);
    Ok(())
}

#[tokio::test]
async fn public_window_limits_each_account() -> Result<()> {
    let env = setup().await?;
    create_sale_launch(&env, "open", Some((2, 5 * TOKEN))).await?;
    let (ft, sale) = (launch_account(&env, "open", "ft"), launch_account(&env, "open", "sale"));
    let info = view(&env.worker, &sale, "get_info", json!({})).await?;
    assert_eq!(info["phase"], "public");

    expect_success(&buy(&env.bob, &sale, 5 * TOKEN, None).await?)?;
    assert_eq!(ft_balance(&env.worker, &ft, env.bob.id().as_str()).await?, 5 * TOKEN);
    let over = buy(&env.bob, &sale, TOKEN, None).await?;
    assert!(failure_text(&over).contains("this account can buy at most 0 more"));

    let underpaid = env
        .alice
        .call(&sale.parse()?, "buy")
        .args_json(json!({ "amount": TOKEN.to_string(), "nft_token_id": null }))
        .deposit(PRICE)
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(failure_text(&underpaid).contains("attach at least"));
    Ok(())
}
