//! Implements the mixcrypt scheme use by the original client.

use ragnarok_formats::archive::FileTableRow;

/// File uses a mixed crypto (Simple DES + Shuffle):
/// - Encrypts the first 0x14 blocks
/// - Encrypts blocks at interval N, where N equals the digit count of original
///   compressed size
/// - For every 7th non-encrypted block: shuffles and modifies the byte values
pub const GRF_FLAG_FULL_MIX_CRYPT: u8 = 1 << 1;

/// Only the first 0x14 compressed blocks are encrypted with DES.
pub const GRF_FLAG_HEADER_DES_CRYPT: u8 = 1 << 2;

/// First 0x14 blocks are always encrypted.
const HEADER_BLOCKS_SIZE: usize = 0x14;

const BLOCK_SIZE: usize = 8;

/// Decrypts a file using the appropriate decryption method.
pub fn decrypt_file(file_information: &FileTableRow, data: &mut [u8]) {
    if let Some((is_limited_crypt, cycle)) = determine_encryption_scheme(file_information.flags, file_information.compressed_size) {
        decrypt_data(data, is_limited_crypt, cycle);
    }
}

/// Determines the encryption scheme used for the file.
/// Returns `Some((only_header_is_encrypted, cycle_length))` if the file is
/// encrypted, `None` otherwise.
///
/// - only_header_is_encrypted: `true` if only first 0x14 blocks are encrypted
///   (GRF_FLAG_HEADER_DES_CRYPT) `false` if using mixed encryption scheme
///   (GRF_FLAG_FULL_MIX_CRYPT)
/// - cycle_length: for mixed encryption, determines interval of encrypted
///   blocks based on compressed file size digits.
fn determine_encryption_scheme(flags: u8, compressed_size: u32) -> Option<(bool, usize)> {
    if flags & GRF_FLAG_FULL_MIX_CRYPT != 0 {
        let digits = count_digits(compressed_size);
        let cycle = calculate_encryption_cycle(digits);
        Some((false, cycle))
    } else if flags & GRF_FLAG_HEADER_DES_CRYPT != 0 {
        Some((true, 0))
    } else {
        None
    }
}

/// Count the number of digits in the compressed file size.
fn count_digits(size: u32) -> usize {
    let mut digits = 0;
    let mut rest_size = size;

    while rest_size > 0 {
        rest_size /= 10;
        digits += 1;
    }

    if digits < 1 {
        digits = 1;
    }

    digits
}

/// Transform digit count into encryption cycle length.
fn calculate_encryption_cycle(digit: usize) -> usize {
    match digit {
        0..3 => 3,
        3..5 => digit + 1,
        5..7 => digit + 9,
        _ => digit + 15,
    }
}

fn decrypt_data(data: &mut [u8], only_header_is_encrypted: bool, cycle: usize) {
    if data.len().is_multiple_of(BLOCK_SIZE) {
        decrypt_data_blocks(data, only_header_is_encrypted, cycle);
    } else {
        let original_length = data.len();
        let full_blocks_size = (original_length / BLOCK_SIZE) * BLOCK_SIZE;
        let remainder_size = original_length % BLOCK_SIZE;

        // Decrypt full blocks in-place.
        if full_blocks_size > 0 {
            decrypt_data_blocks(&mut data[..full_blocks_size], only_header_is_encrypted, cycle);
        }

        // Handle the last incomplete block.
        let mut last_block = [0u8; BLOCK_SIZE];
        last_block[..remainder_size].copy_from_slice(&data[full_blocks_size..]);
        decrypt_data_blocks(&mut last_block, only_header_is_encrypted, cycle);
        data[full_blocks_size..].copy_from_slice(&last_block[..remainder_size]);
    }
}

fn decrypt_data_blocks(data: &mut [u8], only_header_is_encrypted: bool, cycle: usize) {
    for (block_number, block_data) in data.chunks_exact_mut(BLOCK_SIZE).enumerate() {
        if should_apply_des(block_number, only_header_is_encrypted, cycle) {
            let mut block = u64::from_be_bytes(block_data.try_into().unwrap());
            block = decode_des_block(block);
            block_data.copy_from_slice(&block.to_be_bytes());
        } else if should_apply_scramble(block_number, only_header_is_encrypted) {
            scramble_block(block_data);
        }
    }
}

