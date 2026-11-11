use glib::Sender;
use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};

#[derive(Debug, Clone)]
pub enum TrayCommand {
    PickColor(Option<String>),
    ShowWindow(Option<String>),
    ShowAbout,
    Quit,
}

fn get_activation_token() -> Option<String> {
    if let r @ Some(_) = ksni::current_xdg_activation_token() {
        return r;
    }
    // KDE Plasma may deliver the activation token slightly after the tray event
    std::thread::sleep(std::time::Duration::from_millis(2));
    if let r @ Some(_) = ksni::current_xdg_activation_token() {
        return r;
    }
    std::thread::sleep(std::time::Duration::from_millis(5));
    ksni::current_xdg_activation_token()
}

pub struct PickTray {
    tx: Sender<TrayCommand>,
}

impl PickTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        Self { tx }
    }
}

impl ksni::Tray for PickTray {
    fn id(&self) -> String {
        crate::APP_ID.into()
    }

    fn icon_name(&self) -> String {
        crate::APP_ID.into()
    }

    fn title(&self) -> String {
        crate::APP_NAME.into()
    }

    // Left click on tray icon: open magnifier directly
    fn activate(&mut self, _x: i32, _y: i32) {
        let token = get_activation_token();
        let _ = self.tx.send(TrayCommand::PickColor(token));
    }

    // Secondary activate (middle click): show window
    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let token = get_activation_token();
        let _ = self.tx.send(TrayCommand::ShowWindow(token));
    }

    // Right click on tray icon: context menu
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx_pick = self.tx.clone();
        let tx_show = self.tx.clone();
        let tx_about = self.tx.clone();
        let tx_quit = self.tx.clone();

        vec![
            StandardItem {
                label: "Capturar cor".into(),
                icon_name: "color-picker".into(),
                activate: Box::new(move |_| {
                    let token = get_activation_token();
                    let _ = tx_pick.send(TrayCommand::PickColor(token));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Mostrar janela".into(),
                icon_name: "window-new".into(),
                activate: Box::new(move |_| {
                    let token = get_activation_token();
                    let _ = tx_show.send(TrayCommand::ShowWindow(token));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Sobre".into(),
                icon_name: "help-about".into(),
                activate: Box::new(move |_| {
                    let _ = tx_about.send(TrayCommand::ShowAbout);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Sair".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn_tray(tx: Sender<TrayCommand>) -> Option<ksni::blocking::Handle<PickTray>> {
    let tray = PickTray::new(tx);
    match tray.spawn() {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!("Aviso: Não foi possível registrar o ícone na bandeja do sistema: {e}");
            None
        }
    }
}
