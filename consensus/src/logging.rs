// Copyright © Diem Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use diem_consensus_types::common::Author;
use diem_logger::Schema;
use diem_types::block_info::Round;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    remote_peer: Option<Author>,
    epoch: Option<u64>,
    round: Option<Round>,
}

#[derive(Serialize)]
pub enum LogEvent {
    CommitViaBlock,
    CommitViaSync,
    NewEpoch,
    NewRound,
    Propose,
    ReceiveBatchRetrieval,
    ReceiveBlockRetrieval,
    ReceiveEpochChangeProof,
    ReceiveEpochRetrieval,
    ReceiveMessageFromDifferentEpoch,
    ReceiveNewCertificate,
    ReceiveProposal,
    ReceiveSyncInfo,
    ReceiveVote,
    RetrieveBlock,
    StateSync,
    Timeout,
    Vote,
    VoteNIL,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            remote_peer: None,
            epoch: None,
            round: None,
        }
    }
}
