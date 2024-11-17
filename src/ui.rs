use std::ops::Deref;
use std::sync::{MutexGuard, TryLockError, TryLockResult};

use crate::app::{ActionItem, ActionMenu, App, AppState, MenuState, TypeableState};
use crate::parsing::FormattedSpan;
use crate::styles::Theme;
use crate::utils::{wrapped_iter_enumerate, WIK_TITLE};
use crate::widgets::ScrollBar;
use crate::wikipedia::SearchResult;
use digest::typenum::Mod;
use tui::layout::Rect;
use tui::style::Modifier;
use tui::widgets::{BorderType, Widget};
// use crate::widgets::ScrollBar;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use substring::Substring;

pub fn draw<'a, B: Backend>(frame: &mut Frame<B>, app: &App) {
    let window_area = frame.size();
    frame.render_widget(
        Block::default().style(app.theme.window_background()),
        window_area,
    );
    match app.state {
        AppState::Title => draw_title(frame, app),
        AppState::Search => draw_search(frame, app),
        AppState::SearchMenu => draw_menu(frame, app, &app.search_menu),
        AppState::Credit => draw_credit(frame, app),
        AppState::Article => draw_article(frame, app),
        AppState::ArticleMenu => draw_menu(frame, app, &app.article_menu),
        _ => draw_search(frame, app),
    }
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

fn search_box_widget<'a>(
    app: &'a App,
    typeable: &'a impl TypeableState,
    title: String,
) -> Paragraph<'a> {
    let mut input_text = typeable.get_input().to_owned();
    input_text.push(' ');
    let pre_highlight = input_text.substring(0, typeable.get_cursor_pos());
    let highlight_char =
        input_text.substring(typeable.get_cursor_pos(), typeable.get_cursor_pos() + 1);
    let post_highlight =
        input_text.substring(typeable.get_cursor_pos() + 1, typeable.get_input().len());
    let text_block = if title.len() > 0 {
        Block::default()
            .borders(Borders::ALL)
            .title(title.to_owned())
    } else {
        Block::default().borders(Borders::ALL)
    };
    Paragraph::new(vec![Spans::from(vec![
        Span::raw(pre_highlight.to_owned()),
        Span::styled(highlight_char.to_owned(), app.theme.cursor_style()),
        Span::raw(post_highlight.to_owned()),
    ])])
    .block(text_block)
}

fn draw_search<'a, B: Backend>(frame: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(frame.size());

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

    /*
    let mut input_text = app.search.input.to_owned();
    input_text.push(' ');
    let pre_highlight = input_text.substring(0, app.search.cursor_pos);
    let highlight_char = input_text.substring(app.search.cursor_pos, app.search.cursor_pos + 1);
    let post_highlight = input_text.substring(app.search.cursor_pos + 1, input_text.len());
    let input = Paragraph::new(vec![Spans::from(vec![
        Span::raw(pre_highlight),
        Span::styled(highlight_char, app.theme.cursor_style()),
        Span::raw(post_highlight),
    ])])
    .style(text_block_style)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search Wikipedia"),
    );
    */

    let input_widget = search_box_widget(&app, &app.search, String::from("Search Wikipedia"))
        .style(text_block_style);
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
            let all_spans: Vec<Spans> = wrapped_iter_enumerate(&results, app.search.selected_index)
                .flat_map(|(index, search_result)| -> Vec<Spans<'_>> {
                    let title_style = if index == selected_index {
                        app.theme.highlighted_title_style()
                    } else {
                        app.theme.unhighlighted_title_style()
                    };
                    let title_span = Span::styled(
                        format!(
                            "{} - {}",
                            search_result.title.clone(),
                            search_result.pageid.clone()
                        ),
                        title_style,
                    );
                    if index == selected_index {
                        vec![
                            Spans::from(vec![title_span]),
                            SearchResult::highlighted_snippets(&search_result, &app.theme),
                            Spans::from(vec![Span::raw("")]),
                        ]
                    } else {
                        vec![Spans::from(vec![title_span])]
                    }
                })
                .collect(); // Collect spans into a Vec<Spans>

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
) -> Vec<Spans<'a>> {
    action_items
        .iter()
        .enumerate()
        .map(|(option_index, option)| -> Spans {
            let style = if option_index == selected_index {
                theme.selected_option()
            } else {
                theme.unselected_option()
            };
            Spans::from(Span::styled(option.label(), style))
        })
        .collect()
}

fn draw_menu<'a, B: Backend>(frame: &mut Frame<B>, app: &App, menu: &MenuState) {
    let menu_items = create_option_spans(menu.get_options(), menu.get_index(), &app.theme);

    let area = centered_rect(50, 50, frame.size());
    frame.render_widget(
        Paragraph::new(menu_items)
            .style(app.theme.block_border_focus())
            .block(Block::default().borders(Borders::ALL).title("Menu"))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_credit<B: Backend>(frame: &mut Frame<'_, B>, app: &App) {
    let area = centered_rect(50, 50, frame.size());

    let mut credit_paragraph_text = vec![Spans::from("Made by Mazza :)")];

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

fn draw_title<B: Backend>(frame: &mut Frame<'_, B>, app: &App) {
    let full_area = centered_rect_by_lengths(40, 11, frame.size());

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
            .alignment(Alignment::Center),
        title_areas[0],
    );

    let input_widget = search_box_widget(&app, &app.title, String::from("Search..."));
    frame.render_widget(input_widget, title_areas[1]);
}

fn draw_article<B: Backend>(frame: &mut Frame<'_, B>, app: &App) {
    let article_content: Vec<Spans> = match app.article.is_loading_article.try_lock() {
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

                vecs_of_formatted_spans
                    .iter()
                    .enumerate()
                    .map(|(_, formatted_spans)| -> Spans {
                        Spans::from(
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
                                    } else if let Some(link) = &formatted_span.link {
                                        Span::styled(
                                            formatted_span.text.clone(),
                                            Style::default().add_modifier(Modifier::UNDERLINED),
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

            true => vec![Spans::from(vec![Span::raw("Loading...")])],
        },
        Err(_) => vec![Spans::from(vec![Span::raw("Error loading page...")])],
    };
    frame.render_widget(
        Paragraph::new(article_content)
            .style(app.theme.block_border_focus())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.article.article_name.clone()),
            )
            .wrap(Wrap { trim: true }),
        frame.size(),
    );
}
