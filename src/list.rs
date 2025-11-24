// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use mime::Mime;
use std::{collections::BTreeMap, io::Read, path::Path, str::Lines};

#[derive(Clone, Debug, Default)]
pub struct List {
    pub default_apps: BTreeMap<Mime, Vec<Box<str>>>,
    pub added_associations: BTreeMap<Mime, Vec<Box<str>>>,
    pub removed_associations: BTreeMap<Mime, Vec<Box<str>>>,
}

impl List {
    pub fn set_default_app(&mut self, mime: Mime, app: String) {
        self.default_apps
            .entry(mime)
            .and_modify(|associations| {
                associations.clear();
            })
            .or_default()
            .push(app.into());
    }

    pub fn default_app_for(&self, mime: &Mime) -> Option<&Vec<Box<str>>> {
        self.default_apps.get(mime)
    }

    pub fn default_calendar(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"text/calendar".parse().unwrap())
    }

    pub fn default_file_manager(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"inode/directory".parse().unwrap())
    }

    pub fn default_mail_client(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"x-scheme-handler/mailto".parse().unwrap())
    }

    pub fn default_audio_player(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"audio/x-flac".parse().unwrap())
    }

    pub fn default_image_viewer(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"image/x-jpeg".parse().unwrap())
    }

    pub fn default_video_player(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"video/x-matroska".parse().unwrap())
    }

    pub fn default_web_browser(&self) -> Option<&Vec<Box<str>>> {
        self.default_app_for(&"x-scheme-handler/http".parse().unwrap())
    }

    pub fn load_from(&mut self, list: &str) {
        for token in Iter::new(list) {
            let (map, mime, apps) = match token {
                Ast::AddAssociation(mime, apps) => (&mut self.added_associations, mime, apps),
                Ast::RemoveAssociation(mime, apps) => (&mut self.removed_associations, mime, apps),
                Ast::DefaultApp(mime, apps) => (&mut self.default_apps, mime, apps),
            };

            if let Ok(mime) = mime.parse::<Mime>() {
                map.entry(mime).or_insert_with(|| {
                    apps.split(';')
                        .filter(|app| !app.is_empty())
                        .map(Box::from)
                        .collect()
                });
            }
        }
    }

    pub fn load_from_paths<P: AsRef<Path>>(&mut self, paths: &[P]) {
        self.default_apps.clear();
        self.added_associations.clear();
        self.removed_associations.clear();

        let mut buffer = String::new();

        for list_path in paths {
            let Ok(mut file) = std::fs::OpenOptions::new().read(true).open(list_path) else {
                continue;
            };

            let Ok(metadata) = file.metadata() else {
                continue;
            };

            buffer.clear();

            if metadata.len() > buffer.len() as u64 {
                buffer.reserve_exact((metadata.len() - buffer.len() as u64) as usize);
            }

            if file.read_to_string(&mut buffer).is_ok() {
                self.load_from(&buffer);
            }
        }
    }
}

impl std::fmt::Display for List {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sections = &[
            ("[Default Applications]", &self.default_apps),
            ("[Added Associations]", &self.added_associations),
            ("[Removed Associations]", &self.removed_associations),
        ];

        let content = sections
            .iter()
            .filter(|(_, map)| !map.is_empty())
            .map(|(name, map)| {
                let entries = map
                    .iter()
                    .map(|(mime, apps)| format!("{}={}", mime.essence_str(), apps.join(";")))
                    .collect::<Vec<String>>()
                    .join("\n");
                format!("{}\n{}", name, entries)
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        write!(f, "{}", content)
    }
}

pub enum Ast<'a> {
    AddAssociation(&'a str, &'a str),
    DefaultApp(&'a str, &'a str),
    RemoveAssociation(&'a str, &'a str),
}

pub struct Iter<'a> {
    lines: Lines<'a>,
    active_property: Option<AstMap>,
}

impl<'a> Iter<'a> {
    pub fn new(list: &'a str) -> Self {
        Self {
            lines: list.lines(),
            active_property: None,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Ast<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?.trim();

            if line.is_empty() {
                continue;
            }

            match line {
                "[Default Applications]" => {
                    self.active_property = Some(AstMap::DefaultApplications)
                }
                "[Added Associations]" => self.active_property = Some(AstMap::AddedAssociations),
                "[Removed Associations]" => {
                    self.active_property = Some(AstMap::RemovedAssociations)
                }
                handler => {
                    if handler.starts_with("#") {
                        continue;
                    }

                    if let Some((property, (mime, apps))) =
                        self.active_property.zip(handler.split_once('='))
                    {
                        return Some(match property {
                            AstMap::AddedAssociations => Ast::AddAssociation(mime, apps),
                            AstMap::DefaultApplications => Ast::DefaultApp(mime, apps),
                            AstMap::RemovedAssociations => Ast::RemoveAssociation(mime, apps),
                        });
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum AstMap {
    AddedAssociations,
    DefaultApplications,
    RemovedAssociations,
}
