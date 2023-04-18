#!/usr/bin/env bash

# Installs/upgrades cargo-pgrx based on version specified in plrust/Cargo.toml
#
# Expects the following environment variables to already exist:
#  * None
#
# Expects the following parameters (in-order):
#  * None
#
# Example usage:
#  . /path/to/plrust/.github/scripts/install_cargo_pgrx.sh
#  install_cargo_pgrx

function install_cargo_pgrx() {
  set -o pipefail
  set -e

  if TARGET_VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[]|select(.name=="pgrx")|.version'); then
    echo "Installing/upgrading cargo-pgrx to version $TARGET_VERSION"
    cargo install cargo-pgrx --force --version "$TARGET_VERSION"
  else
    echo "Could not determine cargo-pgrx version to install."
    exit 1
  fi
}

