//! Fonts, the in-game HUD and the front-end screens.
//!
//! Everything is drawn in the 2D pass with macroquad shapes and text: no image
//! assets and no shaders, so the interface looks the same whether or not the
//! bundled CJK font is present.
//!
//! Layout numbers are authored against a 1280x720 window and scaled from the
//! real one, so the frame keeps its proportions on any resolution or DPI.
//!
//! Per frame the caller draws, in this order:
//! `draw_vignette(hurt, low_health)` so the damage flash sits under the HUD,
//! then `draw_hud`, or `draw_menu` / `draw_result` for the front end.

use crate::i18n::{self, Key};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::fmt::{self, Write as _};

// --- palette --------------------------------------------------------------

/// Near-white base ink. Everything unaccented is drawn from this.
const INK: Color = Color::new(0.88, 0.90, 0.93, 1.0);
/// Translucent panel fill, dark enough to keep text legible over the 3D pass.
const PANEL: Color = Color::new(0.05, 0.06, 0.08, 0.72);
/// Amber-orange primary accent.
const ACCENT: Color = Color::new(0.85, 0.45, 0.12, 1.0);
/// Cyan secondary / technical accent.
const CYAN: Color = Color::new(0.35, 0.80, 1.0, 1.0);
/// Damage and failure red.
const DANGER: Color = Color::new(0.85, 0.18, 0.16, 1.0);

/// Height the layout is authored against.
const REF_H: f32 = 720.0;
const MARGIN: f32 = 24.0;
const PAD: f32 = 14.0;
/// Every corner readout is the same height so the frame reads as one system.
const BLOCK_H: f32 = 108.0;

/// Layout units. One unit is one pixel in a 1280x720 window; every length and
/// font size in this module is written in those units and run through `s`.
#[derive(Clone, Copy)]
struct Ui {
    scale: f32,
}

impl Ui {
    fn new() -> Ui {
        Ui {
            scale: (screen_height() / REF_H).clamp(0.55, 1.5),
        }
    }

    fn s(&self, v: f32) -> f32 {
        v * self.scale
    }
}

// --- colour and typography helpers ----------------------------------------

fn fade(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a.clamp(0.0, 1.0))
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Sine wave in `0.0..=1.0`, used for the flashing parts of the HUD.
fn pulse(speed: f32) -> f32 {
    0.5 - 0.5 * (get_time() as f32 * speed).cos()
}

/// Font sizes end up as `u16` inside macroquad; keep the cast in one place and
/// never ask for a zero-sized glyph.
fn font_px(size: f32) -> u16 {
    size.clamp(1.0, 220.0).round() as u16
}

/// Width of `s` once `tracking` extra pixels are added between characters.
fn tracked_w(f: &Fonts, s: &str, size: f32, tracking: f32) -> f32 {
    let mut buf = [0u8; 4];
    let mut w = -tracking;
    for ch in s.chars() {
        w += f.width(ch.encode_utf8(&mut buf), size) + tracking;
    }
    w.max(0.0)
}

fn tracked(f: &Fonts, s: &str, x: f32, y: f32, size: f32, color: Color, tracking: f32) {
    let mut buf = [0u8; 4];
    let mut cx = x;
    for ch in s.chars() {
        let part = ch.encode_utf8(&mut buf);
        f.text(part, cx, y, size, color);
        cx += f.width(part, size) + tracking;
    }
}

fn tracked_c(f: &Fonts, s: &str, cx: f32, y: f32, size: f32, color: Color, tracking: f32) {
    tracked(
        f,
        s,
        cx - tracked_w(f, s, size, tracking) * 0.5,
        y,
        size,
        color,
        tracking,
    );
}

fn tracked_r(f: &Fonts, s: &str, x: f32, y: f32, size: f32, color: Color, tracking: f32) {
    tracked(
        f,
        s,
        x - tracked_w(f, s, size, tracking),
        y,
        size,
        color,
        tracking,
    );
}

/// Writes the longest prefix of `s` that fits in `max_w` into `out`, ending on
/// `..` when it had to be cut, and returns it. `out` is cleared first and keeps
/// its allocation; `measure` is the font's width function, taken as a parameter
/// so the fitting can be exercised without a font.
///
/// The accepted text is always a prefix of `s`, so the candidates are measured
/// as slices of `s`. The copy-per-character this replaced cost a run of
/// allocations every time a line had to be trimmed.
fn fit_into<'a>(
    out: &'a mut String,
    s: &str,
    size: f32,
    max_w: f32,
    measure: &dyn Fn(&str, f32) -> f32,
) -> &'a str {
    out.clear();
    if measure(s, size) <= max_w {
        out.push_str(s);
        return out;
    }
    let dots = measure("..", size);
    let mut end = 0;
    for (i, ch) in s.char_indices() {
        let next = i + ch.len_utf8();
        if measure(&s[..next], size) + dots > max_w {
            break;
        }
        end = next;
    }
    out.push_str(&s[..end]);
    out.push_str("..");
    out
}

/// `fit_into` into a fresh buffer, for the front-end screens, which are drawn
/// once in a while and have nowhere to keep a buffer between frames.
fn fit(f: &Fonts, s: &str, size: f32, max_w: f32) -> String {
    let mut out = String::new();
    fit_into(&mut out, s, size, max_w, &|text, size| f.width(text, size));
    out
}

// --- shape helpers --------------------------------------------------------

/// Four short L-marks at the corners of a rect; the module's signature detail.
fn corner_marks(x: f32, y: f32, w: f32, h: f32, len: f32, t: f32, c: Color) {
    draw_rectangle(x, y, len, t, c);
    draw_rectangle(x, y, t, len, c);
    draw_rectangle(x + w - len, y, len, t, c);
    draw_rectangle(x + w - t, y, t, len, c);
    draw_rectangle(x, y + h - t, len, t, c);
    draw_rectangle(x, y + h - len, t, len, c);
    draw_rectangle(x + w - len, y + h - t, len, t, c);
    draw_rectangle(x + w - t, y + h - len, t, len, c);
}

fn panel(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, PANEL);
    draw_rectangle_lines(x, y, w, h, 1.0, fade(INK, 0.16));
    corner_marks(x, y, w, h, 11.0, 2.0, fade(ACCENT, 0.85));
}

fn bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.55));
    let fill = (w * frac.clamp(0.0, 1.0)).round();
    if fill > 0.5 {
        draw_rectangle(x, y, fill, h, color);
        draw_rectangle(x, y, fill, (h * 0.22).min(3.0), fade(INK, 0.18));
    }
    draw_rectangle_lines(x, y, w, h, 1.0, fade(INK, 0.20));
}

fn seg_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color, segments: usize) {
    bar(x, y, w, h, frac, color);
    if segments < 2 {
        return;
    }
    let seg = w / segments as f32;
    for i in 1..segments {
        let sx = (x + seg * i as f32).round();
        draw_rectangle(
            sx,
            y + 1.0,
            1.0,
            h - 2.0,
            Color::new(0.04, 0.045, 0.06, 0.85),
        );
    }
}

/// Amber while healthy, sliding to red as the bar empties.
fn health_color(frac: f32) -> Color {
    if frac > 0.55 {
        mix(ACCENT, INK, (frac - 0.55) / 0.45)
    } else if frac > 0.25 {
        mix(DANGER, ACCENT, (frac - 0.25) / 0.30)
    } else {
        DANGER
    }
}

/// Rewrites `out` with `args`, keeping its allocation. The HUD reformats about a
/// dozen strings a frame and nearly all of them hold the same digits, or the
/// same number of digits, as the frame before.
fn put(out: &mut String, args: fmt::Arguments<'_>) {
    out.clear();
    // Writing into a `String` cannot fail.
    let _ = out.write_fmt(args);
}

// --- fonts ----------------------------------------------------------------

pub struct Fonts {
    font: Option<Font>,
}

/// The subset CJK font, compiled into the executable.
///
/// Loading it by relative path only works when the process happens to start in
/// the project root — which `cargo run` does and a released executable
/// double-clicked from anywhere else does not. Embedding removes the working
/// directory from the equation entirely.
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/fonts/ui_zh.ttf");

impl Fonts {
    /// Loads the bundled CJK font, falling back to macroquad's built-in font
    /// when it is missing or unreadable. Never panics.
    pub async fn load() -> Fonts {
        if let Ok(font) = macroquad::text::load_ttf_font_from_bytes(BUNDLED_FONT) {
            return Fonts { font: Some(font) };
        }
        for path in ["assets/fonts/ui_zh.ttf", "assets/fonts/ui_zh.otf"] {
            if let Ok(font) = load_ttf_font(path).await {
                return Fonts { font: Some(font) };
            }
        }
        Fonts { font: None }
    }

