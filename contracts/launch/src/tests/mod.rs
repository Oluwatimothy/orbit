//! Unit tests. Each module deploys the parts it needs; `connected()` deploys
//! a launch with a token, a collection, holder claims and a sale.

use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
use near_sdk::{testing_env, AccountId, NearToken};
use orbit_common::config::*;

use crate::Contract;

mod claim;
mod collection;
mod sale;
mod token;

pub const LAUNCH: &str = "genesis.factory.testnet";
pub const CREATOR: &str = "creator.testnet";
pub const NEAR: u128 = 10u128.pow(24);
pub const TOKEN: u128 = 10u128.pow(18);
/// More than any single call here stores; the contract refunds the excess.
pub const STORAGE: u128 = NEAR / 5;

// Holder claims: IDs 1–8 get 1,000 tokens, 9–10 get 5,000.
pub const GENESIS: u128 = 1_000 * TOKEN;
pub const FOUNDERS: u128 = 5_000 * TOKEN;
pub const CLAIM_RESERVE: u128 = 8 * GENESIS + 2 * FOUNDERS;
// Sale: 100 tokens at 0.1 NEAR, holder window from 100 ns, public from 200 ns, ends at 300 ns.
pub const SALE_TOTAL: u128 = 100 * TOKEN;
pub const SALE_PRICE: u128 = NEAR / 10;
pub const HOLDER_START: u64 = 100;
pub const PUBLIC_START: u64 = 200;
pub const SALE_END: u64 = 300;
pub const CREATOR_SHARE: u128 = 1_000_000 * TOKEN;

pub fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

pub fn setup(predecessor: &str, deposit: u128, now_ns: u64) {
    let context = VMContextBuilder::new()
        .current_account_id(account(LAUNCH))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .account_balance(NearToken::from_near(10))
        .block_timestamp(now_ns)
        .build();
    testing_env!(context);
}

pub fn token_config(allocations: Vec<(&str, u128)>) -> TokenConfig {
    TokenConfig {
        name: "Orbit Genesis".into(),
        symbol: "ORBIT".into(),
        decimals: 18,
        icon: Some("data:image/svg+xml;base64,PHN2Zy8+".into()),
        total_supply: U128(allocations.iter().map(|(_, amount)| amount).sum()),
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

pub fn collection_config(supply: Supply) -> CollectionConfig {
    CollectionConfig {
        name: "Genesis".into(),
        symbol: "GEN".into(),
        description: "Genesis collection".into(),
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

pub fn claim_config() -> ClaimConfig {
    ClaimConfig {
        tiers: vec![
            Tier { name: "Genesis".into(), from: 1, to: 8, amount_per_nft: U128(GENESIS) },
            Tier { name: "Founders".into(), from: 9, to: 10, amount_per_nft: U128(FOUNDERS) },
        ],
        starts_at_ns: U64(0),
    }
}

pub fn sale_config() -> SaleConfig {
    SaleConfig {
        price_per_token: U128(SALE_PRICE),
        total_for_sale: U128(SALE_TOTAL),
        per_nft_limit: U128(10 * TOKEN),
        holder_starts_at_ns: U64(HOLDER_START),
        public_starts_at_ns: Some(U64(PUBLIC_START)),
        per_account_public_limit: Some(U128(5 * TOKEN)),
        ends_at_ns: U64(SALE_END),
    }
}

pub fn deploy(
    token: Option<TokenConfig>,
    collection: Option<CollectionConfig>,
    claim: Option<ClaimConfig>,
    sale: Option<SaleConfig>,
) -> Contract {
    setup("factory.testnet", 0, 0);
    Contract::new(account(CREATOR), token, collection, claim, sale)
}

/// A 10-NFT collection with claims and a sale, funded from one reserve.
pub fn connected() -> Contract {
    let token = token_config(vec![(CREATOR, CREATOR_SHARE), (LAUNCH, CLAIM_RESERVE + SALE_TOTAL)]);
    deploy(
        Some(token),
        Some(collection_config(Supply::Fixed { max: 10 })),
        Some(claim_config()),
        Some(sale_config()),
    )
}

/// Mints `quantity` free NFTs to `who`; returns their IDs.
pub fn mint(contract: &mut Contract, who: &str, quantity: u32) -> Vec<String> {
    setup(who, STORAGE, 0);
    contract.nft_mint(quantity).into_iter().map(|t| t.token_id).collect()
}

/// yoctoNEAR transferred to `who` by the receipts this call created.
pub fn transferred_to(who: &str) -> u128 {
    get_created_receipts()
        .into_iter()
        .filter(|r| r.receiver_id == account(who))
        .map(|r| {
            format!("{:?}", r.actions)
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|n| n.parse::<u128>().ok())
                .max()
                .unwrap_or(0)
        })
        .sum()
}
