#[derive(Debug, serde::Serialize, serde::Deserialize, Copy, Clone)]
pub enum InputKeys {
    Start,
    Select,
    A,
    B,
    Up,
    Down,
    Left,
    Right,
    ShoulderLeft,
    ShoulderRight,
}
