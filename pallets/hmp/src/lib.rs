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
	}

	/// A storage item for this pallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.substrate.io/build/runtime-storage/>
	#[pallet::storage]
	#[pallet::getter(fn messages)]
	pub type Messages<T: Config> = StorageMap<_, Blake2_128Concat, u32, [u8; 32], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn message_count)]
	pub type MessageCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Message sent across chains
		MessageSent { message_id: u32, sender: T::AccountId, payload_hash: [u8; 32] },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Message send failed
		SendFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 发送 Hitown 跨链消息协议 (HMP) 的消息
		/// 
		/// # 参数说明
		/// * `origin`: 必须为签名账户（Signed），即消息发送者
		/// * `payload_hash`: 跨链消息内容载荷的 32 字节 Hash 值
		/// 
		/// # 返回值
		/// * `DispatchResult`: 成功返回 Ok()
		/// 
		/// # 用途
		/// 用于将跨链或跨生态的信息进行链上打戳（Timestamp & Verify），
		/// 并发出 MessageSent 事件供跨链中继节点捕获和处理。
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn send_message(origin: OriginFor<T>, payload_hash: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let msg_id = MessageCount::<T>::get();
			Messages::<T>::insert(msg_id, payload_hash);
			MessageCount::<T>::put(msg_id + 1);
			Self::deposit_event(Event::MessageSent { message_id: msg_id, sender: who, payload_hash });
			Ok(())
		}
	}
}
