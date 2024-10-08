// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

//! # diem-ledger
//!
//! `diem-ledger` provides convenience methods to communicate with the Diem app on ledger

#![deny(missing_docs)]

pub use diem_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt};
pub use diem_types::{
    account_address::AccountAddress, transaction::authenticator::AuthenticationKey,
};
use hex::encode;
use ledger_apdu::APDUCommand;
use ledger_transport_hid::{hidapi::HidApi, LedgerHIDError, TransportNativeHID};
use std::{
    collections::HashMap,
    fmt,
    fmt::{Debug, Display},
    ops::Range,
    str,
    string::ToString,
};
use thiserror::Error;

// A piece of data which tells a wallet how to derive a specific key within a tree of keys
// 637 is the key for Diem
const DERIVATIVE_PATH: &str = "m/44'/637'/{index}'/0'/0'";

const CLA_DIEM: u8 = 0x5B; // Diem CLA Instruction class
const INS_GET_VERSION: u8 = 0x03; // Get version instruction code
const INS_GET_APP_NAME: u8 = 0x04; // Get app name instruction code
const INS_GET_PUB_KEY: u8 = 0x05; // Get public key instruction code
const INS_SIGN_TXN: u8 = 0x06; // Sign the transaction
const APDU_CODE_SUCCESS: u16 = 36864; // Success code for transport.exchange

const MAX_APDU_LEN: usize = 255;
const P1_NON_CONFIRM: u8 = 0x00;
const P1_CONFIRM: u8 = 0x01;
const P1_START: u8 = 0x00;
const P2_MORE: u8 = 0x80;
const P2_LAST: u8 = 0x00;

#[derive(Debug, Error)]
/// Diem Ledger Error
pub enum DiemLedgerError {
    /// Error when trying to open a connection to the Ledger device
    #[error("Device not found")]
    DeviceNotFound,

    /// Unexpected error, the Option<u16> is the retcode received from ledger transport
    #[error("Unexpected Error: {0} (Retcode {1:?})")]
    UnexpectedError(String, Option<u16>),
}

impl From<LedgerHIDError> for DiemLedgerError {
    fn from(e: LedgerHIDError) -> Self {
        DiemLedgerError::UnexpectedError(e.to_string(), None)
    }
}

/// Diem version in format major.minor.patch
#[derive(Debug)]
pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Returns the current version of the Diem app on Ledger
pub fn get_app_version() -> Result<Version, DiemLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    match transport.exchange(&APDUCommand {
        cla: CLA_DIEM,
        ins: INS_GET_VERSION,
        p1: P1_NON_CONFIRM,
        p2: P2_LAST,
        data: vec![],
    }) {
        Ok(response) => {
            // Received response from Ledger
            if response.retcode() == APDU_CODE_SUCCESS {
                let major = response.data()[0];
                let minor = response.data()[1];
                let patch = response.data()[2];
                Ok(Version {
                    major,
                    minor,
                    patch,
                })
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {:x}", retcode));
                Err(DiemLedgerError::UnexpectedError(
                    error_string,
                    Option::from(response.retcode()),
                ))
            }
        },
        Err(err) => Err(DiemLedgerError::from(err)),
    }
}

/// Returns the official app name register in Ledger
pub fn get_app_name() -> Result<String, DiemLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    match transport.exchange(&APDUCommand {
        cla: CLA_DIEM,
        ins: INS_GET_APP_NAME,
        p1: P1_NON_CONFIRM,
        p2: P2_LAST,
        data: vec![],
    }) {
        Ok(response) => {
            if response.retcode() == APDU_CODE_SUCCESS {
                let app_name = match str::from_utf8(response.data()) {
                    Ok(v) => v,
                    Err(e) => return Err(DiemLedgerError::UnexpectedError(e.to_string(), None)),
                };
                Ok(app_name.to_string())
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {:x}", retcode));
                Err(DiemLedgerError::UnexpectedError(
                    error_string,
                    Option::from(response.retcode()),
                ))
            }
        },
        Err(err) => Err(DiemLedgerError::from(err)),
    }
}

