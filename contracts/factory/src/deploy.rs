//! Launch creation.
//!
//! Each asset is deployed by one batch receipt:
//! `create_account → transfer → use_global_contract → new(...)`. Actions in a
//! receipt are atomic, so if `new` panics the account is never created and
//! its balance returns to the factory. All batches are joined and followed by
//! `on_launch_deployed`, which records the outcome and refunds the creator for
//! anything that was not created.

use near_sdk::json_types::{U128, U64};
use near_sdk::serde_json::{self, json, Value};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, Promise, PromiseError};
use orbit_common::config::{ClaimConfig, CollectionConfig, SaleConfig, TokenConfig};
use orbit_common::events;
use orbit_common::limits::MAX_DESCRIPTION_LEN;
use orbit_common::validate::{
    launch_account_id, sale_cost, validate_claim_config, validate_collection_config, validate_sale_config,
    validate_slug, validate_text, validate_token_config,
};

use crate::*;

const GAS_FOR_INIT: Gas = Gas::from_tgas(30);
/// Covers the callback itself plus up to two `activate` calls.
const GAS_FOR_DEPLOY_CALLBACK: Gas = Gas::from_tgas(80);
const GAS_FOR_ACTIVATE: Gas = Gas::from_tgas(30);

/// One account to create, in the order its result appears in the callback.
struct Deployment {
    account_id: AccountId,
    balance: NearToken,
    code_hash: Base58CryptoHash,
    init_args: Value,
}

#[near]
impl Contract {
    #[payable]
    pub fn create_token_launch(&mut self, slug: String, description: String, token: TokenConfig) -> Promise {
        self.assert_open(&slug, &description);
        validate_token_config(&token).unwrap_or_else(|e| env::panic_str(&e));
        let token_account = self.launch_account(&slug, "ft");

        let launch = self.new_launch(
            LaunchKind::Token,
            slug,
            description,
            &token.name,
            &token.symbol,
            token.icon.clone(),
        );
        let deployments = vec![Deployment {
            account_id: token_account.clone(),
            balance: TOKEN_ACCOUNT_BALANCE,
            code_hash: self.code.token,
            init_args: json!({ "config": token, "creator_id": launch.creator }),
        }];
        self.register_and_deploy(Launch { token: Some(token_account), ..launch }, deployments)
    }

    #[payable]
    pub fn create_collection_launch(&mut self, slug: String, collection: CollectionConfig) -> Promise {
        self.assert_open(&slug, &collection.description);
        validate_collection_config(&collection).unwrap_or_else(|e| env::panic_str(&e));
        let nft_account = self.launch_account(&slug, "nft");

        let launch = self.new_launch(
            LaunchKind::Collection,
            slug,
            collection.description.clone(),
            &collection.name,
            &collection.symbol,
            Some(collection.cover_media.clone()),
        );
        let deployments = vec![Deployment {
            account_id: nft_account.clone(),
            balance: NFT_ACCOUNT_BALANCE,
            code_hash: self.code.nft,
            init_args: json!({ "owner_id": launch.creator, "config": collection }),
        }];
        self.register_and_deploy(Launch { nft: Some(nft_account), ..launch }, deployments)
    }

    /// A connected launch deploys the collection and token plus a holder claim
    /// contract, an NFT-gated sale contract, or both. The token must allocate
    /// to `<slug>-claim` exactly what the claim tiers reserve, and to
    /// `<slug>-sale` exactly the sale's `total_for_sale`.
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
        self.assert_open(&slug, &description);
        require!(
            claim.is_some() || sale.is_some(),
            "a connected launch needs holder claims, a gated sale, or both"
        );
        validate_token_config(&token).unwrap_or_else(|e| env::panic_str(&e));
        validate_collection_config(&collection).unwrap_or_else(|e| env::panic_str(&e));

        let token_account = self.launch_account(&slug, "ft");
        let nft_account = self.launch_account(&slug, "nft");
        let allocated_to = |account: &AccountId| -> u128 {
            token.allocations.iter().filter(|a| &a.account_id == account).map(|a| a.amount.0).sum()
        };

        let launch = self.new_launch(
            LaunchKind::Connected,
            slug.clone(),
            description,
            &token.name,
            &token.symbol,
            Some(collection.cover_media.clone()),
        );
        let mut deployments = vec![Deployment {
            account_id: nft_account.clone(),
            balance: NFT_ACCOUNT_BALANCE,
            code_hash: self.code.nft,
            init_args: json!({ "owner_id": launch.creator, "config": collection }),
        }];

        let claim_account = claim.map(|claim| {
            let reserved =
                validate_claim_config(&claim, collection.supply).unwrap_or_else(|e| env::panic_str(&e));
            let account = self.launch_account(&slug, "claim");
            require!(
                allocated_to(&account) == reserved,
                format!("the token must allocate exactly {reserved} to {account} to fund holder claims")
            );
            deployments.push(Deployment {
                account_id: account.clone(),
                balance: CLAIM_ACCOUNT_BALANCE,
                code_hash: self.code.claim,
                init_args: json!({
                    "token_id": token_account,
                    "nft_id": nft_account,
                    "creator_id": launch.creator,
                    "config": claim,
                    "nft_supply": collection.supply,
                }),
            });
            account
        });

