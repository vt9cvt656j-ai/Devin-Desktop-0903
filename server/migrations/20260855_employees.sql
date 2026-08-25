-- 智能员工：能替你看管 Mr. Day One 的自主智能体。
--
-- # 这套东西的难点不在功能，在边界
--
-- 它面对的是一个**在跑的生意**：174 个真实用户、真实收款、一台在服务的服务器。
-- 一个能「维护服务器」的员工也能把站点弄挂；一个能「管理用户」的员工也能把余额改错。
-- 所以边界不是事后加的保护，是这张表的第一性设计：
--
--   · 每个员工只有**白名单**里的能力，没勾的它连数据都看不到；
--   · 每个能力自带**风险档位**，档位决定它能不能自己动手；
--   · 越过档位的动作一律进审批队列，等你点头；
--   · 最危险的那一档系统**永远不执行**，只写建议给你，命令你自己跑。
--
-- 四个档位（详见 employees.rs 的能力表）：
--   T0 看     —— 永远自动。读数据不会把任何东西改坏。
--   T1 运维   —— 可逆的操作（下架/恢复出口、调次序、触发探测）。可以配成自动。
--   T2 影响用户 —— 改价、改额度、群发。**永远要批准**，因为错了是用户替你承担。
--   T3 危险   —— 服务器命令、非只读 SQL。**系统永远不执行**，只提建议。
CREATE TABLE IF NOT EXISTS employees (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    -- 职责描述。它会进系统提示词，所以这里写得越具体，它的判断越贴谱。
    role        TEXT NOT NULL DEFAULT '',
    -- 用哪个模型干活。指向 models 表 —— 员工用的是你自己的线路，
    -- 不额外接一家、不额外一个密钥。线路被删时置空，员工停工而不是崩掉。
    model_route UUID REFERENCES models (id) ON DELETE SET NULL,
    model_id    TEXT NOT NULL DEFAULT '',
    -- 能力白名单。存能力的标识符，比如 ["read.health","ops.probe"]。
    -- 空 = 什么都不能做，包括看 —— 默认最小权限，不是默认全开。
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    -- 自己动手的上限：'none' 只提建议 / 't1' 可以做可逆运维。
    -- 刻意没有 't2'、't3' 这两个值：影响用户和危险动作永远要人点头，
    -- 不给一个「配置成全自动」的开关，因为那个开关迟早会有人打开。
    autonomy    TEXT NOT NULL DEFAULT 'none',
    -- 多久自己跑一次（分钟）。0 = 只在你手动点的时候跑。
    every_minutes INTEGER NOT NULL DEFAULT 0,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    last_run_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT employees_autonomy_known CHECK (autonomy IN ('none', 't1'))
);

CREATE INDEX IF NOT EXISTS idx_employees_due
    ON employees (enabled, every_minutes, last_run_at);

-- 每跑一次留一条工作记录。
--
-- 这既是给你看的（它看了什么、得出什么结论、动了什么），也是员工之间**唯一**的协调渠道：
-- 一个「主管」员工可以读别人的工作记录再决定做什么。刻意不做智能体之间的自由消息传递
-- —— 那种协调没法审计，出了事没人说得清是谁让谁干的。
CREATE TABLE IF NOT EXISTS employee_runs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    employee_id UUID NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    -- manual / scheduled
    trigger     TEXT NOT NULL DEFAULT 'manual',
    -- ok / failed
    status      TEXT NOT NULL DEFAULT 'ok',
    -- 一句话结论，列表里显示这个。
    summary     TEXT NOT NULL DEFAULT '',
    -- 完整的发现和推理。
    detail      TEXT NOT NULL DEFAULT '',
    -- 它看了哪些数据（能力标识符），出问题时用来判断是不是给的信息不够。
    used        TEXT[] NOT NULL DEFAULT '{}',
    error       TEXT NOT NULL DEFAULT '',
    tokens_in   BIGINT NOT NULL DEFAULT 0,
    tokens_out  BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_employee_runs_recent
    ON employee_runs (employee_id, created_at DESC);

-- 员工想做的每一个动作。
--
-- 自动执行的也在这里留一行（status='done'），不是只记要批准的 —— 否则「它到底动过什么」
-- 只能靠翻日志，而这正是自主系统最需要说清楚的一件事。
CREATE TABLE IF NOT EXISTS employee_actions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id      UUID NOT NULL REFERENCES employee_runs (id) ON DELETE CASCADE,
    employee_id UUID NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    -- 能力标识符，如 ops.delist
    capability  TEXT NOT NULL,
    -- 参数，形状由能力自己定义
    args        JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- 它为什么要做这件事。批准与否主要看这一句。
    reason      TEXT NOT NULL DEFAULT '',
    tier        SMALLINT NOT NULL DEFAULT 0,
    -- pending 等批准 / done 已执行 / rejected 你否了 / failed 执行失败 / advice 只是建议
    status      TEXT NOT NULL DEFAULT 'pending',
    result      TEXT NOT NULL DEFAULT '',
    decided_by  UUID REFERENCES users (id) ON DELETE SET NULL,
    decided_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 审批队列按「还等着的」查，所以这个索引只覆盖 pending。
CREATE INDEX IF NOT EXISTS idx_employee_actions_pending
    ON employee_actions (created_at DESC) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_employee_actions_run
    ON employee_actions (run_id, created_at);
