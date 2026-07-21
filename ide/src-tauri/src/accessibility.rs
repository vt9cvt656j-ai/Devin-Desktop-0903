//! macOS-only: read the frontmost app's interactive UI elements via the System
//! Events accessibility API, driven through `osascript` (JavaScript for
//! Automation). No unsafe FFI and no extra crates — the Rust side is just a
//! bounded subprocess + JSON parse, so it compiles and is checkable everywhere.
//!
//! Best-effort and TIME-BOUNDED: if Accessibility permission is off, the app
//! doesn't expose a tree, or enumeration is slow, it returns nothing and the
//! caller falls back to the coordinate grid (no regression, never hangs).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct AccessibilityTarget {
    pid: i64,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct UiSnapshot {
    target: Option<AccessibilityTarget>,
    elements: Vec<UiElement>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct AxElementSignature {
    role: String,
    text: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    value: String,
    enabled: bool,
}

impl From<&UiElement> for AxElementSignature {
    fn from(element: &UiElement) -> Self {
        Self {
            role: element.role.clone(),
            text: element.text.clone(),
            x: element.x,
            y: element.y,
            w: element.w,
            h: element.h,
            value: element.value.clone(),
            enabled: element.enabled,
        }
    }
}

#[derive(Clone, Debug)]
struct AxRefBinding {
    raw_ref: u32,
    signature: AxElementSignature,
}

#[derive(Default)]
struct AxRefState {
    target: Option<AccessibilityTarget>,
    refs: std::collections::HashMap<u32, AxRefBinding>,
}

static LATEST_AX_REFS: once_cell::sync::Lazy<std::sync::Mutex<AxRefState>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(AxRefState::default()));
static NEXT_AX_REF: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn clear_latest_ax_refs() -> Result<(), String> {
    let mut latest = LATEST_AX_REFS
        .lock()
        .map_err(|_| "accessibility target state is unavailable".to_string())?;
    *latest = AxRefState::default();
    Ok(())
}

fn install_ax_snapshot(snapshot: &mut UiSnapshot) -> Result<(), String> {
    let mut latest = LATEST_AX_REFS
        .lock()
        .map_err(|_| "accessibility target state is unavailable".to_string())?;
    *latest = AxRefState::default();
    let Some(target) = snapshot.target.clone() else {
        return Ok(());
    };
    for element in &mut snapshot.elements {
        let binding = AxRefBinding {
            raw_ref: element.ref_,
            signature: AxElementSignature::from(&*element),
        };
        let opaque_ref = loop {
            let candidate = NEXT_AX_REF.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if candidate != 0 && !latest.refs.contains_key(&candidate) {
                break candidate;
            }
        };
        element.ref_ = opaque_ref;
        latest.refs.insert(opaque_ref, binding);
    }
    latest.target = Some(target);
    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_ax_ref(reference: u32) -> Result<(AxRefBinding, AccessibilityTarget), String> {
    let latest = LATEST_AX_REFS
        .lock()
        .map_err(|_| "accessibility target state is unavailable".to_string())?;
    let target = latest.target.clone().ok_or_else(|| {
        "no actionable accessibility snapshot; run read_screen for the current foreground app"
            .to_string()
    })?;
    let binding =
        latest.refs.get(&reference).cloned().ok_or_else(|| {
            "stale or unknown accessibility ref; run read_screen again".to_string()
        })?;
    Ok((binding, target))
}

fn default_true() -> bool {
    true
}

/// One UI element from the accessibility tree, positions in SCREEN POINTS
/// (top-left origin). The caller maps these into the screenshot's space.
#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct ReadScreenResponse {
    pub source: String,
    pub elements: Vec<UiElement>,
    pub limitations: Vec<String>,
}

