//! Rebuilds the crate when a file is added under `web/`; the embedding macro alone only
//! notices changes to files that already existed.
fn main() {
    println!("cargo:rerun-if-changed=../../web");
}
