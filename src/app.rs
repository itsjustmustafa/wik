use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::parsing::FormattedSpan;
use crate::styles::Theme;
use crate::utils::clargs::Args;
use crate::utils::{create_shared, remainder, shared_copy};
use crate::wikipedia::{self, SearchResult};
use crate::{caching::CachingSession, utils::Shared};

use std::char;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

pub enum AppState {
    Title,
    Search,
    SearchMenu,
    Article,
    ArticleMenu,
    Credit,
    ThemeMenu,
}
pub type AppAction = Arc<dyn Fn(&mut App) + Send + Sync>;

pub struct ActionItem {
    label: String,
    action: AppAction,
}

impl ActionItem {
    pub fn new<F>(label: &str, action: F) -> Self
    where
        F: Fn(&mut App) + Send + Sync + 'static,
    {
        ActionItem {
            label: label.to_string(),
            action: Arc::new(action),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn action_clone(&self) -> AppAction {
        Arc::clone(&self.action)
    }
}

pub enum ScrollDirection {
    UP,
    DOWN,
}

pub enum CursorDirection {
    LEFT,
    RIGHT,
}

pub trait ActionMenu {
    fn total_options(&self) -> usize;
    fn set_index(&mut self, new_index: usize) -> ();
    fn get_index(&self) -> usize;
    fn get_options(&self) -> &Vec<ActionItem>;

    fn scroll(&mut self, scroll_direction: ScrollDirection) -> () {
        let total_options = self.total_options();

        match scroll_direction {
            ScrollDirection::DOWN => self.set_index(remainder(self.get_index() + 1, total_options)),
            ScrollDirection::UP => self.set_index(
                remainder(self.get_index() as i64 - 1, total_options as i64)
                    .try_into()
                    .unwrap_or(0),
            ),
        }
    }

    fn get_selected_action(&self) -> AppAction {
        let selected_option = &self.get_options()[self.get_index()];
        selected_option.action_clone()
    }

    fn handle_key(&mut self, keyevent: KeyEvent) {
        match keyevent.code {
            KeyCode::Up => {
                self.scroll(ScrollDirection::UP);
            }
            KeyCode::Down => {
                self.scroll(ScrollDirection::DOWN);
            }
            _ => {}
        }
    }
}

pub trait TypeableState {
    fn get_input(&self) -> String;
    fn insert_to_input_at_cursor(&mut self, c: char) -> ();
    fn remove_from_input_at_cursor(&mut self) -> ();
    fn get_cursor_pos(&self) -> usize;
    fn set_cursor_pos(&mut self, new_cursor_pos: usize) -> ();
    fn trigger_text_focus(&mut self) -> () {}

    fn move_cursor_to_start(&mut self) -> () {
        self.set_cursor_pos(0);
        self.trigger_text_focus();
    }

    fn move_cursor_to_end(&mut self) -> () {
        self.set_cursor_pos(self.get_input().len());
        self.trigger_text_focus();
    }

    fn move_cursor_one_step(&mut self, cursor_direction: CursorDirection) {
        // Limit cursor_pos to be between 0 and (length of the input) - 1
        match cursor_direction {
            CursorDirection::LEFT => {
                self.set_cursor_pos(self.get_cursor_pos().saturating_sub(1));
            }
            CursorDirection::RIGHT => {
                if self.get_cursor_pos() < self.get_input().len() {
                    self.set_cursor_pos(self.get_cursor_pos() + 1);
                }
            }
        }
        self.set_cursor_pos(self.get_cursor_pos().clamp(0, self.get_input().len() + 1));
        self.trigger_text_focus();
    }

    fn type_char(&mut self, c: char) {
        if !(self.get_cursor_pos() > self.get_input().len()) {
            self.insert_to_input_at_cursor(c);
            self.move_cursor_one_step(CursorDirection::RIGHT);
        }
        self.trigger_text_focus();
    }

    fn backspace(&mut self) {
        if !(self.get_input().is_empty()) {
            if self.get_cursor_pos() > 0 {
                self.remove_from_input_at_cursor();
                self.move_cursor_one_step(CursorDirection::LEFT);
            }
        }
        self.trigger_text_focus();
    }
    fn handle_key(&mut self, keyevent: KeyEvent) {
        match keyevent.code {
            KeyCode::Char(c) => {
                // Append character to input
                self.type_char(c);
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Left if keyevent.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_to_start();
            }
            KeyCode::Left => {
                self.move_cursor_one_step(CursorDirection::LEFT);
            }
            KeyCode::Right if keyevent.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_to_end();
            }
            KeyCode::Right => {
                self.move_cursor_one_step(CursorDirection::RIGHT);
            }
            _ => {}
        }
    }
}

pub struct TitleState {
    pub input: String,
    pub cursor_pos: usize,
}

impl TypeableState for TitleState {
    fn get_input(&self) -> String {
        self.input.clone()
    }

