//! Generates the encrypted password and nonce at compile time. Both will be
//! stored in the resulting binary. The application identifier
//! (`BUILD_ARG_APP_ID`) and the machine identifier (system's machine-id) are
//! used to generate the symmetric encryption key.
//!
//! At runtime, the encryption (decryption) key is generated again from the
//! application identifier and the machine identifier. Therefore, the encrypted
//! password can only be decrypted on the same machine where it was generated
//! (where the crate was compiled).

use encryption::encrypt;

fn main() {
    let plaintext = env!("BUILD_ARG_PASSWORD").as_bytes();
    let (nonce, ciphertext) = encrypt(plaintext).expect("compile-time password encryption failed");
    let nonce_hex = hex::encode(nonce);
    let ciphertext_hex = hex::encode(ciphertext);
    println!("cargo:rerun-if-env-changed=BUILD_ARG_PASSWORD");
    println!("cargo:rustc-env=BUILD_ARG_NONCE_HEX={}", nonce_hex);
    println!(
        "cargo:rustc-env=BUILD_ARG_CIPHERTEXT_HEX={}",
        ciphertext_hex
    );
}
