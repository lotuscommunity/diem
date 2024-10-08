// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_single_node_test_config, network};
use diem_config::config::{NodeConfig, WaypointConfig};
use diem_event_notifications::EventSubscriptionService;
use diem_infallible::RwLock;
use diem_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use diem_temppath::TempPath;
use diem_types::{
    chain_id::ChainId, on_chain_config::ON_CHAIN_CONFIG_REGISTRY, waypoint::Waypoint,
};
use std::{fs, sync::Arc};

/// A mock database implementing DbReader and DbWriter
pub struct MockDatabase;
impl DbReader for MockDatabase {}
impl DbWriter for MockDatabase {}

#[test]
#[should_panic(expected = "Validator networks must always have mutual_authentication enabled!")]
fn test_mutual_authentication_validators() {
    // Create a default node config for the validator
    let temp_path = TempPath::new();
    let mut node_config = NodeConfig::get_default_validator_config();
    node_config.set_data_dir(temp_path.path().to_path_buf());
    node_config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());

    // Disable mutual authentication for the config
    let validator_network = node_config.validator_network.as_mut().unwrap();
    validator_network.mutual_authentication = false;

    // Create an event subscription service
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(DbReaderWriter::new(MockDatabase {}))),
    );

    // Set up the networks and gather the application network handles. This should panic.
    let peers_and_metadata = network::create_peers_and_metadata(&node_config);
    let _ = network::setup_networks_and_get_interfaces(
        &node_config,
        ChainId::test(),
        peers_and_metadata,
        &mut event_subscription_service,
    );
}

#[cfg(feature = "check-vm-features")]
#[test]
fn test_diem_vm_does_not_have_test_natives() {
    diem_vm::natives::assert_no_test_natives(crate::utils::ERROR_MSG_BAD_FEATURE_FLAGS)
}

#[test]
fn test_create_single_node_test_config() {
    // create a test config override and merge it with the default config
    // this will get cleaned up by the tempdir when it goes out of scope
    let test_dir = diem_temppath::TempPath::new().as_ref().to_path_buf();
    fs::DirBuilder::new()
        .recursive(true)
        .create(&test_dir)
        .expect("Failed to create test_dir");
    let config_override_path = test_dir.join("override.yaml");
    let config_override: serde_yaml::Value = serde_yaml::from_str(
        r#"
        storage:
            enable_indexer: true
        indexer_grpc:
            enabled: true
            address: 0.0.0.0:50053
            processor_task_count: 10
            processor_batch_size: 100
            output_batch_size: 100
        api:
            address: 0.0.0.0:8081
        "#,
    )
    .unwrap();
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&config_override_path)
        .expect("Couldn't open file");
    serde_yaml::to_writer(f, &config_override).unwrap();

    // merge it
    let default_node_config = NodeConfig::get_default_validator_config();
    let merged_config =
        create_single_node_test_config(None, Some(config_override_path), false).unwrap();

    // overriden configs
    assert!(merged_config.storage.enable_indexer);
    assert!(merged_config.indexer_grpc.enabled);
    // default config is unchanged
    assert_eq!(
        merged_config
            .state_sync
            .state_sync_driver
            .bootstrapping_mode,
        default_node_config
            .state_sync
            .state_sync_driver
            .bootstrapping_mode
    );
}
