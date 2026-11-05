//! NEP-171 collection whose price, supply cap and mint window are fixed at
//! initialisation and enforced on every mint.

use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, NFT_METADATA_SPEC};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{assert_one_yocto, env, near, require, AccountId, BorshStorageKey, PanicOnDefault};
use orbit_common::config::{CollectionConfig, TokenMedia};
use orbit_common::validate::validate_collection_config;

pub mod mint;
mod standards;

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    OwnerById,
    TokenMetadata,
    Enumeration,
    Approval,
    Metadata,
    Config,
    MintedBy,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.1.0",
        link = "https://github.com/Oluwatimothy/orbit",
        standard(standard = "nep171", version = "1.2.0"),
        standard(standard = "nep177", version = "2.0.0"),
        standard(standard = "nep178", version = "1.0.0"),
        standard(standard = "nep181", version = "1.0.0"),
        standard(standard = "nep297", version = "1.0.0"),
        standard(standard = "nep330", version = "1.2.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
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

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, config: CollectionConfig) -> Self {
        validate_collection_config(&config).unwrap_or_else(|e| env::panic_str(&e));
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
                StorageKey::OwnerById,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(metadata)),
            config: LazyOption::new(StorageKey::Config, Some(config)),
            minted: 0,
            minted_by: LookupMap::new(StorageKey::MintedBy),
            paused: false,
        }
    }

    /// Owner-only switch for public and creator minting. Price, supply and
    /// timing cannot be changed.
    #[payable]
    pub fn set_paused(&mut self, paused: bool) {
        assert_one_yocto();
        self.assert_owner();
        self.paused = paused;
    }

    pub fn orbit_config(&self) -> CollectionConfig {
        self.config().clone()
    }

    pub fn orbit_mint_stats(&self) -> MintStats {
        MintStats {
            minted: self.minted,
            max_supply: self.config().supply.max(),
            paused: self.paused,
            owner_id: self.tokens.owner_id.clone(),
        }
    }

    pub fn orbit_minted_by(&self, account_id: AccountId) -> u32 {
        self.minted_by.get(&account_id).copied().unwrap_or(0)
    }
}

impl Contract {
    fn config(&self) -> &CollectionConfig {
        self.config.get().as_ref().unwrap_or_else(|| env::panic_str("config missing"))
    }

    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.tokens.owner_id,
            "only the collection owner can call this method"
        );
    }
}

#[cfg(test)]
mod tests;
