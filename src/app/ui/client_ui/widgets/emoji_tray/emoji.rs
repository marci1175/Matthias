use egui::{vec2, Image, ImageButton, RichText};
use std::{any::Any, collections::BTreeMap};

include!(concat!(env!("OUT_DIR"), "\\emoji_header.rs"));

use crate::app::backend;

fn char_name(chr: char) -> String {
    special_char_name(chr)
        .map(|s| s.to_owned())
        //Deleted unciode names crate cuz of big bin size
        // .or_else(|| unicode_names2::name(chr).map(|name| name.to_string().to_lowercase()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn special_char_name(chr: char) -> Option<&'static str> {
    #[allow(clippy::match_same_arms)] // many "flag"
    match chr {
        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Private use area extensions:
        '\u{FE4E5}' => Some("flag japan"),
        '\u{FE4E6}' => Some("flag usa"),
        '\u{FE4E7}' => Some("flag"),
        '\u{FE4E8}' => Some("flag"),
        '\u{FE4E9}' => Some("flag"),
        '\u{FE4EA}' => Some("flag great britain"),
        '\u{FE4EB}' => Some("flag"),
        '\u{FE4EC}' => Some("flag"),
        '\u{FE4ED}' => Some("flag"),
        '\u{FE4EE}' => Some("flag south korea"),
        '\u{FE82C}' => Some("number sign in square"),
        '\u{FE82E}' => Some("digit one in square"),
        '\u{FE82F}' => Some("digit two in square"),
        '\u{FE830}' => Some("digit three in square"),
        '\u{FE831}' => Some("digit four in square"),
        '\u{FE832}' => Some("digit five in square"),
        '\u{FE833}' => Some("digit six in square"),
        '\u{FE834}' => Some("digit seven in square"),
        '\u{FE835}' => Some("digit eight in square"),
        '\u{FE836}' => Some("digit nine in square"),
        '\u{FE837}' => Some("digit zero in square"),

        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Web services / operating systems / browsers
        '\u{E600}' => Some("web-dribbble"),
        '\u{E601}' => Some("web-stackoverflow"),
        '\u{E602}' => Some("web-vimeo"),
        '\u{E603}' => Some("web-twitter"),
        '\u{E604}' => Some("web-facebook"),
        '\u{E605}' => Some("web-googleplus"),
        '\u{E606}' => Some("web-pinterest"),
        '\u{E607}' => Some("web-tumblr"),
        '\u{E608}' => Some("web-linkedin"),
        '\u{E60A}' => Some("web-stumbleupon"),
        '\u{E60B}' => Some("web-lastfm"),
        '\u{E60C}' => Some("web-rdio"),
        '\u{E60D}' => Some("web-spotify"),
        '\u{E60E}' => Some("web-qq"),
        '\u{E60F}' => Some("web-instagram"),
        '\u{E610}' => Some("web-dropbox"),
        '\u{E611}' => Some("web-evernote"),
        '\u{E612}' => Some("web-flattr"),
        '\u{E613}' => Some("web-skype"),
        '\u{E614}' => Some("web-renren"),
        '\u{E615}' => Some("web-sina-weibo"),
        '\u{E616}' => Some("web-paypal"),
        '\u{E617}' => Some("web-picasa"),
        '\u{E618}' => Some("os-android"),
        '\u{E619}' => Some("web-mixi"),
        '\u{E61A}' => Some("web-behance"),
        '\u{E61B}' => Some("web-circles"),
        '\u{E61C}' => Some("web-vk"),
        '\u{E61D}' => Some("web-smashing"),
        '\u{E61E}' => Some("web-forrst"),
        '\u{E61F}' => Some("os-windows"),
        '\u{E620}' => Some("web-flickr"),
        '\u{E621}' => Some("web-picassa"),
        '\u{E622}' => Some("web-deviantart"),
        '\u{E623}' => Some("web-steam"),
        '\u{E624}' => Some("web-github"),
        '\u{E625}' => Some("web-git"),
        '\u{E626}' => Some("web-blogger"),
        '\u{E627}' => Some("web-soundcloud"),
        '\u{E628}' => Some("web-reddit"),
        '\u{E629}' => Some("web-delicious"),
        '\u{E62A}' => Some("browser-chrome"),
        '\u{E62B}' => Some("browser-firefox"),
        '\u{E62C}' => Some("browser-ie"),
        '\u{E62D}' => Some("browser-opera"),
        '\u{E62E}' => Some("browser-safari"),
        '\u{E62F}' => Some("web-google-drive"),
        '\u{E630}' => Some("web-wordpress"),
        '\u{E631}' => Some("web-joomla"),
        '\u{E632}' => Some("lastfm"),
        '\u{E633}' => Some("web-foursquare"),
        '\u{E634}' => Some("web-yelp"),
        '\u{E635}' => Some("web-drupal"),
        '\u{E636}' => Some("youtube"),
        '\u{F189}' => Some("vk"),
        '\u{F1A6}' => Some("digg"),
        '\u{F1CA}' => Some("web-vine"),
        '\u{F8FF}' => Some("os-apple"),

        // Special private-use-area extensions found in `Ubuntu-Light.ttf`
        '\u{F000}' => Some("uniF000"),
        '\u{F001}' => Some("fi"),
        '\u{F002}' => Some("fl"),
        '\u{F506}' => Some("one seventh"),
        '\u{F507}' => Some("two sevenths"),
        '\u{F508}' => Some("three sevenths"),
        '\u{F509}' => Some("four sevenths"),
        '\u{F50A}' => Some("five sevenths"),
        '\u{F50B}' => Some("six sevenths"),
        '\u{F50C}' => Some("one ninth"),
        '\u{F50D}' => Some("two ninths"),
        '\u{F50E}' => Some("four ninths"),
        '\u{F50F}' => Some("five ninths"),
        '\u{F510}' => Some("seven ninths"),
        '\u{F511}' => Some("eight ninths"),
        '\u{F800}' => Some("zero.alt"),
        '\u{F801}' => Some("one.alt"),
        '\u{F802}' => Some("two.alt"),
        '\u{F803}' => Some("three.alt"),
        '\u{F804}' => Some("four.alt"),
        '\u{F805}' => Some("five.alt"),
        '\u{F806}' => Some("six.alt"),
        '\u{F807}' => Some("seven.alt"),
        '\u{F808}' => Some("eight.alt"),
        '\u{F809}' => Some("nine.alt"),
        '\u{F80A}' => Some("zero.sups"),
        '\u{F80B}' => Some("one.sups"),
        '\u{F80C}' => Some("two.sups"),
        '\u{F80D}' => Some("three.sups"),
        '\u{F80E}' => Some("four.sups"),
        '\u{F80F}' => Some("five.sups"),
        '\u{F810}' => Some("six.sups"),
        '\u{F811}' => Some("seven.sups"),
        '\u{F812}' => Some("eight.sups"),
        '\u{F813}' => Some("nine.sups"),
        '\u{F814}' => Some("zero.sinf"),
        '\u{F815}' => Some("one.sinf"),
        '\u{F816}' => Some("two.sinf"),
        '\u{F817}' => Some("three.sinf"),
        '\u{F818}' => Some("four.sinf"),
        '\u{F819}' => Some("five.sinf"),
        '\u{F81A}' => Some("six.sinf"),
        '\u{F81B}' => Some("seven.sinf"),
        '\u{F81C}' => Some("eight.sinf"),
        '\u{F81D}' => Some("nine.sinf"),

        _ => None,
    }
}

