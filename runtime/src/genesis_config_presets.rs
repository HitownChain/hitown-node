// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{AccountId, BalancesConfig, RuntimeGenesisConfig, SudoConfig};
use alloc::{vec, vec::Vec};
use frame_support::build_struct_json_patch;
use serde_json::Value;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_genesis_builder::{self, PresetId};
use sp_keyring::Sr25519Keyring;

use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_core::ByteArray;

fn make_account_id(ss58: &str) -> AccountId32 {
	AccountId32::from_ss58check(ss58).unwrap()
}

fn make_babe_id(hex_str: &str) -> BabeId {
	let mut bytes = [0u8; 32];
	hex::decode_to_slice(hex_str, &mut bytes).expect("Failed to decode hex to babe id");
	BabeId::from_slice(&bytes).unwrap()
}

fn make_grandpa_id(hex_str: &str) -> GrandpaId {
	let mut bytes = [0u8; 32];
	hex::decode_to_slice(hex_str, &mut bytes).expect("Failed to decode hex to grandpa id");
	GrandpaId::from_slice(&bytes).unwrap()
}

// Returns the genesis config presets populated with given parameters.
fn testnet_genesis(
	initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
	_endowed_accounts: Vec<AccountId>,
	root: AccountId,
) -> Value {
	let balances = vec![];

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances,
		},
		session: pallet_session::GenesisConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						crate::SessionKeys { babe: x.1.clone(), grandpa: x.2.clone() },
					)
				})
				.collect::<Vec<_>>(),
		},
		hpos: pallet_hpos::GenesisConfig {
			genesis_validators: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		babe: pallet_babe::GenesisConfig {
			authorities: vec![],
			epoch_config: sp_consensus_babe::BabeEpochConfiguration {
				c: (1, 4),
				allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
			},
		},
		grandpa: pallet_grandpa::GenesisConfig {
			authorities: vec![],
		},
		sudo: SudoConfig { key: Some(root) },
		evm_chain_id: pallet_evm_chain_id::GenesisConfig { chain_id: 332211 },
	})
}

/// Return the development genesis config.
pub fn development_config_genesis() -> Value {
	testnet_genesis(
		vec![
			(
				sp_keyring::Sr25519Keyring::Alice.to_account_id(),
				sp_keyring::Sr25519Keyring::Alice.public().into(),
				sp_keyring::Ed25519Keyring::Alice.public().into(),
			),
			(
				sp_keyring::Sr25519Keyring::Bob.to_account_id(),
				sp_keyring::Sr25519Keyring::Bob.public().into(),
				sp_keyring::Ed25519Keyring::Bob.public().into(),
			),
			(
				make_account_id("5Hdp8KnaDyxCF8Vd4unq19ZpyRBAcxWyHdgr4RBGGDDWJWgH"),
				make_babe_id("f66d5f405991f62ab7b987719e3a1dfabbc540dbc912df36c612004984d6423b"),
				make_grandpa_id("7ea1140e19dfe0da45ace309e183692eef5166802f47571549dcdffe2de6d45d"),
			),
			(
				make_account_id("5GnVeSGNGNJ2K22tpiutR4ZMXAuscyZ7XYdNM9fWEjzWedTM"),
				make_babe_id("d0d0229eaf1cb693e04b61c3d05f5702db7a2f1a2d6447d0a1d2c563878a1339"),
				make_grandpa_id("293fdc711dc51ac614cc895c58854d3959b193c3cf5cb8011abb413699c8817f"),
			),
			(
				make_account_id("5EUSSocGwkQqsxiTkvSmP6HAiZMkxsziCBoWW4AVVfRDK8fD"),
				make_babe_id("6a927a1eb5ab16ca8a299ba1a6b849e926574f3d84671be115a74d103c0c6338"),
				make_grandpa_id("0c43636529dcfee5c72c7a8a715ba29c9bedbc3f5537801d69f2c4964d7f7d32"),
			),
		],
		vec![
			Sr25519Keyring::Alice.to_account_id(),
			Sr25519Keyring::Bob.to_account_id(),
			make_account_id("5Hdp8KnaDyxCF8Vd4unq19ZpyRBAcxWyHdgr4RBGGDDWJWgH"),
			make_account_id("5GnVeSGNGNJ2K22tpiutR4ZMXAuscyZ7XYdNM9fWEjzWedTM"),
			make_account_id("5EUSSocGwkQqsxiTkvSmP6HAiZMkxsziCBoWW4AVVfRDK8fD"),
			Sr25519Keyring::AliceStash.to_account_id(),
			Sr25519Keyring::BobStash.to_account_id(),
		],
		sp_keyring::Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Return the local genesis config preset.
pub fn local_config_genesis() -> Value {
	testnet_genesis(
		vec![
			(
				sp_keyring::Sr25519Keyring::Alice.to_account_id(),
				sp_keyring::Sr25519Keyring::Alice.public().into(),
				sp_keyring::Ed25519Keyring::Alice.public().into(),
			),
			(
				sp_keyring::Sr25519Keyring::Bob.to_account_id(),
				sp_keyring::Sr25519Keyring::Bob.public().into(),
				sp_keyring::Ed25519Keyring::Bob.public().into(),
			),
			(
				make_account_id("5Hdp8KnaDyxCF8Vd4unq19ZpyRBAcxWyHdgr4RBGGDDWJWgH"),
				make_babe_id("f66d5f405991f62ab7b987719e3a1dfabbc540dbc912df36c612004984d6423b"),
				make_grandpa_id("7ea1140e19dfe0da45ace309e183692eef5166802f47571549dcdffe2de6d45d"),
			),
			(
				make_account_id("5GnVeSGNGNJ2K22tpiutR4ZMXAuscyZ7XYdNM9fWEjzWedTM"),
				make_babe_id("d0d0229eaf1cb693e04b61c3d05f5702db7a2f1a2d6447d0a1d2c563878a1339"),
				make_grandpa_id("293fdc711dc51ac614cc895c58854d3959b193c3cf5cb8011abb413699c8817f"),
			),
			(
				make_account_id("5EUSSocGwkQqsxiTkvSmP6HAiZMkxsziCBoWW4AVVfRDK8fD"),
				make_babe_id("6a927a1eb5ab16ca8a299ba1a6b849e926574f3d84671be115a74d103c0c6338"),
				make_grandpa_id("0c43636529dcfee5c72c7a8a715ba29c9bedbc3f5537801d69f2c4964d7f7d32"),
			),
		],
		Sr25519Keyring::iter()
			.filter(|v| v != &Sr25519Keyring::One && v != &Sr25519Keyring::Two)
			.map(|v| v.to_account_id())
			.collect::<Vec<_>>(),
		Sr25519Keyring::Alice.to_account_id(),
	)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_config_genesis(),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
