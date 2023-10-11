#![no_std]
#![deny(unused_crate_dependencies)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use bitvec::prelude::{BitSlice, BitVec, Msb0};
use sha2::{Digest, Sha256};

pub mod error;

#[cfg(feature = "sufficient-memory")]
pub mod regular;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "sufficient-memory", test))]
pub mod wordlist;

use crate::error::ErrorWordList;

pub const TOTAL_WORDS: usize = 2048;
pub const WORD_MAX_LEN: usize = 8;
pub const SEPARATOR_LEN: usize = 1;

#[derive(Clone, Copy, Debug)]
pub struct Bits11(u16);

impl Bits11 {
    pub fn bits(self) -> u16 {
        self.0
    }
    pub fn from(i: u16) -> Result<Self, ErrorWordList> {
        if (i as usize) < TOTAL_WORDS {Ok(Self(i))}
        else {Err(ErrorWordList::InvalidWordNumber)}
    }
}

pub struct WordListElement<L: AsWordList + ?Sized> {
    pub word: L::Word,
    pub bits11: Bits11,
}

pub trait AsWordList {
    type Word: AsRef<str>;
    fn get_word(&self, bits: Bits11) -> Result<Self::Word, ErrorWordList>;
    fn get_words_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<WordListElement<Self>>, ErrorWordList>;
    fn bits11_for_word(&self, word: Self::Word) -> Result<Bits11, ErrorWordList>;
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
    fn from(len: usize) -> Result<Self, ErrorWordList> {
        match len {
            12 => Ok(Self::Words12),
            15 => Ok(Self::Words15),
            18 => Ok(Self::Words18),
            21 => Ok(Self::Words21),
            24 => Ok(Self::Words24),
            _ => Err(ErrorWordList::WordsNumber),
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

pub struct WordSet {
    pub bits11_set: Vec<Bits11>,
}

impl WordSet {
    pub fn from_entropy(entropy: &[u8]) -> Result<Self, ErrorWordList> {
        if entropy.len() < 16 || entropy.len() > 32 || entropy.len() % 4 != 0 {
            return Err(ErrorWordList::InvalidEntropy);
        }

        let checksum_byte = sha256_first_byte(entropy);
        let mut entropy_bits: BitVec<u8, Msb0> = BitVec::with_capacity((entropy.len() + 1) * 8);
        entropy_bits.extend_from_bitslice(&BitVec::<u8, Msb0>::from_slice(entropy));
        entropy_bits.extend_from_bitslice(&BitVec::<u8, Msb0>::from_element(checksum_byte));

        let mut bits11_set: Vec<Bits11> = Vec::new();
        for chunk in entropy_bits.chunks_exact(11usize) {
            let mut bits11: u16 = 0;
            for (i, bit) in chunk.into_iter().enumerate() {
                if *bit {
                    bits11 |= 1 << (10 - i)
                }
            }
            bits11_set.push(Bits11(bits11));
        }
        Ok(Self { bits11_set })
    }

    pub fn new() -> Self {
        Self {
            bits11_set: Vec::new(),
        }
    }

    pub fn add_word<L: AsWordList>(
        &mut self,
        word: L::Word,
        wordlist: &L,
    ) -> Result<(), ErrorWordList> {
        let bits11 = wordlist.bits11_for_word(word)?;
        self.bits11_set.push(bits11);
        Ok(())
    }

    pub fn is_finalizable(&self) -> bool {
        MnemonicType::from(self.bits11_set.len()).is_ok()
    }

    pub fn to_entropy(&self) -> Result<Vec<u8>, ErrorWordList> {
        let mnemonic_type = MnemonicType::from(self.bits11_set.len())?;

        let mut entropy_bits: BitVec<u8, Msb0> = BitVec::with_capacity(mnemonic_type.total_bits());

        for bits11 in self.bits11_set.iter() {
            entropy_bits.extend_from_bitslice(
                &BitSlice::<u8, Msb0>::from_slice(&bits11.bits().to_be_bytes())[5..16],
            )
        }

        let mut entropy = entropy_bits.into_vec();
        let entropy_len = mnemonic_type.entropy_bits() / 8;

        let actual_checksum = checksum(entropy[entropy_len], mnemonic_type.checksum_bits());

        entropy.truncate(entropy_len);

        let checksum_byte = sha256_first_byte(&entropy);

        let expected_checksum = checksum(checksum_byte, mnemonic_type.checksum_bits());

        if actual_checksum != expected_checksum {
            Err(ErrorWordList::InvalidChecksum)
        } else {
            Ok(entropy)
        }
    }

    pub fn to_phrase<L: AsWordList>(&self, wordlist: &L) -> Result<String, ErrorWordList> {
        let mut phrase =
            String::with_capacity(self.bits11_set.len() * (WORD_MAX_LEN + SEPARATOR_LEN) - 1);
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
    assert!(bits <= 8);
    source >> (8 - bits)
}

fn sha256_first_byte(input: &[u8]) -> u8 {
    Sha256::digest(input)[0]
}
