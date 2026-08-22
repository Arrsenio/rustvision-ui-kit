use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Frame, Margin, RichText, Sense, Stroke,
    TextStyle, Ui, Vec2, Visuals,
};

/// OLED black + soft ambient cyan/green Tron palette.
pub const OLED_BLACK: Color32 = Color32::BLACK;
pub const PANEL_BLACK: Color32 = Color32::BLACK;
pub const NEON: Color32 = Color32::from_rgb(170, 255, 242);
pub const NEON_DIM: Color32 = Color32::from_rgb(115, 212, 197);
pub const AMBER: Color32 = Color32::from_rgb(255, 176, 0);
pub const TEXT: Color32 = Color32::from_rgb(247, 255, 255);
pub const TEXT_DIM: Color32 = Color32::from_rgb(160, 205, 210);
/// Barely-there separator/border tint, so panels read as one surface on OLED.
pub const HAIRLINE: Color32 = Color32::from_rgb(30, 66, 68);

/// Outer padding. The status / navigation bars are hidden (see `Immersive.java`)
/// and the system already reserves the display-cutout strip, so these only need
/// to keep strokes off the physical screen edge.
pub const SAFE_TOP: f32 = 10.0;
/// Slightly larger than the top so the button row clears the home-swipe zone.
pub const SAFE_BOTTOM: f32 = 22.0;
pub const SCREEN_PAD: f32 = 12.0;
pub const PANEL_GAP: f32 = 8.0;
pub const TOUCH_MIN: f32 = 48.0;

pub fn apply_tron_theme(ctx: &Context) {
    // Do not inflate scale — phone PPP is already high enough.
    ctx.set_visuals(tron_visuals());

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(4.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 6.0);
    style.spacing.window_margin = Margin::same(0);
    style.spacing.indent = 6.0;
    style.interaction.interact_radius = 4.0;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(15.0, FontFamily::Monospace),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Monospace));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );

    ctx.set_style(style);
}

fn tron_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT);
    visuals.window_fill = OLED_BLACK;
    visuals.panel_fill = OLED_BLACK;
    visuals.extreme_bg_color = OLED_BLACK;
    visuals.faint_bg_color = OLED_BLACK;
    visuals.window_stroke = Stroke::new(1.0, NEON_DIM);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;

    visuals.widgets.noninteractive.bg_fill = OLED_BLACK;
    visuals.widgets.noninteractive.weak_bg_fill = OLED_BLACK;
    visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    visuals.widgets.inactive.bg_fill = OLED_BLACK;
    visuals.widgets.inactive.weak_bg_fill = OLED_BLACK;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, NEON_DIM);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0, 24, 28);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0, 24, 28);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2, NEON_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0, 36, 42);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0, 36, 42);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, NEON_DIM);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(115, 212, 197, 55);
    visuals.selection.stroke = Stroke::new(1.0, NEON_DIM);
    visuals.hyperlink_color = TEXT_DIM;
    visuals.error_fg_color = AMBER;
    visuals.warn_fg_color = AMBER;

    visuals
}

pub fn neon_stroke(width: f32) -> Stroke {
    Stroke::new(width, NEON)
}

/// Compact Tron panel. Border is a hairline so stacked panels don't read as
/// nested boxes; the corner brackets carry the accent instead.
pub fn panel_frame() -> Frame {
    Frame::NONE
        .fill(PANEL_BLACK)
        .stroke(Stroke::new(1.0, HAIRLINE))
        .inner_margin(Margin::symmetric(10, 8))
        .outer_margin(Margin::same(0))
}

pub fn paint_corner_brackets(painter: &egui::Painter, rect: egui::Rect, len: f32) {
    paint_corner_brackets_colored(painter, rect, len, NEON_DIM);
}

pub fn paint_corner_brackets_colored(
    painter: &egui::Painter,
    rect: egui::Rect,
    len: f32,
    color: Color32,
) {
    let stroke = Stroke::new(1.4, color);
    // Inset 1px so right/bottom brackets aren't clipped by the parent rect.
    let rect = rect.shrink(1.0);
    let corners = [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.left_bottom(), 1.0, -1.0),
        (rect.right_bottom(), -1.0, -1.0),
    ];
    for (origin, sx, sy) in corners {
        painter.line_segment([origin, origin + egui::vec2(len * sx, 0.0)], stroke);
        painter.line_segment([origin, origin + egui::vec2(0.0, len * sy)], stroke);
    }
}

/// Framed section: hairline border + accent corner brackets, drawn behind content.
pub fn paint_section(painter: &egui::Painter, rect: egui::Rect, accent: Color32) {
    let r = rect.shrink(0.5);
    painter.rect_stroke(
        r,
        CornerRadius::ZERO,
        Stroke::new(1.0, HAIRLINE),
        egui::StrokeKind::Inside,
    );
    paint_corner_brackets_colored(painter, r, 12.0, accent);
}

