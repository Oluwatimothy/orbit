use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::metadata::NonFungibleTokenMetadataProvider;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_created_receipts, get_logs};
use orbit_common::config::{CollectionConfig, Supply, TokenMedia};

use super::{account, collection_config, setup as setup_at, Contract};

const PRICE: u128 = 1_000_000_000_000_000_000_000_000; // 1 NEAR
const STORAGE_ALLOWANCE: u128 = 200_000_000_000_000_000_000_000; // 0.2 NEAR, enough for 10 NFTs

fn setup(predecessor: &str, deposit: u128, now_ns: u64) {
    setup_at(predecessor, deposit, now_ns);
}

fn config(supply: Supply) -> CollectionConfig {
    CollectionConfig {
        price: U128(PRICE),
        starts_at_ns: U64(1_000),
        ends_at_ns: Some(U64(2_000)),
        max_per_wallet: Some(3),
        ..collection_config(supply)
    }
}

fn deploy_with(config: CollectionConfig) -> Contract {
    super::deploy(None, Some(config), None, None)
}

fn deploy() -> Contract {
    deploy_with(config(Supply::Fixed { max: 5 }))
}

fn mint_as(contract: &mut Contract, who: &str, quantity: u32, deposit: u128) -> Vec<String> {
    setup(who, deposit, 1_500);
    contract.nft_mint(quantity).into_iter().map(|t| t.token_id).collect()
}

#[test]
fn mint_assigns_sequential_ids_and_ownership() {
    let mut contract = deploy();
    let ids = mint_as(&mut contract, "alice.testnet", 2, 2 * PRICE + STORAGE_ALLOWANCE);
    assert_eq!(ids, vec!["1", "2"]);
    assert_eq!(contract.nft_token("2".into()).unwrap().owner_id, account("alice.testnet"));
    assert_eq!(contract.nft_total_supply(), U128(2));
    assert_eq!(contract.orbit_minted_by(account("alice.testnet")), 2);
    assert_eq!(contract.orbit_mint_stats().minted, 2);
}

#[test]
fn mint_emits_nep171_event_and_pays_owner_and_refunds_excess() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    assert!(get_logs().iter().any(|l| l.contains(r#""event":"nft_mint""#)));

    let receipts = get_created_receipts();
    let receivers: Vec<_> = receipts.iter().map(|r| r.receiver_id.to_string()).collect();
    assert!(receivers.contains(&"creator.testnet".to_string()), "price paid to owner");
    assert!(receivers.contains(&"alice.testnet".to_string()), "excess refunded to minter");
}

#[test]
fn token_metadata_uses_collection_media() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    let metadata = contract.nft_token("1".into()).unwrap().metadata.unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Genesis #1"));
    assert_eq!(metadata.media.as_deref(), Some("ipfs://bafymedia"));
    assert_eq!(contract.nft_metadata().base_uri, None);
}

#[test]
fn directory_media_uses_relative_paths_and_base_uri() {
    let mut cfg = config(Supply::Fixed { max: 5 });
    cfg.media =
        TokenMedia::Directory { base_uri: "https://gw.example/ipfs/bafydir".into(), extension: "png".into() };
    let mut contract = deploy_with(cfg);
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    let metadata = contract.nft_token("1".into()).unwrap().metadata.unwrap();
    assert_eq!(metadata.media.as_deref(), Some("1.png"));
    assert_eq!(contract.nft_metadata().base_uri.as_deref(), Some("https://gw.example/ipfs/bafydir"));
}

#[test]
#[should_panic(expected = "is less than the required")]
fn mint_rejects_underpayment() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 2, 2 * PRICE - 1);
}

#[test]
#[should_panic(expected = "is less than the required")]
fn mint_rejects_payment_without_storage() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 1, PRICE);
}

#[test]
#[should_panic(expected = "only 1 tokens remain")]
fn mint_enforces_max_supply() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 3, 3 * PRICE + STORAGE_ALLOWANCE);
    mint_as(&mut contract, "bob.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    mint_as(&mut contract, "carol.testnet", 2, 2 * PRICE + STORAGE_ALLOWANCE);
}

#[test]
#[should_panic(expected = "this wallet may mint at most 3 tokens")]
fn mint_enforces_wallet_limit_across_calls() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 2, 2 * PRICE + STORAGE_ALLOWANCE);
    mint_as(&mut contract, "alice.testnet", 2, 2 * PRICE + STORAGE_ALLOWANCE);
}

#[test]
#[should_panic(expected = "quantity must be between 1 and 10")]
fn mint_rejects_zero_quantity() {
    let mut contract = deploy_with(config(Supply::Unlimited));
    mint_as(&mut contract, "alice.testnet", 0, STORAGE_ALLOWANCE);
}

#[test]
#[should_panic(expected = "quantity must be between 1 and 10")]
fn mint_rejects_quantity_over_per_call_limit() {
    let mut cfg = config(Supply::Unlimited);
    cfg.max_per_wallet = None;
    let mut contract = deploy_with(cfg);
    mint_as(&mut contract, "alice.testnet", 11, 11 * PRICE + STORAGE_ALLOWANCE);
}

