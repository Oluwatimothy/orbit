use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::test_utils::{get_created_receipts, get_logs, VMContextBuilder};
use near_sdk::{testing_env, AccountId, NearToken, PromiseResult, RuntimeFeesConfig};
use orbit_common::config::*;

use crate::*;

const FACTORY: &str = "orbit-factory.testnet";
const CREATOR: &str = "creator.testnet";

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn hash(byte: u8) -> Base58CryptoHash {
    [byte; 32].into()
}

fn context(predecessor: &str, deposit: NearToken, results: Vec<PromiseResult>) {
    let ctx = VMContextBuilder::new()
        .current_account_id(account(FACTORY))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(deposit)
        .account_balance(NearToken::from_near(50))
        .build();
    testing_env!(ctx, near_sdk::test_vm_config(), RuntimeFeesConfig::test(), Default::default(), results);
}

fn factory() -> Contract {
    context(FACTORY, NearToken::from_yoctonear(0), vec![]);
    Contract::new(
        account("owner.testnet"),
        CodeHashes { token: hash(1), nft: hash(2), claim: hash(3), sale: hash(4) },
    )
}

fn token(allocations: Vec<(&str, u128)>) -> TokenConfig {
    TokenConfig {
        name: "Orbit Genesis".into(),
        symbol: "ORBIT".into(),
        decimals: 18,
        icon: None,
        total_supply: U128(allocations.iter().map(|(_, a)| a).sum()),
        allocations: allocations
            .into_iter()
            .map(|(id, amount)| TokenAllocation {
                account_id: account(id),
                amount: U128(amount),
                label: "Allocation".into(),
            })
            .collect(),
    }
}

fn collection(max: u32) -> CollectionConfig {
    CollectionConfig {
        name: "Genesis".into(),
        symbol: "GEN".into(),
        description: "Genesis NFTs".into(),
        cover_media: "ipfs://bafycover".into(),
        media: TokenMedia::Shared { media: "ipfs://bafymedia".into(), media_hash: None },
        supply: Supply::Fixed { max },
        price: U128(0),
        starts_at_ns: U64(0),
        ends_at_ns: None,
        max_per_wallet: None,
        public_mint: true,
    }
}

fn claim_config() -> ClaimConfig {
    ClaimConfig {
        tiers: vec![Tier { name: "Genesis".into(), from: 1, to: 100, amount_per_nft: U128(1_000) }],
        starts_at_ns: U64(0),
    }
}

fn deposit(contract: &Contract, kind: LaunchKind) -> NearToken {
    NearToken::from_yoctonear(contract.quote_deposit(kind).0)
}

fn create_token(contract: &mut Contract, slug: &str) {
    context(CREATOR, deposit(contract, LaunchKind::Token), vec![]);
    let _ = contract.create_token_launch(slug.into(), "A token".into(), token(vec![(CREATOR, 1_000)]));
}

#[test]
fn token_launch_is_registered_as_deploying() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    let launch = contract.get_launch(0).unwrap();
    assert_eq!(launch.kind, LaunchKind::Token);
    assert_eq!(launch.status, LaunchStatus::Deploying);
    assert_eq!(launch.creator, account(CREATOR));
    assert_eq!(launch.token, Some(account("genesis-ft.orbit-factory.testnet")));
    assert_eq!((launch.nft, launch.claim), (None, None));
    assert!(!contract.is_slug_available("genesis".into()));
    assert!(get_logs().iter().any(|l| l.contains("launch_created")));
}

#[test]
fn token_launch_schedules_one_deployment_and_refunds_excess() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Token), vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), token(vec![(CREATOR, 1)]));
    let receivers: Vec<String> = get_created_receipts().iter().map(|r| r.receiver_id.to_string()).collect();
    assert!(receivers.contains(&"genesis-ft.orbit-factory.testnet".to_string()));
    assert!(receivers.contains(&CREATOR.to_string()), "unused registry allowance is refunded");
}

#[test]
#[should_panic(expected = "attach at least")]
fn launch_requires_deposit_for_account_balances() {
    let mut contract = factory();
    context(CREATOR, TOKEN_ACCOUNT_BALANCE, vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), token(vec![(CREATOR, 1)]));
}

