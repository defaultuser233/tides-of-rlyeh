//! The Windows accessibility shortcuts a shooter has to opt out of.
//!
//! Three features are armed by the same keys this game binds: Sticky Keys by
//! tapping Shift five times, Filter Keys by holding Right Shift for eight
//! seconds, and Toggle Keys by holding NumLock for five. Each answers with a
//! dialog offering to turn it on, which takes focus away mid-fight — and Shift
//! is sprint here, so the first of them is one short burst of sprint taps from
//! firing every match.
//!
//! The IME was the same story from the other direction (see `disable_ime`) and
//! the fix is the same in spirit: tell Windows this process does not want the
//! shortcut. There is no supported per-application switch, so the flags are
//! cleared for the session and put back when the game exits. `SPIF_UPDATEINIFILE`
//! is deliberately never passed, so nothing is written to the user's profile and
//! even a crash leaves the setting no more changed than a logoff would.
//!
//! The three declarations below are hand-written rather than taken from a
//! bindings crate. This project carries exactly one dependency on purpose —
//! `macroquad`, which is the renderer — and one function is not worth the second
//! one. The whole surface is a single `extern` block whose signature is quoted
//! from the documented prototype, and every call site names the invariant it
//! relies on.
//!
//! Constants are quoted from `winuser.h`, not recalled:
//! `SPI_SETSTICKYKEYS` is `0x003B`, not the `0x000C` the neighbouring names
//! suggest, and getting that one wrong would have silently disabled nothing.

/// `SPI_*`, from `winuser.h:5139-5148`.
const SPI_GETFILTERKEYS: u32 = 0x0032;
const SPI_SETFILTERKEYS: u32 = 0x0033;
const SPI_GETTOGGLEKEYS: u32 = 0x0034;
const SPI_SETTOGGLEKEYS: u32 = 0x0035;
const SPI_GETSTICKYKEYS: u32 = 0x003A;
const SPI_SETSTICKYKEYS: u32 = 0x003B;

/// `SKF_HOTKEYACTIVE` / `FKF_HOTKEYACTIVE` / `TKF_HOTKEYACTIVE` — all three are
/// bit 2, which is why one mask covers every block below.
const HOTKEY_ACTIVE: u32 = 0x0000_0004;

/// `FILTERKEYS` is six `DWORD`s (`winuser.h:5645-5652`); `STICKYKEYS` and
/// `TOGGLEKEYS` are two. Every field is a `u32`, so one slice type stands in
/// for all three — the layout is what the API reads, and it is identical.
const FILTER_WORDS: usize = 6;

// The one Windows API this module needs. The signature is the documented one:
// `BOOL SystemParametersInfoW(UINT uiAction, UINT uiParam, PVOID pvParam, UINT fWinIni)`.
#[link(name = "user32")]
unsafe extern "system" {
    fn SystemParametersInfoW(action: u32, param: u32, data: *mut u8, win_ini: u32) -> i32;
}

/// `SYSTEMTIME`, from `minwinbase.h:83-94`: eight `WORD`s in this order. Only
/// `wHour` and `wMinute` are read here, but the whole block has to be the right
/// size because Windows fills all of it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SystemTime {
    year: u16,
    month: u16,
    weekday: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemTime(out: *mut SystemTime);
    fn GetLocalTime(out: *mut SystemTime);
}

/// How far the machine's clock is from UTC, in seconds, positive east of it.
///
/// Read as the difference between the two clocks Windows keeps, which is the only
/// way to ask this without a date library: `GetSystemTime` is UTC and
/// `GetLocalTime` is the same instant in the user's own zone. `None` when the two
/// cannot be read at all, in which case a caller should print UTC and say so.
///
/// The minute is the resolution of both clocks, so the difference is exact to the
/// minute — which is all a date stamp in a corner needs.
pub fn utc_offset_seconds() -> Option<i64> {
    // SAFETY: both calls take a pointer to one `SYSTEMTIME` and write all eight
    // words of it; `SystemTime` above is exactly that layout and outlives the
    // call.
    let (utc, local) = unsafe {
        let mut utc = SystemTime::default();
        let mut local = SystemTime::default();
        GetSystemTime(&mut utc);
        GetLocalTime(&mut local);
        (utc, local)
    };
    let (Some(utc), Some(local)) = (civil_seconds(utc), civil_seconds(local)) else {
        return None;
    };
    Some(local - utc)
}

