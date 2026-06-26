mod app;
mod bindings;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

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
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;

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
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let command = command_for_key(key);
                    if app.apply_command(command)? {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
