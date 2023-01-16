/*! PL/Rust adopts the tactic of always explicitly specifying which target to build for.
    This prevents using the "fallback" logic of Cargo leaving builds in an unlabeled directory.
    This is a precaution as PL/Rust is a cross-compiler.
    so a normal build-and-test cycle may create artifacts for multiple targets.
*/

use once_cell::sync::Lazy;
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::path::Path;

mod host {
    use std::env::consts::ARCH;
    cfg_if::cfg_if! { if #[cfg(target_env = "gnu")] {
        const ENV: &str = "gnu";
    } else if #[cfg(target_env = "musl")] {
        const ENV: &str = "musl";
    } else {
        const ENV: &str = "";
    }}

    #[allow(non_snake_case)]
    fn VENDOR() -> &'static str {
        cfg_if::cfg_if! {
        if #[cfg(all(
               target_os = "linux",
               any(target_arch = "x86_64", target_arch = "aarch64")
           ))]
        {
            if crate::gucs::PLRUST_USE_POSTGRESTD.get() {
                "postgres"
            } else {
                "unknown"
            }
        } else if #[cfg(target_vendor = "apple")] {
            "apple"
        } else if #[cfg(target_os = "windows")] {
            "pc"
        } else {
            "unknown"
        }}
    }

    cfg_if::cfg_if! { if #[cfg(target_os = "macos")] {
        const OS: &str = "darwin";
    } else {
        const OS: &str = std::env::consts::OS;
    }}

    pub(super) fn target_tuple() -> String {
        let tuple = [ARCH, VENDOR(), OS, ENV];
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

#[derive(Debug, Clone, PartialOrd, PartialEq, Hash, Ord, Eq)]
#[repr(transparent)]
pub(crate) struct CompilationTarget(String);
impl Deref for CompilationTarget {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<&str> for CompilationTarget {
    fn from(s: &str) -> Self {
        CompilationTarget(s.into())
    }
}
impl From<&String> for CompilationTarget {
    fn from(s: &String) -> Self {
        CompilationTarget(s.clone())
    }
}
impl From<String> for CompilationTarget {
    fn from(s: String) -> Self {
        CompilationTarget(s)
    }
}
impl Display for CompilationTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<Path> for CompilationTarget {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}
impl AsRef<OsStr> for CompilationTarget {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(&self.0)
    }
}
impl CompilationTarget {
    pub fn as_str(&self) -> &str {
        &self
    }
}

pub(crate) fn tuple() -> Result<&'static CompilationTarget, &'static TargetErr> {
    static TARGET_TRIPLE: Lazy<Result<CompilationTarget, TargetErr>> =
        Lazy::new(|| Ok(host::target_tuple().into()));
    TARGET_TRIPLE.as_ref()
}
