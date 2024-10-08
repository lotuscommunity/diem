// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use diem_config::network_id::PeerNetworkId;
use diem_logger::Schema;
use diem_peer_monitoring_service_types::request::PeerMonitoringServiceRequest;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    #[schema(debug)]
    error: Option<&'a Error>,
    event: Option<LogEvent>,
    message: Option<&'a str>,
    #[schema(display)]
    peer: Option<&'a PeerNetworkId>,
    #[schema(debug)]
    request: Option<&'a PeerMonitoringServiceRequest>,
    request_id: Option<u64>,
    request_type: Option<&'a str>,
}

impl<'a> LogSchema<'a> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            event: None,
            message: None,
            peer: None,
            request: None,
            request_id: None,
            request_type: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    LatencyPing,
    MetadataUpdateLoop,
    NetworkInfoRequest,
    NodeInfoRequest,
    PeerMonitorLoop,
    SendRequest,

    #[cfg(feature = "network-perf-test")] // Disabled by default
    PerformanceMonitoringRequest,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    InvalidResponse,
    LogAllPeerStates,
    PeerPingError,
    ResponseError,
    ResponseSuccess,
    SendRequest,
    StartedMetadataUpdaterLoop,
    StartedPeerMonitorLoop,
    TooManyPingFailures,
    UnexpectedErrorEncountered,
}
