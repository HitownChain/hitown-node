//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use futures::FutureExt;
use sc_client_api::{Backend, BlockBackend};
use sc_consensus_babe::{BabeBlockImport, BabeLink, BabeWorkerHandle, SlotProportion};
use sc_consensus_grandpa::SharedVoterState;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager, WarpSyncConfig};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use solochain_template_runtime::{self, apis::RuntimeApi, opaque::Block};
use std::{sync::Arc, time::Duration, path::Path};

#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
);

#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
);

pub type FullClient = sc_service::TFullClient<
	Block,
	RuntimeApi,
	sc_executor::WasmExecutor<HostFunctions>,
>;
pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

pub type Service = sc_service::PartialComponents<
	FullClient,
	FullBackend,
	FullSelectChain,
	sc_consensus::DefaultImportQueue<Block>,
	sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
	(
		sc_consensus_babe::BabeBlockImport<Block, FullClient, sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>>,
		sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
		BabeLink<Block>,
		BabeWorkerHandle<Block>,
		Option<Telemetry>,
	),
>;

pub fn new_partial(config: &Configuration) -> Result<Service, ServiceError> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_wasm_executor::<HostFunctions>(&config.executor);
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::configuration(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	let cidp_client = client.clone();
	let (import_queue, babe_worker_handle) =
		sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: block_import.clone(),
			justification_import: Some(Box::new(grandpa_block_import)),
			client: client.clone(),
			select_chain: select_chain.clone(),
			create_inherent_data_providers: move |_, ()| {
				let cidp_client = cidp_client.clone();
				async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							sc_consensus_babe::configuration(&*cidp_client)?.slot_duration(),
						);

					Ok((slot, timestamp))
				}
			},
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: sc_transaction_pool_api::OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		})?;

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (block_import, grandpa_link, babe_link, babe_worker_handle, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full<
	N: sc_network::NetworkBackend<Block, <Block as sp_runtime::traits::Block>::Hash>,
>(
	config: Configuration,
	eth_config: crate::eth::EthConfiguration,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (block_import, grandpa_link, babe_link, babe_worker_handle, mut telemetry),
	} = new_partial(&config)?;

	let mut net_config = sc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as sp_runtime::traits::Block>::Hash,
		N,
	>::new(&config.network, config.prometheus_registry().cloned());
	let metrics = N::register_notification_metrics(config.prometheus_registry());

	let peer_store_handle = net_config.peer_store_handle();
	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config::<_, N>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			peer_store_handle,
		);
	net_config.add_notification_protocol(grandpa_protocol_config);

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	if config.offchain_worker.enabled {
		let offchain_workers =
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: config.role.is_authority(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
		);
	}

	let role = config.role;
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	
	let is_authority = config.role.is_authority();

	let storage_override = Arc::new(fc_storage::StorageOverrideHandler::new(client.clone()));

	let frontier_backend = match eth_config.frontier_backend_type {
		crate::eth::BackendType::KeyValue => Arc::new(fc_db::Backend::KeyValue(
			Arc::new(fc_db::kv::Backend::open(
				Arc::clone(&client),
				&config.database,
				&crate::eth::db_config_dir(&config),
			)
			.map_err(|err| sc_service::Error::Other(err.to_string()))?),
		)),
		crate::eth::BackendType::Sql => {
			let db_path = crate::eth::db_config_dir(&config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///").join(db_path).join("frontier.db3").to_str().unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				storage_override.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			Arc::new(fc_db::Backend::Sql(Arc::new(backend)))
		}
	};

	let crate::eth::FrontierPartialComponents {
		filter_pool,
		fee_history_cache,
		fee_history_cache_limit,
	} = crate::eth::new_frontier_partial(&eth_config)?;

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		storage_override.clone(),
		eth_config.eth_log_block_cache,
		eth_config.eth_statuses_cache,
		prometheus_registry.clone(),
	));

	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	let subscription_task_executor: sc_rpc::SubscriptionTaskExecutor = Arc::new(task_manager.spawn_handle());

let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let sync = sync_service.clone();
		let is_authority = config.role.is_authority();
		let enable_dev_signer = eth_config.enable_dev_signer;
		let max_past_logs = eth_config.max_past_logs;
		let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = block_data_cache.clone();
		let storage_override = storage_override.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();

		Box::new(move |subscription_task_executor| {
			let eth_deps = crate::rpc::EthDeps {
				client: client.clone(),
				pool: pool.clone(),
				graph: pool.clone(),
				converter: Some(solochain_template_runtime::TransactionConverter::<Block>::default()),
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				sync: sync.clone(),
				frontier_backend: match &*frontier_backend {
					fc_db::Backend::KeyValue(b) => b.clone(),
					fc_db::Backend::Sql(b) => b.clone(),
				},
				storage_override: storage_override.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers: move |_, ()| async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
					// dummy slot for pending
					let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						sp_consensus_babe::SlotDuration::from_millis(5000),
					);
					Ok((slot, timestamp))
				},
			};

			let deps = crate::rpc::FullDeps { 
				client: client.clone(), 
				pool: pool.clone(),
				eth: eth_deps,
			};
			crate::rpc::create_full(deps, subscription_task_executor, pubsub_notification_sinks.clone()).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: Arc::new(network.clone()),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend: backend.clone(),
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		config,
		telemetry: telemetry.as_mut(),
	})?;

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer_factory,
			block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: {
				let cidp_client = client.clone();
				move |_, ()| {
					let cidp_client = cidp_client.clone();
					async move {
						let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

						let slot =
							sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
								*timestamp,
								sc_consensus_babe::configuration(&*cidp_client)?.slot_duration(),
							);

						Ok((slot, timestamp))
					}
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("babe", Some("block-authoring"), babe);
	}

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

		let grandpa_config = sc_consensus_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: sync_service.clone(),
			notification_service: grandpa_notification_service,
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	task_manager.keep_alive(babe_worker_handle);

	crate::eth::spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend.clone(),
		frontier_backend,
		filter_pool,
		storage_override,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	);

	Ok(task_manager)
}

