//! Auto-buy players on the transfer market — a feature of this version
//! (the original program doesn't have it).
//!
//! The person opens the market's buy page with the search results
//! themselves (Transfer → ซื้อ → ผลการค้นหา). Then each purchase is:
//!   1. find a result row whose level badge is the chosen +N (+1..+10)
//!   2. click that row (it gets a yellow frame)
//!   3. click "ซื้อ" at the bottom right
//!   4. click "ซื้อ" in the "ซื้อนักเตะ" confirm window
//!   5. close the pop-up with "ยอมรับ". Only the "ซื้อ <name>" window
//!      counts 1 bought. The "ข้อความ" window "โปรดรอสักครู่แล้วลองอีกครั้ง"
//!      (the game is busy, attachments/picture-12) has the same "ยอมรับ"
//!      but is NOT a purchase: it is counted apart, and the bot waits and
//!      starts the purchase again from step 1.
//! When the count reaches the target the run stops; with the switch on it
//! first goes Team → จัดการทีม → จัดการนักเตะ → ระดับ (the upgrade page).
//!
//! Screens and button names come from the person's screenshots of the
//! game (attachments/picture-4..11). Waits and match thresholds reuse the
//! original bot's values for similar buttons (estimated, MECHANICS.md).

use crate::buttons::{BuyButton, IDLE_MINUTES, MAX_ROUNDS, MIN_ROUNDS};
use std::time::Instant;

pub const DEFAULT_TARGET: u32 = 5;
pub const MIN_LEVEL: u8 = 1;
pub const MAX_LEVEL: u8 = 10;
/// Demo: seconds the game takes to answer each click (estimated).
const DEMO_SCREEN_SECS: f64 = 1.0;
/// After "please wait and try again", wait this long before trying again.
/// Reuses the original's PICK_COOLDOWN (4 s, fo3bot.bot) — estimated fit.
pub const RETRY_WAIT_SECS: f64 = crate::bot::PICK_COOLDOWN_SECS;

/// What the pop-up after "confirm purchase" said.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuyOutcome {
    /// "ซื้อ <name>" — the player was bought.
    Bought,
    /// "ข้อความ: โปรดรอสักครู่แล้วลองอีกครั้ง" — not bought, try again.
    TryAgain,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuyStep {
    FindRow,
    SelectRow,
    Buy,
    Confirm,
    Accept,
}

