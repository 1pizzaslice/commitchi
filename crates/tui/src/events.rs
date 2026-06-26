use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Input(KeyEvent),
    Tick(Duration),
    Render,
}

#[derive(Debug, Clone)]
pub struct EventSchedule {
    tick_interval: Duration,
    render_interval: Duration,
    last_tick: Instant,
    last_render: Instant,
}

impl EventSchedule {
    pub fn new(tick_interval: Duration, render_interval: Duration) -> Self {
        Self::starting_at(Instant::now(), tick_interval, render_interval)
    }

    pub fn starting_at(now: Instant, tick_interval: Duration, render_interval: Duration) -> Self {
        Self {
            tick_interval,
            render_interval,
            last_tick: now,
            last_render: now,
        }
    }

    pub fn poll_timeout(&self, now: Instant) -> Duration {
        let tick_remaining = remaining_until(now, self.last_tick, self.tick_interval);
        let render_remaining = remaining_until(now, self.last_render, self.render_interval);
        tick_remaining.min(render_remaining)
    }

    pub fn drain_due(&mut self, now: Instant) -> Vec<AppEvent> {
        let mut events = Vec::with_capacity(2);

        let tick_elapsed = now.saturating_duration_since(self.last_tick);
        if tick_elapsed >= self.tick_interval {
            self.last_tick = now;
            events.push(AppEvent::Tick(tick_elapsed));
        }

        let render_elapsed = now.saturating_duration_since(self.last_render);
        if render_elapsed >= self.render_interval {
            self.last_render = now;
            events.push(AppEvent::Render);
        }

        events
    }
}

fn remaining_until(now: Instant, last: Instant, interval: Duration) -> Duration {
    interval.saturating_sub(now.saturating_duration_since(last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_targets_next_tick_or_render() {
        let start = Instant::now();
        let schedule =
            EventSchedule::starting_at(start, Duration::from_millis(50), Duration::from_millis(16));

        assert_eq!(
            schedule.poll_timeout(start + Duration::from_millis(10)),
            Duration::from_millis(6)
        );
    }

    #[test]
    fn due_events_separate_tick_and_render() {
        let start = Instant::now();
        let mut schedule =
            EventSchedule::starting_at(start, Duration::from_millis(50), Duration::from_millis(16));

        let events = schedule.drain_due(start + Duration::from_millis(50));

        assert_eq!(
            events,
            vec![AppEvent::Tick(Duration::from_millis(50)), AppEvent::Render]
        );
    }
}
