use near_contract_standards::fungible_token::metadata::FungibleTokenMetadataProvider;
use near_contract_standards::fungible_token::FungibleTokenCore;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::test_utils::get_logs;

use super::*;

const TOTAL: u128 = 10_000_000 * TOKEN;

fn token_only() -> Contract {
    deploy(
        Some(token_config(vec![(CREATOR, TOTAL / 10 * 8), ("treasury.testnet", TOTAL / 10 * 2)])),
        None,
        None,
        None,
    )
}

#[test]
fn new_mints_exact_supply_to_allocations() {
    let contract = token_only();
    assert_eq!(contract.ft_total_supply(), U128(TOTAL));
    assert_eq!(contract.ft_balance_of(account(CREATOR)), U128(TOTAL / 10 * 8));
    assert_eq!(contract.ft_balance_of(account("treasury.testnet")), U128(TOTAL / 10 * 2));
    assert_eq!(contract.ft_balance_of(account("someone.testnet")), U128(0));
    assert_eq!(contract.orbit_allocations().len(), 2);
    assert_eq!(contract.orbit_creator(), account(CREATOR));
}

#[test]
fn new_emits_one_mint_event_for_all_allocations() {
    token_only();
    let logs = get_logs();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].starts_with("EVENT_JSON:"));
    assert!(logs[0].contains(r#""event":"ft_mint""#));
    assert!(logs[0].contains(CREATOR) && logs[0].contains("treasury.testnet"));
}

#[test]
fn metadata_matches_config() {
    let metadata = token_only().ft_metadata();
    assert_eq!(metadata.spec, "ft-1.0.0");
    assert_eq!(metadata.symbol, "ORBIT");
    assert_eq!(metadata.decimals, 18);
}

#[test]
#[should_panic(expected = "allocations add up to")]
fn new_rejects_allocations_not_matching_supply() {
    let mut bad = token_config(vec![(CREATOR, TOTAL)]);
    bad.total_supply = U128(TOTAL + 1);
    deploy(Some(bad), None, None, None);
}

#[test]
#[should_panic(expected = "tokens there are locked")]
fn token_only_launch_cannot_allocate_to_itself() {
    deploy(Some(token_config(vec![(CREATOR, 1), (LAUNCH, 1)])), None, None, None);
}

#[test]
fn transfer_requires_registration_and_moves_balance() {
    let mut contract = token_only();
    let bounds = contract.storage_balance_bounds();

    setup("alice.testnet", bounds.min.as_yoctonear(), 0);
    contract.storage_deposit(None, None);
    assert!(contract.storage_balance_of(account("alice.testnet")).is_some());

    setup(CREATOR, 1, 0);
    contract.ft_transfer(account("alice.testnet"), U128(500), None);
    assert_eq!(contract.ft_balance_of(account("alice.testnet")), U128(500));
    assert_eq!(contract.ft_total_supply(), U128(TOTAL));
}

#[test]
#[should_panic(expected = "is not registered")]
fn transfer_to_unregistered_account_fails() {
    let mut contract = token_only();
    setup(CREATOR, 1, 0);
    contract.ft_transfer(account("bob.testnet"), U128(1), None);
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn transfer_requires_one_yocto() {
    let mut contract = token_only();
    setup(CREATOR, 0, 0);
    contract.ft_transfer(account("treasury.testnet"), U128(1), None);
}

#[test]
#[should_panic(expected = "The attached deposit is less than the minimum storage balance")]
fn storage_deposit_below_minimum_fails() {
    let mut contract = token_only();
    setup("alice.testnet", 1, 0);
    contract.storage_deposit(None, None);
}

#[test]
#[should_panic(expected = "this launch has no collection")]
fn token_only_launch_has_no_nft_methods() {
    let contract = token_only();
    contract.nft_token("1".into());
}
