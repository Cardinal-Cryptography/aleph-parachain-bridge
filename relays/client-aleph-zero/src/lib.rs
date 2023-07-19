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
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(5);

	type SignedBlock = bp_polkadot_core::SignedBlock;
	type Call = RuntimeCall;
}

impl ChainWithBalances for AlephZero {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		bp_polkadot_core::AccountInfoStorageMapKeyProvider::final_key(account_id)
	}
}

/*impl ChainWithTransactions for AlephZero {
	type AccountKeyPair = sp_core::sr25519::Pair;
    type SignedTransaction = UncheckedExtrinsic<RuntimeCall, SignedExtension>;

    fn sign_transaction(
        param: relay_substrate_client::SignParam<Self>,
        unsigned: relay_substrate_client::UnsignedTransaction<Self>,
    ) -> Result<Self::SignedTransaction, relay_substrate_client::Error>
    where
        Self: Sized,
    {
        println!("{:?}", hex::encode(unsigned.call.encode()));

        let extra = SignedExtension::new(
            (
                (),                       // non-zero sender
                (),                       // spec version
                (),                       // tx version
                (),                       // genesis
                unsigned.era.frame_era(), // era
                unsigned.nonce.into(),    // nonce (compact encoding)
                (),                       // Check weight
                unsigned.tip.into(),      // transaction payment / tip (compact encoding)
            ),
            Some((
                (),
                param.spec_version,
                param.transaction_version,
                param.genesis_hash,
                unsigned.era.signed_payload(param.genesis_hash),
                (),
                (),
                (),
            )),
        );

        let raw_payload = SignedPayload::new(unsigned.call, extra);
        let raw_payload = raw_payload?;

        let signature = raw_payload.using_encoded(|payload| param.signer.sign(payload));
        let (call, extra, _) = raw_payload.deconstruct();

        Ok(UncheckedExtrinsic::new_signed(
            call,
            MultiAddress::Id(param.signer.public().into()),
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
}*/

pub type SigningParams = sp_core::sr25519::Pair;
pub type SyncHeader = relay_substrate_client::SyncHeader<bp_aleph_zero::Header>;
