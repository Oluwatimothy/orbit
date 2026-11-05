use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_created_receipts, get_logs, VMContextBuilder};
use near_sdk::{serde_json, testing_env, AccountId, NearToken, PromiseResult, RuntimeFeesConfig};
use orbit_common::config::{SaleConfig, Supply};

use crate::buy::{Charges, Slot};
use crate::{Contract, Phase, Status, MIN_PURCHASE_DEPOSIT};

const SELF_ID: &str = "genesis-sale.factory.testnet";
const TOKEN: u128 = 10u128.pow(18);
const NEAR: u128 = 10u128.pow(24);
const PRICE: u128 = NEAR / 10; // 0.1 NEAR per token
const HOLDER_START: u64 = 100;
const PUBLIC_START: u64 = 200;
const END: u64 = 300;

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn context(predecessor: &str, deposit: u128, now: u64, results: Vec<PromiseResult>) {
    let ctx = VMContextBuilder::new()
        .current_account_id(account(SELF_ID))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .account_balance(NearToken::from_near(1))
        .block_timestamp(now)
        .build();
    testing_env!(ctx, near_sdk::test_vm_config(), RuntimeFeesConfig::test(), Default::default(), results);
}

fn config() -> SaleConfig {
    SaleConfig {
        price_per_token: U128(PRICE),
        total_for_sale: U128(100 * TOKEN),
        per_nft_limit: U128(10 * TOKEN),
        holder_starts_at_ns: U64(HOLDER_START),
        public_starts_at_ns: Some(U64(PUBLIC_START)),
        per_account_public_limit: Some(U128(5 * TOKEN)),
        ends_at_ns: U64(END),
    }
}

fn active() -> Contract {
    context("factory.testnet", 0, 0, vec![]);
    let mut contract = Contract::new(
        account("genesis-ft.factory.testnet"),
        account("genesis-nft.factory.testnet"),
        account("creator.testnet"),
        18,
        config(),
        Supply::Fixed { max: 100 },
    );
    context(SELF_ID, 0, 0, vec![]);
    assert!(contract.on_balance_checked(Ok(U128(100 * TOKEN))));
    contract
}

fn deposit_for(amount: u128) -> u128 {
    amount / TOKEN * PRICE + MIN_PURCHASE_DEPOSIT.as_yoctonear()
}

fn charges(amount: u128) -> Charges {
    Charges {
        cost: U128(amount / TOKEN * PRICE),
        deposit: NearToken::from_yoctonear(deposit_for(amount)),
        storage_fee: NearToken::from_micronear(700),
    }
}

fn owner_result(id: &str, owner: &str) -> PromiseResult {
    let token =
        Token { token_id: id.into(), owner_id: account(owner), metadata: None, approved_account_ids: None };
    PromiseResult::Successful(serde_json::to_vec(&Some(token)).unwrap())
}

fn buy(contract: &mut Contract, who: &str, amount: u128, nft: Option<&str>, now: u64) {
    context(who, deposit_for(amount), now, vec![]);
    let _ = contract.buy(U128(amount), nft.map(String::from));
}

fn deliver(contract: &mut Contract, who: &str, amount: u128, slot: Slot, ok: bool) -> bool {
    let result = if ok { PromiseResult::Successful(vec![]) } else { PromiseResult::Failed };
    context(SELF_ID, 0, 150, vec![result]);
    contract.on_delivered(account(who), U128(amount), charges(amount), slot)
}

#[test]
fn funding_is_required_before_sales() {
    context("factory.testnet", 0, 0, vec![]);
    let mut contract = Contract::new(
        account("ft.testnet"),
        account("nft.testnet"),
        account("creator.testnet"),
        18,
        config(),
        Supply::Fixed { max: 100 },
    );
    context(SELF_ID, 0, 0, vec![]);
    assert!(!contract.on_balance_checked(Ok(U128(100 * TOKEN - 1))));
    assert_eq!(contract.get_info().status, Status::AwaitingFunding);
}

