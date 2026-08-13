//! Domain-knowledge base: a small corpus of curated best-practice markdown the IDE
//! agent can retrieve on demand (agentic RAG). Files live under `./knowledge/<domain>/
//! <topic>.md`, chunked by `##` headers into self-contained sections, indexed with
//! BM25 in memory at startup. The `knowledge_search` tool hits `/api/knowledge/search`.
//!
//! Why server-side: the operator updates the corpus centrally (redeploy) and every
//! user/model benefits instantly — no app rebuild. Weak models (GPT-mini/DeepSeek/
//! MiniMax) get an expert cheat-sheet to lift their effective IQ on domain tasks.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Serialize;

#[derive(Clone)]
struct Chunk {
    domain: String,  // subdir, e.g. "web-frontend"
    topic: String,   // filename stem, e.g. "react-nextjs"
    section: String, // the `##` header text
    text: String,    // full section body (for return)
    tf: HashMap<String, u32>,
    len: u32,
}

pub struct KnowledgeIndex {
    chunks: Vec<Chunk>,
    df: HashMap<String, u32>,
    avg_len: f64,
    /// (domain, [topics]) for the /domains listing.
    pub domains: Vec<(String, Vec<String>)>,
}

#[derive(Serialize)]
pub struct SearchHit {
    pub domain: String,
    pub topic: String,
    pub section: String,
    pub text: String,
    pub score: f64,
}

static INDEX: OnceLock<KnowledgeIndex> = OnceLock::new();

const STOP: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "to", "of", "in", "on", "and", "or", "for",
    "with", "it", "this", "that", "these", "those", "as", "at", "by", "from", "not", "you", "your",
    "use", "using", "used", "when", "if", "its", "but", "can", "will", "should", "do", "does",
    "how", "what", "why", "which", "into", "out", "up", "so", "than", "then",
];

