//! Fixed-supply NEP-141 token (with NEP-145 storage and NEP-148 metadata).
//! The entire supply is minted to the configured allocations in `new`; no
//! method can create tokens afterwards.

use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::{FungibleToken, FungibleTokenCore, FungibleTokenResolver};
use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement};
use near_sdk::json_types::U128;
use near_sdk::store::LazyOption;
use near_sdk::{env, near, AccountId, NearToken, PromiseOrValue};
use orbit_common::config::{TokenAllocation, TokenConfig};

use crate::{part, part_mut, Contract, ContractExt, StorageKey};

const NAME: &str = "token";

#[near]
pub struct Fungible {
    pub ft: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    allocations: LazyOption<Vec<TokenAllocation>>,
}

impl Fungible {
    /// Registers and credits every allocation. Storage for those registrations
    /// is paid from the launch account's initial balance.
    pub fn new(config: TokenConfig) -> Self {
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

        let mut ft = FungibleToken::new(StorageKey::FtAccounts);
        for allocation in &config.allocations {
            ft.internal_register_account(&allocation.account_id);
            ft.internal_deposit(&allocation.account_id, allocation.amount.0);
        }
        let mints: Vec<FtMint> = config
            .allocations
            .iter()
            .map(|a| FtMint { owner_id: &a.account_id, amount: a.amount, memo: Some(&a.label) })
            .collect();
        FtMint::emit_many(&mints);

        Self {
            ft,
            metadata: LazyOption::new(StorageKey::FtMetadata, Some(metadata)),
            allocations: LazyOption::new(StorageKey::FtAllocations, Some(config.allocations)),
        }
    }

    /// Moves `amount` from the launch account's reserve to `receiver_id`,
    /// registering the receiver first if needed. The caller charges for the
    /// storage written; the returned yoctoNEAR is the extra to charge so that a
    /// new registration costs at least what `storage_unregister` refunds
    /// (`storage_balance_bounds().min`, sized for a 64-character account).
    /// Otherwise register/unregister cycles with short account IDs would pay
    /// out of the launch's own balance.
    pub fn pay_out(&mut self, receiver_id: &AccountId, amount: u128, memo: String) -> u128 {
        let mut surcharge = 0;
        if !self.ft.accounts.contains_key(receiver_id) {
            let before = env::storage_usage();
            self.ft.internal_register_account(receiver_id);
            let used = crate::storage_cost_since(before);
            surcharge = self.ft.storage_balance_bounds().min.as_yoctonear().saturating_sub(used);
        }
        self.ft.internal_transfer(&env::current_account_id(), receiver_id, amount, Some(memo));
        surcharge
    }
}

#[near]
impl Contract {
    /// The initial distribution recorded at launch. Balances may have moved since.
    pub fn orbit_allocations(&self) -> Vec<TokenAllocation> {
        part(&self.token, NAME).allocations.get().clone().unwrap_or_default()
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        part_mut(&mut self.token, NAME).ft.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        part_mut(&mut self.token, NAME).ft.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        part(&self.token, NAME).ft.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        part(&self.token, NAME).ft.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128) -> U128 {
        let (used_amount, burned_amount) =
            part_mut(&mut self.token, NAME).ft.internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
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
        part_mut(&mut self.token, NAME).ft.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        part_mut(&mut self.token, NAME).ft.storage_withdraw(amount)
    }

    /// With `force: true` a positive balance is burned, reducing total supply.
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        match part_mut(&mut self.token, NAME).ft.internal_storage_unregister(force) {
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
        part(&self.token, NAME).ft.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        part(&self.token, NAME).ft.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        part(&self.token, NAME).metadata.get().clone().unwrap_or_else(|| env::panic_str("metadata missing"))
    }
}
