use std::{env, fs, io::Cursor, path::PathBuf};

use crate::app::backend::{display_error_message, register, Application, ProfileImage, Register};
use anyhow::bail;
use egui::{
    vec2, Area, Color32, Id, Image, ImageButton, LayerId, Pos2, Rect, RichText, Slider, Stroke,
    TextEdit,
};
use egui_extras::DatePickerButton;
use image::{io::Reader as ImageReader, DynamicImage};

impl Application
{
    pub fn state_register(&mut self, _frame: &mut eframe::Frame, ctx: &egui::Context)
    {
        egui::TopBottomPanel::top("register_menu").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui(vec2(40., 50.), |ui| {
                        if ui
                            .add(ImageButton::new(egui::include_image!(
                                "../../../../assets/icons/logout.png"
                            )))
                            .clicked()
                        {
                            self.main.register_mode = false;

                            //Reset register state
                            self.register = Register::default();
                        };
                    });
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::from("Create a Matthias account")
                                .strong()
                                .size(35.),
                        )
                    });
                });
            });
        });

        if let Ok(app_data_path) = env::var("APPDATA") {
            egui::CentralPanel::default().show(ctx, |ui| {
                //Username and password
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.label(RichText::from("Enter credentials").size(20.).strong());
                        ui.label("Username");
                        ui.text_edit_singleline(&mut self.register.username);
                        ui.label("Password");
                        ui.add(TextEdit::singleline(&mut self.register.password).password(true));

                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Birthdate:");
                            ui.add(DatePickerButton::new(&mut self.register.birth_date));
                        });

                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Gender");
                            egui::ComboBox::from_label("Select one")
                                .selected_text(match self.register.gender {
                                    Some(false) => "Male",
                                    Some(true) => "Female",
                                    None => "Rather not answer",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.register.gender,
                                        Some(false),
                                        "Male",
                                    );
                                    ui.selectable_value(
                                        &mut self.register.gender,
                                        Some(true),
                                        "Female",
                                    );
                                    ui.selectable_value(
                                        &mut self.register.gender,
                                        None,
                                        "Rather not answer",
                                    );
                                });
                        });

                        ui.separator();

                        ui.add_enabled_ui(
                            self.register.gender.is_some()
                                && !(self.register.normal_profile_picture.is_empty()
                                    || self.register.small_profile_picture.is_empty()
                                    || self.register.password.is_empty()
                                    || self.register.username.is_empty()),
                            |ui| {
                                if ui.button(RichText::from("Register").strong()).clicked() {
                                    match register(self.register.clone()) {
                                        Ok(user_information) => {
                                            //Redirect the user immediately after registering
                                            self.main.client_mode = true;
                                            self.main.register_mode = false;

                                            self.opened_user_information = user_information;
                                        },
                                        Err(err) => {
                                            //Avoid panicking when trying to display a Notification
                                            //This is very rare but can still happen
                                            display_error_message(err, self.toasts.clone());
                                        },
                                    }
                                };
                            },
                        );
                    });
                    columns[1].vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Upload profile picture");
                            if !self.register.normal_profile_picture.is_empty() {
                                ui.label(RichText::from("Success!").color(Color32::GREEN));
                            }
                        });

                        // ui.label(RichText::from("You can only set pngs as profile pictures right now, this will be fixed in a later release").weak());
                        if let Some(image) = self.register.image.selected_image_bytes.clone() {
                            let center_pos = Pos2::new(
                                ui.available_width() / 2. * 3.,
                                ui.available_height() / 2.,
                            );

                            //I dont know why it needs to be a 100 to work, please dont ever touch ever touch this again
                            let size_of_side = 100.;
                            //I dont know why it needs to be a 100 to work, please dont ever touch ever touch this again

                            let left_top = Pos2::new(
                                center_pos.x - (size_of_side / 2.),
                                center_pos.y + (size_of_side / 2.),
                            );
                            let right_bottom = Pos2::new(
                                center_pos.x + (size_of_side / 2.),
                                center_pos.y - (size_of_side / 2.),
                            );

                            //Create rect
                            let rectangle_rect = Rect {
                                min: left_top,
                                max: right_bottom,
                            };

                            draw_rect(
                                ui,
                                Stroke::new(1., Color32::WHITE),
                                center_pos,
                                self.register.image.image_size,
                            );

                            //Draw background
                            ui.painter().rect_filled(
                                Rect::everything_right_of(ui.available_width()),
                                0.,
                                Color32::from_black_alpha(160),
                            );
                            Area::new(Id::new("IMAGE_SELECTOR_CONTROLS"))
                                .fixed_pos(Pos2::new(rectangle_rect.min.x, rectangle_rect.min.y))
                                .show(ctx, |ui| {
                                    //Format it
                                    ui.allocate_space(vec2(ui.available_width(), 5.));

                                    if ui.button("Cancel").clicked() {
                                        //Reset state
                                        self.register.image = ProfileImage::default();
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label("Zoom");
                                        if image.height() > image.width() {
                                            ui.add(Slider::new(
                                                &mut self.register.image.image_size,
                                                0.0..=image.height() as f32,
                                            ));
                                        }
                                        else {
                                            ui.add(Slider::new(
                                                &mut self.register.image.image_size,
                                                0.0..=image.width() as f32,
                                            ));
                                        }
                                    });

                                    if ui.button("Save").clicked() {
                                        let cropped_img: DynamicImage = image.crop_imm(
                                            (rectangle_rect.left()
                                                - self.register.image.image_rect.left()
                                                + (100. - self.register.image.image_size))
                                                as u32,
                                            (rectangle_rect.max.y
                                                - self.register.image.image_rect.min.y
                                                + (100. - self.register.image.image_size) / 2.)
                                                as u32,
                                            self.register.image.image_size as u32,
                                            self.register.image.image_size as u32,
                                        );

                                        if let Err(err) =
                                            self.save_image(cropped_img, app_data_path, ctx)
                                        {
                                            //Avoid panicking when trying to display a Notification
                                            //This is very rare but can still happen
                                            display_error_message(err, self.toasts.clone());
                                        };
                                    }
                                });

                            let image_bounds = Rect::from_two_pos(
                                Pos2::new(
                                    rectangle_rect.min.x - image.width() as f32
                                        + self.register.image.image_size
                                        - (self.register.image.image_size - 100.) * 0.5,
                                    rectangle_rect.min.y - image.height() as f32
                                        + (self.register.image.image_size - 100.) * 0.5,
                                ),
                                Pos2::new(
                                    rectangle_rect.max.x + image.width() as f32
                                        - self.register.image.image_size
                                        + (self.register.image.image_size - 100.) * 0.5,
                                    rectangle_rect.max.y + image.height() as f32
                                        - (self.register.image.image_size - 100.) * 0.5,
                                ),
                            );

                            //Put picture into an Area, so it can be moved
                            //This might be a bit buggy especially with huge images, but it gets the job done
                            Area::new(Id::new("REGISTER_IMAGE_SELECTOR"))
                                .order(egui::Order::Background)
                                .constrain_to(image_bounds)
                                .show(ctx, |ui| {
                                    let allocated_img = ui.allocate_ui(
                                        vec2(image.width() as f32, image.height() as f32),
                                        |ui| {
                                            if let Ok(read_bytes) =
                                                fs::read(self.register.image.image_path.clone())
                                            {
                                                ui.add(Image::from_bytes(
                                                    "bytes://register_image",
                                                    read_bytes,
                                                ));
                                            }
                                        },
                                    );
                                    self.register.image.image_rect = allocated_img.response.rect;
                                });
                        }
                        // else if ui.button("Upload picture").clicked() {
                        //     if let Some(app_data_path) = app_data_path {
                        //         match read_image(&app_data_path) {
                        //             Ok(image) => {
                        //                 //This shouldnt panic as we limit the types of file which can be seletected as a pfp
                        //                 self.register.image.selected_image_bytes = Some(image);
                        //             },
                        //             Err(err) => {
                        //                 //Avoid panicking when trying to display a Notification
                        //                 //This is very rare but can still happen
                        //                 display_error_message(err, self.toasts.clone());
                        //             },
                        //         }
                        //         self.register.image.image_path = app_data_path;
                        //         ctx.forget_image("bytes://register_image");
                        //     }
                        // }

                        if !(self.register.normal_profile_picture.is_empty()
                            && self.register.small_profile_picture.is_empty())
                        {
                            //Display profile picure preview
                            ui.horizontal_centered(|ui| {
                                ui.vertical(|ui| {
                                    ui.allocate_ui(vec2(256., 256.), |ui| {
                                        ui.add(Image::from_bytes(
                                            "bytes://profile_picture_preview_normal",
                                            self.register.normal_profile_picture.clone(),
                                        ));
                                    });
                                    ui.label(RichText::from("256px").weak());
                                });
                                ui.vertical(|ui| {
                                    ui.allocate_ui(vec2(64., 64.), |ui| {
                                        ui.add(Image::from_bytes(
                                            "bytes://profile_picture_preview_small",
                                            self.register.small_profile_picture.clone(),
                                        ));
                                    });
                                    ui.label(RichText::from("64px").weak());
                                });
                            });
                        }
                    });
                })
            });
        }
    }

    fn save_image(
        &mut self,
        image: DynamicImage,
        app_data_path: String,
        ctx: &egui::Context,
    ) -> anyhow::Result<()>
    {
        image
            .resize(256, 256, image::imageops::FilterType::CatmullRom)
            .save(format!(
                "{}\\matthias\\{}_temp_pfp256.png",
                app_data_path, self.register.username
            ))?;

        image
            .resize(64, 64, image::imageops::FilterType::CatmullRom)
            .save(format!(
                "{}\\matthias\\{}_temp_pfp64.png",
                app_data_path, self.register.username
            ))?;

        //Reset image entries to default
        self.register.image = ProfileImage::default();

        //Load both images to memory
        match (
            fs::read(format!(
                "{}\\matthias\\{}_temp_pfp256.png",
                app_data_path, self.register.username
            )),
            fs::read(format!(
                "{}\\matthias\\{}_temp_pfp64.png",
                app_data_path, self.register.username
            )),
        ) {
            (Ok(bytes256), Ok(bytes64)) => {
                //Clear image cache so we will display the latest image
                ctx.forget_image("bytes://profile_picture_preview_small");
                ctx.forget_image("bytes://profile_picture_preview_normal");

                //We will load both files into memory and delete them after it
                self.register.normal_profile_picture = bytes256;
                self.register.small_profile_picture = bytes64;

                fs::remove_file(format!(
                    "{}\\matthias\\{}_temp_pfp256.png",
                    app_data_path, self.register.username
                ))?;

                fs::remove_file(format!(
                    "{}\\matthias\\{}_temp_pfp64.png",
                    app_data_path, self.register.username
                ))?;
            },
            (Ok(_), Err(err)) => {
                bail!(
                    "Successfully read 256 file, but failed to read 64 file: {:?}",
                    err
                );
            },
            (Err(err), Ok(_)) => {
                bail!(
                    "Successfully read 64 file, but failed to read 256 file: {:?}",
                    err
                );
            },
            (Err(err256), Err(err64)) => {
                bail!(
                    "Failed to read both files:\n256: {:?}\n64: {:?}",
                    err256,
                    err64
                );
            },
        }

        Ok(())
    }
}

