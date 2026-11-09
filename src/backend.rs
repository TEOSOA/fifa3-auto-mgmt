//! Backend: real game interaction.
//!
//! On Windows this is where you drive FIFA Online 3: find the game window,
//! hide it without minimizing it (the original forbids minimizing), and
//! inject mouse/keyboard input with SendInput. The original does this from
//! a Python payload we can't see inside the packed exe, so the exact call
//! sequence is yours to implement — every hook is marked below.
//!
//! On every other platform the hooks are no-ops, so the app always runs
//! safely in demo mode.
//!
//! This module also mirrors the original Nuitka bootloader's watchdog
//! (MECHANICS.md, values 8-10). The values come from the decompiled
//! bootloader functions (see fnd_4, evidence ev_7e5aa81ef4b34f31…):
//!
//!   sub_140013e40 (wait + cleanup)
//!       WaitForSingleObject(hProc, 0x1388)  →  5000 ms
//!       SHFileOperationW retry loop: r14d = 0x14 →  20 retries
//!       Sleep(0x64)                        →  100 ms back-off
//!
//!   sub_140012b00 (temp-folder naming)
//!       GetTempPathW, GetCurrentProcessId, GetSystemTimeAsFileTime
//!       {TEMP}\onefile_{PID}_{TIME_US}_{RANDOM}

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Original bootloader `sub_140013e40`:
/// `WaitForSingleObject(hProc, 0x1388)` — total wait for the child process.
/// 0x1388 = 5000 decimal.
const CHILD_WAIT_TIMEOUT_MS: u64 = 5000;

/// Original bootloader `sub_140013e40`:
/// `r14d = 0x14` at the start of the SHFileOperationW retry loop.
/// 0x14 = 20 decimal.
const MAX_RETRY: u32 = 20;

/// Original bootloader `sub_140013e40`:
/// `Sleep(0x64)` inside the retry loop.
/// 0x64 = 100 decimal.
const RETRY_BACKOFF_MS: u64 = 100;

static RUNNING: AtomicBool = AtomicBool::new(false);
static FAILED: AtomicBool = AtomicBool::new(false);
static WORK_DIR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Apply the "hide game window" switch.
pub fn hide_game_window(hide: bool) {
    #[cfg(target_os = "windows")]
    {
        // TODO(you): find the FIFA Online 3 window (FindWindowW /
        // EnumWindows), then ShowWindow(hwnd, SW_HIDE/SW_SHOW). The game
        // keeps running and sending inputs while hidden.
        eprintln!("backend: hide game window -> {}", hide);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hide;
        eprintln!("backend (demo): hide game window is a no-op here");
    }
}

/// Called when the user starts the automation.
pub fn on_start(hide_window: bool) {
    use crate::buttons::*;
    eprintln!(
        "backend: game window \"{}\" ({}), capture {}, scan every {}s, stop after {} repeated clicks or {} min idle",
        WINDOW_TITLE, WINDOW_EXE, CAPTURE_METHOD, SCAN_INTERVAL_SECS, STUCK_CLICKS, IDLE_MINUTES
    );
    {
        use crate::matcher::*;
        let (w, h) = game_window_size().unwrap_or((0, 0));
        // Example: the largest hires scale in the original (EN @1.875).
        let hires = format!("templates/en/{}/next@1.875.png", HIRES_DIR);
        let s = hires_scale(&hires).unwrap_or(1.0);
        eprintln!(
            "backend: matcher default >= {:.2} ({:.2} if resized, floor {:.2}), first look at 1/{} size{} on screens >= {} px, {} best spots +{:.0}% margin; narrow window {} (edge gap {:.2}, slack {:.2}); 100x40 picture -> {:?} ({}), from {}",
            DEFAULT_THRESHOLD,
            scaled_threshold(DEFAULT_THRESHOLD),
            SCALED_FLOOR,
            COARSE,
            if COARSE_GRAY { " in grey" } else { "" },
            COARSE_MIN_AREA,
            COARSE_PEAKS,
            COARSE_MARGIN * 100.0,
            layout_is_clipped(w, h),
            CLIP_EDGE_GAP,
            CLIP_SLACK,
            scaled_size(100, 40, s, s),
            if shrinks(s, s) { "area" } else { "cubic" },
            hires,
        );
    }
    {
        use crate::bot::*;
        eprintln!(
            "backend: click = hover {}s + settle {}s, then look every {}s; wake the game after {} clicks with no effect; stuck = {} repeats and >= {}s (keep {} stuck pictures); window scale {}-{} (same picture within {})",
            HOVER_BEFORE_CLICK_SECS,
            MIN_SETTLE_SECS,
            POLL_INTERVAL_SECS,
            WAKE_AFTER_NO_EFFECT,
            crate::buttons::STUCK_CLICKS,
            STUCK_MIN_SECONDS,
            STUCK_KEEP,
            SCALE_RANGE.0,
            SCALE_RANGE.1,
            SCALE_NEAR
        );    }
    hide_game_window(hide_window);

    // Per-run working directory, mirroring the original's
    // {TEMP}\onefile_{pid}_... folder that the bootloader creates.
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("fifao3-mgmt_{}", pid));
    let made = std::fs::create_dir_all(&dir).is_ok();
    if made {
        *WORK_DIR.lock().unwrap() = Some(dir.to_string_lossy().into_owned());
    }

    RUNNING.store(true, Ordering::SeqCst);
    FAILED.store(false, Ordering::SeqCst);

    // Watchdog: poll the core with the original's 5000 ms budget; if the
    // core has signalled failure, retry up to 20 times (100 ms apart)
    // before reporting the run as stopped.
    // (original: sub_140013e40, r14d=0x14=20, Sleep(0x64=100))
    let watcher = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(CHILD_WAIT_TIMEOUT_MS);
        let mut retries = 0u32;
        while Instant::now() < deadline {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            if FAILED.load(Ordering::SeqCst) {
                if retries >= MAX_RETRY {
                    eprintln!("backend: core failed after {} retries; stopping", MAX_RETRY);
                    RUNNING.store(false, Ordering::SeqCst);
                    break;
                }
                retries += 1;
                eprintln!("backend: retrying core ({}/{})", retries, MAX_RETRY);
                thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS));
            } else {
                thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS / 2));
            }
        }
    });

    #[cfg(target_os = "windows")]
    {
        // TODO(you): locate the game, focus it once, then run the loop.
        eprintln!("backend: automation starting (watchdog budget {} ms)", CHILD_WAIT_TIMEOUT_MS);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hide_window;
        eprintln!("backend (demo): automation starting (watchdog budget {} ms)", CHILD_WAIT_TIMEOUT_MS);
    }
    let _ = watcher; // the loop runs until on_stop()
}

