//! 1:1 UI 还原的**测量**层。
//!
//! # 为什么必须先有测量
//!
//! 在这之前，"还原设计稿"这件事的闭环是：`visual_compare` 把目标图和实现截图**并排拼成
//! 一张图**交给模型，提示词里写着「逐项比对、改到像素级吻合」「别停在差不多」。
//! 但整条链路上**没有任何一个数字**。于是"够不够像"完全由模型自己主观判断，而模型判断
//! 自己的作品像不像，天然偏向"像"。那句"别停在差不多"没有任何东西能兑现它。
//!
//! 这个模块提供缺失的那一半：一个**可以被验证的数**，以及"哪里不对"的坐标。有了它，
//! 还原才从"看着改"变成"测着改"——循环的退出条件不再是模型说行了，而是相似度到了阈值
//! 或者连续几轮不再提升。
//!
//! # 关于 1:1 的诚实说明
//!
//! - **网页**：能做到接近 1:1，因为那不是猜——DOM、计算样式、字体、盒模型都能直接读出来。
//! - **原生应用**：macOS 的辅助功能树给出真实的元素角色、文本和坐标框，同样是提取不是猜。
//! - **只有一张图**：严格的 1:1 做不到。图里恢复不出确切字体名、压缩前的原始色值、
//!   隐藏状态和交互行为。能做到的是**可测量的收敛**，并把真实数字报出来。
//!   这个模块的存在就是为了不让"差不多"冒充"1:1"。
//!
//! # 相似度怎么算
//!
//! 不用朴素的 RGB 欧氏距离——它对人眼不敏感的通道给了过高权重（蓝色差 10 和亮度差 10
//! 在视觉上完全不是一回事），会让"颜色几乎对了但整体偏暗"这种明显问题得到很高的分数。
//! 这里按亮度加权：亮度差算 60%，色度差算 40%，和人眼的敏感度大致对齐。
//!
//! 只报一个总分是不够用的：模型拿到 "87%" 之后不知道该改哪里。所以同时把画面切成网格，
//! 逐格算分并把最差的几格连坐标一起交出去——修正因此是定向的，而不是再猜一遍。

use image::RgbaImage;
use serde::Serialize;

/// 网格划分。16×16 = 256 格：足够定位到"左上角那个按钮"，又不会让返回值淹没上下文。
const GRID: u32 = 16;
/// 最多报几格问题区域。全报回去是噪声，模型一轮也改不完那么多。
const MAX_REGIONS: usize = 12;

#[derive(Serialize, Debug, Clone)]
pub struct DiffRegion {
    /// 网格坐标（列、行），从 0 开始。
    pub col: u32,
    pub row: u32,
    /// 这一格在**目标图**上的像素范围，模型可以直接拿去裁剪细看。
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// 这一格的相似度（0-100）。
    pub similarity: f32,
    /// 这一格**主要**错在哪一类。不是分类器，是三个便宜的启发式，用来把模型的注意力
    /// 引到正确的方向——"整块偏色"和"元素错位"要改的东西完全不同。
    pub kind: &'static str,
    /// 目标区域的平均颜色（十六进制），给"偏色"那一类用。
    pub target_color: String,
    /// 候选区域的平均颜色。
    pub candidate_color: String,
}

#[derive(Serialize, Debug)]
pub struct UiDiff {
    /// 总体相似度 0-100。
    pub similarity: f32,
    /// 完全一致的像素占比（容差内），给"还差多少"一个更直观的数。
    pub exact_ratio: f32,
    /// 比较时用的尺寸（候选图会被缩放到目标图的尺寸）。
    pub width: u32,
    pub height: u32,
    /// 候选图原始尺寸——和目标不一致本身就是要报告的事实，不能悄悄缩放了事。
    pub candidate_width: u32,
    pub candidate_height: u32,
    /// 最差的若干格，按相似度升序（最差的在前）。
    pub worst_regions: Vec<DiffRegion>,
    /// 一句给模型看的结论。
    pub verdict: String,
}

