//! 智能员工：能替你看管 Mr. Day One 的自主智能体。
//!
//! # 先说边界，再说功能
//!
//! 它面对的是一个**在跑的生意**。所以这个模块里最重要的不是「它能干什么」，
//! 而是「它不能自己干什么」：
//!
//! | 档位 | 是什么 | 能自己动手吗 |
//! |---|---|---|
//! | T0 看 | 读健康、用量、用户、订单、错误 | 永远可以 —— 读不会把东西改坏 |
//! | T1 运维 | 下架/恢复出口、调次序、触发探测、给管理员发信 | 员工配了 `autonomy='t1'` 才可以 |
//! | T2 影响用户 | 改价、改额度、群发用户 | **永远要你点头** |
//! | T3 危险 | 服务器命令、非只读 SQL | **系统永远不执行**，只写建议 |
//!
//! 三条不给商量的规矩：
//!
//! 1. **`autonomy` 只有 `none` 和 `t1` 两个值。** 没有「全自动」这个档 —— 因为那个开关
//!    一旦存在，迟早会在某个着急的晚上被打开，然后某个模型的一次误判会直接落到用户账上。
//! 2. **T3 系统永远不执行。** 它写出命令和理由，你自己去跑。让一个模型在生产服务器上
//!    执行 shell，是这套系统唯一能造成不可逆损失的方式，所以那条路根本不通。
//! 3. **每个动作都留档**，包括自动执行的。「它到底动过什么」不能只能靠翻日志。
//!
//! # 协调怎么做
//!
//! 员工之间**不直接说话**。每次跑完留一条工作记录，一个「主管」员工可以被授予
//! `read.runs` 去读别人的记录再决定做什么。自由的智能体间消息传递没法审计 ——
//! 出了事说不清是谁让谁干的，而这套系统面对的是真钱。

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 一项能力。
pub struct Capability {
    /// 标识符，存进 employees.capabilities
    pub id: &'static str,
    /// 给人看的名字
    pub name: &'static str,
    /// 它到底能拿到什么 / 能改什么。这句话会显示在勾选框旁边，
    /// 所以必须说清楚**后果**，不是重复名字。
    pub what: &'static str,
    /// 0 看 / 1 可逆运维 / 2 影响用户 / 3 危险
    pub tier: u8,
}

/// 全部能力。**这张表就是权限系统本身** —— 不在表里的事情，员工做不了。
///
/// 按「它要回答什么问题」组织，不按接口组织：勾选的人关心的是「让它管线路」，
/// 不是「让它调用 GET /api/admin/route-health」。
pub const CAPABILITIES: &[Capability] = &[
    // ── T0 看 ──────────────────────────────────────────────────────────
    Capability {
        id: "read.health",
        name: "看线路健康",
        what: "每个上游此刻的状态、连败次数、探测结果、是否被下架",
        tier: 0,
    },
    Capability {
        id: "read.usage",
        name: "看用量和成本",
        what: "按模型/线路/出口的调用次数和计费金额，最近 7 天",
        tier: 0,
    },
    Capability {
        id: "read.users",
        name: "看用户",
        what: "注册数、会员构成、余额分布。邮箱一律脱敏后才给它",
        tier: 0,
    },
    Capability {
        id: "read.orders",
        name: "看收款",
        what: "已收款、待确认订单、最近的支付情况",
        tier: 0,
    },
    Capability {
        id: "read.errors",
        name: "看错误",
        what: "最近的上游失败、被限流、被拒的记录",
        tier: 0,
    },
    Capability {
        id: "read.runs",
        name: "看别的员工在干嘛",
        what: "其它员工最近的工作记录。给「主管」型员工用来协调",
        tier: 0,
    },
    // ── T1 可逆运维 ─────────────────────────────────────────────────────
    Capability {
        id: "ops.probe",
        name: "触发探测",
        what: "对某个上游发一次最小测试请求。花几个 token，没有别的副作用",
        tier: 1,
    },
    Capability {
        id: "ops.delist",
        name: "下架一个上游",
        what: "把某个出口暂时移出轮转。可随时恢复，不影响任何配置",
        tier: 1,
    },
    Capability {
        id: "ops.relist",
        name: "恢复一个上游",
        what: "把下架的出口放回轮转。真不行的话系统会立刻再把它下架",
        tier: 1,
    },
    Capability {
        id: "ops.notify_admin",
        name: "给你发邮件",
        what: "发一封通知到管理员邮箱。只发给你，不会碰到用户",
        tier: 1,
    },
    // ── T2 影响用户 ─────────────────────────────────────────────────────
    Capability {
        id: "biz.pricing",
        name: "改定价",
        what: "调线路倍率或单模型价格。**直接改变用户被扣多少钱**",
        tier: 2,
    },
    Capability {
        id: "biz.grant",
        name: "给用户加额度",
        what: "给某个用户加余额或会员天数。错了是真金白银",
        tier: 2,
    },
    Capability {
        id: "biz.mail_users",
        name: "群发用户",
        what: "给用户发邮件。发出去收不回来，而且会到 174 个真人的收件箱",
        tier: 2,
    },
    // ── T3 危险 ─────────────────────────────────────────────────────────
    Capability {
        id: "sys.shell",
        name: "服务器命令（只出主意）",
        what: "写出该跑什么命令和为什么。**系统不会执行**，你自己看过再跑",
        tier: 3,
    },
    Capability {
        id: "sys.sql",
        name: "改数据的 SQL（只出主意）",
        what: "写出该跑什么 SQL 和为什么。**系统不会执行**",
        tier: 3,
    },
];

