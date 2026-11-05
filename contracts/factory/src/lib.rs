//! Orbit launch factory. Keeps the on-chain launch registry, stores the
//! launch contract template and deploys it to one keyless subaccount per
//! launch. See `deploy.rs` for the deployment flow.

use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::store::{LookupMap, LookupSet, Vector};
use near_sdk::{
    assert_one_yocto, env, near, require, AccountId, BorshStorageKey, CryptoHash, NearToken, PanicOnDefault,
};
use orbit_common::limits::MAX_PAGE_SIZE;

mod deploy;

/// Raw storage key of the launch contract template. Kept outside the
/// contract state so ordinary calls do not load it.
const TEMPLATE_KEY: &[u8] = b"template";
/// Wasm module header: magic number and version 1.
const WASM_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

/// Balance for the launch account's own state (metadata, allocations,
/// config) on top of its code. It stays locked with the keyless account.
pub const LAUNCH_STATE_ALLOWANCE: NearToken = NearToken::from_millinear(200);
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

/// The launch contract code new launches receive.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateInfo {
    /// SHA-256 of the code, base58, as explorers show a contract's code hash.
    pub hash: Base58CryptoHash,
    pub size: u64,
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
/// call; balances and ownership are always read from the launch contract.
/// `token`, `nft`, `claim` and `sale` name the launch account for each part
/// the launch has.
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
    pub account: AccountId,
    pub token: Option<AccountId>,
    pub nft: Option<AccountId>,
    pub claim: Option<AccountId>,
    pub sale: Option<AccountId>,
}

#[near(serializers = [json])]
pub struct FactoryConfig {
    pub owner_id: AccountId,
    pub template: Option<TemplateInfo>,
    pub paused: bool,
    pub launch_count: u64,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.2.0",
        link = "https://github.com/Oluwatimothy/orbit",
        standard(standard = "nep297", version = "1.0.0"),
        standard(standard = "nep330", version = "1.2.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    template: Option<TemplateInfo>,
    paused: bool,
    launches: Vector<Launch>,
    launches_by_creator: LookupMap<AccountId, Vec<u64>>,
    slugs: LookupSet<String>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            template: None,
            paused: false,
            launches: Vector::new(StorageKey::Launches),
            launches_by_creator: LookupMap::new(StorageKey::LaunchesByCreator),
            slugs: LookupSet::new(StorageKey::Slugs),
        }
    }

    /// Owner only. The call's raw input is the launch contract wasm. Affects
    /// launches created afterwards; existing launches keep the code they were
    /// deployed with. Storage is paid from the factory's balance.
    #[payable]
    pub fn set_template(&mut self) -> TemplateInfo {
        assert_one_yocto();
        self.assert_owner();
        let code = env::input().unwrap_or_default();
        require!(code.starts_with(&WASM_HEADER), "the input must be a wasm module");
        env::storage_write(TEMPLATE_KEY, &code);
        let hash: CryptoHash = env::sha256_array(&code);
        let info = TemplateInfo { hash: hash.into(), size: code.len() as u64 };
        self.template = Some(info.clone());
        orbit_common::events::emit("template_set", &info);
        info
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
            template: self.template.clone(),
            paused: self.paused,
            launch_count: u64::from(self.launches.len()),
        }
    }

    /// Deposit to attach for a launch; the unused part is refunded. The same
    /// for every kind: one account holding the template code and its state.
    pub fn quote_deposit(&self, kind: LaunchKind) -> U128 {
        let _ = kind;
        U128(self.launch_balance().saturating_add(REGISTRY_STORAGE_ALLOWANCE).as_yoctonear())
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

    fn template(&self) -> &TemplateInfo {
        self.template.as_ref().unwrap_or_else(|| env::panic_str("no launch template has been set"))
    }

    /// Initial balance of a launch account: storage for its code plus its state.
    fn launch_balance(&self) -> NearToken {
        env::storage_byte_cost()
            .saturating_mul(u128::from(self.template().size))
            .saturating_add(LAUNCH_STATE_ALLOWANCE)
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
