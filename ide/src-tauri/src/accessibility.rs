//! macOS-only: read the frontmost app's interactive UI elements via the System
//! Events accessibility API, driven through `osascript` (JavaScript for
//! Automation). No unsafe FFI and no extra crates — the Rust side is just a
//! bounded subprocess + JSON parse, so it compiles and is checkable everywhere.
//!
//! Best-effort and TIME-BOUNDED: if Accessibility permission is off, the app
//! doesn't expose a tree, or enumeration is slow, it returns nothing and the
//! caller falls back to the coordinate grid (no regression, never hangs).

use serde::Deserialize;

fn default_true() -> bool {
    true
}

/// One UI element from the accessibility tree, positions in SCREEN POINTS
/// (top-left origin). The caller maps these into the screenshot's space.
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
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[cfg(target_os = "macos")]
pub fn read_ui_elements() -> Vec<UiElement> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // DEEP JXA: enumerate ALL accessibility elements from the frontmost app —
    // every role, all windows (up to 5), with value + enabled state.
    // Priority tiers: interactive controls → text labels → structural containers → rest.
    // This captures elements that the old 24-role filter missed (AXStaticText labels,
    // AXGroup containers, AXToolbar, AXTable, etc.). Cap 500, timeout 6s.
    let script = r##"(function () {
  try {
    var se = Application('System Events');
    var procs = se.applicationProcesses.whose({ frontmost: true });
    if (!procs.length) return '[]';
    var proc = procs[0];
    var CAP = 500;
    var T1 = { AXButton:1,AXTextField:1,AXTextArea:1,AXCheckBox:1,AXRadioButton:1,AXPopUpButton:1,AXMenuButton:1,AXComboBox:1,AXLink:1,AXSlider:1,AXDisclosureTriangle:1,AXSegmentedControl:1,AXTabGroup:1,AXSearchField:1,AXStepper:1,AXIncrementor:1,AXColorWell:1,AXMenuItem:1,AXTab:1,AXDateField:1,AXSecureTextField:1 };
    var T2 = { AXStaticText:1,AXHeading:1 };
    var T3 = { AXGroup:1,AXScrollArea:1,AXSplitGroup:1,AXToolbar:1,AXList:1,AXTable:1,AXOutline:1,AXSheet:1,AXDialog:1,AXBrowser:1,AXDrawer:1,AXLayoutArea:1,AXMatte:1,AXRuler:1,AXSplitter:1,AXGrowArea:1 };
    var a1=[],a2=[],a3=[],a4=[];
    var label = function(el){
      var t='';
      try{t=el.title()||'';}catch(e){}
      if(!t){try{t=el.description()||'';}catch(e){}}
      if(!t){try{t=el.help()||'';}catch(e){}}
      if(!t){try{var v=el.value();if(typeof v==='string')t=v;}catch(e){}}
      return String(t).slice(0,80);
    };
    var gv = function(el){try{var v=el.value();return v!=null?String(v).slice(0,120):'';}catch(e){return '';}};
    var en = function(el){try{return el.enabled()!==false;}catch(e){return true;}};
    var take = function(el,role){
      var p,s; try{p=el.position();s=el.size();}catch(e){return;}
      if(!p||!s||s[0]<2||s[1]<2) return;
      var rec={role:role.replace('AX',''),text:label(el),x:p[0],y:p[1],w:s[0],h:s[1],value:gv(el),enabled:en(el)};
      if(T1[role])a1.push(rec);else if(T2[role])a2.push(rec);else if(T3[role])a3.push(rec);else a4.push(rec);
    };
    var wins; try{wins=proc.windows;}catch(e){wins=[];}
    for(var wi=0;wi<wins.length&&wi<5;wi++){
      var all; try{all=wins[wi].entireContents();}catch(e){all=[];}
      for(var k=0;k<all.length;k++){
        if(a1.length+a2.length+a3.length+a4.length>=CAP*3) break;
        var el=all[k],role; try{role=el.role();}catch(e){continue;}
        take(el,role);
      }
    }
    try{
      var items=proc.menuBars[0].menuBarItems;
      for(var m=0;m<items.length;m++){
        var mi=items[m],p,s; try{p=mi.position();s=mi.size();}catch(e){continue;}
        if(!p||!s||s[0]<2) continue;
        var t=''; try{t=mi.title()||'';}catch(e){}
        a1.push({role:'MenuBarItem',text:String(t).slice(0,80),x:p[0],y:p[1],w:s[0],h:s[1],value:'',enabled:true});
      }
    }catch(e){}
    var merged=a1.concat(a2).concat(a3).concat(a4).slice(0,CAP);
    for(var n=0;n<merged.length;n++) merged[n].ref=n;
    return JSON.stringify(merged);
  }catch(e){return '[]';}
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

    // 6s for complex apps (all windows, all roles, more elements).
    let deadline = Instant::now() + Duration::from_millis(6000);
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

