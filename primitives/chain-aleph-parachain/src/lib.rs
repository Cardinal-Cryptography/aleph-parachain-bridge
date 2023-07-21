#![cfg_attr(not(feature = "std"), no_std)]
// RuntimeApi generated functions
#![allow(clippy::too_many_arguments)]

use bp_runtime::{decl_bridge_finality_runtime_apis, Chain, ChainId, Parachain};
use frame_support::{
	dispatch::DispatchClass,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, IdentityFee, Weight},
	RuntimeDebug, StateVersion,
};
use frame_system::limits;
use sp_core::Hasher as HasherT;
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, MultiSigner, Perbill,
};

pub const ALEPH_PARACHAIN_ID: u32 = 2106;
pub const EXTRA_STORAGE_PROOF_SIZE: u32 = 1024;
pub const TX_EXTRA_BYTES: u32 = 104;
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), 5 * 1024 * 1024);

pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

pub type BlockNumber = u32;
pub type Hash = <BlakeTwo256 as HasherT>::Out;
pub type Hasher = BlakeTwo256;
pub type Header = sp_runtime::generic::Header<BlockNumber, Hasher>;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type AccountSigner = MultiSigner;
pub type Balance = u128;
pub type Moment = u64;
pub type Index = u32;
pub type WeightToFee = IdentityFee<Balance>;

#[derive(RuntimeDebug)]
pub struct AlephParachain;

impl Chain for AlephParachain {
	const ID: ChainId = *b"a0pa";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Index;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V0;

	fn max_extrinsic_size() -> u32 {
		*BlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		BlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl Parachain for AlephParachain {
	const PARACHAIN_ID: u32 = ALEPH_PARACHAIN_ID;
}

pub use bp_bridge_hub_cumulus::SignedExtension;

frame_support::parameter_types! {
	pub BlockLength: limits::BlockLength =
		limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub BlockWeights: limits::BlockWeights =
		limits::BlockWeights::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO);
}


pub const TRANSACTION_PAYMENT_PALLET_NAME: &str = "TransactionPayment";

decl_bridge_finality_runtime_apis!(aleph_parachain);