#[test]
#[should_panic(expected = "this slug is already taken")]
fn slugs_cannot_be_reused() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    create_token(&mut contract, "genesis");
}

#[test]
#[should_panic(expected = "slug must be")]
fn invalid_slug_is_rejected() {
    let mut contract = factory();
    create_token(&mut contract, "Bad_Slug");
}

#[test]
#[should_panic(expected = "allocations add up to")]
fn invalid_token_config_is_rejected_before_deployment() {
    let mut contract = factory();
    let mut config = token(vec![(CREATOR, 10)]);
    config.total_supply = U128(11);
    context(CREATOR, deposit(&contract, LaunchKind::Token), vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), config);
}

#[test]
fn connected_launch_registers_three_accounts() {
    let mut contract = factory();
    let claim_account = "genesis-claim.orbit-factory.testnet";
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), (claim_account, 100_000)]),
        collection(100),
        Some(claim_config()),
        None,
    );
    let launch = contract.get_launch(0).unwrap();
    assert_eq!(launch.kind, LaunchKind::Connected);
    assert_eq!(launch.claim, Some(account(claim_account)));
    assert_eq!(launch.nft, Some(account("genesis-nft.orbit-factory.testnet")));
}

#[test]
#[should_panic(expected = "the token must allocate exactly 100000")]
fn connected_launch_requires_exact_holder_funding() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), ("genesis-claim.orbit-factory.testnet", 99_999)]),
        collection(100),
        Some(claim_config()),
        None,
    );
}

#[test]
#[should_panic(expected = "must cover IDs between 1 and 50")]
fn connected_launch_tiers_must_fit_collection_supply() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), ("genesis-claim.orbit-factory.testnet", 100_000)]),
        collection(50),
        Some(claim_config()),
        None,
    );
}

#[test]
fn successful_deployment_marks_launch_live() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    context(FACTORY, NearToken::from_yoctonear(0), vec![PromiseResult::Successful(vec![])]);
    let _ = contract.on_launch_deployed(0, vec![U128(TOKEN_ACCOUNT_BALANCE.as_yoctonear())]);
    assert_eq!(contract.get_launch(0).unwrap().status, LaunchStatus::Live);
    assert!(get_logs().iter().any(|l| l.contains("launch_live")));
}

#[test]
fn failed_deployment_marks_launch_failed_and_refunds_creator() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    context(FACTORY, NearToken::from_yoctonear(0), vec![PromiseResult::Failed]);
    let _ = contract.on_launch_deployed(0, vec![U128(TOKEN_ACCOUNT_BALANCE.as_yoctonear())]);
    assert_eq!(contract.get_launch(0).unwrap().status, LaunchStatus::Failed);
    assert!(get_created_receipts().iter().any(|r| r.receiver_id == account(CREATOR)));
    assert!(get_logs().iter().any(|l| l.contains("launch_failed")));
}