/// Called when the user stops.
pub fn on_stop() {
    RUNNING.store(false, Ordering::SeqCst);
    cleanup_work_dir();
    // Release any held keys (e.g. WASD) so the game doesn't keep moving.
    eprintln!("backend: automation stopped");
}

/// Remove the per-run working directory, mirroring the original's
/// SHFileOperationW retry loop (sub_140013e40, 20 retries, 100 ms).
fn cleanup_work_dir() {
    if let Some(path) = WORK_DIR.lock().unwrap().take() {
        let mut ok = false;
        for _ in 0..MAX_RETRY {
            if std::fs::remove_dir_all(&path).is_ok() {
                ok = true;
                break;
            }
            thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS));
        }
        if !ok {
            eprintln!("backend: could not delete {} after {} tries", path, MAX_RETRY);
        } else {
            eprintln!("backend: cleaned up {}", path);
        }
    }
}

/// Click one of the original's buttons in the game window.
/// On Windows: find `b.image` on the captured screen with a similarity of
/// at least `b.threshold`, post a click to its centre (the original's
/// default "postmessage" method, which doesn't move the real mouse), then
/// wait until it is gone or `b.delay_after` seconds have passed.
pub fn click_button(b: &crate::buttons::Button) {
    use crate::bot;
    use crate::buttons::Role;
    // TODO(you): screen capture + template matching + PostMessage click:
    // hover bot::HOVER_BEFORE_CLICK_SECS, click, wait bot::MIN_SETTLE_SECS,
    // then look every bot::POLL_INTERVAL_SECS until the button is gone.
    let rule = match b.role {
        Role::Renew | Role::Decline => format!(
            "only with its pair level within {} px and {}-{} px apart",
            bot::CONTRACT_PAIR_DY,
            bot::CONTRACT_PAIR_GAP.0,
            bot::CONTRACT_PAIR_GAP.1
        ),
        Role::Ack => format!(
            "only up to {} px below a known title ({} px for the level-up window)",
            bot::ACK_MAX_DY,
            bot::ack_max_dy(bot::ACK_FAR_TITLES[0])
        ),
        Role::Pick => format!("not again for {} s", bot::PICK_COOLDOWN_SECS),
        _ => String::new(),
    };
    if !rule.is_empty() {
        eprintln!("backend: {} {}", b.key, rule);
    }
    eprintln!(
        "backend: click {} [{}] (match >= {:.2}, {:.2} if resized, priority {}, role {:?}, wait up to {:.1}s)",
        b.key,
        b.image,
        b.threshold,
        crate::matcher::scaled_threshold(b.threshold),
        b.priority,
        b.role,
        b.delay_after
    );
}

/// The game window's size when it's found ("1600×900" on the original's
/// screen). The demo reports that size.
pub fn game_window_size() -> Option<(u32, u32)> {
    // TODO(you): FindWindowW(crate::buttons::WINDOW_TITLE) or the window of
    // crate::buttons::WINDOW_EXE, then GetClientRect.
    Some((1600, 900))
}

/// Whether this program runs with Administrator rights. The original's
/// manifest asks Windows for them (requireAdministrator), because clicks
/// don't reach a game that itself runs as Administrator otherwise.
pub fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "shell32")]
        extern "system" {
            fn IsUserAnAdmin() -> i32;
        }
        // SAFETY: IsUserAnAdmin takes no arguments and only reads the token.
        unsafe { IsUserAnAdmin() != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

/// Click one button of the player-buying feature. `level` is the +N the
/// person looks for; it picks the badge picture (level_plus{N}.png).
/// `lang_dir` picks the picture folder of the game's language, like the
/// original's templates/, templates/en/ and templates/vi/.
pub fn click_buy_button(b: crate::buttons::BuyButton, level: u8, lang_dir: &str) {
    use crate::matcher;
    let spec = b.spec();
    let image = matcher::template_path(lang_dir, &spec.image.replace("{n}", &level.to_string()));
    let (w, h) = game_window_size().unwrap_or((0, 0));
    // TODO(you): find `image` on the game screen with TM_CCOEFF_NORMED
    // (>= spec.threshold, or matcher::scaled_threshold when the picture
    // had to be resized to the window); for LevelBadge pick the first
    // matching row in the search results.
    eprintln!(
        "backend: buy click {:?} [{}] (match >= {:.2}, {:.2} if resized, wait {:.1}s, coarse {}, clipped {})",
        b,
        image,
        spec.threshold,
        matcher::scaled_threshold(spec.threshold),
        spec.delay_after,
        matcher::uses_coarse(w, h),
        matcher::layout_is_clipped(w, h)
    );
}
