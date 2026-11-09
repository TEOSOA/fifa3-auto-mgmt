//! Timing and safety rules of the original's main loop — module
//! `fo3bot.bot` of the unpacked main.dll (MECHANICS.md, "round 6").
//!
//! The module's init code `sub_180052480` stores each value under its
//! name with `mov r8,[value]; mov rdx,[name]; call sub_1800563b0`
//! (0x180053a67..0x180053c2a). Values are blob items 326..339 (constants
//! blob at file offset 0x111e9ae, 386 items); each name is matched by the
//! one function that reads it (listed per value).

/// After a click, wait at least this long before looking again
/// (MIN_SETTLE, read by `_wait_until_gone` sub_180074650).
pub const MIN_SETTLE_SECS: f64 = 0.12;
/// ...then look this often until the button is gone or `delay_after`
/// has passed (POLL_INTERVAL, sub_180074650).
pub const POLL_INTERVAL_SECS: f64 = 0.04;
/// Pause with the pointer on the button before clicking
/// (HOVER_BEFORE_CLICK, read by `run` sub_180075790).
pub const HOVER_BEFORE_CLICK_SECS: f64 = 0.1;
/// After this many clicks that changed nothing on screen, wake the game
/// with one real mouse move (WAKE_AFTER_NO_EFFECT, sub_180075790).
pub const WAKE_AFTER_NO_EFFECT: u32 = 3;
/// A button pressed again and again only counts as stuck once this much
/// time has passed too (STUCK_MIN_SECONDS, read by `note_scan`
/// sub_18006fb90, next to the message "กดปุ่ม '…' ซ้ำ … ครั้งแล้วปุ่มไม่หายไป").
pub const STUCK_MIN_SECONDS: f64 = 60.0;
/// Screenshots of stuck screens kept on disk (STUCK_KEEP, read by
/// `_save_stuck_frame` sub_180072df0).
pub const STUCK_KEEP: usize = 20;
/// Seconds before the same choice may be picked again (PICK_COOLDOWN,
/// read by `decide` sub_1800633d0).
pub const PICK_COOLDOWN_SECS: f64 = 4.0;
/// Window sizes whose scale is within this of a picture's own scale use
/// the picture as is (SCALE_NEAR, read by `_scale_of` sub_180062770).
pub const SCALE_NEAR: f32 = 0.015;
/// Scales the bot accepts at all (SCALE_RANGE, sub_180062770).
pub const SCALE_RANGE: (f32, f32) = (0.5, 3.0);
/// "Accept" is pressed only when it sits at most this many pixels below a
/// known pop-up title (ACK_MAX_DY, read by `_ack_choice`)...
pub const ACK_MAX_DY: u32 = 140;
/// ...or 340 pixels for these titles (ACK_MAX_DY_BY_TITLE, constant dict
/// at 0x181122aee): the level-up window (TH, VI, EN) and the English
/// contract-done window.
pub const ACK_FAR_TITLES: [&str; 4] = [
    "ผู้จัดการเลเวลเพิ่มขึ้น",
    "HLV lên cấp·vi",
    "ผู้จัดการเลเวลเพิ่มขึ้น·en",
    "ต่อสัญญาเสร็จเรียบร้อย·en",
];
pub const ACK_FAR_DY: u32 = 340;
/// The contract window's "Cancel | Renew" only counts as the pair when the
/// two buttons are level within this many pixels (CONTRACT_PAIR_DY)...
pub const CONTRACT_PAIR_DY: u32 = 4;
/// ...and this far apart sideways (CONTRACT_PAIR_GAP, both read by
/// `_contract_choice` sub_18006cff0).
pub const CONTRACT_PAIR_GAP: (u32, u32) = (35, 85);

/// Stuck: the same button pressed `stuck_clicks` times without vanishing,
/// for at least STUCK_MIN_SECONDS.
#[allow(dead_code)] // used by the real game backend (TODO in backend.rs)
pub fn is_stuck(repeats: u32, stuck_clicks: u32, since_secs: f64) -> bool {
    repeats >= stuck_clicks && since_secs >= STUCK_MIN_SECONDS
}

/// Whether two found buttons form the contract window's pair.
#[allow(dead_code)] // used by the real game backend (TODO in backend.rs)
pub fn is_contract_pair(dx: u32, dy: u32) -> bool {
    dy <= CONTRACT_PAIR_DY && (CONTRACT_PAIR_GAP.0..=CONTRACT_PAIR_GAP.1).contains(&dx)
}

/// Largest distance below `title` at which its "Accept" is pressed.
pub fn ack_max_dy(title: &str) -> u32 {
    if ACK_FAR_TITLES.contains(&title) {
        ACK_FAR_DY
    } else {
        ACK_MAX_DY
    }
}

/// Whether a window scale can be used, and whether a picture cut at
/// `picture_scale` fits it without resizing.
#[allow(dead_code)] // used by the real game backend (TODO in backend.rs)
pub fn scale_ok(scale: f32) -> bool {
    (SCALE_RANGE.0..=SCALE_RANGE.1).contains(&scale)
}
#[allow(dead_code)] // used by the real game backend (TODO in backend.rs)
pub fn same_scale(scale: f32, picture_scale: f32) -> bool {
    (scale - picture_scale).abs() <= SCALE_NEAR
}

/// Seconds a click step takes at least: hover, then settle.
pub fn min_click_secs() -> f64 {
    HOVER_BEFORE_CLICK_SECS + MIN_SETTLE_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert!(!is_stuck(8, 8, 30.0));
        assert!(is_stuck(8, 8, 60.0));
        assert!(is_contract_pair(50, 3));
        assert!(!is_contract_pair(90, 0));
        assert!(!is_contract_pair(50, 5));
        assert_eq!(ack_max_dy("ผู้จัดการเลเวลเพิ่มขึ้น"), 340);
        assert_eq!(ack_max_dy("x"), 140);
        assert!(scale_ok(1.875) && !scale_ok(3.5));
        assert!(same_scale(1.67, 1.66) && !same_scale(1.6, 1.67));
    }
}