// Windows: the UI Automation twin of the macOS AX walk above — enumerate the foreground
// window's interactive elements (buttons / edits / checkboxes / menu items / list & tree
// rows …) with on-screen rects, via PowerShell. Controls before bulk rows, cap 120, time-
// bounded → falls back to the coordinate grid on permission/slow/no-tree (no hang). Same
// JSON shape as macOS so the caller is platform-agnostic.
#[cfg(target_os = "windows")]
pub fn read_ui_elements() -> Vec<UiElement> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    // DEEP Windows UI Automation: ALL control types, priority-tiered, cap 500, with value + enabled.
    let script = r#"$ErrorActionPreference='SilentlyContinue'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
Add-Type @"
using System;using System.Runtime.InteropServices;
public class _AW{[DllImport("user32.dll")]public static extern IntPtr GetForegroundWindow();}
"@
$h=[_AW]::GetForegroundWindow()
if($h -eq [IntPtr]::Zero){'[]';exit}
$AE=[System.Windows.Automation.AutomationElement]
$root=$AE::FromHandle($h)
if($root -eq $null){'[]';exit}
$t1=@{'Button'=1;'Edit'=1;'CheckBox'=1;'RadioButton'=1;'ComboBox'=1;'Hyperlink'=1;'Slider'=1;'SplitButton'=1;'Spinner'=1;'MenuItem'=1;'TabItem'=1}
$t2=@{'Text'=1}
$t3=@{'Group'=1;'Pane'=1;'ToolBar'=1;'StatusBar'=1;'ScrollBar'=1;'Table'=1;'DataGrid'=1;'Document'=1;'Window'=1;'Header'=1}
$a1=@();$a2=@();$a3=@();$a4=@()
$els=$root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
foreach($e in $els){
  if(($a1.Count+$a2.Count+$a3.Count+$a4.Count) -ge 1500){break}
  $ct='';try{$ct=$e.Current.ControlType.ProgrammaticName -replace 'ControlType\.',''}catch{continue}
  $r=$null;try{$r=$e.Current.BoundingRectangle}catch{continue}
  if($r -eq $null -or [double]::IsInfinity([double]$r.X) -or $r.Width -lt 2 -or $r.Height -lt 2){continue}
  $nm='';try{$nm=''+$e.Current.Name}catch{}
  if($nm.Length -gt 80){$nm=$nm.Substring(0,80)}
  $val='';try{$vp=$e.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern);$val=''+$vp.Current.Value;if($val.Length -gt 120){$val=$val.Substring(0,120)}}catch{}
  $en=$true;try{$en=$e.Current.IsEnabled}catch{}
  $rec=@{role=$ct;text=$nm;x=[double]$r.X;y=[double]$r.Y;w=[double]$r.Width;h=[double]$r.Height;value=$val;enabled=$en}
  if($t1.ContainsKey($ct)){$a1+=$rec}
  elseif($t2.ContainsKey($ct)){$a2+=$rec}
  elseif($t3.ContainsKey($ct)){$a3+=$rec}
  else{$a4+=$rec}
}
$all=@($a1)+@($a2)+@($a3)+@($a4)
$out=@();for($i=0;$i -lt [Math]::Min($all.Count,500);$i++){$m=$all[$i];$m.ref=$i;$out+=$m}
if($out.Count -eq 0){'[]'}else{ConvertTo-Json -Compress -InputObject @($out)}"#;

    let mut child = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let deadline = Instant::now() + Duration::from_millis(6000);
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
    let t = s.trim();
    if t.starts_with('{') {
        return serde_json::from_str::<UiElement>(t).map(|e| vec![e]).unwrap_or_default();
    }
    serde_json::from_str::<Vec<UiElement>>(t).unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn read_ui_elements() -> Vec<UiElement> {
    Vec::new()
}