/// A `SYSTEMTIME` as seconds since 1970-01-01, which is a civil date and not an
/// instant: the two calls above are compared with each other and never with a
/// clock, so the calendar conversion has to be the *calendar's* and not a
/// count of elapsed days.
fn civil_seconds(t: SystemTime) -> Option<i64> {
    let (y, m, d) = (t.year as i64, t.month as i64, t.day as i64);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1970 {
        return None;
    }
    // Days from 1970-01-01 to the first of the month, by Hinnant's `days_from_
    // civil`: the year is shifted to start in March so the leap day lands at the
    // end of it, and the era arithmetic is what makes it work for any year.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + t.hour as i64 * 3600 + t.minute as i64 * 60)
}

/// Read a settings block, clear its hotkey bit, write it back, and hand back
/// what it said before.
///
/// Fails quietly: a Windows that refuses the call is a Windows where the dialog
/// still appears, which is where the game started.
fn suppress(get: u32, set: u32, block: &mut [u32]) -> Option<Vec<u32>> {
    // `uiParam` is the size of the block *in bytes*. Passing the word count
    // instead is accepted by the compiler, rejected by Windows, and the only
    // symptom is that the hotkey stays armed.
    let size = std::mem::size_of_val(block) as u32;
    block[0] = size;
    // SAFETY: `block` is a live slice of `size` bytes held for the duration of
    // the call, and its first word has just been set to that size, which is the
    // `cbSize` every one of these blocks is required to carry. `get` is one of
    // the `SPI_GET*` codes and `block` is the matching block: two `u32`s for
    // STICKYKEYS and TOGGLEKEYS, six for FILTERKEYS, which is the only thing
    // that distinguishes them.
    let ok = unsafe { SystemParametersInfoW(get, size, block.as_mut_ptr().cast(), 0) };
    if ok == 0 {
        return None;
    }
    let original = block.to_vec();
    block[1] &= !HOTKEY_ACTIVE;
    // SAFETY: as above; `set` is the `SPI_SET*` code paired with `get`, and the
    // block is unchanged apart from the flag just cleared.
    let ok = unsafe { SystemParametersInfoW(set, size, block.as_mut_ptr().cast(), 0) };
    if ok == 0 {
        return None;
    }
    Some(original)
}

fn restore(set: u32, original: &[u32]) {
    let mut block = original.to_vec();
    let size = std::mem::size_of_val(&block[..]) as u32;
    block[0] = size;
    // SAFETY: a fresh `Vec` of the same length as the block that was read, with
    // its `cbSize` word restored; `set` is the `SPI_SET*` code it came from.
    // The API reads `size` bytes and copies them out, so the local going out of
    // scope afterwards is not a concern.
    unsafe { SystemParametersInfoW(set, size, block.as_mut_ptr().cast(), 0) };
}

/// Holds the three settings as they were, and puts them back on the way out.
///
/// The guard exists so the restore runs on a clean quit. A process that is
/// killed outright leaves them cleared for the rest of the session, which is
/// the same thing a logoff would do and is the price of the trick having no
/// per-process form.
pub struct QuietKeys {
    sticky: Option<Vec<u32>>,
    filter: Option<Vec<u32>>,
    toggle: Option<Vec<u32>>,
}

impl QuietKeys {
    /// Clear the hotkey activation on all three features.
    pub fn new() -> Self {
        let sticky = suppress(SPI_GETSTICKYKEYS, SPI_SETSTICKYKEYS, &mut [0; 2]);
        let filter = suppress(SPI_GETFILTERKEYS, SPI_SETFILTERKEYS, &mut [0; FILTER_WORDS]);
        let toggle = suppress(SPI_GETTOGGLEKEYS, SPI_SETTOGGLEKEYS, &mut [0; 2]);
        Self {
            sticky,
            filter,
            toggle,
        }
    }
}

impl Drop for QuietKeys {
    fn drop(&mut self) {
        if let Some(original) = &self.sticky {
            restore(SPI_SETSTICKYKEYS, original);
        }
        if let Some(original) = &self.filter {
            restore(SPI_SETFILTERKEYS, original);
        }
        if let Some(original) = &self.toggle {
            restore(SPI_SETTOGGLEKEYS, original);
        }
    }
}