    fn insert_to_input_at_cursor(&mut self, c: char) -> () {
        self.input.insert(self.cursor_pos, c);
    }

    fn remove_from_input_at_cursor(&mut self) -> () {
        if self.cursor_pos <= self.input.len() && self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
        }
    }

    fn get_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_cursor_pos(&mut self, new_cursor_pos: usize) -> () {
        self.cursor_pos = new_cursor_pos;
    }
}

pub struct SearchState {
    pub input: String,
    pub current_query: String,
    pub results: Shared<Vec<SearchResult>>,
    pub cursor_pos: usize,
    pub is_loading_query: Shared<bool>,
    pub selected_index: usize,
    pub text_box_is_highlighted: bool,
}

impl SearchState {
    pub fn currently_loading(&self) -> bool {
        match self.is_loading_query.try_lock() {
            Ok(is_loading) => *is_loading,
            Err(_) => true,
        }
    }

    pub fn scroll_results(&mut self, scroll_direction: ScrollDirection) {
        if !self.currently_loading() {
            let results = self.results.lock().unwrap();
            if results.len() > 0 {
                match scroll_direction {
                    ScrollDirection::DOWN => {
                        self.selected_index = remainder(self.selected_index + 1, results.len());
                    }
                    ScrollDirection::UP => {
                        self.selected_index =
                            remainder(self.selected_index as i64 - 1, results.len() as i64)
                                as usize;
                    }
                }
            }
        }
        self.text_box_is_highlighted = false;
    }

    pub fn selected_search_result_title(&self) -> Option<String> {
        match self.results.lock().unwrap().get(self.selected_index) {
            Some(result) => Some(result.title.clone()),
            None => None,
        }
    }
}

impl TypeableState for SearchState {
    fn get_input(&self) -> String {
        self.input.clone()
    }

    fn insert_to_input_at_cursor(&mut self, c: char) -> () {
        self.input.insert(self.cursor_pos, c);
    }

    fn remove_from_input_at_cursor(&mut self) -> () {
        if self.cursor_pos <= self.input.len() && self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
        }
    }

    fn get_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_cursor_pos(&mut self, new_cursor_pos: usize) -> () {
        self.cursor_pos = new_cursor_pos;
    }

    fn trigger_text_focus(&mut self) -> () {
        self.text_box_is_highlighted = true;
    }
}
pub struct MenuState {
    pub selected_index: usize,
    pub options: Vec<ActionItem>,
}

impl ActionMenu for MenuState {
    fn total_options(&self) -> usize {
        self.options.len()
    }

    fn set_index(&mut self, new_index: usize) -> () {
        self.selected_index = new_index;
    }

    fn get_index(&self) -> usize {
        self.selected_index
    }

    fn get_options(&self) -> &Vec<ActionItem> {
        &self.options
    }
}

pub struct CreditState {
    pub selected_index: usize,
    pub options: Vec<ActionItem>,
}

impl ActionMenu for CreditState {
    fn total_options(&self) -> usize {
        self.options.len()
    }

    fn set_index(&mut self, new_index: usize) -> () {
        self.selected_index = new_index;
    }

    fn get_index(&self) -> usize {
        self.selected_index
    }

    fn get_options(&self) -> &Vec<ActionItem> {
        &self.options
    }
}

pub struct ArticleState {
    pub article_name: String,
    pub markdown_spans: Shared<Vec<FormattedSpan>>,
    pub has_loaded_article: Shared<bool>,
    pub link_span_indices: Shared<Vec<usize>>,
    pub is_valid_page: Shared<bool>,
    pub selected_link_index: usize,
    pub vertical_scroll: usize,
    back_history: VecDeque<String>,
    forward_history: VecDeque<String>,
}

impl ArticleState {
    pub fn scroll_link(&mut self, direction: ScrollDirection) {
        if let Ok(indices_results) = self.link_span_indices.try_lock() {
            let total_indices = (*indices_results).len();
            if total_indices > 0 {
                let increment = match direction {
                    ScrollDirection::UP => total_indices.saturating_sub(1),
                    ScrollDirection::DOWN => total_indices.saturating_add(1),
                };
                self.selected_link_index =
                    remainder(self.selected_link_index + increment, total_indices);
            }
        }
    }

