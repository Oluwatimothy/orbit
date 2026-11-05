//! Launch configuration passed from the factory to each asset contract's `new`.

use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::{near, AccountId};

/// One recipient of the initial token supply.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct TokenAllocation {
    pub account_id: AccountId,
    pub amount: U128,
    /// Human-readable category, e.g. "Treasury" or "NFT holders".
    pub label: String,
}

/// Fungible token configuration chosen by the creator.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub icon: Option<String>,
    pub total_supply: U128,
    pub allocations: Vec<TokenAllocation>,
}

/// Maximum number of NFTs a collection may ever contain.
#[near(serializers = [json, borsh])]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Supply {
    Fixed {
        max: u32,
    },
    /// Explicitly uncapped. Never the result of a missing field.
    Unlimited,
}

impl Supply {
    pub fn max(&self) -> Option<u32> {
        match self {
            Supply::Fixed { max } => Some(*max),
            Supply::Unlimited => None,
        }
    }
}

/// How each minted token's media is derived.
#[near(serializers = [json, borsh])]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenMedia {
    /// Every token shows the same artwork.
    Shared { media: String, media_hash: Option<Base64VecU8> },
    /// Token `n` uses `{base_uri}/{n}.{extension}` (NEP-177 relative media).
    Directory { base_uri: String, extension: String },
}

/// NFT collection configuration chosen by the creator.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionConfig {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub cover_media: String,
    pub media: TokenMedia,
    pub supply: Supply,
    /// Price per NFT in yoctoNEAR. Zero is a free mint.
    pub price: U128,
    pub starts_at_ns: U64,
    pub ends_at_ns: Option<U64>,
    pub max_per_wallet: Option<u32>,
    pub public_mint: bool,
}

/// A contiguous NFT ID range that receives a fixed token amount per NFT.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct Tier {
    pub name: String,
    pub from: u32,
    pub to: u32,
    pub amount_per_nft: U128,
}

impl Tier {
    pub fn contains(&self, id: u32) -> bool {
        self.from <= id && id <= self.to
    }

    pub fn size(&self) -> u32 {
        self.to - self.from + 1
    }
}

/// Holder-claim configuration for a connected launch.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimConfig {
    pub tiers: Vec<Tier>,
    pub starts_at_ns: U64,
}

/// NFT-gated token sale for a connected launch.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct SaleConfig {
    /// yoctoNEAR per whole token (10^decimals base units). Cost rounds up.
    pub price_per_token: U128,
    /// Base units the sale contract holds and may sell.
    pub total_for_sale: U128,
    /// Base units each eligible NFT may buy during the holder window.
    pub per_nft_limit: U128,
    pub holder_starts_at_ns: U64,
    /// When set, anyone may buy from this time, up to `per_account_public_limit`.
    pub public_starts_at_ns: Option<U64>,
    pub per_account_public_limit: Option<U128>,
    /// Required, so unsold tokens can be returned to the creator afterwards.
    pub ends_at_ns: U64,
}
