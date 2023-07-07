// Copyright 2023 Cardinal Cryptography
// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

// This pallet is largely based on the `GRANDPA` pallet, but is changed to work with AlephBFT

//! Aleph bridging Pallet
//!
//! This pallet is an on-chain AlephBFT light client.
//!
//! Right now, it does not support submiting finality proofs, but only
//! being directly initialized with a header and authority set.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

use bp_aleph_header_chain::{
	aleph_justification::{verify_justification, AlephJustification},
	AuthoritySet, ChainWithAleph, ConsensusLog, InitializationData, ALEPH_ENGINE_ID,
};
use bp_header_chain::{HeaderChain, StoredHeaderData, StoredHeaderDataBuilder};
use bp_runtime::{BlockNumberOf, HashOf, HasherOf, HeaderId, HeaderOf, OwnedBridgeModule};
use frame_support::sp_runtime::traits::Header;

#[cfg(test)]
mod mock;
mod storage_types;

use storage_types::StoredAuthoritySet;

// Re-export in crate namespace for `construct_runtime!`
pub use pallet::*;

pub const LOG_TARGET: &str = "runtime::bridge-aleph";

pub type BridgedChain<T> = <T as Config>::BridgedChain;
pub type BridgedBlockNumber<T> = BlockNumberOf<<T as Config>::BridgedChain>;
pub type BridgedBlockHash<T> = HashOf<<T as Config>::BridgedChain>;
pub type BridgedBlockId<T> = HeaderId<BridgedBlockHash<T>, BridgedBlockNumber<T>>;
pub type BridgedBlockHasher<T> = HasherOf<<T as Config>::BridgedChain>;
pub type BridgedHeader<T> = HeaderOf<<T as Config>::BridgedChain>;
pub type BridgedStoredHeaderData<T> = StoredHeaderData<BridgedBlockNumber<T>, BridgedBlockHash<T>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use bp_runtime::BasicOperatingMode;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type BridgedChain: ChainWithAleph;
		#[pallet::constant]
		type HeadersToKeep: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			Weight::zero()
		}
	}

	impl<T: Config> OwnedBridgeModule<T> for Pallet<T> {
		const LOG_TARGET: &'static str = LOG_TARGET;
		type OwnerStorage = PalletOwner<T>;
		type OperatingMode = BasicOperatingMode;
		type OperatingModeStorage = PalletOperatingMode<T>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Verify a target header is finalized according to the given finality proof.
		///
		/// It will use the underlying storage pallet to fetch information about the current
		/// authorities and best finalized header in order to verify that the header is finalized.
		///
		/// If successful in verification, it will write the target header to the underlying storage
		/// pallet.
		///
		/// The call fails if:
		///
		/// - the pallet is halted;
		///
		/// - the pallet knows better header than the `finality_target`;
		///
		/// - justification is invalid;
		///
		/// For now, weights are incorrect.
		#[pallet::call_index(0)]
		// TODO: Set correct weights
		#[pallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn submit_finality_proof(
			_origin: OriginFor<T>,
			header: BridgedHeader<T>,
			justification: AlephJustification,
		) -> DispatchResultWithPostInfo {
			Self::ensure_not_halted().map_err(Error::<T>::BridgeModule)?;

			// Check of obsolete header
			if let Some(best_finalized_block) = Self::best_finalized() {
				if header.number() <= &best_finalized_block.number() {
					log::debug!(
						target: LOG_TARGET,
						"Skipping import of an old header: {:?}.",
						header.hash()
					);
					return Err(Error::<T>::OldHeader.into())
				}
			}

			// Check justification
			let authority_set = <CurrentAuthoritySet<T>>::get();
			verify_justification::<BridgedHeader<T>>(
				&authority_set.into(),
				&header,
				&justification,
			)
			.map_err(|verification_err| Error::<T>::InvalidJustification(verification_err))?;

			// Check for authority set change digest
			Self::try_enact_authority_change(&header)?;

			// Insert new header
			Self::insert_header(header.clone());
			log::info!(
				target: LOG_TARGET,
				"Successfully imported finalized header with hash {:?}!",
				header.hash()
			);

			Self::deposit_event(Event::UpdatedBestFinalizedHeader {
				number: *header.number(),
				hash: header.hash(),
			});

			Ok(().into())
		}

		/// Bootstrap the bridge pallet with an initial header and authority set from which to sync.
		///
		/// The initial configuration provided does not need to be the genesis header of the bridged
		/// chain, it can be any arbitrary header. You can also provide the next scheduled set
		/// change if it is already known.
		///
		/// This function is only allowed to be called from a trusted origin and writes to storage
		/// with practically no checks in terms of the validity of the data. It is important that
		/// you ensure that valid data is being passed in.
		///
		/// Difference with GRANDPA: This function can only be called by root.
		///
		/// Note: It cannot be called once the bridge has been initialized.
		/// To reinitialize the bridge, you must reinitialize the pallet.
		#[pallet::call_index(1)]
		#[pallet::weight((T::DbWeight::get().reads_writes(2, 5), DispatchClass::Operational))]
		pub fn initialize(
			origin: OriginFor<T>,
			init_data: super::InitializationData<BridgedHeader<T>>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			// Check if already initialized
			let init_allowed = !<BestFinalized<T>>::exists();
			ensure!(init_allowed, <Error<T>>::AlreadyInitialized);
			Self::initialize_bridge(init_data.clone())?;

			log::info!(
				target: LOG_TARGET,
				"Pallet has been initialized with the following parameters: {:?}",
				init_data
			);

			Ok(().into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_operating_mode(
			origin: OriginFor<T>,
			operating_mode: BasicOperatingMode,
		) -> DispatchResult {
			<Self as OwnedBridgeModule<_>>::set_operating_mode(origin, operating_mode)
		}
	}

	/// Hash of the best finalized header.
	#[pallet::storage]
	#[pallet::getter(fn best_finalized)]
	pub type BestFinalized<T: Config> = StorageValue<_, BridgedBlockId<T>, OptionQuery>;

	/// A ring buffer of imported hashes. Ordered by insertion time.
	#[pallet::storage]
	pub(super) type ImportedHashes<T: Config> = StorageMap<
		Hasher = Identity,
		Key = u32,
		Value = BridgedBlockHash<T>,
		QueryKind = OptionQuery,
		OnEmpty = GetDefault,
		MaxValues = MaybeHeadersToKeep<T>,
	>;

	/// Current ring buffer position.
	#[pallet::storage]
	pub(super) type ImportedHashesPointer<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Relevant fields of imported headers.
	#[pallet::storage]
	pub type ImportedHeaders<T: Config> = StorageMap<
		Hasher = Identity,
		Key = BridgedBlockHash<T>,
		Value = BridgedStoredHeaderData<T>,
		QueryKind = OptionQuery,
		OnEmpty = GetDefault,
		MaxValues = MaybeHeadersToKeep<T>,
	>;

	/// The current Aleph Authority set.
	#[pallet::storage]
	pub type CurrentAuthoritySet<T: Config> = StorageValue<_, StoredAuthoritySet<T>, ValueQuery>;

	/// Optional pallet owner.
	///
	/// Pallet owner has the right to halt all pallet operations and then resume them. If it is
	/// `None`, then there are no direct ways to halt/resume pallet operations, but other
	/// runtime methods may still be used to do that (i.e. democracy::referendum to update halt
	/// flag directly or call the `halt_operations`).
	#[pallet::storage]
	pub type PalletOwner<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The current operating mode of the pallet.
	///
	/// Depending on the mode either all, or no transactions will be allowed.
	#[pallet::storage]
	pub type PalletOperatingMode<T: Config> = StorageValue<_, BasicOperatingMode, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub owner: Option<T::AccountId>,
		pub init_data: Option<super::InitializationData<BridgedHeader<T>>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { owner: None, init_data: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if let Some(ref owner) = self.owner {
				<PalletOwner<T>>::put(owner);
			}

			if let Some(init_data) = self.init_data.clone() {
				Pallet::<T>::initialize_bridge(init_data).expect("genesis config is correct; qed");
			} else {
				// Since the bridge hasn't been initialized we shouldn't allow anyone to perform
				// transactions.
				<PalletOperatingMode<T>>::put(BasicOperatingMode::Halted);
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Best finalized chain header has been updated to the header with given number and hash.
		UpdatedBestFinalizedHeader { number: BridgedBlockNumber<T>, hash: BridgedBlockHash<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The given justification is invalid for the given header.
		InvalidJustification(bp_aleph_header_chain::aleph_justification::Error),
		/// The header being imported is older than the best finalized header known to the pallet.
		OldHeader,
		/// The pallet is not yet initialized.
		NotInitialized,
		/// The pallet has already been initialized.
		AlreadyInitialized,
		/// Header is older than the last imported header.
		TooManyAuthoritiesInSet,
		/// Error generated by the `OwnedBridgeModule` trait.
		BridgeModule(bp_runtime::OwnedBridgeModuleError),
	}

	impl<T: Config> Pallet<T> {
		/// Import a previously verified header to the storage.
		///
		/// Note this function solely takes care of updating the storage and pruning old entries,
		/// but does not verify the validity of such import.
		pub(crate) fn insert_header(header: BridgedHeader<T>) {
			let hash = header.hash();
			let index = <ImportedHashesPointer<T>>::get();
			let pruning = <ImportedHashes<T>>::try_get(index);
			<BestFinalized<T>>::put(HeaderId(*header.number(), hash));
			<ImportedHeaders<T>>::insert(hash, header.build());
			<ImportedHashes<T>>::insert(index, hash);

			// Update ring buffer pointer and remove old header.
			<ImportedHashesPointer<T>>::put((index + 1) % T::HeadersToKeep::get());
			if let Ok(hash) = pruning {
				log::debug!(target: LOG_TARGET, "Pruning old header: {:?}.", hash);
				<ImportedHeaders<T>>::remove(hash);
			}
		}

		/// Since this writes to storage with no real checks this should only be used in functions
		/// that were called by a trusted origin.
		pub(crate) fn initialize_bridge(
			init_params: super::InitializationData<BridgedHeader<T>>,
		) -> Result<(), Error<T>> {
			let super::InitializationData { header, authority_list, operating_mode } = init_params;
			let authority_set_length = authority_list.len();
			let authority_set = StoredAuthoritySet::<T>::try_new(authority_list)
			.map_err(|e| {
				log::error!(
					target: LOG_TARGET,
					"Failed to initialize bridge. Number of authorities in the set {} is larger than the configured value {}",
					authority_set_length,
					T::BridgedChain::MAX_AUTHORITIES_COUNT,
				);
				e
			})?;

			<ImportedHashesPointer<T>>::put(0);
			Self::insert_header(*header);

			<CurrentAuthoritySet<T>>::put(authority_set);
			<PalletOperatingMode<T>>::put(operating_mode);

			Ok(())
		}

		/// Check the given header for an authority set change. If a change
		/// is found it will be enacted immediately.
		pub(crate) fn try_enact_authority_change(
			header: &BridgedHeader<T>,
		) -> Result<bool, Error<T>> {
			let mut change_enacted = false;

			if let Some(change) = super::get_authority_change(header) {
				let next_authorities = StoredAuthoritySet::<T>::try_new(change)?;
				<CurrentAuthoritySet<T>>::put(&next_authorities);
				change_enacted = true;

				log::info!(
					target: LOG_TARGET,
					"New authorities are: {:?}",
					next_authorities,
				);
			};

			Ok(change_enacted)
		}
	}

	/// Adapter for using `Config::HeadersToKeep` as `MaxValues` bound in our storage maps.
	pub struct MaybeHeadersToKeep<T>(PhantomData<T>);

	// this implementation is required to use the struct as `MaxValues`
	impl<T: Config> Get<Option<u32>> for MaybeHeadersToKeep<T> {
		fn get() -> Option<u32> {
			Some(T::HeadersToKeep::get())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Get the best finalized block number.
	pub fn best_finalized_number() -> Option<BridgedBlockNumber<T>> {
		BestFinalized::<T>::get().map(|id| id.number())
	}
}

/// Bridge Aleph pallet as header chain.
pub type AlephChainHeaders<T> = Pallet<T>;

impl<T: Config> HeaderChain<BridgedChain<T>> for AlephChainHeaders<T> {
	fn finalized_header_state_root(
		header_hash: HashOf<BridgedChain<T>>,
	) -> Option<HashOf<BridgedChain<T>>> {
		ImportedHeaders::<T>::get(header_hash).map(|h| h.state_root)
	}
}

pub(crate) fn get_authority_change<H: Header>(header: &H) -> Option<AuthoritySet> {
	use sp_runtime::generic::OpaqueDigestItemId;
	let id = OpaqueDigestItemId::Consensus(&ALEPH_ENGINE_ID);
	let filter_log = |log: ConsensusLog| match log {
		ConsensusLog::AlephAuthorityChange(change) => Some(change),
	};

	header.digest().convert_first(|l| l.try_to(id).and_then(filter_log))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{
		run_test, test_header, Aleph, RuntimeOrigin, System, TestHeader, TestRuntime,
		MAX_BRIDGED_AUTHORITIES, MAX_HEADERS_KEPT,
	};
	use bp_aleph_header_chain::{
		aleph_justification::test_utils::{
			aleph_justification_from_hex, decode_from_hex, raw_authorities_into_authority_set,
			AURA_ENGINE_ID,
		},
		AuthorityId,
	};
	use bp_runtime::{BasicOperatingMode, UnverifiedStorageProof};
	use bp_test_utils::{generate_owned_bridge_module_tests, Account, ALICE, BOB, CHARLIE};
	use frame_support::{
		assert_noop, assert_ok, dispatch::PostDispatchInfo, storage::generator::StorageValue,
	};
	use hex::FromHex;
	use sp_runtime::{Digest, DigestItem, DispatchError};

	fn into_authority_set(accounts: Vec<Account>) -> Vec<AuthorityId> {
		accounts.into_iter().map(|a| AuthorityId::from(a)).collect()
	}

	fn initialize_with_custom_data(init_data: InitializationData<TestHeader>) {
		System::set_block_number(1);
		System::reset_events();

		assert_ok!(init_with_origin(RuntimeOrigin::root(), init_data));
	}

	fn test_init_data() -> InitializationData<TestHeader> {
		let genesis = test_header(0);
		let authority_list = into_authority_set(vec![ALICE, BOB, CHARLIE]);
		let operating_mode = BasicOperatingMode::Normal;
		InitializationData { header: Box::new(genesis), authority_list, operating_mode }
	}

	fn initialize_substrate_bridge() {
		System::set_block_number(1);
		System::reset_events();

		let init_data = test_init_data();
		assert_ok!(init_with_origin(RuntimeOrigin::root(), init_data));
	}

	fn init_with_origin(
		origin: RuntimeOrigin,
		init_data: InitializationData<TestHeader>,
	) -> Result<
		InitializationData<TestHeader>,
		sp_runtime::DispatchErrorWithPostInfo<PostDispatchInfo>,
	> {
		Aleph::initialize(origin, init_data.clone()).map(|_| init_data)
	}

	generate_owned_bridge_module_tests!(BasicOperatingMode::Normal, BasicOperatingMode::Halted);

	#[test]
	fn init_root_origin_can_initialize_pallet() {
		run_test(|| {
			assert_ok!(init_with_origin(RuntimeOrigin::root(), test_init_data()));
		})
	}

	#[test]
	fn init_random_user_cannot_initialize_pallet() {
		run_test(|| {
			assert_noop!(
				init_with_origin(RuntimeOrigin::signed(1), test_init_data()),
				DispatchError::BadOrigin
			);
		})
	}

	#[test]
	fn init_owner_cannot_initialize_pallet() {
		run_test(|| {
			PalletOwner::<TestRuntime>::put(2);
			assert_noop!(
				init_with_origin(RuntimeOrigin::signed(2), test_init_data()),
				DispatchError::BadOrigin
			);
		})
	}

	#[test]
	fn init_storage_entries_are_correctly_initialized() {
		run_test(|| {
			assert_eq!(BestFinalized::<TestRuntime>::get(), None,);
			assert_eq!(Aleph::best_finalized(), None);
			assert_eq!(PalletOperatingMode::<TestRuntime>::try_get(), Err(()));

			let init_data = init_with_origin(RuntimeOrigin::root(), test_init_data()).unwrap();

			assert!(<ImportedHeaders<TestRuntime>>::contains_key(init_data.header.hash()));
			assert_eq!(BestFinalized::<TestRuntime>::get().unwrap().1, init_data.header.hash());
			assert_eq!(
				CurrentAuthoritySet::<TestRuntime>::get().authorities,
				init_data.authority_list
			);
			assert_eq!(
				PalletOperatingMode::<TestRuntime>::try_get(),
				Ok(BasicOperatingMode::Normal)
			);
		})
	}

	#[test]
	fn init_can_only_initialize_pallet_once() {
		run_test(|| {
			initialize_substrate_bridge();
			assert_noop!(
				init_with_origin(RuntimeOrigin::root(), test_init_data()),
				<Error<TestRuntime>>::AlreadyInitialized
			);
		})
	}

	#[test]
	fn init_fails_if_there_are_too_many_authorities_in_the_set() {
		run_test(|| {
			let genesis = test_header(0);
			let init_data = InitializationData {
				header: Box::new(genesis),
				authority_list: into_authority_set(
					(0..(MAX_BRIDGED_AUTHORITIES as u16) + 1).map(|x| Account(x)).collect(),
				),
				operating_mode: BasicOperatingMode::Normal,
			};

			assert_noop!(
				Aleph::initialize(RuntimeOrigin::root(), init_data),
				Error::<TestRuntime>::TooManyAuthoritiesInSet,
			);
		});
	}

	#[test]
	fn parse_finalized_storage_accepts_valid_proof() {
		run_test(|| {
			let (state_root, storage_proof) = UnverifiedStorageProof::try_from_entries::<
				sp_core::Blake2Hasher,
			>(Default::default(), &[(b"key1".to_vec(), None)])
			.expect("UnverifiedStorageProof::try_from_entries() shouldn't fail in tests");

			let mut header = test_header(2);
			header.set_state_root(state_root);

			let hash = header.hash();
			<BestFinalized<TestRuntime>>::put(HeaderId(2, hash));
			<ImportedHeaders<TestRuntime>>::insert(hash, header.build());

			assert_ok!(Aleph::verify_storage_proof(hash, storage_proof).map(|_| ()));
		});
	}

	#[test]
	fn storage_keys_computed_properly() {
		assert_eq!(
			PalletOperatingMode::<TestRuntime>::storage_value_final_key().to_vec(),
			bp_header_chain::storage_keys::pallet_operating_mode_key("Aleph").0,
		);

		assert_eq!(
			CurrentAuthoritySet::<TestRuntime>::storage_value_final_key().to_vec(),
			bp_header_chain::storage_keys::current_authority_set_key("Aleph").0,
		);

		assert_eq!(
			BestFinalized::<TestRuntime>::storage_value_final_key().to_vec(),
			bp_header_chain::storage_keys::best_finalized_key("Aleph").0,
		);
	}

	#[test]
	fn insert_header_simple() {
		run_test(|| {
			initialize_substrate_bridge();

			let header = test_header(1);
			let hash = header.hash();

			Aleph::insert_header(header.clone());
			assert_eq!(<ImportedHeaders<TestRuntime>>::get(hash), Some(header.build()));
		})
	}

	#[test]
	fn insert_header_pruning() {
		run_test(|| {
			initialize_substrate_bridge();

			for i in 0..2 * MAX_HEADERS_KEPT {
				let header = test_header(i as u64);
				Aleph::insert_header(header.clone());
			}

			assert_eq!(ImportedHeaders::<TestRuntime>::iter().count(), MAX_HEADERS_KEPT as usize);

			for i in 0..MAX_HEADERS_KEPT {
				let header = test_header(i as u64);
				assert!(!<ImportedHeaders<TestRuntime>>::contains_key(header.hash()));
			}

			for i in MAX_HEADERS_KEPT..2 * MAX_HEADERS_KEPT {
				let header = test_header(i as u64);
				let hash = header.hash();
				assert!(<ImportedHeaders<TestRuntime>>::contains_key(header.hash()));
				assert_eq!(<ImportedHeaders<TestRuntime>>::get(hash), Some(header.build()));
			}
		})
	}

	// Some tests with "real-life" data
	// Best case these were testnet/mainnet blocks, but there are no needed digests there yet
	const FIRST_RAW_DEVNET_AUTHORITY_SET: [&str; 4] = [
		"11bf91f48b4e2d71fb33e4690e427ed12e989a9d9adea06ab18cacd7ea859a29",
		"09a63b4c82345fac9594b7e0ccfc007983f8be6e75de1fe52e7d1d083b9d8efd",
		"4002cbce061068c7e090124116e3d3e8a489fc8e78889ed530db385b72a7e733",
		"40d67c86151aec6be972125a9330c4b385ad6faf6f5caacbe5c9c52259df5cff",
	];

	// It's devnet so only reorder
	const SECOND_RAW_DEVNET_AUTHORITY_SET: [&str; 4] = [
		"40d67c86151aec6be972125a9330c4b385ad6faf6f5caacbe5c9c52259df5cff",
		"11bf91f48b4e2d71fb33e4690e427ed12e989a9d9adea06ab18cacd7ea859a29",
		"09a63b4c82345fac9594b7e0ccfc007983f8be6e75de1fe52e7d1d083b9d8efd",
		"4002cbce061068c7e090124116e3d3e8a489fc8e78889ed530db385b72a7e733",
	];

	// Block in old session
	fn devnet_header_and_justification_1() -> (AuthoritySet, TestHeader, AlephJustification) {
		(
			raw_authorities_into_authority_set(&FIRST_RAW_DEVNET_AUTHORITY_SET),
			Header::new(
			67,
			decode_from_hex("5076726bb7e9891769edee00786fc7198c8342571f536ec2cad6ad70070a2d4a"),
			decode_from_hex("107c5dbfafd10aabfb58858aa638d79f3c271bc01c07807c810eb5a68f618397"),
			decode_from_hex("6a6dafa93280b8ca231c8101506526f385a348a89399abbee19454c5204dbf11"), 
			Digest {
				logs: vec![
				DigestItem::PreRuntime(AURA_ENGINE_ID, FromHex::from_hex("0dd4a66400000000").unwrap()), 
				DigestItem::Seal(AURA_ENGINE_ID, FromHex::from_hex("2204ea168b79d00d57d54ecd4c8e43318a9dd7a6d438bd4dedc9f3048c6fd626de3d0bdd67a2986e82b100895ba263a9f98eb907560c16aa5a896e0ef723778c").unwrap())
			]}
			),
			aleph_justification_from_hex("0300c60000100001d21a34871a5cadd58acf9f25bbac2fed401a9e74a468a62713e3dd6b9a08fef28c11ceb90e6f2ab83b8ef41cf00aff649c7815d555b4864c4093dd67ce2d8b080183834ada5c662c1237893ec1ee73fc0eb7de63dfaf4353b7677e02355b128d09be7eb0b2f73ee086131296aa7f668af9bf4ae063bebfe9bfeade0a988ff56709013ef0131b3164341c225091e7c3cdb1c069e4a0a6cf7a1fcb0576f18b6194bf3112bbbee7dc111093be93c945386b558448e3b7dee17a3652d567d39163db0605")
		)
	}

	// Block with authority set change
	fn devnet_header_and_justification_2() -> (AuthoritySet, TestHeader, AlephJustification) {
		(
			raw_authorities_into_authority_set(&FIRST_RAW_DEVNET_AUTHORITY_SET),
			Header::new(
			89,
			decode_from_hex("c1321ba69772a314d049d0afae0b67399ba738ee4c2c54e5e4162e8e9b6d5950"),
			decode_from_hex("c768176add8e5ec8d6b480267322990a8b7ad3cbf73d89567a2aec099c671338"),
			decode_from_hex("1f3b80b9e560fbecac1191c7ba45555bcba13211a6c9fcc78ab861a92a12e08c"), 
			Digest {
				logs: vec![
				DigestItem::PreRuntime(AURA_ENGINE_ID, FromHex::from_hex("23d4a66400000000").unwrap()), 
				DigestItem::Consensus(ALEPH_ENGINE_ID, FromHex::from_hex("011040d67c86151aec6be972125a9330c4b385ad6faf6f5caacbe5c9c52259df5cff11bf91f48b4e2d71fb33e4690e427ed12e989a9d9adea06ab18cacd7ea859a2909a63b4c82345fac9594b7e0ccfc007983f8be6e75de1fe52e7d1d083b9d8efd4002cbce061068c7e090124116e3d3e8a489fc8e78889ed530db385b72a7e733").unwrap()),
				DigestItem::Seal(AURA_ENGINE_ID, FromHex::from_hex("4ac4d2960b601fface5822bc14a330ed1537da4b1fed746f9d33b9c3fe39724ff7ef7b9d17278b1bde49fe2be87469a5bd3521b3b8b1d9c67867eeaa78977f8d").unwrap())
			]}
			),
			aleph_justification_from_hex("0300c600001001af459ef570e50fd2ed66782d8f748ed6d168b13fb18573b308c3a8394b0925a135dae4027dc429d20e1f876424d5587b8c6f44b0cd151a3d64e6e11def0e9f0700017883bbbfca6151d66f0dc8f7bce09f80996170e45759054ad00e66aee9165c67dde33065a9c2901d90f78b1cbf525fdc34af8ffbb5cb047da7f4865242aacb02018dc0ea01007e7b532f8f4346ead3a8418c7ed3984d94e66029120fedda4107f4757617f816cddfe188e95844d20733851e25ac3c0f87ed25a990704095dfb508")
		)
	}

	// Block in new session
	fn devnet_header_and_justification_3() -> (AuthoritySet, TestHeader, AlephJustification) {
		(
			raw_authorities_into_authority_set(&SECOND_RAW_DEVNET_AUTHORITY_SET),
			Header::new(
			110,
			decode_from_hex("dbc61ebc503528f4a604837267e15c05c954da7cddd819e435f4dceeceb4fec3"),
			decode_from_hex("1578c9b389cbbf8c01ec0617fe6c6c74f515f3d3c02160bd1f1eff78efea1a41"),
			decode_from_hex("02c6cff666769bcfe0d41d39915e9f2d5a7007c85b285e11f85a9e36b6d7456c"), 
			Digest {
				logs: vec![
				DigestItem::PreRuntime(AURA_ENGINE_ID, FromHex::from_hex("38d4a66400000000").unwrap()), 
				DigestItem::Seal(AURA_ENGINE_ID, FromHex::from_hex("369c0862010710603e91c024bc3ecdf0a4de082e364cfe8569643332b177df069441220a457ececde41f7a4740679de289b85f632ad161f2671101f2551a0b8a").unwrap())
			]}
			),
			aleph_justification_from_hex("0300c6000010016ab9f905b9a9d3d19d61165e58998b9c774066e14cff2f3fccb0eac86c497645aacebc33c0ca9df45f11ffcf41c8d308ba414d3462ef8b3374bc6d0d7976a705013aebdf2ea618c094962a7180b2da31b738f36abf1aef3d2e63693f736a934c927c8959aec8b451700c2d503d2bbc89e9ab1db9211c27b7ad949701441fc5c1060001d060b064a4cb27991084f5ea84504a429178b0846420d44a990a249b2db7d918c410a97091d79e6aeab773a99f292ff4f798012af0b97fd0d559019e462a2e05")
		)
	}

	#[test]
	fn accepts_devnet_justification() {
		run_test(|| {
			let (init_authority_set, init_header, _justification) =
				devnet_header_and_justification_1();
			initialize_with_custom_data(InitializationData {
				authority_list: init_authority_set,
				header: Box::new(init_header),
				operating_mode: BasicOperatingMode::Normal,
			});

			let (_authority_set, header, justification) = devnet_header_and_justification_2();
			assert_ok!(Aleph::submit_finality_proof(
				RuntimeOrigin::signed(1),
				header,
				justification
			));
		})
	}

	#[test]
	fn finds_authority_change_log() {
		let (_, header, _) = devnet_header_and_justification_2();
		assert!(get_authority_change(&header).is_some());
	}

	#[test]
	fn accepts_testnet_justifications_with_authority_change() {
		run_test(|| {
			let (init_authority_set, init_header, _justification) =
				devnet_header_and_justification_1();
			initialize_with_custom_data(InitializationData {
				authority_list: init_authority_set,
				header: Box::new(init_header),
				operating_mode: BasicOperatingMode::Normal,
			});

			let (_authority_set, header, justification) = devnet_header_and_justification_2();
			assert_ok!(Aleph::submit_finality_proof(
				RuntimeOrigin::signed(1),
				header,
				justification
			));

			let (_authority_set_2, header_2, justification_2) = devnet_header_and_justification_3();
			assert_ok!(Aleph::submit_finality_proof(
				RuntimeOrigin::signed(1),
				header_2,
				justification_2
			));
		})
	}

	#[test]
	fn rejects_justification_with_old_authority_set() {
		run_test(|| {
			let (init_authority_set, init_header, _justification) =
				devnet_header_and_justification_1();
			initialize_with_custom_data(InitializationData {
				authority_list: init_authority_set,
				header: Box::new(init_header),
				operating_mode: BasicOperatingMode::Normal,
			});

			let (_authority_set, header, justification) = devnet_header_and_justification_3();
			assert_noop!(
				Aleph::submit_finality_proof(RuntimeOrigin::signed(1), header, justification),
				Error::<TestRuntime>::InvalidJustification
			);
		})
	}

	#[test]
	fn rejects_old_headers() {
		run_test(|| {
			let (init_authority_set, init_header, _justification) =
				devnet_header_and_justification_2();
			initialize_with_custom_data(InitializationData {
				authority_list: init_authority_set,
				header: Box::new(init_header),
				operating_mode: BasicOperatingMode::Normal,
			});

			let (_authority_set, header, justification) = devnet_header_and_justification_1();
			assert_noop!(
				Aleph::submit_finality_proof(RuntimeOrigin::signed(1), header, justification),
				Error::<TestRuntime>::OldHeader
			);
		})
	}
}
