use near_sdk::json_types::{U128, U64};
use near_sdk::AccountId;

use crate::config::*;
use crate::limits::*;
use crate::validate::*;

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn allocation(name: &str, amount: u128) -> TokenAllocation {
    TokenAllocation { account_id: account(name), amount: U128(amount), label: "Team".into() }
}

fn token(total: u128, allocations: Vec<TokenAllocation>) -> TokenConfig {
    TokenConfig {
        name: "Orbit Genesis".into(),
        symbol: "ORBIT".into(),
        decimals: 18,
        icon: None,
        total_supply: U128(total),
        allocations,
    }
}

fn collection(supply: Supply) -> CollectionConfig {
    CollectionConfig {
        name: "Genesis".into(),
        symbol: "GEN".into(),
        description: "A collection".into(),
        cover_media: "ipfs://bafycover".into(),
        media: TokenMedia::Shared { media: "ipfs://bafymedia".into(), media_hash: None },
        supply,
        price: U128(0),
        starts_at_ns: U64(0),
        ends_at_ns: None,
        max_per_wallet: None,
        public_mint: true,
    }
}

fn tier(from: u32, to: u32, amount: u128) -> Tier {
    Tier { name: format!("{from}-{to}"), from, to, amount_per_nft: U128(amount) }
}

#[test]
fn token_allocations_must_sum_to_supply() {
    let ok = token(100, vec![allocation("a.testnet", 60), allocation("b.testnet", 40)]);
    assert_eq!(validate_token_config(&ok), Ok(()));

    let short = token(100, vec![allocation("a.testnet", 99)]);
    assert!(validate_token_config(&short).unwrap_err().contains("add up to 99"));

    let over = token(100, vec![allocation("a.testnet", 101)]);
    assert!(validate_token_config(&over).is_err());
}

#[test]
fn token_rejects_zero_duplicate_and_overflowing_allocations() {
    let zero = token(10, vec![allocation("a.testnet", 10), allocation("b.testnet", 0)]);
    assert!(validate_token_config(&zero).unwrap_err().contains("greater than zero"));

    let duplicate = token(10, vec![allocation("a.testnet", 5), allocation("a.testnet", 5)]);
    assert!(validate_token_config(&duplicate).unwrap_err().contains("more than one"));

    let overflow = token(u128::MAX, vec![allocation("a.testnet", u128::MAX), allocation("b.testnet", 1)]);
    assert_eq!(validate_token_config(&overflow), Err("allocations overflow".into()));
}

#[test]
fn token_allocation_count_is_bounded() {
    let at_limit: Vec<_> = (0..MAX_ALLOCATIONS).map(|i| allocation(&format!("a{i}.testnet"), 1)).collect();
    assert_eq!(validate_token_config(&token(MAX_ALLOCATIONS as u128, at_limit)), Ok(()));

    let over: Vec<_> = (0..=MAX_ALLOCATIONS).map(|i| allocation(&format!("a{i}.testnet"), 1)).collect();
    assert!(validate_token_config(&token(MAX_ALLOCATIONS as u128 + 1, over)).is_err());
    assert!(validate_token_config(&token(0, vec![])).is_err());
}

#[test]
fn token_rejects_bad_metadata() {
    let mut config = token(1, vec![allocation("a.testnet", 1)]);
    config.decimals = 25;
    assert!(validate_token_config(&config).is_err());

    let mut config = token(1, vec![allocation("a.testnet", 1)]);
    config.symbol = "orbit".into();
    assert!(validate_token_config(&config).is_err());

    let mut config = token(1, vec![allocation("a.testnet", 1)]);
    config.name = "   ".into();
    assert!(validate_token_config(&config).is_err());
}

#[test]
fn media_urls_are_allow_listed() {
    assert_eq!(validate_media_url("m", "ipfs://bafy123/1.png", false), Ok(()));
    assert_eq!(validate_media_url("m", "https://gateway.example/ipfs/bafy", false), Ok(()));
    assert_eq!(validate_media_url("m", "data:image/png;base64,iVBORw0KGgo=", true), Ok(()));

    assert!(validate_media_url("m", "data:image/png;base64,iVBORw0KGgo=", false).is_err());
    assert!(validate_media_url("m", "javascript:alert(1)", false).is_err());
    assert!(validate_media_url("m", "http://insecure.example/a.png", false).is_err());
    assert!(validate_media_url("m", "https://a.example/\"onerror=", false).is_err());
    assert!(validate_media_url("m", "data:text/html;base64,PGgxPg==", true).is_err());
    assert!(validate_media_url("m", "data:image/svg+xml;base64,<svg>", true).is_err());
    let too_long = format!("https://{}", "a".repeat(MAX_URL_LEN));
    assert!(validate_media_url("m", &too_long, false).is_err());
}

