#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod apis;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
pub mod configs;
pub mod precompiles;

extern crate alloc;
use alloc::vec::Vec;
use frame_support::pallet_prelude::ConstU32;
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiAddress, MultiSignature,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub mod genesis_config_presets;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.

use sp_core::{H160, H256, U256, OpaqueMetadata};
use fp_rpc::TransactionStatus;
use pallet_ethereum::{PostLogContent, Transaction as EthereumTransaction};
use codec::{Encode, Decode};
use fp_evm::Account as EVMAccount;
use core::marker::PhantomData;
use sp_runtime::traits::{Block as BlockT, DispatchInfoOf, Dispatchable, PostDispatchInfoOf};
use sp_runtime::transaction_validity::{TransactionValidity, TransactionValidityError};

pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub grandpa: Grandpa,
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("solochain-template-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("solochain-template-runtime"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

mod block_times {
	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLI_SECS_PER_BLOCK: u64 = 5000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	// Attempting to do so will brick block production.
	pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;
	
	// Epoch length 4320 slots (6 hours)
	pub const EPOCH_DURATION_IN_SLOTS: u64 = 4320;
}
pub use block_times::*;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLI_SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const BLOCK_HASH_COUNT: BlockNumber = 2400;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000_000_000;
pub const MILLI_UNIT: Balance = 1_000_000_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000_000_000;

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;

/// SS58 Prefix configuration.
/// See https://docs.substrate.io/reference/address-formats/
pub const SS58_PREFIX: u16 = 42;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The `TransactionExtension` to the basic transaction logic.
pub type TxExtension = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
	frame_system::WeightReclaim<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

/// All migrations of the runtime, aside from the ones declared in the pallets.
///
/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
#[allow(unused_parens)]
type Migrations = ();

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(3)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(4)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(5)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(6)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(7)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(17)]
	pub type Session = pallet_session;

		// Include the custom logic from the pallet-template in the runtime.
		#[runtime::pallet_index(8)]
		pub type Template = pallet_template;
		#[runtime::pallet_index(9)]
		pub type Hpos = pallet_hpos;
		#[runtime::pallet_index(10)]
		pub type Hmp = pallet_hmp;
		#[runtime::pallet_index(11)]
		pub type Did = pallet_did;
		#[runtime::pallet_index(12)]
		pub type Pol = pallet_pol;
		#[runtime::pallet_index(13)]
		pub type HanToken = pallet_han_token;
		#[runtime::pallet_index(14)]
		pub type EvmMapping = pallet_evm_mapping;
		#[runtime::pallet_index(15)]
		pub type Governance = pallet_governance;
		#[runtime::pallet_index(16)]
		pub type WasmBridge = pallet_wasm_bridge;
		#[runtime::pallet_index(24)]
		pub type Storage = pallet_storage;
		#[runtime::pallet_index(18)]
		pub type Ethereum = pallet_ethereum;
		#[runtime::pallet_index(19)]
		pub type EVM = pallet_evm;
		#[runtime::pallet_index(20)]
		pub type EVMChainId = pallet_evm_chain_id;
		#[runtime::pallet_index(21)]
		pub type BaseFee = pallet_base_fee;
		#[runtime::pallet_index(22)]
		pub type DynamicFee = pallet_dynamic_fee;
		#[runtime::pallet_index(23)]
		pub type Authorship = pallet_authorship;
	}

impl pallet_hpos::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_hpos::weights::SubstrateWeight<Runtime>;
	type MaxGenesisValidators = ConstU32<33>;
	type Currency = Balances;
}

impl pallet_hmp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_hmp::weights::SubstrateWeight<Runtime>;
}

impl pallet_did::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_did::weights::SubstrateWeight<Runtime>;
}

impl pallet_pol::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_pol::weights::SubstrateWeight<Runtime>;
}

frame_support::parameter_types! {
	pub const BlockRewardAmount: Balance = 32 * UNIT; // 32 HAN per block
}

pub struct SessionValidators;
impl frame_support::traits::Get<Vec<AccountId>> for SessionValidators {
	fn get() -> Vec<AccountId> {
		pallet_session::Validators::<Runtime>::get()
	}
}

impl pallet_han_token::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_han_token::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type BlockReward = BlockRewardAmount;
	type ActiveValidators = SessionValidators;
}

impl pallet_evm_mapping::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_evm_mapping::weights::SubstrateWeight<Runtime>;
}

frame_support::parameter_types! {
	pub const ProposalBondAmount: Balance = 100 * UNIT; // 100 HAN to propose
}

impl pallet_governance::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_governance::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type ProposalBond = ProposalBondAmount;
}

impl pallet_wasm_bridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_wasm_bridge::weights::SubstrateWeight<Runtime>;
}

frame_support::parameter_types! {
	pub const StoragePoolId: frame_support::PalletId = frame_support::PalletId(*b"ht/stora");
}

impl pallet_storage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type StoragePoolId = StoragePoolId;
	type WeightToFee = frame_support::weights::IdentityFee<Balance>;
	type LengthToFee = frame_support::weights::IdentityFee<Balance>;
}

#[derive(Clone)]
pub struct TransactionConverter<B>(PhantomData<B>);

impl<B> Default for TransactionConverter<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: BlockT> fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> for TransactionConverter<B> {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> <B as BlockT>::Extrinsic {
		let extrinsic = UncheckedExtrinsic::new_bare(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		<B as BlockT>::Extrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => {
				call.pre_dispatch_self_contained(info, dispatch_info, len)
			}
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
				Some(call.dispatch(RuntimeOrigin::from(
					pallet_ethereum::RawOrigin::EthereumTransaction(info),
				)))
			}
			_ => None,
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[pallet_evm, EVM]
	);
}

