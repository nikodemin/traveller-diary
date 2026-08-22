use crate::converter::{Converter, Mapper, TravelDefaults};
use crate::dao::{Dao, DaoOps};
use crate::model::Travel;
use log::error;
use refinery::{Migration, Runner};
use rusqlite::Connection;
use rust_embed::Embed;
use slint::{
    CloseRequestResponse, Image, ModelRc, SharedString, VecModel, quit_event_loop,
    select_bundled_translation,
};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod converter;
mod dao;
mod model;

slint::include_modules!();

#[derive(Embed)]
#[folder = "migrations/"]
struct Migrations;

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

fn main() -> Result<(), Box<dyn Error>> {
    let main_window = MainWindow::new()?;

    let languages = vec![("English", "en"), ("Русский", "ru")];
    main_window.set_languages(ModelRc::from(Rc::new(VecModel::<(
        SharedString,
        SharedString,
    )>::from(
        languages
            .iter()
            .map(|(s, s2)| ((*s).into(), (*s2).into()))
            .collect::<Vec<(SharedString, SharedString)>>(),
    ))));

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
    let dao = Dao::new(&mut conn);

    let travel_cover_default: Vec<u8> = Assets::get("island.png").unwrap().data.into();
    let travel_defaults = TravelDefaults {
        cover: travel_cover_default.convert()?,
    };

    main_window.set_state(State::TravelsView);
    match dao.list_travels(20, 0) {
        Ok(travels) => {
            let items: VecModel<TravelItem> = travels
                .iter()
                .map(|tr| travel_defaults.convert(tr))
                .collect();
            main_window
                .global::<TravelViewProps>()
                .set_travels(ModelRc::new(VecModel::from(items)));
        }
        Err(err) => error!("Error obtaining list of travels: {:?}", err),
    }

    let exit = Arc::new(Mutex::new(Some(|| {
        println!("Exiting main window");
        conn.close().unwrap();
        quit_event_loop().unwrap()
    })));
    let exit2 = exit.clone();

    main_window.on_setLanguage(|lang| {
        select_bundled_translation(lang.as_str()).unwrap();
    });
    main_window.on_exit(move || {
        if let Ok(mut guard) = exit.lock() {
            if let Some(f) = guard.take() {
                f();
            }
        }
    });
    main_window.window().on_close_requested(move || {
        if let Ok(mut guard) = exit2.lock() {
            if let Some(f) = guard.take() {
                f();
            }
        }
        CloseRequestResponse::HideWindow
    });

    main_window.run().map_err(Box::new)?;
    Ok(())
}
