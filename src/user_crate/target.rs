/*! PL/Rust adopts the tactic of always explicitly specifying which target to build for.
    This prevents using the "fallback" logic of Cargo leaving builds in an unlabeled directory.
    This is a precaution as PL/Rust is a cross-compiler.
    so a normal build-and-test cycle may create artifacts for multiple targets.
!*/

use std::env;
use std::ffi::OsString;

pub(crate) mod host {
    use std::env::consts::*;
    cfg_if::cfg_if! { if #[cfg(target_env = "gnu")] {
        pub(crate) const ENV: &str = "gnu";
    } else if #[cfg(target_env = "musl")] {
        pub(crate) const ENV: &str = "musl";
    } else {
        pub(crate) const ENV: &str = "";
    }}
    cfg_if::cfg_if! { if #[cfg(target_vendor = "apple")] {
        pub(crate) const VENDOR: &str = "apple";
    } else if #[cfg(target_os = "windows")] {
        pub(crate) const VENDOR: &str = "pc";
    } else {
        pub(crate) const VENDOR: &str = "unknown";
    }}

    pub(crate) fn target_tuple() -> String {
        let os = match OS {
            "macos" => "darwin",
            os => os
        };
        super::stringify_tuple([ARCH, VENDOR, os, ENV])
    }
}

// Assemble a String from the components of a build tuple.
fn stringify_tuple(tuple: [&str; 4]) -> String {
    let mut s = String::from(tuple[0]);
    for t in &tuple[1..] {
        if t != &"" {
            s.push('-');
            s.push_str(t);
        }
    }
    s
}

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // Such is the life of cfg code
pub(crate) enum TargetErr {
    #[error("unsupported target tuple")]
    Unsupported,
    #[error("non-UTF-8 target tuple specifiers are invalid: {}", .0.to_string_lossy())]
    InvalidSpec(OsString),
}

pub(crate) fn tuple() -> Result<String, TargetErr> {
    match env::var("PLRUST_TARGET") {
        Ok(v) => Ok(v),
        Err(env::VarError::NotPresent) => {
            cfg_if::cfg_if! {
                if #[cfg(all(feature = "target_postgrestd", target_arch = "x86_64", target_os = "linux"))] {
                    Ok("x86_64-postgres-linux-gnu".to_string())
                } else if #[cfg(feature = "target_postgrestd")] {
                    Err(TargetErr::Unsupported)
                } else {
                    Ok(host::target_tuple())
                }
            }
        }
        Err(env::VarError::NotUnicode(s)) => Err(TargetErr::InvalidSpec(s)),
    }
}
