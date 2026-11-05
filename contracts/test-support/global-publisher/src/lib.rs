//! Sandbox-only helper: `near-workspaces` cannot send a `DeployGlobalContract`
//! action, so integration tests publish code through this contract.

use near_sdk::{env, near, Promise};

#[near(contract_state)]
#[derive(Default)]
pub struct Contract {}

#[near]
impl Contract {
    /// Publishes `code` as a global contract identified by its SHA-256 hash.
    pub fn publish(&mut self, #[serializer(borsh)] code: Vec<u8>) -> Promise {
        Promise::new(env::current_account_id()).deploy_global_contract(code)
    }
}
