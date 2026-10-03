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
//! then the combat feedback (`draw_kill_flash`, `draw_hurt_arcs`,
//! `draw_damage_numbers`), then `draw_hud`, or `draw_menu` / `draw_result` for
//! the front end.

use crate::i18n::{self, Key};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::fmt::{self, Write as _};

// --- palette --------------------------------------------------------------

/// Near-white base ink. Everything unaccented is drawn from this.
const INK: Color = Color::new(0.88, 0.90, 0.93, 1.0);
/// Translucent panel fill, dark enough to keep text legible over the 3D pass.
const PANEL: Color = Color::new(0.05, 0.06, 0.08, 0.72);
/// The accent a menu, and any arena that does not name one, falls back to.
const DEFAULT_ACCENT: Color = Color::new(0.85, 0.45, 0.12, 1.0);

thread_local! {
    /// The current arena's accent, set once a frame before anything is drawn.
    /// A `Cell` in const-initialised thread-local storage, because the HUD is
    /// drawn from a dozen functions that would otherwise all have to take it as
    /// a parameter, and macroquad only ever draws from one thread.
    static ACCENT: std::cell::Cell<Color> = const { std::cell::Cell::new(DEFAULT_ACCENT) };
}

/// Point the HUD at the arena's accent colour. Called once a frame, before the
/// panels are drawn: amber panels floating over a violet arena read as two
/// different games.
pub fn set_accent(c: Color) {
    ACCENT.with(|a| a.set(c));
}

fn accent() -> Color {
    ACCENT.with(std::cell::Cell::get)
}

/// Scale factor for anything drawn outside this module's own layout. The sanity
/// meter predates `Ui` and was the one readout that did not follow the window,
/// so on a 4K screen it came out a third the size of everything around it.
pub fn ui_scale() -> f32 {
    Ui::new().scale
}

/// The sanity meter's colour. It is the one place the game says "madness" in
/// colour rather than in words, so it keeps a violet of its own — but its low
/// state uses the shared danger red rather than the one-off magenta that
/// appeared nowhere else in the game.
pub fn sanity_colour(ratio: f32) -> Color {
    const MADNESS: Color = Color::new(0.52, 0.38, 0.82, 1.0);
    if ratio < 0.35 { DANGER } else { MADNESS }
}

