use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/platform/wayland/wayland_shim.c");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let xdg_shell_xml = find_xdg_shell_xml().unwrap_or_else(|err| panic!("{err}"));
    let scanner = env::var("WAYLAND_SCANNER").unwrap_or_else(|_| "wayland-scanner".to_owned());
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_owned());

    let xdg_header = out_dir.join("xdg-shell-client-protocol.h");
    let xdg_code = out_dir.join("xdg-shell-protocol.c");
    run(
        Command::new(&scanner).args([
            "client-header",
            xdg_shell_xml
                .to_str()
                .expect("xdg-shell path is not valid UTF-8"),
            xdg_header
                .to_str()
                .expect("xdg header path is not valid UTF-8"),
        ]),
        "wayland-scanner client-header",
    );
    run(
        Command::new(&scanner).args([
            "private-code",
            xdg_shell_xml
                .to_str()
                .expect("xdg-shell path is not valid UTF-8"),
            xdg_code.to_str().expect("xdg code path is not valid UTF-8"),
        ]),
        "wayland-scanner private-code",
    );

    let shim_obj = out_dir.join("wayland_shim.o");
    run(
        Command::new(&cc)
            .arg("-c")
            .arg("src/platform/wayland/wayland_shim.c")
            .arg("-o")
            .arg(&shim_obj)
            .arg("-I")
            .arg(&out_dir),
        "compile wayland_shim.c",
    );

    let xdg_obj = out_dir.join("xdg-shell-protocol.o");
    run(
        Command::new(&cc)
            .arg("-c")
            .arg(&xdg_code)
            .arg("-o")
            .arg(&xdg_obj)
            .arg("-I")
            .arg(&out_dir),
        "compile xdg-shell-protocol.c",
    );

    let archive = out_dir.join("liboab_wayland.a");
    run(
        Command::new(&ar)
            .arg("crus")
            .arg(&archive)
            .arg(&shim_obj)
            .arg(&xdg_obj),
        "archive wayland objects",
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=oab_wayland");
    println!("cargo:rustc-link-lib=wayland-client");
}

fn find_xdg_shell_xml() -> Result<&'static Path, String> {
    let candidates: [&Path; 5] = [
        Path::new("/usr/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml"),
        Path::new("/usr/share/wayland-protocols/unstable/xdg-shell/xdg-shell.xml"),
        Path::new("/usr/share/qt6/wayland/protocols/xdg-shell/xdg-shell.xml"),
        Path::new("/usr/share/qt5/wayland/protocols/xdg-shell/xdg-shell.xml"),
        Path::new("/usr/local/share/wayland-protocols/stable/xdg-shell/xdg-shell.xml"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(
        "Failed to locate xdg-shell.xml. Install wayland-protocols (or Qt Wayland protocol files)."
            .to_owned(),
    )
}

fn run(cmd: &mut Command, step: &str) {
    let output = cmd.output().unwrap_or_else(|err| {
        panic!("{step} failed to start: {err}");
    });
    if output.status.success() {
        return;
    }
    panic!(
        "{step} failed (exit={:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
