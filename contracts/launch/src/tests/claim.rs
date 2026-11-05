use near_contract_standards::fungible_token::FungibleTokenCore;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::test_utils::get_logs;
use orbit_common::config::Supply;

use super::*;

fn claim_as(contract: &mut Contract, who: &str, id: &str) -> U128 {
    setup(who, STORAGE, 10);
    contract.claim(id.into())
}

fn balance(contract: &Contract, who: &str) -> u128 {
    contract.ft_balance_of(account(who)).0
}

#[test]
fn claim_pays_the_tier_amount_from_the_reserve() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    assert_eq!(claim_as(&mut contract, "alice.testnet", "1"), U128(GENESIS));

    assert_eq!(balance(&contract, "alice.testnet"), GENESIS);
    assert_eq!(balance(&contract, LAUNCH), CLAIM_RESERVE + SALE_TOTAL - GENESIS);
    let record = contract.claim_record("1".into()).unwrap();
    assert_eq!((record.claimer, record.amount), (account("alice.testnet"), U128(GENESIS)));
    let info = contract.claim_info();
    assert_eq!((info.claimed_total, info.claims_count), (U128(GENESIS), 1));
    assert_eq!(info.total_reserved, U128(CLAIM_RESERVE));
    let logs = get_logs();
    assert!(logs.iter().any(|l| l.contains(r#""event":"ft_transfer""#)), "standard transfer event");
    assert!(logs.iter().any(|l| l.contains(r#""event":"claim""#)), "orbit claim event");
}

#[test]
fn tiers_pay_different_amounts() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 10);
    claim_as(&mut contract, "alice.testnet", "8");
    claim_as(&mut contract, "alice.testnet", "9");
    assert_eq!(balance(&contract, "alice.testnet"), GENESIS + FOUNDERS);
    assert_eq!(contract.claim_allocation("10".into()), Some(U128(FOUNDERS)));
    assert_eq!(contract.claim_allocation("11".into()), None);
}

#[test]
#[should_panic(expected = "this NFT has already been claimed")]
fn an_nft_claims_once() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "alice.testnet", "1");
    claim_as(&mut contract, "alice.testnet", "1");
}

#[test]
#[should_panic(expected = "this NFT has already been claimed")]
fn a_new_holder_cannot_claim_a_claimed_nft_again() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "alice.testnet", "1");
    setup("alice.testnet", 1, 10);
    contract.nft_transfer(account("bob.testnet"), "1".into(), None, None);
    claim_as(&mut contract, "bob.testnet", "1");
}

#[test]
fn eligibility_follows_the_current_holder() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    setup("alice.testnet", 1, 10);
    contract.nft_transfer(account("bob.testnet"), "1".into(), None, None);
    claim_as(&mut contract, "bob.testnet", "1");
    assert_eq!(balance(&contract, "bob.testnet"), GENESIS);
    assert_eq!(balance(&contract, "alice.testnet"), 0);
}

#[test]
#[should_panic(expected = "you do not own this NFT")]
fn non_owner_cannot_claim() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "mallory.testnet", "1");
}

#[test]
#[should_panic(expected = "this NFT has not been minted")]
fn unminted_nft_cannot_claim() {
    let mut contract = connected();
    claim_as(&mut contract, "mallory.testnet", "3");
}

#[test]
#[should_panic(expected = "not eligible")]
fn non_canonical_ids_are_rejected() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "alice.testnet", "01");
}

#[test]
#[should_panic(expected = "claims have not started")]
fn claims_wait_for_the_start_time() {
    let mut claim = claim_config();
    claim.starts_at_ns = near_sdk::json_types::U64(1_000);
    let token = token_config(vec![(CREATOR, 1), (LAUNCH, CLAIM_RESERVE)]);
    let mut contract =
        deploy(Some(token), Some(collection_config(Supply::Fixed { max: 10 })), Some(claim), None);
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "alice.testnet", "1");
}

#[test]
fn claim_charges_only_the_storage_it_adds() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    setup("alice.testnet", STORAGE, 10);
    let before = near_sdk::env::storage_usage();
    contract.claim("1".into());
    let kept = u128::from(near_sdk::env::storage_usage() - before)
        * near_sdk::env::storage_byte_cost().as_yoctonear();
    let written = kept;
    let charged = STORAGE - transferred_to("alice.testnet");
    // Every byte written, topped up so a new registration costs at least the
    // standard's minimum (what `storage_unregister` would refund).
    let min = contract.storage_balance_bounds().min.as_yoctonear();
    assert!(written > 0 && charged >= written && charged < written + min, "charged {charged}");
}

#[test]
#[should_panic(expected = "is less than the required")]
fn claim_without_storage_deposit_fails() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 1);
    setup("alice.testnet", 0, 10);
    contract.claim("1".into());
}

#[test]
fn registered_holders_pay_only_for_the_record() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 2);
    claim_as(&mut contract, "alice.testnet", "1");
    claim_as(&mut contract, "alice.testnet", "2");
    assert_eq!(balance(&contract, "alice.testnet"), 2 * GENESIS);
}

#[test]
fn claim_records_batch_lookup() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 2);
    claim_as(&mut contract, "alice.testnet", "2");
    let records = contract.claim_records(vec!["1".into(), "2".into()]);
    assert!(records[0].is_none());
    assert_eq!(records[1].as_ref().unwrap().claimer, account("alice.testnet"));
}

#[test]
#[should_panic(expected = "at most 50 IDs per call")]
fn claim_records_is_bounded() {
    connected().claim_records((0..51).map(|i| i.to_string()).collect());
}

#[test]
#[should_panic(expected = "this launch has no holder claims")]
fn launches_without_claims_reject_claims() {
    let token = token_config(vec![(CREATOR, 1), (LAUNCH, SALE_TOTAL)]);
    let mut contract =
        deploy(Some(token), Some(collection_config(Supply::Fixed { max: 10 })), None, Some(sale_config()));
    mint(&mut contract, "alice.testnet", 1);
    claim_as(&mut contract, "alice.testnet", "1");
}

#[test]
fn every_nft_can_claim_and_the_reserve_is_exact() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 10);
    for id in 1..=10 {
        claim_as(&mut contract, "alice.testnet", &id.to_string());
    }
    assert_eq!(balance(&contract, "alice.testnet"), CLAIM_RESERVE);
    assert_eq!(balance(&contract, LAUNCH), SALE_TOTAL, "the sale's tokens are untouched");
    assert_eq!(contract.ft_total_supply(), U128(CREATOR_SHARE + CLAIM_RESERVE + SALE_TOTAL));
}

#[test]
fn register_unregister_cycles_cannot_drain_the_launch() {
    let mut contract = connected();
    mint(&mut contract, "al.testnet", 1);
    setup("al.testnet", STORAGE, 10);
    let before = near_sdk::env::storage_usage();
    contract.claim("1".into());
    let written = u128::from(near_sdk::env::storage_usage() - before)
        * near_sdk::env::storage_byte_cost().as_yoctonear();
    let kept = STORAGE - transferred_to("al.testnet");
    let refund_on_unregister = contract.storage_balance_bounds().min.as_yoctonear();
    assert!(kept >= written, "every byte written is charged");
    assert!(kept >= refund_on_unregister, "unregistering cannot refund more than the claim paid");
}
