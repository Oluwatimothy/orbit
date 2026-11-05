use near_contract_standards::fungible_token::FungibleTokenCore;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::U128;
use near_sdk::test_utils::get_logs;

use super::*;
use crate::{Phase, SaleStatus};

fn cost(amount: u128) -> u128 {
    amount / TOKEN * SALE_PRICE
}

fn buy(contract: &mut Contract, who: &str, amount: u128, nft: Option<&str>, now: u64) -> U128 {
    setup(who, cost(amount) + STORAGE, now);
    contract.buy(U128(amount), nft.map(String::from))
}

fn with_holder(contract: &mut Contract) {
    mint(contract, "alice.testnet", 2);
}

fn balance(contract: &Contract, who: &str) -> u128 {
    contract.ft_balance_of(account(who)).0
}

#[test]
fn holder_buys_with_an_owned_nft_and_the_creator_is_paid() {
    let mut contract = connected();
    with_holder(&mut contract);
    assert_eq!(buy(&mut contract, "alice.testnet", 4 * TOKEN, Some("1"), 150), U128(cost(4 * TOKEN)));

    assert_eq!(transferred_to(CREATOR), cost(4 * TOKEN));
    assert_eq!(balance(&contract, "alice.testnet"), 4 * TOKEN);
    assert_eq!(contract.sale_purchased_by_nft("1".into()), U128(4 * TOKEN));
    let info = contract.sale_info();
    assert_eq!((info.sold, info.phase, info.status), (U128(4 * TOKEN), Phase::Holders, SaleStatus::Active));
    assert!(get_logs().iter().any(|l| l.contains(r#""event":"purchase""#)));
}

#[test]
fn buyer_pays_storage_and_gets_the_rest_back() {
    let mut contract = connected();
    with_holder(&mut contract);
    setup("alice.testnet", cost(TOKEN) + STORAGE, 150);
    let before = near_sdk::env::storage_usage();
    contract.buy(U128(TOKEN), Some("1".into()));
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
fn underpayment_is_rejected() {
    let mut contract = connected();
    with_holder(&mut contract);
    setup("alice.testnet", cost(TOKEN), 150);
    contract.buy(U128(TOKEN), Some("1".into()));
}

#[test]
#[should_panic(expected = "the holder window requires an NFT ID")]
fn holder_window_requires_an_nft() {
    let mut contract = connected();
    with_holder(&mut contract);
    buy(&mut contract, "alice.testnet", TOKEN, None, 150);
}

#[test]
#[should_panic(expected = "you do not own this NFT")]
fn holder_window_rejects_nfts_the_buyer_does_not_own() {
    let mut contract = connected();
    with_holder(&mut contract);
    buy(&mut contract, "mallory.testnet", TOKEN, Some("1"), 150);
}

#[test]
#[should_panic(expected = "this NFT has not been minted")]
fn holder_window_rejects_unminted_ids() {
    let mut contract = connected();
    buy(&mut contract, "mallory.testnet", TOKEN, Some("7"), 150);
}

#[test]
#[should_panic(expected = "this NFT can buy at most 6000000000000000000 more")]
fn per_nft_limit_holds_across_purchases() {
    let mut contract = connected();
    with_holder(&mut contract);
    buy(&mut contract, "alice.testnet", 4 * TOKEN, Some("1"), 150);
    buy(&mut contract, "alice.testnet", 7 * TOKEN, Some("1"), 150);
}

#[test]
#[should_panic(expected = "this NFT can buy at most 0 more")]
fn per_nft_limit_follows_the_nft_not_the_holder() {
    let mut contract = connected();
    with_holder(&mut contract);
    buy(&mut contract, "alice.testnet", 10 * TOKEN, Some("1"), 150);
    setup("alice.testnet", 1, 150);
    contract.nft_transfer(account("bob.testnet"), "1".into(), None, None);
    buy(&mut contract, "bob.testnet", TOKEN, Some("1"), 150);
}

#[test]
fn public_window_uses_per_account_limits() {
    let mut contract = connected();
    buy(&mut contract, "carol.testnet", 5 * TOKEN, None, 250);
    assert_eq!(contract.sale_purchased_by_account(account("carol.testnet")), U128(5 * TOKEN));
    assert_eq!(contract.sale_info().phase, Phase::Public);
}

#[test]
#[should_panic(expected = "this account can buy at most 0 more")]
fn public_limit_holds_across_purchases() {
    let mut contract = connected();
    buy(&mut contract, "carol.testnet", 5 * TOKEN, None, 250);
    buy(&mut contract, "carol.testnet", TOKEN, None, 250);
}

#[test]
#[should_panic(expected = "the sale has not started")]
fn purchases_wait_for_the_holder_window() {
    let mut contract = connected();
    with_holder(&mut contract);
    buy(&mut contract, "alice.testnet", TOKEN, Some("1"), 50);
}

#[test]
#[should_panic(expected = "the sale has ended")]
fn purchases_stop_at_the_end() {
    let mut contract = connected();
    buy(&mut contract, "carol.testnet", TOKEN, None, SALE_END);
}

#[test]
#[should_panic(expected = "not enough tokens left in the sale")]
fn cannot_buy_more_than_is_for_sale() {
    let mut contract = connected();
    buy(&mut contract, "carol.testnet", SALE_TOTAL + 1, None, 250);
}

#[test]
fn quote_prices_base_units() {
    // Rounding is covered by `sale_cost` tests in orbit-common.
    let contract = connected();
    assert_eq!(contract.sale_quote(U128(TOKEN)), U128(SALE_PRICE));
    assert_eq!(contract.sale_quote(U128(1)), U128(SALE_PRICE / TOKEN));
}

#[test]
fn creator_withdraws_unsold_tokens_once_after_the_end() {
    let mut contract = connected();
    buy(&mut contract, "carol.testnet", 5 * TOKEN, None, 250);
    setup(CREATOR, 0, SALE_END);
    assert_eq!(contract.withdraw_unsold(), U128(SALE_TOTAL - 5 * TOKEN));
    assert_eq!(balance(&contract, CREATOR), CREATOR_SHARE + SALE_TOTAL - 5 * TOKEN);
    assert_eq!(balance(&contract, LAUNCH), CLAIM_RESERVE, "the claim reserve is untouched");
    assert_eq!(contract.sale_info().status, SaleStatus::Closed);
}

#[test]
#[should_panic(expected = "unsold tokens have already been withdrawn")]
fn unsold_tokens_are_withdrawn_once() {
    let mut contract = connected();
    setup(CREATOR, 0, SALE_END);
    contract.withdraw_unsold();
    contract.withdraw_unsold();
}

#[test]
#[should_panic(expected = "the sale has not ended")]
fn unsold_tokens_wait_for_the_end() {
    let mut contract = connected();
    setup(CREATOR, 0, 250);
    contract.withdraw_unsold();
}

#[test]
#[should_panic(expected = "only the creator can withdraw unsold tokens")]
fn only_the_creator_withdraws() {
    let mut contract = connected();
    setup("mallory.testnet", 0, SALE_END);
    contract.withdraw_unsold();
}

#[test]
fn sale_and_claims_draw_from_separate_reserves() {
    let mut contract = connected();
    mint(&mut contract, "alice.testnet", 10);
    for (id, now) in (1..=10).zip([150u64; 10]) {
        buy(&mut contract, "alice.testnet", 10 * TOKEN, Some(&id.to_string()), now);
    }
    assert_eq!(contract.sale_info().sold, U128(SALE_TOTAL));
    for id in 1..=10 {
        setup("alice.testnet", STORAGE, 150);
        contract.claim(id.to_string());
    }
    assert_eq!(balance(&contract, LAUNCH), 0);
    assert_eq!(balance(&contract, "alice.testnet"), SALE_TOTAL + CLAIM_RESERVE);
}
