//! File checksums through Windows' own hashing (CNG), so no extra crates.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use windows::Win32::Security::Cryptography::{
    BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE, BCRYPT_MD5_ALGORITHM, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS, BCRYPT_SHA1_ALGORITHM,
    BCRYPT_SHA256_ALGORITHM, BCRYPT_SHA512_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptCreateHash, BCryptDestroyHash,
    BCryptFinishHash, BCryptHashData, BCryptOpenAlgorithmProvider,
};
use windows::core::PCWSTR;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashKind {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl HashKind {
    pub const ALL: [HashKind; 4] = [HashKind::Md5, HashKind::Sha1, HashKind::Sha256, HashKind::Sha512];

    pub fn label(self) -> &'static str {
        match self {
            HashKind::Md5 => "MD5",
            HashKind::Sha1 => "SHA-1",
            HashKind::Sha256 => "SHA-256",
            HashKind::Sha512 => "SHA-512",
        }
    }

    fn algorithm(self) -> PCWSTR {
        match self {
            HashKind::Md5 => BCRYPT_MD5_ALGORITHM,
            HashKind::Sha1 => BCRYPT_SHA1_ALGORITHM,
            HashKind::Sha256 => BCRYPT_SHA256_ALGORITHM,
            HashKind::Sha512 => BCRYPT_SHA512_ALGORITHM,
        }
    }

    fn len(self) -> usize {
        match self {
            HashKind::Md5 => 16,
            HashKind::Sha1 => 20,
            HashKind::Sha256 => 32,
            HashKind::Sha512 => 64,
        }
    }

    /// Guesses the kind from the length of a pasted hex checksum.
    pub fn from_hex_len(len: usize) -> Option<HashKind> {
        HashKind::ALL.into_iter().find(|k| k.len() * 2 == len)
    }
}

struct Hasher {
    alg: BCRYPT_ALG_HANDLE,
    hash: BCRYPT_HASH_HANDLE,
    kind: HashKind,
}

impl Hasher {
    fn new(kind: HashKind) -> Result<Hasher> {
        let mut alg = BCRYPT_ALG_HANDLE::default();
        let mut hash = BCRYPT_HASH_HANDLE::default();
        unsafe {
            BCryptOpenAlgorithmProvider(&mut alg, kind.algorithm(), PCWSTR::null(), BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0))
                .ok()
                .map_err(|e| anyhow!("could not start {}: {e}", kind.label()))?;
            if let Err(e) = BCryptCreateHash(alg, &mut hash, None, None, 0).ok() {
                let _ = BCryptCloseAlgorithmProvider(alg, 0);
                bail!("could not start {}: {e}", kind.label());
            }
        }
        Ok(Hasher { alg, hash, kind })
    }

    fn update(&mut self, data: &[u8]) -> Result<()> {
        unsafe { BCryptHashData(self.hash, data, 0).ok().map_err(|e| anyhow!("hashing failed: {e}")) }
    }

    fn finish(self) -> Result<String> {
        let mut out = vec![0u8; self.kind.len()];
        unsafe { BCryptFinishHash(self.hash, &mut out, 0).ok().map_err(|e| anyhow!("hashing failed: {e}"))? };
        Ok(out.iter().map(|b| format!("{b:02x}")).collect())
    }
}

impl Drop for Hasher {
    fn drop(&mut self) {
        unsafe {
            let _ = BCryptDestroyHash(self.hash);
            let _ = BCryptCloseAlgorithmProvider(self.alg, 0);
        }
    }
}

pub fn hash_bytes(kind: HashKind, data: &[u8]) -> Result<String> {
    let mut h = Hasher::new(kind)?;
    h.update(data)?;
    h.finish()
}

/// Lowercase hex checksum of a file, read in 1 MB pieces.
pub fn hash_file(kind: HashKind, path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|e| anyhow!("could not open the file: {e}"))?;
    let mut h = Hasher::new(kind)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| anyhow!("could not read the file: {e}"))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n])?;
    }
    h.finish()
}

/// Strips spaces, colons and a leading "sha256:" style label, lowercases.
pub fn normalize(expected: &str) -> String {
    let lower = expected.trim().to_ascii_lowercase();
    let labels = ["sha-512", "sha512", "sha-256", "sha256", "sha-1", "sha1", "md5"];
    let text = labels
        .iter()
        .find_map(|l| lower.strip_prefix(l).map(|rest| rest.trim_start_matches([':', '=', ' '])))
        .unwrap_or(&lower);
    text.chars().filter(|c| c.is_ascii_hexdigit()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        assert_eq!(hash_bytes(HashKind::Md5, b"abc").unwrap(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hash_bytes(HashKind::Sha1, b"abc").unwrap(), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hash_bytes(HashKind::Sha256, b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash_bytes(HashKind::Sha512, b"abc").unwrap().starts_with("ddaf35a193617aba"));
    }

    #[test]
    fn file_matches_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        assert_eq!(hash_file(HashKind::Sha256, &path).unwrap(), hash_bytes(HashKind::Sha256, &data).unwrap());
    }

    #[test]
    fn normalizes_pasted_text() {
        assert_eq!(normalize(" sha256:AB CD "), "abcd");
        assert_eq!(normalize("AB:CD:EF"), "abcdef");
        assert_eq!(normalize("MD5 = 00ff"), "00ff");
        assert_eq!(HashKind::from_hex_len(64), Some(HashKind::Sha256));
        assert_eq!(HashKind::from_hex_len(10), None);
    }
}
