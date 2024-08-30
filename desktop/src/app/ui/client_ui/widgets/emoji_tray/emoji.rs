use egui::{
    load::{BytesPoll, LoadError},
    vec2, Image, ImageButton, RichText,
};
include!(concat!(env!("OUT_DIR"), "\\emoji_header.rs"));

use crate::app::backend;

impl ToString for EmojiTypes
{
    fn to_string(&self) -> String
    {
        match self {
            EmojiTypes::AnimatedBlobs(_) => String::from("Animated blobs"),
            EmojiTypes::Blobs(_) => String::from("Blobs"),
            EmojiTypes::Foods(_) => String::from("Foods"),
            EmojiTypes::Icons(_) => String::from("Icons"),
            EmojiTypes::Letters(_) => String::from("Letters"),
            EmojiTypes::Numbers(_) => String::from("Numbers"),
            EmojiTypes::Turtles(_) => String::from("Turtles"),
        }
    }
}

impl backend::Application
{
    /// We return the name of the emoji selected, if none was selected in that frame we return None
    pub fn draw_emoji_selector(&mut self, ui: &mut egui::Ui, ctx: &egui::Context)
        -> Option<String>
    {
        ui.allocate_ui(vec2(400., 350.), |ui| {
            let scroll_area = egui::ScrollArea::vertical().show(ui, |ui| {
                for emoji_type in EMOJIS.emoji_types.iter() {
                    match emoji_type {
                        EmojiTypes::AnimatedBlobs(animated_blobs) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in animated_blobs {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Blobs(blobs) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in blobs {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Icons(icons) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in icons {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Letters(letters) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in letters {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Numbers(numbers) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in numbers {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Turtles(turtles) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in turtles {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                        EmojiTypes::Foods(foods) => {
                            ui.label(
                                RichText::from(emoji_type.to_string())
                                    .strong()
                                    .size(self.font_size),
                            );

                            //Display emojis
                            if let Some(emoji_name) = ui
                                .horizontal_wrapped(|ui| {
                                    for emoji in foods {
                                        if let Some(emoji_name) = ui
                                            .allocate_ui(vec2(25., 25.), |ui| {
                                                display_emoji(ctx, emoji.name, ui)
                                            })
                                            .inner
                                        {
                                            return Some(emoji_name);
                                        }
                                    }

                                    None
                                })
                                .inner
                            {
                                return Some(emoji_name);
                            }
                        },
                    };

                    ui.separator();
                }

                None
            });

            scroll_area.inner
        })
        .inner
    }
}

/// This will display the emoji under the given name, if it is not found in the egui image buffer it will automatically load it
pub fn display_emoji(ctx: &egui::Context, emoji_name: &str, ui: &mut egui::Ui) -> Option<String>
{
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
                    .add(
                        ImageButton::new(Image::from_uri(format!("bytes://{}", emoji_name)))
                            .frame(false),
                    )
                    .clicked()
                {
                    return Some(emoji_name.to_string());
                };
            }
        },
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
                }
                else {
                    tracing::error!("{}", inner);
                }
            }
            else {
                tracing::error!("{}", err);
            }
        },
    }

    None
}
