//! NFT-gated token sale. During the holder window only the current owner of
//! an NFT may buy, up to `per_nft_limit` per NFT ID; an optional public window
//! lets anyone buy up to `per_account_public_limit`. Purchases are paid in
//! NEAR and the creator is paid only after the token transfer succeeds.
//! See `buy.rs` for the asynchronous purchase flow.

use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::storage_management::ext_storage_management;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, near, require, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
};
use orbit_common::config::{SaleConfig, Supply};
use orbit_common::events;
use orbit_common::validate::{sale_cost, validate_sale_config};

mod buy;

pub use buy::{FT_STORAGE_DEPOSIT, MIN_PURCHASE_DEPOSIT};

const GAS_FOR_BALANCE_VIEW: Gas = Gas::from_tgas(5);
const GAS_FOR_CALLBACK: Gas = Gas::from_tgas(10);
const GAS_FOR_ACTIVATION_CALLBACK: Gas = Gas::from_tgas(5);
const GAS_FOR_STORAGE_DEPOSIT: Gas = Gas::from_tgas(10);
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(15);

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    PurchasedByNft,
    PurchasedByAccount,
}

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    AwaitingFunding,
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

#[near(serializers = [json])]
pub struct SaleInfo {
    pub token_id: AccountId,
    pub nft_id: AccountId,
    pub creator_id: AccountId,
    pub decimals: u8,
    pub config: SaleConfig,
    pub status: Status,
    pub phase: Phase,
    pub sold: U128,
    pub pending: U128,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.1.0",
        link = "https://github.com/Oluwatimothy/orbit",
        standard(standard = "nep297", version = "1.0.0"),
        standard(standard = "nep330", version = "1.2.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    token_id: AccountId,
    nft_id: AccountId,
    creator_id: AccountId,
    decimals: u8,
    /// Highest NFT ID in the collection; IDs outside 1..=max are rejected outright.
    nft_max_supply: u32,
    config: SaleConfig,
    status: Status,
    /// Delivered to buyers.
    sold: u128,
    /// Reserved by purchases whose transfer has not resolved yet.
    pending: u128,
    /// Includes pending purchases, so limits hold while calls are in flight.
    purchased_by_nft: LookupMap<String, u128>,
    purchased_by_account: LookupMap<AccountId, u128>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(
        token_id: AccountId,
        nft_id: AccountId,
        creator_id: AccountId,
        decimals: u8,
        config: SaleConfig,
        nft_supply: Supply,
    ) -> Self {
        validate_sale_config(&config, nft_supply).unwrap_or_else(|e| env::panic_str(&e));
        let nft_max_supply =
            nft_supply.max().unwrap_or_else(|| env::panic_str("a fixed NFT supply is required"));
        require!(
            sale_cost(config.total_for_sale.0, config.price_per_token.0, decimals).is_some(),
            "the full sale value does not fit in a NEAR amount"
        );
        Self {
            token_id,
            nft_id,
            creator_id,
            decimals,
            nft_max_supply,
            config,
            status: Status::AwaitingFunding,
            sold: 0,
            pending: 0,
            purchased_by_nft: LookupMap::new(StorageKey::PurchasedByNft),
            purchased_by_account: LookupMap::new(StorageKey::PurchasedByAccount),
        }
    }

    /// Opens the sale once this contract holds `total_for_sale`. Anyone may call it.
    pub fn activate(&mut self) -> Promise {
        require!(self.status == Status::AwaitingFunding, "the sale is already active");
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
        let funded = matches!(balance, Ok(b) if b.0 >= self.config.total_for_sale.0);
        if funded && self.status == Status::AwaitingFunding {
            self.status = Status::Active;
            events::emit("sale_activated", &json!({ "total_for_sale": self.config.total_for_sale }));
        }
        funded
    }

    /// Creator only, after the sale has ended and no purchase is in flight:
    /// returns every unsold token to the creator.
    pub fn withdraw_unsold(&mut self) -> Promise {
        require!(
            env::predecessor_account_id() == self.creator_id,
            "only the creator can withdraw unsold tokens"
        );
        require!(self.status == Status::Active, "the sale is not active");
        require!(self.phase() == Phase::Ended, "the sale has not ended");
        require!(self.pending == 0, "purchases are still being settled");
        let unsold = self.config.total_for_sale.0 - self.sold;
        require!(unsold > 0, "nothing left to withdraw");
        self.status = Status::Closed;

        ext_storage_management::ext(self.token_id.clone())
            .with_static_gas(GAS_FOR_STORAGE_DEPOSIT)
            .with_attached_deposit(FT_STORAGE_DEPOSIT)
            .storage_deposit(Some(self.creator_id.clone()), Some(true))
            .then(
                ext_ft_core::ext(self.token_id.clone())
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .with_attached_deposit(NearToken::from_yoctonear(1))
                    .ft_transfer(
                        self.creator_id.clone(),
                        U128(unsold),
                        Some("Orbit sale: unsold tokens".into()),
                    ),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_CALLBACK)
                    .on_unsold_withdrawn(U128(unsold)),
            )
    }

    #[private]
    pub fn on_unsold_withdrawn(&mut self, amount: U128) -> bool {
        let ok = !matches!(env::promise_result_checked(0, 0), Err(PromiseError::Failed));
        if ok {
            events::emit("sale_closed", &json!({ "returned": amount }));
        } else {
            // Re-open so the creator can retry.
            self.status = Status::Active;
        }
        ok
    }

    pub fn get_info(&self) -> SaleInfo {
        SaleInfo {
            token_id: self.token_id.clone(),
            nft_id: self.nft_id.clone(),
            creator_id: self.creator_id.clone(),
            decimals: self.decimals,
            config: self.config.clone(),
            status: self.status,
            phase: self.phase(),
            sold: U128(self.sold),
            pending: U128(self.pending),
        }
    }

    /// Cost in yoctoNEAR of `amount` base units (excludes the storage deposit).
    pub fn quote(&self, amount: U128) -> U128 {
        U128(
            sale_cost(amount.0, self.config.price_per_token.0, self.decimals)
                .unwrap_or_else(|| env::panic_str("amount too large")),
        )
    }

    pub fn get_purchased_by_nft(&self, nft_token_id: String) -> U128 {
        U128(self.purchased_by_nft.get(&nft_token_id).copied().unwrap_or(0))
    }

    pub fn get_purchased_by_account(&self, account_id: AccountId) -> U128 {
        U128(self.purchased_by_account.get(&account_id).copied().unwrap_or(0))
    }
}

impl Contract {
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

    fn available(&self) -> u128 {
        self.config.total_for_sale.0 - self.sold - self.pending
    }
}

#[cfg(test)]
mod tests;
