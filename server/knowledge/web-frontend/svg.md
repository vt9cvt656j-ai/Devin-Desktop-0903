# SVG: Icons, Animation & Graphics

Concrete, copy-pasteable SVG. Every block below is real, valid markup you can paste into an HTML file and it works. SVG is just XML drawn with a coordinate system. Master the `viewBox` first — it is the single biggest source of confusion.

---

## viewBox — the coordinate system (read this first)

`viewBox="min-x min-y width height"` defines an **internal coordinate system**. Everything you draw uses those internal units. The `width`/`height` attributes (or CSS) define the **display size** in pixels. These two are completely decoupled — that is the whole point.

```html
<!-- Internal coords go 0..100 on both axes. Displayed at 24x24 px. -->
<svg viewBox="0 0 100 100" width="24" height="24" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" fill="currentColor"/>
</svg>
```

`cx="50" cy="50"` means "center of a 100x100 grid", regardless of whether it renders at 24px or 2400px. The browser scales the grid to fit the box. Origin `(0,0)` is **top-left**; X increases right, Y increases **down** (Y is flipped vs math class).

Why this matters:
- **Resolution independence.** Same markup, any size, always crisp. To resize, change `width`/`height` (or CSS `width`) — never touch the path coordinates.
- **Authoring at a convenient scale.** Draw in a `0 0 100 100` world even if the icon ships at 16px.

```html
<!-- SAME drawing, three display sizes — coordinates never change -->
<svg viewBox="0 0 100 100" width="16"  height="16"><rect x="20" y="20" width="60" height="60" fill="red"/></svg>
<svg viewBox="0 0 100 100" width="64"  height="64"><rect x="20" y="20" width="60" height="60" fill="red"/></svg>
<svg viewBox="0 0 100 100" width="256" height="256"><rect x="20" y="20" width="60" height="60" fill="red"/></svg>
```

`min-x`/`min-y` pan the view. `viewBox="0 0 100 100"` shows the region from (0,0). `viewBox="50 50 100 100"` shows the region starting at (50,50) — i.e. you scrolled right+down by 50 units. Useful for cropping/zooming into part of a drawing.

**width/height vs viewBox — when to set which:**
- Omit `width`/`height` and the SVG fills its container (responsive). Keep `viewBox` so it keeps its aspect ratio and scales. This is the modern default for responsive graphics.
- Set `width`/`height` for a fixed intrinsic size (icons). You can still override with CSS.
- Set CSS `width` only, keep `viewBox`, and height auto-derives from the aspect ratio.

```html
<!-- Responsive: scales to fill parent, keeps 2:1 ratio -->
<svg viewBox="0 0 200 100" xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto">
  <rect width="200" height="100" fill="#4f46e5"/>
</svg>
```

### preserveAspectRatio

Controls how the viewBox fits the viewport when their aspect ratios differ. Format: `preserveAspectRatio="<align> <meetOrSlice>"`. Default is `xMidYMid meet`.

- `meet` — scale to **fit entirely** inside (letterbox; may leave empty space). Like `object-fit: contain`.
- `slice` — scale to **cover** the whole box (crops overflow). Like `object-fit: cover`.
- align — which edge/center to pin: `xMin|xMid|xMax` × `YMin|YMid|YMax`, e.g. `xMidYMid`, `xMinYMin`.
- `none` — **stretch** to fill, ignoring aspect ratio (distorts). Like `object-fit: fill`.

```html
<svg viewBox="0 0 100 100" width="200" height="100" preserveAspectRatio="xMidYMid slice">
  <circle cx="50" cy="50" r="50" fill="teal"/>  <!-- fills, crops left/right -->
</svg>
```

**COMMON PITFALLS**
- No `viewBox` → the icon won't scale; it renders at fixed pixel size and ignores CSS `width`/`height` sizing semantics. **Always include a viewBox.**
- Path coordinates outside the viewBox get clipped (SVG clips to the viewport by default unless `overflow:visible`).
- Y axis points **down**. Forgetting this flips your drawing vertically.
- Mismatched viewBox vs path scale: if your paths use coords up to 1000 but `viewBox="0 0 24 24"`, you see only the tiny top-left corner. The viewBox numbers must match the coordinate range your shapes actually use.

---

## Core shapes & paths

### Basic shapes

```html
<svg viewBox="0 0 120 120" width="240" height="240" xmlns="http://www.w3.org/2000/svg">
  <!-- rect: x,y = top-left corner; rx/ry = corner radius -->
  <rect x="10" y="10" width="40" height="30" rx="6" fill="#3b82f6"/>

  <!-- circle: cx,cy = center; r = radius -->
  <circle cx="90" cy="25" r="18" fill="#ef4444"/>

  <!-- ellipse: rx,ry = horizontal/vertical radii -->
  <ellipse cx="30" cy="80" rx="22" ry="12" fill="#10b981"/>

  <!-- line: from (x1,y1) to (x2,y2). Lines need a stroke (no fill) -->
  <line x1="60" y1="60" x2="110" y2="110" stroke="#111" stroke-width="3"/>

  <!-- polyline: connected points, NOT auto-closed -->
  <polyline points="60,100 75,70 90,100" fill="none" stroke="#a855f7" stroke-width="3"/>

  <!-- polygon: points auto-close back to start (filled triangle) -->
  <polygon points="95,60 115,95 75,95" fill="#f59e0b"/>
</svg>
```