/// Read the frontmost application's accessibility tree. OCR is an explicit
/// fallback for self-drawn interfaces; OCR refs are informational because they
/// do not correspond to an accessibility node that can receive AX actions.
#[tauri::command]
pub async fn read_screen(ocr: Option<bool>) -> Result<ReadScreenResponse, String> {
    let use_ocr = ocr.unwrap_or(false);
    // Invalidate old refs before starting a new read. A failed or concurrent read
    // must never leave a ref from an older foreground process actionable.
    clear_latest_ax_refs()?;
    let mut snapshot = tauri::async_runtime::spawn_blocking(move || {
        if use_ocr {
            UiSnapshot {
                target: None,
                elements: read_ocr_elements(),
            }
        } else {
            read_ui_snapshot()
        }
    })
    .await
    .map_err(|error| format!("screen reader task failed: {error}"))?;
    install_ax_snapshot(&mut snapshot)?;
    let elements = snapshot.elements;

    let mut limitations = Vec::new();
    if use_ocr {
        limitations.push(
            "OCR text boxes are observations, not actionable AX refs; use their coordinates only through an explicitly approved automation action."
                .into(),
        );
    }
    if elements.is_empty() {
        limitations.push(
            "No elements were returned. The frontmost app may expose no accessibility tree, or Michael IDE may lack Accessibility/Screen Recording permission."
                .into(),
        );
    }
    Ok(ReadScreenResponse {
        source: if use_ocr {
            "screen_ocr"
        } else {
            "accessibility_tree"
        }
        .into(),
        elements,
        limitations,
    })
}

/// Perform a real accessibility action against an opaque ref from the latest
/// frontmost UI tree. The target PID and original node index are both resolved
/// from that snapshot; callers must read again whenever the target UI changes.
#[tauri::command]
pub async fn ui_click(
    reference: u32,
    action: String,
    value: Option<String>,
) -> Result<serde_json::Value, String> {
    if !matches!(action.as_str(), "press" | "set_value" | "focus") {
        return Err("action must be press, set_value, or focus".into());
    }
    if action == "set_value" && value.is_none() {
        return Err("set_value requires value".into());
    }
    #[cfg(target_os = "macos")]
    let (binding, expected_target) = resolve_ax_ref(reference)?;
    #[cfg(not(target_os = "macos"))]
    let binding = AxRefBinding {
        raw_ref: reference,
        signature: AxElementSignature {
            role: String::new(),
            text: String::new(),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            value: String::new(),
            enabled: false,
        },
    };
    #[cfg(not(target_os = "macos"))]
    let expected_target = AccessibilityTarget {
        pid: 0,
        name: String::new(),
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        perform_ax_action(&binding, &action, value.as_deref(), &expected_target)
    })
    .await
    .map_err(|error| format!("accessibility action task failed: {error}"))??;
    parse_action_result(&result)
}

fn parse_action_result(result: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_str(result)
        .map_err(|error| format!("invalid accessibility result: {error}"))?;
    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        return Ok(value);
    }
    let reason = value
        .get("err")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("accessibility action failed");
    Err(reason.to_string())
}

#[cfg(target_os = "macos")]
const AX_SIGNATURE_BUILDERS_JS: &str = r#"var axLabel = function(el){
  var t='';
  try{t=el.title()||'';}catch(e){}
  if(!t){try{t=el.description()||'';}catch(e){}}
  if(!t){try{t=el.help()||'';}catch(e){}}
  if(!t){try{var v=el.value();if(typeof v==='string')t=v;}catch(e){}}
  return String(t).slice(0,80);
};
var axValue = function(el){try{var v=el.value();return v!=null?String(v).slice(0,120):'';}catch(e){return '';}};
var axEnabled = function(el){try{return el.enabled()!==false;}catch(e){return true;}};
var axElementSignature = function(el,role,p,s){
  return {role:role.replace('AX',''),text:axLabel(el),x:p[0],y:p[1],w:s[0],h:s[1],value:axValue(el),enabled:axEnabled(el)};
};
var axMenuSignature = function(el,p,s){
  var t='';try{t=el.title()||'';}catch(e){}
  return {role:'MenuBarItem',text:String(t).slice(0,80),x:p[0],y:p[1],w:s[0],h:s[1],value:'',enabled:true};
};"#;

