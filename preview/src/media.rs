use std::path::PathBuf;

pub struct MediaItem {
    pub path: PathBuf,
    pub pos: egui::Pos2,
    pub size: egui::Vec2,
    texture: egui::TextureHandle,
}

impl MediaItem {
    pub fn new(path: PathBuf, ctx: &egui::Context) -> Self {
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

    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.pos, self.size)
    }

    pub fn draw(&self, ui: &egui::Ui) {
        ui.painter().image(
            self.texture.id(),
            self.rect(),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    pub fn draw_selection(&self, ui: &egui::Ui) {
        ui.painter().rect_stroke(
            self.rect(),
            0.0,
            egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
            egui::StrokeKind::Outside,
        );
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.size.x / self.size.y
    }
}
