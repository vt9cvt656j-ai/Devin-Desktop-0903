# Michael Design Library — enterprise-standard

Enterprise-grade delivery standard (企业大厂标准): every website ships at the polish level of a funded product company — Stripe/Linear/Vercel/Airbnb tier — never a personal side-project look (个人小作品感 = 不合格). These sections define what "enterprise" means concretely so implementation can be audited against them.

## Enterprise Design System Baseline — tokens, scales, states before any page [sections/enterprise-design-system]

Enterprise visual style starts from a real design system, not per-element styling (先建体系再写页面):

- **Spacing scale**: one 4px-based scale (4/8/12/16/24/32/48/64/96) used everywhere — section padding `py-16 md:py-24 lg:py-32`, card padding `p-6`/`p-8`, consistent `gap-6`/`gap-8` in grids. Random one-off margins are an amateur tell.
- **Type scale**: display/h1 `text-4xl md:text-5xl lg:text-6xl tracking-tight`, h2 `text-3xl md:text-4xl`, h3 `text-xl md:text-2xl`, body `text-base leading-relaxed`, caption `text-sm text-muted-foreground`. Max 2 font families (display + body pairing), 3–4 weights total.
- **Color tokens**: full semantic set mapped to CSS variables and Tailwind theme — `--background/--foreground/--card/--card-foreground/--muted/--muted-foreground/--primary/--primary-foreground/--secondary/--accent/--border/--ring/--radius`. Business components consume ONLY semantic classes (`bg-background`, `text-muted-foreground`, `bg-primary`); raw hex in JSX is a defect. **Palette values must be adopted from the michael-design knowledge-base hits for this task — record the source section for each token; do not invent colors from model memory (配色必须采用知识库命中蓝本的具体色值并记录来源).**
- **Radius & elevation**: one radius token (`--radius`, typically 8–16px) reused across buttons/cards/inputs; a 3-step shadow scale (subtle/card/overlay). Mixed radii and random shadows read as personal projects.
- **Interactive states**: every control defines hover, active, focus-visible (`ring-2 ring-ring ring-offset-2`), disabled (`opacity-50 pointer-events-none`) — enterprises never ship hover-only.
- **Component variants**: Button default/secondary/outline/ghost/destructive via cva/variants; one `<Button>` consumed everywhere — five hand-styled `<div>` buttons on one page is the #1 amateur tell.

## Enterprise Block Repertoire — derive structure from the business, never a fixed template [sections/enterprise-block-repertoire]

Structure is DERIVED from the business category and user journey each time — there is no fixed section order (结构按业务推导，禁止固定模板). Composing the same nav→hero→features→steps→subscribe skeleton for every site is a template smell; two sites in different categories must not share the same outline.

- **Derivation first**: list the category's real user journey (discover → evaluate → trust → act), then choose 8–12 blocks that serve THIS journey. A lodge site needs rooms/availability/local guide; a SaaS needs integrations/workflow/pricing; a portfolio needs projects/process/credits — never translate back into generic Hero/Features/Pricing.
- **Block menu to compose from** (pick and reorder freely): announcement bar, sticky translucent nav, hero (full-bleed/split/carousel), social-proof logo wall, alternating value splits, bento feature grid, product showcase tabs, media rail, metrics band, story/timeline, team/hosts, testimonials, case studies, comparison table, pricing tiers, FAQ accordion, local guide/map, blog/resources teaser, final CTA band, multi-column footer.
- **Variety rule**: adjacent blocks must not repeat the same composition (two same-width card grids in a row = defect); at least 4 distinct composition patterns per page from the layout repertoire.
- **Every block earns its place**: each block must carry real category content; a block kept only to fill the outline gets cut. Depth beats template completeness — 9 rich, differentiated blocks beat 12 thin ones.
- **Anchor blocks stay flexible**: nav and a complete multi-column footer are the only near-constants; everything between follows the journey, and even hero style (full-bleed vs split vs carousel) must fit the category media.

## Trust & Credibility Signals — what makes a site feel like a real company [sections/trust-credibility]

Enterprise sites earn trust visually (信任感设计); missing these makes any design feel personal regardless of styling:

