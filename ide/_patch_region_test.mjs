// 一次性补丁：安装源地区注入的服务端守卫测试。用完即删。锚点唯一命中，否则报错退出。
import { readFileSync, writeFileSync } from "node:fs";
const p = "/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/src/prompts.rs";
let s = readFileSync(p, "utf8");
const from = `    #[test]
    fn anthropic_thinking_gate_by_model() {`;
const to = `    #[test]
    fn region_mirror_guidance_targets_cn_agent_requests_only() {
        let assemble = |mode: &str, region: Option<&str>| {
            let mut headers = HeaderMap::new();
            headers.insert("x-ide-mode", mode.parse().unwrap());
            if let Some(region) = region {
                headers.insert("x-ide-region", region.parse().unwrap());
            }
            let mut body = serde_json::json!({
                "model": "gpt-5.6-sol",
                "messages": [{"role": "user", "content": "帮我初始化一个 React 项目并安装依赖"}]
            });
            assemble_into(&headers, &mut body).unwrap();
            body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["role"] == "user")
                .next_back()
                .unwrap()["content"]
                .as_str()
                .unwrap()
                .to_string()
        };
        // 中国大陆 + agent → 注入镜像指引（含官方源回退与"不改锁文件"约束）。
        let cn = assemble("agent", Some("cn"));
        assert!(cn.contains("安装源·按用户网络地区"));
        assert!(cn.contains("registry.npmmirror.com"));
        assert!(cn.contains("回退官方默认源"));
        // 其他地区 / 未上报 / 非法值 / 非 agent 模式 → 一个字都不注入。
        assert!(!assemble("agent", Some("us")).contains("安装源"));
        assert!(!assemble("agent", None).contains("安装源"));
        assert!(!assemble("agent", Some("CN")).contains("安装源"), "非小写地区码必须按缺失处理");
        assert!(!assemble("chat", Some("cn")).contains("安装源"));
        // 注入走最新 user 消息通道，系统前缀保持字节稳定（前缀缓存纪律）。
        assert!(!read_prompt("agent_core").unwrap().contains("安装源·按用户网络地区"));
    }

    #[test]
    fn anthropic_thinking_gate_by_model() {`;
const i = s.indexOf(from);
if (i === -1) throw new Error("anchor not found");
if (s.indexOf(from, i + 1) !== -1) throw new Error("anchor not unique");
s = s.slice(0, i) + to + s.slice(i + from.length);
writeFileSync(p, s);
console.log("ok");
