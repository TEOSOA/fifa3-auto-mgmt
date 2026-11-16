//! The real bot: look at the game window, find the button pictures,
//! click them. Each run (league or buying) is a worker thread that sends
//! what it did to the panel as `Event`s; the panel only shows them.
//!
//! Pictures load from the `templates` folder beside the program, the same
//! layout the original keeps inside itself (MECHANICS.md, "pictures"):
//! Thai in `templates/`, English and Vietnamese in `templates/en|vi/`,
//! bigger copies in `hires/` named `@scale`. The original's pictures were
//! cut from a game 480 px tall; the buying pictures from a game 1125 px
//! tall (the person's screenshots). The game is looked at shrunk to
//! WORK_H_LEAGUE (league) or at most WORK_H_BUY (buying) pixels tall, and
//! each picture's size is found once (a sweep around the expected size,
//! like the original's SCALE_RANGE search) and kept while the window keeps
//! its size.

use crate::bot;
use crate::buttons::{self, BuyButton, Role};
use crate::game::{self, Hwnd};
use crate::matcher;
use crate::vision::{self, Gray, Hit};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Height the original's pictures were cut at (VSCOM matched at ×2.39 in a
/// game 1125 px tall, attachments/picture-14; hires names @1.20..2.23 fit).
pub const REF_H_ORIGINAL: f32 = 480.0;
/// Height of the game in the person's screenshots the buying pictures were
/// cut from (attachments/picture-4..12).
pub const REF_H_BUY: f32 = 1125.0;
/// The league screen is looked at this tall (fast, still sharper than the
/// original's 480 px pictures).
pub const WORK_H_LEAGUE: usize = 540;
/// The buying screen at most this tall (the level badges are small).
pub const WORK_H_BUY: usize = 1125;
/// Sizes tried around the expected one before a picture's size is known.
const SWEEP: [f32; 11] = [1.0, 0.97, 1.03, 0.94, 1.06, 0.91, 1.09, 0.88, 1.12, 0.85, 1.15];
/// Once one picture's size is known, others try only these around it.
const NEAR: [f32; 3] = [1.0, 0.97, 1.03];
/// How long to wait for the next window of a purchase before starting over.
const BUY_WAIT_SECS: f64 = 6.0;
/// How long to wait for the pop-up after confirming.
const POPUP_WAIT_SECS: f64 = 8.0;
/// How long to wait for each menu on the way to the upgrade page.
const NAV_WAIT_SECS: f64 = 10.0;

/// Something the panel should tell the person.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Problem {
    NoGame,
    Minimized,
    NoTemplates,
    NoLevelPicture,
}

static PROBLEM: Mutex<Option<Problem>> = Mutex::new(None);

pub fn problem() -> Option<Problem> {
    *PROBLEM.lock().unwrap()
}

fn set_problem(p: Option<Problem>) {
    *PROBLEM.lock().unwrap() = p;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// A league button was clicked and went away.
    Clicked(&'static str),
    /// Buying: now at this step of `buyer::BUY_STEPS`.
    BuyStep(usize),
    /// On the way to the upgrade page: at this step of `buyer::NAV_STEPS`.
    NavStep(usize),
    Bought,
    TryAgain,
    /// The run did all it was asked.
    Done,
    /// The run stopped itself: stuck, or something it needs is missing.
    Stalled,
}

/// A running worker. Dropping it stops it.
pub struct Worker {
    stop: Arc<AtomicBool>,
    rx: Receiver<Event>,
}

impl Worker {
    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        set_problem(None);
    }
}

fn spawn(f: impl FnOnce(&AtomicBool, &Sender<Event>) + Send + 'static) -> Option<Worker> {
    if !cfg!(target_os = "windows") {
        return None; // other systems run the panel's demo
    }
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel();
    let s = stop.clone();
    std::thread::spawn(move || f(&s, &tx));
    Some(Worker { stop, rx })
}

