//! A GRF file containing game assets.
mod builder;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer};
use ragnarok_bytes::{ByteStream, FixedByteSize, FromBytes};
use ragnarok_formats::archive::{AssetTable, FileTableRow, Header};
use yazi::{decompress, Format};
pub use self::builder::NativeArchiveBuilder;
use crate::loaders::archive::Archive;

/// Represents a GRF file. GRF Files are an archive to store game assets.
/// Each GRF contains a [`Header`] with metadata (number of files, size,
/// etc.) and a table [`AssetTable`] with information about individual assets.
type FileTable = HashMap<String, FileTableRow>;

pub struct NativeArchive {
    file_table: FileTable,
    file_handle: Mutex<File>,
}

pub struct DesDecryption {}

impl Archive for NativeArchive {
    fn from_path(path: &Path) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {}", path.display().magenta()));
        let mut file = File::open(path).unwrap();

        let mut file_header_buffer = vec![0u8; Header::size_in_bytes()];
        file.read_exact(&mut file_header_buffer).unwrap();
        let file_header = Header::from_bytes(&mut ByteStream::<()>::without_metadata(&file_header_buffer)).unwrap();

        assert_eq!(file_header.version, 0x200, "invalid grf version");

        let _ = file.seek(SeekFrom::Current(file_header.file_table_offset as i64)).unwrap();
        let mut file_table_buffer = vec![0; AssetTable::size_in_bytes()];

        file.read_exact(&mut file_table_buffer).unwrap();
        let file_table = AssetTable::from_bytes(&mut ByteStream::<()>::without_metadata(&file_table_buffer)).unwrap();

        let mut compressed_file_table_buffer = vec![0u8; file_table.compressed_size as usize];
        file.read_exact(&mut compressed_file_table_buffer).unwrap();
        let (decompressed, _checksum) = decompress(&compressed_file_table_buffer, Format::Zlib).unwrap();

        let file_count = file_header.get_file_count();

        let mut file_table_byte_stream = ByteStream::<()>::without_metadata(&decompressed);
        let mut assets = HashMap::with_capacity(file_count);

        for _index in 0..file_count {
            let file_information = FileTableRow::from_bytes(&mut file_table_byte_stream).unwrap();
            let file_name = file_information.file_name.to_lowercase();

            assets.insert(file_name, file_information);
        }

        #[cfg(feature = "debug")]
        timer.stop();

        // TODO: only take 64..? bytes so that loaded game archives can be extended
        // aswell
        Self {
            file_table: assets,
            file_handle: Mutex::new(file),
        }
    }

    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>> {
        self.file_table.get(asset_path).and_then(|file_information| {
            let mut compressed_file_buffer = vec![0u8; file_information.compressed_size_aligned as usize];
            let position = file_information.offset as u64 + Header::size_in_bytes() as u64;

            {
                // Since the calling threads are sharing the IO bandwidth anyhow, I don't think
                // we need to allow this to run in parallel.
                let mut file_handle = self.file_handle.lock().unwrap();
                file_handle.seek(SeekFrom::Start(position)).unwrap();
                file_handle
                    .read_exact(&mut compressed_file_buffer)
                    .expect("Can't read archive content");
            }

            DesDecryption::decrypt_file(file_information, &mut compressed_file_buffer);

            let (uncompressed_file_buffer, _checksum) =
                decompress(&compressed_file_buffer, Format::Zlib).expect("Can't decompress archive content");

            Some(uncompressed_file_buffer)
        })
    }

    fn get_lua_files(&self, lua_files: &mut Vec<String>) {
        let files = self
            .file_table
            .iter()
            .filter(|(file_name, row)| file_name.ends_with(".lub") && row.flags == 0x01)
            .map(|(file_name, _)| file_name.clone());

        lua_files.extend(files);
    }
}

/**
 * Straight ripoff from Tokeiburu's GRFEditor code:
 *   - https://github.com/Tokeiburu/GRFEditor/blob/main/Utilities/DesDecryption.cs
 *   - https://github.com/Tokeiburu/GRFEditor/blob/main/GRF/Core/FileEntry.cs
 */
