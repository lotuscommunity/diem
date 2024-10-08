#!/bin/bash

# Copyright © Diem Foundation
# SPDX-License-Identifier: Apache-2.0

# This builds the move docs for the Diem-framework
# Removes the awkward links so that it can be reasonably well self-hosted
# And moves them to a local folder

current_path=$PWD

root_path="$(dirname $0)/.."
move_path="$root_path/diem-move/framework"

cd $move_path
cargo run

if [[ "$1" ]]; then
  outpath="$1"
else
  outpath="output"
fi

cd $current_path
rm -rf $outpath
mkdir -p $outpath

RELEASE_PATH="$move_path/diem-framework/releases/artifacts/current/build"

for folder in $(ls $RELEASE_PATH); do
  mkdir -p $outpath/$folder
  for file in $(ls $RELEASE_PATH/$folder/docs); do
    outfile=$outpath/$folder/$file
    cp $RELEASE_PATH/$folder/docs/$file $outfile
    sed -i \
      's#../../../../../../../diem-framework/releases/artifacts/current/build/\(.*\)/docs/\(.*.md\)#../\1/\2#g' \
      $outfile
  done
done
