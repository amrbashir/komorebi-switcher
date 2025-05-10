#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fmt::Display;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::app::{App, AppMessage};

mod app;
mod egui_glue;
mod komorebi;
mod multi_map;
mod taskbar;
mod tray_icon;
mod utils;
mod widgets;
mod window_registry_info;
mod windows;

fn error_dialog<T: Display>(error: T) {
    rfd::MessageDialog::new()
        .set_title("komorebi-switcher")
        .set_description(error.to_string())
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn run() -> anyhow::Result<()> {
    let evl = EventLoop::<AppMessage>::with_user_event().build()?;

    let proxy = evl.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |e| {
        if let Err(e) = proxy.send_event(AppMessage::MenuEvent(e)) {
            tracing::error!("Failed to send `AppMessage::MenuEvent`: {e}")
        }
    }));

    let mut app = App::new(evl.create_proxy())?;
    evl.run_app(&mut app)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_env("KOMOREBI_SWITCHER_LOG").unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy()
    });

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .with_target(false)
        .with_env_filter(env_filter)
        .finish();

    #[cfg(not(debug_assertions))]
    let (file_log_layer, _f_guard) = {
        use std::time::{Duration, SystemTime};

        use anyhow::Context;

        let logs_dir = dirs::data_dir()
            .context("Failed to get $data_dir path")?
            .join("komorebi-switcher")
            .join("logs");

        const MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30);
        let now = SystemTime::now();

        // remove logs older than a month
        for entry in std::fs::read_dir(&logs_dir).ok().into_iter().flatten() {
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(modified_time) = entry.metadata().and_then(|m| m.modified()) else {
                continue;
            };

            let elapsed = now.duration_since(modified_time).unwrap_or_default();
            if elapsed > MONTH {
                let _ = std::fs::remove_file(entry.path());
            }
        }

        let appender = tracing_appender::rolling::daily(&logs_dir, "komorebi-switcher.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(appender);
        let layer = tracing_subscriber::fmt::Layer::default()
            // disable ansi coloring in log file
            .with_ansi(false)
            .with_writer(non_blocking);

        (layer, _guard)
    };

    #[cfg(not(debug_assertions))]
    use tracing_subscriber::layer::SubscriberExt;
    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(file_log_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    tracing::debug!("Initialized Logger");

    std::panic::set_hook(Box::new(|info| {
        error_dialog(info);
        tracing::error!("{info}");
    }));

    if let Err(e) = run() {
        error_dialog(&e);
        tracing::error!("{e}");
        std::process::exit(1);
    }

    Ok(())
}
