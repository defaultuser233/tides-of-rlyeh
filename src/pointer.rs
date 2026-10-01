//! Pointer confinement and recentring.
//!
//! ## Why this module exists
//!
//! macroquad derives its mouse delta from the cursor's *absolute* position, and
//! it only refreshes that position while the cursor is **not** grabbed:
//!
//! ```text
//! fn mouse_motion_event(&mut self, x: f32, y: f32) {
//!     if !context.cursor_grabbed { context.mouse_position = Vec2::new(x, y); }
//! }
//! ```
//!
//! So macroquad's own `set_cursor_grab(true)` freezes mouse look completely —
//! movement keeps working while the view stops turning. We therefore confine the
//! pointer through **miniquad** directly, which is what actually issues
//! `ClipCursor`, and leave macroquad's `cursor_grabbed` flag alone.
//!
//! Confining alone is not enough: `ClipCursor` pins the cursor to the window, so
//! after one sweep of the mouse it reaches an edge and the delta stays zero
//! forever. The pointer is recentred instead, every frame.
//!
//! Naively cancelling each frame's movement does **not** work: the cancelling
//! move is itself picked up as the next frame's delta, and the feedback loop
//! drifts until the view is pointing at the sky. Two things avoid that here:
//!
//! * the caller owns the delta baseline (`Game::last_mouse`) and rebases it to
//!   the centre we just moved to, so the recentring contributes a delta of zero;
//! * the move is made with calls that are **not** subject to pointer
//!   acceleration — `SetCursorPos` on Windows and `XWarpPointer` on X11 — so the
//!   cursor really does land where we say it does.

use macroquad::prelude::*;

/// Confine the pointer to the window without disabling macroquad's tracking.
pub fn clip(on: bool) {
    macroquad::miniquad::window::set_cursor_grab(on);
}

pub fn show(shown: bool) {
    macroquad::miniquad::window::show_mouse(shown);
}

/// Moves the pointer to the centre of the window.
///
/// `current` is the pointer's position in logical window coordinates, as
/// returned by `macroquad::input::mouse_position`. On success the pointer now
/// sits at `vec2(screen_width() * 0.5, screen_height() * 0.5)`, which is what
/// the caller should rebase its delta against. Returns `false` when the
/// platform call is unavailable, in which case nothing moved.
#[cfg(target_os = "windows")]
pub fn recenter(current: Vec2) -> Option<Vec2> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    unsafe extern "system" {
        fn GetCursorPos(p: *mut Point) -> i32;
        fn SetCursorPos(x: i32, y: i32) -> i32;
    }

    let scale = macroquad::miniquad::window::dpi_scale();
    let mut p = Point { x: 0, y: 0 };
    // SAFETY: `Point` is `#[repr(C)]` with two `i32`s, which is Win32's `POINT`;
    // `GetCursorPos` writes exactly that through the pointer, which is valid and
    // aligned for the duration of the call. A zero return means the call failed
    // and nothing was written, which is handled rather than assumed away.
    if unsafe { GetCursorPos(&mut p) } == 0 {
        return None;
    }
    // The move is expressed relative to where the cursor actually is, so the
    // window's origin on screen never has to be known. It is rounded ONCE, and
    // that same rounded value is what we report back: assuming the cursor
    // landed on the exact centre leaves a sub-pixel residual that repeats every
    // frame and slowly rotates the view.
    let dx = ((screen_width() * 0.5 - current.x) * scale).round();
    let dy = ((screen_height() * 0.5 - current.y) * scale).round();
    // Absolute and unaccelerated, so there is no drift to accumulate.
    // SAFETY: `SetCursorPos` takes two plain `i32`s — no pointer, no lifetime —
    // so the only obligation is that the values fit the parameter type, which
    // the casts above enforce.
    if unsafe { SetCursorPos(p.x + dx as i32, p.y + dy as i32) } == 0 {
        return None;
    }
    Some(vec2(current.x + dx / scale, current.y + dy / scale))
}

/// X11 has no link-time dependency in this build (macroquad `dlopen`s libX11 at
/// runtime), so the two functions we need are resolved the same way. A null
/// `dest_w` makes the coordinates relative to the pointer's current position,
/// and `XWarpPointer` is not subject to pointer acceleration, so the move is
/// exact.
#[cfg(target_os = "linux")]
pub fn recenter(current: Vec2) -> Option<Vec2> {
    use std::ffi::{c_char, c_int, c_void};
    use std::sync::OnceLock;

    type Display = c_void;
    type XWarpPointer =
        unsafe extern "C" fn(*mut Display, u64, u64, c_int, c_int, u32, u32, c_int, c_int) -> c_int;

    unsafe extern "C" {
        fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_LAZY: c_int = 1;

    static HANDLE: OnceLock<Option<(usize, usize)>> = OnceLock::new();

    // SAFETY: `dlopen` and `dlsym` take a NUL-terminated C string — `c"…"`
    // literals are exactly that — and return opaque handles or null, both of
    // which are checked before use. The two results are stored as `usize`
    // because a raw pointer cannot be sent across the `OnceLock`; nothing is
    // dereferenced here.
    let loaded = HANDLE.get_or_init(|| unsafe {
        let lib = dlopen(c"libX11.so.6".as_ptr(), RTLD_LAZY);
        if lib.is_null() {
            return None;
        }
        let open = dlsym(lib, c"XOpenDisplay".as_ptr());
        let warp = dlsym(lib, c"XWarpPointer".as_ptr());
        if open.is_null() || warp.is_null() {
            return None;
        }
        // SAFETY: `open` was resolved from libX11 under the name
        // `XOpenDisplay` and checked non-null; the signature is the one X11
        // declares for it — `Display *XOpenDisplay(const char *display_name)`.
        let open: unsafe extern "C" fn(*const c_char) -> *mut Display = std::mem::transmute(open);
        let dpy = open(std::ptr::null());
        if dpy.is_null() {
            return None;
        }
        Some((warp as usize, dpy as usize))
    });

    let Some((warp, dpy)) = *loaded else {
        return None;
    };
    // SAFETY: the address came from `dlsym("XWarpPointer")` and was checked
    // non-null, and libX11 is `dlopen`ed without a matching `dlclose`, so it
    // outlives every call made through this pointer. The signature is X11's,
    // with `Window` being `unsigned long`: nine arguments, the first a
    // `Display *`. This is the one assertion in the file a compiler cannot
    // check, and getting it wrong corrupts the stack rather than failing.
    let warp: XWarpPointer = unsafe { std::mem::transmute(warp) };
    let scale = macroquad::miniquad::window::dpi_scale();
    // A null `dest_w` makes the coordinates relative to the pointer's current
    // position, and `XWarpPointer` is not subject to pointer acceleration.
    let dx = ((screen_width() * 0.5 - current.x) * scale).round();
    let dy = ((screen_height() * 0.5 - current.y) * scale).round();
    // SAFETY: `dpy` is the non-null `Display *` from `XOpenDisplay` and `warp`
    // is the checked `XWarpPointer` above; both outlive the call. The remaining
    // arguments are plain integers, and the null `dest_w` that makes the move
    // relative is the documented meaning of a zero window, not an accident.
    unsafe {
        warp(
            dpy as *mut Display,
            0,
            0,
            0,
            0,
            0,
            0,
            dx as c_int,
            dy as c_int,
        );
    }
    Some(vec2(current.x + dx / scale, current.y + dy / scale))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn recenter(_current: Vec2) -> Option<Vec2> {
    None
}
