use egui::{Color32, RichText};

use regex::Regex;
use tap::Tap;

//use crate::app::account_manager::write_file;
use crate::app::backend::TemplateApp;

impl TemplateApp {
    pub fn markdown_text_display(&mut self, input: &String, ui: &mut egui::Ui) {
        if (input.contains('[') && input.contains(']'))
            && (input.contains('(') && input.contains(')'))
        {
            let regex = Regex::new(r"\[\s*(?P<text>[^\]]*)\]\((?P<link_target>[^)]+)\)").unwrap();

            let mut captures: Vec<String> = Vec::new();

            for capture in regex.captures_iter(input) {
                //We iterate over all the captures
                for i in 1..capture.len() {
                    //We push back the captures into the captures vector
                    captures.push(capture[i].to_string());
                }
            }

            if captures.is_empty() {
                ui.label(RichText::from(input).size(self.font_size));
            } else {
                ui.horizontal(|ui| {
                    let input_clone = input.clone();

                    let temp = input_clone.split_whitespace().collect::<Vec<_>>();

                    for item in temp.iter() {
                        if let Some(capture) = regex.captures(item) {
                            // capture[0] combined
                            // capture[1] disp
                            // capture[2] URL
                            ui.hyperlink_to(capture[1].to_string(), capture[2].to_string());
                        } else {
                            ui.label(*item);
                        }
                    }
                });
            }
        } else if let Some(index) = input.find('@') {
            let result = input[index + 1..].split_whitespace().collect::<Vec<&str>>()[0];
            if self.login_username == result {
                ui.label(
                    RichText::from(input)
                        .size(self.font_size)
                        .color(Color32::YELLOW),
                );
            }
        } else if input.contains('#') && input.rmatches('#').count() <= 5 {
            let split_lines = input.rsplit_once('#').unwrap();
            ui.horizontal(|ui| {
                ui.label(RichText::from(split_lines.0.replace('#', "")).size(self.font_size));
                ui.label(RichText::from(split_lines.1).strong().size(
                    self.font_size
                        * match input.rmatches('#').collect::<Vec<&str>>().len() {
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
            ui.label(RichText::from(input).size(self.font_size));
        }
    }
}
