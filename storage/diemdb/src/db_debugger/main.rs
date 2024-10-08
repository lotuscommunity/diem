// Copyright © Diem Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_db::db_debugger::Cmd;
use clap::Parser;

fn main() -> Result<()> {
    Cmd::parse().run()
}