#[cfg(target_os = "macos")]
fn read_ui_snapshot() -> UiSnapshot {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    // DEEP JXA: enumerate ALL accessibility elements from the frontmost app —
    // every role, all windows (up to 5), with value + enabled state.
    // Priority tiers: interactive controls → text labels → structural containers → rest.
    // This captures elements that the old 24-role filter missed (AXStaticText labels,
    // AXGroup containers, AXToolbar, AXTable, etc.). Cap 500, timeout 6s.
    let script = r##"(function () {
  try {
    __SIGNATURE_BUILDERS__
    var se = Application('System Events');
    var procs = se.applicationProcesses.whose({ frontmost: true });
    if (!procs.length) return JSON.stringify({target:null,elements:[]});
    var proc = procs[0];
    var pid=0,pname='';
    try{pid=Number(proc.unixId());}catch(e){}
    try{pname=String(proc.name()||'').slice(0,120);}catch(e){}
    if(!isFinite(pid)||pid<=0) return JSON.stringify({target:null,elements:[]});
    var CAP = 500;
    var T1 = { AXButton:1,AXTextField:1,AXTextArea:1,AXCheckBox:1,AXRadioButton:1,AXPopUpButton:1,AXMenuButton:1,AXComboBox:1,AXLink:1,AXSlider:1,AXDisclosureTriangle:1,AXSegmentedControl:1,AXTabGroup:1,AXSearchField:1,AXStepper:1,AXIncrementor:1,AXColorWell:1,AXMenuItem:1,AXTab:1,AXDateField:1,AXSecureTextField:1 };
    var T2 = { AXStaticText:1,AXHeading:1 };
    var T3 = { AXGroup:1,AXScrollArea:1,AXSplitGroup:1,AXToolbar:1,AXList:1,AXTable:1,AXOutline:1,AXSheet:1,AXDialog:1,AXBrowser:1,AXDrawer:1,AXLayoutArea:1,AXMatte:1,AXRuler:1,AXSplitter:1,AXGrowArea:1 };
    var a1=[],a2=[],a3=[],a4=[];
    var take = function(el,role){
      var p,s; try{p=el.position();s=el.size();}catch(e){return;}
      if(!p||!s||s[0]<2||s[1]<2) return;
      var rec=axElementSignature(el,role,p,s);
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
        a1.push(axMenuSignature(mi,p,s));
      }
    }catch(e){}
    var merged=a1.concat(a2).concat(a3).concat(a4).slice(0,CAP);
    for(var n=0;n<merged.length;n++) merged[n].ref=n;
    return JSON.stringify({target:{pid:pid,name:pname},elements:merged});
  }catch(e){return JSON.stringify({target:null,elements:[]});}
})()"##
        .replace("__SIGNATURE_BUILDERS__", AX_SIGNATURE_BUILDERS_JS);

    let mut child = match crate::process_util::command("osascript")
        .args(["-l", "JavaScript", "-e", script.as_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return UiSnapshot::default(),
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
                    return UiSnapshot::default();
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => return UiSnapshot::default(),
        }
    }

    let mut s = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut s);
    }
    serde_json::from_str::<UiSnapshot>(s.trim()).unwrap_or_default()
}

#[cfg(target_os = "macos")]
pub fn read_ui_elements() -> Vec<UiElement> {
    read_ui_snapshot().elements
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

    let mut child = match crate::process_util::command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
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
        return serde_json::from_str::<UiElement>(t)
            .map(|e| vec![e])
            .unwrap_or_default();
    }
    serde_json::from_str::<Vec<UiElement>>(t).unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn read_ui_elements() -> Vec<UiElement> {
    Vec::new()
}

