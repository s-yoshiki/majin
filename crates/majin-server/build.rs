//! `rust-embed` refuses to compile when its source folder is missing, which
//! would make a fresh clone fail `cargo check` before the frontend has ever
//! been built. Creating an empty placeholder keeps the Rust side buildable on
//! its own; a real build gets the assets from `turbo run build`, which orders
//! the web app ahead of this crate.

use std::path::Path;

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/web/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(&dist);
    }
    println!("cargo:rerun-if-changed=../../apps/web/dist");
}
