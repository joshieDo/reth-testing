use reth::api::FullNodeComponents;
use reth_consensus_debug_client::EtherscanBlockProvider;
use reth_exex::ExExContext;

pub fn etherscan_provider<Node: FullNodeComponents>(
    ctx: &ExExContext<Node>,
    etherscan: Option<String>,
) -> Result<EtherscanBlockProvider, eyre::Error> {
    let chain = ctx.config.chain.chain;
    let etherscan_url = etherscan.map(Ok).unwrap_or_else(|| {
        chain
            .etherscan_urls()
            .map(|urls| urls.0.to_string())
            .ok_or_else(|| eyre::eyre!("failed to get etherscan url for chain: {chain}"))
    })?;
    let etherscan_api_key = chain.etherscan_api_key().ok_or_else(|| {
        eyre::eyre!("etherscan api key not found for rpc consensus client for chain: {chain}")
    })?;
    Ok(EtherscanBlockProvider::new(etherscan_url, etherscan_api_key))
}
