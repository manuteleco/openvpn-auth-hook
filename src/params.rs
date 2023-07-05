//! Access to parameters provided at run-time (environment variables) and
//! compile-time (stored in the binary in encrypted form).

use encryption::decrypt;
use once_cell::sync::Lazy;
use std::{env, error::Error, ffi::CString};

/// Path for the file that contains the VPN connection username and password
/// (one per line).
///
/// It needs to be specified here in exactly the same way as it is specified in
/// the `auth-user-pass` OpenVPN configuration file (or `--auth-user-pass`
/// command line argument). E.g., if it is specified as a relative path there,
/// it should be specified as the same relative path here. We make a simple
/// string comparison to identify that the auth file is being opened.
pub static AUTH_FILE_PATH: Lazy<Option<String>> = Lazy::new(|| match env::var("AUTH_FILE_PATH") {
    Ok(path) => Some(path),
    Err(env::VarError::NotPresent) => {
        eprintln!("[Hook] ERROR: The environment variable AUTH_FILE_PATH is not set");
        None
    }
    Err(env::VarError::NotUnicode(_)) => {
        eprintln!(
            "[Hook] ERROR: The environment variable AUTH_FILE_PATH is not a valid UTF-8 string"
        );
        None
    }
});

/// OpenVPN connection password. It will be injected when OpenVPN reads the auth
/// file, making it believe the password was actually written in the second line
/// of the file.
///
/// It is stored in the binary in obfuscated form.
pub fn password_line() -> Result<CString, Box<dyn Error>> {
    let nonce = hex::decode(NONCE_HEX)?;
    let nonce = nonce
        .try_into()
        .map_err(|v| format!("Invalid nonce. Must be 12 bytes long. Was: {v:?}"))?;
    let ciphertext = hex::decode(CIPHERTEXT_HEX)?;

    let password = decrypt(&nonce, &ciphertext)?;
    let password = String::from_utf8(password)?;

    Ok(CString::new(format!("{}\n", password).as_bytes())?)
}

/// The nonce used to encrypt the password. It is provided at compilation time
/// and stored in the binary in plain text, as it is needed to decrypt the
/// password as runtime, and it is not considered a secret.
const NONCE_HEX: &str = env!("BUILD_ARG_NONCE_HEX");

/// The encrypted password. The ciphertext is generated at compile time and
/// stored in the binary, so that it can be decrypted at runtime, using the
/// nonce and the encryption key, which is generated from the machine ID at
/// runtime.
const CIPHERTEXT_HEX: &str = env!("BUILD_ARG_CIPHERTEXT_HEX");
