-- 模型能接收/产出哪些模态。
--
-- **单独一个迁移**，不是回去改 20260838：那一份已经在生产上执行过了，sqlx 会为每个
-- 已执行的迁移记校验和，改动它的内容会让下次启动直接报
-- `migration 20260838 was previously applied but has been modified` 并**拒绝启动**——
-- 我就是这么把线上打挂过一次的（2026-08-16，后端 restart loop）。已执行的迁移不可变，
-- 加列一律新开一个。
--
-- 空数组 = 目录没给这一项，调用方回落到按模型名猜，**不是**"这个模型没有任何模态"。
ALTER TABLE model_catalog
  ADD COLUMN IF NOT EXISTS input_modalities  jsonb NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS output_modalities jsonb NOT NULL DEFAULT '[]'::jsonb;
