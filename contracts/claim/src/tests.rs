use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_logs, VMContextBuilder};
use near_sdk::{serde_json, testing_env, AccountId, NearToken, PromiseResult, RuntimeFeesConfig};
use orbit_common::config::{ClaimConfig, Supply, Tier};

use crate::claim::CLAIM_DEPOSIT;
use crate::{ClaimState, Contract, Status};

const SELF_ID: &str = "genesis-claim.factory.testnet";
const GENESIS: u128 = 1_000;
const FOUNDERS: u128 = 5_000;
// IDs 1–8 Genesis, 9–10 Founders.
const TOTAL: u128 = 8 * GENESIS + 2 * FOUNDERS;

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn context(predecessor: &str, deposit: NearToken, results: Vec<PromiseResult>) {
    let ctx = VMContextBuilder::new()
        .current_account_id(account(SELF_ID))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(deposit)
        .account_balance(NearToken::from_near(1))
        .block_timestamp(10)
        .build();
    testing_env!(ctx, near_sdk::test_vm_config(), RuntimeFeesConfig::test(), Default::default(), results);
}

fn deploy() -> Contract {
    context("factory.testnet", NearToken::from_yoctonear(0), vec![]);
    let config = ClaimConfig {
        tiers: vec![
            Tier { name: "Genesis".into(), from: 1, to: 8, amount_per_nft: U128(GENESIS) },
            Tier { name: "Founders".into(), from: 9, to: 10, amount_per_nft: U128(FOUNDERS) },
        ],
        starts_at_ns: U64(0),
    };
    Contract::new(
        account("genesis-ft.factory.testnet"),
        account("genesis-nft.factory.testnet"),
        account("creator.testnet"),
        config,
        Supply::Fixed { max: 10 },
    )
}

fn activate(contract: &mut Contract, balance: u128) -> bool {
    let bytes = serde_json::to_vec(&U128(balance)).unwrap();
    context(SELF_ID, NearToken::from_yoctonear(0), vec![PromiseResult::Successful(bytes)]);
    contract.on_balance_checked(Ok(U128(balance)))
}

fn active() -> Contract {
    let mut contract = deploy();
    assert!(activate(&mut contract, TOTAL));
    contract
}

fn owner_result(token_id: &str, owner: &str) -> PromiseResult {
    let token = Token {
        token_id: token_id.into(),
        owner_id: account(owner),
        metadata: None,
        approved_account_ids: None,
    };
    PromiseResult::Successful(serde_json::to_vec(&Some(token)).unwrap())
}

fn start_claim(contract: &mut Contract, who: &str, id: &str) {
    context(who, CLAIM_DEPOSIT, vec![]);
    let _ = contract.claim(id.into());
}

fn owner_checked(contract: &mut Contract, result: PromiseResult, id: &str, claimer: &str, amount: u128) {
    context(SELF_ID, NearToken::from_yoctonear(0), vec![result]);
    let _ = contract.on_owner_checked(id.into(), account(claimer), U128(amount), CLAIM_DEPOSIT);
}

fn transferred(
    contract: &mut Contract,
    result: PromiseResult,
    id: &str,
    claimer: &str,
    amount: u128,
) -> bool {
    context(SELF_ID, NearToken::from_yoctonear(0), vec![result]);
    contract.on_transferred(id.into(), account(claimer), U128(amount), CLAIM_DEPOSIT)
}

fn complete_claim(contract: &mut Contract, who: &str, id: &str, amount: u128) {
    start_claim(contract, who, id);
    owner_checked(contract, owner_result(id, who), id, who, amount);
    assert!(transferred(contract, PromiseResult::Successful(vec![]), id, who, amount));
}

#[test]
fn new_computes_reserve_and_waits_for_funding() {
    let info = deploy().get_info();
    assert_eq!(info.total_reserved, U128(TOTAL));
    assert_eq!(info.status, Status::AwaitingFunding);
}

#[test]
#[should_panic(expected = "connected launches require a fixed NFT supply")]
fn new_rejects_unlimited_collection() {
    context("factory.testnet", NearToken::from_yoctonear(0), vec![]);
    let config = ClaimConfig {
        tiers: vec![Tier { name: "All".into(), from: 1, to: 1, amount_per_nft: U128(1) }],
        starts_at_ns: U64(0),
    };
    Contract::new(
        account("ft.testnet"),
        account("nft.testnet"),
        account("c.testnet"),
        config,
        Supply::Unlimited,
    );
}

#[test]
fn activation_requires_full_funding() {
    let mut contract = deploy();
    assert!(!activate(&mut contract, TOTAL - 1));
    assert_eq!(contract.get_info().status, Status::AwaitingFunding);
    assert!(activate(&mut contract, TOTAL));
    assert_eq!(contract.get_info().status, Status::Active);
    assert!(get_logs().iter().any(|l| l.contains("claim_activated")));
}

#[test]
fn activation_failure_keeps_claims_closed() {
    let mut contract = deploy();
    context(SELF_ID, NearToken::from_yoctonear(0), vec![PromiseResult::Failed]);
    assert!(!contract.on_balance_checked(Err(near_sdk::PromiseError::Failed)));
    assert_eq!(contract.get_info().status, Status::AwaitingFunding);
}

#[test]
#[should_panic(expected = "claims are not active")]
fn claim_before_activation_fails() {
    let mut contract = deploy();
    start_claim(&mut contract, "alice.testnet", "1");
}

