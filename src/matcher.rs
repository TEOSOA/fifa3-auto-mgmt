//! How the original finds a button picture on the game screen — module
//! `fo3bot.matcher` of the unpacked main.dll (MECHANICS.md, "round 5").
//!
//! Values come from the module's constants blob (file offset 0x114023e,
//! 151 constants) and its init code `sub_180165550`, which stores each
//! one under its name (`mov rdx,[name]; mov r8,[value]; call
//! sub_1801673c0` at 0x180166dcb..0x180166f72). The scaling rule is the
//! compiled body of `scale_template` (`sub_18016bac0`, source line 67).

/// `Template.threshold` default (constant 0.85, class at source line 37).
pub const DEFAULT_THRESHOLD: f32 = 0.85;

/// A picture resized to fit the game window may be matched a little more
/// loosely, but never below this (SCALED_FLOOR = 0.75, slot 0x1813d71e8).
pub const SCALED_FLOOR: f32 = 0.75;
/// ...by this much (SCALED_SLACK = 0.04, slot 0x1813d71f0).
pub const SCALED_SLACK: f32 = 0.04;

/// Large screens are first searched at reduced size: halve the frame
/// (COARSE = 2), in grey (COARSE_GRAY = True), when it has at least this
/// many pixels (COARSE_MIN_AREA = 350000), keep the best COARSE_PEAKS = 3
/// spots and re-check each at full size with a COARSE_MARGIN = 0.15 border.
pub const COARSE: u32 = 2;
pub const COARSE_GRAY: bool = true;
pub const COARSE_MIN_AREA: u64 = 350_000;
pub const COARSE_PEAKS: usize = 3;
pub const COARSE_MARGIN: f32 = 0.15;

/// A window narrower than 4:3 may cut off the game's right side
/// (`layout_is_clipped`, constants 4 and 3, source line 138); then a
/// button may also be found with part of it missing on the right.
pub const CLIP_EDGE_GAP: f32 = 0.05;
pub const CLIP_SLACK: f32 = 0.04;

/// Picture names may carry the scale they were cut at:
/// `<dir>/hires/<name>@<scale>.png` (regex at 0x181122c09, fo3bot.bot).
pub const HIRES_DIR: &str = "hires";

/// Size of a picture resized by (sx, sy): `max(1, round(w*sx))`,
/// `max(1, round(h*sy))` (sub_18016bac0).
pub fn scaled_size(w: u32, h: u32, sx: f32, sy: f32) -> (u32, u32) {
    let s = |v: u32, f: f32| ((v as f32 * f).round() as u32).max(1);
    (s(w, sx), s(h, sy))
}

/// Shrinking uses area averaging, growing uses cubic (INTER_AREA /
/// INTER_CUBIC in sub_18016bac0).
pub fn shrinks(sx: f32, sy: f32) -> bool {
    sx < 1.0 && sy < 1.0
}

/// Threshold of a picture after resizing (`slack` on):
/// `max(min(th, SCALED_FLOOR), th - SCALED_SLACK)` (sub_18016bac0,
/// source line 78).
pub fn scaled_threshold(th: f32) -> f32 {
    th.min(SCALED_FLOOR).max(th - SCALED_SLACK)
}

/// Whether the game window is narrower than 4:3 (`W*3 < H*4`).
pub fn layout_is_clipped(w: u32, h: u32) -> bool {
    (w as u64) * 3 < (h as u64) * 4
}

/// Whether the screen is big enough for the reduced-size first look.
pub fn uses_coarse(w: u32, h: u32) -> bool {
    (w as u64) * (h as u64) >= COARSE_MIN_AREA
}

/// Folder of the pictures for a language: Thai ones sit in `templates/`,
/// English and Vietnamese in `templates/en/` and `templates/vi/`.
pub fn template_path(lang_dir: &str, image: &str) -> String {
    match (lang_dir, image.strip_prefix("templates/")) {
        ("", _) | (_, None) => image.to_string(),
        (dir, Some(rest)) => format!("templates/{}/{}", dir, rest),
    }
}

/// The scale written in a hires picture's name (`.../hires/x@1.875.png`).
pub fn hires_scale(path: &str) -> Option<f32> {
    let (dir, file) = path.rsplit_once('/')?;
    if !dir.ends_with(HIRES_DIR) {
        return None;
    }
    file.strip_suffix(".png")?.rsplit_once('@')?.1.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_rule() {
        assert!((scaled_threshold(0.85) - 0.81).abs() < 1e-6);
        assert!((scaled_threshold(0.78) - 0.75).abs() < 1e-6);
        assert!((scaled_threshold(0.70) - 0.70).abs() < 1e-6);
    }

    #[test]
    fn sizes_and_layout() {
        assert_eq!(scaled_size(100, 40, 1.875, 1.875), (188, 75));
        assert_eq!(scaled_size(1, 1, 0.1, 0.1), (1, 1));
        assert!(layout_is_clipped(1000, 800));
        assert!(!layout_is_clipped(1600, 900));
        assert!(uses_coarse(1600, 900));
    }

    #[test]
    fn paths() {
        assert_eq!(template_path("vi", "templates/next.png"), "templates/vi/next.png");
        assert_eq!(template_path("", "templates/next.png"), "templates/next.png");
        assert_eq!(hires_scale("templates/en/hires/next@1.875.png"), Some(1.875));
        assert_eq!(hires_scale("templates/next.png"), None);
    }
}
