//! NFT-gated token sale. During the holder window only the current owner of
//! an NFT may buy, up to `per_nft_limit` per NFT ID; an optional public window
//! lets anyone buy up to `per_account_public_limit`. Tokens come from the
//! launch account's reserve and are paid for in NEAR, which goes to the
//! creator in the same receipt.

use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId};
use orbit_common::config::SaleConfig;
use orbit_common::events;
use orbit_common::validate::sale_cost;

use crate::{
    collection, part, part_mut, settle_deposit, storage_cost_since, transfer, Contract, ContractExt,
    StorageKey,
};

const NAME: &str = "sale";

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SaleStatus {
    Active,
    /// Unsold tokens have been returned to the creator.
    Closed,
}

#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    NotStarted,
    Holders,
    Public,
    Ended,
}

#[near]
pub struct Sale {
    decimals: u8,
    config: SaleConfig,
    status: SaleStatus,
    sold: u128,
    by_nft: LookupMap<String, u128>,
    by_account: LookupMap<AccountId, u128>,
}

#[near(serializers = [json])]
pub struct SaleInfo {
    pub token_id: AccountId,
    pub nft_id: AccountId,
    pub creator_id: AccountId,
    pub decimals: u8,
    pub config: SaleConfig,
    pub status: SaleStatus,
    pub phase: Phase,
    pub sold: U128,
}

impl Sale {
    /// The config has already been validated against the token and collection.
    pub fn new(config: SaleConfig, decimals: u8) -> Self {
        Self {
            decimals,
            config,
            status: SaleStatus::Active,
            sold: 0,
            by_nft: LookupMap::new(StorageKey::SaleByNft),
            by_account: LookupMap::new(StorageKey::SaleByAccount),
        }
    }

    fn phase(&self) -> Phase {
        let now = env::block_timestamp();
        if now >= self.config.ends_at_ns.0 {
            Phase::Ended
        } else if self.config.public_starts_at_ns.is_some_and(|start| now >= start.0) {
            Phase::Public
        } else if now >= self.config.holder_starts_at_ns.0 {
            Phase::Holders
        } else {
            Phase::NotStarted
        }
    }

    fn cost(&self, amount: u128) -> u128 {
        sale_cost(amount, self.config.price_per_token.0, self.decimals)
            .unwrap_or_else(|| env::panic_str("amount too large"))
    }

    fn unsold(&self) -> u128 {
        self.config.total_for_sale.0 - self.sold
    }
}

#[near]
impl Contract {
    /// Buys `amount` base units. In the holder window `nft_token_id` is
    /// required and the caller must own it. Attach `sale_quote(amount)` plus
    /// the storage the purchase adds (about 0.003 NEAR); the excess is refunded.
    /// Returns the cost paid.
    #[payable]
    pub fn buy(&mut self, amount: U128, nft_token_id: Option<String>) -> U128 {
        let buyer = env::predecessor_account_id();
        let sale = part(&self.sale, NAME);
        require!(sale.status == SaleStatus::Active, "the sale is closed");
        require!(amount.0 > 0, "amount must be greater than zero");
        require!(amount.0 <= sale.unsold(), "not enough tokens left in the sale");
        let cost = sale.cost(amount.0);
        let phase = sale.phase();

        if phase == Phase::Holders {
            let id = nft_token_id
                .as_deref()
                .unwrap_or_else(|| env::panic_str("the holder window requires an NFT ID"));
            match part(&self.collection, collection::NAME).owner_of(id) {
                Some(owner) if owner == buyer => {}
                Some(_) => env::panic_str("you do not own this NFT"),
                None => env::panic_str("this NFT has not been minted"),
            }
        }

        let storage_before = env::storage_usage();
        let sale = part_mut(&mut self.sale, NAME);
        let slot = match phase {
            Phase::Holders => {
                let id = nft_token_id.unwrap_or_default();
                let used = sale.by_nft.get(&id).copied().unwrap_or(0);
                let limit = sale.config.per_nft_limit.0;
                require!(used + amount.0 <= limit, format!("this NFT can buy at most {} more", limit - used));
                sale.by_nft.insert(id.clone(), used + amount.0);
                sale.by_nft.flush();
                json!({ "nft": id })
            }
            Phase::Public => {
                let limit = sale.config.per_account_public_limit.map_or(0, |l| l.0);
                let used = sale.by_account.get(&buyer).copied().unwrap_or(0);
                require!(
                    used + amount.0 <= limit,
                    format!("this account can buy at most {} more", limit - used)
                );
                sale.by_account.insert(buyer.clone(), used + amount.0);
                sale.by_account.flush();
                json!({ "account": buyer })
            }
            Phase::NotStarted => env::panic_str("the sale has not started"),
            Phase::Ended => env::panic_str("the sale has ended"),
        };
        sale.sold += amount.0;
        let surcharge =
            part_mut(&mut self.token, "token").pay_out(&buyer, amount.0, "Orbit sale purchase".into());

        let required = cost
            .checked_add(storage_cost_since(storage_before) + surcharge)
            .unwrap_or_else(|| env::panic_str("deposit overflow"));
        settle_deposit(&buyer, required);
        transfer(&self.creator_id, cost);
        events::emit(
            "purchase",
            &json!({ "buyer": buyer, "amount": amount, "cost": U128(cost), "slot": slot }),
        );
        U128(cost)
    }

    /// Creator only, after the sale has ended: returns every unsold token to
    /// the creator. Registration storage, if needed, is paid by the launch account.
    pub fn withdraw_unsold(&mut self) -> U128 {
        require!(
            env::predecessor_account_id() == self.creator_id,
            "only the creator can withdraw unsold tokens"
        );
        let sale = part_mut(&mut self.sale, NAME);
        require!(sale.status == SaleStatus::Active, "unsold tokens have already been withdrawn");
        require!(sale.phase() == Phase::Ended, "the sale has not ended");
        let unsold = sale.unsold();
        require!(unsold > 0, "nothing left to withdraw");
        sale.status = SaleStatus::Closed;

        let creator = self.creator_id.clone();
        // At most one registration, paid by the launch account (bounded: once per launch).
        let _ =
            part_mut(&mut self.token, "token").pay_out(&creator, unsold, "Orbit sale: unsold tokens".into());
        events::emit("sale_closed", &json!({ "returned": U128(unsold) }));
        U128(unsold)
    }

    pub fn sale_info(&self) -> SaleInfo {
        let sale = part(&self.sale, NAME);
        SaleInfo {
            token_id: env::current_account_id(),
            nft_id: env::current_account_id(),
            creator_id: self.creator_id.clone(),
            decimals: sale.decimals,
            config: sale.config.clone(),
            status: sale.status,
            phase: sale.phase(),
            sold: U128(sale.sold),
        }
    }

    /// Cost in yoctoNEAR of `amount` base units (excludes storage).
    pub fn sale_quote(&self, amount: U128) -> U128 {
        U128(part(&self.sale, NAME).cost(amount.0))
    }

    pub fn sale_purchased_by_nft(&self, nft_token_id: String) -> U128 {
        U128(part(&self.sale, NAME).by_nft.get(&nft_token_id).copied().unwrap_or(0))
    }

    pub fn sale_purchased_by_account(&self, account_id: AccountId) -> U128 {
        U128(part(&self.sale, NAME).by_account.get(&account_id).copied().unwrap_or(0))
    }
}
