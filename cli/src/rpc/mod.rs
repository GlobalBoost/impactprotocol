//! A collection of node-specific RPC methods.

use std::sync::Arc;

use jsonrpsee::RpcModule;
// Substrate
use sc_client_api::{
	backend::{Backend, StorageProvider},
	client::BlockchainEvents,
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::ChainApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::Block as BlockT;
// Runtime
use impact_runtime::{opaque::Block};
use impact_primitives::{AccountId, Balance, Index};

mod eth;
pub use self::eth::{create_eth, overrides_handle, EthDeps};

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, CT> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// Ethereum-compatibility specific dependencies.
	pub eth: EthDeps<C, P, A, CT, Block>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, BE, A, CT>(
	deps: FullDeps<C, P, A, CT>,
	subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C: BlockchainEvents<Block> + 'static,
	C: HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ StorageProvider<Block, BE>,
	BE: Backend<Block> + 'static,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,
	CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		deny_unsafe,
		eth,
	} = deps;

	io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
	io.merge(TransactionPayment::new(client).into_rpc())?;

	// Ethereum compatibility RPCs
	let io = create_eth::<_, _, _, _, _, _>(io, eth, subscription_task_executor)?;

	Ok(io)
}
