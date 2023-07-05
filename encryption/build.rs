fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_ARG_APP_ID");
}