// ------------------------------------------------------------------ log

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// One line in FiFAO3Mgmt-log.txt beside the program (what was seen and
/// clicked, to find out why a run didn't do what was expected).
pub fn log(line: &str) {
    use std::io::Write;
    let path = exe_dir().join("FiFAO3Mgmt-log.txt");
    // Start over when it gets big.
    let append = std::fs::metadata(&path).map_or(false, |m| m.len() < 2_000_000);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(append).write(true).truncate(!append).open(&path) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let _ = writeln!(f, "{:02}:{:02}:{:02} {}", t / 3600 % 24, t / 60 % 60, t % 60, line);
    }
}

// ------------------------------------------------------------- pictures

fn templates_dir() -> Option<PathBuf> {
    [exe_dir().join("templates"), PathBuf::from("templates"), PathBuf::from("windows/templates")]
        .into_iter()
        .find(|d| d.is_dir())
}

/// One picture to look for, ready at the size it was found at.
struct Pic {
    base: Gray,
    /// Game height the picture was cut at.
    ref_h: f32,
    threshold: f32,
    /// The picture resized to the game, once its size is known.
    ready: Option<(Gray, f32)>,
}

/// A place a picture was found, in game-window pixels.
#[derive(Clone, Copy, Debug)]
pub struct Found {
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
    pub score: f32,
    /// Game pixels per picture pixel at REF_H_ORIGINAL (for the pop-up rules).
    pub scale: f32,
}

/// The game window shrunk to the working height.
struct Screen {
    gray: Gray,
    /// Game pixels per working pixel.
    ratio: f32,
    client: (u32, u32),
}

fn grab(h: Hwnd, max_h: usize) -> Option<Screen> {
    let f = game::capture(h)?;
    Some(shrink(Gray::from_bgra(f.w, f.h, &f.bgra), max_h))
}

fn shrink(full: Gray, max_h: usize) -> Screen {
    let (w, h) = (full.w, full.h);
    let wh = max_h.min(h);
    let ww = ((w as f32) * wh as f32 / h as f32).round().max(1.0) as usize;
    let gray = if wh == h { full } else { full.resize(ww, wh) };
    Screen { ratio: h as f32 / gray.h as f32, client: (w as u32, h as u32), gray }
}

struct Library {
    pics: HashMap<String, Pic>,
    /// Size factor found for each kind of picture (by ref height).
    group: HashMap<u32, f32>,
    work_h: usize,
    client: (u32, u32),
}

impl Library {
    fn new(work_h: usize) -> Self {
        Self { pics: HashMap::new(), group: HashMap::new(), work_h, client: (0, 0) }
    }

