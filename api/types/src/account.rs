// Copyright © Diem Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{HexEncodedBytes, U64};
use diem_types::account_config::AccountResource;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Account data
///
/// A simplified version of the onchain Account resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: HexEncodedBytes,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        let authentication_key: HexEncodedBytes = ar.authentication_key().to_vec().into();
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key,
        }
    }
}
