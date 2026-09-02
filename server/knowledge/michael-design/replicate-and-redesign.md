# Michael Design Library — replicate-and-redesign

Workflows for reference-site replication (仿站), existing-project redesign (重构/改版), knowledge-base evidence composition (证据合成调用) and real asset sourcing (真实素材来源). These make KB hits ACTIONABLE — palette, style, motion and media get combined into one delivered system instead of being "referenced".

## Reference Site Replication Workflow — 仿站: learn the design system, rebuild it legally [sections/reference-replication-workflow]

Replicating a reference site means extracting its design SYSTEM and rebuilding it with our stack — never copying its DOM, copy or brand (仿的是体系，不是复制):

- **① Extract evidence**: `learn_design`/`web_fetch` the reference URL; record exact palette hex (CSS variables/computed styles), font families + type scale, spacing rhythm, container widths/breakpoints, nav & hero pattern, every section's composition in order, card/button styles (radius/shadow/states), motion inventory (scroll reveals, hover behaviors, transitions). Screenshot desktop + mobile as the comparison baseline.
- **② Translate to our stack**: map extracted values into semantic tokens (`--primary/--background/--radius`…) and the Tailwind theme; substitute commercial fonts with Google/Fontshare equivalents of the same voice; rebuild interactions as shadcn/ui primitives with `cva` variants; reproduce composition patterns (split/bento/rail) rather than pixel-for-pixel DOM cloning.
- **③ Non-copy boundaries**: never reuse the reference's copywriting, logo, brand name, proprietary photos/illustrations, fonts you can't license, or its API/DOM/source. All content is rewritten for the actual business; all media comes from KB blueprint assets, stock or generation.
- **④ Fill blind spots from KB**: the reference won't reveal mobile behavior, reduced-motion or empty states — pull the closest michael-design category blueprint + repertoire sections to fill these gaps in the same visual language.
- **⑤ Verify**: side-by-side screenshots at 375/768/1440 against the baseline; check palette fidelity, spacing rhythm, hero pattern, motion presence; write down deliberate deviations and why.

## Website Redesign & Refactor Workflow — 重构/改版: audit first, migrate in working slices [sections/redesign-refactor-workflow]

Redesigning an existing project starts from evidence about what exists, and migrates without ever breaking the build (先审计后改造，切片迁移不断链):

- **Audit before touching**: stack and entry points, routes/pages, existing components and their reuse, data sources/APIs, global styles and any existing tokens; screenshot every page as regression baseline; split the plan into KEEP (business logic, data flow, SEO/urls) vs REPLACE (visual layer).
- **Migration order**: ① design tokens first — introduce semantic color/spacing/radius/font variables and map the KB-adopted palette onto them; ② base primitives — swap buttons/inputs/cards to shadcn + cva one component at a time; ③ layout shells — nav and footer; ④ section-by-section rebuild using the layout repertoire, highest-traffic page first; ⑤ motion layer last (SectionReveal + micro-interactions + signature moment).
- **Always-compiling slices**: each step ships a working page; old styles coexist until a section fully migrates; never restyle and change business logic in the same slice.
- **KB adoption in redesign**: palette from the closest category blueprint hit or Curated Palette Library (exact hex, recorded source); composition upgrades from Layout Composition Repertoire; motion system from ONE motion hit. Redesigns follow the same "来源 section → token/class → 页面落点" bookkeeping as greenfield builds.
- **Regression pass per slice**: compare against baseline screenshots; verify 375/768/1440, forms still submit, links/routes resolve, no dead imports left behind.

## KB Evidence Composition — palette, style, motion and media from hits become ONE system [sections/kb-evidence-composition]

Retrieval without adoption is a defect: every michael-design hit must produce recorded adoptions or an explicit rejection reason (每条命中要么落地要么写明弃用，禁止"参考了知识库"空话):

- **Compose across hits**: exact palette hex from the closest category blueprint (or Curated Palette Library set) → semantic tokens; typography pairing from the typography section; per-section composition from Layout Composition Repertoire; ONE compatible motion combo taken from a single hit (mixing two blueprints' motion systems clashes); real media URLs from blueprint hits.
- **Bookkeeping format**: `来源 section → 采用项(具体值) → token/class → 页面落点` for every adoption — e.g. "Lodge Booking [apps/lodge-booking-app] → #B45309 amber CTA → --primary → hero 双按钮/日期选择器高亮".
- **Conflict rule**: when hits disagree (radius, shadow depth, easing), the category blueprint outranks generic rule sections; keep exactly ONE value per token — never blend two radii or two easings into the same page.
- **Coherence gate before writing code**: single neutral + accent palette, one easing + duration scale, one icon family, one radius token, media style consistent (photography vs illustration, not both). If the composed set fails any of these, re-pick before implementation.

## Real Asset Sourcing — images, video, avatars from KB hits and reliable URL patterns [sections/real-asset-sourcing]

Media makes the site real; sourcing follows a strict priority so every image URL actually loads (素材有优先级，每个 URL 必须真实可加载):

- **Priority**: ① real asset URLs inside michael-design blueprint hits — the corpus carries curated CDN images/videos per category (preview/asset links and in-prompt media URLs), reuse them when the category matches; ② project `assets/`/`public/` files; ③ deterministic stock patterns — `https://images.pexels.com/photos/{id}/...?auto=compress&w=1200` for category photos, `https://picsum.photos/seed/{slug}/1200/800` for stable placeholders; ④ avatars — `https://i.pravatar.cc/150?img={1-70}` or Pexels portraits for every human (testimonials/team/hosts), initials circles forbidden; ⑤ `generate_image` only for brand-specific art: logo, mascot, custom illustration, UI mockups.
- **Category match**: media must depict the actual business (民宿 → cabins/forest/interiors; cafe → drinks/space/hands) — a beautiful but off-category photo is a defect.
- **Every media element ships**: `loading="lazy"`, an `aspect-ratio` container (no layout shift), meaningful `alt`, and an `onError` fallback swapping to a same-size backup URL — never `display:none` leaving holes.
- **Verification**: each external URL is checked to render before delivery (curl status / screenshot pass); a full site carries 3+ loadable media minimum, with video/GIF in at least one key section when the category warrants motion imagery.