Key stroke properties: `stroke`, `stroke-width`, `stroke-linecap` (`butt|round|square`), `stroke-linejoin` (`miter|round|bevel`), `fill="none"` to draw outline only.

### `<path>` — the d attribute

`<path d="...">` is the universal shape. Commands take coordinate args. **Uppercase = absolute coords, lowercase = relative** to the current point.

| Cmd | Name | Args | Meaning |
|-----|------|------|---------|
| `M`/`m` | Move | `x y` | Lift pen, move to point (start a subpath) |
| `L`/`l` | Line | `x y` | Draw straight line to point |
| `H`/`h` | Horizontal | `x` | Horizontal line to x |
| `V`/`v` | Vertical | `y` | Vertical line to y |
| `C`/`c` | Cubic Bézier | `x1 y1 x2 y2 x y` | Curve with **two** control points |
| `S`/`s` | Smooth cubic | `x2 y2 x y` | Cubic, first control reflected from previous |
| `Q`/`q` | Quadratic Bézier | `x1 y1 x y` | Curve with **one** control point |
| `T`/`t` | Smooth quad | `x y` | Quadratic, control reflected from previous |
| `A`/`a` | Arc | `rx ry rot large-arc sweep x y` | Elliptical arc to point |
| `Z`/`z` | Close | — | Straight line back to subpath start |

```html
<svg viewBox="0 0 100 100" width="200" height="200" xmlns="http://www.w3.org/2000/svg">
  <!-- M: pen to (10,10). L: line to (90,10). L: line to (50,90). Z: close triangle -->
  <path d="M10 10 L90 10 L50 90 Z" fill="#60a5fa"/>
</svg>
```

**Cubic curve (C)** — two control points pull the curve:
```html
<!-- Start (10,80). Control1 (40,10), Control2 (60,10), end (90,80). A smooth hill. -->
<path d="M10 80 C40 10 60 10 90 80" fill="none" stroke="#111" stroke-width="3"/>
```

**Quadratic curve (Q)** — one shared control point:
```html
<!-- Start (10,80). Single control (50,10), end (90,80). -->
<path d="M10 80 Q50 10 90 80" fill="none" stroke="#111" stroke-width="3"/>
```

**Arc (A)** — the confusing one. `A rx ry x-axis-rotation large-arc-flag sweep-flag x y`:
```html
<!-- Half circle: radius 40, no rotation, large-arc=0, sweep=1, end at (90,50) -->
<path d="M10 50 A40 40 0 0 1 90 50" fill="none" stroke="#e11d48" stroke-width="3"/>
```
- `large-arc-flag` (0/1): take the smaller or larger of the two possible arcs.
- `sweep-flag` (0/1): clockwise (1) or counter-clockwise (0).
- Flip either flag to get a different one of the 4 possible arcs between the two points.

### How to read/edit a path

Read it command by command, tracking the "current point". Numbers can be separated by spaces or commas — `M10,10` and `M 10 10` are identical. A command repeats if you give more coordinate sets: `L10 10 20 20 30 30` is three line segments. To **edit**: find the command, change its numbers. To move a whole shape, you generally must shift every absolute coordinate (or wrap it in `<g transform="translate(...)">` instead — much easier than editing every number).

**COMMON PITFALLS**
- Lines/polylines with no `stroke` are invisible (they have no fill area).
- `polyline` does NOT close; `polygon` does. Picking the wrong one leaves a gap or adds an unwanted closing edge.
- Forgetting `fill="none"` on an outline path fills it solid black by default.
- Arc flags are 0/1 single digits and can be written without spaces (`...0 0 1 90 50`), which makes minified paths hard to read — count carefully.

---

## SVG icons the right way

### Delivery method — when to use which

| Method | Use when | Trade-off |
|--------|----------|-----------|
| **Inline `<svg>`** | You need to style/animate it, change color via CSS, or it's a one-off | Bloats HTML if repeated; can't be cached separately |
| **`<img src="icon.svg">`** | Static decorative icon, want caching, don't need CSS control of internals | Can't recolor with `currentColor`/CSS; can't animate internals |
| **SVG sprite** (`<use>`) | Many icons reused across a page/app | One request, one definition, reused everywhere |
| **Icon font** | Legacy systems only | Blurry, a11y issues, hacky — **avoid in new code** |

**Inline** — full control, inherits color:
```html
<button>
  <svg viewBox="0 0 24 24" width="20" height="20" fill="none"
       stroke="currentColor" stroke-width="2" stroke-linecap="round"
       stroke-linejoin="round" aria-hidden="true">
    <path d="M5 12h14M12 5v14"/>  <!-- plus icon -->
  </svg>
  Add item
</button>
```

**Sprite** — define once in a hidden block, reference many times:
```html
<!-- Define once, hidden, near top of <body> -->
<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <defs>
    <symbol id="icon-trash" viewBox="0 0 24 24">
      <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14"
            fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round" stroke-linejoin="round"/>
    </symbol>
  </defs>
</svg>

<!-- Reuse anywhere -->
<svg width="24" height="24" aria-hidden="true"><use href="#icon-trash"/></svg>
<svg width="32" height="32" aria-hidden="true"><use href="#icon-trash"/></svg>
```

