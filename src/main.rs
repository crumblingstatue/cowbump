#![feature(try_blocks, let_chains, map_many_mut)]
#![windows_subsystem = "windows"]
#![warn(
    unused_qualifications,
    single_use_lifetimes,
    redundant_imports,
    trivial_casts,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::semicolon_if_nothing_returned,
    clippy::doc_markdown,
    clippy::missing_panics_doc,
    clippy::explicit_iter_loop,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_lossless,
    clippy::cloned_instead_of_copied,
    clippy::map_unwrap_or,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    //clippy::indexing_slicing <- TODO: Enable when I'm feeling more motivated
)]

mod application;
mod collection;
mod db;
mod entry;
mod entry_utils;
mod filter_reqs;
mod folder_scan;
mod gui;
mod preferences;
mod sequence;
mod serialization;
mod tag;

const VERSION: &str = env!("CARGO_PKG_VERSION");

use {
    crate::application::Application,
    egui_sfml::{
        egui,
        sfml::{
            graphics::RenderWindow,
            window::{Event, Style},
        },
        SfEgui,
    },
    std::io::IsTerminal,
};

fn try_main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s
        } else {
            "Unknown panic payload"
        };
        let (file, line, column) = match panic_info.location() {
            Some(loc) => (loc.file(), loc.line().to_string(), loc.column().to_string()),
            None => ("unknown", "unknown".into(), "unknown".into()),
        };
        let btrace = std::backtrace::Backtrace::force_capture();
        eprintln!("{btrace}");
        fatal_error_report(
            "Cowbump panic",
            &format!(
                "\
            {msg}\n\n\
            Location:\n\
            {file}:{line}:{column}\n\n\
            Backtrace:\n\
            {btrace}"
            ),
        );
    }));
    let mut app = Application::new()?;
    gui::run(&mut app)
}

fn main() {
    if let Err(e) = try_main() {
        fatal_error_report("Fatal runtime error", &format!("{e:?}"));
    }
}

/// Report fatal error or panic
///
/// # Panics
///
/// If the egui pass fails, and maybe some other catastrophic stuff
fn fatal_error_report(title: &str, mut msg: &str) {
    if std::io::stderr().is_terminal() {
        eprintln!("== {title} ==\n");
        eprintln!("{msg}");
        return;
    }
    let mut rw = RenderWindow::new((800, 600), title, Style::CLOSE, &Default::default());
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);
    while rw.is_open() {
        while let Some(ev) = rw.poll_event() {
            sf_egui.add_event(&ev);
            sf_egui.begin_pass();
            egui::CentralPanel::default().show(sf_egui.context(), |ui| {
                ui.heading("Cowbump panicked");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add_sized(
                            ui.available_size(),
                            egui::TextEdit::multiline(&mut msg).code_editor(),
                        );
                    });
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
