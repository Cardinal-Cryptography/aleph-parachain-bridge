use codec::{Compact, Decode, Encode};
use relay_substrate_client::{
	BalanceOf, Chain, ChainWithBalances, ChainWithMessages, ChainWithTransactions,
	ChainWithUtilityPallet, Error as SubstrateError, FullRuntimeUtilityPallet, IndexOf, SignParam,
	UnderlyingChainProvider, UnsignedTransaction,
};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount};
use std::time::Duration;

// pub type HeaderId = relay_utils::HeaderId<millau_runtime::Hash, millau_runtime::BlockNumber>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlephZero;

impl UnderlyingChainProvider for Millau {
	type Chain = bp_millau::Millau;
}

impl Chain for AlephZero {
	const NAME: &'static str = "AlephZero";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		bp_aleph_zero::BEST_FINALIZED_MILLAU_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(1);

	//type SignedBlock = millau_runtime::SignedBlock;
	//type Call = millau_runtime::RuntimeCall;
}

impl ChainWithBalances for AlephZero {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		use frame_support::storage::generator::StorageMap;
		StorageKey(frame_system::Account::<millau_runtime::Runtime>::storage_map_final_key(
			account_id,
		))
	}
}

impl ChainWithTransactions for AlephZero {
	type AccountKeyPair = sp_core::sr25519::Pair;
	//type SignedTransaction = millau_runtime::UncheckedExtrinsic;

	fn sign_transaction(
		param: SignParam<Self>,
		unsigned: UnsignedTransaction<Self>,
	) -> Result<Self::SignedTransaction, SubstrateError> {
		let raw_payload = SignedPayload::from_raw(
			unsigned.call.clone(),
			(
				frame_system::CheckNonZeroSender::<millau_runtime::Runtime>::new(),
				frame_system::CheckSpecVersion::<millau_runtime::Runtime>::new(),
				frame_system::CheckTxVersion::<millau_runtime::Runtime>::new(),
				frame_system::CheckGenesis::<millau_runtime::Runtime>::new(),
				frame_system::CheckEra::<millau_runtime::Runtime>::from(unsigned.era.frame_era()),
				frame_system::CheckNonce::<millau_runtime::Runtime>::from(unsigned.nonce),
				frame_system::CheckWeight::<millau_runtime::Runtime>::new(),
				pallet_transaction_payment::ChargeTransactionPayment::<millau_runtime::Runtime>::from(unsigned.tip),
				millau_runtime::BridgeRejectObsoleteHeadersAndMessages,
				millau_runtime::BridgeRefundRialtoParachainMessages::default(),
			),
			(
				(),
				param.spec_version,
				param.transaction_version,
				param.genesis_hash,
				unsigned.era.signed_payload(param.genesis_hash),
				(),
				(),
				(),
				(),
				()
			),
		);
		let signature = raw_payload.using_encoded(|payload| param.signer.sign(payload));
		let signer: sp_runtime::MultiSigner = param.signer.public().into();
		let (call, extra, _) = raw_payload.deconstruct();

		Ok(millau_runtime::UncheckedExtrinsic::new_signed(
			call.into_decoded()?,
			signer.into_account(),
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
			.map(|(address, _, _)| {
				*address == millau_runtime::Address::from(*signer.public().as_array_ref())
			})
			.unwrap_or(false)
	}

	fn parse_transaction(tx: Self::SignedTransaction) -> Option<UnsignedTransaction<Self>> {
		let extra = &tx.signature.as_ref()?.2;
		Some(
			UnsignedTransaction::new(
				tx.function.into(),
				Compact::<IndexOf<Self>>::decode(&mut &extra.5.encode()[..]).ok()?.into(),
			)
			.tip(Compact::<BalanceOf<Self>>::decode(&mut &extra.7.encode()[..]).ok()?.into()),
		)
	}
}

impl ChainWithUtilityPallet for Millau {
	type UtilityPallet = FullRuntimeUtilityPallet<millau_runtime::Runtime>;
}

/// Millau signing params.
pub type SigningParams = sp_core::sr25519::Pair;

/// Millau header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<millau_runtime::Header>;

#[cfg(test)]
mod tests {
	use super::*;
	use relay_substrate_client::TransactionEra;

	#[test]
	fn parse_transaction_works() {
		let unsigned = UnsignedTransaction {
			call: millau_runtime::RuntimeCall::System(millau_runtime::SystemCall::remark {
				remark: b"Hello world!".to_vec(),
			})
			.into(),
			nonce: 777,
			tip: 888,
			era: TransactionEra::immortal(),
		};
		let signed_transaction = Millau::sign_transaction(
			SignParam {
				spec_version: 42,
				transaction_version: 50000,
				genesis_hash: [42u8; 64].into(),
				signer: sp_core::sr25519::Pair::from_seed_slice(&[1u8; 32]).unwrap(),
			},
			unsigned.clone(),
		)
		.unwrap();
		let parsed_transaction = Millau::parse_transaction(signed_transaction).unwrap();
		assert_eq!(parsed_transaction, unsigned);
	}
}