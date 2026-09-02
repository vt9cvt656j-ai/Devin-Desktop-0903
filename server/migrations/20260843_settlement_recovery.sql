-- 结算恢复：让「上游已服务、但结算事务失败（回滚→没扣到钱）」的调用能被后台补扣一次，
-- 且**绝不重复扣钱**。核心是一张按 settlement_id 做主键的幂等账本。
--
-- 为什么不能用 (user_id, request_id) 当幂等键：一个 x-ide-request-id 在客户端超时重试时会
-- 在服务端产生多条 model_usage（结算查询端点正是对它做 SUM、attempt_count 可 >1）——它是
-- 1:N，不能标识「这一笔」。settlement_id 每次 bill() 新生成、每笔唯一，才是对的锚点。

-- 幂等账本：一行 = 一笔已应用的结算。付费结算在同一个事务里写这行，于是「扣了钱」与
-- 「记了账」共命运：事务提交则两者都在；回滚则都不在。恢复时先查这张表——
-- 若某笔的原始提交其实落库了（commit 报错但数据已提交的「模糊提交」），这里就有它，
-- 恢复据此跳过、不再扣第二次。
CREATE TABLE IF NOT EXISTS settled_requests (
    settlement_id UUID PRIMARY KEY,
    settled_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- 保留期清理用（恢复只关心近几分钟内的失败；旧账本行没用了）。
CREATE INDEX IF NOT EXISTS idx_settled_requests_at ON settled_requests (settled_at);

-- 失败结算的持久队列。后台 worker 逐条幂等补扣。字段是重跑一次 bill() 所需的全部输入快照。
CREATE TABLE IF NOT EXISTS unsettled_charges (
    settlement_id UUID PRIMARY KEY,          -- 恢复时复用它，于是账本认领是「精确一次」
    user_id       UUID NOT NULL,
    conn_id       UUID NOT NULL,
    request_id    TEXT,                       -- 仅供对账/排查，不作幂等键
    cost_cents    BIGINT NOT NULL,
    use_quota     BOOLEAN NOT NULL,
    free_pool     BOOLEAN NOT NULL,
    free_micro_usd BIGINT NOT NULL,
    prompt_tokens BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens BIGINT NOT NULL DEFAULT 0,
    cache_creation_tokens BIGINT NOT NULL DEFAULT 0,
    model_name    TEXT NOT NULL DEFAULT '',
    estimated     BOOLEAN NOT NULL DEFAULT true,
    ide_mode      TEXT,
    is_tool_turn  BOOLEAN,
    emitted_tool  TEXT,
    stage         TEXT NOT NULL,              -- 是哪个失败分支入的队（排查用）
    attempts      INT NOT NULL DEFAULT 0,
    last_error    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at   TIMESTAMPTZ                 -- 补扣成功/确认已扣后置位；NULL = 待处理
);
-- 恢复扫描：待处理、最旧优先。
CREATE INDEX IF NOT EXISTS idx_unsettled_pending ON unsettled_charges (created_at) WHERE resolved_at IS NULL;

-- 同时把 settlement_id 记到用量行，让一笔扣费端到端可追。可空：老行没有，新老都不影响。
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS settlement_id UUID;
