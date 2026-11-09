//! Look and words of the control panel.
//!
//! Layout (top to bottom), after the original's v1.6 screen:
//!   * header: logo, name, TH / EN / VI switch, light/dark button
//!   * game connection card (status dot, game size, Admin badge)
//!   * rounds card (status pill, reset, big count / target, progress,
//!     running time, average per round)
//!   * current step card (7 segments)
//!   * settings card (target rounds, unlimited, hide game, auto-renew)
//!   * big yellow Start button with F8 badge, and a footer line
//!
//! All words are in `Texts`; fonts are set up in `install_fonts`.

use crate::automation::{Automation, RunState, Step, STEPS};
use eframe::egui::{
    self, Align, Align2, Color32, FontFamily, FontId, Layout, Margin,
    Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

pub const APP_NAME: &str = "FiFAO3 Mgmt";
pub const VERSION: &str = "v1.6";

// ---------------------------------------------------------------- fonts

/// Prompt (SIL Open Font License) covers Thai, Latin and Vietnamese, so no
/// letter shows as a box. It's built into the program, so it works on any
/// computer, Windows included.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "prompt".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Prompt-Regular.ttf")),
    );
    fonts.font_data.insert(
        "prompt-semibold".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Prompt-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        "prompt-bold".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Prompt-Bold.ttf")),
    );
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    let with = |first: &str| {
        let mut v = vec![first.to_string()];
        v.extend(fallback.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with("prompt"));
    fonts.families.insert(FontFamily::Name("semi".into()), with("prompt-semibold"));
    fonts.families.insert(FontFamily::Name("bold".into()), with("prompt-bold"));
    ctx.set_fonts(fonts);
}

pub(crate) fn semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semi".into()))
}
pub(crate) fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub(crate) fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

// ---------------------------------------------------------------- colours

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub sub: Color32,
    pub muted: Color32,
    pub track: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub green: Color32,
    pub pill: Color32,
    pub knob: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(12, 12, 14),
            card: Color32::from_rgb(28, 28, 31),
            border: Color32::from_rgb(52, 52, 57),
            text: Color32::from_rgb(242, 242, 244),
            sub: Color32::from_rgb(196, 196, 200),
            muted: Color32::from_rgb(140, 140, 146),
            track: Color32::from_rgb(64, 64, 70),
            accent: Color32::from_rgb(245, 222, 58),
            on_accent: Color32::from_rgb(24, 22, 10),
            green: Color32::from_rgb(52, 211, 105),
            pill: Color32::from_rgb(48, 48, 53),
            knob: Color32::from_rgb(236, 236, 238),
        }
    }
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(240, 240, 243),
            card: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgb(218, 218, 224),
            text: Color32::from_rgb(22, 22, 26),
            sub: Color32::from_rgb(70, 70, 78),
            muted: Color32::from_rgb(120, 120, 128),
            track: Color32::from_rgb(214, 214, 220),
            accent: Color32::from_rgb(240, 206, 20),
            on_accent: Color32::from_rgb(24, 22, 10),
            green: Color32::from_rgb(24, 170, 80),
            pill: Color32::from_rgb(236, 236, 240),
            knob: Color32::WHITE,
        }
    }
}

// ---------------------------------------------------------------- words

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Thai,
    English,
    Vietnamese,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Thai, Lang::English, Lang::Vietnamese];
    pub fn code(self) -> &'static str {
        match self {
            Lang::Thai => "TH",
            Lang::English => "EN",
            Lang::Vietnamese => "VI",
        }
    }
    /// Button-picture folder of this language, as in the original
    /// (templates/, templates/en/, templates/vi/).
    pub fn template_dir(self) -> &'static str {
        match self {
            Lang::Thai => "",
            Lang::English => "en",
            Lang::Vietnamese => "vi",
        }
    }
}

pub struct Texts {
    pub connected: &'static str,
    pub not_found: &'static str,
    pub not_found_sub: &'static str,
    pub not_admin: &'static str,
    pub ready: &'static str,
    pub running: &'static str,
    pub finished: &'static str,
    pub stalled: &'static str,
    pub reset: &'static str,
    pub rounds_done: &'static str,
    pub run_time: &'static str,
    pub average: &'static str,
    pub current_step: &'static str,
    pub not_started: &'static str,
    pub steps: [&'static str; 7],
    pub target: &'static str,
    pub target_hint: &'static str,
    pub unlimited_hint: &'static str,
    pub unlimited: &'static str,
    pub hide: &'static str,
    pub hide_hint: &'static str,
    pub hide_on_hint: &'static str,
    pub renew: &'static str,
    pub renew_hint: &'static str,
    pub renew_on_hint: &'static str,
    pub start: &'static str,
    pub stop: &'static str,
    pub footer: &'static str,
}

impl Texts {
    pub fn get(lang: Lang) -> &'static Texts {
        match lang {
            Lang::Thai => &TH,
            Lang::English => &EN,
            Lang::Vietnamese => &VI,
        }
    }
}

