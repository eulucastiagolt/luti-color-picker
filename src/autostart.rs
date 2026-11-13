use std::io;
use std::path::PathBuf;

pub fn supported() -> bool {
    !std::path::Path::new("/.flatpak-info").exists() && std::env::var_os("SNAP").is_none()
}

fn entry_path() -> PathBuf {
    glib::user_config_dir()
        .join("autostart")
        .join(format!("{}.desktop", crate::APP_ID))
}

pub fn enabled() -> bool {
    let entry = glib::KeyFile::new();
    entry
        .load_from_file(entry_path(), glib::KeyFileFlags::NONE)
        .is_ok()
        && !entry.boolean("Desktop Entry", "Hidden").unwrap_or(false)
}

pub fn enable_by_default() -> io::Result<()> {
    if supported() && !entry_path().try_exists()? {
        set_enabled(true)?;
    }
    Ok(())
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    let path = entry_path();

    let executable = launcher()?;
    let executable = executable
        .to_str()
        .filter(|value| !value.chars().any(char::is_control))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Unsupported launcher path"))?;
    let command = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");

    let entry = glib::KeyFile::new();
    entry.set_string("Desktop Entry", "Type", "Application");
    entry.set_string("Desktop Entry", "Name", crate::APP_NAME);
    entry.set_string("Desktop Entry", "Exec", &format!("env -- \"{command}\""));
    entry.set_string("Desktop Entry", "Icon", crate::APP_ID);
    entry.set_boolean("Desktop Entry", "Terminal", false);
    entry.set_boolean("Desktop Entry", "Hidden", !enabled);

    std::fs::create_dir_all(glib::user_config_dir().join("autostart"))?;
    glib::file_set_contents(path, entry.to_data().as_bytes()).map_err(io::Error::other)
}

pub(crate) fn launcher() -> io::Result<PathBuf> {
    std::env::var_os("APPIMAGE")
        .or_else(|| std::env::var_os("LUTI_COLOR_PICKER_LAUNCHER"))
        .map(PathBuf::from)
        .map_or_else(std::env::current_exe, Ok)
}