impl DesDecryption {
    const IP_TABLE: [u8; 64] = [
        58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4,
        62, 54, 46, 38, 30, 22, 14, 6, 64, 56, 48, 40, 32, 24, 16, 8,
        57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3,
        61, 53, 45, 37, 29, 21, 13, 5, 63, 55, 47, 39, 31, 23, 15, 7
    ];
    const FP_TABLE: [u8; 64] = [
        40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31,
        38, 6, 46, 14, 54, 22, 62, 30, 37, 5, 45, 13, 53, 21, 61, 29,
        36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11, 51, 19, 59, 27,
        34, 2, 42, 10, 50, 18, 58, 26, 33, 1, 41, 9, 49, 17, 57, 25
    ];
    const TP_TABLE: [u8; 32] = [
        16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10,
        2, 8, 24, 14, 32, 27, 3, 9, 19, 13, 30, 6, 22, 11, 4, 25
    ];
    const S_TABLE: [[u8; 64]; 4] = [
        [
            0xef, 0x03, 0x41, 0xfd, 0xd8, 0x74, 0x1e, 0x47, 0x26, 0xef, 0xfb, 0x22, 0xb3, 0xd8, 0x84, 0x1e,
            0x39, 0xac, 0xa7, 0x60, 0x62, 0xc1, 0xcd, 0xba, 0x5c, 0x96, 0x90, 0x59, 0x05, 0x3b, 0x7a, 0x85,
            0x40, 0xfd, 0x1e, 0xc8, 0xe7, 0x8a, 0x8b, 0x21, 0xda, 0x43, 0x64, 0x9f, 0x2d, 0x14, 0xb1, 0x72,
            0xf5, 0x5b, 0xc8, 0xb6, 0x9c, 0x37, 0x76, 0xec, 0x39, 0xa0, 0xa3, 0x05, 0x52, 0x6e, 0x0f, 0xd9
        ],
        [
            0xa7, 0xdd, 0x0d, 0x78, 0x9e, 0x0b, 0xe3, 0x95, 0x60, 0x36, 0x36, 0x4f, 0xf9, 0x60, 0x5a, 0xa3,
            0x11, 0x24, 0xd2, 0x87, 0xc8, 0x52, 0x75, 0xec, 0xbb, 0xc1, 0x4c, 0xba, 0x24, 0xfe, 0x8f, 0x19,
            0xda, 0x13, 0x66, 0xaf, 0x49, 0xd0, 0x90, 0x06, 0x8c, 0x6a, 0xfb, 0x91, 0x37, 0x8d, 0x0d, 0x78,
            0xbf, 0x49, 0x11, 0xf4, 0x23, 0xe5, 0xce, 0x3b, 0x55, 0xbc, 0xa2, 0x57, 0xe8, 0x22, 0x74, 0xce
        ],
        [
            0x2c, 0xea, 0xc1, 0xbf, 0x4a, 0x24, 0x1f, 0xc2, 0x79, 0x47, 0xa2, 0x7c, 0xb6, 0xd9, 0x68, 0x15,
            0x80, 0x56, 0x5d, 0x01, 0x33, 0xfd, 0xf4, 0xae, 0xde, 0x30, 0x07, 0x9b, 0xe5, 0x83, 0x9b, 0x68,
            0x49, 0xb4, 0x2e, 0x83, 0x1f, 0xc2, 0xb5, 0x7c, 0xa2, 0x19, 0xd8, 0xe5, 0x7c, 0x2f, 0x83, 0xda,
            0xf7, 0x6b, 0x90, 0xfe, 0xc4, 0x01, 0x5a, 0x97, 0x61, 0xa6, 0x3d, 0x40, 0x0b, 0x58, 0xe6, 0x3d
        ],
        [
            0x4d, 0xd1, 0xb2, 0x0f, 0x28, 0xbd, 0xe4, 0x78, 0xf6, 0x4a, 0x0f, 0x93, 0x8b, 0x17, 0xd1, 0xa4,
            0x3a, 0xec, 0xc9, 0x35, 0x93, 0x56, 0x7e, 0xcb, 0x55, 0x20, 0xa0, 0xfe, 0x6c, 0x89, 0x17, 0x62,
            0x17, 0x62, 0x4b, 0xb1, 0xb4, 0xde, 0xd1, 0x87, 0xc9, 0x14, 0x3c, 0x4a, 0x7e, 0xa8, 0xe2, 0x7d,
            0xa0, 0x9f, 0xf6, 0x5c, 0x6a, 0x09, 0x8d, 0xf0, 0x0f, 0xe3, 0x53, 0x25, 0x95, 0x36, 0x28, 0xcb
        ]
    ];
    const MASK: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

    const FLAG_HEADER_CRYPTED: u8 = 1 << 1;
    const FLAG_DATA_CRYPTED: u8 = 1 << 2;

    pub fn decrypt_file(file_information: &FileTableRow, data: &mut Vec<u8>) {
        let mut decrypt = false;
        let mut cycle = 0;

        if (file_information.flags & Self::FLAG_HEADER_CRYPTED) == Self::FLAG_HEADER_CRYPTED {
            decrypt = true;
            cycle = 1;

            let mut i: u32 = 10;
            while file_information.compressed_size >= i {
                cycle += 1;
                i = match i.checked_mul(10) {
                    Some(i) => i,
                    None => break,
                };
            }
        } else if (file_information.flags & Self::FLAG_DATA_CRYPTED) == Self::FLAG_DATA_CRYPTED {
            decrypt = true;
        }

        if decrypt {
            Self::decrypt_file_data(data, cycle == 0, cycle);
        }
    }