#[cfg(not(target_os = "macos"))]
fn read_ui_snapshot() -> UiSnapshot {
    UiSnapshot {
        target: None,
        elements: read_ui_elements(),
    }
}

// ===== Direct accessibility ACTION on a node by ref (AXPress / set value) =====
// "把软件转成节点、直接点" — instead of vision (screenshot → locate → pixel-click), perform
// a real accessibility action on the enumerated element. The opaque `ref` is mapped back
// to its node index, then the same deterministic enumeration is repeated (cap 500), so
// ref→element is stable as long as the UI didn't change. Returns a small JSON
// {ok, role, name, value} for text-only verification (no screenshot needed).

#[cfg(target_os = "macos")]
fn run_osa(script: &str, ms: u64) -> Option<String> {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let mut child = crate::process_util::command("osascript")
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
const AX_SIGNATURE_HELPER_JS: &str = r#"var axSignatureChanges = function(expected, actual) {
  var fields = ['role','text','x','y','w','h','value','enabled'];
  var changed = [];
  for (var i=0;i<fields.length;i++) {
    var field=fields[i];
    if (actual[field] !== expected[field]) changed.push(field);
  }
  return changed;
};"#;

#[cfg(target_os = "macos")]
const AX_ACTION_JS: &str = r##"(function(){
  try {
    __SIGNATURE_BUILDERS__
    __SIGNATURE_HELPER__
    var CTX = __CONTEXT__;
    var REF = CTX.reference; var ACT = CTX.action; var VAL = CTX.value;
    var se = Application('System Events');
    var procs = se.applicationProcesses.whose({ frontmost: true });
    if (!procs.length) return JSON.stringify({ok:false, err:'no frontmost app'});
    var proc = procs[0];
    var actualPid=0; try{actualPid=Number(proc.unixId());}catch(e){}
    if(!isFinite(actualPid)||actualPid<=0) return JSON.stringify({ok:false, err:'could not identify frontmost process'});
    if(actualPid!==CTX.pid) return JSON.stringify({ok:false, err:'frontmost app changed; run read_screen again', expectedPid:CTX.pid, actualPid:actualPid});
    var actualAppName=''; try{actualAppName=String(proc.name()||'').slice(0,120);}catch(e){}
    if(actualAppName!==CTX.appName) return JSON.stringify({ok:false, err:'frontmost app identity changed; run read_screen again'});
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
        var entry={el:el,signature:axElementSignature(el,role,p,s)};
        if(T1[role])a1.push(entry);else if(T2[role])a2.push(entry);else if(T3[role])a3.push(entry);else a4.push(entry);
      }
    }
    try{
      var items=proc.menuBars[0].menuBarItems;
      for(var m=0;m<items.length;m++){var mi=items[m],pp,ss;try{pp=mi.position();ss=mi.size();}catch(e){continue;}if(!pp||!ss||ss[0]<2)continue;a1.push({el:mi,signature:axMenuSignature(mi,pp,ss)});}
    }catch(e){}
    var merged=a1.concat(a2).concat(a3).concat(a4).slice(0,CAP);
    if(REF<0||REF>=merged.length) return JSON.stringify({ok:false, err:'ref out of range (0..'+merged.length+')', n:merged.length});
    var candidate=merged[REF];
    var changed=axSignatureChanges(CTX.signature,candidate.signature);
    if(changed.length) return JSON.stringify({ok:false, err:'accessibility ref is stale; element signature changed ('+changed.join(', ')+'); run read_screen again'});
    var el=candidate.el;
    var role=candidate.signature.role;
    var name=candidate.signature.text;
    if (ACT === 'set_value') {
      try { el.value = VAL; } catch(e) {
        return JSON.stringify({ok:false, err:'AX set_value failed: '+String(e), action:'set_value', role:role, name:String(name).slice(0,60)});
      }
      var nv;
      try { nv = el.value(); } catch(e) {
        return JSON.stringify({ok:false, err:'AX set_value could not be verified: '+String(e), action:'set_value', role:role, name:String(name).slice(0,60)});
      }
      if(String(nv)!==String(VAL)) {
        return JSON.stringify({ok:false, err:'AX set_value did not apply the requested value', action:'set_value', role:role, name:String(name).slice(0,60), value:String(nv).slice(0,140)});
      }
      return JSON.stringify({ok:true, action:'set_value', role:role, name:String(name).slice(0,60), value:String(nv).slice(0,140)});
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
fn build_ax_action_script(
    binding: &AxRefBinding,
    action: &str,
    value: Option<&str>,
    expected_target: &AccessibilityTarget,
) -> Result<String, String> {
    let act = match action {
        "set_value" | "focus" | "press" => action,
        _ => return Err("unsupported accessibility action".to_string()),
    };
    let context = serde_json::json!({
        "reference": binding.raw_ref,
        "action": act,
        "value": value.unwrap_or(""),
        "pid": expected_target.pid,
        "appName": &expected_target.name,
        "signature": &binding.signature,
    });
    let context_json = serde_json::to_string(&context)
        .map_err(|error| format!("could not encode accessibility action: {error}"))?;
    Ok(AX_ACTION_JS
        .replace("__SIGNATURE_BUILDERS__", AX_SIGNATURE_BUILDERS_JS)
        .replace("__SIGNATURE_HELPER__", AX_SIGNATURE_HELPER_JS)
        .replace("__CONTEXT__", &context_json))
}

#[cfg(target_os = "macos")]
fn perform_ax_action(
    binding: &AxRefBinding,
    action: &str,
    value: Option<&str>,
    expected_target: &AccessibilityTarget,
) -> Result<String, String> {
    let script = build_ax_action_script(binding, action, value, expected_target)?;
    run_osa(&script, 6000).ok_or_else(|| "辅助功能操作超时或无权限".to_string())
}

#[cfg(not(target_os = "macos"))]
fn perform_ax_action(
    _binding: &AxRefBinding,
    _action: &str,
    _value: Option<&str>,
    _expected_target: &AccessibilityTarget,
) -> Result<String, String> {
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
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let bin = std::path::PathBuf::from("/tmp/michael_ide_vision_ocr_v1");

    if !bin.exists() {
        let src = "/tmp/michael_ide_vision_ocr_v1.swift";
        if std::fs::write(src, VISION_OCR_SWIFT).is_err() {
            return Vec::new();
        }
        let arch = std::env::consts::ARCH;
        let target = format!(
            "{}-apple-macos14.0",
            if arch == "aarch64" { "arm64" } else { "x86_64" }
        );
        let ok = crate::process_util::command("swiftc")
            .args([
                "-O",
                "-target",
                &target,
                "-o",
                bin.to_str().unwrap_or(""),
                src,
            ])
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Vec::new();
        }
    }

    let mut child = match crate::process_util::command(&bin)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    fn element(raw_ref: u32) -> UiElement {
        UiElement {
            ref_: raw_ref,
            role: "Button".to_string(),
            text: "Save".to_string(),
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 20.0,
            value: String::new(),
            enabled: true,
        }
    }

    #[cfg(target_os = "macos")]
    fn signature_gate_result(
        expected: &AxElementSignature,
        actual: &AxElementSignature,
    ) -> serde_json::Value {
        let context = serde_json::json!({"expected": expected, "actual": actual});
        let context_json = serde_json::to_string(&context).expect("test signatures should encode");
        let script = r#"(function(){
__SIGNATURE_HELPER__
var ctx=__CONTEXT__;
var operated=false;
var changed=axSignatureChanges(ctx.expected,ctx.actual);
if(changed.length) return JSON.stringify({operated:operated,changed:changed});
operated=true;
return JSON.stringify({operated:operated,changed:changed});
})()"#
            .replace("__SIGNATURE_HELPER__", AX_SIGNATURE_HELPER_JS)
            .replace("__CONTEXT__", &context_json);
        let output = run_osa(&script, 1000).expect("signature gate JXA should run");
        serde_json::from_str(&output).expect("signature gate should return JSON")
    }

    #[test]
    fn failed_action_result_is_an_error() {
        let error = parse_action_result(r#"{"ok":false,"err":"AX set_value failed"}"#)
            .expect_err("a failed AX result must not resolve as a successful command");
        assert_eq!(error, "AX set_value failed");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn refs_are_invalidated_when_a_new_snapshot_is_installed() {
        clear_latest_ax_refs().expect("state should be writable");
        let mut first = UiSnapshot {
            target: Some(AccessibilityTarget {
                pid: 101,
                name: "First".to_string(),
            }),
            elements: vec![element(7)],
        };
        install_ax_snapshot(&mut first).expect("first snapshot should install");
        let first_ref = first.elements[0].ref_;
        let (binding, target) = resolve_ax_ref(first_ref).expect("first ref should resolve");
        assert_eq!(binding.raw_ref, 7);
        assert_eq!(
            binding.signature,
            AxElementSignature::from(&first.elements[0])
        );
        assert_eq!(target.pid, 101);

        let mut second = UiSnapshot {
            target: Some(AccessibilityTarget {
                pid: 202,
                name: "Second".to_string(),
            }),
            elements: vec![element(3)],
        };
        install_ax_snapshot(&mut second).expect("second snapshot should install");
        assert_ne!(first_ref, second.elements[0].ref_);
        assert!(resolve_ax_ref(first_ref).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn same_pid_reordering_fails_signature_gate_before_operation() {
        let expected = AxElementSignature::from(&element(0));
        let mut replacement = element(0);
        replacement.text = "Delete".to_string();
        replacement.x = 100.0;
        replacement.value = "danger".to_string();
        replacement.enabled = false;
        let actual = AxElementSignature::from(&replacement);

        let rejected = signature_gate_result(&expected, &actual);
        assert_eq!(rejected["operated"], false);
        let changed = rejected["changed"]
            .as_array()
            .expect("changed fields should be an array");
        for field in ["text", "x", "value", "enabled"] {
            assert!(changed.iter().any(|changed| changed == field));
        }

        let accepted = signature_gate_result(&expected, &expected);
        assert_eq!(accepted["operated"], true);
        assert_eq!(accepted["changed"], serde_json::json!([]));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn action_script_checks_full_signature_without_keyboard_fallback() {
        let target = AccessibilityTarget {
            pid: 4242,
            name: "Target".to_string(),
        };
        let source = element(9);
        let binding = AxRefBinding {
            raw_ref: 9,
            signature: AxElementSignature::from(&source),
        };
        let script = build_ax_action_script(
            &binding,
            "set_value",
            Some("literal __PID__ and \"quotes\""),
            &target,
        )
        .expect("valid action should produce a script");

        assert!(script.contains("\"pid\":4242"));
        assert!(script.contains("\"reference\":9"));
        assert!(script.contains("actualPid!==CTX.pid"));
        assert!(script.contains("axElementSignature(el,role,p,s)"));
        assert!(script.contains("axSignatureChanges(CTX.signature,candidate.signature)"));
        assert!(script.contains(r#"literal __PID__ and \"quotes\""#));
        assert!(!script.contains("keystroke"));
        assert!(!script.contains("__SIGNATURE_BUILDERS__"));
        assert!(!script.contains("__SIGNATURE_HELPER__"));
        assert!(!script.contains("__CONTEXT__"));

        let gate = script
            .find("if(changed.length)")
            .expect("signature gate should exist");
        for operation in ["el.value = VAL", "el.focused = true", "el.click()"] {
            assert!(gate < script.find(operation).expect("AX operation should exist"));
        }
    }
}
