# openvpn-auth-hook

[![CICD](https://github.com/manuteleco/openvpn-auth-hook/actions/workflows/rust.yml/badge.svg)](https://github.com/manuteleco/openvpn-auth-hook/actions/workflows/rust.yml)

`LD_PRELOAD` hook for the OpenVPN client to replace the `auth-user-pass`
password on the fly.

## Description

This crate provides an OpenVPN client `LD_PRELOAD` hook that replaces, on the
fly, the user password defined in the configured `auth-user-pass` file.

The hook is designed to be used in combination with the `auth-user-pass` OpenVPN
configuration directive (or `--auth-user-pass` command line argument). This
directive specifies the path to a file that contains credentials for client
authentication. The `auth-user-pass` file is a plain text file that typically
contains two lines: the first line is the username, and the second line is the
password.

When OpenVPN reads the contents of the `auth-user-pass` file, the hook replaces
the password transparently. OpenVPN only ever sees the replacement password. The
actual password stored in the file is never really used when the hook is
enabled, but a password must be there nonetheless. It's value can be considered
a decoy.

It works by intercepting the `fopen`, `fgets` and `fclose` functions, which
happen to be used by OpenVPN to read the `auth-user-pass`. An additional
`AUTH_FILE_PATH` environment variable passed at runtime allows the hook to
identify which file to track and replace the password in, while ignoring other
files.

The replacement password is stored in the dynamic library binary itself, in
encrypted form. This means that the legitimate password must be provided at
build time. It also means that in order to use the hook for two different client
connections, two different binaries must be built (assuming said connections use
different passwords).

The encryption key applied to the password is derived from the
[machine-id][machine-id]. Given that the password must be encrypted at build
time and decrypted at runtime, it follows that the hook must be built on the
same machine where it will be used.

[machine-id]: https://www.freedesktop.org/software/systemd/man/machine-id.html

## Motivation

The `auth-user-pass` file is a plain text file that contains the user password.
Even if it is only readable by the user running the OpenVPN client when
properly configured, it is still a plain text password that is stored in the
filesystem. This is a security risk.

This felt like a good opportunity to learn about `LD_PRELOAD` hooks and dynamic
libraries in Rust and overengineer a solution to a problem that doesn't seem to
matter.

In that spirit, this hook aims to mitigate this risk by replacing the password
on the fly. The password is stored in encrypted form in the dynamic library
binary itself. The encryption key is derived from the machine-id, which means
that the password can only be decrypted on the same machine where the hook was
built.

## Security Disclaimer

Despite using encryption at some point, this hook just provides obfuscation, not
real security. Getting a hold of the binary and the machine-id is enough to
reverse engineer the password. Both factors are stored in the same machine, so
it is not a stretch to assume that an attacker that has access to one of them
will also have access to the other.

Still, this is probably better than storing the password in plain text in the
`auth-user-pass` file.

## Compatibility

This hook has been tested with OpenVPN 2.6.5 on Linux.

It is likely to work with other versions of OpenVPN, as long as they use the
same `auth-user-pass` file format and the same `fopen`, `fgets` and `fclose`
calls to read the file. That's unlikely to change any time soon.

In its current form, it is unlikely to work on other operating systems, as it
uses the Linux-specific `/etc/machine-id`. Maybe it could also work on Mac by
manually writing that file. Unclear.

## Testing

Normally we would only need to run `cargo tests`. However, the tests require
that the dynamic library is available, which is not the case if we haven't built
it yet. This is a chicken and egg problem that has been solved within cargo for
executables (see [`CARGO_BIN_EXE_<name>`][cargo-envs]), but not for dynamic
libraries ([see][issue8193] [these][issue8311] [issues][issue8628]).

We can work around this limitation by building first and then running the tests:

```shell
cargo build && cargo test
```

Additionally, `cargo test` requires `gcc` to build a small C application as part of our integration tests.

[cargo-envs]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
[issue8193]: https://github.com/rust-lang/cargo/issues/8193
[issue8311]: https://github.com/rust-lang/cargo/issues/8311
[issue8628]: https://github.com/rust-lang/cargo/issues/8628

## Installation

The installation is a three step process:

1. Build the hook.
2. Install the hook in a location where it can be found by the OpenVPN client.
3. Configure the OpenVPN client to use the hook.

As discussed before, it must be built on the same machine where it will be used.

### Build the hook

The hook dynamic library can be built with Cargo, providing the password as a
build argument in the `BUILD_ARG_PASSWORD` environment variable. But being
careful not to leak the password in the shell history. For example, by using the
provided build script, which prompts for the password:

```shell
$ ./build.sh
Password:
   Compiling openvpn-auth-hook v0.1.0 (/home/manu/private/repos/openvpn-auth-hook)
    Finished release [optimized] target(s) in 3.22s
The dynamic library has been written to target/release/libopenvpn_auth_hook.so
```

We can also quickly verify that the hook works by running the test application,
which was automatically built during `cargo test`:

```shell
$ echo "user\npw" > /tmp/test_file
# WARNING: Prints the password in the terminal!
$ LD_PRELOAD=target/release/libopenvpn_auth_hook.so AUTH_FILE_PATH=/tmp/test_file tests/test_app /tmp/test_file 4096
user
<the_password_entered_during_build_sh>
```

### Install the hook

It just needs to be copied somewhere where the OpenVPN client can use it and
with appropriate permissions. But its permissions need to be restricted as much
as possible, exactly like it is done for the `auth-user-pass` file. For example:

```shell
# As root
# Assuming OpenVPN client is run as user `openvpn`
install -Dm400 -o openvpn target/release/libopenvpn_auth_hook.so /usr/local/lib/libopenvpn_auth_hook.so
```

### Configure OpenVPN client to use the hook

The `auth-user-pass` file must be defined with the real username (first line)
and a dummy or decoy password (second line).

The OpenVPN client must be configured to use the hook dynamic library. This can
be done by setting the `LD_PRELOAD` environment variable to the path where the
hook dynamic library was installed.

Additionally, the `AUTH_FILE_PATH` variable must be set to the path of the
`auth-user-pass` file that the hook should intercept. It has to be the same path
that the OpenVPN client is configured to use in the `auth-user-pass` directive.
But note that the path has to be completely identical; the hook does not perform
any kind of path normalization or expansion, it just does a byte-by-byte
comparison of the path provided in the `AUTH_FILE_PATH` environment variable
with the path provided to `fopen`.

#### Systemd example

Let's assume we have an OpenVPN client configuration named `office`, located at
`/etc/openvpn/client/office.conf`.

The Systemd service unit for this connection is therefore named
`openvpn-client@office.service`.

The `office.conf` file contains an `auth-user-pass` directive that points to the
relative `office/auth-user-pass.txt` file, which corresponds to
`/etc/openvpn/client/office/auth-user-pass.txt` in absolute terms. That file
contains:

```
my_username
my_decoy_password_that_is_not_used_but_must_be_here
```

Then the Systemd `Environment` directive can be used to set both the
`LD_PRELOAD` and `AUTH_FILE_PATH` environment variables for this connection
with:

```shell
# As root

# Configure the hook as a Systemd drop-in for the service unit
mkdir -p /etc/systemd/system/openvpn-client@office.service.d
echo -e '[Service]\nEnvironment=LD_PRELOAD=/usr/local/lib/libopenvpn_auth_hook.so AUTH_FILE_PATH=office/auth-user-pass.txt' > /etc/systemd/system/openvpn-client@office.service.d/openvpn-auth-hook.conf

# Apply the changes and restart the service
systemctl daemon-reload
systemctl restart openvpn-client@office.service

# Check the logs and make sure there are no errors and authentication is successful
journalctl -u openvpn-client@office.service
```
