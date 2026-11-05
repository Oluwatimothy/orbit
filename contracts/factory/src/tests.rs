use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_created_receipts, get_logs, VMContextBuilder};
use near_sdk::{testing_env, AccountId, NearToken, PromiseResult, RuntimeFeesConfig};
use orbit_common::config::*;

use crate::*;

const FACTORY: &str = "orbit-factory.testnet";
const CREATOR: &str = "creator.testnet";
const LAUNCH: &str = "genesis.orbit-factory.testnet";
const TEMPLATE_SIZE: usize = 1_000;

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn context_with_input(predecessor: &str, deposit: NearToken, results: Vec<PromiseResult>, input: Vec<u8>) {
    let mut ctx = VMContextBuilder::new()
        .current_account_id(account(FACTORY))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(deposit)
        .account_balance(NearToken::from_near(50))
        .build();
    ctx.input = input.into();
    testing_env!(ctx, near_sdk::test_vm_config(), RuntimeFeesConfig::test(), Default::default(), results);
}

fn context(predecessor: &str, deposit: NearToken, results: Vec<PromiseResult>) {
    context_with_input(predecessor, deposit, results, vec![]);
}

fn template() -> Vec<u8> {
    let mut code = WASM_HEADER.to_vec();
    code.resize(TEMPLATE_SIZE, 7);
    code
}

fn bare_factory() -> Contract {
    context(FACTORY, NearToken::from_yoctonear(0), vec![]);
    Contract::new(account("owner.testnet"))
}

fn factory() -> Contract {
    let mut contract = bare_factory();
    context_with_input("owner.testnet", NearToken::from_yoctonear(1), vec![], template());
    contract.set_template();
    contract
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

fn deposit(contract: &Contract) -> NearToken {
    NearToken::from_yoctonear(contract.quote_deposit(LaunchKind::Connected).0)
}

fn create_token(contract: &mut Contract, slug: &str) {
    context(CREATOR, deposit(contract), vec![]);
    let _ = contract.create_token_launch(slug.into(), "A token".into(), token(vec![(CREATOR, 1_000)]));
}

fn create_connected(
    contract: &mut Contract,
    allocations: Vec<(&str, u128)>,
    claim: Option<ClaimConfig>,
    sale: Option<SaleConfig>,
) {
    context(CREATOR, deposit(contract), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(allocations),
        collection(100),
        claim,
        sale,
    );
}

#[test]
fn template_is_stored_by_hash_and_size() {
    let contract = factory();
    let info = contract.get_config().template.unwrap();
    assert_eq!(info.size, TEMPLATE_SIZE as u64);
    let expected: near_sdk::json_types::Base58CryptoHash = near_sdk::env::sha256_array(template()).into();
    assert_eq!(info.hash, expected);
    assert_eq!(near_sdk::env::storage_read(TEMPLATE_KEY), Some(template()));
}

#[test]
#[should_panic(expected = "the input must be a wasm module")]
fn template_must_be_wasm() {
    let mut contract = bare_factory();
    context_with_input("owner.testnet", NearToken::from_yoctonear(1), vec![], b"not wasm".to_vec());
    contract.set_template();
}

#[test]
#[should_panic(expected = "only the factory owner")]
fn only_the_owner_sets_the_template() {
    let mut contract = bare_factory();
    context_with_input("mallory.testnet", NearToken::from_yoctonear(1), vec![], template());
    contract.set_template();
}

#[test]
#[should_panic(expected = "no launch template has been set")]
fn launches_wait_for_a_template() {
    let mut contract = bare_factory();
    context(CREATOR, NearToken::from_near(10), vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), token(vec![(CREATOR, 1)]));
}

#[test]
fn quote_covers_code_storage_state_and_registry() {
    let contract = factory();
    let code = near_sdk::env::storage_byte_cost().as_yoctonear() * TEMPLATE_SIZE as u128;
    let expected = code + LAUNCH_STATE_ALLOWANCE.as_yoctonear() + REGISTRY_STORAGE_ALLOWANCE.as_yoctonear();
    for kind in [LaunchKind::Token, LaunchKind::Collection, LaunchKind::Connected] {
        assert_eq!(contract.quote_deposit(kind), U128(expected));
    }
}

#[test]
fn token_launch_is_registered_as_deploying() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    let launch = contract.get_launch(0).unwrap();
    assert_eq!(launch.kind, LaunchKind::Token);
    assert_eq!(launch.status, LaunchStatus::Deploying);
    assert_eq!(launch.creator, account(CREATOR));
    assert_eq!(launch.account, account(LAUNCH));
    assert_eq!(launch.token, Some(account(LAUNCH)));
    assert_eq!((launch.nft, launch.claim, launch.sale), (None, None, None));
    assert!(!contract.is_slug_available("genesis".into()));
    assert!(get_logs().iter().any(|l| l.contains("launch_created")));
}

