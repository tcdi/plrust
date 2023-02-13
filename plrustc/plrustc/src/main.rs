#![feature(rustc_private)]
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;

extern crate rustc_lint;
extern crate rustc_lint_defs;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_tools_util;

use rustc_interface::interface;
// use rustc_session::parse::ParseSess;
use rustc_span::symbol::Symbol;
use std::env;
use std::path::PathBuf;
use std::process::exit;

mod lints;

struct PlRustcPassthroughCallbacks;

impl rustc_driver::Callbacks for PlRustcPassthroughCallbacks {}

struct PlrustcCallbacks {
    lints_enabled: bool,
    plrustc_flags: Option<String>,
}

impl rustc_driver::Callbacks for PlrustcCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        let plrustc_flags = self.plrustc_flags.take();
        config.parse_sess_created = Some(Box::new(move |parse_sess| {
            parse_sess.env_depinfo.get_mut().insert((
                Symbol::intern("PLRUSTC_FLAGS_TRACKED"),
                plrustc_flags.as_deref().map(Symbol::intern),
            ));
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
    // Snapshot the environment before we call functions which may mess with it.
    // let env_snapshot = env::vars_os().collect::<Vec<_>>();
    rustc_driver::install_ice_hook();
    rustc_driver::init_rustc_env_logger();
    exit(rustc_driver::catch_with_exit_code(move || {
        let orig_args: Vec<String> = env::args().collect();
        let target_arg = arg_value(&orig_args, "--target", |_| true);
        let have_target_arg = target_arg.is_some();

        let sysroot_arg = arg_value(&orig_args, "--sysroot", |_| true);
        let have_sysroot_arg = sysroot_arg.is_some();
        let sysroot = sysroot_arg
            .map(ToString::to_string)
            .or_else(|| guess_sysroot());

        let mut args: Vec<String> = orig_args.clone();
        // TODO: does this actually make sense under `PLRUST_PASSTHROUGH`?
        if !have_sysroot_arg {
            if let Some(sysroot) = &sysroot {
                args.extend(["--sysroot".into(), sysroot.clone()]);
            }
        }

        if let Some(crate_kind) = env::var_os("PLRUSTC_PASSTHROUGH") {
            rustc_driver::init_rustc_env_logger();
            let target_crate = if crate_kind == "target" {
                true
            } else if crate_kind == "host" {
                false
            } else {
                // Hm....
                have_target_arg
            };

            run_compiler(
                env::args().collect(),
                target_crate,
                &mut PlRustcPassthroughCallbacks,
            );
        }
        // Intentionally avoid complaining about the sysroot while in
        // passthrough mode.
        sysroot.expect(
            "\
            Failed to locate sysroot. Ensure it's provided via one of: \
            - arguments (`--sysroot`) \
            - environment (`PLRUST_SYSROOT`, `SYSROOT`) \
            - rustup installation",
        );

        // if orig_args.iter().any(|a| a == "--version" || a == "-V") {
        //     let version_info = get_version_info!();
        //     println!("{}", version_info);
        //     exit(0);
        // }

        // Setting RUSTC_WRAPPER causes Cargo to pass 'rustc' as the first
        // argument. We're invoking the compiler programmatically, so we ignore
        // this, and ensure we can still invoke it normally.
        let wrapper_mode = orig_args
            .get(1)
            .map(std::path::Path::new)
            .and_then(std::path::Path::file_stem)
            == Some("rustc".as_ref());

        if wrapper_mode {
            args.remove(1);
        }

        // if !wrapper_mode
        //     && (orig_args.iter().any(|a| a == "--help" || a == "-h") || orig_args.len() == 1)
        // {
        //     display_help();
        //     exit(0);
        // }

        // let cap_lints_allow = arg_value(&orig_args, "--cap-lints", |val| val == "allow").is_some()
        //     && arg_value(&orig_args, "--force-warn", |val| val.contains("plrustc_lints")).is_none();
        // let in_primary_package = env::var("CARGO_PRIMARY_PACKAGE").is_ok();

        let lints_enabled = true; // !cap_lints_allow && in_primary_package;

        run_compiler(
            args,
            target_arg.is_some(),
            &mut PlrustcCallbacks {
                plrustc_flags: std::env::var("PLRUSTC_FLAGS_TRACKED").ok(),
                lints_enabled,
            },
        );
    }))
}

fn arg_value<'a, T: AsRef<str>>(
    args: &'a [T],
    find_arg: &str,
    pred: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    let mut args = args.iter().map(|s| s.as_ref());
    while let Some(arg) = args.next() {
        let mut arg = arg.splitn(2, '=');
        if arg.next() != Some(find_arg) {
            continue;
        }

        match arg.next().or_else(|| args.next()) {
            Some(v) if pred(v) => return Some(v),
            _ => {}
        }
    }
    None
}
// Get the sysroot, looking from most specific to this invocation to the
// least.
//
// - command line `--sysroot` arg (happens in caller)
//
// - runtime environment
//    - PLRUSTC_SYSROOT
//    - SYSROOT
//    - RUSTUP_HOME, RUSTUP_TOOLCHAIN
//
// - sysroot from rustc in the path
//
// - compile-time environment
//    - PLRUSTC_SYSROOT
//    - SYSROOT
//    - RUSTUP_HOME, RUSTUP_TOOLCHAIN
fn guess_sysroot() -> Option<String> {
    std::env::var("PLRUSTC_SYSROOT")
        .ok()
        .map(PathBuf::from)
        // .or_else(|| std::env::var("PLRUSTC_SYSROOT_TARGETONLY"))
        .or_else(|| std::env::var("SYSROOT").ok().map(PathBuf::from))
        .or_else(|| {
            sysroot_from_rustup(
                std::env::var("RUSTUP_HOME").ok().as_deref(),
                std::env::var("RUSTUP_TOOLCHAIN").ok().as_deref(),
            )
        })
        .or_else(|| {
            std::process::Command::new("rustc")
                .arg("--print=sysroot")
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
        })
        .or_else(|| {
            let plrust_sysroot: Option<_> = option_env!("PLRUSTC_SYSROOT");
            plrust_sysroot.or(option_env!("SYSROOT")).map(PathBuf::from)
        })
        .or_else(|| {
            sysroot_from_rustup(option_env!("RUSTUP_HOME"), option_env!("RUSTUP_TOOLCHAIN"))
        })
        .map(|pb| pb.to_string_lossy().to_string())
}

fn sysroot_from_rustup(home: Option<&str>, toolchain: Option<&str>) -> Option<PathBuf> {
    home.and_then(|home| {
        toolchain.map(|toolchain| {
            let mut path = PathBuf::from(home);
            path.push("toolchains");
            path.push(toolchain);
            path
        })
    })
}
fn run_compiler(
    mut args: Vec<String>,
    target_crate: bool,
    callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> ! {
    let mut extra_args = vec![];
    if target_crate {
        extra_args.push("--cfg=plrust");
    }
    args.splice(1..1, extra_args.iter().map(ToString::to_string));

    // Invoke compiler, and handle return code.
    let exit_code = rustc_driver::catch_with_exit_code(move || {
        rustc_driver::RunCompiler::new(&args, callbacks).run()
    });
    std::process::exit(exit_code)
}
