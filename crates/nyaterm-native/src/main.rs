pub mod ai_http;
mod assets;
mod cloud_sync_http;
mod translation_http;
mod ui;
mod update_http;

use anyhow::Context as _;
use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use nyaterm_domain::{AppRuntime, LOG_FILE_PREFIX, LOG_FILE_SUFFIX};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    let runtime = AppRuntime::resolve().context("resolve nyaterm runtime")?;
    runtime
        .ensure_directories()
        .context("prepare runtime directories")?;
    let _log_guard = init_tracing(&runtime);

    Application::new()
        .with_assets(assets::NyaTermAssets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
            let app_runtime = runtime.clone();

            cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |_, cx| cx.new(|cx| ui::NyaTermApp::new(app_runtime, cx)),
            )
            .expect("failed to open NyaTerm window");

            cx.activate(true);
        });

    Ok(())
}

fn init_tracing(runtime: &AppRuntime) -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("nyaterm=info,nyaterm_domain=info,nyaterm_session=info,warn")
    });
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .build(runtime.log_dir())
        .ok();

    if let Some(file_appender) = file_appender {
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().compact())
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(file_writer),
            )
            .try_init()
            .ok();
        Some(guard)
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .try_init()
            .ok();
        None
    }
}
