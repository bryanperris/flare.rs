pub struct SoftRenderOptions {
    pub enable: bool,
    pub use_clip: bool,
    pub use_clip_left: bool,
    pub use_clip_right: bool,
    pub use_clip_bottom: bool,
    pub use_clip_top: bool,
    pub use_clip_far: bool,
}

impl Default for SoftRenderOptions {
    fn default() -> Self {
        Self {
            use_clip: Default::default(),
            use_clip_left: Default::default(),
            use_clip_right: Default::default(),
            use_clip_bottom: Default::default(),
            use_clip_top: Default::default(),
            use_clip_far: Default::default(),
            enable: false,
        }
    }
}
