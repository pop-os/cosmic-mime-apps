// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::List;
use freedesktop_desktop_entry as fde;
use mime::Mime;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

/// Fetches available desktop entries and their mime type associations.
pub fn by_app(list: &List) -> BTreeMap<Arc<str>, Arc<App>> {
    let locales = fde::get_languages_from_env();
    let desktop_entries = fde::Iter::new(fde::default_paths()).entries(Some(&locales));

    let mut associations = BTreeMap::new();

    for desktop_entry in desktop_entries {
        let mime_types = desktop_entry.mime_type().unwrap_or_else(Vec::new);

        if let Some(name) = desktop_entry.name(&locales) {
            let mut app = App {
                appid: desktop_entry.appid.to_owned().into(),
                name: name.into_owned().into(),
                icon: desktop_entry.icon().unwrap_or("").to_owned().into(),
                path: desktop_entry.path.to_owned().into(),
                mime_types: mime_types.iter().fold(
                    Vec::with_capacity(mime_types.len()),
                    |mut vec, mime_type| {
                        if let Ok(mime_type) = mime_type.parse::<Mime>() {
                            vec.push(mime_type);
                        }

                        vec
                    },
                ),
            };

            if let Some(file_name) = desktop_entry.path.file_name().and_then(OsStr::to_str) {
                for (added_mime, added_apps) in &list.added_associations {
                    for name in added_apps {
                        if file_name == name.as_ref() && !app.mime_types.contains(added_mime) {
                            app.mime_types.push(added_mime.clone());
                        }
                    }
                }

                for (removed_mime, removed_apps) in &list.removed_associations {
                    for name in removed_apps {
                        if file_name == name.as_ref() && app.mime_types.contains(removed_mime) {
                            app.mime_types.retain(|mime| mime != removed_mime)
                        }
                    }
                }
            }

            app.mime_types.sort_unstable();
            associations.insert(app.appid.clone().into(), Arc::new(app));
        }
    }

    associations
}

#[derive(Clone, Debug)]
pub struct App {
    pub appid: Box<str>,
    pub name: Box<str>,
    pub icon: Box<str>,
    pub path: Box<Path>,
    pub mime_types: Vec<Mime>,
}
