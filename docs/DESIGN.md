# Design system

## Intent

An immersive **OLED HUD**: the phone’s black pixels are the brand. Content sits on one continuous black surface. Accent is a soft cyan/green neon — used sparingly for brackets, active states, and status chips. Amber is reserved for warnings / stop / delete.

Avoid:

- Purple / indigo AI-default gradients  
- Cream / terracotta editorial looks  
- Soft cards with multi-layer shadows  
- Rounded-full pills and emoji decoration  
- Dense “dashboard” chrome in the first viewport  

## Palette

| Token | RGB | Hex (approx) | Role |
| --- | --- | --- | --- |
| `OLED_BLACK` | `0,0,0` | `#000000` | App clear color, panel fill |
| `NEON` | `170,255,242` | `#AAFFF2` | Primary accent (active, REC, hot SEND) |
| `NEON_DIM` | `115,212,197` | `#73D4C5` | Default borders, idle accents |
| `AMBER` | `255,176,0` | `#FFB000` | STOP, DEL, errors |
| `TEXT` | `247,255,255` | `#F7FFFF` | Primary copy |
| `TEXT_DIM` | `160,205,210` | `#A0CDD2` | Secondary / disabled labels |
| `HAIRLINE` | `30,66,68` | `#1E4244` | Quiet separators (almost invisible on OLED) |

Hover / pressed fills (buttons):

- Hover: `rgb(0, 24, 28)`  
- Active / held: `rgb(0, 36, 42)` … `rgb(0, 44, 50)`  

Selection highlight: neon at ~55 alpha.

## Typography

Everything is **monospace** (`FontFamily::Monospace`). Sizes used in production:

| Style | Size | Helper |
| --- | --- | --- |
| Brand title | 17, strong | `brand_title` |
| Body (prompt, default) | 14 | egui `TextStyle::Body` |
| Reply / long read | 16 | `reply_text` |
| Button label | 13 | painted in `tron_button_accent` |
| Section label (`◆ NAME`) | 11, neon dim | `section_label` |
| Status chip | 11 | `chip` |
| Dim helper | 12 | `dim_text` |
| Overlay HUD (`● REC`) | 10 | painter text |

egui style overrides (see `apply_tron_theme`): Heading 15, Body 14, Button 13, Small 12, Monospace 13. Do **not** inflate `pixels_per_point` on phones — PPP is already high.

## Spacing & touch

| Constant | Value | Why |
| --- | --- | --- |
| `SAFE_TOP` | 10 | Edge clearance after bars are hidden; cutout is system-reserved |
| `SAFE_BOTTOM` | 22 | Clears the home-gesture swipe zone |
| `SCREEN_PAD` | 12 | Left/right inset for all panels |
| `PANEL_GAP` | 8 | Gap between stacked sections |
| `TOUCH_MIN` | 48 | Minimum button height (≈ Material touch target) |

Item spacing inside egui style: `4×4`. Button padding in style: `8×6` (custom buttons allocate their own rects).

## Shape language

- **Zero corner radius** everywhere (`CornerRadius::ZERO`).  
- Sections: **hairline rectangle** + **L-shaped corner brackets** (accent color). Brackets carry identity; the border stays quiet so stacked panels don’t look nested.  
- Buttons: filled black, 1px accent stroke, smaller brackets (len ≈ 9).  
- Progress: **segmented cells**, not a smooth bar — matches the bracketed HUD.  
- Empty states: faint **scanlines** + centered monospace status (`NO SIGNAL`, `NO ENTRIES`).

## Motion

Keep motion functional:

- Progress leading cell **pulses** with `sin(time * 3.2)` so opaque stages still feel alive.  
- Live camera: continuous texture update (no decorative animation).  
- Prefer `ctx.request_repaint()` while generating or live.

## Clear color

eframe `clear_color` must be opaque black:

```rust
fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}
```

Otherwise Android shows through as grey/white between frames.
