# Michael Design Library — sites-agency-portfolio

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 17 entries.

## New Era Bold Hero — Agency [sites/1]

- Preview: https://motionsites.ai/assets/hero-new-era-preview-CocuDUm9.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/1.gif

Create a responsive, full-screen Hero section using React and Tailwind CSS with the following specifications:

1. Layout & Positioning:

Set the container to at least screen height (min-h-screen) with a dark blue fallback background (#21346e).
Align the main content to the top of the page (not centered), adding significant top padding (approx pt-32 on mobile, pt-48 on desktop).
Use a standard container with horizontal padding.

2. Background Video:

Implement a full-screen, absolute-positioned background video.
The video must be set to autoPlay, loop, muted, and playsInline.
Use object-cover to ensure it fills the screen without distortion.
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260206_044704_dd33cb15-c23f-4cfc-aa09-a0465d4dcb54.mp4

3. Typography (Main Headline):

Font Family: Rubik (sans-serif).
Style: Bold, Uppercase, White text.
Layout: Display the text on three separate lines:
Line 1: "NEW ERA"
Line 2: "OF DESIGN"
Line 3: "STARTS NOW"
Sizing: Large and responsive (text-6xl mobile, text-8xl tablet, text-[100px] desktop).
Spacing: Very tight line height (0.98) and negative letter spacing (-2px to -4px).

4. Custom CTA Button:

Place a button below the headline with a fixed size of 184px wide by 65px high.
Interaction: Add a hover effect that slightly scales up (scale-105) and an active press effect (scale-95).
Background: Instead of a standard CSS background, use an SVG element that fills the button container (absolute inset-0). Use a custom path for the shape filled with white.
Text: Centered label "GET STARTED".
Text Style: Rubik, Bold, Uppercase, 20px size, dark text color (#161a20).

## Framelix 3D Studios — Agency [sites/13]

- Preview: https://motionsites.ai/assets/hero-framelix-preview-DsyIImVY.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/13.gif

Create a dark-themed landing page for "Framelix 3D" — a cinematic motion studio brand. The entire site uses a pure black background (#000) with white text. Use the Inter font (import from Google Fonts). The project uses React, Tailwind CSS, TypeScript, and framer-motion.

Global Theme (CSS variables):

Background: black (0 0% 0%), Foreground: white (0 0% 100%), Primary: white, Primary-foreground: black. Border radius: 9999px (fully rounded). No light mode — dark only.

1. Navbar:

Black background, horizontal padding 36px, top padding 32px, bottom padding 20px.

Left: Logo image (apply brightness-0 invert so it appears white), height 36px.

Center: 3 columns of nav links (hidden on mobile), each column has 2 stacked links: [Reels, Services], [Projects, Pipeline], [Careers, Get In Touch]. Gap between columns: 64px.

Right: A custom ticket/coupon-shaped SVG cart icon (27x30 viewBox with scalloped edges) with a "0" count overlaid centered on the icon.

2. Hero Section:

Full-width section, relative positioned, min-height screen, overflow hidden.

Background: an auto-playing, looped, muted, inline video taking full width (w-full h-auto object-cover). Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260223_060517_9feec9ab-18e4-477a-b034-de5903a67e91.mp4

Overlay text at top (absolute, top 50px, centered): "Framelix" with superscript "3D" (26px, medium weight), below it "Cinematic Motion Studios" heading (clamp 2rem–68px, medium weight). Animate in with framer-motion fade+slide down (0.8s).

Overlay CTA at bottom (absolute, bottom 12%, centered): "Explore Reel" button (white bg, black text, 22px, rounded-full, px-14 py-4). Below it "Ready in 24-48 hours" muted text. Animate in with framer-motion fade+slide up (0.8s, 0.3s delay).

3. Marquee Banner:

Background color: #A6A4FF (lavender/purple). Black text, 16px, medium weight.

Infinitely scrolling marquee text: "New! 3D^OS V1.2.1 out now!" repeated 6 times in a row, duplicated for seamless loop. Use CSS animation translateX(0) to translateX(-50%) over 20s linear infinite. Vertical padding 14px, gap 60px between items.

4. Shipping Section:

Background: #EAEAEA (light gray), with rounded bottom corners (40px radius). All text is black.

Top: Centered text — "Framelix" with superscript "3D" (20px) and "Shipping Now" heading (clamp 2rem–52px). Top padding 64px.

Center: Auto-playing, looped, muted video, 800x800px, object-contain, rounded-2xl, with -my-24 negative vertical margin to reduce whitespace. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260223_063954_03a5f7ec-5f10-4acb-ba8d-dce4815217db.mp4

Bottom: Centered "Buy Now" button (black bg, white text, 18px, rounded-full, px-46 py-3), below it "Explore now" text (20px, font-weight 450). Bottom padding 128px on the wrapper.

Tailwind config: Add a marquee keyframe and animation (translateX 0 to -50%, 20s linear infinite). Use tailwindcss-animate plugin.

## Logoisum Video Agency — Agency [sites/14]

- Preview: https://motionsites.ai/assets/hero-logoisum-preview-yhpSc7Yy.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/14.gif

Build a premium, high-end hero section for a video editing agency named 'Logoisum' with the following specifications:

Background: Implement a full-screen, looping video background using this URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260228_065522_522e2295-ba22-457e-8fdb-fbcd68109c73.mp4. The video must be muted, autoplaying, and set to object-cover to fill the section without any color overlays.

Navigation Bar: A floating white navigation bar with rounded-[16px] and a subtle shadow.

Left: The agency logo.

Center: A menu with links for 'About', 'Works', 'Services', and 'Testimonial' using 14px Barlow Medium font.

Right: A dark (#222) primary CTA button labeled 'Book A Free Meeting' featuring a unique 45-degree arrow icon in a circular housing.

Typography & Hero Content:

Primary Headline: Centered layout. The first line 'Agency that makes your' should use a bold/medium Barlow font with tight tracking (tracking-[-4px]). The second line 'videos & reels viral' must use a large, elegant 'Instrument Serif' italic font (text-[84px]).

Subtext: Below the headline, add the text 'Short-form video editing for Influencers, Creators and Brands' in Barlow Medium, 18px, centered.

Secondary CTA: A large white pill-shaped button below the subtext labeled 'See Our Workreel' with a small play icon on the left.

Overall Aesthetic: The design should be minimal, ultra-modern, and responsive. Ensure all text and buttons are layered on top of the video background with clear visibility and proper spacing (min-h-[90vh]).

## Buzzentic Agency — Agency [sites/21]

- Preview: https://motionsites.ai/assets/hero-buzzentic-preview-CbopM29R.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/21.gif

Project Requirements: Build a high-impact, full-screen React hero section using Tailwind CSS v4 and custom typography.

1. Background & Layout:

Full-Screen Video: Implement a background video that covers the entire viewport (object-cover).

Video Source: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260306_074215_04640ca7-042c-45d6-bb56-58b1e8a42489.mp4

Video Settings: Auto-play, loop, muted, and playsInline with no color overlays or filters.

Content Spacing: The main content block should have 250px of bottom padding to create breathing room above the fold.

2. Typography & Colors:

Primary Font: "Barlow" (sans-serif) for general UI and body text.

Accent Font: "Instrument Serif" (italic) for poetic emphasis.

Color Palette: Primary text is pure white (#FFFFFF) or white at 75% opacity. CTA buttons and badges use a neutral off-white (#f8f8f8).

3. Specific UI Elements:

Transparent Navigation: A floating navbar with no background fill and no border strokes. All navigation links and the brand logo must be white.

Featured Badge: A "Featured in Fortune" badge centered at the top. It features a "liquid glass" effect using a white/10 background with backdrop-blur-sm on the outer ring and white/90 with backdrop-blur-md on the inner pill.

Dynamic Headline:

Line 1: "Agency that makes your" (Barlow, font-light, text-white, 64px).

Line 2: "videos & reels viral" (Instrument Serif, italic, text-white, 64px).

Sub-headline: A max-width paragraph in Barlow font, white at 75% opacity, explaining the agency's value proposition.

Button Styling: Rectangular buttons with a very sharp 2px border radius, #f8f8f8 background, and #171717 medium Barlow text.

Corner Accents: Four 7px x 7px solid white squares positioned exactly at the four corners of the central hero content container.

4. Interactions & Animations:

All buttons and interactive badges should have smooth transition-colors on hover.

Buttons should shift from #f8f8f8 to pure white on hover.

Navigation items should have a subtle white/10 background highlight on hover.

## Glassmorphism Agency Hero — Agency [sites/5]

- Preview: https://motionsites.ai/assets/hero-glassmorphism-agency-preview-CGqeRoqP.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/5.gif

Build a production-ready, responsive landing page using React, Tailwind CSS v4, and Vite. The design should feature a high-end, dark-mode "glassmorphism" aesthetic with specific purple/pink gradients.

1. Tech Stack & Libraries:
Use hls.js for video streaming.
Use motion/react (formerly Framer Motion) for animations.
Use react-use-measure for sizing logic.
Use clsx and tailwind-merge for class management.
Use lucide-react for standard icons (if needed), but I will provide custom SVG paths for specific UI elements.

2. Global Styling:
Background: Dark/Black (#010101).
Primary Gradient: A diagonal gradient used for accents: from-[#FA93FA] via-[#C967E8] to-[#983AD6].
Typography: Modern sans-serif, center-aligned hero text.

3. Hero Section Components:
Announcement Pill:
A pill-shaped top badge.
Background: Semi-transparent dark (bg-[rgba(28,27,36,0.15)]) with a subtle border.
Icon: A "Zap" icon inside a gradient-filled box with a glow effect.
Text: "Used by founders. Loved by devs." in light grey.

Main Headline (H1):
Large text (responsive sizing: 48px mobile to 80px desktop).
Text: "Your Vision" on line 1, "Our Digital Reality." on line 2.
Style: Text should have a gradient fill (White to Purple/Pink).

Subheadline:
Text: "We turn bold ideas into modern designs that don't just look amazing, they grow your business fast."
Color: text-white/80.

CTA Button:
"Book a 15-min call" text.
Rounded full button with a white background and black text.
Includes a circle icon with an arrow inside, styled with the primary purple gradient.
Outer border wrapper with a glass effect.

4. Hero Video Integration (Critical Details):
Source: HLS Stream URL: https://customer-cbeadsgr09pnsezs.cloudflarestream.com/697945ca6b876878dba3b23fbd2f1561/manifest/video.m3u8
Fallback: If HLS fails, fallback to this MP4: /_videos/v1/f0c78f536d5f21a047fb7792723a36f9d647daa1
Implementation: Do NOT use react-player. Use a native <video> tag with a custom useEffect hook implementation of hls.js.
Styling:
Blend Mode: Use mix-blend-screen so the video black background blends into the page.
Positioning: The video should be at the bottom of the hero. Apply a negative top margin (-mt-[150px]) so it overlaps behind the text.
Z-Index: Ensure the text content is z-20 (above) and video is z-10 (below).
Layout: The video must be 100% width (w-full), auto height, and stretch edge-to-edge without being cropped (do not use object-contain or fixed heights).
Overlay: Add a gradient fade (from-[#010101] via-transparent to-[#010101]) over the video container.

5. Logo Cloud Section (Animated):
Place this section immediately below the video.
Background: Semi-transparent glass (bg-black/20 backdrop-blur-sm) with a top border (border-white/5).
Layout:
Desktop: "Powering the best teams" text on the left, separated by a vertical divider. Animated logo slider on the right.
Mobile: Stacked vertically.
Animation: Create an InfiniteSlider component using motion/react that scrolls logos horizontally forever.
Logos: Use these SVG URLs (OpenAI, Nvidia, GitHub, etc.) and apply brightness-0 invert to make them white.
https://html.tailus.io/blocks/customers/openai.svg
https://html.tailus.io/blocks/customers/nvidia.svg
(Include others similarly)

Please assemble these into a cohesive Hero.tsx, App.tsx, and components/ui/infinite-slider.tsx structure.

## Creative Studio — Agency [sites/creative-studio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(71).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/creative-studio.webp

Build a full-screen hero section using React, Tailwind CSS, Framer Motion, and Lucide React icons. Use the Inter font. The page is fully mobile-responsive. Here are the exact specifications:

---

**BACKGROUND:**
- A full-screen autoplaying, looping, muted video covering the entire viewport as a background.
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260517_222138_3e3205be-3364-417b-a64a-bfe087acbec4.mp4`
- The video is positioned absolute, inset-0, with `object-cover` to fill the viewport.

---

**COLOR:**
- Accent color: `#5E0ED7` (deep purple). Used for the logo dot, the "+" symbols in stats, and the CTA link text.
- All body text is black (#000).

---

**FONT:**
- Font family: `'Inter', sans-serif` applied to the root container.
- All text is uppercase with wide letter-spacing (`tracking-widest` or `tracking-wide`).
- Font weights: 600 (semibold) throughout.

---

**LAYOUT (flex column, min-h-screen):**
The page is a flex column with three vertical sections:
1. **Nav** (top, fixed height)
2. **Stats row** (flex-1, vertically centered, right-aligned)
3. **Bottom content** (pinned to bottom with padding)

---

**NAVIGATION BAR:**
- Horizontal flex, items centered, justified between. Padding: `px-5 sm:px-8 md:px-12 pt-5 md:pt-6`.
- **Left:** A circular logo — 32px round div with 2px border in accent color, containing a 10px solid circle in accent color.
- **Center (hidden on mobile, visible md+):** Four nav links: "Story", "Expertise", "Studios", "Feedback". Text: 14px, font-semibold, tracking-widest, uppercase, black.
- **Right:** A 36px round black button with three horizontal white lines (hamburger icon — three `span` elements, each `w-4 h-0.5 bg-white` with `gap-1`). This opens the mobile menu on click.

---

**MOBILE MENU OVERLAY:**
- Triggered by hamburger click. Fixed, full-screen, z-50, white background.
- Top row: same logo (left) and a 36px round black close button with an X icon (right).
- Below: vertical list of the 4 nav links at `text-3xl`, font-semibold, tracking-widest, uppercase, with `gap-8` and `mt-16`.
- Bottom (mt-auto): "Work With Us" CTA in accent color with ArrowUpRight icon, `text-xl`.

---

**STATS ROW (middle section):**
- Container: `flex-1 flex items-center justify-end`, with same horizontal padding. `py-8 md:py-0`.
- Three stat items in a horizontal row with `gap-5 sm:gap-8 md:gap-10`, each right-aligned:
  - **+300** / CRAFTED BRANDS
  - **+200** / DIGITAL PRODUCTS
  - **+100** / VENTURES FUNDED
- Number styling: `fontSize: clamp(1.5rem, 5vw, 3.5rem)`, weight 600. The "+" is rendered separately in accent color at 0.5em size. The number is black.
- Label: `text-[10px] sm:text-xs md:text-sm`, font-semibold, tracking-widest, uppercase, black, `whitespace-pre-line leading-tight` (each label has a line break between the two words).

---

**BOTTOM SECTION:**
- Padding: `px-5 sm:px-8 md:px-12 pb-8 md:pb-12`. Flex column with `gap-6 md:gap-12`.

**Row A (tagline + CTA):**
- Flex row, items-center, justify-between, gap-4.
- **Left:** Small uppercase tagline paragraph: "Shaping Bold / Visions Into Power / For Your Tribe" (with `<br />` line breaks). Text: `text-[10px] sm:text-xs md:text-sm`, font-semibold, tracking-widest, max-width `130px sm:160px md:max-w-xs`.
- **Right:** CTA link "Work With Us" with ArrowUpRight icon. Text: `text-base sm:text-xl md:text-2xl`, accent color, weight 600, `whitespace-nowrap`. Icon: 18px on mobile, 22px on sm+.

**Row B (description + main heading):**
- Flex row, `items-end`, justify-between, `gap-3 sm:gap-4`.
- **Left:** A fixed-width container (`w-[120px] sm:w-[180px] md:w-[280px]`, shrink-0) containing a paragraph: "Creative Studios Built Around Elevating Your Vision Into Striking Reality". Text: `text-[9px] sm:text-xs md:text-sm`, font-semibold, tracking-widest, uppercase, `text-left md:text-right`.
- **Right:** The main heading — three words stacked vertically: "Fearless", "Vision", "Delivered". Each word in its own `overflow-hidden` wrapper. Text: `fontSize: clamp(2rem, 9vw, 9rem)`, `lineHeight: 0.88`, weight 600, uppercase, black, text-right.

---

**ANIMATIONS (Framer Motion):**

All animations fire on page load (initial -> animate).

1. **fadeDown variant** (nav elements):
   - From: `{ opacity: 0, y: -20 }`
   - To: `{ opacity: 1, y: 0 }`
   - Each element has a custom stagger index. Delay: `index * 0.1s`. Duration: 0.5s. Ease: `[0.22, 1, 0.36, 1]`.
   - Applied to: logo (custom=0), each nav link (custom=1-4), hamburger (custom=5).

2. **fadeUp variant** (stats + bottom content):
   - From: `{ opacity: 0, y: 32 }`
   - To: `{ opacity: 1, y: 0 }`
   - Delay: `index * 0.12s`. Duration: 0.6s. Ease: `[0.22, 1, 0.36, 1]`.
   - Applied to: each stat card (custom=2,3,4), tagline paragraph (custom=5), CTA link (custom=6), description block (custom=7).

3. **Heading slide-up** (main heading words):
   - Each word slides up from `y: "110%"` to `y: 0` within its overflow-hidden parent (clip reveal effect).
   - Delay: `0.4 + wordIndex * 0.14` (so 0.4s, 0.54s, 0.68s). Duration: 0.7s. Ease: `[0.22, 1, 0.36, 1]`.

---

**RESPONSIVE BREAKPOINTS:**
- Mobile-first. Three tiers: default (mobile), `sm:` (640px), `md:` (768px).
- Nav links hidden on mobile, shown md+.
- Spacing, font sizes, and widths scale up at each breakpoint.
- Mobile menu provides full navigation on small screens.

---

**DEPENDENCIES:**
- React 18
- Tailwind CSS 3
- framer-motion
- lucide-react (ArrowUpRight, X icons)

## Modern Agency — Agency [sites/modern-agency]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(27).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/modern-agency.webp

Build a React + Vite + Tailwind CSS landing page for "Axion Studio" - a design agency site. Use the `shaders` package (npm: `shaders`) for the hero background, `lucide-react` for icons. The page has 3 sections. Match every detail exactly:

---

### SECTION 1: HERO (Full viewport height)

**Background:** Light gray `#EFEFEF` with a full-screen animated shader overlay (positioned absolute, inset-0, z-10, pointer-events-none). The shader stack uses components from `shaders/react`:
- `Swirl` - colorA: `#ffffff`, colorB: `#f0f0f0`, detail: 1.7
- `ChromaFlow` - baseColor: `#ffffff`, downColor/leftColor/rightColor/upColor: `#ff5f03`, momentum: 13, radius: 3.5
- `FlutedGlass` - aberration: 0.61, angle: 31, frequency: 8, highlight: 0.12, highlightSoftness: 0, lightAngle: -90, refraction: 4, shape: "rounded", softness: 1, speed: 0.15
- `FilmGrain` - strength: 0.05

**Navigation (z-20, relative):** A pill-shaped white navbar (`bg-white rounded-full`) with 5px padding, inside a max-w-[1440px] container with p-2 sm:p-3.

- LEFT: Dark circle logo (w-9 h-9 sm:w-10 sm:h-10, bg-gray-900, rounded-full) with white text "AX" (10px/11px, font-bold, tracking-tight). Next to it (hidden on mobile, shown md+): nav links "Projects", "Studio", "Journal", "Connect" - 14px, text-gray-900, hover:text-gray-500, transition-colors duration-300, gap-6.

- RIGHT (hidden on mobile, shown md+):
  - Text "Taking on projects for Q1 2026" (13px, text-gray-600, hidden below lg)
  - Clock icon (lucide, size 14) + live London time "{HH:MM} in London" (13px, text-gray-600)
  - CTA button: bg-gray-900, text-white, 13px font-medium, rounded-full, pl-5 pr-2 py-2. Text "Book a strategy call" with a HOVER TEXT ROLL animation: the text is duplicated inside a flex-col container with overflow-hidden h-[20px], on group-hover it translates -50% vertically (duration-500, ease cubic-bezier(0.25,0.1,0.25,1)). Arrow icon in a white circle (w-6 h-6) that rotates -45deg on hover (same easing).

- MOBILE: A "Menu"/"Close" toggle button (md:hidden), bg-gray-900, rounded-full, with Menu/X icons from lucide-react.

**Mobile Menu Overlay:** Fixed inset-0, z-50. Black/60 backdrop. A white bottom sheet (rounded-2xl, mx-3 mb-3) that slides up (translate-y-full to translate-y-0, duration-500, ease cubic-bezier(0.32,0.72,0,1)). Contains: time badge, nav links (28px/32px font-medium), and a "Start a project" button with arrow.

**Hero Content (z-20):** Positioned at the bottom of the viewport using flexbox (flex-1 spacer above). Max-w-[1440px], px-5 sm:px-8 lg:px-12, pb-14 sm:pb-16 lg:pb-20.

- Small label: "Axion Studio" (13px/14px, text-gray-900, tracking-wide, mb-5 sm:mb-8)
- Headline h1: "We craft digital experiences / for brands ready to dominate / their category online." - clamp(1.75rem,7vw,4.2rem) on mobile, clamp(2.5rem,5vw,4.2rem) on sm+. font-medium, leading-[1.08], tracking-[-0.03em], text-gray-900. Line breaks hidden on mobile (uses `<br className="hidden sm:block" />` with `<span className="sm:hidden"> </span>` fallback spaces).
- CTA row (mt-8 sm:mt-12, flex-col sm:flex-row, gap-4 sm:gap-5):
  - Orange button: bg-[#F26522], hover:bg-[#e05a1a], text-white, 13px/14px, rounded-full, pl-5 sm:pl-6 pr-2 py-2. Same text-roll hover animation for "Start a project". White circle (w-7 h-7 sm:w-8 sm:h-8) with orange ArrowRight that rotates -45deg on hover.
  - Partner badge: White pill with subtle shadow (0_2px_8px_rgba(0,0,0,0.08)), hover shadow (0_4px_16px_rgba(0,0,0,0.12)), rounded-[4px]. Contains an inline SVG icon (the starburst/compass shape below, w-5 h-5 sm:w-6 sm:h-6, fill-current text-[#E8704E]), text "Certified Partner" (13px/14px font-medium), and a dark badge "Featured" (10px/11px, bg-gray-900, text-white, px-1.5 sm:px-2 py-0.5, rounded).

**SVG Icon for partner badge:**
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path d="m19.6 66.5 19.7-11 .3-1-.3-.5h-1l-3.3-.2-11.2-.3L14 53l-9.5-.5-2.4-.5L0 49l.2-1.5 2-1.3 2.9.2 6.3.5 9.5.6 6.9.4L38 49.1h1.6l.2-.7-.5-.4-.4-.4L29 41l-10.6-7-5.6-4.1-3-2-1.5-2-.6-4.2 2.7-3 3.7.3.9.2 3.7 2.9 8 6.1L37 36l1.5 1.2.6-.4.1-.3-.7-1.1L33 25l-6-10.4-2.7-4.3-.7-2.6c-.3-1-.4-2-.4-3l3-4.2L28 0l4.2.6L33.8 2l2.6 6 4.1 9.3L47 29.9l2 3.8 1 3.4.3 1h.7v-.5l.5-7.2 1-8.7 1-11.2.3-3.2 1.6-3.8 3-2L61 2.6l2 2.9-.3 1.8-1.1 7.7L59 27.1l-1.5 8.2h.9l1-1.1 4.1-5.4 6.9-8.6 3-3.5L77 13l2.3-1.8h4.3l3.1 4.7-1.4 4.9-4.4 5.6-3.7 4.7-5.3 7.1-3.2 5.7.3.4h.7l12-2.6 6.4-1.1 7.6-1.3 3.5 1.6.4 1.6-1.4 3.4-8.2 2-9.6 2-14.3 3.3-.2.1.2.3 6.4.6 2.8.2h6.8l12.6 1 3.3 2 1.9 2.7-.3 2-5.1 2.6-6.8-1.6-16-3.8-5.4-1.3h-.8v.4l4.6 4.5 8.3 7.5L89 80.1l.5 2.4-1.3 2-1.4-.2-9.2-7-3.6-3-8-6.8h-.5v.7l1.8 2.7 9.8 14.7.5 4.5-.7 1.4-2.6 1-2.7-.6-5.8-8-6-9-4.7-8.2-.5.4-2.9 30.2-1.3 1.5-3 1.2-2.5-2-1.4-3 1.4-6.2 1.6-8 1.3-6.4 1.2-7.9.7-2.6v-.2H49L43 72l-9 12.3-7.2 7.6-1.7.7-3-1.5.3-2.8L24 86l10-12.8 6-7.9 4-4.6-.1-.5h-.3L17.2 77.4l-4.7.6-2-2 .2-3 1-1 8-5.5Z"/></svg>
```

---

### SECTION 2: ABOUT (White background)

`bg-white`, pt-16 sm:pt-20 lg:pt-32, pb-12 sm:pb-16 lg:pb-24, overflow-hidden. Max-w-[1440px] container.

**Badge row:** px-5 sm:px-8 lg:px-12, flex items-center gap-3, mb-6 sm:mb-8.
- Numbered circle: w-6 h-6 sm:w-7 sm:h-7, rounded-full, bg-gray-900, text-white, 11px/12px font-semibold. Shows "1".
- Pill label: "Introducing Axion" - 12px/13px, font-medium, border border-gray-200, rounded-full, px-3 sm:px-4 py-1 sm:py-1.5.

**Heading h2:** "Strategy-led creatives, delivering / results in digital and beyond." - clamp(1.5rem,4vw,3.2rem), font-medium, leading-[1.12], tracking-[-0.02em], text-gray-900, mb-12 sm:mb-16 lg:mb-28.

**Content area (responsive):**

- MOBILE/TABLET (lg:hidden): Stacked - paragraph + button, then images.
  - Paragraph: "Through research, creative thinking and iteration we help growing brands realize their digital full potential." - 15px/17px, leading-[1.6], font-medium, text-gray-900.
  - Button: "About our studio" - orange (#F26522), same text-roll animation, white arrow circle rotates -45deg.
  - Two images: flex-col sm:flex-row, gap-4 sm:gap-5. First: sm:w-[45%] aspect-[438/346]. Second: sm:w-[55%] aspect-[900/600]. Both rounded-xl sm:rounded-2xl, object-cover.

- DESKTOP (hidden lg:grid): `grid-cols-[26%_1fr_48%] items-end gap-6 xl:gap-8`.
  - Left column (self-end): Small image, aspect-[438/346], rounded-2xl.
  - Center column (self-start, flex justify-end): Paragraph (16px/18px, leading-[1.65], whitespace-nowrap, with `<br/>` between lines) + orange button.
  - Right column (self-end): Large image, aspect-[3/2], rounded-2xl.

**Image URLs:**
- Small image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260516_090123_74be96d4-9c1b-40cf-932a-96f4f4babed3.png&w=1280&q=85`
- Large image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260516_090133_c157d30b-a99a-4477-bec1-a446149ec3f2.png&w=1280&q=85`

---

### SECTION 3: CASE STUDIES (Light gray background)

`bg-[#F5F5F5]`, pt-16 sm:pt-20 lg:pt-28, pb-16 sm:pb-20 lg:pb-28. Max-w-[1440px] container.

**Badge row:** Same pattern as Section 2, but number is "2", label is "Featured client work", border-gray-300.

**Heading h2:** "Our projects" - same clamp sizing as hero headline (clamp(1.75rem,7vw,4.2rem) / clamp(2.5rem,5vw,4.2rem)), font-medium, leading-[1.08], tracking-[-0.03em], mb-10 sm:mb-14 lg:mb-16.

**Cards Grid:** `grid grid-cols-1 md:grid-cols-2 gap-5 sm:gap-6 lg:gap-7`, px-5 sm:px-8 lg:px-12.

**Card 1 (Narrativ):**
- Video container: aspect-[329/246], rounded-2xl, overflow-hidden, bg-[#1a1d2e], group, cursor-pointer.
- Video: `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260516_122702_390f5305-8719-41d5-ae80-d23ab3796c28.mp4"`, autoPlay, muted, loop, playsInline, w-full h-full object-cover.
- Hover button (absolute bottom-4 left-4): A white circle (h-9 w-9) that expands to w-[148px] on group-hover (transition-all duration-300 ease-in-out). Contains "Learn more" text (13px, font-medium, opacity-0 to opacity-100 on hover with delay-100) and a link/chain SVG icon (14x14, -rotate-45 to rotate-0 on hover). The SVG is the lucide "link" icon drawn manually with two arc paths.
- Description: "Winner of Site of the Month 2025 - an interactive 3D showcase driving record engagement" - 13px/14px, text-gray-600, mt-4, leading-relaxed.
- Title: "Narrativ" - 14px/15px, font-semibold, text-gray-900, mt-1.

**Card 2 (Luminar):**
- Video container: aspect-square, rounded-2xl, overflow-hidden, bg-[#6b6b6b], group, cursor-pointer.
- Video: `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260516_123323_f909c2b8-ff6c-4edf-882b-8ebcdbe389b5.mp4"`, autoPlay, muted, loop, playsInline, w-full h-full object-cover.
- Hover button (absolute bottom-4 left-4): A DARK circle (bg-gray-900, h-9 w-9) that expands to w-[168px] on group-hover. Contains "View case study" text (13px, font-medium, text-white) and a white ArrowRight icon (size 14) that transitions from -rotate-45 to rotate-0 on hover.
- Description: "Transforming a dated platform into a conversion-focused brand experience" - 13px/14px, text-gray-600, mt-4, leading-relaxed.
- Title: "Luminar" - 14px/15px, font-semibold, text-gray-900, mt-1.

---

### GLOBAL STYLES (index.css):

Standard Tailwind directives plus two utility classes (not actively used in current layout but defined):
- `.liquid-glass`: rgba(255,255,255,0.01) bg, backdrop-filter blur(4px), inset box-shadow, pseudo-element gradient border using mask-composite.
- `.liquid-glass-strong`: Same but blur(50px), no pseudo-element.

---

### TECHNICAL DETAILS:
- **Framework:** React 18 + TypeScript + Vite
- **Styling:** Tailwind CSS 3.4 (default config, no custom theme extensions)
- **Packages:** `shaders` (for Shader, ChromaFlow, FilmGrain, FlutedGlass, Swirl from `shaders/react`), `lucide-react` (ArrowRight, Clock, Menu, X)
- **Font:** System default (no custom font loaded)
- **All animations use:** `duration-500 ease-[cubic-bezier(0.25,0.1,0.25,1)]` unless noted otherwise
- **Max content width:** 1440px, centered with mx-auto
- **Responsive breakpoints:** Default Tailwind (sm: 640px, md: 768px, lg: 1024px, xl: 1280px)
- **Live clock:** Updates every second, shows London timezone in HH:MM format

## Orbit Engineers — Agency [sites/orbit-engineers]

- Preview: https://motionsites.ai/assets/hero-orbit-engineers-poster-BT1ffUzn.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbit-engineers.png

Create a single-page landing page for a fictional space engineering consultancy called "WE ARE ORBIT ENGINEERS". The page has 3 full-screen hero sections that the user navigates between using buttons (not scroll). Use React + Tailwind CSS + framer-motion + lucide-react icons (ChevronDown, ArrowRight).

Font & Color System
Font: Inter (Google Fonts: https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap), set as font-sans in Tailwind config.
Color scheme (HSL in CSS variables):
--background: 210 33% 19% (dark blue-gray)
--foreground: 0 0% 100% (white)
--primary: 199 89% 60% (cyan accent)
--accent: 199 89% 60% (same cyan)
All text is white (text-foreground). Background is irrelevant since videos cover the full viewport.

Background Videos
Three elements are fixed, full-screen, layered behind content at -z-10. Each is autoPlay loop muted playsInline with object-cover. They crossfade using transition-opacity duration-700 — the active section's video is opacity-100, others are opacity-0.

Section 0: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_190803_f5595254-156c-4d10-ad09-51a56eb4bc1e.mp4
Section 1: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_204728_2dbcd1c4-63bc-4779-b06c-b7e2d5788ea7.mp4
Section 2: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_210050_14d4d9cf-782b-4f6f-9764-08d793cf427c.mp4

Navigation Bar (always visible, z-20)
Left: Small logo icon (8x8) + bold text stack: WE ARE / ORBIT / ENGINEERS (text-xl, font-bold, tracking-tight, line-height none, stacked with line breaks)
Center (hidden on mobile, hidden md:flex): Three links — Industries, Projects, Insights — uppercase, tracking-widest, text-sm, font-medium
Right: Geographic coordinates in monospace font: 51.50732 N / -0.12765 W (text-xs, tracking-widest, uppercase, font-mono, text-right, stacked with line breaks)
Horizontal padding: px-10, top padding: pt-8

Section Transitions (framer-motion AnimatePresence mode="wait")
All sections use cinematic framer-motion transitions:

Section 0 enter: opacity: 0 → 1, scale: 1.05 → 1, blur: 10px → 0px (duration 0.8s, ease [0.22, 1, 0.36, 1])
Section 0 exit: opacity → 0, scale → 0.92, blur → 12px, y → -60
Section 1 & 2 enter: opacity: 0 → 1, y: 80 → 0, scale: 1.08 → 1, blur: 14px → 0px (duration 0.9s)
Section 1 & 2 exit: opacity → 0, y → -80, scale → 0.95, blur → 10px

Section 0 — Hero Landing
Layout: Flex column, centered content
Decorative elements: Two vertical line images positioned absolutely on left and right edges (left-10, right-10), spanning nearly full height (h-[calc(100%-3rem)])
Headline: "Unlock Tactical / Excellence through Space / Engineering" — text-2xl sm:text-3xl md:text-4xl lg:text-5xl, font-normal, letter-spacing: -3px, centered, max-w-4xl, line breaks

Bottom bar: Left side has a "Scroll to explore" button with bouncing ChevronDown icon (navigates to section 1). Right side has a small logo icon at 60% opacity.

Section 1 — Mission Statement
Layout: Responsive — stacked vertically on mobile (flex-col items-center gap-10), horizontal row on desktop (md:flex-row md:items-center md:gap-16 md:justify-between)
Left: Large heading "Spatial Vision / at Your Command" — text-4xl sm:text-5xl md:text-5xl lg:text-6xl, font-light, letter-spacing: -2px, leading-[0.95]. Animates in from x: -60 with 0.3s delay.
Center: CTA button — "Begin Your Mission" with ArrowRight icon. White background, dark text (bg-foreground text-background), rounded-full, px-8 py-4, tracking 0.25em, uppercase. Hover scales to 105%. Animates in from y: 30 with 0.4s delay.
Right: A "Why We Are" label (text-xs, tracking 0.3em, uppercase) alongside 4 vertically stacked dots (first filled, rest outline — small 5w images). Animates in from x: 60 with 0.45s delay.
Bottom bar: Three elements — "Back to top" button (left, with rotated ChevronDown), centered tagline "Orbital Solutions is a key strategic / consulting firm in space engineering" (text-xs, tracking 0.25em, uppercase, animates from y: 30), and "Next" button (right, with bouncing ChevronDown).

Section 2 — Service Detail
Layout: Centered content with decorative vertical lines on left/right (same as section 0 but h-[calc(100%-1rem)])
Content stack (centered, gap-6):
Section number "01" — text-sm, tracking 0.3em, uppercase, 60% opacity foreground, font-mono. Animates from y: 20, delay 0.3s.
Heading "Operational Feasibility / Evaluation" — text-3xl sm:text-4xl md:text-5xl lg:text-6xl, font-light, letter-spacing: -2px, leading-[1.05]. Animates from y: 40, delay 0.4s.
Description paragraph — "We analyze engineering proposals against / strategic benchmarks to uncover growth paths / and highlight untapped market potential." — text-sm, tracking 0.15em, 70% opacity foreground, max-w-md, font-mono, leading-relaxed. Animates from y: 30, delay 0.55s.
Bottom bar: "Back" button (left), CTA button "Reach Out" with + symbol (center, same style as section 1 CTA), empty spacer div on right (w-20).

Key Design Patterns
All navigation between sections uses useState(0) with handleNext (min +1, max 2) and handlePrev (max -1, min 0)
The entire page is min-h-screen with overflow-hidden
All interactive elements use cursor-pointer
Navigation text uses tracking-widest uppercase consistently
Headlines use negative letter-spacing for a tight, architectural feel
Technical/data text uses font-mono
Buttons and links use hover:text-foreground/80 or hover:scale-105 transitions

## Velorah — Agency [sites/velorah-hero]

- Preview: https://motionsites.ai/assets/hero-velorah-preview-CJNTtbpd.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/velorah-hero.gif

Create a single-page hero section with a fullscreen looping background video, glassmorphic navigation, and cinematic typography. Use React + Vite + Tailwind CSS + TypeScript with shadcn/ui.

Video Background:

Fullscreen <video> element with autoPlay, loop, muted, playsInline
Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131748_f2ca2a28-fed7-44c8-b9a9-bd9acdd5ec31.mp4
Positioned absolute inset-0 w-full h-full object-cover z-0

Fonts:

Import from Google Fonts: Instrumental Serif (display) and Inter weights 400/500 (body)
CSS variables: --font-display: 'Instrument Serif', serif and --font-body: 'Inter', sans-serif
Body uses var(--font-body), headings use inline fontFamily: "'Instrument Serif', serif"

Color Theme (dark, HSL values for CSS variables):

--background: 201 100% 13% (deep navy blue)
--foreground: 0 0% 100% (white)
--muted-foreground: 240 4% 66% (muted gray)
--primary: 0 0% 100%, --primary-foreground: 0 0% 4%
--secondary: 0 0% 10%, --muted: 0 0% 10%, --accent: 0 0% 10%
--border: 0 0% 18%, --input: 0 0% 18%

Navigation Bar:

relative z-10, flex row, justify-between, px-8 py-6, max-w-7xl mx-auto
Logo: "Velorah®" (® as <sup className="text-xs">), text-3xl tracking-tight, Instrument Serif font, text-foreground
Nav links (hidden on mobile, md:flex): Home (active, text-foreground), Studio, About, Journal, Reach Us — all text-sm text-muted-foreground with hover:text-foreground transition-colors
CTA button: "Begin Journey", liquid-glass rounded-full px-6 py-2.5 text-sm text-foreground, hover:scale-[1.03]

Hero Section:

relative z-10, flex column, centered, text-center, px-6 pt-32 pb-40 py-[90px]
H1: "Where dreams rise through the silence." — text-5xl sm:text-7xl md:text-8xl, leading-[0.95], tracking-[-2.46px], max-w-7xl, font-normal, Instrument Serif. The words "dreams" and "through the silence." wrapped in <em className="not-italic text-muted-foreground"> for color contrast
Subtext: text-muted-foreground text-base sm:text-lg max-w-2xl mt-8 leading-relaxed — "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work."
CTA button: "Begin Journey", liquid-glass rounded-full px-14 py-5 text-base text-foreground mt-12, hover:scale-[1.03] cursor-pointer

Liquid Glass Effect (CSS class .liquid-glass):

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

Animations (CSS keyframes + classes):

@keyframes fade-rise {
  from { opacity: 0; transform: translateY(24px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-rise { animation: fade-rise 0.8s ease-out both; }
.animate-fade-rise-delay { animation: fade-rise 0.8s ease-out 0.2s both; }
.animate-fade-rise-delay-2 { animation: fade-rise 0.8s ease-out 0.4s both; }

H1 gets animate-fade-rise
Subtext gets animate-fade-rise-delay
Hero CTA button gets animate-fade-rise-delay-2

Layout: No decorative blobs, radial gradients, or overlays. Minimalist, cinematic, vertically centered hero. The video provides all visual depth.

## Product Studio — Agency Website [sites/product-studio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/Product-Studio.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/product-studio.mp4

Create a two-section dark landing page for a digital studio called "KineticForge" using **React + TypeScript + Vite + Tailwind CSS**. Use **lucide-react** for the logo icon (`Atom`). No other UI libraries.

---

### FONT SETUP

In `index.html`, load "Helvetica Now Var" via this stylesheet link in `<head>`:
```
https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var
```

In `tailwind.config.js`, extend `fontFamily.sans` to:
```js
sans: ['"Helvetica Now Var"', 'sans-serif']
```

Page title: "Kinetic Forge"

---

### GLOBAL CSS (`index.css`)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body {
  overflow-x: hidden;
}
```

---

### ARCHITECTURE: FIXED STACKED BACKGROUND VIDEOS WITH SCROLL-TRIGGERED CROSSFADE

The page has two fullscreen videos positioned `fixed inset-0 z-0`, absolutely stacked on top of each other. All page content scrolls over them in a wrapper with `relative z-10`.

**Video 1 (Hero background):**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260703_152235_56f10620-8704-4c63-8ddd-f146a7085404.mp4`
- Attributes: `muted`, `playsInline`, `autoPlay`, `loop`, `preload="auto"`
- Classes: `absolute inset-0 w-full h-full object-cover transition-opacity duration-700 ease-in-out`
- Starts at opacity 1

**Video 2 (Section 2 background):**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260703_075205_41edc37d-b74b-4c8c-a1e2-2b7879cb4386.mp4`
- Attributes: `muted`, `playsInline`, `loop`, `preload="auto"`
- Classes: same as video 1
- Starts at opacity 0

**Crossfade Logic (IntersectionObserver):**
- Observe section 2 with `threshold: 0.15`
- When section 2 enters viewport: set video 1 opacity to 0, video 2 opacity to 1, reset video 2 `currentTime = 0` and call `.play()`
- When section 2 exits viewport: set video 1 opacity to 1, video 2 opacity to 0, call video 1 `.play()`
- Use React state `activeVideo` (1 or 2) to control inline `style={{ opacity: activeVideo === N ? 1 : 0 }}`

---

### NAVBAR (fixed, z-50)

Position: `fixed top-0 left-0 right-0 z-50`
Layout: `flex items-center justify-between px-6 py-5 md:px-10 lg:px-14`

**Left side:**
- Lucide `Atom` icon: `w-6 h-6 text-white strokeWidth={1.5}`
- Text "KineticForge": `text-white text-base font-medium tracking-tight`
- Container: `flex items-center gap-2`

**Right side (desktop, hidden on mobile):**
- `hidden md:flex items-center gap-8 text-white/80 text-sm font-light`
- Items: "our studio", "expertise", "projects", "get in touch"
- Each: `hover:text-white transition-colors cursor-pointer`

**Right side (mobile hamburger, hidden on desktop):**
- Button: `md:hidden text-white relative w-6 h-6 flex items-center justify-center z-50`
- Two `<span>` elements representing lines:
  - Each: `absolute w-5 h-[1.5px] bg-white transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)]`
  - Closed state: first line `-translate-y-[5px]`, second line `translate-y-[5px]`
  - Open state: first line `rotate-45 translate-y-0`, second line `-rotate-45 translate-y-0`

---

### MOBILE MENU OVERLAY

Container: `fixed inset-0 z-40 bg-black/95 backdrop-blur-xl flex flex-col items-center justify-center md:hidden`
Transition: `transition-all duration-500 ease-[cubic-bezier(0.25,0.1,0.25,1)]`
- Open: `opacity-100 pointer-events-auto`
- Closed: `opacity-0 pointer-events-none`

**Menu items** (same 4 nav labels):
- `text-white text-2xl font-light tracking-wide cursor-pointer hover:text-white/70`
- Stagger animation: `transition-all duration-500 ease-[cubic-bezier(0.25,0.1,0.25,1)]`
  - Open: `opacity-100 translate-y-0` with `transitionDelay: ${100 + i * 75}ms`
  - Closed: `opacity-0 translate-y-4` with `transitionDelay: '0ms'`
- Each item closes the menu on click

**Body scroll lock:** When menu is open, set `document.body.style.overflow = 'hidden'`; restore on close/unmount.

---

### SECTION 1: HERO

Container: `h-screen w-full flex flex-col items-center justify-center px-6 text-center`

**Heading:**
```
Transforming the
online interaction
since 2001
```
- Line breaks using `<br />`
- Classes: `text-white text-4xl sm:text-6xl md:text-7xl lg:text-8xl xl:text-9xl font-normal leading-[1.1] tracking-[-0.06em] max-w-5xl`

**Subheading:**
```
A Vancouver digital studio Specializing
in Web Products and Interface Design
```
- Line break between the two lines using `<br className="hidden sm:block" />` (hidden on mobile, visible on sm+)
- Classes: `mt-6 sm:mt-8 text-white/70 text-sm sm:text-lg font-light max-w-lg leading-relaxed`

---

### SECTION 2: ABOUT + CLIENTS

Container: `min-h-screen w-full` (this element gets the IntersectionObserver ref)
Inner layout: `flex flex-col lg:flex-row min-h-screen`

### Left Column
Container: `flex-1 flex flex-col justify-between px-6 pt-24 pb-12 md:px-10 lg:px-14 lg:pt-28 lg:pb-20`

**Heading:**
```
We craft award
winning platforms
and tools
```
- Line breaks using `<br />`
- Classes: `text-white text-2xl sm:text-3xl md:text-4xl lg:text-5xl font-normal leading-[1.1] tracking-[-0.06em] max-w-lg`

**Body text block** (below heading, `mt-10 lg:mt-0`):
- Wrapper: `max-w-sm space-y-5`
- Paragraph 1: "With studios in Vancouver, Montreal and Berlin, we design and develop full-scale digital products that generate real outcomes, cut costs, boost engagement and grow revenue."
- Paragraph 2: "For over 23 years, Kinetic Forge has partnered with organizations large and small with a roster that features some of the most recognized names worldwide. We're always eager to arrange a meeting to explore your next venture so don't hesitate to reach out for a free consultation or quote!"
- Both paragraphs: `text-white/60 text-xs sm:text-sm leading-relaxed font-light`

**Button:**
- Text: "Learn more here"
- Classes: `mt-6 sm:mt-8 border border-white/30 text-white text-xs sm:text-sm font-light px-5 sm:px-6 py-2.5 sm:py-3 rounded-sm hover:bg-white/10 transition-colors`

### Right Column
Container: `flex-1 flex flex-col justify-end px-6 pb-12 md:px-10 lg:px-14 lg:pb-20`

**Label:** "Partners we're proud to work with"
- Classes: `text-white/60 text-xs sm:text-sm font-light mb-6 sm:mb-8`

**Client Grid:**
- Container: `grid grid-cols-2 gap-x-8 sm:gap-x-12 gap-y-8 sm:gap-y-10 max-w-md`

**6 grid items:**

1. **NASA** -- `<span>` with classes: `text-white text-xl sm:text-3xl font-bold tracking-widest`
2. **Google** -- `<span>` with classes: `text-white text-xl sm:text-3xl font-medium tracking-tight`
3. **Canadian Digital Service** -- SVG infinity-loop icon (w-6 h-6 sm:w-8 sm:h-8, fill="currentColor", white) + text "Canadian\nDigital Service" (text-[10px] sm:text-xs font-light, line break via `<br />`)
   - SVG path: `M18.6,6.62C17.16,6.62 15.8,7.18 14.83,8.15L7.8,14.39C7.16,15.03 6.31,15.38 5.4,15.38C3.53,15.38 2,13.87 2,12C2,10.13 3.53,8.62 5.4,8.62C6.31,8.62 7.16,8.97 7.84,9.65L8.97,10.65L10.5,9.31L9.22,8.2C8.2,7.18 6.84,6.62 5.4,6.62C2.42,6.62 0,9.04 0,12C0,14.96 2.42,17.38 5.4,17.38C6.84,17.38 8.2,16.82 9.17,15.85L16.2,9.61C16.84,8.97 17.69,8.62 18.6,8.62C20.47,8.62 22,10.13 22,12C22,13.87 20.47,15.38 18.6,15.38C17.69,15.38 16.84,15.03 16.16,14.35L15.03,13.34L13.5,14.68L14.78,15.8C15.8,16.82 17.16,17.38 18.6,17.38C21.58,17.38 24,14.96 24,12C24,9.04 21.58,6.62 18.6,6.62Z`
4. **United Nations** -- SVG globe icon (w-8 h-8 sm:w-10 sm:h-10, stroke="currentColor", strokeWidth="0.8", fill="none") with: circle cx=12 cy=12 r=10, ellipse cx=12 cy=12 rx=4 ry=10, three horizontal lines at y=7, y=12, y=17 (x1=4/2/4 x2=20/22/20) + text "United Nations" (text-[10px] sm:text-xs font-light)
5. **Canada** -- SVG star icon (w-5 h-5 sm:w-7 sm:h-7, fill="currentColor", white) path: `M12 2L9.5 8.5H2L8 12.5L5.5 19L12 15L18.5 19L16 12.5L22 8.5H14.5L12 2Z` + text "Canada" (text-xs sm:text-sm font-light)
6. **Department of Administration** -- `<span>` "mn" (text-base sm:text-lg font-bold) + text "Department of\nAdministration" (text-[10px] sm:text-xs font-light, line break via `<br />`)

All grid items use `flex items-center gap-2 sm:gap-3` (except NASA and Google which use `flex items-center justify-start`).

## Investor Deck — Investor Presentations [sites/deck-investor]

- Preview: https://motionsites.ai/assets/hero-deck-preview-CbidQJxW.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/deck-investor.gif

Build a presentation-style slide deck web app with 5 slides using React, Tailwind CSS, hls.js for Mux HLS video playback, and motion (from motion/react) for animations. The font is Aeonik, sans-serif. The entire deck has a black background (bg-black) so transitions fade through black, never white.

Global Architecture

App container: Full-width/full-height, overflow-hidden, font-['Aeonik',sans-serif], bg-black.

All 5 slides are always mounted simultaneously (not conditionally rendered) so HLS videos preload in the background. Only opacity changes via Motion's animate={{ opacity: isActive ? 1 : 0 }} with duration: 0.35, ease: "easeInOut". The active slide gets zIndex: 10, inactive slides get zIndex: 0 and pointerEvents: "none".

Keyboard navigation: ArrowRight/ArrowDown/Spacebar = next slide, ArrowLeft/ArrowUp = previous slide.

Navigation dots: Centered at bottom (bottom-5, left-1/2, -translate-x-1/2, z-20), a row of clickable dots with gap-2. Active dot: bg-white w-6 h-2 rounded-full. Inactive dot: bg-white/40 w-2 h-2 rounded-full. All dots have transition-all duration-300.

Each slide receives an isActive prop. Internally tracks activationCount state that increments each time isActive becomes true. The content wrapper uses key={activationCount} to re-mount and re-trigger all animations fresh each time the slide becomes active, while the video stays persistently mounted outside this keyed wrapper.

Shared Components

Logo: A white SVG logo, 116px wide × 36px tall, rendered from imported SVG path data. Appears in the top-left of every slide.

AnimatedText components (reusable across all slides):

SlideUpLine: Clip-reveal slide-up for headings. Wraps children in overflow-hidden inline-block span. Inner motion.span animates from y: "100%" to y: "0%". Default duration 0.7s. Easing: [0.25, 0.1, 0.25, 1].

WordByWordReveal: Splits text by spaces. Each word is in an overflow-hidden inline-block span with mr-[0.27em]. Inner motion.span animates from y: "100%" to y: "0%" with stagger. Default stagger: 0.035s, duration: 0.55s. Same easing curve.

BlurReveal: motion.div animates from opacity: 0, filter: "blur(8px)" to opacity: 1, filter: "blur(0px)". Default duration: 0.9s. Same easing curve. Used for metadata, labels, and description paragraphs.

Common Slide Layout Pattern

Every slide follows this structure:

Full-size container (w-full h-full flex flex-col relative overflow-hidden)

Background video (absolutely positioned, persistently mounted outside the keyed wrapper)

relative z-10 content wrapper (keyed by activationCount)

Top bar: Logo (left) + slide number or metadata (right), wrapped in BlurReveal, with px-[5%] pt-[3.5%]

Divider: mt-6 px-[5%] container with a bg-white/15 h-px w-full line

Content area below

HLS Video Setup Pattern

Each background video uses hls.js:

Create a <video> element with autoPlay muted loop playsInline

In useEffect: if Hls.isSupported(), create new Hls({ autoStartLoad: true }), load source, attach media, play on MANIFEST_PARSED. Cleanup destroys HLS instance. Fallback for Safari: set video.src directly and play on loadedmetadata.

Slide 1 — Title Slide "Innovation and Growth"

Background: bg-black

Video URL: https://stream.mux.com/Aa02T7oM1wH5Mk5EEVDYhbZ1ChcdhRsS2m1NYyx4Ua1g.m3u8

Video styling: absolute inset-0 w-full h-full object-cover (full opacity, no transform)

Top bar (in BlurReveal delay={0.1}):

Left: Logo

Right: 4 metadata items in a flex gap-8 row. Each item is a flex-col with gap-[2px]: label in text-[#80838e] text-[13px], value in text-white text-[13px]. Items: Type→"Investor Deck", Investor→"BlackRock", Date→"February 2026", Industry→"Artificial Intelligence"

Divider: px-[5%] mt-6, bg-white/15 h-px w-full

Title text: Positioned at bottom-left using flex-1 flex items-end px-[5%] pb-[8%]. <h1> with text-white leading-[0.9] tracking-tight, fontSize: clamp(48px, 10vw, 140px). Two lines using SlideUpLine:

Line 1: "Innovation" with delay={0.3} duration={0.7}

<br />

Line 2: "and Growth" with delay={0.4} duration={0.7}

Slide 2 — Problem Statement with Stat Cards

Background: bg-black

Video URL: https://stream.mux.com/s8pMcOvMQXc4GD6AX4e1o01xFogFxipmuKltNfSYza0200.m3u8

Video styling: absolute inset-0 w-full h-full object-cover (full opacity, no transform)

Top bar (in BlurReveal delay={0.05}):

Left: Logo

Right: text-[#80838e] text-[20px] leading-[1.4] showing "02"

Content area: flex flex-col flex-1 justify-between pt-[4%] pb-[5%] inside px-[5%] wrapper

Upper section (max-w-[85%]):

Subtitle: BlurReveal delay={0.15}, text-[#80838e], fontSize: clamp(12px, 1.2vw, 18px), text "Problem Statement"

Heading: WordByWordReveal with text-white leading-[1.04], fontSize: clamp(22px, 3.5vw, 56px), text "In the realm of AI, businesses face challenges in data analysis, decision-making bottlenecks, and seamless integration of AI solutions", baseDelay={0.25} stagger={0.035} duration={0.55}

Stat cards (bottom, flex gap-4 w-full): 3 StatCard components, each flex flex-1 flex-col gap-3 min-w-0. Each card is a motion.div animating from y: 30, opacity: 0 to y: 0, opacity: 1 with duration: 0.6, delay: 0.6 + index * 0.1, ease: [0.25, 0.1, 0.25, 1].

Card 1: value "80%" / label "Face data complexity"

Card 2: value "63%" / label "Struggle with AI integration"

Card 3: value "47%" / label "Delay decisions due to bottlenecks"

Value styling: text-white leading-[0.96] tracking-tight, fontSize: clamp(32px, 6vw, 96px)

Label styling: text-white leading-[1.4], fontSize: clamp(13px, 1.2vw, 20px)

Slide 3 — Market Opportunity with Growth Chart

Background: bg-black

Video URL: https://stream.mux.com/Gs3wZfrtz6ZfqZqQ02c02Z7lugV00FGZvRpcqFTel66r3g.m3u8

Video styling: absolute inset-0 w-full h-full object-cover with transform: scale(-1, -1) (flipped both vertically and horizontally) and opacity: 0.5

Top bar (in BlurReveal delay={0.05}):

Left: Logo

Right: "03" in text-[#80838e] text-[20px] leading-[1.4]

Text content (max-w-[55%] px-[5%] pt-[3%]):

Subtitle: BlurReveal delay={0.15}, text "Market Opportunity", text-[#80838e], fontSize: clamp(12px, 1.2vw, 18px)

Heading: WordByWordReveal, text "At Viktory, we target a growing market for AI innovation, especially in cybersecurity and web3", text-white leading-[1.04], fontSize: clamp(20px, 3.2vw, 52px), baseDelay={0.25} stagger={0.035} duration={0.55}

Description: BlurReveal delay={0.8}, text "We strategically focus on AI innovation at the intersection of cybersecurity and web3, meeting the growing demand for advanced solutions. Positioned at the forefront, we drive transformative technology in the evolving digital landscape.", text-[#80838e] max-w-[90%], fontSize: clamp(12px, 1.1vw, 18px)

Chart: motion.div positioned absolute bottom-[3%] left-0 right-0 top-[40%], animates from opacity: 0, y: 30 to opacity: 1, y: 0 with duration: 0.8, delay: 0.7, ease: [0.25, 0.1, 0.25, 1]. Contains:

ChartArea (absolute bottom-0 right-0 w-[55%] h-[70%]): A purple-to-pink gradient growth curve composed of:

Gradient area fill image (imported PNG)

Opacity line: SVG path with strokeWidth: 24, gradient stroke from white/0.15 to white/0

Main gradient line: SVG path with strokeWidth: 4, linear gradient from #8238DC to #F75CB7

SectorMarker (absolute bottom-[22%] left-[44%]): Shows "32%" value, a 2px white vertical line (40px), and a 100x100px sector circle with white/0.08 fill and a centered gradient dot (#7FBAFF to #536EFB with white stroke)

TopValue (absolute top-[2%] right-[5%]): Shows "127%" in fontSize: clamp(32px, 4vw, 64px), a 2px white vertical line (50px), and a small gradient dot

MidDot (absolute top-[40%] right-[35%]): Small gradient dot only

XAxis (absolute bottom-0 left-0 right-0): Horizontal line in bg-[#1a2035], 8 tick marks, and year labels 2017-2024 in text-[#80838e], fontSize: clamp(11px, 1vw, 18px)

No teal arrow callout

Slide 4 — Sales and Distribution Channels

Background: bg-black

Video URL: https://stream.mux.com/PkFsoKeakRLgL01gjf02CRcSbsJ600Z00NvLr9eRZ92pLbA.m3u8

Video styling: absolute top-0 bottom-0 right-0 h-full object-cover with left: 400px (400px left padding offset)

Top bar: Absolutely positioned (absolute top-0 left-0 right-0 pt-[3.5%]), BlurReveal delay={0.05}:

Left: Logo

Right: "04" in text-[#80838e] text-[20px] leading-[1.4]

Divider: Absolutely positioned at top-[calc(3.5%+52px)], bg-white/15 h-px w-full

Content wrapper: flex flex-col w-full h-full justify-center (vertically centered)

Text content (max-w-[65%] px-[5%]):

Subtitle: BlurReveal delay={0.15}, text "Sales and Distribution Channels", text-[#80838e], fontSize: clamp(12px, 1.2vw, 26px)

Heading: WordByWordReveal, text "Our direct sales team engages with enterprises, while online platforms and strategic partnerships expand our outreach", text-white leading-[1.04], fontSize: clamp(20px, 4vw, 80px), baseDelay={0.25} stagger={0.035} duration={0.55}

Description: BlurReveal delay={1.2}, text "Our direct engagement with enterprises ensures tailored solutions through consultations. Meanwhile, the strategic utilization of online platforms and partnerships significantly broadens our reach, ensuring impactful and diverse distribution.", text-[#80838e] max-w-[784px], fontSize: clamp(12px, 1.1vw, 26px)

Slide 5 — Global Expansion

Background: bg-[#131318]

Video URL: https://stream.mux.com/BuGGTsiXq1T00WUb8qfURrHkTCbhrkfFLSv4uAOZzdhw.m3u8

Video styling: absolute object-cover with inline styles: width: "200%", height: "200%", bottom: 0, left: 0. This makes the video 200% of the slide size with its focal point anchored to the bottom-left corner.

Top bar (in BlurReveal delay={0.05}):

Left: Logo

Right: "05" in text-[#80838e] text-[20px] leading-[1.4]

Divider: mt-6 px-[5%], bg-white/15 h-px w-full

Spacer: flex-1 div pushes text content to the bottom

Text content (at bottom, max-w-[55%] px-[5%] pb-[5%]):

Subtitle: BlurReveal delay={0.15}, text "Global Expansion", text-[#80838e], fontSize: clamp(12px, 1.2vw, 26px)

Heading: WordByWordReveal, text "Opportunities across continents", text-white leading-[1.04], fontSize: clamp(20px, 4vw, 80px), baseDelay={0.25} stagger={0.035} duration={0.55}

Description: BlurReveal delay={0.6}, text "Our global break-even journey aligns revenue with expenses worldwide. Through strategic cost management and international growth initiatives, we target break-even within 18 months, fortifying a strong global financial foundation for success.", text-[#80838e] max-w-[680px], fontSize: clamp(12px, 1.1vw, 26px)

Dependencies

hls.js — HLS video streaming

motion — animations (import from motion/react)

Tailwind CSS v4

Color Palette

Backgrounds: black (slides 1-4), #131318 (slide 5)

Primary text: white

Secondary/muted text: #80838e

Dividers: white/15 (15% opacity white)

Chart gradient: #8238DC → #F75CB7 (purple to pink)

Chart dots gradient: #7FBAFF → #536EFB

X-axis elements: #1a2035

## Dark Portfolio Hero — Portfolio [sites/15]

- Preview: https://motionsites.ai/assets/hero-portfolio-dark-preview-RZYzJHIL.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/15.gif

Build a Next.js 14 portfolio landing page with a full-screen hero section and an animated loading screen. The entire site uses a dark theme. Here is the exact specification:

Tech Stack
Next.js 14 (App Router) + TypeScript
Tailwind CSS v3 with CSS custom properties for theming
GSAP for hero entrance animations
Framer Motion (AnimatePresence, motion) for the loading screen
Google Fonts: Inter (body, variable --font-body) and Instrument Serif (display/headings, variable --font-display, weight 400, italic)

Dark Theme — CSS Variables
Set on [data-theme="dark"] (force dark mode, no toggle):

--bg: #0a0a0a
--surface: #141414
--text: #f5f5f5
--muted: #888888
--accent: #f5f5f5
--stroke: #1f1f1f

Map these in Tailwind config as bg, surface, text, muted, accent, stroke color tokens. Font families: font-display → var(--font-display), font-body → var(--font-body).

Accent Gradient (used everywhere)
linear-gradient(90deg, #89AACC 0%, #4E85BF 100%)
This blue gradient is used for: the navbar logo ring, button hover borders, the "Say hi" hover ring, and the loading screen progress bar.

Component 1: Loading Screen
A full-screen loader (fixed inset-0 z-[9999]) with solid bg-bg background. It runs for 2.7 seconds, then fades out with Framer Motion exit={{ opacity: 0 }} over 0.6s.

Layout (3 elements):

Top-left: The word "Portfolio" — text-xs md:text-sm, text-muted, uppercase, tracking-[0.3em]. Positioned top-8 left-8 md:top-12 md:left-12. Animates in: y: -20 → 0, opacity: 0 → 1, duration 0.6s, delay 0.1s.
Center: Three words rotate in sequence — "Design" → "Create" → "Inspire" — one every 900ms. Styled text-4xl md:text-6xl lg:text-7xl font-display italic text-text/80. Uses AnimatePresence mode="wait", each word enters from y: 20, exits to y: -20, duration 0.4s, easing [0.4, 0, 0.2, 1].
Bottom-right: A counter that counts from 000 → 100 over 2.7s using requestAnimationFrame. Styled text-6xl md:text-8xl lg:text-9xl font-display text-text tabular-nums. Positioned bottom-8 right-8 md:bottom-12 md:right-12. Animates in from y: 20.
Progress bar: A thin 3px line at the very bottom. Background track is bg-stroke/50. The fill uses the accent gradient (#89AACC → #4E85BF) with a subtle glow (box-shadow: 0 0 8px rgba(137, 170, 204, 0.35)). Scales from scaleX(0) to scaleX(1) using transform-origin: left.

Behavior: After the counter hits 100, wait 400ms, then call onComplete(). The parent AppWrapper toggles isLoading to false, which fades the loader out and fades the page content in (opacity 0 → 1, transition 0.5s ease-out).

Component 2: Navbar (inside Hero, fixed)
A floating pill navbar, fixed top-0 left-0 right-0, centered with flex justify-center, z-50.

Pill container: inline-flex, rounded-full, backdrop-blur-md, border border-white/10, bg-surface, px-2 py-2. On scroll past 100px, adds shadow-md shadow-black/10.

Contents (left to right):

Logo — a 36x36px circle (w-9 h-9) with the accent gradient as a 2px ring (p-[2px]). The inside is bg-bg with the letters "JA" centered in text-[13px] font-display italic tracking-tighter. On hover the gradient rotates (from/to colors swap) and the text scales 110%.
Divider — w-px h-5 bg-stroke mx-1 (hidden on mobile)
Nav links: "Home", "Work", "Resume" — text-xs sm:text-sm, rounded-full, px-3 sm:px-4 py-1.5 sm:py-2. Active state: text-text bg-stroke/50. Hover: text-text bg-stroke/50.
Divider
"Say hi ↗" button — same pill styling, with a gradient border ring on hover.
Divider

Component 3: Hero Section
Full viewport height (min-h-screen), flex column, centered content.

Background video layer (absolute inset-0 z-0):
Video URL: https://stream.mux.com/Gs3wZfrtz6ZfqZqQ02c02Z7lugV00FGZvRpcqFTel66r3g.m3u8
An <video> element: autoPlay muted loop playsInline, with a .avif poster image as fallback.
The video is centered and covers the area: absolute top-1/2 left-1/2 min-w-full min-h-full -translate-x-1/2 -translate-y-1/2 object-cover.
A subtle overlay: absolute inset-0 bg-black/20.
A bottom fade gradient: absolute inset-x-0 bottom-0 h-48 bg-gradient-to-t from-bg to-transparent.

Content (centered, z-10, text-center):

Eyebrow label: "COLLECTION '26" — text-xs text-muted uppercase tracking-[0.3em] mb-8. Class blur-in.
Name: "Michael Smith" — text-6xl md:text-8xl lg:text-9xl font-display italic leading-[0.9] tracking-tight text-text mb-6. Class name-reveal.
Role line: A [Role] lives in Chicago. — text-lg md:text-xl lg:text-2xl text-muted mb-10. The [Role] cycles through "Creative" → "Fullstack" → "Founder" → "Scholar" every 2 seconds, styled as font-display italic text-text with a CSS animate-fade-in animation.
Bio: "Designing seamless digital interactions by focusing on the unique nuances which bring systems to life." — text-sm md:text-base text-muted leading-relaxed max-w-md mb-12.
CTA buttons (side by side):
"See Works": px-7 py-3.5 bg-text text-bg text-sm rounded-full. On hover: scale-105, gradient border ring appears.
"Reach out...": px-7 py-3.5 bg-bg text-text text-sm rounded-full border-2 border-stroke. Same gradient hover border technique.

Scroll indicator (bottom center, absolute bottom-8):
The word "SCROLL" — text-xs text-muted uppercase tracking-[0.2em].
Below it, a thin vertical line (w-px h-10 bg-stroke) with an animated dot sliding down on a 1.5s infinite loop.

GSAP Entrance Animations (Hero)
On mount, a GSAP timeline (power3.out ease):
.name-reveal: opacity 0→1, y 50→0, duration 1.2s, starting at 0.1s.
.blur-in: opacity 0→1, filter blur(10px)→blur(0px), y 20→0, duration 1s, stagger 0.1s, starting at 0.3s.

## Viktor Portfolio — Portfolio [sites/19]

- Preview: https://motionsites.ai/assets/hero-viktor-portfolio-preview-Bd2-Dg_u.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/19.gif

Build a high-end, cinematic 2-page architectural portfolio using React, Tailwind CSS, and Framer Motion (motion/react). The aesthetic is minimalist, dark-themed (black background, white text), and uses a sophisticated typographic hierarchy.

Global Configuration:

Fonts: Import and use 'Orbitron' for display headings, 'Space Grotesk' for body text, and 'JetBrains Mono' for technical/mono elements.

Selection: Custom selection color (white background, black text).

Icons: Use lucide-react (Snowflake, Maximize, Zap, ArrowLeft, ArrowRight).

Page 1: Hero (Modern Architect)

Background: Full-screen looping video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260304_101127_49ce07b7-f19a-4882-b19c-1d2a27d97ac3.mp4.

Settings: Muted, playsInline, loop, preload="auto".

Overlays: Radial vignette on desktop (70% transparent center to 70% black edges) and a bottom-fade gradient on mobile.

Top Nav: Right-aligned '1/01' counter with a progress line and a 'Next Project' button in mono font.

Main Content:

Large title: 'Viktor-O // MODERN ARCHITECT' (font-display, uppercase, tracking-tighter).

Description: Light-weight sans text with a max-width of 450px.

Technical Specs (Right Column): A list (Stack, Logic, Uptime, Scale) with labels and values separated by thin border-white/20.

Bottom Section:

A glass-morphism card (bg-white/5, backdrop-blur-xl) with a tech image (https://picsum.photos/seed/tech/200/200), project title 'VK-01: React Engine', and a 'View Project' button.

A row of pill-shaped tags (TS/JS, V1, Full-Stack, Cloud-Ready) at the bottom right.

Page 2: Project Details (Projecty Engine)

Background: Full-screen looping video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260304_102019_f84678ca-ffe7-49a5-895a-75ac1f71ad46.mp4.

Overlays: Subtle elliptical vignette to ensure text legibility.

Top Nav: 'Back Home' button with an arrow icon.

Main Content:

Massive display title on the left: 'PROJECTY ENGINE' (leading-0.85, uppercase).

Right-aligned description text about a flagship React engine, with a 'Read More' link featuring a minimalist underline.

Bottom Section:

Two structured info blocks: '01 // CORE ARCHITECTURE' and '02 // PERFORMANCE METRICS' with uppercase mono subtext.

Navigation arrows (Left/Right) in circular borders and a meta-info block (Date | Project) with an italicized caption.

Interactions & Responsiveness:

Use AnimatePresence for smooth opacity/exit transitions between pages.

Implement entrance animations for text and cards (opacity and y-offset).

On mobile: The video should occupy the top half (h-[50vh]) with a smooth gradient transition to the content below.

## 3D Jack Portfolio — Portfolio [sites/3d-jack-portfolio-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/jackportofplio.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/3d-jack-portfolio-hero.mp4

Build a 3D Creator portfolio landing page for "Jack" using React, TypeScript, Tailwind CSS, Framer Motion, and Lucide React. The page has a dark theme (#0C0C0C background) with the font Kanit (Google Fonts, weights 300-900). The page title is "Jack -- 3D Creator".

GLOBAL STYLES
Background: #0C0C0C on html, body, #root, and the main wrapper
Font family: 'Kanit', sans-serif
Global reset: box-sizing border-box, margin 0, padding 0
CSS class .hero-heading: gradient text using background: linear-gradient(180deg, #646973 0%, #BBCCD7 100%) with -webkit-background-clip: text and -webkit-text-fill-color: transparent
Main wrapper has overflowX: 'clip'
SECTION ORDER
HeroSection
MarqueeSection
AboutSection
ServicesSection
ProjectsSection
1. HERO SECTION
Full viewport height (h-screen), flex column layout with overflowX: clip.

Navbar: Horizontal nav bar with 4 links -- "About", "Price", "Projects", "Contact" -- evenly spaced with justify-between. Text color #D7E2EA, font-medium, uppercase, tracking-wider. Sizes: text-sm md:text-lg lg:text-[1.4rem]. Padding: px-6 md:px-10 pt-6 md:pt-8. Hover: opacity 70% with 200ms transition.

Hero Heading: Massive h1 with text "Hi, i'm jack" (lowercase "i", curly apostrophe via &apos;). Uses the .hero-heading gradient text class. Font-black, uppercase, tracking-tight, leading-none, whitespace-nowrap, w-full. Font sizes: text-[14vw] sm:text-[15vw] md:text-[16vw] lg:text-[17.5vw]. Margin top: mt-6 sm:mt-4 md:-mt-5. Wrapped in overflow-hidden container.

Bottom bar: Flexbox justify-between items-end with pb-7 sm:pb-8 md:pb-10:

Left: paragraph text "a 3d creator driven by crafting striking and unforgettable projects", color #D7E2EA, font-light, uppercase, tracking-wide, leading-snug. Font size: clamp(0.75rem, 1.4vw, 1.5rem). Max-width: max-w-[160px] sm:max-w-[220px] md:max-w-[260px].
Right: ContactButton component (see below)
Hero Portrait: Centered absolutely. Uses a Magnet component (mouse-following magnetic effect) wrapping an image. Image URL: https://shrug-person-78902957.figma.site/_components/v2/d24c01ad3a56fc65e942a1f501eb73db42d7cf9a/Rectangle_40443.81459862.png. Magnet settings: padding 150, strength 3, activeTransition "transform 0.3s ease-out", inactiveTransition "transform 0.6s ease-in-out". Positioning: absolute left-1/2 -translate-x-1/2 z-10. Width: w-[280px] sm:w-[360px] md:w-[440px] lg:w-[520px]. On mobile: top-1/2 -translate-y-1/2. On sm+: sm:top-auto sm:translate-y-0 sm:bottom-0.

FadeIn animations: Navbar fades in with delay 0, y -20. Heading: delay 0.15, y 40. Left text: delay 0.35, y 20. Contact button: delay 0.5, y 20. Portrait: delay 0.6, y 30.

2. MARQUEE SECTION
Two rows of images that scroll horizontally based on page scroll position. Background #0C0C0C. Padding: pt-24 sm:pt-32 md:pt-40 pb-10.

21 GIF images from motionsites.ai (exact URLs):


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
Row 1: first 11 images, tripled for seamless scrolling. Moves RIGHT on scroll (translateX(offset - 200)).
Row 2: remaining 10 images, tripled. Moves LEFT on scroll (translateX(-(offset - 200))).
Scroll offset calculated as: (window.scrollY - sectionTop + window.innerHeight) * 0.3
Each image tile: 420px x 270px, rounded-2xl, object-cover, lazy loaded.
Gap between tiles: gap-3. Gap between rows: gap-3.
Uses willChange: 'transform' for performance. Scroll listener is passive.
3. ABOUT SECTION
Full-height centered section with min-h-screen, padding px-5 sm:px-8 md:px-10 py-20.

Four decorative 3D images positioned absolutely in corners:

Top-left: Moon icon -- https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/moon_icon.11395d36.png -- w-[120px] sm:w-[160px] md:w-[210px], positioned top-[4%] left-[1%] sm:left-[2%] md:left-[4%]. FadeIn: delay 0.1, x -80, y 0, duration 0.9.
Bottom-left: 3D object -- https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/p59_1.4659672e.png -- w-[100px] sm:w-[140px] md:w-[180px], positioned bottom-[8%] left-[3%] sm:left-[6%] md:left-[10%]. FadeIn: delay 0.25, x -80, y 0, duration 0.9.
Top-right: Lego icon -- https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/lego_icon-1.703bb594.png -- w-[120px] sm:w-[160px] md:w-[210px], positioned top-[4%] right-[1%] sm:right-[2%] md:right-[4%]. FadeIn: delay 0.15, x 80, y 0, duration 0.9.
Bottom-right: 3D group -- https://shrug-person-78902957.figma.site/_components/v2/ebb2b8f25d8e24d5f0a5ca8af4c950de81aa2fd7/Group_134-1.2e04f3ce.png -- w-[130px] sm:w-[170px] md:w-[220px], positioned bottom-[8%] right-[3%] sm:right-[6%] md:right-[10%]. FadeIn: delay 0.3, x 80, y 0, duration 0.9.
Heading: "About me" using .hero-heading gradient text, font-black, uppercase, leading-none, tracking-tight, centered. Font size: clamp(3rem, 12vw, 160px). FadeIn: delay 0, y 40.

Animated paragraph: Uses a character-by-character scroll-driven opacity animation. Text: "With more than five years of experience in design, i focus on branding, web design, and user experience, i truly enjoy working with businesses that aim to stand out and present their best image. Let's build something incredible together!" -- color #D7E2EA, font-medium, centered, leading-relaxed, max-w-[560px], font size clamp(1rem, 2vw, 1.35rem). Each character animates from opacity 0.2 to 1 based on scroll progress, with scroll offset ['start 0.8', 'end 0.2'].

Contact button below the text block. Gap between heading/text: gap-10 sm:gap-14 md:gap-16. Gap between text block and button: gap-16 sm:gap-20 md:gap-24.

4. SERVICES SECTION
White background (#FFFFFF), with rounded-t-[40px] sm:rounded-t-[50px] md:rounded-t-[60px] top corners. Padding: px-5 sm:px-8 md:px-10 py-20 sm:py-24 md:py-32.

Heading: "Services" in #0C0C0C, font-black, uppercase, centered, font size clamp(3rem, 12vw, 160px). Margin bottom: mb-16 sm:mb-20 md:mb-28.

5 service items in a vertical list, max-w-5xl, centered:

01 - 3D Modeling: "Creation of detailed objects, characters, or environments tailored to specific client needs, ideal for games, products, and visualizations."
02 - Rendering: "High-quality, photorealistic renders that showcase designs with custom lighting, textures, and materials to bring concepts to life."
03 - Motion Design: "Dynamic animations and motion graphics that add energy and storytelling to brands, products, and digital experiences."
04 - Branding: "Crafting cohesive visual identities -- from logos to full brand systems -- that communicate a clear and memorable presence."
05 - Web Design: "Designing clean, modern, and conversion-focused websites with attention to layout, typography, and user experience."
Each item: horizontal layout with number (font-black, font size clamp(3rem, 10vw, 140px), color #0C0C0C) on the left and name + description stacked vertically on the right. Name: font-medium, uppercase, font size clamp(1rem, 2.2vw, 2.1rem). Description: font-light, leading-relaxed, max-w-2xl, font size clamp(0.85rem, 1.6vw, 1.25rem), opacity 0.6. Items separated by 1px borders (rgba(12, 12, 12, 0.15)). Padding: py-8 sm:py-10 md:py-12. Staggered FadeIn: each item delays by i * 0.1.

5. PROJECTS SECTION
Dark background (#0C0C0C), rounded top corners rounded-t-[40px] sm:rounded-t-[50px] md:rounded-t-[60px], pulled up with -mt-10 sm:-mt-12 md:-mt-14, z-10.

Heading: "Project" (singular) using .hero-heading gradient, same styling as other headings.

3 sticky-stacking project cards that scale down as you scroll past them (card stacking effect using Framer Motion useScroll and useTransform). Each card is sticky top-24 md:top-32 inside an h-[85vh] container.

Scale calculation: targetScale = 1 - (totalCards - 1 - index) * 0.03. Each card offset by top: ${index * 28}px.

Each card has: rounded-[40px] sm:rounded-[50px] md:rounded-[60px], border-2 border-[#D7E2EA], background #0C0C0C, padding p-4 sm:p-6 md:p-8.

Card layout:

Top row: Number (huge, same style as services), category label, project name, and a "Live Project" ghost button (rounded-full, border-2 #D7E2EA, uppercase, tracking-widest).
Bottom row: Two-column image grid -- left column (40% width) has 2 stacked images, right column (60%) has 1 tall image. All images have heavy border radius rounded-[40px] sm:rounded-[50px] md:rounded-[60px]. Left top image height: clamp(130px, 16vw, 230px). Left bottom image height: clamp(160px, 22vw, 340px).
Project data with CloudFront image URLs:

Project 01 - "Nextlevel Studio" (Client):

Col1 image 1: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055344_5eff02e0-87a5-41ce-b64f-eb08da8f33db.png&w=1280&q=85
Col1 image 2: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055431_11d841fd-8b41-46a5-82e4-b04f2407a7d8.png&w=1280&q=85
Col2 image: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055451_e317bf2d-28d4-48cc-86b0-6f72f25b6327.png&w=1280&q=85
Project 02 - "Aura Brand Identity" (Personal):

Col1 image 1: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055654_911201c5-36d9-4bc6-bac7-331adfce159f.png&w=1280&q=85
Col1 image 2: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055723_5ceda0b8-d9c2-4665-b2e3-83ba19ba76d1.png&w=1280&q=85
Col2 image: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055753_adc5dcbd-a8e6-49c0-b43a-9b030d835cea.png&w=1280&q=85
Project 03 - "Solaris Digital" (Client):

Col1 image 1: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055759_963cfb0b-4bd1-4b0f-9d0a-09bd6cf95b2f.png&w=1280&q=85
Col1 image 2: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_060108_438f781a-9846-4dcc-89ab-c4e6cb830f5b.png&w=1280&q=85
Col2 image: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260412_055818_9d062121-ad7e-46b9-999a-1a6a692ef1ee.png&w=1280&q=85
REUSABLE COMPONENTS
ContactButton: Rounded-full pill button with gradient background linear-gradient(123deg, #18011F 7%, #B600A8 37%, #7621B0 72%, #BE4C00 100%), inner box-shadow 0px 4px 4px rgba(181, 1, 167, 0.25), 4px 4px 12px #7721B1 inset, white 2px outline with -3px offset. Text: white, font-medium, uppercase, tracking-widest. Sizes: px-8 py-3 sm:px-10 sm:py-3.5 md:px-12 md:py-4, text text-xs sm:text-sm md:text-base. Label: "Contact Me".

LiveProjectButton: Ghost/outline pill button. Rounded-full, border-2 border-[#D7E2EA], text color #D7E2EA, font-medium, uppercase, tracking-widest. Sizes: px-8 py-3 sm:px-10 sm:py-3.5, text text-sm sm:text-base. Hover: bg-[#D7E2EA]/10. Label: "Live Project".

FadeIn: Framer Motion wrapper using whileInView with viewport={{ once: true, margin: "50px", amount: 0 }}. Accepts delay, duration (default 0.7), x (default 0), y (default 30). Easing: [0.25, 0.1, 0.25, 1]. Uses motion.create() for dynamic element types.

Magnet: Mouse-following magnetic hover effect. Tracks mouse position relative to element center, applies translate3d transform divided by strength factor. Activates when cursor is within padding distance of element edge. Smooth transition in (0.3s ease-out) and out (0.6s ease-in-out). Uses willChange: 'transform'.

AnimatedText: Character-by-character scroll-reveal text animation. Each character goes from opacity 0.2 to 1 based on its position in the text relative to scroll progress. Uses Framer Motion useScroll targeting the paragraph element with offset ['start 0.8', 'end 0.2']. Each character uses invisible placeholder + absolute positioned animated span.

KEY DEPENDENCIES
react, react-dom (^18.3.1)
framer-motion (^12.38.0)
lucide-react (^0.344.0)
tailwindcss (^3.4.1)
vite, typescript
RESPONSIVE BREAKPOINTS
All sections use Tailwind's default breakpoints (sm: 640px, md: 768px, lg: 1024px) with mobile-first approach. Heavy use of clamp() for fluid typography. The entire design scales gracefully from mobile to ultra-wide screens.

## Bold Portfolio Hero — Portfolio [sites/6]

- Preview: https://motionsites.ai/assets/hero-portfolio-bold-preview-9Yfbi-Wg.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/6.gif

Build a high-end, responsive Hero Section for a personal portfolio website using React and Tailwind CSS. The design should feel bold, modern, and energetic.

1. Global Styles & Theme:

Background: A vertical gradient from a vibrant red-orange [#fd2601] at the top to a lighter orange [#f37e1c] at the bottom.

Typography:
Headlines: Use the font "Anton" (or a similar heavy, condensed sans-serif). Text should be massive, uppercase, and tracking wide.

Body/UI: Use "Inter" or a clean sans-serif. Minimalist, uppercase, and legible.

Colors: All text and icons should be White.

Selection State: When user selects text, make the background White and text Orange [#fd2601].

2. Ambient Background Elements:

Place a large, faint SVG text or pattern (like the name "OLIVIA") in the absolute background. It should be centered, white, very low opacity (0.08), and blurred (4px).

Add glowing "blobs" behind the main content:

One bottom-right (300x300px, #F4791B, blur 80px).

One bottom-left (600x300px, #F4791B, blur 80px).

Use mix-blend-screen and opacity-60 for these blobs to make them blend beautifully.

3. Navigation Bar (Floating Top):

Position: Absolute top, full width, transparent background.

Left: Text logo "✱ VIKTORODDY" (Sans-serif, tracking wide).

Center (Desktop only): Links [PROJECTS, BLOG, ABOUT, RESUME]. Hover effect: opacity-80.

Right: A "HIRE ME" CTA.

Format: Text // HIRE ME followed by a circular button (white border) containing a diagonal arrow icon.

Hover: The circle fills white, icon turns orange.

4. Main Hero Content (Centered):

Headline: The text "NEW DESIGN ERA".

Desktop: Display on one line (or slightly wrapped), font size ~12vw (max 180px), z-index: 10.

Mobile: Stack vertically: "NEW" / "DESIGN" / "ERA".

Central Image: Place a portrait image of a person absolutely centered over the headline. CRITICAL: Apply a CSS Mask (paint brush stroke shape) to the image so it looks like a rough cutout, not a rectangle. The image should overlap the text, creating depth (z-index: 20).

Floating Elements (Desktop):

Bottom-Left: Intro text: "// I'm Olivia — a freelance UI/UX designer..." indented with a clean hierarchy.

Right-Middle: Tagline: "// DESIGN THAT [newline] SPEAKS YOUR BRAND". Text aligned right.

Mobile Layout: Move the floating elements below the main image/text stack so they don't overlap.

5. Footer / Brand Strip:

A row of white, semi-transparent (opacity-90) logos at the bottom (e.g., Gucci, Zara, Vogue, Sony, Zalora).

Desktop: Spread evenly across the width.

Mobile: Wrap them nicely in the center.

Technical Requirements:

Use flexbox for layout alignment.

Ensure the image is pointer-events-none so it doesn't block text selection.

Make it fully responsive: The massive text must scale down on mobile, and the layout must shift from absolute positioning (Desktop) to stacked block layout (Mobile).

## Portfolio Cosmic — Portfolio [sites/portfolio-cosmic-hero]

- Preview: https://motionsites.ai/assets/hero-portfolio-cosmic-preview-BpvWJ3Nc.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/portfolio-cosmic-hero.gif

Prompt to recreate this landing page:

Build a single-page dark portfolio landing page using React + Vite + Tailwind CSS + TypeScript + GSAP + Framer Motion + hls.js.

---

### Global Design System

### Fonts
Google Fonts import: Inter (300–700) and Instrument Serif (italic, 400).
- --font-body: 'Inter', sans-serif → Tailwind font-body
- --font-display: 'Instrument Serif', serif → Tailwind font-display

### CSS Custom Properties (HSL, no hsl() wrapper — Tailwind adds it)
--bg: 0 0% 4%;
--surface: 0 0% 8%;
--text: 0 0% 96%;
--muted: 0 0% 53%;
--stroke: 0 0% 12%;
--accent: 0 0% 96%;

### Tailwind Custom Colors
bg: "hsl(var(--bg))",
surface: "hsl(var(--surface))",
"text-primary": "hsl(var(--text))",
muted: "hsl(var(--muted))",
stroke: "hsl(var(--stroke))",

### Accent Gradient
linear-gradient(90deg, #89AACC 0%, #4E85BF 100%) — used on logo ring, hover borders, progress bars. CSS utility class .accent-gradient.

### Custom Animations (in index.css)
- @keyframes scroll-down — translateY(-100%) → translateY(200%), 1.5s ease-in-out infinite
- @keyframes role-fade-in — opacity 0 + translateY(8px) → opacity 1 + translateY(0), 0.4s ease-out
- @keyframes gradient-shift — background-position 0% 50% → 100% 50% → 0% 50%, 6s ease infinite (for animated gradient borders)

### Forced dark theme — no light mode toggle. body gets bg-bg text-text-primary.

---

### Page Structure (Index.tsx)

{isLoading && <LoadingScreen onComplete={() => setIsLoading(false)} />}

---

### Section 1: Loading Screen

Full-screen overlay (fixed inset-0 z-[9999] bg-bg). Uses requestAnimationFrame counter from 000→100 over 2700ms.

- Top-left: "Portfolio" label — text-xs text-muted uppercase tracking-[0.3em]. Animates y:-20→0, opacity 0→1.
- Center: Rotating words ["Design", "Create", "Inspire"] cycling every 900ms. AnimatePresence mode="wait" with y:20→0→-20 transitions. text-4xl md:text-6xl lg:text-7xl font-display italic text-text-primary/80.
- Bottom-right: Counter display — text-6xl md:text-8xl lg:text-9xl font-display text-text-primary tabular-nums. Shows String(count).padStart(3, "0").
- Bottom progress bar: h-[3px] bg-stroke/50, inner div with .accent-gradient, scaleX(count/100) transform, box-shadow: 0 0 8px rgba(137, 170, 204, 0.35).
- On complete (count reaches 100): 400ms delay then calls onComplete.

---

### Section 2: Hero

Full-viewport section with background HLS video and centered content.

### Background Video
- HLS source: https://stream.mux.com/Aa02T7oM1wH5Mk5EEVDYhbZ1ChcdhRsS2m1NYyx4Ua1g.m3u8
- Uses hls.js — if Hls.isSupported(), create HLS instance; else if native HLS support, set video.src directly.
- Video: autoPlay muted loop playsInline, absolutely positioned and centered with min-w-full min-h-full object-cover -translate-x-1/2 -translate-y-1/2.
- Dark overlay: bg-black/20
- Bottom fade: h-48 bg-gradient-to-t from-bg to-transparent

### Navbar (fixed, floats at top center)
fixed top-0 left-0 right-0 z-50 flex justify-center pt-4 md:pt-6 px-4.

Inner pill: inline-flex items-center rounded-full backdrop-blur-md border border-white/10 bg-surface px-2 py-2. Gets shadow-md shadow-black/10 when scrollY > 100.

Contents (left to right):
1. Logo: 9×9 circle with accent gradient border (reverses direction on hover). Inner bg-bg circle with "JA" in font-display italic text-[13px]. Scales 110% on hover.
2. Divider: w-px h-5 bg-stroke mx-1 (hidden on mobile)
3. Nav links: ["Home", "Work", "Resume"] — text-xs sm:text-sm rounded-full px-3 sm:px-4 py-1.5 sm:py-2. Active: text-text-primary bg-stroke/50. Inactive: text-muted hover:text-text-primary hover:bg-stroke/50.
4. Divider
5. "Say hi" button: Same size as nav links. On hover, shows accent gradient border behind (using absolute span with inset: -2px). Inner content wrapped in bg-surface rounded-full backdrop-blur-md. Includes "↗" arrow.

### Hero Content (centered, z-10)
- Eyebrow: text-xs text-muted uppercase tracking-[0.3em] mb-8 — "COLLECTION '26". Class blur-in.
- Name: text-6xl md:text-8xl lg:text-9xl font-display italic leading-[0.9] tracking-tight text-text-primary mb-6 — "Michael Smith". Class name-reveal.
- Role line: "A {role} lives in Chicago." — roles cycle every 2s through ["Creative", "Fullstack", "Founder", "Scholar"]. Role word uses font-display italic text-text-primary animate-role-fade-in inline-block with key={roleIndex} for re-triggering animation.
- Description: text-sm md:text-base text-muted max-w-md mb-12 — "Designing seamless digital interactions by focusing on the unique nuances which bring systems to life."
- CTA Buttons (inline-flex gap-4):
  - "See Works": Solid button. Default: bg-text-primary text-bg. Hover: bg-bg text-text-primary with accent gradient border ring.
  - "Reach out...": Outlined button. Default: border-2 border-stroke bg-bg text-text-primary. Hover: border-transparent with accent gradient border ring.
  - Both: rounded-full text-sm px-7 py-3.5 hover:scale-105.

### GSAP Entrance
Timeline with ease: "power3.out":
- .name-reveal: opacity 0→1, y 50→0, duration 1.2s, delay 0.1s
- .blur-in: opacity 0→1, filter blur(10px)→blur(0px), y 20→0, duration 1s, stagger 0.1, delay 0.3s

### Scroll Indicator
Bottom-center, text-xs text-muted uppercase tracking-[0.2em] "SCROLL" label above a w-px h-10 bg-stroke line with animated highlight using .animate-scroll-down.

---

### Section 3: Selected Works

bg-bg py-12 md:py-16. Inner: max-w-[1200px] mx-auto px-6 md:px-10 lg:px-16.

### Header
Framer Motion whileInView — opacity 0→1, y 30→0, duration 1s, ease [0.25,0.1,0.25,1], viewport once margin "-100px".
- Eyebrow: w-8 h-px bg-stroke + "Selected Work" text-xs text-muted uppercase tracking-[0.3em]
- Heading: "Featured *projects*" — italic word in font-display italic
- Subtext: "A selection of projects I've worked on, from concept to launch."
- "View all work" button (desktop only, hidden md:inline-flex) — rounded-full with gradient hover border ring + right arrow

### Bento Grid
grid grid-cols-1 md:grid-cols-12 gap-5 md:gap-6. Column spans alternate: 7/5/5/7.

4 project cards with titles: Automotive Motion, Urban Architecture, Human Perspective, Brand Identity.

Each card: bg-surface border border-stroke rounded-3xl with aspect ratios. Contains:
- Background image with object-cover group-hover:scale-105
- Halftone overlay: radial-gradient(circle, #000 1px, transparent 1px) at 4×4px, opacity-20 mix-blend-multiply
- Hover: bg-bg/70 opacity-0→1 + backdrop-blur-lg
- Hover label: pill with animated gradient border, white bg, "View — *Title*" (title in font-display italic)

---

### Section 4: Journal

bg-bg py-16 md:py-24. Same header pattern (eyebrow + "Recent *thoughts*" + subtext + "View all" button).

4 journal entries displayed as horizontal pills (rounded-[40px] sm:rounded-full) with titles, images, read times, and dates.

Each entry: flex items-center gap-6 p-4 bg-surface/30 hover:bg-surface border border-stroke.

---

### Section 5: Explorations (Parallax Gallery)

min-h-[300vh] section for scroll-driven parallax.

### Layer 1: Pinned Center (z-10)
h-screen div pinned with GSAP ScrollTrigger.create({ pin: contentRef, pinSpacing: false }).
- Eyebrow: "Explorations"
- Heading: "Visual *playground*"
- Subtext + Dribbble button

### Layer 2: Parallax Columns (z-20, absolute)
grid grid-cols-2 gap-12 md:gap-40 inside max-w-[1400px].

6 items split into 2 columns with GSAP scroll-driven parallax movement.
Cards: aspect-square max-w-[320px], with rotation and lightbox on click.

---

### Section 6: Stats

bg-bg py-16 md:py-24. 3-column grid with stats: 20+ Years Experience, 95+ Projects Done, 200% Satisfied Clients.

---

### Section 7: Contact / Footer

bg-bg pt-16 md:pt-20 pb-8 md:pb-12 overflow-hidden.

### Background Video
Same HLS source as hero, but flipped vertically (scale-y-[-1]). Heavier overlay: bg-black/60.

### GSAP Marquee
"BUILDING THE FUTURE • " repeated 10×. GSAP xPercent: -50, duration 40, ease "none", repeat -1.

### CTA
Email button: mailto:hello@michaelsmith.com with gradient hover border ring.

### Footer Bar
Social links [Twitter, LinkedIn, Dribbble, GitHub] + Green pulsing dot + "Available for projects"

---

### Dependencies
gsap, framer-motion, hls.js, react-router-dom, tailwindcss-animate

Add smooth scroll nav and page transitions.

## Pro AI Deck — Presentation [sites/pro-ai-deck]

- Preview: https://motionsites.ai/assets/hero-pro-ai-deck-preview-BBbLJNeM.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/pro-ai-deck.gif

PROMPT TO RECREATE STUDIO PITCH DECK SLIDES

Create a full-screen slide deck presentation app using React, TypeScript, Vite, Tailwind CSS, Motion (framer-motion), hls.js, and Lucide React icons. The presentation has 7 slides with horizontal swipe/scroll/keyboard navigation and animated transitions.

GLOBAL SETUP

Dependencies (package.json):

react ^18.3.1, react-dom ^18.3.1
motion ^12.38.0 (import from "motion/react")
hls.js ^1.6.15
lucide-react ^0.344.0
tailwindcss ^3.4.1, autoprefixer, postcss
vite ^5.4.2, @vitejs/plugin-react ^4.3.1
typescript ^5.5.3
Fonts (loaded via Google Fonts in index.html):
Instrument Serif (italic) -- used as font-heading for all headings
Barlow (weights 300, 400, 500, 600) -- used as font-body for all body text
Material Symbols Rounded (opsz 24, wght 400, FILL 1, GRAD 0) -- used for icons on slide 2
Load via these exact <link> tags in <head>:
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap" rel="stylesheet" />
<link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@24,400,1,0" rel="stylesheet" />
Tailwind Config:
fontFamily.heading: ["'Instrument Serif'", 'serif']
fontFamily.body: ["'Barlow'", 'sans-serif']
Custom CSS color variables for background, foreground, card, primary, secondary, muted, accent, destructive, border, input, ring

borderRadius.full: var(--radius) where --radius: 9999px
CSS Variables (index.css :root):
--background: 213 45% 67%;
--foreground: 0 0% 100%;
--card: 213 45% 62%;
--card-foreground: 0 0% 100%;
--primary: 0 0% 100%;
--primary-foreground: 213 45% 67%;
--secondary: 213 45% 72%;
--secondary-foreground: 0 0% 100%;
--muted: 213 35% 60%;
--muted-foreground: 0 0% 100% / 0.7;
--accent: 213 45% 72%;
--accent-foreground: 0 0% 100%;
--destructive: 0 84.2% 60.2%;
--border: 0 0% 100% / 0.2;
--input: 0 0% 100% / 0.2;
--ring: 0 0% 100% / 0.3;
--radius: 9999px;
--glass-bg: rgba(255, 255, 255, 0.12);
--glass-border: rgba(255, 255, 255, 0.25);
--glass-shadow: 0 4px 30px rgba(0, 0, 0, 0.08);
--glass-blur: 16px;
Glassmorphism CSS classes (defined in
@layer
 components in index.css):

.liquid-glass:
background: rgba(255, 255, 255, 0.01)
background-blend-mode: luminosity
backdrop-filter: blur(4px)
No border

box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1)
position: relative, overflow: hidden
::before pseudo-element creates a gradient border effect using mask-composite: exclude. The gradient goes 180deg from rgba(255,255,255,0.45) at 0% -> 0.15 at 20% -> 0 at 40% -> 0 at 60% -> 0.15 at 80% -> 0.45 at 100%, with 1.4px padding
.liquid-glass-strong:
Same as above but backdrop-filter: blur(50px)

box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15)
::before gradient slightly stronger: 0.5 at 0%/100%, 0.2 at 20%/80%
Body styles: font-family: 'Barlow', sans-serif; background: #000; color: #fff; overflow-x: hidden;
SHARED COMPONENTS
HlsVideo component (src/components/HlsVideo.tsx):
Props: src: string, className?: string, style?: React.CSSProperties
Uses a <video> element with autoPlay, loop, muted, playsInline
On mount: if src ends with .m3u8 and Hls.isSupported(), creates an Hls instance, calls loadSource and attachMedia; returns cleanup that calls hls.destroy(). Otherwise sets video.src directly.
BlurText component (src/components/BlurText.tsx):
Props: text: string, className?: string, delay?: number (default 100)
Splits text into words. Uses IntersectionObserver (threshold 0.2) to trigger animation on visibility.
Each word is a motion.span with initial={{ filter: 'blur(10px)', opacity: 0, y: 50 }}, animates through keyframes: [{ filter: 'blur(5px)', opacity: 0.5, y: -5 }, { filter: 'blur(0px)', opacity: 1, y: 0 }]
Duration 0.7s per word, staggered by (i * delay) / 1000, easeOut
Each word has class inline-block mr-[0.25em]
SlideControls component (src/components/SlideControls.tsx):
Fixed to bottom of screen: fixed bottom-0 left-0 right-0 z-50 px-8 lg:px-12 pb-6
Left side: slide counter 01 / 07 format (text-white/30, text-xs, tracking-[0.2em], uppercase) | divider (w-px h-4 bg-white/15) | animated slide label (text-white/50, text-xs)
Right side: dot indicators (h-1 rounded-full, active dot is w-24 bg-white, inactive is w-8 bg-white/20 hover:bg-white/40, transition-all duration-500) | divider | prev/next buttons using ChevronLeft/ChevronRight from lucide-react (w-8 h-8 rounded-full, text-white/50 hover:text-white hover:bg-white/10, disabled:opacity-20)
APP COMPONENT (src/App.tsx):
7 slides in order: TitleSlide ("Introduction"), ProblemSlide ("The Process"), CapabilitiesSlide ("Capabilities"), WhyUsSlide ("Differentiators"), StatsSlide ("Traction"), TestimonialsSlide ("Social Proof"), CtaSlide ("Next Steps")
AnimatePresence mode="wait" with custom direction
Slide transition variants:enter: { x: dir > 0 ? '100%' : '-100%', opacity: 0, scale: 0.95 }
center: { x: 0, opacity: 1, scale: 1 }
exit: { x: dir > 0 ? '-30%' : '30%', opacity: 0, scale: 0.95 }
transition: duration 0.65, ease [0.4, 0, 0.2, 1]

Navigation: mouse wheel (threshold 30px delta), keyboard (ArrowRight/ArrowDown/Space = next, ArrowLeft/ArrowUp = prev), touch swipe (threshold 60px)

Animation lock: 800ms cooldown between navigations using useRef
SLIDE 1 -- TitleSlide (src/slides/TitleSlide.tsx):
Background: Full-screen <video> (autoPlay, loop, muted, playsInline, object-cover) with NO overlay. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260411_104229_49794008-3d16-4cb6-9a8c-73d7751b0e79.mp4
Layout: relative z-10, full height, flex column, justify-between, px-10 lg:px-20 py-12 pb-20
Top bar (motion.div, fade from y:-10, delay 0.1):Logo: w-8 h-8 rounded-full bg-white with "S" in black font-heading italic text-sm
"Studio" in font-heading italic text-white/80 text-lg
Divider: w-px h-5 bg-white/20 mx-2
"Pitch Deck 2026" in text-white/30 font-body text-[10px] tracking-[0.2em] uppercase

Center content (flex-1, flex column, justify-center, max-w-3xl):Pill badge (motion.div, scale from 0.95, delay 0.2): liquid-glass rounded-full, contains "New" badge (bg-white text-black rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-wider) and "AI-powered web design" (text-white/70 text-xs font-light)
H1: text-5xl md:text-7xl lg:text-8xl xl:text-[6.5rem] font-heading italic text-white leading-[0.85] tracking-[-3px] mb-8 using BlurText component with text "The Website Your Brand Deserves" delay={80}
Subtitle (motion.p, slide from x:-20, delay 0.9): "Stunning design. Blazing performance. Built by AI, refined by experts." -- text-base md:text-lg text-white/50 font-body font-light leading-relaxed max-w-xl mb-10
CTA button (motion.div, slide from x:-20, delay 1.1): liquid-glass-strong rounded-full px-6 py-3, "Get Started" with ArrowUpRight icon (w-4 h-4)

Bottom trusted-by bar (motion.div, fade delay 1.4): "Trusted by" label + ['Stripe', 'Vercel', 'Linear', 'Notion', 'Figma'] in text-lg md:text-xl font-heading italic text-white/20
SLIDE 2 -- ProblemSlide (src/slides/ProblemSlide.tsx):

Background: HlsVideo component with src https://stream.mux.com/9JXDljEVWYwWu01PUkAemafDugK89o01BR6zqJ3aS9u00A.m3u8 (absolute inset-0, object-cover)
Overlay: <div className="absolute inset-0 bg-black/60 z-[1]" /> -- 60% black overlay
Layout: relative z-10, full height, flex, px-10 lg:px-20 py-12 pb-20. Inner div: flex-col lg:flex-row, items-start, gap-12 lg:gap-20, w-full, my-auto
Left column (flex-1):Section label (motion.span, slide from x:-15, delay 0.1): "The Process" -- text-white/30 font-body text-[10px] tracking-[0.3em] uppercase, mb-6
Headline (motion.h2, slide from x:-20, delay 0.2): "Our AI simplifies data analysis, eliminates decision bottlenecks, and seamlessly integrates" -- text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-heading italic text-white tracking-tight leading-[0.9] mb-8
Body (motion.p, slide from x:-15, delay 0.4): "Our AI algorithms strategically address industry challenges, enhancing efficiency and facilitating optimal decision-making, providing a definitive solution for businesses in the AI era." -- text-white/40 font-body font-light text-sm md:text-base leading-relaxed max-w-xl

Right column (w-full lg:w-[420px], flex-col gap-4, shrink-0): 3 cards, each staggered (delay 0.3 + i*0.12, slide from x:30):Card container: liquid-glass rounded-2xl p-6 lg:p-7

Icon + title row: flex items-center gap-3 mb-3Icon container: w-10 h-10 rounded-xl bg-gradient-to-br from-white/15 to-white/5
Icon: <span className="material-symbols-rounded text-white/80 text-xl">{iconName}</span>
Title: text-base font-body font-semibold text-white
Description: text-sm font-body font-light text-white/40 leading-relaxed pl-[52px]

Card data:icon: "query_stats", title: "Streamlined Analytics", desc: "Our AI simplifies intricate data analysis, providing businesses with quick and accurate insights."
icon: "psychology", title: "Decision Optimization", desc: "Eliminate decision-making bottlenecks for timely and informed choices, enhancing overall operational efficiency."
icon: "integration_instructions", title: "Effortless Integration", desc: "Our algorithms seamlessly integrate AI with plug-and-play ease, empowering businesses without disruption."

SLIDE 3 -- CapabilitiesSlide (src/slides/CapabilitiesSlide.tsx):

Background: HlsVideo component with src https://stream.mux.com/s8pMcOvMQXc4GD6AX4e1o01xFogFxipmuKltNfSYza0200.m3u8 (absolute inset-0, object-cover, style={{ opacity: 0.5 }})
Base: bg-black behind the video
Layout: relative z-10, full height, flex column, px-10 lg:px-20 py-12 pb-20
Section label (motion.span, slide from x:-15, delay 0.1): "Capabilities" -- text-white/30 font-body text-[10px] tracking-[0.3em] uppercase, mb-4

Headline (motion.h2, slide from x:-20, delay 0.2): "Pro features.\nZero complexity." (line break with <br />) -- text-6xl md:text-8xl lg:text-9xl xl:text-[10rem] font-heading italic text-white tracking-tight leading-[0.85] mb-8 lg:mb-auto
Cards grid: grid-cols-1 lg:grid-cols-2 gap-6 lg:gap-8. Two cards, staggered (delay 0.35 + i*0.15, slide from y:30):Card container: liquid-glass rounded-2xl overflow-hidden flex flex-col

Video area: h-44 lg:h-56, overflow-hidden, relative. Contains either:MP4 video (<video> with autoPlay, loop, muted, playsInline, object-cover)
HLS video (HlsVideo component, object-cover)

Text area: p-6 lg:p-8Title: text-lg md:text-xl font-heading italic text-white mb-2 leading-tight
Body: text-sm font-body font-light text-white/40 leading-relaxed

Card data:title: "Designed to convert. Built to perform.", body: "Every pixel is intentional. Our AI studies what works across thousands of top sites -- then builds yours to outperform them all.", videoSrc: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260302_085844_21a8f4b3-dea5-4ede-be16-d53f6973bb14.mp4 (MP4)
title: "It gets smarter. Automatically.", body: "Your site evolves on its own. AI monitors every click, scroll, and conversion -- then optimizes in real time.", videoSrc: https://stream.mux.com/T6oQJQ02cQ6N01TR6iHwZkKFkbepS34dkkIc9iukgy400g.m3u8 (HLS)

SLIDE 4 -- WhyUsSlide (src/slides/WhyUsSlide.tsx):
Background: Full-screen <video> (autoPlay, loop, muted, playsInline, object-cover) with NO opacity reduction. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260411_104032_69319010-2458-492b-b04d-b40a5dfa4482.mp4
Gradient overlay: <div className="absolute bottom-0 left-0 right-0 h-[50%] z-[1] pointer-events-none" style={{ background: 'linear-gradient(to top, black, transparent)' }} /> -- black-to-transparent gradient from bottom to middle
Layout: relative z-10, full height, flex column, px-10 lg:px-20 py-12 pb-20

Top section (flex-col lg:flex-row lg:items-end lg:justify-between, mb-auto):Left: section label "Why Us" (text-white/30, text-[10px], tracking-[0.3em], uppercase, mb-4) + headline "The difference\nis everything." (text-3xl md:text-4xl lg:text-5xl font-heading italic text-white tracking-tight leading-[0.9])
Right: subtitle (motion.p, slide from x:20, delay 0.3): "We do not just build websites. We engineer competitive advantages that compound over time." -- text-white/35 font-body font-light text-sm max-w-sm

Bottom cards (flex-1 flex items-end): grid-cols-2 lg:grid-cols-4, gap-4 lg:gap-6, w-full. 4 cards staggered (delay 0.3 + i*0.1, slide from y:30):Card: liquid-glass rounded-2xl p-6 lg:p-8 flex flex-col
Icon: liquid-glass-strong rounded-full w-10 h-10 containing Lucide icon (w-4 h-4 text-white), mb-6
Title: text-sm md:text-base font-body font-semibold text-white mb-2
Desc: text-xs font-body font-light text-white/40 leading-relaxed

Card data (icons from lucide-react):Zap, "Days, Not Months", "Concept to launch at a pace that redefines fast. Because waiting is not a strategy."
Palette, "Obsessively Crafted", "Every detail considered. Every element refined. Design so precise, it feels inevitable."
BarChart3, "Built to Convert", "Layouts informed by data. Decisions backed by performance. Results you can measure."
Shield, "Secure by Default", "Enterprise-grade protection comes standard. SSL, DDoS mitigation, compliance. All included."

SLIDE 5 -- StatsSlide (src/slides/StatsSlide.tsx):
Background: HlsVideo component with src https://stream.mux.com/NcU3HlHeF7CUL86azTTzpy3Tlb00d6iF3BmCdFslMJYM.m3u8 (absolute inset-0, object-cover, style={{ filter: 'saturate(0)' }} -- desaturated/grayscale). NO overlay.
Layout: relative z-10, full height, flex column, px-10 lg:px-20 py-12 pb-20
Top section (mb-auto):Section label "Traction" (text-white/30, text-[10px], tracking-[0.3em], uppercase, mb-6)
Headline: "Numbers that speak for themselves" -- text-4xl md:text-5xl lg:text-7xl font-heading italic text-white tracking-tight leading-[0.9] max-w-4xl

Stats grid: flex-col gap-6, containing two rows of grid-cols-1 lg:grid-cols-2 gap-4First row: stats[0] and stats[1], staggered delay 0.35 + i*0.1
Second row: stats[2] and stats[3], staggered delay 0.55 + i*0.1

Each stat: flex-col gap-8Top divider: <div className="h-px bg-white/20" />

Content: flex items-start gap-10 lg:gap-14Number: text-7xl md:text-8xl lg:text-[9.5rem] font-heading italic text-white leading-none shrink-0 (WHITE color, not blue)
Description: pt-3 lg:pt-4 pr-8 lg:pr-20 flex-1, text-white text-base md:text-lg lg:text-2xl font-body font-normal leading-relaxed

Stat data:"200+" -- "Sites launched and generating measurable results for brands across industries"
"98%" -- "Client satisfaction rate across all projects delivered in the last two years"
"3.2x" -- "Average conversion uplift compared to previous client sites and industry benchmarks"
"5 days" -- "Average delivery from concept to production-ready launch across all project types"

SLIDE 6 -- TestimonialsSlide (src/slides/TestimonialsSlide.tsx):

Background: Full-screen <video> (autoPlay, loop, muted, playsInline, object-cover) with NO overlay. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260406_094145_4a271a6c-3869-4f1c-8aa7-aeb0cb227994.mp4
Layout: relative z-10, full height, flex column, px-10 lg:px-20 py-12 pb-20
Section label: "Social Proof" (text-white/30, text-[10px], tracking-[0.3em], uppercase, mb-4)
Headline: "Don't take our\nword for it." -- text-3xl md:text-4xl lg:text-5xl font-heading italic text-white tracking-tight leading-[0.9] mb-auto
Testimonial cards grid: grid-cols-1 lg:grid-cols-3 gap-5 lg:gap-6. 3 cards staggered (delay 0.3 + i*0.12, slide from y:25):Card: liquid-glass rounded-2xl p-8 lg:p-10 flex flex-col justify-between
Quote section (mb-8): opening curly quote in text-3xl font-heading italic text-white/15 block mb-4, then quote text in text-white/70 font-body font-light text-sm lg:text-base italic leading-relaxed

Attribution (flex items-center gap-3, pt-4 border-t border-white/10):Avatar: w-8 h-8 rounded-full bg-white/10, initials in text-white/60 font-body text-xs font-medium
Name: text-white font-body font-medium text-sm
Role: text-white/40 font-body font-light text-xs

Testimonial data:"A complete rebuild in five days. The result outperformed everything we had spent months building before." -- Sarah Chen, CEO, Luminary
"Conversions up 4x. That is not a typo. The design just works differently when it is built on real data." -- Marcus Webb, Head of Growth, Arcline
"They did not just design our site. They defined our brand. World-class does not begin to cover it." -- Elena Voss, Brand Director, Helix

SLIDE 7 -- CtaSlide (src/slides/CtaSlide.tsx):
Background: Full-screen <video> (autoPlay, loop, muted, playsInline, object-cover) with NO overlay. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_024928_1efd0b0d-6c02-45a8-8847-1030900c4f63.mp4
Layout: relative z-10, full height, flex column, px-10 lg:px-20 py-12 pb-20
Section label: "Next Steps" (text-white/30, text-[10px], tracking-[0.3em], uppercase, mb-4)
Main content (flex-1, flex-col lg:flex-row, items-start lg:items-center, gap-12 lg:gap-20):
Left column (flex-1, max-w-2xl):Headline (motion.h2, slide from x:-25, delay 0.2): "Your next website\nstarts here." -- text-5xl md:text-6xl lg:text-7xl xl:text-8xl font-heading italic text-white leading-[0.85] tracking-tight mb-6
Body (motion.p, slide from x:-20, delay 0.5): "Book a free strategy call. See what AI-powered design can do. No commitment, no pressure. Just possibilities." -- text-white/40 font-body font-light text-sm md:text-base leading-relaxed max-w-md mb-10

Buttons (motion.div, slide from x:-15, delay 0.7): flex items-center gap-4Primary: bg-white text-black rounded-full px-6 py-3 text-sm font-body font-semibold with "Book a Call" + ArrowUpRight icon
Secondary: liquid-glass-strong rounded-full px-6 py-3 text-sm font-body font-medium text-white with "View Pricing"

Right column (motion.div, slide from x:30, delay 0.6): liquid-glass rounded-2xl p-8 lg:p-10 w-full max-w-xsHeader: Mail icon in liquid-glass-strong rounded-full w-10 h-10 + "Get in touch" label
Contact info: hello@studio.ai, +1 (555) 000-0000
Locations (pt-4 border-t border-white/10): San Francisco, CA and London, UK in text-white/30 text-xs

Footer (motion.div, fade delay 1.0): flex justify-between, border-t border-white/10 pt-4Left: "(c) 2026 Studio. All rights reserved." in text-white/30 text-xs
Right: Privacy, Terms, Contact links in text-white/30 text-xs hover:text-white/60

VIDEO URL REFERENCE:
Slide 1 BG: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260411_104229_49794008-3d16-4cb6-9a8c-73d7751b0e79.mp4
Slide 2 BG: https://stream.mux.com/9JXDljEVWYwWu01PUkAemafDugK89o01BR6zqJ3aS9u00A.m3u8 (HLS)
Slide 3 BG: https://stream.mux.com/s8pMcOvMQXc4GD6AX4e1o01xFogFxipmuKltNfSYza0200.m3u8 (HLS, 50% opacity)
Slide 3 Card 1 video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260302_085844_21a8f4b3-dea5-4ede-be16-d53f6973bb14.mp4
Slide 3 Card 2 video: https://stream.mux.com/T6oQJQ02cQ6N01TR6iHwZkKFkbepS34dkkIc9iukgy400g.m3u8 (HLS)
Slide 4 BG: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260411_104032_69319010-2458-492b-b04d-b40a5dfa4482.mp4
Slide 5 BG: https://stream.mux.com/NcU3HlHeF7CUL86azTTzpy3Tlb00d6iF3BmCdFslMJYM.m3u8 (HLS, desaturated)
Slide 6 BG: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260406_094145_4a271a6c-3869-4f1c-8aa7-aeb0cb227994.mp4
Slide 7 BG: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_024928_1efd0b0d-6c02-45a8-8847-1030900c4f63.mp4
FILE STRUCTURE:
index.html
package.json
tailwind.config.js
postcss.config.js
vite.config.ts
tsconfig.json / http://tsconfig.app.json / tsconfig.node.json
src/
  main.tsx
  App.tsx
  index.css
  vite-env.d.ts
  components/
    BlurText.tsx
    HlsVideo.tsx
    SlideControls.tsx
  slides/
    TitleSlide.tsx
    ProblemSlide.tsx
    CapabilitiesSlide.tsx
    WhyUsSlide.tsx
    StatsSlide.tsx
    TestimonialsSlide.tsx
    CtaSlide.tsx