#[test]
fn phases_follow_the_configured_windows() {
    let contract = active();
    for (now, phase) in [
        (0, Phase::NotStarted),
        (HOLDER_START, Phase::Holders),
        (PUBLIC_START, Phase::Public),
        (END, Phase::Ended),
    ] {
        context(SELF_ID, 0, now, vec![]);
        assert_eq!(contract.get_info().phase, phase);
    }
}

#[test]
fn holder_purchase_reserves_then_settles_and_pays_creator() {
    let mut contract = active();
    buy(&mut contract, "alice.testnet", 4 * TOKEN, Some("7"), 150);
    assert_eq!(contract.get_purchased_by_nft("7".into()), U128(4 * TOKEN));
    assert_eq!(contract.get_info().pending, U128(4 * TOKEN));

    context(SELF_ID, 0, 150, vec![owner_result("7", "alice.testnet")]);
    let _ = contract.on_owner_checked(
        account("alice.testnet"),
        U128(4 * TOKEN),
        charges(4 * TOKEN),
        Slot::Nft("7".into()),
    );
    assert_eq!(contract.get_info().sold, U128(0), "not sold before the transfer resolves");

    assert!(deliver(&mut contract, "alice.testnet", 4 * TOKEN, Slot::Nft("7".into()), true));
    let info = contract.get_info();
    assert_eq!((info.sold, info.pending), (U128(4 * TOKEN), U128(0)));
    assert!(get_created_receipts().iter().any(|r| r.receiver_id == account("creator.testnet")));
    assert!(get_logs().iter().any(|l| l.contains(r#""event":"purchase""#)));
}

#[test]
#[should_panic(expected = "the holder window requires an NFT ID")]
fn holder_window_requires_an_nft() {
    let mut contract = active();
    buy(&mut contract, "alice.testnet", TOKEN, None, 150);
}

#[test]
#[should_panic(expected = "this NFT can buy at most 2000000000000000000 more")]
fn per_nft_limit_counts_in_flight_purchases() {
    let mut contract = active();
    buy(&mut contract, "alice.testnet", 8 * TOKEN, Some("7"), 150);
    buy(&mut contract, "bob.testnet", 3 * TOKEN, Some("7"), 150);
}

#[test]
fn non_owner_purchase_is_rolled_back() {
    let mut contract = active();
    buy(&mut contract, "mallory.testnet", 5 * TOKEN, Some("7"), 150);
    context(SELF_ID, 0, 150, vec![owner_result("7", "alice.testnet")]);
    let _ = contract.on_owner_checked(
        account("mallory.testnet"),
        U128(5 * TOKEN),
        charges(5 * TOKEN),
        Slot::Nft("7".into()),
    );
    assert_eq!(contract.get_purchased_by_nft("7".into()), U128(0));
    assert_eq!(contract.get_info().pending, U128(0));
    assert!(get_logs().iter().any(|l| l.contains("purchase_rejected")));
}

#[test]
fn failed_delivery_releases_the_reservation() {
    let mut contract = active();
    buy(&mut contract, "carol.testnet", 5 * TOKEN, None, 250);
    assert_eq!(contract.get_purchased_by_account(account("carol.testnet")), U128(5 * TOKEN));
    assert!(!deliver(
        &mut contract,
        "carol.testnet",
        5 * TOKEN,
        Slot::Account(account("carol.testnet")),
        false
    ));
    assert_eq!(contract.get_purchased_by_account(account("carol.testnet")), U128(0));
    let info = contract.get_info();
    assert_eq!((info.sold, info.pending), (U128(0), U128(0)));
}

#[test]
#[should_panic(expected = "this account can buy at most 0 more")]
fn public_limit_is_per_account() {
    let mut contract = active();
    buy(&mut contract, "carol.testnet", 5 * TOKEN, None, 250);
    buy(&mut contract, "carol.testnet", TOKEN, None, 250);
}

#[test]
#[should_panic(expected = "not enough tokens left")]
fn sale_cannot_oversell() {
    let mut contract = active();
    // Ten different NFTs each buy their 10-token limit: exactly the 100 for sale.
    for id in 1..=10 {
        buy(&mut contract, "alice.testnet", 10 * TOKEN, Some(&id.to_string()), 150);
    }
    buy(&mut contract, "alice.testnet", TOKEN, Some("11"), 150);
}

#[test]
#[should_panic(expected = "attach at least")]
fn underpayment_is_rejected() {
    let mut contract = active();
    context("alice.testnet", deposit_for(TOKEN) - 1, 150, vec![]);
    let _ = contract.buy(U128(TOKEN), Some("1".into()));
}

#[test]
#[should_panic(expected = "the sale has ended")]
fn no_purchases_after_end() {
    let mut contract = active();
    buy(&mut contract, "alice.testnet", TOKEN, None, END);
}

#[test]
#[should_panic(expected = "the sale has not ended")]
fn unsold_withdrawal_waits_for_the_end() {
    let mut contract = active();
    context("creator.testnet", 0, 150, vec![]);
    let _ = contract.withdraw_unsold();
}

#[test]
#[should_panic(expected = "only the creator")]
fn only_creator_withdraws_unsold() {
    let mut contract = active();
    context("mallory.testnet", 0, END, vec![]);
    let _ = contract.withdraw_unsold();
}

#[test]
fn creator_withdraws_unsold_once_and_can_retry_after_failure() {
    let mut contract = active();
    context("creator.testnet", 0, END, vec![]);
    let _ = contract.withdraw_unsold();
    assert_eq!(contract.get_info().status, Status::Closed);

    context(SELF_ID, 0, END, vec![PromiseResult::Failed]);
    assert!(!contract.on_unsold_withdrawn(U128(100 * TOKEN)));
    assert_eq!(contract.get_info().status, Status::Active, "failure re-opens withdrawal");
}

#[test]
fn quote_rounds_up() {
    let contract = active();
    assert_eq!(contract.quote(U128(1)), U128(PRICE / TOKEN));
    assert_eq!(contract.quote(U128(TOKEN + 1)), U128(PRICE + PRICE / TOKEN));
    assert_eq!(contract.quote(U128(3 * TOKEN)), U128(3 * PRICE));
}

#[test]
#[should_panic(expected = "not an NFT ID in this collection")]
fn holder_purchases_reject_ids_outside_the_collection() {
    let mut contract = active();
    buy(&mut contract, "mallory.testnet", TOKEN, Some("101"), 150);
}

#[test]
#[should_panic(expected = "not an NFT ID in this collection")]
fn holder_purchases_reject_non_canonical_ids() {
    let mut contract = active();
    buy(&mut contract, "mallory.testnet", TOKEN, Some("07"), 150);
}

#[test]
fn rejected_purchases_free_their_records_and_keep_a_fee() {
    let mut contract = active();
    buy(&mut contract, "mallory.testnet", 5 * TOKEN, Some("7"), 150);
    context(SELF_ID, 0, 150, vec![owner_result("7", "alice.testnet")]);
    let _ = contract.on_owner_checked(
        account("mallory.testnet"),
        U128(5 * TOKEN),
        charges(5 * TOKEN),
        Slot::Nft("7".into()),
    );
    let refund = get_created_receipts()
        .into_iter()
        .find(|r| r.receiver_id == account("mallory.testnet"))
        .expect("refund receipt");
    let refunded: u128 = format!("{:?}", refund.actions)
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .max()
        .unwrap();
    assert_eq!(refunded, deposit_for(5 * TOKEN) - crate::buy::REJECTION_FEE.as_yoctonear());
    // The record was removed, not left at zero.
    assert_eq!(contract.get_purchased_by_nft("7".into()), U128(0));
}
