# Michael Design Library — sections-cta-footer

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 15 entries.

## Editorial Collection CTA — CTA [sections/editorial-collection-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/editorial-collection-cta.webp

---

### Prompt: Recreate the "Stay in the Collection" Newsletter Section

Build a single React component named `StaySection` using React + Vite + Tailwind CSS + Framer Motion. This is a full-viewport newsletter section with a background video and blur-up scroll-in animations.

### Dependencies

- `react` + `react-dom`
- `framer-motion`
- `tailwindcss`

### Fonts

Load via Google Fonts in `index.html`:

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Manrope:wght@300;400;500;600&display=swap" rel="stylesheet" />
```

Tailwind `fontFamily` config:

```js
fontFamily: {
  serif: ['"Instrument Serif"', 'serif'],
  sans: ['Manrope', 'sans-serif'],
}
```

### Animation Variant (shared)

A reusable `blurUp` object that fades, lifts, and unblurs as it enters the viewport — fired once, when 30% of the element is visible:

```ts
const blurUp = {
  initial: { opacity: 0, y: 40, filter: 'blur(20px)' },
  whileInView: { opacity: 1, y: 0, filter: 'blur(0px)' },
  viewport: { once: true, amount: 0.3 },
  transition: { duration: 1, ease: 'easeOut' },
};
```

### Section Structure

A `<section>` with:
- `position: relative`
- `min-height: 100vh`
- `background-color: #ffffff` (Tailwind `bg-white`)
- `overflow: hidden`

### Layer 1 — Background Video (no overlay on top)

A `<video>` element absolutely positioned and pinned to the bottom of the section:

- Classes: `absolute inset-x-0 bottom-0 w-full object-cover object-bottom pointer-events-none`
- Attributes: `autoPlay`, `loop`, `muted`, `playsInline`
- Source (use exactly this CloudFront URL):
  ```
  https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260603_202301_db51e299-b2f4-4cea-80de-8a6465b7532a.mp4
  ```
- `type="video/mp4"`
- No overlay, no tint, no gradient over the video.

### Layer 2 — Content Container

A `<div>` sibling to the video, with:

- `position: relative` (so it sits above the video without a z-index)
- `max-width: 1480px`, `margin: 0 auto`
- Horizontal padding: `px-8 md:px-16`
- Vertical padding: `pt-20 md:pt-24 pb-20 md:pb-24`
- `min-height: 100vh`
- Flex column with `gap: 32px`

### Block A — Heading (Framer Motion `motion.div` using `blurUp`)

Two stacked lines:

1. Line 1 — `<div>`:
   - Class: `font-serif leading-[0.95]`
   - Inline style: `fontSize: 'clamp(60px, 11vw, 160px)'`
   - Content: `Stay <span class="italic">in</span>`

2. Line 2 — `<div>`:
   - Class: `font-sans font-normal leading-[0.95]`
   - Inline style: `fontSize: 64, letterSpacing: '-0.02em'`
   - Content: `the collection`

### Block B — Newsletter (Framer Motion `motion.div` using `blurUp` with `delay: 0.2`)

- Class: `max-w-md`
- Override transition to `{ duration: 1, ease: 'easeOut', delay: 0.2 }`

Contains:

1. Paragraph:
   - Class: `font-sans mb-6`
   - Inline style: `fontSize: 15, lineHeight: 1.55, color: 'rgba(0,0,0,0.78)'`
   - Text: `Editions and invitations from the Bentley fragrance studio, sent twice a season.`

2. Form:
   - Class: `flex items-center border-b border-black/40 pb-2 gap-3`
   - `onSubmit` calls `e.preventDefault()`
   - Input:
     - `type="email"`
     - `placeholder="your@email.com"`
     - Class: `bg-transparent font-sans text-[15px] flex-1 outline-none placeholder:text-black/40`
   - Button:
     - `type="submit"`
     - Class: `font-sans text-[11px] font-medium uppercase text-black whitespace-nowrap cursor-pointer`
     - Inline style: `letterSpacing: '0.25em'`
     - Text: `Subscribe →` (the arrow is the literal Unicode `→`)

### Full Component Code

```tsx
import { motion } from 'framer-motion';

const blurUp = {
  initial: { opacity: 0, y: 40, filter: 'blur(20px)' },
  whileInView: { opacity: 1, y: 0, filter: 'blur(0px)' },
  viewport: { once: true, amount: 0.3 },
  transition: { duration: 1, ease: 'easeOut' },
};

export default function StaySection() {
  return (
    <section className="relative min-h-screen bg-white overflow-hidden">
      <video
        className="absolute inset-x-0 bottom-0 w-full object-cover object-bottom pointer-events-none"
        autoPlay
        loop
        muted
        playsInline
      >
        <source
          src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260603_202301_db51e299-b2f4-4cea-80de-8a6465b7532a.mp4"
          type="video/mp4"
        />
      </video>

      <div className="relative max-w-[1480px] mx-auto px-8 md:px-16 pt-20 md:pt-24 pb-20 md:pb-24 min-h-screen flex flex-col gap-[32px]">
        <motion.div {...blurUp}>
          <div className="font-serif leading-[0.95]" style={{ fontSize: 'clamp(60px, 11vw, 160px)' }}>
            Stay <span className="italic">in</span>
          </div>
          <div
            className="font-sans font-normal leading-[0.95]"
            style={{ fontSize: 64, letterSpacing: '-0.02em' }}
          >
            the collection
          </div>
        </motion.div>

        <motion.div
          {...blurUp}
          transition={{ ...blurUp.transition, delay: 0.2 }}
          className="max-w-md"
        >
          <p
            className="font-sans mb-6"
            style={{ fontSize: 15, lineHeight: 1.55, color: 'rgba(0,0,0,0.78)' }}
          >
            Editions and invitations from the Bentley fragrance studio, sent twice a season.
          </p>
          <form
            className="flex items-center border-b border-black/40 pb-2 gap-3"
            onSubmit={(e) => e.preventDefault()}
          >
            <input
              type="email"
              placeholder="your@email.com"
              className="bg-transparent font-sans text-[15px] flex-1 outline-none placeholder:text-black/40"
            />
            <button
              type="submit"
              className="font-sans text-[11px] font-medium uppercase text-black whitespace-nowrap cursor-pointer"
              style={{ letterSpacing: '0.25em' }}
            >
              Subscribe →
            </button>
          </form>
        </motion.div>
      </div>
    </section>
  );
}
```

### Color Reference

- Section background: `#ffffff`
- Primary text: `#000000`
- Body paragraph: `rgba(0,0,0,0.78)`
- Form divider: `border-black/40`
- Placeholder: `text-black/40`

### Behavioral Notes

- The video sits behind the content with no overlay, tint, or gradient.
- Heading and newsletter both fade + rise + unblur in sequence (200 ms apart), and only animate the first time the section scrolls into view.
- The form's submit handler is a no-op preventDefault (no persistence) — wire it up to your own backend if needed.

## FAQ CTA — CTA [sections/faq-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/faq-cta.webp

**PROMPT:**

Build a React + TypeScript + Vite + Tailwind CSS page with a "CTA + FAQ + Footer" section using the **Inter** font. Use `lucide-react` for icons (`ChevronDown`, `ChevronUp`). No other UI libraries.

### Layout

A centered container `max-w-[1100px] w-full mx-auto px-5`, white body (`bg-white text-neutral-900`), applied font: `style={{ fontFamily: "'Inter', sans-serif" }}`. Main section has `py-20 max-[900px]:py-[60px]`.

Inside `<main>`, a two-column grid:
- `grid grid-cols-[1.6fr_1fr] gap-[30px] items-stretch max-[900px]:grid-cols-1 max-[900px]:gap-[60px]`

### Left column — Animated Gradient CTA card

A div with class `c5-animated-gradient rounded-[24px] py-20 px-10 text-white flex flex-col justify-center items-center text-center` and inline `boxShadow: '0 10px 30px rgba(0, 0, 0, 0.05)'`.

Contents:
- `<h2>` — "Ready to Transfer<br/>Without Borders?" with classes `font-normal leading-[1.1] mb-[15px]` and inline `fontSize: '3.5rem', letterSpacing: '-0.03em'`.
- `<p>` — "Send Money Worldwide at the Best Rates" with `text-[0.9rem] mb-[30px] font-normal opacity-85`.
- `<button>` — "Get Started Today", classes `bg-neutral-900 text-white font-semibold cursor-pointer border-none text-[0.95rem] transition-all duration-200 hover:-translate-y-0.5`, inline `padding: '14px 32px', borderRadius: '12px', boxShadow: '0 10px 20px rgba(0,0,0,0.3)'`. On hover, bump shadow to `0 14px 30px rgba(0,0,0,0.4)` via `onMouseEnter`/`onMouseLeave`.

### Animated Gradient CSS (put in `src/index.css` after the Tailwind directives)

Use CSS `@property` declarations so custom properties interpolate smoothly, five radial-gradient blobs that each drift across wide paths AND pulse in size. Fast, looping, respects `prefers-reduced-motion`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@property --c5-x1 { syntax: '<percentage>'; inherits: false; initial-value: 10%; }
@property --c5-y1 { syntax: '<percentage>'; inherits: false; initial-value: 10%; }
@property --c5-x2 { syntax: '<percentage>'; inherits: false; initial-value: 90%; }
@property --c5-y2 { syntax: '<percentage>'; inherits: false; initial-value: 10%; }
@property --c5-x3 { syntax: '<percentage>'; inherits: false; initial-value: 10%; }
@property --c5-y3 { syntax: '<percentage>'; inherits: false; initial-value: 90%; }
@property --c5-x4 { syntax: '<percentage>'; inherits: false; initial-value: 90%; }
@property --c5-y4 { syntax: '<percentage>'; inherits: false; initial-value: 90%; }
@property --c5-x5 { syntax: '<percentage>'; inherits: false; initial-value: 50%; }
@property --c5-y5 { syntax: '<percentage>'; inherits: false; initial-value: 50%; }
@property --c5-s1 { syntax: '<percentage>'; inherits: false; initial-value: 55%; }
@property --c5-s2 { syntax: '<percentage>'; inherits: false; initial-value: 55%; }
@property --c5-s3 { syntax: '<percentage>'; inherits: false; initial-value: 55%; }
@property --c5-s4 { syntax: '<percentage>'; inherits: false; initial-value: 55%; }
@property --c5-s5 { syntax: '<percentage>'; inherits: false; initial-value: 65%; }

