use eframe::{egui, epi};

use std::collections::HashMap;
use remoteio_core::audio::devices::devicecoordinator::{DeviceCoordinator, DeviceCoordinatorState};
use remoteio_core::audio::devices::device::{Device};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // // Example stuff:
    label: String,
    coordinator: DeviceCoordinator,
    target: Vec<(Device, Option<Device>)>
    // // this how you opt-out of serialization of a member
    // #[cfg_attr(feature = "persistence", serde(skip))]
    // value: f32,
}

impl <'switchboard> Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            coordinator: DeviceCoordinator::default(),
            target: Vec::new(),
            // value: 2.7,
        }
    }
}

impl <'switchboard> epi::App for TemplateApp {
        fn name(&self) -> &str {
            "Switch Board"
        }
    
        /// Called once before the first frame.
        fn setup(
            &mut self,
            _ctx: &egui::CtxRef,
            _frame: &epi::Frame,
            _storage: Option<&dyn epi::Storage>,
        ) {
            // Load previous app state (if any).
            // Note that you must enable the `persistence` feature for this to work.
            #[cfg(feature = "persistence")]
            if let Some(storage) = _storage {
                *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
            }
        }
    
        /// Called by the frame work to save state before shutdown.
        /// Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        fn save(&mut self, storage: &mut dyn epi::Storage) {
            epi::set_value(storage, epi::APP_KEY, self);
        }
    
        /// Called each time the UI needs repainting, which may be many times per second.
        /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
            // let Self { label, coordinator, mut target } = self;
            let label = &mut self.label;
            let coordinator = &mut self.coordinator;
            let target = &mut self.target;

            if target.is_empty() {
                println!("target empty!");
                coordinator.devices().unwrap().for_each(|device| {
                    target.push((Device::from(device), None));
                });
            } else {
                // println!("target not empty!");
                let mut target_map = HashMap::<String, String>::new();

                target.iter().for_each(|(from, to_opt)| {
                    match to_opt {
                        //@TODO disgusting ngl
                        Some(to) => {
                            let name_or_panic = |device: &Device| device.name().unwrap_or_else(|_|panic!("could not get name!")).to_owned();

                            target_map.insert(name_or_panic(from), name_or_panic(to));
                        },
                        None => {}
                    };
                });

                coordinator.correct_state(target_map).unwrap();
            }
            
            // Examples of how to create different panels and windows.
            // Pick whichever suits you.
            // Tip: a good default choice is to just keep the `CentralPanel`.
            // For inspiration and more examples, go to https://emilk.github.io/egui
    
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                // The top panel is often a good place for a menu bar:
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            frame.quit();
                        }
                        if ui.button("OogaBooga!").clicked() {
                            println!("ooga booga!");
                        }
                    });
                });
            });
    
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                ui.heading("This is a side panell!!");
                
                egui::ComboBox::from_label("Combo Box Power!")
                    .selected_text(label.to_owned())
                    .show_ui(ui,|ui| {
                        ui.selectable_value(label, "one".to_owned(), "one");
                        ui.selectable_value(label, "tow".to_owned(), "tow");
                        ui.selectable_value(label, "trhtee".to_owned(), "trhtee");
                    });

                target.iter_mut().for_each(|(from, to_opt)| {
                    let from_label = from.name().unwrap_or_else(|_|"ERR".to_owned());
                    let selected = match to_opt {
                        Some(to) => to.name().unwrap_or_else(|_|"ERR".to_owned()),
                        None => {
                            let mut label = from_label.clone();
                            label.push_str("_loop");
                            label
                        }
                    };
                    egui::ComboBox::from_label(from_label.clone())
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            coordinator.devices().unwrap().for_each(|select_device| {
                                let select_name = select_device.name().unwrap_or_else(|_|"ERR".to_owned());
                                ui.selectable_value(to_opt, Some(Device::from(select_device)), select_name.clone());
                            });
                            ui.selectable_value(to_opt, None, from_label);
                        });
                });
                

                // ui.horizontal(|ui| {
                //     ui.label("WRITE!");
                //     ui.text_edit_singleline(label);
                // });
    
                // ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
                // if ui.button("Increment").clicked() {
                //     *value += 1.0;
                // }
    
                // ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                //     ui.horizontal(|ui| {
                //         ui.spacing_mut().item_spacing.x = 0.0;
                //         ui.label("powered by ");
                //         ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                //         ui.label(" and ");
                //         ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                //     });
                // });
            });
    
            egui::CentralPanel::default().show(ctx, |ui| {
                // The central panel the region left after adding TopPanel's and SidePanel's
    
                ui.heading("eframe template");
                ui.hyperlink("https://github.com/emilk/eframe_template");
                ui.add(egui::github_link_file!(
                    "https://github.com/emilk/eframe_template/blob/master/",
                    "Source code."
                ));
                egui::warn_if_debug_build(ui);
            });
    
            if false {
                egui::Window::new("Window").show(ctx, |ui| {
                    ui.label("Windows can be moved by dragging them.");
                    ui.label("They are automatically sized based on contents.");
                    ui.label("You can turn on resizing and scrolling if you like.");
                    ui.label("You would normally chose either panels OR windows.");
                });
            }
        }
    }
    

