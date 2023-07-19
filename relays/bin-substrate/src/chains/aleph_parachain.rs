use crate::cli::CliChain;
use relay_aleph_parachain_client::AlephParachain;
use relay_substrate_client::SimpleRuntimeVersion;

impl CliChain for AlephParachain {
	const RUNTIME_VERSION: Option<SimpleRuntimeVersion> = None;
}
