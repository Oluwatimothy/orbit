//! Purchase flow:
//!
//! ```text
//! buy ── reserve amount on the NFT / account counter and `pending`
//!   holders window ──► nft.nft_token(id) ──► on_owner_checked
//!                         owner ≠ buyer ──► release, refund everything
//!   public window or owner = buyer ──► token.storage_deposit(buyer)
//!                                      └► token.ft_transfer(buyer, amount)
//!   on_delivered ── ok ──► sold += amount, pay creator, refund change
//!                └─ err ─► release, refund
//! ```

use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::non_fungible_token::core::ext_nft_core;
use near_contract_standards::non_fungible_token::Token;
use near_contract_standards::storage_management::ext_storage_management;
use near_sdk::json_types::U128;
use near_sdk::serde_json::{self, json};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, Promise, PromiseError, PromiseOrValue};
use orbit_common::events;
use orbit_common::validate::sale_cost;

use crate::{Contract, ContractExt, Phase, Status, GAS_FOR_FT_TRANSFER, GAS_FOR_STORAGE_DEPOSIT};

/// Attached to `storage_deposit` to register the buyer with the token.
pub const FT_STORAGE_DEPOSIT: NearToken = NearToken::from_micronear(1_500);
/// Attached on top of the purchase cost; covers token registration and the
/// storage of purchase records. Excess is refunded.
pub const MIN_PURCHASE_DEPOSIT: NearToken = NearToken::from_millinear(5);
/// Kept when a purchase is rejected, so reserving supply with NFTs one does
/// not own is not free.
pub const REJECTION_FEE: NearToken = NearToken::from_millinear(1);

const MAX_TOKEN_RESULT_LEN: usize = 16 * 1024;
const GAS_FOR_NFT_TOKEN: Gas = Gas::from_tgas(10);
const GAS_FOR_OWNER_CALLBACK: Gas = Gas::from_tgas(60);
const GAS_FOR_DELIVERY_CALLBACK: Gas = Gas::from_tgas(15);

/// What the buyer attached and what this purchase costs, carried through callbacks.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct Charges {
    pub cost: U128,
    pub deposit: NearToken,
    /// Storage the purchase records added; kept by the contract.
    pub storage_fee: NearToken,
}

/// Which limit a purchase was counted against, so it can be released.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Nft(String),
    Account(AccountId),
}

