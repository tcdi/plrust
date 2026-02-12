#!/usr/bin/env bash
# Portions Copyright 2019-2021 ZomboDB, LLC.
# Portions Copyright 2021-2025 Technology Concepts & Design, Inc.
#
# All rights reserved.
#
# Use of this source code is governed by the PostgreSQL license that can be
# found in the LICENSE.md file.

set -x
cd plrust-trusted-pgrx && cargo publish --no-verify
