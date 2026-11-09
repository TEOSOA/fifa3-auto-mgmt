//! What the bot looks for on the game screen, and how it reacts.
//!
//! Every value here was read from the original's own settings, unpacked
//! from FO3AutoLeague.exe (see MECHANICS.md, "round 3"): module
//! `fo3bot.config`, `_default_buttons()` and `default_config()`, constants
//! at offsets 0x11274dd..0x1127f3a of the unpacked main.dll.
//!
//! A button is found by comparing a picture of it with the game screen
//! (template matching); `threshold` is the lowest similarity that counts.
//! Lower `priority` wins when several buttons are on screen at once.
//! After a click the bot waits until the button is gone, at most
//! `delay_after` seconds, before it may click it again.

/// Title of the game window the bot attaches to.
pub const WINDOW_TITLE: &str = "FIFA ONLINE3 - Developed by SPEARHEAD";
/// The game's program, used when the title doesn't match.
pub const WINDOW_EXE: &str = "fifazf.exe";
/// Seconds between two looks at the game screen.
pub const SCAN_INTERVAL_SECS: f64 = 0.06;
/// How the screen is read: Windows Graphics Capture first.
pub const CAPTURE_METHOD: &str = "wgc";
/// Stop when the same button was clicked this many times without vanishing.
pub const STUCK_CLICKS: u32 = 8;
/// Stop when no known button was seen for this many minutes.
pub const IDLE_MINUTES: u32 = 5;
/// Rounds the person may ask for.
pub const MIN_ROUNDS: u32 = 1;
pub const MAX_ROUNDS: u32 = 999;

/// What a button means to the contract / pop-up logic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Ordinary step button.
    None,
    /// "Renew contract" (costs in-game EP).
    Renew,
    /// "Cancel" next to Renew.
    Decline,
    /// Renew shown greyed out.
    RenewOff,
    /// Title of a pop-up whose "Accept" may be pressed.
    AckTitle,
    /// "Accept" — pressed only under a known title.
    Ack,
    /// "Master League mode" choice.
    Pick,
}

#[derive(Clone, Copy, Debug)]
pub struct Button {
    pub key: &'static str,
    pub image: &'static str,
    pub threshold: f32,
    pub priority: u8,
    pub delay_after: f32,
    pub role: Role,
}

const fn b(
    key: &'static str,
    image: &'static str,
    threshold: f32,
    priority: u8,
    delay_after: f32,
    role: Role,
) -> Button {
    Button { key, image, threshold, priority, delay_after, role }
}

/// The original's default button list (Thai game client), in its order.
pub const BUTTONS: [Button; 14] = [
    b("contract_renew", "templates/contract_renew.png", 0.92, 0, 1.5, Role::Renew),
    b("contract_cancel", "templates/contract_cancel.png", 0.92, 0, 1.5, Role::Decline),
    b("contract_renew_off", "templates/contract_renew_off.png", 0.90, 0, 1.5, Role::RenewOff),
    b("contract_done_title", "templates/contract_done_title.png", 0.88, 0, 1.0, Role::AckTitle),
    b("level_up_title", "templates/level_up_title.png", 0.80, 0, 1.0, Role::AckTitle),
    b("accept", "templates/contract_accept.png", 0.88, 0, 1.0, Role::Ack),
    b("mode_master", "templates/mode_master.png", 0.88, 0, 1.0, Role::Pick),
    b("sponsor_ok", "templates/sponsor_ok.png", 0.90, 1, 1.5, Role::None),
    b("random_pick", "templates/random_pick.png", 0.88, 1, 1.5, Role::None),
    b("result", "templates/result.png", 0.90, 1, 1.5, Role::None),
    b("next", "templates/next.png", 0.85, 1, 1.5, Role::None),
    b("sim_match", "templates/sim_match.png", 0.85, 2, 1.0, Role::None),
    b("VSCOM", "templates/VSCOM.png", 0.85, 3, 1.0, Role::None),
    b("League", "templates/League.png", 0.85, 2, 1.0, Role::None),
];

pub fn by_key(key: &str) -> Option<&'static Button> {
    BUTTONS.iter().find(|b| b.key == key)
}

/// The contract pop-up shows "Cancel | Renew" side by side; the original
/// presses Renew only when the person turned auto-renew on.
pub fn contract_choice(auto_renew: bool) -> &'static Button {
    let key = if auto_renew { "contract_renew" } else { "contract_cancel" };
    by_key(key).expect("contract buttons are in the table")
}

// ---------------------------------------------------------------- buying

/// Buttons of the player-buying feature (new in this version). Names and
/// places come from the person's game screenshots; the thresholds and
/// waits reuse the original's values for similar buttons (estimated).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuyButton {
    /// The "+N" level badge in a search-result row (one picture per level).
    LevelBadge,
    /// "ซื้อ" at the bottom right of the search results.
    BuyBottom,
    /// "ซื้อ" in the "ซื้อนักเตะ" confirm window.
    BuyConfirm,
    /// "ยอมรับ" in the pop-up after confirming (both pop-ups below have it).
    Accept,
    /// Title of the "ซื้อ <name>" window: the player was bought.
    BoughtTitle,
    /// The "ข้อความ" window's text "โปรดรอสักครู่แล้วลองอีกครั้ง" (please
    /// wait and try again): not bought (attachments/picture-12).
    TryAgainMessage,
    /// "Team" in the bottom menu bar.
    NavTeam,
    /// The "จัดการทีม" card on the Team screen.
    TeamManage,
    /// The "จัดการนักเตะ" tab.
    PlayerManageTab,
    /// The "ระดับ" sub-tab (upgrade page).
    LevelTab,
}

pub struct BuySpec {
    pub image: &'static str,
    pub threshold: f32,
    pub delay_after: f32,
}

/// Pictures without a value of their own use the original's
/// `Template.threshold` default (fo3bot.matcher, see matcher.rs).
use crate::matcher::DEFAULT_THRESHOLD;

impl BuyButton {
    pub fn spec(self) -> BuySpec {
        let (image, threshold, delay_after) = match self {
            BuyButton::LevelBadge => ("templates/buy/level_plus{n}.png", 0.90, 1.0),
            BuyButton::BuyBottom => ("templates/buy/buy.png", 0.88, 1.5),
            BuyButton::BuyConfirm => ("templates/buy/buy_confirm.png", 0.88, 1.5),
            BuyButton::Accept => ("templates/buy/accept.png", 0.88, 1.0),
            BuyButton::NavTeam => ("templates/nav/team.png", DEFAULT_THRESHOLD, 1.5),
            BuyButton::TeamManage => ("templates/nav/team_manage.png", DEFAULT_THRESHOLD, 1.5),
            BuyButton::PlayerManageTab => ("templates/nav/player_manage.png", DEFAULT_THRESHOLD, 1.0),
            BuyButton::LevelTab => ("templates/nav/level.png", DEFAULT_THRESHOLD, 1.0),
        };
        BuySpec { image, threshold, delay_after }
    }
}
