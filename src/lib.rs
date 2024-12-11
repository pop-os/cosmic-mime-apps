// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub mod associations;
#[doc(inline)]
pub use associations::App;

pub mod list;
#[doc(inline)]
pub use list::List;

pub mod mime_info;

use mime::Mime;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

pub fn apps_for_mime<'a>(
    mime: &'a Mime,
    apps: &'a BTreeMap<Arc<str>, Arc<App>>,
) -> impl Iterator<Item = (&'a Arc<str>, &'a Arc<App>)> + 'a {
    apps.iter().filter(|(_, app)| app.mime_types.contains(mime))
}

/// Known mime-types which have configured applications
pub fn configured_mime_types<'a>(apps: &'a BTreeMap<Arc<str>, Arc<App>>) -> BTreeSet<Mime> {
    let mut bset = BTreeSet::new();

    apps.iter().for_each(|(_, app)| {
        app.mime_types.iter().for_each(|mime_type| {
            if !bset.contains(mime_type) {
                bset.insert(mime_type.clone());
            }
        });
    });

    bset
}

/// https://specifications.freedesktop.org/mime-apps-spec/latest/file.html
pub fn list_paths() -> Vec<PathBuf> {
    // Lists for the current desktop take precedence over the default mimeapps.list
    let desktop_filename = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|de| [&de.to_ascii_lowercase(), "-mimeapps.list"].concat());

    let desktop_filename = desktop_filename.as_deref();

    let mut paths = Vec::with_capacity(8);

    if let Ok(base_dirs) = xdg::BaseDirectories::new() {
        // user overrides
        paths = apply_existing_paths(paths, base_dirs.get_config_home(), desktop_filename);

        // sysadmin and ISV overrides
        for config_dir in base_dirs.get_config_dirs() {
            paths = apply_existing_paths(paths, config_dir, desktop_filename)
        }

        // distribution-provided defaults
        for data_dir in base_dirs.get_data_dirs() {
            paths = apply_existing_paths(paths, data_dir.join("applications"), desktop_filename)
        }
    }

    paths
}

/// Returns `~/.config/${XDG_CURRENT_DESKTOP}-mimeapps.list` if it exists,
/// otherwise `~/.config/mimeapps.list`.
pub fn local_list_path() -> Option<PathBuf> {
    let base_dirs = xdg::BaseDirectories::new().ok()?;
    let home = base_dirs.get_config_home();

    let path = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|de| home.join([&de.to_ascii_lowercase(), "mimeapps.list"].concat()))
        .filter(|de| de.exists())
        .unwrap_or_else(|| home.join("mimeapps.list"));

    Some(path)
}

fn apply_existing_paths(
    mut paths: Vec<PathBuf>,
    mut dir: PathBuf,
    desktop_filename: Option<&str>,
) -> Vec<PathBuf> {
    const MIMEAPPS_LIST_FILENAME: &str = "mimeapps.list";

    if !dir.exists() {
        return paths;
    }

    if let Some(filename) = desktop_filename {
        let mimeapps_list_path = dir.join(filename);
        if mimeapps_list_path.exists() {
            paths.push(mimeapps_list_path);
        }
    }

    dir.push(MIMEAPPS_LIST_FILENAME);

    if dir.exists() {
        paths.push(dir);
    }

    paths
}
