use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, time::Duration};

mod api;
mod app;
mod auth;
mod countries;
mod login;
mod models;
mod tokens;
mod ui;
mod wireguard;

use api::ProtonClient;
use app::{App, InputMode, SplitFocus};
use auth::ProtonAuth;
use login::{run_login, show_authenticating, show_error, show_loading, LoginResult};
use tokens::{load_tokens, save_tokens, StoredTokens};

const POLL_INTERVAL_MS: u64 = 50;
const CANCEL_POLL_INTERVAL_MS: u64 = 50;

/// Authenticate using TUI login form
async fn authenticate_tui<B: Backend>(terminal: &mut Terminal<B>) -> Result<Option<StoredTokens>> {
    loop {
        // Show login form
        let result = run_login(terminal)?;

        match result {
            LoginResult::Cancel => return Ok(None),
            LoginResult::Submit { username, password } => {
                // Show authenticating status
                show_authenticating(terminal)?;

                // Perform authentication
                let auth = ProtonAuth::new()?;
                let auth_result = tokio::select! {
                    result = auth.authenticate(&username, &password) => Some(result),
                    _ = wait_for_cancel_key() => None,
                };

                match auth_result {
                    Some(Ok(auth_result)) => {
                        let tokens = StoredTokens::from(auth_result);
                        save_tokens(&tokens)?;
                        return Ok(Some(tokens));
                    }
                    Some(Err(e)) => {
                        let error_msg = e.to_string();
                        let short_error = if error_msg.len() > 40 {
                            format!("{}...", &error_msg[..40])
                        } else {
                            error_msg
                        };

                        if !show_error(terminal, &short_error)? {
                            return Ok(None);
                        }
                        // Loop back to login form for retry
                    }
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Get tokens - either from disk or by authenticating via TUI
async fn get_tokens<B: Backend>(terminal: &mut Terminal<B>) -> Result<Option<StoredTokens>> {
    // Try to load saved tokens first
    if let Some(tokens) = load_tokens()? {
        return Ok(Some(tokens));
    }

    // No saved tokens, need to authenticate
    authenticate_tui(terminal).await
}

async fn wait_for_cancel_key() -> io::Result<()> {
    loop {
        let maybe_key = tokio::task::spawn_blocking(|| -> io::Result<Option<KeyEvent>> {
            if event::poll(Duration::from_millis(CANCEL_POLL_INTERVAL_MS))? {
                if let Event::Key(key) = event::read()? {
                    return Ok(Some(key));
                }
            }
            Ok(None)
        })
        .await
        .map_err(io::Error::other)??;

        if let Some(key) = maybe_key {
            if key.code == KeyCode::Esc
                || (key.code == KeyCode::Char('c')
                    && key.modifiers.contains(event::KeyModifiers::CONTROL))
            {
                return Ok(());
            }
        }
    }
}

fn cleanup_terminal<B: Backend + io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

enum LoopAction {
    Continue,
    Exit,
}

async fn handle_normal_mode_key(app: &mut App, key: KeyEvent) -> io::Result<LoopAction> {
    match key.code {
        KeyCode::Char('q') => return Ok(LoopAction::Exit),
        KeyCode::Char('?') => {
            app.show_help_popup = true;
        }
        KeyCode::Char('v') => {
            app.toggle_split_view();
        }
        KeyCode::Tab => {
            if app.split_view {
                app.split_switch_focus();
            }
        }
        KeyCode::Char('s') => {
            if app.split_view {
                if let Some(server_idx) = app.get_selected_server_idx_in_split() {
                    let _ = app.create_config(server_idx).await;
                }
            } else {
                app.save_selected_config().await;
            }
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.split_view {
                app.split_next();
            } else {
                app.next();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.split_view {
                app.split_previous();
            } else {
                app.previous();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.split_view {
                if app.split_focus == SplitFocus::Servers {
                    app.split_focus = SplitFocus::Countries;
                }
            } else {
                app.collapse_selected();
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.split_view {
                if app.split_focus == SplitFocus::Countries {
                    app.split_focus = SplitFocus::Servers;
                }
            } else {
                app.expand_selected();
            }
        }
        KeyCode::PageDown => {
            if app.split_view {
                app.split_page_down();
            } else {
                app.page_down();
            }
        }
        KeyCode::PageUp => {
            if app.split_view {
                app.split_page_up();
            } else {
                app.page_up();
            }
        }
        KeyCode::Home | KeyCode::Char('g') => {
            if app.split_view {
                app.split_go_to_first();
            } else {
                app.go_to_first();
            }
        }
        KeyCode::End | KeyCode::Char('G') => {
            if app.split_view {
                app.split_go_to_last();
            } else {
                app.go_to_last();
            }
        }
        KeyCode::Enter => {
            if app.split_view {
                if app.split_focus == SplitFocus::Servers {
                    if let Some(server_idx) = app.get_selected_server_idx_in_split() {
                        if let Some(config_path) = app.create_config(server_idx).await {
                            if let Some(server) = app.all_servers.get(server_idx) {
                                app.start_wireguard(
                                    config_path.to_str().unwrap_or("wg0.conf"),
                                    server.name.clone(),
                                )
                                .await;
                            }
                        }
                    }
                } else {
                    // Switch to servers pane when Enter is pressed on country
                    app.split_focus = SplitFocus::Servers;
                }
            } else {
                app.connect_to_selected().await;
            }
        }
        KeyCode::Char(' ') => {
            if !app.split_view {
                app.toggle_current_selection();
            }
        }
        KeyCode::Esc => {
            if app.split_view {
                if !app.search_query.is_empty() {
                    // Clear search and restore full lists
                    app.search_query.clear();
                    app.search_cursor_position = 0;
                    app.update_split_view_for_search();
                } else {
                    // Exit split view
                    app.toggle_split_view();
                }
            } else if !app.search_query.is_empty() {
                app.search_query.clear();
                app.update_display_list();
            }
        }
        _ => {}
    }

    Ok(LoopAction::Continue)
}

async fn handle_search_mode_key(app: &mut App, key: KeyEvent) -> io::Result<LoopAction> {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            if app.search_cursor_position > 0 {
                app.search_query.remove(app.search_cursor_position - 1);
                app.search_cursor_position -= 1;
                if app.split_view {
                    app.update_split_view_for_search();
                } else {
                    app.update_display_list();
                }
            }
        }
        KeyCode::Left => {
            if app.search_cursor_position > 0 {
                app.search_cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            if app.search_cursor_position < app.search_query.len() {
                app.search_cursor_position += 1;
            }
        }
        KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.search_cursor_position = 0;
        }
        KeyCode::Char('e') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.search_cursor_position = app.search_query.len();
        }
        KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.search_query.drain(..app.search_cursor_position);
            app.search_cursor_position = 0;
            if app.split_view {
                app.update_split_view_for_search();
            } else {
                app.update_display_list();
            }
        }
        KeyCode::Char('k') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.search_query.drain(app.search_cursor_position..);
            if app.split_view {
                app.update_split_view_for_search();
            } else {
                app.update_display_list();
            }
        }
        KeyCode::Char('w') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            if app.search_cursor_position > 0 {
                let mut start_index = app.search_cursor_position;
                let chars: Vec<char> = app.search_query.chars().collect();

                // Skip trailing spaces if any
                while start_index > 0 && chars[start_index - 1].is_whitespace() {
                    start_index -= 1;
                }
                // Skip non-spaces (the word)
                while start_index > 0 && !chars[start_index - 1].is_whitespace() {
                    start_index -= 1;
                }

                app.search_query
                    .drain(start_index..app.search_cursor_position);
                app.search_cursor_position = start_index;
                if app.split_view {
                    app.update_split_view_for_search();
                } else {
                    app.update_display_list();
                }
            }
        }
        KeyCode::Char(c)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            app.search_query.insert(app.search_cursor_position, c);
            app.search_cursor_position += 1;
            if app.split_view {
                app.update_split_view_for_search();
            } else {
                app.update_display_list();
            }
        }
        _ => {}
    }

    Ok(LoopAction::Continue)
}

#[tokio::main]
async fn main() -> Result<()> {
    // TUI Setup (early, for login screen)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get authentication tokens
    let tokens = match get_tokens(&mut terminal).await? {
        Some(t) => t,
        None => {
            // User cancelled - cleanup and exit
            cleanup_terminal(&mut terminal)?;
            return Ok(());
        }
    };

    let client = ProtonClient::new(tokens.uid, tokens.access_token);

    // Prevent Ctrl+C from killing the app (useful when running sudo subprocess)
    tokio::spawn(async move {
        loop {
            let _ = tokio::signal::ctrl_c().await;
        }
    });

    // Show loading screen while fetching servers
    show_loading(&mut terminal, "Loading servers...")?;

    let servers_result = tokio::select! {
        result = client.get_logical_servers() => Some(result),
        _ = wait_for_cancel_key() => None,
    };

    let mut servers = match servers_result {
        Some(Ok(s)) => s,
        Some(Err(e)) => {
            // If we get an auth error, delete saved tokens
            let err_str = e.to_string();
            if err_str.contains("401")
                || err_str.contains("Unauthorized")
                || err_str.contains("Invalid")
            {
                tokens::delete_tokens()?;
                // Show error and offer to retry
                if show_error(&mut terminal, "Session expired. Press Enter to login again")? {
                    // Restart auth flow
                    match authenticate_tui(&mut terminal).await? {
                        Some(new_tokens) => {
                            let new_client =
                                ProtonClient::new(new_tokens.uid, new_tokens.access_token);
                            new_client.get_logical_servers().await?
                        }
                        None => {
                            cleanup_terminal(&mut terminal)?;
                            return Ok(());
                        }
                    }
                } else {
                    cleanup_terminal(&mut terminal)?;
                    return Ok(());
                }
            } else {
                cleanup_terminal(&mut terminal)?;
                return Err(e);
            }
        }
        None => {
            cleanup_terminal(&mut terminal)?;
            return Ok(());
        }
    };

    // Sort by country
    servers.sort_by(|a, b| {
        let name_a = countries::get_country_name(&a.exit_country);
        let name_b = countries::get_country_name(&b.exit_country);
        name_a.cmp(&name_b).then(a.name.cmp(&b.name))
    });

    let app = App::new(client, servers);
    let res = run_app(&mut terminal, app).await;

    // TUI Cleanup
    cleanup_terminal(&mut terminal)?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        if app.should_redraw {
            terminal.clear()?;
            app.should_redraw = false;
        }

        // Update stats if connected (polling every frame)
        if app.connection_status.is_some() {
            app.update_traffic_stats();
        }

        terminal.draw(|f| ui::ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(POLL_INTERVAL_MS))? {
            if let Event::Key(key) = event::read()? {
                // Global Ctrl+C handler
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(event::KeyModifiers::CONTROL)
                {
                    app.input_mode = InputMode::Normal;
                    continue;
                }

                // Help Popup Handling (any key closes it)
                if app.show_help_popup {
                    app.show_help_popup = false;
                    continue;
                }

                // Connection Popup Handling
                if app.show_connection_popup {
                    match key.code {
                        KeyCode::Char('d') => {
                            app.stop_wireguard().await;
                        }
                        KeyCode::Esc => {
                            app.show_connection_popup = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                let action = match app.input_mode {
                    InputMode::Normal => handle_normal_mode_key(&mut app, key).await?,
                    InputMode::Search => handle_search_mode_key(&mut app, key).await?,
                };

                if let LoopAction::Exit = action {
                    return Ok(());
                }
            }
        }
    }
}
