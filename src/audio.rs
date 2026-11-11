use std::process::Command;

/// Plays system feedback sounds (e.g., "camera-shutter", "dialog-information")
/// Matches the exact fire-and-forget subprocess call used in Pick
pub fn play_sound(sound_id: &str) {
    let _ = Command::new("canberra-gtk-play")
        .arg("-i")
        .arg(sound_id)
        .spawn();
}