fn read_image(app_data_path: &PathBuf) -> anyhow::Result<DynamicImage>
{
    let image_reader = ImageReader::new(Cursor::new(fs::read(app_data_path)?))
        .with_guessed_format()?
        .decode()?;

    Ok(image_reader)
}

fn draw_rect(ui: &mut egui::Ui, stroke: Stroke, center_pos: Pos2, size_of_side: f32)
{
    let a_point = Pos2::new(
        center_pos.x - (size_of_side / 2.),
        center_pos.y - (size_of_side / 2.),
    );
    let b_point = Pos2::new(
        center_pos.x + (size_of_side / 2.),
        center_pos.y - (size_of_side / 2.),
    );
    let c_point = Pos2::new(
        center_pos.x + (size_of_side / 2.),
        center_pos.y + (size_of_side / 2.),
    );
    let d_point = Pos2::new(
        center_pos.x - (size_of_side / 2.),
        center_pos.y + (size_of_side / 2.),
    );

    //Sides
    ui.painter()
        .clone()
        .with_layer_id(LayerId::new(egui::Order::Foreground, "circle0".into()))
        .line_segment([a_point, b_point], stroke);

    ui.painter()
        .clone()
        .with_layer_id(LayerId::new(egui::Order::Foreground, "circle1".into()))
        .line_segment([b_point, c_point], stroke);

    ui.painter()
        .clone()
        .with_layer_id(LayerId::new(egui::Order::Foreground, "circle2".into()))
        .line_segment([c_point, d_point], stroke);

    ui.painter()
        .clone()
        .with_layer_id(LayerId::new(egui::Order::Foreground, "circle3".into()))
        .line_segment([d_point, a_point], stroke);
}
