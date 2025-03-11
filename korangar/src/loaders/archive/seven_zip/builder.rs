//! Implements a writable instance of a 7zip File.
//!
//! This implementation is writing the data into the 7zip file right away and
//! will finish the file on drop.

use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use sevenz_rust2::{SevenZArchiveEntry, SevenZMethod, SevenZMethodConfiguration, SevenZWriter};

use super::SevenZipArchive;
use crate::loaders::archive::{Compression, Writable};

pub struct SevenZipArchiveBuilder {
    writer: Option<SevenZWriter<BufWriter<File>>>,
    folder_seen: HashSet<String>,
}

impl SevenZipArchiveBuilder {
    pub fn from_path(path: &Path) -> Self {
        let file = File::create(path).expect("can't create archive file");
        let writer = SevenZWriter::new(BufWriter::new(file)).unwrap();

        Self {
            writer: Some(writer),
            folder_seen: HashSet::default(),
        }
    }

    pub fn copy_file_from_archive(&mut self, archive: &SevenZipArchive, path: &str) {
        let Some(mut compression) = archive.file_is_compressed(path) else {
            return;
        };

        let path_with_slash = path.replace('\\', "/").to_string();

        let data = archive
            .reader
            .lock()
            .unwrap()
            .read_file(&path_with_slash)
            .expect("Unable to read file from archive");

        get_parent_directories(&path_with_slash)
            .iter()
            .for_each(|directory| self.add_directory(directory));

        // Custom overrides if we want to use different compressions on re-sync in
        // future versions.
        if path.ends_with(".dds") {
            compression = Compression::Off;
        }

        self.add_file(path, data, compression);
    }

    fn add_directory(&mut self, path: &str) {
        if !self.folder_seen.contains(path) {
            self.folder_seen.insert(path.to_string());
        }
    }
}

impl Writable for SevenZipArchiveBuilder {
    fn add_file(&mut self, path: &str, asset_data: Vec<u8>, compression: Compression) {
        let path = path.replace('\\', "/").to_string();

        get_parent_directories(&path)
            .iter()
            .for_each(|directory| self.add_directory(directory));

        if let Some(writer) = self.writer.as_mut() {
            let mut entry = SevenZArchiveEntry::new_file(&path);

            let now = sevenz_rust2::nt_time::FileTime::now();
            let has_date = now > sevenz_rust2::nt_time::FileTime::NT_TIME_EPOCH;

            entry.creation_date = now;
            entry.has_creation_date = has_date;
            entry.last_modified_date = now;
            entry.has_last_modified_date = has_date;

            match compression {
                Compression::Off => writer.set_content_methods(vec![SevenZMethodConfiguration::new(SevenZMethod::COPY)]),
                Compression::Default => writer.set_content_methods(vec![SevenZMethodConfiguration::new(SevenZMethod::LZMA)]),
            };

            writer
                .push_archive_entry(entry, Some(asset_data.as_slice()))
                .expect("Failed to write file to archive");
        }
    }

    fn finish(&mut self) -> Result<(), std::io::Error> {
        // File will be finished on drop
        Ok(())
    }
}

impl Drop for SevenZipArchiveBuilder {
    fn drop(&mut self) {
        if let Some(mut writer) = self.writer.take() {
            let mut folder_seen: Vec<String> = self.folder_seen.drain().collect();
            folder_seen.sort();

            for path in folder_seen.iter() {
                let folder = SevenZArchiveEntry::new_folder(path);
                writer
                    .push_archive_entry::<&[u8]>(folder, None)
                    .unwrap_or_else(|_| panic!("can't add path '{path}' to archive"));
            }

            writer.finish().unwrap();
        }
    }
}

fn get_parent_directories(asset_path: &str) -> Vec<String> {
    let mut result = Vec::new();
    let parts: Vec<&str> = asset_path.split('/').collect();

    for index in 1..parts.len() {
        let path = parts[..index].join("/");
        if !path.is_empty() {
            result.push(path);
        }
    }

    result
}
