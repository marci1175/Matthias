use egui::{Color32, RichText};

use regex::Regex;

//use crate::app::account_manager::write_file;
use crate::app::backend::TemplateApp;

impl TemplateApp {
    pub fn markdown_text_display(&mut self, i: &String, ui: &mut egui::Ui) {
        if (i.contains('[') && i.contains(']')) && (i.contains('(') && i.contains(')')) {
            let re = Regex::new(r"\[(?P<link_text>[^\]]*)\]\((?P<link_target>[^)]+)\)").unwrap();
            let mut captures: Vec<String> = Vec::new();
            for capture in re.captures_iter(i) {
                for i in 1..capture.len() {
                    captures.push(capture[i].to_string());
                }
            }
            if captures.is_empty() {
                ui.label(RichText::from(i).size(self.font_size));
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::from(re.replace_all::<&str>(i, "")).size(self.font_size));
                    for i in (0..captures.len()).step_by(2) {
                        ui.add(egui::Hyperlink::from_label_and_url(
                            RichText::from(&captures[i]).size(self.font_size),
                            &captures[i + 1],
                        ));
                    }
                });
            }
        } else if let Some(index) = i.find('@') {
            let result = i[index + 1..].split_whitespace().collect::<Vec<&str>>()[0];
            if self.login_username == result {
                ui.label(
                    RichText::from(i)
                        .size(self.font_size)
                        .color(Color32::YELLOW),
                );
            }
        } else if i.contains('#') && i.rmatches('#').count() <= 5 {
            let split_lines = i.rsplit_once('#').unwrap();
            ui.horizontal(|ui| {
                ui.label(RichText::from(split_lines.0.replace('#', "")).size(self.font_size));
                ui.label(RichText::from(split_lines.1).strong().size(
                    self.font_size
                        * match i.rmatches('#').collect::<Vec<&str>>().len() {
                            1 => 2.0,
                            2 => 1.8,
                            3 => 1.6,
                            4 => 1.4,
                            5 => 1.2,
                            _ => 1.,
                        } as f32,
                ));
            });
        } else {
            ui.label(RichText::from(i).size(self.font_size));
        }
    }
}
