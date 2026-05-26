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
#[cfg(feature = "tokio")]
use tokio::fs::File;

pub fn apps_for_mime<'a>(
    mime: &'a Mime,
    apps: &'a BTreeMap<Arc<str>, Arc<App>>,
) -> impl Iterator<Item = (&'a Arc<str>, &'a Arc<App>)> + 'a {
    apps.iter().filter(|(_, app)| app.mime_types.contains(mime))
}

/// Known mime-types which have configured applications
pub fn configured_mime_types(apps: &BTreeMap<Arc<str>, Arc<App>>) -> BTreeSet<Mime> {
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
    let base_dirs = xdg::BaseDirectories::new();

    // user overrides
    if let Some(config_home) = base_dirs.get_config_home() {
        paths = apply_existing_paths(paths, config_home, desktop_filename);
    }

    // sysadmin and ISV overrides
    for config_dir in base_dirs.get_config_dirs() {
        paths = apply_existing_paths(paths, config_dir, desktop_filename)
    }

    // distribution-provided defaults
    for data_dir in base_dirs.get_data_dirs() {
        paths = apply_existing_paths(paths, data_dir.join("applications"), desktop_filename)
    }

    paths
}

/// Returns `~/.config/${XDG_CURRENT_DESKTOP}-mimeapps.list` if it exists,
/// otherwise `~/.config/mimeapps.list`.
pub fn local_list_path() -> Option<PathBuf> {
    let base_dirs = xdg::BaseDirectories::new();
    let home = base_dirs.get_config_home()?;

    let path = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|de| home.join([&de.to_ascii_lowercase(), "-mimeapps.list"].concat()))
        .filter(|de| de.exists())
        .unwrap_or_else(|| home.join("mimeapps.list"));

    Some(path)
}

/// Copy the existing mimeapps if this is not found. Create an empty file if neither exists.
#[cfg(feature = "tokio")]
pub async fn load_user_mimeapps() -> std::io::Result<(List, File)> {
    use std::io::SeekFrom;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let base_dirs = xdg::BaseDirectories::new();
    let Some(home) = base_dirs.get_config_home() else {
        return Err(std::io::Error::other("XDG config home not set"));
    };

    let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") else {
        return Err(std::io::Error::other("XDG_CURRENT_DESKTOP unset"));
    };

    let default_mimeapps = &*home.join("mimeapps.list");
    let desktop_mimeapps = &*home.join([&desktop.to_ascii_lowercase(), "-mimeapps.list"].concat());

    let mut mimeapps_file = if desktop_mimeapps.exists() {
        tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&desktop_mimeapps)
            .await?
    } else {
        let desktop_mimeapps_fut = File::create_new(&desktop_mimeapps);
        if default_mimeapps.exists() {
            let (mut default_file, mut desktop_file) =
                futures_util::future::try_join(File::open(&default_mimeapps), desktop_mimeapps_fut)
                    .await?;

            if let Ok(length) = default_file.metadata().await.map(|m| m.len()) {
                _ = desktop_file.set_len(length).await;
            }

            tokio::io::copy(&mut default_file, &mut desktop_file).await?;
            desktop_file.seek(SeekFrom::Current(0)).await?;
            desktop_file
        } else {
            File::create_new(&desktop_mimeapps).await?
        }
    };

    let capacity = mimeapps_file
        .metadata()
        .await
        .ok()
        .and_then(|m| usize::try_from(m.len()).ok())
        .unwrap_or_default();

    let mut buffer = String::with_capacity(capacity);
    mimeapps_file.read_to_string(&mut buffer).await?;
    mimeapps_file.seek(SeekFrom::Start(0)).await?;

    let mut list = List::default();
    list.load_from(&buffer);
    Ok((list, mimeapps_file))
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

#[cfg(test)]
mod test {
    use mime::Mime;

    use crate::List;

    #[test]
    fn can_load_multiple_mimeapps_files() {
        let mut list = List::default();
        let paths = vec!["fixtures/mimeapps-1.list", "fixtures/mimeapps-2.list"];

        list.load_from_paths(&paths);

        assert_eq!(list.default_apps.len(), 3);
        assert_eq!(list.added_associations.len(), 3);
        assert_eq!(list.removed_associations.len(), 3);
    }

    #[test]
    fn can_parse_mime_type() {
        let mut list = List::default();
        let paths = vec!["fixtures/mimeapps-1.list"];

        list.load_from_paths(&paths);

        assert_eq!(list.default_apps.len(), 2);

        let text_plain = "text/plain".parse::<Mime>().unwrap();
        let default_apps = list.default_app_for(&text_plain).unwrap();
        assert_eq!(default_apps.len(), 1);
        assert_eq!(
            default_apps[0].to_string(),
            "com.system76.CosmicEdit.desktop"
        );
    }
}
