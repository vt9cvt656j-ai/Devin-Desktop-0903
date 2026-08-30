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

/// 工程路径对 `CN_EN` 的**覆盖 + 增补**，不是替换。
///
/// 为什么要覆盖：`CN_EN` 是照着**设计路径**调的。「后台管理 → dashboard admin panel」
/// 「网站 → website site landing page」全是 UI 词，对着 michael-design 那 452 段检索时
/// 完全正确。但工程路径（`search_excluding` 排掉设计蓝本）共用同一张表之后，同样的词
/// 落进了完全不同的语料："admin panel" 撞上渗透测试的攻击面、"terminal" 撞上逆向工程的
/// binary triage、"landing page" 剩下的最近邻是 CSRF。实测 20 条真实请求，跑偏的 5 条
/// 全是这么来的。
///
/// 为什么**不是**整张替换：`CN_EN` 里绝大多数条目（考试、多租户、电商、医疗、支付……）
/// 在两条路上都对。整张换掉的话它们一起消失 —— 实测「做个在线考试」当场从命中变成零命中。
/// 所以这里只列**两种**条目：工程侧要改写的（键在 CN_EN 里存在，值不一样），
/// 以及 CN_EN 根本没有的工程词。
///
/// 空值 `""` 是有意义的：表示这个词在工程路径上**不扩展**。语料里没有对应内容时，
/// 扩展只会把它推向最近的无关邻居（「命令行」→ terminal → 逆向工程）——
/// 那比不给更糟，因为噪音会占掉仅有的 2 个名额。
///
/// 判据是**语料里真的有什么**，不是概念上该有什么：每一条都对着 knowledge/ 下的英文
/// 正文挑过词。漏一条只是退回原样、没帮上忙，不会帮倒忙 —— 这是它敢用手工表的前提。
const CN_EN_ENGINEERING: &[(&str, &str)] = &[
    // —— 覆盖：这几个词在 CN_EN 里映射到 UI 术语，工程路径上会带偏 ——
    ("后台管理", "crud schema authorization rbac admin"),
    ("命令行", ""),
    ("工具", ""),
    ("网站", ""),
    ("官网", ""),
    ("建站", ""),
    ("网页", ""),
    ("首页", ""),
    ("博客", "seo content indexing sitemap"),
    ("原生", ""),
    ("模板", ""),
    ("脚手架", "scaffold project structure"),
    ("聊天", "websocket realtime message delivery presence"),
    // —— 那 8 条零命中，逐条对着语料正文挑的词 ——
    ("会员", "subscription plan tier entitlement membership"),
    ("订阅", "subscription billing plan tier"),
    ("数据同步", "pipeline idempotent upsert incremental sync backfill"),
    ("同步", "sync replication idempotent"),
    ("记账", "ledger double-entry bookkeeping transaction reconciliation"),
    ("账目", "ledger reconciliation"),
    ("抢票", "inventory oversell reservation lock optimistic concurrency"),
    ("秒杀", "inventory oversell flash sale lock contention"),
    ("超卖", "oversell inventory lock"),
    ("爬虫", "crawler scraping rate limit retry backoff politeness robots"),
    ("抓取", "crawl fetch rate limit backoff"),
    ("审批", "state machine workflow transition approval"),
    ("流程", "state machine workflow transition"),
    ("工作流", "state machine workflow orchestration"),
    ("即时通讯", "websocket realtime message delivery presence"),
    ("实时", "websocket realtime streaming push"),
    ("灰度", "canary progressive rollout feature flag blue green"),
    ("发布", "deploy release rollout rollback"),
    ("回滚", "rollback revert deploy"),
    // —— 常见工程意图，语料里都有对应正文 ——
    ("接口", "api endpoint contract versioning"),
    ("鉴权", "auth authentication authorization token session"),
    ("登录", "authentication login session token password"),
    ("权限", "authorization rbac permission scope"),
    ("索引", "index query plan explain analyze"),
    ("慢查询", "slow query index query plan explain"),
    ("分页", "pagination cursor keyset offset"),
    ("事务", "transaction isolation acid rollback"),
    ("并发", "concurrency lock contention race"),
    ("高并发", "concurrency throughput event loop backpressure"),
    ("限流", "rate limit throttle token bucket backpressure"),
    ("重试", "retry backoff idempotent"),
    ("幂等", "idempotent idempotency key"),
    ("缓存", "cache invalidation ttl stampede"),
    ("队列", "queue worker background job"),
    ("消息队列", "message queue broker consumer producer"),
    ("微服务", "microservice service boundary decomposition"),
    ("拆分", "decomposition boundary service split"),
    ("部署", "deploy rollout container orchestration"),
    ("容器", "container docker image runtime"),
    ("监控", "observability metrics logging tracing alert"),
    ("日志", "logging structured log observability"),
    ("测试", "test coverage integration unit fixture"),
    ("上传", "upload file validation storage content type"),
    ("文件", "file storage upload path"),
    ("搜索", "search index query ranking"),
    ("推送", "push notification websocket delivery"),
    ("多租户", "multi-tenant isolation tenant row level security"),
    ("对账", "reconciliation ledger settlement"),
    ("支付", "payment gateway idempotency settlement webhook"),
    ("退款", "refund reversal settlement"),
    ("表结构", "schema modeling normalization migration"),
    ("建模", "schema modeling entity relationship"),
    ("迁移", "migration schema versioning backfill"),
    ("选型", "selection tradeoff comparison when to use"),
    // —— 中文工程里最高频的那几个通用词。刻意不针对评测集里的失败项造词：
    //     这些是任何中文工程请求都会出现的词，映射到语料真正用的术语。
    ("后端", "backend server api service"),
    ("服务端", "backend server api service"),
    ("前端", "frontend client browser ui rendering"),
    ("服务", "service api endpoint"),
    ("防御", "prevent mitigation defense validation sanitize"),
    ("防止", "prevent mitigation validation"),
    ("加固", "hardening mitigation defense"),
    ("安全", "security validation sanitize authentication authorization"),
    ("性能", "performance latency throughput profiling"),
    ("优化", "optimization performance index caching"),
    ("架构", "architecture boundary decomposition tradeoff"),
    ("扩展", "scalability horizontal scaling partition"),
    ("可靠", "reliability retry idempotent failover"),
    ("容灾", "failover redundancy backup recovery"),
    ("备份", "backup restore recovery snapshot"),
    ("聊天室", "websocket realtime message delivery presence room"),
    ("在线", "realtime presence websocket online"),
];

