//! The "ซื้อนักเตะ" (buy players) page: how many to buy, which +N level
//! to look for, a bought / target counter, and a switch to go to the
//! upgrade page (Team → จัดการทีม → จัดการนักเตะ → ระดับ) when done.

use crate::buttons::{MAX_ROUNDS, MIN_ROUNDS};
use crate::buyer::{BuyState, Buyer, BuyStep, NavStep, BUY_STEPS, MAX_LEVEL, MIN_LEVEL, NAV_STEPS};
use crate::ui::{
    bar, card, divider, pill, regular, reset_button, semi, bold, square_button, switch_row,
    Lang, Palette,
};
use eframe::egui::{self, Align, Align2, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub struct BuyTexts {
    pub tab_league: &'static str,
    pub tab_buy: &'static str,
    pub open_page: &'static str,
    pub ready: &'static str,
    pub buying: &'static str,
    pub going: &'static str,
    pub finished: &'static str,
    pub stalled: &'static str,
    pub reset: &'static str,
    pub bought: &'static str,
    pub looking_for: &'static str,
    pub current_step: &'static str,
    pub not_started: &'static str,
    pub buy_steps: [&'static str; 5],
    pub nav_steps: [&'static str; 4],
    pub target: &'static str,
    pub target_hint: &'static str,
    pub level: &'static str,
    pub level_hint: &'static str,
    pub goto: &'static str,
    pub goto_hint: &'static str,
}

impl BuyTexts {
    pub fn get(lang: Lang) -> &'static BuyTexts {
        match lang {
            Lang::Thai => &TH,
            Lang::English => &EN,
            Lang::Vietnamese => &VI,
        }
    }
}

static TH: BuyTexts = BuyTexts {
    tab_league: "ลีกอัตโนมัติ",
    tab_buy: "ซื้อนักเตะ",
    open_page: "เปิดหน้า ตลาดแลกเปลี่ยน → ซื้อ → ผลการค้นหา ในเกมก่อน แล้วจึงกดเริ่มทำงาน",
    ready: "พร้อมใช้งาน",
    buying: "กำลังซื้อนักเตะ",
    going: "กำลังไปหน้าอัปเกรด",
    finished: "ซื้อครบแล้ว",
    stalled: "หยุดอัตโนมัติ",
    reset: "รีเซ็ตจำนวน",
    bought: "ซื้อสำเร็จแล้ว",
    looking_for: "มองหาระดับ",
    current_step: "ขั้นตอนปัจจุบัน",
    not_started: "ยังไม่เริ่มดำเนินการ",
    buy_steps: ["หานักเตะระดับที่เลือก", "เลือกนักเตะ", "กดซื้อ", "ยืนยันการซื้อ", "ยอมรับ"],
    nav_steps: ["Team", "จัดการทีม", "จัดการนักเตะ", "ระดับ"],
    target: "จำนวนที่ต้องการซื้อ",
    target_hint: "หยุดอัตโนมัติเมื่อซื้อครบ",
    level: "มองหาระดับ",
    level_hint: "ซื้อเฉพาะนักเตะที่มีระดับนี้ในผลการค้นหา",
    goto: "ซื้อครบแล้วไปหน้าอัปเกรด",
    goto_hint: "Team → จัดการทีม → จัดการนักเตะ → ระดับ",
};

static EN: BuyTexts = BuyTexts {
    tab_league: "Auto league",
    tab_buy: "Buy players",
    open_page: "Open Transfer → Buy → Search results in the game first, then press Start",
    ready: "Ready",
    buying: "Buying players",
    going: "Going to upgrade page",
    finished: "All bought",
    stalled: "Auto-stopped",
    reset: "Reset count",
    bought: "Bought",
    looking_for: "Looking for",
    current_step: "Current step",
    not_started: "Not started",
    buy_steps: ["Find chosen level", "Select player", "Click Buy", "Confirm purchase", "Accept"],
    nav_steps: ["Team", "Team management", "Player management", "Level"],
    target: "Players to buy",
    target_hint: "Stops automatically when reached",
    level: "Level to look for",
    level_hint: "Only buys players with this level in the search results",
    goto: "Go to the upgrade page when done",
    goto_hint: "Team → Team management → Player management → Level",
};

static VI: BuyTexts = BuyTexts {
    tab_league: "Giải tự động",
    tab_buy: "Mua cầu thủ",
    open_page: "Mở Chuyển nhượng → Mua → Kết quả tìm kiếm trong game trước, rồi bấm Bắt đầu",
    ready: "Sẵn sàng",
    buying: "Đang mua cầu thủ",
    going: "Đang tới trang nâng cấp",
    finished: "Đã mua đủ",
    stalled: "Tự động dừng",
    reset: "Đặt lại số lượng",
    bought: "Đã mua",
    looking_for: "Tìm cấp",
    current_step: "Bước hiện tại",
    not_started: "Chưa bắt đầu",
    buy_steps: ["Tìm cấp đã chọn", "Chọn cầu thủ", "Bấm Mua", "Xác nhận mua", "Chấp nhận"],
    nav_steps: ["Team", "Quản lý đội", "Quản lý cầu thủ", "Cấp"],
    target: "Số cầu thủ cần mua",
    target_hint: "Tự động dừng khi mua đủ",
    level: "Cấp cần tìm",
    level_hint: "Chỉ mua cầu thủ có cấp này trong kết quả tìm kiếm",
    goto: "Mua xong thì tới trang nâng cấp",
    goto_hint: "Team → Quản lý đội → Quản lý cầu thủ → Cấp",
};

/// The badge colours the game uses for +N levels (+1 plain, +2..+4 bronze,
/// +5..+7 silver, +8..+10 gold — estimated from the screenshots).
fn badge_colours(level: u8) -> (Color32, Color32) {
    match level {
        1 => (Color32::from_rgb(40, 44, 50), Color32::from_rgb(200, 204, 210)),
        2..=4 => (Color32::from_rgb(176, 92, 52), Color32::from_rgb(255, 226, 200)),
        5..=7 => (Color32::from_rgb(70, 76, 86), Color32::WHITE),
        _ => (Color32::from_rgb(196, 160, 40), Color32::from_rgb(40, 30, 0)),
    }
}

/// The game's "+N" badge, drawn small.
fn badge(painter: &egui::Painter, r: Rect, level: u8) {
    let (fill, text) = badge_colours(level);
    painter.rect(r, 4.0, fill, Stroke::new(1.0_f32, text.gamma_multiply(0.6)));
    painter.text(r.center(), Align2::CENTER_CENTER, format!("+{level}"), semi(12.0), text);
}

/// Draw the page. The connection card and start button are shared with
/// the league page and drawn by the caller.
pub fn show(ui: &mut egui::Ui, p: &Palette, lang: Lang, buyer: &mut Buyer) {
    let t = BuyTexts::get(lang);
    hint_card(ui, p, t);
    count_card(ui, p, t, buyer);
    step_card(ui, p, t, buyer);
    settings_card(ui, p, t, buyer);
}

fn hint_card(ui: &mut egui::Ui, p: &Palette, t: &BuyTexts) {
    egui::Frame::none()
        .fill(p.accent.gamma_multiply(0.10))
        .stroke(Stroke::new(1.0_f32, p.accent.gamma_multiply(0.45)))
        .rounding(12.0)
        .inner_margin(egui::Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(t.open_page).font(regular(12.0)).color(p.text)).wrap());
        });
}