// Wording follows the original's own text table (its `TEXT` dict, three
// languages, Thai first), unpacked from the program — see MECHANICS.md.

static TH: Texts = Texts {
    connected: "เชื่อมต่อเกมแล้ว",
    not_found: "ไม่พบเกม",
    not_found_sub: "กรุณาเปิดเกมแบบหน้าต่าง",
    not_admin: "ไม่ใช่ Admin",
    ready: "พร้อมใช้งาน",
    running: "กำลังทำงาน",
    finished: "เสร็จสิ้น",
    stalled: "หยุดอัตโนมัติ",
    reset: "รีเซ็ตรอบ",
    rounds_done: "รอบที่ดำเนินการแล้ว",
    run_time: "ระยะเวลาทำงาน",
    average: "เฉลี่ยต่อรอบ",
    current_step: "ขั้นตอนปัจจุบัน",
    not_started: "ยังไม่เริ่มดำเนินการ",
    steps: [
        "VS COM",
        "League",
        "จำลองการแข่งขัน",
        "รอการแข่งขันสิ้นสุด",
        "ดูผลการแข่งขัน",
        "เลือกรางวัลแบบสุ่ม",
        "ต่อไป",
    ],
    target: "จำนวนรอบที่ต้องการ",
    target_hint: "หยุดอัตโนมัติเมื่อครบจำนวนรอบ",
    unlimited_hint: "ทำงานต่อเนื่องจนกว่าจะสั่งหยุด",
    unlimited: "ไม่จำกัดจำนวนรอบ",
    hide: "ซ่อนหน้าต่างเกม",
    hide_hint: "แนะนำให้ใช้แทนการย่อหน้าต่างเกม",
    hide_on_hint: "ระบบทำงานต่อเนื่องขณะที่ท่านใช้งานโปรแกรมอื่น",
    renew: "ต่อสัญญานักเตะอัตโนมัติ",
    renew_hint: "ปิดอยู่: ระบบจะกด \"ยกเลิก\" เมื่อมีหน้าต่างต่อสัญญา หากเปิดใช้ ระบบจะกด \"ต่อสัญญา\" ซึ่งมีค่าใช้จ่าย",
    renew_on_hint: "ระบบจะกด \"ต่อสัญญา\" ทุกครั้งที่มีหน้าต่างปรากฏ การต่อสัญญานักเตะมีค่าใช้จ่าย (หัก EP ในเกม)",
    start: "เริ่มทำงาน",
    stop: "หยุดทำงาน",
    footer: "ระบบหยุดอัตโนมัติหากไม่มีความคืบหน้าเกิน {m} นาที",
};

static EN: Texts = Texts {
    connected: "Game connected",
    not_found: "Game not found",
    not_found_sub: "Use windowed mode",
    not_admin: "Not admin",
    ready: "Ready",
    running: "Running",
    finished: "Finished",
    stalled: "Auto-stopped",
    reset: "Reset rounds",
    rounds_done: "Rounds completed",
    run_time: "Elapsed time",
    average: "Average per round",
    current_step: "Current step",
    not_started: "Not started",
    steps: [
        "VS COM",
        "League",
        "Simulate match",
        "Await full time",
        "View match result",
        "Select random reward",
        "Next",
    ],
    target: "Target rounds",
    target_hint: "Stops automatically when reached",
    unlimited_hint: "Runs continuously until stopped",
    unlimited: "Unlimited rounds",
    hide: "Hide game window",
    hide_hint: "Recommended instead of minimizing the game",
    hide_on_hint: "Automation continues while you use other applications",
    renew: "Auto-renew player contracts",
    renew_hint: "Off: the system clicks \u{201c}Cancel\u{201d} when the contract window appears. If turned on, it clicks \u{201c}Renew contract\u{201d}, which has a cost.",
    renew_on_hint: "The system clicks \u{201c}Renew contract\u{201d} every time the window appears. Renewing player contracts has a cost (deducted from in-game EP).",
    start: "Start",
    stop: "Stop",
    footer: "Stops automatically after {m} min without progress",
};

