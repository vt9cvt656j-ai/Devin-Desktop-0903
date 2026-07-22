# Modern CSS Effects & Styling

This file teaches the ACTUAL CSS that makes UIs look premium instead of flat and amateur. Copy the code, use the exact values, avoid the documented pitfalls. The golden rule running through everything: **premium UIs are restrained**. Low-opacity borders, muted gradients, layered soft shadows, consistent radii, no pure black. Loud is amateur; subtle is professional.

---

## Gradients done right

The amateur tell is the rainbow: `linear-gradient(red, blue)` or `45deg, #ff0000, #00ff00`. Real product gradients are MUTED — two hues close on the color wheel, low saturation, often near-monochrome.

### Linear, radial, conic syntax

```css
/* Linear: angle, then color stops. 135deg = top-left to bottom-right. */
background: linear-gradient(135deg, #6366f1, #8b5cf6);

/* Radial: shape size at position, then stops */
background: radial-gradient(circle at 30% 20%, #1e293b, #0f172a);
background: radial-gradient(ellipse 80% 50% at 50% 0%, #312e81, transparent);

/* Conic: sweeps around a center point (great for rings/loaders) */
background: conic-gradient(from 180deg at 50% 50%, #6366f1, #8b5cf6, #6366f1);
```

### Multi-stop with explicit positions

```css
/* Position each stop in % to control where the blend happens.
   Double-position a color (0%, 60%) to hold it flat then transition. */
background: linear-gradient(
  180deg,
  #0f172a 0%,
  #0f172a 60%,
  #1e293b 100%
);
```

### The "mesh gradient" look (layered radial-gradients)

This is the single highest-impact background trick. Stack several large, soft, semi-transparent radial-gradients at different corners over a base color. The blobs blend into an organic, expensive-looking surface.

```css
.mesh-bg {
  background-color: #0f172a; /* base sets the mood */
  background-image:
    radial-gradient(at 20% 25%, hsla(253, 70%, 60%, 0.30) 0px, transparent 50%),
    radial-gradient(at 80% 10%, hsla(190, 70%, 55%, 0.25) 0px, transparent 50%),
    radial-gradient(at 75% 75%, hsla(320, 70%, 60%, 0.22) 0px, transparent 50%),
    radial-gradient(at 10% 90%, hsla(220, 80%, 60%, 0.25) 0px, transparent 50%);
}
```

Keep every blob's alpha ≤ 0.30 and the base color dark (or very light). The transparency is what makes them merge.

### Subtle background gradients (not garish)

For a card or section, you want a gradient you can BARELY see — it adds depth without screaming.

```css
/* Near-monochrome: same hue, two close lightness values */
.card {
  background: linear-gradient(180deg, #1e293b, #172033);
}

/* Light mode: white to a faint cool tint */
.panel {
  background: linear-gradient(180deg, #ffffff, #f8fafc);
}

/* A faint top sheen (the "glassy highlight" many premium cards have) */
.sheen {
  background:
    linear-gradient(180deg, rgba(255,255,255,0.06), transparent 40%),
    #18181b;
}
```

### Gradient text (`background-clip: text`)

```css
.gradient-text {
  background: linear-gradient(135deg, #818cf8, #c084fc);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent; /* fallback if text-fill unsupported */
}
```

Pitfall: you MUST keep both `-webkit-background-clip` and the standard property, and set `-webkit-text-fill-color: transparent`. Without `text-fill-color` it stays solid in Safari/Chrome.

### Gradient borders (two reliable techniques)

**A) `border-image`** — simplest, but cannot combine with `border-radius`:

```css
.grad-border {
  border: 2px solid transparent;
  border-image: linear-gradient(135deg, #6366f1, #ec4899) 1;
}
```

**B) The padding-box / `background-origin` trick** — works WITH rounded corners (use this for real UI):

```css
.grad-border-rounded {
  border: 1px solid transparent;
  border-radius: 12px;
  background:
    linear-gradient(#0f172a, #0f172a) padding-box,            /* fills inside */
    linear-gradient(135deg, #6366f1, #ec4899) border-box;      /* shows in border */
  background-origin: border-box;
  background-clip: padding-box, border-box;
}
```