        let sale_account = sale.map(|sale| {
            validate_sale_config(&sale, collection.supply).unwrap_or_else(|e| env::panic_str(&e));
            require!(
                sale_cost(sale.total_for_sale.0, sale.price_per_token.0, token.decimals).is_some(),
                "the full sale value does not fit in a NEAR amount"
            );
            let account = self.launch_account(&slug, "sale");
            let total = sale.total_for_sale.0;
            require!(
                allocated_to(&account) == total,
                format!("the token must allocate exactly {total} to {account} to fund the sale")
            );
            deployments.push(Deployment {
                account_id: account.clone(),
                balance: SALE_ACCOUNT_BALANCE,
                code_hash: self.code.sale,
                init_args: json!({
                    "token_id": token_account,
                    "nft_id": nft_account,
                    "creator_id": launch.creator,
                    "decimals": token.decimals,
                    "config": sale,
                    "nft_supply": collection.supply,
                }),
            });
            account
        });

        deployments.push(Deployment {
            account_id: token_account.clone(),
            balance: TOKEN_ACCOUNT_BALANCE,
            code_hash: self.code.token,
            init_args: json!({ "config": token, "creator_id": launch.creator }),
        });
        let launch = Launch {
            token: Some(token_account),
            nft: Some(nft_account),
            claim: claim_account,
            sale: sale_account,
            ..launch
        };
        self.register_and_deploy(launch, deployments)
    }

    /// Records the result of every deployment batch. For a live connected
    /// launch, asks each distribution contract to verify its own funding.
    #[private]
    pub fn on_launch_deployed(&mut self, launch_id: u64, refunds: Vec<U128>) -> bool {
        let mut launch = self.get_launch(launch_id).unwrap_or_else(|| env::panic_str("unknown launch"));
        let mut refund = 0u128;
        for (index, balance) in refunds.iter().enumerate() {
            let failed = matches!(env::promise_result_checked(index as u64, 0), Err(PromiseError::Failed));
            if failed {
                refund += balance.0;
            }
        }

        let live = refund == 0;
        if live {
            launch.status = LaunchStatus::Live;
            events::emit("launch_live", &json!({ "id": launch_id }));
            // Claims and sales open only after each contract checks its own token balance.
            for distributor in [&launch.claim, &launch.sale].into_iter().flatten() {
                Promise::new(distributor.clone())
                    .function_call("activate", Vec::new(), NearToken::from_yoctonear(0), GAS_FOR_ACTIVATE)
                    .detach();
            }
        } else {
            launch.status = LaunchStatus::Failed;
            Promise::new(launch.creator.clone()).transfer(NearToken::from_yoctonear(refund)).detach();
            events::emit("launch_failed", &json!({ "id": launch_id, "refunded": U128(refund) }));
        }
        self.launches.replace(launch_id as u32, launch);
        live
    }
}

impl Contract {
    fn assert_open(&self, slug: &str, description: &str) {
        require!(!self.paused, "new launches are paused");
        validate_slug(slug).unwrap_or_else(|e| env::panic_str(&e));
        require!(!self.slugs.contains(slug), "this slug is already taken");
        validate_text("description", description, MAX_DESCRIPTION_LEN).unwrap_or_else(|e| env::panic_str(&e));
    }

    fn launch_account(&self, slug: &str, suffix: &str) -> AccountId {
        launch_account_id(slug, suffix, &env::current_account_id()).unwrap_or_else(|e| env::panic_str(&e))
    }

    fn new_launch(
        &self,
        kind: LaunchKind,
        slug: String,
        description: String,
        name: &str,
        symbol: &str,
        image: Option<String>,
    ) -> Launch {
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
            token: None,
            nft: None,
            claim: None,
            sale: None,
        }
    }

    /// Stores the launch, charges the creator for registry storage plus the
    /// new accounts' balances, refunds the rest and schedules deployment.
    fn register_and_deploy(&mut self, launch: Launch, deployments: Vec<Deployment>) -> Promise {
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

        let balances =
            deployments.iter().fold(NearToken::from_yoctonear(0), |sum, d| sum.saturating_add(d.balance));
        let required = storage_cost.saturating_add(balances);
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
            &json!({
                "id": id,
                "kind": launch.kind,
                "slug": launch.slug,
                "creator": creator,
                "token": launch.token,
                "nft": launch.nft,
                "claim": launch.claim,
            "sale": launch.sale,
            }),
        );

        let refunds: Vec<U128> = deployments.iter().map(|d| U128(d.balance.as_yoctonear())).collect();
        let joined = deployments
            .into_iter()
            .map(deployment_promise)
            .reduce(Promise::and)
            .unwrap_or_else(|| env::panic_str("nothing to deploy"));
        joined.then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_DEPLOY_CALLBACK)
                .on_launch_deployed(id, refunds),
        )
    }
}

fn deployment_promise(deployment: Deployment) -> Promise {
    let args =
        serde_json::to_vec(&deployment.init_args).unwrap_or_else(|_| env::panic_str("invalid init args"));
    Promise::new(deployment.account_id)
        .create_account()
        .transfer(deployment.balance)
        .use_global_contract(deployment.code_hash)
        .function_call("new", args, NearToken::from_yoctonear(0), GAS_FOR_INIT)
}
