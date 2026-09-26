pub mod app;
pub mod ui;

pub use app::App;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

/// Run the interactive Terminal UI dashboard connecting to the specified NexusLB Admin API URL
pub async fn run_dashboard(admin_url: String) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(admin_url);
    // Initial poll
    app.poll_metrics().await;

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Dashboard error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut poll_interval = tokio::time::interval(Duration::from_millis(500));

    while app.is_running {
        terminal.draw(|f| ui::render(f, app))?;

        tokio::select! {
            _ = poll_interval.tick() => {
                app.poll_metrics().await;
            }
            _ = async {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.is_running = false;
                                }
                                KeyCode::Char('r') => {
                                    app.trigger_reload().await;
                                }
                                KeyCode::Char('d') => {
                                    app.toggle_drain().await;
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.previous_backend();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.next_backend();
                                }
                                KeyCode::Char(' ') => {
                                    app.poll_metrics().await;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } => {}
        }
    }

    Ok(())
}
