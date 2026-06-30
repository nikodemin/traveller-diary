mod backend;
mod dao;
mod model;

use crate::backend::Backend;
use crate::dao::{Dao, DaoOps};
use crate::model::Response;
use anyhow::anyhow;
use chrono::NaiveDateTime;
use config::Config;
use egui::Widget;
use egui::load::Bytes;
use egui::{ImageSource, Sense};
use egui_flex::{Flex, FlexAlignContent, FlexInstance, item};
use model::{AppState, Cmd};
use refinery::{Migration, Runner};
use rusqlite::Connection;
use rust_embed::Embed;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

struct DiaryApp {
    state: AppState,
    event_sender: Sender<Cmd>,
    event_receiver: Receiver<Response>,
    localization_conf: Config,
    settings_conf: Config,
}

impl DiaryApp {
    fn new(
        event_sender: Sender<Cmd>,
        event_receiver: Receiver<Response>,
        localization_conf: Config,
        settings_conf: Config,
    ) -> Result<Box<Self>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let default_language = settings_conf.get_string("default_language")?;

        let conn = Connection::open("traveller_diary.db")?;
        let dao = Dao::new(conn);
        let years = dao.list_travel_years()?;
        let (travels, first_year) = match years.first() {
            Some(y) => (
                HashMap::from_iter([(*y, dao.list_travels_by_year(*y)?)].into_iter()),
                Some(*y),
            ),
            None => (HashMap::new(), None),
        };

        Ok(Box::new(DiaryApp {
            state: AppState {
                language: default_language,
                years,
                travels,
                selected_travel_year: first_year,
            },
            event_sender,
            event_receiver,
            localization_conf,
            settings_conf,
        }))
    }

    fn handle_events(&mut self) {
        match self.event_receiver.try_recv() {
            Ok(Response::LoadTravelsByYear { year, travels }) => {
                self.state.travels.insert(year, travels);
            }
            Ok(Response::AddTravel { id }) => {}
            _ => (),
        }
    }
}

