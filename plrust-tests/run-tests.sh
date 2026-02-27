#! /bin/bash
# Portions Copyright 2021-2025 Technology Concepts & Design, Inc.
#
# All rights reserved.
#
# Use of this source code is governed by the PostgreSQL license that can be
# found in the LICENSE.md file.

VERSION=$1

if [ -z ${VERSION} ]; then
	echo "usage:  ./run-tests.sh pgXX [test-name]"
	exit 1
fi

TEST_DIR=`pwd`

set -e

# install the plrust extension into the pgrx-managed postgres
echo "============================"
echo " installing plrust"
echo
cd ../plrust
echo "\q" | cargo pgrx run ${VERSION}

# run the test suite from this crate
cd ${TEST_DIR}

echo
echo "============================"
echo " running plrust-tests suite"
echo

cargo pgrx test ${VERSION} $2

