/*! PL/Rust adopts the tactic of always explicitly specifying which target to build for.
    This prevents using the "fallback" logic of Cargo leaving builds in an unlabeled directory.
    This is a precaution as PL/Rust is a cross-compiler.
    so a normal build-and-test cycle may create artifacts for multiple targets.
*/

use std::env;
use std::ffi::OsString;

mod host {
    use std::env::consts::ARCH;
    cfg_if::cfg_if! { if #[cfg(target_env = "gnu")] {
        const ENV: &str = "gnu";
    } else if #[cfg(target_env = "musl")] {
        const ENV: &str = "musl";
    } else {
        const ENV: &str = "";
    }}
    cfg_if::cfg_if! { if #[cfg(feature = "target_postgrestd")] {
        const VENDOR: &str = "postgres";
    } else if #[cfg(target_vendor = "apple")] {
        const VENDOR: &str = "apple";
    } else if #[cfg(target_os = "windows")] {
        const VENDOR: &str = "pc";
    } else {
        const VENDOR: &str = "unknown";
    }}

    cfg_if::cfg_if! { if #[cfg(target_os = "macos")] {
        const OS: &str = "darwin";
    } else {
        const OS: &str = std::env::consts::OS;
    }}

    pub(super) fn target_tuple() -> String {
        let tuple = [ARCH, VENDOR, OS, ENV];
        let mut s = String::from(tuple[0]);
        for t in &tuple[1..] {
            if t != &"" {
                s.push('-');
                s.push_str(t);
            }
        }
        s
    }
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
                if #[cfg(all(feature = "target_postgrestd",
                    any(target_arch = "x86_64", target_arch = "aarch64"),
                    target_os = "linux",
                    target_env = "gnu"))]
                {
                    Ok(host::target_tuple())
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
