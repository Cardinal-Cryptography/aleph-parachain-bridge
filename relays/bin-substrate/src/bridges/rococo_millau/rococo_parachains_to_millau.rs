// Copyright 2019-2021 Parity Technologies (UK) Ltd.
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

//! Rococo-to-Millau parachains sync entrypoint.

use crate::cli::bridge::{CliBridgeBase, ParachainToRelayHeadersCliBridge};
use relay_millau_client::Millau;
use relay_rococo_client::Rococo;
use relay_aleph_parachain_client::AlephParachain;
use substrate_relay_helper::parachains::{
	DirectSubmitParachainHeadsCallBuilder, SubstrateParachainsPipeline,
};

/// Rococo-to-Millau parachains sync description.
#[derive(Clone, Debug)]
pub struct RococoParachainsToMillau;

impl SubstrateParachainsPipeline for RococoParachainsToMillau {
	type SourceParachain = AlephParachain;
	type SourceRelayChain = Rococo;
	type TargetChain = Millau;

	type SubmitParachainHeadsCallBuilder = RococoParachainsToMillauSubmitParachainHeadsCallBuilder;
}

/// `submit_parachain_heads` call builder for Rococo-to-Millau parachains sync pipeline.
pub type RococoParachainsToMillauSubmitParachainHeadsCallBuilder =
	DirectSubmitParachainHeadsCallBuilder<
		RococoParachainsToMillau,
		millau_runtime::Runtime,
		millau_runtime::WithRococoParachainsInstance,
	>;

//// `AlephParachain` to `Millau` bridge definition.
pub struct AlephParachainToMillauCliBridge {}

impl ParachainToRelayHeadersCliBridge for AlephParachainToMillauCliBridge {
	type SourceRelay = Rococo;
	type ParachainFinality = RococoParachainsToMillau;
	type RelayFinality =
		crate::bridges::rococo_millau::rococo_headers_to_millau::RococoFinalityToMillau;
}

impl CliBridgeBase for AlephParachainToMillauCliBridge {
	type Source = AlephParachain;
	type Target = Millau;
}