/// Returns the the batch/HashMap of the accounts for the account index in index_range
/// Note: We only allow a range of 10 for performance purpose
///
/// # Arguments
///
/// * `index_range` - start(inclusive) - end(exclusive) acounts, that you want to fetch, if None default to 0-10
pub fn fetch_batch_accounts(
    index_range: Option<Range<u32>>,
) -> Result<HashMap<String, AccountAddress>, DiemLedgerError> {
    let range = if let Some(range) = index_range {
        range
    } else {
        0..10
    };

    // Make sure the range is within 10 counts
    if range.end - range.start > 10 {
        return Err(DiemLedgerError::UnexpectedError(
            "Unexpected Error: Make sure the range is less than or equal to 10".to_string(),
            None,
        ));
    }

    // Open connection to ledger
    let transport = open_ledger_transport()?;

    let mut accounts = HashMap::new();
    for i in range {
        let path = DERIVATIVE_PATH.replace("{index}", &i.to_string());
        let cdata = serialize_bip32(&path);

        match transport.exchange(&APDUCommand {
            cla: CLA_DIEM,
            ins: INS_GET_PUB_KEY,
            p1: P1_NON_CONFIRM,
            p2: P2_LAST,
            data: cdata,
        }) {
            Ok(response) => {
                // Got the response from ledger after user has confirmed on the ledger wallet
                if response.retcode() == APDU_CODE_SUCCESS {
                    // Extract the Public key from the response data
                    let mut offset = 0;
                    let response_buffer = response.data();
                    let pub_key_len: usize = (response_buffer[offset] - 1).into();
                    offset += 1;

                    // Skipping weird 0x04 - because of how the Diem Ledger parse works when return pub key
                    offset += 1;

                    let pub_key_buffer = response_buffer[offset..offset + pub_key_len].to_vec();
                    let hex_string = encode(pub_key_buffer);
                    let public_key = match Ed25519PublicKey::from_encoded_string(&hex_string) {
                        Ok(pk) => Ok(pk),
                        Err(err) => Err(DiemLedgerError::UnexpectedError(
                            err.to_string(),
                            Some(response.retcode()),
                        )),
                    };
                    let account = account_address_from_public_key(&public_key?);
                    accounts.insert(path, account);
                } else {
                    let error_string = response
                        .error_code()
                        .map(|error_code| error_code.to_string())
                        .unwrap_or_else(|retcode| format!("Error with retcode: {:x}", retcode));
                    return Err(DiemLedgerError::UnexpectedError(
                        error_string,
                        Option::from(response.retcode()),
                    ));
                }
            },
            Err(err) => return Err(DiemLedgerError::from(err)),
        }
    }

    Ok(accounts)
}

/// Returns the public key of your Diem account in Ledger device at index 0
///
/// # Arguments
///
/// * `display` - If true, the public key will be displayed on the Ledger device, and confirmation is needed
pub fn get_public_key(path: &str, display: bool) -> Result<Ed25519PublicKey, DiemLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    // Serialize the derivative path
    let cdata = serialize_bip32(path);

    // APDU command's instruction parameter 1 or p1
    let p1: u8 = match display {
        true => P1_CONFIRM,
        false => P1_NON_CONFIRM,
    };

    match transport.exchange(&APDUCommand {
        cla: CLA_DIEM,
        ins: INS_GET_PUB_KEY,
        p1,
        p2: 0,
        data: cdata,
    }) {
        Ok(response) => {
            // Got the response from ledger after user has confirmed on the ledger wallet
            if response.retcode() == APDU_CODE_SUCCESS {
                // Extract the Public key from the response data
                let mut offset = 0;
                let response_buffer = response.data();
                let pub_key_len: usize = (response_buffer[offset] - 1).into();
                offset += 1;

                // Skipping weird 0x04 - because of how the Diem Ledger parse works when return pub key
                offset += 1;

                let pub_key_buffer = response_buffer[offset..offset + pub_key_len].to_vec();
                let hex_string = encode(pub_key_buffer);
                match Ed25519PublicKey::from_encoded_string(&hex_string) {
                    Ok(pk) => Ok(pk),
                    Err(err) => Err(DiemLedgerError::UnexpectedError(err.to_string(), None)),
                }
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {:x}", retcode));
                Err(DiemLedgerError::UnexpectedError(
                    error_string,
                    Option::from(response.retcode()),
                ))
            }
        },
        Err(err) => Err(DiemLedgerError::from(err)),
    }
}

