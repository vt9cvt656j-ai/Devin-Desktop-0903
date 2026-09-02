-- 「Claude 强力版」线路标记。
--
-- 运维在后台新增/编辑线路时勾上它，这条线路就成为强力版的承载线路；IDE 的 Claude
-- 模型卡片右上角有一个开关，打开后**这一轮请求只走勾了这个标记的线路**。
--
-- 为什么是线路级而不是模型级：同一个模型名（claude-opus-5）可以同时挂在普通线路和
-- 强力线路上，区别在于背后的通道质量/价格，而不在模型本身。用线路做开关，运维加一条
-- 新通道时勾一下就接上了，不用回来改代码或逐个模型配。
ALTER TABLE models
  ADD COLUMN IF NOT EXISTS power_route boolean NOT NULL DEFAULT false;