#[near]
impl Contract {
    /// Buys `amount` base units. In the holder window `nft_token_id` is required
    /// and the caller must own it. Attach `quote(amount) + MIN_PURCHASE_DEPOSIT`.
    #[payable]
    pub fn buy(&mut self, amount: U128, nft_token_id: Option<String>) -> PromiseOrValue<bool> {
        require!(self.status == Status::Active, "the sale is not active");
        require!(amount.0 > 0, "amount must be greater than zero");
        require!(amount.0 <= self.available(), "not enough tokens left in the sale");

        let buyer = env::predecessor_account_id();
        let cost = sale_cost(amount.0, self.config.price_per_token.0, self.decimals)
            .unwrap_or_else(|| env::panic_str("amount too large"));
        let deposit = env::attached_deposit();
        let required = NearToken::from_yoctonear(cost).saturating_add(MIN_PURCHASE_DEPOSIT);
        require!(deposit >= required, format!("attach at least {} yoctoNEAR", required.as_yoctonear()));

        let storage_before = env::storage_usage();
        let slot = match self.phase() {
            Phase::Holders => {
                let id =
                    nft_token_id.unwrap_or_else(|| env::panic_str("the holder window requires an NFT ID"));
                require!(self.is_valid_nft_id(&id), "not an NFT ID in this collection");
                let used = self.purchased_by_nft.get(&id).copied().unwrap_or(0);
                require!(
                    used + amount.0 <= self.config.per_nft_limit.0,
                    format!("this NFT can buy at most {} more", self.config.per_nft_limit.0 - used)
                );
                self.purchased_by_nft.insert(id.clone(), used + amount.0);
                Slot::Nft(id)
            }
            Phase::Public => {
                let limit = self.config.per_account_public_limit.map_or(0, |l| l.0);
                let used = self.purchased_by_account.get(&buyer).copied().unwrap_or(0);
                require!(
                    used + amount.0 <= limit,
                    format!("this account can buy at most {} more", limit - used)
                );
                self.purchased_by_account.insert(buyer.clone(), used + amount.0);
                Slot::Account(buyer.clone())
            }
            Phase::NotStarted => env::panic_str("the sale has not started"),
            Phase::Ended => env::panic_str("the sale has ended"),
        };
        self.pending += amount.0;
        // Purchase records are paid for by the buyer, never by the contract.
        let storage_fee = NearToken::from_yoctonear(
            env::storage_byte_cost().as_yoctonear()
                * u128::from(env::storage_usage().saturating_sub(storage_before)),
        );
        require!(
            deposit
                >= NearToken::from_yoctonear(cost)
                    .saturating_add(FT_STORAGE_DEPOSIT)
                    .saturating_add(storage_fee),
            "the deposit does not cover token registration and record storage"
        );
        let charges = Charges { cost: U128(cost), deposit, storage_fee };

        match &slot {
            Slot::Nft(id) => ext_nft_core::ext(self.nft_id.clone())
                .with_static_gas(GAS_FOR_NFT_TOKEN)
                .nft_token(id.clone())
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_OWNER_CALLBACK)
                        .on_owner_checked(buyer, amount, charges, slot),
                )
                .into(),
            Slot::Account(_) => self.deliver(buyer, amount, charges, slot).into(),
        }
    }

    #[private]
    pub fn on_owner_checked(
        &mut self,
        buyer: AccountId,
        amount: U128,
        charges: Charges,
        slot: Slot,
    ) -> PromiseOrValue<bool> {
        // Parsed by hand so a malformed result still reaches the rollback.
        let owner = env::promise_result_checked(0, MAX_TOKEN_RESULT_LEN)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Option<Token>>(&bytes).ok())
            .flatten()
            .map(|token| token.owner_id);
        if owner.as_ref() != Some(&buyer) {
            self.release(&slot, amount.0);
            refund(&buyer, charges.deposit.saturating_sub(charges.storage_fee.max(REJECTION_FEE)));
            events::emit(
                "purchase_rejected",
                &json!({ "buyer": buyer, "slot": slot, "reason": "caller does not own this NFT" }),
            );
            return PromiseOrValue::Value(false);
        }
        self.deliver(buyer, amount, charges, slot).into()
    }

    #[private]
    pub fn on_delivered(&mut self, buyer: AccountId, amount: U128, charges: Charges, slot: Slot) -> bool {
        let Charges { cost, deposit, storage_fee } = charges;
        let delivered = !matches!(env::promise_result_checked(0, 0), Err(PromiseError::Failed));
        if !delivered {
            self.release(&slot, amount.0);
            refund(&buyer, deposit.saturating_sub(FT_STORAGE_DEPOSIT).saturating_sub(storage_fee));
            events::emit("purchase_failed", &json!({ "buyer": buyer, "amount": amount }));
            return false;
        }

        self.pending -= amount.0;
        self.sold += amount.0;
        let price = NearToken::from_yoctonear(cost.0);
        if !price.is_zero() {
            Promise::new(self.creator_id.clone()).transfer(price).detach();
        }
        refund(
            &buyer,
            deposit.saturating_sub(price).saturating_sub(FT_STORAGE_DEPOSIT).saturating_sub(storage_fee),
        );
        events::emit("purchase", &json!({ "buyer": buyer, "amount": amount, "cost": cost, "slot": slot }));
        true
    }
}

impl Contract {
    fn deliver(&mut self, buyer: AccountId, amount: U128, charges: Charges, slot: Slot) -> Promise {
        ext_storage_management::ext(self.token_id.clone())
            .with_static_gas(GAS_FOR_STORAGE_DEPOSIT)
            .with_attached_deposit(FT_STORAGE_DEPOSIT)
            .storage_deposit(Some(buyer.clone()), Some(true))
            .then(
                ext_ft_core::ext(self.token_id.clone())
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .with_attached_deposit(NearToken::from_yoctonear(1))
                    .ft_transfer(buyer.clone(), amount, Some("Orbit sale purchase".into())),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_DELIVERY_CALLBACK)
                    .on_delivered(buyer, amount, charges, slot),
            )
    }

    /// Undoes the reservation made in `buy`, deleting records that drop to zero.
    fn release(&mut self, slot: &Slot, amount: u128) {
        self.pending -= amount;
        match slot {
            Slot::Nft(id) => {
                let left = self.purchased_by_nft.get(id).copied().unwrap_or(0).saturating_sub(amount);
                if left == 0 {
                    self.purchased_by_nft.remove(id);
                } else {
                    self.purchased_by_nft.insert(id.clone(), left);
                }
            }
            Slot::Account(account) => {
                let left =
                    self.purchased_by_account.get(account).copied().unwrap_or(0).saturating_sub(amount);
                if left == 0 {
                    self.purchased_by_account.remove(account);
                } else {
                    self.purchased_by_account.insert(account.clone(), left);
                }
            }
        }
    }

    /// Canonical decimal IDs from 1 to the collection's maximum supply.
    fn is_valid_nft_id(&self, id: &str) -> bool {
        id.parse::<u32>().is_ok_and(|n| n >= 1 && n <= self.nft_max_supply && n.to_string() == id)
    }
}

fn refund(account_id: &AccountId, amount: NearToken) {
    if !amount.is_zero() {
        Promise::new(account_id.clone()).transfer(amount).detach();
    }
}