    /// Draw text using the CJK font when available, else the default font.
    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        if s.is_empty() {
            return;
        }
        if self.font.is_some() {
            draw_text_ex(s, x, y, self.params(size, color));
        } else {
            draw_text(s, x, y, size, color);
        }
    }

    /// Horizontally centred on `cx`.
    pub fn text_c(&self, s: &str, cx: f32, y: f32, size: f32, color: Color) {
        self.text(s, cx - self.width(s, size) * 0.5, y, size, color);
    }

    /// Right-aligned to `x`.
    pub fn text_r(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        self.text(s, x - self.width(s, size), y, size, color);
    }

    pub fn width(&self, s: &str, size: f32) -> f32 {
        if s.is_empty() {
            return 0.0;
        }
        measure_text(s, self.font.as_ref(), font_px(size), 1.0).width
    }

    fn params(&self, size: f32, color: Color) -> TextParams<'_> {
        TextParams {
            font: self.font.as_ref(),
            font_size: font_px(size),
            font_scale: 1.0,
            color,
            ..Default::default()
        }
    }
}

// --- in-game HUD ----------------------------------------------------------

/// Everything the in-game HUD needs. Primitive fields only — do not couple to
/// the game's own structs.
pub struct HudState<'a> {
    pub health: f32,
    pub max_health: f32,
    pub armor: f32,
    pub score: u32,
    pub kills: u32,
    pub elapsed: f32,
    pub enemies_left: usize,
    pub objective: &'a str,
    pub objective_sub: &'a str,
    pub weapon_name: &'a str,
    pub mag: u32,
    pub mag_size: u32,
    /// `Some(0.0..=1.0)` while reloading, else `None`.
    pub reload_progress: Option<f32>,
    /// Short labels for each carried weapon plus which one is active.
    pub slots: Vec<(&'a str, bool)>,
    /// Current shot spread, in radians.
    pub spread: f32,
    pub hit_marker: f32,
    pub headshot_marker: f32,
    /// 0..=1 red damage flash.
    pub hurt: f32,
    /// True when health is critically low; drives a persistent pulse.
    pub low_health: bool,
    pub prompt: Option<&'a str>,
    /// Large transient centre-screen text, e.g. "WAVE 2". (`text`, alpha)
    pub banner: Option<(&'a str, f32)>,
    /// True while the mission is still running; false freezes objective styling.
    pub locked: bool,
}

/// Every string the in-game HUD rewrites each frame, kept between frames so the
/// buffers are reused rather than reallocated. Score, kills and the clock barely
/// move; ammo and health move without changing width. Reformatted into the same
/// buffers, a steady frame allocates nothing at all.
#[derive(Default)]
struct HudText {
    health: String,
    armor: String,
    mag: String,
    mag_size: String,
    slot_keys: Vec<String>,
    hostiles: String,
    score: String,
    kills: String,
    time: String,
    // Filled by the draw functions, which are the only places that know how much
    // room each value has to fit into.
    weapon: String,
    slot_label: String,
    objective: String,
    objective_sub: String,
    prompt: String,
}

impl HudText {
    /// Reformats the values a frame always shows.
    fn refresh(&mut self, s: &HudState) {
        put(
            &mut self.health,
            format_args!("{:.0}", s.health.max(0.0).ceil()),
        );
        if s.armor > 0.0 {
            put(&mut self.armor, format_args!("{:.0}", s.armor.ceil()));
        }
        if s.reload_progress.is_none() {
            put(&mut self.mag_size, format_args!("/ {}", s.mag_size));
            put(&mut self.mag, format_args!("{}", s.mag));
        }
        let n = s.slots.len();
        if self.slot_keys.len() < n {
            self.slot_keys.resize_with(n, String::new);
        }
        for (i, key) in self.slot_keys.iter_mut().take(n).enumerate() {
            put(key, format_args!("{}", i + 1));
        }
        put(&mut self.hostiles, format_args!("{}", s.enemies_left));
        put(&mut self.score, format_args!("{}", s.score));
        put(&mut self.kills, format_args!("{}", s.kills));
        let secs = s.elapsed.max(0.0) as u32;
        put(
            &mut self.time,
            format_args!("{}:{:02}", secs / 60, secs % 60),
        );
    }
}

thread_local! {
    /// macroquad draws from one thread, so a thread-local is the whole of the
    /// sharing problem here.
    static TEXT: RefCell<HudText> = RefCell::new(HudText::default());
}

pub fn draw_hud(f: &Fonts, s: &HudState) {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();

    TEXT.with(|cell| {
        let mut t = cell.borrow_mut();
        t.refresh(s);

        draw_crosshair(s, w, h);
        draw_health(f, s, &mut t, &ui, h);
        draw_ammo(f, s, &mut t, &ui, w, h);
        draw_objective(f, s, &mut t, &ui);
        draw_stats(f, &t, &ui, w);

        if let Some(prompt) = s.prompt {
            draw_prompt(f, &mut t, prompt, &ui, w, h);
        }
        if let Some((text, alpha)) = s.banner {
            draw_banner(f, text, alpha, &ui, w, h);
        }
    });
}

fn draw_crosshair(s: &HudState, w: f32, h: f32) {
    let ui = Ui::new();
    let cx = w * 0.5;
    let cy = h * 0.5;

    // Project the angular spread into screen pixels so a tighter gun really does
    // get a tighter gap.
    let gap = (s.spread * h * 0.92).clamp(ui.s(6.0), ui.s(96.0));
    let body = s.hit_marker.clamp(0.0, 1.0);
    let head = s.headshot_marker.clamp(0.0, 1.0);
    let head_col = mix(ACCENT, WHITE, 0.30);
    let col = mix(mix(fade(INK, 0.92), CYAN, body), head_col, head);

    let len = ui.s(8.0) * (1.0 + head * 0.45);
    let under = Color::new(0.0, 0.0, 0.0, 0.35 * (1.0 - body.max(head) * 0.5));
    for (dx, dy) in [(0.0f32, -1.0f32), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
        let ax = cx + dx * gap;
        let ay = cy + dy * gap;
        draw_line(ax, ay, ax + dx * len, ay + dy * len, ui.s(4.0), under);
        draw_line(ax, ay, ax + dx * len, ay + dy * len, ui.s(2.0), col);
    }
    draw_circle(cx, cy, ui.s(1.6 + head * 0.8), col);

    if body > 0.002 {
        hit_ticks(cx, cy, CYAN, body, &ui, 9.0);
    }
    if head > 0.002 {
        hit_ticks(cx, cy, head_col, head, &ui, 13.0);
    }
}

/// Four diagonal ticks that snap inward as the hit marker fades.
fn hit_ticks(cx: f32, cy: f32, color: Color, amt: f32, ui: &Ui, reach: f32) {
    let a = amt.clamp(0.0, 1.0);
    let inner = ui.s(6.0 + (1.0 - a) * 5.0);
    let outer = inner + ui.s(reach);
    let c = fade(color, 0.25 + 0.75 * a);
    // Corner direction, normalised.
    const DIAG: f32 = std::f32::consts::FRAC_1_SQRT_2;
    for (dx, dy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        draw_line(
            cx + dx * inner * DIAG,
            cy + dy * inner * DIAG,
            cx + dx * outer * DIAG,
            cy + dy * outer * DIAG,
            ui.s(2.0),
            c,
        );
    }
}

fn draw_health(f: &Fonts, s: &HudState, t: &mut HudText, ui: &Ui, h: f32) {
    let pw = ui.s(300.0);
    let x = ui.s(MARGIN);
    let y = h - ui.s(MARGIN) - ui.s(BLOCK_H);
    panel(x, y, pw, ui.s(BLOCK_H));

    let ix = x + ui.s(PAD);
    let right = x + pw - ui.s(PAD);
    let iw = pw - ui.s(PAD * 2.0);

    let max_hp = if s.max_health > 0.0 {
        s.max_health
    } else {
        1.0
    };
    let frac = (s.health / max_hp).clamp(0.0, 1.0);
    let col = health_color(frac);
    let hurt = s.hurt.clamp(0.0, 1.0);

    tracked(
        f,
        i18n::t(Key::Health),
        ix,
        y + ui.s(26.0),
        ui.s(12.0),
        fade(INK, 0.55),
        ui.s(2.0),
    );
    f.text_r(
        &t.health,
        right,
        y + ui.s(44.0),
        ui.s(30.0),
        mix(col, mix(DANGER, WHITE, 0.15), hurt * 0.6),
    );
    // Incoming damage also blinks the underside of the readout.
    if hurt > 0.01 {
        draw_rectangle(
            x + 1.0,
            y + ui.s(BLOCK_H) - ui.s(3.0),
            pw - 2.0,
            ui.s(2.0),
            fade(DANGER, 0.7 * hurt),
        );
    }

    // Armor is a thinner cyan strip above the health bar, gone entirely at zero.
    if s.armor > 0.0 {
        bar(
            ix,
            y + ui.s(54.0),
            iw,
            ui.s(7.0),
            (s.armor / max_hp).clamp(0.0, 1.0),
            CYAN,
        );
    }
    seg_bar(ix, y + ui.s(68.0), iw, ui.s(16.0), frac, col, 28);

    if s.armor > 0.0 {
        draw_rectangle(ix, y + ui.s(88.0), ui.s(7.0), ui.s(7.0), fade(CYAN, 0.9));
        f.text(
            &t.armor,
            ix + ui.s(14.0),
            y + ui.s(96.0),
            ui.s(13.0),
            fade(CYAN, 0.85),
        );
    }
    if s.low_health {
        let a = 0.45 + 0.55 * pulse(6.2);
        let label = i18n::t(Key::LowHealth);
        let wx = right - f.width(label, ui.s(14.0));
        let my = y + ui.s(95.0);
        draw_triangle(
            vec2(wx - ui.s(15.0), my + ui.s(2.0)),
            vec2(wx - ui.s(9.0), my - ui.s(8.0)),
            vec2(wx - ui.s(3.0), my + ui.s(2.0)),
            fade(DANGER, a),
        );
        f.text_r(label, right, y + ui.s(96.0), ui.s(14.0), fade(DANGER, a));
    }
}

fn draw_ammo(f: &Fonts, s: &HudState, t: &mut HudText, ui: &Ui, w: f32, h: f32) {
    let pw = ui.s(268.0);
    let x = w - ui.s(MARGIN) - pw;
    let y = h - ui.s(MARGIN) - ui.s(BLOCK_H);
    panel(x, y, pw, ui.s(BLOCK_H));

    let ix = x + ui.s(PAD);
    let right = x + pw - ui.s(PAD);
    let iw = pw - ui.s(PAD * 2.0);

    f.text_r(
        fit_into(
            &mut t.weapon,
            s.weapon_name,
            ui.s(13.0),
            iw,
            &|text, size| f.width(text, size),
        ),
        right,
        y + ui.s(26.0),
        ui.s(13.0),
        fade(INK, 0.60),
    );

    match s.reload_progress {
        Some(p) => {
            let a = 0.55 + 0.45 * pulse(7.0);
            tracked_r(
                f,
                i18n::t(Key::Reloading),
                right,
                y + ui.s(58.0),
                ui.s(14.0),
                fade(ACCENT, a),
                ui.s(2.0),
            );
            bar(ix, y + ui.s(70.0), iw, ui.s(7.0), p, ACCENT);
        }
        None => {
            let small = t.mag_size.as_str();
            let sw = f.width(small, ui.s(16.0));
            f.text_r(small, right, y + ui.s(74.0), ui.s(16.0), fade(INK, 0.45));

            let nearly_empty =
                s.mag_size > 0 && s.mag as f32 <= (s.mag_size as f32 * 0.25).max(1.0);
            let col = if s.mag == 0 {
                fade(DANGER, 0.55 + 0.45 * pulse(8.0))
            } else if nearly_empty {
                DANGER
            } else {
                INK
            };
            f.text_r(
                &t.mag,
                right - sw - ui.s(8.0),
                y + ui.s(74.0),
                ui.s(46.0),
                col,
            );
        }
    }

    draw_slots(
        f,
        s,
        t,
        ui,
        Slots {
            ix,
            right,
            y: y + ui.s(82.0),
            bh: ui.s(20.0),
        },
    );
}

/// Where the weapon-slot strip sits inside the ammo panel. Grouped so the strip
/// takes one argument instead of four.
#[derive(Clone, Copy)]
struct Slots {
    ix: f32,
    right: f32,
    y: f32,
    bh: f32,
}

fn draw_slots(f: &Fonts, s: &HudState, t: &mut HudText, ui: &Ui, at: Slots) {
    let Slots { ix, right, y, bh } = at;
    let n = s.slots.len();
    if n == 0 {
        return;
    }
    let bw = ui.s(26.0);
    let gap = ui.s(6.0);
    let total = bw * n as f32 + gap * (n as f32 - 1.0);
    let mut active_label = "";

    for (i, (label, active)) in s.slots.iter().enumerate() {
        let x = right - total + i as f32 * (bw + gap);
        if *active {
            draw_rectangle(x, y, bw, bh, fade(ACCENT, 0.90));
            draw_rectangle_lines(x, y, bw, bh, 1.0, fade(WHITE, 0.35));
            active_label = label;
        } else {
            draw_rectangle(x, y, bw, bh, Color::new(0.0, 0.0, 0.0, 0.45));
            draw_rectangle_lines(x, y, bw, bh, 1.0, fade(INK, 0.22));
        }
        let key = t.slot_keys.get(i).map_or("", String::as_str);
        let key_col = if *active {
            Color::new(0.06, 0.05, 0.04, 1.0)
        } else {
            fade(INK, 0.60)
        };
        f.text_c(
            key,
            x + bw * 0.5,
            y + bh * 0.5 + ui.s(6.0),
            ui.s(14.0),
            key_col,
        );
    }

    let room = right - total - ui.s(10.0) - ix;
    if !active_label.is_empty() && room > ui.s(24.0) {
        f.text_r(
            fit_into(
                &mut t.slot_label,
                active_label,
                ui.s(12.0),
                room,
                &|text, size| f.width(text, size),
            ),
            right - total - ui.s(10.0),
            y + bh * 0.5 + ui.s(5.0),
            ui.s(12.0),
            fade(INK, 0.70),
        );
    }
}

fn draw_objective(f: &Fonts, s: &HudState, t: &mut HudText, ui: &Ui) {
    let pw = ui.s(400.0);
    let x = ui.s(MARGIN);
    let y = ui.s(MARGIN);
    panel(x, y, pw, ui.s(BLOCK_H));

    let ix = x + ui.s(PAD);
    let right = x + pw - ui.s(PAD);
    let iw = pw - ui.s(PAD * 2.0);

    // The marker breathes while the mission is live and freezes once it is over.
    let marker = if s.locked { pulse(3.4) } else { 1.0 };
    draw_rectangle(
        ix,
        y + ui.s(18.0),
        ui.s(7.0),
        ui.s(7.0),
        fade(ACCENT, 0.55 + 0.45 * marker),
    );
    tracked(
        f,
        i18n::t(Key::Objective),
        ix + ui.s(15.0),
        y + ui.s(26.0),
        ui.s(12.0),
        fade(INK, 0.55),
        ui.s(2.0),
    );

    let hostiles = t.hostiles.as_str();
    let hw = f.width(hostiles, ui.s(22.0));
    tracked_r(
        f,
        i18n::t(Key::Hostiles),
        right - hw - ui.s(10.0),
        y + ui.s(26.0),
        ui.s(11.0),
        fade(INK, 0.50),
        ui.s(1.6),
    );
    f.text_r(
        hostiles,
        right,
        y + ui.s(30.0),
        ui.s(22.0),
        if s.enemies_left == 0 { CYAN } else { ACCENT },
    );

    let obj_col = if s.locked { INK } else { fade(CYAN, 0.85) };
    f.text(
        fit_into(
            &mut t.objective,
            s.objective,
            ui.s(20.0),
            iw,
            &|text, size| f.width(text, size),
        ),
        ix,
        y + ui.s(60.0),
        ui.s(20.0),
        obj_col,
    );
    draw_rectangle(ix, y + ui.s(72.0), iw, 1.0, fade(INK, 0.14));

    if !s.objective_sub.is_empty() {
        f.text(
            fit_into(
                &mut t.objective_sub,
                s.objective_sub,
                ui.s(13.0),
                iw,
                &|text, size| f.width(text, size),
            ),
            ix,
            y + ui.s(94.0),
            ui.s(13.0),
            fade(INK, 0.55),
        );
    }
}

fn draw_stats(f: &Fonts, t: &HudText, ui: &Ui, w: f32) {
    let pw = ui.s(250.0);
    let x = w - ui.s(MARGIN) - pw;
    let y = ui.s(MARGIN);
    panel(x, y, pw, ui.s(BLOCK_H));

    let ix = x + ui.s(PAD);
    let right = x + pw - ui.s(PAD);
    let rows = [
        (i18n::t(Key::Score), t.score.as_str(), INK),
        (i18n::t(Key::Kills), t.kills.as_str(), INK),
        (i18n::t(Key::Time), t.time.as_str(), CYAN),
    ];

    for (i, (label, value, col)) in rows.iter().enumerate() {
        let yy = y + ui.s(30.0) + i as f32 * ui.s(32.0);
        tracked(f, label, ix, yy, ui.s(12.0), fade(INK, 0.50), ui.s(2.0));
        f.text_r(value, right, yy, ui.s(22.0), *col);
        if i + 1 < rows.len() {
            draw_rectangle(
                ix,
                yy + ui.s(10.0),
                pw - ui.s(PAD * 2.0),
                1.0,
                fade(INK, 0.07),
            );
        }
    }
}

fn draw_prompt(f: &Fonts, t: &mut HudText, text: &str, ui: &Ui, w: f32, h: f32) {
    let cx = w * 0.5;
    let label = fit_into(&mut t.prompt, text, ui.s(17.0), w * 0.5, &|s, size| {
        f.width(s, size)
    });
    if label.is_empty() {
        return;
    }
    let bw = f.width(label, ui.s(17.0)) + ui.s(36.0);
    let bh = ui.s(32.0);
    let y = h * 0.5 - ui.s(104.0);
    let bx = cx - bw * 0.5;
    let by = y - ui.s(23.0);
    let a = 0.35 + 0.65 * pulse(4.6);

    draw_rectangle(bx, by, bw, bh, Color::new(0.05, 0.06, 0.08, 0.78));
    draw_rectangle_lines(bx, by, bw, bh, ui.s(1.5), fade(ACCENT, a));
    corner_marks(
        bx,
        by,
        bw,
        bh,
        ui.s(8.0),
        ui.s(2.0),
        fade(ACCENT, 0.4 + 0.6 * a),
    );
    f.text_c(label, cx, y, ui.s(17.0), INK);
}

fn draw_banner(f: &Fonts, text: &str, alpha: f32, ui: &Ui, w: f32, h: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.002 || text.is_empty() {
        return;
    }
    let cx = w * 0.5;
    let y = h * 0.5 - ui.s(160.0);
    let size = ui.s(52.0);
    let tw = tracked_w(f, text, size, ui.s(8.0));
    let bw = tw + ui.s(96.0);

    draw_rectangle(
        cx - bw * 0.5,
        y - ui.s(52.0),
        bw,
        ui.s(76.0),
        Color::new(0.03, 0.03, 0.04, 0.55 * a),
    );
    // The rules open up as the text fades out.
    let rw = tw + ui.s(40.0 + 80.0 * (1.0 - a));
    draw_rectangle(
        cx - rw * 0.5,
        y - ui.s(54.0),
        rw,
        ui.s(2.0),
        fade(ACCENT, 0.85 * a),
    );
    draw_rectangle(
        cx - rw * 0.5,
        y + ui.s(22.0),
        rw,
        ui.s(2.0),
        fade(ACCENT, 0.85 * a),
    );
    tracked_c(
        f,
        text,
        cx,
        y,
        size,
        fade(mix(INK, ACCENT, 0.55), a),
        ui.s(8.0),
    );
}

/// Red/edge damage flash plus a low-health pulse.
pub fn draw_vignette(hurt: f32, low_health: bool) {
    let w = screen_width();
    let h = screen_height();

    let mut amt = hurt.clamp(0.0, 1.0) * 0.85;
    if low_health {
        amt = amt.max(0.20 + 0.26 * pulse(3.2));
    }
    if amt <= 0.002 {
        return;
    }

    let red = Color::new(0.72, 0.06, 0.05, 1.0);
    draw_rectangle(0.0, 0.0, w, h, Color::new(red.r, red.g, red.b, amt * 0.11));

    // Four edge strips, each built from stacked translucent rectangles that get
    // fainter the further they reach in — no shader needed. Enough steps that
    // the banding stops reading as stripes.
    let depth = (h * 0.30).min(w * 0.42);
    let steps = 96usize;
    let band = depth / steps as f32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let a = amt * 0.92 * (1.0 - t) * (1.0 - t);
        if a <= 0.002 {
            continue;
        }
        let c = Color::new(red.r, red.g, red.b, a);
        let o = band * i as f32;
        // Bands tile exactly: overlapping edges would double-blend into stripes.
        draw_rectangle(0.0, o, w, band, c);
        draw_rectangle(0.0, h - o - band, w, band, c);
        draw_rectangle(o, 0.0, band, h, c);
        draw_rectangle(w - o - band, 0.0, band, h, c);
    }
}

