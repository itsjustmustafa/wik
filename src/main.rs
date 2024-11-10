mod app;
mod caching;
mod styles;
mod ui;
mod wikipedia;

use crate::app::App;
use caching::CachingSession;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};
use std::{io, sync::Mutex};
use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break, // Exit on Esc
                    KeyCode::Char(c) => {
                        // app.input.push(c); // Append character to input
                        let cursor_pos = app.cursor_pos;
                        if !(cursor_pos > app.input.len()) {
                            app.input.insert(cursor_pos, c);
                            app.move_cursor_one_step(1);
                        }
                    }
                    KeyCode::Backspace => {
                        if !app.input.is_empty() {
                            let cursor_pos = app.cursor_pos;
                            if cursor_pos > 0 {
                                app.input.remove(cursor_pos - 1); // Remove character before cursor
                                app.move_cursor_one_step(-1);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // let mut app = app.lock().unwrap();
                        // Spawn a new thread to perform search and update results

                        if app.input.len() > 0 {
                            if app.is_this_lockable() {
                                // let app_clone = Arc::clone(&app).lock().unwrap();
                                // app_clone.start_search();
                                *app.is_loading_query.lock().unwrap() = true;
                                let loading_flag = Arc::clone(&app.is_loading_query);
                                let app_results = Arc::clone(&app.results);
                                let caching_session = Arc::clone(&app.cache);
                                let input = app.input.clone();
                                thread::spawn(move || {
                                    if let Ok(results) =
                                        wikipedia::get_wikipedia_query(input.as_str())
                                    {
                                        *app_results.lock().unwrap() = results;
                                        *loading_flag.lock().unwrap() = false;
                                    }
                                });
                            }
                        }
                    }
                    KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.move_cursor_to_start();
                    }
                    KeyCode::Left => {
                        app.move_cursor_one_step(-1);
                    }
                    KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.move_cursor_to_end();
                    }
                    KeyCode::Right => {
                        app.move_cursor_one_step(1);
                    }
                    KeyCode::Up => {
                        if app.is_this_lockable() {
                            if app.results.lock().unwrap().len() > 0 {
                                app.scroll_results(-1);
                            }
                        }
                    }
                    KeyCode::Down => {
                        if app.is_this_lockable() {
                            if app.results.lock().unwrap().len() > 0 {
                                app.scroll_results(1);
                            }
                        }
                    }

                    _ => {}
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

    // let session = CachingSession::new();
    // println!("{:?}", session.get_cache_file_path("filename.json"));
    // CachingSession::clear_caches()?;
    // std::fs::create_dir_all(session.get_cache_file_path("filename.json"))?;
    Ok(())
}
