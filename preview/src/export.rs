use crate::media;

#[derive(serde::Serialize)]
pub struct LayoutItem {
    path: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl From<&media::MediaItem> for LayoutItem {
    fn from(item: &media::MediaItem) -> Self {
        Self {
            path: item.path.clone().into_os_string().into_string().unwrap(),
            x: item.pos.x,
            y: item.pos.y,
            w: item.size.x,
            h: item.size.y,
        }
    }
}
