# Layout

## Screen shell (VISION tab)

Use egui’s panel system — not hand-placed absolute rects for the whole screen. Manual rect math caused the early “squished” layout; panels reflow cleanly.

```
┌─────────────────────────────────────┐
│ TopBottomPanel::top  "header"       │  SAFE_TOP + brand + chips + tabs + rule
├─────────────────────────────────────┤
│                                     │
│ CentralPanel                        │  viewport (~46%) + reply (rest)
│   ◆ VIEWPORT                        │
│   ◆ REPLY (+ progress when running) │
│                                     │
├─────────────────────────────────────┤
│ TopBottomPanel::bottom "controls"   │  ◆ PROMPT + button row + SAFE_BOTTOM
└─────────────────────────────────────┘
```

### Header

1. Horizontal: `brand_title` left, status `chip`s right-to-left (`IMG · ENGINE · MODEL`).  
2. Tab row: `VISION` / `HISTORY (n)` via outlined tab chips.  
3. `paint_rule` under the tabs.

Frame:

```rust
egui::TopBottomPanel::top("rv_header")
    .frame(Frame::NONE.fill(OLED_BLACK).inner_margin(Margin {
        left: SCREEN_PAD as i8,
        right: SCREEN_PAD as i8,
        top: SAFE_TOP as i8,
        bottom: 0,
    }))
    .show_separator_line(false)
```

### Central: viewport + reply

Split available height: viewport gets ~46% (clamped), reply gets the remainder, with `PANEL_GAP` between.

1. Compute `viewport_rect` and `reply_rect` from `ui.available_rect_before_wrap()`.  
2. `paint_section` each rect (hairline + brackets). Accent goes bright (`NEON`) when live / generating.  
3. `ui.scope_builder(UiBuilder::new().id_salt("viewport").max_rect(...))` for content — **unique `id_salt`** is mandatory (see EGUI_PITFALLS).  
4. Images **letterbox** (fit inside, preserve aspect). Never stretch to exact panel size.

Live overlay: corner brackets on the image + `● REC · CAP OR TAP` at top-left.

### Bottom controls

1. Section label `◆ PROMPT`.  
2. Single-line `TextEdit`, full width, height ~42.  
3. Equal-width button row via `ui.columns(N, …)` — currently four: `LOAD · LIVE/STOP · CAP · SEND`.

Accent rules:

| Control | Accent when |
| --- | --- |
| LIVE | `NEON_DIM`; STOP uses `AMBER` |
| CAP | `NEON` only while feed is on; else `TEXT_DIM` |
| SEND | `NEON` when image ready and not generating |

## HISTORY tab

Same header. Bottom panel swaps to a single `[ CLEAR ALL ]` (amber when non-empty). Central panel becomes a scrollable archive:

- Each entry: hairline frame, kind chip (`CAPTURE` / `REPLY`), relative time, `[ COPY ]` / `[ DEL ]`.  
- Thumbnail (72×72) + selectable prompt/reply text.  
- Tap thumbnail → fullscreen **PREVIEW** (large texture, letterboxed, CLOSE or tap to dismiss).  
- COPY copies **reply only**.

## Composition rules (replicate the look)

1. **One black surface** — no nested grey cards.  
2. **Brand in the header**, not buried as an eyebrow under a louder headline.  
3. **One job per section** — viewport shows media; reply shows text; controls own input.  
4. **No hero overlays** except functional live HUD (REC).  
5. Prefer section labels with a diamond prefix: `◆ VIEWPORT`, `◆ REPLY`, `◆ ARCHIVE`.
