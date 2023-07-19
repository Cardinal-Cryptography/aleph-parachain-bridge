use crate::cli::CliChain;
use relay_aleph_zero_client::AlephZero;
use relay_substrate_client::SimpleRuntimeVersion;

impl CliChain for AlephZero {
	const RUNTIME_VERSION: Option<SimpleRuntimeVersion> = None;
}
