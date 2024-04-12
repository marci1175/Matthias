use egui::{Context, RichText};

use crate::app::backend::{ServerMessageType, ServerOutput, TemplateApp};

impl TemplateApp {
    pub fn deleted_message(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        input: &ServerOutput,
    ) {
        if let ServerMessageType::Deleted = input.MessageType {
            ui.label(RichText::from("Deleted message").strong().size(self.font_size));
        }
    }
}