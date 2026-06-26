mod animation;
mod app;
mod bindings;
mod events;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use animation::AnimationConfig;
use app::App;
use bindings::command_for_key;
use clap::Parser;
use color_eyre::eyre::Result;
use commitchi_core::DiffOptions;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use events::{AppEvent, EventSchedule};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;

const TICK_INTERVAL: Duration = Duration::from_millis(50);
const RENDER_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Parser)]
#[command(name = "commitchi")]
#[command(about = "Replay local Git history in a terminal time machine")]
struct Cli {
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,

    #[arg(long, default_value_t = 2_000)]
    large_diff_line_limit: usize,

    #[arg(long, default_value_t = 100)]
    large_diff_file_limit: usize,

    #[arg(long, default_value_t = 30.0, value_parser = parse_positive_f64)]
    lines_per_second: f64,

    #[arg(long, default_value_t = 1.0, value_parser = parse_positive_f64)]
    commits_per_second: f64,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let repo_path = match cli.repo {
        Some(path) => path,
        None => std::env::current_dir()?,
    };
    let app = App::load(
        repo_path,
        DiffOptions {
            line_limit: cli.large_diff_line_limit,
            file_limit: cli.large_diff_file_limit,
            ..DiffOptions::default()
        },
        AnimationConfig::new(cli.lines_per_second, cli.commits_per_second),
    )?;

    run(app)
}

fn run(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_result = run_loop(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);

    restore_result?;
    run_result
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut schedule = EventSchedule::new(TICK_INTERVAL, RENDER_INTERVAL);
    handle_app_event(terminal, app, AppEvent::Render)?;

    loop {
        if event::poll(schedule.poll_timeout(Instant::now()))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && handle_app_event(terminal, app, AppEvent::Input(key))?
                {
                    break;
                }
            }
        }

        for app_event in schedule.drain_due(Instant::now()) {
            if handle_app_event(terminal, app, app_event)? {
                return Ok(());
            }
        }
    }

    Ok(())
}

fn handle_app_event<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    app_event: AppEvent,
) -> Result<bool> {
    match app_event {
        AppEvent::Input(key) => {
            let command = command_for_key(key);
            app.apply_command(command).map_err(Into::into)
        }
        AppEvent::Tick(elapsed) => {
            app.tick(elapsed)?;
            Ok(false)
        }
        AppEvent::Render => {
            terminal.draw(|frame| ui::draw(frame, app))?;
            Ok(false)
        }
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn parse_positive_f64(value: &str) -> std::result::Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|err| format!("must be a number: {err}"))?;

    if parsed.is_finite() && parsed > 0.0 {
        Ok(parsed)
    } else {
        Err("must be greater than 0".to_owned())
    }
}
