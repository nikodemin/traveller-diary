use crate::dao::Dao;
use rusqlite::Connection;
use slint::{
    CloseRequestResponse, ModelRc, SharedString, VecModel, quit_event_loop,
    select_bundled_translation,
};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod dao;
mod model;

slint::include_modules!();

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
    let dao = Dao::new(&mut conn);

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
