use egui::Context;

pub struct EUI {
    context: Context,
}

impl EUI {
    pub fn new() -> Self {
        Self {
            context: Context::default()
        }
    }
}