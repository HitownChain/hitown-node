#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::{Currency, ReservableCurrency, Get};
	use sp_runtime::traits::{AccountIdConversion, Saturating};
	extern crate alloc;
	use alloc::vec::Vec;

	use frame_support::weights::WeightToFee;
	
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: ReservableCurrency<Self::AccountId>;
		#[pallet::constant]
		type StoragePoolId: Get<frame_support::PalletId>;
		
		type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;
		type LengthToFee: WeightToFee<Balance = BalanceOf<Self>>;
	}

	#[pallet::storage]
	pub type Providers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::storage]
	pub type Orders<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, u64, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proofs)]
	pub type Proofs<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, Vec<u8>,
		Blake2_128Concat, T::AccountId,
		bool,
		OptionQuery
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		OrderCreated { cid: Vec<u8>, size: u64 },
		ProviderRegistered { provider: T::AccountId },
		ProofSubmitted { provider: T::AccountId, cid: Vec<u8> },
	}

	#[pallet::error]
	pub enum Error<T> {
		OrderAlreadyExists,
		OrderNotFound,
		ProviderAlreadyRegistered,
		ProviderNotRegistered,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
		pub fn register_provider(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Providers::<T>::contains_key(&who), Error::<T>::ProviderAlreadyRegistered);
			Providers::<T>::insert(&who, 0u64);
			Self::deposit_event(Event::ProviderRegistered { provider: who });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
		pub fn create_order(origin: OriginFor<T>, cid: Vec<u8>, size: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Orders::<T>::contains_key(&cid), Error::<T>::OrderAlreadyExists);
			
			// 1. Calculate Base Fee for this transaction weight
			let current_weight = Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1);
			// Assuming WeightToFee and LengthToFee are accessible or we simplify by a fixed ratio
			// We'll use a fixed storage fee logic here based on size as the "10x gas" is an economic model concept.
			// To strictly implement 10x GAS, we interact with pallet_transaction_payment to get the current base fee.
			// Assuming WeightToFee and LengthToFee are correctly configured in runtime, we query the exact fee.
			
			let len = cid.len() as u32 + 8; // approx size
			
			// Compute the actual fee using the configured WeightToFee
			let weight_fee = T::WeightToFee::weight_to_fee(&current_weight);
			let length_fee = T::LengthToFee::weight_to_fee(&Weight::from_parts(len as u64, 0));
			let base_fee = T::WeightToFee::weight_to_fee(&T::BlockWeights::get().base_block);
			
			// We convert these fees to the pallet's BalanceOf type
			// Normally these are already Balance types configured in runtime
			let total_base_gas: BalanceOf<T> = weight_fee.saturating_add(length_fee).saturating_add(base_fee);
			
			let surcharge = total_base_gas.saturating_mul(10u32.into());
			
			// 2. Transfer surcharge to Storage Pool
			let pool_account = T::StoragePoolId::get().into_account_truncating();
			T::Currency::transfer(&who, &pool_account, surcharge, frame_support::traits::ExistenceRequirement::KeepAlive)?;

			Orders::<T>::insert(&cid, size);
			Self::deposit_event(Event::OrderCreated { cid, size });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 1))]
		pub fn submit_proof(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Providers::<T>::contains_key(&who), Error::<T>::ProviderNotRegistered);
			ensure!(Orders::<T>::contains_key(&cid), Error::<T>::OrderNotFound);
			
			// 简单的模拟逻辑：直接累加算力
			let size = Orders::<T>::get(&cid).unwrap_or(0);
			Providers::<T>::mutate(&who, |val| *val = val.saturating_add(size));
			
			Self::deposit_event(Event::ProofSubmitted { provider: who, cid });
			Ok(())
		}
	}
}
