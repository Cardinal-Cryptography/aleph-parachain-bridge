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

//! Millau-to-RialtoParachain headers sync entrypoint.

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

//! Millau-to-AlephParachain headers sync entrypoint.

use crate::cli::bridge::{CliBridgeBase, MessagesCliBridge, RelayToRelayHeadersCliBridge};
use substrate_relay_helper::finality::{
	engine::Grandpa as GrandpaFinalityEngine, SubstrateFinalitySyncPipeline,
};

substrate_relay_helper::generate_submit_finality_proof_call_builder!(
	MillauFinalityToAlephParachain,
	MillauFinalityToAlephParachainCallBuilder,
	relay_aleph_parachain_client::RuntimeCall::BridgeMillauGrandpa,
	relay_aleph_parachain_client::BridgeGrandpaCall::submit_finality_proof
);

/// Description of Millau -> Aleph Parachain finalized headers bridge.
#[derive(Clone, Debug)]
pub struct MillauFinalityToAlephParachain;

impl SubstrateFinalitySyncPipeline for MillauFinalityToAlephParachain {
	type SourceChain = relay_millau_client::Millau;
	type TargetChain = relay_aleph_parachain_client::AlephParachain;

	type FinalityEngine = GrandpaFinalityEngine<Self::SourceChain>;
	type SubmitFinalityProofCallBuilder = MillauFinalityToAlephParachainCallBuilder;
}

//// `Millau` to `AlephParachain`  bridge definition.
pub struct MillauToAlephParachainCliBridge {}

impl CliBridgeBase for MillauToAlephParachainCliBridge {
	type Source = relay_millau_client::Millau;
	type Target = relay_aleph_parachain_client::AlephParachain;
}

impl RelayToRelayHeadersCliBridge for MillauToAlephParachainCliBridge {
	type Finality = MillauFinalityToAlephParachain;
}

impl MessagesCliBridge for MillauToAlephParachainCliBridge {
	type MessagesLane =
		crate::bridges::aleph_parachain_millau::millau_messages_to_aleph_parachain::MillauMessagesToAlephParachain;
}