// --- front-end screens ----------------------------------------------------

pub struct MenuItem<'a> {
    pub label: &'a str,
    pub enabled: bool,
}

/// Dim wash, faint scanlines and a corner frame shared by both screens.
fn backdrop(ui: &Ui) {
    let w = screen_width();
    let h = screen_height();
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.03, 0.035, 0.05, 0.88));

    let step = ui.s(4.0).max(2.0);
    let mut y = 0.0;
    while y < h {
        draw_rectangle(0.0, y, w, 1.0, Color::new(0.35, 0.80, 1.0, 0.025));
        y += step;
    }
    let m = ui.s(22.0);
    corner_marks(
        m,
        m,
        w - m * 2.0,
        h - m * 2.0,
        ui.s(54.0),
        ui.s(2.0),
        fade(ACCENT, 0.45),
    );
}

/// Draws a menu and returns the clickable rect for each item as
/// `(x, y, w, h)`, so the caller can hit-test the mouse.
pub fn draw_menu(
    f: &Fonts,
    title: &str,
    subtitle: Option<&str>,
    items: &[MenuItem],
    selected: usize,
) -> Vec<(f32, f32, f32, f32)> {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    backdrop(&ui);

    let cx = w * 0.5;
    let ty = h * 0.20 + ui.s(30.0);
    tracked_c(f, title, cx, ty, ui.s(54.0), INK, ui.s(10.0));

    let mut rule_y = ty + ui.s(26.0);
    if let Some(sub) = subtitle
        && !sub.is_empty()
    {
        f.text_c(sub, cx, ty + ui.s(32.0), ui.s(15.0), fade(INK, 0.55));
        rule_y = ty + ui.s(56.0);
    }
    let rw = (w * 0.34).clamp(ui.s(280.0), ui.s(520.0));
    draw_rectangle(cx - rw * 0.5, rule_y, rw, ui.s(2.0), fade(ACCENT, 0.85));
    draw_rectangle(cx - rw * 0.5, rule_y, rw * 0.16, ui.s(2.0), INK);

    item_list(
        f,
        items,
        selected,
        &ui,
        cx,
        rule_y + ui.s(46.0),
        (w * 0.42).clamp(ui.s(360.0), ui.s(520.0)),
    )
}