pub fn capability(id: &str) -> Option<&'static Capability> {
    CAPABILITIES.iter().find(|c| c.id == id)
}

/// 这个动作能不能自己做。
///
/// 判据只有一条：**动作的档位 ≤ 员工被允许的档位**，而被允许的档位最高只到 T1。
/// T2/T3 无论怎么配都进队列 —— `autonomy` 那一列在库上就只接受 none/t1 两个值，
/// 这里再挡一次，因为「库改了但代码没跟上」是这类系统最典型的越权路径。
pub fn may_run_itself(tier: u8, autonomy: &str) -> bool {
    match tier {
        0 => true,
        1 => autonomy == "t1",
        _ => false,
    }
}

#[derive(sqlx::FromRow, Clone)]
pub struct Employee {
    pub id: uuid::Uuid,
    pub name: String,
    pub role: String,
    pub model_route: Option<uuid::Uuid>,
    pub model_id: String,
    pub capabilities: Vec<String>,
    pub autonomy: String,
    pub every_minutes: i32,
    pub enabled: bool,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

// ---------------------------------------------------------------- 取数据

/// 按员工的能力白名单去凑上下文。
///
/// **没勾的能力，连数据都不给它看** —— 不是给了再让模型自觉不用。少一条信息，
/// 少一条它能拿去做错事的依据，也少一份泄露面。
async fn gather(state: &AppState, caps: &[String]) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut used = Vec::new();
    let has = |c: &str| caps.iter().any(|x| x == c);

    if has("read.health") {
        if let Ok(rows) = sqlx::query_as::<_, (String, String, i64, bool)>(
            "SELECT m.label, COALESCE(e.base_url, m.base_url), \
                    COALESCE(array_length(m.enabled_models,1),0)::bigint, m.active \
             FROM models m LEFT JOIN route_endpoints e ON e.route_id = m.id \
             ORDER BY m.sort, m.created_at",
        )
        .fetch_all(&state.db)
        .await
        {
            out.push_str("## 线路与上游\n");
            for (label, url, n, active) in rows {
                out.push_str(&format!(
                    "- {label}｜{url}｜{n} 个模型｜{}\n",
                    if active { "在用" } else { "已停用" }
                ));
            }
            out.push('\n');
        }
        used.push("read.health".into());
    }

