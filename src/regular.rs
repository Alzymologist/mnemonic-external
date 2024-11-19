#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::vec::Vec;

use crate::error::ErrorMnemonic;
use crate::wordlist::WORDLIST_ENGLISH;
use crate::{AsWordList, Bits11, WordListElement};

pub struct InternalWordList;

impl AsWordList for InternalWordList {
    type Word = &'static str;

    fn get_word(&self, bits: Bits11) -> Result<Self::Word, ErrorMnemonic> {
        let word_order = bits.bits() as usize;
        Ok(WORDLIST_ENGLISH[word_order])
    }

    fn get_words_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<WordListElement<Self>>, ErrorMnemonic> {
        let mut out: Vec<WordListElement<Self>> = Vec::new();
        for (i, word) in WORDLIST_ENGLISH.iter().enumerate() {
            if word.starts_with(prefix) {
                out.push(WordListElement {
                    word,
                    bits11: Bits11::from(i as u16)?,
                })
            }
        }
        Ok(out)
    }

    fn bits11_for_word(&self, word: &str) -> Result<Bits11, ErrorMnemonic> {
        for (i, element) in WORDLIST_ENGLISH.iter().enumerate() {
            if element == &word {
                return Bits11::from(i as u16);
            }
        }
        Err(ErrorMnemonic::NoWord)
    }
}