/// Full-screen mission result panel.
pub fn draw_result(
    f: &Fonts,
    title: &str,
    success: bool,
    rows: &[(&str, String)],
    items: &[MenuItem],
    selected: usize,
) -> Vec<(f32, f32, f32, f32)> {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    backdrop(&ui);

    let cx = w * 0.5;
    let col = if success { CYAN } else { DANGER };
    draw_rectangle(0.0, 0.0, w, h, fade(col, 0.05));

    let ey = h * 0.11 + ui.s(26.0);
    emblem(cx, ey, col, success, &ui);

    let ty = ey + ui.s(84.0);
    tracked_c(f, title, cx, ty, ui.s(50.0), col, ui.s(10.0));
    let rw = (w * 0.40).clamp(ui.s(320.0), ui.s(560.0));
    draw_rectangle(
        cx - rw * 0.5,
        ty + ui.s(18.0),
        rw,
        ui.s(2.0),
        fade(col, 0.85),
    );

    let tw = (w * 0.42).clamp(ui.s(340.0), ui.s(520.0));
    let tx = cx - tw * 0.5;
    let mut y = ty + ui.s(54.0);
    for (i, (label, value)) in rows.iter().enumerate() {
        if i % 2 == 0 {
            draw_rectangle(tx, y, tw, ui.s(26.0), fade(INK, 0.035));
        }
        f.text(
            label,
            tx + ui.s(12.0),
            y + ui.s(19.0),
            ui.s(15.0),
            fade(INK, 0.60),
        );
        f.text_r(value, tx + tw - ui.s(12.0), y + ui.s(19.0), ui.s(17.0), INK);

        // Dotted leader between the two columns.
        let mut dx = tx + ui.s(12.0) + f.width(label, ui.s(15.0)) + ui.s(12.0);
        let end = tx + tw - ui.s(12.0) - f.width(value, ui.s(17.0)) - ui.s(12.0);
        while dx + ui.s(3.0) < end {
            draw_rectangle(dx, y + ui.s(18.0), ui.s(2.0), 1.0, fade(INK, 0.18));
            dx += ui.s(7.0);
        }
        draw_rectangle(tx, y + ui.s(26.0), tw, 1.0, fade(INK, 0.08));
        y += ui.s(30.0);
    }

    item_list(f, items, selected, &ui, cx, y + ui.s(26.0), tw)
}

/// Success check or failure cross inside a diamond.
fn emblem(cx: f32, cy: f32, col: Color, success: bool, ui: &Ui) {
    let r = ui.s(26.0);
    draw_triangle(
        vec2(cx, cy - r),
        vec2(cx + r, cy),
        vec2(cx, cy + r),
        fade(col, 0.14),
    );
    draw_triangle(
        vec2(cx, cy - r),
        vec2(cx, cy + r),
        vec2(cx - r, cy),
        fade(col, 0.14),
    );
    let pts = [
        vec2(cx, cy - r),
        vec2(cx + r, cy),
        vec2(cx, cy + r),
        vec2(cx - r, cy),
    ];
    for i in 0..4 {
        let a = pts[i];
        let b = pts[(i + 1) % 4];
        draw_line(a.x, a.y, b.x, b.y, ui.s(2.0), fade(col, 0.9));
    }
    let t = ui.s(3.0);
    if success {
        draw_line(
            cx - ui.s(11.0),
            cy + ui.s(1.0),
            cx - ui.s(3.0),
            cy + ui.s(10.0),
            t,
            col,
        );
        draw_line(
            cx - ui.s(3.0),
            cy + ui.s(10.0),
            cx + ui.s(12.0),
            cy - ui.s(10.0),
            t,
            col,
        );
    } else {
        draw_line(
            cx - ui.s(9.0),
            cy - ui.s(9.0),
            cx + ui.s(9.0),
            cy + ui.s(9.0),
            t,
            col,
        );
        draw_line(
            cx + ui.s(9.0),
            cy - ui.s(9.0),
            cx - ui.s(9.0),
            cy + ui.s(9.0),
            t,
            col,
        );
    }
}