### `currentColor` — make icons inherit text color

`currentColor` is a CSS keyword that resolves to the element's computed `color`. Set `fill="currentColor"` (or `stroke="currentColor"`) and the icon follows the surrounding text color automatically — including on hover, in dark mode, etc.

```html
<style>
  .link { color: #2563eb; }
  .link:hover { color: #dc2626; }   /* icon recolors on hover too, for free */
</style>
<a class="link" href="#">
  <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
    <path d="M10 6L8.6 7.4 13.2 12l-4.6 4.6L10 18l6-6z"/>
  </svg>
  Next
</a>
```

### Sizing

```html
<!-- Pixel-fixed -->
<svg viewBox="0 0 24 24" width="24" height="24">…</svg>

<!-- Scales with font-size: 1em square. Great for inline-with-text icons. -->
<svg viewBox="0 0 24 24" width="1em" height="1em" style="font-size:1.25rem">…</svg>

<!-- CSS-controlled -->
<svg viewBox="0 0 24 24" class="icon">…</svg>
<style>.icon { width: 1.5rem; height: 1.5rem; }</style>
```

### `stroke` icons vs `fill` icons

Two design styles. Don't mix conventions for one set.
- **Stroke icons** (Lucide, Feather, Tabler): outlined, drawn with `stroke`, `fill="none"`. Control weight with `stroke-width` (usually 1.5–2). Set `stroke="currentColor"`.
- **Fill icons** (Heroicons solid, Material filled): solid shapes drawn with `fill`. Set `fill="currentColor"`, no stroke.

```html
<!-- Stroke (outline) heart -->
<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M19 5c-1.5-1.5-4-1.5-5.5 0L12 6.5 10.5 5C9-.. "/>
</svg>

<!-- Fill (solid) heart -->
<svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor">
  <path d="M12 21l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.18z"/>
</svg>
```

### Accessibility

- **Decorative** (icon next to a text label, or purely visual): `aria-hidden="true"` so screen readers skip it.
- **Meaningful** (icon-only button conveying info): give it an accessible name.

```html
<!-- Icon-only button: label the BUTTON (cleanest) -->
<button aria-label="Delete">
  <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="currentColor">…</svg>
</button>

<!-- Or label the SVG itself as an image -->
<svg viewBox="0 0 24 24" width="24" height="24" role="img" aria-label="Warning" fill="currentColor">
  <title>Warning</title>   <!-- also surfaces as a tooltip -->
  <path d="M12 2 1 21h22z…"/>
</svg>
```

**COMMON PITFALLS**
- Hardcoding `fill="#000"` instead of `currentColor` → icon ignores theme/hover and stays black in dark mode.
- Decorative icon with no `aria-hidden` → screen reader announces noise, or reads nothing useful.
- Icon-only button with no label → unusable for assistive tech.
- `<use href>` is the modern attr; old `xlink:href` is deprecated but still seen. Use `href`.
- `<img src>` icons can't be recolored with `currentColor` — if you need theming, inline it.

---

## SVG animation with CSS (handles ~80%)

You can target SVG elements with CSS and animate `transform`, `opacity`, `fill`, `stroke`, `stroke-dashoffset`, etc. Prefer `transform` and `opacity` — they're GPU-accelerated and don't trigger layout.

### The #1 SVG rotation gotcha: transform-box + transform-origin

By default an SVG element's `transform-origin` is the **origin of the SVG canvas (0,0)**, NOT the center of the shape. So `transform: rotate(45deg)` spins the element around the top-left of the whole SVG and it flies off-screen. Fix with `transform-box: fill-box`, which makes `transform-origin` relative to the element's own bounding box; then `transform-origin: center` rotates in place.

```html
<svg viewBox="0 0 100 100" width="120" height="120" xmlns="http://www.w3.org/2000/svg">
  <rect class="spin" x="35" y="35" width="30" height="30" fill="#6366f1"/>
</svg>
<style>
  .spin {
    transform-box: fill-box;     /* origin now relative to the rect's own box */
    transform-origin: center;    /* spin around its center, not canvas (0,0) */
    animation: spin 2s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
```

### Hover effects

```html
<svg viewBox="0 0 100 100" width="80" height="80">
  <circle class="btn" cx="50" cy="50" r="30" fill="#0ea5e9"/>
</svg>
<style>
  .btn {
    transform-box: fill-box; transform-origin: center;
    transition: transform .2s ease, fill .2s ease;
  }
  .btn:hover { transform: scale(1.15); fill: #2563eb; }
</style>
```

### Spinning loader (CSS)

```html
<svg viewBox="0 0 50 50" width="40" height="40" class="loader" xmlns="http://www.w3.org/2000/svg">
  <circle cx="25" cy="25" r="20" fill="none" stroke="#e5e7eb" stroke-width="5"/>
  <circle cx="25" cy="25" r="20" fill="none" stroke="#3b82f6" stroke-width="5"
          stroke-linecap="round" stroke-dasharray="90 150"/>
</svg>
<style>
  .loader { transform-box: fill-box; transform-origin: center;
            animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
```

### Pulsing dot

