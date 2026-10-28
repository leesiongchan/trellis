//! The static macOS ONNX archive uses Clang's OS-version availability helper.
//! Rust links with -nodefaultlibs, so Clang does not add this support archive itself.
use std::{path::PathBuf, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=CC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return Ok(());
    }
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    let output = Command::new(compiler)
        .arg("--print-file-name=libclang_rt.osx.a")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other("cannot locate the macOS Clang support archive").into());
    }
    let path = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    if !path.is_absolute() || !path.is_file() {
        return Err(std::io::Error::other("Clang macOS support archive is unavailable").into());
    }
    println!("cargo:rustc-link-arg={}", path.display());
    Ok(())
}