#[test]
fn collection_supply_and_window_are_validated() {
    assert_eq!(validate_collection_config(&collection(Supply::Fixed { max: 1 })), Ok(()));
    assert_eq!(validate_collection_config(&collection(Supply::Unlimited)), Ok(()));
    assert!(validate_collection_config(&collection(Supply::Fixed { max: 0 })).is_err());
    assert!(validate_collection_config(&collection(Supply::Fixed { max: MAX_FIXED_SUPPLY + 1 })).is_err());

    let mut config = collection(Supply::Fixed { max: 10 });
    config.starts_at_ns = U64(100);
    config.ends_at_ns = Some(U64(100));
    assert!(validate_collection_config(&config).is_err());

    let mut config = collection(Supply::Fixed { max: 10 });
    config.max_per_wallet = Some(0);
    assert!(validate_collection_config(&config).is_err());
}

#[test]
fn directory_media_requires_https_base_and_known_extension() {
    let mut config = collection(Supply::Fixed { max: 10 });
    config.media =
        TokenMedia::Directory { base_uri: "https://gw.example/ipfs/bafy".into(), extension: "png".into() };
    assert_eq!(validate_collection_config(&config), Ok(()));

    config.media = TokenMedia::Directory { base_uri: "ipfs://bafy".into(), extension: "png".into() };
    assert!(validate_collection_config(&config).is_err());

    config.media =
        TokenMedia::Directory { base_uri: "https://gw.example/ipfs/bafy".into(), extension: "exe".into() };
    assert!(validate_collection_config(&config).is_err());
}

#[test]
fn claim_tiers_compute_exact_total() {
    let config = ClaimConfig {
        tiers: vec![tier(1, 1000, 1_000), tier(1001, 1900, 5_000), tier(1901, 2000, 10_000)],
        starts_at_ns: U64(0),
    };
    let total = validate_claim_config(&config, Supply::Fixed { max: 2000 }).unwrap();
    assert_eq!(total, 1000 * 1_000 + 900 * 5_000 + 100 * 10_000);
}

#[test]
fn claim_tiers_reject_overlap_gaps_out_of_range_and_unlimited() {
    let supply = Supply::Fixed { max: 100 };
    let claim = |tiers| ClaimConfig { tiers, starts_at_ns: U64(0) };

    assert!(validate_claim_config(&claim(vec![tier(1, 50, 1), tier(50, 100, 1)]), supply)
        .unwrap_err()
        .contains("overlaps"));
    assert!(validate_claim_config(&claim(vec![tier(0, 10, 1)]), supply).is_err());
    assert!(validate_claim_config(&claim(vec![tier(10, 5, 1)]), supply).is_err());
    assert!(validate_claim_config(&claim(vec![tier(1, 101, 1)]), supply).is_err());
    assert!(validate_claim_config(&claim(vec![tier(1, 10, 0)]), supply).is_err());
    assert!(validate_claim_config(&claim(vec![]), supply).is_err());
    assert!(validate_claim_config(&claim(vec![tier(1, 10, 1)]), Supply::Unlimited).is_err());
    // Gaps are allowed: IDs outside every tier are simply not eligible.
    assert_eq!(validate_claim_config(&claim(vec![tier(1, 10, 2), tier(20, 30, 1)]), supply), Ok(31));
}

#[test]
fn claim_total_overflow_is_rejected() {
    let config = ClaimConfig { tiers: vec![tier(1, 100, u128::MAX)], starts_at_ns: U64(0) };
    assert!(validate_claim_config(&config, Supply::Fixed { max: 100 }).is_err());
}

#[test]
fn slugs_and_launch_accounts() {
    for ok in ["ab", "genesis", "orbit-genesis-2"] {
        assert_eq!(validate_slug(ok), Ok(()), "{ok}");
    }
    for bad in ["a", "-ab", "ab-", "a--b", "Genesis", "a_b", "a.b", "abcdefghijklmnopqrstu"] {
        assert!(validate_slug(bad).is_err(), "{bad}");
    }
    let factory = account("orbit-factory.testnet");
    assert_eq!(launch_account_id("genesis", &factory).unwrap().as_str(), "genesis.orbit-factory.testnet");
}

fn sale() -> SaleConfig {
    SaleConfig {
        price_per_token: U128(10u128.pow(22)),
        total_for_sale: U128(1_000 * 10u128.pow(18)),
        per_nft_limit: U128(100 * 10u128.pow(18)),
        holder_starts_at_ns: U64(0),
        public_starts_at_ns: Some(U64(100)),
        per_account_public_limit: Some(U128(10 * 10u128.pow(18))),
        ends_at_ns: U64(200),
    }
}

