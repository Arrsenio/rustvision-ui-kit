# Snippets

Minimal patterns you can paste while reading the longer docs.

## Panel shell

```rust
egui::TopBottomPanel::top("header")
    .frame(egui::Frame::NONE.fill(OLED_BLACK).inner_margin(egui::Margin {
        left: SCREEN_PAD as i8,
        right: SCREEN_PAD as i8,
        top: SAFE_TOP as i8,
        bottom: 0,
    }))
    .show_separator_line(false)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(brand_title("YOUR APP"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(chip("READY", NEON_DIM));
            });
        });
        ui.add_space(6.0);
        paint_rule(ui.painter(), ui.max_rect(), ui.cursor().top() - 2.0);
    });

egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(OLED_BLACK).inner_margin(egui::Margin {
        left: SCREEN_PAD as i8,
        right: SCREEN_PAD as i8,
        top: PANEL_GAP as i8,
        bottom: 0,
    }))
    .show(ctx, |ui| {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(
            egui::UiBuilder::new().id_salt("main").max_rect(area.shrink(10.0)),
            |ui| {
                ui.label(section_label("◆ MAIN"));
                // ...
            },
        );
    });
```

## Button row

```rust
let mut action = None;
ui.columns(3, |cols| {
    for (i, (id, label, on, accent)) in [
        ("a", "[ ONE ]", true, NEON_DIM),
        ("b", "[ TWO ]", true, NEON_DIM),
        ("c", "[ GO ]", true, NEON),
    ]
    .into_iter()
    .enumerate()
    {
        if cols[i]
            .push_id(id, |ui| tron_button_accent(ui, label, on, accent))
            .inner
            .clicked()
            && on
        {
            action = Some(i);
        }
    }
});
```

## Letterboxed texture

```rust
let tex_size = tex.size_vec2();
let scale = (inner.width() / tex_size.x)
    .min(inner.height() / tex_size.y)
    .max(0.001);
let img_rect = egui::Rect::from_center_size(inner.center(), tex_size * scale);
let sized = egui::load::SizedTexture::from_handle(tex);
egui::Image::from_texture(sized).paint_at(ui, img_rect);
```