pub const BUY_STEPS: [(BuyStep, BuyButton); 5] = [
    (BuyStep::FindRow, BuyButton::LevelBadge),
    (BuyStep::SelectRow, BuyButton::LevelBadge),
    (BuyStep::Buy, BuyButton::BuyBottom),
    (BuyStep::Confirm, BuyButton::BuyConfirm),
    (BuyStep::Accept, BuyButton::Accept),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavStep {
    Team,
    TeamManage,
    PlayerManage,
    Level,
}

pub const NAV_STEPS: [(NavStep, BuyButton); 4] = [
    (NavStep::Team, BuyButton::NavTeam),
    (NavStep::TeamManage, BuyButton::TeamManage),
    (NavStep::PlayerManage, BuyButton::PlayerManageTab),
    (NavStep::Level, BuyButton::LevelTab),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuyState {
    Idle,
    Buying,
    /// Bought enough, walking to the upgrade page.
    GoingToUpgrade,
    Finished,
    Stalled,
}

pub struct Buyer {
    pub state: BuyState,
    pub step: usize,
    step_started: Instant,
    last_progress: Instant,
    pub bought: u32,
    pub target: u32,
    /// The +N level to look for.
    pub level: u8,
    pub goto_upgrade: bool,
    /// Picture folder of the game's language ("" Thai, "en", "vi").
    pub lang_dir: &'static str,
    /// "Please wait and try again" answers this run (not counted as bought).
    pub try_again: u32,
    /// Try-agains in a row, and when that streak began.
    try_again_streak: u32,
    streak_since: Instant,
    /// Purchases attempted this run (the demo uses it to pick an outcome).
    attempts: u32,
    /// Waiting before trying again until this moment.
    retry_until: Option<Instant>,
}

impl Buyer {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: BuyState::Idle,
            step: 0,
            step_started: now,
            last_progress: now,
            bought: 0,
            target: DEFAULT_TARGET,
            level: MIN_LEVEL,
            goto_upgrade: false,
            lang_dir: "",
            try_again: 0,
            try_again_streak: 0,
            streak_since: now,
            attempts: 0,
            retry_until: None,
        }
    }

    /// Waiting because the game said "please wait and try again".
    pub fn waiting_retry(&self) -> bool {
        self.retry_until.is_some_and(|t| Instant::now() < t)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, BuyState::Buying | BuyState::GoingToUpgrade)
    }

    pub fn toggle(&mut self) {
        if self.is_running() {
            self.stop(BuyState::Idle);
        } else {
            self.start();
        }
    }

    pub fn start(&mut self) {
        if self.bought >= self.target {
            self.bought = 0;
        }
        let now = Instant::now();
        self.state = BuyState::Buying;
        self.step = 0;
        self.step_started = now;
        self.last_progress = now;
        self.try_again = 0;
        self.try_again_streak = 0;
        self.attempts = 0;
        self.retry_until = None;
        crate::backend::on_start(false);
    }

    pub fn stop(&mut self, why: BuyState) {
        self.state = why;
        crate::backend::on_stop();
    }

    pub fn reset(&mut self) {
        self.bought = 0;
        self.try_again = 0;
        if !self.is_running() {
            self.state = BuyState::Idle;
        }
    }

    pub fn set_target(&mut self, n: u32) {
        self.target = n.clamp(MIN_ROUNDS, MAX_ROUNDS);
    }

    pub fn set_level(&mut self, n: u8) {
        self.level = n.clamp(MIN_LEVEL, MAX_LEVEL);
    }

    /// Steps of the phase running now.
    pub fn step_count(&self) -> usize {
        if self.state == BuyState::GoingToUpgrade {
            NAV_STEPS.len()
        } else {
            BUY_STEPS.len()
        }
    }

    fn step_secs(&self) -> f64 {
        let b = if self.state == BuyState::GoingToUpgrade {
            NAV_STEPS[self.step].1
        } else {
            BUY_STEPS[self.step].1
        };
        // hover + settle before the wait (fo3bot.bot, see bot.rs)
        DEMO_SCREEN_SECS + crate::bot::min_click_secs() + b.spec().delay_after as f64
    }

    pub fn step_fraction(&self) -> f32 {
        (self.step_started.elapsed().as_secs_f64() / self.step_secs()).clamp(0.0, 1.0) as f32
    }

    pub fn tick(&mut self) {
        if !self.is_running() {
            return;
        }
        if self.last_progress.elapsed().as_secs_f64() > IDLE_MINUTES as f64 * 60.0 {
            self.stop(BuyState::Stalled);
            return;
        }
        if self.waiting_retry() {
            return;
        }
        if let Some(t) = self.retry_until.take() {
            self.step_started = t;
        }
        if self.step_started.elapsed().as_secs_f64() < self.step_secs() {
            return;
        }
        let buying = self.state == BuyState::Buying;
        // Before closing the last pop-up, read which one it is.
        let outcome = if buying && BUY_STEPS[self.step].0 == BuyStep::Accept {
            self.attempts += 1;
            Some(crate::backend::buy_outcome(self.lang_dir, self.attempts))
        } else {
            None
        };
        let button = if self.state == BuyState::GoingToUpgrade {
            NAV_STEPS[self.step].1
        } else {
            BUY_STEPS[self.step].1
        };
        crate::backend::click_buy_button(button, self.level, self.lang_dir);
        let now = Instant::now();
        self.step_started = now;
        self.last_progress = now;
        if self.step + 1 < self.step_count() {
            self.step += 1;
            return;
        }
        self.step = 0;
        if outcome == Some(BuyOutcome::TryAgain) {
            // Not bought: count it apart, wait, then start over at step 1.
            self.try_again += 1;
            if self.try_again_streak == 0 {
                self.streak_since = now;
            }
            self.try_again_streak += 1;
            let since = self.streak_since.elapsed().as_secs_f64();
            if crate::bot::is_stuck(self.try_again_streak, crate::buttons::STUCK_CLICKS, since) {
                self.stop(BuyState::Stalled);
                return;
            }
            self.retry_until = Some(now + std::time::Duration::from_secs_f64(RETRY_WAIT_SECS));
            return;
        }
        self.try_again_streak = 0;
        match self.state {
            BuyState::Buying => {
                self.bought += 1;
                if self.bought >= self.target {
                    if self.goto_upgrade {
                        self.state = BuyState::GoingToUpgrade;
                    } else {
                        self.stop(BuyState::Finished);
                    }
                }
            }
            _ => self.stop(BuyState::Finished),
        }
    }
}
