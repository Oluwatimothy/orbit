//! Validation shared by the factory (before deployment) and each asset
//! contract (in `new`). Returning `Err` instead of panicking keeps the rules
//! testable without a mocked blockchain.

use std::collections::HashSet;

use near_sdk::AccountId;

use crate::config::{ClaimConfig, CollectionConfig, SaleConfig, Supply, TokenConfig, TokenMedia};
use crate::limits::*;

pub type ValidationResult<T = ()> = Result<T, String>;

const DATA_URI_PREFIXES: [&str; 5] = [
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/webp;base64,",
    "data:image/gif;base64,",
    "data:image/svg+xml;base64,",
];
const DIRECTORY_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "svg"];

pub fn validate_text(field: &str, value: &str, max_len: usize) -> ValidationResult {
    let len = value.chars().count();
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if len > max_len {
        return Err(format!("{field} must be at most {max_len} characters"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(())
}

pub fn validate_symbol(symbol: &str) -> ValidationResult {
    let valid_chars = symbol.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if !(MIN_SYMBOL_LEN..=MAX_SYMBOL_LEN).contains(&symbol.len()) || !valid_chars {
        return Err(format!("symbol must be {MIN_SYMBOL_LEN}-{MAX_SYMBOL_LEN} uppercase letters or digits"));
    }
    Ok(())
}

fn is_url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&()*+,;=%".contains(c)
}

/// Accepts `ipfs://…` and `https://…` references, and optionally small inline
/// `data:image/…;base64,` URIs. Anything else (including `javascript:` and
/// `http:`) is rejected.
pub fn validate_media_url(field: &str, url: &str, allow_data_uri: bool) -> ValidationResult {
    if let Some(prefix) = DATA_URI_PREFIXES.iter().find(|p| url.starts_with(**p)) {
        if !allow_data_uri {
            return Err(format!("{field} must be an ipfs:// or https:// URL"));
        }
        if url.len() > MAX_DATA_URI_LEN {
            return Err(format!("{field} data URI must be at most {MAX_DATA_URI_LEN} bytes"));
        }
        let payload = &url[prefix.len()..];
        let is_base64 = !payload.is_empty()
            && payload.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');
        return if is_base64 { Ok(()) } else { Err(format!("{field} data URI payload must be base64")) };
    }

    let rest = url
        .strip_prefix("ipfs://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| format!("{field} must start with ipfs:// or https://"))?;
    if rest.is_empty() || url.len() > MAX_URL_LEN {
        return Err(format!("{field} must be a non-empty URL of at most {MAX_URL_LEN} characters"));
    }
    if !rest.chars().all(is_url_char) {
        return Err(format!("{field} contains characters that are not allowed in a URL"));
    }
    Ok(())
}

/// Total supply must be positive and exactly equal to the sum of allocations.
pub fn validate_token_config(config: &TokenConfig) -> ValidationResult {
    validate_text("token name", &config.name, MAX_NAME_LEN)?;
    validate_symbol(&config.symbol)?;
    if config.decimals > MAX_DECIMALS {
        return Err(format!("decimals must be at most {MAX_DECIMALS}"));
    }
    if let Some(icon) = &config.icon {
        validate_media_url("icon", icon, true)?;
    }
    let total_supply = config.total_supply.0;
    if total_supply == 0 {
        return Err("total supply must be greater than zero".into());
    }
    if config.allocations.is_empty() || config.allocations.len() > MAX_ALLOCATIONS {
        return Err(format!("between 1 and {MAX_ALLOCATIONS} allocations are required"));
    }

    let mut seen = HashSet::new();
    let mut sum: u128 = 0;
    for allocation in &config.allocations {
        validate_text("allocation label", &allocation.label, MAX_LABEL_LEN)?;
        if allocation.amount.0 == 0 {
            return Err("every allocation must be greater than zero".into());
        }
        if !seen.insert(&allocation.account_id) {
            return Err(format!("{} appears in more than one allocation", allocation.account_id));
        }
        sum = sum.checked_add(allocation.amount.0).ok_or("allocations overflow")?;
    }
    if sum != total_supply {
        return Err(format!("allocations add up to {sum} but total supply is {total_supply}"));
    }
    Ok(())
}

pub fn validate_collection_config(config: &CollectionConfig) -> ValidationResult {
    validate_text("collection name", &config.name, MAX_NAME_LEN)?;
    validate_symbol(&config.symbol)?;
    validate_text("description", &config.description, MAX_DESCRIPTION_LEN)?;
    validate_media_url("cover media", &config.cover_media, false)?;

    match &config.media {
        TokenMedia::Shared { media, .. } => validate_media_url("token media", media, false)?,
        TokenMedia::Directory { base_uri, extension } => {
            if !base_uri.starts_with("https://") {
                return Err("media base URI must be an https:// gateway URL".into());
            }
            validate_media_url("media base URI", base_uri, false)?;
            if base_uri.ends_with('/') {
                return Err("media base URI must not end with '/'".into());
            }
            if !DIRECTORY_EXTENSIONS.contains(&extension.as_str()) {
                return Err(format!("media extension must be one of {DIRECTORY_EXTENSIONS:?}"));
            }
        }
    }

    if let Supply::Fixed { max } = config.supply {
        if max == 0 || max > MAX_FIXED_SUPPLY {
            return Err(format!("fixed supply must be between 1 and {MAX_FIXED_SUPPLY}"));
        }
    }
    if let Some(ends_at) = config.ends_at_ns {
        if ends_at.0 <= config.starts_at_ns.0 {
            return Err("mint end time must be after start time".into());
        }
    }
    if config.max_per_wallet == Some(0) {
        return Err("max per wallet must be at least 1 when set".into());
    }
    Ok(())
}

/// Returns the total token amount the claim contract must hold.
pub fn validate_claim_config(config: &ClaimConfig, supply: Supply) -> ValidationResult<u128> {
    let max_id = supply.max().ok_or("connected launches require a fixed NFT supply")?;
    if config.tiers.is_empty() || config.tiers.len() > MAX_TIERS {
        return Err(format!("between 1 and {MAX_TIERS} tiers are required"));
    }

    let mut sorted: Vec<_> = config.tiers.iter().collect();
    sorted.sort_by_key(|tier| tier.from);
    let mut previous_to = 0u32;
    let mut total: u128 = 0;
    for tier in sorted {
        validate_text("tier name", &tier.name, MAX_LABEL_LEN)?;
        if tier.from == 0 || tier.from > tier.to || tier.to > max_id {
            return Err(format!(
                "tier '{}' must cover IDs between 1 and {max_id} with from <= to",
                tier.name
            ));
        }
        if tier.from <= previous_to {
            return Err(format!("tier '{}' overlaps another tier", tier.name));
        }
        if tier.amount_per_nft.0 == 0 {
            return Err(format!("tier '{}' must allocate a positive amount", tier.name));
        }
        let tier_total =
            tier.amount_per_nft.0.checked_mul(u128::from(tier.size())).ok_or("tier total overflows")?;
        total = total.checked_add(tier_total).ok_or("claim total overflows")?;
        previous_to = tier.to;
    }
    Ok(total)
}

pub fn validate_slug(slug: &str) -> ValidationResult {
    let chars_ok = slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !(MIN_SLUG_LEN..=MAX_SLUG_LEN).contains(&slug.len())
        || !chars_ok
        || slug.starts_with('-')
        || slug.ends_with('-')
        || slug.contains("--")
    {
        return Err(format!(
            "slug must be {MIN_SLUG_LEN}-{MAX_SLUG_LEN} lowercase letters, digits or single hyphens"
        ));
    }
    Ok(())
}

/// The account a launch is deployed to, e.g. `genesis.orbit-factory.testnet`.
pub fn launch_account_id(slug: &str, factory: &AccountId) -> ValidationResult<AccountId> {
    format!("{slug}.{factory}")
        .parse()
        .map_err(|_| format!("'{slug}' produces an invalid account ID under {factory}"))
}

/// The parts of one launch. A launch has a token, a collection, or both; with
/// both it also has holder claims, a gated sale, or both.
#[derive(Clone, Copy)]
pub struct LaunchParts<'a> {
    pub token: Option<&'a TokenConfig>,
    pub collection: Option<&'a CollectionConfig>,
    pub claim: Option<&'a ClaimConfig>,
    pub sale: Option<&'a SaleConfig>,
}

/// Validates every part and how they fit together. Claim and sale reserves
/// are held by the launch account itself, so the token must allocate exactly
/// their sum to `launch_account` (and nothing when there is neither).
/// Returns the claim reserve.
pub fn validate_launch(parts: LaunchParts, launch_account: &AccountId) -> ValidationResult<u128> {
    if let Some(token) = parts.token {
        validate_token_config(token)?;
    }
    if let Some(collection) = parts.collection {
        validate_collection_config(collection)?;
    }
    let (claim_reserve, sale_total) = match (parts.token, parts.collection) {
        (None, None) => return Err("a launch needs a token, a collection, or both".into()),
        (Some(token), Some(collection)) => {
            if parts.claim.is_none() && parts.sale.is_none() {
                return Err("a connected launch needs holder claims, a gated sale, or both".into());
            }
            let claim_reserve = match parts.claim {
                Some(claim) => validate_claim_config(claim, collection.supply)?,
                None => 0,
            };
            let sale_total = match parts.sale {
                Some(sale) => {
                    validate_sale_config(sale, collection.supply)?;
                    if sale_cost(sale.total_for_sale.0, sale.price_per_token.0, token.decimals).is_none() {
                        return Err("the full sale value does not fit in a NEAR amount".into());
                    }
                    sale.total_for_sale.0
                }
                None => 0,
            };
            (claim_reserve, sale_total)
        }
        _ if parts.claim.is_some() || parts.sale.is_some() => {
            return Err("holder claims and gated sales need both a token and a collection".into())
        }
        _ => (0, 0),
    };

    let reserve = claim_reserve.checked_add(sale_total).ok_or("reserves overflow")?;
    let allocated: u128 = parts
        .token
        .map(|t| t.allocations.iter().filter(|a| &a.account_id == launch_account).map(|a| a.amount.0).sum())
        .unwrap_or(0);
    if allocated != reserve {
        return Err(if reserve == 0 {
            format!(
                "the token cannot allocate to the launch account {launch_account}; tokens there are locked"
            )
        } else {
            format!(
                "the token must allocate exactly {reserve} to {launch_account} to fund claims and the sale"
            )
        });
    }
    Ok(claim_reserve)
}

pub fn validate_sale_config(config: &SaleConfig, supply: Supply) -> ValidationResult {
    supply.max().ok_or("connected launches require a fixed NFT supply")?;
    if config.price_per_token.0 == 0 {
        return Err("sale price must be greater than zero".into());
    }
    if config.total_for_sale.0 == 0 {
        return Err("tokens for sale must be greater than zero".into());
    }
    if config.per_nft_limit.0 == 0 {
        return Err("per-NFT purchase limit must be greater than zero".into());
    }
    let holder_start = config.holder_starts_at_ns.0;
    let end = config.ends_at_ns.0;
    match (config.public_starts_at_ns, config.per_account_public_limit) {
        (Some(public_start), Some(limit)) => {
            if public_start.0 <= holder_start || public_start.0 >= end {
                return Err(
                    "the public window must start after the holder window and before the sale ends".into()
                );
            }
            if limit.0 == 0 {
                return Err("per-account public limit must be greater than zero".into());
            }
        }
        (None, None) => {
            if end <= holder_start {
                return Err("the sale must end after it starts".into());
            }
        }
        _ => return Err("a public window needs both a start time and a per-account limit".into()),
    }
    Ok(())
}

mod wide {
    // The lint fires inside the `uint` macro's generated code, not ours.
    #![allow(clippy::manual_div_ceil)]
    uint::construct_uint! {
        /// 256-bit integer for `amount × price`, which exceeds u128 at realistic scales.
        pub struct U256(4);
    }
}
use wide::U256;

/// Cost in yoctoNEAR of `amount` base units, rounded up so the seller is never
/// short-changed. `None` if the result does not fit in u128.
pub fn sale_cost(amount: u128, price_per_token: u128, decimals: u8) -> Option<u128> {
    let unit = U256::from(10u128.checked_pow(u32::from(decimals))?);
    let numerator = U256::from(amount) * U256::from(price_per_token);
    let (quotient, remainder) = numerator.div_mod(unit);
    let cost = if remainder.is_zero() { quotient } else { quotient + U256::one() };
    (cost <= U256::from(u128::MAX)).then(|| cost.as_u128())
}