/// 查询扩展。`engineering` 决定用哪张映射表。
///
/// 分路不是可选的：同一个中文词在两条路上要映射到**不同的英文词**。
/// 「后台管理」对设计路径是 dashboard / admin panel（UI 版式），对工程路径是
/// CRUD 建模 + 权限；把设计那份用在工程路径上，实测会命中渗透测试的攻击面。
fn expand_query_for(q: &str, engineering: bool) -> String {
    let mut extra = String::new();
    let mut push = |en: &str| {
        if !en.is_empty() {
            extra.push(' ');
            extra.push_str(en);
        }
    };
    for (cn, en) in CN_EN {
        if !q.contains(cn) {
            continue;
        }
        if engineering {
            // 工程路径先看有没有覆盖。空值 = 这个词在工程路径上不扩展（见表头注释）。
            if let Some((_, over)) = CN_EN_ENGINEERING.iter().find(|(k, _)| k == cn) {
                push(over);
                continue;
            }
        }
        push(en);
    }
    if engineering {
        // 再加上 CN_EN 里根本没有的工程词。
        for (cn, en) in CN_EN_ENGINEERING {
            if q.contains(cn) && !CN_EN.iter().any(|(k, _)| k == cn) {
                push(en);
            }
        }
    }
    if extra.is_empty() {
        q.to_string()
    } else {
        format!("{q}{extra}")
    }
}

#[cfg(test)]
fn expand_query(q: &str) -> String {
    expand_query_for(q, false)
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
    search_inner(query, domain, top_k, None, None)
}