```html
<svg viewBox="0 0 40 40" width="40" height="40">
  <circle class="pulse-ring" cx="20" cy="20" r="8" fill="#22c55e" opacity="0.5"/>
  <circle cx="20" cy="20" r="6" fill="#22c55e"/>
</svg>
<style>
  .pulse-ring {
    transform-box: fill-box; transform-origin: center;
    animation: pulse 1.5s ease-out infinite;
  }
  @keyframes pulse {
    0%   { transform: scale(1);   opacity: .6; }
    100% { transform: scale(2.6); opacity: 0; }
  }
</style>
```

**COMMON PITFALLS**
- Rotation/scale flies off-screen → you forgot `transform-box: fill-box`. This is THE classic SVG bug.
- Animating `x`/`y`/`cx`/`cy`/`width` triggers layout and is janky — animate `transform: translate()`/`scale()` instead.
- `transform-origin: center` without `transform-box: fill-box` still uses the canvas, not the shape.
- Safari historically needed `transform-box` explicitly; always set it, don't rely on defaults.
- CSS transforms on SVG use the SVG user-coordinate system; percentages in `transform-origin` resolve against the bounding box only once `fill-box` is set.

---

## The line-draw animation (stroke-dasharray + stroke-dashoffset)

The famous "self-drawing" effect (signatures, checkmarks, logo reveals). Mechanism:
1. `stroke-dasharray: L` makes one dash exactly as long as the whole path (length `L`), so the path is one continuous dash.
2. `stroke-dashoffset: L` shifts that dash entirely off the start → the line looks erased.
3. Animate `stroke-dashoffset` from `L` to `0` → the stroke appears to draw itself.

### Getting the path length L

Three ways:
- **JS:** `path.getTotalLength()` returns the exact length.
- **`pathLength="1"` trick:** add `pathLength="1"` to the path; now dasharray/offset are normalized to `0..1` regardless of real length. Use `stroke-dasharray: 1; stroke-dashoffset: 1;`. No JS needed — the cleanest approach.
- **Eyeball it:** set dasharray to a number bigger than the path; tweak until it covers fully.

### Pure-CSS draw with the pathLength trick (no JS)

```html
<svg viewBox="0 0 100 100" width="120" height="120" xmlns="http://www.w3.org/2000/svg">
  <!-- checkmark; pathLength=1 normalizes the length to 1 -->
  <path class="draw" d="M20 52 L42 74 L82 28" pathLength="1"
        fill="none" stroke="#16a34a" stroke-width="8"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>
<style>
  .draw {
    stroke-dasharray: 1;        /* dash = full normalized length */
    stroke-dashoffset: 1;       /* pushed fully off → invisible */
    animation: draw 0.8s ease forwards;
  }
  @keyframes draw { to { stroke-dashoffset: 0; } }   /* reveal */
</style>
```

### Draw on hover (transition, not keyframes)

```html
<svg viewBox="0 0 200 60" width="200" height="60">
  <path class="sig" d="M10 40 Q30 5 50 40 T90 40 T130 40" pathLength="1"
        fill="none" stroke="#111" stroke-width="3" stroke-linecap="round"/>
</svg>
<style>
  .sig { stroke-dasharray: 1; stroke-dashoffset: 1;
         transition: stroke-dashoffset 1s ease; }
  svg:hover .sig { stroke-dashoffset: 0; }
</style>
```

### JS version (exact length, programmatic control)

```html
<svg viewBox="0 0 100 100" width="120" height="120">
  <path id="logo" d="M20 80 L20 20 L50 60 L80 20 L80 80"
        fill="none" stroke="#7c3aed" stroke-width="6" stroke-linecap="round"/>
</svg>
<script>
  const p = document.getElementById('logo');
  const len = p.getTotalLength();           // exact path length
  p.style.strokeDasharray  = len;
  p.style.strokeDashoffset = len;           // hidden
  p.getBoundingClientRect();                // force reflow so transition runs
  p.style.transition = 'stroke-dashoffset 1.5s ease';
  p.style.strokeDashoffset = '0';           // draw
</script>
```

**COMMON PITFALLS**
- Nothing draws → the element has no `stroke` (dash effects only apply to strokes, not fills).
- Partial draw / overshoot → dasharray length doesn't match real path length. Use `pathLength="1"` to dodge this entirely.
- Multiple subpaths (`M...M...`) make `getTotalLength` cover all of them; dashes span the gaps. Either animate each subpath separately or accept the combined length.
- Closed shapes (`Z`) draw fine but the "start" point is where `M` is — the reveal direction follows path order. Reorder points or reverse the path to change direction.
- Forgetting `animation-fill-mode: forwards` (`forwards` keyword) → it snaps back to hidden at the end.

---

## SMIL animation (`<animate>` family)

SMIL animates SVG attributes from **inside** the SVG, no CSS/JS. Still useful when you want a self-contained animated SVG file (e.g. an animated icon you drop in as `<img>` and it just animates), or to animate attributes CSS can't easily touch. Widely supported in modern browsers (note: was once deprecation-flagged in Chrome but is supported; CSS/JS is the more future-proof default for app code).

### `<animate>` — animate one attribute

