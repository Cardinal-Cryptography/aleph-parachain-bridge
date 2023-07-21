pub mod runtime_codegen;

use bp_bridge_hub_cumulus::BridgeHubSignedExtension;
use codec::Encode;
use relay_substrate_client::{
	Chain, ChainWithBalances, ChainWithTransactions, Error as SubstrateError,
	SignParam, UnderlyingChainProvider, UnsignedTransaction,
};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount, MultiAddress};
use std::time::Duration;

pub use runtime_codegen::api::runtime_types;

pub type RuntimeCall = runtime_types::aleph_parachain_runtime::RuntimeCall;
pub type SudoCall = runtime_types::pallet_sudo::pallet::Call;
pub type BridgeAlephCall = runtime_types::pallet_bridge_aleph::pallet::Call;

/// The address format for describing accounts.
pub type Address = MultiAddress<bp_aleph_parachain::AccountId, ()>;

#[derive(Debug, Clone, Copy)]
pub struct AlephParachain;

impl UnderlyingChainProvider for AlephParachain {
	type Chain = bp_aleph_parachain::AlephParachain;
}

impl Chain for AlephParachain {
	const NAME: &'static str = "AlephParachain";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		bp_aleph_parachain::BEST_FINALIZED_ALEPH_PARACHAIN_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(12);

	type SignedBlock = bp_polkadot_core::SignedBlock;
	type Call = RuntimeCall;
}

impl ChainWithBalances for AlephParachain {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		bp_polkadot_core::AccountInfoStorageMapKeyProvider::final_key(account_id)
	}
}

impl ChainWithTransactions for AlephParachain {
	type AccountKeyPair = sp_core::sr25519::Pair;
	type SignedTransaction =
		bp_polkadot_core::UncheckedExtrinsic<Self::Call, bp_aleph_parachain::SignedExtension>;

	fn sign_transaction(
		param: SignParam<Self>,
		unsigned: UnsignedTransaction<Self>,
	) -> Result<Self::SignedTransaction, SubstrateError> {
		let raw_payload = SignedPayload::new(
			unsigned.call,
			bp_aleph_parachain::SignedExtension::from_params(
				param.spec_version,
				param.transaction_version,
				unsigned.era,
				param.genesis_hash,
				unsigned.nonce,
				unsigned.tip,
			),
		)?;

		let signature = raw_payload.using_encoded(|payload| param.signer.sign(payload));
		let signer: sp_runtime::MultiSigner = param.signer.public().into();
		let (call, extra, _) = raw_payload.deconstruct();

		Ok(Self::SignedTransaction::new_signed(
			call,
			signer.into_account().into(),
			signature.into(),
			extra,
		))
	}

	fn is_signed(tx: &Self::SignedTransaction) -> bool {
		tx.signature.is_some()
	}

	fn is_signed_by(signer: &Self::AccountKeyPair, tx: &Self::SignedTransaction) -> bool {
		tx.signature
			.as_ref()
			.map(|(address, _, _)| *address == Address::Id(signer.public().into()))
			.unwrap_or(false)
	}

	fn parse_transaction(tx: Self::SignedTransaction) -> Option<UnsignedTransaction<Self>> {
		let extra = &tx.signature.as_ref()?.2;
		Some(UnsignedTransaction::new(tx.function, extra.nonce()).tip(extra.tip()))
	}
}

pub type SigningParams = sp_core::sr25519::Pair;
pub type SyncHeader = relay_substrate_client::SyncHeader<bp_aleph_parachain::Header>;
