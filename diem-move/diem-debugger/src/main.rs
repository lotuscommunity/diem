// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_debugger::DiemDebugger;
use diem_rest_client::Client;
use diem_vm::DiemVM;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use url::Url;

#[derive(Subcommand)]
pub enum Target {
    /// Use full node's rest api as query endpoint.
    Rest { endpoint: String },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf },
}
#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    target: Target,

    #[clap(long)]
    begin_version: u64,

    #[clap(long)]
    limit: u64,

    #[clap(long, default_value_t = 1)]
    concurrency_level: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    diem_logger::Logger::new().init();
    let args = Argument::parse();
    DiemVM::set_concurrency_level_once(args.concurrency_level);

    let debugger = match args.target {
        Target::Rest { endpoint } => {
            DiemDebugger::rest_client(Client::new(Url::parse(&endpoint)?))?
        },
        Target::DB { path } => DiemDebugger::db(path)?,
    };

    println!(
        "{:#?}",
        debugger
            .execute_past_transactions(args.begin_version, args.limit)
            .await?
    );

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
