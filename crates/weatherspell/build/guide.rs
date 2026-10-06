// USER_GUIDE.md as the one page the app opens on F1, as tools/GuideBuilder
// makes it for 0.1: CommonMark with pipe tables, in the template's page,
// and GitHub's heading ids, so a link such as #credits-and-licences works
// on GitHub and in the page alike.

use std::collections::HashMap;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

pub fn page(markdown: &str, template: &str) -> String {
    let mut events: Vec<Event> = Parser::new_ext(markdown, Options::ENABLE_TABLES).collect();
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < events.len() {
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
    template.replace("{{content}}", body.trim_end())
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
        let page = page("# A\n\n## Same\n\n## Same\n", "{{content}}");
        assert!(page.contains("<h2 id=\"same\">Same</h2>"));
        assert!(page.contains("<h2 id=\"same-1\">Same</h2>"));
    }

    // Every link within the guide reaches a heading in the built page, and
    // the section About opens is there.
    #[test]
    fn the_guides_own_links_all_land() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let markdown = std::fs::read_to_string(root.join("USER_GUIDE.md")).unwrap();
        let template =
            std::fs::read_to_string(root.join("tools/GuideBuilder/template.html")).unwrap();
        let page = page(&markdown, &template);
        assert!(page.starts_with("<!DOCTYPE html>"));
        assert!(page.contains("<html lang=\"en-CA\">"));
        assert!(page.contains("id=\"credits-and-licences\""));
        let links: Vec<&str> = markdown
            .split("](#")
            .skip(1)
            .map(|rest| rest.split(')').next().unwrap())
            .collect();
        assert!(links.len() > 5);
        for link in links {
            assert!(page.contains(&format!("id=\"{link}\"")), "#{link}");
        }
    }
}