impl eframe::App for DiaryApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.handle_events();

        let localization_conf = self
            .localization_conf
            .get_table(&self.state.language)
            .unwrap();

        let get_loc_str = |name: &str| {
            localization_conf
                .get(name)
                .unwrap()
                .clone()
                .into_string()
                .unwrap()
        };

        egui::Panel::top("menu").show(ui, |ui| {
            egui::menu::MenuBar::new().ui(ui, |ui| {
                let file_str = get_loc_str("file_menu");
                let settings_str = get_loc_str("settings_menu");
                let change_language_str = get_loc_str("change_language_menu");

                let languages = self
                    .settings_conf
                    .get_array("languages")
                    .unwrap()
                    .iter()
                    .flat_map(|x| x.clone().into_string())
                    .collect::<Vec<_>>();

                ui.menu_button(file_str, |ui| {});

                ui.menu_button(settings_str, |ui| {
                    ui.menu_button(change_language_str, |ui| {
                        languages.iter().for_each(|l| {
                            if ui.button(l).clicked() {
                                self.state = AppState {
                                    language: l.to_owned(),
                                    ..self.state.clone()
                                }
                            }
                        })
                    })
                });
            });
        });

        let travels_panel_default_width = self
            .settings_conf
            .get_float("travels_panel_default_width")
            .unwrap() as f32;
        let travels_panel_min_width = self
            .settings_conf
            .get_float("travels_panel_min_width")
            .unwrap() as f32;
        let travels_panel_max_width = self
            .settings_conf
            .get_float("travels_panel_max_width")
            .unwrap() as f32;

        egui::Panel::bottom("travel_dates")
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 10,
                            right: 10,
                            top: 5,
                            bottom: 12,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                for year in self.state.years.iter() {
                                    if (ui.button(year.to_string()).clicked()) {
                                        if (!self.state.travels.contains_key(year)) {
                                            self.event_sender
                                                .send(Cmd::LoadTravelsByYear { year: *year })
                                                .unwrap();
                                        }
                                        self.state.selected_travel_year = Some(*year);
                                    };
                                }
                                ui.add_space(20.0);
                            });
                        });
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let empty_vec = Vec::new();
            let travels = match self.state.selected_travel_year {
                Some(y) => self.state.travels.get(&y).unwrap_or_else(|| &empty_vec),
                None => &empty_vec,
            };

            let make_travel =
                |flex: &mut FlexInstance,
                 b: Vec<u8>,
                 id: String,
                 img_id: String,
                 meta: Option<(String, String, NaiveDateTime, NaiveDateTime)>| {
                    let image = egui::Image::new(ImageSource::Bytes {
                        uri: Cow::Owned(img_id),
                        bytes: Bytes::Shared(Arc::from(b)),
                    })
                    .max_width(100.0)
                    .max_height(100.0)
                    .sense(egui::Sense::click());

                    image.image_options().corner_radius.at_least(5);

                    let add_label =
                        |flex: &mut FlexInstance, str| flex.add(item(), egui::Label::new(str));

                    flex.add_flex(
                        item().sense(Sense::click()),
                        Flex::vertical().align_content(FlexAlignContent::Center),
                        |flex| match meta {
                            None => {
                                flex.add(item(), image);
                                add_label(flex, get_loc_str("new_travel"));
                            }
                            Some((country, city, began, ended)) => {
                                flex.add(item(), image);
                                add_label(flex, country);
                                add_label(flex, city);
                                add_label(flex, format!("{:?} - {:?}", began.date(), ended.date()));
                            }
                        },
                    )
                };

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Flex::horizontal().wrap(true).show(ui, |flex| {
                        let plus_bytes = Assets::get("plus-icon.png").unwrap().data.to_vec();

                        let resp = make_travel(
                            flex,
                            plus_bytes,
                            "plus".to_string(),
                            "plus".to_string(),
                            None,
                        );

                        if resp.response.clicked() {
                            //todo fix
                            println!("new travel");
                        }

                        for travel in travels.iter() {
                            let (bytes, uri) = travel
                                .cover
                                .clone()
                                .map(|p| (p.data, p.id.to_string()))
                                .unwrap_or_else(|| {
                                    (
                                        Assets::get("island.png").unwrap().data.to_vec(),
                                        "island".to_string(),
                                    )
                                });

                            make_travel(
                                flex,
                                bytes,
                                travel.id.to_string(),
                                uri,
                                Some((
                                    travel.country.clone(),
                                    travel.city.clone(),
                                    travel.began,
                                    travel.ended,
                                )),
                            );
                        }
                    });
                });
        });
    }
}

#[derive(Embed)]
#[folder = "migrations/"]
struct Migrations;

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

fn main() -> Result<(), anyhow::Error> {
    let (cmd_sender, cmd_receiver) = channel::<Cmd>();
    let (rsp_sender, rsp_receiver) = channel::<Response>();

    let localization_conf = Config::builder()
        .add_source(config::File::with_name("localization.toml"))
        .build()?;
    let settings_conf = Config::builder()
        .add_source(config::File::with_name("settings.toml"))
        .build()?;

    let mut conn = Connection::open("traveller_diary.db")?;
    let migrations: Vec<Migration> = Migrations::iter()
        .flat_map(|path| Migrations::get(path.as_ref()).map(|f| (path, f)))
        .map(|(p, f)| {
            Migration::unapplied(
                p.as_ref(),
                String::from_utf8(f.data.into()).unwrap().as_str(),
            )
            .unwrap()
        })
        .collect();
    let runner = Runner::new(&migrations);
    runner.run(&mut conn)?;
    let dao = Dao::new(conn);

    std::thread::spawn(move || {
        let backend = Backend::new(dao, rsp_sender, cmd_receiver);
        backend.serve()
    });

    let window_init_size_x = settings_conf.get_float("window_init_size_x")? as f32;
    let window_init_size_y = settings_conf.get_float("window_init_size_y")? as f32;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Traveller diary")
            .with_inner_size([window_init_size_x, window_init_size_y]),
        ..Default::default()
    };
    eframe::run_native(
        "Traveller diary",
        options,
        Box::new(|context| {
            egui_extras::install_image_loaders(&context.egui_ctx);
            DiaryApp::new(cmd_sender, rsp_receiver, localization_conf, settings_conf)
                .map(|app| app as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow!("eframe error: {}", e))
}
