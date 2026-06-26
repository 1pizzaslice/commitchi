//! Pet domain types.
//!
//! Phase 1 only scaffolds this crate so later phases can add persistence,
//! hook handling, and TUI rendering without changing the workspace shape.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetScope {
    Repo,
    Global,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Thriving,
    Content,
    Neutral,
    Anxious,
    Sulking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Calm,
    Excited,
    Curious,
    Confused,
    Wincing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetState {
    mood: Mood,
}

impl PetState {
    pub fn new(mood: Mood) -> Self {
        Self { mood }
    }

    pub fn mood(&self) -> Mood {
        self.mood
    }
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            mood: Mood::Neutral,
        }
    }
}