#[test]
#[should_panic(expected = "attach at least")]
fn claim_requires_deposit() {
    let mut contract = active();
    context("alice.testnet", NearToken::from_yoctonear(CLAIM_DEPOSIT.as_yoctonear() - 1), vec![]);
    let _ = contract.claim("1".into());
}

#[test]
#[should_panic(expected = "not eligible")]
fn claim_outside_tiers_fails() {
    let mut contract = active();
    start_claim(&mut contract, "alice.testnet", "11");
}

#[test]
#[should_panic(expected = "not eligible")]
fn non_canonical_ids_are_rejected() {
    let mut contract = active();
    start_claim(&mut contract, "alice.testnet", "01");
}

#[test]
fn tier_amounts_are_looked_up_by_id() {
    let contract = deploy();
    assert_eq!(contract.get_allocation("8".into()), Some(U128(GENESIS)));
    assert_eq!(contract.get_allocation("9".into()), Some(U128(FOUNDERS)));
    assert_eq!(contract.get_allocation("0".into()), None);
    assert_eq!(contract.get_allocation("abc".into()), None);
}

#[test]
fn successful_claim_is_recorded_once_transfer_succeeds() {
    let mut contract = active();
    start_claim(&mut contract, "alice.testnet", "9");
    assert_eq!(contract.get_claim("9".into()).unwrap().state, ClaimState::Pending);
    assert_eq!(contract.get_info().pending_total, U128(FOUNDERS));

    owner_checked(&mut contract, owner_result("9", "alice.testnet"), "9", "alice.testnet", FOUNDERS);
    assert_eq!(
        contract.get_claim("9".into()).unwrap().state,
        ClaimState::Pending,
        "not claimed before transfer result"
    );

    assert!(transferred(&mut contract, PromiseResult::Successful(vec![]), "9", "alice.testnet", FOUNDERS));
    let record = contract.get_claim("9".into()).unwrap();
    assert_eq!(record.state, ClaimState::Claimed);
    assert_eq!(record.claimer, account("alice.testnet"));
    let info = contract.get_info();
    assert_eq!((info.claimed_total, info.pending_total, info.claims_count), (U128(FOUNDERS), U128(0), 1));
    assert!(get_logs().iter().any(|l| l.contains(r#""event":"claim""#)));
}

#[test]
#[should_panic(expected = "already been claimed")]
fn duplicate_claim_is_rejected() {
    let mut contract = active();
    complete_claim(&mut contract, "alice.testnet", "1", GENESIS);
    start_claim(&mut contract, "alice.testnet", "1");
}

#[test]
#[should_panic(expected = "already been claimed")]
fn new_owner_cannot_reclaim_after_transfer() {
    let mut contract = active();
    complete_claim(&mut contract, "alice.testnet", "1", GENESIS);
    start_claim(&mut contract, "bob.testnet", "1");
}

#[test]
#[should_panic(expected = "already in progress")]
fn concurrent_claim_is_rejected_while_pending() {
    let mut contract = active();
    start_claim(&mut contract, "alice.testnet", "1");
    start_claim(&mut contract, "bob.testnet", "1");
}

#[test]
fn non_owner_claim_is_rolled_back() {
    let mut contract = active();
    start_claim(&mut contract, "mallory.testnet", "2");
    owner_checked(&mut contract, owner_result("2", "alice.testnet"), "2", "mallory.testnet", GENESIS);
    assert_eq!(contract.get_claim("2".into()), None);
    assert_eq!(contract.get_info().pending_total, U128(0));
    assert!(get_logs().iter().any(|l| l.contains("claim_rejected")));
    // The real owner can still claim.
    complete_claim(&mut contract, "alice.testnet", "2", GENESIS);
}

#[test]
fn failed_or_malformed_ownership_lookup_is_rolled_back() {
    let mut contract = active();
    for result in [
        PromiseResult::Failed,
        PromiseResult::Successful(b"not json".to_vec()),
        PromiseResult::Successful(b"null".to_vec()),
    ] {
        start_claim(&mut contract, "alice.testnet", "3");
        owner_checked(&mut contract, result, "3", "alice.testnet", GENESIS);
        assert_eq!(contract.get_claim("3".into()), None);
    }
    assert_eq!(contract.get_info().pending_total, U128(0));
}

#[test]
fn failed_transfer_is_rolled_back_and_retryable() {
    let mut contract = active();
    start_claim(&mut contract, "alice.testnet", "4");
    owner_checked(&mut contract, owner_result("4", "alice.testnet"), "4", "alice.testnet", GENESIS);
    assert!(!transferred(&mut contract, PromiseResult::Failed, "4", "alice.testnet", GENESIS));
    assert_eq!(contract.get_claim("4".into()), None);
    let info = contract.get_info();
    assert_eq!((info.claimed_total, info.pending_total), (U128(0), U128(0)));
    assert!(get_logs().iter().any(|l| l.contains("claim_failed")));

    complete_claim(&mut contract, "alice.testnet", "4", GENESIS);
    assert_eq!(contract.get_info().claimed_total, U128(GENESIS));
}

#[test]
fn claimed_total_never_exceeds_reserve() {
    let mut contract = active();
    for id in 1..=10u32 {
        let amount = if id <= 8 { GENESIS } else { FOUNDERS };
        complete_claim(&mut contract, "alice.testnet", &id.to_string(), amount);
    }
    let info = contract.get_info();
    assert_eq!(info.claimed_total, info.total_reserved);
    assert_eq!(info.claims_count, 10);
}

#[test]
#[should_panic(expected = "at most 50 IDs")]
fn batch_lookup_is_bounded() {
    let contract = deploy();
    contract.get_claims((1..=51).map(|i| i.to_string()).collect());
}
