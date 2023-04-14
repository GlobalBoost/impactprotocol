#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, Encode};
use sp_std::{
	collections::btree_map::BTreeMap, iter::FromIterator, ops::Bound::Included, prelude::*,
};



#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod default_weights;
pub use default_weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	
	use frame_support::pallet_prelude::*;
	use frame_support::{
			ensure,
		traits::{Currency, Get, LockIdentifier, LockableCurrency, WithdrawReasons, FindAuthor},
		weights::Weight,
		ConsensusEngineId,
	};
	use frame_system::pallet_prelude::*;
	use sp_consensus_pow::POW_ENGINE_ID;
	use sp_runtime::traits::{Saturating, Zero};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[derive(Encode, Decode, TypeInfo, Clone, Copy, PartialEq, Eq, Debug)]
	pub struct LockParameters {
		pub period: u16,
		pub divide: u16,
	}

	pub struct LockBounds {
		pub period_max: u16,
		pub period_min: u16,
		pub divide_max: u16,
		pub divide_min: u16,
	}
	/// Trait for generating reward locks.
	pub trait GenerateRewardLocks<T: Config> {
		/// Generate reward locks.
		fn generate_reward_locks(
			current_block: T::BlockNumber,
			total_reward: BalanceOf<T>,
			lock_parameters: Option<LockParameters>,
		) -> BTreeMap<T::BlockNumber, BalanceOf<T>>;

		fn max_locks(lock_bounds: LockBounds) -> u32;
	}



	/// Type alias for currency balance.
	pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// An implementation of on-chain currency.
		type Currency: LockableCurrency<Self::AccountId>;
		/// Donation destination.
		type DonationDestination: Get<Self::AccountId>;
		/// Generate reward locks.
		type GenerateRewardLocks: GenerateRewardLocks<Self>;
		/// Weights for this pallet.
		type WeightInfo: WeightInfo;
		/// Lock Parameters Bounds.
		type LockParametersBounds: Get<LockBounds>;
	}

		#[pallet::storage]
		#[pallet::getter(fn author)]
		pub type Author<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

		/// Current block reward for miner.
		#[pallet::storage]
		#[pallet::getter(fn reward)]
		pub type Reward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

		/// Current block reward for miner.
		#[pallet::storage]
		#[pallet::getter(fn reward_locks)]
		pub(super) type RewardLocks<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BTreeMap<T::BlockNumber, BalanceOf<T>>,ValueQuery>;

		/// Reward changes planned in the future.
		#[pallet::storage]
		#[pallet::getter(fn reward_changes)]
		pub(super) type RewardChanges<T: Config> = StorageValue<_,BTreeMap<T::BlockNumber, BalanceOf<T>>,ValueQuery>;	
		
		/// Current block mints.
		#[pallet::storage]
		#[pallet::getter(fn mints)]
		pub(super) type Mints<T: Config> = StorageValue<_,BTreeMap<T::AccountId, BalanceOf<T>>,ValueQuery>;

		/// Current block mints.
		#[pallet::storage]
		#[pallet::getter(fn mint_changes)]
		pub(super) type MintChanges<T: Config> = StorageValue<_,BTreeMap<T::BlockNumber, BTreeMap<T::AccountId, BalanceOf<T>>>,ValueQuery>;

		#[pallet::storage]
		#[pallet::getter(fn lock_params)]
		pub type LockParams<T: Config> = StorageValue<_, LockParameters, OptionQuery>;

		#[pallet::genesis_config]
		pub struct GenesisConfig<T: Config> {
			pub reward: BalanceOf<T>,
			pub mints: BTreeMap<T::AccountId, BalanceOf<T>>,
		}

		#[cfg(feature = "std")]
		impl<T: Config> Default for GenesisConfig<T> {
			fn default() -> Self {
				Self {
					reward: Default::default(),
					mints: Default::default(),
				}
			}
		}

		#[pallet::genesis_build]
		impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
			fn build(&self) {
				<Reward<T>>::put(&self.reward);
				<Mints<T>>::put(&self.mints);
			}
		}

		// Pallets use events to inform users when important changes are made.
		// https://docs.substrate.io/main-docs/build/events-errors/
		#[pallet::event]
		#[pallet::generate_deposit(pub(super) fn deposit_event)]
		pub enum Event<T: Config> {
			/// Event documentation should end with an array that provides descriptive names for event
			/// A new schedule has been set.
			ScheduleSet,
			/// Reward has been sent.
			Rewarded(T::AccountId, BalanceOf<T>),
			/// Reward has been changed.
			RewardChanged(BalanceOf<T>),
			/// Mint has been sent.
			Minted(T::AccountId, BalanceOf<T>),
			/// Mint has been changed.
			MintsChanged(BTreeMap<T::AccountId, BalanceOf<T>>),
			/// Lock Parameters have been changed.
			LockParamsChanged(LockParameters),
		}

		// Errors inform users that something went wrong.
		#[pallet::error]
		pub enum Error<T> {
			/// Reward set is too low.
			RewardTooLow,
			/// Mint value is too low.
			MintTooLow,
			/// Reward curve is not sorted.
			NotSorted,
			/// Lock parameters are out of bounds.
			LockParamsOutOfBounds,
			/// Lock period is not a mutiple of the divide.
			LockPeriodNotDivisible,
		}

		// Pallet implements [`Hooks`] trait to define some logic to execute in some context.
		#[pallet::hooks]
		impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

			fn on_initialize(now: T::BlockNumber) -> Weight {
				let author = frame_system::Pallet::<T>::digest()
				.logs
				.iter()
				.filter_map(|s| s.as_pre_runtime())
				.filter_map(|(id, mut data)| if id == POW_ENGINE_ID {
					T::AccountId::decode(&mut data).ok()
				} else {
					None
				})
				.next();

				if let Some(author) = author {
					<Author<T>>::put(author);
				}

				<RewardChanges::<T>>::mutate(|reward_changes| {
					let mut removing = Vec::new();
	
					for (block_number, reward) in reward_changes.range((Included(Zero::zero()), Included(now))) {
						Reward::<T>::set(*reward);
						removing.push(*block_number);
	
						Self::deposit_event(Event::<T>::RewardChanged(*reward));

					}
	
					for block_number in removing {
						reward_changes.remove(&block_number);
					}
				});

				<MintChanges::<T>>::mutate(|mint_changes| {
					let mut removing = Vec::new();
	
					for (block_number, mints) in mint_changes.range((Included(Zero::zero()), Included(now))) {
						<Mints::<T>>::set(mints.clone());
						removing.push(*block_number);
	
						Self::deposit_event(Event::<T>::MintsChanged(mints.clone()));
					}
	
					for block_number in removing {
						mint_changes.remove(&block_number);
					}
				});

				T::WeightInfo::on_initialize().saturating_add(T::WeightInfo::on_finalize())

			}

			// `on_finalize` is executed at the end of block after all extrinsic are dispatched.
			fn on_finalize(now: T::BlockNumber) {
				if let Some(author) = <Author<T>>::get() {
					let reward = Reward::<T>::get();
					Self::do_reward(&author, reward, now);
				}

				let mints = Mints::<T>::get();
				Self::do_mints(&mints);

				<Author<T>>::kill();
			}
		
		}

		#[pallet::call]
		impl<T: Config> Pallet<T> {
			#[pallet::call_index(0)]
			#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
			pub fn set_schedule(
				origin: OriginFor<T>,
				reward: BalanceOf<T>,
				mints: Vec<(T::AccountId, BalanceOf<T>)>,
				reward_changes: Vec<(T::BlockNumber, BalanceOf<T>)>,
				mint_changes: Vec<(T::BlockNumber, Vec<(T::AccountId, BalanceOf<T>)>)>,
			) -> DispatchResultWithPostInfo {
				ensure_root(origin)?;
	
				let mints = BTreeMap::from_iter(mints.into_iter());
				let reward_changes = BTreeMap::from_iter(reward_changes.into_iter());
				let mint_changes = BTreeMap::from_iter(
					mint_changes.into_iter().map(|(k, v)| (k, BTreeMap::from_iter(v.into_iter())))
				);
	
				ensure!(reward >= T::Currency::minimum_balance(), Error::<T>::RewardTooLow);
				for (_, mint) in &mints {
					ensure!(*mint >= T::Currency::minimum_balance(), Error::<T>::MintTooLow);
				}
				for (_, reward_change) in &reward_changes {
					ensure!(*reward_change >= T::Currency::minimum_balance(), Error::<T>::RewardTooLow);
				}
				for (_, mint_change) in &mint_changes {
					for (_, mint) in mint_change {
						ensure!(*mint >= T::Currency::minimum_balance(), Error::<T>::MintTooLow);
					}
				}
	
				Reward::<T>::put(reward);
				Self::deposit_event(Event::<T>::RewardChanged(reward));
	
				Mints::<T>::put(mints.clone());
				Self::deposit_event(Event::<T>::MintsChanged(mints));
	
				RewardChanges::<T>::put(reward_changes);
				MintChanges::<T>::put(mint_changes);
				Self::deposit_event(Event::<T>::ScheduleSet);

				Ok(().into())
			}	
			
			#[pallet::call_index(1)]
			#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
			pub fn set_lock_params(origin: OriginFor<T>, lock_params: LockParameters) -> DispatchResultWithPostInfo {
				ensure_root(origin)?;
	
				let bounds = T::LockParametersBounds::get();
				ensure!((bounds.period_min..=bounds.period_max).contains(&lock_params.period) &&
					(bounds.divide_min..=bounds.divide_max).contains(&lock_params.divide), Error::<T>::LockParamsOutOfBounds);
				ensure!(lock_params.period % lock_params.divide == 0, Error::<T>::LockPeriodNotDivisible);
	
				LockParams::<T>::put(lock_params);
				Self::deposit_event(Event::<T>::LockParamsChanged(lock_params));

				Ok(().into())
			}

			/// Unlock any vested rewards for `target` account.
			#[pallet::call_index(2)]
			#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
			pub fn unlock(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
				ensure_signed(origin)?;

				let locks = Self::reward_locks(&target);
				let current_number = frame_system::Pallet::<T>::block_number();
				Self::do_update_reward_locks(&target, locks, current_number);

				Ok(().into())
			}
		}

		impl<T: Config> FindAuthor<T::AccountId> for Pallet<T> {
			fn find_author<'a, I>(_digests: I) -> Option<T::AccountId>
			where
				I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
			{
				let author = frame_system::Pallet::<T>::digest()
				.logs
				.iter()
				.filter_map(|s| s.as_pre_runtime())
				.filter_map(|(id, mut data)| if id == POW_ENGINE_ID {
					T::AccountId::decode(&mut data).ok()
				} else {
					None
				})
				.next();

				author
			}
		}	



     const REWARDS_ID: LockIdentifier = *b"rewards ";

	impl<T: Config> Pallet<T> {

		pub fn do_reward(author: &T::AccountId, reward: BalanceOf<T>, when: T::BlockNumber){
			let miner_total = reward;
	
			let miner_reward_locks =
				T::GenerateRewardLocks::generate_reward_locks(when, miner_total, LockParams::<T>::get());
	
			drop(T::Currency::deposit_creating(&author, miner_total));
	
			if miner_reward_locks.len() > 0 {
				let mut locks = Self::reward_locks(&author);
	
				for (new_lock_number, new_lock_balance) in miner_reward_locks {
					let old_balance = *locks
						.get(&new_lock_number)
						.unwrap_or(&BalanceOf::<T>::default());
					let new_balance = old_balance.saturating_add(new_lock_balance);
					locks.insert(new_lock_number, new_balance);
				}
	
				Self::do_update_reward_locks(&author, locks, when);

			}
		}

		pub fn do_update_reward_locks (
			author: &T::AccountId,
			mut locks: BTreeMap<T::BlockNumber, BalanceOf<T>>,
			current_number: T::BlockNumber,
		){
			let mut expired = Vec::new();
			let mut total_locked: BalanceOf<T> = Zero::zero();
	
			for (block_number, locked_balance) in &locks {
				if block_number <= &current_number {
					expired.push(*block_number);
				} else {
					total_locked = total_locked.saturating_add(*locked_balance);
				}
			}
	
			for block_number in expired {
				locks.remove(&block_number);
			}
	
			T::Currency::set_lock(
				REWARDS_ID,
				&author,
				total_locked,
				WithdrawReasons::except(WithdrawReasons::TRANSACTION_PAYMENT),
			);
	
			RewardLocks::<T>::insert(author, locks);

		}
	
		pub fn do_mints(mints: &BTreeMap<T::AccountId, BalanceOf<T>>){
			for (destination, mint) in mints {
				drop(T::Currency::deposit_creating(&destination, *mint));
			}
		}

	}
}