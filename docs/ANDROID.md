# Android immersive OLED shell

Goal: **no status bar, no navigation bar**, pure black edge-to-edge UI from the first frame.

## 1. Manifest / cargo-apk2 theme

In `Cargo.toml` package metadata, set the activity theme to fullscreen **before** native code runs so there is no status-bar flash:

```toml
[package.metadata.android.application]
theme = "@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen"

[[package.metadata.android.application.activity]]
name = "android.app.NativeActivity"
exported = true
theme = "@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen"
```

Ship Java helpers with:

```toml
[package.metadata.android]
java_sources = "java"
```

## 2. Immersive.java

Full source: [`reference/Immersive.java`](../reference/Immersive.java).

Behavior:

- API 30+: `setDecorFitsSystemWindows(false)`, hide `systemBars()`, transient bars on swipe.  
- Older: `SYSTEM_UI_FLAG_IMMERSIVE_STICKY` + re-apply on visibility change.  
- **Insets listener** re-hides bars after returning from gallery / camera — avoids polling with a stale Activity pointer.  
- Do **not** force `SHORT_EDGES` cutout mode; leave the punch-hole reserved (invisible on OLED black).

Always `runOnUiThread` for window ops.

## 3. JNI: Activity vs Application

`ndk_context::android_context().context()` returns the **Application** context.  
`Immersive.apply(Activity)` needs the **Activity**.

Store `AndroidApp::activity_as_ptr()` in a static at `android_main`, then pass that jobject into JNI. Passing Application → CheckJNI abort:

```text
attempt to pass an instance of android.app.Application as argument 1 to
void com.rustvision.app.Immersive.apply(android.app.Activity)
```

Apply immersive **once** after the activity pointer is valid. Prefer Java-side insets re-hide over a Rust timer that caches Activity.

## 4. Safe insets in egui

With bars hidden:

- `SAFE_TOP = 10`, `SAFE_BOTTOM = 22`, `SCREEN_PAD = 12`  
- System still reserves the camera cutout; your pad only keeps strokes off the physical edge and the gesture zone.

## 5. Clear color

Always clear to opaque black in eframe so the surface never flashes a non-black buffer.

## 6. Phantom taps after launch

Android often delivers leftover pointer events on the first frames. Gate button actions:

```rust
input_ready_at: ctx.input(|i| i.time) + 1.25
// ...
let input_live = ui.input(|i| i.time) >= self.input_ready_at;
```

## Checklist for a new OLED app

- [ ] Fullscreen theme in metadata  
- [ ] `Immersive.apply(Activity)` with real Activity  
- [ ] Insets / sticky re-hide after external activities  
- [ ] OLED clear color + theme fills  
- [ ] Bottom safe pad for gesture nav  
- [ ] Phantom-tap grace period  
