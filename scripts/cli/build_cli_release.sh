#!/bin/bash
# Copyright © Diem Foundation
# SPDX-License-Identifier: Apache-2.0

###########################################
# Build and package a release for the CLI #
###########################################

# Note: This must be run from the root of the diem-core repository

set -e

NAME='diem-cli'
CRATE_NAME='diem'
CARGO_PATH="crates/$CRATE_NAME/Cargo.toml"
PLATFORM_NAME="$1"

# Grab system information
ARCH=`uname -m`
OS=`uname -s`
VERSION=`cat "$CARGO_PATH" | grep "^\w*version =" | sed 's/^.*=[ ]*"//g' | sed 's/".*$//g'`

echo "Building release $VERSION of $NAME for $OS-$PLATFORM_NAME"
cargo build -p $CRATE_NAME --profile cli

cd target/cli/

# Compress the CLI
ZIP_NAME="$NAME-$VERSION-$PLATFORM_NAME-$ARCH.zip"

echo "Zipping release: $ZIP_NAME"
zip $ZIP_NAME $CRATE_NAME
mv $ZIP_NAME ../..