    /// Load a picture by its path in the original's form
    /// ("templates/next.png"), in the game's language, falling back to Thai.
    /// Among it and its hires copies the one nearest the working size wins.
    fn add(&mut self, key: &str, image: &str, lang_dir: &str, ref_h: f32, threshold: f32) -> bool {
        let Some(root) = templates_dir() else { return false };
        let localized = matcher::template_path(lang_dir, image);
        let rel = [localized.as_str(), image]
            .into_iter()
            .find(|p| root.join(p.trim_start_matches("templates/")).is_file());
        let Some(rel) = rel else { return false };
        let rel = rel.trim_start_matches("templates/");
        let path = root.join(rel);
        let mut best: Option<(Gray, f32)> = vision::Gray::load(&path).map(|g| (g, ref_h));
        // hires copies: <dir>/hires/<name>@<scale>.png
        if let (Some(dir), Some(stem)) = (path.parent(), path.file_stem().and_then(|s| s.to_str())) {
            if let Ok(rd) = std::fs::read_dir(dir.join(matcher::HIRES_DIR)) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let Some(s) = name.strip_prefix(stem).and_then(|r| r.strip_prefix('@')).and_then(|r| r.strip_suffix(".png")).and_then(|r| r.parse::<f32>().ok()) else { continue };
                    let rh = ref_h * s;
                    let better = best.as_ref().map_or(true, |(_, b)| {
                        (rh / self.work_h as f32).ln().abs() < (*b / self.work_h as f32).ln().abs()
                    });
                    if better {
                        if let Some(g) = vision::Gray::load(&e.path()) {
                            best = Some((g, rh));
                        }
                    }
                }
            }
        }
        let Some((base, ref_h)) = best else { return false };
        self.pics.insert(key.to_string(), Pic { base, ref_h, threshold, ready: None });
        true
    }

    fn has(&self, key: &str) -> bool {
        self.pics.contains_key(key)
    }

    /// Forget the sizes when the game window changes size.
    fn fit(&mut self, s: &Screen) {
        if s.client != self.client {
            self.client = s.client;
            self.group.clear();
            for p in self.pics.values_mut() {
                p.ready = None;
            }
        }
    }

    /// Every place `key` shows on the screen (best first), or inside
    /// `area` (working pixels: x, y, w, h) only.
    fn find(&mut self, s: &Screen, key: &str, max_hits: usize, area: Option<(usize, usize, usize, usize)>) -> Vec<Found> {
        let Some(p) = self.pics.get_mut(key) else { return Vec::new() };
        let (img, ox, oy) = match area {
            Some((x, y, w, h)) => (s.gray.crop(x, y, w, h), x.min(s.gray.w), y.min(s.gray.h)),
            None => (s.gray.clone(), 0, 0),
        };
        let expected = s.gray.h as f32 / p.ref_h;
        let th_for = |f: f32, th: f32| if (expected * f - 1.0).abs() < 0.03 { th } else { matcher::scaled_threshold(th) };
        let hits: Vec<(Hit, f32)> = if let Some((g, f)) = &p.ready {
            vision::find_all(&img, g, th_for(*f, p.threshold), max_hits).into_iter().map(|h| (h, *f)).collect()
        } else {
            let gk = p.ref_h as u32;
            let tries: Vec<f32> = match self.group.get(&gk) {
                Some(g) => NEAR.iter().map(|n| n * g).collect(),
                None => SWEEP.to_vec(),
            };
            let mut best: Option<(Hit, f32, Gray)> = None;
            for f in tries {
                let g = p.base.scaled(expected * f);
                if let Some(h) = vision::find_best(&img, &g) {
                    if best.as_ref().map_or(true, |b| h.score > b.0.score) {
                        best = Some((h, f, g));
                    }
                }
            }
            match best {
                Some((h, f, g)) if h.score >= th_for(f, p.threshold) => {
                    log(&format!("size of {key}: x{:.3} (score {:.3})", expected * f, h.score));
                    self.group.entry(gk).or_insert(f);
                    let hits = vision::find_all(&img, &g, th_for(f, p.threshold), max_hits);
                    p.ready = Some((g, f));
                    hits.into_iter().map(|h| (h, f)).collect()
                }
                _ => Vec::new(),
            }
        };
        let r = s.ratio;
        hits.into_iter()
            .map(|(h, f)| Found {
                cx: (ox + h.x) as f32 * r + h.w as f32 * r / 2.0,
                cy: (oy + h.y) as f32 * r + h.h as f32 * r / 2.0,
                w: h.w as f32 * r,
                h: h.h as f32 * r,
                score: h.score,
                scale: expected * f * r * p.ref_h / REF_H_ORIGINAL,
            })
            .collect()
    }

    fn find_one(&mut self, s: &Screen, key: &str) -> Option<Found> {
        self.find(s, key, 1, area_of(key, s)).into_iter().next()
    }

    /// Whether `key` is still where `at` was.
    fn still_there(&mut self, s: &Screen, key: &str, at: &Found) -> bool {
        let r = s.ratio;
        let (w, h) = (at.w / r, at.h / r);
        let x = ((at.cx / r) - w * 1.5).max(0.0) as usize;
        let y = ((at.cy / r) - h * 1.5).max(0.0) as usize;
        !self.find(s, key, 1, Some((x, y, (w * 3.0) as usize + 2, (h * 3.0) as usize + 2))).is_empty()
    }
}

