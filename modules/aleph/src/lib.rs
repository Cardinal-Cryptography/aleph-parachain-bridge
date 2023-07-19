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

use bp_aleph_header_chain::{ChainWithAleph, InitializationData};
use bp_header_chain::{HeaderChain, StoredHeaderData, StoredHeaderDataBuilder};
use bp_runtime::{BlockNumberOf, HashOf, HeaderId, HeaderOf, OwnedBridgeModule};
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
		/// Bootstrap the bridge pallet with an initial header and authority set from which to sync.
		///
		/// The initial configuration provided does not need to be the genesis header of the bridged
		/// chain, it can be any arbitrary header.
		///
		/// This function is only allowed to be called by a root origin and writes to storage
		/// with practically no checks in terms of the validity of the data. It is important that
		/// you ensure that valid data is being passed in.
		///
		/// Difference with GRANDPA: This function can only be called by root.
		///
		/// Note: It cannot be called once the bridge has been initialized.
		/// To reinitialize the bridge, you must reinitialize the pallet.
		#[pallet::call_index(1)]
		#[pallet::weight((T::DbWeight::get().reads_writes(3, 7), DispatchClass::Operational))]
		pub fn initialize(
			origin: OriginFor<T>,
			init_data: super::InitializationData<BridgedHeader<T>>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			// Ensure that the bridge has not been already initialized.
			let init_allowed = !<BestFinalized<T>>::exists();
			ensure!(init_allowed, <Error<T>>::AlreadyInitialized);

			// Initialize the bridge.
			Self::initialize_bridge(init_data.clone())?;

			log::info!(
				target: LOG_TARGET,
				"Pallet has been initialized with the following parameters: {:?}",
				init_data
			);

			Ok(().into())
		}

		/// Change `PalletOwner`.
		///
		/// May only be called either by root or current `PalletOwner`.
		#[pallet::call_index(2)]
		#[pallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_owner(origin: OriginFor<T>, new_owner: Option<T::AccountId>) -> DispatchResult {
			<Self as OwnedBridgeModule<_>>::set_owner(origin, new_owner)
		}

		/// Halt or resume all pallet operations.
		///
		/// May only be called either by root, or by `PalletOwner`.
		#[pallet::call_index(3)]
		#[pallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_operating_mode(
			origin: OriginFor<T>,
			operating_mode: BasicOperatingMode,
		) -> DispatchResult {
			<Self as OwnedBridgeModule<_>>::set_operating_mode(origin, operating_mode)
		}
	}

	/// Hash and number of the best finalized header.
	#[pallet::storage]
	#[pallet::getter(fn best_finalized)]
	pub type BestFinalized<T: Config> = StorageValue<_, BridgedBlockId<T>, OptionQuery>;

	/// A ring buffer of imported hashes. Ordered by insertion time.
	#[pallet::storage]
	pub(super) type ImportedHashes<T: Config> = StorageMap<
		Hasher = Identity,
		Key = u32,
		Value = BridgedBlockHash<T>,
		MaxValues = HeadersToKeepOption<T>,
	>;

	/// Current ring buffer position.
	#[pallet::storage]
	pub(super) type ImportedHashesPointer<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// A ring buffer for relevant fields of imported headers.
	#[pallet::storage]
	pub type ImportedHeaders<T: Config> = StorageMap<
		Hasher = Identity,
		Key = BridgedBlockHash<T>,
		Value = BridgedStoredHeaderData<T>,
		MaxValues = HeadersToKeepOption<T>,
	>;

	/// The current Aleph Authority set.
	#[pallet::storage]
	pub type CurrentAuthoritySet<T: Config> = StorageValue<_, StoredAuthoritySet<T>, ValueQuery>;

	/// Optional pallet owner.
	///
	/// Pallet owner has the right to halt all pallet operations and then resume them.
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
	pub enum Event<T: Config> {
		/// Best finalized chain header has been updated to the header with given number and hash.
		UpdatedBestFinalizedHeader { number: BridgedBlockNumber<T>, hash: BridgedBlockHash<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The header being imported is older than the best finalized header known to the pallet.
		OldHeader,
		/// The pallet is not yet initialized.
		NotInitialized,
		/// The pallet has already been initialized.
		AlreadyInitialized,
		/// Too many authorities in the set.
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
			let header_to_prune = <ImportedHashes<T>>::try_get(index).ok();
			<BestFinalized<T>>::put(HeaderId(*header.number(), hash));
			<ImportedHeaders<T>>::insert(hash, header.build());
			<ImportedHashes<T>>::insert(index, hash);

			// Update ring buffer pointer and remove old header.
			<ImportedHashesPointer<T>>::put((index + 1) % T::HeadersToKeep::get());
			if let Some(pruned_header_hash) = header_to_prune {
				log::debug!(target: LOG_TARGET, "Pruning old header: {:?}.", pruned_header_hash);
				<ImportedHeaders<T>>::remove(pruned_header_hash);
			}
		}

		/// Since this writes to storage with no real checks this should only be used in functions
		/// that were called by a trusted origin.
		fn initialize_bridge(
			init_params: super::InitializationData<BridgedHeader<T>>,
		) -> Result<(), Error<T>> {
			let super::InitializationData { header, authority_list, operating_mode } = init_params;
			let authority_set_length = authority_list.len();
			let authority_set = StoredAuthoritySet::<T>::try_new(authority_list)
			.map_err(|err| {
				log::error!(
					target: LOG_TARGET,
					"Failed to initialize bridge. Number of authorities in the set {} is larger than the configured value {}",
					authority_set_length,
					T::BridgedChain::MAX_AUTHORITIES_COUNT,
				);
				err
			})?;

			<ImportedHashesPointer<T>>::put(0);
			Self::insert_header(*header);

			<CurrentAuthoritySet<T>>::put(authority_set);
			<PalletOperatingMode<T>>::put(operating_mode);

			Ok(())
		}
	}

	/// Adapter for using `Config::HeadersToKeep` as `MaxValues` bound in our storage maps.
	// We need to use it since `StorageMap` implementation expects Get<Option<u32>> for `MaxValues`.
	pub struct HeadersToKeepOption<T>(PhantomData<T>);

	impl<T: Config> Get<Option<u32>> for HeadersToKeepOption<T> {
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{
		run_test, test_header, Aleph, RuntimeOrigin, System, TestHeader, TestRuntime,
	};
	use bp_aleph_header_chain::AuthorityId;
	use bp_runtime::{BasicOperatingMode, UnverifiedStorageProof};
	use bp_test_utils::{generate_owned_bridge_module_tests, Account, ALICE, BOB, CHARLIE};
	use frame_support::{
		assert_noop, assert_ok, dispatch::PostDispatchInfo, storage::generator::StorageValue,
	};
	use sp_runtime::DispatchError;

	fn into_authority_set(accounts: Vec<Account>) -> Vec<AuthorityId> {
		accounts.into_iter().map(|a| AuthorityId::from(a)).collect()
	}

	fn initialize_substrate_bridge() {
		System::set_block_number(1);
		System::reset_events();

		assert_ok!(init_with_origin(RuntimeOrigin::root()));
	}

	fn init_with_origin(
		origin: RuntimeOrigin,
	) -> Result<
		InitializationData<TestHeader>,
		sp_runtime::DispatchErrorWithPostInfo<PostDispatchInfo>,
	> {
		let genesis = test_header(0);

		let init_data = InitializationData {
			header: Box::new(genesis),
			authority_list: into_authority_set(vec![ALICE, BOB, CHARLIE]),
			operating_mode: BasicOperatingMode::Normal,
		};

		Aleph::initialize(origin, init_data.clone()).map(|_| init_data)
	}

	generate_owned_bridge_module_tests!(BasicOperatingMode::Normal, BasicOperatingMode::Halted);

	#[test]
	fn init_root_origin_can_initialize_pallet() {
		run_test(|| {
			assert_ok!(init_with_origin(RuntimeOrigin::root()));
		})
	}

	#[test]
	fn init_normal_user_cannot_initialize_pallet() {
		run_test(|| {
			assert_noop!(init_with_origin(RuntimeOrigin::signed(1)), DispatchError::BadOrigin);
		})
	}

	#[test]
	fn init_owner_cannot_initialize_pallet() {
		run_test(|| {
			PalletOwner::<TestRuntime>::put(2);
			assert_noop!(init_with_origin(RuntimeOrigin::signed(2)), DispatchError::BadOrigin);
		})
	}

	#[test]
	fn init_storage_entries_are_correctly_initialized() {
		run_test(|| {
			assert_eq!(BestFinalized::<TestRuntime>::get(), None,);
			assert_eq!(Aleph::best_finalized(), None);
			assert_eq!(PalletOperatingMode::<TestRuntime>::try_get(), Err(()));

			let init_data = init_with_origin(RuntimeOrigin::root()).unwrap();

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
				init_with_origin(RuntimeOrigin::root()),
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
					(0..(<<TestRuntime as Config>::BridgedChain as ChainWithAleph>::MAX_AUTHORITIES_COUNT as u16) + 1).map(|x| Account(x)).collect(),
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
			let headers_to_keep = <TestRuntime as Config>::HeadersToKeep::get();

			for i in 0..2 * headers_to_keep {
				let header = test_header(i as u64);
				Aleph::insert_header(header.clone());
			}

			assert_eq!(ImportedHeaders::<TestRuntime>::iter().count(), headers_to_keep as usize);

			for i in 0..headers_to_keep {
				let header = test_header(i as u64);
				assert!(!<ImportedHeaders<TestRuntime>>::contains_key(header.hash()));
			}

			for i in headers_to_keep..2 * headers_to_keep {
				let header = test_header(i as u64);
				let hash = header.hash();
				assert!(<ImportedHeaders<TestRuntime>>::contains_key(header.hash()));
				assert_eq!(<ImportedHeaders<TestRuntime>>::get(hash), Some(header.build()));
			}
		})
	}

	#[test]
	fn insert_header_best_finalized() {
		run_test(|| {
			initialize_substrate_bridge();

			let header_1 = test_header(1);
			Aleph::insert_header(header_1.clone());

			let header_3 = test_header(3);
			Aleph::insert_header(header_3.clone());

			assert_eq!(Aleph::best_finalized_number(), Some(3));
		})
	}
}
