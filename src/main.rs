mod dao;
mod model;

use anyhow::anyhow;
use config::Config;
use egui::menu::menu_button;
use std::sync::mpsc::{Receiver, Sender, channel};

use model::{AppState, Event};

struct DiaryApp {
    state: AppState,
    background_event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
    localization_conf: Config,
    settings_conf: Config,
}

impl DiaryApp {
    fn new(
        background_event_sender: Sender<Event>,
        event_receiver: Receiver<Event>,
        localization_conf: Config,
        settings_conf: Config,
    ) -> Result<Box<Self>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let default_language = settings_conf.get_string("default_language")?;

        Ok(Box::new(DiaryApp {
            state: AppState {
                language: default_language,
            },
            background_event_sender,
            event_receiver,
            localization_conf,
            settings_conf,
        }))
    }

    fn handle_gui_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                Event::ChangeLanguage => todo!(),
            }
        }
    }
}

impl eframe::App for DiaryApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_gui_events();

        let localization_conf = self
            .localization_conf
            .get_table(&self.state.language)
            .unwrap();

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let file_str = localization_conf
                    .get("file_menu")
                    .unwrap()
                    .clone()
                    .into_string()
                    .unwrap();
                let new_travel_str = localization_conf
                    .get("new_travel_btn")
                    .unwrap()
                    .clone()
                    .into_string()
                    .unwrap();

                let file_menu = ui.menu_button(file_str, |ui| {
                    if ui.button(new_travel_str).clicked() {
                        println!("new travel")
                    }
                });

                let settings_str = localization_conf
                    .get("settings_menu")
                    .unwrap()
                    .clone()
                    .into_string()
                    .unwrap();
                let change_language_str = localization_conf
                    .get("change_language_menu")
                    .unwrap()
                    .clone()
                    .into_string()
                    .unwrap();
                let languages = self
                    .settings_conf
                    .get_array("languages")
                    .unwrap()
                    .iter()
                    .flat_map(|x| x.clone().into_string())
                    .collect::<Vec<_>>();

                let settings_menu = ui.menu_button(settings_str, |ui| {
                    ui.menu_button(change_language_str, |ui| {
                        languages.iter().for_each(|l| {
                            if ui.button(l).clicked() {
                                self.state = AppState {
                                    language: l.to_owned(),
                                    ..self.state
                                }
                            }
                        })
                    })
                });

                file_menu;
                settings_menu;
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
            .default_width(travels_panel_default_width)
            .resizable(true)
            .min_width(travels_panel_min_width)
            .max_width(travels_panel_max_width)
            .show(ctx, |ui| {});

        egui::CentralPanel::default().show(ctx, |ui| {});
    }
}

fn main() -> Result<(), anyhow::Error> {
    let (background_event_sender, background_event_receiver) = channel::<Event>();
    let (event_sender, event_receiver) = channel::<Event>();

    let localization_conf = Config::builder()
        .add_source(config::File::with_name("localization.toml"))
        .build()
        .unwrap();
    let settings_conf = Config::builder()
        .add_source(config::File::with_name("settings.toml"))
        .build()
        .unwrap();

    std::thread::spawn(move || {
        while let Ok(event) = background_event_receiver.recv() {
            let sender = event_sender.clone();
            handle_events(event, sender);
        }
    });

    let window_init_size_x = settings_conf.get_float("window_init_size_x").unwrap() as f32;
    let window_init_size_y = settings_conf.get_float("window_init_size_y").unwrap() as f32;

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
            DiaryApp::new(
                background_event_sender,
                event_receiver,
                localization_conf,
                settings_conf,
            )
            .map(|app| app as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow!("eframe error: {}", e))
}

fn handle_events(event: Event, sender: Sender<Event>) {}