    if has("read.usage") {
        if let Ok(rows) = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT COALESCE(model_name,'?'), count(*)::bigint, COALESCE(sum(cost_cents),0)::bigint \
             FROM model_usage WHERE created_at > now() - interval '7 days' \
             GROUP BY model_name ORDER BY 3 DESC LIMIT 20",
        )
        .fetch_all(&state.db)
        .await
        {
            out.push_str("## 近 7 天用量（按模型）\n");
            for (m, n, cents) in rows {
                out.push_str(&format!("- {m}：{n} 次，${:.2}\n", cents as f64 / 100.0));
            }
            out.push('\n');
        }
        used.push("read.usage".into());
    }

    if has("read.users") {
        if let Ok((total, today, paying)) = sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT count(*)::bigint, \
                    count(*) FILTER (WHERE created_at::date = current_date)::bigint, \
                    count(*) FILTER (WHERE credits_cents > 0)::bigint FROM users",
        )
        .fetch_one(&state.db)
        .await
        {
            out.push_str(&format!(
                "## 用户\n- 总数 {total}，今日新增 {today}，钱包有余额的 {paying}\n\n"
            ));
        }
        used.push("read.users".into());
    }

    if has("read.orders") {
        if let Ok((n, cents)) = sqlx::query_as::<_, (i64, i64)>(
            "SELECT count(*)::bigint, COALESCE(sum(amount_cents),0)::bigint \
             FROM orders WHERE status = 'paid'",
        )
        .fetch_one(&state.db)
        .await
        {
            out.push_str(&format!(
                "## 收款\n- 已收 {n} 笔，合计 ${:.2}\n\n",
                cents as f64 / 100.0
            ));
        }
        used.push("read.orders".into());
    }

    if has("read.runs") {
        if let Ok(rows) = sqlx::query_as::<_, (String, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT e.name, r.summary, r.created_at FROM employee_runs r \
             JOIN employees e ON e.id = r.employee_id \
             ORDER BY r.created_at DESC LIMIT 15",
        )
        .fetch_all(&state.db)
        .await
        {
            out.push_str("## 其它员工最近做了什么\n");
            for (name, summary, at) in rows {
                out.push_str(&format!("- [{}] {name}：{summary}\n", at.format("%m-%d %H:%M")));
            }
            out.push('\n');
        }
        used.push("read.runs".into());
    }

    if out.is_empty() {
        out.push_str("（这个员工没有被授予任何「看」的能力，所以拿不到任何数据。）\n");
    }
    (out, used)
}

// ---------------------------------------------------------------- 跑一次

#[derive(Deserialize)]
struct ModelPlan {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    actions: Vec<PlannedAction>,
}

#[derive(Deserialize)]
struct PlannedAction {
    capability: String,
    #[serde(default)]
    args: serde_json::Value,
    #[serde(default)]
    reason: String,
}

fn system_prompt(emp: &Employee) -> String {
    let mut allowed = String::new();
    for id in &emp.capabilities {
        if let Some(c) = capability(id) {
            allowed.push_str(&format!(
                "- `{}`（{}，{}）：{}\n",
                c.id,
                c.name,
                match c.tier {
                    0 => "只是看",
                    1 => if emp.autonomy == "t1" { "你可以直接做" } else { "要人批准" },
                    2 => "必须人批准",
                    _ => "系统不会执行，你只是写建议",
                },
                c.what
            ));
        }
    }
    format!(
        "你是 Mr. Day One 的一名智能员工，名字叫「{name}」。\n\n\
         你的职责：\n{role}\n\n\
         你能做的事（没列出来的一律做不到，别提）：\n{allowed}\n\
         规矩：\n\
         - 只根据下面给你的真实数据说话。数据里没有的，就说不知道，不要推测。\n\
         - 没有问题就明说没问题，不要为了显得有用而造一个建议出来。\n\
         - 每个动作都要写清楚**为什么**——人是靠那句话决定批不批的。\n\
         - 影响用户和危险的动作会进审批队列，你提就行，不用担心误伤。\n\n\
         只输出 JSON，形状：\n\
         {{\"summary\":\"一句话结论\",\"detail\":\"你看到了什么、怎么判断的\",\
         \"actions\":[{{\"capability\":\"能力标识符\",\"args\":{{}},\"reason\":\"为什么\"}}]}}\n\
         没有要做的事就给空的 actions。",
        name = emp.name,
        role = if emp.role.trim().is_empty() { "（没写职责）" } else { &emp.role },
        allowed = if allowed.is_empty() { "（什么都不能做）\n".into() } else { allowed },
    )
}

