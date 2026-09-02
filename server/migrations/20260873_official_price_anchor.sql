-- 目录价按「官方价」走，不跟着 OpenRouter 的活动降价往下掉。
--
-- `official_price()` 返回的一直是 OpenRouter **当天**挂牌的价，而那个价会因为活动、
-- 促销、某家承载商临时降价而下调。它是计费兜底价（每模型没单独配价时就用它），
-- 所以目录一降，我们对用户的收费当场跟着降 —— 而我们付给上游的钱没降。
--
-- 这里存每天观测到的价，`refresh` 之后取**窗口内最高**当作官方价。
-- 活动期的低价进不了账（窗口内还有活动前的高价压着），而真正的永久降价会在
-- 窗口走完之后自动生效 —— 不需要任何人记得去改。
--
-- 只存价，不存能力：窗口、档位这些跟着最新一次走才对，它们不会「搞活动」。
CREATE TABLE IF NOT EXISTS model_catalog_price_log (
    day           date              NOT NULL,
    norm_id       text              NOT NULL,
    input_price   double precision,
    output_price  double precision,
    PRIMARY KEY (day, norm_id)
);

-- 当天实际观测到的价。**和 input_price 分开存**：一个是我们计费用的官方价（锚定后的），
-- 一个是此刻目录上挂的。两者一分开，「现在在打折」这件事才在后台看得见 ——
-- 合成一个数的话，运营只会看到一个数字，看不出它是不是被活动价拉下来的。
ALTER TABLE model_catalog
    ADD COLUMN IF NOT EXISTS spot_input_price  double precision,
    ADD COLUMN IF NOT EXISTS spot_output_price double precision;

-- 窗口天数。可配置而不是写死：多长算「活动」是运营判断，不同厂商的促销周期差很远。
-- 0 = 关掉锚定，直接用当天价（也就是这次改动之前的行为）。
ALTER TABLE app_settings
    ADD COLUMN IF NOT EXISTS official_price_window_days integer
        CHECK (official_price_window_days IS NULL
               OR official_price_window_days BETWEEN 0 AND 365);