/// Where on the game a buying picture can be (share of width and height:
/// x, y, w, h), from the person's screenshots. Elsewhere look-alikes show:
/// a "ซื้อ" in the top-left menu scores 0.89-0.92 against the confirm
/// window's, and the chosen player's card on the right has a badge too.
fn area_of(key: &str, s: &Screen) -> Option<(usize, usize, usize, usize)> {
    let (fx, fy, fw, fh) = match key {
        "badge" => (0.0, 0.0, 0.55, 1.0),
        "BuyBottom" => (0.5, 0.7, 0.5, 0.3),
        // "Team" also shows in the top bar of the Team screens; "ระดับ" also
        // heads a column on the player page (picture-9).
        "NavTeam" => (0.3, 0.88, 0.4, 0.12),
        "PlayerManageTab" => (0.0, 0.0, 1.0, 0.25),
        "LevelTab" => (0.0, 0.0, 0.5, 0.3),
        "BuyConfirm" | "Accept" | "AcceptBusy" | "BoughtTitle" | "TryAgainMessage" => (0.2, 0.25, 0.6, 0.7),
        _ => return None,
    };
    let (w, h) = (s.gray.w as f32, s.gray.h as f32);
    Some(((fx * w) as usize, (fy * h) as usize, (fw * w) as usize, (fh * h) as usize))
}

fn click(h: Hwnd, at: &Found, real: bool, why: &str) {
    log(&format!("click {why} at {:.0},{:.0} (score {:.3}{})", at.cx, at.cy, at.score, if real { ", real mouse" } else { "" }));
    if !game::click(h, at.cx.round() as i32, at.cy.round() as i32, real) {
        // The original's wake-up message: another window covers the game.
        log("มีหน้าต่างอื่นบัง กดไม่ได้");
    }
}

/// Wait for the game window and a picture of it; None when asked to stop.
fn next_screen(stop: &AtomicBool, max_h: usize, hwnd: &mut Option<Hwnd>) -> Option<(Hwnd, Screen)> {
    loop {
        if stop.load(Ordering::SeqCst) {
            return None;
        }
        if hwnd.map_or(true, |h| !game::alive(h)) {
            *hwnd = game::find_window();
        }
        let Some(h) = *hwnd else {
            set_problem(Some(Problem::NoGame));
            sleep(Duration::from_secs(1));
            continue;
        };
        match game::client_size(h) {
            Some((w, hh)) if w > 100 && hh > 100 => {}
            _ => {
                set_problem(Some(Problem::Minimized));
                sleep(Duration::from_secs(1));
                continue;
            }
        }
        match grab(h, max_h) {
            Some(s) => {
                set_problem(None);
                return Some((h, s));
            }
            None => sleep(Duration::from_millis(300)),
        }
    }
}

fn pause(stop: &AtomicBool, secs: f64) -> bool {
    let end = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < end {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        sleep(Duration::from_millis(20));
    }
    true
}

/// Remembers clicks that had no effect: the real mouse after
/// WAKE_AFTER_NO_EFFECT, stop when stuck (bot::is_stuck).
#[derive(Default)]
struct Stuck {
    key: String,
    repeats: u32,
    since: Option<Instant>,
    no_effect: u32,
}

impl Stuck {
    fn wake(&self) -> bool {
        self.no_effect >= bot::WAKE_AFTER_NO_EFFECT
    }
    /// Note a click on `key` and whether it worked; true = stuck.
    fn note(&mut self, key: &str, worked: bool) -> bool {
        if worked {
            *self = Stuck::default();
            return false;
        }
        self.no_effect += 1;
        if self.key != key {
            self.key = key.to_string();
            self.repeats = 0;
            self.since = Some(Instant::now());
        }
        self.repeats += 1;
        let secs = self.since.map_or(0.0, |t| t.elapsed().as_secs_f64());
        bot::is_stuck(self.repeats, buttons::STUCK_CLICKS, secs)
    }
}