fn decode(bytes: &[u8], label: &str) -> Result<RgbaImage, String> {
    image::load_from_memory(bytes)
        .map_err(|e| format!("{label}解码失败：{e}"))
        .map(|img| img.to_rgba8())
}

/// data URL 或裸 base64 → 字节。
fn from_data_url(s: &str, label: &str) -> Result<Vec<u8>, String> {
    let payload = match s.find(";base64,") {
        Some(i) => &s[i + 8..],
        None => s,
    };
    b64_decode(payload.trim()).ok_or_else(|| format!("{label}不是合法的 base64 图片数据"))
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut n = 0;
    for &b in &bytes {
        if b == b'=' {
            break;
        }
        acc = (acc << 6) | val(b)?;
        n += 1;
        if n == 4 {
            out.push((acc >> 16) as u8);
            out.push((acc >> 8) as u8);
            out.push(acc as u8);
            acc = 0;
            n = 0;
        }
    }
    match n {
        3 => {
            out.push((acc >> 10) as u8);
            out.push((acc >> 2) as u8);
        }
        2 => out.push((acc >> 4) as u8),
        0 => {}
        _ => return None,
    }
    Some(out)
}

/// 单个像素的感知距离，0.0（完全相同）到 1.0。
///
/// 按亮度加权而不是朴素 RGB 欧氏距离：蓝通道差 10 和亮度差 10 在视觉上完全不是一回事。
/// 用朴素距离的话，"配色几乎对了但整体偏暗"会拿到很高的分——而那恰恰是最刺眼的一类错误。
fn pixel_distance(a: [u8; 4], b: [u8; 4]) -> f32 {
    // 透明度先合成到白底：截图里的半透明区域直接比 RGBA 会把"看起来一样"判成不一样。
    let blend = |p: [u8; 4]| {
        let alpha = f32::from(p[3]) / 255.0;
        [
            f32::from(p[0]) * alpha + 255.0 * (1.0 - alpha),
            f32::from(p[1]) * alpha + 255.0 * (1.0 - alpha),
            f32::from(p[2]) * alpha + 255.0 * (1.0 - alpha),
        ]
    };
    let (a, b) = (blend(a), blend(b));
    let luma = |p: [f32; 3]| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
    let dl = (luma(a) - luma(b)).abs() / 255.0;
    let dc = ((a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()) / (3.0 * 255.0);
    (dl * 0.6 + dc * 0.4).min(1.0)
}

fn mean_color(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> [f32; 3] {
    let mut sum = [0f32; 3];
    let mut n = 0f32;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y).0;
            let alpha = f32::from(p[3]) / 255.0;
            sum[0] += f32::from(p[0]) * alpha + 255.0 * (1.0 - alpha);
            sum[1] += f32::from(p[1]) * alpha + 255.0 * (1.0 - alpha);
            sum[2] += f32::from(p[2]) * alpha + 255.0 * (1.0 - alpha);
            n += 1.0;
        }
    }
    if n == 0.0 {
        return [0.0; 3];
    }
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn hex(c: [f32; 3]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        c[0].round().clamp(0.0, 255.0) as u8,
        c[1].round().clamp(0.0, 255.0) as u8,
        c[2].round().clamp(0.0, 255.0) as u8
    )
}

/// 区域内的边缘密度：相邻像素亮度突变的比例。用来区分"整块偏色"和"元素错位/缺失"。
fn edge_density(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> f32 {
    let luma = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        let a = f32::from(p[3]) / 255.0;
        0.2126 * (f32::from(p[0]) * a + 255.0 * (1.0 - a))
            + 0.7152 * (f32::from(p[1]) * a + 255.0 * (1.0 - a))
            + 0.0722 * (f32::from(p[2]) * a + 255.0 * (1.0 - a))
    };
    let mut edges = 0f32;
    let mut n = 0f32;
    for y in y0..y1 {
        for x in x0..x1.saturating_sub(1) {
            if (luma(x, y) - luma(x + 1, y)).abs() > 24.0 {
                edges += 1.0;
            }
            n += 1.0;
        }
    }
    if n == 0.0 {
        0.0
    } else {
        edges / n
    }
}