/// One column of menu rows: filled, outlined, numbered and left-aligned.
fn item_list(
    f: &Fonts,
    items: &[MenuItem],
    selected: usize,
    ui: &Ui,
    cx: f32,
    top: f32,
    iw: f32,
) -> Vec<(f32, f32, f32, f32)> {
    let ih = ui.s(50.0);
    let gap = ui.s(10.0);

    let x = cx - iw * 0.5;
    let mut rects = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let y = top + i as f32 * (ih + gap);
        let sel = i == selected;
        // Disabled rows keep their shape but drop to 45% alpha.
        let a = if item.enabled { 1.0 } else { 0.45 };

        let fill = if sel {
            Color::new(0.17, 0.12, 0.08, 0.92)
        } else {
            Color::new(0.06, 0.07, 0.09, 0.80)
        };
        draw_rectangle(x, y, iw, ih, fade(fill, a));
        if sel {
            draw_rectangle(x, y, ui.s(4.0), ih, fade(ACCENT, a));
        }
        draw_rectangle_lines(
            x,
            y,
            iw,
            ih,
            1.0,
            if sel {
                fade(ACCENT, 0.85 * a)
            } else {
                fade(INK, 0.18 * a)
            },
        );

        f.text(
            &format!("{:02}", i + 1),
            x + ui.s(20.0),
            y + ih * 0.5 + ui.s(6.0),
            ui.s(14.0),
            fade(INK, 0.35 * a),
        );
        f.text(
            &fit(f, item.label, ui.s(20.0), iw - ui.s(100.0)),
            x + ui.s(58.0),
            y + ih * 0.5 + ui.s(7.0),
            ui.s(20.0),
            fade(INK, if sel { a } else { 0.72 * a }),
        );
        if sel {
            let mx = x + iw - ui.s(34.0);
            let my = y + ih * 0.5;
            draw_triangle(
                vec2(mx, my - ui.s(7.0)),
                vec2(mx, my + ui.s(7.0)),
                vec2(mx + ui.s(11.0), my),
                fade(ACCENT, a),
            );
        }
        rects.push((x, y, iw, ih));
    }
    rects
}

// --- tab, map, mission and wheel screens ----------------------------------
//
// The screens the player opens by hand rather than ones the game draws every
// frame. They keep the front end's frame language — letter-spaced title over an
// amber rule, corner-marked panels, dotted leaders — and, being modal, dim the
// frame behind them. As everywhere else in this module the lengths below are
// `Ui` units, so the whole set scales with the window.

/// Shown wherever a slot or a list has nothing in it. `Key` has no such entry.
const NONE: &str = "—";

/// `InventoryState` carries raw health and armour, so their bars need the same
/// scales the game plays with (`Player::max_health`, `main`'s `ARMOR_MAX`).
const INV_HEALTH_MAX: f32 = 100.0;
const INV_ARMOR_MAX: f32 = 50.0;

/// A non-finite length can only ever produce a non-finite shape, so it is
/// swapped for a safe value before it reaches a draw call.
fn sane(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

/// Dims everything behind a modal screen.
fn scrim(a: f32) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, sane(a, 0.0).clamp(0.0, 1.0)),
    );
}

/// Letter-spaced screen title with the amber rule the front end draws under
/// every heading. Returns the y of the rule.
fn heading(f: &Fonts, ui: &Ui, title: &str, cx: f32, y: f32, rule_w: f32) -> f32 {
    tracked_c(f, title, cx, y, ui.s(38.0), INK, ui.s(9.0));
    let ry = y + ui.s(20.0);
    let rw = sane(rule_w, 0.0).clamp(0.0, screen_width());
    draw_rectangle(cx - rw * 0.5, ry, rw, ui.s(2.0), fade(ACCENT, 0.85));
    draw_rectangle(cx - rw * 0.5, ry, rw * 0.16, ui.s(2.0), INK);
    ry
}

/// Dotted leader between two x positions.
fn dotted(ui: &Ui, x0: f32, x1: f32, y: f32, a: f32) {
    let step = ui.s(7.0).max(1.0);
    if !x0.is_finite() || !x1.is_finite() || x1 <= x0 {
        return;
    }
    // Bounded by construction, so a runaway span can never spin here.
    for i in 0..((x1 - x0) / step).clamp(0.0, 400.0) as usize {
        draw_rectangle(
            x0 + step * i as f32,
            y,
            ui.s(2.0).max(1.0),
            1.0,
            fade(INK, a),
        );
    }
}

/// One label/value row: label at `x`, value right-aligned to `x + w`, dotted
/// leader filling the gap between them.
#[allow(clippy::too_many_arguments)] // a row of a label/value table
fn kv_row(
    f: &Fonts,
    ui: &Ui,
    x: f32,
    w: f32,
    y: f32,
    h: f32,
    label: &str,
    value: &str,
    lsize: f32,
    vsize: f32,
    vcolor: Color,
) {
    let cy = y + h * 0.5;
    f.text(label, x, cy + lsize * 0.36, lsize, fade(INK, 0.62));
    if value.is_empty() {
        return;
    }
    f.text_r(value, x + w, cy + vsize * 0.36, vsize, vcolor);
    dotted(
        ui,
        x + f.width(label, lsize) + ui.s(10.0),
        x + w - f.width(value, vsize) - ui.s(10.0),
        cy,
        0.16,
    );
}

/// Section header: tracked caps with a rule running out to `right`.
fn section_head(f: &Fonts, ui: &Ui, text: &str, x: f32, right: f32, y: f32, color: Color) {
    let size = ui.s(13.0);
    let tracking = ui.s(2.2);
    let tw = tracked_w(f, text, size, tracking);
    tracked(f, text, x, y, size, fade(color, 0.95), tracking);
    let rx = x + tw + ui.s(12.0);
    if right - rx > ui.s(8.0) {
        draw_rectangle(rx, y - ui.s(5.0), right - rx, 1.0, fade(INK, 0.14));
    }
}

/// Boxed key hint along the bottom of an overlay.
fn hint_line(f: &Fonts, ui: &Ui, text: &str, cx: f32, y: f32) {
    if text.is_empty() {
        return;
    }
    let size = ui.s(13.0);
    let bw = tracked_w(f, text, size, ui.s(2.0)) + ui.s(34.0);
    let bh = ui.s(26.0);
    let bx = cx - bw * 0.5;
    let by = y - ui.s(16.0);
    draw_rectangle(bx, by, bw, bh, Color::new(0.05, 0.06, 0.08, 0.72));
    draw_rectangle_lines(bx, by, bw, bh, 1.0, fade(INK, 0.20));
    corner_marks(bx, by, bw, bh, ui.s(7.0), ui.s(2.0), fade(ACCENT, 0.55));
    tracked_c(f, text, cx, y + ui.s(2.0), size, fade(INK, 0.62), ui.s(2.0));
}

/// Centres a `pw` x `ph` overlay panel on the screen. The width is clipped to
/// the window so the panel never spills sideways; the height is not, so a panel
/// taller than the window keeps its internal spacing and is anchored to the top.
pub(crate) fn panel_fit(w: f32, h: f32, pw: f32, ph: f32) -> (f32, f32, f32, f32) {
    let w = sane(w, 0.0).max(0.0);
    let h = sane(h, 0.0).max(0.0);
    let pw = sane(pw, 0.0).clamp(0.0, w);
    let ph = sane(ph, 0.0).max(0.0);
    (((w - pw) * 0.5).max(0.0), ((h - ph) * 0.5).max(0.0), pw, ph)
}

// --- map geometry ---------------------------------------------------------
//
// Plain maths, kept apart from the drawing so the screen above and anything
// that wants to place a marker on it share one definition of the projection.

/// Pixels per metre for an arena of half-extent `bounds` drawn `side` wide.
/// Zero or nonsense extents fall back to a unit arena, so this never divides by
/// zero and never returns a non-finite value.
pub(crate) fn map_ppu(bounds: f32, side: f32) -> f32 {
    let b = sane(bounds, 1.0);
    let b = if b.abs() > 1.0e-3 { b.abs() } else { 1.0 };
    (sane(side, 0.0).max(0.0) / (b * 2.0)).min(1.0e6)
}

/// Screen position of a world point on the XZ plane. North is up, so the
/// arena's +z ("south") is drawn downwards and +x to the right.
pub(crate) fn map_point(p: Vec2, centre: Vec2, ppu: f32) -> Vec2 {
    let k = sane(ppu, 0.0);
    vec2(centre.x + p.x * k, centre.y + p.y * k)
}

/// Screen direction the player faces at `yaw`; matches `Player::forward`
/// (`(-sin yaw, 0, -cos yaw)`), so yaw 0 points up the map.
pub(crate) fn map_face(yaw: f32) -> Vec2 {
    let y = sane(yaw, 0.0);
    vec2(-y.sin(), -y.cos())
}

