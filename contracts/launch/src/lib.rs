//! One Orbit launch in one contract. Depending on what the creator chose, the
//! launch account is a fixed-supply NEP-141 token, a NEP-171 collection with
//! on-chain mint rules, or both, with holder claims and/or an NFT-gated sale
//! that pay out of tokens held by the launch account itself.
//!
//! Because the token and the collection live in the same contract, claims
//! and purchases check NFT ownership and move tokens synchronously: each one
//! either completes in a single receipt or reverts entirely.

use near_sdk::{env, near, AccountId, BorshStorageKey, NearToken, PanicOnDefault, Promise};
use orbit_common::config::{ClaimConfig, CollectionConfig, SaleConfig, TokenConfig};
use orbit_common::validate::{validate_launch, LaunchParts};

mod claim;
mod collection;
mod nft_standards;
mod sale;
mod token;

pub use claim::{ClaimInfo, ClaimRecord};
pub use collection::{MintStats, TRANSFER_STORAGE_RESERVE};
pub use sale::{Phase, SaleInfo, SaleStatus};

use claim::Claims;
use collection::Collection;
use sale::Sale;
use token::Fungible;

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    FtAccounts,
    FtMetadata,
    FtAllocations,
    NftOwnerById,
    NftTokenMetadata,
    NftEnumeration,
    NftApproval,
    NftMetadata,
    NftConfig,
    NftMintedBy,
    ClaimRecords,
    SaleByNft,
    SaleByAccount,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.2.0",
        link = "https://github.com/Oluwatimothy/orbit",
        standard(standard = "nep141", version = "1.0.0"),
        standard(standard = "nep145", version = "1.0.0"),
        standard(standard = "nep148", version = "1.0.0"),
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
    creator_id: AccountId,
    token: Option<Fungible>,
    collection: Option<Collection>,
    claims: Option<Claims>,
    sale: Option<Sale>,
}

#[near]
impl Contract {
    /// Called by the factory in the same batch that creates and deploys the
    /// account. Token allocations to this account fund claims and the sale.
    #[init]
    pub fn new(
        creator_id: AccountId,
        token: Option<TokenConfig>,
        collection: Option<CollectionConfig>,
        claim: Option<ClaimConfig>,
        sale: Option<SaleConfig>,
    ) -> Self {
        let parts = LaunchParts {
            token: token.as_ref(),
            collection: collection.as_ref(),
            claim: claim.as_ref(),
            sale: sale.as_ref(),
        };
        let claim_reserve =
            validate_launch(parts, &env::current_account_id()).unwrap_or_else(|e| env::panic_str(&e));
        let decimals = token.as_ref().map_or(0, |t| t.decimals);

        Self {
            token: token.map(Fungible::new),
            collection: collection.map(|c| Collection::new(creator_id.clone(), c)),
            claims: claim.map(|c| Claims::new(c, claim_reserve)),
            sale: sale.map(|s| Sale::new(s, decimals)),
            creator_id,
        }
    }

    pub fn orbit_creator(&self) -> AccountId {
        self.creator_id.clone()
    }
}

/// Unwraps a launch part, panicking with a clear message when this launch
/// does not have it.
fn part<'a, T>(part: &'a Option<T>, name: &str) -> &'a T {
    part.as_ref().unwrap_or_else(|| env::panic_str(&format!("this launch has no {name}")))
}

fn part_mut<'a, T>(part: &'a mut Option<T>, name: &str) -> &'a mut T {
    part.as_mut().unwrap_or_else(|| env::panic_str(&format!("this launch has no {name}")))
}

/// Cost of the storage written since `before`.
fn storage_cost_since(before: u64) -> u128 {
    env::storage_byte_cost().as_yoctonear() * u128::from(env::storage_usage().saturating_sub(before))
}

/// Panics (reverting the call) if the deposit is below `required`; refunds the excess.
fn settle_deposit(payer: &AccountId, required: u128) {
    let attached = env::attached_deposit().as_yoctonear();
    if attached < required {
        env::panic_str(&format!(
            "attached deposit {attached} is less than the required {required} yoctoNEAR"
        ));
    }
    transfer(payer, attached - required);
}

fn transfer(account_id: &AccountId, amount: u128) {
    if amount > 0 {
        Promise::new(account_id.clone()).transfer(NearToken::from_yoctonear(amount)).detach();
    }
}

#[cfg(test)]
mod tests;
