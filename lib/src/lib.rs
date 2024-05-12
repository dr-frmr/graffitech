use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "graffitech:graffitech:mothu.eth";

#[derive(Serialize, Deserialize)]
pub struct CanvasMessage {
    pub x: f64,
    pub y: f64,
}

impl std::fmt::Display for CanvasMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