/// Screen rect of one arena footprint, clipped to the map frame `(x, y, side)`
/// so nothing is drawn outside it. Non-finite input collapses to a zeroed rect,
/// which callers skip.
pub(crate) fn map_rect(
    min: Vec2,
    max: Vec2,
    centre: Vec2,
    ppu: f32,
    frame: (f32, f32, f32),
) -> (f32, f32, f32, f32) {
    if !(min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite()) {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let a = map_point(min, centre, ppu);
    let b = map_point(max, centre, ppu);
    if !(a.x.is_finite() && a.y.is_finite() && b.x.is_finite() && b.y.is_finite()) {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let (fx, fy, side) = frame;
    let wx = clamp_span(a.x, b.x, fx, fx + side, 1.0);
    let wy = clamp_span(a.y, b.y, fy, fy + side, 1.0);
    (wx.0, wy.0, wx.1, wy.1)
}

/// Keeps a span inside `lo..=hi` while leaving it at least `min_w` wide: a span
/// clipped away to nothing at the far edge slides back inside instead. Never
/// panics, whatever the inputs are.
fn clamp_span(a: f32, b: f32, lo: f32, hi: f32, min_w: f32) -> (f32, f32) {
    let lo_v = sane(lo, 0.0);
    let hi_v = sane(hi, lo_v).max(lo_v);
    let room = hi_v - lo_v;
    let min_w = sane(min_w, 0.0).clamp(0.0, room);
    let x0 = a.min(b).clamp(lo_v, hi_v);
    let x1 = a.max(b).clamp(lo_v, hi_v);
    let span = (x1 - x0).min(room);
    let (x, w) = if span < min_w {
        ((x1 - min_w).max(lo_v), min_w)
    } else {
        (x0, span)
    };
    (x.min(hi_v - w), w)
}

/// A round metre interval that leaves an arena no more than six cells wide.
fn nice_step(bounds: f32) -> f32 {
    const STEPS: [f32; 8] = [1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0];
    let b = sane(bounds, 1.0).abs().max(1.0e-3);
    for s in STEPS {
        if b / s <= 6.0 {
            return s;
        }
    }
    500.0
}

// --- wheel geometry -------------------------------------------------------

/// Outline of one annular sector: out along `a0`, around the outer arc, back
/// along `a1` and around the inner arc. The sector is star-shaped about
/// `centre`, so a triangle fan from the centre fills it. Angles are radians
/// measured the way macroquad draws them — 0 at 3 o'clock, growing downwards —
/// and the polygon closes implicitly, the last point joining the first.
pub(crate) fn ring_sector(centre: Vec2, r0: f32, r1: f32, a0: f32, a1: f32) -> Vec<Vec2> {
    let r0 = sane(r0, 0.0).max(0.0);
    let r1 = sane(r1, r0).max(0.0);
    let start = sane(a0, 0.0);
    let span = sane(a1, 0.0) - start;
    if !span.is_finite() {
        return Vec::new();
    }
    let steps = ((span.abs() / (std::f32::consts::TAU / 48.0)).ceil() as usize).clamp(1, 96);
    let at = |k: usize, r: f32| {
        let a = start + span * (k as f32 / steps as f32);
        centre + vec2(a.cos(), a.sin()) * r
    };
    let mut pts = Vec::with_capacity(steps * 2 + 2);
    for k in 0..=steps {
        pts.push(at(k, r1));
    }
    for k in (0..=steps).rev() {
        pts.push(at(k, r0));
    }
    pts
}

/// Fills one wheel sector and outlines it. `t` is the outline thickness.
#[allow(clippy::too_many_arguments)] // one sector of the radial menu
fn wheel_sector(
    centre: Vec2,
    r0: f32,
    r1: f32,
    a0: f32,
    a1: f32,
    fill: Color,
    edge: Color,
    t: f32,
) {
    let pts = ring_sector(centre, r0, r1, a0, a1);
    let mut rest = pts.iter().copied();
    let first = match rest.next() {
        Some(p) => p,
        None => return,
    };
    let mut prev = first;
    for p in rest {
        draw_triangle(centre, prev, p, fill);
        draw_line(prev.x, prev.y, p.x, p.y, t, edge);
        prev = p;
    }
    draw_line(prev.x, prev.y, first.x, first.y, t, edge);
}

// --- tab: inventory -------------------------------------------------------

/// Tab — weapons carried and consumables held.
pub struct InventoryState<'a> {
    pub primary: Option<&'a str>,
    pub secondary: Option<&'a str>,
    pub pistol: Option<&'a str>,
    pub melee: Option<&'a str>,
    pub active_slot: usize,
    pub grenades: &'a [(String, u32)],
    pub meds: &'a [(String, u32)],
    pub armour: f32,
    pub health: f32,
}

pub fn draw_inventory(f: &Fonts, s: &InventoryState) {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    scrim(0.60);

    let slots = [
        (i18n::t(Key::SlotPrimary), s.primary),
        (i18n::t(Key::SlotSecondary), s.secondary),
        (i18n::t(Key::SlotPistol), s.pistol),
        (i18n::t(Key::SlotMelee), s.melee),
    ];

    // Vertical budget in units: title block, four slot rows, the two lists, the
    // health strip and the footer. The lists take what is left over and
    // truncate, rather than pushing the strip off the bottom of a short window.
    let pad = ui.s(20.0);
    let slot_h = ui.s(40.0);
    let slot_gap = ui.s(8.0);
    let row_h = ui.s(26.0);
    let head_h = ui.s(80.0);
    let list_gap = ui.s(40.0);
    let strip_gap = ui.s(14.0);
    let strip_h = ui.s(70.0);
    let foot_h = ui.s(52.0);
    let slots_h = slot_h * slots.len() as f32 + slot_gap * (slots.len() as f32 - 1.0);
    let fixed = head_h + slots_h + list_gap + strip_gap + strip_h + foot_h;
    let entries = s.grenades.len().max(s.meds.len());

    let room = (h - ui.s(MARGIN) * 2.0 - pad - fixed).max(0.0);
    let shown = ((room / row_h).floor() as usize).min(entries);

    let want_w = (w * 0.68).clamp(ui.s(380.0), ui.s(760.0));
    let want_h = pad + fixed + shown as f32 * row_h;
    let (px, py, pw, ph) = panel_fit(w, h, want_w, want_h);
    panel(px, py, pw, ph);

    let ix = px + pad;
    let iw = (pw - pad * 2.0).max(0.0);
    let right = px + pw - pad;

    let rule_y = heading(
        f,
        &ui,
        i18n::t(Key::Inventory),
        px + pw * 0.5,
        py + pad + ui.s(38.0),
        pw * 0.44,
    );

    let mut y = rule_y + ui.s(22.0);
    for (i, (name, weapon)) in slots.iter().enumerate() {
        if i > 0 {
            y += slot_gap;
        }
        slot_row(
            f,
            &ui,
            s.active_slot == i,
            name,
            *weapon,
            ix,
            right,
            y,
            slot_h,
        );
        y += slot_h;
    }

    // --- grenades | medical ------------------------------------------------
    let list_top = y + ui.s(16.0);
    let list_bottom = list_top + ui.s(24.0) + shown as f32 * row_h;
    let col_gap = ui.s(30.0);
    let colw = ((iw - col_gap) * 0.5).max(0.0);
    list_column(
        f,
        &ui,
        ix,
        colw,
        list_top,
        list_bottom,
        row_h,
        i18n::t(Key::Equipment),
        s.grenades,
        ACCENT,
    );
    list_column(
        f,
        &ui,
        ix + colw + col_gap,
        colw,
        list_top,
        list_bottom,
        row_h,
        i18n::t(Key::Consumables),
        s.meds,
        CYAN,
    );

    // --- health and armour strip -------------------------------------------
    let strip_top = list_bottom + strip_gap;
    draw_rectangle(ix, strip_top, iw, 1.0, fade(INK, 0.14));

    let hp = sane((s.health / INV_HEALTH_MAX).clamp(0.0, 1.0), 0.0);
    let hp_col = health_color(hp);
    tracked(
        f,
        i18n::t(Key::Health),
        ix,
        strip_top + ui.s(21.0),
        ui.s(12.0),
        fade(INK, 0.55),
        ui.s(2.0),
    );
    f.text_r(
        &format!("{:.0}", s.health.max(0.0).ceil()),
        right,
        strip_top + ui.s(26.0),
        ui.s(26.0),
        hp_col,
    );
    seg_bar(ix, strip_top + ui.s(34.0), iw, ui.s(16.0), hp, hp_col, 28);

    // Armour is a thin cyan strip under it, as in the in-game readout.
    let af = sane((s.armour / INV_ARMOR_MAX).clamp(0.0, 1.0), 0.0);
    draw_rectangle(
        ix,
        strip_top + ui.s(56.0),
        ui.s(8.0),
        ui.s(8.0),
        fade(CYAN, 0.90),
    );
    f.text(
        &format!("{:.0}", s.armour.max(0.0).ceil()),
        ix + ui.s(15.0),
        strip_top + ui.s(64.0),
        ui.s(13.0),
        fade(CYAN, 0.85),
    );
    let bx = ix + ui.s(58.0);
    bar(
        bx,
        strip_top + ui.s(58.0),
        (right - bx).max(0.0),
        ui.s(7.0),
        af,
        CYAN,
    );

    hint_line(
        f,
        &ui,
        i18n::t(Key::HintClose),
        px + pw * 0.5,
        py + ph - ui.s(26.0),
    );
}

/// One weapon line: the slot name, what is in it, and the amber bar that marks
/// the active slot.
#[allow(clippy::too_many_arguments)] // one weapon slot row
fn slot_row(
    f: &Fonts,
    ui: &Ui,
    active: bool,
    slot: &str,
    weapon: Option<&str>,
    ix: f32,
    right: f32,
    y: f32,
    h: f32,
) {
    let fill = if active {
        Color::new(0.17, 0.12, 0.08, 0.92)
    } else {
        Color::new(0.06, 0.07, 0.09, 0.62)
    };
    draw_rectangle(ix, y, right - ix, h, fill);
    if active {
        draw_rectangle(ix, y, ui.s(4.0), h, ACCENT);
    }
    draw_rectangle_lines(
        ix,
        y,
        right - ix,
        h,
        1.0,
        if active {
            fade(ACCENT, 0.85)
        } else {
            fade(INK, 0.16)
        },
    );
    tracked(
        f,
        slot,
        ix + ui.s(20.0),
        y + h * 0.5 + ui.s(5.0),
        ui.s(13.0),
        if active {
            fade(ACCENT, 0.95)
        } else {
            fade(INK, 0.45)
        },
        ui.s(2.0),
    );

    let value = match weapon {
        Some(v) if !v.is_empty() => v,
        _ => NONE,
    };
    let vx = ix + ui.s(168.0);
    f.text(
        &fit(f, value, ui.s(19.0), (right - ui.s(16.0) - vx).max(0.0)),
        vx,
        y + h * 0.5 + ui.s(7.0),
        ui.s(19.0),
        if active { INK } else { fade(INK, 0.70) },
    );
}

/// One of the inventory's two lists: header, then a row per entry with its
/// count. Rows that would fall past `bottom` are dropped and marked `..`.
#[allow(clippy::too_many_arguments)] // one column of a name/count list
fn list_column(
    f: &Fonts,
    ui: &Ui,
    x: f32,
    w: f32,
    top: f32,
    bottom: f32,
    row_h: f32,
    header: &str,
    items: &[(String, u32)],
    accent: Color,
) {
    section_head(
        f,
        ui,
        header,
        x,
        x + w,
        top + ui.s(6.0),
        mix(INK, accent, 0.55),
    );

    let mut y = top + ui.s(20.0);
    if items.is_empty() {
        kv_row(
            f,
            ui,
            x,
            w,
            y,
            row_h,
            NONE,
            "",
            ui.s(15.0),
            ui.s(15.0),
            fade(INK, 0.45),
        );
        return;
    }

    let mut shown = 0usize;
    for (name, count) in items {
        if y + row_h > bottom {
            break;
        }
        kv_row(
            f,
            ui,
            x,
            w,
            y,
            row_h,
            &fit(f, name, ui.s(14.0), w * 0.66),
            &count.to_string(),
            ui.s(14.0),
            ui.s(16.0),
            accent,
        );
        draw_rectangle(x, y + row_h, w, 1.0, fade(INK, 0.06));
        y += row_h;
        shown += 1;
    }
    if shown < items.len() {
        f.text(
            "..",
            x,
            y + row_h * 0.5 + ui.s(4.0),
            ui.s(14.0),
            fade(INK, 0.45),
        );
    }
}

// --- M: arena map ---------------------------------------------------------

/// A dot on the map: a world position, its colour, its radius in layout units
/// and an optional single-character tag.
pub struct MapMarker {
    pub pos: Vec2,
    pub color: Color,
    pub size: f32,
    pub label: Option<char>,
}

/// M — top-down map of the arena.
pub fn draw_map(
    f: &Fonts,
    boxes: &[(Vec2, Vec2)],
    bounds: f32,
    player: (Vec2, f32),
    markers: &[MapMarker],
) {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    scrim(0.60);

    let b = sane(bounds, 1.0);
    let b = if b.abs() > 1.0e-3 { b.abs() } else { 1.0 };

    // The square arena frame everything else on this screen hangs off.
    let head_h = ui.s(108.0);
    let foot_h = ui.s(104.0);
    let margin = ui.s(MARGIN + 12.0);
    let avail_h = (h - head_h - foot_h).max(ui.s(24.0));
    let side = (w - margin * 2.0).min(avail_h).max(ui.s(24.0));
    let frame_x = (w - side) * 0.5;
    let frame_y = head_h + (avail_h - side) * 0.5;
    let centre = vec2(frame_x + side * 0.5, frame_y + side * 0.5);
    let ppu = map_ppu(b, side);

    heading(
        f,
        &ui,
        i18n::t(Key::MapScreen),
        w * 0.5,
        ui.s(MARGIN) + ui.s(38.0),
        ui.s(320.0),
    );

    // Arena floor, then a grid at a round metre interval.
    panel(frame_x, frame_y, side, side);
    let step = nice_step(b);
    let cells = (b / step).floor().clamp(0.0, 12.0) as i32;
    for k in -cells..=cells {
        let d = k as f32 * step * ppu;
        if d.abs() > side * 0.5 {
            continue;
        }
        let a = if k == 0 { 0.15 } else { 0.06 };
        draw_rectangle(centre.x + d, frame_y + 1.0, 1.0, side - 2.0, fade(INK, a));
        draw_rectangle(frame_x + 1.0, centre.y + d, side - 2.0, 1.0, fade(INK, a));
    }

    // Footprints.
    for (min, max) in boxes {
        let (x, y, bw, bh) = map_rect(*min, *max, centre, ppu, (frame_x, frame_y, side));
        if bw <= 0.0 || bh <= 0.0 {
            continue;
        }
        draw_rectangle(x, y, bw, bh, Color::new(0.20, 0.22, 0.27, 0.92));
        draw_rectangle_lines(x, y, bw, bh, 1.0, fade(INK, 0.22));
    }

    // Player: an amber triangle pointing the way the camera looks.
    let dot = map_point(player.0, centre, ppu);
    let r = ui.s(11.0);
    let pc = vec2(
        dot.x
            .clamp(frame_x + r, (frame_x + side - r).max(frame_x + r)),
        dot.y
            .clamp(frame_y + r, (frame_y + side - r).max(frame_y + r)),
    );
    let face = map_face(player.1);
    let side_v = vec2(-face.y, face.x);
    let tip = pc + face * r;
    let tail_l = pc - face * (r * 0.72) + side_v * (r * 0.62);
    let tail_r = pc - face * (r * 0.72) - side_v * (r * 0.62);
    draw_triangle(tip, tail_l, tail_r, ACCENT);
    draw_triangle_lines(
        tip,
        tail_l,
        tail_r,
        ui.s(1.5),
        Color::new(0.03, 0.03, 0.04, 0.85),
    );

    // Markers: a dot, its outline and an optional one-character tag.
    for m in markers {
        let q = map_point(m.pos, centre, ppu);
        let mr = sane(m.size, ui.s(4.0)).clamp(ui.s(1.5), ui.s(18.0));
        let q = vec2(
            q.x.clamp(frame_x + mr, (frame_x + side - mr).max(frame_x + mr)),
            q.y.clamp(frame_y + mr, (frame_y + side - mr).max(frame_y + mr)),
        );
        draw_circle(q.x, q.y, mr + 1.5, Color::new(0.02, 0.02, 0.03, 0.75));
        draw_circle(q.x, q.y, mr, m.color);
        if let Some(c) = m.label {
            let tag = c.to_string();
            let size = ui.s(14.0);
            f.text_c(
                &tag,
                q.x,
                (q.y - mr - ui.s(3.0)).max(frame_y + size),
                size,
                INK,
            );
        }
    }

    // North is up, marked without a legend.
    let nx = frame_x + side - ui.s(22.0);
    let ny = frame_y + ui.s(26.0);
    draw_line(
        nx,
        ny + ui.s(14.0),
        nx,
        ny - ui.s(4.0),
        ui.s(1.5),
        fade(INK, 0.55),
    );
    draw_triangle(
        vec2(nx, ny - ui.s(14.0)),
        vec2(nx - ui.s(6.0), ny - ui.s(4.0)),
        vec2(nx + ui.s(6.0), ny - ui.s(4.0)),
        fade(INK, 0.55),
    );

    // Scale bar: a round number of metres, with the extent it came from.
    let bar_y = frame_y + side + ui.s(34.0);
    let cap = (side * 0.42).max(0.0);
    let bar_len = (step * ppu).min(cap).max(ui.s(18.0).min(cap));
    let bar_x = centre.x - bar_len * 0.5;
    draw_line(
        bar_x,
        bar_y,
        bar_x + bar_len,
        bar_y,
        ui.s(2.0),
        fade(INK, 0.75),
    );
    for x in [bar_x, bar_x + bar_len] {
        draw_line(
            x,
            bar_y - ui.s(6.0),
            x,
            bar_y + ui.s(6.0),
            ui.s(2.0),
            fade(INK, 0.75),
        );
    }
    f.text_c(
        &format!("{:.0} m", step),
        centre.x,
        bar_y - ui.s(10.0),
        ui.s(13.0),
        fade(INK, 0.60),
    );
    f.text(
        &format!("{:.0} x {:.0} m", b * 2.0, b * 2.0),
        frame_x,
        bar_y + ui.s(5.0),
        ui.s(12.0),
        fade(INK, 0.45),
    );

    hint_line(f, &ui, i18n::t(Key::HintClose), w * 0.5, h - ui.s(24.0));
}

// --- J: mission panel -----------------------------------------------------

/// One row of a panel section.
pub struct PanelRow<'a> {
    pub label: &'a str,
    pub value: String,
}

