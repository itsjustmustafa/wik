use std::ops::Deref;
use std::sync::{MutexGuard, TryLockError, TryLockResult};

use crate::app::{ActionItem, ActionMenu, App, AppState, MenuState, TypeableState};
use crate::parsing::FormattedSpan;
use crate::styles::Theme;
use crate::utils::{wrapped_iter_enumerate, WIK_TITLE};
use crate::widgets::{AlphaBox, Eraser, ScrollBar, TextBox};
use crate::wikipedia::SearchResult;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::Line;
// use crate::widgets::ScrollBar;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// use substring::Substring;

pub fn draw(frame: &mut Frame, app: &App) {
    let window_area = frame.area();
    frame.render_widget(
        Block::default().style(app.theme.window_background()),
        window_area,
    );
    match app.state {
        AppState::Title => draw_title(frame, app),
        AppState::Search => draw_search(frame, app),
        AppState::SearchMenu => draw_search_menu(frame, app),
        AppState::Credit => draw_credit(frame, app),
        AppState::Article => draw_article(frame, app),
        AppState::ArticleMenu => draw_article_menu(frame, app),
        AppState::ThemeMenu => draw_theme_selection(frame, app),
        // _ => draw_search(frame, app),
    }
}

fn draw_article_menu(frame: &mut Frame, app: &App) {
    draw_article(frame, app);
    frame.render_widget(AlphaBox::new(Color::DarkGray, 50), frame.area());
    draw_menu(frame, app, &app.article_menu);
}

fn draw_search_menu(frame: &mut Frame, app: &App) {
    draw_search(frame, app);
    frame.render_widget(AlphaBox::new(Color::DarkGray, 50), frame.area());
    draw_menu(frame, app, &app.search_menu);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(vertical_layout[1])[1]
}

fn centered_rect_by_lengths(length_x: u16, length_y: u16, r: Rect) -> Rect {
    let full_height = r.height;
    let full_width = r.width;

    let length_x = length_x.min(full_width);
    let length_y = length_y.min(full_height);
    let outer_x = (full_width - length_x) / 2;
    let outer_y = (full_height - length_y) / 2;

    // length_x = full_width - 2 * outer_x;
    // length_y = full_height - 2 * outer_y;

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(outer_y),
                Constraint::Length(length_y),
                Constraint::Length(outer_y),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(outer_x),
                Constraint::Length(length_x),
                Constraint::Length(outer_x),
            ]
            .as_ref(),
        )
        .split(vertical_layout[1])[1]
}

pub fn draw_search(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(app.config.margin.into())
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(frame.area());

    // Search input box
    let text_box_is_highlighted = app.search.text_box_is_highlighted;
    let text_block_style = match text_box_is_highlighted {
        true => app.theme.block_border_focus(),
        false => app.theme.block_border_unfocus(),
    };

    let result_block_style = match text_box_is_highlighted {
        true => app.theme.block_border_unfocus(),
        false => app.theme.block_border_focus(),
    };
    let input_widget = TextBox::new(app.search.get_input(), app.search.get_cursor_pos())
        .cursor_style(app.theme.cursor_style())
        .text_style(text_block_style);
    frame.render_widget(input_widget, chunks[0]);

    let mut is_loading = false;
    if let Ok(is_loading_guard) = app.search.is_loading_query.try_lock() {
        if *is_loading_guard {
            is_loading = true;
        }
    }

    let mut available_results: TryLockResult<MutexGuard<'_, Vec<SearchResult>>> =
        Err(TryLockError::WouldBlock);

    if !is_loading {
        available_results = app.search.results.try_lock();
    }

    match available_results {
        Ok(results) => {
            // Collect spans into a Vec<Spans>
            // let results = result_guard;
            let selected_index = app.search.selected_index;
            let all_spans: Vec<Line> = wrapped_iter_enumerate(&results, app.search.selected_index)
                .flat_map(|(index, search_result)| -> Vec<Line> {
                    let title_style = if index == selected_index {
                        app.theme.highlighted_title_style()
                    } else {
                        app.theme.unhighlighted_title_style()
                    };
                    let title_span = Span::styled(
                        // format!(
                        //     "{} - {}",
                        //     search_result.title.clone(),
                        //     search_result.pageid.clone()
                        // ),
                        search_result.title.clone(),
                        title_style,
                    );
                    if index == selected_index {
                        vec![
                            Line::from(vec![title_span]),
                            Line::from(SearchResult::highlighted_snippets(
                                &search_result,
                                &app.theme,
                            )),
                            Line::from(vec![Span::raw("")]),
                        ]
                    } else {
                        vec![Line::from(vec![title_span])]
                    }
                })
                .collect(); // Collect spans into a Vec<Line>

            let result_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                .split(chunks[1]);

            // Render the results
            frame.render_widget(
                Paragraph::new(all_spans)
                    .style(result_block_style)
                    .block(Block::default().borders(Borders::ALL).title("Results"))
                    .wrap(Wrap { trim: true }),
                result_chunks[0],
            );

            let scroll_bar = ScrollBar::new(
                result_chunks[1].height as usize,
                app.search.selected_index,
                results.len(),
            )
            .bar_style(Style::default().fg(app.theme.secondary))
            .handle_style(Style::default().fg(app.theme.tertiary));
            frame.render_widget(scroll_bar, result_chunks[1]);
        }
        Err(e) => {
            let waiting_message = match e {
                TryLockError::Poisoned(_) => "Errored!",
                TryLockError::WouldBlock => "Loading...",
            };
            frame.render_widget(
                Paragraph::new(Span::styled(waiting_message, app.theme.loading()))
                    .style(result_block_style)
                    .block(Block::default().borders(Borders::ALL).title("Results")),
                chunks[1],
            );
        }
    }
}