.c5-animated-gradient {
  background-color: #ff8e53;
  background-image:
    radial-gradient(circle at var(--c5-x1) var(--c5-y1), #fff1aa 0px, transparent var(--c5-s1)),
    radial-gradient(circle at var(--c5-x2) var(--c5-y2), #ff4b2b 0px, transparent var(--c5-s2)),
    radial-gradient(circle at var(--c5-x3) var(--c5-y3), #8aff8a 0px, transparent var(--c5-s3)),
    radial-gradient(circle at var(--c5-x4) var(--c5-y4), #ffd000 0px, transparent var(--c5-s4)),
    radial-gradient(circle at var(--c5-x5) var(--c5-y5), #ff1493 0px, transparent var(--c5-s5));
  animation:
    c5-blob1 5s ease-in-out infinite,
    c5-blob2 6s ease-in-out infinite,
    c5-blob3 5.5s ease-in-out infinite,
    c5-blob4 6.5s ease-in-out infinite,
    c5-blob5 4s ease-in-out infinite,
    c5-size1 3.5s ease-in-out infinite,
    c5-size2 4.2s ease-in-out infinite,
    c5-size3 3.8s ease-in-out infinite,
    c5-size4 4.6s ease-in-out infinite,
    c5-size5 3s ease-in-out infinite;
}

@keyframes c5-blob1 {
  0%,100% { --c5-x1: 5%;  --c5-y1: 5%;  }
  25%     { --c5-x1: 45%; --c5-y1: 20%; }
  50%     { --c5-x1: 30%; --c5-y1: 55%; }
  75%     { --c5-x1: 0%;  --c5-y1: 30%; }
}
@keyframes c5-blob2 {
  0%,100% { --c5-x2: 95%; --c5-y2: 5%;  }
  33%     { --c5-x2: 55%; --c5-y2: 35%; }
  66%     { --c5-x2: 80%; --c5-y2: 65%; }
}
@keyframes c5-blob3 {
  0%,100% { --c5-x3: 5%;  --c5-y3: 95%; }
  40%     { --c5-x3: 45%; --c5-y3: 65%; }
  70%     { --c5-x3: 25%; --c5-y3: 100%; }
}
@keyframes c5-blob4 {
  0%,100% { --c5-x4: 95%; --c5-y4: 95%; }
  30%     { --c5-x4: 60%; --c5-y4: 70%; }
  60%     { --c5-x4: 100%; --c5-y4: 50%; }
}
@keyframes c5-blob5 {
  0%,100% { --c5-x5: 50%; --c5-y5: 50%; }
  25%     { --c5-x5: 70%; --c5-y5: 30%; }
  50%     { --c5-x5: 40%; --c5-y5: 70%; }
  75%     { --c5-x5: 30%; --c5-y5: 40%; }
}

@keyframes c5-size1 { 0%,100% { --c5-s1: 45%; } 50% { --c5-s1: 80%; } }
@keyframes c5-size2 { 0%,100% { --c5-s2: 45%; } 50% { --c5-s2: 85%; } }
@keyframes c5-size3 { 0%,100% { --c5-s3: 45%; } 50% { --c5-s3: 78%; } }
@keyframes c5-size4 { 0%,100% { --c5-s4: 45%; } 50% { --c5-s4: 82%; } }
@keyframes c5-size5 { 0%,100% { --c5-s5: 50%; } 50% { --c5-s5: 85%; } }

@media (prefers-reduced-motion: reduce) {
  .c5-animated-gradient { animation: none; }
}
```

### Right column — FAQ accordion

State: `const [activeIndex, setActiveIndex] = useState<number | null>(0);` with toggle function.

FAQ data array (in order):
1. Q: "What is the maximum amount I can send?" — A: "Transfer limits depend on your verification level and country. You can check your limits inside your account settings."
2. Q: "Does my recipient need an account?" — A: "No, your recipient doesn't need an account. Funds can be sent directly to their bank account or mobile wallet."
3. Q: "Is there a mobile app available?" — A: "Yes, our mobile app is available on both iOS and Android for easy transfers on the go."
4. Q: "Can I cancel a transfer?" — A: "Transfers can be cancelled if they have not yet been processed by the receiving bank. Check your transfer status for options."
5. Q: "What currencies are supported?" — A: "We support over 50 currencies worldwide. You can view the full list of supported currencies in our app or website."

Container: `flex flex-col justify-center gap-3`.

Each item: clickable div, `bg-white border rounded-[10px] py-[18px] px-5 cursor-pointer transition-all duration-200`, border color `#eaeaea` when active else `#f0f0f0` (+ `hover:border-[#eaeaea]`). Box shadow `0 4px 12px rgba(0,0,0,0.04)` when active, else `0 2px 8px rgba(0,0,0,0.02)`.

Row: `flex justify-between items-center font-normal text-[0.9rem] text-neutral-900`, question on left, `ChevronUp` (size 20) if active else `ChevronDown`.

When active, answer block below: `mt-3 text-[0.9rem] text-[#666] leading-[1.6]`.

### Footer

`<footer className="bg-[#fafafa] pt-20 pb-5 max-[900px]:pt-[60px]">`, container `max-w-[1100px] w-full mx-auto px-5`.

Grid: `grid grid-cols-[2fr_1fr_1fr_2fr] gap-10 mb-[50px] max-[900px]:grid-cols-2 max-[480px]:grid-cols-1`.

1. **Logo column**: `<img src="https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/Tests/logoipsum-415.svg" className="h-6 mb-[15px]" style={{ filter: 'brightness(0)' }}/>` then `<p className="text-[0.85rem] text-[#888] leading-[1.6] max-w-[220px]">Reliable transfers that always reach their destination on time.</p>`.
2. **Navigation**: `<h4 className="font-semibold mb-5 text-[0.95rem] text-neutral-900">Navigation</h4>` + `<ul>` of `Features, Benefits, Testimonials, Pricing` — each `<li className="mb-3">` with `<a href="#" className="text-[#888] no-underline text-[0.85rem] transition-colors duration-200 hover:text-neutral-900">`.
3. **Pages**: same styling, items `Home, Contact, 404`.
4. **Newsletter**: heading "Newsletter", p: "Join our newsletter and get notified." (`text-[0.85rem] text-[#888] mb-[15px]`), then `flex gap-[10px]`:
   - Input: `type="email"`, placeholder "Enter your email...", classes `flex-grow border border-[#f0f0f0] bg-white outline-none transition-colors duration-200 focus:border-[#ccc] text-[0.9rem]`, inline `padding: '12px 16px', borderRadius: '10px', boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.02)'`.
   - Button "Subscribe": `bg-neutral-900 text-white border-none font-semibold cursor-pointer transition-all duration-200 hover:-translate-y-0.5 text-[0.9rem]`, inline `padding: '12px 28px', borderRadius: '10px', boxShadow: '0 12px 24px rgba(0,0,0,0.4)'`.

Bottom bar: `border-t border-[#f0f0f0] pt-[25px] pb-[10px] flex justify-between text-[0.85rem] text-[#888] max-[480px]:flex-col max-[480px]:gap-[15px] max-[480px]:items-center` containing "All rights reserved. © 2025" and "Designed by Peter Design".

### Font loading

Add to `index.html` `<head>`: Google Fonts Inter preconnect + stylesheet link:
```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
```

### Notes

- Gradient uses five colors: base `#ff8e53`, blobs `#fff1aa`, `#ff4b2b`, `#8aff8a`, `#ffd000`, `#ff1493`.
- Animation uses CSS `@property` for GPU-friendly custom-property interpolation — this is the modern standard for animated CSS gradients (no JS, no canvas).
- Blobs travel wide paths and pulse in radius; durations 3–6.5s, each offset for organic motion.

## Global CTA Footer — CTA [sections/global-cta-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/global-cta-footer.webp

Build a landing page for "Highframe" -- a SaaS product for building internal workflows, smart forms, and automations without code. The page has two responsive versions: **desktop** (served as a standalone HTML file) and **mobile** (served as a standalone HTML file). A React app switches between them via iframe based on viewport width (breakpoint: 768px).

---

### BRAND IDENTITY

- **Product name:** Highframe
- **Logo:** SVG circle icon -- outer circle (r=12, stroke #fff 1.6px), inner arc path (`M13 5a8 8 0 1 1-5.66 2.34`, stroke #fff 1.6px, round linecap), center dot (r=2.4, fill #fff), all within a 26x26 viewBox
- **Tagline:** "Build the tools your team *actually* needs"
- **Sub-headline:** "Turn any process into an intelligent form that routes data and triggers actions instantly."

### FONTS

Load from Google Fonts:
- **Hanken Grotesk** (weights: 400, 500, 600, 700) -- used for body, nav, buttons, UI
- **EB Garamond** (weights: 400, 500, 600 + italic variants) -- used for h1 headings and footer headings

### COLOR SYSTEM (CSS custom properties)

```
--ink: #0c0d0d
--paper: #f4f3f0
--lime: #c7ef6b
--lime-deep: #b6e34f (desktop only)
--green: #16331f (desktop only)
--muted: rgba(255,255,255,.72) (desktop) / rgba(255,255,255,.65) (mobile)
--line: rgba(255,255,255,.16) (desktop) / rgba(255,255,255,.13) (mobile)
--card: rgba(255,255,255,.055) (mobile only)
```

Body background: `#000` (desktop), `#060707` (mobile). All text white.

### HERO BACKGROUND VIDEO

- **URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260604_125109_19424216-4e2a-4560-b9f2-f1b5f6eb2c2e.mp4`
- Attributes: autoplay, muted, loop (desktop) / no loop (mobile), playsinline
- Position: absolute, inset 0, 100% width/height, object-fit: cover
- **Horizontally flipped** using `transform: scaleX(-1)`
- Desktop also has `scale(1.12)` and a Ken Burns animation (`scale(1.12)` to `scale(1.2)`, 26s ease-in-out infinite alternate)
- Mobile has `object-position: center top`; desktop has `object-position: center`
- **No overlay/scrim** on the video
- Mobile loops the video back to 0 at 2 seconds using a `timeupdate` listener
- z-index: -2

---

### DESKTOP HERO SECTION

**Layout:** `.hero` section, min-height 880px, `isolation: isolate`, overflow visible.

**Nav bar** (88px height, flexbox space-between, z-index 5):
- Left: brand logo (26x26 SVG) + "Highframe" text (20px, weight 600, letter-spacing -.01em)
- Right nav links: "Product" (with chevron dropdown SVG), "Resources" (with chevron), "Pricing", "Customers", "Book a demo" (pill button with border `rgba(255,255,255,.4)`, border-radius 999px, padding 10px 20px, frosted background `rgba(255,255,255,.04)`)
- Nav links: 15.5px, weight 500, color `rgba(255,255,255,.88)`, gap 34px

**Hero content** (grid layout, single column):
- `.hero-copy` container: padding 78px top 110px bottom, max-width 620px, z-index 4
- **h1:** EB Garamond, 62px, line-height 1.06, weight 400, letter-spacing -.005em, text-wrap balance. Contains `<em>` for "actually" (italic)
- **Subtitle (`.sub`):** EB Garamond, 22px, line-height 1.5, color `var(--muted)`, max-width 430px
- **CTA buttons** (flex row, gap 14px, margin-top 38px):
  - Primary: "Get started for free" -- `#f4f3f0` bg, `#121312` text, 999px radius, 54px height, 16px weight 600, box-shadow `0 8px 30px rgba(0,0,0,.35)`. Hover: translateY(-2px) + stronger shadow
  - Ghost: "Watch demo" with Material Icons Round play_circle icon -- `rgba(12,13,13,.72)` bg, white text, border `rgba(255,255,255,.32)`, backdrop-filter blur(14px), gap 10px

**Dashboard iframe** (`.dash`):
- Positioned absolutely via CSS custom properties: `--dash-w: 670px`, `--dash-x: 59%`, `--dash-y: 55px`
- border-radius 16px, box-shadow `0 40px 90px -22px rgba(0,0,0,.7), 0 0 0 1px rgba(255,255,255,.06)`
- Contains iframe to `assets/dashboard-orchestrator.html`, aspect-ratio 1456/1138

**Reveal animations** (triggered by JS on load):
- `.reveal` elements start with `opacity:0; transform:translateY(22px)` and transition with `.8s cubic-bezier(.2,.7,.2,1)`
- Dashboard reveal starts with `translateX(46px) scale(.985)`
- Staggered delays: nav 0s, h1 .12s, subtitle .26s, CTA .4s, dashboard .5s
- Handles visibility state: if page loads hidden, snaps content instantly then animates on visibility change

**Responsive (max-width 860px):**
- Hide nav links, h1 to 42px, hero min-height auto, hero-copy padding reduced
- Dashboard becomes relative positioned, full width, border-radius 14px

**Responsive (max-width 1100px):**
- h1 to 54px

---

### DESKTOP FOOTER

- Transparent background with frosted glass `::after` pseudo-element: `backdrop-filter: blur(12px) saturate(120%)`, gradient from `rgba(10,12,11,.38)` to `rgba(10,12,11,.80)`, `box-shadow: inset 0 1px 0 rgba(255,255,255,.18)`, border-radius 22px 22px 0 0
- `margin-top: -120px` to overlap hero, `border-top: 1px solid rgba(255,255,255,.14)`, border-radius 22px 22px 0 0

**Top grid** (4 columns: `1.1fr 1fr 1fr 1.5fr`, gap 30px, padding 72px top 64px bottom):

Columns 1-3 are link lists:
- **Product:** Workflow builder, AI automations, Smart forms, Data connections, Internal apps
- **Resources:** Mobile, Manifesto, Press, Docs, Pricing
- **Company:** About, Blog, Careers, Customers
- Column headers: 13px, uppercase, letter-spacing .08em, color `rgba(255,255,255,.45)`, weight 600
- Links: 16px, color `rgba(255,255,255,.82)`, gap 14px. Hover: color #fff, translateX(2px)

Column 4 (`.fbrand`):
- Brand logo (28x28) + "Highframe" (23px)
- Description: "Skip the dev queue. Build internal workflows, smart forms, and automations without code." -- 17px, line-height 1.5, color `rgba(255,255,255,.7)`, max-width 330px
- **Waitlist form** (pill shape): flex row, `rgba(255,255,255,.08)` bg, border `rgba(255,255,255,.16)`, border-radius 999px, padding 6px 6px 6px 20px, max-width 440px, backdrop-filter blur(8px), inset box-shadow
  - Mail icon SVG (17x17) + email input (15.5px)
  - Submit button: white bg, `#0c0d0d` text, 999px radius, padding 12px 20px, 15px weight 600

**Bottom bar** (border-top `rgba(255,255,255,.12)`, flex space-between, padding 26px top 38px bottom):
- Left: "(c) 2026  Highframe. All rights reserved." -- 14.5px, color `rgba(255,255,255,.55)`
- Center: Social icons (X/Twitter, LinkedIn, GitHub) -- SVG 18x18, color `rgba(255,255,255,.6)`, gap 22px. Hover: color #fff, translateY(-2px)
- Right: "Privacy Policy", "Terms of Use", "Cookie Policy" -- 14.5px, color `rgba(255,255,255,.6)`, gap 28px

**Responsive (max-width 860px):**
- Grid becomes 2 columns, brand spans full width, bottom stacks vertically

---

### MOBILE HERO SECTION

Wrapped in `.phone` container (max-width 430px, centered, min-height 100dvh, bg #000).

**Layout:** `.hero` section, min-height 100dvh, flexbox column, padding-bottom 28px, isolation isolate.

**Lime glow:** `::after` pseudo-element -- radial gradient of `rgba(199,239,107,.18)`, 280px circle, centered bottom 12%, blur 22px.

**Nav** (flex space-between, padding 26px 22px 0, z-index 10):
- Left: brand logo (24x24) + "Highframe" (18px, weight 600)
- Right: hamburger button (42x42, rounded 11px, border `rgba(255,255,255,.22)`, bg `rgba(0,0,0,.28)`, backdrop-filter blur(12px), three 18px wide / 1.4px tall white bars, gap 5px)

**Hero body** (flex column, center aligned, text-align center, padding 32px 22px 38px, z-index 5):
- **Badge** (hidden by default via `display:none`): pill with green dot (7px, `var(--lime)` with glow), "AI-Powered Workflow", frosted glass style
- **h1:** EB Garamond, `clamp(44px, 12vw, 58px)`, line-height 1, letter-spacing -.025em, max-width 9ch, text-shadow `0 10px 30px rgba(0,0,0,.35)`. `<em>` styled with `color: var(--lime)` and italic
- **Subtitle:** 16px, line-height 1.62, color `rgba(255,255,255,.76)`, max-width 31ch
- **CTA stack** (flex column, gap 11px, max-width 340px):
  - Primary: "Start Free Trial" -- `var(--paper)` bg, `var(--ink)` text, 58px height, 18px radius, 15.5px weight 600. Active: scale(.975) opacity .9
  - Ghost: "Watch Demo" with play circle SVG icon -- `rgba(12,13,13,.78)` bg, border `rgba(255,255,255,.30)`, backdrop-filter blur(14px)

**Dashboard preview** (`.hero-preview`):
- Perspective 3D card: `perspective(1400px) rotateX(17deg) rotateY(-8deg) rotateZ(1deg) translateY(10px)`
- Width `min(88vw, 368px)`, border-radius 32px, padding 10px
- Multi-layered gradient background simulating glass edge lighting
- Complex box-shadows for depth
- `::before` adds highlight gradients, `::after` adds colored glow beneath
- Contains `.preview-shell` with iframe to `dashboard-orchestrator.html` (aspect-ratio 1456/1138)
- **Float animation:** 6.5s ease-in-out infinite, subtle rotation/translate changes at 50% keyframe
- Shell has frosted glass styling with gradient borders via mask technique

**Responsive (max-height 760px):** hero-body justify-content flex-end, h1 clamped smaller

**Reveal animations:** `.r` class, `opacity 0 -> 1`, `translateY(20px) -> none`, .75s `cubic-bezier(.16,1,.3,1)`. Hero items animate immediately, footer items use IntersectionObserver (threshold 0.08). Staggered delays from 0s to .46s.

---

### MOBILE FOOTER

Class `.foot`, bg #000, border-top `rgba(255,255,255,.13)`, border-radius 26px 26px 0 0, padding 54px 22px 44px.

Shimmer `::after`: 1px height, horizontal gradient of white at center fading to transparent at edges.

Box-shadow: `0 1px 0 rgba(255,255,255,.10) inset, 0 -1px 0 rgba(0,0,0,.28) inset`

**Elements in order:**

1. **Mail icon circle** (58px, centered, frosted glass pill `rgba(255,255,255,.13)` bg, border `rgba(255,255,255,.30)`, glow box-shadow)

2. **Heading:** "Skip the dev queue" -- EB Garamond, 38px, weight 400, centered, line-height 1.08, letter-spacing -.015em

3. **Subtitle:** "Build internal workflows, smart forms, and automations without code." -- 15px, line-height 1.55, color `rgba(210,195,210,.72)`, centered

4. **Email form:**
   - Input: 52px height, `rgba(255,255,255,.06)` bg, border `rgba(255,255,255,.20)`, 14px radius, 15px font, backdrop-filter blur(10px). Focus: brighter border + glow ring
   - Button: "Sign up for waitlist" + arrow SVG -- 52px height, `var(--paper)` bg, `var(--ink)` text, 14px radius, 15.5px weight 600, box-shadow `0 10px 36px -14px rgba(255,255,255,.38)`

5. **Accordion** (4 sections: Product, Resources, Company, Legal):
   - Trigger: 16px, weight 500, color `rgba(235,220,230,.86)`, flex with chevron SVG
   - Chevron rotates 180deg on open, .26s cubic-bezier(.4,0,.2,1)
   - Body: max-height 0 -> 300px transition, links 15px color `rgba(255,255,255,.6)`
   - Same link lists as desktop (Product: 5 items, Resources: 5 items, Company: 4 items, Legal: Privacy Policy, Terms of Use, Cookie Policy)
   - Only one accordion open at a time (JS closes others on toggle)

6. **Social buttons** (centered row, gap 10px):
   - X/Twitter, LinkedIn, GitHub -- 46px circles, border `rgba(255,255,255,.18)`, bg `rgba(255,255,255,.04)`, color `rgba(215,190,210,.72)`, backdrop-filter blur(8px)
   - Hover: brighter bg/border/color + glow shadow

7. **Brand lockup card:** frosted glass card (18px radius, `rgba(255,255,255,.07)` bg, border `rgba(255,255,255,.16)`, backdrop-filter blur(14px)), logo (22x22) + "Highframe" (20px) + "(c) 2026 Highframe. All rights reserved." (13px, color `rgba(255,255,255,.38)`)

Reveal stagger delays: mail icon 0s, heading .07s, subtitle .13s, form .19s, accordion .25s, socials .37s, brand .43s.

---

### REACT APP (App.jsx)

Simple responsive switcher:
- Uses `window.matchMedia` to detect mobile (<768px)
- Renders a full-viewport iframe pointing to either `/desktop/index.html` or `/mobile/index.html`
- Switches iframe key on breakpoint change

### KEY IMPLEMENTATION NOTES

- All CSS is inline in `<style>` tags within each HTML file (no external stylesheets beyond Google Fonts)
- Material Icons Round loaded via Google Fonts CDN (desktop only)
- The `dashboard-orchestrator.html` iframe content is a separate file (not described here)
- Desktop uses a tweaks panel system (React + Babel standalone) for adjusting dashboard position via sliders -- this is a dev tool overlay
- `-webkit-font-smoothing: antialiased` and `text-rendering: optimizeLegibility` on body
- `-webkit-tap-highlight-color: transparent` on all interactive elements (mobile)
- `scroll-behavior: smooth` on html

## Liquid Glass CTA — CTA [sections/liquid-glass-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/liquid-glass-cta.webp

Build a "CTA + Footer" section component for a React + Vite + Tailwind CSS project. This is a cinematic full-width call-to-action section with an HLS video background, centered text, two CTA buttons, and a minimal footer bar at the bottom. Black background, white text, liquid glassmorphism effects.

---

### FONTS (import in index.css or HTML head)

```
https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap
```

- Headings: `Instrument Serif` italic -- Tailwind class `font-heading`
- Body: `Barlow` -- Tailwind class `font-body`

Add to `tailwind.config.ts` under `theme.extend.fontFamily`:
```js
heading: ["'Instrument Serif'", "serif"],
body: ["'Barlow'", "sans-serif"],
```

Base styles in `index.css`:
```css
body {
  font-family: 'Barlow', sans-serif;
  background: #000;
  color: #fff;
}
h1, h2, h3 {
  font-family: 'Instrument Serif', serif;
}
```

---

### LIQUID GLASS CSS (add to index.css inside `@layer components`)

```css
@layer components {
  .liquid-glass-strong {
    background: rgba(255, 255, 255, 0.01);
    background-blend-mode: luminosity;
    backdrop-filter: blur(50px);
    -webkit-backdrop-filter: blur(50px);
    border: none;
    box-shadow: 4px 4px 4px rgba(0, 0, 0, 0.05),
      inset 0 1px 1px rgba(255, 255, 255, 0.15);
    position: relative;
    overflow: hidden;
  }

  .liquid-glass-strong::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    padding: 1.4px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.5) 0%,
      rgba(255, 255, 255, 0.2) 20%,
      rgba(255, 255, 255, 0) 40%,
      rgba(255, 255, 255, 0) 60%,
      rgba(255, 255, 255, 0.2) 80%,
      rgba(255, 255, 255, 0.5) 100%
    );
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    pointer-events: none;
  }
}
```

The `::before` pseudo-element uses a mask-composite trick to render a thin glowing gradient border that fades out in the middle of each side.

---

### DEPENDENCIES

```
npm install lucide-react hls.js
```

- `ArrowUpRight` icon from `lucide-react`
- `hls.js` for streaming the Mux HLS video

---

### HLS VIDEO URL (Mux)

```
https://stream.mux.com/8wrHPCX2dC3msyYU9ObwqNdm00u3ViXvOSHUMRYSEe5Q.m3u8
```

This is an HLS stream that requires `hls.js` to play in non-Safari browsers. Safari supports HLS natively via `<video>`.

---

### EXACT COMPONENT CODE

```tsx
import { useEffect, useRef } from "react";
import { ArrowUpRight } from "lucide-react";
import Hls from "hls.js";

const CtaFooter = () => {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const src = "https://stream.mux.com/8wrHPCX2dC3msyYU9ObwqNdm00u3ViXvOSHUMRYSEe5Q.m3u8";

    if (Hls.isSupported()) {
      const hls = new Hls();
      hls.loadSource(src);
      hls.attachMedia(video);
      return () => hls.destroy();
    } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
      video.src = src;
    }
  }, []);

  return (
    <section className="relative py-32 px-6 md:px-16 lg:px-24 text-center overflow-hidden">
      {/* Background HLS Video */}
      <video
        ref={videoRef}
        autoPlay
        loop
        muted
        playsInline
        className="absolute inset-0 w-full h-full object-cover z-0"
      />

      {/* Top fade */}
      <div
        className="absolute top-0 left-0 right-0 z-[1] pointer-events-none"
        style={{ height: '200px', background: 'linear-gradient(to bottom, black, transparent)' }}
      />
      {/* Bottom fade */}
      <div
        className="absolute bottom-0 left-0 right-0 z-[1] pointer-events-none"
        style={{ height: '200px', background: 'linear-gradient(to top, black, transparent)' }}
      />

      {/* Content */}
      <div className="relative z-10">
        <h2 className="text-5xl md:text-6xl lg:text-7xl font-heading italic text-white tracking-tight leading-[0.85] max-w-3xl mx-auto mb-4">
          Your next website starts here.
        </h2>
        <p className="text-white/60 font-body font-light text-sm md:text-base max-w-xl mx-auto mb-8">
          Book a free strategy call. See what AI&#8209;powered design can do. No commitment, no pressure. Just possibilities.
        </p>
        <div className="flex items-center justify-center gap-6">
          <button className="liquid-glass-strong rounded-full px-6 py-3 text-sm font-medium text-white flex items-center gap-2 hover:bg-white/10 transition-all font-body">
            Book a Call
            <ArrowUpRight className="h-5 w-5" />
          </button>
          <button className="bg-white text-black rounded-full px-6 py-3 text-sm font-medium flex items-center gap-2 hover:bg-white/90 transition-colors font-body">
            View Pricing
            <ArrowUpRight className="h-4 w-4" />
          </button>
        </div>

        {/* Footer */}
        <div className="mt-32 pt-8 border-t border-white/10 flex flex-col md:flex-row items-center justify-between gap-4">
          <p className="text-white/40 font-body font-light text-xs">
            &copy; 2026 Studio. All rights reserved.
          </p>
          <div className="flex items-center gap-6">
            {["Privacy", "Terms", "Contact"].map((link) => (
              <a key={link} href="#" className="text-white/40 hover:text-white/70 font-body font-light text-xs transition-colors">
                {link}
              </a>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
};

export default CtaFooter;
```

---

### SECTION STRUCTURE BREAKDOWN

```
<section>  (relative, py-32, px-6 md:px-16 lg:px-24, text-center, overflow-hidden)
  |
  +-- <video>  (absolute inset-0, full cover, z-0, autoPlay loop muted playsInline)
  |
  +-- Top gradient fade  (absolute top-0, 200px tall, black->transparent, z-[1])
  +-- Bottom gradient fade  (absolute bottom-0, 200px tall, transparent<-black, z-[1])
  |
  +-- Content wrapper  (relative z-10)
       |
       +-- <h2> heading
       +-- <p> subtext
       +-- Button row (flex, centered, gap-6)
       |    +-- "Book a Call" (liquid-glass-strong, rounded-full)
       |    +-- "View Pricing" (bg-white text-black, rounded-full)
       |
       +-- Footer bar (mt-32, border-t border-white/10)
            +-- Copyright (left)
            +-- Links: Privacy, Terms, Contact (right)
```

---

### HLS VIDEO SETUP PATTERN

The `useEffect` hook initializes `hls.js` for non-Safari browsers and falls back to native HLS for Safari:

1. Check `Hls.isSupported()` -- if true, create an `Hls` instance, load the `.m3u8` source, attach to the `<video>` element
2. If not supported but the browser can play `application/vnd.apple.mpegurl` (Safari), set `video.src` directly
3. Cleanup: `hls.destroy()` on unmount

The `<video>` element uses `autoPlay loop muted playsInline` -- all four attributes are required for autoplay to work across browsers (especially mobile).

---

### VIDEO OVERLAY FADE PATTERN

Two absolutely positioned `<div>` elements create black gradient fades at the top and bottom edges, making the video blend seamlessly into the surrounding black background:

- **Top fade**: `height: 200px`, `background: linear-gradient(to bottom, black, transparent)`, `z-[1]`, `pointer-events-none`
- **Bottom fade**: `height: 200px`, `background: linear-gradient(to top, black, transparent)`, `z-[1]`, `pointer-events-none`

Content sits at `z-10` above both the video and the fades.

---

### RESPONSIVE BEHAVIOR

| Breakpoint | Heading size | Padding | Footer layout |
|---|---|---|---|
| Mobile (default) | `text-5xl` | `px-6` | Stacked column (`flex-col`) |
| Tablet (`md:`) | `text-6xl` | `px-16` | Horizontal row (`md:flex-row`) |
| Desktop (`lg:`) | `text-7xl` | `px-24` | Horizontal row |

- Button row always horizontal (`flex items-center justify-center gap-6`), buttons stack naturally if viewport is very narrow
- Footer: `flex-col md:flex-row` -- copyright and links stack on mobile, sit side-by-side on tablet+
- Subtext constrained to `max-w-xl mx-auto`
- Heading constrained to `max-w-3xl mx-auto`

---

### TYPOGRAPHY DETAILS

| Element | Classes |
|---|---|
| Heading | `text-5xl md:text-6xl lg:text-7xl font-heading italic text-white tracking-tight leading-[0.85] max-w-3xl mx-auto mb-4` |
| Subtext | `text-white/60 font-body font-light text-sm md:text-base max-w-xl mx-auto mb-8` |
| Glass button text | `text-sm font-medium text-white font-body` |
| Solid button text | `text-sm font-medium` (inherits `text-black` from `bg-white text-black`) |
| Copyright | `text-white/40 font-body font-light text-xs` |
| Footer links | `text-white/40 hover:text-white/70 font-body font-light text-xs transition-colors` |

---

### BUTTON DETAILS

**Primary CTA ("Book a Call"):**
`liquid-glass-strong rounded-full px-6 py-3 text-sm font-medium text-white flex items-center gap-2 hover:bg-white/10 transition-all font-body`
- Glass background with gradient border via `::before`
- `ArrowUpRight` icon at `h-5 w-5`

**Secondary CTA ("View Pricing"):**
`bg-white text-black rounded-full px-6 py-3 text-sm font-medium flex items-center gap-2 hover:bg-white/90 transition-colors font-body`
- Solid white background, black text
- `ArrowUpRight` icon at `h-4 w-4` (slightly smaller than the primary)

---

### EXACT TEXT CONTENT

**Heading**: "Your next website starts here."
**Subtext**: "Book a free strategy call. See what AI-powered design can do. No commitment, no pressure. Just possibilities."
**Button 1**: "Book a Call"
**Button 2**: "View Pricing"
**Copyright**: "(c) 2026 Studio. All rights reserved."
**Footer links**: "Privacy", "Terms", "Contact"

---

### PARENT CONTEXT

This section sits on a `bg-black` parent container as the last section of the page. The top gradient fade blends the video into the section above (which also has a black background). The footer bar is part of this same component -- there is no separate footer component.

## Mouse Trail CTA — CTA [sections/mouse-trail-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/mouse-trail-cta.webp

Build a React + TypeScript + Tailwind CSS + Vite section called "Partner with us". It's a large rounded white card with a giant serif heading, a dark pill button containing a portrait avatar, and an interactive mouse-trail effect that drops fading, slightly rotated images wherever the user moves the cursor inside the card.

### Fonts (global, in `src/index.css` before `@tailwind` directives)

```css
@font-face {
  font-family: 'PP Neue Montreal';
  src: url('https://assets.website-files.com/6009ec8cda7f305645c9d91b/60176f9bb43e36419997ecfe_PPNeueMontreal-Book.otf') format('opentype');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'PP Neue Montreal';
  src: url('https://assets.website-files.com/6009ec8cda7f305645c9d91b/60176f9b39c5673e51a86f5a_PPNeueMontreal-Medium.otf') format('opentype');
  font-weight: 500;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'PP Mondwest';
  src: url('/PPMondwest-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  font-family: 'PP Neue Montreal', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

@keyframes fadeInUp {
  0%   { opacity: 0; transform: translateY(30px); }
  100% { opacity: 1; transform: translateY(0); }
}
.animate-fade-in-up {
  animation: fadeInUp 0.8s ease-out forwards;
  opacity: 0;
}
```

Place `PPMondwest-Regular.woff2` in `public/`. Also place a square portrait JPG in `public/` (e.g., `viktor.jpg`) to use as the avatar inside the CTA button.

### In-view hook (`src/hooks/useInViewAnimation.ts`)

`IntersectionObserver` with `threshold = 0.1`. Returns `{ ref, isInView }`. Sticky once true.

### Component (`src/components/PartnerSection.tsx`)

**Props**
- `images: string[]` — a list of image URLs used as the trail. Use the same GIF/animation URLs you have on the rest of the page (any 8+ rectangular assets).

**State / refs**
- `trailImages: TrailImage[]` where `TrailImage = { id: number; x: number; y: number; timestamp: number; src: string; rotation: number }`.
- `isHovered: boolean` (default false).
- `sectionRef` — ref on the inner rounded card (for bounding-rect math).
- `lastSpawnTime` — `useRef(0)` for spawn throttling.
- `imageIdCounter` — `useRef(0)` monotonically increasing id.
- `animationRef`, `isInView` from `useInViewAnimation()` placed on the outer `

`.

**Mouse trail behaviour**
- `onMouseEnter` sets `isHovered = true`; `onMouseLeave` sets it false.
- `onMouseMove`:
  - Bail if `!isHovered` or `!sectionRef.current`.
  - Throttle: ignore if `Date.now() - lastSpawnTime.current < 80` ms.
  - Compute `(x, y)` relative to the section's `getBoundingClientRect()` (subtract `rect.left`, `rect.top`).
  - Pick a random image from `images`. Compute `rotation = (Math.random() - 0.5) * 20` degrees.
  - Push a new `TrailImage` with a fresh id, `timestamp = Date.now()`.
- Cleanup loop: `setInterval` every 50 ms removes any trail entry whose age `> 1000` ms.

**Per-image fade math** (applied as inline style on the trail wrapper):
- `age = now - img.timestamp`, `progress = min(age / 1000, 1)`.
- `opacity = 1 - progress`.
- `scale = 1 - progress * 0.15`.
- Position: `left = img.x - 50`, `top = img.y - 50`.
- Transform: `scale(${scale}) rotate(${img.rotation}deg)`.

**Markup**

Outer wrapper:
```html



```
Inner rounded card (this is the trail surface — attach `sectionRef`, `onMouseMove`, `onMouseEnter`, `onMouseLeave` here):
```
w-full max-w-7xl mx-auto py-48 bg-white relative overflow-hidden
shadow-[0_0_0_0.5px_rgba(0,0,0,0.05),0_4px_30px_rgba(0,0,0,0.08)]
rounded-[40px]
```

Inside the card, a centered content block above the trail:
```html



```

Heading:
```
text-[48px] md:text-[64px] lg:text-[80px] leading-[1.1]
text-[#0D212C] tracking-tight font-normal mb-12
```
- Inline style `fontFamily: "'PP Mondwest', serif"`.
- Text: `Partner with us`.
- Animation: when `isInView`, class `animate-fade-in-up`; else `opacity-0`. Inline `animationDelay: isInView ? '0.1s' : '0s'`.

CTA button (centered with `mx-auto`):
```
bg-[#051A24] text-white px-6 py-3.5 rounded-full
flex items-center gap-3
shadow-[0_1px_2px_0_rgba(5,26,36,0.1),0_4px_4px_0_rgba(5,26,36,0.09),0_9px_6px_0_rgba(5,26,36,0.05),0_17px_7px_0_rgba(5,26,36,0.01),0_26px_7px_0_rgba(5,26,36,0),inset_0_2px_8px_0_rgba(255,255,255,0.5)]
hover:bg-[#0D212C] transition-colors duration-200
```
- Animation: `animate-fade-in-up` (or `opacity-0`) with `animationDelay: '0.2s'`.
- Contents:
  1. `` of the portrait: `w-10 h-10 rounded-full object-cover`, `src` = portrait JPG path, `alt="Viktor Oddy"`.
  2. `Start chat with Viktor`.

After the centered content block, render the trail images list (still inside the card, not behind `z-10`):
```jsx
{trailImages.map((img) => (






))}
```

### Colors used
- Dark text / button bg: `#051A24` (button), `#0D212C` (heading and button hover).
- Card surface: `white`.
- Card shadow: `0 0 0 0.5px rgba(0,0,0,0.05), 0 4px 30px rgba(0,0,0,0.08)`.
- Button shadow (layered + inset highlight): as listed above.

### Behaviour summary
- Section fades in on scroll via `IntersectionObserver` (heading delay 0.1s, button delay 0.2s).
- Hovering the rounded card drops a randomized, rotated thumbnail every ~80 ms at the cursor; each thumbnail fades out and shrinks slightly over 1 second, then is removed.
- Trail thumbnails are 96 px wide (`w-24`), centered on the cursor (offset by 50 px), absolutely positioned, `pointer-events-none`, with `rounded-xl` and a soft `shadow-lg`.

### Required dependencies
`react`, `react-dom`, plus Vite + Tailwind toolchain. No `lucide-react` needed for this section.

---

## Nimbus Ops — CTA [sections/nimbus-ops]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nimbus-ops.webp

---

### Prompt to Recreate the Nimbus Grid Operations Section (Standalone)

Build a standalone single-section page: the "Operations" section from Nimbus Grid — the one with the eyebrow "Operations", heading "A control layer for every storage move your business makes.", and the interactive 3D exploding cube. Use plain HTML, CSS, and vanilla JS (Vite project, no frameworks). Match every detail below exactly.

---

### Global Setup

**Fonts (Google Fonts, preconnect both `fonts.googleapis.com` and `fonts.gstatic.com`):**
- `IBM Plex Sans` weights 400, 500
- `IBM Plex Mono` weights 400, 500

**CSS variables on `:root`:**
```css
:root {
  color-scheme: dark;
  --bg: #17130d;
  --ink: #fff4d5;
  --muted: #dacaa1;
  --line: rgba(255, 240, 199, 0.28);
  --glass: rgba(255, 239, 199, 0.16);
  --glass-strong: rgba(255, 239, 199, 0.24);
  --accent: #ead09a;
  --accent-2: #ffd879;
  --deep: #4d3f24;
  --radius: 8px;
  --font-sans: "IBM Plex Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --font-mono: "IBM Plex Mono", "SF Mono", ui-monospace, monospace;
}
```

**Resets:** Universal `box-sizing: border-box`. `html { scroll-behavior: smooth; }`. Body: `margin: 0`, background `var(--bg)`, color `var(--ink)`, `font-family: var(--font-sans)`, `font-size: 1rem`, `font-weight: 400`, `line-height: 1.375`, `letter-spacing: 0.0175rem`, antialiased. Anchors: `color: inherit; text-decoration: none;`. All headings/paragraphs: `margin-top: 0`.

---

### Color Theme Note

This section uses a **dual-palette**: the left copy column uses the warm gold theme (`var(--accent)`, `var(--ink)`), while the right visual area blends gold and cyan (`rgba(151, 211, 235, ...)`, `rgba(234, 208, 154, ...)`). The background itself mixes both palettes.

---

### Section: `.operations-cubes`

`<section class="operations-cubes" id="operations" aria-labelledby="operations-title">`

**Outer container:**
```css
.operations-cubes {
  position: relative;
  min-height: 96svh;
  padding: clamp(84px, 9vw, 132px) clamp(20px, 5vw, 72px);
  overflow: hidden;
  border-top: 1px solid rgba(255, 240, 199, 0.1);
  background:
    radial-gradient(circle at 74% 42%, rgba(151, 211, 235, 0.16), transparent 25rem),
    radial-gradient(circle at 22% 78%, rgba(255, 216, 121, 0.13), transparent 24rem),
    #0c0d0a;
}
```

Background is a dark olive-black (`#0c0d0a`) with a cyan radial glow at upper-right and a warm gold glow at lower-left.

**Overlay gradient (`::before`) — fogs the left side to make copy readable over the visual:**
```css
.operations-cubes::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background:
    linear-gradient(90deg, rgba(12, 13, 10, 0.94) 0%, rgba(12, 13, 10, 0.68) 42%, rgba(12, 13, 10, 0.08) 100%),
    linear-gradient(180deg, rgba(255, 247, 222, 0.05), transparent 34%);
}
```

This is a left-to-right fog: nearly opaque on the left (where the copy sits), fading to nearly transparent on the right (where the cube visual lives). Plus a very subtle top-to-bottom warm glow.

---

### Layout: `.operations-inner`

```html
<div class="operations-inner">
  <div class="operations-copy">...</div>
  <div class="operations-visual">...</div>
</div>
```

```css
.operations-inner {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: minmax(280px, 0.44fr) minmax(420px, 0.56fr);
  gap: clamp(44px, 7vw, 108px);
  align-items: center;
  width: min(100%, 1320px);
  min-height: calc(96svh - clamp(168px, 18vw, 264px));
  margin-inline: auto;
}
```

Two-column grid: 44% copy on the left, 56% visual on the right. Vertically centered.

---

### Left: `.operations-copy`

```html
<div class="operations-copy">
  <p class="eyebrow">Operations</p>
  <h2 id="operations-title">A control layer for every storage move your business makes.</h2>
  <p>
    Route migrations, active workspaces, archives, and compliance exports through one operational grid.
    Nimbus Grid keeps capacity, policy, and transfer status visible before teams hit a limit.
  </p>
  <a class="operations-cta" href="#plans">Plan operations</a>
</div>
```

```css
.operations-copy h2 {
  max-width: 740px;
  margin-bottom: 26px;
  font-size: clamp(34px, 4.4vw, 72px);
  line-height: 0.98;
}

.operations-copy p:not(.eyebrow) {
  max-width: 560px;
  margin-bottom: 34px;
  color: rgba(255, 244, 213, 0.76);
  font-size: clamp(16px, 1.25vw, 20px);
  line-height: 1.58;
}
```

**CTA button (`.operations-cta`) — gold filled button, dark text:**
```css
.operations-cta {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 46px;
  padding: 0 20px;
  border: 1px solid rgba(255, 247, 222, 0.32);
  border-radius: var(--radius);
  color: #1b160d;
  background: var(--accent);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1rem;
  letter-spacing: 0.04rem;
  text-transform: uppercase;
  transition: background 160ms ease, transform 160ms ease;
}

.operations-cta:hover {
  background: var(--accent-2);
  transform: translateY(-2px);
}
```

This is the only filled/solid CTA on the page — gold background with dark text. On hover it brightens to `--accent-2` and lifts 2px.

---

### Right: `.operations-visual` (the 3D exploding cube)

This is the most complex visual element: a CSS 3D cube with 6 faces, surrounded by particle fragments that explode outward on click.

```html
<div class="operations-visual">
  <button class="modal-cube-shell" type="button" aria-label="Explode storage operations cube">
    <!-- 10 rectangular particles -->
    <span class="cube-particle" style="--tx: -210px; --ty: -132px; --tz: 80px; --s: 0.42; --r: -18deg; --d: 0ms;"></span>
    <span class="cube-particle" style="--tx: -122px; --ty: -188px; --tz: 140px; --s: 0.34; --r: 12deg; --d: 28ms;"></span>
    <span class="cube-particle" style="--tx: 156px; --ty: -170px; --tz: 120px; --s: 0.38; --r: -8deg; --d: 46ms;"></span>
    <span class="cube-particle" style="--tx: 252px; --ty: -88px; --tz: 50px; --s: 0.52; --r: 18deg; --d: 72ms;"></span>
    <span class="cube-particle" style="--tx: -262px; --ty: 4px; --tz: 100px; --s: 0.5; --r: 22deg; --d: 98ms;"></span>
    <span class="cube-particle" style="--tx: -174px; --ty: 104px; --tz: 40px; --s: 0.36; --r: -32deg; --d: 118ms;"></span>
    <span class="cube-particle" style="--tx: 188px; --ty: 86px; --tz: 150px; --s: 0.44; --r: 28deg; --d: 140ms;"></span>
    <span class="cube-particle" style="--tx: 280px; --ty: 162px; --tz: 78px; --s: 0.58; --r: -16deg; --d: 168ms;"></span>
    <span class="cube-particle" style="--tx: -42px; --ty: -228px; --tz: 210px; --s: 0.26; --r: 34deg; --d: 188ms;"></span>
    <span class="cube-particle" style="--tx: 62px; --ty: 224px; --tz: 175px; --s: 0.32; --r: -24deg; --d: 210ms;"></span>

    <!-- 4 dot (circular) particles -->
    <span class="cube-particle dot" style="--tx: -308px; --ty: -92px; --tz: 40px; --s: 0.12; --d: 24ms;"></span>
    <span class="cube-particle dot" style="--tx: 326px; --ty: -8px; --tz: 90px; --s: 0.1; --d: 84ms;"></span>
    <span class="cube-particle dot" style="--tx: -238px; --ty: 198px; --tz: 30px; --s: 0.11; --d: 126ms;"></span>
    <span class="cube-particle dot" style="--tx: 142px; --ty: -246px; --tz: 70px; --s: 0.09; --d: 164ms;"></span>

    <!-- The main cube -->
    <span class="operations-core-cube">
      <span class="cube-face cube-face-front"></span>
      <span class="cube-face cube-face-back"></span>
      <span class="cube-face cube-face-right"></span>
      <span class="cube-face cube-face-left"></span>
      <span class="cube-face cube-face-top"></span>
      <span class="cube-face cube-face-bottom"></span>
    </span>
  </button>
</div>
```

**Custom properties per particle (set via inline style):**
- `--tx`, `--ty`, `--tz`: 3D translation offset when exploded
- `--s`: scale factor when exploded
- `--r`: rotation angle when exploded (rectangular particles only)
- `--d`: stagger delay for the explosion transition

### Visual container

```css
.operations-visual {
  position: relative;
  min-height: min(58vw, 620px);
}

.operations-visual::before {
  content: "";
  position: absolute;
  inset: 8% -14% 0;
  background: radial-gradient(ellipse at center, rgba(234, 208, 154, 0.18), rgba(151, 211, 235, 0.08) 34%, transparent 68%);
  filter: blur(24px);
}
```

A diffuse glow behind the cube — gold at center, fading to cyan, then transparent. Blurred for a soft halo.

### Button shell

```css
.modal-cube-shell {
  position: absolute;
  inset: 0;
  width: 100%;
  min-height: inherit;
  border: 0;
  padding: 0;
  color: inherit;
  background: transparent;
  cursor: pointer;
  perspective: 1000px;
  transform-style: preserve-3d;
  touch-action: manipulation;
  -webkit-tap-highlight-color: transparent;
}

.modal-cube-shell:focus-visible {
  outline: 1px solid rgba(255, 247, 222, 0.72);
  outline-offset: 10px;
}
```

The entire visual area is a `<button>` so the cube is clickable. `perspective: 1000px` enables 3D rendering for children.

### Core cube (6-face CSS cube)

```css
.operations-core-cube,
.cube-particle {
  --cube-size: clamp(142px, 18vw, 250px);
  position: absolute;
  top: 50%;
  left: 50%;
  width: var(--cube-size);
  height: var(--cube-size);
  transform-style: preserve-3d;
}

.operations-core-cube {
  transform: translate(-50%, -48%) rotateX(-16deg) rotateY(34deg) rotateZ(0deg);
  animation: core-cube-float 6s ease-in-out infinite;
  transition: transform 620ms cubic-bezier(0.2, 0.8, 0.2, 1), filter 620ms ease;
  filter: drop-shadow(0 34px 62px rgba(0, 0, 0, 0.48));
}
```

The cube is centered in the visual area and rotated to show 3 faces (front, right, top) via `rotateX(-16deg) rotateY(34deg)`. It gently floats via the `core-cube-float` animation.

**Float animation:**
```css
@keyframes core-cube-float {
  0%, 100% {
    transform: translate(-50%, -48%) rotateX(-16deg) rotateY(34deg) translateY(0);
  }
  50% {
    transform: translate(-50%, -52%) rotateX(-14deg) rotateY(38deg) translateY(-10px);
  }
}
```

Subtle breathing: the cube drifts 10px upward and rotates slightly at midpoint, then returns. 6-second cycle.

**Exploded state (when `.is-exploded` class is on the shell):**
```css
.modal-cube-shell.is-exploded .operations-core-cube {
  transform: translate(-50%, -46%) rotateX(-14deg) rotateY(42deg) rotateZ(0deg) scale(0.72);
  filter: drop-shadow(0 18px 44px rgba(0, 0, 0, 0.38));
  animation: none;
}
```

On click, the core cube shrinks to 72%, rotates slightly further, and stops floating.

### Cube faces

```css
.cube-face {
  position: absolute;
  inset: 0;
  border: 1px solid rgba(255, 247, 222, 0.18);
  border-radius: 18px;
  background:
    radial-gradient(circle at 48% 44%, rgba(255, 216, 121, 0.98) 0 11%, rgba(255, 216, 121, 0.32) 22%, transparent 48%),
    radial-gradient(circle at 18% 15%, rgba(255, 247, 222, 0.92), transparent 30%),
    linear-gradient(135deg, rgba(151, 211, 235, 0.46), rgba(234, 208, 154, 0.78) 38%, rgba(43, 35, 19, 0.92) 100%);
  box-shadow:
    inset 0 2px 6px rgba(255, 247, 222, 0.38),
    inset -22px -28px 36px rgba(0, 0, 0, 0.34),
    inset 18px 12px 28px rgba(151, 211, 235, 0.18);
  opacity: 0.98;
  backface-visibility: hidden;
}
```

Each face has rounded corners (18px), a complex layered background (gold hotspot center, bright highlight top-left, cyan-to-gold-to-dark diagonal gradient), and multiple inset shadows for depth. `backface-visibility: hidden` prevents rendering the back side.

**Face transforms (standard CSS cube):**
```css
.cube-face-front  { transform: translateZ(calc(var(--cube-size) / 2)); }
.cube-face-back   { transform: rotateY(180deg) translateZ(calc(var(--cube-size) / 2)); }
.cube-face-right  { transform: rotateY(90deg) translateZ(calc(var(--cube-size) / 2)); }
.cube-face-left   { transform: rotateY(-90deg) translateZ(calc(var(--cube-size) / 2)); }
.cube-face-top    { transform: rotateX(90deg) translateZ(calc(var(--cube-size) / 2)); }
.cube-face-bottom { transform: rotateX(-90deg) translateZ(calc(var(--cube-size) / 2)); }
```

### Particles (scattered fragments)

There are 14 particles total: 10 rectangular (`cube-particle`) and 4 circular (`cube-particle dot`).

**Default state (collapsed — hidden inside/behind the cube):**
```css
.cube-particle {
  --cube-size: clamp(72px, 8vw, 116px);
  opacity: 0;
  transform: translate(-50%, -48%) rotateX(-16deg) rotateY(34deg) scale(0.12);
  transition:
    opacity 420ms ease var(--d),
    transform 760ms cubic-bezier(0.17, 0.78, 0.18, 1) var(--d),
    filter 760ms ease var(--d);
  filter: blur(4px) brightness(0.7);
}
```

Particles start invisible, tiny (scale 0.12), blurred, and dimmed. Each transition is staggered by `--d` (0ms to 210ms).

**Particle appearance (two pseudo-elements create a layered shard):**
```css
.cube-particle::before,
.cube-particle::after {
  content: "";
  position: absolute;
  width: 100%;
  height: 100%;
  border-radius: 9px;
}

.cube-particle::before {
  background:
    radial-gradient(circle at 48% 44%, rgba(255, 216, 121, 0.9), transparent 36%),
    linear-gradient(135deg, rgba(255, 247, 222, 0.42), rgba(234, 208, 154, 0.76) 42%, rgba(26, 28, 20, 0.88));
  box-shadow: inset 0 1px 4px rgba(255, 247, 222, 0.4), 0 20px 44px rgba(0, 0, 0, 0.36);
}

.cube-particle::after {
  inset: 10%;
  border: 1px solid rgba(255, 247, 222, 0.14);
  background:
    linear-gradient(90deg, rgba(151, 211, 235, 0.28), transparent 42%),
    linear-gradient(180deg, rgba(255, 247, 222, 0.26), transparent 48%);
}
```

`::before` is the main body (gold-highlighted shard). `::after` is a smaller inset overlay (10% padding) with a cyan-to-transparent gradient and a faint border — adding a glass-like inner surface.

**Dot particles (circular):**
```css
.cube-particle.dot {
  --cube-size: clamp(48px, 5vw, 74px);
}

.cube-particle.dot::before,
.cube-particle.dot::after {
  border-radius: 50%;
}
```

Same as rectangular particles but smaller and fully round.

**Exploded state:**
```css
.modal-cube-shell.is-exploded .cube-particle {
  opacity: 1;
  transform:
    translate(-50%, -48%)
    translate3d(calc(var(--tx) * var(--spread, 1)), calc(var(--ty) * var(--spread, 1)), calc(var(--tz) * var(--spread, 1)))
    rotateX(-16deg)
    rotateY(34deg)
    rotateZ(var(--r, 0deg))
    scale(var(--s));
  filter: blur(0) brightness(1);
}
```

On explosion: each particle flies to its `--tx/--ty/--tz` position (multiplied by `--spread` for responsive scaling), rotates by `--r`, scales to `--s`, becomes fully visible, and unblurs. The staggered `--d` delay creates a cascading burst effect.

---

### JavaScript: Toggle explode on click

```js
const operationsCube = document.querySelector(".modal-cube-shell");

if (operationsCube) {
  const toggleCube = () => operationsCube.classList.toggle("is-exploded");

  operationsCube.addEventListener("click", toggleCube);
  operationsCube.addEventListener("keydown", (event) => {
    if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      toggleCube();
    }
  });
}
```

Click or press Enter/Space toggles the `.is-exploded` class. All animation is CSS transition-driven — JS just toggles the class. First click explodes, second click implodes (particles fly back to center).

---

### Responsive Breakpoints

### `@media (max-width: 820px)`

```css
.operations-cubes {
  min-height: auto;
  padding-block: 76px;
}

.operations-inner {
  grid-template-columns: 1fr;
  gap: 42px;
  min-height: 0;
}

.operations-cubes::before {
  background:
    linear-gradient(180deg, rgba(12, 13, 10, 0.9) 0%, rgba(12, 13, 10, 0.62) 54%, rgba(12, 13, 10, 0.18) 100%),
    linear-gradient(180deg, rgba(255, 247, 222, 0.05), transparent 34%);
}

.operations-visual {
  min-height: 460px;
}

.modal-cube-shell {
  --spread: 0.72;
}
```

At 820px: Stacks to single column (copy above visual). The overlay gradient changes from left-to-right to top-to-bottom. Particles spread at 72% of their desktop distance. Visual min-height reduces.

### `@media (max-width: 520px)`

```css
.operations-cubes {
  padding: 64px 18px;
}

.operations-copy h2 {
  font-size: clamp(32px, 9vw, 48px);
}

.operations-copy p:not(.eyebrow) {
  font-size: 15px;
}

.operations-visual {
  min-height: 360px;
}

.modal-cube-shell {
  --spread: 0.48;
}
```

At 520px: Even tighter padding. Heading scales down. Particles spread at 48% distance. Visual shrinks to 360px.

---

### Project structure

```
index.html       (section markup + font links)
styles.css       (all styles + media queries + keyframes)
script.js        (click-to-explode toggle)
package.json     (vite ^5.4.2, "type": "module", scripts: dev/build/preview)
vite.config.js   (default export)
```

No images, no frameworks, no 3D libraries. The entire cube is pure CSS `transform-style: preserve-3d` with 6 positioned faces. Particles are CSS pseudo-elements with staggered transitions. The only JS is a single class toggle.

## Orbis CTA — CTA [sections/orbis-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/orbis-cta.webp

---

> **Prerequisites (fonts, Tailwind config, CSS):**
>
> **Google Fonts** in `<head>`:
> ```html
> <link rel="preconnect" href="https://fonts.googleapis.com" />
> <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
> <link href="https://fonts.googleapis.com/css2?family=Anton&family=Condiment&display=swap" rel="stylesheet" />
> ```
>
> **Tailwind custom config:**
> ```js
> fontFamily: {
>   grotesk: ['Anton', 'sans-serif'],
>   condiment: ['Condiment', 'cursive'],
> },
> colors: {
>   cream: '#EFF4FF',
>   neon: '#6FFF00',
> }
> ```
>
> **CSS class `.liquid-glass`** (glassmorphism container):
> ```css
> .liquid-glass {
>   background: rgba(255, 255, 255, 0.01);
>   background-blend-mode: luminosity;
>   backdrop-filter: blur(4px);
>   -webkit-backdrop-filter: blur(4px);
>   border: none;
>   box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1);
>   position: relative;
>   overflow: hidden;
> }
> .liquid-glass::before {
>   content: '';
>   position: absolute;
>   inset: 0;
>   border-radius: inherit;
>   padding: 1.4px;
>   background: linear-gradient(180deg,
>     rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
>     rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
>     rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
>   -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
>   -webkit-mask-composite: xor;
>   mask-composite: exclude;
>   pointer-events: none;
> }
> ```
>
> **Icons needed:** `Mail`, `Twitter`, `Github` from `lucide-react`.
>
> ---
>
> **Build the following section as a React component using Tailwind CSS:**
>
> A `<section>` with classes `relative overflow-hidden`. Inside it, a single `<div>` with classes `relative w-full`. This wrapper contains two children: the video and the overlay.
>
> ---
>
> **CHILD 1 -- Background video:**
>
> A `<video>` element that is NOT absolutely positioned -- it flows naturally and defines the section's height. Classes: `w-full h-auto block`. Attributes: `autoPlay`, `loop`, `muted`, `playsInline`. Source:
> ```
> https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_055729_72d66327-b59e-4ae9-bb70-de6ccb5ecdb0.mp4
> ```
> `type="video/mp4"`. The video's natural aspect ratio determines the section height -- there is no fixed height.
>
> ---
>
> **CHILD 2 -- Overlay content container:**
>
> A `<div>` absolutely positioned over the video with classes: `absolute inset-0 max-w-[1831px] mx-auto px-4 sm:px-6 md:px-8 flex items-start pt-12 sm:pt-16 md:pt-20 lg:pt-20 -translate-y-[10%] md:-translate-y-[10%] lg:translate-y-0`.
>
> Key details:
> - `absolute inset-0` makes it cover the full video area.
> - `max-w-[1831px] mx-auto` centers the content with the same max-width as the rest of the site.
> - `flex items-start` aligns content to the top.
> - `-translate-y-[10%]` shifts the entire overlay upward by 10% on mobile/tablet to compensate for the video's aspect ratio on smaller screens. On `lg:` it resets to `translate-y-0`.
> - Responsive top padding: `pt-12` (mobile), `sm:pt-16`, `md:pt-20`, `lg:pt-20`.
>
> Inside this overlay is a `<div>` with classes `w-full flex flex-col gap-16 sm:gap-24`. It contains two children: the text block and the social sidebar.
>
> ---
>
> **OVERLAY CHILD A -- Text block:**
>
> A `<div>` with classes `flex justify-end lg:pr-[20%] lg:pl-[15%] relative`. This pushes the text to the right side of the section. On large screens, `lg:pr-[20%]` adds 20% right padding and `lg:pl-[15%]` adds 15% left padding, centering the text block roughly in the right-center area of the video.
>
> Inside it, a `<div>` with classes `text-left max-w-[600px] relative`. This constrains the text width and establishes a positioning context for the accent text. Contains two children:
>
> **The heading `<h2>`:** Classes: `font-grotesk text-[16px] sm:text-[20px] md:text-[30px] lg:text-[60px] font-normal uppercase leading-[1.05] sm:leading-[1] md:leading-[1]`.
>
> The text content is structured as:
> ```jsx
> <span className="inline-block mb-4 sm:mb-6 md:mb-8 lg:mb-12">JOIN US.</span><br />
> REVEAL WHAT'S HIDDEN.<br />
> DEFINE WHAT'S NEXT.<br />
> FOLLOW THE SIGNAL.
> ```
> - "JOIN US." is wrapped in a `<span>` with `inline-block` and responsive bottom margin (`mb-4 sm:mb-6 md:mb-8 lg:mb-12`) to create visual separation from the lines below.
> - The remaining three lines are separated by `<br />` tags.
> - Font sizes scale aggressively: `16px` on mobile up to `60px` on large screens.
>
> **The accent text `<span>`:** Positioned absolutely within the `max-w-[600px]` container. Classes: `font-condiment text-neon text-[17px] sm:text-[24px] md:text-[34px] lg:text-[68px] font-normal normal-case absolute top-[8px] sm:top-[11px] md:top-[18px] lg:top-[37px] left-0 mix-blend-exclusion opacity-90`.
>
> Text content: **"Go beyond"**
>
> Key positioning details:
> - `absolute left-0` -- anchored to the left edge of the text container, aligning with "JOIN US." above.
> - `top-[8px]` on mobile, `sm:top-[11px]`, `md:top-[18px]`, `lg:top-[37px]` -- positions it so it overlaps just below the "JOIN US." line, sitting between "JOIN US." and "REVEAL WHAT'S HIDDEN."
> - `font-condiment` = Condiment cursive font. `normal-case` overrides the parent uppercase.
> - `text-neon` = `#6FFF00`. `mix-blend-exclusion` blends with the video. `opacity-90`.
> - Font sizes match the heading scaling: `17px` mobile, `24px` sm, `34px` md, `68px` lg.
>
> ---
>
> **OVERLAY CHILD B -- Social sidebar (bottom-left):**
>
> A `<div>` with classes `absolute left-[8%] bottom-[12%] sm:bottom-[14%] md:bottom-[16%] lg:bottom-[18%] xl:bottom-[20%]`. This positions the sidebar in the bottom-left area of the video, with the bottom offset increasing at each breakpoint so it stays proportionally placed as the video gets taller.
>
> Inside it, a `<div>` with the `liquid-glass` class and these additional classes: `rounded-[0.5rem] sm:rounded-[1.25rem] p-[0.25rem] sm:p-[0.75rem] md:p-[0.5625rem] lg:p-[0.98rem] w-fit flex flex-col gap-[0.0625rem] sm:gap-[0.125rem] md:gap-[0.09375rem] lg:gap-[0.16rem]`. This creates a vertical glass pill container.
>
> Inside the glass pill are **3 `<button>` elements** stacked vertically. Each button has these classes:
> ```
> w-[14vw] sm:w-[14.375rem] md:w-[10.78125rem] lg:w-[16.77rem]
> h-[1.8vh] sm:h-[3.5rem] md:h-[2.625rem] lg:h-[4.09rem]
> min-w-[3.5rem] sm:min-w-[8rem] md:min-w-[6rem] lg:min-w-[9.33rem]
> min-h-[0.75rem] sm:min-h-[2.5rem] md:min-h-[1.875rem] lg:min-h-[2.92rem]
> flex items-center justify-center hover:bg-white/10 transition-colors
> ```
> - The first two buttons also have `border-b border-white/10` (a subtle white divider line). The third (last) button does NOT have the border.
> - Width uses `14vw` on mobile (viewport-relative) then switches to fixed `rem` values at larger breakpoints.
> - Height uses `1.8vh` on mobile then fixed `rem` values.
> - `min-w` and `min-h` ensure buttons don't collapse too small on tiny screens.
>
> Each button contains one icon from `lucide-react`, in this order top to bottom:
> 1. `<Mail />`
> 2. `<Twitter />`
> 3. `<Github />`
>
> All icons share the same responsive classes: `w-[0.625rem] h-[0.625rem] sm:w-[1.25rem] sm:h-[1.25rem] md:w-[0.9375rem] md:h-[0.9375rem] lg:w-[1.46rem] lg:h-[1.46rem] text-cream`.
> (Icons are `10px` on mobile, `20px` on sm, `15px` on md, `~23px` on lg.)
>
> ---
>
> **There are no animations or keyframes in this section.** The only motion is the autoplaying video. Buttons have `hover:bg-white/10 transition-colors` for a subtle hover effect.

---

## Rocket CTA — CTA [sections/rocket-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/rocket-cta.webp

Build a pixel-faithful recreation of the landing CTA section. Use React 18 + Vite + TypeScript, TailwindCSS, `framer-motion`, `clsx` + `tailwind-merge` as `cn()`. Dark theme background (`#000000`), Inter font, Material Symbols Outlined for icons. Use the white-alpha "landing" palette in `tailwind.config.ts`:

```ts
landing: {
  surface: "rgba(255, 255, 255, 0.10)",
  "surface-hover": "rgba(255, 255, 255, 0.16)",
  border: "rgba(255, 255, 255, 0.10)",
  "border-strong": "rgba(255, 255, 255, 0.20)",
  text: "rgba(255, 255, 255, 0.80)",
  "text-muted": "rgba(255, 255, 255, 0.60)",
}
```

Add a global `.liquid-glass` utility (frosted translucent surface):
```css
.liquid-glass {
  background: rgba(255,255,255,0.06);
  border: 1px solid rgba(255,255,255,0.10);
  backdrop-filter: blur(20px) saturate(140%);
  -webkit-backdrop-filter: blur(20px) saturate(140%);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.08), 0 10px 30px rgba(0,0,0,0.25);
}
```

Add keyframes in `index.css` for the inner Velorah hero animation:
```css
@keyframes fade-rise { from { opacity: 0; transform: translateY(14px); } to { opacity: 1; transform: translateY(0); } }
.animate-fade-rise          { animation: fade-rise .8s ease-out both; }
.animate-fade-rise-delay    { animation: fade-rise .8s ease-out .25s both; }
.animate-fade-rise-delay-2  { animation: fade-rise .8s ease-out .5s both; }
.scrollbar-hide::-webkit-scrollbar { display: none; }
.scrollbar-hide { scrollbar-width: none; }
```

Load fonts in `index.html`: Inter (400/500/600), Instrument Serif (400 + italic), Material Symbols Outlined.

---

### Assets

- Foreground "grass / horizon" PNG that overlays the bottom of the section — load directly from this URL (no local asset):
  `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1780586778/cta-bg_mlwy5s.png`
- CloudFront video URL used **inside** the Velorah dashboard preview (exact):
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131748_f2ca2a28-fed7-44c8-b9a9-bd9acdd5ec31.mp4`

---

### Helper components (must exist)

### `FadeUp` (`framer-motion`)
```tsx
<motion.div
  initial={{ opacity: 0, y: y ?? 24 }}
  whileInView={{ opacity: 1, y: 0 }}
  viewport={{ once: true, amount: 0.3 }}
  transition={{ duration: 0.6, delay, ease: [0.22, 1, 0.36, 1] }}
/>
```

### `MIcon` — Material Symbols Outlined span, supports `name`, `size`, `fill`, `weight`, `grade`, `opticalSize`, applied via `fontVariationSettings: "'FILL' x, 'wght' y, 'GRAD' z, 'opsz' s"`.

### `PrimaryButton` (landing primitive)
- White pill, black text, `rounded-full`, sizes `sm/md/lg` (default `lg`: `h-12 px-9 text-sm font-medium`).
- Class: `inline-flex items-center justify-center rounded-full bg-white/80 hover:bg-white text-black leading-none transition-colors`.
- Wraps children in an `AnimatedText` component that slides current text up and reveals duplicate from below on hover (200–300ms ease).
- Polymorphic via `as="a" | "button"`, default `a`.

### `ChatPanel` (left side of the dashboard mock)
- Vertical flex column inside `rounded-2xl border border-white/10`, background `rgba(8,8,10,0.6)` + `backdrop-filter: blur(24px)`.
- Header row: 28px circular `bg-white/5` with `MIcon name="auto_awesome" size={14}`, then two-line label: **"Vibe Design course"** (text-sm font-medium white) + **"Learn how to build website with AI"** (text-[11px] white/40).
- Messages scroll area (`scrollbar-hide`, `space-y-4`, px-4 py-5). Seed messages:
  1. assistant — "Welcome to the Vibe Design course! I'll guide you through building stunning websites with AI. What would you like to learn first?"
  2. user — "I want to learn how to build a hero section with a cinematic video background using AI."
  3. assistant — "Great choice! In this course, you'll learn how to create full-screen looping videos, liquid glass nav bars, email signups, and manifesto buttons — all with AI assistance. Let's dive in!"
- Message bubbles: `max-w-[85%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed`. User = `bg-white/15 text-white/90` (right-aligned). Assistant = `bg-white/5 text-white/70 border border-white/5` (left).
- Props: `initialScroll?: "top" | "bottom"` (CTA uses `"top"`), `animateMessagesIn?: boolean` (CTA passes `true`, each message wrapped in `FadeUp delay={i * 0.12} y={16}`).
- Input row: `.liquid-glass rounded-2xl` containing a 1-row autosize `<textarea>` (transparent, placeholder "Ask about the course...") + white square send button (`bg-white text-black rounded-xl p-2`) with `MIcon name="arrow_upward" size={16}`. Enter (no shift) sends. Pushing user msg also pushes a canned assistant reply.

### `VelorahHeroPreview` (right side of the dashboard mock)
```tsx
const VIDEO_SRC = "https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131748_f2ca2a28-fed7-44c8-b9a9-bd9acdd5ec31.mp4";
```
- Wrapper: `relative w-full h-full overflow-hidden rounded-2xl`, inline `backgroundColor: "hsl(201 100% 13%)"` (deep teal as the video-loading color).
- `<video autoPlay loop muted playsInline preload="auto">` absolutely positioned, `object-cover`, `z-0`.
- Nav row (`relative z-10 flex items-center justify-between px-3 sm:px-4 md:px-6 py-2 sm:py-3 md:py-4`):
  - Left brand: `Velorah®` with `font-family: 'Instrument Serif', serif`, `text-sm sm:text-base md:text-lg`, `tracking-tight`, `®` as `<sup className="text-[0.5em]">`.
  - Center (hidden < md): `Home` (white) · `Studio` · `About` · `Journal` · `Reach Us` — `text-[9px] lg:text-[10px] text-white/60` with `hover:text-white` on the inactive items.
  - Right: `.liquid-glass rounded-full px-2.5 sm:px-3 py-1 text-[9px] sm:text-[10px] text-white` reading **Begin Journey**.
- Hero block (`flex flex-col items-center text-center px-3 sm:px-4 pt-3 sm:pt-5 md:pt-7 pb-6`):
  - `<h1>` Instrument Serif, `font-normal leading-[0.95] tracking-[-0.03em]`, `text-lg sm:text-2xl md:text-3xl lg:text-4xl max-w-[90%]`, class `animate-fade-rise`. Content: `Where <em class="not-italic text-white/55">dreams</em> rise <em class="not-italic text-white/55">through the silence.</em>`
  - Paragraph `animate-fade-rise-delay text-white/60 text-[9px] sm:text-[11px] md:text-xs leading-relaxed max-w-[80%] sm:max-w-sm md:max-w-md mt-2 sm:mt-3 md:mt-4`: "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work."
  - Pill button `animate-fade-rise-delay-2 liquid-glass rounded-full px-4 sm:px-5 md:px-6 py-1.5 sm:py-2 md:py-2.5 text-[9px] sm:text-[10px] text-white mt-3 sm:mt-4 md:mt-5` reading **Begin Journey**.

### `CtaDashboardMock`
Frame around ChatPanel + VelorahHeroPreview:
```tsx
<div className="liquid-glass w-full max-w-[1100px] aspect-[3/4] sm:aspect-[16/10] lg:aspect-[16/9] rounded-2xl mx-auto overflow-hidden p-2 sm:p-3">
  <div className="grid h-full grid-cols-1 sm:grid-cols-[minmax(220px,320px)_1fr] gap-2 sm:gap-3">
    <div className="min-h-0 hidden sm:block"><ChatPanel initialScroll="top" animateMessagesIn /></div>
    <div className="min-h-0"><VelorahHeroPreview /></div>
  </div>
</div>
```

---

### `CtaSection` — exact layout & behavior

```tsx
const sectionRef = useRef<HTMLElement>(null);
const isMobile = useIsMobile(); // tailwind md breakpoint hook
const { scrollYProgress } = useScroll({ target: sectionRef, offset: ["start end", "end start"] });
const dashboardY = useTransform(scrollYProgress, [0, 1], ["120px", "-120px"]);
const grassY     = useTransform(scrollYProgress, [0, 1], isMobile ? ["80px", "-40px"] : ["200px", "-200px"]);
```

Markup:
```tsx
<section
  ref={sectionRef}
  id="cta"
  className="relative w-full"
  style={{ background: "linear-gradient(to bottom, transparent 0%, #14191E 100%)" }}
>
  <div className="relative mx-auto max-w-[1080px] px-4 sm:px-6 pt-24 sm:pt-32 md:pt-40 pb-[440px] sm:pb-[520px] md:pb-[440px]">
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-8 items-start">
      {/* Left column */}
      <div className="relative z-20 max-w-[400px]">
        <FadeUp delay={1}>
          <h2 className="text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground">
            Learn how can one go from 0 to $11.5k with AI in 60 days.
          </h2>
        </FadeUp>
        <FadeUp delay={0.1}>
          <p className="mt-6 text-landing-text text-base sm:text-lg leading-[1.5] max-w-[380px]">
            Learn to turn your ideas into stunning websites with AI — the same skills agencies charge $5,000 for. Join the UI Rocket training and start building like a pro today.
          </p>
        </FadeUp>
        <FadeUp delay={0.2} className="mt-10">
          <PrimaryButton as="button">Start for free</PrimaryButton>
        </FadeUp>
      </div>
    </div>
  </div>

  {/* Dashboard pinned to right edge, behind grass, parallax Y */}
  <motion.div
    style={{ y: dashboardY }}
    className="absolute top-[440px] sm:top-[460px] md:top-[500px] lg:top-20 left-4 right-4 sm:left-auto sm:-right-[8%] md:-right-[10%] lg:-right-[12%] z-10 sm:w-[85%] md:w-[80%] lg:w-[68%]"
  >
    <CtaDashboardMock />
  </motion.div>

  {/* Foreground grass — in front of dashboard, parallax Y */}
  <motion.img
    src="https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1780586778/cta-bg_mlwy5s.png"
    alt=""
    aria-hidden
    style={{ y: grassY }}
    className="pointer-events-none select-none absolute left-0 right-0 bottom-[-40px] sm:bottom-[-80px] lg:bottom-[-140px] w-full z-30 object-cover"
  />
</section>
```

### Behavior summary
- Section fades from transparent → `#14191E` vertically.
- On scroll, the dashboard translates from `+120px` → `-120px` (parallax up). The grass image translates `+200px` → `-200px` desktop, `+80px` → `-40px` mobile, on top of the dashboard (`z-30` over `z-10`).
- Left column stays static, on top of the grass (`z-20` left text container would otherwise be covered — keep grass at `z-30` to overlap the dashboard while the heading visually sits left of it).
- Headline `Learn how can one go from 0 to $11.5k with AI in 60 days.` enters with a delayed FadeUp; paragraph + button stagger in afterward.
- Mobile (< sm): dashboard stacks below the heading (`top-[440px]`), chat panel hides (`hidden sm:block`), only the Velorah preview shows.
- Button is `PrimaryButton as="button"` reading **Start for free**; hover triggers the AnimatedText slide.

---

### Acceptance checklist
- [ ] Section bg: `transparent → #14191E` linear gradient bottom.
- [ ] Inter font globally; Instrument Serif used in Velorah brand + headline; Material Symbols Outlined for icons.
- [ ] Headline copy is exactly: `Learn how can one go from 0 to $11.5k with AI in 60 days.`
- [ ] Subtext copy is exactly the paragraph above; button label exactly `Start for free`.
- [ ] `CtaDashboardMock` uses `.liquid-glass` frame, aspect `3/4 → 16/10 → 16/9`, ChatPanel left (hidden on mobile) + Velorah right.
- [ ] Velorah `<video>` uses the exact CloudFront URL, `autoPlay loop muted playsInline preload="auto"`, fallback bg `hsl(201 100% 13%)`.
- [ ] Velorah inner copy/animation classes match (`animate-fade-rise`, `-delay`, `-delay-2`).
- [ ] `useScroll` + `useTransform` parallax: dashboard `120 → -120`, grass `200 → -200` (desktop) / `80 → -40` (mobile).
- [ ] Grass image loaded from Cloudinary URL (`https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1780586778/cta-bg_mlwy5s.png`) sits at `z-30` over dashboard (`z-10`), pointer-events-none, full width.
- [ ] FadeUp entrance order: heading (delay 1) → paragraph (0.1) → button (0.2).
- [ ] PrimaryButton: white pill, black text, AnimatedText hover slide.

## Community CTA — CTA Section [sections/community-cta]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/community-cta.gif

Build a single-file HTML community CTA card section with the following exact specifications.
Fonts & Setup

Import DM Sans from Google Fonts with weights 400, 500, 600, 700, 800 (opsz 9..40)
Use a universal *, *::before, *::after reset: box-sizing: border-box, zero margin/padding
Body: DM Sans font family, background #f0f2f7, text color #1e2240, flex column centered, min-height: 100vh, padding 24px 16px

Layout Structure
<section class="cta-section">
  <div class="cta-wrapper">
    <video class="cta-bg" autoplay muted loop playsinline>
      <source src="[VIDEO_URL]" type="video/mp4" />
    </video>
    <div class="cta-content">
      <h2>Subscribe to Our Community</h2>
      <p>Get exclusive access to cutting-edge tech insights, industry trends, and expert advice delivered straight to your inbox. Join our growing community today!</p>
      <div class="cta-form">
        <input type="email" placeholder="Enter your email here" />
        <button type="button">Join Now</button>
      </div>
      <div class="cta-members">
        <div class="cta-avatars">
          <span class="av1"></span>
          <span class="av2"></span>
          <span class="av3"></span>
          <span class="av4"></span>
        </div>
        <p>5,000+ happy members</p>
      </div>
    </div>
  </div>
</section>
Asset URLs
Background video (set on <video class="cta-bg"> source, MP4 type):
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260503_101827_abebfeec-f243-466b-b494-7f6814c0fbbf.mp4
Avatar images (CleanShot CDN, JPEG, used as background-image on .av1–.av4 respectively):
av1: https://media.cleanshot.cloud/media/21620/CG4uIqBDEVKcxvtOnH2si7r1u5ne9QKdmAAj5Ym5.jpeg?Expires=1777825539&Signature=OF0EIwcA6IsXNVbGzc9-3KdoOBpMbtOrzarsjwKbOCM7bpPmnKhA18dnDGmy2sF0g7y2mcwmptgBLWmHJHESlMuQxzUqjuc2kqF75vztQIMdR1DYroQ53P~DB8tGIjNyp4-Wgc5bigDyODIrQMeJfzTlhaPjHIkoZNWZsKtJahOHMh6znpuNRcx-oOUvi1JsHe7ObRI27rz-~qDod8w3XsyzLvsSxdf6dlNdJ9Xo650r1tHtwMyh8QXBu037lYRKYD1qSB9-sA6J0a~Xq7ZxhVKady-BbhWk6sEY0XZO4UqAp1IWuPQESPWyAXJ3PmD5gep0mc7igPVcw5EqqvSBaA__&Key-Pair-Id=K269JMAT9ZF4GZ

av2: https://media.cleanshot.cloud/media/21620/dsPsOuiJtbftO4aFjajygweQMGFKZOJk4ac3ujD1.jpeg?Expires=1777825533&Signature=s~QBv9pAVu7NEuyEuJP3u875TwxED6c1MCZFGrhEyU8Puj7Yt2I3V0DxTNUy0eOSu26RSV5yzrkfl~O7d5zk32X7SqsNesIxA6urpBUrSaU79LxwsQf0TLAeq3-1nSHUSC96Q0OnHAZLpBZ0qbZcKQ8CQCiC9vjcBm~RkDy1mCLlK8SfVsqRMch0yVfYAZcNovalP9jBQ2kesLFu68h~eSbrUzWHhni7t4WQI3V3qNVEZCgzProoMdG~zwq1gBew1KFOrz2MS765pqym0hlIIwPiKQv6BeIYYHpdYXdWpl-ycjhsbW8GqcJkAeDkCmuy~raeqHmVzaNwzDvFZrb2yw__&Key-Pair-Id=K269JMAT9ZF4GZ

av3: https://media.cleanshot.cloud/media/21620/tvzgP1YqhKu4Rj3N1FmqAmZVgNs6jl9gzgmpmUHk.jpeg?Expires=1777825526&Signature=m6W6cOzHlaMAfw4gXnU9Vpvxsko-nOaqqsPwsUNgPbeSjLuKTUaeIsTnbqcA0PZjtr-EX0iI9y4OhuF6p2sFG93diwprvjGKOhlErVlnx-gCoQGk73PKcrlKjelXp4QOg56rbRL79VAVEVvZ8klyh--cH9uZNlo4Z53qP272dSXYQfj3YGWTevKEnwr3p5~sjUWW4-BBSJ7l-~Z5SA~n7W8G-FKm~LVNUqdz633IwnCbwaF7CJtvhlycOjnsJXZdYl0ZesQTRn~yYrblzLL5sAnWEU0NDPeuPwnIdE7z3SDL7xm1SBXetkF~P9or-XFMEsVePC~idOeAYwG2zSxoew__&Key-Pair-Id=K269JMAT9ZF4GZ

av4: https://media.cleanshot.cloud/media/21620/xuWsFI56rqEUeYTmLVTHCoq224TH90PxcBKrsp4N.jpeg?Expires=1777825521&Signature=lLNMvpDUre8-UAsh1mRdGGLOnBaEGY4hcmQjpbCMkwTHK48cfU5OW5RVkmlcJffulFiUhEfB7qkPSFmIJ7vUcg9nU5qa8iHb6~RGpCHSqTbqK6c2LWy3unWA4e~UY3E9Q4tEQ7eewEbIlZscERJK7XtsoPgb9mde5TlLzjp90bTXbJwwSU5dXo6dhrvml5PMmJa8BDUcVIV2a8BCzkw8OzDBQUwWMhammdmGLBMNpRTJbnaNYM4pXgrcABcJ0DMBLkUjCUTtftKNmYM4O32SlRWbZvXY73H2qRUfL0wBwxM0c35gf372hh1tdkoEoixnneCW5TBs79wyS7xV3dF61Q__&Key-Pair-Id=K269JMAT9ZF4GZ
CSS Specifications
.cta-section: width 100%, padding 0 32px 48px
.cta-wrapper: position relative, border-radius 28px, overflow hidden, border 1px solid rgba(13, 36, 72, 0.15), NO box-shadow, background #d8e5f2, min-height 220px, max-width 1150px, margin 0 auto
.cta-bg (the video element): position absolute, inset: 0, width/height 100%, object-fit: cover, z-index 0, opacity 1 (no overlay)
.cta-content: position relative, z-index 1, padding 40px, max-width 560px
.cta-content h2: DM Sans, font-size 26px, weight 700, color #08063C, margin-bottom 12px, line-height 1.2, letter-spacing -0.015em. Text: "Subscribe to Our Community"
.cta-content p: font-size 13.5px, weight 400, color #08063C, line-height 1.65, margin-bottom 28px, max-width 400px
.cta-form: flex with gap 10px, align-items center, flex-wrap wrap, max-width 80%
.cta-form input: flex 1, min-width 200px, padding 13px 22px, border-radius 50px, border 1px solid rgba(195, 210, 235, 0.75), background rgba(255, 255, 255, 0.96), DM Sans 13px, color #08063C, outline none, box-shadow 0 1px 5px rgba(100, 110, 180, 0.07), transitions on box-shadow and border-color (0.2s)

Placeholder: color #b2bbd4, weight 400
Focus: border-color rgba(130, 155, 220, 0.65), box-shadow 0 1px 10px rgba(100, 120, 210, 0.16)

.cta-form button: padding 13px 24px, border-radius 50px, no border, background #f8f9fc, color #08063C, DM Sans 13px weight 600, cursor pointer, white-space nowrap, letter-spacing 0.01em, box-shadow 0 2px 10px rgba(100, 110, 180, 0.15), transitions on background, transform (0.15s), and box-shadow (0.2s)

Hover: background #eef0f5, transform: translateY(-1px), box-shadow 0 4px 16px rgba(100, 110, 180, 0.25)
Active: transform: translateY(0)
Text: "Join Now"

.cta-members: flex, align-items center, gap 10px, margin-top 20px
.cta-avatars: flex container
.cta-avatars span: inline-block, 32×32px, border-radius 50%, no border, margin-left -9px (overlap), overflow hidden, background-size cover, background-position center top, box-shadow 0 1px 4px rgba(0, 0, 0, 0.12), flex-shrink 0

First child: margin-left 0
Apply each avatar URL as background-image to .av1, .av2, .av3, .av4

.cta-members p: font-size 12.5px, weight 600, color #08063C, no margin. Text: "5,000+ happy members"
Mobile breakpoint (max-width: 600px)

.cta-content: padding 24px, max-width 100%
h2: font-size 19px
p: font-size 11.5px, max-width 100%, margin-bottom 16px
.cta-form: flex-direction column, gap 8px, max-width 100%
Form input and button: width 100%, text-align center
.cta-members: flex-wrap wrap, gap 8px
Avatars: 28×28px, margin-left -7px

Animations / Transitions

Input: smooth 0.2s transition on border-color and box-shadow when focused
Button: 0.2s background, 0.15s transform, 0.2s box-shadow — lifts 1px on hover, returns on active
Background video: autoplay, muted, loop, playsinline (no JS animations)

No additional overlays, gradients, or drop shadows on the card itself. The video plays at full opacity directly behind the content.

## Stark Minimal Footer — Footer [sections/stark-minimal-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/stark-minimal-footer.webp

---

Build a **Site Footer** for an aerospace company called "EngineTech." This is a black-background footer with an animated dotted top border, a four-column nav grid with a large heading, an oversized wordmark brand row, and a legal line.

---

### ROOT CONTAINER (`.site-footer`)

- Position relative, z-index 100, overflow hidden.
- Background: `#000000`. Color: `#ffffff`.

---

### ANIMATED DOTS STRIP (`.footer-dots`)

A decorative band at the very top of the footer with horizontally drifting dots.

- Position relative, height 120px, overflow hidden, background `#000000`.
- Has `aria-hidden="true"`.

**Inside (`.footer-dots__line`):**

- Position absolute, `left: 0; top: 50%`. Width **200%**, height 70px.
- Opacity 0.75. `transform: translateY(-50%)`.
- Background-image (three layered radial-gradient dot patterns):
  - `radial-gradient(circle, rgb(255 255 255 / 0.55) 1.5px, transparent 2px)`
  - `radial-gradient(circle, rgb(255 255 255 / 0.35) 1px, transparent 1.5px)`
  - `radial-gradient(circle, rgb(255 255 255 / 0.45) 1.2px, transparent 1.8px)`
- Background-position: `0 8px, 24px 22px, 48px 14px`.
- Background-size: `72px 38px, 110px 44px, 160px 52px`.
- Animation: `footerDotsMove 18s linear infinite`.

**Keyframes:**

```
@keyframes footerDotsMove {
  from { transform: translate3d(0, -50%, 0); }
  to   { transform: translate3d(-50%, -50%, 0); }
}
```

---

### FOOTER INNER (`.site-footer__inner`)

- Width: `min(100% - 96px, var(--hero-max-width))` where `--hero-max-width: 1820px`. Margin `0 auto`.
- Padding: `clamp(34px, 4vw, 66px) 0 clamp(18px, 2vw, 34px)`.

---

### TOP GRID (`.site-footer__top`)

- Display grid. Columns: `minmax(320px, 1.25fr) repeat(3, minmax(150px, 0.42fr))`.
- Gap: `clamp(28px, 4vw, 76px)`. Min-height: `clamp(220px, 24vw, 330px)`.

### H2 (first cell)

- Text: "Proven Advanced Propulsion Technology".
- Max-width 680px, margin 0, color `#ffffff`, `font-size: clamp(34px, 3.5vw, 62px)`, weight 220, letter-spacing 0, line-height 1.06.

### Nav columns (three `.site-footer__nav` elements)

Each nav is `display: flex; flex-direction: column; align-items: flex-start; gap: clamp(14px, 1.35vw, 22px)`.

Each link `<a>`:
- Color `rgb(255 255 255 / 0.88)`, font-size 16px, weight 650, line-height 1.1.
- Transition: `color 180ms ease, transform 180ms ease`.
- Hover: color `#ffffff`, `transform: translateX(3px)`.

**Nav 1 (`aria-label="Footer navigation"`):**
- Company → `#company`
- Technology → `#technology`
- Solutions → `#solutions`
- Our Edge → `#our-edge`
- Investors → `#investors`

**Nav 2 (`aria-label="Company links"`):**
- Our Team → `#our-team`
- News → `#news`
- Careers → `#careers`
- Contact Us → `#contact`

**Nav 3 (`aria-label="Social links"`):**
- LinkedIn → `https://www.linkedin.com` (`target="_blank" rel="noreferrer"`)
- Follow Us on X → `https://x.com` (`target="_blank" rel="noreferrer"`)

---

### BRAND ROW (`.site-footer__brand-row`)

- Width 100%. Margin-top: `clamp(18px, 3vw, 46px)`.

**Brand link (`.site-footer__brand`):**

- Anchor `href="/"`, `aria-label="EngineTech home"`.
- Display flex, align-items center, width 100%, color `#ffffff`.

**Brand mark (`.site-footer__mark`):**

- Position relative, `flex: 0 0 clamp(58px, 6.1vw, 118px)`, `aspect-ratio: 1`.
- Margin-right `clamp(14px, 1.6vw, 28px)`. Overflow hidden, border-radius 50%.
- Background `#ffffff`.
- `::before` pseudo: absolute `inset: -18%`, background `#000000`, with `clip-path: polygon(0 20%, 100% 8%, 100% 19%, 0 31%, 0 43%, 100% 31%, 100% 42%, 0 54%, 0 66%, 100% 54%, 100% 65%, 0 77%)`. This creates a zig-zag wave pattern inside the white circle.
- Has `aria-hidden="true"`.

**Brand wordmark (second `<span>`):**

- Text: "EngineTech".
- Display block, `flex: 1 1 auto`, min-width 0.
- `font-size: clamp(58px, 11.1vw, 214px)`. Weight 760. `letter-spacing: -0.055em`. Line-height 0.78.
- `white-space: nowrap`.

---

### LEGAL LINE (`.site-footer__legal`)

- Flex row, wrap allowed, justify-content flex-start, gap `8px 18px`.
- Margin-top: `clamp(14px, 1.4vw, 24px)`.
- Color `rgb(255 255 255 / 0.52)`, font-size 9px, line-height 1.35.

Contents:
- `<p>`: "© 2026 EngineTech. All rights reserved." (margin 0)
- `<a href="#privacy">`: "Privacy Policy" (color inherit, hover `#ffffff`)
- `<a href="#terms">`: "Terms of Use" (same styling)

---

### RESPONSIVE BREAKPOINTS

**At 980px:**

- `.site-footer__inner` width: `min(100% - 48px, var(--hero-max-width))`.
- Top grid: `grid-template-columns: 1fr 1fr` (two columns).
- H2 spans full width: `grid-column: 1 / -1`.

**At 560px:**

- `.site-footer__inner` width: `min(100% - 32px, var(--hero-max-width))`.
- Top grid: single column (`grid-template-columns: 1fr`). Min-height auto.
- Nav links font-size 15px.
- Brand mark flex-basis: `clamp(38px, 12vw, 58px)`.
- Brand wordmark font-size: `clamp(45px, 18vw, 84px)`.

---

### GLOBAL STYLES

**CSS custom property used:** `--hero-max-width: 1820px`.

**Font stack:** `"Geist", "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` with `-webkit-font-smoothing: antialiased` and `text-rendering: geometricPrecision`.

**Anchor reset:** `a { color: inherit; text-decoration: none; }`.

**Color palette:** No purple or violet. Pure black `#000000` background, pure white `#ffffff` text, with `rgb(255 255 255 / 0.88)` for nav links, `rgb(255 255 255 / 0.55)` / `0.45` / `0.35` for the dot pattern, and `rgb(255 255 255 / 0.52)` for legal text.

## HAUL! — Footer Section [sections/haul-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/haul-footer.png

Build a React functional component using Tailwind CSS, `motion/react` for animations, and `lucide-react` for icons.

**1. Typography & Setup:**
- Import the "Inter" font from Google Fonts (weights 400, 500, 600, 700) and set it as the default sans-serif font in the Tailwind config/CSS.
- The overall background of the page should be `#f8f9fa`.

**2. Top Spacer Section (View Below):**
- Create a section at the top of the page. Height should be `50vh` (on mobile/lg) and `30vh` (on md screens).
- Background color: `#FDFDFD`.
- Center a text element that says "View Below". The text should be `text-gray-300`, small font, bold, uppercase, with wide `tracking-[0.5em]`.
- Animate this text with Framer Motion to fade in from `opacity: 0` to `opacity: 1`.

**3. Main Parallax Container:**
- Below the spacer, create a main full-viewport-height (`h-screen`) section.
- Set its background image to: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260430_115327_3f256636-9e63-4885-8d0b-09317dc2b0a5.png&w=1280&q=85`
- Make sure the background covers the container (`bg-cover bg-center`) and set `overflow-hidden` with `relative` positioning.
- Set up a Framer Motion `useScroll` target on this container. Map the `scrollYProgress` from `[0, 1]` to `[-50, 150]` using `useTransform`. Apply this transformed y-value to the foreground truck image layer (described below).

**4. The Top-Aligned Footer Card:**
- Position a container `absolute top-0 w-full` inside the main parallax section. Give it top padding (`pt-12` mobile/lg, `pt-24` tablet).
- Inside, create a card constrained to `max-w-7xl mx-auto`.
- Card Styling: `bg-white/95`, `backdrop-blur-sm`, `shadow-xl`, rounded corners (`rounded-2xl` mobile, `rounded-3xl` desktop), `overflow-hidden`.
- Animation: The card should slide down and fade in (`initial={{ opacity: 0, y: -20 }}`, `animate={{ opacity: 1, y: 0 }}`, duration 0.8s easeOut).
- **Footer Content (Top Half):**
  - Use a flex row layout (flex-col on mobile, flex-row on md+) with spread space.
  - **Logo Area**: Include an orange square (`bg-orange-500`, 40x40px mobile, 48x48px desktop, rounded-lg, shadow-inner, p-2). Inside the square, place an SVG with viewBox "0 0 256 256" and this exact white path: `d="M 228 0 C 172.772 0 128 44.772 128 100 L 128 0 L 0 0 L 0 28 C 0 83.228 44.772 128 100 128 L 0 128 L 0 256 L 28 256 C 83.228 256 128 211.228 128 156 L 128 256 L 256 256 L 256 228 C 256 172.772 211.228 128 156 128 L 256 128 L 256 0 Z"`. Next to the logo block, add the text "HAUL!" (`text-gray-900`, 2xl/3xl, font-bold, tracking-tighter).
  - **Links Area**: Display 3 columns of links using flex. Layout: `Company` (Founding, Platform, Testify), `Mobile` (Get Apple App, Get Google App), `Contracts` (Private Data, User Consent). Section headers should be uppercase, tracking-widest, text-sm, bold. Link items should be gray-500, font-medium, and hover to `orange-600` with transition.
- **Footer Content (Bottom Bar):**
  - Add a top border (`border-gray-100`) and use a solid white background (`bg-white`).
  - Layout: flex, space between, aligning text to the left and social icons to the right.
  - Text: "© 2026 HAUL! All Rights Reserved" (text-sm, gray-500, medium).
  - **Social Icons**: Map through an array of icons imported from `lucide-react`: Facebook, Twitter, Instagram, Linkedin (w-5 h-5). Wrap them in `a` tags shaped as 40x40px circles with `border-gray-100`. On hover, they should turn `bg-orange-500` with white text and an `orange-500` border (transition all duration-300).

**5. Background Truck Parallax Layer:**
- Add a `motion.div` placed absolutely at the bottom of the container (`absolute inset-x-0 bottom-0 h-full`).
- Add standard pointer-events-none and z-20.
- Ensure the `y` axis style is tied to the `useTransform` created in step 3 so it scrolls at a different speed than the background.
- Inside, place an image with `src="https://roof-wish-40038865.figma.site/_components/v2/f31fd17907ce60745d45e83a61d44fd3810d5f25/truck_1.8c4bff83.png"`.
- Image styling: `w-full h-full object-contain object-bottom origin-bottom`. Add scale responsive classes (`scale-[1.5]` mobile, `scale-110` sm, `scale-[2.0]` md, `scale-105` lg) to ensure the truck fits properly on various screen widths.

## Kresna Footer — Footer Section [sections/kresna-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/kresna-footer.gif

Build a single-file HTML footer component called Kresna — a sales-automation SaaS brand. The deliverable is one self-contained .html file with inline <style> and inline <script>. Render it inside a <section class="footer-section"> on a white page (body { background: #ffffff; padding: 48px 24px; }).
Fonts
Load from Google Fonts in the <head>:

DM Sans — weights 400, 500, 600, 700 (body, nav links, buttons, headings, watermark)
Caveat — weights 500, 600, 700 (handwritten accents: "Stay in touch!", "Feeling lucky?", column titles "Navigation"/"Company")

Default body font: 'DM Sans', sans-serif. Body color #2d3148.
Use *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }.
Layout structure
A .footer-wrapper with max-width: 1150px, centered, CSS grid grid-template-columns: 350px 1fr, gap: 16px, align-items: stretch. Two cards side by side:
Left card — .footer-left (video background)

Position relative, min-height: 340px, border-radius: 28px, padding: 32px, overflow: hidden
Box shadow: 0 12px 40px rgba(21, 76, 189, 0.25)
Fallback background: #1e4fc0
Flex column, justify-content: space-between
Contains, in order:

A <video class="footer-left-video" autoplay muted loop playsinline preload="auto"> with <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260503_104800_bc43ae09-f494-43e3-97d7-2f8c1692cfd7.mp4" type="video/mp4" />. Style: position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: 0; pointer-events: none;. No overlays, no tints, no noise texture.
.footer-logo — flex row, gap: 10px, position: relative; z-index: 1. Contains:

.footer-logo-mark — a 32×32 rounded square (border-radius: 8px), background: rgba(255,255,255,0.15), border: 1.5px solid rgba(255,255,255,0.85), centered bold "K" letter inside (DM Sans, 16px, weight 700, white, letter-spacing: -0.02em)
<span class="footer-logo-name">Kresna</span> — DM Sans, 22px, weight 700, white, letter-spacing: -0.02em


.footer-tagline-container — margin-top: auto; margin-bottom: 28px, z-index: 1. Contains .footer-tagline (19px, weight 400, white, line-height: 1.45) with text:



     Smarter sales automation,<br>
     <span>powered by AI.</span>
 The inner `<span>` uses `color: rgba(255, 255, 255, 0.65)`.
4. .footer-social-row — flex row, justify-content: space-between, align-items: center, gap: 12px, z-index: 1. Contains:
- .footer-social-label — Caveat, 17px, weight 600, color rgba(255,255,255,0.9), letter-spacing: 0.3px, text: "Stay in touch!"
- .footer-social-icons — flex row, gap: 7px. Four .social-icon divs, each 36×36, border-radius: 9px, background: #0e1014, centered 15×15 white SVG, box shadow 0 6px 18px rgba(0,0,0,0.35), 0 2px 6px rgba(0,0,0,0.2). Hover: background: #000, transform: translateY(-2px), deeper shadow, transition: background 0.2s, transform 0.15s, box-shadow 0.2s. Icons in order: Discord, X (Twitter), LinkedIn, GitHub — use the official brand path d= strings for each in a <svg viewBox="0 0 24 24">.
Right card — .footer-right (light gray)

background: #f0f1f5, border-radius: 28px, padding: 40px, overflow: visible, box-shadow: 0 4px 20px rgba(0,0,0,0.04)
Flex column, justify-content: space-between, position relative
Contains:

Floating "Feeling lucky?" badge — .footer-lucky-graphic
Absolutely positioned, top: -36px; right: 40px, z-index: 10, flex column, align-items: flex-start, gap: 6px. Overflows above the top edge of the card.

.lucky-cube — 96×96, border-radius: 22px, transform: rotate(-10deg), gradient linear-gradient(135deg, #5b9ffb 0%, #1e5dd7 55%, #1448be 100%), layered shadows:

  inset 3px 3px 8px rgba(255,255,255,0.35),
  inset -3px -3px 12px rgba(0,0,0,0.18),
  8px 14px 28px rgba(20,72,200,0.35)
Inside, a <span class="lucky-cube-mark">K</span>: DM Sans, 42px, weight 700, white, letter-spacing: -0.04em, transform: rotate(10deg) (counter-rotates the cube), text-shadow: 0 3px 6px rgba(0,0,0,0.25), line-height: 1.

.lucky-text-row — flex row, gap: 6px, align-items: center, transform: rotate(-4deg), margin-top: 4px. Contains:

.lucky-arrow — 22×22 inline SVG, color: #9ca3af. SVG content: a curved hand-drawn arrow:



html    <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
      <path d="M3 20 C 6 14, 10 9, 18 5" />
      <path d="M18 5 L 12 5" />
      <path d="M18 5 L 18 11" />
    </svg>
SVG paths: `stroke: currentColor; fill: none; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round`.

.lucky-text — Caveat, 20px, weight 600, color #9ca3af, white-space: nowrap, text: "Feeling lucky?"

Top — .footer-right-top with .footer-nav-cols
Flex row, gap: 72px, padding-top: 8px. Two .footer-col columns:

Column titles (.footer-col-title): Caveat, 24px, weight 600, italic, color #9ca3af, margin-bottom: 18px
Links (.footer-col a): block, DM Sans, 14px, weight 600, color #111827, margin-bottom: 14px, no underline, hover color #1f65d6, transition: color 0.2s

Column 1 — title "Navigation", links: How it works, Features, Pricing, Testimonials, FAQ
Column 2 — title "Company", links: Blog, About, Terms and Condition, Privacy Policy
Bottom — .footer-bottom
Flex row, align-items: flex-end, justify-content: space-between, margin-top: 48px. Contains:

.footer-copyright — DM Sans, 12.5px, weight 500, color #9ca3af, text: "© 2025 Kresna. All rights reserved."
.footer-cta-mini — flex column, gap: 14px, contains:

<h4> — 15px, weight 400, color #6b7280, line-height: 1.45, with text:



    AI moves fast.<br><strong>Stay ahead with Kresna.</strong>
The `<strong>` is block-level, 19px, weight 700, color `#111827`.

.footer-subscribe-row — flex row, width: 310px, background: #fff, border: 1px solid #e5e7eb, border-radius: 12px, padding: 5px, box-shadow: 0 2px 10px rgba(0,0,0,0.04). Contains:

<input type="email" placeholder="Enter email address"> — flex 1, padding: 11px 14px, transparent, no border, DM Sans 13.5px, color #111827, placeholder #9ca3af
<button type="button">Subscribe</button> — padding: 11px 22px, background: #111214, white text, DM Sans 13.5px weight 600, border-radius: 8px, shadow 0 6px 20px rgba(0,0,0,0.28), 0 2px 8px rgba(0,0,0,0.15). Hover: background: #000, deeper shadow, transform: translateY(-1px), transition: background 0.2s, box-shadow 0.2s, transform 0.15s.



Watermark — .footer-watermark (sits outside .footer-wrapper but inside the section)
A massive faded "Kresna" wordmark that scales fluidly to the full footer wrapper width with the visible glyph edges flush against the container edges.
CSS:
css.footer-watermark {
  max-width: 1150px;
  margin: -60px auto 0;
  pointer-events: none;
  user-select: none;
  position: relative;
  z-index: 0;
  line-height: 0;
}
.footer-watermark svg {
  display: block;
  width: 100%;
  height: auto;
  overflow: visible;
}
.footer-watermark text {
  font-family: 'DM Sans', sans-serif;
  font-weight: 700;
  letter-spacing: -0.03em;
  fill: rgba(0, 0, 0, 0.04);
}
HTML:
html<div class="footer-watermark" aria-hidden="true">
  <svg id="watermarkSvg" viewBox="62 95 876 175" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
    <text id="watermarkText" x="500" y="240" text-anchor="middle" font-size="320">Kresna</text>
  </svg>
</div>
Inline JS at the end of the section measures the rendered text bounding box with getBBox() and updates the SVG viewBox so the visible glyph edges sit flush against the container — runs after document.fonts.ready and on resize:
html<script>
  function fitWatermark() {
    const svg = document.getElementById('watermarkSvg');
    const text = document.getElementById('watermarkText');
    if (!svg || !text) return;
    try {
      const bbox = text.getBBox();
      svg.setAttribute('viewBox',
        `${bbox.x} ${bbox.y} ${bbox.width} ${bbox.height}`);
    } catch (e) {}
  }
  if (document.fonts && document.fonts.ready) {
    document.fonts.ready.then(fitWatermark);
  } else {
    window.addEventListener('load', fitWatermark);
  }
  window.addEventListener('resize', fitWatermark);
</script>
Responsive breakpoints
@media (max-width: 860px):

.footer-wrapper becomes grid-template-columns: 1fr
.footer-left min-height: auto, gap: 40px

@media (max-width: 560px):

.footer-right padding: 24px
.footer-nav-cols gap: 40px
.footer-bottom flex-direction: column, align-items: flex-start, gap: 24px
.footer-subscribe-row width: 100%
.footer-lucky-graphic right: 12px, top: -28px
.lucky-cube width: 72px, height: 72px
.lucky-cube-mark scaled proportionally if needed

Animations / transitions
No keyframe animations. All motion is hover-driven via CSS transition:

Social icons: background, transform, box-shadow on hover
Subscribe button: background, box-shadow, lift on hover
Nav links: color shift on hover

The video on the left card autoplays, loops, muted, plays inline (no controls).
Final markup order inside <section class="footer-section">
<section class="footer-section">
  <div class="footer-wrapper">
    <div class="footer-left"> [video, logo, tagline, social row] </div>
    <div class="footer-right"> [floating lucky badge, nav cols, bottom row] </div>
  </div>
  <div class="footer-watermark"> [SVG] </div>
  <script> [fitWatermark] </script>
</section>

## Lumina — Footer Section [sections/lumina-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/lumina-footer.gif

Create a React frontend using Tailwind CSS v4, the `motion/react` library for animations, and `lucide-react` for icons. I want to build a page with an immersive video background and a highly stylized "liquid glass" footer.

Please follow these exact specifications:

1. Global CSS & Fonts (`index.css`):
Add the following exact `@font-face` to the CSS file and set it as the root Tailwind `--font-sans`:
@font-face {
    font-family: "Helvetica Regular";
    src: url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.eot");
    src: url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.eot?#iefix")format("embedded-opentype"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.woff2")format("woff2"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.woff")format("woff"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.ttf")format("truetype"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.svg#Helvetica Regular")format("svg");
}

2. The "Liquid Glass" CSS:
Add this exact custom CSS for the liquid glass effect bordering:
.liquid-glass {
  background: rgba(255, 255, 255, 0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  border: none;
  box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1);
  position: relative;
  overflow: hidden;
}
.liquid-glass::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  padding: 1.4px;
  background: linear-gradient(180deg,
    rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
    rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}

3. Main App Structure (`App.tsx`):
- Wrap the page in a `<main>` with `relative w-full min-h-[115vh] overflow-x-hidden flex flex-col items-center font-sans selection:bg-white/20 selection:text-white`.
- Add a `<video>` element fixed to the background (`fixed inset-0 w-full h-full object-cover z-[0]`) that auto-plays, loops, and is muted.
- The `src` for the video must be exactly this CloudFront URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260429_114316_1c7889ad-2885-410e-b493-98119fee0ddb.mp4`

4. Content Wrapper:
On top of the video (`z-10`), add a `max-w-7xl` container that holds an upper CTA (you can use a placeholder for the CTA) and pushes the footer to the bottom.

5. The Footer (`motion.footer`):
- Start it with these exact Framer Motion props: `initial={{ opacity: 0, y: 40 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 1, delay: 0.4, ease: "easeOut" }}`
- Give it the classes: `liquid-glass w-full rounded-3xl p-6 md:p-10 text-white/70 mt-32 md:mt-64`.

6. Footer Layout - Top Grid:
- A 12-column grid (`grid-cols-1 md:grid-cols-12 gap-10 md:gap-12 mb-10`).
- First column (md:col-span-5):
  - An SVG Logo `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 256 256" fill="currentColor"><path d="M 4.688 136 C 68.373 136 120 187.627 120 251.312 C 120 252.883 119.967 254.445 119.905 256 L 0 256 L 0 136.096 C 1.555 136.034 3.117 136 4.688 136 Z M 251.312 136 C 252.883 136 254.445 136.034 256 136.096 L 256 256 L 136.095 256 C 136.032 254.438 136.001 252.875 136 251.312 C 136 187.627 187.627 136 251.312 136 Z M 119.905 0 C 119.967 1.555 120 3.117 120 4.688 C 120 68.373 68.373 120 4.687 120 C 3.117 120 1.555 119.967 0 119.905 L 0 0 Z M 256 119.905 C 254.445 119.967 252.883 120 251.312 120 C 187.627 120 136 68.373 136 4.687 C 136 3.117 136.033 1.555 136.095 0 L 256 0 Z" /></svg>` along with the text "LUMINA" (text-xl font-medium).
  - A description below it: "Lumina provides premium clarity on global events and cosmic wonders - shared with all for free." (`text-sm leading-relaxed max-w-sm`).

7. Footer Layout - Links Section (md:col-span-7):
Make a 3-column grid containing these lists:
- Discover: Labs & Workshops, Deep Dive Series, Global Circle, Resource Vault, Future Roadmap
- The Mission: Origin Story, The Collective, Newsroom Hub, Join the Team
- Concierge: Get in Touch, Legal Privacy, User Agreement, Report Concern
(Headers should be `text-sm uppercase tracking-wider text-white font-medium mb-4` and links `text-xs space-y-2 hover:text-white transition-colors`).

8. Footer Layout - Bottom Bar:
- Create a bottom border (`pt-6 border-t border-white/10 flex flex-col md:flex-row items-center justify-between gap-6 md:gap-4`).
- Left side: `<p className="text-[10px] uppercase tracking-widest opacity-50">Curated by @GotInGeorgiG</p>`
- Right side: A label `<span className="text-[10px] uppercase tracking-widest opacity-50">Join the Journey:</span>` alongside a horizontal flex row of `lucide-react` icons (sizes 16): Music2, Facebook, Twitter, Youtube, and Instagram. Wrap each in an `<a>` with `opacity-70 hover:opacity-100 transition-colors hover:text-white`.

## Vize Footer — Footer Section [sections/vize-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/vize-footer.png

Build a highly polished, responsive Footer component for a React application using Vite, Tailwind CSS, `lucide-react` for icons, and `motion/react` for animations.

The design relies on a premium "layered card" aesthetic, precise typography, and a massive background-blended text element utilizing advanced, handcrafted SVG filters.

### 1. Dependencies
Ensure the project has:
`npm install lucide-react motion`

### 2. Global CSS (`src/index.css`)
Use the exact following CSS to define the Inter font, the Tailwind layer, and advanced `glass-card` and `liquid-glass` utilities:
```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
}

@layer utilities {
  .glass-card {
    background: rgba(255, 255, 255, 0.4);
    backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.5);
    box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.05);
  }

  .text-glass {
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.3) 0%, rgba(255, 255, 255, 0.1) 100%);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.2);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  .liquid-glass {
    background: rgba(255, 255, 255, 0.01);
    background-blend-mode: luminosity;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    border: none;
    box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1);
    position: relative;
    overflow: hidden;
  }

  .liquid-glass::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    padding: 1.4px;
    background: linear-gradient(180deg,
      rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
      rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
      rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    pointer-events: none;
  }
}

body {
  @apply bg-[#F9F9FB] text-[#141414] font-sans antialiased;
}
3. Application Layout (src/App.tsx)
Render the layout wrapper mimicking a full-screen application view exactly like this:
code
Tsx
import Footer from './components/Footer';

export default function App() {
  return (
    <div className="min-h-screen md:h-screen bg-[#F0F1F3] flex flex-col items-center justify-start md:justify-center overflow-y-auto md:overflow-hidden pt-8 md:pt-0 p-4">
      <Footer />
    </div>
  );
}
4. The Footer Component (src/components/Footer.tsx)
Create this file and structure it strictly with the following inner components and specific Tailwind dimensions/hex codes:
Component 1: LogoIcon
Render a square icon box.
Classes: w-8 h-8 bg-[#31A8FF] rounded-[8px] flex items-center justify-center
SVG Code: <svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"> <path d="M4 20C4 20 4 14 10 10C16 6 20 4 20 4C20 4 18 8 14 14C10 20 4 20 4 20Z" fill="white" /> <path d="M4 20L10 14" stroke="white" strokeWidth="2" strokeLinecap="round" /> </svg>
Component 2: FooterCard
A massive layered card layout holding the footer directories.
Wrappers:
Main Container: w-full max-w-6xl mx-auto
Outer Gray Body: bg-[#E9EBEE] rounded-[48px] border border-slate-200 shadow-sm overflow-hidden
Inner White Box: bg-white rounded-[40px] m-2 shadow-sm
Content Grid Space (Inside White Box): p-8 md:p-10 lg:p-12 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-12
Grid Columns Layout:
Brand Info (lg:col-span-2 space-y-8):
A row (flex items-center gap-2.5) with <LogoIcon /> and <span className="text-[26px] font-bold tracking-tight text-[#0F172A]">vize</span>
Description: <p className="text-[#64748B] leading-relaxed text-[16px] font-normal max-w-[320px]">Premium strategic solutions designed to elevate your brand presence through advanced marketing.</p>
Socials Group: Map an array of Linkedin, Twitter, Instagram (imported from lucide-react). Make them buttons with classes: w-[44px] h-[44px] flex items-center justify-center rounded-xl border border-slate-100 bg-white shadow-[0_1px_2px_rgba(0,0,0,0.05)] hover:bg-slate-50 transition-all active:scale-95 group. Inside each put the Icon component with className="w-5 h-5 text-slate-800".
Product Column (space-y-6): Header <h4 className="text-[14px] font-medium text-[#94A3B8]">Product</h4>. Links (href="#" target): Features, Solutions, Pricing, Updates. Styling for links: text-[15px] font-medium text-[#1E293B] hover:text-[#31A8FF] transition-colors. Keep in a <ul> with space-y-4.
Science Column (space-y-6): Header Science. Links: Approach, Identity, Research, Metrics. Same link styling.
Company Column (space-y-6): Header Company. Links: About Us, Partners, Careers. Same link styling.
Bottom Legal Bar (Inside Gray Outer Wrap, OUTSIDE of White Box):
Container: px-6 sm:px-12 md:px-16 lg:px-20 py-5 flex flex-col md:flex-row justify-between items-center gap-6 text-[15px]
Left side: <p className="text-[#64748B] font-medium">© 2025 Vize. All rights reserved.</p>
Right side: Flex row (gap-8 text-[#64748B] font-medium items-center) featuring:
<a href="#" className="hover:text-[#1E293B] transition-colors">Legal Center</a>
Vertical Separator: <div className="w-[1px] h-4 bg-slate-300" />
<a href="#" className="hover:text-[#1E293B] transition-colors">User Agreement</a>
Component 3: GlassText
This must be perfectly implemented to work. It uses an absolute hidden SVG defining a filter, paired with Framer Motion.
Container: relative w-full flex items-center justify-center select-none pt-0.
Invisible SVG: <svg className="absolute w-0 h-0" aria-hidden="true" focusable="false">
Filter setup within SVG:
code
Xml
<defs>
  <filter id="glass-effect" x="-50%" y="-50%" width="200%" height="200%">
    <feDropShadow dx="0" dy="4" stdDeviation="6" floodColor="#000000" floodOpacity="0.25" result="outer-shadow"/>
    <feComponentTransfer in="SourceAlpha" result="alpha"><feFuncA type="linear" slope="1" /></feComponentTransfer>
    <feOffset in="alpha" dx="0" dy="4" result="offset-white" />
    <feGaussianBlur in="offset-white" stdDeviation="4" result="blur-white" />
    <feComposite in="alpha" in2="blur-white" operator="out" result="inner-white-mask" />
    <feFlood floodColor="#ffffff" floodOpacity="0.25" result="white-fill" />
    <feComposite in="white-fill" in2="inner-white-mask" operator="in" result="inner-white-final" />
    <feGaussianBlur in="alpha" stdDeviation="6" result="blur-black" />
    <feComposite in="alpha" in2="blur-black" operator="out" result="inner-black-mask" />
    <feFlood floodColor="#000000" floodOpacity="0.25" result="black-fill" />
    <feComposite in="black-fill" in2="inner-black-mask" operator="in" result="inner-black-final" />
    <feMerge>
      <feMergeNode in="outer-shadow" />
      <feMergeNode in="SourceGraphic" />
      <feMergeNode in="inner-white-final" />
      <feMergeNode in="inner-black-final" />
    </feMerge>
  </filter>
</defs>
Motion Element placed underneath the SVG code:
<motion.div initial={{ opacity: 0, scale: 0.98 }} whileInView={{ opacity: 1, scale: 1 }} transition={{ duration: 1.8, ease: [0.16, 1, 0.3, 1] }} className="relative">
Text Element logic: <h1 className="text-[min(25vw,400px)] font-bold tracking-normal leading-none select-none text-white px-4" style={{ filter: 'url(#glass-effect)' }}>vize</h1>
Final Default Export for Footer.tsx
code
Tsx
export default function Footer() {
  return (
    <footer className="w-full flex flex-col items-center gap-0">
      <FooterCard />
      <GlassText />
    </footer>
  );
}

## Zenith Footer — Footer Section [sections/zenith-footer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/zenith-footer.gif

Please create a React/Vite application utilizing Tailwind CSS, Lucide React, and Framer Motion. I want to build a landing page layout with a specific full-screen background video and a highly styled footer component.

### 1. Global Setup & Fonts
- In your global CSS file, import the "Geist" font from Google Fonts (weights 100..900).
- Set up a Tailwind custom theme variable for `--font-geist` using `"Geist", sans-serif`.
- Apply `@apply font-geist antialiased;` to the `body`.

### 2. Main App Layout & Background Video
Create a main layout that takes up the full screen (`min-h-[100dvh] h-full lg:h-[100dvh]`) styled with a black background (`bg-black`), flexbox column, relative positioning, and hidden overflow.

Inside this main layout, add a background `<video>`:
- **URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260429_111347_9cf2a2b0-2c10-475b-a132-147a046b4927.mp4`
- **Attributes**: `autoPlay`, `muted`, `loop`, `playsInline`
- **Classes**: `absolute inset-0 w-full h-full object-cover pointer-events-none`

Overlay a scrollable foreground container (`z-10`, `flex-1`, `px-4`, `overflow-y-auto`). Inside this, add a max-width container (`max-w-5xl mx-auto w-full flex-1 flex flex-col min-h-full`). The page content should go at the top, and the Footer component should be pushed to the bottom using `mt-auto pb-8`.

### 3. The Reusable `FitnessButton` Component
Create a `FitnessButton` component that uses `motion.button` from `motion/react`.
- **Props**: `children`, `icon`, `variant` ('primary' or 'secondary'), `className`, `onClick`.
- **Animations**: `whileHover={{ scale: 1.02, translateY: -1 }}` and `whileTap={{ scale: 0.98 }}`.
- **Base Classes**: `px-[18px] py-[12px] rounded-lg flex items-center justify-center gap-[10px] text-base font-geist font-normal cursor-pointer transition-all duration-200`.
- **Primary Variant**: `bg-[#060606] text-white shadow-[inset_0px_4px_8px_rgba(255,255,255,0.25),0px_4px_12px_rgba(255,255,255,0.25)] hover:shadow-[inset_0px_4px_8px_rgba(255,255,255,0.35),0px_8px_24px_rgba(255,255,255,0.35)]`. If it's the primary variant, it should render a `Rocket` icon from `lucide-react` (`w-5 h-5 text-white`) next to the text.
- **Secondary Variant**: `bg-[#F5F5F5] text-[#060606] border border-transparent hover:bg-white hover:border-black/10 shadow-sm`.
- Allow the `className` prop to override these defaults.

### 4. The `Footer` Component
Create the `Footer` component containing a `motion.div` container with the following properties:
- **Animations**: Reval inside view over 0.5s from `{ opacity: 0, y: 20 }` to `{ opacity: 1, y: 0 }` once.
- **Container Styling**: `bg-[#FFFFF0]/95 backdrop-blur-md rounded-[32px] p-6 sm:py-8 md:px-12 md:py-8 shadow-xl border border-white/20 flex flex-col`.

Inside the container, create a responsive CSS grid (`grid-cols-2 lg:grid-cols-[auto_1fr_auto_auto] gap-x-8 lg:gap-x-16 gap-y-6 sm:gap-y-6`).

Populate the grid with the following elements:
- **Top Left (Icon)**: A `Dumbbell` icon from `lucide-react` (`text-black w-8 h-8`, `strokeWidth={2.5}`).
- **Top Center (Heading)**: An `h2` with the text "Move, Heal, Bloom" (`text-2xl md:text-3xl font-medium tracking-tight text-black`).
- **Middle Left (Buttons)**: A flex wrap container (`gap-3`) holding two of our `FitnessButton`s: "Join Today" (primary) and "View Clubs" (secondary). Both need their padding, gap, and text size overridden with `!py-2 !px-5 !gap-2 !text-xs`.
- **Right Menus**: Two columns of text links aligned to the bottom (`sm:self-end flex flex-col gap-3`).
  - **Column 1 ("Insights")**: Links for 'Vitality Lab', 'Active Armor', 'Social Circles', and 'Get In Touch'.
  - **Column 2 ("Connect")**: Links for 'Meta Space', 'Pro Network', 'Vlog Stream', and 'Visual Feed'.
  - **Menu Heading styling**: `font-medium text-black uppercase tracking-[0.05em] text-[11px]`.
  - **Menu Link styling**: `text-black/70 hover:text-black transition-colors text-sm font-medium whitespace-nowrap`.
- **Bottom (Copyright)**: Below the grid, add a copyright footer section (`mt-6 sm:mt-8 flex flex-wrap gap-x-4 gap-y-1 text-[#060606]/40 text-[10px] font-medium tracking-tight uppercase`). Inside, render two span elements that both say "© Zenith Media Group 2025".
