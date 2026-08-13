//! 验证 mouse.click / mouse.move 是否真的认 {x,y}。
//!
//! 位置用 mouse.position 读——和执行注入的是同一个 enigo 实例，是唯一可信的 oracle。
//! 裸进程里 NSEvent::mouseLocation 不走 run loop，CGEventCreate(source) 读的又是该
//! source 上一个事件的位置，两者都会给出常量假读数。
use rust_automation_framework::{RpcRequest, RpcServer};

fn main() {
    let server = RpcServer::new(0).unwrap();
    let call = |m: &str, p: serde_json::Value| -> serde_json::Value {
        let resp = server.handle_request(RpcRequest {
            jsonrpc: "2.0".to_string(), method: m.to_string(), params: p, id: None,
        });
        serde_json::to_value(&resp).unwrap_or_default()
    };
    let pos = || -> (f64, f64) {
        let r = call("mouse.position", serde_json::json!({}));
        let r = &r["result"];
        (r["x"].as_f64().unwrap_or(-1.0), r["y"].as_f64().unwrap_or(-1.0))
    };

    let home = pos();
    println!("起始指针位置: {:?}", home);
    let mut failures = 0;

    // ① 整数坐标的 mouse.move（一直是好的，作为 oracle 自身的对照）
    call("mouse.move", serde_json::json!({"x": 900, "y": 700}));
    let p1 = pos();
    println!("\n① mouse.move 整数 (900,700) → {:?}  {}", p1, if p1 == (900.0, 700.0) { "✅" } else { failures += 1; "❌" });

    // ② 小数坐标：模型按 number 类型 schema 完全可能给 423.0，as_i64 会判成"没传"
    let r2 = call("mouse.move", serde_json::json!({"x": 423.0, "y": 317.0}));
    let p2 = pos();
    let ok2 = p2 == (423.0, 317.0);
    println!("② mouse.move 小数 (423.0,317.0) → {:?}  {}", p2, if ok2 { "✅" } else { failures += 1; "❌" });
    if !ok2 { println!("   响应: {}", r2); }

    // ③ 字符串坐标
    call("mouse.move", serde_json::json!({"x": 900, "y": 700}));
    let r3 = call("mouse.move", serde_json::json!({"x": "512", "y": "384"}));
    let p3 = pos();
    let ok3 = p3 == (512.0, 384.0);
    println!("③ mouse.move 字符串 (\"512\",\"384\") → {:?}  {}", p3, if ok3 { "✅" } else { failures += 1; "❌" });
    if !ok3 { println!("   响应: {}", r3); }

    // ④ 核心修复：mouse.click 认坐标。点在屏幕中部——菜单栏 (y≈4) 会触发系统 UI 并把
    //    指针拉走，那不是坐标没生效，是被系统抢了。
    call("mouse.move", serde_json::json!({"x": 900, "y": 700}));
    let parked = pos();
    let r4 = call("mouse.click", serde_json::json!({"x": 1200, "y": 400, "button": "left"}));
    let imm = pos();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let t50 = pos();
    std::thread::sleep(std::time::Duration::from_millis(400));
    let t450 = pos();
    println!("   点击后连读: 立刻 {:?} / +50ms {:?} / +450ms {:?}", imm, t50, t450);
    let p4 = imm;
    let ok4 = p4 == (1200.0, 400.0);
    println!("\n④ mouse.click 带坐标：{:?} → {:?}（目标 1200,400）  {}", parked, p4, if ok4 { "✅" } else { failures += 1; "❌ {x,y} 仍被忽略" });
    println!("   回执: {}", serde_json::to_string(&r4["result"]).unwrap_or_default());

    // ⑤ 不带坐标时保持原地点击的老语义
    call("mouse.move", serde_json::json!({"x": 640, "y": 480}));
    call("mouse.click", serde_json::json!({"button": "left"}));
    let p5 = pos();
    println!("⑤ mouse.click 不带坐标应原地不动 → {:?}  {}", p5, if p5 == (640.0, 480.0) { "✅" } else { failures += 1; "❌" });

    call("mouse.move", serde_json::json!({"x": home.0 as i64, "y": home.1 as i64}));
    println!("\n指针已归位。失败项: {}", failures);
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
