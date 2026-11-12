use gio::prelude::*;
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const API_URL: &str =
    "https://api.github.com/repos/eulucastiagolt/luti-color-picker/releases/latest";
const NOTIFICATION_ID: &str = "update-available";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub fn start(app: &gtk::Application) {
    let open_release = gio::SimpleAction::new("open-release", Some(glib::VariantTy::STRING));
    open_release.connect_activate(|_, parameter| {
        let Some(version) = parameter
            .and_then(glib::Variant::str)
            .and_then(|value| Version::parse(value).ok())
        else {
            return;
        };
        let url = format!("{}/releases/tag/v{version}", env!("CARGO_PKG_REPOSITORY"));
        if let Err(error) = gio::AppInfo::launch_default_for_uri(&url, gio::AppLaunchContext::NONE)
        {
            eprintln!("Could not open the release page: {error}");
        }
    });
    app.add_action(&open_release);

    let app = app.downgrade();
    glib::spawn_future_local(async move {
        let mut notified_version = None;
        loop {
            let result = gio::spawn_blocking(latest_version).await;
            let Some(app) = app.upgrade() else {
                break;
            };
            if let Ok(Some(version)) = result {
                if notified_version.as_ref() != Some(&version) {
                    notify(&app, &version);
                    notified_version = Some(version);
                }
            }
            drop(app);
            glib::timeout_future(CHECK_INTERVAL).await;
        }
    });
}

fn latest_version() -> Option<Version> {
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("luti-color-picker/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?
        .get(API_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .ok()?
        .error_for_status()
        .ok()?;
    let release: Release = serde_json::from_reader(response).ok()?;
    newer_stable_version(&release.tag_name, env!("CARGO_PKG_VERSION"))
}

fn newer_stable_version(tag: &str, current: &str) -> Option<Version> {
    let latest = Version::parse(tag.strip_prefix('v')?).ok()?;
    let current = Version::parse(current).ok()?;
    (latest.pre.is_empty() && latest.cmp_precedence(&current).is_gt()).then_some(latest)
}

fn notify(app: &gtk::Application, version: &Version) {
    let notification = gio::Notification::new("Luti Color Picker update available");
    notification.set_body(Some(&format!("Version {version} is available on GitHub.")));
    notification.set_icon(&gio::ThemedIcon::new(crate::APP_ID));
    let target = version.to_string().to_variant();
    notification.set_default_action_and_target_value("app.open-release", Some(&target));
    notification.add_button_with_target_value("View release", "app.open-release", Some(&target));
    app.send_notification(Some(NOTIFICATION_ID), &notification);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_newer_stable_versions_are_reported() {
        assert_eq!(
            newer_stable_version("v1.10.0", "1.9.0"),
            Some(Version::new(1, 10, 0))
        );
        for tag in ["v1.9.0", "v1.9.0+build", "v1.8.0", "v2.0.0-rc.1", "latest"] {
            assert_eq!(newer_stable_version(tag, "1.9.0"), None, "{tag}");
        }
    }
}