/// Chinese→English term map. The corpus is English; agents often query in Chinese,
/// and the ASCII tokenizer drops Chinese chars entirely — so a pure-Chinese query
/// would match nothing. At QUERY time we append the English equivalents of any
/// Chinese tech terms found, so "玻璃拟态 阴影" → also searches "glassmorphism shadow".
const CN_EN: &[(&str, &str)] = &[
    // visual / CSS / animation (the user's pain area)
    ("动画", "animation animate"),
    ("缓动", "easing ease"),
    ("过渡", "transition"),
    ("关键帧", "keyframes"),
    ("玻璃拟态", "glassmorphism backdrop blur"),
    ("毛玻璃", "glassmorphism backdrop blur"),
    ("阴影", "shadow box-shadow"),
    ("渐变", "gradient"),
    ("特效", "effect"),
    ("圆角", "border-radius rounded"),
    ("模糊", "blur filter"),
    ("滤镜", "filter"),
    ("视差", "parallax"),
    ("悬停", "hover"),
    ("旋转", "rotate transform"),
    ("缩放", "scale transform"),
    ("位移", "translate transform"),
    ("弹性", "spring"),
    ("微交互", "micro-interaction"),
    ("骨架屏", "skeleton loading"),
    ("加载动画", "loading spinner"),
    ("轮播", "carousel slider"),
    ("瀑布流", "masonry grid"),
    ("滚动", "scroll"),
    ("视图过渡", "view transition"),
    // SVG
    ("路径", "path"),
    ("描边", "stroke"),
    ("图标", "icon"),
    ("插画", "illustration"),
    ("矢量", "svg vector"),
    ("曲线", "curve bezier"),
    ("形状", "shape"),
    // layout / styling
    ("布局", "layout grid flex flexbox"),
    ("响应式", "responsive media query"),
    ("暗色", "dark mode theme"),
    ("主题", "theme"),
    ("字体", "font typography"),
    ("排版", "typography"),
    ("间距", "spacing padding margin"),
    ("对齐", "align alignment"),
    ("颜色", "color palette"),
    ("配色", "color palette"),
    ("对比度", "contrast"),
    ("样式", "css style styling"),
    ("可访问性", "accessibility a11y"),
    // components
    ("按钮", "button"),
    ("表单", "form input validation"),
    ("弹窗", "modal dialog"),
    ("下拉", "dropdown select"),
    ("提示", "tooltip toast"),
    ("卡片", "card"),
    ("导航", "navbar navigation"),
    ("侧边栏", "sidebar"),
    ("分页", "pagination"),
    // backend / db / security
    ("数据库", "database schema sql"),
    ("索引", "index"),
    ("迁移", "migration"),
    ("事务", "transaction"),
    ("缓存", "cache"),
    ("鉴权", "auth authentication"),
    ("认证", "authentication"),
    ("授权", "authorization"),
    ("登录", "login session"),
    ("密码", "password hash"),
    ("加密", "encryption hash"),
    ("注入", "injection sql"),
    ("安全", "security"),
    ("漏洞", "vulnerability"),
    ("接口", "api endpoint rest"),
    ("分页查询", "pagination"),
    ("状态码", "status code http"),
    ("限流", "rate limit"),
    // devops
    ("部署", "deploy deployment"),
    ("容器", "docker container"),
    ("镜像", "docker image"),
    ("环境变量", "env config"),
    ("性能", "performance"),
    ("日志", "logging"),
    ("健康检查", "healthcheck"),
    ("反向代理", "reverse proxy nginx"),
    // react / frontend
    ("组件", "component"),
    ("状态管理", "state management"),
    ("副作用", "useEffect effect"),
    ("渲染", "render rendering"),
    ("路由", "router routing"),
    ("钩子", "hook"),
    // reverse engineering
    ("逆向", "reverse engineering binary"),
    ("反编译", "decompile decompiler reverse"),
    ("反汇编", "disassemble disassembly objdump radare"),
    ("脱壳", "unpack packer upx"),
    ("解包", "unpack extract archive installer"),
    ("壳", "packer protector"),
    ("安装包", "installer nsis inno setup msi extract"),
    ("可执行", "executable binary pe elf"),
    ("二进制", "binary executable analysis"),
    ("混淆", "obfuscation deobfuscate"),
    ("加壳", "packed packer protector"),
    ("脱混淆", "deobfuscate"),
    ("反调试", "anti-debug debugger"),
    ("调试", "debug debugger gdb"),
    ("字节码", "bytecode dotnet java decompile"),
    ("签名算法", "signature signing algorithm hmac reverse"),
    ("特征", "signature magic bytes"),
    ("壳特征", "packer signature die detect-it-easy"),
    // systems programming / Rust / C++
    ("所有权", "ownership borrow rust"),
    ("借用", "borrow borrowing rust"),
    ("生命周期", "lifetime rust"),
    ("异步", "async await tokio"),
    ("并发", "concurrency concurrent mutex"),
    ("线程", "thread spawn"),
    ("内存安全", "memory safety unsafe"),
    ("编译器", "compiler cargo build"),
    ("类型系统", "type system generic trait"),
    ("宏", "macro derive proc-macro"),
    ("内核", "kernel cuda gpu"),
    ("优化", "optimization performance benchmark"),
    // mobile
    ("移动端", "mobile ios android"),
    ("安卓", "android kotlin compose"),
    ("苹果", "ios swift swiftui"),
    ("跨平台", "cross-platform flutter react-native"),
    ("原生", "native ios android swift kotlin"),
    // data / ML
    ("数据管道", "data pipeline etl dag"),
    ("特征工程", "feature engineering"),
    ("模型训练", "model training fine-tune"),
    ("微调", "fine-tune lora qlora"),
    ("向量", "vector embedding rag"),
    ("检索增强", "rag retrieval augmented"),
    ("机器学习", "machine learning ml"),
    ("深度学习", "deep learning neural"),
    ("测试", "test testing unit integration e2e"),
    ("重构", "refactor refactoring"),
    ("文档", "documentation docs docstring"),
    ("代码审查", "code review"),
    // penetration testing / security tooling (authorized testing)
    ("渗透", "penetration testing pentest exploit"),
    ("渗透测试", "penetration testing pentest"),
    ("信息收集", "recon reconnaissance osint enumeration"),
    ("枚举", "enumeration enum"),
    ("端口扫描", "port scan nmap masscan"),
    ("扫描", "scan nmap recon"),
    ("提权", "privilege escalation privesc linpeas"),
    ("横向移动", "lateral movement pivot"),
    ("爆破", "brute force password hydra hashcat"),
    ("密码破解", "password cracking hashcat john"),
    ("哈希", "hash crack hashcat"),
    ("字典", "wordlist rockyou dictionary"),
    ("漏洞利用", "exploit exploitation metasploit"),
    ("反弹", "reverse shell payload"),
    ("内网", "internal network pivot active directory"),
    ("域", "active directory domain kerberos"),
    ("权限维持", "persistence post-exploitation"),
    ("后渗透", "post-exploitation privesc loot"),
    ("木马", "payload shell"),
    ("载荷", "payload msfvenom"),
    ("免杀", "evasion av bypass"),
    ("社工", "osint social phishing recon"),
    ("子域名", "subdomain enumeration"),
    ("目录扫描", "directory fuzzing ffuf gobuster"),
    ("注入", "injection sqlmap sql"),
    // finance
    ("金融", "finance fintech banking"),
    ("交易", "trading transaction exchange"),
    ("量化", "quantitative quant algo trading"),
    ("支付", "payment checkout stripe"),
    ("风控", "risk control risk management"),
    ("对账", "reconciliation settlement"),
    ("清算", "clearing settlement"),
    ("风险", "risk management"),
    ("账本", "ledger bookkeeping"),
    ("双重记账", "double-entry bookkeeping"),
    ("反洗钱", "aml anti-money-laundering"),
    ("合约", "contract futures options"),
    ("期权", "options derivatives"),
    ("回撤", "drawdown"),
    ("夏普", "sharpe ratio"),
    ("套利", "arbitrage"),
    // healthcare
    ("医疗", "healthcare medical clinical"),
    ("病历", "medical record ehr emr"),
    ("处方", "prescription medication pharmacy"),
    ("诊断", "diagnosis diagnostic"),
    ("患者", "patient"),
    ("临床", "clinical trial cds"),
    ("电子病历", "ehr emr fhir"),
    ("医嘱", "clinical order cpoe"),
    ("药品", "drug medication formulary"),
    ("影像", "medical imaging dicom"),
    ("合规", "compliance hipaa gdpr"),
    ("隐私", "privacy phi pii gdpr"),
    ("脱敏", "de-identification anonymization"),
    // legal
    ("合同", "contract agreement clause"),
    ("条款", "clause terms conditions"),
    ("诉讼", "litigation lawsuit case"),
    ("知识产权", "intellectual property ip patent"),
    ("版权", "copyright license"),
    ("商标", "trademark"),
    ("专利", "patent"),
    ("仲裁", "arbitration dispute"),
    ("电子签名", "e-signature esign"),
    ("尽职调查", "due diligence"),
    ("保密", "confidentiality nda"),
    // ecommerce
    ("电商", "ecommerce e-commerce shopping"),
    ("购物车", "cart checkout"),
    ("库存", "inventory stock"),
    ("订单", "order fulfillment"),
    ("商品", "product catalog sku"),
    ("变体", "variant sku"),
    ("物流", "shipping logistics fulfillment"),
    ("退款", "refund return"),
    ("优惠券", "coupon promo discount"),
    ("促销", "promotion discount"),
    ("结算", "checkout payment"),
    ("供应链", "supply chain"),
    // gaming
    ("游戏", "game gaming"),
    ("引擎", "engine unity unreal godot"),
    ("碰撞", "collision collider physics"),
    ("物理引擎", "physics engine rigidbody"),
    ("网络同步", "netcode multiplayer sync"),
    ("帧率", "fps frame rate"),
    ("粒子", "particle effect vfx"),
    ("着色器", "shader glsl hlsl"),
    ("场景", "scene level"),
    ("预制体", "prefab"),
    ("精灵", "sprite"),
    ("寻路", "pathfinding a-star navmesh"),
    ("状态机", "state machine fsm"),
    // iot / embedded
    ("物联网", "iot internet-of-things"),
    ("嵌入式", "embedded firmware"),
    ("传感器", "sensor telemetry"),
    ("固件", "firmware ota update"),
    ("低功耗", "low-power sleep deep-sleep"),
    ("串口", "serial uart spi i2c"),
    ("看门狗", "watchdog timer"),
    ("设备影子", "device shadow twin"),
    ("边缘计算", "edge computing"),
    ("网关", "gateway mqtt broker"),
    // blockchain
    ("区块链", "blockchain distributed ledger"),
    ("智能合约", "smart contract solidity"),
    ("钱包", "wallet metamask"),
    ("代币", "token erc20 erc721"),
    ("质押", "staking stake"),
    ("挖矿", "mining proof-of-work"),
    ("共识", "consensus pos pow"),
    ("去中心化", "decentralized defi"),
    ("闪电贷", "flash loan"),
    ("预言机", "oracle chainlink"),
    ("燃气", "gas fee gwei"),
    ("签名", "signature eip-712"),
    // education
    ("教育", "education edtech lms"),
    ("课程", "course curriculum"),
    ("考试", "exam quiz assessment"),
    ("作业", "assignment homework"),
    ("评分", "grading rubric score"),
    ("抄袭", "plagiarism detection"),
    ("间隔重复", "spaced repetition anki fsrs"),
    ("学习管理", "lms learning management"),
    ("自适应", "adaptive learning"),
    ("题库", "question bank"),
    ("在线课堂", "online classroom"),
    ("学分", "credit academic"),
    // saas
    ("多租户", "multi-tenancy tenant isolation"),
    ("订阅", "subscription recurring billing"),
    ("计费", "billing metered usage-based"),
    ("套餐", "plan tier pricing"),
    ("配额", "quota limit rate-limit"),
    ("功能开关", "feature flag toggle"),
    ("入驻", "onboarding tenant provisioning"),
    ("席位", "seat license"),
    ("账单", "invoice billing"),
    ("试用", "trial freemium"),
    // marketing
    ("营销", "marketing growth"),
    ("搜索引擎优化", "seo search engine optimization"),
    ("邮件营销", "email marketing drip campaign"),
    ("客户关系", "crm customer relationship"),
    ("漏斗", "funnel conversion"),
    ("留存", "retention cohort"),
    ("获客", "acquisition cac customer-acquisition"),
    ("归因", "attribution utm"),
    ("着陆页", "landing page"),
    ("转化率", "conversion rate optimization cro"),
    ("用户画像", "persona segmentation"),
    ("内容营销", "content marketing"),
    // networking / debugging
    ("抓包", "packet capture intercept sniff proxy traffic"),
    ("代理", "proxy forward reverse"),
    ("证书", "certificate tls ssl cert"),
    ("流量", "traffic network bandwidth"),
    ("请求", "request http api fetch"),
    ("拦截", "intercept hook middleware"),
    ("调试", "debug debugger devtools"),
    ("断点", "breakpoint debug"),
    // mobile / mini-program
    ("微信", "wechat weixin"),
    ("聊天", "chat messaging conversation"),
    ("消息", "message messaging inbox"),
    ("社交", "social community feed"),
    ("协作", "collaboration workspace productivity saas"),
    ("小程序", "mini program miniapp"),
    ("支付宝", "alipay"),
    ("原生", "native platform"),
    ("混合", "hybrid webview"),
    ("热更新", "hot reload update"),
    // general tools
    ("工具", "tool utility"),
    ("插件", "plugin extension addon"),
    ("脚手架", "scaffold boilerplate cli"),
    ("模板", "template starter"),
    ("命令行", "cli command line terminal"),
    // website building / business categories — the michael-design corpus is English;
    // a Chinese business word MUST map to English category terms or a Chinese "做网站"
    // query matches nothing (or only random generic blueprints).
    ("网站", "website site landing page"),
    ("官网", "website landing page homepage brand"),
    ("建站", "website landing page"),
    ("网页", "web page website"),
    ("主页", "homepage landing"),
    ("首页", "homepage landing hero"),
    ("门户", "portal"),
    ("动效", "motion animation scroll interactive"),
    ("交互", "interactive interaction"),
    ("咖啡", "coffee cafe"),
    ("餐厅", "restaurant dining menu"),
    ("饭店", "restaurant dining"),
    ("菜单", "menu"),
    ("美食", "food dining restaurant"),
    ("烘焙", "bakery dessert"),
    ("甜品", "dessert bakery cake"),
    ("酒吧", "bar lounge cocktail"),
    ("奶茶", "tea drink cafe"),
    ("民宿", "lodge cabin bnb booking stay"),
    ("酒店", "hotel resort booking hospitality"),
    ("度假", "resort vacation travel"),
    ("预订", "booking reservation"),
    ("订座", "reservation booking"),
    ("旅游", "travel tourism trip"),
    ("旅行", "travel trip journey"),
    ("健身", "fitness gym workout"),
    ("瑜伽", "yoga wellness studio"),
    ("美容", "beauty salon spa"),
    ("理发", "barber salon"),
    ("宠物", "pet"),
    ("花店", "florist flower shop"),
    ("婚礼", "wedding event"),
    ("摄影", "photography portfolio gallery"),
    ("画廊", "gallery art exhibition"),
    ("作品集", "portfolio showcase project"),
    ("工作室", "studio agency creative"),
    ("设计公司", "design agency studio"),
    ("律所", "law firm legal attorney"),
    ("律师", "lawyer attorney legal"),
    ("房产", "real estate property listing"),
    ("地产", "real estate property"),
    ("汽车", "car automotive dealership"),
    ("装修", "interior renovation design"),
    ("家具", "furniture interior"),
    ("服装", "fashion clothing apparel"),
    ("珠宝", "jewelry luxury"),
    ("音乐", "music band artist"),
    ("乐队", "band music concert"),
    ("活动", "event conference"),
    ("会议", "conference summit event"),
    ("展览", "exhibition gallery museum"),
    ("博客", "blog editorial article"),
    ("新闻", "news magazine editorial"),
    ("杂志", "magazine editorial"),
    ("公益", "nonprofit charity"),
    ("慈善", "charity nonprofit donation"),
    ("动物", "animal pet rescue"),
    ("救助", "rescue shelter charity nonprofit"),
    ("领养", "adopt adoption rescue shelter"),
    ("教堂", "church community faith"),
    ("农场", "farm organic agriculture"),
    ("科技公司", "tech startup saas"),
    ("初创", "startup saas"),
    ("创业公司", "startup saas"),
    ("人工智能", "ai artificial intelligence"),
    ("智能体", "ai agent assistant"),
    ("仪表盘", "dashboard analytics"),
    ("后台管理", "dashboard admin panel"),
    ("个人网站", "personal portfolio website"),
    ("简历", "resume cv personal"),
    // replication / redesign / asset sourcing (仿站、改版、素材检索)
    ("仿站", "clone replicate reference website"),
    ("仿照", "replicate reference imitate"),
    ("仿一个", "replicate clone reference"),
    ("复刻", "replicate clone rebuild"),
    ("参考网站", "reference website"),
    ("重新设计", "redesign revamp"),
    ("改版", "redesign revamp refresh"),
    ("视觉升级", "visual redesign upgrade"),
    ("素材", "asset image media resource"),
    ("资源", "asset resource"),
    ("头像", "avatar portrait photo"),
    ("图片", "image photo picture"),
    ("视频", "video media"),
];

