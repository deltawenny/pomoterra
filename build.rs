//! Expose the dependency's matching font to NSH's asset packager.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let font =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("lucide.ttf");
    fs::write(font, lucide_icons::LUCIDE_FONT_BYTES).expect("write bundled Lucide font");
}
