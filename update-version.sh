#!/usr/bin/env bash
# Portions Copyright 2019-2021 ZomboDB, LLC.
# Portions Copyright 2021-2025 Technology Concepts & Design, Inc.
#
# All rights reserved.
#
# Use of this source code is governed by the PostgreSQL license that can be
# found in the LICENSE.md file.

##
## This script requires `cargo install cargo-workspace-version` from https://crates.io/crates/cargo-workspace-version
##

NEW_VERSION=$1

if [ -z "${NEW_VERSION}" ]; then
  echo usage:  ./update-version.sh new.version.number
  exit 1
fi

## update versions
cargo workspace-version update v"${NEW_VERSION}"
cargo generate-lockfile

cd plrustc
cargo workspace-version update v"${NEW_VERSION}"
cargo generate-lockfile