/// 和 `search` 一样，但**排除某一个域**。
///
/// 唯一的用处是「无域自动检索」那条路：它只有 2 个名额，而 michael-design 一个域
/// 就占了全库 52%（468/893 段），实测 10 条真实中文建站/写工具请求，设计段拿走 13/20。
/// 于是「做个网站」拿回来的两段全是配色克制和信任信号，而 Database Selection
/// Decision Tree、Service Decomposition Rules、ORM & Driver Selection 一条都进不来 ——
/// 架构和选型全凭模型印象。
///
/// 设计活**另有专属注入通道**（design_knowledge_block），它在这条路上占名额是纯重复。
/// 排除的是过滤不是打分，限定域的路径和那条专属通道都不受影响。
/// **只在明确要它时才该出现的域。** 无域自动检索一律排除它们。
///
/// 判据不是「这些内容不好」，而是「它们各有专属的到达方式」：域路由按画像旗标走
/// （`semantic_knowledge_domain`），真做渗透测试或逆向工程时会被限定到那个域，
/// 走的是另一条路、拿的是 4 个名额。而这条无域路径服务的是**普通写代码**的请求，
/// 只有 2 个名额。
///
/// 不排会怎样（实测）：
///   · michael-design 452 段占全库 54.6%，「做个网站」拿回来两段配色克制
///   · penetration-testing + reverse-engineering 58 段全是攻击视角，而攻防用同一批词——
///     「怎么防 SQL 注入」命中 web-exploitation（怎么打），「写个后台管理」命中凭据攻击，
///     「帮我写个命令行工具」命中 binary triage
/// 三种都不是检索器算错了，是这些段本来就不该来竞争这 2 个名额。
pub const AUTO_EXCLUDED_DOMAINS: &[&str] =
    &["michael-design", "penetration-testing", "reverse-engineering"];

pub fn search_excluding(
    query: &str,
    domain: Option<&str>,
    top_k: usize,
    exclude_domain: &str,
) -> Vec<SearchHit> {
    search_inner(query, domain, top_k, None, Some(exclude_domain))
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
    search_inner(query, domain, top_k, Some(cap), None)
}

