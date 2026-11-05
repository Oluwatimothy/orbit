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

    let ft = launch_account(&env, "treasury");
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
    let nft = launch_account(&env, "art");

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
    // An empty wasm module deploys but has no `new`, so the batch fails.
    let empty_module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    expect_success(&set_template(&env.factory, &env.owner, empty_module).await?)?;

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
        env.worker.view_account(&launch_account(&env, "broken").parse()?).await.is_err(),
        "account not created"
    );
    let spent = before.saturating_sub(env.creator.view_account().await?.balance);
    assert!(spent < NearToken::from_millinear(50), "account balance refunded, spent {spent}");
    Ok(())
}

#[tokio::test]
async fn non_owner_cannot_reconfigure_factory() -> Result<()> {
    let env = setup().await?;
    let template = set_template(&env.factory, &env.alice, wasm("orbit_launch")?).await?;
    assert!(failure_text(&template).contains("only the factory owner"));
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
    let nft: near_workspaces::AccountId = launch_account(&env, "storage").parse()?;

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

/// The largest launch the limits allow must fit `LAUNCH_STATE_ALLOWANCE`
/// (0.2 NEAR) on top of its code: 10 allocations, an 8 KiB icon, a
/// 1,000-character description, 512-character URLs, 10 tiers and a sale.
#[tokio::test]
async fn largest_launch_state_fits_the_allowance() -> Result<()> {
    const ALLOWANCE_BYTES: u64 = 20_000;
    let env = setup().await?;
    let launch = launch_account(&env, "largest");
    let token = 10u128.pow(18);
    let reserve = 10 * 1_000 * token + 1_000 * token;
    let others: Vec<_> = (0..9)
        .map(|i| json!({ "account_id": format!("holder-{i}-with-a-long-account-name.testnet"), "amount": token.to_string(), "label": "L".repeat(32) }))
        .collect();
    let mut allocations =
        vec![json!({ "account_id": launch, "amount": reserve.to_string(), "label": "L".repeat(32) })];
    allocations.extend(others);
    let icon = format!("data:image/png;base64,{}", "A".repeat(8_192 - 22));
    let url = format!("https://example.com/{}", "a".repeat(512 - 20));
    let tiers: Vec<_> = (0..10)
        .map(|i| json!({ "name": "T".repeat(32), "from": i * 10 + 1, "to": i * 10 + 10, "amount_per_nft": (100 * token).to_string() }))
        .collect();

    let result = env
        .creator
        .call(env.factory.id(), "create_connected_launch")
        .args_json(json!({
            "slug": "largest",
            "description": "D".repeat(1_000),
            "token": {
                "name": "N".repeat(64), "symbol": "SYMBOLSYMBOL", "decimals": 18, "icon": icon,
                "total_supply": (reserve + 9 * token).to_string(),
                "allocations": allocations,
            },
            "collection": {
                "name": "N".repeat(64), "symbol": "SYMBOLSYMBOL", "description": "D".repeat(1_000),
                "cover_media": url,
                "media": { "kind": "shared", "media": url, "media_hash": null },
                "supply": { "kind": "fixed", "max": 100 }, "price": "0",
                "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": null, "public_mint": true,
            },
            "claim": { "tiers": tiers, "starts_at_ns": "0" },
            "sale": {
                "price_per_token": "1000000000000000000000", "total_for_sale": (1_000 * token).to_string(),
                "per_nft_limit": (10 * token).to_string(), "holder_starts_at_ns": "0",
                "public_starts_at_ns": "1", "per_account_public_limit": token.to_string(),
                "ends_at_ns": "4000000000000000000",
            },
        }))
        .deposit(quote_for(&env, "connected").await?)
        .gas(MAX_GAS)
        .transact()
        .await?;
    expect_success(&result)?;
    println!("connected launch burnt {} Tgas", result.total_gas_burnt.as_tgas());

    let code = wasm("orbit_launch")?.len() as u64;
    let account = env.worker.view_account(&launch.parse()?).await?;
    let state = account.storage_usage - code;
    println!("largest launch state: {state} bytes on top of {code} bytes of code");
    assert!(state < ALLOWANCE_BYTES, "{state} bytes exceeds the state allowance");
    Ok(())
}

/// A failed launch refunds the creator from the factory's balance in the
/// deployment callback. On testnet the factory's spare balance is smaller than
/// one launch account's, so this relies on the failed batch's deposit
/// returning to the factory before the callback runs. Checked here with a
/// factory drained to 0.5 NEAR of spare balance and a ~5 NEAR launch account.
#[tokio::test]
async fn failed_launch_refund_does_not_need_spare_factory_balance() -> Result<()> {
    let env = setup().await?;
    expect_success(&set_template(&env.factory, &env.owner, module_without_new(500_000)).await?)?;

    let factory = env.factory.as_account();
    let details = factory.view_account().await?;
    let locked = NearToken::from_yoctonear(u128::from(details.storage_usage) * 10u128.pow(19));
    let spare = details.balance.saturating_sub(locked).saturating_sub(NearToken::from_millinear(500));
    factory.transfer_near(env.owner.id(), spare).await?.into_result()?;

    let deposit = token_quote(&env).await?;
    assert!(
        deposit > NearToken::from_near(5),
        "the launch account balance exceeds the factory's spare balance"
    );
    let before = env.creator.view_account().await?.balance;
    let result = env
        .creator
        .call(env.factory.id(), "create_token_launch")
        .args_json(token_args(&env, "refunded"))
        .deposit(deposit)
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(!result.receipt_failures().is_empty(), "the deployment receipt fails");
    env.worker.fast_forward(3).await?;

    let launch = view(&env.worker, env.factory.id().as_str(), "get_launch", json!({ "id": 0 })).await?;
    assert_eq!(launch["status"], "failed");
    let spent = before.saturating_sub(env.creator.view_account().await?.balance);
    assert!(spent < NearToken::from_millinear(50), "account balance refunded, spent {spent}");
    Ok(())
}

/// A valid wasm module with a `payload`-byte custom section and no exports.
fn module_without_new(payload: usize) -> Vec<u8> {
    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x00];
    let mut size = payload + 2;
    loop {
        let byte = (size & 0x7f) as u8;
        size >>= 7;
        module.push(if size == 0 { byte } else { byte | 0x80 });
        if size == 0 {
            break;
        }
    }
    module.extend([1, b'x']);
    module.extend(std::iter::repeat_n(0u8, payload));
    module
}
