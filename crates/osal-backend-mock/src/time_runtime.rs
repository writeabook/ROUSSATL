//! Mock time runtime — shared virtual clock and timer registry.
//!
//! Callbacks are taken out of the registry before execution and put
//! back afterward if the timer is still alive (Periodic) or left as
//! None (OneShot, deleted). This avoids needing to clone `FnMut`.

use alloc::vec::Vec;
use core::time::Duration;

use osal_api::traits::timer::TimerCallback;
use osal_api::types::TimerMode;
use osal_portable::timer_state::TimerState;

struct MockTimerEntry {
    id: u64,
    state: TimerState,
    callback: Option<TimerCallback>,
    creation_order: u64,
    deleted: bool,
}

pub struct MockTimeRuntime {
    now: Duration,
    next_timer_id: u64,
    next_creation_order: u64,
    timers: Vec<MockTimerEntry>,
}

impl MockTimeRuntime {
    pub fn new() -> Self {
        Self {
            now: Duration::ZERO,
            next_timer_id: 1,
            next_creation_order: 0,
            timers: Vec::new(),
        }
    }

    pub fn now(&self) -> Duration {
        self.now
    }

    pub fn reset(&mut self) {
        self.now = Duration::ZERO;
        self.next_timer_id = 1;
        self.next_creation_order = 0;
        self.timers.clear();
    }

    pub fn advance(&mut self, d: Duration) {
        self.now = self.now.saturating_add(d);
        self.dispatch_expired();
    }

    pub fn register_timer(
        &mut self,
        period: Duration,
        mode: TimerMode,
        callback: TimerCallback,
    ) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        let order = self.next_creation_order;
        self.next_creation_order += 1;
        self.timers.push(MockTimerEntry {
            id,
            state: TimerState::new(period, mode)
                .expect("TimerState::new should be validated by caller"),
            callback: Some(callback),
            creation_order: order,
            deleted: false,
        });
        id
    }

    fn find_mut(&mut self, id: u64) -> Option<&mut MockTimerEntry> {
        self.timers.iter_mut().find(|e| e.id == id && !e.deleted)
    }

    pub fn start_timer(&mut self, id: u64) {
        let now = self.now;
        if let Some(e) = self.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.start(now);
        }
    }
    pub fn stop_timer(&mut self, id: u64) {
        if let Some(e) = self.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            e.state.stop();
        }
    }
    pub fn reset_timer(&mut self, id: u64) {
        let now = self.now;
        if let Some(e) = self.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.reset(now);
        }
    }
    pub fn change_period(&mut self, id: u64, new_period: Duration) {
        if let Some(e) = self.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.change_period(new_period);
        }
    }
    pub fn deregister_timer(&mut self, id: u64) {
        if let Some(e) = self.find_mut(id) {
            e.deleted = true;
            e.state.stop();
            e.callback = None;
        }
    }

    fn dispatch_expired(&mut self) {
        loop {
            let action = self.next_expired_action();
            match action {
                Some((id, generation, mut cb)) => {
                    cb();
                    self.finish_expired(id, generation, cb);
                }
                None => break,
            }
        }
    }

    /// Find the timer with the earliest deadline <= now, prepare its
    /// expiration, and return its callback for out-of-lock execution.
    fn next_expired_action(&mut self) -> Option<(u64, u64, TimerCallback)> {
        let now = self.now;
        let mut best_idx: Option<usize> = None;

        for (i, e) in self.timers.iter().enumerate() {
            if e.deleted || e.callback.is_none() {
                continue;
            }
            if let Some(d) = e.state.deadline() {
                if d <= now {
                    match best_idx {
                        None => best_idx = Some(i),
                        Some(bi) => {
                            let bd = self.timers[bi].state.deadline().unwrap();
                            if d < bd || (d == bd && e.creation_order < self.timers[bi].creation_order) {
                                best_idx = Some(i);
                            }
                        }
                    }
                }
            }
        }

        let idx = best_idx?;
        let entry = &mut self.timers[idx];

        let token = entry.state.prepare_expiration(now)?;
        let generation = token.generation;
        let callback = entry.callback.take()?;

        Some((entry.id, generation, callback))
    }

    fn finish_expired(&mut self, id: u64, gen_before: u64, callback: TimerCallback) {
        if let Some(entry) = self.find_mut(id) {
            if entry.state.generation() == gen_before {
                // State unchanged during callback — apply auto-reload
                entry.state.finish_expiration(crate::time_runtime::dummy_token());
            }
            // Put callback back if timer is still alive and periodic
            if !entry.deleted && entry.state.deadline().is_some() {
                entry.callback = Some(callback);
            }
        }
    }
}

// Dummy token for finish_expiration (generation check already done above)
pub(crate) fn dummy_token() -> osal_portable::timer_state::ExpirationToken {
    osal_portable::timer_state::ExpirationToken {
        generation: 0,
        scheduled_deadline: Duration::ZERO,
        mode: TimerMode::OneShot,
    }
}
