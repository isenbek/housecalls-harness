//! The code sequence: a 12-word BIP-39 phrase, because the crypto-wallet
//! muscle memory is the whole UX. Generated here, validated here, and never
//! stored by anything we run.

use bip39::{Language, Mnemonic};

use crate::{EngineError, Result};

/// Twelve fresh words. Shown to the user once; losing them loses the backup,
/// and there is no back door, which is the point.
pub fn generate() -> String {
    // 12 words = 128 bits of entropy from the OS (getrandom js on wasm).
    let m = Mnemonic::generate_in(Language::English, 12).expect("entropy source");
    m.to_string()
}

/// Normalize + validate a typed phrase (case, stray spaces). Returns the
/// canonical form the rest of the engine derives from.
pub fn normalize(words: &str) -> Result<String> {
    let cleaned = words.trim().to_lowercase();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let m = Mnemonic::parse_in(Language::English, &cleaned)
        .map_err(|e| EngineError::BadSeed(e.to_string()))?;
    Ok(m.to_string())
}

/// The 64-byte BIP-39 seed for a canonical phrase (empty passphrase: the
/// twelve words ARE the credential; a 13th secret word would be a second
/// thing to lose).
pub fn to_seed_bytes(canonical: &str) -> Result<[u8; 64]> {
    let m = Mnemonic::parse_in(Language::English, canonical)
        .map_err(|e| EngineError::BadSeed(e.to_string()))?;
    Ok(m.to_seed(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_twelve_valid_words() {
        let w = generate();
        assert_eq!(w.split_whitespace().count(), 12);
        assert_eq!(normalize(&w).unwrap(), w);
    }

    #[test]
    fn normalize_forgives_case_and_spacing() {
        let w = generate();
        let messy = format!("  {}  ", w.to_uppercase().replace(' ', "   "));
        assert_eq!(normalize(&messy).unwrap(), w);
    }

    #[test]
    fn rejects_junk() {
        assert!(normalize("not a real seed phrase at all").is_err());
    }

    #[test]
    fn same_words_same_seed() {
        let w = generate();
        assert_eq!(to_seed_bytes(&w).unwrap(), to_seed_bytes(&w).unwrap());
    }
}
