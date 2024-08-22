use jsonrpsee::http_client::{transport::HttpBackend, HttpClient};
use reth::{
    primitives::BlockHash,
    rpc::{api::EngineApiClient, types::engine::ForkchoiceState},
};
use reth_consensus_debug_client::{rich_block_to_execution_payload_v3, EtherscanBlockProvider};
use reth_node_ethereum::EthEngineTypes;
use reth_rpc_layer::AuthClientService;
use reth_tracing::tracing::warn;

/// Advances the chain by querying `etherscan` for a specific block and issues a `newPayload` &
/// `FCU` request from that.
pub async fn advance_chain(
    auth_client: &HttpClient<AuthClientService<HttpBackend>>,
    etherscan: &EtherscanBlockProvider,
    block_number: u64,
    finalized_hash: BlockHash,
) -> Result<(), eyre::Error> {
    let etherscan_block = etherscan.load_block(block_number.into()).await?;

    let payload = rich_block_to_execution_payload_v3(etherscan_block);
    let block_hash = payload.block_hash();

    EngineApiClient::<EthEngineTypes>::new_payload_v3(
        auth_client,
        payload.execution_payload_v3,
        payload.versioned_hashes,
        payload.parent_beacon_block_root,
    )
    .await
    .inspect_err(|err|  {
        warn!(target: "exex-consensus", %err, %block_hash,  %block_number, "failed to submit new payload to execution client");
    })?;

    EngineApiClient::<EthEngineTypes>::fork_choice_updated_v3(
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