static VI: Texts = Texts {
    connected: "Đã kết nối game",
    not_found: "Chưa có game",
    not_found_sub: "Mở game dạng cửa sổ",
    not_admin: "Thiếu Admin",
    ready: "Sẵn sàng",
    running: "Đang hoạt động",
    finished: "Hoàn tất",
    stalled: "Tự động dừng",
    reset: "Đặt lại số vòng",
    rounds_done: "Số vòng đã hoàn thành",
    run_time: "Thời gian chạy",
    average: "Trung bình/vòng",
    current_step: "Bước hiện tại",
    not_started: "Chưa bắt đầu",
    steps: [
        "VS COM",
        "League",
        "Mô phỏng trận đấu",
        "Chờ kết thúc trận",
        "Xem kết quả trận đấu",
        "Chọn phần thưởng ngẫu nhiên",
        "Tiếp theo",
    ],
    target: "Số vòng mục tiêu",
    target_hint: "Tự động dừng khi đạt số vòng",
    unlimited_hint: "Chạy liên tục đến khi được dừng",
    unlimited: "Không giới hạn số vòng",
    hide: "Ẩn cửa sổ game",
    hide_hint: "Khuyến nghị dùng thay cho thu nhỏ cửa sổ game",
    hide_on_hint: "Hệ thống tiếp tục hoạt động khi bạn dùng ứng dụng khác",
    renew: "Tự động gia hạn hợp đồng cầu thủ",
    renew_hint: "Đang tắt: hệ thống sẽ nhấn \u{201c}Hủy\u{201d} khi hiện cửa sổ gia hạn. Nếu bật, hệ thống sẽ nhấn \u{201c}Gia hạn\u{201d} và việc này có phí.",
    renew_on_hint: "Hệ thống sẽ nhấn \u{201c}Gia hạn\u{201d} mỗi lần cửa sổ xuất hiện. Gia hạn hợp đồng cầu thủ có phí (trừ EP trong game).",
    start: "Bắt đầu",
    stop: "Dừng",
    footer: "Tự động dừng nếu không có tiến triển sau {m} phút",
};

fn step_index(step: Step) -> usize {
    STEPS.iter().position(|(s, _)| *s == step).unwrap_or(0)
}

pub(crate) fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

// ---------------------------------------------------------------- state

/// Which page is open: the league bot or the player buyer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    League,
    Buy,
}

pub struct Ui {
    pub lang: Lang,
    pub dark: bool,
    pub page: Page,
}

