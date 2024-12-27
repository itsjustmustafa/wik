mod app;
mod caching;
mod parsing;
mod styles;
mod ui;
mod utils;
mod widgets;
mod wikipedia;

use app::{ActionMenu, App, AppState, ScrollDirection, TypeableState};
use caching::CachingSession;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use dialoguer::Input;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;
use std::{error::Error, time::Duration};
use utils::clargs::{load_arg_from_config, save_arg_to_file, Args};

const APP_REFRESH_TIME_MILLIS: u64 = 16;
// const APP_DEFAULT_MARGIN: u16 = 2;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = Args::parse();

    let mut app = App::new();
    app.is_running = true;

    // Check if the user has saved configurations
    if let Some(loaded_args) = load_arg_from_config() {
        // If no args were provided, then use the saved args
        if args.is_default_configs() {
            args.load_from(loaded_args);
        } else {
            if ask_yes_or_no("Save your config to file?") {
                save_arg_to_file(&args)?;
            }
        }
    } else {
        if ask_yes_or_no("Create a config file?") {
            save_arg_to_file(&args)?;
        }
    }

    // Setup terminal
    let mut fixed_size = false;
    let mut size = size()?;
    // let mut margin: u16 = APP_DEFAULT_MARGIN;

    if let Some(cols) = args.cols {
        size.0 = cols;
        fixed_size = true;
    }
    if let Some(rows) = args.rows {
        size.1 = rows;
        fixed_size = true;
    }

    // Backup in case user has not provided args but size is invalid:
    if size.0 < 1 || size.1 < 1 {
        fixed_size = true;
        size = prompt_for_size()?;
        args.cols = Some(size.0);
        args.rows = Some(size.1);
        args.margin = get_dimension("margin size");
    }

    if let Some(query) = args.search {
        app.search_and_load(query.clone());
    }

    if let Some(title) = args.page {
        // try getting user requested page
        // app.search.input = title.clone();
        app.try_getting_page(title.clone());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let area = Rect::new(0, 0, size.0, size.1);
    let mut terminal = match fixed_size {
        true => Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(area),
            },
        )?,
        false => Terminal::new(backend)?,
    };

    /*
    app.theme = Theme::from_hex_string_series(
        String::from("light"),
        String::from("e8ebed-1f1f1f-89579b-2e4499-ffb0b2-404040"),
    );
    */

    // Main loop
    loop {
        if !app.is_running {
            break;
        }
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(APP_REFRESH_TIME_MILLIS))? {
            if let Event::Key(key) = event::read()? {
                match app.state {
                    AppState::Title => match key.code {
                        // MARK: - Title State
                        KeyCode::Enter => {
                            // app.state = AppState::Search;
                            app.search_and_load(app.title.input.clone());
                            // app.search.input = app.title.input.clone();
                            // app.load_wikipedia_search_query();
                        }
                        KeyCode::Esc => {
                            app.is_running = false;
                        }
                        _ => {
                            app.title.handle_key(key);
                        }
                    },
                    AppState::Search => match key.code {
                        // MARK: - Search State
                        KeyCode::Esc => {
                            // Enter Escape menu, from where one can exit normally
                            app.state = AppState::SearchMenu;
                        }
                        KeyCode::F(1) => {
                            // Just-in-case exit
                            app.is_running = false
                        }
                        // MARK: - the josh mann bookmark
                        KeyCode::Enter => {
                            if app.search.text_box_is_highlighted {
                                app.load_wikipedia_search_query();
                            } else {
                                app.view_selected_article_from_search();
                            }
                        }
                        KeyCode::Up => {
                            app.search.scroll_results(ScrollDirection::UP);
                        }
                        KeyCode::Down => {
                            app.search.scroll_results(ScrollDirection::DOWN);
                        }

                        _ => {
                            app.search.handle_key(key);
                        }
                    },
                    AppState::SearchMenu => match key.code {
                        // MARK: - Search Menu State
                        KeyCode::Esc => {
                            app.state = AppState::Search;
                        }

                        KeyCode::Up => {
                            app.search_menu.scroll(ScrollDirection::UP);
                        }

                        KeyCode::Down => {
                            app.search_menu.scroll(ScrollDirection::DOWN);
                        }

                        KeyCode::Enter => {
                            app.search_menu.get_selected_action()(&mut app);
                        }

                        KeyCode::F(1) => {
                            // Just-in-case exit
                            app.is_running = false;
                        }
                        _ => {}
                    },
                    AppState::Credit => match key.code {
                        // MARK: - Credit State
                        KeyCode::Esc => {
                            app.state = AppState::SearchMenu;
                        }

                        KeyCode::Up => {
                            app.credit.scroll(ScrollDirection::UP);
                        }
                        KeyCode::Down => {
                            app.credit.scroll(ScrollDirection::DOWN);
                        }

                        KeyCode::Enter => {
                            app.credit.get_selected_action()(&mut app);
                        }

                        _ => {}
                    },
                    AppState::Article => match key.code {
                        // MARK: - Article State
                        KeyCode::Esc => {
                            app.state = AppState::ArticleMenu;
                        }
                        KeyCode::Left => {
                            app.article.scroll_link(ScrollDirection::UP);
                        }
                        KeyCode::Right => {
                            app.article.scroll_link(ScrollDirection::DOWN);
                        }
                        KeyCode::Up => {
                            app.article.scroll_vertically(ScrollDirection::UP);
                        }
                        KeyCode::Down => {
                            app.article.scroll_vertically(ScrollDirection::DOWN);
                        }
                        KeyCode::Enter => {
                            app.view_selected_article_from_selected_link();
                        }
                        _ => {}
                    },
                    AppState::ArticleMenu => match key.code {
                        // MARK: - Article Menu State
                        KeyCode::Esc => {
                            app.state = AppState::Article;
                        }
                        KeyCode::Up => {
                            app.article_menu.scroll(ScrollDirection::UP);
                        }

                        KeyCode::Down => {
                            app.article_menu.scroll(ScrollDirection::DOWN);
                        }

                        KeyCode::Enter => {
                            app.article_menu.get_selected_action()(&mut app);
                        }
                        _ => {}
                    },
                    AppState::ThemeMenu => match key.code {
                        // MARK: - Theme State
                        KeyCode::Enter => {
                            app.theme_menu.get_selected_action()(&mut app);
                        }
                        KeyCode::Esc => {
                            app.state = AppState::Search;
                            // app.article.selected_link_index += 1;
                        }
                        // KeyCode::Left => {}
                        _ => app.theme_menu.handle_key(key),
                    }, // _ => app.is_running = false,
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    CachingSession::clear_caches()?;

    Ok(())
}

fn prompt_for_size() -> Result<(u16, u16), std::io::Error> {
    eprintln!("Unable to automatically determine console dimensions.");
    let width = get_dimension("columns");
    let height = get_dimension("rows");
    return Ok((width, height));
}

fn yes_or_no(input: &str) -> Option<bool> {
    match input.trim().to_lowercase().as_str() {
        "yes" | "y" | "true" | "1" => Some(true),
        "no" | "n" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn ask_yes_or_no(question: &str) -> bool {
    loop {
        let input: String = Input::new()
            .with_prompt(format!("{} (Y/N)", question))
            .interact_text()
            .unwrap();
        if let Some(response) = yes_or_no(input.as_str()) {
            return response;
        } else {
            eprintln!("Invalid response, needs to be a Y or N");
            continue;
        }
    }
}

fn get_dimension(dimension_name: &str) -> u16 {
    loop {
        let input: String = Input::new()
            .with_prompt(format!("Enter {}", dimension_name))
            .interact_text()
            .unwrap();
        match input.as_str().parse::<u16>() {
            Ok(dimension) => return dimension,
            Err(_e) => {
                eprintln!("Invalid input, please enter a positive integer.");
                continue;
            }
        };
    }
}
