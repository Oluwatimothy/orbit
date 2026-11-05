//! Paid public minting and owner minting.

use near_contract_standards::non_fungible_token::events::NftMint;
use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::Token;
use near_sdk::{env, near, require, AccountId, NearToken, Promise};
use orbit_common::config::TokenMedia;
use orbit_common::limits::MAX_MINT_PER_CALL;

use crate::{Contract, ContractExt};

/// Kept from every mint so the contract can pay for a new owner's token set
/// when NFTs are transferred. The standard frees a set when it empties, so the
/// number of sets never exceeds the number of tokens; this reserve covers it.
pub const TRANSFER_STORAGE_RESERVE: NearToken = NearToken::from_micronear(2_500);

#[near]
impl Contract {
    /// Mints `quantity` tokens to the caller. The attached deposit must cover
    /// `price × quantity`, the storage this call adds and the per-token
    /// transfer reserve; the excess is refunded and the price is paid to the
    /// collection owner.
    #[payable]
    pub fn nft_mint(&mut self, quantity: u32) -> Vec<Token> {
        let config = self.config();
        let now = env::block_timestamp();
        require!(!self.paused, "minting is paused");
        require!(config.public_mint, "public minting is not enabled for this collection");
        require!(now >= config.starts_at_ns.0, "minting has not started");
        if let Some(ends_at) = config.ends_at_ns {
            require!(now < ends_at.0, "minting has ended");
        }

        let receiver_id = env::predecessor_account_id();
        let already_minted = self.orbit_minted_by(receiver_id.clone());
        if let Some(limit) = config.max_per_wallet {
            require!(
                already_minted.saturating_add(quantity) <= limit,
                format!("this wallet may mint at most {limit} tokens")
            );
        }
        let price_total = config
            .price
            .0
            .checked_mul(u128::from(quantity))
            .unwrap_or_else(|| env::panic_str("price overflow"));

        // Measured across every write in this call, including the wallet counter.
        let storage_before = env::storage_usage();
        self.minted_by.insert(receiver_id.clone(), already_minted + quantity);
        let tokens = self.mint_tokens(&receiver_id, quantity);
        let storage_cost = self.storage_charge(storage_before, quantity);

        let required =
            price_total.checked_add(storage_cost).unwrap_or_else(|| env::panic_str("deposit overflow"));
        self.settle_deposit(&receiver_id, required);
        if price_total > 0 {
            Promise::new(self.tokens.owner_id.clone())
                .transfer(NearToken::from_yoctonear(price_total))
                .detach();
        }
        tokens
    }

    /// Owner-only minting to any account. Supply cap still applies; the owner
    /// pays only for storage and the transfer reserve.
    #[payable]
    pub fn creator_mint(&mut self, receiver_id: AccountId, quantity: u32) -> Vec<Token> {
        self.assert_owner();
        require!(!self.paused, "minting is paused");
        let storage_before = env::storage_usage();
        let tokens = self.mint_tokens(&receiver_id, quantity);
        let storage_cost = self.storage_charge(storage_before, quantity);
        self.settle_deposit(&env::predecessor_account_id(), storage_cost);
        tokens
    }
}

impl Contract {
    /// Mints sequential IDs to `receiver_id`.
    fn mint_tokens(&mut self, receiver_id: &AccountId, quantity: u32) -> Vec<Token> {
        require!(
            (1..=MAX_MINT_PER_CALL).contains(&quantity),
            format!("quantity must be between 1 and {MAX_MINT_PER_CALL}")
        );
        let next_total =
            self.minted.checked_add(quantity).unwrap_or_else(|| env::panic_str("supply overflow"));
        if let Some(max) = self.config().supply.max() {
            require!(next_total <= max, format!("only {} tokens remain", max - self.minted));
        }

        let mut tokens = Vec::with_capacity(quantity as usize);
        for n in self.minted + 1..=next_total {
            let metadata = self.token_metadata(n);
            tokens.push(self.tokens.internal_mint_with_refund(
                n.to_string(),
                receiver_id.clone(),
                Some(metadata),
                None,
            ));
        }
        self.minted = next_total;

        let ids: Vec<&str> = tokens.iter().map(|t| t.token_id.as_str()).collect();
        NftMint { owner_id: receiver_id, token_ids: &ids, memo: None }.emit();
        tokens
    }

    /// Storage added since `before`, plus the transfer reserve for `quantity` tokens.
    fn storage_charge(&self, before: u64, quantity: u32) -> u128 {
        let used = env::storage_usage().saturating_sub(before);
        env::storage_byte_cost().as_yoctonear() * u128::from(used)
            + TRANSFER_STORAGE_RESERVE.as_yoctonear() * u128::from(quantity)
    }

    fn token_metadata(&self, n: u32) -> TokenMetadata {
        let config = self.config();
        let (media, media_hash) = match &config.media {
            TokenMedia::Shared { media, media_hash } => (media.clone(), media_hash.clone()),
            TokenMedia::Directory { extension, .. } => (format!("{n}.{extension}"), None),
        };
        TokenMetadata {
            title: Some(format!("{} #{n}", config.name)),
            media: Some(media),
            media_hash,
            issued_at: Some(env::block_timestamp_ms().to_string()),
            ..Default::default()
        }
    }

    /// Panics (reverting the mint) if the deposit is short; refunds any excess.
    fn settle_deposit(&self, payer: &AccountId, required: u128) {
        let attached = env::attached_deposit().as_yoctonear();
        require!(
            attached >= required,
            format!("attached deposit {attached} is less than the required {required} yoctoNEAR")
        );
        let refund = attached - required;
        if refund > 0 {
            Promise::new(payer.clone()).transfer(NearToken::from_yoctonear(refund)).detach();
        }
    }
}
