use commitchi_core::{DiffLine, DiffLineKind, FileDiff, FileStatus, StructuredDiff};
use commitchi_pet::{now_seconds, ActivityRecord, Mood, PetScope};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, PetStatus};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(6),
            Constraint::Length(5),
        ])
        .split(area);

    render_header(frame, layout[0], app);
    render_body(frame, layout[1], app);
    render_timeline(frame, layout[2], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let commit = app.selected_commit();
    let diff = app.diff();
    let (position, total) = app.position();
    let (visible_lines, total_lines) = app.diff_reveal_progress();
    let animation_config = app.animation_config();
    let playback_state = if app.is_playing() {
        "playing"
    } else {
        "paused"
    };
    let truncated = if diff.truncated { " truncated" } else { "" };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                commit.short_hash.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(commit.summary.clone()),
        ]),
        Line::from(format!(
            "{} <{}> | parents: {} | repo: {}",
            commit.author_name,
            commit.author_email,
            commit.parent_count,
            app.repo_root().display()
        )),
        Line::from(format!(
            "commit {}/{} | {} files | +{} -{}{}",
            position,
            total,
            diff.stats.files_changed,
            diff.stats.additions,
            diff.stats.deletions,
            truncated
        )),
        Line::from(format!(
            "{} | reveal {}/{} @ {:.1} lps | playback {:.1} cps",
            playback_state,
            visible_lines,
            total_lines,
            animation_config.lines_per_second(),
            animation_config.commits_per_second()
        )),
    ];

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Commit"));
    frame.render_widget(paragraph, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if area.width >= 78 {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30),
                Constraint::Min(22),
                Constraint::Length(22),
            ])
            .split(area);

        render_files(frame, layout[0], app.diff());
        render_diff(frame, layout[1], app);
        render_pet(frame, layout[2], &app.pet_status());
    } else {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(34), Constraint::Min(20)])
            .split(area);

        render_files(frame, layout[0], app.diff());
        render_diff(frame, layout[1], app);
    }
}

fn render_files(frame: &mut Frame<'_>, area: Rect, diff: &StructuredDiff) {
    let items = diff
        .files
        .iter()
        .map(|file| ListItem::new(file_item_line(file)))
        .collect::<Vec<_>>();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Files"));
    frame.render_widget(list, area);
}

fn render_diff(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let (visible_lines, total_lines) = app.diff_reveal_progress();
    let lines = diff_lines(app.diff(), visible_lines);
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Diff {visible_lines}/{total_lines}")),
        )
        .scroll((app.diff_scroll(), 0));
    frame.render_widget(paragraph, area);
}

fn render_pet(frame: &mut Frame<'_>, area: Rect, status: &PetStatus) {
    let mood = primary_mood(status);
    let style = mood_style(mood);
    let mut lines = pet_sprite(mood)
        .into_iter()
        .map(|line| Line::styled(line, style))
        .collect::<Vec<_>>();

    lines.push(Line::from(""));
    match status.scope {
        PetScope::Repo => {
            lines.extend(scope_lines(
                "repo",
                status.repo_mood,
                &status.repo_last_activity,
            ));
        }
        PetScope::Global => {
            lines.extend(scope_lines(
                "global",
                status.global_mood,
                &status.global_last_activity,
            ));
        }
        PetScope::Both => {
            lines.extend(scope_lines(
                "repo",
                status.repo_mood,
                &status.repo_last_activity,
            ));
            lines.extend(scope_lines(
                "global",
                status.global_mood,
                &status.global_last_activity,
            ));
        }
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Pet {}", status.scope)),
    );
    frame.render_widget(paragraph, area);
}

fn render_timeline(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let (position, total) = app.position();
    let lines = vec![
        Line::from(timeline_line(area.width, position, total)),
        Line::from("h/l Left/Right: commit | j/k PgUp/PgDn: jump | Up/Down: scroll"),
        Line::from("Space: play/pause | +/-: commit speed | []: reveal speed | q: quit"),
    ];
    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Timeline"));
    frame.render_widget(paragraph, area);
}

fn primary_mood(status: &PetStatus) -> Mood {
    match status.scope {
        PetScope::Repo => status.repo_mood.unwrap_or(Mood::Neutral),
        PetScope::Global => status.global_mood.unwrap_or(Mood::Neutral),
        PetScope::Both => status
            .repo_mood
            .or(status.global_mood)
            .unwrap_or(Mood::Neutral),
    }
}

fn scope_lines(
    label: &'static str,
    mood: Option<Mood>,
    activity: &Option<ActivityRecord>,
) -> Vec<Line<'static>> {
    let mood = mood.unwrap_or(Mood::Neutral);
    let age = activity
        .as_ref()
        .map(activity_age)
        .unwrap_or_else(|| "no commits".to_owned());
    vec![
        Line::from(vec![
            Span::styled(format!("{label:>6} "), Style::default().fg(Color::DarkGray)),
            Span::styled(mood.to_string(), mood_style(mood)),
        ]),
        Line::from(vec![
            Span::raw("       "),
            Span::styled(age, Style::default().fg(Color::DarkGray)),
        ]),
    ]
}

fn activity_age(activity: &ActivityRecord) -> String {
    let seconds = now_seconds()
        .saturating_sub(activity.recorded_at_seconds)
        .max(0);
    if seconds < 60 {
        "just now".to_owned()
    } else if seconds < 3_600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h ago", seconds / 3_600)
    } else {
        format!("{}d ago", seconds / 86_400)
    }
}

