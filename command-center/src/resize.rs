/// Size of the resize handle (the rectangle you need to grab to resize)
const HANDLE_SIZE: f32 = 10.0;
/// Minimum size of the resized element
const MIN_SIZE: f32 = 20.0;

#[derive(Copy, Clone)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeHandle {
    pub fn all() -> [Self; 4] {
        [
            Self::TopLeft,
            Self::TopRight,
            Self::BottomLeft,
            Self::BottomRight,
        ]
    }

    /// Give the rectangle to draw for a given ResizeHandle
    pub fn rect(self, rect: egui::Rect) -> egui::Rect {
        let hs = egui::vec2(HANDLE_SIZE, HANDLE_SIZE);
        let offset = egui::vec2(HANDLE_SIZE / 2.0, HANDLE_SIZE / 2.0);

        let pos = match self {
            Self::TopLeft => rect.left_top() - offset,
            Self::TopRight => rect.right_top() - offset,
            Self::BottomLeft => rect.left_bottom() - offset,
            Self::BottomRight => rect.right_bottom() - offset,
        };

        egui::Rect::from_min_size(pos, hs)
    }

    /// Apply a resizing delta to a size and position
    pub fn apply(self, pos: &mut egui::Pos2, size: &mut egui::Vec2, delta: egui::Vec2) {
        match self {
            Self::TopLeft => {
                *pos += delta;
                *size -= delta;
            }
            Self::TopRight => {
                pos.y += delta.y;
                size.x += delta.x;
                size.y -= delta.y;
            }
            Self::BottomLeft => {
                pos.x += delta.x;
                size.x -= delta.x;
                size.y += delta.y;
            }
            Self::BottomRight => {
                *size += delta;
            }
        }

        size.x = size.x.max(MIN_SIZE);
        size.y = size.y.max(MIN_SIZE);
    }
}