- Named humans: testimonials/team with real avatar photos, full names, roles, companies — never initials-in-circles.
- Concrete numbers over adjectives: "4.8/5 · 2,300+ 住客评价", "12 个城市 · 240+ 房源", "98% 准时入住" — placed in hero proof line, metrics band, and section subheads.
- Institutional markers: partner/press logo wall, certifications, payment method icons, security notes near forms ("SSL 加密 · 免费取消").
- Complete legal layer: privacy, terms, contact with real address/email in footer; cookie/consent hint where relevant.
- Professional copy tone: benefit-led, specific, no filler superlatives; consistent zh/en, no lorem ipsum, no "欢迎来到我们的网站".
- Every form has validation states, success confirmation, and privacy microcopy under the submit button.

## Professional Polish Checklist — the last 10% that separates enterprise from personal [sections/professional-polish]

Audit every page against this list before delivery (交付前逐项检查):

- Accessibility: WCAG AA contrast (4.5:1 body, 3:1 large), `focus-visible` rings on all interactive elements, alt text on every image, semantic landmarks (`header/nav/main/section/footer`), `aria-label` on icon-only buttons.
- Responsive integrity: audit at 375/768/1024/1440; no horizontal scroll, no orphan grid cards, tap targets ≥44px, typography reflows (no 3-word-per-line headlines on mobile).
- Loading & states: image `loading="lazy"` + `aspect-ratio` placeholders (no layout shift), skeletons for async content, designed empty/error states, `onError` fallback images (never `display:none` holes).
- Micro-consistency: one icon family/stroke, one easing curve, one duration scale, aligned baselines, equal card heights within a row (`h-full flex flex-col`), no double borders (border+shadow stacking).
- Meta completeness: `<title>` + description, OG tags, favicon, `lang` attribute, `prefers-reduced-motion` honored globally.
- Never ship: default violet/indigo accent on non-tech brands, Sparkles-as-AI icons, empty heroes, lorem ipsum, single-line footers, unstyled focus outlines removed without replacement.

## Curated Palette Library — ready-to-adopt enterprise token sets by category [sections/curated-palette-library]