fn pet_sprite(mood: Mood) -> Vec<&'static str> {
    match mood {
        Mood::Thriving => vec![" /\\_/\\", "( ^.^ )", " /|_|\\", "  / \\"],
        Mood::Content => vec![" /\\_/\\", "( o.o )", " /|_|\\", "  / \\"],
        Mood::Neutral => vec![" /\\_/\\", "( -.- )", " /|_|\\", "  / \\"],
        Mood::Anxious => vec![" /\\_/\\", "( o_o )", " /|_|\\", "  / \\"],
        Mood::Sulking => vec![" /\\_/\\", "( v_v )", " /|_|\\", "  / \\"],
    }
}

fn file_item_line(file: &FileDiff) -> Line<'static> {
    let marker = match file.status {
        FileStatus::Added => "A",
        FileStatus::Deleted => "D",
        FileStatus::Modified => "M",
        FileStatus::Renamed => "R",
        FileStatus::Copied => "C",
        FileStatus::TypeChanged => "T",
        FileStatus::Conflicted => "!",
        FileStatus::Truncated => "...",
    };
    let style = file_status_style(file.status);
    let suffix = if file.binary {
        " binary".to_owned()
    } else if file.additions == 0 && file.deletions == 0 {
        String::new()
    } else {
        format!(" +{} -{}", file.additions, file.deletions)
    };

    Line::from(vec![
        Span::styled(format!("{marker:>3} "), style),
        Span::raw(file.display_path()),
        Span::styled(suffix, Style::default().fg(Color::DarkGray)),
    ])
}

fn diff_lines(diff: &StructuredDiff, visible_limit: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let total_lines = diff
        .files
        .iter()
        .map(|file| file.lines.len())
        .sum::<usize>();

    if total_lines > 0 && visible_limit == 0 {
        return vec![Line::styled(
            "Revealing diff...",
            Style::default().fg(Color::DarkGray),
        )];
    }

    for file in &diff.files {
        for line in &file.lines {
            if lines.len() >= visible_limit {
                break;
            }
            lines.push(render_diff_line(line));
        }

        if lines.len() >= visible_limit {
            break;
        }
    }

    if lines.is_empty() {
        lines.push(Line::from("No textual diff for this commit."));
    }

    lines
}

fn render_diff_line(line: &DiffLine) -> Line<'static> {
    let style = diff_line_style(line.kind);
    let old = line
        .old_lineno
        .map(|value| value.to_string())
        .unwrap_or_default();
    let new = line
        .new_lineno
        .map(|value| value.to_string())
        .unwrap_or_default();
    let prefix = match line.kind {
        DiffLineKind::Addition => "+",
        DiffLineKind::Deletion => "-",
        DiffLineKind::Context => " ",
        DiffLineKind::HunkHeader => "@",
        DiffLineKind::FileHeader => "=",
        DiffLineKind::Binary => "!",
        DiffLineKind::Truncated => "...",
    };

    Line::from(vec![
        Span::styled(format!("{prefix:>3} {old:>4} {new:>4} "), style),
        Span::styled(line.content.clone(), style),
    ])
}

fn timeline_line(width: u16, position: usize, total: usize) -> String {
    if total == 0 {
        return "[] 0/0".to_owned();
    }

    let reserved = 16usize;
    let width = usize::from(width).saturating_sub(reserved).clamp(8, 80);
    let mut bar = vec!['-'; width];

    if total == 1 {
        bar[0] = '*';
    } else {
        for index in 0..total {
            let mark = index * (width - 1) / (total - 1);
            bar[mark] = '|';
        }
        let active = (position - 1) * (width - 1) / (total - 1);
        bar[active] = '*';
    }

    format!(
        "[{}] {}/{}",
        bar.into_iter().collect::<String>(),
        position,
        total
    )
}

fn file_status_style(status: FileStatus) -> Style {
    match status {
        FileStatus::Added => Style::default().fg(Color::Green),
        FileStatus::Deleted => Style::default().fg(Color::Red),
        FileStatus::Modified => Style::default().fg(Color::Blue),
        FileStatus::Renamed | FileStatus::Copied => Style::default().fg(Color::Cyan),
        FileStatus::TypeChanged => Style::default().fg(Color::Magenta),
        FileStatus::Conflicted => Style::default().fg(Color::LightRed),
        FileStatus::Truncated => Style::default().fg(Color::DarkGray),
    }
}

fn mood_style(mood: Mood) -> Style {
    match mood {
        Mood::Thriving => Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
        Mood::Content => Style::default().fg(Color::Green),
        Mood::Neutral => Style::default().fg(Color::Yellow),
        Mood::Anxious => Style::default().fg(Color::LightMagenta),
        Mood::Sulking => Style::default().fg(Color::LightRed),
    }
}

fn diff_line_style(kind: DiffLineKind) -> Style {
    match kind {
        DiffLineKind::Addition => Style::default().fg(Color::Green),
        DiffLineKind::Deletion => Style::default().fg(Color::Red),
        DiffLineKind::HunkHeader => Style::default().fg(Color::Cyan),
        DiffLineKind::FileHeader => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        DiffLineKind::Binary | DiffLineKind::Truncated => Style::default().fg(Color::DarkGray),
        DiffLineKind::Context => Style::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_marks_single_commit() {
        assert!(timeline_line(40, 1, 1).contains("*"));
    }

    #[test]
    fn timeline_marks_current_position() {
        let line = timeline_line(40, 2, 3);
        assert!(line.contains("2/3"));
        assert_eq!(line.matches('*').count(), 1);
    }
}
