//! Expose the dependency's matching font to NSH's asset packager.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let font =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("lucide.ttf");
    fs::write(font, lucide_icons::LUCIDE_FONT_BYTES).expect("write bundled Lucide font");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
        && env::var_os("CARGO_FEATURE_NATIVE").is_some()
    {
        println!("cargo:rerun-if-changed=assets/icons/pomoterra.gresource.xml");
        println!("cargo:rerun-if-changed=assets/icons/pomoterra.png");
        let resource = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"))
            .join("pomoterra.gresource");
        let status = std::process::Command::new("glib-compile-resources")
            .arg("--sourcedir=assets/icons")
            .arg("--target")
            .arg(resource)
            .arg("assets/icons/pomoterra.gresource.xml")
            .status()
            .expect("glib-compile-resources is required for the native Linux application icon");
        assert!(status.success(), "compile Pomoterra icon resources");
    }
    println!("cargo:rerun-if-changed=assets/icons/pomoterra.rc");
    println!("cargo:rerun-if-changed=assets/icons/pomoterra.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icons = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/icons");
        embed_resource::compile(
            "assets/icons/pomoterra.rc",
            embed_resource::ParamsIncludeDirs([icons]),
        )
        .manifest_required()
        .expect("embed Pomoterra application icon");
    }
}
