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

//! Substrate chain configurations.

use sp_consensus_grandpa::AuthorityId as GrandpaId;
use impact_runtime::{
	wasm_binary_unwrap, AuthorityDiscoveryConfig,
	BalancesConfig, Block, CouncilConfig, DemocracyConfig, ElectionsConfig, GrandpaConfig,
	IndicesConfig, RewardsConfig, DifficultyConfig, MaxNominations, NominationPoolsConfig, SessionConfig,
	opaque::SessionKeys, SocietyConfig, StakerStatus, StakingConfig, SudoConfig, SystemConfig,
	TechnicalCommitteeConfig,EVMChainIdConfig, EVMConfig, SS58Prefix
};
use impact_primitives::currency::*;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public, H160, U256};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

use std::{collections::BTreeMap, str::FromStr};

pub use impact_runtime::GenesisConfig;
pub use impact_primitives::{AccountId, Balance, Signature};

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;
/// Impact Protocol testnet generator
pub fn impact_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/impact-testnet.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, authority_discovery }
}

fn staging_testnet_config_genesis() -> GenesisConfig {
	#[rustfmt::skip]
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	//
	// and
	//
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		AuthorityDiscoveryId,
	)> = vec![
		(
			// 5FNCTJVDxfFnmUYKHqbJHjUi7UFbZ6pzC39sL6E5RVpB4vc9
			array_bytes::hex_n_into_unchecked("920c238572e2b31c2efd19dad1a5674c8188388d9a30d0d01847759a5dc64069"),
			// 5GgaLpTUcgbCTGnwVkCjSSzZ5jTaEPuxtWGRDhi8M1BP1hTs
			array_bytes::hex_n_into_unchecked("cc4c78c7f22298f17e0e2dcefb7cff85b30e19dc1699cb9d1de00e5ea65a433d"),
			// 5Fm7Lc3XDxxbH4LBKxn1tf44P1R5M5cm2vmuLZbUnPFLfu5p
			array_bytes::hex2array_unchecked("a3859016b0b17b7ed6a5b2efcb4ce0e2b6b56ec8594d416c0ea3685929f0a15c").unchecked_into(),
			// 5Eeard4qtNM8DBvqDEKn5GBAspbT7QEvhAjxSsYePB26XAiJ
			array_bytes::hex2array_unchecked("724f3e6ec8a61ea3dc5b76c00a049f84fd7f212443b01241e0a2bb4ce503b345").unchecked_into(),			
			),
			(
			// 5DP3mCevjzqrYhJgPpQFkpoERKg55K422u5KiRGPQaoJEgRH
			array_bytes::hex_n_into_unchecked("3a39a8d0654e0f52b2ee8202ed3488e7a82650dde0daadaddbc8ea825e408d13"),
			// 5HeTTicL5u17JCkDhAwcAHUXMGEzXbDLjPYmNC5ahKhwaLgt
			array_bytes::hex_n_into_unchecked("f6eb0cff5244d7437ed659ac34e6ea66daa857f3d1c580f452b8512ae7fdba0f"),
			// 5FKFid7kAaVFkfbpShH8dzw3wJipiuGPruTzc6WB2WKMviUX
			array_bytes::hex2array_unchecked("8fcd640390db86812092a0b2b244aac9d8375be2c0a3434eb9062b58643c60fb").unchecked_into(),
			// 5DhZENrJzzaJL2MwLsQsvxARhhAPCVXdHxs2oSJuJLxhUsbg
			array_bytes::hex2array_unchecked("485746d4cc0f20b5581f24b30f91b34d49a7b96b85bb8ba202f354aea8e14b1f").unchecked_into(),			
			),
			(
			// 5DJQ1NXeThmu2N5yQHZUsY64Lmgm95nnchpRWi1nSBU2rgod
			array_bytes::hex_n_into_unchecked("36ad94b252606800bc80869baf453663ac2e9276e83f0401107384c053552f3e"),
			// 5EWQq4ns7miu8B8ArsspZ9KBHX6gwjJXptJq5dbLgQucZvdc
			array_bytes::hex_n_into_unchecked("6c1386fd76e4eec0365a439db0decae0d5d715e33db934bc44be28f73df50674"),
			// 5EUsrdaXAAJ87Y7yCRdrYKeyHdTYbSr9tJFCYEy12CNap2v2
			array_bytes::hex2array_unchecked("6ae80477725a1e4f3194fac59286662ea491c9461cb54909432228351be3474a").unchecked_into(),
			// 5CLfsFaNYPGQvpYkroN1qrWLt54Xpmn6shAxdE45bCy1cvgv
			array_bytes::hex2array_unchecked("0c2d3a4c604c4ad68e285cc1c401dd2665c1cd7193b16d4d9c854c27a9238a1a").unchecked_into(),
			),
			(
			// 5EtMni1z8bHkrFboqro7R7PvfDcPqkpnS8tbW14SaR38rt4c
			array_bytes::hex_n_into_unchecked("7cd0b0c03ff149e9d758a11594f5d1844ebc3d6a291eeb157344d057ab4ec240"),
			// 5D2zD2XGcJUerPbmuZ4qjuAgkV5sX3PmWZFjzyQ5FPvQ4YQ4
			array_bytes::hex_n_into_unchecked("2aecc8f8047ab4ef7393113fb1421d935f6649a95eeed9a1adbf719528fd0f11"),
			// 5CFk6YgnQGqQYL3bjUwMegmWq6JjambP5U5kc1pCTCcJGoWG
			array_bytes::hex2array_unchecked("086b3ccfb1eb8e1af18db604665be263ffe74c0853b9014ac5c4b572d6edf294").unchecked_into(),
			// 5HgfLj5eN6s4ipJMoDYzRV5b4DkZMKimP3Wa3H1DHqeGK5wh
			array_bytes::hex2array_unchecked("f899880823b45669c5b7215fc0fa1f0a348113cfbfe450c41018a6699c76a23a").unchecked_into(),
			),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = array_bytes::hex_n_into_unchecked(
		// 5Fk6QsYKvDXxdXumGdHnNQ7V7FziREy6qn8WjDLEWF8WsbU3
		"a2bf32e50edd79c181888da41c80c67c191e9e6b29d3f2efb102ca0e2b53c558",
	);

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(U256::from(200),initial_authorities, vec![], root_key, Some(endowed_accounts), SS58Prefix::get() as u64)
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"Staging Testnet",
		"staging_testnet",
		ChainType::Live,
		staging_testnet_config_genesis,
		boot_nodes,
		Some(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		),
		None,
		None,
		None,
		Default::default(),
	)
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
	initial_difficulty: U256,
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
	chain_id: u64,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		]
	});
	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(x) {
				endowed_accounts.push(x.clone())
			}
		});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		},
		indices: IndicesConfig { indices: vec![] },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: CouncilConfig::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		sudo: SudoConfig { key: Some(root_key) },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		society: SocietyConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		},
		vesting: Default::default(),
		assets: Default::default(),
		transaction_storage: Default::default(),
		transaction_payment: Default::default(),
		alliance: Default::default(),
		alliance_motion: Default::default(),
		nomination_pools: NominationPoolsConfig {
			min_create_bond: 10 * DOLLARS,
			min_join_bond: 1 * DOLLARS,
			..Default::default()
		},
		difficulty: DifficultyConfig { initial_difficulty },
		rewards: RewardsConfig {
			reward: 60 * DOLLARS,
			mints: Default::default(),
		},
		// EVM compatibility
		evm_chain_id: EVMChainIdConfig { chain_id },
		evm: EVMConfig {
			accounts: {
				let mut map = BTreeMap::new();
				map.insert(
					// H160 address of Alice dev account
					// Derived from SS58 (42 prefix) address
					// SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
					// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
					// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
					H160::from_str("d43593c715fdd31c61141abd04a99fd6822c8558")
						.expect("internal H160 is valid; qed"),
					fp_evm::GenesisAccount {
						balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
							.expect("internal U256 is valid; qed"),
						code: Default::default(),
						nonce: Default::default(),
						storage: Default::default(),
					},
				);
				map.insert(
					// H160 address of CI test runner account
					H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b")
						.expect("internal H160 is valid; qed"),
					fp_evm::GenesisAccount {
						balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
							.expect("internal U256 is valid; qed"),
						code: Default::default(),
						nonce: Default::default(),
						storage: Default::default(),
					},
				);
				map.insert(
					// H160 address for benchmark usage
					H160::from_str("1000000000000000000000000000000000000001")
						.expect("internal H160 is valid; qed"),
					fp_evm::GenesisAccount {
						nonce: U256::from(1),
						balance: U256::from(1_000_000_000_000_000_000_000_000u128),
						storage: Default::default(),
						code: vec![0x00],
					},
				);
				map
			},
		},
		ethereum: Default::default(),
		dynamic_fee: Default::default(),
		base_fee: Default::default(),
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		U256::from(10_000),
		vec![authority_keys_from_seed("Alice")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		// Ethereum chain ID
		SS58Prefix::get() as u64,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		None,
		None,
		None,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		U256::from(200),
		vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		SS58Prefix::get() as u64,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		None,
		None,
		None,
		None,
		Default::default(),
	)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::service::{new_full_base, NewFullBase};
	use sc_service_test;
	use sp_runtime::BuildStorage;

	fn local_testnet_genesis_instant_single() -> GenesisConfig {
		testnet_genesis(
			U256::from(200),
			vec![authority_keys_from_seed("Alice")],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
			SS58Prefix::get() as u64,
		)
	}

	/// Local testnet config (single validator - Alice)
	pub fn integration_test_config_with_single_authority() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis_instant_single,
			vec![],
			None,
			None,
			None,
			None,
			Default::default(),
		)
	}

	/// Local testnet config (multivalidator Alice + Bob)
	pub fn integration_test_config_with_two_authorities() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis,
			vec![],
			None,
			None,
			None,
			None,
			Default::default(),
		)
	}

	#[test]
	#[ignore]
	fn test_connectivity() {
		sp_tracing::try_init_simple();

		sc_service_test::connectivity(integration_test_config_with_two_authorities(), |config| {
			let NewFullBase { task_manager, client, network, transaction_pool, .. } =
				new_full_base(config, false)?;
			Ok(sc_service_test::TestNetComponents::new(
				task_manager,
				client,
				network,
				transaction_pool,
			))
		});
	}

	#[test]
	fn test_create_development_chain_spec() {
		development_config().build_storage().unwrap();
	}

	#[test]
	fn test_create_local_testnet_chain_spec() {
		local_testnet_config().build_storage().unwrap();
	}

	#[test]
	fn test_staging_test_net_chain_spec() {
		staging_testnet_config().build_storage().unwrap();
	}
}
