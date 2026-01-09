use std::path::PathBuf;

use serde::Serialize;

const HANDLE_SIZE: f32 = 10.0;

struct MediaItem {
    path: PathBuf,
    pos: egui::Pos2,
    size: egui::Vec2,
    texture: egui::TextureHandle,
}

impl MediaItem {
    fn new(path: PathBuf, ctx: &egui::Context) -> Self {
        let image = image::ImageReader::open(path.clone())
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let size: [usize; 2] = [image.width() as usize, image.height() as usize];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let texture = ctx.load_texture(
            path.file_name().unwrap().to_string_lossy(),
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            egui::TextureOptions::default(),
        );

        Self {
            path,
            pos: egui::Pos2::default(),
            size: egui::vec2(image.width() as f32, image.height() as f32),
            texture,
        }
    }
}

#[derive(Serialize)]
struct LayoutItem {
    path: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl From<&MediaItem> for LayoutItem {
    fn from(item: &MediaItem) -> Self {
        Self {
            path: item.path.clone().into_os_string().into_string().unwrap(),
            x: item.pos.x,
            y: item.pos.y,
            w: item.size.x,
            h: item.size.y,
        }
    }
}

#[derive(Default)]
pub struct DebugApp {
    items: Vec<MediaItem>,
    selected_item: Option<usize>,
}

fn resize_handles(rect: egui::Rect) -> [egui::Rect; 4] {
    let hs = egui::vec2(HANDLE_SIZE, HANDLE_SIZE);

    let top_left = egui::Pos2::new(
        rect.left() - (HANDLE_SIZE / 2.0),
        rect.top() - (HANDLE_SIZE / 2.0),
    );

    let top_right = egui::Pos2::new(
        rect.right() - (HANDLE_SIZE / 2.0),
        rect.top() - (HANDLE_SIZE / 2.0),
    );

    let bottom_left = egui::Pos2::new(
        rect.left() - (HANDLE_SIZE / 2.0),
        rect.bottom() - (HANDLE_SIZE / 2.0),
    );

    let bottom_right = egui::Pos2::new(
        rect.right() - (HANDLE_SIZE / 2.0),
        rect.bottom() - (HANDLE_SIZE / 2.0),
    );

    [
        egui::Rect::from_min_size(top_left, hs),
        egui::Rect::from_min_size(top_right, hs),
        egui::Rect::from_min_size(bottom_left, hs),
        egui::Rect::from_min_size(bottom_right, hs),
    ]
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // The settings gui
        egui::Window::new("Settings").show(ctx, |ui| {
            if ui.button("Add media").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.items.push(MediaItem::new(path, ctx));
            }

            if ui.button("Delete selected").clicked()
                && let Some(i) = self.selected_item
            {
                self.items.remove(i);
                self.selected_item = None;
            }

            if ui.button("Export").clicked() {
                let layout: Vec<LayoutItem> = self.items.iter().map(LayoutItem::from).collect();
            }
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                for (i, item) in self.items.iter_mut().enumerate() {
                    let rect = egui::Rect::from_min_size(item.pos, item.size);
                    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                    if response.dragged() {
                        item.pos += response.drag_delta();
                    }

                    if response.clicked() {
                        self.selected_item = Some(i);
                    }

                    ui.painter().image(
                        item.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    if self.selected_item == Some(i) {
                        println!("Selected item : {}", i);
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                            egui::StrokeKind::Outside,
                        );

                        for (handle_index, handle_rect) in
                            resize_handles(rect).into_iter().enumerate()
                        {
                            let response = ui.allocate_rect(handle_rect, egui::Sense::drag());

                            ui.painter()
                                .rect_filled(handle_rect, 2.0, egui::Color32::YELLOW);

                            if response.dragged() {
                                let delta = response.drag_delta();

                                match handle_index {
                                    0 => {
                                        // TL
                                        item.pos += delta;
                                        item.size -= delta;
                                    }
                                    1 => {
                                        // TR
                                        item.pos.y += delta.y;
                                        item.size.x += delta.x;
                                        item.size.y -= delta.y;
                                    }
                                    2 => {
                                        // BL
                                        item.pos.x += delta.x;
                                        item.size.x -= delta.x;
                                        item.size.y += delta.y;
                                    }
                                    3 => {
                                        // BR
                                        item.size += delta;
                                    }
                                    _ => {}
                                }

                                item.size.x = item.size.x.max(20.0);
                                item.size.y = item.size.y.max(20.0);
                            }
                        }
                    }
                }
            });
    }
}

fn main() {
    let viewport = egui::ViewportBuilder::default()
        .with_fullscreen(true)
        .with_transparent(true)
        .with_has_shadow(false);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "FriendlyFire Debug",
        options,
        Box::new(|_cc| Ok(Box::new(DebugApp::default()))),
    )
    .unwrap();
}
