use super::*;
use core::marker::PhantomData;
use sp_core::{H160, H256, U256};
use fp_evm::weight_per_gas;
use pallet_ethereum::{PostLogContent};
use pallet_evm::{EnsureAddressTruncated, FeeCalculator, HashedAddressMapping, Runner};
use crate::precompiles::FrontierPrecompiles;
use frame_support::traits::FindAuthor;
use sp_runtime::{ConsensusEngineId, Permill};
use sp_runtime::traits::BlakeTwo256;
use codec::Encode;
use crate::{BaseFee, EVMChainId, Timestamp, Balances};
use sp_runtime::Perbill;

const WEIGHT_MILLISECS_PER_BLOCK: u64 = 5000;
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);


impl pallet_evm_chain_id::Config for Runtime {}

pub struct BabeFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for BabeFindAuthor<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authorities = pallet_babe::Authorities::<Runtime>::get();
			if let Some(authority) = authorities.get(author_index as usize) {
				let encoded = authority.0.encode();
				let hash = sp_core::hashing::blake2_256(&encoded);
				return Some(H160::from_slice(&hash[0..20]));
			}
		}
		None
	}
}

const BLOCK_GAS_LIMIT: u64 = 75_000_000;
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
const MAX_STORAGE_GROWTH: u64 = 400 * 1024;

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
	pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
	pub const GasLimitStorageGrowthRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_STORAGE_GROWTH);
	pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_parts(weight_per_gas(BLOCK_GAS_LIMIT, NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK), 0);
}

impl pallet_evm::Config for Runtime {
	type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type PrecompilesType = FrontierPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type OnCreate = ();
	type FindAuthor = BabeFindAuthor<Babe>; // Note: changed to Babe
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
	type Timestamp = Timestamp;
	type CreateOriginFilter = ();
	type CreateInnerOriginFilter = ();
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self::Version>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
	pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}
pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}
impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}
