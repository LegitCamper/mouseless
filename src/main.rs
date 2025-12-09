use ashpd::desktop::screenshot::Screenshot;
use eframe::egui::{
    self, Color32, Context, Image, Pos2, Rect, Sense, Stroke, Vec2, ViewportBuilder, X11WindowType,
};
use eframe::{App, Frame};
use enigo::{Button, Coordinate, Direction::Press, Enigo, Mouse, Settings};
use std::i32;
use std::{fs::File, io::Read};

const GRID_SIZE: usize = 26;

fn idx_to_letter(idx: usize) -> char {
    (b'A' + idx as u8) as char
}

fn cell_code(x: usize, y: usize) -> String {
    let c1 = idx_to_letter(x); // column letter
    let c2 = idx_to_letter(y); // row letter
    format!("{}{}", c1, c2)
}

fn load_image_once<'a>(screenshot: String) -> Image<'a> {
    let mut image_file = File::open(screenshot).unwrap();
    let mut contents = Vec::new();

    image_file.read_to_end(&mut contents).unwrap();
    Image::from_bytes("bytes://desktop.bmp", contents)
}

struct GridOverlay<'a> {
    typed: String,
    screenshot: Image<'a>,
}

impl GridOverlay<'_> {
    fn new(screenshot: String) -> Self {
        let screenshot = load_image_once(screenshot);
        Self {
            typed: String::new(),
            screenshot,
        }
    }
}

static mut SELECTED: Option<(u32, u32)> = None;

impl App for GridOverlay<'_> {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.allocate_rect(rect, Sense::hover());

                self.screenshot.paint_at(ui, rect);

                let painter = ui.painter_at(rect);

                let cell_w = rect.width() / GRID_SIZE as f32;
                let cell_h = rect.height() / GRID_SIZE as f32;

                ui.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Key {
                            key,
                            physical_key: _,
                            pressed: _,
                            repeat: _,
                            modifiers: _,
                        } = event
                        {
                            if *key == egui::Key::Backspace {
                                let _ = self.typed.pop();
                            }
                        };
                        if let egui::Event::Text(text) = event {
                            for ch in text.chars() {
                                if ch.is_ascii_alphabetic() {
                                    let c = ch.to_ascii_uppercase();

                                    self.typed.push(c);
                                }
                            }
                        }
                    }
                });

                for y in 0..GRID_SIZE {
                    for x in 0..GRID_SIZE {
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(
                                rect.left() + x as f32 * cell_w,
                                rect.top() + y as f32 * cell_h,
                            ),
                            Vec2::new(cell_w, cell_h),
                        );

                        let code = cell_code(x, y);

                        let fill_color = Color32::from_rgba_premultiplied(50, 50, 50, 40);

                        painter.rect_filled(cell_rect, 0.0, fill_color);

                        // border
                        painter.rect_stroke(
                            cell_rect,
                            0.0,
                            Stroke::new(1., Color32::BLACK),
                            egui::StrokeKind::Outside,
                        );

                        // Draw the AA/AB/... text centered
                        let center = cell_rect.center();
                        painter.text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            &code,
                            egui::FontId::monospace(16.0),
                            if self.typed.starts_with(code.chars().next().unwrap())
                                && code.contains(&self.typed)
                            {
                                Color32::LIGHT_RED
                            } else {
                                Color32::WHITE
                            },
                        );
                    }
                }

                if self.typed.len() == 2 {
                    let c0 = self.typed.chars().nth(0).unwrap();
                    let c1 = self.typed.chars().nth(1).unwrap();

                    let x = (c0 as u8 - b'A') as usize;
                    let y = (c1 as u8 - b'A') as usize;

                    if x < GRID_SIZE && y < GRID_SIZE {
                        let cell_center_x = rect.left() + x as f32 * cell_w + cell_w / 2.0;

                        let cell_center_y = rect.top() + y as f32 * cell_h + cell_h / 2.0;

                        unsafe { SELECTED = Some((cell_center_x as u32, cell_center_y as u32)) };

                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
    }
}

async fn get_desktop() -> Result<String, ()> {
    let response = Screenshot::request()
        .interactive(false)
        .modal(true)
        .send()
        .await
        .unwrap()
        .response()
        .unwrap();

    let path = String::from(response.uri().path());
    Ok(path)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = get_desktop().await.unwrap();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_clamp_size_to_monitor_size(true)
            .with_window_type(X11WindowType::Utility)
            .with_mouse_passthrough(false)
            .with_decorations(false)
            .with_fullscreen(true)
            .with_maximized(true)
            .with_resizable(false)
            .with_always_on_top()
            .with_title("Mouseless"),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Mouseless",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(GridOverlay::new(screenshot)))
        }),
    )
    .unwrap();

    if let Some((cell_center_x, cell_center_y)) = unsafe { SELECTED } {
        let mut enigo = Enigo::new(&Settings::default()).unwrap();

        enigo
            .move_mouse(i32::MIN, i32::MIN, Coordinate::Rel)
            .unwrap();

        enigo
            .move_mouse(cell_center_x as i32, cell_center_y as i32, Coordinate::Rel)
            .unwrap();

        enigo.button(Button::Left, Press).unwrap();
    }

    Ok(())
}
