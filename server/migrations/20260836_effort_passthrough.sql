-- 每条上游线路：要不要把客户端拨的思考档位**原样**发给上游。
--
-- 背景：anthropic 桥一直把 Claude 一族的 high / xhigh / max 一律改写成
-- output_config.effort="high"（models.rs 的 effort 映射）。而 "high" 正是这一族的
-- API 默认值 —— 也就是说 IDE 上最深的那一档，发出去的东西和什么都不发一模一样，
-- 深度旋钮在这条线路上是个摆设。
--
-- 那个天花板的理由写在注释里：「这是转卖渠道不是 Anthropic 直连，它不认识的 effort
-- 词会返回空 completion 而不是干净的 400」。翻遍整个仓库（src/ docs/ scripts/
-- migrations/ prompts/ 和测试）没有任何一次真实探测记录 —— 它是一条从未被验证的推断，
-- 而且两个仓库里的两条注释互相引用、谁也没打过那一枪。
--
-- 所以不写死成"信"或"不信"，做成每条线路可配的开关：默认 false = 保持今天的行为
-- （升级不改变任何现有流量），管理员在后台对某一条线路打开、试完就知道上游认不认，
-- 不认就关回去，不用改代码重新发版。
ALTER TABLE models
  ADD COLUMN IF NOT EXISTS effort_passthrough BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN models.effort_passthrough IS
  '开启后，客户端拨的 reasoning_effort（含 xhigh/max）原样发给上游；关闭时封顶在 high。';
