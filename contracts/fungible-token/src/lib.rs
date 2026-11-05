//! Fixed-supply NEP-141 token. The entire supply is minted to the configured
//! allocations in `new`; no method can create tokens afterwards.

use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::{FungibleToken, FungibleTokenCore, FungibleTokenResolver};
use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement};
use near_sdk::json_types::U128;
use near_sdk::store::LazyOption;
use near_sdk::{env, near, AccountId, BorshStorageKey, NearToken, PanicOnDefault, PromiseOrValue};
use orbit_common::config::{TokenAllocation, TokenConfig};
use orbit_common::validate::validate_token_config;

#[near]
#[derive(BorshStorageKey)]
enum StorageKey {
    Accounts,
    Metadata,
    Allocations,
}

#[near(
    contract_state,
    contract_metadata(
        version = "0.1.0",
        link = "https://github.com/orbit-launch/orbit",
        standard(standard = "nep141", version = "1.0.0"),
        standard(standard = "nep145", version = "1.0.0"),
        standard(standard = "nep148", version = "1.0.0"),
        standard(standard = "nep297", version = "1.0.0"),
        standard(standard = "nep330", version = "1.2.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    allocations: LazyOption<Vec<TokenAllocation>>,
    creator_id: AccountId,
}

#[near]
impl Contract {
    /// Registers and credits every allocation. Storage for those registrations
    /// is paid from this account's initial balance.
    #[init]
    pub fn new(config: TokenConfig, creator_id: AccountId) -> Self {
        validate_token_config(&config).unwrap_or_else(|e| env::panic_str(&e));
        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: config.name,
            symbol: config.symbol,
            icon: config.icon,
            reference: None,
            reference_hash: None,
            decimals: config.decimals,
        };
        metadata.assert_valid();

        let mut token = FungibleToken::new(StorageKey::Accounts);
        for allocation in &config.allocations {
            token.internal_register_account(&allocation.account_id);
            token.internal_deposit(&allocation.account_id, allocation.amount.0);
        }
        let mints: Vec<FtMint> = config
            .allocations
            .iter()
            .map(|a| FtMint { owner_id: &a.account_id, amount: a.amount, memo: Some(&a.label) })
            .collect();
        FtMint::emit_many(&mints);

        Self {
            token,
            metadata: LazyOption::new(StorageKey::Metadata, Some(metadata)),
            allocations: LazyOption::new(StorageKey::Allocations, Some(config.allocations)),
            creator_id,
        }
    }

    /// The initial distribution recorded at launch. Balances may have moved since.
    pub fn orbit_allocations(&self) -> Vec<TokenAllocation> {
        self.allocations.get().clone().unwrap_or_default()
    }

    pub fn orbit_creator(&self) -> AccountId {
        self.creator_id.clone()
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128) -> U128 {
        let (used_amount, burned_amount) =
            self.token.internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            FtBurn {
                owner_id: &sender_id,
                amount: U128(burned_amount),
                memo: Some("receiver unregistered during transfer"),
            }
            .emit();
        }
        used_amount.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    /// With `force: true` a positive balance is burned, reducing total supply.
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        match self.token.internal_storage_unregister(force) {
            Some((account_id, balance)) => {
                if balance > 0 {
                    FtBurn {
                        owner_id: &account_id,
                        amount: U128(balance),
                        memo: Some("storage unregistered"),
                    }
                    .emit();
                }
                true
            }
            None => false,
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().clone().unwrap_or_else(|| env::panic_str("metadata missing"))
    }
}

#[cfg(test)]
mod tests;