fn expand_query(q: &str) -> String {
    let mut extra = String::new();
    for (cn, en) in CN_EN {
        if q.contains(cn) {
            extra.push(' ');
            extra.push_str(en);
        }
    }
    if extra.is_empty() {
        q.to_string()
    } else {
        format!("{q}{extra}")
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    // CJK bigrams: consecutive Chinese chars are indexed as overlapping 2-char
    // tokens (standard CJK BM25 practice) so Chinese text in the corpus/query is
    // no longer silently dropped by the ASCII-only tokenizer.
    let mut cjk_prev: Option<char> = None;
    let push = |cur: &mut String, out: &mut Vec<String>| {
        if cur.len() >= 2 && !STOP.contains(&cur.as_str()) {
            out.push(cur.clone());
        }
        cur.clear();
    };
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
            cjk_prev = None;
        } else if is_cjk(ch) {
            push(&mut cur, &mut out);
            if let Some(prev) = cjk_prev {
                out.push(format!("{prev}{ch}"));
            }
            cjk_prev = Some(ch);
        } else {
            push(&mut cur, &mut out);
            cjk_prev = None;
        }
    }
    push(&mut cur, &mut out);
    out
}

/// Split one markdown doc into `##`-bounded sections. Content before the first `##`
/// (title + intro) becomes its own "Overview" chunk so nothing is lost.
fn chunk_markdown(body: &str) -> Vec<(String, String)> {
    let mut chunks: Vec<(String, String)> = Vec::new();
    let mut cur_title = String::from("Overview");
    let mut cur_buf = String::new();
    for line in body.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            if !cur_buf.trim().is_empty() {
                chunks.push((cur_title.clone(), cur_buf.clone()));
            }
            cur_title = h.trim().to_string();
            cur_buf = format!("## {}\n", h.trim());
        } else {
            cur_buf.push_str(line);
            cur_buf.push('\n');
        }
    }
    if !cur_buf.trim().is_empty() {
        chunks.push((cur_title, cur_buf));
    }
    chunks
}

