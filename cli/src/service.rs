// This file is part of Impact Protocol.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Service implementation. Specialized wrapper over substrate service.
use async_trait::async_trait;
use codec::Encode;
use futures::prelude::*;
use impact_primitives::Block;
use sc_client_api::{BlockBackend, StateBackendFor};
use sc_consensus::BasicQueue;
use sc_executor::{NativeElseWasmExecutor, NativeExecutionDispatch};
use grandpa::{SharedVoterState, GrandpaBlockImport};
use sc_network::{event::Event, NetworkEventStream};
use sc_network_common::sync::warp::WarpSyncParams;
use sc_service::{config::Configuration, error::Error as ServiceError, PartialComponents, TaskManager};
use impact_consensus_pow::*;
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_api::ConstructRuntimeApi;
use log::*;

use sp_core::{
	crypto::{Ss58AddressFormat, Ss58Codec, UncheckedFrom},
	Pair, U256, H256,
};

use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::traits::{Block as BlockT, BlakeTwo256};
use std::{sync::Arc, time::Duration};
use std::thread;
use std::path::PathBuf;
use std::str::FromStr;
use sp_trie::PrefixedMemoryDB;

// Runtime
use impact_runtime::TransactionConverter;

use crate::{
	client::{FullBackend, FullClient, RuntimeApiCollection},
	eth::{
		new_frontier_partial, spawn_frontier_tasks, FrontierBackend, FrontierBlockImport,
		FrontierPartialComponents,
	},
};

pub use crate::{
	client::{Client, TemplateRuntimeExecutor},
	eth::{db_config_dir, EthConfiguration},
};


#[allow(missing_docs)]
pub fn decode_author(
	author: Option<&str>,
	keystore: SyncCryptoStorePtr,
	keystore_path: Option<PathBuf>,
) -> Result<impact_consensus_pow::app::Public, String> {
	if let Some(author) = author {
		if author.starts_with("0x") {
			Ok(impact_consensus_pow::app::Public::unchecked_from(
				H256::from_str(&author[2..]).map_err(|_| "Invalid author account".to_string())?,
			)
			.into())
		} else {
			let (address, version) = impact_consensus_pow::app::Public::from_ss58check_with_version(author)
				.map_err(|_| "Invalid author address".to_string())?;
			if version != Ss58AddressFormat::custom(45){
				return Err("Invalid author version".to_string());
			}
			Ok(address)
		}
	} else {
		info!("The node is configured for mining, but no author key is provided.");

		let (pair, phrase, _) = impact_consensus_pow::app::Pair::generate_with_phrase(None);

		SyncCryptoStore::insert_unknown(
			&*keystore.as_ref(),
			impact_consensus_pow::app::ID,
			&phrase,
			pair.public().as_ref(),
		)
		.map_err(|e| format!("Registering mining key failed: {:?}", e))?;

		info!(
			"Generated a mining key with address: {}",
			pair.public()
				.to_ss58check_with_version(Ss58AddressFormat::custom(45))
		);

		match keystore_path {
			Some(path) => info!("You can go to {:?} to find the seed phrase of the mining key.", path),
			None => warn!("Keystore is not local. This means that your mining key will be lost when exiting the program. This should only happen if you are in dev mode."),
		}

		Ok(pair.public())
	}
}

// Our native executor instance.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	/// Only enable the benchmarking host functions when we actually want to benchmark.
	#[cfg(feature = "runtime-benchmarks")]
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
	/// Otherwise we only use the default Substrate host functions.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		impact_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		impact_runtime::native_version()
	}
}

type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

type FullGrandpaBlockImport<Client> =
	GrandpaBlockImport<FullBackend, Block, Client, FullSelectChain>;

/// The transaction pool type defintion.
pub type FullPool<Client> = sc_transaction_pool::FullPool<Block, Client>;

/// Generate the inherent data providers expected by the runtime
pub struct CreateInherentDataProviders;

#[async_trait]
impl sp_inherents::CreateInherentDataProviders<Block, ()> for CreateInherentDataProviders {
	type InherentDataProviders = sp_timestamp::InherentDataProvider;

	async fn create_inherent_data_providers(
		&self,
		_parent: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		Ok(sp_timestamp::InherentDataProvider::from_system_time())
	}
}

/// Creates a new partial node.
pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
	_eth_config: &EthConfiguration,
) -> Result<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		FullPool<FullClient<RuntimeApi, Executor>>,
		(
			impl Fn(
				impact_rpc::DenyUnsafe,
				sc_rpc::SubscriptionTaskExecutor,
			) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>,
			(
				sc_consensus_pow::PowBlockImport<
				Block,
				FullGrandpaBlockImport<FullClient<RuntimeApi, Executor>>,
				FullClient<RuntimeApi, Executor>,
				FullSelectChain,
				impact_consensus_pow::YescryptAlgorithm<FullClient<RuntimeApi, Executor>>,
				CreateInherentDataProviders,
				>,
				grandpa::LinkHalf<Block, FullClient<RuntimeApi, Executor>, FullSelectChain>,
			),
			SharedVoterState,
			Option<Telemetry>,
			Arc<FrontierBackend>,
		),
	>,
	ServiceError,
