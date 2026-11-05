//! Puts the digging-D icon and the version into `diggr.exe`, for Explorer, the taskbar and Task
//! Manager. Nothing happens on other systems. When the exe is cross-built on a Mac, the resource
//! compiler is LLVM's `llvm-rc` (on the PATH, as `scripts/make-windows.sh` arranges).

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let icon = manifest.join("../../assets/icon/diggr.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", icon.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets it");
    let mut numbers = version
        .split(['.', '-'])
        .map(|part| part.parse::<u16>().unwrap_or(0));
    let [major, minor, patch] = std::array::from_fn(|_| numbers.next().unwrap_or(0));
    let rc = format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "ProductName", "Diggr\0"
      VALUE "FileDescription", "Diggr\0"
      VALUE "InternalName", "diggr\0"
      VALUE "OriginalFilename", "Diggr.exe\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = icon.display().to_string().replace('\\', "/"),
    );
    let file = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it")).join("diggr.rc");
    std::fs::write(&file, rc).expect("write diggr.rc");
    embed_resource::compile(&file, embed_resource::NONE)
        .manifest_required()
        .expect(
            "embedding the icon and version (llvm-rc not on the PATH? see README, Build a release)",
        );
}
