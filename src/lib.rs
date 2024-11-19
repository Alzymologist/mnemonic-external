#![no_std]
#![deny(unused_crate_dependencies)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod error;

#[cfg(feature = "sufficient-memory")]
pub mod regular;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "sufficient-memory", test))]
pub mod wordlist;

use crate::error::ErrorMnemonic;

pub const TOTAL_WORDS: usize = 2048;
pub const WORD_MAX_LEN: usize = 8;
pub const SEPARATOR_LEN: usize = 1;

pub const MAX_SEED_LEN: usize = 24;

#[derive(Clone, Copy, Debug, Zeroize)]
pub struct Bits11(u16);

impl Bits11 {
    pub fn bits(self) -> u16 {
        self.0
    }
    pub fn from(i: u16) -> Result<Self, ErrorMnemonic> {
        if (i as usize) < TOTAL_WORDS {
            Ok(Self(i))
        } else {
            Err(ErrorMnemonic::InvalidWordNumber)
        }
    }
}

#[derive(Clone, Debug)]
pub struct WordListElement<L: AsWordList + ?Sized> {
    pub word: L::Word,
    pub bits11: Bits11,
}

pub trait AsWordList {
    type Word: AsRef<str>;
    fn get_word(&self, bits: Bits11) -> Result<Self::Word, ErrorMnemonic>;
    fn get_words_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<WordListElement<Self>>, ErrorMnemonic>;
    fn bits11_for_word(&self, word: &str) -> Result<Bits11, ErrorMnemonic>;
}

#[derive(Debug, Copy, Clone)]
pub enum MnemonicType {
    Words12,
    Words15,
    Words18,
    Words21,
    Words24,
}

impl MnemonicType {
    fn from(len: usize) -> Result<Self, ErrorMnemonic> {
        match len {
            12 => Ok(Self::Words12),
            15 => Ok(Self::Words15),
            18 => Ok(Self::Words18),
            21 => Ok(Self::Words21),
            24 => Ok(Self::Words24),
            _ => Err(ErrorMnemonic::WordsNumber),
        }
    }
    fn checksum_bits(&self) -> u8 {
        match &self {
            Self::Words12 => 4,
            Self::Words15 => 5,
            Self::Words18 => 6,
            Self::Words21 => 7,
            Self::Words24 => 8,
        }
    }
    fn entropy_bits(&self) -> usize {
        match &self {
            Self::Words12 => 128,
            Self::Words15 => 160,
            Self::Words18 => 192,
            Self::Words21 => 224,
            Self::Words24 => 256,
        }
    }
    fn total_bits(&self) -> usize {
        self.entropy_bits() + self.checksum_bits() as usize
    }
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
struct BitsHelper {
    bits: Vec<bool>,
}

impl BitsHelper {
    fn with_capacity(cap: usize) -> Self {
        Self {
            bits: Vec::with_capacity(cap),
        }
    }

    fn extend_from_byte(&mut self, byte: u8) {
        for i in 0..BITS_IN_BYTE {
            let bit = (byte & (1 << (BITS_IN_BYTE - 1 - i))) != 0;
            self.bits.push(bit);
        }
    }

