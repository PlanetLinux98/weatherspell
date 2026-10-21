// THIRD-PARTY-NOTICES.md must name every Rust library the shipped apps are
// built from, at the version Cargo.lock has, and none that has gone: after
// a change to the dependencies, run tools/Update-Notices.ps1. The list
// comes from cargo tree, so CI needs no cargo-about to check it.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

// The workspace's own crates, which the notices leave out.
const OURS: [&str; 3] = ["weatherspell", "weatherspell-core", "wx-accessibility"];

// One page covers every system, so it names what each build adds (the
// Mac's and Linux's certificate stores, for one): the same list as
// tools/notices/about.toml. cargo tree needs no installed target for these.
const TARGETS: [&str; 4] = [
    "x86_64-pc-windows-msvc",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
];

// cargo-about joins the targets into one graph, so it also credits these,
// which no single build has: schannel is rustls-native-certs' Windows
// half, and only Linux builds rustls-native-certs. Crediting too much does
// no harm.
const JOINED_ONLY: [&str; 1] = ["schannel"];

#[test]
fn the_notices_name_every_library_in_the_apps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let notices = std::fs::read_to_string(root.join("THIRD-PARTY-NOTICES.md")).unwrap();
    let named: BTreeSet<String> = notices
        .lines()
        .filter_map(|line| line.strip_prefix("Used by "))
        .flat_map(|list| list.trim_end_matches('.').split(", "))
        .map(str::to_string)
        .collect();

    let mut built = BTreeSet::new();
    for target in TARGETS {
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
                target,
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
        built.extend(
            String::from_utf8(tree.stdout)
                .unwrap()
                .lines()
                .filter_map(|line| {
                    let mut words = line.split_whitespace();
                    let name = words.next()?;
                    let version = words.next()?.strip_prefix('v')?;
                    (!OURS.contains(&name)).then(|| format!("{name} {version}"))
                }),
        );
    }

    let missing: Vec<_> = built.difference(&named).collect();
    let gone: Vec<_> = named
        .difference(&built)
        .filter(|crate_| {
            !JOINED_ONLY
                .iter()
                .any(|name| crate_.split(' ').next() == Some(name))
        })
        .collect();
    assert!(
        missing.is_empty() && gone.is_empty(),
        "THIRD-PARTY-NOTICES.md is out of date; run tools/Update-Notices.ps1.\nNot in it: {missing:?}\nNo longer built in: {gone:?}"
    );
}