/// J — mission panel: titled sections of label/value rows.
pub fn draw_panel(f: &Fonts, title: &str, sections: &[(&str, Vec<PanelRow>)], footer: &str) {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    scrim(0.60);

    let pad = ui.s(22.0);
    let head_h = ui.s(84.0);
    let foot_h = ui.s(54.0);
    let sec_head_h = ui.s(34.0);
    let sec_gap = ui.s(16.0);

    // Rows share out whatever height is left, down to a readable minimum, and
    // the rest are dropped on the `..` marker rather than drawn off-panel.
    let rows_total: usize = sections.iter().map(|(_, rows)| rows.len()).sum();
    let secs = sections.len() as f32;
    let fixed = pad * 2.0
        + head_h
        + foot_h
        + secs * sec_head_h
        + if sections.is_empty() {
            0.0
        } else {
            (secs - 1.0) * sec_gap
        };
    let room = (h - ui.s(MARGIN) * 2.0 - fixed).max(0.0);
    let row_h = if rows_total > 0 {
        (room / rows_total as f32).clamp(ui.s(15.0), ui.s(30.0))
    } else {
        ui.s(30.0)
    };

    let want_w = (w * 0.62).clamp(ui.s(400.0), ui.s(720.0));
    let want_h = fixed + rows_total as f32 * row_h;
    let (px, py, pw, ph) = panel_fit(w, h, want_w, want_h);
    panel(px, py, pw, ph);

    let ix = px + pad;
    let iw = (pw - pad * 2.0).max(0.0);
    let right = px + pw - pad;
    heading(
        f,
        &ui,
        title,
        px + pw * 0.5,
        py + pad + ui.s(38.0),
        pw * 0.44,
    );

    let bottom = py + ph - foot_h;
    let mut y = py + pad + head_h;

    'sections: for (si, (header, rows)) in sections.iter().enumerate() {
        if si > 0 {
            y += sec_gap;
        }
        if y + sec_head_h > bottom {
            break;
        }
        section_head(
            f,
            &ui,
            header,
            ix,
            right,
            y + ui.s(14.0),
            mix(INK, ACCENT, 0.55),
        );
        y += sec_head_h;

        if rows.is_empty() {
            if y + row_h <= bottom {
                kv_row(
                    f,
                    &ui,
                    ix + ui.s(10.0),
                    iw - ui.s(20.0),
                    y,
                    row_h,
                    NONE,
                    "",
                    ui.s(14.0),
                    ui.s(14.0),
                    fade(INK, 0.45),
                );
                y += row_h;
            }
            continue;
        }

        for (ri, row) in rows.iter().enumerate() {
            if y + row_h > bottom {
                f.text(
                    "..",
                    ix + ui.s(12.0),
                    y + row_h * 0.5,
                    ui.s(13.0),
                    fade(INK, 0.40),
                );
                break 'sections;
            }
            if ri % 2 == 0 {
                draw_rectangle(ix, y, iw, row_h, fade(INK, 0.035));
            }
            kv_row(
                f,
                &ui,
                ix + ui.s(10.0),
                iw - ui.s(20.0),
                y,
                row_h,
                row.label,
                &row.value,
                ui.s(14.0),
                ui.s(16.0),
                INK,
            );
            draw_rectangle(ix, y + row_h, iw, 1.0, fade(INK, 0.06));
            y += row_h;
        }
    }

    hint_line(f, &ui, footer, px + pw * 0.5, py + ph - ui.s(26.0));
}