    pub fn scroll_vertically(&mut self, direction: ScrollDirection) {
        match direction {
            ScrollDirection::UP => self.vertical_scroll = self.vertical_scroll.saturating_sub(1),
            ScrollDirection::DOWN => self.vertical_scroll = self.vertical_scroll.saturating_add(1),
        }
    }

    pub fn get_selected_link(&self) -> Option<String> {
        if let Ok(indices_results) = self.link_span_indices.try_lock() {
            if let Some(&index) = (*indices_results).get(self.selected_link_index) {
                if let Ok(spans) = self.markdown_spans.try_lock() {
                    if let Some(span) = (*spans).get(index) {
                        return span.link.clone();
                    }
                }
            }
        }
        return None;
    }

    pub fn go_back_a_page(&mut self) {
        // take the last off back_history, put it at front of forward_history
        if self.back_history.len() <= 1 {
            return;
        }
        if let Some(title) = self.back_history.pop_back() {
            self.forward_history.push_front(title);
        }
    }

    pub fn go_forward_a_page(&mut self) {
        // take the first off forward_history, put it at back of back_history
        if let Some(title) = self.forward_history.pop_front() {
            self.back_history.push_back(title);
        }
    }
}

pub struct ThemeState {
    pub themes: Vec<Theme>,
    pub options: Vec<ActionItem>,
    pub selected_index: usize,
}

impl ActionMenu for ThemeState {
    fn total_options(&self) -> usize {
        self.options.len()
    }

    fn set_index(&mut self, new_index: usize) -> () {
        self.selected_index = new_index;
    }

    fn get_index(&self) -> usize {
        self.selected_index
    }

    fn get_options(&self) -> &Vec<ActionItem> {
        &self.options
    }
}

pub struct App {
    pub title: TitleState,
    pub search: SearchState,
    pub search_menu: MenuState,
    pub credit: CreditState,
    pub article: ArticleState,
    pub article_menu: MenuState,
    pub theme_menu: ThemeState,
    pub cache: Shared<CachingSession>,
    pub is_running: bool,
    pub state: AppState,
    pub theme: Theme,
    pub config: Args,
    pub debug_text: String,
}

impl Default for App {
    fn default() -> Self {
        let mut app = App {
            title: TitleState {
                input: String::new(),
                cursor_pos: 0,
            },
            search: SearchState {
                input: String::new(),
                current_query: String::new(),
                results: create_shared(Vec::new()),
                cursor_pos: 0,
                is_loading_query: create_shared(false),
                selected_index: 0,
                text_box_is_highlighted: true,
            },
            search_menu: MenuState {
                selected_index: 0,
                options: vec![],
            },
            credit: CreditState {
                selected_index: 0,
                options: vec![],
            },
            article: ArticleState {
                article_name: String::from("Philosophy"),
                markdown_spans: create_shared(Vec::new()),
                has_loaded_article: create_shared(false),
                link_span_indices: create_shared(vec![]),
                is_valid_page: create_shared(true),
                selected_link_index: 0,
                vertical_scroll: 0,
                back_history: VecDeque::new(),
                forward_history: VecDeque::new(),
            },
            article_menu: MenuState {
                selected_index: 0,
                options: vec![],
            },
            theme_menu: ThemeState {
                themes: vec![],
                selected_index: 0,
                options: vec![],
            },
            cache: create_shared(CachingSession::new()),
            is_running: false,
            state: AppState::Title,
            theme: Theme::default(),
            config: Args::default(),
            debug_text: String::from(""),
        };

        app.search_menu.options = vec![
            ActionItem::new("Resume", |app| app.state = AppState::Search),
            ActionItem::new("Themes", |app| app.state = AppState::ThemeMenu),
            ActionItem::new("Credits", |app| app.state = AppState::Credit),
            ActionItem::new("Quit", |app| app.is_running = false),
        ];

        app.article_menu.options = vec![
            ActionItem::new("Resume", |app| app.state = AppState::Article),
            ActionItem::new("Search", |app| app.state = AppState::Search),
            ActionItem::new("← Go back", |app| app.go_to_previous_article()),
            ActionItem::new("Go forward →", |app| app.go_to_next_article()),
            ActionItem::new("Quit", |app| app.is_running = false),
        ];

        app.credit.options = vec![
            ActionItem::new("Go to repo!", |_| {
                webbrowser::open("https://github.com/itsjustmustafa/wik").unwrap_or(())
            }),
            ActionItem::new("Back to menu", |app| app.state = AppState::SearchMenu),
        ];

        let theme_file_result = File::options().read(true).write(false).open("./themes.txt");
        if let Ok(theme_file) = theme_file_result {
            let reader = BufReader::new(theme_file);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        let line_split: Vec<&str> = line.split(' ').collect();
                        let maybe_theme_name = line_split.get(0);
                        let maybe_theme_colors = line_split.get(1);
                        if let Some(&theme_name) = maybe_theme_name {
                            if let Some(&theme_colors) = maybe_theme_colors {
                                app.theme_menu.themes.push(Theme::from_hex_string_series(
                                    String::from(theme_name),
                                    String::from(theme_colors),
                                ));
                                app.theme_menu.options.push(ActionItem::new(
                                    theme_name,
                                    move |app| {
                                        // app.theme_menu.selected_index = line_index.to_owned();
                                        app.theme = app.theme_menu.themes
                                            [app.theme_menu.selected_index]
                                            .clone();
                                    },
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        app.theme_menu.themes.push(Theme::from_hex_string_series(
                            String::from("Normal"),
                            String::from("2a3138-ffffff-c19c00-13a10e-3b78ff-000000"),
                        ));
                        app.theme_menu
                            .options
                            .push(ActionItem::new("Normal", move |app| {
                                app.theme =
                                    app.theme_menu.themes[app.theme_menu.selected_index].clone();
                            }));
                    }
                }
            }
        }

        app
    }
}

impl App {
    pub fn new() -> Self {
        App::default()
    }