fn should_apply_des(block_number: usize, only_header_is_encrypted: bool, cycle: usize) -> bool {
    block_number < HEADER_BLOCKS_SIZE || (!only_header_is_encrypted && block_number.is_multiple_of(cycle))
}

fn should_apply_scramble(block_num: usize, only_header_is_encrypted: bool) -> bool {
    !only_header_is_encrypted && block_num % 8 == 7
}

fn scramble_block(block: &mut [u8]) {
    let mut block_copy = [0; BLOCK_SIZE];
    block_copy.copy_from_slice(block);

    const SHUFFLE: [usize; 7] = [3, 4, 6, 0, 1, 2, 5];
    for (index, &position) in SHUFFLE.iter().enumerate() {
        block[index] = block_copy[position];
    }

    block[7] = match block_copy[7] {
        0x00 => 0x2B,
        0x2B => 0x00,
        0x01 => 0x68,
        0x68 => 0x01,
        0x48 => 0x77,
        0x77 => 0x48,
        0x60 => 0xFF,
        0xFF => 0x60,
        0x6C => 0x80,
        0x80 => 0x6C,
        0xB9 => 0xC0,
        0xC0 => 0xB9,
        0xEB => 0xFE,
        0xFE => 0xEB,
        x => x,
    };
}

pub fn decode_des_block(mut block: u64) -> u64 {
    block = des::ip(block);
    block = des::round(block, 0);
    // Gravity accidentally swapped the sides.
    block = block.rotate_left(32);
    des::fp(block)
}

// Hard copy of the rust `des` crate, written by the RustCrypto team.
// We need access to private functions, so we needed to copy them.
mod des {
    // Licensed under the Apache License, Version 2.0 ( the "License" );
    // you may not use this file except in compliance with the License.
    // You may obtain a copy of the License at
    //
    // http://www.apache.org/licenses/LICENSE-2.0
    //
    // Unless required by applicable law or agreed to in writing, software
    // distributed under the License is distributed on an "AS IS" BASIS,
    // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    // See the License for the specific language governing permissions and
    // limitations under the License.

