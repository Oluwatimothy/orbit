//! Distributes a fixed token reserve to NFT holders. Each NFT ID can be
//! claimed once, by whoever owns it at claim time. See `claim.rs` for the
//! asynchronous claim flow.

use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::json_types::{U128, U64};
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseError};
use orbit_common::config::{ClaimConfig, Supply, Tier};
use orbit_common::events;
use orbit_common::limits::MAX_PAGE_SIZE;
use orbit_common::validate::validate_claim_config;

mod claim;

const GAS_FOR_BALANCE_VIEW: Gas = Gas::from_tgas(5);
const GAS_FOR_ACTIVATION_CALLBACK: Gas = Gas::from_tgas(5);

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    Claims,
}

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Claims are closed until the token balance covers `total_reserved`.
    AwaitingFunding,
    Active,
}

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClaimState {
    /// Ownership check or token transfer in flight. Blocks a second claim.
    Pending,
    Claimed,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimRecord {
    pub claimer: AccountId,
    pub amount: U128,
    pub state: ClaimState,
    pub updated_at_ns: U64,
}

#[near(serializers = [json])]
pub struct ClaimInfo {
    pub token_id: AccountId,
    pub nft_id: AccountId,
    pub creator_id: AccountId,
    pub tiers: Vec<Tier>,
    pub starts_at_ns: U64,
    pub status: Status,
    pub total_reserved: U128,
    pub claimed_total: U128,
    pub pending_total: U128,
    pub claims_count: u32,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.1.0",
        link = "https://github.com/orbit-launch/orbit",
        standard(standard = "nep297", version = "1.0.0"),
        standard(standard = "nep330", version = "1.2.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    token_id: AccountId,
    nft_id: AccountId,
    creator_id: AccountId,
    tiers: Vec<Tier>,
    starts_at_ns: u64,
    status: Status,
    total_reserved: u128,
    claimed_total: u128,
    pending_total: u128,
    claims_count: u32,
    claims: LookupMap<String, ClaimRecord>,
}

#[near]
impl Contract {
    /// `nft_supply` is the collection's supply so tiers can be checked against it.
    #[init]
    pub fn new(
        token_id: AccountId,
        nft_id: AccountId,
        creator_id: AccountId,
        config: ClaimConfig,
        nft_supply: Supply,
    ) -> Self {
        let total_reserved =
            validate_claim_config(&config, nft_supply).unwrap_or_else(|e| env::panic_str(&e));
        Self {
            token_id,
            nft_id,
            creator_id,
            tiers: config.tiers,
            starts_at_ns: config.starts_at_ns.0,
            status: Status::AwaitingFunding,
            total_reserved,
            claimed_total: 0,
            pending_total: 0,
            claims_count: 0,
            claims: LookupMap::new(StorageKey::Claims),
        }
    }

    /// Opens claims once this contract's token balance covers every allocation.
    /// Anyone may call it; it only ever moves `AwaitingFunding → Active`.
    pub fn activate(&mut self) -> Promise {
        require!(self.status == Status::AwaitingFunding, "claims are already active");
        ext_ft_core::ext(self.token_id.clone())
            .with_static_gas(GAS_FOR_BALANCE_VIEW)
            .ft_balance_of(env::current_account_id())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_ACTIVATION_CALLBACK)
                    .on_balance_checked(),
            )
    }

    #[private]
    pub fn on_balance_checked(&mut self, #[callback_result] balance: Result<U128, PromiseError>) -> bool {
        let Ok(balance) = balance else {
            env::log_str("token balance could not be read; claims stay closed");
            return false;
        };
        if balance.0 < self.total_reserved {
            env::log_str(&format!(
                "balance {} is below the reserved {}; claims stay closed",
                balance.0, self.total_reserved
            ));
            return false;
        }
        if self.status == Status::AwaitingFunding {
            self.status = Status::Active;
            events::emit(
                "claim_activated",
                &near_sdk::serde_json::json!({
                    "token_id": self.token_id,
                    "balance": balance,
                    "total_reserved": U128(self.total_reserved),
                }),
            );
        }
        true
    }

    pub fn get_info(&self) -> ClaimInfo {
        ClaimInfo {
            token_id: self.token_id.clone(),
            nft_id: self.nft_id.clone(),
            creator_id: self.creator_id.clone(),
            tiers: self.tiers.clone(),
            starts_at_ns: U64(self.starts_at_ns),
            status: self.status,
            total_reserved: U128(self.total_reserved),
            claimed_total: U128(self.claimed_total),
            pending_total: U128(self.pending_total),
            claims_count: self.claims_count,
        }
    }

    /// The token amount an NFT ID is entitled to, or `None` if it is in no tier.
    pub fn get_allocation(&self, nft_token_id: String) -> Option<U128> {
        self.allocation_for(&nft_token_id).map(U128)
    }

    pub fn get_claim(&self, nft_token_id: String) -> Option<ClaimRecord> {
        self.claims.get(&nft_token_id).cloned()
    }

    /// Batch lookup for a holder's NFTs, in the order given.
    pub fn get_claims(&self, nft_token_ids: Vec<String>) -> Vec<Option<ClaimRecord>> {
        require!(
            nft_token_ids.len() <= MAX_PAGE_SIZE as usize,
            format!("at most {MAX_PAGE_SIZE} IDs per call")
        );
        nft_token_ids.iter().map(|id| self.claims.get(id).cloned()).collect()
    }
}

impl Contract {
    fn allocation_for(&self, nft_token_id: &str) -> Option<u128> {
        let id: u32 = nft_token_id.parse().ok()?;
        // Reject non-canonical forms such as "007" so one NFT has exactly one claim key.
        if id.to_string() != nft_token_id {
            return None;
        }
        self.tiers.iter().find(|t| t.contains(id)).map(|t| t.amount_per_nft.0)
    }
}

#[cfg(test)]
mod tests;
