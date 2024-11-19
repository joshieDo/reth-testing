#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use alloy_chains::Chain;
use alloy_primitives::BlockHash;
use jsonrpsee::http_client::{transport::HttpBackend, HttpClient};
use reth::{
    api::EngineTypes,
    rpc::{api::EngineApiClient, types::engine::ForkchoiceState},
};
use reth_consensus_debug_client::{block_to_execution_payload_v3, EtherscanBlockProvider};
use reth_rpc_layer::AuthClientService;
use reth_tracing::tracing::warn;

/// A fake consensus layer that advances the chain on-demand by using therscan
pub struct FakeCl {
    // A [`EtherscanBlockProvider`].
    pub etherscan: EtherscanBlockProvider,
}

impl FakeCl {
    /// Creates a [`Self`] from a [`Chain`] and `etherscan_url` if it exists.
    ///
    /// Requires an etherscan api key to be set as an environment variable.
    pub fn new(chain: Chain, etherscan_url: Option<String>) -> Result<Self, eyre::Error> {
        let etherscan_url = etherscan_url.map(Ok).unwrap_or_else(|| {
            chain
                .etherscan_urls()
                .map(|urls| urls.0.to_string())
                .ok_or_else(|| eyre::eyre!("failed to get etherscan url for chain: {chain}"))
        })?;
        let etherscan_api_key = chain.etherscan_api_key().ok_or_else(|| {
            eyre::eyre!("etherscan api key not found for rpc consensus client for chain: {chain}")
        })?;

        Ok(Self { etherscan: EtherscanBlockProvider::new(etherscan_url, etherscan_api_key) })
    }

    /// Advances the chain by querying `etherscan` for a specific block and issues a `newPayload` &
    /// `FCU` request from that.
    pub async fn advance_chain<E: EngineTypes>(
        &mut self,
        auth_client: &HttpClient<AuthClientService<HttpBackend>>,
        block_number: u64,
        finalized_hash: BlockHash,
    ) -> Result<(), eyre::Error> {
        let etherscan_block = self.etherscan.load_block(block_number.into()).await?;

        let payload = block_to_execution_payload_v3(etherscan_block);
        let block_hash = payload.block_hash();

        EngineApiClient::<E>::new_payload_v3(
            auth_client,
            payload.execution_payload_v3,
            payload.versioned_hashes,
            payload.parent_beacon_block_root,
        )
        .await
        .inspect_err(|err|  {
            warn!(target: "exex-consensus", %err, %block_hash,  %block_number, "failed to submit new payload to execution client");
        })?;

        EngineApiClient::<E>::fork_choice_updated_v3(
            auth_client,
            ForkchoiceState {
                head_block_hash: block_hash,
                safe_block_hash: finalized_hash,
                finalized_block_hash: finalized_hash,
            },
            None,
        )
        .await
        .inspect_err(|err| {
            warn!(target: "exex-consensus", %err, "failed to submit fork
                choice update to execution client");
        })?;

        Ok(())
    }
}
