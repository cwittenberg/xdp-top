// src/main.rs
mod app;
mod ui;

use anyhow::Result;
use app::{App, AppMode, Focus};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    io,
    process::Command,
    time::{Duration, Instant},
};
use ui::{draw_ui, is_inside};

fn check_os_dependencies() -> Result<(), String> {
    let deps = ["ip", "ethtool", "lspci"];
    let mut missing = Vec::new();

    for cmd in deps {
        match Command::new("which").arg(cmd).output() {
            Ok(output) if output.status.success() => {}
            _ => missing.push(cmd),
        }
    }

    if !missing.is_empty() {
        return Err(format!(
            "Missing required OS utilities: {}.\nPlease install them via your package manager (e.g., 'apt install iproute2 ethtool pciutils').",
            missing.join(", ")
        ));
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let tick_rate = Duration::from_millis(500);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    match app.mode {
                        AppMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => app.quit = true,
                                KeyCode::Right => app.next_nic(),
                                KeyCode::Left => app.prev_nic(),
                                KeyCode::Tab => {
                                    app.focus = match app.focus {
                                        None => Some(Focus::NicBtn),
                                        Some(Focus::NicBtn) => Some(Focus::ToggleBtn),
                                        Some(Focus::ToggleBtn) => Some(Focus::FilterBtn),
                                        Some(Focus::FilterBtn) => Some(Focus::AboutBtn),
                                        Some(Focus::AboutBtn) => Some(Focus::QuitBtn),
                                        Some(Focus::QuitBtn) => Some(Focus::NicBtn),
                                    };
                                }
                                KeyCode::BackTab => {
                                    app.focus = match app.focus {
                                        None => Some(Focus::QuitBtn),
                                        Some(Focus::NicBtn) => Some(Focus::QuitBtn),
                                        Some(Focus::ToggleBtn) => Some(Focus::NicBtn),
                                        Some(Focus::FilterBtn) => Some(Focus::ToggleBtn),
                                        Some(Focus::AboutBtn) => Some(Focus::FilterBtn),
                                        Some(Focus::QuitBtn) => Some(Focus::AboutBtn),
                                    };
                                }
                                KeyCode::Char('m') | KeyCode::Enter => {
                                    match app.focus {
                                        Some(Focus::NicBtn) | None => {
                                            app.mode = AppMode::NicMenu;
                                            app.menu_state.select(Some(app.selected_idx));
                                        },
                                        Some(Focus::ToggleBtn) => {
                                            app.show_throughput = !app.show_throughput;
                                        }
                                        Some(Focus::FilterBtn) => {
                                            app.filter_drv_only = !app.filter_drv_only;
                                        }
                                        Some(Focus::AboutBtn) => {
                                            app.mode = AppMode::About;
                                        }
                                        Some(Focus::QuitBtn) => {
                                            app.quit = true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        AppMode::NicMenu => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => {
                                    app.mode = AppMode::Normal;
                                }
                                KeyCode::Up => {
                                    app.hovered_nic_idx = None; 
                                    let i = match app.menu_state.selected() {
                                        Some(i) => if i == 0 { app.nics.len() - 1 } else { i - 1 },
                                        None => 0,
                                    };
                                    app.menu_state.select(Some(i));
                                }
                                KeyCode::Down => {
                                    app.hovered_nic_idx = None;
                                    let i = match app.menu_state.selected() {
                                        Some(i) => if i >= app.nics.len() - 1 { 0 } else { i + 1 },
                                        None => 0,
                                    };
                                    app.menu_state.select(Some(i));
                                }
                                KeyCode::Enter => {
                                    if let Some(i) = app.menu_state.selected() {
                                        app.select_nic(i);
                                    }
                                    app.mode = AppMode::Normal;
                                }
                                _ => {}
                            }
                        }
                        AppMode::About => {
                            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                                app.mode = AppMode::Normal;
                            }
                        }
                    }
                },
                Event::Mouse(mouse_event) => {
                    app.mouse_pos = (mouse_event.column, mouse_event.row);

                    if app.mode == AppMode::NicMenu {
                        if is_inside(app.mouse_pos, app.list_rect) {
                            let inner_y = app.mouse_pos.1.saturating_sub(app.list_rect.y + 1);
                            if inner_y < app.nics.len() as u16 {
                                app.hovered_nic_idx = Some(inner_y as usize);
                            } else {
                                app.hovered_nic_idx = None;
                            }
                        } else {
                            app.hovered_nic_idx = None;
                        }
                    }

                    if mouse_event.kind == MouseEventKind::Up(MouseButton::Left) {
                        match app.mode {
                            AppMode::Normal => {
                                app.focus = None; 
                                if is_inside(app.mouse_pos, app.btn_quit_rect) {
                                    app.quit = true;
                                } else if is_inside(app.mouse_pos, app.btn_about_rect) {
                                    app.mode = AppMode::About;
                                } else if is_inside(app.mouse_pos, app.btn_filter_rect) {
                                    app.filter_drv_only = !app.filter_drv_only;
                                } else if is_inside(app.mouse_pos, app.btn_toggle_rect) {
                                    app.show_throughput = !app.show_throughput;
                                } else if is_inside(app.mouse_pos, app.btn_nic_rect) {
                                    app.mode = AppMode::NicMenu;
                                    app.menu_state.select(Some(app.selected_idx));
                                }
                            }
                            AppMode::NicMenu => {
                                if let Some(idx) = app.hovered_nic_idx {
                                    app.select_nic(idx);
                                    app.mode = AppMode::Normal;
                                } else if !is_inside(app.mouse_pos, app.list_rect) {
                                    app.mode = AppMode::Normal;
                                }
                            }
                            AppMode::About => {
                                app.mode = AppMode::Normal;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_stats();
            last_tick = Instant::now();
        }

        if app.quit {
            return Ok(());
        }
    }
}

fn main() -> Result<()> {
    if let Err(e) = check_os_dependencies() {
        eprintln!("Startup Error:\n{}", e);
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app);

    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableMouseCapture);
    let _ = execute!(stdout, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();

    if let Err(err) = res {
        eprintln!("Error running xdp-top: {:?}", err);
    }

    Ok(())
}