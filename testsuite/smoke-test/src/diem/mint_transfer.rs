// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_diem;
use diem_cached_packages::diem_stdlib;
use diem_debugger::DiemDebugger;
use diem_forge::Swarm;
use diem_types::transaction::{ExecutionStatus, TransactionStatus};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_mint_transfer() {
    let mut swarm = new_local_swarm_with_diem(1).await;
    let mut info = swarm.diem_public_info();

    let mut account1 = info.random_account();
    info.create_user_account(account1.public_key())
        .await
        .unwrap();
    let account2 = info.random_account();
    info.create_user_account(account2.public_key())
        .await
        .unwrap();

    // NOTE(Gas): For some reason, there needs to be a lot of funds in the account in order for the
    //            test to pass.
    //            Is this caused by us increasing the default max gas amount in
    //            testsuite/forge/src/interface/diem.rs?
    info.mint(account1.address(), 100_000_000_000)
        .await
        .unwrap();

    let transfer_txn = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(diem_stdlib::diem_coin_transfer(account2.address(), 40000)),
    );
    info.client().submit_and_wait(&transfer_txn).await.unwrap();
    assert_eq!(
        info.client()
            .get_account_balance(account2.address())
            .await
            .unwrap()
            .into_inner()
            .get(),
        40000
    );

    // test delegation
    let txn_factory = info.transaction_factory();
    let delegate_txn1 = info
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(
            diem_stdlib::diem_coin_delegate_mint_capability(account1.address()),
        ));
    info.client().submit_and_wait(&delegate_txn1).await.unwrap();

    // Test delegating more than one at a time: faucet startup stampeding herd
    let delegate_txn2 = info
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(
            diem_stdlib::diem_coin_delegate_mint_capability(account2.address()),
        ));
    info.client().submit_and_wait(&delegate_txn2).await.unwrap();

    let claim_txn = account1.sign_with_transaction_builder(
        txn_factory.payload(diem_stdlib::diem_coin_claim_mint_capability()),
    );
    info.client().submit_and_wait(&claim_txn).await.unwrap();
    let mint_txn = account1.sign_with_transaction_builder(
        txn_factory.payload(diem_stdlib::diem_coin_mint(account1.address(), 10000)),
    );
    info.client().submit_and_wait(&mint_txn).await.unwrap();

    // Testing the DiemDebugger by reexecuting the transaction that has been published.
    println!("Testing....");
    let debugger = DiemDebugger::rest_client(info.client().clone()).unwrap();

    let txn_ver = debugger
        .get_version_by_account_sequence(account1.address(), 0)
        .await
        .unwrap()
        .unwrap();

    let output = debugger
        .execute_past_transactions(txn_ver, 1)
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
}