**C) `::before` mask** — for a gradient ring that doesn't repaint the fill:

```css
.ring { position: relative; border-radius: 12px; }
.ring::before {
  content: "";
  position: absolute;
  inset: 0;
  padding: 1px;              /* border thickness */
  border-radius: inherit;
  background: linear-gradient(135deg, #6366f1, #ec4899);
  -webkit-mask:
    linear-gradient(#000 0 0) content-box,
    linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
          mask-composite: exclude;   /* punches out the center */
  pointer-events: none;
}
```

**Common gradient pitfalls**
- Rainbow / fully-saturated stops → amateur. Pick hues within ~60° of each other, drop saturation.
- Forgetting `transparent` actually fades to *transparent black* in some color spaces, giving a gray "dirty" edge. Fade to a same-hue color with `alpha 0` instead: `hsla(253,70%,60%,0)` not `transparent`, when the result looks muddy.
- `border-image` silently kills `border-radius` — use technique B or C for rounded gradient borders.

---

## Shadows that create depth

The #1 amateur mistake is a single harsh shadow: `box-shadow: 0 0 10px black;` or `0 4px 8px rgba(0,0,0,0.5)`. Real shadows in the physical world are **layered and soft**: a tight contact shadow plus progressively larger, fainter, more-offset shadows.

### The layered box-shadow technique

Stack 2–5 shadows. Each successive layer has larger blur, larger y-offset, and LOWER opacity. This approximates how light actually falls off.

```css
/* Soft, realistic elevation — copy this exact value */
.elevated {
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.06),
    0 2px 4px rgba(0, 0, 0, 0.06),
    0 4px 8px rgba(0, 0, 0, 0.06),
    0 8px 16px rgba(0, 0, 0, 0.06),
    0 16px 32px rgba(0, 0, 0, 0.06);
}
```

### Elevation system (sm / md / lg / xl presets)

Define once as tokens; reuse everywhere for consistency.

```css
:root {
  --shadow-xs: 0 1px 2px rgba(16, 24, 40, 0.05);
  --shadow-sm:
    0 1px 3px rgba(16, 24, 40, 0.10),
    0 1px 2px rgba(16, 24, 40, 0.06);
  --shadow-md:
    0 4px 8px -2px rgba(16, 24, 40, 0.10),
    0 2px 4px -2px rgba(16, 24, 40, 0.06);
  --shadow-lg:
    0 12px 16px -4px rgba(16, 24, 40, 0.08),
    0 4px 6px -2px rgba(16, 24, 40, 0.03);
  --shadow-xl:
    0 20px 24px -4px rgba(16, 24, 40, 0.08),
    0 8px 8px -4px rgba(16, 24, 40, 0.03);
  --shadow-2xl: 0 24px 48px -12px rgba(16, 24, 40, 0.18);
}

.card    { box-shadow: var(--shadow-md); }
.modal   { box-shadow: var(--shadow-2xl); }
.dropdown{ box-shadow: var(--shadow-lg); }
```

Note the NEGATIVE spread (`-2px`, `-4px`) on larger layers — it pulls the shadow inward so it doesn't bleed sideways, giving a tighter, more believable falloff.

### Colored shadows (premium accent glow)

Tint the shadow with the element's own color at low alpha — buttons and brand cards feel "lit".

```css
.btn-primary {
  background: #6366f1;
  box-shadow:
    0 1px 2px rgba(99, 102, 241, 0.20),
    0 4px 12px rgba(99, 102, 241, 0.35);   /* indigo glow, not black */
}
.btn-primary:hover {
  box-shadow:
    0 2px 4px rgba(99, 102, 241, 0.25),
    0 8px 24px rgba(99, 102, 241, 0.45);
}
```

### Inner shadows (insets — depth, wells, pressed states)

