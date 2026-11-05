//! Orbit launch factory. Keeps the on-chain launch registry and deploys each
//! launch's contracts as keyless subaccounts that reference global contract
//! code by hash (NEP-591). See `deploy.rs` for the deployment flow.

use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::store::{LookupMap, LookupSet, Vector};
use near_sdk::{assert_one_yocto, env, near, require, AccountId, BorshStorageKey, NearToken, PanicOnDefault};
use orbit_common::limits::MAX_PAGE_SIZE;

mod deploy;

/// Initial balance of each launch account. It pays for that contract's own
/// state and stays locked with the keyless account.
pub const TOKEN_ACCOUNT_BALANCE: NearToken = NearToken::from_millinear(250);
pub const NFT_ACCOUNT_BALANCE: NearToken = NearToken::from_millinear(300);
pub const CLAIM_ACCOUNT_BALANCE: NearToken = NearToken::from_millinear(100);
pub const SALE_ACCOUNT_BALANCE: NearToken = NearToken::from_millinear(100);
/// Upper bound quoted for the registry entry. Actual usage is measured and
/// the difference refunded.
/// Covers the largest record: an 8 KiB data-URI icon plus a 1,000-character description.
pub const REGISTRY_STORAGE_ALLOWANCE: NearToken = NearToken::from_millinear(100);

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    Launches,
    LaunchesByCreator,
    Slugs,
}

/// Global contract code hashes used for new launches.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct CodeHashes {
    pub token: Base58CryptoHash,
    pub nft: Base58CryptoHash,
    pub claim: Base58CryptoHash,
    pub sale: Base58CryptoHash,
}

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LaunchKind {
    Token,
    Collection,
    Connected,
}

#[near(serializers = [json, borsh])]
#[serde(rename_all = "snake_case")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LaunchStatus {
    Deploying,
    Live,
    Failed,
}

/// Registry entry. Display fields are copied here so discovery needs one view
/// call; balances and ownership are always read from the asset contracts.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct Launch {
    pub id: u64,
    pub kind: LaunchKind,
    pub slug: String,
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub image: Option<String>,
    pub creator: AccountId,
    pub created_at_ns: U64,
    pub status: LaunchStatus,
    pub token: Option<AccountId>,
    pub nft: Option<AccountId>,
    pub claim: Option<AccountId>,
    pub sale: Option<AccountId>,
}

#[near(serializers = [json])]
pub struct FactoryConfig {
    pub owner_id: AccountId,
    pub code: CodeHashes,
    pub paused: bool,
    pub launch_count: u64,
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
    owner_id: AccountId,
    code: CodeHashes,
    paused: bool,
    launches: Vector<Launch>,
    launches_by_creator: LookupMap<AccountId, Vec<u64>>,
    slugs: LookupSet<String>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, code: CodeHashes) -> Self {
        Self {
            owner_id,
            code,
            paused: false,
            launches: Vector::new(StorageKey::Launches),
            launches_by_creator: LookupMap::new(StorageKey::LaunchesByCreator),
            slugs: LookupSet::new(StorageKey::Slugs),
        }
    }

    /// Owner only. Affects launches created afterwards; existing launches
    /// keep the code they were deployed with.
    #[payable]
    pub fn set_code_hashes(&mut self, code: CodeHashes) {
        assert_one_yocto();
        self.assert_owner();
        self.code = code;
    }

    /// Owner only. Stops new launches; existing launches are unaffected.
    #[payable]
    pub fn set_paused(&mut self, paused: bool) {
        assert_one_yocto();
        self.assert_owner();
        self.paused = paused;
    }

    pub fn get_config(&self) -> FactoryConfig {
        FactoryConfig {
            owner_id: self.owner_id.clone(),
            code: self.code.clone(),
            paused: self.paused,
            launch_count: u64::from(self.launches.len()),
        }
    }

    /// Deposit to attach for a launch of `kind`; the unused part is refunded.
    pub fn quote_deposit(&self, kind: LaunchKind) -> U128 {
        let balances = match kind {
            LaunchKind::Token => TOKEN_ACCOUNT_BALANCE,
            LaunchKind::Collection => NFT_ACCOUNT_BALANCE,
            LaunchKind::Connected => TOKEN_ACCOUNT_BALANCE
                .saturating_add(NFT_ACCOUNT_BALANCE)
                .saturating_add(CLAIM_ACCOUNT_BALANCE)
                .saturating_add(SALE_ACCOUNT_BALANCE),
        };
        U128(balances.saturating_add(REGISTRY_STORAGE_ALLOWANCE).as_yoctonear())
    }

    pub fn is_slug_available(&self, slug: String) -> bool {
        !self.slugs.contains(&slug)
    }

    pub fn get_launch_count(&self) -> u64 {
        u64::from(self.launches.len())
    }

    pub fn get_launch(&self, id: u64) -> Option<Launch> {
        u32::try_from(id).ok().and_then(|i| self.launches.get(i)).cloned()
    }

    /// Launches in creation order starting at `from_index`.
    pub fn get_launches(&self, from_index: Option<u64>, limit: Option<u32>) -> Vec<Launch> {
        let (start, limit) = page(from_index, limit);
        self.launches.iter().skip(start).take(limit).cloned().collect()
    }

    pub fn get_launches_by_creator(
        &self,
        account_id: AccountId,
        from_index: Option<u64>,
        limit: Option<u32>,
    ) -> Vec<Launch> {
        let (start, limit) = page(from_index, limit);
        self.launches_by_creator
            .get(&account_id)
            .map(|ids| ids.iter().skip(start).take(limit).filter_map(|id| self.get_launch(*id)).collect())
            .unwrap_or_default()
    }
}

impl Contract {
    fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.owner_id,
            "only the factory owner can call this method"
        );
    }
}

fn page(from_index: Option<u64>, limit: Option<u32>) -> (usize, usize) {
    let limit = limit.unwrap_or(MAX_PAGE_SIZE);
    require!(limit <= MAX_PAGE_SIZE, format!("limit must be at most {MAX_PAGE_SIZE}"));
    let start = usize::try_from(from_index.unwrap_or(0)).unwrap_or(usize::MAX);
    (start, limit as usize)
}

#[cfg(test)]
mod tests;
