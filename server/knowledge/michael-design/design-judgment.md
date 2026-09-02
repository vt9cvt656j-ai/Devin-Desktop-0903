# Michael Design Library — design-judgment

Cross-cutting design judgment rules distilled from real delivery feedback (真实交付反馈沉淀的设计判断规则). Apply these BEFORE and WHILE composing any blueprint: they override lazy defaults. Each section is a self-contained checklist the implementation must pass.

## Hero Media Discipline — full-bleed image / carousel / video, never an empty hero [sections/hero-media-discipline]

A hero that is only heading + subtitle + two buttons floating on a blank canvas is a delivery failure (空 hero 留白 = 不合格). The hero must carry real, category-matched media as its visual anchor. Choose ONE pattern:

- **Full-bleed background**: category-matched photo or looping video covering the hero (`absolute inset-0 object-cover`), with a gradient scrim for text contrast (`bg-gradient-to-b from-black/50 via-black/25 to-black/60` on dark imagery, adjust for light). Copy sits on top with white/high-contrast text.
- **Auto-playing carousel 轮播图**: 3–5 category images, crossfade 800ms every 5–7s, dot indicators, pause on hover, swipe + `snap-x` on mobile, autoplay disabled under `prefers-reduced-motion`.
- **Split hero**: copy left (45–55%), media right — large rounded image, image collage, or 2×2 bento of photos. Never leave the media half empty.
- **Layered motion 动效**: headline/subtitle/CTA fade-up 0.6–0.8s with 0.1–0.15s stagger, `cubic-bezier(0.16,1,0.3,1)`; background media gets slow Ken Burns (scale 1.0→1.06 over 12–20s) or scroll parallax (translateY 10–20%); carousel crossfades.
- **Responsive 响应式**: mobile keeps the media (50–70vh full-bleed or stacked below copy at reduced height) — never drop to a text-only hero on small screens; `prefers-reduced-motion` disables Ken Burns/parallax/autoplay, keeps ≤300ms opacity fades only.
- Media must match the business category: lodge/民宿 → cabin in forest at dusk; cafe → latte art / warm interior; fitness → training floor. Generic stock tech imagery on a nature brand is a mismatch.

## Card Count Adaptive Grid — count cards first, never orphan the last card [sections/card-count-grid]

Count the actual number of repeated cards FIRST, then derive the grid (先数卡片再定网格). A 4-card section rendered as `grid-cols-3` leaves one orphan card alone on the second row bottom-left — this exact bug is forbidden.

- **2 cards** → `grid-cols-1 sm:grid-cols-2`, or a split layout.
- **3 cards** → `grid-cols-1 md:grid-cols-3`; mobile stacks or horizontal snap rail.
- **4 cards** → **2×2**: `grid-cols-1 sm:grid-cols-2` (do NOT add `lg:grid-cols-3`). Only go single-row `lg:grid-cols-4` when the container is wide and cards are compact. Alternative: 1 featured card spanning 2 columns/rows + 3 standard (`lg:grid-cols-3` with `lg:col-span-2 lg:row-span-2` on the featured card).
- **5 cards** → 3+2 with the last row centered (wrap last two in a centered flex row, or `col-start` offsets), or 1 featured + 2×2.
- **6 cards** → `md:grid-cols-2 lg:grid-cols-3` (3×2).
- **7 cards** → featured + 3×2, or 4+3 with centered last row.
- **Dynamic count** → `grid-template-columns: repeat(auto-fit, minmax(280px, 1fr))` and still verify the last row visually balances; if it cannot, promote one card to featured.
- Mobile: single column or `overflow-x-auto snap-x snap-mandatory` media rail; tablet re-flows (2-col) before desktop.
- Hover: `hover:-translate-y-1 hover:shadow-lg transition` on every interactive card, consistent across the set.

## Semantic Icon Mapping — infer the concept, no generic star / sparkles filler [sections/semantic-icon-mapping]

Before rendering any icon, write the mapping: business concept → object/action/state → concrete icon name (业务概念 → 对象/动作/状态 → 具体图标). Reason about what the phrase MEANS, then pick the icon whose literal object matches:

- 精选房源 / curated listings = "each home is vetted & verified" → `BadgeCheck`, `ShieldCheck`, or `Home` + check. NOT `Star` (a star means rating, which sits elsewhere on the same page) and NOT `Sparkles`.
- 绝佳位置 / prime locations → `MapPin` or `Map`. 贴心服务 / responsive host service, 24h → `Headset`, `MessageCircle`, or `Clock`. 安全支付 / secure payment → `Lock`, `ShieldCheck`, `CreditCard`. 灵活取消 / flexible cancellation → `CalendarX`, `RotateCcw`. AI assistant → `Bot`, `Cpu`. Automation → `Workflow`.
- `Sparkles` / `Wand` / `Stars` are ONLY allowed when the meaning is literally shine/magic — never as an "AI 高级感" filler icon (万能 AI 图标 = 禁止).
- One icon family per page (Lucide preferred), same stroke weight, same optical size (20–24px). Icon container tint must reuse the BRAND accent scale (`bg-primary/10 text-primary`), never an unrelated hue.
- If the exact concept icon doesn't exist in the current set, search the icon library for the object noun first; only fall back to an adjacent concept, never to a decorative placeholder.

## Category Palette Harmony — derive the accent from the business, never default purple [sections/category-palette]

The accent color must be derived from the business category and its imagery, not from a framework default (品牌强调色由业务品类推导，禁止默认紫色/violet). The same applies to every template-habit hue — default blue/indigo, neon-on-black, blanket amber/yellow: any hue must be justified by the category's materials, imagery or brand, or taken from a palette-library set; a hue chosen from model habit instead of category evidence is a defect（一切凭惯性而非品类证据选出的色相都算缺陷）. A nature lodge brand with forest photography shipping `violet-600` buttons and lavender section tints is a visible mismatch.