    const S_BOXES: [[u8; 64]; 8] = [
        [
            0x0E, 0x00, 0x04, 0x0F, 0x0D, 0x07, 0x01, 0x04, 0x02, 0x0E, 0x0F, 0x02, 0x0B, 0x0D, 0x08, 0x01, 0x03, 0x0A, 0x0A, 0x06, 0x06,
            0x0C, 0x0C, 0x0B, 0x05, 0x09, 0x09, 0x05, 0x00, 0x03, 0x07, 0x08, 0x04, 0x0F, 0x01, 0x0C, 0x0E, 0x08, 0x08, 0x02, 0x0D, 0x04,
            0x06, 0x09, 0x02, 0x01, 0x0B, 0x07, 0x0F, 0x05, 0x0C, 0x0B, 0x09, 0x03, 0x07, 0x0E, 0x03, 0x0A, 0x0A, 0x00, 0x05, 0x06, 0x00,
            0x0D,
        ],
        [
            0x0F, 0x03, 0x01, 0x0D, 0x08, 0x04, 0x0E, 0x07, 0x06, 0x0F, 0x0B, 0x02, 0x03, 0x08, 0x04, 0x0E, 0x09, 0x0C, 0x07, 0x00, 0x02,
            0x01, 0x0D, 0x0A, 0x0C, 0x06, 0x00, 0x09, 0x05, 0x0B, 0x0A, 0x05, 0x00, 0x0D, 0x0E, 0x08, 0x07, 0x0A, 0x0B, 0x01, 0x0A, 0x03,
            0x04, 0x0F, 0x0D, 0x04, 0x01, 0x02, 0x05, 0x0B, 0x08, 0x06, 0x0C, 0x07, 0x06, 0x0C, 0x09, 0x00, 0x03, 0x05, 0x02, 0x0E, 0x0F,
            0x09,
        ],
        [
            0x0A, 0x0D, 0x00, 0x07, 0x09, 0x00, 0x0E, 0x09, 0x06, 0x03, 0x03, 0x04, 0x0F, 0x06, 0x05, 0x0A, 0x01, 0x02, 0x0D, 0x08, 0x0C,
            0x05, 0x07, 0x0E, 0x0B, 0x0C, 0x04, 0x0B, 0x02, 0x0F, 0x08, 0x01, 0x0D, 0x01, 0x06, 0x0A, 0x04, 0x0D, 0x09, 0x00, 0x08, 0x06,
            0x0F, 0x09, 0x03, 0x08, 0x00, 0x07, 0x0B, 0x04, 0x01, 0x0F, 0x02, 0x0E, 0x0C, 0x03, 0x05, 0x0B, 0x0A, 0x05, 0x0E, 0x02, 0x07,
            0x0C,
        ],
        [
            0x07, 0x0D, 0x0D, 0x08, 0x0E, 0x0B, 0x03, 0x05, 0x00, 0x06, 0x06, 0x0F, 0x09, 0x00, 0x0A, 0x03, 0x01, 0x04, 0x02, 0x07, 0x08,
            0x02, 0x05, 0x0C, 0x0B, 0x01, 0x0C, 0x0A, 0x04, 0x0E, 0x0F, 0x09, 0x0A, 0x03, 0x06, 0x0F, 0x09, 0x00, 0x00, 0x06, 0x0C, 0x0A,
            0x0B, 0x01, 0x07, 0x0D, 0x0D, 0x08, 0x0F, 0x09, 0x01, 0x04, 0x03, 0x05, 0x0E, 0x0B, 0x05, 0x0C, 0x02, 0x07, 0x08, 0x02, 0x04,
            0x0E,
        ],
        [
            0x02, 0x0E, 0x0C, 0x0B, 0x04, 0x02, 0x01, 0x0C, 0x07, 0x04, 0x0A, 0x07, 0x0B, 0x0D, 0x06, 0x01, 0x08, 0x05, 0x05, 0x00, 0x03,
            0x0F, 0x0F, 0x0A, 0x0D, 0x03, 0x00, 0x09, 0x0E, 0x08, 0x09, 0x06, 0x04, 0x0B, 0x02, 0x08, 0x01, 0x0C, 0x0B, 0x07, 0x0A, 0x01,
            0x0D, 0x0E, 0x07, 0x02, 0x08, 0x0D, 0x0F, 0x06, 0x09, 0x0F, 0x0C, 0x00, 0x05, 0x09, 0x06, 0x0A, 0x03, 0x04, 0x00, 0x05, 0x0E,
            0x03,
        ],
        [
            0x0C, 0x0A, 0x01, 0x0F, 0x0A, 0x04, 0x0F, 0x02, 0x09, 0x07, 0x02, 0x0C, 0x06, 0x09, 0x08, 0x05, 0x00, 0x06, 0x0D, 0x01, 0x03,
            0x0D, 0x04, 0x0E, 0x0E, 0x00, 0x07, 0x0B, 0x05, 0x03, 0x0B, 0x08, 0x09, 0x04, 0x0E, 0x03, 0x0F, 0x02, 0x05, 0x0C, 0x02, 0x09,
            0x08, 0x05, 0x0C, 0x0F, 0x03, 0x0A, 0x07, 0x0B, 0x00, 0x0E, 0x04, 0x01, 0x0A, 0x07, 0x01, 0x06, 0x0D, 0x00, 0x0B, 0x08, 0x06,
            0x0D,
        ],
        [
            0x04, 0x0D, 0x0B, 0x00, 0x02, 0x0B, 0x0E, 0x07, 0x0F, 0x04, 0x00, 0x09, 0x08, 0x01, 0x0D, 0x0A, 0x03, 0x0E, 0x0C, 0x03, 0x09,
            0x05, 0x07, 0x0C, 0x05, 0x02, 0x0A, 0x0F, 0x06, 0x08, 0x01, 0x06, 0x01, 0x06, 0x04, 0x0B, 0x0B, 0x0D, 0x0D, 0x08, 0x0C, 0x01,
            0x03, 0x04, 0x07, 0x0A, 0x0E, 0x07, 0x0A, 0x09, 0x0F, 0x05, 0x06, 0x00, 0x08, 0x0F, 0x00, 0x0E, 0x05, 0x02, 0x09, 0x03, 0x02,
            0x0C,
        ],
        [
            0x0D, 0x01, 0x02, 0x0F, 0x08, 0x0D, 0x04, 0x08, 0x06, 0x0A, 0x0F, 0x03, 0x0B, 0x07, 0x01, 0x04, 0x0A, 0x0C, 0x09, 0x05, 0x03,
            0x06, 0x0E, 0x0B, 0x05, 0x00, 0x00, 0x0E, 0x0C, 0x09, 0x07, 0x02, 0x07, 0x02, 0x0B, 0x01, 0x04, 0x0E, 0x01, 0x07, 0x09, 0x04,
            0x0C, 0x0A, 0x0E, 0x08, 0x02, 0x0D, 0x00, 0x0F, 0x06, 0x0C, 0x0A, 0x09, 0x0D, 0x00, 0x0F, 0x03, 0x03, 0x05, 0x05, 0x06, 0x08,
            0x0B,
        ],
    ];

