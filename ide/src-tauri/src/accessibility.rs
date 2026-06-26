//! macOS-only: read the frontmost app's interactive UI elements via the System
//! Events accessibility API, driven through `osascript` (JavaScript for
//! Automation). No unsafe FFI and no extra crates — the Rust side is just a
//! bounded subprocess + JSON parse, so it compiles and is checkable everywhere.
//!
//! Best-effort and TIME-BOUNDED: if Accessibility permission is off, the app
//! doesn't expose a tree, or enumeration is slow, it returns nothing and the
//! caller falls back to the coordinate grid (no regression, never hangs).

use serde::Deserialize;

/// One interactive element, positions in SCREEN POINTS (top-left origin) as
/// reported by System Events. The caller maps these into the screenshot's space.
#[derive(Deserialize)]
pub struct UiElement {
    #[serde(rename = "ref")]
    pub ref_: u32,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[cfg(target_os = "macos")]
pub fn read_ui_elements() -> Vec<UiElement> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // JXA: enumerate the front window's interactive descendants with role +
    // position + size + a label. Wrapped so any failure yields '[]'.
    let script = r##"(function () {
  try {
    var se = Application('System Events');
    var procs = se.applicationProcesses.whose({ frontmost: true });
    if (!procs.length) return '[]';
    var proc = procs[0];
    var wins = proc.windows;
    if (!wins.length) return '[]';
    var all;
    try { all = wins[0].entireContents(); } catch (e) { return '[]'; }
    var want = { AXButton:1, AXTextField:1, AXTextArea:1, AXCheckBox:1, AXRadioButton:1, AXPopUpButton:1, AXMenuButton:1, AXComboBox:1, AXLink:1, AXSlider:1, AXDisclosureTriangle:1, AXSegmentedControl:1, AXTabGroup:1, AXSearchField:1, AXStepper:1 };
    var out = [], i = 0;
    for (var k = 0; k < all.length; k++) {
      if (i >= 50) break;
      var el = all[k], role;
      try { role = el.role(); } catch (e) { continue; }
      if (!want[role]) continue;
      var p, s;
      try { p = el.position(); s = el.size(); } catch (e) { continue; }
      if (!p || !s || s[0] < 2 || s[1] < 2) continue;
      var t = '';
      try { t = el.title() || ''; } catch (e) {}
      if (!t) { try { t = el.description() || ''; } catch (e) {} }
      if (!t) { try { var v = el.value(); if (typeof v === 'string') t = v; } catch (e) {} }
      out.push({ ref: i, role: role.replace('AX', ''), text: String(t).slice(0, 50), x: p[0], y: p[1], w: s[0], h: s[1] });
      i++;
    }
    return JSON.stringify(out);
  } catch (e) { return '[]'; }
})()"##;

    let mut child = match Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Bound it — entireContents() can be slow on big windows; never block the UI.
    let deadline = Instant::now() + Duration::from_millis(3000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Vec::new();
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => return Vec::new(),
        }
    }

    let mut s = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut s);
    }
    serde_json::from_str::<Vec<UiElement>>(s.trim()).unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
pub fn read_ui_elements() -> Vec<UiElement> {
    Vec::new()
}