- Category → palette starting points: nature lodge / cabin / 民宿 → warm amber `amber-600`/`orange-700` + deep forest `green-800`/`emerald-900` + warm neutrals (`stone`); cafe/coffee → espresso brown + cream + amber; spa/wellness → sage/eucalyptus greens + sand; fintech/SaaS → confident blue or emerald + cool `zinc`/`slate`; luxury/jewelry → near-black + gold; kids/education → saturated primaries with soft pastels; nonprofit / charity / animal rescue / 公益/救助 → `teal-600` or `emerald-600` primary + `rose-500` warmth accent on light `stone-50`/white — trustworthy and warm WITHOUT yellow; pets / vet / 宠物 → `sky-600` or `teal-600` + soft `orange-400` accent on white.
- **"Warm feeling" ≠ yellow (温暖 ≠ 黄色，amber 滥用 = 和默认紫一样的逃生色)**: amber/yellow belongs ONLY to real warm-material brands (wood cabins, coffee, bakery, autumn produce). Charity, animal welfare, community, health, education-for-good all read "warm" through light backgrounds, rounded shapes, rose/orange micro-accents and real photography — NOT through amber CTAs on dark brown. If the last obvious pick is amber, justify it from the category's physical materials or pick the non-yellow set.
- **Light canvas is the default (未指定时根背景必须浅色)**: root background = white or `stone-50`/`zinc-50`-class light neutral unless the user explicitly asked for dark; a full-bleed dark hero photo never licenses turning the entire page dark.
- Sample the dominant tones of the actual hero/section imagery and align the accent with them (wood, foliage, dusk light → warm accent).
- Palette contract (全页配色契约): exactly 1 neutral family (warm `stone` for warm brands, `zinc/slate` for cool) + 1 brand accent family + optional 1 support family. Map to tokens (`--primary`, `--accent`, `--background`, `--muted`...) and make EVERY colored element consume them: CTA buttons, links, numbered step circles, icon container tints, badges, section band backgrounds, focus rings.
- **Restraint ratio (用色克制律，高级感的第一定律)**: >=90% of the page AREA must be neutral (white / near-white grays / near-black text) — the accent family appears ONLY in a handful of small, high-value roles: primary CTA, links, active/selected states, focus rings, and at most one featured highlight per screen. Section backgrounds default to white / lightest-neutral alternation, NEVER tinted accent bands per section; cards sit on neutral surfaces with the accent reserved for one tint or one border-emphasis at most. Spraying the accent across section backgrounds, every icon, every badge and every card reads cheap immediately (满页刷强调色 = 廉价感的头号来源).
- **Monochrome is a complete design (黑白灰本身就是合格方案)**: a page built purely from white + one neutral gray ramp + near-black text, with hierarchy carried by type scale/weight, spacing and imagery, is a PREMIUM baseline (Apple/Linear-grade) — when unsure how much color to use, err toward none. Color is a scalpel for attention, not decoration; hierarchy comes from typography, whitespace and layout FIRST, color LAST.
- **Over-decoration is a defect (过度装饰=缺陷)**: tinted section band + colored border + shadow + gradient + icon circles stacked together reads as template junk. Each section gets at most ONE decorative device; the rest of the depth comes from spacing and type.
- Audit pass: scan every colored element (subscribe band background, step numbers, icon circles, hover states) — any hue not in the token list is a defect. One section must never introduce a foreign hue the rest of the page doesn't use. ALSO audit the ratio: if neutral coverage is below ~90% of page area, strip accent usages back to CTA/links/selected/focus until it is.
- Source of truth (配色以知识库为准): adopt the EXACT palette values found in this task's michael-design blueprint hits and record the source section per token; when no hit carries a usable palette, take a ready-made set from the Curated Palette Library section — never invent hues from model memory.
- **Category gate on palette adoption (跨品类命中只借结构不借配色)**: a blueprint hit from a DIFFERENT business category may donate layout, motion, component and density ideas — NEVER its palette. Palette comes only from a category-matched hit or the Curated Palette Library's closest set. Claiming "adopted from the blueprint hits" for a palette (e.g. "warm tones") that appears in none of this task's hits is fabrication and a defect（命中里根本没有的配色不许假托"命中蓝图"之名）.

## Section Rhythm & Scroll Motion — responsive animation defaults for every page [sections/section-scroll-motion]

Static sections that pop in with no choreography read as unfinished. Every full page ships a small motion system, not ad-hoc effects (整页动效系统，非零散特效):

- **Reusable SectionReveal**: IntersectionObserver / `whileInView` wrapper — fade-up 24px, 0.6–0.8s, `cubic-bezier(0.16,1,0.3,1)`, children stagger 0.08–0.12s, `once: true`. Apply to every major section's heading + content.
- **One signature moment** per page: hero parallax/Ken Burns, sticky storytelling section, horizontal scroll media rail, or count-up stats — pick the one that fits the content; do not stack them all.
- **Micro-interactions**: card hover `-translate-y-1` + shadow deepen; primary button hover scale 1.02–1.03 / active 0.97; images zoom `scale-105` inside `overflow-hidden` on hover; nav links underline slide.
- **Responsive motion 响应式动效**: mobile shortens distances (24px→12px), removes parallax/pin/scrub and heavy canvas, keeps taps ≥44px; `prefers-reduced-motion: reduce` swaps everything to opacity-only ≤300ms (`motion-reduce:` variants or `useReducedMotion`).
- Timing consistency: one easing curve + one duration scale (0.2s micro / 0.6–0.8s reveal / 12–20s ambient) across the page.
