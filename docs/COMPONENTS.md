# Components

All production widgets live in [`reference/tron.rs`](../reference/tron.rs). This page describes how to use and extend them.

## Theme bootstrap

```rust
apply_tron_theme(&cc.egui_ctx);
```

Sets dark visuals, OLED fills, neon strokes, monospace text styles, and tight spacing.

## Text helpers

```rust
ui.label(brand_title("NORTH MICROSIGHT"));
ui.label(section_label("◆ VIEWPORT"));
ui.label(chip("LIVE", NEON));
ui.label(dim_text("secondary hint"));
ui.add(egui::Label::new(reply_text(body)).wrap().selectable(true));
```

## Corner brackets & sections

```rust
paint_section(ui.painter(), rect, NEON_DIM);           // hairline + brackets
paint_corner_brackets_colored(painter, rect, 16.0, NEON);
paint_rule(painter, max_rect, y);                      // header separator
draw_scanlines(painter, empty_rect);                   // empty-state texture
```

Bracket implementation: four independent L’s from each corner, inset 1px so strokes aren’t clipped.

## `tron_button_accent`

Custom painted control (not egui’s default `Button`):

- Height `TOUCH_MIN` (48), width = available column width.  
- Disabled → hover-only sense, hairline border, dim text.  
- Enabled → accent border + brackets; press darkens fill.  
- Label centered, monospace 13.

```rust
tron_button_accent(ui, "[ SEND ]", enabled, NEON);
```

### Column ID rule

`ui.columns` children share a default id salt. Always wrap each button:

```rust
ui.columns(4, |cols| {
    cols[i].push_id("btn_send", |ui| {
        tron_button_accent(ui, "[ SEND ]", on, accent)
    });
});
```

Without unique ids, one tap fires every button.

## Segmented progress bar

```rust
tron_progress_bar(ui, fraction /* 0..=1 */, now /* ctx time */);
```

Discrete cells, hairline track, pulsing leading cell. Pair with a `%` chip and a stage line (`ENCODING IMAGE · 12s`).

## Tab chip

Outlined black button, neon stroke when active:

```rust
fn tab_chip(ui: &mut Ui, label: &str, active: bool) -> bool {
    let color = if active { NEON } else { TEXT_DIM };
    let stroke = Stroke::new(1.0, if active { NEON_DIM } else { HAIRLINE });
    ui.add(
        Button::new(RichText::new(format!(" {label} ")).monospace().size(12.0).color(color))
            .stroke(stroke)
            .fill(OLED_BLACK),
    )
    .clicked()
}
```

## Image presentation

1. Decode / upload to `TextureHandle`.  
2. Letterbox into available rect (`min(w_scale, h_scale)`).  
3. `Image::from_texture(...).paint_at(ui, img_rect)`.  
4. Optional hairline + brackets around `img_rect`.  
5. Live: `tex.set(color_image, LINEAR)` each frame — do not allocate a new texture at 30 fps.

## History-style list row

- `Frame::NONE.stroke(HAIRLINE).inner_margin(10)`  
- Header chips + text action buttons (`.frame(false)` so they stay HUD-flat)  
- Horizontal: clickable thumbnail + selectable text column  

## Copy / clipboard (Android)

egui `ctx.copy_text` alone is unreliable on NativeActivity. Mirror with a Java `ClipboardHelper.setText(Context, String)` JNI call (see the main app). Desktop can rely on egui alone.