impl backend::TemplateApp {
    pub fn available_characters(ui: &egui::Ui, family: egui::FontFamily) -> BTreeMap<char, String> {
        ui.fonts(|f| {
            f.lock()
                .fonts
                .font(&egui::FontId::new(10.0, family)) // size is arbitrary for getting the characters
                .characters()
                .iter()
                .filter(|chr| !chr.is_whitespace() && !chr.is_ascii_control())
                .map(|&chr| (chr, char_name(chr)))
                .collect()
        })
    }

    pub fn draw_emoji_selector(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<Option<()>> {
        let emoji_button = ui.menu_button(
            RichText::from(&self.client_ui.random_emoji).size(self.font_size * 1.2),
            |ui| {
                //Main emoji tabs
                ui.horizontal_top(|ui| {
                    for emoji_type in EMOJIS.emoji_types.iter() {
                        match emoji_type {
                            EmojiTypes::AnimatedBlobs(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::AnimatedBlobs,
                                "Animated Blobs",
                            ),
                            EmojiTypes::Blobs(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::Blobs,
                                "Blobs",
                            ),
                            EmojiTypes::Icons(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::Icons,
                                "Icons",
                            ),
                            EmojiTypes::Letters(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::Letters,
                                "Letters",
                            ),
                            EmojiTypes::Numbers(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::Numbers,
                                "Numbers",
                            ),
                            EmojiTypes::Turtles(_) => ui.selectable_value(
                                &mut self.client_ui.emoji_tab_state,
                                backend::EmojiTypesDiscriminants::Turtles,
                                "Turtles",
                            ),
                        };
                    }
                });

                ui.separator();

                match self.client_ui.emoji_tab_state {
                    backend::EmojiTypesDiscriminants::AnimatedBlobs => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::AnimatedBlobs(animated_blobs) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for animated_blob in animated_blobs {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", animated_blob.name),
                                                    animated_blob.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &animated_blob.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                    backend::EmojiTypesDiscriminants::Blobs => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::Blobs(blobs) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for blob in blobs {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", blob.name),
                                                    blob.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &blob.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                    backend::EmojiTypesDiscriminants::Icons => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::Icons(icons) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for icon in icons {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", icon.name),
                                                    icon.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &icon.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                    backend::EmojiTypesDiscriminants::Letters => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::Letters(letters) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for letter in letters {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", letter.name),
                                                    letter.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &letter.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                    backend::EmojiTypesDiscriminants::Numbers => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::Numbers(numbers) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for number in numbers {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", number.name),
                                                    number.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &number.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                    backend::EmojiTypesDiscriminants::Turtles => {
                        for emoji_type in EMOJIS.emoji_types.iter() {
                            if let EmojiTypes::Turtles(turtles) = emoji_type {
                                ui.horizontal_wrapped(|ui| {
                                    for turtle in turtles {
                                        ui.allocate_ui(vec2(30., 30.), |ui| {
                                            if ui
                                                .add(ImageButton::new(Image::from_bytes(
                                                    format!("bytes://{}", turtle.name),
                                                    turtle.bytes,
                                                )))
                                                .clicked()
                                            {
                                                let is_inserting_front =
                                                    self.client_ui.text_edit_cursor_index
                                                        == self.client_ui.message_edit_buffer.len();

                                                self.client_ui.message_edit_buffer.insert_str(
                                                    self.client_ui.text_edit_cursor_index,
                                                    &format!(":{}:", &turtle.name),
                                                );

                                                if is_inserting_front {
                                                    self.client_ui.text_edit_cursor_index =
                                                        self.client_ui.message_edit_buffer.len();
                                                }
                                            };
                                        });
                                    }
                                });
                            }
                        }
                    }
                }
            },
        );
        emoji_button
    }
}
