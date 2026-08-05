//! The purchase receipt email.
//!
//! Why this is hand-inlined instead of Tailwind classes: email clients are not browsers.
//! Gmail strips `<style>` blocks containing class selectors, Outlook renders through Word,
//! and none of them fetch an external stylesheet. A Tailwind-classed email arrives as
//! unstyled text. So the *design language* is the console's — the same zinc ramp, the same
//! spacing steps, the same 12px radius — but every rule is written inline, and layout is
//! tables rather than flex/grid, because Outlook supports neither.
//!
//! Nothing here is decorative-only: the figures a buyer checks a receipt for are what they
//! paid, what they got, and how long it lasts. Those are the three biggest things on it.

/// Console's zinc-950 / zinc-500 / zinc-200 — the same values the dashboard uses.
const INK: &str = "#09090b";
const MUTED: &str = "#71717a";
const LINE: &str = "#e4e4e7";
const CANVAS: &str = "#fafafa";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub struct Receipt<'a> {
    pub product: &'a str,
    /// Already formatted with its symbol, e.g. "¥5.00" — the currency actually charged.
    pub amount: &'a str,
    /// What the account received, e.g. "$10.00 额度".
    pub granted: &'a str,
    /// e.g. "30 天" — None for a credits top-up, which does not expire.
    pub duration: Option<&'a str>,
    pub order_id: &'a str,
    pub console_url: &'a str,
}

/// A row in the details table. `strong` is for the figures worth scanning to.
fn row(label: &str, value: &str, strong: bool) -> String {
    let weight = if strong { "600" } else { "400" };
    format!(
        r#"<tr>
  <td style="padding:12px 0;border-bottom:1px solid {LINE};color:{MUTED};font-size:14px;">{}</td>
  <td style="padding:12px 0;border-bottom:1px solid {LINE};color:{INK};font-size:14px;font-weight:{weight};text-align:right;">{}</td>
</tr>"#,
        esc(label),
        esc(value)
    )
}

pub fn purchase_html(r: &Receipt) -> String {
    let duration_row = r
        .duration
        .map(|d| row("有效期", d, false))
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html lang="zh"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>购买成功</title></head>
<body style="margin:0;padding:0;background:{CANVAS};">
<!-- Preheader: what the inbox list shows next to the subject. Hidden in the body. -->
<div style="display:none;max-height:0;overflow:hidden;opacity:0;">已开通 {product_pre} · {amount_pre}</div>

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background:{CANVAS};padding:32px 16px;">
<tr><td align="center">

<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:480px;background:#ffffff;border:1px solid {LINE};border-radius:12px;">

  <tr><td style="padding:32px 32px 0 32px;">
    <div style="font-size:15px;font-weight:600;color:{INK};letter-spacing:-0.01em;">Mr.day One</div>
  </td></tr>

  <tr><td style="padding:24px 32px 0 32px;">
    <div style="font-size:22px;font-weight:600;color:{INK};letter-spacing:-0.02em;">购买成功</div>
    <div style="margin-top:8px;font-size:14px;line-height:1.6;color:{MUTED};">
      额度已经发放到你的账号，可以直接开始使用。
    </div>
  </td></tr>

  <tr><td style="padding:24px 32px 0 32px;">
    <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
      {rows}
    </table>
  </td></tr>

  <tr><td style="padding:24px 32px 0 32px;">
    <!-- Bulletproof-ish button: a padded anchor, since Outlook ignores button elements. -->
    <a href="{console}" style="display:inline-block;background:{INK};color:#ffffff;text-decoration:none;font-size:14px;font-weight:500;padding:12px 20px;border-radius:8px;">
      查看我的账号
    </a>
  </td></tr>

  <tr><td style="padding:24px 32px 32px 32px;">
    <div style="border-top:1px solid {LINE};padding-top:16px;font-size:12px;line-height:1.7;color:{MUTED};">
      付款由 <strong style="color:{INK};font-weight:600;">Stripe</strong> 处理，银行卡信息不会经过 Mr.day One 的服务器。<br>
      如需发票或有任何疑问，直接回复这封邮件即可。
    </div>
  </td></tr>

</table>

<div style="max-width:480px;margin-top:16px;font-size:12px;color:{MUTED};text-align:center;">
  订单号 {order}
</div>

</td></tr>
</table>
</body></html>"#,
        product_pre = esc(r.product),
        amount_pre = esc(r.amount),
        rows = format!(
            "{}{}{}",
            row("套餐", r.product, true),
            row("实付", r.amount, true),
            format!("{}{}", row("获得", r.granted, true), duration_row)
        ),
        console = esc(r.console_url),
        order = esc(r.order_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        purchase_html(&Receipt {
            product: "测试套餐",
            amount: "¥5.00",
            granted: "$10.00 额度",
            duration: Some("30 天"),
            order_id: "ord_123",
            console_url: "https://code.mrday.one/billing",
        })
    }

    #[test]
    fn every_style_is_inline() {
        let html = sample();
        // A <style> block or a class attribute means some client will drop the styling.
        assert!(!html.contains("<style"), "email must not rely on a style block");
        assert!(!html.contains("class="), "email must not rely on class selectors");
    }

    #[test]
    fn the_three_figures_a_buyer_checks_are_present() {
        let html = sample();
        for want in ["测试套餐", "¥5.00", "$10.00 额度", "30 天", "ord_123"] {
            assert!(html.contains(want), "receipt is missing {want}");
        }
    }

    #[test]
    fn injected_markup_cannot_escape_a_field() {
        // Product labels are operator-editable rows, so they reach this template as data.
        let html = purchase_html(&Receipt {
            product: "<script>alert(1)</script>",
            amount: "¥1",
            granted: "x",
            duration: None,
            order_id: "o",
            console_url: "https://code.mrday.one/billing",
        });
        assert!(!html.contains("<script>"), "product label was not escaped");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn a_credits_top_up_has_no_expiry_row() {
        let html = purchase_html(&Receipt {
            product: "Package A",
            amount: "¥30",
            granted: "$4.50 额度",
            duration: None,
            order_id: "o",
            console_url: "https://code.mrday.one/billing",
        });
        assert!(!html.contains("有效期"), "credits never expire — do not imply they do");
    }
}
