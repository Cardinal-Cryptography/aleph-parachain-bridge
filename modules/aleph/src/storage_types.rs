// Copyright 2023 Cardinal Cryptography
// Copyright 2022 Parity Technologies (UK) Ltd.
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

//! Wrappers for public types that are implementing `MaxEncodedLen`

use crate::{Config, Error};

use bp_aleph_header_chain::{AuthorityId, AuthoritySet, ChainWithAleph};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec, RuntimeDebugNoBound};
use scale_info::TypeInfo;

use sp_std::marker::PhantomData;

pub struct MaxAuthoritiesCount<T: Config>(PhantomData<T>);

impl<T: Config> Get<u32> for MaxAuthoritiesCount<T> {
	fn get() -> u32 {
		T::BridgedChain::MAX_AUTHORITIES_COUNT
	}
}

/// Bounded AlephBFT Authority Set.
#[derive(Clone, Decode, Encode, TypeInfo, MaxEncodedLen, RuntimeDebugNoBound)]
#[scale_info(skip_type_params(T))]
pub struct StoredAuthoritySet<T: Config> {
	/// List of AlephBFT authorities for the current session.
	pub authorities: BoundedVec<AuthorityId, MaxAuthoritiesCount<T>>,
}

impl<T: Config> StoredAuthoritySet<T> {
	/// Try to create a new bounded AlephBFT Authority Set from unbounded list.
	///
	/// Returns error if number of authorities in the provided list is too large.
	pub fn try_new(authorities: AuthoritySet) -> Result<Self, Error<T>> {
		Ok(Self {
			authorities: TryFrom::try_from(authorities)
				.map_err(|_| Error::TooManyAuthoritiesInSet)?,
		})
	}
}

impl<T: Config> Default for StoredAuthoritySet<T> {
	fn default() -> Self {
		Self { authorities: BoundedVec::default() }
	}
}

impl<T: Config> From<StoredAuthoritySet<T>> for AuthoritySet {
	fn from(t: StoredAuthoritySet<T>) -> Self {
		t.authorities.into()
	}
}
