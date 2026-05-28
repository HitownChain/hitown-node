#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::traits::{Currency, Get};
use sp_runtime::{Perbill, traits::Saturating};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// The currency mechanism.
		type Currency: Currency<Self::AccountId>;
		/// The amount of reward given for authoring a block.
		#[pallet::constant]
		type BlockReward: Get<BalanceOf<Self>>;
		/// A type that provides the current active validators.
		type ActiveValidators: Get<alloc::vec::Vec<Self::AccountId>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Block reward has been minted and given to the author.
		BlockRewardMinted {
			author: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// Validator reward has been distributed.
		ValidatorRewardMinted {
			validator: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> pallet_authorship::EventHandler<T::AccountId, frame_system::pallet_prelude::BlockNumberFor<T>> for Pallet<T> {
	fn note_author(author: T::AccountId) {
		let total_reward = T::BlockReward::get();
		
		// Ratio: 61.8% for the block author, 38.2% distributed equally among active validators
		let author_ratio = Perbill::from_rational(618u32, 1000u32);
		
		let author_reward = author_ratio * total_reward;
		
		// 1. Mint reward to the block author
		let _ = T::Currency::deposit_creating(&author, author_reward);
		Self::deposit_event(Event::BlockRewardMinted { author: author.clone(), amount: author_reward });
		
		// 2. Mint reward to all active validators
		let active_validators = T::ActiveValidators::get();
		let num_validators = active_validators.len() as u32;

		let remainder = total_reward.saturating_sub(author_reward);
		let mut actual_validator_reward: BalanceOf<T> = 0u32.into();
		if num_validators > 0 {
			// Do not use Perbill for splitting to avoid losing precision.
			// Calculate exact division using integer arithmetic.
			let reward_per_validator = remainder / num_validators.into();
			for validator in active_validators {
				let _ = T::Currency::deposit_creating(&validator, reward_per_validator);
				Self::deposit_event(Event::ValidatorRewardMinted { validator, amount: reward_per_validator });
				actual_validator_reward = actual_validator_reward + reward_per_validator;
			}
		}

		// Because of integer division, there might be a tiny remainder. 
		// We deposit the final remainder to the author to ensure exactly total_reward is minted per block.
		let final_remainder = remainder.saturating_sub(actual_validator_reward);
		let zero: BalanceOf<T> = 0u32.into();
		if final_remainder > zero {
			let _ = T::Currency::deposit_creating(&author, final_remainder);
		}
	}
}
