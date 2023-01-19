// This file is part of Impact Protocol.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
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

//! Low-level types used throughout the impact code.

//#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic,
};

use sp_api::decl_runtime_apis;

/// Block Difficulty 
pub type Difficulty = sp_core::U256;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Difficulty adjustment parameters
pub mod difficulty {
	use crate::time::BLOCK_TIME_SEC;
	/// Nominal height for standard time intervals, hour is 60 blocks
	pub const HOUR_HEIGHT: u64 = 3600 / BLOCK_TIME_SEC;
	/// A day is 1440 blocks
	pub const DAY_HEIGHT: u64 = 24 * HOUR_HEIGHT;
	/// A week is 10_080 blocks
	pub const WEEK_HEIGHT: u64 = 7 * DAY_HEIGHT;
	/// A year is 524_160 blocks
	pub const YEAR_HEIGHT: u64 = 52 * WEEK_HEIGHT;
	
	/// Number of blocks used to calculate difficulty adjustments
	pub const DIFFICULTY_ADJUST_WINDOW: u64 = HOUR_HEIGHT;
	/// Clamp factor to use for difficulty adjustment
	/// Limit value to within this factor of goal
	pub const CLAMP_FACTOR: u128 = 2;
	/// Dampening factor to use for difficulty adjustment
	pub const DIFFICULTY_DAMP_FACTOR: u128 = 3;
	/// Minimum difficulty, enforced in diff retargetting
	/// avoids getting stuck when trying to increase difficulty subject to dampening
	pub const MIN_DIFFICULTY: u128 = DIFFICULTY_DAMP_FACTOR;
	/// Maximum difficulty.
	pub const MAX_DIFFICULTY: u128 = u128::max_value();
	}

/// Money matters.
pub mod currency {
	use crate::Balance;
	
	/// The existential deposit.
	pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS;
	/// Value of microcents relative to IMPACT.
	pub const MICROCENTS: Balance = 1_000_0;
    /// Value of millicents relative to IMPACT.
	pub const MILLICENTS: Balance = 1_000 * MICROCENTS;
	///Value of cents relative to IMPACT 
	pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	///Value of Dollars relative to IMPACT 
	pub const DOLLARS: Balance = 100 * CENTS;
	/// deposit function
	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}

/// Time.
pub mod time {
	use crate::{BlockNumber, Moment};
/// Block interval, in seconds, the network will tune its next_target for.
pub const BLOCK_TIME_SEC: u64 = 60;
/// Block time interval in milliseconds.
pub const BLOCK_TIME: u64 = BLOCK_TIME_SEC * 1000;
/// Block number of one minute.
pub const MINUTES: BlockNumber = 60 / (BLOCK_TIME_SEC as BlockNumber);
/// Block number of one hour.
pub const HOURS: BlockNumber = 60;
/// Block number of one day.
pub const DAYS: BlockNumber = 24 * HOURS;

/// Estimated time of a block in milliseconds
pub const MILLISECS_PER_BLOCK: Moment = BLOCK_TIME;
/// Estimated time of a block in seconds
pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
///       Attempting to do so will brick block production.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

/// 1 in 4 blocks (on average, not counting collisions) will be primary POW blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
///       Attempting to do so will brick block production.
pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
/// epoch Duration in slot
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
};

}


///Algorithm Used for POW
pub const ALGORITHM_IDENTIFIER: [u8; 8] = *b"yescrypt";

decl_runtime_apis! {
	/// POW Algorithm API
	pub trait AlgorithmApi {
		fn identifier() -> [u8; 8];
	}
}