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

//! Everything required to serve Millau <-> AlephParachain messages.

use crate::{
	Runtime, RuntimeOrigin, WithAlephParachainMessagesInstance, WithRococoParachainsInstance,
};

use bp_messages::LaneId;
use bridge_runtime_common::{
	messages::{
		self, source::TargetHeaderChainAdapter, target::SourceHeaderChainAdapter, MessageBridge,
	},
	messages_xcm_extension::{XcmBlobHauler, XcmBlobHaulerAdapter},
};
use frame_support::{parameter_types, weights::Weight, RuntimeDebug};
use xcm::latest::prelude::*;
use xcm_builder::HaulBlobExporter;

/// Default lane that is used to send messages to Aleph parachain.
pub const XCM_LANE: LaneId = LaneId([0, 0, 0, 0]);
/// Weight of 2 XCM instructions is for simple `Trap(42)` program, coming through bridge
/// (it is prepended with `UniversalOrigin` instruction). It is used just for simplest manual
/// tests, confirming that we don't break encoding somewhere between.
pub const BASE_XCM_WEIGHT_TWICE: Weight = crate::xcm_config::BaseXcmWeight::get().saturating_mul(2);

parameter_types! {
	/// Weight credit for our test messages.
	///
	/// 2 XCM instructions is for simple `Trap(42)` program, coming through bridge
	/// (it is prepended with `UniversalOrigin` instruction).
	pub const WeightCredit: Weight = BASE_XCM_WEIGHT_TWICE;
}

/// Message payload for Millau -> AlephParachain messages.
pub type ToAlephParachainMessagePayload = messages::source::FromThisChainMessagePayload;

/// Message verifier for Millau -> AlephParachain messages.
pub type ToAlephParachainMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithAlephParachainMessageBridge>;

/// Message payload for AlephParachain -> Millau messages.
pub type FromAlephParachainMessagePayload = messages::target::FromBridgedChainMessagePayload;

/// Call-dispatch based message dispatch for AlephParachain -> Millau messages.
pub type FromAlephParachainMessageDispatch =
	bridge_runtime_common::messages_xcm_extension::XcmBlobMessageDispatch<
		bp_millau::Millau,
		bp_rococo::Rococo,
		crate::xcm_config::OnMillauBlobDispatcher,
		(),
	>;

/// Maximal outbound payload size of Millau -> AlephParachain messages.
pub type ToAlephParachainMaximalOutboundPayloadSize =
	messages::source::FromThisChainMaximalOutboundPayloadSize<WithAlephParachainMessageBridge>;

/// Millau <-> AlephParachain message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithAlephParachainMessageBridge;

impl MessageBridge for WithAlephParachainMessageBridge {
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = bp_millau::WITH_MILLAU_MESSAGES_PALLET_NAME;

	type ThisChain = Millau;
	type BridgedChain = AlephParachain;
	type BridgedHeaderChain = pallet_bridge_parachains::ParachainHeaders<
		Runtime,
		WithRococoParachainsInstance,
		bp_aleph_parachain::AlephParachain,
	>;
}

/// Millau chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Millau;

impl messages::UnderlyingChainProvider for Millau {
	type Chain = bp_millau::Millau;
}

impl messages::ThisChainWithMessages for Millau {
	type RuntimeOrigin = RuntimeOrigin;
}

/// AlephParachain chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct AlephParachain;
/// AlephParachain as source header chain.
pub type AlephParachainAsSourceHeaderChain =
	SourceHeaderChainAdapter<WithAlephParachainMessageBridge>;
/// AlephParachain as target header chain.
pub type AlephParachainAsTargetHeaderChain =
	TargetHeaderChainAdapter<WithAlephParachainMessageBridge>;

impl messages::UnderlyingChainProvider for AlephParachain {
	type Chain = bp_aleph_parachain::AlephParachain;
}

impl messages::BridgedChainWithMessages for AlephParachain {}

/// Export XCM messages to be relayed to Rialto.
pub type ToAlephParachainBlobExporter = HaulBlobExporter<
	XcmBlobHaulerAdapter<ToAlephParachainXcmBlobHauler>,
	crate::xcm_config::AlephParachainNetwork,
	(),
>;

/// To-AlephParachain XCM hauler.
pub struct ToAlephParachainXcmBlobHauler;

impl XcmBlobHauler for ToAlephParachainXcmBlobHauler {
	type MessageSender =
		pallet_bridge_messages::Pallet<Runtime, WithAlephParachainMessagesInstance>;
	type MessageSenderOrigin = RuntimeOrigin;

	fn message_sender_origin() -> RuntimeOrigin {
		pallet_xcm::Origin::from(MultiLocation::new(1, crate::xcm_config::UniversalLocation::get()))
			.into()
	}

	fn xcm_lane() -> LaneId {
		XCM_LANE
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		PriorityBoostPerMessage, RococoGrandpaInstance, Runtime, WithAlephParachainMessagesInstance,
	};

	use bridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_bridge_constants, check_message_lane_weights,
			AssertBridgeMessagesPalletConstants, AssertBridgePalletNames, AssertChainConstants,
			AssertCompleteBridgeConstants,
		},
	};

	#[test]
	fn ensure_millau_message_lane_weights_are_correct() {
		check_message_lane_weights::<bp_millau::Millau, Runtime>(
			bp_aleph_parachain::EXTRA_STORAGE_PROOF_SIZE,
			bp_millau::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			bp_millau::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_grandpa_instance: RococoGrandpaInstance,
			with_bridged_chain_messages_instance: WithAlephParachainMessagesInstance,
			bridge: WithAlephParachainMessageBridge,
			this_chain: bp_millau::Millau,
			bridged_chain: bp_rococo::Rococo,
		);

		assert_complete_bridge_constants::<
			Runtime,
			RococoGrandpaInstance,
			WithAlephParachainMessagesInstance,
			WithAlephParachainMessageBridge,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: bp_millau::BlockLength::get(),
				block_weights: bp_millau::BlockWeights::get(),
			},
			messages_pallet_constants: AssertBridgeMessagesPalletConstants {
				max_unrewarded_relayers_in_bridged_confirmation_tx:
					bp_aleph_parachain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
				max_unconfirmed_messages_in_bridged_confirmation_tx:
					bp_aleph_parachain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
				bridged_chain_id: bp_runtime::ALEPH_PARACHAIN_CHAIN_ID,
			},
			pallet_names: AssertBridgePalletNames {
				with_this_chain_messages_pallet_name: bp_millau::WITH_MILLAU_MESSAGES_PALLET_NAME,
				with_bridged_chain_grandpa_pallet_name: bp_rococo::WITH_ROCOCO_GRANDPA_PALLET_NAME,
				with_bridged_chain_messages_pallet_name:
					bp_aleph_parachain::WITH_ALEPH_PARACHAIN_MESSAGES_PALLET_NAME,
			},
		});

		bridge_runtime_common::priority_calculator::ensure_priority_boost_is_sane::<
			Runtime,
			WithAlephParachainMessagesInstance,
			PriorityBoostPerMessage,
		>(1_000_000);
	}
}
