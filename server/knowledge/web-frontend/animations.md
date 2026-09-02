# Web Animation Techniques

Concrete, copy-pasteable animation techniques for building smooth, professional web UI. Default to CSS for state changes; reach for a library only when the problem demands it. Every animated UI must include a `prefers-reduced-motion` block (see Accessibility).

## The Golden Rule: Only Animate `transform` and `opacity`

There are exactly two properties the browser can animate on the **compositor (GPU) thread** without recalculating layout or repainting: `transform` and `opacity`. Animating anything else (`width`, `height`, `top`, `left`, `right`, `bottom`, `margin`, `padding`, `border-width`) forces the browser to run **layout** (reflow) and **paint** on every single frame on the main thread — this is what makes animations stutter, drop below 60fps, and feel "janky".

The pipeline: **JS → Style → Layout → Paint → Composite**. `transform`/`opacity` skip straight to Composite. Layout-triggering properties run the whole pipeline every frame.

```css
/* ❌ WRONG — animates layout properties, janky, runs layout+paint every frame */
.box {
  position: absolute;
  left: 0;
  width: 100px;
  transition: left 0.3s, width 0.3s, top 0.3s;
}
.box:hover {
  left: 200px;   /* triggers layout */
  width: 150px;  /* triggers layout */
  top: 40px;     /* triggers layout */
}

/* ✅ RIGHT — same visual result via transform, runs on GPU, smooth 60fps */
.box {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.box:hover {
  transform: translate(200px, 40px) scale(1.5);
}
```

**Translating layout intent to transforms:**

| You want to animate | Don't use | Use instead |
|---|---|---|
| Move horizontally | `left` / `margin-left` | `transform: translateX()` |
| Move vertically | `top` / `margin-top` | `transform: translateY()` |
| Grow/shrink size | `width` / `height` | `transform: scale()` |
| Fade | `visibility`, `display` | `opacity` |
| Reposition X+Y | `top` + `left` | `transform: translate(x, y)` |

