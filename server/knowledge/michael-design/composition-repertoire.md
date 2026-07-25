# Michael Design Library — composition-repertoire

Layout arrangements, motion/effects, shadcn/ui component coverage and content density standards (排列风格 / 动效特效 / 组件库 / 内容密度武器库). These are repertoires to COMPOSE FROM per business category — not checklists to dump onto one page.

## Layout Composition Repertoire — vary the arrangement style per section [sections/layout-composition-repertoire]

Every page uses at least 4 distinct composition patterns; adjacent sections never repeat the same one (相邻区块不得同构图). Pick by content type, all Tailwind-native:

- **Bento grid**: mixed tile sizes — `grid md:grid-cols-3 auto-rows-[180px] gap-4` with feature tiles `md:col-span-2 md:row-span-2`; best for feature sets, galleries, stats mixes.
- **Alternating split (zigzag)**: 40/60 or 50/50 media+copy, `md:flex-row` / `md:flex-row-reverse` per row; the workhorse for value narratives.
- **Editorial asymmetric**: offset columns (`md:mt-16` on one column), overlapping images (`-mt-12 md:-ml-16 z-10`), pull quotes, generous whitespace; for story/brand sections.
- **Masonry**: `columns-2 md:columns-3 gap-4 [&>*]:mb-4 [&>*]:break-inside-avoid`; for image-heavy galleries and testimonials of varied length.
- **Horizontal snap rail**: `flex overflow-x-auto snap-x snap-mandatory gap-6 [&>*]:snap-start [&>*]:shrink-0`; for cards exceeding 4–6 on mobile/desktop, destinations, menus.
- **Sticky scroll story**: left `md:sticky md:top-24 self-start` media, right column of scroll steps; desktop-only signature pattern, stacks on mobile.
- **Timeline**: vertical `border-l-2` line + alternating/offset cards, dot markers `absolute -left-[9px]`; for process, history, itinerary.
- **Full-bleed band**: `w-full` colored/photo band with inner `max-w-7xl mx-auto px-6`; for metrics, CTA, quotes — must use the brand accent scale.
- **Dense table/list**: real `<Table>` with sorting for spec/pricing comparisons; enterprises show data in tables, not cards.
- **Layered/overlap hero**: foreground card overlapping background media (`relative -mb-24` + next section `pt-32`); creates depth without heavy effects.
- Mobile fallback defined per pattern: rails stay rails, bento flattens to 1–2 cols, sticky story becomes plain stacked, overlaps reduce offsets.

## Motion & Effects Repertoire — scroll, hover, ambient, text effects with responsive degradation [sections/motion-effects-repertoire]

Compose a motion system: 1 signature moment + reusable section reveals + micro-interactions (动效体系＝1 标志性时刻+复用揭示+微交互). All effects ship a mobile and `prefers-reduced-motion` story or they don't ship.

- **Scroll-driven**: `whileInView` fade/slide reveals with children stagger 0.08–0.12s; parallax layers (`useScroll`+`useTransform` translateY 10–20%); top scroll progress bar; sticky pinning sequences (desktop only); scroll-linked image swap for product walkthroughs.
- **Ambient**: Ken Burns on hero media (scale 1→1.06, 12–20s); animated gradient mesh/aurora blobs (`blur-3xl opacity-30` + 18–25s keyframed morph, 2–4 blobs max); logo-wall marquee (CSS `animate-[marquee_30s_linear_infinite]`, pause on hover, static grid on mobile); subtle noise overlay (SVG feTurbulence ~2% opacity).
- **Hover/micro**: 3D tilt cards (rotateX/Y ≤6° from pointer, desktop only); spotlight card (radial-gradient following cursor); border-beam/gradient-border on featured cards; magnetic buttons (≤8px pull); image zoom `group-hover:scale-105` inside `overflow-hidden`; underline slide-in for links; icon nudge `group-hover:translate-x-1`.
- **Text & numbers**: gradient text (`bg-gradient-to-r bg-clip-text text-transparent`), shine sweep on headings, staggered word reveal for hero headline, count-up metrics on view (once), typewriter only when category justifies it.
- **Glass & depth**: glassmorphism panels (`bg-white/10 backdrop-blur-xl border border-white/15` + inset top highlight) over photo/gradient backgrounds only — never on flat white; layered shadow scale for elevation.
- **Implementation**: Framer Motion variants for orchestration; pure CSS keyframes in globals for ambient loops; IntersectionObserver fallback without the lib. One easing (`cubic-bezier(0.16,1,0.3,1)`) + one duration scale page-wide.
- **Responsive motion**: mobile removes parallax/pin/tilt/particles/marquee-autoplay, shortens reveal distances to ~12px; `prefers-reduced-motion: reduce` → opacity-only ≤300ms everywhere (`useReducedMotion` / `motion-reduce:` variants).

