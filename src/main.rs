mod backend;
mod dao;
mod model;

use crate::backend::Backend;
use crate::dao::Dao;
use crate::model::{Response, Travel};
use anyhow::anyhow;
use chrono::NaiveDateTime;
use config::Config;
use model::{AppState, Cmd};
use refinery::{Migration, Runner};
use rusqlite::Connection;
use rust_embed::Embed;
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

        Ok(Box::new(DiaryApp {
            state: AppState {
                language: default_language,
                travels: Vec::new(),
            },
            event_sender,
            event_receiver,
            localization_conf,
            settings_conf,
        }))
    }

    fn handle_events(&mut self) {
        match self.event_receiver.try_recv() {
            Ok(Response::LoadTravels { .. }) => {}
            Ok(Response::AddTravel { id }) => self.state.travels.push(id.to_string()),
            _ => (),
        }
    }
}

impl eframe::App for DiaryApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
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

        egui::SidePanel::left("travels_panel")
            .resizable(true)
            .default_width(travels_panel_default_width)
            .width_range(travels_panel_min_width..=travels_panel_max_width)
            .show(ctx, |ui| {
                ui.label(self.state.travels.iter().fold(String::new(), |x, y| x + y));
                egui::ScrollArea::vertical().show(ui, |ui| {});
            });

        egui::CentralPanel::default().show(ctx, |ui| {});
    }
}

#[derive(Embed)]
#[folder = "migrations/"]
struct Migrations;

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
