# RustVision UI Kit

**Replicable design system** extracted from the [RustVision](https://github.com/Arrsenio/rustvision-01) Android app: OLED-black Tron aesthetic, egui panels, custom widgets, and sticky immersive full-screen on Android.

This repo is documentation + reference source — not a runnable crate. Copy the patterns into any egui / eframe / cargo-apk2 project.

| Document | What it covers |
| --- | --- |
| [docs/DESIGN.md](docs/DESIGN.md) | Palette, typography, spacing, visual rules |
| [docs/LAYOUT.md](docs/LAYOUT.md) | Screen shell, tabs, viewport/reply split, history |
| [docs/COMPONENTS.md](docs/COMPONENTS.md) | Buttons, chips, progress bar, scanlines, preview |
| [docs/ANDROID.md](docs/ANDROID.md) | Fullscreen theme, Immersive.java, JNI Activity pitfalls |
| [docs/EGUI_PITFALLS.md](docs/EGUI_PITFALLS.md) | Widget ID collisions, phantom taps, letterboxing |
| [reference/tron.rs](reference/tron.rs) | Full theme module as used in production |
| [reference/Immersive.java](reference/Immersive.java) | Sticky immersive helper |

## Stack this UI assumes

- **Rust** + **egui 0.31** + **eframe** (`wgpu` / `glow`, `android-native-activity`)
- **cargo-apk2** for APK packaging
- Optional Java helpers packaged via `package.metadata.android.java_sources`

Desktop egui apps can use the same theme and layout without the Android sections.

## Design in one sentence

Pure black OLED canvas, monospace HUD typography, hairline borders, neon corner brackets instead of heavy cards, and 48dp touch targets — no rounded pills, no purple gradients, no floating badges.

## Quick start (new egui app)

1. Copy `reference/tron.rs` into a `theme` module (or crate depending on `egui`).
2. Call `apply_tron_theme(&cc.egui_ctx)` once in `App::new`.
3. Build the shell with `TopBottomPanel` (header + controls) and `CentralPanel` (content). Use `Frame::NONE.fill(OLED_BLACK)` and the spacing constants from the theme.
4. Draw content sections with `paint_section`, labels with `section_label` / `chip` / `reply_text`.
5. On Android, add the fullscreen theme + `Immersive.java` (see [docs/ANDROID.md](docs/ANDROID.md)).

## Source app

Patterns here were proven in **NORTH MICROSIGHT** (`rustvision-01`): live Camera2 viewport, staged inference progress bar, VISION / HISTORY tabs, capture archive with preview and copy.

## License

Reference code and docs are provided as-is for reuse in your own projects. Match the license of your downstream app.
