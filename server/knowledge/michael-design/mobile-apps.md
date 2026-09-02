# Michael Design Library — mobile-apps

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 22 entries.

## Lodge Booking App — Booking [apps/lodge-booking-app]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/d32f8934d738fe4e9378f37412e72071.webp
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/lodge-booking-app.webp

Create a "WoodNest" luxury cabin booking showcase with 3 phone mockups displayed side-by-side on desktop (stacked on mobile). The design uses a liquid glass / iOS 26 frosted glass aesthetic with a dark background (#030508) and animated organic liquid blobs behind the phones.

**Tech Stack:** React + TypeScript + Vite + Tailwind CSS + lucide-react (Leaf, Menu, Star, X, Calendar, ChevronDown, ChevronLeft, ChevronRight, Pencil icons).

**Fonts:**
- Body: Inter (weights 300, 400, 500, 600) from Google Fonts
- Display headings: General Sans (weights 400, 500, 600, 700) from Fontshare: `https://api.fontshare.com/v2/css?f[]=general-sans@400,500,600,700&display=swap`

**Background:**
- Fixed full-screen layer with 4 animated liquid blobs (organic border-radius shapes, cyan/teal/emerald/white gradients at very low opacity 3-8%, blurred 60-90px, each with unique 18-25s looping morph animation changing position, rotation, scale, and border-radius).
- 3 specular "caustic" highlights (tiny white/cyan circles with blur, animating position/opacity over 8-12s).
- A glass-refraction layer (subtle diagonal gradient with white at 0.5-1% opacity + 0.5px backdrop blur).
- A noise texture overlay (SVG feTurbulence fractalNoise) at 1.5% opacity.

**Phone Frame:**
- Desktop: 380px wide, 780px tall, border-radius 50px, border 8px solid #2a2a3a, outer 2px ring #1a1a2a, heavy box-shadow (40px/80px black shadows + inset 20px). Dynamic Island notch (::before pseudo-element): 120x28px, #1a1a2a, centered at top, border-radius 0 0 16px 16px. Inner .phone-content: 100% size, overflow hidden, border-radius 42px.
- Mobile: `width: calc(100vw - 40px)`, `height: calc((100vw - 40px) * 2.05)`, border-radius 40px, border 6px. Notch shrinks to 100x24px. Inner content border-radius 34px.

**Container layout:** `flex flex-col md:flex-row items-center gap-6 md:gap-[50px] justify-center`, outer wrapper has 20px padding (mobile), 32px padding (desktop).

---

**PHONE 1 (Left) - Featured Lodges listing:**

- Background: looping video `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260620_140846_aef8cb19-5ec8-4b45-974b-020aed20f297.mp4`
- Content overlay gradient: top rgba(30,60,80,0.4) -> mid rgba(30,60,80,0.2) -> lower rgba(10,30,40,0.6) -> bottom rgba(10,20,30,0.9)
- Header: Leaf icon (amber-400) + "WoodNest" label + hamburger Menu toggle (opens fullscreen frosted nav with staggered fadeIn links: Locations, Rooms, Experiences, Contact)
- Title: "Featured Lodges" (font-display, 2.5rem, font-light, white) + subtitle "This week's most loved retreats" (white/60, sm)
- Card 1 (liquid-glass-dropdown background): video thumbnail `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_193839_c320d45f-58ff-4c65-b4dc-b6afd855f68f.mp4` (h-48, rounded-[16px], autoplay muted loop), title "Spruce Hill Lodge", price "$450/night" (3xl font-light) + "+$25.00 taxes", white "Reserve" button (rounded-[16px])
- Card 2 (bg-blue-400/5 backdrop-blur-sm): video `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_193851_8f42b8d7-c4e2-480f-8c2e-4a8415e67774.mp4`, title "Cedar Valley Cabin", price "$320/night" + "+$18.00 taxes", same Reserve button

---

**PHONE 2 (Center) - Hero / Landing:**

- Background: static image (as background-image on a div): `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_141501_77d33995-a443-4890-ad2a-afb0108874ea.png&w=1280&q=85`
- Same header (Leaf + "WoodNest" + menu toggle)
- Hero title: "Nature's Perfect Hideaways" (font-display, 3rem, font-light, "Perfect" is white/50)
- Subtitle paragraph: "Discover handpicked luxury cabins in breathtaking locations. Unplug, unwind, and reconnect with what matters most." (white/70, sm, max-w-[260px])
- Bottom section: Rating badge aligned right (Star icon filled amber-400 + "4.7" bold xl + "from 1,800+ stays"), and a full-width "Book Now" button with white/95 background, rounded-[16px], and an animated pulse-glow box-shadow (amber at 10-25% oscillating)

---

**PHONE 3 (Right) - Reservation / Booking form:**

- Background: same video as Phone 1 (`hf_20260620_140846_...`)
- Same overlay gradient + header + menu
- Title: "Reserve Your Retreat" (font-display, 2.2rem, font-light)
- Main card (liquid-glass-dropdown): static image `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_140711_d9bf3815-d23c-4737-9390-2eb93594839a.png&w=1280&q=85` (h-56, rounded-[16px])
- Lodge name: "Evergreen Pine Family Lodge" (xl, font-light) with a pencil edit button (round frosted glass)
- Interactive date pickers: two side-by-side pickers showing Calendar icon + formatted date (default: Feb 11 and Mar 25, 2026). Clicking opens a full inline calendar with: month navigation (ChevronLeft/Right), day headers (Su-Sa), day grid with selected dates highlighted white, in-range dates at white/15, others white/40 hover white/10.
- Check-in/Check-out time selector: split row (border-r divider), showing "After 2:00 PM" / "Until 12:00 PM" by default. Clicking opens a scrollable time picker dropdown (liquid-glass-time-dropdown: rgba(10,18,35,0.95), 40px blur, border 1px white/8).
- Price: "$359/night" (3xl font-light) + "2-5 guests" aligned right
- Full-width white "Reserve" button

---

**Glass effects used throughout:**
- `.liquid-glass-menu`: rgba(15,25,45,0.4), backdrop-filter blur(24px) saturate(1.4), inset top 1px white/6 border, shadow 20px 60px black/40
- `.liquid-glass-dropdown`: rgba(15,25,45,0.4), backdrop-filter blur(20px) saturate(1.3), inset top 1px white/6 border, shadow 12px 40px black/40
- `.liquid-glass-time-dropdown`: rgba(10,18,35,0.95), backdrop-filter blur(40px) saturate(1.5), border 1px white/8, shadow 16px 48px black/60

**Animations:**
- `.animate-fade-in-up`: translateY(24px)->0, 0.8s cubic-bezier(0.16,1,0.3,1)
- `.animate-fade-in-scale`: scale(0.92)->1, 0.9s same easing
- Staggered delays: 0.15s, 0.3s, 0.5s, 0.7s, 0.9s, 1.1s (elements start at opacity:0)
- `.btn-hover`: scale(1.03) + white glow shadow on hover, scale(0.97) on active
- `.card-hover`: translateY(-4px) + deeper shadow on hover
- `.animate-pulse-glow`: 3s infinite amber glow oscillation on Book Now button

## Church Community — Church [apps/church-community]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/churchserice.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/church-community.mp4

Create a mobile app showcase page displaying 3 iPhone mockups for "Christ Family Church" -- a church website mobile design. Use React Native with Expo Router, react-native-reanimated, and Manrope font (weights: Light, Regular, Medium).

**General layout:**
- Background color: `#5A4C41`
- 3 iPhone frames shown side by side on desktop (horizontal scroll), stacked vertically on mobile (<900px) with scale-to-fit
- Desktop: 40px padding, 48px gap between phones
- Mobile: 20px padding, 32px gap, phones scale down to fit screen width
- Each phone frame: 375x812 artboard inside a black rounded frame (borderRadius 50, borderWidth 2, borderColor `#2a2a2a`), with dynamic island notch (126x36, black, borderRadius 18) and home indicator bar (134x5, white 30% opacity)
- Soft shadow on frames: `shadowColor: #000, offset: {0, 12}, opacity: 0.3, radius: 24`

**Assets (all from framerusercontent.com):**
- Portrait photo: `https://framerusercontent.com/images/7nIpqB1Y0QYgLe70j5NmdtK5Rk.png`
- Logo (cross icon): `https://framerusercontent.com/images/Fr3jIzrNgNkSo8ZgFFTkpS308.png`
- Quote mark icon: `https://framerusercontent.com/images/DmIPflrtvNHr7mnr6k3K5Ayn8w.png`
- Star/compass decorative icon: `https://framerusercontent.com/images/yi0dRg7NDCZUtTbPxCa115nMU5M.png`
- Hero image (hands raised worship): `https://framerusercontent.com/images/G9ZdWZubRnpc37d5d7uUzqaBqiw.png`
- Avatars row: `https://framerusercontent.com/images/l3LBaTwnoXLWZd6axR7m3Q9iWeU.png`
- Arrow icon: `https://framerusercontent.com/images/zBmfi9e2hdwkTHpcMqbS61FIc3c.png`
- Screen 3 hero (preacher): `https://framerusercontent.com/images/Q7jLZsObox26xQCiWPAVYWzTsYs.png`
- Play button icon: `https://framerusercontent.com/images/3M0CPgfOsuyRxuRs37KkTOTrUM.png`

**Color palette:**
- Dark background: `rgb(29, 25, 26)`
- Cream/gold accent: `rgb(241, 229, 198)` / `#F1E5C6`
- White: `#FFFFFF`
- Light text on dark: `rgba(255, 255, 255, 0.77)`, `rgba(255, 255, 255, 0.6)`, `rgba(255, 255, 255, 0.5)`, `rgba(255, 255, 255, 0.3)`
- Off-white: `#F5F0E8`

**Typography (all Manrope):**
- Brand name in header: 17px Regular, white, lineHeight 20
- Rotated sidebar text: 14px, Medium (name) and Regular (role), letterSpacing 1.2, half-transparent white
- Quote text: 20px Regular, lineHeight 27, `rgba(255,255,255,0.77)`
- Event card title: 36px Medium, letterSpacing -0.5, color `#1a1a1a`
- Event card date: 17px Regular, `#888888`
- Hero headline (Screen 2): 52px Light, lineHeight 52, letterSpacing -2.5, cream color `#F1E5C6`
- Hero subtext: 21px Regular, lineHeight 27, white 60% opacity
- CTA button text: 21px Medium, dark color
- Section title "Upcoming" (Screen 3): 38px Regular, letterSpacing -0.8, white
- Event list titles: 21px Regular, lineHeight 26
- Event times: 17px Regular, `#999999`
- Date card day: 34px Medium
- Date card month: 18px Regular
- Menu links: 36px Light, letterSpacing -0.8, white

**Animations:**
- Each iPhone frame fades in with `FadeIn.duration(600)` using react-native-reanimated
- All text uses a typewriter effect (characters appear one by one) with configurable delay and speed
- Menu overlay fades in with `FadeIn.duration(200)`

**Screen 1 - Testimonial:**
- Dark background (`rgb(29, 25, 26)`)
- Header: logo (46x46) + "Christ Family Church" text on left, hamburger menu (3 lines, 2 long + 1 short) on right, paddingTop 48, paddingHorizontal 19
- Rotated text on left side: "Anna Miller" / "Community Member" rotated -90deg, positioned at left: -44, top: 115, translateX: -60
- Portrait photo: positioned at top: 120, left: 125, size 240x300
- Quote icon (28x24) at left: 19, top: 395
- Quote text at left: 19, top: 440, width: 336: "We want to be a family where people can connect and benefit from friendships in Christ."
- White card at bottom (height 245): contains "Sunday Worship Service" (36px), "Dec 7th, 10-11:30am", "Learn more" with arrow, star icon (48x48) bottom-right

**Screen 2 - Hero/Landing:**
- Full hero image covering top 472px with dark overlay (15% black)
- Same header as Screen 1
- Avatars image (149x53) at left: 19, top: 442
- Headline at top: 520: "Take a step toward the light"
- Subtext: "Discover faith, hope, and a home for your soul"
- Cream CTA button at bottom (height 52, bottom: 32): "Join us" with arrow icon

**Screen 3 - Sermons/Events:**
- Hero image at top: covers 345px height (width: 415, offset left: -20), with 30% black overlay
- Play button (65x65 circle, cream background) centered at top: 175
- White body section starting at top: 343
- Dark band inside body (height 220, `rgb(29, 25, 26)`) behind "Upcoming" title
- Events list with date cards (68x90):
  - 14 Dec: "Luke 1 | A Story From Zechariah" - 6:30 - 8:00 pm (white date card)
  - 21 Dec: "Romans 15 | Living For Christ Alone" - 8:30 - 10:00 am (cream date card)
  - 28 Dec: "Romans 9 | The Sovereignty Of God" - 5:30 - 7:00 pm (cream date card)
  - 4 Jan: "John 3 | Born Again" (partial/faded at 50% opacity, cream date card)
- Gap between event rows: 22px

**Menu overlay (shared across all screens):**
- Full-screen dark overlay (`rgb(29, 25, 26)`), z-index 100
- Header with logo + brand name and X close button (two rotated lines)
- Navigation links: Home, About, Events, Sermons, Contact (36px Light)
- Gap between links: 22px
- Cream CTA "Join us" button at bottom (bottom: 36)
- Hamburger button on each screen opens the menu; close button dismisses it

## Fine Jewelry Shop — Ecommerce [apps/fine-jewelry-shop]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/CleanShot%202026-07-13%20at%2007.51.09.png
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/fine-jewelry-shop.png

**Create a luxury jewelry mobile app concept displayed inside a realistic iPhone mockup. The design is a "Blue Nile" branded mobile screen with a warm orange hero section and a product card below. Use React, Tailwind CSS, and Lucide React icons. The entire phone is centered on a warm off-white page background.**

---

### Page & Phone Frame

- **Page background:** `#f5f5f0` (warm off-white), full viewport height, centered with flexbox and `p-8` padding.
- **Phone mockup:** 375x812px, `rounded-[55px]`, black frame with 12px padding. Drop shadow: `0 50px 100px -20px rgba(0,0,0,0.5), 0 30px 60px -10px rgba(0,0,0,0.4)`. A subtle white/10 border highlight on the outer frame. Side buttons on left (silent switch at top-120px, two volume buttons at 170px and 235px) and right (power button at 185px). Screen area has `rounded-[43px]` with overflow hidden.
- **Dynamic Island:** Centered pill at top-12px, 126x36px, black rounded-full, containing a small 10x10px dark circle (camera lens) with a border of `#2a2a3e`.

---

### Fonts (Critical)

- **Body font:** "Test Founders Grotesk Light" loaded from: `https://db.onlinewebfonts.com/c/7973d1644865c7217230fea96daae6fe?family=Test+Founders+Grotesk+Light`
- **Logo font:** "NimbusSanExt" loaded from: `https://db.onlinewebfonts.com/c/12487acadbf8efa35235fe8d339411ec?family=NimbusSanExt`

---

### Color Palette

- **Brand orange (hero background):** `#E96B00`
- **Brand peach (glow):** `#F6BB7E`
- **Brand dark:** `#0B2122`

---

### Hero Section (Top ~73% of screen)

- **Background:** Solid `#E96B00` (brand-orange), full width, `flex-1` to fill available space, `overflow-hidden`, positioned relative.
- **Gradient glow:** A 600x600px circle positioned at `-left-24`, `top-[33%]`, color `brand-peach/40`, blurred 200px.
- **Vertical grid lines:** Two vertical 1px white lines at `left-1/3` and `left-2/3`, opacity 16% (`bg-white/[0.16]`), spanning full height.
- **Navigation bar:** Absolutely positioned at top, `px-4 pt-14`, z-index 70, flexbox space-between.
  - **Logo (left):** White bold text "Blue" on first line, "Nile" on second line (`<br/>`), font-family `NimbusSanExt, sans-serif`, `text-xl font-bold leading-none`.
  - **Menu button (right):** 40x40px flex center, white. Shows a Lucide `Menu` icon (5x5) by default, crossfades/rotates to Lucide `X` icon when menu is open. Transition: 300ms cubic-bezier(0.77,0,0.18,1), with rotation and scale effects.

- **Hero image (center):** Absolutely positioned at `-bottom-10`, centered horizontally with `left-1/2 -translate-x-1/2`, z-20, width `132%` (`max-w-none`).
  - **Image URL:** `https://soft-zoom-63098134.figma.site/_assets/v11/9028130a3e77802079d3a2e663b85ee12d365b61.png`
  - **Behind the image:** An 80% wide, 60% tall rounded-full glow using `brand-peach/40` with `blur-[80px]`, centered via absolute + translate.

- **Awards section (bottom-left and bottom-right):** Absolutely positioned at `left-4 right-4 bottom-4`, z-30, flex space-between, vertically centered.
  - **Left side:** `[12+]` displayed as:
    - `[` in white, `text-5xl font-light`
    - `12` in white, `text-5xl font-bold`
    - `+` in white, `text-lg font-bold`, positioned slightly up with `relative -top-1`
    - `]` in white, `text-5xl font-light`
  - **Right side:** White uppercase text, `text-base`, `tracking-wide leading-snug`, reading "Awards / Celebrate / Innovation" (each word on its own line via `<br />`).

---

### Mobile Menu Overlay

- **Backdrop:** Full-screen absolute overlay, z-60, with `bg-black/60 backdrop-blur-sm`. Fades in/out over 500ms. Clicking it closes the menu.
- **Drawer:** Slides in from the left, 80% width (max 280px), background `brand-orange`, with `shadow-2xl`. Transition: translateX, 500ms, cubic-bezier(0.77,0,0.18,1).
- **Menu items:** List of links ["Search", "Catalog", "About", "Profile", "Favorites"], white text, `text-2xl font-medium`, each with `py-2.5` and a bottom border of `white/10`. Each item staggers in with 50ms delay intervals (starting at 80ms) via opacity and translateX animation. Hover: slides right 2px and dims to 80% opacity.

---

### Product Card (Bottom ~27% of screen)

- **Container:** White background, `p-3`, fixed height 220px, flex column justify-between.
- **Product info (top-left):**
  - Title: "Coco Crush ring", black, `text-lg font-medium leading-tight`
  - Subtitle: "18K yellow", `text-black/60 text-xs mt-1`
- **Product image (centered):** Absolutely positioned center (50%/50% translate), width 70%, object-contain.
  - **Image URL:** `https://soft-zoom-63098134.figma.site/_assets/v11/6297b1b8b8a1c0720cbd098274da6619ad35b486.png`
- **Price info (bottom-left):**
  - Label: "From", `text-brand-dark/64 text-xs`
  - Price: "$25,550", `text-brand-dark text-lg font-medium`
- **Arrow button (bottom-right corner):** 72x68px black box, absolutely positioned at `bottom-0 right-0`, containing a white Lucide `ArrowUpRight` icon (4x4). Hover: slightly lighter black (`bg-black/90`) with transition.

---

### Animations (Staggered on mount)

All elements animate in on mount with a 100ms initial delay:

1. **Translate stagger:** Elements fade in (opacity 0 to 1) and slide up (translateY 20px to 0) with `0.7s cubic-bezier(0.16, 1, 0.3, 1)` easing, each subsequent element delayed by 120ms.
2. **Scale stagger:** Elements scale from 0.92 to 1 with opacity fade, `0.8s cubic-bezier(0.16, 1, 0.3, 1)`.
3. **Fade stagger:** Pure opacity fade, `0.8s cubic-bezier(0.16, 1, 0.3, 1)`.

Stagger order (index): grid lines (0), logo (1), menu button (2), hero image fade (3), awards left (4), awards right (5), product card container (6), product text top (7), product image (8), price section (9), arrow button scale (10).

---

### CSS Utilities

- `.scrollbar-hide` class to hide scrollbars on the screen content area.
- Global reset: `* { margin: 0; padding: 0; box-sizing: border-box; }`
- Body: font-family set to "Test Founders Grotesk Light", with `-webkit-font-smoothing: antialiased`.

---

### Tech Stack

- React 18 + TypeScript
- Vite
- Tailwind CSS 3.4
- Lucide React (for Menu, X, ArrowUpRight icons)
- No additional UI libraries

## Gear Shop — Ecommerce App [apps/gear-shop]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/CleanShot%202026-07-09%20at%2009.34.47.png
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/gear-shop.png

Build a static Vite HTML page that displays 3 iPhone 15 Pro mockup screens side by side (stacking vertically on screens under 1000px). The design is an audio/headphones e-commerce app with a warm coral/peach color palette. Use only vanilla HTML and inline CSS -- no frameworks.

---

### LAYOUT & BACKGROUND

- Page background: `#F6E2DE` with a fixed SVG overlay showing concentric circles (coral `#e2574c` stroked circles at r=260 opacity 0.5, r=180 opacity 0.7, white `#ffffff` circle at r=100, filled coral dot at r=26).
- 3 screens sit centered in a flex row with `gap: 28px`, wrapped in `zoom: 0.744`. On `max-width: 1000px`, stack vertically.
- Each screen is `390px x 844px`, `border-radius: 44px`, `box-shadow: 0 30px 60px rgba(190, 120, 110, 0.28)`, `overflow: hidden`.

---

### FONT

- Custom font "Substance" loaded via `@font-face` with weights 100-900 (thin, extralight, light, regular, medium, bold, extrabold, black) from `.otf` files in `assets/fonts/`.
- Fallback stack: `-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, sans-serif`.
- Prices use system font: `-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif`.

---

### iOS CHROME (on every screen)

- **Dynamic Island**: absolute positioned, `top: 11px`, centered horizontally, `120px x 35px`, `border-radius: 22px`, solid black.
- **Status bar**: absolute top, flex row with 140px gap, centered. Left: time "9:41" in SF Pro/system font, `font-weight: 600`, `16px`. Right: signal bars SVG (4 rects), WiFi SVG, battery SVG. Screen 1 uses white icons; Screens 2-3 use black icons.
- **Home indicator**: absolute bottom, centered bar `134px x 5px`, `border-radius: 100px`. White `rgba(255,255,255,0.75)` on Screen 1, dark `rgba(0,0,0,0.28)` on Screens 2-3.

---

### SCREEN 1: HERO

- Background: `#F3B7AE`
- Full-bleed cover image: `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image-4.d020e935.png` with Ken Burns animation (scale 1.16 to 1 over 16s) + fade in over 1.4s.
- **"20 new arrivals" badge**: absolute `top: 62px; left: 24px`, white pill (`border-radius: 999px`), `padding: 12px 20px`, `font-size: 13px`, `font-weight: 500`, with fire emoji. Shadow: `0 8px 20px rgba(160, 80, 70, 0.18)`. Animates in from left.
- **Play button**: absolute `top: 268px; right: 19px`, `120x124px` SVG with:
  - Two concentric stroke circles (`#FFE8DD`, r=64.4 and r=44.4)
  - Radial gradient glow ring (r=40)
  - Solid radial gradient button (r=30.5) going from `#FCB6AD` center to `#E5665B` edge
  - White play triangle with glow filter (`feGaussianBlur stdDeviation="3.2"`)
  - Pop animation (scale 0.8 to 1 with spring easing `cubic-bezier(0.34, 1.5, 0.5, 1)`)
- **Headline**: absolute `left: 28px; bottom: 150px`
  - Line 1: "- old tracks -" (using centered dots), `font-size: 32px`, `font-weight: 200`, color `#d6837b`, `-webkit-text-stroke: 1.5px #d6837b`
  - Line 2: "With new sounds", `font-size: 75px`, `line-height: 0.95`, `font-weight: 500`, white, `text-shadow: 0 2px 18px rgba(120, 40, 30, 0.18)`

---

### SCREEN 2: NEW ARRIVALS

- Background: `#fdbdb4`
- **Top bar** (below status bar, `padding: 62px 24px 20px 24px`):
  - Search pill: white, `border-radius: 999px`, `height: 52px`, with search icon SVG (`stroke-width: 2.5`) + "search" text
  - Avatar: `52px` circle, image from `https://images.unsplash.com/photo-1516726817505-f5ed825624d8?w=120&h=120&fit=crop&crop=faces`
- **Content sheet**: `background: #FBE7E2`, `border-radius: 60px 60px 0 0`, `margin-top: 26px`, `padding: 56px 20px 0 20px`
  - "New arrivals" heading: `font-size: 38px`, `font-weight: 400`, `-webkit-text-stroke: 0.6px #111111`, line break between "New" and "arrivals"
  - Hamburger menu icon SVG (3 lines of decreasing length)
  - **Filter pills row**: "All" (filled black `#111111`, white text), "Headphones" and "Speakers" (transparent with `1.5px solid #111111` border). All `font-size: 16px`, `padding: 15px 28-30px`, `border-radius: 999px`.
  - **Product grid**: 2-column CSS grid, `gap: 14px`, each card is `background: #f2cfcb`, `border-radius: 26px`, centered content:
    - Airpods Pro: `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image-3.82d4333a.png` -- $499.00
    - Speakers: `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image-2.04df17d2.png` -- $359.00
    - Headphones: `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image.a862e80a.png` -- $650.00
    - Earphones: `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image-1.3ff69aba.png` -- $60.00
    - Product names: `17px`, `font-weight: 500`, `#111111`. Prices: `14px`, `font-weight: 600`, `#7c6a66`, system font.
- **Floating cart pill**: absolute centered bottom `26px`, black `#111111` pill, "$1080.00" in white `13px font-weight: 300` + white circle with shopping bag SVG icon + small red dot indicator (`#e2574c`, 5px). Shadow: `0 12px 28px rgba(60, 20, 15, 0.35)`.

---

### SCREEN 3: PRODUCT DETAIL

- Background: `#fdbdb4`
- **Top bar**: flex space-between, `padding: 62px 28px 0 28px`. Left: 3 white vertical dots SVG. Right: white hamburger icon SVG.
- **Product image**: centered, `330x330px`, `https://order-twine-70493179.figma.site/_components/v2/1fb0bd10fd40f9a9e279e8076f3762dd0f7d9889/image.a862e80a.png`, scale-in animation.
- **Color swatches row**: 4 items, each `72x72px`, `border-radius: 22px`, `background: #FBDFD9`, containing a `42px` circle with `11px` colored border:
  - Navy blue: `#1e3a6e`
  - Green: `#9fd6a0`
  - Gray: `#c9c2c0`
  - Coral: `#ef8177`
- **Bottom sheet**: `background: #FBE7E2`, `border-radius: 60px 60px 0 0`, `margin-top: 28px`, `padding: 34px 26px 44px 26px`
  - "Apple Airpods" heading: `font-size: 45px`, `font-weight: 400`, `-webkit-text-stroke: 0.5px #111111`
  - Description: "A mesh textile wraps the ear cushions to provide pillow-like softness", `font-size: 18px`, `line-height: 1.5`, color `#b0a6a1`
  - Footer row: "$499.00" (`22px`, `font-weight: 500`, system font) + "Add to cart" pill (`background: #F6C6BE`, `border-radius: 999px`) with shopping bag SVG icon

---

### ANIMATIONS (all use `animation-fill-mode: backwards`)

- **dcRise**: translateY(28px) + opacity 0 to normal, 0.85s, `cubic-bezier(0.22, 1, 0.36, 1)`
- **dcRiseSm**: translateY(16px) + opacity, same easing
- **dcFade**: opacity 0 to 1
- **dcInLeft**: translateX(-18px) + opacity
- **dcPop**: scale(0.8) + opacity, spring easing `cubic-bezier(0.34, 1.5, 0.5, 1)`
- **dcScale**: scale(0.9) + opacity, 1s duration
- **dcRiseC**: translateX(-50%) translateY(18px) to translateX(-50%) translateY(0) (for centered absolute elements)
- **dcKen**: scale(1.16) to scale(1), 16s ease-out
- **Stagger delays**: `.dc-d1` through `.dc-d10` (0.08s to 1.16s increments)
- **Stagger children**: `.dc-stagger > *` uses dcRiseSm starting at 0.38s with 0.12s increments; `.dc-stagger2 > *` starts at 0.64s
- Respect `prefers-reduced-motion: reduce` by disabling all animations.

## Pet Products — Ecommerce App [apps/pet-products]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/doggy.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/pet-products.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>CozyPaws - Everything Your Pets Love</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=DM+Serif+Display&family=Inter:wght@400;500;600&family=Poppins:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: 'Inter', sans-serif; background: linear-gradient(to bottom right, #e2e8f0, #f8fafc, #e2e8f0); min-height: 100vh; }

    .page { display: flex; justify-content: center; align-items: center; min-height: 100vh; padding: 20px; overflow-x: hidden; }
    .phones { display: flex; flex-direction: column; align-items: center; gap: 32px; zoom: 0.85; }
    @media (min-width: 640px) { .phones { zoom: 0.95; } }
    @media (min-width: 1024px) { .phones { flex-direction: row; gap: 50px; zoom: 1; } }

    /* Phone shell */
    .phone { position: relative; width: 390px; flex-shrink: 0; }
    .phone-frame { position: relative; border-radius: 60px; border: 14px solid #1a1a1a; background: #1a1a1a; box-shadow: 0 50px 100px -20px rgba(0,0,0,0.4), 0 30px 60px -15px rgba(0,0,0,0.3), inset 0 -2px 6px rgba(255,255,255,0.05); }
    .notch { position: absolute; top: 12px; left: 50%; transform: translateX(-50%); width: 110px; height: 32px; background: #000; border-radius: 9999px; z-index: 60; }
    .screen { position: relative; border-radius: 46px; overflow: hidden; background: #f0f9f1; aspect-ratio: 9 / 19.5; }
    .screen-inner { position: absolute; inset: 0; display: flex; flex-direction: column; }
    .phone-shadow { position: absolute; bottom: -16px; left: 10%; right: 10%; height: 32px; background: rgba(0,0,0,0.1); border-radius: 50%; filter: blur(12px); }

    /* Header */
    .header { position: relative; z-index: 30; padding: 48px 20px 8px; animation: fadeIn 0.6s ease-out both; animation-delay: 300ms; }
    .header-inner { display: flex; align-items: center; justify-content: space-between; }
    .menu-btn { width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; background: none; border: none; cursor: pointer; animation: slideInLeft 0.8s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 400ms; }
    .header-right { display: flex; align-items: center; gap: 10px; animation: slideInRight 0.8s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 400ms; }
    .star-btn { position: relative; width: 32px; height: 32px; border-radius: 50%; background: #E86A10; display: flex; align-items: center; justify-content: center; border: none; cursor: pointer; }
    .badge { position: absolute; top: -4px; right: -4px; width: 16px; height: 16px; border-radius: 50%; background: #E86A10; border: 2px solid #f0f9f1; color: white; font-size: 8px; font-weight: 700; display: flex; align-items: center; justify-content: center; }
    .cart-btn { position: relative; display: flex; align-items: center; gap: 4px; background: none; border: none; cursor: pointer; }
    .cart-circle { position: relative; width: 32px; height: 32px; border-radius: 50%; border: 1px solid #d1d5db; display: flex; align-items: center; justify-content: center; background: white; }
    .cart-price { font-size: 11px; font-weight: 600; color: #1a3d1a; }
    .avatar { width: 32px; height: 32px; border-radius: 50%; overflow: hidden; border: 2px solid #4CAF50; }
    .avatar img { width: 100%; height: 100%; object-fit: cover; }

    /* Left phone */
    .left-content { flex: 1; display: flex; flex-direction: column; animation: slideUp 0.9s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 500ms; }
    .dog-section { position: relative; flex: 1.2; overflow: hidden; }
    .dog-section img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
    .stats-overlay { position: absolute; bottom: 0; left: 0; right: 0; padding: 20px; display: flex; flex-direction: column; align-items: center; text-align: center; animation: fadeIn 0.6s ease-out both; animation-delay: 800ms; }
    .stats-number { font-size: 40px; font-weight: 700; color: #1a3d1a; }
    .stats-avatars { display: flex; align-items: center; margin-left: -8px; }
    .stats-avatar { width: 40px; height: 40px; border-radius: 50%; border: 2px solid white; overflow: hidden; margin-left: -8px; }
    .stats-avatar img { width: 100%; height: 100%; object-fit: cover; }
    .stats-avatar-plus { width: 40px; height: 40px; border-radius: 50%; background: #1a3d1a; border: 2px solid white; display: flex; align-items: center; justify-content: center; margin-left: -8px; }
    .stats-text { font-size: 14px; color: #1a3d1a; line-height: 1.4; font-weight: 500; }
    .video-section { position: relative; flex: 1.3; overflow: hidden; }
    .video-section video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
    .play-btn { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
    .play-btn button { width: 40px; height: 40px; border-radius: 50%; background: #1a3d1a; border: none; display: flex; align-items: center; justify-content: center; box-shadow: 0 4px 12px rgba(0,0,0,0.3); cursor: pointer; }

    /* Center phone */
    .center-content { flex: 1; position: relative; overflow: hidden; display: flex; align-items: center; justify-content: center; }
    .hero-heading { position: absolute; top: calc(10% + 45px); left: 0; right: 0; z-index: 10; color: #1a3d1a; font-size: 46px; line-height: 1.1; letter-spacing: -0.025em; text-align: center; font-family: 'Poppins', sans-serif; font-weight: 400; animation: textReveal 1s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 500ms; }
    .hero-dog { position: absolute; bottom: 0; left: 50%; transform: translateX(-50%); min-width: 132%; width: 132%; height: auto; max-width: none; z-index: 20; animation: photoReveal 1.1s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 700ms; }
    .hero-cta { position: absolute; bottom: 0; left: 0; right: 0; z-index: 20; padding: 0 20px 25px; text-align: center; animation: slideUp 0.9s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 900ms; }
    .hero-cta h2 { color: white; font-size: 22px; font-weight: 700; line-height: 1.2; margin-bottom: 12px; text-shadow: 0 2px 6px rgba(0,0,0,0.5); }
    .explore-btn { display: inline-flex; align-items: center; gap: 8px; background: #E86A10; color: white; font-weight: 600; padding: 10px 20px; border-radius: 9999px; font-size: 12px; border: none; cursor: pointer; }
    .explore-btn span { width: 24px; height: 24px; border-radius: 50%; background: rgba(255,255,255,0.2); display: flex; align-items: center; justify-content: center; }

    /* Right phone */
    .right-content { flex: 1; display: flex; flex-direction: column; animation: slideUp 0.9s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 500ms; }
    .cat-section { position: relative; flex: 1.2; overflow: hidden; }
    .cat-section img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: top; }
    .cat-stats { position: absolute; bottom: 0; left: 0; right: 0; padding: 20px; display: flex; flex-direction: column; align-items: center; text-align: center; animation: fadeIn 0.6s ease-out both; animation-delay: 800ms; }
    .cat-rating { display: flex; align-items: center; gap: 4px; margin-bottom: 8px; }
    .cat-rating span { font-size: 40px; font-weight: 700; color: #1a3d1a; }
    .arrivals-section { flex: 1.3; padding: 12px 20px; background: #A8E7B0; overflow: hidden; }
    .arrivals-title { font-size: 30px; font-weight: 500; color: #1a3d1a; text-align: center; margin-bottom: 12px; }
    .products-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 5px; }
    .product-card { position: relative; background: white; border-radius: 16px; padding: 20px; display: flex; flex-direction: column; align-items: center; }
    .product-card:nth-child(1) { animation: cardPopIn 0.6s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 900ms; }
    .product-card:nth-child(2) { animation: cardPopIn 0.6s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 1000ms; }
    .product-card:nth-child(3) { animation: cardPopIn 0.6s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 1100ms; }
    .product-card:nth-child(4) { animation: cardPopIn 0.6s cubic-bezier(0.16, 1, 0.3, 1) both; animation-delay: 1200ms; }
    .product-plus { position: absolute; top: 8px; right: 8px; background: none; border: none; cursor: pointer; }
    .product-img { width: 100%; aspect-ratio: 6/4; border-radius: 12px; overflow: hidden; margin-bottom: 8px; }
    .product-img img { width: 100%; height: 100%; object-fit: cover; }
    .product-name { font-size: 14px; color: #6b7280; font-weight: 500; }
    .product-price { font-size: 26px; font-weight: 700; color: #1a3d1a; }

    /* Keyframes */
    @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
    @keyframes slideUp { from { opacity: 0; transform: translateY(60px); } to { opacity: 1; transform: translateY(0); } }
    @keyframes slideInLeft { from { opacity: 0; transform: translateX(-40px); } to { opacity: 1; transform: translateX(0); } }
    @keyframes slideInRight { from { opacity: 0; transform: translateX(40px); } to { opacity: 1; transform: translateX(0); } }
    @keyframes textReveal { from { opacity: 0; transform: translateY(40px) skewY(3deg); filter: blur(4px); } to { opacity: 1; transform: translateY(0) skewY(0deg); filter: blur(0px); } }
    @keyframes photoReveal { from { opacity: 0; transform: translateX(-50%) translateY(80px) scale(1.02); } to { opacity: 1; transform: translateX(-50%) translateY(0) scale(1); } }
    @keyframes cardPopIn { from { opacity: 0; transform: translateY(20px) scale(0.9); } to { opacity: 1; transform: translateY(0) scale(1); } }

    /* SVG icons inline */
    .icon { display: inline-block; vertical-align: middle; }
  </style>
</head>
<body>
  <div class="page">
    <div class="phones">

      <!-- LEFT PHONE -->
      <div class="phone">
        <div class="phone-frame">
          <div class="notch"></div>
          <div class="screen">
            <div class="screen-inner">
              <div class="header">
                <div class="header-inner">
                  <button class="menu-btn">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/></svg>
                  </button>
                  <div class="header-right">
                    <button class="star-btn">
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="white" stroke="white" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
                      <span class="badge">4</span>
                    </button>
                    <button class="cart-btn">
                      <div class="cart-circle">
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="21" r="1"/><circle cx="19" cy="21" r="1"/><path d="M2.05 2.05h2l2.66 12.42a2 2 0 0 0 2 1.58h9.78a2 2 0 0 0 1.95-1.57l1.65-7.43H5.12"/></svg>
                        <span class="badge">1</span>
                      </div>
                      <span class="cart-price">$21</span>
                    </button>
                    <div class="avatar">
                      <img src="https://polo-pecan-73837341.figma.site/_assets/v11/e62173d41f91350a59628e8a9a55ae078a886fb9.png?w=128" alt="Avatar" />
                    </div>
                  </div>
                </div>
              </div>

              <div class="left-content">
                <div class="dog-section">
                  <img src="https://polo-pecan-73837341.figma.site/_assets/v11/8d44b25186ef45a5789c74668fb781cea4e1ff49.png" alt="Dachshund" />
                  <div class="stats-overlay">
                    <div style="display:flex;align-items:center;gap:12px;margin-bottom:8px;">
                      <span class="stats-number">98K+</span>
                      <div class="stats-avatars">
                        <div class="stats-avatar"><img src="https://polo-pecan-73837341.figma.site/_assets/v11/e62173d41f91350a59628e8a9a55ae078a886fb9.png?w=128" alt="" /></div>
                        <div class="stats-avatar-plus">
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg>
                        </div>
                      </div>
                    </div>
                    <p class="stats-text">Happy Clients and Their Pets<br>Who Love Our Products</p>
                  </div>
                </div>
                <div class="video-section">
                  <video src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_160640_dc6d2a50-121e-45b0-a84f-331faa58d804.mp4" autoplay muted loop playsinline></video>
                  <div class="play-btn">
                    <button>
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="white" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="6 3 20 12 6 21 6 3"/></svg>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="phone-shadow"></div>
      </div>

      <!-- CENTER PHONE -->
      <div class="phone">
        <div class="phone-frame">
          <div class="notch"></div>
          <div class="screen">
            <div class="screen-inner">
              <div class="header" style="animation-delay:200ms;">
                <div class="header-inner">
                  <button class="menu-btn" style="animation-delay:300ms;">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/></svg>
                  </button>
                  <div class="header-right" style="animation-delay:300ms;">
                    <button class="star-btn">
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="white" stroke="white" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
                    </button>
                    <button class="cart-btn">
                      <div class="cart-circle">
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="21" r="1"/><circle cx="19" cy="21" r="1"/><path d="M2.05 2.05h2l2.66 12.42a2 2 0 0 0 2 1.58h9.78a2 2 0 0 0 1.95-1.57l1.65-7.43H5.12"/></svg>
                        <span class="badge">1</span>
                      </div>
                      <span class="cart-price">$21</span>
                    </button>
                    <div class="avatar">
                      <img src="https://polo-pecan-73837341.figma.site/_assets/v11/e62173d41f91350a59628e8a9a55ae078a886fb9.png?w=128" alt="Avatar" />
                    </div>
                  </div>
                </div>
              </div>

              <div class="center-content">
                <h1 class="hero-heading">Everything<br>Your Pets Love</h1>
                <img class="hero-dog" src="https://polo-pecan-73837341.figma.site/_assets/v11/96745c4e72ad5c5208e53a885df797fd82cd854a.png?h=1024" alt="Golden Retriever" />
                <div class="hero-cta">
                  <h2>Best Products<br>for Your Pet</h2>
                  <button class="explore-btn">
                    Explore Products
                    <span>
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" x2="19" y1="12" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                    </span>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="phone-shadow"></div>
      </div>

      <!-- RIGHT PHONE -->
      <div class="phone">
        <div class="phone-frame">
          <div class="notch"></div>
          <div class="screen">
            <div class="screen-inner">
              <div class="header">
                <div class="header-inner">
                  <button class="menu-btn">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/></svg>
                  </button>
                  <div class="header-right">
                    <button class="star-btn">
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="white" stroke="white" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
                    </button>
                    <button class="cart-btn">
                      <div class="cart-circle">
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="21" r="1"/><circle cx="19" cy="21" r="1"/><path d="M2.05 2.05h2l2.66 12.42a2 2 0 0 0 2 1.58h9.78a2 2 0 0 0 1.95-1.57l1.65-7.43H5.12"/></svg>
                        <span class="badge">1</span>
                      </div>
                      <span class="cart-price">$21</span>
                    </button>
                    <div class="avatar">
                      <img src="https://polo-pecan-73837341.figma.site/_assets/v11/e62173d41f91350a59628e8a9a55ae078a886fb9.png?w=128" alt="Avatar" />
                    </div>
                  </div>
                </div>
              </div>

              <div class="right-content">
                <div class="cat-section">
                  <img src="https://polo-pecan-73837341.figma.site/_assets/v11/81bd2e7a66b58f3d8f3ad78fd1ebf01af8dfdee1.png" alt="Cat" />
                  <div class="cat-stats">
                    <div class="cat-rating">
                      <span>4.6</span>
                      <svg width="22" height="22" viewBox="0 0 24 24" fill="#E86A10" stroke="#E86A10" stroke-width="2"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
                    </div>
                    <p class="stats-text">Based on Reviews from Happy<br>Pet Owners Worldwide</p>
                  </div>
                </div>
                <div class="arrivals-section">
                  <h3 class="arrivals-title">New Arrivals</h3>
                  <div class="products-grid">
                    <div class="product-card">
                      <button class="product-plus"><svg width="23" height="23" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg></button>
                      <div class="product-img"><img src="https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_172706_08d34c84-fa47-4744-835d-aeb8574e894b.png&w=1280&q=85" alt="Sunset Cat Bowl" /></div>
                      <span class="product-name">Sunset Cat Bowl</span>
                      <span class="product-price">$19.99</span>
                    </div>
                    <div class="product-card">
                      <button class="product-plus"><svg width="23" height="23" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg></button>
                      <div class="product-img"><img src="https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_171722_0831d998-50be-461c-89e1-1164db805d12.png&w=1280&q=85" alt="Mint Cat Bowl" /></div>
                      <span class="product-name">Mint Cat Bowl</span>
                      <span class="product-price">$29.99</span>
                    </div>
                    <div class="product-card">
                      <button class="product-plus"><svg width="23" height="23" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg></button>
                      <div class="product-img"><img src="https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_171713_10dca337-a1bd-44c8-be9f-44ef5b5efb5c.png&w=1280&q=85" alt="Cat Toy" /></div>
                      <span class="product-name">Cat Toy</span>
                      <span class="product-price">$12.99</span>
                    </div>
                    <div class="product-card">
                      <button class="product-plus"><svg width="23" height="23" viewBox="0 0 24 24" fill="none" stroke="#1a3d1a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg></button>
                      <div class="product-img"><img src="https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_175012_ec38ea5f-e56d-4158-9970-df6ea6f4641b.png&w=1280&q=85" alt="Cat Bed" /></div>
                      <span class="product-name">Cat Bed</span>
                      <span class="product-price">$34.99</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="phone-shadow"></div>
      </div>

    </div>
  </div>
</body>
</html>

## LearnHub — Education [apps/skills-lea]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/linesbrightArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/skills-lea.mp4

Build a mobile app showcase with 3 iPhone mockups displayed side-by-side (stacking vertically on mobile) on a dark background. Use React + Vite + TypeScript + Tailwind CSS + lucide-react for icons. Use the **Space Grotesk** font from Google Fonts (weights 300-700) as the default sans-serif font via Tailwind config.

---

### Global Setup

- **Font**: Space Grotesk (Google Fonts: `https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@300;400;500;600;700&display=swap`)
- **Tailwind config**: Override `fontFamily.sans` with `['"Space Grotesk"', 'sans-serif']`
- **Page background**: `#0a0a0f`
- **Body CSS**: `font-family: 'Inter', sans-serif` (overridden by Tailwind's Space Grotesk)
- **Root**: `width: 100%; height: 100vh`
- **Hide scrollbars** with `.no-scrollbar` utility (webkit + ms + firefox)

---

### PhoneFrame Component

A reusable wrapper for each phone screen:
- Outer container: `w-[320px] h-[650px]` (sm: `340x700`, md: `370x760`), `shrink-0`
- Inner frame: `rounded-[50px]`, `border-[6px] border-zinc-700/80`, `bg-black`, `overflow-hidden`
- Shadow: `box-shadow: 0 0 60px rgba(0,0,0,0.8), 0 0 120px rgba(80,50,120,0.3)`
- **Dynamic Island**: Absolute positioned pill at top center - `w-[100px] h-[28px] bg-black rounded-full`, `top-[12px]`, `z-30`
- **Home Indicator**: Absolute at bottom center - `w-[120px] h-[4px] bg-white/30 rounded-full`, `bottom-2`, `z-20`

---

### Layout Container

```
div: relative w-full min-h-screen flex items-center justify-center bg-[#0a0a0f] py-10 gap-6 md:gap-10 flex-col md:flex-row px-4
```

---

### Screen 1 - Onboarding

Full-screen background video with text overlay at the bottom.

- **Background video** (autoPlay, muted, playsInline, absolute inset-0 object-cover):
  ```
  https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_072800_64c47fd8-8c4a-431c-ab02-d8c84e461474.mp4
  ```

- **Bottom overlay** (absolute bottom-0 left-0 right-0, px-8 pb-10, z-10):
  - Heading: `text-white text-[42px] leading-[1.15] font-light tracking-tight mb-8`
    - Text: "Learn more &" (line break) "improve **your**" (line break) "**skills.**" (bold words use `font-medium`)
  - Centered circular button: `w-14 h-14 rounded-full bg-white` with ArrowRight icon (w-6 h-6, text-black, strokeWidth 2.5)
    - Hover: `scale-110`, active: `scale-95`, transition 300ms
    - Shadow: `0 0 20px rgba(255,255,255,0.2)`

---

### Screen 2 - Dashboard (Explore)

Scrollable screen with background image, header, heading, filter tabs, and a 2-column card grid.

- **Background image** (absolute inset-0 object-cover):
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_013750_18427eb2-0d19-44fd-a5ab-8ee7c40fa18f.png&w=1280&q=85
  ```

- **Header** (z-10, flex between, px-5 pt-14 pb-4):
  - Left: Avatar (w-11 h-11 rounded-full, gradient border `from-amber-400 to-orange-500`, border-2 border-amber-400/50)
    - Avatar image: `https://images.pexels.com/photos/2379004/pexels-photo-2379004.jpeg?auto=compress&cs=tinysrgb&w=100`
    - Below avatar: "Welcome back" (text-zinc-400 text-xs) + "Adam William" (text-white text-sm font-medium)
  - Right: Search icon button (w-11 h-11 rounded-full bg-white/10, Search icon w-5 h-5 text-white)

- **Heading** (px-5 pt-12 pb-5):
  - "Let's explore" (font-light) + line break + "**new fields**" (font-medium)
  - `text-white text-[42px] leading-[1.2]`

- **Filter Tabs** (horizontal scroll, gap-2, px-5 pb-2, no-scrollbar):
  - "All": `px-5 py-3 rounded-full bg-white text-black text-xs font-medium`
  - Others ("Programming", "Design", "Marketing", "Business", "Finance"): `bg-[#524755] text-white text-xs font-medium`

- **Cards Grid** (grid-cols-2 gap-3 px-6 pb-24 pt-2):
  - 4 cards, each with `rounded-[28px] overflow-hidden`, aspect ratio 1:1 via `paddingBottom: '100%'`
  - Each card has a **video background** (autoPlay, muted, playsInline, crossOrigin="anonymous", absolute inset-0 object-cover):
    - Card 0: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_092940_b96cc608-4646-48fa-b73d-19be2b96f9c9.mp4`
    - Card 1: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_091634_79fb6336-cd01-4002-b9e5-c20c548c6646.mp4`
    - Card 2: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_110013_a8872d8b-6678-48e1-a6ab-db071ac6e5ec.mp4`
    - Card 3: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_100911_5fcfc40c-a4be-4900-aca4-82f09b746d51.mp4`
  - Card overlay (absolute inset-0 flex-col justify-end p-3):
    - Tag pill: colored bg, `text-[9px] text-white px-2 py-0.5 rounded-full font-medium w-fit mb-1`
      - Card 0: "Design" `bg-[#6875CA]`
      - Card 1: "Programming" `bg-[#C6A64F]`
      - Card 2: "Design" `bg-[#65C4C8]`
      - Card 3: "Information" `bg-[#D282AC]`
    - Title: `text-[#1F111D] text-sm font-semibold drop-shadow-lg`
      - Card 0: "UI/UX Design"
      - Card 1: "Advanced .Net"
      - Card 2: "Digital art"
      - Card 3: "Copywriting"
  - Arrow button (absolute top-3 right-3): `w-10 h-10 rounded-full bg-white`, ArrowRight icon `w-4 h-4 text-black -rotate-45 strokeWidth-2.5`

- **Bottom Nav** (absolute bottom-4, centered with -translate-x-1/2, z-20):
  - White pill: `bg-white rounded-full px-2 py-2 flex items-center gap-1`
  - Home button: `w-12 h-12 rounded-full text-zinc-400 hover:text-zinc-600`
  - Settings button: `w-14 h-14 rounded-full bg-black text-white`

---

### Screen 3 - Lesson Schedule

Scrollable screen with background image, back/share buttons, title, interactive calendar, and a bottom card.

- **Background image**: Same as Screen 2 (the higgs.ai URL above)

- **Header** (flex between, px-5 pt-14 pb-4):
  - Left: Back button (ArrowLeft icon, w-11 h-11 rounded-full bg-white/10)
  - Right: Share button (Share2 icon, same style)

- **Title** (px-6 pt-4 pb-6):
  - "**Lesson**" + line break + "**schedule**" (both font-medium)
  - `text-white text-[42px] leading-[1.15] font-light`

- **Calendar Component** (interactive, useState for month/year/selectedDay):
  - Default state: August 2024, day 16 selected
  - Month header: month name + year (text-white text-xl font-semibold) with chevron nav buttons (hover:bg-white/10)
  - Day labels row: `['MON', 'THU', 'WED', 'TUE', 'FRI', 'SAT', 'SUN']` - text-zinc-500 text-[10px] font-medium
  - Days grid (7 columns, gap-y-2):
    - Current month days: text-white, hover:bg-white/10
    - Other month days: text-zinc-600
    - Selected day: `backgroundColor: '#B8C1FF', color: '#1F111D'`, font-bold, scale-110
    - Highlighted days (day 12 current month, day 3 next month): `border: 2px dotted rgba(161,161,170,0.6)`, border-radius 50%
  - Full 6-row grid (42 cells total with prev/next month fill)

- **Bottom Card** (mx-4 mb-6 mt-4, rounded-[28px], overflow-hidden):
  - Background video (absolute inset-0 object-cover, autoPlay muted playsInline):
    ```
    https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_110653_41933aaf-9ec1-423f-851a-043b3407ef44.mp4
    ```
  - Content (relative p-5 pr-16):
    - Time badge: `backdrop-blur-sm text-white text-[11px] font-medium px-3 py-1 rounded-full mb-2`, background `#6276CA`
      - Text: "05:00 - 06:00 PM"
    - Title: `text-[#1F111D] text-lg font-bold leading-tight` - "Components &" + line break + "Variants"
    - Subtitle: `text-[#1F111D]/70 text-xs mt-1` - "Adrian Smith"
  - Arrow button (absolute top-4 right-4): `w-12 h-12 rounded-full bg-white`, ArrowRight icon `w-5 h-5 text-black -rotate-45`

---

### Dependencies

- react, react-dom (^18.3.1)
- lucide-react (^0.344.0)
- tailwindcss (^3.4.1)
- vite + @vitejs/plugin-react
- TypeScript

## Movie Premiere — Entertainment [apps/movie-premiere]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/filmstudioArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/movie-premiere.mp4

Build a mobile movie app UI showcase called "Cineva Mobile UI" using React + TypeScript + Vite + Tailwind CSS + Lucide React icons. The page displays a single centered iPhone mockup (375x780px) on a white background. No routing, no database -- purely a visual UI with interactive card swiping.

---

### TECH STACK

- React 18, TypeScript, Vite, Tailwind CSS 3.4, Lucide React for icons
- Font: Google Fonts "Inter" weights 300-900 applied globally via `* { font-family: 'Inter', sans-serif; }`
- No additional UI libraries

---

### LAYOUT STRUCTURE

**Outer wrapper:** Full viewport (`h-screen w-screen`), `bg-white`, flex centered.

**Phone frame:** Fixed 375x780px, border-radius 52px, overflow hidden. Uses a custom `.phone-frame` class with layered box-shadows to simulate a real device bezel:
```css
.phone-frame {
  border-radius: 52px;
  box-shadow:
    inset 0 0 0 2px rgba(255, 255, 255, 0.08),
    0 0 0 1px rgba(0, 0, 0, 0.6),
    0 0 0 10px #1a1a1e,
    0 0 0 11px rgba(255, 255, 255, 0.06),
    0 0 60px rgba(0, 0, 0, 0.5);
}
```

**Dynamic Island:** Absolute positioned pill at top center, 126x34px, bg-black, rounded-full, z-100.

---

### BACKGROUND

Behind all content inside the phone: the current front card image rendered at full size with `scale-110 blur-[40px] brightness-50`, overlaid with `bg-black/30`.

---

### FOLDER-TAB NAVIGATION (top)

A two-level tab system with staggered fade-up animation (class `animate-stagger-1`, 0.1s delay):

- **Left tab** (outside the black area): "Premieres" - text-[11px], font-medium, px-5 py-2.5, white/40 when inactive, white when active.
- **Main black tab area** with `rounded-tl-[28px] rounded-tr-[28px]`, bg-black, py-4 px-1, containing:
  - A CSS radial-gradient connector div positioned at `-left-7 bottom-0` (28x28px): `radial-gradient(circle at 0% 0%, transparent 28px, #000 28px)` -- creates the smooth curve joining the tab to the content below.
  - "In Theaters" button (left, text-[12px], font-medium)
  - "Upcoming" button (right, ml-auto, with ChevronRight icon size 12)

---

### CONTENT AREA

Below the tabs: `bg-black rounded-tl-[28px]` (pure black, only top-left corner rounded). Contains:

### Date display (animate-stagger-2, 0.2s delay):
- Centered, `text-[58px] font-extrabold leading-none tracking-tight` in white
- Format: "05" in full white, "JUL" in `text-white/80 font-bold`

### Card Stack (animate-stagger-3, 0.35s delay with scale animation):
- Container uses `perspective: 1200px`
- Shows 4 cards stacked, each absolute positioned filling the container with `border-radius: 20px`
- Stack offsets: each card behind gets `translateY(-24px * position)`, `scale(1 - 0.05 * position)`
- Opacity: front=1, second=0.85, third=0.6, fourth=0.4
- Front card has draggable pointer events (swipe down to dismiss)
- SWIPE_THRESHOLD = 80px. Dragging adds `translateY(dragY) rotate(dragY * 0.015deg)` and fades opacity
- On release past threshold: plays `cardDropOut` animation (0.48s) then rotates card order

**Card drop animation:**
```css
@keyframes cardDropOut {
  0% { transform: translateY(var(--drop-start-y)) scale(1) rotate(var(--drop-start-rot)); opacity: var(--drop-start-opacity); }
  60% { opacity: 0.3; }
  100% { transform: translateY(130%) scale(0.8) rotate(8deg); opacity: 0; }
}
```

**Front card overlays:**
- Gradient overlay: `bg-gradient-to-t from-black/40 via-transparent to-transparent`
- Top-left badges (flex column gap-2):
  - Clock icon (12px) + "2h 15m" -- bg-black rounded-full px-3 py-1.5, text-white/90 text-xs
  - Popcorn icon (12px) + "Sci-Fi" -- same styling

---

### BOTTOM NAVIGATION (animate-stagger-4, 0.5s delay with navSlideUp):

Absolute bottom-6, centered. Uses `.liquid-glass` class:
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
```
With a `::before` pseudo-element creating a gradient border effect using mask-composite.

4 nav items: Explore (LayoutGrid), Flicks (Film), Queue (Bookmark), Account (User) -- all size 20. Active item (default: index 1 "Flicks") shows white + label text. Inactive shows white/40. Rounded-full buttons with transition-all duration-300.

---

### STAGGERED ENTRANCE ANIMATIONS

All use `cubic-bezier(0.16, 1, 0.3, 1)` easing with `both` fill-mode:

| Class | Keyframes | Duration | Delay |
|---|---|---|---|
| `.animate-stagger-1` | staggerFadeUp (translateY 20px to 0) | 0.6s | 0.1s |
| `.animate-stagger-2` | staggerFadeUp | 0.6s | 0.2s |
| `.animate-stagger-3` | staggerFadeIn (scale 0.95 to 1) | 0.7s | 0.35s |
| `.animate-stagger-4` | navSlideUp (translate -50%, 20px to -50%, 0) | 0.5s | 0.5s |

---

### IMAGE URLS (6 movie poster cards)

```
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105630_4428f039-9cd3-44a3-bb7f-15c28b0703f2.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105503_6f51b402-7feb-4a64-a154-bce55a4bff52.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105544_9e41e1d6-2da4-458f-99a5-8568241ab76b.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105558_d8ebccec-11f9-4445-8bae-f44e1117ca00.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105613_e3910ee6-0196-48a4-b1e3-2a2c16c721d0.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260705_105621_a97dd98e-cf52-4084-9977-2556ac4fc1fa.png&w=1280&q=85
```

---

### KEY IMPLEMENTATION DETAILS

- Card order managed via `useState<number[]>` initialized as `[0,1,2,3,4,5]`
- On swipe dismiss: shift first element to end of array after animation completes (480ms timeout)
- Pointer capture used on front card for reliable drag tracking
- Only downward drag allowed (`Math.max(0, diff)`)
- CSS custom properties (`--drop-start-y`, `--drop-start-rot`, `--drop-start-opacity`) pass dynamic values into the keyframe animation
- `.stack-card` base transition: `transform 0.5s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1)` -- disabled during active drag on front card
- All cards have `touch-action: none; user-select: none; will-change: transform, opacity`
- Background image changes instantly when card order changes (shows current front card blurred)

## Remit Race — Fintech [apps/remit-race]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/purpleglobapp.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/remit-race.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>NovaPay - One Globe, One Future</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Quicksand:wght@300;400;500;600;700&family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">
  <style>
    @font-face {
      font-family: 'Qanelas-Heavy';
      src: url('https://db.onlinewebfonts.com/t/3010f9da43a41a81d5daa32bd6edebc2.woff2') format('woff2'),
           url('https://db.onlinewebfonts.com/t/3010f9da43a41a81d5daa32bd6edebc2.woff') format('woff');
      font-weight: 900;
      font-style: normal;
      font-display: swap;
    }

    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }

    body {
      margin: 0;
      background: #050410;
      font-family: 'Inter', sans-serif;
      letter-spacing: -0.01em;
      min-height: 100vh;
    }

    .showcase {
      min-height: 100vh;
      display: flex;
      flex-wrap: nowrap;
      align-items: center;
      justify-content: center;
      gap: 40px;
      padding: 24px;
    }

    .phone {
      position: relative;
      zoom: 0.78;
      width: 393px;
      height: 820px;
      overflow: hidden;
      flex: none;
      border-radius: 28px;
      box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6);
      border: 1px solid rgba(148, 145, 182, 0.28);
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .phone[data-screen="competition"],
    .phone[data-screen="about"] {
      background: #0c0a16;
    }

    .phone[data-screen="one-world"] {
      background: #080710;
    }

    .phone-inner {
      position: relative;
      width: 300px;
      height: 626px;
      background: #0c0a16;
      overflow: hidden;
      flex: none;
      transform: scale(1.31);
    }

    .phone-inner-world {
      background: linear-gradient(to bottom, #080710 0%, #080612 14%, #080612 60%, #080611 100%);
    }

    .nav {
      position: relative;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 14px;
      padding: 25px 16px 0 16px;
      z-index: 5;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 0;
      font-family: 'Quicksand', 'Inter', sans-serif;
      font-size: 18px;
      letter-spacing: -0.3px;
      line-height: 1;
      position: relative;
      top: -3px;
    }

    .logo-bold {
      font-weight: 600;
      color: #fff;
    }

    .logo-light {
      font-weight: 300;
      color: #e7e3f2;
    }

    .logo-dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: #7f52ef;
      margin-left: 5px;
      position: relative;
      top: 1px;
    }

    .nav-actions {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .nav-btn {
      font-family: 'Inter', sans-serif;
      font-size: 9.5px;
      border-radius: 999px;
      padding: 9px 13px;
      white-space: nowrap;
      line-height: 1;
      display: inline-block;
    }

    .nav-btn-outline {
      font-weight: 400;
      color: #d9d5e8;
      border: 1px solid rgba(255, 255, 255, 0.3);
    }

    .nav-btn-purple {
      font-weight: 500;
      color: #a483f5;
      border: 1.5px solid #504081;
    }

    .nav-close {
      width: 34px;
      height: 34px;
      border-radius: 50%;
      background: #2a2930;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .ghost-text {
      position: absolute;
      top: -19px;
      left: -19px;
      font-family: 'Qanelas-Heavy', sans-serif;
      font-weight: 900;
      font-size: 68px;
      line-height: 0.72;
      color: #16112b;
      opacity: 0.5;
      letter-spacing: -0.02em;
      text-transform: uppercase;
      pointer-events: none;
      user-select: none;
      height: 108px;
      width: 100px;
    }

    .hero-section {
      position: relative;
      margin-top: 44px;
    }

    .hero-lock-icon {
      position: absolute;
      top: -16px;
      left: 50%;
      transform: translateX(-50%);
      width: 44px;
      height: 44px;
      border-radius: 50%;
      background: #01010e;
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 6;
    }

    .hero-card {
      width: 94%;
      max-width: 290px;
      margin: 22px auto 0;
      padding: 12px 18px;
      border-radius: 20px;
      background: linear-gradient(120deg, rgba(203, 191, 255, 0.11), rgba(203, 191, 255, 0.09));
      backdrop-filter: blur(18px);
      -webkit-backdrop-filter: blur(18px);
      position: relative;
      overflow: hidden;
      z-index: 4;
    }

    .hero-title {
      position: relative;
      z-index: 1;
      margin-left: -1px;
      font-family: 'Qanelas-Heavy', sans-serif;
      font-weight: 900;
      font-size: 37px;
      line-height: 0.98;
      color: #7442e9;
      letter-spacing: -0.7px;
      text-transform: uppercase;
      text-align: left;
      white-space: nowrap;
    }

    .how-to-win {
      position: relative;
      display: flex;
      flex-direction: column;
      align-items: center;
      margin-top: 26px;
      z-index: 4;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 9px;
      font-weight: 600;
      letter-spacing: 0;
      color: #d9d5e8;
      background: #17142a;
      border-radius: 999px;
      padding: 5px 11px;
    }

    .badge-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: #7f52ef;
    }

    .how-to-win-text {
      text-align: left;
      margin-top: 16px;
      font-size: 18px;
      font-weight: 400;
      line-height: 1.25;
      color: #f2f0f8;
    }

    .globe-container {
      position: absolute;
      left: 0;
      right: 0;
      bottom: -100px;
      height: 540px;
      z-index: 3;
      overflow: hidden;
    }

    .globe-container::after {
      content: '';
      position: absolute;
      left: 0;
      right: 0;
      bottom: 100px;
      height: 20%;
      background: linear-gradient(to bottom, transparent, #060410);
      z-index: 4;
      pointer-events: none;
    }

    .globe-img {
      position: absolute;
      left: 50%;
      bottom: 0;
      transform: translateX(-50%);
      width: 135%;
      height: auto;
      object-fit: cover;
    }

    .countdown-wrap {
      position: absolute;
      left: 0;
      right: 0;
      bottom: 44px;
      padding: 0 16px;
      z-index: 20;
    }

    .countdown {
      position: relative;
      height: 40px;
      width: 272px;
      margin: 0 auto;
    }

    .countdown-seg {
      position: absolute;
      top: 0;
      bottom: 0;
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 10.5px;
      font-weight: 600;
      color: #f4f1fb;
      border-radius: 9999px;
    }

    .countdown-seg-1 {
      left: 0;
      right: 0;
      padding: 0 0 0 11px;
      background: rgba(33, 24, 62, 0.72);
      z-index: 1;
      gap: 5px;
    }

    .countdown-seg-2 {
      left: 70px;
      right: 0;
      padding-left: 14px;
      background: rgba(38, 28, 72, 0.78);
      z-index: 2;
      gap: 3px;
    }

    .countdown-seg-3 {
      left: 126px;
      right: 0;
      padding-left: 14px;
      background: rgba(44, 33, 84, 0.84);
      z-index: 3;
      gap: 3px;
    }

    .countdown-seg-4 {
      left: 182px;
      right: 0;
      padding-left: 14px;
      background: rgba(50, 38, 96, 0.9);
      z-index: 4;
      gap: 3px;
    }

    .countdown-label {
      font-weight: 400;
      color: #cabfe6;
    }

    .left-to-win {
      position: absolute;
      left: 0;
      right: 0;
      bottom: 26px;
      text-align: center;
      font-size: 12px;
      font-weight: 500;
      color: #f4f1fb;
      z-index: 20;
    }

    .world-dome {
      position: absolute;
      left: 42%;
      top: 12%;
      width: 285%;
      aspect-ratio: 1/1;
      transform: translateX(-50%);
      border-radius: 50%;
      background: radial-gradient(circle closest-side at 50% 50%, #080612 0%, #080613 40%, #090715 62%, #0b0918 82%, #0c091a 93%, #0d0a1b 100%);
      z-index: 0;
      pointer-events: none;
    }

    .world-title {
      position: absolute;
      left: 22px;
      bottom: 96px;
      font-family: 'Qanelas-Heavy', sans-serif;
      font-weight: 900;
      font-size: 72px;
      line-height: 0.78;
      color: #7f52ef;
      text-transform: uppercase;
      letter-spacing: -0.02em;
      z-index: 2;
    }

    .world-title-globe-row {
      white-space: nowrap;
    }

    .globe-letter {
      position: relative;
      display: inline-block;
      color: transparent;
    }

    .world-globe-img {
      position: absolute;
      left: 50%;
      top: 50%;
      transform: translate(-50%, -50%);
      width: 90px;
      height: 90px;
      object-fit: contain;
      z-index: 30;
    }

    .about-glow {
      position: absolute;
      bottom: -40px;
      left: -40px;
      width: 280px;
      height: 280px;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(90, 50, 190, 0.35), transparent 70%);
    }

    .deck-container {
      position: relative;
      margin: 34px 16px 0;
      height: 314px;
      z-index: 4;
      overflow: visible;
      transform: scale(0.95);
      transform-origin: top center;
    }

    .smoke-wrap {
      position: absolute;
      inset: -12%;
      display: flex;
      align-items: center;
      justify-content: center;
      filter: blur(14px);
      opacity: 0.7;
      pointer-events: none;
      z-index: 0;
      -webkit-mask-image: radial-gradient(65% 55% at 52% 52%, #000 30%, transparent 75%);
      mask-image: radial-gradient(65% 55% at 52% 52%, #000 30%, transparent 75%);
    }

    .smoke-wrap svg {
      width: 120%;
      height: 120%;
    }

    .core-glow {
      position: absolute;
      width: 240px;
      height: 215px;
      top: 50%;
      left: 50%;
      transform: translate(-38%, -52%);
      border-radius: 50%;
      background: radial-gradient(circle, rgba(122, 78, 237, 0.22) 0%, rgba(122, 78, 237, 0) 68%);
      filter: blur(30px);
      pointer-events: none;
      z-index: 0;
    }

    .deck-card {
      position: absolute;
      border-radius: 18px;
      overflow: hidden;
      padding: 19px 20px 28px 18px;
      will-change: transform, opacity;
      backdrop-filter: blur(28px) saturate(120%);
      -webkit-backdrop-filter: blur(28px) saturate(120%);
      transition: left 0.55s cubic-bezier(0.22, 1, 0.36, 1),
                  right 0.55s cubic-bezier(0.22, 1, 0.36, 1),
                  top 0.55s cubic-bezier(0.22, 1, 0.36, 1),
                  bottom 0.55s cubic-bezier(0.22, 1, 0.36, 1),
                  transform 0.55s cubic-bezier(0.22, 1, 0.36, 1),
                  opacity 0.45s ease;
    }

    .deck-slot-0 {
      left: 0;
      right: 48px;
      top: 0;
      bottom: 0;
      z-index: 3;
      background: rgba(255, 255, 255, 0.075);
    }

    .deck-slot-1 {
      left: 24px;
      right: 24px;
      top: 34px;
      bottom: 34px;
      z-index: 2;
      background: rgba(255, 255, 255, 0.055);
    }

    .deck-slot-2 {
      left: 48px;
      right: 0;
      top: 72px;
      bottom: 72px;
      z-index: 1;
      background: rgba(255, 255, 255, 0.045);
    }

    .deck-card-text {
      position: relative;
      z-index: 1;
      transition: opacity 0.4s ease;
    }

    .deck-slot-0 .deck-card-text {
      opacity: 1;
      transition-delay: 0.16s;
    }

    .deck-slot-1 .deck-card-text,
    .deck-slot-2 .deck-card-text {
      opacity: 0;
      transition-delay: 0s;
    }

    .deck-card-title {
      font-family: 'Qanelas-Heavy', sans-serif;
      font-weight: 900;
      font-size: 19px;
      line-height: 0.99;
      color: #7f59e5;
      text-transform: uppercase;
      letter-spacing: -0.02em;
      white-space: pre-line;
    }

    .deck-card-body {
      margin-top: 16px;
      font-family: 'Inter', sans-serif;
      font-weight: 400;
      font-size: 11px;
      line-height: 1.75;
      letter-spacing: 0.1px;
      color: #e9e4f6;
    }

    .deck-card.deck-leaving {
      left: 0;
      right: 48px;
      top: 0;
      bottom: 0;
      z-index: 4;
      background: rgba(255, 255, 255, 0.075);
      transform: translateX(130%) rotate(10deg);
      opacity: 0;
      transition: transform 0.5s cubic-bezier(0.55, 0, 0.75, 0.45), opacity 0.34s ease 0.12s;
    }

    .deck-card.deck-entering-back {
      left: 48px;
      right: 0;
      top: 72px;
      bottom: 72px;
      z-index: 1;
      background: rgba(255, 255, 255, 0.045);
      transform: scale(0.9);
      opacity: 0;
      transition: none;
    }

    .deck-card.deck-entering-front {
      left: 0;
      right: 48px;
      top: 0;
      bottom: 0;
      z-index: 4;
      background: rgba(255, 255, 255, 0.075);
      transform: translateX(-130%) rotate(-10deg);
      opacity: 0;
      transition: none;
    }

    .deck-nav {
      position: absolute;
      left: 0;
      right: 0;
      bottom: 150px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 22px;
      z-index: 5;
    }

    .deck-arrow {
      width: 58px;
      height: 58px;
      border-radius: 50%;
      background: #01010e;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      user-select: none;
      -webkit-tap-highlight-color: transparent;
      transition: transform 0.16s ease, background 0.2s ease;
      border: none;
      outline: none;
    }

    .deck-arrow:hover {
      background: #151129;
    }

    .deck-arrow:active {
      transform: scale(0.86);
    }

    .deck-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
      background: #fff;
    }

    .about-bottom-title {
      position: absolute;
      left: 22px;
      bottom: 32px;
      font-family: 'Qanelas-Heavy', sans-serif;
      font-weight: 900;
      font-size: 31px;
      line-height: 0.94;
      color: #7f52ef;
      text-transform: uppercase;
      letter-spacing: -0.02em;
      z-index: 5;
    }

    @media (max-width: 980px) {
      .showcase {
        flex-wrap: wrap;
      }
    }

    @media (max-width: 440px) {
      .phone {
        zoom: 0.6;
      }
    }
  </style>
</head>
<body>
  <div class="showcase">
    <!-- Phone 1: Competition -->
    <div class="phone" data-screen="competition">
      <div class="phone-inner">
        <!-- Nav -->
        <nav class="nav">
          <div class="logo">
            <span class="logo-bold">nova</span><span class="logo-light">pay</span>
            <span class="logo-dot"></span>
          </div>
          <div class="nav-actions">
            <span class="nav-btn nav-btn-outline">What is?</span>
            <span class="nav-btn nav-btn-purple">Create a team</span>
          </div>
        </nav>

        <!-- Ghost text -->
        <div class="ghost-text">ONE<br>GLOBE,<br>ONE<br>FUTURE</div>

        <!-- Hero card -->
        <div class="hero-section">
          <div class="hero-lock-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#7f52ef" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="transform:rotate(-12deg);">
              <rect x="5" y="11" width="14" height="10" rx="2"></rect>
              <path d="M8 11V7a4 4 0 0 1 8 0v4"></path>
            </svg>
          </div>
          <div class="hero-card">
            <div class="hero-title">THE<br>COMPETITION<br>IS LIVE NOW<span>!</span></div>
          </div>
        </div>

        <!-- How to win -->
        <div class="how-to-win">
          <span class="badge">
            <span class="badge-dot"></span>HOW TO WIN BIG!
          </span>
          <div class="how-to-win-text">Transfer $1 Around the<br>world, Win $30,000!</div>
        </div>

        <!-- Video -->
        <div class="globe-container">
          <video src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260706_033111_a61458df-1103-4d80-95d8-82aef099bbf2.mp4" autoplay muted loop playsinline class="globe-img"></video>
        </div>

        <!-- Countdown -->
        <div class="countdown-wrap">
          <div class="countdown">
            <span class="countdown-seg countdown-seg-1">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#b7a4f0" stroke-width="2.2"><circle cx="12" cy="12" r="9"></circle><path d="M12 7v5l3 2" stroke-linecap="round"></path></svg>
              12&nbsp;<span class="countdown-label">days</span>
            </span>
            <span class="countdown-seg countdown-seg-2">23<span class="countdown-label">hs</span></span>
            <span class="countdown-seg countdown-seg-3">12<span class="countdown-label">min</span></span>
            <span class="countdown-seg countdown-seg-4">30<span class="countdown-label">seconds</span></span>
          </div>
        </div>
        <div class="left-to-win">left to win!</div>
      </div>
    </div>

    <!-- Phone 2: One World -->
    <div class="phone" data-screen="one-world">
      <div class="phone-inner phone-inner-world">
        <div class="world-dome"></div>
        <!-- Nav -->
        <nav class="nav">
          <div class="logo">
            <span class="logo-bold">nova</span><span class="logo-light">pay</span>
            <span class="logo-dot"></span>
          </div>
          <div class="nav-actions">
            <span class="nav-btn nav-btn-outline">What is?</span>
            <span class="nav-btn nav-btn-purple">Create a team</span>
          </div>
        </nav>
        <!-- Big text -->
        <div class="world-title">
          <div>ONE</div>
          <div class="world-title-globe-row">GL<span class="globe-letter">O<img src="https://polo-pecan-73837341.figma.site/_assets/v11/79bb00a5846ae132a06fc1f590fe4d05764300be.png" alt="globe" class="world-globe-img"></span>BE,</div>
          <div>ONE</div>
          <div>FUTURE<span>.</span></div>
        </div>
      </div>
    </div>

    <!-- Phone 3: About -->
    <div class="phone" data-screen="about">
      <div class="phone-inner">
        <div class="about-glow"></div>
        <!-- Nav -->
        <nav class="nav">
          <div class="logo">
            <span class="logo-bold">nova</span><span class="logo-light">pay</span>
            <span class="logo-dot"></span>
          </div>
          <div class="nav-close">
            <svg width="15" height="15" viewBox="0 0 24 24" stroke="#fff" stroke-width="2.4" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"></path></svg>
          </div>
        </nav>

        <!-- Card deck -->
        <div class="deck-container">
          <div class="smoke-wrap">
            <svg viewBox="0 0 800 800" preserveAspectRatio="xMidYMid slice">
              <defs>
                <filter id="nebula" x="-20%" y="-20%" width="140%" height="140%">
                  <feTurbulence type="fractalNoise" baseFrequency="0.0055 0.008" numOctaves="4" seed="11" result="noise"></feTurbulence>
                  <feColorMatrix in="noise" type="matrix" values="0 0 0 0 0.486  0 0 0 0 0.302  0 0 0 0 0.929  1.1 0 0 0 -0.42"></feColorMatrix>
                </filter>
              </defs>
              <rect width="800" height="800" filter="url(#nebula)"></rect>
            </svg>
          </div>
          <div class="core-glow"></div>
          <!-- Cards -->
          <div class="deck-card deck-card-0 deck-slot-0" id="card-0">
            <div class="deck-card-text">
              <div class="deck-card-title">ABOUT THE<br>COMPETITION</div>
              <div class="deck-card-body">The competition is about making global money transfers accessible and engaging. By logging into NovaPay with your phone number, creating a team, receiving a complimentary dollar, and initiating the game worldwide, participants are challenged to transfer this dollar across different locations, showcasing the ease and simplicity of</div>
            </div>
          </div>
          <div class="deck-card deck-card-1 deck-slot-1" id="card-1">
            <div class="deck-card-text">
              <div class="deck-card-title">HOW TO<br>PLAY</div>
              <div class="deck-card-body">Every transfer counts. Pass your dollar to a friend in another country and watch it hop across the map in real time. The further your dollar travels and the more borders it crosses, the higher your team climbs on the global leaderboard, one hop at a time.</div>
            </div>
          </div>
          <div class="deck-card deck-card-2 deck-slot-2" id="card-2">
            <div class="deck-card-text">
              <div class="deck-card-title">WIN BIG<br>PRIZES</div>
              <div class="deck-card-body">The team whose dollar crosses the most borders before the timer hits zero takes home the $30,000 grand prize. Weekly milestones unlock bonus rewards along the way, so keep your dollar moving — every single border it crosses brings your squad one hop closer to the big win.</div>
            </div>
          </div>
        </div>

        <!-- Nav arrows -->
        <div class="deck-nav">
          <button class="deck-arrow" id="prev-card" aria-label="Previous card">
            <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#7c54e5" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"></path><path d="M12 5l-7 7 7 7"></path></svg>
          </button>
          <span class="deck-dot"></span>
          <button class="deck-arrow" id="next-card" aria-label="Next card">
            <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#7c54e5" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"></path><path d="M12 5l7 7-7 7"></path></svg>
          </button>
        </div>

        <!-- Big bottom text -->
        <div class="about-bottom-title">WHAT IS<br>ONE GLOBE,<br>ONE FUTURE?</div>
      </div>
    </div>
  </div>

  <script>
    const cards = document.querySelectorAll('.deck-card');
    let active = 0;
    let animating = false;

    function getSlotClass(position) {
      return `deck-slot-${position}`;
    }

    function updateCards() {
      cards.forEach((card, i) => {
        const pos = (i - active + 3) % 3;
        card.className = `deck-card deck-card-${i} ${getSlotClass(pos)}`;
      });
    }

    document.getElementById('next-card').addEventListener('click', () => {
      if (animating) return;
      animating = true;

      const leaving = active;
      active = (active + 1) % 3;

      cards[leaving].className = `deck-card deck-card-${leaving} deck-leaving`;

      cards.forEach((card, i) => {
        if (i === leaving) return;
        const pos = (i - active + 3) % 3;
        card.className = `deck-card deck-card-${i} ${getSlotClass(pos)}`;
      });

      setTimeout(() => {
        cards[leaving].className = `deck-card deck-card-${leaving} deck-entering-back`;
        setTimeout(() => {
          cards[leaving].className = `deck-card deck-card-${leaving} deck-slot-2`;
          animating = false;
        }, 60);
      }, 480);
    });

    document.getElementById('prev-card').addEventListener('click', () => {
      if (animating) return;
      animating = true;

      const to = (active + 2) % 3;
      cards[to].className = `deck-card deck-card-${to} deck-entering-front`;

      setTimeout(() => {
        active = to;
        updateCards();
        setTimeout(() => {
          animating = false;
        }, 580);
      }, 60);
    });
  </script>
</body>
</html>

## AI Calorie Tracker — Health [apps/ai-calorie-tracker]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/calorieArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/ai-calorie-tracker.mp4

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Food Tracker</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600;700&display=swap" rel="stylesheet" />
<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html, body { min-height: 100%; font-family: "DM Sans", system-ui, -apple-system, sans-serif; background: #3c3c3c; }

.page {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 50px;
  min-height: 100vh;
  width: 100%;
  overflow: hidden;
  background: #CFC5BD;
  padding: 48px 20px;
}

@media (min-width: 1024px) {
  .page {
    flex-direction: row;
    justify-content: center;
    align-items: center;
    gap: 48px;
    padding: 48px 32px;
  }
}

.phone-wrapper {
  flex-shrink: 0;
}

@media (min-width: 1024px) {
  .phone-wrapper { flex-shrink: 1; align-self: auto !important; }
}

.phone-inner {
  width: 395px;
  height: 832px;
  transform-origin: top left;
}

.iphone-frame {
  position: relative;
  width: 395px;
  height: 832px;
  border-radius: 56px;
  background: #111111;
  padding: 10px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(255,255,255,0.08), 0 25px 50px -12px rgba(0,0,0,0.4), 0 12px 24px -8px rgba(0,0,0,0.3);
}

.iphone-viewport {
  width: 375px;
  height: 812px;
  border-radius: 46px;
  overflow: hidden;
  position: relative;
}

.btn-right { position: absolute; right: -2px; top: 180px; width: 3px; height: 80px; border-radius: 0 2px 2px 0; background: #222; }
.btn-left-1 { position: absolute; left: -2px; top: 130px; width: 3px; height: 28px; border-radius: 2px 0 0 2px; background: #222; }
.btn-left-2 { position: absolute; left: -2px; top: 185px; width: 3px; height: 52px; border-radius: 2px 0 0 2px; background: #222; }
.btn-left-3 { position: absolute; left: -2px; top: 245px; width: 3px; height: 52px; border-radius: 2px 0 0 2px; background: #222; }

.status-bar {
  position: absolute; left: 0; top: 0; z-index: 50;
  display: flex; align-items: center; justify-content: space-between;
  width: 375px; height: 54px; padding: 0 29px;
}
.status-time { width: 54px; text-align: center; font-size: 16px; font-weight: 600; line-height: 21px; letter-spacing: -0.32px; color: black; padding-top: 14px; }
.status-spacer { width: 125px; }
.status-icons { display: flex; align-items: center; gap: 6px; padding-top: 14px; }

.dynamic-island {
  position: absolute; left: 50%; top: 11px; z-index: 50;
  width: 125px; height: 37px; border-radius: 9999px; background: black;
  transform: translateX(-50%);
}

/* Screen backgrounds */
.screen { position: relative; width: 375px; height: 812px; overflow: hidden; }
.screen-onboarding { background: #F4F1EB; }
.screen-dashboard { background: #282828; }
.screen-recipes { background: #282828; }

.content-area {
  position: absolute; left: 0; top: 0;
  width: 375px; height: 716px;
  overflow: hidden; border-radius: 0 0 32px 32px;
  background: #F4F1EB;
}
.content-area::after {
  content: ''; position: absolute; left: 50%; top: 65%;
  width: 500px; height: 500px; transform: translateX(-50%);
  background: radial-gradient(circle, #FFA371 0%, transparent 70%);
  opacity: 0.6; pointer-events: none;
}

/* Bottom nav */
.bottom-nav {
  position: absolute; bottom: 0; left: 0; z-index: 40;
  display: flex; align-items: center; justify-content: center;
  width: 375px; height: 96px; padding: 0 16px;
}
.bottom-nav-inner {
  display: flex; align-items: center; justify-content: space-between;
  width: 343px; height: 56px; gap: 8px;
}
.nav-icon { display: flex; align-items: center; justify-content: center; width: 56px; height: 56px; border-radius: 9999px; }
.nav-icon-group { display: flex; align-items: center; gap: 12px; }
.nav-center { background: #FE9B66; }

/* Onboarding specific */
.onboarding-image { position: absolute; left: 0; top: 0; width: 375px; height: 573px; object-fit: cover; }
.blur-overlay {
  position: absolute; left: 0; bottom: 196px; width: 375px; height: 245px;
  background: rgba(244,241,235,0.6);
  backdrop-filter: blur(6px); -webkit-backdrop-filter: blur(6px);
  mask-image: linear-gradient(180deg, rgba(0,0,0,0) 0%, rgba(0,0,0,1) 62.5%);
  -webkit-mask-image: linear-gradient(180deg, rgba(0,0,0,0) 0%, rgba(0,0,0,1) 62.5%);
}
.onboarding-text {
  position: absolute; left: 0; top: 518px;
  display: flex; flex-direction: column; align-items: center; gap: 24px;
  width: 375px; padding: 0 16px 24px;
  opacity: 0; transform: translateY(30px);
  transition: opacity 0.8s cubic-bezier(0.16,1,0.3,1), transform 0.8s cubic-bezier(0.16,1,0.3,1);
}
.onboarding-text.visible { opacity: 1; transform: translateY(0); }
.onboarding-title {
  width: 343px; text-align: center; font-size: 48px; font-weight: 600;
  line-height: 50px; letter-spacing: -0.05em; color: #282828; text-transform: capitalize;
}
.onboarding-subtitle {
  width: 343px; text-align: center; font-size: 18px; font-weight: 500;
  line-height: 24px; letter-spacing: -0.02em; color: #908D86;
}
.dots { display: flex; gap: 4px; }
.dot { width: 6px; height: 6px; border-radius: 9px; background: #D7D1C5; }
.dot.active { background: #424141; }
.cta-button {
  display: flex; align-items: center; justify-content: center;
  width: 343px; height: 56px; border-radius: 20px; background: #282828;
  color: white; font-size: 16px; font-weight: 500; text-transform: capitalize;
  border: none; cursor: pointer; font-family: inherit;
}

/* Calorie labels */
.calorie-label {
  position: absolute; display: flex; flex-direction: column; align-items: center;
  opacity: 0; transform: translateY(40px);
  transition: opacity 0.8s cubic-bezier(0.16,1,0.3,1), transform 0.8s cubic-bezier(0.16,1,0.3,1);
}
.calorie-label.visible { opacity: 1; transform: translateY(0); }
.calorie-pill {
  display: flex; align-items: center; justify-content: center;
  height: 40px; border-radius: 32px; background: rgba(255,255,255,0.8);
  font-size: 18px; font-weight: 600; line-height: 20px; letter-spacing: -0.03em; color: #282828;
  position: relative;
}
.calorie-pill::after {
  content: ''; position: absolute; left: 50%; bottom: -6px; transform: translateX(-50%);
  width: 12px; height: 12px; border-radius: 50%; background: #FFA270;
}
.calorie-line {
  display: flex; flex-direction: column; align-items: center; overflow: hidden;
}
.calorie-line-bar {
  width: 3px; flex: 1;
  background: linear-gradient(180deg, rgba(255,255,255,0) 0%, rgba(255,255,255,1) 100%);
  transform: scaleY(0); transform-origin: top;
  transition: transform 1s cubic-bezier(0.16,1,0.3,1);
}
.calorie-label.visible .calorie-line-bar { transform: scaleY(1); }
.calorie-line-dot {
  width: 12px; height: 12px; border-radius: 50%; background: white; flex-shrink: 0;
  opacity: 0; transition: opacity 0.4s ease;
}
.calorie-label.visible .calorie-line-dot { opacity: 1; }

/* Dashboard */
.dashboard-header {
  margin-top: 56px; display: flex; align-items: center; justify-content: space-between;
  width: 343px; height: 56px;
}
.header-circle { width: 44px; height: 44px; border-radius: 50%; background: white; display: flex; align-items: center; justify-content: center; }
.header-title { font-size: 14px; font-weight: 500; line-height: 20px; color: #282828; }

.progress-ring { position: relative; width: 343px; height: 231px; overflow: hidden; margin-top: 16px; }
.progress-ring svg { position: absolute; left: 0; top: 0; width: 343px; height: 343px; }
.ring-center {
  position: absolute; left: 50%; top: 100px; transform: translateX(-50%);
  display: flex; flex-direction: column; align-items: center; gap: 4px; z-index: 10;
}
.ring-date { font-size: 14px; font-weight: 500; line-height: 18px; color: #FE9B66; }
.ring-kcal { margin-top: 4px; font-size: 28px; font-weight: 600; line-height: 32px; letter-spacing: -0.02em; color: #282828; text-align: center; }
.ring-goal { font-size: 14px; font-weight: 600; line-height: 16px; color: #FFA270; text-align: center; }

.add-btn {
  display: flex; align-items: center; justify-content: center;
  width: 343px; height: 48px; border-radius: 20px; background: #E8E3D8;
}

.meal-card {
  display: flex; flex-direction: column; justify-content: space-between;
  width: 343px; height: 138px; border-radius: 24px; padding: 12px 20px 12px 12px;
  background: linear-gradient(178deg, rgba(255,255,255,1) 0%, rgba(255,255,255,0.6) 100%);
  opacity: 0; transform: translateY(20px);
  transition: opacity 0.6s ease, transform 0.6s ease;
}
.meal-card.visible { opacity: 1; transform: translateY(0); }
.meal-top { display: flex; align-items: flex-start; justify-content: space-between; }
.meal-left { display: flex; align-items: center; gap: 12px; }
.meal-img { height: 64px; object-fit: contain; }
.meal-info { display: flex; flex-direction: column; justify-content: center; }
.meal-title { font-size: 18px; font-weight: 600; line-height: 25px; letter-spacing: -0.03em; color: #282828; text-transform: capitalize; }
.meal-time { font-size: 14px; font-weight: 500; line-height: 18px; color: #908D86; }
.meal-right { display: flex; flex-direction: column; align-items: flex-end; }
.meal-kcal { font-size: 28px; font-weight: 600; line-height: 32px; letter-spacing: -0.02em; color: #282828; }
.meal-percent { font-size: 14px; font-weight: 600; line-height: 18px; color: #FFA270; }
.meal-bottom { display: flex; align-items: center; justify-content: space-between; padding-left: 8px; height: 42px; }
.meal-macros { display: flex; width: 180px; align-items: center; justify-content: space-between; }
.macro-item { display: flex; flex-direction: column; align-items: flex-start; }
.macro-label { font-size: 14px; font-weight: 500; line-height: 18px; color: #908D86; }
.macro-value { font-size: 14px; font-weight: 600; line-height: 20px; color: #282828; }

/* Recipes */
.recipes-header {
  display: flex; align-items: center; gap: 12px;
  width: 343px; height: 56px; padding-top: 56px;
}
.search-bar {
  display: flex; align-items: center; gap: 10px;
  width: 279px; height: 56px; border-radius: 20px; background: white;
  padding: 15px 16px;
}
.search-text { font-size: 18px; font-weight: 500; line-height: 20px; letter-spacing: -0.03em; color: #908D86; }
.bell-circle { width: 56px; height: 56px; border-radius: 50%; background: white; display: flex; align-items: center; justify-content: center; }

.categories {
  display: flex; gap: 4px; overflow: hidden; padding-left: 16px; width: 375px; margin-top: 20px;
}
.category-item {
  display: flex; flex-direction: column; align-items: center; gap: 8px; width: 80px;
  opacity: 0; transform: translateY(20px) scale(0.8);
  transition: opacity 0.5s ease, transform 0.5s cubic-bezier(0.16,1,0.3,1);
}
.category-item.visible { opacity: 1; transform: translateY(0) scale(1); }
.category-img-wrap {
  display: flex; align-items: center; justify-content: center;
  width: 80px; height: 78px; border-radius: 20px; background: white; overflow: hidden;
}
.category-img { width: 56px; height: 56px; object-fit: contain; }
.category-label { width: 80px; text-align: center; font-size: 14px; font-weight: 500; line-height: 1.2em; color: #282828; }

.trending-header {
  display: flex; align-items: center; justify-content: space-between; width: 343px;
}
.trending-title { font-size: 20px; font-weight: 500; line-height: 25px; letter-spacing: -0.03em; color: #282828; text-transform: capitalize; }
.trending-link { display: flex; align-items: center; gap: 2px; font-size: 14px; font-weight: 500; line-height: 1.2em; color: #908D86; }

.carousel { position: relative; width: 375px; height: 400px; overflow: hidden; }
.carousel-card {
  position: absolute; left: 50%; top: 50%;
  width: 325px; border-radius: 24px; overflow: hidden;
  transition: transform 0.8s cubic-bezier(0.16,1,0.3,1), height 0.8s cubic-bezier(0.16,1,0.3,1);
}
.carousel-card img {
  position: absolute; left: 50%; transform: translateX(-50%); object-fit: contain;
  transition: all 0.8s cubic-bezier(0.16,1,0.3,1);
}
.carousel-card-top { position: absolute; left: 0; top: 0; width: 100%; padding: 20px; display: flex; align-items: center; justify-content: space-between; }
.time-badge { display: flex; align-items: center; gap: 8px; }
.time-badge-circle { width: 24px; height: 24px; border-radius: 50%; background: #FFA270; display: flex; align-items: center; justify-content: center; }
.time-badge-text { font-size: 13px; font-weight: 600; line-height: 16px; color: #282828; }
.time-pill { display: flex; align-items: center; gap: 4px; border-radius: 9999px; background: #FFA270; padding: 3px 8px; }
.time-pill-text { font-size: 10px; font-weight: 600; line-height: 12px; color: #282828; }
.card-bottom-center {
  position: absolute; bottom: 16px; left: 0; width: 100%; padding: 0 20px;
  display: flex; align-items: center; justify-content: space-between;
}
.difficulty { font-size: 22px; font-weight: 500; line-height: 28px; letter-spacing: -0.02em; color: #908D86; }
.difficulty-dots { display: flex; align-items: center; gap: 5px; }
.difficulty-dot { width: 9px; height: 26px; border-radius: 9999px; }
.card-kcal { font-size: 28px; font-weight: 600; line-height: 34px; letter-spacing: -0.02em; color: #282828; }
.card-bottom-side {
  position: absolute; left: 50%; top: 308px; width: 264px; transform: translateX(-50%);
  display: flex; align-items: center; justify-content: space-between;
}
.side-difficulty { font-size: 17px; font-weight: 500; line-height: 22px; letter-spacing: -0.02em; color: #908D86; }
.side-kcal { font-size: 27px; font-weight: 600; line-height: 30px; letter-spacing: -0.02em; color: #282828; text-align: right; }
</style>
</head>
<body>
<div class="page" id="page">
  <!-- Phone 1: Dashboard -->
  <div class="phone-wrapper" id="pw1" style="align-self: flex-start;">
    <div class="phone-inner" id="pi1">
      <div class="iphone-frame">
        <div class="iphone-viewport">
          <div class="screen screen-dashboard" id="dashboard-screen">
            <div class="content-area"></div>
            <!-- Status Bar -->
            <div class="status-bar">
              <span class="status-time">9:41</span>
              <div class="status-spacer"></div>
              <div class="status-icons">
                <svg width="18" height="12" viewBox="0 0 18 12" fill="none"><rect x="0" y="7" width="3" height="5" rx="1" fill="black"/><rect x="4.5" y="5" width="3" height="7" rx="1" fill="black"/><rect x="9" y="3" width="3" height="9" rx="1" fill="black"/><rect x="13.5" y="0" width="3" height="12" rx="1" fill="black"/></svg>
                <svg width="16" height="12" viewBox="0 0 16 12" fill="none"><path d="M1.5 4.5C4 2 6 1 8 1s4 1 6.5 3.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M3.5 6.5C5 5 6.5 4 8 4s3 1 4.5 2.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M5.5 8.5C6.5 7.5 7 7 8 7s1.5.5 2.5 1.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/></svg>
                <svg width="27" height="13" viewBox="0 0 27 13" fill="none"><rect x="0.5" y="0.5" width="23" height="12" rx="3.5" stroke="black" stroke-opacity="0.35"/><rect x="2" y="2" width="20" height="9" rx="2" fill="black"/><path d="M25 4.5v4a2 2 0 000-4z" fill="black" fill-opacity="0.4"/></svg>
              </div>
            </div>
            <div class="dynamic-island"></div>
            <!-- Dashboard Content -->
            <div style="position:absolute;left:0;top:0;z-index:10;display:flex;flex-direction:column;align-items:center;justify-content:space-between;width:375px;height:716px;padding-bottom:16px;">
              <div style="display:flex;flex-direction:column;align-items:center;width:375px;">
                <div class="dashboard-header">
                  <div style="display:flex;align-items:center;justify-content:center;width:56px;height:56px;">
                    <div class="header-circle">
                      <svg width="22" height="22" viewBox="0 0 24 24" fill="#282828"><path d="M8 2a1 1 0 0 1 1 1v1h6V3a1 1 0 1 1 2 0v1h1a4 4 0 0 1 4 4v10a4 4 0 0 1-4 4H6a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4h1V3a1 1 0 0 1 1-1z"/><path d="M2 10h20v8a4 4 0 0 1-4 4H6a4 4 0 0 1-4-4v-8z" fill="#282828"/><circle cx="8" cy="15" r="1.5" fill="white"/><circle cx="12" cy="15" r="1.5" fill="white"/><circle cx="16" cy="15" r="1.5" fill="white"/></svg>
                    </div>
                  </div>
                  <span class="header-title">Dashboard</span>
                  <div style="position:relative;display:flex;align-items:center;justify-content:center;width:56px;height:56px;">
                    <div class="header-circle">
                      <svg width="22" height="22" viewBox="0 0 24 24" fill="#282828"><path d="M12 2a6 6 0 0 0-6 6c0 3.09-.78 5.4-1.65 6.95-.42.75-.64 1.13-.62 1.22.02.1.05.16.13.22.07.05.46.05 1.24.05h13.8c.78 0 1.17 0 1.24-.05.08-.06.11-.12.13-.22.02-.09-.2-.47-.62-1.22C18.78 13.4 18 11.09 18 8a6 6 0 0 0-6-6z"/><path d="M9.35 21a3.02 3.02 0 0 0 5.3 0H9.35z" fill="#282828"/><circle cx="16" cy="6" r="4" fill="#FFA270" stroke="white" stroke-width="2"/></svg>
                    </div>
                  </div>
                </div>
                <div class="progress-ring" id="progress-ring">
                  <svg viewBox="0 0 343 343" id="ring-svg"></svg>
                  <div class="ring-center">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="#FE9B66" stroke="#FE9B66" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" fill="#FE9B66"/></svg>
                    <span class="ring-date">20 Aug</span>
                    <span class="ring-kcal" id="ring-kcal">0 kcal</span>
                    <span class="ring-goal">Goal 2000 kcal</span>
                  </div>
                </div>
              </div>
              <div style="display:flex;flex-direction:column;align-items:center;width:343px;gap:12px;">
                <div class="add-btn">
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#282828" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                </div>
                <div style="display:flex;flex-direction:column;gap:4px;">
                  <div class="meal-card visible" id="meal1">
                    <div class="meal-top">
                      <div class="meal-left">
                        <img src="https://framerusercontent.com/images/BqzJnhxC4oTmZS4LXssyLAmGuKQ.png" alt="" class="meal-img" style="width:61px;">
                        <div class="meal-info"><span class="meal-title">Lunch</span><span class="meal-time">02:30 PM</span></div>
                      </div>
                      <div class="meal-right"><span class="meal-kcal" id="meal1-kcal">0 kcal</span><span class="meal-percent">35% of goal</span></div>
                    </div>
                    <div class="meal-bottom">
                      <div class="meal-macros">
                        <div class="macro-item"><span class="macro-label">Protein</span><span class="macro-value" id="meal1-protein">0g</span></div>
                        <div class="macro-item"><span class="macro-label">Carbs</span><span class="macro-value" id="meal1-carbs">0g</span></div>
                        <div class="macro-item"><span class="macro-label">Fat</span><span class="macro-value" id="meal1-fat">0g</span></div>
                      </div>
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="#282828"><path d="M13.26 3.6l-8.21 8.69c-.31.33-.61.98-.67 1.43l-.37 3.24c-.13 1.17.71 1.97 1.87 1.77l3.22-.55c.45-.08 1.08-.4 1.39-.72l8.21-8.69c1.42-1.5 2.06-3.21-.15-5.3-2.2-2.07-3.87-1.37-5.29.13z"/><path d="M11.89 5.05a6.126 6.126 0 0 0 5.45 5.15" stroke="#282828" stroke-width="1.5" stroke-miterlimit="10" stroke-linecap="round" stroke-linejoin="round"/><path d="M3 22h18" stroke="#282828" stroke-width="1.5" stroke-miterlimit="10" stroke-linecap="round" stroke-linejoin="round"/></svg>
                    </div>
                  </div>
                  <div class="meal-card visible" id="meal2" style="transition-delay:200ms;">
                    <div class="meal-top">
                      <div class="meal-left">
                        <img src="https://framerusercontent.com/images/lvh2dnFe15JCcyQRI1L0nukdQCU.png" alt="" class="meal-img" style="width:63px;">
                        <div class="meal-info"><span class="meal-title">Breakfast</span><span class="meal-time">11:30 AM</span></div>
                      </div>
                      <div class="meal-right"><span class="meal-kcal" id="meal2-kcal">0 kcal</span><span class="meal-percent">25% of goal</span></div>
                    </div>
                    <div class="meal-bottom">
                      <div class="meal-macros">
                        <div class="macro-item"><span class="macro-label">Protein</span><span class="macro-value" id="meal2-protein">0g</span></div>
                        <div class="macro-item"><span class="macro-label">Carbs</span><span class="macro-value" id="meal2-carbs">0g</span></div>
                        <div class="macro-item"><span class="macro-label">Fat</span><span class="macro-value" id="meal2-fat">0g</span></div>
                      </div>
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="#282828"><path d="M13.26 3.6l-8.21 8.69c-.31.33-.61.98-.67 1.43l-.37 3.24c-.13 1.17.71 1.97 1.87 1.77l3.22-.55c.45-.08 1.08-.4 1.39-.72l8.21-8.69c1.42-1.5 2.06-3.21-.15-5.3-2.2-2.07-3.87-1.37-5.29.13z"/><path d="M11.89 5.05a6.126 6.126 0 0 0 5.45 5.15" stroke="#282828" stroke-width="1.5" stroke-miterlimit="10" stroke-linecap="round" stroke-linejoin="round"/><path d="M3 22h18" stroke="#282828" stroke-width="1.5" stroke-miterlimit="10" stroke-linecap="round" stroke-linejoin="round"/></svg>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <!-- Dashboard Bottom Nav -->
            <div class="bottom-nav">
              <div class="bottom-nav-inner">
                <div class="nav-icon-group">
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FE9B66"><path d="M3 10.5L12 3l9 7.5V21a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V10.5z"/><path d="M9 23V13h6v10" fill="#282828"/></svg></div>
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M6.5 2C5.12 2 4 3.12 4 4.5v15C4 20.88 5.12 22 6.5 22H20V2H6.5z"/><path d="M4 17.5A2.5 2.5 0 0 1 6.5 15H20v7H6.5A2.5 2.5 0 0 1 4 19.5v-2z"/><rect x="8" y="6" width="2" height="8" rx="1" fill="#282828"/></svg></div>
                </div>
                <div class="nav-icon nav-center"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#282828" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 8V6a2 2 0 0 1 2-2h2"/><path d="M16 4h2a2 2 0 0 1 2 2v2"/><path d="M20 16v2a2 2 0 0 1-2 2h-2"/><path d="M8 20H6a2 2 0 0 1-2-2v-2"/><line x1="4" y1="12" x2="20" y2="12"/></svg></div>
                <div class="nav-icon-group">
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M12 2l2.9 6.26L22 9.27l-5 4.87L18.18 22 12 18.27 5.82 22 7 14.14l-5-4.87 7.1-1.01L12 2z"/></svg></div>
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M19.14 12.94a7.2 7.2 0 0 0 .05-.94c0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.03 7.03 0 0 0-1.62-.94l-.36-2.54a.48.48 0 0 0-.48-.41h-3.84a.48.48 0 0 0-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.48.48 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.07.63-.07.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.37 1.03.7 1.62.94l.36 2.54c.05.24.25.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.57 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.01-1.58z"/><circle cx="12" cy="12" r="2.2" fill="#282828"/></svg></div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="btn-right"></div>
        <div class="btn-left-1"></div>
        <div class="btn-left-2"></div>
        <div class="btn-left-3"></div>
      </div>
    </div>
  </div>

  <!-- Phone 2: Onboarding -->
  <div class="phone-wrapper" id="pw2" style="align-self: center;">
    <div class="phone-inner" id="pi2">
      <div class="iphone-frame">
        <div class="iphone-viewport">
          <div class="screen screen-onboarding">
            <div class="status-bar">
              <span class="status-time">9:41</span>
              <div class="status-spacer"></div>
              <div class="status-icons">
                <svg width="18" height="12" viewBox="0 0 18 12" fill="none"><rect x="0" y="7" width="3" height="5" rx="1" fill="black"/><rect x="4.5" y="5" width="3" height="7" rx="1" fill="black"/><rect x="9" y="3" width="3" height="9" rx="1" fill="black"/><rect x="13.5" y="0" width="3" height="12" rx="1" fill="black"/></svg>
                <svg width="16" height="12" viewBox="0 0 16 12" fill="none"><path d="M1.5 4.5C4 2 6 1 8 1s4 1 6.5 3.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M3.5 6.5C5 5 6.5 4 8 4s3 1 4.5 2.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M5.5 8.5C6.5 7.5 7 7 8 7s1.5.5 2.5 1.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/></svg>
                <svg width="27" height="13" viewBox="0 0 27 13" fill="none"><rect x="0.5" y="0.5" width="23" height="12" rx="3.5" stroke="black" stroke-opacity="0.35"/><rect x="2" y="2" width="20" height="9" rx="2" fill="black"/><path d="M25 4.5v4a2 2 0 000-4z" fill="black" fill-opacity="0.4"/></svg>
              </div>
            </div>
            <div class="dynamic-island"></div>
            <div style="position:absolute;left:0;top:0;width:375px;height:573px;overflow:hidden;">
              <img src="https://framerusercontent.com/images/vzFRLyDH4mObF0PMM1sV4zN10k.png" alt="" class="onboarding-image">
              <!-- Calorie labels -->
              <div class="calorie-label" id="cal1" style="left:17px;top:93px;width:95px;">
                <div class="calorie-pill" style="width:95px;">170 kkal</div>
                <div class="calorie-line" style="height:110px;"><div class="calorie-line-bar" style="transition-delay:600ms;"></div><div class="calorie-line-dot" style="transition-delay:1100ms;"></div></div>
              </div>
              <div class="calorie-label" id="cal2" style="left:141px;top:174px;width:86px;">
                <div class="calorie-pill" style="width:86px;">90 kkal</div>
                <div class="calorie-line" style="height:150px;"><div class="calorie-line-bar" style="transition-delay:900ms;"></div><div class="calorie-line-dot" style="transition-delay:1400ms;"></div></div>
              </div>
              <div class="calorie-label" id="cal3" style="left:262px;top:99px;width:86px;">
                <div class="calorie-pill" style="width:86px;">110 kkal</div>
                <div class="calorie-line" style="height:90px;"><div class="calorie-line-bar" style="transition-delay:1200ms;"></div><div class="calorie-line-dot" style="transition-delay:1700ms;"></div></div>
              </div>
            </div>
            <div class="blur-overlay"></div>
            <div class="onboarding-text" id="onboarding-text">
              <div style="display:flex;flex-direction:column;align-items:center;gap:12px;width:343px;">
                <h1 class="onboarding-title">Your food, decoded by AI</h1>
                <p class="onboarding-subtitle">From scanning to tracking - everything happens automatically.</p>
              </div>
              <div class="dots"><div class="dot"></div><div class="dot"></div><div class="dot active"></div></div>
              <button class="cta-button">Get Started</button>
            </div>
          </div>
        </div>
        <div class="btn-right"></div>
        <div class="btn-left-1"></div>
        <div class="btn-left-2"></div>
        <div class="btn-left-3"></div>
      </div>
    </div>
  </div>

  <!-- Phone 3: Recipes -->
  <div class="phone-wrapper" id="pw3" style="align-self: center;">
    <div class="phone-inner" id="pi3">
      <div class="iphone-frame">
        <div class="iphone-viewport">
          <div class="screen screen-recipes">
            <div class="content-area"></div>
            <div class="status-bar">
              <span class="status-time">9:41</span>
              <div class="status-spacer"></div>
              <div class="status-icons">
                <svg width="18" height="12" viewBox="0 0 18 12" fill="none"><rect x="0" y="7" width="3" height="5" rx="1" fill="black"/><rect x="4.5" y="5" width="3" height="7" rx="1" fill="black"/><rect x="9" y="3" width="3" height="9" rx="1" fill="black"/><rect x="13.5" y="0" width="3" height="12" rx="1" fill="black"/></svg>
                <svg width="16" height="12" viewBox="0 0 16 12" fill="none"><path d="M1.5 4.5C4 2 6 1 8 1s4 1 6.5 3.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M3.5 6.5C5 5 6.5 4 8 4s3 1 4.5 2.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/><path d="M5.5 8.5C6.5 7.5 7 7 8 7s1.5.5 2.5 1.5" stroke="black" stroke-width="1.5" stroke-linecap="round"/></svg>
                <svg width="27" height="13" viewBox="0 0 27 13" fill="none"><rect x="0.5" y="0.5" width="23" height="12" rx="3.5" stroke="black" stroke-opacity="0.35"/><rect x="2" y="2" width="20" height="9" rx="2" fill="black"/><path d="M25 4.5v4a2 2 0 000-4z" fill="black" fill-opacity="0.4"/></svg>
              </div>
            </div>
            <div class="dynamic-island"></div>
            <!-- Recipes content -->
            <div style="position:absolute;left:0;top:0;z-index:10;display:flex;flex-direction:column;justify-content:space-between;width:375px;height:716px;padding-bottom:16px;">
              <div style="display:flex;flex-direction:column;align-items:center;width:375px;gap:20px;">
                <div style="display:flex;align-items:center;padding:56px 16px 0;width:375px;">
                  <div class="recipes-header">
                    <div class="search-bar">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#908D86" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
                      <span class="search-text">Search</span>
                    </div>
                    <div class="bell-circle">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="#282828"><path d="M12 2a6 6 0 0 0-6 6c0 3.09-.78 5.4-1.65 6.95-.42.75-.64 1.13-.62 1.22.02.1.05.16.13.22.07.05.46.05 1.24.05h13.8c.78 0 1.17 0 1.24-.05.08-.06.11-.12.13-.22.02-.09-.2-.47-.62-1.22C18.78 13.4 18 11.09 18 8a6 6 0 0 0-6-6z"/><path d="M9.35 21a3.02 3.02 0 0 0 5.3 0H9.35z" fill="#282828"/><circle cx="16" cy="6" r="4" fill="#FFA270" stroke="white" stroke-width="2"/></svg>
                    </div>
                  </div>
                </div>
                <div class="categories" id="categories">
                  <div class="category-item" style="transition-delay:0ms;"><div class="category-img-wrap"><img src="https://framerusercontent.com/images/cSUYlXEgijN1waXIccAabRGBTKs.png" alt="" class="category-img"></div><span class="category-label">All</span></div>
                  <div class="category-item" style="transition-delay:100ms;"><div class="category-img-wrap"><img src="https://framerusercontent.com/images/dvL0ds50sM1lbWt50gA2MeCoN7k.png" alt="" class="category-img" style="transform:rotate(6deg);"></div><span class="category-label">Vegan</span></div>
                  <div class="category-item" style="transition-delay:200ms;"><div class="category-img-wrap"><img src="https://framerusercontent.com/images/WslNoldhHMK5kUkfZvSQ0tjDjy8.png" alt="" class="category-img"></div><span class="category-label">Protein</span></div>
                  <div class="category-item" style="transition-delay:300ms;"><div class="category-img-wrap"><img src="https://framerusercontent.com/images/eGKYnKG12y3dNxuDXID4rF7pNXU.png" alt="" class="category-img"></div><span class="category-label">Snacks</span></div>
                  <div class="category-item" style="transition-delay:400ms;"><div class="category-img-wrap"><img src="https://framerusercontent.com/images/eGKYnKG12y3dNxuDXID4rF7pNXU.png" alt="" class="category-img"></div><span class="category-label">Drinks</span></div>
                </div>
              </div>
              <div style="display:flex;flex-direction:column;align-items:center;width:375px;gap:20px;">
                <div class="trending-header">
                  <span class="trending-title">Trending recipes</span>
                  <div class="trending-link"><span>See All</span><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#908D86" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg></div>
                </div>
                <div class="carousel" id="carousel"></div>
              </div>
            </div>
            <!-- Recipes Bottom Nav -->
            <div class="bottom-nav" style="background:#282828;">
              <div class="bottom-nav-inner">
                <div class="nav-icon-group">
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M3 10.5L12 3l9 7.5V21a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V10.5z"/><path d="M9 23V13h6v10" fill="#282828"/></svg></div>
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FE9B66"><path d="M6.5 2C5.12 2 4 3.12 4 4.5v15C4 20.88 5.12 22 6.5 22H20V2H6.5z"/><path d="M4 17.5A2.5 2.5 0 0 1 6.5 15H20v7H6.5A2.5 2.5 0 0 1 4 19.5v-2z"/><rect x="8" y="6" width="2" height="8" rx="1" fill="#282828"/></svg></div>
                </div>
                <div class="nav-icon nav-center"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#282828" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 8V6a2 2 0 0 1 2-2h2"/><path d="M16 4h2a2 2 0 0 1 2 2v2"/><path d="M20 16v2a2 2 0 0 1-2 2h-2"/><path d="M8 20H6a2 2 0 0 1-2-2v-2"/><line x1="4" y1="12" x2="20" y2="12"/></svg></div>
                <div class="nav-icon-group">
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M12 2l2.9 6.26L22 9.27l-5 4.87L18.18 22 12 18.27 5.82 22 7 14.14l-5-4.87 7.1-1.01L12 2z"/></svg></div>
                  <div class="nav-icon"><svg width="24" height="24" viewBox="0 0 24 24" fill="#FFFFFF"><path d="M19.14 12.94a7.2 7.2 0 0 0 .05-.94c0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.03 7.03 0 0 0-1.62-.94l-.36-2.54a.48.48 0 0 0-.48-.41h-3.84a.48.48 0 0 0-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.48.48 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.07.63-.07.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.37 1.03.7 1.62.94l.36 2.54c.05.24.25.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.57 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.01-1.58z"/><circle cx="12" cy="12" r="2.2" fill="#282828"/></svg></div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="btn-right"></div>
        <div class="btn-left-1"></div>
        <div class="btn-left-2"></div>
        <div class="btn-left-3"></div>
      </div>
    </div>
  </div>
</div>

<script>
// --- Responsive scaling ---
function setupScaling() {
  const wrappers = [
    { wrapper: document.getElementById('pw1'), inner: document.getElementById('pi1') },
    { wrapper: document.getElementById('pw2'), inner: document.getElementById('pi2') },
    { wrapper: document.getElementById('pw3'), inner: document.getElementById('pi3') },
  ];
  const page = document.getElementById('page');

  function update() {
    const style = getComputedStyle(page);
    const isColumn = style.flexDirection === 'column';
    const padding = parseFloat(style.paddingLeft) + parseFloat(style.paddingRight);
    const gap = isColumn ? 50 : 48;
    const count = wrappers.length;

    wrappers.forEach(({ wrapper, inner }) => {
      let availableWidth;
      if (isColumn) {
        availableWidth = page.clientWidth - padding;
      } else {
        availableWidth = (page.clientWidth - padding - gap * (count - 1)) / count;
      }
      const s = Math.min(availableWidth / 395, 1);
      const w = 395 * s;
      const h = 832 * s;
      wrapper.style.width = w + 'px';
      wrapper.style.height = h + 'px';
      wrapper.style.maxWidth = '100%';
      inner.style.transform = `scale(${s})`;
    });
  }

  const obs = new ResizeObserver(update);
  obs.observe(page);
  update();
}

// --- Count up animation ---
function countUp(el, end, duration, delay, suffix) {
  setTimeout(() => {
    const start = performance.now();
    function tick() {
      const elapsed = performance.now() - start;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      el.textContent = Math.round(eased * end) + suffix;
      if (progress < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }, delay);
}

// --- Progress Ring ---
function buildProgressRing() {
  const svg = document.getElementById('ring-svg');
  const totalSegments = 10;
  const filledSegments = 7;
  const cx = 171.5, cy = 180;
  const innerRadius = 90, outerRadius = 145;
  const startAngle = -180, endAngle = 0;
  const totalArc = endAngle - startAngle;
  const gapAngle = 4;
  const segmentAngle = (totalArc - gapAngle * (totalSegments - 1)) / totalSegments;
  const cornerOffset = 2.2;

  function polar(angle, r) {
    const rad = angle * Math.PI / 180;
    return { x: cx + r * Math.cos(rad), y: cy + r * Math.sin(rad) };
  }

  function segPath(i) {
    const a1 = startAngle + i * (segmentAngle + gapAngle);
    const a2 = a1 + segmentAngle;
    const oS = polar(a1 + cornerOffset, outerRadius);
    const oE = polar(a2 - cornerOffset, outerRadius);
    const iS = polar(a1 + cornerOffset, innerRadius);
    const iE = polar(a2 - cornerOffset, innerRadius);
    const oSc = polar(a1, outerRadius);
    const oEc = polar(a2, outerRadius);
    const iSc = polar(a1, innerRadius);
    const iEc = polar(a2, innerRadius);
    const ocS = polar(a1, outerRadius - 8);
    const ocE = polar(a2, outerRadius - 8);
    const icS = polar(a1, innerRadius + 8);
    const icE = polar(a2, innerRadius + 8);
    return [
      `M ${ocS.x} ${ocS.y}`,
      `Q ${oSc.x} ${oSc.y} ${oS.x} ${oS.y}`,
      `A ${outerRadius} ${outerRadius} 0 0 1 ${oE.x} ${oE.y}`,
      `Q ${oEc.x} ${oEc.y} ${ocE.x} ${ocE.y}`,
      `L ${icE.x} ${icE.y}`,
      `Q ${iEc.x} ${iEc.y} ${iE.x} ${iE.y}`,
      `A ${innerRadius} ${innerRadius} 0 0 0 ${iS.x} ${iS.y}`,
      `Q ${iSc.x} ${iSc.y} ${icS.x} ${icS.y}`,
      `Z`
    ].join(' ');
  }

  const paths = [];
  for (let i = 0; i < totalSegments; i++) {
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', segPath(i));
    path.setAttribute('fill', '#E8E3D8');
    path.style.opacity = '0.4';
    path.style.transition = 'opacity 0.3s ease';
    svg.appendChild(path);
    paths.push(path);
  }

  // Animate
  setTimeout(() => {
    const start = performance.now();
    const duration = 1800;
    function tick() {
      const elapsed = performance.now() - start;
      const p = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - p, 3);
      const visible = Math.round(eased * filledSegments);
      paths.forEach((path, i) => {
        if (i < visible) { path.setAttribute('fill', '#FE9B66'); path.style.opacity = '1'; }
        else if (i < filledSegments) { path.setAttribute('fill', '#E8E3D8'); path.style.opacity = (0.4 + 0.6 * eased).toString(); }
        else { path.setAttribute('fill', '#E8E3D8'); path.style.opacity = '1'; }
      });
      if (p < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }, 400);
}

// --- Recipe Carousel ---
function buildCarousel() {
  const container = document.getElementById('carousel');
  const cards = [
    { img: 'https://framerusercontent.com/images/vwY8y1o6djQqxCPbNAHzOMM5vG4.png', time: '48 min', difficulty: 'Easy', kcal: '750 kcal', dots: 2 },
    { img: 'https://framerusercontent.com/images/aGIOC9rOY7Vpwd5aA6qsaRDDE.png', time: '35 min', difficulty: 'Medium', kcal: '620 kcal', dots: 3 },
    { img: 'https://framerusercontent.com/images/vwY8y1o6djQqxCPbNAHzOMM5vG4.png', time: '22 min', difficulty: 'Easy', kcal: '580 kcal', dots: 1 },
  ];

  let activeIndex = 0;
  const cardEls = [];

  cards.forEach((card, i) => {
    const el = document.createElement('div');
    el.className = 'carousel-card';
    el.innerHTML = `
      <img src="${card.img}" alt="">
      <div class="carousel-card-top">
        <div class="time-badge"><div class="time-badge-circle"><svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M12 6v6l4 4" stroke="white" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg></div><span class="time-badge-text">${card.time}</span></div>
        <svg width="24" height="24" viewBox="0 0 24 24" fill="#282828"><path d="M12 2l2.9 6.26L22 9.27l-5 4.87L18.18 22 12 18.27 5.82 22 7 14.14l-5-4.87 7.1-1.01L12 2z"/></svg>
      </div>
      <div class="card-bottom-center">
        <div style="display:flex;align-items:center;gap:12px;">
          <span class="difficulty">${card.difficulty}</span>
          <div class="difficulty-dots">${Array.from({length:5}, (_,j) => `<div class="difficulty-dot" style="background:${j < card.dots ? '#FE9B66' : '#E0DDD7'}"></div>`).join('')}</div>
        </div>
        <span class="card-kcal">${card.kcal}</span>
      </div>
    `;
    container.appendChild(el);
    cardEls.push(el);
  });

  function updatePositions() {
    cardEls.forEach((el, i) => {
      const diff = (i - activeIndex + cards.length) % cards.length;
      const img = el.querySelector('img');
      const bottomCenter = el.querySelector('.card-bottom-center');
      if (diff === 0) {
        el.style.transform = 'translate(calc(-50%), -50%) scale(1)';
        el.style.height = '388px';
        el.style.zIndex = '10';
        el.style.background = 'linear-gradient(180deg, rgba(255,255,255,1) 0%, rgba(255,255,255,0.8) 100%)';
        img.style.top = '40px'; img.style.width = '250px'; img.style.height = '250px';
        bottomCenter.style.display = 'flex';
      } else if (diff === 1) {
        el.style.transform = 'translate(calc(-50% + 300px), -50%) scale(0.94)';
        el.style.height = '364px';
        el.style.zIndex = '5';
        el.style.background = 'linear-gradient(180deg, rgba(255,255,255,1) 0%, rgba(255,255,255,0.6) 100%)';
        img.style.top = '45px'; img.style.width = '264px'; img.style.height = '278px';
        bottomCenter.style.display = 'flex';
      } else {
        el.style.transform = 'translate(calc(-50% - 300px), -50%) scale(0.94)';
        el.style.height = '364px';
        el.style.zIndex = '5';
        el.style.background = 'linear-gradient(180deg, rgba(255,255,255,1) 0%, rgba(255,255,255,0.6) 100%)';
        img.style.top = '45px'; img.style.width = '264px'; img.style.height = '278px';
        bottomCenter.style.display = 'flex';
      }
    });
  }

  updatePositions();
  setInterval(() => { activeIndex = (activeIndex + 1) % cards.length; updatePositions(); }, 5000);
}

// --- Init ---
document.addEventListener('DOMContentLoaded', () => {
  setupScaling();
  buildProgressRing();
  buildCarousel();

  // Onboarding animations
  setTimeout(() => { document.getElementById('cal1').classList.add('visible'); }, 300);
  setTimeout(() => { document.getElementById('cal2').classList.add('visible'); }, 600);
  setTimeout(() => { document.getElementById('cal3').classList.add('visible'); }, 900);
  setTimeout(() => { document.getElementById('onboarding-text').classList.add('visible'); }, 600);

  // Categories animation
  setTimeout(() => {
    document.querySelectorAll('.category-item').forEach(el => el.classList.add('visible'));
  }, 200);

  // Count up animations
  countUp(document.getElementById('ring-kcal'), 1250, 2000, 400, ' kcal');
  countUp(document.getElementById('meal1-kcal'), 693, 1500, 0, ' kcal');
  countUp(document.getElementById('meal1-protein'), 48, 1500, 200, 'g');
  countUp(document.getElementById('meal1-carbs'), 83, 1500, 200, 'g');
  countUp(document.getElementById('meal1-fat'), 25, 1500, 200, 'g');
  countUp(document.getElementById('meal2-kcal'), 500, 1500, 200, ' kcal');
  countUp(document.getElementById('meal2-protein'), 36, 1500, 400, 'g');
  countUp(document.getElementById('meal2-carbs'), 57, 1500, 400, 'g');
  countUp(document.getElementById('meal2-fat'), 14, 1500, 400, 'g');
});
</script>
</body>
</html>

## Supplement Shop — Health [apps/supplement-shop]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/tablets.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/supplement-shop.mp4

Create a mobile supplement e-commerce product screen displayed inside a realistic iPhone mockup frame. Use React with Tailwind CSS and lucide-react icons. Load Google Fonts: **DM Sans** (400, 500) and **Inter** (400, 500, 600).

### Phone Frame
- Centered on page with `bg-neutral-100` background
- Frame: `w-[375px] h-[812px]`, white bg, `border-[8px] border-neutral-900 rounded-[50px]`, box-shadow `0 25px 50px -12px rgba(0,0,0,0.25)`
- Dynamic Island: `w-[120px] h-[32px]` black pill centered at top
- Home indicator at bottom: `w-[134px] h-[5px]` black rounded-full bar

### Status Bar
- Left: "9:41" (Inter 600)
- Right: SVG signal bars, WiFi icon, battery icon (all black)

### Header
- Left: hamburger Menu icon (lucide, size 22, strokeWidth 1.5)
- Center: "TerraElix" (DM Sans 500, letter-spacing -0.03em)
- Right: ShoppingBag icon with "10" badge (18x18 black circle, white 9px text)

### Slide-out Menu
- Overlay: `bg-black/40`, slides from left, `w-[260px]` white panel
- Logo + X close button at top
- Nav items: "About", "Products", "Promotions", "Contact" (Inter 400, hover:bg-neutral-100 rounded-lg)

### Title Section
- "Supplements" in DM Sans 400, `text-[42px] leading-[1]`

### Category Tabs
- Items: "All", "Capsules", "Tablets", "Functional Powders"
- Active tab (index 1 "Capsules"): black text, font-semibold, border-b-2 border-black
- Inactive: text-neutral-400, no border
- Font: Inter, letter-spacing -0.01em, text-sm

### Product Carousel (auto-plays every 3s, loops infinitely)
- 5 visible slots: farLeft, left, **center**, right, farRight
- Constants: `ITEM_WIDTH = 105`, `CENTER_WIDTH = 160`, `GAP = 12`
- Center item: 160x260px, opacity 1, z-10
- Adjacent items: 105x200px, opacity 0.7, z-0, clickable (go left/right)
- Far items: 105x200px, opacity 0.3
- Animation: All properties (width, height, transform, opacity) transition simultaneously with `0.45s cubic-bezier(0.25, 0.1, 0.25, 1)` -- items smoothly scale up/down as they move to/from center
- On transition end: update active product, reset quantity to 1

### Products data:
```
[
  { id: 1, name: 'Herbix 60', subtitle: 'Vitamin Complex', dosage: '250 mg', price: 30.0, image: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_182445_93ebd4ab-c1d7-447d-a033-c817f33efcd0.png&w=1280&q=85' },
  { id: 2, name: 'Herbix 30', subtitle: 'Immunity Boost', dosage: '500 mg', price: 25.0, image: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_184200_daae953b-540b-48d5-8a1f-70323e53af56.png&w=1280&q=85' },
  { id: 3, name: 'Herbix 90', subtitle: 'Joint Support', dosage: '300 mg', price: 35.0, image: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_184122_8a7d693e-37da-417d-804a-807ea67916af.png&w=1280&q=85' },
]
```

### Product Info (below carousel, centered)
- Name: DM Sans 400, text-2xl, fades out during animation (opacity 0)
- Subtitle: Inter 400, text-sm, text-neutral-500
- Dosage: Inter 400, text-xs, text-neutral-400

### Price + Quantity + Buy
- Price: DM Sans 500, text-2xl, letter-spacing -0.02em, shows `price * quantity`, fades during animation
- Quantity selector: two round bordered buttons (w-9 h-9, rounded-full, border-neutral-300) with Minus/Plus icons, number in center (Inter 500, text-lg)
- "Buy Now" button: full-width, `h-14 bg-black text-white rounded-xl`, Inter 500, hover:bg-neutral-800, active:scale-[0.97]

### Mount Animation (staggered on page load)
- Each section fades/slides in with `duration-700` and increasing delays (0ms, 100ms, 200ms, 300ms, 400ms, 500ms, 600ms)
- Carousel section uses `scale-95 -> scale-100` entrance

### CSS Requirements
- Tailwind CSS
- Custom `.scrollbar-hide` utility class to hide scrollbars
- Body: margin 0, font-smoothing antialiased

## Dental Implant Clinic — Healthcare [apps/dental-implant-clinic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/dentalimplantArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/dental-implant-clinic.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>SmileLab - Phone Mockups</title>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Inter', sans-serif;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
      min-height: 100vh;
      background: #f0f2f5;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 48px;
      padding: 32px;
    }

    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    @keyframes blurIn {
      from { opacity: 0; filter: blur(12px); }
      to { opacity: 1; filter: blur(0px); }
    }

    @keyframes slideDown {
      from { opacity: 0; transform: translateY(-20px); }
      to { opacity: 1; transform: none; }
    }

    @keyframes slideUp {
      from { opacity: 0; transform: translateY(30px); }
      to { opacity: 1; transform: none; }
    }

    /* Phone Mockup */
    .phone {
      position: relative;
      width: 375px;
      height: 812px;
      border-radius: 54px;
      border: 12px solid #1a1a1a;
      background: #1a1a1a;
      box-shadow: 0 50px 100px -20px rgba(0,0,0,0.4), 0 30px 60px -30px rgba(0,0,0,0.5), inset 0 -2px 6px 0 rgba(255,255,255,0.05);
      overflow: hidden;
    }

    .phone__frame-highlight {
      position: absolute;
      inset: 0;
      border-radius: 42px;
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.1);
      pointer-events: none;
      z-index: 50;
    }

    .phone__island {
      position: absolute;
      top: 14px;
      left: 50%;
      transform: translateX(-50%);
      width: 126px;
      height: 34px;
      background: black;
      border-radius: 9999px;
      z-index: 40;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .phone__island-dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: #1a1a1a;
      border: 1px solid #2a2a2a;
      margin-right: 32px;
    }

    .phone__screen {
      position: relative;
      width: 100%;
      height: 100%;
      border-radius: 42px;
      overflow: hidden;
      background: #5F9AD1;
    }

    .phone__bottom-bar {
      position: absolute;
      bottom: 8px;
      left: 50%;
      transform: translateX(-50%);
      width: 134px;
      height: 5px;
      background: rgba(255,255,255,0.3);
      border-radius: 9999px;
      z-index: 40;
    }

    /* Shared Header */
    .header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 56px 20px 0;
      animation: slideDown 0.7s ease-out 0.1s both;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .logo-text {
      color: white;
      font-size: 18px;
      font-weight: 500;
      letter-spacing: -0.025em;
    }

    .menu-btn {
      width: 40px;
      height: 40px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: none;
      border: none;
      color: white;
      cursor: pointer;
    }

    /* Screen 1: Dental Implants */
    .screen1 {
      position: relative;
      height: 100%;
      width: 100%;
      overflow: hidden;
      background: #5F9AD1;
      display: flex;
      flex-direction: column;
    }

    .screen1__content {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      position: relative;
    }

    .screen1__heading {
      text-align: center;
      padding: 0 20px;
      margin-top: 96px;
      animation: blurIn 0.9s ease-out 0.3s both;
    }

    .screen1__heading-wrapper {
      position: relative;
      display: inline-block;
    }

    .screen1__heading-text {
      display: block;
      color: white;
      font-size: 64px;
      font-weight: 400;
      line-height: 1.1;
      letter-spacing: -0.025em;
    }

    .screen1__heading-text--back { position: relative; z-index: 0; }
    .screen1__heading-text--front { position: relative; z-index: 20; }

    .screen1__implant-img {
      position: absolute;
      z-index: 10;
      left: 50%;
      transform: translateX(-50%);
      bottom: -12px;
      height: 180%;
      width: auto;
      object-fit: contain;
      pointer-events: none;
    }

    .screen1__subtext {
      margin-top: 32px;
      font-size: 14px;
      line-height: 1.4;
      max-width: 240px;
      margin-left: auto;
      margin-right: auto;
    }

    .screen1__subtext--muted { color: rgba(255,255,255,0.7); }
    .screen1__subtext--white { color: white; }

    .screen1__bottom {
      position: absolute;
      bottom: 0;
      left: 0;
      right: 0;
    }

    .screen1__stat {
      position: absolute;
      bottom: 180px;
      left: 56px;
      z-index: 20;
      animation: slideUp 0.9s ease-out 0.6s both;
      display: flex;
      flex-direction: column;
      align-items: center;
    }

    .screen1__stat-number {
      color: #3D8CD5;
      font-size: 30px;
      font-weight: 700;
      text-align: center;
    }

    .screen1__stat-label {
      color: #3D8CD5;
      font-size: 12px;
      font-weight: 500;
      text-align: center;
      line-height: 1.3;
    }

    .screen1__avatars {
      position: absolute;
      bottom: 50px;
      right: 20px;
      z-index: 30;
      display: flex;
      align-items: center;
      animation: slideUp 0.9s ease-out 0.8s both;
    }

    .screen1__avatar {
      width: 48px;
      height: 48px;
      border-radius: 50%;
      object-fit: cover;
      box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1);
      margin-left: -12px;
    }

    .screen1__avatar:first-child { margin-left: 0; }

    .screen1__avatar-badge {
      width: 48px;
      height: 48px;
      border-radius: 50%;
      background: #EBFA73;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1);
      margin-left: -12px;
    }

    .screen1__avatar-badge span {
      color: #3D8CD5;
      font-size: 12px;
      font-weight: 700;
    }

    .screen1__girl {
      position: relative;
      z-index: 10;
      animation: slideUp 0.9s ease-out 0.7s both;
      padding-left: 24px;
    }

    .screen1__girl img {
      width: 150%;
      height: auto;
      object-fit: contain;
      object-position: bottom;
    }

    /* Screen 2: Hero Video */
    .screen2 {
      position: relative;
      height: 100%;
      width: 100%;
      overflow: hidden;
      background: #5F9AD1;
    }

    .screen2__video {
      position: absolute;
      bottom: 0;
      left: 0;
      top: 30%;
      height: 70%;
      width: 100%;
      object-fit: cover;
      object-position: 80% center;
      animation: fadeIn 1.2s ease-out 0.2s both;
    }

    .screen2__gradient {
      position: absolute;
      left: 0;
      right: 0;
      top: 30%;
      height: 128px;
      z-index: 1;
      background: linear-gradient(to bottom, #5F9AD1, transparent);
    }

    .screen2__content {
      position: relative;
      z-index: 10;
      height: 100%;
      display: flex;
      flex-direction: column;
    }

    .screen2__heading {
      padding: 0 20px;
      margin-top: 24px;
      text-align: center;
      animation: blurIn 0.9s ease-out 0.3s both;
    }

    .screen2__heading h1 {
      color: white;
      font-size: 64px;
      font-weight: 400;
      line-height: 0.9;
      letter-spacing: -0.025em;
    }
  </style>
</head>
<body>

  <!-- Phone 1: Dental Implants Screen -->
  <div class="phone">
    <div class="phone__frame-highlight"></div>
    <div class="phone__island"><div class="phone__island-dot"></div></div>
    <div class="phone__screen">
      <section class="screen1">
        <header class="header">
          <div class="logo">
            <svg width="24" height="28" viewBox="0 0 32 36" fill="none">
              <path d="M16 0C10.5 0 7 3 5.5 6C4 9 3.5 12.5 3.5 16C3.5 20 4.5 24 7 27.5C9 30.5 11 33 13.5 35C15 36.2 16 36 16 36C16 36 17 36.2 18.5 35C21 33 23 30.5 25 27.5C27.5 24 28.5 20 28.5 16C28.5 12.5 28 9 26.5 6C25 3 21.5 0 16 0Z" fill="white"/>
              <path d="M16 5C12.5 5 10 6.5 9 8.5C8 10.5 7.5 12.5 7.5 15C7.5 18 8.5 21 10.5 23.5C12 25.5 13.5 27.5 15 29C15.5 29.5 16 29.5 16 29.5C16 29.5 16.5 29.5 17 29C18.5 27.5 20 25.5 21.5 23.5C23.5 21 24.5 18 24.5 15C24.5 12.5 24 10.5 23 8.5C22 6.5 19.5 5 16 5Z" fill="#5F9AD1"/>
            </svg>
            <span class="logo-text">SmileLab</span>
          </div>
          <button class="menu-btn" aria-label="Menu">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/>
            </svg>
          </button>
        </header>

        <div class="screen1__content">
          <div class="screen1__heading">
            <div class="screen1__heading-wrapper">
              <span class="screen1__heading-text screen1__heading-text--back">Dental</span>
              <img
                src="https://soft-zoom-63098134.figma.site/_assets/v11/2d10b6434e9908d20016ce4631e30910b16512fb.png"
                alt="Dental implant"
                class="screen1__implant-img"
              />
              <span class="screen1__heading-text screen1__heading-text--front">Implants</span>
            </div>

            <p class="screen1__subtext">
              <span class="screen1__subtext--muted">Dental implants are our core expertise, performed with </span>
              <span class="screen1__subtext--white">precision</span>
              <span class="screen1__subtext--muted"> and </span>
              <span class="screen1__subtext--white">long-term care.</span>
            </p>
          </div>

          <div class="screen1__bottom">
            <div class="screen1__stat">
              <p class="screen1__stat-number">98%</p>
              <p class="screen1__stat-label">loyal dental<br/>patients</p>
            </div>

            <div class="screen1__avatars">
              <img src="https://images.pexels.com/photos/1239291/pexels-photo-1239291.jpeg?auto=compress&cs=tinysrgb&w=100" alt="Patient" class="screen1__avatar" />
              <img src="https://images.pexels.com/photos/774909/pexels-photo-774909.jpeg?auto=compress&cs=tinysrgb&w=100" alt="Patient" class="screen1__avatar" />
              <div class="screen1__avatar-badge"><span>+2k</span></div>
            </div>

            <div class="screen1__girl">
              <img
                src="https://soft-zoom-63098134.figma.site/_assets/v11/ecccf0c10f5c64505f8cb104b04c72aba0b85b0c.png?w=512"
                alt="Happy patient"
              />
            </div>
          </div>
        </div>
      </section>
    </div>
    <div class="phone__bottom-bar"></div>
  </div>

  <!-- Phone 2: Hero Video Screen -->
  <div class="phone">
    <div class="phone__frame-highlight"></div>
    <div class="phone__island"><div class="phone__island-dot"></div></div>
    <div class="phone__screen">
      <section class="screen2">
        <video autoplay muted loop playsinline class="screen2__video">
          <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260710_141802_1d85412a-1df8-4993-8fc4-7400520bb1d1.mp4" type="video/mp4" />
        </video>
        <div class="screen2__gradient"></div>

        <div class="screen2__content">
          <header class="header">
            <div class="logo">
              <svg width="24" height="28" viewBox="0 0 32 36" fill="none">
                <path d="M16 0C10.5 0 7 3 5.5 6C4 9 3.5 12.5 3.5 16C3.5 20 4.5 24 7 27.5C9 30.5 11 33 13.5 35C15 36.2 16 36 16 36C16 36 17 36.2 18.5 35C21 33 23 30.5 25 27.5C27.5 24 28.5 20 28.5 16C28.5 12.5 28 9 26.5 6C25 3 21.5 0 16 0Z" fill="white"/>
                <path d="M16 5C12.5 5 10 6.5 9 8.5C8 10.5 7.5 12.5 7.5 15C7.5 18 8.5 21 10.5 23.5C12 25.5 13.5 27.5 15 29C15.5 29.5 16 29.5 16 29.5C16 29.5 16.5 29.5 17 29C18.5 27.5 20 25.5 21.5 23.5C23.5 21 24.5 18 24.5 15C24.5 12.5 24 10.5 23 8.5C22 6.5 19.5 5 16 5Z" fill="#5F9AD1"/>
              </svg>
              <span class="logo-text">SmileLab</span>
            </div>
            <button class="menu-btn" aria-label="Menu">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/>
              </svg>
            </button>
          </header>

          <div class="screen2__heading">
            <h1>Restore<br/>Your True<br/>Smile</h1>
          </div>
        </div>
      </section>
    </div>
    <div class="phone__bottom-bar"></div>
  </div>

</body>
</html>

## Coffee Rewards — Loyalty App [apps/coffee-rewards]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/coffeorangeArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/coffee-rewards.mp4

Build a mobile coffee profile screen inside a phone mockup frame. Use vanilla HTML, CSS, and JS with Vite as the bundler. The design is dark/warm-toned, inspired by iOS profile screens with glassmorphic UI elements.

**Phone mockup:**
- 390x844px at zoom 0.78, black background, 44px border-radius, overflow hidden, strong drop shadow
- Internal `.screen` div with dark background (`#180a06`), vertical scroll, hidden scrollbar

**Hero section (top):**
- Full-width, 430px tall
- Contains a looping muted autoplay video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260707_003042_3d2380a6-1ce6-4407-a2e2-cfec46546407.mp4`
- Video covers the area with `object-fit: cover; object-position: center top`
- Bottom gradient overlay fading from transparent at 52% to the background color at 100%
- Top bar with two glass circle buttons (edit icon on left, X close SVG on right), positioned absolute top 18px

**Identity section (overlaps hero by -112px margin-top):**
- Centered name "Dasha" (28px, weight 500)
- Left and right laurel images flanking the name (local assets `/assets/images/laurel-left.png` and `laurel-right.png`), 73px tall, 0.6 opacity, positioned via `right: calc(50% + 66px)` / `left: calc(50% + 66px)`
- Subtitle "Plum Parfait Latte" below (15px, muted color `rgba(235, 220, 205, 0.55)`)

**Achievements pill (centered, 30px below identity):**
- Glass pill button, 54px tall, 225px wide, 27px border-radius
- Trophy icon (local `/assets/images/icon-trophy.png`, 18px) + text "12 achievements" (18px, weight 500)

**Stats grid (3 columns, 12px gap, 26px top padding, 16px horizontal padding):**
- Each card: semi-transparent background `rgba(255,255,255,0.06)`, 24px border-radius
- Card 1: coffee image `https://polo-pecan-73837341.figma.site/_assets/v11/a8ba62db54d1e331b7beb36d69308e9b92516b99.png` (84x84), number "154", label "drinks consumed"
- Card 2: sandwich image `https://polo-pecan-73837341.figma.site/_assets/v11/953600065119f54f64ab9edb076b3cbb289fcff8.png` (84x84), number "36", label "sandwiches eaten"
- Card 3: cafe image `https://polo-pecan-73837341.figma.site/_assets/v11/aef68e05f729a30ed177f74c2cece578c05bfdba.png` (84x84), number "12", label "cafes visited"
- Numbers: 25px, weight 500. Labels: 13px, color `#BAAA9A8C`

**Favorite card (12px below stats, 16px horizontal margin):**
- Same card background, 24px radius, 110px height, flex row with 16px gap
- Latte image `https://polo-pecan-73837341.figma.site/_assets/v11/976a811111808abc50be33c2483872dbdb6ad5a8.png` (108x108, object-fit contain)
- Text column: "Favorite" (13px, weight 500), "Latte" (19px, weight 500), "Ordered 73 times" (13px, color `#BAAA9A8C`)
- Shuffle button on right (glass circle, local icon `/assets/images/icon-shuffle.png` 19px)

**Partial next card:** Same card style but only 34px tall with flat bottom corners (teaser for scroll)

**Glass button system:**
- Shared `.glass` class: no border, `rgba(255,255,255,0.03)` background, subtle inset box-shadows for edge highlights, `backdrop-filter: blur(2px) saturate(1.3)`, scale-down on `:active`
- `.glass.circle`: 58x44px, 22px radius
- `.glass.pill`: 54px tall, pill-shaped

**Fonts:**
- Primary: "Neue Haas Unica" (loaded from local `/assets/fonts/neue-haas-unica/stylesheet.css`)
- Also load "Helvetica Now Display" from `/assets/fonts/helvetica-now/stylesheet.css`
- Fallbacks: -apple-system, SF Pro Display, Inter, Segoe UI, Roboto

**Page background (behind phone):**
- Multiple warm-toned radial gradients over `#070402`:
  - `radial-gradient(ellipse 65% 55% at 15% 52%, rgba(168, 78, 10, 0.22))`
  - `radial-gradient(ellipse 52% 48% at 83% 26%, rgba(122, 52, 8, 0.17))`
  - `radial-gradient(ellipse 44% 52% at 56% 92%, rgba(98, 36, 5, 0.14))`
  - `radial-gradient(ellipse 30% 30% at 72% 75%, rgba(60, 20, 5, 0.10))`

**Entry animations (respects prefers-reduced-motion):**
- Hero: `heroReveal` - opacity 0 + scale(1.01) to normal, 1.9s, cubic-bezier(0.16, 1, 0.3, 1)
- Top bar buttons: `dropIn` from translateY(-10px), 0.7s, staggered 0.35s/0.42s
- Laurels: `fadeIn` 0.8s, delay 0.5s
- Name: `fadeRise` from translateY(16px), 0.7s, delay 0.5s
- Subtitle: same, delay 0.58s
- Achievements: same, delay 0.66s
- Stat cards: staggered at 0.74s, 0.80s, 0.86s
- Favorite card: delay 0.94s
- Next card: fadeIn delay 1.02s
- All use cubic-bezier(0.16, 1, 0.3, 1) except simple fadeIn which uses ease-out

**JavaScript (liquid glass effect):**
- Generates a displacement map canvas for each `[data-liquid]` element based on its dimensions and border-radius
- Uses SDF (signed distance field) of a rounded rectangle to compute refraction vectors
- Creates SVG `<feDisplacementMap>` filters and applies them as `backdrop-filter: url(#id) blur(0.3px) saturate(1.3)`
- Falls back to standard blur if SVG filter not supported

**CSS variables:**
```
--bg: #180a06
--card: rgba(255, 255, 255, 0.06)
--text: #ede4d8
--muted: rgba(235, 220, 205, 0.55)
--radius-card: 24px
```

**Responsive:** At 440px viewport, phone scales to zoom 0.6. Body flex-wraps at 900px.

## Innovation Summit — Mobile App [apps/innovation-summit]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/mobileupArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/innovation-summit.mp4

Create a mobile app showcase page that displays 3 phone-screen mockups side by side for an event called "Unfold" by MEWS. The page background is a muted purple (`#433B73`). Each screen is displayed inside a phone frame (393x873px with rounded corners). The screens should have entrance animations on page load.

---

### Screen 1: Home

A full-bleed looping background video with the event title centered.

- **Video background URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260704_080218_722465a5-cef1-4c6b-976a-948823474a8b.mp4`
- Video covers the entire frame, with a dark overlay on top (`rgba(0,0,0,.12)`), a bottom gradient fade (transparent to near-black over the bottom 34%), and a top gradient fade (semi-black fading to transparent over the top 16%)
- **Top bar:** "MEWS" in all caps (Michroma font, 16px, bold, letter-spacing .22em, white) on the left. On the right, a 46px circular button with purple-tinted border (`#78739f`) and two horizontal lines inside (hamburger menu)
- **Center content:**
  - Title "Unfold" in a large elegant serif/sans (General Sans semibold, 88px, white, slight negative letter-spacing)
  - Below by 48px: a metadata row centered with "Volume 05", a white pill shape (46x15px rounded), "Amsterdam", a small white dot (7px circle), "29.05.2024" -- all in Axiforma/Poppins medium 14px
- **Bottom (44px from bottom):** A white rounded pill button "Get a ticket" with right arrow, Inter bold 18px, padding 19px 34px, border-radius 40px, subtle box shadow. Include a one-time sheen/gloss animation sweeping across the button after the entrance animation

---

### Screen 2: Speakers

Dark screen (`#08080a`) with subtle border (`rgba(148,145,182,.28)`).

- **Top bar:** "Unfold" (General Sans semibold, 24px, white) on the left, white circular menu button (40px) on the right
- **Title:** "This Year's Speakers" on two lines (Axiforma regular, 48px, white, margin-top ~80px)
- **Description:** "Hospitality's best and brightest are invited to speak at Unfold. Here are some of the people who'll inspire attendees this year." (Inter 14px, line-height 1.6, white at 55% opacity)
- **Divider:** thin horizontal line (`rgba(255,255,255,.14)`)
- **Filter chips row** (flex wrap): "All" (active, filled purple `#9393f3` with dark text), "Keynote", "Hotelier", "Technology", "Consultant" (outlined, white at 55% opacity, rounded pill shape, 13.5px)
- **Speaker cards** (2-column grid, gap 14px):
  - Card 1: Image of Dimitris Manikis (URL: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_082140_91d42552-7cae-4b03-9a4d-f36a29ae93dc.png&w=1280&q=85`), aspect ratio ~150:172, border-radius 20px. Name: "Dimitris Manikis" (Inter 15px, white). Role: "President and MD for..." (Inter 12px, white 50%, ellipsis truncated)
  - Card 2: Image of Fiona McDonnell (URL: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_082107_663b204f-717a-49cf-9e97-7632b85a1cfd.png&w=1280&q=85`). Name: "Fiona McDonnell". Role: "VP Global Partner..."

---

### Screen 3: FAQs

Dark screen (`#0c0e12`) with subtle border.

- **Top bar:** Same as Speakers screen
- **Content scaled to 93%** (to fit more content elegantly)
- **Title:** "FAQs" (Axiforma regular, 52px, white, margin-top 74px)
- **Divider:** same style, wider vertical margins (72px top, 40px bottom)
- **Filter chips** (flex wrap with line breaks):
  - Row 1: "All" (active), "Where can I stay?", "In-person experience"
  - Row 2: "Tickets", "Venue", "Workshop sessions"
  - Row 3: "Networking & afterparty", "Other"
- **FAQ accordion items** (no expand behavior needed, just visual):
  - Each group has a small label tab above the first item (10px text on dark background `#1a1c22`, rounded top corners, text color `rgba(214,211,242,.9)`)
  - Each item: dark card (`#131519`), border `1.5px solid rgba(255,255,255,.14)`, border-radius 22px, padding ~22px. Question text on the left (Inter 17px, white), "+" toggle button on the right (36px circle with thin white border, white plus SVG icon)
  - Group 1 -- label "Where can I stay?": one question "Where can I stay?"
  - Group 2 -- label "In-person experience": two questions "What time does the event start?" and "What's included in my ticket?"

---

### Animations

All CSS-only, triggered on page load with staggered delays:

1. **Rise in** (translateY 26px to 0, fade in) - for headers, descriptions, CTA
2. **Blur in** (translateY + scale + blur + letter-spacing animate to normal) - for the "Unfold" title
3. **Clip up** (clip-path reveal from bottom) - for section titles
4. **Line grow** (scaleX 0 to 1 from left) - for dividers
5. **Stagger children** (each child delays by +70ms) - for chips and grid cards
6. **Sheen** (white gradient sweeps across CTA button once after 1.5s)

Use `cubic-bezier(.16, 1, .3, 1)` (expo out) for most animations. Respect `prefers-reduced-motion`.

Stagger timing: Home elements start at 0.45s-0.95s. Speakers elements at 0.6s-1.3s. FAQs elements at 0.8s-1.25s.

---

### Typography Stack

- **Michroma** - logo only
- **General Sans** (FontShare) - "Unfold" title and brand text
- **Satoshi** (FontShare) - fallback
- **Axiforma** - section headings (Speakers title, FAQ title)
- **Inter** - body text, chips, cards, questions, CTA button
- **Poppins** - fallback for meta text
- **Archivo** - base/fallback font

---

### Color System

| Token | Value |
|-------|-------|
| Page bg | `#433B73` |
| Frame dark | `#000`, `#08080a`, `#0c0e12` |
| Frame border | `rgba(148, 145, 182, .28)` |
| Active chip bg | `#9393f3` |
| Active chip text | `#181523` |
| Menu accent | `#78739f` |
| FAQ card bg | `#131519` |
| FAQ label bg | `#1a1c22` |
| FAQ label text | `rgba(214, 211, 242, .9)` |
| Border subtle | `rgba(255, 255, 255, .14)` |
| Text primary | `#fff` |
| Text secondary | `rgba(255, 255, 255, .55)` |
| Text tertiary | `rgba(255, 255, 255, .5)` |

---

### Responsive

- 3 screens side by side on desktop (flexbox, gap 40px)
- Wrap on screens below 900px
- Scale down phone frames on very small screens (below 440px)

---

### Key Design Details

- Phone frames should look like real device bezels (large border-radius 44px, dark background, overflow hidden)
- The home screen frame has no visible border; the other two have a subtle light purple/gray border
- Speaker card images have a dark fallback background (`#1a1a1e`) while loading
- The FAQ content uses a scale trick (93% transform) to fit more content while looking natural
- No JavaScript interactions needed - this is a static visual showcase with CSS entrance animations only

## Footballer Portfolio — Sports [apps/footballer-portfolio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/soccerplanArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/footballer-portfolio.mp4

Create a React + Vite + Tailwind CSS project that displays 3 mobile phone screens (375x812px each) side by side on a page with a `#BBB8B9` body background. No phone frames -- just raw screen content. The middle screen is positioned 75px higher than center, and the left/right screens are 45px lower than center. Screens are spaced with a 40px gap.

**Dependencies:** `react`, `react-dom`, `react-router-dom`, `lucide-react`, `@supabase/supabase-js`, Tailwind CSS.

**Fonts (Google Fonts loaded in index.html):**
- `Hammersmith One` (headings, logo, quotes -- uppercase, bold display)
- `Inter` (body text, labels, inputs -- weights 400, 500, 600, 700)

**Tailwind Config -- custom extensions:**
- `fontFamily.hammersmith`: `"Hammersmith One", sans-serif`
- `fontFamily.inter`: `"Inter", sans-serif`
- `colors.brand`: `#E30A17` (Turkish flag red, used as accent)

**Custom CSS Animations (defined in index.css):**
- `animate-fade-up`: translateY(24px) to 0, opacity 0 to 1, 0.8s, cubic-bezier(0.22, 1, 0.36, 1)
- `animate-fade-in`: opacity 0 to 1, 0.8s, same easing
- `animate-scale-in`: scale(0.92) to 1 + fade, same easing
- `animate-slide-right`: translateX(-30px) to 0 + fade
- `animate-slide-left`: translateX(30px) to 0 + fade
- Delay classes: `.delay-0` through `.delay-1200` (100ms increments)

---

### SCREEN 1 -- Hero (White Background)

**Layout:** `w-full h-[812px] bg-white overflow-hidden relative`

**Navigation (shared across all 3 screens):**
- Absolutely positioned at top, `px-5 pt-[60px] pb-4`, z-50
- Left: 44x44px circle with `bg-brand` (or custom `logoBg` prop), contains "Arda" / "Guler" text in white, `font-hammersmith text-[9px]`, stacked vertically
- Right: Hamburger button -- 3 bars (w-6 h-[2px]), animates to X when toggled
- Full-screen overlay menu on toggle with links: Partners, Collaboration, Home + social icons (TikTok, Instagram, Spotify, YouTube, X)
- `dark` prop variant: white text/bars on dark backgrounds

**Name Title:**
- Absolutely positioned, `top-[15%]`, centered, z-10
- `font-hammersmith text-brand uppercase`, font-size: 90px, line-height: 0.85, letter-spacing: -0.02em
- Text: "ARDA" (line break) "GULER"
- Animation: `animate-fade-in delay-100`

**Player Image:**
- Absolutely positioned, spans full height, z-20
- Image URL: `https://soft-zoom-63098134.figma.site/_assets/v11/617399912274f2b80327c4a1be99d14720bd14f3.png?h=1024`
- Positioned at bottom center, `h-[90%] w-auto object-contain`
- CSS mask: `linear-gradient(to bottom, black 60%, transparent 100%)` -- fades out at bottom
- Animation: `animate-fade-in delay-300`

**Quote Section:**
- Absolutely positioned, `bottom-[200px]`, z-30, `px-6`
- Opening curly quote character: `font-hammersmith text-[28px] uppercase leading-[28px]` in black
- Quote text: `font-hammersmith text-[22px] uppercase leading-[28px] tracking-[-0.01em]` in black
- Content: "Only a few of my dreams have come true; I still have a lot of dreams to achieve."
- Attribution: "Arda Guler" in `text-black/60 text-xs font-medium`
- Animation: `animate-fade-up delay-500`

**Bottom Cards Row:**
- Absolutely positioned at bottom, z-30, `flex gap-2 px-4 pb-10`
- Animation: `animate-fade-up delay-700`

**Left card (Video thumbnail):**
- `w-[42%]`, image height 140px, `object-cover`
- Image URL: `https://soft-zoom-63098134.figma.site/_assets/v11/6784c1243841844bc70e510357fd3060179cce83.png`
- Centered play button: 48x48 circle, `bg-brand`, white triangle SVG

**Right card (Next Game):**
- `flex-1`, `bg-black p-4`
- "NEXT GAME" pill badge: `bg-brand rounded-full`, white text 10px font-semibold
- Time: "19:00" in `text-white/70 text-xs`
- Two team badges: RM (gold `#FEBE10` circle, blue `#00529F` border/text) vs PC (navy `#162577` circle)
- VS circle: 28px, `border border-white/40`, white text

---

### SCREEN 2 -- Partners/Gallery (Dark Background)

**Layout:** `w-full h-[812px] bg-[#1C1C1D] overflow-hidden flex flex-col`

**Nav:** dark variant (white bars, white text)

**Heading:**
- `text-white font-hammersmith text-[32px] uppercase leading-[36px] px-5 mb-10`
- Text: "I'm more than a football player."
- Animation: `animate-fade-up delay-200`

**Horizontal Gallery Carousel:**
- `flex gap-2 overflow-x-auto pl-5 pr-2`, hidden scrollbar
- Animation: `animate-fade-up delay-400`
- Each card: `w-[280px]` fixed, flex-shrink-0

**Gallery Items (6 items):**
1. Image: `https://soft-zoom-63098134.figma.site/_assets/v11/3c81261e2c9a7b9a500141e5b3a3fdafd3d52409.png?h=512` | Title: "A seguir adelante" | Date: "10 March 2024"
2. Image: `https://soft-zoom-63098134.figma.site/_assets/v11/2f008d7054282f95f88c0be3bc528a7b36faf30c.png` | Title: "Puente Romano" | Date: "2 April 2024"
3. Image: `https://soft-zoom-63098134.figma.site/_assets/v11/de02babc0cd2f4a0166cb2ed7140bc3ef52e412b.png` | Title: "Mother" | Date: "25 May 2020"
4. Same as #1 | Title: "La vida ultimamente" | Date: "2 April 2024"
5. Same as #2 | Title: "Home" | Date: "2 April 2024"
6. Same as #3 | Title: "Puente Romano" | Date: "2 April 2024"

**Card structure:**
- Image container: `w-full h-[420px] overflow-hidden rounded-sm`, img is `object-cover`
- Below: title in `text-white text-[15px] font-semibold leading-tight`, date in `text-white/50 text-[12px] font-medium`
- Spacing: `pt-3 pb-8` below image

---

### SCREEN 3 -- Collaboration/Contact Form (Red Background)

**Layout:** `w-full h-[812px] bg-[#E30A17] overflow-hidden flex flex-col`

**Nav:** dark variant with `logoBg="bg-[#1C1C1D]"` (black logo circle instead of red)

**Content:** flex-1, flex-col, justify-end, `px-6 pb-14 gap-5`

**Heading:**
- `text-white font-hammersmith text-[32px] uppercase leading-[36px]`
- Text: "Do you have an ideas for collaboration?"
- Animation: `animate-fade-up delay-200`

**Description:**
- `text-white/80 text-sm font-normal leading-[22px]`
- Text: "For professional inquiries, collaborations, or media requests, please get in touch using the form below."
- Animation: `animate-fade-up delay-300`

**Form Fields (all have `border-b border-white/70`, h-[50px]):**
- Each field: input on left (white text, 60% opacity placeholder), label on right (`text-white/80 text-[11px] font-medium`)
- First Name: placeholder "David" | animate-fade-up delay-400
- Last Name: placeholder "Beckham" | same animation group
- Email: placeholder "davidbeckham@gmail.com" | animate-fade-up delay-500
- Message: textarea, `h-[90px]`, placeholder "Message" | animate-fade-up delay-600

**Checkbox + Submit (animate-fade-up delay-700):**
- Checkbox: 20x20, toggles between `border border-white/70` and `bg-[#1C1C1D]` with white checkmark SVG
- Label: "I accept the Terms of Conditions" (Terms in white, rest in white/80, text-[11px])
- Submit button: `h-[48px] w-full bg-[#1C1C1D] rounded`, "Submit Message" in white text-sm font-medium

---

### App Layout

```
Page: min-h-screen, flex items-center justify-center, py-10, gap-10, flex-wrap
Body background: #BBB8B9
Left phone wrapper: mt-[45px]
Middle phone wrapper: -mt-[75px]
Right phone wrapper: mt-[45px]
Each phone container: w-[375px] h-[812px] overflow-hidden (no frame/border/rounding)
```

Left phone = Screen 1 (Hero), Middle phone = Screen 2 (Partners), Right phone = Screen 3 (Collaboration).

## CARGOX Mobile — Transportation [apps/cargox-mobile]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/carbo.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/cargox-mobile.mp4

Create a Vite + React + TypeScript + Tailwind CSS project that displays 3 iPhone mockups side-by-side (stacking vertically on mobile). Each phone contains a different screen of the "CARGOX GROUP" logistics website. Use `motion/react` (Framer Motion v11+) for animations and `lucide-react` for icons.

---

### FONTS

- Google Fonts: `Barlow Condensed` weight 800 only
- System: `Helvetica, Arial, sans-serif` for body text

Import in CSS:
```
@import url('https://fonts.googleapis.com/css2?family=Barlow+Condensed:wght@800&display=swap');
```

---

### COLOR PALETTE

- Background (page): `#0a0a0a`
- Hero screen background: `#06181B`
- Yellow accent: `#ffda00`
- Dark teal/navy: `#002a35`
- Contact screen background: `#0a1f2b`
- Info section gradient: `linear-gradient(180deg, #C8C7B3 0%, #F0B172 50%, #EA7C58 100%)`
- Phone frame: `#1a1a1a` background, `#2a2a2a` 8px border, `inset 0 0 0 2px #3a3a3a`
- Text gray: `#b0b8bc`
- Text dark: `#1a1a1a`
- Footer text: `#6b7a80`

---

### PHONE MOCKUP (CSS, not a library)

Each phone is a div with class `iphone-frame`:
- `aspect-ratio: 393 / 852` (iPhone 15 Pro proportions)
- `height: 95vh; max-height: 900px` on desktop
- `border-radius: 54px`
- `border: 8px solid #2a2a2a`
- `box-shadow: 0 0 0 2px #0a0a0a, 0 40px 80px rgba(0,0,0,0.6), 0 20px 40px rgba(0,0,0,0.4), inset 0 0 0 2px #3a3a3a`
- Contains: Dynamic Island notch (absolute, top 12px, centered, 126x36px, `#000`, border-radius 20px, z-index 200)
- Contains: Screen area (flex:1, overflow hidden, border-radius 46px)
- Contains: Home indicator (absolute, bottom 8px, centered, 134x5px, `rgba(255,255,255,0.3)`, border-radius 3px, z-index 200)
- The screen inner is `position: absolute; inset: 0; overflow-y: auto; overflow-x: hidden` with hidden scrollbar

---

### LAYOUT

`.showcase-wrapper`:
- `display: flex; align-items: center; justify-content: center`
- `width: 100%; min-height: 100%; padding: 40px 24px; gap: 50px`

Phone order (left to right): InfoSection, HeroSection, ContactSection

On mobile (max-width 900px): stack vertically, gap 50px, padding 20px. Frame becomes `width: 393px; height: auto`. The `.phone-mockup` container gets CSS `zoom` via JS: `Math.min(1, (window.innerWidth - 40) / 393)`.

On medium (901-1200px): frame height 90vh, max-height 800px.

Each phone has entry animation: `opacity: 0, y: 60` -> `opacity: 1, y: 0`, duration 0.9s, ease `[0.16, 1, 0.3, 1]`. Center phone delay 0, side phones delay 0.3.

---

### SCREEN 1: HERO SECTION (center phone)

**Full-height dark screen** (`#06181B` background, `height: 100%`)

**Navbar** (absolute, top 0, left/right 0, z-100, padding 24px, transparent bg):
- Left: Logo text "CARGOX" (white) / "GROUP" (yellow `#ffda00`) -- `Barlow Condensed 800`, 32px, line-height 0.9, uppercase, letter-spacing -0.01em. Animates from `x: -24, opacity: 0`.
- Right: Hamburger icon (lucide `Menu`/`X`, 28px, white, 40x40 button). Animates from `x: 24, opacity: 0`.

**Mobile Menu** (AnimatePresence): when open, fixed full-screen overlay `#6682c2`, z-99. Items: "Services", "Industries", "Company" -- white, 24px, Helvetica. Fade in with stagger (0.05s each), slide from y:20.

**Hero Layout** (height 100%, flex column):
- **Video area** (63% height, flex-shrink 0): Autoplaying muted loop video:
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260620_185230_f7f71ef4-6655-469f-b9c6-efbdc1f7684a.mp4`
  Object-fit cover. Gradient overlay on bottom: `linear-gradient(to bottom, transparent 0%, #06181B 92%, #06181B 100%)`, height 55%, bottom -2px.

- **Text + CTA** (appears after video `onCanPlay`, AnimatePresence, flex-col, justify-end, padding `0 20px 24px`):
  - Big text: `Barlow Condensed 800`, 72px, line-height 0.82, letter-spacing -0.02em, uppercase, overflow clip:
    - "BEYOND" -- white, slides from `x: -400` (0.85s, delay 0)
    - "BORDERS" -- yellow `#ffda00`, text-align right, slides from `x: 400` (0.85s, delay 0.13)
    - "AND LIMITS" -- white, slides from `x: -400` (0.85s, delay 0.26)
  - Margin-bottom 16px between text block and button.

- **CTA Button** (custom SVG shape, full width, 56px height):
  - Background is an SVG `viewBox="0 0 434.001 68"` with fill `#ffda00` -- a pill shape with a circular cutout on the right side (the full SVG path is in the code).
  - Text "Get in touch" -- 20px, `#002a35`, Helvetica, centered in left portion (right 14.43% excluded).
  - Right circle contains an arrow SVG (chevron/arrow pointing up-left, rotating from -135deg to -90deg on hover). Arrow stroke white, strokeWidth 2.2.
  - Hover: scale 1.08, y -2. Tap: scale 0.97.
  - Animates in: opacity 0, y 20 -> visible (0.7s, delay 0.5).

---

### SCREEN 2: INFO SECTION (left phone)

**Single section with warm gradient background**: `linear-gradient(180deg, #C8C7B3 0%, #F0B172 50%, #EA7C58 100%)`. Padding: 60px 20px 40px. Min-height 100%, flex-1, centered.

**Tagline** (useInView animated, marginBottom 32px):
- "LOGISTICS" -- `Barlow Condensed 800`, 64px, line-height 0.9, letter-spacing -0.02em, uppercase, white. Slides from `x: -50`.
- "shaped by scale" / "powered by precision" -- Helvetica 26px, line-height 1.2, letter-spacing -0.02em, `#1a1a1a`. Slides from `x: -30`, delay 0.12.

**Map section** (aspect-ratio 435/340, marginBottom 40px, extends full width with -20px margins):
- Background image: `https://polo-pecan-73837341.figma.site/_assets/v11/b6d561167283e799453232309bd13dd78b2d1afa.png`
  (object-contain, absolute inset-0)

- **Route lines overlay** (positioned at left 10%, top 18%, width 80%, aspect-ratio 299/143):
  SVG viewBox `0 0 299.037 142.509`, overflow visible. 4 animated paths with stroke `#FFDA00`, strokeWidth 2.5:
  ```
  M128.161 74.6764C79.9989 130.001 71.9994 46.0005 20.9815 111.737
  M216.999 9.99985C260.499 12.4998 222.499 71.9998 291.999 58.9998
  M130.102 70.9998C144.499 -32.0002 183.852 70.2739 219.999 3.99985
  M14.4999 16.9998C111 20.9998 -53.0003 73.4998 21.4999 107
  ```
  Each path animates `pathLength: 0->1`, duration 1s, staggered delay 0.15s.
  Each path has a triangle `polygon points="0,-4 8,0 0,4"` fill `#FFDA00` animating along it via `<animateMotion>`.

- **Stop dots** (5 dots at specific coordinates):
  ```
  [9.519, 15.519], [289.519, 59.518], [220.519, 9.519], [125.518, 78.519], [19.519, 104.519]
  ```
  Each: outer circle r=9.519 fill `#FFDA00`, inner circle r=3.389 fill `#002A35`. Scale in with stagger.

- **Floating transport icons** (3 circular white badges, width 16% of map container, aspect-ratio 1):
  1. Ship at left 26%, top 28.9% -- image: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/08d6a37375d428e07c59e24a8529de89bfee157e.08d6a373.png`
  2. Car at left 70.8%, top 15.6% -- rotated 9.73deg -- image: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/7d6f50a87e1427d9b4d1a9c9f1c064ff04b2b3f9.7d6f50a8.png`
  3. Plane at left 55.2%, top 52.1% -- rotated 180deg scaleY(-1) -- image: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/0e0282ab1c70db03d437b0d01875ce45557d49f6.0e0282ab.png`

  Each: white rounded-full bg, box-shadow `0 4px 20px rgba(0,0,0,0.2)`, images 80% width/height object-cover. Scale in with delays 0.3/0.5/0.7, then float up/down infinitely (y oscillates by -6/-8/-5px, duration ~2.5-3.3s each).

**Stats** (flex column, gap 48px):
1. "3M+" (white, Barlow Condensed 800, 72px) + "tons of cargo / delivered / without delays" (18px, line-height 1.3, `#1a1a1a`). Gap 16px between number and text. Slides from `x: -60`.
2. "13+" same style + "years of trusted / and reliable / operations". Indented `marginLeft: 90px`. Slides from `x: 60`.

Both use `useInView` trigger with `margin: '0px 0px -40px 0px'`.

---

### SCREEN 3: CONTACT SECTION (right phone)

**Full-height dark screen** (`#0a1f2b`, padding 48px 20px 36px, flex-1).

**Heading** (useInView):
- "CONTACT " (white) + "US" (yellow `#FFDA00`) -- `Barlow Condensed 800`, 64px, line-height 0.9, uppercase, marginBottom 20px. Animates y: 30 -> 0.

**Subtitle**:
- "Complete the form and our team will contact you soon." -- Helvetica 18px, line-height 1.4, `#b0b8bc`, marginBottom 72px, maxWidth 400px. Animates y: 20 -> 0, delay 0.1.

**Form** (flex column, gap 16px, marginBottom 44px):
- 3 inputs: "First Name", "Last Name", "E-mail"
  - Style: `padding: 18px 24px; border-radius: 40px; border: none; background: rgba(255,255,255,0.08); color: #fff; font-size: 16px`
  - Each animates in (y: 20 -> 0) with stagger (0.15, 0.25, 0.35)
  - On focus: background `rgba(255,255,255,0.14)`, scale 1.01

- **Submit button** "Send": `width: 100%; padding: 18px; border-radius: 40px; background: #FFDA00; color: #0a1f2b; font-size: 20px; font-weight: 700`. marginTop 4px.
  - Hover: scale 1.03, y -2, background `#ffe84d`.
  - Tap: scale 0.97.

**Contact info** (marginBottom 32px):
- `info@cargox-group.com` -- 18px, `#b0b8bc`, no underline. Hover: white, x +4.
- `+380 44 234-7890` -- same style.

**Footer row** (flex, space-between, marginBottom 32px):
- Left: 3 social icons (Instagram, LinkedIn, Facebook) as inline SVGs, each in a 44x44px white circle. Hover: scale 1.15, y -3.
- Right: Scroll-to-top button (44x44px white circle with up-arrow SVG, stroke `#0a1f2b`, strokeWidth 2.5).

**Copyright**: "(c) 2025. All rights reserved." -- 14px, `#6b7a80`, text-align left.

---

### ANIMATION EASINGS

- `EXPO_OUT: [0.16, 1, 0.3, 1]` -- primary easing for most entrance animations
- `EASE_OUT: [0.25, 0.46, 0.45, 0.94]` -- secondary easing for paths and form fields

---

### KEY IMPLEMENTATION DETAILS

1. All `useInView` hooks use `{ once: true, margin: '0px 0px -40px 0px' }` for triggering slightly before elements enter viewport.
2. The video-ready state gates the hero text appearance (shows only after `onCanPlay` fires).
3. The CTA button arrow rotates from -135deg (default) to -90deg (hovered) with 0.35s transition.
4. Map route arrows use native SVG `<animateMotion>` with `rotate="auto"`.
5. Floating icons use Framer Motion keyframe arrays for infinite Y oscillation.
6. The phone zoom on mobile is calculated in JS and applied as CSS `zoom` property on `.phone-mockup`.

## Cross-Border — Transportation [apps/cross-border]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/freeautomobile.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/cross-border.mp4

Create a mobile-first logistics company landing page for "CARGOX GROUP" displayed inside an iPhone 15 Pro mockup frame. Use React, TypeScript, Tailwind CSS, and the `motion` library (motion/react) for animations. Use Vite as the build tool.

### Structure

The page is wrapped in a realistic iPhone 15 Pro mockup (aspect ratio 393:852) with:
- Dark frame (#1a1a1a) with 54px border-radius, 8px border (#2a2a2a), inset highlight (#3a3a3a)
- Dynamic Island (126x36px, centered at top, black, 20px border-radius, z-200)
- Home indicator at bottom (134x5px, white 30% opacity)
- Scrollable content area inside with hidden scrollbar and `container-type: size`

The body background is #111111. The scrollable container uses `container-type: size; container-name: phone;` for container queries.

### Fonts

- Import **Barlow Condensed 800** from Google Fonts
- Body font: Helvetica, Arial, sans-serif

### SECTION 1: Hero (full viewport height of the container using `100cqb`)

**Background:** Autoplaying, muted, looping video:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260620_185230_f7f71ef4-6655-469f-b9c6-efbdc1f7684a.mp4
```

**Navbar (absolute, top):**
- Left: "CARGOX" (white) / "GROUP" (yellow #ffda00) in Barlow Condensed 800, uppercase, clamp(22px, 6vw, 32px)
- Right: Hamburger menu icon (lucide-react Menu/X, 28px, white)
- Both slide in from left/right with 0.6s expo-out ease

**Mobile menu overlay:** Fixed, z-99, background #6682c2, centered nav items (Services, Industries, Company) in white 24px. AnimatePresence with scale + opacity transitions. Items stagger in from bottom.

**Hero content (bottom of section, z-10, padding 0 20px 24px):**
- Large headline in Barlow Condensed 800, uppercase, clamp(48px, 14vw, 72px), line-height 0.82:
  - "BEYOND" (white) - slides from x:-400
  - "BORDERS" (yellow #ffda00, text-align right) - slides from x:+400
  - "AND LIMITS" (white) - slides from x:-400
  - Staggered delays: 0s, 0.13s, 0.26s. Duration 0.85s, expo-out [0.16, 1, 0.3, 1]
- CTA Button: Custom SVG pill shape (fill #ffda00) with a circular end section containing a rotating arrow (white stroke, rotates from -135deg to -90deg on hover). Text "Get in touch" centered in the non-circle area. Font: Helvetica 20px, color #002a35. Hover: scale 1.08, y:-2. Tap: scale 0.97.

**Show hero content only after video `onCanPlay` fires** (fade in with AnimatePresence).

### SECTION 2: Info Card

**Background:** `linear-gradient(180deg, #C8C7B3 0%, #F0B172 50%, #EA7C58 100%)`
**Padding:** clamp(60px, 12vh, 120px) 20px

**Tagline** (scroll-triggered, slides from left):
- "LOGISTICS" in Barlow Condensed 800, white, clamp(44px, 13vw, 64px)
- "shaped by scale" / "powered by precision" in Helvetica, clamp(18px, 5vw, 26px), color #1a1a1a

**World Map:**
- Background map image: `https://polo-pecan-73837341.figma.site/_assets/v11/b6d561167283e799453232309bd13dd78b2d1afa.png`
- Aspect ratio 435/340, extends 20px beyond container edges
- SVG overlay (viewBox 0 0 299.037 142.509) at left:10%, top:18%, width:80% with 4 curved route paths in yellow (#FFDA00, 2.5 stroke):
  ```
  M128.161 74.6764C79.9989 130.001 71.9994 46.0005 20.9815 111.737
  M216.999 9.99985C260.499 12.4998 222.499 71.9998 291.999 58.9998
  M130.102 70.9998C144.499 -32.0002 183.852 70.2739 219.999 3.99985
  M14.4999 16.9998C111 20.9998 -53.0003 73.4998 21.4999 107
  ```
- Route lines animate with `pathLength` from 0 to 1, staggered
- Animated yellow arrow polygons (points="0,-4 8,0 0,4") using SVG `<animateMotion>` along each path, rotating automatically
- 5 stop dots at coordinates: [9.519,15.519], [289.519,59.518], [220.519,9.519], [125.518,78.519], [19.519,104.519] - each is a yellow circle r=9.519 with dark center r=3.389 (#002A35). They pop in with scale animation.
- 3 floating transport icons (white circle bg, 16% width, rounded-full, box-shadow):
  - Ship: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/08d6a37375d428e07c59e24a8529de89bfee157e.08d6a373.png` at left:26%, top:28.9%
  - Car: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/7d6f50a87e1427d9b4d1a9c9f1c064ff04b2b3f9.7d6f50a8.png` at left:70.8%, top:15.6%, rotate(9.73deg)
  - Plane: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/0e0282ab1c70db03d437b0d01875ce45557d49f6.0e0282ab.png` at left:55.2%, top:52.1%, rotate(180deg) scaleY(-1)
  - Icons pop in (scale 0.5 -> 1), then continuously float up/down with infinite y animation

**Stats** (scroll-triggered, slide in from opposite sides):
- "3M+" white Barlow Condensed 800, clamp(50px, 14vw, 72px) + "tons of cargo / delivered / without delays" in #1a1a1a, clamp(14px, 3.8vw, 18px)
- "13+" same styling, indented left clamp(40px, 12vw, 90px) + "years of trusted / and reliable / operations"

### SECTION 3: Contact Us

**Background:** #0a1f2b
**Padding:** clamp(48px, 10vh, 96px) 20px clamp(32px, 6vh, 64px)

**Heading** (scroll-triggered fade+slide from bottom):
- "CONTACT " (white) + "US" (yellow #ffda00), Barlow Condensed 800, clamp(44px, 13vw, 64px)
- Subtitle: "Complete the form and our team will contact you soon." in #b0b8bc, clamp(14px, 3.8vw, 18px)

**Form fields** (scroll-triggered, stagger from bottom):
- 3 inputs (First Name, Last Name, E-mail): pill-shaped (40px radius), bg rgba(255,255,255,0.08), white text, clamp(14px, 3.5vw, 16px). On focus: bg brightens to 0.14, slight scale 1.01.
- "Send" button: pill, bg #FFDA00, color #0a1f2b, font-weight 700, clamp(16px, 4vw, 20px). Hover: scale 1.03, y:-2, bg #ffe84d. Tap: scale 0.97.

**Footer info:**
- Email: info@cargox-group.com
- Phone: +380 44 234-7890
- Color #b0b8bc, hover slides right 4px and turns white

**Social icons** (3 white circles 44x44, hover: scale 1.15, y:-3):
- Instagram, LinkedIn, Facebook (inline SVG icons, fill #0a1f2b)

**Scroll-to-top button** (right side): white 44x44 circle with up-arrow SVG, same hover animation.

**Copyright:** "(c) 2025. All rights reserved." in #6b7a80, clamp(12px, 3.2vw, 14px)

### Animation System

- Use `motion/react` (NOT framer-motion)
- Easing curves: EXPO_OUT = [0.16, 1, 0.3, 1], EASE_OUT = [0.25, 0.46, 0.45, 0.94]
- Scroll-triggered reveals using `useInView` from motion/react with `once: true` and margin: '0px 0px -40px 0px' or '0px 0px -60px 0px'
- All scroll animations fire only once
- Transport icons have infinite floating y-axis animation with varying durations (2.5-3.3s)
- Mobile menu uses AnimatePresence for enter/exit

### Dependencies

```json
{
  "motion": "^12.40.0",
  "lucide-react": "^0.344.0",
  "react": "^18.3.1",
  "react-dom": "^18.3.1"
}
```

## Place Saver — Travel [apps/place-saver]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/perplxmobile.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/place-saver.mp4

**Build a single-page HTML showcase displaying two iOS device frames (370x790px each) side by side on a neutral `#F4F4F4` stage. The stage auto-scales to fit the viewport using JS. Both phones have a Dynamic Island, status bar (time "11:11", signal/wifi/battery icons in white SVG), and a home indicator bar. Animations are gated behind video `loadeddata` events + font loading, with a 5s safety timeout.**

---

### Fonts

1. **ITC Garamond Std Narrow** (self-hosted from Cloudinary):
   - Light (300): `https://res.cloudinary.com/dgupuutfn/raw/upload/v1783596334/ITCGaramondStd-LtNarrow_i2zcip.woff2` / `.woff` variant `ITCGaramondStd-LtNarrow_soc5vc.woff`
   - Book (400): `https://res.cloudinary.com/dgupuutfn/raw/upload/v1783596334/ITCGaramondStd-BkNarrow_xjfoc0.woff2` / `.woff` variant `ITCGaramondStd-BkNarrow_wfoxm1.woff`
   - Book Italic (400 italic): `https://res.cloudinary.com/dgupuutfn/raw/upload/v1783596334/ITCGaramondStd-BkNarrowIta_hiy9ld.woff2` / `.woff` variant `ITCGaramondStd-BkNarrowIta_rlarxo.woff`

2. **Google Fonts**: `Playfair Display` (400, 500, 600 + italic) and `Inter` (400, 500, 600, 700)

---

### Screen 1 -- "The place for all your places" (Light device frame)

**Background**: Dark (#02040c). Uses a native 470x1008 design scaled down to 370x790 via `transform: scale(0.787234)` with `transform-origin: top left`.

**Video background (hero)**:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260710_114906_ad7cee37-9e56-434f-99bc-92d5bdc4f9fe.mp4
```
- `autoplay loop muted playsinline`, `object-fit: cover`, `object-position: center 48%`, covers entire frame, z-index 0.

**Hero fade overlay**: Bottom 46% of the frame, gradient from `rgba(2,4,12,0)` to `rgba(2,4,12,.35)` at 46% to `rgba(2,4,12,.72)` at 100%. z-index 1.

**Logo** (centered, top 74px):
```
https://polo-pecan-73837341.figma.site/_assets/v11/b1ddc82509144261f1999a0c4d92be5ce6689c0f.png
```
- Width 118px, `filter: drop-shadow(0 0 7px rgba(190,215,255,.28))`. z-index 4.

**Title** (top 618px, centered, z-index 4):
- Font: ITC Garamond Std Narrow, weight 300, 66px, line-height 68px, letter-spacing 0.2px
- Text: `The place for all` then line break, then `your places` in italic (weight 400)
- Text shadow: `0 0 34px rgba(255,255,255,.22), 0 1px 2px rgba(0,0,0,.35)`
- The italic "your places" has an extra glow: `0 0 10px rgba(255,255,255,.6), 0 0 20px rgba(255,235,190,.5), 0 0 40px rgba(255,210,140,.32)`

**Subtitle** (top 787px, centered, z-index 4):
- Font: Inter, 16.5px, weight 400, line-height 26px, color `rgba(255,255,255,.52)`
- Text: "Save, Organize and Share\nyour favorite places"

**Button** (top 874px, left 32px, 406x55px, z-index 4):
- White background, border-radius 28px, box-shadow `0 6px 26px rgba(0,0,0,.28)`
- Apple logo SVG (18x21, fill #1a1a1a) + text "Continue with Apple"
- Font: Helvetica Neue, 18px, weight 500, -webkit-text-stroke 0.6px #1a1a1a

**Terms** (top 950px, centered, z-index 4):
- Font: 12px, weight 400, color `rgba(255,255,255,.42)`
- Text: "By continuing, you agree to **Terms of Use**" (bold text is `rgba(255,255,255,.82)`, weight 400)

---

### Screen 2 -- "Unlock Pro" (Dark device frame)

**Background**: Dark (#14151d).

**Video background**:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260710_115050_a1ba47d0-aedf-413c-9dea-14509599d3dd.mp4
```
- `autoplay loop muted playsinline`, positioned `left: 0; top: -30%`, 370x790px, `object-fit: cover`, z-index 0.

**Background fade** (full overlay, z-index 1):
```css
linear-gradient(to bottom,
  rgba(20,21,29,0) 0%, rgba(20,21,29,0) 40%,
  rgba(20,21,29,0.55) 55%, rgba(20,21,29,0.92) 66%,
  #14151d 74%, #14151d 100%);
```

**Heading** (left 28px, top 386px):
- Font: ITC Garamond Std Narrow, weight 500, 26px, line-height 1, color white
- Letter-spacing 0.2px, text-shadow `0 0 18px rgba(120,180,220,0.35)`
- Text: "Unlock Pro:"

**Divider** (left 28px, top 418px, 265x1px):
- `linear-gradient(to right, rgba(255,255,255,0.30) 0%, rgba(255,255,255,0.30) 70%, rgba(255,255,255,0) 100%)`

**Feature list** (left 28px, top 429px, width 314px):
Each row is 24px tall with a 22px-wide icon area + text (13.5px, weight 400, white, 4px left margin).

Icons are all SVG, 19x19 (or 20x20 for infinity), stroke #fff, stroke-width 1.7, no fill:

1. **Layers icon** (paths: `M12 2 2 7l10 5 10-5-10-5Z` / `M2 12l10 5 10-5` / `M2 17l10 5 10-5`) -- "Create private Guides"
2. **Phone icon** (rect 6,2.5 12x19 rx2.5 + line 10.5,18.5 to 13.5,18.5) -- "Import from social media"
3. **Infinity icon** (path: `M18.178 8c5.096 0 5.096 8 0 8-5.095 0-7.988-8-13.083-8-5.096 0-5.096 8 0 8 5.095 0 7.988-8 13.083-8z`) -- "Unlimited Guides"
4. **Sparkle icon** (path: `M12 3c.4 3.6 1.4 4.6 5 5-3.6.4-4.6 1.4-5 5-.4-3.6-1.4-4.6-5-5 3.6-.4 4.6-1.4 5-5Z`) -- "AI search"
5. **People icon** (circle cx8.5 cy8 r3 + path for body + second person outline) -- "Collaborate with friends"

**Pricing cards** (left 28px, top 561px, 314x123px):

- **Monthly card** (left 0, 144x123px, border-radius 14px):
  - Background image: `https://polo-pecan-73837341.figma.site/_assets/v11/ef4533e6536f2495088e56e0f98036b5ff15446d.png` (cover, centered)
  - Border: 1px solid `rgba(255,255,255,0.11)`
  - Inner padding: 14px 14px 15px 15px
  - "Monthly" label (13px, weight 400), "$20" price (19px, weight 500, margin-top 6px, letter-spacing 0.3px), "Billed Monthly" at bottom-left (12px)
  - All text white with `text-shadow: 0 1px 6px rgba(0,0,0,0.35)`

- **Yearly card** (left 154px, 160x123px, border-radius 14px):
  - Background: #1e212a, border: 1px solid `rgba(255,255,255,0.11)`
  - "Yearly" label + "Billed Yearly" in `rgba(255,255,255,0.50)`
  - "$200" price in `rgba(255,255,255,0.62)`
  - **Save badge** (left 15px, top 66px): inline-flex pill, padding 5px 8px, border-radius 11px, background #4d5057, text "Save $40.00" (10.5px, weight 600, color `rgba(255,255,255,0.65)`, letter-spacing 0.2px)

**Subscribe button** (left 28px, top 709px, 314x50px):
- White background, border-radius 26px
- "Subscribe" text: Helvetica Neue, 16px, weight 500, color #0c0c0e, -webkit-text-stroke 0.4px
- Right chevron SVG: 9x15, stroke #0c0c0e, stroke-width 2, path `M1.5 1.5 7 7.5 1.5 13.5`

---

### iOS Device Frame (reusable for both)

- Width: 370px, Height: 790px, Border-radius: 48px
- Light frame: background `#F2F2F7`; Dark frame: background `#000`
- Box-shadow: `0 40px 80px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.12)`
- Dynamic Island: 126x37px, border-radius 24px, black, centered at top 11px
- Status bar time: SF Pro weight 590, 17px, white
- Status bar icons: signal bars (4 rects), WiFi (3 arcs + dot), battery (rect + fill + nub) -- all white SVG
- Home indicator: 139x5px bar, border-radius 100px, `rgba(0,0,0,0.25)` on light / `rgba(255,255,255,0.7)` on dark

---

### Stage Layout

- Gap between phones: 70px
- Viewport padding: 40px
- Background: `#F4F4F4`
- Auto-scale JS: measures stage vs viewport, applies `transform: scale(min(1, fitRatio) * 0.95)` centered

---

### Entrance Animations

All paused until `.ze-ready` class is added to viewport (triggered when both videos fire `loadeddata` + fonts ready, or 5s timeout).

**Keyframes used:**
- `zeBgSettle`: scale 1.12 + opacity 0 to scale 1 + opacity 1 (1.7s, for backgrounds)
- `zeReveal`: translateY(26px) + scale(0.985) + blur(7px) + opacity 0 to normal (0.9s)
- `zeDrop`: translateY(-16px) + scale(0.90) + opacity 0 to normal (0.9s)
- `zeLine`: scaleX(0) to scaleX(1), transform-origin left (0.9s)
- `zePop`: translateY(8px) + scale(0.78) to bounce scale(1.07) to scale(1) (0.7s, spring easing)

**Easing:** `cubic-bezier(0.16, 1, 0.3, 1)` (expo-out). Save badge uses `cubic-bezier(0.34, 1.56, 0.64, 1)` (spring).

**Stagger (Screen 1):**
- Hero bg: 0s delay
- Logo (zeDrop): 0.45s
- Title (zeReveal): 0.62s
- Subtitle: 0.78s
- Button: 0.94s
- Terms: 1.06s

**Stagger (Screen 2):**
- Video bg (zeBgSettle): 0.12s delay
- Heading (zeReveal): 0.58s
- Divider (zeLine): 0.72s
- Row 1-5 (zeReveal): 0.80s, 0.88s, 0.96s, 1.04s, 1.12s
- Monthly card: 1.22s
- Yearly card: 1.30s
- Subscribe button: 1.42s
- Save badge (zePop): 1.55s

**Reduced motion:** All animations disabled via `@media (prefers-reduced-motion: reduce)`.

-

## Travel Explorer — Travel [apps/travel-explorer]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/natureweb3Area.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/travel-explorer.mp4

Build a React + Vite + TypeScript + Tailwind CSS mobile app mockup called "Roam Beyond Borders" -- a travel app displayed inside a realistic iPhone frame on a white background. It has TWO screens that switch via state: a Home screen and an Explore screen.

---

### SETUP REQUIREMENTS

**Tech stack:** React 18, Vite, TypeScript, Tailwind CSS, lucide-react for icons.

**Fonts (loaded in index.html):**
- Nomada Didone (serif heading font): `https://db.onlinewebfonts.com/c/5f01c59c653c8200fb4ec7f1a81d22ba?family=Nomada+Didone`
- Inter (body font): `https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap`

**Page title:** "Roam Beyond Borders"

---

### PHONE FRAME

- Centered on screen (`h-screen w-screen bg-white flex items-center justify-center overflow-hidden`)
- Frame dimensions: `width: 340px`, `height: 740px`, `border-radius: 52px`, `overflow: hidden`
- Frame shadow (class `phone-frame`):
```css
box-shadow:
  inset 0 0 0 2px rgba(255, 255, 255, 0.08),
  0 0 0 1px rgba(0, 0, 0, 0.6),
  0 0 0 10px #1a1a1e,
  0 0 0 11px rgba(255, 255, 255, 0.06),
  0 0 60px rgba(0, 0, 0, 0.3);
```
- **Dynamic Island:** Absolute positioned, `top-[12px]`, centered horizontally, `w-[126px] h-[37px] bg-black rounded-full z-50`

---

### ANIMATIONS (index.css)

```css
@keyframes fadeSlideUp {
  from { opacity: 0; transform: translateY(18px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-stagger { opacity: 0; animation: fadeSlideUp 0.6s ease-out forwards; }
.stagger-1 { animation-delay: 0.1s; }
.stagger-2 { animation-delay: 0.2s; }
.stagger-3 { animation-delay: 0.3s; }
.stagger-4 { animation-delay: 0.4s; }
.stagger-5 { animation-delay: 0.5s; }
.stagger-6 { animation-delay: 0.6s; }
.stagger-7 { animation-delay: 0.7s; }
```

Also in CSS:
```css
* { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
.font-heading { font-family: 'Nomada Didone', Georgia, serif; font-weight: normal; }
.font-inter { font-family: 'Inter', sans-serif; }
.no-scrollbar { -ms-overflow-style: none; scrollbar-width: none; }
.no-scrollbar::-webkit-scrollbar { display: none; }
```

---

### SCREEN 1: HOME SCREEN

Full-bleed background video with text overlay. Clicking the button transitions to Screen 2.

**Background video (absolute, covers frame):**
```
src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260705_060733_a39bb3eb-6b8a-4117-a7cc-0c6ceb74f1bb.mp4"
autoPlay muted loop playsInline
```

**Content overlay** (`relative flex-1 flex flex-col px-7 pt-20 pb-8 z-10`):

- **Heading** (stagger-1): Font: Nomada Didone, color `#01080A`, size `52px`, line-height `1.05`, tracking-tight, mb-5. Text:
  ```
  Roam
  Beyond
  Borders
  ```
  (Each word on its own line via `<br />`)

- **Subtext** (stagger-2): Color `#01080A/70`, size `15px`, leading-relaxed, max-w-[240px], font-inter. Text: "Uncover hidden gems and craft memories that last forever"

- **Spacer:** `flex-1`

- **CTA Button** (stagger-3, wrapped in `w-full px-2 pb-2`): `w-full bg-white text-gray-900 font-medium text-base py-4 rounded-full transition-transform active:scale-[0.98] hover:shadow-lg`. Text: "Begin Your Journey"

---

### SCREEN 2: EXPLORE SCREEN

Dark background (`bg-[#1C1C1C]`), scrollable content.

### Header (stagger-1, `px-6 pt-16 pb-8`)
- Left: Avatar (40x40 rounded-full) + text column
  - Avatar image: `https://images.pexels.com/photos/774909/pexels-photo-774909.jpeg?auto=compress&cs=tinysrgb&w=100`
  - Small text: "Welcome back," (`text-white/60 text-xs font-inter`)
  - Name: "Elena Castillo" (`text-white text-[22px] font-heading`)
- Right: Bell icon (lucide-react, size 20, white)

### Search Bar (stagger-2, `px-6 mb-3`)
- Container: `bg-[#333333] rounded-full pl-4 pr-0 py-0`, flex row
- Placeholder text: "Search dream destinations" (`text-white/40 text-sm font-inter`)
- Right icon circle: `w-9 h-9 bg-[#979797] rounded-full` with Search icon (size 16, color `#1C1C1C`)

### Filter Chips (stagger-3, `mb-7`, horizontally scrollable, no-scrollbar)
- Container: `flex gap-2 px-6`
- Chips: `px-4 py-2 bg-[#333333] text-white/80 text-xs font-inter rounded-full`
- Labels: "Top Picks Now", "Quick Escapes", "South America", "Europe"

### Section Header (stagger-4, `px-6 mb-4`)
- Left: "Destinations" (`font-heading text-white text-[22px]`)
- Right: "View all" (`text-white/50 text-xs font-inter`)

### Horizontal Cards (stagger-5, flex-1, overflow-x-auto, no-scrollbar)
- Container: `flex gap-4 px-6 h-full pb-4`
- Each card: `w-[240px] h-[310px] rounded-3xl overflow-hidden`, position relative

**Card 1 - Ireland:**
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_153452_6ac31a99-2fe2-46aa-8d3c-edcd3fc0ac9a.png&w=1280&q=85`
- Price: "From $3,200"
- Description: "Emerald meadows, rugged coastal cliffs, and folk tunes drifting from pubs"
- Has "Curated" badge: YES

**Card 2 - Norway:**
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_154229_5be57df0-6fc9-49ba-840e-62f1b686c7f5.png&w=1280&q=85`
- Price: "From $4,100"
- Description: "Majestic fjords, aurora skies, and peaceful seaside hamlets"
- Has "Curated" badge: NO

**Card 3 - Japan:**
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_154713_941acc2f-d44d-4473-87fd-13b46129423b.png&w=1280&q=85`
- Price: "From $3,800"
- Description: "Sacred shrines, blooming sakura, and neon-lit streets woven into tradition"
- Has "Curated" badge: YES

**Card 4 - Iceland:**
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_154824_77eb9d80-7654-47e2-baf1-89adb5c2f094.png&w=1280&q=85`
- Price: "From $4,500"
- Description: "Vast ice caps, erupting hot springs, and surreal terrain under endless sky"
- Has "Curated" badge: NO

**Card overlay styling:**
- Bottom gradient: `absolute bottom-0 left-0 right-0 h-[50%] rounded-b-3xl backdrop-blur-sm bg-gradient-to-t from-black/40 via-black/20 to-transparent` with mask: `linear-gradient(to top, black 60%, transparent 100%)`
- "Curated" badge (top-right): `bg-white/90 backdrop-blur-md rounded-lg px-2 py-1.5` with BadgeCheck icon (size 12) + text `text-gray-800 text-[10px] font-inter font-semibold`
- Bottom info (`absolute bottom-0 p-5`):
  - Country name: `text-white font-heading text-xl`
  - Price: `text-white text-[11px] font-inter font-medium`
  - Description: `text-white/70 text-[11px] leading-snug font-inter`

### Bottom Navigation (stagger-6, `flex justify-center pb-8 pt-3`)
- Pill container: `bg-[#333333] rounded-full p-2 flex items-center gap-6`
- Active tab (Home): `w-9 h-9 bg-white rounded-full` with Home icon (size 18, black)
- Inactive tabs: `w-9 h-9` with icons Compass, Heart, User (size 18, `text-white/40`)

---

### ICONS USED (all from lucide-react)
Search, Bell, Home, Compass, Heart, User, BadgeCheck

## Travel Journal — Travel [apps/travel-journal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/balitravel.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/travel-journal.mp4

Create a mobile travel app UI mockup displayed inside a realistic iPhone-style phone frame, centered on a white webpage. Tech stack: React + TypeScript, Tailwind CSS, Lucide React icons, Vite. Font: Google Fonts "Inter" (weights 300-900).

---

**PAGE WRAPPER:**

Full viewport (`h-screen w-screen`), solid white background, flexbox centered, overflow hidden.

---

**PHONE FRAME:**

- Dimensions: 375px x 780px
- Background: `#0a0a0c`
- Border-radius: 52px
- Overflow: hidden
- Box-shadow (layered bezel effect):
  ```
  inset 0 0 0 2px rgba(255, 255, 255, 0.08),
  0 0 0 1px rgba(0, 0, 0, 0.6),
  0 0 0 10px #1a1a1e,
  0 0 0 11px rgba(255, 255, 255, 0.06),
  0 0 60px rgba(0, 0, 0, 0.5)
  ```

**Dynamic Island notch:** Absolute, top-0, centered horizontally, 120px wide, 28px tall, solid black, `rounded-b-2xl`, z-50.

---

**HEADER (absolute, top):**

- Z-index: 30
- `backdrop-filter: blur(12px)` with `background-color: rgba(10, 10, 12, 0.75)`
- Padding: `px-6 pt-14 pb-4`
- Left side: Button with "Asia" text (text-lg, font-semibold, white) + ChevronDown icon (size 18, text-white/70)
- Right side: Calendar icon button (size 22, text-white/70)

---

**SCROLLABLE CONTENT:**

- `overflow-y-auto` with hidden scrollbar
- Padding: `px-6 pt-28 pb-24`
- Vertical spacing: `space-y-4` (16px gap)

---

**DESTINATION CARDS (4 total):**

Each card:
- Full width, height 200px, rounded-2xl, overflow hidden, position relative
- Full-bleed background image (`absolute inset-0, object-cover`)
- Gradient overlay: `bg-gradient-to-t from-black/20 to-transparent`
- **Top-left:** liquid-glass pill (`rounded-full px-3 py-1`) with "{N} moments" text (text-white/90, text-xs, font-normal)
- **Top-right:** liquid-glass circle (w-8 h-8, rounded-full) with MoreHorizontal icon (size 16, text-white/80)
- **Bottom:** Destination name in 96px bold font, centered horizontally, clipped in an 80px tall container with `overflow: hidden`. Color: `rgba(255, 255, 255, 0.55)`, tracking-tight, leading-none, margin-top 2px

**Card data with exact image URLs:**

1. **Tokyo** - 23 moments
   `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_101902_e8f0f37b-18b7-4c14-bb5c-99f0724d2646.png&w=1280&q=85`

2. **Seoul** - 18 moments
   `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_101935_4b17f250-8ddb-4ff2-b63d-dfd3497d4428.png&w=1280&q=85`

3. **Bali** - 29 moments
   `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_101958_7116d6bf-fd6f-496f-b3cf-007688cd5123.png&w=1280&q=85`

4. **Rome** - 15 moments
   `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_143008_72ee7299-04a8-474c-ae73-220d45b24a20.png&w=1280&q=85`

---

**FLOATING BOTTOM NAV BAR:**

- Absolute, bottom-6, centered horizontally, z-30
- liquid-glass pill (`rounded-full, flex, gap-6, px-6 py-2`)
- 3 nav items (flex-col, items-center, gap-0.5):
  - "Feed" + Home icon (size 20) -- inactive (`text-white/50`)
  - "Account" + User icon (size 20) -- inactive (`text-white/50`)
  - "Trips" + FileText icon (size 20) -- active (`text-white`)
- Label style: `text-[10px] font-medium`

---

**LIQUID-GLASS CSS:**

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

**STAGGERED ENTRANCE ANIMATION:**

Each card starts at `opacity: 0` and `transform: translateY(24px)`. On component mount, a `setTimeout` triggers a state change that transitions to `opacity: 1` and `translateY(0)`. Transition: `transition-all duration-700 ease-out`. Stagger delay: first card 150ms, each subsequent +120ms (150, 270, 390, 510ms). Implemented with React `useState` + `useEffect`.

---

**SCROLLBAR HIDING:**

```css
.scrollbar-hide::-webkit-scrollbar { display: none; }
.scrollbar-hide { -ms-overflow-style: none; scrollbar-width: none; }
```

---

**GLOBAL:**

```css
* { font-family: 'Inter', sans-serif; }
```

Load in HTML head:
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

## Luxury Escapes — Travel App [apps/luxury-escapes]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/traveltobalilight.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/luxury-escapes.mp4

Build a React + Vite showcase displaying two mobile app screens side by side inside realistic iPhone device frames, presented on a cinematic gradient background. No additional npm dependencies beyond React and ReactDOM.

---

### SETUP

- Vite + React (no TypeScript)
- Google Font: Anton (loaded via `<link href="https://fonts.googleapis.com/css2?family=Anton&display=swap">` in index.html)
- Local font: "Linotype Projekt" loaded from `/public/fonts/linotype-projekt-regular.woff2` and `.woff` (not used in these two screens but present in the project)
- No Tailwind, no CSS modules -- all styles are inline JSX with a minimal global `styles.css` for reset and keyframe animations

---

### BACKGROUND / VIEWPORT (App.jsx)

Full-viewport container (`100vw x 100vh`, `overflow: hidden`) with:
- Background: `radial-gradient(120% 90% at 18% 8%, #FBEFDD 0%, #F3D9BE 22%, #E1A98C 42%, #9C6E8F 62%, #4B4470 80%, #23274A 100%)`
- Two decorative radial-gradient orbs (pointer-events: none):
  - Top-left: 900x900px circle, `rgba(255,225,180,0.55)` fading to transparent, positioned `top: -320, left: -220`, `filter: blur(10px)`
  - Bottom-right: 700x700px circle, `rgba(60,90,150,0.45)` fading to transparent, positioned `bottom: -260, right: -180`, `filter: blur(10px)`
- Center-aligned flex container with `gap: 70px` holding both screens
- Auto-scaling logic: on mount and resize, measure the stage's natural dimensions and scale it down (never up) to fit within `(viewport - 40px)` padding using CSS transform with `transform-origin: center center`

---

### IOS DEVICE FRAME (IOSDevice component)

Dimensions: `370px wide x 790px tall`, `border-radius: 48px`
- `box-shadow: 0 40px 80px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.12)`
- Dynamic Island: absolute positioned black pill, `width: 126px, height: 37px, border-radius: 24px`, centered horizontally at `top: 11px`, `z-index: 50`
- Home indicator bar: absolute bottom, centered, `width: 139px, height: 5px, border-radius: 100px`, z-index 60
  - Light mode: `rgba(0,0,0,0.25)` | Dark mode: `rgba(255,255,255,0.7)`
- Status bar (IOSStatusBar): shows "11:11" time on left (font: `-apple-system, "SF Pro", system-ui`, weight 590, size 17px) and signal/wifi/battery SVG icons on right. Color adapts to `dark` prop (white vs black).
- Props: `dark` (boolean) -- sets background to `#000` when true, `#F2F2F7` when false

---

### SCREEN 1: OFFER TEASER (Light mode device)

**Layout:** Full-height flex column, `background: #f4f4f4`, padding `66px 14px 14px`

**Headline block** (top):
- Font: `'Anton', sans-serif`, size 69px, weight 900, line-height 0.94, letter-spacing 0.5, color `#2c2c2c`, uppercase, centered
- Three lines: "Experience" / "Unparalleled" / "Luxury"
- Each line wrapped in `overflow: hidden` container with inner span animated via `zeRise` keyframe (0.9s, cubic-bezier(0.22,1,0.36,1)) with staggered delays: 0.10s, 0.22s, 0.34s

**Video card** (fills remaining space):
- `flex: 1`, `border-radius: 26px`, `overflow: hidden`, `margin: 0 8px`
- Animated in with `zeCardReveal` (1.1s, delay 0.45s)
- Background video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260707_004919_5e1b7e08-d723-4ecb-8afe-d613d730984c.mp4`
  - Attributes: autoPlay, loop, muted, playsInline
  - Style: absolute fill, `object-fit: cover`, `filter: saturate(0.84) contrast(1.05)`
- Blurred duplicate video layer (aria-hidden): same video, `filter: blur(16px) saturate(1.15)`, `transform: scale(1.08)`, masked with vertical gradient (`mask-image: linear-gradient(180deg, transparent 0%-48%, 0.35 at 62%, 0.85 at 78%, 1 at 92%)`)
- Warm overlay: `rgba(122,107,82,0.21)` solid
- Green gradient overlay: `linear-gradient(180deg, rgba(40,70,35,0) 0%-42%, rgba(45,80,40,0.18) 60%, rgba(35,65,32,0.4) 78%, rgba(28,52,26,0.55) 100%)`
- 4-pointed star logo (SVG, white, 22x22, viewBox 0 0 1024 1024): positioned absolute top-left (20px, 20px), animated with `zeBloom` (0.9s, delay 1.05s)
  - Path: `M87 116C260 108 408 168 512 300C616 168 764 108 937 116C945 289 885 437 753 541C885 645 945 793 937 966C764 974 616 914 512 782C408 914 260 974 87 966C79 793 139 645 271 541C139 437 79 289 87 116Z`
- Bottom content (absolute, padding `22px 22px 24px`, flex column, gap 10):
  - Title: "Bali Exclusive\nLuxury Getaway" -- Helvetica 34px, weight 500, line-height 1.12, white, letter-spacing -0.2, animated `zeFadeUp` (0.85s, delay 0.80s)
  - Subtitle: "Breathtaking locations, bespoke services, with a focus on exclusivity" -- 13.5px, line-height 1.45, `rgba(255,255,255,0.88)`, max-width 250, animated `zeFadeUp` (delay 0.92s)
  - CTA pill button: animated `zePillPop` (0.75s, delay 1.10s)
    - Gradient background: `linear-gradient(90deg, #FAD5D7 0%-38%, #FFFFFF 50%, #9CE2F9 62%-100%)`
    - `border-radius: 999px`, padding `7px 7px 7px 14px`, `box-shadow: 0 6px 18px rgba(0,0,0,0.25)`
    - Label "VIEW OFFER": Helvetica 10px, weight 750, letter-spacing 0.6, color `#1a1a2e`
    - Eye icon SVG (12.5x12.5): stroke `#1a1a2e`, strokeWidth 2

---

### SCREEN 2: GETAWAY DETAIL (Dark mode device)

**SVG filter** (hidden, 0x0): Custom color grading matrix applied to the video:
```
values="0.6666 -0.0742 0.0785 0 0.1499 -0.0627 0.7320 0.0649 0 0.0943 -0.0701 0.1109 0.7276 0 0.0471 0 0 0 1 0"
```

**Background video:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260707_004833_4cc93fa3-27f5-4cec-b1b2-0b4fb073c13a.mp4`
- Attributes: autoPlay, loop, muted, playsInline
- Style: absolute fill, `object-fit: cover`, `filter: url(#grade)`, `transform: translate(0%, -4%) scale(1.07)`

**Top gradient vignette:** `linear-gradient(to bottom, rgba(8,12,16,0.30), transparent)`, height 22%

**Progressive radial blur system** (6 layered divs, all pointer-events: none):
Each uses `backdrop-filter: blur(Npx)` with a `radial-gradient(circle at 55% 115%, black Rpx, transparent R+65px)` mask. From outermost to innermost:
1. blur(1px), black 480px / transparent 545px
2. blur(1.5px), black 440px / transparent 505px
3. blur(3px), black 390px / transparent 455px
4. blur(5px), black 320px / transparent 395px
5. blur(7px), black 240px / transparent 325px
6. blur(9px), black 130px / transparent 235px

**Color atmosphere overlay:**
`linear-gradient(to top, rgba(110,160,195,0.30) 0%, rgba(110,160,195,0.19) 7%, rgba(112,160,192,0.07) 16%, transparent 34%), linear-gradient(285deg, rgba(240,225,205,0.18) 0%, rgba(240,225,205,0.09) 24%, transparent 48%), radial-gradient(ellipse 60% 30% at 0% 76%, rgba(5,25,75,0.20), transparent 70%)`

**Content overlay** (absolute, flex column, padding `64px 24px 48px`):

- **Nav bar** (animated `zeFadeDown`, 0.7s, delay 0.25s):
  - Left: back arrow SVG (rotated 90deg, white, strokeWidth 2.2)
  - Right: hamburger menu (two white bars, 18x2px, gap 5px)

- **Title block** (centered, marginTop 46):
  - "BALI EXCLUSIVE" / "LUXURY GETAWAY": Anton, 52px, weight 900, line-height 1.0, letter-spacing 0.5, white, uppercase, text-shadow `0 2px 18px rgba(0,0,0,0.35)`
  - Each line: `zeRise` (0.95s) with delays 0.35s / 0.48s
  - "by ZENITH ESCAPES": "by" in Helvetica 13px bold `rgba(255,255,255,0.85)`, "ZENITH ESCAPES" in Anton 22px weight 900, letter-spacing 1, white. Animated `zeFadeUp` (0.8s, delay 0.70s)

- **Center logo**: Same 4-pointed star SVG (30x30, white), `drop-shadow(0 2px 10px rgba(0,0,0,0.35))`, animated `zeBloom` (1.0s, delay 0.95s), centered with paddingTop 40

- **Bottom stats** (marginTop auto, flex column, gap 20):
  - Grid (2 columns):
    - "Exclusive" label (Helvetica 11px, bold, `rgba(255,255,255,0.75)`) + "8 GUESTS" (Anton 33px, white) -- animated `zeFadeUp` (delay 0.85s)
    - "Availability" label + "12 DAYS" (same style, marginLeft 48) -- animated `zeFadeUp` (delay 0.98s)
  - Description row (flex, align flex-end, gap 16):
    - Text: "Escape to an elite getaway, where every detail is meticulously designed to meet the highest expectations of luxury and serenity." -- Helvetica 12.5px, line-height 1.55, `rgba(255,255,255,0.92)`, max-width 270, animated `zeFadeUp` (delay 1.12s)
    - Down arrow SVG (18x14, white, strokeWidth 2.2, marginLeft 22) -- animated `zeArrowDrop` (0.7s, delay 1.35s)

---

### KEYFRAME ANIMATIONS (styles.css)

```css
@keyframes zeRise {
  from { opacity: 0; transform: translateY(110%); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes zeFadeUp {
  from { opacity: 0; transform: translateY(26px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes zeFadeDown {
  from { opacity: 0; transform: translateY(-14px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes zeCardReveal {
  from { opacity: 0; transform: translateY(34px) scale(0.965); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
@keyframes zeBloom {
  0% { opacity: 0; transform: scale(0.2) rotate(-90deg); }
  60% { opacity: 1; transform: scale(1.12) rotate(8deg); }
  100% { opacity: 1; transform: scale(1) rotate(0deg); }
}
@keyframes zePillPop {
  0% { opacity: 0; transform: translateY(14px) scale(0.85); }
  65% { opacity: 1; transform: translateY(-2px) scale(1.03); }
  100% { opacity: 1; transform: translateY(0) scale(1); }
}
@keyframes zeArrowDrop {
  0% { opacity: 0; transform: translateY(-10px); }
  55% { opacity: 1; transform: translateY(3px); }
  100% { opacity: 1; transform: translateY(0); }
}
```

All animations use `animation-fill-mode: both` and `cubic-bezier(0.22, 1, 0.36, 1)` easing (smooth overshoot).

---

### GLOBAL CSS RESET

```css
html, body { margin: 0; height: 100%; overflow: hidden; }
#root { height: 100%; }
```

## Mood Tracker — Wellness [apps/mood-tracker]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobile%20apps/mobilenature33.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/mood-tracker.mp4

Create a wellness/mental health app mockup showing 3 iPhone screens side-by-side in realistic phone frames. The app is called "Soul Canvas". Use React with Tailwind CSS and lucide-react icons. Use Vite + TypeScript.

---

### TECHNICAL SETUP

- **Font**: "Helvetica Neue ME" loaded from `https://db.onlinewebfonts.com/c/95cecf452d3208890088a5b4c19c7ecf?family=Helvetica+Neue+ME` (add in index.html `<head>`). Set as `font-inter` in Tailwind config mapped to `'Helvetica Neue ME', sans-serif`.
- **Icons**: lucide-react (Search, Home, Clock, LayoutGrid, Plus, ArrowRight, Info, ChevronLeft, Sun, Activity, Users, Moon, Minus)
- **Dependencies**: react, react-dom, lucide-react, tailwindcss

---

### GLOBAL CSS (index.css)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  font-family: 'Helvetica Neue ME', sans-serif;
  margin: 0;
  padding: 0;
}

@keyframes fadeSlideUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.stagger-item {
  opacity: 0;
  animation: fadeSlideUp 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

.stagger-delay-1 { animation-delay: 0.1s; }
.stagger-delay-2 { animation-delay: 0.2s; }
.stagger-delay-3 { animation-delay: 0.35s; }
.stagger-delay-4 { animation-delay: 0.5s; }
.stagger-delay-5 { animation-delay: 0.65s; }
.stagger-delay-6 { animation-delay: 0.8s; }
.stagger-delay-7 { animation-delay: 0.95s; }
.stagger-delay-8 { animation-delay: 1.1s; }
```

---

### PAGE LAYOUT

- White background (`bg-white`), `min-h-screen`, flexbox centered.
- Direction: `flex-col` on mobile, `flex-row` on md+.
- Gap: `gap-6 md:gap-8`.
- `overflow-x-auto md:overflow-x-visible` for horizontal scroll on mobile.
- `p-4 py-8 md:p-4`.

---

### PHONE FRAME (shared for all 3)

Each phone is:
- `w-[290px] h-[700px] md:w-[390px] md:h-[800px]`
- `rounded-[45px] md:rounded-[60px]`
- `bg-black shadow-2xl border-[9px] md:border-[12px] border-neutral-900 overflow-hidden flex-shrink-0`
- Contains a **Dynamic Island**: `absolute top-3 left-1/2 -translate-x-1/2 w-[126px] h-[34px] bg-black rounded-full z-50`
- Contains an **iOS Status Bar** (absolute top, z-50): time "9:41" on left, signal bars + WiFi SVG + battery on right.
- Each has a looping **background video** (`absolute inset-0 w-full h-full object-cover`, autoPlay, muted, loop, playsInline).
- Each has a **color overlay** div (`absolute inset-0`) with a specific color at 40% opacity.
- Content container: `relative h-full flex flex-col px-6 pt-16 pb-2` (NO overflow-hidden on this div -- important for backdrop-filter to work).

---

### IMPORTANT: BACKDROP-FILTER RULE

When using the `stagger-item` animation class (which uses `transform`), NEVER put it on a parent wrapper of elements that have `backdrop-blur-xl`. The transform creates a new stacking context that breaks backdrop-filter on children. Instead, put `stagger-item` directly on the element that also has `backdrop-blur-xl`.

---

### SCREEN 1: HOME / DAILY CHECK-IN

**Video**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260705_143518_88285de9-3f05-4256-9e49-025f75cb6bcb.mp4`
**Overlay**: `bg-[#4C5930]/40`
**Status bar**: White text/icons.

**Elements top-to-bottom:**

1. **User Profile Pill + Search Button** (`stagger-item stagger-delay-1`, `mb-5`)
   - Left: Pill with avatar + "Lena Voss". Pill: `bg-white/20 backdrop-blur-xl rounded-full pl-0.5 py-0.5 pr-4`. Avatar: `https://images.pexels.com/photos/1239291/pexels-photo-1239291.jpeg?auto=compress&cs=tinysrgb&w=100`, `w-9 h-9 rounded-full object-cover`. Name: `text-white text-sm font-medium`.
   - Right: Search button `w-10 h-10 rounded-full bg-white/20 backdrop-blur-md`, Search icon size 18.

2. **Header Text** (`stagger-item stagger-delay-2`, `mb-1`)
   - Subtitle: `text-white/70 text-sm font-medium mb-1` -- "Morning ritual"
   - Title: `text-white text-[38px] leading-[1.1] font-normal tracking-[-0.03em]` -- "What is\nyour inner\nworld?"

3. **Section Header + Toggle** (`stagger-item stagger-delay-3`, `mt-4 mb-4`)
   - Left: `text-white text-lg font-semibold` -- "Your calm\nsnapshot"
   - Right: Toggle pill `bg-white/20 backdrop-blur-md rounded-full p-1`. Active tab: `px-4 py-1.5 bg-white rounded-full text-sm font-medium text-neutral-800` ("Today"). Inactive: `px-4 py-1.5 text-sm font-medium text-white/80` ("Week").

4. **"Set your vibe" Card** (`stagger-item stagger-delay-4`)
   - Container: `bg-white/20 backdrop-blur-xl rounded-3xl p-5 mb-3 border border-white/20`
   - Top row: "Set your vibe" (`text-white font-medium text-base`) + Info button (`w-7 h-7 rounded-full bg-white/30`, Info icon size 14).
   - Bottom row: Italic text `text-white/60 text-sm italic leading-snug max-w-[200px]` -- "Share here what's in\nyour heart right now" + White circle button `w-12 h-12 rounded-full bg-white shadow-lg` with Plus icon (size 20, `text-neutral-700`).

5. **Bottom Cards Grid** (`grid grid-cols-2 gap-3` -- NO stagger-item on the grid wrapper)
   - Each card: `bg-white/20 backdrop-blur-xl rounded-3xl p-5 border border-white/20 flex flex-col justify-between aspect-square stagger-item stagger-delay-5`
   - Card 1: "Reflect on\nyour day" (`text-white font-medium text-base leading-snug`) + Plus button bottom-right.
   - Card 2: "See your\npatterns" + ArrowRight button bottom-right.
   - Buttons: Same white circle style as above.

6. **Spacer** (`flex-1`)

7. **Bottom Navigation** (`stagger-item stagger-delay-6`, `py-3`, `justify-around`)
   - Home icon (white, size 24) with dot below (`w-1 h-1 rounded-full bg-white`).
   - Clock icon (`text-white/50`, size 24).
   - LayoutGrid icon (`text-white/50`, size 24).

8. **Home Indicator**: `w-32 h-1 bg-white rounded-full`, centered, `pb-2`.

---

### SCREEN 2: INSIGHTS / PATTERNS

**Video**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260705_141850_7f43c95b-a2cd-4586-ae77-86a10221b9e1.mp4`
**Overlay**: `bg-[#4A3968]/40`
**Status bar**: White text/icons.

**Elements top-to-bottom:**

1. **Top Bar** (`stagger-item stagger-delay-1`, `mb-6`)
   - Left: Back button `w-10 h-10 rounded-full bg-[#4A3968]/25 backdrop-blur-xl`, ChevronLeft icon size 20.
   - Right: Name pill `bg-[#4A3968]/25 backdrop-blur-xl rounded-full pl-4 py-0.5 pr-0.5`. Text "Lena Voss" + avatar (same Pexels URL).

2. **Header** (`stagger-item stagger-delay-2`, `mb-2`)
   - Title: `text-white text-[38px] leading-[1.1] font-normal tracking-[-0.03em] mb-2` -- "Your patterns"
   - Subtitle: `text-white/70 text-lg leading-snug` -- "Factors shaping\nyour state"

3. **Insight Cards** (`flex flex-col gap-3 mt-6`) -- each card gets its own stagger class:
   - Each card: `bg-[#4A3968]/25 backdrop-blur-xl rounded-3xl px-5 py-5 border border-white/20 flex items-center justify-between`
   - Left side: Icon (size 20, white) + label (`text-white font-medium text-[15px]`)
   - Right side: Percentage (`font-semibold text-lg`) + description (`text-white/60 text-xs`)

   Card data:
   - Sun icon, "Early sun exposure", `+28%` (color `#CBD89E`), "Boosts wellness" -- `stagger-delay-3`
   - Activity icon, "Regular movement", `+22%` (color `#CBD89E`), "Boosts wellness" -- `stagger-delay-4`
   - Users icon, "Social gathering", `+18%` (color `#CBD89E`), "Boosts wellness" -- `stagger-delay-5`
   - Moon icon, "Low rest", `+18%` (color `#F5B5B6`), "Drains wellness" -- `stagger-delay-6`

4. **Spacer** (`flex-1`)

5. **CTA Button** (`stagger-item stagger-delay-7`, `mb-4`)
   - `w-full bg-white rounded-2xl py-4 px-6 flex items-center justify-between shadow-lg`
   - Text: "Explore suggestions" (`text-neutral-800 font-medium text-base`)
   - ArrowRight icon (size 20, `text-neutral-700`)

6. **Home Indicator**: Same as screen 1.

---

### SCREEN 3: MOOD LOG / CHECK-IN FORM

**Video**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260705_142807_689d92ab-24f9-419b-a340-76863735ff03.mp4`
**Overlay**: `bg-[#7f7160]/40`
**Status bar**: Dark text/icons (`text-neutral-700`, `bg-neutral-700` for bars/battery).

**State**: `selectedMood` (default 'well'), `notes` (string), `energyLevel` (default 4).

**Elements top-to-bottom:**

1. **Top Bar** (`stagger-item stagger-delay-1`, `mb-6`)
   - Left: Back button `w-10 h-10 rounded-full bg-[#7f7160]/25 backdrop-blur-xl`, ChevronLeft.
   - Right: Name pill `bg-[#7f7160]/25 backdrop-blur-xl rounded-full pl-4 py-0.5 pr-0.5` + avatar.

2. **Title** (`stagger-item stagger-delay-2`, `mb-5`)
   - `text-white text-[28px] leading-[1.2] font-normal tracking-[-0.02em]` -- "What describes you?"

3. **Mood Selection Row** (`stagger-item stagger-delay-3`, `mb-6 px-1`, `justify-between`)
   - 4 mood buttons (Low, Worried, Settled, Well), each with a custom SVG face (64x64 viewBox):
   - Unselected: Circle stroke white, eyes white filled, mouth white stroke.
   - Selected: Circle filled white (no stroke), eyes `#6B6B6B`, mouth `#6B6B6B`.
   - Face expressions:
     - **Low**: Eyes at (22,26) and (42,26) r=3. Mouth: deep frown path `M20 44c3-5 8-8 12-8s9 3 12 8`.
     - **Worried**: Same eyes. Mouth: slight frown `M22 42c2.5-3 6-5 10-5s7.5 2 10 5`.
     - **Settled**: Same eyes. Mouth: straight line `M22 40h20`.
     - **Well**: Same eyes. Mouth: smile `M20 38c3 5 8 8 12 8s9-3 12-8`.
   - Labels below: `text-white text-xs`, `font-semibold` when selected, `font-medium` when not.

4. **"Jot your thoughts"** heading (`stagger-item stagger-delay-4`, `text-white text-[22px] leading-[1.2] font-normal tracking-[-0.02em] mb-3`)

5. **Notes Textarea** (`stagger-item stagger-delay-5`)
   - Container: `bg-[#7f7160]/20 backdrop-blur-xl rounded-2xl p-5 border border-white/30 mb-6`
   - Textarea: `w-full bg-transparent text-white text-[15px] leading-relaxed placeholder-white/50 resize-none outline-none min-h-[80px]`, rows=3.
   - Placeholder: "Tell us your day?\nWhat was the thing\nthat shifted your mood?"

6. **"Vitality Meter"** heading (`stagger-item stagger-delay-6`, `text-white text-[22px] leading-[1.2] font-normal tracking-[-0.02em] mb-4`)

7. **Energy Level Bars** (`stagger-item stagger-delay-7`, `flex items-stretch gap-2 mb-4`)
   - Minus button: `w-10 h-20 rounded-full bg-[#C5B9AA]`, Minus icon size 16.
   - 5 bars (`flex-1 flex items-end gap-1.5`): Each bar `flex-1 h-20 rounded-xl`. Filled (level <= energyLevel): `bg-white shadow-sm`. Empty: `border-2 border-white/40 bg-white/20`.
   - Plus button: `w-10 h-20 rounded-full bg-[#C5B9AA]`, Plus icon size 16.

8. **Spacer** (`flex-1`)

9. **Done Button** (`stagger-item stagger-delay-8`, `mb-4`)
   - Same style as "Explore suggestions" button but text says "Done".

10. **Home Indicator**: Same white bar.

---

### PROFILE IMAGE (used across all 3 screens)

`https://images.pexels.com/photos/1239291/pexels-photo-1239291.jpeg?auto=compress&cs=tinysrgb&w=100`

## Wellness Companion — Wellness [apps/wellness-companion]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/mobileArea.mp4
- Asset: https://code.mrday.one/design-assets/apps/visuals-by-id/wellness-companion.mp4

Build a mobile wellness quiz screen inside a realistic phone frame mockup, centered on a white page. Use React with Tailwind CSS and Lucide React icons.

**Phone Frame:**
- Dimensions: 375px wide x 780px tall
- Border radius: 52px
- Background color: `#8a9aaa`
- Box shadow to simulate a real phone bezel: `inset 0 0 0 2px rgba(255,255,255,0.08), 0 0 0 1px rgba(0,0,0,0.6), 0 0 0 10px #1a1a1e, 0 0 0 11px rgba(255,255,255,0.06), 0 0 60px rgba(0,0,0,0.5)`
- A black pill-shaped Dynamic Island at the top center: 120px wide, 32px tall, fully rounded, z-index 50

**Background:**
- Full-bleed background image using this exact URL: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260704_143500_a76b8e64-2c69-4683-80e7-2bb060a921d6.png&w=1280&q=85`
- Apply `blur(12px)` and `scale(1.1)` to the background image
- Semi-transparent overlay: `#8a9aaa` at 30% opacity on top

**Font:**
- Load "Helvetica Now Var" from: `https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var`
- Fallback stack: 'Helvetica Neue', Helvetica, Arial, sans-serif
- Apply globally to all elements

**Content Layout (flex column, padding: 56px top, 24px sides, 24px bottom):**

1. **Header Badge** (top, with 40px margin-bottom):
   - Liquid glass pill with Timer icon (12px, white/80) + text "Vitaforge Daily" (12px, white/90, medium weight)
   - Padding: 10px vertical, 12px horizontal

2. **Title Section** (32px margin-bottom):
   - Subtitle: "Choose all that apply" - white/60, 14px
   - Heading: "What aspects of your wellness would you like to boost?" - white, 28px, normal weight, tight leading and tracking

3. **Selection Grid** (2 columns, 12px gap, pushes to fill available space):
   - 4 cards: "Sleep quality", "Stress", "Weight", "Skin"
   - Each card: rounded-[32px], 100px height, padding 16px
   - Shows a number label (01, 02, etc.) in white/50, 11px, medium weight
   - Option text in white, 16px, medium weight
   - "Stress" (id:2) and "Skin" (id:4) are pre-selected
   - Cards are toggleable on click

4. **Voice Button** (centered, 24px vertical margin):
   - Yellow/gold radial glow behind: `radial-gradient(ellipse at center, rgba(220,200,80,0.5) 0%, rgba(180,160,40,0.2) 40%, transparent 70%)`
   - 64px circular liquid glass button with a waveform SVG icon (white strokes, strokeWidth 2, strokeLinecap round) showing 5 vertical bars of varying heights
   - "voice" label below in white/70, 12px

5. **Slide-to-Confirm Button** (bottom, inside 24px horizontal padding):
   - Full-width rounded-full track, 56px tall, liquid glass style
   - White circular thumb (44px) on the left with ArrowRight icon (gray-800)
   - "Done" text centered in white/60, 14px, medium weight
   - 3 ChevronRight icons on the right (14px) at white/40, white/50, white/60 opacity
   - Draggable thumb with pointer events: snaps back if not dragged past 85%, snaps to end if past 85%

**Liquid Glass Effect (CSS classes):**

`.liquid-glass`:
- Background: `rgba(255,255,255,0.01)` with luminosity blend mode
- Backdrop filter: `blur(4px)`
- Box shadow: `inset 0 1px 1px rgba(255,255,255,0.1)`
- `::before` pseudo-element for gradient border: `linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)` with 1.4px padding and mask-composite exclude technique

`.liquid-glass-selected`:
- Same as above but background: `rgba(255,255,255,0.12)`, blur 8px, stronger box shadow (`inset 0 1px 2px rgba(255,255,255,0.2)`), and brighter gradient border (0.6 alpha at edges, 0.25 at 20%/80%)

**Animations:**
- Staggered fade-up animation on all elements
- Keyframes: from `opacity:0, translateY(16px)` to `opacity:1, translateY(0)`
- Duration: 0.5s, easing: `cubic-bezier(0.22, 1, 0.36, 1)`, fill: forwards
- Delays: header 0.1s, title 0.25s, grid cards 0.4s/0.48s/0.56s/0.64s, voice 0.7s, slider 0.85s

**Dependencies:**
- React 18, Tailwind CSS 3, Lucide React, Vite, TypeScript
