#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;

	use frame_support::{
		traits::{Get, OnTimestampSet},
	};

	use impact_primitives::{
		Difficulty, difficulty::{CLAMP_FACTOR, DIFFICULTY_ADJUST_WINDOW, DIFFICULTY_DAMP_FACTOR, MAX_DIFFICULTY,
		MIN_DIFFICULTY}
	};
	
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	use sp_core::U256;
	use sp_runtime::traits::UniqueSaturatedInto;
	use sp_std::cmp::{max, min};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[derive(Encode, Decode, TypeInfo, Clone, Copy, Eq, PartialEq, Debug)]
	pub struct DifficultyAndTimestamp<M> {
		pub difficulty: Difficulty,
		pub timestamp: M,
	}

	/// Move value linearly toward a goal
	pub fn damp(actual: u128, goal: u128, damp_factor: u128) -> u128 {
		(actual + (damp_factor - 1) * goal) / damp_factor
	}


	/// limit value to be within some factor from a goal
	pub fn clamp(actual: u128, goal: u128, clamp_factor: u128) -> u128 {
		max(goal / clamp_factor, min(actual, goal * clamp_factor))
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// Target block time in millseconds.
		type TargetBlockTime: Get<Self::Moment>;
	}

	/// Past difficulties and timestamps, from earliest to latest.
	#[pallet::storage]
	pub type PastDifficultiesAndTimestamps<T: Config>
		= StorageValue<_, BoundedVec<Option<DifficultyAndTimestamp<T::Moment>>,ConstU32<60>>, ValueQuery>;

	/// Current difficulty.
	#[pallet::storage]
    #[pallet::getter(fn difficulty)]
    pub type CurrentDifficulty<T: Config> = StorageValue<_, Difficulty, ValueQuery>;

	/// Initial difficulty.
	#[pallet::storage]
    #[pallet::getter(fn initial_difficulty)]
    pub type InitialDifficulty<T: Config> = StorageValue<_, Difficulty, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub initial_difficulty: U256,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				initial_difficulty: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {			
			<InitialDifficulty<T>>::put(&self.initial_difficulty);
			<CurrentDifficulty<T>>::put(&self.initial_difficulty);
			let mut data: BoundedVec<Option<DifficultyAndTimestamp<T::Moment>>,ConstU32<60>> = Default::default();
			for _ in 0..60{
					data.try_push(None).unwrap();
			}

			<PastDifficultiesAndTimestamps::<T>>::put(data);
		}
	}

	impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
		fn on_timestamp_set(now: T::Moment) {
			let block_time =
				UniqueSaturatedInto::<u128>::unique_saturated_into(T::TargetBlockTime::get());
			let block_time_window = DIFFICULTY_ADJUST_WINDOW as u128 * block_time;
	
			let mut data = <PastDifficultiesAndTimestamps::<T>>::get();

			for i in 1..data.len() {
				data[i - 1] = data[i];
			}

			let datalenght = data.len();

			data[datalenght - 1] = Some(DifficultyAndTimestamp {
				timestamp: now,
				difficulty: Self::difficulty(),
			});

			let mut ts_delta = 0;

			for i in 1..(DIFFICULTY_ADJUST_WINDOW as usize) {
				let prev: Option<u128> = data[i - 1].map(|d| d.timestamp.unique_saturated_into());
				let cur: Option<u128> = data[i].map(|d| d.timestamp.unique_saturated_into());
	
				let delta = match (prev, cur) {
					(Some(prev), Some(cur)) => cur.saturating_sub(prev),
					_ => block_time.into(),
				};
				ts_delta += delta;
			}
	
			if ts_delta == 0 {
				ts_delta = 1;
			}

			let mut diff_sum = U256::zero();
			for i in 0..(DIFFICULTY_ADJUST_WINDOW as usize) {
				let diff = match data[i].map(|d| d.difficulty) {
					Some(diff) => diff,
					None => InitialDifficulty::<T>::get(),
				};
				diff_sum += diff;
			}
	
			if diff_sum < U256::from(MIN_DIFFICULTY) {
				diff_sum = U256::from(MIN_DIFFICULTY);
			}
	
			// adjust time delta toward goal subject to dampening and clamping
			let adj_ts = clamp(
				damp(ts_delta, block_time_window, DIFFICULTY_DAMP_FACTOR),
				block_time_window,
				CLAMP_FACTOR,
			);
	
			// minimum difficulty avoids getting stuck due to dampening
			let difficulty = min(
				U256::from(MAX_DIFFICULTY),
				max(
					U256::from(MIN_DIFFICULTY),
					diff_sum * U256::from(block_time) / U256::from(adj_ts),
				),
			);

			PastDifficultiesAndTimestamps::<T>::put(data);
			CurrentDifficulty::<T>::put(difficulty);
		}
    }

}