#[test]
fn partial_connected_failure_refunds_only_failed_accounts() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), ("genesis-claim.orbit-factory.testnet", 100_000)]),
        collection(100),
        Some(claim_config()),
        None,
    );
    let refunds = vec![U128(1), U128(20), U128(300)];
    let results =
        vec![PromiseResult::Successful(vec![]), PromiseResult::Failed, PromiseResult::Successful(vec![])];
    context(FACTORY, NearToken::from_yoctonear(0), results);
    let _ = contract.on_launch_deployed(0, refunds);
    assert_eq!(contract.get_launch(0).unwrap().status, LaunchStatus::Failed);
    assert!(get_logs().iter().any(|l| l.contains(r#""refunded":"20""#)));
}

#[test]
fn pagination_and_creator_index() {
    let mut contract = factory();
    for i in 0..5 {
        create_token(&mut contract, &format!("launch-{i}"));
    }
    assert_eq!(contract.get_launch_count(), 5);
    let page = contract.get_launches(Some(3), Some(10));
    assert_eq!(page.iter().map(|l| l.id).collect::<Vec<_>>(), vec![3, 4]);
    assert_eq!(contract.get_launches_by_creator(account(CREATOR), Some(1), Some(2)).len(), 2);
    assert!(contract.get_launches_by_creator(account("nobody.testnet"), None, None).is_empty());
    assert_eq!(contract.get_launch(99), None);
}

#[test]
#[should_panic(expected = "limit must be at most 50")]
fn page_size_is_bounded() {
    factory().get_launches(None, Some(51));
}

#[test]
#[should_panic(expected = "new launches are paused")]
fn paused_factory_rejects_launches() {
    let mut contract = factory();
    context("owner.testnet", NearToken::from_yoctonear(1), vec![]);
    contract.set_paused(true);
    create_token(&mut contract, "genesis");
}

#[test]
#[should_panic(expected = "only the factory owner")]
fn admin_methods_reject_non_owner() {
    let mut contract = factory();
    context("mallory.testnet", NearToken::from_yoctonear(1), vec![]);
    contract.set_code_hashes(CodeHashes { token: hash(9), nft: hash(9), claim: hash(9), sale: hash(9) });
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn admin_methods_require_one_yocto() {
    let mut contract = factory();
    context("owner.testnet", NearToken::from_yoctonear(0), vec![]);
    contract.set_paused(true);
}

#[test]
fn quote_covers_all_account_balances() {
    let contract = factory();
    let connected = contract.quote_deposit(LaunchKind::Connected).0;
    let balances = TOKEN_ACCOUNT_BALANCE.as_yoctonear()
        + NFT_ACCOUNT_BALANCE.as_yoctonear()
        + CLAIM_ACCOUNT_BALANCE.as_yoctonear()
        + SALE_ACCOUNT_BALANCE.as_yoctonear();
    assert_eq!(connected, balances + REGISTRY_STORAGE_ALLOWANCE.as_yoctonear());
}

fn sale_config(total: u128) -> SaleConfig {
    SaleConfig {
        price_per_token: U128(10u128.pow(22)),
        total_for_sale: U128(total),
        per_nft_limit: U128(100),
        holder_starts_at_ns: U64(0),
        public_starts_at_ns: None,
        per_account_public_limit: None,
        ends_at_ns: U64(1_000),
    }
}

#[test]
fn connected_launch_with_claims_and_sale_registers_four_accounts() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![
            (CREATOR, 850_000),
            ("genesis-claim.orbit-factory.testnet", 100_000),
            ("genesis-sale.orbit-factory.testnet", 50_000),
        ]),
        collection(100),
        Some(claim_config()),
        Some(sale_config(50_000)),
    );
    let launch = contract.get_launch(0).unwrap();
    assert_eq!(launch.sale, Some(account("genesis-sale.orbit-factory.testnet")));
    assert_eq!(launch.claim, Some(account("genesis-claim.orbit-factory.testnet")));
}

#[test]
fn sale_only_connected_launch_is_allowed() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 950_000), ("genesis-sale.orbit-factory.testnet", 50_000)]),
        collection(100),
        None,
        Some(sale_config(50_000)),
    );
    let launch = contract.get_launch(0).unwrap();
    assert_eq!((launch.claim, launch.sale.is_some()), (None, true));
}

#[test]
#[should_panic(expected = "the token must allocate exactly 50000")]
fn sale_must_be_funded_exactly() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 960_000), ("genesis-sale.orbit-factory.testnet", 40_000)]),
        collection(100),
        None,
        Some(sale_config(50_000)),
    );
}

#[test]
#[should_panic(expected = "needs holder claims, a gated sale, or both")]
fn connected_launch_needs_a_relationship() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 1_000)]),
        collection(100),
        None,
        None,
    );
}

#[test]
fn live_connected_launch_activates_its_distributors() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract, LaunchKind::Connected), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), ("genesis-claim.orbit-factory.testnet", 100_000)]),
        collection(100),
        Some(claim_config()),
        None,
    );
    let results = (0..3).map(|_| PromiseResult::Successful(vec![])).collect();
    context(FACTORY, NearToken::from_yoctonear(0), results);
    assert!(contract.on_launch_deployed(0, vec![U128(1), U128(2), U128(3)]));
    assert!(get_created_receipts()
        .iter()
        .any(|r| r.receiver_id == account("genesis-claim.orbit-factory.testnet")));
}