```css
/* Inset well for inputs / track backgrounds */
.input {
  box-shadow: inset 0 1px 2px rgba(16, 24, 40, 0.08);
}
/* Pressed button */
.btn:active {
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.20);
}
/* Subtle top highlight + bottom shadow = embossed */
.embossed {
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.10),
    inset 0 -1px 0 rgba(0, 0, 0, 0.20);
}
```

### Layering a hairline border into the shadow

A `0 0 0 1px` shadow gives a crisp 1px ring that respects border-radius and stacks with other shadows:

```css
.card {
  box-shadow:
    0 0 0 1px rgba(16, 24, 40, 0.05),   /* hairline ring */
    var(--shadow-md);
}
```

**Common shadow pitfalls**
- Single `0 0 10px black` / high-opacity blobs → flat and ugly. Always layer, always low alpha.
- Pure `black` shadows look dirty. Tint toward your background's hue (e.g. `rgba(16,24,40,…)` for cool UIs).
- Same opacity on every layer → no falloff. DECREASE opacity as blur increases.
- Forgetting dark mode: a dark-on-dark shadow is invisible. In dark themes, lean on borders/inset highlights and stronger, larger shadows (`rgba(0,0,0,0.4–0.6)`).

---

## Glassmorphism

Frosted-glass panels: a blurred view of whatever is behind, plus a translucent fill, a faint border, and a soft shadow. The blur is the whole effect — without `backdrop-filter` it's just a transparent box.

```css
.glass {
  background: rgba(255, 255, 255, 0.08);          /* translucent fill */
  -webkit-backdrop-filter: blur(16px) saturate(160%);
          backdrop-filter: blur(16px) saturate(160%);
  border: 1px solid rgba(255, 255, 255, 0.15);    /* catches the light */
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

/* Light-on-dark variant often adds a top sheen */
.glass-sheen {
  background:
    linear-gradient(180deg, rgba(255,255,255,0.12), rgba(255,255,255,0.04));
  -webkit-backdrop-filter: blur(20px);
          backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.12);
}
```

`saturate(160%)` punches up the colors bleeding through — it's what separates "real" glass from a plain blur.

**Fallback** for browsers without `backdrop-filter` (rare now, but Firefox lagged): make the fill more opaque so content stays readable.

```css
@supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .glass { background: rgba(30, 41, 59, 0.85); }
}
```

**Common glass pitfalls**
- Omitting the `-webkit-` prefix → no blur in Safari (where users most expect it).
- Glass over a flat solid background = invisible. It NEEDS something colorful/varied behind it (image, mesh gradient, content).
- Over-blurring text behind it hurts contrast/accessibility — keep panel content high-contrast and don't put body text directly on heavy glass.
- `backdrop-filter` needs the element to be its own stacking context; if it does nothing, ensure no ancestor `overflow`/`filter` is clipping it.

---

## Modern CSS layout you must know

### Flexbox patterns

```css
/* Perfect centering */
.center { display: flex; align-items: center; justify-content: center; }

/* Space-between bar (logo left, actions right) */
.navbar { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }

/* Sticky footer: footer sits at bottom even on short pages */
body { min-height: 100vh; display: flex; flex-direction: column; }
main { flex: 1; }        /* main grows to fill, pushing footer down */

/* Equal-width cards in a row that wrap */
.row { display: flex; flex-wrap: wrap; gap: 1rem; }
.row > * { flex: 1 1 280px; }   /* grow, shrink, 280px ideal basis */

/* Push one item to the end */
.toolbar { display: flex; gap: .5rem; }
.toolbar .spacer { margin-inline-start: auto; }
```

### CSS Grid

```css
/* Responsive auto-fit cards — the single most useful grid line.
   Cards are ≥250px, fill the row, wrap automatically. No media queries. */
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1.5rem;
}
```

Use `auto-fill` instead of `auto-fit` when you want empty tracks to remain (so a lone card stays its min width instead of stretching full-width).

```css
/* Holy-grail layout: header, footer, sidebar, content */
.layout {
  display: grid;
  grid-template:
    "header  header"  auto
    "sidebar main"    1fr
    "footer  footer"  auto
    / 240px 1fr;
  min-height: 100vh;
}
.layout > header  { grid-area: header; }
.layout > nav     { grid-area: sidebar; }
.layout > main    { grid-area: main; }
.layout > footer  { grid-area: footer; }
```

