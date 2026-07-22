# Michael Design Library — sections-content-social

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 21 entries.

## Axion About — About [sections/axion-about]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/axion-about.webp

Build a single React component for an "About" section using Tailwind CSS. Use `lucide-react` for the ArrowRight icon. System font stack only (no custom fonts). Match every detail exactly:

---

### Outer wrapper

`<section>` with `bg-white pt-16 sm:pt-20 lg:pt-32 pb-12 sm:pb-16 lg:pb-24 overflow-hidden`. Inner container: `max-w-[1440px] mx-auto`.

---

### Badge row

`px-5 sm:px-8 lg:px-12 flex items-center gap-3 mb-6 sm:mb-8`.

- **Numbered circle:** `w-6 h-6 sm:w-7 sm:h-7 rounded-full bg-gray-900 text-white flex items-center justify-center text-[11px] sm:text-[12px] font-semibold`. Displays "1".
- **Pill label:** Text "Introducing Axion". `text-[12px] sm:text-[13px] font-medium rounded-full px-3 sm:px-4 py-1 sm:py-1.5`. No border, no background.

---

### Heading

`<h2>` with `px-5 sm:px-8 lg:px-12 text-[clamp(1.5rem,4vw,3.2rem)] font-medium leading-[1.12] tracking-[-0.02em] text-gray-900 mb-12 sm:mb-16 lg:mb-28`.

Text: "Strategy-led creatives, delivering / results in digital and beyond." - the `/` represents a line break that is `<br className="hidden sm:block" />` with a `<span className="sm:hidden"> </span>` fallback space before it (so on mobile it reads as one flowing line, on sm+ it breaks into two lines).

---

### Content area - MOBILE / TABLET layout (lg:hidden)

Wrapper: `lg:hidden px-5 sm:px-8`.

1. **Paragraph:** "Through research, creative thinking and iteration we help growing brands realize their digital full potential." - `text-[15px] sm:text-[17px] leading-[1.6] font-medium text-gray-900 mb-6`.

2. **CTA button** (inside a `mb-8` wrapper): Orange button (`bg-[#F26522] hover:bg-[#e05a1a]`) with text "About our studio", `text-white text-[13px] sm:text-[14px] font-medium rounded-full pl-5 sm:pl-6 pr-2 py-2 flex items-center gap-3`. Contains:
   - **Text-roll hover animation:** The button text is inside `overflow-hidden h-[20px]` > `flex flex-col` container. The text is duplicated (two identical `h-[20px] flex items-center` spans). On `group-hover`, the flex-col translates `-translate-y-1/2` with `transition-transform duration-500 ease-[cubic-bezier(0.25,0.1,0.25,1)]`.
   - **Arrow circle:** White circle `bg-white w-7 h-7 sm:w-8 sm:h-8 rounded-full flex items-center justify-center`. Contains `ArrowRight` from lucide-react (size 14), `text-[#F26522]`, starts at `-rotate-45`, on `group-hover` rotates to `rotate-0` (same duration-500 easing). The entire button has `className="group"`.

3. **Images:** `flex flex-col sm:flex-row gap-4 sm:gap-5`.
   - First: `sm:w-[45%]`, `<img>` with `w-full aspect-[438/346] rounded-xl sm:rounded-2xl object-cover`.
   - Second: `sm:w-[55%]`, `<img>` with `w-full aspect-[900/600] rounded-xl sm:rounded-2xl object-cover`.

---

### Content area - DESKTOP layout (hidden lg:grid)

Wrapper: `hidden lg:grid grid-cols-[26%_1fr_48%] items-end gap-6 xl:gap-8 px-5 sm:px-8 lg:px-12`.

- **Left column** (`self-end`): Small image, `w-full aspect-[438/346] rounded-2xl object-cover`.
- **Center column** (`self-start flex flex-col justify-end`):
  - Paragraph: `text-[16px] xl:text-[18px] leading-[1.65] font-medium text-gray-900 whitespace-nowrap mb-6`. Text with explicit `<br/>` tags: "Through research, creative thinking`<br/>`and iteration we help growing brands`<br/>`realize their digital full potential."
  - Same orange CTA button as mobile (identical text-roll animation).
- **Right column** (`self-end`): Large image, `w-full aspect-[3/2] rounded-2xl object-cover`.

---

### Image URLs

- **Small image:** `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260516_090123_74be96d4-9c1b-40cf-932a-96f4f4babed3.png&w=1280&q=85`
- **Large image:** `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260516_090133_c157d30b-a99a-4477-bec1-a446149ec3f2.png&w=1280&q=85`

---

### Technical details

- **Framework:** React 18 + TypeScript + Tailwind CSS 3.4 (default config, no custom theme extensions)
- **Icons:** `ArrowRight` from `lucide-react`
- **Font:** System default (no custom font loaded)
- **All hover animations:** `duration-500 ease-[cubic-bezier(0.25,0.1,0.25,1)]`
- **Max content width:** 1440px, centered with `mx-auto`
- **Responsive breakpoints:** Default Tailwind (sm: 640px, md: 768px, lg: 1024px, xl: 1280px)

---

## Botanical Shadow About — About [sections/botanical-shadow-about]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/botanical-shadow-about.webp

Create a React + TypeScript + Vite project with Tailwind CSS. Build a full-viewport hero section called `about.tsx` with the following exact specifications.

### Fonts (load in `index.html` `<head>`)

```html
<link href="https://db.onlinewebfonts.com/c/076f8c5b3b67616658dd1e4e9bac62ec?family=Zimula+Trial+Med" rel="stylesheet">
<link href="https://db.onlinewebfonts.com/c/08d8ca53f66ab5b48659912fa0136b78?family=Zimula+Trial+Bd" rel="stylesheet">
```

And in `index.css`:
```css
@import url('https://db.onlinewebfonts.com/c/46024824a3dd3309c3a7f46f4f1283ba?family=Zimula+Trial+Reg');
```

### Global CSS (`index.css`)

- Reset: `*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }`
- `html { scroll-behavior: smooth; }`
- `body { font-family: 'Zimula Trial Med', sans-serif; background: #0e0c0a; overflow-x: hidden; }`
- Custom scrollbar: 6px wide, track `#0e0c0a`, thumb `rgba(255,255,255,0.15)` with 3px radius.

### Section Container

A `<section>` with: `position: relative; width: 100%; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow: hidden; fontFamily: "'Zimula Trial Med', sans-serif"`.

### Layer 1 — Background Video (z-index 0)

A `<video>` element with `autoPlay muted loop playsInline`, absolutely positioned to fill the section (`inset: 0; width: 100%; height: 100%; objectFit: cover`).

**Exact video URL (Cloudinary, not CloudFront):**
```
https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/f_auto/v1779835701/bg-2-video_sgbpqt.mp4
```

### Layer 2 — Warm Overlay (z-index 1)

Absolutely positioned div, `inset: 0`, `background: rgba(242, 238, 230, 0.38)`, `pointerEvents: none`. This warm off-white tint sits over the video for text legibility.

### Layer 3 — Centered Headline (z-index 2)

Absolutely positioned flex container (`inset: 0`), centered both axes, `pointerEvents: none`, `textAlign: center`, `padding: 0 24px`.

Inside, a `<p>` with:
- `fontSize: clamp(32px, 5.5vw, 80px)`
- `lineHeight: 1.18`
- `color: #2a2420` (warm dark brown)
- `maxWidth: 780px`
- `letterSpacing: -0.025em`
- `fontWeight: 400`
- `margin: 0`

Text content with explicit `<br />` line breaks:
```
What stands the
test of time is all
that guides the
work.
```

### Layer 4 — Bottom Element (z-index 2)

Absolutely positioned at `bottom: clamp(24px, 4vh, 48px)`, `left: 0`, `right: 0`. Flex column, items centered, `textAlign: center`, `padding: 0 24px`.

Children, in order:

**1. Vertical divider line:**
- `width: 1px; height: 56px; background: rgba(42,36,32,0.25)`

**2. Wrapper** with `marginTop: 22px`, flex column centered, `gap: 14px`, containing:

**a) Inline SVG map-pin icon** (24×28, viewBox `0 0 26 30`, fill none):
```jsx
<path
  d="M13 1C6.373 1 1 6.373 1 13c0 5.52 3.55 10.23 8.52 11.94l3.26 3.76a.75.75 0 001.14 0l3.26-3.76C22.45 23.23 25 18.52 25 13 25 6.373 19.627 1 13 1z"
  stroke="#2a2420"
  strokeWidth="1.4"
  fill="none"
/>
```

**b) Subtext `<p>`:**
- `fontSize: clamp(11px, 1.4vw, 13px)`
- `color: #2a2420`
- `letterSpacing: 0.04em`
- `lineHeight: 1.6`
- `maxWidth: 340px`
- `opacity: 0.75`
- Content: `Civic bodies and private clients trust us to shape resilient communities and purposeful places.`

### Notes

- **No animations, no scroll listeners, no parallax.** Section 2 is intentionally static. The only motion is the looping background video.
- All styling is inline (no CSS classes, no Tailwind utility classes inside the component) to keep the file self-contained.
---

## LaunchEx About — About [sections/launchex-about]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/launchex-about.webp

--

**Prompt to recreate the About section:**

> Build a full-viewport "About the Founders" section using React with Tailwind CSS and lucide-react for icons. This is a single `<section>` with the following exact specifications:
>
> **Section container:**
> - `id="about"`
> - Background color: `#F0F5F7`
> - `min-height: 100vh`
> - Position relative
> - Padding: `py-20 sm:py-28 px-6 sm:px-10`
> - Uses `flex flex-col justify-center` to vertically center content
>
> **Inner wrapper:**
> - `max-w-7xl mx-auto`
>
> **Top row -- heading + description (side by side on desktop):**
> - A flex container: `flex flex-col lg:flex-row items-start justify-between gap-10 lg:gap-20`
> - All text color: `#154359`
>
> **Left side -- Section heading:**
> - `<h2>` with text `About` on line 1, `the founders` on line 2 (separated by `<br />`)
> - Uses custom font class `.font-firs` (font-family: `'TT Firs Neue', 'Inter', system-ui, sans-serif`)
> - Font sizes: `text-[36px] sm:text-[48px] lg:text-[54px]`
> - `font-semibold uppercase tracking-tight leading-[0.95]`
>
> **Right side -- Description block:**
> - `flex flex-col max-w-xl`
> - Text container: `text-[17px] sm:text-[18px] leading-[1.5]`, color `#154359`
> - Paragraph 1: `"Launchex.Hub is a platform that is part of a portfolio of companies Launchex, for sourcing and showcasing groundbreaking innovations."`
> - Paragraph 2: `"Launchex.Hub's mission is to offer every local-language innovator the chance to reshape our world with their pioneering creation."` -- with `mt-4` spacing
> - Below paragraphs, an external link:
>   - `<a>` tag pointing to `https://base.launchex.vc/`, opens in new tab (`target="_blank" rel="noreferrer"`)
>   - `group inline-flex items-center gap-4 mt-6 text-[14px] font-medium`, color `#154359`
>   - Text: `"Launchex.Hub website"`
>   - Next to it, an icon button: `flex items-center justify-center w-8 h-8 border`, border color `#154359`, with `transition-transform group-hover:-translate-y-0.5`
>   - The icon button uses a chamfered clip-path: `polygon(8px 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%, 0 8px)`
>   - Inside the button: `<ArrowUpRight>` from lucide-react, `w-3.5 h-3.5`, `strokeWidth={2}`
>
> **Stats cards grid (below the heading row):**
> - `mt-14 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5`
> - Contains 3 stat cards with this exact data:
>   1. value: `"7+ years"`, text: `"Launchex has served the market, guiding ventures and their journeys"`, image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_154203_6c6f94dc-a07e-4ba5-8688-106f01ccd2c8.png&w=1280&q=85`, offset: false
>   2. value: `"15000+"`, text: `"innovation ventures moved through the Launchex pipeline"`, image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_154151_45c62c60-3bcc-4f21-8f9d-03722ebb5df8.png&w=1280&q=85`, offset: true
>   3. value: `"120+"`, text: `"accelerator sessions delivered by Launchex across Eastern Europe"`, image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_152238_24ec8db4-d728-4739-bb30-e985533e9637.png&w=1280&q=85`, offset: false
>
> **Each stat card:**
> - Outer wrapper: `relative w-full h-[280px] sm:h-[340px]`
> - The 2nd card (index 1, offset: true) gets `lg:mt-24` to create a staggered effect
> - Outer wrapper has `backgroundColor: 'rgba(255, 255, 255, 0.8)'` and `padding: '1.5px'` (acts as a thin white border)
> - Each card uses a unique polygon clip-path for chamfered/angular corners:
>   - Card 1: `polygon(64px 0, calc(100% - 14px) 0, calc(100% - 4px) 4px, 100% 14px, 100% calc(100% - 14px), calc(100% - 4px) calc(100% - 4px), calc(100% - 14px) 100%, 14px 100%, 4px calc(100% - 4px), 0 calc(100% - 14px), 0 64px)` -- large chamfer on top-left
>   - Card 2: `polygon(0 14px, 4px 4px, 14px 0, calc(100% - 64px) 0, 100% 64px, 100% calc(100% - 14px), calc(100% - 4px) calc(100% - 4px), calc(100% - 14px) 100%, 64px 100%, 0 calc(100% - 64px))` -- large chamfer on top-right and bottom-left
>   - Card 3: `polygon(0 14px, 4px 4px, 14px 0, calc(100% - 64px) 0, 100% 64px, 100% calc(100% - 64px), calc(100% - 64px) 100%, 14px 100%, 4px calc(100% - 4px), 0 calc(100% - 14px))` -- large chamfer on top-right, bottom-right
> - The same clip-path is applied to both the outer div and the inner image div (creating an inset border effect)
> - Inner div: `relative w-full h-full overflow-hidden bg-cover bg-center`, with the card's image as `backgroundImage`, `mixBlendMode: 'Normal'`
>
> **Text overlay inside each card:**
> - Positioned absolutely with different placements per card:
>   - Card 1: `left-6 right-6 bottom-6`
>   - Card 2: `left-6 bottom-20`
>   - Card 3: `left-6 right-28 bottom-6`
> - All have `max-w-[66%]`
> - The stat value uses `.font-firs font-semibold uppercase leading-none text-[36px] sm:text-[52px]`
> - Value text has a gradient fill: `linear-gradient(294deg, #185B7B 20%, #4BBDF0)` applied via `background`, `WebkitBackgroundClip: 'text'`, `backgroundClip: 'text'`, `color: 'transparent'`
> - Description text: `mt-3 text-[14px] leading-[1.4]`, color `#154359`
>
> **Bottom fade overlay:**
> - `pointer-events-none absolute inset-x-0 bottom-0 h-40 sm:h-56 z-10`
> - Background: `linear-gradient(to bottom, rgba(240, 245, 247, 0) 0%, rgba(240, 245, 247, 0.7) 60%, #F0F5F7 100%)` -- fades to the same background color
>
> **Fonts required in CSS:**
> ```css
> html, body {
>   font-family: 'Inter', system-ui, -apple-system, sans-serif;
>   -webkit-font-smoothing: antialiased;
> }
> .font-firs {
>   font-family: 'TT Firs Neue', 'Inter', system-ui, sans-serif;
> }
> ```
>
> **Color palette used:**
> - Section background: `#F0F5F7`
> - All text: `#154359` (dark teal/navy)
> - Card outer background/border: `rgba(255, 255, 255, 0.8)`
> - Stat value gradient: `#185B7B` to `#4BBDF0` at 294deg
> - Link icon border: `#154359`
> - Bottom gradient: fades to `#F0F5F7`
>
> **Key design details:**
> - The stat cards use CSS `clip-path` polygons (not border-radius) for angular/chamfered corner shapes -- each card has a different polygon creating visual variety
> - The 1.5px padding on the outer wrapper + white background creates the appearance of a thin white border inside the clip-path
> - The 2nd card is offset downward by `lg:mt-24` to create a staggered/masonry feel on desktop
> - Background images are loaded via inline `backgroundImage` style, not `<img>` tags
> - The external link arrow icon sits inside a small chamfered square button using clip-path
> - Responsive: cards stack 1-column on mobile, 2-column on `md:`, 3-column on `lg:`
> - No animations beyond the hover lift on the external link icon button (`group-hover:-translate-y-0.5`)

---

## Orbis Hello — About [sections/orbis-hello]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/orbis-hello.webp

---

> **Setup requirements before building the section:**
>
> **Google Fonts** -- Load these in `index.html` `<head>`:
> ```html
> <link rel="preconnect" href="https://fonts.googleapis.com" />
> <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
> <link href="https://fonts.googleapis.com/css2?family=Anton&family=Condiment&display=swap" rel="stylesheet" />
> ```
>
> **Tailwind config** -- Extend `theme` with these exact custom values:
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
> `font-grotesk` maps to **Anton** (a tall, condensed display font). `font-condiment` maps to **Condiment** (a flowing cursive/script font).
>
> **No additional CSS classes or animations are used in this section.** No keyframes, no transitions, no hover states. It is a static layout.
>
> ---
>
> **Build the following section as a React component using Tailwind CSS:**
>
> A `<section>` tag with classes `relative overflow-hidden min-h-screen`. No background color -- the background is a fullscreen video.
>
> **Background video:** An absolutely positioned `<video>` element covering the entire section. Classes: `absolute inset-0 w-full h-full object-cover`. Attributes: `autoPlay`, `loop`, `muted`, `playsInline`. The `<source>` element points to:
> ```
> https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_151551_992053d1-3d3e-4b8c-abac-45f22158f411.mp4
> ```
> with `type="video/mp4"`.
>
> **Content wrapper:** A `<div>` sitting on top of the video with classes: `relative max-w-[1831px] mx-auto px-4 sm:px-6 md:px-8 py-16 sm:py-20 md:py-24 z-10`.
>
> Inside the content wrapper are **two rows**:
>
> ---
>
> **ROW 1 (top):** A `<div>` with classes `flex flex-col lg:flex-row justify-between items-start gap-8 lg:gap-12 mb-12 sm:mb-16 md:mb-20`. Contains two children:
>
> **Child A -- The heading:** An `<h2>` with classes `font-grotesk text-[32px] sm:text-[48px] md:text-[60px] font-normal uppercase leading-[1.05] sm:leading-[1] md:leading-[1] text-cream relative`. The text content is:
> ```
> Hello!<br />
> I'm orbis
> ```
> (Literally "Hello!" on line 1, "I'm orbis" on line 2, separated by a `<br />`. All rendered uppercase by Tailwind so it displays as "HELLO!" and "I'M ORBIS".)
>
> **Inside the `<h2>`**, after the text, there is an absolutely positioned `<span>` with the word **"Orbis"**. This span has classes: `font-condiment text-[36px] sm:text-[52px] md:text-[68px] font-normal normal-case text-neon mix-blend-exclusion leading-[0.79] tracking-[0.03em] absolute right-[-8px] bottom-[-20px] sm:bottom-[-30px] md:bottom-[-40px] -rotate-1 opacity-90`.
>
> Key details of this span:
> - `normal-case` overrides the parent's uppercase, so it renders as "Orbis" (capital O, lowercase rbis) in the Condiment cursive font.
> - `text-neon` = `#6FFF00` (bright green).
> - `mix-blend-exclusion` makes the green text interact with the video background.
> - `absolute right-[-8px] bottom-[-20px]` (responsive: `sm:bottom-[-30px] md:bottom-[-40px]`) positions it hanging below and slightly right of the parent heading, overlapping the word "orbis" above it.
> - `-rotate-1` gives it a slight counter-clockwise tilt (-1 degree).
> - `leading-[0.79]` is a very tight line-height. `tracking-[0.03em]` adds subtle letter spacing.
> - `opacity-90` makes it 90% opaque.
>
> **Child B -- The paragraph:** A `<p>` with classes `font-mono text-[14px] sm:text-[15px] md:text-[16px] uppercase text-cream max-w-[266px] leading-relaxed`. The text is:
> > "A digital object fixed beyond time and place. An exploration of distance, form, and silence in space"
>
> (`font-mono` uses the browser's default monospace font. `leading-relaxed` = 1.625 line-height.)
>
> ---
>
> **ROW 2 (bottom):** A `<div>` with classes `flex justify-between items-start`. Contains two children:
>
> **Child A -- Left text column** (always visible): A `<div>` with classes `flex flex-col gap-5 max-w-[335px]`. Contains **two identical `<p>` tags**, each with classes `font-mono text-[14px] sm:text-[15px] md:text-[16px] uppercase lg:text-cream text-[#010828] opacity-10 leading-relaxed`. Both contain the same text:
> > "A digital object fixed beyond time and place. An exploration of distance, form, and silence in space"
>
> Key detail: The color is `text-[#010828]` (near-invisible dark navy matching the page background) by default, switching to `lg:text-cream` (`#EFF4FF`) on large screens. Combined with `opacity-10`, this text is extremely faint/ghostly -- almost invisible, serving as a subtle texture element rather than readable content.
>
> **Child B -- Right text column** (desktop only): A `<div>` with classes `hidden lg:flex flex-col gap-5 max-w-[335px]`. Contains **two identical `<p>` tags** with the exact same classes and text as Child A. This column is hidden on mobile/tablet and only appears on `lg:` (1024px+) screens.
>
> ---
>
> **There are no animations, transitions, hover effects, scroll effects, or JavaScript interactions in this section.** It is purely a static layout with a looping background video. The only "motion" comes from the autoplaying video itself.

---

## Portfolio About — About [sections/portfolio-about]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/portfolio-about.webp

**Prompt:**

Create an "About Me" section using React, Tailwind CSS, and **framer-motion**. The site uses **Google Font "Kanit"** (weights 300-900) and a dark background `#0C0C0C`.

**Section layout:**
- Full-width section, `min-h-screen`, flexbox column, centered both axes
- Padding: `px-5 sm:px-8 md:px-10 py-20`
- Background: `#0C0C0C` (inherited from page)
- `position: relative` -- the section has 4 decorative floating images placed absolutely in the corners

**4 decorative corner images (absolute positioned, z-0):**

1. **Top-left** -- Moon icon
   - URL: `https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/moon_icon.11395d36.png`
   - Position: `top-[4%] left-[1%] sm:left-[2%] md:left-[4%]`
   - Size: `w-[120px] sm:w-[160px] md:w-[210px] h-auto`
   - Fade-in animation: `delay: 0.1`, slides from left (`x: -80, y: 0`), `duration: 0.9`

2. **Bottom-left** -- 3D object
   - URL: `https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/p59_1.4659672e.png`
   - Position: `bottom-[8%] left-[3%] sm:left-[6%] md:left-[10%]`
   - Size: `w-[100px] sm:w-[140px] md:w-[180px] h-auto`
   - Fade-in animation: `delay: 0.25`, slides from left (`x: -80, y: 0`), `duration: 0.9`

3. **Top-right** -- Lego icon
   - URL: `https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/lego_icon-1.703bb594.png`
   - Position: `top-[4%] right-[1%] sm:right-[2%] md:right-[4%]`
   - Size: `w-[120px] sm:w-[160px] md:w-[210px] h-auto`
   - Fade-in animation: `delay: 0.15`, slides from right (`x: 80, y: 0`), `duration: 0.9`

4. **Bottom-right** -- 3D group
   - URL: `https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/Group_134-1.2e04f3ce.png`
   - Position: `bottom-[8%] right-[3%] sm:right-[6%] md:right-[10%]`
   - Size: `w-[130px] sm:w-[170px] md:w-[220px] h-auto`
   - Fade-in animation: `delay: 0.3`, slides from right (`x: 80, y: 0`), `duration: 0.9`

**Center content (relative z-10, max-w-4xl, centered):**

Vertical layout with `gap-16 sm:gap-20 md:gap-24`, containing two groups:

**Group 1 -- Heading + Animated Text** (gap `10 sm:14 md:16`):

- **Heading "About me":**
  - `font-black uppercase leading-none tracking-tight text-center`
  - Font size: `clamp(3rem, 12vw, 160px)`
  - Uses a CSS class `hero-heading` which applies a gradient text fill:
    ```css
    .hero-heading {
      background: linear-gradient(180deg, #646973 0%, #BBCCD7 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    ```
  - Fade-in: `delay: 0, y: 40`

- **Animated paragraph** (scroll-driven character-by-character reveal):
  - Text content: `"With more than five years of experience in design, i focus on branding, web design, and user experience, i truly enjoy working with businesses that aim to stand out and present their best image. Let's build something incredible together!"`
  - Styling: `text-[#D7E2EA] font-medium text-center leading-relaxed max-w-[560px]`
  - Font size: `clamp(1rem, 2vw, 1.35rem)`
  - **Animation behavior** (uses framer-motion `useScroll` + `useTransform`):
    - Each character is rendered as an individual `<span>` with `position: relative; display: inline-block`
    - An invisible duplicate holds the space; the visible character is absolutely positioned on top
    - Scroll tracking: `useScroll({ target: containerRef, offset: ['start 0.8', 'end 0.2'] })`
    - Per-character opacity: calculate `charProgress = index / totalChars`, then `start = max(0, charProgress - 0.1)` and `end = min(1, charProgress + 0.05)`. Map `scrollYProgress` from `[start, end]` to opacity `[0.2, 1]`
    - Spaces are rendered as `\u00A0` (non-breaking space)
    - Characters start dim (opacity 0.2) and brighten to full opacity (1) as the user scrolls through the section, creating a progressive text reveal from left to right

**Group 2 -- Contact Button:**
- Fade-in: `delay: 0.3, y: 20`
- Pill-shaped button (rounded-full), text "Contact Me"
- Responsive padding: `px-8 py-3 sm:px-10 sm:py-3.5 md:px-12 md:py-4`
- Text: `text-white font-medium uppercase tracking-widest`, size `text-xs sm:text-sm md:text-base`
- Background gradient: `linear-gradient(123deg, #18011F 7%, #B600A8 37%, #7621B0 72%, #BE4C00 100%)`
- Box shadow: `0px 4px 4px rgba(181, 1, 167, 0.25), 4px 4px 12px #7721B1 inset`
- Outline: `2px solid #E3E3E3` with `outlineOffset: -3px`
- Hover: `opacity: 0.9`, Active: `opacity: 0.75`, transition 200ms
- Links to `#contact`

**FadeIn component (reusable, framer-motion):**
- Props: `delay`, `duration` (default 0.7), `x` (default 0), `y` (default 30), `className`, `style`, `as` (HTML element tag, default `div`)
- Uses `motion.create()` to make any HTML element animatable
- Variants: `hidden` state sets `opacity: 0` + the x/y offsets; `visible` animates to `opacity: 1, x: 0, y: 0`
- Easing: cubic bezier `[0.25, 0.1, 0.25, 1]`
- Viewport trigger: `{ once: true, margin: "50px", amount: 0 }`

**Font (loaded in HTML head):**
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Kanit:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

CSS base: `font-family: 'Kanit', sans-serif` on html/body.

---

## Tech-Noir About — About [sections/tech-noir-about]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/tech-noir-about.webp

---

### Prompt

Create a single full-page section with a solid `#FF0000` red background using React 19, TypeScript, Vite, Tailwind CSS v4 (`@tailwindcss/vite`), and `motion` (from `motion/react`).

### Fonts (index.css)

```css
@import url('https://fonts.googleapis.com/css2?family=Italiana&family=Manrope:wght@400;600&family=Marck+Script&display=swap');
@import "tailwindcss";

@theme {
  --font-manrope: "Manrope", sans-serif;
  --font-italiana: "Italiana", serif;
  --font-marck: "Marck Script", cursive;
}
```

### Section Container

```
<section className="relative min-h-screen w-full bg-[#FF0000] flex flex-col z-10">
```

---

### 1. Centered Content

**Outer wrapper:**
```
<div className="flex-1 flex flex-col items-center w-full pt-[100px] md:pt-[400px]">
```

**Inner container:**
```
<div className="flex flex-col items-center w-full px-8 text-center z-20 relative max-w-[900px] h-auto md:h-[620px] mx-auto">
```

**a) Logo SVG** -- white fill, 80x80, `mb-12`:
```tsx
<svg width="80" height="80" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path fillRule="evenodd" clipRule="evenodd" d="M60 120C26.8629 120 0 93.1371 0 60V0C22.5654 0 42.2213 12.4569 52.4662 30.8691C38.4788 34.2089 28.0787 46.7902 28.0787 61.8006V63.1443C28.0787 79.9648 41.7146 93.6006 58.5353 93.6006H59.8789L59.8785 61.8006C59.8785 79.3633 74.1159 93.6006 91.6787 93.6006L91.6787 61.8006C91.6787 44.2783 77.5071 30.0661 60 30.0008L60 0H62.5352C94.2722 0 120 25.7279 120 57.4648V60C120 93.1371 93.1371 120 60 120Z" fill="white"/>
</svg>
```

**b) Mission statement:**
```tsx
<p className="text-white text-[16px] h-[100px] w-full max-w-[400px] leading-[1.6] mb-[40px] uppercase tracking-wider mx-auto">
  We built this platform with a single purpose to eliminate operational chaos and restore balance to your daily business routine
</p>
```