/// What the person clicked this frame.
#[derive(Default)]
pub struct Actions {
    pub toggle_run: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self { lang: Lang::Thai, dark: true, page: Page::League }
    }

    pub fn palette(&self) -> Palette {
        if self.dark {
            Palette::dark()
        } else {
            Palette::light()
        }
    }

    /// Draw the whole window.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        auto: &mut Automation,
        buyer: &mut crate::buyer::Buyer,
    ) -> Actions {
        let p = self.palette();
        let t = Texts::get(self.lang);
        let mut actions = Actions::default();
        let page = egui::Frame::none().fill(p.bg).inner_margin(Margin::symmetric(16.0, 12.0));

        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::none()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(16.0, 12.0))
                    .stroke(Stroke::new(1.0_f32, p.border)),
            )
            .show(ctx, |ui| self.header(ui, &p));

        egui::TopBottomPanel::bottom("start")
            .frame(page.inner_margin(Margin { left: 16.0, right: 16.0, top: 10.0, bottom: 12.0 }))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let running = match self.page {
                    Page::League => auto.is_running(),
                    Page::Buy => buyer.is_running(),
                };
                if start_button(ui, &p, t, running) {
                    actions.toggle_run = true;
                }
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}",
                            t.footer.replace("{m}", &crate::buttons::IDLE_MINUTES.to_string()),
                            VERSION
                        ))
                            .font(regular(11.5))
                            .color(p.muted),
                    );
                });
            });

        egui::CentralPanel::default().frame(page).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(4.0);
                connection_card(ui, &p, t);
                match self.page {
                    Page::League => {
                        rounds_card(ui, &p, t, auto);
                        step_card(ui, &p, t, auto);
                        settings_card(ui, &p, t, auto);
                    }
                    Page::Buy => crate::buy_ui::show(ui, &p, self.lang, buyer),
                }
            });
        });
        actions
    }

    fn header(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
            logo(ui.painter(), rect, p);
            ui.add_space(4.0);
            ui.label(RichText::new(APP_NAME).font(bold(19.0)).color(p.text));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Light / dark button.
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
                let fill = if resp.hovered() { p.pill } else { p.card };
                ui.painter().rect(r, 10.0, fill, Stroke::new(1.0_f32, p.border));
                if self.dark {
                    sun(ui.painter(), r.center(), p.text);
                } else {
                    moon(ui.painter(), r.center(), p.text, fill);
                }
                if resp.clicked() {
                    self.dark = !self.dark;
                }
                ui.add_space(4.0);

                // TH / EN / VI segmented switch.
                let seg = Vec2::new(46.0, 32.0);
                let pad = 4.0;
                let (outer, _) = ui.allocate_exact_size(
                    Vec2::new(seg.x * 3.0 + pad * 2.0, seg.y + pad * 2.0),
                    Sense::hover(),
                );
                ui.painter().rect(outer, 10.0, p.card, Stroke::new(1.0_f32, p.border));
                for (i, lang) in Lang::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(
                        outer.min + Vec2::new(pad + seg.x * i as f32, pad),
                        seg,
                    );
                    let resp = ui.interact(r, ui.id().with(("lang", i)), Sense::click());
                    let on = *lang == self.lang;
                    if on {
                        ui.painter().rect_filled(r, 7.0, p.pill);
                    }
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        lang.code(),
                        semi(13.5),
                        if on || resp.hovered() { p.text } else { p.muted },
                    );
                    if resp.clicked() {
                        self.lang = *lang;
                    }
                }
            });
        });
        ui.add_space(10.0);
        self.page_tabs(ui, p);
    }

    /// Two tabs under the header: the league bot and the player buyer.
    fn page_tabs(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let bt = crate::buy_ui::BuyTexts::get(self.lang);
        let pad = 4.0;
        let (outer, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::hover());
        ui.painter().rect(outer, 10.0, p.card, Stroke::new(1.0_f32, p.border));
        let w = (outer.width() - pad * 2.0) / 2.0;
        for (i, (page, label)) in [(Page::League, bt.tab_league), (Page::Buy, bt.tab_buy)]
            .into_iter()
            .enumerate()
        {
            let r = Rect::from_min_size(
                outer.min + Vec2::new(pad + w * i as f32, pad),
                Vec2::new(w, outer.height() - pad * 2.0),
            );
            let resp = ui.interact(r, ui.id().with(("page", i)), Sense::click());
            let on = self.page == page;
            if on {
                ui.painter().rect_filled(r, 7.0, p.accent);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 7.0, p.pill);
            }
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                label,
                semi(14.0),
                if on { p.on_accent } else { p.sub },
            );
            if resp.clicked() {
                self.page = page;
            }
        }
    }
}

// ---------------------------------------------------------------- cards

