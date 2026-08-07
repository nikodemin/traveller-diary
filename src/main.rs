use slint::{ModelRc, SharedString, VecModel, quit_event_loop, select_bundled_translation};
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
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
    fn exit() {
        quit_event_loop().unwrap()
    }

    main_window.on_setLanguage(|lang| {
        select_bundled_translation(lang.as_str()).unwrap();
    });
    main_window.on_exit(|| exit());

    main_window.run()
}
