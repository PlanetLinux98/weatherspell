// What the app takes in at build time: the user guide and the third-party
// notices, made into pages from USER_GUIDE.md and THIRD-PARTY-NOTICES.md
// (build/guide.rs); the version, from the git tag
// (build/version.rs); and on Windows the resources: the manifest (common
// controls 6, which wxWidgets insists on, and per-monitor DPI awareness),
// the icon, and the version Explorer shows in the file's properties.

#[path = "build/guide.rs"]
mod guide;
#[path = "build/version.rs"]
mod version;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = crate_dir.join("../..");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let template = root.join("tools/GuideBuilder/template.html");
    watch(&template);
    let titles = [
        "Weatherspell User Guide",
        "Weatherspell Third-Party Notices",
    ];
    for ((md, html), title) in guide::PAGES.iter().zip(titles) {
        let md = root.join(md);
        watch(&md);
        let page = guide::page(&read(&md), &read(&template), title);
        fs::write(out.join(html), page).unwrap();
    }

    let version = version::from_describe(describe(&root).as_deref());
    println!("cargo::rustc-env=WEATHERSPELL_VERSION={}", version.text);

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = root.join("Assets/Weatherspell.ico");
        let manifest = crate_dir.join("weatherspell.exe.manifest");
        watch(&icon);
        watch(&manifest);
        let rc = out.join("weatherspell.rc");
        fs::write(&rc, resources(&version, &icon, &manifest)).unwrap();
        embed_resource::compile(&rc, embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn watch(path: &Path) {
    println!("cargo::rerun-if-changed={}", path.display());
}

// Nothing when there is no git or no tag (a source archive, say). The
// version is worked out again when HEAD, the branch or the tags move.
fn describe(root: &Path) -> Option<String> {
    let git = root.join(".git");
    if git.is_dir() {
        watch(&git.join("HEAD"));
        watch(&git.join("refs/tags"));
        watch(&git.join("packed-refs"));
        if let Some(branch) = fs::read_to_string(git.join("HEAD"))
            .ok()
            .and_then(|head| head.trim().strip_prefix("ref: ").map(str::to_string))
        {
            watch(&git.join(branch));
        }
    }
    let output = Command::new("git")
        .args(["describe", "--tags", "--long", "--match", "v[0-9]*"])
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// Icon 1 is the one Explorer shows for the exe, and the one the window
// loads (system.rs).
fn resources(version: &version::Version, icon: &Path, manifest: &Path) -> String {
    let quoted = |p: &Path| p.display().to_string().replace('\\', "\\\\");
    let (major, minor, patch) = version.numbers;
    let text = &version.text;
    format!(
        r#"1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "PlanetLinux98"
      VALUE "FileDescription", "Weatherspell"
      VALUE "FileVersion", "{major}.{minor}.{patch}.0"
      VALUE "InternalName", "weatherspell"
      VALUE "LegalCopyright", "Copyright (c) 2026 PlanetLinux98"
      VALUE "OriginalFilename", "weatherspell.exe"
      VALUE "ProductName", "Weatherspell"
      VALUE "ProductVersion", "{text}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = quoted(icon),
        manifest = quoted(manifest),
    )
}
