use htmd::HtmlToMarkdown;
use ratatui::text::Span;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{error::Error, thread};

use crate::parsing;
use crate::parsing::FormattedSpan;
use crate::{caching::CachingSession, styles::Theme, utils::Shared};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub pageid: i32,
    pub snippet: String,
}

const OPENING_TAG: &str = "<span class=\"searchmatch\">";
const CLOSING_TAG: &str = "</span>";

const SEARCH_RESULT_LIMIT: u16 = 25;

impl SearchResult {
    pub fn highlighted_snippets<'a>(
        search_results: &'a SearchResult,
        theme: &'a Theme,
    ) -> Vec<Span<'a>> {
        // The search results from the Wikipedia API encloses the parts of the snippet
        // that matches the search term with the values in OPENING_TAG and then CLOSING_TAG,
        // This provides the snippet with the matches highlighted.

        let mut spans = Vec::new();
        let parts: Vec<&str> = search_results.snippet.split(OPENING_TAG).collect();

        for (i, &part) in parts.iter().enumerate() {
            // the strings in `part` will start with the highlighted text, followed by closing tag, and then rest of text
            // unless its the first part, in which case its just the starting unhighlighted bit
            if i > 0 {
                let highlighted_part = part.split(CLOSING_TAG).next().unwrap_or("");
                let remaining_part = part.split(CLOSING_TAG).nth(1).unwrap_or("");
                spans.push(Span::styled(
                    highlighted_part.to_string(),
                    theme.highlighted_snippet_style(),
                ));
                spans.push(Span::styled(
                    remaining_part.to_string(),
                    theme.unhighlighted_snippet_style(),
                ));
            } else {
                spans.push(Span::styled(part, theme.unhighlighted_snippet_style()));
            }
        }
        return spans;
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Query {
    search: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WikiSearchResponse {
    query: Query,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WikiPageData {
    pub title: String,
    pub markdown_content: String,
}

pub fn get_wikipedia_query(
    query: &str,
    shared_caching_session: Shared<CachingSession>,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit={SEARCH_RESULT_LIMIT}&format=json",
        query
    );
    let mut caching_session = shared_caching_session.lock().unwrap();

    let query_response: Option<WikiSearchResponse> = match caching_session.has_url(&url) {
        true => {
            // get form cache
            caching_session.get_from_cache::<WikiSearchResponse>(&url)
        }
        false => {
            let client = Client::new();
            let fresh_response = client.get(&url).send()?.json::<WikiSearchResponse>()?;
            caching_session.write_to_cache(&url, &fresh_response)?;
            Some(fresh_response)
        }
    };

    match query_response {
        Some(response) => Ok(response.query.search),
        None => Err("Could not get the query".into()),
    }
}

pub fn get_wikipedia_page(
    page_title: &str,
    shared_caching_session: Shared<CachingSession>,
) -> Result<Vec<FormattedSpan>, Box<dyn Error>> {
    let url = format!("https://en.wikipedia.org/w/rest.php/v1/page/{page_title}/html");
    let mut caching_session = shared_caching_session.lock().unwrap();

    let page_data_response: Option<WikiPageData> = match caching_session.has_url(&url.clone()) {
        true => caching_session.get_from_cache::<WikiPageData>(&url),
        false => {
            if let Ok(response) = reqwest::blocking::get(url.clone()) {
                if let Ok(html_content) = response.text() {
                    let converter = HtmlToMarkdown::builder()
                        .skip_tags(vec!["script", "style", "table", "sup"])
                        .build();

                    let markdown_text = match converter.convert(&html_content) {
                        Ok(content) => content,
                        Err(_) => String::from(""),
                    };

                    let page_data = WikiPageData {
                        title: url.clone(),
                        markdown_content: markdown_text.clone(),
                    };
                    caching_session.write_to_cache(&url.clone(), &page_data)?;
                    Some(page_data)
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    match page_data_response {
        Some(page_data) => {
            let mut spans = parsing::parse_markdown(&page_data.markdown_content);
            spans = remove_unnecessary_spans(spans);
            Ok(spans)
        }
        None => Err("Could not get page data".into()),
    }
}

pub fn load_search_query_to_app(
    input: String,
    loading_flag: Shared<bool>,
    search_results: Shared<Vec<SearchResult>>,
    cache: Shared<CachingSession>,
) {
    *loading_flag.lock().unwrap() = true;
    thread::spawn(move || {
        if let Ok(mut results) = get_wikipedia_query(input.as_str(), cache) {
            for search_result in results.iter_mut() {
                search_result.snippet = format!("...{}...", search_result.snippet);
            }
            *search_results.lock().unwrap() = results;
            *loading_flag.lock().unwrap() = false;
        }
    });
}

pub fn load_article_to_app(
    title: String,
    loading_flag: Shared<bool>,
    markdown_spans: Shared<Vec<FormattedSpan>>,
    link_indices: Shared<Vec<usize>>,
    cache: Shared<CachingSession>,
) {
    *loading_flag.lock().unwrap() = true;
    thread::spawn(move || {
        if let Ok(results) = get_wikipedia_page(title.as_str(), cache) {
            *markdown_spans.lock().unwrap() = results.clone();
            *loading_flag.lock().unwrap() = false;
            *link_indices.lock().unwrap() = results
                .iter()
                .filter(|formatted_span| {
                    if let Some(_) = formatted_span.link {
                        return true;
                    } else {
                        return false;
                    }
                })
                .map(|formatted_span| formatted_span.index)
                .collect::<Vec<usize>>();
        }
    });
}

pub fn remove_unnecessary_spans(mut spans: Vec<FormattedSpan>) -> Vec<FormattedSpan> {
    let mut remove_by_index: Vec<bool> = Vec::new();
    let mut found_see_also_header = false;
    let mut removing_flag = false;
    let flagged_titles = ["Notes", "References"];
    for (_, span) in spans.iter().enumerate() {
        if span.is_heading && found_see_also_header {
            removing_flag = true;
        }
        if span.is_heading && flagged_titles.contains(&span.text.as_str()) {
            removing_flag = true;
        }
        if span.is_heading && span.text == "See Also" {
            found_see_also_header = true;
        }

        remove_by_index.push(removing_flag);
    }

    spans.retain(|span| remove_by_index[span.index] == false);

    spans
}