fn load(dir: &str) -> KnowledgeIndex {
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut domains_map: HashMap<String, Vec<String>> = HashMap::new();

    // Walk dir/<domain>/<topic>.md (one level of domain subdirs).
    if let Ok(domain_entries) = std::fs::read_dir(dir) {
        for de in domain_entries.flatten() {
            if !de.path().is_dir() {
                continue;
            }
            let domain = de.file_name().to_string_lossy().to_string();
            if let Ok(files) = std::fs::read_dir(de.path()) {
                for fe in files.flatten() {
                    let p = fe.path();
                    if p.extension().and_then(|e| e.to_str()) != Some("md") {
                        continue;
                    }
                    let topic = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let body = match std::fs::read_to_string(&p) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    domains_map
                        .entry(domain.clone())
                        .or_default()
                        .push(topic.clone());
                    for (section, text) in chunk_markdown(&body) {
                        // Index over section title + text (title weighted by repetition).
                        // Blueprint headers end in a slug like "[sites/coffee-shop-landing]" —
                        // those slug words ARE the business category, so weight them ×3 on
                        // top of the ×2 title so a category query beats the generic style
                        // words (card/grid/tailwind…) that appear in every blueprint.
                        let slug = section
                            .rfind('[')
                            .map(|i| {
                                section[i + 1..]
                                    .trim_end_matches(']')
                                    .replace(['/', '-', '_'], " ")
                            })
                            .unwrap_or_default();
                        let toks =
                            tokenize(&format!("{section} {section} {slug} {slug} {slug} {text}"));
                        if toks.len() < 3 {
                            continue;
                        }
                        let mut tf: HashMap<String, u32> = HashMap::new();
                        for t in &toks {
                            *tf.entry(t.clone()).or_insert(0) += 1;
                        }
                        let len = toks.len() as u32;
                        chunks.push(Chunk {
                            domain: domain.clone(),
                            topic: topic.clone(),
                            section,
                            text,
                            tf,
                            len,
                        });
                    }
                }
            }
        }
    }

    // Document frequencies + average length.
    let mut df: HashMap<String, u32> = HashMap::new();
    let mut total_len: u64 = 0;
    for c in &chunks {
        total_len += c.len as u64;
        for term in c.tf.keys() {
            *df.entry(term.clone()).or_insert(0) += 1;
        }
    }
    let avg_len = if chunks.is_empty() {
        1.0
    } else {
        total_len as f64 / chunks.len() as f64
    };

    let mut domains: Vec<(String, Vec<String>)> = domains_map.into_iter().collect();
    domains.sort_by(|a, b| a.0.cmp(&b.0));
    for d in &mut domains {
        d.1.sort();
    }

    tracing::info!(
        "[knowledge] indexed {} sections across {} domains from {}",
        chunks.len(),
        domains.len(),
        dir
    );
    KnowledgeIndex {
        chunks,
        df,
        avg_len,
        domains,
    }
}