fn create_option_spans<'a>(
    action_items: &'a Vec<ActionItem>,
    selected_index: usize,
    theme: &'a Theme,
) -> Vec<Line<'a>> {
    action_items
        .iter()
        .enumerate()
        .map(|(option_index, option)| -> Line {
            let style = if option_index == selected_index {
                theme.selected_option()
            } else {
                theme.unselected_option()
            };
            Line::from(Span::styled(option.label(), style))
        })
        .collect()
}

fn draw_menu(frame: &mut Frame, app: &App, menu: &MenuState) {
    let menu_items = create_option_spans(menu.get_options(), menu.get_index(), &app.theme);

    let area = centered_rect(50, 50, frame.area());
    frame.render_widget(Eraser {}, area);
    frame.render_widget(
        Paragraph::new(menu_items)
            .style(app.theme.block_border_focus())
            .block(Block::default().borders(Borders::ALL).title("Menu"))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_credit(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, frame.area());

    let mut credit_paragraph_text = vec![Line::from("Made by Mazza :)")];

    credit_paragraph_text.append(&mut create_option_spans(
        &app.credit.options,
        app.credit.selected_index,
        &app.theme,
    ));

    frame.render_widget(
        Paragraph::new(credit_paragraph_text)
            .style(app.theme.block_border_focus())
            .block(Block::default().borders(Borders::ALL).title("Credit"))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_theme_selection(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, frame.area());

    let mut credit_paragraph_text = vec![];

    credit_paragraph_text.append(&mut create_option_spans(
        &app.theme_menu.get_options(),
        app.theme_menu.get_index(),
        &app.theme,
    ));

    frame.render_widget(
        Paragraph::new(credit_paragraph_text)
            .style(app.theme.block_border_focus())
            .block(Block::default().borders(Borders::ALL).title("Themes"))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_title(frame: &mut Frame, app: &App) {
    let full_area = centered_rect_by_lengths(40, 11, frame.area());

    let title_areas = Layout::default()
        .constraints(vec![Constraint::Min(0), Constraint::Length(3)])
        .direction(Direction::Vertical)
        .split(full_area);

    frame.render_widget(
        Paragraph::new(WIK_TITLE)
            // .block(
            //     Block::default()
            //         .borders(Borders::ALL)
            //         .border_type(BorderType::Double),
            // )
            .style(Style::default().fg(app.theme.text))
            .alignment(Alignment::Center),
        title_areas[0],
    );

    // let input_widget = search_box_widget(&app, &app.title, String::from("Search..."))
    //     .style(app.theme.block_border_focus());
    let input_widget = TextBox::new(app.title.get_input(), app.title.get_cursor_pos())
        .cursor_style(app.theme.cursor_style())
        .text_style(app.theme.block_border_focus());

    frame.render_widget(input_widget, title_areas[1]);
}

fn draw_article(frame: &mut Frame, app: &App) {
    let article_content: Vec<Line> = match app.article.is_loading_article.try_lock() {
        Ok(loading_result) => match *loading_result {
            false => {
                let vecs_of_formatted_spans = app
                    .article
                    .markdown_spans
                    .lock()
                    .unwrap()
                    .deref()
                    .split(|formatted_span| formatted_span.is_break)
                    .map(|slice| -> Vec<FormattedSpan> { slice.to_vec() })
                    .collect::<Vec<Vec<FormattedSpan>>>();

                let link_span_indices = app.article.link_span_indices.lock().unwrap().clone();

                let selected_index = link_span_indices
                    .get(app.article.selected_link_index)
                    .unwrap_or(&0);

                vecs_of_formatted_spans
                    .iter()
                    .enumerate()
                    .map(|(_, formatted_spans)| -> Line {
                        Line::from(
                            formatted_spans
                                .iter()
                                .enumerate()
                                .map(|(_, formatted_span)| -> Span {
                                    if formatted_span.is_heading {
                                        Span::styled(
                                            formatted_span.text.clone(),
                                            if formatted_span.heading_level > 2 {
                                                Style::default().add_modifier(Modifier::BOLD)
                                            } else {
                                                Style::default()
                                                    .add_modifier(Modifier::BOLD)
                                                    .add_modifier(Modifier::ITALIC)
                                            },
                                        )
                                    } else if let Some(_link) = &formatted_span.link {
                                        Span::styled(
                                            formatted_span.text.clone(),
                                            if selected_index.eq(&formatted_span.index) {
                                                app.theme
                                                    .highlighted_snippet_style()
                                                    .add_modifier(Modifier::UNDERLINED)
                                            } else {
                                                app.theme
                                                    .unhighlighted_snippet_style()
                                                    .add_modifier(Modifier::UNDERLINED)
                                            },
                                        )
                                    } else {
                                        Span::raw(formatted_span.text.clone())
                                    }
                                })
                                .collect::<Vec<Span>>(),
                        )
                    })
                    .collect()
            }

            true => vec![Line::from(vec![Span::raw("Loading...")])],
        },
        Err(_) => vec![Line::from(vec![Span::raw("Error loading page...")])],
    };
    frame.render_widget(
        Paragraph::new(article_content)
            .style(app.theme.block_border_focus())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.article.article_name.clone()),
            )
            .wrap(Wrap { trim: true })
            .scroll((app.article.vertical_scroll as u16, 0)),
        frame.area(),
    );
}
