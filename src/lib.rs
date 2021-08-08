//! Generate easy to remember sentences that acts as human readable UUIDs.
//!
//! - Built on UUID v4
//! - Optionally pass your UUID to derive a sentence from it
//! - Grammatically _correct_ sentences
//! - Easy to remember (or at least part of it)
//! - Size choice (32-bit token or 128-bit token using `short()` or `generate()` respectively)
//!
//! ## Security
//! This project does not mean to be crypto safe ! **Don't use this as a secure random generator**.
//!
//! Even if we derive sentences from UUID (that are crypto safe), there can still be some collision
//! with 2 differents UUID but resulting in the same sentence.
//!
//! - `25^12` possible combinations for `generate()` (uses 128-bit Token)
//! - `25^5` possible combinations for `short()` (uses 32-bit Token)
//!
//! Note that the sentence generated by `generate()` and the original UUID form a bijection, hence no loss of entropy.
//!
//! ## Example
//! ```
//! use uuid::Uuid;
//! use uuid_readable_rs::{generate_from, short_from, generate, short, generate_inverse};
//!
//! // You can define your own UUID and pass it to uuid_readable_rs like so
//! let uuid = Uuid::new_v4();
//! let sentence_128: String = generate_from(uuid);
//! let sentence_32: String = short_from(uuid);
//!
//! // You can also get an UUID from a sentence that was previously generated
//! let original_uuid: Uuid = generate_inverse(sentence_128).unwrap();
//! assert_eq!(uuid, original_uuid);
//!
//! // Or let uuid_readable_rs handle the Uuid generation
//! let sentence_128: String = generate();
//! let sentence_32: String = short();
//! ```

#[macro_use]
extern crate anyhow;

use anyhow::{Context, Result};
use data::{
    adjectives::ADJECTIVES, animals::ANIMALS, names::NAMES, personal_nouns::PERSONAL_NOUNS,
    places::PLACES, verbs::VERBS,
};
use uuid::Uuid;

mod data;

// TODO - Add a reverse method for sentence -> uuid

/// Mask used for the long version, this allow us to convert a 16 items
/// totalling 128 bit into 12 items for the same number of bits.
/// - 12 => 2**12 = 4096    ==> NAMES
/// - 11 => 2**11 = 2048    ==> NAMES
/// - 14 => 2**14 = 16384   ==> NAMES
/// - 13 => 2**13 = 8192    ==> PERSONAL_NOUNS
/// - 13 => 2**13 = 8192    ==> PLACES
/// - 10 => 2**10 = 1024    ==> VERBS
/// - 12 => 2**12 = 4096    ==> NAMES
/// - 11 => 2**11 = 2048    ==> NAMES
/// - 14 => 2**14 = 16384   ==> NAMES
/// - 5  => 2**5  = 32      ==> MAX 32 as u8
/// - 6  => 2**6  = 64      ==> ADJECTIVES
/// - 7  => 2**7  = 128     ==> ANIMALS
const NORMAL: [u8; 12] = [12, 11, 14, 13, 13, 10, 12, 11, 14, 5, 6, 7];

/// Used for low entropy in the short methods. Higher chances of collisions
/// between two generated sentences. 32 bit into 5 items.
/// - 6 => 2**6 = 64        ==> NAMES
/// - 6 => 2**6 = 64        ==> VERBS
/// - 7 => 2**7 = 128       ==> MAX 128 as u8
/// - 8 => 2**8 = 256       ==> ADJECTIVES
/// - 5 => 2**5 = 32        ==> ANIMALS
const SHORT: [u8; 5] = [6, 6, 7, 8, 5];

/// Convert an array of bytes to a Vec of individuals bits (1-0)
fn to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits: Vec<u8> = Vec::with_capacity(128);

    for b in bytes {
        bits.extend(u16_to_bits(*b as u16, 8));
    }

    bits
}

/// Convert an array of bytes to a Vec of individuals bits (1-0)
fn to_bits_parted(bytes: &[u16]) -> Vec<u8> {
    let mut bits: Vec<u8> = Vec::with_capacity(128);

    for (i, b) in bytes.iter().enumerate() {
        bits.extend(u16_to_bits(*b, NORMAL[i]));
    }

    bits
}

/// Helper used to convert a single digit (u16) into a Vec of indiviuals bits (1-0)
#[inline]
fn u16_to_bits(mut b: u16, lenght: u8) -> Vec<u8> {
    let mut bits = Vec::with_capacity(lenght as usize);

    for _ in 0..lenght {
        bits.push((b % 2) as u8);
        b >>= 1;
    }
    bits.reverse();

    bits
}

/// Convert an array of individuals bits to a byte
fn to_byte(bits: &[u8]) -> u16 {
    let mut _byte = 0u16;

    for b in bits {
        _byte = 2 * _byte + *b as u16;
    }
    _byte
}

/// Convert bytes to bits and group them into 12 distinct numbers
fn partition(parts: &[u8], bytes: &[u8]) -> [usize; 12] {
    let mut bits: Vec<u8> = to_bits(bytes);

    let mut _bytes: [usize; 12] = [0; 12];
    for (idx, p) in parts.iter().enumerate() {
        let tmp = bits.drain(0..(*p as usize));
        _bytes[idx] = to_byte(tmp.as_slice()) as usize;
    }

    _bytes
}

/// Convert bits to bytes, grouping them 8 by 8 because it's u8
fn de_partition(bits: &[u8]) -> [u8; 16] {
    let mut bytes = [0; 16];

    for i in 0..16 {
        bytes[i] = to_byte(&bits[8 * i..8 * (i + 1)]) as u8;
    }

    bytes
}

