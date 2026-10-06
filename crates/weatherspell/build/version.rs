// The version from the git tag, as MinVer gives 0.1's (Weatherspell.csproj:
// tag prefix "v", minor auto-increment), so both builds of a commit say
// the same: on a tagged commit the tag ("v0.1.0" is 0.1.0); after a
// release tag, the next minor version as an alpha with the number of
// commits since ("0.2.0-alpha.0.18"); after a pre-release tag, the commits
// added to it ("0.1.0-beta.1.3"). Never hand-edited.

pub struct Version {
    // "0.2.0-alpha.0.18": what the app shows.
    pub text: String,
    // 0, 2, 0: the numbers Windows' file version takes.
    pub numbers: (u32, u32, u32),
}

// From `git describe --tags --long --match "v[0-9]*"` ("v0.1.0-18-g09e697d"),
// or None without git or a tag.
pub fn from_describe(describe: Option<&str>) -> Version {
    let parsed = describe.and_then(|d| {
        let mut parts = d.trim().rsplitn(3, '-');
        let _hash = parts.next()?;
        let height: u32 = parts.next()?.parse().ok()?;
        let tag = parts.next()?.strip_prefix('v')?;
        Some((tag.to_string(), height))
    });
    let Some((tag, height)) = parsed else {
        return Version {
            text: "0.0.0-alpha.0".to_string(),
            numbers: (0, 0, 0),
        };
    };
    let (release, pre) = match tag.split_once('-') {
        Some((release, pre)) => (release, Some(pre)),
        None => (tag.as_str(), None),
    };
    let mut numbers = release.split('.').map(|n| n.parse::<u32>().unwrap_or(0));
    let (major, minor, patch) = (
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
    );
    match (height, pre) {
        (0, _) => Version {
            text: tag.clone(),
            numbers: (major, minor, patch),
        },
        (_, Some(_)) => Version {
            text: format!("{tag}.{height}"),
            numbers: (major, minor, patch),
        },
        (_, None) => Version {
            text: format!("{major}.{}.0-alpha.0.{height}", minor + 1),
            numbers: (major, minor + 1, 0),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::from_describe;

    #[test]
    fn versions_are_minvers() {
        let text = |d: Option<&str>| from_describe(d).text;
        assert_eq!(text(Some("v0.1.0-0-g09e697d")), "0.1.0");
        assert_eq!(text(Some("v0.1.0-18-g09e697d\n")), "0.2.0-alpha.0.18");
        assert_eq!(text(Some("v0.1.0-beta.1-0-g6fdb747")), "0.1.0-beta.1");
        assert_eq!(text(Some("v0.1.0-beta.1-3-g6fdb747")), "0.1.0-beta.1.3");
        assert_eq!(text(Some("v1.2.3-alpha.2-1-gabc")), "1.2.3-alpha.2.1");
        assert_eq!(text(None), "0.0.0-alpha.0");
        assert_eq!(text(Some("not a description")), "0.0.0-alpha.0");
        assert_eq!(from_describe(Some("v0.1.0-18-g09e697d")).numbers, (0, 2, 0));
        assert_eq!(from_describe(Some("v0.1.0-beta.1-3-gx")).numbers, (0, 1, 0));
    }
}
