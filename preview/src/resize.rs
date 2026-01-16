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

    /// Give the right mouse cursor for each ResizeHandle
    pub fn cursor(self) -> egui::CursorIcon {
        match self {
            Self::TopLeft | Self::BottomRight => egui::CursorIcon::ResizeNwSe,
            Self::TopRight | Self::BottomLeft => egui::CursorIcon::ResizeNeSw,
        }
    }

    /// Apply a resizing delta to a size and position
    pub fn apply(
        self,
        pos: &mut egui::Pos2,
        size: &mut egui::Vec2,
        delta: egui::Vec2,
        aspect_ratio: f32,
        lock_aspect: bool,
    ) {
        let mut delta = delta;

        // if shift button is pressed
        if lock_aspect {
            // Handle when shift button is pressed 
            delta.y = delta.x / (size.x / size.y);
            match self {
                Self::TopLeft => {
                    if size.x - delta.x >= MIN_SIZE && size.y - delta.y >= MIN_SIZE {
                        *size -= delta;
                        *pos += delta;
                    }
                }
                Self::TopRight => {
                    if size.y + delta.y >= MIN_SIZE && size.x + delta.x >= MIN_SIZE {
                        size.y += delta.y;
                        pos.y -= delta.y;
                        size.x += delta.x;
                     }
                }
                Self::BottomLeft => {
                    if size.x - delta.x >= MIN_SIZE && size.y - delta.y >= MIN_SIZE {
                        size.x -= delta.x;
                        pos.x += delta.x;
                        size.y -= delta.y;
                    }                   
                }
                Self::BottomRight => {
                    if size.x + delta.x >= MIN_SIZE && size.y + delta.y >= MIN_SIZE {
                        *size += delta;
                    }
                }
            }
        } else {
            // Without shift button pressed (i.e. image stretching allowed)
            match self {
                Self::TopLeft => {
                    if size.x - delta.x >= MIN_SIZE {
                        size.x -= delta.x;
                        pos.x += delta.x; 
                    }
                    if size.y - delta.y >= MIN_SIZE {
                        size.y -= delta.y;
                        pos.y += delta.y;
                    }
                }                
                Self::TopRight => {
                    if size.y - delta.y >= MIN_SIZE {
                        size.y -= delta.y;
                        pos.y += delta.y;
                    }
                    size.x += delta.x;
                }
                Self::BottomLeft => {
                    if size.x - delta.x >= MIN_SIZE {
                        size.x -= delta.x;
                        pos.x += delta.x;
                    }
                    size.y += delta.y;
                }
                Self::BottomRight => {
                    *size += delta;          
                }
            }
        }
        
        size.x = size.x.max(MIN_SIZE);
        size.y = size.y.max(MIN_SIZE);
    }
}