/// The colour a headshot is shown in: the arena accent pulled towards white, so
/// it stays legible on every arena palette while still belonging to the one the
/// player is standing in. Shared, because a headshot that changes colour between
/// the crosshair and the damage readout is two different pieces of news.
fn head_colour() -> Color {
    mix(accent(), WHITE, 0.30)
}
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
    corner_marks(x, y, w, h, 11.0, 2.0, fade(accent(), 0.85));
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
        mix(accent(), INK, (frac - 0.55) / 0.45)
    } else if frac > 0.25 {
        mix(DANGER, accent(), (frac - 0.25) / 0.30)
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
const BUNDLED_FONT: &[u8] = crate::assets::bytes(crate::assets::Asset::UiFont);

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
    /// The wave in progress and how many the mission has (both one-based on
    /// the incoming side, as a scoreboard reads).
    pub wave: u32,
    pub waves: u32,
    pub objective: &'a str,
    pub objective_sub: &'a str,
    pub weapon_name: &'a str,
    pub mag: u32,
    pub mag_size: u32,
    /// Rounds carried outside the magazine; `None` for weapons that do not
    /// feed from one.
    pub reserve: Option<u32>,
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
    reserve: String,
    slot_keys: Vec<String>,
    hostiles: String,
    wave: String,
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
        put(&mut self.wave, format_args!("{} / {}", s.wave, s.waves));
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
    let head_col = head_colour();
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
                fade(accent(), a),
                ui.s(2.0),
            );
            bar(ix, y + ui.s(70.0), iw, ui.s(7.0), p, accent());
        }
        None => {
            let small = t.mag_size.as_str();
            let sw = f.width(small, ui.s(16.0));
            f.text_r(small, right, y + ui.s(74.0), ui.s(16.0), fade(INK, 0.45));

            // What is left outside the magazine, on the left of the same line:
            // the number a player checks before deciding to spray.
            if let Some(rounds) = s.reserve {
                put(
                    &mut t.reserve,
                    format_args!("{} {}", i18n::t(Key::Spare), rounds),
                );
                f.text(&t.reserve, ix, y + ui.s(72.0), ui.s(15.0), fade(INK, 0.45));
            }

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
            draw_rectangle(x, y, bw, bh, fade(accent(), 0.90));
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
        fade(accent(), 0.55 + 0.45 * marker),
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
        if s.enemies_left == 0 { CYAN } else { accent() },
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
    // One row taller than the objective panel: the wave line joined the
    // score, kills and clock (reference §19.4-6 — the HUD has to say where
    // in the mission you are, and how strong the mission's hostiles are).
    panel(x, y, pw, ui.s(BLOCK_H + 32.0));

    let ix = x + ui.s(PAD);
    let right = x + pw - ui.s(PAD);
    let rows = [
        (i18n::t(Key::Wave), t.wave.as_str(), INK),
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
    draw_rectangle_lines(bx, by, bw, bh, ui.s(1.5), fade(accent(), a));
    corner_marks(
        bx,
        by,
        bw,
        bh,
        ui.s(8.0),
        ui.s(2.0),
        fade(accent(), 0.4 + 0.6 * a),
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
        fade(accent(), 0.85 * a),
    );
    draw_rectangle(
        cx - rw * 0.5,
        y + ui.s(22.0),
        rw,
        ui.s(2.0),
        fade(accent(), 0.85 * a),
    );
    tracked_c(
        f,
        text,
        cx,
        y,
        size,
        fade(mix(INK, accent(), 0.55), a),
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

// --- combat feedback ------------------------------------------------------

/// How far the kill flash reaches in from each edge, as a fraction of the screen
/// height. Well under half, so the middle of the screen — the part the player is
/// aiming through — is never covered.
const KILL_DEPTH: f32 = 0.13;

/// Radius of the damage ring, in layout units. Fixed, because an indicator that
/// lands somewhere different every frame cannot be read at a glance.
const HURT_RADIUS: f32 = 132.0;
/// Thickness of an arc's bright core, in layout units.
const HURT_THICK: f32 = 5.0;
/// Arc length at strength 0 and at strength 1, in radians. A graze still gets a
/// tick big enough to notice; the strongest hit stops short of a quarter turn,
/// so an arc can never read as a hit from two directions at once.
const HURT_SPAN_MIN: f32 = 0.34;
const HURT_SPAN_MAX: f32 = 1.10;
/// How far the dark plate behind an arc reaches past it, in radians.
const HURT_PLATE_PAD: f32 = 0.05;

/// Plate drawn under a feedback glyph. Arcs and floating numbers land on sky,
/// walls and enemies alike, and a dark edge is what keeps them legible on all
/// three without turning them into solid boxes that hide what they are about.
const FEEDBACK_PLATE: Color = Color::new(0.02, 0.025, 0.03, 0.80);

/// Kill confirmation: a warm glow in from every edge plus a bright frame around
/// them.
///
/// `alpha` is `0..=1`; at or below ~0 nothing is drawn, so a decayed value can be
/// passed straight through. Only the outer band of the screen is touched: a kill
/// lands in the middle of a firefight, and the player has to keep shooting
/// through it. It is also deliberately cheap — a few bands and four rectangles,
/// no shader and no full-screen transparency — because it plays over the busiest
/// moment of the frame, and the frame it draws is near-white whatever the
/// arena's accent is, so the pulse reads even where a tint would not.
pub fn draw_kill_flash(alpha: f32) {
    let a = sane(alpha, 0.0).clamp(0.0, 1.0);
    if a <= 0.002 {
        return;
    }
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();

    let warm = mix(INK, accent(), 0.55);
    let depth = h * KILL_DEPTH;
    // Bands get finer as the window grows rather than more numerous with it, so
    // the glow stays smooth at 4K without the draw calls following the pixels up.
    let steps = (depth / ui.s(1.5).max(1.0)).ceil().clamp(6.0, 64.0) as usize;
    let band = depth / steps as f32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let c = fade(warm, a * 0.55 * kill_falloff(t));
        if c.a <= 0.002 {
            continue;
        }
        let o = band * i as f32;
        // Bands tile exactly: overlapping edges would double-blend into stripes.
        draw_rectangle(0.0, o, w, band, c);
        draw_rectangle(0.0, h - o - band, w, band, c);
        draw_rectangle(o, 0.0, band, h, c);
        draw_rectangle(w - o - band, 0.0, band, h, c);
    }

    // The frame carries the read on its own: a bright border is there even where
    // the glow is lost against a bright sky, and unlike a colour it cannot be
    // missed by anyone.
    let t = ui.s(2.5).max(1.0);
    let edge = fade(mix(warm, WHITE, 0.40), a);
    draw_rectangle(0.0, 0.0, w, t, edge);
    draw_rectangle(0.0, h - t, w, t, edge);
    draw_rectangle(0.0, 0.0, t, h, edge);
    draw_rectangle(w - t, 0.0, t, h, edge);
}

/// Glow strength `t` of the way in from the edge of the screen, falling to
/// exactly zero at the inner limit so the glow ends in nothing rather than on a
/// visible line.
fn kill_falloff(t: f32) -> f32 {
    let k = 1.0 - sane(t, 1.0).clamp(0.0, 1.0);
    k * k
}

/// Directional damage indicator: one arc per incoming hit, on a ring around the
/// centre of the screen.
///
/// Each entry is `(bearing, strength)`:
///
/// * `bearing` is where the hit came *from*, in radians, relative to where the
///   player is looking: `0.0` is dead ahead, positive is clockwise seen from
///   above, so `+FRAC_PI_2` is the player's right and `±PI` is directly behind.
///   `player.yaw - util::yaw_of(pos_of_source - player.pos)` is that bearing; it
///   need not be wrapped, because the value only ever reaches `sin`/`cos`.
/// * `strength` is `0..=1` and drives opacity and arc length together: a graze is
///   a short faint tick, a solid hit a long bright fan.
///
/// The reading is positional first — ahead is up, right is right, behind is down
/// — and the arc's length carries the strength, so the indicator survives any
/// colour vision; the ramp from near-white to the damage red only reinforces it.
/// Each arc sits on a dark plate and the strongest still stop short of a quarter
/// turn, so nothing here can be mistaken for the crosshair or for a panel.
pub fn draw_hurt_arcs(arcs: &[(f32, f32)]) {
    if arcs.is_empty() {
        return;
    }
    let ui = Ui::new();
    let centre = vec2(screen_width() * 0.5, screen_height() * 0.5);
    // Fixed radius, pulled in only when the window is too short for it, so the
    // ring never runs off an edge and the eye always knows where to look.
    let r = ui
        .s(HURT_RADIUS)
        .min(screen_width().min(screen_height()) * 0.34);
    let core = ui.s(HURT_THICK).max(1.0);
    let pad = ui.s(1.8).max(1.0);

    // Plates first, then the arcs: several pellets from one shot arrive on the
    // same bearing in the same frame, and an arc drawn over an earlier one must
    // not erase its plate or swallow its core.
    for (bearing, strength) in arcs {
        let s = hurt_strength(*strength);
        if s <= 0.002 {
            continue;
        }
        let (a0, a1) = hurt_arc_angles(*bearing, s);
        arc_band(
            centre,
            r - core * 0.5 - pad,
            r + core * 0.5 + pad,
            a0 - HURT_PLATE_PAD,
            a1 + HURT_PLATE_PAD,
            fade(FEEDBACK_PLATE, 0.5 + 0.5 * s),
        );
    }
    for (bearing, strength) in arcs {
        let s = hurt_strength(*strength);
        if s <= 0.002 {
            continue;
        }
        let (a0, a1) = hurt_arc_angles(*bearing, s);
        let colour = fade(hurt_colour(s), hurt_alpha(s));
        arc_band(centre, r - core * 0.5, r + core * 0.5, a0, a1, colour);

        // A tick past the arc fixes the exact bearing, so a hit from dead astern
        // reads as "straight behind" rather than "somewhere along the bottom".
        // It sits outside the plate, so it carries its own dark under-stroke —
        // the same doubling the crosshair uses to stay legible over the world.
        let d = hurt_dir(*bearing);
        let tip = centre + d * (r + core * 0.5 + ui.s(2.0));
        let tail = centre + d * (r + core * 0.5 + ui.s(8.0));
        draw_line(
            tip.x,
            tip.y,
            tail.x,
            tail.y,
            ui.s(3.2).max(1.5),
            fade(FEEDBACK_PLATE, 0.85),
        );
        draw_line(tip.x, tip.y, tail.x, tail.y, ui.s(1.6).max(1.0), colour);
    }
}

/// Screen direction of a hit bearing: ahead is up the screen, the player's right
/// is to the right of it. Screen y grows downwards, so straight ahead is `-y`.
fn hurt_dir(bearing: f32) -> Vec2 {
    let b = sane(bearing, 0.0);
    vec2(b.sin(), -b.cos())
}

/// A strength that is known to be usable: `0..=1`, with a non-finite value read
/// as no hit at all rather than allowed to reach the vertices as a NaN.
fn hurt_strength(strength: f32) -> f32 {
    sane(strength, 0.0).clamp(0.0, 1.0)
}

/// Arc length in radians for a hit of `strength`.
fn hurt_span(strength: f32) -> f32 {
    HURT_SPAN_MIN + (HURT_SPAN_MAX - HURT_SPAN_MIN) * hurt_strength(strength)
}

/// Start and end screen angles of an arc, in the convention macroquad draws in
/// (0 at 3 o'clock, growing downwards). The span is split evenly either side of
/// the bearing, so an arc stays centred on where it came from however long the
/// hit makes it.
fn hurt_arc_angles(bearing: f32, strength: f32) -> (f32, f32) {
    let middle = sane(bearing, 0.0) - std::f32::consts::FRAC_PI_2;
    let half = hurt_span(strength) * 0.5;
    (middle - half, middle + half)
}

/// Opacity of an arc: a weak hit is a faint tick, a heavy one is solid.
fn hurt_alpha(strength: f32) -> f32 {
    0.30 + 0.70 * hurt_strength(strength)
}

/// Near-white for a graze, the damage red at full strength. Second to the arc's
/// length, never a substitute for it.
fn hurt_colour(strength: f32) -> Color {
    mix(INK, DANGER, hurt_strength(strength))
}

/// Fills one annular sector, given in radians, by tiling it with triangles.
///
/// `wheel_sector` draws the same shape, but it builds the outline as a `Vec`
/// first — fine for a menu, which is drawn once in a while, and not for an
/// indicator that runs for every incoming hit in the same frame, where this
/// module's per-frame path stays allocation-free. Triangles tile the band
/// exactly, so overlapping edges cannot double-blend into seams.
fn arc_band(centre: Vec2, r0: f32, r1: f32, a0: f32, a1: f32, colour: Color) {
    let r0 = sane(r0, 0.0).max(0.0);
    let r1 = sane(r1, r0).max(0.0);
    let start = sane(a0, 0.0);
    let span = sane(a1, 0.0) - start;
    if !span.is_finite() || span.abs() < 1.0e-3 || r1 <= r0 || r1 <= 0.0 {
        return;
    }
    let steps = ((span.abs() / (std::f32::consts::TAU / 64.0)).ceil() as usize).clamp(1, 48);
    let at = |t: f32, r: f32| {
        let a = start + span * t;
        centre + vec2(a.cos(), a.sin()) * r
    };
    for k in 0..steps {
        let t0 = k as f32 / steps as f32;
        let t1 = (k + 1) as f32 / steps as f32;
        let (i0, o0) = (at(t0, r0), at(t0, r1));
        let (i1, o1) = (at(t1, r0), at(t1, r1));
        draw_triangle(i0, o0, o1, colour);
        draw_triangle(i0, o1, i1, colour);
    }
}

thread_local! {
    /// The frame's damage numbers are reformatted from scratch every frame and
    /// repeat heavily — the same gun does the same damage shot after shot — so
    /// the digits go into one reused buffer instead of a `String` per number.
    /// One buffer is enough because each number is drawn before the next is
    /// written into it.
    static NUM_TEXT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Floating damage readouts, one per entry of `nums`, each
/// `(screen position, damage, headshot)`.
///
/// The caller owns the 3D-to-2D projection and the lifetime, and hands over the
/// list for the frame as it stands; nothing is remembered between frames. Each
/// number is centred on its point, horizontally and vertically, and is drawn at
/// a fixed opacity — a caller that wants it to fade should offset it instead.
///
/// A headshot is both larger and brighter, because at combat distance the size is
/// what carries: the same number in a different colour alone would leave a
/// colour-blind player reading a body shot. Damage is rounded to whole units, as
/// the rest of the HUD's values are.
pub fn draw_damage_numbers(f: &Fonts, nums: &[(Vec2, f32, bool)]) {
    if nums.is_empty() {
        return;
    }
    let ui = Ui::new();
    let (w, h) = (screen_width(), screen_height());
    NUM_TEXT.with(|cell| {
        let mut buf = cell.borrow_mut();
        for (at, amount, head) in nums {
            if !at.x.is_finite() || !at.y.is_finite() {
                // A point projected from behind the camera comes back wild;
                // drawing it would smear a glyph across the whole screen.
                continue;
            }
            let size = ui.s(damage_number_size(*head));
            if at.x < -size || at.x > w + size || at.y < -size || at.y > h + size {
                continue;
            }
            put(&mut buf, format_args!("{:.0}", sane(*amount, 0.0).max(0.0)));

            // `text_c` takes a baseline: 0.36 of the size is what centres a line
            // of this font on a y, the same lift the wheel's labels use.
            let base = at.y + size * 0.36;
            // Dark halo all round, drawn first. The numbers land on sky as often
            // as on a wall, and an outline is what keeps them legible on both —
            // one that only shaded one side would disappear on the other.
            let halo = ui.s(1.4).max(1.0);
            for (dx, dy) in [(-1.0f32, 0.0f32), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                f.text_c(
                    &buf,
                    at.x + dx * halo,
                    base + dy * halo,
                    size,
                    FEEDBACK_PLATE,
                );
            }
            f.text_c(&buf, at.x, base, size, damage_number_colour(*head));
        }
    });
}

/// Damage readouts are sized in layout units and scaled by the caller; the
/// headshot's is the larger of the two.
fn damage_number_size(head: bool) -> f32 {
    if head { 30.0 } else { 20.0 }
}

/// Body shots use the module's base ink, headshots the crosshair's headshot
/// colour, so one colour means the same thing everywhere in the frame.
fn damage_number_colour(head: bool) -> Color {
    if head { head_colour() } else { INK }
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
        fade(accent(), 0.45),
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
    // A long list needs the header to move up: with ten rows the authored
    // 20% title block would leave the last two rows off the bottom of a 720p
    // window, and the settings screen is one row away from that already.
    let head = if items.len() > 6 { h * 0.11 } else { h * 0.20 };
    let ty = head + ui.s(30.0);
    tracked_c(f, title, cx, ty, ui.s(54.0), INK, ui.s(10.0));

    let mut rule_y = ty + ui.s(26.0);
    if let Some(sub) = subtitle
        && !sub.is_empty()
    {
        f.text_c(sub, cx, ty + ui.s(32.0), ui.s(15.0), fade(INK, 0.55));
        rule_y = ty + ui.s(56.0);
    }
    let rw = (w * 0.34).clamp(ui.s(280.0), ui.s(520.0));
    draw_rectangle(cx - rw * 0.5, rule_y, rw, ui.s(2.0), fade(accent(), 0.85));
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

/// Row height and gutter for a menu list of `n` items given the room below
/// `top`, in `Ui` units. The authored list is 50 units of row and 10 of gutter,
/// which is what a short list always gets; a long one is squeezed until it
/// fits, down to a floor that still holds the label.
///
/// Split out of the drawing so the fit can be asserted: the settings screen is
/// the long list, and every setting it gains — it holds eleven now — must keep
/// the last row on a 720p window. Rows shrink rather than run off, and this is
/// the math that says so.
fn list_metrics(n: usize, room: f32, authored: f32) -> (f32, f32) {
    let room = sane(room, 1.0).max(1.0);
    let authored = sane(authored, 60.0).max(1.0);
    let n = n.max(1) as f32;
    let pitch = (room / n).min(authored);
    let ih = (pitch * (50.0 / 60.0)).max(authored * (20.0 / 60.0));
    let gap = (pitch - ih).max(0.0);
    (ih, gap)
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
    // The list has to fit the window it lands in: four screens share this, the
    // settings screen has eleven rows, and `Ui` stops growing at 1.5 — so past
    // 1080p the layout space stops growing with the screen while the authored
    // pitch does not, and the last rows ran off the bottom. Rows shrink (down to
    // a floor that still holds the label) rather than overflow; at 720p the
    // authored pitch only binds when the list is short.
    let (ih, gap) = list_metrics(items.len(), screen_height() - ui.s(26.0) - top, ui.s(60.0));
    // The row's contents are authored against the full 50-unit row, so a
    // squeezed row scales its type with itself.
    let k = (ih / ui.s(50.0)).clamp(0.3, 1.0);

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
            draw_rectangle(x, y, ui.s(4.0), ih, fade(accent(), a));
        }
        draw_rectangle_lines(
            x,
            y,
            iw,
            ih,
            1.0,
            if sel {
                fade(accent(), 0.85 * a)
            } else {
                fade(INK, 0.18 * a)
            },
        );

        f.text(
            &format!("{:02}", i + 1),
            x + ui.s(20.0),
            y + ih * 0.5 + ui.s(6.0) * k,
            ui.s(14.0) * k,
            fade(INK, 0.35 * a),
        );
        let label_size = ui.s(20.0) * k;
        f.text(
            &fit(f, item.label, label_size, iw - ui.s(100.0)),
            x + ui.s(58.0),
            y + ih * 0.5 + ui.s(7.0) * k,
            label_size,
            fade(INK, if sel { a } else { 0.72 * a }),
        );
        if sel {
            let mx = x + iw - ui.s(34.0);
            let my = y + ih * 0.5;
            draw_triangle(
                vec2(mx, my - ui.s(7.0) * k),
                vec2(mx, my + ui.s(7.0) * k),
                vec2(mx + ui.s(11.0) * k, my),
                fade(accent(), a),
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
    draw_rectangle(cx - rw * 0.5, ry, rw, ui.s(2.0), fade(accent(), 0.85));
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
    corner_marks(bx, by, bw, bh, ui.s(7.0), ui.s(2.0), fade(accent(), 0.55));
    tracked_c(f, text, cx, y + ui.s(2.0), size, fade(INK, 0.62), ui.s(2.0));
}

/// Separator between the hints of one footer, the same one the mission panel
/// puts between its hints in `main.rs`.
const HINT_SEP: &str = "   ·   ";

thread_local! {
    /// A footer's hints are joined into one reused buffer rather than formatted
    /// into a fresh `String`: an overlay redraws its footer every frame it is
    /// open, and the hints themselves never change.
    static HINTS: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Draws an overlay's footer: its own hints first, then whatever else the
/// screen wants to advertise, joined into the one boxed line the module uses.
fn overlay_hints(f: &Fonts, ui: &Ui, hints: &[&str], cx: f32, y: f32) {
    HINTS.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for (i, hint) in hints.iter().enumerate() {
            if i > 0 {
                buf.push_str(HINT_SEP);
            }
            buf.push_str(hint);
        }
        hint_line(f, ui, buf.as_str(), cx, y);
    });
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
        accent(),
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

    overlay_hints(
        f,
        &ui,
        &[i18n::t(Key::HintClose)],
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
        draw_rectangle(ix, y, ui.s(4.0), h, accent());
    }
    draw_rectangle_lines(
        ix,
        y,
        right - ix,
        h,
        1.0,
        if active {
            fade(accent(), 0.85)
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
            fade(accent(), 0.95)
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
    draw_triangle(tip, tail_l, tail_r, accent());
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

    overlay_hints(f, &ui, &[i18n::t(Key::HintClose)], w * 0.5, h - ui.s(24.0));
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
            mix(INK, accent(), 0.55),
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
                    fade(accent(), 0.95)
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
                    accent()
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
                    draw_circle(tx, ty, ui.s(4.0), accent());
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
        fade(accent(), 0.95),
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
                accent()
            },
        );
    }
}

// --- field manual ---------------------------------------------------------

/// One row of the controls page: what to press, and what it does. Both strings
/// arrive already localised, so the page is a pure function of its data.
pub struct HelpRow {
    pub keys: &'static str,
    pub text: &'static str,
}

/// A titled group of control rows.
pub struct HelpSection {
    pub title: &'static str,
    pub rows: &'static [HelpRow],
}

/// One card in the manual: an archetype, a weapon or a piece of kit.
///
/// `stats` is written by `codex` out of the same tables the simulation reads, so
/// the manual cannot drift from the game. `portrait` is the archetype's own
/// model in its local space with its feet on `y = 0`, drawn flat.
pub struct CodexRow {
    pub name: &'static str,
    pub tag: &'static str,
    pub note: &'static str,
    pub stats: String,
    pub swatch: Color,
    pub portrait: Option<Mesh>,
}

// The manual's own measurements, in layout units. Its pages are authored
// against the same 1280x720 the rest of the module is, but with more room per
// item than the HUD gives its readouts: a page here exists to be read, and its
// items are few enough (four groups of keys, a page of cards) to afford it.
/// Padding inside the manual's panel.
const MAN_PAD: f32 = 22.0;
/// Top of the panel down to the top of the content: the band the heading and
/// its rule live in.
const MAN_HEAD: f32 = 74.0;
/// The band at the foot of the panel the hint line and the pager live in.
const MAN_FOOT: f32 = 46.0;
/// A section's own heading, and the space that follows a section.
const MAN_SEC_HEAD: f32 = 22.0;
const MAN_SEC_GAP: f32 = 13.0;
/// A control row: the height it is authored at, the floor it may be squeezed to
/// on a short window, and the size of each of its two halves.
const MAN_ROW_H: f32 = 19.0;
const MAN_ROW_MIN: f32 = 14.0;
const MAN_KEY_SIZE: f32 = 12.5;
const MAN_TEXT_SIZE: f32 = 14.0;
/// The narrowest the key column may be squeezed to before the bindings start
/// truncating.
const MAN_KEY_MIN: f32 = 52.0;
/// Gutter between the controls page's two columns.
const MAN_COL_GAP: f32 = 34.0;
/// The goal paragraph: its label, its leading, and the most lines it may take
/// before the key groups are the ones paying for it.
const MAN_GOAL_LABEL: f32 = 26.0;
const MAN_GOAL_LEAD: f32 = 17.0;
const MAN_GOAL_SIZE: f32 = 14.0;
const MAN_GOAL_MAX: usize = 2;
/// The survival notes: the rule above them, and their type size.
const MAN_TIPS_HEAD: f32 = 26.0;
const MAN_TIP_SIZE: f32 = 13.5;
/// The card grid: its gutter, the padding inside a card, the shortest card
/// worth drawing, and the most columns a page may ever use.
const MAN_CARD_GAP: f32 = 10.0;
const MAN_CARD_PAD: f32 = 10.0;
const MAN_CARD_MIN: f32 = 46.0;
const MAN_CARD_MAX_COLS: usize = 3;
/// A portrait: the width of the well it stands in, and the height it is drawn
/// at when its card can afford one.
const MAN_PORTRAIT_W: f32 = 62.0;
const MAN_PORTRAIT_H: f32 = 90.0;
/// Card type: the name, the tag, the stat line and the note, with the leading
/// they share — the authored one, and the floor a short card squeezes to. A
/// card offers all four lines and draws the ones it has room for.
const MAN_CARD_LINES: usize = 4;
const MAN_CARD_NAME: f32 = 16.0;
const MAN_CARD_TAG: f32 = 12.0;
const MAN_CARD_STATS: f32 = 12.0;
const MAN_CARD_NOTE: f32 = 11.5;
const MAN_CARD_LEAD: f32 = 20.0;
const MAN_CARD_MIN_LEAD: f32 = 12.0;

// Every rectangle on a manual page is worked out here, before a shape is drawn:
// layout that can be exercised without a window or a font is layout that can be
// checked, and the fit of a page is exactly the kind of thing that is otherwise
// only found by looking at it.

/// The manual's content area, in layout units: the panel with its padding off,
/// from the rule under the title to the top of the footer. `uw` and `uh` are
/// the window measured in layout units, which is what `Ui` scaling divides out.
fn manual_body(uw: f32, uh: f32) -> (f32, f32, f32, f32) {
    let x = MARGIN + MAN_PAD;
    let top = MARGIN + MAN_HEAD;
    let right = (sane(uw, 0.0) - MARGIN - MAN_PAD).max(x);
    let bottom = (sane(uh, 0.0) - MARGIN - MAN_FOOT).max(top);
    (x, top, right - x, bottom - top)
}

/// The baseline of an overlay's footer hints, in layout units: high enough in
/// the panel that the box `hint_line` draws around them stays inside it.
fn manual_footer_y(uh: f32) -> f32 {
    sane(uh, 0.0) - MARGIN - 10.0
}

/// Height of one control section at `row_h` per row: its heading, its rows, and
/// the gap that follows it.
fn section_height(rows: usize, row_h: f32) -> f32 {
    MAN_SEC_HEAD + rows as f32 * sane(row_h, 0.0).max(0.0) + MAN_SEC_GAP
}

/// Index of the section the second column starts at: sections join the first
/// column until it holds at least half of the page's height. Groups of similar
/// size split down the middle, and one long group keeps a column to itself
/// instead of being cut in two.
fn column_split(sections: &[HelpSection], row_h: f32) -> usize {
    let n = sections.len();
    // Fewer than two groups is nothing to balance, and they all go left: an
    // empty first column reads as a broken layout rather than as a choice.
    if n < 2 {
        return n;
    }
    let total: f32 = sections
        .iter()
        .map(|s| section_height(s.rows.len(), row_h))
        .sum();
    let half = total * 0.5;
    let mut split = 0;
    let mut filled = 0.0;
    while split + 1 < n && filled < half {
        filled += section_height(sections[split].rows.len(), row_h);
        split += 1;
    }
    split
}

/// Row height a page can afford: `rows` rows plus `fixed` units of headings
/// have to fit into `avail`, and the answer is clamped to `min..=nominal`.
///
/// The height is only ever squeezed, never stretched — a page with room to
/// spare keeps the spacing it was authored with — and a nonsense `avail` falls
/// back to the authored height rather than to a negative one.
fn fitted_row_h(rows: usize, fixed: f32, avail: f32, nominal: f32, min: f32) -> f32 {
    let lo = sane(min, 1.0).max(0.1);
    let hi = sane(nominal, lo).max(lo);
    let fixed = sane(fixed, 0.0).max(0.0);
    if rows == 0 {
        return hi;
    }
    let avail = sane(avail, fixed + hi * rows as f32);
    ((avail - fixed) / rows as f32).clamp(lo, hi)
}

/// Width of the controls page's key column: the widest binding plus the gap
/// after it, capped so the description keeps the larger share of the row.
fn key_column(widest: f32, col_w: f32, min_w: f32) -> f32 {
    let col_w = sane(col_w, 0.0).max(0.0);
    let min_w = sane(min_w, 0.0).max(0.0);
    let cap = (col_w * 0.46).max(min_w);
    (sane(widest, 0.0).max(0.0) + 12.0).clamp(min_w, cap)
}

/// How many columns a card page needs to show every card at least `min_h` units
/// tall: as few as will do, capped at `max_cols` and at one column per card.
fn card_columns(rows: usize, avail_h: f32, min_h: f32, max_cols: usize) -> usize {
    let rows = rows.max(1);
    let max_cols = max_cols.max(1);
    let avail = sane(avail_h, 0.0).max(0.0);
    let min_h = sane(min_h, 1.0).max(0.1);
    let per_col = (avail / min_h).floor().max(1.0);
    let need = (rows as f32 / per_col).ceil().max(1.0);
    // The cap is what keeps a short window from turning a card page into a wall
    // of slivers; a page always shows at least one card.
    (need as usize).clamp(1, max_cols).min(rows)
}

/// Rect of card `i` in a grid `cols` wide, in layout units.
///
/// Cards fill down a column before the next column starts, so the order the
/// rows arrive in is the order they are read in — a bestiary is ordered by the
/// mission that introduces each archetype, and reading across would interleave
/// two of them.
fn card_rect(
    i: usize,
    rows: usize,
    cols: usize,
    area: (f32, f32, f32, f32),
    gap: f32,
) -> (f32, f32, f32, f32) {
    let (ax, ay, aw, ah) = area;
    let (ax, ay) = (sane(ax, 0.0), sane(ay, 0.0));
    let (aw, ah) = (sane(aw, 0.0).max(0.0), sane(ah, 0.0).max(0.0));
    let gap = sane(gap, 0.0).max(0.0);
    if rows == 0 {
        return (ax, ay, 0.0, 0.0);
    }
    let i = i.min(rows - 1);
    let cols = cols.clamp(1, rows);
    let per_col = rows.div_ceil(cols);
    let cw = ((aw - gap * (cols as f32 - 1.0)) / cols as f32).max(0.0);
    let ch = ((ah - gap * (per_col as f32 - 1.0)) / per_col as f32).max(0.0);
    let (col, row) = (i / per_col, i % per_col);
    (
        ax + col as f32 * (cw + gap),
        ay + row as f32 * (ch + gap),
        cw,
        ch,
    )
}

/// Room inside a card, in layout units: what is left of its height once its
/// padding is off. Never negative, whatever card it is handed.
fn card_room(card_h: f32, pad: f32) -> f32 {
    (sane(card_h, 0.0).max(0.0) - 2.0 * sane(pad, 0.0).max(0.0)).max(0.0)
}

/// Lines a card of `card_h` units has room for. The name, tag, stats and note
/// are offered in that order and whatever does not fit is dropped, so a short
/// card loses its prose and never its name; at least one line is always
/// reported, because a card with no room at all still has to draw something.
fn card_lines(card_h: f32, pad: f32, min_lead: f32, max_lines: usize) -> usize {
    let lead = sane(min_lead, 1.0).max(0.1);
    ((card_room(card_h, pad) / lead).floor() as usize).clamp(1, max_lines.max(1))
}

/// Leading between a card's lines: the room divided by the lines that fit,
/// inside the range the four type sizes can live with.
fn card_lead(card_h: f32, pad: f32, lines: usize, max_lead: f32, min_lead: f32) -> f32 {
    let lo = sane(min_lead, 1.0).max(0.1);
    let hi = sane(max_lead, lo).max(lo);
    (card_room(card_h, pad) / lines.max(1) as f32).clamp(lo, hi)
}

/// The pager's label. A page number off either end is clamped into range rather
/// than printed: "0 / 0" and "7 / 3" both read as a bug in the manual instead of
/// as a page that is stuck, which is what they are.
fn page_label(out: &mut String, page: usize, pages: usize) -> &str {
    let last = pages.max(1);
    put(out, format_args!("{} / {}", page.clamp(1, last), last));
    out.as_str()
}

/// Greedy line breaks for a paragraph: `out` is filled with the byte ranges of
/// the lines that fit `max_w` at `size`, breaking between words. `measure` is
/// the font's width function, taken as a parameter so the breaking can be
/// exercised without a font.
///
/// A word wider than a whole line is cut at the last character that fits rather
/// than allowed to overflow, and every line takes at least one character, so a
/// nonsense width can only produce one glyph per line — never a loop that does
/// not end.
fn wrap_ranges(
    s: &str,
    size: f32,
    max_w: f32,
    measure: &dyn Fn(&str, f32) -> f32,
    out: &mut Vec<(usize, usize)>,
) {
    out.clear();
    let max_w = sane(max_w, 0.0).max(0.0);
    let n = s.len();
    let mut start = 0usize;
    while start < n {
        // A line starts on a visible character: the space a break fell on is
        // dropped rather than indenting the next line.
        while start < n && s.as_bytes()[start] == b' ' {
            start += 1;
        }
        if start >= n {
            break;
        }
        let mut end = start;
        let mut brk: Option<usize> = None;
        for (i, ch) in s[start..].char_indices() {
            let next = start + i + ch.len_utf8();
            if measure(&s[start..next], size) > max_w {
                break;
            }
            end = next;
            if ch == ' ' {
                brk = Some(next);
            }
        }
        if end == start {
            // Nothing fits: take one character anyway, so the loop advances.
            end = start + s[start..].chars().next().map_or(1, char::len_utf8);
        } else if end < n {
            if let Some(b) = brk {
                end = b;
            }
        }
        let mut stop = end;
        while stop > start && s.as_bytes()[stop - 1] == b' ' {
            stop -= 1;
        }
        out.push((start, stop));
        start = end;
    }
}

thread_local! {
    /// The manual's scratch, kept between frames: a page redraws every frame it
    /// is open, so the strings that have to be fitted and the ranges the goal
    /// wraps into may not be rebuilt from scratch each time.
    static MAN_TEXT: RefCell<ManualText> = RefCell::new(ManualText::default());
}

#[derive(Default)]
struct ManualText {
    /// Byte ranges of the goal's wrapped lines, in order.
    goal: Vec<(usize, usize)>,
    /// One fitted line of text, drawn before the next is written into it: a
    /// card writes four lines through it in a row and never needs two at once.
    line: String,
    /// The pager's label.
    pager: String,
}

/// Fits `s` into `max_w` and draws it, through the manual's scratch buffer.
///
/// Every string on a manual page comes through here. The cards are a fixed
/// size and their text is not — a name, a tag and a stat line are the codex's,
/// and a longer localisation of any of them would otherwise run over the card
/// beside it.
fn manual_text(f: &Fonts, s: &str, x: f32, y: f32, size: f32, color: Color, max_w: f32) {
    MAN_TEXT.with(|cell| {
        let mut t = cell.borrow_mut();
        let out = fit_into(&mut t.line, s, size, max_w.max(0.0), &|text, size| {
            f.width(text, size)
        });
        f.text(out, x, y, size, color);
    });
}

/// The controls page: the goal, then the key groups in two columns, with the
/// survival notes at the foot of the second one.
///
/// `page` is one-based for the pager; `pages` is how many pages the manual has.
pub fn draw_manual_controls(
    f: &Fonts,
    page: usize,
    pages: usize,
    sections: &[HelpSection],
    tips: &[&str],
) {
    let ui = Ui::new();
    let (w, h) = (screen_width(), screen_height());
    scrim(0.60);
    panel(
        ui.s(MARGIN),
        ui.s(MARGIN),
        (w - ui.s(MARGIN * 2.0)).max(0.0),
        (h - ui.s(MARGIN * 2.0)).max(0.0),
    );
    // Everything below is placed in layout units and scaled on the way out.
    let (uw, uh) = (w / ui.scale, h / ui.scale);
    heading(
        f,
        &ui,
        i18n::t(Key::HowToPlay),
        w * 0.5,
        ui.s(MARGIN + 38.0),
        ui.s(420.0),
    );
    let (bx, by, bw, bh) = manual_body(uw, uh);

    // --- the goal ---------------------------------------------------------
    tracked(
        f,
        i18n::t(Key::ManualGoal),
        ui.s(bx),
        ui.s(by + 12.0),
        ui.s(12.0),
        mix(INK, accent(), 0.55),
        ui.s(2.2),
    );

    // The goal is the page's one piece of prose, so it is wrapped rather than
    // truncated: a goal the player cannot finish reading is worse than a
    // shorter list of keys. Lines past the cap end on `..`, from the same
    // fitter the cards use.
    let body = i18n::t(Key::ManualGoalBody);
    let mut ranges = [(0usize, 0usize); MAN_GOAL_MAX];
    let (drawn, cut) = MAN_TEXT.with(|cell| {
        let mut t = cell.borrow_mut();
        // Taken and put back so the buffer keeps its allocation while the
        // ranges are being written — and so a second borrow of the cell is not
        // needed to draw them.
        let mut goal = std::mem::take(&mut t.goal);
        wrap_ranges(
            body,
            ui.s(MAN_GOAL_SIZE),
            ui.s(bw),
            &|s, size| f.width(s, size),
            &mut goal,
        );
        let total = goal.len();
        for (k, r) in goal.iter().take(MAN_GOAL_MAX).enumerate() {
            ranges[k] = *r;
        }
        t.goal = goal;
        (total.min(MAN_GOAL_MAX), total > MAN_GOAL_MAX)
    });
    for (k, &(a, b)) in ranges.iter().take(drawn).enumerate() {
        let y = ui.s(by + MAN_GOAL_LABEL + k as f32 * MAN_GOAL_LEAD + MAN_GOAL_SIZE * 0.78);
        if cut && k + 1 == drawn {
            manual_text(
                f,
                &body[a..],
                ui.s(bx),
                y,
                ui.s(MAN_GOAL_SIZE),
                fade(INK, 0.82),
                ui.s(bw),
            );
        } else {
            manual_text(
                f,
                &body[a..b],
                ui.s(bx),
                y,
                ui.s(MAN_GOAL_SIZE),
                fade(INK, 0.82),
                ui.s(bw),
            );
        }
    }
    let col_top = by + MAN_GOAL_LABEL + drawn as f32 * MAN_GOAL_LEAD + MAN_SEC_GAP;

    // --- the key groups ---------------------------------------------------
    let col_h = (by + bh - col_top).max(0.0);
    let col_w = ((bw - MAN_COL_GAP) * 0.5).max(0.0);
    let split = column_split(sections, MAN_ROW_H).min(sections.len());
    let (left, right) = sections.split_at(split);
    let rows_left: usize = left.iter().map(|s| s.rows.len()).sum();
    let rows_right: usize = right.iter().map(|s| s.rows.len()).sum::<usize>() + tips.len();
    let fixed_left = left.len() as f32 * (MAN_SEC_HEAD + MAN_SEC_GAP);
    let fixed_right = right.len() as f32 * (MAN_SEC_HEAD + MAN_SEC_GAP)
        + if tips.is_empty() {
            0.0
        } else {
            MAN_TIPS_HEAD + MAN_SEC_GAP
        };
    // The row height is set by whichever column is taller, both measured at the
    // authored height: a column with few rows and many headings can still be
    // the one that does not fit.
    let (rows_max, fixed_max) = if fixed_left + rows_left as f32 * MAN_ROW_H
        >= fixed_right + rows_right as f32 * MAN_ROW_H
    {
        (rows_left, fixed_left)
    } else {
        (rows_right, fixed_right)
    };
    let row_h = fitted_row_h(rows_max, fixed_max, col_h, MAN_ROW_H, MAN_ROW_MIN);

    // The bindings are measured once for the whole page and the column is sized
    // from the widest of them, which is what puts every description at the same
    // x down the page.
    let mut widest = 0.0f32;
    for s in sections {
        for r in s.rows {
            widest = widest.max(f.width(r.keys, ui.s(MAN_KEY_SIZE)) / ui.scale);
        }
    }
    let key_w = key_column(widest, col_w, MAN_KEY_MIN);

    let bottom = by + bh;
    let rx = bx + col_w + MAN_COL_GAP;
    if sections.is_empty() && tips.is_empty() {
        // Nothing to lay out: one dash in the middle of the page, which is how
        // the empty lists elsewhere in this module say the same thing.
        f.text_c(
            NONE,
            ui.s(bx + bw * 0.5),
            ui.s(col_top + row_h),
            ui.s(15.0),
            fade(INK, 0.45),
        );
    } else {
        control_column(f, &ui, left, bx, col_w, col_top, bottom, row_h, key_w);
        let y = control_column(f, &ui, right, rx, col_w, col_top, bottom, row_h, key_w);
        tips_block(
            f,
            &ui,
            tips,
            NotesBlock {
                x: rx,
                w: col_w,
                y,
                bottom,
                row_h,
            },
        );
    }

    overlay_hints(
        f,
        &ui,
        &[i18n::t(Key::HintClose), i18n::t(Key::ManualPages)],
        w * 0.5,
        ui.s(manual_footer_y(uh)),
    );
    pager(f, &ui, page, pages, uw, uh);
}

/// Draws a run of control sections down one column and returns the y it
/// finished at. A row that would fall past `bottom` is dropped and marked with
/// `..` — the degradation every other list in this module uses, and the reason
/// a page can never draw outside itself.
#[allow(clippy::too_many_arguments)] // one column of a two-column page
fn control_column(
    f: &Fonts,
    ui: &Ui,
    sections: &[HelpSection],
    x: f32,
    w: f32,
    mut y: f32,
    bottom: f32,
    row_h: f32,
    key_w: f32,
) -> f32 {
    for section in sections {
        if y + MAN_SEC_HEAD > bottom {
            break;
        }
        section_head(
            f,
            ui,
            section.title,
            ui.s(x),
            ui.s(x + w),
            ui.s(y + 13.0),
            mix(INK, accent(), 0.55),
        );
        y += MAN_SEC_HEAD;
        for (i, row) in section.rows.iter().enumerate() {
            if y + row_h > bottom {
                f.text(
                    "..",
                    ui.s(x),
                    ui.s(y + row_h * 0.5),
                    ui.s(13.0),
                    fade(INK, 0.40),
                );
                return y;
            }
            control_row(f, ui, x, w, y, row_h, row, key_w, i % 2 == 0);
            y += row_h;
        }
        y += MAN_SEC_GAP;
    }
    y
}

/// One control row: the binding in a dark keycap, and what it does beside it.
///
/// The keycap is what separates "what to press" from "what it does" without
/// leaning on a hue — the column alone would leave the two halves of a row
/// reading as one sentence, which is exactly the misreading left to a
/// colour-blind player if the split were carried by colour.
#[allow(clippy::too_many_arguments)] // one row of a key/description table
fn control_row(
    f: &Fonts,
    ui: &Ui,
    x: f32,
    w: f32,
    y: f32,
    row_h: f32,
    row: &HelpRow,
    key_w: f32,
    band: bool,
) {
    if band {
        draw_rectangle(ui.s(x), ui.s(y), ui.s(w), ui.s(row_h), fade(INK, 0.035));
    }
    // The key cap column only exists for rows that split their binding out.
    // The manual's own rows carry the keys inside the description ("W A S D
    // move"), so for them the cap would be an empty box pretending to be a
    // column — and the text starts at the row's left edge instead.
    let text_x = if row.keys.is_empty() {
        x
    } else {
        let cap_h = (row_h - 5.0).clamp(9.0, 18.0);
        let cap_w = (key_w - 12.0).max(8.0);
        let cap_y = y + (row_h - cap_h) * 0.5;
        draw_rectangle(
            ui.s(x),
            ui.s(cap_y),
            ui.s(cap_w),
            ui.s(cap_h),
            Color::new(0.05, 0.06, 0.08, 0.80),
        );
        draw_rectangle_lines(
            ui.s(x),
            ui.s(cap_y),
            ui.s(cap_w),
            ui.s(cap_h),
            1.0,
            fade(INK, 0.18),
        );
        manual_text(
            f,
            row.keys,
            ui.s(x + 6.0),
            ui.s(cap_y + cap_h * 0.5 + MAN_KEY_SIZE * 0.36),
            ui.s(MAN_KEY_SIZE),
            INK,
            ui.s(cap_w - 12.0),
        );
        x + key_w
    };
    manual_text(
        f,
        row.text,
        ui.s(text_x),
        ui.s(y + row_h * 0.5 + MAN_TEXT_SIZE * 0.36),
        ui.s(MAN_TEXT_SIZE),
        fade(INK, 0.85),
        ui.s(w - (text_x - x)),
    );
    draw_rectangle(ui.s(x), ui.s(y + row_h), ui.s(w), 1.0, fade(INK, 0.05));
}

/// The survival notes: a rule, then one bulleted line each, in the shadow of
/// the key groups above them. Where the block goes is the caller's business, so
/// the rectangle travels as one value rather than as four loose numbers.
struct NotesBlock {
    x: f32,
    w: f32,
    y: f32,
    bottom: f32,
    row_h: f32,
}

fn tips_block(f: &Fonts, ui: &Ui, tips: &[&str], at: NotesBlock) {
    let NotesBlock {
        x,
        w,
        mut y,
        bottom,
        row_h,
    } = at;
    if tips.is_empty() || y + MAN_TIPS_HEAD > bottom {
        return;
    }
    draw_rectangle(ui.s(x), ui.s(y + 6.0), ui.s(w), 1.0, fade(INK, 0.14));
    y += MAN_TIPS_HEAD;
    for tip in tips {
        if y + row_h > bottom {
            f.text(
                "..",
                ui.s(x),
                ui.s(y + row_h * 0.5),
                ui.s(13.0),
                fade(INK, 0.40),
            );
            return;
        }
        // The notes are the manual's opinions rather than its key bindings, and
        // a square bullet is the cheapest way to say so without reaching for a
        // second colour to say it with.
        draw_rectangle(
            ui.s(x + 1.0),
            ui.s(y + row_h * 0.5 - 2.0),
            ui.s(4.0),
            ui.s(4.0),
            fade(accent(), 0.85),
        );
        manual_text(
            f,
            tip,
            ui.s(x + 14.0),
            ui.s(y + row_h * 0.5 + MAN_TIP_SIZE * 0.36),
            ui.s(MAN_TIP_SIZE),
            fade(INK, 0.78),
            ui.s(w - 14.0),
        );
        y += row_h;
    }
}

/// A page of cards: every card on the page at once, in a grid that takes
/// another column rather than a shorter card.
pub fn draw_manual_cards(f: &Fonts, page: usize, pages: usize, title: &str, rows: &[CodexRow]) {
    let ui = Ui::new();
    let (w, h) = (screen_width(), screen_height());
    scrim(0.60);
    panel(
        ui.s(MARGIN),
        ui.s(MARGIN),
        (w - ui.s(MARGIN * 2.0)).max(0.0),
        (h - ui.s(MARGIN * 2.0)).max(0.0),
    );
    let (uw, uh) = (w / ui.scale, h / ui.scale);
    heading(f, &ui, title, w * 0.5, ui.s(MARGIN + 38.0), ui.s(420.0));
    let (bx, by, bw, bh) = manual_body(uw, uh);

    if rows.is_empty() {
        // An empty page says so and stops: there is nothing to lay out, and a
        // page of nothing but a frame reads as a broken screen.
        f.text_c(
            NONE,
            w * 0.5,
            ui.s(by + bh * 0.5),
            ui.s(15.0),
            fade(INK, 0.45),
        );
    } else {
        // A card the player has to squint at is worse than a longer page, so
        // the grid grows a column before the cards go short — and an archetype
        // card needs room for its portrait before anything else is drawn in it.
        let min_h = if rows.iter().any(|r| r.portrait.is_some()) {
            MAN_PORTRAIT_H + MAN_CARD_PAD * 2.0
        } else {
            MAN_CARD_MIN
        };
        let cols = card_columns(rows.len(), bh, min_h, MAN_CARD_MAX_COLS);
        for (i, row) in rows.iter().enumerate() {
            let (cx, cy, cw, ch) = card_rect(i, rows.len(), cols, (bx, by, bw, bh), MAN_CARD_GAP);
            codex_card(f, &ui, row, cx, cy, cw, ch);
        }
    }

    overlay_hints(
        f,
        &ui,
        &[i18n::t(Key::HintClose), i18n::t(Key::ManualPages)],
        w * 0.5,
        ui.s(manual_footer_y(uh)),
    );
    pager(f, &ui, page, pages, uw, uh);
}

/// One card: the portrait well (archetypes only), the swatch, the name, the
/// tag, the stat line and, when the card is tall enough to hold it, the note.
///
/// Everything here is measured in layout units and scaled on the way out, so a
/// card keeps its proportions at every window size. Lines are dropped from the
/// bottom up when the card is short: whatever else a card loses, it keeps the
/// name it is filed under.
fn codex_card(f: &Fonts, ui: &Ui, row: &CodexRow, x: f32, y: f32, w: f32, h: f32) {
    // A card is a plate inside the panel, a shade darker than it, so a grid of
    // short cards still reads as separate cards rather than as a slab of text.
    draw_rectangle(
        ui.s(x),
        ui.s(y),
        ui.s(w),
        ui.s(h),
        Color::new(0.06, 0.07, 0.09, 0.55),
    );
    draw_rectangle_lines(ui.s(x), ui.s(y), ui.s(w), ui.s(h), 1.0, fade(INK, 0.14));

    let pad = MAN_CARD_PAD;
    let mut tx = x + pad;
    if let Some(mesh) = &row.portrait {
        let well_w = MAN_PORTRAIT_W.min((w - pad * 2.0) * 0.45).max(14.0);
        let well_h = (h - pad * 2.0).max(0.0);
        // The well: a darker plate for the model to stand in, which is what
        // keeps a dark silhouette off a dark card edge.
        draw_rectangle(
            ui.s(tx),
            ui.s(y + pad),
            ui.s(well_w),
            ui.s(well_h),
            Color::new(0.02, 0.025, 0.03, 0.60),
        );
        draw_rectangle_lines(
            ui.s(tx),
            ui.s(y + pad),
            ui.s(well_w),
            ui.s(well_h),
            1.0,
            fade(INK, 0.10),
        );
        let ph = MAN_PORTRAIT_H.min(well_h - 6.0).max(0.0);
        draw_portrait(
            mesh,
            vec2(ui.s(tx + well_w * 0.5), ui.s(y + pad + well_h - 4.0)),
            ui.s(ph),
            ui.s((well_w - 6.0).max(0.0)),
        );
        tx += well_w + 10.0;
    }

    let right = x + w - pad;
    let lines = card_lines(h, pad, MAN_CARD_MIN_LEAD, MAN_CARD_LINES);
    let lead = card_lead(h, pad, lines, MAN_CARD_LEAD, MAN_CARD_MIN_LEAD);
    let mut ly = y + pad + lead * 0.80;
    let name_x = tx + 14.0;
    let name_w = (right - name_x).max(0.0);

    // The swatch: the one saturated colour on the card, ringed in the plate's
    // own dark so it reads whatever colour the archetype wears. Everything else
    // on the card is separated by brightness, so the hue is a label here and
    // never the only thing that tells two cards apart.
    let dot = vec2(tx + 5.0, ly - MAN_CARD_NAME * 0.30);
    draw_circle(ui.s(dot.x), ui.s(dot.y), ui.s(4.2), row.swatch);
    draw_circle_lines(
        ui.s(dot.x),
        ui.s(dot.y),
        ui.s(4.2),
        1.0,
        Color::new(0.02, 0.02, 0.03, 0.90),
    );

    manual_text(
        f,
        row.name,
        ui.s(name_x),
        ui.s(ly),
        ui.s(MAN_CARD_NAME),
        INK,
        ui.s(name_w),
    );
    if lines >= 2 {
        ly += lead;
        manual_text(
            f,
            row.tag,
            ui.s(name_x),
            ui.s(ly),
            ui.s(MAN_CARD_TAG),
            mix(INK, accent(), 0.55),
            ui.s(name_w),
        );
    }
    if lines >= 3 {
        ly += lead;
        manual_text(
            f,
            &row.stats,
            ui.s(name_x),
            ui.s(ly),
            ui.s(MAN_CARD_STATS),
            fade(INK, 0.60),
            ui.s(name_w),
        );
    }
    if lines >= 4 {
        ly += lead;
        manual_text(
            f,
            row.note,
            ui.s(name_x),
            ui.s(ly),
            ui.s(MAN_CARD_NOTE),
            fade(INK, 0.45),
            ui.s(name_w),
        );
    }
}

/// The pager line, bottom right: which page of how many.
fn pager(f: &Fonts, ui: &Ui, page: usize, pages: usize, uw: f32, uh: f32) {
    MAN_TEXT.with(|cell| {
        let mut t = cell.borrow_mut();
        let label = page_label(&mut t.pager, page, pages);
        let size = ui.s(14.0);
        let x = ui.s(uw - MARGIN - MAN_PAD) - f.width(label, size);
        f.text(
            label,
            x,
            ui.s(manual_footer_y(uh) + 2.0),
            size,
            fade(INK, 0.80),
        );
    });
}

// --- portraits ------------------------------------------------------------
//
// The one piece of new geometry in the manual: an archetype's own model, in its
// own local space with its feet on `y = 0`, drawn flat into its card.

thread_local! {
    /// Triangle order for the portrait being drawn, kept between frames: the
    /// sort would otherwise allocate once per portrait per frame, and a card
    /// page redraws every frame it is open.
    static PORTRAIT_ORDER: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// Screen position of a model-space point in a portrait whose feet stand on
/// `anchor` (screen pixels).
///
/// Two signs are load-bearing here, and both come from the same camera: an
/// orthographic view of the model's *front*, which is where the player meets
/// the archetype, so the camera sits on `-z` looking towards `+z` — the model's
/// own forward is `-z`, the direction `Player::forward` points at yaw 0.
///
/// The screen's `y` grows downwards while the model's grows up from its feet,
/// so the height is *subtracted*: a point above the feet has to come out above
/// the anchor. Adding it would hang the figure head-down below its own feet,
/// which is the one error a portrait cannot hide. `x` is negated because that
/// camera's right hand points along `-x`: the model's `+x` is its left, and its
/// left belongs on the left of the card.
fn portrait_point(p: Vec3, anchor: Vec2, scale: f32) -> Vec2 {
    anchor + vec2(-sane(p.x, 0.0), -sane(p.y, 0.0)) * sane(scale, 0.0)
}

/// Scale and horizontal centre for a portrait of `mesh` that comes out `height`
/// units tall and no wider than `max_w`: the scale that makes the model's own
/// height `height`, and where the middle of its footprint is, so a model built
/// off-centre is still centred in its card.
///
/// A mesh with nothing above the floor has no height to scale from and fits at
/// zero, which the caller reads as "nothing to draw" — the alternative is a
/// division by zero and a portrait made of NaNs.
fn portrait_fit(mesh: &Mesh, height: f32, max_w: f32) -> (f32, f32) {
    let height = sane(height, 0.0);
    if height <= 0.0 {
        return (0.0, 0.0);
    }
    let mut top = f32::NEG_INFINITY;
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for v in &mesh.vertices {
        let p = v.position;
        if !(p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        top = top.max(p.y);
        left = left.min(p.x);
        right = right.max(p.x);
    }
    if !top.is_finite() || top <= 1.0e-3 {
        return (0.0, 0.0);
    }
    // A wide model is held back by the well it has to stand in, even though the
    // height is what the portrait is measured by.
    let by_height = height / top;
    let wide = (right - left).max(0.0);
    let by_width = if wide > 1.0e-3 && sane(max_w, 0.0) > 0.0 {
        sane(max_w, by_height) / wide
    } else {
        by_height
    };
    (by_height.min(by_width), (left + right) * 0.5)
}

/// Mean depth of triangle `t`: the `z` the painter's algorithm sorts on. An
/// orthographic view along `z` leaves the depth of a face at the average of its
/// three corners, and `+z` is *away* from the camera, so a larger depth is
/// further.
///
/// A triangle whose indices point nowhere has no depth at all and sorts to the
/// far end of the order, where it is drawn first and draws nothing.
fn tri_depth(mesh: &Mesh, t: usize) -> f32 {
    let start = t * 3;
    let mut sum = 0.0;
    let mut n = 0.0;
    for k in 0..3 {
        if let Some(v) = mesh
            .indices
            .get(start + k)
            .and_then(|i| mesh.vertices.get(*i as usize))
        {
            sum += sane(v.position.z, 0.0);
            n += 1.0;
        }
    }
    if n > 0.0 { sum / n } else { f32::INFINITY }
}

/// Fills `out` with the triangle indices of `mesh`, ordered far to near.
///
/// This is the whole of the depth test: a portrait is drawn with the 2D pass
/// and there is no depth buffer in it, so every triangle of the model — its
/// back faces included — is filled flat, and this order is the only thing
/// deciding which one wins. Drawn the other way round, the far side of a torso
/// paints over the near side and the model reads as a broken box rather than as
/// a silhouette.
fn portrait_order(mesh: &Mesh, out: &mut Vec<usize>) {
    out.clear();
    out.extend(0..mesh.indices.len() / 3);
    // `total_cmp` orders the NaN a broken vertex would produce instead of
    // panicking the way `partial_cmp` would, and the sort is stable, so a mesh
    // of equally deep triangles keeps the order it was built in.
    out.sort_by(|a, b| tri_depth(mesh, *b).total_cmp(&tri_depth(mesh, *a)));
}

/// A vertex's colour as the 2D pass wants it. The models are flat-shaded when
/// they are built and every corner of a face carries the same colour, so one
/// triangle is one colour and `draw_triangle` is the whole of the fill.
fn vertex_colour(v: &Vertex) -> Color {
    Color::new(
        v.color[0] as f32 / 255.0,
        v.color[1] as f32 / 255.0,
        v.color[2] as f32 / 255.0,
        v.color[3] as f32 / 255.0,
    )
}

/// Draws `mesh` as a portrait `height` pixels tall standing on `feet` (screen
/// pixels), centred horizontally on it and held inside `max_w`.
fn draw_portrait(mesh: &Mesh, feet: Vec2, height: f32, max_w: f32) {
    let (scale, mid_x) = portrait_fit(mesh, height, max_w);
    // A model with no height, or a nonsense fit, draws nothing rather than
    // dividing by it. `is_finite` is what keeps a NaN out of the projection.
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let anchor = vec2(feet.x + mid_x * scale, feet.y);
    // The contact shadow first, as the one cue that the figure stands in its
    // well rather than floating over the card.
    draw_ellipse(
        anchor.x,
        anchor.y,
        (max_w * 0.30).min(height * 0.30).max(0.0),
        (height * 0.05).max(0.0),
        0.0,
        Color::new(0.0, 0.0, 0.0, 0.35),
    );
    draw_model(mesh, anchor, scale);
}

/// The flat-triangle pass every portrait-style draw goes through: sorted
/// back-to-front, projected through [`portrait_point`], one colour per face.
fn draw_model(mesh: &Mesh, anchor: Vec2, scale: f32) {
    PORTRAIT_ORDER.with(|cell| {
        let mut order = cell.borrow_mut();
        portrait_order(mesh, &mut order);
        for t in order.iter().copied() {
            let i = t * 3;
            let (Some(ia), Some(ib), Some(ic)) = (
                mesh.indices.get(i),
                mesh.indices.get(i + 1),
                mesh.indices.get(i + 2),
            ) else {
                continue;
            };
            let (Some(va), Some(vb), Some(vc)) = (
                mesh.vertices.get(*ia as usize),
                mesh.vertices.get(*ib as usize),
                mesh.vertices.get(*ic as usize),
            ) else {
                continue;
            };
            draw_triangle(
                portrait_point(va.position, anchor, scale),
                portrait_point(vb.position, anchor, scale),
                portrait_point(vc.position, anchor, scale),
                vertex_colour(va),
            );
        }
    });
}

/// Scale and centre for a model drawn into a `max_w × max_h` well, measured on
/// the whole bounding box — a model that lies down (the armory's weapon) has
/// no "height" to fit by the way a standing figure does. `(0, origo)` means
/// nothing to draw.
fn fit_model(mesh: &Mesh, max_w: f32, max_h: f32) -> (f32, Vec2) {
    let (mut lo_x, mut hi_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut lo_y, mut hi_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in &mesh.vertices {
        let p = v.position;
        if !(p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        lo_x = lo_x.min(p.x);
        hi_x = hi_x.max(p.x);
        lo_y = lo_y.min(p.y);
        hi_y = hi_y.max(p.y);
    }
    if !lo_x.is_finite() || !lo_y.is_finite() {
        return (0.0, Vec2::ZERO);
    }
    let wide = (hi_x - lo_x).max(1e-3);
    let tall = (hi_y - lo_y).max(1e-3);
    let scale = (sane(max_w, 0.0) / wide).min(sane(max_h, 0.0) / tall);
    if !scale.is_finite() || scale <= 0.0 {
        return (0.0, Vec2::ZERO);
    }
    (scale, vec2((lo_x + hi_x) * 0.5, (lo_y + hi_y) * 0.5))
}

// --- the armory -------------------------------------------------------------

/// One stat line on the armory's right panel.
pub struct ArmoryStat {
    pub label: String,
    pub text: String,
    /// The number the line moves from while a part is being weighed, printed
    /// beside [`ArmoryStat::text`] as "old → new". `None` when the line is a
    /// verdict on the gun in hand rather than a trade the bench is offering.
    pub was: Option<String>,
    /// -1 worse than the build it is measured against, 0 the same, 1 better;
    /// colours the line.
    pub good: i8,
}

/// One candidate in the bench's middle column: a part that fits the focused
/// slot, or "none" for taking the slot's part off.
pub struct ArmoryPart<'a> {
    pub label: &'a str,
    /// Whether this is what the slot already holds.
    pub fitted: bool,
}

/// Which of the bench's two columns the keyboard is working in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArmoryColumn {
    /// The left column: the section, the weapon, the slots — and the exit.
    Slots,
    /// The focused slot's candidates.
    Parts,
}

/// The bench's middle column and the state that goes with it.
pub struct ArmoryBench<'a> {
    /// The candidates for the focused slot, "none" first.
    pub parts: &'a [ArmoryPart<'a>],
    /// The candidate the numbers describe — the one under the pointer, or the
    /// one under the keyboard's caret. `None` leaves the panel on the build in
    /// hand.
    pub cursor: Option<usize>,
    /// Where the caret is, so the middle column can show it even while the
    /// pointer is somewhere else.
    pub column: ArmoryColumn,
}

/// Everything the armory screen draws, assembled by `main` from the live kit.
pub struct ArmoryView<'a> {
    pub weapon: &'a str,
    pub rows: &'a [MenuItem<'a>],
    /// The bench, on the tab that edits a weapon's parts. `None` on the paint
    /// and loadout tabs, which are row lists and nothing more.
    pub bench: Option<&'a ArmoryBench<'a>>,
    pub stats: &'a [ArmoryStat],
    /// The weapon as the armory draws it; `None` draws the panel without it.
    pub preview: Option<&'a Mesh>,
    /// A line under the numbers: the wear tier, or the bench's hint.
    pub note: &'a str,
}

/// Where the armory's pieces landed, grouped by the question a click on them
/// answers: a row of the left column (the exit is the last of them), or a
/// candidate in the bench's middle column. Handing back one flat list is what
/// forced every click to mean "step the row under the cursor".
pub struct ArmoryRects {
    pub rows: Vec<(f32, f32, f32, f32)>,
    pub parts: Vec<(f32, f32, f32, f32)>,
}

/// The armory: on the parts tab a bench of three columns — the slots and what
/// they hold, the focused slot's candidates, and the weapon with the numbers a
/// candidate would move — and on the other tabs the single list of rows they
/// were built around.
pub fn draw_armory(f: &Fonts, view: &ArmoryView<'_>, selected: usize) -> ArmoryRects {
    let w = screen_width();
    let h = screen_height();
    let ui = Ui::new();
    backdrop(&ui);

    // The screen's heading sits over the left column on every tab, so the
    // bench grows a middle column to the right of it rather than moving it.
    let head = h * 0.10;
    let ty = head + ui.s(26.0);
    let cx = w * 0.285;
    tracked_c(f, i18n::t(Key::Armory), cx, ty, ui.s(36.0), INK, ui.s(8.0));
    let rule_y = ty + ui.s(18.0);
    let rw = (w * 0.34).clamp(ui.s(260.0), ui.s(430.0));
    draw_rectangle(cx - rw * 0.5, rule_y, rw, ui.s(2.0), fade(accent(), 0.85));

    match view.bench {
        Some(bench) => draw_bench(
            f,
            view,
            bench,
            selected,
            &ui,
            &BenchLayout::new(&ui, cx, rule_y, head),
        ),
        None => {
            let rows = item_list(f, view.rows, selected, &ui, cx, rule_y + ui.s(40.0), rw);
            armory_panel(f, view, &ui, w * 0.52, w * 0.43, head);
            ArmoryRects {
                rows,
                parts: Vec::new(),
            }
        }
    }
}

/// The bench's layout in pixels: the three columns and the room their rows
/// have. Passed around as one value, because the pieces that draw the columns
/// each need most of it and none of them needs to redo the arithmetic.
struct BenchLayout {
    /// Left edge and width of the slot column, the candidate column, and the
    /// panel.
    slots: (f32, f32),
    parts: (f32, f32),
    panel: (f32, f32),
    /// y of the first row, and the vertical room a column has for its rows.
    top: f32,
    room: f32,
    /// The height of the screen's heading block, which the panel's name line
    /// hangs from.
    head: f32,
}

impl BenchLayout {
    /// Three columns where the other tabs need two: the slot column keeps the
    /// heading's centre line and the panel keeps the armory's right edge, so
    /// the tab still reads as the same bench; the candidates take what is left
    /// between them. The slot column is the wider of the two, because its rows
    /// carry a fitted part's name beside the slot's — "MUZZLE COMPENSATOR" is
    /// the longest line the armory prints.
    fn new(ui: &Ui, cx: f32, rule_y: f32, head: f32) -> BenchLayout {
        let w = screen_width();
        let sw = (w * 0.32).clamp(ui.s(260.0), ui.s(470.0));
        let pw = (w * 0.21).clamp(ui.s(200.0), ui.s(340.0));
        let gap = ui.s(22.0);
        let lx = cx - sw * 0.5;
        let px = lx + sw + gap;
        let panel_x = px + pw + gap;
        let top = rule_y + ui.s(52.0);
        BenchLayout {
            slots: (lx, sw),
            parts: (px, pw),
            panel: (panel_x, (w - panel_x - ui.s(26.0)).max(ui.s(180.0))),
            top,
            room: screen_height() - ui.s(26.0) - top,
            head,
        }
    }
}

/// The bench's three columns: the slots and what they hold, the focused slot's
/// candidates, and the panel with the weapon and the numbers a candidate would
/// move.
fn draw_bench(
    f: &Fonts,
    view: &ArmoryView<'_>,
    bench: &ArmoryBench<'_>,
    selected: usize,
    ui: &Ui,
    layout: &BenchLayout,
) -> ArmoryRects {
    let (lx, sw) = layout.slots;
    let (px, _) = layout.parts;
    let caption_y = layout.top - ui.s(26.0);
    let cap = ui.s(13.0);
    tracked(
        f,
        i18n::t(Key::ArmoryColSlots),
        lx + ui.s(2.0),
        caption_y,
        cap,
        fade(
            INK,
            if bench.column == ArmoryColumn::Slots {
                0.72
            } else {
                0.38
            },
        ),
        ui.s(2.0),
    );
    tracked(
        f,
        i18n::t(Key::ArmoryColParts),
        px + ui.s(2.0),
        caption_y,
        cap,
        fade(
            accent(),
            if bench.column == ArmoryColumn::Parts {
                0.95
            } else {
                0.55
            },
        ),
        ui.s(2.0),
    );

    let rows = item_list(f, view.rows, selected, ui, lx + sw * 0.5, layout.top, sw);
    let parts = part_column(
        f,
        bench.parts,
        bench.cursor,
        bench.column == ArmoryColumn::Parts,
        ui,
        layout,
    );
    armory_panel(f, view, ui, layout.panel.0, layout.panel.1, layout.head);
    ArmoryRects { rows, parts }
}

/// The bench's middle column: one row per candidate for the focused slot,
/// drawn in the same language as the list on its left. The row the numbers
/// belong to is lit, a caret marks the keyboard's row when it has one, and the
/// part the slot already holds is marked — the three things the cycle this
/// replaced could never show.
fn part_column(
    f: &Fonts,
    parts: &[ArmoryPart],
    cursor: Option<usize>,
    caret: bool,
    ui: &Ui,
    layout: &BenchLayout,
) -> Vec<(f32, f32, f32, f32)> {
    let (x, w) = layout.parts;
    let (top, room) = (layout.top, layout.room);
    let (ih, gap) = list_metrics(parts.len(), room, ui.s(60.0));
    let k = (ih / ui.s(50.0)).clamp(0.3, 1.0);
    let mut rects = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let y = top + i as f32 * (ih + gap);
        let lit = cursor == Some(i);
        let fill = if lit {
            Color::new(0.17, 0.12, 0.08, 0.92)
        } else {
            Color::new(0.06, 0.07, 0.09, 0.80)
        };
        draw_rectangle(x, y, w, ih, fill);
        if lit {
            draw_rectangle(x, y, ui.s(4.0), ih, fade(accent(), 1.0));
        }
        draw_rectangle_lines(
            x,
            y,
            w,
            ih,
            1.0,
            if lit {
                fade(accent(), 0.85)
            } else {
                fade(INK, 0.18)
            },
        );

        // The row's number makes the size of the list readable at a glance;
        // the fitted row wears a dot in its place, because it is not one of
        // the choices so much as the answer to "what is on the gun".
        if part.fitted {
            draw_circle(
                x + ui.s(20.0),
                y + ih * 0.5,
                (ui.s(3.2) * k).max(1.5),
                fade(accent(), if lit { 1.0 } else { 0.8 }),
            );
        } else {
            f.text(
                &format!("{:02}", i + 1),
                x + ui.s(14.0),
                y + ih * 0.5 + ui.s(6.0) * k,
                ui.s(14.0) * k,
                fade(INK, 0.35),
            );
        }

        let tag = if part.fitted {
            i18n::t(Key::PartFitted)
        } else {
            ""
        };
        let tag_size = ui.s(12.0) * k;
        // The caret lives at the row's right edge, so the tag it shares the row
        // with keeps its distance — the two are on the same line whenever the
        // caret is on the part the slot already holds, which is where it starts.
        let caret_w = if caret && lit { ui.s(13.0) } else { 0.0 };
        let tag_w = if tag.is_empty() {
            0.0
        } else {
            f.width(tag, tag_size) + ui.s(10.0) + caret_w
        };
        let label = fit(f, part.label, ui.s(19.0) * k, w - ui.s(50.0) - tag_w);
        f.text(
            &label,
            x + ui.s(38.0),
            y + ih * 0.5 + ui.s(7.0) * k,
            ui.s(19.0) * k,
            fade(INK, if lit { 1.0 } else { 0.72 }),
        );
        if !tag.is_empty() {
            f.text_r(
                tag,
                x + w - ui.s(14.0) - caret_w,
                y + ih * 0.5 + ui.s(5.0) * k,
                tag_size,
                fade(accent(), 0.7),
            );
        }
        if caret && lit {
            let mx = x + w - ui.s(12.0);
            let my = y + ih * 0.5;
            draw_triangle(
                vec2(mx, my - ui.s(6.0) * k),
                vec2(mx, my + ui.s(6.0) * k),
                vec2(mx + ui.s(9.0) * k, my),
                fade(accent(), 1.0),
            );
        }
        rects.push((x, y, w, ih));
    }
    rects
}

