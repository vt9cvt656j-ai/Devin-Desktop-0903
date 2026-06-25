//! OS-level "computer use": the agent can SEE the whole screen and control the
//! real mouse & keyboard to operate ANY desktop app — not just the browser.
//! Cross-platform via pure-Rust `enigo` (input) + `xcap` (screen capture).
//!
//! This is powerful and sensitive (it drives the user's actual machine). It runs
//! input on a blocking thread; the prompt instructs the agent to act carefully and
//! to look (screenshot) before and after every action.

use std::time::Duration;

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use serde::Serialize;

/// Screen state returned after every action: size (the agent's coordinate space)
/// plus a fresh full-screen screenshot fed back to the model as vision.
#[derive(Serialize)]
pub struct ScreenState {
    width: u32,
    height: u32,
    screenshot: String, // data:image/png;base64,...
}

fn capture_png() -> Result<(u32, u32, Vec<u8>), String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let mon = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or("没有可用的显示器")?;
    let img = mon.capture_image().map_err(|e| e.to_string())?; // RgbaImage
    let (w, h) = (img.width(), img.height());
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok((w, h, buf.into_inner()))
}

fn screen_state() -> Result<ScreenState, String> {
    let (width, height, png) = capture_png()?;
    Ok(ScreenState {
        width,
        height,
        screenshot: format!("data:image/png;base64,{}", crate::capture::b64(&png)),
    })
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
        e.move_mouse(x, y, Coordinate::Abs).map_err(|er| er.to_string())
    })
    .await
}

/// Move to (x, y) and click. `button` = left | right | middle (default left).
#[tauri::command]
pub async fn computer_click(x: i32, y: i32, button: Option<String>) -> Result<ScreenState, String> {
    run(move || {
        let mut e = make_enigo()?;
        e.move_mouse(x, y, Coordinate::Abs).map_err(|er| er.to_string())?;
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
        e.move_mouse(x, y, Coordinate::Abs).map_err(|er| er.to_string())?;
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