#[inline]
fn _generate(uuid: &Uuid) -> String {
    // Convert the Uuid to an array of bytes
    let uuid = uuid.as_bytes();
    // Get the partition (it's basically random numbers (12) from the uuid)
    let words = partition(&NORMAL, uuid);

    // Generate the sentence and return it
    format!(
        "{} {} {} the {} of {} {} {} {} {} and {} {} {}",
        NAMES[words[0]],
        NAMES[words[1]],
        NAMES[words[2]],
        PERSONAL_NOUNS[words[3]],
        PLACES[words[4]],
        VERBS[words[5]],
        NAMES[words[6]],
        NAMES[words[7]],
        NAMES[words[8]],
        words[9],
        ADJECTIVES[words[10]],
        ANIMALS[words[11]]
    )
}

/// Create a long sentence using a new random UUID.
///
/// Example of return: `Joy Bolt Kahler the avenger of Esbon jumped Carey Fatma Sander and 8 large ducks`
pub fn generate() -> String {
    // Generate a new Uuid using the v4 RFC
    let uuid = Uuid::new_v4();

    // Create the sentence from the Uuid
    _generate(&uuid)
}

/// Derive a long sentence from a UUID.
///
/// Example of return: `Joy Bolt Kahler the avenger of Esbon jumped Carey Fatma Sander and 8 large ducks`
pub fn generate_from(uuid: Uuid) -> String {
    // Create the sentence from the Uuid
    _generate(&uuid)
}

/// Get the original uuid from a sentence.
///
/// Example of return: `0ee001c7-12f3-4b29-a4cc-f48838b3587a`
pub fn generate_inverse<S: AsRef<str>>(sentence: S) -> Result<Uuid> {
    // Split the sentence
    let splitted: Vec<&str> = sentence.as_ref().split(' ').collect();
    // Sanity check that we have enough values to work with
    if splitted.len() < 15 {
        return Err(anyhow!(
            "The sentence does not correspond to a one from uuid-readable-rs."
        ));
    }
    // Collect the index of each parts
    let index_values = [
        NAMES
            .iter()
            .position(|&r| r == splitted[0])
            .context("NAMES (0) not found")? as u16,
        NAMES
            .iter()
            .position(|&r| r == splitted[1])
            .context("NAMES (1) not found")? as u16,
        NAMES
            .iter()
            .position(|&r| r == splitted[2])
            .context("NAMES (2) not found")? as u16,
        PERSONAL_NOUNS
            .iter()
            .position(|&r| r == splitted[4])
            .context("PERSONAL_NOUNS (4) not found")? as u16,
        PLACES
            .iter()
            .position(|&r| r == splitted[6])
            .context("PLACES (6) not found")? as u16,
        VERBS
            .iter()
            .position(|&r| r == splitted[7])
            .context("VERBS (7) not found")? as u16,
        NAMES
            .iter()
            .position(|&r| r == splitted[8])
            .context("NAMES (8) not found")? as u16,
        NAMES
            .iter()
            .position(|&r| r == splitted[9])
            .context("NAMES (9) not found")? as u16,
        NAMES
            .iter()
            .position(|&r| r == splitted[10])
            .context("NAMES (10) not found")? as u16,
        splitted[12].parse::<u16>()?,
        ADJECTIVES
            .iter()
            .position(|&r| r == splitted[13])
            .context("ADJECTIVES (13) not found")? as u16,
        ANIMALS
            .iter()
            .position(|&r| r == splitted[14])
            .context("ANIMALS (14) not found")? as u16,
    ];
    // Convert the index into bits
    let bits = to_bits_parted(&index_values);
    // Convert the bits to bytes
    let bytes = de_partition(&bits);

    // Convert the bytes into the Uuid
    Ok(Uuid::from_slice(&bytes)?)
}

#[inline]
fn _short(uuid: &Uuid) -> String {
    // Convert the Uuid to an array of bytes
    let uuid = uuid.as_bytes();
    // Get the partition (it's basically random numbers (12) from the uuid)
    let words = partition(&SHORT, uuid);

    // Generate the sentence and return it
    format!(
        "{} {} by {} {} {}",
        NAMES[words[0]], VERBS[words[1]], words[2], ADJECTIVES[words[3]], ANIMALS[words[4]],
    )
}

/// Create a short sentence using a new random UUID.
///
/// Example of return: `Alex sang by 60 narrow chickens`
pub fn short() -> String {
    // Generate a new Uuid using the v4 RFC
    let uuid = Uuid::new_v4();

    // Create the sentence from the Uuid
    _short(&uuid)
}

/// Derive a short sentence from a UUID.
///
/// Example of return: `Alex sang by 60 narrow chickens`
pub fn short_from(uuid: Uuid) -> String {
    // Create the sentence from the Uuid
    _short(&uuid)
}

#[test]
fn sanity() {
    let uuid = Uuid::parse_str("0ee001c7-12f3-4b29-a4cc-f48838b3587a").unwrap();

    let g = generate_from(uuid);
    assert_eq!(
        g,
        "Serena Brie Rehnberg the loki of Manteo slowed Mariya Megargee Marigolda and 26 enthusiastically toads"
    );

    let i = generate_inverse(&g).unwrap();
    assert_eq!(i, uuid);

    let s = short_from(uuid);
    assert_eq!(s, "Josselyn interrupted by 0 fiercely mice");
}

#[test]
fn test_bits_convertion() {
    let arr = [41];
    let bits = to_bits(&arr);
    assert_eq!(bits, vec![0, 0, 1, 0, 1, 0, 0, 1]);

    let byte = to_byte(&bits);
    assert_eq!(byte, 41);
}
