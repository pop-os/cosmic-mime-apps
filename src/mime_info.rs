// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use mime::Mime;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::BTreeSet;
use std::io::BufReader;

pub fn mime_types() -> BTreeSet<Mime> {
    let mut mime_types = BTreeSet::new();

    if let Ok(base_dirs) = xdg::BaseDirectories::new() {
        for data_dir in base_dirs.get_data_dirs() {
            let packages_dir = data_dir.join("mime/packages");

            let Ok(packages_dir) = packages_dir.read_dir() else {
                continue;
            };

            for entry in packages_dir.filter_map(Result::ok) {
                let Ok(file) = std::fs::File::open(entry.path()) else {
                    continue;
                };

                let mut reader = Reader::from_reader(BufReader::new(file));
                reader.config_mut().trim_text(true);

                let mut buffer = Vec::new();

                loop {
                    buffer.clear();
                    match reader.read_event_into(&mut buffer) {
                        Ok(Event::Start(tag_start)) => {
                            if tag_start.name().as_ref() != b"mime-type" {
                                continue;
                            }

                            for attribute in tag_start.attributes().filter_map(Result::ok) {
                                if attribute.key.as_ref() != b"type" {
                                    continue;
                                }

                                let Ok(value) = attribute.unescape_value() else {
                                    continue;
                                };

                                if let Ok(mime) = value.parse() {
                                    mime_types.insert(mime);
                                }
                            }
                        }

                        Ok(Event::Eof) | Err(_) => break,

                        _ => continue,
                    }
                }
            }
        }
    }

    mime_types
}
