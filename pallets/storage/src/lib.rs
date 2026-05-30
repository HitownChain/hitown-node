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
		/// 运行时事件类型 / The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		
		/// 货币类型，用于处理存储费用的支付 / The currency trait
		type Currency: ReservableCurrency<Self::AccountId>;
		
		/// 存储节点奖金池的 Pallet ID / The Pallet ID of the Storage Pool
		#[pallet::constant]
		type StoragePoolId: Get<frame_support::PalletId>;
		
		/// 权重到费用的转换 / Convert weight to fee
		type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;
		
		/// 长度到费用的转换 / Convert length to fee
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
		/// 注册成为存储节点 / Register as a storage provider
		/// 
		/// # 参数 / Arguments
		/// * `origin` - 交易发起者 / The caller
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
		pub fn register_provider(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Providers::<T>::contains_key(&who), Error::<T>::ProviderAlreadyRegistered);
			Providers::<T>::insert(&who, 0u64);
			Self::deposit_event(Event::ProviderRegistered { provider: who });
			Ok(())
		}

		/// 创建存储订单并支付 10 倍 GAS 的存储费 / Create a storage order and pay 10x GAS as storage fee
		/// 
		/// # 参数 / Arguments
		/// * `origin` - 交易发起者 / The caller
		/// * `cid` - 文件的 IPFS CID 或其他哈希标识 / The IPFS CID or file hash
		/// * `size` - 文件大小（字节） / The size of the file in bytes
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
		pub fn create_order(origin: OriginFor<T>, cid: Vec<u8>, size: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Orders::<T>::contains_key(&cid), Error::<T>::OrderAlreadyExists);
			
			// 计算当前交易权重的基础费用 / Calculate Base Fee for this transaction weight
			let current_weight = Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1);
			
			let len = cid.len() as u32 + 8; // approx size
			
			// 使用配置的 WeightToFee 计算实际费用 / Compute the actual fee using the configured WeightToFee
			let weight_fee = T::WeightToFee::weight_to_fee(&current_weight);
			let length_fee = T::LengthToFee::weight_to_fee(&Weight::from_parts(len as u64, 0));
			let base_fee = T::WeightToFee::weight_to_fee(&T::BlockWeights::get().base_block);
			
			// 转换为 Pallet 的 BalanceOf 类型 / Convert these fees to the pallet's BalanceOf type
			let total_base_gas: BalanceOf<T> = weight_fee.saturating_add(length_fee).saturating_add(base_fee);
			
			// 存储费设定为 GAS 费的 10 倍 / Set storage fee to 10x the GAS fee
			let surcharge = total_base_gas.saturating_mul(10u32.into());
			
			// 将附加费转移到存储节点奖金池 / Transfer surcharge to Storage Pool
			let pool_account = T::StoragePoolId::get().into_account_truncating();
			T::Currency::transfer(&who, &pool_account, surcharge, frame_support::traits::ExistenceRequirement::KeepAlive)?;

			Orders::<T>::insert(&cid, size);
			Self::deposit_event(Event::OrderCreated { cid, size });
			Ok(())
		}

		/// 提交存储证明 / Submit a proof of storage
		/// 
		/// # 参数 / Arguments
		/// * `origin` - 存储节点 / The storage provider
		/// * `cid` - 对应订单的文件哈希 / The IPFS CID or file hash
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 1))]
		pub fn submit_proof(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Providers::<T>::contains_key(&who), Error::<T>::ProviderNotRegistered);
			ensure!(Orders::<T>::contains_key(&cid), Error::<T>::OrderNotFound);
			
			// 简单的模拟逻辑：直接累加该节点的有效存储算力 / Simple logic: accumulate effective storage power
			let size = Orders::<T>::get(&cid).unwrap_or(0);
			Providers::<T>::mutate(&who, |val| *val = val.saturating_add(size));
			
			Self::deposit_event(Event::ProofSubmitted { provider: who, cid });
			Ok(())
		}
	}
}
