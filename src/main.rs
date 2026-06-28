mod backend;
mod dao;
mod model;

use crate::backend::Backend;
use crate::dao::{Dao, DaoOps};
use crate::model::{Response, Travel};
use anyhow::anyhow;
use chrono::NaiveDateTime;
use config::Config;
use egui::ImageSource;
use egui::load::Bytes;
use egui_flex::{Flex, item};
use model::{AppState, Cmd};
use refinery::{Migration, Runner};
use rusqlite::Connection;
use rust_embed::Embed;
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
                let new_travel_str = get_loc_str("new_travel_btn");
                let settings_str = get_loc_str("settings_menu");
                let change_language_str = get_loc_str("change_language_menu");

                let languages = self
                    .settings_conf
                    .get_array("languages")
                    .unwrap()
                    .iter()
                    .flat_map(|x| x.clone().into_string())
                    .collect::<Vec<_>>();

                ui.menu_button(file_str, |ui| {
                    if ui.button(new_travel_str).clicked() {
                        self.event_sender
                            .send(Cmd::AddTravel {
                                travel: Travel {
                                    id: 0,
                                    country: "USA".into(),
                                    city: "Las Vegas".into(),
                                    began: NaiveDateTime::parse_from_str(
                                        "2026-03-14 12:00",
                                        "%Y-%m-%d %H:%M",
                                    )
                                    .unwrap(),
                                    ended: NaiveDateTime::parse_from_str(
                                        "2026-03-21 11:00",
                                        "%Y-%m-%d %H:%M",
                                    )
                                    .unwrap(),
                                    cover: None,
                                },
                            })
                            .unwrap();
                        println!("new travel")
                    }
                });

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
            Flex::horizontal().wrap(true).show(ui, |flex| {
                for travel in travels.iter() {
                    let bytes = Assets::get("plus-icon.png").unwrap().data.into_owned();
                    let image = egui::Image::new(ImageSource::Bytes {
                        uri: Default::default(),
                        bytes: Bytes::Shared(Arc::from(bytes)),
                    })
                    .max_width(100.0)
                    .max_height(100.0)
                    .sense(egui::Sense::click());

                    image.image_options().corner_radius.at_least(5);

                    let resp = flex.add(item(), image);

                    if resp.hovered() {
                        egui::Tooltip::for_widget(&resp)
                            .show(|ui| ui.label(travel.city.to_string()));
                    }
                }
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