// --- radial selectors -----------------------------------------------------

/// Radial selector, drawn centred on `centre`. `items` are (label, count,
/// usable); `selected` may be `usize::MAX` for "nothing highlighted".
pub fn draw_wheel(
    f: &Fonts,
    title: &str,
    items: &[(String, u32, bool)],
    centre: Vec2,
    radius: f32,
    selected: usize,
) {
    let ui = Ui::new();
    scrim(0.45);

    let outer = radius.max(0.0).min(ui.s(2000.0));
    let inner = outer * 0.44;
    let disc = (inner * 0.94).max(ui.s(26.0));
    // Labels go round the ring only when there is room between the disc and the
    // outer edge; a selector too small for that names the highlighted entry in
    // the disc instead of stacking unreadable text on top of itself.
    let mid_r = (inner + outer) * 0.5;
    let ring_labels = mid_r - disc > ui.s(18.0);
    let mut picked: Option<&(String, u32, bool)> = None;

    if outer > ui.s(8.0) {
        if items.is_empty() {
            // An empty selector still shows its footprint, so the screen does
            // not read as broken when there is nothing to pick.
            draw_circle_lines(centre.x, centre.y, outer, ui.s(1.5), fade(INK, 0.20));
            draw_circle_lines(centre.x, centre.y, inner, ui.s(1.0), fade(INK, 0.14));
        } else {
            let span = std::f32::consts::TAU / items.len() as f32;
            for (i, item) in items.iter().enumerate() {
                let (label, count, usable) = item;
                let a0 = -std::f32::consts::FRAC_PI_2 + span * i as f32;
                let sel = i == selected;
                // Unusable entries keep their sector but fade it out.
                let k = if *usable { 1.0 } else { 0.45 };
                let fill = if sel {
                    fade(Color::new(0.30, 0.16, 0.06, 0.90), k)
                } else {
                    fade(Color::new(0.06, 0.07, 0.09, 0.70), k)
                };
                let edge = if sel {
                    fade(ACCENT, 0.95)
                } else {
                    fade(INK, 0.20 * k)
                };
                wheel_sector(centre, inner, outer, a0, a0 + span, fill, edge, ui.s(1.5));

                // One item fills the whole ring, so its label belongs at the
                // top rather than half a turn round the back.
                let half = if items.len() == 1 { 0.0 } else { span * 0.5 };
                let mid = if items.len() == 1 {
                    -std::f32::consts::FRAC_PI_2
                } else {
                    a0 + half
                };
                if sel && !ring_labels {
                    picked = Some(item);
                }
                let chord = if items.len() == 1 {
                    outer * 1.1
                } else {
                    (2.0 * mid_r * sane(half.sin(), 0.0)).abs() * 0.92
                };
                let size = ui.s(16.0).min(chord * 0.24).max(ui.s(9.0));
                if !ring_labels {
                    continue;
                }

                let lx = centre.x + mid.cos() * mid_r;
                let ly = centre.y + mid.sin() * mid_r;
                f.text_c(
                    &fit(f, label, size, chord),
                    lx,
                    ly + size * 0.36,
                    size,
                    if sel { INK } else { fade(INK, 0.45 + 0.35 * k) },
                );

                let csize = (size * 0.82).max(ui.s(9.0));
                let ccol = if *count == 0 {
                    fade(DANGER, 0.85 * k)
                } else if sel {
                    ACCENT
                } else {
                    fade(CYAN, 0.80 * k)
                };
                f.text_c(
                    &count.to_string(),
                    lx,
                    ly + size * 0.36 + size * 1.35,
                    csize,
                    ccol,
                );

                if sel {
                    let tx = centre.x + mid.cos() * (outer + ui.s(7.0));
                    let ty = centre.y + mid.sin() * (outer + ui.s(7.0));
                    draw_circle(tx, ty, ui.s(4.0), ACCENT);
                }
            }
            draw_circle_lines(centre.x, centre.y, outer, ui.s(1.5), fade(INK, 0.22));
            draw_circle_lines(centre.x, centre.y, inner, ui.s(1.0), fade(INK, 0.16));
        }
    }

    // The centre disc goes on last so no sector can cover the title.
    draw_circle(
        centre.x,
        centre.y,
        disc + ui.s(2.0),
        Color::new(0.02, 0.025, 0.03, 0.94),
    );
    draw_circle_lines(centre.x, centre.y, disc, ui.s(1.5), fade(INK, 0.25));

    // Shrink the title until it fits the disc, then clamp it to one line.
    let room = (disc * 1.8).max(ui.s(16.0));
    let mut size = ui.s(15.0);
    while size > ui.s(9.0) && tracked_w(f, title, size, ui.s(1.6)) > room {
        size -= ui.s(1.0);
    }
    let label = fit(f, title, size, room);
    let title_y = if picked.is_some() {
        centre.y - size * 0.30
    } else {
        centre.y + size * 0.36
    };
    tracked_c(
        f,
        &label,
        centre.x,
        title_y,
        size,
        fade(ACCENT, 0.95),
        ui.s(1.6),
    );

    // Small ring: the highlighted entry, named where it can be read.
    if let Some((label, count, usable)) = picked {
        let k = if *usable { 1.0 } else { 0.45 };
        let lsize = (size * 0.92).max(ui.s(9.0));
        f.text_c(
            &fit(f, label, lsize, room),
            centre.x,
            centre.y + size * 0.85,
            lsize,
            fade(INK, 0.80 * k),
        );
        f.text_c(
            &count.to_string(),
            centre.x,
            centre.y + size * 0.85 + lsize * 1.25,
            (lsize * 0.82).max(ui.s(9.0)),
            if *count == 0 {
                fade(DANGER, 0.85 * k)
            } else {
                ACCENT
            },
        );
    }
}