// impl epi::App for TemplateApp {
//     fn name(&self) -> &str {
//         "eframe template"
//     }

//     /// Called once before the first frame.
//     fn setup(
//         &mut self,
//         _ctx: &egui::CtxRef,
//         _frame: &epi::Frame,
//         _storage: Option<&dyn epi::Storage>,
//     ) {
//         // Load previous app state (if any).
//         // Note that you must enable the `persistence` feature for this to work.
//         #[cfg(feature = "persistence")]
//         if let Some(storage) = _storage {
//             *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
//         }
//     }

//     /// Called by the frame work to save state before shutdown.
//     /// Note that you must enable the `persistence` feature for this to work.
//     #[cfg(feature = "persistence")]
//     fn save(&mut self, storage: &mut dyn epi::Storage) {
//         epi::set_value(storage, epi::APP_KEY, self);
//     }

//     /// Called each time the UI needs repainting, which may be many times per second.
//     /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
//     fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
//         let Self { label, value } = self;

//         // Examples of how to create different panels and windows.
//         // Pick whichever suits you.
//         // Tip: a good default choice is to just keep the `CentralPanel`.
//         // For inspiration and more examples, go to https://emilk.github.io/egui

//         egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
//             // The top panel is often a good place for a menu bar:
//             egui::menu::bar(ui, |ui| {
//                 ui.menu_button("File", |ui| {
//                     if ui.button("Quit").clicked() {
//                         frame.quit();
//                     }
//                     if ui.button("OogaBooga!").clicked() {
//                         println!("ooga booga!");
//                     }
//                 });
//             });
//         });

//         egui::SidePanel::left("side_panel").show(ctx, |ui| {
//             ui.heading("This is a side panell!!");

//             ui.horizontal(|ui| {
//                 ui.label("WRITE!");
//                 ui.text_edit_singleline(label);
//             });

//             ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
//             if ui.button("Increment").clicked() {
//                 *value += 1.0;
//             }

//             ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
//                 ui.horizontal(|ui| {
//                     ui.spacing_mut().item_spacing.x = 0.0;
//                     ui.label("powered by ");
//                     ui.hyperlink_to("egui", "https://github.com/emilk/egui");
//                     ui.label(" and ");
//                     ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
//                 });
//             });
//         });

//         egui::CentralPanel::default().show(ctx, |ui| {
//             // The central panel the region left after adding TopPanel's and SidePanel's

//             ui.heading("eframe template");
//             ui.hyperlink("https://github.com/emilk/eframe_template");
//             ui.add(egui::github_link_file!(
//                 "https://github.com/emilk/eframe_template/blob/master/",
//                 "Source code."
//             ));
//             egui::warn_if_debug_build(ui);
//         });

//         if false {
//             egui::Window::new("Window").show(ctx, |ui| {
//                 ui.label("Windows can be moved by dragging them.");
//                 ui.label("They are automatically sized based on contents.");
//                 ui.label("You can turn on resizing and scrolling if you like.");
//                 ui.label("You would normally chose either panels OR windows.");
//             });
//         }
//     }
// }