/// Returns the signed signature of the raw transaction user provided
///
/// # Arguments
///
/// * `raw_txn` - the serialized raw transaction that need to be signed
pub fn sign_txn(path: &str, raw_txn: Vec<u8>) -> Result<Vec<u8>, DiemLedgerError> {
    // open connection to ledger
    let transport = open_ledger_transport()?;

    // Serialize the derivative path
    let derivative_path_bytes = serialize_bip32(path);

    // Send the derivative path over as first message
    let sign_start = transport.exchange(&APDUCommand {
        cla: CLA_DIEM,
        ins: INS_SIGN_TXN,
        p1: P1_START,
        p2: P2_MORE,
        data: derivative_path_bytes,
    });

    if let Err(err) = sign_start {
        return Err(DiemLedgerError::UnexpectedError(err.to_string(), None));
    }

    let chunks = raw_txn.chunks(MAX_APDU_LEN);
    let chunks_count = chunks.len();

    for (i, chunk) in chunks.enumerate() {
        let is_last_chunk = chunks_count == i + 1;
        match transport.exchange(&APDUCommand {
            cla: CLA_DIEM,
            ins: INS_SIGN_TXN,
            p1: (i + 1) as u8,
            p2: if is_last_chunk { P2_LAST } else { P2_MORE },
            data: chunk.to_vec(),
        }) {
            Ok(response) => {
                // success response
                if response.retcode() == APDU_CODE_SUCCESS {
                    if is_last_chunk {
                        let response_buffer = response.data();

                        let signature_len: usize = response_buffer[0] as usize;
                        let signature_buffer = &response_buffer[1..1 + signature_len];
                        return Ok(signature_buffer.to_vec());
                    }
                } else {
                    let error_string = response
                        .error_code()
                        .map(|error_code| error_code.to_string())
                        .unwrap_or_else(|retcode| {
                            format!("Unknown Ledger APDU retcode: {:x}", retcode)
                        });
                    return Err(DiemLedgerError::UnexpectedError(
                        error_string,
                        Option::from(response.retcode()),
                    ));
                }
            },
            Err(err) => return Err(DiemLedgerError::from(err)),
        };
    }
    Err(DiemLedgerError::UnexpectedError(
        "Unable to process request".to_string(),
        None,
    ))
}

/// This is the Rust version of the serialization of BIP32 from Petra Wallet
fn serialize_bip32(path: &str) -> Vec<u8> {
    let parts: Vec<u32> = path
        .split('/')
        .skip(1)
        .map(|part| {
            if let Some(part) = part.strip_suffix('\'') {
                part.parse::<u32>().unwrap() + 0x80000000
            } else {
                part.parse::<u32>().unwrap()
            }
        })
        .collect();

    let mut serialized = vec![0u8; 1 + parts.len() * 4];
    serialized[0] = parts.len() as u8;

    for (i, part) in parts.iter().enumerate() {
        serialized[(1 + i * 4)..(5 + i * 4)].copy_from_slice(&part.to_be_bytes());
    }

    serialized
}

fn open_ledger_transport() -> Result<TransportNativeHID, DiemLedgerError> {
    // open connection to ledger
    // NOTE: ledger has to be unlocked
    let hidapi = match HidApi::new() {
        Ok(hidapi) => hidapi,
        Err(_err) => return Err(DiemLedgerError::DeviceNotFound),
    };

    // Open transport to the first device
    let transport = match TransportNativeHID::new(&hidapi) {
        Ok(transport) => transport,
        Err(_err) => return Err(DiemLedgerError::DeviceNotFound),
    };

    Ok(transport)
}

fn account_address_from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    let auth_key = AuthenticationKey::ed25519(public_key);
    AccountAddress::new(*auth_key.derived_address())
}
