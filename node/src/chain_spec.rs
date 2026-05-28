use sc_service::ChainType;
use solochain_template_runtime::WASM_BINARY;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

pub fn testnet_properties() -> sc_service::Properties {
        let mut properties = sc_service::Properties::new();
        properties.insert("tokenSymbol".into(), "HAN".into());
        properties.insert("tokenName".into(), "Hitown Asset Nexus".into());
        properties.insert("tokenDecimals".into(), 18.into());
        properties.insert("ss58Format".into(), 42.into());
        properties
}

pub fn development_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("HitownChain Testnet")
	.with_id("hitown_testnet")
	.with_chain_type(ChainType::Development)
	.with_properties(testnet_properties())
	.with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
	.build())
}

pub fn local_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("HitownChain Testnet")
	.with_id("hitown_testnet")
	.with_chain_type(ChainType::Local)
	.with_properties(testnet_properties())
	.with_genesis_config_preset_name(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.build())
}