fn count_card(ui: &mut egui::Ui, p: &Palette, t: &BuyTexts, buyer: &mut Buyer) {
    card(ui, p, |ui| {
        ui.horizontal(|ui| {
            let (label, dot) = match buyer.state {
                BuyState::Idle => (t.ready, p.sub),
                BuyState::Buying => (t.buying, p.green),
                BuyState::GoingToUpgrade => (t.going, p.green),
                BuyState::Finished => (t.finished, p.accent),
                BuyState::Stalled => (t.stalled, Color32::from_rgb(240, 140, 60)),
            };
            pill(ui, p, label, dot);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if reset_button(ui, p, t.reset) {
                    buyer.reset();
                }
            });
        });
        ui.add_space(-10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(buyer.bought.to_string()).font(bold(52.0)).color(p.accent));
            ui.label(RichText::new(format!("/ {}", buyer.target)).font(semi(24.0)).color(p.muted));
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.label(RichText::new(t.bought).font(regular(13.0)).color(p.sub));
            });
        });
        ui.add_space(-6.0);
        bar(ui, p, (buyer.bought as f32 / buyer.target.max(1) as f32).min(1.0), 7.0);
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.looking_for).font(regular(13.0)).color(p.sub));
            let (r, _) = ui.allocate_exact_size(Vec2::new(40.0, 24.0), Sense::hover());
            badge(ui.painter(), r, buyer.level);
        });
    });
}

