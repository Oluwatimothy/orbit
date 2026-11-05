//! Launch creation.
//!
//! Each launch is deployed by one batch receipt:
//! `create_account → transfer → deploy_contract(template) → new(...)`. Actions
//! in a receipt are atomic, so if `new` panics the account is never created
//! and its balance returns to the factory. `on_launch_deployed` then records
//! the outcome and refunds the creator if the launch failed.

use near_sdk::json_types::U128;
use near_sdk::serde_json::{self, json};
use near_sdk::{env, near, AccountId, Gas, NearToken, Promise, PromiseError};
use orbit_common::config::{ClaimConfig, CollectionConfig, SaleConfig, TokenConfig};
use orbit_common::events;
use orbit_common::limits::MAX_DESCRIPTION_LEN;
use orbit_common::validate::{launch_account_id, validate_launch, validate_slug, validate_text, LaunchParts};

use crate::*;

const GAS_FOR_INIT: Gas = Gas::from_tgas(50);
const GAS_FOR_DEPLOY_CALLBACK: Gas = Gas::from_tgas(15);

#[near]
impl Contract {
    #[payable]
    pub fn create_token_launch(&mut self, slug: String, description: String, token: TokenConfig) -> Promise {
        let parts = LaunchParts { token: Some(&token), collection: None, claim: None, sale: None };
        let launch = self.prepare(
            LaunchKind::Token,
            slug,
            description,
            parts,
            (&token.name, &token.symbol),
            token.icon.clone(),
        );
        self.register_and_deploy(launch, json!({ "token": token }))
    }

    #[payable]
    pub fn create_collection_launch(&mut self, slug: String, collection: CollectionConfig) -> Promise {
        let parts = LaunchParts { token: None, collection: Some(&collection), claim: None, sale: None };
        let launch = self.prepare(
            LaunchKind::Collection,
            slug,
            collection.description.clone(),
            parts,
            (&collection.name, &collection.symbol),
            Some(collection.cover_media.clone()),
        );
        self.register_and_deploy(launch, json!({ "collection": collection }))
    }

    /// A connected launch has a collection and a token plus holder claims, an
    /// NFT-gated sale, or both. The token must allocate to the launch account
    /// (`<slug>.<factory>`) exactly the claim tiers' reserve plus the sale's
    /// `total_for_sale`.
    #[payable]
    pub fn create_connected_launch(
        &mut self,
        slug: String,
        description: String,
        token: TokenConfig,
        collection: CollectionConfig,
        claim: Option<ClaimConfig>,
        sale: Option<SaleConfig>,
    ) -> Promise {
        let parts = LaunchParts {
            token: Some(&token),
            collection: Some(&collection),
            claim: claim.as_ref(),
            sale: sale.as_ref(),
        };
        let launch = self.prepare(
            LaunchKind::Connected,
            slug,
            description,
            parts,
            (&token.name, &token.symbol),
            Some(collection.cover_media.clone()),
        );
        self.register_and_deploy(
            launch,
            json!({ "token": token, "collection": collection, "claim": claim, "sale": sale }),
        )
    }

    /// Records the result of the deployment batch, refunding the account
    /// balance to the creator if it failed.
    #[private]
    pub fn on_launch_deployed(&mut self, launch_id: u64, balance: U128) -> bool {
        let mut launch = self.get_launch(launch_id).unwrap_or_else(|| env::panic_str("unknown launch"));
        let live = !matches!(env::promise_result_checked(0, 0), Err(PromiseError::Failed));
        if live {
            launch.status = LaunchStatus::Live;
            events::emit("launch_live", &json!({ "id": launch_id }));
        } else {
            launch.status = LaunchStatus::Failed;
            Promise::new(launch.creator.clone()).transfer(NearToken::from_yoctonear(balance.0)).detach();
            events::emit("launch_failed", &json!({ "id": launch_id, "refunded": balance }));
        }
        self.launches.replace(launch_id as u32, launch);
        live
    }
}

impl Contract {
    /// Checks the slug and every part, and builds the registry entry.
    fn prepare(
        &self,
        kind: LaunchKind,
        slug: String,
        description: String,
        parts: LaunchParts,
        (name, symbol): (&str, &str),
        image: Option<String>,
    ) -> Launch {
        require!(!self.paused, "new launches are paused");
        validate_slug(&slug).unwrap_or_else(|e| env::panic_str(&e));
        require!(!self.slugs.contains(&slug), "this slug is already taken");
        validate_text("description", &description, MAX_DESCRIPTION_LEN)
            .unwrap_or_else(|e| env::panic_str(&e));
        let account =
            launch_account_id(&slug, &env::current_account_id()).unwrap_or_else(|e| env::panic_str(&e));
        validate_launch(parts, &account).unwrap_or_else(|e| env::panic_str(&e));

        let has = |present: bool| present.then(|| account.clone());
        Launch {
            id: u64::from(self.launches.len()),
            kind,
            slug,
            name: name.to_string(),
            symbol: symbol.to_string(),
            description,
            image,
            creator: env::predecessor_account_id(),
            created_at_ns: U64(env::block_timestamp()),
            status: LaunchStatus::Deploying,
            token: has(parts.token.is_some()),
            nft: has(parts.collection.is_some()),
            claim: has(parts.claim.is_some()),
            sale: has(parts.sale.is_some()),
            account,
        }
    }

    /// Stores the launch, charges the creator for registry storage plus the
    /// new account's balance, refunds the rest and schedules deployment.
    fn register_and_deploy(&mut self, launch: Launch, mut init_args: serde_json::Value) -> Promise {
        let code = env::storage_read(TEMPLATE_KEY)
            .unwrap_or_else(|| env::panic_str("no launch template has been set"));
        let balance = self.launch_balance();

        let storage_before = env::storage_usage();
        let id = launch.id;
        let creator = launch.creator.clone();
        self.slugs.insert(launch.slug.clone());
        self.launches_by_creator.entry(creator.clone()).or_default().push(id);
        self.launches.push(launch.clone());
        self.launches.flush();
        self.launches_by_creator.flush();
        let storage_cost = env::storage_byte_cost()
            .saturating_mul(u128::from(env::storage_usage().saturating_sub(storage_before)));

        let required = storage_cost.saturating_add(balance);
        let attached = env::attached_deposit();
        require!(
            attached >= required,
            format!("attach at least {} yoctoNEAR for this launch", required.as_yoctonear())
        );
        let excess = attached.saturating_sub(required);
        if !excess.is_zero() {
            Promise::new(creator.clone()).transfer(excess).detach();
        }

        events::emit(
            "launch_created",
            &json!({ "id": id, "kind": launch.kind, "slug": launch.slug, "creator": creator, "account": launch.account }),
        );

        init_args["creator_id"] = json!(creator);
        let args = serde_json::to_vec(&init_args).unwrap_or_else(|_| env::panic_str("invalid init args"));
        deployment(launch.account, balance, code, args).then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_DEPLOY_CALLBACK)
                .on_launch_deployed(id, U128(balance.as_yoctonear())),
        )
    }
}

fn deployment(account: AccountId, balance: NearToken, code: Vec<u8>, args: Vec<u8>) -> Promise {
    Promise::new(account).create_account().transfer(balance).deploy_contract(code).function_call(
        "new",
        args,
        NearToken::from_yoctonear(0),
        GAS_FOR_INIT,
    )
}
