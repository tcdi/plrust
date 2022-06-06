#![cfg(all(target_os = "macos", target_arch = "x86_64"))]
/*!
    Darwin x86_64 is a peculiar platform for `dlclose`, this exists for a workaround to support
    `CREATE OR REPLACE FUNCTION`.

    If we unload something from `LOADED_SYMBOLS`, then load a recreated `so`, Darwin will have never
    properly unloaded it, and will load the old shared object (and the old symbol). This is surprising
    behavior to the user, and does not offer a good experience.

    Instead, we create a 'generation' for each build, and always load the largest numbered `so`. Since
    these `so`s are unique, Darwin loads the new one correctly. This technically 'leaks', but only
    because Darwin's `dlclose` 'leaks'.

    **This behavior is not required on other operating systems or architectures.**

    We expected this to also be required on Darwin aarch64, but testing on hardware has proven otherwise.

    See https://github.com/rust-lang/rust/issues/28794#issuecomment-368693049 which cites
    https://developer.apple.com/videos/play/wwdc2017/413/?time=1776.
!*/

use std::{fs, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("No generations found (Mac OS x86_64 specific)")]
    NoGenerations,
    #[error("std::io::Error: {0}")]
    StdIoError(#[from] std::io::Error),
}

/// Find existing generations of a given prefix.
#[tracing::instrument(level = "debug")]
pub(crate) fn all_generations(
    prefix: &str,
) -> eyre::Result<Box<dyn Iterator<Item = (usize, PathBuf)> + '_>> {
    let work_dir = crate::gucs::work_dir();
    let read_dir = fs::read_dir(work_dir).ok();
    match read_dir {
        Some(read_dir) => {
            let filtered = read_dir
                .flat_map(|entry| {
                    let entry = entry.ok()?;
                    if !entry.file_type().ok()?.is_file() {
                        return None;
                    }
                    let path = entry.path();
                    let stem = path.file_stem().and_then(|f| f.to_str())?.to_string();
                    Some((stem, path))
                })
                .filter(move |(stem, _path)| stem.starts_with(prefix))
                .flat_map(|(stem, path)| {
                    let generation = stem.split('_').last()?;
                    let generation = generation.parse::<usize>().ok()?;
                    tracing::trace!(%generation, path = %path.display(), "Got generation");
                    Some((generation, path))
                });

            Ok(Box::from(filtered))
        }
        None => Ok(Box::from(std::iter::empty())),
    }
}

/// Get the next generation number to be created.
///
/// If `vacuum` is set, this will pass the setting on to [`latest_generation`].
#[tracing::instrument(level = "debug")]
pub(crate) fn next_generation(prefix: &str, vacuum: bool) -> eyre::Result<usize> {
    let latest = latest_generation(prefix, vacuum);
    Ok(latest.map(|this| this.0 + 1).unwrap_or_default())
}

/// Get the latest created generation night.
///
/// If `vacuum` is set, this garbage collect old `so` files.
#[tracing::instrument(level = "debug")]
pub(crate) fn latest_generation(prefix: &str, vacuum: bool) -> eyre::Result<(usize, PathBuf)> {
    let mut generations = all_generations(prefix)?.collect::<Vec<_>>();
    // We could use max_by, but might need to vacuum.
    generations.sort_by_key(|(generation, _path)| *generation);
    let latest = generations.pop();

    if vacuum {
        for (_index, old_path) in generations {
            pgx::info!("Vacuuming {:?}", old_path);
            std::fs::remove_file(old_path)?;
        }
    }

    let latest = latest.ok_or(Error::NoGenerations)?;

    Ok(latest)
}
