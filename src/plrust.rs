/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::{gucs, user_crate::{UserCrate, StateLoaded}};
use once_cell::unsync::Lazy;
use pgx::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    env::consts::DLL_SUFFIX,
    path::PathBuf,
    process::Output,
};

static mut LOADED_SYMBOLS: Lazy<
    HashMap<
        pg_sys::Oid,
        UserCrate<StateLoaded>,
    >,
> = Lazy::new(|| Default::default());

pub(crate) fn init() {
    ()
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub mod generation {
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

    use super::*;
    use std::fs;

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("No generations found (Mac OS x86_64 specific)")]
        NoGenerations,
        #[error("std::io::Error: {0}")]
        StdIoError(#[from] std::io::Error),
    }

    /// Find existing generations of a given prefix.
    #[tracing::instrument(level = "debug")]
    pub(crate) fn all_generations(
        prefix: &str,
    ) -> Result<Box<dyn Iterator<Item = (usize, PathBuf)> + '_>, Error> {
        let work_dir = gucs::work_dir();
        let filtered = fs::read_dir(work_dir)?
            .flat_map(|entry| {
                let path = entry.ok()?.path();
                let stem = path.file_stem().and_then(|f| f.to_str())?.to_string();
                Some((stem, path))
            })
            .filter(move |(stem, _path)| stem.starts_with(prefix))
            .flat_map(|(stem, path)| {
                let generation = stem.split('_').last()?;
                let generation = generation.parse::<usize>().ok()?;
                Some((generation, path))
            });

        Ok(Box::from(filtered))
    }

    /// Get the next generation number to be created.
    ///
    /// If `vacuum` is set, this will pass the setting on to [`latest_generation`].
    #[tracing::instrument(level = "debug")]
    pub(crate) fn next_generation(prefix: &str, vacuum: bool) -> Result<usize, Error> {
        let latest = latest_generation(prefix, vacuum);
        Ok(latest.map(|this| this.0 + 1).unwrap_or_default())
    }

    /// Get the latest created generation night.
    ///
    /// If `vacuum` is set, this garbage collect old `so` files.
    #[tracing::instrument(level = "debug")]
    pub(crate) fn latest_generation(prefix: &str, vacuum: bool) -> Result<(usize, PathBuf), Error> {
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

        latest.ok_or(Error::NoGenerations)
    }
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    let removed = LOADED_SYMBOLS.remove(&fn_oid);
    if let Some(_symbol) = removed {
        tracing::info!("unloaded function");
    }
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn lookup_function(
    fn_oid: pg_sys::Oid,
) -> eyre::Result<&'static mut UserCrate<StateLoaded<'static>>> {
    match LOADED_SYMBOLS.entry(fn_oid) {
        entry @ Entry::Occupied(_) => {
            Ok(entry.or_insert_with(|| unreachable!("Occupied entry was vacant")))
        }
        entry @ Entry::Vacant(_) => {
            let crate_name = crate_name(fn_oid);

            #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
            let crate_name = {
                let mut crate_name = crate_name;
                let latest = generation::latest_generation(&crate_name, true)
                    .expect("Could not find latest generation.")
                    .0;

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };

            let shared_library = gucs::work_dir().join(&format!("{crate_name}{DLL_SUFFIX}"));
            let user_crate = UserCrate::load_file(fn_oid, &shared_library)?;

            Ok(entry.or_insert(user_crate))
        }
    }
}

pub(crate) fn symbol_name(fn_oid: pg_sys::Oid) -> String {
    format!("plrust_fn_oid_{}_wrapper", fn_oid)
}

#[tracing::instrument(level = "debug")]
pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> eyre::Result<(PathBuf, Output)> {
    let work_dir = gucs::work_dir();
    let pg_config = gucs::pg_config();
    let target_dir = work_dir.join("target");

    let generated = unsafe { UserCrate::try_from_fn_oid(fn_oid)? };
    let provisioned = generated.provision(&work_dir)?;
    let (built, output) = provisioned.build(&work_dir, pg_config, Some(target_dir.as_path()))?;

    let shared_object = built.shared_object();

    Ok((shared_object.into(), output))
}

pub fn crate_name(fn_oid: pg_sys::Oid) -> String {
    format!("plrust_fn_oid_{}", fn_oid)
}
