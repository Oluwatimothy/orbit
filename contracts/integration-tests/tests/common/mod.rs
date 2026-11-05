//! Sandbox setup shared by the integration tests.
//!
//! Contracts must be built first: `./scripts/build-contracts.sh`.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use near_workspaces::network::Sandbox;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::{Gas, NearToken};
use near_workspaces::{Account, Contract, Worker};
use serde_json::{json, Value};

pub const MAX_GAS: Gas = Gas::from_tgas(300);

// Each test binary compiles this module separately and uses a subset of it.
#[allow(dead_code)]
pub struct Env {
    pub worker: Worker<Sandbox>,
    pub factory: Contract,
    pub owner: Account,
    pub creator: Account,
    pub alice: Account,
    pub bob: Account,
}

pub fn wasm(name: &str) -> Result<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../target/near/{name}/{name}.wasm"));
    std::fs::read(&path)
        .with_context(|| format!("{} missing — run ./scripts/build-contracts.sh", path.display()))
}

/// Uploads `code` as the factory's launch template (raw call input).
pub async fn set_template(
    factory: &Contract,
    owner: &Account,
    code: Vec<u8>,
) -> Result<ExecutionFinalResult> {
    Ok(owner
        .call(factory.id(), "set_template")
        .args(code)
        .deposit(NearToken::from_yoctonear(1))
        .gas(MAX_GAS)
        .transact()
        .await?)
}

pub async fn setup() -> Result<Env> {
    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    let owner = sub(&root, "owner", 10).await?;
    let factory_account = sub(&root, "factory", 20).await?;
    let factory = factory_account.deploy(&wasm("orbit_factory")?).await?.into_result()?;
    let init = factory.call("new").args_json(json!({ "owner_id": owner.id() })).transact().await?;
    expect_success(&init)?;
    expect_success(&set_template(&factory, &owner, wasm("orbit_launch")?).await?)?;

    Ok(Env {
        creator: sub(&root, "creator", 50).await?,
        alice: sub(&root, "alice", 50).await?,
        bob: sub(&root, "bob", 50).await?,
        worker,
        factory,
        owner,
    })
}

async fn sub(root: &Account, name: &str, near: u128) -> Result<Account> {
    Ok(root
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(near))
        .transact()
        .await?
        .into_result()?)
}

/// Fails if the transaction or **any** receipt it produced failed.
pub fn expect_success(result: &ExecutionFinalResult) -> Result<()> {
    if let Some(failure) = result.receipt_failures().first() {
        bail!("receipt failed: {failure:?}");
    }
    if result.is_failure() {
        bail!("transaction failed: {:?}", result.clone().into_result().err());
    }
    Ok(())
}

/// Returns the first failure message, for asserting on expected rejections.
pub fn failure_text(result: &ExecutionFinalResult) -> String {
    result.receipt_failures().iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join("\n")
}

pub async fn view(worker: &Worker<Sandbox>, contract: &str, method: &str, args: Value) -> Result<Value> {
    let id = contract.parse()?;
    Ok(worker.view(&id, method).args_json(args).await?.json()?)
}

pub async fn ft_balance(worker: &Worker<Sandbox>, token: &str, account: &str) -> Result<u128> {
    let value = view(worker, token, "ft_balance_of", json!({ "account_id": account })).await?;
    Ok(value.as_str().context("balance is a string")?.parse()?)
}

pub fn quote(value: &Value) -> Result<NearToken> {
    Ok(NearToken::from_yoctonear(value.as_str().context("quote is a string")?.parse()?))
}

pub fn launch_account(env: &Env, slug: &str) -> String {
    format!("{slug}.{}", env.factory.id())
}

pub async fn quote_for(env: &Env, kind: &str) -> Result<NearToken> {
    quote(&view(&env.worker, env.factory.id().as_str(), "quote_deposit", json!({ "kind": kind })).await?)
}