pub(crate) fn card(ui: &mut egui::Ui, p: &Palette, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(p.card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(14.0)
        .inner_margin(Margin::symmetric(18.0, 14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

pub(crate) fn connection_card(ui: &mut egui::Ui, p: &Palette, t: &Texts) {
    card(ui, p, |ui| {
        ui.horizontal(|ui| {
            let size = crate::backend::game_window_size();
            let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 20.0), Sense::hover());
            let dot = if size.is_some() { p.green } else { Color32::from_rgb(230, 80, 70) };
            ui.painter().circle_filled(r.center(), 6.0, dot);
            ui.painter().circle_filled(r.center(), 9.0, dot.gamma_multiply(0.18));
            ui.add_space(4.0);
            match size {
                Some((w, h)) => {
                    ui.label(RichText::new(t.connected).font(semi(15.0)).color(p.text));
                    ui.label(RichText::new(format!("{w}×{h}")).font(regular(12.5)).color(p.muted));
                }
                None => {
                    ui.label(RichText::new(t.not_found).font(semi(15.0)).color(p.text));
                    ui.label(RichText::new(t.not_found_sub).font(regular(12.5)).color(p.muted));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // The original needs Administrator rights (its manifest says
                // requireAdministrator) and shows this badge either way.
                let (label, col) = if crate::backend::is_admin() {
                    ("Admin", p.green)
                } else {
                    (t.not_admin, Color32::from_rgb(240, 140, 60))
                };
                let galley = ui.painter().layout_no_wrap(label.into(), semi(13.0), col);
                let size = galley.size() + Vec2::new(28.0, 10.0);
                let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().rect_filled(r, r.height() / 2.0, col.gamma_multiply(0.16));
                ui.painter().galley(r.center() - galley.size() / 2.0, galley, col);
            });
        });
    });
}

fn rounds_card(ui: &mut egui::Ui, p: &Palette, t: &Texts, auto: &mut Automation) {
    card(ui, p, |ui| {
        ui.horizontal(|ui| {
            let (label, dot) = match auto.state {
                RunState::Idle => (t.ready, p.sub),
                RunState::Running => (t.running, p.green),
                RunState::Finished => (t.finished, p.accent),
                RunState::Stalled => (t.stalled, Color32::from_rgb(240, 140, 60)),
            };
            pill(ui, p, label, dot);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if reset_button(ui, p, t.reset) {
                    auto.reset();
                }
            });
        });
        ui.add_space(-14.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(auto.completed.to_string()).font(bold(64.0)).color(p.accent),
            );
            let target = if auto.unlimited { "∞".to_string() } else { auto.target.to_string() };
            ui.label(RichText::new(format!("/ {target}")).font(semi(24.0)).color(p.muted));
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.label(RichText::new(t.rounds_done).font(regular(13.0)).color(p.sub));
            });
        });
        ui.add_space(-6.0);
        let frac = if auto.unlimited {
            0.0
        } else {
            (auto.completed as f32 / auto.target.max(1) as f32).min(1.0)
        };
        bar(ui, p, frac, 7.0);
        ui.add_space(6.0);
        ui.columns(2, |cols| {
            stat(&mut cols[0], p, t.run_time, clock(auto.elapsed_secs()));
            stat(
                &mut cols[1],
                p,
                t.average,
                auto.average_secs().map(clock).unwrap_or_else(|| "–".into()),
            );
        });
    });
}

fn step_card(ui: &mut egui::Ui, p: &Palette, t: &Texts, auto: &Automation) {
    card(ui, p, |ui| {
        let running = auto.is_running();
        let current = step_index(auto.current_step());
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.current_step).font(semi(14.0)).color(p.sub));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if running { t.steps[current] } else { t.not_started };
                ui.label(RichText::new(label).font(semi(15.0)).color(p.text));
            });
        });
        ui.add_space(2.0);
        let n = STEPS.len();
        let gap = 6.0;
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0), Sense::hover());
        let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
        for i in 0..n {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + i as f32 * (w + gap), rect.top()),
                Vec2::new(w, rect.height()),
            );
            ui.painter().rect_filled(r, 3.5, p.track);
            let fill = if !running {
                0.0
            } else if i < current {
                1.0
            } else if i == current {
                auto.step_fraction().max(0.08)
            } else {
                0.0
            };
            if fill > 0.0 {
                let mut f = r;
                f.set_width(r.width() * fill);
                ui.painter().rect_filled(f, 3.5, p.accent);
            }
        }
    });
}

fn settings_card(ui: &mut egui::Ui, p: &Palette, t: &Texts, auto: &mut Automation) {
    card(ui, p, |ui| {
        // Rounds to play: − [n] +
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(t.target).font(semi(15.0)).color(p.text));
                ui.label(RichText::new(if auto.unlimited { t.unlimited_hint } else { t.target_hint }).font(regular(12.0)).color(p.muted));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let enabled = !auto.unlimited;
                if square_button(ui, p, "+", enabled) {
                    auto.set_target(auto.target + 1);
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::new(78.0, 40.0), Sense::hover());
                ui.painter().rect(r, 9.0, p.bg, Stroke::new(1.0_f32, p.border));
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    auto.target.to_string(),
                    semi(17.0),
                    if enabled { p.text } else { p.muted },
                );
                if enabled && resp.hovered() {
                    let scroll = ui.input(|i| i.raw_scroll_delta.y);
                    if scroll > 0.0 {
                        auto.set_target(auto.target + 1);
                    } else if scroll < 0.0 {
                        auto.set_target(auto.target.saturating_sub(1));
                    }
                }
                if square_button(ui, p, "−", enabled) {
                    auto.set_target(auto.target.saturating_sub(1));
                }
            });
        });
        divider(ui, p);
        let mut v = auto.unlimited;
        if switch_row(ui, p, t.unlimited, None, &mut v) {
            auto.unlimited = v;
        }
        divider(ui, p);
        let mut v = auto.hide_window;
        if switch_row(ui, p, t.hide, Some(if auto.hide_window { t.hide_on_hint } else { t.hide_hint }), &mut v) {
            auto.set_hide_window(v);
        }
        divider(ui, p);
        let mut v = auto.renew_contracts;
        if switch_row(ui, p, t.renew, Some(if auto.renew_contracts { t.renew_on_hint } else { t.renew_hint }), &mut v) {
            auto.renew_contracts = v;
        }
    });
}

