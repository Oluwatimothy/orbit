//! Holder claims: each NFT ID in a tier can claim that tier's amount once, by
//! whoever owns it at claim time. The reserve is held by the launch account
//! and was minted to it at launch, so claims are open from `starts_at_ns`.
//!
//! A claim checks ownership, records the claim and transfers the tokens in
//! one receipt; if any step fails, none of it happens.

use near_sdk::json_types::{U128, U64};
use near_sdk::serde_json::json;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId};
use orbit_common::config::{ClaimConfig, Tier};
use orbit_common::events;
use orbit_common::limits::MAX_PAGE_SIZE;

use crate::{
    collection, part, part_mut, settle_deposit, storage_cost_since, Contract, ContractExt, StorageKey,
};

const NAME: &str = "holder claims";

#[near]
pub struct Claims {
    tiers: Vec<Tier>,
    starts_at_ns: u64,
    total_reserved: u128,
    claimed_total: u128,
    claims_count: u32,
    records: LookupMap<String, ClaimRecord>,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimRecord {
    pub claimer: AccountId,
    pub amount: U128,
    pub claimed_at_ns: U64,
}

#[near(serializers = [json])]
pub struct ClaimInfo {
    pub token_id: AccountId,
    pub nft_id: AccountId,
    pub creator_id: AccountId,
    pub tiers: Vec<Tier>,
    pub starts_at_ns: U64,
    pub total_reserved: U128,
    pub claimed_total: U128,
    pub claims_count: u32,
}

impl Claims {
    /// `total_reserved` is the sum over tiers, already validated.
    pub fn new(config: ClaimConfig, total_reserved: u128) -> Self {
        Self {
            tiers: config.tiers,
            starts_at_ns: config.starts_at_ns.0,
            total_reserved,
            claimed_total: 0,
            claims_count: 0,
            records: LookupMap::new(StorageKey::ClaimRecords),
        }
    }

    fn allocation_for(&self, nft_token_id: &str) -> Option<u128> {
        let id: u32 = nft_token_id.parse().ok()?;
        // Reject non-canonical forms such as "007" so one NFT has exactly one claim key.
        if id.to_string() != nft_token_id {
            return None;
        }
        self.tiers.iter().find(|t| t.contains(id)).map(|t| t.amount_per_nft.0)
    }
}

#[near]
impl Contract {
    /// Claims the allocation of `nft_token_id` for the caller, who must own it.
    /// The deposit must cover the storage the claim adds (token registration
    /// and the claim record, about 0.003 NEAR); the excess is refunded.
    #[payable]
    pub fn claim(&mut self, nft_token_id: String) -> U128 {
        let claimer = env::predecessor_account_id();
        let claims = part_mut(&mut self.claims, NAME);
        require!(env::block_timestamp() >= claims.starts_at_ns, "claims have not started");
        let amount = claims
            .allocation_for(&nft_token_id)
            .unwrap_or_else(|| env::panic_str("this NFT is not eligible for an allocation"));
        require!(!claims.records.contains_key(&nft_token_id), "this NFT has already been claimed");
        match part(&self.collection, collection::NAME).owner_of(&nft_token_id) {
            Some(owner) if owner == claimer => {}
            Some(_) => env::panic_str("you do not own this NFT"),
            None => env::panic_str("this NFT has not been minted"),
        }

        let storage_before = env::storage_usage();
        let claims = part_mut(&mut self.claims, NAME);
        let record = ClaimRecord {
            claimer: claimer.clone(),
            amount: U128(amount),
            claimed_at_ns: U64(env::block_timestamp()),
        };
        claims.records.insert(nft_token_id.clone(), record);
        claims.records.flush();
        claims.claimed_total += amount;
        claims.claims_count += 1;
        let surcharge = part_mut(&mut self.token, "token").pay_out(
            &claimer,
            amount,
            format!("Orbit claim for NFT #{nft_token_id}"),
        );
        settle_deposit(&claimer, storage_cost_since(storage_before) + surcharge);

        events::emit(
            "claim",
            &json!({ "nft_token_id": nft_token_id, "claimer": claimer, "amount": U128(amount) }),
        );
        U128(amount)
    }

    pub fn claim_info(&self) -> ClaimInfo {
        let claims = part(&self.claims, NAME);
        ClaimInfo {
            token_id: env::current_account_id(),
            nft_id: env::current_account_id(),
            creator_id: self.creator_id.clone(),
            tiers: claims.tiers.clone(),
            starts_at_ns: U64(claims.starts_at_ns),
            total_reserved: U128(claims.total_reserved),
            claimed_total: U128(claims.claimed_total),
            claims_count: claims.claims_count,
        }
    }

    /// The token amount an NFT ID is entitled to, or `None` if it is in no tier.
    pub fn claim_allocation(&self, nft_token_id: String) -> Option<U128> {
        part(&self.claims, NAME).allocation_for(&nft_token_id).map(U128)
    }

    pub fn claim_record(&self, nft_token_id: String) -> Option<ClaimRecord> {
        part(&self.claims, NAME).records.get(&nft_token_id).cloned()
    }

    /// Batch lookup for a holder's NFTs, in the order given.
    pub fn claim_records(&self, nft_token_ids: Vec<String>) -> Vec<Option<ClaimRecord>> {
        require!(
            nft_token_ids.len() <= MAX_PAGE_SIZE as usize,
            format!("at most {MAX_PAGE_SIZE} IDs per call")
        );
        let claims = part(&self.claims, NAME);
        nft_token_ids.iter().map(|id| claims.records.get(id).cloned()).collect()
    }
}
