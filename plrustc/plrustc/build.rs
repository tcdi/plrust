fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rustc-env=PROFILE={}",
        std::env::var("PROFILE").unwrap_or_default(),
    );
    rustc_tools_util::setup_version_info!();
}
