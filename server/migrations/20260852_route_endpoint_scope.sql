-- 出口可以只承载线路的一部分模型，也可以用另一种协议。
--
-- # 为什么需要「这个出口有哪些模型」
--
-- 转卖商之间的货不一样：同一条 Claude 线路挂三个出口，可能只有一个真的有 opus-5，
-- 另外两个只有 sonnet。没有这一列的话，opus-5 的请求也会被派到没有它的出口上，
-- 撞一个 404 再换下一个 —— 而每个请求只有两次机会，这一撞就浪费掉一半。
--
-- 空数组 = 这个出口承载线路的全部模型。这是默认值，也就是「不填就和以前一样」。
ALTER TABLE route_endpoints
    ADD COLUMN IF NOT EXISTS enabled_models TEXT[] NOT NULL DEFAULT '{}';

-- # 为什么需要「这个出口用什么协议」
--
-- 协议是**这条线怎么说话**，不是这条线路卖什么。同一个 Claude，官方直连走 Anthropic 原生
-- /v1/messages，而一堆便宜转卖只提供 OpenAI 兼容的 /chat/completions。没有这一列，
-- 那类转卖就根本挂不上来 —— 而它们恰恰是最便宜的那批。
--
-- 空串 = 跟线路一样。价格、计费方式这些**故意不在这里**：那些是线路的身份，
-- 出口只管怎么把请求送出去。见 route_endpoints.rs 开头。
ALTER TABLE route_endpoints
    ADD COLUMN IF NOT EXISTS protocol TEXT NOT NULL DEFAULT '';
