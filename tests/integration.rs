use once_cell::sync::OnceCell;
use std::io::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

/// Reading a temporary file that doesn't match the name passed via
/// `AUTH_FILE_PATH` environment variable causes the file to be read as usual.
/// The password is not replaced.
#[test]
fn test_auth_file_path_not_matching() {
    setup();
    let output = run(
        STANDARD_FILE_CONTENTS,
        MIN_BUFFER_SIZE,
        AuthFilePath::DoesNotMatch,
    );
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, STANDARD_FILE_CONTENTS);
}

/// Reading a temporary file that matches the path passed via `AUTH_FILE_PATH`
/// environment variable and contains username and password with line lengths
/// shorter than the `fgets` buffer causes the password to be replaced. This is
/// the most common case and the one we expect during actual usage with OpenVPN.
#[test]
fn test_auth_file_path_matching() {
    setup();
    let output = run(
        STANDARD_FILE_CONTENTS,
        MIN_BUFFER_SIZE,
        AuthFilePath::Matches,
    );
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, format!("username\n{PASSWORD}\n"));
}

/// If the password length (with extra new line and null character) is longer
/// than the buffer size, don't replace the password.
#[test]
fn test_password_too_long() {
    setup();
    let output = run(
        STANDARD_FILE_CONTENTS,
        MIN_BUFFER_SIZE - 1,
        AuthFilePath::Matches,
    );
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stderr,
        "[Hook] WARNING: Replacement line is too long to fit in the buffer (12 > 11)\n"
    );
    assert_eq!(output.stdout, STANDARD_FILE_CONTENTS);
}

/// If the `auth-user-pass` file contains additional lines, just print them
/// normally after the password replacement. Note that this is not a common case
/// and OpenVPN will just ignore additional lines, but it's good to have it
/// covered.
#[test]
fn test_auth_file_with_extra_lines() {
    setup();
    let output = run(
        &(STANDARD_FILE_CONTENTS.to_owned() + "extra line\n"),
        MIN_BUFFER_SIZE,
        AuthFilePath::Matches,
    );
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, format!("username\n{PASSWORD}\nextra line\n"));
}

/// If the `auth-user-pass` file doesn't end in newline character, it should
/// still work (i.e., the password should be replaced). Note that the
/// replacement password will still include an ending newline of its own.
#[test]
fn test_auth_file_not_ending_in_newline() {
    setup();
    let output = run("username\npassword", MIN_BUFFER_SIZE, AuthFilePath::Matches);
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, format!("username\n{PASSWORD}\n"));
}

//
// HELPERS
//

/// This is the replacement password that is embedded in the binary at compile
/// time.
const PASSWORD: &str = env!("BUILD_ARG_PASSWORD");

/// Minimum size of the buffer used by `fgets` so that the a password like
/// containinig `PASSWORD` can fit in it.
const MIN_BUFFER_SIZE: usize = PASSWORD.len() + 2; // +2 for the new line character and the null character

/// Typical contents of the `auth-user-pass` file, without any special cases.
const STANDARD_FILE_CONTENTS: &str = "username\npassword\n";

fn setup() {
    static CELL: OnceCell<()> = OnceCell::new();
    CELL.get_or_init(|| {
        Command::new("gcc")
            .args(&["tests/test_app.c", "-o", "tests/test_app"])
            .status()
            .unwrap();
    });
}

fn create_temporary_file(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();
    temp_file
}

struct Output {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

enum AuthFilePath {
    Matches,
    DoesNotMatch,
}

fn run(file_contents: &str, buffer_size: usize, auth_file: AuthFilePath) -> Output {
    setup();

    let temp_file = create_temporary_file(file_contents);
    let file_path = temp_file.path().to_str().unwrap();
    let auth_file_path = match auth_file {
        AuthFilePath::Matches => file_path,
        AuthFilePath::DoesNotMatch => "does_not_match",
    };

    let output = Command::new("tests/test_app")
        .env("LD_PRELOAD", "target/debug/libopenvpn_auth_hook.so")
        .env("AUTH_FILE_PATH", auth_file_path)
        .args(&[file_path, &buffer_size.to_string()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap();
    Output {
        stdout,
        stderr,
        exit_code,
    }
}
