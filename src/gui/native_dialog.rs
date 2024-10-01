use {
    egui_sfml::{
        egui,
        sfml::{
            graphics::RenderWindow,
            window::{ContextSettings, Event, Style},
        },
        SfEgui,
    },
    std::fmt::Debug,
};

/// Show a blocking error window
///
/// # Panics
///
/// If the egui pass fails, and maybe some other catastrophic stuff
pub fn error_blocking<E: Debug>(title: &str, err: E) {
    let mut rw = RenderWindow::new(
        (800, 600),
        title,
        Style::DEFAULT,
        &ContextSettings::default(),
    );
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);
    while rw.is_open() {
        while let Some(ev) = rw.poll_event() {
            sf_egui.add_event(&ev);
            sf_egui.begin_pass();
            egui::CentralPanel::default().show(sf_egui.context(), |ui| {
                ui.label(format!("{err:?}"));
            });
            sf_egui.end_pass(&mut rw).unwrap();
            if let Event::Closed = ev {
                rw.close();
            }
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}