**Pitfalls:**
- `display: none` → `display: block` is NOT animatable (it's discrete). Animate `opacity` + `transform` and toggle `display` with `transition-behavior: allow-discrete` (modern) or a JS timeout.
- Animating `box-shadow` directly causes repaints and is slow. To animate a shadow cheaply, put the shadow on a pseudo-element and animate its `opacity` instead.
- Scaling with `transform: scale()` scales children and borders too. If you need a crisp box that grows, that's a layout animation — use FLIP or a library's layout animation, not naive `width`.

## CSS Transitions: Syntax, Duration, and Easing

A transition animates a property when its value changes (hover, focus, class toggle, etc.). Set the transition on the **base/resting state**, not only on `:hover`, so it animates both in AND out.

```css
.button {
  background: #2563eb;
  transform: scale(1);
  /* property | duration | easing | delay */
  transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1),
              background-color 0.2s ease-out;
}
.button:hover {
  background: #1d4ed8;
  transform: scale(1.03);
}
```

**Durations (use these as defaults):**
- **100–150ms** — tiny UI feedback: button press, checkbox, toggle, tooltip.
- **150–250ms** — standard UI state change: hover, focus, small reveals, dropdowns.
- **250–400ms** — larger elements: modals, drawers, accordion expand, page sections.
- **400–600ms** — full-screen / hero / dramatic entrances. Rarely longer.
- Anything over ~600ms for a UI interaction feels sluggish. Loading/ambient loops (spin, shimmer) are the exception — those are 1–2s.

**Never leave easing as the default `ease` or `linear` for UI.** `linear` looks robotic; default `ease` is bland. Real interfaces use an aggressive ease-out so motion starts fast and settles gently.

**Easing curve table — named curve → `cubic-bezier()` (these actually look good):**

| Name | cubic-bezier | Feel / use for |
|---|---|---|
| **easeOutExpo** ⭐ | `cubic-bezier(0.16, 1, 0.3, 1)` | THE good default. Snappy start, soft landing. Use for almost everything. |
| easeOutQuart | `cubic-bezier(0.25, 1, 0.5, 1)` | Slightly gentler than above. Reveals, modals. |
| easeOutCubic | `cubic-bezier(0.33, 1, 0.68, 1)` | Smooth, understated ease-out. |
| easeInOutCubic | `cubic-bezier(0.65, 0, 0.35, 1)` | Symmetric. Use when an element moves A→B and both ends matter. |
| easeInOutQuart | `cubic-bezier(0.76, 0, 0.24, 1)` | Punchier symmetric. Slides, carousels. |
| easeInCubic | `cubic-bezier(0.32, 0, 0.67, 0)` | Accelerate away. Use for EXIT animations (leaving screen). |
| **spring (soft)** | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Overshoots slightly then settles — playful, premium. Buttons, popovers. |
| spring (bouncy) | `cubic-bezier(0.68, -0.55, 0.265, 1.55)` | Strong overshoot at both ends. Use sparingly. |
| easeOutBack | `cubic-bezier(0.175, 0.885, 0.32, 1.275)` | Overshoots at the end only. Pop-in / scale-in. |

Rule of thumb: **enter with ease-out, exit with ease-in, move with ease-in-out.** `cubic-bezier(0.16, 1, 0.3, 1)` is the safest single choice if you only pick one.

**Pitfalls:**
- `transition: all 0.3s` is lazy and dangerous — it animates properties you didn't intend (including layout props), causing surprise jank. List properties explicitly.
- Putting the `transition` only inside `:hover` makes it animate in but snap out instantly. Put it on the base selector.
- Cubic-bezier `y` values outside `[0,1]` (like `1.56`) create overshoot/spring — that's intentional for springy curves, but the `x` values must stay in `[0,1]` or the curve is invalid.

## CSS `@keyframes`: Ready-to-Paste Animations

Keyframes define multi-step animations that run on load or when a class is added. `from`/`to` = `0%`/`100%`. Use these directly.

```css
/* FADE IN UP — the workhorse entrance */
@keyframes fadeInUp {
  from { opacity: 0; transform: translateY(16px); }
  to   { opacity: 1; transform: translateY(0); }
}
.fade-in-up {
  animation: fadeInUp 0.5s cubic-bezier(0.16, 1, 0.3, 1) both;
}

/* SCALE IN (pop) */
@keyframes scaleIn {
  from { opacity: 0; transform: scale(0.92); }
  to   { opacity: 1; transform: scale(1); }
}
.scale-in {
  animation: scaleIn 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275) both;
}

/* SLIDE IN FROM LEFT */
@keyframes slideInLeft {
  from { opacity: 0; transform: translateX(-24px); }
  to   { opacity: 1; transform: translateX(0); }
}
.slide-in-left {
  animation: slideInLeft 0.45s cubic-bezier(0.16, 1, 0.3, 1) both;
}

/* SHIMMER / SKELETON LOADING — animates background-position (cheap, no layout) */
@keyframes shimmer {
  from { background-position: -200% 0; }
  to   { background-position: 200% 0; }
}
.skeleton {
  background: linear-gradient(
    90deg,
    #e2e8f0 25%,
    #f1f5f9 37%,
    #e2e8f0 63%
  );
  background-size: 200% 100%;
  animation: shimmer 1.4s ease-in-out infinite;
  border-radius: 6px;
}

/* PULSE — gentle breathing for "loading" / "live" states */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50%      { opacity: 0.5; }
}
.pulse { animation: pulse 1.5s ease-in-out infinite; }

/* SPIN — loading spinner */
@keyframes spin {
  to { transform: rotate(360deg); }
}
.spinner {
  width: 24px; height: 24px;
  border: 3px solid rgba(0,0,0,0.1);
  border-top-color: #2563eb;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;  /* spin is the ONE place linear is correct */
}

/* BOUNCE — attention nudge (use rarely) */
@keyframes bounce {
  0%, 100%      { transform: translateY(0);    animation-timing-function: cubic-bezier(0.8, 0, 1, 1); }
  50%           { transform: translateY(-25%); animation-timing-function: cubic-bezier(0, 0, 0.2, 1); }
}
.bounce { animation: bounce 1s infinite; }
```

**The `animation` shorthand order:** `name | duration | timing-function | delay | iteration-count | direction | fill-mode`. Example: `animation: fadeInUp 0.5s ease-out 0.1s 1 normal both;`

**`animation-fill-mode` matters:**
- `both` (or `forwards`) — element keeps the final keyframe state after finishing. **Almost always use `both` for entrances**, otherwise the element snaps back to its un-animated CSS (e.g. flashes to `opacity:1` then jumps to `opacity:0`).
- Without a fill-mode, a `fadeInUp` element is visible (CSS default) BEFORE the animation runs, flickers, then animates. `both` fixes the pre-animation flash.

**Pitfalls:**
- Forgetting `both`/`forwards` → element flashes its final state at frame 0 then resets. This is the #1 keyframe bug.
- `infinite` loops on `transform`/`opacity` are fine; never loop layout properties.
- `spin` is the rare legit use of `linear` — a spinner with `ease` visibly stutters each rotation.

## `transform` Deep Dive: translate / scale / rotate / skew / 3D

`transform` accepts a space-separated list applied **right-to-left** (the last function is applied first). Order changes the result.

```css
/* 2D primitives */
transform: translateX(20px);
transform: translateY(-10px);
transform: translate(20px, -10px);   /* x, y */
transform: scale(1.2);               /* uniform */
transform: scale(1.2, 0.8);          /* scaleX, scaleY */
transform: rotate(15deg);
transform: skewX(-12deg);

/* Combining — order matters. This rotates, THEN translates the rotated frame. */
transform: translateX(100px) rotate(45deg);
/* vs: rotate first, then translate along original axes */
transform: rotate(45deg) translateX(100px);
```

**`transform-origin`** sets the pivot point (default `50% 50%`, the center):

```css
.menu { transform-origin: top right; transform: scale(0); }   /* grows from corner */
.card { transform-origin: bottom center; }                    /* tilts from base */
```

**3D transforms** — needed for flip cards, carousels, depth:

```css
/* Parent establishes the 3D viewing distance. Smaller perspective = more dramatic. */
.scene { perspective: 800px; }

/* Child can rotate in 3D */
.card-3d {
  transform-style: preserve-3d;        /* children keep their own 3D position */
  transition: transform 0.6s cubic-bezier(0.16, 1, 0.3, 1);
}
.card-3d:hover { transform: rotateY(180deg); }

/* Flip card faces */
.face {
  position: absolute; inset: 0;
  backface-visibility: hidden;          /* hide the mirror-image back of each face */
}
.face--back { transform: rotateY(180deg); }
```

Key 3D properties:
- `perspective: <px>` on the **parent** — vanishing-point distance. 600–1200px typical. Without it, `rotateY` looks flat.
- `transform-style: preserve-3d` — lets nested elements live in shared 3D space (required for flip cards).
- `backface-visibility: hidden` — hides an element when it's rotated away from you.
- `rotateY(180deg)` / `rotateX()` — rotate around vertical/horizontal axis.
- `translateZ(<px>)` — push toward/away from viewer (needs perspective).

**Pitfalls:**
- `rotateY` with no parent `perspective` → flat, lifeless. Always set `perspective`.
- Transform order bugs: `scale() translate()` translates by the *scaled* distance. Put `translate` first if you want untouched pixel distances: `translate(...) scale(...)`.
- A `transform` on an element creates a new stacking context and containing block — `position: fixed` children become positioned relative to the transformed ancestor. Surprising but expected.

## Micro-interactions: Buttons, Cards, Inputs, Ripple, Icons

Small, fast (100–250ms), tactile feedback. These are what separate professional UI from amateur.

```css
/* BUTTON: hover lift + press-down. The active scale 0.97 is the key "tactile" trick. */
.btn {
  transform: translateY(0) scale(1);
  transition: transform 0.15s cubic-bezier(0.16, 1, 0.3, 1),
              box-shadow 0.15s ease-out,
              background-color 0.15s ease-out;
}
.btn:hover  { transform: translateY(-1px); }
.btn:active { transform: scale(0.97); transition-duration: 0.08s; }  /* snap on press */

/* CARD HOVER LIFT: translateY up + bigger softer shadow */
.card {
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1),
              box-shadow 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
.card:hover {
  transform: translateY(-4px);
  box-shadow: 0 12px 24px rgba(0,0,0,0.12);
}

/* INPUT FOCUS: animated ring via box-shadow (no layout shift, unlike border-width) */
.input {
  border: 1px solid #cbd5e1;
  transition: border-color 0.15s ease-out, box-shadow 0.15s ease-out;
}
.input:focus {
  outline: none;
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.25);   /* the focus glow */
}

/* ICON ROTATE on toggle (e.g. chevron in an accordion) */
.chevron { transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1); }
[aria-expanded="true"] .chevron { transform: rotate(180deg); }
```

**Ripple effect (Material-style), minimal JS + CSS:**

```css
.ripple-btn { position: relative; overflow: hidden; }
.ripple {
  position: absolute;
  border-radius: 50%;
  transform: scale(0);
  background: rgba(255, 255, 255, 0.5);
  pointer-events: none;
  animation: ripple-expand 0.6s ease-out;
}
@keyframes ripple-expand {
  to { transform: scale(4); opacity: 0; }
}
```

```js
button.addEventListener('click', (e) => {
  const rect = button.getBoundingClientRect();
  const size = Math.max(rect.width, rect.height);
  const ripple = document.createElement('span');
  ripple.className = 'ripple';
  ripple.style.width = ripple.style.height = `${size}px`;
  ripple.style.left = `${e.clientX - rect.left - size / 2}px`;
  ripple.style.top  = `${e.clientY - rect.top  - size / 2}px`;
  button.appendChild(ripple);
  ripple.addEventListener('animationend', () => ripple.remove());
});
```

**Pitfalls:**
- Hover-only micro-interactions are inaccessible on touch. Provide `:active`/`:focus-visible` states too.
- Animating a `border` width on focus shifts layout (everything jumps 1–2px). Use `box-shadow` for rings instead.
- Forgetting to remove ripple nodes leaks DOM elements — always clean up on `animationend`.

## Entrance & Scroll Animations: IntersectionObserver + Scroll-Driven CSS

**The standard pattern:** elements start hidden, and an `IntersectionObserver` adds a class when they scroll into view, triggering a CSS transition. This is performant (no scroll listener) and works everywhere.

```css
/* Resting (hidden) state */
.reveal {
  opacity: 0;
  transform: translateY(24px);
  transition: opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1),
              transform 0.6s cubic-bezier(0.16, 1, 0.3, 1);
  will-change: opacity, transform;   /* hint; removed after reveal below */
}
/* Visible state (class added by JS) */
.reveal.is-visible {
  opacity: 1;
  transform: translateY(0);
}
```

```js
const observer = new IntersectionObserver((entries) => {
  for (const entry of entries) {
    if (entry.isIntersecting) {
      entry.target.classList.add('is-visible');
      entry.target.style.willChange = 'auto';   // cleanup hint after it fires
      observer.unobserve(entry.target);          // animate once, then stop watching
    }
  }
}, {
  threshold: 0.15,                  // fire when 15% visible
  rootMargin: '0px 0px -10% 0px',   // trigger slightly before it hits the bottom edge
});

document.querySelectorAll('.reveal').forEach((el) => observer.observe(el));
```

**Stagger children** — offset each child's delay for a cascade. With JS, set a CSS variable per child:

```js
document.querySelectorAll('.stagger > *').forEach((el, i) => {
  el.style.transitionDelay = `${i * 60}ms`;   // 60ms between items
});
```

Or pure CSS with `nth-child` (when count is known):

```css
.stagger > *:nth-child(1) { transition-delay: 0ms; }
.stagger > *:nth-child(2) { transition-delay: 60ms; }
.stagger > *:nth-child(3) { transition-delay: 120ms; }
.stagger > *:nth-child(4) { transition-delay: 180ms; }
```

**Modern CSS scroll-driven animations** (no JS, Chrome/Edge 115+, behind support check) — `animation-timeline: view()` ties the animation's progress to the element's position in the viewport:

```css
@keyframes reveal-on-scroll {
  from { opacity: 0; transform: translateY(40px); }
  to   { opacity: 1; transform: translateY(0); }
}
@supports (animation-timeline: view()) {
  .scroll-reveal {
    animation: reveal-on-scroll linear both;
    animation-timeline: view();                  /* progress = element's view position */
    animation-range: entry 0% cover 35%;         /* start as it enters, finish at 35% covered */
  }
}
```

`scroll()` timeline drives by the scroll container's overall scroll; `view()` drives by *this element's* visibility. Use `view()` for reveals, `scroll()` for progress bars.

**Pitfalls:**
- A `scroll` event listener that does work every frame is the classic janky-scroll mistake. Use `IntersectionObserver` (no per-frame work) or CSS scroll-timelines.
- Not calling `unobserve` after reveal keeps the observer firing and may re-trigger. Unobserve once shown (unless you want re-animation on re-entry).
- Scroll-driven CSS (`view()`) lacks support in older Safari — always wrap in `@supports` and ship the IntersectionObserver path as the baseline.
- Stagger delays over ~80ms/item make long lists feel slow. Keep 40–70ms.

## When to Use a Library vs Platform CSS

**Decision rule:**
- **CSS transitions/keyframes** — state changes (hover/focus/active), simple entrances, loaders, anything declarative. Default here. Zero JS, best performance.
- **Framer Motion (`motion` for React)** — React component **enter/exit** (mounting/unmounting), **layout animations** (auto-animate position/size changes), `AnimatePresence`, gesture/drag, spring physics with almost no code. Use when elements appear/disappear from the React tree or reorder.
- **GSAP** — complex **timelines** (orchestrate many elements with precise sequencing), scroll-triggered scenes, SVG path morphing, anything that needs frame-accurate control or runs outside React. Use for "scrollytelling", hero sequences, SVG drawing.

If it's a hover or a class toggle, **do not reach for a library** — that's an amateur tell (shipping 40KB of JS to fade a button).

**Framer Motion — enter/exit with `AnimatePresence`:**

```jsx
import { motion, AnimatePresence } from 'motion/react';

function Toast({ show }) {
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ opacity: 0, y: 20, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 20, scale: 0.95 }}
          transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
        >
          Saved!
        </motion.div>
      )}
    </AnimatePresence>
  );
}
```

**Framer Motion — automatic layout animation** (animates the position/size change for free when layout changes):

```jsx
// Add `layout` and FM animates any layout change (reorder, resize, expand) smoothly.
<motion.div layout transition={{ type: 'spring', stiffness: 500, damping: 40 }} />
```

**Framer Motion — imperative `useAnimate`** (for sequences / event-driven):

```jsx
import { useAnimate } from 'motion/react';

function Box() {
  const [scope, animate] = useAnimate();
  const run = async () => {
    await animate(scope.current, { scale: 1.2 }, { duration: 0.2 });
    await animate(scope.current, { scale: 1 }, { type: 'spring', stiffness: 400, damping: 15 });
  };
  return <div ref={scope} onClick={run} />;
}
```

**GSAP — timeline (precise multi-element sequencing):**

```js
import gsap from 'gsap';

const tl = gsap.timeline({ defaults: { ease: 'power3.out', duration: 0.6 } });
tl.from('.hero-title', { y: 40, opacity: 0 })
  .from('.hero-sub',   { y: 20, opacity: 0 }, '-=0.3')   // overlap previous by 0.3s
  .from('.hero-cta',   { scale: 0.8, opacity: 0 }, '<')  // start with previous
  .from('.hero-img',   { x: 60, opacity: 0 }, 0.2);      // absolute time 0.2s
```

**GSAP — ScrollTrigger (scroll-driven scenes):**

```js
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
gsap.registerPlugin(ScrollTrigger);

gsap.to('.panel', {
  xPercent: -100,
  ease: 'none',
  scrollTrigger: {
    trigger: '.container',
    pin: true,             // pin the section while it animates
    scrub: 1,              // tie progress to scrollbar (1 = 1s smoothing)
    start: 'top top',
    end: '+=2000',         // scroll distance the scene lasts
  },
});
```

GSAP `ease` names map to the same feel as the cubic-bezier table: `power2.out`/`power3.out`/`power4.out` ≈ stronger ease-out, `back.out(1.7)` ≈ overshoot, `elastic.out` ≈ springy, `expo.out` ≈ `cubic-bezier(0.16,1,0.3,1)`.

**Pitfalls:**
- Using GSAP/Framer for a simple hover — overkill, ships unnecessary JS. CSS wins.
- Forgetting `AnimatePresence` → exit animations never run (React unmounts instantly). Exit animations REQUIRE the component to stay in the tree until done; that's what `AnimatePresence` provides.
- Forgetting `gsap.registerPlugin(ScrollTrigger)` → ScrollTrigger silently does nothing.
- Framer's `layout` prop on an element whose content changes can distort text/images briefly — pair with `layout="position"` to only animate position, not size, when that happens.

## Spring Physics: Why Springs Beat Durations

Duration-based easing always takes a fixed time regardless of distance, which feels mechanical for interactive motion. **Springs** are defined by physics (stiffness, damping, mass) — they respond to velocity, settle naturally, and can carry momentum from a gesture. A spring-driven drag-release feels alive; a 300ms tween does not.

A spring is tuned by:
- **stiffness** — how strong the pull toward target (higher = faster, snappier). 200–600 typical.
- **damping** — resistance/friction (higher = less bounce; too low = wobbles forever). 20–40 typical.
- **mass** — heavier = slower, more lagging. Usually leave at 1.

**Framer Motion spring config:**

```jsx
// Snappy UI spring (minimal overshoot) — great default for interactions
<motion.div
  animate={{ scale: 1 }}
  transition={{ type: 'spring', stiffness: 500, damping: 30, mass: 1 }}
/>

// Bouncy / playful (visible overshoot)
<motion.div transition={{ type: 'spring', stiffness: 300, damping: 12 }} />

// Or specify by feel instead of physics:
<motion.div transition={{ type: 'spring', bounce: 0.25, duration: 0.5 }} />
```

Guidelines: **stiffness 500 / damping 30** ≈ crisp, almost no bounce (use for most UI). **damping 12–15** at the same stiffness = pronounced bounce. Lower stiffness = slower, softer.

**CSS approximation of a spring** — you can't do true physics in plain CSS, but the overshoot cubic-beziers fake it well:

```css
/* soft spring (subtle overshoot) */
transition: transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
/* back-out (overshoot at end only) — good for pop-in */
transition: transform 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
```

Modern CSS also ships a real `linear()` easing that can encode a sampled spring curve (generate via a spring-to-`linear()` tool):

```css
/* A baked spring curve as linear() — copy from a generator, this is illustrative */
transition: transform 0.6s linear(0, 0.5, 0.9, 1.05, 1.02, 1);
```

**Pitfalls:**
- Springs with very low damping (<10) wobble for a long time and feel broken — keep damping ≥ 12 for UI.
- Don't put springy overshoot on EVERYTHING (see "everything bouncing" amateur tell). Reserve overshoot for playful accents; use crisp ease-out for the bulk of UI.

## Performance: `will-change`, rAF, Layout Thrashing, `content-visibility`

**`will-change`** promotes an element to its own GPU layer *ahead of time* so the first animation frame isn't a stutter. It is a sharp tool:
- Add it shortly before animating, **remove it after** (set to `auto`). A permanent `will-change` on many elements eats GPU memory and can *hurt* performance.
- Never put `will-change: transform` on hundreds of elements or globally.

```css
.modal { will-change: transform, opacity; }   /* only while it animates */
```
```js
el.style.willChange = 'transform';
// ...animate...
el.addEventListener('transitionend', () => { el.style.willChange = 'auto'; }, { once: true });
```

**`requestAnimationFrame` for JS-driven animation** — never animate with `setInterval`/`setTimeout`; rAF syncs to the display refresh and pauses in background tabs:

```js
function animate(ts) {
  // update positions using `ts` (a high-res timestamp)
  el.style.transform = `translateY(${offset}px)`;
  if (!done) requestAnimationFrame(animate);
}
requestAnimationFrame(animate);
```

**Layout thrashing (read-then-write batching)** — interleaving DOM reads and writes forces synchronous reflows. Batch all reads, then all writes:

```js
// ❌ thrashes: read, write, read, write — forces layout each iteration
items.forEach((el) => { el.style.height = el.offsetHeight + 10 + 'px'; });

// ✅ batched: all reads first, then all writes
const heights = items.map((el) => el.offsetHeight);   // READ phase
items.forEach((el, i) => { el.style.height = heights[i] + 10 + 'px'; });  // WRITE phase
```

Properties that force synchronous layout when *read* mid-frame: `offsetTop/Left/Width/Height`, `clientWidth/Height`, `getBoundingClientRect()`, `getComputedStyle()`, `scrollTop`. Read them once, cache, then write.

**Avoid animating during scroll.** Heavy work in a `scroll` handler blocks the main thread and stutters scrolling. Prefer `IntersectionObserver`, CSS scroll-timelines, or `position: sticky` over scroll-listener animations. If you must listen, throttle with rAF and keep handlers trivial.

**`content-visibility: auto`** skips rendering of off-screen content, hugely speeding up long pages (pair with `contain-intrinsic-size` to reserve space and avoid scrollbar jumps):

```css
.long-section {
  content-visibility: auto;
  contain-intrinsic-size: auto 600px;   /* estimated height so scrollbar stays stable */
}
```

**Pitfalls:**
- Leaving `will-change` on permanently (the "missing cleanup" tell) — it's a hint to allocate a layer, not a free speed-up; overuse degrades performance.
- `requestAnimationFrame` loops that never stop (no exit condition) run forever and drain battery — always have a termination check.
- Reading `getBoundingClientRect()` inside an animation loop = guaranteed thrash.

## Accessibility: `prefers-reduced-motion`

Some users get motion sickness or vestibular disorders from animation. Respecting `prefers-reduced-motion: reduce` is non-negotiable — **every animated page must include this.** Drop this block once, globally:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

This near-instantly completes animations (so state still changes, but without motion). Prefer reducing motion to removing it entirely — opacity fades are usually fine; large translates/parallax are what to kill.

For finer control, gate motion *in* rather than out:

```css
@media (prefers-reduced-motion: no-preference) {
  .reveal { transition: transform 0.6s, opacity 0.6s; }
}
```

In JS (Framer Motion / GSAP), check the query and skip motion:

```js
const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
// Framer Motion: wrap your app in <MotionConfig reducedMotion="user"> to auto-honor it.
```

**Pitfalls:**
- Shipping any animation without this block is a hard accessibility failure and an instant amateur tell.
- Setting `animation: none` can break layouts that depend on the end state (e.g. `fill-mode: forwards` hiding/showing). Prefer the `0.01ms` duration trick so the final frame still applies.

## Page / Route Transitions: View Transitions API

The **View Transitions API** animates between two DOM states (or two pages) with a crossfade and shared-element morphs — no library. `document.startViewTransition(callback)` snapshots the old state, runs your DOM update, then animates old→new.

```js
function navigate(updateDOM) {
  if (!document.startViewTransition) {   // fallback for unsupported browsers
    updateDOM();
    return;
  }
  document.startViewTransition(() => updateDOM());
}
```

Customize the default crossfade in CSS via the `::view-transition-*` pseudo-elements:

```css
::view-transition-old(root),
::view-transition-new(root) {
  animation-duration: 0.3s;
  animation-timing-function: cubic-bezier(0.16, 1, 0.3, 1);
}
```

**Shared element transition** — give the same `view-transition-name` to an element in both states and it morphs (position/size) between them:

```css
.hero-image { view-transition-name: hero; }   /* same name on both pages → morphs */
```

For **multi-page apps** (full navigations), opt in with one line — Chrome animates between pages automatically:

```css
@view-transition { navigation: auto; }
```

In frameworks: Next.js App Router and Astro have built-in View Transition wrappers; React Router exposes `unstable_viewTransition` on `<Link>`. The plain API above works without any of them.

**Pitfalls:**
- `view-transition-name` must be **unique per snapshot** — two visible elements sharing a name throws and aborts the transition.
- No support check / fallback → in older Safari the DOM update simply won't run if you assume `startViewTransition` exists. Always guard with the `if (!document.startViewTransition)` fallback.
- Long view-transition durations feel slow; keep route crossfades ~200–350ms.

## Common Amateur Tells (and the Fix)

A checklist of the mistakes that make animations look amateur, each with the fix:

1. **Animating layout properties** (`width`, `height`, `top`, `left`, `margin`) → janky, sub-60fps. **Fix:** animate `transform` + `opacity` only.
2. **No easing / `linear` / default `ease`** → robotic, lifeless motion. **Fix:** use `cubic-bezier(0.16, 1, 0.3, 1)` (ease-out) as the default; ease-in for exits.
3. **`transition: all`** → animates unintended properties, surprise jank. **Fix:** name properties explicitly.
4. **Wrong durations** — 800ms hover (sluggish) or 50ms reveal (a flash). **Fix:** 150–250ms for UI, 300–500ms for larger elements.
5. **Everything bounces / overshoots** → toy-like, exhausting. **Fix:** reserve springy overshoot for occasional accents; crisp ease-out for the bulk.
6. **No `prefers-reduced-motion`** → accessibility failure. **Fix:** include the 4-line reduced-motion block on every project.
7. **Janky scroll animations** (work in a `scroll` listener every frame) → stutters scrolling. **Fix:** `IntersectionObserver` or CSS scroll-timelines.
8. **Missing `will-change` cleanup** (or slapping it on everything) → GPU memory bloat, sometimes slower. **Fix:** add right before animating, set back to `auto` after.
9. **Keyframe flash** — entrance element shows its final state at frame 0 then resets. **Fix:** `animation-fill-mode: both`.
10. **Exit animations that never run** in React. **Fix:** wrap in `AnimatePresence` (or keep the node mounted until the animation ends).
11. **Hover-only feedback** (dead on touch/keyboard). **Fix:** add `:active` and `:focus-visible` states.
12. **Animating `box-shadow`/`filter` directly in hot paths** → repaints. **Fix:** animate a pseudo-element's `opacity`, or accept the cost only on infrequent transitions.
13. **Layout thrashing** — reading `getBoundingClientRect()`/`offsetHeight` then writing in a loop. **Fix:** batch reads, then writes.
14. **Shipping a 40KB animation library to fade a button.** **Fix:** use CSS for state changes; libraries only for enter/exit, timelines, or layout animations.