#[test]
fn sale_windows_and_limits_are_validated() {
    let fixed = Supply::Fixed { max: 10 };
    assert_eq!(validate_sale_config(&sale(), fixed), Ok(()));
    assert!(validate_sale_config(&sale(), Supply::Unlimited).is_err());

    let mut s = sale();
    s.public_starts_at_ns = Some(U64(0));
    assert!(validate_sale_config(&s, fixed).is_err(), "public must start after holders");
    let mut s = sale();
    s.public_starts_at_ns = Some(U64(200));
    assert!(validate_sale_config(&s, fixed).is_err(), "public must start before end");
    let mut s = sale();
    s.per_account_public_limit = None;
    assert!(validate_sale_config(&s, fixed).is_err(), "public window needs a limit");
    let mut s = sale();
    s.public_starts_at_ns = None;
    s.per_account_public_limit = None;
    assert_eq!(validate_sale_config(&s, fixed), Ok(()), "holder-only sale");
    s.ends_at_ns = U64(0);
    assert!(validate_sale_config(&s, fixed).is_err());
    let mut s = sale();
    s.price_per_token = U128(0);
    assert!(validate_sale_config(&s, fixed).is_err());
}

#[test]
fn sale_cost_rounds_up_and_detects_overflow() {
    // 1.5 tokens at 2 NEAR each (18 decimals) = 3 NEAR.
    let near = 10u128.pow(24);
    assert_eq!(sale_cost(15 * 10u128.pow(17), 2 * near, 18), Some(3 * near));
    // One base unit at 1 yocto per whole token rounds up to 1 yocto.
    assert_eq!(sale_cost(1, 1, 18), Some(1));
    // 10M tokens (18 decimals) at 1,000 NEAR each fits only through 256-bit math.
    assert_eq!(sale_cost(10_000_000 * 10u128.pow(18), 1_000 * near, 18), Some(10_000_000_000 * near));
    assert_eq!(sale_cost(u128::MAX, 2, 0), None);
}

fn parts<'a>(
    token: Option<&'a TokenConfig>,
    collection: Option<&'a CollectionConfig>,
    claim: Option<&'a ClaimConfig>,
    sale: Option<&'a SaleConfig>,
) -> LaunchParts<'a> {
    LaunchParts { token, collection, claim, sale }
}

#[test]
fn launch_reserves_must_be_allocated_to_the_launch_account() {
    let launch = account("genesis.factory.testnet");
    let fixed = collection(Supply::Fixed { max: 10 });
    let claim = ClaimConfig { tiers: vec![tier(1, 10, 100)], starts_at_ns: U64(0) };
    let sale = sale();
    let sale_total = sale.total_for_sale.0;
    let funded = |reserve: u128| {
        token(reserve + 50, vec![allocation("genesis.factory.testnet", reserve), allocation("a.testnet", 50)])
    };

    let both = funded(1_000 + sale_total);
    assert_eq!(
        validate_launch(parts(Some(&both), Some(&fixed), Some(&claim), Some(&sale)), &launch),
        Ok(1_000)
    );
    let claims_only = funded(1_000);
    assert_eq!(
        validate_launch(parts(Some(&claims_only), Some(&fixed), Some(&claim), None), &launch),
        Ok(1_000)
    );
    let sale_only = funded(sale_total);
    assert_eq!(validate_launch(parts(Some(&sale_only), Some(&fixed), None, Some(&sale)), &launch), Ok(0));

    let short = funded(999 + sale_total);
    let err =
        validate_launch(parts(Some(&short), Some(&fixed), Some(&claim), Some(&sale)), &launch).unwrap_err();
    assert!(err.contains("must allocate exactly"), "{err}");
}

#[test]
fn launch_shape_rules() {
    let launch = account("genesis.factory.testnet");
    let plain = token(100, vec![allocation("a.testnet", 100)]);
    let fixed = collection(Supply::Fixed { max: 10 });
    let claim = ClaimConfig { tiers: vec![tier(1, 10, 1)], starts_at_ns: U64(0) };

    assert_eq!(validate_launch(parts(Some(&plain), None, None, None), &launch), Ok(0));
    assert_eq!(validate_launch(parts(None, Some(&fixed), None, None), &launch), Ok(0));
    assert!(validate_launch(parts(None, None, None, None), &launch).is_err());
    let err = validate_launch(parts(Some(&plain), Some(&fixed), None, None), &launch).unwrap_err();
    assert!(err.contains("needs holder claims"), "{err}");
    let err = validate_launch(parts(Some(&plain), None, Some(&claim), None), &launch).unwrap_err();
    assert!(err.contains("both a token and a collection"), "{err}");

    // Without claims or a sale, tokens sent to the launch account would be locked.
    let locked = token(100, vec![allocation("genesis.factory.testnet", 100)]);
    let err = validate_launch(parts(Some(&locked), None, None, None), &launch).unwrap_err();
    assert!(err.contains("locked"), "{err}");

    // Claims need a fixed supply.
    let unlimited = collection(Supply::Unlimited);
    let funded = token(10, vec![allocation("genesis.factory.testnet", 10)]);
    assert!(validate_launch(parts(Some(&funded), Some(&unlimited), Some(&claim), None), &launch).is_err());
}
