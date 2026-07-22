# UI/UX & Visual Design

> Retrieval cheat-sheet for an LLM coding agent building interfaces. Default output of a naive model looks amateur: cramped spacing, pure black on white, tiny gray text, no hover states, gray placeholder boxes, everything centered. This file is the antidote. Apply these rules and the result reads as a polished, professional product. When in doubt, prefer MORE whitespace, FEWER colors, FEWER fonts, and stronger contrast for hierarchy.

## Spacing & Layout

Spacing is the single biggest tell of amateur vs. professional UI. Inconsistent gaps look broken even when nothing else is wrong.

- **Use a fixed spacing scale. Never use arbitrary px values.** Base unit = 4px. Scale: `4, 8, 12, 16, 24, 32, 48, 64, 96` (px). In Tailwind these are `1, 2, 3, 4, 6, 8, 12, 16, 24`. Every margin, padding, and gap must come from this scale. A `13px` or `7px` gap is a bug.
- **The 8px rhythm:** prefer multiples of 8 for layout-level spacing (section gaps, card padding); use 4px multiples only for tight intra-component spacing (icon-to-label, badge padding).
- **Whitespace is not wasted space.** It groups, separates, and creates focus. The most common fix to "this looks cheap" is *add more space*. Generous negative space signals confidence and premium quality.
- **Padding minimums (don't go below these):**
  - Buttons: `8–12px` vertical, `16–24px` horizontal.
  - Cards / panels: `16–24px` all sides (`24–32px` for hero/feature cards).
  - Page gutters (mobile): `16px` min; (desktop): `24–48px`.
  - Input fields: `8–12px` vertical, `12–16px` horizontal.
  - Modal body: `24px`.
- **Proximity = relationship.** Related items close together (`8–12px`); unrelated groups far apart (`32–48px`). The gap *between* groups must be visibly larger than the gap *within* a group, or grouping reads as ambiguous.
- **Alignment: pick an edge and commit.** Left-align text and form fields to a shared vertical axis. Misaligned left edges (even by 2px) are instantly noticeable. Use a layout grid or fl/grid container — never eyeball positions with random margins.
- **Grid:** use a 12-column grid for desktop layouts; it divides cleanly into halves, thirds, quarters. Gutter `16–24px`. CSS Grid or flexbox with `gap` — never margins on children to fake gaps (margins collapse and double up).
- **Max content width for readability.** Long-form text: cap line length at **~65 characters** (`max-width: 65ch`, roughly `600–700px`). Body paragraphs spanning the full width of a 1440px screen are unreadable. Full-width layouts cap the *container* at `1200–1280px` and center it; text columns are narrower still.
- **Consistent gaps in lists/grids.** Every card in a grid uses the identical `gap`. A 23px gap here and 25px there looks broken.
- **Vertical rhythm.** Stack sections with consistent vertical spacing (e.g. `64–96px` between major page sections, `32px` between subsections).

**COMMON PITFALLS (cheap tells):**
- Arbitrary, inconsistent spacing (`margin: 13px`, `padding: 7px 19px`). → Snap everything to the 4/8 scale.
- Cramped layouts with no breathing room — content jammed edge-to-edge. → Add padding; let it breathe.
- Equal spacing everywhere (no proximity grouping), so nothing reads as related. → Vary spacing to express structure.
- Text running the full browser width. → Cap at `65ch`.
- Faking gaps with `<br>` or `&nbsp;` or per-child margins instead of `gap`. → Use flex/grid `gap`.

## Typography

- **Limit to 1–2 typefaces.** One is often enough. If two: one for headings, one for body (or one sans + one mono for code). Three or more fonts looks chaotic and unprofessional.
- **Safe, high-quality defaults:** system stack `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif` is clean and zero-cost. For more character: `Inter`, `Geist`, `IBM Plex Sans`, `Söhne`-likes for UI; `Source Serif`, `Newsreader` for editorial. Mono: `JetBrains Mono`, `Fira Code`, `ui-monospace`.
- **Use a modular type scale.** Don't pick random sizes. A 1.2–1.25 ratio scale (px): `12, 14, 16, 18, 20, 24, 30, 36, 48, 60`. Body text = **16px** (never below 14px for primary content). Captions/labels = `12–14px`.
- **Line-height (leading):**
  - Body text: **1.5** (range 1.4–1.6). Default browser `normal` (~1.2) is too tight for paragraphs.
  - Headings: **1.1–1.25** (tighter — large text needs less leading).
  - UI labels / single lines: `1.2–1.4`.
- **Font-weight hierarchy.** Establish clear steps: body `400` (regular), emphasis/labels `500` (medium) or `600` (semibold), headings `600–700` (semibold/bold). Avoid `300` (light) for body — it's thin and low-contrast, especially on screens. Don't bold everything; if everything is bold, nothing stands out.
- **Letter-spacing (tracking):** large headings often look better slightly tightened (`-0.01em` to `-0.02em`). ALL-CAPS labels need *added* tracking (`0.05em–0.1em`) or they look mashed. Never track-tighten small body text.
- **Contrast — the #1 readability tell.** Body text on white should be near-black but not pure black: `#1A1A1A`–`#212529` (gray-900). Pure `#000` on `#FFF` is harsh and "vibrates." Secondary text: gray-600 (`#4B5563`-ish) — must still pass **4.5:1**. Never use light gray (`#999`, `#AAA`, `#CCC`) for text humans must read.
- **Hierarchy via size + weight + color**, not size alone. Page title `30–36px/700`, section heading `20–24px/600`, body `16px/400`, caption `13–14px/500 gray-500`.
- **Measure and wrapping:** prevent orphans/widows where easy; cap heading line length too. Use `text-wrap: balance` for headings, `text-wrap: pretty` for body where supported.

**COMMON PITFALLS:**
- Tiny gray text (`12px #999`) for anything important — reads as disabled and fails contrast. → `14–16px`, gray-600+ minimum.
- Pure black `#000` body on pure white. → gray-900 (`#1A1A1A`).
- Too many font sizes (10 ad-hoc sizes). → Pick 5–7 from a scale.
- Default `line-height: normal` on paragraphs (too cramped). → `1.5`.
- Mixing 3+ typefaces. → 1–2 max.
- Light `300` weight body text. → `400`+.
- Headings barely larger than body (no contrast in scale). → Use a real ratio; make H1 clearly dominant.

## Color

- **Restrained palette.** One primary brand color + a neutral gray ramp + 1–2 accents. That's it. A rainbow of unrelated colors looks amateur. Most professional UIs are mostly neutral with sparing color.
- **60–30–10 rule.** ~60% dominant/neutral (backgrounds, surfaces), ~30% secondary (text, containers), ~10% accent (primary actions, highlights). The accent is rare *on purpose* — that's what makes the primary CTA pop.
- **Use a real neutral ramp, not ad-hoc grays.** Define ~10 steps and use them consistently:
  ```
  gray-50  #F9FAFB   (page bg / subtle fills)
  gray-100 #F3F4F6   (hover bg, muted surfaces)
  gray-200 #E5E7EB   (borders, dividers)
  gray-300 #D1D5DB   (input borders, disabled borders)
  gray-400 #9CA3AF   (placeholder text, icons-muted)
  gray-500 #6B7280   (secondary text — min for body-on-white)
  gray-600 #4B5563   (body secondary, strong)
  gray-700 #374151   (headings-muted)
  gray-800 #1F2937   (high-emphasis text)
  gray-900 #111827   (primary text / near-black)
  ```
  Slightly cool or warm grays (a hint of blue or beige) look more designed than pure neutral `#808080`. Pick one temperature and stay consistent.
- **Borders & dividers should be subtle.** Use gray-200 (`#E5E7EB`) on white — a faint line, not a hard `1px solid black`. Heavy black borders look like a wireframe.
- **Semantic colors** (with light bg variants for banners/badges):
  - Success: green `#16A34A` (text/icon), bg `#F0FDF4`, border `#BBF7D0`.
  - Warning: amber `#D97706`, bg `#FFFBEB`, border `#FDE68A`.
  - Error/destructive: red `#DC2626`, bg `#FEF2F2`, border `#FECACA`.
  - Info: blue `#2563EB`, bg `#EFF6FF`, border `#BFDBFE`.
  Never use raw `red`/`green`/`yellow` keywords — they're garish. Use the tuned hexes.
- **Generate tints/shades from the primary**, don't pick unrelated colors. For a blue primary `#2563EB`: hover `#1D4ED8` (darker), light fill `#EFF6FF`, ring `rgba(37,99,235,0.4)`.
- **Don't rely on color alone** to convey meaning (accessibility + colorblind users): pair color with an icon, label, or shape (e.g. error = red + warning icon + message).
- **Saturation discipline:** avoid pure max-saturation fills across large areas — they fatigue the eye. Desaturate slightly for surfaces; reserve vivid saturation for small accents.

### Dark mode (done right)

- **Don't just invert.** Inversion produces muddy, harsh results. Design dark mode as its own palette.
- **No pure black background.** Use a dark gray like `#0F172A`, `#121212`, or `#18181B`. Pure `#000` makes shadows invisible and elevation impossible to read; it also looks like an "off" screen.
- **No pure white text.** Use `#E5E7EB`/`#F3F4F6` (~90% white) for body to reduce glare/halation.
- **Elevation = lighter, not shadow.** In dark mode, raise surfaces by making them *lighter* (`#1E293B` card on `#0F172A` bg), since shadows barely show. Higher elevation → lighter surface.
- **Desaturate accents.** Saturated colors glow uncomfortably on dark backgrounds. Lighten and slightly desaturate the primary so it's legible (e.g. blue-400 instead of blue-600).
- **Maintain contrast both ways:** still need 4.5:1 for text. Mid-grays that pass on white often fail on dark — re-check.
- **Borders:** use subtle light borders (`rgba(255,255,255,0.08–0.12)`) instead of dark ones.

**COMMON PITFALLS:**
- Too many competing colors / no clear neutral base. → 1 primary + grays + 1 accent.
- Pure `#808080` ad-hoc grays scattered around. → One consistent gray ramp.
- Garish keyword colors (`color: red`). → Tuned semantic hexes.
- Hard `1px solid #000` borders everywhere. → Subtle gray-200.
- Dark mode = `filter: invert()` or pure-black bg + pure-white text. → Purpose-built dark palette, `#121212`-ish bg, ~90% white text, lighter elevated surfaces.
- Accent color used so much it stops being an accent. → Keep it ~10%.

## Visual Hierarchy

The eye should land on the most important thing first, then flow in priority order. Amateur UIs are flat — everything competes equally.

- **Four levers of hierarchy:** size, weight, color/contrast, and spacing (isolation). Combine them; don't rely on one. To emphasize: bigger + bolder + higher-contrast + more surrounding whitespace. To de-emphasize: smaller + lighter weight + muted gray + grouped tightly.
- **One clear primary action per screen/section.** Exactly one solid, high-contrast button (the "happy path"). Everything else is secondary. Two equally-weighted primary buttons create decision paralysis and look unconsidered.
- **De-emphasize secondary actions.** Secondary = outline/ghost/tonal button or a plain text link. Tertiary/destructive-but-rare = subtle text button. Never render "Cancel" as a loud filled button next to "Save."
- **Establish a scan pattern.** Western readers scan top-left → right, in an F or Z pattern. Put the most important content/CTA where the eye lands. Don't bury the primary action at bottom-right of a sea of equal elements.
- **Contrast creates focus.** The primary CTA should have the highest contrast on the screen. If everything is high-contrast, add muting to the rest so the CTA wins.
- **Size signals importance** but use restraint — a 72px number for a key metric, 14px for its label. Make the hierarchy *obvious*, not subtle.
- **Group and chunk.** Use cards, dividers, and whitespace to break dense screens into digestible sections. A wall of uniform rows has no hierarchy.
- **Reduce visual noise** competing with content: thin borders, muted secondary text, limited color. Let the important thing be the loud thing.

**COMMON PITFALLS:**
- Flat design where every element has equal weight. → Deliberately rank with size/weight/color/space.
- Multiple primary buttons. → One primary; rest secondary.
- Loud "Cancel"/secondary buttons competing with the main CTA. → Mute them.
- Everything bold or everything the same size. → Create real steps.
- No focal point — user doesn't know where to look. → Make the #1 element clearly dominant.

## Components That Look Pro

The difference between amateur and pro is mostly **states and details**. A button with no hover state instantly looks unfinished.

### Buttons
- **Always implement all states:** default, **hover**, **active/pressed**, **focus-visible** (ring), **disabled**, and **loading**. Missing hover/focus is the most common "unfinished" tell.
- Sizing: height `36–44px` (`40px` is a great default); padding `8–12px / 16–24px`; `border-radius: 6–8px` (or pill `9999px` for a softer brand); font-weight `500–600`.
- **Hover:** darken primary by ~8–10% (or raise elevation). **Active:** darken a touch more / `transform: translateY(1px)` or `scale(0.98)`. Add `transition: 150ms ease`.
- **Disabled:** `opacity: 0.5` (or muted gray fill), `cursor: not-allowed`, no hover effect. Don't just gray the text.
- **Loading:** show a spinner *in place*, keep the button width stable (don't let it collapse), disable interaction, optionally dim label. Never leave a button looking clickable while an action is in flight.
- Variants: **primary** (solid, brand), **secondary** (outline or gray tonal), **ghost** (transparent, hover bg), **destructive** (red). Icon buttons need a min `40×40px` hit area and an `aria-label`.

### Inputs / Forms
- Height match buttons (`36–44px`), padding `8–12px / 12–16px`, `border: 1px solid gray-300`, `border-radius: 6–8px`, font `14–16px` (16px on mobile to prevent iOS zoom).
- **Focus ring is mandatory.** On focus: change border to primary AND add a ring (`box-shadow: 0 0 0 3px rgba(primary,0.2)`). Don't remove the outline without replacing it — keyboard users are then lost. **Never `outline: none`** alone.
- **Placeholder ≠ label.** Always have a visible `<label>` above (or floating). Placeholder text disappears on input and fails contrast if used as the label. Placeholder color = gray-400.
- **Error state:** red border + red helper text *below* the field + (ideally) an icon. State *what's wrong and how to fix it* ("Enter a valid email", not "Invalid"). Don't rely on red border alone (colorblind).
- Helper/hint text: gray-500, `12–14px`, below the field. Required marker: `*` or "(required)".
- Generous vertical spacing between fields (`16–24px`); group related fields.

### Cards
- **Subtle elevation, not heavy.** Either a thin border (`1px solid gray-200`) OR a soft shadow — usually not both heavy. Good shadow: `box-shadow: 0 1px 3px rgba(0,0,0,0.1), 0 1px 2px rgba(0,0,0,0.06)`. Avoid harsh `0 0 10px black` drop shadows (mid-2000s look).
- Padding `16–24px`, `border-radius: 8–12px`, white/`gray-50` surface.
- Shadows should be soft, diffuse, low-opacity, and directionally consistent (light from above → shadow below). Multiple competing shadow directions look broken.
- On hover (if interactive): slightly raise shadow / translateY(-2px) with transition.

### Modals / Dialogs
- **Backdrop:** semi-opaque overlay `rgba(0,0,0,0.4–0.6)` (dims background, focuses attention). Never a modal floating with no backdrop.
- Centered (or top-anchored), `max-width: 400–560px` for dialogs, `border-radius: 8–16px`, padding `24px`, soft large shadow, white surface.
- Structure: title (`18–20px/600`), body, action row bottom-right (primary + secondary). Close (×) top-right.
- **Trap focus** inside; **Esc** closes; clicking backdrop closes (unless destructive). Return focus to the trigger on close. Lock body scroll while open.
- Entrance: fade + slight scale/slide (`150–200ms`). Don't pop in abruptly.

### Empty States
- **Never show a blank screen or empty table.** An empty state needs: a friendly illustration/icon, a one-line headline ("No projects yet"), a sentence of guidance, and a primary CTA ("Create your first project").
- Make it inviting and actionable, not a dead end. First-run empty states are a key onboarding moment.

### Loading / Skeletons
- For content loads, prefer **skeleton screens** (gray placeholder shapes matching the final layout) over spinners — they reduce perceived wait and prevent layout shift.
- Skeletons: `gray-100/200` blocks with a subtle shimmer animation (`1.5s` pulse), shaped like the real content (avatar circle, text bars). Reserve final dimensions to avoid CLS.
- Spinners are fine for short/button-level waits. For >~10s operations, show progress or status text.

**COMMON PITFALLS:**
- Buttons/links with no hover or focus state. → Implement all states + focus ring.
- `outline: none` with no replacement. → Always provide a visible focus indicator.
- Placeholder used instead of a label. → Real `<label>`.
- Heavy/harsh drop shadows or shadow + thick border together. → One subtle elevation cue.
- Blank empty states. → Illustration + guidance + CTA.
- Layout jump when content loads (no skeleton/reserved space). → Skeletons / fixed dimensions.
- Disabled buttons that still look/behave clickable. → `opacity:0.5` + `cursor:not-allowed` + no handlers.

## Real Content, Never Placeholder Gray Boxes

Nothing screams "unfinished prototype" like gray rectangles labeled "image" and "Lorem ipsum." Always populate with realistic content.

- **Images:** use real placeholder *photos*, not gray boxes.
  - `https://picsum.photos/600/400` (random photo) or seeded `https://picsum.photos/seed/{any}/600/400` for stable images.
  - Unsplash Source / real Unsplash URLs for topical imagery.
  - Always set explicit `width`/`height` (or aspect-ratio) to prevent layout shift, and meaningful `alt` text.
- **Avatars:** use a real avatar service, never an empty circle.
  - `https://i.pravatar.cc/100?img=12` (real face photos) or `https://i.pravatar.cc/100?u={email}` (stable per user).
  - DiceBear for illustrated/initials avatars: `https://api.dicebear.com/7.x/avataaars/svg?seed={name}` (styles: `initials`, `bottts`, `notionists`, `personas`).
- **Copy:** write **realistic domain copy**, not Lorem ipsum, in anything user-facing or demo-facing. Real names ("Sarah Chen"), plausible product names, real-sounding email subjects, actual button verbs ("Save changes", "Invite teammate"). Lorem ipsum is acceptable only as a transient internal stub — never in delivered output.
- **Realistic data:** populate tables/lists with believable rows (varied names, dates, amounts, statuses) so layout is tested against real-shaped data. Include long values and edge cases (very long name, $1,234,567.89, empty optional field) to catch overflow.
- **Icons:** use a real icon set (Lucide, Heroicons, Phosphor, Tabler) — consistent stroke width and size. Don't mix icon families (mismatched weights look off). Size to text (`16–20px`), align to baseline.
- **Favicons / logos:** include a real or generated logo/mark, not a default placeholder.

**COMMON PITFALLS:**
- Gray boxes with "image" / "photo" text. → `picsum.photos`.
- Empty avatar circles or "JD" on flat gray. → `pravatar`/`dicebear`.
- "Lorem ipsum dolor sit amet" shipped to users. → Real, contextual copy.
- Single-row demo data that hides overflow/wrapping bugs. → Varied, realistic, edge-case data.
- Mixed icon sets / inconsistent icon sizes. → One family, consistent size & stroke.

## Responsive Design

- **Mobile-first.** Write base styles for small screens, then add complexity at larger breakpoints with `min-width` media queries. It forces prioritization and yields cleaner CSS.
- **Standard breakpoints (px):** `sm 640`, `md 768`, `lg 1024`, `xl 1280`, `2xl 1536`. Design for the common buckets: phone (`<640`), tablet (`768–1024`), desktop (`>1024`).
- **No horizontal scrolling** (unless an intentional carousel). Use `max-width: 100%` on media, `box-sizing: border-box`, flex-wrap, and fluid units. A page that scrolls sideways on mobile looks broken. Test at `360px` width.
- **Touch targets ≥ 44×44px** (Apple HIG) / 48×48dp (Material). Small tap targets and links crammed together cause mis-taps. Add spacing between tappable items (`8px+`).
- **Fluid type & spacing:** use `clamp()` for headings (`font-size: clamp(1.75rem, 4vw, 3rem)`) so they scale smoothly instead of snapping. Relative units (`rem`) respect user font settings.
- **Reflow, don't shrink.** Multi-column desktop layouts should stack to one column on mobile, not just scale down. Convert sidebars to top nav / drawer; convert wide tables to cards or horizontally-scrollable containers with sticky first column.
- **Navigation:** desktop horizontal nav → mobile hamburger/drawer or bottom tab bar. Ensure the menu is reachable and closes properly.
- **Test the real breakpoints** and especially the awkward in-betweens (e.g. `820px`) where layouts often break.
- **Images:** serve responsive sizes (`srcset`/`sizes`), use `object-fit: cover` to avoid distortion, lazy-load below the fold.

**COMMON PITFALLS:**
- Fixed pixel widths that overflow small screens → horizontal scroll. → Fluid widths, `max-width:100%`.
- Desktop-only layout merely zoomed out on phones (tiny tap targets). → Reflow to single column, ≥44px targets.
- Links/buttons too small or too close to tap. → 44px targets, spacing.
- Wide tables that blow out mobile width. → Card layout or scroll container.
- Forgetting to test `<400px` and tablet widths. → Test 360/768/1024.

## Microinteractions & Feedback

Every user action needs immediate, legible feedback. Silence after a click feels broken.

- **Hover states on everything interactive.** Buttons, links, rows, cards, icons. Cursor `pointer` on clickables. Hover = subtle bg change, color shift, or elevation. A clickable element that doesn't react on hover feels dead (and is undiscoverable).
- **Transition timing:** `150–250ms` is the sweet spot for most UI (hover, color, small moves). Use `200ms` as a default. Micro (color/opacity): `100–150ms`. Larger entrances (modals, drawers): `200–300ms`. Anything `>400ms` feels sluggish; `<80ms` feels instant/jarring.
- **Easing:** use `ease`, `ease-out` (great for entrances — fast then settle), or a custom `cubic-bezier(0.4, 0, 0.2, 1)` (Material standard). **Never `linear`** for UI motion — it feels robotic. `ease-in` for exits.
- **Animate the right properties.** Prefer `transform` and `opacity` (GPU-accelerated, smooth). Avoid animating `width/height/top/left/margin` (cause layout reflow, jank).
- **Respect `prefers-reduced-motion`** — disable/reduce non-essential animation for users who request it.
- **Provide state feedback for every action:**
  - Click a submit → button enters loading (spinner) immediately.
  - Success → toast/inline confirmation; navigate or update UI.
  - Failure → clear error message, button re-enabled.
- **Optimistic UI** for fast-feeling apps: update the UI immediately on action (e.g. like count +1, item appears), then reconcile with the server; roll back + toast on failure. Great for likes, toggles, adding list items.
- **Toasts, not `alert()`.** Use non-blocking toast notifications (top-right or bottom-center) for transient feedback. Auto-dismiss after `4–6s`, with success/error/info color + icon, and a manual dismiss. **Never** use browser `alert()`/`confirm()`/`prompt()` — they're jarring, unstyled, and block the thread. For confirmations use a styled modal.
- **Skeletons/spinners** during waits (see Components). Disable submit while in flight to prevent double-submit.
- **Animate purposefully.** Motion should clarify (where did this come from, what changed), not decorate. No gratuitous bouncing/spinning. Subtle > flashy.

**COMMON PITFALLS:**
- No hover feedback on buttons/links/rows. → Add hover styles + `cursor:pointer`.
- Instant, abrupt state changes (no transition). → `150–250ms ease`.
- `linear` easing / overly long animations. → `ease-out`, `~200ms`.
- `alert()`/`confirm()` for feedback. → Toasts + styled modals.
- No feedback after submit (user clicks again). → Loading state + disable.
- Animating layout props causing jank. → Animate `transform`/`opacity`.
- Decorative motion everywhere distracting from content. → Purposeful, subtle motion; honor reduced-motion.

## Accessibility Basics

Accessible UIs are also clearer, more usable UIs. These are baseline, not optional.

- **Contrast ratios (WCAG AA):**
  - Normal text: **≥ 4.5:1** against its background.
  - Large text (≥`18px bold` or `24px regular`): **≥ 3:1**.
  - UI components & meaningful graphics (icons, input borders, focus indicators): **≥ 3:1**.
  - Verify with a contrast checker; don't eyeball. Light-gray-on-white text is the most common failure.
- **Visible focus indicator.** Keyboard users must always see what's focused. Keep a clear focus ring (`2–3px`, high-contrast, offset). Use `:focus-visible` to show it for keyboard but not mouse if desired — but never remove it outright.
- **Semantic HTML.** Use the right element: `<button>` for actions (not a clickable `<div>`), `<a>` for navigation, `<nav>/<main>/<header>/<footer>/<section>`, `<h1>`–`<h6>` in order (one `<h1>`, no skipping levels), `<ul>/<li>` for lists, `<label>` tied to inputs (`for`/`id`), `<table>` for tabular data. Semantics give you keyboard + screen-reader behavior for free.
- **Alt text** on meaningful images (describe content/purpose); empty `alt=""` for purely decorative images (so screen readers skip them). Don't write "image of…" — just describe it.
- **Keyboard navigation.** Everything operable by mouse must work by keyboard: Tab order logical (follows visual order), Enter/Space activate buttons, Esc closes overlays, arrow keys for menus/tabs/sliders. Don't create keyboard traps. Manage focus on route/modal changes.
- **ARIA where needed (and only where needed).** Native semantics first; ARIA to fill gaps: `aria-label`/`aria-labelledby` for icon-only buttons and unlabeled controls, `role="dialog"` + `aria-modal="true"` for modals, `aria-live="polite"` regions for toasts/async updates, `aria-expanded`/`aria-controls` for disclosures/menus, `aria-current` for active nav. **Rule: no ARIA is better than wrong ARIA.** Don't slap `role`s on semantic elements.
- **Forms:** every input has a programmatic label; errors associated via `aria-describedby`; required state via `required`/`aria-required`; group with `<fieldset>/<legend>`.
- **Don't convey info by color alone** (also a design rule): pair with text/icon/pattern.
- **Targets & motion:** ≥44px touch targets; honor `prefers-reduced-motion`; don't autoplay disruptive motion.
- **Respect user settings:** `rem`-based sizing so browser zoom / font-size preferences work; layout must survive 200% zoom.

**COMMON PITFALLS:**
- Low-contrast text/icons (fails 4.5:1 / 3:1). → Darken; verify with a checker.
- `<div onclick>` instead of `<button>`. → Real semantic elements.
- Removed focus outline. → Keep a visible focus indicator.
- Icon-only buttons with no accessible name. → `aria-label`.
- Skipped heading levels / multiple `<h1>`. → Logical, ordered headings.
- Missing/auto-generated `alt`. → Meaningful alt; `alt=""` for decorative.
- Meaning conveyed by color only. → Add text/icon.

## Common Amateur Tells (Quick Audit Checklist)

Run this list against any UI you build; each item is a frequent reason output looks cheap. Fix every one.

- **Inconsistent spacing** — random px values, uneven gaps. → Snap to a 4/8px scale; equalize gaps.
- **Too many fonts / sizes / colors** — visual chaos. → 1–2 typefaces, a type scale, 1 primary + neutrals + 1–2 accents.
- **Pure `#000` on pure `#FFF`** — harsh, "vibrates." → gray-900 text on off-white/white.
- **Tiny low-contrast gray text** (`12px #999`) for important content. → `14–16px`, gray-600+, pass 4.5:1.
- **No hover / focus / active states** — feels dead and unfinished. → Implement all interactive states + focus ring.
- **Gray placeholder boxes & Lorem ipsum** in delivered UI. → Real images (picsum), avatars (pravatar/dicebear), realistic copy.
- **Cramped or absent whitespace** — content jammed together. → Generous padding; let it breathe.
- **Default browser styling** — stock blue links, system buttons, ugly default form controls, no reset. → Apply a baseline reset and intentional component styles.
- **Everything centered** — center-aligned paragraphs and forms look unstable and hard to scan. → Left-align text and form fields; center only short headings/hero or truly symmetric content.
- **No visual hierarchy** — flat, everything equal weight, no clear primary action. → Rank with size/weight/color/space; one primary CTA.
- **Heavy/harsh borders & drop shadows** — wireframe or mid-2000s look. → Subtle gray-200 borders, soft low-opacity shadows.
- **Misaligned elements** — left edges off by a few px, inconsistent baselines. → Align to a shared grid/axis.
- **Full-width long text** — unreadable line lengths. → Cap at `~65ch`.
- **Garish saturated colors** everywhere / raw keyword colors. → Tuned, mostly-neutral palette; restrained accent.
- **`alert()` for feedback** / no feedback at all. → Toasts, inline states, loading indicators.
- **Inconsistent corner radii** (some 4px, some 16px, some square) — looks unsystematic. → Pick a radius scale (e.g. `6/8/12px`) and apply consistently.
- **Broken/zoomed-out mobile layout, tiny tap targets.** → Mobile-first, reflow, ≥44px targets, no horizontal scroll.

### One-line "make it look pro" defaults

When unsure, reach for these safe values:

- Spacing scale: `4 8 12 16 24 32 48 64 96`
- Body: `16px / line-height 1.5 / weight 400 / color #1A1A1A`
- Radius: `8px` (inputs/buttons/cards); pills `9999px`
- Border: `1px solid #E5E7EB`
- Card shadow: `0 1px 3px rgba(0,0,0,0.1), 0 1px 2px rgba(0,0,0,0.06)`
- Transition: `all 200ms ease` (color/opacity/transform only)
- Focus ring: `0 0 0 3px rgba(primaryRGB, 0.4)`
- Primary button: solid brand, `40px` tall, `8–12px / 16–24px` padding, weight `600`, hover darken ~10%
- Palette: 1 primary + the gray ramp above + 1 accent; mostly neutral, 60-30-10
- Content max-width: `1200px` container, `65ch` for text
- Touch target: `≥44px`
- Always: real images/avatars, real copy, all button states, visible focus, single primary CTA per screen

## Copy-Paste CSS Design Tokens (start every project with these)

```css
:root {
  --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  --text-xs: 0.75rem; --text-sm: 0.875rem; --text-base: 1rem;
  --text-lg: 1.125rem; --text-xl: 1.25rem; --text-2xl: 1.5rem;
  --text-3xl: 1.875rem;
  --leading-tight: 1.15; --leading-normal: 1.55;
  --sp-1: 4px; --sp-2: 8px; --sp-3: 12px; --sp-4: 16px;
  --sp-6: 24px; --sp-8: 32px; --sp-12: 48px; --sp-16: 64px;
  --bg: #ffffff; --surface: #f9fafb; --border: #e5e7eb;
  --text: #1f2937; --text-muted: #6b7280; --text-faint: #9ca3af;
  --primary: #2563eb; --primary-hover: #1d4ed8; --primary-light: #eff6ff;
  --success: #16a34a; --warning: #d97706; --danger: #dc2626;
  --radius-sm: 4px; --radius-md: 8px; --radius-lg: 12px; --radius-full: 9999px;
  --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.07), 0 2px 4px -2px rgb(0 0 0 / 0.05);
  --duration: 150ms; --ease: cubic-bezier(0.16, 1, 0.3, 1);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0f172a; --surface: #1e293b; --border: #334155;
    --text: #e2e8f0; --text-muted: #94a3b8; --text-faint: #64748b;
    --primary: #3b82f6; --primary-hover: #60a5fa; --primary-light: #1e3a5f;
    --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.3);
    --shadow-md: 0 4px 6px rgb(0 0 0 / 0.4);
  }
}
```

## Copy-Paste Component Patterns

**Button (all states):**
```css
.btn { display: inline-flex; align-items: center; justify-content: center; gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-4); font: 500 var(--text-sm) var(--font-sans);
  border-radius: var(--radius-md); border: 1px solid transparent; cursor: pointer;
  transition: background var(--duration) var(--ease), box-shadow var(--duration) var(--ease);
  min-height: 36px; }
.btn-primary { background: var(--primary); color: #fff; }
.btn-primary:hover { background: var(--primary-hover); }
.btn-secondary { background: var(--surface); color: var(--text); border-color: var(--border); }
.btn-secondary:hover { background: var(--border); }
.btn:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
.btn:disabled { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
.btn--loading { position: relative; color: transparent; pointer-events: none; }
.btn--loading::after { content: ''; position: absolute; width: 16px; height: 16px;
  border: 2px solid currentColor; border-right-color: transparent; border-radius: 50%;
  animation: spin 0.6s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
```

**Input (all states):**
```css
.input { width: 100%; padding: var(--sp-2) var(--sp-3); font: var(--text-sm) var(--font-sans);
  border: 1px solid var(--border); border-radius: var(--radius-md);
  background: var(--bg); color: var(--text);
  transition: border-color var(--duration) var(--ease), box-shadow var(--duration) var(--ease);
  min-height: 36px; }
.input:focus { border-color: var(--primary); outline: none;
  box-shadow: 0 0 0 3px rgb(37 99 235 / 0.15); }
.input::placeholder { color: var(--text-faint); }
.input--error { border-color: var(--danger); }
.input--error:focus { box-shadow: 0 0 0 3px rgb(220 38 38 / 0.15); }
.input:disabled { background: var(--surface); opacity: 0.6; cursor: not-allowed; }
```

**Card:**
```css
.card { background: var(--bg); border: 1px solid var(--border);
  border-radius: var(--radius-lg); padding: var(--sp-6); }
.card--hoverable { transition: box-shadow var(--duration) var(--ease),
  transform var(--duration) var(--ease); cursor: pointer; }
.card--hoverable:hover { box-shadow: var(--shadow-md); transform: translateY(-1px); }
```

**Badge / Tag:**
```css
.badge { display: inline-flex; align-items: center; padding: 2px var(--sp-2);
  font-size: var(--text-xs); font-weight: 500; border-radius: var(--radius-full);
  background: var(--primary-light); color: var(--primary); }
.badge--success { background: #dcfce7; color: var(--success); }
.badge--danger { background: #fef2f2; color: var(--danger); }
```

**Empty State:**
```css
.empty-state { display: flex; flex-direction: column; align-items: center;
  justify-content: center; padding: var(--sp-16) var(--sp-8);
  text-align: center; color: var(--text-muted); }
.empty-state svg { width: 48px; height: 48px; margin-bottom: var(--sp-4);
  color: var(--text-faint); }
.empty-state h3 { font-size: var(--text-lg); font-weight: 600; color: var(--text);
  margin-bottom: var(--sp-2); }
.empty-state p { font-size: var(--text-sm); max-width: 40ch; margin-bottom: var(--sp-6); }
```

## Anti-Pattern Quick Reference

| ❌ Never | ✅ Always |
|----------|-----------|
| `color: #333` / `color: black` | `color: var(--text)` |
| `padding: 15px` / `margin: 7px` | `padding: var(--sp-4)` (nearest scale value) |
| `border-radius: 5px` | `border-radius: var(--radius-sm)` |
| `box-shadow: 0 0 10px black` | `box-shadow: var(--shadow-sm)` |
| `transition: all 0.3s` | `transition: background var(--duration) var(--ease)` |
| Emoji as icons (✅ ❌ ⚠️) | SVG icons (Lucide/Heroicons) |
| Only default state | hover + focus-visible + disabled + loading |
| `font-size: 13px` arbitrary | `font-size: var(--text-sm)` from scale |
| Text spanning full width | `max-width: 65ch` on text containers |
| `<div onclick>` | `<button>` with proper semantics |
