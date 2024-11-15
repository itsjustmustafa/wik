use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc, thread};
use tui::text::{Span, Spans};
use std::collections::HashMap;

use crate::{
    app::App,
    caching::CachingSession,
    styles::{highlighted_snippet_style, unhighlighted_snippet_style},
    utils::Shared,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub extract: String,
    pub index: i32,
}


impl SearchResult {
    pub fn highlighted_snippets(search_results: &SearchResult) -> Spans {
        let mut spans = Vec::new();
        spans.push(Span::styled(
            search_results.extract.clone(),
            unhighlighted_snippet_style(),
        ));

        return Spans::from(spans);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Query {
    pages: HashMap<i32,SearchResult>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WikiSearchResponse {
    query: Query,
}

pub fn get_wikipedia_query(
    query: &str,
    shared_caching_session: Shared<CachingSession>,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?format=json&action=query&prop=extracts&generator=search&gsrsearch={}&exlimit=max&exintro&explaintext=1&exsectionformat=plain",
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
        Some(response) => {
            let mut sorted_response: Vec<SearchResult> = response.query.pages.values().cloned().collect();
            sorted_response.sort_by(|a, b| a.index.cmp(&b.index));
            return Ok(sorted_response);
        }
        None => Err("Could not get the query".into()),
    }
}

pub fn load_wikipedia_search_query_to_app(app: &App) {
    if app.input.len() > 0 {
        if app.is_this_lockable() {
            let input = app.input.clone();
            let loading_flag = Arc::clone(&app.is_loading_query);
            let app_results = Arc::clone(&app.results);
            let caching_session = Arc::clone(&app.cache);
            *loading_flag.lock().unwrap() = true;
            thread::spawn(move || {
                if let Ok(results) = get_wikipedia_query(input.as_str(), caching_session) {
                    *app_results.lock().unwrap() = results;
                    *loading_flag.lock().unwrap() = false;
                }
            });
        }
    }
}