// --------------------------------------------------------------- league

pub struct LeagueCfg {
    pub lang_dir: &'static str,
    pub renew_contracts: bool,
}

pub fn start_league(cfg: LeagueCfg) -> Option<Worker> {
    spawn(move |stop, tx| league(cfg, stop, tx))
}

/// The pop-up title a "ยอมรับ" may sit under, in the original's names.
fn ack_title(key: &str, lang_dir: &str) -> &'static str {
    match (key, lang_dir) {
        ("level_up_title", "vi") => bot::ACK_FAR_TITLES[1],
        ("level_up_title", "en") => bot::ACK_FAR_TITLES[2],
        ("level_up_title", _) => bot::ACK_FAR_TITLES[0],
        ("contract_done_title", "en") => bot::ACK_FAR_TITLES[3],
        _ => "",
    }
}

fn league(cfg: LeagueCfg, stop: &AtomicBool, tx: &Sender<Event>) {
    let mut lib = Library::new(WORK_H_LEAGUE);
    for b in buttons::BUTTONS.iter() {
        lib.add(b.key, b.image, cfg.lang_dir, REF_H_ORIGINAL, b.threshold);
    }
    log(&format!("league start: {} of {} pictures", lib.pics.len(), buttons::BUTTONS.len()));
    if lib.pics.is_empty() {
        set_problem(Some(Problem::NoTemplates));
        let _ = tx.send(Event::Stalled);
        return;
    }
    let mut hwnd = None;
    let mut last: HashMap<&'static str, Instant> = HashMap::new();
    let mut stuck = Stuck::default();
    let recent = |last: &HashMap<&'static str, Instant>, k: &str, secs: f64| {
        last.get(k).is_some_and(|t| t.elapsed().as_secs_f64() < secs)
    };
    while let Some((h, s)) = next_screen(stop, WORK_H_LEAGUE, &mut hwnd) {
        lib.fit(&s);
        let mut seen: Vec<(&'static buttons::Button, Found)> = Vec::new();
        for b in buttons::BUTTONS.iter() {
            if let Some(f) = lib.find_one(&s, b.key) {
                seen.push((b, f));
            }
        }
        let get = |k: &str| seen.iter().find(|(b, _)| b.key == k).map(|(b, f)| (*b, *f));
        // 1. Contract window: Renew when auto-renew is on and it's not
        //    greyed out, otherwise Cancel (`_contract_choice`).
        let contract = if cfg.renew_contracts && get("contract_renew_off").is_none() {
            get("contract_renew").or_else(|| get("contract_cancel"))
        } else {
            get("contract_cancel")
        };
        // 2. "ยอมรับ" only under a known title, not again within 6 s.
        let ack = get("accept").filter(|_| !recent(&last, "accept", bot::REPEAT_COOLDOWN_SECS)).filter(|(_, a)| {
            seen.iter().any(|(t, f)| {
                t.role == Role::AckTitle
                    && bot::ack_under_title(ack_title(t.key, cfg.lang_dir), a.cx - f.cx, a.cy - f.cy, f.scale)
            })
        });
        // 3. Otherwise the step button with the smallest priority number.
        let step = seen
            .iter()
            .filter(|(b, _)| matches!(b.role, Role::None | Role::Pick))
            .filter(|(b, _)| b.role != Role::Pick || !recent(&last, b.key, bot::PICK_COOLDOWN_SECS))
            .min_by(|a, b| a.0.priority.cmp(&b.0.priority).then(b.1.score.total_cmp(&a.1.score)))
            .map(|(b, f)| (*b, *f));
        let Some((b, at)) = contract.or(ack).or(step) else {
            if !pause(stop, buttons::SCAN_INTERVAL_SECS) {
                break;
            }
            continue;
        };
        click(h, &at, stuck.wake(), b.key);
        last.insert(b.key, Instant::now());
        // Wait until it's gone, at most its delay_after.
        if !pause(stop, bot::MIN_SETTLE_SECS) {
            break;
        }
        let end = Instant::now() + Duration::from_secs_f64(b.delay_after as f64);
        let mut gone = false;
        while Instant::now() < end && !stop.load(Ordering::SeqCst) {
            let Some(s2) = grab(h, WORK_H_LEAGUE) else { break };
            if !lib.still_there(&s2, b.key, &at) {
                gone = true;
                break;
            }
            sleep(Duration::from_secs_f64(bot::POLL_INTERVAL_SECS));
        }
        if gone {
            let _ = tx.send(Event::Clicked(b.key));
        }
        if stuck.note(b.key, gone) {
            log(&format!("stuck on {}", b.key));
            let _ = tx.send(Event::Stalled);
            return;
        }
    }
}

