use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellStatus {
    Idle,
    Casting {
        spell_id: u32,
        start_time: Instant,
        cast_time: Duration,
        target_id: u32,
    },
}

pub struct SpellManager {
    status: SpellStatus,
}

impl SpellManager {
    pub fn new() -> Self {
        Self {
            status: SpellStatus::Idle,
        }
    }

    pub fn start_cast(&mut self, spell_id: u32, cast_time_ms: u32, target_id: u32) {
        self.status = SpellStatus::Casting {
            spell_id,
            start_time: Instant::now(),
            cast_time: Duration::from_millis(cast_time_ms as u64),
            target_id,
        };
    }

    pub fn interrupt(&mut self) {
        self.status = SpellStatus::Idle;
    }

    pub fn is_casting(&self) -> bool {
        matches!(self.status, SpellStatus::Casting { .. })
    }

    pub fn check_completion(&mut self) -> Option<u32> {
        if let SpellStatus::Casting { spell_id, start_time, cast_time, .. } = self.status {
            if start_time.elapsed() >= cast_time {
                self.status = SpellStatus::Idle;
                return Some(spell_id);
            }
        }
        None
    }

    pub fn current_spell(&self) -> Option<u32> {
        if let SpellStatus::Casting { spell_id, .. } = self.status {
            Some(spell_id)
        } else {
            None
        }
    }

    pub fn current_target(&self) -> Option<u32> {
        if let SpellStatus::Casting { target_id, .. } = self.status {
            Some(target_id)
        } else {
            None
        }
    }
}