#[test]
#[should_panic(expected = "minting has not started")]
fn mint_rejects_before_start() {
    let mut contract = deploy();
    setup("alice.testnet", PRICE + STORAGE_ALLOWANCE, 999);
    contract.nft_mint(1);
}

#[test]
#[should_panic(expected = "minting has ended")]
fn mint_rejects_after_end() {
    let mut contract = deploy();
    setup("alice.testnet", PRICE + STORAGE_ALLOWANCE, 2_000);
    contract.nft_mint(1);
}

#[test]
#[should_panic(expected = "public minting is not enabled")]
fn mint_rejects_when_public_mint_disabled() {
    let mut cfg = config(Supply::Fixed { max: 5 });
    cfg.public_mint = false;
    let mut contract = deploy_with(cfg);
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
}

#[test]
fn unlimited_supply_has_no_cap() {
    let mut cfg = config(Supply::Unlimited);
    cfg.max_per_wallet = None;
    let mut contract = deploy_with(cfg);
    for _ in 0..3 {
        mint_as(&mut contract, "alice.testnet", 10, 10 * PRICE + STORAGE_ALLOWANCE);
    }
    assert_eq!(contract.orbit_mint_stats().minted, 30);
    assert_eq!(contract.orbit_mint_stats().max_supply, None);
}

#[test]
fn creator_mint_ignores_price_and_window_but_not_supply() {
    let mut contract = deploy();
    setup("creator.testnet", STORAGE_ALLOWANCE, 0);
    let tokens = contract.creator_mint(account("bob.testnet"), 5);
    assert_eq!(tokens.len(), 5);
    assert_eq!(contract.nft_token("5".into()).unwrap().owner_id, account("bob.testnet"));
}

#[test]
#[should_panic(expected = "only 0 tokens remain")]
fn creator_mint_respects_supply() {
    let mut contract = deploy();
    setup("creator.testnet", STORAGE_ALLOWANCE, 0);
    contract.creator_mint(account("bob.testnet"), 5);
    contract.creator_mint(account("bob.testnet"), 1);
}

#[test]
#[should_panic(expected = "only the collection owner")]
fn creator_mint_rejects_non_owner() {
    let mut contract = deploy();
    setup("mallory.testnet", STORAGE_ALLOWANCE, 0);
    contract.creator_mint(account("mallory.testnet"), 1);
}

#[test]
#[should_panic(expected = "minting is paused")]
fn pause_blocks_minting() {
    let mut contract = deploy();
    setup("creator.testnet", 1, 0);
    contract.set_paused(true);
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
}

#[test]
#[should_panic(expected = "only the collection owner")]
fn pause_rejects_non_owner() {
    let mut contract = deploy();
    setup("mallory.testnet", 1, 0);
    contract.set_paused(true);
}

#[test]
fn transfer_changes_owner_and_enumeration() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    setup("alice.testnet", 1, 1_500);
    contract.nft_transfer(account("bob.testnet"), "1".into(), None, None);
    assert_eq!(contract.nft_token("1".into()).unwrap().owner_id, account("bob.testnet"));
    assert_eq!(contract.nft_supply_for_owner(account("alice.testnet")), U128(0));
    assert_eq!(contract.nft_tokens_for_owner(account("bob.testnet"), None, None).len(), 1);
}

#[test]
#[should_panic(expected = "Sender not approved")]
fn transfer_by_non_owner_fails() {
    let mut contract = deploy();
    mint_as(&mut contract, "alice.testnet", 1, PRICE + STORAGE_ALLOWANCE);
    setup("mallory.testnet", 1, 1_500);
    contract.nft_transfer(account("mallory.testnet"), "1".into(), None, None);
}

#[test]
fn mint_keeps_storage_and_transfer_reserve() {
    let mut contract = deploy_with(config(Supply::Unlimited));
    setup("alice.testnet", PRICE + STORAGE_ALLOWANCE, 1_500);
    let before = near_sdk::env::storage_usage();
    contract.nft_mint(1);
    let bytes = u128::from(near_sdk::env::storage_usage() - before);
    let kept = bytes * near_sdk::env::storage_byte_cost().as_yoctonear()
        + crate::TRANSFER_STORAGE_RESERVE.as_yoctonear();
    let refund = get_created_receipts()
        .into_iter()
        .find(|r| r.receiver_id == account("alice.testnet"))
        .expect("refund receipt");
    let refunded: u128 = format!("{:?}", refund.actions)
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .max()
        .unwrap();
    assert_eq!(refunded, STORAGE_ALLOWANCE - kept, "minted_by entry and reserve are charged");
}

#[test]
#[should_panic(expected = "this launch has no token")]
fn collection_only_launch_has_no_token_methods() {
    use near_contract_standards::fungible_token::FungibleTokenCore;
    deploy().ft_total_supply();
}