## shadcn/ui Component Coverage — real primitives with Tailwind semantics everywhere [sections/shadcn-component-coverage]

A full site genuinely USES 8–10+ distinct shadcn/ui primitives with real interaction logic — importing without wiring is a defect (至少 8-10 种 primitive 真实交互使用，不是只 import). Map needs to primitives instead of hand-rolling divs:

- **Navigation**: `NavigationMenu` (desktop dropdowns), `Sheet` (mobile drawer menu), `DropdownMenu` (user/actions), `Breadcrumb` (inner pages).
- **Content & display**: `Card` (only for true repeated entities), `Tabs` (use-case/product switching, controlled state), `Accordion` (FAQ, 6–10 items), `Carousel` (embla — hero or testimonial sliders), `HoverCard` (rich previews), `Tooltip` (icon buttons), `Badge` (status/labels), `Avatar` with `AvatarFallback`, `AspectRatio` (media stability), `Separator`.
- **Forms & input**: `Form` + zod validation with error/success states, `Input`/`Textarea` with focus rings, `Select`, `RadioGroup`/`Checkbox`/`Switch`, `Slider` (price/filter ranges), `Calendar` + `Popover` (date pickers for bookings), `InputOTP` (verification flows).
- **Feedback & flow**: `Dialog` (detail views, booking flow), `AlertDialog` (confirmations), `Sonner` toast (form success), `Skeleton` (loading), `Progress`, designed empty states.
- **Data**: `Table` with sorting for comparisons/specs, `Pagination`, `Command` (search palette for content-heavy sites).
- **Discipline**: variants via `cva` (`buttonVariants({ variant: "outline", size: "lg" })`); `asChild` to render link CTAs as `<a>`; all styling through semantic Tailwind classes (`bg-primary text-primary-foreground`, `text-muted-foreground`, `border-border`) so theme tokens propagate; extend components in `components/ui`, don't fork styles inline.

## Content Richness & Authenticity — sparse pages read as fake sites [sections/content-richness]

A site with 3 placeholder cards per section reads as a fake/demo site (内容稀疏＝假网站). Enterprise sites feel real because they are DENSE with specific content:

- **Volume floor for a full site**: 25+ real content items overall — e.g. 8–12 listings/products with distinct photos/prices/details, 6+ testimonials with avatar+name+role, 8–10 FAQ entries with substantive answers, 4+ team/host profiles, 6–10 destination/category tiles, real metrics.
- **Specificity**: concrete prices, dates, locations, dimensions, ratings with review counts ("4.8 · 127 条评价"), operating hours, policies — numbers must stay consistent across sections. No "优质服务/卓越体验" filler adjectives.
- **Data modeling**: content lives in typed arrays (`src/data/*.ts` — `listings`, `testimonials`, `faqs`) and sections render the full list (grids, rails, filters); never hardcode 3 JSX cards. This makes density cheap and future CMS-ready.
- **Copy variance**: mix long and short entries, different sentence rhythms per testimonial/FAQ, category-correct vocabulary (民宿写「林间/入住/房型」，SaaS 写 workflow/integration) — uniform-length AI copy is a fake-site tell.
- **Media everywhere**: every entity has a real image (multiple for featured entities — gallery dialogs), all humans have real avatar photos, key sections carry video/GIF where the category warrants.
- **Secondary depth**: nav anchors/links resolve to real sections or pages, footer links are real, at least one entity type opens a detail Dialog/page with expanded content; microcopy on forms (hints, privacy note, success state) completes the illusion of a living product.