// --------------------------------------------------------------- buying

pub struct BuyCfg {
    pub lang_dir: &'static str,
    pub level: u8,
    /// Players still to buy.
    pub remaining: u32,
    pub goto_upgrade: bool,
}

pub fn start_buy(cfg: BuyCfg) -> Option<Worker> {
    spawn(move |stop, tx| buy(cfg, stop, tx))
}

fn key_of(b: BuyButton) -> String {
    format!("{b:?}")
}

/// Wait until one of `keys` shows; returns which and where.
fn wait_for(
    stop: &AtomicBool,
    lib: &mut Library,
    hwnd: &mut Option<Hwnd>,
    keys: &[&str],
    secs: f64,
) -> Option<(Hwnd, String, Found, Screen)> {
    let end = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < end {
        let (h, s) = next_screen(stop, WORK_H_BUY, hwnd)?;
        lib.fit(&s);
        for k in keys {
            if let Some(f) = lib.find_one(&s, k) {
                return Some((h, k.to_string(), f, s));
            }
        }
        sleep(Duration::from_secs_f64(buttons::SCAN_INTERVAL_SECS));
    }
    None
}

fn buy(cfg: BuyCfg, stop: &AtomicBool, tx: &Sender<Event>) {
    let mut lib = Library::new(WORK_H_BUY);
    let badge = BuyButton::LevelBadge.spec();
    let badge_img = badge.image.replace("{n}", &cfg.level.to_string());
    let has_badge = lib.add("badge", &badge_img, cfg.lang_dir, REF_H_BUY, badge.threshold);
    for b in [
        BuyButton::BuyBottom,
        BuyButton::BuyConfirm,
        BuyButton::Accept,
        BuyButton::BoughtTitle,
        BuyButton::TryAgainMessage,
        BuyButton::NavTeam,
        BuyButton::TeamManage,
        BuyButton::PlayerManageTab,
        BuyButton::LevelTab,
    ] {
        let sp = b.spec();
        lib.add(&key_of(b), sp.image, cfg.lang_dir, REF_H_BUY, sp.threshold);
    }
    // "ยอมรับ" is smaller in the "ข้อความ" window (picture-12) than in the
    // bought window (picture-7): a second picture for it.
    lib.add("AcceptBusy", "templates/buy/accept_busy.png", cfg.lang_dir, REF_H_BUY, BuyButton::Accept.spec().threshold);
    log(&format!("buy start: +{} x{}, {} pictures", cfg.level, cfg.remaining, lib.pics.len()));
    if !has_badge {
        set_problem(Some(Problem::NoLevelPicture));
        let _ = tx.send(Event::Stalled);
        return;
    }
    let needed = ["BuyBottom", "BuyConfirm", "Accept", "BoughtTitle"];
    if needed.iter().any(|k| !lib.has(k)) {
        set_problem(Some(Problem::NoTemplates));
        let _ = tx.send(Event::Stalled);
        return;
    }
    let mut hwnd = None;
    let mut bought = 0;
    let mut stuck = Stuck::default();
    let mut busy_streak = Stuck::default();
    while bought < cfg.remaining {
        // 1. A row with the +N badge (the top one).
        let _ = tx.send(Event::BuyStep(0));
        let Some((h, s)) = next_screen(stop, WORK_H_BUY, &mut hwnd) else { return };
        lib.fit(&s);
        let mut rows = lib.find(&s, "badge", 12, area_of("badge", &s));
        rows.sort_by(|a, b| a.cy.total_cmp(&b.cy));
        let Some(row) = rows.first().copied() else {
            if !pause(stop, buttons::SCAN_INTERVAL_SECS * 5.0) {
                return;
            }
            continue;
        };
        // 2. Select it.
        let _ = tx.send(Event::BuyStep(1));
        click(h, &row, stuck.wake(), "row");
        if !pause(stop, badge.delay_after as f64 * 0.5) {
            return;
        }
        // 3. "ซื้อ" at the bottom (only once it's lit: greyed scores ~0.6).
        let _ = tx.send(Event::BuyStep(2));
        let Some((h, _, at, _)) = wait_for(stop, &mut lib, &mut hwnd, &["BuyBottom"], BUY_WAIT_SECS) else {
            if stop.load(Ordering::SeqCst) || stuck.note("BuyBottom", false) {
                return stall(tx, stop);
            }
            continue;
        };
        click(h, &at, stuck.wake(), "buy");
        if !pause(stop, bot::MIN_SETTLE_SECS) {
            return;
        }
        // 4. "ซื้อ" in the confirm window.
        let _ = tx.send(Event::BuyStep(3));
        let Some((h, _, at, _)) = wait_for(stop, &mut lib, &mut hwnd, &["BuyConfirm"], BUY_WAIT_SECS) else {
            if stop.load(Ordering::SeqCst) || stuck.note("BuyConfirm", false) {
                return stall(tx, stop);
            }
            continue;
        };
        click(h, &at, stuck.wake(), "confirm");
        stuck.note("", true);
        if !pause(stop, bot::MIN_SETTLE_SECS) {
            return;
        }
        // 5. Read the pop-up, close it with "ยอมรับ".
        let _ = tx.send(Event::BuyStep(4));
        let keys = ["BoughtTitle", "TryAgainMessage", "AcceptBusy", "Accept"];
        let Some((h, which, _, s)) = wait_for(stop, &mut lib, &mut hwnd, &keys, POPUP_WAIT_SECS) else {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            log("no pop-up after confirming");
            continue;
        };
        // Let the pop-up finish drawing, then look at it once more.
        let s = if pause(stop, 0.15) { grab(h, WORK_H_BUY).unwrap_or(s) } else { return };
        let bought_btn = lib.find_one(&s, "BoughtTitle");
        let busy_text = lib.find_one(&s, "TryAgainMessage");
        let accept = lib.find_one(&s, "Accept");
        let accept_busy = lib.find_one(&s, "AcceptBusy");
        // "ส่งเข้าทีม" sits beside "ยอมรับ" only in the bought window; the
        // busy text sits above "ยอมรับ" (picture-12).
        let near = |a: &Option<Found>, b: &Option<Found>| match (a, b) {
            (Some(a), Some(b)) => (a.cy - b.cy).abs() < a.h.max(b.h) * 1.5,
            _ => false,
        };
        let was_bought = bought_btn.is_some() && (accept.is_none() || near(&bought_btn, &accept)) && busy_text.is_none();
        log(&format!(
            "pop-up ({which}): bought {:?} busy {:?} accept {:?} accept_busy {:?} -> {}",
            bought_btn.map(|f| f.score),
            busy_text.map(|f| f.score),
            accept.map(|f| f.score),
            accept_busy.map(|f| f.score),
            if was_bought { "bought" } else { "not bought" }
        ));
        let close = if was_bought { accept.or(accept_busy) } else { accept_busy.or(accept) };
        if let Some(at) = close {
            click(h, &at, false, "accept");
            pause(stop, BuyButton::Accept.spec().delay_after as f64);
        }
        if was_bought {
            bought += 1;
            busy_streak = Stuck::default();
            let _ = tx.send(Event::Bought);
        } else {
            let _ = tx.send(Event::TryAgain);
            if busy_streak.note("busy", false) {
                return stall(tx, stop);
            }
            if !pause(stop, crate::buyer::RETRY_WAIT_SECS) {
                return;
            }
        }
    }
    if cfg.goto_upgrade {
        for (i, (_, b)) in crate::buyer::NAV_STEPS.iter().enumerate() {
            let _ = tx.send(Event::NavStep(i));
            let k = key_of(*b);
            let Some((h, _, at, _)) = wait_for(stop, &mut lib, &mut hwnd, &[k.as_str()], NAV_WAIT_SECS) else {
                return stall(tx, stop);
            };
            click(h, &at, false, &k);
            if !pause(stop, b.spec().delay_after as f64) {
                return;
            }
        }
    }
    let _ = tx.send(Event::Done);
}

