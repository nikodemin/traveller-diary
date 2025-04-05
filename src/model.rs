use chrono::prelude::*;
use egui::Image;

#[derive(Debug, Clone)]
pub struct AppState {
    pub language: String,
}

pub enum Event {
    ChangeLanguage,
}