```html
<svg viewBox="0 0 100 100" width="100" height="100" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="10" fill="#ef4444">
    <animate attributeName="r" values="10;40;10" dur="2s" repeatCount="indefinite"/>
    <animate attributeName="opacity" values="1;0;1" dur="2s" repeatCount="indefinite"/>
  </circle>
</svg>
```

### `<animateTransform>` — rotate/scale/translate

```html
<svg viewBox="0 0 100 100" width="100" height="100" xmlns="http://www.w3.org/2000/svg">
  <rect x="40" y="40" width="20" height="20" fill="#0ea5e9">
    <!-- rotate from 0 to 360 around point (50,50). 'from'/'to' include the pivot. -->
    <animateTransform attributeName="transform" type="rotate"
      from="0 50 50" to="360 50 50" dur="1.5s" repeatCount="indefinite"/>
  </rect>
</svg>
```
Note: SMIL `rotate` takes the pivot as part of the value (`angle cx cy`) — no `transform-box` headache, unlike CSS.

### `<animateMotion>` — move along a path

```html
<svg viewBox="0 0 200 100" width="200" height="100" xmlns="http://www.w3.org/2000/svg">
  <path id="track" d="M10 50 Q100 0 190 50" fill="none" stroke="#ddd"/>
  <circle r="6" fill="#16a34a">
    <!-- rotate="auto" turns the object to face its direction of travel -->
    <animateMotion dur="3s" repeatCount="indefinite" rotate="auto">
      <mpath href="#track"/>   <!-- follow the path above -->
    </animateMotion>
  </circle>
</svg>
```

**Trigger on click/hover** with `begin`: `<animate ... begin="click"/>` or `begin="someId.mouseover"`.

**CSS/JS alternative:** CSS `offset-path: path('...')` + `offset-distance` animation does motion-along-path without SMIL:
```css
.dot { offset-path: path('M10 50 Q100 0 190 50');
       animation: move 3s linear infinite; }
@keyframes move { to { offset-distance: 100%; } }
```

