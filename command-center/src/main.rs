mod export;
mod media;
mod resize;

#[derive(Default)]
pub struct DebugApp {
    items: Vec<media::MediaItem>,
    selected_item: Option<usize>,
}

/// Implementation of the resize module in the main App
fn resize_ui(ui: &mut egui::Ui, item: &mut media::MediaItem) {
    let rect = item.rect();

    let aspect = item.aspect_ratio();
    let lock_aspect = ui.input(|i| i.modifiers.shift);

    for handle in resize::ResizeHandle::all() {
        let handle_rect = handle.rect(rect);
        let response = ui.allocate_rect(handle_rect, egui::Sense::drag());

        ui.painter()
            .rect_filled(handle_rect, 2.0, egui::Color32::YELLOW);

        if response.dragged() {
            handle.apply(
                &mut item.pos,
                &mut item.size,
                response.drag_delta(),
                aspect,
                lock_aspect,
            );
        }

        response.on_hover_cursor(handle.cursor());
    }
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // The settings gui
        egui::Window::new("Settings").show(ctx, |ui| {
            if ui.button("Add media").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.items.push(media::MediaItem::new(path, ctx));
            }

            if ui.button("Delete selected").clicked()
                && let Some(i) = self.selected_item
            {
                self.items.remove(i);
                self.selected_item = None;
            }

            if ui.button("Export").clicked() {
                let layout: Vec<export::LayoutItem> =
                    self.items.iter().map(export::LayoutItem::from).collect();
            }
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                for (i, item) in self.items.iter_mut().enumerate() {
                    let rect = item.rect();
                    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                    if response.dragged() {
                        item.pos += response.drag_delta();
                    }

                    if response.clicked() {
                        self.selected_item = Some(i);
                    }

                    item.draw(ui);

                    if self.selected_item == Some(i) {
                        item.draw_selection(ui);
                        resize_ui(ui, item);
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
