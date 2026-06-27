mod animation;
mod app;
mod bindings;
mod config;
mod events;
mod hooks;
mod sprite;
mod ui;
mod watch;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use app::App;
use bindings::command_for_key;
use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::Result;
use commitchi_core::RepoHandle;
use commitchi_pet::{now_seconds, repo_id_from_path, ActivityRecord, PetScope, PetStateFiles};
use config::{ConfigOverrides, RuntimeConfig};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use events::{AppEvent, EventSchedule};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use watch::StateWatcher;

const TICK_INTERVAL: Duration = Duration::from_millis(50);
const RENDER_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug, Parser)]
#[command(name = "commitchi")]
#[command(version)]
#[command(about = "Replay local Git history in an animated terminal time machine")]
#[command(
    long_about = "Commitchi replays local Git history as an animated terminal diff timeline and keeps a small persistent pet companion. It reads local Git metadata only, runs offline, and can install a Git post-commit hook for pet mood persistence."
)]
#[command(
    after_help = "Config: Commitchi reads commitchi.toml from the repository root by default. Use --config <FILE> to choose another file. CLI flags override config values."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Git repository path to open or update"
    )]
    repo: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Read settings from a TOML config file"
    )]
    config: Option<PathBuf>,

    #[command(flatten)]
    run: RunArgs,

    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Args)]
struct RunArgs {
    #[arg(
        long,
        value_name = "LINES",
        value_parser = parse_positive_usize,
        help = "Maximum diff lines to load per commit"
    )]
    large_diff_line_limit: Option<usize>,

    #[arg(
        long,
        value_name = "FILES",
        value_parser = parse_positive_usize,
        help = "Maximum changed files to load per commit"
    )]
    large_diff_file_limit: Option<usize>,

    #[arg(
        long,
        value_name = "LINES",
        value_parser = parse_positive_f64,
        help = "Diff reveal speed in lines per second"
    )]
    lines_per_second: Option<f64>,

    #[arg(
        long,
        value_name = "COMMITS",
        value_parser = parse_positive_f64,
        help = "Playback speed in commits per second"
    )]
    commits_per_second: Option<f64>,

    #[arg(
        long,
        value_name = "SCOPE",
        help = "Pet state display scope: repo, global, or both"
    )]
    pet_scope: Option<PetScope>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(about = "Commands intended for Git hook integration")]
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },

    #[command(about = "Install or update the managed Git post-commit hook")]
    InstallHook(InstallHookArgs),

    #[command(
        hide = true,
        about = "Print every pet expression to stdout for previewing"
    )]
    PetDemo,
}

#[derive(Debug, Subcommand)]
enum HookCommand {
    #[command(about = "Record the current HEAD commit in pet state")]
    PostCommit(HookPostCommitArgs),
}

#[derive(Debug, Args)]
struct HookPostCommitArgs {
    #[arg(
        long,
        value_name = "SCOPE",
        help = "Pet state recording scope: repo, global, or both"
    )]
    scope: Option<PetScope>,
}

#[derive(Debug, Args)]
struct InstallHookArgs {
    #[arg(
        long,
        value_name = "SCOPE",
        help = "Pet state recording scope written into the hook"
    )]
    scope: Option<PetScope>,
}

impl RunArgs {
    fn overrides(&self) -> ConfigOverrides {
        ConfigOverrides {
            large_diff_line_limit: self.large_diff_line_limit,
            large_diff_file_limit: self.large_diff_file_limit,
            lines_per_second: self.lines_per_second,
            commits_per_second: self.commits_per_second,
            pet_scope: self.pet_scope,
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config_path = cli.config.clone();
    let repo_path = match cli.repo.clone() {
        Some(path) => path,
        None => std::env::current_dir()?,
    };

    match cli.command {
        Some(CliCommand::Hook {
            command: HookCommand::PostCommit(args),
        }) => record_post_commit(repo_path, config_path, args.scope),
        Some(CliCommand::InstallHook(args)) => install_hook(repo_path, config_path, args.scope),
        Some(CliCommand::PetDemo) => {
            print_pet_demo();
            Ok(())
        }
        None => run_tui(repo_path, config_path, cli.run),
    }
}

fn print_pet_demo() {
    println!("commitchi pet expressions (phase 0 and 1, with a blink frame):\n");
    for (label, expr) in sprite::PREVIEW_EXPRESSIONS {
        println!("  {label}");
        let frames = [
            sprite::preview_lines(expr, false, 0),
            sprite::preview_lines(expr, false, 1),
            sprite::preview_lines(expr, true, 0),
        ];
        let height = frames.iter().map(Vec::len).max().unwrap_or(0);
        for row in 0..height {
            let mut line = String::from("    ");
            for frame in &frames {
                let cell = frame.get(row).cloned().unwrap_or_default();
                line.push_str(&cell);
                line.push_str("   ");
            }
            println!("{line}");
        }
        println!();
    }
}

fn run_tui(repo_path: PathBuf, config_path: Option<PathBuf>, args: RunArgs) -> Result<()> {
    let repo = RepoHandle::discover(&repo_path)?;
    let mut config = RuntimeConfig::load(repo.root(), config_path.as_deref())?;
    config.apply_overrides(args.overrides())?;

    let app = App::load(
        repo_path,
        config.diff_options,
        config.animation_config,
        config.pet_scope,
        config.mood_config,
    )?;

    run(app)
}

fn record_post_commit(
    repo_path: PathBuf,
    config_path: Option<PathBuf>,
    scope: Option<PetScope>,
) -> Result<()> {
    let repo = RepoHandle::discover(repo_path)?;
    let config = RuntimeConfig::load(repo.root(), config_path.as_deref())?;
    let scope = scope.unwrap_or(config.pet_scope);
    let commit = repo.head_commit_summary()?;
    let record = ActivityRecord::new(
        repo_id_from_path(repo.root()),
        commit.hash.clone(),
        commit.time_seconds,
        now_seconds(),
    );
    let state_files = PetStateFiles::for_git_dir(repo.git_dir(), scope)?;
    state_files.ensure_parent_dirs()?;
    state_files.record_commit(scope, record)?;

    println!(
        "commitchi recorded {} in {} pet state",
        commit.short_hash, scope
    );
    Ok(())
}

fn install_hook(
    repo_path: PathBuf,
    config_path: Option<PathBuf>,
    scope: Option<PetScope>,
) -> Result<()> {
    let repo = RepoHandle::discover(repo_path)?;
    let config = RuntimeConfig::load(repo.root(), config_path.as_deref())?;
    let scope = scope.unwrap_or(config.pet_scope);
    let hook_path = hooks::install_post_commit_hook(repo.git_dir(), scope)?;

    println!(
        "commitchi installed post-commit hook at {}",
        hook_path.display()
    );
    Ok(())
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
    let mut state_watcher = StateWatcher::watch(app.pet_watch_paths()).ok().flatten();
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

        if let Some(watcher) = state_watcher.as_mut() {
            if watcher.drain_changed()
                && handle_app_event(terminal, app, AppEvent::StateFileChanged)?
            {
                return Ok(());
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
            if app.is_jumping() {
                app.handle_jump_key(key).map_err(Into::into)
            } else {
                let command = command_for_key(key);
                app.apply_command(command).map_err(Into::into)
            }
        }
        AppEvent::Tick(elapsed) => {
            app.tick(elapsed)?;
            Ok(false)
        }
        AppEvent::StateFileChanged => {
            app.reload_pet_state()?;
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

fn parse_positive_usize(value: &str) -> std::result::Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|err| format!("must be an integer: {err}"))?;

    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("must be greater than 0".to_owned())
    }
}