// ===== Direct accessibility ACTION on a node by ref (AXPress / set value) =====
// "把软件转成节点、直接点" — instead of vision (screenshot → locate → pixel-click), perform
// a real accessibility action on the enumerated element. The `ref` is re-resolved by
// re-running the SAME deterministic enumeration as read_ui_elements (controls-first, cap
// 120), so ref→element is stable as long as the UI didn't change. Returns a small JSON
// {ok, role, name, value} for text-only verification (no screenshot needed).

#[cfg(target_os = "macos")]
fn run_osa(script: &str, ms: u64) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_millis(ms);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => return None,
        }
    }
    let mut s = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut s);
    }
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

#[cfg(target_os = "macos")]
const AX_ACTION_JS: &str = r##"(function(){
  try {
    var REF = __REF__; var ACT = "__ACT__"; var VAL = __VAL__;
    var se = Application('System Events');
    var procs = se.applicationProcesses.whose({ frontmost: true });
    if (!procs.length) return JSON.stringify({ok:false, err:'no frontmost app'});
    var proc = procs[0];
    var CAP = 500;
    var T1 = { AXButton:1,AXTextField:1,AXTextArea:1,AXCheckBox:1,AXRadioButton:1,AXPopUpButton:1,AXMenuButton:1,AXComboBox:1,AXLink:1,AXSlider:1,AXDisclosureTriangle:1,AXSegmentedControl:1,AXTabGroup:1,AXSearchField:1,AXStepper:1,AXIncrementor:1,AXColorWell:1,AXMenuItem:1,AXTab:1,AXDateField:1,AXSecureTextField:1 };
    var T2 = { AXStaticText:1,AXHeading:1 };
    var T3 = { AXGroup:1,AXScrollArea:1,AXSplitGroup:1,AXToolbar:1,AXList:1,AXTable:1,AXOutline:1,AXSheet:1,AXDialog:1,AXBrowser:1,AXDrawer:1,AXLayoutArea:1,AXMatte:1,AXRuler:1,AXSplitter:1,AXGrowArea:1 };
    var a1=[],a2=[],a3=[],a4=[];
    var wins; try{wins=proc.windows;}catch(e){wins=[];}
    for(var wi=0;wi<wins.length&&wi<5;wi++){
      var all; try{all=wins[wi].entireContents();}catch(e){all=[];}
      for(var k=0;k<all.length;k++){
        if(a1.length+a2.length+a3.length+a4.length>=CAP*3) break;
        var el=all[k],role; try{role=el.role();}catch(e){continue;}
        var p,s; try{p=el.position();s=el.size();}catch(e){continue;}
        if(!p||!s||s[0]<2||s[1]<2) continue;
        if(T1[role])a1.push(el);else if(T2[role])a2.push(el);else if(T3[role])a3.push(el);else a4.push(el);
      }
    }
    try{
      var items=proc.menuBars[0].menuBarItems;
      for(var m=0;m<items.length;m++){var mi=items[m],pp,ss;try{pp=mi.position();ss=mi.size();}catch(e){continue;}if(!pp||!ss||ss[0]<2)continue;a1.push(mi);}
    }catch(e){}
    var merged=a1.concat(a2).concat(a3).concat(a4).slice(0,CAP);
    if(REF<0||REF>=merged.length) return JSON.stringify({ok:false, err:'ref out of range (0..'+merged.length+')', n:merged.length});
    var el=merged[REF];
    var role=''; try{role=String(el.role()).replace('AX','');}catch(e){}
    var name=''; try{name=el.title()||el.description()||'';}catch(e){}
    if (ACT === 'set_value') {
      var done=false;
      try { el.focused = true; } catch(e) {}
      try { el.value = VAL; done=true; } catch(e) {}
      if (!done) { try { se.keystroke(VAL); done=true; } catch(e) {} }
      var nv=''; try { nv = el.value(); } catch(e) {}
      return JSON.stringify({ok:done, action:'set_value', role:role, name:String(name).slice(0,60), value:String(nv).slice(0,140)});
    } else if (ACT === 'focus') {
      var f=false; try { el.focused = true; f=true; } catch(e) {}
      return JSON.stringify({ok:f, action:'focus', role:role, name:String(name).slice(0,60)});
    } else {
      try { el.click(); } catch(e) { return JSON.stringify({ok:false, err:'AXPress failed: '+e, role:role, name:String(name).slice(0,60)}); }
      var nv2=''; try { var vv=el.value(); if(typeof vv==='string') nv2=vv; } catch(e) {}
      return JSON.stringify({ok:true, action:'press', role:role, name:String(name).slice(0,60), value:String(nv2).slice(0,140)});
    }
  } catch(e) { return JSON.stringify({ok:false, err:String(e)}); }
})()"##;

