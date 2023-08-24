// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{golden_output::GoldenOutputs, pretty};
use diem_api::{attach_poem_to_runtime, BasicError, Context};
use diem_api_types::{
    mime_types, HexEncodedBytes, TransactionOnChainData, X_DIEM_CHAIN_ID,
    X_DIEM_LEDGER_TIMESTAMP, X_DIEM_LEDGER_VERSION,
};
use diem_cached_packages::diem_stdlib;
use diem_config::{
    config::{
        NodeConfig, RocksdbConfigs, BUFFERED_STATE_TARGET_ITEMS,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    },
    keys::ConfigKey,
};
use diem_crypto::{ed25519::Ed25519PrivateKey, hash::HashValue, SigningKey};
use diem_db::DiemDB;
use diem_executor::{block_executor::BlockExecutor, db_bootstrapper};
use diem_executor_types::BlockExecutorTrait;
use diem_framework::BuiltPackage;
use diem_mempool::mocks::MockSharedMempool;
use diem_mempool_notifications::MempoolNotificationSender;
use diem_sdk::{
    bcs,
    transaction_builder::TransactionFactory,
    types::{
        account_config::diem_test_root_address, transaction::SignedTransaction, AccountKey,
        LocalAccount,
    },
};
use diem_storage_interface::{state_view::DbStateView, DbReaderWriter};
use diem_temppath::TempPath;
use diem_types::{
    account_address::{create_multisig_account_address, AccountAddress},
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{Transaction, TransactionPayload, TransactionStatus},
};
use diem_vm::DiemVM;
use diem_vm_validator::vm_validator::VMValidator;
use bytes::Bytes;
use hyper::{HeaderMap, Response};
use rand::SeedableRng;
use serde_json::{json, Value};
use std::{boxed::Box, iter::once, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use warp::{http::header::CONTENT_TYPE, Filter, Rejection, Reply};
use warp_reverse_proxy::reverse_proxy_filter;

const TRANSFER_AMOUNT: u64 = 10_000_000;

#[derive(Clone, Debug)]
pub enum ApiSpecificConfig {
    // The SocketAddr is the address where the Poem backend is running.
    V1(SocketAddr),
}

impl ApiSpecificConfig {
    pub fn get_api_base_path(&self) -> String {
        match &self {
            ApiSpecificConfig::V1(_) => "/v1".to_string(),
        }
    }

    pub fn assert_content_type(&self, headers: &HeaderMap) {
        match &self {
            ApiSpecificConfig::V1(_) => assert!(headers[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with(mime_types::JSON),),
        }
    }

    pub fn signing_message_endpoint(&self) -> &'static str {
        match &self {
            ApiSpecificConfig::V1(_) => "/transactions/encode_submission",
        }
    }

    pub fn unwrap_signing_message_response(&self, resp: serde_json::Value) -> HexEncodedBytes {
        match self {
            ApiSpecificConfig::V1(_) => resp.as_str().unwrap().parse().unwrap(),
        }
    }
}

pub fn new_test_context(
    test_name: String,
    node_config: NodeConfig,
    use_db_with_indexer: bool,
) -> TestContext {
    // Speculative logging uses a global variable and when many instances use it together, they
    // panic, so we disable this to run tests.
    diem_vm_logging::disable_speculative_logging();
    let tmp_dir = TempPath::new();
    tmp_dir.create_as_dir().unwrap();

    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let builder = diem_genesis::builder::Builder::new(
        tmp_dir.path(),
        diem_cached_packages::head_release_bundle().clone(),
    )
    .unwrap()
    .with_init_genesis_config(Some(Arc::new(|genesis_config| {
        genesis_config.recurring_lockup_duration_secs = 86400;
    })))
    .with_randomize_first_validator_ports(false);

    let (root_key, genesis, genesis_waypoint, validators) = builder.build(&mut rng).unwrap();
    let (validator_identity, _, _, _) = validators[0].get_key_objects(None).unwrap();
    let validator_owner = validator_identity.account_address.unwrap();

    let (db, db_rw) = if use_db_with_indexer {
        DbReaderWriter::wrap(DiemDB::new_for_test_with_indexer(&tmp_dir))
    } else {
        DbReaderWriter::wrap(
            DiemDB::open(
                &tmp_dir,
                false,                       /* readonly */
                NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
                RocksdbConfigs::default(),
                false, /* indexer */
                BUFFERED_STATE_TARGET_ITEMS,
                DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            )
            .unwrap(),
        )
    };
    let ret =
        db_bootstrapper::maybe_bootstrap::<DiemVM>(&db_rw, &genesis, genesis_waypoint).unwrap();
    assert!(ret);

    let mempool = MockSharedMempool::new_in_runtime(&db_rw, VMValidator::new(db.clone()));

    let context = Context::new(
        ChainId::test(),
        db.clone(),
        mempool.ac_client.clone(),
        node_config.clone(),
    );

    // Configure the testing depending on which API version we're testing.
    let runtime_handle = tokio::runtime::Handle::current();
    let poem_address = attach_poem_to_runtime(&runtime_handle, context.clone(), &node_config, true)
        .expect("Failed to attach poem to runtime");
    let api_specific_config = ApiSpecificConfig::V1(poem_address);

    TestContext::new(
        context,
        rng,
        root_key,
        validator_owner,
        Box::new(BlockExecutor::<DiemVM>::new(db_rw)),
        mempool,
        db,
        test_name,
        api_specific_config,
    )
}

#[derive(Clone)]
pub struct TestContext {
    pub context: Context,
    pub validator_owner: AccountAddress,
    pub mempool: Arc<MockSharedMempool>,
    pub db: Arc<DiemDB>,
    rng: rand::rngs::StdRng,
    root_key: ConfigKey<Ed25519PrivateKey>,
    executor: Arc<dyn BlockExecutorTrait>,
    expect_status_code: u16,
    test_name: String,
    golden_output: Option<GoldenOutputs>,
    fake_time_usecs: u64,
    pub api_specific_config: ApiSpecificConfig,
}

impl TestContext {
    pub fn new(
        context: Context,
        rng: rand::rngs::StdRng,
        root_key: Ed25519PrivateKey,
        validator_owner: AccountAddress,
        executor: Box<dyn BlockExecutorTrait>,
        mempool: MockSharedMempool,
        db: Arc<DiemDB>,
        test_name: String,
        api_specific_config: ApiSpecificConfig,
    ) -> Self {
        Self {
            context,
            rng,
            root_key: ConfigKey::new(root_key),
            validator_owner,
            executor: executor.into(),
            mempool: Arc::new(mempool),
            expect_status_code: 200,
            db,
            test_name,
            golden_output: None,
            fake_time_usecs: 0,
            api_specific_config,
        }
    }

    pub fn set_fake_time_usecs(&mut self, fake_time_usecs: u64) {
        self.fake_time_usecs = fake_time_usecs;
    }

    pub fn check_golden_output_no_prune(&mut self, msg: Value) {
        if self.golden_output.is_none() {
            self.golden_output = Some(GoldenOutputs::new(self.test_name.replace(':', "_")));
        }

        self.golden_output.as_ref().unwrap().log(&msg.to_string());
    }

    pub fn check_golden_output(&mut self, msg: Value) {
        if self.golden_output.is_none() {
            self.golden_output = Some(GoldenOutputs::new(self.test_name.replace(':', "_")));
        }

        let msg = pretty(&Self::prune_golden(msg));
        let re = regex::Regex::new("hash\": \".*\"").unwrap();
        let msg = re.replace_all(&msg, "hash\": \"\"");

        self.golden_output.as_ref().unwrap().log(&msg);
    }

    pub fn last_updated_gas_schedule(&self) -> Option<u64> {
        self.context.last_updated_gas_schedule()
    }

    pub fn last_updated_gas_estimation_cache_size(&self) -> usize {
        self.context.last_updated_gas_estimation_cache_size()
    }

    /// Prune well-known excessively large entries from a resource array response.
    /// TODO: we can't dump all resources of an account as golden output. As functionality
    /// grows this becomes too much. Need a way to filter only the resources which folks want.
    fn prune_golden(val: Value) -> Value {
        if !val.is_array() {
            return val;
        }

        val.as_array()
            .unwrap()
            .iter()
            .map(|field| {
                if let Some(changes) = field.as_object().unwrap().get("changes") {
                    let mut nfield = field.clone();
                    nfield["changes"] = changes
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|change| {
                            let mut nchange = change.clone();
                            nchange["data"] = Self::resource_replacer(&change["data"]);
                            nchange
                        })
                        .collect();
                    nfield
                } else {
                    field.clone()
                }
            })
            .collect()
    }

    // Resource may appear in many different places, so make a convenient stripper
    fn resource_replacer(val: &Value) -> Value {
        let mut nval = val.clone();

        // Skip things that change, plus bytecode and others that don't have a type
        nval["data"] = match val["type"].as_str() {
            Some("0x1::code::PackageRegistry") => {
                Value::String("package registry omitted".to_string())
            },
            // Ideally this wouldn't be stripped, but it changes by minor changes to the
            // Move modules, which leads to a bad devx.
            Some("0x1::state_storage::StateStorageUsage") => {
                Value::String("state storage omitted".to_string())
            },
            Some("0x1::state_storage::GasParameter") => {
                Value::String("state storage gas parameter omitted".to_string())
            },
            _ => {
                if val["bytecode"].as_str().is_some() {
                    Value::String("bytecode omitted".to_string())
                } else {
                    val["data"].clone()
                }
            },
        };
        nval
    }

    pub fn rng(&mut self) -> &mut rand::rngs::StdRng {
        &mut self.rng
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.context.chain_id())
    }

    pub async fn root_account(&self) -> LocalAccount {
        // Fetch the actual root account's sequence number in case it has been used to sign
        // transactions before.
        let root_sequence_number = self.get_sequence_number(diem_test_root_address()).await;
        LocalAccount::new(
            diem_test_root_address(),
            self.root_key.private_key(),
            root_sequence_number,
        )
    }

    pub fn latest_state_view(&self) -> DbStateView {
        self.context
            .state_view_at_version(self.get_latest_ledger_info().version())
            .unwrap()
    }

    pub fn gen_account(&mut self) -> LocalAccount {
        LocalAccount::generate(self.rng())
    }

    pub async fn create_account(&mut self) -> LocalAccount {
        let mut root = self.root_account().await;
        let account = self.gen_account();
        let factory = self.transaction_factory();
        let txn = root.sign_with_transaction_builder(
            factory
                .account_transfer(account.address(), TRANSFER_AMOUNT)
                .expiration_timestamp_secs(u64::MAX),
        );

        let bcs_txn = bcs::to_bytes(&txn).unwrap();
        self.expect_status_code(202)
            .post_bcs_txn("/transactions", bcs_txn)
            .await;
        self.commit_mempool_txns(1).await;
        account
    }

    pub async fn create_user_account(&self, account: &LocalAccount) -> SignedTransaction {
        let mut tc = self.root_account().await;
        self.create_user_account_by(&mut tc, account)
    }

    pub async fn mint_user_account(&self, account: &LocalAccount) -> SignedTransaction {
        let mut tc = self.root_account().await;
        let factory = self.transaction_factory();
        tc.sign_with_transaction_builder(
            factory
                .account_transfer(account.address(), TRANSFER_AMOUNT)
                .expiration_timestamp_secs(u64::MAX),
        )
    }

    pub async fn execute_multisig_transaction(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        expected_status_code: u16,
    ) {
        self.api_execute_txn_expecting(
            owner,
            json!({
                "type": "multisig_payload",
                "multisig_address": multisig_account.to_hex_literal(),
            }),
            expected_status_code,
        )
        .await;
    }

    pub async fn execute_multisig_transaction_with_payload(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        function: &str,
        type_args: &[&str],
        args: &[&str],
        expected_status_code: u16,
    ) {
        self.api_execute_txn_expecting(
            owner,
            json!({
                "type": "multisig_payload",
                "multisig_address": multisig_account.to_hex_literal(),
                "transaction_payload": {
                    "function": function,
                    "type_arguments": type_args,
                    "arguments": args
                }
            }),
            expected_status_code,
        )
        .await;
    }

    pub async fn create_multisig_account(
        &mut self,
        account: &mut LocalAccount,
        additional_owners: Vec<AccountAddress>,
        signatures_required: u64,
        initial_balance: u64,
    ) -> AccountAddress {
        let factory = self.transaction_factory();
        let multisig_address =
            create_multisig_account_address(account.address(), account.sequence_number());
        let create_multisig_txn = account.sign_with_transaction_builder(
            factory
                .create_multisig_account(additional_owners, signatures_required)
                .expiration_timestamp_secs(u64::MAX),
        );
        self.commit_block(&vec![
            create_multisig_txn,
            self.account_transfer_to(account, multisig_address, initial_balance),
        ])
        .await;
        multisig_address
    }

    pub async fn create_multisig_transaction(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) {
        let factory = self.transaction_factory();
        let txn = owner.sign_with_transaction_builder(
            factory
                .create_multisig_transaction(multisig_account, payload)
                .expiration_timestamp_secs(u64::MAX),
        );
        self.commit_block(&vec![txn]).await;
    }

    pub async fn approve_multisig_transaction(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) {
        let factory = self.transaction_factory();
        let txn = owner.sign_with_transaction_builder(
            factory
                .approve_multisig_transaction(multisig_account, transaction_id)
                .expiration_timestamp_secs(u64::MAX),
        );
        self.commit_block(&vec![txn]).await;
    }

    pub async fn reject_multisig_transaction(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) {
        let factory = self.transaction_factory();
        let txn = owner.sign_with_transaction_builder(
            factory
                .reject_multisig_transaction(multisig_account, transaction_id)
                .expiration_timestamp_secs(u64::MAX),
        );
        self.commit_block(&vec![txn]).await;
    }

    pub async fn create_multisig_transaction_with_payload_hash(
        &mut self,
        owner: &mut LocalAccount,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) {
        let factory = self.transaction_factory();
        let txn = owner.sign_with_transaction_builder(
            factory
                .create_multisig_transaction_with_payload_hash(multisig_account, payload)
                .expiration_timestamp_secs(u64::MAX),
        );
        self.commit_block(&vec![txn]).await;
    }

    pub fn account_transfer(
        &self,
        sender: &mut LocalAccount,
        receiver: &LocalAccount,
        amount: u64,
    ) -> SignedTransaction {
        self.account_transfer_to(sender, receiver.address(), amount)
    }

    pub fn account_transfer_to(
        &self,
        sender: &mut LocalAccount,
        receiver: AccountAddress,
        amount: u64,
    ) -> SignedTransaction {
        let factory = self.transaction_factory();
        sender.sign_with_transaction_builder(
            factory
                .account_transfer(receiver, amount)
                .expiration_timestamp_secs(u64::MAX),
        )
    }

    pub fn create_user_account_by(
        &self,
        creator: &mut LocalAccount,
        account: &LocalAccount,
    ) -> SignedTransaction {
        let factory = self.transaction_factory();
        creator.sign_with_transaction_builder(
            factory
                .create_user_account(account.public_key())
                .expiration_timestamp_secs(u64::MAX),
        )
    }

    pub async fn create_invalid_signature_transaction(&mut self) -> SignedTransaction {
        let factory = self.transaction_factory();
        let root_account = self.root_account().await;
        let txn = factory
            .transfer(root_account.address(), 1)
            .sender(root_account.address())
            .sequence_number(root_account.sequence_number())
            .build();
        let invalid_key = AccountKey::generate(self.rng());
        txn.sign(invalid_key.private_key(), root_account.public_key().clone())
            .unwrap()
            .into_inner()
    }

    pub fn get_latest_ledger_info(&self) -> diem_api_types::LedgerInfo {
        self.context.get_latest_ledger_info::<BasicError>().unwrap()
    }

    pub fn get_transactions(&self, start: u64, limit: u16) -> Vec<TransactionOnChainData> {
        self.context
            .get_transactions(start, limit, self.get_latest_ledger_info().version())
            .unwrap()
    }

    pub fn expect_status_code(&self, status_code: u16) -> TestContext {
        let mut ret = self.clone();
        ret.expect_status_code = status_code;
        ret
    }

    pub fn build_package(
        path: PathBuf,
        named_addresses: Vec<(String, AccountAddress)>,
    ) -> TransactionPayload {
        let mut build_options = diem_framework::BuildOptions::default();
        named_addresses.into_iter().for_each(|(name, address)| {
            build_options.named_addresses.insert(name, address);
        });

        let package = BuiltPackage::build(path, build_options).unwrap();
        let code = package.extract_code();
        let metadata = package.extract_metadata().unwrap();

        diem_stdlib::code_publish_package_txn(bcs::to_bytes(&metadata).unwrap(), code)
    }

    pub async fn publish_package(
        &mut self,
        publisher: &mut LocalAccount,
        payload: TransactionPayload,
    ) -> SignedTransaction {
        let txn =
            publisher.sign_with_transaction_builder(self.transaction_factory().payload(payload));
        let bcs_txn = bcs::to_bytes(&txn).unwrap();
        self.expect_status_code(202)
            .post_bcs_txn("/transactions", bcs_txn)
            .await;
        self.commit_mempool_txns(1).await;
        txn
    }

    pub async fn commit_mempool_txns(&mut self, size: u64) {
        let txns = self.mempool.get_txns(size);
        self.commit_block(&txns).await;
        for txn in txns {
            self.mempool.remove_txn(&txn);
        }
    }

    pub async fn commit_block(&mut self, signed_txns: &[SignedTransaction]) {
        let metadata = self.new_block_metadata();
        let timestamp = metadata.timestamp_usecs();
        let txns: Vec<Transaction> = std::iter::once(Transaction::BlockMetadata(metadata.clone()))
            .chain(
                signed_txns
                    .iter()
                    .cloned()
                    .map(Transaction::UserTransaction),
            )
            .chain(once(Transaction::StateCheckpoint(metadata.id())))
            .collect();

        // Check that txn execution was successful.
        let parent_id = self.executor.committed_block_id();
        let result = self
            .executor
            .execute_block((metadata.id(), txns.clone()).into(), parent_id, None)
            .unwrap();
        let mut compute_status = result.compute_status().clone();
        assert_eq!(compute_status.len(), txns.len(), "{:?}", result);
        if matches!(compute_status.last(), Some(TransactionStatus::Retry)) {
            // a state checkpoint txn can be Retry if prefixed by a write set txn
            compute_status.pop();
        }
        // But the rest of the txns must be Kept.
        for st in result.compute_status() {
            match st {
                TransactionStatus::Discard(st) => panic!("transaction is discarded: {:?}", st),
                TransactionStatus::Retry => panic!("should not retry"),
                TransactionStatus::Keep(_) => (),
            }
        }

        self.executor
            .commit_blocks(
                vec![metadata.id()],
                self.new_ledger_info(&metadata, result.root_hash(), txns.len()),
            )
            .unwrap();

        self.mempool
            .mempool_notifier
            .notify_new_commit(txns, timestamp, 1000)
            .await
            .unwrap();
    }

    pub async fn get_sequence_number(&self, account: AccountAddress) -> u64 {
        let account_resource = self
            .gen_resource(&account, "0x1::account::Account")
            .await
            .unwrap();
        account_resource["data"]["sequence_number"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    }

    pub async fn get_apt_balance(&self, account: AccountAddress) -> u64 {
        let coin_balance = self
            .api_get_account_resource(
                account,
                "0x1",
                "coin",
                "CoinStore<0x1::diem_coin::DiemCoin>",
            )
            .await;
        coin_balance["data"]["coin"]["value"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    }

    pub async fn gen_events_by_handle(
        &self,
        account_address: &AccountAddress,
        resource: &str,
        field_name: &str,
    ) -> Value {
        let request = format!(
            "/accounts/{}/events/{}/{}",
            account_address, resource, field_name
        );
        self.get(&request).await
    }

    pub async fn gen_events_by_creation_num(
        &self,
        account_address: &AccountAddress,
        creation_num: u64,
    ) -> Value {
        let request = format!("/accounts/{}/events/{}", account_address, creation_num);
        self.get(&request).await
    }

    // return a specific resource for an account. None if not found.
    pub async fn gen_resource(
        &self,
        account_address: &AccountAddress,
        resource: &str,
    ) -> Option<Value> {
        let request = format!("/accounts/{}/resources", account_address);
        let response = self.get(&request).await;
        response
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["type"] == resource)
            .cloned()
    }

    // return all resources for an account
    pub async fn gen_all_resources(&self, account_address: &AccountAddress) -> Value {
        let request = format!("/accounts/{}/resources", account_address);
        self.get(&request).await
    }

    // TODO: Add support for generic_type_params if necessary.
    pub async fn api_get_account_resource(
        &self,
        account: AccountAddress,
        resource_account_address: &str,
        module: &str,
        name: &str,
    ) -> serde_json::Value {
        let resource = format!("{}::{}::{}", resource_account_address, module, name);
        self.gen_resource(&account, &resource).await.unwrap()
    }

    // TODO: remove the helper function since we don't publish module directly anymore
    pub async fn api_publish_module(&mut self, account: &mut LocalAccount, code: HexEncodedBytes) {
        self.api_execute_txn(
            account,
            json!({
                "type": "module_bundle_payload",
                "modules" : [
                    {"bytecode": code},
                ],
            }),
        )
        .await;
    }

    pub async fn api_execute_entry_function(
        &mut self,
        account: &mut LocalAccount,
        function: &str,
        type_args: serde_json::Value,
        args: serde_json::Value,
    ) {
        self.api_execute_txn(
            account,
            json!({
                "type": "entry_function_payload",
                "function": function,
                "type_arguments": type_args,
                "arguments": args
            }),
        )
        .await;
    }

    pub async fn api_execute_txn(&mut self, account: &mut LocalAccount, payload: Value) {
        self.api_execute_txn_expecting(account, payload, 202).await;
    }

    pub async fn api_execute_txn_expecting(
        &mut self,
        account: &mut LocalAccount,
        payload: Value,
        status_code: u16,
    ) {
        let mut request = json!({
            "sender": account.address(),
            "sequence_number": account.sequence_number().to_string(),
            "gas_unit_price": "0",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "16373698888888",
            "payload": payload,
        });

        let resp = self
            .post(
                self.api_specific_config.signing_message_endpoint(),
                request.clone(),
            )
            .await;

        let signing_msg = self
            .api_specific_config
            .unwrap_signing_message_response(resp);

        let sig = account
            .private_key()
            .sign_arbitrary_message(signing_msg.inner());

        request["signature"] = json!({
            "type": "ed25519_signature",
            "public_key": HexEncodedBytes::from(account.public_key().to_bytes().to_vec()),
            "signature": HexEncodedBytes::from(sig.to_bytes().to_vec()),
        });

        self.expect_status_code(status_code)
            .post("/transactions", request)
            .await;
        self.commit_mempool_txns(1).await;
        *account.sequence_number_mut() += 1;
    }

    pub async fn simulate_multisig_transaction(
        &mut self,
        owner: &LocalAccount,
        multisig_account: AccountAddress,
        function: &str,
        type_args: &[&str],
        args: &[&str],
        expected_status_code: u16,
    ) -> Value {
        self.simulate_transaction(
            owner,
            json!({
                "type": "multisig_payload",
                "multisig_address": multisig_account.to_hex_literal(),
                "transaction_payload": {
                    "function": function,
                    "type_arguments": type_args,
                    "arguments": args
                }
            }),
            expected_status_code,
        )
        .await
    }

    pub async fn simulate_transaction(
        &mut self,
        sender: &LocalAccount,
        payload: Value,
        status_code: u16,
    ) -> Value {
        let mut request = json!({
            "sender": sender.address(),
            "sequence_number": sender.sequence_number().to_string(),
            "gas_unit_price": "0",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "16373698888888",
            "payload": payload,
        });

        // We're intentionally using invalid signatures since simulation API rejects valid ones.
        let random_account = self.gen_account();
        let resp = self
            .post(
                self.api_specific_config.signing_message_endpoint(),
                request.clone(),
            )
            .await;

        let signing_msg = self
            .api_specific_config
            .unwrap_signing_message_response(resp);

        let sig = random_account
            .private_key()
            .sign_arbitrary_message(signing_msg.inner());
        request["signature"] = json!({
            "type": "ed25519_signature",
            "public_key": HexEncodedBytes::from(sender.public_key().to_bytes().to_vec()),
            "signature": HexEncodedBytes::from(sig.to_bytes().to_vec()),
        });

        self.expect_status_code(status_code)
            .post("/transactions/simulate", request)
            .await
    }

    pub fn prepend_path(&self, path: &str) -> String {
        format!("{}{}", self.api_specific_config.get_api_base_path(), path)
    }

    pub async fn get(&self, path: &str) -> Value {
        self.execute(
            warp::test::request()
                .method("GET")
                .path(&self.prepend_path(path)),
        )
        .await
    }

    pub async fn post(&self, path: &str, body: Value) -> Value {
        self.execute(
            warp::test::request()
                .method("POST")
                .path(&self.prepend_path(path))
                .json(&body),
        )
        .await
    }

    pub async fn post_bcs_txn(&self, path: &str, body: impl AsRef<[u8]>) -> Value {
        self.execute(
            warp::test::request()
                .method("POST")
                .path(&self.prepend_path(path))
                .header(CONTENT_TYPE, mime_types::BCS_SIGNED_TRANSACTION)
                .body(body),
        )
        .await
    }

    pub async fn reply(&self, req: warp::test::RequestBuilder) -> Response<Bytes> {
        match self.api_specific_config {
            ApiSpecificConfig::V1(address) => req.reply(&self.get_routes_with_poem(address)).await,
        }
    }

    // Currently we still run our tests with warp.
    // https://github.com/aptos-labs/diem-core/issues/2966
    pub fn get_routes_with_poem(
        &self,
        poem_address: SocketAddr,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("v1" / ..).and(reverse_proxy_filter(
            "v1".to_string(),
            format!("http://{}/v1", poem_address),
        ))
    }

    pub async fn execute(&self, req: warp::test::RequestBuilder) -> Value {
        let resp = self.reply(req).await;

        let headers = resp.headers();

        self.api_specific_config.assert_content_type(headers);

        let body = serde_json::from_slice(resp.body()).expect("response body is JSON");
        assert_eq!(
            self.expect_status_code,
            resp.status(),
            "\nresponse: {}",
            pretty(&body)
        );

        if self.expect_status_code < 300 {
            let ledger_info = self.get_latest_ledger_info();
            assert_eq!(headers[X_DIEM_CHAIN_ID], "4");
            assert_eq!(
                headers[X_DIEM_LEDGER_VERSION],
                ledger_info.version().to_string()
            );
            assert_eq!(
                headers[X_DIEM_LEDGER_TIMESTAMP],
                ledger_info.timestamp().to_string()
            );
        }

        body
    }

    fn new_block_metadata(&mut self) -> BlockMetadata {
        let round = 1;
        let id = HashValue::random_with_rng(&mut self.rng);
        // Incrementing half a second every time
        self.fake_time_usecs += (Duration::from_millis(500).as_micros()) as u64;
        BlockMetadata::new(
            id,
            1,
            round,
            self.validator_owner,
            vec![0],
            vec![],
            self.fake_time_usecs,
        )
    }

    fn new_ledger_info(
        &self,
        metadata: &BlockMetadata,
        root_hash: HashValue,
        block_size: usize,
    ) -> LedgerInfoWithSignatures {
        let parent = self
            .context
            .get_latest_ledger_info_with_signatures()
            .unwrap();
        let epoch = parent.ledger_info().next_block_epoch();
        let version = parent.ledger_info().version() + (block_size as u64);
        let info = LedgerInfo::new(
            BlockInfo::new(
                epoch,
                metadata.round(),
                metadata.id(),
                root_hash,
                version,
                metadata.timestamp_usecs(),
                None,
            ),
            HashValue::zero(),
        );
        LedgerInfoWithSignatures::new(info, AggregateSignature::empty())
    }
}
