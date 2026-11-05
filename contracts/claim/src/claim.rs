//! Claim flow:
//!
//! ```text
//! claim(id) ── record Pending ──► nft.nft_token(id)
//!     on_owner_checked ── owner ≠ caller ──► delete record, refund
//!                      └─ owner = caller ──► token.storage_deposit(caller)
//!                                            └► token.ft_transfer(caller, amount)
//!     on_transferred ── ok ──► record Claimed
//!                    └─ err ─► delete record, refund
//! ```
//!
//! A record exists from the first call until a final outcome, so a second
//! claim of the same NFT is rejected even while the first is in flight.

use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::non_fungible_token::core::ext_nft_core;
use near_contract_standards::non_fungible_token::Token;
use near_contract_standards::storage_management::ext_storage_management;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde_json::{self, json};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, Promise, PromiseError, PromiseOrValue};
use orbit_common::events;

use crate::{ClaimRecord, ClaimState, Contract, ContractExt, Status};

/// Covers the claimer's token storage registration and the claim record.
pub const CLAIM_DEPOSIT: NearToken = NearToken::from_millinear(5);
/// Kept when a claim is rejected, so blocking someone else's claim is not free.
pub const REJECTION_FEE: NearToken = NearToken::from_millinear(1);
/// Attached to `storage_deposit`; anything above the token's minimum comes
/// back to this contract.
pub const FT_STORAGE_DEPOSIT: NearToken = NearToken::from_micronear(1_500);

/// Upper bound on the `nft_token` result read in the owner callback.
const MAX_TOKEN_RESULT_LEN: usize = 16 * 1024;

const GAS_FOR_NFT_TOKEN: Gas = Gas::from_tgas(10);
const GAS_FOR_OWNER_CALLBACK: Gas = Gas::from_tgas(60);
const GAS_FOR_STORAGE_DEPOSIT: Gas = Gas::from_tgas(10);
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(15);
const GAS_FOR_TRANSFER_CALLBACK: Gas = Gas::from_tgas(15);

#[near]
impl Contract {
    /// Claims the allocation of `nft_token_id` for the caller, who must own it.
    /// Attach at least `CLAIM_DEPOSIT`; anything above it is refunded.
    #[payable]
    pub fn claim(&mut self, nft_token_id: String) -> Promise {
        require!(self.status == Status::Active, "claims are not active");
        require!(env::block_timestamp() >= self.starts_at_ns, "claims have not started");
        let deposit = env::attached_deposit();
        require!(
            deposit >= CLAIM_DEPOSIT,
            format!("attach at least {} yoctoNEAR", CLAIM_DEPOSIT.as_yoctonear())
        );
        let amount = self
            .allocation_for(&nft_token_id)
            .unwrap_or_else(|| env::panic_str("this NFT is not eligible for an allocation"));
        if let Some(existing) = self.claims.get(&nft_token_id) {
            env::panic_str(match existing.state {
                ClaimState::Claimed => "this NFT has already been claimed",
                ClaimState::Pending => "a claim for this NFT is already in progress",
            });
        }

        let claimer = env::predecessor_account_id();
        self.claims.insert(nft_token_id.clone(), self.record(&claimer, amount, ClaimState::Pending));
        self.pending_total += amount;

        ext_nft_core::ext(self.nft_id.clone())
            .with_static_gas(GAS_FOR_NFT_TOKEN)
            .nft_token(nft_token_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_OWNER_CALLBACK)
                    .on_owner_checked(nft_token_id, claimer, U128(amount), deposit),
            )
    }

    #[private]
    pub fn on_owner_checked(
        &mut self,
        nft_token_id: String,
        claimer: AccountId,
        amount: U128,
        deposit: NearToken,
    ) -> PromiseOrValue<bool> {
        // Parsed by hand: a malformed result must still reach the rollback below.
        let owner = env::promise_result_checked(0, MAX_TOKEN_RESULT_LEN)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Option<Token>>(&bytes).ok())
            .flatten()
            .map(|token| token.owner_id);
        if owner.as_ref() != Some(&claimer) {
            self.release(&nft_token_id, amount.0);
            refund(&claimer, deposit.saturating_sub(REJECTION_FEE));
            events::emit(
                "claim_rejected",
                &json!({
                    "nft_token_id": nft_token_id,
                    "claimer": claimer,
                    "reason": if owner.is_some() { "caller does not own this NFT" } else { "NFT not found" },
                }),
            );
            return PromiseOrValue::Value(false);
        }

        ext_storage_management::ext(self.token_id.clone())
            .with_static_gas(GAS_FOR_STORAGE_DEPOSIT)
            .with_attached_deposit(FT_STORAGE_DEPOSIT)
            .storage_deposit(Some(claimer.clone()), Some(true))
            .then(
                ext_ft_core::ext(self.token_id.clone())
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .with_attached_deposit(NearToken::from_yoctonear(1))
                    .ft_transfer(
                        claimer.clone(),
                        amount,
                        Some(format!("Orbit claim for NFT #{nft_token_id}")),
                    ),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_TRANSFER_CALLBACK)
                    .on_transferred(nft_token_id, claimer, amount, deposit),
            )
            .into()
    }

    #[private]
    pub fn on_transferred(
        &mut self,
        nft_token_id: String,
        claimer: AccountId,
        amount: U128,
        deposit: NearToken,
    ) -> bool {
        // `ft_transfer` returns nothing; only a failed receipt counts as failure.
        let transferred = !matches!(env::promise_result_checked(0, 0), Err(PromiseError::Failed));
        if !transferred {
            self.release(&nft_token_id, amount.0);
            // The storage registration may have succeeded and kept its deposit.
            refund(&claimer, deposit.saturating_sub(FT_STORAGE_DEPOSIT));
            events::emit(
                "claim_failed",
                &json!({
                    "nft_token_id": nft_token_id,
                    "claimer": claimer,
                    "amount": amount,
                }),
            );
            return false;
        }

        self.claims.insert(nft_token_id.clone(), self.record(&claimer, amount.0, ClaimState::Claimed));
        self.pending_total -= amount.0;
        self.claimed_total += amount.0;
        self.claims_count += 1;
        refund(&claimer, deposit.saturating_sub(CLAIM_DEPOSIT));
        events::emit(
            "claim",
            &json!({
                "nft_token_id": nft_token_id,
                "claimer": claimer,
                "amount": amount,
            }),
        );
        true
    }
}

impl Contract {
    fn record(&self, claimer: &AccountId, amount: u128, state: ClaimState) -> ClaimRecord {
        ClaimRecord {
            claimer: claimer.clone(),
            amount: U128(amount),
            state,
            updated_at_ns: U64(env::block_timestamp()),
        }
    }

    /// Removes a pending claim so the NFT can be claimed again.
    fn release(&mut self, nft_token_id: &String, amount: u128) {
        self.claims.remove(nft_token_id);
        self.pending_total -= amount;
    }
}

fn refund(account_id: &AccountId, amount: NearToken) {
    if !amount.is_zero() {
        Promise::new(account_id.clone()).transfer(amount).detach();
    }
}