fn search_inner(
    query: &str,
    domain: Option<&str>,
    top_k: usize,
    cap_override: Option<usize>,
    exclude_domain: Option<&str>,
) -> Vec<SearchHit> {
    const K1: f64 = 1.5;
    const B: f64 = 0.75;
    let idx = get();
    if idx.chunks.is_empty() {
        return Vec::new();
    }
    // 排除设计蓝本 = 这是工程路径，用工程那张映射表。判据不是新加的：
    // `exclude_domain` 本来就只有无域自动检索那一条路会传（见 prompts.rs 的调用点）。
    let q_toks = tokenize(&expand_query_for(query, exclude_domain.is_some()));
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
        // 排除某个域。只在「无域自动检索」那条路上用，见 `search_excluding`。
        // 放在打分**之前**：这是过滤，不是降权 —— 降权会让它在别的段都不相关时
        // 又冒出来，而它在这条路上占名额本来就是纯重复。
        // 传了排除信号 = 这是无域自动检索那条路。排的不只是传进来那一个域，
        // 而是整组「只在明确要它时才该出现的域」—— 见 AUTO_EXCLUDED_DOMAINS 的说明。
        // 参数保留成单个域是为了不动调用点：它的取值本来就只有 michael-design 一个。
        if exclude_domain.is_some()
            && AUTO_EXCLUDED_DOMAINS
                .iter()
                .any(|ex| c.domain.eq_ignore_ascii_case(ex))
        {
            continue;
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
    // 问配色时，配色库必须够得着。
    //
    // michael-design 只发 3 条，而提示词明确教模型去查 `<品类> palette`——实测
    // `law firm palette` 里配色库排第 4、`律所 palette` 排第 7、`portfolio palette` 也进不去，
    // 恰恰是最需要"取最接近的一套"的那些品类查不到。提示词许下的承诺检索兑现不了，
    // 模型只能回去自己编色。所以问配色时把它提到第一位；不问配色时一切照旧。
    if design_mode {
        let q = query.to_lowercase();
        let asks_palette = ["palette", "colour", "color", "配色", "色板", "颜色"]
            .iter()
            .any(|k| q.contains(k));
        if asks_palette {
            if let Some(at) = scored
                .iter()
                .position(|(i, _)| idx.chunks[*i].section.to_lowercase().contains("curated palette library"))
            {
                let hit = scored.remove(at);
                scored.insert(0, hit);
            }
        }
    }
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
    // **一个文件最多占一个名额**（只在名额很少时生效）。
    //
    // 无域自动检索只有 2 个名额，而 BM25 会把同一份文件里几个相邻小节一起推上来 ——
    // 实测「登录怎么做才安全」拿回的是 security/appsec 的两个小节，2 个名额只覆盖 1 个文件，
    // 等于一半的窗口白给。同一文件的另一节能补的信息，远不如换一个文件来得多。
    //
    // 只在 take <= 3 时生效：明确按域检索（4 个名额）和 michael-design（要整份蓝图）
    // 本来就该让同一份文件多占几条，那时相邻小节是**连续的正文**，不是重复。
    if take <= 3 {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        scored.retain(|(i, _)| {
            let c = &idx.chunks[*i];
            let key = (c.domain.as_str(), c.topic.as_str());
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
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
    /// 提示词教模型去查 `<品类> palette`，检索就必须兑现。
    ///
    /// michael-design 只发 3 条。实测过：`law firm palette` 里配色库排第 4、
    /// `律所 palette` 排第 7、`portfolio palette` 也进不去——恰恰是知识库里没有同品类
    /// 站点蓝本、最需要"取最接近的一套"的那些品类。提示词许诺了、检索给不出，
    /// 模型就只能回去自己编色，而"自己编色"正是配色难看的根源。
    #[test]
    fn 问配色时配色库一定够得着() {
        for q in [
            "law firm palette",
            "律所 palette",
            "portfolio palette",
            "cafe palette",
            "咖啡店 配色",
            "bookstore color palette",
        ] {
            let hits = super::search(q, Some("michael-design"), 6);
            assert!(
                hits.iter().any(|h| h.section.to_lowercase().contains("curated palette library")),
                "「{q}」查不到配色库，只发 3 条时它被挤掉了：{:?}",
                hits.iter().map(|h| h.section.as_str()).collect::<Vec<_>>()
            );
        }
    }

    /// 不问配色时不该占用那 3 个名额。
    #[test]
    fn 不问配色时配色库不占名额() {
        for q in ["coffee shop hero section", "scroll animation motion"] {
            let hits = super::search(q, Some("michael-design"), 6);
            assert!(
                !hits.iter().any(|h| h.section.to_lowercase().contains("curated palette library")),
                "「{q}」没问配色，配色库不该占名额"
            );
        }
    }

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

#[cfg(test)]
mod design_share_guard {
    /// 无域自动检索只有 2 个名额，而全库 3218 段里 michael-design 占 **2535 段（78.8%）**。
    /// 设计活另有专属注入通道（prompts.rs 的 design_knowledge_block），所以设计段在这里
    /// 抢走名额是纯损失——一个工程回合本该拿到的服务拆分规则/选型决策树被顶掉。
    ///
    /// 但「占比高」不等于「实际抢得到」：BM25 只看词项匹配。实测（2026-08-23，真检索、
    /// 8 条典型工程查询、16 个名额）**设计蓝本只抢到 1 个，6.2%**。
    ///
    /// 所以**没有**给 search 加排除参数：为 6.2% 改一处检索核心，是拿新缺口换旧缺口。
    /// 这条测试把那个数字变成常驻守卫——语料配比一旦变到设计段真的开始抢名额，它会红，
    /// 那时再改才有依据。
    #[test]
    fn design_blueprints_barely_take_the_no_domain_slots() {
        let queries = [
            "how to split services and keep bounded contexts",
            "which database should I choose for a multi-tenant scheduling app",
            "rate limiting per tenant in an API gateway",
            "python ORM default choice and alternatives",
            "how to structure a realtime collaborative todo app",
            "HIPAA requirements for storing patient schedules",
            "refactor a large module without breaking callers",
            "websocket broadcast fanout and conflict resolution",
        ];
        let mut design = 0usize;
        let mut total = 0usize;
        for q in queries {
            for h in super::search(q, None, 2) {
                total += 1;
                if h.domain == "michael-design" {
                    design += 1;
                }
            }
        }
        assert!(total >= 12, "只命中 {total} 个名额，这条守卫失去落点（语料没加载？）");
        let share = design as f64 / total as f64;
        assert!(
            share <= 0.25,
            "设计蓝本抢走了 {:.1}% 的无域名额（{design}/{total}）——实测基线是 6.2%。\n             涨到这个程度就该给 search 加一个排除 michael-design 的信号了：\n             设计活另有专属注入通道，它在这里占名额是纯损失。",
            share * 100.0
        );
    }

    /// 上面那条守卫量的是**英文架构黑话**，而真实用户说的是中文。
    ///
    /// 用 10 条真实形状的中文请求量出来，设计段占 65%（13/20），远超那条 25% 的线 ——
    /// 也就是说守卫存在、却一直在量另一个查询分布，从没响过。
    /// 现在无域那条路会排除 michael-design（`search_excluding`），这条测的是它真的生效。
    /// 检索质量**不许倒退**。
    ///
    /// 上面那条 `retrieval_eval` 是给人看的（--ignored，跑起来会打一大片）。这一条是门：
    /// 每次改检索（同义表、排除域、去重、打分）都会被它拦一次。
    ///
    /// 基线是 2026-08-28 实测出来的：改之前 Recall@2 = 50.0% / MRR = 0.462 / 误伤 50%，
    /// 改之后 84.6% / 0.731 / 0%。这里的线**刻意压在实测值之下**（80% / 0.65 / 0 条），
    /// 留出标注集微调的余地 —— 它要拦的是「大幅倒退」，不是「小数点后一位变了」。
    ///
    /// 误伤那条是 0 容忍：语料里确实没有对应内容时硬塞一段，会占掉仅有的 2 个名额，
    /// 而模型被告知那是「与你的请求相关的工程参考」。宁可什么都不给。
    #[test]
    fn retrieval_quality_does_not_regress() {
        #[derive(serde::Deserialize)]
        struct Row {
            #[serde(default)]
            q: String,
            #[serde(default)]
            relevant: Vec<String>,
        }
        let rows: Vec<Row> = include_str!("../knowledge_eval.jsonl")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Row>(l).ok())
            .filter(|r: &Row| !r.q.is_empty())
            .collect();
        assert!(rows.len() >= 20, "评测集只剩 {} 条，这道门失去落点", rows.len());

        let (mut labeled, mut hit2, mut rr, mut empty, mut noise) = (0usize, 0usize, 0f64, 0usize, 0usize);
        let mut bad: Vec<String> = Vec::new();
        for r in &rows {
            let hits = super::search_excluding(&r.q, None, 2, "michael-design");
            let got: Vec<String> = hits.iter().map(|h| format!("{}/{}", h.domain, h.topic)).collect();
            if r.relevant.is_empty() {
                empty += 1;
                if hits.iter().any(|h| h.score >= 3.0) {
                    noise += 1;
                    bad.push(format!("误伤 {} → {:?}", r.q, got));
                }
                continue;
            }
            labeled += 1;
            match got.iter().position(|g| r.relevant.iter().any(|w| w == g)) {
                Some(0) => { hit2 += 1; rr += 1.0; }
                Some(_) => { hit2 += 1; rr += 0.5; }
                None => {}
            }
        }
        let recall = hit2 as f64 / labeled as f64;
        let mrr = rr / labeled as f64;
        assert_eq!(
            noise, 0,
            "语料里没有对应内容时塞了 {noise}/{empty} 条噪音 —— 它会占掉仅有的 2 个名额：\n  {}",
            bad.join("\n  ")
        );
        assert!(
            recall >= 0.80,
            "Recall@2 掉到 {:.1}%（{hit2}/{labeled}），基线 84.6%、下限 80%。\n\
             跑 cargo test --offline -- --ignored --nocapture retrieval_eval 看是哪几条漏了",
            recall * 100.0
        );
        assert!(mrr >= 0.65, "MRR@2 掉到 {mrr:.3}，基线 0.731、下限 0.65");
    }

    /// 工程路径和设计路径**必须用不同的映射表**。
    ///
    /// 这条守的是根因：`CN_EN` 是照设计路径调的（后台管理 → dashboard admin panel），
    /// 工程路径共用它的时候，同样的词落进完全不同的语料，实测 20 条真实请求里跑偏 5 条。
    #[test]
    fn the_two_paths_expand_queries_differently() {
        let design = super::expand_query_for("写个后台管理", false);
        let eng = super::expand_query_for("写个后台管理", true);
        assert_ne!(design, eng, "两条路的查询扩展一模一样 —— 那这次分路等于没做");
        assert!(design.contains("admin panel"), "设计路径的映射被改掉了");
        assert!(
            !eng.contains("admin panel"),
            "工程路径还在往查询里塞 UI 词 —— 它会命中渗透测试的攻击面：{eng}"
        );
        // 空值 = 这个词在工程路径上不扩展。语料里没有 CLI 工程内容，扩展只会把它
        // 推向最近的无关邻居（terminal → 逆向工程的 binary triage）。
        assert!(
            !super::expand_query_for("帮我写个命令行工具", true).contains("terminal"),
            "「命令行」在工程路径上还在扩展成 terminal"
        );
        // 基础表里两条路都对的那些不能因为分路而丢掉（实测「在线考试」曾当场从命中变零命中）。
        assert!(
            super::expand_query_for("做个在线考试", true).contains("quiz"),
            "分路把基础表里通用的那些一起丢了"
        );
    }

    /// 名额只有 2 个时，一个文件最多占一个。
    #[test]
    fn scarce_slots_are_not_wasted_on_one_file() {
        for q in ["登录怎么做才安全", "怎么防 SQL 注入", "数据库索引怎么建"] {
            let hits = super::search_excluding(q, None, 2, "michael-design");
            if hits.len() < 2 {
                continue;
            }
            assert_ne!(
                (hits[0].domain.as_str(), hits[0].topic.as_str()),
                (hits[1].domain.as_str(), hits[1].topic.as_str()),
                "「{q}」的两个名额被同一个文件占满了：{}/{}",
                hits[0].domain, hits[0].topic
            );
        }
    }

    /// 只在明确要它时才该出现的域，无域路径一律排除。
    #[test]
    fn attack_oriented_domains_stay_out_of_the_domain_less_path() {
        assert!(super::AUTO_EXCLUDED_DOMAINS.contains(&"penetration-testing"));
        assert!(super::AUTO_EXCLUDED_DOMAINS.contains(&"reverse-engineering"));
        assert!(super::AUTO_EXCLUDED_DOMAINS.contains(&"michael-design"));
        // 攻防用同一批词，防守型问题会被攻击视角的段压过（实测：「怎么防 SQL 注入」
        // 命中 web-exploitation「怎么打」）。真做渗透时走域路由，拿的是 4 个名额、另一条路。
        for q in ["怎么防 SQL 注入", "写个后台管理", "登录怎么做才安全", "帮我写个命令行工具"] {
            for h in super::search_excluding(q, None, 2, "michael-design") {
                assert!(
                    !super::AUTO_EXCLUDED_DOMAINS.contains(&h.domain.as_str()),
                    "「{q}」拿回了 {}/{} —— 那是只在明确要它时才该出现的域",
                    h.domain, h.section
                );
            }
        }
        // 而**明确按域检索**时它们照常可用，这条排除不该把它们变成死语料。
        assert!(
            !super::search("privilege escalation", Some("penetration-testing"), 3).is_empty(),
            "按域检索也拿不到了 —— 那是把语料废掉，不是排除"
        );
    }

    /// **检索质量的尺**。跑：cargo test --offline -- --ignored --nocapture retrieval_eval
    ///
    /// 为什么要有它：换检索器（同义表扩容 / 查询改写 / 向量）之前，「效果提升多少」只能靠感觉。
    /// 有了它，每一次改动都能给出一个可比的数，而不是又一句「感觉好多了」。
    ///
    /// 判据落在 **@2**：无域注入门只有 2 个名额（AUTO_KNOWLEDGE_MAX_HITS），
    /// 排第 3 名的命中对用户根本不存在。
    ///
    /// 三个数各回答一件事：
    ///   · Recall@2  —— 该给的给到了吗（有标注的那些查询）
    ///   · MRR@2     —— 给对了但排第几（第 1 名和第 2 名对模型的分量不同）
    ///   · 误伤率     —— 语料里**根本没有**对应内容时，有没有硬塞一段无关的
    ///                   （这一项比前两个更要紧：塞进去的噪音会占掉仅有的 2 个名额，
    ///                    而模型被告知那是「与你的请求相关的工程参考」）
    #[test]
    #[ignore = "评测用，不进常规套件"]
    fn retrieval_eval() {
        #[derive(serde::Deserialize)]
        struct Row {
            #[serde(default)]
            q: String,
            #[serde(default)]
            relevant: Vec<String>,
            #[serde(default)]
            note: String,
        }
        let raw = include_str!("../knowledge_eval.jsonl");
        let rows: Vec<Row> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Row>(l).ok())
            .filter(|r: &Row| !r.q.is_empty())
            .collect();
        assert!(rows.len() >= 20, "评测集只剩 {} 条，尺子失去意义", rows.len());

        let (mut labeled, mut hit1, mut hit2, mut rr) = (0usize, 0usize, 0usize, 0f64);
        let (mut empty_label, mut false_inject) = (0usize, 0usize);
        let mut misses: Vec<String> = Vec::new();
        let mut noise: Vec<String> = Vec::new();

        for r in &rows {
            // 生产无域路径：排除设计蓝本，取 2 个名额。
            let hits = super::search_excluding(&r.q, None, 2, "michael-design");
            let got: Vec<String> = hits
                .iter()
                .map(|h| format!("{}/{}", h.domain, h.topic))
                .collect();

            if r.relevant.is_empty() {
                // 语料里没有对应内容 —— 正确行为是**什么都不给**。
                empty_label += 1;
                // 注入门还有一道分数线；低于它的命中进不了提示词，不算误伤。
                if hits.iter().any(|h| h.score >= 3.0) {
                    false_inject += 1;
                    noise.push(format!("{} → {}", r.q, got.join(", ")));
                }
                continue;
            }
            labeled += 1;
            let rank = got.iter().position(|g| r.relevant.iter().any(|w| w == g));
            match rank {
                Some(0) => {
                    hit1 += 1;
                    hit2 += 1;
                    rr += 1.0;
                }
                Some(_) => {
                    hit2 += 1;
                    rr += 0.5;
                }
                None => misses.push(format!(
                    "{}\n      期望 {:?}\n      实得 {:?}  [{}]",
                    r.q, r.relevant, got, r.note
                )),
            }
        }

        let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 * 100.0 / b as f64 };
        println!("\n╭─ 平台知识库检索评测 ─────────────────────────────");
        println!("│ 语料 828 小节 / 65 文件 / 22 域；路径 = 无域自动检索（排除设计蓝本），名额 2");
        println!("│");
        println!("│ 有标注的查询 {labeled} 条");
        println!("│   Recall@1   {:>5.1}%   ({hit1}/{labeled})   排第一就命中", pct(hit1, labeled));
        println!("│   Recall@2   {:>5.1}%   ({hit2}/{labeled})   两个名额里命中", pct(hit2, labeled));
        println!("│   MRR@2      {:>5.3}", if labeled == 0 { 0.0 } else { rr / labeled as f64 });
        println!("│");
        println!("│ 语料里确实没有的查询 {empty_label} 条");
        println!("│   误伤率     {:>5.1}%   ({false_inject}/{empty_label})   该沉默却塞了噪音", pct(false_inject, empty_label));
        println!("╰──────────────────────────────────────────────────");
        if !misses.is_empty() {
            println!("\n漏掉的（{} 条）：", misses.len());
            for m in &misses {
                println!("  ✗ {m}");
            }
        }
        if !noise.is_empty() {
            println!("\n误伤的（{} 条）：", noise.len());
            for n in &noise {
                println!("  ⚠ {n}");
            }
        }
        println!();
    }

    #[test]
    fn the_domain_less_path_leaves_room_for_architecture() {
        let real_requests = [
            "做个网站",
            "帮我写个命令行工具",
            "做一个会员系统",
            "写个后台管理",
            "做个博客",
            "帮我搭个 API 服务",
            "做个小程序后端",
            "写个数据同步脚本",
            "做个多租户的排班系统",
            "帮我做个电商下单流程",
        ];
        let mut design = 0usize;
        let mut total = 0usize;
        for q in real_requests {
            for h in super::search_excluding(q, None, 2, "michael-design") {
                total += 1;
                if h.domain == "michael-design" {
                    design += 1;
                }
            }
        }
        assert!(total >= 8, "只命中 {total} 个名额，这条测试失去落点（语料没加载？）");
        assert_eq!(
            design, 0,
            "无域路径还是让设计蓝本占了 {design}/{total} 个名额 —— \n             架构和选型的段进不来，用户看到的是「配色讲得很细、技术选型全凭印象」",
        );

        // 上面测的是这个函数的行为；这一条钉的是**调用点真的在用它**。
        // 少了这条，把 prompts.rs 那边改回 `search(...)` 照样全绿，而线上就退回原样了。
        let call_site = include_str!("prompts.rs");
        assert!(
            call_site.contains("crate::knowledge::search_excluding(&query, domain, max_hits, \"michael-design\")"),
            "无域自动检索那条路没在用排除版 —— 设计蓝本会继续占掉那 2 个名额",
        );
        // 而且**只在无域时**排除：限定了域的请求（包括明确要 michael-design 的）不受影响。
        assert!(
            call_site.contains("let hits = if domain.is_some() {"),
            "排除没有限定在无域那一支 —— 明确要设计知识的请求会被一起挡掉",
        );
    }
}
