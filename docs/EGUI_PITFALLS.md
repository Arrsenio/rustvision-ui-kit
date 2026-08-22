# egui pitfalls (learned the hard way)

## Widget ID collisions in `columns`

`ui.columns(N, …)` gives each column a Ui whose default id salt is the same (`"child"`). Identical widgets (three custom buttons) **merge into one logical widget**. A tap anywhere in the row can fire LOAD and SEND together.

**Fix:** `push_id("unique_salt", …)` per control, and unique `id_salt` on every `UiBuilder` / `scope_builder` (`"viewport"`, `"reply"`, …).

## Manual full-screen rect math

Computing every region as a fraction of screen size produced a squished layout across densities and cutouts. Prefer:

1. `TopBottomPanel` for header / chrome  
2. `CentralPanel` for the flexible middle  
3. Split only the **available** central rect  

## Stretching images

Fitting a texture exactly to the panel rect destroys aspect ratio (portrait photos look landscape). Always letterbox:

```rust
let scale = (inner.width() / tex_size.x)
    .min(inner.height() / tex_size.y)
    .max(0.001);
let img_rect = Rect::from_center_size(inner.center(), tex_size * scale);
```

## Texture churn

Creating a new `TextureHandle` every camera frame stalls the GPU. Keep one live texture and `set()` RGBA each frame.

## Live preview via JPEG + disk ≈ 3 fps

Encoding camera frames as JPEG, writing files, throttling UI poll, and decoding in Rust capped the viewport around 3 fps. For realtime: `YUV_420_888` → RGBA in Java, **direct ByteBuffer**, no frame skip, reuse texture on the Rust side.

## Progress that “looks hung”

A single `eval_chunks` (or any long FFI) with only Started/Finished events freezes a bar at 0%. Split stages, emit fractional progress, and pulse the leading cell during opaque stages.

## Selectable / copyable text

```rust
Label::new(...).wrap().selectable(true)
```

On Android NativeActivity, also push to the system clipboard via JNI.

## Nested panels looking “cardy”

Use `HAIRLINE` strokes and let **corner brackets** carry accent. Thick neon rectangles around every panel recreate a dashboard of boxes — the opposite of the OLED HUD look.
