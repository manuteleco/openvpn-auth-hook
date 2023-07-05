//! Helper crate to encrypt a password at compile time and decrypt it at run
//! time.
//!
//! Internally manages the generation of the encryption key and the nonce value.
//! The encryption key is generated from the application identifier (a constant
//! hardcoded value) and the machine identifier (read from the filesystem).
use std::io;

use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, KeyInit,
};
use hkdf::Hkdf;
use obfstr::obfstr;
use sha2::Sha256;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error obtaining the machine identifier")]
    Io(#[from] io::Error),

    #[error("cipher error (opaque)")]
    Cipher,
}

/// Size of the encryption key generated with
/// [HDKF](https://datatracker.ietf.org/doc/html/rfc5869) and fed into the
/// AES256-GCM cipher.
///
/// It should be at most 255 * HashLength octets for HDKF to be able to generate
/// it (8160 octets for SHA256). But it must be exactly 32 octets for AES256-GCM
/// to be able to use it.
const KEY_SIZE: usize = 32;

/// Size of the nonce value for encryption/decryption. Must be exactly 12 octets
/// for AES256-GCM.
const NONCE_SIZE: usize = 12;

/// Encrypt the given plaintext with a randomly generated nonce.
///
/// Returns both the ciphertext and the nonce, which is required for decryption.
/// The encryption key is internally generated from the application identifier
/// and the machine identifier.
pub fn encrypt(plaintext: &[u8]) -> Result<([u8; NONCE_SIZE], Vec<u8>), Error> {
    let cipher = create_cipher()?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| Error::Cipher)?;
    Ok((nonce.into(), ciphertext))
}

/// Decrypt the ciphertext with the given nonce.
///
/// The decryption key is internally generated from the application identifier
/// and the machine identifier.
pub fn decrypt(nonce: &[u8; NONCE_SIZE], ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = create_cipher()?;
    cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| Error::Cipher)
}

/// Create a AES-GCM cipher with a 256-bit key and 96-bit nonce for symmetric
/// key encryption/decryption. Intialized with a key generated from the
/// application identifier and the machine identifier.
fn create_cipher() -> Result<Aes256Gcm, io::Error> {
    let key = generate_key(&app_id(), machine_id()?.as_bytes());
    Ok(Aes256Gcm::new(&key.into()))
}

/// Generate an encryption key with
/// [HDKF](https://datatracker.ietf.org/doc/html/rfc5869) by combining the
/// application identifier (as Input Key Material) and the machine identifier
/// (as Info).
fn generate_key(app_id: &[u8], machine_id: &[u8]) -> [u8; KEY_SIZE] {
    let ikm = app_id;
    let info = machine_id;
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut okm = [0u8; KEY_SIZE];
    hk.expand(info, &mut okm).expect(
        // Should never panic, as the key must be exactly 32 bytes long for the
        // AES256-GCM cypher, and that requirement is already being enforced by
        // the compiler (changing the value of KEY_SIZE breaks the build).
        "{KEY_SIZE} should be a valid length for SHA256 to output (should be <= 32 * 255 = 8160)",
    );
    okm
}

/// Constant identifier of the application.
fn app_id() -> Vec<u8> {
    hex::decode(obfstr!(env!("BUILD_ARG_APP_ID")))
        .expect("BUILD_ARG_APP_ID should be a valid hex string")
}

/// Possible paths to the machine-id file. Taken from [DBus'
/// documentation](https://dbus.freedesktop.org/doc/dbus-specification.html#standard-interfaces-peer).
const MACHINE_ID_PATHS: [&str; 2] = ["/etc/machine-id", "/var/lib/dbus/machine-id"];

/// Read the machine-id from the filesystem.
fn machine_id() -> Result<String, io::Error> {
    for path in MACHINE_ID_PATHS.iter() {
        if std::path::Path::new(path).exists() {
            return std::fs::read_to_string(path);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "no machine-id found (tried: {})",
            MACHINE_ID_PATHS.join(", ")
        ),
    ))
}