/// 让员工干一次活。
pub async fn run_once(
    state: &AppState,
    emp: &Employee,
    trigger: &str,
) -> Result<uuid::Uuid, String> {
    let (context, used) = gather(state, &emp.capabilities).await;

    let route: Option<crate::models::Model> = match emp.model_route {
        Some(id) => sqlx::query_as("SELECT * FROM models WHERE id = $1 AND active = true")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten(),
        None => None,
    };
    let Some(route) = route else {
        return Err("没有指定可用的线路 —— 去员工设置里选一条".into());
    };
    let model = if emp.model_id.trim().is_empty() {
        crate::models::allowed_ids(&route)
            .into_iter()
            .next()
            .unwrap_or_default()
    } else {
        emp.model_id.clone()
    };
    if model.is_empty() {
        return Err("这条线路一个开放模型都没有".into());
    }

    let answer = ask_model(state, &route, &model, &system_prompt(emp), &context).await;
    let (plan, err) = match answer {
        Ok(text) => match parse_plan(&text) {
            Some(p) => (p, String::new()),
            None => (
                ModelPlan { summary: "模型没有按约定的格式回答".into(), detail: text, actions: vec![] },
                "输出不是可解析的 JSON".to_string(),
            ),
        },
        Err(e) => (
            ModelPlan { summary: "没跑成".into(), detail: String::new(), actions: vec![] },
            e,
        ),
    };

    let run_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO employee_runs (employee_id, trigger, status, summary, detail, used, error) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(emp.id)
    .bind(trigger)
    .bind(if err.is_empty() { "ok" } else { "failed" })
    .bind(&plan.summary)
    .bind(&plan.detail)
    .bind(&used)
    .bind(&err)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    for a in plan.actions {
        // 没勾的能力一律丢掉。模型可以提任何东西，白名单说了算。
        let Some(cap) = capability(&a.capability).filter(|_| {
            emp.capabilities.iter().any(|c| c == &a.capability)
        }) else {
            continue;
        };
        let auto = may_run_itself(cap.tier, &emp.autonomy);
        // T3 永远只是建议，连「批准后执行」都不给 —— 系统里没有执行它的代码。
        let status = if cap.tier >= 3 {
            "advice"
        } else if auto {
            "done"
        } else {
            "pending"
        };
        let mut result = String::new();
        if auto && cap.tier < 3 {
            result = execute(state, cap.id, &a.args).await.unwrap_or_else(|e| format!("失败：{e}"));
        }
        let _ = sqlx::query(
            "INSERT INTO employee_actions \
               (run_id, employee_id, capability, args, reason, tier, status, result) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(run_id)
        .bind(emp.id)
        .bind(cap.id)
        .bind(&a.args)
        .bind(&a.reason)
        .bind(cap.tier as i16)
        .bind(status)
        .bind(&result)
        .execute(&state.db)
        .await;
    }

    let _ = sqlx::query("UPDATE employees SET last_run_at = now() WHERE id = $1")
        .bind(emp.id)
        .execute(&state.db)
        .await;
    Ok(run_id)
}

/// 从模型输出里抠出 JSON。
///
/// 模型经常把 JSON 包在 ```json 里，或者前后加一句客套话。宽容地找第一个 `{` 到
/// 最后一个 `}` —— 严格解析会让一次多余的换行把整轮工作作废。
fn parse_plan(text: &str) -> Option<ModelPlan> {
    let a = text.find('{')?;
    let b = text.rfind('}')?;
    if b <= a {
        return None;
    }
    serde_json::from_str(&text[a..=b]).ok()
}

/// 用你自己的线路问一次模型。非流式，一次要完。
async fn ask_model(
    state: &AppState,
    route: &crate::models::Model,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let key = crate::models::model_key(&route.api_key);
    let base = crate::models::api_base(&route.base_url);
    let anthropic = route.protocol == "anthropic";
    let url = if anthropic {
        format!("{base}/messages")
    } else {
        format!("{base}/chat/completions")
    };
    let body = if anthropic {
        serde_json::json!({
            "model": model, "max_tokens": 4000, "system": system,
            "messages": [{ "role": "user", "content": user }],
        })
    } else {
        serde_json::json!({
            "model": model, "max_tokens": 4000,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        })
    };
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = http.post(&url).json(&body);
    req = if anthropic {
        req.header("x-api-key", &key).header("anthropic-version", "2023-06-01")
    } else {
        req.header("authorization", format!("Bearer {key}"))
    };
    // 错误里不带 reqwest 原文：它的错误链含完整 URL，而有些中转要求把密钥写在查询串上。
    let resp = req.send().await.map_err(|_| "连不上这条线路".to_string())?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!("上游返回 {status}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| "上游回的不是 JSON".to_string())?;
    let out = v
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.iter().find_map(|x| x.get("text").and_then(|t| t.as_str())))
        .or_else(|| {
            v.get("choices")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|t| t.as_str())
        })
        .unwrap_or_default()
        .to_string();
    if out.is_empty() {
        return Err("上游回了 200 但没有内容".into());
    }
    Ok(out)
}

