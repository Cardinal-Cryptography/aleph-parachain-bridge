#![cfg_attr(not(feature = "std"), no_std)]
// RuntimeApi generated functions
#![allow(clippy::too_many_arguments)]

use frame_support::{StateVersion, sp_runtime::{traits::{BlakeTwo256, IdentifyAccount, Verify}, generic, MultiSignature}};
use sp_core::H256;

use bp_aleph_header_chain::ChainWithAleph;
use bp_runtime::{decl_bridge_finality_runtime_apis, Chain, ChainId};
use frame_support::weights::Weight;

// Upper bound on the number of authorities in one session.
const MAX_AUTHORITIES_COUNT: u32 = 200;

// Upper bound on the size of a header (in bytes).
// TODO: check if this is a reasonable value.
const MAX_HEADER_SIZE: u32 = 32 * 1024;

pub type BlockNumber = u32;
pub type Hash = H256;
pub type Hasher = BlakeTwo256;
pub type Header = generic::Header<BlockNumber, Hasher>;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Index = u32;

/// Aleph Zero Chain
pub struct AlephZero;

impl Chain for AlephZero {
	const ID: ChainId = *b"alp0";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = BlakeTwo256;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Index;
	type Signature = MultiSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		unimplemented!("Used unimplemented function: max_extrinsic_size")
	}

	fn max_extrinsic_weight() -> Weight {
		unimplemented!("Used unimplemented function: max_extrinsic_weight")
	}
}

impl ChainWithAleph for AlephZero {
	const WITH_CHAIN_ALEPH_PALLET_NAME: &'static str = "BridgeAleph";
	const MAX_AUTHORITIES_COUNT: u32 = MAX_AUTHORITIES_COUNT;
	const MAX_HEADER_SIZE: u32 = MAX_HEADER_SIZE;
}

pub const WITH_ALEPH_PALLET_NAME: &str = "BridgeAleph";

// It needs `Hash` and `BlockNumber` to be defined.
decl_bridge_finality_runtime_apis!(aleph_zero);
