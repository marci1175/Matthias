use egui::{Pos2, vec2};

use crate::app::backend::{self, TemplateApp};

impl backend::TemplateApp {
    pub fn emoji_reaction_instance(&mut self, ctx: &egui::Context, index: usize ,pos: egui::Rect) -> egui::InnerResponse<()> {
        egui::Area::new("Reaction_tray")
        .fixed_pos(pos.max)
        .movable(false)
        .show(ctx, |ui| {
            let filter = &self.filter;
                let named_chars = self.named_chars
                    .entry(egui::FontFamily::Monospace)
                    .or_insert_with(|| TemplateApp::available_characters(ui, egui::FontFamily::Monospace));
                
                ui.group(|ui| {
                    ui.allocate_ui(vec2(300., 300.), |ui|{
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);
        
                                for (&chr, name) in named_chars {
                                    if filter.is_empty()
                                        || name.contains(filter)
                                        || *filter == chr.to_string()
                                    {
                                        let button = egui::Button::new(
                                            egui::RichText::new(chr.to_string()).font(egui::FontId {
                                                size: self.font_size,
                                                family: egui::FontFamily::Proportional,
                                            }),
                                        )
                                        .frame(false);
        
                                        if ui.add(button).clicked() {
                                            
                                        }
                                    }
                                }
                            });
                        });
                    });
                });
        })
    }
    
}