    pub fn load_wikipedia_search_query(&mut self) {
        if self.search.input.len() > 0 {
            if !self.search.currently_loading() {
                let input = self.search.input.clone();
                self.search.current_query = input.clone();

                let loading_flag = shared_copy(&self.search.is_loading_query);
                let app_results = shared_copy(&self.search.results);
                let caching_session = shared_copy(&self.cache);

                wikipedia::load_search_query_to_app(
                    input,
                    loading_flag,
                    app_results,
                    caching_session,
                );
            }
        }
        self.search.text_box_is_highlighted = false;
    }

    pub fn search_and_load(&mut self, title: String) {
        self.state = AppState::Search;
        self.search.input = title;
        self.load_wikipedia_search_query();
    }

    pub fn try_getting_page(&mut self, title: String) {
        // load the page, if a Page is found, return Ok, else Err
        self.set_article_page(title.clone());

        let is_valid_page: bool;
        loop {
            match self.article.has_loaded_article.try_lock() {
                Ok(loaded_result) => match *loaded_result {
                    true => {
                        is_valid_page = *self.article.is_valid_page.lock().unwrap();
                        break;
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
        if is_valid_page {
            self.state = AppState::Article;
            return;
        }
        self.search_and_load(title.clone());
    }

    pub fn set_article_page(&mut self, title: String) {
        // *self.article.has_loaded_article.lock().unwrap() = false;

        self.article.article_name = title.clone();
        let markdown_spans = shared_copy(&self.article.markdown_spans);
        let has_loaded_flag = shared_copy(&self.article.has_loaded_article);
        let cache = shared_copy(&self.cache);
        let link_indices = shared_copy(&self.article.link_span_indices);
        let is_valid_page = shared_copy(&self.article.is_valid_page);
        wikipedia::load_article_to_app(
            title.clone(),
            has_loaded_flag,
            markdown_spans,
            link_indices,
            is_valid_page,
            cache,
        );
    }

    pub fn view_selected_article_from_search(&mut self) {
        if let Some(title) = self.search.selected_search_result_title() {
            self.state = AppState::Article;
            self.set_article_page(title.clone());
            self.article.back_history.clear();
            self.article.forward_history.clear();
            self.article.back_history.push_back(title.clone());
            // self.article.history.push_back(title.clone());
        } else {
            self.state = AppState::SearchMenu;
        }
    }
    pub fn view_selected_article_from_selected_link(&mut self) {
        if let Some(title) = self.article.get_selected_link() {
            self.article.selected_link_index = 0;
            self.article.vertical_scroll = 0;
            let formatted_title = title.replace("_", " ").replace("./", "");
            self.set_article_page(formatted_title.clone());
            self.article.forward_history.clear();
            self.article.back_history.push_back(formatted_title.clone());
            // self.article.history.push_back(formatted_title.clone());
        }
    }

    fn load_page_from_history(&mut self) {
        if let Some(title) = self.article.back_history.back() {
            self.set_article_page(title.clone());
        }
    }

    pub fn go_to_previous_article(&mut self) {
        self.article.go_back_a_page();
        self.load_page_from_history();
    }

    pub fn go_to_next_article(&mut self) {
        self.article.go_forward_a_page();
        self.load_page_from_history();
    }
}
