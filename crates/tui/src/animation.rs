use std::time::Duration;

const DEFAULT_LINES_PER_SECOND: f64 = 30.0;
const DEFAULT_COMMITS_PER_SECOND: f64 = 1.0;
const MIN_LINES_PER_SECOND: f64 = 1.0;
const MAX_LINES_PER_SECOND: f64 = 1_000.0;
const MIN_COMMITS_PER_SECOND: f64 = 0.1;
const MAX_COMMITS_PER_SECOND: f64 = 30.0;
const SPEED_FACTOR: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationConfig {
    lines_per_second: f64,
    commits_per_second: f64,
}

impl AnimationConfig {
    pub fn new(lines_per_second: f64, commits_per_second: f64) -> Self {
        Self {
            lines_per_second: sanitize_speed(
                lines_per_second,
                DEFAULT_LINES_PER_SECOND,
                MIN_LINES_PER_SECOND,
                MAX_LINES_PER_SECOND,
            ),
            commits_per_second: sanitize_speed(
                commits_per_second,
                DEFAULT_COMMITS_PER_SECOND,
                MIN_COMMITS_PER_SECOND,
                MAX_COMMITS_PER_SECOND,
            ),
        }
    }

    pub fn lines_per_second(self) -> f64 {
        self.lines_per_second
    }

    pub fn commits_per_second(self) -> f64 {
        self.commits_per_second
    }

    pub fn increase_line_speed(&mut self) {
        self.lines_per_second = (self.lines_per_second * SPEED_FACTOR).min(MAX_LINES_PER_SECOND);
    }

    pub fn decrease_line_speed(&mut self) {
        self.lines_per_second = (self.lines_per_second / SPEED_FACTOR).max(MIN_LINES_PER_SECOND);
    }

    pub fn increase_commit_speed(&mut self) {
        self.commits_per_second =
            (self.commits_per_second * SPEED_FACTOR).min(MAX_COMMITS_PER_SECOND);
    }

    pub fn decrease_commit_speed(&mut self) {
        self.commits_per_second =
            (self.commits_per_second / SPEED_FACTOR).max(MIN_COMMITS_PER_SECOND);
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self::new(DEFAULT_LINES_PER_SECOND, DEFAULT_COMMITS_PER_SECOND)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffAnimation {
    total_lines: usize,
    visible_lines: usize,
    line_progress: f64,
}

impl DiffAnimation {
    pub fn new(total_lines: usize) -> Self {
        Self {
            total_lines,
            visible_lines: 0,
            line_progress: 0.0,
        }
    }

    pub fn reset(&mut self, total_lines: usize) {
        self.total_lines = total_lines;
        self.visible_lines = 0;
        self.line_progress = 0.0;
    }

    pub fn advance(&mut self, elapsed: Duration, lines_per_second: f64) {
        if self.visible_lines >= self.total_lines {
            self.line_progress = 0.0;
            return;
        }

        self.line_progress += elapsed.as_secs_f64() * lines_per_second.max(0.0);
        let reveal_count = self.line_progress.floor() as usize;
        if reveal_count == 0 {
            return;
        }

        self.visible_lines = self
            .visible_lines
            .saturating_add(reveal_count)
            .min(self.total_lines);
        if self.visible_lines == self.total_lines {
            self.line_progress = 0.0;
        } else {
            self.line_progress -= reveal_count as f64;
        }
    }

    pub fn visible_lines(&self) -> usize {
        self.visible_lines
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }
}

fn sanitize_speed(value: f64, default: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value.clamp(min, max)
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_progress_accumulates_fractional_lines() {
        let mut animation = DiffAnimation::new(5);

        animation.advance(Duration::from_millis(100), 5.0);
        assert_eq!(animation.visible_lines(), 0);

        animation.advance(Duration::from_millis(100), 5.0);
        assert_eq!(animation.visible_lines(), 1);
    }

    #[test]
    fn reveal_progress_stops_at_total_lines() {
        let mut animation = DiffAnimation::new(3);

        animation.advance(Duration::from_secs(10), 30.0);

        assert_eq!(animation.visible_lines(), 3);
        assert_eq!(animation.total_lines(), 3);
    }

    #[test]
    fn reset_hides_lines_for_next_commit() {
        let mut animation = DiffAnimation::new(3);
        animation.advance(Duration::from_secs(1), 30.0);

        animation.reset(7);

        assert_eq!(animation.visible_lines(), 0);
        assert_eq!(animation.total_lines(), 7);
    }

    #[test]
    fn speed_controls_adjust_line_and_commit_rates() {
        let mut config = AnimationConfig::new(30.0, 1.0);

        config.increase_line_speed();
        config.increase_commit_speed();
        assert_eq!(config.lines_per_second(), 60.0);
        assert_eq!(config.commits_per_second(), 2.0);

        config.decrease_line_speed();
        config.decrease_commit_speed();
        assert_eq!(config.lines_per_second(), 30.0);
        assert_eq!(config.commits_per_second(), 1.0);
    }
}