fn stall(tx: &Sender<Event>, stop: &AtomicBool) {
    if !stop.load(Ordering::SeqCst) {
        let _ = tx.send(Event::Stalled);
    }
}

// ------------------------------------------------------------ the panel

/// The game window's inside size, looked up at most once a second.
pub fn game_size() -> Option<(u32, u32)> {
    static CACHE: Mutex<Option<(Instant, Option<(u32, u32)>)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap();
    if let Some((t, v)) = *c {
        if t.elapsed() < Duration::from_secs(1) {
            return v;
        }
    }
    let v = game::find_window().and_then(game::client_size);
    *c = Some((Instant::now(), v));
    v
}

/// `--scan <screen.png> league|buy [level]`: what the bot would see on a
/// saved picture of the game (checks the pictures and sizes without it).
pub fn scan_cli(args: &[String]) {
    let Some(img) = args.get(2).and_then(|p| Gray::load(p.as_ref())) else {
        eprintln!("usage: --scan <screen.png> league|buy [level]");
        return;
    };
    let buy = args.get(3).map(String::as_str) == Some("buy");
    let level: u8 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(1);
    let s = shrink(img, if buy { WORK_H_BUY } else { WORK_H_LEAGUE });
    let mut lib = Library::new(s.gray.h);
    let mut keys: Vec<String> = Vec::new();
    if buy {
        let b = BuyButton::LevelBadge.spec();
        lib.add("badge", &b.image.replace("{n}", &level.to_string()), "", REF_H_BUY, b.threshold);
        keys.push("badge".into());
        for b in [BuyButton::BuyBottom, BuyButton::BuyConfirm, BuyButton::Accept, BuyButton::BoughtTitle, BuyButton::TryAgainMessage, BuyButton::NavTeam, BuyButton::TeamManage, BuyButton::PlayerManageTab, BuyButton::LevelTab] {
            let sp = b.spec();
            lib.add(&key_of(b), sp.image, "", REF_H_BUY, sp.threshold);
            keys.push(key_of(b));
        }
        lib.add("AcceptBusy", "templates/buy/accept_busy.png", "", REF_H_BUY, BuyButton::Accept.spec().threshold);
        keys.push("AcceptBusy".into());
    } else {
        for b in buttons::BUTTONS.iter() {
            lib.add(b.key, b.image, "", REF_H_ORIGINAL, b.threshold);
            keys.push(b.key.into());
        }
    }
    lib.fit(&s);
    println!("screen {}x{}, working height {}, {} pictures", s.client.0, s.client.1, s.gray.h, lib.pics.len());
    for k in keys {
        for f in lib.find(&s, &k, 12, area_of(&k, &s)) {
            println!("{k:16} at {:5.0},{:5.0} score {:.3} scale {:.2}", f.cx, f.cy, f.score, f.scale);
        }
    }
}