**c) Cursive signature:**
```tsx
<div className="font-marck text-white text-[120px] leading-none mb-[32px]">
  S.P.D
</div>
```

**d) Two paragraphs** (Title Case, font-light):
```tsx
<div className="text-white leading-[1.6] mb-[100px] md:mb-24 w-full flex flex-col items-center font-light">
  <p className="mb-[24px] text-[16px] w-[400px] max-w-full text-center">
    I Was Exhausted By Software That Demanded More Effort Than It Actually Saved. That Is Why We Engineered An Autonomous Architecture That Operates Silently In The Background.
  </p>
  <p className="text-[16px] w-[400px] max-w-full text-center">
    Your Business Should Serve Your Life, Not Consume It. Let Our Algorithms Handle The Heavy Lifting, So You Can Focus On The Vision.
  </p>
</div>
```

---

### 2. Bottom Video with Red Gradient Blend

```tsx
<div className="relative w-full shrink-0">
  <div className="absolute top-0 left-0 w-full h-[100px] bg-gradient-to-b from-[#FF0000] to-transparent z-10 pointer-events-none" />
  <video autoPlay loop muted playsInline className="w-full h-auto block object-contain">
    <source
      src="https://res.cloudinary.com/daklr2whx/video/upload/v1778602552/track-video_2_s9lp53.mp4"
      type="video/mp4"
    />
  </video>
</div>
```

A 100px gradient overlay at the top of the video fades from `#FF0000` to transparent, seamlessly blending the red background into the video. The video uses `object-contain` -- native aspect ratio, full width, no cropping. `shrink-0` prevents flexbox from compressing it.

---

### Asset URL

| Asset | URL |
|---|---|
| Bottom video | `https://res.cloudinary.com/daklr2whx/video/upload/v1778602552/track-video_2_s9lp53.mp4` |

Hosted on **Cloudinary**.

## Blog Showcase — Blog [sections/blog-showcase]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/blog-showcase.webp

Build a "Behind the Lens" photography blog section with the following exact specifications:

**Layout and Structure:**
- White background page, max-width 1200px, centered, 60px vertical padding, 20px horizontal padding
- Header with: a small grey "Blog" badge (bg #f4f4f4, 8px border-radius), a large heading "Behind the lens" (64px, Outfit font, weight 500, letter-spacing -2.5px), a subtitle paragraph and a "View all posts" button side by side
- Subtitle: "Thoughts, insights, and stories from my photography journey. Take a peek into my creative process and recent projects." (max-width 480px, #666 color, 18px, weight 500, opacity 0.8)
- "View all posts" button: black bg, white text, 40px border-radius pill shape, 14px font, weight 600, scales 1.02 on hover

**Featured Post (full-width card):**
- 2-column grid (1fr 1fr), 20px border-radius, 1px solid #f0f0f0 border, min-height 520px, bg #fcfcfc
- Left side: autoplaying looped muted video, object-fit cover, fills the entire area
- Right side: 60px padding, contains a black "Must Read" pill badge (12px font, 20px border-radius), title in Outfit font 48px weight 500 (letter-spacing -1.5px), description in #666 at 17px, and a footer with author name and colored category badge pushed to the bottom via margin-top auto
- Featured post data: title "Full-Frame vs. Crop Sensor: Which for Photography?", description "An honest look at the real-world differences between these camera systems to help you choose what's actually right for your photography needs.", author "By August Renner (c)", category "Gear" with color #7d1a4a
- Featured video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260507_155500_808e6fdd-761f-4acd-b3be-cb7e6e700def.mp4`

**Blog Grid (3 standard cards):**
- 3-column grid, 25px gap, below the featured post
- Each card: video with 16/10 aspect ratio, 20px border-radius, title below (Outfit 17px weight 600) with a colored category badge aligned right
- Card 1: "Finding Natural Light in Unexpected Places", category "Lighting" (#2c4c34), video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260506_030111_a9e15665-d379-4a7f-8116-695bbe452ad1.mp4`
- Card 2: "My Approach to Editing: Creating a Consistent Photography Style", category "Editing" (#a63e2d), video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_064122_c4750c0e-7476-4b44-94a2-a85a65c63bf2.mp4`
- Card 3: "Pricing Your Photography: Strategies That Work", category "Business" (#1a2b8c), video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260507_154232_f8809bd2-a6c3-4a38-908d-2005e5b3cb3e.mp4`

**Hover Interactions (on all video containers):**
- Videos scale to 1.08 on hover with cubic-bezier(0.33, 1, 0.68, 1) over 0.5s
- A dark overlay (rgba(0,0,0,0.25)) fades in over 0.4s on hover
- A centered "+" icon inside a 70px circle (rgba(255,255,255,0.2) bg) scales from 0.7 to 1.0 on hover over 0.3s
- White "L-shaped" corner brackets (12px, 1.5px border) in all 4 corners of each video, 15px inset from edges

**Category Badges:**
- Pill shape (20px border-radius), white text, 11px font, weight 600, 4px 12px padding, capitalized, background color matches each post's assigned color

**Fonts:**
- Google Fonts: Inter (400, 500, 600) for body text, Outfit (500, 600, 700) for headings and titles

**Responsive:**
- At 1024px: featured post becomes single column, grid becomes 2 columns, featured content padding 40px
- At 768px: heading drops to 48px, header-bottom stacks vertically, grid becomes 1 column, featured title drops to 32px

**Data Source:**
- Store all blog post data (type, badge, title, description, author, category, category_color, image/video URL, display_order) in a Supabase `blog_posts` table
- Fetch and render dynamically, ordered by display_order ascending
- Enable RLS with public read access for anon and authenticated users

**Tech Stack:**
- React + TypeScript + Vite + Tailwind CSS (for base resets only, use custom CSS for the blog styles)
- Supabase JS client for data fetching
- All videos use autoPlay, loop, muted, playsInline attributes

## Pixel Grid Hover — Case Studies [sections/pixel-grid-hover]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/pixel-grid-hover.webp

Build a "Projects / Case Studies" section as a React + TypeScript component using Tailwind CSS 3 and Framer Motion. Font is `'DM Sans', sans-serif` (Google Fonts: `https://fonts.googleapis.com/css?family=DM+Sans:500,400`). White background, black text. Here is the exact specification:

---

**Section Container:**
- `<section>` with `relative bg-white text-black`, inline style `fontFamily: "'DM Sans', sans-serif"`
- All animations use easing `[0.22, 1, 0.36, 1]`
- Inject a `<style>` block for the marquee keyframe:
  ```css
  @keyframes marqueeProjects {
    from { transform: translateX(0); }
    to   { transform: translateX(-50%); }
  }
  .marquee-projects {
    animation: marqueeProjects 28s linear infinite;
  }
  .marquee-projects:hover {
    animation-play-state: paused;
  }
  ```

---

**Top Area (Header with floating squares):**
- Container: `relative px-6 pb-10 pt-32 sm:px-10 lg:px-16 lg:pt-40`
- Contains a `pointer-events-none absolute inset-0 overflow-hidden` layer with 8 parallax floating black squares. Each square uses `useScroll` (target: the section ref, offset `["start end", "end start"]`) and `useTransform` + `useSpring` for vertical parallax, plus a gentle infinite bob animation (`y: [0, -10, 0]`, duration `3s + index * 0.4s`, ease `easeInOut`, staggered delay `index * 0.3s`).
- Square positions (x%, y%, size px):
  ```
  (6, 20, 12), (12, 32, 8), (8, 44, 6), (88, 18, 10),
  (92, 30, 14), (85, 42, 7), (90, 52, 5), (14, 56, 5)
  ```
- Parallax formula: `useTransform(scrollYProgress, [0, 1], [0, -(80 + index * 30)])`, smoothed with `useSpring({ stiffness: 40, damping: 20 })`

**Header text** (centered, inside `relative mx-auto max-w-7xl text-center`):
- Animates from `opacity: 0, y: 24` to visible, duration `0.7s`, triggered by `useInView` with `{ once: true, margin: "-60px" }`
- Badge: "Projects" in `mb-5 inline-block bg-black px-4 py-1.5 text-[13px] font-medium tracking-wide text-white`
- Heading: `text-[clamp(1.8rem,3.2vw,2.8rem)] font-light leading-[1.25] tracking-tight`
  - "Insights from " in `text-black`, then "Our" in `text-black/40`
  - New line: "Case Studies" in `text-black/40`

---

**Case Study Cards (2x2 grid):**
- Container: `mx-auto max-w-7xl px-6 pb-16 sm:px-10 lg:px-16`, inner `grid gap-4 md:grid-cols-2`
- Each card animates from `opacity: 0, y: 30` to visible, staggered `delay: index * 0.1`, duration `0.7s`

**4 case studies data:**
```
1. id: "heartx", title: "HeartX", category: "Brand Strategy & Product Design", year: "2026"
   image: https://images.pexels.com/photos/7691249/pexels-photo-7691249.jpeg?auto=compress&cs=tinysrgb&w=800

2. id: "swave", title: "Swave\u00AE", category: "Web Design & Identity", year: "2025"
   image: https://images.pexels.com/photos/2559941/pexels-photo-2559941.jpeg?auto=compress&cs=tinysrgb&w=800

3. id: "eduspark", title: "EduSpark", category: "Brand Strategy & Web Design", year: "2023"
   image: https://images.pexels.com/photos/5428003/pexels-photo-5428003.jpeg?auto=compress&cs=tinysrgb&w=800

4. id: "greenergy", title: "Greenergy", category: "Brand Strategy & Web Design", year: "2022"
   image: https://images.pexels.com/photos/2800832/pexels-photo-2800832.jpeg?auto=compress&cs=tinysrgb&w=800
```

**Each card structure** (aspect ratio `4/3`, `group relative overflow-hidden`):

1. **Background image**: absolutely positioned, `h-full w-full object-cover`

2. **Pixel-block hover overlay**: A 12-column x 8-row grid of absolutely positioned `bg-black/80` blocks. Each block covers `100/12 %` width and `100/8 %` height. On hover they animate `scale: 0 -> 1, opacity: 0 -> 1` with a diagonal stagger: `delayIn = (row + col) * 0.018s`, `delayOut = ((8 - row) + (12 - col)) * 0.012s`. Duration `0.25s`. This creates a pixel-dissolve reveal effect on hover.

3. **Magnetic squares**: 5-6 small black squares per card, absolutely positioned. They react to the cursor via `useMotionValue` + `useTransform` + `useSpring`: when the card is hovered, each square shifts toward the pointer proportionally (`dist * 40`). Spring config: `{ stiffness: 80, damping: 18, mass: 0.6 }`. When pointer leaves, they spring back to center (pointer resets to `0.5, 0.5`).

   Square positions per card (x%, y%, size px):
   ```
   HeartX:    (5,30,16), (10,42,10), (3,52,7), (80,70,14), (85,82,9), (78,60,6)
   Swave:     (82,55,16), (88,68,10), (78,72,7), (85,42,6), (90,80,8)
   EduSpark:  (4,24,16), (10,36,10), (2,44,7), (78,78,14), (84,88,8)
   Greenergy: (82,26,14), (88,38,10), (78,44,7), (84,54,5), (90,60,8)
   ```

4. **Plus button** (top right): `absolute right-4 top-4`, `h-7 w-7 items-center justify-center border border-white/30 text-xs text-white`, "+" text, `zIndex: 10`

5. **Info plate** (bottom left): `absolute bottom-0 left-0 bg-white px-4 pb-3 pt-2.5`, `zIndex: 20`, `maxWidth: "70%"`
   - Title: `text-[clamp(1.4rem,2.2vw,2rem)] font-normal leading-tight text-black`
   - Below: flex row with category (`text-[12px] text-black/60`) and year (`text-[12px] font-medium text-black`), `mt-1.5 gap-4`

---

**Footer Area:**
- Container: `mx-auto max-w-7xl px-6 pb-6 sm:px-10 lg:px-16`
- Flex row on desktop (`md:flex-row md:items-end md:justify-between`), column on mobile

**Left side** (`max-w-md`):
- Plus button: `mb-4 flex h-7 w-7 items-center justify-center border border-black/20 text-xs text-black`, "+"
- Paragraph: "We partner with ambitious brands that are ready to move beyond fragmented visuals and shallow quick fixes -- turning their identity, website, and messaging into one focused engine for growth." in `text-[14px] leading-[1.7] text-black/60`
- CTA button (`mt-6`): A `<button>` with `group flex items-end`:
  - Main label: `inline-flex items-center gap-[10px] border border-black/20 bg-black px-3 py-2 text-base font-medium text-white`, hover `bg-black/85`. Text: "Let's work together"
  - Arrow badge: A small `h-6 w-6` black square with `mb-6`, containing a diagonal arrow SVG (white, 16x16, viewBox `0 0 24 24`, path: `M18.75 6V15.75C18.75 15.949 18.671 16.14 18.53 16.28C18.39 16.421 18.199 16.5 18 16.5C17.801 16.5 17.61 16.421 17.47 16.28C17.329 16.14 17.25 15.949 17.25 15.75V7.81L6.53 18.53C6.39 18.671 6.199 18.75 6 18.75C5.801 18.75 5.61 18.671 5.47 18.53C5.329 18.39 5.25 18.199 5.25 18C5.25 17.801 5.329 17.61 5.47 17.47L16.19 6.75H8.25C8.051 6.75 7.86 6.671 7.72 6.53C7.579 6.39 7.5 6.199 7.5 6C7.5 5.801 7.579 5.61 7.72 5.47C7.86 5.329 8.051 5.25 8.25 5.25H18C18.199 5.25 18.39 5.329 18.53 5.47C18.671 5.61 18.75 5.801 18.75 6Z`). On group hover, the badge shifts up: `mb-6 -> mb-9`, with `transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]`

**Right side** (`flex-1 overflow-hidden md:ml-12`, with `border-t border-black/10` on mobile, no border on desktop):
- An infinite horizontal marquee (`overflow-hidden py-5`)
- Inner track: `marquee-projects flex w-max` (uses the CSS keyframe above, 28s duration)
- Pauses on hover
- 8 logos, doubled (16 total) for seamless loop. Each item: `flex shrink-0 items-center gap-2.5 px-8`
  - An SVG icon (black, varying per logo)
  - Name text: `whitespace-nowrap text-sm font-medium tracking-wide text-black/80`

**8 marquee logos** (name, icon type):
```
("Codecraft_", code), ("ennLabs", dots), ("GlobalBank", circle-ring),
("45 Degrees\u00b0", arrow), ("AlphaWave", wave-circle), ("Biosynthesis", lines),
("Boltshift", bolt), ("Clandestine", plus)
```

**SVG icon definitions** (all black stroke or fill):
- **code**: 22x18, viewBox `0 0 22 18`, stroke, strokeWidth 2, round caps. Two polylines `6,4 1,9 6,14` and `16,4 21,9 16,14`, one line `13,2 to 9,16`
- **dots**: 20x20, viewBox `0 0 20 20`, filled. 9 circles at grid positions `[3,10,17] x [3,10,17]`, radius `2.2`
- **circle-ring**: 22x22, viewBox `0 0 22 22`, stroke, strokeWidth 2. Two circles at `(11,11)` with radii `9` and `4`
- **arrow**: 18x18, viewBox `0 0 18 18`, stroke, strokeWidth 2, round caps. Line `2,16 to 16,2`, polyline `7,2 16,2 16,11`
- **wave-circle**: 22x22, viewBox `0 0 22 22`, stroke, strokeWidth 1.5. Circle `(11,11)` r=9, path `M5 11Q8 7 11 11Q14 15 17 11`
- **lines**: 24x18, viewBox `0 0 24 18`, stroke, strokeWidth 2.2, round caps. Three horizontal lines: `(0,3 to 24,3)`, `(6,9 to 24,9)`, `(0,15 to 18,15)`
- **bolt**: 14x20, viewBox `0 0 14 20`, filled. Polygon `8,0 0,11 6,11 6,20 14,9 8,9`
- **plus**: 18x18, viewBox `0 0 18 18`, filled. Two rects: `(7.5, 0, 3, 18)` and `(0, 7.5, 18, 3)`

**Bottom spacer**: `<div className="h-12" />`

---

**Dependencies:** React 18, Framer Motion (v12+), Tailwind CSS 3. Uses `useRef`, `useState`, `useCallback`, `useScroll`, `useTransform`, `useSpring`, `useMotionValue`, `useInView`, and `motion` from framer-motion. No other libraries.

## Rocket FAQ — FAQ [sections/rocket-faq]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/rocket-faq.webp

Build a dark-themed FAQ section for a React 18 + Vite + TypeScript app using TailwindCSS, framer-motion, and Radix UI Accordion. Match these specs exactly.

### Stack & Global Setup

- React 18, Vite, TypeScript, TailwindCSS
- Dependencies: `framer-motion`, `@radix-ui/react-accordion`, `clsx`, `tailwind-merge`
- `cn()` helper: `twMerge(clsx(inputs))` exported from `@/lib/utils`
- Dark theme background `#000000`, font `Inter`, Material Symbols Outlined for icons
- Tailwind `theme.extend.colors`:
  - `landing.surface: "rgba(255,255,255,0.10)"`
  - `landing.surface-hover: "rgba(255,255,255,0.16)"`
  - `border: "rgba(255,255,255,0.10)"`
  - `foreground: "hsl(0 0% 100%)"` (semantic; use `text-foreground`, `text-foreground/60`, `text-foreground/70`, `text-foreground/80`)
  - `background: "#000000"` (semantic)

### Helper Components

### MIcon (Material Symbols Outlined)
```tsx
type Props = { name: string; size?: number; className?: string; fill?: 0|1; weight?: number; grade?: number; opsz?: number };
export const MIcon = ({ name, size=20, className, fill=0, weight=400, grade=0, opsz=24 }: Props) => (
  <span
    className={cn("material-symbols-outlined leading-none", className)}
    style={{ fontSize: size, fontVariationSettings: `'FILL' ${fill}, 'wght' ${weight}, 'GRAD' ${grade}, 'opsz' ${opsz}` }}
  >{name}</span>
);
```
Include in `index.html`:
```html
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200" />
```

### FadeUp (framer-motion scroll reveal)
- `motion.div`, `initial={{opacity:0, y:24}}`, `whileInView={{opacity:1, y:0}}`, `viewport={{once:true, amount:0.3}}`, `transition={{duration:0.6, delay, ease:[0.22,1,0.36,1]}}`. Honors `useReducedMotion`.

### SpotlightBorder (cursor-tracked 1px gradient ring)
- Wrapper `relative` with `rounded-{radius}` (xl/2xl/3xl/full).
- Inside, an absolute-inset `<span>` with style:
```ts
{
  background: `radial-gradient(${size}px circle at var(--spot-x,-200px) var(--spot-y,-200px), rgba(255,255,255,${intensity}), rgba(255,255,255,0) 60%)`,
  padding: "1px",
  WebkitMask: "linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)",
  WebkitMaskComposite: "xor",
  maskComposite: "exclude",
}
```
- Global mousemove listener writes `--spot-x` / `--spot-y` on the element (relative to its bounding rect).
- Polymorphic `as`: div | button | section. Export both the component and `spotlightMaskStyle(size, intensity)` helper for reuse inline.

### Radix Accordion wrapper (shadcn-style)
`Accordion`, `AccordionItem` (`border-b` default), `AccordionTrigger` (flex justify-between, hides built-in chevron when class includes `[&>svg]:hidden`), `AccordionContent` (with `data-[state=open]:animate-accordion-down`, `data-[state=closed]:animate-accordion-up`). Use standard shadcn accordion file.

Tailwind keyframes (in config):
```js
keyframes: {
  "accordion-down": { from: { height: "0" }, to: { height: "var(--radix-accordion-content-height)" } },
  "accordion-up":   { from: { height: "var(--radix-accordion-content-height)" }, to: { height: "0" } },
},
animation: { "accordion-down": "accordion-down 0.2s ease-out", "accordion-up": "accordion-up 0.2s ease-out" }
```

### Data

```ts
type CategoryKey = "general" | "ai" | "integrations";
const categories = [
  { key: "general", label: "General" },
  { key: "ai", label: "AI & Capabilities" },
  { key: "integrations", label: "Integrations & Security" },
];
const faqs: Record<CategoryKey, {q:string; a:string}[]> = {
  general: [
    { q: "What is UI Rocket?", a: "UI Rocket is a learning platform for designers who want to master AI-powered design workflows and ship production-ready websites faster." },
    { q: "Who is this for?", a: "Designers, founders, and creators who want to level up their AI design skills and build real, shippable products." },
    { q: "Do I need prior design experience?", a: "No. The curriculum starts from fundamentals and progressively builds toward advanced AI-driven workflows." },
    { q: "How long does it take?", a: "Most members make meaningful progress within a few weeks of consistent practice." },
    { q: "Is there a community?", a: "Yes. You get access to a private community of designers and founders building with AI." },
  ],
  ai: [
    { q: "Which AI tools do you cover?", a: "We focus on Lovable, Figma AI, image generation models, and the workflows that tie them together into a real design process." },
    { q: "Will AI replace designers?", a: "No. Designers who use AI fluently will replace those who don't. The course teaches you to be the former." },
    { q: "Do I need API keys?", a: "No. Everything you need is included — no separate API keys, subscriptions, or hidden setup." },
    { q: "Can I use these skills with any tool?", a: "Yes. The principles transfer across tools — you'll learn frameworks, not button-clicks." },
    { q: "How often is the content updated?", a: "Regularly. As AI tools evolve, we update the curriculum so you're always learning what's current." },
  ],
  integrations: [
    { q: "Which tools does UI Rocket integrate with?", a: "UI Rocket works alongside Lovable, Figma, and the most common modern design and dev tools." },
    { q: "Is my data secure?", a: "Yes. Your data is encrypted in transit and at rest, and never shared with third parties." },
    { q: "Is my data used to train AI models?", a: "No. Your work and account data are never used to train AI models." },
    { q: "Who can access our workspace data?", a: "Only members you explicitly invite. Access is fully under your control." },
    { q: "Where is my data stored?", a: "On secure cloud infrastructure with industry-standard compliance and backups." },
  ],
};
```

### Section Layout (`FaqSection.tsx`)

- `<section id="faq" className="relative w-full bg-background py-12 sm:py-16">`
- Inner container: `mx-auto max-w-[1080px] px-4 sm:px-6`

### Header (top, two-column on lg)
- Flex column on mobile, `lg:flex-row lg:items-end lg:justify-between`, `mb-14`, `gap-10`.
- Left block (`max-w-2xl`):
  - Pill (FadeUp delay 1): `inline-flex items-center gap-2 rounded-full bg-landing-surface border border-white/10 px-3 py-1 text-xs text-foreground/80 backdrop-blur`, with leading `1.5×1.5` dot `bg-foreground/70`, label "FAQ".
  - Heading (FadeUp delay 0.1): `text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground` — "Answers to the questions" `<br className="hidden sm:block"/>` " that come up most."
- Right paragraph (FadeUp delay 0.2): `max-w-sm text-sm sm:text-base text-foreground/60` — "Learn how UI Rocket works, what it covers, how the workflow flows, and what you can expect day to day."

### Body grid
`grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-8 lg:gap-12 items-stretch`

### Left column (sticky category list + "Got Questions" card)
- Outer `flex flex-col gap-4 lg:h-full`.
- Top wrapper `lg:flex-1` containing a `SpotlightBorder` (radius `2xl`, size 280) with classes `flex flex-col p-2 sm:p-3 lg:sticky lg:top-24`.
  - Inside, map `categories`. Each item is itself a `SpotlightBorder as="button" radius="full" size={200} intensity={0.4}` with classes:
    - Base: `w-full text-center px-5 py-3 text-sm transition-colors`
    - Active: `bg-landing-surface border border-white/10 text-foreground`
    - Inactive: `border border-transparent text-foreground/60 hover:text-foreground`
  - `onClick` sets `active` state (`useState<CategoryKey>("general")`).
- Bottom "Got Questions?" card: `SpotlightBorder` radius `2xl` size 360 with `mt-8 lg:mt-0 p-2 sm:p-3`, containing nested `SpotlightBorder` radius `2xl` size 260 intensity 0.4 with `border border-white/10 bg-landing-surface p-6`:
  - `<h3 className="text-lg font-semibold text-foreground">Got Questions?</h3>`
  - `<p className="mt-2 text-sm text-foreground/60 leading-relaxed">Need help with something? Our team is here to make things easy. Don't hesitate to reach out.</p>`
  - `<a href="mailto:hello@uirocket.com" className="mt-6 inline-flex items-center gap-1 text-sm text-foreground hover:text-foreground/80">Email us <span aria-hidden>→</span></a>`

### Right column (accordion)
- Outer `SpotlightBorder radius="2xl" size={360} className="p-2 sm:p-3"`.
- Inside: `<Accordion type="single" collapsible className="flex flex-col gap-3">`.
- Use `itemRefs = useRef<Array<HTMLDivElement|null>>([])` and a `useEffect` that adds a `mousemove` listener writing `--spot-x`/`--spot-y` on each item's bounding rect (so each card has its own spotlight).
- Map `faqs[active]`. Each entry wrapped in `<FadeUp delay={0.15 * idx} key={`${active}-${idx}`}>`.
- `AccordionItem`:
  - `value={`${active}-${idx}`}`, ref into `itemRefs`.
  - Classes: `relative rounded-2xl border border-white/10 bg-landing-surface px-6 [&[data-state=open]]:bg-landing-surface-hover`
  - Inside, an absolute-inset `<span aria-hidden className="pointer-events-none absolute inset-0 rounded-2xl" style={spotlightMaskStyle(260, 0.4)} />` to render the per-card spotlight ring.
- `AccordionTrigger`: `py-7 text-left text-sm sm:text-base font-medium text-foreground hover:no-underline [&>svg]:hidden`
  - Children: `<span className="flex-1 pr-4">{q}</span>` and a 28px circular icon button: `flex h-7 w-7 items-center justify-center rounded-full border border-white/15 bg-white/[0.04] text-foreground/70 transition-transform duration-200 group-data-[state=open]:rotate-180`, containing `<MIcon name="expand_more" size={16} />`.
- `AccordionContent`: `pb-7 text-sm text-foreground/60 leading-relaxed` — render `{a}`.

### Behavior

- Switching category re-keys the AccordionItem `value` prefix so all items collapse when category changes.
- Only one item open at a time (`type="single" collapsible`).
- Spotlight ring follows mouse on the outer wrapper containers AND on every individual accordion item independently.
- All entrance animations use FadeUp; reduced-motion disables translate but keeps opacity.

### Acceptance Checklist

- Dark `#000000` background, Inter text, max width 1080px.
- Header: small pill with dot + "FAQ", large heading "Answers to the questions / that come up most.", right-side paragraph aligned to bottom on lg.
- Two-column body: left 280px sticky category list with pill buttons (active = filled surface, inactive = transparent), plus "Got Questions?" card with email CTA arrow.
- Right column: cards stacked with `gap-3`, rounded-2xl, surface fill, open state slightly brighter, 28px circular chevron button rotating 180° on open, content fades/animates via Radix accordion keyframes.
- 1px spotlight ring follows cursor on every SpotlightBorder wrapper and every accordion card.
- Scroll-in FadeUp stagger on header items and on each FAQ row (`0.15 * idx`).
- Switching categories collapses any open item and swaps the question list.

## 3D Studio Pricing — Pricing [sections/3d-studio-pricing]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/3d-studio-pricing.mp4

Create a full-screen hero section with a looping video background and a pricing card overlay. Use React with Tailwind CSS and Lucide React icons. The font is "Geist" loaded from Google Fonts.

**Video Background:**
- Full-screen `<video>` element, autoplaying, looped, muted, playsInline
- Covers the entire viewport with `object-cover`, positioned `object-right` on mobile, `object-center` on md+
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_102608_5fa1187d-9ac6-44fb-82ab-54376200abc0.mp4`

**Layout:**
- Section is `relative min-h-screen w-full overflow-hidden`, uses `font-geist` (custom Tailwind font family mapped to 'Geist')
- Content sits at `z-10`, arranged as a flex column filling min-h-screen
- Top area has generous padding (`pt-10 sm:pt-16 md:pt-24 px-5 sm:px-8 md:px-16 lg:px-24`)
- A flex-1 spacer pushes the bottom card down
- Bottom area has padding `px-4 sm:px-8 md:px-16 lg:px-24 pb-8 sm:pb-12 md:pb-20`

**Heading (top-left):**
- Text: "Studio" on line 1, "rate" on line 2
- Size: `text-[clamp(2.5rem,12vw,10rem)]`
- Weight: `font-light`, line-height: `leading-[0.9]`, tracking: `tracking-[-0.03em]`, color: white

**Pricing Card (bottom):**
- Outer container: full-width, `border border-white/10 rounded-2xl bg-black/70 backdrop-blur-md`, padding `p-3 sm:p-4 md:p-5`, fixed height `h-[280px] sm:h-[310px] md:h-[340px]`, flex column
- Inside: a 3-column grid (`grid-cols-1 md:grid-cols-3 gap-4 md:gap-5 items-stretch flex-1 min-h-0`)

**Column 1 - Toggle card:**
- `bg-[#11120F]/60 backdrop-blur-xl border border-white/10 rounded-xl p-5 sm:p-6`, flex column justify-between
- Heading: "Want cinematic fidelity?" - white, `text-base sm:text-lg font-medium mb-2`, left-aligned
- Description: "Activate photorealistic global illumination with sub-surface scattering and volumetric atmosphere depth." - `text-white/60 text-sm leading-relaxed`, left-aligned
- Bottom row: price "+$520" (`text-white text-lg font-light`) on left, custom toggle on right
- Toggle: `w-14 h-7 rounded-full`, active color `bg-[#B2D770]`, inactive `bg-white/20`, with a `w-5 h-5` black circle thumb that translates left/right. Uses React state.

**Column 2 - Price display:**
- `flex flex-col justify-between py-3 sm:py-4 md:py-5`
- Main price: "$3,180" at `text-4xl sm:text-5xl md:text-5xl lg:text-7xl font-light text-white tracking-tight`
- Suffix: "/deliver" in `text-white/50 text-sm sm:text-base ml-1`
- Bottom row: "Rush-mode" (`text-white/50 text-sm`) left, "12-36 hours" (`text-white/80 text-sm font-medium`) right

**Column 3 - Features + CTA:**
- `flex flex-col justify-between py-3 sm:py-4 md:py-5`
- 4 feature items with `CheckCircle2` icon (from lucide-react) in `text-[#B2D770] w-4 h-4 sm:w-5 sm:h-5` and label in `text-white/70 text-sm`:
  1. "Boundless iterations"
  2. "Cinema 8K mastergrade"
  3. "Bespoke 3D materials"
  4. "Dedicated render engineer 24/7"
- CTA button: split into two segments with `gap-[3px]`, both `bg-[#B2D770] text-black rounded-lg py-2.5`
  - Left: "Start a brief" with `text-sm font-medium px-4 sm:px-5`
  - Right: `ArrowUpRight` icon (lucide-react), `w-10` square-ish

**Accent Color:** `#B2D770` (lime green) used for toggle active state, checkmark icons, and CTA button.

**Global CSS (index.css):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: 'Geist', sans-serif;
  background: #000;
  color: #fff;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

**Tailwind Config:**
- Extends `fontFamily` with `geist: ['Geist', 'sans-serif']`

**Google Font (in index.html head):**
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Geist:wght@100;200;300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

## Nex Max Upgrade — Pricing [sections/nex-max-upgrade]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nex-max-upgrade.webp

Build a single full-viewport React + TypeScript section (Vite, Tailwind available but styles written via a <style> block) that renders a fixed background video with a pricing-style glass card overlay. The video must play back and forth in a boomerang loop via throttled manual seeking (no native .play()).

Video
Source URL (exact): 
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_064209_0cb7d815-ff61-4caa-a6d5-bbff145ab272.mp4

<video> attributes: muted, playsInline, crossOrigin="anonymous", preload="auto", src={VIDEO_SRC}, attached ref.
The video is wrapped in a div.c4-video-wrap that is position: fixed; inset: 0; width:100%; height:100%; z-index:0; pointer-events:none; overflow:hidden.
The video itself is width:100%; height:100%; object-fit:cover; transform: scale(1.35).
Boomerang playback logic (useEffect on mount)
Refs: videoRef, directionRef (1 | -1, default 1), rafRef (number | null), lastTickRef (number, default 0).
Constants: SEEK_INTERVAL_MS = 33, STEP_SECONDS = 0.05.
On loadedmetadata (or immediately if readyState >= 1):
video.pause().
Start a requestAnimationFrame loop loop(now):
If !video.duration || isNaN(video.duration) → schedule next frame and return.
If now - lastTickRef.current >= SEEK_INTERVAL_MS && !video.seeking:
lastTickRef.current = now
next = video.currentTime + STEP_SECONDS * directionRef.current
If next >= video.duration: clamp to duration, flip direction to -1.
Else if next <= 0: clamp to 0, flip direction to 1.
Assign video.currentTime = next.
Schedule next frame.
Cleanup: cancelAnimationFrame(rafRef.current) if set.
The !video.seeking guard is critical — it tells the browser to only issue the next seek once the previous frame has finished decoding/rendering.

Layout / content (inside div.c4-content, max-width 700px, padding 20px, z-index 1)
h1.c4-title — "Power up with Nex Max"
p.c4-subtitle — "Access more tools with a single bundle."
div.c4-grid (two columns) containing two div.c4-card cards:
Card 1 (Base):
Header: span.c4-tier "Base"
div.c4-price "$0"
div.c4-list-title "Contains"
ul.c4-list with items: "Talk with your tabs", "Custom Macros", "An elite web-based tool"
Card 2 (Max):
Header: span.c4-badge "MAX" + span.c4-trial "14-day sample run"
div.c4-price "$25" + inner <span> "a month"
div.c4-list-title "Has all the tools from Base, plus"
ul.c4-list with one item: "Nex unlocked. Chat as much as you want, without meeting limits.*"
button.c4-btn containing an inline Apple-logo <svg> (14×14, viewBox 0 0 24 24, path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.15 2.65.72 3.4 1.8-3.02 1.93-2.48 5.68.3 6.84-.66 1.76-1.5 3.33-2.35 4.37zm-2.9-15.18c-.46 2.06-2.45 3.48-4.41 3.2.14-2.18 1.93-3.8 3.9-3.95.12 1.54-.36 2.39.51.75z") and text "Download Nex to start".
p.c4-footer — * if your usage aligns with our <a href="#">Usage Policy</a>, naturally.
Font
Import Google Fonts Inter (weights 300, 400, 500, 600) inside the <style> block:
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap');
Apply font-family: 'Inter', sans-serif to body.

CSS (exact)

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: 'Inter', sans-serif;
  background: #050505;
  color: white;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  position: relative;
  text-align: center;
  padding: 40px 0;
}

#root {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  position: relative;
  z-index: 1;
}

.c4-video-wrap {
  position: fixed; inset: 0;
  width: 100%; height: 100%;
  z-index: 0; pointer-events: none;
  overflow: hidden;
}
.c4-video-wrap video {
  width: 100%; height: 100%;
  object-fit: cover;
  transform: scale(1.35);
}

.c4-content { z-index: 1; width: 100%; max-width: 700px; padding: 20px; }

.c4-title {
  font-size: 2.5rem; font-weight: 300; margin-bottom: 10px;
  background: linear-gradient(to right, #737373, #ffffff);
  -webkit-background-clip: text; -webkit-text-fill-color: transparent;
  background-clip: text; display: inline-block;
}
.c4-subtitle { font-size: 1rem; color: #a3a3a3; margin-bottom: 40px; }

.c4-grid {
  display: grid; grid-template-columns: 1fr 1fr; gap: 20px;
  margin-bottom: 40px; text-align: left;
}
.c4-card {
  background: rgba(255,255,255,0.05);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 20px; padding: 30px;
  backdrop-filter: blur(10px);
}
.c4-card-header {
  display: flex; justify-content: space-between;
  align-items: center; margin-bottom: 20px;
}
.c4-tier { font-size: 0.85rem; color: #a3a3a3; }
.c4-badge {
  background: white; color: black;
  font-size: 0.7rem; font-weight: 600;
  padding: 2px 6px; border-radius: 4px;
}
.c4-trial {
  border: 1px solid rgba(255,255,255,0.2);
  font-size: 0.75rem; padding: 4px 10px;
  border-radius: 20px; color: #d4d4d4;
}
.c4-price {
  font-size: 2.5rem; font-weight: 400; margin-bottom: 30px;
  display: flex; align-items: baseline;
}
.c4-price span { font-size: 0.9rem; color: #a3a3a3; margin-left: 5px; }
.c4-list-title { font-size: 0.85rem; font-weight: 500; margin-bottom: 15px; }
.c4-list { list-style: none; }
.c4-list li {
  font-size: 0.85rem; color: #d4d4d4;
  margin-bottom: 12px;
  display: flex; align-items: flex-start; gap: 8px;
}
.c4-list li::before { content: '\2713'; color: #fff; }

.c4-btn {
  background: white; color: black; border: none;
  padding: 12px 24px; border-radius: 24px;
  font-weight: 500; font-size: 0.9rem; cursor: pointer;
  display: inline-flex; align-items: center; gap: 8px;
  margin-bottom: 20px;
}
.c4-footer { font-size: 0.7rem; color: #737373; }
.c4-footer a { color: #a3a3a3; text-decoration: underline; }

@media (max-width: 768px) {
  .c4-title { font-size: 2rem; }
  .c4-subtitle { font-size: 0.9rem; margin-bottom: 30px; }
  .c4-grid { grid-template-columns: 1fr; gap: 15px; }
  .c4-card { padding: 25px; }
  .c4-price { font-size: 2rem; }
  .c4-content { padding: 15px; }
}
@media (max-width: 480px) {
  .c4-title { font-size: 1.75rem; }
  .c4-subtitle { font-size: 0.85rem; }
  .c4-card { padding: 20px; }
  .c4-price { font-size: 1.75rem; }
  .c4-list li { font-size: 0.8rem; }
  .c4-btn { padding: 10px 20px; font-size: 0.85rem; }
}
Animations
No CSS keyframe animations.
The sole "animation" is the boomerang video playback driven by the RAF loop described above (33 ms throttle, 0.05 s step, reverses at both ends, skips ticks while video.seeking is true).

## NimBus Pricing — Pricing [sections/nimbus-pricing]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nimbus-pricing.webp

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

**Shared eyebrow style:**
```css
.eyebrow {
  margin: 0 0 16px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 400;
  line-height: 1.18;
  letter-spacing: 0.04rem;
  text-transform: uppercase;
}
```

**Shared h2 base:**
```css
h2 {
  max-width: 920px;
  margin-bottom: 0;
  font-size: clamp(25px, 4vw, 52px);
  line-height: 1.08;
  letter-spacing: 0.005rem;
  font-weight: 400;
}
```

---

### Section: `.pricing-section`

`<section class="pricing-section" id="pricing" aria-labelledby="pricing-title">`

**Outer container:**
```css
.pricing-section {
  position: relative;
  min-height: auto;
  padding: clamp(72px, 9vw, 118px) clamp(20px, 5vw, 72px) 0;
  overflow: hidden;
  border-top: 1px solid rgba(255, 240, 199, 0.1);
  background:
    linear-gradient(180deg, rgba(255, 240, 199, 0.05), transparent 44%),
    #11120f;
}
```

**Cyan radial blur (decorative `::before`):**
```css
.pricing-section::before {
  content: "";
  position: absolute;
  top: -27vw;
  left: -21vw;
  z-index: 1;
  width: 69vw;
  height: 69vw;
  pointer-events: none;
  background: radial-gradient(
    circle,
    rgba(151, 211, 235, 0.14) 0%,
    rgba(151, 211, 235, 0.07) 34%,
    rgba(151, 211, 235, 0) 68%
  );
  filter: blur(22px);
}
```

---

### Part 1: `.pricing-top` (two-column grid: copy + table)

```html
<div class="pricing-top">
  <div class="pricing-copy">...</div>
  <div class="pricing-table" aria-label="Nimbus Grid pricing examples">...</div>
</div>
```

```css
.pricing-top {
  position: relative;
  z-index: 2;
  display: grid;
  grid-template-columns: minmax(280px, 0.38fr) minmax(360px, 0.62fr);
  gap: clamp(42px, 8vw, 118px);
  max-width: 1320px;
  margin-inline: auto;
}
```

### Left: `.pricing-copy`

```html
<div class="pricing-copy">
  <p class="eyebrow">Pricing</p>
  <h2 id="pricing-title">Only pay for cloud storage your teams actually use.</h2>
  <p>
    Scale capacity up for active projects and cool it down when workspaces go quiet.
    Nimbus Grid keeps storage, transfer, and policy costs visible before they become invoices.
  </p>
</div>
```

```css
.pricing-copy h2 {
  max-width: 560px;
  margin-bottom: 54px;
  font-size: clamp(34px, 4vw, 68px);
  line-height: 1;
}

.pricing-copy p:not(.eyebrow) {
  max-width: 470px;
  color: var(--muted);
  font-size: clamp(15px, 1.2vw, 19px);
  line-height: 1.55;
}
```

### Right: `.pricing-table`

```html
<div class="pricing-table" aria-label="Nimbus Grid pricing examples">
  <div class="pricing-table-header">
    <h3>Storage costs</h3>
    <div class="billing-toggle" aria-label="Billing mode">
      <span>Per month</span>
      <strong>Per GiB</strong>
    </div>
  </div>
  <div class="pricing-row">
    <span>Encrypted active storage</span>
    <strong>$0.021 / GiB / month</strong>
  </div>
  <div class="pricing-row">
    <span>Warm collaboration tier</span>
    <strong>$0.012 / GiB / month</strong>
  </div>
  <div class="pricing-row">
    <span>Cold retained archive</span>
    <strong>$0.004 / GiB / month</strong>
  </div>
  <div class="pricing-row">
    <span>Regional accelerated transfer</span>
    <strong>$0.018 / GiB moved</strong>
  </div>
  <div class="pricing-row">
    <span>Customer-managed key vault</span>
    <strong>included</strong>
  </div>
</div>
```

```css
.pricing-table {
  display: grid;
  align-content: start;
  color: var(--muted);
}

.pricing-table-header,
.pricing-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 24px;
  border-bottom: 1px solid rgba(255, 247, 222, 0.2);
  padding: 18px 0;
}

.pricing-table-header {
  padding-top: 0;
}

.pricing-table h3 {
  margin-bottom: 0;
  color: var(--ink);
  font-size: clamp(20px, 1.7vw, 28px);
  font-weight: 400;
  line-height: 1.2;
  letter-spacing: 0.0125rem;
}

.billing-toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 999px;
  background: rgba(255, 247, 222, 0.1);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.02rem;
}

.billing-toggle span {
  padding-inline: 10px;
  color: rgba(255, 247, 222, 0.55);
}

.billing-toggle strong {
  padding: 7px 12px;
  border-radius: 999px;
  background: var(--accent);
  color: #241d0f;
  font-weight: 500;
}

.pricing-row span,
.pricing-row strong {
  font-size: clamp(14px, 1.2vw, 18px);
  font-weight: 400;
}

.pricing-row strong {
  color: var(--ink);
  font-family: var(--font-mono);
}
```

---

### Part 2: `.pricing-plan-row` (3 plan cards)

```html
<div class="pricing-plan-row">
  <div class="pricing-plan starter">
    <h3>Starter</h3>
    <p>For small teams consolidating shared project files.</p>
    <a href="#pricing">Start small</a>
  </div>
  <div class="pricing-plan team">
    <h3>Team</h3>
    <p>For departments scaling collaboration and regional transfer.</p>
    <a href="#pricing">Build team plan</a>
  </div>
  <div class="pricing-plan enterprise">
    <h3>Enterprise</h3>
    <p>For organizations prioritizing governance, residency, and support.</p>
    <a href="#plans">Talk to sales</a>
  </div>
</div>
```

```css
.pricing-plan-row {
  position: relative;
  z-index: 2;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: clamp(42px, 8vw, 120px);
  max-width: 1320px;
  margin: clamp(136px, 16vw, 224px) auto 0;
  padding-bottom: 6px;
}

.pricing-plan {
  width: min(300px, 100%);
  color: var(--ink);
}

.pricing-plan h3 {
  margin-bottom: 22px;
  font-size: clamp(20px, 1.8vw, 30px);
  font-weight: 400;
  line-height: 1.2;
  letter-spacing: 0.0125rem;
}

.pricing-plan p {
  margin-bottom: 24px;
  color: var(--muted);
  line-height: 1.55;
}

.pricing-plan a {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 42px;
  padding: 0 18px;
  border: 1px solid rgba(255, 247, 222, 0.24);
  border-radius: 999px;
  color: var(--ink);
  background: rgba(255, 247, 222, 0.08);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1rem;
  letter-spacing: 0.04rem;
  text-transform: uppercase;
  transition: border-color 160ms ease, background 160ms ease;
}

.pricing-plan a:hover {
  border-color: rgba(255, 247, 222, 0.48);
  background: rgba(255, 247, 222, 0.14);
}
```

---

### Part 3: `.pricing-bars` (the golden bars at the bottom)

12 full-bleed vertical bars aligned to the bottom edge, with scroll-driven height animation.

```html
<div class="pricing-bars" aria-hidden="true">
  <div class="pricing-bar" style="--bar-height: 66%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 58%;"></div>
  <div class="pricing-bar" style="--bar-height: 50%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 62%;"></div>
  <div class="pricing-bar" style="--bar-height: 45%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 54%;"></div>
  <div class="pricing-bar" style="--bar-height: 48%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 64%;"></div>
  <div class="pricing-bar" style="--bar-height: 72%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 70%;"></div>
  <div class="pricing-bar" style="--bar-height: 78%;"></div>
  <div class="pricing-bar muted" style="--bar-height: 82%;"></div>
</div>
```

Note the alternating pattern: regular, `.muted`, regular, `.muted`, etc. The 12 base heights are: `66%, 58%, 50%, 62%, 45%, 54%, 48%, 64%, 72%, 70%, 78%, 82%`.

```css
.pricing-bars {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  align-items: end;
  width: 100vw;
  height: 480px;
  margin-top: 36px;
  margin-left: calc(50% - 50vw);
}
```

**Fade overlay (dark gradient from top, fading bars into section background):**
```css
.pricing-bars::before {
  content: "";
  position: absolute;
  inset: -28% 0 0;
  z-index: -1;
  background: linear-gradient(
    180deg,
    rgba(17, 18, 15, 0),
    rgba(17, 18, 15, 0.78) 36%,
    #11120f 100%
  );
  pointer-events: none;
}
```

This `::before` covers the top ~28% above and the top portion of the bars container, creating the effect where the bars fade from the dark section background into visibility as they descend.

**Individual bar:**
```css
.pricing-bar {
  height: calc(var(--bar-height) + var(--bar-morph, 0px));
  min-height: 120px;
  background:
    radial-gradient(circle at 30% 18%, rgba(255, 247, 222, 0.32), transparent 26%),
    linear-gradient(180deg, rgba(234, 208, 154, 0.82), rgba(87, 76, 43, 0.42));
  box-shadow: inset 0 1px 0 rgba(255, 247, 222, 0.25);
  transition: height 80ms linear;
}

.pricing-bar.muted {
  background:
    radial-gradient(circle at 26% 20%, rgba(255, 247, 222, 0.24), transparent 28%),
    linear-gradient(180deg, rgba(201, 180, 124, 0.7), rgba(78, 69, 42, 0.38));
}
```

**Bar visual details:**
- Each bar has a gold gradient from top to bottom.
- A radial-gradient highlight near the top-left gives a specular/glow spot.
- `box-shadow: inset 0 1px 0` creates a subtle bright top edge.
- `.muted` bars use a slightly dimmer, cooler gold palette — giving the alternating light/dark bar pattern visible in the screenshot.
- Bars have no gap between them (the grid columns are flush).
- `min-height: 120px` ensures bars always have substance even at small percentages.

---

### JavaScript: Scroll-driven bar animation

```js
const pricingSection = document.querySelector(".pricing-section");
const pricingBars = Array.from(document.querySelectorAll(".pricing-bar"));

function updatePricingBars() {
  if (!pricingSection || !pricingBars.length) return;

  const rect = pricingSection.getBoundingClientRect();
  const viewport = window.innerHeight || 1;
  const progress = Math.min(1, Math.max(0,
    (viewport - rect.top) / (viewport + rect.height)
  ));

  pricingBars.forEach((bar, index) => {
    const wave = Math.sin(progress * Math.PI * 2 + index * 0.72);
    const secondaryWave = Math.cos(progress * Math.PI + index * 0.34);
    const morph = Math.round(wave * 34 + secondaryWave * 14);
    bar.style.setProperty("--bar-morph", `${morph}px`);
  });
}

window.addEventListener("scroll", updatePricingBars, { passive: true });
window.addEventListener("resize", updatePricingBars);
updatePricingBars();
```

**How the bar animation works:**
- `progress` is 0 when the section enters the bottom of the viewport, and approaches 1 as you scroll past.
- For each bar, two sinusoidal waves are computed, offset by the bar index (`index * 0.72` and `index * 0.34`).
- `wave * 34 + secondaryWave * 14` produces a morph offset in the range of roughly -48px to +48px.
- This value is written to `--bar-morph`, which is added to the base `--bar-height` via `calc()`.
- The `transition: height 80ms linear` smooths the per-frame jitter into a fluid wave.
- The result: bars gently undulate as you scroll, each offset from its neighbor, creating a rolling-wave effect across the 12 columns.

---

### Responsive Breakpoints

### `@media (max-width: 820px)`

```css
.pricing-section {
  padding-top: 64px;
}

.pricing-top {
  grid-template-columns: 1fr;
  gap: 38px;
}

.pricing-copy h2 {
  margin-bottom: 28px;
}

.pricing-table-header,
.pricing-row {
  grid-template-columns: 1fr;
  gap: 8px;
}

.pricing-bars {
  height: 480px;
}

.pricing-plan-row {
  grid-template-columns: 1fr;
  gap: 28px;
  margin-top: 48px;
}
```

At 820px: Pricing top stacks to single column (copy above table). Table rows become single column (label above value). Plan cards stack vertically. Bars stay at 480px.

### `@media (max-width: 520px)`

```css
.pricing-bars {
  height: 480px;
}

.pricing-plan {
  width: min(280px, 100%);
}

.eyebrow {
  font-size: 12px;
}
```

At 520px: Plan card max-width narrows. Eyebrow shrinks. Bars remain 480px.

---

### Project structure

```
index.html       (section markup + font links)
styles.css       (all styles + media queries)
script.js        (scroll-driven bar morph)
package.json     (vite ^5.4.2, "type": "module", scripts: dev/build/preview)
vite.config.js   (default export)
```

## What Package Fits You — Pricing [sections/package-fits-pricing]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/package-fits-pricing.gif

Build a single standalone HTML file (component2.html) for a pricing-packages section. Use one <style> block and one inline <script>. No frameworks. CSS class names must be prefixed with c2- to avoid collisions.

Fonts
Import via Google Fonts in <style>:

@import url('https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,500;1,400&family=Inter:wght@300;400;500;600&family=Space+Grotesk:wght@400;500;600&family=Cormorant+Garamond:wght@400;500;600&display=swap');
Body default: 'Inter', sans-serif
Title: 'Playfair Display', serif, 400 weight
Prices: 'Cormorant Garamond', serif, 400 weight, letter-spacing -0.02em
Page setup
* { box-sizing: border-box; margin: 0; padding: 0; }
body: background #F0EFED, padding 48px 20px, flex centered, min-height 100vh, antialiased font smoothing.
.c2-container: max-width: 800px; width: 100%.
Header
Eyebrow <p class="c2-eyebrow">PRICING</p>: centered, 0.75rem, letter-spacing 0.18em, uppercase, color #888, margin-bottom 14px. Animates in via c2-fadeUp 0.7s cubic-bezier (.16, 1, .3, 1).
Title <h2 class="c2-title">What package fits <em>you?</em></h2>:
2.8rem, centered, color #2a2a2a, font-weight 400, letter-spacing -0.01em, margin-bottom 36px.
The <em> is italic with color #5a5a5a.
Animates in via c2-fadeUp 0.8s 0.1s cubic-bezier (.16, 1, .3, 1).
Grid wrapper
.c2-grid-wrapper: background #E8E7E5, border-radius 20px, padding 16px, margin-bottom 20px. Animates in c2-fadeUp 0.9s 0.2s.
.c2-grid: CSS grid, two equal columns, gap 20px.
Cards (two of them inside the grid)
.c2-card: white background, border-radius 16px, padding 24px, position relative, overflow hidden, box-shadow 0 2px 10px rgba(0,0,0,0.03), flex column. Transition transform 0.35s, box-shadow 0.35s (same easing). On :hover lifts translateY(-4px) and shadow becomes 0 12px 32px rgba(0,0,0,0.08).

Four 6×6 corner dots, color #d0d0d0, border-radius 50%, pointer-events none, 12px from each corner:

Top-left and top-right via ::before and ::after.
Bottom-left and bottom-right as inline <div class="c2-dot-bl"></div> and <div class="c2-dot-br"></div> placed inside the card.
Card title (.c2-card-title): 1.3rem, font-weight 500, margin-bottom 8px, color #2a2a2a, letter-spacing -0.01em.

Price block (.c2-price): Cormorant Garamond, 3.2rem, baseline-aligned flex with gap 8px, padding-bottom 12px, border-bottom 1px solid #e5e5e5, min-height 76px. The <span> for the unit text inside is Inter, 0.85rem, color #999, weight 300. The amount span (.c2-price-amount) has a transition opacity 0.25s, transform 0.25s; class c2-flip sets opacity: 0; transform: translateY(-6px) for the price-change animation.

List (.c2-list): no list-style, font-size 0.85rem, color #555, margin-bottom 16px, padding-bottom 16px. Variants .bordered adds border-bottom: 1px solid #e5e5e5; .bordered-top adds border-top: 1px solid #e5e5e5; padding-top: 20px. Each <li>: margin-bottom 12px, flex centered, gap 8px. li::before content '❖', color #c0c0c0, font-size 0.9rem.

Description (.c2-desc): 0.85rem, color #666, line-height 1.6, max-width 210px, margin-bottom: auto.

Button (.c2-btn): black bg, white text, padding 11px 26px, border-radius 30px, font-size 0.85rem, weight 500, font-family inherit, margin-top 32px, width fit-content, z-index 2, position relative. Transition transform 0.2s, background 0.2s, box-shadow 0.2s. Hover: translateY(-1px), background #1a1a1a, shadow 0 6px 18px rgba(0,0,0,0.18). Active: translateY(0). :focus-visible: outline 2px solid #2a2a2a, outline-offset 3px.

Tree image (.c2-tree): position absolute, bottom 0, right 20px, width 120px, z-index 1, pointer-events none. Transition transform 0.5s (cubic-bezier (.16, 1, .3, 1)). On .c2-card:hover .c2-tree: translateY(-3px) rotate(-1.5deg).

Card 1 — Product Design
Title: Product Design
Price: $75 + unit Hourly
List with bordered bottom, items: Experienced Designer, Fast Delivery, Conversion Focused, Tailored Design Strategy
Description: Perfect if you're looking to build a dashboard, app, etc... and get it "done-right" the first time.
Button: Contact Us with data-package="Product Design"
Tree image: https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/viktor/gold%20tree.webp, alt empty, loading="lazy"
Card 2 — Web Design
data-card="web" on the card div
Title: Web Design
Price: <span class="c2-price-amount" data-price>$1,500</span> + <span data-price-note>One-time</span>
Toggle (.c2-toggle, role="tablist", aria-label="Page count"):
Inline-flex, background #f0f0f0, border-radius 20px, padding 4px, margin-bottom 15px, font-size 0.75rem, user-select none.
Two <span> children with role="tab", tabindex="0", data-pages="single"|"multi", aria-selected. Padding 5px 14px, border-radius 16px, color #666, cursor pointer, transition background/color/box-shadow 0.25s.
.active: background white, color #1a1a1a, shadow 0 2px 5px rgba(0,0,0,0.05). Default Single-page is active and aria-selected="true"; Multi-page is aria-selected="false".
Switch row (.c2-framer, role="switch", tabindex="0", aria-checked="false", aria-label="Add Framer development"): flex centered, gap 10px, font-size 0.8rem, color #555, margin-bottom 30px, cursor pointer, user-select none, width fit-content.
Label: Framer Development
.c2-switch: 34×20px, background #e5e5e5, border-radius 10px, position relative, transition background 0.25s.
.c2-switch::after: 16×16px white circle, left 2px, top 2px, border-radius 50%, shadow 0 1px 3px rgba(0,0,0,0.15), transition left 0.25s (same easing).
.c2-framer.on .c2-switch: background #2a2a2a. .c2-framer.on .c2-switch::after: left 16px.
List with bordered bordered-top, items: Experienced Designer, Fast Delivery, Conversion Focused, 50/50 Secure Payments
Two <br> tags before the button
Button: Contact Us with data-package="Web Design"
Tree image: https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/viktor/purple%20tree.webp, alt empty, loading="lazy"
Bottom card (outside the grid wrapper)
.c2-bottom-card: background #E8E7E5, border-radius 16px, padding 32px 32px 4px 32px, flex column, position relative, overflow hidden, min-height 220px. Animates in c2-fadeUp 1s 0.35s cubic-bezier (.16, 1, .3, 1). Hide .c2-dot-bl, .c2-dot-br inside this variant. The .c2-btn inside has margin-top 60px.

Contents:

<h3 class="c2-card-title">Unique Request</h3> with weight 500
<p class="c2-bottom-desc">Are you looking for something custom?<br>Don't hesitate to contact us, and we'll help brainstorming your product to success.</p> — 0.85rem, color #666, line-height 1.6, max-width 600px, margin-bottom -20px, z-index 2, position relative.
Button: Contact Us with data-package="Custom"
Landscape image (.c2-landscape): https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/viktor/landscape.webp, position absolute, bottom 0, right 0, width 360px, z-index 1, pointer-events none, alt empty, loading="lazy".
Keyframes
@keyframes c2-fadeUp {
  from { opacity: 0; transform: translateY(18px); }
  to   { opacity: 1; transform: translateY(0); }
}
The eyebrow uses translateY(10px) initial, the title translateY(14px), grid wrapper and bottom card translateY(20px).

Media queries
@media (max-width: 768px):

body padding 32px 16px
.c2-title: 2.1rem, margin-bottom 28px
.c2-grid: grid-template-columns: 1fr
.c2-tree: width 96px, right 12px, opacity 0.85
.c2-desc, .c2-bottom-desc: max-width none
.c2-landscape: width 240px, opacity 0.85
.c2-bottom-card: padding 28px 24px 4px 24px; its .c2-btn margin-top 40px
@media (max-width: 480px):

.c2-title: 1.8rem
.c2-price: 2.6rem
.c2-landscape: width 180px
@media (prefers-reduced-motion: reduce): set animation-duration, animation-iteration-count, transition-duration to 0.01ms !important on *, *::before, *::after.

JavaScript (IIFE at end of <body>)
State for the Web Design card only:

const BASE = { single: 1500, multi: 2500 };
const FRAMER_ADDON = 800;
const state = { pages: 'single', framer: false };
render():

Compute total = BASE[state.pages] + (state.framer ? FRAMER_ADDON : 0).
Format as '$' + total.toLocaleString('en-US').
If unchanged, return early.
Add c2-flip to [data-price], then after 160ms set textContent and remove the class.
Set [data-price-note] to 'One-time + Framer' if framer is on, else 'One-time'.
selectPages(value):

No-op if same. Update state. Toggle active class and aria-selected on each tab. Call render.
Bind to each toggle option:

click → selectPages(opt.dataset.pages)
keydown: Enter/Space selects; ArrowLeft selects single, ArrowRight selects multi, then focus the just-selected element.
toggleFramer():

Flip state.framer. Toggle .on class on .c2-framer. Update aria-checked. Call render.
Bound to click and keydown (Enter/Space).
Each .c2-btn: click logs [c2] contact requested: followed by the package label; for Web Design, append the current page mode and + Framer if on.

Verification expectations
After loading at desktop width:

Default Web Design price reads $1,500 with note One-time.
Clicking Multi-page updates price to $2,500 with brief flip animation.
Toggling Framer adds $800 and changes note to One-time + Framer (so multi + framer = $3,300).
Mobile (375px): grid stacks, trees scale to 96px, landscape to 240px, no horizontal overflow.
No console errors.

## Rocket Pricing — Pricing [sections/rocket-pricing]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/rocket-pricing.webp

Build a `PricingSection` React component that matches the spec below exactly.

### Stack & global setup

- React 18 + Vite + TypeScript, TailwindCSS, `framer-motion`, `clsx` + `tailwind-merge` exposed as `cn()` from `@/lib/utils`.
- Dark theme. Page background `#000000`. Font: Inter (`font-inter`). Icons: Google **Material Symbols Outlined** (loaded globally).
- Tailwind config must include semantic HSL tokens plus:
  ```ts
  theme.extend.colors.landing = {
    surface: "rgba(255,255,255,0.10)",
    "surface-hover": "rgba(255,255,255,0.16)",
    border: "rgba(255,255,255,0.10)",
  }
  ```
- `--background` / `--foreground` HSL tokens drive `bg-background` (dark) and `text-foreground` (near-white).

### Helper components (reuse exact behavior)

### `MIcon`
Material Symbols span. Props: `name`, `size=20`, `weight=400`, `fill=0`, `grade=0`, `opticalSize=24`, `className`.
```tsx
<span
  className={cn("material-symbols-outlined select-none leading-none", className)}
  style={{
    fontSize: size,
    fontVariationSettings: `'FILL' ${fill}, 'wght' ${weight}, 'GRAD' ${grade}, 'opsz' ${opticalSize}`,
  }}
>{name}</span>
```

### `FadeUp`
`framer-motion` wrapper: `initial={{opacity:0, y:24}}`, `whileInView={{opacity:1, y:0}}`, `viewport={{once:true, amount:0.3}}`, `transition={{duration:0.6, delay, ease:[0.22,1,0.36,1]}}`. Props: `children`, `delay=0`, `className`.

### `SpotlightBorder`
1px gradient border that follows the cursor via CSS masks.
- Props: `children`, `className`, `radius="2xl"`, `size=520`, `intensity=0.5`.
- Wrapper sets CSS vars `--spot-x`, `--spot-y` (default `-9999px`) updated on `pointermove` relative to element.
- Two stacked layers using `-webkit-mask` + `mask` `linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)` with `mask-composite: exclude` to produce a 1px ring; the ring is painted with `radial-gradient(circle var(--size) at var(--spot-x) var(--spot-y), rgba(255,255,255, var(--intensity)), transparent 60%)`.
- Outer ring: `rounded-2xl border border-white/10`. Inner highlight ring: thinner, brighter on hover. Pointer events on inner content only.

### `PrimaryButton` / `SecondaryButton`
- Both: `inline-flex items-center justify-center rounded-full`, Inter, leading-none, hover text-up-from-below animation (`AnimatedText`).
- PrimaryButton: `bg-white/80 hover:bg-white text-black`. Size `sm` = `h-8 px-4 text-sm`.
- SecondaryButton: `bg-landing-surface hover:bg-landing-surface-hover border border-landing-border text-foreground backdrop-blur-[2.5px] font-medium`. Size `sm` = `h-8 px-4 text-sm`.

### Section structure — `PricingSection`

```tsx
<section id="pricing" className="relative w-full bg-background py-12 sm:py-16">
  <div className="mx-auto max-w-[1080px] px-4 sm:px-6">
    {/* HEADER */}
    <div className="mb-14 flex flex-col items-start gap-10 lg:flex-row lg:items-end lg:justify-between">
      <div className="max-w-2xl">
        <FadeUp>
          <span className="mb-6 inline-flex items-center gap-2 rounded-full bg-landing-surface border border-white/10 px-3 py-1 text-xs text-foreground/80 backdrop-blur">
            <span className="h-1.5 w-1.5 rounded-full bg-foreground/70" />
            Pricing
          </span>
        </FadeUp>
        <FadeUp delay={0.1}>
          <h2 className="text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground">
            Clear pricing plans
            <br className="hidden sm:block" /> that scale with you.
          </h2>
        </FadeUp>
      </div>
      <FadeUp delay={0.2}>
        <p className="max-w-sm text-sm sm:text-base text-foreground/60">
          One-time payment. Lifetime access. Pick the plan that fits how far
          you want to go.
        </p>
      </FadeUp>
    </div>

    {/* CARDS */}
    <div className="mx-auto grid max-w-3xl grid-cols-1 gap-6 md:grid-cols-2">
      {plans.map(p => <PricingCard key={p.name} plan={p} />)}
    </div>
  </div>
</section>
```

### Plans data (exact)

```ts
type Feature = { text: string; included: boolean };
type Plan = {
  name: string; price: string; originalPrice?: string; description: string;
  features: Feature[]; featured?: boolean; badge?: string; bg: string;
};

const plans: Plan[] = [
  {
    name: "Course",
    price: "159", originalPrice: "497",
    description: "Once. Lifetime. 68% off.",
    bg: "#161616",
    features: [
      { text: "All courses and videos", included: true },
      { text: "All modules. Lifetime access.", included: true },
      { text: "AI Builder", included: true },
      { text: "Unlimited Templates", included: false },
      { text: "Unlimited Motion Videos", included: false },
    ],
  },
  {
    name: "Course + Lovable Templates",
    price: "239", originalPrice: "697",
    description: "Once. Lifetime. Best deal.",
    bg: "#252525",
    features: [
      { text: "All courses and videos", included: true },
      { text: "All modules. Lifetime access.", included: true },
      { text: "AI Builder", included: true },
      { text: "Unlimited Templates", included: true },
      { text: "Unlimited Motion Videos", included: true },
    ],
    featured: true,
    badge: "Best Value",
  },
];
```

### `PricingCard`

```tsx
<SpotlightBorder radius="2xl" size={460} intensity={0.5}
  className="relative h-full p-2 sm:p-3">
  <div
    className="relative flex h-full flex-col rounded-2xl border border-white/10 p-7 sm:p-8"
    style={{ backgroundColor: plan.bg }}
  >
    {plan.badge && (
      <div className="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full border border-white/15 bg-white px-3 py-1 text-xs font-medium text-black">
        {plan.badge}
      </div>
    )}

    <FadeUp delay={0}>
      <div className="text-[11px] uppercase tracking-[0.2em] text-foreground/60">
        {plan.name}
      </div>
    </FadeUp>
    <div className="mt-3 border-t border-white/10" />

    <FadeUp delay={0.1}>
      <div className="mt-10 flex items-baseline gap-2">
        <span className="text-[2.75rem] leading-none font-normal tracking-tight text-foreground">${plan.price}</span>
        {plan.originalPrice && (
          <span className="text-lg text-foreground/40 line-through">${plan.originalPrice}</span>
        )}
      </div>
    </FadeUp>

    <FadeUp delay={0.2}>
      <p className="mt-4 text-sm leading-relaxed text-foreground/60">{plan.description}</p>
    </FadeUp>

    <FadeUp delay={0.3}>
      <div className="mt-7">
        {plan.featured
          ? <PrimaryButton href="/auth?mode=signup" size="sm">Get Started</PrimaryButton>
          : <SecondaryButton href="/auth?mode=signup" size="sm">Get Started</SecondaryButton>}
      </div>
    </FadeUp>

    <FadeUp delay={0.4}>
      <ul className="mt-7 flex flex-1 flex-col gap-2">
        {plan.features.map((f, i) => (
          <li key={f.text}
            className={cn(
              "flex items-center gap-3 py-4 text-sm",
              i !== 0 && "border-t border-white/10",
              f.included ? "text-foreground/85" : "text-foreground/40"
            )}>
            <span className={cn(
              "flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full border",
              f.included ? "border-white/20 bg-white/[0.06]" : "border-white/10 bg-transparent"
            )}>
              {f.included
                ? <MIcon name="check" size={12} className="text-foreground" />
                : <MIcon name="close" size={12} className="text-foreground/50" />}
            </span>
            {f.text}
          </li>
        ))}
      </ul>
    </FadeUp>
  </div>
</SpotlightBorder>
```

### Acceptance checklist

- Section id `pricing`, `bg-background`, vertical padding `py-12 sm:py-16`, max width `1080px`.
- Header: pill ("Pricing" with dot), heading "Clear pricing plans / that scale with you." (line break ≥sm), right-aligned paragraph (max-w-sm) on `lg+`. All three with staggered `FadeUp` delays 0, 0.1, 0.2.
- Cards grid: `max-w-3xl mx-auto`, `gap-6`, 1 col → 2 cols at `md`.
- Card 1 bg `#161616`, card 2 bg `#252525` with "Best Value" pill (`-top-3`, white bg, black text), featured uses `PrimaryButton`, other uses `SecondaryButton`.
- `SpotlightBorder` 1px cursor-following ring on each card (size 460, intensity 0.5).
- Card content order: eyebrow (uppercase, `text-[11px] tracking-[0.2em]`), divider, price row (`$2.75rem` + line-through original `text-foreground/40`), description, button, feature list.
- Feature rows: `py-4`, divided by `border-t border-white/10` except first. Included = 20px circle `border-white/20 bg-white/[0.06]` with check; excluded = transparent circle with close, text `text-foreground/40`.
- Inner card `FadeUp` stagger: 0, 0.1, 0.2, 0.3, 0.4.
- Buttons link to `/auth?mode=signup`.
- All colors via HSL tokens / declared landing surface tokens; never hardcode hex outside the two card backgrounds and `#000000`.

## SaaS Pricing Flow — Pricing [sections/saas-pricing-flow]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/saas-pricing-flow.webp

**Prompt:**

Build a React + TypeScript + Vite pricing page section with a full-screen background video and three pricing cards. Use the Inter font (no import needed, system fallback to sans-serif).

**Background video:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_064122_c4750c0e-7476-4b44-94a2-a85a65c63bf2.mp4`
- Fixed, full viewport (100vw x 100vh), `object-fit: cover`, `z-index: -1`, no overlays, `autoPlay`, `muted`, `playsInline`, no `loop` attribute.
- Implement **boomerang playback via throttled seek**:
  - State: `direction` (1 fwd / -1 rev), `currentTarget` (sec), `seekPending` flag, `lastTs`, `rafId`.
  - On `ended`: set `direction = -1`, `currentTarget = video.duration`, `seekPending = false`, start `requestAnimationFrame(step)`.
  - `step(ts)`: compute `dt` from `lastTs`, decrement `currentTarget` by `dt`. If `<= 0`, set `direction = 1`, `video.currentTime = 0`, call `video.play()`, return. Else call `doSeek()` and request next frame.
  - `doSeek()`: if `video.seeking`, set `seekPending = true` and return (never stack seeks). Else clear `seekPending` and set `video.currentTime = currentTarget`.
  - On `seeked`: if `direction === -1 && seekPending`, call `doSeek()` again.

**Header (`.c3-header`)** — centered, max-width 1100px, margin-bottom 40px:
- Logo (absolute left): `https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/Tests/logoipsum-415.svg`, 32px wide, `filter: brightness(0) invert(1)`.
- Nav pill (`.c3-nav`) centered: `background: rgba(30,35,45,0.75)`, `backdrop-filter: blur(10px)`, border-radius 100px, 1px border `rgba(255,255,255,0.15)`, inset shadow. Links: Home, Pricing (active = white/500), FAQ, Contact, and a white pill "Download" button (`.c3-nav-btn`) with black text. Font size 0.82rem, gap 20px. Close button (`.c3-nav-close`) is a 32px circle shown in mobile drawer.
- Hamburger (mobile only, absolute right): 32px circle with 3-line span.

**Watermark (`.c3-watermark-container`)** — absolute, top 150px, centered:
- `.c3-watermark-top`: "Forma AI", 2.8rem, weight 600, color `rgba(164,244,253,1)`, positioned with `top: -20px` and `margin-bottom: -90px` so it overlaps.
- `.c3-watermark-main`: "Pricing", 16rem, weight 800, line-height 0.9, letter-spacing -0.05em. Text uses gradient `linear-gradient(to right, #091020 0%, #0B2551 25%, #A4F4FD 65%, #00d2ff 100%)` clipped to text. Apply `filter: url(#c3-noise)`.
- Include inline SVG filter `#c3-noise` with `feTurbulence` (`fractalNoise`, baseFrequency 0.5, 2 octaves), `feComponentTransfer` slope 0.075 alpha, `feComposite` in, `feBlend` overlay.

**Grid (`.c3-grid`)** — 3 columns, gap 24px, max-width 1100px, margin-top 175px, `transform: translateX(20px)`.

**Cards (`.c3-card`)**:
- Background `linear-gradient(135deg, rgba(0,0,0,0.7), rgba(0,0,0,0.4))`, `backdrop-filter: blur(14px) brightness(0.91)`, 1px white border, `border-radius: 44px`, padding `50px 24px`, min-height 580px, `transition: all 0.6s cubic-bezier(0.22,1,0.36,1)`.
- `::before` overlay: `linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0) 50%)`.
- Hover: background `rgba(15,15,15,0.6)`, border `rgba(34,211,238,0.7)`, `transform: translateY(-12px) scale(1.01)`.
- `.c3-card-pro` variant has slightly darker gradient.
- Each card has tier label (`.c3-tier-small`, 1.1rem, weight 400, `rgba(255,255,255,0.6)`), price (`.c3-tier-large`, 2.8rem, weight 500, letter-spacing -0.02em), description (`.c3-desc`, 0.88rem, `rgba(255,255,255,0.45)`, min-height 3.2em, margin-bottom 40px), 5-item checklist, and "Choose Plan" button.

**Checklist items (`.c3-list li`)**: gap 14px, 0.92rem, `rgba(255,255,255,0.8)`, margin-bottom 18px. Check icon is a 28px circle `rgba(255,255,255,0.15)` containing a 14px white stroke SVG check (path `M20 6L9 17l-5-5`, stroke-width 4, round caps).

**Card content:**
1. **Free / Free** — "For creators taking their first steps with Forma." — Up to 3 projects in the cloud; Image export up to 1080p; Basic editing tools; Free templates and icons; Access via web and mobile app.
2. **Standard / $9,99/m (or $99,99/y)** — "For freelancers and small teams who need more freedom and flexibility." — Up to 50 projects in the cloud; Export up to 4K; Advanced editing toolkit; Team collaboration (up to 5 members); Access to premium template library.
3. **Pro / $19,99/m (or $199,99/y)** (use `.c3-card-pro`) — "For studios, agencies, and professional creators working with brands." — Unlimited projects; Export up to 8K + animations; AI-powered content generation tools; Unlimited team members; Brand customization.

**Button (`.c3-btn`)**: white bg, black text, 10px/32px padding, border-radius 100px, weight 600, 0.88rem, margin-top auto, centered. Hover: `#f5f5f5`, scale 1.02, shadow.

**Yearly toggle (`.c3-toggle-wrap`)** below grid, max-width 1100px:
- `.c3-toggle`: 52x28 white pill with black 20px knob at left+4/top+4. Active state: bg `rgba(255,255,255,0.2)`, knob translates 24px right and turns white. Transition `cubic-bezier(0.4,0,0.2,1)`.
- Label "Yearly" — 1rem, weight 500, white.
- Clicking toggles `yearly` state, swapping prices between monthly ($9,99/m, $19,99/m) and yearly ($99,99/y, $199,99/y).

**Body:** `background-color: #000`, padding `40px 20px`, flex column centered, `overflow-x: hidden`.

**Responsive (`max-width: 1024px`):**
- Watermark becomes relative/centered; "Forma AI" 2rem; "Pricing" 6rem, solid `#00d2ff` (no gradient/filter).
- Grid becomes horizontal scroll-snap row (`overflow-x: auto`, `scroll-snap-type: x mandatory`, cards `flex: 0 0 320px`, `scroll-snap-align: center`), hidden scrollbar, full viewport width.
- Nav hidden by default, becomes full-screen overlay (`rgba(15,20,25,0.6)` + blur) when `.active`, links 1.5rem, close button top-right.
- Hamburger visible.

Use class prefix `c3-` throughout. All state in a single `App.tsx` component using `useState` for `menuOpen` and `yearly`, and `useRef` + `useEffect` for the video boomerang logic.

## Digital Reality — Social Media [sections/digital-reality-hero]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/digital-reality-hero.gif

Build a React functional component using Tailwind CSS that replicates a cinematic, glassmorphic social media post or digital portfolio hero section.
Structure & Layout:
The main wrapper should take up the full screen (min-h-screen) with a solid black background (bg-[#000000]), centering its contents using flexbox.
Inside, create a fixed aspect ratio card that is exactly 600px wide and 800px high using inline styles. It should have a black background, shadow-2xl, be relative, and have overflow-hidden.
Background Media:
Place a full-cover background <video> element filling the card (absolute inset-0 z-0 h-full w-full object-cover opacity-100).
The video source must strictly use this exact URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260429_212252_7d25a6d2-cf7f-465c-9bd1-a1496112806e.mp4
Make sure the video is configured exactly with autoPlay, loop, muted, and playsInline. Do not put any dark opacity gradient overlays on top of the video directly.
SVG Film Noise Overlay:
Add an absolute layer over the video to create a grainy film effect.
Set the classes to absolute inset-0 z-50 opacity-[0.06] mix-blend-overlay pointer-events-none.
Use this exact inline style for the background image to dynamically generate the CSS noise: backgroundImage: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")
Fonts Configuration (Global CSS):
Import the "Inter" font (weights 100 to 500) from Google Fonts.
Import the "Didot" font from this specific online web font URL: https://db.onlinewebfonts.com/c/251039e6849ad977a8bfc40b564dce89?family=Didot
Configure Tailwind theme variables via @theme: set --font-serif to Didot (with fallbacks), and --font-sans to Inter.
Typography & Content Container:
Create a relative z-20 content container taking h-full w-full with padding px-12 py-10, using a flex column layout.
Top Left Text: "Work fast. Live slow." styled with text-[22px], font-serif, tracking-normal, text-[#f0f0f0], and drop-shadow-md.
Add a flex-1 spacer div to push the remaining content layout to the bottom.
Bottom Content Block:
Wrap in a flex flex-col mb-12 container.
Title: "Create your digital reality." Styled with text-[38px] leading-tight font-serif text-white mb-2 tracking-normal drop-shadow-md whitespace-nowrap ml-[-0.3px].
Subtitle: "From nothing to everything, let's bring your vision to life." Styled with text-[15.5px] font-sans text-[#a3a3a3] mb-8 font-extralight tracking-wide.
The Glassmorphic Button:
Create a "Send a message" button inside a div wrapper.
Give the button the classes: group relative px-6 py-[10px] rounded-full font-sans text-[14px] text-[#e0e0e0] font-light transition-all duration-300 overflow-hidden backdrop-blur-md
Give the button the following complex inline styles:
background: 'rgba(255, 255, 255, 0.03)'
boxShadow: '0 4px 30px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 140, 70, 0.5), inset 0 0 0 1px rgba(255, 255, 255, 0.1), inset 0 -1px 2px rgba(0, 0, 0, 0.8)'
Inside the button, include an absolute top highlight effect: absolute inset-x-0 top-0 h-[20px] bg-gradient-to-b from-[#ff8c46]/10 to-transparent pointer-events-none z-0 rounded-t-full
Add an inner hover radial glow layer inside the button (opacity-0 group-hover:opacity-100 transition-opacity duration-300 z-0) with the inline style: background: 'radial-gradient(circle at center, rgba(255, 120, 50, 0.1) 0%, transparent 70%)'
The actual text inside the button should be wrapped in a span taking precedence (relative z-10 text-[14px] drop-shadow-[0_1px_2px_rgba(0,0,0,0.5)] tracking-wide).
Footer Layout:
Set up a flex row with justify-between items-end w-full.
Use classes: text-[15px] font-sans text-[#7a7a7a] font-light tracking-wide translate-y-[10px].
Left side: "your.name" with a hover:text-white transition-colors cursor-pointer.
Right side: A flex container with items-center gap-3. Include "web", "product", "brand" (all having hover:text-white transition-colors cursor-default) separated by diamond layout dividers "✦" styled with text-[11px] text-[#555] opacity-80 mt-[-2px].

## Social Media Posts — Social Media [sections/social-media-posts-hero]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/social-media-posts-hero.gif

Create a 3x2 grid of 6 social media post cards (384px x 384px each, 16px gap) centered on the page. Every card has 12px rounded corners and overflow-hidden.

Background image: Download this image and save it as public/bg.jpeg, then set it as the full-page background with background-size: cover, background-position: center, background-repeat: no-repeat on the outermost container. The container should be min-h-screen, flexbox centered, and allow overflow scrolling.

Image URL: https://media.cleanshot.cloud/media/21620/LCl5ZplnQ16cj11YhT3EoWIj6Tfpndn3OEd9I1e4.jpeg?Expires=1775649006&Signature=n3nir~OV2wZrAh~Yw8rkuVkFKm0gABTt8LqemBwvbCWoeMbn4-fcW~FEbzlhutQ7k9i9EZWqNRW4~XSoX6QnyzBv6MzFCfc0gEuKGOI6Bb7bD-ExdwZKuGDGIqRuwG7fRSHfVrl4HimKHJt9zj~NeY6-evt6HjBdEmb4sA5mWOefxDqMfWZZrUZseo0PxYnnggxHvzcdfclviUMo~A-mH8qa9MgqcRWWAp-sk6t8qM8UP0MvWkOCKFzD1-yAm4UUmy2RHtp9UiD2LFk47SjZV~4OQN~4Ogm30DBe74mkFR0-~RkPLb-M3z3UVlUNhScSI1LmMCfiK5JptwlCmFflRA__&Key-Pair-Id=K269JMAT9ZF4GZ

GOOGLE FONTS (load all in index.html via single link):

Anton (used as font-grotesk in Tailwind)
Condiment (used as font-condiment)
Barlow weights 300-700 (used as font-barlow)
Instrument Serif regular + italic (used as font-instrument)
Inter weights 400-700 (used as font-inter)
Poppins weights 300-700 (used as font-poppins)
Source Serif 4 regular + italic weights 400-600 (used as font-source-serif)
Tailwind config -- extend fontFamily with these mappings:

grotesk -> "Anton", sans-serif
condiment -> "Condiment", cursive
barlow -> "Barlow", sans-serif
instrument -> "Instrument Serif", serif
inter -> "Inter", sans-serif
poppins -> "Poppins", sans-serif
source-serif -> "Source Serif 4", serif
GLOBAL CSS (index.css) -- custom animations and classes:

@keyframes fadeInUp -- from opacity:0, translateY(30px) to opacity:1, translateY(0). Class .animate-fade-in-up uses it at 0.6s ease-out forwards.

@keyframes fadeInOverlay -- from opacity:0 to opacity:1. Class .animate-fade-in-overlay uses it at 0.4s ease-out forwards.

@keyframes fade-rise -- from opacity:0, translateY(24px) to opacity:1, translateY(0). Three classes:

.animate-fade-rise -- 0.8s ease-out both (no delay)
.animate-fade-rise-delay -- 0.8s ease-out 0.2s both
.animate-fade-rise-delay-2 -- 0.8s ease-out 0.4s both
.liquid-glass -- background: rgba(255,255,255,0.01), background-blend-mode: luminosity, backdrop-filter: blur(4px), no border, box-shadow: inset 0 1px 1px rgba(255,255,255,0.1), position relative, overflow hidden. Has a ::before pseudo-element that creates a thin gradient border effect using mask-composite: exclude with a padding: 1.4px gradient border going from rgba(255,255,255,0.45) at top/bottom to transparent in the middle.

.liquid-glass-strong -- Same concept but with backdrop-filter: blur(50px), box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15). Its ::before uses slightly stronger white values (0.5 at edges, 0.2 at 20%/80%).

CARD 1 (top-left) -- "Simplify Your Work With AI"

Black background, 384x384, 12px rounded corners, p-7, flex column justify-between
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260407_080531_1fe9b14c-9396-4b78-9372-42f4ddbd74c7.mp4
Positioned absolute top-0 right-0, height 75%, object-cover, horizontally flipped via transform: scaleX(-1), pointer-events-none, autoPlay loop muted playsInline
Gradient overlay: absolute inset-0, bg-gradient-to-t from-black via-black/40 to-transparent
Top content (z-10):
Row: Lucide Sparkles icon (w-4 h-4, amber-400) + text "The Future Is Now" in font-condiment, amber-400, text-sm, tracking-wide, with mb-3 and gap-1.5
Heading: font-grotesk (Anton), white, 32px, leading 1.05, uppercase, tracking-tight. Three lines: "Simplify" / "Your Work" / "With AI" (last line in amber-400)
Bottom content (z-10):
Paragraph: font-mono, white/60, 10px, leading-relaxed, mb-4. Text: "Automate repetitive tasks, generate content in seconds, and let intelligent tools handle the heavy lifting -- so you can focus on what truly matters."
Row with justify-between:
Left: circle (w-7 h-7, rounded-full, bg-amber-400/20) containing Lucide Zap (w-3.5 h-3.5, amber-400) + label "AI Powered" in font-mono, white/40, 8px, uppercase, tracking-widest
Right: pill button (bg-white/10, rounded-full, px-3.5 py-1.5, backdrop-blur-sm) with text "Learn More" in font-grotesk white 10px uppercase tracking-wide + Lucide ArrowRight (w-3.5 h-3.5 white)
CARD 2 (top-center) -- "Your Insights. One Clear Overview."

Black background, 384x384, 12px rounded corners
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260307_083826_e938b29f-a43a-41ec-a153-3d4730578ab8.mp4
Absolute inset-0, full width/height, object-cover, z-0, autoPlay loop muted playsInline
Gradient overlay z-[1]: bg-gradient-to-t from-black via-black/60 to-black/80
Content (z-10, flex column, h-full, justify-between, p-7):
Top section:
Pill badge using .liquid-glass class, rounded-lg, px-3 py-1.5, inline-flex, gap-2, mb-5, with .animate-fade-rise:
Inner white label: bg-white text-black rounded-md 7px font-medium px-1.5 py-0.5, text "New"
Text: "Say Hello to Corewave v3.2" in 7px font-medium white/60 font-inter
Heading: font-inter font-medium, white, 36px, leading 1.05, tracking -1.5px, .animate-fade-rise-delay. Text: "Your Insights." / "One Clear " then "Overview." in font-instrument italic font-normal text-white/80
Paragraph: font-inter, color #d4d8e8, 8.5px, leading-relaxed, max-w 240px, mt-3, opacity-90, .animate-fade-rise-delay-2. Text: "Neuralyn helps teams track metrics, goals, and progress with precision."
Bottom section (.animate-fade-rise-delay-2):
Row: "Neuralyn" in font-inter white/30 9px font-semibold tracking-tight, then a 1px gradient divider line (from-white/15 to-transparent, flex-1), then "Analytics" in font-inter white/20 7px tracking-widest uppercase
CARD 3 (top-right) -- "Work Smarter. Move Faster. AI Powers You Up."

White background, 384x384, 12px rounded corners, font-inter
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260330_153826_e9005cf7-a1c7-4c7d-886f-fea22d644a9c.mp4
Absolute inset-0, full width/height, object-cover, pointer-events-none, pt-[140px] (pushes video down), autoPlay loop muted playsInline
White fade overlay: absolute, left-0 right-0, top: 140px, height: 120px, linear-gradient(to bottom, white 0%, transparent 100%), z-10
Content (z-20, flex column, h-full):
Top area (flex-1, flex column, items-center, justify-start, pt-10, text-center, px-5):
Logo row (.animate-fade-in-up, animationDelay 0.1s, initial opacity 0): Lucide Star (w-3 h-3, fill-black) + "Stellar.ai" in 8px font-medium black
Heading (.animate-fade-in-up, delay 0.2s, initial opacity 0): 30px font-normal, leading 1.08, tracking-tight, mb-2.5. Text: "Work Smarter." / "Move Faster." / "AI Powers You Up." where last line has bg-gradient-to-r from-black via-gray-500 to-gray-400 bg-clip-text text-transparent
Paragraph (.animate-fade-in-up, delay 0.3s, initial opacity 0): 9px, text-gray-500, leading-relaxed, max-w 240px. Text: "Intelligent automation syncs with the tools you love to streamline tasks, boost output, and save time."
Bottom section (flex column, items-center, gap-2, pb-4, .animate-fade-in-up, delay 0.4s, initial opacity 0):
Small pill: rounded-full, px-2.5 py-0.5, 7px font-medium, text-white, backdrop-blur-md bg-white/15 border border-white/20. Text: "Collaborating with top aerospace pioneers globally"
Row of 5 brand names: ['Aeon', 'Vela', 'Apex', 'Orbit', 'Zeno'] each in text-base italic text-white tracking-tight with fontFamily: 'Georgia, serif', spaced with gap-5
CARD 4 (bottom-left) -- "Focus in a Distracted World"

Black background, 384x384, 12px rounded corners
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_151826_c7218672-6e92-402c-9e45-f1e0f454bdc4.mp4
Absolute inset-0, full width/height, object-cover, z-0, autoPlay loop muted playsInline
Gradient overlay z-[1]: bg-gradient-to-t from-black/70 via-transparent to-transparent
Content (z-10, flex column, h-full, justify-end, px-6, pb-7):
Heading: font-instrument, white, 32px, leading 0.95, tracking-tight, .animate-fade-rise. Text: "Focus in a" / "Distracted World"
Paragraph: font-inter, white/70, 8px, leading-relaxed, max-w 220px, mt-3, .animate-fade-rise-delay. Text: "Designing tools for deep thinkers, bold creators, and quiet rebels. Digital spaces for sharp focus and inspired work."
Brand: font-instrument, white/25, 10px, tracking-tight, mt-4, .animate-fade-rise-delay-2. Text: "Velorah" with a <sup> registered trademark symbol at 5px
CARD 5 (bottom-center) -- "Beyond silence, we build the eternal."

White background, 384x384, 12px rounded corners
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_083109_283f3553-e28f-428b-a723-d639c617eb2b.mp4
Positioned in a container: absolute left-0 right-0 bottom-0, top: 140px. Video fills that container, object-cover. NOT looping -- uses a custom fade effect via React useRef/useEffect:
FADE_DURATION = 0.5 seconds
A requestAnimationFrame loop checks currentTime: if within first 0.5s, fade opacity up from 0; if within last 0.5s, fade opacity down to 0; else opacity 1
On ended event: set opacity to 0, wait 100ms, reset to beginning and play again
Video starts with style={{ opacity: 0 }}, autoPlay muted playsInline (no loop)
Complex white overlay (z-[1], absolute inset-0, pointer-events-none): linear-gradient(to bottom, white 0%, white 20%, transparent 45%, transparent 65%, white 90%, white 100%)
Content (z-10, flex column, items-center, justify-center, h-full, text-center, px-5):
Heading: font-instrument, 34px, leading 0.95, tracking-tight (-0.8px letterSpacing), black, .animate-fade-rise. Text: "Beyond " then <em> "silence," in #6F6F6F / "we build " then <em> "the eternal." in #6F6F6F
Paragraph: font-inter, #6F6F6F, 8px, leading-relaxed, max-w 230px, mt-3, .animate-fade-rise-delay. Text: "Platforms for deep thinkers and fearless makers. Digital havens for focused work and pure creative flow."
Brand: font-instrument, 10px, black/30, mt-5, tracking-tight, .animate-fade-rise-delay-2. Text: "Aethera" with <sup> registered trademark at 5px
CARD 6 (bottom-right) -- "Innovating the spirit of bloom"

Black background, 384x384, 12px rounded corners
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260315_073750_51473149-4350-4920-ae24-c8214286f323.mp4
Absolute inset-0, full width/height, object-cover, z-0, autoPlay loop muted playsInline
Gradient overlay z-[1]: bg-gradient-to-t from-black/80 via-black/20 to-black/40
Content (z-10, flex column, h-full, justify-between, p-6):
Top: Row with Lucide Sparkles (w-3.5 h-3.5, white/50) + text "AI-Powered Floral Design" in font-poppins, white/50, 9px, font-medium, uppercase, tracking [0.2em]
Bottom section:
Heading: font-poppins font-medium, white, 38px, leading 0.95, tracking -0.05em, .animate-fade-rise. Text: "Innovating the" / then <em> "spirit of" in font-source-serif italic font-medium white/80, followed by " bloom"
Paragraph: font-poppins, white/50, 8px, leading-relaxed, max-w 200px, mt-3, .animate-fade-rise-delay. Text: "Where artificial intelligence meets nature's artistry. Sculpting living compositions beyond imagination."
Two pills (mt-4, gap-2, .animate-fade-rise-delay-2): each uses .liquid-glass class, rounded-full, px-3 py-1, 7px, white/80, font-poppins. Labels: "AI Generation" and "3D Structures"
LAYOUT (App.tsx):
The outer div has the background image and centering. The inner div uses display: grid, gridTemplateColumns: repeat(3, 384px), gap: 16px, shrink-0. Cards are placed in order: CardOne, CardTwo, CardThree, CardFour, CardFive, CardSix.

## Velorah Focus — Social Media [sections/velorah-focus-hero]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/velorah-focus-hero.gif

Social meida post #1 (on the left)
Create a React application that displays a stylized, 3:4 aspect ratio social media post/landing page. The app must exactly match the following specifications, layout, animations, and CSS effects:
1. Layout & Structure:
Main Container: Full screen, black background (min-h-screen bg-black), centering its child content.
Social Frame: A centered container measuring exactly 600px wide by 800px high (w-[600px] h-[800px]). It should have a subtle white border (border-white/10), rounded corners (rounded-2xl), hidden overflow, and a heavy drop shadow.
Video Background: An absolute-positioned, full-cover HTML <video> element playing continuously in the background (autoplay, loop, muted, playsInline).
Exact Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260505_110052_2e127257-5236-40b1-ba48-4690260f1185.mp4
2. Visual Effects & Overlays (Critical Custom CSS):
Add the following visual layers exactly as described using custom CSS in index.css:
VHS Noise: An absolute full-cover div using an inline SVG fractal noise filter as its background image, with mix-blend-mode: overlay, opacity: 0.15, and a 0.2s step-keyframe animation to simulate static.
VHS Scanlines: A repeating linear gradient over the entire frame (50% transparent, 50% slight black transparent) with a background size of 100% 4px.
VHS Glitch Bar: A 40px tall horizontal bar that moves from top to bottom continuously over 4 seconds (top: -10% to top: 110%). It should have a backdrop blur of 2px, a 5deg hue rotation, and subtle white top/bottom borders.
RGB Text Glitch: A custom CSS animation (rgb-text-glitch) that applies a flickering text-shadow consisting of offset red (rgba(255, 0, 0, 0.5)) and cyan (rgba(0, 255, 255, 0.5)).
3. Typography:
Import two Google Fonts: Instrument Serif (for headings and logo) and Inter (for body text and buttons). Map these to Tailwind's font-serif and font-sans.
4. Content Elements (Centered over the video, shifted slightly up by -mt-[180px]):
Headline: "Focus in a<br/>Distracted World". Uses Instrument Serif, text size 64px, tight leading, tight tracking (tracking-[-2.46px]), animated to fade and rise upwards, and applying the rgb-text-glitch effect.
Subtext: "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work." Uses Inter, text size 17px, max-width 480px, delayed fade-rise animation, and applies the rgb-text-glitch effect.
Button: Text reads "Begin Journey". Uses Inter, 15px. Add a custom .liquid-glass CSS class. The liquid glass effect uses background: rgba(255, 255, 255, 0.01), a luminosity mix-blend-mode, 4px backdrop blur, and an advanced mask-composite border gradient trick to simulate a sleek glass edge. Add a slight hover scale effect.
Footer Logo: Positioned absolutely at the bottom center. Reads "Velorah" in Instrument Serif, size 3xl, with a small superscript trademark symbol ®. Applies the rgb-text-glitch effect.
5. Animations:
Implement a @keyframes fade-rise going from opacity: 0, translateY(24px) to opacity: 1, translateY(0).
Apply staggered animation classes to the headline (0s delay), paragraph (0.2s delay), and button (0.4s delay) so they slide in smoothly upon load.
Please write the complete React component (src/App.tsx) and the accompanying CSS stylesheet (src/index.css) utilizing standard React and Tailwind classes alongside the specific custom CSS for the VHS, glitch, and glassmorphism effects.

Social media post #2 (on the right)

Please build a React application with Tailwind CSS that recreates a cinematic 600x800px social media post component with VHS and RGB glitch effects. 

Please use the exact code below for the two main files to recreate my layout perfectly.

File 1: src/index.css
```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Inter:wght@400;500;600&display=swap');
@import "tailwindcss";

@theme {
  --font-serif: "Instrument Serif", ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
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

@keyframes fade-rise {
  from { opacity: 0; transform: translateY(24px); }
  to { opacity: 1; transform: translateY(0); }
}

.animate-fade-rise { animation: fade-rise 0.8s ease-out both; }
.animate-fade-rise-delay { animation: fade-rise 0.8s ease-out 0.2s both; }
.animate-fade-rise-delay-2 { animation: fade-rise 0.8s ease-out 0.4s both; }

/* VHS Effects */
.vhs-noise {
  position: absolute;
  inset: -100%;
  width: 300%;
  height: 300%;
  pointer-events: none;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
  opacity: 0.15;
  mix-blend-mode: overlay;
  animation: vhs-noise-anim 0.2s steps(2) infinite;
  z-index: 50;
}

@keyframes vhs-noise-anim {
  0% { transform: translate(0, 0); }
  20% { transform: translate(-5%, 5%); }
  40% { transform: translate(-10%, -5%); }
  60% { transform: translate(5%, 10%); }
  80% { transform: translate(10%, -10%); }
  100% { transform: translate(0, 5%); }
}

.vhs-scanlines {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(
    to bottom,
    rgba(255,255,255,0),
    rgba(255,255,255,0) 50%,
    rgba(0,0,0,0.15) 50%,
    rgba(0,0,0,0.15)
  );
  background-size: 100% 4px;
  z-index: 51;
}

.vhs-glitch-bar {
  position: absolute;
  left: 0;
  width: 100%;
  height: 40px;
  background: rgba(255, 255, 255, 0.03);
  backdrop-filter: blur(2px) hue-rotate(5deg);
  -webkit-backdrop-filter: blur(2px) hue-rotate(5deg);
  z-index: 52;
  pointer-events: none;
  animation: glitch-bar-anim 4s linear infinite;
  box-shadow: 0 0 10px rgba(255, 255, 255, 0.1);
  border-top: 1px solid rgba(255,255,255,0.05);
  border-bottom: 1px solid rgba(255,255,255,0.05);
}

@keyframes glitch-bar-anim {
  0% { top: -10%; opacity: 0; }
  10% { opacity: 1; }
  90% { top: 110%; opacity: 1; }
  100% { top: 110%; opacity: 0; }
}

.rgb-text-glitch {
  text-shadow: 
    1px 0 0 rgba(255, 0, 0, 0.5),
    -1px 0 0 rgba(0, 255, 255, 0.5);
  animation: rgb-flicker 3s infinite;
}

@keyframes rgb-flicker {
  0%, 95% { text-shadow: 1px 0 0 rgba(255, 0, 0, 0.5), -1px 0 0 rgba(0, 255, 255, 0.5); }
  96% { text-shadow: 3px 0 0 rgba(255, 0, 0, 0.8), -3px 0 0 rgba(0, 255, 255, 0.8); }
  97% { text-shadow: -2px 0 0 rgba(255, 0, 0, 0.8), 2px 0 0 rgba(0, 255, 255, 0.8); }
  100% { text-shadow: 1px 0 0 rgba(255, 0, 0, 0.5), -1px 0 0 rgba(0, 255, 255, 0.5); }
}
File 2: src/App.tsx
code
Tsx
export default function App() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center p-4">
      {/* 3:4 Aspect Ratio Frame (600x800) */}
      <div className="w-[600px] h-[800px] shrink-0 border border-white/10 rounded-2xl overflow-hidden relative shadow-2xl flex flex-col bg-black">
        {/* Background Video */}
        <video
          autoPlay
          loop
          muted
          playsInline
          className="absolute inset-0 w-full h-full object-cover z-0 opacity-100"
          src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260505_105838_084968f2-4415-42a4-971a-3bec54539549.mp4"
        />
        
        {/* VHS Overlay Elements (On top of everything) */}
        <div className="vhs-scanlines"></div>
        <div className="vhs-noise"></div>
        <div className="vhs-glitch-bar"></div>
        
        {/* Content Area - Middle of the frame */}
        <div className="relative z-10 flex-1 flex flex-col items-center justify-center text-center px-6 -mt-[358px]">
          
          <h1 className="font-serif text-[64px] leading-[0.95] tracking-[-2.46px] max-w-xl text-white animate-fade-rise rgb-text-glitch">
            Focus in a<br/>Distracted World
          </h1>
          
          <p className="font-sans text-[17px] text-white/95 mt-8 leading-relaxed max-w-[480px] animate-fade-rise-delay rgb-text-glitch">
            We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work.
          </p>

          <button className="liquid-glass rounded-full px-14 py-4 text-white text-[15px] font-sans mt-12 hover:scale-[1.03] transition-transform animate-fade-rise-delay-2 tracking-wide">
            Begin Journey
          </button>
        </div>

        {/* Footer Navbar */}
        <div className="absolute bottom-8 left-1/2 -translate-x-1/2 z-10">
          <div className="text-3xl tracking-tight text-white font-serif rgb-text-glitch">
            Velorah<sup className="text-[10px] ml-0.5 relative -top-3">®</sup>
          </div>
        </div>

      </div>
    </div>
  );
}
code
Code
***

### Option 2: The Highly Detailed Descriptive Prompt
*Use this if you want an AI to construct it from scratch based entirely on instructions rather than writing the pre-built code block.*

```text
Please build a React + Tailwind CSS web application that recreates a specific 3:4 aspect ratio social media post perfectly. Follow these exact formatting rules:

1. **Global Configuration (CSS)**:
   - Import 'Instrument Serif' and 'Inter' from Google Fonts. Map Instrument Serif to `--font-serif` and Inter to `--font-sans` in the CSS theme block.

2. **Core Layout (`App.tsx`)**:
   - The outer container should strictly be `min-h-screen bg-black flex items-center justify-center p-4`.
   - The central post frame needs to be exactly `w-[600px] h-[800px]` with `border border-white/10 rounded-2xl overflow-hidden relative shadow-2xl bg-black`.

3. **Background Media**:
   - Add an absolutely positioned, full-cover `<video>` tag behind all content (z-0 index, opacity-100).
   - Use `autoPlay loop muted playsInline`. 
   - The source URL must be exactly: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260505_105838_084968f2-4415-42a4-971a-3bec54539549.mp4`

4. **VHS & RGB Glitch Layers (CSS required)**:
   - Create a `.vhs-scanlines` class using a transparent-to-black linear gradient (background-size: 100% 4px).
   - Create a `.vhs-noise` class using a base64 SVG `<feTurbulence>` fractal noise filter layered with blend-mode overlay.
   - Create a `.vhs-glitch-bar` horizontally scanning top-to-bottom across the screen over 4 seconds continuously.
   - Create an `.rgb-text-glitch` class using animated text-shadows that split cyan (`rgba(0, 255, 255, 0.5)`) and red (`rgba(255, 0, 0, 0.5)`) rhythmically. Apply this text glitch class to ALL text elements.

5. **Main Content Overlay**:
   - Shift the main central content heavily up the page using Tailwind's exact specific margin: `-mt-[358px]`.
   - Heading (H1): "Focus in a<br/>Distracted World". Styling: 64px font-serif, leading-[0.95], tight tracking (-2.46px), white text fading up.
   - Subtitle (P): "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work." Styling: 17px font-sans, leading-relaxed, fading up with a 0.2s delay.
   - Button: "Begin Journey". Include a custom `.liquid-glass` CSS class giving it a luminosity background blend, 4px backdrop blur, and a sub-pixel linear-gradient border using `-webkit-mask`. Add hover scaling (`hover:scale-[1.03]`).

6. **Footer / Branding**:
   - Add a bottom footer absolutely positioned to `bottom-8 left-1/2 -translate-x-1/2`.
   - The text should say "Velorah" with a registered trademark symbol `®` raised slightly (`sup` tag with `-top-3 text-[10px]`). Apply the text glitch and serif styling.

## Kova Testimonial — Testimonial [sections/kova-testimonial]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/kova-testimonial.webp

---

Build a Testimonial section for a fintech app called "Kova" using React + Tailwind CSS 3 + Framer Motion. Use TypeScript. Do NOT use purple/indigo colors anywhere.

---

### FONTS (prerequisite)

These two web fonts must be loaded in `index.html`:

```html
<link href="https://db.onlinewebfonts.com/c/53077f9a3eee9c479d37d6af20394ded?family=Cooper+BT+W01+Light" rel="stylesheet">
<link href="https://db.onlinewebfonts.com/c/5ade3423145f3b9f7031574333ca0b73?family=Cooper+BT+W01+Medium" rel="stylesheet">
```

CSS utility classes:

```css
.font-cooper {
  font-family: 'Cooper BT W01 Light', 'Georgia', serif;
}
.font-cooper-medium {
  font-family: 'Cooper BT W01 Medium', 'Cooper BT W01 Light', 'Georgia', serif;
  font-weight: 500;
}
```

---

### FADEUP ANIMATION COMPONENT (prerequisite)

This section uses a `<FadeUp>` component with scroll-triggered animation (NOT immediate). Each element animates independently with staggered delays.

**Props:** `children`, `delay` (number, default 0), `className` (string, optional), `immediate` (boolean, default false)

**Variants:**
- `hidden`: `{ opacity: 0, y: 24, filter: 'blur(8px)' }`
- `visible`: `{ opacity: 1, y: 0, filter: 'blur(0px)', transition: { duration: 0.7, delay, ease: [0.25, 0.1, 0.25, 1] } }`

**Behavior for this section:** `initial="hidden" whileInView="visible" viewport={{ once: true, margin: '-60px' }}` (scroll-triggered, fires once when element enters viewport minus 60px margin).

---

### COLOR PALETTE

- Primary dark green: `#08150C`
- Hover dark green: `#1a2e1f`
- Warm cream background: `#FDF5EB`
- Text: Tailwind `stone-500`, `stone-700`, `stone-800`

---

### SECTION STRUCTURE

`<section>` element with class: `bg-[#FDF5EB] py-14 sm:py-20 px-5 sm:px-10 lg:px-20`

Inner container: `max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-[3fr_2fr] gap-10 md:gap-16 items-center`

This creates a two-column layout on desktop (left column 60%, right column 40%) that stacks to single column on mobile.

---

### LEFT COLUMN — Text Content

Five elements, each wrapped in its own `<FadeUp>` with staggered delays:

### 1. Section Heading (FadeUp delay=0)

`<h2>` with class: `font-cooper-medium text-2xl sm:text-3xl text-[#08150C] leading-snug mb-6 sm:mb-8`

Text: **"Trusted by ambitious, fast-moving teams"**

---

### 2. Company Badge (FadeUp delay=0.1)

Container: `flex items-center gap-2 mb-5 sm:mb-6`

- Square icon: `<div>` with class `w-7 h-7 rounded-md bg-[#08150C] flex items-center justify-center text-white text-xs font-bold` — contains the letter **"A"**
- Company name: `<span>` with class `text-sm font-semibold text-stone-800` — text **"Arcvex"**

---

### 3. Testimonial Quote (FadeUp delay=0.2)

`<blockquote>` with class: `font-cooper text-stone-700 text-lg sm:text-xl md:text-2xl leading-relaxed mb-5 sm:mb-6`

Text (including opening and closing quotation marks): **"With Kova, I have full visibility into our team's spending in real time. It feels like having a sharp financial advisor available at every hour, helping us stay on budget and make wiser calls."**

---

### 4. Attribution (FadeUp delay=0.3)

Container: `<div>` with class `mb-6 sm:mb-8`

- Name: `<p>` with class `text-sm font-semibold text-[#08150C]` — text **"Maya Reeves"**
- Title: `<p>` with class `text-xs text-stone-500` — text **"Director, Arcvex"**

---

### 5. CTA Button (FadeUp delay=0.4)

`<button>` with class: `flex items-center gap-2 bg-[#08150C] text-white text-sm font-medium px-5 py-2.5 rounded-xl hover:bg-[#1a2e1f] transition-colors`

Contents:
- Text: **"All Stories"**
- Custom arrow SVG icon (inline, NOT a Lucide icon):

```html
<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
  <path d="M2 7h10M8 3l4 4-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
</svg>
```

---

### RIGHT COLUMN — Video

Wrapped in `<FadeUp delay={0.15} className="flex justify-center md:justify-end">`.

Inner container: `<div>` with class `w-full max-w-xs sm:max-w-sm`

Contains a `<video>` element:
- **src:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260517_074029_c7a854bd-2d6e-4b62-96b3-ae8c16311e44.mp4`
- Attributes: `autoPlay`, `loop`, `muted`, `playsInline`
- Class: `w-full rounded-2xl object-cover aspect-square`

The video plays automatically, loops infinitely, has no audio, and is cropped to a square aspect ratio with rounded corners (16px radius).

---

### RESPONSIVE BEHAVIOR

- **Mobile (default):** Single column stack. Left text content appears first, video appears below. Heading is `text-2xl`. Quote is `text-lg`. Spacing uses smaller values (`py-14`, `mb-5`, `gap-10`).
- **sm (640px+):** Heading scales to `text-3xl`. Quote scales to `text-xl`. Spacing increases (`py-20`, `mb-6`, `mb-8`). Video container expands to `max-w-sm`.
- **md (768px+):** Switches to two-column grid `grid-cols-[3fr_2fr]` with `gap-16`. Quote scales to `text-2xl`. Video aligns to the right (`md:justify-end`).
- **lg (1024px+):** Horizontal padding increases to `px-20`.

---

### KEY IMPLEMENTATION NOTES

- The section background is the same warm cream `#FDF5EB` as the Features section below it, creating a seamless visual flow.
- The grid uses fractional columns `[3fr_2fr]` (not equal halves) to give the text content more horizontal space than the video.
- The video is `aspect-square` — it crops the video to a perfect square regardless of the source video's native aspect ratio.
- The FadeUp animations are scroll-triggered (NOT immediate like the Hero section), so they fire as the user scrolls this section into view.
- The left column elements animate in sequence with 0.1s delay increments (0, 0.1, 0.2, 0.3, 0.4).
- The right column video animates at delay=0.15, which means it starts between the heading (0) and quote (0.2) animations on the left — creating a natural cross-column stagger.
- The button uses `rounded-xl` (12px radius), NOT `rounded-full`.
- The arrow icon is a hand-drawn SVG, not a Lucide icon. It has a horizontal line from x=2 to x=12, and a chevron from (8,3) to (12,7) to (8,11).

## Arceage Testimonial — Testimonials [sections/arceage-testimonial]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/arceage-testimonial.webp

Create a React + Tailwind CSS v4 + Motion ("motion/react") customer feedback / testimonial carousel section. Use Vite as the bundler. Fully mobile responsive.

### Fonts

Import from Google Fonts in your global CSS:
```
@import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Instrument+Serif:ital@0;1&display=swap');
```

Define two Tailwind v4 theme fonts:
- `--font-sans: "Barlow", ui-sans-serif, system-ui, sans-serif;` (primary UI font via `font-sans`)
- `--font-dm-serif: "Instrument Serif", serif;` (accent font -- not used in this section but defined globally)

The page wrapper uses `bg-black font-sans text-white`. This section overrides to `bg-white text-black`.

### Dependencies

- `react` v19 (uses `useState`)
- `motion` (npm package "motion", import `motion`, `AnimatePresence` from `motion/react`)
- `lucide-react` (import `ArrowLeft`, `ArrowRight`)
- `tailwindcss` v4 with `@tailwindcss/vite` plugin
- Vite v6+

### Also requires a Typewriter component

A reusable character-by-character reveal animation triggered on scroll:
- Props: `text: string`, `delay?: number` (default 0), `speed?: number` (default 0.015), `className?: string` (default "")
- Uses `useRef`, `useInView(ref, { once: true, margin: "-10px" })` from `motion/react`
- Renders a `motion.span` with `initial="hidden"` and `animate={inView ? "visible" : "hidden"}`
- Parent variants: hidden = `{ opacity: 1 }`, visible = `{ opacity: 1, transition: { staggerChildren: speed, delayChildren: delay } }`
- Splits text into individual characters, each wrapped in `motion.span` with variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1 }`

---

### Data: Feedback Array

A static array of 3 feedback objects, each with `quote`, `author`, `title`, `avatar`:

```js
const feedbacks = [
  {
    quote: "\u00abWorking with the Acreage Ag team gave us a competitive edge in bringing our crops to market. Their technical expertise, machinery, and customer service are outstanding. We consider them a key partner for all our harvesting needs\u00bb",
    author: "Maranda Walsh",
    title: "Operations Manager, GreenAcres Farms",
    avatar: "https://picsum.photos/seed/maranda/100/100"
  },
  {
    quote: "\u00abThe team's dedication and innovative approach transformed our farm operations. They delivered a high-quality harvest on time and within budget. We highly recommend their services.\u00bb",
    author: "John Doe",
    title: "Owner, Valley Wheat Producers",
    avatar: "https://picsum.photos/seed/john/100/100"
  },
  {
    quote: "\u00abExceptional service and outstanding yields. The operators were highly skilled and integrated seamlessly with our in-house farm hands. A truly remarkable partnership.\u00bb",
    author: "Sarah Smith",
    title: "Chief Agronomist, HarvestYield Co.",
    avatar: "https://picsum.photos/seed/sarah/100/100"
  }
];
```

Note: Quotes use guillemet characters (the `<<` and `>>` style quotation marks).

---

### State Management

Two pieces of React state:
- `currentIndex` (number, default 0) -- tracks which feedback is visible
- `direction` (number, default 1) -- tracks slide direction for animation (+1 = forward, -1 = backward)

Two handler functions:
- `nextSlide`: sets direction to `1`, increments `currentIndex` wrapping with modulo
- `prevSlide`: sets direction to `-1`, decrements `currentIndex` wrapping with modulo

---

### Slide Animation Variants

Custom directional variants for the `AnimatePresence` carousel:

```js
const variants = {
  enter: (direction) => ({
    x: direction > 0 ? 100 : -100,
    opacity: 0
  }),
  center: {
    zIndex: 1,
    x: 0,
    opacity: 1
  },
  exit: (direction) => ({
    zIndex: 0,
    x: direction < 0 ? 100 : -100,
    opacity: 0
  })
};
```

Transition for the slide: `{ x: { type: "spring", stiffness: 300, damping: 30 }, opacity: { duration: 0.2 } }`

---

### Section Container

`<section>` with:
- `id="feedback"`
- Classes: `w-full bg-white text-black py-8 md:py-24 px-6 md:px-12 lg:px-[120px] flex flex-col justify-center overflow-hidden`

### Staggered Reveal Wrapper

The entire section content is wrapped in a `motion.div`:
- `initial="hidden"`, `whileInView="visible"`, `viewport={{ once: true, margin: "-100px" }}`
- Variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1, transition: { staggerChildren: 0.05 } }`
- Classes: `w-full`

---

### Element 1: Section Title (h2)

- Wrapped in `motion.h2` with variants: hidden = `{ opacity: 0, y: 20 }`, visible = `{ opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } }`
- Classes: `text-sm md:text-base mb-6 font-medium tracking-wide`
- Content: `<Typewriter text="Customer Feedback" delay={0} speed={0.012} />`

### Element 2: Top Divider Line

- `motion.div` with variants: hidden = `{ scaleX: 0 }`, visible = `{ scaleX: 1, transition: { duration: 0.8, ease: "easeOut" } }`
- Classes: `w-full h-[1px] bg-[#D9D9D9] mb-12 md:mb-20 origin-left`
- Animates by scaling from left to right.

### Element 3: Quote Carousel Area

- Wrapper `motion.div`:
  - Variants: hidden = `{ opacity: 0, y: 20 }`, visible = `{ opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } }`
  - Classes: `relative overflow-hidden min-h-[300px] md:min-h-[250px] flex items-center`
- Inside: `<AnimatePresence initial={false} custom={direction} mode="wait">`
  - `motion.div` keyed by `currentIndex`, using the directional slide variants described above
  - Classes: `w-full`
  - Contains the quote `<p>`:
    - Classes: `text-2xl md:text-4xl lg:text-[44px] font-light leading-snug md:leading-tight text-right tracking-tight`
    - Content: `<Typewriter text={feedbacks[currentIndex].quote} delay={0.2} speed={0.012} />`

### Element 4: Bottom Divider Line

- Same animation as top divider: `motion.div` with scaleX variants
- Classes: `w-full h-[1px] bg-[#D9D9D9] mt-12 md:mt-20 mb-8 origin-left`

### Element 5: Author Info + Navigation Arrows

- Wrapper `motion.div`:
  - Variants: hidden = `{ opacity: 0, y: 20 }`, visible = `{ opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } }`
  - Classes: `flex flex-col sm:flex-row justify-between items-center gap-6`

**Left side: Author info (animated on slide change)**
- `<AnimatePresence mode="wait">` wrapping a `motion.div` keyed by `currentIndex`:
  - `initial={{ opacity: 0, y: 10 }}`, `animate={{ opacity: 1, y: 0 }}`, `exit={{ opacity: 0, y: -10 }}`
  - `transition={{ duration: 0.2 }}`
  - Classes: `flex items-center gap-4 w-full sm:w-auto`
- Avatar `<img>`:
  - `src={feedbacks[currentIndex].avatar}`
  - Classes: `w-14 h-14 rounded-full object-cover`
  - `referrerPolicy="no-referrer"`
- Author name `<h3>`: classes `font-medium text-lg`, content `<Typewriter text={author} delay={0.4} speed={0.012} />`
- Author title `<p>`: classes `text-gray-500 text-sm`, content `<Typewriter text={title} delay={0.5} speed={0.012} />`

**Right side: Navigation arrows**
- Wrapper: `flex gap-2 w-full sm:w-auto justify-end`
- Two circular buttons, each:
  - Classes: `w-14 h-14 bg-[#D9D9D9] hover:bg-[#c9c9c9] transition-colors flex items-center justify-center rounded-full`
  - Left button: `onClick={prevSlide}`, contains `<ArrowLeft className="w-6 h-6 text-black" strokeWidth={1.5} />`, `aria-label="Previous feedback"`
  - Right button: `onClick={nextSlide}`, contains `<ArrowRight className="w-6 h-6 text-black" strokeWidth={1.5} />`, `aria-label="Next feedback"`

---

### Mobile Responsiveness Summary

- Section padding: `py-8 px-6` on mobile, `md:py-24 md:px-12`, `lg:px-[120px]`
- Title: `text-sm` on mobile, `md:text-base`
- Divider margins: `mb-12` / `mt-12` on mobile, `md:mb-20` / `md:mt-20` on desktop
- Quote text: `text-2xl` on mobile, `md:text-4xl`, `lg:text-[44px]`
- Quote container min-height: `min-h-[300px]` on mobile, `md:min-h-[250px]` on desktop
- Author + arrows row: stacks vertically on mobile (`flex-col`), horizontal at `sm:` (`sm:flex-row`)
- Arrow buttons and author section go full width on mobile (`w-full`), auto-width at `sm:` (`sm:w-auto`)
- Arrows align right on mobile via `justify-end`

---

## Radial Diagram — Testimonials [sections/radial-diagram]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/radial-diagram.webp

Build an "Our Comprehensive Branding Approach" section as a React component using TypeScript, Tailwind CSS 3, and Framer Motion. The section sits on a dark background (`#0f0f0f`) with white text. Font is `'DM Sans', sans-serif` (loaded via Google Fonts: `https://fonts.googleapis.com/css?family=DM+Sans:500,400`). Here is the exact specification:

---

**Overall Section Container:**

- `<section>` with `overflow-x-hidden`, `bg-[#0f0f0f]`, `text-white`, inline style `fontFamily: "'DM Sans', sans-serif"`

- Inner wrapper: `mx-auto max-w-7xl px-6 py-24 sm:px-10 lg:px-16 lg:py-32`

- All animations use the easing curve `[0.22, 1, 0.36, 1]` and trigger once when scrolled into view using Framer Motion's `useInView` with `{ once: true, margin: "-60px" }`

---

**Header (top of section):**

- Flex row (`flex items-start gap-4`) with `mb-20`

- Two lines of text stacked vertically:

  - Line 1: "Our Comprehensive" in `text-[#6e6e6e]` (gray), `font-light`, size `clamp(2rem, 3.4vw, 2.6rem)`, `leading-[1.18]`, `tracking-[-0.01em]`. Animates from `opacity:0, y:20` to visible, duration `0.7s`.

  - Line 2: "Branding Approach" in `text-white`, same font styling. Same animation but with `delay: 0.1s`.

- A small square button to the right: `h-7 w-7`, `border border-white/20`, contains a plus icon (SVG: two perpendicular lines forming a +, `stroke="currentColor"`, `strokeWidth="1.3"`, viewBox `0 0 12 12`). Text color `text-white/70`, hover: `bg-white/10 text-white`. Animates scale from 0.8 to 1, opacity 0 to 1, delay `0.25s`.

---

**Content Row (below header):**

- `flex flex-col gap-12 lg:flex-row lg:items-start lg:gap-10`

- Left side: `flex min-w-0 flex-1 flex-col gap-8 sm:flex-row sm:items-start sm:gap-10` containing the portrait and testimonial

- Right side: the circle diagram

---

**Left: Glitch Portrait**

- Container: `relative shrink-0`, fixed size `width: 250px`, `height: 310px`

- Contains an `<img>` filling the container with `object-cover`, using this Pexels image: `https://images.pexels.com/photos/3778212/pexels-photo-3778212.jpeg?auto=compress&cs=tinysrgb&w=600`

- 10 small white glitch blocks (`bg-white`) absolutely positioned around the edges of the portrait (some overflow outside). Each block has a fixed pixel `width` and `height` and percentage-based `left`/`top`. They animate in with `scale: 0 -> 1`, `opacity: [0, 1, 0.9]`, staggered by `0.05s` starting at `delay: 0.5s`, duration `0.35s`.

- The exact glitch block positions (x%, y%, width px, height px):

  ```

  (2, -3, 22, 22), (12, -5, 14, 10), (28, -2, 10, 10),

  (82, 22, 8, 8), (-4, 75, 16, 12), (8, 82, 10, 10),

  (-2, 88, 18, 16), (56, 82, 12, 14), (70, 90, 10, 10),

  (42, 94, 8, 6)

  ```

- The entire portrait group animates from `opacity:0, y:24` to visible, duration `0.8s`, delay `0.2s`.

---

**Left: Testimonial Text (next to portrait)**

- Container: `min-w-0 max-w-[420px]`

- Opening curly quote character `\u201C` in `text-[#555]`, font `Georgia, 'Times New Roman', serif`, `fontSize: "3.2rem"`, `lineHeight: 0.7`. Animates in with `y:14`, delay `0.3s`.

- Quote paragraph: "We kept seeing the same pattern -- brands with potential lost between messy processes, scattered visuals, and forgettable websites. This studio exists to align it all into one clear, consistent story." Use `&mdash;` for the em-dash. Styled as `text-white/90`, size `clamp(1.05rem, 1.5vw, 1.28rem)`, `font-normal`, `leading-[1.58]`. Animates from `y:20`, delay `0.4s`.

- Attribution block `mt-10`:

  - Name: "Alex West" in `text-[1.15rem] font-medium tracking-[0.01em] text-white`

  - Title: "Founder & Creative Director" (`&amp;` in JSX) in `mt-1 text-[0.85rem] tracking-wide text-[#6e6e6e]`

  - Both animate together from `y:14`, delay `0.55s`.

---

**Right: Circle Diagram**

- Wrapper: `flex w-full max-w-[360px] shrink-0 items-center justify-center self-center sm:max-w-[400px] lg:max-w-[440px]`

- Inner container has `aspect-ratio: 1/1`, position relative, animates opacity 0->1, delay `0.4s`, duration `0.8s`.

- SVG with `viewBox="0 0 100 100"`, absolutely filling the container:

  - A circle centered at `(50, 50)` with radius `30`, `stroke="white"`, `strokeWidth="0.18"`, `opacity="0.45"`

  - 3 lines radiating from center `(50,50)` outward to radius `36` (30+6) at these angles:

    - "websites" at 215 degrees

    - "brands" at 335 degrees

    - "ui/ux design" at 110 degrees

  - Lines default: `strokeWidth: 0.18`, `opacity: 0.45`. On hover of corresponding label: `strokeWidth: 0.6`, `opacity: 1`. Transition duration `0.3s`.

- 3 text labels positioned at radius `46` (30+16) from center at the same angles, using `transform: translate(-50%, -50%)` for centering. Styled with `fontSize: clamp(1.25rem, 2.8vw, 2.4rem)`, `letterSpacing: -0.01em`, `text-white`, `whitespace-nowrap`.

  - Default `fontWeight: 300`, on hover of that label: `fontWeight: 700` (transition `0.25s`).

  - Each label animates in with `opacity:0, y:16` to visible, staggered by `0.15s` starting at delay `0.6s`, duration `0.7s`.

- Hover state is shared: hovering a label highlights both the label (bold) and its corresponding SVG line.

---

**Dependencies:** React 18, Framer Motion (v12+), Tailwind CSS 3. Uses `useState`, `useRef`, `useInView` from framer-motion, and `motion` components for all animations. No external animation libraries beyond Framer Motion.
