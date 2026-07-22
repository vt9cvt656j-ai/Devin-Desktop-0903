# Michael Design Library — sections-app-ui

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 14 entries.

## 404 Planet — 404 [sections/404-planet]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/404-planet.webp

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

## Beauty Categories — Categories [sections/beauty-categories]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/beauty-categories.webp

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

## Dashboard UI — Dashboard [sections/dashboard]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/dashboard.webp

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

## Modern HR Dashboard — Dashboard [sections/modern-hr-dashboard]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/modern-hr-dashboard.png

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

## Nimbus Demo — Dashboard Demo [sections/nimbus-demo]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nimbus-demo.webp

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

## Arceage Contact Us — Form [sections/arceage-contact-us]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/arceage-contact-us.webp

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

## Daisy Sweet — Product [sections/daisy-sweet]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/daisy-sweet.webp

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

## Daisy Wild — Product [sections/daisy-wild]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/daisy-wild.webp

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

## Beauty Products — Products [sections/beauty-products]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/beauty-products.webp

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

## Projects Catalog — Projects [sections/projects-catalog]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/projects-catalog.webp

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

## Price Calculator — SaaS [sections/price-calculator]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/price-calculator.gif

Recreate Project Estimation Calculator Section

Create a full-width dark calculator section with id calculator-section. Background: bg-background, padding py-16 md:py-28 px-4 md:px-16, max-width max-w-7xl centered.

Header: Centered. Small mono uppercase tracking-widest label "Try project estimation calculator" in text-muted-foreground. Below it, an h2: "Get premium website within your budget" — text-3xl md:text-4xl lg:text-5xl font-normal.

Layout: 2-column grid (grid-cols-1 lg:grid-cols-2), rounded-2xl overflow-hidden, no gap.

LEFT COLUMN (Calculator Form): Background #0D0D0D, padding p-8 lg:p-12, sections divided by divide-y divide-[#1E1E1E].

4 sections separated by horizontal dividers:

Service Type (radio buttons): h3 "What kind of service do you need?" — 3 options: "Only Design" (design), "Only Development" (development), "Design + Development" (both, default). Custom radio circles: w-5 h-5 rounded-full border-2, active = border-[#FF5656] with inner w-2 h-2 rounded-full bg-[#FF5656].

Number of Pages (slider): h3 with current value in #FF5656. Shadcn <Slider> min=1, max=30, step=1, default=5. Labels "1" and "30" below.

Add-ons (checkboxes): Two checkboxes with price labels on the right in #FF5656:

"I will need help with content" → +$50/pages

"I want to optimize my website for SEO" → +$50/pages Custom checkboxes: w-5 h-5 border-2 rounded, checked = border-[#FF5656] bg-[#FF5656] with white SVG checkmark.

Timeline (radio buttons): h3 "How fast do you need this?" — 3 options with prices:

"Within 7 Days" → +$100/pages

"Within 14 Days" → +$25/pages

"Regular Speed (Based on discussion)" → no extra cost (default)

RIGHT COLUMN (Cost Estimation): Padding p-8 lg:p-12, border border-white/10 rounded-r-2xl, min-height 717.98px.

h3 "Estimated Cost" + description paragraph.

3 stacked cards (rounded-2xl p-6 space-y-3):

Agency card: bg-muted/50. Title "Typical Agency charges minimum". Large price text-4xl font-bold. Subtitle: "+ Too much extra time & additional cost".

Freelancer card: bg-muted/50. Title "Regular Freelancer charges minimum". Large price text-4xl font-bold. Subtitle: "+ Too much headache & back-and-forth".

Your price card: bg-gradient-to-r from-pink-500 to-orange-500 text-white. Title "With Webfluin Studio". Price text-5xl font-bold. Subtitle: "Save your money, time & headache".

PRICING LOGIC:

calculatePrice():
  Base prices by service:
    design: base=399, perPage=100
    development: base=199, perPage=100
    both: base=499, perPage=200
  
  total = max(base, base + (pages - 1) * perPage)
  if needContent: total += pages * 50
  if needSEO: total += pages * 50
  if rush: total += pages * 100
  if fast: total += pages * 25

calculateAgencyCost():
  perPage = (both ? 1000 : 400)
  return 8000 + (pages - 1) * perPage

calculateFreelancerCost():
  perPage = (both ? 500 : 200)
  return 3000 + (pages - 1) * perPage

All prices displayed with .toLocaleString() and $ prefix.

State: serviceType (design|development|both, default both), pages (number, default 5), needContent (bool), needSEO (bool), timeline (regular|fast|rush, default regular).

Dependencies: Shadcn Slider component, useToast hook.

## Agency Services — Services [sections/agency-services]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/agency-services.webp

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

## Arceage Services — Services [sections/arceage-services]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/arceage-services.webp

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

## Solace sign-in — Sign In Form [sections/solace-sign-in]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/solace-sign-in.webp

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
