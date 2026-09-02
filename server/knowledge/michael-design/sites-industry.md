# Michael Design Library — sites-industry

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 21 entries.

## 3D Collectible Hero — 3D Website [sites/3d-collectible-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(24).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/3d-collectible-hero.webp

Build a single full-viewport hero section in React + TypeScript + Vite + Tailwind CSS, using `lucide-react` for icons. The component is a character-figurine carousel called "TOONHUB".

**Fonts (load in `index.html` head):**
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Anton&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
```
Body font: `'Inter', sans-serif`. Display font (huge ghost text + bottom-right link): `'Anton', sans-serif`.

**Image data (4 items, exact URLs and colors):**
```ts
const IMAGES = [
  { src: 'https://fifth-gentle-45902158.figma.site/_components/v2/4de492f6d9cf8244ad5293233e5c6f52407d42fc/1.02464a56.png', bg: '#F4845F', panel: '#F79B7F' },
  { src: 'https://fifth-gentle-45902158.figma.site/_components/v2/4de492f6d9cf8244ad5293233e5c6f52407d42fc/2.b977faab.png', bg: '#6BBF7A', panel: '#85CC92' },
  { src: 'https://fifth-gentle-45902158.figma.site/_components/v2/4de492f6d9cf8244ad5293233e5c6f52407d42fc/3.4df853b4.png', bg: '#E882B4', panel: '#ED9DC4' },
  { src: 'https://fifth-gentle-45902158.figma.site/_components/v2/4de492f6d9cf8244ad5293233e5c6f52407d42fc/4.4457fbce.png', bg: '#6EB5FF', panel: '#8DC4FF' },
];
```
Preload all 4 images on mount via `new Image()`.

**State & logic:**
- `activeIndex` (0–3), `isAnimating` boolean lock, `isMobile` (`window.innerWidth < 640`, updated on resize).
- `navigate('next' | 'prev')`: ignore if animating; set `isAnimating=true`; bump `activeIndex` `(prev+1)%4` or `(prev+3)%4`; release lock after `650ms`.
- Roles derived from activeIndex: `center=activeIndex`, `left=(activeIndex+3)%4`, `right=(activeIndex+1)%4`, `back=(activeIndex+2)%4`.

**Layout structure:**
Outer `<div>` has `backgroundColor: IMAGES[activeIndex].bg`, transition `background-color 650ms cubic-bezier(0.4,0,0.2,1)`, `fontFamily: 'Inter, sans-serif'`, `relative w-full overflow-hidden`. Inside, a `relative w-full` div with `height: 100vh; overflow: hidden`.

1. **Grain overlay** (`absolute inset-0 pointer-events-none`, zIndex 50): SVG fractalNoise data URI, `baseFrequency=0.9`, `numOctaves=4`, opacity 0.08 inside SVG, container `opacity: 0.4`, `backgroundSize: 200px 200px`, repeat.

2. **Giant ghost text "3D SHAPE"** (`absolute inset-x-0 flex items-center justify-center pointer-events-none select-none`, zIndex 2, `top: 18%`): font Anton, `fontSize: clamp(90px, 28vw, 380px)`, weight 900, color white, opacity 1, lineHeight 1, uppercase, letterSpacing `-0.02em`, whiteSpace nowrap.

3. **Top-left brand label "TOONHUB"** (`absolute top-6 left-4 sm:left-8`, zIndex 60): `text-xs font-semibold uppercase`, white, opacity 0.9, letterSpacing `0.18em`.

4. **Carousel** (`absolute inset-0`, zIndex 3): map all 4 IMAGES; each item is `position:absolute`, `aspectRatio: '0.6 / 1'`, with role-based styles below. Inside, an `<img>` `width:100%; height:100%; objectFit:contain; objectPosition:bottom center; draggable=false`.

   Per-role style:
   - **center**: `transform: translateX(-50%) scale(${isMobile?1.25:1.68})`, no blur, opacity 1, zIndex 20, `left:50%`, `height: isMobile?'60%':'92%'`, `bottom: isMobile?'22%':0`.
   - **left**: `translateX(-50%) scale(1)`, blur 2px, opacity 0.85, zIndex 10, `left: isMobile?'20%':'30%'`, `height: isMobile?'16%':'28%'`, `bottom: isMobile?'32%':'12%'`.
   - **right**: same as left but `left: isMobile?'80%':'70%'`.
   - **back**: `translateX(-50%) scale(1)`, blur 4px, opacity 1, zIndex 5, `left:50%`, `height: isMobile?'13%':'22%'`, `bottom: isMobile?'32%':'12%'`.

   Transition on each item: `transform 650ms cubic-bezier(0.4,0,0.2,1), filter 650ms ..., opacity 650ms ..., left 650ms ...`. `willChange: transform, filter, opacity`.

5. **Bottom-left text + nav buttons** (`absolute bottom-6 left-4 sm:bottom-20 sm:left-24`, zIndex 60, `maxWidth:320px`):
   - `<p>` "TOONHUB FIGURINES" — bold uppercase, tracking-widest, `mb-2 sm:mb-3 text-base sm:text-[22px]`, white, opacity 0.95, letterSpacing `0.02em`.
   - `<p>` (hidden on mobile, `hidden sm:block`): "The artwork is stunning, shipped fully prepared. The finish is a vision, the 3D craft is flawless. Many thanks! Wishing you the win. Order now." — `text-xs sm:text-sm`, white, opacity 0.85, lineHeight 1.6, `mb-4 sm:mb-5`.
   - Two circular buttons (`w-12 h-12 sm:w-16 sm:h-16`, transparent bg, 2px white border, white icon): `ArrowLeft` and `ArrowRight` from lucide-react, size 26, strokeWidth 2.25. On hover: scale 1.08 + bg `rgba(255,255,255,0.12)`. Transition `transform 150ms, background-color 150ms`. Click triggers `navigate('prev')` / `navigate('next')`.

6. **Bottom-right link "DISCOVER IT"** (`absolute bottom-6 right-4 sm:bottom-20 sm:right-10`, zIndex 60): `<a>` flex items-center, font Anton, `fontSize: clamp(20px, 4vw, 56px)`, weight 400, white, opacity 0.95→1 on hover (200ms), letterSpacing `-0.02em`, lineHeight 1, uppercase, no underline. Followed by `ArrowRight` (`w-5 h-5 sm:w-8 sm:h-8`, strokeWidth 2.25).

**Behavior summary:** clicking arrows rotates roles; background color, image positions, scales, blurs, and opacities all crossfade simultaneously over 650ms with `cubic-bezier(0.4,0,0.2,1)`. The character images sit at the bottom of the screen overlapping the giant "3D SHAPE" text behind them.

## Pulse 3D — 3D Website [sites/pulse-3d]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(16).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/pulse-3d.webp

### PROJECT OVERVIEW

Build a **single-screen, scroll-driven, custom-gesture landing page** called **"Inner Circle"**. There is **no native browser scrolling** — `document.body` and `html` both have `overflow: hidden`. A wheel/touch gesture controller drives a single `scrollProgress` numeric state from `0` to `3.5`. All animations (video scrubbing, text exits, rising panel, cylindrical drum) are derived from this single value.

### Stack
- **Vite + React 19 + TypeScript**
- **Tailwind CSS v4** (via `@tailwindcss/vite`)
- **GSAP 3.15** for char-level split-text and parallax wiggle
- **lucide-react** for menu icons (`Menu`, `X`)
- `motion`, `@google/genai`, `express`, `dotenv` installed (the page itself only needs GSAP + Tailwind + React)

### Fonts (loaded in `src/index.css`)
```css
@import url('https://fonts.googleapis.com/css2?family=Manrope:wght@300;400;500;600;700;800&family=Michroma&display=swap');
@import "tailwindcss";