/// 比较两张图，给出相似度和最差的那些区域。
///
/// `candidate` 会被缩放到 `target` 的尺寸再比——尺寸不同本身也会如实报告出来，
/// 因为"我渲染的画布就比设计稿小"是个必须先解决的问题，不能靠缩放掩盖过去。
pub fn diff_images(target_bytes: &[u8], candidate_bytes: &[u8]) -> Result<UiDiff, String> {
    let target = decode(target_bytes, "目标图")?;
    let candidate_raw = decode(candidate_bytes, "候选图")?;
    let (w, h) = target.dimensions();
    let (cw, ch) = candidate_raw.dimensions();
    if w == 0 || h == 0 {
        return Err("目标图尺寸为 0".into());
    }
    // 上限：一张 4K 截图逐像素比要 800 万次，没必要——降采样到长边 1200 已经足够定位
    // 到单个组件，而且让整个循环快到可以每轮都跑。
    let scale = (1200.0 / w.max(h) as f32).min(1.0);
    let (sw, sh) = (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    );
    let target = image::imageops::resize(&target, sw, sh, image::imageops::FilterType::Triangle);
    let candidate =
        image::imageops::resize(&candidate_raw, sw, sh, image::imageops::FilterType::Triangle);

    let mut total = 0f64;
    let mut exact = 0u64;
    let mut cell_sum = vec![0f64; (GRID * GRID) as usize];
    let mut cell_n = vec![0f64; (GRID * GRID) as usize];
    for y in 0..sh {
        for x in 0..sw {
            let d = pixel_distance(target.get_pixel(x, y).0, candidate.get_pixel(x, y).0);
            total += f64::from(d);
            if d < 0.02 {
                exact += 1;
            }
            let col = (x * GRID / sw).min(GRID - 1);
            let row = (y * GRID / sh).min(GRID - 1);
            let idx = (row * GRID + col) as usize;
            cell_sum[idx] += f64::from(d);
            cell_n[idx] += 1.0;
        }
    }
    let px = f64::from(sw) * f64::from(sh);
    let similarity = ((1.0 - total / px) * 100.0).clamp(0.0, 100.0) as f32;
    let exact_ratio = ((exact as f64 / px) * 100.0) as f32;

    let mut regions: Vec<DiffRegion> = Vec::new();
    for row in 0..GRID {
        for col in 0..GRID {
            let idx = (row * GRID + col) as usize;
            if cell_n[idx] == 0.0 {
                continue;
            }
            let cell_sim = ((1.0 - cell_sum[idx] / cell_n[idx]) * 100.0).clamp(0.0, 100.0) as f32;
            // 98 以上的格子不值得报——报回去只会占着模型的注意力。
            if cell_sim >= 98.0 {
                continue;
            }
            let (x0, y0) = (col * sw / GRID, row * sh / GRID);
            let (x1, y1) = (
                ((col + 1) * sw / GRID).min(sw).max(x0 + 1),
                ((row + 1) * sh / GRID).min(sh).max(y0 + 1),
            );
            let tc = mean_color(&target, x0, y0, x1, y1);
            let cc = mean_color(&candidate, x0, y0, x1, y1);
            let color_delta =
                ((tc[0] - cc[0]).abs() + (tc[1] - cc[1]).abs() + (tc[2] - cc[2]).abs()) / 3.0;
            let te = edge_density(&target, x0, y0, x1, y1);
            let ce = edge_density(&candidate, x0, y0, x1, y1);
            // 三个便宜的启发式，只为把注意力引对方向，不是分类器：
            //  · 目标这里有大量边缘、候选几乎没有 → 该有的东西没画出来
            //  · 反过来 → 多画了不该有的东西
            //  · 边缘密度相近但平均色差大 → 结构对了，颜色/明暗不对
            //  · 都不满足 → 位置/尺寸对不上
            let kind = if te > 0.05 && ce < te * 0.35 {
                "missing"
            } else if ce > 0.05 && te < ce * 0.35 {
                "extra"
            } else if color_delta > 18.0 {
                "color"
            } else {
                "layout"
            };
            regions.push(DiffRegion {
                col,
                row,
                // 报回**原始目标图**的坐标，不是降采样后的——模型要拿这个去裁剪原图细看。
                x: (f64::from(x0) / f64::from(sw) * f64::from(w)).round() as u32,
                y: (f64::from(y0) / f64::from(sh) * f64::from(h)).round() as u32,
                w: (f64::from(x1 - x0) / f64::from(sw) * f64::from(w)).round().max(1.0) as u32,
                h: (f64::from(y1 - y0) / f64::from(sh) * f64::from(h)).round().max(1.0) as u32,
                similarity: cell_sim,
                kind,
                target_color: hex(tc),
                candidate_color: hex(cc),
            });
        }
    }
    regions.sort_by(|a, b| a.similarity.partial_cmp(&b.similarity).unwrap_or(std::cmp::Ordering::Equal));
    let total_bad = regions.len();
    regions.truncate(MAX_REGIONS);

    let size_note = if (cw, ch) != (w, h) {
        format!(
            "⚠️ 尺寸不一致：目标 {w}×{h}，你的实现 {cw}×{ch}。已缩放后比较，但**先把画布尺寸对上**——尺寸不同的情况下这个分数会系统性偏低，改颜色改间距都救不回来。"
        )
    } else {
        String::new()
    };
    let verdict = format!(
        "相似度 {similarity:.1}%（完全一致像素 {exact_ratio:.1}%）。{}共 {total_bad} 个网格区域低于 98%，下面列出最差的 {}。{}",
        if similarity >= 98.0 {
            "已经非常接近。"
        } else if similarity >= 92.0 {
            "整体对了，剩下的是细节。"
        } else if similarity >= 75.0 {
            "大结构对得上，但有明显差异。"
        } else {
            "差得还很远——先对结构和布局，不要先抠颜色。"
        },
        regions.len(),
        size_note
    );

    Ok(UiDiff {
        similarity,
        exact_ratio,
        width: w,
        height: h,
        candidate_width: cw,
        candidate_height: ch,
        worst_regions: regions,
        verdict,
    })
}