pub fn get() -> &'static KnowledgeIndex {
    INDEX.get_or_init(|| {
        let dir = std::env::var("KNOWLEDGE_DIR").unwrap_or_else(|_| "./knowledge".to_string());
        load(&dir)
    })
}

/// BM25 search over the corpus. `domain` optionally restricts to one domain.
pub fn search(query: &str, domain: Option<&str>, top_k: usize) -> Vec<SearchHit> {
    search_inner(query, domain, top_k, None)
}

/// Like `search`, but the caller controls the per-hit text cap (bytes, truncated on a
/// char boundary) and gets exactly `top_k` hits (clamped to 20) even for michael-design.
/// The server-side design-knowledge injection needs a wide candidate pool that it filters
/// itself, with tighter per-section budgets than the interactive `knowledge_search` tool.
pub fn search_with_cap(
    query: &str,
    domain: Option<&str>,
    top_k: usize,
    cap: usize,
) -> Vec<SearchHit> {
    search_inner(query, domain, top_k, Some(cap))
}

fn search_inner(
    query: &str,
    domain: Option<&str>,
    top_k: usize,
    cap_override: Option<usize>,
) -> Vec<SearchHit> {
    const K1: f64 = 1.5;
    const B: f64 = 0.75;
    let idx = get();
    if idx.chunks.is_empty() {
        return Vec::new();
    }
    let q_toks = tokenize(&expand_query(query));
    if q_toks.is_empty() {
        return Vec::new();
    }
    // Resolve a possibly-loose domain to a real one. Models guess "backend" / "frontend"
    // when the real folders are "backend-api" / "web-frontend" — an exact filter then
    // returns NOTHING. Match exact → substring (either way) → else ignore the filter
    // (search all) so a wrong domain guess never silently yields zero results.
    let resolved_domain: Option<String> = domain.and_then(|d| {
        let d = d.trim().to_lowercase();
        if d.is_empty() {
            return None;
        }
        if idx.domains.iter().any(|(dom, _)| dom.to_lowercase() == d) {
            return Some(d);
        }
        idx.domains
            .iter()
            .map(|(dom, _)| dom.to_lowercase())
            .find(|dom| dom.contains(&d) || d.contains(dom.as_str()))
    });
    let n = idx.chunks.len() as f64;
    let mut scored: Vec<(usize, f64)> = Vec::new();
    for (i, c) in idx.chunks.iter().enumerate() {
        if let Some(ref d) = resolved_domain {
            if c.domain.to_lowercase() != *d {
                continue;
            }
        }
        let mut score = 0.0;
        for term in &q_toks {
            let f = match c.tf.get(term) {
                Some(f) => *f as f64,
                None => continue,
            };
            let dfi = *idx.df.get(term).unwrap_or(&0) as f64;
            if dfi == 0.0 {
                continue;
            }
            let idf = (1.0 + (n - dfi + 0.5) / (dfi + 0.5)).ln();
            let denom = f + K1 * (1.0 - B + B * (c.len as f64 / idx.avg_len));
            score += idf * (f * (K1 + 1.0)) / denom;
        }
        if score >= 2.0 {
            scored.push((i, score));
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    // michael-design hits are complete production-grade blueprints (4–11KB each):
    // truncating them to 2400 bytes cuts off the layout/motion/interaction specs —
    // exactly the part the agent needs — and there is no follow-up "read more" API.
    // So for an explicit michael-design search return FEWER hits but each IN FULL;
    // other domains keep the tight cap (their sections are short best-practice notes).
    let design_mode = resolved_domain.as_deref() == Some("michael-design");
    let take = if cap_override.is_some() || !design_mode {
        top_k.clamp(1, 20)
    } else {
        top_k.clamp(1, 3)
    };
    // BM25 的长度归一化会把「几乎没有正文的小节」推到前面：查 "hero section" 时返回的前三名
    // 曾经是 373/262/305 字节的标题壳。放在别的域没什么，但 michael-design 只发 3 条，
    // 一条空壳就是三分之一的名额没了——而模型拿不到可抄的蓝图，就只能凭印象编。
    //
    // 处理方式是"够用就不动"：只有当确实存在足够多的实心命中时，才把小节壳滤掉；
    // 否则宁可返回壳，也不返回空。
    // 只在**名额稀缺**的那条路上滤：工具路径只发 3 条，一条壳就是三分之一没了。
    // 系统提示词的注入路径取 12 条、另有总量预算，滤掉小节只会让每条都换成大块，
    // 白白把提示词顶大——那里一条壳的代价很小。
    if design_mode && cap_override.is_none() {
        const MIN_USEFUL_BYTES: usize = 900;
        let solid = scored
            .iter()
            .filter(|(i, _)| idx.chunks[*i].text.len() >= MIN_USEFUL_BYTES)
            .count();
        if solid >= take {
            scored.retain(|(i, _)| idx.chunks[*i].text.len() >= MIN_USEFUL_BYTES);
        }
    }
    scored
        .into_iter()
        .take(take)
        .map(|(i, score)| {
            let c = &idx.chunks[i];
            // Cap each returned section so a few hits don't blow the agent's context.
            // Truncate on a CHAR boundary — byte-slicing mid-UTF-8 (the corpus has →/—
            // and other multibyte chars) panics.
            let cap = cap_override.unwrap_or(if design_mode { 12_000 } else { 2400 });
            let text = if c.text.len() > cap {
                let mut end = cap;
                while end > 0 && !c.text.is_char_boundary(end) {
                    end -= 1;
                }
                format!(
                    "{}\n…(本节后续略，需要更多细节可再 read 或继续问)",
                    &c.text[..end]
                )
            } else {
                c.text.clone()
            };
            SearchHit {
                domain: c.domain.clone(),
                topic: c.topic.clone(),
                section: c.section.clone(),
                text,
                score: (score * 1000.0).round() / 1000.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod knowledge_design_tests {
    /// michael-design 只发 3 条，一条空壳就是三分之一的名额没了。
    ///
    /// BM25 的长度归一化天然偏袒短文档：查 "hero section" 时前三名曾经是 373/262/305 字节的
    /// 标题壳——模型拿不到可抄的蓝图，只能凭印象编，页面就难看。这条钉住"有实心命中时
    /// 不许让壳占位"。
    #[test]
    fn design_search_does_not_waste_its_three_slots_on_stubs() {
        let hits = super::search("hero section", Some("michael-design"), 6);
        assert!(!hits.is_empty(), "查不到任何东西，说明语料没加载");
        for h in &hits {
            assert!(
                h.text.len() >= 900,
                "空壳占了名额：{} ({} 字节)",
                h.section,
                h.text.len()
            );
        }
    }

    /// 具体做法层必须在语料里，且检索得到。
    ///
    /// 这批内容原本躺在 prompts/css_concrete_tokens.txt 里，既不在 PROMPT_NAMES 也不在
    /// prompt_graph 里——写好了却从不注入，等于不存在。搬进语料后由 knowledge_search 承载：
    /// 不占提示词预算，按需取。
    #[test]
    fn concrete_css_craft_is_in_the_corpus_and_retrievable() {
        let hits = super::search("card surface craft inner highlight shadow", Some("michael-design"), 6);
        assert!(
            hits.iter().any(|h| h.topic == "concrete-css-craft"),
            "做法层检索不到：{:?}",
            hits.iter().map(|h| h.topic.as_str()).collect::<Vec<_>>()
        );
    }
}
