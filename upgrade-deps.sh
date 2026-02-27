#! /bin/bash
# Portions Copyright 2021-2025 Technology Concepts & Design, Inc.
#
# All rights reserved.
#
# Use of this source code is governed by the PostgreSQL license that can be
# found in the LICENSE.md file.

# requires:  "cargo install cargo-edit" from https://github.com/killercup/cargo-edit

cargo update

# generally speaking, the only pinned dependency we use is pgrx, and generally speaking the only time we run this script
# is when we want to upgrade to a newer pgrx.  `--pinned` has entered the chat
cargo upgrade --pinned --incompatible
cargo generate-lockfile

cd plrustc
cargo upgrade --incompatible
cargo generate-lockfile
