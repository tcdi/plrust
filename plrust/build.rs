/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "cargo:rustc-env=PLRUST_TRUSTED_PGX_VERSION={}",
        find_trusted_pgx_current_version()?
    );
    Ok(())
}

fn find_trusted_pgx_current_version() -> Result<String, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("tree")
        .arg("-p")
        .arg("plrust-trusted-pgx")
        .arg("--depth")
        .arg("0")
        .output()?;

    // looking for some output similar to:
    //
    //      plrust-trusted-pgx v1.0.0 (/home/zombodb/_work/plrust/plrust-trusted-pgx)
    //
    // and we want the "v1.0.0" part
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts = stdout.split_whitespace().collect::<Vec<_>>();
    let version = parts
        .get(1)
        .ok_or_else(|| "unexpected `cargo tree` output")?;
    let version = &version[1..]; // strip off the leading 'v'
    Ok(version.to_string())
}
