//! The game window on Windows: find it, read its picture, click in it.
//!
//! Rebuilt from the original's `fo3bot.window`, `fo3bot.capture` and
//! `fo3bot.clicker` (module inits sub_18017c330, sub_1800814a0,
//! sub_1800956f0; their constants and Thai notes are listed in MECHANICS.md
//! round 12):
//! - find_window: skip "Ghost" windows (what Windows shows for a hung game),
//!   take the exact title first, then a title containing it, then the
//!   largest window of the game's program (fifazf.exe).
//! - capture: PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT); a picture
//!   whose brightest pixel is under 10 is "blank" (`is_blank`: img.max() <
//!   10) and the original then uses Windows Graphics Capture; this build
//!   copies the game's part of the screen instead.
//! - click ("postmessage", the default): to the child window under the
//!   point (ChildWindowFromPointEx + MapWindowPoints), WM_MOUSEMOVE, wait
//!   0.01 s, WM_LBUTTONDOWN, hold random.uniform(0.03, 0.08) s,
//!   WM_LBUTTONUP; the point moves by random.randint(-jitter, jitter),
//!   jitter 2 by default.
//! - "sendinput" (the wake-up): only when the game itself is under the
//!   point (WindowFromPoint → GetAncestor(GA_ROOT)), otherwise "มีหน้าต่าง
//!   อื่นบัง กดไม่ได้"; move the real mouse, press, release, move it back.
//! - hide: `hide_in_place` — nearly transparent, the mouse goes through it,
//!   at the bottom of the window stack (moving it off screen makes some
//!   game pages stop); `show_in_place` puts it back on top without focus.
//! On other systems every call answers "no game", so the app runs its demo.

// The values below are only used by the Windows half.
#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

/// A captured frame of the game window's inside (client area), BGRA.
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub bgra: Vec<u8>,
}

/// `is_blank`: a picture whose brightest value is under this is black
/// (fo3bot.capture constants at 0x181125017: `img.max() < 10`).
pub const BLANK_MAX: u8 = 10;
/// Hold the button this long (fo3bot.clicker `_hold_time`:
/// random.uniform(0.03, 0.08), constants at 0x18112788c / 0x181127895).
pub const HOLD_SECS: (f64, f64) = (0.03, 0.08);
/// Wait between the mouse move and the press (0x181127951: 0.01 s).
pub const MOVE_TO_PRESS_SECS: f64 = 0.01;
/// `click(..., jitter=2)` default (0x181128a09: defaults 'postmessage',
/// False, 2, ...): the point moves by up to 2 px each way.
pub const CLICK_JITTER: i32 = 2;
/// `_wake_game` sleeps 0.35 s after bringing the game forward, then clicks
/// with method="sendinput", jitter=0 (fo3bot.gui constants 0x1811307c5).
pub const WAKE_SETTLE_SECS: f64 = 0.35;

/// Small random numbers without another library.
fn rand_u32() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(0);
    let mut s = SEED.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos()) | 1;
    }
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    SEED.store(s, Ordering::Relaxed);
    s
}

/// random.uniform(a, b)
pub fn uniform(a: f64, b: f64) -> f64 {
    a + (b - a) * (rand_u32() as f64 / u32::MAX as f64)
}