Adopt one set (or a KB blueprint hit's palette) and map straight to semantic tokens; never improvise hues (成套采用，禁止临场编色). Format: background / foreground / primary / accent / muted. **Cite and apply colors by Tailwind family+step (stone-800, amber-700…), never by raw hex**: the hex values below are only the reference definition of each step — business code consumes semantic tokens (`bg-primary`/`bg-card`), token files may hold the hex with its Tailwind name noted, and any blueprint color that comes without a Tailwind name must first be snapped to the nearest Tailwind family+step (e.g. #17130d→stone-950) before use（配色词汇表=Tailwind 族+档 与语义 token 两层，叙述与业务代码禁止裸 hex）.

- **Nature lodge / 民宿 / travel stay**: `#FAF7F2` warm paper / `#292524` stone-800 / `#B45309` amber-700 / `#166534` green-800 / `#F5F0E8` — warm stone neutrals, amber CTA, forest support.
- **Cafe / coffee / bakery**: `#FFFBF5` cream / `#3E2723` espresso / `#92400E` amber-800 / `#C2410C` orange-700 / `#F7F0E5`.
- **SaaS / tech / AI / chat / messaging / collaboration / productivity tools**: light `#FFFFFF` / `#09090B` zinc-950 / `#2563EB` blue-600 or `#059669` emerald-600 / `#4F46E5` support / `#F4F4F5`; dark mode `#09090B` bg + `#FAFAFA` fg + same accent scale.
- **Finance / fintech**: `#F8FAFC` slate-50 / `#0F172A` slate-900 / `#1D4ED8` blue-700 / `#047857` emerald-700 / `#F1F5F9` — conservative, high contrast.
- **Health / clinic / wellness**: `#F7FAF9` / `#1C2B2A` / `#0D9488` teal-600 / `#65A30D` lime-600 support / `#EEF5F3`.
- **Luxury / jewelry / fashion**: `#0C0A09` near-black bg / `#FAFAF9` fg / `#CA8A04` gold / `#78716C` stone support / `#1C1917` surface.
- **Education / kids**: `#FFFDF7` / `#1E293B` / `#EA580C` orange-600 / `#0891B2` cyan-600 / `#FEF3C7` tints.
- **Real estate / architecture**: `#FAFAF9` stone-50 / `#1C1917` stone-900 / `#0F766E` teal-700 or `#A16207` bronze / `#E7E5E4`.
- **Nonprofit / charity / animal rescue / 公益救助**: stone-50 `#FAFAF9` bg / stone-900 `#1C1917` fg / teal-600 `#0D9488` primary / rose-500 `#F43F5E` warmth accent / stone-100 `#F5F5F4` muted — light, trustworthy, warm without any yellow.
- **Pets / vet / 宠物服务**: white `#FFFFFF` bg / zinc-900 `#18181B` fg / sky-600 `#0284C7` primary / orange-400 `#FB923C` soft accent / zinc-100 `#F4F4F5` muted.
- Category not listed above → pick the CLOSEST set by business nature and say which one you took; inventing a new palette because "the category is missing" is a defect（品类没列出就取最接近的一套并说明，缺品类不是编色的许可）.
- Application rule: primary drives CTA/links/active/focus ring/icon tints; accent only for secondary highlights (badges, gradients, illustration fills); verify AA contrast for primary-on-background and foreground-on-primary before shipping. **Usage ratio: the palette's neutrals carry >=90% of page area; primary/accent stay in small high-value roles only — section backgrounds alternate white/lightest-neutral, never accent-tinted bands. When in doubt, LESS color: a near-monochrome page with one crisp CTA color outclasses a colorful one every time（宁可近黑白 + 一个利落的主色 CTA，不要满页彩色）.**

## Dark Theme Execution Standard — only when the user explicitly chose dark [sections/dark-theme-execution]

Never self-select dark; when the user did choose it, execute to this standard (仅用户明确选暗色时使用):

- Canvas near-black with subtle tint, never pure black: `zinc-950`-class `#010102`; text `zinc-50`-class `#f7f8f8`, never pure white.
- Depth = surface ladder (`#0f1011` → `#141516` → `#18191a`, i.e. ascending dark-neutral steps) + 1px hairline borders `#23252a` + top inner highlight `inset 0 1px 0 rgba(255,255,255,.06)`; heavy drop shadows are forbidden on dark.
- Exactly ONE accent family from the task's palette hit; decoration/icons/charts/mockups only use its opacity steps (.55/.30/.12) or lightness steps. No rainbow scatter, no framework-default indigo/violet.
- Semantic colors (success/error) only express real state, never decoration.
- Cards: radius 8-12px (16+ forbidden), padding on base-4 (24/32/48), display headings with tight negative tracking (48px → -2.4px); card surface must be LIGHTER than the canvas.
- Mockups must be real screenshots or pixel-accurate fake UI (realistic browser chrome: 44px bar + 12px low-saturation traffic lights + URL pill); colored blobs/dots/grey boxes as placeholders are forbidden — blobs only as background glow.
- Full-page SVG feTurbulence noise (baseFrequency .65, opacity .10) to cover banding; ambient glow = large absolute radial (800px + blur(100px)) behind content; direct outer-glow on content elements is forbidden; glass only on modal/dropdown/nav with blur 5px.

## Typography Pairings — display + body combinations by brand tone [sections/typography-pairings]

Two families max, loaded with `display=swap`, subset where possible (字体配对，非 Inter 单打天下):

- **Warm hospitality / lodge / cafe**: display "Fraunces" or "General Sans" (Fontshare) + body "Inter"; generous `leading-relaxed`, display `tracking-tight`.
- **Modern SaaS / tech**: display "Cal Sans"/"Space Grotesk" + body "Inter"; tabular figures for metrics (`font-variant-numeric: tabular-nums`).
- **Editorial / magazine / portfolio**: display "Playfair Display"/"Newsreader" italic accents + body "Source Serif 4" or "Inter"; drop caps and pull quotes where content warrants.
- **Luxury**: display "Cormorant Garamond"/"Marcellus" + body "Jost"/"Inter" with wide uppercase eyebrow labels (`tracking-[0.2em] uppercase text-xs`).
- **Chinese-first pages**: pair the Latin display with system CJK stack `"PingFang SC", "Noto Sans SC", "Microsoft YaHei"`; keep CJK body ≥15px, `leading-[1.8]`, avoid faux-bold on CJK; punctuation-aware line breaking (`text-wrap: balance` for headings).
- Hierarchy discipline: eyebrow (accent, uppercase, small) → headline (display, tight) → subcopy (muted, relaxed) — reuse this trio in every section for rhythm; numbers in metrics use the display family.
