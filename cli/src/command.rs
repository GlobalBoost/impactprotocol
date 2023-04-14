// This file is part of Impact Protocol.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service::{self, db_config_dir},
};

use sc_service::DatabaseSource;

// Frontier
use fc_db::frontier_database_dir;

use impact_runtime::RuntimeApi;
use impact_primitives::Block;
use sc_cli::{ChainSpec, Result, RuntimeVersion, SubstrateCli};
use sc_keystore::LocalKeystore;
use sc_service::config::KeystoreConfig;
use sp_core::{
	crypto::{Pair, Ss58AddressFormat, Ss58Codec},
	hexdisplay::HexDisplay,
};
use sp_keystore::SyncCryptoStore;
use log::*;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Impact Protocol Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/GlobalBoost/substrate/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2023
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" =>
				return Err(
					"Please specify which chain you want to run, e.g. --dev or --chain=local"
						.into(),
				),
			"dev" => Box::new(chain_spec::development_config()),
			"local" => Box::new(chain_spec::local_testnet_config()),
			"testnet" | "impact-testnet" => Box::new(chain_spec::impact_testnet_config()?),
			"staging" => Box::new(chain_spec::staging_testnet_config()),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		};
		Ok(spec)
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&impact_runtime::VERSION
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full(config,cli.eth, cli.author.as_ref().map(|s| s.as_str()))
					.map_err(sc_cli::Error::Service)
			})
		},
		
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run::<Block, RuntimeApi, service::ExecutorDispatch>(config))
		},
		#[cfg(feature = "runtime-benchmarks")]
		Some(Subcommand::Benchmark(cmd)) => {
			use crate::benchmarking::{
				inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder,
			};
			use frame_benchmarking_cli::{
				BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE,
			};

			use impact_runtime::{Block, ExistentialDeposit};

			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							)
						}

						cmd.run::<Block, service::ExecutorDispatch>(config)
					},
					BenchmarkCmd::Block(cmd) => {
						// ensure that we keep the task manager alive
						let partial = new_partial(&config, &cli.eth)?;
						cmd.run(partial.client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						// ensure that we keep the task manager alive
						let partial = new_partial(&config)?;
						let db = partial.backend.expose_db();
						let storage = partial.backend.expose_storage();

						cmd.run(config, partial.client, db, storage)
					},
					BenchmarkCmd::Overhead(cmd) => {
						// ensure that we keep the task manager alive
						let partial = new_partial(&config, &cli.eth)?;
						let ext_builder = RemarkBuilder::new(partial.client.clone());

						cmd.run(
							config,
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
					},
					BenchmarkCmd::Extrinsic(cmd) => {
						// ensure that we keep the task manager alive
						let partial = service::new_partial(&config, &cli.eth)?;
						// Register the *Remark* and *TKA* builders.
						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::new(partial.client.clone())),
							Box::new(TransferKeepAliveBuilder::new(
								partial.client.clone(),
								Sr25519Keyring::Alice.to_account_id(),
								ExistentialDeposit::get(),
							)),
						]);

						cmd.run(
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_factory,
						)
					},
					BenchmarkCmd::Machine(cmd) =>
						cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()),
				}
			})
		},
		Some(Subcommand::FrontierDb(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|mut config| {
				let (client, _, _, _, frontier_backend) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				cmd.run(client, frontier_backend)
			})
		}
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| 
				{
					// Remove Frontier offchain db
					let db_config_dir = db_config_dir(&config);
					let frontier_database_config = match config.database {
						DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
							path: frontier_database_dir(&db_config_dir, "db"),
							cache_size: 0,
						},
						DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
							path: frontier_database_dir(&db_config_dir, "paritydb"),
						},
						_ => {
							return Err(format!("Cannot purge `{:?}` database", config.database).into())
						}
					};				
				cmd.run(frontier_database_config)?;
				cmd.run(config.database)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, backend, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				let aux_revert = Box::new(move |client, _, blocks| {
					grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::ImportMiningKey(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let keystore = match &config.keystore {
					KeystoreConfig::Path { path, password } => {
						LocalKeystore::open(path.clone(), password.clone())
							.map_err(|e| format!("Open keystore failed: {:?}", e))?
					}
					KeystoreConfig::InMemory => LocalKeystore::in_memory(),
				};

				let pair = impact_consensus_pow::app::Pair::from_string(&cmd.suri, None)
					.map_err(|e| format!("Invalid seed: {:?}", e))?;

				SyncCryptoStore::insert_unknown(
					&keystore,
					impact_consensus_pow::app::ID,
					&cmd.suri,
					pair.public().as_ref(),
				)
				.map_err(|e| format!("Registering mining key failed: {:?}", e))?;

				info!(
					"Registered one mining key (public key 0x{}).",
					HexDisplay::from(&pair.public().as_ref())
				);

				Ok(())
			})
		},
		Some(Subcommand::GenerateMiningKey(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				let keystore = match &config.keystore {
					KeystoreConfig::Path { path, password } => {
						LocalKeystore::open(path.clone(), password.clone())
							.map_err(|e| format!("Open keystore failed: {:?}", e))?
					}
					KeystoreConfig::InMemory => LocalKeystore::in_memory(),
				};

				let (pair, phrase, _) = impact_consensus_pow::app::Pair::generate_with_phrase(None);

				SyncCryptoStore::insert_unknown(
					&keystore,
					impact_consensus_pow::app::ID,
					&phrase,
					pair.public().as_ref(),
				)
				.map_err(|e| format!("Registering mining key failed: {:?}", e))?;

				info!("Generated one mining key.");

				println!(
					"Public key: 0x{}\nSecret seed: {}\nAddress: {}",
					HexDisplay::from(&pair.public().as_ref()),
					phrase,
					pair.public()
						.to_ss58check_with_version(Ss58AddressFormat::custom(42)),
				);

				Ok(())
			})
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

				Ok((cmd.run::<Block, ExecutorDispatch>(config), task_manager))
			})
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
	}
}
