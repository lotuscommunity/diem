// Copyright © Diem Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use diem_fuzzer::FuzzTarget;
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(
    name = "Diem-Fuzzer Investigator",
    author = "Diem",
    about = "Utility tool to investigate fuzzing artifacts"
)]
struct Args {
    /// Admission Control port to connect to.
    #[clap(short = 'i', long)]
    pub input_file: Option<String>,
}

fn main() {
    // args
    let args = Args::parse();

    // check if it exists
    let input_file = PathBuf::from(args.input_file.expect("input file must be set via -i"));
    if !input_file.exists() {
        println!("usage: cargo run investigate -i <artifacts/.../corpus_input>");
        return;
    }

    // get target from path (input_file = .../<target>/<corpus_input>)
    let mut ancestors = input_file.ancestors();
    ancestors.next(); // skip full path
    let target_name = ancestors
        .next()
        .expect("input file should be inside target directory")
        .iter()
        .last()
        .unwrap()
        .to_str()
        .unwrap();
    let target = FuzzTarget::by_name(target_name)
        .unwrap_or_else(|| panic!("unknown fuzz target: {}", target_name));

    // run the target fuzzer on the file
    let data = fs::read(input_file).expect("failed to read artifact");
    target.fuzz(&data);
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
