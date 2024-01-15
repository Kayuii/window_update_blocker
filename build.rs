use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    let git_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();
    println!("cargo:rustc-env=CARGO_BUILD_GIT_HASH={}", git_hash);

    if git_hash.len() > 5 {
        println!(
            "cargo:rustc-env=CARGO_BUILD_GIT_HASH_SHORT={}",
            &git_hash[..7]
        );
    } else {
        println!("cargo:rustc-env=CARGO_BUILD_GIT_HASH_SHORT=???????");
    }

    let utc = chrono::Local::now();
    let date = utc.format("%Y-%m-%d %H:%M").to_string();
    println!("cargo:rustc-env=CARGO_BUILD_DATE={}", date);
    let date = utc.format("%Y%m%d%H%M").to_string();
    println!("cargo:rustc-env=CARGO_BUILD_DATE_SHORT={}", date);
}
