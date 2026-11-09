//! FiFAO3 Mgmt — main entry point.
//!
//! Desktop control panel for FIFA Online 3 league (VS COM) automation:
//! rounds counter with a target, 7-step progress, hide-game-window and
//! auto-renew switches, TH / EN / VI, light and dark look, F8 to start/stop.
//!
//! `--screenshot <file.png>`: open the window without focus, draw ~120
//! frames, save the window as a PNG to the given path, and exit.

// No console window behind the app on Windows.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod automation;
mod bot;
mod backend;
mod buttons;
mod buy_ui;
mod buyer;
mod matcher;
mod ui;

use eframe::egui;
use std::path::PathBuf;

struct App {
    ui: ui::Ui,
    automation: automation::Automation,
    buyer: buyer::Buyer,
    screenshot_path: Option<PathBuf>,
    frames_drawn: u32,
    screenshot_done: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>) -> Self {
        ui::install_fonts(&cc.egui_ctx);
        Self {
            ui: {
                // `--page buy` opens on the buy-players page.
                let mut u = ui::Ui::new();
                let args: Vec<String> = std::env::args().collect();
                if args.windows(2).any(|w| w[0] == "--page" && w[1] == "buy") {
                    u.page = ui::Page::Buy;
                }
                u
            },
            automation: automation::Automation::new(),
            buyer: buyer::Buyer::new(),
            screenshot_path: shot,
            frames_drawn: 0,
            screenshot_done: false,
        }
    }
}

impl App {
    /// Start or stop the page that's open. Both bots drive the same game
    /// window, so starting one stops the other.
    fn toggle_open_page(&mut self) {
        match self.ui.page {
            ui::Page::League => {
                if self.buyer.is_running() {
                    self.buyer.stop(buyer::BuyState::Idle);
                }
                self.automation.toggle();
            }
            ui::Page::Buy => {
                if self.automation.is_running() {
                    self.automation.stop(automation::RunState::Idle);
                }
                self.buyer.toggle();
            }
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.ui.palette().bg.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames_drawn += 1;
        ctx.set_visuals(if self.ui.dark { egui::Visuals::dark() } else { egui::Visuals::light() });

        // F8 starts / stops the open page, like the original.
        let mut toggle = ctx.input(|i| i.key_pressed(egui::Key::F8));

        self.automation.tick();
        self.buyer.lang_dir = self.ui.lang.template_dir();
        self.buyer.tick();
        let actions = self.ui.show(ctx, &mut self.automation, &mut self.buyer);
        toggle |= actions.toggle_run;
        if toggle {
            self.toggle_open_page();
        }

        // Clocks and the step bars move on their own while running.
        if self.automation.is_running() || self.buyer.is_running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        if self.screenshot_path.is_some() && !self.screenshot_done {
            ctx.request_repaint();
            if self.frames_drawn == 120 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let shot = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = shot {
                if let Some(path) = &self.screenshot_path {
                    save_png(&image, path);
                }
                self.screenshot_done = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // Safety net: if the picture never arrives, leave anyway.
            if self.frames_drawn >= 400 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

/// Save an egui ColorImage as a PNG.
fn save_png(img: &egui::ColorImage, path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let (w, h) = (img.size[0] as u32, img.size[1] as u32);
    let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    let out = image::RgbaImage::from_raw(w, h, rgba).expect("pixels match size");
    match out.save_with_format(path, image::ImageFormat::Png) {
        Ok(()) => eprintln!("wrote {}", path.display()),
        Err(e) => eprintln!("failed to write {}: {}", path.display(), e),
    }
}

/// `--screenshot <file.png>` or `--screenshot=file.png`.
fn parse_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            return Some(PathBuf::from(args.get(i + 1).cloned().unwrap_or("screenshot.png".into())));
        }
        if let Some(rest) = args[i].strip_prefix("--screenshot=") {
            return Some(PathBuf::from(rest));
        }
        i += 1;
    }
    None
}

fn main() -> eframe::Result<()> {
    let screenshot_path = parse_args();
    let shooting = screenshot_path.is_some();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(egui::vec2(480.0, 960.0))
        .with_min_inner_size(egui::vec2(420.0, 620.0))
        .with_title(ui::APP_NAME);
    if shooting {
        // Don't grab focus while a check takes the picture.
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    let options = eframe::NativeOptions {
        viewport,
        run_and_return: false,
        event_loop_builder: Some(event_loop_hook(shooting)),
        ..Default::default()
    };

    eframe::run_native(
        ui::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot_path.clone())))),
    )
}

#[cfg(not(target_os = "macos"))]
fn event_loop_hook(_shooting: bool) -> eframe::EventLoopBuilderHook {
    Box::new(|_builder: &mut eframe::EventLoopBuilder<eframe::UserEvent>| {})
}

/// On macOS: when taking the picture, don't raise the window over the
/// person's other apps.
#[cfg(target_os = "macos")]
fn event_loop_hook(shooting: bool) -> eframe::EventLoopBuilderHook {
    Box::new(move |builder: &mut eframe::EventLoopBuilder<eframe::UserEvent>| {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        if shooting {
            builder.with_activate_ignoring_other_apps(false);
        }
    })
}