fn step_card(ui: &mut egui::Ui, p: &Palette, t: &BuyTexts, buyer: &Buyer) {
    card(ui, p, |ui| {
        let running = buyer.is_running();
        let nav = buyer.state == BuyState::GoingToUpgrade;
        let label = if !running {
            t.not_started
        } else if nav {
            let i = NAV_STEPS.iter().position(|(s, _)| *s == NAV_STEPS[buyer.step].0).unwrap_or(0);
            t.nav_steps[i]
        } else {
            let i = BUY_STEPS.iter().position(|(s, _)| *s == BUY_STEPS[buyer.step].0).unwrap_or(0);
            t.buy_steps[i]
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.current_step).font(semi(14.0)).color(p.sub));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(label).font(semi(15.0)).color(p.text));
            });
        });
        ui.add_space(2.0);
        let n = buyer.step_count();
        let gap = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0), Sense::hover());
        let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
        for i in 0..n {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + i as f32 * (w + gap), rect.top()),
                Vec2::new(w, rect.height()),
            );
            ui.painter().rect_filled(r, 3.5, p.track);
            let fill = if !running {
                0.0
            } else if i < buyer.step {
                1.0
            } else if i == buyer.step {
                buyer.step_fraction().max(0.08)
            } else {
                0.0
            };
            if fill > 0.0 {
                let mut f = r;
                f.set_width(r.width() * fill);
                ui.painter().rect_filled(f, 3.5, if nav { p.green } else { p.accent });
            }
        }
        // Keep the step names referenced so the tables stay in sync.
        let _ = (BuyStep::FindRow, NavStep::Team);
    });
}

fn settings_card(ui: &mut egui::Ui, p: &Palette, t: &BuyTexts, buyer: &mut Buyer) {
    card(ui, p, |ui| {
        // 1. How many to buy: − [n] +
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(t.target).font(semi(15.0)).color(p.text));
                ui.label(RichText::new(t.target_hint).font(regular(12.0)).color(p.muted));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if square_button(ui, p, "+", buyer.target < MAX_ROUNDS) {
                    buyer.set_target(buyer.target + 1);
                }
                let (r, resp) = ui.allocate_exact_size(Vec2::new(78.0, 40.0), Sense::hover());
                ui.painter().rect(r, 9.0, p.bg, Stroke::new(1.0_f32, p.border));
                ui.painter().text(r.center(), Align2::CENTER_CENTER, buyer.target.to_string(), semi(17.0), p.text);
                if resp.hovered() {
                    let scroll = ui.input(|i| i.raw_scroll_delta.y);
                    if scroll > 0.0 {
                        buyer.set_target(buyer.target + 1);
                    } else if scroll < 0.0 {
                        buyer.set_target(buyer.target.saturating_sub(1));
                    }
                }
                if square_button(ui, p, "−", buyer.target > MIN_ROUNDS) {
                    buyer.set_target(buyer.target.saturating_sub(1));
                }
            });
        });
        divider(ui, p);

        // 2. Which level: +1 .. +10
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(RichText::new(t.level).font(semi(15.0)).color(p.text));
        ui.label(RichText::new(t.level_hint).font(regular(12.0)).color(p.muted));
        ui.add_space(6.0);
        let cols = 5;
        let gap = 8.0;
        let w = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
        let h = 32.0;
        let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h * 2.0 + gap), Sense::hover());
        for lvl in MIN_LEVEL..=MAX_LEVEL {
            let i = (lvl - 1) as usize;
            let r = Rect::from_min_size(
                Pos2::new(area.left() + (i % cols) as f32 * (w + gap), area.top() + (i / cols) as f32 * (h + gap)),
                Vec2::new(w, h),
            );
            let resp = ui.interact(r, ui.id().with(("level", lvl)), Sense::click());
            let on = buyer.level == lvl;
            let fill = if on { p.accent.gamma_multiply(0.16) } else if resp.hovered() { p.track } else { p.pill };
            let stroke = if on { Stroke::new(2.0_f32, p.accent) } else { Stroke::new(1.0_f32, p.border) };
            ui.painter().rect(r, 9.0, fill, stroke);
            badge(ui.painter(), Rect::from_center_size(r.center(), Vec2::new(40.0, 24.0)), lvl);
            if resp.clicked() && !buyer.is_running() {
                buyer.set_level(lvl);
            }
        }
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.y = 10.0;
        divider(ui, p);

        // 4. Go to the upgrade page when done.
        let mut v = buyer.goto_upgrade;
        if switch_row(ui, p, t.goto, Some(t.goto_hint), &mut v) {
            buyer.goto_upgrade = v;
        }
    });
}