// ---------------------------------------------------------------- widgets

pub(crate) fn divider(ui: &mut egui::Ui, p: &Palette) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, p.border);
}

fn stat(ui: &mut egui::Ui, p: &Palette, label: &str, value: String) {
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.label(RichText::new(label).font(regular(12.5)).color(p.sub));
    ui.label(RichText::new(value).font(semi(20.0)).color(p.text));
}

pub(crate) fn bar(ui: &mut egui::Ui, p: &Palette, frac: f32, h: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    ui.painter().rect_filled(r, h / 2.0, p.track);
    if frac > 0.0 {
        let mut f = r;
        f.set_width((r.width() * frac).max(h));
        ui.painter().rect_filled(f, h / 2.0, p.accent);
    }
}

pub(crate) fn pill(ui: &mut egui::Ui, p: &Palette, label: &str, dot: Color32) {
    let galley = ui.painter().layout_no_wrap(label.to_string(), semi(13.5), p.text);
    let size = Vec2::new(galley.size().x + 44.0, 34.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(r, 17.0, p.pill);
    ui.painter().circle_filled(Pos2::new(r.left() + 18.0, r.center().y), 3.5, dot);
    ui.painter().galley(
        Pos2::new(r.left() + 30.0, r.center().y - galley.size().y / 2.0),
        galley,
        p.text,
    );
}

pub(crate) fn reset_button(ui: &mut egui::Ui, p: &Palette, label: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(label.to_string(), semi(13.5), p.muted);
    let size = Vec2::new(galley.size().x + 50.0, 34.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if resp.hovered() { p.track } else { p.pill };
    ui.painter().rect_filled(r, 17.0, fill);
    let c = Pos2::new(r.left() + 21.0, r.center().y);
    let col = if resp.hovered() { p.text } else { p.muted };
    // Circular arrow.
    let rad = 7.0;
    let pts: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = -1.2 + i as f32 / 24.0 * 5.0;
            c + Vec2::new(a.cos(), a.sin()) * rad
        })
        .collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.8_f32, col)));
    let tip = c + Vec2::new((-1.2f32).cos(), (-1.2f32).sin()) * rad;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![tip + Vec2::new(-4.5, -3.0), tip + Vec2::new(3.0, -4.0), tip + Vec2::new(0.5, 3.5)],
        col,
        Stroke::NONE,
    ));
    ui.painter().galley(
        Pos2::new(r.left() + 36.0, r.center().y - galley.size().y / 2.0),
        galley,
        col,
    );
    resp.clicked()
}

pub(crate) fn square_button(ui: &mut egui::Ui, p: &Palette, label: &str, enabled: bool) -> bool {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(40.0), sense);
    let fill = if enabled && resp.hovered() { p.track } else { p.pill };
    ui.painter().rect(r, 9.0, fill, Stroke::new(1.0_f32, p.border));
    // Draw − and + as lines so they're always crisp and the same size.
    let col = if enabled { p.text } else { p.muted };
    let c = r.center();
    let stroke = Stroke::new(2.2_f32, col);
    ui.painter().line_segment([c - Vec2::new(7.0, 0.0), c + Vec2::new(7.0, 0.0)], stroke);
    if label == "+" {
        ui.painter().line_segment([c - Vec2::new(0.0, 7.0), c + Vec2::new(0.0, 7.0)], stroke);
    }
    enabled && resp.clicked()
}