> 
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		&client.clone(),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let frontier_backend = Arc::new(FrontierBackend::open(
		client.clone(),
		&config.database,
		&db_config_dir(config),
	)?);

	let frontier_block_import = FrontierBlockImport::new(
		grandpa_block_import.clone(),
		client.clone(),
		frontier_backend.clone(),
	);

	let algorithm = impact_consensus_pow::YescryptAlgorithm::new(client.clone());

	let pow_block_import = sc_consensus_pow::PowBlockImport::new(
		grandpa_block_import.clone(),
		client.clone(),
		algorithm.clone(),
		0, // check inherents starting at block 0
		select_chain.clone(),
		CreateInherentDataProviders,
	);

	let import_queue = sc_consensus_pow::import_queue(
		Box::new(frontier_block_import.clone()),
		Some(Box::new(grandpa_block_import)),
		algorithm.clone(),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),	
	)?;

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let import_setup = (pow_block_import, grandpa_link);

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();

		let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_backend = backend.clone();
		let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
			let deps = impact_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				grandpa: impact_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
			};

			impact_rpc::create_full(deps, rpc_backend.clone()).map_err(Into::into)
		};

		(rpc_extensions_builder, shared_voter_state2)
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (rpc_extensions_builder, import_setup, rpc_setup, telemetry, frontier_backend),
	})
}


/// Creates a full service from the configuration.
pub fn new_full_base<RuntimeApi, Executor>(
	mut config: Configuration,
	eth_config: EthConfiguration,
	author: Option<&str>,
) -> Result<TaskManager, ServiceError>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{

	let PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (_rpc_builder, import_setup, rpc_setup, mut telemetry, frontier_backend),
	} = new_partial::<RuntimeApi, Executor>(&config, &eth_config)?;

	let FrontierPartialComponents {
		filter_pool,
		fee_history_cache,
		fee_history_cache_limit,
	} = new_frontier_partial(&eth_config)?;

	let shared_voter_state = rpc_setup;
	
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	
	let grandpa_protocol_name = grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	config
		.network
		.extra_sets
		.push(grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone()));
	let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: Some(WarpSyncParams::WithProvider(warp_sync)),
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let role = config.role.clone();
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	let keystore_path = config.keystore.path().map(|p| p.to_owned());
	
	// for ethereum-compatibility rpc.
	config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));
	let overrides = crate::rpc::overrides_handle(client.clone());
	let eth_rpc_params = crate::rpc::EthDeps {
		client: client.clone(),
		pool: transaction_pool.clone(),
		graph: transaction_pool.pool().clone(),
		converter: Some(TransactionConverter),
		is_authority: config.role.is_authority(),
		enable_dev_signer: eth_config.enable_dev_signer,
		network: network.clone(),
		sync: sync_service.clone(),
		frontier_backend: frontier_backend.clone(),
		overrides: overrides.clone(),
		block_data_cache: Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			overrides.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		)),
		filter_pool: filter_pool.clone(),
		max_past_logs: eth_config.max_past_logs,
		fee_history_cache: fee_history_cache.clone(),
		fee_history_cache_limit,
		execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
	};

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, subscription_task_executor| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				eth: eth_rpc_params.clone(),
			};

			crate::rpc::create_full(deps, subscription_task_executor).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_builder: Box::new(rpc_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
	})?;

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
	);


	let (pow_block_import, grandpa_link) = import_setup;

	if let sc_service::config::Role::Authority { .. } = &role {
		
		let author = decode_author(author, keystore_container.sync_keystore(), keystore_path)?;
		let algorithm = impact_consensus_pow::YescryptAlgorithm::new(client.clone());

		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let (_worker, worker_task) = sc_consensus_pow::start_mining_worker(
			Box::new(pow_block_import.clone()),
			client.clone(),
			select_chain.clone(),
			algorithm,
			proposer,
			sync_service.clone(),
			sync_service.clone(),
			Some(author.encode()),
			CreateInherentDataProviders,
			Duration::new(10, 0),
			Duration::new(10, 0),
		);	

		task_manager
		.spawn_essential_handle()
		.spawn_blocking(
			"pow-proposer",
			Some("block-authoring"),
			worker_task,
		);

		// Start Mining
		let mut nonce: U256 = U256::from(0);
		thread::spawn(move || loop {
			let worker = _worker.clone();
			let metadata = worker.metadata();
			if let Some(metadata) = metadata {
				let compute = Compute {
					difficulty: metadata.difficulty,
					pre_hash: metadata.pre_hash,
					nonce,
				};
				let seal = compute.compute();
				if hash_meets_difficulty(&seal.work, seal.difficulty) {
					nonce = U256::from(0);
					let worker = worker;
						
					let _ = futures::executor::block_on(worker.submit(seal.encode()));
					
					
				} else {
					nonce = nonce.saturating_add(U256::from(1));
					if nonce == U256::MAX {
						nonce = U256::from(0);
					}
				}
			} else {
				thread::sleep(Duration::new(1, 0));
			}
		});		

	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network.event_stream("authority-discovery").filter_map(|e| async move {
				match e {
					Event::Dht(e) => Some(e),
					_ => None,
				}
			});
		let (authority_discovery_worker, _service) =
			sc_authority_discovery::new_worker_and_service_with_config(
				sc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					..Default::default()
				},
				client.clone(),
				network.clone(),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore =
		if role.is_authority() { Some(keystore_container.sync_keystore()) } else { None };

	let config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = grandpa::GrandpaParams {
			config,
			link: grandpa_link,
			network: network.clone(),
			sync: Arc::new(sync_service.clone()),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	eth_config: EthConfiguration,
	author: Option<&str>,
) -> Result<TaskManager, ServiceError> {
	new_full_base::<impact_runtime::RuntimeApi, TemplateRuntimeExecutor>(config, eth_config, author)
}

pub fn new_chain_ops(
	mut config: &mut Configuration,
	eth_config: &EthConfiguration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
		Arc<FrontierBackend>,
	),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let PartialComponents {
		client,
		backend,
		import_queue,
		task_manager,
		other,
		..
	} = new_partial::<impact_runtime::RuntimeApi, TemplateRuntimeExecutor>(
		config,
		eth_config,
	)?;
	Ok((client, backend, import_queue, task_manager, other.4))
}