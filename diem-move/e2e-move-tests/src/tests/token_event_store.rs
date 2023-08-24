// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use diem_cached_packages::diem_stdlib::diem_token_stdlib;

#[test]
fn test_token_creation_with_token_events_store() {
    let mut h = MoveHarness::new();

    // Deploy a package that initially does not have the module that has the init_module function.
    let acc = h.diem_framework_account();
    let token_owner = acc.address();
    let collection_name = b"collection_name".to_vec();
    let token_name = b"token_name".to_vec();
    // create the collection and token

    h.run_transaction_payload(
        &acc,
        diem_token_stdlib::token_create_collection_script(
            collection_name.clone(),
            "collection description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
            100000,
            vec![false, false, false],
        ),
    );
    h.run_transaction_payload(
        &acc,
        diem_token_stdlib::token_create_token_script(
            collection_name,
            token_name,
            "collection description".to_owned().into_bytes(),
            10,
            u64::MAX,
            "uri".to_owned().into_bytes(),
            *token_owner,
            0,
            0,
            vec![false, false, false, false, true],
            vec![Vec::new()],
            vec![Vec::new()],
            vec![Vec::new()],
        ),
    );

    // mutate the token properties
    let signed_txn =
        h.create_transaction_payload(&acc, diem_token_stdlib::token_opt_in_direct_transfer(true));
    let (_, mut events) = h.run_with_events(signed_txn);
    let event = events.pop().unwrap();
    assert_eq!(
        "0x3::token_event_store::OptInTransferEvent".to_string(),
        event.type_tag().to_string()
    );
}
