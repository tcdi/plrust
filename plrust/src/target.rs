/*! PL/Rust adopts the tactic of always explicitly specifying which target to build for.
    This prevents using the "fallback" logic of Cargo leaving builds in an unlabeled directory.
    This is a precaution as PL/Rust is a cross-compiler.
    so a normal build-and-test cycle may create artifacts for multiple targets.
*/

use crate::gucs;
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
    const fn VENDOR() -> &'static str {
        if crate::TRUSTED {
            "postgres"
        } else {
            cfg_if::cfg_if! {
                if #[cfg(target_vendor = "apple")] {
                    "apple"
                } else if #[cfg(target_os = "windows")] {
                    "pc"
                } else {
                    "unknown"
                }
            }
        }
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

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub(crate) enum CrossCompilationTarget {
    X86_64 = 1,
    Aarch64 = 2,
}

impl Display for CrossCompilationTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CrossCompilationTarget::X86_64 => write!(f, "x86_64"),
            CrossCompilationTarget::Aarch64 => write!(f, "aarch64"),
        }
    }
}

impl CrossCompilationTarget {
    pub(crate) fn target(self) -> CompilationTarget {
        self.into()
    }

    pub(crate) fn linker_envar(&self) -> (String, String) {
        let key = format!(
            "CARGO_TARGET_{}_LINKER",
            self.target().as_str().to_uppercase().replace('-', "_")
        );

        let linker = gucs::get_linker_for_target(self).unwrap_or_else(|| match self {
            CrossCompilationTarget::X86_64 => "x86_64-linux-gnu-gcc".into(),
            CrossCompilationTarget::Aarch64 => "aarch64-linux-gnu-gcc".into(),
        });

        (key, linker)
    }
}

impl TryFrom<&str> for CrossCompilationTarget {
    type Error = TargetErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "x86_64" => Ok(CrossCompilationTarget::X86_64),
            "aarch64" => Ok(CrossCompilationTarget::Aarch64),
            _ => Err(TargetErr::Unsupported),
        }
    }
}

impl From<CrossCompilationTarget> for CompilationTarget {
    fn from(cct: CrossCompilationTarget) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature = "trusted")] {
                match cct {
                    CrossCompilationTarget::X86_64 => "x86_64-postgres-linux-gnu",
                    CrossCompilationTarget::Aarch64 => "aarch64-postgres-linux-gnu",
                }.into()
            } else {
                match cct {
                    CrossCompilationTarget::X86_64 => "x86_64-unknown-linux-gnu",
                    CrossCompilationTarget::Aarch64 => "aarch64-unknown-linux-gnu",
                }.into()
            }
        }
    }
}

pub(crate) fn tuple() -> Result<&'static CompilationTarget, &'static TargetErr> {
    static TARGET_TRIPLE: Lazy<Result<CompilationTarget, TargetErr>> =
        Lazy::new(|| Ok(host::target_tuple().into()));
    TARGET_TRIPLE.as_ref()
}