```css
/* Subgrid: child inherits parent's columns so nested items align across cards */
.cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; }
.card  {
  display: grid;
  grid-template-rows: subgrid;   /* title/body/footer rows line up across all cards */
  grid-row: span 3;
}
```

### The `gap` property

Use `gap` for spacing between flex/grid children instead of margins — it never adds outer margin and never collapses. Works on flexbox too (not just grid).

```css
.stack { display: flex; flex-direction: column; gap: 12px; }
.grid  { display: grid; gap: 24px 16px; }  /* row-gap column-gap */
```

**Common layout pitfalls**
- Reaching for `float` or absolute-positioning hacks for layout → use flex/grid.
- Margins between flex items instead of `gap` → collapsing/edge-margin bugs.
- `100vw` causes horizontal scroll when a scrollbar is present (it ignores the scrollbar width). Prefer `100%` or `100dvw`.
- Fixed-height containers that clip content → let content define height; use `min-height`.

---

## Modern CSS features (2024–2026)

### `:has()` — the parent/previous-sibling selector

```css
/* Card that CONTAINS an image gets different padding */
.card:has(img) { padding-top: 0; }

/* Style a label when its checkbox is checked (no JS) */
.option:has(input:checked) { border-color: #6366f1; background: #eef2ff; }

/* Form field with an invalid input shows red */
.field:has(input:invalid:not(:placeholder-shown)) { color: #dc2626; }

/* Dim the page when any <dialog open> exists */
body:has(dialog[open]) { overflow: hidden; }
```

### Container queries (`@container`) — style by PARENT width, not viewport

```css
.card-wrap { container-type: inline-size; }   /* declare the container */

@container (min-width: 400px) {
  .card { display: grid; grid-template-columns: 120px 1fr; gap: 1rem; }
}
```

This makes a component responsive to wherever it's placed — the same card adapts in a wide main area vs a narrow sidebar. Named: `container: card / inline-size;` then `@container card (...)`.

### CSS nesting (native, no preprocessor)

```css
.btn {
  background: #6366f1;
  color: #fff;

  &:hover { background: #4f46e5; }
  &:focus-visible { outline: 2px solid #6366f1; outline-offset: 2px; }

  .icon { width: 1em; height: 1em; }   /* descendant */

  @media (min-width: 768px) { padding: .75rem 1.5rem; }
}
```

Pitfall: a bare element selector inside a rule (`span { }`) is treated as a nested descendant — fine — but to chain onto the parent you need `&` (`&.active`, `&:hover`).

### `:is()` / `:where()` — group selectors; `:where()` has ZERO specificity

```css
/* Without :is(), you'd repeat the prefix three times */
:is(h1, h2, h3) { line-height: 1.2; }

/* :where() = same grouping but 0 specificity, so it's trivially overridable.
   Ideal for resets and base styles you never want to fight later. */
:where(ul, ol) { margin: 0; padding: 0; }
```

### Logical properties (RTL-safe, axis-based)

```css
/* Instead of margin-left/right/top/bottom: */
.box {
  margin-inline: auto;        /* left+right */
  margin-block: 1rem;         /* top+bottom */
  padding-inline-start: 1rem; /* left in LTR, right in RTL */
  border-inline-start: 2px solid #6366f1;
  inset-inline-start: 0;
}
```

### `clamp()` for fluid type & spacing

`clamp(MIN, PREFERRED, MAX)` — the value scales with the viewport but never exits the bounds. Kills most typography media queries.

```css
/* Fluid heading: 1.5rem floor, grows with viewport, 3rem ceiling */
h1 { font-size: clamp(1.5rem, 1rem + 3vw, 3rem); }

/* Fluid section padding */
.section { padding-block: clamp(2rem, 5vw, 6rem); }

/* Fluid body with a sensible middle term */
body { font-size: clamp(1rem, 2vw + 1rem, 1.25rem); }

/* Constrain a content column responsively */
.prose { width: clamp(45ch, 60%, 75ch); margin-inline: auto; }
```

