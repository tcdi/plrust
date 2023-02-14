#!/usr/bin/env bash

# Installs/upgrades cargo-pgx based on version specified in plrust/Cargo.toml
#
# Expects the following environment variables to already exist:
#  * None
#
# Expects the following parameters (in-order):
#  * None
#
# Example usage:
#  . /path/to/plrust/.github/scripts/install_cargo_pgx.sh
#  install_cargo_pgx

function install_cargo_pgx() {
  set -o pipefail
  set -e

  if TARGET_VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[]|select(.name=="pgx")|.version'); then
    echo "Installing/upgrading cargo-pgx to version $TARGET_VERSION"
    cargo install cargo-pgx --force --version "$TARGET_VERSION"
  else
    echo "Could not determine cargo-pgx version to install."
    exit 1
  fi
}

