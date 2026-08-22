# Replication recipe

End-to-end checklist to recreate this UI in a new egui Android (or desktop) app.

## A. Theme crate / module

1. Add dependency: `egui = "0.31"` (or match your eframe version).  
2. Copy [`reference/tron.rs`](../reference/tron.rs) into `src/theme/tron.rs` (or a `*-theme` crate).  
3. Export `pub mod tron;`.  
4. In `App::new` / `CreationContext`: `apply_tron_theme(&cc.egui_ctx)`.  
5. Set `clear_color` to `[0,0,0,1]`.

## B. Screen shell

1. Top panel: brand + status chips + optional tabs + `paint_rule`.  
2. Bottom panel: primary input + equal-width `tron_button_accent` row with **unique `push_id`s**.  
3. Central panel: split into sections; `paint_section` behind each; content in `scope_builder` with unique `id_salt`.  
4. Use spacing constants (`SAFE_*`, `SCREEN_PAD`, `PANEL_GAP`) — don’t invent denser padding.

## C. Visual details that sell the look

1. Section titles: `◆ NAME` via `section_label`.  
2. Empty media: `draw_scanlines` + centered `NO SIGNAL`.  
3. Images letterboxed with optional brackets.  
4. Progress: `tron_progress_bar` + `%` chip + stage line.  
5. Accent only when something is “hot” (live, runnable SEND, generating).

## D. Android OLED

1. Fullscreen NoActionBar theme in cargo-apk2 metadata.  
2. Copy [`reference/Immersive.java`](../reference/Immersive.java); call with **Activity**, not Application.  
3. Re-hide via insets listener after external intents.  
4. Phantom-tap grace (~1.25s) before accepting button clicks.  
5. Bottom pad ≥ ~22 logical px for gesture nav.

## E. Don’t copy (app-specific)

Inference, Camera2 YUV pipeline, GGUF loading, and history persistence are product features of RustVision — not required for the UI kit. Steal the **presentation** patterns (archive list, preview overlay, progress staging) if useful.

## F. Verify visually

On a real OLED device (or black emulator skin):

- [ ] No status / nav bar after launch and after gallery return  
- [ ] No grey clear flashes  
- [ ] Buttons only fire their own action  
- [ ] Portrait images not stretched  
- [ ] Touch targets comfortable; bottom row above gesture zone  
- [ ] Brand readable as the primary header signal  
