//! 人类化输入运动学。把「瞬移点击 + 一次性灌入文本」换成**带轨迹的移动**和**有节奏的
//! 敲键**——让浏览器自动化看起来、也让被自动化的页面感觉起来像真人在操作，而不是一台机器
//! 在同一像素上瞬间完成一切。
//!
//! 这里全是**纯函数**：不碰 CDP、不碰时钟、不 sleep。上层拿到「一串路径点」和「一串毫秒
//! 延时」之后，自己去 dispatch 鼠标事件 + tokio sleep。这样做有两个好处：① 运动学本身能被
//! 单元测试钉死（轨迹是否落在终点、延时是否落在人类区间），不需要真起一个浏览器；② 主 App
//! （headless_chrome）和这个 sidecar（chromiumoxide）两套栈可以共用同一套曲线。
//!
//! 抖动用一个**可复现**的 LCG/SplitMix 产生——种子由调用方按坐标/文本派生。可复现是刻意的：
//! 测试要能断言边界，而真人感来自「曲线形状 + 区间随机」，不来自「不可复现」。

/// SplitMix64 —— 小而好的确定性伪随机。种子相同则输出相同。
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // 混一下种子，避免相邻种子（相邻坐标）产生肉眼可辨的相似序列。
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// [0, 1)
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// [lo, hi]
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// 一次移动分几步。距离越远步数越多（大约每 12px 一步），但夹在 [6, 48]：太少不流畅、
/// 太多白白灌一堆 CDP 事件拖慢速度。哪怕原地不动也走几步（微调落点）。
pub fn move_steps(dist: f64) -> usize {
    let n = (dist / 12.0).round() as i64;
    n.clamp(6, 48) as usize
}

/// cubic ease-in-out：起步慢、中段快、收尾慢——真人手臂移动的速度曲线。t∈[0,1]→[0,1]。
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let u = -2.0 * t + 2.0;
        1.0 - u * u * u / 2.0
    }
}

/// 从 `from` 到 `to` 的一条人手轨迹。返回**不含起点、含终点**的一串点：
///  · 主进度用 ease-in-out（不是匀速）；
///  · 叠一条垂直于移动方向的正弦弧（`sin(pi·t)`，两端为 0、中段最大）——手不走直线；
///  · 每步再加 ±0.6px 的微抖动——真人不会像素级完美；
///  · **末点精确拽回 `to`**，保证点击真的落在目标上。
pub fn ease_path(from: (f64, f64), to: (f64, f64), seed: u64) -> Vec<(f64, f64)> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = move_steps(dist);
    let mut rng = Rng::new(seed);

    // 弧幅：距离的一小截，带抖动，封顶 40px；短距离几乎是直线。方向随机左右。
    let arc = (dist * 0.08).min(40.0) * rng.range(0.4, 1.0) * if rng.coin() { 1.0 } else { -1.0 };
    // 垂直于移动方向的单位向量。
    let (nx, ny) = if dist > 1e-6 {
        (-dy / dist, dx / dist)
    } else {
        (0.0, 0.0)
    };

    let mut out = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let e = ease_in_out(t);
        let bx = from.0 + dx * e;
        let by = from.1 + dy * e;
        let a = arc * (std::f64::consts::PI * t).sin();
        let jx = rng.range(-0.6, 0.6);
        let jy = rng.range(-0.6, 0.6);
        out.push((bx + nx * a + jx, by + ny * a + jy));
    }
    if let Some(last) = out.last_mut() {
        *last = to;
    }
    out
}

/// 相邻两步移动之间的停顿（ms）。很短——凑成一次流畅滑动，而不是一格一格地跳。
pub fn move_step_delay_ms(seed: u64) -> u64 {
    Rng::new(seed).range(4.0, 12.0).round() as u64
}

/// 鼠标「按下」到「抬起」之间的按住时长（ms）。真人一次点击大约 55–120ms。
pub fn press_hold_ms(seed: u64) -> u64 {
    Rng::new(seed).range(55.0, 120.0).round() as u64
}