    pub fn decrypt_file_data(data: &mut Vec<u8>, file_type: bool, cycle: usize) {
        if data.len() % 8 != 0 {
            let ideal_size = ((data.len() / 8) + 1) * 8;
            let data_len = data.len();
            let mut data_fixed = vec![0u8; ideal_size];

            data_fixed[..data_len].copy_from_slice(&data[..data_len]);
            Self::decode_file_data(&mut data_fixed, file_type, cycle);

            // Create a temporary slice to avoid the double borrow
            let decoded_data = data_fixed[..data_len].to_vec();
            data.copy_from_slice(&decoded_data);
        } else {
            Self::decode_file_data(data, file_type, cycle);
        }
    }

    fn decode_file_data(data: &mut [u8], file_type: bool, cycle: usize) {
        let mut cnt = 0;
        let mut offset = 0;
        let length = data.len();

        let cycle = match cycle {
            c if c < 3 => 3,
            c if c < 5 => c + 1,
            c if c < 7 => c + 9,
            c => c + 15,
        };

        let mut lop = 0;
        while lop * 8 < length {
            if lop < 20 || (!file_type && lop % cycle == 0) {
                Self::des_decode_block(data, offset);
            } else {
                if cnt == 7 && !file_type {
                    let mut tmp = [0u8; 8];
                    tmp.copy_from_slice(&data[offset..offset + 8]);
                    cnt = 0;

                    data[offset] = tmp[3];
                    data[offset + 1] = tmp[4];
                    data[offset + 2] = tmp[6];
                    data[offset + 3] = tmp[0];
                    data[offset + 4] = tmp[1];
                    data[offset + 5] = tmp[2];
                    data[offset + 6] = tmp[5];

                    let a = match tmp[7] {
                        0x00 => 0x2b,
                        0x2b => 0x00,
                        0x01 => 0x68,
                        0x68 => 0x01,
                        0x48 => 0x77,
                        0x77 => 0x48,
                        0x60 => 0xff,
                        0xff => 0x60,
                        0x6c => 0x80,
                        0x80 => 0x6c,
                        0xb9 => 0xc0,
                        0xc0 => 0xb9,
                        0xeb => 0xfe,
                        0xfe => 0xeb,
                        x => x,
                    };
                    data[offset + 7] = a;
                }
                cnt += 1;
            }
            offset += 8;
            lop += 1;
        }
    }

    fn round_function(src: &mut [u8]) {
        let mut block = [0u8; 8];

        block[0] = ((src[7] << 5) | (src[4] >> 3)) & 0x3f;
        block[1] = ((src[4] << 1) | (src[5] >> 7)) & 0x3f;
        block[2] = ((src[4] << 5) | (src[5] >> 3)) & 0x3f;
        block[3] = ((src[5] << 1) | (src[6] >> 7)) & 0x3f;
        block[4] = ((src[5] << 5) | (src[6] >> 3)) & 0x3f;
        block[5] = ((src[6] << 1) | (src[7] >> 7)) & 0x3f;
        block[6] = ((src[6] << 5) | (src[7] >> 3)) & 0x3f;
        block[7] = ((src[7] << 1) | (src[4] >> 7)) & 0x3f;

        for i in 0..Self::S_TABLE.len() {
            block[i] = (Self::S_TABLE[i][block[i * 2] as usize] & 0xf0) |
                (Self::S_TABLE[i][block[i * 2 + 1] as usize] & 0x0f);
        }

        block[4..8].fill(0);

        for i in 0..Self::TP_TABLE.len() {
            let j = Self::TP_TABLE[i] - 1;
            if (block[(j >> 3) as usize] & Self::MASK[(j & 7) as usize]) != 0 {
                block[(i >> 3) + 4] |= Self::MASK[i & 7];
            }
        }

        for i in 0..4 {
            src[i] ^= block[i + 4];
        }
    }

    pub fn des_decode_block(src: &mut [u8], offset: usize) {
        let mut block = [0u8; 8];
        block.copy_from_slice(&src[offset..offset + 8]);

        Self::ip(&mut block);
        Self::round_function(&mut block);
        Self::fp(&mut block);

        src[offset..offset + 8].copy_from_slice(&block);
    }

    fn fp(src: &mut [u8]) {
        let mut block = [0u8; 8];

        for i in 0..Self::FP_TABLE.len() {
            let j = Self::FP_TABLE[i] - 1;

            if (src[(j >> 3) as usize & 7] & Self::MASK[(j & 7) as usize]) != 0 {
                block[(i >> 3) & 7] |= Self::MASK[i & 7];
            }
        }

        src[..8].copy_from_slice(&block);
    }

    fn ip(src: &mut [u8]) {
        let mut block = [0u8; 8];

        for i in 0..Self::IP_TABLE.len() {
            let j = Self::IP_TABLE[i] - 1;

            if (src[(j >> 3) as usize & 7] & Self::MASK[(j & 7) as usize]) != 0 {
                block[(i >> 3) & 7] |= Self::MASK[i & 7];
            }
        }

        src[..8].copy_from_slice(&block);
    }
}