    pub(super) fn round(input: u64, key: u64) -> u64 {
        let left = input & (0xFFFF_FFFF << 32);
        let right = input << 32;
        right | ((feistel(right, key) ^ left) >> 32)
    }

    fn feistel(right: u64, key: u64) -> u64 {
        let expanded = expand(right);
        let xored = expanded ^ key;
        let substituted = sbox_substitute(xored);
        pbox_permute(substituted)
    }

    fn sbox_substitute(input: u64) -> u64 {
        let mut output: u64 = 0;

        for (index, sbox) in S_BOXES.iter().enumerate() {
            let val = (input >> (58 - (index * 6))) & 0x3F;
            output |= u64::from(sbox[val as usize]) << (60 - (index * 4));
        }

        output
    }

    fn delta_swap(a: u64, delta: u64, mask: u64) -> u64 {
        let b = (a ^ (a >> delta)) & mask;
        a ^ b ^ (b << delta)
    }

    pub(super) fn fp(mut message: u64) -> u64 {
        message = delta_swap(message, 24, 0x000000FF000000FF);
        message = delta_swap(message, 24, 0x00000000FF00FF00);
        message = delta_swap(message, 36, 0x000000000F0F0F0F);
        message = delta_swap(message, 18, 0x0000333300003333);
        delta_swap(message, 9, 0x0055005500550055)
    }

    pub(super) fn ip(mut message: u64) -> u64 {
        message = delta_swap(message, 9, 0x0055005500550055);
        message = delta_swap(message, 18, 0x0000333300003333);
        message = delta_swap(message, 36, 0x000000000F0F0F0F);
        message = delta_swap(message, 24, 0x00000000FF00FF00);
        delta_swap(message, 24, 0x000000FF000000FF)
    }

    fn expand(block: u64) -> u64 {
        const BLOCK_LEN: usize = 32;
        const RESULT_LEN: usize = 48;

        let b1 = (block << (BLOCK_LEN - 1)) & 0x8000000000000000;
        let b2 = (block >> 1) & 0x7C00000000000000;
        let b3 = (block >> 3) & 0x03F0000000000000;
        let b4 = (block >> 5) & 0x000FC00000000000;
        let b5 = (block >> 7) & 0x00003F0000000000;
        let b6 = (block >> 9) & 0x000000FC00000000;
        let b7 = (block >> 11) & 0x00000003F0000000;
        let b8 = (block >> 13) & 0x000000000FC00000;
        let b9 = (block >> 15) & 0x00000000003E0000;
        let b10 = (block >> (RESULT_LEN - 1)) & 0x0000000000010000;
        b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8 | b9 | b10
    }

    fn pbox_permute(block: u64) -> u64 {
        let block = block.rotate_left(44);
        let b1 = (block & 0x0000000000200000) << 32;
        let b2 = (block & 0x0000000000480000) << 13;
        let b3 = (block & 0x0000088000000000) << 12;
        let b4 = (block & 0x0000002020120000) << 25;
        let b5 = (block & 0x0000000442000000) << 14;
        let b6 = (block & 0x0000000001800000) << 37;
        let b7 = (block & 0x0000000004000000) << 24;
        let b8 = (block & 0x0000020280015000).wrapping_mul(0x0000020080800083) & 0x02000A6400000000;
        let b9 = (block.rotate_left(29) & 0x01001400000000AA).wrapping_mul(0x0000210210008081) & 0x0902C01200000000;
        let b10 = (block & 0x0000000910040000).wrapping_mul(0x0000000C04000020) & 0x8410010000000000;
        b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8 | b9 | b10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_des_block() {
        let input: u64 = u64::MAX - 123456789;
        let expected: u64 = 12316197016309868543;

        let result = decode_des_block(input);

        assert_eq!(
            result, expected,
            "DES decoding failed! Expected: {:016X}, got: {:016X}",
            expected, result
        );
    }
}
