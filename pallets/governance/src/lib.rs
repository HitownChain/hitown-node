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

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::traits::ReservableCurrency;

	// The `Pallet` struct serves as a placeholder to implement traits, methods and dispatchables
	// (`Call`s) in this pallet.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	///
	/// All our types and constants a pallet depends on must be declared here.
	/// These types are defined generically and made concrete when the pallet is declared in the
	/// `runtime/src/lib.rs` file of your chain.
	pub type BalanceOf<T> =
		<<T as Config>::Currency as frame_support::traits::Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// Currency type for locking funds during proposals and voting.
		type Currency: frame_support::traits::ReservableCurrency<Self::AccountId>;
		/// The amount of currency to reserve for a proposal.
		#[pallet::constant]
		type ProposalBond: Get<BalanceOf<Self>>;
	}

	/// A struct representing a proposal's state.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		pub proposer: T::AccountId,
		pub hash: T::Hash,
		pub end_block: frame_system::pallet_prelude::BlockNumberFor<T>,
		pub yes_votes: BalanceOf<T>,
		pub no_votes: BalanceOf<T>,
		pub status: ProposalStatus,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum ProposalStatus {
		Active,
		Passed,
		Rejected,
	}

	/// A storage item for this pallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.substrate.io/build/runtime-storage/>
	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u32, Proposal<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_count)]
	pub type ProposalCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub type Votes<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, (bool, BalanceOf<T>), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new proposal was created
		Proposed { proposal_index: u32, proposer: T::AccountId, hash: T::Hash },
		/// Voted on a proposal
		Voted { proposal_index: u32, voter: T::AccountId, approve: bool, weight: BalanceOf<T> },
		/// Proposal resolved
		Resolved { proposal_index: u32, passed: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Proposal does not exist
		ProposalNotFound,
		/// Proposal is not active
		ProposalNotActive,
		/// Already voted
		AlreadyVoted,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 发起新的链上治理提案
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed），即提案发起人
		/// * `hash`: 提案内容的 Hash 值（内容原文可存储于 IPFS 等链下设施）
		/// * `duration`: 提案持续的区块数
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		/// 
		/// # 用途
		/// 任何人都可以发起关于参数修改、资金使用或系统升级的链上提案，需要抵押一定的代币。
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn propose(origin: OriginFor<T>, hash: T::Hash, duration: frame_system::pallet_prelude::BlockNumberFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let index = ProposalCount::<T>::get();
			
			// Reserve bond
			T::Currency::reserve(&who, T::ProposalBond::get())?;

			let end_block = <frame_system::Pallet<T>>::block_number() + duration;

			let proposal = Proposal {
				proposer: who.clone(),
				hash,
				end_block,
				yes_votes: 0u32.into(),
				no_votes: 0u32.into(),
				status: ProposalStatus::Active,
			};

			Proposals::<T>::insert(index, proposal);
			ProposalCount::<T>::put(index + 1);
			Self::deposit_event(Event::Proposed { proposal_index: index, proposer: who, hash });
			Ok(())
		}

		/// 对指定的提案进行投票
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed），即投票人
		/// * `proposal_index`: 目标提案的全局唯一 ID
		/// * `approve`: 投票结果（true 表示赞成，false 表示反对）
		/// * `weight`: 抵押用于投票的代币数量
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()，提案不存在则返回 ProposalNotFound 错误
		/// 
		/// # 用途
		/// 社区成员通过该接口对链上未决提案表达立场，并抵押代币作为投票权重，结果将用于计票和决议。
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn vote(origin: OriginFor<T>, proposal_index: u32, approve: bool, weight: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			
			let mut proposal = Proposals::<T>::get(proposal_index).ok_or(Error::<T>::ProposalNotFound)?;
			ensure!(proposal.status == ProposalStatus::Active, Error::<T>::ProposalNotActive);
			
			let current_block = <frame_system::Pallet<T>>::block_number();
			ensure!(current_block <= proposal.end_block, Error::<T>::ProposalNotActive);

			ensure!(!Votes::<T>::contains_key(proposal_index, &who), Error::<T>::AlreadyVoted);

			// Reserve voting weight
			T::Currency::reserve(&who, weight)?;

			if approve {
				proposal.yes_votes += weight;
			} else {
				proposal.no_votes += weight;
			}

			Proposals::<T>::insert(proposal_index, proposal);
			Votes::<T>::insert(proposal_index, &who, (approve, weight));

			Self::deposit_event(Event::Voted { proposal_index, voter: who, approve, weight });
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut weight = T::WeightInfo::do_something();

			// Iterate over active proposals to see if any have ended
			for (index, mut proposal) in Proposals::<T>::iter() {
				if proposal.status == ProposalStatus::Active && n > proposal.end_block {
					// Proposal has ended, resolve it
					let passed = proposal.yes_votes > proposal.no_votes;
					
					proposal.status = if passed { ProposalStatus::Passed } else { ProposalStatus::Rejected };
					Proposals::<T>::insert(index, &proposal);

					// Unreserve the proposer's bond
					T::Currency::unreserve(&proposal.proposer, T::ProposalBond::get());

					// Unreserve all voters' weights
					for (voter, (_, vote_weight)) in Votes::<T>::iter_prefix(index) {
						T::Currency::unreserve(&voter, vote_weight);
					}

					Self::deposit_event(Event::Resolved { proposal_index: index, passed });
					weight = weight.saturating_add(T::WeightInfo::do_something());
				}
			}

			weight
		}
	}
}
