// THIRD-PARTY-NOTICES.md must name every Rust library the exe is built
// from, at the version Cargo.lock has, and none that has gone: after a
// change to the dependencies, run tools/Update-Notices.ps1. The list comes
// from cargo tree, so CI needs no cargo-about to check it.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

// The workspace's own crates, which the notices leave out.
const OURS: [&str; 3] = ["weatherspell", "weatherspell-core", "wx-accessibility"];

#[test]
fn the_notices_name_every_library_in_the_exe() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let notices = std::fs::read_to_string(root.join("THIRD-PARTY-NOTICES.md")).unwrap();
    let named: BTreeSet<String> = notices
        .lines()
        .filter_map(|line| line.strip_prefix("Used by "))
        .flat_map(|list| list.trim_end_matches('.').split(", "))
        .map(str::to_string)
        .collect();

    let tree = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args([
            "tree",
            "--locked",
            "-p",
            "weatherspell",
            "-e",
            "normal",
            "--target",
            "x86_64-pc-windows-msvc",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .output()
        .unwrap();
    assert!(
        tree.status.success(),
        "{}",
        String::from_utf8_lossy(&tree.stderr)
    );
    let built: BTreeSet<String> = String::from_utf8(tree.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            (!OURS.contains(&name)).then(|| format!("{name} {version}"))
        })
        .collect();

    let missing: Vec<_> = built.difference(&named).collect();
    let gone: Vec<_> = named.difference(&built).collect();
    assert!(
        missing.is_empty() && gone.is_empty(),
        "THIRD-PARTY-NOTICES.md is out of date; run tools/Update-Notices.ps1.\nNot in it: {missing:?}\nNo longer built in: {gone:?}"
    );
}
