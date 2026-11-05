//! NEP-171 collection whose price, supply cap and mint window are fixed at
//! launch and enforced on every mint. Standard methods are in `nft_standards.rs`.

use near_contract_standards::non_fungible_token::events::NftMint;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token};
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{assert_one_yocto, env, near, require, AccountId, NearToken};
use orbit_common::config::{CollectionConfig, TokenMedia};
use orbit_common::limits::MAX_MINT_PER_CALL;

use crate::{
    part, part_mut, settle_deposit, storage_cost_since, transfer, Contract, ContractExt, StorageKey,
};

pub(crate) const NAME: &str = "collection";

/// Kept from every mint so the contract can pay for a new owner's token set
/// when NFTs are transferred. The standard frees a set when it empties, so the
/// number of sets never exceeds the number of tokens; this reserve covers it.
pub const TRANSFER_STORAGE_RESERVE: NearToken = NearToken::from_micronear(2_500);

#[near]
pub struct Collection {
    pub tokens: NonFungibleToken,
    pub metadata: LazyOption<NFTContractMetadata>,
    config: LazyOption<CollectionConfig>,
    minted: u32,
    minted_by: LookupMap<AccountId, u32>,
    paused: bool,
}

/// Live mint state for collection pages.
#[near(serializers = [json])]
pub struct MintStats {
    pub minted: u32,
    pub max_supply: Option<u32>,
    pub paused: bool,
    pub owner_id: AccountId,
}

impl Collection {
    pub fn new(owner_id: AccountId, config: CollectionConfig) -> Self {
        let base_uri = match &config.media {
            TokenMedia::Directory { base_uri, .. } => Some(base_uri.clone()),
            TokenMedia::Shared { .. } => None,
        };
        let metadata = NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: config.name.clone(),
            symbol: config.symbol.clone(),
            icon: None,
            base_uri,
            reference: None,
            reference_hash: None,
        };
        metadata.assert_valid();

        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NftOwnerById,
                owner_id,
                Some(StorageKey::NftTokenMetadata),
                Some(StorageKey::NftEnumeration),
                Some(StorageKey::NftApproval),
            ),
            metadata: LazyOption::new(StorageKey::NftMetadata, Some(metadata)),
            config: LazyOption::new(StorageKey::NftConfig, Some(config)),
            minted: 0,
            minted_by: LookupMap::new(StorageKey::NftMintedBy),
            paused: false,
        }
    }

    pub fn config(&self) -> &CollectionConfig {
        self.config.get().as_ref().unwrap_or_else(|| env::panic_str("config missing"))
    }

    /// The current owner of `token_id`, if it has been minted.
    pub fn owner_of(&self, token_id: &str) -> Option<AccountId> {
        self.tokens.owner_by_id.get(&token_id.to_string())
    }

    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.tokens.owner_id,
            "only the collection owner can call this method"
        );
    }

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
}

/// Storage written since `before`, plus the transfer reserve for `quantity` tokens.
fn mint_storage_charge(before: u64, quantity: u32) -> u128 {
    storage_cost_since(before) + TRANSFER_STORAGE_RESERVE.as_yoctonear() * u128::from(quantity)
}

#[near]
impl Contract {
    /// Mints `quantity` tokens to the caller. The attached deposit must cover
    /// `price × quantity`, the storage this call adds and the per-token
    /// transfer reserve; the excess is refunded and the price is paid to the
    /// collection owner.
    #[payable]
    pub fn nft_mint(&mut self, quantity: u32) -> Vec<Token> {
        let collection = part_mut(&mut self.collection, NAME);
        let config = collection.config();
        let now = env::block_timestamp();
        require!(!collection.paused, "minting is paused");
        require!(config.public_mint, "public minting is not enabled for this collection");
        require!(now >= config.starts_at_ns.0, "minting has not started");
        if let Some(ends_at) = config.ends_at_ns {
            require!(now < ends_at.0, "minting has ended");
        }

        let receiver_id = env::predecessor_account_id();
        let already_minted = collection.minted_by.get(&receiver_id).copied().unwrap_or(0);
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
        collection.minted_by.insert(receiver_id.clone(), already_minted + quantity);
        collection.minted_by.flush();
        let tokens = collection.mint_tokens(&receiver_id, quantity);
        let storage = mint_storage_charge(storage_before, quantity);

        let required = price_total.checked_add(storage).unwrap_or_else(|| env::panic_str("deposit overflow"));
        settle_deposit(&receiver_id, required);
        transfer(&collection.tokens.owner_id, price_total);
        tokens
    }

    /// Owner-only minting to any account. Supply cap still applies; the owner
    /// pays only for storage and the transfer reserve.
    #[payable]
    pub fn creator_mint(&mut self, receiver_id: AccountId, quantity: u32) -> Vec<Token> {
        let collection = part_mut(&mut self.collection, NAME);
        collection.assert_owner();
        require!(!collection.paused, "minting is paused");
        let storage_before = env::storage_usage();
        let tokens = collection.mint_tokens(&receiver_id, quantity);
        settle_deposit(&env::predecessor_account_id(), mint_storage_charge(storage_before, quantity));
        tokens
    }

    /// Owner-only switch for public and creator minting. Price, supply and
    /// timing cannot be changed.
    #[payable]
    pub fn set_paused(&mut self, paused: bool) {
        assert_one_yocto();
        let collection = part_mut(&mut self.collection, NAME);
        collection.assert_owner();
        collection.paused = paused;
    }

    pub fn orbit_config(&self) -> CollectionConfig {
        part(&self.collection, NAME).config().clone()
    }

    pub fn orbit_mint_stats(&self) -> MintStats {
        let collection = part(&self.collection, NAME);
        MintStats {
            minted: collection.minted,
            max_supply: collection.config().supply.max(),
            paused: collection.paused,
            owner_id: collection.tokens.owner_id.clone(),
        }
    }

    pub fn orbit_minted_by(&self, account_id: AccountId) -> u32 {
        part(&self.collection, NAME).minted_by.get(&account_id).copied().unwrap_or(0)
    }
}
