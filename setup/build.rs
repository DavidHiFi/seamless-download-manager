//! Builds the installer window and packs the release programs into it.
//!
//! The payload is the app and the browser helper from a release build. It comes from
//! `SDM_PAYLOAD_DIR`, or the release folder beside this build. A plain `cargo build` of the
//! workspace builds the installer before those exist, so a missing payload packs empty
//! stand-ins and the installer refuses to run. Release builds set
//! `SDM_SETUP_REQUIRE_PAYLOAD=1` so a missing file fails the build instead.

use std::path::PathBuf;

const FILES: [&str; 2] = ["seamless-download-manager.exe", "sdm-host.exe"];

fn main() {
    slint_build::compile_with_config(
        "ui/setup.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent-dark".into()),
    )
    .expect("the installer layout failed to compile");
    embed_resource::compile("assets/setup.rc", embed_resource::NONE).manifest_required().expect("icon resource");

    println!("cargo:rerun-if-env-changed=SDM_PAYLOAD_DIR");
    println!("cargo:rerun-if-env-changed=SDM_SETUP_REQUIRE_PAYLOAD");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dir = std::env::var_os("SDM_PAYLOAD_DIR").map(PathBuf::from).unwrap_or_else(|| {
        // OUT_DIR is <target>/<profile>/build/<pkg>-<hash>/out.
        out.ancestors().nth(3).map(|p| p.to_path_buf()).unwrap_or_default()
    });
    let require = std::env::var("SDM_SETUP_REQUIRE_PAYLOAD").is_ok_and(|v| v == "1");
    for name in FILES {
        let src = dir.join(name);
        let dst = out.join(name);
        println!("cargo:rerun-if-changed={}", src.display());
        if src.is_file() {
            std::fs::copy(&src, &dst).expect("copy payload");
        } else if require {
            panic!("payload file missing: {}", src.display());
        } else {
            std::fs::write(&dst, b"").expect("write stand-in");
        }
    }
}
