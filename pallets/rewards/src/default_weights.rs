// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Impact Protocol.
//
// Copyright (c) 2023 Gabriel Mendoza.
//
// Impact Protocol is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Impact Protocol is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Impact Protocol. If not, see <http://www.gnu.org/licenses/>.
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{constants::RocksDbWeight as DbWeight, Weight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn on_initialize() -> Weight;
    fn on_finalize() -> Weight;
    fn unlock() -> Weight;
    fn set_schedule() -> Weight;
    fn set_lock_params() -> Weight;
}

/// Weight functions for `pallet_rewards`.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn on_initialize() -> Weight {
        Weight::from_ref_time(14_800_000 as u64)
        .saturating_add(T::DbWeight::get().reads(2 as u64))
        .saturating_add(T::DbWeight::get().writes(2 as u64))
    }
    fn on_finalize() -> Weight {
        Weight::from_ref_time(121_500_000 as u64)
        .saturating_add(T::DbWeight::get().reads(5 as u64))
        .saturating_add(T::DbWeight::get().writes(3 as u64))
    }
    fn unlock() -> Weight {
        Weight::from_ref_time(46_000_000 as u64)
        .saturating_add(T::DbWeight::get().reads(1 as u64))
        .saturating_add(T::DbWeight::get().writes(1 as u64))
    }
    fn set_schedule() -> Weight {
        Weight::from_ref_time(32_900_000 as u64)
        .saturating_add(T::DbWeight::get().writes(4 as u64))
    }
    fn set_lock_params() -> Weight {
        Weight::from_ref_time(0 as u64)
        .saturating_add(T::DbWeight::get().writes(1 as u64))
    }

}

impl crate::WeightInfo for () {
	fn on_initialize() -> Weight {
		(Weight::from_ref_time(14_800_000 as u64))
			.saturating_add(DbWeight::get().reads(2 as u64))
			.saturating_add(DbWeight::get().writes(2 as u64))
	}
    
	fn on_finalize() -> Weight {
		(Weight::from_ref_time(121_500_000 as u64))
			.saturating_add(DbWeight::get().reads(5 as u64))
			.saturating_add(DbWeight::get().writes(3 as u64))
	}

	fn unlock() -> Weight {
		(Weight::from_ref_time(46_000_000 as u64))
			.saturating_add(DbWeight::get().reads(1 as u64))
			.saturating_add(DbWeight::get().writes(1 as u64))
	}

	fn set_schedule() -> Weight {
		(Weight::from_ref_time(32_900_000 as u64))
        .saturating_add(DbWeight::get().writes(4 as u64))
	}

	fn set_lock_params() -> Weight {
		(Weight::from_ref_time(0 as u64))
        .saturating_add(DbWeight::get().writes(1 as u64))
	}
}