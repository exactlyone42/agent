use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

fn is_local_file_link(target: &str) -> bool {
    !target.contains("://") && !target.starts_with('#') && !target.starts_with("mailto:")
}

pub fn markdown_html(markdown: &str) -> String {
    let mut stripped = false;
    let events =
        Parser::new_ext(markdown, Options::ENABLE_TABLES).filter_map(move |event| match event {
            Event::Start(Tag::Link { ref dest_url, .. }) if is_local_file_link(dest_url) => {
                stripped = true;
                None
            }
            Event::End(TagEnd::Link) if stripped => {
                stripped = false;
                None
            }
            other => Some(other),
        });
    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}
