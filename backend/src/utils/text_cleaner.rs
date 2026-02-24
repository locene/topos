use html_escape::decode_html_entities;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref HTML_TAG_REGEX: Regex = Regex::new(r"<[^>]*>").unwrap();
}

pub fn strip_and_decode_html(html_content: &str) -> String {
    let text_without_tags = HTML_TAG_REGEX.replace_all(html_content, "");
    let decoded_text = decode_html_entities(&text_without_tags);

    decoded_text.trim().to_string()
}
