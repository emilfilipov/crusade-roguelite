use std::fs;
use std::path::Path;

fn display_version_from_installer_json() -> Option<String> {
    let path = Path::new("installer/version.json");
    let raw = fs::read_to_string(path).ok()?;
    let key = "\"installer_patch\"";
    let key_start = raw.find(key)?;
    let after_key = &raw[key_start + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = &after_key[colon + 1..];
    let digits: String = after_colon
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    let patch: u32 = digits.parse().ok()?;
    Some(format!("0.0.{patch}"))
}

fn main() {
    println!("cargo:rerun-if-changed=installer/version.json");

    let display_version = display_version_from_installer_json()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CRUSADE_ROGUELITE_DISPLAY_VERSION={display_version}");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/branding/game_icon.ico");
    res.compile()
        .expect("failed to compile Windows icon resource");
}
