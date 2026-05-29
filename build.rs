use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GALAXY_BUILD_META");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/logs/HEAD");

    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "dev".to_string());
    let build_meta = std::env::var("GALAXY_BUILD_META")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(default_build_meta);

    println!("cargo:rustc-env=GALAXY_BUILD_VERSION={pkg_version}+{build_meta}");
}

fn default_build_meta() -> String {
    command_output("git", &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string())
}

fn command_output(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
