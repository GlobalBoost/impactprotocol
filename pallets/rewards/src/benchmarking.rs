//! Benchmarking setup for pallet-rewards

use super::*;

#[allow(unused)]
use crate::Pallet as Rewards;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::traits::{OnFinalize, OnInitialize,Currency};
use frame_support::traits::Get;
use frame_system::{EventRecord, RawOrigin};
use sp_runtime::traits::Bounded;
use sp_runtime::{
	generic
};

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

// This function creates a new lock on `who` every block for `num_of_locks`
// starting at block zero.
fn create_locks<T: Config>(who: &T::AccountId, num_of_locks: u32) {
	let mut locks: BTreeMap<T::BlockNumber, BalanceOf<T>> = BTreeMap::new();
	let reward = T::Currency::minimum_balance();
	for i in 0..num_of_locks {
		locks.insert(i.into(), reward);
	}

	RewardLocks::<T>::insert(who, locks);
}

benchmarks! {
	on_initialize {
		let author: T::AccountId = account("author", 0, 0);
		let author_digest = generic::DigestItem::PreRuntime(sp_consensus_pow::POW_ENGINE_ID, author.encode());
		frame_system::Pallet::<T>::deposit_log(author_digest);

		<Reward::<T>>::put(T::Currency::minimum_balance());

		// Whitelist transient storage items
		frame_benchmarking::benchmarking::add_to_whitelist(Author::<T>::hashed_key().to_vec().into());

		let block_number = frame_system::Pallet::<T>::block_number();
	}: { crate::Pallet::<T>::on_initialize(block_number); }
	verify {
		assert_eq!(Author::<T>::get(), Some(author));
	}

	// Worst case: This author already has `max_locks` locked up, produces a new block, and we unlock
	// everything in addition to creating brand new locks for the new reward.
	on_finalize {
		let author: T::AccountId = account("author", 0, 0);
		let reward = BalanceOf::<T>::max_value();

		// Setup pallet variables
		<Author::<T>>::put(&author);
		<Reward::<T>>::put(reward);

		// Create existing locks on author.
		let max_locks = T::GenerateRewardLocks::max_locks(T::LockParametersBounds::get());
		create_locks::<T>(&author, max_locks);

		// Move to a point where all locks would unlock.
		frame_system::Pallet::<T>::set_block_number(max_locks.into());
		assert_eq!(RewardLocks::<T>::get(&author).iter().count() as u32, max_locks);

		// Whitelist transient storage items
		frame_benchmarking::benchmarking::add_to_whitelist(Author::<T>::hashed_key().to_vec().into());

		let block_number = frame_system::Pallet::<T>::block_number();
	}: { crate::Pallet::<T>::on_finalize(block_number); }
	verify {
		assert!(Author::<T>::get().is_none());
		assert!(RewardLocks::<T>::get(&author).iter().count() > 0);
	}

	// Worst case: Target user has `max_locks` which are all unlocked during this call.
	unlock {
		let miner = account("miner", 0, 0);
		let max_locks = T::GenerateRewardLocks::max_locks(T::LockParametersBounds::get());
		create_locks::<T>(&miner, max_locks);
		let caller = whitelisted_caller();
		frame_system::Pallet::<T>::set_block_number(max_locks.into());
		assert_eq!(RewardLocks::<T>::get(&miner).iter().count() as u32, max_locks);
	}: _(RawOrigin::Signed(caller), miner.clone())
	verify {
		assert_eq!(RewardLocks::<T>::get(&miner).iter().count(), 0);
	}

	set_schedule {

	}: _(RawOrigin::Root, T::Currency::minimum_balance(), Vec::new(), Vec::new(), Vec::new())

	// Worst case: a new lock params is set.
	set_lock_params {

	}: _(RawOrigin::Root, LockParameters {period: 150, divide: 25} )

	impl_benchmark_test_suite!(Rewards, crate::mock::new_test_ext(15230153553874516190), crate::mock::Test);
}

