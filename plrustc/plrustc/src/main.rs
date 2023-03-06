#![feature(rustc_private)]
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;

extern crate rustc_lint;
extern crate rustc_lint_defs;
extern crate rustc_session;
extern crate rustc_span;

use rustc_driver::Callbacks;
use rustc_interface::interface;
use rustc_session::parse::ParseSess;
use rustc_span::source_map::FileLoader;
use rustc_span::Symbol;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const PLRUSTC_USER_CRATE_NAME: &str = "PLRUSTC_USER_CRATE_NAME";
const PLRUSTC_USER_CRATE_MAY_ACCESS: &str = "PLRUSTC_USER_CRATE_MAY_ACCESS";

mod lints;

struct PlrustcCallbacks {
    lints_enabled: bool,
    config: PlrustcConfig,
}

impl Callbacks for PlrustcCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        let cfg = self.config.clone();
        config.parse_sess_created = Some(Box::new(move |parse_sess| {
            cfg.track(parse_sess);
        }));
        if self.lints_enabled {
            let previous = config.register_lints.take();
            config.register_lints = Some(Box::new(move |sess, lint_store| {
                if let Some(previous) = &previous {
                    (previous)(sess, lint_store);
                }
                lints::register(lint_store, sess);
            }));
        }
    }
}

fn main() {
    rustc_driver::install_ice_hook();
    rustc_driver::init_rustc_env_logger();
    std::process::exit(rustc_driver::catch_with_exit_code(move || {
        let orig_args: Vec<String> = std::env::args().collect();

        let sysroot_arg = arg_value(&orig_args, "--sysroot");
        let have_sysroot_arg = sysroot_arg.is_some();
        let sysroot = sysroot_arg
            .map(ToString::to_string)
            .or_else(|| sysroot().map(|p| p.display().to_string()))
            .expect("Failed to find sysroot");

        let mut args: Vec<String> = orig_args.clone();

        if !have_sysroot_arg {
            args.extend(["--sysroot".to_string(), sysroot.to_string()]);
        }

        let our_exe_filename = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(ToOwned::to_owned))
            .unwrap_or_else(|| "plrustc".into());

        let wrapper_mode = orig_args
            .get(1)
            .map(std::path::Path::new)
            .and_then(std::path::Path::file_stem)
            .map_or(false, |name| {
                name == our_exe_filename || name == "plrustc" || name == "rustc"
            });

        if wrapper_mode {
            args.remove(1);
        }

        run_compiler(
            args,
            &mut PlrustcCallbacks {
                // FIXME SOMEDAY: check caplints?
                lints_enabled: true,
                config: PlrustcConfig::from_env_and_args(&orig_args),
            },
        );
    }))
}

#[derive(Debug, Clone)]
struct PlrustcConfig {
    // If `--crate-name` was provided, that.
    crate_name_arg: Option<String>,
    // PLRUSTC_USER_CRATE_NAME
    plrust_user_crate_name: Option<String>,
    // PLRUSTC_USER_CRATE_MAY_ACCESS
    plrust_user_crate_may_access: Option<String>,
    // PLRUSTC_TARGET_DIR
    // plrust_target_dir: Option<String>,
}

impl PlrustcConfig {
    fn from_env_and_args(args: &[String]) -> Self {
        PlrustcConfig {
            crate_name_arg: arg_value(args, "--crate-name").map(|s| s.to_string()),
            plrust_user_crate_name: std::env::var(PLRUSTC_USER_CRATE_NAME).ok(),
            plrust_user_crate_may_access: std::env::var(PLRUSTC_USER_CRATE_MAY_ACCESS).ok(),
        }
    }

    fn compiling_user_crate(&self) -> bool {
        if let (Some(current), Some(user)) = (
            self.crate_name_arg.as_deref(),
            self.plrust_user_crate_name.as_deref(),
        ) {
            current == user
        } else {
            false
        }
    }

    fn track(&self, parse_sess: &mut ParseSess) {
        if self.compiling_user_crate() {
            parse_sess.env_depinfo.lock().insert((
                Symbol::intern(PLRUSTC_USER_CRATE_NAME),
                self.plrust_user_crate_name.as_deref().map(Symbol::intern),
            ));
            parse_sess.env_depinfo.lock().insert((
                Symbol::intern(PLRUSTC_USER_CRATE_MAY_ACCESS),
                self.plrust_user_crate_may_access
                    .as_deref()
                    .map(Symbol::intern),
            ));
        }
    }

    fn make_file_loader(&self) -> Box<dyn FileLoader + Send + Sync> {
        if !self.compiling_user_crate() {
            Box::new(ErrorHidingFileLoader)
        } else {
            let Some(allowed) = self.plrust_user_crate_may_access.as_deref() else {
                eprintln!("fatal error: if `{PLRUSTC_USER_CRATE_NAME}` is provided, then `{PLRUSTC_USER_CRATE_MAY_ACCESS}` should also be provided");
                std::process::exit(1);
            };
            // Used by cargo also
            const ASCII_UNIT_SEP: char = '\u{1f}';
            let allowed_source_dirs = allowed.split(ASCII_UNIT_SEP).filter(|s| !s.is_empty()).map(|s| {
                let p = Path::new(s);
                let path = p.canonicalize().unwrap_or_else(|_| p.to_owned());
                if !path.is_absolute() {
                    eprintln!("fatal error: `{PLRUSTC_USER_CRATE_MAY_ACCESS}` contains relative path: {path:?}");
                    std::process::exit(1);
                }
                let Some(pathstr) = path.to_str() else {
                    eprintln!("fatal error: `{PLRUSTC_USER_CRATE_MAY_ACCESS}` contains non-UTF-8 path: {path:?}");
                    std::process::exit(1);
                };
                pathstr.strip_suffix('/').unwrap_or(pathstr).to_string()
            }).collect::<Vec<String>>();

            Box::new(PlrustcFileLoader {
                allowed_source_dirs,
            })
        }
    }
}