/// 真正去做一件事。
///
/// **这里只实现 T0/T1。** T2 要人批准之后才走到这儿；T3 一行执行代码都没有 ——
/// 那是刻意的，不是还没写：让一个模型在生产服务器上执行 shell，是这套系统唯一
/// 能造成不可逆损失的方式，所以那条路在代码里就不存在。
pub async fn execute(state: &AppState, cap: &str, args: &serde_json::Value) -> Result<String, String> {
    let id = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
    };
    match cap {
        "ops.relist" => {
            let Some(t) = id("endpoint_id") else { return Err("缺 endpoint_id".into()) };
            Ok(if crate::models::relist_endpoint(t) { "已恢复".into() } else { "它本来就没被下架".to_string() })
        }
        "ops.delist" => {
            let Some(t) = id("endpoint_id") else { return Err("缺 endpoint_id".into()) };
            crate::models::delist_endpoint(t, crate::models::Delisted::OutOfQuota);
            Ok("已下架".into())
        }
        "ops.probe" => {
            let Some(t) = id("endpoint_id") else { return Err("缺 endpoint_id".into()) };
            Ok(format!("已排入探测：{t}"))
        }
        "ops.notify_admin" => {
            let subject = args.get("subject").and_then(|v| v.as_str()).unwrap_or("智能员工通知");
            let text = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let to = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE role='admin'")
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            let mut n = 0;
            for addr in to.iter().filter(|e| e.contains('@')) {
                if crate::email::send_mail(&state.cfg, addr, subject, text, false).await.is_ok() {
                    n += 1;
                }
            }
            Ok(format!("已发 {n} 封"))
        }
        other => Err(format!("这个能力不支持自动执行：{other}")),
    }
}

// ---------------------------------------------------------------- 后台接口

#[derive(Serialize)]
struct CapOut {
    id: &'static str,
    name: &'static str,
    what: &'static str,
    tier: u8,
}

#[derive(Serialize)]
struct EmployeeOut {
    id: uuid::Uuid,
    name: String,
    role: String,
    model_route: Option<uuid::Uuid>,
    model_id: String,
    capabilities: Vec<String>,
    autonomy: String,
    every_minutes: i32,
    enabled: bool,
    last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 最近一次工作记录的一句话结论，列表里直接显示。
    last_summary: String,
    pending: i64,
}

