use egui::{
    load::{BytesPoll, LoadError},
    vec2, Image, ImageButton,
};
use std::collections::BTreeMap;

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

impl backend::Application {
    /// Iterates over all the characters
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

    /// We return the name of the emoji selected, if none was selected in that frame we reutrn None
    pub fn draw_emoji_selector(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<String> {
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
                    EmojiTypes::Foods(_) => ui.selectable_value(
                        &mut self.client_ui.emoji_tab_state,
                        backend::EmojiTypesDiscriminants::Foods,
                        "Foods",
                    ),
                };
            }
        });

        ui.separator();

        //This value will become a Some if the user selects (clicks) an emoji
        let mut selected_emoji: Option<String> = None;

        match self.client_ui.emoji_tab_state {
            backend::EmojiTypesDiscriminants::AnimatedBlobs => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::AnimatedBlobs(animated_blobs) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for animated_blob in animated_blobs {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, animated_blob.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Blobs => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Blobs(blobs) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for blob in blobs {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, blob.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Icons => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Icons(icons) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for icon in icons {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, icon.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Letters => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Letters(letters) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for letter in letters {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, letter.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Numbers => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Numbers(numbers) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for number in numbers {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, number.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Turtles => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Turtles(turtles) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for turtle in turtles {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, turtle.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
            backend::EmojiTypesDiscriminants::Foods => {
                //If we have selected an emoji we just return it, god forgive me for this piece of code
                if selected_emoji.is_some() {
                    return selected_emoji;
                }
                
                for emoji_type in EMOJIS.emoji_types.iter() {
                    //If its some we break the loop
                    if selected_emoji.is_some() {
                        break;
                    }

                    if let EmojiTypes::Foods(foods) = emoji_type {
                        ui.horizontal_wrapped(|ui| {
                            for food in foods {
                                ui.allocate_ui(vec2(30., 30.), |ui| {
                                    selected_emoji = display_emoji(ctx, food.name, ui);
                                });

                                //If its some we break the loop
                                if selected_emoji.is_some() {
                                    break;
                                }
                            }
                        });
                    }
                }
            }
        };

        selected_emoji
    }
}

/// This will display the emoji under the given name, if it is not found in the egui image buffer it will automaticly load it
pub fn display_emoji(
    ctx: &egui::Context,
    emoji_name: &str,
    ui: &mut egui::Ui,
) -> Option<String> {
    match ctx.try_load_bytes(&format!("bytes://{}", emoji_name)) {
        Ok(bytespoll) => {
            if let BytesPoll::Ready {
                size: _,
                bytes,
                mime: _,
            } = bytespoll
            {
                if bytes.to_vec() == vec![0] {
                    eprintln!(
                        "The called emoji was not found in the emoji header: {}",
                        emoji_name
                    );
                }
                if ui
                    .add(ImageButton::new(Image::from_uri(&format!(
                        "bytes://{}",
                        emoji_name
                    ))).frame(false))
                    .clicked()
                {
                    return Some(emoji_name.to_string());
                };
            }
        }
        Err(err) => {
            if let LoadError::Loading(inner) = err {
                if inner == "Bytes not found. Did you forget to call Context::include_bytes?" {
                    //check if we are visible, so there are no unnecessary requests
                    if !ui.is_rect_visible(ui.min_rect()) {
                        return None;
                    }

                    ctx.include_bytes(
                        format!("bytes://{}", &emoji_name),
                        EMOJI_TUPLES
                            .get(emoji_name)
                            .map_or_else(|| vec![0], |v| v.to_vec()),
                    );
                } else {
                    dbg!(inner);
                }
            } else {
                dbg!(err);
            }
        }
    }

    None
}