Always include a `rem` term in the middle (`1rem + 3vw`) so zoom/accessibility still scales the text; a pure `vw` value ignores user font-size settings.

### `aspect-ratio`

```css
.video    { aspect-ratio: 16 / 9; width: 100%; }
.avatar   { aspect-ratio: 1; width: 48px; border-radius: 50%; object-fit: cover; }
.card-img { aspect-ratio: 4 / 3; object-fit: cover; width: 100%; }
```

### `inset` shorthand

```css
.overlay { position: absolute; inset: 0; }              /* top/right/bottom/left: 0 */
.badge   { position: absolute; inset: 8px 8px auto auto; } /* top-right corner */
```

### `accent-color` — theme native form controls in one line

```css
:root { accent-color: #6366f1; }   /* checkboxes, radios, range, progress */
```

### `color-mix()` — derive colors at runtime

```css
:root { --brand: #6366f1; }

.btn        { background: var(--brand); }
.btn:hover  { background: color-mix(in srgb, var(--brand) 85%, black); } /* darken 15% */
.btn-subtle { background: color-mix(in srgb, var(--brand) 12%, white); } /* tint */
.border     { border-color: color-mix(in srgb, var(--brand) 30%, transparent); }
```

This is huge for theming: define ONE brand color, derive hover/active/tint/border variants without hand-picking hex codes.

---

## Theming with custom properties

Build two layers: **primitive** tokens (raw palette) and **semantic** tokens (roles like `--bg`, `--text`, `--border`). Components reference only semantic tokens, so theming = reassigning semantics.

```css
:root {
  /* --- primitives (never used directly in components) --- */
  --slate-50: #f8fafc;  --slate-100:#f1f5f9; --slate-200:#e2e8f0;
  --slate-700:#334155;  --slate-800:#1e293b; --slate-900:#0f172a;
  --indigo-500:#6366f1; --indigo-600:#4f46e5;

  /* --- semantic tokens (components use THESE) --- */
  --color-bg:           var(--slate-50);
  --color-surface:      #ffffff;
  --color-text:         var(--slate-900);
  --color-text-muted:   var(--slate-700);
  --color-border:       rgba(15, 23, 42, 0.10);
  --color-primary:      var(--indigo-500);
  --color-primary-hover:var(--indigo-600);

  /* shared scale tokens */
  --radius: 12px;
  --radius-sm: 8px;
  --space: 1rem;
  --shadow-md: 0 4px 8px -2px rgba(16,24,40,.10), 0 2px 4px -2px rgba(16,24,40,.06);
}

/* Dark mode: ONLY remap semantics — components don't change */
[data-theme="dark"] {
  --color-bg:         var(--slate-900);
  --color-surface:    var(--slate-800);
  --color-text:       var(--slate-100);
  --color-text-muted: var(--slate-200);
  --color-border:     rgba(255, 255, 255, 0.10);
}

/* System-preference fallback when no explicit theme is set */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --color-bg: var(--slate-900);
    --color-surface: var(--slate-800);
    --color-text: var(--slate-100);
    --color-border: rgba(255,255,255,0.10);
  }
}

/* Component reads only semantic tokens */
.card {
  background: var(--color-surface);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: var(--shadow-md);
  padding: var(--space);
}
```

**Runtime theming** (e.g. a brand-color picker): set a property on `:root` from JS and everything derived via `color-mix`/`var` updates instantly.

```js
document.documentElement.style.setProperty('--color-primary', userColor);
document.documentElement.dataset.theme = 'dark';
```

**Common theming pitfalls**
- Hardcoding hex in components instead of semantic tokens → can't theme.
- Toggling `prefers-color-scheme` AND `[data-theme]` without a precedence rule → flicker. Let `[data-theme]` win; use `:not([data-theme])` on the media query as above.
- Forgetting `color-scheme: light dark;` on `:root` — set it so native widgets/scrollbars match the theme:
  ```css
  :root { color-scheme: light dark; }
  ```

