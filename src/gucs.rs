use pgx::*;
use std::path::PathBuf;
use std::str::FromStr;

static PLRUST_WORK_DIR: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_PG_CONFIG: GucSetting<Option<&'static str>> = GucSetting::new(None);

pub(crate) fn init() {
    GucRegistry::define_string_guc(
        "plrust.work_dir",
        "The directory where pl/rust will build functions with cargo",
        "The directory where pl/rust will build functions with cargo",
        &PLRUST_WORK_DIR,
        GucContext::Sighup,
    );

    GucRegistry::define_string_guc(
        "plrust.pg_config",
        "What is the full path to the `pg_config` tool for this Postgres installation?",
        "What is the full path to the `pg_config` tool for this Postgres installation?",
        &PLRUST_PG_CONFIG,
        GucContext::Sighup,
    );

    // create our work directory if it doesn't exist
    let work_dir = work_dir();
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir)
            .expect("failed to create directory specified by plrust.work_dir");
    }

    // bootstrap a configuration for pgx
    let mut pgx_dir = work_dir;
    pgx_dir.push(".pgx");
    if !pgx_dir.exists() {
        std::fs::create_dir_all(&pgx_dir)
            .expect("failed to create pgx directory in plrust.work_dir");
    }

    let mut config_toml = pgx_dir;
    config_toml.push("config.toml");
    std::fs::write(
        config_toml,
        &format!(
            r#"[configs]
pg{}="{}"
"#,
            pg_sys::get_pg_major_version_string(),
            pg_config()
        ),
    )
    .expect("failed to write config.toml file");
}

pub(crate) fn work_dir() -> PathBuf {
    PathBuf::from_str(
        &PLRUST_WORK_DIR
            .get()
            .expect("plrust.work_dir is not set in postgresql.conf"),
    )
    .expect("plrust.work_dir is not a valid path")
}

pub(crate) fn pgx_dir() -> PathBuf {
    let mut path = work_dir();
    path.push(".pgx");
    path
}

fn pg_config() -> String {
    PLRUST_PG_CONFIG
        .get()
        .expect("plrust.pg_config is not set in postgresql.conf")
}
