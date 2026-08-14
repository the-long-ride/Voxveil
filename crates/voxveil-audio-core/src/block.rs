#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

impl StereoFrame {
    pub const fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }
}
