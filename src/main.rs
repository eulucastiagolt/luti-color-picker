pub mod audio;
mod autostart;
mod updates;
pub mod capture;
pub mod color;
pub mod storage;
pub mod ui;

use clap::Parser;
use gio::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use ui::MainWindow;

pub const APP_NAME: &str = "Luti Color Picker";
pub const APP_ID: &str = "io.github.eulucastiagolt.LutiColorPicker";

#[derive(Parser, Debug)]
#[command(name = "luti-color-picker", version, about)]
struct Args {
    #[arg(long, help = "Capture a color immediately")]
    pick: bool,

    #[arg(long, help = "Show the About dialog")]
    about: bool,

    #[arg(long, help = "Use humorous color names on startup")]
    snark: bool,

    #[arg(long, alias = "tray", help = "Keep running in the system tray")]
    background: bool,
}

fn main() {
    Args::parse();

    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK.");
        std::process::exit(1);
    }

    glib::set_application_name(APP_NAME);
    gtk::Window::set_default_icon_name(APP_ID);

    let app = gtk::Application::new(Some(APP_ID), gio::ApplicationFlags::HANDLES_COMMAND_LINE);
    app.connect_startup(updates::start);
    let current_window = Rc::new(RefCell::new(None::<MainWindow>));
    app.connect_command_line({
        let current_window = current_window.clone();
        move |app, command| {
            let Ok(args) = Args::try_parse_from(command.arguments()) else {
                return 2.into();
            };

            let window = {
                let mut current = current_window.borrow_mut();
                let mut start_minimized = false;
                let main_window = current.get_or_insert_with(|| {
                    if let Err(error) = autostart::enable_by_default() {
                        eprintln!("Could not enable startup at login: {error}");
                    }
                    let state = storage::load_state();
                    start_minimized = state.settings.start_minimized;
                    MainWindow::new(app, args.snark, args.background, state)
                });
                if args.pick {
                    main_window.capture();
                } else if start_minimized && !args.about {
                    main_window.window.minimize();
                    main_window.window.set_visible(true);
                } else {
                    main_window.window.unminimize();
                    main_window.window.present();
                }
                main_window.window.clone()
            };
            if args.about {
                let window = window.downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(window) = window.upgrade() {
                        window.present();
                        ui::about::show_about_dialog(Some(window.upcast_ref()));
                    }
                });
            }
            glib::ExitCode::SUCCESS
        }
    });

    let exit_code = app.run();
    current_window.borrow_mut().take();
    std::process::exit(exit_code.get().into());
}
