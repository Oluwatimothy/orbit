//! The full connected-launch journey: factory → token + NFT + claim →
//! mint → claim → duplicate and non-owner rejection → transfer → claim.

mod common;

use anyhow::Result;
use common::*;
use near_workspaces::types::NearToken;
use serde_json::json;

const DECIMALS: u32 = 18;
const SUPPLY: u128 = 10_000_000 * 10u128.pow(DECIMALS);
const GENESIS: u128 = 1_000 * 10u128.pow(DECIMALS);
const FOUNDERS: u128 = 5_000 * 10u128.pow(DECIMALS);
// NFT supply 5: IDs 1–4 Genesis, ID 5 Founders.
const RESERVED: u128 = 4 * GENESIS + FOUNDERS;
const MINT_PRICE: NearToken = NearToken::from_millinear(100);

async fn create_connected(env: &Env, slug: &str) -> Result<()> {
    let claim_account = launch_account(env, slug, "claim");
    let deposit = quote(
        &view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": "connected" }))
            .await?,
    )?;
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
                    { "account_id": claim_account, "amount": RESERVED.to_string(), "label": "NFT holders" },
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
        .deposit(deposit)
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

async fn claim(
    who: &near_workspaces::Account,
    claim: &str,
    id: &str,
) -> Result<near_workspaces::result::ExecutionFinalResult> {
    Ok(who
        .call(&claim.parse()?, "claim")
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
    let (ft, nft, claim_id) = (
        launch_account(&env, "genesis", "ft"),
        launch_account(&env, "genesis", "nft"),
        launch_account(&env, "genesis", "claim"),
    );
    let w = &env.worker;

    // Registry, metadata, supply and funding.
    let launch = view(w, env.factory.id().as_str(), "get_launch", json!({ "id": 0 })).await?;
    assert_eq!(launch["status"], "live");
    assert_eq!(launch["claim"], claim_id);
    let metadata = view(w, &ft, "ft_metadata", json!({})).await?;
    assert_eq!((metadata["symbol"].as_str(), metadata["decimals"].as_u64()), (Some("ORBIT"), Some(18)));
    assert_eq!(view(w, &ft, "ft_total_supply", json!({})).await?, json!(SUPPLY.to_string()));
    assert_eq!(ft_balance(w, &ft, env.creator.id().as_str()).await?, SUPPLY - RESERVED);
    assert_eq!(ft_balance(w, &ft, &claim_id).await?, RESERVED);
    let info = view(w, &claim_id, "get_info", json!({})).await?;
    assert_eq!(info["status"], "active", "factory callback activated claims after verifying funding");
    assert_eq!(info["total_reserved"], json!(RESERVED.to_string()));

    // Launch accounts are keyless.
    for account in [&ft, &nft, &claim_id] {
        let keys = w.view_access_keys(&account.parse()?).await?;
        assert!(keys.is_empty(), "{account} must have no access keys");
    }

    // Alice mints 4 (IDs 1–4), Bob mints the last one (ID 5); a sixth is rejected.
    let creator_before = env.creator.view_account().await?.balance;
    expect_success(&mint(&env.alice, &nft, 4).await?)?;
    expect_success(&mint(&env.bob, &nft, 1).await?)?;
    let over = mint(&env.alice, &nft, 1).await?;
    assert!(failure_text(&over).contains("only 0 tokens remain"), "{}", failure_text(&over));
    let token = view(w, &nft, "nft_token", json!({ "token_id": "5" })).await?;
    assert_eq!(token["owner_id"], env.bob.id().as_str());
    let creator_after = env.creator.view_account().await?.balance;
    // Late gas refunds from the creator's own earlier transaction can land in
    // this window, so allow a small surplus above the exact proceeds.
    let received = creator_after.saturating_sub(creator_before);
    let proceeds = MINT_PRICE.saturating_mul(5);
    assert!(
        received >= proceeds && received < proceeds.saturating_add(NearToken::from_millinear(20)),
        "creator received {received}"
    );

    // Alice claims NFT #1.
    let result = claim(&env.alice, &claim_id, "1").await?;
    expect_success(&result)?;
    assert!(result.json::<bool>()?);
    assert_eq!(ft_balance(w, &ft, env.alice.id().as_str()).await?, GENESIS);

    // Duplicate claim of #1 is rejected — by Alice and by anyone else.
    let duplicate = claim(&env.alice, &claim_id, "1").await?;
    assert!(failure_text(&duplicate).contains("already been claimed"));
    let by_bob = claim(&env.bob, &claim_id, "1").await?;
    assert!(failure_text(&by_bob).contains("already been claimed"));

    // Bob cannot claim #2, which Alice owns: rolled back, no tokens move.
    let not_owner = claim(&env.bob, &claim_id, "2").await?;
    expect_success(&not_owner)?;
    assert!(!not_owner.json::<bool>()?);
    assert_eq!(ft_balance(w, &ft, env.bob.id().as_str()).await?, 0);
    assert_eq!(view(w, &claim_id, "get_claim", json!({ "nft_token_id": "2" })).await?, json!(null));

    // After Alice transfers #2 to Bob, Bob can claim it.
    let transfer = env
        .alice
        .call(&nft.parse()?, "nft_transfer")
        .args_json(json!({ "receiver_id": env.bob.id(), "token_id": "2" }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    expect_success(&transfer)?;
    expect_success(&claim(&env.bob, &claim_id, "2").await?)?;

    // Bob claims the Founders tier NFT.
    expect_success(&claim(&env.bob, &claim_id, "5").await?)?;

    // Final balances add up exactly.
    let alice = ft_balance(w, &ft, env.alice.id().as_str()).await?;
    let bob = ft_balance(w, &ft, env.bob.id().as_str()).await?;
    let remaining = ft_balance(w, &ft, &claim_id).await?;
    assert_eq!(alice, GENESIS);
    assert_eq!(bob, GENESIS + FOUNDERS);
    assert_eq!(remaining, RESERVED - alice - bob);
    let info = view(w, &claim_id, "get_info", json!({})).await?;
    assert_eq!(info["claimed_total"], json!((alice + bob).to_string()));
    assert_eq!(info["pending_total"], json!("0"));
    assert_eq!(info["claims_count"], json!(3));
    assert_eq!(view(w, &ft, "ft_total_supply", json!({})).await?, json!(SUPPLY.to_string()));
    Ok(())
}

#[tokio::test]
async fn callbacks_are_private() -> Result<()> {
    let env = setup().await?;
    create_connected(&env, "genesis").await?;
    let claim_id = launch_account(&env, "genesis", "claim");
    let forged = env
        .alice
        .call(&claim_id.parse()?, "on_transferred")
        .args_json(json!({ "nft_token_id": "1", "claimer": env.alice.id(), "amount": "1", "deposit": "0" }))
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(failure_text(&forged).contains("is private"), "{}", failure_text(&forged));

    let forged_launch = env
        .alice
        .call(env.factory.id(), "on_launch_deployed")
        .args_json(json!({ "launch_id": 0, "refunds": ["1000000000000000000000000"] }))
        .gas(MAX_GAS)
        .transact()
        .await?;
    assert!(failure_text(&forged_launch).contains("is private"));
    Ok(())
}