#[test]
fn launch_deploys_the_template_and_refunds_excess() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract), vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), token(vec![(CREATOR, 1)]));
    let receipts = get_created_receipts();
    let deploy = receipts.iter().find(|r| r.receiver_id == account(LAUNCH)).expect("deployment receipt");
    let actions = format!("{:?}", deploy.actions);
    assert!(actions.contains("CreateAccount") && actions.contains("DeployContract"), "{actions}");
    assert!(
        receipts.iter().any(|r| r.receiver_id == account(CREATOR)),
        "unused registry allowance is refunded"
    );
}

#[test]
#[should_panic(expected = "attach at least")]
fn launch_requires_deposit_for_the_account_balance() {
    let mut contract = factory();
    context(CREATOR, LAUNCH_STATE_ALLOWANCE, vec![]);
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
    context(CREATOR, deposit(&contract), vec![]);
    let _ = contract.create_token_launch("genesis".into(), "A token".into(), config);
}

#[test]
fn connected_launch_with_claims_and_sale_uses_one_account() {
    let mut contract = factory();
    create_connected(
        &mut contract,
        vec![(CREATOR, 850_000), (LAUNCH, 150_000)],
        Some(claim_config()),
        Some(sale_config(50_000)),
    );
    let launch = contract.get_launch(0).unwrap();
    assert_eq!(launch.kind, LaunchKind::Connected);
    for part in [launch.token, launch.nft, launch.claim, launch.sale] {
        assert_eq!(part, Some(account(LAUNCH)));
    }
}

#[test]
fn sale_only_connected_launch_is_allowed() {
    let mut contract = factory();
    create_connected(
        &mut contract,
        vec![(CREATOR, 950_000), (LAUNCH, 50_000)],
        None,
        Some(sale_config(50_000)),
    );
    let launch = contract.get_launch(0).unwrap();
    assert_eq!((launch.claim, launch.sale.is_some()), (None, true));
}

#[test]
#[should_panic(expected = "the token must allocate exactly 100000")]
fn connected_launch_requires_exact_holder_funding() {
    let mut contract = factory();
    create_connected(&mut contract, vec![(CREATOR, 900_001), (LAUNCH, 99_999)], Some(claim_config()), None);
}

#[test]
#[should_panic(expected = "the token must allocate exactly 150000")]
fn connected_launch_funds_claims_and_sale_together() {
    let mut contract = factory();
    create_connected(
        &mut contract,
        vec![(CREATOR, 900_000), (LAUNCH, 100_000)],
        Some(claim_config()),
        Some(sale_config(50_000)),
    );
}

#[test]
#[should_panic(expected = "must cover IDs between 1 and 50")]
fn connected_launch_tiers_must_fit_collection_supply() {
    let mut contract = factory();
    context(CREATOR, deposit(&contract), vec![]);
    let _ = contract.create_connected_launch(
        "genesis".into(),
        "Connected".into(),
        token(vec![(CREATOR, 900_000), (LAUNCH, 100_000)]),
        collection(50),
        Some(claim_config()),
        None,
    );
}

#[test]
#[should_panic(expected = "needs holder claims, a gated sale, or both")]
fn connected_launch_needs_a_relationship() {
    let mut contract = factory();
    create_connected(&mut contract, vec![(CREATOR, 1_000)], None, None);
}

#[test]
fn successful_deployment_marks_launch_live() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    context(FACTORY, NearToken::from_yoctonear(0), vec![PromiseResult::Successful(vec![])]);
    assert!(contract.on_launch_deployed(0, U128(5)));
    assert_eq!(contract.get_launch(0).unwrap().status, LaunchStatus::Live);
    assert!(get_logs().iter().any(|l| l.contains("launch_live")));
}

#[test]
fn failed_deployment_marks_launch_failed_and_refunds_creator() {
    let mut contract = factory();
    create_token(&mut contract, "genesis");
    context(FACTORY, NearToken::from_yoctonear(0), vec![PromiseResult::Failed]);
    assert!(!contract.on_launch_deployed(0, U128(5)));
    assert_eq!(contract.get_launch(0).unwrap().status, LaunchStatus::Failed);
    assert!(get_created_receipts().iter().any(|r| r.receiver_id == account(CREATOR)));
    assert!(get_logs().iter().any(|l| l.contains(r#""refunded":"5""#)));
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
    contract.set_paused(true);
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn admin_methods_require_one_yocto() {
    let mut contract = factory();
    context("owner.testnet", NearToken::from_yoctonear(0), vec![]);
    contract.set_paused(true);
}