/// 比较两张图。两个参数都接受 `data:image/...;base64,...` 或裸 base64。
#[tauri::command(async)]
pub fn ui_diff(target: String, candidate: String) -> Result<UiDiff, String> {
    let t = from_data_url(&target, "目标图")?;
    let c = from_data_url(&candidate, "候选图")?;
    diff_images(&t, &c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            *p = image::Rgba([rgb[0], rgb[1], rgb[2], 255]);
        }
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(&img, w, h, image::ExtendedColorType::Rgba8)
            .unwrap();
        out
    }

    fn half_split(w: u32, h: u32, left: [u8; 3], right: [u8; 3]) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for (x, _y, p) in img.enumerate_pixels_mut() {
            let c = if x < w / 2 { left } else { right };
            *p = image::Rgba([c[0], c[1], c[2], 255]);
        }
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(&img, w, h, image::ExtendedColorType::Rgba8)
            .unwrap();
        out
    }

    use image::ImageEncoder;

    #[test]
    fn identical_images_score_100() {
        let a = solid(200, 200, [40, 90, 200]);
        let d = diff_images(&a, &a).unwrap();
        assert!(d.similarity > 99.9, "相同的图应当接近 100：{}", d.similarity);
        assert!(d.worst_regions.is_empty(), "相同的图不该报出问题区域");
    }

    #[test]
    fn a_slightly_off_colour_is_not_reported_as_a_match() {
        // 这是整个模块存在的理由：肉眼"差不多"的两张图必须拿到一个**低于 100** 的数，
        // 否则循环没有继续下去的依据，模型会停在"差不多"。
        let a = solid(200, 200, [255, 255, 255]);
        let b = solid(200, 200, [235, 235, 235]);
        let d = diff_images(&a, &b).unwrap();
        assert!(d.similarity < 99.0, "8% 的明度差被判成一致了：{}", d.similarity);
        assert!(d.similarity > 85.0, "8% 的明度差不该被判成面目全非：{}", d.similarity);
        assert!(!d.worst_regions.is_empty());
        assert_eq!(d.worst_regions[0].kind, "color", "整块偏色要被归到 color");
        assert_eq!(d.worst_regions[0].target_color, "#ffffff");
    }

    #[test]
    fn a_wrong_half_is_localised_to_that_half_not_smeared_over_the_whole_image() {
        // 只报一个总分是不够用的：模型拿到 "87%" 不知道该改哪儿。
        let a = half_split(320, 320, [255, 255, 255], [255, 255, 255]);
        let b = half_split(320, 320, [255, 255, 255], [10, 10, 10]);
        let d = diff_images(&a, &b).unwrap();
        assert!(d.similarity < 60.0, "半张图全黑，分数不该还很高：{}", d.similarity);
        assert!(!d.worst_regions.is_empty());
        // 最差的那些格子必须全在右半边——定位错了，模型就会去改没问题的那一半
        for r in &d.worst_regions {
            assert!(r.col >= GRID / 2, "问题定位到了左半边（col={}），而错的是右半边", r.col);
        }
        // 坐标要按**原始尺寸**给，模型拿它去裁原图
        assert!(d.worst_regions.iter().all(|r| r.x < 320 && r.y < 320));
    }

    #[test]
    fn size_mismatch_is_reported_rather_than_silently_scaled_away() {
        let a = solid(400, 300, [200, 200, 200]);
        let b = solid(200, 150, [200, 200, 200]);
        let d = diff_images(&a, &b).unwrap();
        assert_eq!((d.width, d.height), (400, 300));
        assert_eq!((d.candidate_width, d.candidate_height), (200, 150));
        assert!(d.verdict.contains("尺寸不一致"), "缩放掉了却不说：{}", d.verdict);
        assert!(d.verdict.contains("先把画布尺寸对上"), "没给出下一步");
    }

    #[test]
    fn missing_content_and_extra_content_are_told_apart() {
        // 目标有内容（大量边缘）、候选是纯色 → missing；反过来 → extra。
        // 两者要改的东西完全不同，混作一谈会把模型引到反方向。
        let busy = {
            let (w, h) = (160u32, 160u32);
            let mut img = RgbaImage::new(w, h);
            for (x, y, p) in img.enumerate_pixels_mut() {
                let on = ((x / 4) + (y / 4)) % 2 == 0;
                let v = if on { 255 } else { 0 };
                *p = image::Rgba([v, v, v, 255]);
            }
            let mut out = Vec::new();
            image::codecs::png::PngEncoder::new(&mut out)
                .write_image(&img, w, h, image::ExtendedColorType::Rgba8)
                .unwrap();
            out
        };
        let flat = solid(160, 160, [128, 128, 128]);

        let missing = diff_images(&busy, &flat).unwrap();
        assert!(missing.worst_regions.iter().any(|r| r.kind == "missing"),
            "目标有内容、实现是空白，应当报 missing：{:?}",
            missing.worst_regions.iter().map(|r| r.kind).collect::<Vec<_>>());

        let extra = diff_images(&flat, &busy).unwrap();
        assert!(extra.worst_regions.iter().any(|r| r.kind == "extra"),
            "实现多画了东西，应当报 extra：{:?}",
            extra.worst_regions.iter().map(|r| r.kind).collect::<Vec<_>>());
    }

    #[test]
    fn base64_and_data_url_are_both_accepted() {
        let png = solid(32, 32, [1, 2, 3]);
        let raw = crate::capture::b64(&png);
        assert!(from_data_url(&raw, "t").is_ok(), "裸 base64 应当能收");
        let url = format!("data:image/png;base64,{raw}");
        assert_eq!(from_data_url(&url, "t").unwrap(), png);
        assert!(from_data_url("not base64 @@@", "t").is_err());
    }
}