#[cfg(target_os = "macos")]
pub fn perform_ax_action(ref_: u32, action: &str, value: Option<&str>) -> Result<String, String> {
    let act = match action {
        "set_value" | "focus" | "press" => action,
        _ => "press",
    };
    let val_json =
        serde_json::to_string(value.unwrap_or("")).unwrap_or_else(|_| "\"\"".to_string());
    let script = AX_ACTION_JS
        .replace("__REF__", &ref_.to_string())
        .replace("__ACT__", act)
        .replace("__VAL__", &val_json);
    run_osa(&script, 6000).ok_or_else(|| "辅助功能操作超时或无权限".to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn perform_ax_action(_ref: u32, _action: &str, _value: Option<&str>) -> Result<String, String> {
    // Non-macOS: no AX-action path yet → caller falls back to a pixel click.
    Err("当前平台暂不支持节点直接操作，请改用坐标点击".to_string())
}

// ===== Apple Vision OCR: extract visible text from ANY app via VNRecognizeTextRequest =====
// Self-drawn apps (WeChat, QQ, games) don't expose AX trees. Vision OCR reads text
// directly from screen pixels — returns text + bounding boxes in screen-point coords,
// same space as AX elements. Compiled once to a cached binary; ~200-500ms per call.

#[cfg(target_os = "macos")]
const VISION_OCR_SWIFT: &str = r#"import Foundation
import Vision
import AppKit
guard let img = CGWindowListCreateImage(CGRect.null, .optionOnScreenOnly, kCGNullWindowID, [.bestResolution]) else {
    print("[]"); exit(0)
}
let pw = Double(img.width), ph = Double(img.height)
let sc = Double(NSScreen.main?.backingScaleFactor ?? 2.0)
let handler = VNImageRequestHandler(cgImage: img, options: [:])
let req = VNRecognizeTextRequest()
req.recognitionLevel = .accurate
req.recognitionLanguages = ["zh-Hans", "zh-Hant", "en-US", "ja-JP", "ko-KR"]
req.usesLanguageCorrection = true
do { try handler.perform([req]) } catch { print("[]"); exit(0) }
let observations = req.results ?? []
var out = [[String: Any]]()
var idx = 0
for obs in observations {
    guard let top = obs.topCandidates(1).first else { continue }
    let b = obs.boundingBox
    let x = b.origin.x * pw / sc
    let y = (1.0 - b.origin.y - b.height) * ph / sc
    let w = b.width * pw / sc
    let h = b.height * ph / sc
    if w < 3 || h < 3 { continue }
    out.append(["ref": idx, "role": "OCRText", "text": String(top.string.prefix(80)),
                "x": (x * 10).rounded() / 10, "y": (y * 10).rounded() / 10,
                "w": (w * 10).rounded() / 10, "h": (h * 10).rounded() / 10,
                "value": "", "enabled": true])
    idx += 1
}
if let j = try? JSONSerialization.data(withJSONObject: out),
   let s = String(data: j, encoding: .utf8) { print(s) } else { print("[]") }
"#;

#[cfg(target_os = "macos")]
pub fn read_ocr_elements() -> Vec<UiElement> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let bin = std::path::PathBuf::from("/tmp/michael_ide_vision_ocr_v1");

    if !bin.exists() {
        let src = "/tmp/michael_ide_vision_ocr_v1.swift";
        if std::fs::write(src, VISION_OCR_SWIFT).is_err() {
            return Vec::new();
        }
        let arch = std::env::consts::ARCH;
        let target = format!("{}-apple-macos14.0", if arch == "aarch64" { "arm64" } else { "x86_64" });
        let ok = Command::new("swiftc")
            .args(["-O", "-target", &target, "-o", bin.to_str().unwrap_or(""), src])
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Vec::new();
        }
    }

    let mut child = match Command::new(&bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let deadline = Instant::now() + Duration::from_millis(5000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Vec::new();
                }
                std::thread::sleep(Duration::from_millis(50));
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
pub fn read_ocr_elements() -> Vec<UiElement> {
    Vec::new()
}