---

## clip-path & masks

### `clip-path` — clip an element to a shape

```css
/* Angled section divider (diagonal cut) */
.hero { clip-path: polygon(0 0, 100% 0, 100% 92%, 0 100%); }

/* Hexagon / badge */
.hex { clip-path: polygon(50% 0, 100% 25%, 100% 75%, 50% 100%, 0 75%, 0 25%); }

/* Circle reveal (also animatable) */
.dot { clip-path: circle(50%); }

/* Rounded inset (modern alternative to overflow clipping) */
.frame { clip-path: inset(0 round 16px); }
```

### Animated reveal with clip-path

```css
.reveal { clip-path: inset(0 100% 0 0); transition: clip-path .6s ease; }
.reveal.is-visible { clip-path: inset(0 0 0 0); }  /* wipes in left→right */
```

### `mask` — show/hide by alpha (fades, image cutouts, icons)

```css
/* Fade the bottom of a scroll area out (gradient mask) */
.fade-bottom {
  -webkit-mask-image: linear-gradient(to bottom, #000 80%, transparent);
          mask-image: linear-gradient(to bottom, #000 80%, transparent);
}

/* Fade both edges of a horizontal carousel */
.edge-fade {
  -webkit-mask-image: linear-gradient(to right, transparent, #000 5%, #000 95%, transparent);
          mask-image: linear-gradient(to right, transparent, #000 5%, #000 95%, transparent);
}

/* Recolor a single-color icon via mask (icon = the mask, bg = the color) */
.icon-mask {
  width: 24px; height: 24px;
  background: currentColor;
  -webkit-mask: url(/icons/star.svg) center / contain no-repeat;
          mask: url(/icons/star.svg) center / contain no-repeat;
}
```

**Pitfalls**
- Always ship the `-webkit-mask*` prefix alongside `mask*`.
- `clip-path: inset(... round Npx)` clips children to rounded corners WITHOUT `overflow:hidden`, which is handy when you also need a shadow (overflow hidden would crop the shadow).
- Mask gradients use ALPHA: `#000` = visible, `transparent` = hidden. Colors don't matter, only opacity.

---

## Filters & blend modes

### `filter` — direct pixel effects (great for hover)

```css
/* Subtle image hover: brighten + saturate */
.thumb { transition: filter .25s ease; }
.thumb:hover { filter: brightness(1.05) saturate(1.15); }

/* Grayscale → color on hover (logos, portraits) */
.logo { filter: grayscale(1); opacity: .7; transition: .3s; }
.logo:hover { filter: grayscale(0); opacity: 1; }

/* Soft glow on an element */
.glow { filter: drop-shadow(0 0 12px rgba(99,102,241,.6)); }
```

`drop-shadow()` (unlike `box-shadow`) follows the element's actual alpha shape — use it for shadows on PNGs/SVGs/clip-paths.

### `backdrop-filter` beyond glass

```css
/* Dim + blur a page behind a modal */
.scrim {
  position: fixed; inset: 0;
  background: rgba(0,0,0,0.4);
  -webkit-backdrop-filter: blur(4px);
          backdrop-filter: blur(4px);
}
```

### `mix-blend-mode` — overlays, duotone, text over images

```css
/* Text that inverts against any background underneath */
.cutout-text { color: #fff; mix-blend-mode: difference; }

/* Duotone image: color layer multiplied over a grayscale photo */
.duotone { position: relative; }
.duotone img { filter: grayscale(1) contrast(1.1); }
.duotone::after {
  content: ""; position: absolute; inset: 0;
  background: linear-gradient(135deg, #6366f1, #ec4899);
  mix-blend-mode: screen;   /* or 'multiply' for darker duotone */
  pointer-events: none;
}

/* Colored highlight that interacts with content below */
.highlight { background: #fde047; mix-blend-mode: multiply; }
```

