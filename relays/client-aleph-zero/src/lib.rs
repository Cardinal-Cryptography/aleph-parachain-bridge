pub mod runtime_codegen;

use codec::Encode;
use relay_substrate_client::{
	Chain, ChainWithBalances, ChainWithTransactions, Error as SubstrateError,
	SignParam, UnderlyingChainProvider, UnsignedTransaction,
};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount, MultiAddress};
use std::time::Duration;
use sp_runtime::generic::UncheckedExtrinsic;

pub use runtime_codegen::api::runtime_types;

pub type RuntimeCall = runtime_types::aleph_runtime::RuntimeCall;
pub type SudoCall = runtime_types::pallet_sudo::pallet::Call;

/// The address format for describing accounts.
pub type Address = MultiAddress<bp_aleph_zero::AccountId, ()>;

#[derive(Debug, Clone, Copy)]
pub struct AlephZero;

impl UnderlyingChainProvider for AlephZero {
	type Chain = bp_aleph_zero::AlephZero;
}

impl Chain for AlephZero {
	const NAME: &'static str = "AlephZero";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		bp_aleph_zero::BEST_FINALIZED_ALEPH_ZERO_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(1);

	type SignedBlock = bp_polkadot_core::SignedBlock;
	type Call = RuntimeCall;
}

impl ChainWithBalances for AlephZero {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		bp_polkadot_core::AccountInfoStorageMapKeyProvider::final_key(account_id)
	}
}

pub type SigningParams = sp_core::sr25519::Pair;
pub type SyncHeader = relay_substrate_client::SyncHeader<bp_aleph_zero::Header>;