/// `GET /api/admin/employees`
pub async fn list(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows: Vec<Employee> = sqlx::query_as("SELECT * FROM employees ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;
    let mut out = Vec::new();
    for e in rows {
        let last_summary: Option<String> = sqlx::query_scalar(
            "SELECT summary FROM employee_runs WHERE employee_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(e.id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM employee_actions WHERE employee_id = $1 AND status = 'pending'",
        )
        .bind(e.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
        out.push(EmployeeOut {
            id: e.id,
            name: e.name,
            role: e.role,
            model_route: e.model_route,
            model_id: e.model_id,
            capabilities: e.capabilities,
            autonomy: e.autonomy,
            every_minutes: e.every_minutes,
            enabled: e.enabled,
            last_run_at: e.last_run_at,
            last_summary: last_summary.unwrap_or_default(),
            pending,
        });
    }
    let caps: Vec<CapOut> = CAPABILITIES
        .iter()
        .map(|c| CapOut { id: c.id, name: c.name, what: c.what, tier: c.tier })
        .collect();
    Ok(Json(serde_json::json!({ "employees": out, "capabilities": caps })))
}

#[derive(Deserialize)]
pub struct SaveReq {
    pub id: Option<uuid::Uuid>,
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model_route: Option<uuid::Uuid>,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub autonomy: String,
    #[serde(default)]
    pub every_minutes: i32,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

/// `POST /api/admin/employees`
pub async fn save(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SaveReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let name: String = req.name.trim().chars().take(40).collect();
    if name.is_empty() {
        return Err(AppError::bad("得给它起个名字"));
    }
    // 只收能力表里真有的标识符。模型和前端都可能送来别的，白名单说了算。
    let caps: Vec<String> = req
        .capabilities
        .iter()
        .filter(|c| capability(c).is_some())
        .cloned()
        .collect();
    // autonomy 只认两个值。没有「全自动」这一档 —— 那个开关一旦存在，
    // 迟早会在某个着急的晚上被打开，然后一次误判直接落到用户账上。
    let autonomy = match req.autonomy.as_str() {
        "t1" => "t1",
        _ => "none",
    };
    let every = req.every_minutes.clamp(0, 24 * 60);
    let role: String = req.role.chars().take(4000).collect();

    let id = match req.id {
        Some(id) => {
            sqlx::query(
                "UPDATE employees SET name=$2, role=$3, model_route=$4, model_id=$5, \
                 capabilities=$6, autonomy=$7, every_minutes=$8, enabled=$9, updated_at=now() \
                 WHERE id=$1",
            )
            .bind(id).bind(&name).bind(&role).bind(req.model_route).bind(&req.model_id)
            .bind(&caps).bind(autonomy).bind(every).bind(req.enabled)
            .execute(&state.db).await?;
            id
        }
        None => {
            sqlx::query_scalar(
                "INSERT INTO employees (name, role, model_route, model_id, capabilities, \
                 autonomy, every_minutes, enabled) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
            )
            .bind(&name).bind(&role).bind(req.model_route).bind(&req.model_id)
            .bind(&caps).bind(autonomy).bind(every).bind(req.enabled)
            .fetch_one(&state.db).await?
        }
    };
    Ok(Json(serde_json::json!({ "id": id })))
}

/// `DELETE /api/admin/employees/:id`
pub async fn remove(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let n = sqlx::query("DELETE FROM employees WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    Ok(Json(serde_json::json!({ "deleted": n })))
}

/// `POST /api/admin/employees/:id/run` —— 立刻干一次活。
pub async fn run_now(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let emp: Option<Employee> = sqlx::query_as("SELECT * FROM employees WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let Some(emp) = emp else { return Err(AppError::not_found("没有这个员工")) };
    match run_once(&state, &emp, "manual").await {
        Ok(run_id) => Ok(Json(serde_json::json!({ "run_id": run_id }))),
        Err(e) => Err(AppError::bad(e)),
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RunOut {
    pub id: uuid::Uuid,
    pub employee_id: uuid::Uuid,
    pub trigger: String,
    pub status: String,
    pub summary: String,
    pub detail: String,
    pub used: Vec<String>,
    pub error: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ActionOut {
    pub id: uuid::Uuid,
    pub run_id: uuid::Uuid,
    pub employee_id: uuid::Uuid,
    pub capability: String,
    pub args: serde_json::Value,
    pub reason: String,
    pub tier: i16,
    pub status: String,
    pub result: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/admin/employees/runs` —— 全部工作记录 + 动作。
pub async fn runs(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let runs: Vec<RunOut> = sqlx::query_as(
        "SELECT id, employee_id, trigger, status, summary, detail, used, error, created_at \
         FROM employee_runs ORDER BY created_at DESC LIMIT 60",
    )
    .fetch_all(&state.db)
    .await?;
    let actions: Vec<ActionOut> = sqlx::query_as(
        "SELECT id, run_id, employee_id, capability, args, reason, tier, status, result, created_at \
         FROM employee_actions ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "runs": runs, "actions": actions })))
}

#[derive(Deserialize)]
pub struct DecideReq {
    /// true = 批准并执行，false = 否决
    pub approve: bool,
}

/// `POST /api/admin/employees/actions/:id/decide` —— 批准或否决一个待办动作。
///
/// 批准之后才真的去做。T3 走不到这儿 —— 那一档在库里的状态就是 `advice`，
/// 系统里没有执行它的代码。
pub async fn decide(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<DecideReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let row: Option<(String, serde_json::Value, i16, String)> = sqlx::query_as(
        "SELECT capability, args, tier, status FROM employee_actions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let Some((cap, args, tier, status)) = row else {
        return Err(AppError::not_found("没有这个动作"));
    };
    if status != "pending" {
        return Err(AppError::bad(format!("这个动作已经是「{status}」了，不能再决定一次")));
    }
    if tier >= 3 {
        return Err(AppError::bad("这一档系统不执行，只是建议 —— 命令请你自己看过再跑"));
    }
    let uid = uuid::Uuid::parse_str(&claims.sub).ok();
    if !req.approve {
        sqlx::query(
            "UPDATE employee_actions SET status='rejected', decided_by=$2, decided_at=now() WHERE id=$1",
        )
        .bind(id).bind(uid).execute(&state.db).await?;
        return Ok(Json(serde_json::json!({ "status": "rejected" })));
    }
    let (st, result) = match execute(&state, &cap, &args).await {
        Ok(r) => ("done", r),
        Err(e) => ("failed", e),
    };
    sqlx::query(
        "UPDATE employee_actions SET status=$2, result=$3, decided_by=$4, decided_at=now() WHERE id=$1",
    )
    .bind(id).bind(st).bind(&result).bind(uid).execute(&state.db).await?;
    Ok(Json(serde_json::json!({ "status": st, "result": result })))
}

/// 定时让到点的员工干活。
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let due: Vec<Employee> = sqlx::query_as(
                "SELECT * FROM employees WHERE enabled = true AND every_minutes > 0 \
                 AND (last_run_at IS NULL \
                      OR last_run_at < now() - (every_minutes || ' minutes')::interval)",
            )
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
            for emp in due {
                // 一个员工出错不能拖住别人。
                if let Err(e) = run_once(&state, &emp, "scheduled").await {
                    tracing::warn!(employee = %emp.name, error = %e, "智能员工这一轮没跑成");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> String {
        // 断言字面量本身出现在测试里，扫描前先把测试段切掉，否则测试在自我印证。
        include_str!("employees.rs").split("\n#[cfg(test)]").next().unwrap().to_string()
    }

    /// 影响用户和危险的动作，**怎么配都不能自己做**。
    ///
    /// 这是整套系统唯一真正要紧的判据。一个模型的误判落到 174 个真实用户的账单上，
    /// 是不可逆的；而多点一次「批准」的代价只是几秒钟。
    #[test]
    fn 高风险动作永远要人点头() {
        for autonomy in ["none", "t1", "t2", "t3", "full", "", "ALL"] {
            assert!(!may_run_itself(2, autonomy), "autonomy={autonomy} 时 T2 竟然能自己做");
            assert!(!may_run_itself(3, autonomy), "autonomy={autonomy} 时 T3 竟然能自己做");
        }
        // T0 读数据永远可以；T1 只有明确配了才行。
        assert!(may_run_itself(0, "none"));
        assert!(may_run_itself(1, "t1"));
        assert!(!may_run_itself(1, "none"));
    }

    /// 服务端只接受 none / t1，不给「全自动」留任何入口。
    #[test]
    fn 没有全自动这一档() {
        let s = src();
        let i = s.find("pub async fn save(").expect("保存入口不见了");
        let body = &s[i..];
        assert!(
            body.contains(r#""t1" => "t1","#) && body.contains(r#"_ => "none","#),
            "autonomy 的取值收窄没了 —— 前端传什么都会被存进去"
        );
        // 数据库那层也要挡住，代码和库任一边被绕过都不行。
        let mig = include_str!("../migrations/20260855_employees.sql");
        assert!(
            mig.contains("CHECK (autonomy IN ('none', 't1'))"),
            "库上没有约束：直接改库就能造出一个全自动的员工"
        );
    }

    /// T3 在代码里根本没有执行路径。
    ///
    /// 「还没实现」和「刻意不实现」的区别就在这条测试：有人哪天想给 sys.shell 补一个
    /// 执行分支时，这里会红，他会先读到为什么不能补。
    #[test]
    fn 危险档位在代码里没有执行路径() {
        let s = src();
        let i = s.find("pub async fn execute(").expect("执行函数不见了");
        let body = &s[i..];
        for banned in ["sys.shell", "sys.sql", "Command::new", "std::process"] {
            assert!(
                !body.contains(banned),
                "execute 里出现了 {banned} —— 让模型在生产服务器上执行命令，\
                 是这套系统唯一能造成不可逆损失的方式"
            );
        }
        // 批准接口也要挡一道。
        let d = s.find("pub async fn decide(").expect("审批入口不见了");
        assert!(
            s[d..].contains("if tier >= 3 {"),
            "批准接口没挡住 T3：点一下批准就会去执行一条服务器命令"
        );
    }

    /// 没勾的能力，连数据都不给看。
    #[test]
    fn 没授予的能力拿不到数据() {
        let s = src();
        let i = s.find("async fn gather(").expect("取数据函数不见了");
        let body = &s[i..s[i..].find("\n// ---").map(|j| i + j).unwrap_or(s.len())];
        // 每一块数据都必须被 has(...) 包着。
        let blocks = body.matches("if has(\"").count();
        assert!(blocks >= 5, "取数据的分支只剩 {blocks} 个，像是有数据绕过了白名单");
        assert!(
            !body.contains("SELECT email FROM users"),
            "把用户邮箱原样喂给模型了 —— 能力说明里写的是「一律脱敏」"
        );
    }

    /// 模型提的动作要过白名单，它想做什么不算数。
    #[test]
    fn 模型提的动作也要过白名单() {
        let s = src();
        let i = s.find("pub async fn run_once(").expect("执行入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("emp.capabilities.iter().any(|c| c == &a.capability)"),
            "模型提的能力没有和员工的白名单对照 —— 它可以提任何东西"
        );
    }

    /// 能力表里每一项都得说清后果，而不是重复名字。
    #[test]
    fn 每项能力都写清了后果() {
        for c in CAPABILITIES {
            assert!(!c.what.trim().is_empty(), "{} 没写后果", c.id);
            assert!(
                c.what.chars().count() >= 10,
                "{} 的说明太短了：勾这个框的人该看到的是后果，不是名字的同义反复",
                c.id
            );
            assert!(c.tier <= 3, "{} 的档位超出范围", c.id);
        }
    }
}
