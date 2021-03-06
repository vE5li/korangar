use procedural::*;
use derive_new::new;
use std::collections::HashMap;
use std::fs::read;
use yazi::*;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::ByteStream;
use crate::traits::ByteConvertable;

#[derive(Clone, ByteConvertable)]
pub struct FileHeader {
    encryption: [u8; 14],
    file_table_offset: u32,
    reserved_files: u32,
    raw_file_count: u32,
    version: u32,
}

impl FileHeader {

    pub fn validate_version(&self) {
        assert_eq!(self.version, 0x200, "invalid grf version");
    }

    pub fn get_file_table_offset(&self) -> usize {
        self.file_table_offset as usize
    }

    pub fn get_file_count(&self) -> usize {
        (self.raw_file_count - self.reserved_files) as usize - 7
    }
}

#[derive(Clone, ByteConvertable)]
pub struct FileTable {
    compressed_size: u32,
    uncompressed_size: u32,
}

impl FileTable {

    pub fn get_compressed_size(&self) -> usize {
        self.compressed_size as usize
    }
}

#[derive(Clone, Debug, ByteConvertable)]
pub struct FileInformation {
    pub file_name: String,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
    pub offset: u32,
}

#[derive(Clone, new)]
pub struct GameArchive {
    #[new(default)]
    cache: HashMap<String, Vec<u8>>,
    files: HashMap<String, FileInformation>,
    data: Vec<u8>,
}

impl GameArchive {

    fn load(&self, file_path: &str) -> Option<Vec<u8>> {

        let file_information = self.files.get(file_path)?;

        let mut byte_stream = ByteStream::new(&self.data);
        byte_stream.skip(file_information.offset as usize + 46);

        let compressed = byte_stream.slice(file_information.compressed_size_aligned as usize);
        let (uncompressed, _checksum) = decompress(&compressed, Format::Zlib).unwrap(); 

        uncompressed.into()
    }

    pub fn get(&mut self, path: &str) -> Option<Vec<u8>> {
        match self.cache.get(path) {
            Some(data) => data.clone().into(),
            None => self.load(path),
        }
    }
}

#[derive(Default)]
pub struct GameFileLoader {
    archives: HashMap<String, GameArchive>,
}

impl GameFileLoader {

    pub fn add_archive(&mut self, path: String) {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {}{}{}", MAGENTA, path, NONE));

        let bytes = read(path.clone()).unwrap_or_else(|_| panic!("failed to load archive from {}", path));
        let mut byte_stream = ByteStream::new(&bytes);

        assert!(byte_stream.string(16).as_str() == "Master of Magic", "failed to read magic number"); // TODO: change failed to invalid

        let file_header = FileHeader::from_bytes(&mut byte_stream, None);
        file_header.validate_version();

        byte_stream.skip(file_header.get_file_table_offset());
        let file_table = FileTable::from_bytes(&mut byte_stream, None);

        let compressed = byte_stream.slice(file_table.get_compressed_size());
        let (decompressed, _checksum) = decompress(&compressed, Format::Zlib).unwrap();

        let file_count = file_header.get_file_count();

        let mut byte_stream = ByteStream::new(&decompressed);
        let mut files = HashMap::with_capacity(file_count);

        for _index in 0..file_count {
            let file_information = FileInformation::from_bytes(&mut byte_stream, None);

            if file_information.file_name.contains("cursor") || file_information.file_name.contains("mouse") {
                println!("{}", file_information.file_name);
            }

            files.insert(file_information.file_name.clone(), file_information);
        }

        #[cfg(feature = "debug")]
        timer.stop();

        let game_archive = GameArchive::new(files, bytes);
        self.archives.insert(path, game_archive);
    }

    pub fn get(&mut self, path: &str) -> Result<Vec<u8>, String> {

        let result = self.archives
            .values_mut() // convert this to a multithreaded iter ?
            .find_map(|archive| archive.get(path))
            .ok_or(format!("failed to find file {}", path));

        if result.is_err() { // TEMP

            #[cfg(feature = "debug")]
            print_debug!("failed to find file {}; tying to replace it with placeholder", path);

            let delimiter = path.len() - 4;
            match &path[delimiter..] {
                ".bmp" | ".BMP" => return self.get("data\\texture\\BLACK.BMP"),
                ".rsm" => return self.get("data\\model\\abyss\\coin_j_01.rsm"),
                ".spr" => return self.get("data\\sprite\\npc\\1_f_maria.spr"),
                ".act" => return self.get("data\\sprite\\npc\\1_f_maria.act"),
                _other => {},
            }
        }

        result
    }
}