**Pitfalls**
- `filter: blur()` on an element blurs the element ITSELF; to blur what's behind it use `backdrop-filter`.
- `mix-blend-mode` needs an isolated stacking context to not bleed into the whole page — wrap the group with `isolation: isolate;`.
- Heavy/large `blur()` is GPU-expensive; keep radii reasonable and avoid animating blur on big surfaces.

---

## Borders & dividers that look pro

The amateur tell is `border: 1px solid #000` or `#ccc`. Pros use **low-opacity colors** so the border reads as a hairline that adapts to the background.

```css
:root {
  --border: rgba(15, 23, 42, 0.10);          /* light theme hairline */
  --border-strong: rgba(15, 23, 42, 0.16);
}
[data-theme="dark"] {
  --border: rgba(255, 255, 255, 0.10);        /* dark theme hairline */
  --border-strong: rgba(255, 255, 255, 0.16);
}

.card    { border: 1px solid var(--border); }
.divider { height: 1px; background: var(--border); border: 0; }
```

### Gradient / fading dividers (premium separators)

```css
/* Horizontal rule that fades at both ends */
.hr-fade {
  height: 1px; border: 0;
  background: linear-gradient(90deg, transparent, var(--border-strong), transparent);
}
```

### Crisp sub-pixel borders on hi-DPI

A true 1px border can look heavy; render a hairline with a box-shadow ring (also respects radius):

```css
.hairline { box-shadow: 0 0 0 1px var(--border); border-radius: 12px; }
```

### Focus rings — modern, accessible

Use `:focus-visible` (keyboard focus only, not mouse clicks) + `outline` + `outline-offset`. NEVER `outline: none` without a replacement.

```css
.btn:focus-visible,
.input:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;            /* the gap is what makes it look intentional */
}
```

### Ring via box-shadow (softer, themeable, "focus glow")

```css
.input:focus-visible {
  outline: none;
  box-shadow:
    0 0 0 1px var(--color-primary),               /* solid inner ring */
    0 0 0 4px color-mix(in srgb, var(--color-primary) 25%, transparent); /* soft halo */
}
```