/// Thin full-width rule used under the header.
pub fn paint_rule(painter: &egui::Painter, rect: egui::Rect, y: f32) {
    painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0, HAIRLINE),
    );
}

pub fn draw_scanlines(painter: &egui::Painter, rect: egui::Rect) {
    let line = Color32::from_rgba_unmultiplied(115, 212, 197, 18);
    let step = 5.0;
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            Stroke::new(1.0, line),
        );
        y += step;
    }
}

pub fn brand_title(text: &str) -> RichText {
    RichText::new(text)
        .monospace()
        .size(17.0)
        .color(TEXT)
        .strong()
}

pub fn section_label(text: &str) -> RichText {
    RichText::new(text)
        .monospace()
        .size(11.0)
        .color(NEON_DIM)
}

/// Status chip: `LABEL` in dim caps, value in accent.
pub fn chip(text: impl Into<String>, accent: Color32) -> RichText {
    RichText::new(text.into()).monospace().size(11.0).color(accent)
}

pub fn status_text(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).monospace().size(13.0).color(TEXT)
}

pub fn reply_text(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).monospace().size(16.0).color(TEXT)
}

pub fn dim_text(text: impl Into<String>) -> RichText {
    RichText::new(text.into())
        .monospace()
        .size(12.0)
        .color(TEXT_DIM)
}

/// Primary control. `accent` lets SEND glow while it is the useful action.
pub fn tron_button(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    tron_button_accent(ui, label, enabled, NEON_DIM)
}

pub fn tron_button_accent(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    accent: Color32,
) -> egui::Response {
    let desired = Vec2::new(ui.available_width().max(64.0), TOUCH_MIN);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired, sense);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Inset so bottom/right strokes stay inside the clip rect.
        let draw = rect.shrink(1.0);
        let held = enabled && response.is_pointer_button_down_on();
        let border = if enabled { accent } else { HAIRLINE };
        let fill = if held {
            Color32::from_rgb(0, 44, 50)
        } else {
            OLED_BLACK
        };
        painter.rect_filled(draw, CornerRadius::ZERO, fill);
        painter.rect_stroke(
            draw,
            CornerRadius::ZERO,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        paint_corner_brackets_colored(painter, draw, 9.0, border);
        painter.text(
            draw.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::monospace(13.0),
            if enabled { TEXT } else { TEXT_DIM },
        );
    }

    response
}

/// Segmented Tron progress bar.
///
/// Filled as discrete cells rather than a smooth sweep so it reads as part of the
/// bracketed panel style. `fraction` is clamped to 0..=1; pass `time` (seconds) to
/// animate the leading cell, which is what tells the user a long opaque stage is
/// still alive.
pub fn tron_progress_bar(ui: &mut Ui, fraction: f32, time: f64) -> egui::Response {
    const HEIGHT: f32 = 14.0;
    const CELL: f32 = 7.0;
    const GAP: f32 = 2.0;

    let fraction = fraction.clamp(0.0, 1.0);
    let desired = Vec2::new(ui.available_width(), HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, HAIRLINE),
        egui::StrokeKind::Inside,
    );

    let track = rect.shrink2(Vec2::new(3.0, 3.0));
    let step = CELL + GAP;
    let cells = ((track.width() + GAP) / step).floor().max(1.0) as usize;
    let filled = (fraction * cells as f32).round() as usize;
    // Pulse between dim and full so the frontier cell reads as "working".
    let pulse = 0.55 + 0.45 * (time * 3.2).sin() as f32;

    for i in 0..cells {
        let x = track.left() + i as f32 * step;
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, track.top()),
            Vec2::new(CELL, track.height()),
        );
        if i < filled.saturating_sub(1) {
            painter.rect_filled(cell, CornerRadius::ZERO, NEON_DIM);
        } else if i < filled {
            painter.rect_filled(cell, CornerRadius::ZERO, NEON.gamma_multiply(pulse));
        } else {
            painter.rect_filled(cell, CornerRadius::ZERO, HAIRLINE.gamma_multiply(0.5));
        }
    }

    paint_corner_brackets_colored(painter, rect, 8.0, NEON_DIM);
    response
}

pub fn tron_button_wide(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), TOUCH_MIN);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let border = NEON_DIM;
        let fill = if response.is_pointer_button_down_on() && enabled {
            Color32::from_rgb(0, 40, 48)
        } else if enabled {
            Color32::from_rgb(0, 18, 22)
        } else {
            OLED_BLACK
        };
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.2, border),
            egui::StrokeKind::Inside,
        );
        paint_corner_brackets(painter, rect, 8.0);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::monospace(11.0),
            if enabled { TEXT } else { TEXT_DIM },
        );
    }

    response
}