**COMMON PITFALLS**
- SMIL only runs when the SVG is inline or loaded as `<img>`/`<object>`; it does NOT run for SVG set as a CSS `background-image` in some engines.
- `repeatCount="indefinite"` (not "infinite" — that's CSS). Mixing the keywords is a classic mistake.
- `<mpath>` must reference an existing path id; inline `path="..."` on `<animateMotion>` also works.
- SMIL and CSS animations on the same attribute can conflict; pick one.

---

## SVG with GSAP

GSAP is the go-to JS library for complex, sequenced, interactive SVG animation. Core tweens plus SVG plugins (DrawSVG, MorphSVG, MotionPath). Assume `gsap` is loaded and (for plugins) registered.

### DrawSVG — line draw, the easy way

```html
<svg viewBox="0 0 100 100" width="120" height="120">
  <path id="check" d="M20 52 L42 74 L82 28" fill="none"
        stroke="#16a34a" stroke-width="8" stroke-linecap="round"/>
</svg>
<script>
  gsap.registerPlugin(DrawSVGPlugin);
  // animate the visible portion from 0% to 100% — no manual dasharray math
  gsap.from("#check", { duration: 1, drawSVG: "0%", ease: "power1.inOut" });
</script>
```

### MorphSVG — morph one shape's path into another

```html
<svg viewBox="0 0 100 100" width="160" height="160">
  <path id="start" d="M50 10 L90 90 L10 90 Z" fill="#f59e0b"/>  <!-- triangle -->
  <path id="end"   d="M10 50 A40 40 0 1 1 90 50 A40 40 0 1 1 10 50" style="visibility:hidden"/>
</svg>
<script>
  gsap.registerPlugin(MorphSVGPlugin);
  // tween #start's d-attribute into the shape of #end (paths can have different point counts)
  gsap.to("#start", { duration: 1.5, morphSVG: "#end", repeat: -1, yoyo: true });
</script>
```
MorphSVG handles differing point counts by remapping; you can also pass a raw path string: `morphSVG: "M..."`.

### MotionPath — move/orient along a path

```html
<svg viewBox="0 0 300 150" width="300" height="150">
  <path id="route" d="M20 130 C80 10 220 10 280 130" fill="none" stroke="#eee"/>
  <circle id="ball" r="8" fill="#3b82f6"/>
</svg>
<script>
  gsap.registerPlugin(MotionPathPlugin);
  gsap.to("#ball", {
    duration: 3, repeat: -1, ease: "none",
    motionPath: { path: "#route", align: "#route", autoRotate: true,
                  alignOrigin: [0.5, 0.5] }   // center the ball on the path
  });
</script>
```

### General transform tween (note transformOrigin)

```html
<script>
  // GSAP handles the SVG transform-origin quirk for you; just name the origin.
  gsap.to("#gear", { rotation: 360, duration: 2, repeat: -1, ease: "none",
                     transformOrigin: "50% 50%" });
</script>
```

**COMMON PITFALLS**
- Forgetting `gsap.registerPlugin(...)` → plugin tween silently does nothing.
- DrawSVG/MorphSVG are premium GSAP plugins (Club GreenSock) — confirm availability before relying on them; DrawSVG can be hand-rolled with the dasharray technique above if unavailable.
- For SVG rotation, use GSAP's `rotation` + `transformOrigin` (GSAP normalizes the SVG origin issue), not a CSS class.
- `motionPath.align` must point to the same path (or an element) to actually follow its position, not just its shape.

---

## Gradients & filters inside SVG

Define reusable paint/effects in `<defs>` and reference by `id`.

### Linear gradient

```html
<svg viewBox="0 0 200 100" width="200" height="100" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <!-- x1,y1 -> x2,y2 sets the gradient direction (default left->right) -->
    <linearGradient id="grad" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%"   stop-color="#6366f1"/>
      <stop offset="100%" stop-color="#ec4899"/>
    </linearGradient>
  </defs>
  <rect width="200" height="100" rx="12" fill="url(#grad)"/>
</svg>
```

### Radial gradient

```html
<svg viewBox="0 0 120 120" width="120" height="120" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <radialGradient id="rg" cx="50%" cy="40%" r="60%">
      <stop offset="0%"   stop-color="#fff"/>
      <stop offset="60%"  stop-color="#f59e0b"/>
      <stop offset="100%" stop-color="#b45309"/>
    </radialGradient>
  </defs>
  <circle cx="60" cy="60" r="55" fill="url(#rg)"/>  <!-- glossy sphere -->
</svg>
```

### Soft drop shadow — `feDropShadow` (the easy one)

```html
<svg viewBox="0 0 120 120" width="120" height="120" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="shadow" x="-50%" y="-50%" width="200%" height="200%">
      <!-- dx,dy = offset; stdDeviation = blur; flood-* = shadow color/alpha -->
      <feDropShadow dx="0" dy="4" stdDeviation="4"
                    flood-color="#000" flood-opacity="0.3"/>
    </filter>
  </defs>
  <rect x="30" y="30" width="60" height="60" rx="10" fill="#3b82f6" filter="url(#shadow)"/>
</svg>
```
**Important:** filters clip to the filter region. Default region is `-10%..110%`; a blur/offset can get cut off. Expand it with `x="-50%" y="-50%" width="200%" height="200%"` as above.

### Manual soft shadow + glow (feGaussianBlur)

```html
<svg viewBox="0 0 120 120" width="120" height="120" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="5" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>          <!-- blurred copy underneath -->
        <feMergeNode in="SourceGraphic"/> <!-- crisp shape on top -->
      </feMerge>
    </filter>
  </defs>
  <circle cx="60" cy="60" r="25" fill="#22d3ee" filter="url(#glow)"/>
</svg>
```

### Blur for a glow that pulses (filter + CSS)

```html
<style>
  .neon { filter: drop-shadow(0 0 6px #22d3ee); animation: throb 1.2s ease-in-out infinite alternate; }
  @keyframes throb { to { filter: drop-shadow(0 0 16px #22d3ee); } }
</style>
<svg viewBox="0 0 100 100" width="100" height="100">
  <circle class="neon" cx="50" cy="50" r="20" fill="#0e7490"/>
</svg>
```
CSS `filter: drop-shadow()` is often simpler than an SVG `<filter>` for a quick shadow/glow and respects the shape's alpha (unlike `box-shadow`).

**COMMON PITFALLS**
- Gradient/filter `id` must be unique per document; duplicate ids → only the first applies. Prefix ids when inlining many SVGs (`grad-card1`).
- `fill="url(#grad)"` needs the `#`; `fill="grad"` does nothing.
- Filter output getting clipped → expand the filter region (`x/y/width/height` on `<filter>`).
- Gradient coords default to `objectBoundingBox` units (0–1). To use pixel coords add `gradientUnits="userSpaceOnUse"`.
- Heavy filters (large blur) are expensive — fine for a few elements, costly if animated on many.

---

## Practical recipes (copy-paste)

### 1. Loading spinner (arc + rotation)

```html
<svg viewBox="0 0 50 50" width="44" height="44" class="spinner" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Loading">
  <circle cx="25" cy="25" r="20" fill="none" stroke="#e5e7eb" stroke-width="5"/>
  <path d="M25 5 a20 20 0 0 1 20 20" fill="none" stroke="#3b82f6" stroke-width="5" stroke-linecap="round"/>
</svg>
<style>
  .spinner { transform-box: fill-box; transform-origin: center;
             animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
```

### 2. Hamburger → X (animated toggle)

Three lines; toggle a class to morph into an X. Top/bottom lines rotate and meet; middle fades.

```html
<button class="burger" aria-label="Menu" aria-expanded="false" onclick="this.classList.toggle('open');
        this.setAttribute('aria-expanded', this.classList.contains('open'))">
  <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <line class="top"    x1="3" y1="6"  x2="21" y2="6"/>
    <line class="mid"    x1="3" y1="12" x2="21" y2="12"/>
    <line class="bottom" x1="3" y1="18" x2="21" y2="18"/>
  </svg>
</button>
<style>
  .burger line { transform-box: fill-box; transform-origin: center;
                 transition: transform .3s ease, opacity .2s ease; }
  .burger.open .top    { transform: translateY(6px)  rotate(45deg); }
  .burger.open .mid    { opacity: 0; }
  .burger.open .bottom { transform: translateY(-6px) rotate(-45deg); }
</style>
```
(Note: `transform-box: fill-box` makes each line rotate around its own center; the translate brings top & bottom to the middle, then they cross into an X.)

### 3. Success checkmark (circle draws, then check draws)

```html
<svg viewBox="0 0 52 52" width="80" height="80" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Success">
  <circle class="ok-circle" cx="26" cy="26" r="24" fill="none" stroke="#16a34a" stroke-width="3" pathLength="1"/>
  <path class="ok-check" d="M14 27 L23 36 L39 18" fill="none" stroke="#16a34a" stroke-width="4"
        stroke-linecap="round" stroke-linejoin="round" pathLength="1"/>
</svg>
<style>
  .ok-circle { stroke-dasharray:1; stroke-dashoffset:1; animation: draw .5s ease forwards; }
  .ok-check  { stroke-dasharray:1; stroke-dashoffset:1; animation: draw .3s .45s ease forwards; }
  @keyframes draw { to { stroke-dashoffset: 0; } }
</style>
```

### 4. Progress ring (circular progress with stroke-dasharray)

The dash length = circumference `2πr`. For `r=45`, circumference ≈ `282.7`. `stroke-dashoffset` from full→0 fills the ring. Set offset = `circumference * (1 - percent/100)`.

```html
<svg viewBox="0 0 100 100" width="120" height="120" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="65 percent">
  <circle cx="50" cy="50" r="45" fill="none" stroke="#e5e7eb" stroke-width="8"/>
  <circle id="ring" cx="50" cy="50" r="45" fill="none" stroke="#3b82f6" stroke-width="8"
          stroke-linecap="round"
          transform="rotate(-90 50 50)"
          stroke-dasharray="282.7" stroke-dashoffset="282.7"/>
</svg>
<script>
  // rotate(-90 50 50) starts the fill at 12 o'clock instead of 3 o'clock
  const C = 2 * Math.PI * 45;            // ≈ 282.74
  function setProgress(pct) {
    document.getElementById('ring').style.strokeDashoffset = C * (1 - pct/100);
  }
  document.getElementById('ring').style.transition = 'stroke-dashoffset .6s ease';
  setProgress(65);
</script>
```
Pure-CSS variant with `pathLength="100"` so the offset equals the remaining percent directly:
```html
<circle r="45" cx="50" cy="50" pathLength="100" stroke-dasharray="100"
        stroke-dashoffset="35" transform="rotate(-90 50 50)" fill="none" stroke="#3b82f6" stroke-width="8"/>
<!-- dashoffset 35 == 65% filled. No π math needed. -->
```

### 5. Simple animated logo (draw + fade)

```html
<svg viewBox="0 0 120 40" width="180" height="60" xmlns="http://www.w3.org/2000/svg">
  <path class="mark" d="M10 30 L10 10 L25 25 L40 10 L40 30" pathLength="1"
        fill="none" stroke="#7c3aed" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
  <text class="word" x="50" y="28" font-family="system-ui, sans-serif" font-size="20" font-weight="700" fill="#111">acme</text>
</svg>
<style>
  .mark { stroke-dasharray:1; stroke-dashoffset:1; animation: draw 1s ease forwards; }
  .word { opacity:0; animation: fade .6s .9s ease forwards; }
  @keyframes draw { to { stroke-dashoffset:0; } }
  @keyframes fade { to { opacity:1; } }
</style>
```

### 6. Wavy / blob shape (organic, animated)

A blob is a closed path of cubic curves. Animate by swapping the `d` between two blob shapes via SMIL (works without JS).

```html
<svg viewBox="0 0 200 200" width="200" height="200" xmlns="http://www.w3.org/2000/svg">
  <path fill="#8b5cf6"
        d="M48 -64C62 -53 73 -38 76 -22C79 -5 74 13 65 28C56 43 43 55 27 63C11 70 -8 73 -25 67C-43 61 -59 47 -67 30C-75 13 -74 -7 -67 -24C-60 -41 -47 -55 -31 -64C-16 -73 2 -77 19 -75C36 -73 34 -75 48 -64Z"
        transform="translate(100 100)">
    <animate attributeName="d" dur="6s" repeatCount="indefinite"
      values="M48 -64C62 -53 73 -38 76 -22C79 -5 74 13 65 28C56 43 43 55 27 63C11 70 -8 73 -25 67C-43 61 -59 47 -67 30C-75 13 -74 -7 -67 -24C-60 -41 -47 -55 -31 -64C-16 -73 2 -77 19 -75C36 -73 34 -75 48 -64Z;
              M52 -68C66 -58 73 -39 76 -21C78 -3 76 16 67 31C57 46 41 56 24 64C7 71 -12 75 -29 69C-46 62 -60 45 -68 27C-75 9 -75 -12 -67 -29C-59 -45 -44 -57 -28 -66C-12 -75 6 -80 23 -77C40 -74 38 -78 52 -68Z;
              M48 -64C62 -53 73 -38 76 -22C79 -5 74 13 65 28C56 43 43 55 27 63C11 70 -8 73 -25 67C-43 61 -59 47 -67 30C-75 13 -74 -7 -67 -24C-60 -41 -47 -55 -31 -64C-16 -73 2 -77 19 -75C36 -73 34 -75 48 -64Z"/>
  </path>
</svg>
```
(Generate blob paths from a tool like blobmaker.app; the `transform="translate(100 100)"` recenters paths that use a 0,0-centered coordinate system.)

**Wave (water-line) using a repeating curve animated horizontally:**
```html
<svg viewBox="0 0 200 60" width="200" height="60" xmlns="http://www.w3.org/2000/svg">
  <path class="wave" d="M0 30 Q25 10 50 30 T100 30 T150 30 T200 30 V60 H0 Z" fill="#38bdf8"/>
</svg>
<style>
  .wave { animation: roll 2s linear infinite; }
  @keyframes roll { to { transform: translateX(-50px); } } /* shift one wavelength */
</style>
```

---

## Generating & optimizing SVG

### Where to get icons (don't hand-draw these)
- **Lucide** (lucide.dev) — clean stroke icons, the Feather successor. `stroke="currentColor"`, 24×24, `stroke-width="2"`. Great default.
- **Heroicons** (heroicons.com) — by the Tailwind team; outline + solid variants.
- **Tabler Icons** (tabler.io/icons) — huge set, consistent 24×24 stroke style.
- **Feather** (feathericons.com) — minimal, original stroke set.
- Others: Phosphor, Material Symbols, Bootstrap Icons, Radix Icons.

Copy the SVG, set `stroke`/`fill` to `currentColor`, add `aria-hidden` or a label, drop it inline.

### SVGO — optimize exported SVGs

Designer-exported SVGs (Figma/Illustrator/Sketch) are bloated: editor metadata, comments, redundant groups, absurd coordinate precision, inline styles. Run **SVGO** to strip it. Often cuts file size 50–70%.

```bash
npx svgo input.svg -o output.svg            # single file
npx svgo -f ./icons -o ./icons-optimized    # whole folder
```

What it removes/does: editor metadata & comments, empty groups, hidden elements, default attribute values, collapses transforms, rounds path precision. Online equivalent: SVGOMG (jakearchie.github.io/svgomg). **Watch out:** aggressive SVGO can strip `id`s your CSS/JS targets, or merge paths you wanted separate for animation — exclude those plugins (e.g. `--disable=cleanupIds`) when the SVG is animated.

### Inlining vs external — decision

- **Inline** when you need CSS theming (`currentColor`), animation, or interaction. Cost: HTML weight, no separate cache.
- **External (`<img>`/`<use>` sprite)** for static, repeated, or many icons — cacheable, keeps HTML lean. Sprite when reused across the page.
- **Build-time inlining**: SVGR (`@svgr/webpack`) turns `.svg` into React components: `import Icon from './icon.svg'; <Icon className="..."/>` — best of both in component frameworks.

### Hand-code vs ask `generate_image`

- **Hand-code SVG** when it's geometric/logical: icons, charts, diagrams, progress rings, logos with simple shapes, anything you'll animate or theme. SVG is precise, tiny, scalable, animatable, and recolorable. Default to hand-coding for UI graphics.
- **Ask `generate_image`** for rich, painterly, photorealistic, or highly detailed illustrations (a detailed mascot, a textured hero scene, complex organic art) where authoring path data by hand is impractical. Raster output isn't scalable/animatable the same way — request a transparent PNG and place it with `<img>`. Heuristic: if you'd be writing hundreds of hand-tuned bezier points to fake a painting, generate it instead; if it's lines, circles, and text, code it.

---

## Common amateur SVG mistakes (checklist)

1. **Hardcoded `fill="#000"` instead of `currentColor`.** Icon won't follow text color, breaks in dark mode and on hover. Use `fill="currentColor"` / `stroke="currentColor"`.
2. **Wrong or missing `viewBox`.** Without it the icon ignores scaling and renders at a fixed size; with a viewBox that doesn't match the coordinate range, you see a cropped fragment. Always include a correct `viewBox`.
3. **Rotation pivot wrong (no `transform-box`).** CSS `rotate()`/`scale()` spins around canvas (0,0) and flies off-screen. Add `transform-box: fill-box; transform-origin: center;`.
4. **Giant unoptimized exported SVG.** 40KB of Illustrator metadata for a 24px icon. Run SVGO.
5. **No accessibility.** Either missing `aria-hidden` on decorative icons (screen-reader noise) or missing `aria-label`/`<title>` on meaningful ones (invisible to AT).
6. **Animating the wrong property.** Animating `cx`/`cy`/`x`/`y`/`width`/`r` causes layout jank; animate `transform: translate()/scale()` and `opacity` instead. Line-draw must animate `stroke-dashoffset`, not `stroke-width`.
7. **Duplicate `id`s when inlining many SVGs.** Gradients/filters/symbols collide — only the first wins. Namespace ids (`grad-hero`, `glow-btn`).
8. **`fill="url(grad)"` without the `#`.** Paint reference needs `url(#grad)`.
9. **Deprecated `xlink:href`.** Use plain `href` on `<use>`/`<mpath>`.
10. **Filter output clipped.** Blur/shadow cut off because the default filter region is too small. Set `x="-50%" y="-50%" width="200%" height="200%"` on the `<filter>`.
11. **`infinite` vs `indefinite` mixup.** CSS uses `infinite`; SMIL uses `repeatCount="indefinite"`. Wrong keyword = no loop.
12. **Forgetting `forwards` fill-mode.** A one-shot reveal animation snaps back to its start state without `animation-fill-mode: forwards`.
