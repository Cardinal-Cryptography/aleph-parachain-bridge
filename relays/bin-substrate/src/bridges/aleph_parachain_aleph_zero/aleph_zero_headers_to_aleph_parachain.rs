use crate::cli::bridge::{CliBridgeBase, RelayToRelayHeadersCliBridge};
use relay_substrate_client::{CallOf, HeaderOf, SyncHeader};
use substrate_relay_helper::finality::{
	engine::AlephEngine, SubmitFinalityProofCallBuilder, SubstrateFinalitySyncPipeline,
};

use bp_aleph_header_chain::aleph_justification::AlephJustificationWithTarget;

pub struct AlephZeroFinalityToAlephParachainCallBuilder;
impl SubmitFinalityProofCallBuilder<AlephZeroFinalityToAlephParachain>
	for AlephZeroFinalityToAlephParachainCallBuilder
{
	fn build_submit_finality_proof_call(
		header: SyncHeader<
			HeaderOf<
				<AlephZeroFinalityToAlephParachain as SubstrateFinalitySyncPipeline>::SourceChain,
			>,
		>,
		proof: AlephJustificationWithTarget<
			HeaderOf<
				<AlephZeroFinalityToAlephParachain as SubstrateFinalitySyncPipeline>::SourceChain,
			>,
		>,
	) -> CallOf<<AlephZeroFinalityToAlephParachain as SubstrateFinalitySyncPipeline>::TargetChain> {
		relay_aleph_parachain_client::RuntimeCall::BridgeAleph(
			relay_aleph_parachain_client::BridgeAlephCall::submit_finality_proof {
				header: header.into_inner(),
				justification: proof.into(),
			},
		)
	}
}

/// Description of AlephZero -> Aleph Parachain finalized headers bridge.
#[derive(Clone, Debug)]
pub struct AlephZeroFinalityToAlephParachain;

impl SubstrateFinalitySyncPipeline for AlephZeroFinalityToAlephParachain {
	type SourceChain = relay_aleph_zero_client::AlephZero;
	type TargetChain = relay_aleph_parachain_client::AlephParachain;

	type FinalityEngine = AlephEngine<Self::SourceChain>;
	type SubmitFinalityProofCallBuilder = AlephZeroFinalityToAlephParachainCallBuilder;
}

//// `AlephZero` to `AlephParachain`  bridge definition.
pub struct AlephZeroToAlephParachainCliBridge {}

impl CliBridgeBase for AlephZeroToAlephParachainCliBridge {
	type Source = relay_aleph_zero_client::AlephZero;
	type Target = relay_aleph_parachain_client::AlephParachain;
}

impl RelayToRelayHeadersCliBridge for AlephZeroToAlephParachainCliBridge {
	type Finality = AlephZeroFinalityToAlephParachain;
}
