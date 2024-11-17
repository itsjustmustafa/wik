use crate::parsing::FormattedSpan;
use regex::Regex;

pub fn parse_markdown(text: &str) -> Vec<FormattedSpan> {
    let mut spans: Vec<FormattedSpan> = Vec::new();

    let heading_regex = Regex::new("^(?<hashes>#{1,6})\\s+(?<text>.*)").unwrap();
    let link_regex =
        Regex::new("\\[(?<text>.*?)\\]\\(\\./([^\\s]+) \\\"(?<link>[^\\\"]+)\\\"\\)").unwrap();
    let image_regex = Regex::new("^\\[\\!\\[").unwrap();

    let mut index = 0;

    for line in text.lines() {
        // check if the entire line is a heading, this is the only way a header should exist
        if let Some(captures) = heading_regex.captures(line) {
            if let Some(text_match) = captures.name("text") {
                if let Some(hashes) = captures.name("hashes") {
                    spans.push(FormattedSpan {
                        index: index,
                        text: text_match.as_str().to_owned(),
                        is_heading: true,
                        heading_level: hashes.len(),
                        link: None,
                        is_break: false,
                    });
                    index += 1;
                }
            }
        } else if let Some(_) = image_regex.captures(line) {
            continue;
        } else {
            let line_content = line.to_string();
            let mut current_pos: usize = 0;

            for link_capture in link_regex.captures_iter(line) {
                let start_pos = match link_capture.get(0) {
                    Some(link_match) => link_match.start(),
                    None => current_pos,
                };

                let end_pos = match link_capture.get(0) {
                    Some(link_match) => link_match.end(),
                    None => current_pos,
                };

                let text_part = match link_capture.name("text") {
                    Some(link_match) => link_match.as_str().to_string(),
                    None => String::from(""),
                };

                let link_part = match link_capture.name("link") {
                    Some(link_match) => link_match.as_str().to_string(),
                    None => String::from(""),
                };

                // Text before the link
                if current_pos < start_pos {
                    let pre_link_text = line_content[current_pos..start_pos].to_string();
                    spans.push(FormattedSpan {
                        index: index,
                        text: pre_link_text,
                        is_heading: false,
                        heading_level: 0,
                        link: None,
                        is_break: false,
                    });
                    index += 1;
                }
                // link snippet
                spans.push(FormattedSpan {
                    index: index,
                    text: text_part,
                    is_heading: false,
                    heading_level: 0,
                    link: Some(link_part),
                    is_break: false,
                });
                index += 1;

                current_pos = end_pos;
            }

            // Any remaining text
            if current_pos < line_content.len() {
                spans.push(FormattedSpan {
                    index: index,
                    text: line_content[current_pos..].to_string(),
                    is_heading: false,
                    heading_level: 0,
                    link: None,
                    is_break: false,
                });
                index += 1;
            }
        }
        spans.push(FormattedSpan {
            index,
            text: String::from(""),
            is_heading: false,
            heading_level: 0,
            link: None,
            is_break: true,
        });
        index += 1;
    }

    spans
}
