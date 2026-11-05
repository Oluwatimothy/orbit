use near_contract_standards::fungible_token::metadata::FungibleTokenMetadataProvider;
use near_contract_standards::fungible_token::FungibleTokenCore;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::test_utils::{get_logs, VMContextBuilder};
use near_sdk::{testing_env, AccountId, NearToken};
use orbit_common::config::{TokenAllocation, TokenConfig};

use crate::Contract;

const TOTAL: u128 = 10_000_000 * 10u128.pow(18);

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn setup(predecessor: &str, deposit: NearToken) {
    let context = VMContextBuilder::new()
        .current_account_id(account("genesis-ft.factory.testnet"))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(deposit)
        .account_balance(NearToken::from_near(5))
        .build();
    testing_env!(context);
}

fn config() -> TokenConfig {
    TokenConfig {
        name: "Orbit Genesis".into(),
        symbol: "ORBIT".into(),
        decimals: 18,
        icon: Some("data:image/svg+xml;base64,PHN2Zy8+".into()),
        total_supply: U128(TOTAL),
        allocations: vec![
            TokenAllocation {
                account_id: account("creator.testnet"),
                amount: U128(TOTAL / 10 * 8),
                label: "Creator".into(),
            },
            TokenAllocation {
                account_id: account("genesis-claim.factory.testnet"),
                amount: U128(TOTAL / 10 * 2),
                label: "NFT holders".into(),
            },
        ],
    }
}

fn deploy() -> Contract {
    setup("factory.testnet", NearToken::from_yoctonear(0));
    Contract::new(config(), account("creator.testnet"))
}

#[test]
fn new_mints_exact_supply_to_allocations() {
    let contract = deploy();
    assert_eq!(contract.ft_total_supply(), U128(TOTAL));
    assert_eq!(contract.ft_balance_of(account("creator.testnet")), U128(TOTAL / 10 * 8));
    assert_eq!(contract.ft_balance_of(account("genesis-claim.factory.testnet")), U128(TOTAL / 10 * 2));
    assert_eq!(contract.ft_balance_of(account("someone.testnet")), U128(0));
    assert_eq!(contract.orbit_allocations().len(), 2);
    assert_eq!(contract.orbit_creator(), account("creator.testnet"));
}

#[test]
fn new_emits_one_mint_event_per_allocation() {
    deploy();
    let logs = get_logs();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].starts_with("EVENT_JSON:"));
    assert!(logs[0].contains(r#""event":"ft_mint""#));
    assert!(logs[0].contains("creator.testnet"));
    assert!(logs[0].contains("genesis-claim.factory.testnet"));
}

#[test]
fn metadata_matches_config() {
    let metadata = deploy().ft_metadata();
    assert_eq!(metadata.spec, "ft-1.0.0");
    assert_eq!(metadata.symbol, "ORBIT");
    assert_eq!(metadata.decimals, 18);
}

#[test]
#[should_panic(expected = "allocations add up to")]
fn new_rejects_allocations_not_matching_supply() {
    setup("factory.testnet", NearToken::from_yoctonear(0));
    let mut bad = config();
    bad.total_supply = U128(TOTAL + 1);
    Contract::new(bad, account("creator.testnet"));
}

#[test]
fn transfer_requires_registration_and_moves_balance() {
    let mut contract = deploy();
    let bounds = contract.storage_balance_bounds();

    setup("alice.testnet", bounds.min);
    contract.storage_deposit(None, None);
    assert!(contract.storage_balance_of(account("alice.testnet")).is_some());

    setup("creator.testnet", NearToken::from_yoctonear(1));
    contract.ft_transfer(account("alice.testnet"), U128(500), None);
    assert_eq!(contract.ft_balance_of(account("alice.testnet")), U128(500));
    assert_eq!(contract.ft_total_supply(), U128(TOTAL));
}

#[test]
#[should_panic(expected = "is not registered")]
fn transfer_to_unregistered_account_fails() {
    let mut contract = deploy();
    setup("creator.testnet", NearToken::from_yoctonear(1));
    contract.ft_transfer(account("bob.testnet"), U128(1), None);
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn transfer_requires_one_yocto() {
    let mut contract = deploy();
    setup("creator.testnet", NearToken::from_yoctonear(0));
    contract.ft_transfer(account("genesis-claim.factory.testnet"), U128(1), None);
}

#[test]
#[should_panic(expected = "The attached deposit is less than the minimum storage balance")]
fn storage_deposit_below_minimum_fails() {
    let mut contract = deploy();
    setup("alice.testnet", NearToken::from_yoctonear(1));
    contract.storage_deposit(None, None);
}
