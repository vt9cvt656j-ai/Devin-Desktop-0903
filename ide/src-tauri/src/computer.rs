//! OS-level "computer use": the agent can SEE the whole screen and control the
//! real mouse & keyboard to operate ANY desktop app — not just the browser.
//! Cross-platform via pure-Rust `enigo` (input) + `xcap` (screen capture).
//!
//! This is powerful and sensitive (it drives the user's actual machine). It runs
//! input on a blocking thread; the prompt instructs the agent to act carefully and
//! to look (screenshot) before and after every action.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use serde::Serialize;

/// Whether to overlay the coordinate grid. ON for normal agent control (accurate
/// clicking), turned OFF while recording a user-facing demo so frames stay clean.
static DRAW_GRID: AtomicBool = AtomicBool::new(true);

/// Screen state returned after every action: size (the agent's coordinate space)
/// plus a fresh full-screen screenshot fed back to the model as vision.
#[derive(Serialize)]
pub struct ScreenState {
    width: u32,
    height: u32,
    screenshot: String, // data:image/png;base64,...
}

/// Cap the screenshot width — keeps the JPEG small AND defines the coordinate
/// space the agent works in. Clicks are scaled back to real pixels (see `to_real`).
const SHOT_W: u32 = 1366;

fn primary_monitor() -> Result<xcap::Monitor, String> {
    let mons = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let idx = mons
        .iter()
        .position(|m| m.is_primary().unwrap_or(false))
        .unwrap_or(0);
    mons.into_iter()
        .nth(idx)
        .ok_or_else(|| "没有可用的显示器".to_string())
}

// 3x5 bitmap font for digits 0-9 (each row's low 3 bits = left/mid/right pixels).
// Lets us draw coordinate labels with zero font dependencies.
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 2, 2, 2],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

fn blend_px(img: &mut image::RgbaImage, x: u32, y: u32, c: [u8; 3], a: f32) {
    if x >= img.width() || y >= img.height() {
        return;
    }
    let p = img.get_pixel(x, y).0;
    img.put_pixel(
        x,
        y,
        image::Rgba([
            (p[0] as f32 * (1.0 - a) + c[0] as f32 * a) as u8,
            (p[1] as f32 * (1.0 - a) + c[1] as f32 * a) as u8,
            (p[2] as f32 * (1.0 - a) + c[2] as f32 * a) as u8,
            255,
        ]),
    );
}

/// Draw a number at (x,y) — a dark translucent plate for legibility on any
/// background, then white digits via the bitmap font, scaled by `s`.
fn put_number(img: &mut image::RgbaImage, x: u32, y: u32, n: u32, s: u32) {
    let txt = n.to_string();
    let dw = 3 * s + s; // digit cell + spacing
    let w = txt.len() as u32 * dw + s;
    let h = 5 * s + 2 * s;
    for by in 0..h {
        for bx in 0..w {
            blend_px(img, x + bx, y + by, [0, 0, 0], 0.6);
        }
    }
    let mut cx = x + s;
    for ch in txt.bytes() {
        let d = (ch.wrapping_sub(b'0')) as usize;
        if d < 10 {
            let pat = DIGITS[d];
            for (r, &row) in pat.iter().enumerate() {
                for c in 0..3u32 {
                    if (row >> (2 - c)) & 1 == 1 {
                        for yy in 0..s {
                            for xx in 0..s {
                                blend_px(img, cx + c * s + xx, y + s + r as u32 * s + yy, [255, 255, 255], 1.0);
                            }
                        }
                    }
                }
            }
        }
        cx += dw;
    }
}

/// Overlay a faint coordinate grid + axis labels so the model can read off
/// accurate click coordinates (the #1 fix for computer-use misclicks).
fn draw_grid(img: &mut image::RgbaImage, step: u32) {
    let (w, h) = (img.width(), img.height());
    let line = [0u8, 200, 255];
    let mut x = step;
    while x < w {
        for y in 0..h {
            blend_px(img, x, y, line, 0.16);
        }
        put_number(img, x + 2, 2, x, 2);
        x += step;
    }
    let mut y = step;
    while y < h {
        for xx in 0..w {
            blend_px(img, xx, y, line, 0.16);
        }
        put_number(img, 2, y + 2, y, 2);
        y += step;
    }
}

fn screen_state() -> Result<ScreenState, String> {
    let mon = primary_monitor()?;
    let img = mon.capture_image().map_err(|e| e.to_string())?; // RgbaImage
    let (rw, rh) = (img.width(), img.height());
    // Reported coordinate space = the (possibly downscaled) screenshot size, so
    // what the agent sees matches the coordinates it gives. The screenshot is
    // downscaled + JPEG-compressed so a multi-MB grab can't blow the AI body
    // limit (413). click/move scale these coords back to real pixels.
    let (width, height) = if rw > SHOT_W {
        (SHOT_W, ((rh as u64 * SHOT_W as u64) / rw.max(1) as u64) as u32)
    } else {
        (rw, rh)
    };
    // Resize to the reported size, draw the coordinate grid (so labels match the
    // coordinate space the agent works in), then JPEG-encode.
    let mut rgba = if rw > SHOT_W {
        image::DynamicImage::ImageRgba8(img)
            .resize(width, height, image::imageops::FilterType::Triangle)
            .to_rgba8()
    } else {
        img
    };
    if DRAW_GRID.load(Ordering::Relaxed) {
        draw_grid(&mut rgba, 100);
    }
    let screenshot =
        crate::capture::jpeg_data_url(image::DynamicImage::ImageRgba8(rgba), width, 65)?;
    Ok(ScreenState {
        width,
        height,
        screenshot,
    })
}

