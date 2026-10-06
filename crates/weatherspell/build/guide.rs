// USER_GUIDE.md as the page the app opens on F1, as tools/GuideBuilder
// makes it for 0.1: CommonMark with pipe tables, in the template's page,
// and GitHub's heading ids, so a link such as #credits-and-licences works
// on GitHub and in the page alike. THIRD-PARTY-NOTICES.md becomes a page
// beside it, and links between the two files lead to the pages instead.

use std::collections::HashMap;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};

// Each file and the page the app writes it as, in the same folder.
pub const PAGES: [(&str, &str); 2] = [
    ("USER_GUIDE.md", "user-guide.html"),
    ("THIRD-PARTY-NOTICES.md", "third-party-notices.html"),
];

pub fn page(markdown: &str, template: &str, title: &str) -> String {
    let mut events: Vec<Event> = Parser::new_ext(markdown, Options::ENABLE_TABLES).collect();
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < events.len() {
        if let Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) = &events[i]
            && let Some(local) = local_page(dest_url)
        {
            events[i] = Event::Start(Tag::Link {
                link_type: *link_type,
                dest_url: CowStr::from(local),
                title: title.clone(),
                id: id.clone(),
            });
        }
        if let Event::Start(Tag::Heading { id: None, .. }) = &events[i] {
            let mut text = String::new();
            for event in &events[i + 1..] {
                match event {
                    Event::End(TagEnd::Heading(_)) => break,
                    Event::Text(t) | Event::Code(t) => text.push_str(t),
                    _ => {}
                }
            }
            let id = unique(github_id(&text), &mut used);
            if let Event::Start(Tag::Heading {
                level,
                classes,
                attrs,
                ..
            }) = events[i].clone()
            {
                events[i] = Event::Start(Tag::Heading {
                    level,
                    id: Some(id.into()),
                    classes,
                    attrs,
                });
            }
        }
        i += 1;
    }
    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    template
        .replace(
            "<title>Weatherspell User Guide</title>",
            &format!("<title>{title}</title>"),
        )
        .replace("{{content}}", body.trim_end())
}

// "USER_GUIDE.md#credits-and-licences" is "user-guide.html#credits-and-licences".
fn local_page(url: &str) -> Option<String> {
    let (file, fragment) = match url.split_once('#') {
        Some((file, fragment)) => (file, Some(fragment)),
        None => (url, None),
    };
    let (_, html) = PAGES.iter().find(|(md, _)| *md == file)?;
    Some(match fragment {
        Some(fragment) => format!("{html}#{fragment}"),
        None => html.to_string(),
    })
}

// GitHub's: lower case, punctuation dropped (but not hyphens or
// underscores), each space a hyphen.
pub fn github_id(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

// A repeated heading gets "-1", "-2" after it, as on GitHub.
fn unique(id: String, used: &mut HashMap<String, usize>) -> String {
    let count = used.entry(id.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        id
    } else {
        format!("{id}-{}", *count - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_ids_are_githubs() {
        assert_eq!(github_id("Credits and licences"), "credits-and-licences");
        assert_eq!(
            github_id("Reading an alert's details"),
            "reading-an-alerts-details"
        );
        assert_eq!(
            github_id("Text size and the window"),
            "text-size-and-the-window"
        );
        let page = page("# A\n\n## Same\n\n## Same\n", "{{content}}", "A");
        assert!(page.contains("<h2 id=\"same\">Same</h2>"));
        assert!(page.contains("<h2 id=\"same-1\">Same</h2>"));
    }

    #[test]
    fn links_between_the_files_lead_to_their_pages() {
        let page = page(
            "[a](USER_GUIDE.md#credits-and-licences) [b](THIRD-PARTY-NOTICES.md) [c](https://example.com/USER_GUIDE.md)",
            "<title>Weatherspell User Guide</title>{{content}}",
            "Third-party notices",
        );
        assert!(page.starts_with("<title>Third-party notices</title>"));
        assert!(page.contains("href=\"user-guide.html#credits-and-licences\""));
        assert!(page.contains("href=\"third-party-notices.html\""));
        assert!(page.contains("href=\"https://example.com/USER_GUIDE.md\""));
    }

    // Every link within the guide and the notices reaches a heading in the
    // built pages, and the section About opens is there.
    #[test]
    fn the_guides_own_links_all_land() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let template =
            std::fs::read_to_string(root.join("tools/GuideBuilder/template.html")).unwrap();
        let pages: Vec<(&str, String, String)> = PAGES
            .iter()
            .map(|(md, html)| {
                let markdown = std::fs::read_to_string(root.join(md)).unwrap();
                let built = page(&markdown, &template, md);
                (*html, markdown, built)
            })
            .collect();
        let built = |html: &str| &pages.iter().find(|(h, _, _)| *h == html).unwrap().2;
        assert!(built("user-guide.html").starts_with("<!DOCTYPE html>"));
        assert!(built("user-guide.html").contains("<html lang=\"en-CA\">"));
        assert!(built("user-guide.html").contains("id=\"credits-and-licences\""));
        let mut count = 0;
        for (html, markdown, _) in &pages {
            for rest in markdown.split("](").skip(1) {
                let link = rest.split(')').next().unwrap();
                let (file, fragment) = match link.split_once('#') {
                    Some((file, fragment)) => (file, Some(fragment)),
                    None => (link, None),
                };
                let target = if file.is_empty() {
                    *html
                } else if let Some((_, target)) = PAGES.iter().find(|(md, _)| *md == file) {
                    target
                } else {
                    assert!(file.contains("://"), "{html}: {link}");
                    continue;
                };
                if let Some(fragment) = fragment {
                    assert!(
                        built(target).contains(&format!("id=\"{fragment}\"")),
                        "{html}: {link}"
                    );
                }
                count += 1;
            }
        }
        assert!(count > 5);
    }
}
