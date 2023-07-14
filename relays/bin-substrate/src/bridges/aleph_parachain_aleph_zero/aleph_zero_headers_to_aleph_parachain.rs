use crate::cli::bridge::{CliBridgeBase, MessagesCliBridge, RelayToRelayHeadersCliBridge};
use substrate_relay_helper::finality::{
	engine::Grandpa as GrandpaFinalityEngine, SubstrateFinalitySyncPipeline,
};

substrate_relay_helper::generate_submit_finality_proof_call_builder!(
	MillauFinalityToRialtoParachain,
	MillauFinalityToRialtoParachainCallBuilder,
	relay_rialto_parachain_client::RuntimeCall::BridgeMillauGrandpa,
	relay_rialto_parachain_client::BridgeGrandpaCall::submit_finality_proof
);

/// Description of Millau -> Rialto finalized headers bridge.
#[derive(Clone, Debug)]
pub struct MillauFinalityToRialtoParachain;

impl SubstrateFinalitySyncPipeline for MillauFinalityToRialtoParachain {
	type SourceChain = relay_millau_client::Millau;
	type TargetChain = relay_rialto_parachain_client::RialtoParachain;

	type FinalityEngine = GrandpaFinalityEngine<Self::SourceChain>;
	type SubmitFinalityProofCallBuilder = MillauFinalityToRialtoParachainCallBuilder;
}

//// `Millau` to `RialtoParachain`  bridge definition.
pub struct MillauToRialtoParachainCliBridge {}

impl CliBridgeBase for MillauToRialtoParachainCliBridge {
	type Source = relay_millau_client::Millau;
	type Target = relay_rialto_parachain_client::RialtoParachain;
}

impl RelayToRelayHeadersCliBridge for MillauToRialtoParachainCliBridge {
	type Finality = MillauFinalityToRialtoParachain;
}

impl MessagesCliBridge for MillauToRialtoParachainCliBridge {
	type MessagesLane =
		crate::bridges::rialto_parachain_millau::millau_messages_to_rialto_parachain::MillauMessagesToRialtoParachain;
}