/// A label (and optional hint) on the left, an on/off switch on the right.
/// Returns true when the switch was flipped.
pub(crate) fn switch_row(
    ui: &mut egui::Ui,
    p: &Palette,
    title: &str,
    hint: Option<&str>,
    on: &mut bool,
) -> bool {
    let switch_w = 50.0;
    let mut flipped = false;
    ui.horizontal(|ui| {
        let text_w = ui.available_width() - switch_w - 16.0;
        ui.allocate_ui_with_layout(
            Vec2::new(text_w, 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(title).font(semi(15.0)).color(p.text));
                if let Some(h) = hint {
                    ui.add(
                        egui::Label::new(RichText::new(h).font(regular(12.0)).color(p.muted))
                            .wrap(),
                    );
                }
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(switch_w, 28.0), Sense::click());
            if resp.clicked() {
                *on = !*on;
                flipped = true;
            }
            let k = ui.ctx().animate_bool(resp.id, *on);
            let track = lerp_color(p.track, p.accent, k);
            ui.painter().rect_filled(r, 14.0, track);
            let x = egui::lerp(r.left() + 14.0..=r.right() - 14.0, k);
            ui.painter().circle_filled(Pos2::new(x, r.center().y), 10.5, p.knob);
        });
    });
    flipped
}

pub(crate) fn start_button(ui: &mut egui::Ui, p: &Palette, t: &Texts, running: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 66.0), Sense::click());
    let (fill, fg) = if running {
        (Color32::from_rgb(229, 72, 77), Color32::WHITE)
    } else {
        (p.accent, p.on_accent)
    };
    let fill = if resp.hovered() { fill.gamma_multiply(0.92) } else { fill };
    // Soft glow under the button.
    for i in 1..=6 {
        let grow = i as f32 * 2.5;
        ui.painter()
            .rect_filled(r.expand(grow), 16.0 + grow, fill.gamma_multiply(0.05));
    }
    ui.painter().rect_filled(r, 16.0, fill);

    let label = if running { t.stop } else { t.start };
    let galley = ui.painter().layout_no_wrap(label.to_string(), bold(20.0), fg);
    let badge = ui.painter().layout_no_wrap("F8".into(), semi(12.0), fg);
    let icon_w = 14.0;
    let badge_w = badge.size().x + 14.0;
    let total = icon_w + 12.0 + galley.size().x + 12.0 + badge_w;
    let mut x = r.center().x - total / 2.0;
    let cy = r.center().y;
    if running {
        ui.painter().rect_filled(
            Rect::from_center_size(Pos2::new(x + 7.0, cy), Vec2::splat(12.0)),
            2.0,
            fg,
        );
    } else {
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x + 1.0, cy - 7.0),
                Pos2::new(x + 13.0, cy),
                Pos2::new(x + 1.0, cy + 7.0),
            ],
            fg,
            Stroke::NONE,
        ));
    }
    x += icon_w + 12.0;
    let gh = galley.size().y;
    let gw = galley.size().x;
    ui.painter().galley(Pos2::new(x, cy - gh / 2.0), galley, fg);
    x += gw + 12.0;
    let br = Rect::from_min_size(Pos2::new(x, cy - 12.0), Vec2::new(badge_w, 24.0));
    ui.painter().rect_filled(br, 6.0, fg.gamma_multiply(0.14));
    let bs = badge.size();
    ui.painter().galley(br.center() - bs / 2.0, badge, fg);
    resp.clicked()
}

// ---------------------------------------------------------------- icons

fn logo(painter: &egui::Painter, r: Rect, p: &Palette) {
    let c = r.center();
    let s = r.width() / 2.0;
    let pts: Vec<Pos2> = (0..6)
        .map(|i| {
            let a = std::f32::consts::FRAC_PI_3 * i as f32 + std::f32::consts::FRAC_PI_6;
            c + Vec2::new(a.cos(), a.sin()) * s
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(pts, p.accent, Stroke::NONE));
    painter.text(c + Vec2::new(0.0, 1.0), Align2::CENTER_CENTER, "M", bold(15.0), p.on_accent);
}

fn sun(painter: &egui::Painter, c: Pos2, col: Color32) {
    painter.circle_stroke(c, 4.5, Stroke::new(1.8_f32, col));
    for i in 0..8 {
        let a = std::f32::consts::FRAC_PI_4 * i as f32;
        let d = Vec2::new(a.cos(), a.sin());
        painter.line_segment([c + d * 7.5, c + d * 10.0], Stroke::new(1.8_f32, col));
    }
}

fn moon(painter: &egui::Painter, c: Pos2, col: Color32, bg: Color32) {
    painter.circle_filled(c, 8.0, col);
    painter.circle_filled(c + Vec2::new(4.0, -3.5), 7.0, bg);
}

pub(crate) fn lerp_color(a: Color32, b: Color32, k: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