/// random.randint(-j, j)
pub fn jitter(j: i32) -> i32 {
    if j <= 0 {
        0
    } else {
        (rand_u32() % (2 * j as u32 + 1)) as i32 - j
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::Frame;
    use std::ffi::c_void;

    pub type Hwnd = isize;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        xppm: i32,
        yppm: i32,
        clr_used: u32,
        clr_important: u32,
    }
    #[repr(C)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [u32; 1],
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MouseInput {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        flags: u32,
        time: u32,
        extra: usize,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Input {
        kind: u32,
        mi: MouseInput,
    }

    type EnumProc = unsafe extern "system" fn(Hwnd, isize) -> i32;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: EnumProc, lparam: isize) -> i32;
        fn GetWindowTextW(h: Hwnd, buf: *mut u16, n: i32) -> i32;
        fn GetClassNameW(h: Hwnd, buf: *mut u16, n: i32) -> i32;
        fn IsWindowVisible(h: Hwnd) -> i32;
        fn IsWindow(h: Hwnd) -> i32;
        fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
        fn ClientToScreen(h: Hwnd, p: *mut Point) -> i32;
        fn GetDC(h: Hwnd) -> isize;
        fn ReleaseDC(h: Hwnd, dc: isize) -> i32;
        fn PrintWindow(h: Hwnd, dc: isize, flags: u32) -> i32;
        fn PostMessageW(h: Hwnd, msg: u32, w: usize, l: isize) -> i32;
        fn SetCursorPos(x: i32, y: i32) -> i32;
        fn GetCursorPos(p: *mut Point) -> i32;
        fn SendInput(n: u32, inputs: *const Input, size: i32) -> u32;
        fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
        fn ChildWindowFromPointEx(h: Hwnd, p: Point, flags: u32) -> Hwnd;
        fn MapWindowPoints(from: Hwnd, to: Hwnd, pts: *mut Point, n: u32) -> i32;
        fn WindowFromPoint(p: Point) -> Hwnd;
        fn GetAncestor(h: Hwnd, flags: u32) -> Hwnd;
        fn GetWindowLongW(h: Hwnd, idx: i32) -> i32;
        fn SetWindowLongW(h: Hwnd, idx: i32, v: i32) -> i32;
        fn SetLayeredWindowAttributes(h: Hwnd, key: u32, alpha: u8, flags: u32) -> i32;
        fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
        fn SetForegroundWindow(h: Hwnd) -> i32;
        fn GetForegroundWindow() -> Hwnd;
        fn AttachThreadInput(a: u32, b: u32, attach: i32) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn CreateCompatibleBitmap(dc: isize, w: i32, h: i32) -> isize;
        fn SelectObject(dc: isize, obj: isize) -> isize;
        fn BitBlt(dst: isize, x: i32, y: i32, w: i32, h: i32, src: isize, sx: i32, sy: i32, rop: u32) -> i32;
        fn GetDIBits(dc: isize, bmp: isize, start: u32, lines: u32, bits: *mut c_void, bi: *mut BitmapInfo, usage: u32) -> i32;
        fn DeleteObject(obj: isize) -> i32;
        fn DeleteDC(dc: isize) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn QueryFullProcessImageNameW(p: isize, flags: u32, buf: *mut u16, n: *mut u32) -> i32;
        fn CloseHandle(h: isize) -> i32;
        fn GetCurrentProcessId() -> u32;
        fn GetCurrentThreadId() -> u32;
    }

    const WM_MOUSEMOVE: u32 = 0x0200;
    const WM_LBUTTONDOWN: u32 = 0x0201;
    const WM_LBUTTONUP: u32 = 0x0202;
    const MK_LBUTTON: usize = 1;
    const SRCCOPY: u32 = 0x00CC_0020;
    const PW_CLIENTONLY: u32 = 1;
    const PW_RENDERFULLCONTENT: u32 = 2;
    const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    const CWP_SKIPINVISIBLE: u32 = 1;
    const CWP_SKIPDISABLED: u32 = 2;
    const CWP_SKIPTRANSPARENT: u32 = 4;
    const GA_ROOT: u32 = 2;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_LAYERED: i32 = 0x0008_0000;
    const WS_EX_TRANSPARENT: i32 = 0x0000_0020;
    const LWA_ALPHA: u32 = 2;
    const HWND_BOTTOM: Hwnd = 1;
    const HWND_TOPMOST: Hwnd = -1;
    const HWND_NOTOPMOST: Hwnd = -2;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    /// hide_in_place (sub_1801931e0, line 0x117): "โปร่งใสเกือบหมด". Its
    /// alpha comes from the shared constant pair at 0x181365820 (+0x10),
    /// not this module's table, so 1 is still estimated.
    const HIDDEN_ALPHA: u8 = 1;
    /// show_in_place (sub_1801944f0): constant slot 0x1813d7b08 = 255.
    const SHOWN_ALPHA: u8 = 255;

    fn text16(f: unsafe extern "system" fn(Hwnd, *mut u16, i32) -> i32, h: Hwnd) -> String {
        let mut buf = [0u16; 256];
        // SAFETY: buf is valid for 256 u16s.
        let n = unsafe { f(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn pid_of(h: Hwnd) -> u32 {
        let mut pid = 0u32;
        // SAFETY: valid out-pointer.
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        pid
    }

    fn exe_name(h: Hwnd) -> String {
        // SAFETY: plain Win32 calls with valid out-pointers; the handle is closed.
        unsafe {
            let p = OpenProcess(0x1000, 0, pid_of(h)); // PROCESS_QUERY_LIMITED_INFORMATION
            if p == 0 {
                return String::new();
            }
            let mut buf = [0u16; 520];
            let mut n = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(p, 0, buf.as_mut_ptr(), &mut n);
            CloseHandle(p);
            if ok == 0 {
                return String::new();
            }
            let path = String::from_utf16_lossy(&buf[..n as usize]);
            path.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase()
        }
    }

    /// `list_windows`: visible top-level windows that have a title, not ours.
    unsafe extern "system" fn each(h: Hwnd, lparam: isize) -> i32 {
        let out = &mut *(lparam as *mut Vec<Hwnd>);
        if IsWindowVisible(h) != 0 && GetWindowTextW(h, std::ptr::null_mut(), 0) >= 0 {
            out.push(h);
        }
        1
    }

    /// Windows that look hidden to `list_windows` but were hidden in place
    /// by us are still visible (only transparent), so one list is enough.
    pub fn find_window(title: &str, title_part: &str, exe: &str) -> Option<Hwnd> {
        let mut all: Vec<Hwnd> = Vec::new();
        // SAFETY: the callback only pushes into the Vec passed by pointer.
        unsafe { EnumWindows(each, &mut all as *mut Vec<Hwnd> as isize) };
        let own = unsafe { GetCurrentProcessId() };
        let wins: Vec<(Hwnd, String)> = all
            .into_iter()
            .filter(|h| pid_of(*h) != own)
            .filter(|h| text16(GetClassNameW, *h) != "Ghost")
            .map(|h| (h, text16(GetWindowTextW, h)))
            .filter(|(_, t)| !t.is_empty())
            .collect();
        let part = title_part.to_lowercase();
        wins.iter()
            .find(|(_, t)| t == title)
            .or_else(|| wins.iter().find(|(_, t)| t.to_lowercase().contains(&part)))
            .map(|(h, _)| *h)
            .or_else(|| {
                // `_largest_game_window`: the game's program, biggest client.
                wins.iter()
                    .filter(|(h, _)| exe_name(*h) == exe)
                    .filter_map(|(h, _)| client_size(*h).map(|(w, hh)| (*h, w * hh, w, hh)))
                    .filter(|&(_, _, w, hh)| w >= super::MIN_GAME_WINDOW.0 && hh >= super::MIN_GAME_WINDOW.1)
                    .max_by_key(|&(_, a, _, _)| a)
                    .map(|(h, _, _, _)| h)
            })
    }

    pub fn alive(h: Hwnd) -> bool {
        // SAFETY: IsWindow accepts any value.
        unsafe { IsWindow(h) != 0 }
    }

    pub fn client_size(h: Hwnd) -> Option<(u32, u32)> {
        let mut r = Rect::default();
        // SAFETY: r is a valid out-pointer.
        if unsafe { GetClientRect(h, &mut r) } == 0 {
            return None;
        }
        let (w, hh) = (r.right - r.left, r.bottom - r.top);
        (w > 0 && hh > 0).then_some((w as u32, hh as u32))
    }

    fn grab(src_dc: isize, w: i32, h: i32, from: Option<(i32, i32)>, window: Option<Hwnd>) -> Option<Vec<u8>> {
        // SAFETY: every GDI object made here is selected out and deleted
        // before returning; buffers are sized for w*h*4 bytes.
        unsafe {
            let mem = CreateCompatibleDC(src_dc);
            let bmp = CreateCompatibleBitmap(src_dc, w, h);
            if mem == 0 || bmp == 0 {
                if bmp != 0 {
                    DeleteObject(bmp);
                }
                if mem != 0 {
                    DeleteDC(mem);
                }
                return None;
            }
            let old = SelectObject(mem, bmp);
            let ok = match (from, window) {
                (Some((x, y)), _) => BitBlt(mem, 0, 0, w, h, src_dc, x, y, SRCCOPY) != 0,
                (None, Some(hw)) => PrintWindow(hw, mem, PW_CLIENTONLY | PW_RENDERFULLCONTENT) != 0,
                _ => false,
            };
            let mut buf = vec![0u8; (w * h * 4) as usize];
            let mut bi = BitmapInfo {
                header: BitmapInfoHeader {
                    size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                    width: w,
                    height: -h,
                    planes: 1,
                    bit_count: 32,
                    compression: 0,
                    size_image: 0,
                    xppm: 0,
                    yppm: 0,
                    clr_used: 0,
                    clr_important: 0,
                },
                colors: [0],
            };
            let lines = if ok { GetDIBits(mem, bmp, 0, h as u32, buf.as_mut_ptr() as *mut c_void, &mut bi, 0) } else { 0 };
            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
            (lines == h).then_some(buf)
        }
    }

    /// `is_blank`: img.max() < 10 (colour channels only).
    fn blank(bgra: &[u8]) -> bool {
        !bgra.chunks_exact(4).any(|p| p[0] >= super::BLANK_MAX || p[1] >= super::BLANK_MAX || p[2] >= super::BLANK_MAX)
    }

    pub fn capture(h: Hwnd) -> Option<Frame> {
        let (w, hh) = client_size(h)?;
        let (w, hh) = (w as i32, hh as i32);
        // SAFETY: DCs are released below.
        unsafe {
            let wdc = GetDC(h);
            if wdc != 0 {
                let got = grab(wdc, w, hh, None, Some(h));
                ReleaseDC(h, wdc);
                if let Some(px) = got.filter(|p| !blank(p)) {
                    return Some(Frame { w: w as usize, h: hh as usize, bgra: px });
                }
            }
            // Fallback: copy that part of the screen (the game must be visible).
            let mut p = Point::default();
            ClientToScreen(h, &mut p);
            let sdc = GetDC(0);
            if sdc == 0 {
                return None;
            }
            let got = grab(sdc, w, hh, Some((p.x, p.y)), None);
            ReleaseDC(0, sdc);
            got.filter(|p| !blank(p)).map(|px| Frame { w: w as usize, h: hh as usize, bgra: px })
        }
    }

    fn lparam(x: i32, y: i32) -> isize {
        (((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF)) as isize
    }

    /// `_target_window`: the child window under the point, and the point in
    /// its coordinates ("ถ้าเกมวาดภาพลงหน้าต่างลูก ต้องส่งคลิกไปที่หน้าต่างลูกนั้น").
    fn target_window(h: Hwnd, x: i32, y: i32) -> (Hwnd, i32, i32) {
        // SAFETY: plain Win32 calls with a valid point buffer.
        unsafe {
            let child = ChildWindowFromPointEx(h, Point { x, y }, CWP_SKIPINVISIBLE | CWP_SKIPDISABLED | CWP_SKIPTRANSPARENT);
            if child == 0 || child == h {
                return (h, x, y);
            }
            let mut p = Point { x, y };
            MapWindowPoints(h, child, &mut p, 1);
            (child, p.x, p.y)
        }
    }

    /// `_click_messages` with hover on: move, 0.01 s, down, hold, up.
    pub fn post_click(h: Hwnd, x: i32, y: i32, hold_secs: f64) {
        use std::time::Duration;
        let (t, x, y) = target_window(h, x, y);
        // SAFETY: posting a message to a window handle is always allowed.
        unsafe { PostMessageW(t, WM_MOUSEMOVE, 0, lparam(x, y)) };
        std::thread::sleep(Duration::from_secs_f64(super::MOVE_TO_PRESS_SECS));
        // SAFETY: as above.
        unsafe { PostMessageW(t, WM_LBUTTONDOWN, MK_LBUTTON, lparam(x, y)) };
        std::thread::sleep(Duration::from_secs_f64(hold_secs));
        // SAFETY: as above.
        unsafe { PostMessageW(t, WM_LBUTTONUP, 0, lparam(x, y)) };
    }

    /// `force_foreground`: attach to the foreground thread's input first
    /// (Windows refuses it from a background program otherwise).
    pub fn force_foreground(h: Hwnd) {
        // SAFETY: plain Win32 calls; the input attachment is undone.
        unsafe {
            let fg = GetForegroundWindow();
            let me = GetCurrentThreadId();
            let other = if fg != 0 { GetWindowThreadProcessId(fg, std::ptr::null_mut()) } else { 0 };
            if other != 0 && other != me {
                AttachThreadInput(me, other, 1);
                SetForegroundWindow(h);
                AttachThreadInput(me, other, 0);
            } else {
                SetForegroundWindow(h);
            }
        }
    }

    /// `_click_sendinput`: press with the real mouse, only when the game is
    /// what's under the point; false when another window covers it.
    pub fn real_click(h: Hwnd, x: i32, y: i32, hold_secs: f64) -> bool {
        use std::time::Duration;
        let mut back = Point::default();
        let mut p = Point { x, y };
        // SAFETY: valid pointers; inputs live for the SendInput calls.
        unsafe {
            ClientToScreen(h, &mut p);
            let under = WindowFromPoint(p);
            if under == 0 || GetAncestor(under, GA_ROOT) != h {
                return false;
            }
            GetCursorPos(&mut back);
            SetCursorPos(p.x, p.y);
            let mk = |flags| Input { kind: 0, mi: MouseInput { dx: 0, dy: 0, mouse_data: 0, flags, time: 0, extra: 0 } };
            let down = [mk(MOUSEEVENTF_LEFTDOWN)];
            SendInput(1, down.as_ptr(), std::mem::size_of::<Input>() as i32);
            std::thread::sleep(Duration::from_secs_f64(hold_secs));
            let up = [mk(MOUSEEVENTF_LEFTUP)];
            SendInput(1, up.as_ptr(), std::mem::size_of::<Input>() as i32);
            SetCursorPos(back.x, back.y);
        }
        true
    }

    static OLD_EX: std::sync::Mutex<Option<(Hwnd, i32)>> = std::sync::Mutex::new(None);

    /// `hide_in_place` / `show_in_place`.
    pub fn show(h: Hwnd, visible: bool) {
        // SAFETY: plain Win32 calls on a window handle.
        unsafe {
            let mut old = OLD_EX.lock().unwrap();
            if !visible {
                let ex = GetWindowLongW(h, GWL_EXSTYLE);
                if old.is_none() {
                    *old = Some((h, ex));
                }
                SetWindowLongW(h, GWL_EXSTYLE, ex | WS_EX_LAYERED | WS_EX_TRANSPARENT);
                SetLayeredWindowAttributes(h, 0, HIDDEN_ALPHA, LWA_ALPHA);
                SetWindowPos(h, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            } else if let Some((oh, ex)) = old.take() {
                if oh == h {
                    // show_in_place (sub_1801944f0): opaque again first.
                    SetLayeredWindowAttributes(h, 0, SHOWN_ALPHA, LWA_ALPHA);
                    SetWindowLongW(h, GWL_EXSTYLE, ex);
                    let f = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED;
                    SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, f);
                    SetWindowPos(h, HWND_NOTOPMOST, 0, 0, 0, 0, f);
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use win::Hwnd;
#[cfg(not(target_os = "windows"))]
pub type Hwnd = isize;

/// `MIN_GAME_WINDOW`: the smallest client a window of the game's program
/// must have to count when found by program name. The name is in
/// fo3bot.window (0x181144252) but its number isn't readable there; 200×150
/// is estimated.
pub const MIN_GAME_WINDOW: (u32, u32) = (200, 150);

/// The game window, if it's open.
pub fn find_window() -> Option<Hwnd> {
    #[cfg(target_os = "windows")]
    {
        win::find_window(crate::buttons::WINDOW_TITLE, "FIFA ONLINE3", crate::buttons::WINDOW_EXE)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn alive(h: Hwnd) -> bool {
    #[cfg(target_os = "windows")]
    {
        win::alive(h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = h;
        false
    }
}

pub fn client_size(h: Hwnd) -> Option<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        win::client_size(h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = h;
        None
    }
}

pub fn capture(h: Hwnd) -> Option<Frame> {
    #[cfg(target_os = "windows")]
    {
        win::capture(h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = h;
        None
    }
}

/// Click at a point inside the game window. Normal clicks are posted with
/// the original's jitter (±2 px); `real` is the wake-up: bring the game
/// forward, wait 0.35 s, press with the real mouse, no jitter. Returns
/// false when the real mouse can't reach the game (another window covers
/// it).
pub fn click(h: Hwnd, x: i32, y: i32, real: bool) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::time::Duration;
        let hold = uniform(HOLD_SECS.0, HOLD_SECS.1);
        std::thread::sleep(Duration::from_secs_f64(crate::bot::HOVER_BEFORE_CLICK_SECS));
        if real {
            win::force_foreground(h);
            std::thread::sleep(Duration::from_secs_f64(WAKE_SETTLE_SECS));
            win::real_click(h, x, y, hold)
        } else {
            win::post_click(h, x + jitter(CLICK_JITTER), y + jitter(CLICK_JITTER), hold);
            true
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (h, x, y, real);
        true
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn show(h: Hwnd, visible: bool) {
    #[cfg(target_os = "windows")]
    {
        win::show(h, visible)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (h, visible);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_ranges() {
        for _ in 0..500 {
            let j = jitter(CLICK_JITTER);
            assert!((-2..=2).contains(&j));
            let u = uniform(HOLD_SECS.0, HOLD_SECS.1);
            assert!((0.03..=0.08).contains(&u));
        }
        assert_eq!(jitter(0), 0);
    }
}
