use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Quit,
    PreviousCommit,
    NextCommit,
    JumpBackward,
    JumpForward,
    FirstCommit,
    LastCommit,
    ScrollUp,
    ScrollDown,
    TogglePlayback,
    FasterPlayback,
    SlowerPlayback,
    FasterReveal,
    SlowerReveal,
    BeginJump,
    Noop,
}

pub fn command_for_key(key: KeyEvent) -> Command {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Command::Quit;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Command::Quit,
        KeyCode::Char('h') | KeyCode::Left => Command::PreviousCommit,
        KeyCode::Char('l') | KeyCode::Right => Command::NextCommit,
        KeyCode::Char('k') | KeyCode::PageUp => Command::JumpBackward,
        KeyCode::Char('j') | KeyCode::PageDown => Command::JumpForward,
        KeyCode::Home => Command::FirstCommit,
        KeyCode::End => Command::LastCommit,
        KeyCode::Up => Command::ScrollUp,
        KeyCode::Down => Command::ScrollDown,
        KeyCode::Char(' ') => Command::TogglePlayback,
        KeyCode::Char('+') | KeyCode::Char('=') => Command::FasterPlayback,
        KeyCode::Char('-') => Command::SlowerPlayback,
        KeyCode::Char(']') => Command::FasterReveal,
        KeyCode::Char('[') => Command::SlowerReveal,
        KeyCode::Char('g') | KeyCode::Char(':') => Command::BeginJump,
        _ => Command::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_commit_navigation_keys() {
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            Command::PreviousCommit
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Command::PreviousCommit
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
            Command::NextCommit
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Command::NextCommit
        );
    }

    #[test]
    fn maps_jump_and_scroll_keys() {
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            Command::JumpForward
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            Command::JumpForward
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Command::ScrollUp
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            Command::ScrollDown
        );
    }

    #[test]
    fn maps_quit_keys() {
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Command::Quit
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Command::Quit
        );
    }

    #[test]
    fn maps_playback_and_speed_keys() {
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            Command::TogglePlayback
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE)),
            Command::FasterPlayback
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE)),
            Command::SlowerPlayback
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE)),
            Command::FasterReveal
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Command::SlowerReveal
        );
    }

    #[test]
    fn maps_jump_keys() {
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
            Command::BeginJump
        );
        assert_eq!(
            command_for_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
            Command::BeginJump
        );
    }
}
