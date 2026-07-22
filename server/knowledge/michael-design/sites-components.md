# Michael Design Library — sites-components

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 89 entries.

## Nexto 404 — 404 [sites/404]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/CleanShot_2026-05-15_at_15.46.24_2x_i163nd.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/404.png

Build a 404 "Page Not Found" hero page as a single full-viewport (100vh, no scroll) React + Vite + Tailwind CSS application using the DM Sans font and Google Material Symbols Rounded icons. The page must match the following specification exactly:

---

### Fonts & External Resources

- **Google Font:** DM Sans (all weights, variable: `opsz 9..40, wght 100..1000`)
- **Google Material Symbols Rounded:** `opsz,wght,FILL,GRAD@24,400,1,0`
- **Logo image:** `https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/Tests/logoipsum-415.svg` (rendered with `filter: brightness(0)` to make it black, height 28px)
- **Background spaceship image:** `https://pub-e68758f43067417dba612b2371819aa1.r2.dev/viktor-components/alien-spaceship.png`

---

### Layout

The entire page is exactly `100vh` with `overflow: hidden` on html, body, and `#root`. No scrolling. The body uses `display: flex; flex-direction: column`. The `#root` div also uses `height: 100vh; display: flex; flex-direction: column; overflow: hidden`.

---

### Background

Body has a layered background:
1. The spaceship PNG centered at `center 40%`, sized with `background-size: contain`
2. A `linear-gradient(to top left, #F5F5F5, #F7F7F7)` covering the full page

Both are `background-attachment: fixed` and `no-repeat`.

---

### Color Variables (CSS custom properties)

```
--text-main: #1a1a1a
--text-secondary: #888888
--bg-page: #F5F5F5
--card-bg: #ffffff
```

---

### Navbar

- Max-width `1100px`, centered, padding `28px 40px`
- Has a dashed bottom border made with `background-image: linear-gradient(to right, rgba(0,0,0,0.08) 2px, transparent 2px); background-size: 6px 1px` on a `::after` pseudo-element
- **Left:** Logo (the SVG image + the text "nexto." in 20px bold, -0.3px letter-spacing, color #111, flex with 9px gap)
- **Center:** Nav links ("Our Team", "Solutions" with a dropdown arrow character, "Showcase", "News") - 14px, weight 400, opacity 0.65, hover to opacity 1, gap 36px
- **Right:** CTA button "Let's Connect" - dark gradient button (`linear-gradient(180deg, #2c2c2c 0%, #111111 100%)`), white text 13px weight 500, border-radius 40px, padding `5px 16px 5px 5px`. Has a white circular arrow icon (24px circle) on the LEFT side with a chevron SVG inside. Box-shadow `0 4px 15px rgba(0,0,0,0.15)`. On hover: translateY(-1px), stronger shadow, brightness(1.1).
- **Hamburger (mobile only):** 3 spans, 24px wide, 2px height, animates to X when active. Hidden on desktop, shown on mobile (`display: flex` at max-width 768px).

---

### Mobile Navigation

- Fixed full-screen overlay, slides in from right with `transform: translateX(100%)` -> `translateX(0)`, cubic-bezier(0.77, 0, 0.175, 1) transition
- On mobile: left-aligned, large links (38px, weight 800, letter-spacing -1.5px), each with bottom border, padding 24px 0
- Last link is the CTA button styled same as navbar but with 32px arrow circle

---

### Main Content Area

- `flex: 1`, centered both ways (`align-items: center; justify-content: center`), max-width 700px, padding `20px 20px 30px`
- **Lost text:** "Seems you've wandered off..." - 15px, color `--text-secondary`, weight 400, margin-bottom 12px
- **Title wrapper:** `position: relative; display: inline-block; margin-bottom: 14px`
  - **Cloud decoration:** Material Symbols "cloud" icon, positioned `top: -18px; left: -24px`, font-size 42px, with gradient text fill (`linear-gradient(to bottom, #F7B2FB 50%, #786EF1 80%, #5588FB 100%)` using `-webkit-background-clip: text; -webkit-text-fill-color: transparent`), white drop-shadow outline, `floatSlow` animation (5s, 0.3s delay)
  - **Heart decoration:** Material Symbols "favorite" icon, positioned `bottom: -15px; right: 20px`, font-size 32px, same gradient fill, white drop-shadow outline, `floatSlow` animation (4.5s, 1s delay)
  - **Title:** "Whoops! Nothing here yet" - `font-size: clamp(34px, 5vw, 52px)`, weight 500, letter-spacing -1.5px, line-height 1.08, color #0f0f0f
- **Subtext:** "Grab a 30-minute `chat` to explore your ideas, scope, and vision. We'll find common ground, sync and `define` a clear roadmap." - 14px, color `--text-secondary`, line-height 1.7, max-width 470px, margin-bottom 28px. The words "chat" and "define" are in highlighted tags (inline-flex, background #E0E2E7, 12.5px, weight 600, padding 2px 12px, border-radius 6px)

---

### Navigation Cards

- Flex column, gap 12px, max-width 460px, positioned at bottom with `margin-top: auto`
- **Card 1 "Main Page":** House SVG icon (path: `M3 9.5L12 3l9 6.5V20a1 1 0 01-1 1H5a1 1 0 01-1-1V9.5z` with door `M9 21V12h6v9` in white). Subtitle: "Back where it all begins..."
- **Card 2 "Showcase":** Circle-dot SVG icon (circle r=9 filled, inner circle r=3.5 white). Subtitle: "Where we walk the walk"
- Each card: white background, border-radius 18px, padding 18px 22px, flex between, 1px border rgba(0,0,0,0.05), shadow `0 2px 12px rgba(0,0,0,0.04)`. On hover: translateY(-3px), shadow `0 8px 28px rgba(0,0,0,0.08)`.
- Icon container: 48px circle, background #eaecf0, scales 1.05 on card hover
- Right chevron arrow (rsaquo character, 21px), translateX(6px) on hover
- Card title: 15px weight 600, subtitle: 12px color `--text-secondary`

---

### Animations

```css
@keyframes floatSlow {
  0%, 100% { transform: translateY(0px) rotate(0deg); }
  50% { transform: translateY(-10px) rotate(3deg); }
}
```

---

### Responsive Breakpoints

**768px and below:**
- Hide nav-links and desktop CTA button, show hamburger
- Background-size: 90%, position: center 45%
- Navbar padding: 20px
- Title: 30px, decorations smaller
- Cards: full width, gap 10px, smaller padding/icons

**480px and below:**
- Title: 26px
- Background-size: 100%
- Decorations even smaller

---

## 404 Planet — 404 [sites/404-planet]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(7).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/404-planet.webp

Build a full-page 404 error page for a hosting company called "NEXOVA". The entire page is a single viewport-height layout with a looping background video, a navigation bar, a centered hero/404 section, and a multi-column footer. Use React + Tailwind CSS + Lucide React icons. No other UI libraries.

---

**FONT**

Load "Helvetica Now Var" via this stylesheet in `index.html`:
```
<link href="https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var" rel="stylesheet">
```
Apply it globally on the root container via inline style:
```
fontFamily: '"Helvetica Now Var", Helvetica, Arial, sans-serif'
```

---

**BACKGROUND VIDEO**

A `<video>` element with `autoPlay muted loop playsInline`, positioned `absolute inset-0 w-full h-full object-cover` behind all content. The video source URL is:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260613_180732_a54afbf6-b30d-470e-861f-669871f09f67.mp4
```
This is a cinematic dark-blue Earth-from-space shot.

---

**LAYOUT STRUCTURE**

The root is `relative min-h-screen flex flex-col`. Inside it:
1. The background `<video>` (absolute, behind everything)
2. A content wrapper `relative z-10 flex flex-col min-h-screen` containing nav, hero, and footer

---

**NAVIGATION BAR**

- Flex row, `items-center justify-between`, padding `px-6 md:px-12 lg:px-16 py-5`
- **Logo (left):** A custom SVG icon (4 quarter-circle leaf shapes forming a circle, white fill, `w-8 h-8`) next to the text "NEXOVA" in `text-white text-xl font-bold tracking-wider`. The exact SVG path is:
  ```
  M480 240a240 240 0 0 0-240 240 240 240 0 0 0 240-240Z
  M240 0A240 240 0 0 0 0 240 240 240 0 0 0 240 0Z
  M480 240A240 240 0 0 0 240 0a240 240 0 0 0 240 240Z
  M240 480A240 240 0 0 0 0 240a240 240 0 0 0 240 240Z
  ```
  viewBox `0 0 480 480`
- **Desktop nav links (center):** Hidden below `lg`. Links: Domain, Servers, Cloud, Managed, Email, Privacy. Styled `text-white/80 hover:text-white text-sm tracking-wide` with 200ms color transition, `gap-8`.
- **Login button (right):** Hidden below `lg`. Gradient button `bg-gradient-to-r from-emerald-400 to-cyan-500`, white text, `text-sm font-semibold px-6 py-2.5 rounded-full`. Text "LOG IN" with a Lucide `ArrowRight` icon (w-4 h-4) beside it.
- **Mobile hamburger:** Visible below `lg` breakpoint. A button with `z-[60]` showing Lucide `Menu` / `X` icons that cross-fade with rotation: the active icon is `opacity-100 rotate-0 scale-100`, the inactive is `opacity-0 rotate-90 scale-75` (or `-rotate-90`), all with `transition-all duration-300`.

---

**MOBILE MENU**

Uses two state variables: `mobileMenuOpen` (controls mount) and `menuVisible` (controls animation). When opening, `mobileMenuOpen` is set true, then `menuVisible` becomes true via `useEffect`. When closing, `menuVisible` is set false first, then after a 500ms timeout `mobileMenuOpen` is set false.

- **Backdrop:** Fixed overlay `inset-0 z-40 bg-black/40 backdrop-blur-md`, fades in/out with 400ms opacity transition. Clicking it closes the menu.
- **Menu panel:** Absolutely positioned `left-0 right-0 top-[68px] z-50`. Contains a backdrop-only blur layer (`backdrop-blur-xl`, no background color, `rounded-b-2xl`) and content on top (`relative z-10`).
- **Menu items:** Each nav link is centered, `text-lg sm:text-xl font-light tracking-[0.08em]`, `text-white/80 hover:text-white`. They stagger-animate in: each link has a `transitionDelay` of `350 + (index * 50)ms` when appearing (0ms when disappearing), transitioning opacity 0->1 and translateY 12px->0 over 400ms with `ease-out`.
- **Login button:** Same gradient style as desktop, appears last in the stagger sequence with delay `350 + (linkCount * 50)ms`.

---

**HERO / 404 SECTION**

Centered vertically and horizontally in the remaining space: `flex-1 flex flex-col items-center justify-center text-center px-4 sm:px-6 py-12 sm:py-16 md:py-0`.

1. **Subtitle lines (two h1 tags):**
   - "This page seems to have" and "slipped beyond our reach :/"
   - Both: `text-white/80 text-lg xs:text-2xl sm:text-3xl md:text-5xl font-light leading-snug tracking-tight`
   - First line: `mb-1 sm:mb-2`, second line: `mb-8 sm:mb-12`

2. **Giant "404" text:**
   - Wrapped in a `relative mb-8 sm:mb-12 w-full flex justify-center overflow-visible` div
   - The `<span>`: `text-[80px] xs:text-[100px] sm:text-[140px] md:text-[200px] lg:text-[260px] font-black text-white leading-none tracking-tighter select-none`
   - Has class `four-oh-four` which applies this CSS glow:
     ```css
     .four-oh-four {
       text-shadow: 0 0 80px rgba(255,255,255,0.3), 0 0 160px rgba(255,255,255,0.1);
     }
     ```

3. **"Return to Main Page" button:**
   - An `<a>` tag with class `liquid-glass` (glassmorphism effect) + `text-white text-[10px] xs:text-xs sm:text-sm tracking-[0.15em] sm:tracking-[0.2em] font-medium px-6 sm:px-8 py-3 sm:py-3.5 rounded-full uppercase`
   - The `liquid-glass` CSS class:
     ```css
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
     ```

---

**FOOTER**

Positioned at the bottom: `relative z-10 px-4 sm:px-6 md:px-12 lg:px-16 pb-8 sm:pb-10 pt-10 sm:pt-16`.

Grid: `grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-6 sm:gap-8 lg:gap-6`.

**4 link columns** (iterated from data):
- SERVERS: Web Servers, VPS Servers, Cloud Servers, Managed Instances, Bare Metal
- DOMAINS: Find Domain, Move Domains, DNS Manager, Domain Costs
- HELP US: Open a Ticket, FAQs, Docs, Tutorials, Forum
- ABOUT: Our Story, Leadership Team, Press Room, We Hire, Alliance, Blog

Each column title: `text-white text-[10px] sm:text-xs font-bold tracking-[0.15em] mb-3 sm:mb-4`. Links: `text-white/50 hover:text-white/80 text-[10px] sm:text-xs` with 200ms transition, in a `space-y-2 sm:space-y-2.5` list.

**Newsletter + Social column** (`col-span-2 lg:col-span-2`):
- Heading "JOIN FOR EXCLUSIVE DEALS" (same title style)
- Email input + "SEND IT" button side by side in a flex row, `max-w-sm`. Input: white bg, `rounded-l-md`, placeholder "Type your email to sign up". Button: same emerald-to-cyan gradient, `rounded-r-md`, `font-bold tracking-wider`.
- Heading "CONNECT" with `mt-5 sm:mt-6 mb-3`
- 6 social icons (Lucide: Facebook, Twitter, Dribbble, Youtube, Linkedin, Instagram), each `w-4 h-4`, `text-white/50 hover:text-white`, `gap-3`.

---

**RESPONSIVE BREAKPOINTS**
- `xs` is not a default Tailwind breakpoint -- if used, it needs to be added, or replaced with `sm`. The design uses mobile-first sizing that scales up at `sm` (640px), `md` (768px), and `lg` (1024px).
- Mobile: 2-col footer grid, hamburger menu, smaller text sizes
- Tablet (md): 4-col footer grid so newsletter sits beside the last link column
- Desktop (lg): 6-col footer grid, full horizontal nav, login button visible

## Axion About — About [sites/axion-about]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(29).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/axion-about.webp

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

## Botanical Shadow About — About [sites/botanical-shadow-about]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(11).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/botanical-shadow-about.webp

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

## LaunchEx About — About [sites/launchex-about]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(49).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/launchex-about.webp

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

## Orbis Hello — About [sites/orbis-hello]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(53).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbis-hello.webp

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

## Portfolio About — About [sites/portfolio-about]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(23).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/portfolio-about.webp

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

## Tech-Noir About — About [sites/tech-noir-about]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(48).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/tech-noir-about.webp

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

## Apex Program Accordion — Accordion [sites/apex-program-accordion]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(25).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/apex-program-accordion.webp

### Recreate the "Course Curriculum" Section — Exact Prompt

Build a React + TypeScript + TailwindCSS section called `CurriculumSection` that exactly matches the spec below.

### Stack & Global Setup

- React 18 + Vite + TypeScript
- TailwindCSS with HSL semantic tokens
- `framer-motion` for animations
- `clsx` + `tailwind-merge` exposed as `cn()` helper in `@/lib/utils`
- Dark theme background: `#000000` (or whatever the page bg is; section uses `bg-background`)
- Body font: **Inter** (loaded globally)
- Icon font: **Material Symbols Outlined** (loaded globally via Google Fonts link, with variable axes FILL, wght, GRAD, opsz)
- Tailwind token: `colors.landing.surface = "rgba(255,255,255,0.10)"`
- Foreground text token resolves to white/near-white via HSL

```html
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
<link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200" rel="stylesheet" />
```

### Helper Components (exact code)

### `cn` helper — `@/lib/utils.ts`
```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
export function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)); }
```

### `MIcon` — Material Symbols icon (`@/components/landing/icons/MIcon.tsx`)
```tsx
import { CSSProperties } from "react";
interface MIconProps { name: string; size?: number; className?: string; filled?: boolean; weight?: number; style?: CSSProperties; }
export const MIcon = ({ name, size = 16, className = "", filled = false, weight = 400, style }: MIconProps) => (
  <span aria-hidden="true"
    className={`material-symbols-outlined select-none leading-none inline-flex items-center justify-center ${className}`}
    style={{
      fontSize: size, width: size, height: size,
      fontVariationSettings: `'FILL' ${filled ? 1 : 0}, 'wght' ${weight}, 'GRAD' 0, 'opsz' ${Math.min(48, Math.max(20, size))}`,
      ...style,
    }}>{name}</span>
);
```

### `FadeUp` — scroll reveal (`@/components/landing/primitives/FadeUp.tsx`)
- Uses `framer-motion` `motion.div` with `initial={{opacity:0, y:24}}`, `whileInView={{opacity:1, y:0}}`, `viewport={{ once:true, amount:0.3 }}`, transition `{ duration:0.6, delay, ease:[0.22,1,0.36,1] }`
- Honors `prefers-reduced-motion` (skip the y translate)
- Props: `delay`, `duration=0.6`, `y=24`, `once=true`, `amount=0.3`, `className`, children

### `SpotlightBorder` — mouse-tracked 1px gradient border (`@/components/landing/effects/SpotlightBorder.tsx`)
- A wrapper with `padding: 1px` whose background is a radial gradient following the cursor.
- Uses CSS masks (`linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)` with `mask-composite: exclude` / `-webkit-mask-composite: xor`) so only the 1px border ring is visible.
- A global `window` `mousemove` listener writes `--spot-x` / `--spot-y` CSS variables on the element via `getBoundingClientRect()`.
- Background style:
```
radial-gradient(${size}px circle at var(--spot-x,-200px) var(--spot-y,-200px), rgba(255,255,255,${intensity}), rgba(255,255,255,0) 60%)
```
- Props: `radius` ('xl'|'2xl'|'3xl'|'full', default '3xl'), `size` (default 300), `intensity` (default 0.35), `as` ('div'|'button'|'section'), `className`, children.
- Renders the parent `Tag` with `relative` + `rounded-*`, and an absolutely-positioned `<span aria-hidden>` filling `inset-0` with the mask style above.

### Section: `CurriculumSection`

File: `src/components/landing/curriculum/CurriculumSection.tsx`

### Data (verbatim)
```ts
const modules = [
  { title: "Module 1", subtitle: "Foundations of AI Design",
    lessons: [
      "Intro to The Future of Design and Building, how it started and where it's going.",
      "AI Design Philosophy / Why and what makes good AI design",
      "What is Claude and 10+ other best AI tools for design",
      "Setting up Claude",
    ]},
  { title: "Module 2", subtitle: "Building with AI",
    lessons: [
      "Creating the Branding/Logo with AI",
      "Pitch Deck Build",
      "Landing Page wireframes",
      "Design a Landing Page with AI",
      "Building High-end web app",
      "GitHub & Vercel Deploy",
      "Creating social media design with AI",
    ]},
  { title: "Module 3", subtitle: "Launch & Growth",
    lessons: [
      "Getting Seen, Launch Videos",
      "Building a portfolio",
      "X (Twitter) Strategy for Designers",
      "Making money selling digital products as a designer",
    ]},
  { title: "Module 4", subtitle: "Making Money as an AI Designer",
    lessons: [
      "Finding Clients + Making Money",
      "How to Make Clients Find You",
      "Selling AI Powered templates",
    ]},
];
```

### Layout & Structure

- `<section>` root: `relative w-full bg-background py-12 sm:py-16`
- Inner container: `mx-auto max-w-[1080px] px-4 sm:px-6`

**Header (centered, mb-12):**
- Pill badge wrapped in `<FadeUp delay={0}>`:
  - `inline-flex items-center gap-2 rounded-full bg-landing-surface border border-white/10 px-3 py-1 text-xs text-foreground/80 backdrop-blur`
  - Inside: `<span class="h-1.5 w-1.5 rounded-full bg-foreground/70" />` then text `"Course Curriculum"`
  - `mb-6`
- Heading in `<FadeUp delay={0.1}>`:
  - `<h2 class="text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground">A fully modern curriculum.</h2>`

**Accordion container:**
- `<SpotlightBorder radius="2xl" size={520} intensity={0.5} className="mx-auto w-full p-2 sm:p-3">`
- Inside: a div `rounded-2xl border border-white/10 px-6 sm:px-8` with inline style `backgroundColor: "#161616"`
- Map `modules`, each wrapped in `<FadeUp delay={0.15 * i} className="border-b border-white/10 last:border-b-0">` containing the `ModuleAccordion`.

### `ModuleAccordion` component (inside same file)

State: `openIndex` (number|null) controlled by parent; initial value `0` (Module 1 open on mount). Click toggles: open if closed, close if same index.

**Header button** (`<button>` full width):
- Class: `flex w-full items-center justify-between gap-4 py-6 text-left`
- Left content (div):
  - Eyebrow: `<div class="text-[11px] uppercase tracking-[0.2em] text-foreground/50">{title}</div>`
  - Subtitle: `<div class="mt-2 text-lg sm:text-xl font-normal text-foreground">{subtitle}</div>`
- Right icon container:
  - `flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full border border-white/15 bg-white/[0.04] transition-transform duration-300`
  - When open: add `rotate-180`
  - Icon: `<MIcon name="expand_more" size={16} className="text-foreground/80" />`

**Collapse body:**
- Wrap in `<AnimatePresence initial={false}>`
- When open, render `motion.div` with:
  - `initial={{ height: 0, opacity: 0 }}`
  - `animate={{ height: "auto", opacity: 1 }}`
  - `exit={{ height: 0, opacity: 0 }}`
  - `transition={{ duration: 0.3, ease: "easeInOut" }}`
  - `className="overflow-hidden"`
- Inside: `<ul class="pb-6">` mapping each lesson:
  - `<li class="flex items-center gap-3 border-t border-white/10 py-4 text-sm text-foreground/85">`
  - Check bullet: `<span class="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full border border-white/20 bg-white/[0.06]"><MIcon name="check" size={12} className="text-foreground" /></span>`
  - Lesson text follows the bullet.

### Behavior Specs
- Module 1 starts open on first render.
- Only one module open at a time. Clicking the open module closes it (openIndex set to null).
- The chevron rotates 180° smoothly (300ms) when its module opens.
- Collapse animates height + opacity over 300ms easeInOut.
- The whole accordion card has a 1px border that picks up a soft white glow following the cursor anywhere on the page (SpotlightBorder, size=520, intensity=0.5).
- Pill, heading, and each module row fade up on scroll (intersection ≥30%, fires once). Modules stagger by 0.15s per index.

### Tailwind config additions
```ts
// tailwind.config.ts theme.extend.colors
landing: { surface: "rgba(255, 255, 255, 0.10)" }
```
Background tokens (`bg-background`, `text-foreground`) come from your HSL CSS variables in `index.css`.

### Full Component Source (drop-in)

```tsx
import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";
import { MIcon } from "@/components/landing/icons/MIcon";
import { SpotlightBorder } from "@/components/landing/effects/SpotlightBorder";
import { FadeUp } from "@/components/landing/primitives/FadeUp";

type Module = { title: string; subtitle: string; lessons: string[] };

const modules: Module[] = [ /* ...data above verbatim... */ ];

const ModuleAccordion = ({ module, isOpen, onToggle }:{
  module: Module; isOpen: boolean; onToggle: () => void;
}) => (
  <div>
    <button onClick={onToggle} className="flex w-full items-center justify-between gap-4 py-6 text-left">
      <div>
        <div className="text-[11px] uppercase tracking-[0.2em] text-foreground/50">{module.title}</div>
        <div className="mt-2 text-lg sm:text-xl font-normal text-foreground">{module.subtitle}</div>
      </div>
      <div className={cn("flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full border border-white/15 bg-white/[0.04] transition-transform duration-300", isOpen && "rotate-180")}>
        <MIcon name="expand_more" size={16} className="text-foreground/80" />
      </div>
    </button>
    <AnimatePresence initial={false}>
      {isOpen && (
        <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.3, ease: "easeInOut" }} className="overflow-hidden">
          <ul className="pb-6">
            {module.lessons.map((lesson, i) => (
              <li key={i} className="flex items-center gap-3 border-t border-white/10 py-4 text-sm text-foreground/85">
                <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full border border-white/20 bg-white/[0.06]">
                  <MIcon name="check" size={12} className="text-foreground" />
                </span>
                {lesson}
              </li>
            ))}
          </ul>
        </motion.div>
      )}
    </AnimatePresence>
  </div>
);

export const CurriculumSection = () => {
  const [openIndex, setOpenIndex] = useState<number | null>(0);
  return (
    <section className="relative w-full bg-background py-12 sm:py-16">
      <div className="mx-auto max-w-[1080px] px-4 sm:px-6">
        <div className="mb-12 flex flex-col items-center text-center">
          <FadeUp delay={0}>
            <span className="mb-6 inline-flex items-center gap-2 rounded-full bg-landing-surface border border-white/10 px-3 py-1 text-xs text-foreground/80 backdrop-blur">
              <span className="h-1.5 w-1.5 rounded-full bg-foreground/70" />
              Course Curriculum
            </span>
          </FadeUp>
          <FadeUp delay={0.1}>
            <h2 className="text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground">
              A fully modern curriculum.
            </h2>
          </FadeUp>
        </div>
        <SpotlightBorder radius="2xl" size={520} intensity={0.5} className="mx-auto w-full p-2 sm:p-3">
          <div className="rounded-2xl border border-white/10 px-6 sm:px-8" style={{ backgroundColor: "#161616" }}>
            {modules.map((m, i) => (
              <FadeUp key={m.title} delay={0.15 * i} className="border-b border-white/10 last:border-b-0">
                <ModuleAccordion module={m} isOpen={openIndex === i} onToggle={() => setOpenIndex(openIndex === i ? null : i)} />
              </FadeUp>
            ))}
          </div>
        </SpotlightBorder>
      </div>
    </section>
  );
};
```

### Acceptance Checklist
- [ ] Centered pill "● Course Curriculum" + heading "A fully modern curriculum." (Inter, normal weight, tight tracking)
- [ ] Card bg `#161616`, 1px white/10 border, rounded-2xl, mouse-tracked spotlight glow on border (size 520, intensity 0.5)
- [ ] 4 module rows separated by `border-white/10`, last row no border
- [ ] Each row: tiny uppercase eyebrow "Module N" (tracking 0.2em, foreground/50) + subtitle (text-lg sm:text-xl)
- [ ] Right-side 36px circular chevron button, rotates 180° on open (300ms)
- [ ] Module 1 open by default; only one open at a time; click open module to close
- [ ] Lessons list inside collapse: each lesson row separated by top border, 20px circular check bullet, text-sm foreground/85
- [ ] Collapse animates height + opacity 300ms easeInOut
- [ ] Pill, heading, and each module fade up on scroll (once, amount 0.3), modules stagger 0.15s
- [ ] Respects `prefers-reduced-motion`

## Guardnet Benefits — Benefits [sites/guardnet-benefits]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(34).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/guardnet-benefits.webp

Build a single React + TypeScript section using Tailwind CSS. No extra libraries. Fully mobile-responsive. Black background, white text.

### Global Prerequisites

- Font: `@import url(https://db.onlinewebfonts.com/c/e55e9079ee863276569c8a68d776ef04?family=Futura+Md+BT+Medium);`
- Body: `font-family: 'Futura Md BT Medium', system-ui, -apple-system, sans-serif; background-color: #000; color: #fff; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;`
- Section sits inside a `w-full max-w-[1400px]` wrapper on a black page.

---

### BenefitsSection

Container: `relative w-full bg-black px-4 sm:px-6 md:px-10 py-12 sm:py-20`

### Section Heading

`text-white text-3xl sm:text-4xl md:text-5xl font-light text-center mb-12 sm:mb-24` with inline style `letterSpacing: '-0.04em'`

Text: **"Key Benefits"**

### Three-Column Card Grid

`grid grid-cols-1 md:grid-cols-3 gap-3 sm:gap-4`

All three cards share: `relative h-[380px] sm:h-[460px] rounded-2xl bg-neutral-950 overflow-hidden`

---

### Card 1: Text Card (Left)

Additional classes: `p-6 sm:p-8`

**Blue Blob:** `absolute top-1/2 -translate-y-1/2 -left-[420px] h-[460px] w-[460px] rounded-full bg-[#1e3a8a] blur-3xl opacity-40`

**Content wrapper:** `relative z-10 flex flex-col h-full`

**Heading:** `text-white text-xl sm:text-2xl font-light leading-tight`
Text (two lines with `<br />`):
```
Preemptive Risks
Scouting and Reactions
```

**Body paragraph:** `mt-12 sm:mt-20 text-[13px] sm:text-[14px] leading-relaxed text-white/70 font-light max-w-[280px]`
Text: **"Defense platforms constantly observe bandwidth streams, record files, and machine behaviors to uncover unusual patterns or outliers that could signal a defensive failure."**

---

### Card 2: Video Card (Center)

Additional classes: `flex flex-col` (no padding on card itself)

**Top video region:** `relative w-full overflow-hidden` with inline style `height: '75%'`

- `<video>` element: `w-full h-full object-cover block`, attributes: `autoPlay loop muted playsInline`
- **Exact URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260421_072701_f6a01abb-eb30-4559-9d6e-774362defbc3.mp4`
- **Bottom fade overlay inside video wrapper:** `pointer-events-none absolute bottom-0 left-0 right-0 h-32 bg-gradient-to-b from-transparent to-neutral-950`

**Bottom text region:** `flex-1 flex items-center justify-start p-6 sm:p-8`

**Heading:** `text-white text-xl sm:text-2xl font-light leading-tight text-left`
Text (two lines with `<br />`):
```
Know-how and Sectoral
Awareness
```

---

### Card 3: Text Card (Right)

Additional classes: `p-6 sm:p-8`

**Blue Blob:** `absolute -top-28 -right-28 h-56 w-56 rounded-full bg-[#1e3a8a] blur-3xl opacity-40`

**Content wrapper:** `relative z-10 flex flex-col h-full`

**Heading:** `text-white text-xl sm:text-2xl font-light leading-tight`
Text (two lines with `<br />`):
```
Preemptive Risks
Scouting and Reactions
```

**Body paragraph:** `mt-auto text-[13px] sm:text-[14px] leading-relaxed text-white/70 font-light max-w-[320px]`
Text: **"Defense platforms constantly observe bandwidth streams, record files, and machine behaviors to uncover unusual patterns or outliers that could signal a defensive failure."**

Key difference from Card 1: the paragraph uses `mt-auto` to pin it to the **bottom** of the card, versus Card 1 which uses `mt-12 sm:mt-20` to place it in the **middle**.

---

### Color Palette Reference

| Token | Hex |
|---|---|
| Background | `#000000` (black) |
| Card surface | `neutral-950` (Tailwind) |
| Blob blue | `#1e3a8a` |
| Video fade target | `neutral-950` (matches card bg) |
| Body text | `white/70` |
| Heading text | `white` |

### Responsive Breakpoints

- Default (mobile): `< 640px` -- cards stack in a single column
- `sm:` at `640px` -- cards grow taller (460px), text/padding increases
- `md:` at `768px` -- switches to 3-column grid layout

### Interactions

- No hover states or JavaScript animations
- All motion comes from the looping background video in Card 2
- The bottom fade on the video blends seamlessly into the `neutral-950` card surface

## Kova Features — Benefits [sites/kova-features]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(28).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/kova-features.webp

---

**PROMPT:**

---

Build a "Features" section for a fintech landing page called "Kova" using React 18, TypeScript, Tailwind CSS 3, Framer Motion, and Lucide React. No purple/indigo colors.

---

### PREREQUISITE: FONTS

Load these two web fonts in `index.html` inside `<head>`:

```html
<link href="https://db.onlinewebfonts.com/c/53077f9a3eee9c479d37d6af20394ded?family=Cooper+BT+W01+Light" rel="stylesheet">
<link href="https://db.onlinewebfonts.com/c/5ade3423145f3b9f7031574333ca0b73?family=Cooper+BT+W01+Medium" rel="stylesheet">
```

Add these CSS utility classes in `index.css` (outside Tailwind layers, after the `@tailwind` directives):

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

### PREREQUISITE: FADEUP ANIMATION COMPONENT

Create a reusable `FadeUp` component (`src/FadeUp.tsx`) using Framer Motion's `motion.div`.

**TypeScript interface:**
```ts
interface FadeUpProps {
  children: ReactNode;
  delay?: number;       // default 0
  className?: string;
  immediate?: boolean;  // default false
}
```

**Variants object:**
```ts
const variants = {
  hidden: { opacity: 0, y: 24, filter: 'blur(8px)' },
  visible: { opacity: 1, y: 0, filter: 'blur(0px)' },
};
```

**Shared transition:** `{ duration: 0.7, delay, ease: [0.25, 0.1, 0.25, 1] }`

**Two modes:**
- `immediate={true}`: uses `animate="visible"` (plays on mount)
- `immediate={false}` (default, used in THIS section): uses `whileInView="visible"` with `viewport={{ once: true, margin: '-60px' }}`

Both modes use `initial="hidden"`.

---

### COLOR PALETTE

| Token | Hex | Usage |
|---|---|---|
| Primary dark green | `#08150C` | Heading text, dark card backgrounds, button bg |
| Hover dark green | `#1a2e1f` | Button hover state |
| Warm cream | `#FDF5EB` | Section background |
| Light beige | `#EBE4DC` | Card 3 outer background |
| Inner card bg | `#F4F1EC` | Card 3 inner content area + donut base circle |
| Burnt orange | `#C46B2D` | Donut segment 1 |
| Olive green | `#7A8C3E` | Donut segment 2 |
| Sage green | `#A8B87A` | Donut segment 3 |
| Warm gray | `#B8AFA4` | Donut segment 4 |

Text colors: `text-white`, `text-white/80`, `text-stone-500`, `text-stone-700`, `text-stone-800`

---

### SECTION WRAPPER

```html
<section class="bg-[#FDF5EB] py-14 sm:py-20 px-5 sm:px-10 lg:px-20">
  <div class="max-w-7xl mx-auto">
    <!-- header row -->
    <!-- cards grid -->
  </div>
</section>
```

---

### HEADER ROW

Container: `flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-8 sm:mb-10`

### Heading (FadeUp delay=0)

```html
<h2 class="font-cooper-medium text-2xl sm:text-3xl md:text-4xl text-[#08150C] leading-snug">
  Designed to sharpen every decision
</h2>
```

Font sizes: `text-2xl` (24px) on mobile, `sm:text-3xl` (30px) at 640px+, `md:text-4xl` (36px) at 768px+. Uses the Cooper BT Medium font via the `.font-cooper-medium` utility class.

### Button (FadeUp delay=0.1)

```html
<button class="self-start sm:self-auto flex-shrink-0 flex items-center gap-2 bg-[#08150C] text-white text-sm font-medium px-5 py-2.5 rounded-xl hover:bg-[#1a2e1f] transition-colors">
  Watch Demo
  <Play size={13} className="fill-white" />
</button>
```

- `text-sm` = 14px
- `font-medium` = font-weight 500
- `rounded-xl` = 12px border radius (NOT rounded-full)
- `self-start` on mobile (left-aligned), `sm:self-auto` on desktop (right-aligned by the `justify-between` parent)
- Lucide `Play` icon at 13px, filled white, placed AFTER the text

---

### CARDS GRID

Container: `grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4`

- Mobile: 1 column
- sm (640px+): 2 columns
- lg (1024px+): 4 columns
- Gap: `gap-4` = 16px

All four cards share `rounded-2xl overflow-hidden aspect-[3/4]` (16px border radius, 3:4 portrait aspect ratio).

---

### CARD 1 — Smart Budgeting (FadeUp delay=0.05)

```html
<div class="relative rounded-2xl overflow-hidden bg-[#08150C] aspect-[3/4] flex flex-col justify-between">
  <img
    src="https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061249_f20dfeda-1033-45ce-a3ee-070965599cbf.png&w=1280&q=85"
    alt="Smart Budgeting"
    class="absolute inset-0 w-full h-full object-cover"
  />
  <div class="absolute inset-0 bg-gradient-to-t from-[#08150C]/80 via-[#08150C]/20 to-transparent"></div>
  <div class="relative z-10 p-4">
    <div class="inline-flex items-center gap-1.5 text-white text-sm font-medium px-2.5 py-1 rounded-full">
      <Sparkles size={16} class="text-white" />
      Smart Budgeting
    </div>
  </div>
  <div class="relative z-10 p-4">
    <p class="text-white/80 text-sm sm:text-base leading-relaxed">
      Let AI reshape how you plan your spending. Kova adapts to your...
    </p>
  </div>
</div>
```

- Fallback bg `bg-[#08150C]` shows while image loads
- Background image is absolutely positioned, covering the full card
- Gradient overlay: 3-stop, bottom-to-top: 80% opacity dark green at bottom, 20% in middle, transparent at top
- Top-left label: icon + text in a pill shape, white text at `text-sm` (14px)
- Bottom-left description: `text-white/80` (white at 80% opacity), `text-sm` (14px) on mobile, `sm:text-base` (16px) at 640px+

---

### CARD 2 — Bank-Grade Security (FadeUp delay=0.15)

Identical structure to Card 1, with these differences:

- Fallback bg: `bg-stone-700` (instead of `bg-[#08150C]`)
- Image src: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061305_db631f5f-185f-4fda-a7a8-1dd7359ef2ea.png&w=1280&q=85`
- Image alt: `"Bank-Grade Security"`
- Icon: `<ShieldCheck size={16} className="text-white" />`
- Label text: **"Bank-Grade Security"**
- Description: **"Keep your money safe with end-to-end encryption, live fraud alerts, and two-factor auth..."**

---

### CARD 3 — Spend Insights (FadeUp delay=0.25)

This card is structurally DIFFERENT from the other three. It has NO background image, NO gradient overlay. It is a light-colored card with a donut chart visualization.

```html
<div class="relative rounded-2xl overflow-hidden aspect-[3/4] flex flex-col p-5" style="background-color: #EBE4DC">
```

**Top label:**
```html
<div class="inline-flex items-center gap-1.5 text-stone-700 text-sm font-medium px-2.5 py-1 rounded-full self-start mb-4">
  <PieChart size={16} />
  Spend Insights
</div>
```
- Text and icon are `text-stone-700` (NOT white like the other cards)
- `self-start` aligns the pill to the left
- `mb-4` = 16px bottom margin

**Inner content container:**
```html
<div class="flex flex-col items-center justify-center flex-1 gap-3 rounded-2xl p-4" style="background-color: #F4F1EC">
```
- Takes up remaining vertical space via `flex-1`
- Inner rounded container with lighter background `#F4F1EC`
- `gap-3` = 12px between children

**Title block** (inside inner container):
```html
<div class="text-center mb-1">
  <p class="text-sm sm:text-base font-semibold text-stone-800">Monthly Spend</p>
  <p class="text-xs sm:text-sm text-stone-500">1 Apr – 30 May 2026</p>
</div>
```
- "Monthly Spend": `text-sm` (14px) mobile, `sm:text-base` (16px), `font-semibold` (600), `text-stone-800`
- Date range: `text-xs` (12px) mobile, `sm:text-sm` (14px), `text-stone-500`

**Donut chart wrapper:**
```html
<div class="relative w-28 h-28 sm:w-36 sm:h-36">
```
- 112x112px on mobile, 144x144px at sm (640px+)

**SVG donut chart:**
```html
<svg viewBox="0 0 36 36" class="w-full h-full -rotate-90">
```
- `-rotate-90` rotates the entire SVG so arcs start from 12 o'clock instead of 3 o'clock
- All circles use `cx="18" cy="18" r="14" fill="none" strokeWidth="5"`
- Total circumference = 2 * pi * 14 = ~87.96

**Base circle (background ring):**
```html
<circle cx="18" cy="18" r="14" fill="none" stroke="#F4F1EC" strokeWidth="5" />
```

**Four colored segments (in drawing order, back to front):**

| # | Color | Hex | strokeDasharray | strokeDashoffset | Arc % |
|---|---|---|---|---|---|
| 1 | Burnt orange | `#C46B2D` | `26.4 61.56` | `0` | 30% |
| 2 | Olive green | `#7A8C3E` | `22 65.96` | `-26.4` | 25% |
| 3 | Sage green | `#A8B87A` | `17.6 70.36` | `-48.4` | 20% |
| 4 | Warm gray | `#B8AFA4` | `22 65.96` | `-66` | 25% |

Math explanation: each segment's `strokeDasharray` is `[arcLength] [circumference - arcLength]`. Each segment's `strokeDashoffset` is the negative sum of all previous arc lengths, shifting the start position clockwise.

**Center text overlay:**
```html
<div class="absolute inset-0 flex flex-col items-center justify-center">
  <span class="text-lg sm:text-xl font-bold text-[#08150C]">50%</span>
  <span class="text-xs sm:text-sm text-stone-500">of budget</span>
</div>
```
- "50%": `text-lg` (18px) mobile, `sm:text-xl` (20px), `font-bold` (700), dark green `#08150C`
- "of budget": `text-xs` (12px) mobile, `sm:text-sm` (14px), `text-stone-500`

---

### CARD 4 — Wealth Building (FadeUp delay=0.35)

Identical structure to Card 2 (and Card 1), with these differences:

- Fallback bg: `bg-stone-700`
- Image src: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061316_50e651f8-02d0-4add-9ddb-7d81d15ac02e.png&w=1280&q=85`
- Image alt: `"Wealth Building"`
- Icon: `<TrendingUp size={16} className="text-white" />`
- Label text: **"Wealth Building"**
- Description: **"Grow your net worth with tools that help you set targets, monitor gains, and act..."**

---

### LUCIDE REACT ICONS USED

Import from `lucide-react`:
- `Play` (header button, size 13, fill-white)
- `Sparkles` (Card 1, size 16)
- `ShieldCheck` (Card 2, size 16)
- `PieChart` (Card 3, size 16)
- `TrendingUp` (Card 4, size 16)

---

### COMPLETE ANIMATION DELAY MAP

| Element | FadeUp delay | Mode |
|---|---|---|
| Section heading | `0` | scroll-triggered |
| Watch Demo button | `0.1` | scroll-triggered |
| Card 1 (Smart Budgeting) | `0.05` | scroll-triggered |
| Card 2 (Bank-Grade Security) | `0.15` | scroll-triggered |
| Card 3 (Spend Insights) | `0.25` | scroll-triggered |
| Card 4 (Wealth Building) | `0.35` | scroll-triggered |

All use `whileInView="visible"` with `viewport={{ once: true, margin: '-60px' }}`. Animation plays once when element scrolls into view (with a -60px inset margin trigger point).

---

### RESPONSIVE BREAKPOINT SUMMARY

| Breakpoint | Grid | Heading | Body text | Donut size | Padding |
|---|---|---|---|---|---|
| Default (mobile) | `grid-cols-1` | `text-2xl` (24px) | `text-sm` (14px) | 112x112px | `py-14 px-5` |
| sm (640px+) | `sm:grid-cols-2` | `sm:text-3xl` (30px) | `sm:text-base` (16px) | 144x144px | `sm:py-20 sm:px-10` |
| md (768px+) | (still 2-col) | `md:text-4xl` (36px) | (same) | (same) | (same) |
| lg (1024px+) | `lg:grid-cols-4` | (same) | (same) | (same) | `lg:px-20` |

---

### CRITICAL IMPLEMENTATION DETAILS

1. The section background `#FDF5EB` must match any adjacent sections for a seamless look -- no visible divider.
2. Cards 1, 2, and 4 share identical markup structure (image + gradient overlay + top label + bottom text). Only the image src, icon, label text, and description differ.
3. Card 3 is the only light-themed card -- it uses inline `style={{ backgroundColor: '#EBE4DC' }}` for the outer div and `style={{ backgroundColor: '#F4F1EC' }}` for the inner container (these are NOT Tailwind classes because they are custom hex values).
4. The `aspect-[3/4]` class on every card enforces a consistent 3-wide-by-4-tall portrait ratio across the grid.
5. All buttons use `rounded-xl` (12px), NOT `rounded-full`.
6. The gradient overlay on image cards is `bg-gradient-to-t from-[#08150C]/80 via-[#08150C]/20 to-transparent` -- this is a Tailwind gradient going from bottom (80% opaque dark green) through middle (20%) to top (fully transparent), ensuring bottom text remains readable over any image.
7. The donut SVG has `-rotate-90` on the `<svg>` element itself to rotate the coordinate system so segments begin at the top (12 o'clock) rather than the right (3 o'clock).

## Bento Grid Stats — Bento [sites/bento-grid-stats]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(58).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bento-grid-stats.webp

Build a "Why Us?" bento grid section as a React + TypeScript component using Tailwind CSS 3 and Framer Motion. Font is `'DM Sans', sans-serif` (Google Fonts: `https://fonts.googleapis.com/css?family=DM+Sans:500,400`). Dark background `#0f0f0f`. Here is the exact specification:

---

**Section Container:**

- `<section>` with `bg-[#0f0f0f] px-6 py-24 sm:px-10 lg:px-16 lg:py-32`, inline style `fontFamily: "'DM Sans', sans-serif"`

- Inner wrapper: `mx-auto max-w-7xl`

- All animations use easing `[0.22, 1, 0.36, 1]`, triggered once on scroll via Framer Motion `useInView` with `{ once: true, margin: "-60px" }`

- Each card uses a shared `CardMotion` wrapper that animates from `opacity: 0, scale: 0.95` to `opacity: 1, scale: 1`, duration `0.65s`, with staggered delays

---

**Layout:**

- On mobile (`md:hidden`): single column, cards stacked vertically with `gap-4`

- On desktop (`md:grid md:grid-cols-6 md:gap-4`): explicit 6-column grid with `gridTemplateRows: "repeat(10, minmax(46px, auto))"`. Card placements:

  - Header card: `gridColumn: "1 / 3"`, `gridRow: "1 / 5"`

  - Speed card (5x): `gridColumn: "3 / 5"`, `gridRow: "1 / 6"`

  - Text card (dark): `gridColumn: "5 / 7"`, `gridRow: "1 / 5"`

  - Income card (32M+): `gridColumn: "1 / 3"`, `gridRow: "5 / 11"`

  - Photo card (100+): `gridColumn: "3 / 5"`, `gridRow: "6 / 11"`

  - Projects card (200+): `gridColumn: "5 / 7"`, `gridRow: "5 / 11"`

---

**Shared "Plus Button" component:**

- A `<span>` with `inline-flex h-7 w-7 shrink-0 items-center justify-center border text-xs font-light`, containing the text "+". Has a `dark` variant: when `dark=true` uses `border-black/20 text-black`, otherwise `border-white/30 text-white`.

---

**Card 1 -- Header Card (no background):**

- Delay `0`. Classes: `flex h-full flex-col justify-end pb-4 pr-4`

- Label: "why us?" in `mb-4 inline-block text-sm tracking-[0.15em] text-white/70`

- Heading: `text-[clamp(1.6rem,2.6vw,2.4rem)] font-light leading-[1.2] tracking-tight`

  - Line 1: "Seamless" in `text-white`

  - Line 2: "Brand, Identity," in `text-[#666]`

  - Line 3: "and Web" in `text-[#666]`

  - Each line separated by `<br />`

---

**Card 2 -- Income Card (32M+), white background:**

- Delay `0.08`. Classes: `flex h-full flex-col bg-white p-7`

- Top row: flex between the stat and a dark PlusBtn

  - Stat: "32M +" in `text-[clamp(1.8rem,3.5vw,2.8rem)] font-light leading-none tracking-tight text-black`

  - Subtitle: "Income produced for our customers." in `mt-2 text-[13px] leading-snug text-[#777]`

- Bottom (pushed down with `mt-auto pt-8`): A staircase dot chart

  - 26 columns, each a `flex flex-col-reverse gap-[2px]` of 15 cells

  - Each cell is `h-[5px] w-[5px] sm:h-[7px] sm:w-[7px]`

  - Filled cells (`bg-black`) are computed: for column `c`, base = `floor(c * 0.55)`, active rows = `[base]`, add `base+1` if `c` is odd, add `base-1` if `c > 4` (filter negatives)

  - Columns separated by `gap-[2px]`

  - Below the chart: year labels `["2016", "2018", "2022", "2024", "2026"]` in `mt-3 flex justify-between pr-2 text-[10px] tracking-wide text-[#aaa]`

---

**Card 3 -- Speed Card (5x), white background:**

- Delay `0.12`. Classes: `flex h-full flex-col bg-white p-7`

- Top area (`flex flex-1 items-start justify-center pt-2`): An SVG diagram `viewBox="0 0 200 180"`, `h-auto w-full max-w-[240px]`:

  - 3 concentric circles centered at `(110, 80)` with radii `75`, `50`, `25`, `stroke="#222"`, `strokeWidth="0.8"`, opacities `0.2`, `0.3`, `0.4`

  - 4 small black squares with icons at specific positions:

    - `rect(68, 42, 16, 16)` with "+" text (white, fontSize 11, fontWeight 300)

    - `rect(102, 36, 20, 20)` with lightning bolt emoji (&#9889;, white, fontSize 13)

    - `rect(82, 128, 14, 14)` with minus sign (&minus;, white, fontSize 11, fontWeight 300)

    - `rect(138, 128, 14, 14)` with inner white square `rect(142, 132, 6, 6)`

- Bottom (`mt-2`):

  - "5x" centered, `text-[clamp(2rem,3.5vw,3rem)] font-light tracking-tight text-black`

  - "Quicker than competing firms." centered, `mt-1 text-[13px] text-[#777]`

---

**Card 4 -- Text Card, dark background:**

- Delay `0.18`. Classes: `flex h-full flex-col bg-[#1a1a1a] p-7`

- Top right: a light PlusBtn (white variant), wrapped in `flex justify-end`

- Bottom (pushed down with `mt-auto space-y-5 pb-1`): Two paragraphs:

  - "We partner with ambitious brands to craft unified digital identities that merge strategy, design, and code into one seamless experience."

  - "We accelerate the journey from concept to launch, eliminating the friction of scattered teams and misaligned visions."

  - Both: `text-[13px] leading-[1.7] text-white/60`

---

**Card 5 -- Projects Card (200+), white background:**

- Delay `0.22`. Classes: `relative flex h-full flex-col bg-white p-7`

- Top: A `relative h-24 w-full` container with 7 scattered black squares (`bg-black`), each positioned absolutely with percentage left/top and pixel sizes:

  ```

  (55%, 2%, 30px), (80%, 0%, 24px), (70%, 28%, 16px),

  (92%, 18%, 14px), (58%, 22%, 10px), (88%, 36%, 10px),

  (46%, 14%, 8px)

  ```

- Stat: "200 +" in `mt-4 text-[clamp(1.8rem,3.5vw,2.8rem)] font-light leading-none tracking-tight text-black`

- Description: "Delivering projects globally, assisting our clients in reaching their objectives." in `mt-3 max-w-[210px] text-[13px] leading-[1.7] text-[#777]`

---

**Card 6 -- Photo Card (100+), dark overlay on image:**

- Delay `0.28`. Classes: `relative h-full overflow-hidden bg-black`

- Background image: `https://images.pexels.com/photos/3184291/pexels-photo-3184291.jpeg?auto=compress&cs=tinysrgb&w=800`, absolutely positioned, `object-cover opacity-55`

- Gradient overlay: `bg-gradient-to-t from-black/60 via-transparent to-black/30`

- Content container: `relative flex h-full min-h-[380px] flex-col justify-between p-6`

  - **Top left**: A logo-like mark -- bold "N" (`text-[1.6rem] font-bold leading-none text-white`) with 3 small white squares beside it: one `10x10`, and two stacked `7x7` (the bottom one at `opacity 50%`), arranged with `gap-[3px]`

  - **Top right**: "4.9 / 5" in `text-base font-light leading-none text-white`, with 5 white star SVGs below (`width="11" height="11"`, star path: `M6 0l1.8 3.6L12 4.2 8.9 7.1l.7 4.2L6 9.3 2.4 11.3l.7-4.2L0 4.2l4.2-.6z`)

  - **Middle left**: Two small white rectangles: `18x18 bg-white/70` and `18x26 bg-white/40`, with `gap-2`

  - **Bottom**: flex between stat and a white square

    - "100 +" in `text-[clamp(1.8rem,3.5vw,2.8rem)] font-light leading-none text-white`

    - "Joyful clients and growing." in `mt-2 text-[13px] text-white/60`

    - A plain `h-12 w-12 bg-white` square on the right

---

**Dependencies:** React 18, Framer Motion (v12+), Tailwind CSS 3. Uses `useRef`, `useInView`, and `motion` from framer-motion. No other libraries.

## Blog Showcase — Blog [sites/blog-showcase]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(22).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/blog-showcase.webp

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

## Cognitra Offer — Cards [sites/cognitra-offer]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(56).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cognitra-offer.webp

---

**Prompt:**

Create a full-viewport services section (100vh) with a solid gray background (`#C5C5C5`), vertically centered content, containing a counter label, a heading/subtext row, and a 3-column card grid with video thumbnails.

**Section layout:**
- `position: relative; z-index: 2; background-color: #C5C5C5`
- `display: flex; flex-direction: column; justify-content: center`
- `height: 100vh`
- Padding: `70px 32px 80px 32px`

**Content (top to bottom):**

**1. Counter label:**
- Text: `"003 / 005"`
- `font-size: 11px; letter-spacing: 0.08em; color: #666; margin-bottom: 20px`
- Fade-up animation with `delay: 0`

**2. Heading + subtext row:**
- A flex row (`display: flex; gap: 48px; align-items: flex-start; margin-bottom: 32px`)
- **Left column** (heading): `flex-shrink: 0; width: 32%`
  - `<h2>` with text: `"EXPLORE WHAT WE OFFER"`
  - Each word is an individual `<span>`, displayed via `display: flex; flex-wrap: wrap; gap: 0.25em`
  - Word-by-word staggered fade-up: first word at `delay: 0.1`, each adds `0.1s`, with `y: 28px`
  - Typography: `font-size: clamp(26px, 3vw, 42px); font-weight: 700; line-height: 1.05; letter-spacing: -0.01em; text-transform: uppercase; color: #1a1a1a; margin: 0; max-width: 320px`
- **Right column** (subtext): `flex: 1; padding-top: 8px`
  - `<p>` text: `"We provide all-in-one AI automation services in one place."`
  - `font-size: 14px; line-height: 1.65; color: #3a3a3a; max-width: 320px; margin: 0`
  - Fade-up with `delay: 0.25`

**3. Cards grid:**
- `display: grid; grid-template-columns: repeat(3, 1fr); grid-auto-rows: 1fr; gap: 20px; align-items: stretch`
- 3 cards, each with staggered fade-up: `delay: 0.4`, `0.55`, `0.70`

**Card data:**

| # | Video URL | Title | Description |
|---|-----------|-------|-------------|
| 1 | `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_220333_48163edc-995f-4513-9f44-48dbb07a7329.mp4` | Process Streamlining | We automate your processes by linking together the daily tools you rely upon. Lifting throughput and improving overall output. |
| 2 | `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_221040_e6ba7c5a-864e-46e9-871e-341a176a7e3e.mp4` | Strategic advisory | We craft intelligent assistants that are adaptive, grasp context, and are skilled enough to handle highly intricate customer requests. |
| 3 | `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_221104_fb538584-5b87-495f-952e-09ddd5a1792a.mp4` | Assistant engineering | Through our knowledge, we explore deep into your business and advise you on how AI powered automations may transform your operations. |

**Card structure (each card):**
- Container: `background: transparent; border: 1px solid rgba(0,0,0,0.18); border-radius: 20px; overflow: hidden; display: flex; flex-direction: column; min-height: 0; padding-top: 16px`
- **Video area:** `width: 100%; aspect-ratio: 4/3; position: relative; overflow: hidden`
  - `<video>` with `autoPlay muted loop playsInline`, styled `position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block`
- **Text area:** `padding: 24px 28px 28px 28px`
  - Title `<h3>`: `font-size: 18px; font-weight: 600; color: #1a1a1a; margin: 0; margin-bottom: 14px`
  - Description `<p>`: `font-size: 13px; line-height: 1.6; color: #3a3a3a; margin: 0`

**Animation (FadeUp component -- same as section 2):**
All animated elements use Framer Motion `whileInView` with `viewport: { once: true, amount: 0.2 }`, easing `[0.22, 1, 0.36, 1]`, duration `0.7s`, default `y: 24px` unless overridden.

**Font:**
```css
@import url('https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var');
* { font-family: 'Helvetica Now Var', 'Helvetica Neue', Helvetica, Arial, sans-serif; }
```

**Mobile responsive (max-width: 900px):**
- Section padding: `90px 18px 60px 18px`
- Section becomes `height: auto; min-height: 100vh` (so cards aren't crushed)
- Heading + subtext row stacks vertically: `flex-direction: column; gap: 16px; margin-bottom: 24px`
- Heading column becomes `width: 100%`
- Cards grid becomes single column: `grid-template-columns: 1fr; gap: 16px`

**Tech stack:** React 18, TypeScript, Vite, Tailwind CSS 3, Framer Motion 12.

---

## Nimbus Security — Cards [sites/nimbus-security]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(68).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-security.webp

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

**Shared h3 base:**
```css
h3 {
  margin-bottom: 14px;
  font-size: 24px;
  line-height: 1.2;
  letter-spacing: 0.0125rem;
  font-weight: 400;
}
```

---

### Section: `.security-section`

`<section class="security-section" id="security" aria-labelledby="security-title">`

**Outer container:**
```css
.security-section {
  padding: clamp(72px, 8vw, 116px) clamp(20px, 5vw, 72px);
  border-top: 1px solid rgba(255, 240, 199, 0.1);
  background:
    radial-gradient(circle at 88% 20%, rgba(234, 208, 154, 0.12), transparent 22rem),
    radial-gradient(circle at 16% 82%, rgba(255, 216, 121, 0.07), transparent 26rem),
    #120f0a;
}
```

The background is a dark warm brown (`#120f0a`) with two subtle radial glows: a warm gold top-right and a dimmer gold bottom-left.

---

### Part 1: `.security-heading` (two-column header)

```html
<div class="security-heading">
  <div>
    <p class="eyebrow">Security</p>
    <h2 id="security-title">Modern encryption and compliance controls without slowing the team down.</h2>
  </div>
  <p>
    Role-based access, customer-managed keys, immutable retention, and regional storage policies give business
    clients a cloud layer that can satisfy procurement, IT, and legal from the first deployment.
  </p>
</div>
```

```css
.security-heading {
  display: grid;
  grid-template-columns: minmax(0, 0.58fr) minmax(280px, 0.42fr);
  gap: clamp(28px, 5vw, 64px);
  align-items: end;
  width: min(100%, 1320px);
  margin: 0 auto clamp(36px, 5vw, 64px);
}

.security-heading h2 {
  max-width: 820px;
}

.security-heading > p {
  margin-bottom: 0;
  color: rgba(255, 244, 213, 0.68);
  font-size: clamp(16px, 1.25vw, 19px);
  line-height: 1.6;
}
```

The left column has the eyebrow + h2 stacked. The right column has the body paragraph, aligned to the bottom of the left column (`align-items: end`).

---

### Part 2: `.security-card-grid` (3 cards in a row)

```html
<div class="security-card-grid">
  <article class="security-card api-card">...</article>
  <article class="security-card compliance-card">...</article>
  <article class="security-card economics-card">...</article>
</div>
```

```css
.security-card-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: clamp(16px, 2vw, 22px);
  width: min(100%, 1320px);
  margin-inline: auto;
}
```

**Shared card base:**
```css
.security-card {
  position: relative;
  display: grid;
  grid-template-rows: auto 1fr;
  min-height: 464px;
  overflow: hidden;
  border: 1px solid rgba(255, 247, 222, 0.17);
  border-radius: 0;
  background:
    linear-gradient(180deg, rgba(255, 247, 222, 0.035), rgba(255, 247, 222, 0.012)),
    #0f0c08;
}
```

Cards are tall rectangles with sharp corners (no border-radius). Background is a near-black warm tone with a very subtle top-to-bottom fade. Border is a faint warm white. Each card is a 2-row grid: copy on top, visual below filling the remaining space.

**Card copy area:**
```css
.security-card-copy {
  padding: 20px 22px 0;
}

.security-card h3 {
  margin-bottom: 6px;
  color: rgba(255, 247, 222, 0.9);
  font-size: 16px;
  line-height: 1.25;
}

.security-card p {
  max-width: 330px;
  margin-bottom: 0;
  color: rgba(255, 247, 222, 0.52);
  font-size: 15px;
  line-height: 1.35;
}
```

**Shared visual area min-height:**
```css
.security-api-visual,
.compliance-list,
.economics-visual {
  position: relative;
  min-height: 300px;
}
```

---

### Card 1: "Full policy control" (`.api-card`)

```html
<article class="security-card api-card">
  <div class="security-card-copy">
    <h3>Full policy control</h3>
    <p>First-class API access for storage pools, keys, regions, and retention rules. No vendor lock-in to proprietary workflows.</p>
  </div>
  <div class="security-api-visual" aria-hidden="true">
    <div class="api-window">
      <span></span>
      <span></span>
      <span></span>
      <pre><code>-> nimbus auth login
Enter code
VAULT-9AMP

-> policy attach
workspace/client-vault</code></pre>
    </div>
    <div class="api-spec">
      <pre><code>openapi: 3.0.0
info:
  title: Nimbus API
paths:
  /storage/pools:
  /keys:
  /regions:
  /retention:</code></pre>
    </div>
  </div>
</article>
```

This card has two overlapping panels: a dark terminal window (bottom-left) and a golden spec card (top-right).

**Terminal window (`.api-window`):**
```css
.api-window {
  position: absolute;
  left: 26px;
  bottom: 28px;
  width: min(58%, 230px);
  min-height: 184px;
  border: 1px solid rgba(255, 247, 222, 0.13);
  background: rgba(8, 7, 5, 0.86);
}

.api-window > span {
  display: inline-block;
  width: 9px;
  height: 9px;
  margin: 10px 0 0 7px;
  border-radius: 50%;
  background: rgba(255, 247, 222, 0.28);
}

.api-window pre {
  padding: 62px 16px 16px;
}

.api-window pre,
.api-spec pre {
  margin: 0;
  color: rgba(255, 247, 222, 0.6);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.42;
  letter-spacing: 0;
  white-space: pre-wrap;
}
```

The 3 `<span>` elements are macOS-style dots positioned at the top-left of the window. The `pre` text sits below them with large top padding (62px) to create space below the dots.

**API spec card (`.api-spec`):**
```css
.api-spec {
  position: absolute;
  top: 56px;
  right: 26px;
  width: min(58%, 238px);
  padding: 16px 18px;
  border: 1px solid rgba(234, 208, 154, 0.38);
  background: rgba(64, 52, 30, 0.86);
  box-shadow: 0 22px 48px rgba(0, 0, 0, 0.3);
}

.api-spec pre {
  color: var(--accent);
}
```

This panel is a warm-brown card with a golden border, overlapping the terminal from the top-right. Text is in accent gold.

---

### Card 2: "Full compliance" (`.compliance-card`)

```html
<article class="security-card compliance-card">
  <div class="security-card-copy">
    <h3>Full compliance</h3>
    <p>SOC 2, ISO 27001, and GDPR-ready controls help teams satisfy audits, procurement reviews, and data residency requirements.</p>
  </div>
  <div class="compliance-list" aria-hidden="true">
    <div class="compliance-row">
      <span></span>
      <small>SOC 2</small>
      <strong>Type II controls</strong>
    </div>
    <div class="compliance-row">
      <span></span>
      <small>ISO 27001</small>
      <strong>Security management</strong>
    </div>
    <div class="compliance-row">
      <span></span>
      <small>GDPR</small>
      <strong>Regional data policy</strong>
    </div>
  </div>
</article>
```

3 stacked compliance badge rows, each with a checkmark circle, a standard label, and a description.

```css
.compliance-list {
  display: grid;
  align-content: center;
  gap: 12px;
  padding: 0 28px 30px;
}

.compliance-row {
  display: grid;
  grid-template-columns: 24px 1fr;
  grid-template-rows: auto auto;
  column-gap: 12px;
  align-items: center;
  min-height: 52px;
  padding: 10px 14px 10px 10px;
  border: 1px solid rgba(234, 208, 154, 0.28);
  background: rgba(48, 39, 23, 0.84);
}
```

Each row is a 2-column grid (icon column + text column) with 2 implicit rows (small label on top, strong description below). The `<span>` spans both rows via `grid-row: 1 / 3`.

**Checkmark circle (pure CSS):**
```css
.compliance-row span {
  grid-row: 1 / 3;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--accent);
}

.compliance-row span::before {
  content: "";
  display: block;
  width: 7px;
  height: 4px;
  margin: 6px 0 0 5px;
  border-bottom: 2px solid #211a0e;
  border-left: 2px solid #211a0e;
  transform: rotate(-45deg);
}
```

A gold circle with a dark checkmark made from two CSS borders rotated -45deg.

**Text labels:**
```css
.compliance-row small {
  color: rgba(234, 208, 154, 0.58);
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.05rem;
  text-transform: uppercase;
}

.compliance-row strong {
  color: var(--accent);
  font-size: 14px;
  font-weight: 400;
}
```

---

### Card 3: "Ownership and predictable economics" (`.economics-card`)

```html
<article class="security-card economics-card">
  <div class="security-card-copy">
    <h3>Ownership and predictable economics</h3>
    <p>Reserved capacity, clear transfer lanes, and audit-ready billing make storage spend easy to forecast across business units.</p>
  </div>
  <div class="economics-visual" aria-hidden="true">
    <pre class="binary-map">1111111111111111111111111111
1111111111000000111111111111
1111111100011110001111111111
1111111000111111000111111111
1111111000111111000111111111
1111111100000000001111111111
1111110000001100000011111111
1111100000001100000001111111
1111110000000000000011111111
1111111111111111111111111111</pre>
    <div class="asset-table">
      <div><span>Reserved tier</span><strong>24 TiB</strong></div>
      <div><span>Transfer lane</span><strong>EU Central</strong></div>
      <div><span>Revision</span><strong>Q603</strong></div>
    </div>
  </div>
</article>
```

This card has a binary art pattern (a lock icon formed from 0s and 1s) above a 3-row data table.

**Binary map:**
```css
.economics-visual {
  display: grid;
  align-content: end;
  justify-items: center;
  gap: 18px;
  padding: 0 26px 30px;
}

.binary-map {
  margin: 0;
  color: rgba(234, 208, 154, 0.72);
  font-family: var(--font-mono);
  font-size: clamp(10px, 0.9vw, 12px);
  line-height: 1.18;
  letter-spacing: 0;
}
```

The binary text is a 28-character-wide, 10-line block of 1s and 0s that visually forms a padlock shape. The 0s carve out the lock body and shackle. Displayed in gold monospace.

**Asset table:**
```css
.asset-table {
  width: min(100%, 302px);
  border: 1px solid rgba(255, 247, 222, 0.15);
}

.asset-table div {
  display: grid;
  grid-template-columns: 1fr 1fr;
  min-height: 32px;
  border-bottom: 1px solid rgba(255, 247, 222, 0.09);
}

.asset-table div:last-child {
  border-bottom: 0;
}

.asset-table span,
.asset-table strong {
  align-self: center;
  padding: 0 12px;
  color: rgba(255, 247, 222, 0.56);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0.035rem;
  text-transform: uppercase;
}

.asset-table strong {
  color: rgba(255, 247, 222, 0.78);
  text-transform: none;
}
```

A minimal bordered table with label/value columns. Labels are muted uppercase mono, values are brighter mono.

---

### Responsive Breakpoints

### `@media (max-width: 820px)`

```css
.security-heading {
  grid-template-columns: 1fr;
}

.security-card-grid {
  grid-template-columns: 1fr;
}

.security-card {
  min-height: 420px;
}
```

At 820px: Heading becomes single column (h2 above paragraph). Cards stack vertically. Card min-height reduces slightly.

### `@media (max-width: 520px)`

```css
.security-section {
  padding-inline: 18px;
}

.security-card-copy {
  padding: 18px 18px 0;
}

.security-card p {
  max-width: none;
}

.api-window {
  left: 18px;
  width: 62%;
}

.api-spec {
  right: 18px;
  width: 60%;
}

.compliance-list,
.economics-visual {
  padding-inline: 18px;
}
```

At 520px: Tighter padding throughout. API window and spec card widen proportionally. Card descriptions remove max-width constraint.

---

### Project structure

```
index.html       (section markup + font links)
styles.css       (all styles + media queries)
script.js        (empty or minimal — no JS needed for this section)
package.json     (vite ^5.4.2, "type": "module", scripts: dev/build/preview)
vite.config.js   (default export)
```

## Nimbus Sticky Cards — Cards [sites/nimbus-sticky-cards]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(42).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-sticky-cards.webp

---

### Prompt to Recreate the Nimbus Grid Platform Accordion Section (Standalone)

Build a standalone single-section page: the "Platform Accordion" from Nimbus Grid. Use plain HTML, CSS, and vanilla JS (Vite project, no frameworks). Match every detail below exactly.

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

**Resets:** Universal `box-sizing: border-box`. `html { scroll-behavior: smooth; }`. Body: `margin: 0`, background `var(--bg)`, color `var(--ink)`, `font-family: var(--font-sans)`, `font-size: 1rem`, `font-weight: 400`, `line-height: 1.375`, `letter-spacing: 0.0175rem`, antialiased (`-webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale`). Anchors: `color: inherit; text-decoration: none;`. All headings/paragraphs: `margin-top: 0`.

**Screen-reader utility:**
```css
.sr-only {
  position: absolute; width: 1px; height: 1px;
  overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap;
}
```

---

### Section: `.platform-accordion`

`<section class="platform-accordion" id="platform">` with a screen-reader-only `<h2 class="sr-only">` inside.

**Outer container styles:**
```css
.platform-accordion {
  position: relative;
  min-height: 420svh;
  border-top: 1px solid rgba(255, 240, 199, 0.1);
  background:
    radial-gradient(circle at 86% 30%, rgba(234, 208, 154, 0.13), transparent 24rem),
    #050604;
}
```

### `.accordion-inner` (sticky viewport frame)

```css
.accordion-inner {
  position: sticky;
  top: 0;
  display: grid;
  grid-template-columns: minmax(160px, 0.22fr) minmax(0, 0.78fr);
  gap: clamp(28px, 5vw, 72px);
  height: 100svh;
  padding: clamp(48px, 7vw, 86px) clamp(20px, 5vw, 72px);
  overflow: hidden;
}
```

---

### Left column: `.accordion-nav`

`<div class="accordion-nav" role="tablist">` containing 4 `<button class="accordion-tab">` elements. First button gets class `.active`.

**Tab labels (in order):**
1. `Programmable infra`
2. `Data residency`
3. `Elastic scaling`
4. `Unified visibility`

Each button has `data-accordion-tab="infra|residency|scaling|visibility"` respectively, `role="tab"`, `aria-selected="true|false"`.

```css
.accordion-nav {
  align-self: start;
  display: grid;
  gap: 16px;
  padding-top: 14px;
}

.accordion-tab {
  position: relative;
  border: 0;
  background: transparent;
  color: rgba(255, 247, 222, 0.38);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1rem;
  letter-spacing: 0.08rem;
  text-align: left;
  text-transform: uppercase;
  cursor: pointer;
  transition: color 160ms ease, transform 160ms ease;
}

.accordion-tab::before {
  content: "";
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 12px;
  border-radius: 1px;
  background: currentColor;
  vertical-align: 1px;
}

.accordion-tab.active {
  color: var(--accent);
  transform: translateX(2px);
}
```

---

### Right column: `.accordion-stack`

`<div class="accordion-stack" aria-live="polite">` containing 4 `<article class="accordion-card">` elements. First card gets class `.active`.

Each card has `data-accordion-card="infra|residency|scaling|visibility"`.

```css
.accordion-stack {
  position: relative;
  height: min(80svh, 820px);
  align-self: center;
  overflow: hidden;
}

.accordion-card {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-columns: minmax(220px, 0.35fr) minmax(340px, 0.65fr);
  border-top: 1px solid rgba(255, 247, 222, 0.2);
  background: #050604;
  transform: translateY(var(--card-y, 100%));
  clip-path: inset(0 0 var(--card-clip-bottom, 0px) 0);
  will-change: transform, clip-path;
}
```

---

### Card inner structure (each card)

**Left: `.accordion-copy`**
```html
<div class="accordion-copy">
  <h3>[Card title]</h3>
  <p>[Card description]</p>
</div>
```

```css
.accordion-copy {
  padding: 26px 30px 0 0;
}

.accordion-card h3 {
  margin-bottom: 28px;
  color: var(--ink);
  font-size: clamp(24px, 2.2vw, 40px);
  font-weight: 400;
  line-height: 1.2;
  letter-spacing: 0.0125rem;
}

.accordion-card p {
  max-width: 340px;
  margin-bottom: 0;
  color: var(--muted);
  font-size: clamp(15px, 1.4vw, 22px);
  line-height: 1.55;
}
```

**Right: `.accordion-visual`**

Gold gradient panel containing a dark code window.

```css
.accordion-visual {
  min-height: 100%;
  padding: clamp(34px, 5vw, 64px);
  background:
    linear-gradient(135deg, rgba(234, 208, 154, 0.92), rgba(106, 91, 52, 0.68)),
    radial-gradient(circle at 15% 20%, rgba(255, 247, 222, 0.65), transparent 20rem);
  overflow: hidden;
}
```

**Code window (`.code-window`):**
```css
.code-window {
  width: min(420px, 100%);
  margin-inline: auto;
  padding: 18px 20px 24px;
  border-radius: 8px;
  background: rgba(8, 10, 10, 0.88);
  box-shadow: 0 28px 70px rgba(0, 0, 0, 0.34);
}

.metric-window {
  margin-top: clamp(18px, 5vw, 70px);
}

[data-accordion-card="infra"] .code-window {
  margin-top: 40px;
}

[data-accordion-card="visibility"] .metric-window {
  margin-top: clamp(12px, 3.75vw, 52px);
}

.code-window > span {
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 5px;
  border-radius: 50%;
  background: rgba(255, 247, 222, 0.56);
}

.code-window pre {
  margin: 22px 0 0;
  color: rgba(255, 247, 222, 0.82);
  font-family: var(--font-mono);
  font-size: clamp(11px, 1vw, 14px);
  line-height: 1.55;
  letter-spacing: 0;
  white-space: pre-wrap;
}
```

Each code window has 3 `<span></span>` dot elements (the 3 macOS-style dots), then a `<pre><code>` block.

---

### Card content (exact text)

**Card 1 — `infra`:**
- Title: `Programmable infra`
- Description: `Define storage pools, quotas, regions, and access policy in code so every workspace ships with the same controls.`
- Code window (no `.metric-window` class):
```
01  storage_pool = {
02    name      = "client-vault"
03    region    = "eu-central"
04    quota     = "24 TiB"
05    policy    = encrypted_fast
06  }
```

**Card 2 — `residency`:**
- Title: `Data residency`
- Description: `Pin departments and client workspaces to approved regions with retention and encryption rules attached from day one.`
- Code window (class: `.code-window .metric-window`):
```
Region policy

EU Central     locked
US East        allowed
AP Southeast   review
Retention      7 years
```

**Card 3 — `scaling`:**
- Title: `Elastic scaling`
- Description: `Expand capacity before teams hit limits, route large transfers through faster lanes, and keep procurement predictable.`
- Code window (class: `.code-window .metric-window`):
```
Capacity forecast

Used       18.4 TiB
Reserved   24 TiB
Burst      ready
Next tier  approved
```

**Card 4 — `visibility`:**
- Title: `Unified visibility`
- Description: `Track growth, usage pressure, inactive files, and compliance drift from a single operational surface.`
- Code window (class: `.code-window .metric-window`):
```
Operations view

Sync health       stable
Cold data         14%
Policy drift       0
Audit export      live
```

---

### JavaScript: Scroll-driven accordion

```js
const accordionSection = document.querySelector(".platform-accordion");
const accordionTabs = Array.from(document.querySelectorAll("[data-accordion-tab]"));
const accordionCards = Array.from(document.querySelectorAll("[data-accordion-card]"));

function activateAccordionPanel(panelName) {
  accordionTabs.forEach((tab) => {
    const isActive = tab.dataset.accordionTab === panelName;
    tab.classList.toggle("active", isActive);
    tab.setAttribute("aria-selected", String(isActive));
  });
  accordionCards.forEach((card) => {
    card.classList.toggle("active", card.dataset.accordionCard === panelName);
  });
}

function updateScrollAccordion() {
  if (!accordionSection || !accordionCards.length) return;

  const rect = accordionSection.getBoundingClientRect();
  const scrollable = Math.max(1, rect.height - window.innerHeight);
  const progress = Math.min(1, Math.max(0, -rect.top / scrollable));
  const maxIndex = accordionCards.length - 1;
  const rawIndex = progress * maxIndex;
  const activeIndex = Math.min(maxIndex, Math.max(0, Math.round(rawIndex)));
  const stack = document.querySelector(".accordion-stack");
  const stackHeight = stack ? stack.clientHeight : window.innerHeight * 0.74;
  const collapsedHeight = window.innerWidth <= 820 ? 96 : 84;

  const cardPositions = accordionCards.map((_, index) => {
    let y = 0;
    if (index > 0) {
      const segmentProgress = Math.min(1, Math.max(0, rawIndex - (index - 1)));
      const startY = stackHeight + collapsedHeight;
      const endY = index * collapsedHeight;
      y = startY + (endY - startY) * segmentProgress;
    }
    return Math.round(y);
  });

  accordionCards.forEach((card, index) => {
    const y = cardPositions[index];
    const nextY = cardPositions[index + 1];
    const visibleHeight = typeof nextY === "number"
      ? Math.max(collapsedHeight, Math.min(stackHeight, nextY + 2))
      : stackHeight;
    const clipBottom = Math.max(0, stackHeight - visibleHeight);

    card.style.setProperty("--card-y", `${Math.round(y)}px`);
    card.style.setProperty("--card-clip-bottom", `${Math.round(clipBottom)}px`);
    card.style.zIndex = String(index + 1);
  });

  const activeCard = accordionCards[activeIndex];
  if (activeCard) activateAccordionPanel(activeCard.dataset.accordionCard);
}

// Tab click scrolls to the correct segment
accordionTabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    if (!accordionSection || !accordionCards.length) return;
    const index = accordionCards.findIndex(
      (card) => card.dataset.accordionCard === tab.dataset.accordionTab
    );
    const maxIndex = accordionCards.length - 1;
    const scrollable = accordionSection.offsetHeight - window.innerHeight;
    const target = accordionSection.offsetTop + (index / maxIndex) * scrollable;
    window.scrollTo({ top: target, behavior: "smooth" });
  });
});

window.addEventListener("scroll", updateScrollAccordion, { passive: true });
window.addEventListener("resize", updateScrollAccordion);
updateScrollAccordion();
```

**How the scroll math works:**
- The section is `420svh` tall. As the user scrolls through it, `progress` goes from 0 to 1.
- `rawIndex` maps progress to a float 0..3 (for 4 cards).
- Each card starts off-screen at `stackHeight + collapsedHeight` px below, then slides up to `index * collapsedHeight` (84px desktop, 96px mobile) — stacking as visible header strips.
- `clip-path: inset(0 0 VAR 0)` clips the bottom of each card so only the header strip shows when another card sits on top.
- The active card (nearest integer index) is fully revealed (no clip).

---

### Responsive Breakpoints

### `@media (max-width: 820px)`

```css
.platform-accordion {
  min-height: 420svh;
}

.accordion-inner {
  grid-template-columns: 1fr;
  grid-template-rows: auto 1fr;
  gap: 22px;
  padding: 34px 20px;
}

.accordion-nav {
  align-self: start;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  padding-top: 0;
}

.accordion-stack {
  height: 78svh;
  align-self: stretch;
}

.accordion-card {
  grid-template-columns: 1fr;
  grid-template-rows: auto 1fr;
}

.accordion-copy {
  padding: 22px 0 24px;
}

.accordion-visual {
  min-height: 0;
  padding: 28px;
}
```

At 820px: Nav becomes a 2-column grid above the stack. Cards become single-column (copy stacked above visual). Collapsed height changes to 96px (handled in JS via `window.innerWidth <= 820 ? 96 : 84`).

### `@media (max-width: 520px)`

```css
.accordion-nav {
  grid-template-columns: 1fr;
}

.accordion-inner {
  padding-inline: 18px;
}

.accordion-card h3 {
  font-size: 26px;
}

.accordion-card p {
  font-size: 14px;
}

.accordion-visual {
  padding: 18px;
}
```

At 520px: Nav becomes single-column. Tighter padding. Smaller text.

---

### Project structure

```
index.html       (section markup + font links)
styles.css       (all styles + media queries)
script.js        (scroll-driven accordion + tab click)
package.json     (vite ^5.4.2, "type": "module", scripts: dev/build/preview)
vite.config.js   (default export)
```

## Orbis Cards — Cards [sites/orbis-cards]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(16).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbis-cards.webp

---
### Prerequisites

**Fonts** (loaded via Google Fonts in `index.html`):
- `Anton` (mapped to `font-grotesk` in Tailwind)
- `Condiment` (mapped to `font-condiment` in Tailwind)

**Tailwind custom colors** (in `tailwind.config.js`):
- `cream`: `#EFF4FF`
- `neon`: `#6FFF00`

**Global background color**: `#010828` (dark navy/space blue)

**Custom CSS class `liquid-glass`** (defined in `index.css`):
```css
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
```

---

### Section Structure (line-by-line)

**Outer wrapper**: `<section>` with classes `relative py-16 sm:py-24 md:py-32` and inline style `backgroundColor: '#010828'`.

**Inner container**: `<div>` with `max-w-[1831px] mx-auto px-4 sm:px-6 md:px-8`.

---

### Header Row

A `flex flex-col lg:flex-row items-start lg:justify-between gap-6 lg:gap-0 mb-8 sm:mb-12` div containing:

**1. Title (left side)**:
- `<h2>` with `font-grotesk text-[32px] sm:text-[48px] md:text-[60px] font-normal uppercase leading-[1.05] sm:leading-[1] md:leading-[1]`
- Line 1: `Collection of` followed by `<br />`
- Line 2: Wrapped in `<span className="ml-12 sm:ml-24 md:ml-32 inline-block">` containing:
  - `<span className="font-condiment text-neon text-[36px] sm:text-[52px] md:text-[68px] font-normal normal-case">Space</span>` (the word "Space" in neon green cursive Condiment font)
  - followed by the text ` objects` (in Anton/grotesk uppercase)

**2. "SEE ALL CREATORS" button (right side)**:
- `<button className="group relative flex flex-col items-start">`
- Inner div: `font-grotesk font-normal uppercase leading-[1.1] flex items-start gap-3`
  - `<span className="text-[32px] sm:text-[48px] md:text-[60px] text-white">SEE</span>`
  - A `flex flex-col items-start leading-[0.9]` div with:
    - `<span className="text-[20px] sm:text-[28px] md:text-[36px] text-white">ALL</span>`
    - `<span className="text-[20px] sm:text-[28px] md:text-[36px] text-white">CREATORS</span>`
- Below text: `<div className="w-full h-[6px] sm:h-[8px] md:h-[10px] bg-neon mt-3 sm:mt-4" />` (neon green underline bar)

---

### Card Grid

`<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">`

Mapped over an array of 3 items, each with a `video` URL and `score` string:

**Card data**:
```js
[
  {
    video: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_053923_22c0a6a5-313c-474c-85ff-3b50d25e944a.mp4',
    score: '8.7/10'
  },
  {
    video: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_054411_511c1b7a-fb2f-42ef-bf6c-32c0b1a06e79.mp4',
    score: '9/10'
  },
  {
    video: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_055427_ac7035b5-9f3b-4289-86fc-941b2432317d.mp4',
    score: '8.2/10'
  }
]
```

**Each card** (`flex flex-col gap-4 group`):

**Video container**:
- Outer: `liquid-glass rounded-[32px] p-[18px] hover:bg-white/10 transition-all duration-300`
- Inner: `relative rounded-[24px] overflow-hidden pb-[75%]` (4:3 aspect ratio via padding-bottom trick)
- `<video autoPlay loop muted playsInline className="absolute inset-0 w-full h-full object-cover">`
  - `<source src={item.video} type="video/mp4" />`

**Info bar below video**:
- `liquid-glass rounded-[20px] px-5 py-4 flex items-center justify-between`
- Left side (`flex flex-col`):
  - Label: `<span className="font-grotesk text-[14px] uppercase text-cream/70 mb-1">RARITY SCORE:</span>`
  - Value: `<span className="font-grotesk text-[20px] font-normal text-cream">{item.score}</span>`
- Right side: circular arrow button
  - `<button className="w-[48px] h-[48px] liquid-glass rounded-full flex items-center justify-center hover:scale-110 hover:bg-white/10 transition-all flex-shrink-0">`
  - SVG chevron right: `<svg className="w-[18px] h-[18px] text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M9 5l7 7-7 7" /></svg>`

---

### Summary of Visual Design

- Dark space-navy background (`#010828`)
- Glassmorphism cards using the `liquid-glass` class (subtle white gradient border, blur backdrop, inner glow)
- Typography: Anton font (bold geometric sans-serif) for all headings, uppercase
- The word "Space" breaks style by using Condiment (cursive/script) in neon green (`#6FFF00`)
- Cream white text (`#EFF4FF`) throughout
- 3-column responsive grid (1 col mobile, 2 col tablet, 3 col desktop)
- Each card has an autoplay looping muted video with 4:3 aspect ratio inside a glass container with 32px rounding and 18px padding, plus a glass info bar below showing rarity score and a circular chevron button
- Responsive font scaling from 32px to 60px for headings across breakpoints
- Max content width: 1831px, centered with horizontal padding of 16px/24px/32px at sm/md breakpoints

## Veloce Cards — Cards [sites/veloce-cards]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(60).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/veloce-cards.webp

Build an "Insights" stats section for a fintech landing page using React + TypeScript + Vite + Tailwind CSS + Framer Motion. This is a standalone section component. It must be fully mobile responsive.

**Dependencies:** `framer-motion`

**Tailwind config -- extend `fontFamily`:**
```js
'helvetica-neue': ['Helvetica Neue', 'Helvetica', 'Arial', 'sans-serif'],
```

**CSS (`index.css`) -- add globally:**
```css
@layer base {
  * {
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
}
```

---

**BlurIn utility component (`src/components/BlurIn.tsx`):**

A reusable wrapper. Uses Framer Motion's `useInView` hook with `{ once: true }`. Wraps children in a `motion.div` that animates from `filter: blur(20px), opacity: 0` to `filter: blur(0px), opacity: 1` over `duration: 1.2` seconds when it enters the viewport. Named export `BlurIn`.

---

**InsightsSection component (`src/components/InsightsSection.tsx`):**

**Outer container:** `<div>` with class `px-6 md:px-12 lg:px-[60px] flex flex-col gap-[90px] py-20 bg-white`.

---

**Top text block** (`max-w-[517px] flex flex-col gap-10`):

1. Heading wrapped in `<BlurIn>`: Text `"Instant payment clarity counts"`. Class: `text-[#00041F] text-4xl md:text-5xl lg:text-6xl font-helvetica-neue font-medium leading-[1] lg:leading-[60px] tracking-[-0.03em]`.

2. Paragraph (NOT wrapped in BlurIn): Text `"Real-time data powers smarter spending choices every day"`. Class: `text-[#49484F] text-base md:text-lg lg:text-xl font-helvetica-neue max-w-[361px]`.

---

**Cards row** -- a `motion.div` with class `flex flex-col lg:flex-row items-stretch lg:items-end gap-5`.

Framer Motion config on the row container:
- `variants`: `hidden: { opacity: 0 }`, `visible: { opacity: 1, transition: { staggerChildren: 0.2 } }`
- `initial="hidden"`
- `whileInView="visible"`
- `viewport={{ once: true, amount: 0.2 }}`

Each card is a `motion.div` with variants: `hidden: { opacity: 0, y: 30 }`, `visible: { opacity: 1, y: 0, transition: { duration: 0.6, ease: 'easeOut' } }`.

Each card has base class: `flex-1 p-10 rounded-[40px] relative overflow-hidden flex flex-col justify-end`.

Each card contains (in order):
1. A `<video autoPlay loop muted playsInline>` with class `absolute inset-0 w-full h-full object-cover` and a `<source>` with the specific CloudFront URL and `type="video/mp4"`.
2. A color overlay `<div>` with class `absolute inset-0` and a specific background color.
3. A content block `<div>` with class `relative z-10 max-w-[388px] flex flex-col gap-5` containing the stat number and description.

Stat number style (all cards): `text-[#00041F] text-5xl md:text-[60px] font-helvetica-neue font-medium leading-[1] md:leading-[60px]`

Description style (all cards): `text-[#49484F] text-lg md:text-[22px] font-helvetica-neue opacity-80`

---

**Card 1:**
- Min height: `min-h-[450px]`
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_143605_bc7bd6c0-9c68-49ff-a9d3-073a10759fa4.mp4`
- Overlay color: `bg-[rgba(206,223,235,0.25)]`
- Stat: `1.6M`
- Description: `"Active members rely on us for effortless payment experiences"` with `max-w-[377px]`

**Card 2:**
- Min height: `min-h-[350px]` (shorter than the others -- this creates the staggered bottom alignment on desktop via `lg:items-end` on the parent)
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_145119_f4ec4d9f-3ecd-4116-baa3-26e8cf2df976.mp4`
- Overlay color: `bg-[rgba(247,236,233,0.6)]`
- Stat: `850K`
- Description: `"Transfers completed each day, quick and protected"` with `max-w-[351px]`

**Card 3:**
- Min height: `min-h-[450px]`
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_140728_ae719193-f10b-4105-82fc-c989610b3aa6.mp4`
- Overlay color: `bg-[rgba(218,218,218,0.2)]`
- Stat: `120+`
- Description: `"Nations enabled for instant checkouts and worldwide remittance"` with `max-w-[351px]`

---

**Responsive behavior:**
- On mobile/tablet (`< lg`): Cards stack vertically (`flex-col`), each stretching full width (`items-stretch`).
- On desktop (`lg:`): Cards display in a horizontal row (`flex-row`) aligned to the bottom (`items-end`), so the shorter middle card (350px) sits higher at the bottom while the taller outer cards (450px) extend above it.
- Horizontal padding scales: `px-6` on mobile, `md:px-12` on tablet, `lg:px-[60px]` on desktop.
- Heading scales: `text-4xl` on mobile, `md:text-5xl` on tablet, `lg:text-6xl` on desktop.

---

## FlowMate Carousal — Carousal [sites/flowmate-carousal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(65).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/flowmate-carousal.webp

**PROMPT:**

Build a standalone React + TypeScript + Vite page with Tailwind CSS, Framer Motion, and Lucide React. The page contains ONLY a single Cards Carousel section. No sidebar, no navbar, no footer, no other sections.

Use the system font stack: `-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif` with `-webkit-font-smoothing: antialiased` and `-moz-osx-font-smoothing: grayscale`.

The section has a white background (`bg-white`), padding `py-20 px-6`, a top border of `1px solid #e8e8e8`, and a max-width container of `max-w-7xl` centered with `mx-auto`.

At the top of the section is a header row using flexbox with `justify-between` and `mb-12`. On the left is the text "FlowMate" in `text-xl` on mobile, `text-2xl` on `md:` breakpoint, `font-medium`, `text-black`. On the right are two circular navigation buttons side by side with `gap-3`. Each button is `w-10 h-10`, `rounded-full`, has a `border border-black/20` that changes to `border-black/40` on hover with `transition-colors`, and centers a Lucide `ChevronLeft` or `ChevronRight` icon at `w-5 h-5 text-black`.

Below the header is a card grid: `grid grid-cols-1 md:grid-cols-3 gap-6 relative`. It displays 3 cards at a time from a rotating pool of 5 cards. The carousel auto-advances every 4 seconds, cycling forward through the cards. The visible cards are calculated by taking 3 consecutive items starting from the current index, wrapping around using modulo.

The 5 cards have this exact data:

Card 1 -- label: "For Everyone", text: "Unleash your creative vision", image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081328_19f48c5b-ea4d-4f23-8f80-7374f31015d4.png&w=1280&q=85`

Card 2 -- label: "For Teams", text: "Smart helper supporting each teammate daily", image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081342_ad378347-1ebd-4b17-a716-ee895bf739c0.png&w=1280&q=85`

Card 3 -- label: "For Enterprises", text: "Elevate your whole organization using business AI", image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081415_a6e8a76c-224e-417b-bf99-6b86d6494644.png&w=1280&q=85`

Card 4 -- label: "Platform", text: "Enhanced with FlowMate", image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081513_cf1cd2c1-2122-4de6-90ed-acae8bfbdb00.png&w=1280&q=85`

Card 5 -- label: "Security", text: "Creating trusted and helpful AI", image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081541_9d2d28bf-d6a3-4b31-b0bb-cfc5202d4fcd.png&w=1280&q=85`

Each card is `h-[500px]`, `rounded-2xl`, `overflow-hidden`, with `cursor-pointer` and a CSS group for hover. It has three layers stacked with absolute positioning:

Layer 1 (bottom): The background image set via inline `backgroundImage` style, with classes `absolute inset-0 bg-cover bg-center transition-transform duration-500 group-hover:scale-105` so the image zooms slightly on hover.

Layer 2 (middle): A gradient overlay div with `absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent`.

Layer 3 (top): A content div with `relative h-full flex flex-col justify-between p-6`. At the top of this flex column is the label as a `span` with `inline-block py-1 text-white text-sm font-medium`. At the bottom is the description text as a `p` with `text-white text-xl font-normal leading-snug`.

Card slide animations use Framer Motion's AnimatePresence with `mode="popLayout"` and `initial={false}`. Track a `direction` state variable (1 for forward, -1 for backward). Each card's motion.div key must be `${card.id}-${currentIndex}-${idx}` to force remount on every change. Animation values:

- initial: `{ opacity: 0, x: direction > 0 ? 100 : -100, scale: 0.95 }`
- animate: `{ opacity: 1, x: 0, scale: 1 }`
- exit: `{ opacity: 0, x: direction > 0 ? -100 : 100, scale: 0.95 }`
- transition: `{ duration: 0.7, ease: [0.32, 0.72, 0, 1], opacity: { duration: 0.5 } }`

The entire section content (header row and cards grid) is wrapped in a scroll-triggered reveal animation. When the section scrolls into view (detected once), each direct child staggers in with 0.1s delay between them. Each child animates from `{ opacity: 0, y: 18 }` to `{ opacity: 1, y: 0 }` using a spring transition. Use Framer Motion's `useInView` with `{ once: true }` for detection.

The left arrow button decreases the index by 1 (wrapping to end), setting direction to -1. The right arrow button increases the index by 1 (wrapping to start), setting direction to 1. The auto-advance timer always sets direction to 1 before incrementing.

Responsive: on mobile the grid is single column (`grid-cols-1`), on `md:` breakpoint and up it becomes 3 columns (`md:grid-cols-3`). Title scales from `text-xl` to `md:text-2xl`. Cards stay `h-[500px]` at all sizes.

---

## Pixel Grid Hover — Case Studies [sites/pixel-grid-hover]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(15).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/pixel-grid-hover.webp

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

## Beauty Categories — Categories [sites/beauty-categories]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(67).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/beauty-categories.webp

---

**Prompt to recreate the "Categories" section:**

> Build a "Categories" section in React + Tailwind CSS with the following exact specifications:
>
> **Section Container:**
> - Full-width `<section>` with `bg-white`, `text-white`, `min-h-screen`.
> - Flexbox column layout with `justify-center` to vertically center the grid content.
> - No horizontal or vertical padding on the section itself.
>
> **Grid Layout:**
> - A CSS grid: `grid-cols-1` on mobile, `md:grid-cols-3` on medium+.
> - The entire grid uses an IntersectionObserver-based reveal animation (threshold `0.1`): transitions from `opacity-0 translate-y-12` to `opacity-100 translate-y-0` over `duration-1000 ease-out`.
>
> **3 Category Cards with exact data:**
> 1. Name: `"face"` | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203023_87a26602-2898-4acc-a396-c7a2b5ad84fd.mp4`
> 2. Name: `"beauty tools"` | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203415_b86e3f19-2aec-46cd-9a86-b64c40118e38.mp4`
> 3. Name: `"body"` | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203051_85fee398-ea01-4aa0-972b-137a74213be5.mp4`
>
> **Card Layout (each card):**
> - `position: relative`, flexbox column with `justify-between`, `items-start`.
> - Padding: `p-6` mobile, `sm:p-8`, `md:p-12`.
> - Heights: `min-h-[400px]` mobile, `sm:min-h-[500px]`, `md:min-h-[750px]`.
> - `overflow-hidden`.
> - Each card has a staggered `transitionDelay` of `index * 150ms`.
> - Uses `group` for hover interactions.
>
> **Background Video:**
> - `<video>` element with attributes: `autoPlay`, `loop`, `muted`, `playsInline`.
> - Positioned absolutely: `absolute inset-0 w-full h-full object-cover`.
> - Hover effect: `scale-105` over `duration-700` via `transition-transform group-hover:scale-105`.
> - The `src` attribute is set directly from the video URL (no `<source>` tag).
>
> **Dark Overlay:**
> - A `<div>` absolutely positioned over the video: `absolute inset-0`.
> - Default: `bg-black/10`. On hover: `group-hover:bg-black/20`.
> - `transition-colors duration-500`.
>
> **Category Name (vertical text):**
> - `<h2>` tag, positioned above overlay: `relative z-10`.
> - Font sizes: `text-5xl` mobile, `sm:text-6xl`, `md:text-7xl`, `lg:text-8xl`. Weight: `font-medium`.
> - **Vertical text**: achieved with inline style `writingMode: 'vertical-lr'` combined with `transform: 'rotate(180deg)'` (this makes text read bottom-to-top).
> - Hover: `group-hover:-translate-y-2` over `duration-500`.
> - Text is lowercase (rendered as-is from the data: "face", "beauty tools", "body").
>
> **Shop Button:**
> - `<button>` with class `btn-primary` (see CSS below) plus `relative z-10 mt-auto px-8 py-3 bg-white text-black rounded-full text-sm`.
> - Text: `"shop {category name}"` in lowercase (e.g., "shop face", "shop beauty tools", "shop body").
> - `mt-auto` pushes the button to the bottom of the card.
>
> **Required CSS for `btn-primary` (in global stylesheet):**
> ```css
> .btn-primary {
>   position: relative;
>   overflow: hidden;
>   transition: transform 0.3s ease, box-shadow 0.3s ease;
> }
> .btn-primary::before {
>   content: '';
>   position: absolute;
>   inset: 0;
>   background: linear-gradient(120deg, transparent 0%, rgba(0, 0, 0, 0.05) 50%, transparent 100%);
>   transform: translateX(-100%);
>   transition: transform 0.5s ease;
> }
> .btn-primary:hover {
>   transform: translateY(-2px);
>   box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
> }
> .btn-primary:hover::before {
>   transform: translateX(100%);
> }
> .btn-primary:active {
>   transform: translateY(0);
>   box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
> }
> ```
>
> **IntersectionObserver hook (`useInView`):**
> - Accepts a `threshold` parameter (default `0.15`), uses a ref.
> - Observes the element; once `isIntersecting` is true, sets `isVisible = true` and unobserves (one-shot animation).
> - Returns `{ ref, isVisible }`.
> - This section calls it with threshold `0.1`.

---

## Loader Animation — Component [sites/18]

- Preview: https://motionsites.ai/assets/hero-loader-animation-preview-C3_SX_Io.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/18.gif

Build a fullscreen loading screen component in React (Next.js 14, TypeScript). Uses Framer Motion for animations. Here is the exact specification:

Theme

css

--bg: #0a0a0a;
--text: #f5f5f5;
--muted: #888888;
--stroke: #1f1f1f;

Fonts: font-display → Instrument Serif (Google Fonts, italic, weight 400).

Component: LoadingScreen

Receives one prop: onComplete: () => void.

Container: <motion.div> — fixed inset-0 z-[9999] bg-bg. Exit animation: exit={{ opacity: 0 }}, duration 0.6s, ease [0.4, 0, 0.2, 1]. Wrap in <AnimatePresence mode="wait"> from the parent.

Element 1: "Portfolio" Label (Top-Left)

<motion.div> — absolute top-8 left-8 md:top-12 md:left-12.
Text: "Portfolio"
Class: text-xs md:text-sm text-muted uppercase tracking-[0.3em]
Entrance animation: initial={{ opacity: 0, y: -20 }}, animate={{ opacity: 1, y: 0 }}, duration 0.6s, delay 0.1s

Element 2: Rotating Words (Center)

absolute inset-0 flex items-center justify-center.
Three words cycle in sequence: "Design" → "Create" → "Inspire". A new word appears every 900ms. The word index increments via setInterval and stops at the last word (doesn't loop).

Each word is a <motion.span> inside <AnimatePresence mode="wait">, keyed by wordIndex:
Class: text-4xl md:text-6xl lg:text-7xl font-display italic text-text/80
initial={{ opacity: 0, y: 20 }}
animate={{ opacity: 1, y: 0 }}
exit={{ opacity: 0, y: -20 }}
transition={{ duration: 0.4, ease: [0.4, 0, 0.2, 1] }}

Element 3: Counter (Bottom-Right)

<motion.div> — absolute bottom-8 right-8 md:bottom-12 md:right-12.
A number that counts from 000 → 100 over exactly 2.7 seconds using requestAnimationFrame. Each frame calculates elapsed / 2700 * 100. The number is displayed zero-padded to 3 digits (e.g. 007, 042, 100):

{Math.round(progress).toString().padStart(3, '0')}

Class: text-6xl md:text-8xl lg:text-9xl font-display text-text tabular-nums
Entrance animation: initial={{ opacity: 0, y: 20 }}, animate={{ opacity: 1, y: 0 }}, duration 0.6s, delay 0.1s

When progress reaches 100: Wait 400ms, then call onComplete(). Use a ref for onComplete to avoid stale closures.

Element 4: Progress Bar (Bottom Edge)

absolute bottom-0 left-0 right-0. A 3px tall track:
Track: h-[3px] bg-stroke/50 (full width)
Fill: <motion.div> inside the track:
h-full origin-left
Background: linear-gradient(90deg, #89AACC 0%, #4E85BF 100%)
Glow: boxShadow: "0 0 8px rgba(137, 170, 204, 0.35)"
initial={{ scaleX: 0 }}
animate={{ scaleX: progress / 100 }}
transition={{ duration: 0.1, ease: "linear" }}

Parent Wrapper Behavior

The parent component (AppWrapper) controls visibility:
State: isLoading starts true
Renders <LoadingScreen onComplete={() => setIsLoading(false)} /> inside <AnimatePresence mode="wait"> only when isLoading is true
Main page content sits below with: style={{ opacity: isLoading ? 0 : 1, transition: "opacity 0.5s ease-out" }}
When the loader calls onComplete, it triggers: loader fades out (0.6s) → page fades in (0.5s)

Timing Summary

0.0s — Loader appears, "Portfolio" slides in, counter starts at 000
0.0s — "Design" appears
0.9s — "Create" replaces "Design"
1.8s — "Inspire" replaces "Create"
2.7s — Counter hits 100, progress bar full
3.1s — onComplete fires (400ms delay)
3.1s — Loader fades out (0.6s exit animation)
3.7s — Page content fades in (0.5s opacity transition)

## Animated Cards — Component [sites/animated-cards]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(8).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/animated-cards.webp

Create a high-performance, interactive 3D horizontal cylinder carousel showing premium animated bank cards.
Core Features & Interactions:
Use React (useState, useEffect, useRef), Tailwind CSS v4, and standard requestAnimationFrame for a smooth 60fps render loop. No external animation libraries needed.
The scene should behave like a continuous circular scroll/carousel, updating a continuous progress variable.
Add interactive 3D parallax tilt to the cards that smoothly responds to mouse cursor movement (mousemove), using inertia damping to lag slightly behind the cursor.
The cards should have real volumetric 3D thickness (achieved by stacking multiple div layers close together, simulating 3D depth).
The carousel math should push cards to the sides (using smoothstep interpolation) and hide them gracefully using perspective formulas as they move completely off-screen.
Each card must have a front and back face. The front face includes an autoplaying video background, a silver metallic chip (SVG), an embedded JWT logo top-right, and intersecting circles bottom-right. The back face should blur the same video background, have a dark magnetic stripe across the top, and feature the cardholder name, number, and CVV in JetBrains Mono.
Visual Styling:
Use a pure black background (#000000).
The application relies exclusively on the interactive 3D card layout (no text layers over the background).
Make sure the scene's wrapper uses CSS perspective: 1350px; and standard transformStyle: preserve-3d.
Please use the exact code below for src/App.tsx and src/index.css to build this exactly as requested.

import React, { useState, useEffect, useRef } from 'react';

const CARD_VIDEOS = [
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260506_030111_a9e15665-d379-4a7f-8116-695bbe452ad1.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260429_171347_f640c30d-ec21-426a-98bc-77e07c2c60cb.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260503_104800_bc43ae09-f494-43e3-97d7-2f8c1692cfd7.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_161253_c72b1869-400f-45ed-ac0c-52f68c2ed5bd.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260418_115655_b4d9cd77-feed-43cd-a198-af78ebdf1f7a.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_024928_1efd0b0d-6c02-45a8-8847-1030900c4f63.mp4',
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_024928_1efd0b0d-6c02-45a8-8847-1030900c4f63.mp4'
];

// Nine beautiful premium solid colors to clearly track the cards
const CARD_COLORS = [
  '#FF3B30', // Apple Red
  '#FF9500', // Apple Orange
  '#FFCC00', // Apple Yellow
  '#34C759', // Apple Green
  '#007AFF', // Apple Blue
  '#5856D6', // Apple Purple
  '#FF2D55', // Apple Pink
  '#AF52DE', // Apple Violet
  '#00C7BE', // Apple Teal
];

// Different card details for each of the cards
const CARD_DETAILS = [
  { number: '4232 8908 1121 4892', name: 'ZACHARY MERCER', cvv: '382' },
  { number: '4154 7831 9904 5124', name: 'SOPHIA MARTINEZ', cvv: '109' },
  { number: '5457 4120 7733 9035', name: 'BENJAMIN CARTER', cvv: '764' },
  { number: '4441 5567 1223 2468', name: 'EMILY MORRISON', cvv: '491' },
  { number: '5375 8891 2234 7713', name: 'JACKSON REID', cvv: '255' },
];


export default function App() {
  const cardCount = 5;
  const cardsRefs = useRef<(HTMLDivElement | null)[]>([]);
  const frameId = useRef<number>(0);
  
  // Continuous scroll progress
  const progress = useRef<number>(0);

  // Track mouse coordinates for interactive 3D parallax tilt with inertia damping
  const mouse = useRef({ x: 0, y: 0, targetX: 0, targetY: 0 });

  // Responsive state containing card dimensions
  const [metrics, setMetrics] = useState({
    cardW: 336,
    cardH: 211, // 1.59 standard credit card ratio
  });

  // Typography metrics to prevent collisions beautifully across all viewports
  const [fontMetrics, setFontMetrics] = useState({
    titleFontSize: '1.5rem',
    sigFontSize: '2.5rem',
    descFontSize: '14px',
    titleGap: '40px',
    pl: '0px'
  });

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      // Screen-space cursor offset relative to window center, clamped to [-1.0, 1.0] range
      const rx = (e.clientX - window.innerWidth / 2) / (window.innerWidth / 2);
      const ry = (e.clientY - window.innerHeight / 2) / (window.innerHeight / 2);
      mouse.current.targetX = Math.max(-1, Math.min(1, rx));
      mouse.current.targetY = Math.max(-1, Math.min(1, ry));
    };

    const handleMouseLeave = () => {
      // Return gently to center orientation when mouse focus is lost or moves away
      mouse.current.targetX = 0;
      mouse.current.targetY = 0;
    };

    window.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseleave', handleMouseLeave);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, []);

  useEffect(() => {
    const handleResize = () => {
      const w = window.innerWidth;
      const h = window.innerHeight;

      // 1. Calculate Card Metrics (shrink cards if height is small to save vertical space)
      let cardW = Math.round(w * 0.16 + 130);
      
      const heightFactor = Math.min(1.0, Math.max(0.65, h / 850));
      cardW = Math.round(cardW * heightFactor);
      
      cardW = Math.min(336, Math.max(150, cardW));
      const cardH = Math.round(cardW / 1.5925); // Standard credit card ratio

      setMetrics({ cardW, cardH });

      // 2. Calculate Typography Metrics (shrink font sizes aggressively if height or width is small)
      const isMobile = w < 640;
      
      let titleSize = '';
      let sigSize = '';
      let descSize = '';
      let titleGap = '40px'; 
      let plVal = '0px';

      if (isMobile) {
        // Mobile style: centered, text size increased by 30% for high legibility
        titleSize = 'clamp(1.8rem, 5.2vw + 0.4rem, 2.2rem)';
        sigSize = 'clamp(2.86rem, 7.8vw + 0.6rem, 3.5rem)';
        descSize = 'clamp(0.72rem, 1.4vw + 0.35rem, 0.95rem)';
        titleGap = '24px';
        plVal = '0px';
      } else {
        // Desktop / Tablet style: aligned bottom-left
        // Scale factor depends on width and height to shrink before hitting cards
        const scale = Math.min(1.0, Math.max(0.48, (w * 0.45 + h * 0.55) / 1300));
        
        titleSize = `${Math.max(1.15, 3.5 * scale).toFixed(3)}rem`;
        sigSize = `${Math.max(1.5, 4.5 * scale).toFixed(3)}rem`;
        descSize = `${Math.max(11, 16 * scale).toFixed(1)}px`;
        titleGap = `${Math.max(16, Math.round(40 * scale))}px`;
        plVal = `${Math.min(6, Math.max(2.8, 3.5 * scale + 2.2)).toFixed(2)}rem`;
      }

      setFontMetrics({
        titleFontSize: titleSize,
        sigFontSize: sigSize,
        descFontSize: descSize,
        titleGap,
        pl: plVal
      });
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Compute positions, rotations, and visual rules at 60fps
  const renderLoop = () => {
    // Upward flow speed of continuous transition - decreased speed by more than half for slower, premium, and calmer transitions
    progress.current += 0.0016; 

    // Smoothly interpolate current mouse variables towards their target positions (damping/inertia logic)
    mouse.current.x += (mouse.current.targetX - mouse.current.x) * 0.08;
    mouse.current.y += (mouse.current.targetY - mouse.current.y) * 0.08;

    const cards = cardsRefs.current;
    const h = window.innerHeight;
    const { cardH } = metrics;

    const continuousProgress = progress.current;
    const roundedIndex = Math.round(continuousProgress);
    const diffFromRound = continuousProgress - roundedIndex; // ranges between [-0.5, 0.5]
    
    // Custom non-linear magnetic step logic
    // It creates a gorgeous brief "dwell/pause" at front center before accelerating to the next card
    const easedDiff = Math.sign(diffFromRound) * Math.pow(Math.abs(diffFromRound) * 2, 4.2) / 2;
    const virtualActiveIndex = roundedIndex + easedDiff;

    for (let i = 0; i < cardCount; i++) {
      const card = cards[i];
      if (!card) continue;

      // Solve circular wrapping to get closest representation in [-cardCount/2, cardCount/2]
      let offset = i - virtualActiveIndex;
      const halfCount = cardCount / 2;
      while (offset > halfCount) offset -= cardCount;
      while (offset < -halfCount) offset += cardCount;

      const absOffset = Math.abs(offset);
      const sign = Math.sign(offset);

      // Allow cards to render completely off-screen smoothly up to offset 3.0. This prevents any clipping or sudden pop-outs.
      if (absOffset > 3.0) {
        card.style.visibility = 'hidden';
        continue;
      } else {
        card.style.visibility = 'visible';
      }

      // Spacing gap between center card and adjacent cards
      const gap = 36;
      const peekAmount = -55; // Push the card's edge 55px past the screen boundary to hide a premium portion of it!
      const D = 1350; // Perspective distance

      let y = 0;
      let z = 0;
      let rot = 0;

      if (absOffset <= 1) {
        // Smoothstep interpolation from 0 to 1 (Center card to first adjacent card)
        const t = absOffset;
        const easedT = t * t * (3 - 2 * t);

        // Y moves from 0 to (cardH + gap)
        const targetY = cardH + gap;
        y = -sign * (easedT * targetY);

        // Z moves from 400 (center) to 220 (adjacent)
        z = 400 + easedT * (220 - 400);

        // Rotation moves from 0 to 132 degrees (beautiful tilted back face)
        rot = easedT * 132;
      } else if (absOffset <= 2) {
        // Smoothstep interpolation from 1 to 2 (Adjacent card to peeking screen-edge card)
        const t = absOffset - 1;
        const easedT = t * t * (3 - 2 * t);

        const yStart = cardH + gap;
        const zStart = 220;
        const rotStart = 132;

        const zEnd = -60;
        const rotEnd = 175;

        // Perspective-aware formula for exact edge alignment at the screen boundary (peekAmount = 26px inside)
        const sEnd = D / (D - zEnd);
        const yEnd = (h / 2 - peekAmount) / sEnd - (cardH / 2);

        const currentY = yStart + easedT * (yEnd - yStart);
        y = -sign * currentY;

        z = zStart + easedT * (zEnd - zStart);
        rot = rotStart + easedT * (rotEnd - rotStart);
      } else {
        // Smoothstep interpolation from 2 to 3 (Peeking card to completely off-screen card)
        const t = Math.min(absOffset - 2, 1);
        const easedT = t * t * (3 - 2 * t);

        const zStart = -60;
        const rotStart = 175;

        const zEnd3 = -250;
        const rotEnd3 = 195;

        const sEnd2 = D / (D - zStart);
        const yEnd2 = (h / 2 - peekAmount) / sEnd2 - (cardH / 2);

        // Calculate yEnd3 dynamically so that the card's edge is completely 100px past the screen boundary
        const sEnd3 = D / (D - zEnd3);
        const yEnd3 = (h / 2 + 100) / sEnd3 + (cardH / 2);

        const currentY = yEnd2 + easedT * (yEnd3 - yEnd2);
        y = -sign * currentY;

        z = zStart + easedT * (zEnd3 - zStart);
        rot = rotStart + easedT * (rotEnd3 - rotStart);
      }

      const localCardRotation = -sign * rot;

      // Determine how close this card is to the exact center (1.0 = center, 0.0 = adjacent/offscreen)
      const centerFactor = Math.max(0, 1 - absOffset);

      // Vertical tilt (around X-axis) and horizontal tilt (around Y-axis) driven by mouse coordinates
      const maxTiltY = 15; // Max angle tilt left-to-right (degrees)
      const maxTiltX = 12; // Max angle tilt up-and-down (degrees)

      const activeTiltX = -mouse.current.y * maxTiltX * centerFactor;
      const activeTiltY = mouse.current.x * maxTiltY * centerFactor;

      const totalRotX = localCardRotation + activeTiltX;
      const totalRotY = activeTiltY;

      // Depth z-index layer
      card.style.zIndex = Math.round(z).toString();
      card.style.opacity = '1';

      // Inject translation matrix with the premium -3deg tilt combined with dynamic mouse-interactive 3D tilt
      card.style.transform = `translateY(${y.toFixed(2)}px) translateZ(${z.toFixed(2)}px) rotateX(${totalRotX.toFixed(2)}deg) rotateY(${totalRotY.toFixed(2)}deg) rotateZ(-3deg)`;
    }
  };

  useEffect(() => {
    const tick = () => {
      renderLoop();
      frameId.current = requestAnimationFrame(tick);
    };

    frameId.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frameId.current);
  }, [metrics]);

  // Slices for 3D volumetric depth with 30% reduced thickness
  // Span from -1.47px to 1.47px creates an extremely premium real 3D volume feel
  const thicknessLayers = [-1.47, -0.73, 0, 0.73, 1.47];

  return (
    <div className="absolute inset-0 bg-[#000000] text-white flex items-center justify-center overflow-hidden select-none">
      
      {/* 3D perspective camera space */}
      <div
        className="relative w-full h-full flex items-center justify-center pointer-events-none"
        style={{
          perspective: '1350px',
        }}
      >
        {/* Dynamic 3D coordinate viewport */}
        <div
          className="absolute"
          style={{
            width: `${metrics.cardW}px`,
            height: `${metrics.cardH}px`,
            transformStyle: 'preserve-3d',
          }}
        >
          {Array.from({ length: cardCount }).map((_, i) => (
            <div
              key={i}
              ref={(el) => { cardsRefs.current[i] = el; }}
              className="absolute inset-0"
              style={{
                width: `${metrics.cardW}px`,
                height: `${metrics.cardH}px`,
                transformStyle: 'preserve-3d',
                backfaceVisibility: 'visible',
              }}
            >
              {/* Build physical 3D volumetric thickness by dense parallel layering */}
              {thicknessLayers.map((zOffset, layerIdx) => {
                const isFrontFace = layerIdx === thicknessLayers.length - 1;
                const isBackFace = layerIdx === 0;

                const videoSrc = CARD_VIDEOS[i % CARD_VIDEOS.length];
                const baseBgColor = '#0f0f0f';

                // Middle structural slice
                if (!isFrontFace && !isBackFace) {
                  return (
                    <div
                      key={layerIdx}
                      className="absolute inset-0 rounded-[16px] border border-[#808080] pointer-events-none overflow-hidden"
                      style={{
                        backgroundColor: '#808080',
                        transform: `translateZ(${zOffset}px)`,
                      }}
                    />
                  );
                }

                // Front face slice
                if (isFrontFace) {
                  const frontBorderStyle = "border border-white/15";
                  return (
                    <div
                      key={layerIdx}
                      className={`absolute inset-0 rounded-[16px] ${frontBorderStyle} pointer-events-none overflow-hidden`}
                      style={{
                        backgroundColor: baseBgColor,
                        transform: `translateZ(${zOffset}px)`,
                        backfaceVisibility: 'hidden',
                        boxShadow: 'inset 0 1px 1px rgba(255,255,255,0.15)',
                      }}
                    >
                      <video
                        src={videoSrc}
                        autoPlay
                        loop
                        muted
                        playsInline
                        className="absolute inset-0 w-full h-full object-cover rounded-[16px]"
                      />

                      <div className="absolute inset-0 p-5 sm:p-6 text-white h-full w-full font-sans z-10 bg-black/15">
                        {/* Golden/Silver Metallic Contact Chip - positioned mid-left (vertically centered on the card) with custom user vectors */}
                        <div className="absolute left-5 sm:left-6 top-1/2 -translate-y-1/2">
                          <svg
                            className="w-6 h-6 sm:w-[29px] sm:h-[29px]"
                            viewBox="0 0 60 60"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                          >
                            <path
                              fillRule="evenodd"
                              clipRule="evenodd"
                              d="M20 8H40V14C40.0016 14.5299 40.2128 15.0377 40.5875 15.4125C40.9623 15.7872 41.4701 15.9984 42 16H59V24H42C41.4701 24.0016 40.9623 24.2128 40.5875 24.5875C40.2128 24.9623 40.0016 25.4701 40 26V52H20V8ZM18 8H8.00039C4.47435 8 1.56576 10.6083 1.08 14H18V8ZM1 16V24V26V34V36V44H18V36H1V34H18V26H1V24H18V16H1ZM1.08 46C1.56576 49.3917 4.47435 52 8.00039 52H18V46H1.08ZM42 14V8H52.0004C55.5264 8 58.4342 10.6084 58.92 14H42ZM59 26H42V34H59V26ZM59 36H42V44H59V36ZM52.0004 52H42V46H58.92C58.4342 49.3916 55.5264 52 52.0004 52Z"
                              fill={`url(#paint0_linear_1032_4_${i})`}
                            />
                            <path
                              fillRule="evenodd"
                              clipRule="evenodd"
                              d="M1.02453 14.4146C1.00608 14.609 0.998061 14.8045 1.00039 15C1.00039 14.8028 1.00854 14.6076 1.02453 14.4146ZM1.00039 45C0.998061 45.1955 1.00608 45.391 1.02453 45.5854C1.00854 45.3924 1.00039 45.1972 1.00039 45ZM59.0004 15C59.0026 14.8176 58.9955 14.6353 58.9794 14.4538C58.9933 14.634 59.0004 14.8162 59.0004 15ZM59.0004 45C59.0004 45.1838 58.9933 45.366 58.9794 45.5462C58.9955 45.3647 59.0026 45.1824 59.0004 45Z"
                              fill="#B7B7B7"
                            />
                            <defs>
                              <linearGradient
                                id={`paint0_linear_1032_4_${i}`}
                                x1="30"
                                y1="8"
                                x2="30"
                                y2="52"
                                gradientUnits="userSpaceOnUse"
                              >
                                <stop stopColor="white" />
                                <stop offset="1" stopColor="#999999" />
                              </linearGradient>
                            </defs>
                          </svg>
                        </div>

                        {/* JWT Brand Logo - positioned at top-right */}
                        <div className="absolute right-5 sm:right-6 top-5 sm:top-6 opacity-95">
                          <svg
                            className="w-[84px] xs:w-[101px] sm:w-[120px] h-auto"
                            viewBox="0 0 341 49"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                          >
                            <path
                              d="M8.75294 47.68C6.10761 47.68 4.10227 47.04 2.73694 45.76C1.41427 44.48 0.582275 42.7733 0.240941 40.64C-0.100392 38.464 -0.0790588 36.0747 0.304941 33.472C0.731608 30.8267 1.37161 28.1813 2.22494 25.536C3.07827 22.848 3.99561 20.3307 4.97694 17.984C6.00094 15.5947 6.93961 13.5893 7.79294 11.968C8.26227 11.072 8.88094 10.56 9.64894 10.432C10.4169 10.2613 11.1423 10.368 11.8249 10.752C12.5503 11.136 13.0623 11.6907 13.3609 12.416C13.7023 13.1413 13.6383 13.9307 13.1689 14.784C11.2916 18.368 9.79828 21.7813 8.68894 25.024C7.57961 28.2667 6.85427 31.1467 6.51294 33.664C6.21427 36.1387 6.23561 38.1013 6.57694 39.552C6.96094 40.96 7.68628 41.664 8.75294 41.664C9.73428 41.664 10.8009 41.3013 11.9529 40.576C13.1049 39.8507 14.3423 38.5493 15.6649 36.672C17.0303 34.6667 18.3529 32.064 19.6329 28.864C20.9556 25.6213 22.1289 21.8667 23.1529 17.6C23.4089 16.6187 23.8783 15.9573 24.5609 15.616C25.2863 15.2747 26.0329 15.2107 26.8009 15.424C27.5689 15.6373 28.1876 16.064 28.6569 16.704C29.1263 17.3013 29.2543 18.0693 29.0409 19.008C27.9316 23.616 27.3769 27.5627 27.3769 30.848C27.4196 34.1333 27.7609 36.5227 28.4009 38.016C28.8703 39.0827 29.4249 39.8507 30.0649 40.32C30.7476 40.7893 31.4943 41.024 32.3049 41.024C33.1156 41.024 33.9689 40.7253 34.8649 40.128C35.8036 39.488 36.7209 38.4 37.6169 36.864C38.5556 35.328 39.3876 33.216 40.1129 30.528C37.6809 28.48 35.6756 25.7707 34.0969 22.4C32.5183 19.0293 31.7289 15.168 31.7289 10.816C31.7289 8.93867 31.9423 7.21067 32.3689 5.632C32.7956 4.05333 33.5209 2.79467 34.5449 1.856C35.5689 0.874666 36.9769 0.383999 38.7689 0.383999C40.9449 0.383999 42.7156 1.17333 44.0809 2.752C45.4463 4.288 46.4489 6.37867 47.0889 9.024C47.7289 11.6267 48.0063 14.5493 47.9209 17.792C47.8783 21.0347 47.5369 24.3413 46.8969 27.712C47.5369 28.0107 48.2196 28.2453 48.9449 28.416C49.7129 28.5867 50.4809 28.672 51.2489 28.672C52.9983 28.672 54.7903 28.416 56.6249 27.904C58.5023 27.3493 60.1023 26.6453 61.4249 25.792C62.2783 25.2373 63.0676 25.088 63.7929 25.344C64.5183 25.5573 65.0943 26.0053 65.521 26.688C65.9476 27.328 66.1183 28.0533 66.0329 28.864C65.9903 29.632 65.5636 30.272 64.7529 30.784C62.8756 32.0213 60.7423 33.0027 58.3529 33.728C56.0063 34.4533 53.6383 34.816 51.2489 34.816C49.2863 34.816 47.3449 34.4533 45.4249 33.728C44.1876 37.7387 42.5023 40.96 40.3689 43.392C38.2356 45.824 35.5476 47.04 32.3049 47.04C30.2569 47.04 28.3583 46.4427 26.6089 45.248C24.9023 44.0107 23.6223 42.4107 22.7689 40.448C22.5983 40.064 22.4276 39.6587 22.2569 39.232C22.1289 38.8053 22.0223 38.4 21.9369 38.016C21.7236 38.4 21.4889 38.7627 21.2329 39.104C21.0196 39.4453 20.7849 39.7867 20.5289 40.128C18.9503 42.3467 17.1796 44.16 15.2169 45.568C13.2969 46.976 11.1423 47.68 8.75294 47.68ZM41.5849 23.104C42.0116 19.9893 42.1183 17.3653 41.9049 15.232C41.6916 13.0987 41.3503 11.392 40.8809 10.112C40.4116 8.78933 39.9423 7.85067 39.4729 7.296C39.0463 6.69867 38.8116 6.4 38.7689 6.4C38.7689 6.4 38.6836 6.42133 38.5129 6.464C38.3849 6.464 38.2356 6.76267 38.0649 7.36C37.9369 7.91467 37.8729 9.06667 37.8729 10.816C37.8729 12.992 38.1929 15.168 38.8329 17.344C39.4729 19.4773 40.3903 21.3973 41.5849 23.104Z"
439:                               fill="white"
440:                             />
441:                             <path
442:                               d="M91.5429 48.768C89.5376 48.768 87.9163 48.3627 86.6789 47.552C85.4843 46.784 84.6096 45.76 84.0549 44.48C83.5003 43.1573 83.2016 41.7493 83.1589 40.256C81.3243 42.4747 79.2763 44.224 77.0149 45.504C74.7963 46.7413 72.4709 47.36 70.0389 47.36C68.1189 47.36 66.3056 46.912 64.5989 46.016C62.8923 45.0773 61.5056 43.6907 60.4389 41.856C59.4149 39.9787 58.9029 37.6107 58.9029 34.752C58.9029 31.7653 59.5216 28.8427 60.7589 25.984C62.0389 23.0827 63.7669 20.48 65.9429 18.176C68.1616 15.8293 70.6789 13.9733 73.4949 12.608C76.3536 11.2 79.3403 10.496 82.4549 10.496C84.5029 10.496 86.5296 10.752 88.5349 11.264C90.5403 11.776 92.2896 12.5227 93.7829 13.504C94.6363 14.0587 95.1056 14.72 95.1909 15.488C95.2763 16.256 95.0843 16.9813 94.6149 17.664C94.1883 18.304 93.6123 18.752 92.8869 19.008C92.1616 19.264 91.3936 19.136 90.5829 18.624C89.7723 18.112 88.5563 17.6427 86.9349 17.216C85.3563 16.7467 83.8629 16.512 82.4549 16.512C80.0229 16.512 77.7616 17.0667 75.6709 18.176C73.5803 19.2853 71.7243 20.736 70.1029 22.528C68.5243 24.32 67.2869 26.2827 66.3909 28.416C65.4949 30.5493 65.0469 32.6613 65.0469 34.752C65.0469 35.8187 65.1749 36.864 65.4309 37.888C65.7296 38.8693 66.2416 39.7013 66.9669 40.384C67.6923 41.024 68.7163 41.344 70.0389 41.344C71.3189 41.344 72.7483 40.9173 74.3269 40.064C75.9483 39.168 77.4843 37.76 78.9349 35.84C79.8309 34.6453 80.7696 33.216 81.7509 31.552C82.7323 29.8453 83.6283 28.16 84.4389 26.496C85.2923 24.7893 85.9749 23.3387 86.4869 22.144C86.8283 21.2907 87.3403 20.736 88.0229 20.48C88.7483 20.224 89.4736 20.224 90.1989 20.48C90.9243 20.6933 91.5003 21.0987 91.9269 21.696C92.3536 22.2933 92.4816 23.04 92.3109 23.936L89.4949 37.632C89.1963 39.1253 89.1963 40.2347 89.4949 40.96C89.7936 41.6853 90.1776 42.176 90.6469 42.432C91.1163 42.6453 91.4149 42.752 91.5429 42.752C92.2256 42.752 93.1003 42.432 94.1669 41.792C95.2336 41.1093 96.5563 39.8507 98.1349 38.016C99.4576 36.5227 100.823 34.7733 102.231 32.768C103.682 30.72 105.068 28.6293 106.391 26.496C107.756 24.32 108.972 22.272 110.039 20.352C111.148 18.3893 112.023 16.768 112.663 15.488C113.09 14.592 113.687 14.0587 114.455 13.888C115.223 13.7173 115.948 13.824 116.631 14.208C117.356 14.5493 117.868 15.0827 118.167 15.808C118.508 16.4907 118.466 17.28 118.039 18.176C117.356 19.584 116.439 21.2907 115.287 23.296C114.178 25.3013 112.919 27.4347 111.511 29.696C110.146 31.9147 108.695 34.0907 107.159 36.224C105.666 38.3573 104.194 40.2773 102.743 41.984C101.036 43.9467 99.2869 45.568 97.4949 46.848C95.7456 48.128 93.7616 48.768 91.5429 48.768Z"
443:                               fill="white"
444:                             />
445:                             <path
446:                               d="M118.45 48.448C115.549 48.448 113.351 47.6373 111.858 46.016C110.407 44.352 109.533 42.0267 109.234 39.04C108.978 36.0533 109.17 32.5547 109.81 28.544C110.493 24.5333 111.517 20.16 112.882 15.424C113.181 14.4427 113.693 13.8027 114.418 13.504C115.143 13.1627 115.89 13.12 116.658 13.376C117.426 13.632 118.023 14.08 118.45 14.72C118.919 15.36 119.026 16.1493 118.77 17.088C117.191 22.464 116.146 26.8373 115.634 30.208C115.165 33.536 115.037 36.096 115.25 37.888C115.463 39.6373 115.869 40.832 116.466 41.472C117.106 42.112 117.767 42.432 118.45 42.432C119.303 42.432 120.413 41.9413 121.778 40.96C123.143 39.936 124.594 38.5067 126.13 36.672C127.666 34.8373 129.138 32.7253 130.546 30.336C129.778 27.904 129.394 25.152 129.394 22.08C129.394 20.2027 129.501 18.176 129.714 16C129.97 13.7813 130.397 11.6907 130.994 9.728C131.634 7.76533 132.509 6.18667 133.618 4.992C134.77 3.79733 136.242 3.264 138.034 3.392C139.485 3.52 140.573 4.032 141.298 4.928C142.066 5.824 142.535 6.95467 142.706 8.32C142.919 9.68533 142.941 11.1573 142.77 12.736C142.599 14.272 142.343 15.808 142.002 17.344C141.661 18.8373 141.319 20.16 140.978 21.312C139.954 24.8107 138.781 28.032 137.458 30.976C138.61 33.024 140.061 34.432 141.81 35.2C143.559 35.968 145.33 36.2453 147.122 36.032C148.914 35.776 150.45 35.2427 151.73 34.432C152.583 33.8773 153.373 33.728 154.098 33.984C154.823 34.1973 155.399 34.6453 155.826 35.328C156.295 35.968 156.487 36.6933 156.402 37.504C156.317 38.272 155.869 38.912 155.058 39.424C152.967 40.7893 150.642 41.6427 148.082 41.984C145.565 42.3253 143.09 42.0907 140.658 41.28C138.226 40.4693 136.093 39.04 134.258 36.992C132.039 40.576 129.586 43.392 126.898 45.44C124.253 47.4453 121.437 48.448 118.45 48.448ZM135.666 18.112C136.391 15.5947 136.882 13.7173 137.138 12.48C137.394 11.2427 137.522 10.432 137.522 10.048C137.522 9.62133 137.522 9.408 137.522 9.408C137.522 9.408 137.394 9.68533 137.138 10.24C136.882 10.752 136.605 11.648 136.306 12.928C136.007 14.1653 135.794 15.8933 135.666 18.112Z"
447:                               fill="white"
448:                             />
449:                             <path
450:                               d="M164.834 48.512C161.762 48.512 159.117 47.808 156.898 46.4C154.68 44.9493 152.973 43.008 151.778 40.576C150.584 38.1013 149.986 35.328 149.986 32.256C149.986 29.2267 150.562 26.3893 151.714 23.744C152.866 21.056 154.36 18.7093 156.194 16.704C158.072 14.656 160.056 13.0773 162.146 11.968C164.28 10.816 166.306 10.24 168.226 10.24C169.762 10.24 171.17 10.5387 172.45 11.136C173.73 11.7333 174.754 12.5867 175.522 13.696C176.333 14.8053 176.738 16.1493 176.738 17.728C176.738 20.0747 176.034 22.1227 174.626 23.872C173.261 25.5787 171.384 27.4773 168.994 29.568C167.202 31.1467 165.325 32.64 163.362 34.048C161.4 35.456 159.352 36.8427 157.218 38.208C158.584 41.0667 161.122 42.496 164.834 42.496C165.858 42.496 166.946 42.3467 168.098 42.048C169.25 41.7067 170.552 41.024 172.002 40C173.453 38.976 175.16 37.376 177.122 35.2C177.762 34.4747 178.466 34.1333 179.234 34.176C180.045 34.2187 180.749 34.5173 181.346 35.072C181.944 35.584 182.285 36.2453 182.37 37.056C182.498 37.824 182.242 38.5707 181.602 39.296C178.445 42.7947 175.458 45.2053 172.642 46.528C169.869 47.8507 167.266 48.512 164.834 48.512ZM156.13 31.744C157.752 30.6773 159.309 29.6107 160.802 28.544C162.296 27.4347 163.704 26.2827 165.026 25.088C167.245 23.1253 168.738 21.504 169.506 20.224C170.317 18.9013 170.722 18.0693 170.722 17.728C170.722 17.5573 170.594 17.28 170.338 16.896C170.082 16.4693 169.378 16.256 168.226 16.256C167.16 16.256 165.944 16.6613 164.578 17.472C163.256 18.24 161.954 19.328 160.674 20.736C159.437 22.144 158.392 23.7867 157.538 25.664C156.685 27.5413 156.216 29.568 156.13 31.744Z"
451:                               fill="white"
452:                             />
453:                             <path
454:                               d="M201.487 13.248C204.773 13.248 207.717 13.9733 210.319 15.424C212.922 16.8747 214.949 18.9013 216.399 21.504C217.893 24.1067 218.639 27.1147 218.639 30.528C218.639 33.9413 217.893 36.9707 216.399 39.616C214.949 42.2187 212.922 44.2453 210.319 45.696C207.717 47.1467 204.773 47.872 201.487 47.872C198.97 47.872 196.666 47.3813 194.575 46.4C192.485 45.4187 190.757 43.9893 189.391 42.112V47.488H183.503V0H189.647V18.688C191.013 16.896 192.719 15.552 194.767 14.656C196.815 13.7173 199.055 13.248 201.487 13.248ZM200.975 42.496C203.151 42.496 205.093 42.0053 206.799 41.024C208.549 40 209.914 38.592 210.895 36.8C211.919 34.9653 212.431 32.8747 212.431 30.528C212.431 28.1813 211.919 26.112 210.895 24.32C209.914 22.4853 208.549 21.0773 206.799 20.096C205.093 19.1147 203.151 18.624 200.975 18.624C198.842 18.624 196.901 19.1147 195.151 20.096C193.402 21.0773 192.037 22.4853 191.055 24.32C190.074 26.112 189.583 28.1813 189.583 30.528C189.583 32.8747 190.074 34.9653 191.055 36.8C192.037 38.592 193.402 40 195.151 41.024C196.901 42.0053 198.842 42.496 200.975 42.496Z"
455:                               fill="white"
456:                             />
457:                             <path
458:                               d="M256.568 13.568V47.488H250.68V42.112C249.315 43.9893 247.587 45.4187 245.496 46.4C243.406 47.3813 241.102 47.872 238.584 47.872C235.299 47.872 232.355 47.1467 229.752 45.696C227.15 44.2453 225.102 42.2187 223.608 39.616C222.158 36.9707 221.432 33.9413 221.432 30.528C221.432 27.1147 222.158 24.1067 223.608 21.504C225.102 18.9013 227.15 16.8747 229.752 15.424C232.355 13.9733 235.299 13.248 238.584 13.248C241.016 13.248 243.256 13.7173 245.304 14.656C247.352 15.552 249.059 16.896 250.424 18.688V13.568H256.568ZM239.096 42.496C241.23 42.496 243.171 42.0053 244.92 41.024C246.67 40 248.035 38.592 249.016 36.8C249.998 34.9653 250.488 32.8747 250.488 30.528C250.488 28.1813 249.998 26.112 249.016 24.32C248.035 22.4853 246.67 21.0773 244.92 20.096C243.171 19.1147 241.23 18.624 239.096 18.624C236.92 18.624 234.958 19.1147 233.208 20.096C231.502 21.0773 230.136 22.4853 229.112 24.32C228.131 26.112 227.64 28.1813 227.64 30.528C227.64 32.8747 228.131 34.9653 229.112 36.8C230.136 38.592 231.502 40 233.208 41.024C234.958 42.0053 236.92 42.496 239.096 42.496Z"
459:                               fill="white"
460:                             />
461:                             <path
462:                               d="M283.745 13.248C288.055 13.248 291.468 14.5067 293.985 17.024C296.545 19.4987 297.825 23.1467 297.825 27.968V47.488H291.681V28.672C291.681 25.3867 290.892 22.912 289.313 21.248C287.735 19.584 285.473 18.752 282.529 18.752C279.201 18.752 276.577 19.7333 274.657 21.696C272.737 23.616 271.777 26.3893 271.777 30.016V47.488H265.633V13.568H271.521V18.688C272.759 16.9387 274.423 15.5947 276.513 14.656C278.647 13.7173 281.057 13.248 283.745 13.248ZM319.82 31.68L312.78 38.208V47.488H306.636V0H312.78V30.464L331.276 13.568H338.7L324.428 27.584L340.108 47.488H332.556L319.82 31.68Z"
463:                               fill="white"
464:                             />
465:                           </svg>
466:                         </div>
467: 
468:                         {/* Double intersecting circle Brand Logo - bottom right corner */}
469:                         <div className="absolute right-5 sm:right-6 bottom-5 sm:bottom-6 flex -space-x-3 items-center opacity-90">
470:                           <div className="w-5 h-5 sm:w-6 sm:h-6 rounded-full bg-white/20 backdrop-blur-[1px] border border-white/10" />
471:                           <div className="w-5 h-5 sm:w-6 sm:h-6 rounded-full bg-white/35 backdrop-blur-[1px] border border-white/10" />
472:                         </div>
473:                       </div>
474:                     </div>
475:                   );
476:                 }
477: 
478:                 // Back face slice
479:                 if (isBackFace) {
480:                   const backBorderStyle = "border border-white/15";
481:                   const details = CARD_DETAILS[i % CARD_DETAILS.length];
482:                   return (
483:                     <div
484:                       key={layerIdx}
485:                       className={`absolute inset-0 rounded-[16px] ${backBorderStyle} pointer-events-none overflow-hidden`}
486:                       style={{
487:                         backgroundColor: baseBgColor,
488:                         transform: `translateZ(${zOffset}px) rotateX(180deg)`,
489:                         backfaceVisibility: 'hidden',
490:                         boxShadow: 'inset 0 1px 1px rgba(255,255,255,0.15)',
491:                       }}
492:                     >
493:                       {/* Render Video with premium 16px blur on the back face of the card */}
494:                       <div className="absolute inset-0 pointer-events-none" style={{ filter: 'blur(16px)', transform: 'scale(1.15)' }}>
495:                         <video
496:                           src={videoSrc}
497:                           autoPlay
498:                           loop
499:                           muted
500:                           playsInline
501:                           className="absolute inset-0 w-full h-full object-cover"
502:                         />
503:                       </div>
504: 
505:                       {/* Premium Real Magnetic stripe */}
506:                       <div className="absolute left-0 right-0 top-4 sm:top-5 h-7 sm:h-9 bg-black/85 backdrop-blur-md z-10" />
507: 
508:                       {/* Card holder info and details on the bottom-left */}
509:                       <div 
510:                         className="absolute left-4 sm:left-6 bottom-4 sm:bottom-5 z-20 flex flex-col gap-0.5 sm:gap-1 text-left"
511:                         style={{ fontFamily: '"JetBrains Mono", monospace' }}
512:                       >
513:                         {/* Card Number */}
514:                         <div className="font-mono text-[10px] sm:text-[12px] font-medium tracking-[0.14em] text-white select-none">
515:                           {details.number}
516:                         </div>
517:                         {/* Owner & CVV */}
518:                         <div className="font-mono text-[7px] sm:text-[9px] font-medium text-white/70 tracking-wide flex items-center gap-2 select-none">
519:                           <span className="uppercase">{details.name}</span>
520:                           <span className="text-white/40 font-light">•</span>
521:                           <span>CVV: {details.cvv}</span>
522:                         </div>
523:                       </div>
524:                     </div>
525:                   );
526:                 }
527: 
528:                 return null;
529:               })}
530:             </div>
531:           ))}
532:         </div>
533:       </div>
534:     </div>
535:   );
536: }

## Build With Us — Contact us [sites/build-with-us]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(45).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/build-with-us.webp

Build a single-page React + TypeScript + Vite + Tailwind site that is a full-screen video-background landing page with a contact form. Use `lucide-react` for icons.

**Layout & Sizing**
- Root: `min-h-screen` white background with padding `p-3 sm:p-4 md:p-6`.
- Inside the root, one large rounded card with `rounded-2xl sm:rounded-3xl`, `overflow-hidden`. Heights: `min-h-[calc(100vh-24px)] sm:min-h-[calc(100vh-32px)] md:min-h-[calc(100vh-48px)] lg:h-[calc(100vh-48px)]`. On desktop it locks to viewport; on tablet/mobile it expands to content.
- Background video fills the card (`absolute inset-0 w-full h-full object-cover`). The video element has `autoPlay muted loop playsInline`. Use this exact URL:
  ```
  https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260602_150901_c45b90ec-18d7-42ff-90e2-b95d7109e330.mp4
  ```
- Content layer: `relative z-10 flex flex-col` with the same min-height ladder as the card and `lg:h-full`, padding `p-4 sm:p-6 md:p-8`, `gap-6`.

**Fonts**
- Import from Google Fonts in `index.css`: `Inter` (weights 300–700) and `Instrument Serif` (italic + regular).
- Set `* { font-family: 'Inter', sans-serif; }` globally.
- Use `Instrument Serif` italic for one accent word inline (see headline below).

**Navbar (top)**
- Pill bar with `bg-white/60 backdrop-blur-md rounded-2xl shadow-sm`, padding `pl-3 sm:pl-4 pr-2 py-2`, `w-full sm:w-auto`, `flex items-center gap-3 sm:gap-6`.
- Logo: 32x32 inline SVG (`viewBox="0 0 256 256"`) with two black filled paths forming a stylized "M":
  `M 256 256 L 128 256 L 0 128 L 128 128 Z M 256 128 L 128 128 L 0 0 L 128 0 Z`.
- Links (hidden on mobile, shown `sm:flex`): `Our story`, `Expertise`, `Our work`, `Journal` — class `text-gray-800 text-sm font-medium hover:opacity-60 transition-opacity whitespace-nowrap`.
- CTA button on the right: black pill `bg-black text-white text-sm font-medium px-4 sm:px-5 py-2 rounded-xl hover:bg-gray-800` with label `Start a project`. On mobile it floats right with `ml-auto`.

**Spacer**
- A `<div className="flex-1 min-h-[2rem]" />` between nav and the bottom row.

**Bottom row (headline + form)**
- Container: `flex flex-col lg:flex-row lg:items-end lg:justify-between gap-6`.

**Headline (left)**
- `<p>` with white text, `text-3xl sm:text-4xl xl:text-5xl font-medium leading-tight drop-shadow-lg lg:max-w-lg xl:max-w-2xl shrink-0`.
- Content (with `<br />`):
  `We craft bold ideas` / `and ship them as *products*`
- The word `products` is wrapped in a `<span>` with inline style: `fontFamily: "'Instrument Serif', serif"`, `fontStyle: 'italic'`, `fontWeight: 400`.

**Contact form card (right)**
- Outer: `w-full lg:w-[min(480px,45%)] shrink-0`.
- Card: `bg-white rounded-2xl sm:rounded-3xl shadow-2xl overflow-hidden`, inner padding `p-4 sm:p-6`, `flex flex-col gap-4`.

1. **Heading:** `Say hello! 👋` — `text-xl sm:text-2xl font-semibold text-black tracking-tight`.

2. **Email + socials row** (always horizontal): `flex flex-row items-center justify-between gap-3 bg-gray-50 rounded-2xl px-4 py-2.5`.
   - Left: small grey label `Drop us a line`, then mailto link `hello@forma.co` in `text-blue-600 font-semibold hover:underline truncate`.
   - Right: four 32x32 rounded-xl buttons (`w-8 h-8 rounded-xl flex items-center justify-center hover:opacity-80 transition-opacity`) using lucide icons size 13:
     - Twitter — `bg-gray-100 text-gray-800`
     - Circle — `bg-pink-100 text-pink-500`
     - Instagram — `bg-orange-100 text-orange-400`
     - Linkedin — `bg-blue-100 text-blue-600`
   - Extract this into a small `SocialBtn` helper component.

3. **OR divider:** horizontal lines on either side of the word `OR` (`text-gray-400 font-medium text-sm`, lines `flex-1 h-px bg-gray-200`).

4. **Form** (`flex flex-col gap-4`):
   - Label `Tell us about your vision` (`text-sm font-medium text-black`).
   - Name + Email inputs side by side on `sm:` (`flex flex-col sm:flex-row gap-2`), placeholders `Full name` and `Email`. Input style: `flex-1 min-w-0 text-sm px-3 py-2.5 rounded-xl border border-gray-200 bg-transparent placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent transition`.
   - Textarea, 4 rows, placeholder `What are you looking to build or improve...`, same input style plus `resize-none`.
   - Service tags section: label `I need help with...`. Tags wrap (`flex flex-wrap gap-1.5`). Each tag is a button `text-xs font-medium px-3 py-2 rounded-lg border transition-all`. Inactive: `bg-white text-gray-700 border-gray-200 hover:border-gray-400`. Active (selected): `bg-gray-100 text-black border-black`. Multi-select toggle via state.
     - Services list (exact order): `Website`, `Mobile App`, `Web App`, `E-Commerce`, `Visual Identity`, `3D & Motion`, `Digital Marketing`, `Growth & Consulting`, `Other`.
   - Submit button: `w-full bg-black text-white text-sm font-semibold py-3 rounded-2xl hover:bg-gray-800 transition-colors disabled:opacity-60`. Label: `Send my message` (or `Sending...` while submitting).

5. **Submit behavior:** On submit, set `sending=true`, await a 1-second fake delay (`new Promise(r => setTimeout(r, 1000))`), then show a success state in place of the form: centered column with `py-6 gap-3`, a 48x48 green check pill (`w-12 h-12 rounded-full bg-green-50 flex items-center justify-center text-xl` containing `✓`), heading `You're all set!` (`text-base font-semibold text-gray-900`), and subtext `Expect a reply within 24 hours.` (`text-sm text-gray-500`).

**State (useState)**
- `selected: string[]` (toggled service chips)
- `name`, `email`, `message`: strings
- `sending`, `sent`: booleans

**Transitions/animations**
- All interactive elements use Tailwind `transition-*` utilities (opacity, colors, all).
- No external animation library; rely on Tailwind hover/focus transitions and `backdrop-blur-md` on the navbar.

**Constants at the top of the file**
- `VIDEO_URL` (the CloudFront URL above) and `SERVICES` array.

**Files**
- `src/App.tsx` — entire component plus `SocialBtn` helper.
- `src/index.css` — Google Fonts import + Tailwind directives + global `* { font-family: 'Inter', sans-serif; }`.
- Standard Vite + Tailwind config (`tailwind.config.js` scanning `./index.html` and `./src/**/*.{ts,tsx}`).

## Editorial Collection CTA — CTA [sites/editorial-collection-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(70).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/editorial-collection-cta.webp

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

## FAQ CTA — CTA [sites/faq-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(61).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/faq-cta.webp

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

## Global CTA Footer — CTA [sites/global-cta-footer]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(64).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/global-cta-footer.webp

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

## Liquid Glass CTA — CTA [sites/liquid-glass-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(17).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/liquid-glass-cta.webp

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

## Mouse Trail CTA — CTA [sites/mouse-trail-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(46).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/mouse-trail-cta.webp

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

## Nimbus Ops — CTA [sites/nimbus-ops]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(27).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-ops.webp

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

## Orbis CTA — CTA [sites/orbis-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(19).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbis-cta.webp

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

## Rocket CTA — CTA [sites/rocket-cta]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(69).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/rocket-cta.webp

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

## Community CTA — CTA Section [sites/community-cta]

- Preview: https://motionsites.ai/assets/cta-community-preview-C90X-RHI.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/community-cta.gif

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

## Dashboard UI — Dashboard [sites/dashboard]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(63).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/dashboard.webp

Build a premium **Conference Dashboard** in **React + TypeScript + Vite + Tailwind** with a **liquid glassmorphism** aesthetic. Use **lucide-react** for icons. Use the **Inter** font family (weights 300, 400, 500, 600, 700) loaded from Google Fonts.

### Background

Use two looping fullscreen background videos (autoplay, muted, loop, playsInline, object-fit: cover, fixed inset, z-index -1) — swap them based on dark/light theme. **No overlays.**

- Light mode video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_103318_2aa26b55-df1a-43a6-903d-941e718c9366.mp4`
- Dark mode video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_102933_4e8f73b5-775a-4179-b2fb-472f59063dcd.mp4`

### CSS Variables (`:root`)

```
--glass-bg: rgba(255, 255, 255, 0.55);
--glass-border: rgba(255, 255, 255, 0.6);
--glass-blur: 8px;
--text-main: #1a1a1a;
--text-muted: #6b7280;
--accent: #000000;
--card-radius: 40px;
--transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1);
```

In `.dark-mode`: `--glass-bg: rgba(0,0,0,0.45)`; `--glass-border: rgba(255,255,255,0.08)`; `--text-main:#fff`; `--text-muted:#b0b0b0`; `--accent:#fff`.

Body: Inter font, `height:100vh`, `padding:32px 40px`, `display:flex; flex-direction:column; overflow:hidden`, black fallback bg.

### Top Navigation (grid: auto auto 1fr auto auto, gap 16px, mb 40px)

1. **Profile button** — 48×48 circular avatar `https://i.pravatar.cc/100?u=current_user`.
2. **Toggle container** — pill containing:
   - **Mode switch** (88×48, white pill, inner blue track 76×40 `#3b82f6`, white 32×32 handle on right; in dark mode handle slides left via `transform: translateX(-36px)`; small icon `☾`/`☀` slides via `translateX(42px)`).
   - **Settings nav-btn** — pill, 10px 24px, `rgba(0,0,0,0.04)` bg with blur, white text.
3. **Meeting alert** (justify-self center) — white pill, `padding 6px 6px 6px 16px`, gap 12, shadow `0 4px 20px rgba(0,0,0,0.08)`. Contains: 32px host avatar `https://i.pravatar.cc/100?u=meeting_host`, label "Meeting is about to start", grey time-tag pill "-5:23" (`#f0f0f0`, 4px 10px), and a 32×32 close button with an SVG progress ring (gray track + black arc, `stroke-dasharray=88 stroke-dashoffset=25 rotate(-90)`) and a centered Lucide `X` (12px). Hidden on mobile.
4. **View switcher** — pill, `rgba(0,0,0,0.04)` bg, 4px padding, two buttons "Dashboard" and "Rooms"; active = white bg, black text, shadow `0 4px 12px rgba(0,0,0,0.1)`. Default active = "Rooms".
5. **Search button** — 48×48 circular, Lucide `Search`.

### Dashboard Grid (4 cols × 2 rows, 24px gap, max-width 1400px, fills available height)

Card base: `padding:28px 20px`, flex column, `border-radius:40px`, hover `translateY(-3px) scale(1.01)`.

### Card 1 — Empty / Create Room (glass)
- Translucent dark glass `rgba(0,0,0,0.18)` (light), `rgba(255,255,255,0.08)` (dark).
- Centered Lucide `Plus` (32px) + label "Create a room", white text.

### Card 2 — Subscription Growth Experiments (solid white)
- Title: "Subscription Growth Experiments" (1.35rem, weight 400, letter-spacing -0.03em).
- Subtitle: "Sprint Retrospective".
- Header icon: Lucide `Zap` (16px, opacity 0.5).
- Footer: 3 overlapping 32px avatars (pravatar `u=1,2,3`, `margin-left:-12px`) + count badge "9" (38×38 circle, `rgba(0,0,0,0.08)`).

### Card 3 — Weekly Insights (solid white)
- Title only: "Weekly Insights".
- **Bar chart** (height 60px, `gap:2px`, `align-items:flex-end`):
  - First **24** bars are blue `#3b82f6` with heights: `35,45,30,55,40,65,50,75,60,85,70,80,65,55,45,70,60,75,55,65,50,75,60,55`.
  - Next **36** bars grey `#e5e7eb` with heights: `45,70,60,75,55,65,50,75,60,85,70,55,45,70,60,75,55,65,50,75,60,55,45,70,60,75,55,65,50,75,60,55,45,70,60,75`.
- **Chart markers row** (`justify-content:space-between; padding:0 20px; margin-bottom:24px`): single 18px avatar `u=m1`, then group of two `u=m2`(margin-right -8) + `u=m3`, then group `u=m4`(-8) + `u=m5`. All have 1.5px white border.
- Footer: two overlapping avatars `u=large1`, `u=large2`, plus 54×54 white play button (`rgba(245,245,245,0.85)`) with Lucide `Play` (20px, fill black).

### Card 4 — Product Strategy 2023 (glass, dark translucent)
- Title "Product Strategy 2023" + subtitle "No upcoming meetings".
- Header icon: Lucide `MoreHorizontal` (16px, opacity 0.5).
- Footer: single 32px avatar `u=6` + count badge "32".

### Card 5 — User Onboarding Team (solid white)
- Title "User Onboarding Team" + subtitle "Sprint Planning".
- Header icon: Lucide `BarChart2`.
- Footer: 3 overlap avatars `u=7,8,9` + badge "3".

### Card 6 — User & Market Research (glass)
- Title "User & Market Research" + subtitle "No upcoming meetings".
- Icon: `MoreHorizontal`. Footer: avatar `u=10` + badge "6".

### Card 7 — Core Product Team (solid white)
- Title and subtitle both "Core Product Team".
- Icon: Lucide `Video`. Footer: 2 overlap avatars `u=11,12` + badge "2".

### Card 8 — Screen Share (solid card-alt; gradient `linear-gradient(to bottom,#f4f4f4 0%, #ffffff 50%, #ffffff 100%)`)
- Header row of two pill chips (justify start, gap 8): "Screen Share" (blue text `#3b82f6`) and "0:30" (black text). Both white pills, `padding:6px 14px; font-size:0.75rem; box-shadow:0 2px 8px rgba(0,0,0,0.06)`.
- Horizontal scroll row (`overflow-x:auto; gap:12px; margin: 20px -20px 0; padding:0 20px 16px; hide scrollbar; cursor:grab; drag-to-scroll`):
  - 4 thumbnails 160×100, `border-radius:16px`, backgrounds `https://picsum.photos/seed/screen1..4/300/200`.
  - On thumbnail #2: bottom-right floating tag with 24px avatar `u=alice_av` + orange `#e05e36` pill labeled "Alice" (white text 0.65rem, 2px 8px, radius 100).
- Footer: 2 avatars `u=13,14` + badge "8" (background `#F3F3F3`).

### Indicators (under grid)

Three 12×12 dots, white, gap 16, `margin: 24px 0 120px`. First dot active (opacity 1); others opacity 0.3.

### Bottom Bar (fixed, centered, glass pill)

`bottom:32px; left:50%; translateX(-50%); padding:10px 16px; border-radius:100px`.
Contains active-participants row:
- **Active speaker** 44×44 circle `u=speaker` with **voice indicator** badge (top-right -2/-2): white 18×18 circle with shadow containing **3 wave bars** (2px wide, grey `#4b5563`, animated via `@keyframes voice-wave` between 4px and 10px height, 1s ease-in-out infinite, delays 0/0.2s/0.4s).
- 40×40 participant `u=p1` (opacity 0.7).
- 40×40 participant `u=p2` with another voice indicator.
- 40×40 participant `u=p3`.
- "+17" 40×40 round chip `rgba(255,255,255,0.25)`, white bold.

### Components button (fixed bottom-left, 32px from edges)

44×44 rounded-rect (radius 14, `rgba(0,0,0,0.04)` blur), 2×2 grid of 4 small avatars `u=c1..c4`.

### Floating Controls (fixed bottom-right, 32px)

Pill `rgba(0,0,0,0.04)`, padding 10px 14px, gap 12. Two 44×44 round buttons:
- Video toggle: Lucide `Video` ↔ `VideoOff`. When off, bg `#ff4545`, white icon.
- Mic toggle: Lucide `Mic` ↔ `MicOff`. When muted, bg `#ff4545`.
Hover: `scale(1.08)`.

### Glass Utility

```
.glass {
  background: var(--glass-bg);
  backdrop-filter: blur(8px) saturate(1.8);
  border-radius: 40px;
  box-shadow: 0 4px 24px rgba(0,0,0,0.06), 0 1px 4px rgba(0,0,0,0.04), inset 0 1px 0 rgba(255,255,255,0.5);
}
.glass::after { content:''; position:absolute; inset:0; border-radius:inherit; pointer-events:none; filter:url(#noise-filter); opacity:0.06; mix-blend-mode:overlay; }
```

Inline an SVG `<filter id="noise-filter">` using `feTurbulence baseFrequency=0.65 numOctaves=3 stitchTiles=stitch` + `feComposite operator=in in2=SourceGraphic` for the grain texture.

### Solid Card

`background:#fff; box-shadow:0 4px 20px rgba(0,0,0,0.03), 0 1px 3px rgba(0,0,0,0.01)`. In dark: `rgba(26,26,26,0.98)` with white text.

### Animations / Transitions

- All interactive elements: `transition: all 0.4s cubic-bezier(0.22, 1, 0.36, 1)`.
- Card hover: `translateY(-3px) scale(1.01)` + larger shadow.
- Theme-switch handle: `transform 0.4s cubic-bezier(0.4, 0, 0.2, 1)`.
- Voice waves: `voice-wave` keyframes (height 4px → 10px → 4px), 3 staggered bars.
- Pulse-red keyframe available for emergencies (red ring expand-fade).

### State / Interactions

- `isDark` toggles `body.dark-mode` and swaps the background `<video>` (use `key` to force reload).
- View switcher toggles active button.
- Mic/video buttons toggle `muted`/`off` class swapping icons via lucide-react.
- Screen-share strip supports mouse drag-to-scroll (mousedown/move/up/leave).

### Responsive

- ≤1200px: grid → 2 columns, rows 280px, body becomes scrollable.
- ≤768px: grid → 1 column, padding 16px, hide meeting alert, view-switcher full-width on second row, bottom bar near full width, floating controls + components button move up to bottom 80px.

### Persistence

A Supabase database is available; no specific data persistence is required for this purely visual dashboard, but if needed wire it via `@supabase/supabase-js` using `VITE_SUPABASE_URL` / `VITE_SUPABASE_ANON_KEY` from `.env`.

### Color Rules

Avoid purple/indigo. Palette: blue accent `#3b82f6`, neutral whites/blacks/greys, alert red `#ff4545`, orange tag `#e05e36`. All text contrast-safe in both themes.

## Modern HR Dashboard — Dashboard [sites/modern-hr-dashboard]

- Preview: https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779784289/CleanShot_2026-05-26_at_15.26.21_2x_f2ytq9.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/modern-hr-dashboard.png

**Build an HR dashboard called "Talvex" using React, TypeScript, Vite, Tailwind CSS, Recharts, and Lucide React icons. The page must be fully responsive across mobile, tablet, and desktop. Here is every specification:**

---

### Font

Use "Sofia Pro Regular" loaded from this external stylesheet in `index.html`:
```
https://db.onlinewebfonts.com/c/060e116a70e3096c52db16f61aaab194?family=Sofia+Pro+Regular
```
Set `font-family: "Sofia Pro Medium", sans-serif` on all elements via CSS `*` selector with `-webkit-font-smoothing: antialiased`. The root `<div>` inline style uses `"Sofia Pro Regular", sans-serif`.

---

### Color Palette

- Yellow accent: `#FFD85F`
- Dark gray (text, dark fills): `#303030`
- Light gray (secondary text, borders): `#898989`
- Card backgrounds: `white/60` with `backdrop-blur-3xl`
- Card shadows: `0 2px 20px rgba(0,0,0,0.06)`
- Profile photo card shadow: `0 2px 20px rgba(0,0,0,0.10)`

---

### Background

A full-screen fixed SVG background covering the viewport with `preserveAspectRatio="xMidYMid slice"`. Viewbox is `0 0 1280 832`.

- Base: a rect filling `1280x832` with `#E3E5E6`
- On top: a large yellow `#FFD85F` blob path with a heavy Gaussian blur (`stdDeviation="250"`), creating a warm diffused glow in the lower portion:
```
M904 404C942.8 189.6 1234.83 123.333 1376 117V1093.5H-227V792.5C-161.5 706.167 0.5 556.6 124.5 649C248.5 741.4 473.833 727.5 571 709C665.833 696.667 865.2 618.4 904 404Z
```
The filter uses `feFlood` -> `feBlend` -> `feGaussianBlur` with filter region `x="-727" y="-383" width="2603" height="1976.5"`.

---

### Layout Structure

Two layout containers layered on top of the SVG (both `relative z-10`, `max-w-[1400px] mx-auto`):

1. **Desktop (lg+):** `hidden lg:flex`, `h-screen`, `px-6 py-6`, `flex-col overflow-hidden`. The dashboard grid area is `flex-1 min-h-0`.
2. **Mobile/Tablet (<lg):** `lg:hidden`, `min-h-screen`, `px-4 sm:px-6 py-6`, `flex flex-col gap-0 overflow-y-auto`, with a `h-6` spacer at the bottom.

Both contain: Navbar, WelcomeRow, DashboardGrid (in that order).

---

### Navbar

Full-width nav with `mb-4`.

**Left:** "Talvex" text in a pill — `border border-[#898989]/30 rounded-full px-5 py-2 text-[#303030] text-base select-none`.

**Right (flex items-center gap-2):**

1. **Nav links (desktop only, hidden lg:flex):** Pill container `bg-white/60 border border-[#898989]/20 rounded-full px-1 py-1 shadow-sm`, containing buttons for: `Dashboard, People, Hiring, Devices, Apps, Salary, Calendar, Reviews`. Each button: `px-4 py-2 rounded-full text-sm transition-all duration-200`. Active state: `bg-[#303030] text-white`. Inactive: `text-[#898989] hover:text-[#303030]`. Default active = "Dashboard".

2. **Configs button (hidden sm:flex):** `bg-white/60 border border-[#898989]/20 rounded-full px-4 py-2.5 text-sm text-[#303030] shadow-sm hover:bg-white/80 transition-colors`. Contains Settings icon (14px) + "Configs" text.

3. **Bell button:** Same pill style (`bg-white/60 border border-[#898989]/20 rounded-full px-3.5 py-2.5 shadow-sm hover:bg-white/80`). Bell icon 15px. Yellow notification dot: `absolute top-1.5 right-1.5 w-2 h-2 bg-[#FFD85F] rounded-full`.

4. **Profile avatar:** `w-10 h-10` circular pill button with image:
```
https://images.pexels.com/photos/1239291/pexels-photo-1239291.jpeg?auto=compress&cs=tinysrgb&w=80
```

5. **Hamburger (lg:hidden only):** `w-10 h-10`, toggles between Menu and X icons (16px). Opens a dropdown: `mt-2 bg-white/80 backdrop-blur-xl border border-[#898989]/20 rounded-2xl p-2 shadow-md flex flex-wrap gap-1` with same nav buttons.

---

### Welcome Row

`w-full mb-4`. Flex row on sm+ (`flex-col sm:flex-row sm:items-end sm:justify-between gap-4 sm:gap-8`).

**Left side (flex-[3]):**
- Greeting: `text-3xl sm:text-4xl lg:text-5xl tracking-tight text-[#303030] leading-tight` — "Good morning, Kasven"
- Segment bar below: 4 flex segments with proportional widths (flex: 15, 15, 60, 10):
  - "Screenings" 15% — dark gray pill `bg-[#303030] text-white rounded-full px-3 py-2`
  - "Placed" 15% — yellow pill `bg-[#FFD85F] text-[#303030] rounded-full`
  - "Sprint cycle" 60% — hatched pattern pill using `repeating-linear-gradient(-45deg, #e5e5e5 0px, #e5e5e5 2px, #f5f5f5 2px, #f5f5f5 10px)` with `border: 1px solid #ddd`
  - "Return" 10% — outlined pill `border border-[#898989]/40 bg-white/60`
  - Each segment has a label above (`text-xs text-[#303030] mb-1`)

**Right side (flex-[2]):** 3 stat blocks, each with:
- An icon badge: `bg-[#898989]/15 rounded-lg p-1.5 mb-1` with Users or Monitor icon (14px, `text-[#898989]`)
- Large number: `text-3xl sm:text-4xl lg:text-5xl text-[#303030] leading-none`
- Label: `text-xs text-[#303030]`
- Stats: 78 Members (Users icon), 56 Openings (Users icon), 203 Launches (Monitor icon)

---

### Dashboard Grid

Three breakpoint layouts:

**Mobile (<md):** Single column `flex flex-col gap-3`. Cards in order: ProfilePhotoCard, ProgressCard, TimeTrackerCard, OnboardingColumn, AccordionCard, CalendarCard. No wrapper divs — cards render directly.

**Tablet (md to lg):** 2-column CSS grid `gap-3`, `gridTemplateColumns: '1fr 1fr'`, `alignItems: 'stretch'`. Cards: ProfilePhotoCard, ProgressCard, TimeTrackerCard, AccordionCard (2x2), then CalendarCard and OnboardingColumn each `col-span-2`.

**Desktop (lg+):** 4-column CSS grid `gap-1 h-full`, `gridTemplateColumns: 'repeat(4, 1fr)'`, `gridTemplateRows: '1fr 1fr'`. Each cell has `h-full min-h-0`:
- Col 1, Row 1: ProfilePhotoCard
- Col 2, Row 1: ProgressCard
- Col 3, Row 1: TimeTrackerCard
- Col 4, Row 1-2 (spans both): OnboardingColumn
- Col 1, Row 2: AccordionCard
- Col 2-3, Row 2: CalendarCard

---

### Card 1: Profile Photo Card

Rounded-3xl with overflow hidden. Uses `aspect-[4/3] lg:aspect-auto` (natural aspect on mobile, fills parent on desktop). `lg:h-full`. Shadow `0 2px 20px rgba(0,0,0,0.10)`.

**Image:**
```
https://images.pexels.com/photos/1130626/pexels-photo-1130626.jpeg?auto=compress&cs=tinysrgb&w=600
```
`w-full h-full object-cover object-top`.

**Blur overlay at bottom (35% height):** An absolutely positioned div with:
- `backdropFilter: 'blur(18px) saturate(140%)'`
- `maskImage: 'linear-gradient(to top, black 40%, transparent 100%)'`
- `background: 'linear-gradient(to top, rgba(0,0,0,0.28) 0%, transparent 100%)'`

**Info bar:** Absolute bottom-3 left-3 right-3, flex between, `px-4 py-3 rounded-2xl`:
- Left: "Nora Elliston" (`text-white text-sm font-medium`) + "UI/UX Architect" (`text-white/70 text-xs`)
- Right: "$1,200" in a pill `border: 1px solid rgba(255,255,255,0.35)`, `text-white text-xs font-medium`

---

### Card 2: Activity (Progress) Card

`bg-white/60 backdrop-blur-3xl rounded-3xl p-5`, `flex flex-col gap-3 lg:h-full`.

**Header:** "Activity" (`text-lg text-[#303030]`) + arrow-up-right button (white circle `w-8 h-8`).

**Stat:** "6.1 h" (`text-4xl text-[#303030]`) + "Logged hrs / this week" (`text-xs text-[#898989]` with `<br />`).

**Bar chart area:** `h-48 lg:flex-1 lg:h-auto min-h-0` wrapping a Recharts `ResponsiveContainer`.

**Bar chart data:**
```
S:3.5, M:5.0, T:4.2, W:6.0, T:4.8, F:7.2, S:2.0
```
- `barCategoryGap="30%"`, margins `top:20 right:0 left:0 bottom:0`
- XAxis: `fill: #898989, fontSize: 11, fontFamily: 'Sofia Pro Regular'`, no axis/tick lines
- Bar radius `[6,6,6,6]`. All bars `#303030` except Friday (index 5) which is `#FFD85F`
- Custom tooltip: only shows on the Friday bar (value 7.2), displays "5h 23m" in a yellow pill `bg-[#FFD85F] text-[#303030] text-xs rounded-full px-3 py-1 shadow-md`. Cursor disabled, position `y: -30`.

---

### Card 3: Focus Timer Card

Same card wrapper as Activity card. Title: "Focus timer".

**Ring (SVG 180x180):**
- Center: cx=90, cy=90, radius=68
- Yellow arc stroke (`#FFD85F`, strokeWidth 10) covering 70% of circumference, starting from top (rotated -90deg)
- 60 tick marks around the ring (tickInner = r-4, tickOuter = r+4). Ticks in the progress zone are hidden; remaining ticks are `#898989` at 90% opacity, strokeWidth 1.2, round linecap.
- Center text: "02:35" (fontSize 22, fill #303030) and "Deep Focus" below (fontSize 10, fill #898989)

**Controls:** Below ring, full width flex between:
- Left: Play + Pause buttons (white circles `w-10 h-10`, icons 14px)
- Right: Reset button (dark circle `bg-[#303030]`, RotateCcw icon in white)

---

### Card 4: Onboarding / Induction Column

Same card wrapper. `flex flex-col gap-4 lg:h-full`.

**Header:** "Induction" (`text-lg`) + "18%" (`text-4xl`).

**Stacked bar (3 segments, flex row):**
- 30% yellow `bg-[#FFD85F] rounded-xl h-10` with "Task" label inside
- 25% dark `bg-[#303030] rounded-xl h-10`
- 20% gray `bg-[#898989] rounded-xl h-10`
- Each has a percentage label above (`text-xs text-[#898989]`)

**Task list (dark panel):** `bg-[#303030] rounded-3xl p-5 flex flex-col gap-2 flex-1`.
- Header: "Pending Actions" (`text-lg text-white`) + "2/8" (`text-base text-[#898989]`)
- 5 task rows, each: `flex items-center gap-3 py-2 border-b border-white/5 last:border-0`
  - Icon circle: `w-8 h-8 rounded-full bg-white/10`, icon 13px `text-white/60`
  - Title + time. Done tasks: `line-through text-white/30`. Undone: `text-white`
  - Time: `text-xs text-white/30`
  - Checkbox: done = yellow circle `w-5 h-5 bg-[#FFD85F]` with Check icon 10px; undone = `border border-white/20` circle

**Tasks data:**
1. Screening, Sep 13 08:30, done, Monitor icon
2. Sync Session, Sep 13 10:30, done, Users icon
3. Sprint Recap, Sep 13 13:00, not done, MessageSquare icon
4. Set Q3 Targets, Sep 13 14:45, not done, Pencil icon
5. Policy Walkthru, Sep 13 16:30, not done, Link icon

---

### Card 5: Accordion Card

`bg-white/60 backdrop-blur-3xl rounded-3xl overflow-hidden lg:h-full`. No padding on the outer container.

4 accordion items separated by `border-t border-[#898989]/15`:
1. "Retirement savings"
2. "Hardware" (expandable, open by default)
3. "Earnings breakdown"
4. "Perks & Benefits"

Each item button: `px-5 py-4 hover:bg-[#898989]/8 transition-colors`, with ChevronDown/Up (15px, `text-[#898989]`).

**Expanded "Hardware" content:** `px-5 pb-4 flex items-center gap-3 border-t border-[#898989]/15`:
- Thumbnail: `w-12 h-10 rounded-lg` with image:
```
https://images.pexels.com/photos/18105/pexels-photo.jpg?auto=compress&cs=tinysrgb&w=120
```
- Text: "ThinkPad Pro" (`text-sm text-[#303030]`) + "Edition X1" (`text-xs text-[#898989]`)
- MoreVertical icon button

---

### Card 6: Calendar Card

Same card wrapper. `lg:h-full flex flex-col`.

**Month header:** Flex between — "July" (`text-sm text-[#898989]`), "August 2024" (`text-base text-[#303030]`), "September" in outlined pill (`border border-[#898989]/25 rounded-full px-4 py-1`).

**Day headers (ml-14 sm:ml-16 mb-2):** 6 days — Mon 22, Tue 23, Wed 24, Thu 25, Fri 26, Sat 27. Day 24 is highlighted (`text-[#303030]`), others `text-[#898989]`.

**Time grid area:** `h-40 lg:flex-1`.
- Left column (`w-14 sm:w-16`): times 8:00am, 9:00am, 10:00am, 11:00am
- Right: relative container with dashed vertical column lines (`border-l border-dashed border-[#898989]/20`)
- **Event 1:** Dark card `bg-[#303030] rounded-2xl`, absolute positioned `left: 16.66%, right: 33%, top: 4px, height: 58px`. "Monthly All-Hands" + "Recap milestones across squads" (hidden on small screens). 3 avatar group.
- **Event 2:** White card `bg-white border border-[#898989]/25 rounded-2xl`, `left: 55%, right: 0%, top: 84px, height: 56px`. "Induction Briefing" + "Orientation for new joiners". 2 avatar group.

**Avatar group images:**
```
https://images.pexels.com/photos/415829/pexels-photo-415829.jpeg?auto=compress&cs=tinysrgb&w=60
https://images.pexels.com/photos/1681010/pexels-photo-1681010.jpeg?auto=compress&cs=tinysrgb&w=60
https://images.pexels.com/photos/774909/pexels-photo-774909.jpeg?auto=compress&cs=tinysrgb&w=60
```
Each avatar: `w-6 h-6 rounded-full object-cover border-2 border-white`, overlapping with `marginLeft: -6px` after the first.

---

### Responsive Behavior Summary

- Cards use `lg:h-full` so they fill parent on desktop but size to content on mobile/tablet.
- Profile photo card uses `aspect-[4/3] lg:aspect-auto`.
- Activity chart uses `h-48 lg:flex-1 lg:h-auto`.
- Calendar event area uses `h-40 lg:flex-1`.
- Tablet grid uses `alignItems: 'stretch'` so same-row cards match height.
- Mobile is a simple vertical stack with no fixed heights.
- Segment bar text is `text-xs sm:text-sm`.
- Calendar event descriptions are `hidden sm:block`.
- Configs button is `hidden sm:flex`.
- Nav links desktop only; hamburger mobile only.

---

### Dependencies (package.json)

- react 18.3.1, react-dom 18.3.1
- recharts 3.8.1
- lucide-react 0.344.0
- @supabase/supabase-js 2.57.4
- Tailwind CSS 3.4.1 with autoprefixer and postcss
- Vite 5.4.2, TypeScript 5.5.3

---

### Tailwind Config

Extend `borderRadius` with `3xl: '24px'` and `4xl: '32px'`. Custom colors: `yellow.DEFAULT: '#FFD85F'`, `dark-gray: '#303030'`, `light-gray: '#898989'`. Font family sans set to Century Gothic (fallback only, actual rendering uses Sofia Pro from the external sheet).

---

### CSS (index.css)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  font-family: "Sofia Pro Medium", sans-serif;
  -webkit-font-smoothing: antialiased;
}

:root {
  scrollbar-width: thin;
  scrollbar-color: #e5e5e5 transparent;
}
```

## Nimbus Demo — Dashboard Demo [sites/nimbus-demo]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(57).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-demo.webp

--

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

**Shared h3 base:**
```css
h3 {
  margin-bottom: 14px;
  font-size: 24px;
  line-height: 1.2;
  letter-spacing: 0.0125rem;
  font-weight: 400;
}
```

---

### Color Theme Note

This section uses a **cool cyan/blue palette** (`#97d3eb`, `rgba(151, 211, 235, ...)`, `#dff5ff`) rather than the warm gold used elsewhere. This is intentional — it differentiates the console/product UI section from the marketing sections.

---

### Section: `.console-showcase`

`<section class="console-showcase" id="plans" aria-labelledby="console-title">`

**Outer container:**
```css
.console-showcase {
  position: relative;
  min-height: 112svh;
  padding: clamp(72px, 8vw, 120px) clamp(20px, 5vw, 72px);
  overflow: hidden;
  border-top: 1px solid rgba(255, 240, 199, 0.1);
  background:
    radial-gradient(circle at 82% 34%, rgba(151, 211, 235, 0.12), transparent 24rem),
    #070a0b;
}
```

Background is a very dark cool-black (`#070a0b`) with a subtle cyan radial glow at top-right.

**Decorative ruled-lines block (`::after`):**
```css
.console-showcase::after {
  content: "";
  position: absolute;
  top: 19%;
  right: 8%;
  width: min(360px, 24vw);
  height: 210px;
  border-radius: 4px;
  background:
    linear-gradient(180deg, rgba(255, 247, 222, 0.04), rgba(255, 247, 222, 0.02)),
    repeating-linear-gradient(180deg, rgba(255, 247, 222, 0.08) 0 48px, transparent 48px 70px);
  opacity: 0.55;
}
```

This creates a subtle decorative element in the top-right — faint horizontal ruled lines, like a ghost of a document. Purely atmospheric.

---

### Part 1: `.console-showcase-heading` (two-column header)

```html
<div class="console-showcase-heading">
  <h2 id="console-title">The biggest forward leap in business cloud storage operations.</h2>
  <p>
    A single control plane for provisioning storage pools, reviewing policy, watching growth,
    and shipping audit-ready reports without asking teams to change how they work.
  </p>
</div>
```

```css
.console-showcase-heading {
  position: relative;
  z-index: 2;
  display: grid;
  grid-template-columns: minmax(0, 720px) minmax(220px, 360px);
  justify-content: space-between;
  gap: clamp(32px, 6vw, 86px);
  width: 100%;
}

.console-showcase-heading h2 {
  max-width: 720px;
  color: #dff5ff;
  font-size: clamp(25px, 4vw, 52px);
  line-height: 1.08;
  letter-spacing: 0.005rem;
  font-weight: 400;
}

.console-showcase-heading p {
  align-self: end;
  justify-self: end;
  max-width: 360px;
  color: rgba(223, 245, 255, 0.72);
  font-size: 19px;
  line-height: 1.55;
}
```

The h2 is in a light cyan-white (`#dff5ff`). The paragraph is a muted cyan, aligned to the bottom-right.

---

### Part 2: `.console-figure-label` (figure caption)

```html
<div class="console-figure-label">
  <span>Fig. 2</span>
  Nimbus Grid web console
</div>
```

```css
.console-figure-label {
  position: relative;
  z-index: 2;
  width: 100%;
  margin: clamp(42px, 6vw, 72px) 0 14px;
  color: rgba(255, 247, 222, 0.42);
  font-family: var(--font-mono);
  font-size: 12px;
  letter-spacing: 0.08rem;
  text-transform: uppercase;
}

.console-figure-label span {
  display: inline-flex;
  padding: 5px 8px;
  margin-right: 10px;
  border: 1px solid rgba(255, 247, 222, 0.18);
  border-radius: 2px;
}
```

A small technical label — "Fig. 2" in a bordered inline badge, then "Nimbus Grid web console" as adjacent text. All uppercase mono, muted color.

---

### Part 3: `.dashboard-shell` (the full dashboard mockup)

This is the centerpiece — a complete faux web-app UI with hover effects.

```html
<div class="dashboard-shell">
  <div class="dashboard-topbar">
    <span></span>
    <span></span>
    <span></span>
    <strong></strong>
  </div>
  <div class="dashboard-body">
    <aside class="dashboard-sidebar">
      <strong>Client Vault</strong>
      <nav aria-label="Console sections">
        <span>Workspaces</span>
        <span class="active">Storage Pools</span>
        <span>Retention</span>
        <span>Access</span>
        <span>Transfers</span>
        <span>Reports</span>
      </nav>
    </aside>

    <div class="dashboard-main">
      <div class="dashboard-title-row">
        <h3>Storage Pools</h3>
        <button type="button">New pool</button>
      </div>

      <div class="dashboard-table" role="table" aria-label="Storage pool status">
        <div class="dashboard-row header" role="row">
          <span>Name</span>
          <span>Region</span>
          <span>Used</span>
          <span>Policy</span>
          <span>State</span>
        </div>
        <div class="dashboard-row" role="row">
          <span>finance-vault</span>
          <span>EU Central</span>
          <span>18.4 TiB</span>
          <span>7 years</span>
          <strong>Healthy</strong>
        </div>
        <div class="dashboard-row" role="row">
          <span>design-assets</span>
          <span>US East</span>
          <span>9.8 TiB</span>
          <span>Versioned</span>
          <strong>Syncing</strong>
        </div>
        <div class="dashboard-row" role="row">
          <span>legal-archive</span>
          <span>EU Central</span>
          <span>42.1 TiB</span>
          <span>Immutable</span>
          <strong>Healthy</strong>
        </div>
        <div class="dashboard-row" role="row">
          <span>migration-lane</span>
          <span>AP South</span>
          <span>6.2 TiB</span>
          <span>Temporary</span>
          <strong>Queued</strong>
        </div>
      </div>
    </div>
  </div>

  <div class="dashboard-toast">
    <strong>Pool created</strong>
    finance-vault ready
  </div>
</div>
```

### Dashboard shell (outer frame with 3D hover)

```css
.dashboard-shell {
  position: relative;
  z-index: 2;
  width: 100%;
  min-height: 620px;
  border: 1px solid rgba(151, 211, 235, 0.18);
  border-radius: 8px;
  overflow: hidden;
  background: rgba(5, 8, 10, 0.9);
  box-shadow: 0 36px 120px rgba(0, 0, 0, 0.44);
  transform: perspective(1400px) rotateX(0) rotateY(0) translateY(0);
  transition: transform 220ms ease, border-color 220ms ease, box-shadow 220ms ease;
}
```

**Specular shine overlay (`::before`):**
```css
.dashboard-shell::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1;
  pointer-events: none;
  background: linear-gradient(
    115deg,
    rgba(223, 245, 255, 0) 0%,
    rgba(223, 245, 255, 0.08) 42%,
    rgba(223, 245, 255, 0) 64%
  );
  opacity: 0;
  transform: translateX(-34%);
  transition: opacity 220ms ease, transform 520ms ease;
}
```

**Hover state (3D tilt + shine sweep):**
```css
.dashboard-shell:hover {
  border-color: rgba(151, 211, 235, 0.34);
  box-shadow: 0 44px 140px rgba(0, 0, 0, 0.52), 0 0 80px rgba(151, 211, 235, 0.08);
  transform: perspective(1400px) rotateX(1deg) rotateY(-1.2deg) translateY(-8px);
}

.dashboard-shell:hover::before {
  opacity: 1;
  transform: translateX(34%);
}
```

On hover, the entire dashboard tilts slightly in 3D (1deg X, -1.2deg Y), lifts up 8px, gets a brighter border and an expanded shadow, and a diagonal light-sweep slides across the surface from left to right.

### Top bar

```css
.dashboard-topbar {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 58px;
  padding: 0 18px;
  border-bottom: 1px solid rgba(151, 211, 235, 0.14);
}

.dashboard-topbar span {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: rgba(255, 247, 222, 0.14);
}

.dashboard-topbar strong {
  width: 170px;
  height: 13px;
  margin-left: 72px;
  border-radius: 2px;
  background: rgba(255, 247, 222, 0.09);
}
```

3 dot placeholders (window controls) + a rectangular "address bar" placeholder. All ghost/skeleton elements.

### Body (sidebar + main)

```css
.dashboard-body {
  position: relative;
  z-index: 2;
  display: grid;
  grid-template-columns: 240px 1fr;
  min-height: 560px;
}
```

### Sidebar

```css
.dashboard-sidebar {
  padding: 26px 18px;
  border-right: 1px solid rgba(151, 211, 235, 0.12);
  color: rgba(223, 245, 255, 0.44);
}

.dashboard-sidebar strong {
  display: block;
  margin-bottom: 34px;
  color: rgba(223, 245, 255, 0.7);
  font-weight: 400;
}

.dashboard-sidebar nav {
  display: grid;
  gap: 12px;
}

.dashboard-sidebar span {
  padding: 7px 10px;
  border-radius: 3px;
}

.dashboard-sidebar .active {
  color: #97d3eb;
  background: rgba(151, 211, 235, 0.13);
}
```

Sidebar nav items: Workspaces, **Storage Pools** (active, highlighted cyan), Retention, Access, Transfers, Reports. "Client Vault" as the workspace name at top.

### Main content area

```css
.dashboard-main {
  padding: clamp(34px, 5vw, 60px);
}

.dashboard-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 34px;
}

.dashboard-title-row h3 {
  margin: 0;
  color: #97d3eb;
  font-size: clamp(28px, 2.8vw, 44px);
}

.dashboard-title-row button {
  min-height: 36px;
  padding: 0 14px;
  border: 1px solid rgba(151, 211, 235, 0.3);
  border-radius: 3px;
  color: #97d3eb;
  background: rgba(151, 211, 235, 0.1);
  font-family: var(--font-mono);
  font-size: 12px;
  letter-spacing: 0.06rem;
  text-transform: uppercase;
}
```

### Data table

```css
.dashboard-table {
  display: grid;
  border: 1px solid rgba(255, 247, 222, 0.12);
  border-radius: 4px;
  overflow: hidden;
}

.dashboard-row {
  display: grid;
  grid-template-columns: 1.3fr 1fr 0.8fr 1fr 0.8fr;
  min-height: 54px;
  align-items: center;
  border-bottom: 1px solid rgba(255, 247, 222, 0.08);
  transition: background 160ms ease;
}

.dashboard-row:not(.header):hover {
  background: rgba(151, 211, 235, 0.06);
}

.dashboard-row:last-child {
  border-bottom: 0;
}

.dashboard-row span,
.dashboard-row strong {
  padding: 0 16px;
  color: rgba(223, 245, 255, 0.64);
  font-weight: 400;
}

.dashboard-row.header span {
  color: rgba(223, 245, 255, 0.42);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.09rem;
  text-transform: uppercase;
}

.dashboard-row strong {
  color: #97d3eb;
  font-family: var(--font-mono);
  font-size: 12px;
  text-transform: uppercase;
}
```

5 columns: Name, Region, Used, Policy, State. Header row has uppercase mono labels. Data rows have muted text for spans and cyan mono for strong (status). Rows highlight on hover.

**Table data (exact):**

| Name | Region | Used | Policy | State |
|---|---|---|---|---|
| finance-vault | EU Central | 18.4 TiB | 7 years | **Healthy** |
| design-assets | US East | 9.8 TiB | Versioned | **Syncing** |
| legal-archive | EU Central | 42.1 TiB | Immutable | **Healthy** |
| migration-lane | AP South | 6.2 TiB | Temporary | **Queued** |

### Toast notification

```css
.dashboard-toast {
  position: absolute;
  right: clamp(28px, 7vw, 90px);
  bottom: 58px;
  width: min(330px, 34vw);
  padding: 18px 20px;
  border: 1px solid rgba(151, 211, 235, 0.22);
  border-radius: 4px;
  color: #97d3eb;
  background: rgba(8, 34, 42, 0.92);
  box-shadow: 0 22px 60px rgba(0, 0, 0, 0.34);
}

.dashboard-toast strong {
  display: block;
  margin-bottom: 6px;
  font-weight: 400;
}
```

A floating notification card positioned bottom-right inside the shell. Dark teal background with cyan border and text. Shows: bold "Pool created" label, then "finance-vault ready" body text.

---

### Responsive Breakpoints

### `@media (max-width: 820px)`

```css
.console-showcase-heading {
  grid-template-columns: 1fr;
}

.console-showcase-heading h2 {
  max-width: 760px;
  font-size: clamp(25px, 6vw, 52px);
}

.console-showcase-heading p {
  justify-self: start;
  max-width: 420px;
}

.dashboard-body {
  grid-template-columns: 1fr;
}

.dashboard-sidebar {
  border-right: 0;
  border-bottom: 1px solid rgba(151, 211, 235, 0.12);
}

.dashboard-sidebar nav {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.dashboard-row {
  grid-template-columns: 1.2fr 0.9fr 0.8fr;
}

.dashboard-row span:nth-child(4),
.dashboard-row span:nth-child(5),
.dashboard-row strong {
  display: none;
}

.dashboard-toast {
  position: static;
  width: auto;
  margin: 24px;
}
```

At 820px: Heading stacks to single column. Dashboard sidebar moves to top with 2-column nav. Table hides columns 4 (Policy) and 5 (State/strong) — only Name, Region, Used visible. Toast becomes static at the bottom of the shell.

### `@media (max-width: 520px)`

```css
.console-showcase {
  padding-inline: 18px;
}

.console-showcase-heading h2 {
  font-size: clamp(25px, 8vw, 44px);
}

.dashboard-shell {
  min-height: 0;
}

.dashboard-main {
  padding: 24px 16px;
}

.dashboard-title-row {
  align-items: flex-start;
  flex-direction: column;
}
```

At 520px: Tighter padding. Dashboard min-height removed. Title row stacks vertically (heading above button).

---

### JavaScript

No JavaScript is required for this section. The only interactive behavior is the CSS hover effect on `.dashboard-shell` (3D tilt + shine sweep), which is entirely CSS-driven via transitions.

---

### Project structure

```
index.html       (section markup + font links)
styles.css       (all styles + media queries)
script.js        (empty — no JS needed)
package.json     (vite ^5.4.2, "type": "module", scripts: dev/build/preview)
vite.config.js   (default export)
```

## Rocket FAQ — FAQ [sites/rocket-faq]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(31).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/rocket-faq.webp

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

## Cognitra Feature — Feature [sites/cognitra-feature]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(72).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cognitra-feature.webp

---

**Prompt:**

Create a full-viewport section (100vh) that sits over a fixed background video. The section has no background color of its own -- it is fully transparent so the fixed video behind it shows through.

**Background video (fixed, behind everything):**
A `<video>` element fixed to the viewport (`position: fixed; top: 0; left: 0; width: 100%; height: 100vh; object-fit: cover; z-index: 0`), using this source:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_135830_bb6491d1-9b66-4aec-9722-13b4dfe3fb46.mp4
```
It should `autoPlay`, be `muted`, `loop`, and `playsInline`.

**Section layout:**
- `position: relative; z-index: 1`
- `display: flex; flex-direction: column; justify-content: center` (centers content vertically)
- `height: 100vh`
- Padding: `70px 32px 32px 32px`

**Content block** (inside the section):
- A wrapper `div` with `display: flex; flex-direction: column; align-items: flex-start; max-width: 720px`
- **Heading (`<h2>`):**
  - Text: `"WE BUILD END-TO-END AI AUTOMATION SYSTEMS."`
  - Each word is wrapped in an individual `<span>` element, displayed using `display: flex; flex-wrap: wrap; gap: 0.25em`
  - Each word animates in with a staggered fade-up animation: starts at `opacity: 0, y: 32px`, animates to `opacity: 1, y: 0` using Framer Motion `whileInView` with `viewport: { once: true, amount: 0.2 }`
  - Stagger: first word at `delay: 0.15`, each subsequent word adds `0.08s` (so word 2 = 0.23, word 3 = 0.31, etc.)
  - Animation easing: `[0.22, 1, 0.36, 1]`, duration: `0.7s`
  - Typography: `font-size: clamp(26px, 3vw, 42px); font-weight: 700; line-height: 1.08; letter-spacing: -0.01em; text-transform: uppercase; color: #fff; margin: 0`

- **Subtext (`<p>`):**
  - Text: `"We provide all-in-one AI automation services in one place."`
  - `margin-top: 24px; font-size: 14px; line-height: 1.65; color: rgba(255,255,255,0.85); max-width: 260px`
  - Same fade-up animation as the words but with `delay: 0.9` and default `y: 24px`

**Font:**
```css
@import url('https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var');
* { font-family: 'Helvetica Now Var', 'Helvetica Neue', Helvetica, Arial, sans-serif; }
```

**FadeUp component (reusable, Framer Motion):**
```tsx
import { motion } from 'framer-motion';
import { CSSProperties, ReactNode } from 'react';

type FadeUpProps = {
  children: ReactNode;
  delay?: number;
  duration?: number;
  y?: number;
  className?: string;
  style?: CSSProperties;
  as?: 'div' | 'section' | 'span' | 'h1' | 'h2' | 'h3' | 'p' | 'nav';
  once?: boolean;
};

export function FadeUp({
  children, delay = 0, duration = 0.7, y = 24,
  className, style, as = 'div', once = true,
}: FadeUpProps) {
  const Tag = motion[as];
  return (
    <Tag
      className={className}
      style={style}
      initial={{ opacity: 0, y }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once, amount: 0.2 }}
      transition={{ duration, delay, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </Tag>
  );
}
```

**Mobile responsive (max-width: 900px):**
- Section padding changes to `90px 18px 32px 18px`

**Tech stack:** React 18, TypeScript, Vite, Tailwind CSS 3, Framer Motion 12.

---

## Capabilities Overview — Features [sites/features]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(39).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/features.webp

Build a Capabilities section for an aerospace company called "EngineTech." This is a light-background proof grid with a header, a CTA pill button, and a 3-column bento-grid of mixed card types (video cards, quote card, metric card, tool-marquee card, and contact card).

---

SECTION CONTAINER

- Class: `.capabilities`. Position relative, z-index 70, min-height 100vh.
- Padding: `clamp(34px, 4vw, 72px) clamp(16px, 3.8vw, 72px)`.
- Background: `#f7f8f8`. Color: `#111111`.

---

HEADER (`.capabilities__header`)

- Flex row, `align-items: flex-start`, `justify-content: space-between`, gap 32px.
- Max-width 1820px (`var(--hero-max-width)`), centered, margin-bottom `clamp(24px, 3vw, 42px)`.

Left intro (`.capabilities__intro`):

- Max-width 860px.
- H2: "Propulsion programs need a partner that can move from concept to certified hardware."
  - Max-width 920px, margin 0, color `#111111`, `font-size: clamp(29px, 3.2vw, 54px)`, weight 300, letter-spacing 0, line-height 1.08.
- P: "EngineTech combines precision manufacturing, hot-fire validation, materials engineering, and mission support for aircraft and spacecraft programs that cannot afford uncertainty."
  - Max-width 760px, margin `18px 0 0`, color `#677070`, `font-size: clamp(14px, 1vw, 17px)`, weight 400, line-height 1.62.

Right CTA button (`.capabilities__button`):

- `flex: 0 0 auto`, `align-self: flex-start`, inline-flex, centered, gap 10px.
- Min-height 48px, padding `0 20px`.
- Border: `1px solid rgb(17 17 17 / 0.1)`, border-radius 999px.
- Background: `rgb(255 255 255 / 0.78)`, color `#111111`, font-size 14px, weight 700.
- Box-shadow: `inset 0 1px 0 rgb(255 255 255 / 0.95), 0 18px 44px rgb(31 44 44 / 0.08)`.
- Text: "Start a Program" followed by Phosphor icon `ph-arrow-up-right` at 18px.

---

BENTO GRID (`.capabilities__grid`)

- CSS grid: `grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr)`.
- Gap: `clamp(14px, 1.25vw, 22px)`. Max-width 1820px, centered.
- Min-height: `clamp(620px, 72vh, 780px)`.

Each column is a `.capabilities__stack` -- a nested grid with `gap: clamp(14px, 1.25vw, 22px)`.

---

COLUMN 1: Single tall video card

Stack grid-template-rows: `minmax(210px, 0.74fr) minmax(270px, 1fr)` (but this column only has one card spanning the full height).

Card: `.cap-card.cap-card--tall.cap-card--media`

- Position relative, overflow hidden, border `1px solid rgb(18 35 35 / 0.09)`, border-radius 18px.
- Background: `#dce3e3`. Color: `#ffffff`. Min-height implied by grid.
- Box-shadow: `0 22px 60px rgb(21 34 34 / 0.08)`.

Video: `` element, absolute inset 0, 100% w/h, `object-fit: cover`, `transform: scale(1.02)`.
- Source: `https://assets.mixkit.co/videos/45229/45229-720.mp4`. Autoplay, muted, loop, playsinline.

**Dark shade overlay (`.cap-cardshade`):** Absolute inset 0:
- `background: linear-gradient(180deg, rgb(5 12 14 / 0.3), transparent 34%), linear-gradient(0deg, rgb(5 12 14 / 0.78), transparent 48%)`.

**Top label (`.cap-cardlabel`):** Relative z-index 1, flex centered, padding 24px, color `rgb(255 255 255 / 0.78)`, font-size 11px, weight 760, `letter-spacing: 0.18em`, uppercase. Text: "Program Background".

**Timeline overlay (`.cap-cardtimeline`):** Absolute positioned, z-index 1, `right: 20px; bottom: 20px; left: 20px`. Grid with 12px gap. Each row is a 4-column grid: `grid-template-columns: 58px 16px minmax(0, 1fr) auto`, align-items center, gap 10px, color `rgb(255 255 255 / 0.76)`, font-size 12px. Contains:

| Year | Dot | Program | Note |
|------|-----|---------|------|
| 2026 | (5px white circle at 62% opacity) | **Reusable upper-stage demonstrator** (clamp(13px, 0.95vw, 15px), weight 650, white) | *Thermal qualification* (rgb(255 255 255 / 0.58), normal style) |
| 2025 | dot | **Hybrid-electric aircraft platform** | *Combustor redesign* |
| 2024 | dot | **Orbital transfer vehicle** | *Flight article delivery* |

---

### COLUMN 2: Quote card + Metric card

**Stack grid-template-rows:** `minmax(210px, 0.74fr) minmax(270px, 1fr)`.

**Card A: `.cap-card.cap-card--quote`**

- Flex column, `justify-content: space-between`, padding 24px.
- Background: `linear-gradient(135deg, rgb(255 255 255 / 0.72), rgb(238 244 244 / 0.86)), #edf2f2`.
- Border: `1px solid rgb(18 35 35 / 0.09)`, border-radius 18px.

- Top label: left-aligned (`.cap-cardlabel--left`), no padding, color `#758080`, text: "Mission Voice".
- Blockquote: margin `clamp(22px, 2.4vw, 34px) 0 20px`, color `#263030`, `font-size: clamp(15px, 1vw, 18px)`, line-height 1.62. Text: "EngineTech brought the discipline we needed: clear design reviews, repeatable test data, and hardware that arrived ready for integration."
- Attribution: `

` with `` "Dr. Lena Morris" (block, color `#111111`, 15px) then "Propulsion Lead, Orbital Systems Group" (color `#6b7676`, 14px, line-height 1.5).

**Card B: `.cap-card.cap-card--metric.cap-card--video-panel`**

- Display block, min-height 320px. Background `#dce3e3`, color `#ffffff`.

- Video: same absolute pattern. Source: `https://assets.mixkit.co/videos/23211/23211-720.mp4`. Autoplay, muted, loop, playsinline.
- Dark shade overlay (same gradient as column 1 tall card).
- Metric overlay (`.cap-cardmetric`): absolute inset 0, z-index 1, 100% w/h, text-align center, `text-shadow: 0 12px 32px rgb(0 0 0 / 0.3)`.
  - ``: "2K" -- absolute `top: 50%; left: 50%; transform: translate(-50%, -50%)`, `font-size: clamp(82px, 7.4vw, 134px)`, weight 220, line-height 0.9.
  - ``: "Highly Qualified Engineers" -- absolute `right: 24px; bottom: 24px; left: 24px`, color `rgb(255 255 255 / 0.82)`, `font-size: clamp(14px, 1.05vw, 18px)`, line-height 1.4.

---

### COLUMN 3: Tool-marquee card + Contact card

**Stack with modifier `.capabilitiesstack--systems`:**
- `grid-template-rows: minmax(420px, 1.45fr) auto`.

**Card A: `.cap-card.cap-card--tools.cap-card--tools-media.cap-card--video-panel`**

- Flex column, `justify-content: space-between`. Min-height 420px.
- Background: transparent (video fills).

- Video: Source `https://assets.mixkit.co/videos/23843/23843-720.mp4`. Autoplay, muted, loop, playsinline.
- Shade overlay (modified): `linear-gradient(180deg, rgb(5 12 14 / 0.18), transparent 34%), linear-gradient(0deg, rgb(5 12 14 / 0.32), transparent 56%)`.
- Top label: color `rgb(255 255 255 / 0.82)`, text "Core Systems".

**Tool marquee (`.tool-marquee`):** Grid, gap 14px, overflow hidden, padding `26px 0 8px`. Horizontal fade mask: `mask-image: linear-gradient(to right, transparent, #000 9%, #000 91%, transparent)`.

Two rows of pill tags, each row is a flex container (`width: max-content`, gap 12px). Each pill: inline-flex, centered, gap 8px, min-height 54px, padding `0 16px`, border `1px solid rgb(255 255 255 / 0.2)`, border-radius 14px, background `rgb(255 255 255 / 0.18)`, color `#ffffff`, font-size 13px, weight 700, `backdrop-filter: blur(10px)`, box-shadow `inset 0 1px 0 rgb(255 255 255 / 0.24)`. Icon: Phosphor at 20px.

**Row 1 (animates left, 24s linear infinite):** Turbopumps (ph-gear-six), Hot-fire (ph-fire), Telemetry (ph-gauge), Alloys (ph-atom), Assembly (ph-wrench) -- duplicated once for seamless loop.

**Row 2 (animates right, 28s linear infinite, starts at -50% translateX):** Controls (ph-cpu), Vibration (ph-wave-sine), Certification (ph-shield-check), Launch (ph-rocket-launch), Analysis (ph-chart-line-up) -- duplicated once.

Marquee keyframes:
- `marquee-left`: `translateX(0)` to `translateX(-50%)`
- `marquee-right`: `translateX(-50%)` to `translateX(0)`

**Card B: `.cap-card.cap-card--contact`**

- Flex row, `align-items: center`, `justify-content: space-between`, gap 20px.
- Min-height 118px. Padding: `20px 76px 20px 24px`.
- Background: same gradient as quote card.
- Border, border-radius same as other cards.

- Left side:
  - Label: "Reach Engineering" (left-aligned, `#758080`, 11px, weight 760, `letter-spacing: 0.18em`, uppercase).
  - Email link: "programs@enginetech.com" -- `font-size: clamp(18px, 1.45vw, 24px)`, weight 360, color `#111111`, margin `14px 0 6px`.
  - Phone: "+1 415 018 4270" -- color `#6b7676`, 14px, line-height 1.5.
- Right side: Circular icon button (`.cap-card__icon-button`), absolute `top: 50%; right: 16px; transform: translateY(-50%)`, 42px square, border-radius 50%, border `1px solid rgb(17 17 17 / 0.1)`, background `#111111`, color `#ffffff`. Contains Phosphor `ph-arrow-up-right` at 19px.

---

### RESPONSIVE BREAKPOINTS

**At 1080px:**
- Grid becomes 2 columns: `repeat(2, minmax(0, 1fr))`.
- Min-height auto. Tall card gets `min-height: 620px`.
- Third stack (systems) spans full width: `grid-column: 1 / -1`, becomes 2-col sub-grid `grid-template-columns: repeat(2, minmax(0, 1fr))`, `grid-template-rows: minmax(260px, 1fr)`.

**At 760px:**
- Header becomes `flex-direction: column`. Button goes full width.
- Grid, stacks, and systems stack all become single column (`grid-template-columns: 1fr`, rows auto).
- Tall card min-height 560px.
- Timeline grid becomes `grid-template-columns: 52px 14px minmax(0, 1fr)` (date text wraps to third column).

---

### GLOBAL STYLES

**Font stack:** `"Geist", "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` with `-webkit-font-smoothing: antialiased` and `text-rendering: geometricPrecision`.

**Icon library:** Phosphor Icons from `https://unpkg.com/@phosphor-icons/web@2.1.1/src/regular/style.css`.

**Color palette:** No purple or violet. Neutral `#f7f8f8` background, dark `#111111` text, teal-gray accents `#677070`, `#758080`, `#6b7676`. Card backgrounds use white and soft mint gradients.

## Interior Features — Features [sites/interior-features]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(9).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/interior-features.webp

Build a single interactive product card that swaps between a still photo (light mode) and a looping video (dark mode), with a slow crossfade between them.

**Font**

- Use `Neue Montreal` (custom `@font-face`, weights 400/500/700, `.woff2`), falling back to `sans-serif`. Load Material Icons from `https://fonts.googleapis.com/icon?family=Material+Icons`.

**Container / card chrome**

- The card lives in a grid `.cards` (`display:grid; grid-template-columns:repeat(4,1fr); gap:clamp(12px,1.2vw,20px); align-items:stretch;`).
- Each card is wrapped in `.card-frame`:
  - `aspect-ratio:319 / 404;`
  - `border-radius:clamp(18px,1.7vw,28px);`
  - `overflow:hidden; background:#0e0d0c;`
  - `box-shadow:0 36px 70px -34px rgba(28,18,8,.42), 0 4px 14px -8px rgba(0,0,0,.18);`

**Card internals**

- `.direct-card`: `position:relative; width:100%; height:100%; overflow:hidden; background:#1a1714; isolation:isolate;`
- A fixed-size artboard `.direct-card__artboard` (`#directCardTwoArtboard`) that is scaled to fit:
  - `position:absolute; top:50%; left:50%; width:660px; height:836px; border-radius:30px; overflow:hidden; background:#1a1714; isolation:isolate; container-type:inline-size; transform:translate(-50%,-50%) scale(var(--direct-scale,1)); transform-origin:center center;`
  - Compute `--direct-scale` in JS as `Math.min(card.clientWidth/660, card.clientHeight/836)` and recompute on load and `resize`.

**Media layers (inside the artboard, in this order)**

1. Image `.direct-card__photo`:
   - `src="https://res.cloudinary.com/dgupuutfn/image/upload/v1780913983/room2_pihyox.png"`
   - `data-night-src="https://res.cloudinary.com/dgupuutfn/image/upload/v1780913982/room2_night_qc4qeq.png"`
   - `alt="Living room interior"`
   - CSS: `position:absolute; inset:0; width:100%; height:100%; object-fit:cover; object-position:50% 42%; z-index:0;`
2. Video `.direct2-video`:
   - `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260609_072414_1e4448ec-537f-4f94-b56b-19e00c1550e8.mp4"`
   - attributes: `muted loop playsinline preload="auto"`
   - CSS: same box as photo (`position:absolute; inset:0; width:100%; height:100%; object-fit:cover; object-position:50% 42%; z-index:0;`) plus `opacity:0; pointer-events:none; transition:opacity 4.5s cubic-bezier(0.22, 1, 0.36, 1);`
3. Gradient scrim `.direct2-grade`:
   - `position:absolute; inset:0; z-index:1; pointer-events:none;`
   - `background:linear-gradient(to top, rgba(8,6,4,.92) 0%, rgba(8,6,4,.6) 13%, rgba(8,6,4,.1) 28%, rgba(8,6,4,0) 42%), linear-gradient(to bottom, rgba(20,14,8,.34) 0%, rgba(20,14,8,0) 20%);`

**Footer block `.direct-footer`** (`position:absolute; z-index:4; left:6.2%; right:6.2%; bottom:5.4%; color:#fff;`):
- `.direct-footer__head` (`display:flex; align-items:center; gap:3%; margin-bottom:3.2%;`) containing:
  - `.direct-footer__icon` round badge: `flex:none; width:9.6%; aspect-ratio:1; border-radius:50%; display:flex; align-items:center; justify-content:center;` with inline style `background:#3fae6b; color:#fff; box-shadow:0 10px 24px -10px rgba(63,174,107,.6);`, holding `<span class="material-icons">construction</span>` (icon sized `font-size:clamp(15px,5.6cqw,34px); line-height:1;`).
  - `.direct-footer__title` text "Build the room in real time": `font-weight:700; line-height:1.05; letter-spacing:-.6px; white-space:nowrap; font-size:clamp(18px,5.6cqw,36px);`
- `.direct-footer__desc` text "Move pieces, explore finishes, and align with your studio on one shared canvas.": `color:#d7d2c9; font-weight:500; line-height:1.28; max-width:100%; font-size:clamp(13px,4.2cqw,27px);`

(All footer text uses container query units `cqw`, so the artboard must set `container-type:inline-size`.)

**Dark-mode behavior**

- A `body.is-night` class toggles dark mode (driven by a theme switch elsewhere on the page).
- When `body.is-night` is set: `.direct2-video { opacity:1; }` (fades in over 4.5s; the photo stays beneath). In light mode the video is `opacity:0`.
- In the toggle handler, also `video.play()` (catch/ignore promise rejection) when entering night and `video.pause()` when leaving.
- The still photo additionally swaps its `src` to `data-night-src` on entering night and back on day, with a matching `opacity 4.5s cubic-bezier(0.22, 1, 0.36, 1)` crossfade (fade to 0, swap on `load`, fade back to 1).

**Responsiveness**

- `@media (max-width:920px)`: page scrolls normally (`html,body{height:auto;overflow-y:auto;}`), `.cards` becomes `grid-template-columns:repeat(2,1fr); gap:clamp(16px,3vw,24px);`.
- `@media (max-width:540px)`: `.cards` becomes a single column, `max-width:380px; margin:0 auto;`.
- The artboard scale (`--direct-scale`) keeps the 660×836 internal layout perfectly proportioned at any card size.

## LaunchEx Submissions — Features [sites/launchex-submissions]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(52).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/launchex-submissions.webp

**Prompt to recreate the Submissions section:**

> Build a full-viewport "Submissions" section using React with Tailwind CSS (no extra UI libraries). This is a single `<section>` with the following exact specifications:
>
> **Section container:**
> - `id="nominations"`
> - Background color: `#F0F0F0`
> - `min-height: 100vh`
> - Overflow hidden, position relative
> - Padding: `py-20 sm:py-28 px-6 sm:px-10`
> - Uses `flex flex-col justify-center` to vertically center content
>
> **Inner layout:**
> - A single flex container: `flex flex-col lg:flex-row`, `items-start lg:items-stretch`, `justify-center`, `gap-10 lg:gap-12`, `max-w-5xl mx-auto`, position relative
> - Three columns: left nomination cards, center video + heading, right nomination cards
> - On mobile, the center column appears first (`order-1 lg:order-2`), left cards second (`order-2 lg:order-1`), right cards third (`order-3`)
>
> **Center column** (flex-1, flex-col, items-center, justify-start):
> - A heading block with `uppercase`, color `#154359`, flex-col items-center gap-2:
>   - Small label: `<span>` with text `[submissions]`, font-size `12px`, letter-spacing `0.24em`
>   - Large heading: `<h2>` with text `submissions`, uses custom font class `.font-firs` (font-family: `'TT Firs Neue', 'Inter', system-ui, sans-serif`), font-size `44px sm:54px`, `font-semibold`, `tracking-tight`
> - Below heading, a video container with `mt-6 sm:mt-8`, sized `w-[220px] sm:w-[380px] lg:w-[460px]` and `h-[220px] sm:h-[380px] lg:h-[460px]`:
>   - `<video>` element, `object-cover`, `w-full h-full`, autoPlay, loop, muted, playsInline
>   - Video source URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_154120_b89bfedd-530d-4ebb-9eb7-42eeafe08667.mp4`
>
> **Left nomination cards column:**
> - `flex flex-col gap-4 items-center lg:items-start lg:mt-36`
> - Contains 3 NominationCard components with these exact titles/subtitles:
>   1. title: `"Lead"`, subtitle: `"AI venture for commerce"`
>   2. title: `"Emerging innovations"`, subtitle: `"in food commerce"`
>   3. title: `"The finest innovations"`, subtitle: `"for learners and young students"`
>
> **Right nomination cards column:**
> - `flex flex-col gap-4 items-center lg:items-end lg:mt-36`
> - Contains 3 NominationCard components:
>   1. title: `"Innovations for advanced"`, subtitle: `"career training"`
>   2. title: `"The finest innovations"`, subtitle: `"in finance"`
>   3. title: `"Categories"`, subtitle: `"coming soon"`
>
> **NominationCard component** (each card):
> - An `<a href="#">` tag, `group relative block w-full max-w-[20em] h-[5em]`, with `transition-transform hover:-translate-y-0.5`
> - Inside, an SVG positioned `absolute inset-0 w-full h-full`, with `preserveAspectRatio="none"` and `viewBox="0 0 100 100"`:
>   - A single `<polygon>` with `points="14,0 100,0 100,86 86,100 0,100 0,14"` -- this creates a hexagonal/chamfered-corner shape
>   - `fill="none"`, `stroke="rgba(6, 99, 119, 0.25)"` (teal at 25% opacity), `strokeWidth="1"`, `vectorEffect="non-scaling-stroke"`
> - Over the SVG, a `relative` div with `flex items-center justify-center w-full h-full`:
>   - Inner `text-center px-4` div, color `#154359`:
>     - Title line: `text-[13px] font-semibold leading-tight`
>     - Subtitle line: `text-[12px] font-normal leading-tight opacity-80`
>
> **Bottom fade overlay:**
> - `pointer-events-none absolute inset-x-0 bottom-0 h-40 sm:h-56 z-10`
> - Background: `linear-gradient(to bottom, rgba(240, 245, 247, 0) 0%, rgba(240, 245, 247, 0.7) 60%, #F0F5F7 100%)` -- fades from transparent to the next section's background color
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
> - Section background: `#F0F0F0`
> - Text color: `#154359` (dark teal/navy)
> - Card border stroke: `rgba(6, 99, 119, 0.25)` (muted teal)
> - Bottom gradient target: `#F0F5F7`
>
> **Key design details:**
> - The nomination cards use an SVG polygon border (not CSS border) to create chamfered/cut corners -- top-left and bottom-right corners are clipped at 14% of the viewBox
> - The left and right card columns are pushed down with `lg:mt-36` so they sit roughly mid-way beside the taller center video, creating a staggered visual hierarchy
> - The video has no border-radius and plays inline with no controls
> - The entire section is responsive: stacks vertically on mobile (center first, then left cards, then right cards) and goes to a 3-column layout on `lg:` breakpoint
> - No animations beyond the hover lift (`hover:-translate-y-0.5`) on the nomination cards

---

## Liquid Glass Features — Features [sites/liquid-glass-features]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(10).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/liquid-glass-features.webp

Build a "Features Chess" section for a React + Vite + Tailwind CSS project. This is a single section component with an alternating two-row layout (text left / image right, then image left / text right) on a solid black background with white text and liquid glassmorphism effects.

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
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.45) 0%,
      rgba(255, 255, 255, 0.15) 20%,
      rgba(255, 255, 255, 0) 40%,
      rgba(255, 255, 255, 0) 60%,
      rgba(255, 255, 255, 0.15) 80%,
      rgba(255, 255, 255, 0.45) 100%
    );
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    pointer-events: none;
  }

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

---

### ICON DEPENDENCY

Uses `ArrowUpRight` from `lucide-react`:
```
npm install lucide-react
```

---

### IMAGE ASSETS (External GIF URLs)

Two animated GIFs are used. These are external URLs -- do NOT import them, use them directly as `src` strings:

- **Row 1 (right side)**: `https://motionsites.ai/assets/hero-grow-ai-preview-BlQ8tAQ-.gif`
  Shows an AI-designed website preview with growth/analytics theme
- **Row 2 (left side)**: `https://motionsites.ai/assets/hero-glassmorphism-agency-preview-CGqeRoqP.gif`
  Shows a glassmorphism agency website preview

---

### EXACT COMPONENT CODE

```tsx
import { ArrowUpRight } from "lucide-react";

const FEATURE_1_GIF = "https://motionsites.ai/assets/hero-grow-ai-preview-BlQ8tAQ-.gif";
const FEATURE_2_GIF = "https://motionsites.ai/assets/hero-glassmorphism-agency-preview-CGqeRoqP.gif";

const FeaturesChess = () => {
  return (
    <section className="py-24 px-6 md:px-16 lg:px-24">
      {/* Section header */}
      <div className="text-center mb-20">
        <span className="liquid-glass rounded-full px-3.5 py-1 text-xs font-medium text-white font-body inline-block mb-4">
          Capabilities
        </span>
        <h2 className="text-4xl md:text-5xl lg:text-6xl font-heading italic text-white tracking-tight leading-[0.9]">
          Pro features. Zero complexity.
        </h2>
      </div>

      {/* Row 1: Content left, Image right */}
      <div className="flex flex-col lg:flex-row items-center gap-12 lg:gap-20 mb-24">
        <div className="flex-1 space-y-6">
          <h3 className="text-3xl md:text-4xl font-heading italic text-white leading-[0.9] tracking-tight">
            Designed to convert. Built to perform.
          </h3>
          <p className="text-white/70 font-body font-light leading-relaxed text-sm md:text-base max-w-lg">
            Every pixel is intentional. Our AI studies what works across thousands of top sites—then builds yours to outperform them all.
          </p>
          <button className="liquid-glass-strong rounded-full px-5 py-2.5 text-sm font-medium text-white flex items-center gap-2 hover:bg-white/10 transition-all font-body">
            Learn more
            <ArrowUpRight className="h-4 w-4" />
          </button>
        </div>
        <div className="flex-1">
          <div className="liquid-glass rounded-2xl overflow-hidden">
            <img src={FEATURE_1_GIF} alt="AI-designed website preview" className="w-full h-auto" />
          </div>
        </div>
      </div>

      {/* Row 2: Image left, Content right */}
      <div className="flex flex-col lg:flex-row-reverse items-center gap-12 lg:gap-20">
        <div className="flex-1 space-y-6">
          <h3 className="text-3xl md:text-4xl font-heading italic text-white leading-[0.9] tracking-tight">
            It gets smarter. Automatically.
          </h3>
          <p className="text-white/70 font-body font-light leading-relaxed text-sm md:text-base max-w-lg">
            Your site evolves on its own. AI monitors every click, scroll, and conversion—then optimizes in real time. No manual updates. Ever.
          </p>
          <button className="liquid-glass-strong rounded-full px-5 py-2.5 text-sm font-medium text-white flex items-center gap-2 hover:bg-white/10 transition-all font-body">
            See how it works
            <ArrowUpRight className="h-4 w-4" />
          </button>
        </div>
        <div className="flex-1">
          <div className="liquid-glass rounded-2xl overflow-hidden">
            <img src={FEATURE_2_GIF} alt="Adaptive AI system" className="w-full h-auto" />
          </div>
        </div>
      </div>
    </section>
  );
};

export default FeaturesChess;
```

---

### LAYOUT & RESPONSIVE BEHAVIOR

- **Section padding**: `py-24 px-6 md:px-16 lg:px-24`
- **Header bottom margin**: `mb-20`
- **Row gap**: `gap-12 lg:gap-20`
- **Row 1 spacing from Row 2**: `mb-24`
- **Mobile**: Both rows stack vertically (`flex-col`), content always appears above image
- **Desktop (lg+)**: Row 1 = text left / image right (`lg:flex-row`). Row 2 = text right / image left (`lg:flex-row-reverse`). Each side is `flex-1` (50/50 split).

---

### BADGE PATTERN ("Capabilities" pill)

`liquid-glass rounded-full px-3.5 py-1 text-xs font-medium text-white font-body inline-block mb-4`

Floating pill with barely-visible glass background and thin gradient border from `::before`.

---

### BUTTON PATTERN (CTA buttons)

`liquid-glass-strong rounded-full px-5 py-2.5 text-sm font-medium text-white flex items-center gap-2 hover:bg-white/10 transition-all font-body`

Strong glass variant (50px backdrop blur). `ArrowUpRight` icon at `h-4 w-4`. Hover adds subtle white overlay.

---

### IMAGE CONTAINER PATTERN

`liquid-glass rounded-2xl overflow-hidden` wrapping an `<img>` with `w-full h-auto`. The liquid-glass gives the gradient border treatment; `rounded-2xl overflow-hidden` clips corners.

---

### TYPOGRAPHY DETAILS

| Element | Classes |
|---|---|
| Section badge | `text-xs font-medium text-white font-body` |
| Section heading | `text-4xl md:text-5xl lg:text-6xl font-heading italic text-white tracking-tight leading-[0.9]` |
| Row heading | `text-3xl md:text-4xl font-heading italic text-white leading-[0.9] tracking-tight` |
| Body text | `text-white/70 font-body font-light leading-relaxed text-sm md:text-base max-w-lg` |
| Button text | `text-sm font-medium text-white font-body` |

---

### EXACT TEXT CONTENT

**Section badge**: "Capabilities"
**Section heading**: "Pro features. Zero complexity."

**Row 1 heading**: "Designed to convert. Built to perform."
**Row 1 body**: "Every pixel is intentional. Our AI studies what works across thousands of top sites--then builds yours to outperform them all."
**Row 1 button**: "Learn more"

**Row 2 heading**: "It gets smarter. Automatically."
**Row 2 body**: "Your site evolves on its own. AI monitors every click, scroll, and conversion--then optimizes in real time. No manual updates. Ever."
**Row 2 button**: "See how it works"

---

### PARENT CONTEXT

This section sits on a `bg-black` parent container. No video backgrounds. No animations beyond button hover transitions. The em dash in "top sites--then" is a real `—` (U+2014) character, not two hyphens. The black background is essential for the liquid glass effect to render correctly.

## Max Reed Portfolio — Features [sites/max-reed-portfolio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(6).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/max-reed-portfolio.webp

Build a full-viewport dark personal portfolio Features section using React + TypeScript + Tailwind CSS + lucide-react.

**Layout & Structure:**
- Full screen dark background `#0a0a0a`, white text, Inter font with antialiased smoothing
- Top header row: left side has a heading "Hi, I'm Max Reed!" (size `text-[28px] sm:text-3xl md:text-4xl lg:text-[44px]`, leading `1.15`, font-normal, tracking-tight) followed by a paragraph "A London-based independent creator shaping sharp visual systems, web-ready products, and story-first campaigns. With a decade of craft behind me, I help ideas move with focus and intention." (text-sm md:text-[15px], leading-[1.6], text-white/60, max-w-3xl). Header container has `max-w-3xl`.
- Right side of header: a liquid-glass rounded-full button "Let's Team Up Today" (px-5 sm:px-6, py-2.5 sm:py-3)
- Overall section padding: `px-4 sm:px-6 md:px-10 lg:px-14 py-6 sm:py-8 md:py-10`, full screen `lg:h-screen`

**Grid (3 columns on lg, 2 on md, 1 on mobile, gap-4 md:gap-5):**

**Column 1 - Background card (rounded-2xl, bg-black):**
- Background video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260507_150203_44a5bd32-516a-47ce-a077-8acbf9aa8991.mp4` (autoPlay loop muted playsInline, absolute inset-0 object-cover)
- Top: centered "BACKGROUND" section label (uppercase, tracking-[0.22em], text-[11px], text-white/70) with Sparkle icons on each side (h-3 w-3, strokeWidth 1.5)
- Bottom: career timeline as a 4-col grid `[auto_auto_1fr_auto]`:
  - 2023-Now · Freelance Creative · Solo Studio
  - 2020-2023 · Head of Brand Design · Rove Studio
  - 2017-2020 · Visual Stylist · Ember Works
  - Separator between year and role is a Sparkle icon (h-3 w-3, text-white/60)

**Column 2 (stacked rows, md:grid-rows-[auto_1fr]):**

Top - Client Voice card (rounded-2xl, bg-[#324444], p-5 md:p-6, with noise-overlay):
- Left-aligned "CLIENT VOICE" label with Sparkle icons (justify-start)
- Quote: "Max reshaped our image with a degree of finesse and vision that surpassed what we'd hoped for. The process felt graceful, and the outcomes speak for themselves." (text-[13px] sm:text-[13.5px], leading-[1.6], text-white/85)
- Attribution: **Elena Brooks**, Creative Director — Halcyon

Bottom - 10M+ card (rounded-2xl, bg-black):
- Background video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260507_154543_d5b83fc1-9cea-44f3-b5e8-8f325935211a.mp4`
- Centered huge text "10M+" (text-5xl sm:text-6xl md:text-7xl lg:text-[88px], font-light, tracking-tight, drop-shadow)
- Bottom caption "Raised for startups" (centered, text-white/85)

**Column 3 (stacked):**

Top - Daily Software card (rounded-2xl, bg-black):
- Background video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260507_153148_d7a3e1dd-e5d0-4ce6-8306-00d7522ecc44.mp4`
- Top: "DAILY SOFTWARE" section label
- Bottom: two scrolling marquee rows of liquid-glass icon tiles (h-14 w-14 md:h-16 md:w-16, rounded-xl). Row 1 scrolls left with icons [Figma, Framer, Palette, PenTool, Layers, Type, Aperture, Chrome]. Row 2 scrolls right with icons [Camera, Brush, Box, Wand2, Figma, Framer, Type, Layers]. Each row duplicated for seamless loop. Mask fade on both edges with `[mask-image:linear-gradient(to_right,transparent,black_8%,black_92%,transparent)]`.

Bottom - Reach Me card (rounded-2xl, bg-[#324444], p-5 md:p-6, noise-overlay):
- "REACH ME" section label
- Email: hi@maxreed.com
- Phone: +44 207 81 63
- Top-right ArrowUpRight icon button (h-9 w-9 rounded-full)

**Custom CSS in index.css:**

```css
.liquid-glass {
  background: rgba(255, 255, 255, 0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
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
  background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}

@keyframes marquee-left { from { transform: translateX(0); } to { transform: translateX(-50%); } }
@keyframes marquee-right { from { transform: translateX(-50%); } to { transform: translateX(0); } }
.animate-marquee-left { animation: marquee-left 22s linear infinite; }
.animate-marquee-right { animation: marquee-right 26s linear infinite; }

.noise-overlay::after {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: 0.55;
  mix-blend-mode: soft-light;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='matrix' values='0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1 0'/></filter><rect width='100%25' height='100%25' filter='url(%23n)'/></svg>");
  background-size: 240px 240px;
}
```

Font: Inter (system fallback). Icons from lucide-react: ArrowUpRight, Sparkle, Figma, Framer, Palette, PenTool, Layers, Type, Aperture, Chrome, Camera, Brush, Box, Wand2. All icons use strokeWidth 1.5.

## NexaCore Control — Features [sites/nexacore-control]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(33).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexacore-control.webp

Build a single React + TypeScript + Tailwind CSS v3 component called `FreedomSection`. It uses `hls.js` for HLS video streaming and `useEffect` / `useRef` from React. No external icon libraries — all icons are inline SVG or `<img>` tags. Fully mobile-responsive. No hover states.

---

### Global font

Register **"Mazzard H"** in `index.css` and apply it globally:

```css
@font-face {
  font-family: 'Mazzard H';
  font-weight: 400;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.ttf') format('truetype');
}
@font-face {
  font-family: 'Mazzard H';
  font-weight: 500;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.ttf') format('truetype');
}

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html, body, * { font-family: 'Mazzard H', sans-serif; }
}
```

---

### Dependencies

`hls.js` must be installed: `npm install hls.js`. It is imported as `import Hls from 'hls.js'`.

---

### File: `src/components/FreedomSection.tsx`

### Constants (top of file)

```ts
const HLS_SRC = 'https://stream.mux.com/bnYL6x5cAX6WiJv2pOKpITehZd3NVdXpj3ylJFpX5Lk.m3u8';

const CROSS_ICON = 'https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/686cc0f520a992816d8b15dc_bullet-list-cross.svg';
const CHECK_ICON = 'https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/686cc068490683bbb3377d04_bullet-list.svg';

const negatives = [
  'Reactive firefighting when foundational issues surface too late',
  'Bloated coordination overhead drains bandwidth from core teams',
  "Constant re-verification because source data can't be trusted",
  'Fragmented vendor relations produce mismatched deliverables',
  'Scattered specs and decisions buried across siloed systems',
];

const positives = [
  'Layered dependency maps eliminate costly surprises at every phase',
  'Streamlined team handoffs deliver production-ready outcomes fast',
  'Live validation loops keep requirements locked across all stages',
  'Unified vendor management through a single accountable contact',
  'Centralized context and clear records accelerate every decision',
];
```

---

### `HlsVideo` sub-component (defined above `FreedomSection`, not exported)

```ts
function HlsVideo() {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    if (Hls.isSupported()) {
      const hls = new Hls({
        startLevel: -1,
        capLevelToPlayerSize: false,
        maxMaxBufferLength: 60,
        enableWorker: true,
      });
      hls.loadSource(HLS_SRC);
      hls.attachMedia(video);
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        hls.currentLevel = hls.levels.length - 1;
        video.play().catch(() => {});
      });
      return () => hls.destroy();
    } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
      video.src = HLS_SRC;
      video.play().catch(() => {});
    }
  }, []);

  return (
    <video
      ref={videoRef}
      autoPlay
      loop
      muted
      playsInline
      style={{
        width: '160%',
        height: '160%',
        objectFit: 'cover',
        position: 'absolute',
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
      }}
    />
  );
}
```

The video is zoomed to `160% x 160%` and centered with `translate(-50%, -50%)` inside a circular clipping container, so it fills the circle with no letterboxing.

---

### `FreedomSection` component

**`<section>`** — Tailwind: `w-full flex flex-col items-center`

Inline:
```
background-color: #ffffff
padding: clamp(48px, 6vw, 80px) clamp(16px, 3vw, 40px)
gap: 36px
```

---

### Block 1 — Header

Tailwind: `flex flex-col items-center gap-9 text-center`

**Badge pill:**

Tailwind: `flex items-center gap-2 text-lg font-medium rounded-full`

Inline: `background-color: rgb(249, 249, 249)`, `padding: 0.9vw 1.25vw`, `color: rgb(26, 11, 84)`

Contains this inline SVG (`width: 19px`, `height: 18px`, `flex-shrink: 0`, `viewBox="0 0 17 16"`, `fill="none"`, `xmlns="http://www.w3.org/2000/svg"`):

```xml
<g clipPath="url(#freedom-clip)">
  <path
    fillRule="evenodd"
    clipRule="evenodd"
    d="M8.50037 3.66955C7.53221 2.82462 6.41758 2.275 5.333 2.07887C4.11096 1.85888 2.84987 2.0826 1.96658 2.95885C1.10056 3.81944 0.866218 5.04172 1.06751 6.23193C1.24778 7.29835 1.7803 8.39907 2.60501 9.35959C2.41536 10.1071 2.46371 10.8946 2.7434 11.6137C3.02308 12.3327 3.52035 12.9481 4.16678 13.375C4.81321 13.802 5.57702 14.0195 6.35308 13.9976C7.12915 13.9758 7.87933 13.7157 8.50037 13.2531C9.12146 13.7161 9.87183 13.9765 10.6482 13.9985C11.4245 14.0205 12.1886 13.8029 12.8352 13.3758C13.4819 12.9487 13.9792 12.3331 14.2588 11.6137C14.5384 10.8943 14.5865 10.1065 14.3965 9.35884C15.2204 8.39832 15.753 7.29835 15.9325 6.23119C16.1338 5.04098 15.8994 3.81944 15.0334 2.9596C14.1501 2.0826 12.889 1.85888 11.667 2.07962C10.5824 2.275 9.46854 2.82537 8.50037 3.66955Z"
    fill="rgb(200, 111, 255)"
  />
</g>
<defs>
  <clipPath id="freedom-clip">
    <rect width="16" height="16" fill="white" transform="translate(0.5)" />
  </clipPath>
</defs>
```

After the SVG: plain text **"Control"**

**`<h2>`** — Tailwind: `font-medium`

Inline: `font-size: clamp(32px, 4vw, 56px)`, `color: rgb(26, 11, 84)`, `line-height: 1.15`, `margin: 0`

Structure:
```
Stop absorbing the chaos.<br />
<span gradient>Run with confidence.</span>
```

Gradient `<span>` inline styles:
```
background-image: linear-gradient(90deg, rgb(43,167,255), rgb(202,69,255) 50%, rgb(254,136,27))
-webkit-background-clip: text
background-clip: text
-webkit-text-fill-color: transparent
color: transparent
padding-bottom: 0.3vw
display: inline-block
```

---

### Block 2 — Three-column grid

Tailwind: `w-full flex flex-col lg:grid`

Inline:
```
grid-template-columns: 26vw 1fr 26vw
column-gap: 36px
row-gap: 24px
align-items: start
padding: 0 clamp(0px, 2.92vw, 40px)
gap: 24px
```

On mobile (`flex flex-col`): stacks vertically. On `lg:` and above: renders as 3-column grid with `gridTemplateColumns: '26vw 1fr 26vw'`.

---

### Left column — Negatives

Tailwind: `flex flex-col`

Inline: `gap: 12px`, `font-size: clamp(13px, 1.15vw, 17px)`, `color: rgb(131, 121, 158)`

Map over `negatives`. Each card `<div>` — Tailwind: `flex flex-col`

Inline:
```
gap: 12px
padding: clamp(12px, 0.97vw, 16px) clamp(14px, 1.25vw, 20px)
border-radius: 18px
background-color: rgb(255, 255, 255)
box-shadow: 0 3px 9.1px #3f4a7e0d, 0 1px 29px #3f4a7e1a
```

Contents:
1. `<img src={CROSS_ICON} alt="" aria-hidden style={{ width: 'clamp(16px, 1.25vw, 20px)', flexShrink: 0 }} />`
2. `<div>{text}</div>` — inherits parent `color: rgb(131, 121, 158)`

---

### Center column — Video circle

Tailwind: `flex items-center justify-center order-first lg:order-none`

Inline: `align-self: center`

On mobile, `order-first` places the video above both card columns. On `lg:`, `lg:order-none` restores it to the middle.

Inside, a circular container:
```
position: relative
border-radius: 50%
overflow: hidden
width: clamp(200px, 22vw, 400px)
height: clamp(200px, 22vw, 400px)
flex-shrink: 0
```

Inside the circle: `<HlsVideo />` (described above — the `<video>` is absolutely positioned at 160% size, centered with translate -50% -50%).

---

### Right column — Positives

Tailwind: `flex flex-col`

Inline: `gap: 12px`, `font-size: clamp(13px, 1.15vw, 17px)`

Map over `positives`. Each card `<div>` — Tailwind: `flex flex-col`

Inline: (same shadow/padding/border-radius as negatives)
```
gap: 12px
padding: clamp(12px, 0.97vw, 16px) clamp(14px, 1.25vw, 20px)
border-radius: 18px
background-color: rgb(255, 255, 255)
box-shadow: 0 3px 9.1px #3f4a7e0d, 0 1px 29px #3f4a7e1a
```

Contents:
1. `<img src={CHECK_ICON} alt="" aria-hidden style={{ width: 'clamp(16px, 1.25vw, 20px)', flexShrink: 0 }} />`
2. `<div style={{ color: 'rgb(26, 11, 84)' }}>{text}</div>`

---

### Layout summary

- **Mobile**: flex-col — video first (order-first), then left negatives, then right positives stacked vertically
- **Desktop (lg+)**: CSS grid — left negatives | center video circle | right positives
- Section background is pure white `#ffffff`
- No animations, no hover states, no scroll effects

## NexaCore Results — Features [sites/nexacore-results]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(40).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexacore-results.webp

Build a single React + TypeScript + Tailwind CSS v3 component called `PrecisionSection`. No external icon libraries — all icons are inline SVG or `<img>` tags. No `useState`, no animations, no hover states. Two separate layouts: a desktop staircase (absolutely-positioned pillars) and a mobile alternating-flow layout. The `sm:` breakpoint controls visibility between them.

---

### Global font

Register **"Mazzard H"** in `index.css` and apply it globally:

```css
@font-face {
  font-family: 'Mazzard H';
  font-weight: 400;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.ttf') format('truetype');
}
@font-face {
  font-family: 'Mazzard H';
  font-weight: 500;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.ttf') format('truetype');
}

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html, body, * { font-family: 'Mazzard H', sans-serif; }
}
```

---

### File: `src/components/PrecisionSection.tsx`

### Constants (top of file, before the component)

```ts
const LOGO_ICON =
  'https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/6870f623cf3df417ce45df05_icon%20logo%20eternacloud.png';

const LINE_GRADIENT =
  'linear-gradient(rgb(28, 78, 255), rgb(254, 136, 27) 0%, rgb(172, 36, 255) 25%, rgb(247, 159, 255) 50%, rgb(255, 214, 0) 66%, rgb(254, 136, 27) 84%, rgba(254, 136, 27, 0) 102%)';

const PILLARS = [
  { label: 'Scopes',     items: ['conditions', 'capacity', 'specs', 'timelines'],     leftVw: 2.8,  bottomVw: 7     },
  { label: 'Integrates', items: ['civil', 'mechanical', 'electrical', 'controls'],     leftVw: 22.4, bottomVw: 9.08  },
  { label: 'Certifies',  items: ['redundancy', 'testing', 'compliance', 'sign-offs'], leftVw: 41.2, bottomVw: 11.16 },
  { label: 'Activates',  items: ['cutover', 'runbooks', 'handoff', 'SLAs'],           leftVw: 61.1, bottomVw: 13.24 },
];
```

---

### Section element

Inline styles only (no Tailwind on the `<section>` itself):

```
background-image: url("https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260418_125638_553b96dc-a1fd-4b2b-81a9-ed7daa80006e.png&w=1280&q=85")
background-size: cover
background-position: center
background-repeat: no-repeat
width: 100%
display: flex
flex-direction: column
align-items: center
text-align: center
padding: clamp(48px, 8vw, 120px) clamp(16px, 4vw, 60px) clamp(48px, 5.56vw, 80px)
gap: clamp(32px, 4vw, 56px)
```

---

### Block 1 — Header

Wrapper `<div>` — inline: `display: flex`, `flex-direction: column`, `align-items: center`, `gap: 36px`

### Badge pill `<div>`

Inline:
```
background-color: rgb(249, 249, 249)
display: flex
align-items: center
gap: 8px
font-size: clamp(14px, 1.1vw, 18px)
font-weight: 500
border-radius: 36px
padding: clamp(8px, 0.9vw, 14px) clamp(12px, 1.25vw, 20px)
color: rgb(26, 11, 84)
white-space: nowrap
```

Contains this inline SVG (`width: 19`, `height: 18`, `flex-shrink: 0`, `viewBox="0 0 17 16"`, `fill="none"`):

```xml
<g clipPath="url(#prec-clip)">
  <circle cx="8.5" cy="8" r="7" stroke="#c86fff" fill="none" />
  <path d="M9.5 11.5V10.5H7.5V11.5H9.5ZM7.5 14.5C7.5 15.0523 7.94772 15.5 8.5 15.5C9.05228 15.5 9.5 15.0523 9.5 14.5H7.5ZM8.5 11.5H7.5V14.5H8.5H9.5V11.5H8.5Z" fill="rgb(200, 111, 255)" />
  <path d="M12 7H11V9H12V7ZM15 9C15.5523 9 16 8.55228 16 8C16 7.44772 15.5523 7 15 7V9ZM12 8V9H15V8V7L12 7V8Z" fill="rgb(200, 111, 255)" />
  <path d="M5 9H6V7H5V9ZM2 7C1.44772 7 1 7.44772 1 8C1 8.55228 1.44772 9 2 9V7ZM5 8V7H2V8V9H5V8Z" fill="rgb(200, 111, 255)" />
  <path d="M7.5 4.5V5.5H9.5V4.5H7.5ZM9.5 1.5C9.5 0.947715 9.05228 0.5 8.5 0.5C7.94772 0.5 7.5 0.947715 7.5 1.5H9.5ZM8.5 4.5H9.5V1.5H8.5H7.5V4.5H8.5Z" fill="rgb(200, 111, 255)" />
</g>
<defs>
  <clipPath id="prec-clip">
    <rect width="16" height="16" fill="white" transform="translate(0.5)" />
  </clipPath>
</defs>
```

After the SVG: plain text **"Structured Delivery"**

### Heading + subtext `<div>`

Inline: `display: flex`, `flex-direction: column`, `align-items: center`, `max-width: clamp(700px, 60vw, 900px)`, `gap: 22px`

`<h2>` — inline: `font-size: clamp(28px, 4vw, 56px)`, `font-weight: 500`, `color: rgb(26, 11, 84)`, `line-height: 1.15`, `margin: 0`

Inside the `<h2>`, two `<span>` elements:

**Span 1** — Tailwind class `sm:whitespace-nowrap`, inline `display: block`:
> **One integrated, end-to-end system.**

**Span 2** — inline only:
```
background-image: linear-gradient(90deg, rgb(43, 167, 255), rgb(202, 69, 255) 50%, rgb(254, 136, 27))
-webkit-background-clip: text
background-clip: text
-webkit-text-fill-color: transparent
color: transparent
padding-bottom: 0.3vw
display: block
```
> **Compounding operational value.**

`<p>` below heading — inline: `font-size: clamp(15px, 1.2vw, 20px)`, `color: rgb(169, 151, 206)`, `margin: 0`
> **"NexaCore teams capture, align, validate and deliver exactly what keeps your programs on track."**

---

### Block 2 — Pillars container `<div>`

Inline: `width: 100%`, `max-width: 82.292vw`, `margin: 0 auto`

Contains two children:

---

### Desktop pillars — `hidden sm:block` (Tailwind)

Inline:
```
position: relative
width: 82.292vw
height: 31.94vw
color: rgb(26, 11, 84)
```

Map over `PILLARS`. Each pillar wrapper `<div>`:
```
position: absolute
bottom: `${pillar.bottomVw}vw`
left: `${pillar.leftVw}vw`
display: flex
flex-direction: column
align-items: center
justify-content: flex-start
```

**Chip `<div>`:**
```
display: flex
align-items: center
justify-content: center
background-image: linear-gradient(135deg, rgb(255, 255, 255), rgba(255, 255, 255, 0.6))
font-size: 18px
font-weight: 500
border-radius: 20px
padding-top: 0.972vw
padding-bottom: 0.972vw
padding-left: 1.736vw
padding-right: 1.736vw
white-space: nowrap
gap: 8px
```

Chip contents:
1. `<img src={LOGO_ICON} alt="" style={{ width: '1.111vw', height: 'auto', display: 'inline-block' }} />`
2. `{pillar.label}`

**Line + items wrapper `<div>`** (directly below chip):
```
position: relative
display: flex
flex-direction: column
align-items: center
justify-content: flex-end
```

**Items container** (absolutely positioned, overlays the line):
```
position: absolute
top: 0.56vw
left: 1.94vw
display: flex
flex-direction: column
gap: 4px
font-size: 16px
align-items: flex-start
justify-content: space-between
```

Each item `<div>`:
```
padding-top: 0.69vw
padding-bottom: 0.69vw
padding-left: 1.04vw
padding-right: 1.04vw
display: flex
align-items: flex-start
```
Text: the item string.

**Vertical gradient line `<div>`** (sibling of items container, rendered after it):
```
background-image: LINE_GRADIENT  (see constant above)
width: 1px
height: 14.24vw
```

---

### Mobile pillars — Tailwind: `flex flex-col sm:hidden w-full`

Inline: `color: rgb(26, 11, 84)`, `gap: 0`

Map over `PILLARS` with index. `isRight = index % 2 !== 0` (index 1 and 3 are right-aligned).

**Pillar wrapper `<div>`:**
```
display: flex
flex-direction: column
align-items: isRight ? 'flex-end' : 'flex-start'
width: 100%
padding-bottom: 8px
```

**Chip `<div>`:**
```
display: inline-flex
align-items: center
background-image: linear-gradient(135deg, rgb(255, 255, 255), rgba(255, 255, 255, 0.6))
font-size: 15px
font-weight: 500
border-radius: 20px
padding: 10px 18px
white-space: nowrap
gap: 7px
```

Chip contents:
1. `<img src={LOGO_ICON} alt="" style={{ width: 16, height: 'auto' }} />`
2. `{pillar.label}`

**Line + items row `<div>`:**
```
display: flex
flex-direction: isRight ? 'row-reverse' : 'row'
align-items: stretch
width: 100%
```

**Vertical line `<div>`:**
```
width: 1px
flex-shrink: 0
background-image: LINE_GRADIENT
margin-left: isRight ? 0 : 22px
margin-right: isRight ? 22px : 0
min-height: 120px
```

**Items `<div>`:**
```
display: flex
flex-direction: column
gap: 0
padding-left: isRight ? 0 : 20px
padding-right: isRight ? 20px : 0
padding-top: 8px
padding-bottom: 8px
align-items: isRight ? 'flex-end' : 'flex-start'
```

Each item `<div>`:
```
font-size: 14px
color: rgb(100, 80, 160)
padding: 8px 0
```
Text: the item string.

---

### Pillar data reference

| Label | Items | Desktop left | Desktop bottom |
|---|---|---|---|
| Scopes | conditions, capacity, specs, timelines | 2.8vw | 7vw |
| Integrates | civil, mechanical, electrical, controls | 22.4vw | 9.08vw |
| Certifies | redundancy, testing, compliance, sign-offs | 41.2vw | 11.16vw |
| Activates | cutover, runbooks, handoff, SLAs | 61.1vw | 13.24vw |

---

**No animations. No hover states. No scroll effects. No JavaScript logic. Static render only. Desktop: 4 pillars arranged in a rising staircase via `position: absolute` with `bottom` and `left` in `vw` units. Mobile: single column, even-indexed pillars align left, odd-indexed align right, each with a vertical gradient line beside its items list.**

## Nike Hover — Features [sites/nike-hover]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(41).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nike-hover.webp

Create a single full-viewport (`h-[100dvh]`) Nike-branded section in React + Tailwind CSS + GSAP. It must be **fully mobile responsive**. The app requires `react-player` and `gsap` installed via npm.

---

### 1. Dependencies to Install
Install `react-player` and `gsap`.

### 2. Globals & Configuration (`src/index.css`)
Replace `index.css` with this exact Tailwind v4 and Google Fonts configuration:
```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Manrope:wght@400;500;600;700&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Manrope", sans-serif;
  --font-serif: "Instrument Serif", serif;
}
```

### 3. SpotlightReveal Component (`src/components/SpotlightReveal.tsx`)

An interactive cursor-following SVG spotlight mask. The user's mouse reveals a hidden video layer underneath a static image overlay. On mobile/touch devices, falls back to touch tracking.

```tsx
import { useEffect, useRef } from 'react';

interface SpotlightRevealProps {
  imageSrc: string;
  videoSrc: string;
  isPlaying?: boolean;
  baseRadius?: number;
}

export default function SpotlightReveal({
  imageSrc,
  videoSrc,
  isPlaying = true,
  baseRadius = 420,
}: SpotlightRevealProps) {
  const NUM_TRAILS = 6;
  const videoRef = useRef<HTMLVideoElement>(null);
  const pointsRef = useRef(
    Array.from({ length: NUM_TRAILS }, () => ({ x: -1000, y: -1000 }))
  );

  useEffect(() => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.play().catch(() => {});
      } else {
        videoRef.current.pause();
      }
    }
  }, [isPlaying]);

  useEffect(() => {
    let targetX = window.innerWidth / 2,
      targetY = window.innerHeight / 2;
    const handleMouseMove = (e: MouseEvent) => {
      targetX = e.clientX;
      targetY = e.clientY;
    };
    window.addEventListener('mousemove', handleMouseMove);

    let animationFrameId: number;
    const animate = () => {
      const points = pointsRef.current;
      points[0].x += (targetX - points[0].x) * 0.2;
      points[0].y += (targetY - points[0].y) * 0.2;
      for (let i = 1; i < points.length; i++) {
        points[i].x += (points[i - 1].x - points[i].x) * 0.35;
        points[i].y += (points[i - 1].y - points[i].y) * 0.35;
      }
      for (let i = 0; i < points.length; i++) {
        const circle = document.getElementById(`trail-${i}`);
        if (circle) {
          circle.setAttribute('cx', points[i].x.toString());
          circle.setAttribute('cy', points[i].y.toString());
        }
      }
      animationFrameId = requestAnimationFrame(animate);
    };
    animate();
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      cancelAnimationFrame(animationFrameId);
    };
  }, []);

  return (
    <div className="absolute inset-0 w-full h-full z-0 bg-black pointer-events-none overflow-hidden flex items-center justify-center">
      <div className="absolute inset-0 w-full h-full flex items-center justify-center overflow-hidden pointer-events-none">
        <video
          ref={videoRef}
          src={videoSrc}
          className="absolute inset-0 w-full h-full object-cover"
          muted
          loop
          playsInline
        />
      </div>
      <svg
        className="absolute inset-0 w-full h-full"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <radialGradient id="holeGradient">
            <stop offset="0%" stopColor="black" stopOpacity="1" />
            <stop offset="60%" stopColor="black" stopOpacity="0.8" />
            <stop offset="100%" stopColor="black" stopOpacity="0" />
          </radialGradient>
          <mask
            id="spotlight-mask"
            maskContentUnits="userSpaceOnUse"
            x="0"
            y="0"
            width="100%"
            height="100%"
          >
            <rect width="100%" height="100%" fill="white" />
            {Array.from({ length: NUM_TRAILS })
              .reverse()
              .map((_, reversedIndex) => {
                const i = NUM_TRAILS - 1 - reversedIndex;
                return (
                  <circle
                    key={`trail-${i}`}
                    id={`trail-${i}`}
                    cx="-1000"
                    cy="-1000"
                    r={baseRadius - i * 35}
                    fill="url(#holeGradient)"
                    opacity={1 - i * 0.15}
                  />
                );
              })}
          </mask>
        </defs>
        <image
          href={imageSrc}
          width="100%"
          height="100%"
          preserveAspectRatio="xMidYMid slice"
          mask="url(#spotlight-mask)"
        />
      </svg>
    </div>
  );
}
```

**How it works:**
- A `<video>` plays fullscreen behind everything.
- An SVG `<image>` is overlaid on top, masked by a radial gradient mask.
- 6 trail circles follow the cursor with easing (leader at 0.2 lerp, followers at 0.35 lerp). Where the circles are, the mask cuts a hole revealing the video underneath.
- `baseRadius` controls the spotlight size (default 420 for section 1, 520 for this section 2).
- `isPlaying` toggles video play/pause via hover zones defined in the parent.

---

### 4. Section 2 Layout (`src/App.tsx`)

**Exact assets:**
- **Image overlay (static):** `https://github.com/dsMagnatov/Acreage-landing-assets/blob/main/02604201313.png?raw=true`
- **Video (revealed on hover):** `https://pikaso.cdnpk.net/private/production/4024859125/d070ae9c-55df-47aa-acbe-4ee66337855c-0.mp4?token=exp=1777075200~hmac=4202c1d0ec90137eb6dffa8e0db93ed7569a68b2016165d8b1b567f888869ff5`
- **SpotlightReveal baseRadius:** `520`

**Section container:**
```tsx
<section
  className="relative z-10 w-full h-[100dvh] overflow-hidden bg-black text-white"
  style={{ boxShadow: '0 -20px 50px rgba(0,0,0,0.5)' }}
>
```
- Full viewport height, black background, white text, top inset shadow for depth when scrolling.

**Element 1 -- SpotlightReveal background:**
```tsx
<SpotlightReveal
  imageSrc="https://github.com/dsMagnatov/Acreage-landing-assets/blob/main/02604201313.png?raw=true"
  videoSrc="https://pikaso.cdnpk.net/private/production/4024859125/d070ae9c-55df-47aa-acbe-4ee66337855c-0.mp4?token=exp=1777075200~hmac=4202c1d0ec90137eb6dffa8e0db93ed7569a68b2016165d8b1b567f888869ff5"
  isPlaying={isSecondVideoPlaying}
  baseRadius={520}
/>
```

**Element 2 -- Two invisible hover trigger zones (toggle video play/pause):**
```tsx
{/* Right-side hover zone */}
<div
  className="absolute right-[calc(8%+100px)] bottom-[12%] w-[calc(50%-50px)] h-[calc(50%+230px)] z-30"
  onMouseEnter={() => setIsSecondVideoPlaying(true)}
  onMouseLeave={() => setIsSecondVideoPlaying(false)}
/>
{/* Left-side hover zone */}
<div
  className="absolute left-[calc(8%+200px)] top-[calc(20%+190px)] w-[calc(15%+250px)] h-[calc(22.5%+130px)] -translate-y-full z-30"
  onMouseEnter={() => setIsSecondVideoPlaying(true)}
  onMouseLeave={() => setIsSecondVideoPlaying(false)}
/>
```
These are transparent interactive areas that trigger the video. Make them responsive: on mobile, simplify to a single full-width touch zone or auto-play the video.

**Element 3 -- Stats card (top-left area):**
Positioned `absolute left-[calc(8%+200px)] top-[20%] z-20`. Width `320px`. Glassmorphism card with:
- `background: rgba(0, 0, 0, 0.16)`, `backdrop-filter: blur(80px)`, `border: 1px solid rgba(255,255,255,0.1)`, `border-radius: 2px (rounded-sm)`.
- Padding: `px-8 py-6`.

Card contents:
1. **Big stat:** `78%` in `font-serif italic`, color `#DA3A16`, size `72px`, `leading-[80px]`, `tracking-tight`.
2. **Inline SVG chart** next to the stat (inside a `w-[11px]` wrapper, but the SVG itself is `width: 160px, height: 80px`). The chart is a wavy line in `#DA3A16` with a drop shadow filter in the same orange-red color. Exact SVG path:
```svg
<svg style="width:160px;height:80px" viewBox="0 0 289 138" fill="none" xmlns="http://www.w3.org/2000/svg">
  <g filter="url(#filter0_d_878_28499)">
    <path d="M22.5 48.7306C39.7833 48.7306 49.34 54.94 63.1667 69.2965C76.9933 83.653 86.55 110.5 103.833 110.5C121.117 110.5 130.673 84.2876 144.5 59.2856C158.327 34.2837 167.883 19.5573 185.167 19.5573C202.45 19.5573 208.55 57.6673 225.833 57.6673C243.117 57.6673 249.217 19.5 266.5 19.5" stroke="#DA3A16" stroke-width="2"/>
  </g>
  <defs>
    <filter id="filter0_d_878_28499" x="0" y="0" width="289" height="138" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
      <feFlood flood-opacity="0" result="BackgroundImageFix"/>
      <feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
      <feOffset dy="4"/>
      <feGaussianBlur stdDeviation="11.25"/>
      <feComposite in2="hardAlpha" operator="out"/>
      <feColorMatrix type="matrix" values="0 0 0 0 0.854902 0 0 0 0 0.227451 0 0 0 0 0.0862745 0 0 0 1 0"/>
      <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_878_28499"/>
      <feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_878_28499" result="shape"/>
    </filter>
  </defs>
</svg>
```
3. **Title:** `"NEXT-GEN CUSHIONING ARCHITECTURE"` -- `font-serif`, white, `text-[15px]`, `tracking-[0.02em]`, uppercase, `mb-2`, `leading-tight`.
4. **Subtitle:** `"Impact Absorption & Energy Return Dynamics"` -- `font-serif`, `text-white/60`, `text-[13px]`.

On mobile: reposition this card to `left-4 top-[15%]` or `top-auto bottom-[55%]`, reduce width to `w-[280px]`, scale the stat to `text-[48px]`.

**Element 4 -- Hero headline (bottom-left):**
Positioned `absolute left-[8%] bottom-[12%] z-20`, `max-w-[500px]`.

```html
<h2 class="text-[44px] leading-[1.05] tracking-tight flex flex-col">
  <span class="font-sans font-medium">Bringing Aerospace-</span>
  <span class="font-sans font-medium">Grade Infrastructure</span>
  <span class="font-serif font-normal pt-1">
    <span class="not-italic">Directly To Your </span>
    <span class="italic">Everyday</span>
  </span>
  <span class="font-serif italic font-normal">Urban Exploration</span>
</h2>
```

- Lines 1-2 use `font-sans` (Manrope) `font-medium`.
- Lines 3-4 use `font-serif` (Instrument Serif). Line 3 mixes non-italic "Directly To Your" with italic "Everyday". Line 4 is fully italic.
- On mobile: reduce to `text-[24px] sm:text-[32px] md:text-[44px]`, position `left-4 bottom-[8%]`, `max-w-[90%]`.

**Element 5 -- Nike branded CTA block (bottom-right):**
Positioned `absolute right-[calc(8%+100px)] bottom-[12%] z-20`, stacked vertically with `flex flex-col items-center`.

Two stacked boxes, each `w-[180px]`:
1. **Top box (white):** `bg-white`, `py-[6px]`, centered text: `"THE SCIENCE OF IMPACT CONTROL"` in `text-black font-serif text-[10px] uppercase font-bold tracking-[0.08em] leading-[16px]`.
2. **Bottom box (Nike red):** `bg-[#DA3A16]`, `h-[100px]`, centered Nike swoosh SVG in white, `width="86"`. Exact swoosh path:
```svg
<svg width="86" viewBox="135.5 361.38 420.32 149.8" fill="white" xmlns="http://www.w3.org/2000/svg">
  <path d="m181.86 511.11c-12.524-0.49755-22.77-3.9244-30.782-10.289-1.529-1.2159-5.1725-4.8616-6.3949-6.3992-3.2489-4.0853-5.4578-8.0611-6.931-12.472-4.5334-13.579-2.2002-31.397 6.6737-50.953 7.5979-16.742 19.322-33.347 39.776-56.344 3.013-3.384 11.986-13.281 12.043-13.281 0.0216 0-0.46749 0.84706-1.083 1.8786-5.3183 8.9082-9.8689 19.401-12.348 28.485-3.9823 14.576-3.502 27.085 1.4068 36.784 3.3862 6.6822 9.1913 12.47 15.719 15.67 11.428 5.5993 28.159 6.0625 48.592 1.3554 1.4068-0.32599 71.116-18.831 154.91-41.123 83.794-22.294 152.36-40.52 152.37-40.505 0.0237 0.0193-194.68 83.333-295.75 126.56-16.007 6.8431-20.287 8.5715-27.812 11.214-19.236 6.7551-36.467 9.9783-50.396 9.4251z"/>
</svg>
```

On mobile: reposition to `right-4 bottom-[8%]`, reduce width to `w-[140px]`, reduce box height to `h-[80px]`.

---

### 5. Color Palette
| Token | Value | Usage |
|---|---|---|
| Background | `#000000` | Section bg |
| Nike Red/Orange | `#DA3A16` | Stat text, chart stroke, chart glow shadow, Nike logo box |
| Text primary | `#FFFFFF` | Headlines, card title |
| Text muted | `rgba(255,255,255,0.6)` | Card subtitle (`text-white/60`) |
| Card bg | `rgba(0,0,0,0.16)` | Glassmorphism card |
| Card border | `rgba(255,255,255,0.1)` | Card border |
| CTA top box | `#FFFFFF` bg / `#000000` text | Label box |

### 6. Typography Rules
| Element | Font | Weight | Size | Style |
|---|---|---|---|---|
| Headline lines 1-2 | Manrope (`font-sans`) | 500 (medium) | 44px | Normal |
| Headline lines 3-4 | Instrument Serif (`font-serif`) | 400 (normal) | 44px | Italic (mixed on line 3) |
| Stat number | Instrument Serif | 400 | 72px | Italic |
| Card title | Instrument Serif | 400 | 15px | Normal, uppercase |
| Card subtitle | Instrument Serif | 400 | 13px | Normal |
| CTA label | Instrument Serif | 700 (bold) | 10px | Normal, uppercase |

### 7. Mobile Responsive Requirements

Implement these breakpoints:
- **< 640px (mobile):** Stack elements vertically. Stats card moves to top-center with reduced dimensions. Headline drops to `text-[24px]` at `left-4 bottom-[30%]`. Nike CTA block moves to center-bottom. Hover zones become a single full-area touch zone. Consider auto-playing the video on mobile since there's no hover. Reduce `baseRadius` to `280` on mobile.
- **640px-1024px (tablet):** Stats card shifts to `left-[5%] top-[18%]`, headline to `text-[32px]`. CTA block to `right-[5%]`.
- **> 1024px (desktop):** Use the exact desktop positions described above unchanged.

### 8. State Management
```tsx
const [isSecondVideoPlaying, setIsSecondVideoPlaying] = useState(false);
```
Controlled by the invisible hover zones. On mobile, default to `true` (auto-play) or use `onTouchStart`/`onTouchEnd`.

---

## Benefits Features — Features Section [sites/benefits-features]

- Preview: https://motionsites.ai/assets/features-benefits-preview-DO4ULagO.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/benefits-features.gif

Please build a React component that perfectly replicates a specific "Benefits" section. Use Tailwind CSS for styling, `motion/react` for animations, and `lucide-react` for icons.

### 1. Typography and Global Styles
- Use the `Inter` font everywhere (import via Google Fonts).
- The section background should be `#F4F6FA` and the base text color `#111`.
- Heavily utilize `tracking-tight` for headings and `tracking-wide` for small eyebrow tags.

### 2. Main Wrapper Layout
- Create a `<section>` with `py-20 lg:py-32 bg-[#F4F6FA] overflow-hidden`.
- Inside, create a container with `w-[90%] md:w-[85%] max-w-[1600px] mx-auto`.
- Use a 12-column grid layout on large screens: `grid grid-cols-1 lg:grid-cols-12 gap-16 lg:gap-12 items-start`.

### 3. Left Column (Takes up `lg:col-span-5`)
This column should contain staggered elements wrapped in `motion.div`. Use `initial={{ opacity: 0, y: 20 }}`, `whileInView={{ opacity: 1, y: 0 }}`, `viewport={{ once: true, amount: 0.3 }}` with a `duration: 0.6` and incrementing delays.

1. **Eyebrow Tag:** A flex container with a solid blue dot (`w-2 h-2 rounded-full bg-[#3b82f6]`) and the text "The benefit" (`text-[15px] font-medium tracking-wide`).
2. **Main Heading:** 
   - Text size: `text-[clamp(1.7rem,5.5vw,4.5rem)] leading-[1.05] tracking-tight font-medium mb-8`.
   - The first line is "Explore [INLINE_IMAGE] our"
   - The inline image must use exactly this URL: `https://res.cloudinary.com/dsdhxhhqh/image/upload/v1777202844/%D0%A1%D0%BA%D1%80%D0%B8%D0%BD%D1%88%D0%BE%D1%82_26-04-2026_134245-removebg-preview_jju5ww.png`. Style it as an inline block: `w-[1.2em] scale-[1.15] h-[0.8em] object-contain rounded-full align-middle mx-[-0.1em] -translate-y-[0.1em]`.
   - The second line is "flexible of activity."
3. **Pills Row (Flex wrap, gap 3, mb-12):** Two white `rounded-full` pills with slight shadows (`shadow-[0_2px_8px_rgba(0,0,0,0.04)]`), padding `px-5 py-2.5`, flex items centered.
   - First Pill: Contains a `<Soup className="w-[18px] h-[18px] text-gray-700"/>` and text "Eating After the Game".
   - Second Pill: Contains a `<Shirt className="w-[18px] h-[18px] text-gray-700"/>` and text "Game Jersey".
4. **Accordion/Tabs List:**
   - Manage state for `activeTab` (defaulting to 'connections'). 
   - Create two tabs: "Connections" and "Sport Pacakge" (keep exact spelling).
   - Wrapper styling for each tab: `rounded-[24px] overflow-hidden transition-all duration-300`. 
   - State styling: If active, apply `bg-white shadow-[0_4px_24px_rgba(0,0,0,0.03)]`. If inactive, apply `hover:bg-black/5 cursor-pointer`.
   - In the tab header (padding `p-7 md:p-8`), map the title (`text-[22px] font-medium`). On the right, include an animated icon toggle. Use framer-motion to cross-fade and rotate between a `<Plus />` and `<Minus />` icon depending on whether the tab is active.
   - Add a smooth expand/collapse `AnimatePresence` revealing content below the header.
   - Connections content: "Built to connect — with people, purpose, and the momentum that moves you forward."
   - Sport Package content: "A comprehensive collection of sporting goods, tailored for maximum performance and everyday agility."

### 4. Right Column (Takes up `lg:col-span-7 h-full`)
Fade this entire side in from the right (`x: 20` to `0`).
- Container: `bg-white rounded-[2.5rem] p-6 md:p-8 xl:p-12 shadow-[0_8px_30px_rgba(0,0,0,0.04)]`. Use `flex flex-col xl:flex-row gap-8 h-full min-h-[560px]`.

**Left Side of Card (Text & Button):**
- Flex column, flex-1, `justify-between`.
- Top section: A flex label `<Target className="w-5 h-5 text-[#ea580c]"/>` with text `EST — 1997` (`font-bold text-[15px] tracking-wide`). Below it, paragraph text: "Smart features designed to move with you — fast, flexible, and built for everyday action." (`max-w-[280px] text-gray-500 text-[18px] leading-[1.6]`).
- Bottom section: A stacked heading: "Visionary" over "Precision Play" (`text-[clamp(1.7rem,5vw,46px)] leading-[1.1] font-medium tracking-tight mb-8`).
- Button: 100% width on mobile, `rounded-full bg-black text-white px-7 py-4 flex items-center justify-between text-[15px] font-medium hover:bg-gray-800`. Text "Join Now!" with an `<ArrowRight />` icon. 

**Right Side of Card (Media and Floating Badges):**
- Container: `w-full xl:w-[320px] 2xl:w-[410px] h-[360px] md:h-[450px] xl:h-auto rounded-[2.5rem] relative overflow-hidden flex-shrink-0`.
- **The Video Layer:** Use a `<video>` tag filling exactly the absolute shape (`absolute inset-0 w-full h-full object-cover z-0`). Props: `autoPlay loop muted playsInline`. 
  - SRC MUST BE EXACTLY: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260426_105815_d66e9c4c-a1f8-4011-9ee5-f0cace24e642.mp4`
- **Top-Right Pill (Absolute Z-10):** Positioned `top-6 right-6 lg:top-8 lg:right-8`. `bg-white text-[#111] px-5 py-2.5 rounded-full flex shadow-md gap-2.5 text-[15px] font-medium`. Contains a `<Gift className="w-5 h-5 text-gray-700" />` and the text "February Sale". Pop this in with framer motion `scale`.
- **Bottom-Left Card (Absolute Z-10):** Positioned `bottom-6 left-6 lg:bottom-8 lg:left-8`. `bg-white text-[#111] rounded-[28px] overflow-hidden w-[190px] shadow-lg`. Pop this in with `x: -20` to `0`.
  - **Counter interaction & logic:** Attach `onViewportEnter={handleStartCount}` to this motion div. State `countValue` should animate from 0 to 86 seamlessly over 2000 duration using `performance.now()` in `requestAnimationFrame` and an ease-out calculation.
  - Top half (white): Padding `pt-6 px-6 pb-4`. Contains text "Tenis Outdor" in `text-gray-600 text-[15px] font-medium mb-1.5`. Below it, map the counter: `{countValue}%` styled `text-[42px] font-medium tracking-tight leading-none`.
  - Bottom half (blue): `bg-[#3585A5] text-white px-6 py-4 flex items-center gap-2.5`. Contains `<Zap className="w-5 h-5 fill-white text-white"/>` and text "Boost" (`font-medium text-[16px]`).

## Glow Features — Features Section [sites/glow-features]

- Preview: https://motionsites.ai/assets/features-glow-poster-CmUBaPAq.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/glow-features.png

Create a React web application using Vite and Tailwind CSS v4 that perfectly replicates a dark-themed glowing feature card section.

**Libraries Required:**
- React 19, Vite, Tailwind CSS v4
- `lucide-react` for icons
- `motion/react` (Framer Motion) for animations

**Global Page Layout:**
- Set the main wrapper to `min-h-screen bg-[#0A0A0B] flex flex-col items-center justify-center p-6 md:p-12 font-sans`.
- Create a CSS grid to hold the cards: `grid grid-cols-1 md:grid-cols-3 gap-10 md:gap-3 lg:gap-3 w-full max-w-[936px]`.

**The Feature Card Component Requirements:**
- Build a reusable `<FeatureCard />` component taking `title`, `description`, `icon`, `gradient`, and `delay` props.
- Wrap the entire card in a `<motion.div>`.
- Card size restrictions wrapper: `relative flex flex-col justify-start items-start w-full max-w-[260px] md:max-w-[300px] group mx-auto`.
- **Glow Background (Crucial):** Create an absolute positioned `div` behind the card content with `w-full h-[260px] md:h-[300px] opacity-60 rounded-[40px] pointer-events-none`. Apply inline styles: `background: gradient` and `filter: "blur(45px)"`.
- **Foreground Card with Gradient Border (Crucial):** On top of the glow, create a relative container with `self-stretch h-[260px] md:h-[300px] rounded-[40px] z-10 overflow-hidden`.
- Apply an 8px solid transparent border to this foreground card.
- Use the background-clip technique strictly for the border gradient via inline styles:
  `background: linear-gradient(#1A1A1C, #1A1A1C) padding-box, ${gradient} border-box;`
- Content Inner Layout: Inside the foreground, use `w-full h-full p-7 flex flex-col justify-between`.
- Icons should have `size={32}` and `strokeWidth={2.5}`, wrapped in a `text-white/90` div.
- Titles: `text-white font-medium text-xl mb-3 tracking-tight`.
- Descriptions: `text-gray-400 text-[14px] leading-[1.6] font-normal selection:bg-white/20`.

**Animations (Framer Motion):**
- The main `<motion.div>` wrapper should animate as follows:
  - Initial state: `{ opacity: 0, y: 30 }`
  - Animate state: `{ opacity: 1, y: 0 }`
  - Transition: `{ duration: 0.8, ease: "easeOut", delay }`

**Data for the 3 Cards:**
Instantiate three of these cards inside the main grid with the following exact data:

1. **Card 1 ("Hardware"):** 
   - Icon: `<Monitor />` from lucide-react. 
   - Delay: `0.1`
   - Description: "My entire desktop setup is built for power. It is silent, durable, and holds my focus."
   - Gradient: `linear-gradient(137deg, #FF3D77 0%, #FFB1CE 45%, #FF9D3C 100%)`

2. **Card 2 ("Studio"):** 
   - Icon: `<Palette />` from lucide-react. 
   - Delay: `0.2`
   - Description: "Studio is where I define every single pixel. It is the hub for each canvas I deliver."
   - Gradient: `linear-gradient(137deg, #FFFFFF 0%, #7DD3FC 45%, #06B6D4 100%)`

3. **Card 3 ("Motion"):** 
   - Icon: `<Zap />` from lucide-react. 
   - Delay: `0.3`
   - Description: "I use Motion to build lively prototypes, bridging the gap between views and code."
   - Gradient: `linear-gradient(137deg, #4361EE 0%, #E0AEFF 45%, #F72585 100%)`

## Keep Ahead Features — Features Section [sites/keep-ahead-features]

- Preview: https://motionsites.ai/assets/features-keep-ahead-poster-CriEHu8p.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/keep-ahead-features.png

Build a single-file HTML page with embedded CSS that recreates a premium dark-mode features section with three glassmorphic gradient cards. Match every specification below exactly.
Page Setup
DOCTYPE & meta: Standard HTML5 with <meta charset="UTF-8"> and <meta name="viewport" content="width=device-width, initial-scale=1.0">. Title: Community Page — Test 3.
Font: Load DM Sans from Google Fonts with this exact URL:
https://fonts.googleapis.com/css2?family=DM+Sans:ital,opsz,wght@0,9..40,400;0,9..40,500;0,9..40,600;0,9..40,700&display=swap
Base tag: Include <base target="_blank"> in the head so all links open in new tabs.
Global Styles
Apply universal box-sizing reset (*, *::before, *::after) with box-sizing: border-box; margin: 0; padding: 0.
Body:

Font: 'DM Sans', sans-serif
Background color: #050505 (near black)
Background image: dual linear-gradients creating a 60×60px grid of faint white lines:

linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px)
linear-gradient(90deg, rgba(255, 255, 255, 0.035) 1px, transparent 1px)


background-size: 60px 60px, background-position: top center
Text color: #2d3148, min-height: 100vh, padding 48px 24px

Section Structure
<section class="features-section">
  <div class="features-wrapper">
    <div class="features-header">...</div>
    <div class="features-cards">[3 cards]</div>
    <div class="features-tags">[3 tags]</div>
  </div>
</section>
.features-section: padding 0 0 48px.
.features-wrapper: max-width: 1100px, centered with margin: 0 auto, padding 60px 48px 40px, position: relative, overflow: hidden.
Header
.features-header: margin-bottom: 40px, position: relative, z-index: 1.

<h2> with text How We Keep You Ahead® (use &#174; for the registered symbol). Font size 34px, weight 700, color #ffffff, margin-bottom: 12px, line-height: 1.2, letter-spacing: -0.01em.
<p> with text: From quick daily updates to deep expert insights, we give you every advantage in the AI revolution. Font size 15px, color rgba(255, 255, 255, 0.55), line-height: 1.6, max-width: 380px, margin-bottom: 16px.
<span class="features-subline"> with text: Here's how we deliver on that promise every day. (use &#8217; for the curly apostrophe). Font size 14px, color rgba(255, 255, 255, 0.35), weight 500.

Cards Grid
.features-cards: CSS Grid with grid-template-columns: repeat(3, 1fr), gap: 20px, margin-bottom: 40px, position: relative, z-index: 1.
Card Structure (3-layer system, but only 2 are rendered)
Each card uses this nested structure:
<div class="feat-card [orange|blue|green]">
  <div class="feat-card-main">
    <div class="feat-icon">[svg]</div>
    <div class="feat-body">
      <div class="feat-title">...</div>
      <div class="feat-desc">...</div>
      <a class="feat-link">...</a>
    </div>
  </div>
</div>
.feat-card: position: relative, min-height: 320px, flex column, transition: transform 0.35s cubic-bezier(0.22, 1, 0.36, 1). On hover: transform: translateY(-4px) (lifts up 4px).
.feat-card-main: This is the visible card body. position: relative, z-index: 1, flex: 1, border-radius: 24px, padding 28px, flex column with gap: 16px, min-height: 290px, overflow: hidden. Hide its ::before and ::after pseudo-elements with display: none.
Card Color Variants
All three variants share an inset bottom glow: box-shadow: inset 0 -4px 15px -2px rgba(255, 255, 255, 0.9) and border: none. The backgrounds layer a top-left radial glow over a vertical 5-stop gradient that fades from near-black at top to white at the very bottom.
Orange (.feat-card.orange .feat-card-main):
cssbackground:
  radial-gradient(circle at 10% 10%, #d9511b50 0%, transparent 40%),
  linear-gradient(180deg, #180D0B 0%, #180D0B 40%, #CF451E 80%, #e9d551 96%, #FFFFFF 100%);
Blue (.feat-card.blue .feat-card-main):
cssbackground:
  radial-gradient(circle at 10% 10%, rgba(80, 150, 255, 0.30) 0%, transparent 40%),
  linear-gradient(180deg, #0B0F17 0%, #0B0F17 40%, #4663BF 80%, #a1ccf7 96%, #FFFFFF 100%);
Green (.feat-card.green .feat-card-main):
cssbackground:
  radial-gradient(circle at 10% 10%, rgba(50, 200, 110, 0.30) 0%, transparent 40%),
  linear-gradient(180deg, #0B0B12 0%, #0B0B12 40%, #38D26B 80%, #aaf8cd 96%, #FFFFFF 100%);
Icon Mini-Card
.feat-icon: 44px × 44px, border-radius: 12px, flex-centered, flex-shrink: 0, margin-bottom: 6px, position: relative, z-index: 2, overflow: hidden.
.feat-icon::before: Decorative top-left glow blob. position: absolute, top: -10px, left: -10px, 32px × 32px, border-radius: 50%, pointer-events: none, z-index: 0, opacity: 0.65.
Per-color icon styling:
Orange icon:

Background: linear-gradient(145deg, rgba(40, 28, 18, 0.9) 0%, rgba(14, 12, 10, 0.98) 100%)
Border: 1px solid rgba(232, 120, 40, 0.22)
Box-shadow: 0 0 12px rgba(232, 76, 10, 0.12), inset 0 1px 0 rgba(255, 200, 150, 0.08)
::before background: radial-gradient(circle, rgba(255, 100, 20, 0.55) 0%, transparent 70%)

Blue icon:

Background: linear-gradient(145deg, rgba(20, 25, 45, 0.9) 0%, rgba(10, 12, 16, 0.98) 100%)
Border: 1px solid rgba(80, 130, 255, 0.22)
Box-shadow: 0 0 12px rgba(42, 106, 238, 0.12), inset 0 1px 0 rgba(180, 210, 255, 0.08)
::before background: radial-gradient(circle, rgba(60, 120, 255, 0.55) 0%, transparent 70%)

Green icon:

Background: linear-gradient(145deg, rgba(18, 35, 25, 0.9) 0%, rgba(10, 14, 12, 0.98) 100%)
Border: 1px solid rgba(50, 200, 110, 0.22)
Box-shadow: 0 0 12px rgba(18, 192, 104, 0.12), inset 0 1px 0 rgba(180, 255, 220, 0.08)
::before background: radial-gradient(circle, rgba(40, 220, 120, 0.55) 0%, transparent 70%)

SVG inside icon: 20px × 20px, fill: none, stroke: rgba(255, 255, 255, 0.88), stroke-width: 2, stroke-linecap: round, stroke-linejoin: round, position: relative, z-index: 1.
Card Body Text
.feat-body: flex: 1, flex column with gap: 10px, position: relative, z-index: 2.

.feat-title: DM Sans, 22px, weight 700, white, line-height: 1.3.
.feat-desc: 14px, color rgba(255, 255, 255, 0.50), line-height: 1.65.
.feat-link: 13.5px, weight 700, white, no underline, inline-flex with gap: 6px, margin-top: 4px. Transition: gap 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s. On hover: gap: 10px (arrow slides right) and opacity: 0.85.

Three Cards — Exact Content
Card 1 (orange): Document/lines icon — SVG with viewBox="0 0 24 24" containing <rect x="2" y="4" width="20" height="16" rx="2" ry="2" />, <path d="M6 8h4" />, <path d="M6 12h12" />, <path d="M6 16h12" />.

Title: Daily Newsletter
Desc: Your shortcut to staying ahead—delivered every morning. (em dash via &#8212;)
Link: Get Daily Briefs → (arrow via &#8594;)

Card 2 (blue): Link/chain icon — SVG viewBox="0 0 24 24" with path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z".

Title: Curated Tools
Desc: The most powerful AI apps and platforms—tested and reviewed for you.
Link: Find My Tools →

Card 3 (green): Cube/package icon — SVG viewBox="0 0 24 24" with path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z", plus <polyline points="3.27 6.96 12 12.01 20.73 6.96" /> and <line x1="12" y1="22.08" x2="12" y2="12" />.

Title: Expert Insights
Desc: Actionable analysis from researchers and founders shaping the future of AI.
Link: Unlock Insights →

Bottom Tag Row
.features-tags: flex row, gap: 32px, items center, justify-content: flex-start, padding-top: 10px, position: relative, z-index: 1.
.feat-tag: flex with gap: 8px, items center, font-size: 13px, color rgba(255, 255, 255, 0.45), weight 600.
.feat-tag svg: 16px × 16px, fill: rgba(255, 255, 255, 0.6), stroke: none.
Three tags in order:

Lightning bolt (Always Current) — SVG viewBox="0 0 24 24", path d="M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.16-.28L11.66 2h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z".
Settings/gear-circle (Focused for You) — SVG viewBox="0 0 24 24", path d="M12 8c-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm8.94 3c-.46-4.17-3.77-7.48-7.94-7.94V1h-2v2.06C6.83 3.52 3.52 6.83 3.06 11H1v2h2.06c.46 4.17 3.77 7.48 7.94 7.94V23h2v-2.06c4.17-.46 7.48-3.77 7.94-7.94H23v-2h-2.06zM12 19c-3.87 0-7-3.13-7-7s3.13-7 7-7 7 3.13 7 7-3.13 7-7 7z".
Checkmark in circle (Actionable Steps) — SVG viewBox="0 0 24 24", path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z".

Animations / Transitions
Only two transitions exist — no keyframes, no scroll triggers, no entrance animations:

Card lift on hover: transition: transform 0.35s cubic-bezier(0.22, 1, 0.36, 1) on .feat-card, lifting translateY(-4px).
Link arrow slide on hover: transition: gap 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s on .feat-link, expanding gap from 6px to 10px and dropping opacity to 0.85.

Responsive Breakpoints
@media (max-width: 900px):

.features-cards collapses to single column (grid-template-columns: 1fr)
.features-wrapper padding becomes 48px 28px 32px

@media (max-width: 560px):

.features-header h2 shrinks to 28px
.features-tags allows wrapping (flex-wrap: wrap) with gap: 16px

## Nexora Features — Features Section [sites/nexora-features]

- Preview: https://motionsites.ai/assets/hero-nexora-features-preview-D26X0IiD.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexora-features.gif

Build a full-width "Features" section on a pure black (#000) background using TanStack Start (route file src/routes/index.tsx) and Tailwind CSS v4 (tokens in src/styles.css). Match the following spec exactly.

Global setup (src/styles.css)

Set body background to #000, text color #fff, and font-family to "Helvetica Neue", Helvetica, Arial, sans-serif.
Add a .liquid-glass button class with this exact CSS:

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
Add card classes (both same color, plain #252B4C, rounded 1.25rem, position: relative; overflow: hidden;):

.feature-card,
.feature-card-dark {
  background: #252B4C;
  border-radius: 1.25rem;
  position: relative;
  overflow: hidden;
}
Components

LiquidButton: a <button> with classes liquid-glass rounded-xl px-5 py-2.5 text-sm text-white/90 transition-transform hover:scale-[1.02]. Hover animation = subtle scale-up to 1.02.
CardVideo({ src }): an absolutely positioned <video> filling the card: className="absolute inset-0 h-full w-full object-cover", attributes autoPlay muted loop playsInline, 100% opacity, no overlay.
Layout

<main> with min-h-screen bg-black px-6 py-16 md:px-12 lg:px-20.

Header (mb-10 flex flex-col gap-6 md:flex-row md:items-start md:justify-between):
Left: stacked headline with two lines, sizes text-2xl md:text-4xl lg:text-[2.75rem], font-normal tracking-tight. Text wraps naturally (no whitespace-nowrap).
Line 1 (white): Curiosity-led tools for truth-seeking minds.
Line 2 (text-white/40, mt-2): Ask with confidence. Powered by AI.
Right (md:pt-3 shrink-0): <LiquidButton>Start Using Nexora</LiquidButton>.
Grid: grid grid-cols-1 gap-5 md:grid-cols-3 md:grid-rows-2.

All cards use p-7 flex flex-col, contain a <CardVideo> as the first child, and all text wrappers use relative so they sit above the video.

Card 01 — feature-card md:row-span-2 min-h-[28rem] (tall left column)
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_104605_2700410c-4303-4d44-a368-e1b8c84eca8c.mp4
Top row (flex justify-between text-sm text-white/60): 01/ left, Found in Curiosity right.
Spacer (flex-1).
Bottom block:
<h2 class="text-xl md:text-2xl font-medium text-white">Great Questions Unearth<br/>Hidden Gems</h2>
Divider: mt-4 h-px w-full bg-white/20.
Paragraph (mt-4 text-xs text-white/70): The best answers come from asking the right questions.<br/>Start your search with purpose today.
Card 02 — feature-card-dark md:col-span-2 (wide top right)
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_104731_bfd355f7-1f84-4f81-ad88-52c2bca70bad.mp4
Top row (flex justify-between): heading left <h2 class="text-xl md:text-2xl font-medium text-white">Where Knowledge Begins</h2>, 02/ right (text-sm text-white/60).
Spacer: flex-1 min-h-48.
Card 03 — feature-card
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_104758_e7d78f06-3700-4862-8c9b-595ed447e81a.mp4
Top row (text-sm text-white/60): In Real Time left, 03/ right.
Paragraph (mt-10 text-xs text-white/80): From complex topics to quick facts, trust what<br/>you learn from every search you perform.
Spacer (flex-1).
Bottom (mt-6): <LiquidButton>Start Using Nexora</LiquidButton>.
Card 04 — feature-card
Video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_105007_f90de0f3-0f93-44d4-9b71-7446f78c4bd2.mp4
Top row (text-sm text-white/60): Just Ask left, 04/ right.
Spacer (flex-1).
Bottom paragraph (text-center text-xs text-white/80): Users Trust Our Search Models.
Animations / fonts

Only animation: liquid-glass button hover scale to 1.02 via Tailwind transition-transform hover:scale-[1.02].
Videos auto-play looping at 100% opacity, no overlay/tint.
Font: Helvetica Neue globally, weights font-normal for headline, font-medium for card titles.

## Stark Minimal Footer — Footer [sites/stark-minimal-footer]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(37).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/stark-minimal-footer.webp

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

## HAUL! — Footer Section [sites/haul-footer]

- Preview: https://motionsites.ai/assets/footer-haul-poster-Do5X7frB.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/haul-footer.png

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

## Kresna Footer — Footer Section [sites/kresna-footer]

- Preview: https://motionsites.ai/assets/footer-kresna-preview-BrIYYd2q.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/kresna-footer.gif

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

## Lumina — Footer Section [sites/lumina-footer]

- Preview: https://motionsites.ai/assets/footer-lumina-preview-CYkr-ACN.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/lumina-footer.gif

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

## Vize Footer — Footer Section [sites/vize-footer]

- Preview: https://motionsites.ai/assets/footer-vize-poster-BRRRDP-A.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vize-footer.png

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

## Zenith Footer — Footer Section [sites/zenith-footer]

- Preview: https://motionsites.ai/assets/footer-zenith-preview-CYxIE6aF.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/zenith-footer.gif

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

## Arceage Contact Us — Form [sites/arceage-contact-us]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(20).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/arceage-contact-us.webp

Create a React + Tailwind CSS v4 + Motion ("motion/react") contact form section with inline validation. Use Vite as the bundler. Fully mobile responsive.

### Fonts

Import from Google Fonts in your global CSS:
```
@import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Instrument+Serif:ital@0;1&display=swap');
```

Define two Tailwind v4 theme fonts:
- `--font-sans: "Barlow", ui-sans-serif, system-ui, sans-serif;` (primary UI font via `font-sans`)
- `--font-dm-serif: "Instrument Serif", serif;` (accent italic font via `font-dm-serif`)

The page wrapper uses `bg-black font-sans text-white`. This section overrides to `bg-white text-black`.

### Dependencies

- `react` v19 (uses `useState`)
- `motion` (npm package "motion", import `motion` from `motion/react`)
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

### State Management

Two pieces of state:

**formData** (object, all strings, default empty):
```js
{ name: '', email: '', phone: '', farm: '', message: '' }
```

**touched** (object, all booleans, default false):
```js
{ name: false, email: false, phone: false, farm: false, message: false }
```

**handleChange**: Updates `formData[e.target.name]` on input change.

**handleBlur**: Sets `touched[e.target.name]` to `true` on blur.

---

### Validation Rules

```js
const validations = {
  name: formData.name.trim().length > 0,
  email: /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(formData.email),
  phone: /^\+[\d\s\-\(\)]{7,20}$/.test(formData.phone),
  farm: formData.farm.trim().length > 0,
  message: formData.message.trim().length > 0,
};
```

---

### Validation Icons (renderIcon function)

A helper function `renderIcon(fieldName, isRequired)` that:
- Returns `null` if the field has not been touched yet
- If valid (and either required OR has content): shows a **green check circle** icon
- If invalid and required: shows a **red X circle** icon
- Both icons use CSS mask-image technique (colored `div` with SVG mask), positioned `absolute right-0 top-1/2 -translate-y-1/2 w-5 h-5`

**Green valid icon:**
- `bg-[#27BD09]`
- SVG mask URL: `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/tick-circle.svg`
- Applied via both `WebkitMaskImage` and `maskImage` inline styles, with `maskSize: 'contain'`, `maskRepeat: 'no-repeat'`, `maskPosition: 'center'`

**Red invalid icon:**
- `bg-[#FF1F1F]`
- SVG mask URL: `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/close-circle.svg`
- Same mask CSS properties as the green icon

---

### Form Field Animation Variants

```js
const formVariants = {
  hidden: { opacity: 0, y: 20 },
  visible: { opacity: 1, y: 0, transition: { duration: 0.6, ease: "easeOut" } }
};
```

---

### Section Container

`<section>` with:
- `id="contact"`
- Classes: `w-full bg-white text-black py-24 px-6 md:px-12 lg:px-[120px] flex flex-col items-center justify-center`

### Staggered Reveal Wrapper

Outer `motion.div`:
- `initial="hidden"`, `whileInView="visible"`, `viewport={{ once: true, margin: "-100px" }}`
- Variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1, transition: { staggerChildren: 0.1 } }`
- Classes: `w-full max-w-3xl mx-auto flex flex-col items-center`

---

### Element 1: Header Area

A `div` with classes `text-center mb-16 w-full`.

**Heading (h2):**
- Classes: `text-[clamp(1.5rem,4vw,3.5rem)] font-medium tracking-tight mb-6 leading-[1.1]`
- Content (3 Typewriter segments across 2 lines):
  - `<span className="text-black font-dm-serif font-normal italic">` wrapping `<Typewriter text="Let's grow!" delay={0} speed={0.012} />`
  - Then a space, then `<Typewriter text="Fill in the form" delay={0.2} speed={0.012} />`
  - Then `<br />`
  - Then `<Typewriter text="and we'll be in touch" delay={0.4} speed={0.012} />`
- "Let's grow!" is rendered in Instrument Serif italic; the rest is in Barlow medium.

**Subtitle (p):**
- Classes: `text-lg md:text-xl text-gray-800`
- Content: `<Typewriter text="Ask us about our precision harvesting services" delay={0.6} speed={0.012} />`

---

### Element 2: Form

`<form>` with:
- Classes: `max-w-2xl w-full mx-auto flex flex-col gap-8`
- `onSubmit={(e) => e.preventDefault()}`

**5 form fields**, each wrapped in `motion.div` using `formVariants`. Each field follows this exact pattern:

- Wrapper `motion.div`: classes `flex flex-col gap-2 border-b border-[#D9D9D9] pb-2 transition-colors duration-300 hover:border-black focus-within:border-black`
- `<label>`: classes `text-sm font-medium`
- Inner `div`: classes `relative w-full`
- `<input>`: classes `w-full bg-transparent outline-none placeholder:text-[#D9D9D9] focus:placeholder:text-gray-500 transition-colors duration-300 text-base pr-8`
  - All inputs have `value`, `onChange={handleChange}`, `onBlur={handleBlur}`, and `name` attribute matching the field key
- After the input: `{renderIcon('fieldName', isRequired)}`

The 5 fields in order:

| # | Label | Name | Type | Placeholder | Required |
|---|-------|------|------|-------------|----------|
| 1 | Your Name* | name | text | Who's reaching out? | true |
| 2 | Email* | email | email | Where can we reach you? | true |
| 3 | Phone Number* | phone | tel | Best number to call you on? | true |
| 4 | Farm / Company | farm | text | Your farm or organization? | false |
| 5 | Tell Us More | message | text | What crops or acreage would you like to discuss? | false |

Fields 1-3 have the HTML `required` attribute and pass `true` to `renderIcon`. Fields 4-5 do not have `required` and pass `false` to `renderIcon`.

---

### Element 3: Submit Button

Wrapped in `motion.div` with `formVariants`, classes `mt-8 flex justify-center`.

**Button:**
- `type="submit"`
- Classes: `bg-black text-white px-6 py-2.5 rounded-full hover:bg-[#27BD09] transition-colors duration-300 text-sm tracking-wide`
- Text: "Send Message"
- Hover changes background from black to green (`#27BD09`).

---

### Mobile Responsiveness Summary

- Section padding: `py-24 px-6` on all sizes, `md:px-12`, `lg:px-[120px]`
- Form constrained to `max-w-2xl` (672px), centered with `mx-auto`
- Outer wrapper constrained to `max-w-3xl` (768px)
- Heading uses fluid typography: `clamp(1.5rem, 4vw, 3.5rem)`
- Subtitle: `text-lg` on mobile, `md:text-xl` on desktop
- Form fields stack vertically with `gap-8` at all breakpoints
- All interactive elements (inputs, button) are full-width and touch-friendly
- Validation icons are absolutely positioned at `right-0` inside each field, always visible regardless of viewport

---

## Guardnet Demo — Info [sites/guardnet-demo]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(26).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/guardnet-demo.webp

Build two React + TypeScript sections using Tailwind CSS. No extra libraries besides React and Tailwind. Everything is fully mobile-responsive. The entire page has a black background with white text.

### Global Prerequisites

- Font: `@import url(https://db.onlinewebfonts.com/c/e55e9079ee863276569c8a68d776ef04?family=Futura+Md+BT+Medium);`
- Body: `font-family: 'Futura Md BT Medium', system-ui, -apple-system, sans-serif; background-color: #000; color: #fff; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;`
- All text is lowercase unless otherwise stated.
- The two sections sit inside a `w-full max-w-[1400px]` wrapper, stacked vertically.

---

### Section 1: SecuritySection

Container: `relative min-h-[600px] h-screen w-full overflow-hidden bg-black`.

### Background Video (absolute fill)

- Classes: `absolute inset-0 w-full h-full object-cover`
- Attributes: `autoPlay loop muted playsInline`
- **Exact URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260421_072418_508a7d2e-396d-4f6f-9d42-ec920fcf7755.mp4`

### Top Fade Overlay

`pointer-events-none absolute top-0 left-0 right-0 h-48 bg-gradient-to-b from-black to-transparent z-10`

### Inner Wrapper

`relative h-full w-full max-w-[1100px] mx-auto`

### Floating Pill Navigation (centered at top)

Positioned: `absolute top-6 sm:top-10 left-1/2 -translate-x-1/2 z-20 w-max max-w-[95vw]`

Pill container: `flex items-center gap-1.5 sm:gap-2 bg-neutral-900/80 backdrop-blur rounded-full p-2 sm:p-3`

Two buttons inside:

1. **Ghost button:** `text-white/90 text-xs sm:text-sm px-4 sm:px-7 py-2 sm:py-3 rounded-full hover:text-white transition-colors whitespace-nowrap` -- label: **"confirm real person"**

2. **Gradient button:** `text-black text-xs sm:text-sm font-normal px-4 sm:px-7 py-2 sm:py-3 rounded-full whitespace-nowrap` with inline style `background: linear-gradient(90deg, #FA8453 0%, #F8C9B2 100%)` -- label: **"run demo"**

### Left Paragraph

`absolute left-4 sm:left-6 md:left-16 top-[62%] sm:top-[56%] max-w-[280px] sm:max-w-[440px] text-[13px] sm:text-[18px] leading-relaxed text-white/80 font-light`

Text: **"shielding users info with premier tech, granting them with safety in all place"**

### Right Paragraph

`absolute right-4 sm:right-6 md:right-16 top-[26%] sm:top-[34%] max-w-[280px] sm:max-w-[500px] text-[13px] sm:text-[18px] leading-relaxed text-white/90 font-light`

Text: **"By teaming up with a defender service, a business can dramatically improve the safeguard of its important info. This covers applying strong obfuscation protocols, gateway barriers, and observation engines to shield against unauthorized entries, info escapes, and malicious cyberhacks."**

---

### Section 2: CompaniesSection

Container: `relative w-full bg-black px-4 sm:px-6 md:px-10 py-12 sm:py-20`

### Company Cards Grid

`grid grid-cols-2 md:grid-cols-4 gap-3 sm:gap-4`

Each card: `relative h-24 sm:h-32 md:h-36 rounded-2xl bg-neutral-950 overflow-hidden flex items-center justify-center`

Each card has:
- One or more **blurred color blobs** (absolutely positioned, `rounded-full blur-3xl`, various opacities)
- A **centered logo** (`relative z-10`) consisting of an inline SVG icon (`h-6 w-6 sm:h-8 sm:w-8`, fill white) and a text wordmark

### Card 1: Apex

- **Blob:** `absolute -top-24 -left-24 h-40 w-40 rounded-full bg-[#1e3a8a] blur-3xl opacity-40`
- **SVG path:** `M12 2l2.39 4.84L20 8l-4 3.9L17.28 18 12 15.27 6.72 18 8 11.9 4 8l5.61-1.16L12 2z` (viewBox `0 0 24 24`)
- **Wordmark:** `text-white text-xl sm:text-3xl font-semibold tracking-tight` -- "Apex"

### Card 2: forge

- **Blob 1:** `absolute -top-24 -left-24 h-40 w-40 rounded-full bg-[#FA8453] blur-3xl opacity-30`
- **Blob 2:** `absolute -bottom-24 -right-24 h-40 w-40 rounded-full bg-[#F5D547] blur-3xl opacity-25`
- **SVG path:** `M20.63 8.46l-4.73-2.73-.53.31 5.1 2.94v5.88l-5.1 2.94.53.3 4.73-2.72V8.46zM8.1 6.04l.53.3L3.53 9.28v5.88L8.63 18.1l-.53.3-4.73-2.72V8.46L8.1 6.04zM16.05 14.3v-4.6L12 7.4 7.95 9.7v4.6L12 16.6l4.05-2.3zm-.53-.3L12 16.02l-3.52-2.02v-4.02L12 7.96l3.52 2.02v4.02z` (viewBox `0 0 24 24`)
- **Wordmark:** `text-white text-xl sm:text-3xl font-semibold tracking-tight` -- "forge"

### Card 3: Eastern Delta

- **Blob:** `absolute -bottom-24 -left-24 h-40 w-40 rounded-full bg-[#F5D547] blur-3xl opacity-30`
- **SVG path:** `M2 4l3 16h3l2-10 2 10h3l3-16h-3l-1.5 10L12 4h-2L8.5 14 7 4H2z` (viewBox `0 0 24 24`)
- **Wordmark:** `text-white text-lg sm:text-2xl font-semibold leading-tight tracking-tight` -- two lines: "Eastern" then `<br />` then "Delta"

### Card 4: Skybank

- **Blob:** `absolute top-1/2 -translate-y-1/2 -right-28 h-48 w-48 rounded-full bg-[#1e3a8a] blur-3xl opacity-40`
- **SVG path:** `M6 2l6 3.75L6 9.5 0 5.75 6 2zm12 0l6 3.75L18 9.5l-6-3.75L18 2zM0 13.25L6 9.5l6 3.75L6 17l-6-3.75zm18-3.75l6 3.75L18 17l-6-3.75 6-3.75zM6 18.25L12 14.5l6 3.75L12 22l-6-3.75z` (viewBox `0 0 24 24`)
- **Wordmark:** `text-white text-xl sm:text-3xl font-semibold tracking-tight` -- "Skybank"

### Bottom Row (below grid)

Container: `mt-16 sm:mt-28 flex flex-col md:flex-row items-start md:items-center justify-between gap-6 sm:gap-8 md:w-[70%] md:ml-auto`

### Left Paragraph

`max-w-md text-[13px] sm:text-[18px] leading-relaxed text-white/70 font-light`

Text: **"shielding users info with premier tech, granting them with safety in all place"**

### Gradient-Border "Run Demo" Button

Outer wrapper: `relative rounded-full p-[1.5px] self-start md:self-auto` with inline style `background: linear-gradient(90deg, #FA8453 0%, #F8C9B2 100%)`

Inner span: `block rounded-full bg-black px-8 sm:px-10 py-2.5 sm:py-3 text-white text-sm` -- label: **"Run Demo"**

---

### Color Palette Reference

| Token | Hex |
|---|---|
| Background | `#000000` (black) |
| Card surface | `neutral-950` (Tailwind) |
| Blob blue | `#1e3a8a` |
| Blob orange | `#FA8453` |
| Blob yellow | `#F5D547` |
| Gradient start | `#FA8453` |
| Gradient end | `#F8C9B2` |
| Body text | `white/70` to `white/90` |

### Responsive Breakpoints

- Default (mobile-first): `< 640px`
- `sm:` at `640px`
- `md:` at `768px`

All text sizes, padding, gaps, and heights scale across these three tiers as specified in the class lists above.

### Interactions

- Ghost button hover: `text-white/90` to `text-white` via `transition-colors`
- No JavaScript animations; all motion comes from the looping background video
- Gradient-border button has no hover state beyond default cursor

## Scroll Marquee — Marquee [sites/scroll-marquee]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(21).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/scroll-marquee.webp

---

**Prompt:**

Create a scroll-driven image marquee section using React, Tailwind CSS, and vanilla JS scroll events (no libraries needed for the marquee itself). The site uses the **Google Font "Kanit"** (weights 300-900) and a dark background color `#0C0C0C`. This section sits directly below a full-screen hero section.

**Section layout:**
- Full-width section with `overflow: hidden`, dark background `#0C0C0C`
- Padding: `pt-24 sm:pt-32 md:pt-40 pb-10`
- Contains two horizontal rows of images stacked vertically with a `gap-3` (12px) between them

**Row 1 images (11 images), scrolls RIGHT as user scrolls down:**
```
https://motionsites.ai/assets/hero-space-voyage-preview-eECLH3Yc.gif
https://motionsites.ai/assets/hero-codenest-preview-Cgppc2qV.gif
https://motionsites.ai/assets/hero-vex-ventures-preview-BczMFIiw.gif
https://motionsites.ai/assets/hero-stellar-ai-v2-preview-DjvxjG3C.gif
https://motionsites.ai/assets/hero-asme-preview-B_nGDnTP.gif
https://motionsites.ai/assets/hero-transform-data-preview-Cx5OU29N.gif
https://motionsites.ai/assets/hero-vitara-preview-Cjz2QYyU.gif
https://motionsites.ai/assets/hero-terra-preview-BFjrCr7T.gif
https://motionsites.ai/assets/hero-skyelite-preview-DHaZIgUv.gif
https://motionsites.ai/assets/hero-aethera-preview-DknSlcTa.gif
https://motionsites.ai/assets/hero-designpro-preview-D8c5_een.gif
```

**Row 2 images (10 images), scrolls LEFT as user scrolls down (opposite direction):**
```
https://motionsites.ai/assets/hero-stellar-ai-preview-D3HL6bw1.gif
https://motionsites.ai/assets/hero-xportfolio-preview-D4A8maiC.gif
https://motionsites.ai/assets/hero-orbit-web3-preview-BXt4OttD.gif
https://motionsites.ai/assets/hero-nexora-preview-cx5HmUgo.gif
https://motionsites.ai/assets/hero-evr-ventures-preview-DZxeVFEX.gif
https://motionsites.ai/assets/hero-planet-orbit-preview-DWAP8Z1P.gif
https://motionsites.ai/assets/hero-new-era-preview-CocuDUm9.gif
https://motionsites.ai/assets/hero-wealth-preview-B70idl_u.gif
https://motionsites.ai/assets/hero-luminex-preview-CxOP7ce6.gif
https://motionsites.ai/assets/hero-celestia-preview-0yO3jXO8.gif
```

**Image cards:**
- Each image card is exactly `420px wide x 270px tall`
- `flex-shrink-0` so they never collapse
- `rounded-2xl` (16px border radius) with `overflow: hidden`
- Images use `object-cover` to fill the container, with `loading="lazy"`
- `gap-3` (12px) between cards horizontally

**Scroll-driven parallax animation (vanilla JS, not CSS animation):**
- Each row's image array is tripled (`[...ROW, ...ROW, ...ROW]`) to create enough content for continuous scrolling appearance
- On every scroll event (passive listener), calculate offset:
  1. Get the section's bounding rect top relative to the document
  2. `scrolled = window.scrollY - sectionTop + window.innerHeight`
  3. `offset = scrolled * 0.3` (parallax factor of 0.3)
- Row 1: `transform: translateX(${offset - 200}px)` -- moves right as user scrolls
- Row 2: `transform: translateX(${-(offset - 200)}px)` -- moves left as user scrolls
- Initial transform: Row 1 starts at `translateX(-200px)`, Row 2 starts at `translateX(200px)`
- Use `willChange: 'transform'` for GPU acceleration
- Apply transforms directly via refs (not state) for 60fps performance
- Use `{ passive: true }` on the scroll listener
- Run the handler once on mount to set initial position
- Clean up the listener on unmount

**Technical details:**
- React functional component using `useRef` and `useEffect`
- No framer-motion or animation library needed for this section
- Rows are built with flexbox (`flex gap-3`)
- Each row is wrapped in an `overflow-hidden w-full` container
- The outer section also has `overflow-hidden` and `w-full`
- All images have empty `alt=""` since they are decorative

**Font (loaded in HTML head):**
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Kanit:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

CSS base: `font-family: 'Kanit', sans-serif` on html/body.

---

## 3D Studio Pricing — Pricing [sites/3d-studio-pricing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/priicnggrow.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/3d-studio-pricing.mp4

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

## Nex Max Upgrade — Pricing [sites/nex-max-upgrade]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(44).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nex-max-upgrade.webp

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

## NimBus Pricing — Pricing [sites/nimbus-pricing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(12).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-pricing.webp

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

## What Package Fits You — Pricing [sites/package-fits-pricing]

- Preview: https://motionsites.ai/assets/hero-package-fits-pricing-preview-Bglk5DXD.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/package-fits-pricing.gif

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

## Rocket Pricing — Pricing [sites/rocket-pricing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(55).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/rocket-pricing.webp

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

## SaaS Pricing Flow — Pricing [sites/saas-pricing-flow]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(35).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/saas-pricing-flow.webp

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

## NexaCore Process — Process [sites/nexacore-process]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(47).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexacore-process.webp

Build two React + TypeScript + Tailwind CSS v3 components: `ServiceCard` and `TrustedSection`. No external icon libraries — all icons are inline SVG. Fully mobile-responsive. Uses `useState` for hover animations on cards.

---

### Global font

Register **"Mazzard H"** in `index.css` and apply it globally:

```css
@font-face {
  font-family: 'Mazzard H';
  font-weight: 400;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.ttf') format('truetype');
}
@font-face {
  font-family: 'Mazzard H';
  font-weight: 500;
  font-style: normal;
  src: url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff2') format('woff2'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff') format('woff'),
       url('https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.ttf') format('truetype');
}

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html, body, * { font-family: 'Mazzard H', sans-serif; }
}
```

---

### File 1: `src/components/ServiceCard.tsx`

### Constants (top of file)

```ts
const CARD_GRADIENT = 'linear-gradient(90deg, rgb(28,78,255), rgb(172,36,255) 50%, rgb(254,136,27))';
const BULLET_URL = 'https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/683ef70a24657b10be91ef49_bullet-list.svg';
const CARD_IMG = 'https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/682c7cb62b8800a7594c5abd_hover_card_img.png';
```

### Props interface

```ts
interface ServiceCardProps {
  label: string;
  icon: React.ReactNode;
  title: React.ReactNode;
  bullets: string[];
}
```

### Component

Uses `useState<boolean>(false)` for `hovered`. `onMouseEnter` sets `true`, `onMouseLeave` sets `false`.

**Card root `<div>`** — Tailwind: `relative flex flex-col overflow-hidden rounded-[36px] cursor-pointer`

Inline styles:
```
background-color: rgba(10, 5, 20, 0.88)
backdrop-filter: blur(36px)
height: clamp(320px, 32vw, 500px)
```

---

### Layer 1 — Top image (always present, animates on hover)

Tailwind: `absolute inset-x-0 top-0 pointer-events-none select-none transition-all duration-500`

Inline styles:
```
height: 55%
z-index: 1
transform: hovered ? 'translateY(0)' : 'translateY(-30%)'
opacity: hovered ? 1 : 0.7
```

Contains `<img src={CARD_IMG} alt="" aria-hidden>` with Tailwind `w-full h-full` and inline:
```
object-fit: cover
object-position: top
```

---

### Layer 2 — Bottom dark gradient overlay (slides up on hover)

Tailwind: `absolute inset-x-0 bottom-0 pointer-events-none select-none transition-all duration-500`

Inline styles:
```
height: 55%
z-index: 1
background: linear-gradient(to top, rgba(10,5,20,0.95) 60%, transparent)
transform: hovered ? 'translateY(0)' : 'translateY(100%)'
opacity: hovered ? 1 : 0
```

---

### Layer 3 — Content (z-index 2)

Tailwind: `relative flex flex-col h-full`

Inline: `z-index: 2`, `padding: clamp(16px, 1.94vw, 32px) clamp(18px, 2.36vw, 36px)`

**Badge** — Tailwind: `flex items-center gap-2 w-fit rounded-full text-white text-sm font-medium flex-shrink-0`

Inline: `background-color: rgb(41, 31, 57)`, `padding: clamp(6px, 0.7vw, 12px) clamp(10px, 1.25vw, 20px)`

Icon wrapper inside badge — Tailwind: `flex items-center justify-center`

Inline: `width: 1.11vw`, `min-width: 14px`, `height: 17px`

Renders `{icon}` prop inside.

After icon wrapper: `{label}` text.

---

**Spacer** — Tailwind: `flex-grow` (pushes content to bottom)

---

**Bottom content block** — Tailwind: `flex flex-col transition-all duration-500`, inline `gap: 16px`

Inside it, an **animated inner block** — Tailwind: `flex flex-col transition-transform duration-500`

Inline:
```
gap: 16px
transform: hovered ? 'translateY(-8px)' : 'translateY(0)'
```

**Title `<div>`** inside animated block — Tailwind: `text-white font-medium leading-snug`

Inline: `font-size: clamp(16px, 1.7vw, 24px)`

Renders `{title}` prop (can contain `<br />` tags).

**Bullets `<ul>`** inside animated block — Tailwind: `flex flex-col`

Inline: `gap: 10px`, `list-style: none`, `padding: 0`, `margin: 0`

Each `<li>` — inline styles only:
```
color: rgb(189, 174, 231)
font-size: clamp(12px, 1vw, 15px)
padding-left: clamp(22px, 1.8vw, 28px)
background-image: url("https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/683ef70a24657b10be91ef49_bullet-list.svg")
background-repeat: no-repeat
background-size: 18px
background-position: 0% 50%
```

---

**Button reveal wrapper** — Tailwind: `transition-all duration-500 overflow-hidden`

Inline:
```
max-height: hovered ? 80 : 0
opacity: hovered ? 1 : 0
transform: hovered ? 'translateY(0)' : 'translateY(20px)'
pointer-events: hovered ? 'auto' : 'none'
```

Inside it, `<a href="#">` — Tailwind: `flex items-center justify-center w-full rounded-xl text-white font-medium`

Inline:
```
background: linear-gradient(90deg, rgb(28,78,255), rgb(172,36,255) 50%, rgb(254,136,27))
padding: clamp(10px, 0.9vw, 14px) 0
font-size: clamp(13px, 1.1vw, 16px)
margin-bottom: clamp(6px, 0.5vw, 10px)
```

Text: **"Learn more"**

---

### File 2: `src/components/TrustedSection.tsx`

### Icon components (defined at top, outside the section component)

All icons: `viewBox="0 0 16 16"`, `fill="none"`, `style={{ width: '100%', height: '100%' }}`, `xmlns="http://www.w3.org/2000/svg"`. All use `fill="rgb(200, 111, 255)"`. Define `const ICON_COLOR = 'rgb(200, 111, 255)'`.

**`DesignIcon`** — hollow ring only:
```xml
<path d="M1 8C1 11.866 4.13401 15 8 15C11.866 15 15 11.866 15 8C15 4.13401 11.866 1 8 1C4.13401 1 1 4.13401 1 8ZM13.6 8C13.6 11.0928 11.0928 13.6 8 13.6C4.90721 13.6 2.4 11.0928 2.4 8C2.4 4.90721 4.90721 2.4 8 2.4C11.0928 2.4 13.6 4.90721 13.6 8Z" fill={ICON_COLOR}/>
```

**`OnboardingIcon`** — ring + small dot (r=2):
```xml
<path d="M0.970459 8C0.970459 11.866 4.10447 15 7.97046 15C11.8365 15 14.9705 11.866 14.9705 8C14.9705 4.13401 11.8365 1 7.97046 1C4.10447 1 0.970459 4.13401 0.970459 8ZM13.5705 8C13.5705 11.0928 11.0633 13.6 7.97046 13.6C4.87766 13.6 2.37046 11.0928 2.37046 8C2.37046 4.90721 4.87766 2.4 7.97046 2.4C11.0633 2.4 13.5705 4.90721 13.5705 8Z" fill={ICON_COLOR}/>
<circle cx="2" cy="2" r="2" transform="matrix(-1 0 0 1 10 6)" fill={ICON_COLOR}/>
```

**`DeliveryIcon`** — ring + medium dot (r=3):
```xml
<path d="M0.970459 8C0.970459 11.866 4.10447 15 7.97046 15C11.8365 15 14.9705 11.866 14.9705 8C14.9705 4.13401 11.8365 1 7.97046 1C4.10447 1 0.970459 4.13401 0.970459 8ZM13.5705 8C13.5705 11.0928 11.0633 13.6 7.97046 13.6C4.87766 13.6 2.37046 11.0928 2.37046 8C2.37046 4.90721 4.87766 2.4 7.97046 2.4C11.0633 2.4 13.5705 4.90721 13.5705 8Z" fill={ICON_COLOR}/>
<circle cx="3" cy="3" r="3" transform="matrix(-1 0 0 1 11 5)" fill={ICON_COLOR}/>
```

**`DeploymentIcon`** — ring + large dot (r=4):
```xml
<path d="M0.970459 8C0.970459 11.866 4.10447 15 7.97046 15C11.8365 15 14.9705 11.866 14.9705 8C14.9705 4.13401 11.8365 1 7.97046 1C4.10447 1 0.970459 4.13401 0.970459 8ZM13.5705 8C13.5705 11.0928 11.0633 13.6 7.97046 13.6C4.87766 13.6 2.37046 11.0928 2.37046 8C2.37046 4.90721 4.87766 2.4 7.97046 2.4C11.0633 2.4 13.5705 4.90721 13.5705 8Z" fill={ICON_COLOR}/>
<circle cx="4" cy="4" r="4" transform="matrix(-1 0 0 1 12 4)" fill={ICON_COLOR}/>
```

### Cards data array

```ts
const CARDS = [
  {
    label: 'Planning',
    icon: <DesignIcon />,
    title: <>Turn new programs<br />into structured plans without the noise.</>,
    bullets: ['Embedded program leads', 'Decision-ready roadmaps'],
  },
  {
    label: 'Procurement',
    icon: <OnboardingIcon />,
    title: <>Source and qualify<br />vendors with far<br />less friction.</>,
    bullets: ['Cross-org scope alignment', 'End-to-end accountability'],
  },
  {
    label: 'Logistics',
    icon: <DeliveryIcon />,
    title: <>Move the right<br />materials on time<br />without surprises.</>,
    bullets: ['Spec and fit validations', 'Change order ownership'],
  },
  {
    label: 'Commissioning',
    icon: <DeploymentIcon />,
    title: <>Activate systems with complete context, not guesswork.</>,
    bullets: ['Uninterrupted workflows', 'Verified clean handoffs'],
  },
];
```

### Section component

**`<section>`** — Tailwind: `relative flex flex-col items-center w-full`

Inline:
```
background-image: url("https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260418_120332_3b24257a-afe6-48ca-875f-78147370f403.png&w=1280&q=85")
background-size: cover
background-position: center
background-repeat: no-repeat
padding: clamp(100px, 12vw, 180px) clamp(16px, 4vw, 40px) clamp(100px, 12vw, 160px)
gap: 110px
```

---

**Header block** — Tailwind: `flex flex-col items-center text-center w-full px-2`

Inline: `max-width: 1200px`, `gap: 20px`

`<h2>` — Tailwind: `text-white font-medium`

Inline: `font-size: clamp(32px, 4vw, 56px)`, `line-height: 1.2`, `margin: 0`

Structure:
```
Relied on by enterprise teams<br />
<span gradient>from groundbreak to go-live.</span>
```

Gradient `<span>` inline styles:
```
background-image: linear-gradient(90deg, rgb(43,167,255), rgb(202,69,255) 50%, rgb(254,136,27))
-webkit-background-clip: text
background-clip: text
-webkit-text-fill-color: transparent
color: transparent
display: inline
```

`<p>` below heading — inline: `color: rgb(189, 174, 231)`, `font-size: clamp(14px, 1.25vw, 18px)`, `margin: 0`

Text: **"Built for operational clarity through constant change. Proven across 530+ MW of critical infrastructure."**

---

**Cards grid** — Tailwind: `w-full grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4`

Inline: `gap: 12px`, `position: relative`, `z-index: 1`

Maps over `CARDS`, renders `<ServiceCard key={card.label} {...card} />` for each.

---

**Bottom white fade overlay** (`position: absolute`, bottom of the section):

```
position: absolute
bottom: 0
left: 0
right: 0
height: 180px
background: linear-gradient(to bottom, transparent, rgb(255, 255, 255))
pointer-events: none
```

---

**Card hover animation summary:**
- Top image: default `opacity: 0.7`, `translateY(-30%)` — on hover: `opacity: 1`, `translateY(0)`
- Bottom dark gradient: default `opacity: 0`, `translateY(100%)` — on hover: `opacity: 1`, `translateY(0)`
- Inner text block: on hover shifts `translateY(-8px)`
- Button reveal: default `max-height: 0`, `opacity: 0`, `translateY(20px)` — on hover: `max-height: 80px`, `opacity: 1`, `translateY(0)`
- All transitions: `duration-500` (500ms)

## Daisy Sweet — Product [sites/daisy-sweet]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(43).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/daisy-sweet.webp

Build a standalone React + TypeScript + Tailwind CSS section component. This is a fragrance product showcase split into two halves: a sky-blue product panel on the left and a looping video on the right. On mobile it stacks vertically (product panel on top, video strip below). Every value below is exact — do not approximate.

### Tech Stack
- React 18 + TypeScript
- Tailwind CSS 3 (default config, default breakpoints: `sm:640px`, `md:768px`)
- Vite
- No extra packages. No icon libraries needed for this section.

### Constants

```ts
const TEXT_COLOR = '#000000';
const BG_BLUE = '#4BB3ED';
const EASE = 'cubic-bezier(0.22, 1, 0.36, 1)';
```

### Animation Helper

Create a reusable function that returns an object with a `style` property for CSS fade+slide entrance animations:

```ts
function anim(visible: boolean, delay: number, opts: { y?: number; x?: number; duration?: number } = {}) {
  const { y = 20, x = 0, duration = 1600 } = opts;
  const translateFrom = y !== 0 ? `translateY(${y}px)` : x !== 0 ? `translateX(${x}px)` : 'none';
  return {
    style: {
      opacity: visible ? 1 : 0,
      transform: visible ? 'translate(0,0)' : translateFrom,
      transition: `opacity ${duration}ms ${EASE} ${delay}ms, transform ${duration}ms ${EASE} ${delay}ms`,
    } as React.CSSProperties,
  };
}
```

This returns `{ style: {...} }` so it can be spread as `{...anim(...)}` directly onto elements (which sets the `style` prop), OR accessed as `anim(...).style` when merging with other inline styles.

### Product Data

```ts
const SCENT_PRODUCT = {
  name: 'Eau So Sweet',
  size: '100 ml / 3.3 oz',
  image: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260511_151640_5b4a7bf8-4eb2-4a49-aa63-17a9bb642b88.png&w=1280&q=85',
  notes: [
    { label: 'Fruity top', ingredient: 'WHITE RASPBERRIES' },
    { label: 'Floral heart', ingredient: 'DAISY TREE PETALS' },
    { label: 'Feminine base', ingredient: 'SUGAR MUSKS' },
  ],
};
```

### Video URL (exact, verbatim)

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151802_1bbf9a81-a7cb-4be1-b858-f1cd92b62b96.mp4
```

---

### Component: `ProductPanel`

Accepts props:

```ts
{
  bg: string;                                         // background color
  product: { name: string; size: string; image: string };
  notes: { label: string; ingredient: string }[];
  visible: boolean;                                   // controls animation trigger
  noteStyle?: 'normal' | 'bold';                      // defaults to 'normal'
}
```

### Outer wrapper
```
<div
  className="relative flex flex-col px-6 md:px-8 pt-6 md:pt-8 pb-8 md:pb-10"
  style={{ backgroundColor: bg, minHeight: '100%' }}
>
```

### 1. Top labels row
```
<div
  className="flex items-start justify-between mb-auto"
  {...anim(visible, 0, { y: 12, duration: 1400 })}
>
```
- Left label: `<span className="text-xs font-normal" style={{ color: TEXT_COLOR }}>` — text is `'Daisy love'` when `noteStyle !== 'bold'`, `'Daisy wild'` when `noteStyle === 'bold'`.
- Right label: same classes/style — text is `'Sweet'` when normal, `'Playful'` when bold.

### 2. Product image block
```
<div
  className="flex flex-col items-center py-8"
  style={{ flex: 1, justifyContent: 'center', ...anim(visible, 300, { y: 40, duration: 1800 }).style }}
>
```

### Image container
```
<div
  className="overflow-hidden"
  style={{
    width: 'clamp(140px, 40%, 220px)',
    aspectRatio: '220/340',
    backgroundColor: '#D9D9D9',
    borderRadius: '2px',
    flexShrink: 0,
  }}
>
  <img
    src={product.image}
    alt={product.name}
    style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
  />
</div>
```

### Caption (below image)
```
<div className="text-center mt-4" {...anim(visible, 600, { y: 10, duration: 1400 })}>
  <p className="text-sm font-normal" style={{ color: TEXT_COLOR }}>{product.name}</p>
  <p className="text-xs font-normal mt-1" style={{ color: TEXT_COLOR }}>{product.size}</p>
</div>
```

### 3. Bottom row — notes + button

```
<div className="flex items-end justify-between gap-4 flex-wrap">
```

### Notes column (left side)
```
<div className="flex flex-col gap-0.5" {...anim(visible, 900, { y: 16, duration: 1400 })}>
```
For each note object, render a `<div key={note.ingredient}>` containing two `<p>` tags:
- Label: `<p className="text-xs leading-snug" style={{ color: TEXT_COLOR, fontWeight: noteStyle === 'bold' ? 700 : 400 }}>{note.label}</p>`
- Ingredient: `<p className="text-xs font-bold tracking-widest uppercase leading-snug" style={{ color: TEXT_COLOR }}>{note.ingredient}</p>`

### SHOP NOW button (right side)
```
<button
  className="text-xs font-bold tracking-widest uppercase border px-6 py-3 relative group shrink-0"
  style={{
    color: TEXT_COLOR,
    borderColor: TEXT_COLOR,
    backgroundColor: 'transparent',
    ...anim(visible, 1150, { y: 16, duration: 1400 }).style,
  }}
>
  <span className="relative z-10 group-hover:text-black transition-colors duration-500">SHOP NOW</span>
  <span
    className="absolute inset-0 origin-left scale-x-0 group-hover:scale-x-100 transition-transform duration-500 ease-out"
    style={{ backgroundColor: '#ffffff' }}
  />
</button>
```

The button has a `1px` solid border (via Tailwind `border` class) colored `#000000`. On hover, a white fill (`#ffffff`) scales in from the left edge (`origin-left`, `scale-x-0` -> `scale-x-100`) over 500ms with `ease-out`. Text stays `z-10` above the fill. The `group-hover:text-black` class ensures text remains black over the white fill.

---

### Component: `ScentFinderSection`

### Visibility trigger
```ts
const ref = useRef<HTMLDivElement>(null);
const [visible, setVisible] = useState(false);

useEffect(() => {
  const observer = new IntersectionObserver(
    ([entry]) => { if (entry.isIntersecting) setVisible(true); },
    { threshold: 0.15 }
  );
  if (ref.current) observer.observe(ref.current);
  return () => observer.disconnect();
}, []);
```

Once 15% of the section is visible, `visible` flips to `true` permanently (one-shot, never resets to false). This triggers the staggered animations inside `ProductPanel`.

### Layout structure

```
<section ref={ref} className="relative w-full">
  <div className="flex flex-col md:grid md:min-h-screen" style={{ gridTemplateColumns: '1fr 1fr' }}>
```

Three children inside:

### Child 1: ProductPanel (always visible)
```
<ProductPanel
  bg={BG_BLUE}
  product={SCENT_PRODUCT}
  notes={SCENT_PRODUCT.notes}
  visible={visible}
/>
```
Called with `noteStyle` defaulting to `'normal'` (not passed explicitly), so top labels read `Daisy love` / `Sweet`, and note labels use `fontWeight: 400`.

### Child 2: Desktop video panel (hidden below `md`)
```
<div className="hidden md:block relative overflow-hidden" style={{ backgroundColor: '#111', minHeight: '100%' }}>
  <video autoPlay muted loop playsInline className="absolute inset-0 w-full h-full object-cover">
    <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151802_1bbf9a81-a7cb-4be1-b858-f1cd92b62b96.mp4" type="video/mp4" />
  </video>
</div>
```
- Background `#111` shows while video loads.
- `minHeight: '100%'` ensures the video panel fills the grid row height.
- Video is `position:absolute inset-0`, fills and covers the container.

### Child 3: Mobile video strip (hidden at `md` and above)
```
<div className="md:hidden relative overflow-hidden" style={{ height: '75vw', backgroundColor: '#111' }}>
  <video autoPlay muted loop playsInline className="absolute inset-0 w-full h-full object-cover">
    <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151802_1bbf9a81-a7cb-4be1-b858-f1cd92b62b96.mp4" type="video/mp4" />
  </video>
</div>
```
- Fixed aspect via `height: 75vw` (so on a 390px phone it is ~293px tall).
- Same video URL, same absolute-cover pattern, same `#111` background.

### Responsive behavior summary

| Viewport | Layout | Product panel | Video |
|---|---|---|---|
| < 768px (below `md`) | `flex flex-col` | Full width, natural height | Full width, `height: 75vw`, below the panel |
| >= 768px (`md` and up) | `grid 1fr 1fr`, `min-h-screen` | Left half, fills grid height | Right half, fills grid height |

On mobile, the desktop video div is `hidden` and the mobile strip is shown. On desktop, the mobile strip is `hidden` and the desktop div is shown. Both use the identical video source.

---

### Animation Stagger Timeline (all triggered when section becomes 15% visible)

| Element | Delay | Duration | Direction | Distance |
|---|---|---|---|---|
| Top labels row | 0ms | 1400ms | translateY | 12px |
| Product image block | 300ms | 1800ms | translateY | 40px |
| Caption (name + size) | 600ms | 1400ms | translateY | 10px |
| Notes column | 900ms | 1400ms | translateY | 16px |
| SHOP NOW button | 1150ms | 1400ms | translateY | 16px |

All use easing `cubic-bezier(0.22, 1, 0.36, 1)`. Each element starts `opacity: 0` + translated down by the specified distance, then transitions to `opacity: 1` + `translate(0,0)`.

### Fonts
No custom fonts or Google Fonts. Uses Tailwind's default sans-serif system font stack for all text in this section. No serif fonts appear here.

### SVGs / Icons
None. This section contains zero SVG elements or icon components.

### Colors used
- `#4BB3ED` — product panel background (sky blue)
- `#000000` — all text color, button border color
- `#D9D9D9` — image placeholder background (visible while image loads)
- `#111` — video panel background (visible while video loads)
- `#ffffff` — button hover fill

## Daisy Wild — Product [sites/daisy-wild]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(32).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/daisy-wild.webp

Build a standalone React + TypeScript + Tailwind CSS section component. This is a fragrance product showcase split into two halves: a looping video on the LEFT and a lime-green product panel on the RIGHT. On mobile it stacks vertically with the product panel ABOVE the video (achieved via `flex-col-reverse`). Every value below is exact.

### Tech Stack
- React 18 + TypeScript
- Tailwind CSS 3 (default config, default breakpoints: `sm:640px`, `md:768px`)
- Vite
- No extra packages. No icon libraries needed.

### Constants

```ts
const TEXT_COLOR = '#000000';
const BG_LIME = '#BDE84F';
const EASE = 'cubic-bezier(0.22, 1, 0.36, 1)';
```

### Animation Helper

```ts
function anim(visible: boolean, delay: number, opts: { y?: number; x?: number; duration?: number } = {}) {
  const { y = 20, x = 0, duration = 1600 } = opts;
  const translateFrom = y !== 0 ? `translateY(${y}px)` : x !== 0 ? `translateX(${x}px)` : 'none';
  return {
    style: {
      opacity: visible ? 1 : 0,
      transform: visible ? 'translate(0,0)' : translateFrom,
      transition: `opacity ${duration}ms ${EASE} ${delay}ms, transform ${duration}ms ${EASE} ${delay}ms`,
    } as React.CSSProperties,
  };
}
```

### Product Data

```ts
const WILD_PRODUCT = {
  name: 'Eau So Extra',
  size: '100 ml / 3.3 oz',
  image: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260511_151621_4fba6892-ed21-4c2e-8cb3-0bd2ec2abefa.png&w=1280&q=85',
  notes: [
    { label: 'Top', ingredient: 'BANANA BLOSSOM ACCORD' },
    { label: 'Heart', ingredient: 'CHOCOLATE DAISY ACCORD' },
    { label: 'Base', ingredient: 'VETIVER OIL' },
  ],
};
```

### Video URL (exact, verbatim)

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151818_65bb22c5-33ae-4e23-85ea-0a3dd89957c2.mp4
```

---

### Component: `ProductPanel`

This is a reusable component shared with Section 2. For this section it is called with `noteStyle="bold"`.

### Props

```ts
{
  bg: string;
  product: { name: string; size: string; image: string };
  notes: { label: string; ingredient: string }[];
  visible: boolean;
  noteStyle?: 'normal' | 'bold';   // defaults to 'normal'
}
```

### Outer wrapper
```jsx
<div
  className="relative flex flex-col px-6 md:px-8 pt-6 md:pt-8 pb-8 md:pb-10"
  style={{ backgroundColor: bg, minHeight: '100%' }}
>
```

### 1. Top labels row
```jsx
<div
  className="flex items-start justify-between mb-auto"
  {...anim(visible, 0, { y: 12, duration: 1400 })}
>
  <span className="text-xs font-normal" style={{ color: TEXT_COLOR }}>
    {noteStyle === 'bold' ? 'Daisy wild' : 'Daisy love'}
  </span>
  <span className="text-xs font-normal" style={{ color: TEXT_COLOR }}>
    {noteStyle === 'bold' ? 'Playful' : 'Sweet'}
  </span>
</div>
```

For this section (`noteStyle="bold"`), the labels read **"Daisy wild"** on the left and **"Playful"** on the right.

### 2. Product image block
```jsx
<div
  className="flex flex-col items-center py-8"
  style={{ flex: 1, justifyContent: 'center', ...anim(visible, 300, { y: 40, duration: 1800 }).style }}
>
```

### Image container
```jsx
<div
  className="overflow-hidden"
  style={{
    width: 'clamp(140px, 40%, 220px)',
    aspectRatio: '220/340',
    backgroundColor: '#D9D9D9',
    borderRadius: '2px',
    flexShrink: 0,
  }}
>
  <img
    src={product.image}
    alt={product.name}
    style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
  />
</div>
```

### Caption (below image)
```jsx
<div className="text-center mt-4" {...anim(visible, 600, { y: 10, duration: 1400 })}>
  <p className="text-sm font-normal" style={{ color: TEXT_COLOR }}>{product.name}</p>
  <p className="text-xs font-normal mt-1" style={{ color: TEXT_COLOR }}>{product.size}</p>
</div>
```

### 3. Bottom row — notes + button

```jsx
<div className="flex items-end justify-between gap-4 flex-wrap">
```

### Notes column (left side)
```jsx
<div className="flex flex-col gap-0.5" {...anim(visible, 900, { y: 16, duration: 1400 })}>
```
For each note, render `<div key={note.ingredient}>` with two `<p>`:
- Label: `<p className="text-xs leading-snug" style={{ color: TEXT_COLOR, fontWeight: noteStyle === 'bold' ? 700 : 400 }}>{note.label}</p>`
- Ingredient: `<p className="text-xs font-bold tracking-widest uppercase leading-snug" style={{ color: TEXT_COLOR }}>{note.ingredient}</p>`

For this section (`noteStyle="bold"`), the note LABELS ("Top", "Heart", "Base") render at `fontWeight: 700`. The ingredient lines are always `font-bold` regardless.

### SHOP NOW button (right side)
```jsx
<button
  className="text-xs font-bold tracking-widest uppercase border px-6 py-3 relative group shrink-0"
  style={{
    color: TEXT_COLOR,
    borderColor: TEXT_COLOR,
    backgroundColor: 'transparent',
    ...anim(visible, 1150, { y: 16, duration: 1400 }).style,
  }}
>
  <span className="relative z-10 group-hover:text-black transition-colors duration-500">SHOP NOW</span>
  <span
    className="absolute inset-0 origin-left scale-x-0 group-hover:scale-x-100 transition-transform duration-500 ease-out"
    style={{ backgroundColor: '#ffffff' }}
  />
</button>
```

Button: `1px` solid border colored `#000000`. On hover, a white fill scales from the left over 500ms. Text stays above (`z-10`).

---

### Component: `WildScentSection`

### Visibility trigger
```ts
const ref = useRef<HTMLDivElement>(null);
const [visible, setVisible] = useState(false);

useEffect(() => {
  const observer = new IntersectionObserver(
    ([entry]) => { if (entry.isIntersecting) setVisible(true); },
    { threshold: 0.15 }
  );
  if (ref.current) observer.observe(ref.current);
  return () => observer.disconnect();
}, []);
```

One-shot: once 15% visible, `visible` becomes `true` permanently, triggering all staggered animations.

### Layout structure

```jsx
<section ref={ref} className="relative w-full">
  <div className="flex flex-col-reverse md:grid md:min-h-screen" style={{ gridTemplateColumns: '1fr 1fr' }}>
```

**Critical difference from Section 2:** This uses `flex-col-reverse` (not `flex-col`). The DOM order is: video divs first, then ProductPanel. But on mobile, `flex-col-reverse` visually flips them so the product panel appears ABOVE the video.

### Three children inside (in DOM order):

### Child 1: Desktop video panel (left half on desktop, hidden below `md`)
```jsx
<div className="hidden md:block relative overflow-hidden" style={{ backgroundColor: '#111', minHeight: '100%' }}>
  <video autoPlay muted loop playsInline className="absolute inset-0 w-full h-full object-cover">
    <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151818_65bb22c5-33ae-4e23-85ea-0a3dd89957c2.mp4" type="video/mp4" />
  </video>
</div>
```

### Child 2: Mobile video strip (hidden at `md` and above)
```jsx
<div className="md:hidden relative overflow-hidden" style={{ height: '75vw', backgroundColor: '#111' }}>
  <video autoPlay muted loop playsInline className="absolute inset-0 w-full h-full object-cover">
    <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151818_65bb22c5-33ae-4e23-85ea-0a3dd89957c2.mp4" type="video/mp4" />
  </video>
</div>
```

### Child 3: ProductPanel (right half on desktop, visually on top on mobile)
```jsx
<ProductPanel
  bg={BG_LIME}
  product={WILD_PRODUCT}
  notes={WILD_PRODUCT.notes}
  visible={visible}
  noteStyle="bold"
/>
```

Called with `noteStyle="bold"`, which changes:
- Top labels: `"Daisy wild"` / `"Playful"` (instead of `"Daisy love"` / `"Sweet"`)
- Note labels: `fontWeight: 700` (instead of `400`)

---

### Responsive Behavior

| Viewport | Layout | Visual order (top to bottom / left to right) |
|---|---|---|
| < 768px | `flex flex-col-reverse` | Product panel (lime, full width) then video strip (`height: 75vw`, full width) |
| >= 768px | `grid 1fr 1fr`, `min-h-screen` | Video (left half) then Product panel (right half, lime) |

The `flex-col-reverse` trick: DOM order is [video-desktop, video-mobile, panel]. On mobile, `flex-col-reverse` reverses visual order to [panel, video-mobile, video-desktop(hidden)]. On desktop, `md:grid` overrides flex, and the grid places them left-to-right in DOM order: video left, panel right.

### Animation Stagger Timeline

All triggered when 15% of the section scrolls into view. Easing: `cubic-bezier(0.22, 1, 0.36, 1)`.

| Element | Delay | Duration | Direction | Distance |
|---|---|---|---|---|
| Top labels ("Daisy wild" / "Playful") | 0ms | 1400ms | translateY | 12px |
| Product image block | 300ms | 1800ms | translateY | 40px |
| Caption (name + size) | 600ms | 1400ms | translateY | 10px |
| Notes column | 900ms | 1400ms | translateY | 16px |
| SHOP NOW button | 1150ms | 1400ms | translateY | 16px |

Each element starts at `opacity: 0` + translated down, then transitions to `opacity: 1` + `translate(0,0)`.

### Colors Used

- `#BDE84F` — product panel background (lime green)
- `#000000` — all text, button border
- `#D9D9D9` — image placeholder background
- `#111` — video panel background (while loading)
- `#ffffff` — button hover fill

### Fonts
No custom or Google Fonts. Tailwind default sans-serif system stack for all text.

### SVGs / Icons
None in this section.

### Key Differences from Section 2 (ScentFinder)

| Aspect | Section 2 (ScentFinder) | Section 3 (WildScent) |
|---|---|---|
| Background color | `#4BB3ED` (sky blue) | `#BDE84F` (lime green) |
| Panel position (desktop) | LEFT half | RIGHT half |
| Video position (desktop) | RIGHT half | LEFT half |
| Flex direction (mobile) | `flex-col` (panel on top, video below) | `flex-col-reverse` (panel on top via reversal, video below) |
| Top labels | "Daisy love" / "Sweet" | "Daisy wild" / "Playful" |
| Note label weight | `fontWeight: 400` (normal) | `fontWeight: 700` (bold) |
| `noteStyle` prop | `'normal'` (default) | `'bold'` |
| Product name | Eau So Sweet | Eau So Extra |
| Product size | 100 ml / 3.3 oz | 100 ml / 3.3 oz |
| Video URL | `...151802_1bbf9a81...` | `...151818_65bb22c5...` |
| Notes content | Fruity top / WHITE RASPBERRIES, Floral heart / DAISY TREE PETALS, Feminine base / SUGAR MUSKS | Top / BANANA BLOSSOM ACCORD, Heart / CHOCOLATE DAISY ACCORD, Base / VETIVER OIL |

## Beauty Products — Products [sites/beauty-products]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(54).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/beauty-products.webp

---

**Prompt to recreate the "Best Sellers" section:**

> Build a "Best Sellers" product carousel section in React + Tailwind CSS with the following exact specifications:
>
> **Layout & Background:**
> - Full-width section with background color `#F9F4F0`, text color black.
> - `min-h-screen`, using flexbox column layout with `justify-center` to vertically center all content.
> - Horizontal padding: `px-4` on mobile, `sm:px-6`, `lg:px-10`. Vertical padding: `py-12`, `sm:py-16`.
>
> **Tab Header:**
> - Two tab buttons side by side in a flex row with `gap-4` (mobile) / `gap-6` (sm+).
> - Bottom margin: `mb-8` (mobile), `sm:mb-12`.
> - Each tab button contains a flex row with `gap-3` (mobile) / `gap-4` (sm+).
> - The active tab shows a filled circle indicator: `w-5 h-5` (mobile) / `sm:w-6 sm:h-6`, `rounded-full`, `bg-[#1a1a1a]`. This circle animates in with a custom `scale-in` keyframe animation: `from { transform: scale(0); opacity: 0 } to { transform: scale(1); opacity: 1 }` using `cubic-bezier(0.34, 1.56, 0.64, 1)` over `0.3s`. The circle only renders when that tab is active (conditional rendering, not visibility toggle).
> - Tab text is `text-2xl` (mobile), `sm:text-4xl`, `md:text-5xl`, `font-medium`. Active tab text: `text-[#1a1a1a]`. Inactive tab text: `text-gray-400` with `group-hover:text-gray-600`. Color transition: `duration-300`.
> - Tab labels (lowercase): "best sellers" and "sets". Default active tab: "best sellers".
> - The entire tab header block uses an IntersectionObserver-based reveal animation (threshold `0.1`): transitions from `opacity-0 translate-y-6` to `opacity-100 translate-y-0` over `duration-800 ease-out`.
>
> **Product Carousel:**
> - Horizontal scrollable flex container with `overflow-x-auto`. Hidden scrollbar using both `.scrollbar-hide::-webkit-scrollbar { display: none }` CSS class and inline styles `scrollbarWidth: 'none', msOverflowStyle: 'none'`.
> - Cursor: `cursor-grab`, `active:cursor-grabbing`.
> - Vertical mouse wheel events are intercepted and converted to horizontal scroll (`e.preventDefault()` + `el.scrollLeft += e.deltaY`), added with `{ passive: false }`.
> - Each product card is `flex-shrink-0` with widths: `w-[260px]` mobile, `sm:w-[280px]`, `md:w-[300px]`, `lg:w-[calc(25%-1px)]`.
> - Cards have `border border-gray-200` with `-ml-[1px]` to collapse borders (first card: `first:ml-0`).
> - Card padding: `pt-4 pb-6`.
> - Each card has a staggered reveal animation tied to the same IntersectionObserver: `opacity-0 translate-y-8` to `opacity-100 translate-y-0`, `duration-500 ease-out`, with `transitionDelay` of `200 + (index * 80)ms`.
>
> **Card Internal Layout:**
> - **Category label area** (top): `px-4`, fixed `h-12`. Category text: `text-xs font-medium tracking-wider uppercase`. Optional subcategory below: `text-xs text-gray-500 uppercase mt-0.5`.
> - **Product image**: `mx-4`, `aspect-[3/4]`, `rounded-lg overflow-hidden`, background `bg-[#F9F4F0]`. Image fills with `object-cover`. Hover: `scale-105` over `duration-500`.
> - **Product info** (below image): `mt-4 text-center`. Name: `text-sm`, hover color transition to `text-[#1a1a1a]` over `duration-300`. Price row: flex centered with `gap-2 mt-1`. Current price: `text-sm`. Old price (if exists): `text-sm text-gray-400 line-through`.
>
> **7 Products with exact data:**
> 1. Category: "ILLUMINATE" | Name: "Illuminating cleansing gel" | Price: "36,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_193822_8c95f5ed-b142-454f-ab87-59ad1f09e758.png&w=1280&q=85`
> 2. Category: "UNIFY" | Subcategory: "TIGHTEN PORES" | Name: "Unifying serum spray" | Price: "34,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194048_278bf3cc-7d1f-43c1-9dc7-73d8fcd9949c.png&w=1280&q=85`
> 3. Category: "NATURAL GLOW" | Name: "Super glow set" | Price: "92,00" | Old price: "99,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194058_d89610de-05f8-45e4-8196-0680296c565a.png&w=1280&q=85`
> 4. Category: "PROTECT" | Subcategory: "ILLUMINATE" | Name: "Radiance day oil" | Price: "59,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194112_1763cbb2-3171-4ad3-9f38-1b738b8f1bb6.png&w=1280&q=85`
> 5. Category: "HYDRATE" | Subcategory: "NOURISH" | Name: "Deep moisture cream" | Price: "48,00" | Image: same as product 1
> 6. Category: "RENEW" | Name: "Night repair elixir" | Price: "72,00" | Old price: "79,00" | Image: same as product 2
> 7. Category: "SMOOTH" | Subcategory: "REFINE" | Name: "Gentle exfoliating toner" | Price: "42,00" | Image: same as product 3
>
> **Scroll Progress Bar:**
> - Centered below carousel: `mt-8` (mobile), `sm:mt-10`, `mx-auto`, `max-w-[280px]`.
> - Track: `h-[2px]`, `bg-gray-300`, `rounded-full`, `relative overflow-hidden`.
> - Thumb: absolutely positioned, `h-full`, `bg-[#1a1a1a]`, `rounded-full`, fixed `width: 30%`. Position is driven by a `translateX` transform calculated as `scrollProgress * (100 / 0.3)%`, where `scrollProgress` = `scrollLeft / (scrollWidth - clientWidth)`. Transition: `duration-150 ease-out`.
>
> **Required CSS (in global stylesheet):**
> ```css
> .scrollbar-hide::-webkit-scrollbar { display: none; }
> @keyframes scale-in {
>   from { transform: scale(0); opacity: 0; }
>   to { transform: scale(1); opacity: 1; }
> }
> .animate-scale-in {
>   animation: scale-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
> }
> ```
>
> **IntersectionObserver hook (`useInView`):**
> - Accepts a `threshold` parameter (default `0.15`), uses a ref.
> - Observes the element; once `isIntersecting` is true, sets `isVisible = true` and unobserves.
> - Returns `{ ref, isVisible }`.
> - This section calls it with threshold `0.1`.

---

## Projects Catalog — Projects [sites/projects-catalog]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(36).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/projects-catalog.webp

**Prompt:**

Create a "Projects" section using React, Tailwind CSS, and **framer-motion** (`useScroll`, `useTransform`, `motion`). The site uses **Google Font "Kanit"** (weights 300-900) and a dark background `#0C0C0C`. This section overlaps the previous white section slightly with a negative top margin and rounded top corners.

---

### Section Container

- Background: `#0C0C0C`
- Padding: `px-5 sm:px-8 md:px-10`
- Top border radius: `rounded-t-[40px] sm:rounded-t-[50px] md:rounded-t-[60px]`
- Negative margin to overlap section above: `-mt-10 sm:-mt-12 md:-mt-14`
- `position: relative`, `z-index: 10`
- Uses framer-motion `useScroll` on the entire section ref with `offset: ['start start', 'end end']` to drive the stacking card animation

### Section Heading

- Wrapped in a `flex flex-col items-center py-20 sm:py-24 md:py-32`
- Text: `"Project"`
- Uses CSS class `hero-heading` which applies:
  ```css
  .hero-heading {
    background: linear-gradient(180deg, #646973 0%, #BBCCD7 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  ```
- `font-black uppercase leading-none tracking-tight text-center w-full`
- Font size: `clamp(3rem, 12vw, 160px)`
- Fade-in animation: `delay: 0, y: 40`

---

### 3 Project Cards (sticky stacking card effect)

Each card is a **sticky card** that stacks on top of the previous card as you scroll, with a subtle scale-down effect.

### Project Data:

**Project 01 -- "Nextlevel Studio" (Client)**
- col1 images (left column, 2 images stacked vertically):
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055344_5eff02e0-87a5-41ce-b64f-eb08da8f33db.png&w=1280&q=85
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055431_11d841fd-8b41-46a5-82e4-b04f2407a7d8.png&w=1280&q=85
  ```
- col2 image (right column, single tall image):
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055451_e317bf2d-28d4-48cc-86b0-6f72f25b6327.png&w=1280&q=85
  ```

**Project 02 -- "Aura Brand Identity" (Personal)**
- col1 images:
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055654_911201c5-36d9-4bc6-bac7-331adfce159f.png&w=1280&q=85
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055723_5ceda0b8-d9c2-4665-b2e3-83ba19ba76d1.png&w=1280&q=85
  ```
- col2 image:
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055753_adc5dcbd-a8e6-49c0-b43a-9b030d835cea.png&w=1280&q=85
  ```

**Project 03 -- "Solaris Digital" (Client)**
- col1 images:
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055759_963cfb0b-4bd1-4b0f-9d0a-09bd6cf95b2f.png&w=1280&q=85
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_060108_438f781a-9846-4dcc-89ab-c4e6cb830f5b.png&w=1280&q=85
  ```
- col2 image:
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055818_9d062121-ad7e-46b9-999a-1a6a692ef1ee.png&w=1280&q=85
  ```

---

### Sticky Stacking Card Animation (framer-motion)

Each card's outer wrapper:
- `height: 85vh`, `display: flex, align-items: start, justify-content: center`
- `position: sticky`, `top: 24px` (md: `top: 32px`)

The inner `motion.div` card:
- `position: absolute`, `width: 100%`, `max-width: 1760px`
- `transform-origin: top`
- Each card is offset vertically: `top: ${index * 28}px` so they peek behind each other
- **Scale animation**: As the user scrolls through the section, earlier cards scale down slightly while later cards remain at scale 1
  - `rangeStart = index / totalCards`
  - `rangeEnd = 1`
  - `targetScale = 1 - (totalCards - 1 - index) * 0.03`
  - `scale = useTransform(progress, [rangeStart, rangeEnd], [1, targetScale])`
  - Example: Card 0 scales from 1 to 0.94, Card 1 scales from 1 to 0.97, Card 2 stays at 1

---

### Project Card Inner Layout

- Background: `#0C0C0C`
- Border: `2px solid #D7E2EA`
- Border radius: `rounded-[40px] sm:rounded-[50px] md:rounded-[60px]`
- Padding: `p-4 sm:p-6 md:p-8`
- `flex flex-col gap-6 sm:gap-8 md:gap-10`

### Top row (header):
- `flex flex-col sm:flex-row sm:items-center sm:justify-between gap-6 sm:gap-4`
- **Left side** (`flex items-center gap-6 sm:gap-8 md:gap-10`):
  - **Number**: `text-[#D7E2EA] font-black uppercase leading-none`, size `clamp(3rem, 10vw, 140px)`
  - **Category + Name** (flex column, `gap-2 sm:gap-4 md:gap-6`):
    - Category: `text-[#D7E2EA] font-medium uppercase`, size `clamp(1rem, 2.2vw, 2.1rem)`
    - Name: `text-[#D7E2EA] font-light tracking-wide`, size `clamp(0.9rem, 2vw, 2rem)`
- **Right side**: "Live Project" button (see below)

### Image grid (bottom):
- `flex flex-col md:flex-row gap-4 md:gap-5 w-full`
- **Left column** (`flex flex-col gap-4 md:gap-5 w-full md:w-[40%]`):
  - Image 1: `w-full object-cover`, border-radius `rounded-[40px] sm:rounded-[50px] md:rounded-[60px]`, height `clamp(130px, 16vw, 230px)`
  - Image 2: `w-full object-cover`, border-radius `rounded-[30px] sm:rounded-[40px] md:rounded-[60px]`, height `clamp(160px, 22vw, 340px)`
- **Right column**: Single image, `w-full md:w-[60%] object-cover`, border-radius `rounded-[30px] sm:rounded-[40px] md:rounded-[60px]`, `self-stretch` (fills the height of both left images)

---

### "Live Project" Button

- Pill-shaped (`rounded-full`)
- Border: `2px solid #D7E2EA`
- No background (transparent)
- Padding: `px-8 py-3 sm:px-10 sm:py-3.5`
- Text: `"Live Project"`, `text-[#D7E2EA] font-medium uppercase tracking-widest`, size `text-sm sm:text-base`
- Hover: `bg-[#D7E2EA]/10`, Active: `bg-[#D7E2EA]/20`, transition 200ms
- If href provided, renders as `<a>` with `target="_blank" rel="noopener noreferrer"`
- All cards link to `#`

---

### FadeIn Component (reusable, framer-motion)

- Props: `delay`, `duration` (default 0.7), `x` (default 0), `y` (default 30), `className`, `style`, `as` (HTML element tag, default `div`)
- Uses `motion.create()` to make any HTML element animatable
- Variants: `hidden` = `{ opacity: 0, x, y }`, `visible` = `{ opacity: 1, x: 0, y: 0 }`
- Easing: `[0.25, 0.1, 0.25, 1]`
- Viewport trigger: `{ once: true, margin: "50px", amount: 0 }`

---

**Font (loaded in HTML head):**
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Kanit:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

CSS base: `font-family: 'Kanit', sans-serif` on html/body.

---

## Agency Services — Services [sites/agency-services]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(14).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/agency-services.webp

**Prompt:**

Create a "Services" section using React, Tailwind CSS, and **framer-motion**. The site uses **Google Font "Kanit"** (weights 300-900) and a dark page background `#0C0C0C`.

**Section layout:**
- This section sits on top of the dark background below it, using a white card-like appearance with rounded top corners
- `display: flex, flex-direction: column`
- Padding: `px-5 sm:px-8 md:px-10 py-20 sm:py-24 md:py-32`
- Top border radius: `rounded-t-[40px] sm:rounded-t-[50px] md:rounded-t-[60px]`
- Background color: `#FFFFFF` (white)

**Heading "Services":**
- `font-black uppercase leading-none tracking-tight text-center w-full`
- Font size: `clamp(3rem, 12vw, 160px)`
- Color: `#0C0C0C`
- Margin bottom: `mb-16 sm:mb-20 md:mb-28`
- Fade-in animation: `delay: 0, y: 40`

**Services list (5 items, vertically stacked, centered, max-w-5xl):**

Each service has a number, name, and description. The services are:

```
01 — 3D Modeling
"Creation of detailed objects, characters, or environments tailored to specific client needs, ideal for games, products, and visualizations."

02 — Rendering
"High-quality, photorealistic renders that showcase designs with custom lighting, textures, and materials to bring concepts to life."

03 — Motion Design
"Dynamic animations and motion graphics that add energy and storytelling to brands, products, and digital experiences."

04 — Branding
"Crafting cohesive visual identities — from logos to full brand systems — that communicate a clear and memorable presence."

05 — Web Design
"Designing clean, modern, and conversion-focused websites with attention to layout, typography, and user experience."
```

**Service item layout:**

Each item is wrapped in a FadeIn with staggered delay (`i * 0.1`) and `y: 30`.

- **Divider line** between items (not above the first): `border-top: 1px solid rgba(12, 12, 12, 0.15)`, full width
- **Row layout**: `display: flex, align-items: start, gap-6 sm:gap-8 md:gap-10, py-8 sm:py-10 md:py-12, w-full`
- **Left side -- Number**:
  - `font-black uppercase leading-none flex-shrink-0`
  - Font size: `clamp(3rem, 10vw, 140px)`
  - Color: `#0C0C0C`
  - Displays the zero-padded number (01, 02, etc.)
- **Right side -- Name + Description** (flex column, `gap-2 sm:gap-4 md:gap-5, pt-1`):
  - **Name**: `font-medium uppercase`, size `clamp(1rem, 2.2vw, 2.1rem)`, color `#0C0C0C`
  - **Description**: `font-light leading-relaxed max-w-2xl`, size `clamp(0.85rem, 1.6vw, 1.25rem)`, color `#0C0C0C` with `opacity: 0.6`

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

## Arceage Services — Services [sites/arceage-services]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(50).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/arceage-services.webp

Create a React + Tailwind CSS v4 + Motion ("motion/react") image/services section component. Use Vite as the bundler. Fully mobile responsive.

### Fonts

Import from Google Fonts in your global CSS:
```
@import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Instrument+Serif:ital@0;1&display=swap');
```

Define two Tailwind v4 theme fonts:
- `--font-sans: "Barlow", ui-sans-serif, system-ui, sans-serif;` (primary UI font via `font-sans`)
- `--font-dm-serif: "Instrument Serif", serif;` (accent/poetic italic font via `font-dm-serif`)

The page wrapper uses `bg-black font-sans text-white`.

### Dependencies

- `react` v19
- `motion` (npm package "motion", import from `motion/react` -- provides `motion`)
- `@lottiefiles/react-lottie-player` (provides `Player` component, import from `@lottiefiles/react-lottie-player`)
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

### Section Structure

The section is a `<section>` with:
- `id="services"`
- Classes: `w-full relative overflow-hidden flex flex-col justify-center`

---

### Layer 1: Full-bleed Background Image

An absolutely-positioned div covering the entire section:
- Wrapper: `absolute inset-0 z-0`
- Image: `<img>` with:
  - `src="https://github.com/dsMagnatov/Acreage-landing-assets/blob/main/1.jpg?raw=true"`
  - `alt="Agriculture Field"`
  - Classes: `w-full h-full object-cover`
  - `referrerPolicy="no-referrer"`

---

### Layer 2: Content Container

A `div` positioned above the background:
- Classes: `relative z-10 w-full mx-auto px-6 md:px-12 lg:px-[120px] py-8 md:py-24 flex flex-col h-full justify-between gap-4 md:gap-24`

---

### Top Content: Headline, Subheadline & Button (3-column grid)

Grid wrapper: `grid grid-cols-1 md:grid-cols-3 gap-12 md:gap-16 w-full items-end`

**Left 2 columns (headline area):**
- Wrapped in `motion.div` with:
  - `initial={{ opacity: 0, y: 20 }}`
  - `whileInView={{ opacity: 1, y: 0 }}`
  - `viewport={{ once: true, margin: "-100px" }}`
  - `transition={{ duration: 0.6, ease: "easeOut" }}`
  - Classes: `md:col-span-2`

- **Heading (h2):**
  - Classes: `text-[clamp(1.5rem,4vw,3.5rem)] font-medium tracking-tight text-white mb-6 leading-[1.1] max-w-[800px]`
  - Content:
    - `<Typewriter text="A Highly Efficient, Precision-Driven Harvesting Process Built For " delay={0} speed={0.012} />`
    - Then a `<span className="font-dm-serif italic font-normal">` wrapping `<Typewriter text="Maximum Yield" delay={0.8} speed={0.012} />`
  - "Maximum Yield" renders in Instrument Serif italic as the accent font.

- **Subheadline (p):**
  - Classes: `text-lg md:text-[24px] text-white/80 font-light tracking-wide`
  - Content: `<Typewriter text="Precision in every pass." delay={0.1} speed={0.012} />`

**Right column (desktop-only button):**
- Wrapped in `motion.div` with:
  - `initial={{ opacity: 0, y: 20 }}`
  - `whileInView={{ opacity: 1, y: 0 }}`
  - `viewport={{ once: true, margin: "-100px" }}`
  - `transition={{ duration: 0.6, delay: 0.1, ease: "easeOut" }}`
  - Classes: `hidden md:flex justify-end w-full max-w-[421px] pb-1`
- **Button:**
  - `onClick` scrolls smoothly to `#contact` section
  - Classes: `px-6 py-2.5 rounded-full bg-white text-black hover:bg-black hover:text-white transition-colors duration-300 text-sm tracking-wide font-medium`
  - Text: "Schedule Service"

---

### Bottom Content: 3 Feature Columns ("How it works")

Grid wrapper: `grid grid-cols-1 md:grid-cols-3 gap-12 md:gap-16 w-full md:mt-[200px]`

Each of the 3 columns follows the same pattern, each wrapped in `motion.div`:
- `initial={{ opacity: 0, y: 20 }}`
- `whileInView={{ opacity: 1, y: 0 }}`
- `viewport={{ once: true, margin: "-100px" }}`
- `transition={{ duration: 0.6, delay: 0.1, ease: "easeOut" }}`

Each column structure (`flex flex-col`):

1. **Lottie Icon** -- `div` with classes `w-12 h-12 mb-6 flex items-center justify-center overflow-hidden`, containing a `<Player>` component:
   - `loop`, `autoplay`
   - `style={{ width: '48px', height: '48px', filter: 'brightness(0) invert(1)' }}` (makes the icon white)

2. **Divider line** -- `div` with classes `w-full h-px bg-white/20 mb-6`

3. **Title (h3)** -- classes `text-2xl font-medium text-white mb-3`, content is a `<Typewriter>`

4. **Description (p)** -- classes `text-sm text-white/70 leading-relaxed max-w-[340px]`, content is a `<Typewriter>`

The 3 columns with their specific content:

**Column 1** (`max-w-[420px]`):
- Lottie src: `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/curry.json`
- Title: "Sustainable Crop Care"
- Description: "Nurturing your fields with eco-friendly practices to ensure healthy growth and robust yields."

**Column 2** (`max-w-[420px]`):
- Lottie src: `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/tractor.json`
- Title: "Advanced Machinery"
- Description: "Deploying state-of-the-art tractors and harvesters for maximum efficiency and speed."

**Column 3** (`max-w-[421px]`):
- Lottie src: `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/beetle.json`
- Title: "Smart Pest Management"
- Description: "Protecting your harvest by monitoring and managing field ecosystems with precision."

---

### Mobile-Only Button (below the 3 columns)

- Wrapped in `motion.div` with:
  - `initial={{ opacity: 0, y: 20 }}`
  - `whileInView={{ opacity: 1, y: 0 }}`
  - `viewport={{ once: true, margin: "-100px" }}`
  - `transition={{ duration: 0.6, delay: 0.2, ease: "easeOut" }}`
  - Classes: `flex md:hidden justify-start w-full`
- Same button as desktop version:
  - Classes: `px-6 py-2.5 rounded-full bg-white text-black hover:bg-black hover:text-white transition-colors duration-300 text-sm tracking-wide font-medium`
  - Text: "Schedule Service"
  - Scrolls to `#contact`

---

### Mobile Responsiveness Summary

- Section padding: `px-6 py-8` on mobile, `md:px-12 md:py-24`, `lg:px-[120px]`
- Grid stacks to single column on mobile (`grid-cols-1`), 3 columns at `md:` breakpoint
- Heading uses fluid type: `clamp(1.5rem, 4vw, 3.5rem)`
- Subheadline: `text-lg` mobile, `md:text-[24px]` desktop
- Desktop button hidden on mobile (`hidden md:flex`), replaced by mobile button at bottom (`flex md:hidden`)
- The `md:mt-[200px]` on the bottom grid creates vertical spacing between top content and feature columns on desktop only; on mobile, `gap-4` between sections keeps it compact
- Feature column max-widths (`max-w-[420px]` / `max-w-[421px]`) constrain reading width on all sizes

---

## Solace sign-in — Sign In Form [sites/solace-sign-in]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(66).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/solace-sign-in.webp

Build a fullscreen sign-in page with a **real-time liquid glass refraction effect** over a looping background video. Use React 18 + TypeScript + Tailwind CSS + Vite. Only dependency beyond React is `lucide-react`. Font: **Inter** (400, 500, 600, 700) from Google Fonts. Must be fully mobile responsive.

Create exactly 4 files: `src/LiquidGlass.tsx`, `src/FadeUp.tsx`, `src/App.tsx`, `src/index.css`.

---

### CRITICAL: The glass effect is NOT a CSS blur or backdrop-filter. It is a per-pixel canvas refraction engine. You MUST use the EXACT code below for `src/LiquidGlass.tsx`. Do not modify, simplify, or rewrite any of the math. Copy it verbatim:

```tsx
import { useEffect, useRef } from 'react';

/* eslint-disable @typescript-eslint/no-explicit-any */

class LiquidGlass {
  bg: any;
  opt: any;
  _lut: Float32Array | null;
  _lutKey: string;

  constructor(bgEl: any, options: any = {}) {
    this.bg = bgEl;
    this.opt = Object.assign({
      size: 120, shape: 'circle', rx: null,
      distort: 0.06, edgeCurl: 0.04, brightness: 0.06,
      specular: 0.20, border: 0.18, borderWidth: 1,
      sceneW: null, sceneH: null,
    }, options);
    this._lut = null;
    this._lutKey = '';
  }

  _getLUT(D: number) {
    const key = `${D}:${this.opt.distort}:${this.opt.edgeCurl}`;
    if (this._lut && this._lutKey === key) return this._lut;
    this._lutKey = key;
    const lut = new Float32Array(256);
    for (let i = 0; i < 256; i++) {
      const r = i / 255;
      if (r < 0.7) {
        lut[i] = r * (1.0 - this.opt.distort * (1 - r));
      } else {
        const t = (r - 0.7) / 0.3;
        lut[i] = Math.min(0.985, r * (1 - this.opt.distort * (1 - r)) + t * t * this.opt.edgeCurl);
      }
    }
    return (this._lut = lut);
  }

  _inShape(nx: number, ny: number, D: number) {
    const { shape, rx, size } = this.opt;
    if (shape === 'circle') return nx * nx + ny * ny < 1.0;
    const r = (rx != null ? rx : size * 0.3) / (size / 2);
    const ax = Math.abs(nx), ay = Math.abs(ny), lim = 1 - r;
    if (ax > 1 || ay > 1) return false;
    if (ax <= lim || ay <= lim) return true;
    return (ax - lim) ** 2 + (ay - lim) ** 2 <= r * r;
  }

  _normR(nx: number, ny: number, D: number) {
    const { shape, rx, size } = this.opt;
    if (shape === 'circle') return Math.sqrt(nx * nx + ny * ny);
    const r = (rx != null ? rx : size * 0.3) / (size / 2);
    const ax = Math.abs(nx), ay = Math.abs(ny), lim = 1 - r;
    const dx = Math.max(0, ax - lim), dy = Math.max(0, ay - lim);
    const dr = Math.sqrt(dx * dx + dy * dy);
    return (dr > 0 ? (lim + dr) : Math.max(ax, ay));
  }

  _cropBackground(cx: number, cy: number, D: number, sW: number, sH: number) {
    if (!this.bg) return null;
    const bw = this.bg.naturalWidth || this.bg.videoWidth;
    const bh = this.bg.naturalHeight || this.bg.videoHeight;
    if (!bw || !bh) return null;
    const R = D / 2;
    const off = document.createElement('canvas');
    off.width = D; off.height = D;
    const octx = off.getContext('2d')!;
    octx.drawImage(this.bg,
      Math.max(0, (cx - R) * (bw / sW)), Math.max(0, (cy - R) * (bh / sH)),
      D * (bw / sW), D * (bh / sH),
      0, 0, D, D
    );
    return octx.getImageData(0, 0, D, D).data;
  }

  _bilinear(sd: Uint8ClampedArray, x: number, y: number, D: number): [number, number, number] {
    const x0 = Math.floor(x), y0 = Math.floor(y);
    const x1 = Math.min(D - 1, x0 + 1), y1 = Math.min(D - 1, y0 + 1);
    const fx = x - x0, fy = y - y0;
    const w00 = (1 - fx) * (1 - fy), w10 = fx * (1 - fy), w01 = (1 - fx) * fy, w11 = fx * fy;
    const a = (y0 * D + x0) * 4, b = (y0 * D + x1) * 4, c = (y1 * D + x0) * 4, d = (y1 * D + x1) * 4;
    return [
      sd[a] * w00 + sd[b] * w10 + sd[c] * w01 + sd[d] * w11,
      sd[a + 1] * w00 + sd[b + 1] * w10 + sd[c + 1] * w01 + sd[d + 1] * w11,
      sd[a + 2] * w00 + sd[b + 2] * w10 + sd[c + 2] * w01 + sd[d + 2] * w11,
    ];
  }

  _drawOverlays(ctx: CanvasRenderingContext2D, D: number, R: number) {
    const { shape, rx, size, specular, border, borderWidth } = this.opt;
    const RX = rx != null ? rx : size * 0.3;
    ctx.save();
    ctx.beginPath();
    shape === 'circle' ? ctx.arc(R, R, R, 0, Math.PI * 2) : ctx.roundRect(0, 0, D, D, RX);
    ctx.clip();
    if (specular > 0) {
      const grd = ctx.createRadialGradient(R * 0.5, R * 0.28, 0, R * 0.5, R * 0.38, R * 0.45);
      grd.addColorStop(0, `rgba(255,255,255,${specular})`);
      grd.addColorStop(0.6, `rgba(255,255,255,${+(specular * 0.2).toFixed(3)})`);
      grd.addColorStop(1, 'rgba(255,255,255,0)');
      ctx.fillStyle = grd; ctx.fillRect(0, 0, D, D);
    }
    ctx.restore();
    if (border > 0) {
      const h = borderWidth / 2;
      ctx.save();
      ctx.strokeStyle = `rgba(255,255,255,${border})`;
      ctx.lineWidth = borderWidth;
      ctx.beginPath();
      shape === 'circle' ? ctx.arc(R, R, R - h, 0, Math.PI * 2)
        : ctx.roundRect(h, h, D - borderWidth, D - borderWidth, RX);
      ctx.stroke(); ctx.restore();
    }
  }

  render(canvas: HTMLCanvasElement, cx: number, cy: number, sW: number, sH: number) {
    const D = this.opt.size, R = D / 2;
    if (canvas.width !== D || canvas.height !== D) {
      canvas.width = D; canvas.height = D;
    }
    const sd = this._cropBackground(cx, cy, D, sW, sH);
    if (!sd) return false;
    const lut = this._getLUT(D);
    const ctx = canvas.getContext('2d')!;
    const out = ctx.createImageData(D, D);
    const od = out.data;
    const boost = this.opt.brightness;
    for (let py = 0; py < D; py++) {
      for (let px = 0; px < D; px++) {
        const nx = (px / R) - 1, ny = (py / R) - 1;
        if (!this._inShape(nx, ny, D)) {
          const i = (py * D + px) * 4; od[i] = od[i + 1] = od[i + 2] = od[i + 3] = 0; continue;
        }
        const normR = this._normR(nx, ny, D);
        const alpha = normR > 0.93 ? Math.max(0, (1 - normR) / 0.07) : 1.0;
        const theta = Math.atan2(ny, nx);
        const bend = lut[Math.min(255, Math.round(normR * 255))];
        const sx = Math.min(D - 1.001, Math.max(0, bend * Math.cos(theta) * R + R));
        const sy = Math.min(D - 1.001, Math.max(0, bend * Math.sin(theta) * R + R));
        const [rv, gv, bv] = this._bilinear(sd, sx, sy, D);
        const b = (1 + boost * Math.max(0, 1 - normR * 1.6)) * 0.75;
        const oi = (py * D + px) * 4;
        od[oi] = Math.min(255, rv * b); od[oi + 1] = Math.min(255, gv * b);
        od[oi + 2] = Math.min(255, bv * b); od[oi + 3] = Math.round(alpha * 255);
      }
    }
    ctx.clearRect(0, 0, D, D);
    ctx.putImageData(out, 0, 0);
    this._drawOverlays(ctx, D, R);
    return true;
  }
}

const GLASS_OPTIONS = {
  shape: 'roundedrect' as const,
  rx: 16,
  distort: 0.06,
  edgeCurl: 0.04,
  brightness: 0.06,
  specular: 0.20,
  border: 0.18,
  borderWidth: 1,
};

interface LiquidGlassCanvasProps {
  videoRef: React.RefObject<HTMLVideoElement | null>;
  sceneRef: React.RefObject<HTMLDivElement | null>;
  cardRef: React.RefObject<HTMLDivElement | null>;
}

export default function LiquidGlassCanvas({ videoRef, sceneRef, cardRef }: LiquidGlassCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let glass: LiquidGlass | null = null;
    let rafId: number | null = null;
    let isRunning = false;

    const BG_VIDEO = videoRef.current;
    const SCENE_EL = sceneRef.current;
    const CARD_EL = cardRef.current;
    const GLASS_CV = canvasRef.current;

    if (!BG_VIDEO || !SCENE_EL || !CARD_EL || !GLASS_CV) return;

    function setupGlass() {
      const sceneR = SCENE_EL!.getBoundingClientRect();
      const cardR = CARD_EL!.getBoundingClientRect();
      const W = Math.round(cardR.width);
      const H = Math.round(cardR.height);
      const isMobile = W < 640;
      const D = isMobile ? W : Math.max(W, H);

      GLASS_CV!.width = D;
      GLASS_CV!.height = D;

      glass = new LiquidGlass(BG_VIDEO, Object.assign({}, GLASS_OPTIONS, {
        size: D,
        sceneW: sceneR.width,
        sceneH: sceneR.height,
      }));
    }

    function renderLoop() {
      if (!glass) return;
      const sceneR = SCENE_EL!.getBoundingClientRect();
      const cardR = CARD_EL!.getBoundingClientRect();
      const cx = cardR.left - sceneR.left + cardR.width / 2;
      const cy = cardR.top - sceneR.top + cardR.height / 2;
      glass.opt.sceneW = sceneR.width;
      glass.opt.sceneH = sceneR.height;
      glass.render(GLASS_CV!, cx, cy, sceneR.width, sceneR.height);
      rafId = requestAnimationFrame(renderLoop);
    }

    function startGlass() {
      if (isRunning) return;
      isRunning = true;
      setupGlass();
      renderLoop();
    }

    function stopGlass() {
      if (rafId) cancelAnimationFrame(rafId);
      rafId = null;
      isRunning = false;
    }

    function handleResize() {
      stopGlass();
      setupGlass();
      renderLoop();
      isRunning = true;
    }

    BG_VIDEO.addEventListener('canplay', startGlass);
    BG_VIDEO.addEventListener('playing', startGlass);

    if (BG_VIDEO.readyState >= 3) startGlass();

    window.addEventListener('resize', handleResize);

    return () => {
      stopGlass();
      BG_VIDEO.removeEventListener('canplay', startGlass);
      BG_VIDEO.removeEventListener('playing', startGlass);
      window.removeEventListener('resize', handleResize);
    };
  }, [videoRef, sceneRef, cardRef]);

  return (
    <canvas
      ref={canvasRef}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        display: 'block',
        zIndex: 0,
      }}
    />
  );
}
```

---

### `src/FadeUp.tsx` -- Use this exact code:

```tsx
import { useEffect, useRef, useState, type ReactNode } from 'react';

interface FadeUpProps {
  children: ReactNode;
  delay?: number;
  duration?: number;
  className?: string;
}

export default function FadeUp({
  children,
  delay = 0,
  duration = 700,
  className = '',
}: FadeUpProps) {
  const [visible, setVisible] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const timer = setTimeout(() => setVisible(true), delay);
    return () => clearTimeout(timer);
  }, [delay]);

  return (
    <div
      ref={ref}
      className={className}
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? 'translateY(0)' : 'translateY(20px)',
        transition: `opacity ${duration}ms cubic-bezier(0.16, 1, 0.3, 1), transform ${duration}ms cubic-bezier(0.16, 1, 0.3, 1)`,
        transitionDelay: '0ms',
      }}
    >
      {children}
    </div>
  );
}
```

---

### `src/index.css`

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: 'Inter', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

---

### `index.html` head -- Load Inter font:

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
<title>Solace - Login</title>
```

---

### `src/App.tsx` -- The sign-in page layout

### Structure:

1. **Outermost div** (assign `sceneRef`): `relative min-h-screen w-full overflow-hidden`. Contains the video and the centered card.

2. **Background video** (assign `videoRef`): `absolute inset-0 h-full w-full object-cover`, autoPlay, muted, loop, playsInline, crossOrigin="anonymous".
   **Video URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260606_135315_5f9e8a4c-09bc-4a97-9f75-8a387d4258ee.mp4`

3. **Centering wrapper:** `relative z-10 flex min-h-screen items-center justify-center px-4 py-8`.

4. **Card** (assign `cardRef`): `relative w-full max-w-lg overflow-hidden rounded-2xl p-6 shadow-2xl sm:p-10 md:p-14`. No background color -- the `<LiquidGlassCanvas>` component is placed inside as the first child, acting as the card's background. All form content goes inside a `relative z-[1]` wrapper div above the canvas.

### Card content (each item wrapped in `<FadeUp>` with staggered delays 0, 100, 200, ... 900ms):

1. **(delay 0) Logo:** An inline SVG, 48x48, viewBox `0 0 256 256`, white fill. Path: `M 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 128 L 64 128 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z M 128 64 L 128 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 Z M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 128 0 L 192 0 Z`. Centered, `mb-6`.

2. **(delay 100) Heading:** `"Step back in!"` -- `text-3xl sm:text-4xl md:text-5xl font-medium tracking-tight text-center mb-2`. Text styled with `bg-gradient-to-r from-white to-purple-300 bg-clip-text text-transparent`.

3. **(delay 200) Subtitle:** `"Log in to continue your mindful exercises, calm routines, and wellness pathway"` -- `text-xs sm:text-sm md:text-base text-white/60 leading-relaxed text-center mb-6 sm:mb-8`. A `<br>` hidden on mobile (`hidden sm:inline`).

4. **(delay 300) Email field:** Label `"Email"` (`text-sm font-medium text-white/70 mb-1.5 block`). Input: `w-full rounded-xl border border-white/15 bg-white/5 px-4 py-3.5 text-sm text-white placeholder-white/30 outline-none transition-colors focus:border-white/30 focus:bg-white/10`. Placeholder: `"Your email address"`.

5. **(delay 400) Password field:** Same label/input styling. Password toggle button using `Eye`/`EyeOff` from lucide-react (size 20): `absolute right-4 top-1/2 -translate-y-1/2 text-white/40 hover:text-white/70`. Placeholder: `"Type your password"`.

6. **(delay 500) Remember/Reset row:** `flex items-center justify-between`. Left: custom checkbox using hidden input with Tailwind `peer` -- a 20x20 div (`h-5 w-5 rounded border border-white/20 bg-white/5 peer-checked:border-purple-400 peer-checked:bg-purple-500`) plus a white checkmark SVG (`absolute left-0.5 top-0.5 hidden peer-checked:block h-4 w-4`, path `M5 13l4 4L19 7` strokeWidth 3). Label `"Stay signed in"` (`text-sm text-white/70`). Right: `"Reset password?"` button (`text-sm text-white/70 hover:text-white`).

7. **(delay 600) Sign In button:** `w-full rounded-full bg-white py-3.5 text-base font-semibold text-gray-900 hover:bg-white/90 active:scale-[0.98]`.

8. **(delay 700) Divider:** `my-6 flex items-center gap-4` with two `h-px flex-1 bg-white/15` lines around `"Or"` (`text-sm text-white/40`).

9. **(delay 800) Google button:** `flex w-full items-center justify-center gap-3 rounded-full border border-white/15 bg-white/5 py-3.5 text-sm font-medium text-white hover:bg-white/10 active:scale-[0.98]`. Contains the standard 4-color Google "G" SVG (20x20, paths with fills `#4285F4`, `#34A853`, `#FBBC05`, `#EA4335`) and text `"Continue with Google"`.

10. **(delay 900) Join link:** `mt-6 text-center text-sm text-white/50` with `"New to this platform?"` and a `"Join Now"` button (`font-medium text-white hover:text-purple-300`).

---

The entire form is wrapped in `<form className="space-y-5">`. The form `onSubmit` handler calls `e.preventDefault()`.

---

## Aurora Onboard — Signup [sites/aurora-onboard]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/CleanShot_2026-05-07_at_15.40.21_2x_kfgapx.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/aurora-onboard.png

Please build a modern, two-column registration interface called "Aurora Sign Up". Use React, Tailwind CSS (v4), `motion/react` (for animations), and `lucide-react` (for icons). The app should be contained entirely in `App.tsx` and `index.css`.

### 1. Global Setup & CSS (`index.css`)
- Import the "Inter" font from Google Fonts (weights 300, 400, 500, 600, 700).
- Extend the Tailwind theme with `--font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;` and a custom color: `--color-brand-gray: #1A1A1A`.
- Apply base styles to the `body`: `@apply font-sans bg-black text-white antialiased;`.

### 2. Main Layout (`App.tsx` container)
- The `<main>` element should have: `flex min-h-screen w-full bg-black selection:bg-white/30 p-2 transition-all duration-500`. 
- On `lg` breakpoints: `lg:h-screen lg:overflow-hidden lg:p-4`.
- Split this container into a Left Column (Hero) and a Right Column (Form).

### 3. Left Column (Hero & Background Video)
- Width on large screens should be exactly `w-[52%]`. It should be hidden on mobile/tablet and only visible `lg:flex`.
- Styles: `relative flex-col items-center justify-end pb-32 px-12 rounded-3xl overflow-hidden shadow-2xl h-full`.
- **Background Video**: Add an absolutely positioned `<video>` tag (`inset-0`, `w-full`, `h-full`, `object-cover`). It must have `autoPlay`, `muted`, `loop`, and `playsInline`. 
- **CRITICAL**: The `<source>` MUST be exactly `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260506_081238_406ed0e3-5d83-436e-a512-0bbff7ec5b95.mp4` (`type="video/mp4"`).
- **CRITICAL**: Do NOT add any dark overlay, gradient, or tint mask over the video. Let it play purely without overlays.
- **Hero Content Container**: Place content over the video (`z-10 w-full max-w-xs space-y-8`).
- **Animations**: Use `motion.div` for a staggered reveal. The container should transition `opacity: 0` to `1` with `staggerChildren: 0.15` and `delayChildren: 0.2`. Every child element inside should fade in and slide up (`y: 10` to `y: 0`, duration `0.5`).
- **Brand/Logo**: A flex row with the `Circle` icon from Lucide (fill-white text-white) and the text "Aurora" (`text-xl font-semibold tracking-tight`).
- **Heading Block**: "Join Aurora" (`text-4xl font-medium tracking-tight whitespace-nowrap`). Below it, a description: "Follow these 3 quick phases to activate your space." (`text-white/60 text-sm leading-relaxed px-4`).
- **Steps**: Render a custom `<StepItem>` component three times. 
  1: "Register your identity" (active state)
  2: "Configure your studio"
  3: "Finalize your profile"

### 4. Right Column (Sign Up Form)
- A container with `flex-1 flex flex-col items-center justify-center py-12 lg:py-6 px-4 sm:px-12 lg:px-16 xl:px-24 overflow-y-auto lg:overflow-hidden`.
- **Animation**: Wrap the interior content in a `motion.div` that fades in (`opacity: 0` to `1`, `duration: 0.8`, `ease: "easeOut"`). Inner width `w-full max-w-xl`, spacing `space-y-8 lg:space-y-6 sm:space-y-10`.
- **Header**: "Create New Profile" (`text-3xl font-medium tracking-tight`). Subtitle: "Input your basic details to begin the journey." (`text-white/40 text-sm`).
- **Social Buttons**: A 2-column grid (`grid grid-cols-2 gap-4`). Render Google (`Chrome` icon) and Github (`Github` icon) using a `<SocialButton>` component.
- **Divider**: A horizontal line (`border-white/10`) with the text "Or" in the center (`bg-black px-4 text-xs font-medium text-white/40 uppercase tracking-widest`).
- **Form Layout**: 
  - First Name and Last Name in a 2-column grid.
  - Email (full width).
  - Password (full width) with a custom `lucide-react` `Eye` toggle icon in the absolute right of the input, and a tiny helper text "Requires at least 8 symbols."
  - **Submit Button**: "Create Account" (`w-full h-14 bg-white text-black font-semibold rounded-xl hover:bg-white/90 active:scale-[0.98] mt-4`).
  - **Footer Link**: "Member of the team? Log in".

### 5. Reusable Components to Create
Create these exact functional components at the bottom of the file:
1. **`<StepItem>`**: Takes `number`, `text`, and an optional `active` boolean.
   - If active: Apply `bg-white text-black border border-white`. The number circle is `bg-black text-white`.
   - If inactive: Apply `bg-brand-gray text-white border-none`. The number circle is `bg-white/10 text-white/40`.
2. **`<SocialButton>`**: Takes `icon` and `label`. Button has `bg-black border border-white/10 rounded-xl hover:bg-white/5`.
3. **`<InputGroup>`**: Takes `label`, `placeholder`, and `type`. The label is `text-sm font-medium text-white`. The input is `bg-brand-gray border-none rounded-xl h-11 px-4 text-white placeholder:text-white/20 focus:ring-2 focus:ring-white/20`.

Ensure the final code uses `export default function App()` at the top.

## NovaDesk Signup — Signup [sites/novadesk-signup]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(69).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/novadesk-signup.webp

Build a full-screen sign-up page as a single-page React + TypeScript + Tailwind CSS + Vite app. Use `lucide-react` icons only. No additional UI libraries.

**Background:**
- Full-viewport autoplaying, muted, looping, `playsInline` HTML5 `<video>` element covering the entire screen (`absolute inset-0 w-full h-full object-cover`).
- Video source URL (exact): `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_191911_e7dc783e-a580-4974-8971-9796ecffd3bd.mp4`
- Page root: `relative min-h-screen w-screen overflow-x-hidden flex items-center justify-center bg-black py-6 sm:py-0 sm:h-screen sm:overflow-hidden`.

**Layout:**
- Centered two-column card, `max-w-4xl mx-4 rounded-2xl overflow-hidden shadow-2xl`, fixed height `sm:h-[660px]`, stacked column on mobile, row `sm:flex-row` on desktop, z-index 10.

**Left column (form, 50% width on desktop, full width on mobile):**
- Background: `rgba(10, 10, 10, 0.92)` (dark opaque).
- Padding: `px-6 sm:px-10 py-8 sm:py-10`. Flex column.
- Top: Brand lockup — custom SVG logo (36px) next to the word "NovaDesk" in brand color `#DA3F23`, `font-semibold tracking-tight text-lg`, gap-2.
- Logo SVG (viewBox 0 0 256 256, fill `#DA3F23`), path:
  `M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 96 95 L 63.5 128 L 64 128 L 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 64 L 64 0 L 192 0 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z`
- Form group: `flex flex-col gap-5 mt-8 sm:mt-auto` (pushed to bottom on desktop).
- Heading: `h1` text "Sign up", `text-white text-2xl font-semibold tracking-tight`.
- Subtext: "Set up your profile and jump in right now.", `text-zinc-400 text-sm mt-1.5 leading-relaxed`.
- Email input: placeholder "Input Email", `w-full bg-zinc-800/70 text-white placeholder-zinc-500 text-sm rounded-lg px-4 py-2.5 focus:outline-none focus:ring-1 focus:ring-zinc-500 transition-colors`.
- Password input: placeholder "Choose Password", same styling plus `pr-11`; trailing eye/eye-off toggle button (`lucide-react` `Eye`/`EyeOff`, size 16, `text-zinc-500 hover:text-zinc-300 transition-colors`, absolute right-3, vertically centered).
- Custom checkbox + terms: hidden native input (`sr-only`), 16x16 rounded border square, unchecked `border-zinc-600 bg-transparent`, checked `bg-white border-white` with black SVG check mark. Label: "I Agree On The [Rules] & [Privacy Notice]" with underlined links `text-zinc-200 hover:text-white`. Text `text-zinc-400 text-xs leading-relaxed`.
- Submit button: "Launch Account", `w-full bg-white text-black text-sm font-semibold rounded-lg py-2.5 hover:bg-zinc-100 active:bg-zinc-200 transition-colors`.
- Divider: two `flex-1 h-px bg-zinc-700/60` lines with centered "or join us via" `text-zinc-500 text-xs`.
- Three social buttons in a row (equal flex): Google (`Chrome` icon), Apple (`Apple` icon), Twitter (`Twitter` icon), each `flex-1 bg-zinc-800/60 rounded-lg py-2 text-zinc-300 text-sm hover:bg-zinc-700/60 hover:text-white transition-colors`, icon size 15.
- Footer line: `text-zinc-500 text-xs text-center` — "Already Hold An Account? [Enter]" with `text-zinc-200 hover:text-white font-medium`.

**Right column (desktop only, 50% width):**
- `hidden sm:flex items-center justify-center`.
- Background: `rgba(255, 255, 255, 0.05)` with 1px `rgba(255,255,255,0.08)` border on all sides (glass panel over the video).
- Contains the logo SVG at size 34, offset upward with inline style `marginTop: -70px`.

**State (React `useState`):**
- `email`, `password` (strings), `agreed` (bool), `showPassword` (bool).

**Fonts:** default Tailwind system font stack (no custom font imports). Weights used: 500 (`font-medium`), 600 (`font-semibold`). Tracking `tracking-tight` on headings and brand.

**Animations:** all transitions via Tailwind `transition-colors` on hover/active states for inputs, buttons, links, and checkbox. No keyframe animations.

**Colors:**
- Brand: `#DA3F23`
- Surfaces: `bg-black`, `rgba(10,10,10,0.92)`, `bg-zinc-800/70`, `bg-zinc-800/60`, `bg-zinc-700/60`
- Text: white, `zinc-200`, `zinc-300`, `zinc-400`, `zinc-500`
- Borders: `zinc-600`, `rgba(255,255,255,0.08)`

## Feedback Slider — Slider [sites/feedback-slider]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(38).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/feedback-slider.webp

Build a React + TypeScript + Tailwind CSS + Vite project with a single section called "What builders say". Use `lucide-react` for icons. Reproduce it exactly as specified below.

### Fonts (load globally in `src/index.css`, before `@tailwind` directives)

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

Place `PPMondwest-Regular.woff2` in the `public/` folder.

### In-view hook (`src/hooks/useInViewAnimation.ts`)

`IntersectionObserver` with `threshold = 0.1`, returns `{ ref, isInView }`. Once `isInView` becomes true, it stays true.

### Component (`src/components/TestimonialCarousel.tsx`)

**Data** — Array of 5 testimonials with `id`, `quote`, `author`, `role`, `company`, `avatar`. Use Pexels stock photos for avatars (resized w=200, h=200, dpr=2):

1. Marcus Anderson — CEO, Data.storage — "With very little guidance team delivered designs that were consistently spot on. We've received so much positive feedback about the design, our community loves it." — `https://images.pexels.com/photos/774909/pexels-photo-774909.jpeg?auto=compress&cs=tinysrgb&w=200&h=200&dpr=2`
2. alexwu — Founder, Nexgate — "Viktor led the creation of our best fundraising deck to date! Knows how to merge sophisticated UX with simple cryptonative design" — `https://images.pexels.com/photos/220453/pexels-photo-220453.jpeg?auto=compress&cs=tinysrgb&w=200&h=200&dpr=2`
3. James Mitchell — VP Product, LaunchPad — "Working with Viktor transformed our product vision into something truly exceptional. The attention to detail and strategic thinking was outstanding." — `https://images.pexels.com/photos/415829/pexels-photo-415829.jpeg?auto=compress&cs=tinysrgb&w=200&h=200&dpr=2`
4. Rachel Foster — Co-founder, Nexus Labs — "The design quality exceeded our expectations. Viktor brought a level of polish and professionalism that elevated our entire brand." — `https://images.pexels.com/photos/1681010/pexels-photo-1681010.jpeg?auto=compress&cs=tinysrgb&w=200&h=200&dpr=2`
5. David Zhang — Head of Design, Paradigm Labs — "Incredible work from start to finish. The team's ability to understand our vision and execute flawlessly was remarkable." — `https://images.pexels.com/photos/733872/pexels-photo-733872.jpeg?auto=compress&cs=tinysrgb&w=200&h=200&dpr=2`

**State / sizing**
- `offset` (px, starts at 0), `isPaused` (false).
- `isMobile = window.innerWidth < 768`.
- `cardWidth = isMobile ? window.innerWidth - 48 : 427.5`.
- `gap = 24`. `cardWithGap = cardWidth + gap`.
- Render `[...testimonials, ...testimonials, ...testimonials]` (tripled) to give the illusion of an infinite loop.

**Auto-advance**
- `setInterval` every 3000ms: `offset += cardWithGap`. When `offset >= cardWithGap * testimonials.length`, reset to 0.
- Pause on `onMouseEnter`, resume on `onMouseLeave` of the carousel wrapper.

**Prev/Next buttons** decrement/increment `offset` by `cardWithGap` with the same wrap logic.

**Section layout**
- `<section className="w-full py-20 bg-white">` wrapper with `ref` from the in-view hook.
- Inner container: `max-w-7xl mx-auto px-6`, then a `w-full md:pr-6` wrapper.
- Header row: `flex flex-col md:flex-row md:items-start md:justify-between mb-16 md:max-w-4xl md:ml-auto gap-6 md:gap-0`.
  - Left (flex-1): heading `text-[32px] md:text-[40px] lg:text-[44px] leading-[1.1] text-[#0D212C] tracking-tight font-normal`. Text: `What ` then a `<span>` with inline style `fontFamily: "'PP Mondwest', serif"` reading `builders`, then ` say`.
  - Right column: `flex flex-col items-start md:items-end gap-2`.
    - Row of 5 `Star` icons from lucide-react, `w-5 h-5 fill-black text-black`.
    - Row: `Clutch` in `text-xl font-semibold text-[#0D212C]` and `5/5` in `text-base text-[#273C46]`.
- Apply fade-in animations: heading delay `0.1s`, Clutch block delay `0.2s`, carousel delay `0.3s`, button row delay `0.4s`. Each element uses `isInView ? 'animate-fade-in-up' : 'opacity-0'` plus `style={{ animationDelay: isInView ? 'Xs' : '0s' }}`.

**Carousel container**
```
relative overflow-hidden md:max-w-4xl md:ml-auto py-6 md:pl-6 -mx-6 md:mx-0
```
Inner track: `flex gap-6 pl-6 md:pl-0` with inline style `transform: translateX(-${offset}px)` and `transition: 'transform 0.8s cubic-bezier(0.4, 0, 0.2, 1)'`.

**Card**
- `bg-white rounded-[32px] md:rounded-[40px] px-6 md:pl-10 md:pr-24 py-8 md:pt-[2.36rem] md:pb-[2.63rem] flex flex-col justify-between flex-shrink-0 shadow-[0_4px_16px_rgba(0,0,0,0.08)]`
- Width set via inline style to `cardWidth` px.
- Per-card exit animation: compute `distanceFromEdge = offset % (cardWithGap * testimonials.length)`, `cardPosition = index * cardWithGap`, `relativePosition = cardPosition - distanceFromEdge`. If `relativePosition < -cardWidth / 2`, compute `exitProgress = min(1, abs(relativePosition) / cardWidth)`, set `opacity = max(0, 1 - exitProgress * 2)` and `scale = max(0.85, 1 - exitProgress * 0.15)`. Apply via inline style with `transition: 'opacity 0.4s ease-out, transform 0.4s ease-out'`.
- Card content top: a quote glyph rendered as inline SVG, `className="w-8 h-8 text-[#0D212C]"`, `fill="currentColor"`, `viewBox="0 0 24 24"`, path `d="M14.017 21v-7.391c0-5.704 3.731-9.57 8.983-10.609l.995 2.151c-2.432.917-3.995 3.638-3.995 5.849h4v10h-9.983zm-14.017 0v-7.391c0-5.704 3.748-9.57 9-10.609l.996 2.151c-2.433.917-3.996 3.638-3.996 5.849h3.983v10h-9.983z"`. Margin `mb-6`.
- Quote `<p>`: `text-base text-[#0D212C] leading-relaxed mb-8`.
- Author row: `flex items-center gap-4`, avatar `<img>` `w-12 h-12 rounded-full object-cover`, then a column with author name `font-semibold text-[#0D212C] text-sm` and a sub-row `text-sm text-[#273C46] flex items-center gap-1` containing a `↳` glyph in `text-xs` and `{role}, {company}`.

**Nav buttons row**
- Container: `flex gap-4 mt-8 md:max-w-4xl md:ml-auto md:pl-6`.
- Each button: `w-12 h-12 rounded-full border border-[#0D212C]/20 flex items-center justify-center hover:bg-[#0D212C]/5 transition-colors`. Icons `ChevronLeft` / `ChevronRight` from lucide-react, `w-5 h-5 text-[#0D212C]`. Add `aria-label`.

### Colors used
- Text dark: `#0D212C`
- Text muted: `#273C46`
- Card shadow: `0 4px 16px rgba(0,0,0,0.08)`
- Background: white

### Required dependencies
`react`, `react-dom`, `lucide-react`, plus Vite + Tailwind toolchain.

---

## Media Card Carousel — Slider [sites/media-card-carousel]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(45).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/media-card-carousel.webp

---

Build a **Video Stories section** for an aerospace company called "EngineTech." This is a light-background section with a centered header and a horizontally-scrolling rail of video story cards with scroll-snap, edge bleed, and hover/focus opacity transitions.

---

### SECTION CONTAINER (`.video-stories`)

- Position relative, z-index 90, min-height 100vh.
- Padding: `clamp(46px, 5vw, 88px) 0 clamp(44px, 4vw, 74px)`.
- Overflow hidden.
- Background: `#f7f8f8`. Color: `#111111`.

---

### HEADER (`.video-stories__header`)

- Width: `min(100% - 96px, 900px)`. Centered with `margin: 0 auto clamp(38px, 4vw, 74px)`.

**H2:**

- Text: "Program stories from the people building flight-ready power."
- Margin 0, color `#111111`, `font-size: clamp(38px, 4.4vw, 76px)`, weight 300, letter-spacing 0, line-height 1.08.

**P:**

- Text: "Short field notes from integration leads, test engineers, and manufacturing teams moving advanced propulsion systems from requirement reviews to repeatable flight hardware."
- Max-width 720px, margin `22px 0 0`, color `#697272`, `font-size: clamp(16px, 1.25vw, 21px)`, weight 420, line-height 1.55.

---

### RAIL (`.video-stories__rail`)

- CSS grid with horizontal auto-flow: `grid-auto-flow: column`, `grid-auto-columns: minmax(520px, 34vw)`.
- Gap: `clamp(28px, 3vw, 54px)`.
- `overflow-x: auto`. `overscroll-behavior-x: contain`. `scroll-snap-type: x mandatory`.
- Padding: `0 max(48px, calc((100vw - var(--hero-max-width)) / 2 + 48px)) 36px` (so first/last card aligns with content max-width on wide screens, with 48px minimum edge gutter).
- Hide scrollbar: `scrollbar-width: none` and `::-webkit-scrollbar { display: none }`.
- Has `aria-label="EngineTech video previews"`.

---

### STORY CARD (`.story-card`)

- `scroll-snap-align: center`. `min-width: 0`.
- Default state: `opacity: 0.54; transform: translateY(10px)`.
- Hover/focus state: `opacity: 1; transform: none`.
- Transition: `opacity 260ms ease, transform 260ms ease`.

**Media (`.story-card__media`):**

- A `<video>` element with `autoplay muted loop playsinline`.
- Display block, width 100%, height auto, `aspect-ratio: 16 / 9`.
- Border-radius 12px. Background `#dfe5e6`. `object-fit: cover`, `object-position: center`.
- Box-shadow: `0 18px 48px rgb(21 34 34 / 0.1)`.

**Content (`.story-card__content`):**

- Padding `24px 28px 0`.
- `<p>` (category tag): margin `0 0 12px`, color `#111111`, font-size 15px, weight 760, line-height 1.
- `<h3>` (title): max-width 680px, margin 0, color `#252b2b`, `font-size: clamp(18px, 1.22vw, 24px)`, weight 520, letter-spacing 0, line-height 1.38.
- `<span>` (meta): display block, margin-top 14px, color `#858d8d`, font-size 14px, line-height 1.4.

---

### THE 5 CARDS (in order)

**Card 1:**
- Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032431_5e054107-51c0-4162-9f0f-3a40054761ef.mp4`
- Category: "Integration Review"
- Title: "How a reusable upper-stage program moved from thermal risk to stable qualification."
- Meta: "Reusable systems · 04:20"

**Card 2:**
- Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032535_4ccc152e-0cc8-4ee5-a698-e1a98cea8a1e.mp4`
- Category: "Hot-Fire Campaign"
- Title: "Inside the test cell where telemetry, vibration, and injector response converge."
- Meta: "Validation · 03:45"

**Card 3:**
- Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_033707_b842a2ea-f223-4804-96d0-737ab67510fc.mp4`
- Category: "Manufacturing Floor"
- Title: "Why sub-micron inspection changes the way aerospace teams plan reliability."
- Meta: "Precision build · 05:10"

**Card 4:**
- Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032431_5e054107-51c0-4162-9f0f-3a40054761ef.mp4` (same as card 1)
- Category: "Hydrogen Pathway"
- Title: "Designing feed systems and ignition envelopes for hydrogen-ready propulsion."
- Meta: "H2 systems · 04:55"

**Card 5:**
- Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032535_4ccc152e-0cc8-4ee5-a698-e1a98cea8a1e.mp4` (same as card 2)
- Category: "Mission Support"
- Title: "The operational cadence behind launch-window support and post-test analysis."
- Meta: "Field readiness · 03:30"

---

### FOOTER PROGRESS INDICATOR (`.video-stories__footer`)

- Display flex, `align-items: center`, gap 8px.
- Width: `min(100% - 96px, 900px)`. Margin: `28px auto 0`.
- Has `aria-hidden="true"`.

Contents (in order):

1. First `<span>`: 56px wide × 4px tall, border-radius 999px, background `#cfd4d4`.
2. Second `<span>`: same as first.
3. Third `<span>`: **320px wide** × 4px tall, border-radius 999px, background `#111111` (active progress).
4. `<strong>`: text "05 / 05", margin-left 18px, color `#7a8282`, font-size 14px, weight 650, `letter-spacing: 0.02em`.

---

### RESPONSIVE BREAKPOINTS

**At 860px:**

- `.video-stories__header` and `.video-stories__footer` width: `min(100% - 48px, 900px)`.
- Rail: `grid-auto-columns: minmax(320px, 82vw)`. Padding: `0 24px 30px`.
- Story cards always at full opacity, no transform offset (`opacity: 1; transform: none`).

**At 560px:**

- Header and footer width: `min(100% - 32px, 900px)`.
- Card content padding becomes `18px 4px 0`.
- Active footer bar (third span) width shrinks to 150px.

---

### GLOBAL STYLES

**CSS custom property used:** `--hero-max-width: 1820px`.

**Font stack:** `"Geist", "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` with `-webkit-font-smoothing: antialiased` and `text-rendering: geometricPrecision`.

**Color palette:** No purple or violet. Light background `#f7f8f8`. Dark text `#111111`, `#252b2b`. Muted neutrals `#697272`, `#858d8d`, `#7a8282`. Pale divider `#cfd4d4`. Video placeholder bg `#dfe5e6`.

## Digital Reality — Social Media [sites/digital-reality-hero]

- Preview: https://motionsites.ai/assets/hero-digital-reality-preview-BogjTXUi.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/digital-reality-hero.gif

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

## Social Media Posts — Social Media [sites/social-media-posts-hero]

- Preview: https://motionsites.ai/assets/hero-social-media-posts-preview-0OltSGuj.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/social-media-posts-hero.gif

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

## Velorah Focus — Social Media [sites/velorah-focus-hero]

- Preview: https://motionsites.ai/assets/hero-velorah-focus-preview-Boo7l3W4.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/velorah-focus-hero.gif

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

## Arceage Stats — Stats [sites/arceage-stats]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(59).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/arceage-stats.webp

Create a React + Tailwind CSS v4 + Motion (framer-motion successor) stats section component. Use Vite as the bundler. The section should be fully mobile responsive.

### Fonts

Import from Google Fonts in your global CSS:
```
@import url('https://fonts.googleapis.com/css2?family=Barlow:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&family=Instrument+Serif:ital@0;1&display=swap');
```

Define two Tailwind v4 theme fonts:
- `--font-sans: "Barlow", ui-sans-serif, system-ui, sans-serif;` (used as the primary UI font via `font-sans`)
- `--font-dm-serif: "Instrument Serif", serif;` (used as the accent/poetic font via `font-dm-serif`)

The page wrapper uses `bg-black font-sans text-white`.

### Dependencies

- `react` v19
- `motion` (npm package "motion", imported as `motion/react` -- provides `motion`, `useInView`, `animate`)
- `tailwindcss` v4 with `@tailwindcss/vite` plugin
- Vite v6+

### Section Layout

The section is a `<section>` with:
- `id="stats"`
- Classes: `bg-black text-white py-8 md:py-24 px-6 md:px-12 lg:px-[120px] w-full border-t border-white/10 overflow-hidden`
- Inner wrapper: `w-full max-w-[1440px] mx-auto`
- Content is a two-column flexbox: `flex flex-col lg:flex-row gap-16 lg:gap-[160px] items-stretch`

### Left Column (flex-1, flex flex-col justify-start)

The entire left column is wrapped in a `motion.div` with staggered reveal animation:
- `initial="hidden"`, `whileInView="visible"`, `viewport={{ once: true, margin: "-100px" }}`
- Variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1, transition: { staggerChildren: 0.06 } }`

**Heading (h2):**
- Classes: `text-[clamp(1.5rem,4vw,3.5rem)] font-medium tracking-tight mb-6 leading-[1.1] w-[590px] max-w-full`
- Content uses a custom `<Typewriter>` component (described below):
  - `<Typewriter text="Powering Harvests" delay={0} speed={0.012} />` followed by `<br />`
  - `<Typewriter text="that " delay={0.25} speed={0.012} />` then a `<span className="font-dm-serif italic font-normal">` wrapping `<Typewriter text="Maximize Your Yield" delay={0.35} speed={0.012} />`
- The phrase "Maximize Your Yield" renders in Instrument Serif italic as the accent font.

**Subtitle (p):**
- Classes: `text-base md:text-lg text-white/40 leading-relaxed font-light max-w-lg whitespace-normal mb-16`
- Content: `<Typewriter text="For over a decade, the region's most demanding agricultural operations have relied on our modern machinery and skilled crews to secure their crops efficiently and reduce loss." delay={0.1} speed={0.012} />`

**Stats Grid:**
- Wrapped in `motion.div` with stagger variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1, transition: { staggerChildren: 0.06, delayChildren: 0.1 } }`
- Classes: `grid grid-cols-2 md:grid-cols-[max-content_max-content] gap-8 md:gap-x-16 lg:gap-x-24`
- 5 stat items, each wrapped in `motion.div` with variants: hidden = `{ opacity: 0, y: 20 }`, visible = `{ opacity: 1, y: 0, transition: { duration: 0.4, ease: "easeOut" } }`
- Each stat item (`flex flex-col`):
  - **Number:** `text-4xl md:text-5xl lg:text-[56px] font-dm-serif tracking-tight mb-3` (uses Instrument Serif)
  - **Label:** `text-[10px] md:text-xs font-semibold text-white/40 uppercase tracking-wider`

The 5 stats with their AnimatedCounter props:
1. `value={500} suffix="K+"` / Label: "Acres Harvested Annually"
2. `value={99.8} decimals={1} suffix="%"` / Label: "Crop Recovery Rate"
3. `value={50} suffix="+"` / Label: "Modern Combines Deployed"
4. `value={15} suffix="+"` / Label: "Crop Varieties Supported"
5. `value={24} suffix="/7"` / Label: "Uptime During Season"

### AnimatedCounter Component

A helper component that animates from 0 to a target value on scroll into view:
- Props: `value: number`, `suffix?: string` (default ""), `prefix?: string` (default ""), `decimals?: number` (default 0)
- Uses `useRef<HTMLSpanElement>`, `useInView(ref, { once: true, margin: "-50px" })` from `motion/react`
- On `inView`, calls `animate(0, value, { duration: 1.5, ease: "easeOut", onUpdate(val) { ref.current.textContent = prefix + val.toFixed(decimals) + suffix } })`
- Returns `<span ref={ref}>{prefix}0{suffix}</span>` as initial render

### Typewriter Component

A reusable character-by-character reveal animation triggered on scroll:
- Props: `text: string`, `delay?: number` (default 0), `speed?: number` (default 0.015), `className?: string` (default "")
- Uses `useRef`, `useInView(ref, { once: true, margin: "-10px" })` from `motion/react`
- Renders a `motion.span` with `initial="hidden"` and `animate={inView ? "visible" : "hidden"}`
- Parent variants: hidden = `{ opacity: 1 }`, visible = `{ opacity: 1, transition: { staggerChildren: speed, delayChildren: delay } }`
- Splits text into individual characters, each wrapped in `motion.span` with variants: hidden = `{ opacity: 0 }`, visible = `{ opacity: 1 }`

### Right Column: Logo-Masked Video

- Wrapper: `flex justify-center lg:justify-end items-center shrink-0 lg:w-1/2`
- Inner `motion.div`:
  - `initial={{ opacity: 0, scale: 0.9 }}`
  - `whileInView={{ opacity: 1, scale: 1.2 }}`
  - `viewport={{ once: true, margin: "-100px" }}`
  - `transition={{ duration: 0.8, delay: 0, ease: "easeOut" }}`
  - Classes: `w-full max-w-[500px] lg:max-w-none lg:w-[120%] aspect-square origin-center`
  - Uses CSS `mask-image` (both `-webkit-mask-image` and `mask-image`) with an inline SVG data URI of a triangular/mountain-like logo shape. The exact SVG path data:
    ```
    m53.54,45.42c2.19-3.79,7.67-3.79,9.86,0l4.54,7.87c1.17,2.02,1.17,4.51,0,6.54l-8.15,13.81c-1.68,2.91.42,6.55,3.78,6.55h17.81c3.45,0,5.61-3.74,3.89-6.73l-28.76-49.81c-2.95-5.12-10.34-5.12-13.29,0l-28.46,49.3c-1.86,3.22.46,7.24,4.18,7.24h10.23c2.55,0,4.91-1.36,6.19-3.57l18.18-31.19Z
    ```
  - SVG viewBox: `0 0 100 100`
  - Mask properties: `maskSize: 'contain'`, `maskRepeat: 'no-repeat'`, `maskPosition: 'center'`
  - Full inline style object:
    ```js
    {
      WebkitMaskImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Cpath d='m53.54,45.42c2.19-3.79,7.67-3.79,9.86,0l4.54,7.87c1.17,2.02,1.17,4.51,0,6.54l-8.15,13.81c-1.68,2.91.42,6.55,3.78,6.55h17.81c3.45,0,5.61-3.74,3.89-6.73l-28.76-49.81c-2.95-5.12-10.34-5.12-13.29,0l-28.46,49.3c-1.86,3.22.46,7.24,4.18,7.24h10.23c2.55,0,4.91-1.36,6.19-3.57l18.18-31.19Z'/%3E%3C/svg%3E")`,
      WebkitMaskSize: 'contain',
      WebkitMaskRepeat: 'no-repeat',
      WebkitMaskPosition: 'center',
      maskImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Cpath d='m53.54,45.42c2.19-3.79,7.67-3.79,9.86,0l4.54,7.87c1.17,2.02,1.17,4.51,0,6.54l-8.15,13.81c-1.68,2.91.42,6.55,3.78,6.55h17.81c3.45,0,5.61-3.74,3.89-6.73l-28.76-49.81c-2.95-5.12-10.34-5.12-13.29,0l-28.46,49.3c-1.86,3.22.46,7.24,4.18,7.24h10.23c2.55,0,4.91-1.36,6.19-3.57l18.18-31.19Z'/%3E%3C/svg%3E")`,
      maskSize: 'contain',
      maskRepeat: 'no-repeat',
      maskPosition: 'center',
    }
    ```

- Inside the masked div, a `<video>` element:
  - Attributes: `autoPlay`, `loop`, `muted`, `playsInline`
  - Classes: `w-full h-full object-cover`
  - Source: `https://app-uploads.krea.ai/wan-videos/7f348c17-c3aa-40c9-9d5b-a2bed9a72c2e.mp4` (type `video/mp4`)

### Mobile Responsiveness Summary

- Section padding: `py-8 px-6` on mobile, `md:py-24 md:px-12`, `lg:px-[120px]`
- Layout stacks vertically on mobile (`flex-col`), goes side-by-side at `lg:` (`flex-row`)
- Heading uses fluid typography: `clamp(1.5rem, 4vw, 3.5rem)`
- Stats grid: 2 columns on mobile (`grid-cols-2`), auto-sized on `md:` (`grid-cols-[max-content_max-content]`)
- Stat numbers: `text-4xl` on mobile, `md:text-5xl`, `lg:text-[56px]`
- Video mask: `max-w-[500px]` on mobile, full width at `lg:` with `lg:w-[120%]`

---

## Glassmorphic Feature Tabs — Tabs [sites/glassmorphic-feature-tabs]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(30).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/glassmorphic-feature-tabs.webp

### Core Features Section

Build a single React + TypeScript + Tailwind + framer-motion section called `CoreFeaturesSection`. Match every detail below exactly.

### Stack & Global Setup

- React 18, Vite, TypeScript, TailwindCSS, `framer-motion`, `clsx` + `tailwind-merge` (helper `cn`).
- Dark theme. Page background `#000000` (set on `body`).
- Font: **Inter** (Google Fonts, weights 300–700) as default sans.
- Icons: **Material Symbols Rounded** via Google Fonts link:
  `https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200`
- Tailwind tokens (HSL): `--background: 270 80% 3%;` `--foreground: 0 0% 100%;` mapped to `background` / `foreground`.
- Extra Tailwind color: `landing.surface: rgba(255,255,255,0.10)`.

### Helper: `cn`
```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
export const cn = (...i: ClassValue[]) => twMerge(clsx(i));
```

### Material Icon component (`MIcon`)
```tsx
export const MIcon = ({ name, size = 16, className = "" }:{name:string;size?:number;className?:string}) => (
  <span
    className={`material-symbols-rounded ${className}`}
    style={{ fontSize: size, fontVariationSettings: `"FILL" 0, "wght" 400, "GRAD" 0, "opsz" ${size}` }}
  >{name}</span>
);
```

### `FadeUp` primitive (scroll reveal)
- `framer-motion` `motion.div`. `initial={{opacity:0,y:24}}`, `whileInView={{opacity:1,y:0}}`, `viewport={{once:true,amount:0.3}}`, `transition={{duration:0.6,delay,ease:[0.22,1,0.36,1]}}`. Respect `useReducedMotion` (no y offset).

### `SpotlightBorder` (mouse-tracked 1px gradient border)
- Wrapper with `position:relative` + chosen radius (`rounded-xl|2xl|3xl|full`).
- Listens to `window` `mousemove`; sets CSS vars `--spot-x`, `--spot-y` from `getBoundingClientRect`.
- Absolutely positioned `<span>` overlay with:
```ts
{
  background: `radial-gradient(${size}px circle at var(--spot-x,-200px) var(--spot-y,-200px), rgba(255,255,255,${intensity}), rgba(255,255,255,0) 60%)`,
  padding: "1px",
  WebkitMask: "linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)",
  WebkitMaskComposite: "xor",
  maskComposite: "exclude",
}
```
- Props: `radius`, `size` (px), `intensity` (0–1). Used three times in this section with sizes 360/600/360 and intensity 0.5.

### Tab data (exact URLs, order matters)
```ts
const tabs = [
  { label: "Exclusive Tutorial",
    image: "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260521_222821_06fd2e74-16a5-4e7f-90ed-14e6760e7edb.png&w=1280&q=85",
    caption: "Step-by-step guides to master AI design tools." },
  { label: "Courses",
    image: "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260521_222901_b133c5f0-191c-4285-a018-a68fd9c9f5ac.png&w=1280&q=85",
    caption: "Structured learning paths to level up your skills." },
  { label: "Templates",
    image: "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260521_225713_3226e3ad-3364-42b1-99bd-ed82005c0524.png&w=1280&q=85",
    caption: "Production-ready designs you can customize instantly." },
  { label: "Animated Backgrounds",
    image: "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260521_222832_223716d0-9b6c-4c48-98a6-a5e3c02e2962.png&w=1280&q=85",
    caption: "Motion-ready visuals that bring your projects to life." },
];
```

### Behavior
- `active` state, default `0`. `pausedRef = useRef(false)`.
- `setInterval` every **5000ms**: if not paused, `active = (active+1) % 4`.
- Arrow buttons & tab clicks set `pausedRef.current = true` (auto-rotation stops after first user interaction).

### Section Layout

```tsx
<section className="relative w-full bg-background py-12 sm:py-16">
  <div className="mx-auto max-w-[1180px] px-4 sm:px-6">
```

### Header row (flex, stacks on mobile)
- Left column `max-w-2xl`:
  - Pill (FadeUp d=0): `inline-flex items-center gap-2 rounded-full bg-landing-surface border border-white/10 px-3 py-1 text-xs text-foreground/80 backdrop-blur` with `1.5x1.5` dot `bg-foreground/70` + text "Core Features".
  - Heading (FadeUp d=0.1): `text-3xl sm:text-4xl font-normal tracking-[-0.02em] leading-[1.05] text-foreground`. Content: `One platform to run your<br className="hidden sm:block"/><span className="text-foreground/55"> entire AI design journey.</span>`
- Right (FadeUp d=0.2, `max-w-sm md:pt-2`): paragraph `text-sm sm:text-base leading-relaxed text-foreground/65`: "UI Rocket brings your lessons, templates, tools, and community into one space — so you stop switching between tabs and start shipping real AI-powered work."
- Wrapper: `mb-14 flex flex-col gap-10 md:flex-row md:items-end md:justify-between`.

### Tab pills (desktop only)
`SpotlightBorder radius="full" size={360} intensity={0.5}` `mx-auto mb-6 hidden w-full p-1 sm:block`.
Inside: `grid grid-cols-2 sm:grid-cols-4 gap-1 rounded-full p-1`.
Each button: `rounded-full px-4 py-2.5 text-sm transition-colors duration-300`. Active: `bg-white/[0.06] text-foreground border border-white/15`. Inactive: `text-foreground/55 hover:text-foreground/80 border border-transparent`. Click → `select(i)`.

### Image stage
`SpotlightBorder radius="2xl" size={600} intensity={0.5}` `relative mx-auto w-full p-2 sm:p-3`.
Inner: `relative overflow-hidden rounded-2xl border border-white/10` with inline `style={{ backgroundColor:"#0e0e0e" }}`.
Aspect frame: `relative aspect-[16/10] w-full`.

Render all 4 `<img>` absolutely stacked (`absolute inset-0 h-full w-full object-cover transition-opacity duration-400`). Active one `opacity-100`, others `opacity-0`. Use `loading="eager" decoding="async"`.

Overlay all 4 `TabDashboardMock` panels (absolute, fade). Wrapper per mock:
`absolute inset-1 flex items-center justify-center p-[3%] sm:p-[4%] transition-opacity duration-300` + active `opacity-100` else `opacity-0 pointer-events-none`.

Mock list (label → title, activeNav):
- Courses → "Courses" / "Courses"
- Templates → "Templates" / "Templates"
- Animated Backgrounds → "Animated Backgrounds" / "Backgrounds"
- Exclusive Tutorial → "Exclusive Tutorials" / "Tutorials"

### Arrow / caption bar
`SpotlightBorder radius="full" size={360} intensity={0.5}` `mx-auto mt-6 w-full p-1`.
Inside: `flex items-center justify-between gap-4 rounded-full px-3 py-2`.
- Left/right buttons: `flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/[0.04] text-foreground/80 transition-colors hover:bg-white/[0.08] hover:text-foreground`. Icons: `arrow_back` / `arrow_forward` size 16. Click → `go(-1)` / `go(1)`.
- Center caption box: `min-h-[1.5rem] flex-1 overflow-hidden text-center`. Use `AnimatePresence mode="wait"` + `motion.p` keyed by `tabs[active].label`, initial `{opacity:0,y:6}`, animate `{opacity:1,y:0}`, exit `{opacity:0,y:-6}`, `duration 0.25`. Classes `px-2 text-sm text-foreground/75`. Mobile shows label (`sm:hidden font-medium text-foreground`), desktop shows `tabs[active].caption` (`hidden sm:inline`).

### `TabDashboardMock` (auto-scaled dashboard preview)

Fixed design canvas **900 × 562** (16:10). Outer wrapper measures itself with `ResizeObserver` and sets `transform: scale(min(w/900, h/562))`, `transformOrigin: center center`, `flexShrink: 0`.

Wrapper: `relative h-full w-full overflow-hidden flex items-center justify-center`.
Inner card (the 900×562 box): `flex overflow-hidden rounded-2xl bg-white/[0.04] backdrop-blur-xl shadow-2xl`.

### Sidebar (`w-[210px]`, `flex flex-col gap-1 p-3`)
- Brand row: `mb-3 flex items-center justify-between px-2 py-2`.
  - Left: icon box `flex h-6 w-6 items-center justify-center rounded-md` with inline gradient `linear-gradient(135deg, rgb(158,103,250), rgb(254,106,187) 50%, rgb(255,156,101))`, `MIcon "rocket_launch" size={14} className="text-white"`, label `text-[13px] font-semibold text-white` = "UI Rocket".
  - Right: `MIcon "search" size={14} className="text-white/40"`.
- Nav list (`flex flex-col gap-0.5`). Items:
  Dashboard/`grid_view`, Courses/`school`, Templates/`dashboard`, Tutorials/`play_circle`, Backgrounds/`auto_awesome`, Pricing/`sell`.
  Item base: `flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-[12px] transition-colors`. Active: `bg-white/[0.08] text-white`. Inactive: `text-white/55 hover:text-white/80`. Icon size 14.

### Main column (`flex min-w-0 flex-1 flex-col p-3`)
Inside: `flex h-full w-full flex-col overflow-hidden rounded-2xl bg-black/20`.
Header: `flex items-center justify-between px-4 py-3`.
- Left group: blue square `flex h-7 w-7 items-center justify-center rounded-md bg-[rgb(59,130,246)] text-white` + `MIcon "add" size={16}`. Title `text-base font-semibold text-white` = `title` prop.
- Right group (`flex items-center gap-2.5`): `MIcon "notifications" size={16} text-white/60`, pill `rounded-full bg-[rgb(59,130,246)] px-2.5 py-1 text-[11px] font-medium text-white` = "Invite", avatar `h-7 w-7 rounded-full bg-cover bg-center ring-1 ring-white/10` with `backgroundImage: url(https://i.pravatar.cc/64?img=12)`.
Body: `flex-1 overflow-hidden px-4 pb-4`. Render `children` here.

### Tab content panels (children passed to mock)

Each panel fills the body with a small grid of cards in the same monochrome white-alpha style (`bg-white/[0.04] border border-white/10 rounded-xl`, headings `text-white text-[13px] font-medium`, body `text-white/60 text-[11px]`). Compose roughly:

- **CoursesTabContent**: 2×3 grid of course cards. Each card: 16:9 thumb using the module covers below; title; meta row "X lessons · Y min" in `text-white/50 text-[10px]`.
- **TemplatesTabContent**: 3×2 grid of template cards with browser-frame thumbnails (3 dots row) using the same module covers as placeholder; small "Premium" pill `bg-white/10 text-[10px] rounded-full px-2 py-0.5`.
- **BackgroundsTabContent**: 3×2 grid of looping `<video autoplay muted loop playsinline>` tiles. Sources: use the CloudFront MP4s in the assets list. Overlay play icon (see asset) bottom-right.
- **ExclusiveTutorialTabContent**: featured large card (left, 60%) + 3 stacked rows (right, 40%). Featured uses tutorial thumbnail with floating play icon, title, "12 min · Pro" meta. Each row: 64x40 thumb + 2-line title + meta.

Assets (use exact URLs):
- Play icon SVG:
  `https://miptxtnhvjrkpmnjgdhk.supabase.co/storage/v1/object/public/training-assets/landing/play_icon.svg`
- Module / template cover thumbnails (cycle through):
  - `https://miptxtnhvjrkpmnjgdhk.supabase.co/storage/v1/object/public/training-assets/landing/module-cover-1.png`
  - `.../module-cover-2.png`
  - `.../module-cover-3.png`
  - `.../module-cover-4.png`
  - `.../module-cover-5.png`
  - `.../module-cover-6.png`
- Tutorial thumbnails: reuse the four `images.higgs.ai` URLs from the tabs array.
- Background videos (CloudFront, use as `<video>` `src`):
  - `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260521_014404_bg1.mp4`
  - `.../hf_20260521_014404_bg2.mp4` … through `bg6.mp4`
  (If a specific URL 404s, fall back to the matching higgs.ai poster image as a static tile.)

### Putting it together
```tsx
<CoreFeaturesSection />
```
Mount inside a page with `bg-background text-foreground font-inter antialiased`. No other dependencies.

### Acceptance checklist
- [ ] 4 tabs auto-rotate every 5s until user interacts, then stop.
- [ ] SpotlightBorder shows a soft white radial highlight that follows the cursor on all three bordered shells.
- [ ] Image cross-fades over 400ms; caption cross-fades with 6px y motion (250ms).
- [ ] TabDashboardMock scales as a single unit to fit its container while preserving 900×562 internal layout.
- [ ] Sidebar active item highlighted matches the visible tab.
- [ ] Header, pill, paragraph use Inter, exact tracking and color tokens specified.
- [ ] All asset URLs above load directly (no local imports).

## Technical Specifications — Tabs [sites/technical-specifications]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(71).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/technical-specifications.webp

Build a Stats section for an aerospace company called "EngineTech." This is a dark-background section with a two-column header, a 4-tab switcher, and an animated horizontal bar chart with range indicators, spark traces, and staggered entrance animations. The tab switching re-renders the chart with new data and replays all animations.

---

SECTION CONTAINER (`.stats`)

- Position relative, z-index 80, min-height 100vh.
- Padding: `clamp(44px, 5vw, 86px) clamp(16px, 3.8vw, 72px) clamp(54px, 5vw, 90px)`.
- Color: `#f7f8f8`.
- Background (layered):
  - `radial-gradient(circle at 78% 18%, rgb(113 145 208 / 0.18), transparent 34%)`
  - `radial-gradient(circle at 18% 88%, rgb(170 184 213 / 0.11), transparent 28%)`
  - `linear-gradient(180deg, #111414 0%, #171a1a 100%)`

---

HEADER (`.stats__header`)

- CSS grid: `grid-template-columns: minmax(0, 1.08fr) minmax(320px, 0.72fr)`.
- Gap: `clamp(32px, 6vw, 120px)`. Max-width 1820px, centered. Margin-bottom: `clamp(34px, 4.5vw, 72px)`.

Left column (`.stats__title-wrap`):

- H2: "Unmatched propulsion data across every flight-critical layer."
  - Max-width 920px, margin 0, color `#f7f8f8`, `font-size: clamp(29px, 3.2vw, 54px)`, weight 300, letter-spacing 0, line-height 1.08.

Right column (`.stats__summary`):

- `align-self: start`, margin 0, color `rgb(247 248 248 / 0.8)`, `font-size: clamp(18px, 1.65vw, 28px)`, weight 360, line-height 1.34.
- Entrance animation: Starts at `opacity: 0; transform: translateY(14px)`. When class `.is-visible` is added: `opacity: 1; transform: none`. Transition: 420ms ease on both properties.
- Default text (for "Cities" tab): "Distributed aerospace infrastructure needs engines that can test, relight, and recover across dense launch corridors and remote operating bases."

---

TAB BAR (`.stats__tabs`)

- CSS grid: `repeat(4, minmax(0, 1fr))`. Gap 0. Max-width 1820px, centered.
- Bottom border: `1px solid rgb(255 255 255 / 0.14)`.

Each tab is a `` (`.statstab`):

- Min-height 58px, padding `0 20px 18px 0`, no border, transparent background.
- Color: `rgb(247 248 248 / 0.5)`. Font: inherit, `font-size: clamp(14px, 1.22vw, 22px)`, weight 430. Text-align left. Cursor pointer.
- Transition: `color 220ms ease`.
- Active state (`.is-active`): color `#ffffff`.

**Active underline (`::after` pseudo-element):**

- Absolute, `right: 16px; bottom: -1px; left: 0`. Height 4px.
- Background: `linear-gradient(90deg, #7191d0, #aab8d5)` (the primary blue to soft blue).
- Default: `transform: scaleX(0); transform-origin: left`.
- Active: `transform: scaleX(1)`. Transition: `360ms cubic-bezier(0.22, 1, 0.36, 1)`.

**The 4 tabs (with `data-stats-tab` attribute):**

| data-stats-tab | Label |
|---|---|
| cities | Cities & Infrastructure |
| materials | Materials & Manufacturing |
| fuels | Fuels & Upstream |
| hydrogen | H2 Hydrogen |

First tab ("cities") starts with `.is-active` and `aria-selected="true"`.

---

### CHART CONTAINER (`.statschart`)

- Position relative, max-width 1820px, `min-height: clamp(520px, 58vh, 680px)`.
- Margin: `clamp(28px, 3vw, 48px) auto 0`. Padding: `0 0 22px`. Overflow hidden.
- Border: `1px solid rgb(255 255 255 / 0.08)`. Border-radius 20px.
- Background-color: `rgb(255 255 255 / 0.025)`.
- **Vertical grid lines** via background-image: `repeating-linear-gradient(to right, transparent 0, transparent calc(10% - 1px), rgb(255 255 255 / 0.07) calc(10% - 1px), rgb(255 255 255 / 0.07) 10%)`.
- Box-shadow: `inset 0 1px 0 rgb(255 255 255 / 0.08), 0 24px 70px rgb(0 0 0 / 0.18)`.
- Has `aria-live="polite"` and `data-stats-chart` attribute.

### Chart head (`.statschart-head`)

- Flex row, `align-items: center`, `justify-content: space-between`, gap 24px.
- Padding: `clamp(18px, 2vw, 28px)`.
- Border-bottom: `1px solid rgb(255 255 255 / 0.08)`. Background: `rgb(255 255 255 / 0.025)`.
- Left ``: dataset title (e.g., "Cities & Infrastructure"), color `#ffffff`, `font-size: clamp(12px, 0.86vw, 14px)`, weight 760, `letter-spacing: 0.16em`, uppercase.
- Right ``: "Operating envelope", color `rgb(247 248 248 / 0.48)`, same font-size, weight 620, `letter-spacing: 0.12em`, uppercase.

### Bars area (`.statsbars`)

- Grid layout, gap `clamp(16px, 2vh, 26px)`, padding `clamp(26px, 3vw, 48px) clamp(24px, 2.4vw, 42px) 0`.

Each bar row (`.stats__bar-row`) is an `

`:

- Grid: `grid-template-columns: minmax(180px, 0.27fr) minmax(0, 1fr)`. Align-items center. Gap: `clamp(18px, 2vw, 34px)`.
- **Entrance:** Starts `opacity: 0; transform: translateY(18px)`.
- When `.statschart.is-ready` is present: plays `stats-row-in` animation -- `520ms cubic-bezier(0.22, 1, 0.36, 1) forwards`, delay `var(--bar-delay)` (set per row: 0ms, 90ms, 180ms, 270ms).
- CSS custom properties set per row: `--bar-value`, `--range-start`, `--range-width`, `--bar-delay`.

**Bar label (`.statsbar-label`):**

- ``: color `#ffffff`, `font-size: clamp(15px, 1.1vw, 19px)`, weight 680, line-height 1.2.
- ``: margin-top 5px, color `rgb(247 248 248 / 0.48)`, `font-size: clamp(12px, 0.86vw, 14px)`, line-height 1.35.

**Track (`.statstrack`):**

- Position relative, `height: clamp(48px, 5.4vh, 64px)`, overflow hidden, border-radius 0.
- Background: `rgb(255 255 255 / 0.055)`.
- Box-shadow: `inset 0 0 0 1px rgb(255 255 255 / 0.075), 0 12px 32px rgb(0 0 0 / 0.16)`.

**Inside the track, 4 layers:**

1. **Range indicator (`.statsrange`):** Absolute, `top: 9px; bottom: 9px; left: var(--range-start); width: var(--range-width)`. Border: `1px solid rgb(170 184 213 / 0.22)`. Background: `linear-gradient(90deg, rgb(113 145 208 / 0.05), rgb(170 184 213 / 0.14), rgb(113 145 208 / 0.05))`. Starts `opacity: 0; transform: scaleX(0.6); transform-origin: left`. Animates with `stats-range-in`: `620ms cubic-bezier(0.22, 1, 0.36, 1) forwards`, delay `var(--bar-delay) + 60ms`.

2. **Fill bar (`.statsbar`):** Position relative, z-index 1, `width: var(--bar-value)`, height 100%. Background: `linear-gradient(90deg, rgb(113 145 208 / 0.62) 0%, #8fb0ef 62%, #d6e3ff 100%)`. Box-shadow: `0 0 34px rgb(113 145 208 / 0.24)`. Starts `transform: scaleX(0); transform-origin: left`. Animates with `stats-fill`: `900ms cubic-bezier(0.22, 1, 0.36, 1) forwards`, delay `var(--bar-delay) + 110ms`.

3. **Value label (`.statsvalue`):** Absolute, z-index 3, `top: 50%; right: 18px; transform: translateY(-50%)`. Color `#ffffff`, `font-size: clamp(14px, 1vw, 18px)`, weight 740. Displays value + unit (e.g., "82%").

4. **Spark trace (`.statstrace`):** Absolute inset 0, z-index 2, pointer-events none. Contains 6 `` elements per row, each positioned at `--point-x` (percentage along the bar) and `--point-y` (alternating 34% and 62% vertically). Each spark:
   - 18px square (variants: 14px for `--1`, 11px for `--2`), border-radius 50%.
   - Background: `radial-gradient(circle, rgb(255 255 255 / 0.95) 0 8%, rgb(214 227 255 / 0.42) 9% 22%, transparent 58%)`.
   - `filter: blur(0.1px)`. Starts `opacity: 0; transform: translate(-50%, -50%) scale(0.2)`.
   - `::before`: Horizontal cross-hair line -- 24px x 1px, centered, `background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.72), transparent)`. Rotated by `var(--spark-rotate)`.
   - `::after`: Vertical cross-hair -- 1px x 18px, centered, `background: linear-gradient(180deg, transparent, rgb(170 184 213 / 0.62), transparent)`. Same rotation.
   - Spark variant rotations: `--1` = 22deg, `--2` = -18deg, default = 0deg.
   - Animates with `stats-point-in`: `420ms cubic-bezier(0.22, 1, 0.36, 1) forwards` to `opacity: 0.86; transform: translate(-50%, -50%) scale(1)`. Delay: `var(--bar-delay) + 260ms + var(--point-delay)` (point-delay increments 70ms per point).

### Axis (`.statsaxis`)

- Below the bars. Grid: `grid-template-columns: minmax(180px, 0.27fr) minmax(0, 1fr)`. Gap `clamp(18px, 2vw, 34px)`. Padding `14px clamp(24px, 2.4vw, 42px) 0`. Color `rgb(247 248 248 / 0.42)`, `font-size: clamp(11px, 0.84vw, 14px)`.
- Left cell: empty ``.
- Right cell: a div with `grid-template-columns: repeat(11, minmax(0, 1fr))` containing 11 `` elements: "0", "10", "20"... "100". First aligned left, last aligned right.

---

### KEYFRAME ANIMATIONS

```
@keyframes stats-row-in {
  to { opacity: 1; transform: none; }
}

@keyframes stats-fill {
  to { transform: scaleX(1); }
}

@keyframes stats-range-in {
  to { opacity: 1; transform: scaleX(1); }
}

@keyframes stats-point-in {
  to { opacity: 0.86; transform: translate(-50%, -50%) scale(1); }
}
```

---

### JS BEHAVIOR (Tab switching + animation replay)

**Data structure:** 4 datasets keyed as `cities`, `materials`, `fuels`, `hydrogen`. Each has `title`, `summary`, and `bars` array (4 items). Each bar: `{ label, value, target, rangeStart, rangeEnd, unit, note, trace }` where `trace` is an array of 6 numbers (x-positions for spark points).

**Full dataset:**

```
cities: {
  title: "Cities & Infrastructure",
  summary: "Distributed aerospace infrastructure needs engines that can test, relight, and recover across dense launch corridors and remote operating bases.",
  bars: [
    { label: "Mobile integration bays", value: 82, target: 88, rangeStart: 58, rangeEnd: 91, unit: "%", note: "deployment coverage", trace: [28, 42, 57, 63, 74, 82] },
    { label: "Airport-adjacent service cells", value: 68, target: 74, rangeStart: 44, rangeEnd: 79, unit: "%", note: "qualified workflows", trace: [18, 36, 41, 55, 61, 68] },
    { label: "Remote launch support", value: 54, target: 63, rangeStart: 30, rangeEnd: 70, unit: "%", note: "field readiness", trace: [14, 24, 39, 43, 48, 54] },
    { label: "Thermal recovery loops", value: 76, target: 81, rangeStart: 50, rangeEnd: 84, unit: "%", note: "heat reuse potential", trace: [26, 38, 49, 66, 72, 76] },
  ],
}

materials: {
  title: "Materials & Manufacturing",
  summary: "EngineTech combines high-temperature alloys, additive tooling, and inspection data to compress the path from design lock to certified hardware.",
  bars: [
    { label: "Nickel superalloy margin", value: 91, target: 94, rangeStart: 68, rangeEnd: 96, unit: "%", note: "thermal headroom", trace: [44, 61, 70, 79, 86, 91] },
    { label: "Additive chamber tooling", value: 72, target: 80, rangeStart: 48, rangeEnd: 86, unit: "%", note: "lead-time reduction", trace: [19, 34, 48, 53, 67, 72] },
    { label: "Sub-micron inspection yield", value: 96, target: 97, rangeStart: 82, rangeEnd: 99, unit: "%", note: "accepted components", trace: [71, 77, 84, 89, 94, 96] },
    { label: "Reusable test article cycles", value: 84, target: 88, rangeStart: 62, rangeEnd: 91, unit: "%", note: "qualification depth", trace: [36, 52, 64, 71, 79, 84] },
  ],
}

fuels: {
  title: "Fuels & Upstream",
  summary: "Fuel-path analysis links propellant availability, storage constraints, and injector behavior before a program commits to flight architecture.",
  bars: [
    { label: "Methane supply compatibility", value: 78, target: 83, rangeStart: 52, rangeEnd: 88, unit: "%", note: "regional availability", trace: [22, 31, 46, 58, 69, 78] },
    { label: "Kerosene retrofit readiness", value: 64, target: 70, rangeStart: 40, rangeEnd: 74, unit: "%", note: "legacy platforms", trace: [28, 35, 39, 52, 57, 64] },
    { label: "Cryogenic storage stability", value: 88, target: 92, rangeStart: 66, rangeEnd: 95, unit: "%", note: "validated envelopes", trace: [45, 56, 68, 74, 83, 88] },
    { label: "Injector response confidence", value: 92, target: 94, rangeStart: 70, rangeEnd: 97, unit: "%", note: "hot-fire data", trace: [48, 62, 73, 85, 89, 92] },
  ],
}

hydrogen: {
  title: "H2 Hydrogen",
  summary: "Hydrogen programs require tight coordination between tankage, feed systems, ignition stability, and ultra-low-temperature operations.",
  bars: [
    { label: "Hydrogen-ready turbopumps", value: 86, target: 90, rangeStart: 62, rangeEnd: 93, unit: "%", note: "design maturity", trace: [30, 46, 60, 71, 79, 86] },
    { label: "LH2 feedline conditioning", value: 74, target: 82, rangeStart: 47, rangeEnd: 86, unit: "%", note: "ground systems", trace: [18, 29, 44, 58, 66, 74] },
    { label: "Ignition stability range", value: 93, target: 95, rangeStart: 72, rangeEnd: 98, unit: "%", note: "transient control", trace: [54, 68, 75, 84, 90, 93] },
    { label: "Zero-carbon flight pathway", value: 81, target: 87, rangeStart: 56, rangeEnd: 90, unit: "%", note: "program fit", trace: [24, 39, 55, 68, 76, 81] },
  ],
}
```

**Tab click behavior:**

1. On click, update `.is-active` class and `aria-selected` on all tab buttons.
2. Remove `.is-visible` from summary and `.is-ready` from chart.
3. After a 140ms delay:
   - Update summary text content.
   - Replace chart innerHTML with new chart-head, bars, and axis markup (using the dataset for the active tab).
   - On the next `requestAnimationFrame`, add `.is-visible` to summary and `.is-ready` to chart, triggering all staggered CSS animations.

---

### RESPONSIVE BREAKPOINTS

**At 980px:**
- Header becomes single column (`grid-template-columns: 1fr`).
- Tabs become flex row with `overflow-x: auto`. Each tab: `flex: 0 0 min(260px, 76vw)`.
- Bar rows become single column (`grid-template-columns: 1fr`, gap 10px).
- Axis becomes single column. Left label `<span>` hidden (`display: none`).

**At 620px:**
- H2 font: `clamp(26px, 8vw, 42px)`.
- Chart: min-height auto, padding-bottom 46px.
- Axis inner grid: `repeat(6, 1fr)` with every even `<span>` hidden (showing 0, 20, 40, 60, 80, 100).

---

### GLOBAL STYLES

**CSS custom property used:** `--hero-max-width: 1820px`, `--hero-blue: #7191d0`, `--hero-blue-soft: #aab8d5`.

**Font stack:** `"Geist", "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` with `-webkit-font-smoothing: antialiased` and `text-rendering: geometricPrecision`.

**Color palette:** No purple or violet. Dark backgrounds `#111414` / `#171a1a`. Blue accents `#7191d0`, `#aab8d5`, `#8fb0ef`, `#d6e3ff`. Text `#f7f8f8` at various opacities.

## Kova Testimonial — Testimonial [sites/kova-testimonial]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(51).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/kova-testimonial.webp

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

## Arceage Testimonial — Testimonials [sites/arceage-testimonial]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(24).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/arceage-testimonial.webp

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

## Radial Diagram — Testimonials [sites/radial-diagram]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(62).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/radial-diagram.webp

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

## Halo Use Case — Use Case [sites/halo-use-case]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(18).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/halo-use-case.webp

**Prompt:**

Build a "Use Cases" section for a fintech stablecoin landing page using **React + TypeScript + Tailwind CSS** with **lucide-react** for icons (use `ArrowRight`). Make it fully mobile responsive.

---

### Font Setup

The page uses **"TT Norms Pro"** loaded via a stylesheet link in `index.html`:

```html
<link href="https://db.onlinewebfonts.com/c/49bf5d043a27b890a040cf393277e2b2?family=TT+Norms+Pro+Regular" rel="stylesheet">
```

Add this `<link>` inside the `<head>` of your `index.html`.

Then in `index.css`, apply it globally:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html {
    font-family: 'TT Norms Pro Regular', ui-sans-serif, system-ui, sans-serif;
  }
  body {
    font-family: 'TT Norms Pro Regular', ui-sans-serif, system-ui, sans-serif;
  }
  * {
    font-family: inherit;
  }
}
```

No local font files are needed. The font is served from the external stylesheet URL above.

---

### Section Component: `UseCasesSection`

**Outer wrapper:** `<section>` with classes `bg-[#F5F5F5] px-6 py-24`.

**Inner container:** `<div>` with classes `max-w-[88rem] mx-auto grid grid-cols-1 md:grid-cols-2 gap-8 items-start`.

---

### LEFT COLUMN (text-only intro)

Wrapper `<div>` with classes `md:pr-12 md:pt-2`.

Contains three elements stacked vertically:

1. **Eyebrow label:**
   - Element: `<p>`
   - Text: **"USD Halo in Practice"**
   - Classes: `text-black/60 text-sm font-normal mb-2`

2. **Section heading:**
   - Element: `<h2>`
   - Text: **"Use modes"**
   - Classes: `text-black text-5xl md:text-6xl font-medium leading-none mb-6`
   - Inline style: `{ letterSpacing: '-0.04em' }`

3. **Description paragraph:**
   - Element: `<p>`
   - Text: **"USD Halo powers a wide range of modes for builders, companies and treasuries wanting safe and rewarding stablecoin integrations plus more"**
   - Classes: `text-black/60 text-base leading-relaxed max-w-sm`

---

### RIGHT COLUMN (large video background card)

Wrapper `<div>` with classes `relative rounded-3xl overflow-hidden min-h-[720px]`.

**Background video** (fills entire card as ambient background):
- Element: `<video>`
- Classes: `absolute inset-0 w-full h-full object-cover`
- Attributes: `autoPlay`, `muted`, `loop`, `playsInline`
- `src` URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_183428_ab5e672a-f608-4dcb-b319-f3e040f02e2d.mp4`

**Content overlay** (sits above video):
- Wrapper `<div>` with classes `relative z-10 p-10 md:p-12`

Contains three elements:

1. **Card heading:**
   - Element: `<h3>`
   - Text: **"Commerce"**
   - Classes: `text-black text-4xl md:text-5xl font-medium leading-tight mb-5`
   - Inline style: `{ letterSpacing: '-0.03em' }`

2. **Card description:**
   - Element: `<p>`
   - Text: **"Lift customer retention by offering USD Halo, a trusted dollar-backed stablecoin with strong yields, letting your patrons earn with zero effort on your platform."**
   - Classes: `text-black/70 text-base leading-relaxed max-w-md mb-8`

3. **"Know more" button:**
   - Element: `<button>`
   - Classes: `inline-flex items-center gap-3 text-black text-base font-medium group`
   - Contains (in this exact order):
     - **Icon circle first:** `<span>` with classes `w-9 h-9 rounded-full bg-white/80 backdrop-blur flex items-center justify-center group-hover:bg-white transition-colors`, containing `<ArrowRight className="w-4 h-4 text-black" />` from lucide-react.
     - **Text label second:** the plain text **"Know more"** (placed after the span, so icon is on the left).

---

### Key Design Specifications

- **Page background:** `#F5F5F5` (light warm gray).
- **Video card:** Video fills the entire rounded card via `object-cover` and loops silently. There is **no gradient overlay, no dark scrim, no blur layer** -- text sits directly on the video.
- **Card corner radius:** `rounded-3xl` (24px).
- **Card minimum height:** `min-h-[720px]`.
- **Typography system:**
  - All text uses inherited "TT Norms Pro Regular" from the web font link.
  - Headings: `font-medium` with tight negative letter-spacing (`-0.04em` for section title, `-0.03em` for card title).
  - Body text: default weight (400), `text-base` size.
  - Color hierarchy: `text-black` for headings, `text-black/70` for card body text, `text-black/60` for muted/secondary text.
- **"Know more" button:** Frosted-glass circle icon (`bg-white/80 backdrop-blur`) transitions to solid white on hover via Tailwind `group`/`group-hover`. Icon circle comes before text label.
- **Layout:** Two-column grid on `md:` breakpoint. Stacks to single column on mobile. Left column has `md:pr-12` and `md:pt-2` for breathing room.
- **Spacing:** `gap-8` between columns. Section padding `py-24` vertical, `px-6` horizontal.

---

### Complete JSX Reference

```tsx
import { ArrowRight } from 'lucide-react';

function UseCasesSection() {
  return (
    <section className="bg-[#F5F5F5] px-6 py-24">
      <div className="max-w-[88rem] mx-auto grid grid-cols-1 md:grid-cols-2 gap-8 items-start">
        {/* Left column */}
        <div className="md:pr-12 md:pt-2">
          <p className="text-black/60 text-sm font-normal mb-2">USD Halo in Practice</p>
          <h2 className="text-black text-5xl md:text-6xl font-medium leading-none mb-6" style={{ letterSpacing: '-0.04em' }}>
            Use modes
          </h2>
          <p className="text-black/60 text-base leading-relaxed max-w-sm">
            USD Halo powers a wide range of modes for builders, companies and treasuries wanting safe and rewarding stablecoin integrations plus more
          </p>
        </div>

        {/* Right column -- big card with bg video */}
        <div className="relative rounded-3xl overflow-hidden min-h-[720px]">
          <video
            className="absolute inset-0 w-full h-full object-cover"
            src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_183428_ab5e672a-f608-4dcb-b319-f3e040f02e2d.mp4"
            autoPlay
            muted
            loop
            playsInline
          />
          <div className="relative z-10 p-10 md:p-12">
            <h3 className="text-black text-4xl md:text-5xl font-medium leading-tight mb-5" style={{ letterSpacing: '-0.03em' }}>
              Commerce
            </h3>
            <p className="text-black/70 text-base leading-relaxed max-w-md mb-8">
              Lift customer retention by offering USD Halo, a trusted dollar-backed stablecoin with strong yields, letting your patrons earn with zero effort on your platform.
            </p>
            <button className="inline-flex items-center gap-3 text-black text-base font-medium group">
              <span className="w-9 h-9 rounded-full bg-white/80 backdrop-blur flex items-center justify-center group-hover:bg-white transition-colors">
                <ArrowRight className="w-4 h-4 text-black" />
              </span>
              Know more
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}
```

---

## No-Code Waitlist — Waitlist [sites/no-code-waitlist]

- Preview: https://stream.mux.com/iY611PYuZ02AKpIzYB4Q1BMpl7W2O3UwqjQm9p01xlmVg.m3u8
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/no-code-waitlist.m3u8

Build a full-screen dark hero section landing page in React + Vite + Tailwind CSS v4 + Motion (framer-motion) + Lucide React icons + hls.js. The page should be a single screen (100vh, no scroll) with a black background, a fullscreen background video, a glassmorphism navbar, and a centered hero with an email capture CTA.
>
> **Dependencies:** `react`, `react-dom`, `motion`, `hls.js`, `lucide-react`, `tailwindcss` v4 with `@tailwindcss/vite`, `@vitejs/plugin-react`
>
> **Fonts:** Import Google Fonts:
> - `Inter` (weights 300, 400, 500, 600) -- used as the base sans-serif font
> - `Instrument Serif` (regular and italic) -- used for the hero heading
>
> **CSS (`index.css`):**
> - Import both Google Font URLs, then `@import "tailwindcss";`
> - Set `@theme { --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif; }`
> - `:root` variables: `--background: #000000; --foreground: #ffffff;`
> - `body`: background-color var(--background), color var(--foreground), font-family var(--font-sans), `-webkit-font-smoothing: antialiased`, `letter-spacing: -0.01em`
> - `.liquid-glass` class: `background: rgba(255,255,255,0.01)`, `background-blend-mode: luminosity`, `backdrop-filter: blur(4px)`, `-webkit-backdrop-filter: blur(4px)`, `border: none`, `box-shadow: inset 0 1px 1px rgba(255,255,255,0.1)`, `position: relative`, `overflow: hidden`. It has a `::before` pseudo-element for a gradient border effect: `padding: 1.4px`, `background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)`, masked with `-webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0)` and `-webkit-mask-composite: xor; mask-composite: exclude;`
> - `.glass-pill` class: `background: rgba(255,255,255,0.04)`, `backdrop-filter: blur(16px) saturate(180%)`, `border-radius: 9999px`, `box-shadow: none !important`
>
> **Background Video component:**
> - Renders an absolutely positioned `<div>` covering the full parent (`absolute inset-0 overflow-hidden pointer-events-none`)
> - Contains a `<video>` element: `autoPlay`, `muted`, `loop`, `playsInline`, classes `w-full h-full object-cover opacity-100`
> - Video source URL: `https://stream.mux.com/kimF2ha9zLrX64H00UgLGPflCzNtl1T0215MlAmeOztv8.m3u8` (this is an HLS stream from Mux, NOT CloudFront)
> - Uses `hls.js`: if the browser natively supports HLS (`video.canPlayType("application/vnd.apple.mpegurl")`), set `video.src` directly; otherwise instantiate `new Hls()`, `loadSource`, `attachMedia`
>
> **Navbar component:**
> - Animates in with `motion.nav`: `initial={{ y: -20, opacity: 0 }}`, `animate={{ y: 0, opacity: 1 }}`
> - Classes: `relative z-20 px-6 py-6 w-full`
> - Inner container: `liquid-glass rounded-full px-6 py-3 flex items-center justify-between max-w-5xl mx-auto`
> - Left side (`flex items-center gap-8`):
>   - Logo: `Globe` icon from lucide-react (w-6 h-6 text-white) + "Asme" text (`text-white font-semibold text-lg`), in a `flex items-center gap-2` wrapper
>   - Nav links: "Features", "Pricing", "About" -- hidden on mobile (`hidden md:flex`), `items-center gap-8 text-white/80 text-sm font-medium`, each link has `hover:text-white transition-colors duration-300`
> - Right side (`flex items-center gap-4`):
>   - "Sign Up" plain text button: `text-white hover:text-white/80 transition-colors text-sm font-medium cursor-pointer`
>   - "Login" glassmorphism button: `liquid-glass rounded-full px-6 py-2 text-sm font-medium text-white hover:opacity-90 transition-opacity cursor-pointer`
>
> **Hero component:**
> - `<section>` with `relative flex-1 flex flex-col items-center justify-center px-6`
> - Content wrapper: `relative z-10 text-center max-w-5xl mx-auto flex flex-col items-center justify-center w-full gap-12`
> - **Tagline** (motion.p): text "BUILD A NO-CODE AI APP IN MINUTES", `text-white/80 text-[10px] md:text-[11px] font-medium tracking-[0.2em] uppercase mb-4`, animates `initial={{ opacity: 0, y: 10 }}`, `animate={{ opacity: 1, y: 0 }}`, `transition={{ delay: 0.1 }}`
> - **Heading** (motion.h1): text "A new way to think and create with computers" (with `<br className="hidden md:block" />` after "create"), `fontFamily: "'Instrument Serif', serif"` set via inline style, classes `text-4xl md:text-[64px] font-medium tracking-[-0.01em] leading-[1.1] mb-6 bg-gradient-to-b from-white via-white/95 to-white/70 bg-clip-text text-transparent max-w-4xl`, animates `initial={{ opacity: 0, y: 20 }}`, `animate={{ opacity: 1, y: 0 }}`, `transition={{ duration: 1, ease: [0.16, 1, 0.3, 1] }}`
> - **CTA area** (motion.div): `min-h-[50px] mt-2`, animates with `delay: 0.4`. Uses `AnimatePresence mode="wait"` to toggle between:
>   - **Button state**: "Get early access" -- `px-10 py-3 text-[14px] font-medium border border-white/10 rounded-full hover:border-white/30 hover:bg-white/[0.02] transition-all duration-300 text-white/90 backdrop-blur-sm cursor-pointer`. On click, switches to email form.
>   - **Email form state**: a `<form>` with `flex items-center gap-2 pl-5 pr-1.5 py-1.5 text-[14px] font-medium border border-white/20 rounded-full bg-white/[0.02] backdrop-blur-sm w-full max-w-[320px] focus-within:border-white/40 transition-colors duration-300`. Contains an email `<input>` (transparent background, white text, `placeholder-white/45`, `autoFocus`) and a submit button with either `ArrowRight` icon (default) or `Check` icon (after submit). Both states animate scale 0.95 to 1 with 0.2s duration.
>   - **Typewriter placeholder**: when the email form opens, the placeholder text "Enter Your Email Here For Early Access" types in character by character at 60ms intervals. After submission, it types "You Will Receive Notifications By Email" instead. After 4 seconds, it resets back to the button state.
> - **"Play Video Demo"** link below (motion.div with `delay: 0.8` fade-in): `text-white/80 hover:text-white/40 transition-colors duration-300 text-[13px] font-medium tracking-wide`
>
> **App root layout:**
> - `<main>` with `relative bg-black h-screen w-screen flex flex-col overflow-hidden selection:bg-white selection:text-black shrink-0`
> - Render order: `BackgroundVideo`, `Navbar`, `Hero`
> - Text selection is styled white bg with black text

---

Key clarification: The video URL is **not** from CloudFront. It is an HLS stream hosted on **Mux**: `https://stream.mux.com/kimF2ha9zLrX64H00UgLGPflCzNtl1T0215MlAmeOztv8.m3u8`. The `.m3u8` format requires hls.js for non-Safari browsers.

## Halo Benefits — Why Us [sites/halo-benefits]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/animated%20(13).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/halo-benefits.webp

Build an "Info" section for a fintech stablecoin landing page using **React + TypeScript + Tailwind CSS** with **lucide-react** for icons (use `ArrowRight`). Fully mobile responsive.

### Font Setup

Load **"TT Norms Pro"** via a stylesheet link in `index.html`:

```html
<link href="https://db.onlinewebfonts.com/c/49bf5d043a27b890a040cf393277e2b2?family=TT+Norms+Pro+Regular" rel="stylesheet">
```

Apply globally in `index.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html {
    font-family: 'TT Norms Pro Regular', ui-sans-serif, system-ui, sans-serif;
  }
  body {
    font-family: 'TT Norms Pro Regular', ui-sans-serif, system-ui, sans-serif;
  }
  * {
    font-family: inherit;
  }
}
```

---

### Section Component: `InfoSection`

**Outer wrapper:** `<section>` with classes `bg-[#F5F5F5] px-6 py-24`.

**Inner container:** `<div>` with classes `max-w-[88rem] mx-auto`.

The section has **two rows** stacked vertically.

---

### ROW 1: Heading + Description (two-column grid)

Wrapper `<div>` with classes `grid grid-cols-1 md:grid-cols-2 gap-12 mb-16 items-start`.

**Left column** (`<div>`, no extra classes):

1. **Heading:**
   - Element: `<h2>`
   - Text: **"Meet USD Halo."**
   - Classes: `text-black text-4xl md:text-5xl font-medium leading-tight mb-8`
   - Inline style: `{ letterSpacing: '-0.03em' }`

2. **CTA pill button** (black capsule, text-left icon-right pattern):
   - Element: `<button>`
   - Classes: `inline-flex items-center gap-3 bg-black text-white text-base font-medium pl-8 pr-2 py-2 rounded-full hover:bg-gray-800 transition-colors duration-200`
   - Contains:
     - Plain text: **"Discover it"**
     - Then a `<span>` with classes `bg-white rounded-full p-2 flex items-center justify-center`, containing `<ArrowRight className="w-5 h-5 text-black" />`

**Right column** (`<div>` with classes `flex items-center`):

1. **Description paragraph:**
   - Element: `<p>`
   - Text: **"USD Halo is a reward-earning dollar coin that lets your savings grow while remaining tied to the U.S. dollar."**
   - Classes: `text-black/70 text-2xl md:text-3xl font-normal leading-relaxed`

---

### ROW 2: Four-column card grid

Wrapper `<div>` with classes `grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4`.

Contains 3 cards (the first spans 2 columns):

**Card 1 -- Image background card (spans 2 columns):**
- Wrapper `<div>` with classes `lg:col-span-2 rounded-2xl overflow-hidden relative min-h-80`
- Inline style with background image:
  ```tsx
  style={{
    backgroundImage: `url('https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260423_164207_f243351d-ed59-48ec-83a0-a5e996bdbe3c.png&w=1280&q=85')`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
  }}
  ```
- Inner content `<div>` with classes `relative z-10 flex flex-col justify-between h-full p-7 min-h-80`
  - **Title:** `<p>` with text **"Savings that bloom"**, classes `text-black text-2xl font-medium leading-snug`, inline style `{ letterSpacing: '-0.02em' }`
  - **Description:** `<p>` with text **"Gain steady returns as your dollar tokens are routed into top-performing DeFi strategies."**, classes `text-black/70 text-base font-normal leading-relaxed max-w-xs`

**Card 2 -- Dark solid card:**
- Wrapper `<div>` with classes `rounded-2xl p-7 flex flex-col justify-between min-h-80`
- Inline style: `{ backgroundColor: '#2B2644' }`
- **Title:** `<p>` with text **"Always fluid,\<br /\>always pegged."**, classes `text-white text-2xl font-medium leading-snug`, inline style `{ letterSpacing: '-0.02em' }`
- **Description:** `<p>` with text **"Keep fully dollar-anchored with on-demand access to funds -- no lockups or waits."**, classes `text-white/60 text-base font-normal leading-relaxed`

**Card 3 -- Dark solid card:**
- Wrapper `<div>` with classes `rounded-2xl p-7 flex flex-col justify-between min-h-80`
- Inline style: `{ backgroundColor: '#2B2644' }`
- **Title:** `<p>` with text **"Fully\<br /\>automated"**, classes `text-white text-2xl font-medium leading-snug`, inline style `{ letterSpacing: '-0.02em' }`
- **Description:** `<p>` with text **"Skip the task of tuning positions yourself. USD Halo runs in the background for you."**, classes `text-white/60 text-base font-normal leading-relaxed`

---

### Key Design Specifications -- InfoSection

- **Background:** `#F5F5F5` seamless with rest of page.
- **Row 1 layout:** Two equal columns, `gap-12`, with `mb-16` separating it from the card grid below. Left column has heading + button stacked. Right column vertically centers a large descriptive paragraph.
- **Pill button:** Black capsule with white text on left, white circle with black arrow icon on right. Asymmetric padding (`pl-8 pr-2 py-2`) creates the capsule-with-embedded-circle look.
- **Card grid:** 4-column on `lg`, 2-column on `sm`, single column on mobile. `gap-4` between cards.
- **Card 1** spans 2 columns on `lg`. Uses a full-bleed background image with no overlay/scrim -- text sits directly on the image. Content is distributed top-to-bottom using `justify-between`.
- **Cards 2 & 3** use a dark navy/purple background (`#2B2644`) with white text. Titles use `<br />` for line breaks. Content distributed top-to-bottom via `justify-between`.
- **All cards:** `rounded-2xl` (16px), `min-h-80` (320px), `p-7` internal padding.
- **Typography:** Card titles are `text-2xl font-medium` with `-0.02em` letter-spacing. Card descriptions are `text-base font-normal`.

---

### Complete JSX -- InfoSection

```tsx
import { ArrowRight } from 'lucide-react';

function InfoSection() {
  return (
    <section className="bg-[#F5F5F5] px-6 py-24">
      <div className="max-w-[88rem] mx-auto">
        {/* Row 1 */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-12 mb-16 items-start">
          <div>
            <h2 className="text-black text-4xl md:text-5xl font-medium leading-tight mb-8" style={{ letterSpacing: '-0.03em' }}>
              Meet USD Halo.
            </h2>
            <button className="inline-flex items-center gap-3 bg-black text-white text-base font-medium pl-8 pr-2 py-2 rounded-full hover:bg-gray-800 transition-colors duration-200">
              Discover it
              <span className="bg-white rounded-full p-2 flex items-center justify-center">
                <ArrowRight className="w-5 h-5 text-black" />
              </span>
            </button>
          </div>
          <div className="flex items-center">
            <p className="text-black/70 text-2xl md:text-3xl font-normal leading-relaxed">
              USD Halo is a reward-earning dollar coin that lets your savings grow while remaining tied to the U.S. dollar.
            </p>
          </div>
        </div>

        {/* Row 2 -- 4-col grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {/* Card 1 -- spans 2 cols, image bg */}
          <div
            className="lg:col-span-2 rounded-2xl overflow-hidden relative min-h-80"
            style={{
              backgroundImage: `url('https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260423_164207_f243351d-ed59-48ec-83a0-a5e996bdbe3c.png&w=1280&q=85')`,
              backgroundSize: 'cover',
              backgroundPosition: 'center',
            }}
          >
            <div className="relative z-10 flex flex-col justify-between h-full p-7 min-h-80">
              <p className="text-black text-2xl font-medium leading-snug" style={{ letterSpacing: '-0.02em' }}>
                Savings that bloom
              </p>
              <p className="text-black/70 text-base font-normal leading-relaxed max-w-xs">
                Gain steady returns as your dollar tokens are routed into top-performing DeFi strategies.
              </p>
            </div>
          </div>

          {/* Card 2 */}
          <div
            className="rounded-2xl p-7 flex flex-col justify-between min-h-80"
            style={{ backgroundColor: '#2B2644' }}
          >
            <p className="text-white text-2xl font-medium leading-snug" style={{ letterSpacing: '-0.02em' }}>
              Always fluid,<br />always pegged.
            </p>
            <p className="text-white/60 text-base font-normal leading-relaxed">
              Keep fully dollar-anchored with on-demand access to funds -- no lockups or waits.
            </p>
          </div>

          {/* Card 3 */}
          <div
            className="rounded-2xl p-7 flex flex-col justify-between min-h-80"
            style={{ backgroundColor: '#2B2644' }}
          >
            <p className="text-white text-2xl font-medium leading-snug" style={{ letterSpacing: '-0.02em' }}>
              Fully<br />automated
            </p>
            <p className="text-white/60 text-base font-normal leading-relaxed">
              Skip the task of tuning positions yourself. USD Halo runs in the background for you.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
```

---

---

### Prompt 2: BackedBySection (Marquee)

Build a "Backed By" marquee section for a fintech stablecoin landing page using **React + TypeScript + Tailwind CSS**. No icon library needed for this section.

### Font Setup

Same as above -- load **"TT Norms Pro"** via the stylesheet link in `index.html`:

```html
<link href="https://db.onlinewebfonts.com/c/49bf5d043a27b890a040cf393277e2b2?family=TT+Norms+Pro+Regular" rel="stylesheet">
```

---

### Section Component: `BackedBySection`

**Outer wrapper:** `<section>` with classes `bg-[#F5F5F5] px-6` (no vertical padding -- this section sits flush between InfoSection and UseCasesSection).

**Inner container:** `<div>` with classes `max-w-[88rem] mx-auto grid grid-cols-1 md:grid-cols-4 gap-8 items-center`.

---

### LEFT COLUMN (1 of 4 columns)

Wrapper `<div>` with classes `md:col-span-1`.

Contains:
- `<p>` with text **"Funded by premier partners\<br /\>and forward-thinking leaders."**
- Classes: `text-black/70 text-base leading-relaxed`

---

### RIGHT COLUMN -- Marquee (3 of 4 columns)

Wrapper `<div>` with classes `md:col-span-3 overflow-hidden`.

**CSS animation** (injected via inline `<style>` tag inside the component):

```css
@keyframes backers-marquee {
  0% { transform: translateX(0); }
  100% { transform: translateX(-50%); }
}
.backers-track {
  display: flex;
  width: max-content;
  animation: backers-marquee 30s linear infinite;
}
```

**Marquee track:** `<div>` with class `backers-track`.

Contains the `BACKER_BRANDS` array duplicated (spread twice: `[...BACKER_BRANDS, ...BACKER_BRANDS]`) and mapped to `<span>` elements.

Each `<span>`:
- Classes: `mx-10 shrink-0 text-black/50 whitespace-nowrap`
- Inline `style` from the brand object (unique font, weight, spacing per brand)
- Key: array index `i`

---

### Brand Data Array

Each brand has a unique font family, weight, letter-spacing, and font-size to simulate distinct brand wordmarks using only text styling (no logos/images):

```tsx
const BACKER_BRANDS: { name: string; style: React.CSSProperties }[] = [
  { name: "Fundamental Labs", style: { fontFamily: "'Times New Roman', serif", fontWeight: 400, letterSpacing: "0.02em", fontSize: "14px" } },
  { name: "KUCOIN", style: { fontFamily: "'Arial Black', sans-serif", fontWeight: 900, letterSpacing: "0.08em", fontSize: "16px" } },
  { name: "NGC", style: { fontFamily: "'Impact', sans-serif", fontWeight: 700, letterSpacing: "0.05em", fontSize: "18px" } },
  { name: "NxGen", style: { fontFamily: "'Georgia', serif", fontWeight: 600, letterSpacing: "-0.02em", fontSize: "17px" } },
  { name: "Matter Labs", style: { fontFamily: "'Helvetica', sans-serif", fontWeight: 700, letterSpacing: "-0.01em", fontSize: "15px" } },
  { name: "DEXTools", style: { fontFamily: "'Verdana', sans-serif", fontWeight: 700, letterSpacing: "0.06em", fontSize: "14px", textTransform: "uppercase" as const } },
  { name: "NGRAVE", style: { fontFamily: "'Courier New', monospace", fontWeight: 700, letterSpacing: "0.18em", fontSize: "14px" } },
  { name: "Polychain", style: { fontFamily: "'Palatino', serif", fontWeight: 500, letterSpacing: "0.03em", fontSize: "15px" } },
];
```

---

### Key Design Specifications -- BackedBySection

- **No vertical padding** on the section (`px-6` only) -- it sits between two `py-24` sections, acting as a visual divider/strip.
- **Layout:** 4-column grid on `md`. Left column takes 1/4 width, marquee takes 3/4. Stacks on mobile.
- **Marquee technique:** CSS-only infinite scroll. The array is duplicated so the second copy seamlessly replaces the first as it scrolls left. `width: max-content` ensures the track is as wide as its content. `-50%` translation moves exactly one copy's width.
- **Animation speed:** `30s` -- slow, ambient scrolling.
- **Brand styling:** Each brand name uses a different system font, weight, and letter-spacing to create visual variety mimicking actual brand wordmarks without needing logo images. Colors are all `text-black/50` (50% opacity black).
- **Spacing between brands:** `mx-10` (80px total gap between adjacent names).

---

### Complete JSX -- BackedBySection

```tsx
const BACKER_BRANDS: { name: string; style: React.CSSProperties }[] = [
  { name: "Fundamental Labs", style: { fontFamily: "'Times New Roman', serif", fontWeight: 400, letterSpacing: "0.02em", fontSize: "14px" } },
  { name: "KUCOIN", style: { fontFamily: "'Arial Black', sans-serif", fontWeight: 900, letterSpacing: "0.08em", fontSize: "16px" } },
  { name: "NGC", style: { fontFamily: "'Impact', sans-serif", fontWeight: 700, letterSpacing: "0.05em", fontSize: "18px" } },
  { name: "NxGen", style: { fontFamily: "'Georgia', serif", fontWeight: 600, letterSpacing: "-0.02em", fontSize: "17px" } },
  { name: "Matter Labs", style: { fontFamily: "'Helvetica', sans-serif", fontWeight: 700, letterSpacing: "-0.01em", fontSize: "15px" } },
  { name: "DEXTools", style: { fontFamily: "'Verdana', sans-serif", fontWeight: 700, letterSpacing: "0.06em", fontSize: "14px", textTransform: "uppercase" as const } },
  { name: "NGRAVE", style: { fontFamily: "'Courier New', monospace", fontWeight: 700, letterSpacing: "0.18em", fontSize: "14px" } },
  { name: "Polychain", style: { fontFamily: "'Palatino', serif", fontWeight: 500, letterSpacing: "0.03em", fontSize: "15px" } },
];

function BackedBySection() {
  return (
    <section className="bg-[#F5F5F5] px-6">
      <div className="max-w-[88rem] mx-auto grid grid-cols-1 md:grid-cols-4 gap-8 items-center">
        <div className="md:col-span-1">
          <p className="text-black/70 text-base leading-relaxed">
            Funded by premier partners<br />and forward-thinking leaders.
          </p>
        </div>
        <div className="md:col-span-3 overflow-hidden">
          <style>{`
            @keyframes backers-marquee {
              0% { transform: translateX(0); }
              100% { transform: translateX(-50%); }
            }
            .backers-track {
              display: flex;
              width: max-content;
              animation: backers-marquee 30s linear infinite;
            }
          `}</style>
          <div className="backers-track">
            {[...BACKER_BRANDS, ...BACKER_BRANDS].map((brand, i) => (
              <span
                key={i}
                className="mx-10 shrink-0 text-black/50 whitespace-nowrap"
                style={brand.style}
              >
                {brand.name}
              </span>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
```