@theme {
  --font-manrope: "Manrope", sans-serif;
  --font-michroma: "Michroma", sans-serif;
}
```
- Use `font-manrope` for body, paragraph drum text, header subtitle, nav.
- Use `font-michroma` for the giant hero title and tile labels.

### Global CSS (also in `index.css`)
```css
::-webkit-scrollbar { width: 8px; }
::-webkit-scrollbar-track { background: #11010a; }
::-webkit-scrollbar-thumb { background: #ea1f63; border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: #ff5c93; }

html, body {
  background-color: #11010a;
  color: #ffffff;
  overflow-x: hidden;
  font-family: var(--font-manrope);
}

@keyframes marquee-scroll {
  from { transform: translateX(0); }
  to   { transform: translateX(-50%); }
}
.marquee-container {
  display: flex; overflow: hidden; width: 100%; position: relative;
  mask-image: linear-gradient(to right, transparent, white 20%, white 80%, transparent);
  -webkit-mask-image: linear-gradient(to right, transparent, white 20%, white 80%, transparent);
}
.marquee-track {
  display: flex; width: max-content; flex-wrap: nowrap;
  animation: marquee-scroll linear infinite;
  will-change: transform;
}
```

### Color palette
- Hero background magenta: `#FF005E`
- Second screen near-black/wine: `#11010a`
- Loader accents: `#ea1f63`, `pink-500` (`#ec4899`), `#ff5c93`
- Text: white (`#ffffff`) and `text-white/60` for low-emphasis drum copy
- **No purple/indigo anywhere.**

### Data files

`src/types.ts`:
```ts
export interface NavigationItem { id: string; label: string; scrollRatio: number; }
export interface Project { title: string; category: string; description: string; tags: string[]; }
export interface ExpertiseItem { title: string; percentage: number; description: string; }
```

`src/data.ts`:
```ts
export const NAVIGATION_ITEMS: NavigationItem[] = [
  { id: "projects",  label: "Projects",  scrollRatio: 0.25 },
  { id: "expertise", label: "Expertise", scrollRatio: 0.50 },
  { id: "about",     label: "About",     scrollRatio: 0.95 },
  { id: "contact",   label: "Manifesto", scrollRatio: 3.50 },
];
```
(Keep `PROJECTS_DATA` and `EXPERTISE_DATA` as defined — unused on this page but kept for parity.)

---

### ROOT LAYOUT (`src/App.tsx`)

### State
- `scrollProgress: number` (0 → 3.5)
- `lerpedScrollProgress: number` — smoothed copy of `scrollProgress`, updated each rAF tick with `currentLerp += (target - currentLerp) * 0.08`. Threshold `0.0001`.
- `activeSectionId: string` — derived via `updateActiveSection(progress)`:
  - `< 0.18` → "hero"
  - `0.18–0.45` → "projects"
  - `0.45–0.68` → "expertise"
  - `0.68–1.15` → "about"
  - else → "contact"

### Gesture controller (runs once on mount)
- Sets `document.body.style.overflow = "hidden"` and same on `documentElement`.
- `wheel` listener (passive: false, `preventDefault()`): `scaleFactor = 0.0006`, new value = `clamp(prev + deltaY * 0.0006, 0, 3.5)`.
- `touchstart` saves `lastTouchY`. `touchmove`: `deltaTouchY = lastTouchY - currentTouchY`, `scaleFactor = 0.0015`, clamp same range.
- If a programmatic nav animation is in flight, cancel it on any user input.

### Programmatic navigation (`handleNavigateToSection`)
- Duration: `1200ms`, easeInOutCubic:
  ```
  ease = p < 0.5 ? 4p³ : 1 - (-2p + 2)³ / 2
  ```
- Lerps `scrollProgress` from current to `item.scrollRatio` while calling `updateActiveSection` each frame.

### Derived values
```ts
const secondScreenProgress = clamp01((lerped - 1.15) / 0.50);
const easedRisingProgress  = 1 - Math.pow(1 - secondScreenProgress, 3);
const smoothBlurAmount     = Math.sin(secondScreenProgress * Math.PI / 2) * 64;
```

### Markup skeleton
```tsx
<main className="relative w-screen h-screen overflow-hidden bg-[#FF005E] text-white">
  <div className="relative w-full h-full overflow-hidden">

    {/* FIRST SCREEN — gets blurred as second screen rises */}
    <div
      className="absolute inset-0 w-full h-full z-10 transition-transform duration-[100ms] ease-out"
      style={{ filter: secondScreenProgress > 0 ? `blur(${smoothBlurAmount}px)` : "none" }}
    >
      <VideoScrubber scrollProgress={Math.min(1, lerpedScrollProgress)} />

      {/* Hero title strip pinned to bottom */}
      <div className="absolute bottom-[40px] left-[1%] right-[1%] w-[98%] pointer-events-none z-20 select-none flex justify-center items-center">
        <ScrollExitSplitText
          scrollProgress={Math.min(1, lerpedScrollProgress)}
          containerClassName="w-full text-[10.4vw] leading-none font-michroma font-normal uppercase text-white whitespace-nowrap text-center transition-all duration-300 will-change-transform"
          style={{ letterSpacing: "-0.07em" }}
        >
          INNER CIRCLE
        </ScrollExitSplitText>
      </div>

      <SoapTiles scrollProgress={lerpedScrollProgress} />
    </div>

    <Header activeSectionId={activeSectionId} onNavigate={handleNavigateToSection} />

    {/* SECOND SCREEN — rises from below, rounded top */}
    <div
      className="absolute bottom-0 left-0 w-full h-full bg-[#11010a] rounded-t-[48px] overflow-hidden z-40"
      style={{
        transform: `translateY(${(1 - easedRisingProgress) * 100}%)`,
        visibility: secondScreenProgress > 0 ? "visible" : "hidden",
        willChange: "transform",
      }}
    >
      <div className="absolute top-5 left-1/2 -translate-x-1/2 w-16 h-[5px] bg-white rounded-full z-50 pointer-events-none" />
      <SecondVideoScrubber scrollProgress={lerpedScrollProgress} />
      <CylindricalTextDrum scrollProgress={lerpedScrollProgress} />

      <div className="absolute bottom-8 sm:bottom-12 md:bottom-16 left-0 w-full sm:w-[65%] md:w-[60%] pl-6 sm:pl-12 md:pl-20 pr-6 sm:pr-12 md:pr-16 z-50 pointer-events-auto">
        <div className="w-full border-t border-white/[0.08] pt-6">
          <Marquee gap="80px" speed={25} fade>
            <GoogleWordmark size={100} />
            <GithubWordmark size={100} />
            <img src="https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/voiceflow-logo-svg-150px.svg" alt="Voiceflow" className="h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity" referrerPolicy="no-referrer" />
            <img src="https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/zendesk-logo-svg-150px.svg"   alt="Zendesk"   className="h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity" referrerPolicy="no-referrer" />
            <img src="https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/pendo-logo-svg-150px.svg"     alt="Pendo"     className="h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity" referrerPolicy="no-referrer" />
            <img src="https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/glide-logo-svg-150px.svg"     alt="Glide"     className="h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity" referrerPolicy="no-referrer" />
            <img src="https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/canva-logo-svg-150px.svg"     alt="Canva"     className="h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity" referrerPolicy="no-referrer" />
          </Marquee>
        </div>
      </div>
    </div>
  </div>
</main>
```

---

### SECTION 1 — HERO (First Screen)

### 1a. Background video — `VideoScrubber`
- **Video URL (exact):**
  `https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/202606021731-e_hqa6sn.mp4`
- `<video>` is `playsInline muted preload="auto"`, `object-cover`, full size, `pointer-events-none`.
- Wrapped in a `style={{ scale: "1.05" }}` div with `will-change-transform`. Container background is `bg-[#FF005E]`.
- **Scrub algorithm:** On every rAF tick:
  - `targetTime = clamp(scrollProgress * duration, 0, duration)` (fallback duration `4.2`).
  - `current += (target - current) * 0.15` (lerp).
  - Seek only when `!video.seeking && Math.abs(video.currentTime - current) > 0.01` → `video.currentTime = current`.
- **GSAP mouse parallax** on the container: on `mousemove`, compute `mx = e.clientX/innerWidth - 0.5`, `my = ...`. Animate `x: -mx*40, y: -my*40, duration: 1.2, ease: "power2.out", overwrite: "auto"`.
- **Loader overlay** (while `!isLoaded`): full-bleed `bg-[#FF005Ef4]`, centered: a 64×64 wrapper with `animate-ping` pink-500/20 ring + a 40×40 spinner ring (`border-4 border-[#ea1f63]/20 border-t-[#ea1f63] animate-spin`), label "LOADING SCROLL STREAM..." in `font-manrope font-semibold text-[12px] uppercase tracking-[0.25em] text-pink-500 drop-shadow-[0_0_8px_rgba(234,31,99,0.4)]`.

### 1b. Hero title — `ScrollExitSplitText`
- Text: `INNER CIRCLE`
- Class on outer container (from App): `text-[10.4vw] leading-none font-michroma font-normal uppercase text-white whitespace-nowrap text-center`, `letterSpacing: -0.07em`.
- Positioned: `absolute bottom-[40px] left-[1%] right-[1%] w-[98%]`, `z-20`, `pointer-events-none`.
- **Split-text mechanic:** split into lines → words → characters. Each char is a `span.char inline-block will-change-transform`. Words separated by a literal `&nbsp;` span.
- **GSAP timeline** (paused, controlled by scroll):
  ```ts
  tl.fromTo(chars,
    { opacity: 1, yPercent: 0, y: 0, scaleY: 1, scaleX: 1, transformOrigin: "50% 0%" },
    { opacity: 0, yPercent: 300, y: "25vh", scaleY: 1.2, scaleX: 0.9, stagger: 0.03, ease: "power2.inOut" }
  );
  ```
- On every `scrollProgress` change: `gsap.to(timeline, { progress: scrollProgress, duration: 0.6, ease: "power1.out", overwrite: "auto" })`. This produces a smooth lag/scrub.

### 1c. Reveal tiles — `SoapTiles`
- Three white pill-cards stacked vertically on the left:
  1. `Private Discord & Networking` — baseXOffset `120`, delay `0ms`
  2. `Weekly Market Alpha Drops` — baseXOffset `180`, delay `100ms`
  3. `Exclusive Web3 Tooling Access` — baseXOffset `240`, delay `200ms`
- Container: `absolute left-4 right-4 md:left-[64px] top-[38%] md:top-1/2 -translate-y-1/2 flex flex-col gap-2 md:gap-[10px] z-40 pointer-events-auto transition-all duration-[800ms] ease-out`. Hidden state: `opacity-0 -translate-x-6 md:-translate-x-12 pointer-events-none`. Visible when `scrollProgress > 0.75`.
- Each tile class: `group relative h-[52px] sm:h-[72px] md:h-[138px] text-black bg-white rounded-xl sm:rounded-2xl md:rounded-[34px] flex items-center justify-center px-4 sm:px-8 md:px-14 w-full md:w-auto md:self-start cursor-pointer origin-left transition-all duration-[400ms] cubic-bezier(0.16, 1, 0.3, 1) whitespace-nowrap`.
- **Entry animation:** `easeProgress = clamp01((scrollProgress - 0.75) / 0.22)`. Each tile:
  - `translateX = (easeProgress - 1) * responsiveOffset` (on mobile, offset is `× 0.25`)
  - `opacity = easeProgress`
  - `filter = blur(${(1 - easeProgress) * 12}px)`
- **Hover behavior (desktop only):** hovered tile scales `1.2`. Non-hovered tiles shift vertically by `±13.8px` (`baseHeight * 0.1`, with `baseHeight` 138/52) — up if above hovered, down if below. On mobile, hover scale stays `1.0`.
- Label inside each tile: `font-michroma font-medium text-[11px] sm:text-[14px] md:text-[23px] leading-[16px] sm:leading-[22px] md:leading-[34px] tracking-tight`, `letter-spacing: -0.03em`.

### 1d. Header — `Header`
- Container: `absolute top-4 left-4 right-4 sm:top-8 sm:left-8 sm:right-8 md:top-[64px] md:left-[64px] md:right-[64px] flex items-center justify-between z-40`.
- **Logo group (left):** clicking navigates to scrollRatio `0`. Contains:
  - `Logo` — 48×48 SVG with this exact path: a stylized "M" mark (`viewBox="0 0 80 80"`, single `<path>`, see code snippet below).
  - Subtitle (hidden < sm): three lines `Full Workflow Automation.` / `We Manage Everything. You` / `Unwind.` in `font-manrope font-normal tracking-wide text-[12px] leading-[16px] text-white`.
- **Desktop nav (≥ md):** the 4 NAVIGATION_ITEMS as buttons: `font-manrope font-medium text-[12px] leading-[16px] tracking-wider text-white px-4 py-2 rounded-full hover:bg-white hover:text-black transition-all duration-300`.
- **Mobile burger:** circular `w-10 h-10 rounded-full border border-white/10 bg-white/5`. Toggles a full-screen overlay `fixed inset-0 bg-[#11010a]/98 backdrop-blur-xl z-30` with each item in `font-michroma text-[16px] uppercase tracking-widest py-4 px-6 border-b border-white/5`; active item: `text-[#FF005E] font-semibold`.

Logo path (exact):
```
M40 80C17.9086 80 0 62.0914 0 40V0C15.0436 0 28.1476 8.30466 34.9776 20.5796C25.6529 22.8063 18.7198 31.1937 18.7198 41.2004V42.0962C18.7198 53.3099 27.8104 62.4004 39.0242 62.4004H39.9199L39.9197 41.2004C39.9197 52.9088 49.4113 62.4004 61.1198 62.4004L61.1198 41.2004C61.1198 29.5187 51.6717 20.0437 40 20.0005L40 0H41.6902C62.8481 0 80 17.1519 80 38.3099V40C80 62.0914 62.0914 80 40 80Z
```

---

### SECTION 2 — SECOND SCREEN (Rising Panel)

### Reveal mechanics
- Triggered when `lerpedScrollProgress > 1.15`. Becomes fully on-screen at `1.65`.
- Panel: `absolute bottom-0 left-0 w-full h-full bg-[#11010a] rounded-t-[48px] overflow-hidden z-40`.
- `transform: translateY((1 - easedRisingProgress) * 100%)` where `easedRisingProgress = 1 - (1 - secondScreenProgress)^3`.
- `visibility: hidden` when `secondScreenProgress === 0`.
- While rising, the **first screen blurs** up to `64px` via `Math.sin(p * π/2) * 64`.
- **iOS grab-handle pill:** `absolute top-5 left-1/2 -translate-x-1/2 w-16 h-[5px] bg-white rounded-full z-50 pointer-events-none`.

### 2a. Background video — `SecondVideoScrubber`
- **Video URL (exact):**
  `https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/2026060218225-v_kcy5rl.mp4`
- Same component skeleton as `VideoScrubber` but with these differences:
  - Background color of the loader bg: `bg-[#11010af4]`.
  - `DRUM_START = 1.45`, `DRUM_END = 3.50`.
  - `drumProgress = clamp01((scrollProgress - 1.45) / (3.50 - 1.45))`, `target = drumProgress * duration`.
  - Same lerp `0.15`, same `!video.seeking && diff > 0.01` guard.
  - Same GSAP mouse parallax `x/y: ±40px, duration 1.2, ease "power2.out"`.
  - Loader label: `LOADING DRUM STREAM...` with `border-pink-500/20 border-t-pink-500`.

### 2b. Cylindrical text drum — `CylindricalTextDrum`
- Container: `absolute inset-y-0 left-0 w-full sm:w-[65%] md:w-[60%] z-30 flex flex-col items-start justify-center pointer-events-none select-none text-left pl-6 sm:pl-12 md:pl-20 py-16`, with `perspective: 1000px; perspectiveOrigin: 25% 50%`.
- Inner: `relative w-full h-[85vh] flex flex-col justify-center items-start overflow-visible` with `transformStyle: "preserve-3d"`.
- **Geometry:** `R = 380`, `lineHeight = 32`.
- `targetIndex = clamp01((scrollProgress - 1.45) / 2.05) * (LINES.length - 1)`.
- For each line `idx`:
  - `indexDiff = idx - targetIndex`
  - `translateY = indexDiff * 32`
  - `angleRad = translateY / 380`, `angleDeg = angleRad * 180/π`
  - `translateZ = cos(angleRad) * 380 - 380`
  - `baseScale = 0.78 + cos(angleRad) * 0.22`
  - `opacity = max(0, (cos(angleRad) - 0.2) / 0.8)`
  - `depthBlur = min(8, max(0, (|indexDiff| - 1.5) * 0.75))`
  - Apply `transform: translateY(${ty}px) translateZ(${tz}px) rotateX(${-angleDeg * 0.8}deg) scale(${baseScale})`, `transformOrigin: "left center"`, plus blur when > 0.1.
- Each line `<p>`: `font-manrope text-[18px] sm:text-[24px] md:text-[28px] lg:text-[32px] font-semibold leading-[0.90] tracking-tight whitespace-nowrap`, `letter-spacing: -0.035em`.
- Text segments: if `highlight === true` use `text-white font-bold opacity-100`; else `text-white/60`. Empty `""` line renders a sized spacer at `opacity * 0.3`.
- **Use the exact `LINES` array below — 32 entries (one empty string at index 15):**

```
1.  Welcome to the [ultimate convergence]
2.  of [digital rebels], [underground creators],
3.  and [top-tier product builders] who
4.  refuse to follow [guidelines].
5.  This is where [high-end design principles]
6.  meet [pure technical execution],
7.  without the [corporate bureaucracy] and
8.  meaningless [standard aesthetics].
9.  We [gather in the shadows] to build
10. the [next generation] of [scalable interfaces],
11. [automated workflows], and [decentralized assets]
12. that move the [cultural needle forward].
13. Experience [zero-bullshit networking],
14. weekly [alpha allocations], and [unreleased]
15. [toolkits] to shape the [internet's landscape].
16. (empty line)
17. This is [not another social club]
18. for casual enthusiasts or [template consumers].
19. This is a [highly selective environment]
20. engineered for [hyper-productive creators],
21. [UI/UX visionaries], and [AI prompt architects]
22. who operate at the [absolute limits]
23. of [digital product creation].
24. Our [framework is simple]:
25. [eliminate intermediate noise],
26. [automate the execution layer],
27. and [deploy elite digital products]
28. while others are still [scheduling meetings].
29. We loop through [complex design systems],
30. [break conventional grids], and
31. [execute fluid interactions] that
32. [redefine digital environments].
```
(Words in `[brackets]` are `highlight: true`.)

### 2c. Logo marquee — `Marquee`
- Position: `absolute bottom-8 sm:bottom-12 md:bottom-16 left-0 w-full sm:w-[65%] md:w-[60%] pl-6 sm:pl-12 md:pl-20 pr-6 sm:pr-12 md:pr-16 z-50`.
- Inside a wrapper `border-t border-white/[0.08] pt-6`.
- `<Marquee gap="80px" speed={25} fade>` produces two duplicated tracks animated infinitely with the keyframe `marquee-scroll` (0% → -50%) over `25s` linear infinite, masked with a left/right transparent fade at 15%/85%.
- Children in order: `<GoogleWordmark size={100} />`, `<GithubWordmark size={100} />`, then `<img>` tags for these exact URLs, each styled `h-6 w-auto object-contain brightness-0 invert opacity-80 hover:opacity-100 transition-opacity`, `referrerPolicy="no-referrer"`:
  - `https://raw.githubusercontent.com/dsMagnatov/Acreage-landing-assets/refs/heads/main/voiceflow-logo-svg-150px.svg`
  - `.../zendesk-logo-svg-150px.svg`
  - `.../pendo-logo-svg-150px.svg`
  - `.../glide-logo-svg-150px.svg`
  - `.../canva-logo-svg-150px.svg`
- `GoogleWordmark` and `GithubWordmark` are inline `<svg>` wordmarks at `viewBox="0 0 115 30"` / `0 0 110 30` — see file for exact text/paths.

---

### INTERACTIONS — Summary table

| Trigger | Effect |
|---|---|
| Mouse wheel | `scrollProgress += deltaY * 0.0006`, clamp `[0, 3.5]` |
| Touch drag | `scrollProgress += (lastY - currentY) * 0.0015` |
| Nav click | 1200ms easeInOutCubic lerp to target ratio |
| `scrollProgress` 0 → 1 | Hero video scrubs forward, "INNER CIRCLE" chars fall (300% y, 25vh) with 0.03 stagger |
| `scrollProgress` > 0.75 | Soap tiles fade/slide in (over a 0.22 range), with 12px → 0 blur |
| Hover tile (desktop) | Hovered tile scales 1.2, neighbors shift ±13.8px |
| `scrollProgress` > 1.15 | Second screen rises (ease-out cubic), first screen blurs to 64px |
| `scrollProgress` 1.45 → 3.50 | Second video scrubs; cylindrical drum rotates; line at center is at scale 1.0/opacity 1.0 |
| Mouse move (anywhere) | Both videos parallax-translate ±40px via GSAP `power2.out`, 1.2s |

---

### DATA PERSISTENCE

Supabase is available. This page is presentational and does not persist user state, so no database tables are required for the recreation. If extending with capture forms, waitlist sign-ups, or analytics events, create a Supabase table with RLS enabled and `auth.uid()`-based policies (one INSERT policy for `authenticated`, restrictive SELECT).

---

### FILE STRUCTURE

```
src/
  App.tsx
  main.tsx
  index.css
  types.ts
  data.ts
  components/
    Header.tsx
    Logo.tsx
    Logos.tsx                 (GoogleWordmark, GithubWordmark exported)
    Marquee.tsx
    VideoScrubber.tsx
    SecondVideoScrubber.tsx
    ScrollExitSplitText.tsx
    SoapTiles.tsx
    CylindricalTextDrum.tsx
```

## New Era Automotive Hero — Automotive [sites/8]

- Preview: https://motionsites.ai/assets/hero-new-era-auto-preview-W56vp0xD.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/8.gif

Build a full-screen automotive hero section for a car dealership/marketplace website. Use Google Fonts: Inter (400, 500, 600) and Bebas Neue.

Background:

Full-viewport-height section (min 600px, max 965px) with a dark (#010101) fallback background.

Looping, muted, autoplaying background video covering the entire section using object-cover. Use this video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260213_051817_c7d8ccc6-bfaa-417c-8474-e5cefeea26b4.mp4

Add a subtle top gradient overlay (260px tall, from black/30 to transparent) and a matching bottom gradient overlay (260px tall, from black/30 to transparent) for text readability.

Large decorative text:

Centered horizontally, positioned about 15% from the top. Display the words "NEW ERA" as very large, bold, all-caps decorative typography spanning about 75% of the width (max 1073px).

Fill the text with a vertical linear gradient: white at 83% opacity at the top, fading to white at 12% opacity at the bottom. This text should be behind the content but above the video.

Top navbar (pinned to top, full width, horizontal padding 80px on desktop):

Left: A small abstract pinwheel/spinner logo icon (28x28, white) next to the brand name "Logoipsum" in white, Inter font, ~24px. Hide the brand name on small screens.

Center: Navigation links — "Home", "Shop", "Blog", "About Us", "Contact Us" — in Inter, light gray (#EEEFF2), with -0.32px letter-spacing. Hidden on screens below lg breakpoint.

Right: A "Sign In" text link in white (#FBFBFD), and a white rounded (8px) "Cart" button (48px tall) with a small shopping cart icon (18x18, dark #272835) and "Cart" label in Inter medium, dark text (#272835). The button has a subtle box-shadow. Hide "Sign In" on small screens.

Bottom CTA area (pinned to bottom of the section, same horizontal padding):

Left side: A paragraph in Inter, white, ~20px/30px line-height, max-width 414px: "Choose from thousands of certified cars you can trust, transparently priced, because buying a car should feel exciting." Next to it, a white rounded (8px) "Shop Now" button (48px tall) with an arrow-right icon (18x18, dark), Inter medium text, dark text (#272835), with a light border (#EEEFF2) and subtle shadow. On small screens, stack the paragraph and button vertically.

Right side: A large tagline in Bebas Neue, white, 64px on desktop (48px–60px on smaller screens), line-height 1, max-width 466px: "Find the perfect car that fits our journey".

On large screens, the left and right sides sit in a single row aligned to the bottom. On smaller screens they stack vertically.

Make the entire section fully responsive. Use Tailwind CSS and React.

## Luxury Focus — E-commerce [sites/luxury-focus]

- Preview: https://stream.mux.com/MSctoqC17nNR00ZSv5l7fTur9QOwcJHC01uL02uOFs1Vvs.m3u8
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luxury-focus.m3u8

Recreate "SCULPTED BY TIME" Luxury Jewelry Scroll Experience

Build a single-page React + TypeScript + Vite landing page for a luxury jewelry brand. It is a **scroll-driven cinematic experience**: the page is very tall, and scrolling scrubs through two background videos while editorial text fades/blurs away and product panels slide in. Use **Tailwind CSS v4** (via `@tailwindcss/vite`) and `lucide-react` for icons. Everything lives in a single `src/App.tsx` component. No backend / no data persistence is needed.

### Stack & setup
- Dependencies: `react@19`, `react-dom@19`, `vite@6`, `@vitejs/plugin-react`, `@tailwindcss/vite`, `tailwindcss@4`, `lucide-react`, `motion`.
- `src/main.tsx` renders `<App />` into `#root`.
- `index.html` has `<div id="root"></div>` and loads `/src/main.tsx`.

### Fonts (src/index.css)
```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital,wght@0,400;1,400&family=Manrope:wght@200;300;400;500;600;700;800&display=swap');
@import "tailwindcss";

@theme {
  --font-serif: "Instrument Serif", serif;
  --font-sans: "Manrope", sans-serif;
}
```
- Serif display font: **Instrument Serif** (`font-serif`).
- Sans UI font: **Manrope** (`font-sans`).

### Asset URLs (use exactly)
- Background video 1 (plays during first 40% of scroll):
  `https://res.cloudinary.com/dbfd996z4/video/upload/q_auto/f_auto/v1781009724/11111111_gvewuj.mp4`
- Background video 2 (plays during second half):
  `https://res.cloudinary.com/dbfd996z4/video/upload/q_auto/f_auto/v1781009724/2222222_x4qpet.mp4`
- Hero product PNG (ring): `https://res.cloudinary.com/dbfd996z4/image/upload/q_auto/f_auto/v1781009782/rng_awymkj.png`
- Panel product 1 (earrings): `https://res.cloudinary.com/dbfd996z4/image/upload/q_auto/f_auto/v1781017114/202606091756_msbh8b.jpg`
- Panel product 2 (ring): `https://res.cloudinary.com/dbfd996z4/image/upload/q_auto/f_auto/v1781019866/2606091843_kfonxp.jpg`

(Note: there is no CloudFront URL in this project — all media is served from Cloudinary at the `dbfd996z4` account. The `/public/main` and `/public/second` JPG frame sequences exist as fallbacks but the live page scrubs the two Cloudinary MP4s, not the JPGs.)

### Color & theme
- Page background: `#020202` (near-black), text white.
- Signature accent (titles, progress bar, hover states, cart dot): `#FBFF8D` (luminous pale chartreuse-yellow).
- Slide-in product panel background: `#FAF9F5` (warm off-white), text `#121212`; muted greys `#8E8B84`, `#73716C`, `#E5E5E2`.
- NO purple/violet anywhere.

### Layout structure (single root `div`, `h-[650vh]`, `bg-[#020202]`, `overflow-x-hidden`, `antialiased`, `font-sans`)

### 1. Fixed background video stage (`fixed inset-0 z-0`, pointer-events-none)
- Two `<video>` elements stacked absolutely, `object-cover`, `muted playsInline preload="auto"`.
- Video 1 opacity `0.85` when `progress < 0.48`, else `0`; Video 2 opacity `0.85` when `progress >= 0.48`, else `0`. Cross-fade with `transition-opacity duration-700 ease-in-out`.
- A hidden `<canvas>` (kept in DOM, `display:none`).
- Three gradient overlays for typography contrast: top (`h-44`, black 85%→transparent), bottom (`h-56`, black 90%→transparent), left (`w-1/2`, black 40%→transparent, only `lg:block`).

### 2. Top progress bar (`fixed top-0 left-0 h-[2.5px] bg-[#FBFF8D] z-50`)
- Width = `${progress * 100}%`, `transition-all duration-75`, pointer-events-none.

### 3. Editorial overlay (`fixed inset-0 z-10`, flex column space-between, padding `p-6 md:p-10 lg:p-12`)
- `pointerEvents: 'none'` when `progress > 0.45`; `visibility: hidden` when `progress >= 0.45`.
- **Header** (`flex justify-between items-start`):
  - Left column (`flex flex-col gap-10`): a 48×48 white brand crest **SVG** (5-petal abstract floral logo — provide the multi-path SVG), and 40px below it a sub-header: `<h3>` "Contemporary Luxury For The / Discerning Minimalist" (semibold, 12-13px, tracking -0.03em) + a `<p>` (11px, white/50, max-width 260px): "Exclusive creations tailored for true aesthetes. We forge more than simple ornaments; we build tactile artifacts of your personal legacy."
  - Right nav (`flex items-center gap-5..8`, 13px white/70): links **Collections, Atelier, Our story, Contact**, each with animated underline-on-hover (`after:` pseudo growing from w-0 to w-full) and `hover:scale-105`. Then a vertical separator, a **User** icon link, and a **ShoppingBag** button (with a pulsing `#FBFF8D` 6.5px dot badge). The bag button smooth-scrolls to the very bottom of the page.
- **Hero main** (`grid grid-cols-1 lg:grid-cols-12`, items-end):
  - Left (cols 1-7): giant serif headline `#FBFF8D`, uppercase, `leading-[0.88]`, fluid `fontSize: clamp(2.5rem, 5.6vw, 7.5rem)`, three lines each as own div: "&nbsp;&nbsp;&nbsp;&nbsp;*SCULPTED*" (italic), "BY TIME. WORN", "BY *YOU*" (italic). Below it two manifesto `<p>` blocks (11px white/50, width 260px) — text: "We craft modern jewelry that speaks volumes through silence..." and "Every piece acts as a personal manifesto...".
  - Right (cols 8-12, max-w-240px): product caption block — small label "Abyssal Silver Ring", `<h2>` "18K White Gold & Rough Onyx", a 10.5px description; then a transparent product card (`aspect-[4/5]`, `hover:scale-[1.04]`) holding the ring PNG (`object-contain`, `group-hover:scale-105`); then a centered serif price "$1,850" that turns `#FBFF8D` on group hover.

### 4. Slide-in product sheet (`fixed top-0 left-0 h-full w-full sm:w-[600px] lg:w-[648px] bg-[#FAF9F5] text-[#121212] z-30`)
- Transform: `translate-x-0` when open else `-translate-x-full`, `transition-transform duration-[1100ms] ease-[cubic-bezier(0.16,1,0.3,1)]`, shadow `12px 0 45px rgba(0,0,0,0.22)`, `overflow-y-auto`.
- Open when `progress >= 0.40 && progress < 0.50` (first card) OR `progress >= 0.90` (second card).
- Top-left **BACK TO SERIES** button (ArrowLeft icon, uppercase, tracking 0.16em) that smooth-scrolls back: to 82% if `progress >= 0.90`, else to 35%.
- Centered product header: category label (uppercase, tracking 0.15em, `#8E8B84`), serif `<h2>` title, 12px description `#73716C`; then a centered product image (`max-w-[460px]`).
- Footer transaction row (border-top, full-bleed via negative margins): a black **ADD TO ATELIER BAG** button (`h-[50px]`, uppercase tracking 0.15em), a `#E5E5E2` quantity stepper (− value +), and a large serif dynamic price `${price * quantity}`.
- **Dynamic product data** driven by `isSecondCard` (`progress >= 0.90`):
  - Second card: category "Atelier Core Edition", title "Sterling Silver Sculpture Ring", price 1350, image = ring JPG, with the sculpture description.
  - Otherwise: category "Aura Fine Earrings", title "18K White Gold & Pink Sapphire", price 1850, image = earrings JPG, with the drop-stud description.
  - Reset `quantity` to 1 whenever the active product title changes.

### Scroll & animation engine (the core)
Use refs (not state) for animation values to avoid re-renders; only `progress` and `quantity` are React state.

- **State refs:** `targetScrollFractionRef`, `currentScrollFractionRef`, `targetVideoRatioRef`, `currentVideoTimeRef`, `targetVideoSecondRatioRef`, `currentVideoSecondTimeRef`, `targetFrameRef`, `currentFrameRef`. `totalFrames = 480`.
- **Scroll listener:** compute `scrollTop / (scrollHeight - clientHeight)`, clamp 0–1, store in `targetScrollFractionRef`. Passive listener; also run once on mount and once after a 500ms timeout.
- **requestAnimationFrame loop** (`smoothUpdate`):
  1. Lerp `currentScrollFraction` toward target by factor **0.05** (snap when very close) — gives a weighted, cinematic, "catch-up" feel. `setProgress(currentScrollFraction)`.
  2. Map `activeProgress` to phases:
     - `<= 0.40`: ratio = p/0.40 → video1 ratio = ratio, video2 = 0, frame = 1 + ratio*239.
     - `0.40–0.50`: video1 = 1.0, frame = 240 (hold — first panel open).
     - `0.50–0.90`: video1 = 1.0, video2 ratio = (p-0.50)/0.40, frame = 241 + ratio*239.
     - `> 0.90`: both = 1.0, frame = 480 (second panel open).
  3. Scrub each video by lerping `currentVideoTime` toward `targetRatio * video.duration` by factor **0.08**, and set `video.currentTime` — **but only when `!video.seeking`** (skip issuing a new seek while the browser is still painting the previous frame, to prevent stutter). Same guard for the second video.
  4. Lerp `currentFrameRef` toward `targetFrameRef` by 0.08, clamped to [1, 480].
  5. Re-request the frame; cancel on cleanup.

### Stagger helper (`getStaggerStyle(start, end)`)
Maps `progress` within a [start,end] window to a fade-out: `opacity = 1 - ratio`, `translateY = -75 * ratio`, `filter: blur(${ratio*16}px)`, `willChange`, and a `cubic-bezier(0.16,1,0.3,1)` 0.35s transition; `pointerEvents: 'none'` once opacity < 0.15. Apply staggered windows to each editorial element so they melt upward and blur away as you scroll (logo 0.00–0.12, subheader 0.06–0.18, nav 0.03–0.15, title lines 0.09/0.12/0.15 → +0.11, manifesto 0.18/0.21, right caption 0.14–0.25, card 0.18–0.29, price 0.20–0.31).

### Embedded `<style>` extras
- `@keyframes scrollLine`.
- Custom dark webkit scrollbar (6px, track `#020202`, thumb white/15, hover `#FBFF8D`/40).
- Media query `(max-height:780px) and (min-width:1024px)`: tighten `#sculpted-title` margin and `#product-image-card` padding.

### Behavior summary
Scrolling 0→100% of the 650vh page: hero text melts away → video 1 scrubs forward → first earrings panel slides in (~40-50%) → video 2 scrubs → second ring panel slides in (~90%+). The ShoppingBag button jumps to the final panel; the panel BACK button scrolls back into the video sequence. All motion is buttery via double-lerp (scroll smoothing 0.05 + media smoothing 0.08) and the `!seeking` seek guard.

## Jewelry Store — Ecommerce [sites/jewelry-store]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/CleanShot%202026-07-11%20at%2016.38.46.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/jewelry-store.png

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Blue Nile</title>
  <link href="https://db.onlinewebfonts.com/c/7973d1644865c7217230fea96daae6fe?family=Test+Founders+Grotesk+Light" rel="stylesheet">
  <link href="https://db.onlinewebfonts.com/c/12487acadbf8efa35235fe8d339411ec?family=NimbusSanExt" rel="stylesheet">
  <style>
    *, *::before, *::after {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: 'Test Founders Grotesk Light', sans-serif;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
      overflow: hidden;
    }

    /* ===== ANIMATIONS ===== */
    @keyframes fade-in-up {
      from { opacity: 0; transform: translateY(24px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @keyframes fade-in-down {
      from { opacity: 0; transform: translateY(-24px); }
      to { opacity: 1; transform: translateY(0); }
    }

    @keyframes fade-in-scale {
      from { opacity: 0; transform: translate(-50%, 0) scale(0.95); }
      to { opacity: 1; transform: translate(-50%, 0) scale(1); }
    }

    @keyframes fade-in {
      from { opacity: 0; }
      to { opacity: 1; }
    }

    @keyframes line-grow {
      from { transform: scaleY(0); }
      to { transform: scaleY(1); }
    }

    .animate-fade-in-up {
      animation: fade-in-up 0.8s cubic-bezier(0.16, 1, 0.3, 1) both;
    }

    .animate-fade-in-down {
      animation: fade-in-down 0.8s cubic-bezier(0.16, 1, 0.3, 1) both;
    }

    .animate-fade-in-scale {
      animation: fade-in-scale 1s cubic-bezier(0.16, 1, 0.3, 1) both;
    }

    .animate-fade-in {
      animation: fade-in 1s cubic-bezier(0.16, 1, 0.3, 1) both;
    }

    .animate-line-grow {
      transform-origin: top;
      animation: line-grow 1.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    }

    /* ===== LAYOUT ===== */
    .page-wrapper {
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }

    .hero-section {
      position: relative;
      flex: 1;
      width: 100%;
      background-color: #E96B00;
      overflow: hidden;
    }

    /* ===== GRADIENT GLOW ===== */
    .gradient-glow {
      position: absolute;
      left: -96px;
      top: 33%;
      width: 1360px;
      height: 1360px;
      border-radius: 50%;
      background: rgba(246, 187, 126, 0.4);
      filter: blur(450px);
    }

    /* ===== GRID LINES ===== */
    .grid-lines {
      position: absolute;
      inset: 0;
      z-index: 1;
      pointer-events: none;
    }

    .grid-line {
      position: absolute;
      top: 0;
      width: 1px;
      height: 100%;
      background: rgba(255, 255, 255, 0.16);
    }

    /* ===== NAVIGATION ===== */
    .nav {
      position: relative;
      z-index: 50;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 28px 24px 0;
    }

    .nav-links {
      display: flex;
      align-items: center;
      gap: 16px;
    }

    .nav-links a {
      color: white;
      text-decoration: none;
      font-size: 20px;
      line-height: 1.25;
      transition: opacity 0.2s;
    }

    .nav-links a:hover {
      opacity: 0.8;
    }

    .cart-badge {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 20px;
      height: 20px;
      background: black;
      border-radius: 50%;
      color: white;
      font-size: 13px;
      line-height: 1;
    }

    .cart-link {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    /* ===== LARGE HEADING ===== */
    .hero-heading-wrapper {
      position: absolute;
      top: 5%;
      left: 0;
      right: 0;
      z-index: 10;
      text-align: center;
    }

    .hero-heading {
      font-family: 'NimbusSanExt', sans-serif;
      font-weight: bold;
      color: white;
      font-size: clamp(12rem, 24vw, 30rem);
      line-height: 0.85;
      letter-spacing: -0.10em;
      white-space: nowrap;
    }

    /* ===== CENTER HERO IMAGE ===== */
    .hero-image-wrapper {
      position: absolute;
      top: 8%;
      left: 50%;
      transform: translateX(-50%);
      z-index: 20;
      width: 54%;
      max-width: 760px;
    }

    .hero-image-wrapper img {
      width: 100%;
      height: auto;
      object-fit: cover;
      position: relative;
      z-index: 10;
    }

    /* ===== LEFT COLUMN ===== */
    .left-column {
      position: absolute;
      left: 0;
      top: 42%;
      bottom: 0;
      z-index: 30;
      width: calc(25.4% + 86px);
      max-width: 451px;
      display: flex;
      flex-direction: column;
    }

    .exclusive-card {
      position: relative;
      background: white;
      aspect-ratio: 280 / 160;
      overflow: hidden;
      flex-shrink: 0;
      width: calc(100% - 86px);
    }

    .exclusive-card .label {
      position: absolute;
      top: 12px;
      left: 12px;
      color: black;
      font-size: 18px;
      font-weight: 500;
      z-index: 10;
    }

    .exclusive-card img {
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      object-fit: cover;
    }

    .left-middle-row {
      display: flex;
      flex: 1;
    }

    .left-middle-text {
      flex: 1;
      padding: 16px 12px;
    }

    .left-middle-text p {
      color: white;
      font-size: 20px;
      line-height: 1.625;
    }

    .arrow-button {
      width: 86px;
      height: 84px;
      background: black;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      align-self: flex-start;
      cursor: pointer;
      transition: background-color 0.2s;
    }

    .arrow-button:hover {
      background: rgba(0, 0, 0, 0.9);
    }

    .arrow-button svg {
      width: 20px;
      height: 20px;
      color: white;
    }

    .explore-button {
      width: calc(100% - 86px);
      height: 88px;
      background: black;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      flex-shrink: 0;
      transition: background-color 0.2s;
    }

    .explore-button:hover {
      background: rgba(0, 0, 0, 0.9);
    }

    .explore-button span {
      color: white;
      font-size: 20px;
      font-weight: 500;
    }

    /* ===== AWARDS SECTION ===== */
    .awards-section {
      display: flex;
      position: absolute;
      right: 24px;
      top: 42%;
      z-index: 30;
      width: calc(25.4% - 24px);
      max-width: 341px;
      align-items: center;
      justify-content: space-between;
    }

    .awards-number {
      display: flex;
      align-items: center;
      gap: 4px;
    }

    .awards-bracket {
      color: white;
      font-size: 48px;
      font-weight: 300;
    }

    .awards-value-wrapper {
      display: flex;
      align-items: flex-start;
    }

    .awards-value {
      color: white;
      font-size: 48px;
      font-weight: 700;
    }

    .awards-plus {
      color: white;
      font-size: 20px;
      font-weight: 700;
      position: relative;
      top: -4px;
    }

    .awards-text {
      text-align: right;
    }

    .awards-text p {
      color: white;
      font-size: 20px;
      text-transform: uppercase;
      line-height: 1.375;
      letter-spacing: 0.025em;
    }

    /* ===== SINCE 2017 ===== */
    .since-2017 {
      position: absolute;
      right: 24px;
      top: 60%;
      z-index: 30;
    }

    .since-2017 span {
      color: white;
      font-size: 20px;
    }

    /* ===== PRODUCT CARD (DESKTOP) ===== */
    .product-card-desktop {
      position: absolute;
      right: 0;
      bottom: 0;
      z-index: 30;
      width: 25.4%;
      max-width: 365px;
    }

    .product-card {
      background: white;
      padding: 12px;
      height: 276px;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      position: relative;
    }

    .product-card h3 {
      color: black;
      font-size: 20px;
      font-weight: 500;
      line-height: 1.25;
    }

    .product-card .subtitle {
      color: rgba(0, 0, 0, 0.6);
      font-size: 14px;
      margin-top: 4px;
    }

    .product-card .center-image {
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      width: 75%;
      height: auto;
      object-fit: contain;
    }

    .product-card .price-label {
      color: rgba(11, 33, 34, 0.64);
      font-size: 14px;
    }

    .product-card .price {
      color: #0B2122;
      font-size: 20px;
      font-weight: 500;
    }

    .product-arrow {
      position: absolute;
      bottom: 0;
      right: 0;
      width: 92px;
      height: 84px;
      background: black;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      transition: background-color 0.2s;
    }

    .product-arrow:hover {
      background: rgba(0, 0, 0, 0.9);
    }

    .product-arrow svg {
      width: 20px;
      height: 20px;
      color: white;
    }

    /* ===== MOBILE STYLES ===== */
    .mobile-only { display: none; }
    .mobile-logo { display: none; }
    .hamburger { display: none; }
    .mobile-menu { display: none; }
    .mobile-product-card { display: none; }
    .mobile-grid-line { display: none; }
    .desktop-grid-line { display: block; }

    @media (max-width: 1023px) {
      .page-wrapper {
        height: 100vh;
      }

      .mobile-only { display: block; }
      .desktop-only { display: none; }
      .mobile-grid-line { display: block; }
      .desktop-grid-line { display: none; }

      .nav {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        padding: 20px 16px 0;
      }

      .nav-links { display: none; }

      .mobile-logo {
        display: block;
        font-family: 'NimbusSanExt', sans-serif;
        color: white;
        font-size: 24px;
        font-weight: bold;
        line-height: 1;
      }

      .hamburger {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 40px;
        height: 40px;
        background: none;
        border: none;
        color: white;
        cursor: pointer;
        position: relative;
        z-index: 50;
      }

      .hamburger svg {
        width: 24px;
        height: 24px;
      }

      .hero-heading-wrapper { display: none; }

      .hero-image-wrapper {
        top: auto;
        bottom: -40px;
        width: 132%;
        max-width: none;
      }

      .left-column { display: none; }

      .awards-section {
        left: 16px;
        right: 16px;
        top: auto;
        bottom: 16px;
        width: auto;
        max-width: none;
      }

      .awards-bracket { font-size: 72px; }
      .awards-value { font-size: 72px; }
      .awards-plus { font-size: 24px; }

      .since-2017 { display: none; }
      .product-card-desktop { display: none; }

      .mobile-product-card {
        display: block;
        position: relative;
        width: 100%;
        flex-shrink: 0;
      }

      .mobile-product-card .product-card {
        height: 260px;
      }
    }

    /* ===== SVG ICON ===== */
    .icon-arrow-up-right {
      stroke: currentColor;
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
      fill: none;
    }
  </style>
</head>
<body>
  <div class="page-wrapper">
    <!-- MAIN HERO SECTION -->
    <section class="hero-section">

      <!-- Gradient Glow -->
      <div class="gradient-glow"></div>

      <!-- Grid Lines -->
      <div class="grid-lines">
        <!-- Mobile lines -->
        <div class="grid-line mobile-grid-line animate-line-grow" style="left: 33.333%; animation-delay: 0.2s;"></div>
        <div class="grid-line mobile-grid-line animate-line-grow" style="left: 66.666%; animation-delay: 0.35s;"></div>
        <!-- Desktop lines -->
        <div class="grid-line desktop-grid-line animate-line-grow" style="left: 25%; animation-delay: 0.2s;"></div>
        <div class="grid-line desktop-grid-line animate-line-grow" style="left: 50%; animation-delay: 0.35s;"></div>
        <div class="grid-line desktop-grid-line animate-line-grow" style="left: 75%; animation-delay: 0.5s;"></div>
      </div>

      <!-- Navigation -->
      <nav class="nav animate-fade-in-down" style="animation-delay: 0.1s;">
        <!-- Desktop left links -->
        <div class="nav-links desktop-only">
          <a href="#">Search</a>
          <a href="#">Catalog</a>
          <a href="#">About</a>
        </div>

        <!-- Desktop right links -->
        <div class="nav-links desktop-only">
          <a href="#">Profile</a>
          <a href="#">Favorites</a>
          <a href="#" class="cart-link">
            Cart
            <span class="cart-badge">2</span>
          </a>
        </div>

        <!-- Mobile logo -->
        <div class="mobile-logo">Blue<br>Nile</div>

        <!-- Mobile hamburger -->
        <button class="hamburger" aria-label="Toggle menu" onclick="toggleMenu()">
          <svg id="menu-icon" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="4" x2="20" y1="12" y2="12"></line>
            <line x1="4" x2="20" y1="6" y2="6"></line>
            <line x1="4" x2="20" y1="18" y2="18"></line>
          </svg>
          <svg id="close-icon" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:none; position:absolute;">
            <path d="M18 6 6 18"></path>
            <path d="m6 6 12 12"></path>
          </svg>
        </button>
      </nav>

      <!-- Mobile Menu Overlay -->
      <div id="mobile-menu" class="mobile-menu" style="position:fixed; inset:0; z-index:40; visibility:hidden; transition: all 0.5s cubic-bezier(0.77,0,0.18,1);">
        <div id="menu-backdrop" style="position:absolute; inset:0; background:rgba(0,0,0,0.6); backdrop-filter:blur(4px); opacity:0; transition: opacity 0.5s cubic-bezier(0.77,0,0.18,1);" onclick="toggleMenu()"></div>
        <div id="menu-panel" style="position:absolute; top:0; left:0; height:100%; width:80%; max-width:320px; background:#E96B00; box-shadow:0 25px 50px -12px rgba(0,0,0,0.25); transform:translateX(-100%); transition: transform 0.5s cubic-bezier(0.77,0,0.18,1);">
          <div style="display:flex; flex-direction:column; gap:4px; padding:96px 24px 0;">
            <a href="#" class="menu-item" style="color:white; text-decoration:none; font-size:30px; font-weight:500; padding:12px 0; border-bottom:1px solid rgba(255,255,255,0.1); transition: all 0.3s; opacity:0; transform:translateX(-20px);">Search</a>
            <a href="#" class="menu-item" style="color:white; text-decoration:none; font-size:30px; font-weight:500; padding:12px 0; border-bottom:1px solid rgba(255,255,255,0.1); transition: all 0.3s; opacity:0; transform:translateX(-20px);">Catalog</a>
            <a href="#" class="menu-item" style="color:white; text-decoration:none; font-size:30px; font-weight:500; padding:12px 0; border-bottom:1px solid rgba(255,255,255,0.1); transition: all 0.3s; opacity:0; transform:translateX(-20px);">About</a>
            <a href="#" class="menu-item" style="color:white; text-decoration:none; font-size:30px; font-weight:500; padding:12px 0; border-bottom:1px solid rgba(255,255,255,0.1); transition: all 0.3s; opacity:0; transform:translateX(-20px);">Profile</a>
            <a href="#" class="menu-item" style="color:white; text-decoration:none; font-size:30px; font-weight:500; padding:12px 0; border-bottom:1px solid rgba(255,255,255,0.1); transition: all 0.3s; opacity:0; transform:translateX(-20px);">Favorites</a>
          </div>
        </div>
      </div>

      <!-- Large "Blue Nile" Heading (desktop) -->
      <div class="hero-heading-wrapper desktop-only animate-fade-in" style="animation-delay: 0.3s;">
        <h1 class="hero-heading">Blue Nile</h1>
      </div>

      <!-- Center Hero Image -->
      <div class="hero-image-wrapper animate-fade-in-scale" style="animation-delay: 0.5s;">
        <img src="https://soft-zoom-63098134.figma.site/_assets/v11/9028130a3e77802079d3a2e663b85ee12d365b61.png" alt="Model showcasing jewelry" />
      </div>

      <!-- Left Column (desktop) -->
      <div class="left-column desktop-only animate-fade-in-up" style="animation-delay: 0.7s;">
        <!-- Exclusive Card -->
        <div class="exclusive-card">
          <span class="label">Exclusive</span>
          <img src="https://soft-zoom-63098134.figma.site/_assets/v11/d499425c000b01d01831365b4b9df4feb1bead8a.png" alt="Exclusive jewelry piece" />
        </div>
        <!-- Middle Row -->
        <div class="left-middle-row">
          <div class="left-middle-text">
            <p>Each design reflects the dialogue between craftsmanship and feeling</p>
          </div>
          <div class="arrow-button">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M7 7h10v10"></path>
              <path d="M7 17 17 7"></path>
            </svg>
          </div>
        </div>
        <!-- Explore Button -->
        <div class="explore-button">
          <span>Explore Collection</span>
        </div>
      </div>

      <!-- Awards Section -->
      <div class="awards-section animate-fade-in-up" style="animation-delay: 0.9s;">
        <div class="awards-number">
          <span class="awards-bracket">[</span>
          <div class="awards-value-wrapper">
            <span class="awards-value">12</span>
            <span class="awards-plus">+</span>
          </div>
          <span class="awards-bracket">]</span>
        </div>
        <div class="awards-text">
          <p>Awards<br>Celebrate<br>Innovation</p>
        </div>
      </div>

      <!-- Since 2017 (desktop) -->
      <div class="since-2017 desktop-only animate-fade-in" style="animation-delay: 1.1s;">
        <span>[ Since 2017 ]</span>
      </div>

      <!-- Product Card Desktop -->
      <div class="product-card-desktop desktop-only animate-fade-in-up" style="animation-delay: 1.0s;">
        <div class="product-card">
          <div>
            <h3>Coco Crush ring</h3>
            <p class="subtitle">18K yellow</p>
          </div>
          <img class="center-image" src="https://soft-zoom-63098134.figma.site/_assets/v11/6297b1b8b8a1c0720cbd098274da6619ad35b486.png" alt="Coco Crush ring" />
          <div>
            <p class="price-label">From</p>
            <p class="price">$25,550</p>
          </div>
        </div>
        <div class="product-arrow">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 7h10v10"></path>
            <path d="M7 17 17 7"></path>
          </svg>
        </div>
      </div>

    </section>

    <!-- Mobile Product Card (below hero section) -->
    <div class="mobile-product-card animate-fade-in-up" style="animation-delay: 0.8s;">
      <div class="product-card">
        <div>
          <h3>Coco Crush ring</h3>
          <p class="subtitle">18K yellow</p>
        </div>
        <img class="center-image" src="https://soft-zoom-63098134.figma.site/_assets/v11/6297b1b8b8a1c0720cbd098274da6619ad35b486.png" alt="Coco Crush ring" />
        <div>
          <p class="price-label">From</p>
          <p class="price">$25,550</p>
        </div>
      </div>
      <div class="product-arrow">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M7 7h10v10"></path>
          <path d="M7 17 17 7"></path>
        </svg>
      </div>
    </div>
  </div>

  <script>
    let menuOpen = false;

    function toggleMenu() {
      menuOpen = !menuOpen;
      const overlay = document.getElementById('mobile-menu');
      const backdrop = document.getElementById('menu-backdrop');
      const panel = document.getElementById('menu-panel');
      const menuIcon = document.getElementById('menu-icon');
      const closeIcon = document.getElementById('close-icon');
      const items = document.querySelectorAll('.menu-item');

      if (menuOpen) {
        overlay.style.visibility = 'visible';
        backdrop.style.opacity = '1';
        panel.style.transform = 'translateX(0)';
        menuIcon.style.display = 'none';
        closeIcon.style.display = 'block';
        items.forEach((item, i) => {
          setTimeout(() => {
            item.style.opacity = '1';
            item.style.transform = 'translateX(0)';
          }, 80 + i * 50);
        });
      } else {
        backdrop.style.opacity = '0';
        panel.style.transform = 'translateX(-100%)';
        menuIcon.style.display = 'block';
        closeIcon.style.display = 'none';
        items.forEach((item) => {
          item.style.opacity = '0';
          item.style.transform = 'translateX(-20px)';
        });
        setTimeout(() => {
          overlay.style.visibility = 'hidden';
        }, 500);
      }
    }
  </script>
</body>
</html>

## OYLA — Ecommerce [sites/oyla]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/%20bewpostArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/oyla.mp4

**Build a luxury handcrafted jewelry brand landing page for "OYLA" — a single-page app using Vite + Express with an `index.html` entry point served through Vite's SPA middleware in development and Express static serving in production. The page features scroll-driven video scrubbing, horizontal product carousel with video reveal, a two-column stats section, and a fixed footer reveal. Use GSAP + ScrollTrigger for all scroll-driven animations.**

---

### BUILD & SERVING ARCHITECTURE

- **Dev:** Express server (`tsx server.ts`) on port 3000 with Vite mounted as middleware (`createViteServer({ server: { middlewareMode: true }, appType: "spa" })`)
- **Build:** `vite build` compiles the frontend into `dist/`, then `esbuild` bundles `server.ts` into `dist/server.cjs` (Node CJS, external packages)
- **Production:** Express serves the `dist/` folder as static, with a catch-all route serving `index.html`
- The main page content lives in `index.html` at the project root (Vite entry point) with inline `<style>` and `<script>` tags
- Express provides an API proxy endpoint at `GET /api/higgsfield-video` that extracts direct MP4 URLs from Higgsfield share pages

---

### BRAND & DESIGN SYSTEM

- **Brand name:** OYLA (handcrafted rings studio, based in Berlin, est. 2019)
- **Brand color:** `#A3111E` (deep crimson red) used for header text, logo, and navigation only
- **Background:** Pure white `#ffffff`
- **Text:** Pure black `#000000` for all body and heading text
- **Dividers:** `#000000` solid 1px lines
- **Color palette:** Strictly black and white with the single crimson accent for the header/nav only

### FONTS (Google Fonts)

Load these exact fonts:
- **Instrument Serif** (weights: 400, italic 400) — used for all large display headings (hero title, stat numbers, left column heading)
- **Inter Tight** (weights: 300–700, italic 300–700) — used for body text, buttons, navigation, footer, and all UI elements
- **Cormorant Garamond** (weights: 300–700, italic) — loaded but not actively used (available for future use)

CSS font stacks:
- Headings: `'Instrument Serif', Georgia, serif`
- Body/UI: `'Inter Tight', 'Inter', Arial, sans-serif`

### CSS VARIABLES

```css
:root {
  --color-bg: #ffffff;
  --color-primary-text: #000000;
  --color-demoted-text: #000000;
  --color-divider: #000000;
  --font-stack: 'Inter Tight', 'Inter', Arial, sans-serif;
}
```

---

### STRUCTURE (TOP TO BOTTOM)

---

### 1. FIXED HEADER

- **Position:** Fixed, `top: 32px`, `left: 32px`, `right: 32px`, `height: 30px`, `z-index: 100`
- **Layout:** Flexbox row, space-between, center aligned
- **Left:** OYLA logo as inline SVG (112x60px viewBox), all paths filled `#A3111E`. The logo contains the letters "OYLA" in a custom serif typeface with a distinctive "O" made of two concentric ellipses
- **Right:** Navigation group containing:
  - "ABOUT" link — `font-size: 15px`, `font-weight: 500`, uppercase, `color: #A3111E`, `gap: 194px` from the cart/menu group
  - Cart/Menu group (`gap: 50px`):
    - Hamburger button: 2 horizontal lines, each `30px` wide, `2.2px` height, `#A3111E`, `5px` gap between them
    - "[ BAG ]" button text — same 15px/500 style as ABOUT, `color: #A3111E`
- **Hover states:** `opacity: 0.7` on all interactive header elements

---

### 2. SCROLL-DRIVEN VIDEO HERO SECTION

- **Container:** `height: 500vh` (creates the scroll distance for video scrubbing), `background: #000000`, `z-index: 10`
- **Sticky viewport:** `position: sticky; top: 0; height: 100vh`, black background, flex column, content pinned to bottom-left with `padding: 48px`
- **Video element:** Absolutely positioned, full-cover (`object-fit: cover`), `z-index: 1`, playsinline, muted, preload="auto"
  - **Primary video URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260627_212146_743b92b3-40a3-46cb-988d-7bf716564ec3.mp4`
- **Hero content overlay** (`z-index: 3`, `max-width: 700px`, left-aligned):
  - **Title:** "MEASURED" on line 1, "PURITY" on line 2
    - Font: Instrument Serif, `clamp(36px, 6vw, 72px)`, weight 400, `line-height: 1.05`, `letter-spacing: -1px`, `color: #000000`
    - Each line wrapped in `<span class="hero-title-line"><span class="hero-title-line-inner">...</span></span>`
  - **DISCOVER button** (below title, `margin-top: 10px`):
    - Capsule shape: white background, `border: 1px solid rgba(0,0,0,0.08)`, `border-radius: 100px`, `padding: 4px 4px 4px 18px`
    - Text "DISCOVER" — 13px, weight 500, uppercase
    - Circle icon: `28px` diameter, `#2e2e2e` background, white `+` cross made with `::before`/`::after` pseudo-elements (10px x 1.5px horizontal, 1.5px x 10px vertical)
    - Hover: background flips to `#000000`, text becomes white, circle becomes white with black cross
    - Click action: smooth scrolls to the footer

---

### CRITICAL VIDEO SCRUBBING TECHNIQUE

The video scrubbing uses a `requestAnimationFrame` loop with a **seeking guard** — this is the most important performance pattern:

```js
if (!video.seeking && Math.abs(video.currentTime - currentTime) > 0.01) {
  video.currentTime = currentTime;
}
```

**The `!video.seeking` check is essential.** We tell the browser: "Update the video frame ONLY when you have completely finished rendering the previous one." (Оновлюй кадр відео ТІЛЬКИ тоді, коли ти повністю закінчив малювати попередній.) Without this guard, the browser's video decoder gets flooded with seek requests, causing freezing, black frames, and stuttering. The smooth interpolation (`currentTime += (targetTime - currentTime) * 0.08`) combined with this seeking guard creates a buttery-smooth cinematic scrubbing experience.

**Full scrubbing behavior:**
- Video is paused; `currentTime` is driven by scroll position within the 500vh container
- Smooth interpolation: `currentTime += (targetTime - currentTime) * 0.08` in a `requestAnimationFrame` loop
- Only updates `currentTime` when `!video.seeking` AND delta > 0.01
- At 80%+ scroll progress, hero text characters individually fade out, blur, and shift upward (per-character staggered animation using cubic easing)
- The DISCOVER button also fades/shifts up with `pow(progress, 4)` easing

**Text splitting:** On DOMContentLoaded, each `.hero-title-line-inner` text is split into individual `<span class="hero-char">` elements for per-character animation.

---

### 3. HORIZONTAL PRODUCT CAROUSEL (Awards Section)

- **Section:** Full viewport height (`100vh`), `overflow: hidden`, `border-top` and `border-bottom: 1px solid #000`, `z-index: 10`
- **Grid:** Flexbox row, `width: max-content`, containing 6 product cards
- **Each card:** `width: 33.333vw`, `border-right: 1px solid #000`, white background, centered content with `padding: clamp(40px, 8vh, 80px) 48px`, `gap: clamp(24px, 4vh, 48px)`
  - Image container: `height: 280px`, centered, `max-width: 85%`, `object-fit: contain`
  - Product name: 18px, weight 400, `letter-spacing: -0.2px`
  - Price: 16px, weight 500, `opacity: 0.6`

**Products (with exact CloudFront image URLs):**

1. **Obsidian Coil** — $480
   - Image: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_144408_92b74dc4-ca69-412a-acfd-304f9b29eb5e_min.webp`

2. **Void Arc** — $560
   - Image: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_145142_ed02063b-d983-47d2-b60b-4b4a5a3448bd_min.webp`

3. **Onyx Hex** — $620
   - Image: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_144747_f21bc119-e460-45be-a071-851291bd71c5_min.webp`

4. **Shadow Sigil** — $740
   - Image: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260627_215521_100b78bd-d24a-4225-b2e8-5bb30d44af73_min.webp`

5. **Eclipse Band** — $820
   - Image: (same as Obsidian Coil) `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_144408_92b74dc4-ca69-412a-acfd-304f9b29eb5e_min.webp`

6. **Matte Skull** — $950
   - Image: (same as Void Arc) `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_145142_ed02063b-d983-47d2-b60b-4b4a5a3448bd_min.webp`

**GSAP ScrollTrigger animation:**
- Section is pinned (`pin: true`, `start: "top top"`)
- Phase 1: Grid scrolls horizontally (translateX from 0 to negative overflow distance), `scrub: true`, `ease: "none"`
- Phase 2: After horizontal scroll completes, a `.video-scaling-wrapper` element symmetrically expands from `width: 0%` to `width: 100%` centered (`left: 50%; transform: translateX(-50%)`) revealing a second scroll-scrubbed video beneath

**Second video:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260628_122130_8b16d300-75cb-49f5-82ce-afd6b79c2d79.mp4`
- Positioned `absolute`, centered with `transform: translate(-50%, -50%)`, `width: 100vw`, `height: 100vh`, `object-fit: cover`
- Uses the same `!video.seeking` guard pattern for smooth scrubbing
- Scrubbed in sync with the expansion progress using the same smooth interpolation technique (0.08 lerp factor)

---

### 4. TWO-COLUMN STATS & COPY SECTION

- **Layout:** CSS Grid `50% 50%`, `border-bottom: 1px solid #000`
- **Left column:** `position: sticky; top: 0; height: 100vh`, flex column, `justify-content: space-between`, `padding: 48px 96px 48px 48px`
  - **Heading:** "Made Without Compromise" — Instrument Serif, `clamp(32px, 4vw, 42px)`, weight 400, `line-height: 1.15`
  - **3 paragraphs** (Inter Tight, 24px, weight 400, `line-height: 1.1`, `letter-spacing: -0.2px`, `gap: 32px`):
    1. "Each ring is forged by a single pair of hands — no factory floor, no assembly line. The material is chosen first; the form follows its nature."
    2. "We work in oxidized silver, blackened bronze, and raw brass. Weights are deliberate. Edges are left where they fall. Nothing is smoothed for comfort."
    3. "OYLA exists for those who wear jewelry that means something. Not decoration — declaration. One piece at a time, made to last a lifetime."
  - **VIEW COLLECTION button** (same capsule style as DISCOVER button)
  - All left-column elements have `data-fade-slide-in` attribute for scroll-triggered fade+slide animation (autoAlpha 0->1, y 20->0, stagger 0.15, duration 0.8, power2.out)

- **Right column:** `border-left: 1px solid #000`, scrollable, contains 4 stat cards:
  - Each card: `padding: 48px`, `min-height: 45vh`, `border-bottom: 1px solid #000` (except last)
  - Each card has a "stomp-wrapper" with two "stomp-stack" divs (stack-a and stack-b), each containing duplicate h1 headings (first is hidden via CSS `:first-child { display: none }`)
  - Heading font: Instrument Serif, `clamp(46px, 5.5vw, 70px)`, `letter-spacing: -1.5px`, `line-height: 0.95`
  - Detail paragraph below: 24px, `padding-top: 42px`, `line-height: 1.2`

**Stat cards content:**

| Stack A | Stack B | Detail |
|---------|---------|--------|
| 100% | Handmade | Every ring crafted by a single artisan, start to finish |
| 14-92g | Per piece | Deliberate weight — each ring is a physical presence you feel |
| Sterling | & Silver | Oxidized metals only — no plating, no compromise |
| Lifetime | Guarantee | We stand behind every piece we make, forever |

Card 4 also has: `<p class="card-subtext">Est. OYLA Studio, 2019.</p>` (13px, `margin-top: 12px`)

**Text splitting for stats:** Each visible `.heading-style-h1:last-child` is split into `<span class="stat-char">` elements. Each `.detail-paragraph` is split into `<span class="detail-word">` elements.

---

### 5. FOOTER (Fixed Reveal Pattern)

- **Position:** `position: fixed; bottom: 0; left: 0; z-index: 1` — revealed as main content scrolls past
- **A `.footer-spacer` div** (transparent, pointer-events none) sits in the normal flow with its height dynamically set to match the footer's height, creating the reveal effect
- **Layout:** CSS Grid `1.2fr 1fr 1.5fr`, `gap: 48px`, `padding: 80px 48px`, white background

**Column 1 — Sign in & Credits:**
- Header: "Sign in" link (18px, weight 400, `letter-spacing: -0.2px`)
- Credits block (18px, `line-height: 1.35`):
  ```
  Handcrafted in small batches
  OYLA Studio
  [blank line]
  Based in Berlin
  (c) OYLA 2026
  All pieces are original designs.
  ```

**Column 2 — Links:**
- Header: "Instagram" link
- Links list (`gap: 12px`): Refund Policy, Privacy Policy, Terms of Service
- All links: 18px, weight 400, hover `opacity: 0.6`

**Column 3 — Newsletter:**
- Header: "Newsletter" (h3, 18px, weight 400)
- Description: "Join our list. Be first to know new drops. 10% off your first order." (18px, `line-height: 1.35`, `max-width: 380px`, `margin-bottom: 24px`)
- Form: Capsule input (`background: #f7f7f5`, `border-radius: 100px`, `padding: 6px 6px 6px 24px`, `max-width: 440px`)
  - Input: placeholder "Email", 14px, transparent background
  - Focus state: `box-shadow: 0 0 0 1px #000000`
  - Submit button: same capsule button pattern with "SUBSCRIBE" text and `+` circle
  - On submit: `alert('You're on the list. Welcome.')`

---

### RESPONSIVE BREAKPOINTS

**Tablet (769px - 1199px):**
- Header: 24px safe zone, nav gap 100px, cart/menu gap 32px
- Left column: padding 40px 32px
- Hero title: 48px fixed
- Stat headings: 44px !important
- Left heading: 30px
- Body text: 18px
- Award cards: 50vw width
- Footer: 2-column grid (last col spans 2)

**Mobile (<=768px):**
- Header: 16px safe zone, 44px touch targets
- Grid collapses to 1 column
- Left column: no longer sticky, auto height
- Hero title: 38px
- Stat headings: 34px !important
- Body text: 15px
- Award cards: 85vw width
- Footer: 1 column, 48px 16px padding
- All buttons: min-height 44px, full-width where appropriate

---

### JAVASCRIPT ANIMATION SYSTEM

Uses GSAP 3.12.5 + ScrollTrigger (loaded from CDN):
```
https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js
https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/ScrollTrigger.min.js
```

Key behaviors:
1. **Video scrub loop** — requestAnimationFrame loop reads scroll position, lerps to target time, uses `!video.seeking` guard to prevent decoder flooding
2. **Hero text exit** — per-character opacity/blur/translateY driven by scroll progress > 80%
3. **Awards horizontal scroll + video reveal** — GSAP timeline with ScrollTrigger pin
4. **Left panel fade-in** — staggered autoAlpha/y animation on scroll
5. **Footer spacer** — dynamically matches footer height for reveal effect
6. **Resize handling** — debounced (200ms), kills all ScrollTriggers and rebuilds on width change
7. **Font-ready init** — animations initialize on `document.fonts.ready` (not window.load) to avoid waiting for slow CDN video downloads

---

### SERVER (Express + Vite)

- Express server on port 3000
- API endpoint `GET /api/higgsfield-video` — proxy that fetches a Higgsfield share page, extracts the direct MP4 URL from HTML source (via regex matching for .mp4 URLs and og:video meta tags), returns JSON `{ success: true, url: "..." }`
- Default share URL: `https://higgsfield.ai/s/keldUFnImRA`
- Second video fetched with query param: `?url=https://higgsfield.ai/s/iK6bYyCxd0I`
- In dev mode: mounts Vite middleware (`middlewareMode: true`, `appType: "spa"`) for HMR
- In production: serves static dist folder with catch-all to `index.html`

---

### KEY IMPLEMENTATION NOTES

- The page content lives in `index.html` with inline CSS and JS (Vite processes it as the SPA entry point)
- Videos have hardcoded CloudFront CDN fallback URLs and also attempt dynamic refresh from the Higgsfield proxy API on page load
- The `.video-overlay` div exists but is fully disabled (`display: none`, transparent)
- Duplicate h1 headings in stomp-stacks: first child hidden by CSS `:first-child { display: none }`, second child is the visible/animated one
- `will-change: width` on video-scaling-wrapper, `will-change: transform` on awards-grid
- `pointer-events: none` on video-scaling-wrapper when width is 0%
- The `!video.seeking` guard is used on BOTH video elements to prevent decoder overload — this is the single most important performance detail for scroll-scrubbed video

## Performance Eyewear — Ecommerce [sites/performance-eyewear]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/glassesstoreArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/performance-eyewear.mp4

Build a single-page vanilla HTML/CSS/JS site (no frameworks, served via Vite) for a luxury performance eyewear brand called "Orven". The entire page is a scroll-driven hero experience with two video scrub phases and animated text reveals. Black background, white text, all elements use `mix-blend-mode: difference` to remain visible over any video frame.

---

### GLOBAL SETUP

- **Font:** Google Fonts "Inter Tight" weights 400 and 500. Body uses weight 500.
- **Colors:** Background `#000`, text `#fff`. No other page background colors.
- **Grid System:** 12-column CSS grid, 24px column-gap, 32px inline padding. CSS custom properties: `--gap: 24px`, `--margin: 32px`, `--ease-expo: cubic-bezier(0.16, 1, 0.3, 1)`.
- **Reset:** `* { margin: 0; box-sizing: border-box }`, `overscroll-behavior: none` on html and body.
- **Dev grid overlay:** A fixed 12-column red-tinted overlay (hidden by default, toggled with "G" key press). Each column is `rgba(255, 0, 0, 0.1)`.

---

### SECTION 1: FIXED HEADER

- Position: fixed, top 24px, full width, height 58px, padding-inline 32px, z-index 100, `mix-blend-mode: difference`.
- Layout: 4 flex columns (equal width `flex: 1 1 0`).
  - **Column 1 (logo):** Text "Orven(R)" at 24px, letter-spacing -0.04em, vertically centered.
  - **Column 2:** Three stacked lines (14px, letter-spacing -0.03em): "Precision engineered", "Essential", "Proven". Left-aligned, 8px gap, left border `1px solid rgba(255,255,255,0.1)`, padding 0 24px.
  - **Column 3:** Two stacked lines (same style): "Innovation Redefined", "Our Story". Same border/padding.
  - **Column 4:** Right-aligned, single line "+ Cart". Same border styling, padding `0 0 0 24px`.

---

### SECTION 2: HERO (scroll height 1200vh)

The hero section is `position: relative; height: 1200vh`. Inside is a sticky container (`position: sticky; top: 0; height: 100vh; overflow: hidden`) containing:

- **Black backdrop** (absolute, inset 0, background #000).
- **Video 2** (behind video 1 in DOM order, starts at opacity 0):
  - URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260712_221651_c3c6edb9-c684-4a9f-b193-fee556ca5622.mp4`
  - Attributes: muted, playsinline, preload="auto". Absolute positioned, object-fit: cover.
- **Video 1** (on top, starts at opacity 1):
  - URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260712_221520_90005dc3-c311-4138-a49f-e22a719f7d8a.mp4`
  - Same attributes/styling.

### Hero UI Overlay (fixed, mix-blend-mode: difference)

- Position: fixed, inset 0, height 100vh, uses the 12-column grid, content aligned to bottom (`align-content: end`), padding-bottom 33px, pointer-events none (children get pointer-events auto).

- **Left cluster (grid-column 1/7):**
  - `<h1>` title "Orven(R)" at 160px, font-weight 500, line-height 100%, letter-spacing -0.04em.
  - Paragraph below: "Performance optics engineered for crystal vision. Precision-crafted lenses tested for durability. Superior technology. Built for excellence" -- 18px, line-height 120%, letter-spacing -0.04em, text-indent 120px.

- **Right cluster (grid-column 9/13):**
  - Min-height 176px, flex column with space-between.
  - Row with label "Info" (width 120px, 18px) and text paragraph (flex 1, 18px, text-indent 120px): "Advanced lens engineering meets optical excellence. Every frame tested for impact resistance. Precision optics drive performance vision forward"
  - Meta row: two spans "(Drop)" and "(2026)" with 25px gap, 18px.

---

### SECTION 3: FEATURE PANELS (fixed overlay, z-index 60)

- Container: fixed, inset 0, padding-top 82px, pointer-events none, `mix-blend-mode: difference`, uses the same 12-column grid.
- Two panels sharing grid-row 1:

**Right Panel (grid-column 9/12):**
- 95px x 2px gradient bar: `linear-gradient(90deg, #888DCC 0%, #02B5B8 25.96%, #868D0A 55.77%, #B2A2B6 95.19%)`, aligned flex-end.
- Body with 80px gap between text and specs list.
- Text (32px, weight 400, line-height 120%, letter-spacing -0.04em, text-indent 120px): "Precision-ground lenses hold razor clarity from edge to edge. Tuned to kill glare, lift contrast, and stay flawless through speed, sweat, and grit."
- Specs list (below a 1px white divider, 12px gap rows):
  - Row: white bar (6x14px) | "Clarity" | "98%"
  - Row: white bar | "Contrast" | "94%"
  - Row: white bar | "UV Filter" | "400nm" | "100%"
  - Labels/values: 14px, uppercase, letter-spacing -0.03em, nowrap. Values right-aligned.

**Left Panel (grid-column 2/5):**
- Same structure as right panel.
- Text: "Aerospace-grade polymer flexes under load and springs back unbroken. Built to swallow impact, shed heat, and stay locked through every hard mile."
- Specs:
  - "Frame Weight" | "22g"
  - "Impact Rating" | "Z87+"
  - "Field" | "180 degrees" | "Wide"

---

### SECTION 4: NEXT (placeholder)

- Simple `<section>` with min-height 100vh, background #000.

---

### ANIMATIONS & SCROLL BEHAVIOR

**Scroll timeline constants:**
- SCRUB1_VH = 3 (video 1 scrub duration in viewport heights)
- FADE_VH = 3 (crossfade duration)
- SCRUB2_VH = 5 (video 2 scrub duration)
- Total hero height = (3 + 3 + 5 + 1) * 100vh = 1200vh

**Phase 1 (0 to 3vh scroll): Video 1 Scrub**
- Video 1 currentTime is scrubbed from 0 to its full duration based on scroll progress 0-1.
- Hero UI text stays visible through first 80% of this phase.
- At 80-100% of phase 1, hero UI fades out (opacity 1 to 0) and drifts up 120px via translateY. Per-word dissolve triggers at 85% (uiFade > 0.15).

**Phase 2 (3vh to 6vh scroll): Crossfade**
- Video 1 opacity goes 1 to 0 AND gets a blur filter from 0 to 80px.
- Video 2 opacity goes 0 to 1.

**Phase 3 (6vh to 11vh scroll): Video 2 Scrub + Feature Panels**
- Video 2 scrubs from 0 to full duration, reaching its last frame at 80% of this phase (then holds).
- Right panel scrolls through viewport over [0%, 40%] of phase 3.
- Left panel scrolls through viewport over [45%, 85%] of phase 3.
- Each panel enters from below the fold (+60px past viewport bottom) and exits fully above the fold.
- Panel movement uses `position: relative; top: Npx` (not transform) to avoid GPU layer issues with mix-blend-mode.

**Video Scrub Engine (RAF-lerp):**
- `onScroll` only updates a `target` time. A persistent requestAnimationFrame loop smoothly interpolates `current` toward `target` with LERP factor 0.09.
- Actual `video.currentTime` seeks are throttled: only when drift > 0.02s AND at least 30ms since last seek.
- Videos are primed on `loadedmetadata` with a play()/pause() trick for iOS Safari compatibility.
- RAF loop auto-stops when both scrubbers settle.

**Word/Char Stagger Reveal System:**
- A `splitReveal(el, mode, stagger)` function splits text into `<span class="reveal-word">` elements, each with `--i` (index) and `--stagger` CSS variables.
- Hidden state: opacity 0, blur 10px, translateY 20px.
- `.reveal-active .reveal-word`: opacity 1, blur 0, translateY 0. Transition 0.4s expo-out, staggered by `calc(var(--i) * var(--stagger))`.
- `.reveal-exit .reveal-word`: opacity 0, blur 9px, translateY -28px. Duration 0.3s.
- Title uses char-level split with 0.025s stagger. All other text uses word-level with 0.05s stagger.
- Hero title cluster reveals immediately on page load.
- Right panel text reveals when its scroll sub-progress is between 0.12 and 0.88.
- Left panel text reveals when its scroll sub-progress is between 0.12 and 0.88.

---

### RESPONSIVE (mobile only, do not change desktop)

**At max-width 768px:**
- Grid: --gap 12px, --margin 16px.
- Title: 56px.
- Title caption: grid-column 1/-1, font-size 14px, text-indent 0.
- Side caption: grid-column 1/-1, min-height auto, margin-top 24px, column direction for row, label width auto at 14px, text 14px with no indent, meta spans 14px.
- Hero UI: padding-bottom 24px.
- Header: padding-inline 16px, top 16px. Nav columns hidden, only logo col shows.
- Feature panels: both go full-width (grid-column 1/-1), text 20px with no indent.

**At max-width 480px:**
- Title: 40px.

---

### TECHNICAL REQUIREMENTS

- Single `index.html` file with all CSS in a `<style>` block and all JS in a `<script>` block at the end of body.
- No external dependencies beyond Google Fonts.
- Served via Vite (plain HTML mode, no bundler plugins needed).
- Videos are hosted externally on CloudFront (URLs above) -- do not download them, reference directly in `<source>` tags.

## Daisy Shop — Ecommerce [sites/shop]

- Preview: https://stream.mux.com/imC5vQh9WbYUhhreIFOOONfNBnQI8AZznRfgZw8p2fA.m3u8
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/shop.m3u8

626f6c742d63632d6167656e74# Complete Prompt: Wild Daisy Fragrances Landing Page

Build a single-page React + TypeScript + Vite landing page using Tailwind CSS. Only `lucide-react` is allowed for icons (none used). Match every detail exactly. Three sections in order: Hero, ScentFinder, WildScent. Page background `#fff`.

### Global Constants

```ts
const TEXT_COLOR = '#000000';
const HERO_TEXT = '#332023';
const BG_BLUE = '#4BB3ED';
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

### Asset URLs (exact, verbatim)

- Hero background video:
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_142713_322c5ac5-8a5d-413b-be68-4a0e82014264.mp4`
- ScentFinder section video (right side / mobile below):
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151802_1bbf9a81-a7cb-4be1-b858-f1cd92b62b96.mp4`
- WildScent section video (left side / mobile below):
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151818_65bb22c5-33ae-4e23-85ea-0a3dd89957c2.mp4`
- Hero card product image (Eau So Fresh):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260511_143221_81001e13-b71c-4a90-b2d7-abf4e2ec08ff.png&w=1280&q=85`
- ScentFinder product image (Eau So Sweet):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260511_151640_5b4a7bf8-4eb2-4a49-aa63-17a9bb642b88.png&w=1280&q=85`
- WildScent product image (Eau So Extra):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260511_151621_4fba6892-ed21-4c2e-8cb3-0bd2ec2abefa.png&w=1280&q=85`

### Product data

```ts
const PRODUCT = { name: 'Eau So Fresh', size: '100 ml / 3.4 oz', image: <hero card image URL above> };

const SCENT_PRODUCT = {
  name: 'Eau So Sweet',
  size: '100 ml / 3.3 oz',
  image: <ScentFinder image URL above>,
  notes: [
    { label: 'Fruity top', ingredient: 'WHITE RASPBERRIES' },
    { label: 'Floral heart', ingredient: 'DAISY TREE PETALS' },
    { label: 'Feminine base', ingredient: 'SUGAR MUSKS' },
  ],
};

const WILD_PRODUCT = {
  name: 'Eau So Extra',
  size: '100 ml / 3.3 oz',
  image: <WildScent image URL above>,
  notes: [
    { label: 'Top', ingredient: 'BANANA BLOSSOM ACCORD' },
    { label: 'Heart', ingredient: 'CHOCOLATE DAISY ACCORD' },
    { label: 'Base', ingredient: 'VETIVER OIL' },
  ],
};
```

### Top-level App

`<div class="min-h-screen" style="backgroundColor:#fff">` containing the three sections in order.

In `App`, create `heroRef` and `v` state. `useEffect` runs `setTimeout(() => setV(true), 200)`.

---

### SECTION 1 — Hero

`<section ref={heroRef} class="relative w-full min-h-screen flex flex-col justify-end overflow-hidden">`

### 1a. Background video
`<video autoPlay muted loop playsInline class="absolute inset-0 w-full h-full object-cover" style="zIndex:0" ref={el => { if (el) el.playbackRate = 1 }}>` with `<source>` `type="video/mp4"` src = hero video URL.

### 1b. Header nav
`<header class="absolute top-0 left-0 w-full flex items-center justify-between px-5 sm:px-8 py-5 sm:py-6" style="zIndex:40, ...anim(v,100,{y:-10,duration:1400}).style">`
- Left: `<div class="font-black text-xs sm:text-sm tracking-widest leading-tight uppercase" style="color:HERO_TEXT">` with two `<div>`s: `Wild Daisy`, `Fragrances`.
- Right: `<nav class="flex gap-5 sm:gap-8">` with two anchors `Shop Now`, `Cart`. Each `<a class="text-xs font-bold tracking-widest uppercase relative group" style="color:HERO_TEXT">` containing inner `<span>` with text and underline `<span class="absolute -bottom-0.5 left-0 h-px w-full origin-left scale-x-0 group-hover:scale-x-100 transition-transform duration-300 ease-out" style="backgroundColor:HERO_TEXT" />`.

### 1c. Scroll indicator (desktop only)
`<div class="hidden sm:block absolute right-8 md:right-10" style="top:50%; transform:translateY(-50%); zIndex:20; ...anim(v,1000,{x:16,duration:1600}).style">` containing `<span class="text-xl tracking-widest" style="fontFamily:'Georgia, serif'; fontStyle:italic; color:HERO_TEXT">Scroll</span>`.

### 1d. Floating product card (desktop only, bottom-right)
`<div class="hidden sm:flex absolute bottom-10 right-10 rounded-2xl items-center gap-2 px-5 py-4" style="zIndex:30; minWidth:260px; backgroundColor:#ffffff; boxShadow:'0 4px 24px rgba(51,32,35,0.08), 0 1px 4px rgba(51,32,35,0.06)'; ...anim(v,1300,{y:20,duration:1400}).style">`
- Image wrapper `<div class="flex-shrink-0 overflow-hidden" style="width:60px; height:76px; borderRadius:8px">` with `<img src=PRODUCT.image alt=PRODUCT.name style="width:130%; height:130%; objectFit:contain; display:block; marginLeft:-15%; marginTop:-15%">`.
- Info column `<div class="flex flex-col">`:
  - `<span class="text-sm font-semibold tracking-wide leading-tight" style="color:HERO_TEXT">Eau So Fresh</span>`
  - `<span class="tracking-wide" style="fontSize:11px; fontWeight:500; marginTop:3px; color:HERO_TEXT">100 ml / 3.4 oz</span>`
  - Button `<button class="text-xs font-bold tracking-widest uppercase self-start leading-tight relative overflow-hidden group" style="marginTop:14px; color:HERO_TEXT">`:
    - `<span class="relative z-10">Add to Cart</span>`
    - `<span class="absolute bottom-0 left-0 h-px w-full origin-left transition-transform duration-300 ease-out scale-x-100 group-hover:scale-x-0" style="backgroundColor:HERO_TEXT" />`
    - `<span class="absolute bottom-0 left-0 h-px w-full origin-right transition-transform duration-300 ease-out delay-150 scale-x-0 group-hover:scale-x-100" style="backgroundColor:HERO_TEXT; opacity:0.4" />`

### 1e. Slide index "01" (desktop only)
`<div class="hidden sm:block absolute left-6 md:left-8" style="top:50%; transform: v ? 'translateY(-50%) translateX(0)' : 'translateY(-50%) translateX(-24px)'; fontFamily:'\"Playfair Display\", \"Didot\", \"Bodoni MT\", \"Times New Roman\", serif'; fontStyle:italic; fontWeight:400; fontSize:'clamp(2.5rem,6.5vw,6rem)'; lineHeight:1; letterSpacing:-0.02em; zIndex:10; color:HERO_TEXT; opacity: v?1:0; transition: 'opacity 1600ms <EASE> 500ms, transform 1600ms <EASE> 500ms'">01</div>`

### 1f. Hero title + mobile card wrapper
`<div class="relative pb-0 sm:pb-12 pl-5 sm:pl-8 pr-0 sm:pr-8" style="zIndex:10">`

`<h1 class="font-medium uppercase leading-tight sm:leading-none" style="fontSize:'clamp(2.2rem,8vw,4rem)'; letterSpacing:-0.01em">` containing six `<span>`:

Mobile lines (each `block sm:hidden`):
1. `Sweet Daisy` — `color:#ffffff; textShadow:'0 2px 16px rgba(0,0,0,0.4)'`, anim(v,600,{y:24,duration:1600}).
2. `Personal Scent` — `color:rgba(255,255,255,0.8); textShadow:'0 2px 12px rgba(0,0,0,0.35)'`, anim(v,800,...).
3. `Finder` — same color/shadow as #2, anim(v,1000,...).

Desktop lines (each `hidden sm:block`):
4. `Sweet Daisy` — `color:HERO_TEXT`, anim(v,600,...).
5. `Personal Scent` — `color:#B0A2A1`, anim(v,800,...).
6. `Finder` — `color:#B0A2A1`, anim(v,1000,...).

### 1g. Mobile inline product card (below title)
`<div class="sm:hidden flex items-center gap-3 mt-4 mr-5 mb-8 px-4 py-4 rounded-2xl" style="backgroundColor:#ffffff; boxShadow:'0 4px 24px rgba(51,32,35,0.08), 0 1px 4px rgba(51,32,35,0.06)'; ...anim(v,1300,{y:20,duration:1400}).style">`
- Image wrapper `<div class="flex-shrink-0 overflow-hidden" style="width:56px; height:70px; borderRadius:6px">` with same image styling pattern as the desktop card (130%, -15% offset).
- Info column `<div class="flex flex-col flex-1">` with name, size (same styles as desktop card), and button identical to the desktop card except `marginTop:12px` and only the FIRST underline span (no second/right-origin underline).

NOTE: card has `mr-5` only (no left margin) so it aligns flush with the title's `pl-5` left edge.

---

### SECTION 2 — ScentFinder

A reusable `ProductPanel({ bg, product, notes, visible, noteStyle = 'normal' })` component:

```
<div class="relative flex flex-col px-6 md:px-8 pt-6 md:pt-8 pb-8 md:pb-10" style="backgroundColor:bg; minHeight:100%">
```

Inside, top labels row:
`<div class="flex items-start justify-between mb-auto" {...anim(visible,0,{y:12,duration:1400})}>`
- Left `<span class="text-xs font-normal" style="color:TEXT_COLOR">{noteStyle==='bold' ? 'Daisy wild' : 'Daisy love'}</span>`
- Right `<span class="text-xs font-normal" style="color:TEXT_COLOR">{noteStyle==='bold' ? 'Playful' : 'Sweet'}</span>`

Product image block `<div class="flex flex-col items-center py-8" style="flex:1; justifyContent:center; ...anim(visible,300,{y:40,duration:1800}).style">`:
- Image wrapper: `<div class="overflow-hidden" style="width:'clamp(140px,40%,220px)'; aspectRatio:'220/340'; backgroundColor:#D9D9D9; borderRadius:2px; flexShrink:0">` with `<img src=product.image alt=product.name style="width:100%; height:100%; objectFit:cover; display:block">`.
- Caption `<div class="text-center mt-4" {...anim(visible,600,{y:10,duration:1400})}>`: name `<p class="text-sm font-normal" style="color:TEXT_COLOR">` and size `<p class="text-xs font-normal mt-1" style="color:TEXT_COLOR">`.

Bottom row `<div class="flex items-end justify-between gap-4 flex-wrap">`:
- Notes column `<div class="flex flex-col gap-0.5" {...anim(visible,900,{y:16,duration:1400})}>`. For each note:
  - Label `<p class="text-xs leading-snug" style="color:TEXT_COLOR; fontWeight: noteStyle==='bold' ? 700 : 400">`
  - Ingredient `<p class="text-xs font-bold tracking-widest uppercase leading-snug" style="color:TEXT_COLOR">`
- Button `<button class="text-xs font-bold tracking-widest uppercase border px-6 py-3 relative group shrink-0" style="color:TEXT_COLOR; borderColor:TEXT_COLOR; backgroundColor:transparent; ...anim(visible,1150,{y:16,duration:1400}).style">`:
  - `<span class="relative z-10 group-hover:text-black transition-colors duration-500">SHOP NOW</span>`
  - `<span class="absolute inset-0 origin-left scale-x-0 group-hover:scale-x-100 transition-transform duration-500 ease-out" style="backgroundColor:#ffffff" />`

### ScentFinderSection

`useRef` + `useState(visible)`, with `IntersectionObserver` threshold `0.15` setting `visible=true` once.

```
<section ref={ref} class="relative w-full">
  <div class="flex flex-col md:grid md:min-h-screen" style="gridTemplateColumns:'1fr 1fr'">
    <ProductPanel bg=BG_BLUE product=SCENT_PRODUCT notes=SCENT_PRODUCT.notes visible=visible />
    <div class="hidden md:block relative overflow-hidden" style="backgroundColor:#111; minHeight:100%">
      <video autoPlay muted loop playsInline class="absolute inset-0 w-full h-full object-cover">
        <source src=<scent video URL> type="video/mp4" />
      </video>
    </div>
    <div class="md:hidden relative overflow-hidden" style="height:75vw; backgroundColor:#111">
      <video autoPlay muted loop playsInline class="absolute inset-0 w-full h-full object-cover">
        <source src=<scent video URL> type="video/mp4" />
      </video>
    </div>
  </div>
</section>
```

---

### SECTION 3 — WildScent

Same observer pattern as Section 2.

```
<section ref={ref} class="relative w-full">
  <div class="flex flex-col-reverse md:grid md:min-h-screen" style="gridTemplateColumns:'1fr 1fr'">
    <div class="hidden md:block relative overflow-hidden" style="backgroundColor:#111; minHeight:100%">
      <video autoPlay muted loop playsInline class="absolute inset-0 w-full h-full object-cover">
        <source src=<wild video URL> type="video/mp4" />
      </video>
    </div>
    <div class="md:hidden relative overflow-hidden" style="height:75vw; backgroundColor:#111">
      <video autoPlay muted loop playsInline class="absolute inset-0 w-full h-full object-cover">
        <source src=<wild video URL> type="video/mp4" />
      </video>
    </div>
    <ProductPanel bg=BG_LIME product=WILD_PRODUCT notes=WILD_PRODUCT.notes visible=visible noteStyle="bold" />
  </div>
</section>
```

Note: `flex-col-reverse` on mobile makes the product panel render above the video (DOM order: video, panel; visual order on mobile: panel, video). On desktop the grid lays them left-to-right (video left, panel right).

---

### Fonts

- No Google Fonts. Tailwind default sans-serif (system stack) for body/UI.
- Inline `Georgia, serif` italic only for the "Scroll" indicator.
- Inline `"Playfair Display", "Didot", "Bodoni MT", "Times New Roman", serif` italic only for the "01" slide index.

### Tailwind / Vite

Stock Tailwind 3 config, default breakpoints (`sm:640px`, `md:768px`). Vite + React + TypeScript starter. No extra packages beyond `react`, `react-dom`, `@supabase/supabase-js`, `lucide-react`.

### Animations Summary

- Hero: triggered 200ms after mount via `setTimeout`. Stagger delays 100, 500, 600, 800, 1000, 1300 ms; durations 1400–1600 ms; easing `cubic-bezier(0.22, 1, 0.36, 1)`. Most spans translateY 20–24px, header translates Y -10, scroll indicator translates X 16, "01" translates X -24 → 0.
- ScentFinder & WildScent: each has `IntersectionObserver` (threshold 0.15) setting `visible=true` once. Stagger inside `ProductPanel`: 0 (top labels, y12), 300 (image block, y40 / 1800ms), 600 (caption, y10 / 1400ms), 900 (notes, y16), 1150 (button, y16).

### SVGs

None. There are no inline SVG paths anywhere in this page.

### Behavioral notes

- Videos autoplay muted in loop, `playsInline` for iOS.
- Mobile breakpoint (<640px): hides desktop floating card, scroll indicator, and "01" index; shows white-with-shadow title and inline product card; videos in sections 2 & 3 become fixed-aspect strips at `height:75vw`.
- Mobile inline card uses `mr-5` (no left margin) so it lines up flush with the title's `pl-5`.
- Section 3 uses `flex-col-reverse` so on mobile the product panel sits above its video.

## Email Marketing — Email Marketing [sites/design-rocket-email-hero]

- Preview: https://motionsites.ai/assets/hero-design-rocket-email-preview-DBed7Yfk.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/design-rocket-email-hero.gif

Prompt: Recreate "Design Rocket Certificates" Email-Style Landing Page
Build a single-page React + TypeScript + Vite + Tailwind CSS project that renders an email-style marketing page for a "Design Rocket Certificates" AI leadership course, built in collaboration with Microsoft. Use lucide-react for icons. No other UI libraries.

Global setup
index.html

Title: Newsletter Design Build Out
Preconnect to fonts.googleapis.com and fonts.gstatic.com
Load Google Fonts: Instrument Serif (ital 0,1) and Inter (weights 400, 500, 600, 700)
src/index.css


@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --font-display: 'Instrument Serif', serif;
  --font-body: 'Inter', sans-serif;
}

body {
  font-family: var(--font-body);
  font-weight: 400;
  -webkit-font-smoothing: antialiased;
}
Headings use inline style={{ fontFamily: "'Instrument Serif', serif" }}. Body copy uses Inter (default).

Page shell
Outer page: min-h-screen bg-[#050505] py-10 px-4 font-sans
Email container: max-w-[640px] mx-auto shadow-2xl overflow-hidden ring-1 ring-white/5
Content card: bg-[#111111] text-[#F2F2F2]
Shared components
Step — numbered row

Wrapper: flex items-start gap-5 mb-6 last:mb-0
Number badge: flex-shrink-0 w-7 h-7 rounded-md bg-[#DCFF00] flex items-center justify-center text-[#0A0A0A] font-bold text-xs mt-1 showing {number}.
Text: text-[17px] leading-[1.55] text-[#E8E8E8]
Divider

py-8 flex justify-center containing h-px w-24 bg-white/20
PrimaryButton (lime CTA, with arrow)

inline-flex items-center gap-3 bg-[#DCFF00] text-[#0A0A0A] font-bold rounded-lg px-6 py-3 hover:bg-[#c9ea00] hover:-translate-y-0.5 transition-all duration-200
Contains the label and a lucide-react ArrowRight icon w-5 h-5 strokeWidth={2.5}
SolidButton (white pill)

inline-block bg-white text-[#0A0A0A] font-bold rounded-lg px-8 py-3 hover:bg-[#E8E8E8] hover:-translate-y-0.5 transition-all duration-200
Section 1 — Hero (video background)
Wrapper: relative w-full overflow-hidden with inline style={{ aspectRatio: '640 / 820' }}
Background video (absolutely filling container, object-cover, autoplay muted loop playsInline): https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260419_064822_f120e48a-d545-45dd-a02d-facb07829888.mp4
Overlay gradient (absolute inset-0): linear-gradient(to bottom, rgba(17,17,17,0) 45%, rgba(17,17,17,0.45) 68%, rgba(17,17,17,0.9) 88%, rgba(17,17,17,1) 100%)
Foreground stack: relative z-10 h-full flex flex-col items-center text-center px-6 pt-12 pb-10
Top brand block (white):
"Design Rocket" — Instrument Serif, text-[28px] leading-[0.95] tracking-tight
"CERTIFICATES" — text-[13px] tracking-[0.22em] font-medium mt-1
Spacer mt-40, then "NOW AVAILABLE" — text-white text-[13px] tracking-[0.28em] font-semibold
flex-1 spacer pushing headline to bottom
Headline (Instrument Serif): text-white text-[58px] leading-[1.02] tracking-tight max-w-[560px]
Text: Learn to lead AI
and unlock new value
CTA pill (note: uses #D8F90A not the card lime):
mt-10 inline-flex items-center gap-3 bg-[#D8F90A] text-[#1E1E1E] font-semibold rounded-full px-8 py-4 hover:bg-[#c9ea00] hover:-translate-y-0.5 transition-all duration-200
Label "Enroll Now" + ArrowRight w-5 h-5 strokeWidth={2.5}
Section 2 — Intro copy + CTA
Container px-[78px] pb-8 pt-4, centered paragraph text-[18px] leading-[1.55]:
Built in collaboration with Microsoft, this certificate course gives you the toolkit to lead AI transformation across your organization. Learn to spot opportunities, launch AI pilots, and scale adoption grounded in responsible practices and proven frameworks.

flex justify-center pb-14 with <PrimaryButton label="Get Started" />
<Divider />
Section 3 — "Transform how you lead with AI"
Heading container px-9 pb-8, Instrument Serif text-center text-[46px] leading-[1.05] tracking-tight: Transform how you lead with AI
Video card px-[42px] pb-10:
Anchor: block overflow-hidden rounded-[14px] group
Video: autoplay/muted/loop/playsInline, w-full h-[370px] object-cover rounded-[14px] transition-transform duration-700 group-hover:scale-[1.03]
Src: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260419_065931_e3ca7b53-d32e-4ad5-81de-dc9d6fcfda6d.mp4
Steps list container px-[76px] pb-10, inner max-w-[489px] mx-auto, rendering four <Step>s:
Learn how to spot AI opportunities that boost productivity across roles and deliver visible results.
Build structures that support your team so AI efficiencies multiply across the organization.
Gain the skills to drive culture change like securing buy-in and reducing resistance.
Get frameworks to deliver AI pilots that prove impact fast and build credibility with measurable results.
flex justify-center pb-14 with <SolidButton label="Enroll Now" />
<Divider />
Section 4 — "Build your AI transformation roadmap"
Heading container pb-7 px-9, Instrument Serif text-center text-[46px] leading-[1.05] tracking-tight:
Build your AI
transformation roadmap
Video card px-[42px] pb-10 (same classes as Section 3) with src:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260417_110451_9f82b157-dc92-4a9f-a341-c25594ec20e1.mp4
Paragraph container px-[78px] pb-8, centered text-[18px] leading-[1.55]:
You'll finish this hands-on course with a personal AI Transformation Plan: your playbook for pilot proposals, data strategy and governance. Use it to help secure buy-in, guide rollout, and scale adoption responsibly.

flex justify-center pb-14 with <SolidButton label="Learn More" />
Section 5 — Lime CTA card
Outer px-14 pb-12
Card: bg-[#D8F90A] rounded-[10px] px-8 py-12 text-center
Heading (Instrument Serif): text-[#1E1E1E] text-[52px] leading-[1.02] tracking-tight mb-3
Ready to lead AI
at work?
Subtext: text-[#1E1E1E] text-[18px] leading-[1.5] mb-8 px-4 — Enroll now and be the leader your team has been waiting for.
Centered <PrimaryButton label="Enroll Now" />
Footer
bg-[#080808] text-white pt-12 px-10 text-center border-t border-white/5
Wordmark link text-[30px] font-bold tracking-tight text-white hover:text-[#DCFF00] transition-colors → "Design Rocket" (wrapped in pb-8 flex justify-center)
Disclaimer paragraph text-[12px] text-[#83837D] leading-[1.5] pb-8:
Microsoft is a collaborator on this specific course. Microsoft does not endorse
Design Rocket generally or other Design Rocket products.

Divider: flex justify-center pb-8 with inner h-px w-24 bg-white/20
Social icon row flex justify-center gap-5 pb-5 — six circular buttons mapping [Facebook, Twitter, Instagram, Youtube, Linkedin, Music2] from lucide-react. Each:
w-10 h-10 rounded-full border border-white/20 flex items-center justify-center hover:bg-white hover:text-[#1E1E1E] hover:border-white transition-colors, icon w-[18px] h-[18px]
Unsubscribe note text-[10px] text-[#83837D] pb-4 leading-[1.6]:
If you no longer want to receive updates on Design Rocket Certificates,
you can unsubscribe at any time by clicking "unsubscribe" below.

Link row text-[12px] pb-3 space-x-2: Support | Privacy | Terms | Unsubscribe (pipes text-[#8F8E88], links hover:underline)
Copyright anchor text-[12px] text-white/80 hover:text-white inline-block:
©2026 Design Rocket, 660 4th Street #443, San Francisco, CA 94107 USA
Trailing pb-10 spacer
Animation / interaction summary
All buttons: hover:-translate-y-0.5 transition-all duration-200 plus background-color change on hover.
Video cards: wrapper overflow-hidden rounded-[14px] group; video scales on hover via transition-transform duration-700 group-hover:scale-[1.03].
Footer wordmark and social icons: smooth color transitions via transition-colors.
Videos auto-play muted, loop, and playsInline for mobile autoplay.
Color palette
Page bg #050505, card bg #111111, footer bg #080808
Text #F2F2F2, secondary #E8E8E8, muted #83837D, divider #8F8E88
Lime primary #DCFF00, lime variant #D8F90A, lime hover #c9ea00
Dark text on lime #0A0A0A / #1E1E1E
Fonts
Display: Instrument Serif (all large headings, wordmark in hero)
Body / UI: Inter

## Outdoor Apparel — Fashion [sites/outdoor-apparel]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/gearstoreArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/outdoor-apparel.mp4

Build a single-page landing website for a fictional outdoor technical gear brand called **ALP1NE** (stylized as `ALP1NE™`). It should be a vanilla HTML/CSS/JS page served by Vite (no React, no frameworks). The entire site lives in one `index.html` file with inline `<style>` and `<script>` tags. Use the font **Inter Tight weight 600 only** from Google Fonts.

---

### GLOBAL SETUP

- Title: `ALP1NE™`
- Font: `Inter Tight`, weight 600 only. Load via: `https://fonts.googleapis.com/css2?family=Inter+Tight:wght@600&display=swap`
- Add `<link rel="preconnect">` for: `fonts.googleapis.com`, `fonts.gstatic.com` (crossorigin), `i.ibb.co` (crossorigin), `d8j0ntlcm91z4.cloudfront.net` (crossorigin)
- CSS reset: `* { box-sizing: border-box; margin: 0; padding: 0; }`
- `html`: `overscroll-behavior: none; overflow-x: hidden;`
- `body`: `background-color: transparent; font-family: "Inter Tight", sans-serif; font-weight: 600; min-height: 100vh; overflow-x: hidden;`

---

### SECTION 1: FIXED BACKGROUND VIDEO (Hero)

A full-viewport fixed video that scrubs forward/backward based on scroll position (not autoplaying normally).

- Container (`#background-wrapper`): `position: fixed; top: 0; left: 0; width: 100%; height: 100vh; z-index: 1; overflow: hidden;`
- Video element (`#bg-video`, class `bg-video`): `width: 100%; height: 100%; object-fit: cover; display: block;`
- Video attributes: `playsinline webkit-playsinline autoplay muted preload="auto" referrerpolicy="no-referrer"`
- **Video URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260706_124521_30407ad9-28f0-481c-9d64-641c619e47e0.mp4`
- The video is "primed" on load for iOS Safari compatibility (muted play then immediate pause to unlock frame-accurate seeking).
- Scroll-to-video mapping: The hero spacer is 250vh tall. As the user scrolls from 0 to that height, `video.currentTime` is lerped from 0 to `video.duration` using `requestAnimationFrame` with smoothing factor 0.12.

---

### SECTION 2: FIXED HEADER (Navigation Bar)

A fixed header at the very top (`z-index: 100`) with three groups: left, center, right.

- Container (`#header-container`): `position: fixed; top: 0; left: 0; right: 0; z-index: 100; display: flex; justify-content: space-between; align-items: center; padding: 12px 14px 0; background: transparent;`

**Cell/Pill base style** (class `.cell`):
- `background-color: #ffffff` (solid white, NOT translucent, for header cells)
- `backdrop-filter: none`
- `color: #000000`
- `font-family: "Inter Tight", sans-serif; font-weight: 600; font-size: 18px; letter-spacing: 0.01em;`
- `padding: 10px 14px; border-radius: 3px; white-space: nowrap; display: inline-flex; align-items: center; justify-content: center;`

**Left group** (`#header-left`): `display: flex; gap: 6px; align-items: center;`
- Cell 1: `ALP1NE™` (id: `cell-brand`)
- Cell 2: Live clock (id: `clock-text`) showing `HH:MM:SS (PST) Weekday Month D YYYY` format, updated every second via `setInterval`. Uses browser local time.

**Center group** (`#header-center`): `display: flex; gap: 6px; align-items: center;`
- Cell: `Collection` (class `center-cell`, `min-width: 140px`)
- Cell: `Journal` (class `center-cell`)
- Cell: `About` (class `center-cell`)

**Right group** (`#header-right`): `display: flex; gap: 6px; align-items: center;`
- Cell: `Instagram`
- Cell: `Press`
- Cell: `Menu` (id: `menu-toggle-btn`, hidden on desktop via `display: none !important`, shown on tablet/mobile)

---

### SECTION 3: MOBILE/TABLET MENU OVERLAY

At `<=1024px` viewport: hide clock, center nav, Instagram, and Press from the header. Show only `ALP1NE™` and `Menu` button. The `Menu` button toggles a fullscreen overlay.

- Overlay (`#mobile-menu-overlay`): `position: fixed; inset: 0; z-index: 99;`
- Inactive state: `opacity: 0; pointer-events: none; background: rgba(0,0,0,0); backdrop-filter: blur(0px);`
- Active state (class `.active`): `opacity: 1; pointer-events: auto; background: rgba(0,0,0,0.08); backdrop-filter: blur(25px);`
- Transition: `opacity 0.4s cubic-bezier(0.16, 1, 0.3, 1)` + matching for background-color and backdrop-filter.
- Content (`.mobile-menu-content`): centered column with `gap: 14px`.
- Each `.cell` inside starts at `opacity: 0; transform: translateY(20px) scale(0.96);` and animates to `opacity: 1; transform: translateY(0) scale(1);` with staggered delays (0.04s increments per child).
- Items: Clock (non-interactive), Collection, Journal, About, Instagram, Press.
- Clicking `Menu` toggles `.active` class and changes button text to `Close`. Clicking any menu item also closes.
- At `<=1024px`, brand cell and menu toggle shrink to `font-size: 16px; padding: 10px 14px;`

---

### SECTION 4: BOTTOM TITLE (Fixed Overlay Text)

A fixed text element at the bottom of the viewport that animates on scroll.

- Container (`#bottom-title-container`): `position: fixed; bottom: 12px; left: 14px; right: 14px; z-index: 19; pointer-events: none; display: flex; justify-content: center; align-items: flex-end; mix-blend-mode: difference;`
- Text (h1, class `bottom-title-text`): `font-size: 80px; font-weight: 600; line-height: 1.0; letter-spacing: -0.015em; text-transform: uppercase; color: #ffffff; mix-blend-mode: difference; text-align: center;`
- Content: `We work in weathered nylon, laminated layers, and raw shell.`
- Responsive: 60px at <=1200px, 40px at <=768px, 28px at <=480px.
- **Scroll animation**: As the user scrolls through the hero (0% to 85% of hero spacer height), the title:
  - Translates up by 150px
  - Scales down from 1.0 to 0.8
  - Fades out (opacity 1 to 0, starting at 30% progress)
  - Blurs up to 24px
  - All smoothed with lerp factor 0.12 per frame.

---

### SECTION 5: SCROLLABLE CONTENT — HERO SPACER

- A transparent div (class `hero-spacer`): `height: 250vh; pointer-events: none;`
- This creates the scroll distance that drives the video scrub and title animation. The first viewport of the page shows the fixed video + bottom title; scrolling through this spacer plays the video forward.

---

### SECTION 6: OLIVE/KHAKI TEXT BLOCK WITH PARALLAX GEAR COLLAGE

After the hero spacer, a section with dark olive background, large white text (with `mix-blend-mode: difference`), and scattered product cutout images that parallax on scroll.

**Section container** (class `yellow-text-section`):
- `background-color: #575234; width: 100%; padding: 224px 24px; display: flex; justify-content: center; align-items: center; position: relative; z-index: 20;`
- CSS variable `--pscale: 1` (scales down at breakpoints: 0.72 at <=1200px, 0.5 at <=768px, 0.4 at <=480px)

**Text block** (`.text-block-container`):
- `max-width: 1406px; width: 100%; margin: 0 auto; color: #ffffff; mix-blend-mode: difference; display: flex; flex-direction: column; gap: 32px; position: relative; z-index: 10;`
- Each `<p>`: `font-size: 70px; line-height: 1.05; letter-spacing: -0.03em; text-indent: 140px; text-align: left;`
- Responsive: 54px/100px indent at <=1200px, 34px/60px at <=768px, 24px/40px at <=480px.

**Text content** (5 paragraphs with decorative Unicode symbols inline):
1. `We build gear ⟡ for people who see the outdoors ≠ as more than scenery ∴ they see it as a challenge.`
2. `Every jacket ⊹ every layer ⊹ every detail is engineered ∿ to perform in the harshest conditions ⟶ so you can stay focused on the journey.`
3. `We believe true performance ⟡ isn't measured by appearance ∴ but by how your gear responds to rain ∧ snow ∧ relentless wind ∧ high-altitude environments.`
4. `Whether you're heading into a multi-day expedition ↟ climbing a summit ◇ or hiking before sunrise ⊹ our equipment is built to help you move with confidence.`
5. `Because the moments that matter most begin where comfort ends ✦ adventure begins.`

**Parallax gear images** — Scattered product cutout PNGs in two layers:

BACK LAYER (`.parallax-layer.back`, `z-index: 5` — behind text):
- Jacket: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/jacket.fca57bc0.png` | class `item-jacket` | position: `top: 2%; left: 7%` | width: `calc(640px * var(--pscale))` | data-speed="0.14" data-drift="-0.22" data-rot="-0.6"
- Gaiters: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/gaiters.aa2524de.png` | class `item-gaiters` | position: `top: 58%; left: 74%` | width: `calc(346px * var(--pscale))` | data-speed="0.42" data-drift="0.26" data-rot="-1.0"

FRONT LAYER (`.parallax-layer.front`, `z-index: 11` — in front of text):
- Shorts: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/shorts.1a032e36.png` | class `item-shorts` | position: `top: 9%; left: 66%` | width: `calc(326px * var(--pscale))` | data-speed="0.44" data-drift="0.28" data-rot="0.9"
- Cap: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/cap.b10e4e2c.png` | class `item-cap` | position: `top: 33%; left: 7%` | width: `calc(310px * var(--pscale))` | data-speed="0.46" data-drift="-0.30" data-rot="1.4"
- Scarf: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/scarf.427effaf.png` | class `item-scarf` | position: `top: 28%; left: 64%` | width: `calc(467px * var(--pscale))` | data-speed="0.30" data-drift="0.24" data-rot="-1.2"
- Backpack: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/backpack.0558dccb.png` | class `item-backpack` | position: `top: 50%; left: 37%` | width: `calc(596px * var(--pscale))` | data-speed="0.18" data-drift="0.16" data-rot="0.5"
- Boot: `https://order-twine-70493179.figma.site/_components/v2/5922cc7522062a402e02d607e26cc654a692d2ad/boot.545d53aa.png` | class `item-boot` | position: `top: 72%; left: 13%` | width: `calc(224px * var(--pscale))` | data-speed="0.60" data-drift="-0.30" data-rot="1.8"

**Parallax animation logic**:
- Each item has `data-speed`, `data-drift`, and `data-rot` attributes.
- On each animation frame, calculate how far the viewport center is from the section center, normalize to a -1.2..1.2 range, smooth with lerp 0.12.
- Vertical offset: `-smoothP * speed * 520px`
- Horizontal offset: `smoothP * speed * drift * 520px`
- Rotation: `smoothP * rot * 4deg`
- Applied via `transform: translate3d(x, y, 0) rotate(r)`

---

### SECTION 7: LOOP SCROLL SPACER

- A div (`#loop-scroll-spacer`): `height: 3400px; width: 100%; background-color: #575234; position: relative; z-index: 20;`
- This olive-colored spacer provides scroll distance that drives the product slider reveal animation below.

---

### SECTION 8: PRODUCT SLIDER (Scroll-Driven Clip-Path Reveal)

A fixed overlay that reveals through an expanding clip-path window as the user scrolls through the loop-scroll-spacer, then swipes horizontally between full-screen product cards.

**Overlay** (`#product-slider-overlay`): `position: fixed; inset: 0; z-index: 30; display: flex; clip-path: inset(50% 50% 50% 50%);`

**Reveal animation**:
- Calculated from the bottom of the page: `REVEAL_PX = 700` scroll pixels for the window to open, then `CARD_PX = 900 * (NUM_CARDS - 1)` scroll pixels for card transitions.
- The clip-path animates from `inset(50% 50% 50% 50%)` (invisible) to `inset(0px 0px round 3px)` (full viewport) using `easeInOutCubic` easing, smoothed with lerp 0.15.

**Card track** (`#product-slider-track`): `display: flex; height: 100%; will-change: transform;`

**Product cards** (4 cards, each `100vw x 100vh`, `flex-shrink: 0`):

1. Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_111701_92784626-1c2b-4db6-afd7-dd456e7a4717.png&w=1920&q=85` | Label: `Shell Jacket ↟` | Price: `$1,200`
2. Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_111730_62a35ec8-335d-4bea-9337-780589328a03.png&w=1920&q=85` | Label: `Arc Layer ◇` | Price: `$890`
3. Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_111749_b8ced0e0-177c-4cca-9d14-b3fdd7bedf27.png&w=1920&q=85` | Label: `Field Cap ⊹` | Price: `$180`
4. Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260707_111837_c28c966d-4a92-408f-8ac3-0c9bac445704.png&w=1920&q=85` | Label: `Summit Haul 40 ↟` | Price: `$540`

**Card image styling**: `width: 140%; height: 100%; object-fit: cover; margin-left: -20%;` with horizontal parallax (18% shift factor) as cards scroll.

**Card info overlay**: positioned `bottom: 32px; left: 32px;` as a flex row with gap 8px. Each info cell is solid white, `font-size: 18px; padding: 13px; letter-spacing: -0.03em;`

**Center symbol** (`#slider-symbol`): `position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); font-size: 600px; color: #ffffff; mix-blend-mode: difference; z-index: 51; transition: opacity 0.2s ease;`
- Randomly swaps between these Unicode symbols on each card transition: `◇ ≠ ↟ ✦ ⊹ ∧ ⟡ ∴ ⟶ ∿`
- Swap animation: fade out opacity to 0, wait 150ms, change character, fade back to 1.
- At <=768px: `font-size: 200px`

---

### ANIMATION ARCHITECTURE (requestAnimationFrame loop)

One single `requestAnimationFrame` loop (`animate()`) drives everything:

1. **Video scrub**: Maps scroll position (0 to hero spacer height) to video currentTime (0 to duration). Smoothed with lerp 0.12. Rate-limited to seek no more than every 30ms.
2. **Title animation**: Maps scroll fraction (0-0.85) to translateY (-150px), scale (1.0 to 0.8), opacity (fade starting at 30%), and blur (up to 24px). Also fades with the slider reveal.
3. **Parallax gear items**: Activates when viewport is near the olive section. Normalized distance drives per-item transforms.
4. **Slider reveal**: Last portion of scroll (bottom REVEAL_PX + CARD_PX of total page) drives clip-path expansion with easeInOutCubic.
5. **Card swipe**: After reveal completes, remaining scroll drives horizontal translateX on the track.
6. **Card image parallax**: Each card image shifts horizontally based on its distance from the current position.
7. **Symbol swap**: Fires when the rounded card index changes.

All values use lerp smoothing (factors 0.12-0.15) for buttery 60fps animation without any libraries.

---

### iOS / MOBILE COMPATIBILITY

- Video is primed via `play().then(pause())` pattern on load and on first touch/click events.
- All `-webkit-` prefixes included for clip-path and backdrop-filter.
- `playsinline` and `webkit-playsinline` attributes on video.
- Touch events registered as `{ passive: true }`.

## Wealth Video Hero — Fintech [sites/0]

- Preview: https://motionsites.ai/assets/hero-wealth-preview-B70idl_u.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/0.gif

Create a modern, high-impact hero section for a wealth management platform using React and Tailwind CSS.

Layout & Background:

The section must be full viewport height (min-h-screen) with a black background.
Background Video: Use this specific video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260207_050933_33e2620d-09cd-43a2-80ef-4cdbb42f4194.mp4. It should be autoplaying, looped, and muted.
Video Styling: The video must be scaled to 150% of its size (scale-150) with the focal point aligned to the top-left corner (object-left-top, origin-top-left).

Navbar:

Place a transparent navbar at the absolute top.
Include a white logo on the left.
Center navigation links: "Features" (with a chevron down icon), "Company", and "Blogs". These should be white with hover opacity effects.
Right side actions: A "Sign in" text link and a white "Get Started" pill-shaped button with black text.

Hero Content (Centered):

Tag: A glassmorphic pill at the top saying "Real-Time Budget Tracking" (white text, semi-transparent border/bg).
Headline: Huge, centered white text saying "Build Wealth That Lasts Generations" (responsive font size, up to ~100px on desktop).
Subtitle: "Transform today's earnings into tomorrow's family fortune with proven wealth-building strategies" (white text with slight transparency).
CTA: A prominent white pill button saying "Start Building Wealth" with black text and a hover scale effect.

Bottom Features Grid:

Place a floating card container near the bottom of the screen.
Style: Dark glassmorphism effect (bg-black/70, backdrop-blur-xl, white border).
Grid: 4 columns listing these steps:
Create Your Free Account: Sign up in seconds using your email address or mobile number.
Connect Your Bank Accounts: Securely link your bank accounts, cards, or digital wallets with.
Set Your Financial Goals: Customize your savings, spending, or investment goals with easy.
Track, Grow, and Optimize: Watch your money work for you in real time—get insights and tips.

## Evergreen Finance — Fintech [sites/evergreen-finance]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(79).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/evergreen-finance.webp

Build a "Kova" fintech landing page in React + Vite + Tailwind CSS + Framer Motion + Lucide React. The page has 3 sections: a full-screen Hero with a boomerang video background, a Testimonial section, and a Features section. Use the exact specifications below. Do NOT use purple/indigo colors anywhere.

---

### FONTS

Load these two web fonts in `index.html` via `<link>` tags:
- `https://db.onlinewebfonts.com/c/53077f9a3eee9c479d37d6af20394ded?family=Cooper+BT+W01+Light`
- `https://db.onlinewebfonts.com/c/5ade3423145f3b9f7031574333ca0b73?family=Cooper+BT+W01+Medium`

Define two utility classes in your CSS:
- `.font-cooper` — `font-family: 'Cooper BT W01 Light', 'Georgia', serif;`
- `.font-cooper-medium` — `font-family: 'Cooper BT W01 Medium', 'Cooper BT W01 Light', 'Georgia', serif; font-weight: 500;`

---

### COLOR PALETTE

- Primary dark green: `#08150C`
- Hover dark green: `#1a2e1f`
- Warm cream background: `#FDF5EB`
- Light beige card: `#EBE4DC`
- Inner card beige: `#F4F1EC`
- Donut chart colors: `#C46B2D`, `#7A8C3E`, `#A8B87A`, `#B8AFA4`
- Body/text: stone-600, stone-700, stone-800 (Tailwind)
- Accent greens: emerald-400, emerald-500 (Tailwind)

---

### ANIMATIONS (FadeUp Component)

Create a reusable `<FadeUp>` component using Framer Motion with two modes:
- **`immediate` (prop)**: Animates on mount using `animate="visible"` — used for Hero elements.
- **Default (scroll-triggered)**: Uses `whileInView="visible"` with `viewport={{ once: true, margin: '-60px' }}` — used for Testimonial and Features sections.

Variants:
- `hidden`: `{ opacity: 0, y: 24, filter: 'blur(8px)' }`
- `visible`: `{ opacity: 1, y: 0, filter: 'blur(0px)' }`
- Transition: `{ duration: 0.7, delay: [configurable], ease: [0.25, 0.1, 0.25, 1] }`

Props: `children`, `delay` (default 0), `className`, `immediate` (default false).

---

### SECTION 1: HERO (full viewport height)

### Background — Boomerang Video

Create a `<BoomerangVideoBg>` component that:
1. Loads this video (muted, playsInline, crossOrigin="anonymous"): `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260517_070729_32a7eb4e-d6e2-4571-badc-91b4dab1ecbe.mp4`
2. Captures every frame into offscreen canvas elements (max width 960px) as the video plays through once using `requestVideoFrameCallback` (with a `requestAnimationFrame` fallback).
3. After the video ends, plays back captured frames in a forward/reverse boomerang loop on a visible `<canvas>` at 30fps.
4. Wraps everything in `absolute inset-0 w-full h-full scale-[1.08] origin-center`.
5. Shows the `<video>` while capturing, then swaps to the `<canvas>` once frames are ready.

### Navbar (FadeUp delay=0, immediate)
- Flex row, `justify-between`, padding `px-5 sm:px-10 lg:px-16 py-5`
- Left: Brand name "Kova" in `font-cooper text-xl sm:text-2xl text-[#08150C] tracking-tight`
- Center (hidden on mobile, `hidden md:flex`): Links "Explore", "Pricing" (active with underline bar), "Perks", "Reach" — `text-sm text-stone-700`, hover to `text-[#08150C]`. Active link has `font-medium text-[#08150C]` with `absolute -bottom-1 left-0 right-0 h-0.5 bg-[#08150C] rounded-full` underline span.
- Right desktop: "Get Started" button — `bg-[#08150C] text-white text-sm font-medium px-5 py-2.5 rounded-xl hover:bg-[#1a2e1f]`
- Right mobile: Hamburger (Menu/X icons from Lucide, size 22), toggles a dropdown menu with same links + button, styled `bg-white/95 backdrop-blur-md shadow-lg`

### Hero Content (centered, flex-col items-center text-center)
- `px-5 sm:px-10 pt-8 sm:pt-14 pb-8 sm:pb-14`
- **Heading** (FadeUp delay=0.1, immediate): `font-cooper text-[2.2rem] sm:text-5xl md:text-6xl lg:text-7xl text-[#08150C] leading-tight max-w-5xl tracking-tight` — Text: "Own your money and build the wealth you deserve"
- **Subtext** (FadeUp delay=0.25, immediate): `mt-4 sm:mt-5 text-sm sm:text-base text-stone-600 max-w-sm sm:max-w-md leading-relaxed` — Text: "Step into a smarter way to bank, right from your pocket. Kova gives you instant control over your money, wherever you are."
- **CTA Buttons** (FadeUp delay=0.4, immediate): Two buttons in `flex-col sm:flex-row gap-3`:
  1. "Watch 30s Demo" — white/80 backdrop-blur, border stone-200, Play icon (size 14, fill-stone-800), rounded-xl
  2. "Get the App" — bg-[#08150C] text-white, Download icon (size 14), rounded-xl

### Dashboard Cards (bottom of hero, FadeUp immediate)
Three cards in a flex row (`items-end justify-center gap-2 sm:gap-4`), outer two hidden on mobile (`hidden sm:block`):

1. **SavingsCard** (delay=0.55, w-44 sm:w-64): White/95 backdrop-blur rounded-2xl, shows "Savings" label, "+25%" badge, "+12%" badge, an SVG line chart (green polyline with gradient fill), month labels Jan-Apr.
2. **OthersCard** (delay=0.65, w-44 sm:w-72): "Others" header with "Monthly" dropdown pill, three percentage stats (78% Groceries, 43% Entertain., 23% Transport), bar chart (12 bars, 5th bar orange `#f97316`, rest gray `#d1d5db`).
3. **BillPayCard** (delay=0.75, w-44 sm:w-64): "Bill Pay" header with "Monthly" dropdown pill, "-8%" red badge, bar chart (12 bars, 7th bar dark `#08150C`, rest light gray `#e5e7eb`), month labels.

---

### SECTION 2: TESTIMONIAL

Background: `bg-[#FDF5EB] py-14 sm:py-20 px-5 sm:px-10 lg:px-20`
Layout: `max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-[3fr_2fr] gap-10 md:gap-16 items-center`

### Left Column (scroll-animated FadeUp, staggered delays 0 through 0.4):
- **Heading** (delay=0): `font-cooper-medium text-2xl sm:text-3xl text-[#08150C] leading-snug mb-6 sm:mb-8` — "Trusted by ambitious, fast-moving teams"
- **Company badge** (delay=0.1): Dark square icon "A" (`w-7 h-7 rounded-md bg-[#08150C]`) + "Arcvex" text
- **Quote** (delay=0.2): `font-cooper text-stone-700 text-lg sm:text-xl md:text-2xl leading-relaxed mb-5 sm:mb-6` — "With Kova, I have full visibility into our team's spending in real time. It feels like having a sharp financial advisor available at every hour, helping us stay on budget and make wiser calls."
- **Attribution** (delay=0.3): "Maya Reeves" (text-sm font-semibold) + "Director, Arcvex" (text-xs text-stone-500)
- **Button** (delay=0.4): "All Stories" with arrow SVG icon, same dark button style

### Right Column (FadeUp delay=0.15, scroll-triggered):
- A looping muted autoplay video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260517_074029_c7a854bd-2d6e-4b62-96b3-ae8c16311e44.mp4`
- Styling: `w-full rounded-2xl object-cover aspect-square`, wrapped in `max-w-xs sm:max-w-sm`

---

### SECTION 3: FEATURES

Background: `bg-[#FDF5EB] py-14 sm:py-20 px-5 sm:px-10 lg:px-20`
Layout: `max-w-7xl mx-auto`

### Header Row (scroll-animated):
- **Heading** (FadeUp delay=0): `font-cooper-medium text-2xl sm:text-3xl md:text-4xl text-[#08150C] leading-snug` — "Designed to sharpen every decision"
- **Button** (FadeUp delay=0.1): "Watch Demo" with Play icon (size 13, fill-white), same dark button style

### Cards Grid: `grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4`

Each card is `aspect-[3/4] rounded-2xl overflow-hidden`, scroll-animated with staggered delays (0.05, 0.15, 0.25, 0.35):

**Card 1 — Smart Budgeting** (delay=0.05):
- Background image (absolute, object-cover): `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061249_f20dfeda-1033-45ce-a3ee-070965599cbf.png&w=1280&q=85`
- Gradient overlay: `bg-gradient-to-t from-[#08150C]/80 via-[#08150C]/20 to-transparent`
- Top label: Sparkles icon (Lucide, size 16, white) + "Smart Budgeting" in white text-sm font-medium
- Bottom text: "Let AI reshape how you plan your spending. Kova adapts to your..." in `text-white/80 text-sm sm:text-base`

**Card 2 — Bank-Grade Security** (delay=0.15):
- Background image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061305_db631f5f-185f-4fda-a7a8-1dd7359ef2ea.png&w=1280&q=85`
- Same gradient overlay
- Top label: ShieldCheck icon (Lucide, size 16, white) + "Bank-Grade Security"
- Bottom text: "Keep your money safe with end-to-end encryption, live fraud alerts, and two-factor auth..."

**Card 3 — Spend Insights** (delay=0.25):
- NO background image. Solid background `#EBE4DC`, with `p-5`
- Top label: PieChart icon (Lucide, size 16, text-stone-700) + "Spend Insights" in `text-stone-700 text-sm font-medium`
- Inner container: `rounded-2xl p-4` with background `#F4F1EC`, centered content:
  - "Monthly Spend" title (text-sm sm:text-base font-semibold text-stone-800)
  - "1 Apr – 30 May 2026" subtitle (text-xs sm:text-sm text-stone-500)
  - Donut chart (SVG, viewBox="0 0 36 36", `-rotate-90`): 4 colored arcs using strokeDasharray/strokeDashoffset on circles (r=14, strokeWidth=5). Colors: `#C46B2D` (26.4/61.56), `#7A8C3E` (22/65.96, offset -26.4), `#A8B87A` (17.6/70.36, offset -48.4), `#B8AFA4` (22/65.96, offset -66)
  - Center overlay: "50%" bold + "of budget" small text

**Card 4 — Wealth Building** (delay=0.35):
- Background image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260517_061316_50e651f8-02d0-4add-9ddb-7d81d15ac02e.png&w=1280&q=85`
- Same gradient overlay
- Top label: TrendingUp icon (Lucide, size 16, white) + "Wealth Building"
- Bottom text: "Grow your net worth with tools that help you set targets, monitor gains, and act..."

---

### DEPENDENCIES

```json
{
  "framer-motion": "^12.38.0",
  "lucide-react": "^0.344.0",
  "react": "^18.3.1",
  "react-dom": "^18.3.1"
}
```

Dev: Vite, Tailwind CSS 3, TypeScript, PostCSS, Autoprefixer.

---

### GLOBAL CSS (`index.css`)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  * { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; overflow-x: hidden; }
}

.font-cooper {
  font-family: 'Cooper BT W01 Light', 'Georgia', serif;
}

.font-cooper-medium {
  font-family: 'Cooper BT W01 Medium', 'Cooper BT W01 Light', 'Georgia', serif;
  font-weight: 500;
}
```

---

### RESPONSIVE BEHAVIOR

- Mobile-first. Cards stack on small screens (1 col), 2 cols at `sm`, 4 cols at `lg`.
- Hero dashboard cards: outer two hidden below `sm`.
- Nav links/CTA hidden below `md`, replaced by hamburger menu.
- All text sizes step up at `sm` and `md` breakpoints.
- Testimonial grid is single column on mobile, `3fr 2fr` at `md`.

---

### KEY IMPLEMENTATION NOTES

- The entire page background is white for the hero (video fills it) and `#FDF5EB` for the lower two sections.
- All buttons use `rounded-xl` (not full pill).
- The BoomerangVideoBg uses `scale-[1.08]` to prevent edge gaps during playback.
- No page scroll on the hero (`min-h-screen overflow-hidden`).
- The hero content uses `flex-1 flex flex-col justify-between` to push cards to the bottom.

## FinFlow — Fintech [sites/finflow]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(81).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/finflow.webp

Build a single-page React + Vite + TypeScript landing page hero section using Tailwind CSS and `lucide-react`. Replace the contents of `src/App.tsx` with a single default-exported component. Do not install any extra packages.

**Layout & background**

- Root container: `relative min-h-screen flex flex-col overflow-hidden`, with inline `fontFamily: "'ITC Avant Garde Gothic W02 Bk', sans-serif"`.
- Behind everything, render an HTML5 `<video>` absolutely positioned `inset-0 w-full h-full object-cover z-0`, with `autoPlay muted loop playsInline` and an inline style `filter: 'saturate(0)'` (fully desaturated/grayscale).
- Use this exact src:
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260602_132418_e0e79d08-5d1f-42d9-b8ae-8dd69217aacf.mp4`
- All foreground content lives inside `<div className="relative z-10 flex flex-col min-h-screen">`.

**Navbar**

- `<nav>` with classes `flex items-center justify-between px-4 sm:px-8 py-4 sm:py-5 max-w-7xl mx-auto w-full`.
- Left: text logo "Fenvex" in `text-lg sm:text-xl font-semibold tracking-tight select-none`, color `#111111`.
- Center (desktop only, `hidden md:flex`): pill-shaped link group with `items-center gap-1 px-2 py-1.5 rounded-full`, background `#e5e5e5`. Links: `Platform`, `Tutorials`, `Compare`, `Solutions`. Each link: `text-sm px-4 py-1.5 rounded-full text-[#1a1a1a] hover:bg-white/50 transition-colors duration-200`.
- Right (desktop only): two buttons.
  - "Log in": `text-sm px-5 py-2 rounded-full transition-colors duration-200 hover:bg-white/20`, inline style `border: '1.5px solid #222222', color: '#222222'`.
  - "Sign up": `text-sm text-white px-5 py-2 rounded-full transition-all duration-200 hover:opacity-90`, inline style `background: 'linear-gradient(to bottom, #3a3a3a, #111111)', border: '1.5px solid transparent'`.
- Mobile (`md:hidden`): hamburger button using `lucide-react`'s `Menu`/`X` icons (size 22, color `#111111`), classes `p-2 rounded-full transition-colors duration-200 hover:bg-white/20`. Toggle a `mobileMenuOpen` `useState` boolean.

**Mobile dropdown**

- When `mobileMenuOpen` is true, render below the nav: `relative z-20 mx-4 rounded-2xl px-4 py-4 flex flex-col gap-2 md:hidden`, background `#e5e5e5`.
- Same four nav links as anchor tags: `text-sm px-4 py-2 rounded-xl text-[#1a1a1a] hover:bg-white/50 transition-colors duration-200`.
- Footer row inside: `flex gap-2 pt-2 border-t border-[#c0c0c0]` containing full-width "Log in" (outline) and "Sign up" (black gradient) buttons matching the desktop styling.

**Hero main**

- `<main className="flex-1 flex flex-col items-center justify-center text-center px-4 pb-32 sm:pb-40 -mt-40">`.
- `<h1>` with classes `font-bold leading-tight mb-4 sm:mb-5`, inline style `fontSize: 'clamp(1.75rem, 6vw, 3.75rem)', maxWidth: '800px', lineHeight: 1.1, color: '#111111'`. Text: **"Discover a faster path to financial flow"**.
- `<p>` with classes `text-sm sm:text-base md:text-lg mb-8 sm:mb-10 max-w-xs sm:max-w-md leading-relaxed`, color `#333333`. Text: **"Tap the Fenvex platform to craft payment experiences that are fast, trusted, and effortless."**
- CTA wrapper: `flex flex-col sm:flex-row items-center justify-center gap-3 sm:gap-4 w-full max-w-xs sm:max-w-none`.
  - Primary CTA "Start building": `w-full sm:w-auto text-center text-white text-sm px-7 py-3 rounded-full transition-all duration-200 hover:opacity-90 shadow-lg`, inline style `background: 'linear-gradient(to bottom, #3a3a3a, #111111)', border: '1.5px solid transparent'`.
  - Secondary CTA "Reach our team": `w-full sm:w-auto text-center text-sm px-7 py-3 rounded-full transition-colors duration-200 backdrop-blur-sm hover:bg-white/20`, inline style `border: '1.5px solid #222222', color: '#222222'`.

**Logos card (bottom)**

- Wrapper: `w-full px-4 pb-6 sm:pb-10 flex justify-center`.
- Card: `w-full max-w-4xl bg-white rounded-2xl px-4 sm:px-8 py-5 sm:py-6 grid grid-cols-3 sm:flex sm:items-center sm:justify-between gap-4 sm:gap-6`, inline style `boxShadow: '0 20px 60px rgba(0,0,0,0.18), 0 4px 16px rgba(0,0,0,0.1)'`.
- Render six logos as `<img>` tags from simple-icons CDN (native brand colors, no filter):
  - Shopify: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/shopify.svg`
  - Stripe: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/stripe.svg`
  - Visa: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/visa.svg`
  - Apple Pay: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/applepay.svg`
  - Mastercard: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/mastercard.svg`
  - PayPal: `https://cdn.jsdelivr.net/npm/simple-icons@v11/icons/paypal.svg`
- Each logo wrapped in `<div className="flex items-center justify-center opacity-60 hover:opacity-100 transition-opacity duration-200" title={name}>`.
- Default image size: `h-6 sm:h-7 w-auto`. For Visa, Apple Pay, and Mastercard only, use `h-7 sm:h-8 w-auto` (~15% larger).

**Other notes**

- Do not add any background overlays — the desaturated video is the direct backdrop and the foreground text is dark gray/black on top.
- No additional animations beyond the Tailwind `transition-*` and `hover:*` utilities specified above.
- Do not modify `index.html`, Tailwind config, or `index.css`. Everything lives in `src/App.tsx`.

---

## Modern Dental Clinic — Healthcare [sites/modern-dental-clinic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/dentalblu.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/modern-dental-clinic.mp4

Build a full-screen dental clinic hero section using React + Tailwind CSS + Lucide React icons. Use the Inter font (weights 300, 400, 500, 600, 700) from Google Fonts. The entire page uses a single `HeroSection` component.

---

**BACKGROUND & VIDEO:**

- The section is `h-screen w-full overflow-hidden` with a fallback background color of `#5F9AD1` (a calm mid-blue).
- Behind all content, place an autoplaying, muted, looping, playsInline `<video>` element absolutely positioned to cover the section.
- Video source URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260710_141802_1d85412a-1df8-4993-8fc4-7400520bb1d1.mp4`
- On desktop (md+): the video is `inset-0 h-full object-center`.
- On mobile: the video sits at the bottom 70% of the screen (`top-[30%] h-[70%]`), with `object-[80%_center]`.
- The video fades in with a custom `fadeIn` animation: `animate-[fadeIn_1.2s_ease-out_0.2s_both]`.
- On mobile only, add a gradient overlay div at `top-[30%]` that fades from `#5F9AD1` to transparent (height 128px, z-index 1) to blend the solid blue top into the video below.

---

**NAVIGATION (header):**

- Positioned at the top with horizontal padding `px-6 md:px-8 lg:px-16` and top padding `pt-6 md:pt-8 lg:pt-12`.
- Animates in with: `animate-[slideDown_0.7s_ease-out_0.1s_both]`.
- **Logo (left):** A custom SVG tooth/pin icon (white fill with a `#5F9AD1` inner shape), 28x32 on mobile, 32x36 on md+. Next to it, the text "SmileLab" in white, `text-xl md:text-2xl lg:text-[28px] font-medium tracking-tight`.
- **Desktop nav links (center, hidden on mobile):** "About" (white, font-medium), "Results", "Pricing", "Reviews", "Blog" (all `text-white/60`, hovering to white). Font size `text-lg`, gap `gap-8 lg:gap-12`.
- **Desktop CTA button (right, hidden on mobile):** A white pill button (`rounded-full px-5 py-3`) containing "Contacts" in black `text-lg`, plus a `#EBFA73` (lime-yellow) circle (w-7 h-7) with an `ArrowUpRight` icon in `#5F9AD1`. The circle scales on hover (`group-hover:scale-110`).
- **Mobile hamburger (hidden on md+):** A toggle button using Lucide `Menu` and `X` icons with animated crossfade (opacity + rotation + scale transitions, 300ms).

---

**MOBILE MENU OVERLAY:**

- Fixed fullscreen, z-50, with a `#5F9AD1/95` backdrop + `backdrop-blur-md`.
- Menu items ("About", "Results", "Pricing", "Reviews", "Blog") are `text-3xl font-light` white, staggered entrance (each item delayed by 60ms + 100ms base).
- A "Contacts" pill button at the bottom (same style as desktop CTA), delayed 400ms.
- Easing: `cubic-bezier(0.16, 1, 0.3, 1)`. Duration: 500ms for all transitions.
- Close button (X icon) positioned `top-6 right-6`.

---

**MAIN HEADING:**

- Container: `px-6 md:px-8 lg:px-16 max-w-3xl mt-8 md:mt-6 lg:mt-10`, centered on mobile, left-aligned on md+.
- Animates with: `animate-[blurIn_0.9s_ease-out_0.3s_both]` (starts blurred and transparent, becomes sharp and opaque).
- The `<h1>` is white, `text-[72px] sm:text-6xl lg:text-[90px] xl:text-[100px]`, `font-normal`, `leading-[0.9] md:leading-[0.85]`, `tracking-tight`.
- Text reads:
  ```
  Restore
  Your True
  Smile [avatars]
  ```
- The word "Smile" and the avatar group are in an `inline-flex items-end gap-4 lg:gap-6` span.
- **Avatar group (hidden on mobile):** Three overlapping circles (`-space-x-2`, `mb-[0.1em]`):
  1. Pexels photo 1239291 (woman), `w-10 h-10 lg:w-14 lg:h-14`, rounded-full, `border-2 border-[#5F9AD1]`, object-cover.
  2. Pexels photo 774909 (woman), same sizing.
  3. A white circle with `+2k` text in `#3D8CD5`, `text-xs lg:text-base font-medium`.

---

**SUBTEXT (hidden on mobile):**

- Below the heading with `mt-5 lg:mt-6`, `max-w-md text-lg leading-tight`.
- Mixed opacity text: "Using " (white/60) + "advanced technology" (white) + ", we deliver comprehensive treatments for a healthy, " (white/60) + "confident smile." (white).

---

**BOTTOM-LEFT STAT + FIGURE (hidden on mobile):**

- Positioned `absolute bottom-0 left-4 lg:left-12`.
- Animates: `animate-[slideUp_0.9s_ease-out_0.8s_both]`.
- **Stat overlay:** Positioned `absolute top-8 lg:top-12 left-3 lg:left-4 z-20` above the image. Shows "98%" in `text-[#3D8CD5] text-2xl lg:text-4xl font-bold` and "loyal dental patients" in `text-xs lg:text-sm font-medium text-center`, same blue color.
- **Person image:** URL `https://soft-zoom-63098134.figma.site/_assets/v11/ecccf0c10f5c64505f8cb104b04c72aba0b85b0c.png?w=512`. Sized `w-52 sm:w-64 lg:w-80`, `object-contain`, z-10. This is a transparent-background PNG of a smiling woman.

---

**CUSTOM KEYFRAME ANIMATIONS (in global CSS):**

```css
@keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }
@keyframes blurIn { from { opacity: 0; filter: blur(12px) } to { opacity: 1; filter: blur(0px) } }
@keyframes slideDown { from { opacity: 0; transform: translateY(-20px) } to { opacity: 1; transform: none } }
@keyframes slideUp { from { opacity: 0; transform: translateY(30px) } to { opacity: 1; transform: none } }
@keyframes float { 0%,100% { transform: translateY(0px) } 50% { transform: translateY(-8px) } }
```

---

**KEY COLORS:**
- Primary blue: `#5F9AD1`
- Accent lime: `#EBFA73`
- Stat text blue: `#3D8CD5`
- White at 60% opacity for secondary text

**FONT:** Inter (Google Fonts), applied to body with antialiased rendering.

**TECH:** React, Tailwind CSS, Lucide React (`ArrowUpRight`, `Menu`, `X`), Vite, TypeScript.

## Scroll Landing Page — Interactive [sites/scroll-landing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(47).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/scroll-landing.webp

**Create a React + Vite + Tailwind CSS v4 landing page for "WISA" -- a premium football/soccer organization website. The page has a scroll-driven video background, 3 content sections, and a glassmorphism footer. Use ONLY these dependencies: react 19, motion (framer-motion v12+), gsap, lucide-react, tailwindcss v4 with @tailwindcss/vite plugin. The design is dark, cinematic, minimal, with Manrope (sans) and JetBrains Mono (mono) fonts.**

---

### GLOBAL SETUP

**package.json dependencies (exact):**
```
react, react-dom ^19.0.0
motion ^12.23.24
gsap ^3.14.2
lucide-react ^0.546.0
tailwindcss ^4.1.14
@tailwindcss/vite ^4.1.14
@vitejs/plugin-react ^5.0.4
vite ^6.2.0
```

**vite.config.ts:** Use `@tailwindcss/vite` plugin + `@vitejs/plugin-react`. Alias `@` to project root.

**index.html:** Standard HTML5. Include `<script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/3.4.0/model-viewer.min.js"></script>` in head.

**src/index.css -- EXACT:**
```css
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Manrope:wght@300;400;500;600;700&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Manrope", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, monospace;
}

@keyframes flyOutRight {
  0% { transform: translateX(0); }
  100% { transform: translateX(250%); }
}

@keyframes flyInLeft {
  0% { transform: translateX(-250%); }
  100% { transform: translateX(0); }
}

.animate-fly-out {
  animation: flyOutRight 0.5s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}

.animate-fly-in {
  animation: flyInLeft 0.5s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}

@keyframes flyOutUp {
  0% { transform: translateY(0); }
  100% { transform: translateY(-150%); }
}

@keyframes flyInUp {
  0% { transform: translateY(150%); }
  100% { transform: translateY(0); }
}

.animate-fly-out-up {
  animation: flyOutUp 0.4s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}

.animate-fly-in-up {
  animation: flyInUp 0.4s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}
```

These define 4 keyframe animations:
- `flyOutRight / flyInLeft` (250% translateX, 0.5s) -- for the arrow button hover
- `flyOutUp / flyInUp` (150% translateY, 0.4s) -- for nav text hover
- All use `cubic-bezier(0.4, 0, 0.2, 1)` easing with `forwards` fill mode

---

### COMPONENT: ScrollReveal (`src/components/ScrollReveal.tsx` + `ScrollReveal.css`)

**ScrollReveal.css:**
```css
.scroll-reveal { margin: 0; }
.scroll-reveal-text { display: flex; flex-wrap: wrap; margin: 0; }
.word { display: inline-block; white-space: pre; }
```

**ScrollReveal.tsx:** A GSAP-powered word-by-word scroll reveal component.
- Props: `children` (string), `scrollContainerRef?`, `enableBlur` (default true), `baseOpacity` (default 0.1), `baseRotation` (default 3), `blurStrength` (default 4), `containerClassName`, `textClassName`, `rotationEnd` (default "bottom bottom"), `wordAnimationEnd` (default "bottom bottom")
- Splits children text by whitespace into `<span className="word">` elements using `useMemo`
- Three GSAP ScrollTrigger animations:
  1. **Rotation**: Container rotates from `baseRotation` degrees to 0, origin "0% 50%", scrub true, trigger start "top bottom", end = `rotationEnd`
  2. **Opacity**: Each `.word` fades from `baseOpacity` to 1, stagger 0.05, scrub true, trigger start "top bottom-=20%", end = `wordAnimationEnd`
  3. **Blur** (if `enableBlur`): Each `.word` goes from `blur(blurStrength px)` to `blur(0px)`, same stagger/trigger as opacity
- Renders: `<h2 ref={containerRef} className="scroll-reveal {containerClassName}"><p className="scroll-reveal-text {textClassName}">{splitText}</p></h2>`
- Cleanup: kills all ScrollTrigger instances on unmount

---

### COMPONENT: Reveal (inline in App.tsx)

A motion.div wrapper for viewport-triggered fade-in:
```tsx
function Reveal({ children, delay = 0, className = "" }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 30 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-50px" }}
      transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1], delay }}
      className={className}
    >
      {children}
    </motion.div>
  );
}
```
Easing is `[0.16, 1, 0.3, 1]` (ease-out-expo style).

---

### COMPONENT: NavItem (inline in App.tsx)

A hover-animated navigation link with vertical text fly animation:
- Uses a `cycle` counter state (useState(0))
- On `mouseEnter` and `mouseLeave`: increment cycle
- When `cycle === 0` (initial, no hover yet): render single `<span>` with `text-white/64` and `group-hover:text-white transition-colors duration-300`
- When `cycle > 0`: render TWO spans keyed by cycle -- one with `.animate-fly-out-up` (exits upward), one absolute-positioned with `.animate-fly-in-up` (enters from below)
- Container: `<a>` with `relative overflow-hidden group flex items-center justify-center py-1`

---

### MAIN APP (src/App.tsx) - ARCHITECTURE

**Video URL constant:**
```
const VIDEO_URL = 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260521_064421_279656fd-e76f-40a0-8fed-7456d4f7715a.mp4';
```

**State & Refs:**
- `arrowCycle` (useState(0)) -- for arrow button hover animation, same pattern as NavItem
- `videoRef` (useRef HTMLVideoElement)
- `videoContainerRef` (useRef HTMLDivElement)
- `isLoaded` (useState false) -- tracks when video is ready
- `screen3Ref` (useRef HTMLDivElement) -- reference to footer section for scroll calculation
- `scrollY` from motion's `useScroll()`
- `headerY` = `useTransform(scrollY, [0, 500, 800], [0, 0, -150])` -- header slides up and out after scrolling past 500px

---

### SCROLL-DRIVEN VIDEO - CRITICAL IMPLEMENTATION

**Effect 1: Video Loading**
```tsx
useEffect(() => {
  const video = videoRef.current;
  if (!video) return;
  const handleCanPlay = () => setIsLoaded(true);
  video.addEventListener('canplaythrough', handleCanPlay);
  video.load();
  return () => video.removeEventListener('canplaythrough', handleCanPlay);
}, []);
```

**Effect 2: Scroll-to-Video-Scrub (with the `video.seeking` guard)**
```tsx
useEffect(() => {
  if (!isLoaded) return;
  const video = videoRef.current;
  if (!video || !video.duration) return;

  const handleScroll = () => {
    if (!screen3Ref.current || video.seeking) return;
    // ^^ CRITICAL: "video.seeking" check tells the browser: "Only update the video
    // frame when you've completely finished rendering the previous one."
    // Without this guard, rapid scroll events queue up competing .currentTime assignments,
    // causing visible frame tearing, flickering, and dropped frames. The browser's
    // internal seek operation is asynchronous -- setting .currentTime while a previous
    // seek is still in progress gets silently ignored or causes visual glitches.
    // By checking video.seeking, we skip scroll events that arrive before the prior
    // frame has been decoded and painted, resulting in smooth, tear-free scrubbing.

    const rect = screen3Ref.current.getBoundingClientRect();
    const absoluteTop = window.scrollY + rect.top;
    const stopScroll = Math.max(1, absoluteTop - (window.innerHeight * 0.2));
    const scrollFraction = Math.max(0, Math.min(1, window.scrollY / stopScroll));
    video.currentTime = scrollFraction * video.duration;
  };

  window.addEventListener('scroll', handleScroll, { passive: true });
  handleScroll();
  return () => window.removeEventListener('scroll', handleScroll);
}, [isLoaded]);
```

The scroll fraction maps from 0 (top of page) to 1 (when the footer section is 20% of viewport height from top). This means the video plays through its full duration as the user scrolls from top to the footer.

---

### SECTION 0: LOADING SCREEN

Shown when `!isLoaded`. Fixed fullscreen, z-50, black bg, centered:
- "LOADING" text: `text-[10px] font-mono tracking-widest mb-4 text-white/50`
- Progress bar below: `w-64 h-[1px] bg-white/10 mt-8 overflow-hidden` with inner `h-full bg-white w-1/3 animate-pulse`

---

### LAYER STRUCTURE

The entire page is layered:
1. **Fixed video background** (`fixed inset-0 z-0 bg-black`) -- video is absolutely centered with cover behavior using `transform: translate(-50%, -50%)`, `minWidth/minHeight: 100%`, `objectFit: cover`
2. **Fixed header** (z-20) -- animated with motion, slides out via `headerY` transform
3. **Scrollable content** (`relative z-10 pointer-events-none`) -- all sections flow here, with `pointer-events-auto` on interactive areas

---

### SECTION 1: HERO (Screen 1)

Container: `w-[90%] mx-auto h-screen flex flex-col py-8 md:py-12 lg:py-16 pb-12`

Inner main: `flex-1 w-full pointer-events-auto flex flex-col md:grid md:grid-cols-12 md:grid-rows-[1fr_auto] gap-y-8 md:gap-y-0 md:gap-x-8`

**Grid layout (desktop 12-col, 2-row):**

1. **Heading** (bottom-left): `md:row-start-2 md:col-start-1 md:col-span-8 flex items-end`
   - H1: `text-[clamp(2.5rem,6vw,5rem)] leading-[1.05] font-medium tracking-tight text-white whitespace-nowrap`
   - Text: "Championing" `<br/>` "The Pitch Of Legends"
   - Wrapped in `<Reveal delay={0.2}>`

2. **Description paragraph** (center-right): `md:row-start-1 md:col-start-8 md:col-span-5 flex flex-col justify-center items-start md:items-end text-left md:text-right`
   - Paragraph: `text-[clamp(1rem,1.6vw,1.375rem)] text-white/64 leading-[1.3] font-normal max-w-[460px] relative -top-[90px]`
   - Text: "Advanced preparation and training of world-class football teams for leagues, tournaments, and trophies. **We bring the trophy closer to your cabinet.**" (bold part is `font-semibold text-white`)
   - Wrapped in `<Reveal delay={0.3}>`

3. **CTA Button** (bottom-right): `md:row-start-2 md:col-start-8 md:col-span-5 flex items-end justify-start md:justify-end`
   - Two-part button with 1px gap (`flex items-stretch gap-1 group cursor-pointer`)
   - **Text part**: `px-8 py-5 bg-white/8 backdrop-blur-[80px]` -> on group-hover: `bg-white`. Text: "EXPLORE OUR STADIUMS" in `font-mono text-[12px] tracking-[-0.01em] text-white/90` -> hover: `text-black`
   - **Arrow part**: `px-6 bg-white/8 backdrop-blur-[80px]` -> hover: `bg-white`. Contains `<ArrowRight>` (lucide, w-5 h-5) with the same fly-out/fly-in animation pattern as NavItem but horizontal (`.animate-fly-out` / `.animate-fly-in`)
   - `arrowCycle` state drives the animation, same increment pattern on mouseEnter/mouseLeave
   - Wrapped in `<Reveal delay={0.4}>`

---

### SECTION 1.5: SPACER

`<div className="h-[200px] w-full"></div>` -- 200px empty gap

---

### SECTION 2: SCROLL-REVEAL TEXT + 3-COLUMN GRID

Container: `w-[90%] mx-auto min-h-screen flex flex-col justify-center py-8 md:py-12 lg:py-16 pointer-events-auto`

Inner: `max-w-[1200px] w-full`

**ScrollReveal component usage:**
```tsx
<ScrollReveal
  baseOpacity={0.1}
  enableBlur={true}
  baseRotation={3}
  blurStrength={4}
  textClassName="text-[clamp(2rem,4.5vw,4rem)] leading-[1.1] font-medium tracking-tight text-white w-full"
>
  Complete Football Programs For Professional Player Development. We Build The Foundations For Next-Generation Strikers, Midfielders, And Star Defenders.
</ScrollReveal>
```

**3-Column Grid below** (`mt-24 grid grid-cols-1 md:grid-cols-12 gap-12 md:gap-8`):

1. **Col 1 (md:col-span-4)**: Globe SVG (71x43 wireframe globe) + WISA logo SVG (157x25, scaled to h-[18px] w-auto) side by side with `gap-4`. Below: tagline "Winning the future on pitch" in `text-[11px] font-mono tracking-widest text-white/60 uppercase leading-relaxed`. Wrapped in `<Reveal delay={0.1}>`

2. **Col 2 (md:col-span-4)**: H3 "Performance Analytics / Facilities" (`text-xl font-medium text-white`), paragraph below (`text-[15px] text-white/80 leading-relaxed`). Wrapped in `<Reveal delay={0.2}>`

3. **Col 3 (md:col-span-4)**: H3 "Matchday Premium / Fan Experiences!" same styling, paragraph same styling. Wrapped in `<Reveal delay={0.3}>`

---

### SECTION 2.5: SPACER

Another `h-[200px]` spacer

---

### SECTION 3: FOOTER (ref={screen3Ref})

This is the scroll endpoint for the video scrub calculation. Wrapped in `pointer-events-auto`.

**Footer container**: `width: 90%, margin: 0 auto, paddingBottom: 64px` (inline styles)

**Inner card** (glassmorphism): `backgroundColor: rgba(26, 26, 26, 0.6)`, `backdropFilter: blur(80px)`, `WebkitBackdropFilter: blur(80px)`, `border: 1px solid rgba(255, 255, 255, 0.1)`, `padding: clamp(32px, 4vw, 64px)` -- all inline styles

**CTA Section** (top of footer card):
- Flexbox wrap, `alignItems: flex-end`, `justifyContent: space-between`, `gap: 40px`
- Bottom border: `1px solid rgba(255, 255, 255, 0.1)`, `paddingBottom: clamp(48px, 4vw, 80px)`
- H2: "Ready To Score / Your Winning Season?" -- `fontSize: clamp(2rem, 4.5vw, 3.5rem)`, `fontWeight: 500`, `letterSpacing: -0.02em`, `lineHeight: 1.05`
- Button: Same two-part pattern (text + arrow) but with white bg / black text, `padding: 20px 32px` and `20px 24px`. Text: "START YOUR SEASONS" in `font-mono, 12px, -0.01em tracking, bold 700`

**Footer Links Grid** (`paddingTop: clamp(48px, 4vw, 64px)`):
- CSS Grid: `repeat(auto-fit, minmax(160px, 1fr))`, `gap: clamp(32px, 3vw, 48px)`
- 4 columns:
  1. **Brand**: WISA logo SVG (h:14px) + tagline paragraph (13px, rgba white 0.4, maxWidth 220)
  2. **Company**: Header "COMPANY" (10px mono, 0.1em tracking, rgba white 0.3) + links: About, Rosters, Press, Contact (14px, rgba white 0.6)
  3. **Services**: Header "SERVICES" same style + links: Coaching, Training Camp, Fitness, Tryout
  4. **Connect**: Header "CONNECT" same style + links: LinkedIn, X / Twitter, YouTube, Newsletter

**Copyright Bar** (`marginTop: 56, paddingTop: 32, borderTop: 1px solid rgba white 0.1`):
- Flex wrap space-between
- Left: "2026 WISA. ALL RIGHTS RESERVED." (11px mono, rgba white 0.25, 0.1em tracking)
- Right: PRIVACY | TERMS links (same styling, gap-24px)

---

### FIXED HEADER

`<motion.header>` with:
- `style={{ y: headerY }}` -- slides out after scroll 500-800px
- `initial={{ opacity: 0, y: 20 }}`, `animate={{ opacity: 1 }}`, easing `[0.16, 1, 0.3, 1]`, duration 0.8
- Classes: `fixed top-0 left-1/2 -translate-x-1/2 z-20 w-[90%] flex items-center justify-between pointer-events-auto py-4 md:py-6 lg:py-8`

**Left: WISA Logo SVG** (157x25, white, 4 paths spelling "WISA")

**Right: Navigation bar** (`hidden lg:flex items-stretch bg-[#1A1A1A]/40 backdrop-blur-[80px]`):
- Nav links container: `flex items-center justify-between px-6 font-mono text-xs tracking-[-0.01em] w-[480px]`
- 5 NavItem components: LEAGUES, STADIUMS, TRAINING, COMPETITIONS, TICKETS
- CTA button: `bg-white text-black px-6 py-5 font-mono text-xs leading-4 font-bold tracking-[-0.01em] hover:bg-gray-200 transition-colors w-[148px]` -- text "BUY MATCH PASS"

---

### SVG ASSETS

**WISA Logo** (used 3 times -- header, section 2, footer): 157x25 viewBox, 4 white paths. The paths spell "W I S A" in a custom typeface.

**Globe icon** (used in section 2 col 1): 71x43 viewBox, wireframe globe with horizontal/vertical/meridian lines, stroke white, no fill.

Both SVGs are inlined directly. They are too detailed to describe -- copy the exact path data from the source code above.

---

### KEY DESIGN TOKENS SUMMARY

| Token | Value |
|-------|-------|
| Font sans | Manrope 300-700 |
| Font mono | JetBrains Mono 400-700 |
| Background | Pure black (#000) |
| Text primary | white |
| Text secondary | white/64 (rgba 255,255,255,0.64) |
| Text muted | white/60, white/50, white/40, white/25 |
| Glass bg | #1A1A1A at 40% opacity |
| Glass blur | 80px |
| Glass border | rgba(255,255,255,0.1) |
| Button bg | white/8 -> white on hover |
| Spacing rhythm | 90% viewport width container, clamp-based responsive values |
| Easing (motion) | [0.16, 1, 0.3, 1] |
| Easing (CSS) | cubic-bezier(0.4, 0, 0.2, 1) |

## Sky Estate — Real Estate [sites/sky-estate]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/galaxyhome.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/sky-estate.mp4

**Create a luxury real estate landing page called "Galaxy Home" (brand name "Aether Lane") using React, TypeScript, Vite, Tailwind CSS, and Framer Motion. Use the Google Font "Inter Tight" (weights 400, 500, 600, 700). The page has a dark/cosmic aesthetic with parallax scrolling effects and layered imagery.**

---

### Tech Stack & Dependencies
- React 18, TypeScript, Vite
- Tailwind CSS 3.4
- Framer Motion 12
- Lucide React for icons

### Tailwind Config
Custom brand colors:
- `brand-blue`: `#8F9EFF`
- `brand-navy`: `#271C40`
- `brand-dark`: `#020319`

Custom animations:
- `marquee`: translateX 0% to -50%, 30s linear infinite
- `marquee-reverse`: translateX -50% to 0%, 30s linear infinite

Font: `'Inter Tight', system-ui, sans-serif`

### Global CSS
```css
body { font-family: 'Inter Tight', system-ui, sans-serif; overflow-x: hidden; }
* { -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
html { scroll-behavior: smooth; }
```

---

### Section 1: Navbar (fixed, centered, floating pill)
- Fixed at top center with `z-[60]`, padding `px-4 pt-4 md:pt-6`
- Pill shape: `rounded-full bg-[#312D7C]/40 backdrop-blur-[15px]`, gaps `gap-4 md:gap-8 lg:gap-20`, padding `px-4 py-3`
- Logo: custom SVG (4-petal/clover shape) at `h-6 w-6 md:h-7 md:w-7`, fill white, viewBox `0 0 256 256`, path: `M 228 0 C 172.772 0 128 44.772 128 100 L 128 0 L 0 0 L 0 28 C 0 83.228 44.772 128 100 128 L 0 128 L 0 256 L 28 256 C 83.228 256 128 211.228 128 156 L 128 256 L 256 256 L 256 228 C 256 172.772 211.228 128 156 128 L 256 128 L 256 0 Z`
- Brand name: "Aether Lane", `text-base md:text-xl font-medium text-white`, gap-2 from logo
- Nav links: `['Home', 'About', 'Estates', 'Projects', 'Inquire']`, hidden on mobile, `gap-6`, `text-sm`, first link white, rest `text-[#B6B8C3] hover:text-white`
- CTA button: "Get in touch", `rounded-full border border-white/80 px-6 py-2 text-[15px] font-medium`, background: `linear-gradient(180deg, rgba(255,255,255,0) 30%, rgba(255,255,255,0.10) 76%), radial-gradient(ellipse at 50% 100%, rgba(255,255,255,0.7) 0%, transparent 100%), #BEC7FF`, text color `text-brand-navy/80 drop-shadow-sm`, hover scale 1.05
- Mobile: Animated hamburger (3 bars that animate to X using Framer Motion), opens fullscreen overlay (`bg-brand-dark/95 backdrop-blur-xl`) with staggered link animations (fade up + blur)

---

### Section 2: Hero (full viewport height, parallax layers)
- Full `h-screen w-full overflow-hidden`, `z-10`
- Uses `useScroll` targeting the section with offset `['start start', 'end start']`
- Parallax `bgY`: transforms scrollYProgress [0,1] to ['0%', '8%']

**Layer 1 (z-0) - Sky background:**
- Image URL: `https://soft-zoom-63098134.figma.site/_assets/v11/7af55796a90a26e2d57c9fa2a48815874023cff0.png`
- `h-[120%] w-full object-cover`, animated with `y: bgY`

**Layer 2 (z-10) - Title text:**
- Positioned with `pt-[22vh] md:pt-32 lg:pt-36`, centered
- Text: "Galaxy  Home" (with `&nbsp;&nbsp;` double space between words)
- Font: `text-[clamp(3rem,14vw,14rem)] font-semibold leading-none whitespace-nowrap`
- Gradient text: `bg-clip-text text-transparent`, `backgroundImage: 'linear-gradient(to bottom, #A8B4FF, #FFFFFF)'`
- `mix-blend-lighten` to blend with layers

**Layer 3 (z-20) - Subtexts:**
- Left: "Elegance Above the Skyline" at `left-6 top-[200px] md:left-12 md:top-[320px] lg:left-24`, `text-lg md:text-[22px] md:leading-6 font-medium text-white/70 mix-blend-overlay`, hidden on mobile
- Right: "Your Dream Residence Starts Here" at `right-6 top-[200px] md:right-12 md:top-[320px] lg:right-24`, same styling but `text-white`, hidden on mobile

**Layer 4 (z-30) - Building image (overlaps text):**
- Image URL: `https://soft-zoom-63098134.figma.site/_assets/v11/644aba5492aa8bd5756bc5c6d65255d577b1aaf3.png`
- `h-[120%] w-full object-cover`, same parallax `y: bgY`

---

### Section 3: Mountain Background + Content (overlaps hero by -25vh)
- Wrapper: `relative z-40 -mt-[25vh]`
- Mountain image in background with separate parallax: `useScroll` on entire wrapper (offset `['start start', 'end end']`), transforms to ['0%', '-20%']
- Mountain image URL: `https://soft-zoom-63098134.figma.site/_assets/v11/3d8fdaf726b804c1299840860af873a910ce1571.png`
- `h-[120%] w-full object-cover object-top`, positioned `absolute -top-[10vh] left-0 right-0 bottom-0`

---

### Section 4: Content Section (inside mountain wrapper)
- Padding: `pt-24 sm:pt-32 md:pt-40`

**Description block:**
- Centered column, `px-5 sm:px-6 py-16 md:py-32`
- Text: "Explore distinguished estates, iconic design, and meticulously curated homes across the globe's most sought-after destinations."
- `max-w-[600px] text-sm sm:text-base md:text-lg font-medium text-white/90 leading-relaxed text-center`
- Text shadow: `2px 4px 26px rgba(0, 0, 0, 0.56)`
- CTA button: same style as navbar "Get in touch", `mt-6 sm:mt-7 px-8 sm:px-10 py-2.5 sm:py-3 text-base sm:text-lg`

---

### Section 5: Text Fill Section (scroll-driven character reveal)
- Container: `max-w-[820px]` centered, `px-5 sm:px-6 pb-16 sm:pb-24 pt-8 sm:pt-12 md:px-12`
- Text: "We present refined estates that merge remarkable design, prime surroundings, and relentless craftsmanship. Each residence is chosen for the experience it delivers not merely the footprint it provides."
- Font: `text-xl sm:text-2xl md:text-[40px] md:leading-[48px] text-center leading-snug tracking-tight font-medium text-white`
- Animation: Each character uses `useTransform` on scroll progress. Characters start at `opacity: 0.25` and animate to `opacity: 1` as the user scrolls. Uses `useScroll` with offset `['start 0.8', 'end 0.2']`. Each character calculates its own start/end threshold based on its index relative to total character count (range +/- 0.01 around its position).

---

### Section 6: Logo Marquee (infinite scroll)
- Container: `max-w-[820px]` centered, `py-4 sm:py-6`, overflow hidden
- Row 1 (left to right): Sparkles/"Prism", Waves/"Cascade", Star/"Pinnacle" (repeated), uses `animate-marquee`
- Row 2 (right to left): Zap/"Impulse", Orbit/"Nexus", Gem/"Radiant" (repeated), uses `animate-marquee-reverse`
- Each logo item: Lucide icon (`h-4 w-4 sm:h-5 sm:w-5 text-white/80`) + name (`text-sm sm:text-base font-medium text-white/90 whitespace-nowrap`)
- Double-duplicated arrays for seamless loop, `gap-8 sm:gap-12`

---

### Section 7: Stats Section
- `px-5 sm:px-6 py-16 md:py-32`
- White gradient overlay on top: `absolute -top-[400px] left-0 right-0 bottom-0`, `bg-gradient-to-b from-transparent via-white/60 to-white`
- Heading: "Only the proven results here", `text-2xl md:text-[40px] md:leading-[44px] font-medium text-brand-navy text-center mb-10 sm:mb-16`
- Stats grid: `grid-cols-2 md:flex`, `max-w-5xl` centered
- Stats data:
  - "500" / "Estates Delivered"
  - "25" / "Exclusive Markets"
  - "12" / "Years in the Field"
  - "99%" / "Owner Satisfaction"
- Values: `text-4xl sm:text-5xl md:text-[64px] md:leading-[76px] font-semibold text-brand-navy`
- Labels: `text-sm sm:text-base md:text-xl font-medium text-brand-navy/70 text-center`
- Dividers between stats (desktop only): `h-[80px] w-px bg-brand-navy/20 mx-8 lg:mx-12`
- Animate in view with Framer Motion `useInView` (once, margin -100px): fade up (`opacity: 0, y: 30` to `opacity: 1, y: 0`), staggered by 0.15s per stat, duration 0.6s

---

### Image URLs (exact)
1. Sky/hero background: `https://soft-zoom-63098134.figma.site/_assets/v11/7af55796a90a26e2d57c9fa2a48815874023cff0.png`
2. Building (transparent foreground): `https://soft-zoom-63098134.figma.site/_assets/v11/644aba5492aa8bd5756bc5c6d65255d577b1aaf3.png`
3. Mountain/landscape: `https://soft-zoom-63098134.figma.site/_assets/v11/3d8fdaf726b804c1299840860af873a910ce1571.png`

## CleanTech — Sustainability [sites/cleantech]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/earhtling.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cleantech.mp4

**Create a scroll-driven landing page for "Terova" -- a circular-systems / waste-reclamation tech company. Use React + Vite + TypeScript + Tailwind CSS + Framer Motion + Lucide React. The page has 4 major zones stacked vertically.**

---

### GLOBAL SETUP

- **Font**: Google Fonts `Inter` (weights 300, 400, 500, 600, 700). Body uses `'Inter', system-ui, sans-serif`. Tailwind config overrides `fontFamily.sans` to `"Flexo Soft Medium", system-ui, sans-serif` (but only affects Tailwind's `font-sans` utility -- the body CSS directly sets Inter).
- **Background color**: `#19261D` (dark muted green-black)
- **Text color**: white
- **Anti-aliasing**: `-webkit-font-smoothing: antialiased`
- **Selection**: `rgba(255, 255, 255, 0.2)` background
- **Dependencies**: `framer-motion ^12.42.2`, `lucide-react ^0.344.0`, `react ^18.3.1`, `react-dom ^18.3.1`

---

### PERSISTENT ELEMENT: DICE ICON (Fixed, bottom-right)

A fixed-position element at `bottom-4 right-4` (responsive: `sm:bottom-6 sm:right-6`), `z-50`. It's a 10x10 (sm:11x11, md:12x12) rounded-md box with background `#E2DBC8`. Inside is a 3x3 CSS grid showing 5 dots (like the "5" face of a die) -- dots are 7px circles colored `#1C261E`. The dots sit at positions: top-left, top-right, center, bottom-left, bottom-right.

---

### SECTION 1: HERO (Fixed fullscreen with parallax fade)

**Behavior**: The hero is `position: fixed; inset: 0; z-index: 0`. As the user scrolls, it fades out using JavaScript: `opacity = Math.max(0, 1 - scrollY / (innerHeight * 0.6))`. A `170vh` spacer div follows in the DOM to create scroll room before the next section.

**Layout**: Full viewport, `flex flex-col`, background `#464340`, with `px-3 py-3` (responsive sm/md padding). Contains a single inner card div that is `flex-1 rounded-2xl relative`.

**Background video** (inside the card, absolutely positioned with `rounded-2xl overflow-hidden`):
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260703_154802_dcffa901-509b-4aa3-8e34-36bbf6edcfcb.mp4
```
`autoPlay muted loop playsInline`, `object-cover h-full w-full`.

**Content overlay** (relative z-10, flex-col justify-between):

- **Top-left**: Large heading: "Circular systems / for a cleaner / planet" (3 lines via `<br>`). Styled: `text-[2.5rem] sm:text-6xl md:text-7xl lg:text-8xl`, `font-normal`, `leading-[1.05]`, `tracking-tighter`, `text-[#E2DBC8]/80`. Wrapped in a `<Reveal delay={200}>` component.

- **Bottom center**: Two elements centered:
  1. Paragraph: "Next-Generation Portable Waste / Reclamation Technology" (line break with `<br>`). `text-sm sm:text-base`, `text-[#E2DBC8]/80`, `max-w-xs`. Wrapped in `<Reveal delay={400}>`.
  2. Link: "Start Here" -- `text-xs uppercase tracking-[0.2em] text-[#E2DBC8]/50` with `hover:text-[#E2DBC8]` transition. Wrapped in `<Reveal delay={500}>`.

- **Bottom-left notch with social icons**: An absolutely positioned element at `bottom-0 left-0`. It contains a `flex items-center gap-4` div with background `#464340`, `px-5 py-3 rounded-tr-[20px]`. Two decorative concave-corner divs use `radial-gradient(circle at 100% 0%, transparent 20px, #464340 20px)` -- one above (top, 20x20) and one to the right (bottom, 20x20). Icons: `Linkedin`, `Phone`, `Mail` from lucide-react, size 18, color `text-[#E2DBC8]/60` with `hover:text-[#E2DBC8]`.

---

### REVEAL COMPONENT (Reusable animation wrapper)

Uses `IntersectionObserver` with `threshold: 0.15`. When visible, transitions from `translate-y-8 opacity-0` to `translate-y-0 opacity-100`. Duration `700ms`, `ease-out`, `will-change-transform`. Accepts optional `delay` (applied as `transitionDelay`), `className`, and `as` ('div' or 'span').

---

### SECTION 2: ABOUT / TEXT REVEAL (Scroll-driven character opacity)

**Background**: `#1C261E`. Layout: `min-h-screen`, content aligned to bottom with `flex items-end`.

**Text content**: A single paragraph with per-character opacity animation driven by scroll. The text reads:

> "Our planet's ecological balance is shifting at an unprecedented pace. Resource recovery obstacles and contamination crises have surpassed critical limits, and the pursuit of transformative green solutions has never carried more weight."

**Animation mechanic** (uses `framer-motion`):
- `useScroll({ target: containerRef, offset: ['start 0.8', 'end 0.2'] })`
- Each character's opacity transitions from `0.3` to `1` based on its position in the text.
- For character at index `i` out of `total`: `start = i/total`, `end = start + 0.005`. UseTransform maps `scrollYProgress` from `[start, end]` to `[0.3, 1]`.
- Words are wrapped in `inline-block whitespace-nowrap` spans to prevent breaks mid-word.
- Visible/invisible span layering: invisible span holds space, absolute motion.span shows the animated character.
- All characters colored `text-[#E2DBC8]`, `font-normal`.
- Typography: `text-2xl sm:text-3xl md:text-5xl lg:text-6xl`, `leading-snug` (lg: `leading-[1.15]`), `tracking-tight`.
- Container padding: `px-5 pb-10 sm:px-8 sm:pb-14 md:px-12 md:pb-16`.

---

### SECTION 3: SCROLL VIDEO + PINNED STAT OVERLAYS

This section has two sub-systems:

### A) Scroll-linked Video Background (Sticky)

**Video URL**:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260703_102446_48ac5215-4f23-49f4-a433-ec9798029150.mp4
```

**Implementation**: Extracts up to 120 frames from the video into `ImageBitmap` objects (scaled to max 1280px width). Renders frames onto a full-screen `<canvas>` driven by scroll progress.

- Container: `sticky top-0 h-screen w-full -z-10 bg-[#0a0a0a]`.
- Falls back to a `<video>` element with seek-based scrubbing if frame extraction fails.
- Scroll progress: calculated from the parent container's scroll position (how far the container's top has moved off-screen relative to its height minus viewport height).
- Smoothing: `smoothed += (targetProgress - smoothed) * 0.1` per animation frame.
- Canvas draws frames using "cover" logic (like `object-cover`).
- Overlay: `absolute inset-0 bg-black/20` tint.
- Top gradient fade: `absolute inset-x-0 top-0 h-40 sm:h-56 md:h-72`, `linear-gradient(to bottom, #1C261E, transparent)` -- blends the previous section's background into the video.

### B) Pinned Statistics Sections (overlaid on the video)

The parent container is `h-[400vh]` (creating 4x viewport of scroll distance). A `sticky top-0 h-screen pointer-events-none z-10` child holds two absolutely-positioned text sections that fade in/out based on scroll progress:

**Section One** ("2.01 billion"):
- Fade in: progress 0 to 0.2
- Stay visible: 0.2 to 0.45
- Fade out: 0.45 to 0.55
- Layout: `flex-col justify-end`, bottom-aligned.
- Heading: "2.01 billion" -- `text-4xl sm:text-5xl md:text-7xl lg:text-8xl`, `font-normal tracking-tighter text-[#E2DBC8]`
- Paragraph: "Tons of household and commercial refuse produced every single year. Lined up in hauling vehicles, this debris would circle the globe **24 times**"
  - `text-sm sm:text-base text-[#E2DBC8]/70 max-w-lg leading-relaxed`
  - "24 times" is in a green badge: `inline-flex items-center rounded bg-[#4caf50] px-2 py-0.5 text-xs sm:text-sm font-semibold text-[#E2DBC8]`
- Both text elements have individual slide-up animation: `translateY(2rem)` when opacity < threshold, `translateY(0)` when visible. `transition-all duration-700 ease-out will-change-transform`.

**Section Two** ("under a fifth"):
- Fade in: progress 0.5 to 0.6
- Stay visible: 0.6 to 0.85
- Fade out: 0.85 to 1.0
- Heading: "under a fifth" -- same typographic treatment as Section One.
- Paragraph: "Of all refuse is reclaimed each year" -- `text-sm sm:text-base text-[#E2DBC8]/70`
- Same slide-up animation pattern with staggered delay of 100ms for the paragraph.

---

### NAVBAR (Fixed, top-right)

Position: `fixed right-4 top-4 z-50` (responsive: `sm:right-8 sm:top-7 md:right-12`). Wrapped in `<Reveal>`.

Contains a link with:
- **Custom SVG logo**: A 24x24 SVG, viewBox `0 0 256 256`, filled `#E2DBC8`. The path is a squircle shape with a leaf/arc cutout: `M 156 0 C 211.228 0 256 44.772 256 100 L 256 256 L 100 256 C 44.772 256 0 211.228 0 156 L 0 0 Z M 80 80 C 80 133.019 122.981 176 176 176 C 176 122.981 133.019 80 80 80 Z`
- **Brand name**: "terova" -- `text-lg font-semibold tracking-tight` in `text-[#E2DBC8]`
- Hover: `opacity-80` transition

---

### COLOR PALETTE

| Token | Hex | Usage |
|-------|-----|-------|
| Page background | `#19261D` | Body, about section |
| Hero card outer bg | `#464340` | Hero section bg, notch bg |
| About section bg | `#1C261E` | Text reveal section |
| Primary text | `#E2DBC8` | Headings, brand, icons |
| Primary text muted | `#E2DBC8/80` | Hero heading, subtext |
| Secondary text | `#E2DBC8/70` | Stat descriptions |
| Tertiary text | `#E2DBC8/50` | CTA link |
| Icon default | `#E2DBC8/60` | Social icons |
| Accent green | `#4caf50` | Badge ("24 times") |
| Dice dots | `#1C261E` | Dots on dice element |
| Dice background | `#E2DBC8` | Dice square |
| Video fallback bg | `#0a0a0a` | Canvas container |
| Overlay | `black/20` | Video darkening |

---

### SCROLL ARCHITECTURE SUMMARY

1. **0px - ~170vh**: Hero is fixed and fades from full opacity to 0
2. **After 170vh spacer**: AboutSection enters viewport (relative, z-10). Character-by-character text reveals on scroll.
3. **After AboutSection**: ScrollVideo container begins. Video is sticky, scrubs with scroll. Pinned stat overlays fade in/out at defined thresholds over 400vh of scroll distance.

---

### KEY TECHNICAL NOTES

- No GSAP or ScrollTrigger -- all scroll logic is vanilla JS `window.addEventListener('scroll')` + `requestAnimationFrame` smoothing, except the About section which uses Framer Motion's `useScroll`/`useTransform`.
- Video frame extraction uses `createImageBitmap` from a seeked `<video>` element for smooth canvas-based playback.
- The canvas uses `devicePixelRatio` (capped at 2) for sharp rendering.
- All responsive breakpoints use Tailwind's default `sm:640px`, `md:768px`, `lg:1024px`.
- The page is fully static/client-side -- no backend, no routing.

## Wanderful Hero — Travel [sites/wanderful-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(78).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/wanderful-hero.webp

Build a full-viewport cinematic hero section for a travel brand called "Wanderful" using React + TypeScript + Vite + Tailwind CSS. Use GSAP for animation and `lucide-react` for icons.

**Fonts (load via Google Fonts in `src/index.css`):**
```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&family=Inter:wght@300;400;500;600;700&display=swap');
```
Also load a custom display font:
```css
@font-face {
  font-family: 'Dirtyline';
  src: url('https://fonts.cdnfonts.com/s/15011/Dirtyline36DaysofType.woff') format('woff');
  font-weight: normal; font-style: normal; font-display: swap;
}
```
Body font: `Barlow`. Hero headings: `Inter`. Body background: `#000`.

**Video background (fixed, full screen, z-0):**
- Use this exact CloudFront URL as the `<video>` src:
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260510_060007_60275ce7-030c-4668-a160-8f364ec537d3.mp4`
- Attributes: `autoPlay muted loop playsInline`, `object-cover`, wrapper scaled `scale-[1.08]` with `origin-center`.
- On `onLoadedMetadata`, set `playbackRate = 1.25`.
- Add GSAP-driven mouse parallax: listen to `mousemove`, compute `targetX/Y = ((clientX - cx)/cx) * 20`, lerp `currentX/Y += (target - current) * 0.06` inside `requestAnimationFrame`, and apply via `gsap.set(videoBg, { x, y })`.

**Liquid-glass utility (add to `index.css`):**
```css
.liquid-glass {
  background: rgba(255,255,255,0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  border: none;
  box-shadow: inset 0 1px 1px rgba(255,255,255,0.1);
  position: relative;
  overflow: hidden;
}
.liquid-glass::before {
  content: "";
  position: absolute; inset: 0;
  border-radius: inherit;
  padding: 1.4px;
  background: linear-gradient(180deg,
    rgba(255,255,255,0.45) 0%,
    rgba(255,255,255,0.15) 20%,
    rgba(255,255,255,0) 40%,
    rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.15) 80%,
    rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}
```

**Header (fixed top, z-50, `px-10 py-8`, flex justify-between items-center):**
- Left: wordmark `Wanderful` followed by `<sup>TM</sup>`, `text-[17px] font-semibold tracking-tight`.
- Center: `<nav>` using `.liquid-glass rounded-full px-2 py-2 flex items-center gap-1`. Links: `JOURNEY`, `BENEFITS`, `JOURNAL`, `GUIDEBOOK`. Each link: `text-[11px] font-medium tracking-[0.12em] text-white/90 hover:text-white px-4 py-1.5 rounded-full transition-colors duration-200`.
- Right: "GET ROAMING" anchor with same `.liquid-glass rounded-full px-5 py-2.5 text-[11px] font-medium tracking-[0.12em] text-white/90 hover:text-white`.

**Hero headline (fixed, `top: 120px`, centered, z-20):**
Two lines, both centered, `Inter` 400, `font-size: clamp(40px, 5.4vw, 72px)`, `line-height: 1.1`, `letter-spacing: -0.02em`:
- Line 1 (white): `Venture without edges.`
- Line 2 (`rgba(255,255,255,0.55)`): `Uncover with keen instinct.`

Fade-in on mount: `opacity 0 → 100` and `translate-y-6 → 0` with `transition-all duration-1000`.

**Bottom block (fixed `bottom-14`, z-20, flex-col items-center gap-6), fade-in with `delay-300`:**
1. Paragraph, `max-w-[620px] text-[15px] leading-relaxed` centered:
   - White: "Our smart itineraries shape around you — your rhythm, your vibe, your hunger for adventure."
   - `text-white/55`: " Each getaway is tailored, seamless, and wholly yours."
2. Button: white bg, black text, `text-[15px] font-medium rounded-full px-8 py-3.5`, hover `scale-[1.03]` + `shadow-[0_0_32px_4px_rgba(255,255,255,0.2)]`, active `scale-[0.97]`. Label: `Plan my escape today`.
3. Row: `Lock` icon from lucide-react (`size={13} strokeWidth={1.5}`) + `text-[11px] font-medium tracking-[0.14em] text-white/70`, text: `SECURE BY DESIGN. ZERO DATA LEAKS.`

**Root container:** `min-h-screen bg-black text-white overflow-x-hidden` with inline `fontFamily: "'Inter', sans-serif"`.

Dependencies: `gsap`, `lucide-react`, `react`, `react-dom`, tailwind configured with content globs `./index.html` and `./src/**/*.{js,ts,jsx,tsx}`.

## Web3 EOS Hero — Web3 [sites/9]

- Preview: https://motionsites.ai/assets/hero-web3-eos-poster-DF0_WdVS.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/9.png

Build a full-screen hero section for a Web3 landing page. Use the font "General Sans" (from Fontshare) throughout. The entire section has a pure black (#000000) background with a fullscreen looping background video (muted, autoplay, playsInline) using this URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260217_030345_246c0224-10a4-422c-b324-070b7c0eceda.mp4. The video is covered by a 50% black overlay (bg-black/50) for readability. All content sits on top of the video.

Navbar:

Horizontally spread across the top with 120px horizontal padding and 20px vertical padding.

Left side: a placeholder logo wordmark (use "LOGOIPSUM" or similar) in white, 187px wide and 25px tall, followed by 4 nav links spaced 30px apart: "Get Started", "Developers", "Features", "Resources". Each nav link is white, 14px, font-medium, with a small white 14px chevron-down arrow icon to the right (14px gap between label and arrow). Nav links are hidden on mobile.

Right side: a "Join Waitlist" pill button. This button has a subtle layered construction — a fully rounded pill shape with a thin 0.6px solid white outer border, and inside that, a black-background pill with the text "Join Waitlist" in white, 14px, font-medium, centered with 29px horizontal and 11px vertical padding. There's also a subtle white glow/light streak effect along the top edge of the button (a blurred white-to-transparent gradient blob positioned at the top).

Hero Content (centered below the navbar):

Vertically centered in the remaining viewport space, pushed down with about 280px top padding on desktop (200px on mobile), 102px bottom padding.

All content is horizontally centered and stacked vertically with 40px gaps.

Badge/pill: A small rounded pill (20px border-radius) with 10% white background and a 1px white/20% border. Inside: a tiny 4px white dot, then text reading "Early access available from" in white at 60% opacity, followed by " May 1, 2026" in solid white. Font is 13px, font-medium.

Heading: Large text reading "Web3 at the Speed of Experience", max-width 613px, 56px on desktop / 36px on mobile, font-medium, line-height 1.28. The text has a gradient fill — a linear-gradient at ~144.5 degrees going from solid white (at ~28%) to fully transparent black (at ~115%), applied as a background-clip text effect so the text itself shows the gradient.

Subtitle: Below the heading with a 24px gap. Text reads: "Powering seamless experiences and real-time connections, EOS is the base for creators who move with purpose, leveraging resilience, speed, and scale to shape the future." — 15px, font-normal, white at 70% opacity, max-width 680px, centered.

CTA Button: A "Join Waitlist" pill button similar to the navbar button but with a white background and black text instead. Same layered construction: 0.6px white outer border, white glow streak on top, and inside the white pill the text is 14px font-medium black, with 29px horizontal and 11px vertical padding.

The entire layout is responsive — nav links collapse on screens below md breakpoint, heading scales down, and padding adjusts.

## Orbit Web3 — Web3 [sites/orbit-web3-hero]

- Preview: https://motionsites.ai/assets/hero-orbit-web3-preview-BXt4OttD.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbit-web3-hero.gif

Create a dark, cinematic hero landing page with these exact specifications:

Font: Google Font Instrument Serif (serif), loaded via <link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap">. Used as the base font for the entire page (font-family: 'Instrument Serif', serif).

Color palette (HSL, CSS variables):
--background: 150 20% 5% (deep dark green-black)
--foreground: 45 30% 90% (warm off-white)
--accent: 45 70% 75% (warm amber/gold for button)
--accent-foreground: 150 20% 5% (dark text on accent)

Background video: Full-screen, absolutely positioned behind all content. Muted, autoplay, loop, playsInline.
URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260325_094440_a3592600-bd1e-49e5-9bce-a73662061d83.mp4
object-cover, fills entire viewport.

Navbar: Horizontal, top-left. Contains an SVG "W" logo (stroke-based zigzag path: M8 10L14 30L20 16L26 30L32 10, strokeWidth 3, rounded caps/joins, 40x40). Nav links: "Vault", "Send", "Receive", "Trade" — text-base tracking-wide, foreground color, hover to 80% opacity. Padding: px-8 py-6 md:px-16, gap-8 between logo and links, gap-6 between links.

Hero heading: Vertically centered in viewport (flex-1 flex flex-col justify-between, content wrapper with my-auto). Max-width max-w-3xl. Font sizes: text-6xl md:text-8xl lg:text-[7rem], leading-[0.95] tracking-tight, white text (text-foreground). Copy:
Own the future of
your assets.

The word "assets." has a neon glow effect: two absolutely-positioned duplicate <span>s layered on top, both white (hsl(0 0% 100%)), with gradient masks (linear-gradient(to bottom left, white 20%/25%, transparent 50%/55%)) creating a directional top-right glow. First span: blur-sm, second: blur-md opacity-60. The parent span has overflow-visible to allow glow to bleed out. The outer heading container and page root also use overflow-visible. Bottom margin on heading: mb-12.

CTA Button (GlowButton): px-10 py-4 rounded-[43px], background accent, text accent-foreground, text-xl. Box shadow: 0px 4px 95px 4px hsl(45 70% 50% / 0.6) (large amber glow). Contains an internal blurred blob: a w-48 h-10 rounded-full span, blur-xl, color hsl(45 60% 95%), positioned at top: -12px, centered horizontally, clipped by overflow-hidden on the button. Hover: scale-105 transition. Text: "Launch your orbit".

Logo marquee: Pinned to bottom of hero (mt-auto). Width: full on mobile, md:w-1/2 lg:w-1/2. Label: "Trusted by top builders" — text-foreground/50 text-base mb-5 text-left. Five logos using Lucide icons (Sun, Box, Star, Feather, Sparkles) with names: Nebulon, Prismify, Nova Labs, Zephyr, Ignite. Each: flex items-center gap-3 mx-6, icon w-6 h-6 text-foreground/60, name text-foreground/60 text-2xl tracking-wide whitespace-nowrap. Infinite horizontal scroll via CSS @keyframes marquee { 0% { translateX(0) } 100% { translateX(-50%) } }, animation: marquee 20s linear infinite. Logos rendered twice for seamless loop.

Layout structure: Root div min-h-screen bg-background flex flex-col relative overflow-visible. Video is z-0. Content wrapper is relative z-10 flex flex-col min-h-screen. Main area: flex-1 flex flex-col justify-between px-8 md:px-16 pb-10.

## Celestial Renewal — Wellness [sites/celestial-renewal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/planetscrollArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/celestial-renewal.mp4

**Build a React + Vite + Tailwind CSS landing page with two full-screen sections for a luxury beauty/wellness brand called "Serene". Use TypeScript.**

---

### Fonts (loaded via Google Fonts in index.html)

Load these three font families from Google Fonts:
- **Dancing Script** (weights: 400, 500, 600, 700) -- used for the brand logo
- **Instrument Serif** (italic: 0, 1) -- used for the hero heading and the quote text
- **Inter** (weights: 300, 400, 500, 600, 700, 800, 900) -- used for body text, navbar links, and buttons

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Dancing+Script:wght@400;500;600;700&family=Instrument+Serif:ital@0;1&family=Inter:wght@300;400;500;600;700;800;900&display=swap" rel="stylesheet" />
```

---

### Global CSS (index.css)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: 'Inter', sans-serif;
  background: #0a0a0c;
  overflow-x: hidden;
}

.font-inter {
  font-family: 'Inter', sans-serif;
}

.font-instrument {
  font-family: 'Instrument Serif', serif;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
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

.text-glow {
  text-shadow: 0 0 40px rgba(255, 255, 255, 0.4), 0 0 80px rgba(255, 255, 255, 0.2), 0 0 120px rgba(255, 255, 255, 0.1);
}

.button-glow {
  box-shadow: 0 0 20px rgba(255, 255, 255, 0.3), 0 0 40px rgba(255, 255, 255, 0.1);
}
```

---

### App Layout (App.tsx)

The wrapper div has `bg-[#0a0608]`. It renders `<Hero />` followed by `<QuoteSection />`.

---

### SECTION 1: Hero

A full-viewport (`h-screen`) section with:

1. **Background video** -- autoplays, muted, loops, playsInline, covers the full section with `object-cover`:
   ```
   https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260613_180732_a54afbf6-b30d-470e-861f-669871f09f67.mp4
   ```

2. **Dark overlay** -- `absolute inset-0 bg-black/20`

3. **Fixed Navbar** -- `fixed top-0 left-0 right-0 z-50`, flex row, space-between, `px-6 md:px-12 py-5`:
   - **Left**: Brand name "Serene" in Dancing Script cursive, white, `text-2xl md:text-3xl`
   - **Center (desktop only, hidden on mobile)**: Navigation links -- "About", "Services", "Journal", "Contact" -- `text-white/80 hover:text-white text-sm tracking-wide`, spaced `gap-12`
   - **Right (desktop)**: White pill button "Book a consultation"
   - **Right (mobile)**: Hamburger icon (3 lines, animated to X on open). Uses cubic-bezier(0.22,1,0.36,1) easing. On open: top line rotates 45deg + translates down 9px; middle line fades/scales to 0; bottom line rotates -45deg + translates up 9px.
   - **Mobile menu**: Slide-in panel from right, `w-[85%] max-w-[340px]`, `bg-[#0a0608]/95 backdrop-blur-xl border-l border-white/10`. Links stagger-animate in (opacity + translateX, 75ms delay between each, starting at 150ms). Button at bottom with 450ms delay.

4. **Center content** -- absolutely positioned, flex column, centered, with `-mt-[120px]` to shift up:
   - **Heading**: `font-instrument text-white text-[36px] md:text-7xl lg:text-[110px] leading-[0.9] tracking-tight text-center text-glow` -- text: "Gentle touch. Radiant presence."
   - **Subtext**: `text-white/70 text-sm md:text-base text-center mt-5 md:mt-7 max-w-xl` -- text: "Expert beauty and holistic wellness, delivered with warmth and intention."
   - **CTA Button**: White pill button "Begin your renewal", `mt-6 md:mt-9`

5. **Sound indicator (desktop only)** -- bottom-left corner (`bottom-8 left-8`), a 40px circle with `border border-white/20` containing a small horizontal bar, next to two lines of text: "Experience" / "with sound" in `text-white/60 text-xs`

**Button component**: `bg-white text-black px-8 py-3.5 rounded-full font-medium text-sm tracking-wide hover:bg-white/90 transition-all duration-300 button-glow`

---

### SECTION 2: Quote Section (with parallax scroll animations)

A full-viewport (`h-screen`) section with:

**Background**: CSS linear-gradient top to bottom:
```
#010A17 0% -> #0A4267 30% -> #20658E 60% -> #6BADC4 100%
```

**Animated layers (requestAnimationFrame-based parallax with lerp smoothing):**

The animation uses a `progress` value (0 to 1) based on how far the section has scrolled through the viewport:
```
progress = clamp(0, 1, (windowHeight - rect.top) / (windowHeight + rect.height))
```

1. **Rainbow image** -- full-width, positioned `absolute inset-x-0 top-0 z-30`. Parallax: moves vertically from +120px to -160px based on scroll progress. Lerp factor: 0.06.
   ```
   https://soft-zoom-63098134.figma.site/_assets/v11/8d520a7515d06cbfc403d0125e3d05b1a7ccd29c.png
   ```

2. **Left cloud** -- `absolute left-0 bottom-[10%] z-10`, hidden on mobile (`hidden sm:block`). Width: `w-[500px] md:w-[650px]`. Has `marginLeft: '-50%'` to let it overflow left. Slides in from -200px on X when in view (progress 0.12-0.92), slides back out when not. Also drifts up (cloudY = progress * -50). Opacity tied to X distance. Lerp factor: 0.04.
   ```
   https://soft-zoom-63098134.figma.site/_assets/v11/0d6dfd3f90b930f21726f2ed56a3320d79b7a797.png
   ```

3. **Right cloud** -- same image as left but `scale-x-[-1]` (flipped), `absolute right-0 bottom-[15%] z-10`. Has `marginRight: '-75%'`. Slides in from +200px. Same lerp/timing as left cloud.

4. **Quote content** -- centered, `z-20`, `max-w-4xl`:
   - **Quote text**: `font-instrument text-white text-xl sm:text-2xl md:text-4xl lg:text-[42px] leading-[1.45] md:leading-[1.5]` -- text: "Serene was founded on a belief in beauty that honors your nature. We pursue refined outcomes, considered approaches, and lasting vitality. We spend time learning what matters to you before deciding what serves you best. No rushing, no excess -- just support that lets you feel radiant." (wrapped in curly quotes)
   - **Attribution**: `mt-6 md:mt-8 text-white/80 text-sm md:text-base tracking-wide` -- text: "Dr. Mia Callahan -- Founder"

**Key animation implementation detail**: All transforms use `translate3d` for GPU acceleration with `will-change-transform`. Initial cloud state is `opacity: 0` and translated off-screen. The lerp function smoothly interpolates current values toward targets each frame: `current + (target - current) * factor`.

---

### Tailwind Config

Default Tailwind config with no extensions -- all custom styling handled via CSS utility classes in index.css.
