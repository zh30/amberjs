//! Cryptographic helpers, JWT tokens, and UUID generation (`amber:std/crypto`).

use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Generate standard UUID v4 string
pub fn generate_uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

/// Generate time-ordered UUID v7 string
pub fn generate_uuid_v7() -> String {
    // Generate timestamp-first UUID v7 format
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ms = now.as_millis() as u64;

    let rand_b = fastrand::u64(..);
    let mut bytes = [0u8; 16];

    // Top 48 bits: millisecond timestamp
    bytes[0] = (ms >> 40) as u8;
    bytes[1] = (ms >> 32) as u8;
    bytes[2] = (ms >> 24) as u8;
    bytes[3] = (ms >> 16) as u8;
    bytes[4] = (ms >> 8) as u8;
    bytes[5] = ms as u8;

    // 4 bits version (0b0111 = 7) + 12 bits random
    let rand_12 = (fastrand::u16(..) & 0x0FFF) | 0x7000;
    bytes[6] = (rand_12 >> 8) as u8;
    bytes[7] = rand_12 as u8;

    // 2 bits variant (0b10) + 62 bits random
    bytes[8] = ((rand_b >> 56) as u8 & 0x3F) | 0x80;
    bytes[9] = (rand_b >> 48) as u8;
    bytes[10] = (rand_b >> 40) as u8;
    bytes[11] = (rand_b >> 32) as u8;
    bytes[12] = (rand_b >> 24) as u8;
    bytes[13] = (rand_b >> 16) as u8;
    bytes[14] = (rand_b >> 8) as u8;
    bytes[15] = rand_b as u8;

    Uuid::from_bytes(bytes).to_string()
}
