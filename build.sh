#!/usr/bin/env bash

set -e

read -sp "Password: " BUILD_ARG_PASSWORD
echo
BUILD_ARG_PASSWORD="$BUILD_ARG_PASSWORD" cargo build --release
echo 'The dynamic library should be available at target/release/libopenvpn_auth_hook.so'
