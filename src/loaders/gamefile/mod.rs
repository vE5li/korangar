use derive_new::new;
use std::collections::HashMap;
use std::fs::read;
use yazi::*;

#[cfg(feature = "debug")]
use debug::*;
use super::ByteStream;

#[derive(new)]
pub struct GameFileLoader {
    #[new(default)]
    cache: HashMap<String, String>,
}

impl GameFileLoader {

    fn load(&mut self, path: String) -> String {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {}{}{}", magenta(), path, none()));

        let bytes = read(path.clone()).expect("u r an idiot!");
        let mut byte_stream = ByteStream::new(bytes.iter());

        let magic = byte_stream.string(16);
        assert!(&magic == "Master of Magic", "failed to read magic number"); // TODO: change failed to invalid

        let allow_encryption = byte_stream.slice(14);
        let file_table_offset = byte_stream.byte();
        let number1 = byte_stream.byte();
        let number2 = byte_stream.byte();
        let version = byte_stream.byte();

        #[cfg(feature = "debug_gamefiles")]
        {
            print_debug!("allow encryption {}{:?}{}", magenta(), allow_encryption, none());
            print_debug!("file table offset {}{}{}", magenta(), file_table_offset, none());
            print_debug!("number1 {}{}{}", magenta(), number1, none());
            print_debug!("number2 {}{}{}", magenta(), number2, none());
            print_debug!("version {}{}{}", magenta(), version, none());
        }

        byte_stream.skip(file_table_offset as usize);

        let uncompressed_length = byte_stream.integer32() as u32;
        let compressed_length = byte_stream.integer32() as u32;

        #[cfg(feature = "debug_gamefiles")]
        {
            print_debug!("uncompressed length {}{:?}{}", magenta(), uncompressed_length, none());
            print_debug!("compressed length {}{}{}", magenta(), compressed_length, none());
        }

        let compressed = byte_stream.slice(compressed_length as usize);
        let (decompressed, checksum) = decompress(&compressed, Format::Zlib).unwrap();

        //let mut destination_buffer = [0; 16];
        //let mut reader = std::io::Cursor::new(zip_buffer);
        //let mut zip_archive = ZipFile::new(reader).expect("di'nt work");
        //let zip_file = zip_archive.read(&destination_buffer[..]);

        //#[cfg(feature = "debug_gamefiles")]
        //{
        //    print_debug!("foo {}{:?}{}", magenta(), zip_archive.len(), none());
        //    print_debug!("{}{:?}{}", magenta(), zip_archive, none());
        //}

        #[cfg(feature = "debug")]
        timer.stop();

        return String::from("fooo");
    }

    pub fn get(&mut self, path: String) -> String {
        match self.cache.get(&path) {
            Some(file) => return file.clone(),
            None => return self.load(path),
        }
    }
}