    fn extend_from_bits11(&mut self, bits11: &Bits11) {
        let two_bytes = bits11.0.to_be_bytes();

        // last 3 bits of first byte - others are always zero
        for i in (BITS_IN_BYTE - BITS_IN_U11 % BITS_IN_BYTE)..BITS_IN_BYTE {
            let bit = (two_bytes[0] & (1 << (BITS_IN_BYTE - 1 - i))) != 0;
            self.bits.push(bit);
        }

        // all bits of second byte
        self.extend_from_byte(two_bytes[1])
    }
}

pub const BITS_IN_BYTE: usize = 8;
pub const BITS_IN_U11: usize = 11;

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct WordSet {
    pub bits11_set: Vec<Bits11>,
}

impl WordSet {
    pub fn from_entropy(entropy: &[u8]) -> Result<Self, ErrorMnemonic> {
        if entropy.len() < 16 || entropy.len() > 32 || entropy.len() % 4 != 0 {
            return Err(ErrorMnemonic::InvalidEntropy);
        }

        let checksum_byte = sha256_first_byte(entropy);

        let mut entropy_bits = BitsHelper::with_capacity((entropy.len() + 1) * BITS_IN_BYTE);
        for byte in entropy {
            entropy_bits.extend_from_byte(*byte);
        }
        entropy_bits.extend_from_byte(checksum_byte);

        let mut bits11_set: Vec<Bits11> = Vec::with_capacity(MAX_SEED_LEN);
        for chunk in entropy_bits.bits.chunks_exact(BITS_IN_U11) {
            let mut bits11: u16 = 0;
            for (i, bit) in chunk.iter().enumerate() {
                if *bit {
                    bits11 |= 1 << (BITS_IN_U11 - 1 - i)
                }
            }
            bits11_set.push(Bits11(bits11));
        }
        Ok(Self { bits11_set })
    }

    pub fn new() -> Self {
        Self {
            bits11_set: Vec::with_capacity(MAX_SEED_LEN),
        }
    }

    pub fn add_word<L: AsWordList>(
        &mut self,
        word: &str,
        wordlist: &L,
    ) -> Result<(), ErrorMnemonic> {
        if self.bits11_set.len() < MAX_SEED_LEN {
            let bits11 = wordlist.bits11_for_word(word)?;
            self.bits11_set.push(bits11);
        }
        Ok(())
    }

    pub fn is_finalizable(&self) -> bool {
        MnemonicType::from(self.bits11_set.len()).is_ok()
    }

    pub fn to_entropy(&self) -> Result<Vec<u8>, ErrorMnemonic> {
        let mnemonic_type = MnemonicType::from(self.bits11_set.len())?;

        let mut entropy_bits = BitsHelper::with_capacity(mnemonic_type.total_bits());

        for bits11 in self.bits11_set.iter() {
            entropy_bits.extend_from_bits11(bits11);
        }

        let mut entropy: Vec<u8> = Vec::with_capacity(mnemonic_type.total_bits() / BITS_IN_BYTE);

        for chunk in entropy_bits.bits.chunks(BITS_IN_BYTE) {
            let mut byte: u8 = 0;
            for (i, bit) in chunk.iter().enumerate() {
                if *bit {
                    byte |= 1 << (BITS_IN_BYTE - 1 - i)
                }
            }
            entropy.push(byte);
        }

        let entropy_len = mnemonic_type.entropy_bits() / BITS_IN_BYTE;

        let actual_checksum = checksum(entropy[entropy_len], mnemonic_type.checksum_bits());

        entropy.truncate(entropy_len);

        let checksum_byte = sha256_first_byte(&entropy);

        let expected_checksum = checksum(checksum_byte, mnemonic_type.checksum_bits());

        if actual_checksum != expected_checksum {
            Err(ErrorMnemonic::InvalidChecksum)
        } else {
            Ok(entropy)
        }
    }

    pub fn to_phrase<L: AsWordList>(&self, wordlist: &L) -> Result<String, ErrorMnemonic> {
        let mut phrase = String::with_capacity(
            self.bits11_set.len() * (WORD_MAX_LEN + SEPARATOR_LEN) - SEPARATOR_LEN,
        );
        for bits11 in self.bits11_set.iter() {
            if !phrase.is_empty() {
                phrase.push(' ')
            }
            let word = wordlist.get_word(*bits11)?;
            phrase.push_str(word.as_ref());
        }
        Ok(phrase)
    }
}

fn checksum(source: u8, bits: u8) -> u8 {
    assert!(bits <= BITS_IN_BYTE as u8);
    source >> (BITS_IN_BYTE as u8 - bits)
}

fn sha256_first_byte(input: &[u8]) -> u8 {
    Sha256::digest(input)[0]
}
