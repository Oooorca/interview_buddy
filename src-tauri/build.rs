use std::{env, path::PathBuf, process::Command};

fn main() {
    configure_macos_swift_linker();
    tauri_build::build()
}

fn configure_macos_swift_linker() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");

    let developer_dir = env::var_os("DEVELOPER_DIR").map(PathBuf::from).or_else(|| {
        let output = Command::new("xcode-select").arg("-p").output().ok()?;
        output
            .status
            .success()
            .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
    });

    let Some(developer_dir) = developer_dir else {
        println!(
            "cargo:warning=Unable to find the Apple developer directory; Swift compatibility libraries may not link"
        );
        return;
    };

    // Full Xcode and the standalone Command Line Tools use different layouts.
    // Swift-based dependencies such as screencapturekit currently assume the
    // full-Xcode layout, so add the CLT layout when that is what is installed.
    let candidates = [
        developer_dir.join("Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"),
        developer_dir.join("usr/lib/swift/macosx"),
    ];

    if let Some(swift_lib_dir) = candidates.into_iter().find(|path| path.is_dir()) {
        println!("cargo:rustc-link-search=native={}", swift_lib_dir.display());
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            swift_lib_dir.display()
        );
    } else {
        println!(
            "cargo:warning=Swift compatibility library directory was not found under {}",
            developer_dir.display()
        );
    }
}
