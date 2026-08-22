mod android_bridge;
mod app;
mod audio8;
mod theme;

pub use app::TtsApp;

use eframe::NativeOptions;
use theme::tron::apply_tron_theme;

pub fn native_options() -> NativeOptions {
    NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Audio8 TTS")
            .with_inner_size([412.0, 892.0])
            .with_min_inner_size([360.0, 640.0]),
        ..Default::default()
    }
}

pub fn start(options: NativeOptions) -> eframe::Result {
    eframe::run_native(
        "Audio8 TTS",
        options,
        Box::new(|cc| {
            apply_tron_theme(&cc.egui_ctx);
            Ok(Box::new(TtsApp::new(cc)))
        }),
    )
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    android_bridge::store_app(app.clone());
    android_bridge::apply_immersive();
    let options = NativeOptions {
        android_app: Some(app),
        ..native_options()
    };
    let _ = start(options);
}

#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> eframe::Result {
    let _ = env_logger::try_init();
    start(native_options())
}
