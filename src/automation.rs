//! The automation core — the part you develop into your own version.
//!
//! One "round" walks the original's seven steps (its `BUTTON_STEP` and
//! `WAIT_STEP` tables): VS COM → League → Simulate match → Await full time
//! → View match result → Select random reward → Next. Each step is a
//! button the bot finds on the game screen and clicks (see
//! `crate::buttons`); "Await full time" has no button, the bot just waits.
//! The App calls `tick()` every frame. In this demo each step takes a fixed
//! time; on Windows the scan → find → click → wait loop goes in
//! `crate::backend`.
//!
//! Real values (read from the unpacked original, MECHANICS.md round 3):
//! rounds 1-999, auto-stop after `IDLE_MINUTES` (5) without a known button,
//! per-button wait `delay_after`, contract pop-up answered Cancel / Renew.
//! Estimated: how long the game takes to show each next screen and to
//! simulate a match, and that a round counts when "Next" is clicked.

use crate::buttons::{self, IDLE_MINUTES, MAX_ROUNDS, MIN_ROUNDS};
use std::time::Instant;

/// Default number of rounds (the original's screen shows "0 / 5").
pub const DEFAULT_TARGET_ROUNDS: u32 = 5;
pub const MIN_TARGET_ROUNDS: u32 = MIN_ROUNDS;
pub const MAX_TARGET_ROUNDS: u32 = MAX_ROUNDS;
/// Auto-stop when nothing progresses this long (original `idle_minutes`).
pub const STALL_LIMIT_SECS: f64 = IDLE_MINUTES as f64 * 60.0;

/// Demo: seconds the game takes to bring up the next screen (estimated).
const DEMO_SCREEN_SECS: f64 = 2.0;
/// Demo: seconds a simulated match runs before full time (estimated).
const DEMO_MATCH_SECS: f64 = 90.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    VsCom,
    League,
    Simulate,
    AwaitFullTime,
    Result,
    RandomReward,
    Next,
}

/// The seven steps, each with the button the bot clicks (None = wait).
pub const STEPS: [(Step, Option<&str>); 7] = [
    (Step::VsCom, Some("VSCOM")),
    (Step::League, Some("League")),
    (Step::Simulate, Some("sim_match")),
    (Step::AwaitFullTime, None),
    (Step::Result, Some("result")),
    (Step::RandomReward, Some("random_pick")),
    (Step::Next, Some("next")),
];

/// How long a step takes in the demo: the screen appearing, then the
/// original's `delay_after` for that button.
fn step_secs(i: usize) -> f64 {
    match STEPS[i].1.and_then(buttons::by_key) {
        // hover + settle before the wait (fo3bot.bot, see bot.rs)
        Some(b) => DEMO_SCREEN_SECS + crate::bot::min_click_secs() + b.delay_after as f64,
        None => DEMO_MATCH_SECS,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunState {
    Idle,
    Running,
    /// Reached the target number of rounds.
    Finished,
    /// Stopped by the no-progress watchdog.
    Stalled,
}

pub struct Automation {
    pub state: RunState,
    /// Index into `STEPS` while running.
    pub step: usize,
    step_started: Instant,
    last_progress: Instant,
    /// Running time accumulated over earlier runs since the last reset.
    banked_secs: f64,
    run_started: Option<Instant>,
    pub completed: u32,
    pub target: u32,
    pub unlimited: bool,
    pub hide_window: bool,
    pub renew_contracts: bool,
}

impl Automation {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: RunState::Idle,
            step: 0,
            step_started: now,
            last_progress: now,
            banked_secs: 0.0,
            run_started: None,
            completed: 0,
            target: DEFAULT_TARGET_ROUNDS,
            unlimited: false,
            hide_window: false,
            renew_contracts: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == RunState::Running
    }

    pub fn toggle(&mut self) {
        if self.is_running() {
            self.stop(RunState::Idle);
        } else {
            self.start();
        }
    }

    pub fn start(&mut self) {
        // Starting again after reaching the target begins a fresh count.
        if !self.unlimited && self.completed >= self.target {
            self.reset();
        }
        let now = Instant::now();
        self.state = RunState::Running;
        self.step = 0;
        self.step_started = now;
        self.last_progress = now;
        self.run_started = Some(now);
        crate::backend::on_start(self.hide_window);
    }

    pub fn stop(&mut self, why: RunState) {
        if let Some(t) = self.run_started.take() {
            self.banked_secs += t.elapsed().as_secs_f64();
        }
        self.state = why;
        crate::backend::on_stop();
    }

    /// "Reset rounds": clear the count and the timers.
    pub fn reset(&mut self) {
        self.completed = 0;
        self.banked_secs = 0.0;
        if self.is_running() {
            self.run_started = Some(Instant::now());
        } else {
            self.state = RunState::Idle;
        }
    }

    pub fn set_target(&mut self, n: u32) {
        self.target = n.clamp(MIN_TARGET_ROUNDS, MAX_TARGET_ROUNDS);
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.banked_secs + self.run_started.map_or(0.0, |t| t.elapsed().as_secs_f64())
    }

    pub fn average_secs(&self) -> Option<f64> {
        (self.completed > 0).then(|| self.elapsed_secs() / self.completed as f64)
    }

    /// Share of the current step that has passed (0..1), for the bar.
    pub fn step_fraction(&self) -> f32 {
        (self.step_started.elapsed().as_secs_f64() / step_secs(self.step)).clamp(0.0, 1.0) as f32
    }

    pub fn current_step(&self) -> Step {
        STEPS[self.step].0
    }

    pub fn set_hide_window(&mut self, on: bool) {
        self.hide_window = on;
        crate::backend::hide_game_window(on);
    }

    /// Advance the demo. Called once per frame.
    pub fn tick(&mut self) {
        if !self.is_running() {
            return;
        }
        if self.last_progress.elapsed().as_secs_f64() > STALL_LIMIT_SECS {
            self.stop(RunState::Stalled);
            return;
        }
        let (step, key) = STEPS[self.step];
        if self.step_started.elapsed().as_secs_f64() < step_secs(self.step) {
            return;
        }
        // The step is done: click its button (if it has one), then move on.
        if let Some(b) = key.and_then(buttons::by_key) {
            crate::backend::click_button(b);
        }
        if step == Step::Result {
            // Contract pop-ups appear between matches; the original presses
            // Cancel, or Renew when auto-renew is on (where in the round it
            // appears is estimated).
            crate::backend::click_button(buttons::contract_choice(self.renew_contracts));
        }
        let now = Instant::now();
        self.step_started = now;
        self.last_progress = now;
        if self.step + 1 < STEPS.len() {
            self.step += 1;
        } else {
            self.completed += 1;
            self.step = 0;
            if !self.unlimited && self.completed >= self.target {
                self.stop(RunState::Finished);
            }
        }
    }
}