/// The armory's right-hand panel: the weapon's name, its portrait in the well,
/// and one line per stat under it — the number, and, while a candidate is being
/// weighed, the number it would move from, so the panel answers "what would
/// this part do" and not only "what is it now".
fn armory_panel(f: &Fonts, view: &ArmoryView<'_>, ui: &Ui, x: f32, w: f32, head: f32) {
    let name_y = head + ui.s(44.0);
    f.text_c(view.weapon, x + w * 0.5, name_y, ui.s(26.0), INK);

    let well_top = name_y + ui.s(22.0);
    let well_h = screen_height() * 0.32;
    if let Some(mesh) = view.preview {
        let (scale, mid) = fit_model(mesh, w * 0.96, well_h * 0.86);
        if scale > 0.0 {
            let centre = vec2(x + w * 0.5, well_top + well_h * 0.5);
            draw_model(mesh, centre + mid * scale, scale);
        }
    }

    // A hairline, then one line per stat, the value right-aligned and coloured
    // by which way it moved.
    let stats_top = well_top + well_h + ui.s(4.0);
    draw_rectangle(x, stats_top, w, ui.s(1.0), fade(INK, 0.15));
    let line = ui.s(21.0);
    let size = ui.s(15.0);
    for (i, s) in view.stats.iter().enumerate() {
        let y = stats_top + ui.s(20.0) + i as f32 * line;
        let col = match s.good {
            1 => CYAN,
            -1 => DANGER,
            _ => fade(INK, 0.72),
        };
        f.text(&s.label, x, y, size, fade(INK, 0.55));
        let arrow = match s.good {
            1 => "  ▲",
            -1 => "  ▼",
            _ => "",
        };
        let now = format!("{}{arrow}", s.text);
        let now_w = f.width(&now, size);
        // Both numbers where there is room for both, and the new one alone
        // where there is not: a narrow panel must not run the two together
        // with the label.
        match &s.was {
            Some(was) if f.width(was, size) + now_w < w * 0.62 => {
                let mark = "  →  ";
                let at = x + w - f.width(was, size) - f.width(mark, size) - now_w;
                f.text(was, at, y, size, fade(INK, 0.42));
                f.text(mark, at + f.width(was, size), y, size, fade(INK, 0.42));
                f.text(
                    &now,
                    at + f.width(was, size) + f.width(mark, size),
                    y,
                    size,
                    col,
                );
            }
            _ => f.text(&now, x + w - now_w, y, size, col),
        }
    }
    if !view.note.is_empty() {
        f.text_c(
            view.note,
            x + w * 0.5,
            stats_top + ui.s(22.0) + view.stats.len() as f32 * line + ui.s(12.0),
            ui.s(14.0),
            fade(accent(), 0.85),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole indicator is a bearing mapped onto the ring around the middle of
    /// the screen. Ahead has to come out at the top, the player's right to the
    /// right of it and behind at the bottom; a flipped sign is the difference
    /// between telling the player "shot from the left" and "shot from the right",
    /// which is worse than saying nothing.
    #[test]
    fn a_bearing_points_where_the_hit_came_from() {
        let ahead = hurt_dir(0.0);
        assert!(ahead.x.abs() < 1e-6 && ahead.y < -0.99, "ahead: {ahead:?}");
        let right = hurt_dir(std::f32::consts::FRAC_PI_2);
        assert!(right.x > 0.99 && right.y.abs() < 1e-6, "right: {right:?}");
        let left = hurt_dir(-std::f32::consts::FRAC_PI_2);
        assert!(left.x < -0.99 && left.y.abs() < 1e-6, "left: {left:?}");
        let behind = hurt_dir(std::f32::consts::PI);
        assert!(
            behind.x.abs() < 1e-6 && behind.y > 0.99,
            "behind: {behind:?}"
        );
        // Off the shoulder stays on the side it came from.
        assert!(
            hurt_dir(0.4).x > 0.0 && hurt_dir(0.4).y < 0.0,
            "front right"
        );
        assert!(
            hurt_dir(-0.4).x < 0.0 && hurt_dir(-0.4).y < 0.0,
            "front left"
        );
        // Only the direction matters, so an unwrapped yaw difference is fine:
        // a bearing a full turn out has to land in the same place.
        assert!((hurt_dir(0.9) - hurt_dir(0.9 + std::f32::consts::TAU)).length() < 1e-5);
    }

    /// The doc comment hands the caller a formula for turning the game's own yaw
    /// into a bearing, and the two conventions are not the same one: `yaw` grows
    /// to the left, this angle grows to the right. The formula is worth a test on
    /// its own, because a caller who derives it slightly differently gets an
    /// indicator that is exactly mirrored — confidently wrong in the worst way.
    #[test]
    fn the_documented_bearing_matches_the_game_yaw() {
        use crate::util::yaw_of;

        // Facing down -Z, the player's right hand points along +X.
        let facing = 0.0f32;
        let bearing = |d: Vec3| facing - yaw_of(d);
        assert!(hurt_dir(bearing(vec3(1.0, 0.0, 0.0))).x > 0.99, "right");
        assert!(hurt_dir(bearing(vec3(-1.0, 0.0, 0.0))).x < -0.99, "left");
        assert!(hurt_dir(bearing(vec3(0.0, 0.0, -1.0))).y < -0.99, "ahead");
        assert!(hurt_dir(bearing(vec3(0.0, 0.0, 1.0))).y > 0.99, "behind");

        // Yaw grows the other way, so a quarter turn to the left has to swing a
        // source that was on the right round to dead astern.
        let turned = std::f32::consts::FRAC_PI_2;
        let astern = hurt_dir(turned - yaw_of(vec3(1.0, 0.0, 0.0)));
        assert!(
            astern.x.abs() < 1e-5 && astern.y > 0.99,
            "a left quarter turn put the source at {astern:?}"
        );
    }

    /// Strength may stretch an arc, never steer it: the fan has to stay centred
    /// on its bearing at every value, or a heavy hit would appear to come from a
    /// direction the player is not being shot from. This is also where the two
    /// geometry helpers are checked against each other — the direction the arc
    /// points and the direction the bearing names are the same mapping.
    #[test]
    fn an_arc_stays_centred_on_its_bearing() {
        for b in [-3.0f32, -1.57, -0.6, 0.0, 0.6, 1.57, 3.0] {
            for s in [0.0f32, 0.25, 0.5, 1.0] {
                let (a0, a1) = hurt_arc_angles(b, s);
                assert!(a0 < a1, "arc runs backwards at {b}/{s}: {a0}..{a1}");
                let mid = (a0 + a1) * 0.5;
                let dir = vec2(mid.cos(), mid.sin());
                let want = hurt_dir(b);
                assert!(
                    (dir - want).length() < 1e-5,
                    "arc at {b}/{s} points {dir:?}, bearing is {want:?}"
                );
            }
        }
    }

    /// Strength is read as opacity and as length at the same time, and both have
    /// to be bounded: a span that grew without limit would wrap the ring into a
    /// hit from nowhere, and a NaN strength would reach macroquad as a NaN vertex.
    #[test]
    fn strength_stretches_an_arc_without_escaping_its_bounds() {
        assert!((hurt_span(0.0) - HURT_SPAN_MIN).abs() < 1e-6);
        assert!((hurt_span(1.0) - HURT_SPAN_MAX).abs() < 1e-6);
        assert!(hurt_span(0.5) > hurt_span(0.25) && hurt_span(0.75) > hurt_span(0.5));
        for s in [-5.0f32, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let span = hurt_span(s);
            assert!(
                span.is_finite() && (HURT_SPAN_MIN..=HURT_SPAN_MAX).contains(&span),
                "strength {s} gave a span of {span}"
            );
        }
        assert!((hurt_alpha(1.0) - 1.0).abs() < 1e-6);
        // Even the faintest hit has to be visible, and still not solid.
        assert!((0.15..0.5).contains(&hurt_alpha(0.0)));
        assert!(hurt_alpha(0.75) > hurt_alpha(0.25));
        assert!(hurt_alpha(f32::NAN).is_finite());
    }

    /// A full-strength arc still has to be a fan. Past a quarter turn it would
    /// cover two sides of the ring at once and stop saying which one the shot
    /// came from — which is the only thing this indicator exists to say.
    #[test]
    fn even_the_strongest_hit_stays_a_sector() {
        // Constant arithmetic, so these are compile-time checks: a tuning change
        // that breaks the fan fails the build, not one run of the suite.
        const _: () = assert!(HURT_SPAN_MAX < std::f32::consts::FRAC_PI_2);
        const _: () = assert!(HURT_SPAN_MIN > 0.0);
        const _: () = assert!(
            HURT_PLATE_PAD > 0.0,
            "an arc with no plate reads on nothing"
        );
    }

    /// The ring has to stay outside everything the crosshair draws. At full
    /// spread its arms reach 96 layout units out and add 11.6 more, and the hit
    /// ticks reach 24; an arc from dead ahead landing on the sight the player is
    /// aiming through would be the one way this indicator actively hurts.
    #[test]
    fn the_damage_ring_clears_the_crosshair() {
        let crosshair = 96.0 + 12.0;
        let inner = HURT_RADIUS - HURT_THICK * 0.5;
        assert!(
            inner > crosshair,
            "the ring's inner edge is at {inner}, inside the crosshair's reach"
        );
    }

    /// Colour is the reinforcement and length is the reading, so the ramp has to
    /// be the one the rest of the HUD uses for damage: the base ink at a graze,
    /// the damage red at full strength, and never a third colour of its own.
    #[test]
    fn an_arc_ramps_from_ink_to_the_damage_red() {
        // `mix` interpolates rather than snapping to its ends, so "at full
        // strength it is the damage red" is one rounding step short of `DANGER`.
        let near = |x: Color, y: Color| {
            (x.r - y.r).abs() < 1e-5 && (x.g - y.g).abs() < 1e-5 && (x.b - y.b).abs() < 1e-5
        };
        assert!(near(hurt_colour(0.0), INK));
        assert!(near(hurt_colour(1.0), DANGER));
        let mid = hurt_colour(0.5);
        assert!(
            mid.r > DANGER.r && mid.g > DANGER.g && mid.b > DANGER.b,
            "the halfway colour is not between the two: {mid:?}"
        );
        // A broken strength reads as a graze rather than as a scramble for the palette.
        assert!(near(hurt_colour(f32::NAN), INK));
    }

    /// The kill flash lands in the middle of a firefight, so it may only ever
    /// touch the edges: the player is still shooting at whatever is behind the
    /// effect, and it has to end in nothing rather than on a visible line.
    #[test]
    fn the_kill_flash_never_reaches_the_middle_of_the_screen() {
        // Compile-time: the band width is a constant, and a message with an
        // inline argument is not available to a const panic.
        const _: () = assert!(KILL_DEPTH < 0.35, "the flash must stay off the centre");
        assert_eq!(kill_falloff(0.0), 1.0, "the edge is not the brightest part");
        assert_eq!(kill_falloff(1.0), 0.0);
        assert!(kill_falloff(0.25) > kill_falloff(0.5));
        assert!(kill_falloff(0.5) > kill_falloff(0.75));
        for t in [1.0f32, 1.5, 8.0, f32::NAN, f32::INFINITY] {
            assert_eq!(kill_falloff(t), 0.0, "glow leaked past the band at {t}");
        }
    }

    /// A headshot has to be a different *readout*, not just a different tint: it
    /// is the larger of the two sizes, and it wears the crosshair's headshot
    /// colour so one colour means "headshot" in the whole frame.
    #[test]
    fn a_headshot_readout_is_bigger_and_keeps_the_headshot_colour() {
        let body = damage_number_size(false);
        let head = damage_number_size(true);
        assert!(
            head > body,
            "headshot {head} is not bigger than body {body}"
        );
        for size in [body, head] {
            // `font_px` rounds and clamps whatever it is handed, so a size outside
            // this range would be drawn at one the HUD never chose.
            assert!(
                (8.0..=220.0).contains(&size),
                "a size of {size} is not one the font can honour"
            );
        }
        assert_eq!(damage_number_colour(false), INK);
        assert_eq!(damage_number_colour(true), head_colour());
        assert_ne!(damage_number_colour(true), damage_number_colour(false));
    }

    // --- the manual -------------------------------------------------------

    /// Fixtures for the manual's layout: groups of one, three, six and twelve
    /// rows, which is what it takes to exercise the split, the squeeze and the
    /// drop. `HelpRow` borrows nothing, so these can be plain statics.
    static ROWS1: [HelpRow; 1] = [HelpRow {
        keys: "TAB",
        text: "INVENTORY",
    }];
    static ROWS3: [HelpRow; 3] = [
        HelpRow {
            keys: "W",
            text: "MOVE",
        },
        HelpRow {
            keys: "SHIFT",
            text: "SPRINT",
        },
        HelpRow {
            keys: "CTRL",
            text: "CROUCH",
        },
    ];
    static ROWS6: [HelpRow; 6] = [
        HelpRow {
            keys: "W A S D",
            text: "MOVE",
        },
        HelpRow {
            keys: "SHIFT",
            text: "SPRINT",
        },
        HelpRow {
            keys: "CTRL",
            text: "CROUCH",
        },
        HelpRow {
            keys: "SPACE",
            text: "JUMP",
        },
        HelpRow {
            keys: "Q / E",
            text: "LEAN",
        },
        HelpRow {
            keys: "MOUSE 1",
            text: "FIRE",
        },
    ];
    static ROWS12: [HelpRow; 12] = [
        HelpRow {
            keys: "W A S D",
            text: "MOVE",
        },
        HelpRow {
            keys: "SHIFT",
            text: "SPRINT",
        },
        HelpRow {
            keys: "CTRL",
            text: "CROUCH",
        },
        HelpRow {
            keys: "SPACE",
            text: "JUMP",
        },
        HelpRow {
            keys: "Q / E",
            text: "LEAN",
        },
        HelpRow {
            keys: "MOUSE 1",
            text: "FIRE",
        },
        HelpRow {
            keys: "R",
            text: "RELOAD",
        },
        HelpRow {
            keys: "V",
            text: "MELEE",
        },
        HelpRow {
            keys: "G",
            text: "GRENADE",
        },
        HelpRow {
            keys: "F",
            text: "PICK UP",
        },
        HelpRow {
            keys: "1-4",
            text: "WEAPONS",
        },
        HelpRow {
            keys: "TAB",
            text: "INVENTORY",
        },
    ];
    /// Four groups of six: the shape a real controls page is expected to have.
    static FOUR_TYPICAL: [HelpSection; 4] = [
        HelpSection {
            title: "MOVEMENT",
            rows: &ROWS6,
        },
        HelpSection {
            title: "COMBAT",
            rows: &ROWS6,
        },
        HelpSection {
            title: "INTERFACE",
            rows: &ROWS6,
        },
        HelpSection {
            title: "SURVIVAL",
            rows: &ROWS6,
        },
    ];
    static FOUR_EQUAL: [HelpSection; 4] = [
        HelpSection {
            title: "MOVEMENT",
            rows: &ROWS3,
        },
        HelpSection {
            title: "COMBAT",
            rows: &ROWS3,
        },
        HelpSection {
            title: "INTERFACE",
            rows: &ROWS3,
        },
        HelpSection {
            title: "SURVIVAL",
            rows: &ROWS3,
        },
    ];
    /// One group that is heavier than the other three put together.
    static ONE_LONG: [HelpSection; 2] = [
        HelpSection {
            title: "MOVEMENT",
            rows: &ROWS12,
        },
        HelpSection {
            title: "INTERFACE",
            rows: &ROWS1,
        },
    ];
    static FOUR_LONG: [HelpSection; 4] = [
        HelpSection {
            title: "MOVEMENT",
            rows: &ROWS12,
        },
        HelpSection {
            title: "COMBAT",
            rows: &ROWS12,
        },
        HelpSection {
            title: "INTERFACE",
            rows: &ROWS12,
        },
        HelpSection {
            title: "SURVIVAL",
            rows: &ROWS12,
        },
    ];

    /// The card grid has to put every card on the page, in order, inside the
    /// area it was given. The reading order is down a column and then across,
    /// so the first column of the bestiary is the four archetypes the campaign
    /// meets first.
    #[test]
    fn cards_fill_their_area_in_order() {
        let area = (20.0f32, 30.0, 400.0, 200.0);
        let (rows, cols, gap) = (8usize, 2usize, 10.0f32);
        let first = card_rect(0, rows, cols, area, gap);
        let second = card_rect(1, rows, cols, area, gap);
        let fifth = card_rect(4, rows, cols, area, gap);

        assert!((second.0 - first.0).abs() < 1e-4, "card 1 left its column");
        assert!(second.1 > first.1, "card 1 is not below card 0");
        assert!((fifth.1 - first.1).abs() < 1e-4, "card 4 is not at the top");
        assert!(fifth.0 > first.0, "card 4 did not start a new column");

        for i in 0..rows {
            let (x, y, w, h) = card_rect(i, rows, cols, area, gap);
            assert!(w > 0.0 && h > 0.0, "card {i} has no size: {w}x{h}");
            assert!(
                x >= area.0 - 1e-3
                    && y >= area.1 - 1e-3
                    && x + w <= area.0 + area.2 + 1e-3
                    && y + h <= area.1 + area.3 + 1e-3,
                "card {i} is outside the page: {x},{y} {w}x{h}"
            );
            for j in 0..i {
                let other = card_rect(j, rows, cols, area, gap);
                let apart = x + w <= other.0 + 1e-3
                    || other.0 + other.2 <= x + 1e-3
                    || y + h <= other.1 + 1e-3
                    || other.1 + other.3 <= y + 1e-3;
                assert!(apart, "cards {j} and {i} overlap");
            }
        }

        // One card takes the whole width, and a nonsense area is empty rather
        // than negative or NaN.
        let (_, _, w1, h1) = card_rect(0, 1, 1, area, gap);
        assert!((w1 - area.2).abs() < 1e-4 && (h1 - area.3).abs() < 1e-4);
        for bad in [
            (f32::NAN, f32::NAN, f32::NAN, f32::NAN),
            (0.0, 0.0, -50.0, -50.0),
        ] {
            let (_, _, w, h) = card_rect(3, 8, 2, bad, gap);
            assert!(w >= 0.0 && h >= 0.0, "a broken area gave {w}x{h}");
        }
    }

    /// The grid grows a column before the cards go short: a card the player has
    /// to squint at is worse than a longer page, and the cap is what stops a
    /// short window from turning the page into a wall of slivers.
    #[test]
    fn card_columns_grows_before_the_cards_shrink() {
        // Eight archetype cards, each needing room for a 90-unit portrait.
        assert_eq!(card_columns(8, 490.0, 110.0, 3), 2);
        // Fourteen kit rows, which are happy at the small card size.
        assert_eq!(card_columns(14, 490.0, 46.0, 3), 2);
        // A page that is short on cards keeps one column.
        assert_eq!(card_columns(4, 490.0, 46.0, 3), 1);

        for rows in [0usize, 1, 3, 40] {
            let cols = card_columns(rows, 120.0, 60.0, 3);
            assert!((1..=3).contains(&cols), "rows {rows} gave {cols} columns");
            assert!(cols <= rows.max(1), "rows {rows} gave {cols} columns");
        }
        // Nothing to work with: the cap, and never a zero or a panic.
        for avail in [0.0f32, -100.0, f32::NAN, f32::INFINITY] {
            let cols = card_columns(9, avail, 60.0, 3);
            assert!((1..=3).contains(&cols), "avail {avail} gave {cols} columns");
        }
        assert_eq!(
            card_columns(9, 1000.0, 60.0, 0),
            1,
            "a zero cap means a panic"
        );
    }

    /// A card loses its prose before it loses its name, and a card with no room
    /// at all still reports one line and a usable leading rather than a
    /// negative height.
    #[test]
    fn a_card_keeps_its_name_and_drops_its_lower_lines() {
        // The enemy card at 640x360: four lines, packed into the room available.
        let lines = card_lines(114.0, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
        assert_eq!(lines, 4);
        let lead = card_lead(114.0, MAN_CARD_PAD, lines, MAN_CARD_LEAD, MAN_CARD_MIN_LEAD);
        assert!(
            lead * lines as f32 <= card_room(114.0, MAN_CARD_PAD) + 1e-3,
            "four lines at {lead} do not fit a 114-unit card"
        );

        // The kit card at 640x360: three lines.
        let short = card_lines(60.9, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
        assert!((2..4).contains(&short), "{short} lines in a kit card");

        for h in [0.0f32, -20.0, f32::NAN, f32::INFINITY] {
            let lines = card_lines(h, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
            let lead = card_lead(h, MAN_CARD_PAD, lines, MAN_CARD_LEAD, MAN_CARD_MIN_LEAD);
            assert!(lines >= 1, "a {h}-unit card gave {lines} lines");
            assert!(
                lead.is_finite() && lead >= MAN_CARD_MIN_LEAD,
                "a {h}-unit card gave a leading of {lead}"
            );
        }
        assert_eq!(card_room(f32::NAN, MAN_CARD_PAD), 0.0);
        assert_eq!(card_room(5.0, MAN_CARD_PAD), 0.0);
    }

    /// The portrait's projection: the model is y-up with its feet on zero while
    /// the screen is y-down, so a point above the feet has to come out *above*
    /// the anchor. Getting this sign wrong is a portrait hanging head-down
    /// below its own feet, which no amount of shading covers up.
    #[test]
    fn a_portrait_stands_on_its_anchor_and_faces_the_viewer() {
        let anchor = vec2(100.0, 200.0);
        let scale = 50.0;

        assert_eq!(portrait_point(Vec3::ZERO, anchor, scale), anchor);
        let head = portrait_point(vec3(0.0, 1.8, 0.0), anchor, scale);
        assert!(head.y < anchor.y, "the head is below the feet: {head:?}");
        assert!((head.y - (anchor.y - 90.0)).abs() < 1e-3, "{head:?}");
        assert!((head.x - anchor.x).abs() < 1e-3, "{head:?}");

        // The front view: the camera is on the model's own `-z`, so the model's
        // `+x` (its left) comes out on the left of the card.
        let left = portrait_point(vec3(1.0, 0.0, 0.0), anchor, scale);
        assert!((left.x - (anchor.x - 50.0)).abs() < 1e-3, "{left:?}");
        // Depth is not a screen direction under an orthographic view.
        assert_eq!(portrait_point(vec3(0.0, 0.0, 3.0), anchor, scale), anchor);
        // A broken vertex must not become a NaN pixel.
        assert_eq!(
            portrait_point(vec3(f32::NAN, f32::NAN, 0.0), anchor, scale),
            anchor
        );
    }

    /// The fit: a model twice as tall wants half the scale, a model that stands
    /// off-centre is centred by the fit rather than by its caller, and a model
    /// with no height at all fits at zero instead of dividing by it.
    #[test]
    fn a_portrait_is_scaled_by_its_own_height() {
        let mut mb = crate::mesh::MeshBuilder::new();
        mb.add_box(vec3(-1.0, 0.0, -0.5), vec3(1.0, 2.0, 0.5), WHITE);
        let mesh = mb.finish();
        let (scale, mid) = portrait_fit(&mesh, 90.0, 1000.0);
        assert!((scale - 45.0).abs() < 1e-3, "scale {scale}");
        assert!(mid.abs() < 1e-3, "centre {mid}");

        let mut mb = crate::mesh::MeshBuilder::new();
        mb.add_box(vec3(1.0, 0.0, -0.5), vec3(3.0, 1.8, 0.5), WHITE);
        let mesh = mb.finish();
        let (scale, mid) = portrait_fit(&mesh, 90.0, 1000.0);
        assert!((scale - 50.0).abs() < 1e-3, "scale {scale}");
        assert!((mid - 2.0).abs() < 1e-3, "centre {mid}");

        // The well holds a wide model back even though height is what a
        // portrait is measured by.
        let (scale, _) = portrait_fit(&mesh, 90.0, 20.0);
        assert!((scale - 10.0).abs() < 1e-3, "width-limited scale {scale}");

        let empty = crate::mesh::MeshBuilder::new().finish();
        assert_eq!(portrait_fit(&empty, 90.0, 100.0), (0.0, 0.0));
        let flat = {
            let mut mb = crate::mesh::MeshBuilder::new();
            mb.add_box(vec3(-1.0, 0.0, -1.0), vec3(1.0, 0.0, 1.0), WHITE);
            mb.finish()
        };
        assert_eq!(portrait_fit(&flat, 90.0, 100.0), (0.0, 0.0));
        assert_eq!(portrait_fit(&empty, 0.0, 100.0), (0.0, 0.0));
        assert_eq!(portrait_fit(&empty, f32::NAN, 100.0), (0.0, 0.0));
    }

    /// The painter's algorithm, which is the whole of the depth test here: with
    /// no depth buffer, the order *is* the depth test. The far side of a box has
    /// to be drawn before the near side, or the near faces are painted over by
    /// the ones behind them and the box reads as broken.
    #[test]
    fn a_portrait_draws_far_before_near() {
        let mut mb = crate::mesh::MeshBuilder::new();
        mb.add_box(vec3(-0.5, 0.0, -0.5), vec3(0.5, 1.8, 0.5), WHITE);
        let mesh = mb.finish();

        let mut order = Vec::new();
        portrait_order(&mesh, &mut order);
        assert_eq!(order.len(), mesh.indices.len() / 3);
        assert_eq!(order.len(), 12, "a box is six quads");

        let mut prev = f32::INFINITY;
        for t in &order {
            let d = tri_depth(&mesh, *t);
            assert!(
                d <= prev + 1e-5,
                "triangle {t} at depth {d} came after {prev}"
            );
            prev = d;
        }
        // The camera is on `-z`, so the far face is the `+z` one and it is
        // drawn first; the near face is drawn last, over it.
        assert!(
            tri_depth(&mesh, order[0]) > 0.4,
            "the far face is not drawn first: {}",
            tri_depth(&mesh, order[0])
        );
        assert!(
            tri_depth(&mesh, order[order.len() - 1]) < -0.4,
            "the near face is not drawn last: {}",
            tri_depth(&mesh, order[order.len() - 1])
        );

        // Every triangle appears exactly once, whatever the sort does.
        let mut seen = order.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), order.len());

        // Two triangles drawn by hand, one in front of the other, come back in
        // the order a painter would use.
        let pair = Mesh {
            vertices: vec![
                Vertex {
                    position: vec3(0.0, 1.0, -1.0),
                    uv: Vec2::ZERO,
                    color: [255, 255, 255, 255],
                    normal: Vec4::ZERO,
                },
                Vertex {
                    position: vec3(1.0, 0.0, -1.0),
                    uv: Vec2::ZERO,
                    color: [255, 255, 255, 255],
                    normal: Vec4::ZERO,
                },
                Vertex {
                    position: vec3(0.0, 0.0, -1.0),
                    uv: Vec2::ZERO,
                    color: [255, 255, 255, 255],
                    normal: Vec4::ZERO,
                },
                Vertex {
                    position: vec3(0.0, 1.0, 1.0),
                    uv: Vec2::ZERO,
                    color: [0, 0, 0, 255],
                    normal: Vec4::ZERO,
                },
                Vertex {
                    position: vec3(1.0, 0.0, 1.0),
                    uv: Vec2::ZERO,
                    color: [0, 0, 0, 255],
                    normal: Vec4::ZERO,
                },
                Vertex {
                    position: vec3(0.0, 0.0, 1.0),
                    uv: Vec2::ZERO,
                    color: [0, 0, 0, 255],
                    normal: Vec4::ZERO,
                },
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            texture: None,
        };
        portrait_order(&pair, &mut order);
        assert_eq!(order, vec![1, 0], "the far triangle is not drawn first");

        // A mesh whose indices point nowhere still sorts and draws nothing,
        // rather than panicking on the way to the screen.
        let broken = Mesh {
            vertices: Vec::new(),
            indices: vec![0, 1, 2],
            texture: None,
        };
        portrait_order(&broken, &mut order);
        assert_eq!(order.len(), 1);
        assert!(tri_depth(&broken, 0).is_infinite());
    }

    /// The pager is the one label on a page whose number the caller cannot see:
    /// a page out of range is clamped rather than printed, because "0 / 0" and
    /// "7 / 3" read as a bug in the manual rather than as a stuck page.
    #[test]
    fn the_pager_never_prints_a_page_that_cannot_exist() {
        let mut buf = String::new();
        assert_eq!(page_label(&mut buf, 2, 5), "2 / 5");
        assert_eq!(page_label(&mut buf, 1, 1), "1 / 1");
        assert_eq!(page_label(&mut buf, 0, 3), "1 / 3");
        assert_eq!(page_label(&mut buf, 9, 3), "3 / 3");
        assert_eq!(page_label(&mut buf, 1, 0), "1 / 1");
        // A count at the end of the range is a caller bug rather than a stuck
        // page, and all the pager owes it is not to panic.
        assert!(page_label(&mut buf, usize::MAX, usize::MAX).ends_with(&usize::MAX.to_string()));
        // The buffer keeps its allocation, so a second call reuses it.
        let capacity = buf.capacity();
        assert_eq!(page_label(&mut buf, 2, 2), "2 / 2");
        assert_eq!(buf.capacity(), capacity);
    }

    /// Long text is cut to the width it has and never past it, and the cut is
    /// visible. The measure here is a character count, so the arithmetic is
    /// exact and the test needs no font.
    #[test]
    fn long_card_text_is_cut_to_its_width() {
        let measure = |s: &str, _size: f32| s.chars().count() as f32;
        let mut buf = String::new();
        // Text that fits comes back whole.
        assert_eq!(
            fit_into(&mut buf, "JUGGERNAUT", 1.0, 12.0, &measure),
            "JUGGERNAUT"
        );
        // Text that does not ends on `..`, stays inside the width, and is a
        // prefix of what it was cut from.
        for (text, w) in [
            ("JUGGERNAUT", 8.0f32),
            ("重装兵重装兵", 5.0),
            ("BREACHER 12", 6.0),
        ] {
            let out = fit_into(&mut buf, text, 1.0, w, &measure);
            assert!(
                out.chars().count() as f32 <= w.max(2.0),
                "{out:?} is wider than {w}"
            );
            assert!(out.ends_with(".."), "{out:?} is not marked as cut");
            let kept = out.trim_end_matches('.');
            assert!(
                text.starts_with(kept),
                "{out:?} is not a prefix of {text:?}"
            );
            assert!(!kept.is_empty(), "{out:?} kept nothing of a {w}-wide line");
        }
        // A width with no room for even the dots still terminates.
        let out = fit_into(&mut buf, "W", 1.0, 0.0, &measure);
        assert_eq!(out, "..");
    }

    /// Wrapping is what keeps the goal paragraph inside its column: no line is
    /// wider than the room it was given, every line is a slice of the text, and
    /// the lines are in order.
    #[test]
    fn the_goal_wraps_inside_its_column() {
        let measure = |s: &str, _size: f32| s.chars().count() as f32;
        let mut lines = Vec::new();
        let text = "CLEAR THE SECTOR AND REACH THE EXTRACTION POINT";
        wrap_ranges(text, 1.0, 12.0, &measure, &mut lines);
        assert!(lines.len() > 1, "the text should not have fitted one line");

        let mut end = 0usize;
        for (a, b) in &lines {
            let line = &text[*a..*b];
            assert!(
                line.chars().count() as f32 <= 12.0,
                "{line:?} is wider than the column"
            );
            assert!(*a >= end && b > a, "lines ran backwards at {a}..{b}");
            end = *b;
        }
        // Every word survives, once, in order.
        let joined: String = lines
            .iter()
            .map(|(a, b)| &text[*a..*b])
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, text);

        // A word wider than a whole line is cut rather than dropped, and a
        // nonsense width can only produce one character per line — never a loop
        // that does not end.
        wrap_ranges("SUPERCALIFRAGILISTIC", 1.0, 5.0, &measure, &mut lines);
        assert_eq!(lines.len(), 4);
        for (a, b) in &lines {
            assert!(b > a, "a line made no progress: {a}..{b}");
        }
        wrap_ranges("ABC", 1.0, f32::NAN, &measure, &mut lines);
        assert_eq!(lines.len(), 3);
        wrap_ranges("", 1.0, 10.0, &measure, &mut lines);
        assert!(lines.is_empty());
    }

    /// The two columns of the controls page are balanced by height: groups of
    /// similar size split down the middle, and one long group keeps a column to
    /// itself rather than being cut in two.
    #[test]
    fn the_controls_page_balances_its_two_columns() {
        assert_eq!(column_split(&FOUR_EQUAL, MAN_ROW_H), 2);
        assert_eq!(column_split(&ONE_LONG, MAN_ROW_H), 1);
        // Nothing to balance: one group goes left, none at all does not panic.
        assert_eq!(column_split(&FOUR_EQUAL[..1], MAN_ROW_H), 1);
        assert_eq!(column_split(&[], MAN_ROW_H), 0);
        // A taller row height makes the same groups heavier, and the split
        // still has to leave both columns with something.
        for row_h in [MAN_ROW_MIN, MAN_ROW_H, 40.0, f32::NAN] {
            let split = column_split(&FOUR_EQUAL, row_h);
            assert!((1..FOUR_EQUAL.len()).contains(&split), "row_h {row_h}");
        }
        // The heights the split is made from are the heights the page draws.
        let tall = section_height(12, MAN_ROW_H);
        let short = section_height(1, MAN_ROW_H);
        assert!(tall > short && short > 0.0);
        assert_eq!(section_height(0, MAN_ROW_H), MAN_SEC_HEAD + MAN_SEC_GAP);
    }

    /// A page with room to spare keeps the spacing it was authored with, a
    /// tight one is squeezed to the floor but never past it, and a nonsense one
    /// falls back to the authored height rather than to a negative one.
    #[test]
    fn a_short_page_squeezes_its_rows_without_escaping_its_bounds() {
        assert_eq!(
            fitted_row_h(4, 100.0, 900.0, MAN_ROW_H, MAN_ROW_MIN),
            MAN_ROW_H
        );
        let tight = fitted_row_h(30, 100.0, 600.0, MAN_ROW_H, MAN_ROW_MIN);
        assert!((tight - 500.0 / 30.0).abs() < 1e-3, "{tight}");
        assert!(tight < MAN_ROW_H && tight > MAN_ROW_MIN, "{tight}");
        // Tighter than the floor allows: the floor, and no further.
        assert_eq!(
            fitted_row_h(40, 100.0, 600.0, MAN_ROW_H, MAN_ROW_MIN),
            MAN_ROW_MIN
        );
        assert_eq!(
            fitted_row_h(0, 100.0, 600.0, MAN_ROW_H, MAN_ROW_MIN),
            MAN_ROW_H
        );
        for avail in [f32::NAN, f32::INFINITY, -50.0, 0.0] {
            let h = fitted_row_h(6, 100.0, avail, MAN_ROW_H, MAN_ROW_MIN);
            assert!(
                h.is_finite() && (MAN_ROW_MIN..=MAN_ROW_H).contains(&h),
                "avail {avail} gave {h}"
            );
        }
        // A caller that swaps the two bounds gets a height, not a panic.
        let h = fitted_row_h(6, 0.0, 600.0, 10.0, 20.0);
        assert!(h.is_finite() && h >= 20.0, "{h}");
    }

    /// The key column is sized from the widest binding on the page, and capped
    /// so the description always keeps the larger share of the row.
    #[test]
    fn the_key_column_gives_the_description_the_wider_half() {
        assert!((key_column(60.0, 600.0, MAN_KEY_MIN) - 72.0).abs() < 1e-3);
        let capped = key_column(500.0, 400.0, MAN_KEY_MIN);
        assert!(capped <= 400.0 * 0.46 + 1e-3, "{capped}");
        assert!(capped >= MAN_KEY_MIN);
        assert_eq!(key_column(f32::NAN, 0.0, MAN_KEY_MIN), MAN_KEY_MIN);
        assert!(key_column(-5.0, 300.0, MAN_KEY_MIN) >= MAN_KEY_MIN);
        assert!(key_column(10_000.0, 300.0, MAN_KEY_MIN) >= MAN_KEY_MIN);
    }

    /// The windows the manual is actually read in. 1280x720 is what the layout
    /// is authored against; 640x360 is the smallest the game is played at, and
    /// there `Ui` has already hit its own floor (0.55), so the layout units
    /// available are *fewer* than at 1280x720 rather than merely smaller —
    /// about 1164 x 655 against 1280 x 720. What fits at 640x360 fits anywhere
    /// above it.
    /// The settings screen is the long list, and the window sizes the game is
    /// actually played in have to show every row of it: at 720p the layout
    /// space is 720 units and at 1080p it is the same 720 (the `Ui` scale is
    /// capped at 1.5), so the honest thing to check is the units.
    #[test]
    fn a_long_menu_fits_the_window_it_is_played_in() {
        // 720 units of layout space, minus the header and the bottom margin:
        // roughly what the settings screen gets at 1280x720 and at 1920x1080.
        let room = 720.0 - 342.0 - 26.0;
        for n in 1..=12usize {
            let (ih, gap) = list_metrics(n, room, 60.0);
            assert!(ih >= 20.0 && gap >= 0.0, "n={n}: {ih} / {gap}");
            let bottom = n as f32 * (ih + gap);
            assert!(
                bottom <= room + 0.001,
                "n={n}: the list needs {bottom} of {room} units"
            );
            assert!(ih.is_finite() && gap.is_finite());
        }

        // A short list keeps the authored pitch exactly.
        let (ih, gap) = list_metrics(4, room, 60.0);
        assert_eq!(ih, 50.0);
        assert_eq!(gap, 10.0);

        // Nonsense in, sane out: no NaN, no negative row.
        for (n, room) in [(0usize, 0.0f32), (10, f32::NAN), (10, -50.0)] {
            let (ih, gap) = list_metrics(n, room, 60.0);
            assert!(ih > 0.0 && ih.is_finite(), "n={n} room={room}: {ih}");
            assert!(gap >= 0.0 && gap.is_finite(), "n={n} room={room}: {gap}");
        }
    }

    #[test]
    fn the_manual_fits_the_windows_it_is_played_in() {
        let tips = [
            "RELOAD BEHIND COVER",
            "WATCH THE FLANKS",
            "MOVE AFTER FIRING",
            "SAVE THE MEDKITS",
        ];
        for (w, h) in [(1280.0f32, 720.0f32), (640.0, 360.0)] {
            let scale = (h / REF_H).clamp(0.55, 1.5);
            let (uw, uh) = (w / scale, h / scale);
            let (bx, by, bw, bh) = manual_body(uw, uh);
            assert!(bw > 0.0 && bh > 0.0, "{w}x{h}: no body: {bw}x{bh}");

            // The bestiary: eight cards, and every card wants a portrait.
            let ecols = card_columns(
                8,
                bh,
                MAN_PORTRAIT_H + MAN_CARD_PAD * 2.0,
                MAN_CARD_MAX_COLS,
            );
            let (_, _, ecw, ech) = card_rect(7, 8, ecols, (bx, by, bw, bh), MAN_CARD_GAP);
            let elines = card_lines(ech, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
            assert_eq!(ecols, 2, "{w}x{h}: the bestiary wanted {ecols} columns");
            assert!(
                ech >= MAN_PORTRAIT_H + MAN_CARD_PAD * 2.0 - 1e-3,
                "{w}x{h}: an enemy card is {ech} tall, too short for a portrait"
            );
            assert!(elines >= 3, "{w}x{h}: an enemy card holds {elines} lines");

            // The armoury: fourteen rows, no portraits.
            let kcols = card_columns(14, bh, MAN_CARD_MIN, MAN_CARD_MAX_COLS);
            let (_, _, kcw, kch) = card_rect(13, 14, kcols, (bx, by, bw, bh), MAN_CARD_GAP);
            let klines = card_lines(kch, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
            assert_eq!(kcols, 2, "{w}x{h}: the kit page wanted {kcols} columns");
            assert!(kch >= MAN_CARD_MIN - 1e-3, "{w}x{h}: a kit card is {kch}");
            assert!(klines >= 2, "{w}x{h}: a kit card holds {klines} lines");
            assert!(
                ecw > 0.0 && kcw > 0.0 && kcw + MAN_CARD_GAP + ecw <= bw + 1e-3,
                "{w}x{h}: the two columns do not fit side by side"
            );

            // The controls page: four groups of six keys, plus four notes.
            let split = column_split(&FOUR_TYPICAL, MAN_ROW_H);
            let (left, right) = FOUR_TYPICAL.split_at(split);
            let rows_left: usize = left.iter().map(|s| s.rows.len()).sum();
            let rows_right: usize = right.iter().map(|s| s.rows.len()).sum::<usize>() + tips.len();
            let fixed_left = left.len() as f32 * (MAN_SEC_HEAD + MAN_SEC_GAP);
            let fixed_right =
                right.len() as f32 * (MAN_SEC_HEAD + MAN_SEC_GAP) + MAN_TIPS_HEAD + MAN_SEC_GAP;
            let tall_left = fixed_left + rows_left as f32 * MAN_ROW_H;
            let tall_right = fixed_right + rows_right as f32 * MAN_ROW_H;
            let (rows_max, fixed_max) = if tall_left >= tall_right {
                (rows_left, fixed_left)
            } else {
                (rows_right, fixed_right)
            };
            let goal = MAN_GOAL_LABEL + MAN_GOAL_MAX as f32 * MAN_GOAL_LEAD + MAN_SEC_GAP;
            let avail = (bh - goal).max(0.0);
            let row_h = fitted_row_h(rows_max, fixed_max, avail, MAN_ROW_H, MAN_ROW_MIN);
            let used = fixed_max + rows_max as f32 * row_h;
            assert_eq!(
                split,
                2,
                "{w}x{h}: four equal groups split {split}/{}",
                FOUR_TYPICAL.len() - split
            );
            assert!(
                used <= avail + 1e-3,
                "{w}x{h}: the controls page overflows by {} units",
                used - avail
            );
            assert!(
                row_h >= MAN_ROW_MIN,
                "{w}x{h}: rows squeezed to {row_h}, under the {MAN_ROW_MIN} floor"
            );

            println!(
                "{w}x{h}: units {uw:.0}x{uh:.0} | body {bw:.0}x{bh:.0} | \
                 bestiary {ecols} cols, card {ecw:.0}x{ech:.0}, {elines} lines | \
                 kit {kcols} cols, card {kcw:.0}x{kch:.0}, {klines} lines | \
                 controls {split}+{} groups, rows {rows_left}/{rows_right}, \
                 row_h {row_h:.1}, column {used:.0}/{avail:.0}",
                FOUR_TYPICAL.len() - split
            );
        }

        // Below anything the game is played at, and above it: nothing may come
        // out negative, NaN or off the page. 320x200 is where `Ui`'s 0.55 floor
        // squeezes the whole layout into 582x364 units; 4K is where the 1.5
        // ceiling stops the layout growing.
        for (w, h) in [(320.0f32, 200.0f32), (4000.0, 2400.0)] {
            let scale = (h / REF_H).clamp(0.55, 1.5);
            let (uw, uh) = (w / scale, h / scale);
            let (bx, by, bw, bh) = manual_body(uw, uh);
            assert!(bw > 0.0 && bh > 0.0, "{w}x{h}: body {bw}x{bh}");
            assert!(uw / scale >= 0.0 && uh.is_finite());
            let cols = card_columns(
                8,
                bh,
                MAN_PORTRAIT_H + MAN_CARD_PAD * 2.0,
                MAN_CARD_MAX_COLS,
            );
            for i in 0..8 {
                let (x, y, cw, ch) = card_rect(i, 8, cols, (bx, by, bw, bh), MAN_CARD_GAP);
                assert!(cw > 0.0 && ch > 0.0 && x.is_finite() && y.is_finite());
                assert!(
                    x >= bx - 1e-3
                        && y >= by - 1e-3
                        && x + cw <= bx + bw + 1e-3
                        && y + ch <= by + bh + 1e-3,
                    "{w}x{h}: card {i} is off the page"
                );
                let lines = card_lines(ch, MAN_CARD_PAD, MAN_CARD_MIN_LEAD, 4);
                let lead = card_lead(ch, MAN_CARD_PAD, lines, MAN_CARD_LEAD, MAN_CARD_MIN_LEAD);
                assert!(lines >= 1 && lead.is_finite() && lead > 0.0, "{w}x{h}");
            }
            // The portrait's well is never a negative height, whatever the card
            // shrinks to.
            let (_, _, _, ch) = card_rect(0, 8, cols, (bx, by, bw, bh), MAN_CARD_GAP);
            let ph = MAN_PORTRAIT_H.min((ch - MAN_CARD_PAD * 2.0).max(0.0) - 6.0);
            assert!(ph.is_finite());
            let row_h = fitted_row_h(
                FOUR_LONG.len() * 12,
                100.0,
                (bh * 0.5).max(0.0),
                MAN_ROW_H,
                MAN_ROW_MIN,
            );
            assert!(row_h.is_finite() && (MAN_ROW_MIN..=MAN_ROW_H).contains(&row_h));
        }
    }
}