/// 一段文本每个字符的敲击间隔（ms），一一对应 `text.chars()`。真人 cadence：
///  · 基线 45–110ms；
///  · 词间/句间更久：**前一个**字符是空格或标点时，这一击停顿加长；
///  · 换行后停顿最久（像换了一行在想）；
///  · 偶发（~8%）一个「想一下」的长停顿；
/// 返回的每个值都夹在 [30, 900] 的人类可信区间内，绝不为 0（0 = 机器瞬打）。
pub fn keystroke_delays(text: &str, seed: u64) -> Vec<u64> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    let mut prev: Option<char> = None;
    for _ in text.chars() {
        let mut d = rng.range(45.0, 110.0);
        if let Some(p) = prev {
            if p == ' ' || ".,!?;:，。！？；：、".contains(p) {
                d += rng.range(60.0, 160.0);
            }
            if p == '\n' {
                d += rng.range(120.0, 280.0);
            }
        }
        if rng.unit() < 0.08 {
            d += rng.range(140.0, 380.0);
        }
        out.push((d.round() as u64).clamp(30, 900));
        prev = text.chars().nth(out.len() - 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_steps_scales_and_clamps() {
        assert_eq!(move_steps(0.0), 6, "原地也要走几步微调落点");
        assert_eq!(move_steps(5.0), 6, "极短距离夹到下限 6");
        assert!((6..=48).contains(&move_steps(120.0)));
        assert_eq!(move_steps(100_000.0), 48, "再远也封在 48，别灌一万个事件");
        // 单调不减
        assert!(move_steps(50.0) <= move_steps(500.0));
    }

    #[test]
    fn ease_path_starts_moving_and_lands_exactly_on_target() {
        let from = (100.0, 100.0);
        let to = (700.0, 420.0);
        let p = ease_path(from, to, 12345);
        assert_eq!(p.len(), move_steps(600.0f64.hypot(320.0)));
        // 末点必须精确落在目标上——点击靠的就是它。
        let last = *p.last().unwrap();
        assert!((last.0 - to.0).abs() < 1e-9 && (last.1 - to.1).abs() < 1e-9, "末点没落在目标: {last:?}");
        // 不含起点：第一个点已经离开了 from（ease-in 很慢，但至少动了）。
        let first = p[0];
        assert!(first != from, "第一个点不该等于起点");
        // 每个点都是有限数，没有 NaN/Inf。
        assert!(p.iter().all(|(x, y)| x.is_finite() && y.is_finite()));
    }

    #[test]
    fn ease_path_arcs_off_the_straight_line_but_stays_bounded() {
        let from = (0.0, 0.0);
        let to = (400.0, 0.0);
        let p = ease_path(from, to, 999);
        // 中段应当偏离直线（y != 0）——手不走直线。
        let mid = p[p.len() / 2];
        assert!(mid.1.abs() > 1.0, "中段没有弧度，成了直线: {mid:?}");
        // 但偏离有上界：弧幅封顶 40px + 抖动 0.6，给点余量。
        assert!(p.iter().all(|(_, y)| y.abs() < 45.0), "弧度失控");
        // 主轴（x）大体是推进的：末点 x 必须到达目标。
        assert!((p.last().unwrap().0 - 400.0).abs() < 1e-9);
    }

    #[test]
    fn ease_path_is_deterministic_for_a_seed() {
        let a = ease_path((3.0, 4.0), (200.0, 90.0), 42);
        let b = ease_path((3.0, 4.0), (200.0, 90.0), 42);
        assert_eq!(a, b, "同一种子必须复现——否则测试钉不住，回放也不稳");
        let c = ease_path((3.0, 4.0), (200.0, 90.0), 43);
        assert!(a != c, "不同种子应当给出不同轨迹");
    }

    #[test]
    fn keystroke_delays_are_human_and_never_instant() {
        let text = "hello, world!\nnext line 中文，标点。done";
        let d = keystroke_delays(text, 7);
        assert_eq!(d.len(), text.chars().count(), "每个字符一个延时");
        // 绝不为 0：0 就是机器瞬打，正是要消灭的。区间也不能荒唐。
        assert!(d.iter().all(|&x| (30..=900).contains(&x)), "延时越界: {d:?}");
        // 确定性。
        assert_eq!(keystroke_delays(text, 7), d);
        assert!(keystroke_delays(text, 8) != d, "不同种子应不同");
    }

    #[test]
    fn keystroke_delays_pause_longer_after_a_space_than_within_a_word() {
        // 词间停顿（空格之后那一击）平均应当比词内长。用大样本比均值，绕开单点抖动。
        let text: String = std::iter::repeat("ab cd ").take(400).collect();
        let d = keystroke_delays(&text, 3);
        let chars: Vec<char> = text.chars().collect();
        let mut after_space = vec![];
        let mut within = vec![];
        for i in 0..chars.len() {
            if i == 0 {
                continue;
            }
            if chars[i - 1] == ' ' {
                after_space.push(d[i] as f64);
            } else if chars[i - 1].is_alphabetic() {
                within.push(d[i] as f64);
            }
        }
        let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        assert!(
            avg(&after_space) > avg(&within) + 30.0,
            "空格后没有明显更长的停顿: after={:.1} within={:.1}",
            avg(&after_space),
            avg(&within)
        );
    }

    #[test]
    fn timing_helpers_stay_in_human_bounds() {
        for s in 0..500u64 {
            assert!((4..=12).contains(&move_step_delay_ms(s)));
            assert!((55..=120).contains(&press_hold_ms(s)));
        }
    }
}