**Pitfalls**
- `outline: none` with no replacement = accessibility failure (keyboard users can't see focus).
- Using `:focus` (not `:focus-visible`) puts a ring on mouse clicks too, which looks broken to mouse users.
- `border` for focus shifts layout (it adds size); prefer `outline`/`box-shadow` which don't reflow. If you must use border, reserve the space with a transparent border at rest.

---

## Premium details

These small touches separate polished from default-browser-styling.

### Smooth scrolling + scroll snapping

```css
html { scroll-behavior: smooth; }
@media (prefers-reduced-motion: reduce) { html { scroll-behavior: auto; } }

/* Offset anchor targets below a sticky header */
:target, [id] { scroll-margin-top: 80px; }

/* Snap a horizontal carousel */
.carousel { display: flex; overflow-x: auto; scroll-snap-type: x mandatory; gap: 1rem; }
.carousel > * { scroll-snap-align: start; flex: 0 0 80%; }
```

### Custom scrollbars (subtle — not chunky)

```css
/* Firefox */
* { scrollbar-width: thin; scrollbar-color: rgba(100,116,139,.4) transparent; }

/* WebKit/Chromium */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
  background: rgba(100,116,139,.4);
  border-radius: 999px;
  border: 2px solid transparent;     /* inset look */
  background-clip: content-box;
}
::-webkit-scrollbar-thumb:hover { background: rgba(100,116,139,.6); }
```

### Selection color

```css
::selection { background: color-mix(in srgb, var(--color-primary) 25%, transparent); color: inherit; }
```

### Text rendering, smoothing, antialiasing

```css
body {
  -webkit-font-smoothing: antialiased;       /* thinner, cleaner text on macOS */
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
  font-feature-settings: "kern", "liga", "calt"; /* kerning + ligatures */
}
/* Tabular numbers for tables/timers so digits don't jitter */
.tabular { font-variant-numeric: tabular-nums; }
```

### Better text wrapping (2024+)

```css
h1, h2, h3 { text-wrap: balance; }   /* even line lengths in headings */
p          { text-wrap: pretty; }    /* avoids orphans/awkward last lines */
```

### `::before` / `::after` decorations

```css
/* Gradient underline that grows on hover */
.link {
  position: relative; text-decoration: none;
}
.link::after {
  content: ""; position: absolute; left: 0; bottom: -2px;
  width: 100%; height: 2px;
  background: linear-gradient(90deg, #6366f1, #ec4899);
  transform: scaleX(0); transform-origin: left;
  transition: transform .3s ease;
}
.link:hover::after { transform: scaleX(1); }

/* "NEW" corner badge via ::before */
.card.is-new::before {
  content: "NEW";
  position: absolute; inset: 8px 8px auto auto;
  font-size: 10px; font-weight: 700; letter-spacing: .05em;
  padding: 2px 6px; border-radius: 999px;
  background: var(--color-primary); color: #fff;
}

/* Decorative top accent line on a card */
.feature::before {
  content: ""; position: absolute; inset: 0 0 auto 0; height: 3px;
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
  border-radius: 12px 12px 0 0;
}
```

### Transitions/hover polish baseline

```css
/* Apply to interactive elements: transition only what changes, with an easing curve */
.btn, .card, .link {
  transition:
    transform .2s cubic-bezier(.4, 0, .2, 1),
    box-shadow .2s cubic-bezier(.4, 0, .2, 1),
    background-color .2s ease;
}
.card:hover { transform: translateY(-2px); box-shadow: var(--shadow-lg); }
.btn:active { transform: translateY(0) scale(.98); }

/* Respect users who don't want motion */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { transition-duration: .01ms !important; animation-duration: .01ms !important; }
}
```

Pitfall: never `transition: all` — it animates unintended properties (and layout props like `width`/`height` cause jank). List the specific properties. Animate `transform`/`opacity` for 60fps; avoid animating `top`/`left`/`width`/`height`/`box-shadow` on large surfaces.

---

## Common amateur CSS tells (and the fix)

A quick audit checklist. If the UI looks "off" or cheap, it's almost always one of these:

| Amateur tell | Why it looks bad | The fix |
|---|---|---|
| `box-shadow: 0 0 10px black` (single, harsh) | flat, no real falloff | layered multi-shadow, low alpha, tinted not black |
| `border: 1px solid #000` / `#ccc` | heavy, dirty edge | `1px solid rgba(15,23,42,0.10)` low-opacity hairline |
| Rainbow / fully-saturated gradients | garish, dated | two close hues, low saturation, muted |
| Inconsistent `border-radius` (4, 6, 9, 12 randomly) | sloppy, unsystematic | one radius scale: `--radius-sm/md/lg`; reuse them |
| Magic numbers (`margin: 13px`, `top: 37px`) | unmaintainable, visually irregular | spacing scale (4/8/12/16/24/32) via tokens |
| `!important` everywhere | specificity war, unoverridable | layered tokens + `:where()` for resets; raise specificity properly |
| Fixed `px` for type/spacing | doesn't scale, ignores zoom | `rem` + `clamp()` for fluid type/spacing |
| No `:hover`/`:focus-visible` states | feels dead/unfinished | transitions + hover lift + visible focus ring |
| `outline: none` with no replacement | inaccessible, no focus indicator | `:focus-visible { outline: 2px solid …; outline-offset: 2px }` |
| Pure `#000` text on `#fff` | harsh, high glare | near-black `#0f172a` / `#18181b` and muted grays |
| `transition: all` | janky, animates layout | name specific props; animate `transform`/`opacity` |
| Solid flat fills everywhere | lifeless | faint same-hue gradient or subtle inner highlight |
| `width: 100vw` | horizontal scrollbar | `100%` / `100dvw` |

**The premium baseline, condensed:** consistent radius scale, spacing scale, low-opacity borders, layered soft shadows, muted gradients, near-black (not pure-black) text, semantic color tokens with dark mode, `:focus-visible` rings, `clamp()` type, and a `transform/opacity` hover on every interactive element. Get those right and a UI reads as professional before you add anything fancy.