/// Scale a point from the reported (downscaled) coordinate space to real pixels.
fn to_real(x: i32, y: i32) -> (i32, i32) {
    let real_w = xcap::Monitor::all()
        .ok()
        .and_then(|m| {
            let idx = m.iter().position(|x| x.is_primary().unwrap_or(false)).unwrap_or(0);
            m.into_iter().nth(idx)
        })
        .and_then(|m| m.width().ok())
        .unwrap_or(0);
    if real_w > SHOT_W {
        let s = real_w as f64 / SHOT_W as f64;
        ((x as f64 * s).round() as i32, (y as f64 * s).round() as i32)
    } else {
        (x, y)
    }
}

fn make_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| e.to_string())
}

/// Map a key name to an enigo Key. Single chars become Unicode keys.
fn parse_key(s: &str) -> Option<Key> {
    Some(match s.to_lowercase().as_str() {
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "cmd" | "command" | "meta" | "super" | "win" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        other => {
            let mut chars = other.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None; // multi-char, unknown name
            }
            Key::Unicode(c)
        }
    })
}

async fn run<F>(f: F) -> Result<ScreenState, String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || -> Result<ScreenState, String> {
        f()?;
        std::thread::sleep(Duration::from_millis(350)); // let the UI react before we look
        screen_state()
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Toggle the coordinate-grid overlay. The frontend turns it OFF while recording
/// a user-facing demo (clean frames) and back ON after.
#[tauri::command]
pub fn computer_set_grid(on: bool) {
    DRAW_GRID.store(on, Ordering::Relaxed);
}

/// Full-screen screenshot (no action) — look at the whole machine.
#[tauri::command]
pub async fn computer_screenshot() -> Result<ScreenState, String> {
    tauri::async_runtime::spawn_blocking(screen_state)
        .await
        .map_err(|e| e.to_string())?
}

/// Move the mouse to absolute (x, y).
#[tauri::command]
pub async fn computer_move(x: i32, y: i32) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        let (rx, ry) = to_real(x, y);
        e.move_mouse(rx, ry, Coordinate::Abs).map_err(|er| er.to_string())
    })
    .await
}

/// Move to (x, y) and click. `button` = left | right | middle (default left).
#[tauri::command]
pub async fn computer_click(x: i32, y: i32, button: Option<String>) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        let (rx, ry) = to_real(x, y);
        e.move_mouse(rx, ry, Coordinate::Abs).map_err(|er| er.to_string())?;
        let b = match button.as_deref() {
            Some("right") => Button::Right,
            Some("middle") => Button::Middle,
            _ => Button::Left,
        };
        e.button(b, Direction::Click).map_err(|er| er.to_string())
    })
    .await
}

/// Move to (x, y) and double-click.
#[tauri::command]
pub async fn computer_double_click(x: i32, y: i32) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        let (rx, ry) = to_real(x, y);
        e.move_mouse(rx, ry, Coordinate::Abs).map_err(|er| er.to_string())?;
        e.button(Button::Left, Direction::Click)
            .map_err(|er| er.to_string())?;
        e.button(Button::Left, Direction::Click)
            .map_err(|er| er.to_string())
    })
    .await
}

/// Type a string at the current focus.
#[tauri::command]
pub async fn computer_type(text: String) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        e.text(&text).map_err(|er| er.to_string())
    })
    .await
}

/// Press a key or a chord like "ctrl+c", "cmd+space", "enter".
#[tauri::command]
pub async fn computer_key(combo: String) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        let parts: Vec<&str> = combo
            .split('+')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return Err("空按键".into());
        }
        let keys: Vec<Key> = parts
            .iter()
            .map(|p| parse_key(p))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("无法识别按键: {combo}"))?;
        let (last, mods) = keys.split_last().unwrap();
        for k in mods {
            e.key(*k, Direction::Press).map_err(|er| er.to_string())?;
        }
        let main_res = e.key(*last, Direction::Click).map_err(|er| er.to_string());
        for k in mods.iter().rev() {
            let _ = e.key(*k, Direction::Release);
        }
        main_res
    })
    .await
}

/// Scroll vertically (positive = down, negative = up), in notches.
#[tauri::command]
pub async fn computer_scroll(amount: i32) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        e.scroll(amount, Axis::Vertical).map_err(|er| er.to_string())
    })
    .await
}
