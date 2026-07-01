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
    /// Interactive elements detected via accessibility (macOS), with a `ref` and
    /// the click-ready CENTER coordinate in the reported space — matches the
    /// numbered marks drawn on the screenshot (desktop Set-of-Mark). Empty when
    /// unavailable (then the screenshot still has the coordinate grid).
    elements: Vec<ScreenElement>,
}

#[derive(Serialize)]
pub struct ScreenElement {
    #[serde(rename = "ref")]
    ref_: u32,
    role: String,
    text: String,
    x: u32, // center, reported space (ready to click)
    y: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    value: String,
    #[serde(default, skip_serializing_if = "is_true")]
    enabled: bool,
}

fn is_true(v: &bool) -> bool {
    *v
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
    let scale = mon.scale_factor().unwrap_or(1.0) as f64; // points → pixels (Retina)
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
    let mut rgba = if rw > SHOT_W {
        image::DynamicImage::ImageRgba8(img)
            .resize(width, height, image::imageops::FilterType::Triangle)
            .to_rgba8()
    } else {
        img
    };

    // DRAW_GRID off ⇒ recording a clean demo: no grid, no marks, and skip the
    // (slow) accessibility query entirely.
    let draw = DRAW_GRID.load(Ordering::Relaxed);
    let mut elements: Vec<ScreenElement> = Vec::new();
    if draw {
        draw_grid(&mut rgba, 100);
        // True desktop Set-of-Mark: pull the frontmost app's interactive elements
        // and draw a numbered badge on each. A screen POINT maps to a downscaled
        // pixel by `scale * reported/real` (the scale cancels the Retina factor).
        let sx = scale * width as f64 / rw.max(1) as f64;
        let sy = scale * height as f64 / rh.max(1) as f64;
        for e in crate::accessibility::read_ui_elements() {
            let rx = e.x * sx;
            let ry = e.y * sy;
            let rwd = e.w * sx;
            let rhd = e.h * sy;
            if rx < 0.0 || ry < 0.0 || rx as u32 >= width || ry as u32 >= height {
                continue;
            }
            // Draw numbered badges for the first 200 elements only (interactive
            // controls come first in the priority-sorted list). The full array
            // still includes ALL elements — the agent can click any ref, not just
            // the badged ones.
            if e.ref_ < 200 {
                put_number(&mut rgba, rx as u32, ry as u32, e.ref_, 2);
            }
            elements.push(ScreenElement {
                ref_: e.ref_,
                role: e.role,
                text: e.text,
                x: (rx + rwd / 2.0).round().max(0.0) as u32,
                y: (ry + rhd / 2.0).round().max(0.0) as u32,
                value: e.value,
                enabled: e.enabled,
            });
        }
        // Vision OCR: when AX tree is sparse (self-drawn apps), OCR the screen for text
        let ax_count = elements.len();
        if ax_count < 20 {
            let ax_texts: std::collections::HashSet<String> = elements
                .iter()
                .filter(|e| !e.text.is_empty())
                .map(|e| e.text.to_lowercase())
                .collect();
            for e in crate::accessibility::read_ocr_elements() {
                if !e.text.is_empty() && ax_texts.contains(&e.text.to_lowercase()) {
                    continue;
                }
                let rx = e.x * sx;
                let ry = e.y * sy;
                let rwd = e.w * sx;
                let rhd = e.h * sy;
                if rx < 0.0 || ry < 0.0 || rx as u32 >= width || ry as u32 >= height {
                    continue;
                }
                let ref_ = elements.len() as u32;
                if ref_ < 200 {
                    put_number(&mut rgba, rx as u32, ry as u32, ref_, 2);
                }
                elements.push(ScreenElement {
                    ref_,
                    role: e.role,
                    text: e.text,
                    x: (rx + rwd / 2.0).round().max(0.0) as u32,
                    y: (ry + rhd / 2.0).round().max(0.0) as u32,
                    value: e.value,
                    enabled: e.enabled,
                });
            }
        }
    }

    let screenshot =
        crate::capture::jpeg_data_url(image::DynamicImage::ImageRgba8(rgba), width, 65)?;
    Ok(ScreenState {
        width,
        height,
        screenshot,
        elements,
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
        std::thread::sleep(Duration::from_millis(250)); // let the UI react before we read nodes
        node_state() // LIGHT: nodes only, no JPEG — acting shouldn't cost a screenshot each time
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Like `run`, but executes the input closure on the MAIN thread. macOS keyboard input
/// (enigo's `key` path queries TSM / the input-source list) ASSERTS the main thread —
/// running it on a spawn_blocking worker traps with SIGTRAP and kills the whole app
/// (`dispatch_assert_queue_fail`). So keyboard ops are dispatched to the main run loop;
/// we then settle + screenshot off-main. Mouse/scroll use CGEvents and are main-safe
/// off-thread, so they keep using `run`.
async fn run_kbd<F>(app: tauri::AppHandle, f: F) -> Result<ScreenState, String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ScreenState, String> {
        // Wait for the main-thread key op (and propagate its error), then settle + read nodes.
        rx.recv()
            .map_err(|_| "键盘操作未返回（主线程繁忙？）".to_string())??;
        std::thread::sleep(Duration::from_millis(250));
        node_state() // LIGHT: nodes only, no JPEG
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

/// Drag with the left button held from (x,y) to (to_x,to_y) — for sliders, reordering,
/// drag-and-drop, canvas painting. Moves in small steps so drop targets register the motion.
#[tauri::command]
pub async fn computer_drag(x: i32, y: i32, tx: i32, ty: i32) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        let (sx, sy) = to_real(x, y);
        let (dx, dy) = to_real(tx, ty);
        e.move_mouse(sx, sy, Coordinate::Abs).map_err(|er| er.to_string())?;
        e.button(Button::Left, Direction::Press).map_err(|er| er.to_string())?;
        let steps = 14;
        for i in 1..=steps {
            let ix = sx + (dx - sx) * i / steps;
            let iy = sy + (dy - sy) * i / steps;
            e.move_mouse(ix, iy, Coordinate::Abs).map_err(|er| er.to_string())?;
        }
        e.button(Button::Left, Direction::Release).map_err(|er| er.to_string())
    })
    .await
}

/// Type a string at the current focus. Runs on the main thread (macOS keyboard/TSM).
#[tauri::command]
pub async fn computer_type(app: tauri::AppHandle, text: String) -> Result<ScreenState, String> {
    run_kbd(app, move || {
        let mut e = make_enigo()?;
        e.text(&text).map_err(|er| er.to_string())
    })
    .await
}

/// Press a key or a chord like "ctrl+c", "cmd+space", "enter". Runs on the main thread
/// (enigo's macOS key path queries TSM, which traps if called off the main thread).
#[tauri::command]
pub async fn computer_key(app: tauri::AppHandle, combo: String) -> Result<ScreenState, String> {
    run_kbd(app, move || {
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

// ===== NODE-FIRST (no vision): act on accessibility elements directly by ref =====

/// Enumerate the frontmost app's interactive nodes (ref/role/text + click-ready center)
/// WITHOUT taking a screenshot — mirrors screen_state's element mapping so cached refs stay
/// in the same coordinate space. Returns (width, height, elements).
fn enumerate_nodes(with_ocr: bool) -> Result<(u32, u32, Vec<ScreenElement>), String> {
    let mon = primary_monitor()?;
    let scale = mon.scale_factor().unwrap_or(1.0) as f64;
    let rw = mon.width().map_err(|e| e.to_string())? as f64;
    let rh = mon.height().map_err(|e| e.to_string())? as f64;
    let width = if rw as u32 > SHOT_W { SHOT_W } else { rw as u32 };
    let height = if rw as u32 > SHOT_W {
        ((rh * SHOT_W as f64) / rw.max(1.0)) as u32
    } else {
        rh as u32
    };
    let sx = scale * width as f64 / rw.max(1.0);
    let sy = scale * height as f64 / rh.max(1.0);
    let mut out = Vec::new();
    for e in crate::accessibility::read_ui_elements() {
        let rx = e.x * sx;
        let ry = e.y * sy;
        let rwd = e.w * sx;
        let rhd = e.h * sy;
        out.push(ScreenElement {
            ref_: e.ref_,
            role: e.role,
            text: e.text,
            x: (rx + rwd / 2.0).round().max(0.0) as u32,
            y: (ry + rhd / 2.0).round().max(0.0) as u32,
            value: e.value,
            enabled: e.enabled,
        });
    }
    if with_ocr && out.len() < 20 {
        let ax_texts: std::collections::HashSet<String> = out
            .iter()
            .filter(|e| !e.text.is_empty())
            .map(|e| e.text.to_lowercase())
            .collect();
        for e in crate::accessibility::read_ocr_elements() {
            if !e.text.is_empty() && ax_texts.contains(&e.text.to_lowercase()) {
                continue;
            }
            let rx = e.x * sx;
            let ry = e.y * sy;
            let rwd = e.w * sx;
            let rhd = e.h * sy;
            out.push(ScreenElement {
                ref_: out.len() as u32,
                role: e.role,
                text: e.text,
                x: (rx + rwd / 2.0).round().max(0.0) as u32,
                y: (ry + rhd / 2.0).round().max(0.0) as u32,
                value: e.value,
                enabled: e.enabled,
            });
        }
    }
    Ok((width, height, out))
}

/// The LIGHT result returned after a single action: node list + screen size, but NO JPEG
/// (empty `screenshot`). This is the whole point — acting shouldn't cost a screenshot every
/// time. The agent calls action:"screenshot" only when it actually needs to SEE the pixels.
fn node_state() -> Result<ScreenState, String> {
    let (width, height, elements) = enumerate_nodes(false)?;
    Ok(ScreenState {
        width,
        height,
        screenshot: String::new(),
        elements,
    })
}

/// Text-only node dump (NO screenshot) — fast act-and-verify. Desktop twin of `browser nodes`.
#[tauri::command]
pub async fn computer_nodes() -> Result<Vec<ScreenElement>, String> {
    tauri::async_runtime::spawn_blocking(|| enumerate_nodes(true).map(|(_, _, els)| els))
        .await
        .map_err(|e| e.to_string())?
}

/// AXPress a node by ref — a REAL accessibility action (no mouse move, no pixel guess).
/// Returns JSON {ok, role, name, value}; on Err/ok:false the JS falls back to a pixel click.
#[tauri::command]
pub async fn computer_press(node: u32) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::accessibility::perform_ax_action(node, "press", None)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Set a node's value (type into a field by ref via accessibility, no pixel focus).
#[tauri::command]
pub async fn computer_set_value(node: u32, text: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::accessibility::perform_ax_action(node, "set_value", Some(&text))
    })
    .await
    .map_err(|e| e.to_string())?
}
