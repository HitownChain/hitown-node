//! # Template Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Substrate template
//! nodes as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! Learn more about FRAME macros [here](https://docs.substrate.io/reference/frame-macros/).
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// FRAME pallets require their own "mock runtimes" to be able to run unit tests. This module
// contains a mock runtime specific for testing this pallet's functionality.
#[cfg(test)]
mod mock;

// This module contains the unit tests for this pallet.
// Learn about pallet unit testing here: https://docs.substrate.io/test/unit-testing/
#[cfg(test)]
mod tests;

// Every callable function or "dispatchable" a pallet exposes must have weight values that correctly
// estimate a dispatchable's execution time. The benchmarking module is used to calculate weights
// for each dispatchable and generates this pallet's weight.rs file. Learn more about benchmarking here: https://docs.substrate.io/test/benchmark/
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

pub mod election;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::{Currency, ReservableCurrency, Get};
	use frame_support::sp_runtime::traits::{Zero, Saturating};
	use alloc::vec::Vec;

	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// The `Pallet` struct serves as a placeholder to implement traits, methods and dispatchables
	// (`Call`s) in this pallet.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	///
	/// All our types and constants a pallet depends on must be declared here.
	/// These types are defined generically and made concrete when the pallet is declared in the
	/// `runtime/src/lib.rs` file of your chain.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// The maximum number of genesis validators.
		#[pallet::constant]
		type MaxGenesisValidators: Get<u32>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::storage]
	pub type ElectedValidators<T: Config> = StorageValue<_, BoundedVec<T::AccountId, T::MaxGenesisValidators>, ValueQuery>;

	/// A storage item for this pallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.substrate.io/build/runtime-storage/>
	#[pallet::storage]
	#[pallet::getter(fn genesis_validators)]
	pub type GenesisValidators<T: Config> = StorageValue<_, BoundedVec<T::AccountId, T::MaxGenesisValidators>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub genesis_validators: Vec<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let bounded_validators: BoundedVec<T::AccountId, T::MaxGenesisValidators> =
				BoundedVec::try_from(self.genesis_validators.clone()).unwrap_or_default();
			GenesisValidators::<T>::put(bounded_validators);
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn validator_weights)]
	pub type ValidatorWeights<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Events that functions in this pallet can emit.
	///
	/// Events are a simple means of indicating to the outside world (such as dApps, chain explorers
	/// or other users) that some notable update in the runtime has occurred. In a FRAME pallet, the
	/// documentation for each event field and its parameters is added to a node's metadata so it
	/// can be used by external interfaces or tools.
	///
	///	The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
	/// will convert the event type of your pallet into `RuntimeEvent` (declared in the pallet's
	/// [`Config`] trait) and deposit it using [`frame_system::Pallet::deposit_event`].
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	/// 记录用户正在解绑的金额及解锁的区块高度
	#[pallet::storage]
	#[pallet::getter(fn unbonding)]
	pub type Unbonding<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (BalanceOf<T>, BlockNumberFor<T>), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user has successfully set a new value.
		WeightUpdated {
			/// The account whose weight was updated.
			validator: T::AccountId,
			/// The new weight value.
			weight: u32,
		},
		/// An account has bonded tokens for staking
		Bonded { account: T::AccountId, amount: BalanceOf<T> },
		/// An account has unbonded tokens
		Unbonded { account: T::AccountId, amount: BalanceOf<T> },
		/// An account has withdrawn unbonded tokens
		Withdrawn { account: T::AccountId, amount: BalanceOf<T> },
		/// An account has nominated a validator
		Nominated { nominator: T::AccountId, validator: T::AccountId },
	}

	/// Errors that can be returned by this pallet.
	///
	/// Errors tell users that something went wrong so it's important that their naming is
	/// informative. Similar to events, error documentation is added to a node's metadata so it's
	/// equally important that they have helpful documentation associated with them.
	///
	/// This type of runtime error can be up to 4 bytes in size should you want to return additional
	/// information.
	#[pallet::error]
	pub enum Error<T> {
		/// The validator is not found in the genesis set or active set.
		ValidatorNotFound,
		/// The weight value exceeds the allowed maximum.
		WeightOverflow,
		/// Only Root can perform this operation.
		RequireRoot,
		/// Insufficient balance to bond.
		InsufficientBalance,
		/// Not enough bonded balance to unbond.
		NotEnoughBonded,
		/// Nothing to withdraw.
		NoUnbondingFunds,
		/// Funds are still locked.
		FundsStillLocked,
	}

	/// The pallet's dispatchable functions ([`Call`]s).
	///
	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// They must always return a `DispatchResult` and be annotated with a weight and call index.
	///
	/// The [`call_index`] macro is used to explicitly
	/// define an index for calls in the [`Call`] enum. This is useful for pallets that may
	/// introduce new dispatchables over time. If the order of a dispatchable changes, its index
	/// will also change which will break backwards compatibility.
	///
	/// The [`weight`] macro is used to assign a weight to each call.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 更新验证者的出块权重（仅限超级管理员执行）
		/// 
		/// # 参数说明
		/// * `origin`: 必须为 Root 权限（如 Sudo 或链上治理通过）
		/// * `validator`: 要更新权重的验证者账户地址
		/// * `weight`: 新的出块权重数值
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()，失败返回 RequireRoot 错误
		/// 
		/// # 用途
		/// 在 HPOS 共识中，根据验证者的历史表现动态调整其在 VRF 出块或 BABE 轮次中的权重参数。
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn update_validator_weight(origin: OriginFor<T>, validator: T::AccountId, weight: u32) -> DispatchResult {
			// Check that the extrinsic was signed by Root.
			ensure_root(origin)?;

			// Update storage.
			ValidatorWeights::<T>::insert(&validator, weight);

			// Emit an event.
			Self::deposit_event(Event::WeightUpdated { validator, weight });

			// Return a successful `DispatchResult`
			Ok(())
		}

		/// 检查验证者的当前状态与权重是否有效
		/// 
		/// # 参数说明
		/// * `origin`: 任意签名账户
		/// * `validator`: 要查询状态的验证者账户
		/// 
		/// # 返回值
		/// * `DispatchResult`: 如果验证者存在且有效则返回 Ok()，否则返回 ValidatorNotFound 错误
		/// 
		/// # 用途
		/// 提供给 DApp 或前端查询某个节点是否具备 HPOS 共识的出块资格。
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::cause_error())]
		pub fn check_validator_status(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			if !ValidatorWeights::<T>::contains_key(&validator) {
				return Err(Error::<T>::ValidatorNotFound.into());
			}

			Ok(())
		}

		/// 抵押代币以参与 Staking
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed），即资金的所有者
		/// * `amount`: 要质押的代币数量
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		/// 
		/// # 用途
		/// 用于验证者或提名者在 HPOS 网络中质押原生代币以获取参与网络共识的权利。
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn bond(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			
			// Ensure user has enough balance and reserve it
			T::Currency::reserve(&who, amount).map_err(|_| Error::<T>::InsufficientBalance)?;

			let current_bond = Bonded::<T>::get(&who);
			Bonded::<T>::insert(&who, current_bond.saturating_add(amount));
			Self::deposit_event(Event::Bonded { account: who, amount });
			Ok(())
		}

		/// 解除抵押代币
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed）
		/// * `amount`: 要解绑的代币数量
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn unbond(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_bond = Bonded::<T>::get(&who);
			
			ensure!(current_bond >= amount, Error::<T>::NotEnoughBonded);
			
			// Reduce bonded amount
			let new_bond = current_bond.saturating_sub(amount);
			if new_bond.is_zero() {
				Bonded::<T>::remove(&who);
				// If fully unbonded, also remove nominations
				Nominators::<T>::remove(&who);
			} else {
				Bonded::<T>::insert(&who, new_bond);
			}

			// Add to unbonding queue (locked for a certain number of blocks, e.g. 7 days in slots)
			// For testnet, let's say 28800 blocks (1 day at 3s/block or something).
			// Here we just use a generic lock period: 10000 blocks.
			let lock_period: BlockNumberFor<T> = 10000u32.into();
			let unlock_block = frame_system::Pallet::<T>::block_number() + lock_period;
			
			// If already unbonding, we add to it and push the timer back
			if let Some((existing_amount, _)) = Unbonding::<T>::get(&who) {
				Unbonding::<T>::insert(&who, (existing_amount.saturating_add(amount), unlock_block));
			} else {
				Unbonding::<T>::insert(&who, (amount, unlock_block));
			}

			Self::deposit_event(Event::Unbonded { account: who, amount });
			Ok(())
		}

		/// 提取已解锁的解绑资金
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			
			let (amount, unlock_block) = Unbonding::<T>::get(&who).ok_or(Error::<T>::NoUnbondingFunds)?;
			let current_block = frame_system::Pallet::<T>::block_number();
			
			ensure!(current_block >= unlock_block, Error::<T>::FundsStillLocked);
			
			// Unreserve the funds so they become free balance
			T::Currency::unreserve(&who, amount);
			Unbonding::<T>::remove(&who);
			
			Self::deposit_event(Event::Withdrawn { account: who, amount });
			Ok(())
		}

		/// 提名目标验证者节点
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed），即提名人
		/// * `validator`: 被提名的验证者账户
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		/// 
		/// # 用途
		/// 用户通过该接口将自己的选票（权重）委托给指定的验证者，用于 NPoS 选举。
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn nominate(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Nominators::<T>::insert(&who, &validator);
			Self::deposit_event(Event::Nominated { nominator: who, validator });
			Ok(())
		}
	}

	impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
			let mut candidates_data = Vec::new();
			
			// 严格遵循规则：“每届任期即使验证者数量不够，中途也不再增补”
			// 这里如果不是正常换届时间点（此处简化抽象，实际由 pallet-session 的 epoch 驱动）
			// 只有在触发新的 election 时，才会更新集合。

			// Collect all bonded users as candidates for now (simplified for demonstration)
			for (account, stake) in Bonded::<T>::iter() {
				// Count nominators for this account
				let nominator_count = Nominators::<T>::iter().filter(|(_, v)| v == &account).count() as u32;

				// Check term limit
				// In testnet: 3 epochs = 18h (6h per epoch)
				// Election period: 3h in testnet (scaled from 30 days in mainnet)
				// Note: Full implementation of term tracking would be needed here.
				// For now we simulate the 18h limit in concept.

				// Simplified activity (e.g. 100)
				let activity = 100;

				// Assuming Balance can be converted to u128 for scoring
				// In production, you would safely convert Balance to u128
				let total_stake: u128 = stake.try_into().unwrap_or(0);

				candidates_data.push(crate::election::CandidateData {
					account,
					total_stake,
					nominator_count,
					activity,
				});
			}

			// Add GenesisValidators to candidates to prevent chain halt if new bonded users have no session keys.
			for genesis_val in GenesisValidators::<T>::get().into_inner() {
				if !Bonded::<T>::contains_key(&genesis_val) {
					candidates_data.push(crate::election::CandidateData {
						account: genesis_val,
						total_stake: 0, // Fallback stake
						nominator_count: 0,
						activity: 100,
					});
				}
			}
			
			let max_validators = T::MaxGenesisValidators::get() as usize;
			let elected = crate::election::compute_election(candidates_data, max_validators);
			
			let bounded_elected: BoundedVec<T::AccountId, T::MaxGenesisValidators> = 
				BoundedVec::try_from(elected.clone()).unwrap_or_default();
				
			ElectedValidators::<T>::put(bounded_elected);
			
			if elected.is_empty() {
				// Fallback to GenesisValidators if no one is elected
				// Note: GenesisValidators are just the initial set (like Bob). They are subject to the same rotation.
				let genesis = GenesisValidators::<T>::get().into_inner();
				if genesis.is_empty() {
					None
				} else {
					Some(genesis)
				}
			} else {
				Some(elected)
			}
		}
		
		fn end_session(_end_index: u32) {}
		fn start_session(_start_index: u32) {}
	}
}
