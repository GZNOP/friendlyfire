use std::path::PathBuf;

use serde::Serialize;

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

            if ui.button("Export").clicked() {
                let layout: Vec<LayoutItem> = self.items.iter().map(LayoutItem::from).collect();
            }
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                for item in &mut self.items {
                    let rect = egui::Rect::from_min_size(item.pos, item.size);
                    let response = ui.allocate_rect(rect, egui::Sense::drag());

                    if response.dragged() {
                        item.pos += response.drag_delta();
                    }

                    ui.painter().image(
                        item.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_secs(1));
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
