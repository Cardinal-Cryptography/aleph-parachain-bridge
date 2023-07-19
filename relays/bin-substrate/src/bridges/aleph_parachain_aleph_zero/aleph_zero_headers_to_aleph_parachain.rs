use crate::cli::bridge::{CliBridgeBase, RelayToRelayHeadersCliBridge};
use substrate_relay_helper::finality::{engine::AlephEngine, SubstrateFinalitySyncPipeline};

pub struct AlephZeroFinalityToAlephParachainCallBuilder;
impl
	::substrate_relay_helper::finality::SubmitFinalityProofCallBuilder<
		AlephZeroFinalityToAlephParachain,
	> for AlephZeroFinalityToAlephParachainCallBuilder
{
	fn build_submit_finality_proof_call(
                    header: relay_substrate_client::SyncHeader<
                        relay_substrate_client::HeaderOf<
                            <AlephZeroFinalityToAlephParachain as ::substrate_relay_helper::finality::SubstrateFinalitySyncPipeline>::SourceChain,
                        >,
                    >,
                    proof: bp_aleph_header_chain::aleph_justification::AlephFullJustification<
                        relay_substrate_client::HeaderOf<
                            <AlephZeroFinalityToAlephParachain as ::substrate_relay_helper::finality::SubstrateFinalitySyncPipeline>::SourceChain,
                        >,
                    >,
                ) -> relay_substrate_client::CallOf<
                    <AlephZeroFinalityToAlephParachain as ::substrate_relay_helper::finality::SubstrateFinalitySyncPipeline>::TargetChain,
	>{
		log::debug!("AlephZeroFinalityToAlephParachainCallBuilder::build_submit_finality_proof_call for header: {:?} and proof: {:?}", header, proof);
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