fn arg_value<'a, T: AsRef<str>>(args: &'a [T], find_arg: &str) -> Option<&'a str> {
    let mut args = args.iter().map(|s| s.as_ref());
    while let Some(arg) = args.next() {
        let mut arg = arg.splitn(2, '=');
        if arg.next() != Some(find_arg) {
            continue;
        }

        if let Some(a) = arg.next().or_else(|| args.next()) {
            return Some(a);
        }
    }
    None
}

struct ErrorHidingFileLoader;

fn replacement_error() -> std::io::Error {
    // Unix-ism, but non-unix could use `io::Error::from(ErrorKind::NotFound)`
    // or something like that.
    std::io::Error::from_raw_os_error(libc::ENOENT)
}

impl FileLoader for ErrorHidingFileLoader {
    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn read_file(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path).map_err(|_| {
            // TODO: Should there be a way to preserve errors for debugging?
            replacement_error()
        })
    }
}

struct PlrustcFileLoader {
    allowed_source_dirs: Vec<String>,
}

impl PlrustcFileLoader {
    fn absify_path(&self, p: &Path) -> std::io::Result<PathBuf> {
        p.canonicalize().or_else(|_| {
            use omnipath::posix::PosixPathExt;
            p.posix_absolute()
        })
    }

    fn is_allowed(&self, p: &Path) -> bool {
        let Ok(abs) = self.absify_path(p) else {
            return false;
        };
        let Some(s) = abs.to_str() else {
            return false;
        };
        self.allowed_source_dirs.iter().any(|d| s.starts_with(d))
    }
}

impl FileLoader for PlrustcFileLoader {
    fn file_exists(&self, path: &Path) -> bool {
        self.is_allowed(path) && path.exists()
    }

    fn read_file(&self, path: &Path) -> std::io::Result<String> {
        if self.is_allowed(path) {
            ErrorHidingFileLoader.read_file(path)
        } else {
            Err(replacement_error())
        }
    }
}

/// Get the sysroot, looking from most specific to this invocation to the
/// least.
///
/// - command line `--sysroot` arg (happens in caller)
///
/// - runtime environment
///    - PLRUSTC_SYSROOT
///    - SYSROOT
///    - RUSTUP_HOME, RUSTUP_TOOLCHAIN
///
/// - sysroot from rustc in the path
///
/// - compile-time environment
///    - PLRUSTC_SYSROOT
///    - SYSROOT
///    - RUSTUP_HOME, RUSTUP_TOOLCHAIN
fn sysroot() -> Option<PathBuf> {
    fn rustup_sysroot<H: ?Sized + AsRef<OsStr>, T: ?Sized + AsRef<Path>>(
        home: &H,
        toolchain: &T,
    ) -> PathBuf {
        let mut path = PathBuf::from(home);
        path.push("toolchains");
        path.push(toolchain);
        path
    }
    fn runtime_rustup_sysroot() -> Option<PathBuf> {
        let home = std::env::var_os("RUSTUP_HOME")?;
        let toolchain = std::env::var_os("RUSTUP_TOOLCHAIN")?;
        Some(rustup_sysroot(&home, &toolchain))
    }
    fn compiletime_rustup_sysroot() -> Option<PathBuf> {
        let home: &str = option_env!("RUSTUP_HOME")?;
        let toolchain: &str = option_env!("RUSTUP_TOOLCHAIN")?;
        Some(rustup_sysroot(&home, &toolchain))
    }
    fn rustc_on_path_sysroot() -> Option<PathBuf> {
        std::process::Command::new("rustc")
            .arg("--print=sysroot")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| PathBuf::from(s.trim()))
    }
    fn runtime_explicit_env() -> Option<PathBuf> {
        let sysroot =
            std::env::var_os("PLRUSTC_SYSROOT").or_else(|| std::env::var_os("SYSROOT"))?;
        Some(PathBuf::from(sysroot))
    }
    fn compiletime_explicit_env() -> Option<PathBuf> {
        let plrustc_sysroot: Option<&str> = option_env!("PLRUSTC_SYSROOT");
        if let Some(plrustc_sysroot) = plrustc_sysroot {
            return Some(plrustc_sysroot.into());
        }
        let sysroot: Option<&str> = option_env!("SYSROOT");
        if let Some(sysroot) = sysroot {
            return Some(sysroot.into());
        }
        None
    }
    runtime_explicit_env()
        .or_else(runtime_rustup_sysroot)
        .or_else(rustc_on_path_sysroot)
        .or_else(compiletime_explicit_env)
        .or_else(compiletime_rustup_sysroot)
}

fn run_compiler(mut args: Vec<String>, callbacks: &mut PlrustcCallbacks) -> ! {
    args.splice(1..1, std::iter::once("--cfg=plrustc".to_string()));

    std::process::exit(rustc_driver::catch_with_exit_code(move || {
        let file_loader = callbacks.config.make_file_loader();
        let mut driver = rustc_driver::RunCompiler::new(&args, callbacks);
        driver.set_file_loader(Some(file_loader));
        driver.run()
    }));
}
