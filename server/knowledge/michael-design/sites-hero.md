# Michael Design Library — sites-hero

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 91 entries.

## AI Workflow Hero — Hero [sites/ai-workflow]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(55).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-workflow.webp

### Stack

- **Vite** + **React 18** + **TypeScript**
- **Tailwind CSS 3.4**
- **lucide-react** for icons (`LogIn`, `UserPlus`, `Play`, `Sparkles`, `Menu`, `X`)
- No Framer Motion -- all animations are CSS `transition-*` classes

---

### Fonts (loaded in `index.html`)

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
<link href="https://db.onlinewebfonts.com/c/6e47ef470dd19698c911332a9b4d1cf4?family=Neue+Haas+Grotesk+Text+Pro" rel="stylesheet" />
<link href="https://db.onlinewebfonts.com/c/dec0d9b4e22ca588dc20e1e2e09a59b5?family=Neue+Haas+Grotesk+Display+Pro+55+Roman" rel="stylesheet" />
```

Body/root font stack (in `index.css`):

```css
html, body, #root {
  height: 100%;
  margin: 0;
  font-family: 'Neue Haas Grotesk Display Pro 55 Roman', 'Neue Haas Grotesk Text Pro', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
}
```

---

### Video URL (CloudFront)

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_131941_d136af49-e243-493a-be14-6ff3f24e09e6.mp4
```

---

### Color Palette

| Token | Hex |
|-------|-----|
| Dark green (text, buttons) | `#1f2a1d` |
| Medium dark green | `#2d3a2a` |
| Button hover | `#2a3827` |
| Body text green | `#4b5b47` |
| Heading primary | `#336443` |
| Heading accent | `#85AB8B` |
| Bottom-left text | `#3d5638` |
| Bottom-left button bg | `#3d5638`, hover `#2d4228` |

---

### Architecture

Two files:

1. **`BoomerangVideoBg.tsx`** -- captures video frames into canvas, then plays them forward/backward in a seamless boomerang loop at 30fps (960px max capture width).
2. **`App.tsx`** -- the full hero section.

---

### `BoomerangVideoBg.tsx` (exact)

```tsx
import { useEffect, useRef, useState } from 'react';

type Props = {
  src: string;
  className?: string;
};

export default function BoomerangVideoBg({ src, className }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const displayCanvasRef = useRef<HTMLCanvasElement>(null);
  const [framesReady, setFramesReady] = useState(false);
  const framesRef = useRef<HTMLCanvasElement[]>([]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const frames: HTMLCanvasElement[] = [];
    let capturing = true;
    let lastTime = -1;
    const MAX_WIDTH = 960;

    const captureFrame = () => {
      if (!capturing || video.readyState < 2) return;
      if (video.currentTime === lastTime) return;
      lastTime = video.currentTime;

      const vw = video.videoWidth;
      const vh = video.videoHeight;
      if (!vw || !vh) return;

      const scale = Math.min(1, MAX_WIDTH / vw);
      const w = Math.round(vw * scale);
      const h = Math.round(vh * scale);

      const canvas = document.createElement('canvas');
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.drawImage(video, 0, 0, w, h);
      frames.push(canvas);
    };

    type VFCVideo = HTMLVideoElement & {
      requestVideoFrameCallback?: (cb: () => void) => number;
    };
    const vfcVideo = video as VFCVideo;
    const hasVFC = typeof vfcVideo.requestVideoFrameCallback === 'function';

    let rafId = 0;
    const rafLoop = () => {
      captureFrame();
      if (capturing) rafId = requestAnimationFrame(rafLoop);
    };

    const vfcLoop = () => {
      captureFrame();
      if (capturing && vfcVideo.requestVideoFrameCallback) {
        vfcVideo.requestVideoFrameCallback(vfcLoop);
      }
    };

    const onEnded = () => {
      capturing = false;
      if (frames.length > 0) {
        framesRef.current = frames;
        setFramesReady(true);
      }
    };

    const onLoaded = () => {
      video.play().catch(() => {});
      if (hasVFC) {
        vfcVideo.requestVideoFrameCallback!(vfcLoop);
      } else {
        rafId = requestAnimationFrame(rafLoop);
      }
    };

    video.addEventListener('loadedmetadata', onLoaded);
    video.addEventListener('ended', onEnded);
    if (video.readyState >= 1) onLoaded();

    return () => {
      capturing = false;
      cancelAnimationFrame(rafId);
      video.removeEventListener('loadedmetadata', onLoaded);
      video.removeEventListener('ended', onEnded);
    };
  }, [src]);

  useEffect(() => {
    if (!framesReady) return;
    const canvas = displayCanvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const frames = framesRef.current;
    if (frames.length === 0) return;

    const first = frames[0];
    canvas.width = first.width;
    canvas.height = first.height;

    let index = 0;
    let direction = 1;
    let last = performance.now();
    const interval = 1000 / 30;
    let rafId = 0;

    const render = (now: number) => {
      if (now - last >= interval) {
        last = now;
        ctx.drawImage(frames[index], 0, 0);
        index += direction;
        if (index >= frames.length - 1) {
          index = frames.length - 1;
          direction = -1;
        } else if (index <= 0) {
          index = 0;
          direction = 1;
        }
      }
      rafId = requestAnimationFrame(render);
    };
    rafId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(rafId);
  }, [framesReady]);

  return (
    <div className={className ?? 'absolute inset-0 w-full h-full'}>
      <video
        ref={videoRef}
        src={src}
        className="w-full h-full object-cover"
        style={{ display: framesReady ? 'none' : 'block' }}
        muted
        playsInline
        preload="auto"
        crossOrigin="anonymous"
      />
      <canvas
        ref={displayCanvasRef}
        className="w-full h-full object-cover"
        style={{ display: framesReady ? 'block' : 'none' }}
      />
    </div>
  );
}
```

---

### `App.tsx` (exact)

```tsx
import { useState, useEffect } from 'react';
import { LogIn, UserPlus, Play, Sparkles, Menu, X } from 'lucide-react';
import BoomerangVideoBg from './BoomerangVideoBg';

const BG_VIDEO =
  'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_131941_d136af49-e243-493a-be14-6ff3f24e09e6.mp4';

function App() {
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (menuOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [menuOpen]);

  const navLinks = [
    { href: '#mission', label: 'Purpose' },
    { href: '#how', label: 'The Process' },
    { href: '#pricing', label: 'Tariffs' },
  ];

  return (
    <section className="relative w-full min-h-screen sm:h-screen overflow-hidden">
      <BoomerangVideoBg src={BG_VIDEO} className="absolute inset-0 w-full h-full" />
      <nav className="absolute top-0 left-0 right-0 z-30 flex items-center justify-between px-4 sm:px-6 md:px-10 py-4 sm:py-6">
        <div className="flex items-center gap-2 text-[#2d3a2a]">
          <span className="text-lg sm:text-xl md:text-2xl font-semibold tracking-tight">
            LinkFlow<sup className="text-[10px] sm:text-xs font-medium">TM</sup>
          </span>
        </div>

        <div className="hidden lg:flex items-center gap-1 bg-white/70 backdrop-blur-md rounded-full pl-6 pr-1 py-1 shadow-sm border border-white/60">
          {navLinks.map((link, i) => (
            <a
              key={link.href}
              href={link.href}
              className={`text-sm px-3 py-2 transition-colors ${
                i === 0 ? 'font-semibold text-[#1f2a1d]' : 'font-medium text-[#4b5b47] hover:text-[#1f2a1d]'
              }`}
            >
              {link.label}
            </a>
          ))}
          <button className="ml-2 bg-[#1f2a1d] hover:bg-[#2a3827] text-white text-sm font-medium px-5 py-2.5 rounded-full transition-colors">
            Try it Live
          </button>
        </div>

        <div className="flex items-center gap-3 sm:gap-6 text-[#2d3a2a]">
          <a href="#signup" className="hidden sm:flex items-center gap-2 text-sm font-medium hover:opacity-80 transition-opacity">
            <UserPlus className="w-4 h-4" />
            Sign Me Up!
          </a>
          <a href="#login" className="hidden sm:flex items-center gap-2 text-sm font-medium hover:opacity-80 transition-opacity">
            <LogIn className="w-4 h-4" />
            Enter
          </a>
          <button
            onClick={() => setMenuOpen((v) => !v)}
            className="lg:hidden relative flex items-center justify-center w-10 h-10 rounded-full bg-white/70 backdrop-blur-md border border-white/60 text-[#1f2a1d] transition-all duration-300 hover:bg-white/90"
            aria-label={menuOpen ? 'Close menu' : 'Open menu'}
            aria-expanded={menuOpen}
          >
            <Menu
              className={`w-5 h-5 absolute transition-all duration-300 ${
                menuOpen ? 'opacity-0 rotate-90 scale-50' : 'opacity-100 rotate-0 scale-100'
              }`}
            />
            <X
              className={`w-5 h-5 absolute transition-all duration-300 ${
                menuOpen ? 'opacity-100 rotate-0 scale-100' : 'opacity-0 -rotate-90 scale-50'
              }`}
            />
          </button>
        </div>
      </nav>

      {/* Mobile menu overlay */}
      <div
        className={`lg:hidden fixed inset-0 z-20 transition-opacity duration-300 ${
          menuOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
        }`}
        onClick={() => setMenuOpen(false)}
      >
        <div className="absolute inset-0 bg-[#1f2a1d]/40 backdrop-blur-sm" />
      </div>

      {/* Mobile menu drawer */}
      <div
        className={`lg:hidden fixed top-0 right-0 bottom-0 z-20 w-[85%] max-w-sm bg-white/95 backdrop-blur-xl shadow-2xl transition-transform duration-500 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          menuOpen ? 'translate-x-0' : 'translate-x-full'
        }`}
      >
        <div className="flex flex-col h-full pt-24 px-8 pb-8">
          <div className="flex flex-col gap-1">
            {navLinks.map((link, i) => (
              <a
                key={link.href}
                href={link.href}
                onClick={() => setMenuOpen(false)}
                className={`text-2xl font-semibold text-[#1f2a1d] py-4 border-b border-[#1f2a1d]/10 transition-all duration-500 ${
                  menuOpen ? 'translate-x-0 opacity-100' : 'translate-x-8 opacity-0'
                }`}
                style={{ transitionDelay: menuOpen ? `${150 + i * 70}ms` : '0ms' }}
              >
                {link.label}
              </a>
            ))}
          </div>

          <div
            className={`mt-8 flex flex-col gap-4 transition-all duration-500 ${
              menuOpen ? 'translate-x-0 opacity-100' : 'translate-x-8 opacity-0'
            }`}
            style={{ transitionDelay: menuOpen ? '400ms' : '0ms' }}
          >
            <a href="#signup" className="flex items-center gap-2 text-sm font-medium text-[#2d3a2a] sm:hidden">
              <UserPlus className="w-4 h-4" />
              Sign Me Up!
            </a>
            <a href="#login" className="flex items-center gap-2 text-sm font-medium text-[#2d3a2a] sm:hidden">
              <LogIn className="w-4 h-4" />
              Enter
            </a>
            <button className="mt-2 bg-[#1f2a1d] hover:bg-[#2a3827] text-white text-sm font-semibold px-5 py-3 rounded-full transition-colors">
              Try it Live
            </button>
          </div>
        </div>
      </div>

      {/* Hero copy */}
      <div className="relative z-10 flex flex-col items-center text-center pt-24 sm:pt-28 md:pt-32 px-4 sm:px-6">
        <h1
          className="font-normal leading-[0.95] text-[#336443] text-[2rem] sm:text-4xl md:text-5xl lg:text-[4.75rem] xl:text-[5.25rem] max-w-5xl"
          style={{ fontFamily: '"Neue Haas Grotesk Display Pro 55 Roman", "Neue Haas Grotesk Text Pro", "Helvetica Neue", Helvetica, Arial, sans-serif', letterSpacing: '-0.035em' }}
        >
          Close the rift{' '}
          <span className="text-[#85AB8B]">
            linking
            <br className="hidden sm:block" /> signals and action
          </span>
        </h1>
        <p className="mt-6 sm:mt-8 text-[#4b5b47] text-sm sm:text-base md:text-lg leading-relaxed max-w-md px-2">
          Shape scattered signals into meaningful outcomes via AI-driven workflows.
        </p>
      </div>

      {/* Bottom-left CTA block */}
      <div className="absolute left-4 right-4 sm:right-auto sm:left-6 md:left-10 bottom-6 sm:bottom-8 md:bottom-10 z-10 max-w-sm">
        <div className="flex items-center gap-2 text-[#3d5638] sm:text-white/95 mb-3">
          <Sparkles className="w-4 h-4" />
          <span className="text-sm font-semibold sm:font-medium">
            FluxEngine<sup className="text-[10px]">TM</sup>
          </span>
        </div>
        <p className="text-[#3d5638]/90 sm:text-white/85 text-xs leading-relaxed mb-6 max-w-xs font-medium sm:font-normal">
          LinkFlow smoothly unites your company systems, streamlining data paths between services without having to write custom scripts.
        </p>
        <div className="flex items-center gap-4 flex-wrap">
          <button className="bg-[#3d5638] sm:bg-white hover:bg-[#2d4228] sm:hover:bg-white/90 text-white sm:text-[#1f2a1d] text-sm font-semibold px-5 sm:px-6 py-2.5 sm:py-3 rounded-full transition-colors shadow-sm">
            Try it Live
          </button>
          <button className="text-[#3d5638] sm:text-white text-sm font-semibold sm:font-medium hover:opacity-80 transition-opacity">
            Know More.
          </button>
        </div>
      </div>

      {/* Bottom-right video link */}
      <div className="hidden sm:flex absolute right-6 md:right-10 bottom-8 md:bottom-10 z-10 items-center gap-2 text-white/90 text-sm">
        <button className="flex items-center justify-center w-6 h-6 rounded-full bg-white/20 backdrop-blur-sm hover:bg-white/30 transition-colors">
          <Play className="w-3 h-3 fill-white text-white ml-0.5" />
        </button>
        <span className="font-medium">How we build?</span>
        <span className="text-white/60">1:35</span>
      </div>
    </section>
  );
}

export default App;
```

---

### Animation Details (all CSS, no Framer Motion)

| Element | Property | Values |
|---------|----------|--------|
| Hamburger Menu/X icon swap | `transition-all duration-300` | Open: Menu gets `opacity-0 rotate-90 scale-50`, X gets `opacity-100 rotate-0 scale-100`. Closed: reverse. |
| Mobile overlay backdrop | `transition-opacity duration-300` | Open: `opacity-100 pointer-events-auto`. Closed: `opacity-0 pointer-events-none`. |
| Mobile drawer slide | `transition-transform duration-500 ease-[cubic-bezier(0.22,1,0.36,1)]` | Open: `translate-x-0`. Closed: `translate-x-full`. |
| Mobile nav links stagger | `transition-all duration-500` | Open: `translate-x-0 opacity-100`, delay per item: `150ms + i * 70ms`. Closed: `translate-x-8 opacity-0`, delay `0ms`. |
| Mobile CTA group | `transition-all duration-500` | Open: `translate-x-0 opacity-100`, delay `400ms`. Closed: `translate-x-8 opacity-0`, delay `0ms`. |
| Nav buttons | `transition-colors` | Default Tailwind duration (150ms). |
| Opacity links | `transition-opacity` | `hover:opacity-80`. |

---

### Key Layout/Spacing Notes

- Root section: `relative w-full min-h-screen sm:h-screen overflow-hidden`
- Navbar padding: `px-4 sm:px-6 md:px-10 py-4 sm:py-6`
- Desktop pill nav: `bg-white/70 backdrop-blur-md rounded-full pl-6 pr-1 py-1 shadow-sm border border-white/60`
- Hero heading: `pt-24 sm:pt-28 md:pt-32`, font sizes `text-[2rem] sm:text-4xl md:text-5xl lg:text-[4.75rem] xl:text-[5.25rem]`, `leading-[0.95]`, `letterSpacing: '-0.035em'`
- Bottom-left block: `absolute left-4 right-4 sm:right-auto sm:left-6 md:left-10 bottom-6 sm:bottom-8 md:bottom-10`
- Bottom-right video: `absolute right-6 md:right-10 bottom-8 md:bottom-10`

---

### Dependencies (package.json)

```json
{
  "dependencies": {
    "lucide-react": "^0.344.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.1",
    "autoprefixer": "^10.4.18",
    "postcss": "^8.4.35",
    "tailwindcss": "^3.4.1",
    "typescript": "^5.5.3",
    "vite": "^5.4.2"
  }
}
```

## Audio Showcase — Hero [sites/audio-showcase]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(70).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/audio-showcase.webp

Build a full-screen hero section for a fictional vinyl record label called **"quietpress"** using React, TypeScript, Tailwind CSS, and Vite. The page is a single viewport-height hero with no scrolling. Use **lucide-react** for icons. No other UI libraries.

---

### Font

Load **Helvetica Regular** via this stylesheet in `index.html`:
```
https://db.onlinewebfonts.com/c/a64ff11d2c24584c767f6257e880dc65?family=Helvetica+Regular
```
Set the base font in CSS:
```css
html { font-family: 'Helvetica Regular', Helvetica, Arial, sans-serif; }
```

---

### Background: Boomerang Video Loop

Use this CloudFront video as the background:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260611_183632_c311af08-e4b7-458f-81e7-79847a49b3d3.mp4
```

Create a `BoomerangVideoBg` component that:
1. Plays the video once (muted, playsInline, crossOrigin="anonymous"), capturing every frame into off-screen canvases (max width 960px, scaled proportionally).
2. Uses `requestVideoFrameCallback` when available, falling back to `requestAnimationFrame`.
3. When the video ends, hides the `<video>` element and renders a `<canvas>` that plays the captured frames in a ping-pong (boomerang) loop at 30fps -- forward then backward, endlessly.
4. The container is `absolute inset-0 z-0` with `scale-[1.08] origin-center overflow-hidden` to slightly zoom the video and hide edges.

---

### Liquid Glass CSS Effect

Create a reusable `.liquid-glass` CSS class:
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

### Fade-Up Entrance Animation

```css
@keyframes fade-up {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: none; }
}
.animate-fade-up {
  animation: fade-up 0.7s cubic-bezier(0.22, 1, 0.36, 1) backwards;
}
.delay-1 { animation-delay: 0.1s; }
.delay-2 { animation-delay: 0.25s; }
.delay-3 { animation-delay: 0.4s; }
.delay-4 { animation-delay: 0.55s; }
.delay-5 { animation-delay: 0.75s; }
@media (prefers-reduced-motion: reduce) {
  .animate-fade-up { animation: none; }
}
```

**CRITICAL:** Use `animation-fill-mode: backwards` (not `both` or `forwards`). Using `both` or `forwards` leaves a `transform` on the element after the animation ends, which breaks `backdrop-filter` on any child using `.liquid-glass`. `backwards` applies the "from" state before the animation starts but fully releases all properties when it finishes, so the glass blur works correctly.

---

### Header (absolute, top, z-20)

- **Logo (left):** A custom SVG icon (a quarter-circle shape with a centered dot, white fill, 20x20px) next to the text "quietpress" in `text-base tracking-tight text-white`.
  - SVG path: `M 256 256 L 128 256 C 198.692 256 256 198.692 256 128 C 256 57.308 198.692 0 128 0 C 57.308 0 0 57.308 0 128 C 0 198.692 57.308 256 128 256 L 0 256 L 0 0 L 256 0 Z M 128 104 C 141.255 104 152 114.745 152 128 C 152 141.255 141.255 152 128 152 C 114.745 152 104 141.255 104 128 C 104 114.745 114.745 104 128 104 Z` (viewBox `0 0 256 256`)

- **Nav links (center, hidden on mobile):** "Anthology", "Talents", "Sound diary", "Playback salon" -- `text-sm text-white/90 hover:text-white`, gap-8.

- **Right side:**
  - **Cart button:** White pill shape (`rounded-xl bg-white p-1 pr-3 sm:pr-4`). Contains a blue-700 icon square (`h-7 w-7 rounded-lg bg-blue-700`) with a `ShoppingCart` icon (size 14, strokeWidth 2), then text "Cart (0)" (hidden on mobile, showing just "(0)" on small screens). Has `hover:scale-105 active:scale-95`.
  - **Mobile menu toggle:** `liquid-glass` square button (`h-9 w-9 rounded-xl`), shows `Menu` or `X` icon (size 18). Hidden on `md:` and above.

- **Mobile nav dropdown** (shown when menu is open): `liquid-glass mx-4 rounded-2xl p-2`, each link is `rounded-xl px-4 py-3 text-sm text-white/90 hover:bg-white/10`.

---

### Hero Content (centered, z-10)

Padding: `pt-28 sm:pt-36 md:pt-44`, `px-4 sm:px-6`.

1. **Tag badge** (animate-fade-up delay-1): `liquid-glass rounded-lg px-4 py-1.5 text-xs sm:text-sm text-white` with inline style `background: rgba(255, 255, 255, 0.16)`. Text: "Press 04 . Vernal woods". Bottom margin `mb-5 sm:mb-6`.

2. **Headline** (animate-fade-up delay-2): `max-w-3xl text-4xl sm:text-5xl md:text-6xl lg:text-7xl leading-[1.1] text-white`. Two lines:
   ```
   records cut for the
   calm listener.
   ```

3. **Subtext** (animate-fade-up delay-3): `mt-5 sm:mt-6 max-w-md text-sm sm:text-base md:text-lg leading-relaxed text-white/90`. Text: "Drone, roots, and nature-captured sound on wax LPs. Every disc cut just once, snag it or miss."

4. **Two buttons** (animate-fade-up delay-4, `mt-8`, stack vertically on mobile, row on `sm:`):
   - **Primary:** `rounded-xl bg-white px-7 py-2.5 text-sm text-gray-900 hover:scale-105 active:scale-95`. Label: "Browse the shelves"
   - **Secondary:** `liquid-glass rounded-xl px-7 py-2.5 text-sm text-white hover:scale-105 active:scale-95`. Label: "Newest arrivals"

---

### Now Playing Widget (bottom-right, z-20)

Positioned `absolute bottom-4 right-4 sm:bottom-6 sm:right-6 md:bottom-8 md:right-10`. Max width `270px` on mobile, `w-72` on sm+. Has `animate-fade-up delay-5`.

- **Track card:** `rounded-2xl bg-white p-2.5 pr-4 shadow-lg`. Contains:
  - Blue icon square (`h-11 w-11 rounded-xl bg-blue-700`) with `BarChart3` icon (size 20, strokeWidth 2.5).
  - Track info: "Helia Marsh -- Fern Light" (truncated, `text-sm text-gray-900`).
  - Progress bar: `h-1 rounded-full bg-gray-200` with `w-[30%] bg-blue-700` fill.
  - Times: "0:33" and "-1:21" in `text-[10px] text-gray-500`.

- **Controls row** (gap-2):
  - "Prev" and "Next" buttons: `flex-1 rounded-2xl bg-white py-2 text-sm text-gray-900 shadow-lg hover:scale-105 active:scale-95`.
  - Heart button (center): `h-10 w-10 rounded-full bg-white shadow-lg hover:scale-110 active:scale-95`. Uses `Heart` icon (size 16) in `text-blue-700`, filled when liked (`fill-blue-700`). Toggles on click.

---

### Key Technical Notes
- The outer wrapper is `relative h-screen w-full overflow-hidden`.
- All interactive elements use `transition-transform duration-200`.
- The accent color throughout is Tailwind's `blue-700`.
- No Supabase or backend needed -- this is purely a static hero.

## Bio-Age Dashboard — Hero [sites/bio-age-dashboard]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(17).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bio-age-dashboard.webp

**Tech Stack:** Vite + React + TypeScript + Tailwind CSS + Lucide React icons. Font: Inter (imported from Google Fonts with weights 300-900).

---

### Background Video

Full-screen background video covering the entire viewport, auto-playing, looping, muted, inline. Fades in over 1500ms on load.

```
URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_044635_8daabe05-1a5c-491c-920f-4b0bd8f04812.mp4
```

Positioned `absolute inset-0 w-full h-full object-cover z-0`.

---

### Navigation Bar

Positioned at top with `px-5 sm:px-8 lg:px-12 pt-6 sm:pt-8 relative z-10`, using `flex items-start justify-between`.

**Left:** Logo SVG image:
```
https://polo-pecan-73837341.figma.site/_assets/v11/f73360d8fc2d33f2b5a4bfb1fa4935fca355946f.svg
```
Size: 160x40px.

**Center:** A help button (HelpCircle icon, 18px, strokeWidth 1.5) in a 36-40px round button with `bg-black/20`, absolutely positioned `left-1/2 top-8 sm:top-10 -translate-x-1/2`.

**Right:** User name "Benjamin Carter" (text-xl sm:text-3xl lg:text-[42px] font-bold, right-aligned, hidden on mobile `hidden md:block`) + a circular profile avatar (44px / 64px / 72px responsive) with image:
```
https://polo-pecan-73837341.figma.site/_assets/v11/745de561b3ebfa8634a3483efc95f21feedd96c9.png
```

---

### Layout (Content Wrapper)

`flex flex-col xl:flex-row xl:items-end xl:justify-between` positioned at bottom on XL screens (`xl:absolute xl:bottom-0 xl:left-0 xl:right-0`), with padding `px-5 sm:px-8 lg:px-12 pb-6 sm:pb-8 lg:pb-12`.

---

### Left Side: Main Age Card

**Card container:** `rounded-[24px] sm:rounded-[32px] lg:rounded-[40px]`, size `w-full sm:w-[520px] lg:w-[620px] h-[420px] sm:h-[500px] lg:h-[550px]`, centered content, `relative overflow-hidden`.

**Rotating background:** An inner div with `absolute inset-[-5%]` (overflows slightly for seamless rotation) with CSS animation `spin-slow 30s linear infinite` (full 360deg rotation). Background image:
```
https://polo-pecan-73837341.figma.site/_assets/v11/d8d9bd498347ea96ca4d675a624c8d90e06786e7.png
```
`background-size: cover; background-position: center;`

**Text overlay (z-10, centered):**
- "Estimated" + "Biological Age" - `text-gray-200 text-base sm:text-lg md:text-[22px] font-medium`
- Large count-up number starting at 0, animating to 28 over 1.8 seconds (40 steps), then incrementing by 1 every 6 seconds indefinitely. Font: `text-[72px] sm:text-[100px] lg:text-[132px] font-semibold leading-[0.85] tracking-tight font-['Inter'] tabular-nums`

---

### Below Age Card: Badge + Ruler Ticker

**Badge:** `"3 Years Younger"` in a pill shape with `border border-[#EFCE96]/50 bg-[#EFCE96]/20 text-white text-xs sm:text-sm font-medium tracking-wide px-4 sm:px-6 py-2 rounded-full`.

**Ruler Ticker:** Infinite horizontal scrolling ruler with gold-colored ticks:
- 61 ticks per set (duplicated for seamless loop)
- Every 10th tick: 26px tall. Every 5th tick: 26px tall. Others: 18px tall.
- Tick color: `rgba(239, 206, 150, 0.5)`, width 1px, rounded.
- Static center indicator tick: 40px tall, 2px wide, color `#EFCE96`, absolutely centered.
- Edge fade mask: `linear-gradient(to right, transparent 0%, black 15%, black 85%, transparent 100%)`
- Animation: `ticker 12s linear infinite` (translateX 0 to -50%)
- Container: `max-w-[620px] h-[40px] mt-2`

---

### Right Side: 4 Info Cards (2x2 grid)

Layout: `flex flex-col gap-4 sm:gap-[20px]`. On mobile, cards stack in pairs (2 per row via `flex-col sm:flex-row`). On XL, they stack vertically aligned right.

All cards: `w-full xl:w-[260px] h-[130px] sm:h-[144px] rounded-[16px] sm:rounded-[20px] p-4 sm:p-5 flex flex-col justify-between`.

**1. Upcoming Activities** - `bg-[#2F2F2F]/60 backdrop-blur-[52px]`, hover: `bg-[#2F2F2F]/70`.
- Title: "Upcoming Activities" (white, text-base sm:text-lg, font-semibold)
- Bottom: "4 events" (text-white/55, text-[11px] sm:text-[12px]) + black circle with ArrowRight icon

**2. Your Insights** - Background image card:
```
https://polo-pecan-73837341.figma.site/_assets/v11/94903fdf21e145cd4ba873c15fc03251c0600ee5.png
```
`background-size: cover; background-position: center;` with `hover:brightness-110`.
- Title: "Your Insights" (white)
- Bottom: "8 Risks" pill (white bg, black text, rounded-full, px-3, h-6 sm:h-7, text-[12px] sm:text-[14px]) + white circle with black ArrowRight

**3. Your Health Snapshot** - Expandable card.
- Default state: `bg-[#2F2F2F]/60 backdrop-blur-[52px]`, white text, shows title + "Recommendations" subtitle + ArrowUp in black circle.
- Expanded state (hover on desktop, click on mobile): `bg-white`, black text, height grows to 280px (desktop) or auto (mobile). Shows full recommendation text: "With a biological age of 28, your body is performing like a young, energetic you. Keep fueling it with movement, nourishing food, quality rest, and a calm mind - so you can stay strong, sharp, and unstoppable." + ArrowDown icon in `bg-[#F0F0F0]` circle. Transition: 300ms ease-in-out on all color/size changes.

**4. Action Plan** - Background image card:
```
https://polo-pecan-73837341.figma.site/_assets/v11/0c38fdb8a933b0da384a5a3f8b0d9986bb919838.png
```
`background-size: cover; background-position: center;` with `hover:brightness-110`.
- Title: "Action Plan" (white)
- Bottom: "Details" pill (white bg, black text) + white circle with black ArrowRight

---

### Animations (Intersection Observer based)

Custom `AnimatedElement` component using IntersectionObserver (threshold 0.1). Elements start invisible with a 40px offset in their specified direction (up/down/left/right) or scale(0.9), then animate to final position with:
- Easing: `cubic-bezier(0.16, 1, 0.3, 1)` (spring-like)
- Duration: 0.8s
- Staggered delays: Nav 100-200ms, Age card 300ms, text 600ms, number 800ms, badge 1000ms, cards 500/650/800/950ms

---

### CSS Keyframes (in index.css)

```css
@keyframes ticker {
  0% { transform: translateX(0); }
  100% { transform: translateX(-50%); }
}
.animate-ticker { animation: ticker 12s linear infinite; }

@keyframes spin-slow {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
.animate-spin-bg { animation: spin-slow 30s linear infinite; }
```

---

### Tailwind Config Additions

```js
colors: {
  surface: { 900: '#0a0a0a', 800: '#1a1a1a', 700: '#2a2a2a', 600: '#3a3a3a' },
  gold: { 400: '#c9a96e', 500: '#b8944d', 600: '#a07d3a' },
},
fontFamily: { sans: ['Inter', 'system-ui', 'sans-serif'] }
```

---

### Global Styles

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap');
body { background-color: #0a0a0a; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
```

---

### Dependencies

- `react` ^18.3.1
- `lucide-react` ^0.344.0 (icons used: ArrowDown, ArrowRight, ArrowUp, HelpCircle)
- `tailwindcss` ^3.4.1
- Vite + React plugin

## Bio-Digital — Hero [sites/bio-digital]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(95).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bio-digital.webp

Build a full-screen hero landing page for a fictional brand called "NeuralKinetics" using React, Vite, Tailwind CSS v4, and Framer Motion (the `motion` package). The page is a single-screen immersive experience with a fixed navbar, a fullscreen looping background video, a centered two-line headline, and a bottom information footer. White background, black text, no purple/violet colors anywhere. The aesthetic is ultra-minimal, luxury tech -- inspired by high-end agency sites.

---

### Tech Stack & Dependencies

- React 19, Vite 6, TypeScript
- Tailwind CSS v4 (using `@tailwindcss/vite` plugin, `@import "tailwindcss"` syntax, and `@theme` block -- NOT the old tailwind.config.js approach)
- `motion` package (Framer Motion v12+, imported as `motion/react`)
- `lucide-react` for the Plus icon
- Google Fonts: **Inter** (weights 400, 500, 600) for body text, **Outfit** (weights 300, 400, 500, 600, 700) as the display/heading font

---

### Fonts & CSS Setup (index.css)

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Outfit:wght@300;400;500;600;700&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-display: "Outfit", ui-sans-serif, system-ui, sans-serif;
  
  --color-brand-black: #000000;
  --color-brand-gray: #F5F5F7;
  --color-brand-text-muted: #6E6E73;
}

@layer base {
  body {
    @apply bg-white text-brand-black font-sans antialiased selection:bg-black selection:text-white;
  }
}
```

This gives us `font-sans` (Inter) and `font-display` (Outfit) as Tailwind utility classes.

---

### Page Structure (App.tsx)

The page is a single `div` with `relative min-h-screen w-full flex flex-col justify-between bg-white text-black font-sans antialiased selection:bg-black selection:text-white overflow-hidden`. It contains these layers in z-order:

### Layer 1: Fullscreen Background Video (z-0)

An absolutely positioned fullscreen container (`absolute inset-0 z-0 pointer-events-none select-none`) containing a `motion.div` that fades in and slightly scales down on load:
- `initial={{ opacity: 0, scale: 1.05 }}`
- `animate={{ opacity: 1, scale: 1 }}`
- `transition={{ duration: 1.8, ease: [0.16, 1, 0.3, 1] }}`

Inside is a `<video>` element:
- **src**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260530_061107_6567e617-ee84-4c3e-ac81-f2d9dda9121a.mp4`
- Attributes: `autoPlay`, `loop`, `muted`, `playsInline`
- Classes: `absolute inset-0 w-full h-full object-cover pointer-events-none`

### Layer 2: Hero Headline (z-10)

A `<main>` element (`flex-1 flex flex-col items-center justify-center px-6 md:px-12 relative z-10`) containing a centered text block:

- Outer wrapper: `text-center w-full max-w-7xl px-4 mt-24 md:mt-0 translate-y-10 md:translate-y-14`
- Inner `motion.div` with entrance animation:
  - `initial={{ opacity: 0, y: 15 }}`
  - `animate={{ opacity: 1, y: 0 }}`
  - `transition={{ duration: 1.4, ease: [0.16, 1, 0.3, 1], delay: 0.2 }}`
  - Classes: `flex flex-col items-center justify-center select-none`

**Line 1 (h1):** "NeuralKinetics"
- Classes: `font-display text-[7.5vw] md:text-[5.8vw] lg:text-[4.6vw] font-medium tracking-tight text-black leading-[0.9]`

**Line 2 (h2):** "cybernetics made organic"
- Same responsive font sizes as h1, same `leading-[0.9]`, with `mt-1 md:mt-1.5`
- "cybernetics" is a `<span>` with `text-black/25 font-light tracking-tight mr-1.5 md:mr-2` (very faded, light weight)
- "made organic" is a `<span>` with `text-black font-medium tracking-tight` (full black, medium weight)

### Layer 3: Fixed Navbar (z-50)

A `motion.nav` fixed at top, full width, with entrance animation:
- `initial={{ y: -16, opacity: 0 }}`
- `animate={{ y: 0, opacity: 1 }}`
- `transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1] }}`
- Classes: `fixed top-0 left-0 w-full p-6 md:p-8 flex flex-col sm:flex-row items-center justify-between gap-4 z-50 pointer-events-none`

**Left side** (`flex flex-wrap items-center gap-3 pointer-events-auto`):

1. **Logo + Brand Name**: A div with `flex items-center gap-1`, containing:
   - A custom SVG logo icon (40x40 viewBox, two black rounded rectangles rotated -35 degrees to form a slanted dual-capsule shape):
     ```
     <rect x="7" y="19" width="15" height="5.5" rx="2.75" transform="rotate(-35 7 19)" />
     <rect x="17.5" y="24" width="15" height="5.5" rx="2.75" transform="rotate(-35 17.5 24)" />
     ```
     Classes: `w-10 h-10 text-black translate-y-[1px]`
   - Text "NeuralKinetics" with `font-display font-medium tracking-tight text-[18px] text-black`

2. **Menu Pill Button**: A black pill button with a white circle containing a Plus icon:
   - Outer button: `flex items-center bg-black hover:bg-zinc-800 text-white p-1 pr-5 gap-2.5 rounded-full transition-all duration-200 cursor-pointer text-[12px] font-medium border border-black/[0.03]`
   - Inner white circle: `w-9 h-9 rounded-full bg-white text-black flex items-center justify-center` containing `<Plus size={13} strokeWidth={3} />` from lucide-react
   - Text "Menu" with `text-[11.5px] pr-1`

3. **Metadata Info Pill** (hidden on mobile, `hidden md:flex`):
   - `items-center bg-[#F4F4F6] border border-black/[0.03] rounded-full px-6 h-11 select-none text-[11.5px] font-normal text-black/60 gap-5`
   - Contains two spans: "Advanced Bionics" and "Cognitive AI"

**Right side** (`pointer-events-auto flex items-center`):

4. **Adaptive Systems Pill**: A light gray compound pill:
   - Outer: `flex items-center bg-[#F4F4F6] hover:bg-[#EAEAEF] transition-colors rounded-full p-1 pr-6 gap-3.5 border border-black/[0.03]`
   - Contains a black circle button (`w-9 h-9 rounded-full bg-black text-white`) with a custom 4-node clover SVG icon (24x24 viewBox, 4 filled circles at cardinal points connected by crosshair lines at 0.6 opacity, center unfilled circle)
   - Text "Adaptive Systems" with `text-[11px] font-medium text-black/70 select-none`

### Layer 4: Footer (z-30)

A footer with `w-full relative z-30 px-8 py-10 md:px-16 md:py-14 bg-gradient-to-t from-white via-white/80 to-transparent` creating a fade-up from white at the bottom.

Inner `motion.div`:
- `initial={{ y: 20, opacity: 0 }}`
- `animate={{ y: 0, opacity: 1 }}`
- `transition={{ delay: 0.5, duration: 1, ease: [0.16, 1, 0.3, 1] }}`
- Classes: `max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-start md:items-end gap-8`

Contains three elements in a row (on desktop):

1. **Left text block** (`max-w-[300px] md:max-w-[340px]`):
   - Label: "Autonomous Dynamics" at `text-[11.5px] font-medium text-black/50`
   - Body: "Unifying biological grace with machine intelligence to design the next era of fusion" at `text-[19px] md:text-[21px] font-normal text-black leading-[1.15] tracking-tight`

2. **Vertical divider** (desktop only): `hidden lg:block w-px h-16 bg-black/[0.08]`

3. **Tag buttons** (`flex flex-wrap gap-2.5`):
   - Three buttons: "Neuromorphic", "AGI", "Cybernetics"
   - Each: `px-6 py-3.5 border border-black/15 hover:border-black text-black text-[11.5px] font-normal rounded-full bg-white hover:bg-black hover:text-white transition-all duration-300 cursor-pointer active:scale-95`

---

### Key Design Details

- **Easing curve used everywhere**: `[0.16, 1, 0.3, 1]` -- a smooth, slightly springy deceleration
- **Color palette**: Pure black (#000), white (#FFF), light gray (#F4F4F6, #EAEAEF), muted text at various black opacities (25%, 50%, 60%, 70%)
- **No purple/indigo/violet anywhere**
- **Typography scale**: Responsive vw-based sizes for the hero (7.5vw mobile, 5.8vw tablet, 4.6vw desktop), pixel-based for UI elements (11px-21px range)
- **All pill-shaped UI elements** use `rounded-full`
- **Selection highlight**: black background, white text (`selection:bg-black selection:text-white`)
- **The background video** plays behind everything, fills the viewport with `object-cover`, and has a subtle scale-down entrance animation
- **Footer gradient** fades from transparent at top to solid white at bottom, ensuring text readability over the video

## Bold Studio — Hero [sites/bold-studio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(10).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bold-studio.webp

Build a fullscreen hero landing page for a creative agency called "VANGUARD" using React, Tailwind CSS, and Vite. The page should be a single viewport-height section with a looping background video and all content overlaid on top.

**Background video:**
Use this exact CloudFront URL as a fullscreen `<video>` element with `autoPlay`, `muted`, `loop`, and `playsInline` attributes, set to `object-cover` to fill the entire viewport:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260606_154941_df1a96e1-a06f-450c-bd02-d863414cc1a0.mp4
```

**Fonts (loaded in index.html):**
1. "FSP DEMO - PODIUM Sharp 4.11" from `https://db.onlinewebfonts.com/c/8b75d9dcff6a48c35a46656192adf019?family=FSP+DEMO+-+PODIUM+Sharp+4.11` -- used for the brand name and main heading. Create a `.font-podium` utility class for it and register it in tailwind.config.js as `fontFamily.podium`.
2. "Inter" from Google Fonts (weights 400, 500, 600, 700) -- used for body text, nav links, stats, and CTAs. Register it in tailwind.config.js as `fontFamily.inter`.

**Icons:** Use `lucide-react` for all icons: `ArrowUpRight`, `Award`, `Crown`, and `X`.

**Navbar:**
- Horizontal bar at the top with responsive padding (`px-6 sm:px-10 lg:px-16`, `py-5 lg:py-7`).
- Left: brand name "VANGUARD" in `font-podium`, white, bold, uppercase, `text-2xl sm:text-3xl`, `tracking-wider`.
- Center (hidden below `md`): four nav links -- "Projects", "Studio", "Offerings", "Inquire" -- in `font-inter`, `text-sm`, `text-white/80`, `tracking-widest`, uppercase, with `hover:text-white` transition.
- Right (hidden below `md`): a "GET IN TOUCH" link with an `ArrowUpRight` icon, styled as a bordered button (`border border-white/30 hover:border-white/60`, `px-6 py-3`, `text-xs`, `tracking-widest`, uppercase, `hover:bg-white/10`).
- Right (visible below `md`): a hamburger button made of three white `div` bars (`w-6 h-0.5`, `w-6 h-0.5`, `w-4 h-0.5` with `space-y-1.5`).

**Mobile Menu Overlay (below `md` only):**
- Fixed fullscreen overlay (`fixed inset-0 z-50`) with `bg-black/95 backdrop-blur-sm`.
- Toggles visibility via React `useState` -- when open: `opacity-100 visible`, when closed: `opacity-0 invisible`, with `transition-all duration-500`.
- Header row matches the navbar: brand name on left, `X` close icon on right.
- Centered vertically: each of the 4 nav links rendered in `font-podium`, `text-4xl sm:text-5xl`, white, uppercase, with staggered entrance animations using inline `style` -- each item gets `transitionDelay: i * 80 + 100ms`, `opacity` and `translateY(20px)` transitions based on the open state.
- Below the links: a "GET IN TOUCH" bordered button with the same staggered animation pattern.
- All links call `setMenuOpen(false)` on click.

**Hero Content (vertically centered, left-aligned):**
All hero elements use staggered `animate-fade-up` animations (defined in CSS as `@keyframes fade-up` translating from `translateY(30px), opacity:0` to `translateY(0), opacity:1` over `0.8s ease-out`). Each successive element has an additional `0.2s` delay. Elements start with `opacity: 0` and use `animation-fill-mode: forwards`.

1. **Tagline:** A `Crown` icon (lucide, `w-4 h-4`, `text-white/70`) followed by "World-Class Digital Collective" in `text-white/70`, `text-xs sm:text-sm`, `font-inter`, `tracking-[0.3em]`, uppercase. Uses `animate-fade-up` (no delay). Has `mb-6 lg:mb-8`.

2. **Main Heading:** Three lines in `font-podium`, white, uppercase, `leading-[0.92]`, `tracking-tight`, each using `text-[clamp(2.8rem,8vw,7rem)]`:
   - "Design."
   - "Disrupt."
   - "Conquer."
   Uses `animate-fade-up-delay-1` (0.2s delay).

3. **Subtext:** "We build fierce brand identities" (line break) "that don't just turn heads --" then bold white "they lead." in `text-white/70`, `text-sm sm:text-base`, `font-inter`, `leading-relaxed`, `max-w-md`. Uses `animate-fade-up-delay-2` (0.4s delay). `mt-6 lg:mt-8`.

4. **CTA Row:** Uses `animate-fade-up-delay-3` (0.6s delay), `mt-8 lg:mt-10`, `flex flex-wrap items-center gap-4 sm:gap-6`.
   - Black button "SEE OUR WORK" with `ArrowUpRight` icon. `bg-black hover:bg-neutral-900`, `px-5 sm:px-7 py-3 sm:py-4`, `text-[11px] sm:text-xs`, `tracking-widest`, uppercase. Arrow has `group-hover:translate-x-0.5 group-hover:-translate-y-0.5` transition.
   - Beside it (hidden on mobile, `hidden sm:flex`): an `Award` icon (`w-8 h-8`, `text-white/50`) with two lines of text: "Top-Rated" / "Brand Studio" in `text-white/60`, `text-xs`, `tracking-wider`, uppercase.

5. **Stats Row:** Uses `animate-fade-up-delay-4` (0.8s delay), `mt-8 sm:mt-10 lg:mt-14`, `flex flex-wrap gap-6 sm:gap-12 lg:gap-16`. Three stats:
   - "250+" / "Brands Transformed"
   - "95%" / "Client Retention"
   - "10+" / "Years in the Game"
   Values in `font-inter`, white, `text-2xl sm:text-4xl lg:text-5xl`, bold, `tracking-tight`. Labels in `text-white/50`, `text-[9px] sm:text-xs`, `tracking-widest`, uppercase, `mt-1`.

**CSS Animations (defined in index.css under `@layer utilities`):**
```css
@keyframes fade-up {
  from { opacity: 0; transform: translateY(30px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes scale-in {
  from { opacity: 0; transform: scale(0.9); }
  to { opacity: 1; transform: scale(1); }
}
```
With classes: `.animate-fade-up` (0s delay), `.animate-fade-up-delay-1` through `.animate-fade-up-delay-4` (0.2s increments, starting `opacity: 0`), `.animate-fade-in`, `.animate-fade-in-delay`.

**Responsive behavior:**
- Full layout is mobile-first with breakpoints at `sm` (640px), `md` (768px), and `lg` (1024px).
- Nav links and "GET IN TOUCH" button show at `md`+; hamburger shows below `md`.
- Award badge hides on mobile (`hidden sm:flex`).
- All text sizes, paddings, gaps, and margins scale up through `sm:` and `lg:` prefixes.
- Stats and CTA row use `flex-wrap` to prevent overflow on small screens.

Make everything fully mobile responsive. Use a single `App.tsx` component with `useState` for the menu toggle. No routing needed.

## Book Hero — Hero [sites/book-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(57).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/book-hero.webp

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Foliom — Book Marquee</title>
  <link href="https://db.onlinewebfonts.com/c/d34add1e23bb969e5eb43cc5a4fab3d0?family=Lawrence+W00+Regular" rel="stylesheet">
  <style>
    *, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
    html, body { width: 100%; height: 100%; overflow: hidden; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #000; }

    body::before {
      content: "";
      position: fixed;
      inset: 0;
      background-image:
        radial-gradient(rgba(255, 240, 200, 0.025) 1px, transparent 1px),
        radial-gradient(rgba(255, 240, 200, 0.015) 1px, transparent 1px);
      background-size: 3px 3px, 7px 7px;
      pointer-events: none;
      z-index: 0;
    }

    :root {
      --book-overlap: 115px;
      --hover-push: 60px;
    }

    /* ---- Hero ---- */
    .hero {
      position: relative;
      width: 100vw;
      height: 100vh;
      overflow: hidden;
      background: #000;
    }

    .hero-title {
      position: absolute;
      top: 130px;
      left: 50%;
      transform: translateX(-50%);
      font-family: 'Lawrence W00 Regular', Georgia, 'Times New Roman', serif;
      font-weight: 400;
      color: #f5f1ea;
      font-size: clamp(120px, 26vw, 420px);
      line-height: 0.9;
      letter-spacing: -0.02em;
      white-space: nowrap;
      z-index: 1;
      user-select: none;
      pointer-events: none;
    }

    /* ---- Categories ---- */
    .hero-categories {
      position: absolute;
      bottom: 40px;
      left: 0;
      right: 0;
      display: flex;
      justify-content: center;
      flex-wrap: wrap;
      gap: 10px;
      z-index: 20;
      padding: 0 20px;
    }

    .hero-category-pill {
      padding: 8px 22px;
      border: 1px solid rgba(245, 241, 234, 0.2);
      border-radius: 999px;
      color: rgba(245, 241, 234, 0.75);
      font-size: 14px;
      letter-spacing: 0.03em;
      background: rgba(255, 255, 255, 0.04);
      backdrop-filter: blur(8px);
      transition: all 300ms ease;
      text-decoration: none;
      cursor: pointer;
    }

    .hero-category-pill:hover {
      color: #f5f1ea;
      border-color: rgba(245, 241, 234, 0.5);
      background: rgba(255, 255, 255, 0.1);
    }

    /* ---- Navbar ---- */
    .navbar {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      z-index: 200;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 20px 48px;
      color: #f5f1ea;
    }

    .nav-brand {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .nav-mark {
      width: 28px;
      height: 32px;
      border: 1.5px solid #f5f1ea;
      border-radius: 4px;
      position: relative;
      flex-shrink: 0;
    }
    .nav-mark::after {
      content: "";
      position: absolute;
      inset: 4px;
      background: #f5f1ea;
      border-radius: 2px;
      clip-path: polygon(0 0, 60% 0, 60% 100%, 0 100%);
    }

    .nav-brand-text {
      font-family: 'Lawrence W00 Regular', Georgia, serif;
      font-size: 1.875rem;
      letter-spacing: -0.025em;
      color: #f5f1ea;
    }

    .nav-links {
      display: flex;
      align-items: center;
      gap: 32px;
      font-size: 14px;
      color: rgba(255, 255, 255, 0.85);
    }
    .nav-links a {
      color: inherit;
      text-decoration: none;
      transition: color 200ms;
    }
    .nav-links a:hover { color: #fff; }

    .nav-right {
      display: flex;
      align-items: center;
      gap: 24px;
      font-size: 14px;
    }
    .nav-right a {
      color: rgba(255, 255, 255, 0.85);
      text-decoration: none;
      transition: color 200ms;
    }
    .nav-right a:hover { color: #fff; }

    .nav-cta {
      background: #f5f1ea !important;
      color: #0a0a0a !important;
      padding: 10px 20px;
      border-radius: 999px;
      font-weight: 500;
      transition: background 200ms;
    }
    .nav-cta:hover { background: #fff !important; }

    .nav-mobile-toggle {
      display: none;
      background: none;
      border: none;
      color: #f5f1ea;
      cursor: pointer;
      padding: 8px;
    }

    /* ---- Marquee ---- */
    .marquee-mask {
      position: absolute;
      top: 62%;
      left: 50%;
      transform: translate(-50%, -50%) rotate(var(--marquee-tilt, -7deg));
      width: 140vw;
      height: calc(286px + 40vw);
      overflow: visible;
      z-index: 10;
    }

    .marquee-fade {
      position: absolute;
      inset: 0;
      overflow: hidden;
      mask-image: linear-gradient(90deg, transparent 0%, black 10%, black 90%, transparent 100%);
      -webkit-mask-image: linear-gradient(90deg, transparent 0%, black 10%, black 90%, transparent 100%);
    }

    .marquee-track {
      position: absolute;
      top: 50%;
      left: 0;
      transform: translateY(-50%);
      display: flex;
      align-items: center;
      width: max-content;
      padding: 60px 0;
      animation: marquee-scroll 60s linear infinite;
      will-change: transform;
    }

    .marquee-mask:has(.book:hover) .marquee-track {
      animation-play-state: paused;
    }

    @keyframes marquee-scroll {
      from { transform: translate(0, -50%); }
      to   { transform: translate(-50%, -50%); }
    }

    /* ---- Book wrapper ---- */
    .book-wrap {
      position: relative;
      width: 200px;
      height: 286px;
      flex-shrink: 0;
      margin-right: calc(-1 * var(--book-overlap));
      transition:
        margin 500ms cubic-bezier(0.2, 0.7, 0.2, 1),
        transform 500ms cubic-bezier(0.2, 0.7, 0.2, 1);
    }
    .book-wrap:last-child { margin-right: 0; }

    .book-wrap:has(+ .book-wrap .book:hover) {
      margin-right: calc(-1 * var(--book-overlap) + var(--hover-push));
    }
    .book-wrap:has(.book:hover) {
      margin-left: var(--hover-push);
      z-index: 9999 !important;
    }

    /* ---- Book ---- */
    .book {
      position: relative;
      height: 286px;
      cursor: pointer;
      transform: rotate(7deg);
      transition: transform 450ms cubic-bezier(0.2, 0.7, 0.2, 1);
      will-change: transform;
    }
    .book:hover {
      transform: rotate(7deg) translateY(-28px) scale(1.06);
      z-index: 100;
    }

    .book-layer {
      position: absolute;
      top: 0;
      left: 0;
      width: 200px;
      height: 286px;
      border-radius: 2px;
      transform-origin: 0 0;
    }

    .book-back-cover {
      box-shadow:
        inset 0 0 0 1px rgba(0, 0, 0, 0.6),
        inset 2px 0 6px rgba(0, 0, 0, 0.5),
        inset -2px 0 6px rgba(0, 0, 0, 0.5);
      z-index: 1;
      filter: brightness(0.7);
    }

    .book-page {
      background: linear-gradient(90deg,
        #8a7649 0%, #c9b88a 6%, #f3e7c9 22%, #fbf3dc 50%,
        #f3e7c9 78%, #c9b88a 94%, #8a7649 100%);
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.4),
        inset 0 -1px 0 rgba(120, 90, 40, 0.25);
    }

    .book-front-cover {
      z-index: 1000;
      background-size: cover;
      background-position: center;
      background-repeat: no-repeat;
      box-shadow:
        0 0 0 1px rgba(0, 0, 0, 0.3),
        inset 0 0 0 1px rgba(255, 255, 255, 0.06),
        inset 8px 0 18px -8px rgba(0, 0, 0, 0.5),
        inset -3px 0 8px -4px rgba(255, 255, 255, 0.08),
        8px 16px 30px rgba(0, 0, 0, 0.6);
    }
    .book-front-cover::before {
      content: "";
      position: absolute;
      inset: 0;
      pointer-events: none;
      background: linear-gradient(90deg,
        rgba(0, 0, 0, 0.30) 0%, rgba(0, 0, 0, 0) 6%,
        rgba(0, 0, 0, 0) 94%, rgba(255, 255, 255, 0.10) 100%);
      border-radius: inherit;
    }

    .book-hinge {
      position: absolute;
      top: 0;
      left: 0;
      height: 5px;
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.08),
        inset 0 -1px 0 rgba(0, 0, 0, 0.6),
        0 1px 2px rgba(0, 0, 0, 0.4);
      border-radius: 1px;
      z-index: 0;
      filter: brightness(0.6);
    }

    /* ---- Responsive ---- */
    @media (max-width: 768px) {
      :root {
        --book-overlap: 90px;
        --hover-push: 30px;
      }
      .navbar { padding: 20px 24px; }
      .nav-links, .nav-right { display: none; }
      .nav-mobile-toggle { display: block; }
      .hero-title {
        top: 100px;
        font-size: clamp(60px, 18vw, 180px);
      }
      .hero-categories { bottom: 30px; gap: 8px; padding: 0 16px; }
      .hero-category-pill { padding: 6px 16px; font-size: 12px; }
      .marquee-mask { top: 58%; height: calc(200px + 30vw); }
      .book-wrap { width: 140px; height: 200px; }
      .book { height: 200px; }
      .book-layer { width: 140px; height: 200px; }
    }

    @media (max-width: 480px) {
      :root {
        --book-overlap: 70px;
        --hover-push: 20px;
      }
      .hero-title {
        top: 80px;
        font-size: clamp(48px, 16vw, 140px);
      }
      .hero-categories { bottom: 20px; gap: 6px; padding: 0 12px; }
      .hero-category-pill { padding: 5px 12px; font-size: 11px; }
      .marquee-mask { top: 55%; }
      .book-wrap { width: 110px; height: 157px; }
      .book { height: 157px; }
      .book-layer { width: 110px; height: 157px; }
    }

    @media (max-width: 360px) {
      .hero-title {
        top: 72px;
        font-size: clamp(40px, 14vw, 100px);
      }
      .hero-categories { bottom: 16px; gap: 5px; padding: 0 10px; }
      .hero-category-pill { padding: 4px 10px; font-size: 10px; }
      .book-wrap { width: 90px; height: 128px; }
      .book { height: 128px; }
      .book-layer { width: 90px; height: 128px; }
    }
  </style>
</head>
<body>
  <section class="hero">
    <!-- Navbar -->
    <nav class="navbar">
      <div class="nav-brand">
        <div class="nav-mark"></div>
        <div class="nav-brand-text">Foliom</div>
      </div>
      <div class="nav-links">
        <a href="#">Catalogs</a>
        <a href="#">Editions</a>
        <a href="#">Hub</a>
        <a href="#">Info</a>
      </div>
      <div class="nav-right">
        <a href="#">Join us</a>
        <a href="#" class="nav-cta">Build Your List</a>
      </div>
      <button class="nav-mobile-toggle" aria-label="Toggle menu">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/></svg>
      </button>
    </nav>

    <!-- Title -->
    <h1 class="hero-title">Foliom</h1>

    <!-- Categories -->
    <div class="hero-categories">
      <a href="#" class="hero-category-pill">Romance</a>
      <a href="#" class="hero-category-pill">Short Story</a>
      <a href="#" class="hero-category-pill">Memoir</a>
      <a href="#" class="hero-category-pill">Classic</a>
      <a href="#" class="hero-category-pill">Fantasy</a>
    </div>

    <!-- Marquee -->
    <div class="marquee-mask" style="--marquee-tilt: -7deg;" id="marquee"></div>
  </section>

  <script>
    const PAGE_STEP = 1.1;
    const PAGE_INSET = 8;
    const SKEW = '30deg';

    const BOOKS = [
      { title: 'Shadows of\nthe Archive', cover: 'linear-gradient(150deg, #2a1a0e 0%, #3d2517 100%)', coverImage: 'book-1.png', pageCount: 22 },
      { title: 'The Temple\nof Lost Suns', cover: 'linear-gradient(150deg, #c46828 0%, #8a4520 100%)', coverImage: 'book-2.png', pageCount: 18 },
      { title: 'Serpent\n& Thorn', cover: 'linear-gradient(150deg, #1a0a10 0%, #2e1018 100%)', coverImage: 'book-3.png', pageCount: 26 },
      { title: 'The Last\nMessage', cover: 'linear-gradient(150deg, #161618 0%, #2a2a2e 100%)', coverImage: 'book-4.png', pageCount: 14 },
      { title: 'All the Light\nWe Cannot See', cover: 'linear-gradient(150deg, #6a7a50 0%, #4a5a38 100%)', coverImage: 'book-5.png', pageCount: 20 },
      { title: 'The Roommate\nRisk', cover: 'linear-gradient(150deg, #e8645a 0%, #c44a40 100%)', coverImage: 'book-6.png', pageCount: 28 },
      { title: 'Ashes of\nAeloria', cover: 'linear-gradient(150deg, #1a1a22 0%, #2a2a30 100%)', coverImage: 'book-7.png', pageCount: 16 },
      { title: 'Own Your\nTime', cover: 'linear-gradient(150deg, #e85a10 0%, #c84a08 100%)', coverImage: 'book-8.png', pageCount: 24 },
      { title: 'The Quiet\nWitness', cover: 'linear-gradient(150deg, #1a1e24 0%, #2a3038 100%)', coverImage: 'book-9.png', pageCount: 30 },
      { title: 'The Light\nWe Carry', cover: 'linear-gradient(150deg, #0e1a30 0%, #1a2a48 100%)', coverImage: 'book-10.png', pageCount: 19 },
      { title: 'The Bright\nBeyond', cover: 'linear-gradient(150deg, #0c1428 0%, #1a2040 100%)', coverImage: 'book-11.png', pageCount: 22 },
      { title: 'The Spaces\nBetween', cover: 'linear-gradient(150deg, #c8b8a0 0%, #a89878 100%)', coverImage: 'book-12.png', pageCount: 15 },
      { title: 'Sunshine and\nSecond Chances', cover: 'linear-gradient(150deg, #f5e6b8 0%, #e8c870 100%)', coverImage: 'book-13.png', pageCount: 20 },
      { title: 'The Stories\nWe Keep', cover: 'linear-gradient(150deg, #b8a0d0 0%, #9480b8 100%)', coverImage: 'book-14.png', pageCount: 18 },
      { title: 'The Right Swing\nWrong Timing', cover: 'linear-gradient(150deg, #f0b8c8 0%, #e8a0b0 100%)', coverImage: 'book-15.png', pageCount: 16 },
      { title: 'Cover 16', cover: 'linear-gradient(150deg, #2a3040 0%, #1a2030 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_045156_4a79ba3c-ba56-4cd4-834b-d9728f56d1a4.png&w=1920&q=85', pageCount: 20 },
      { title: 'Cover 17', cover: 'linear-gradient(150deg, #3a2a18 0%, #2a1a10 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_044451_68c948df-6c4c-45eb-974e-923486a41e41.png&w=1920&q=85', pageCount: 24 },
      { title: 'Cover 18', cover: 'linear-gradient(150deg, #1a2838 0%, #0e1828 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043853_b2f3c7c8-5d47-43bc-9ce4-2fa8d717e42b.png&w=1920&q=85', pageCount: 18 },
      { title: 'Cover 19', cover: 'linear-gradient(150deg, #28201a 0%, #1a1410 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043838_240c7443-18d6-4d61-be4a-2d01e2dd65a6.png&w=1920&q=85', pageCount: 26 },
      { title: 'Cover 20', cover: 'linear-gradient(150deg, #2a3828 0%, #1a2818 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043832_62031210-1de3-47a6-ac3f-78eb84e99858.png&w=1920&q=85', pageCount: 22 },
      { title: 'Cover 21', cover: 'linear-gradient(150deg, #382a20 0%, #281a12 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043220_eb34b2f8-8a78-4b29-bbbc-d61e137aedad.png&w=1920&q=85', pageCount: 14 },
      { title: 'Cover 22', cover: 'linear-gradient(150deg, #1a2028 0%, #101820 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043212_ce1559e3-e0a6-48c8-887f-1331a8e989c5.png&w=1920&q=85', pageCount: 28 },
      { title: 'Cover 23', cover: 'linear-gradient(150deg, #302818 0%, #201810 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_032638_9520b937-d7e8-4f6a-88ba-8bbd1e8ecbfe.png&w=1920&q=85', pageCount: 16 },
      { title: 'Cover 24', cover: 'linear-gradient(150deg, #20282e 0%, #141c22 100%)', coverImage: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260528_043201_f749ce8e-72f6-46eb-8440-0f6bdbf2a782.png&w=1920&q=85', pageCount: 19 },
    ];

    function createBook(book) {
      const depth = PAGE_STEP * (book.pageCount + 1);
      const bookEl = document.createElement('div');
      bookEl.className = 'book';
      bookEl.style.width = `${200 + depth + 1.1}px`;

      // Hinge
      const hinge = document.createElement('div');
      hinge.className = 'book-hinge';
      hinge.style.width = `${depth + 1}px`;
      hinge.style.background = book.cover;
      bookEl.appendChild(hinge);

      // Back cover
      const back = document.createElement('div');
      back.className = 'book-layer book-back-cover';
      back.style.background = book.cover;
      back.style.transform = `translateX(${depth}px) skewY(${SKEW})`;
      bookEl.appendChild(back);

      // Pages
      for (let i = 1; i <= book.pageCount; i++) {
        const t = i / book.pageCount;
        const page = document.createElement('div');
        page.className = 'book-layer book-page';
        page.style.transform = `translateX(${PAGE_STEP * i}px) skewY(${SKEW})`;
        page.style.zIndex = 2 + (book.pageCount - i);
        page.style.filter = `brightness(${(1 - t * 0.06).toFixed(3)})`;
        page.style.top = `${PAGE_INSET / 2}px`;
        page.style.height = `calc(100% - ${PAGE_INSET}px)`;
        bookEl.appendChild(page);
      }

      // Front cover
      const front = document.createElement('div');
      front.className = 'book-layer book-front-cover';
      front.style.transform = `skewY(${SKEW})`;
      if (book.coverImage) {
        front.style.backgroundImage = `url(${book.coverImage})`;
        front.style.backgroundSize = 'cover';
        front.style.backgroundPosition = 'center';
      } else {
        front.style.background = book.cover;
      }
      bookEl.appendChild(front);

      return bookEl;
    }

    function buildMarquee() {
      const marquee = document.getElementById('marquee');
      const fade = document.createElement('div');
      fade.className = 'marquee-fade';

      const track = document.createElement('div');
      track.className = 'marquee-track';

      const allBooks = [...BOOKS, ...BOOKS];
      const total = allBooks.length;

      allBooks.forEach((book, i) => {
        const wrap = document.createElement('div');
        wrap.className = 'book-wrap';
        wrap.style.zIndex = total - i;
        wrap.appendChild(createBook(book));
        track.appendChild(wrap);
      });

      fade.appendChild(track);
      marquee.appendChild(fade);
    }

    buildMarquee();
  </script>
</body>
</html>

## Cargo Group — Hero [sites/cargo-group]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/carArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cargo-group.mp4

Create a full-viewport hero section for "CARGOX GROUP" logistics company using React, Tailwind CSS, Framer Motion (`motion` package from npm - import from `motion/react`), and `lucide-react` for the hamburger icon.

### Tech Stack
- React 18 + TypeScript + Vite
- Tailwind CSS 3
- `motion` package (v12+) - import `{ motion, AnimatePresence }` from `motion/react`
- `lucide-react` for Menu/X icons
- Google Font: `Barlow Condensed` weight 800 (imported in CSS via `@import url('https://fonts.googleapis.com/css2?family=Barlow+Condensed:wght@800&display=swap')`)

### Global CSS
```css
@import url('https://fonts.googleapis.com/css2?family=Barlow+Condensed:wght@800&display=swap');
@tailwind base;
@tailwind components;
@tailwind utilities;

* { margin: 0; padding: 0; box-sizing: border-box; }
html, body, #root { height: 100%; overflow: hidden; }
body { font-family: Helvetica, Arial, sans-serif; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
```

### Layout Structure
Single full-viewport (`min-height: 100vh`) flex column with `overflow: hidden` and dark fallback `backgroundColor: '#1a1a2e'`. All content is layered above a fullscreen video background.

### Video Background
- Absolute positioned, `inset-0`, `object-cover`, `z-0`
- Attributes: `autoPlay`, `muted`, `loop`, `playsInline`
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260620_185230_f7f71ef4-6655-469f-b9c6-efbdc1f7684a.mp4`
- On `onCanPlay`, set a `videoReady` state to `true`
- All content below is wrapped in `<AnimatePresence>` and only renders when `videoReady === true`, fading in with `opacity: 0 -> 1` over 0.3s. The wrapper div uses `className="flex flex-col flex-1 w-full"`.

### Easing
Use a shared constant: `const EXPO_OUT: [number, number, number, number] = [0.16, 1, 0.3, 1];`

### Header (z-50, relative)
- Padding: `clamp(16px, 4vh, 40px) clamp(16px, 3vw, 48px) 0`
- **Logo** (left): Two lines stacked - "CARGOX" in white, "GROUP" in `#ffda00`. Font: `"Barlow Condensed"`, weight 800, size `clamp(22px, min(3.15vh, 2.32vw), 32px)`, line-height 0.9, uppercase, letter-spacing -0.01em. Animates from `opacity:0, y:-20` to visible with EXPO_OUT, duration 0.6.
- **Desktop Nav** (hidden on mobile, flex on md+): Items "Services", "Industries", "Company" each with a chevron-down SVG. Gap: `clamp(20px, 3.8vw, 52px)`. White text, size `clamp(15px, min(1.97vh, 1.45vw), 20px)`, letter-spacing -0.02em. Each item fades in staggered. On hover: color shifts to `#ffda00` with `x: 2`.
- **Mobile Hamburger** (md:hidden): lucide-react `Menu`/`X` icon, white, size 28, toggles a mobile menu overlay.

### Mobile Menu Overlay
- Absolute `inset-0`, z-40, centered flex column, bg `#6682c2`
- Same nav items as buttons, font-size 24px, white, staggered fade-in

### Main Content (z-10, relative)
- `flex-1`, grid: 1 col on mobile, `grid-cols-[2.17fr_1fr]` on lg+
- Padding: `clamp(24px, 8vh, 120px) clamp(16px, 3vw, 48px) 0`
- Gap: `clamp(20px, 4vh, 48px)`

### Left Column - Giant Headline
- Container: `overflow: clip`
- Font: `"Barlow Condensed"`, weight 800, size `clamp(86px, min(14vh, 11vw), 220px)`, line-height 0.78, uppercase, letter-spacing -0.01em
- Three lines with slide-in animations:
  1. "BEYOND" - white, slides from `x: -900` with duration 0.85, delay 0
  2. "BORDERS" - color `#002a35`, `marginLeft: 0.524em`, slides from `x: 900` with duration 0.85, delay 0.13
  3. "AND LIMITS" - white, slides from `x: -900` with duration 0.85, delay 0.26
- All use EXPO_OUT easing

### Right Column
- Flex column, gap: `clamp(16px, 2.66vh, 32px)`

### Tagline Text
- Font: Helvetica, size `clamp(24px, min(4vh, 3vw), 52px)`, line-height 0.9, letter-spacing -0.02em, color `#002a35`
- Three lines with word-by-word reveal animation (each word slides up from `y:'100%'` with rotateX 45deg):
  1. "Logistics" - marginLeft 0, delay 0.3
  2. "shaped by scale" - marginLeft 1.5em, delay 0.5
  3. "powered by precision" - marginLeft 0, delay 0.7
- Each word has 0.08s stagger, duration 0.6, easing `[0.16, 1, 0.3, 1]`

### Map Section
- Container: relative, `aspectRatio: '435 / 263'`
- **Map image**: absolute, inset-0, object-contain
  - URL: `https://polo-pecan-73837341.figma.site/_assets/v11/b6d561167283e799453232309bd13dd78b2d1afa.png`

- **Route Lines SVG Overlay**: absolute, pointer-events-none, positioned at `left: 13.8%`, `top: 24.3%`, `width: 68.7%`, aspectRatio `299/143`
  - SVG viewBox: `0 0 299.037 142.509`, overflow visible
  - 4 animated bezier curve paths in `#FFDA00`, strokeWidth 2.5:
    ```
    M128.161 74.6764C79.9989 130.001 71.9994 46.0005 20.9815 111.737
    M216.999 9.99985C260.499 12.4998 222.499 71.9998 291.999 58.9998
    M130.102 70.9998C144.499 -32.0002 183.852 70.2739 219.999 3.99985
    M14.4999 16.9998C111 20.9998 -53.0003 73.4998 21.4999 107
    ```
  - Each path animates `pathLength: 0->1` with duration 1.1, staggered delay starting at 0.55 + i*0.12
  - Animated arrow polygons (triangles `0,-4 8,0 0,4`) move along each path using `<animateMotion>` with `rotate="auto"`, duration `2.5 + i*0.3`s, infinite repeat
  - 5 stop dots at coordinates: `[9.519, 15.519]`, `[289.519, 59.518]`, `[220.519, 9.519]`, `[125.518, 78.519]`, `[19.519, 104.519]`. Each is a yellow circle (r=9.519, fill `#FFDA00`) with a smaller dark center circle (r=3.389, fill `#002A35`). Spring animation with stiffness 420, damping 14.

- **Transport Icons**: 3 circular white icons absolutely positioned on the map:
  - Ship: `left: 26.0%, top: 28.9%`, delay 2.1, URL: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/08d6a37375d428e07c59e24a8529de89bfee157e.08d6a373.png`
  - Car: `left: 70.8%, top: 15.6%`, delay 2.2, rotate `9.73deg`, URL: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/7d6f50a87e1427d9b4d1a9c9f1c064ff04b2b3f9.7d6f50a8.png`
  - Plane: `left: 55.2%, top: 52.1%`, delay 2.3, rotate `180deg scaleY(-1)`, URL: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/0e0282ab1c70db03d437b0d01875ce45557d49f6.0e0282ab.png`
  - Each icon: `width: 14.9%`, aspect-ratio 1, rounded-full, bg-white, box-shadow `0 4px 12px rgba(0,0,0,0.15)`. On hover: scale 1.12, translateY -4px, enhanced shadow. Spring animation: stiffness 220, damping 16.

- **Map description text**: absolute, hidden on mobile (`hidden sm:block`), positioned `left: 55.6%, top: 89%, width: 44%`. Text: "We ensure full transparency at every stage to build trust and drive results." Size `clamp(12px, min(1.6vh, 1.2vw), 20px)`, color `#002a35`, fades in at delay 2.4.

### Footer (z-10, relative)
- Flex row (column on mobile), space-between, padding: `clamp(12px, 3vh, 32px) clamp(16px, 3vw, 48px) clamp(16px, 5vh, 66px)`

### Left - Stat Block
- Animates from `opacity:0, y:24` with delay 0.45, duration 0.65, EXPO_OUT
- "3M+" in `"Barlow Condensed"` weight 800, size `clamp(52px, min(8vh, 6vw), 98px)`, color `#ffda00`, uppercase
- Description: "tons of cargo / successfully delivered / without delays" - size `clamp(16px, min(1.6vh, 1.2vw), 20px)`, white, line-height 1.25
- Small cargo icon in white circle: `clamp(40px, min(5.5vh, 4vw), 67px)` diameter, URL: `https://image-bottom-92901062.figma.site/_components/v2/142c6a6f3074dd8aee013fa440ff4ff369649d48/b343ed71e721488b90c407df666fd6dc3f5f70b1.b343ed71.png`

### Right - CTA Button
- Custom SVG pill shape with a circle cutout on the right. Fill `#ffda00`. The full SVG path:
  ```
  M316 0C329.08 0 340.435 7.38674 346.121 18.2162C348.618 22.9736 353.086 26.8535 358.459 26.8535H359.252C364.667 26.8535 369.155 22.9169 371.63 18.1007C377.159 7.34039 388.205 0.00015843 400.931 0C419.195 0 434.001 15.1191 434.001 33.7695L433.99 34.6416C433.537 52.8891 418.909 67.5391 400.931 67.5391C387.96 67.5389 376.734 59.9132 371.317 48.8128C368.923 43.9077 364.427 39.873 358.969 39.873C353.492 39.873 348.986 43.9356 346.589 48.8605C341.074 60.1913 329.449 68 316 68H34.001C15.2233 68 0 52.7777 0 34C0 15.2223 15.2233 0 34.001 0H316ZM400.931 2.44141C384.063 2.44163 370.303 16.419 370.303 33.7695C370.303 51.1201 384.063 65.0974 400.931 65.0977C417.798 65.0977 431.56 51.1202 431.56 33.7695C431.56 16.4189 417.798 2.44141 400.931 2.44141Z
  ```
- ViewBox: `0 0 434.001 68`, preserveAspectRatio `none`
- Size: full-width on mobile (h-56px), on sm+: `h-[clamp(48px,min(6vh,4.5vw),68px)]` with `aspect-[434/68]`
- **Arrow** in the circle cutout: SVG arrow (`viewBox="0 0 16.89 20.37"`, white stroke, strokeWidth 2.2) that rotates from `-135deg` to `-90deg` on hover with 0.35s transition
- **Label**: "Get in touch", centered in the pill area (excluding circle), color `#002a35`, size `clamp(14px, min(1.6vh, 1.2vw), 20px)`
- Animates in from `opacity:0, x:60` with delay 0.5. whileHover: scale 1.08, y:-2. whileTap: scale 0.97.

### Color Palette
- Primary dark: `#002a35`
- White: `#ffffff`
- Accent yellow: `#ffda00`
- Fallback bg: `#1a1a2e`
- Mobile menu bg: `#6682c2`

### Key Behaviors
1. All animations are gated behind `videoReady` state - nothing animates until the video fires `onCanPlay`
2. The entire content fades in once the video is ready
3. All content (header, main, footer) uses `relative z-10` or `z-50` to layer above the video (z-0)
4. Fully responsive: single column on mobile, 2-column grid on lg+
5. Mobile hamburger menu with overlay on md breakpoint

## Cinematic Brand — Hero [sites/cinematic-brand]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(65).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cinematic-brand.webp

Build a full-screen cinematic hero landing page for a brand called "VERTX". Use React, TypeScript, Vite, Tailwind CSS, Framer Motion, and Lucide React icons.

### Font

Use the custom font **"Quire Sans Pro"** loaded from this CDN in `index.html`:
```
https://db.onlinewebfonts.com/c/5a981c7d02abe9aec215dbe4606407e2?family=Quire+Sans+Pro
```
Set it as the default `sans` font family in `tailwind.config.js`:
```js
fontFamily: {
  sans: ['"Quire Sans Pro"', 'sans-serif'],
},
```

### Page Title
`Nexora — Beyond the Interface`

### Background Video

Use this CloudFront video URL as a full-screen looping background:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260606_170109_f96e01a5-b0db-4274-b24d-8d97e99ec928.mp4
```
The `<video>` element should be `absolute inset-0`, `object-cover`, with attributes: `autoPlay`, `muted`, `loop`, `playsInline`, `preload="auto"`. Also use a `useRef` + `useEffect` to call `.play()` on mount as a fallback for autoplay.

### Navbar (fixed, top, z-50)

- Animates in from top using Framer Motion: `initial={{ y: -24, opacity: 0 }}`, `animate={{ y: 0, opacity: 1 }}`, `transition={{ duration: 0.6, ease: [0.25, 0.46, 0.45, 0.94] }}`.
- Outer: `fixed inset-x-0 top-0 z-50 px-3 pt-3 sm:px-5 sm:pt-4`.
- Inner container: `mx-auto flex h-14 max-w-7xl items-center justify-between px-4 sm:h-16 sm:px-6`.
- **Logo (left):** Lucide `Hexagon` icon (strokeWidth 1.5, 24x24, white) + text "VERTX" (`text-[15px] font-semibold tracking-tight text-white sm:text-base`). The Hexagon rotates 30deg on hover via a `group` + `group-hover:rotate-[30deg]` with `transition-transform duration-300`.
- **Right side:** Two buttons:
  1. "Contact" -- ghost style: `rounded-full border border-white/15 bg-white/5 px-5 py-2 text-[13px] font-medium text-white/80 backdrop-blur-sm transition-all hover:border-white/30 hover:text-white hover:scale-105 active:scale-100`.
  2. "Sign Up" -- primary style with `btn-glow` class: `rounded-full bg-slate-950 px-5 py-2 text-[13px] font-medium text-white transition-transform hover:scale-105 active:scale-100`.

### Hero Content (centered, shifted up 50px)

The content wrapper: `relative z-10 flex min-h-screen flex-col items-center justify-center px-5 pt-20 text-center sm:px-6 sm:pt-24` with inline style `marginTop: '-50px'`.

### 1. Eyebrow text
Wrapped in a `FadeUp` component (delay=0, mb-4 sm:mb-6):
- A `<p>` with: `flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.25em] text-white/60 sm:gap-3 sm:text-xs md:text-sm md:tracking-[0.3em]`.
- Starts with a small horizontal line: `<span className="inline-block h-px w-4 bg-white/40 sm:w-6" />`.
- Text: **"The future is unfolding"**.

### 2. Main Heading (h1)
`max-w-4xl text-[1.75rem] font-medium leading-[1.15] tracking-tight text-white sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl`.
- Uses a `TypingEffect` component that splits the text into individual characters, each animated with Framer Motion (`initial={{ opacity: 0 }}`, `animate={{ opacity: 1 }}`, staggered with `delay: i * 0.045`, `duration: 0.15`). Words are kept as `inline-block` spans so they wrap naturally, with `&nbsp;` between words.
- Text: **"Innovation that reshapes the fabric of experience"**.

### 3. Subheading
Wrapped in `FadeUp` (delay=2.4, mt-4 sm:mt-6):
- `max-w-xs text-xs font-light leading-relaxed text-white/50 sm:max-w-xl sm:text-sm md:text-base lg:text-lg`.
- Text: **"We craft platforms where insight, power, and design converge -- giving rise to something the world hasn't seen."** (use an em dash).

### 4. Buttons row
Container: `mt-8 flex w-full max-w-sm flex-col items-center gap-3 sm:mt-10 sm:w-auto sm:max-w-none sm:flex-row sm:gap-5`. Stacks vertically on mobile, horizontal on sm+.

- **"Begin Now" button** (FadeUp delay=2.8): `btn-glow flex w-full items-center justify-center gap-2.5 rounded-full bg-slate-950 py-2.5 pl-6 pr-3 text-sm font-medium text-white transition-transform hover:scale-105 active:scale-100 sm:w-auto sm:gap-3 sm:py-3.5 sm:pl-10 sm:pr-5 sm:text-lg`. Contains a circular play icon on the right: a `<span>` styled as `flex h-7 w-7 items-center justify-center rounded-full border border-white/30 sm:h-9 sm:w-9` with a Lucide `Play` icon (`h-3 w-3 fill-current sm:h-4 sm:w-4`).

- **"Watch the story" button** (FadeUp delay=3.0): `w-full rounded-full border border-white/15 bg-white/5 px-6 py-2.5 text-sm font-medium text-white/80 backdrop-blur-sm transition-all hover:border-white/30 hover:text-white hover:scale-105 active:scale-100 sm:w-auto sm:px-10 sm:py-3.5 sm:text-lg`.

### FadeUp Component

A reusable animation wrapper using Framer Motion:
- Props: `children`, `className`, `delay` (default 0), `duration` (default 0.6), `y` (default 24).
- Uses `useInView` with `{ once: true }` to trigger only when scrolled into view.
- `initial={{ opacity: 0, y }}`, animates to `{ opacity: 1, y: 0 }` when in view.
- Easing: `[0.25, 0.46, 0.45, 0.94]`.

### TypingEffect Component

A character-by-character reveal animation:
- Props: `text`, `className`, `charDelay` (default 0.045).
- Uses `useInView` with `{ once: true }`.
- Splits text by spaces into words, then each word into characters. Each character is a `<motion.span>` with `initial={{ opacity: 0 }}`, `animate={{ opacity: 1 }}` (when in view), `transition={{ duration: 0.15, delay: globalCharIndex * charDelay }}`. Characters and words are `inline-block`. Words are separated by `&nbsp;`.

### Custom CSS (index.css)

The `btn-glow` class creates an inner white glow effect on primary buttons:
```css
.btn-glow {
  outline: 1.5px solid rgba(255, 255, 255, 0.6);
  outline-offset: -1.5px;
  box-shadow: inset 0 0 14px 0 rgba(255, 255, 255, 0.7);
}

@media (min-width: 640px) {
  .btn-glow {
    outline-width: 2px;
    outline-offset: -2px;
  }
}
```

### Outer Section
The root `<section>` wrapping everything: `relative min-h-screen w-full overflow-hidden bg-black`.

### Dependencies
- `react`, `react-dom` (v18)
- `framer-motion`
- `lucide-react`
- `tailwindcss`, `autoprefixer`, `postcss`
- Vite + TypeScript

## Contact Cybernetic — Hero [sites/contact-cybernetic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(42).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/contact-cybernetic.webp

Build a modern, interactive hero section using React, Tailwind CSS, and Framer Motion (motion/react). Ensure you follow these precise architecture and styling instructions:
1. Fonts & Global Animations
Import the Inter font from Google Fonts.
In your CSS setup, configure Tailwind to use it by default (--font-sans: 'Inter', ...).
Create a keyframe animation in CSS named blink for the typewriter cursor:
code
CSS
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
.animate-blink { animation: blink 1s step-end infinite; }
2. General Page Structure
Wrap the entire application in a container div with the following classes: relative bg-white text-neutral-900 font-sans selection:bg-[#EAECE9] selection:text-[#1C2E1E] antialiased overflow-x-hidden flex flex-col lg:block lg:min-h-screen.
3. Background Video Component (with Native Scrubbing)
Container element: Add a div containing the background video with classes: order-last lg:order-none relative lg:absolute lg:inset-0 lg:z-0 overflow-hidden pointer-events-none w-full aspect-square md:aspect-video lg:aspect-auto lg:h-full bg-neutral-50 lg:bg-transparent.
Video element: Use <video> with muted, playsInline, preload="auto".
Video Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260601_110537_3a579fa0-7bbc-4d94-9d25-0e816c7840f5.mp4
Classes: w-full h-full object-cover object-right lg:object-right-bottom.
Scrubbing/Playback Logic via useEffect hooks:
Desktop Mouse Scrubbing Hook: Listen to the window mousemove event. If window.innerWidth < 1024, ignore (disable scrubbing). Store the mouse 'previous X' coordinate to calculate the delta against 'current X'. Update the target scrub time based on (delta / window.innerWidth) * 0.8 * video.duration. Clamp the time between 0 and duration. Set video.currentTime = targetTime. Bind a seeked event listener to ensure smooth tracking frame to frame.
Mobile Autoplay Hook: Because scrubbing is disabled on mobile frames, trigger normal playback for screens < 1024 width: video.autoplay = true and video.play().
4. Interactive Navbar
Header wrapper: Wrap the Navbar in <header className="fixed top-0 inset-x-0 z-10 px-5 sm:px-8 py-4 sm:py-5 flex flex-row justify-between items-center bg-transparent">
Logo (Left side): Flex row with gap-3.
Text: Mainframe&reg; (using the ® symbol). Classes: text-[21px] sm:text-[26px] tracking-tight text-black font-medium select-none.
Icon block right beside it: An asterisk &#10033;. Classes: text-[25px] sm:text-[30px] text-black select-none tracking-[-0.02em] font-medium leading-none mb-1.
Desktop Nav Links (Center): Flex row, hidden md:flex, text-[23px] text-black. Links are "Labs", "Studio", "Openings", "Shop" separated by <span className="opacity-40">,&nbsp;</span> dividers. Hover states should use hover:opacity-60 transition-opacity.
Desktop CTA (Right): Hidden on mobile. A link reading "Get in touch" mapped with text-[23px] text-black underline underline-offset-2 hover:opacity-60 transition-opacity.
Mobile Menu Logic:
Hamburger <button> visible below md. Has three w-6 h-[2px] bg-black spans.
Hook it to a local state isMobileMenuOpen. When open, animate the burger into an 'X' (top bar rotate-45 translate-y-[7px], middle bar opacity-0, bottom bar -rotate-45 -translate-y-[7px]). All spans need transition-all duration-300.
Create a full screen Mobile Navigation Overlay div hidden on Desktop. Fixed inset-0 z-[9] with bg-white/95 backdrop-blur-sm. Apply opacity-100 pointer-events-auto when isMobileMenuOpen is true; otherwise, opacity-0 pointer-events-none.
5. Content Layout Container
Below the background video and relative to it, add a content grouping layer: <div className="relative z-10 flex flex-col order-first lg:order-none w-full bg-white lg:bg-transparent pb-8 lg:pb-0 lg:min-h-screen">
Inside that, the overarching layout engine: <main id="spade-hero" className="w-full max-w-7xl mx-auto px-6 py-12 flex-1 flex flex-col justify-center">
6. Typewriter Hook and Headline
Implement a custom useTypewriter(text, speed = 38, startDelay = 600) React hook. It uses setTimeout and setInterval to iteratively build a string slice by slice. It must return an object: { displayed: string, done: boolean }.
Run the hook with the string "we'd love to\nhear from you!".
Wrap the headline securely in a motion.div configured to drop-in (initial: opacity: 0, y: 20, animate: opacity: 1, y: 0, transition duration 0.6).
Render your hook text inside <h1 className="text-5xl md:text-6xl lg:text-[76px] font-normal tracking-tight text-black leading-[1.08] mb-8 select-none w-full whitespace-pre-wrap">.
While typing (!done), output a <span className="inline-block w-[2px] h-[1.1em] bg-black align-middle ml-[2px] animate-blink" /> cursor at the end of the displayed text string.
7. Secondary Description Text
Another motion.div (delay 0.1s from the headline).
Content: <p> tag that reads: Whether you have questions, feedback, <br /> drop us a message and we'll get back to you as soon as possible.
Classes: text-lg md:text-xl text-[#5A635A] leading-relaxed font-normal mb-14 max-w-2xl.
8. Interactive Multi-Select Service Pills
Using setServices track an array ["Brand", "Digital", "Campaign", "Other"].
The prompt Title: "What sort of service?" (text-2xl font-medium tracking-tight mb-2). Subtitle: "Select all that apply" (opacity-85 text-[#738273] mb-8).
Iterate over the options natively outputting motion.button wrapper tags allowing multiple selections inside a flex wrap container.
Pill active traits classes: bg-[#1C2E1E] text-white shadow-md shadow-emerald-950/5 transform. Show a check icon (lucide-react) dropping in using type: "spring", stiffness: 300, damping: 20.
Pill inactive traits classes: bg-white text-[#1C2E1E] border border-[#F1F3F1] hover:bg-[#F1F3F1]/55.
Contingent Feedback Status Banner: Underneath your service pills, write an <AnimatePresence mode="wait"> that tracks user state array length:
Empty: Show a generic placeholder indicating "Please click to select services above." at fifty percent opacity (opacity: 0.5, italic, text-xs).
Active Selection: Swap cleanly into a container <motion.div> that springs height gracefully (height: "auto"). Inside, display an acknowledgment banner reading "Ready to inquire about: [array.join(", ")]" combined with an arrow call-to-action button "Let's Go" (text-[#4D6D47] uppercase text-xs). Style the banner with bg-[#FAFBF9] border rounded-2xl.

## Conversion — Hero [sites/conversion]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(84).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/conversion.webp

**Build a fullscreen hero section with a looping background video, navigation bar, and centered content. Use React, TypeScript, Tailwind CSS, and Lucide React icons. Here are the exact specifications:**

---

### Fonts

Load two fonts in `index.html`:
- **Qanelas-Heavy** from `https://db.onlinewebfonts.com/c/3010f9da43a41a81d5daa32bd6edebc2?family=Qanelas-Heavy`
- **Inter** (weights 400, 500, 600, 700) from Google Fonts

Define custom font utility classes in `index.css`:
- `.font-qanelas` -- font-family: 'Qanelas-Heavy', sans-serif; font-weight: 900;
- `.font-inter` -- font-family: 'Inter', sans-serif;

Register both in `tailwind.config.js` under `theme.extend.fontFamily`.

---

### Layout

The entire section is `w-full h-screen overflow-hidden` with `font-inter` as the base font. Everything is stacked via `relative`/`absolute` positioning.

---

### Background Video

A `<video>` element set to `autoPlay muted loop playsInline` with class `absolute inset-0 w-full h-full object-cover`. The video source URL is:

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260613_235144_2f72690f-ad2d-4b1d-9dc1-96fc97d01ca5.mp4
```

---

### Content Layer

A `relative z-10 flex flex-col h-full` container holds everything on top of the video.

---

### Navigation Bar

- Container: `flex items-center justify-between`, padding `px-5 sm:px-6 md:px-12 lg:px-16 py-4 md:py-5`
- Has the `animate-nav` class (fades down from -20px, 0.6s duration, 0.1s delay, ease-out-expo curve)

**Left -- Brand:**
- Text: `text-xl sm:text-2xl font-bold text-gray-900`
- First part: `"Zipwire."` in `font-qanelas italic`
- Second part: `"Dev"` in `font-inter font-normal text-gray-600`

**Center -- Desktop nav links (hidden below md):**
- `flex items-center gap-8`
- Four links: "Overview" (#overview), "Docs" (#docs), "Our Team" (#team), "Upgrade" (#upgrade)
- Each: `text-sm font-medium text-gray-800 hover:text-gray-600 transition-colors`

**Right -- Desktop actions (hidden below md):**
- Language indicator: a small `w-5 h-4 rounded-sm bg-black` box with white "DE" text (10px), next to "EN" in `font-medium`
- Green CTA button: `bg-[#4CAF50] hover:bg-[#43A047] text-white text-sm font-semibold px-5 py-2.5 rounded-full transition-colors` with a `Download` icon (w-4 h-4) and text "Get It Today"

**Right -- Mobile hamburger (md:hidden):**
- A button with a `relative w-6 h-6` container holding both `X` and `Menu` icons absolutely positioned
- Icons crossfade with rotation: `transition-all duration-300`, the active icon is `opacity-100 rotate-0`, the inactive is `opacity-0` with `rotate-90` or `-rotate-90`

---

### Mobile Menu

- Uses two state variables: `menuOpen` (controls open/close intent) and `menuVisible` (controls DOM presence)
- When `menuOpen` becomes true, `menuVisible` is set true via `useEffect`
- On close, `menuOpen` is set false (triggers exit animation), then `animationend` listener sets `menuVisible` false to unmount
- Container: `absolute top-[60px] left-0 right-0 z-50 bg-white/95 backdrop-blur-md border-b border-gray-200 shadow-lg`
- Gets class `mobile-menu-enter` when opening (slides down from -12px, 0.35s, ease-out-expo) or `mobile-menu-exit` when closing (slides up, 0.25s, ease-in)
- Contains the same 4 nav links (each with `onClick` that calls `handleCloseMenu`) plus the language indicator and green CTA button separated by a `border-t border-gray-200`
- Menu items stagger in via `hero-fade-up` animation with delays: 0.06s, 0.1s, 0.14s, 0.18s, 0.22s (applied via `.mobile-menu-enter > div > *:nth-child(n)` CSS selectors)

---

### Hero Content

- Container: `flex-1 flex flex-col items-center justify-start pt-20 sm:pt-24 md:pt-24 px-4 sm:px-6`

**Heading (animate-hero-1, delay 0.2s):**
- `font-qanelas uppercase text-center tracking-tight text-gray-900`
- Responsive sizing: `text-[2.5rem] leading-[0.95] sm:text-5xl md:text-7xl lg:text-8xl xl:text-[110px]`
- Text: `"Push.Route.Deploy"`

**Subtitle (animate-hero-2, delay 0.45s):**
- `mt-4 sm:mt-5 md:mt-7 text-center text-gray-700 font-medium leading-relaxed max-w-2xl px-2`
- Responsive sizing: `text-sm sm:text-base md:text-lg lg:text-xl`
- Text: `"Get Full Mesh Data Streams, Automatic UDP Hole Punching, Granular Controls, And Many More Cool Tricks!"`

**QR Code Card (animate-hero-3, delay 0.7s):**
- Wrapper: `mt-6 sm:mt-8 md:mt-10 flex flex-col items-center`
- Card: `rounded-xl p-1.5 inline-flex flex-col items-center bg-[#4CAF50]`
- Image: `rounded-lg object-cover` with responsive sizing `w-28 h-28 sm:w-36 sm:h-36 md:w-40 md:h-40`
- Image URL: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260614_015530_9919c38f-0eff-4385-b433-4f14fbf00c73.png&w=1280&q=85`
- Label beneath image: `"Try Now"` in `text-white text-[11px] sm:text-xs font-semibold mt-1.5 sm:mt-2 mb-0.5 tracking-wide`

---

### Scroll Indicator

- Positioned `absolute bottom-4 sm:bottom-6 left-1/2 -translate-x-1/2 animate-bounce`
- Contains a `ChevronDown` icon, `w-5 h-5 sm:w-6 sm:h-6 text-gray-600`

---

### Animations (all in index.css)

All entrance animations use `opacity: 0` as the initial state with `animation-fill-mode: forwards`.

| Class | Keyframes | Duration | Delay | Easing |
|---|---|---|---|---|
| `.animate-nav` | `nav-fade-down` (translateY -20px to 0) | 0.6s | 0.1s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-hero-1` | `hero-fade-up` (translateY 30px to 0) | 0.8s | 0.2s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-hero-2` | `hero-fade-up` | 0.8s | 0.45s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-hero-3` | `hero-fade-up` | 0.8s | 0.7s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.mobile-menu-enter` | `menu-slide-down` (translateY -12px to 0) | 0.35s | 0 | cubic-bezier(0.16, 1, 0.3, 1) |
| `.mobile-menu-exit` | `menu-slide-up` (translateY 0 to -12px) | 0.25s | 0 | cubic-bezier(0.7, 0, 0.84, 0) |

## Cosmic — Hero [sites/cosmic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(25).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cosmic.webp

**Create a full-viewport, dark cinematic hero section for a brand called "COSMIQ." using React, Tailwind CSS, and lucide-react. No other packages. Use Vite + TypeScript.**

---

### Fonts (loaded in `index.html`)

Load these three fonts via `<link>` tags in the `<head>`:

1. **Anton** (Google Fonts) -- used for the logo "COSMIQ." and mobile menu link labels
2. **Inter** weights 300/400/500/600 (Google Fonts) -- used for nav links, share button, and tagline
3. **Black Mustang** (OnlineWebFonts) -- used for the main headline

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Anton&family=Inter:wght@300;400;500;600&display=swap" rel="stylesheet" />
<link href="https://db.onlinewebfonts.com/c/70c5608e6eeb5d6f6fe1b2e5be774ec6?family=Black+Mustang" rel="stylesheet" />
```

Page title: `COSMIQ - What's Beyond`

---

### Layout

- Full viewport: `w-full h-[100dvh] overflow-hidden bg-[#0a0a0a]` with `position: relative`.
- All elements are absolutely/fixed positioned within this container using z-index layering.

---

### Z-Index Layers (bottom to top)

| Z-Index | Element |
|---------|---------|
| 0 | Stars background |
| 1 | Video |
| 5 | Headline text ("WHAT'S BEYOND") |
| 6 | Animated circles |
| 8 | Rock image overlay |
| 10 | Navigation bar |
| 100 | Mobile menu (fixed, full-screen) |

---

### 1. Stars Background (z-0)

- 40 tiny white dots (`div` elements) absolutely positioned across the viewport.
- Sizes alternate: every 3rd dot is 2x2px, others are 1x1px.
- Positions are deterministic: `left: (i * 37 + 13) % 100 %`, `top: (i * 53 + 7) % 100 %`.
- Each star has a `starTwinkle` animation: fades between `opacity: 0.2` and `opacity: 0.8`, duration `2 + (i % 4)` seconds, delay `(i * 0.3) % 3` seconds, infinite, ease-in-out.

---

### 2. Background Video (z-1)

- `<video>` element, absolute, covering full viewport with `object-cover`.
- **Source URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260613_233050_b3f1adc5-8b5f-41bb-b52a-0a79bd796ba8.mp4`
- Attributes: `muted`, `autoPlay`, `playsInline`. Does NOT loop.
- On `loadedMetadata`, set `playbackRate = 1.56`.
- On `ended`, set a `videoEnded` state to `true` (triggers headline animation and rock reveal).

---

### 3. Animated Circles (z-6)

Two concentric circles centered on the viewport (`left-1/2 top-1/2`, translated `-50%, -50%`). Both are `rounded-full border border-white pointer-events-none`.

**Inner circle:**
- Size: `min(95vw, 95vh)`, maxWidth/maxHeight: 1200px.
- Appears after 800ms delay (state-controlled).
- Entrance animation `circleShrink`: 3s, starts at `scale(4) opacity:0`, ends at `scale(1) opacity:0.3`. Easing: `cubic-bezier(0.16, 1, 0.3, 1)`.
- After entrance, plays `circlePulse` infinitely: 6s ease-in-out, gently oscillates between `scale(1) opacity:0.3` and `scale(1.03) opacity:0.4`.

**Outer circle:**
- Size: `min(160vw, 160vh)`, maxWidth/maxHeight: 2200px.
- Entrance animation `circleShrinkOuter`: 3.5s with 0.2s delay, starts at `scale(5) opacity:0`, ends at `scale(1) opacity:0.2`.
- After entrance, plays `circlePulseOuter` infinitely: 8s ease-in-out, oscillates between `scale(1) opacity:0.2` and `scale(1.02) opacity:0.28`.

---

### 4. Headline Text -- "WHAT'S BEYOND" (z-5)

- Positioned with: `bottom-[21vh] sm:bottom-[24vh] md:bottom-[10vh]`, full width, centered horizontally using flex.
- Horizontal padding: `px-2 sm:px-4 md:px-8`.
- `overflow-hidden` wrapper for the reveal effect.
- Font: `"Black Mustang", Anton, sans-serif`.
- Font size: `clamp(2.8rem, 17vw, 23rem)`.
- Letter spacing: `-0.04em`. Line height: `0.85`. White space: `nowrap`. Color: white.
- **Letter-by-letter reveal animation:** Each character is a separate `<span>` with `display: inline-block`. When `videoEnded` is false, each span is at `translateY(120%) opacity:0`. When true, it transitions to `translateY(0) opacity:1`.
  - Transform transition: `0.8s cubic-bezier(0.16, 1, 0.3, 1)` with stagger delay of `i * 60ms`.
  - Opacity transition: `0.5s ease-out` with same stagger delay.
- Spaces are rendered as `\u00A0` with width `0.25em`.

---

### 5. Rock Image Overlay (z-8)

- Absolute, covers full viewport (`inset-0`), `pointer-events-none`.
- Contains an `<img>` with `w-full h-full object-cover block`.
- **Image URL:** `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781394960/rock_vca457.png`
- Fades in when `videoEnded` is true: `opacity 0 -> 1`, transition `0.6s ease-out`.
- This image sits ON TOP of the headline text (z-8 vs z-5), creating a parallax/depth effect where the rock partially covers the bottom of the text.

---

### 6. Navigation Bar (z-10)

- Absolute, top of viewport: `top-0 left-0 right-0`, flex, `items-center justify-between`.
- Padding: `px-5 sm:px-8 md:px-12 py-4 sm:py-6`.
- Fades in: `fadeIn 1s ease-out 0.5s both`.

**Logo (left):**
- Text "COSMIQ." in white, `text-lg`, uppercase, `tracking-widest`.
- Font: `Anton, sans-serif`, letter-spacing: `0.15em`.

**Desktop Nav Links (center, hidden on mobile, visible md+):**
- Three links: "Discover", "Story", "Connect".
- `text-white/70 hover:text-white`, `text-xs`, `tracking-[0.25em]`, uppercase.
- Font: `Inter, sans-serif`. Transition: `colors 300ms`.
- Gap: `gap-10`.

**Right side:**
- Desktop (md+): "Share" text button + `Share2` icon from lucide-react. Same styling as nav links. Icon is `w-4 h-4`.
- Mobile: Hamburger `Menu` icon from lucide-react, `w-5 h-5`, white.

---

### 7. Full-Screen Mobile Menu (z-100, fixed)

Triggered by tapping the hamburger. Uses two states: `menuOpen` (controls mount/unmount) and `menuAnimating` (controls CSS transitions for enter/exit).

**Open:** Set `menuOpen = true`, then on next `requestAnimationFrame` set `menuAnimating = true`.
**Close:** Set `menuAnimating = false`, then after 600ms set `menuOpen = false`.

**Layers:**
1. **Backdrop:** `absolute inset-0 bg-[#0a0a0a]`, opacity transitions over 500ms.
2. **Circle reveal:** A `div` positioned at `top-4 right-5`, `rounded-full bg-white/[0.03]`. Grows from `0` to `300vmax` width/height with `transform: translate(50%, -50%)`. Easing: `cubic-bezier(0.16, 1, 0.3, 1)`, duration 800ms.
3. **Top bar:** Logo "COSMIQ." on left, `X` close icon on right. Fades in with 200ms delay.
4. **Nav links:** Three links ("Discover", "Story", "Connect") in a vertical column. Each is a row with the label on the left and an `ArrowUpRight` icon on the right. Separated by `border-b border-white/[0.08]`.
   - Label font: `Anton, sans-serif`, `text-4xl`, `tracking-tight`. On hover: `tracking-wider`.
   - Icon: `ArrowUpRight` from lucide-react, `w-5 h-5 text-white/30`. On hover: white, translates `+1px right, -1px up`.
   - Staggered entrance: each link has `300 + i * 100 ms` delay, slides up from `translateY(30px)`.
5. **Bottom section:** "Share" button with `Share2` icon, plus a decorative `12px-wide 1px-tall` white/20 line and tagline "Explore the unknown" in `text-[10px] tracking-[0.3em] text-white/30 uppercase`. Appears with 650ms delay.

---

### 8. CSS Keyframes (in `index.css`)

```css
@keyframes circleShrink {
  0% { transform: translate(-50%, -50%) scale(4); opacity: 0; }
  20% { opacity: 0.3; }
  100% { transform: translate(-50%, -50%) scale(1); opacity: 0.3; }
}

@keyframes circleShrinkOuter {
  0% { transform: translate(-50%, -50%) scale(5); opacity: 0; }
  25% { opacity: 0.2; }
  100% { transform: translate(-50%, -50%) scale(1); opacity: 0.2; }
}

@keyframes textReveal {
  0% { transform: translateY(100%); opacity: 0; }
  60% { opacity: 1; }
  100% { transform: translateY(0); opacity: 1; }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes rockFadeIn {
  0% { opacity: 0; transform: translateX(-50%) scale(0.95); }
  100% { opacity: 1; transform: translateX(-50%) scale(1); }
}

@keyframes starTwinkle {
  0%, 100% { opacity: 0.2; }
  50% { opacity: 0.8; }
}

@keyframes circlePulse {
  0%, 100% { opacity: 0.3; transform: translate(-50%, -50%) scale(1); }
  50% { opacity: 0.4; transform: translate(-50%, -50%) scale(1.03); }
}

@keyframes circlePulseOuter {
  0%, 100% { opacity: 0.2; transform: translate(-50%, -50%) scale(1); }
  50% { opacity: 0.28; transform: translate(-50%, -50%) scale(1.02); }
}
```

Also in `index.css`, a global reset:
```css
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { overflow-x: hidden; background: #0a0a0a; }
```

---

### Animation Sequence (Timeline)

1. **0ms** -- Page loads. Stars visible and twinkling. Video begins playing at 1.56x speed.
2. **500ms** -- Nav bar fades in (1s duration).
3. **800ms** -- Circles begin their shrink-in animation (inner 3s, outer 3.5s).
4. **Video ends** -- Headline "WHAT'S BEYOND" reveals letter-by-letter from bottom. Rock image fades in over 0.6s, overlaying the bottom portion of the text for depth.
5. **~3.8s** -- Circles finish entrance, begin gentle infinite pulse.

---

### Dependencies

- `react` ^18.3.1
- `react-dom` ^18.3.1
- `lucide-react` ^0.344.0 (for `Share2`, `Menu`, `X`, `ArrowUpRight` icons)
- `tailwindcss` ^3.4.1
- `vite` ^5.4.2 with `@vitejs/plugin-react`

## CozyPaws — Hero [sites/cozypaws]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/petsArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cozypaws.mp4

### Prompt to Recreate CozyPaws Hero Section

**Build a single-page "CozyPaws" pet store hero section using React, Tailwind CSS, and Lucide React icons. The layout is viewport-height (h-screen), no scroll, with three responsive breakpoints (mobile, tablet md, desktop lg+). Use Vite + TypeScript.**

---

### Fonts (Google Fonts)
- **Inter** (weights: 400, 500, 600) — body/UI text
- **DM Serif Display** (weight: 400) — hero heading only

Load via `<link>` in `index.html`:
```
https://fonts.googleapis.com/css2?family=DM+Serif+Display&family=Inter:wght@400;500;600&display=swap
```

Apply with CSS utility class `.font-serif-display { font-family: 'DM Serif Display', serif; }` and `body { font-family: 'Inter', sans-serif; }`

---

### Color Palette
- Background: `#EFFDF0` (light mint green)
- Primary dark green: `#1a3d1a`
- Hover green: `#2a5a2a`
- Orange accent: `#E86A10`
- Orange hover: `#d45e0d`

---

### Asset URLs (all external, do not download)

| Asset | URL |
|-------|-----|
| Logo SVG | `https://polo-pecan-73837341.figma.site/_assets/v11/0ae29d6d9628bede667f90d57bebe81b8f1ec2bf.svg` |
| Avatar | `https://polo-pecan-73837341.figma.site/_assets/v11/e62173d41f91350a59628e8a9a55ae078a886fb9.png?w=128` |
| Product card (Cat House) | `https://polo-pecan-73837341.figma.site/_assets/v11/3e5158dad63d392ade022e81890edc9f54d750bc.png` |
| Video card (TikTok/YouTube) | `https://polo-pecan-73837341.figma.site/_assets/v11/76be6ec3a93a703b15e9cc01e764a4e3f9d7d2c0.png` |
| Bottom left image | `https://polo-pecan-73837341.figma.site/_assets/v11/8d44b25186ef45a5789c74668fb781cea4e1ff49.png` |
| Bottom center image (tallest) | `https://polo-pecan-73837341.figma.site/_assets/v11/96745c4e72ad5c5208e53a885df797fd82cd854a.png?h=1024` |
| Bottom right image | `https://polo-pecan-73837341.figma.site/_assets/v11/81bd2e7a66b58f3d8f3ad78fd1ebf01af8dfdee1.png` |

---

### Header
- Full-width, `px-12` on desktop, `py-4`, relative z-30
- **Left:** Logo image (205x52px desktop, 130x33px mobile)
- **Center nav (hidden below md):** Links "Home" (text-gray-900), "Shop", "Delivery and payment", "Brands", "Blog" (text-gray-600), text-sm font-medium, gap-8
- **Right:** Search button (circle, border, hidden below sm), Favorites button (orange circle, white star icon, badge "4"), Cart button (circle, border, cart icon, badge "1"), Avatar (circle, 40x40)
- Badges: absolute -top-1 -right-1, 20x20, bg-orange, border-2 border-background, white text 10px bold

---

### Desktop Hero Layout (lg+)

**Text layer (z-5):** Centered, `px-12 pt-[5.4rem]`
- Heading: `font-serif-display`, color `#1a3d1a`, `text-[clamp(60px,7.5vw,110px)]`, `leading-[0.95]`, tracking-tight
- Text reads: "Everything" (line 1), "Your Pets Love" (line 2)
- Each word is an `inline-block` with staggered `animate-word-pop` animation

**Left product card:** Absolutely positioned `top-[50px] left-12`
- Width: `clamp(160px,14vw,260px)`
- Image: aspect-ratio 260/257, rounded-2xl, overflow-hidden
- Arrow button bottom-right corner (dark green circle, ArrowUpRight icon)
- Text below: "Cozy Cat House" in gray-700, "$49.99" in dark green bold
- Responsive font sizes via clamp

**Right video card:** Absolutely positioned `top-[50px] right-12`
- Width: `clamp(120px,10vw,177px)`
- Image: aspect-ratio 177/287, rounded-2xl
- Play button (dark green circle) centered near bottom
- Text below play button: "Watch Product Reviews on TikTok and YouTube"

**Bottom 3 images:** Absolutely positioned `bottom-0 left-0 right-0`, z-10, flex items-end, no gaps
- Left image: `flex-1`, max-height `min(70vh, 55vw)`
- Center image: `flex-[1.265]` (wider), max-height `min(85vh, 70vw)`
- Right image: `flex-1`, max-height `min(70vh, 55vw)`
- All images: `w-full h-auto block`

**Overlays on bottom images:**
- Left: "98K+" stat with avatar stack (avatar + green circle with Plus icon)
- Center: "Best Products for Your Pet" white heading + "Explore Products" orange pill button with ArrowRight icon
- Right: "4.6" rating with orange filled Star icon
- All positioned with `bottom: clamp(20px, 4vh, 50px)`

---

### Tablet Layout (md to lg) — Similar to desktop but smaller
- Heading: text-7xl
- Side cards at `top-[80px]`, left-4/right-4, smaller fixed widths (160px/120px)
- Bottom images: same 3-panel flex, maxHeight 60vh/75vh/60vh

---

### Mobile Layout (below md)
- Top section: centered title (36px), subtitle, "Explore Products" button
- Two cards side-by-side (flex, gap-3): product card (aspect-square) + video card (aspect-3/4)
- Stats row: "98K+" with avatars left, divider, "4.6" star right
- Bottom images: same 3-panel flex, no max-height constraint

---

### Animations (CSS keyframes, custom classes)

| Class | Keyframe | Duration | Easing |
|-------|----------|----------|--------|
| `.animate-fade-up` | 0→30px translateY, 0→1 opacity | 0.8s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-fade-in` | 0→1 opacity | 0.6s | ease-out |
| `.animate-slide-up` | 0→60px translateY | 0.9s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-slide-in-left` | -40px→0 translateX | 0.8s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-slide-in-right` | 40px→0 translateX | 0.8s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-text-reveal` | translateY(40px) skewY(3deg) blur(4px) → none | 1s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-word-pop` | translateY(60px) scale(0.7) rotate(-4deg) blur(8px) → bounce overshoot → settle | 0.9s | cubic-bezier(0.34, 1.56, 0.64, 1) |
| `.animate-scale-in` | scale(0.85)→1 | 0.7s | cubic-bezier(0.16, 1, 0.3, 1) |
| `.animate-photo-reveal` | translateY(80px) scale(1.02) → normal | 1.1s | cubic-bezier(0.16, 1, 0.3, 1) |

All use `animation-fill-mode: both`. `.animate-word-pop` starts with `opacity: 0`.

**Delay classes:** `.delay-100` through `.delay-1200` in 100ms increments.

---

### Stagger Order
1. Header fades in (100-300ms)
2. Hero heading words pop in (200-600ms stagger)
3. Side cards slide in (600-700ms)
4. Bottom photos reveal upward (600-900ms stagger, center first)
5. Overlay stats/buttons pop in (1000-1200ms)

---

### Key Technical Details
- Container: `h-screen flex flex-col overflow-hidden` (no scrolling)
- Header: `shrink-0`
- Hero section: `flex-1 flex flex-col overflow-hidden`
- All responsive layouts use show/hide (`hidden lg:flex`, etc.), not CSS-only media queries
- Extensive use of `clamp()` for fluid typography and spacing
- Lucide icons used: Search, ShoppingCart, Star, ArrowUpRight, Play, ArrowRight, Plus

## Creative Portfolio — Hero [sites/creative-portfolio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(89).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/creative-portfolio.webp

626f6c742d63632d6167656e74Let me read all the relevant files to compile the full recreation prompt.
Here's the full recreation prompt:

---

### Prompt to Recreate This Hero Section

Build a full-screen hero section for a creative portfolio using React, Vite, Tailwind CSS, and the Figtree Google Font. The page has two components: a **Navbar** and a **Hero**. No additional packages beyond `lucide-react` (though it's not used here). The entire page is black background with white text.

---

### Setup

- **Font:** Figtree (400, 500, 600) from Google Fonts, loaded in `index.html`
- **Tailwind custom breakpoints (max-width based):**
  - `mobile`: max 809.98px
  - `md-tablet`: min 810px, max 1199.98px
- **CSS variable:** `--ease-spring: cubic-bezier(0.16, 1, 0.3, 1)`

---

### Video Background

Three full-screen looping videos (muted, autoPlay, playsInline, loop) stacked absolutely with crossfade switching. All three render simultaneously; only the active one has `opacity-100`, the others have `opacity-0` with `transition-opacity duration-[1200ms] ease-in-out`.

**Video URLs (CloudFront):**
1. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260629_030107_874273ea-684a-4e90-bb96-8fdfde48d53d.mp4`
2. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260629_032424_3c9c2a9d-807b-4482-80e6-dd6d9dfd4545.mp4`
3. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260627_094019_4214ea73-b963-46a4-8327-61489192de99.mp4`

**Preloading:** On mount, fetch all videos as blobs and create object URLs for instant playback. Fall back to original URL on failure.

A `bg-black/10` overlay sits above videos at `z-[1]`.

---

### Navbar (absolute positioned, z-10, on top of hero)

- **Layout:** Centered container, max-width 1340px, `py-9 px-[15px]`
- **Left side:** Navigation items formatted as `01 / Works`, `02 / Services`, `03 / About`, `04 / Contact`
  - Index number: `text-[8px] leading-3 tracking-[-0.08px] font-medium uppercase`
  - Label: `text-xs leading-4 tracking-[-0.12px] font-medium uppercase`
  - Each link has a `.nav-link-underline` effect (underline slides in from right on hover via `scaleX` transform)
- **Right side (aligned right):** Email `Davies@gmail.com` and live clock showing `CUP HH:MM:SS` (24h format, updates every second using `Intl.DateTimeFormat('en-GB')`)
- **Mobile:** Nav items hidden, replaced by a `Menu`/`Close` toggle button. Mobile panel uses CSS Grid `grid-rows-[0fr]`/`grid-rows-[1fr]` transition (420ms, spring ease) for smooth expand/collapse. Mobile nav links are large: `text-[28px] leading-8 tracking-[-0.84px]`

---

### Hero Content (z-[2], relative)

Container: `max-w-[1340px]`, full height, flex column, `justify-end items-end`, `gap-[150px]`, `pt-[190px] px-[15px]`

**Section 1 - Video Switcher + Availability (upper area):**
- Left column (`flex-[4]`): Three buttons labeled `01 / WATER WAVE`, `02 / GRIDWAVE`, `03 / LIGHT TUNNEL`. Active button is full opacity, inactive is `opacity-55` with `hover:opacity-75`. On click, sets `activeIndex` to crossfade videos. Each has a `.role-link` class that translates 4px right on hover.
- Right column (`flex-1`): Pulsing dot + "Available for work" text. Dot is 7px circle with glow shadow and infinite pulse animation (scale 1 to 1.45, opacity 1 to 0.45, 1.6s). On slide 1, dot is `#F598F2` pink with pink glow. On slides 2-3, dot is white with white glow.

**Section 2 - Name + CTA (bottom area, pb-[60px]):**
- Left column (`flex-[2]`): Giant name "Viktor." in `text-[200px] leading-[81%] tracking-[-6px] font-medium uppercase`. The period is accent-colored: pink `#F598F2` on slide 1, white on slides 2-3. Animate in with `revealUp` (translateY 80px to 0, 0.9s spring ease).
- Right column (`flex-1`, `pl-[50px]`): Paragraph text ("I craft bold brands and modern websites with purpose...") at `text-base leading-6 tracking-[-0.16px] font-medium`. Below it, a "start a project" button (lowercase) with white border. Button has a fill-up hover effect: `::before` pseudo-element with `#F598F2` background that translateY from 101% to 0 on hover, text turns black, border turns pink. Both animate in with `revealRight` (translateX 100px to 0, 0.9s spring ease), button delayed by 0.08s.

**Reveal animations** trigger once via IntersectionObserver at 0.35 threshold.

---

### Responsive Tablet (810px-1199px)
- Navbar: `py-[30px] px-[18px]`, nav gaps shrink to `gap-4`
- Hero name: `text-[129.6px] leading-[113.4px] tracking-[-7.7px]`
- Bottom section: gap 28px, pb 52px, left padding 24px

### Responsive Mobile (<810px)
- Navbar: `py-6 px-[18px]`, desktop nav hidden, hamburger menu shown
- Hero content: `justify-end items-start gap-[72px] pt-[140px] px-[18px]`
- Switcher + availability stack vertically with `gap-7`
- Bottom section: column layout, `gap-8 pb-11`
- Name: `text-[clamp(68px,21vw,80px)] leading-[96px] tracking-[-4.8px]`
- Paragraph: `max-w-[420px]`

---

### Custom CSS Animations

```css
@keyframes videoFadeIn { from { opacity: 0 } to { opacity: 1 } }
@keyframes revealUp { from { opacity: 0; transform: translateY(80px) } to { opacity: 1; transform: translateY(0) } }
@keyframes revealRight { from { opacity: 0; transform: translateX(100px) } to { opacity: 1; transform: translateX(0) } }
@keyframes dotPulse { 0%,100% { opacity:1; transform:scale(1) } 50% { opacity:0.45; transform:scale(1.45) } }
```

### Accessibility
- `prefers-reduced-motion: reduce` disables all animations
- Semantic landmarks: `<header>`, `<main>`, `<nav>`, `<section>`
- ARIA labels on navigation regions and status elements
- Videos are `aria-hidden="true"`

## Cursor Follow — Hero [sites/cursor-follow]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(48).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cursor-follow.webp

Build a fullscreen hero section for a site called "Orbis.Nft" using React, TypeScript, Tailwind CSS, and Vite. Recreate every detail below precisely.

---

### Video Background with Mouse-Scrub Effect

Use this video as the fullscreen background:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260525_095441_eb28d7e5-72cf-4336-a4cd-543f46f4ff20.mp4
```

The video does NOT autoplay. Instead, implement a **mouse-scrub interaction**: as the user moves their mouse left/right across the viewport, the video scrubs forward/backward through its timeline. Implementation details:

- The video is paused on load at `currentTime = 0`.
- Track the mouse's horizontal position as a normalized value (0 to 1) across `window.innerWidth`.
- On each `mousemove`, compute the delta from the previous X position. Multiply that delta by a `SENSITIVITY` constant of `0.8` and by the video's `duration` to get a time offset.
- Maintain a `targetTime` that accumulates these offsets, clamped between 0 and `duration`.
- Use the video's `seeked` event to chain seeks: when a seek completes, if `targetTime` has diverged from `currentTime` by more than 0.01s, seek again. This prevents dropped seeks since the browser can only process one seek at a time.
- Use a `useRef` to store mutable state (`targetTime`, `isSeeking` flag, `prevX`) to avoid re-renders.
- The `<video>` element has attributes: `muted`, `playsInline`, `preload="auto"`, and is styled `absolute inset-0 h-full w-full object-cover`.

---

### Google Fonts

Load two Google Fonts in `index.html` via `<link>`:
```
https://fonts.googleapis.com/css2?family=Anton&family=Condiment&display=swap
```
- **Anton** -- used for the hero heading (mapped to Tailwind as `font-grotesk`).
- **Condiment** -- a cursive script used for the accent text (mapped as `font-condiment`).

Include `<link rel="preconnect">` tags for `fonts.googleapis.com` and `fonts.gstatic.com` (with `crossorigin`).

---

### Tailwind Config

Extend the default Tailwind theme with:
- **Colors:**
  - `background`: `#010828` (deep navy)
  - `cream`: `#EFF4FF` (off-white for heading text)
  - `neon`: `#6FFF00` (bright green for the cursive accent)
- **Font families:**
  - `grotesk`: `['Anton', 'sans-serif']`
  - `condiment`: `['Condiment', 'cursive']`

---

### Global CSS (`index.css`)

```css
body {
  background-color: #010828;
  color: #EFF4FF;
  margin: 0;
  overflow-x: hidden;
}
```

Also include a `.liquid-glass` utility class (not used in the hero itself, but part of the design system):
- `background: rgba(255, 255, 255, 0.01)` with `background-blend-mode: luminosity`
- `backdrop-filter: blur(4px)` (with `-webkit-` prefix)
- `border: none`
- `box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1)`
- A `::before` pseudo-element creating a gradient border effect using a `mask-composite: exclude` technique. The gradient goes from `rgba(255,255,255,0.45)` at top/bottom to transparent in the middle, with `padding: 1.4px`.

---

### Navbar

A `<nav>` fixed to the top (`fixed top-0 left-0 right-0 z-50`), using `flex items-center justify-between`, with padding `px-5 sm:px-8 py-4 sm:py-5`.

**Left: Logo (inline SVG)**
A custom geometric SVG logo, 28x28, viewBox `0 0 256 256`, filled `#111111`:
```
M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 96 95 L 63.5 128 L 64 128 L 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 64 L 64 0 L 192 0 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z
```

**Center: Desktop pill navigation (hidden on mobile, `hidden md:flex`)**
Absolutely centered with `absolute left-1/2 -translate-x-1/2`. Dark pill container: `bg-gray-900 rounded-full px-2 py-1.5`. Contains 5 nav items: `['Device', 'Real Stories', 'Science', 'Plans', 'Reach Us']`. The first item is the active state: `bg-white text-gray-900 text-sm font-medium px-4 py-1.5 rounded-full`. All others: `text-gray-300 text-sm font-medium px-4 py-1.5 rounded-full hover:text-white transition-colors`.

**Right: Desktop CTA button (hidden on mobile, `hidden md:flex`)**
`bg-gray-900 text-white text-sm font-medium px-5 py-2 rounded-full` with `hover:bg-gray-700 transition-colors`. Contains a small green dot (`w-2 h-2 rounded-full bg-green-400`) followed by text "Reserve Yours".

**Mobile: Hamburger toggle (`md:hidden`)**
Uses `Menu` and `X` icons from `lucide-react` at `size={22}`, colored `text-gray-900`. Toggles a dropdown.

**Mobile dropdown menu**
When open: `fixed top-0 left-0 right-0 z-40 bg-white pt-16 pb-6 px-5 shadow-lg flex flex-col gap-1 md:hidden`. Each nav item is a full-width button: `text-gray-800 text-base font-medium py-3 border-b border-gray-100 text-left hover:text-gray-500 transition-colors`. Includes the same "Reserve Yours" CTA at the bottom with `mt-4`, centered, `rounded-full`.

---

### Hero Text (Bottom-Left)

Positioned inside a `relative z-10 flex flex-col h-full` container. The text block is anchored to the bottom: `flex-1 flex items-end pb-16 sm:pb-20 lg:pb-24 px-6 lg:px-12`. Inner wrapper: `relative lg:ml-12 max-w-[780px]`.

**Main heading `<h1>`:**
- Font: `font-grotesk` (Anton)
- Size: `text-[40px] sm:text-[60px] md:text-[75px] lg:text-[90px]`
- `uppercase`, color `text-cream` (#EFF4FF)
- Line height: `leading-[1.05] sm:leading-[1] md:leading-[1] lg:leading-[1]`
- Text content (with line breaks):
  ```
  Beyond earth
  and ( its ) familiar
  boundaries
  ```
  The parentheses around "its" have spaces inside them: `{'( '}its{' )'}`.

**Cursive accent `<span>`:**
- Absolutely positioned relative to the heading wrapper: `absolute -right-4 sm:right-0 md:right-4 top-0 sm:top-2 md:top-4`
- Font: `font-condiment` (Condiment cursive)
- Size: `text-[24px] sm:text-[32px] md:text-[40px] lg:text-[48px]`
- Color: `text-neon` (#6FFF00)
- Slight rotation: `-rotate-1`
- `opacity-90`
- Inline style: `mixBlendMode: 'exclusion'`
- Text: "Nft collection"

---

### Overall Layout

The root `<section>` is `relative h-screen w-full overflow-hidden bg-background`. The video sits at `absolute inset-0` behind everything. The content layer sits at `relative z-10`. The nav is `fixed z-50`.

The page title in `index.html` is "Orbis.Nft".

---

### Dependencies

- `react`, `react-dom` (v18)
- `lucide-react` (for Menu and X icons)
- Tailwind CSS 3, PostCSS, Autoprefixer
- Vite with `@vitejs/plugin-react`
- TypeScript

No other UI libraries needed.

## Cyberpunk Reveal — Hero [sites/cyberpunk-reveal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(31).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cyberpunk-reveal.webp

Recreate a cyberpunk-style hero section (React + Vite + TypeScript + Tailwind CSS, lucide-react icons)**

Build a single-page app with a full-screen hero section in `src/App.tsx`. Stack: React 18, Tailwind CSS, lucide-react (icons only, `Menu` and `X`). No other packages.

### Assets

```
BG_IMAGE_1 (base, red-tinted portrait):
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_125121_afb71ce9-9c64-4c54-90b5-c89c0764c052.png&w=1920&q=85

BG_IMAGE_2 (alternate reveal image):
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_135737_0da59642-725b-451a-997b-b0283d95a42a.png&w=1280&q=85
```

### Fonts (index.css)

- Google Fonts import: `JetBrains Mono` weights 300–800 plus italic 400/500.
- Apply globally: `* { font-family: 'JetBrains Mono', monospace; }` and a `.font-helvetica-neue` class that also resolves to JetBrains Mono.
- Also declare a `@font-face` for 'Helvetica Neue Roman' from `/fonts/HelveticaNeue-Roman.woff2` / `.woff` (weight 400, `font-display: swap`) — declared but the mono font is what renders.
- Root wrapper div: `min-h-screen bg-white tracking-[-0.02em]` with inline `fontFamily: "'JetBrains Mono', monospace"`.

### Navbar (fixed, z-50)

`<nav className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between md:justify-center p-4 sm:p-5">`

- **Desktop (md+): ONE centered pill** — `bg-black/60 backdrop-blur-md rounded-full pl-3 pr-2 py-2 flex items-center gap-1` containing, in order: a 22x22 white SVG logo (geometric angular mark, viewBox 0 0 256 256, path: `M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 96 95 L 63.5 128 L 64 128 L 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 64 L 64 0 L 192 0 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z`); text link buttons "Module" (white, active), "Case Records", "Biotech", "Tiers", "Live Demo" (gray-300, `text-sm font-medium px-3 py-1.5 rounded-full hover:bg-white/10 hover:text-white`); and a white CTA "Connect" (`bg-white text-gray-900 text-sm font-semibold px-5 py-1.5 rounded-full hover:bg-gray-100 ml-1`).
- **Mobile (<md):** logo in its own black/60 blurred pill (left) and a hamburger toggle pill (right) using lucide `Menu`/`X` size 22. Toggling opens a full-width white dropdown (`fixed top-0 z-40 pt-16 pb-6 px-5 shadow-lg`) listing the 5 links (gray-800, `py-3 border-b border-gray-100`) plus a dark "Connect" pill button; tapping a link closes it.

### Hero section (100dvh, `relative overflow-hidden`)

Layers bottom to top:

1. **Grid background (z-0):** full-size SVG at `opacity: 0.1`, a 48px square `<pattern>` of grid lines (`stroke #64748b`, width 0.6). The pattern's x/y offset follows the mouse with parallax: target = (normalized cursor position − 0.5) × 16px, eased at 0.06 lerp per frame.
2. **Base image (z-10):** `bg-center bg-cover` div with BG_IMAGE_1, with a Ken Burns intro: `@keyframes kenBurns { from { transform: scale(1.12) } to { scale(1) } }`, 2.4s `cubic-bezier(0.22,1,0.36,1)` forwards.
3. **Cursor spotlight reveal layer (z-30):** a second `bg-cover` div with BG_IMAGE_2, masked by a canvas-generated radial gradient that follows the smoothed cursor. Implementation: a hidden full-window canvas; every frame, clear it and fill a circle (radius 260px) at the cursor with a radial gradient (stops: 0→1 opacity, 0.4→1, 0.6→0.75, 0.75→0.4, 0.88→0.12, 1→0), export via `toDataURL()` and set as `mask-image`/`-webkit-mask-image` (`mask-size: 100% 100%`) on the image div. Cursor smoothing: rAF loop with lerp factor 0.1 toward real mouse position, starting offscreen at (−999,−999).
4. **Stats on a fading circular arc (z-50, hidden below sm):** container `absolute inset-y-0 right-0 pointer-events-none`, SVG `viewBox="0 0 380 700"` `preserveAspectRatio="xMaxYMid meet"` `class="h-full w-auto"`. Concentric arcs centered at `(-110, 300)` (off-canvas left, so arcs sweep in from the subject). Data:
   - r=330, arc from −92° to 16°, dot at −46°, stat "10+" / "YEARS REAL"
   - r=395, arc −56° to 60°, dot at 2°, stat "40+" / "USE FORMS"
   - r=460, arc −14° to 72°, dot at 44°, stat "95%" / "REPEAT MEMBERS"

   Each arc is a path (`A r r 0 0 1`) stroked at 1.1 with a per-arc `userSpaceOnUse` linearGradient from arc start point to end point, white with stop-opacities 0 → 0.5 (22%) → 0.5 (55%) → 0.1 (85%) → 0 (100%) so both ends fade out. At each dot position (polar from center): a filled white circle r=3.4, a white ring r=7 at 35% stroke opacity, the number at dot+(16,4) in white 32px (suffix as `<tspan>` 19px raised `dy="-10"`, letter-spacing −1px), and the uppercase label at dot+(18,22), 8.5px, weight 600, letter-spacing 2px, 80% opacity.
5. **Hero text block (z-50):** `absolute bottom-12 sm:bottom-16 md:bottom-24 left-5 sm:left-8 md:left-12 max-w-[300px] sm:max-w-md`:
   - Eyebrow: `Gateway to your *augmented self*` (italic span), `text-[11px] sm:text-xs font-semibold tracking-[0.12em] text-white/90`
   - H1: `A window / of coming / enhancements` (manual `<br/>`), `text-4xl sm:text-5xl md:text-6xl leading-[1.05] tracking-[-0.08em] text-white`
   - Paragraph: "A future where carbon fiber, titanium, and human instinct align. Not machine. Not human. Something wonderfully poised between." `text-sm sm:text-base text-white/90 leading-relaxed`
   - CTA "Reserve Now": white pill `px-7 sm:px-8 py-3 sm:py-3.5 rounded-full shadow-lg shadow-black/20`, hover `scale-[1.04]`, active `scale-95`, plus a shine sweep: an absolutely-positioned gradient span (`from-transparent via-white/60 to-transparent`) translating from `-translate-x-full` to `translate-x-full` over 700ms on group hover.

### Animations (CSS keyframes in index.css)

- `.hero-rise`: from `opacity:0; translateY(26px); blur(8px)` to clear; 0.95s `cubic-bezier(0.22,1,0.36,1)` forwards; staggered inline delays — eyebrow 0.15s, H1 0.3s, paragraph 0.5s, button 0.7s.
- `.nav-drop`: from `opacity:0; translateY(-18px)`; 0.8s same easing, 0.1s delay; applied to all nav pills.
- `.arc-line`: stroke-draw via `stroke-dasharray/offset` set to the arc length (computed `r × Δangle(rad)`, passed as CSS var `--len`), animating offset to 0 over 1.6s `cubic-bezier(0.65,0,0.35,1)`; delays staggered `0.4s + i × 0.22s`.
- `.arc-dot`: `popIn` overshoot (scale 0.4 → 1.25 → 1) 0.55s back-out easing, delay = lineDelay + 0.9s; needs `transform-box: fill-box; transform-origin: center`.
- `.arc-ring`: infinite `pulseRing` (scale 1→1.45, opacity 0.35→0) 2.8s ease-in-out, delay markDelay + 0.3s.
- `.arc-text`: simple fadeIn 0.7s, number at markDelay + 0.15s, label + 0.3s.
- Wrap all of the above in `@media (prefers-reduced-motion: reduce)` resetting animation/opacity/transform/dashoffset.

### Behavior notes

- All mouse-driven effects run in one `requestAnimationFrame` loop; clean up listener and rAF on unmount.
- Stats arc is decorative: `pointer-events-none`, hidden on mobile (`hidden sm:block`).
- Everything must remain readable: white text over the red imagery.

## Cybersecurity Hero — Hero [sites/cybersecurity-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(60).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cybersecurity-hero.webp

Build a **single-page React + TypeScript (Vite)** landing hero for a product called **"Xero"** that recreates the following section exactly. Use the **Inter** Google Font (weights 300, 400, 500, 600, 700, 800). Do not use Tailwind utility classes for the hero — write plain CSS in a global stylesheet. No purple/indigo branding outside the specified pink-magenta gradient arc.

### Layout & Structure

Render three top-level blocks centered on a black page (`#0a0a0f`), each constrained to `max-width: 1600px`, in this vertical order:

1. **`<nav>`** — sticky-style top bar (not actually sticky, just at top)
2. **`<section class="hero-card">`** — the rounded dark hero card with the animated icon pipeline
3. **`<div class="brands">`** — a row of 5 monochrome brand logos

The body uses `display: flex; flex-direction: column; align-items: center; padding: 14px;` and `font-family: 'Inter', sans-serif;`.

### CSS Variables (on `:root`)
```
--bg: #0a0a0f;
--surface: #111118;
--text: #f0f0f5;
--text-muted: #8888a8;
--accent: #c8a0e0;
--accent-pink: #b04090;
--border: rgba(255, 255, 255, 0.08);
```

### NAVBAR

- Grid layout: `grid-template-columns: 1fr auto 1fr; padding: 12px 24px; margin-bottom: 14px;`
- **Left**: `<span class="nav-logo">Xero</span>` — `font-size: 1.05rem; font-weight: 700; letter-spacing: -0.01em;`
- **Center**: `<ul class="nav-links">` with three `<a>` items: **Method**, **Pricing**, **Docs**. Color `--text-muted`, `font-size: 0.85rem`, gap 32px, hover transitions to `--text` over 0.2s.
- **Right**: `<div class="nav-actions">` containing two pill buttons:
  - `.btn-login` — `rgba(255,255,255,0.06)` bg, 1px border `--border`, white text, padding `7px 18px`, `border-radius: 999px`, `font-size: 0.82rem`, `font-weight: 500`. Hover: bg `rgba(255,255,255,0.12)`.
  - `.btn-signup` — solid white bg, black `#0a0a0f` text, same dimensions, `font-weight: 600`. Hover: `opacity: 0.88`.
- The `.nav-menu` wrapper uses `display: contents` on desktop so the `ul` and actions become direct grid children.

### Mobile (≤ 768px)
- Nav becomes flex with space-between.
- A `.menu-toggle` hamburger appears: 24×14 button with two 2px-tall white spans. When `.active`, span 1 rotates `translateY(6px) rotate(45deg)` and span 2 rotates `translateY(-6px) rotate(-45deg)` to form an X.
- `.nav-menu.active` slides in from `right: -100%` to `right: 0` over 0.4s `cubic-bezier(0.4, 0, 0.2, 1)` as a full-screen `var(--bg)` overlay with column-stacked links and full-width buttons.
- Toggling sets `document.body.style.overflow = 'hidden'`.

### HERO CARD

Outer `.hero-card` styles:
- `width: 100%; max-width: 1600px; border-radius: 20px; border: 1px solid rgba(255,255,255,0.07); overflow: hidden; position: relative; background: #0d0b12; padding: 80px 40px 70px; min-height: 640px;`
- `display: flex; flex-direction: column; align-items: center; text-align: center;`

### `::before` Gradient Arc (the signature visual)
A radial gradient positioned at `50% -70%` with **many manually-tuned stops** producing a smooth dark→pink→white arc near the top:
```
background:
  radial-gradient(circle at 50% -70%,
    transparent 60%,
    rgba(176,48,136,0.03) 63%,
    rgba(176,48,136,0.08) 65%,
    rgba(176,48,136,0.16) 67%,
    rgba(176,48,136,0.28) 69%,
    rgba(176,48,136,0.40) 71%,
    rgba(176,48,136,0.52) 73%,
    rgba(176,48,136,0.64) 75%,
    rgba(176,48,136,0.74) 77%,
    rgba(176,48,136,0.82) 79%,
    rgba(210,70,175,0.92) 85%,
    rgba(240,110,210,0.88) 87%,
    rgba(255,205,250,0.92) 91%,
    rgba(255,240,255,0.98) 93%,
    #ffffff 95%),
  radial-gradient(circle at 50% 35%, rgba(120,40,180,0.08) 0%, transparent 50%);
z-index: 0; pointer-events: none;
```

### `.hero-grid` Overlay
A separate absolutely-positioned div with crosshatch grid:
```
background-image:
  linear-gradient(rgba(255,255,255,0.07) 1px, transparent 1px),
  linear-gradient(90deg, rgba(255,255,255,0.07) 1px, transparent 1px);
background-size: 40px 40px;
mask-image: radial-gradient(circle at 50% -70%, transparent 60%, black 78%);
```
This makes the grid only visible inside the arc area.

### ICON PIPELINE (the animated centerpiece)

Container `.icon-pipeline`: `position: relative; display: flex; align-items: center; justify-content: center; max-width: 700px; margin-bottom: 52px; z-index: 1;`

Children in this exact order:

1. **`<svg class="beam-svg">`** — absolutely-positioned over the whole pipeline (`overflow: visible`), containing:
   - A `<filter id="glow">` with `feGaussianBlur stdDeviation="2"` then `feComposite ... operator="over"`.
   - A `<linearGradient id="beam-gradient" gradientUnits="userSpaceOnUse">` with stops:
     - `0%` `#b04090` opacity 0
     - `20%` `#b04090` opacity 0.8
     - `50%` `#fff` opacity 1
     - `80%` `#c8a0e0` opacity 0.8
     - `100%` `#c8a0e0` opacity 0
   - Two `<path>` elements both stroked with `url(#beam-gradient)`:
     - Glow path: `stroke-width="2"`, `filter="url(#glow)"`, `opacity: 0.6`.
     - Core path: `stroke-width="0.8"`.

2. **Left node** `.icon-node.node-light-right` (id `node-stack`) — Lucide-style **layers** SVG (3 stacked diamonds): `<polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/>`.

3. **`.pipeline-line`** — `width: 160px; height: 1px;` linear gradient `90deg, rgba(255,255,255,0.15), rgba(255,255,255,0.07)`.

4. **Center wrapper** with `position: relative;` containing:
   - **`.splash`** — 100×100 absolutely centered, `border-radius: 50%`, `background: radial-gradient(circle, rgba(255,77,200,0.6) 0%, transparent 70%)`, initial `opacity: 0; transform: scale(0.4); z-index: 2;`
   - **`.icon-node-center`** (id `node-x`) — 64×64 round, `background: #1e1e2c`, neumorphic shadow (see below), containing the **Xero "X" logoipsum** SVG (`viewBox="0 0 40 40"`) — the multi-cut path provided in the source.

5. **`.pipeline-line.right`** — same 160×1 line, gradient reversed.

6. **Right node** `.icon-node.node-light-left` (id `node-shield`) — Lucide-style **shield-check** SVG: `<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/>`.

### Side Node Styling
`.icon-node`: 46×46 round, `background: #1a1a24`, `cursor: pointer`, `z-index: 3`, with **neumorphic** shadow stack:
```
box-shadow:
  6px 6px 12px rgba(0,0,0,0.4),
  -4px -4px 10px rgba(255,255,255,0.03),
  inset 1px 1px 1px rgba(255,255,255,0.05),
  inset 4px 4px 8px rgba(0,0,0,0.4);
```
Plus an `::after` dotted outer ring at `inset: -7px` (`border: 1px dotted #1a1a24`).
Hover: `translateY(-1px)` and stronger shadows. Active: inset-only shadows.
Inner SVG: 20×20, stroke `rgba(255,255,255,0.7)`, `stroke-width: 1.5`, fill none, round caps.

### Center Node Styling
`.icon-node-center`: 64×64, `background: #1e1e2c`, similar but stronger neumorphic shadow:
```
8px 8px 16px rgba(0,0,0,0.5),
-6px -6px 14px rgba(255,255,255,0.04),
inset 1px 1px 2px rgba(255,255,255,0.06),
inset 6px 6px 12px rgba(0,0,0,0.5);
```
Inner Xero SVG: 28×28, `fill: white`.

### Side-Light Glows
- `.node-light-right::before` — half-circle radial glow on the right side: `radial-gradient(circle at right, rgba(200,200,200,0.45) 0%, transparent 70%)`, `opacity: 0` default, `opacity: 1` when `.active` (300ms transition).
- `.node-light-left::before` — same but on left, color `rgba(200,100,255,0.5)`.

### Splash Keyframe
```
@keyframes splash-anim {
  0%   { transform: scale(0.4); opacity: 0.8; }
  40%  { opacity: 0.6; }
  100% { transform: scale(1.4); opacity: 0; }
}
```
Triggered by adding `.animate` (0.8s ease-out forwards).

### BEAM ANIMATION (JavaScript / requestAnimationFrame)

Implement a state machine with four phases. On mount and on every window `resize`, recompute the SVG path:

```
const pRect = pipeline.getBoundingClientRect();
const sRect = nodeStack.getBoundingClientRect();
const xRect = nodeX.getBoundingClientRect();
const shRect = nodeShield.getBoundingClientRect();
const startX = sRect.left + sRect.width/2 - pRect.left;
const startY = sRect.top  + sRect.height/2 - pRect.top;
// midX/midY from nodeX, endX/endY from nodeShield
const d = `M ${startX},${startY} L ${midX},${midY} L ${endX},${endY}`;
```
Set this `d` on **both** beam paths.

The gradient is animated by mutating `x1` / `x2` of `#beam-gradient` (in `userSpaceOnUse`) so the bright window slides along. Use `halfWidth = 5` (percentage units), `center = percentage * 100`:
```
gradient.x1 = (center - 5) + '%'
gradient.x2 = (center + 5) + '%'
y1 = y2 = '0%'
```

State machine in a `requestAnimationFrame` loop, tracking `lastStateChange` timestamp:

| State | Duration | Behavior |
|---|---|---|
| **`p1`** | 800 ms | `percentage` interpolates `0 → 0.5`. While `p < 0.4`, add `.active` to `node-stack`; remove after. At end: switch to `splash`, hide both beam paths (`opacity: 0`), add `.animate` to splash. |
| **`splash`** | 800 ms | Wait. After elapsed: switch to `p2`, remove `.animate`, restore `opacity: 1` on both beam paths. |
| **`p2`** | 800 ms | `percentage` interpolates `0.5 → 1.0`. While `p > 0.6`, add `.active` to `node-shield`. At end: remove `.active`, switch to `idle`. |
| **`idle`** | 1000 ms | Wait, then loop back to `p1`. |

Total cycle ≈ 3.4 seconds, infinite.

### HERO TEXT

`.hero-content` `max-width: 620px; z-index: 1;`

```html
<h1 class="hero-heading">
  The simple way
  <strong>encryption your data</strong>
</h1>
<p class="hero-sub">
  Fully managed data encrypting service and annotation<br>
  platform for teams of all industries.
</p>
<a href="#" class="btn-cta">Get Started</a>
```

- `.hero-heading`: `font-size: clamp(2.4rem, 5.5vw, 4rem); font-weight: 300; line-height: 1.1; letter-spacing: -0.02em;`
- `.hero-heading strong`: `display: block; font-weight: 400; margin-top: 4px;` with `background: linear-gradient(to right, #ffffff, #a98597); -webkit-background-clip: text; -webkit-text-fill-color: transparent;`
- `.hero-sub`: 0.9rem, `rgba(255,255,255,0.4)`, `max-width: 440px`, `margin: 0 auto 36px`.
- `.btn-cta`: white pill, black text, `padding: 12px 32px; border-radius: 999px; font-weight: 600;`. Hover: `opacity: 0.9; translateY(-1px)`.

### BRANDS ROW

`.brands`: flex row, `gap: 64px; padding: 32px 24px 10px; flex-wrap: wrap; justify-content: center;`

Five `.brand-item` blocks (each: flex, gap 10, color `rgba(255,255,255,0.35)`, font-size 1.1rem, font-weight 500, white-space nowrap, with a 22×22 SVG):

1. **Expedia** — `<circle cx=12 cy=12 r=10 fill=current /><path fill="var(--bg)" d="M8 9h8v2H8zm0 4h6v2H8z"/>` then text `Expedia`.
2. **asana** — three filled circles: `(12,7,r=4)`, `(5,16,r=3.5)`, `(19,16,r=3.5)`, text `asana`.
3. **zenefits** — three stroked horizontal polylines (lengths 16/8/16) at y=8/12/16, text `zenefits`.
4. **HubSpot** — small filled circle `(15.5,8.5,r=2.5)`, stroked circle `(8.5,8.5,r=2)`, paths connecting them; text `HubSp<span class="hubspot-dot"></span>t` where `.hubspot-dot` is a 6×6 round superscript dot.
5. **loom** — circle `(12,12,r=9)` plus vertical/horizontal/diagonal stroke lines forming a globe-with-X, text `loom`.

### Responsive Breakpoints

- `≤ 860px`: pipeline `gap: 0; margin-bottom: 40px;` `.pipeline-line { width: 80px }`.
- `≤ 768px`: enable mobile hamburger menu, `.icon-node` shrinks to 38×38, `.icon-node-center` to 52×52, `.hero-card { padding: 60px 20px 60px; min-height: auto }`, `.brands { gap: 32px }`.
- `≤ 480px`: `.hero-card { border-radius: 16px }`, `.brands { gap: 24px }`.

### Z-Index Stack (critical for splash/beam layering)

- `0` — gradient arc + grid overlay
- `1` — pipeline container, hero text
- `2` — beam SVG, splash
- `3` — all icon nodes
- `4` — node side-light glows
- `1000-1001` — mobile nav overlay and toggle

Implement all of the above exactly. Use `useRef` for the pipeline, the three nodes, both beam paths, the gradient, and the splash. Use one `useEffect` to set up the resize listener and the `requestAnimationFrame` loop, and clean both up on unmount.

## Cybersecurity Hero v2 — Hero [sites/cybersecurity-hero-v2]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(62).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cybersecurity-hero-v2.webp

**PROMPT:**

Build a dark, premium SaaS landing page hero section for a product called "Xero" — a data encryption service. Use React + TypeScript + Vite + Tailwind CSS + the `shaders` package (`shaders/react`) + `lucide-react`. Font: Inter (weights 300, 400, 500, 600, 700, 800) from Google Fonts.

---

**PAGE STRUCTURE:**

The page has a dark background (`#0a0a0f`) with 14px body padding. Everything is centered in a flex column. The structure is: Navbar > Hero Card > Brand Logos row.

---

**NAVBAR:**

- Full-width, max-width 1600px, using CSS Grid with 3 columns: `1fr auto 1fr`
- Left: Logo text "Xero" — font-size 1.05rem, weight 700, letter-spacing -0.01em, white
- Center: 3 navigation links ("Method", "Pricing", "Docs") — font-size 0.85rem, weight 400, color `#8888a8`, hover to white, 32px gap between links
- Right: Two buttons — "Login" (ghost pill: `rgba(255,255,255,0.06)` background, 1px border `rgba(255,255,255,0.08)`, white text, font-size 0.82rem, weight 500, border-radius 999px, padding 7px 18px) and "Sign Up" (solid white pill: white background, dark text `#0a0a0f`, font-size 0.82rem, weight 600, border-radius 999px, padding 7px 18px)
- Mobile (768px): Hamburger menu toggle (2 spans that animate into an X via translateY/rotate). Full-screen overlay menu slides in from right with `transition: right 0.4s cubic-bezier(0.4, 0, 0.2, 1)`. Links become 1.2rem centered vertically. Buttons become full-width stacked.

---

**HERO CARD:**

- Container: max-width 1600px, border-radius 20px, 1px border `rgba(255,255,255,0.07)`, `overflow: hidden`, position relative, background `#0d0b12`, padding `80px 40px 70px`, flex column centered, min-height 640px, text-align center.

**Layer 1 — Shader Background (z-index 0):**
Position absolute, inset 0, overflow hidden, border-radius 20px, pointer-events none, 100% width/height. Inner div and canvas forced to 100% width/height, position absolute inset 0.

Shader composition (from `shaders/react`):
```jsx
<Shader>
  <SolidColor color="#08071a" />
  <SineWave amplitude={0.36} blendMode="normal-oklch" color="#0582e8" frequency={0.2} position={{ x: 0.65, y: 0.67 }} softness={0.55} speed={0.3} thickness={0.72} />
  <SineWave amplitude={0.17} blendMode="normal-oklch" color="#f00e94" frequency={0.2} position={{ x: 0.6, y: 0.51 }} softness={0.54} speed={0.5} thickness={0.35} />
  <WaveDistortion angle={299} frequency={0.3} speed={0.2} strength={1} />
  <FilmGrain strength={0.07} />
</Shader>
```

**Layer 2 — Radial Gradient Arc (::before pseudo-element, z-index 0):**
A radial-gradient positioned at `circle at 50% -70%`:
- Transparent from 0-60%
- Gradually builds pink/magenta (`rgba(176, 48, 136, ...)`) from 63% to 79% with increasing opacity (0.03 to 0.82)
- Transitions to lighter pink at 85-87% (`rgba(210,70,175,0.92)`, `rgba(240,110,210,0.88)`)
- Near-white at 91-93% (`rgba(255,205,250,0.92)`, `rgba(255,240,255,0.98)`)
- Pure white at 95%
- Second radial gradient: `circle at 50% 35%`, `rgba(120, 40, 180, 0.08)` center, transparent at 50%

**Layer 3 — Grid Overlay (z-index 0):**
Position absolute inset 0. Background: two linear-gradients creating a 40px grid with `rgba(255,255,255,0.07)` 1px lines. Masked with `radial-gradient(circle at 50% -70%, transparent 60%, black 78%)` so the grid only shows where the arc glows.

---

**ICON PIPELINE (z-index 1, margin-bottom 52px):**

A horizontal row of 3 icon nodes connected by lines, with an animated beam traveling between them.

- **Left node** (46px circle, background `#1a1a24`): SVG layers/stack icon (polygon + 2 polylines). Neumorphic box-shadow. Dotted border ring (7px outset). Class `node-light-right` — has a `::before` pseudo with a radial-gradient highlight on the right side that fades in/out (opacity transition 0.3s) when `.active` class is toggled.

- **Center node** (64px circle, background `#1e1e2c`): Custom Xero "X" SVG logo (a circular pinwheel shape, white fill). Larger neumorphic shadows. Wrapped in a container with a `.splash` element — a 100px radial gradient circle (`rgba(255, 77, 200, 0.6)`) that animates scale 0.4 to 1.4 while fading out over 0.8s.

- **Right node** (46px circle): Shield icon with checkmark. Class `node-light-left` — same as left but highlight on the left side with a purple tint (`rgba(200, 100, 255, 0.5)`).

- **Connecting lines**: 160px wide, 1px height, gradient from `rgba(255,255,255,0.15)` to `rgba(255,255,255,0.07)` (reversed for right line).

- **Beam Animation** (requestAnimationFrame loop):
  - SVG overlay with a linearGradient (`#beam-gradient`): 5-stop gradient from transparent pink to white center to transparent purple.
  - Two `<path>` elements use refs — one for glow (strokeWidth 2, filter blur, opacity 0.6) and one crisp (strokeWidth 0.8).
  - Path coordinates computed dynamically from node positions via `getBoundingClientRect()`.
  - Animation states: `p1` (800ms, beam travels 0-50%, left node pulses active at 0-40%), `splash` (800ms pause, beam hidden, center splash animates), `p2` (800ms, beam travels 50-100%, right node activates at 60-100%), `idle` (1000ms pause). Loop repeats.
  - Beam position is controlled by shifting linearGradient x1/x2 attributes.

---

**HERO CONTENT (z-index 1, max-width 620px):**

- **Heading**: `<h1>` with text "The simple way" (weight 300, white) and `<strong>` block "encryption your data" (weight 400, gradient text: `linear-gradient(to right, rgba(255,255,255,1), rgba(255,255,255,0.6))` with background-clip text). Font-size: `clamp(2.4rem, 5.5vw, 4rem)`, line-height 1.1, letter-spacing -0.02em, margin-bottom 24px.

- **Subtitle**: "Fully managed data encrypting service and annotation platform for teams of all industries." — font-size 0.9rem, weight 400, line-height 1.6, color `rgba(255,255,255,0.4)`, max-width 440px, centered, margin-bottom 36px. Has a `<br>` after "annotation".

- **CTA Button**: "Get Started" — white background, dark text, font-size 0.88rem, weight 600, padding 12px 32px, border-radius 999px. Hover: opacity 0.9, translateY(-1px).

---

**BRAND LOGOS ROW (below hero card):**

- Flex row, centered, gap 64px, padding 32px 24px 10px, flex-wrap.
- 5 brand items: Expedia, asana, zenefits, HubSpot (with a superscript dot replacing the "o"), loom.
- Each: flex row, gap 10px, color `rgba(255,255,255,0.35)`, font-size 1.1rem, weight 500.
- Each has a simple 22px SVG icon in matching muted color (geometric/abstract representations, not actual brand logos).

---

**CSS VARIABLES:**
```
--bg: #0a0a0f
--surface: #111118
--text: #f0f0f5
--text-muted: #8888a8
--accent: #c8a0e0
--accent-pink: #b04090
--border: rgba(255, 255, 255, 0.08)
```

---

**RESPONSIVE BREAKPOINTS:**
- 860px: Pipeline lines shrink to 80px
- 768px: Body padding 10px, hamburger menu activates, hero card padding 60px 20px, pipeline margin-bottom 32px, nodes shrink (38px/52px), `<br>` tags hidden, brands gap 32px
- 480px: Hero card border-radius 16px, brands gap 24px

---

**DEPENDENCIES:**
```json
"shaders": "^2.5.124",
"lucide-react": "^0.344.0",
"react": "^18.3.1",
"react-dom": "^18.3.1",
"@supabase/supabase-js": "^2.57.4"
```

Tailwind CSS 3.4, Vite 5.4, TypeScript 5.5.

## Eco Intelligence — Hero [sites/eco-intelligence]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(41).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/eco-intelligence.webp

Create a full-screen hero section for a brand called "TERRA NOVA" using React, Tailwind CSS, and Lucide React icons. It must be fully mobile responsive. Use Vite + React + TypeScript + Tailwind.

**Fonts:**
- Load "Bebas Neue" from Google Fonts for the large background text.
- Load "Helvetica Neue Light" from: `https://db.onlinewebfonts.com/c/0e6de1ec911a2e267ff136bbdd384a44?family=Helvetica+Neue+Light`
- Set body font-family to: `'Helvetica Neue Light', 'Helvetica Neue', Helvetica, Arial, sans-serif` with antialiased rendering.

**Background:**
- Full-screen `<video>` element set to autoPlay, muted, loop, playsInline, covering the entire viewport with `object-cover`.
- Video source URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_204426_c2ec12c0-3159-4601-8f8f-484c9a687833.mp4`
- Fallback background color: `#f5f4f0`

**Layout Structure (all content layered on top of the video with `relative z-10`):**

1. **Navbar** - Flex row, space-between, padding `px-5 sm:px-6 md:px-12 py-5 md:py-6`. Always black text on all screen sizes.
   - Left: A hamburger Menu icon (Lucide `Menu`, 20x20, strokeWidth 1.5) that opens a slide-in mobile menu. The word "Menu" is hidden below `sm` breakpoint.
   - Right: A small black dot (8x8 rounded-full) + "Book a call" text (text-sm tracking-wide).
   - Both sides have `hover:opacity-60 transition-opacity duration-300`.

2. **Main Content Area** - Takes remaining height (`flex-1`), with horizontal margin `mx-5 sm:mx-6 md:mx-12`, position relative.

   - **Vertical border lines (desktop only, hidden below md):** Two absolute-positioned columns (left edge and right edge), each containing: a thin 1px vertical line (bg-black/20) at 15% height, a "+" character (text-black/40, text-xs), a flex-1 line, another "+", and another 15% line.

   - **Decorative glass rectangles (desktop only, hidden below md):** Centered absolutely (top-1/2 left-1/2 -translate). A 2-col 3-row grid (220x330px at md). Three squares placed at positions [row1-col1], [row2-col1], [row3-col2]. Each square is 110x110px at md, with `bg-white/10 border border-white/40` and this exact box-shadow: `inset 0 2px 20px rgba(255,255,255,0.5), inset 0 -2px 14px rgba(0,0,0,0.2), 0 0 20px rgba(255,255,255,0.15), 0 0 40px rgba(255,255,255,0.05)`.

   - **Large background text "TERRA NOVA":** Absolutely positioned at `top-[2%]`, centered horizontally, full viewport width, pointer-events-none, select-none, overflow-hidden. Font is `font-['Bebas_Neue']`, size `text-[18vw] sm:text-[22vw] md:text-[30vw]`, leading-[0.85], tracking-tighter, whitespace-nowrap. The text uses a radial-gradient fill to appear as a subtle ghost/transparent text: `radial-gradient(83.65% 627.96% at 7.96% 53.9%, #c8c8c8 0%, rgba(200, 200, 200, 0) 52.41%, #c8c8c8 100%)` with `background-clip: text` and `-webkit-text-fill-color: transparent`.

   - **Bottom 2-column content (pinned to bottom with `mt-auto`):** Padding `pb-6 sm:pb-8 md:pb-12`, flex-col on mobile, flex-row on md+ with items-end and justify-between. Gap `gap-6 md:gap-12`, inner padding `px-2 sm:px-4 md:px-8`.

     **Left Column** (max-w-sm, `text-white sm:text-black`):
     - Heading: "Signals from" + line break + "the Deep Green". Sizes: `text-xl sm:text-2xl md:text-3xl lg:text-4xl`, font-light, leading-tight, tracking-tight.
     - Paragraph below (mt-3 md:mt-4): "An open research collective mapping, decoding, and archiving the silent vibrations that bind our planet's ecological networks." Sizes: `text-xs sm:text-sm`, color `text-white/70 sm:text-black/60`, max-w-[280px].
     - **Slanted/chamfered button** labeled "Start listening":
       - Dimensions: `w-[220px] sm:w-[260px] h-[44px] sm:h-[48px]`
       - Always black text.
       - Shape is a hexagon-like polygon with 14px chamfer cut at top-left and bottom-right corners, drawn via SVG `<polygon>` with stroke="currentColor" strokeWidth="1.5" and fill-transparent.
       - **On mobile only:** Has a white glass backdrop effect behind it: `bg-white/60 backdrop-blur-md border border-black/20` clipped to the same chamfer polygon shape using CSS `clip-path: polygon(14px 0, 100% 0, 100% calc(100% - 14px), calc(100% - 14px) 100%, 0 100%, 0 14px)`. On sm+ this glass effect is removed (`sm:bg-transparent sm:backdrop-blur-none sm:border-transparent`).
       - Content inside: label text (text-xs sm:text-sm tracking-wide) on left, ArrowRight icon (16x16) on right, with px-5 sm:px-6.
       - `hover:opacity-70 transition-opacity duration-300`.

     **Right Column - Glass Card** (max-w-xs md:max-w-[320px]):
     - `bg-white/60 backdrop-blur-md border border-white/80 p-5 sm:p-6 md:p-8 rounded-sm`
     - Header row: "Latest findings" (text-base sm:text-lg md:text-xl font-medium tracking-tight) left-aligned, "//02" (text-xs text-black/40) right-aligned. Below is a border-b border-black/10 with pb-3 md:pb-4.
     - Two content blocks (space-y-4 md:space-y-5, mt-4 md:mt-5):
       1. Title: "Canopy Pulse Analysis 09.17" (text-sm md:text-base font-semibold tracking-tight). Description: "Identified harmonic oscillation links between root mycelia networks and surrounding atmospheric moisture." (text-xs md:text-sm text-black/50 mt-1 md:mt-1.5 leading-relaxed).
       2. Title: "Watershed Harmonic Index 11.06". Description: "Forecasting framework for ecosystem regeneration spanning six continents using over 2,400 sensor arrays."
     - **Waveform SVG decoration** at bottom (mt-5 md:mt-6, centered): An SVG (viewBox 0 0 220 50, w-full) with a single `<path>` drawing a smooth waveform curve. Stroke: black, strokeWidth: 1.8, fill: none, strokeLinecap: round. Path data: `M0 30 C10 30 12 45 18 45 C24 45 26 10 34 10 C42 10 44 40 52 40 C60 40 62 5 70 5 C78 5 80 42 88 42 C96 42 98 15 106 15 C114 15 116 38 124 38 C132 38 134 20 142 20 C150 20 152 35 160 35 C168 35 170 22 178 22 C186 22 188 32 196 32 C204 32 210 28 220 28`

3. **Mobile Menu (slide-in overlay):**
   - State-controlled open/close. When open, body overflow is hidden.
   - **Backdrop:** Fixed full-screen, `bg-black/40 backdrop-blur-sm`, fades in/out with `transition-opacity duration-500`.
   - **Panel:** Fixed, top-0 left-0, full height, `w-full sm:w-[380px]`, `bg-[#f5f4f0]`, slides in from left with `transition-transform duration-500 ease-[cubic-bezier(0.16,1,0.3,1)]`.
   - Inside panel (px-8 sm:px-10 py-6, flex-col h-full):
     - Close button at top: X icon (20x20, strokeWidth 1.5) + "Close" text, mb-12.
     - Nav links: ['About', 'Research', 'Projects', 'Journal', 'Contact']. Each is a block `py-4 border-b border-black/10`. Text is `text-2xl sm:text-3xl font-light tracking-tight`. On hover, text slides right 8px and an ArrowRight icon (16x16) fades in from the right. Each link has a staggered entrance animation (opacity + translateY) with delays starting at 150ms, incrementing by 75ms.
     - Bottom section (mt-auto pb-8): border-t border-black/10 pt-6, "Get in touch" label (text-xs text-black/40 uppercase tracking-wide mb-3), email link "hello@terranova.earth" (text-sm text-black/70 hover:text-black). Also has staggered entrance with 600ms delay.

**CSS Reset (index.css):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* { margin: 0; padding: 0; box-sizing: border-box; }
html, body {
  font-family: 'Helvetica Neue Light', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  overflow-x: hidden;
}
```

**Key responsive behavior:**
- Mobile (< 640px): White text for heading/paragraph, glass-backed button, no vertical lines, no glass rectangles, compact spacing.
- Tablet (sm, 640px+): Text turns black, button loses glass backdrop, layout still single column.
- Desktop (md, 768px+): Two-column bottom layout, vertical border lines appear, glass rectangles appear, larger font sizes and spacing throughout.

## Equilibrium — Hero [sites/equilibrium]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(93).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/equilibrium.webp

Build a full-screen, single-page React + TypeScript + Vite + Tailwind CSS hero section with a "liquid glass" aesthetic on top of a looping background video. Use `lucide-react` for icons. No other UI libraries.

**Font & Global CSS (`src/index.css`):**
- Import Geist from Google Fonts: `https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&display=swap`
- Apply `Geist` globally via `* { font-family: 'Geist', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }`
- Include `@tailwind base; @tailwind components; @tailwind utilities;`
- Define a `.liquid-glass` class:
  - `background: rgba(255,255,255,0.01);`
  - `background-blend-mode: luminosity;`
  - `backdrop-filter: blur(4px);` plus `-webkit-backdrop-filter`
  - `border: none;`
  - `box-shadow: inset 0 1px 1px rgba(255,255,255,0.1);`
  - `position: relative; overflow: hidden;`
- Add a `.liquid-glass::before` pseudo-element creating a gradient border via mask compositing:
  - `content:''; position:absolute; inset:0; border-radius:inherit; padding:1.4px;`
  - `background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);`
  - `-webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0); -webkit-mask-composite: xor; mask-composite: exclude; pointer-events:none;`

**Component (`src/App.tsx`):**
- Import from `lucide-react`: `ChevronDown`, `Infinity`, `Menu`, `X`. Import `useState` from React.
- Constant `BG_VIDEO = 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_230229_7c9bc431-46cf-489a-948d-e8144d8eb5d4.mp4'`
- `navLinks` array: `{ label: 'Home', active: true }`, `{ label: 'Wellness', dropdown: true }`, `{ label: 'Routine' }`, `{ label: 'Our Team' }`.
- `menuOpen` state via `useState(false)`.

**Layout:**
- Root: `<div class="relative w-full h-screen overflow-hidden">`.
- Background `<video>` absolutely positioned, `w-full h-full object-cover`, `autoPlay muted loop playsInline`, `src={BG_VIDEO}`.

**Navbar** (`absolute top-0 left-0 right-0 z-20 flex items-center justify-between px-5 sm:px-8 py-5`):
- Logo (left): flex with `gap-2 text-white font-medium text-base`. `<Infinity size={22} strokeWidth={1.5} />` followed by `<span>Equilibrium</span>`.
- Nav pill (center, `hidden md:flex`): `liquid-glass items-center gap-1 rounded-xl px-2 py-2`. Map `navLinks`. Each button: `flex items-center gap-0.5 px-3 py-1.5 rounded-md text-sm transition-colors`; active gets `bg-white/15 text-white`, others `text-white/70 hover:text-white`. Dropdown items render a `<ChevronDown size={13} class="mt-px" />`.
- CTAs (right, `hidden md:flex items-center gap-3`):
  - "Log in": `liquid-glass text-white text-sm font-medium px-4 py-2.5 rounded-full hover:bg-white/5 transition-colors`
  - "Begin Now": `bg-white text-black text-sm font-medium px-4 py-2.5 rounded-full hover:bg-white/90 transition-colors`
- Mobile toggle (`md:hidden`): `liquid-glass text-white p-2 rounded-lg`; shows `X` when open else `Menu` (size 18).

**Mobile menu** (when `menuOpen`): `absolute top-[72px] left-4 right-4 z-30 md:hidden liquid-glass rounded-2xl p-4 flex flex-col gap-1`. Same nav links as buttons `flex items-center justify-between w-full px-4 py-3 rounded-lg text-sm`. Bottom CTA row: `flex gap-2 mt-2 pt-3 border-t border-white/10` with two `flex-1` buttons ("Log in", "Begin Now") matching desktop styles.

**Hero content (bottom-left)** `absolute bottom-0 left-0 z-20 px-6 sm:px-12 pb-10 sm:pb-16 max-w-2xl`:
- `<h1>`: `text-white text-4xl sm:text-5xl lg:text-6xl font-medium leading-tight tracking-tight mb-4` — text: `Live Better, Feel Whole Every Day`.
- `<p>`: `text-white/60 text-sm leading-relaxed mb-7 max-w-md` — text: `Take charge of how you feel with a companion built for your journey—build routines, follow your growth, and unlock tailored insights for a steadier, more vibrant life each day.`
- Buttons row `flex flex-wrap items-center gap-3`:
  - "Start Today": `bg-white text-black text-sm sm:text-base font-medium px-6 sm:px-7 py-3 rounded-full hover:bg-white/90 transition-colors`
  - "Discover How": `liquid-glass text-white text-sm sm:text-base font-medium px-6 sm:px-7 py-3 rounded-full hover:bg-white/5 transition-colors`

**Animations/interactions:** all buttons use Tailwind `transition-colors`; liquid-glass effect uses `backdrop-filter: blur(4px)` plus the animated-looking gradient border pseudo. No additional keyframe animations. The background video itself provides motion.

**Dependencies:** `react`, `react-dom`, `lucide-react`, `tailwindcss`, `vite`, `@vitejs/plugin-react`, TypeScript. Tailwind configured with default content globs for `./index.html` and `./src/**/*.{ts,tsx}`.

## FinancialFocus — Hero [sites/financialfocus]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(80).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/financialfocus.webp

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>3D Cylinder Carousel</title>
  
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500;700&family=Manrope:wght@300;400;500;600;700;800&family=Mr+Dafoe&display=swap" rel="stylesheet">
  
  <!-- Tailwind CSS V4 -->
  <script src="https://unpkg.com/@tailwindcss/browser@4"></script>
  
  <!-- React & ReactDOM -->
  <script src="https://unpkg.com/react@18/umd/react.production.min.js" crossorigin></script>
  <script src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js" crossorigin></script>
  
  <!-- Babel for JSX and TS parsing -->
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>

  <style type="text/tailwindcss">
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500;700&family=Manrope:wght@300;400;500;600;700;800&family=Mr+Dafoe&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, monospace;
  --font-manrope: "Manrope", sans-serif;
  --font-signature: "Mr Dafoe", cursive;
}

/* Custom horizontal scanlines or grids for high-tech background */
.bg-grid-subtle {
  background-size: 40px 40px;
  background-image: 
    linear-gradient(to right, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
}

.perspective-1200 {
  perspective: 1200px;
}

/* Scrollbar customizations */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.3);
}
::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.25);
}

    
    body {
      margin: 0;
      padding: 0;
      width: 100vw;
      height: 100vh;
      overflow: hidden;
      background-color: #000;
    }
    
    #root {
      width: 100%;
      height: 100%;
    }
  </style>
</head>
<body>
  <div id="root"></div>

  <script type="text/babel" data-presets="react,typescript">
const { useState, useEffect, useRef } = React;

const Menu = ({ className, strokeWidth = 2 }) => (
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" className={className}>
    <line x1="4" x2="20" y1="12" y2="12" />
    <line x1="4" x2="20" y1="6" y2="6" />
    <line x1="4" x2="20" y1="18" y2="18" />
  </svg>
);


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


function App() {
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
      
      {/* Background full-screen image under the cards component */}
      <div id="full-screen-wave-background" className="absolute inset-0 z-0 pointer-events-none flex items-center justify-center overflow-hidden">
        <img 
          src="https://ais-pre-n2veyqxlgp2lg3yian6tqu-115844097173.asia-southeast1.run.app/wave-icon.svg" 
          alt="Wave Background" 
          className="w-full h-auto max-h-screen select-none pointer-events-none"
          referrerPolicy="no-referrer"
        />
      </div>

      {/* Wavebank Brand Logo at bottom right corner of the screen */}
      <div 
        id="screen-bottom-right-brand"
        className="absolute bottom-5 right-5 sm:bottom-6 sm:right-6 lg:bottom-16 lg:right-16 z-50 hidden sm:flex items-center justify-center opacity-85 hover:opacity-100 transition-opacity duration-300 pointer-events-auto cursor-pointer"
      >
        <img 
          src="https://ais-pre-n2veyqxlgp2lg3yian6tqu-115844097173.asia-southeast1.run.app/w.svg" 
          alt="Brand Logo" 
          className="h-[40px] w-auto select-none pointer-events-none"
          referrerPolicy="no-referrer"
        />
      </div>

      {/* Screen bottom-left Heading & Descriptor Content (Restored to bottom-left with high selectability layering, fluid relative scaling & flawless mobile centering) */}
      <div 
        id="screen-bottom-left-brand-content"
        className="absolute bottom-6 left-1/2 -translate-x-1/2 sm:translate-x-0 sm:bottom-8 sm:left-8 lg:bottom-16 lg:left-16 z-50 flex flex-col items-center text-center sm:items-start sm:text-left w-[92vw] sm:w-auto max-w-[95vw] sm:max-w-xl lg:max-w-[850px] pointer-events-auto select-none"
      >
        <h1 
          className="font-manrope text-white font-semibold leading-[1.1] tracking-tight"
          style={{ fontSize: fontMetrics.titleFontSize }}
        >
          {/* Indentation for "Get More With" starts on screens sm and up to prevent off-centering on mobile */}
          <span 
            className="inline-flex items-baseline md:translate-y-[1px]"
            style={{ paddingLeft: fontMetrics.pl }}
          >
            <span 
              className="font-signature text-[#00FF88] mr-2.5 leading-[0.8] select-none"
              style={{ fontStyle: 'normal', fontSize: fontMetrics.sigFontSize }}
            >
              Get More
            </span>
            <span className="text-white leading-none">With</span>
          </span>
          <br />
          <span className="inline-block leading-none">Our Bank Cards – Easy,</span>
          <br />
          <span className="inline-block leading-none">Secure, Rewarding</span>
        </h1>

        <div 
          className="w-full flex justify-center sm:justify-end"
          style={{ marginTop: fontMetrics.titleGap }}
        >
          <p 
            className="font-manrope text-center sm:text-right text-white/50 leading-relaxed max-w-[85vw] sm:max-w-[280px] md:max-w-[340px] lg:max-w-[420px] tracking-wide font-normal select-none"
            style={{ fontSize: fontMetrics.descFontSize }}
          >
            <span className="block">Experience Effortless Banking With Our Cards That</span>
            <span className="block">Offer Security, Simplicity, And Exciting Rewards</span>
            <span className="block">Tailored For You.</span>
          </p>
        </div>
      </div>

      {/* Wavebank Header brand overlay */}
      <header className="absolute top-0 left-0 right-0 p-5 sm:p-6 lg:p-16 z-50 flex items-center justify-between pointer-events-none">
        {/* Left side: Custom wavebank SVG Logo */}
        <div className="flex items-center pointer-events-auto cursor-pointer group">
          <svg 
            width="182" 
            height="25" 
            viewBox="0 0 341 49" 
            fill="none" 
            xmlns="http://www.w3.org/2000/svg"
            className="w-auto h-[25px] sm:h-[28px] transform group-hover:scale-[1.02] active:scale-[0.98] transition-all duration-300"
          >
            <path d="M8.75294 47.68C6.10761 47.68 4.10227 47.04 2.73694 45.76C1.41427 44.48 0.582275 42.7733 0.240941 40.64C-0.100392 38.464 -0.0790588 36.0747 0.304941 33.472C0.731608 30.8267 1.37161 28.1813 2.22494 25.536C3.07827 22.848 3.99561 20.3307 4.97694 17.984C6.00094 15.5947 6.93961 13.5893 7.79294 11.968C8.26227 11.072 8.88094 10.56 9.64894 10.432C10.4169 10.2613 11.1423 10.368 11.8249 10.752C12.5503 11.136 13.0623 11.6907 13.3609 12.416C13.7023 13.1413 13.6383 13.9307 13.1689 14.784C11.2916 18.368 9.79828 21.7813 8.68894 25.024C7.57961 28.2667 6.85427 31.1467 6.51294 33.664C6.21427 36.1387 6.23561 38.1013 6.57694 39.552C6.96094 40.96 7.68628 41.664 8.75294 41.664C9.73428 41.664 10.8009 41.3013 11.9529 40.576C13.1049 39.8507 14.3423 38.5493 15.6649 36.672C17.0303 34.6667 18.3529 32.064 19.6329 28.864C20.9556 25.6213 22.1289 21.8667 23.1529 17.6C23.4089 16.6187 23.8783 15.9573 24.5609 15.616C25.2863 15.2747 26.0329 15.2107 26.8009 15.424C27.5689 15.6373 28.1876 16.064 28.6569 16.704C29.1263 17.3013 29.2543 18.0693 29.0409 19.008C27.9316 23.616 27.3769 27.5627 27.3769 30.848C27.4196 34.1333 27.7609 36.5227 28.4009 38.016C28.8703 39.0827 29.4249 39.8507 30.0649 40.32C30.7476 40.7893 31.4943 41.024 32.3049 41.024C33.1156 41.024 33.9689 40.7253 34.8649 40.128C35.8036 39.488 36.7209 38.4 37.6169 36.864C38.5556 35.328 39.3876 33.216 40.1129 30.528C37.6809 28.48 35.6756 25.7707 34.0969 22.4C32.5183 19.0293 31.7289 15.168 31.7289 10.816C31.7289 8.93867 31.9423 7.21067 32.3689 5.632C32.7956 4.05333 33.5209 2.79467 34.5449 1.856C35.5689 0.874666 36.9769 0.383999 38.7689 0.383999C40.9449 0.383999 42.7156 1.17333 44.0809 2.752C45.4463 4.288 46.4489 6.37867 47.0889 9.024C47.7289 11.6267 48.0063 14.5493 47.9209 17.792C47.8783 21.0347 47.5369 24.3413 46.8969 27.712C47.5369 28.0107 48.2196 28.2453 48.9449 28.416C49.7129 28.5867 50.4809 28.672 51.2489 28.672C52.9983 28.672 54.7903 28.416 56.6249 27.904C58.5023 27.3493 60.1023 26.6453 61.4249 25.792C62.2783 25.2373 63.0676 25.088 63.7929 25.344C64.5183 25.5573 65.0943 26.0053 65.521 26.688C65.9476 27.328 66.1183 28.0533 66.0329 28.864C65.9903 29.632 65.5636 30.272 64.7529 30.784C62.8756 32.0213 60.7423 33.0027 58.3529 33.728C56.0063 34.4533 53.6383 34.816 51.2489 34.816C49.2863 34.816 47.3449 34.4533 45.4249 33.728C44.1876 37.7387 42.5023 40.96 40.3689 43.392C38.2356 45.824 35.5476 47.04 32.3049 47.04C30.2569 47.04 28.3583 46.4427 26.6089 45.248C24.9023 44.0107 23.6223 42.4107 22.7689 40.448C22.5983 40.064 22.4276 39.6587 22.2569 39.232C22.1289 38.8053 22.0223 38.4 21.9369 38.016C21.7236 38.4 21.4889 38.7627 21.2329 39.104C21.0196 39.4453 20.7849 39.7867 20.5289 40.128C18.9503 42.3467 17.1796 44.16 15.2169 45.568C13.2969 46.976 11.1423 47.68 8.75294 47.68ZM41.5849 23.104C42.0116 19.9893 42.1183 17.3653 41.9049 15.232C41.6916 13.0987 41.3503 11.392 40.8809 10.112C40.4116 8.78933 39.9423 7.85067 39.4729 7.296C39.0463 6.69867 38.8116 6.4 38.7689 6.4C38.7689 6.4 38.6836 6.42133 38.5129 6.464C38.3849 6.464 38.2356 6.76267 38.0649 7.36C37.9369 7.91467 37.8729 9.06667 37.8729 10.816C37.8729 12.992 38.1929 15.168 38.8329 17.344C39.4729 19.4773 40.3903 21.3973 41.5849 23.104ZM91.5429 48.768C89.5376 48.768 87.9163 48.3627 86.6789 47.552C85.4843 46.784 84.6096 45.76 84.0549 44.48C83.5003 43.1573 83.2016 41.7493 83.1589 40.256C81.3243 42.4747 79.2763 44.224 77.0149 45.504C74.7963 46.7413 72.4709 47.36 70.0389 47.36C68.1189 47.36 66.3056 46.912 64.5989 46.016C62.8923 45.0773 61.5056 43.6907 60.4389 41.856C59.4149 39.9787 58.9029 37.6107 58.9029 34.752C58.9029 31.7653 59.5216 28.8427 60.7589 25.984C62.0389 23.0827 63.7669 20.48 65.9429 18.176C68.1616 15.8293 70.6789 13.9733 73.4949 12.608C76.3536 11.2 79.3403 10.496 82.4549 10.496C84.5029 10.496 86.5296 10.752 88.5349 11.264C90.5403 11.776 92.2896 12.5227 93.7829 13.504C94.6363 14.0587 95.1056 14.72 95.1909 15.488C95.2763 16.256 95.0843 16.9813 94.6149 17.664C94.1883 18.304 93.6123 18.752 92.8869 19.008C92.1616 19.264 91.3936 19.136 90.5829 18.624C89.7723 18.112 88.5563 17.6427 86.9349 17.216C85.3563 16.7467 83.8629 16.512 82.4549 16.512C80.0229 16.512 77.7616 17.0667 75.6709 18.176C73.5803 19.2853 71.7243 20.736 70.1029 22.528C68.5243 24.32 67.2869 26.2827 66.3909 28.416C65.4949 30.5493 65.0469 32.6613 65.0469 34.752C65.0469 35.8187 65.1749 36.864 65.4309 37.888C65.7296 38.8693 66.2416 39.7013 66.9669 40.384C67.6923 41.024 68.7163 41.344 70.0389 41.344C71.3189 41.344 72.7483 40.9173 74.3269 40.064C75.9483 39.168 77.4843 37.76 78.9349 35.84C79.8309 34.6453 80.7696 33.216 81.7509 31.552C82.7323 29.8453 83.6283 28.16 84.4389 26.496C85.2923 24.7893 85.9749 23.3387 86.4869 22.144C86.8283 21.2907 87.3403 20.736 88.0229 20.48C88.7483 20.224 89.4736 20.224 90.1989 20.48C90.9243 20.6933 91.5003 21.0987 91.9269 21.696C92.3536 22.2933 92.4816 23.04 92.3109 23.936L89.4949 37.632C89.1963 39.1253 89.1963 40.2347 89.4949 40.96C89.7936 41.6853 90.1776 42.176 90.6469 42.432C91.1163 42.6453 91.4149 42.752 91.5429 42.752C92.2256 42.752 93.1003 42.432 94.1669 41.792C95.2336 41.1093 96.5563 39.8507 98.1349 38.016C99.4576 36.5227 100.823 34.7733 102.231 32.768C103.682 30.72 105.068 28.6293 106.391 26.496C107.756 24.32 108.972 22.272 110.039 20.352C111.148 18.3893 112.023 16.768 112.663 15.488C113.09 14.592 113.687 14.0587 114.455 13.888C115.223 13.7173 115.948 13.824 116.631 14.208C117.356 14.5493 117.868 15.0827 118.167 15.808C118.508 16.4907 118.466 17.28 118.039 18.176C117.356 19.584 116.439 21.2907 115.287 23.296C114.178 25.3013 112.919 27.4347 111.511 29.696C110.146 31.9147 108.695 34.0907 107.159 36.224C105.666 38.3573 104.194 40.2773 102.743 41.984C101.036 43.9467 99.2869 45.568 97.4949 46.848C95.7456 48.128 93.7616 48.768 91.5429 48.768ZM118.45 48.448C115.549 48.448 113.351 47.6373 111.858 46.016C110.407 44.352 109.533 42.0267 109.234 39.04C108.978 36.0533 109.17 32.5547 109.81 28.544C110.493 24.5333 111.517 20.16 112.882 15.424C113.181 14.4427 113.693 13.8027 114.418 13.504C115.143 13.1627 115.89 13.12 116.658 13.376C117.426 13.632 118.023 14.08 118.45 14.72C118.919 15.36 119.026 16.1493 118.77 17.088C117.191 22.464 116.146 26.8373 115.634 30.208C115.165 33.536 115.037 36.096 115.25 37.888C115.463 39.6373 115.869 40.832 116.466 41.472C117.106 42.112 117.767 42.432 118.45 42.432C119.303 42.432 120.413 41.9413 121.778 40.96C123.143 39.936 124.594 38.5067 126.13 36.672C127.666 34.8373 129.138 32.7253 130.546 30.336C129.778 27.904 129.394 25.152 129.394 22.08C129.394 20.2027 129.501 18.176 129.714 16C129.97 13.7813 130.397 11.6907 130.994 9.728C131.634 7.76533 132.509 6.18667 133.618 4.992C134.77 3.79733 136.242 3.264 138.034 3.392C139.485 3.52 140.573 4.032 141.298 4.928C142.066 5.824 142.535 6.95467 142.706 8.32C142.919 9.68533 142.941 11.1573 142.77 12.736C142.599 14.272 142.343 15.808 142.002 17.344C141.661 18.8373 141.319 20.16 140.978 21.312C139.954 24.8107 138.781 28.032 137.458 30.976C138.61 33.024 140.061 34.432 141.81 35.2C143.559 35.968 145.33 36.2453 147.122 36.032C148.914 35.776 150.45 35.2427 151.73 34.432C152.583 33.8773 153.373 33.728 154.098 33.984C154.823 34.1973 155.399 34.6453 155.826 35.328C156.295 35.968 156.487 36.6933 156.402 37.504C156.317 38.272 155.869 38.912 155.058 39.424C152.967 40.7893 150.642 41.6427 148.082 41.984C145.565 42.3253 143.09 42.0907 140.658 41.28C138.226 40.4693 136.093 39.04 134.258 36.992C132.039 40.576 129.586 43.392 126.898 45.44C124.253 47.4453 121.437 48.448 118.45 48.448ZM135.666 18.112C136.391 15.5947 136.882 13.7173 137.138 12.48C137.394 11.2427 137.522 10.432 137.522 10.048C137.522 9.62133 137.522 9.408 137.522 9.408C137.522 9.408 137.394 9.68533 137.138 10.24C136.882 10.752 136.605 11.648 136.306 12.928C136.007 14.1653 135.794 15.8933 135.666 18.112ZM164.834 48.512C161.762 48.512 159.117 47.808 156.898 46.4C154.68 44.9493 152.973 43.008 151.778 40.576C150.584 38.1013 149.986 35.328 149.986 32.256C149.986 29.2267 150.562 26.3893 151.714 23.744C152.866 21.056 154.36 18.7093 156.194 16.704C158.072 14.656 160.056 13.0773 162.146 11.968C164.28 10.816 166.306 10.24 168.226 10.24C169.762 10.24 171.17 10.5387 172.45 11.136C173.73 11.7333 174.754 12.5867 175.522 13.696C176.333 14.8053 176.738 16.1493 176.738 17.728C176.738 20.0747 176.034 22.1227 174.626 23.872C173.261 25.5787 171.384 27.4773 168.994 29.568C167.202 31.1467 165.325 32.64 163.362 34.048C161.4 35.456 159.352 36.8427 157.218 38.208C158.584 41.0667 161.122 42.496 164.834 42.496C165.858 42.496 166.946 42.3467 168.098 42.048C169.25 41.7067 170.552 41.024 172.002 40C173.453 38.976 175.16 37.376 177.122 35.2C177.762 34.4747 178.466 34.1333 179.234 34.176C180.045 34.2187 180.749 34.5173 181.346 35.072C181.944 35.584 182.285 36.2453 182.37 37.056C182.498 37.824 182.242 38.5707 181.602 39.296C178.445 42.7947 175.458 45.2053 172.642 46.528C169.869 47.8507 167.266 48.512 164.834 48.512ZM156.13 31.744C157.752 30.6773 159.309 29.6107 160.802 28.544C162.296 27.4347 163.704 26.2827 165.026 25.088C167.245 23.1253 168.738 21.504 169.506 20.224C170.317 18.9013 170.722 18.0693 170.722 17.728C170.722 17.5573 170.594 17.28 170.338 16.896C170.082 16.4693 169.378 16.256 168.226 16.256C167.16 16.256 165.944 16.6613 164.578 17.472C163.256 18.24 161.954 19.328 160.674 20.736C159.437 22.144 158.392 23.7867 157.538 25.664C156.685 27.5413 156.216 29.568 156.13 31.744ZM201.487 13.248C204.773 13.248 207.717 13.9733 210.319 15.424C212.922 16.8747 214.949 18.9013 216.399 21.504C217.893 24.1067 218.639 27.1147 218.639 30.528C218.639 33.9413 217.893 36.9707 216.399 39.616C214.949 42.2187 212.922 44.2453 210.319 45.696C207.717 47.1467 204.773 47.872 201.487 47.872C198.97 47.872 196.666 47.3813 194.575 46.4C192.485 45.4187 190.757 43.9893 189.391 42.112V47.488H183.503V0H189.647V18.688C191.013 16.896 192.719 15.552 194.767 14.656C196.815 13.7173 199.055 13.248 201.487 13.248ZM200.975 42.496C203.151 42.496 205.093 42.0053 206.799 41.024C208.549 40 209.914 38.592 210.895 36.8C211.919 34.9653 212.431 32.8747 212.431 30.528C212.431 28.1813 211.919 26.112 210.895 24.32C209.914 22.4853 208.549 21.0773 206.799 20.096C205.093 19.1147 203.151 18.624 200.975 18.624C198.842 18.624 196.901 19.1147 195.151 20.096C193.402 21.0773 192.037 22.4853 191.055 24.32C190.074 26.112 189.583 28.1813 189.583 30.528C189.583 32.8747 190.074 34.9653 191.055 36.8C192.037 38.592 193.402 40 195.151 41.024C196.901 42.0053 198.842 42.496 200.975 42.496ZM256.568 13.568V47.488H250.68V42.112C249.315 43.9893 247.587 45.4187 245.496 46.4C243.406 47.3813 241.102 47.872 238.584 47.872C235.299 47.872 232.355 47.1467 229.752 45.696C227.15 44.2453 225.102 42.2187 223.608 39.616C221.432 33.9413 221.432 30.528 221.432 30.528C221.432 27.1147 222.158 24.1067 223.608 21.504C225.102 18.9013 227.15 16.8747 229.752 15.424C232.355 13.9733 235.299 13.248 238.584 13.248C241.016 13.248 243.256 13.7173 245.304 14.656C247.352 15.552 249.059 16.896 250.424 18.688V13.568H256.568ZM239.096 42.496C241.23 42.496 243.171 42.0053 244.92 41.024C246.67 40 248.035 38.592 249.016 36.8C249.998 34.9653 250.488 32.8747 250.488 30.528C250.488 28.1813 249.998 26.112 249.016 24.32C248.035 22.4853 246.67 21.0773 244.92 20.096C243.171 19.1147 241.23 18.624 239.096 18.624C236.92 18.624 234.958 19.1147 233.208 20.096C231.502 21.0773 230.136 22.4853 229.112 24.32C228.131 26.112 227.64 28.1813 227.64 30.528C227.64 32.8747 228.131 34.9653 229.112 36.8C230.136 38.592 231.502 40 233.208 41.024C234.958 42.0053 236.92 42.496 239.096 42.496Z" fill="white"/>
            <path d="M283.745 13.248C288.055 13.248 291.468 14.5067 293.985 17.024C296.545 19.4987 297.825 23.1467 297.825 27.968V47.488H291.681V28.672C291.681 25.3867 290.892 22.912 289.313 21.248C287.735 19.584 285.473 18.752 282.529 18.752C279.201 18.752 276.577 19.7333 274.657 21.696C272.737 23.616 271.777 26.3893 271.777 30.016V47.488H265.633V13.568H271.521V18.688C272.759 16.9387 274.423 15.5947 276.513 14.656C278.647 13.7173 281.057 13.248 283.745 13.248ZM319.82 31.68L312.78 38.208V47.488H306.636V0H312.78V30.464L331.276 13.568H338.7L324.428 27.584L340.108 47.488H332.556L319.82 31.68Z" fill="white"/>
          </svg>
        </div>
 
        {/* Right side: Completely separated action buttons with 0px gap */}
        <div className="flex items-center gap-0 pointer-events-auto">
          <button 
            type="button"
            className="bg-white text-black font-manrope font-semibold px-5 py-2.5 text-xs sm:text-[13px] tracking-wide rounded-full hover:bg-neutral-100 active:scale-[0.97] transition-all duration-200 flex items-center h-9 sm:h-10 cursor-pointer shadow-sm border border-white/5"
          >
            Order Card
          </button>
          <button 
            type="button"
            className="bg-white text-black p-2.5 rounded-full hover:bg-neutral-100 active:scale-[0.97] transition-all duration-200 flex items-center justify-center w-9 h-9 sm:w-10 sm:h-10 cursor-pointer shadow-sm border border-white/5"
            aria-label="Menu"
          >
            <Menu className="w-4 sm:w-4.5 h-4 sm:h-4.5 text-black" strokeWidth={2.5} />
          </button>
        </div>
      </header>



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
                              fill="white"
                            />
                            <path
                              d="M91.5429 48.768C89.5376 48.768 87.9163 48.3627 86.6789 47.552C85.4843 46.784 84.6096 45.76 84.0549 44.48C83.5003 43.1573 83.2016 41.7493 83.1589 40.256C81.3243 42.4747 79.2763 44.224 77.0149 45.504C74.7963 46.7413 72.4709 47.36 70.0389 47.36C68.1189 47.36 66.3056 46.912 64.5989 46.016C62.8923 45.0773 61.5056 43.6907 60.4389 41.856C59.4149 39.9787 58.9029 37.6107 58.9029 34.752C58.9029 31.7653 59.5216 28.8427 60.7589 25.984C62.0389 23.0827 63.7669 20.48 65.9429 18.176C68.1616 15.8293 70.6789 13.9733 73.4949 12.608C76.3536 11.2 79.3403 10.496 82.4549 10.496C84.5029 10.496 86.5296 10.752 88.5349 11.264C90.5403 11.776 92.2896 12.5227 93.7829 13.504C94.6363 14.0587 95.1056 14.72 95.1909 15.488C95.2763 16.256 95.0843 16.9813 94.6149 17.664C94.1883 18.304 93.6123 18.752 92.8869 19.008C92.1616 19.264 91.3936 19.136 90.5829 18.624C89.7723 18.112 88.5563 17.6427 86.9349 17.216C85.3563 16.7467 83.8629 16.512 82.4549 16.512C80.0229 16.512 77.7616 17.0667 75.6709 18.176C73.5803 19.2853 71.7243 20.736 70.1029 22.528C68.5243 24.32 67.2869 26.2827 66.3909 28.416C65.4949 30.5493 65.0469 32.6613 65.0469 34.752C65.0469 35.8187 65.1749 36.864 65.4309 37.888C65.7296 38.8693 66.2416 39.7013 66.9669 40.384C67.6923 41.024 68.7163 41.344 70.0389 41.344C71.3189 41.344 72.7483 40.9173 74.3269 40.064C75.9483 39.168 77.4843 37.76 78.9349 35.84C79.8309 34.6453 80.7696 33.216 81.7509 31.552C82.7323 29.8453 83.6283 28.16 84.4389 26.496C85.2923 24.7893 85.9749 23.3387 86.4869 22.144C86.8283 21.2907 87.3403 20.736 88.0229 20.48C88.7483 20.224 89.4736 20.224 90.1989 20.48C90.9243 20.6933 91.5003 21.0987 91.9269 21.696C92.3536 22.2933 92.4816 23.04 92.3109 23.936L89.4949 37.632C89.1963 39.1253 89.1963 40.2347 89.4949 40.96C89.7936 41.6853 90.1776 42.176 90.6469 42.432C91.1163 42.6453 91.4149 42.752 91.5429 42.752C92.2256 42.752 93.1003 42.432 94.1669 41.792C95.2336 41.1093 96.5563 39.8507 98.1349 38.016C99.4576 36.5227 100.823 34.7733 102.231 32.768C103.682 30.72 105.068 28.6293 106.391 26.496C107.756 24.32 108.972 22.272 110.039 20.352C111.148 18.3893 112.023 16.768 112.663 15.488C113.09 14.592 113.687 14.0587 114.455 13.888C115.223 13.7173 115.948 13.824 116.631 14.208C117.356 14.5493 117.868 15.0827 118.167 15.808C118.508 16.4907 118.466 17.28 118.039 18.176C117.356 19.584 116.439 21.2907 115.287 23.296C114.178 25.3013 112.919 27.4347 111.511 29.696C110.146 31.9147 108.695 34.0907 107.159 36.224C105.666 38.3573 104.194 40.2773 102.743 41.984C101.036 43.9467 99.2869 45.568 97.4949 46.848C95.7456 48.128 93.7616 48.768 91.5429 48.768Z"
                              fill="white"
                            />
                            <path
                              d="M118.45 48.448C115.549 48.448 113.351 47.6373 111.858 46.016C110.407 44.352 109.533 42.0267 109.234 39.04C108.978 36.0533 109.17 32.5547 109.81 28.544C110.493 24.5333 111.517 20.16 112.882 15.424C113.181 14.4427 113.693 13.8027 114.418 13.504C115.143 13.1627 115.89 13.12 116.658 13.376C117.426 13.632 118.023 14.08 118.45 14.72C118.919 15.36 119.026 16.1493 118.77 17.088C117.191 22.464 116.146 26.8373 115.634 30.208C115.165 33.536 115.037 36.096 115.25 37.888C115.463 39.6373 115.869 40.832 116.466 41.472C117.106 42.112 117.767 42.432 118.45 42.432C119.303 42.432 120.413 41.9413 121.778 40.96C123.143 39.936 124.594 38.5067 126.13 36.672C127.666 34.8373 129.138 32.7253 130.546 30.336C129.778 27.904 129.394 25.152 129.394 22.08C129.394 20.2027 129.501 18.176 129.714 16C129.97 13.7813 130.397 11.6907 130.994 9.728C131.634 7.76533 132.509 6.18667 133.618 4.992C134.77 3.79733 136.242 3.264 138.034 3.392C139.485 3.52 140.573 4.032 141.298 4.928C142.066 5.824 142.535 6.95467 142.706 8.32C142.919 9.68533 142.941 11.1573 142.77 12.736C142.599 14.272 142.343 15.808 142.002 17.344C141.661 18.8373 141.319 20.16 140.978 21.312C139.954 24.8107 138.781 28.032 137.458 30.976C138.61 33.024 140.061 34.432 141.81 35.2C143.559 35.968 145.33 36.2453 147.122 36.032C148.914 35.776 150.45 35.2427 151.73 34.432C152.583 33.8773 153.373 33.728 154.098 33.984C154.823 34.1973 155.399 34.6453 155.826 35.328C156.295 35.968 156.487 36.6933 156.402 37.504C156.317 38.272 155.869 38.912 155.058 39.424C152.967 40.7893 150.642 41.6427 148.082 41.984C145.565 42.3253 143.09 42.0907 140.658 41.28C138.226 40.4693 136.093 39.04 134.258 36.992C132.039 40.576 129.586 43.392 126.898 45.44C124.253 47.4453 121.437 48.448 118.45 48.448ZM135.666 18.112C136.391 15.5947 136.882 13.7173 137.138 12.48C137.394 11.2427 137.522 10.432 137.522 10.048C137.522 9.62133 137.522 9.408 137.522 9.408C137.522 9.408 137.394 9.68533 137.138 10.24C136.882 10.752 136.605 11.648 136.306 12.928C136.007 14.1653 135.794 15.8933 135.666 18.112Z"
                              fill="white"
                            />
                            <path
                              d="M164.834 48.512C161.762 48.512 159.117 47.808 156.898 46.4C154.68 44.9493 152.973 43.008 151.778 40.576C150.584 38.1013 149.986 35.328 149.986 32.256C149.986 29.2267 150.562 26.3893 151.714 23.744C152.866 21.056 154.36 18.7093 156.194 16.704C158.072 14.656 160.056 13.0773 162.146 11.968C164.28 10.816 166.306 10.24 168.226 10.24C169.762 10.24 171.17 10.5387 172.45 11.136C173.73 11.7333 174.754 12.5867 175.522 13.696C176.333 14.8053 176.738 16.1493 176.738 17.728C176.738 20.0747 176.034 22.1227 174.626 23.872C173.261 25.5787 171.384 27.4773 168.994 29.568C167.202 31.1467 165.325 32.64 163.362 34.048C161.4 35.456 159.352 36.8427 157.218 38.208C158.584 41.0667 161.122 42.496 164.834 42.496C165.858 42.496 166.946 42.3467 168.098 42.048C169.25 41.7067 170.552 41.024 172.002 40C173.453 38.976 175.16 37.376 177.122 35.2C177.762 34.4747 178.466 34.1333 179.234 34.176C180.045 34.2187 180.749 34.5173 181.346 35.072C181.944 35.584 182.285 36.2453 182.37 37.056C182.498 37.824 182.242 38.5707 181.602 39.296C178.445 42.7947 175.458 45.2053 172.642 46.528C169.869 47.8507 167.266 48.512 164.834 48.512ZM156.13 31.744C157.752 30.6773 159.309 29.6107 160.802 28.544C162.296 27.4347 163.704 26.2827 165.026 25.088C167.245 23.1253 168.738 21.504 169.506 20.224C170.317 18.9013 170.722 18.0693 170.722 17.728C170.722 17.5573 170.594 17.28 170.338 16.896C170.082 16.4693 169.378 16.256 168.226 16.256C167.16 16.256 165.944 16.6613 164.578 17.472C163.256 18.24 161.954 19.328 160.674 20.736C159.437 22.144 158.392 23.7867 157.538 25.664C156.685 27.5413 156.216 29.568 156.13 31.744Z"
                              fill="white"
                            />
                            <path
                              d="M201.487 13.248C204.773 13.248 207.717 13.9733 210.319 15.424C212.922 16.8747 214.949 18.9013 216.399 21.504C217.893 24.1067 218.639 27.1147 218.639 30.528C218.639 33.9413 217.893 36.9707 216.399 39.616C214.949 42.2187 212.922 44.2453 210.319 45.696C207.717 47.1467 204.773 47.872 201.487 47.872C198.97 47.872 196.666 47.3813 194.575 46.4C192.485 45.4187 190.757 43.9893 189.391 42.112V47.488H183.503V0H189.647V18.688C191.013 16.896 192.719 15.552 194.767 14.656C196.815 13.7173 199.055 13.248 201.487 13.248ZM200.975 42.496C203.151 42.496 205.093 42.0053 206.799 41.024C208.549 40 209.914 38.592 210.895 36.8C211.919 34.9653 212.431 32.8747 212.431 30.528C212.431 28.1813 211.919 26.112 210.895 24.32C209.914 22.4853 208.549 21.0773 206.799 20.096C205.093 19.1147 203.151 18.624 200.975 18.624C198.842 18.624 196.901 19.1147 195.151 20.096C193.402 21.0773 192.037 22.4853 191.055 24.32C190.074 26.112 189.583 28.1813 189.583 30.528C189.583 32.8747 190.074 34.9653 191.055 36.8C192.037 38.592 193.402 40 195.151 41.024C196.901 42.0053 198.842 42.496 200.975 42.496Z"
                              fill="white"
                            />
                            <path
                              d="M256.568 13.568V47.488H250.68V42.112C249.315 43.9893 247.587 45.4187 245.496 46.4C243.406 47.3813 241.102 47.872 238.584 47.872C235.299 47.872 232.355 47.1467 229.752 45.696C227.15 44.2453 225.102 42.2187 223.608 39.616C222.158 36.9707 221.432 33.9413 221.432 30.528C221.432 27.1147 222.158 24.1067 223.608 21.504C225.102 18.9013 227.15 16.8747 229.752 15.424C232.355 13.9733 235.299 13.248 238.584 13.248C241.016 13.248 243.256 13.7173 245.304 14.656C247.352 15.552 249.059 16.896 250.424 18.688V13.568H256.568ZM239.096 42.496C241.23 42.496 243.171 42.0053 244.92 41.024C246.67 40 248.035 38.592 249.016 36.8C249.998 34.9653 250.488 32.8747 250.488 30.528C250.488 28.1813 249.998 26.112 249.016 24.32C248.035 22.4853 246.67 21.0773 244.92 20.096C243.171 19.1147 241.23 18.624 239.096 18.624C236.92 18.624 234.958 19.1147 233.208 20.096C231.502 21.0773 230.136 22.4853 229.112 24.32C228.131 26.112 227.64 28.1813 227.64 30.528C227.64 32.8747 228.131 34.9653 229.112 36.8C230.136 38.592 231.502 40 233.208 41.024C234.958 42.0053 236.92 42.496 239.096 42.496Z"
                              fill="white"
                            />
                            <path
                              d="M283.745 13.248C288.055 13.248 291.468 14.5067 293.985 17.024C296.545 19.4987 297.825 23.1467 297.825 27.968V47.488H291.681V28.672C291.681 25.3867 290.892 22.912 289.313 21.248C287.735 19.584 285.473 18.752 282.529 18.752C279.201 18.752 276.577 19.7333 274.657 21.696C272.737 23.616 271.777 26.3893 271.777 30.016V47.488H265.633V13.568H271.521V18.688C272.759 16.9387 274.423 15.5947 276.513 14.656C278.647 13.7173 281.057 13.248 283.745 13.248Z"
                              fill="white"
                            />
                            <path
                              d="M319.82 31.68L312.78 38.208V47.488H306.636V0H312.78V30.464L331.276 13.568H338.7L324.428 27.584L340.108 47.488H332.556L319.82 31.68Z"
                              fill="white"
                            />
                          </svg>
                        </div>

                        {/* Double intersecting circle Brand Logo - bottom right corner */}
                        <div className="absolute right-5 sm:right-6 bottom-5 sm:bottom-6 flex -space-x-3 items-center opacity-90">
                          <div className="w-5 h-5 sm:w-6 sm:h-6 rounded-full bg-white/20 backdrop-blur-[1px] border border-white/10" />
                          <div className="w-5 h-5 sm:w-6 sm:h-6 rounded-full bg-white/35 backdrop-blur-[1px] border border-white/10" />
                        </div>
                      </div>
                    </div>
                  );
                }

                // Back face slice
                if (isBackFace) {
                  const backBorderStyle = "border border-white/15";
                  const details = CARD_DETAILS[i % CARD_DETAILS.length];
                  return (
                    <div
                      key={layerIdx}
                      className={`absolute inset-0 rounded-[16px] ${backBorderStyle} pointer-events-none overflow-hidden`}
                      style={{
                        backgroundColor: baseBgColor,
                        transform: `translateZ(${zOffset}px) rotateX(180deg)`,
                        backfaceVisibility: 'hidden',
                        boxShadow: 'inset 0 1px 1px rgba(255,255,255,0.15)',
                      }}
                    >
                      {/* Render Video with premium 16px blur on the back face of the card */}
                      <div className="absolute inset-0 pointer-events-none" style={{ filter: 'blur(16px)', transform: 'scale(1.15)' }}>
                        <video
                          src={videoSrc}
                          autoPlay
                          loop
                          muted
                          playsInline
                          className="absolute inset-0 w-full h-full object-cover"
                        />
                      </div>

                      {/* Premium Real Magnetic stripe */}
                      <div className="absolute left-0 right-0 top-4 sm:top-5 h-7 sm:h-9 bg-black/85 backdrop-blur-md z-10" />

                      {/* Card holder info and details on the bottom-left */}
                      <div 
                        className="absolute left-4 sm:left-6 bottom-4 sm:bottom-5 z-20 flex flex-col gap-0.5 sm:gap-1 text-left"
                        style={{ fontFamily: '"JetBrains Mono", monospace' }}
                      >
                        {/* Card Number */}
                        <div className="font-mono text-[10px] sm:text-[12px] font-medium tracking-[0.14em] text-white select-none">
                          {details.number}
                        </div>
                        {/* Owner & CVV */}
                        <div className="font-mono text-[7px] sm:text-[9px] font-medium text-white/70 tracking-wide flex items-center gap-2 select-none">
                          <span className="uppercase">{details.name}</span>
                          <span className="text-white/40 font-light">•</span>
                          <span>CVV: {details.cvv}</span>
                        </div>
                      </div>
                    </div>
                  );
                }

                return null;
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}


    const root = ReactDOM.createRoot(document.getElementById('root'));
    root.render(<App />);
  </script>
</body>
</html>

## Futuristic Cinematic — Hero [sites/futuristic-cinematic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(52).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/futuristic-cinematic.webp

Create a full-screen dark hero section for a brand called "axentra" using React, TypeScript, Tailwind CSS, Lucide React icons, and the `shaders` package (`shaders/react`). The page should be a single viewport-height section with a WebGL shader background and centered text overlay.

---

**Font:** Inter (imported from Google Fonts: `https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap`). Apply `-webkit-font-smoothing: antialiased` and `-moz-osx-font-smoothing: grayscale` globally.

---

**Background:** Use the `shaders` npm package (`shaders/react`) to render a full-screen WebGL shader as an absolutely positioned element covering the entire viewport (`absolute inset-0 z-0 w-full h-full`). The shader composition is:

```jsx
<Shader>
  <StudioBackground
    ambientIntensity={32}
    ambientSpeed={0.3}
    backColor="#1a0f2e"
    backIntensity={34}
    backSoftness={61}
    brightness={5}
    center={{ x: 0.49, y: 0.95 }}
    color="#17171c"
    fillAngle={84}
    fillColor="#ffffff"
    fillIntensity={55}
    fillSoftness={100}
    keyColor="#ffffff"
    keyIntensity={15}
    keySoftness={70}
    lightTarget={64}
    seed={42}
    vignette={25}
    wallCurvature={42}
  />
  <Spherize
    depth={1.1}
    lightColor="#a9cbe8"
    lightIntensity={0.3}
    lightPosition={{ x: 0.62, y: 0.01 }}
    lightSoftness={0.2}
    radius={0.9}
  >
    <Swirl colorA="#0a0a0d" colorB="#0f0f1a" colorSpace="oklab" detail={1.2} speed={0.5} />
    <LensFlare
      ghostChroma={0}
      ghostIntensity={0.35}
      ghostSpread={0.78}
      glareIntensity={0.15}
      glareSize={0.15}
      haloChroma={2}
      haloIntensity={0.27}
      haloRadius={0.38}
      haloSoftness={1.1}
      lightPosition={{ x: 0.57, y: 0.25 }}
      speed={0.9}
      starburstIntensity={0.05}
      starburstPoints={4}
      streakIntensity={0}
      streakLength={0.21}
    />
    <FloatingParticles
      angle={188}
      angleVariance={77}
      opacity={0.49}
      particleColor="#c5b7ed"
      particleSize={0.6}
      randomness={0.3}
      speed={0.1}
      speedVariance={0.6}
      twinkle={1}
    />
    <CursorRipples chromaticSplit={3} decay={4} />
  </Spherize>
  <FilmGrain strength={0.05} visible={true} />
</Shader>
```

---

**Navbar (absolutely positioned, z-50):**
- Positioned `absolute top-0 left-0 right-0`, flex row, space-between, padding `px-5 py-4` on mobile, `lg:px-10 lg:py-6` on desktop.
- Left: Brand name "axentra" in white, text-xl, font-semibold, tracking-tight, Inter font.
- Center (desktop only, hidden on mobile): A pill-shaped nav container with a custom "liquid-glass" effect (described below), containing links: "Platform", "How it works", "AI Defense", "Connections", "Insights". Links are white/80 opacity, text-sm, Inter font, with rounded-full hover:bg-white/10 hover:text-white transitions.
- Right: A white "Join the wait" button (hidden on mobile), rounded-full, text-sm, font-medium, black text, px-5 py-2, hover:opacity-80 transition.
- Mobile: An animated hamburger button (Menu/X icons from lucide-react) that toggles a slide-down mobile menu panel with backdrop blur. The mobile menu animates with staggered item reveals using cubic-bezier(0.23, 1, 0.32, 1) easing, 50ms stagger per item. Mobile menu has semi-transparent dark background (rgba(8,8,8,0.97)), items are white/70 with hover states, and includes a full-width "Join the wait" button at the bottom. Escape key closes the menu.

---

**Liquid Glass CSS (custom Tailwind `@layer components`):**
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

**Hero Content (centered, z-20):**
- Container: `flex flex-col items-center justify-center text-center h-full px-5 sm:px-8 lg:px-10`
- Heading (h1): White, font-normal, leading-[1.12], tracking-tight, max-w-3xl, Inter font. Font size uses `clamp(1.75rem, 5vw, 2.6rem)`. Text reads: "When strategy meets its spark" with a line break (hidden on mobile) followed by "and thought reshapes what lies ahead".
- Subtext (p): Courier New monospace font, letter-spacing 0.01em, color rgba(255,255,255,0.6), mt-5 (md:mt-6), text-sm (md:text-base), leading-relaxed, max-w-xs (sm:max-w-sm, md:max-w-md). Text reads: "a fluid channel - where deep resolve" with a line break followed by "and neural insight dissolve as one".
- CTA button: mt-7 (md:mt-8), white background, black text, rounded-full, text-sm, font-medium, px-5 py-2.5, flex row with gap-2.5, Inter font. Label: "See it in motion" with an ArrowRight icon (size 15) that translates right 0.5 on hover. hover:opacity-80 transition.

---

**Outer wrapper:** `relative w-full h-screen overflow-hidden bg-black`, Inter font applied via inline style.

**No CloudFront video URL exists in this project** -- the background is entirely a real-time WebGL shader rendered by the `shaders` npm package, not a video.

**Dependencies:** `react`, `react-dom`, `lucide-react`, `shaders` (v2.5.124+), `tailwindcss`, `vite`, `typescript`.

## Futuristic Tech — Hero [sites/futuristic-tech]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(73).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/futuristic-tech.webp

Build a full-viewport hero section with a dark, cinematic aesthetic. Here are the exact specifications:

**Video Background:**
- Full-screen looping background video, muted, autoplaying, with `playsInline`
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260525_052706_d2e390fd-1846-4fe7-a4d8-8d2f1c875358.mp4`
- Positioned `absolute inset-0`, `object-cover`, `z-0`

**Font:**
- Google Fonts: `Inter` (weights 400, 500, 600, 700) imported via `@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap')`
- Applied globally with `-webkit-font-smoothing: antialiased` and `-moz-osx-font-smoothing: grayscale`

**Container:**
- `relative w-full h-screen overflow-hidden bg-black` with `font-family: 'Inter, sans-serif'`

**Navbar (absolute, z-50, top):**
- Positioned `absolute top-0 left-0 right-0 z-50`, flex row, `items-center justify-between`, padding `px-5 py-4 lg:px-10 lg:py-6`
- **Logo:** Text "axentra" in white, `text-xl font-semibold tracking-tight`, font Inter
- **Desktop nav links** (hidden on mobile, `hidden lg:flex`): Items are "Platform", "How it works", "AI Defense", "Connections", "Insights" inside a pill-shaped container with a custom `liquid-glass` effect (glassmorphism). Each link: `text-white/80 hover:text-white text-sm px-4 py-1.5 rounded-full hover:bg-white/10`
- **CTA button** (desktop only, `hidden lg:block`): "Join the wait", white background (`#ffffff`), black text, `text-sm font-medium px-5 py-2 rounded-full`, hover opacity 0.8
- **Hamburger** (mobile only, `lg:hidden`): Animated toggle between `Menu` and `X` icons from lucide-react (size 20, strokeWidth 1.5, white). Uses cubic-bezier(0.23,1,0.32,1) easing with rotation and scale animations for the icon swap. Background changes to `#1a1a1a` when open.

**Liquid Glass CSS effect** (for the desktop nav pill):
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

**Mobile Menu (slide-down panel):**
- Backdrop: fixed `inset-0 z-30`, blur(12px), `rgba(0,0,0,0.6)` when open, click-to-close
- Panel: fixed `top-0 left-0 right-0 z-40`, max-height animates from 0 to 420px with `cubic-bezier(0.23, 1, 0.32, 1)` over 0.5s
- Panel background: `rgba(8,8,8,0.97)`, bottom border `1px solid rgba(255,255,255,0.08)`, padding `pt-20 pb-6 px-5`
- Each nav item: `text-white/70 hover:text-white text-base py-3 px-3 rounded-xl hover:bg-white/5`, staggered fade-in animation (each item delayed by `i * 50 + 80`ms), translateY(-8px) to 0 on open
- Each item has an `ArrowRight` icon (size 14) that appears on hover (opacity 0 to 0.4, translateX animation)
- Bottom section: separated by `1px solid rgba(255,255,255,0.07)` border, contains full-width "Join the wait" button (white bg, black text, rounded-full)
- Escape key closes the menu

**Hero Content (bottom-left aligned, z-20):**
- Container: `relative z-20 flex flex-col items-start justify-end text-left h-full px-5 sm:px-8 lg:px-10 pb-16 md:pb-20`
- **Heading:** "When strategy meets its spark / and thought reshapes what lies ahead"
  - White, `font-normal`, `leading-[1.12]`, `tracking-tight`, `max-w-3xl`
  - Font size: `clamp(1.75rem, 5vw, 2.6rem)`
  - Line break (`<br className="hidden sm:block" />`) between "spark" and "and thought..."
- **Subtext:** "a fluid channel - where deep resolve / and neural insight dissolve as one"
  - Font: `'Courier New', Courier, monospace` (monospace font)
  - Color: `rgba(255, 255, 255, 0.6)`
  - `text-sm md:text-base leading-relaxed`, `letter-spacing: 0.01em`
  - `max-w-xs sm:max-w-sm md:max-w-md`
  - Margin: `mt-5 md:mt-6`
  - Line break between "resolve" and "and neural..."
- **CTA Button:** "See it in motion" with ArrowRight icon
  - White bg (`#ffffff`), black text, `text-sm font-medium`
  - `px-5 py-2.5 rounded-full`
  - `mt-7 md:mt-8`
  - ArrowRight icon (size 15) translates right 0.5 on hover (`group-hover:translate-x-0.5`)
  - `hover:opacity-80` with 300ms transition

**Dependencies:**
- React 18, TypeScript, Tailwind CSS 3, Vite
- `lucide-react` for icons (ArrowRight, Menu, X)
- Google Fonts Inter

## Growth Marketing SaaS — Hero [sites/growth-marketing-saas]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(28).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/growth-marketing-saas.webp

Build a single landing page with only a fixed Navbar and a full-screen Hero section that contains a parallax dashboard mock and a foreground grass image. Use React + Vite + TypeScript + Tailwind + framer-motion + lucide-react. No backend.

1. Global setup

`index.html` — add fonts in `<head>`
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet" />
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet" />
<link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,400,0..1,0" rel="stylesheet" />
```

Body background must be `#08020e`.

`tailwind.config.ts` — extend
```ts
fontFamily: { inter: ['Inter','ui-sans-serif','system-ui','sans-serif'] },
colors: {
  landing: {
    surface: "rgba(255,255,255,0.10)",
    "surface-hover": "rgba(255,255,255,0.16)",
    border: "rgba(255,255,255,0.10)",
    "border-strong": "rgba(255,255,255,0.20)",
    text: "rgba(255,255,255,0.80)",
    "text-muted": "rgba(255,255,255,0.60)",
  },
}
```

`src/index.css` — add
```css
body { background-color: #08020e; margin: 0; min-height: 100vh; color: white; }

.landing-root {
  --background: 0 0% 0%;
  --foreground: 0 0% 98%;
  --radius: 0.75rem;
  background-color: hsl(var(--background));
  color: hsl(var(--foreground));
}

/* Liquid glass utility */
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
  content: ''; position: absolute; inset: 0;
  border-radius: inherit; padding: 1.4px;
  background: linear-gradient(180deg,
    rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
    rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor; mask-composite: exclude;
  pointer-events: none;
}

.scrollbar-hide { -ms-overflow-style: none; scrollbar-width: none; }
.scrollbar-hide::-webkit-scrollbar { display: none; }
```

Wrap the page in `<div className="landing-root font-inter min-h-screen relative overflow-x-hidden">`.

2. Asset URLs (all remote — no local files)

- Hero background video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260521_014404_fadafdb1-4df6-4699-be9c-77d25f39a3d0.mp4`
- Dashboard live-preview video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_115001_bcdaa3b4-03de-47e7-ad63-ae3e392c32d4.mp4`
- Foreground grass PNG: `https://miptxtnhvjrkpmnjgdhk.supabase.co/storage/v1/object/public/training-assets/landing%2Fhero-bottom-bg.png`

3. Components

`MIcon` (Google Material Symbols)
```tsx
export const MIcon = ({ name, size = 16, className = "", filled = false, weight = 400, style }: {
  name: string; size?: number; className?: string; filled?: boolean; weight?: number; style?: React.CSSProperties;
}) => (
  <span aria-hidden className={`material-symbols-outlined select-none leading-none inline-flex items-center justify-center ${className}`}
    style={{ fontSize: size, width: size, height: size,
      fontVariationSettings: `'FILL' ${filled?1:0}, 'wght' ${weight}, 'GRAD' 0, 'opsz' ${Math.min(48,Math.max(20,size))}`,
      ...style }}>{name}</span>
);
```

`AnimatedText` — text slides up on hover, replacement slides in from below (40px, 0.2s easeInOut). Uses framer-motion `motion.div` parent (`overflow-hidden`) with two stacked `motion.span` children; rest variant `{y:0}` / `{y:40}`, hover variant `{y:-40}` / `{y:0}`.

`FadeUp` — framer-motion wrapper: `initial={{opacity:0, y:24}}`, `whileInView={{opacity:1, y:0}}`, `viewport={{once:true, amount:0.3}}`, `transition={{ duration:0.6, delay, ease:[0.22,1,0.36,1] }}`. Honors `useReducedMotion`. Accepts `delay`, `duration`, `y` props.

`PrimaryButton` — white pill CTA
- Classes: `inline-flex items-center justify-center rounded-full bg-white/80 hover:bg-white text-black leading-none transition-colors h-12 px-9 text-sm font-medium`
- Wraps children in `<AnimatedText>`.

`SecondaryButton` — glass pill
- Classes: `inline-flex items-center justify-center rounded-full bg-landing-surface hover:bg-landing-surface-hover border border-landing-border text-foreground backdrop-blur-[2.5px] font-medium leading-none h-8 px-4 text-sm` (size=sm)
- Wraps children in `<AnimatedText>`.

`HeroBadge`
```tsx
<div className="inline-flex items-center justify-center rounded-full bg-landing-surface border border-landing-border px-4 h-7 text-sm text-landing-text">
  {children}
</div>
```

4. Navbar (fixed, transparent)

```tsx
const navItems = [
  { name: "About", href: "#about" },
  { name: "Features", href: "#features" },
  { name: "What you get", href: "#what-you-get" },
  { name: "Pricing", href: "#pricing" },
];
```

- `<nav className="fixed top-0 left-0 right-0 z-50 w-full bg-transparent">`
- Inner: `mx-auto flex h-16 max-w-[1080px] items-center justify-between px-6 lg:px-0`
- Left logo: `<a href="/" className="flex items-center gap-2 text-foreground">` with `<MIcon name="rocket_launch" size={20} />` + `<span className="text-base font-semibold tracking-tight">UI Rocket</span>`
- Center (lg only): `flex items-center gap-8`, each link `text-sm text-landing-text hover:text-foreground transition-colors`, wrap label in `<AnimatedText>`. Smooth-scroll on click via `document.getElementById(id)?.scrollIntoView({behavior:"smooth"})`.
- Right actions (lg only): `flex items-center gap-5` → "Login" link (same style as nav links, wrapped in AnimatedText) + `<SecondaryButton href="/auth" size="sm">Get started</SecondaryButton>`
- Mobile: menu button (`MIcon name="menu" size={24}`) opens a right-side sheet (use shadcn Sheet) with the same items stacked.

5. Hero section

```tsx
const HERO_VIDEO = "https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260521_014404_fadafdb1-4df6-4699-be9c-77d25f39a3d0.mp4";
const GRASS_IMG  = "https://miptxtnhvjrkpmnjgdhk.supabase.co/storage/v1/object/public/training-assets/landing%2Fhero-bottom-bg.png";
```

Inside the Hero component, set up scroll-linked parallax:
```ts
const sectionRef = useRef<HTMLElement>(null);
const { scrollYProgress } = useScroll({ target: sectionRef, offset: ["start start", "end start"] });
const dashboardY    = useTransform(scrollYProgress, [0, 1],   ["0%", "-25%"]);
const grassY        = useTransform(scrollYProgress, [0, 1],   ["0%",  "20%"]);
const contentY      = useTransform(scrollYProgress, [0, 1],   ["0%", "-60%"]);
const contentOpacity= useTransform(scrollYProgress, [0, 0.6], [1, 0]);
```

Structure:
```tsx
<section ref={sectionRef} id="hero" className="relative w-full min-h-screen">
  {/* 1) Background video — full bleed, no overlay */}
  <video src={HERO_VIDEO} autoPlay muted loop playsInline
    className="absolute inset-0 w-full h-full object-cover z-0" />

  {/* 2) Centered copy + CTA, with scroll fade/translate */}
  <motion.div style={{ y: contentY, opacity: contentOpacity }}
    className="relative z-20 flex flex-col items-center text-center px-4 sm:px-6 pt-28 sm:pt-36 md:pt-44 max-w-[980px] mx-auto">
    <FadeUp delay={0}>
      <HeroBadge>Founder member sale special</HeroBadge>
    </FadeUp>
    <FadeUp delay={0.1}>
      <h1 className="mt-8 text-foreground text-[38px] sm:text-[52px] md:text-[64px] leading-[1.05] tracking-[-0.03em] max-w-[960px]">
        Are you a designer or builder who wants to stay ahead of AI?
      </h1>
    </FadeUp>
    <FadeUp delay={0.2}>
      <p className="mt-6 text-landing-text text-base sm:text-lg leading-[1.5] max-w-[520px]">
        Learn to turn your ideas into stunning websites with AI
      </p>
    </FadeUp>
    <FadeUp delay={0.3} className="mt-10">
      <PrimaryButton as="button">Get course</PrimaryButton>
    </FadeUp>
  </motion.div>

  {/* 3) Dashboard mock — slower parallax (-25%) */}
  <motion.div style={{ y: dashboardY }}
    className="relative z-10 mt-8 sm:mt-10 md:mt-12 px-4 sm:px-6">
    <DashboardMock />
  </motion.div>

  {/* 4) Foreground grass — in front of dashboard, drifts down 20% */}
  <motion.img src={GRASS_IMG} alt="" aria-hidden style={{ y: grassY }}
    className="pointer-events-none select-none absolute left-0 right-0 bottom-[-40px] sm:bottom-[-100px] lg:bottom-[-220px] w-full z-30 object-cover" />
</section>
```

6. DashboardMock — liquid-glass wrapper with two-column grid

```tsx
<div className="liquid-glass w-full max-w-[1100px] aspect-[3/4] sm:aspect-[16/10] lg:aspect-[16/9] rounded-2xl mx-auto overflow-hidden p-2 sm:p-3">
  <div className="grid h-full grid-cols-1 sm:grid-cols-[minmax(220px,320px)_1fr] gap-2 sm:gap-3">
    <div className="min-h-0 hidden sm:block"><ChatPanel animateMessagesIn /></div>
    <div className="min-h-0"><LivePreviewHero /></div>
  </div>
</div>
```

`ChatPanel` (left column)
- Container: `flex h-full flex-col overflow-hidden rounded-2xl border border-white/10`, inline style `background: rgba(8,8,10,0.6); backdropFilter: blur(24px); WebkitBackdropFilter: blur(24px)`.
- Header: `flex items-center gap-2 px-4 py-3 border-b border-white/5`. Circle `w-7 h-7 rounded-full bg-white/5 flex items-center justify-center` with `<MIcon name="auto_awesome" size={14} className="text-white/80" />`. Text column: `Vibe Design course` (`text-sm font-medium text-white`) + subtitle `Learn how to build website with AI` (`text-[11px] text-white/40`).
- Messages list: `flex-1 overflow-y-auto scrollbar-hide px-4 py-5 space-y-4`. Each row wrapped in `<FadeUp delay={i*0.12} y={16}>`. Layout:
  - Row: `flex justify-end` (user) or `flex justify-start` (assistant).
  - Bubble: `max-w-[85%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed`; user = `bg-white/15 text-white/90`; assistant = `bg-white/5 text-white/70 border border-white/5`.
- Seed messages (exact text):
  1. assistant — "Welcome to the Vibe Design course! I'll guide you through building stunning websites with AI. What would you like to learn first?"
  2. user — "I want to learn how to build a hero section with a cinematic video background using AI."
  3. assistant — "Great choice! In this course, you'll learn how to create full-screen looping videos, liquid glass nav bars, email signups, and manifesto buttons — all with AI assistance. Let's dive in!"
- Input: outer `p-3 border-t border-white/5`. Inner `liquid-glass rounded-2xl flex items-end gap-2 p-2` with a `<textarea rows={1}>` (`flex-1 resize-none bg-transparent px-3 py-2 text-sm text-white placeholder:text-white/40 focus:outline-none max-h-32`, placeholder "Ask about the course...") and a send button `bg-white text-black rounded-xl p-2 hover:bg-white/90` containing `<MIcon name="arrow_upward" size={16} className="text-black" />`. Enter (no shift) sends; appends a user message then a canned assistant reply. After updates, scroll list to bottom smoothly.

`LivePreviewHero` (right column)
Uses `lucide-react` icons `Globe, ArrowRight, Instagram, Twitter`.

- Outer: `relative w-full h-full min-h-[500px] overflow-hidden rounded-2xl bg-black`.
- Background video (with JS fade-in/out loop):
  ```tsx
  <video ref={videoRef} src={DASHBOARD_VIDEO} muted autoPlay playsInline preload="auto"
    className="absolute inset-0 w-full h-full object-cover translate-y-[17%]"
    style={{ opacity: 0 }} />
  ```
  Behavior (in `useEffect`):
  - On `loadeddata`: set opacity 0, `play()`, fade opacity to 1 over 500ms via `requestAnimationFrame` linear tween.
  - On `timeupdate`: when `duration - currentTime < 0.55s` and not already fading out, fade opacity to 0 over 500ms.
  - On `ended`: snap opacity to 0, after 100ms reset `currentTime=0`, `play()`, reset fadingOut flag, fade back to 1.
  - Cleanup all listeners and cancel RAF on unmount.

- Inner content stack (`relative z-10 flex flex-col min-h-full h-full`):

  Mini-nav `relative z-20 px-3 sm:px-4 py-3`, inside a `rounded-full px-2 sm:px-4 py-1.5 flex items-center justify-between max-w-5xl mx-auto`:
  - Left group `flex items-center gap-3 sm:gap-5`: `Globe size={14} text-white` + `<span className="text-white font-semibold text-xs sm:text-sm">Asme</span>`. After it, `hidden md:flex items-center gap-5` of links `Features`, `Pricing`, `About` each `text-white/80 hover:text-white text-[11px] font-medium`.
  - Right group `flex items-center gap-2 sm:gap-3`: "Sign Up" link (`text-white text-[11px] font-medium hidden sm:inline`) + glass pill `<a className="liquid-glass rounded-full px-3 sm:px-4 py-1 text-white text-[11px] font-medium">Login</a>`.

  Hero block `relative z-10 flex-1 flex flex-col items-center justify-center px-4 sm:px-6 py-4 text-center -translate-y-[8%] sm:-translate-y-[15%]`:
  - `<h1 className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl text-white mb-4 sm:mb-5 tracking-tight whitespace-nowrap" style={{ fontFamily: "'Instrument Serif', serif" }}>Built for the curious</h1>`
  - Inner column `max-w-sm w-full space-y-3`:
    - Email pill `liquid-glass rounded-full pl-4 pr-1.5 py-1.5 flex items-center gap-2`: `<input type="email" placeholder="Enter your email" className="flex-1 bg-transparent text-white placeholder:text-white/40 text-xs focus:outline-none" />` + circular submit `bg-white rounded-full p-1.5 text-black` with `<ArrowRight size={14} />`.
    - Paragraph `text-white/80 text-[11px] leading-relaxed px-2`: "Stay updated with the latest news and insights. Subscribe to our newsletter today and never miss out on exciting updates."
    - Centered glass pill button: `liquid-glass rounded-full px-5 py-1.5 text-white text-[11px] font-medium hover:bg-white/5 transition-colors` → label "Manifesto".

  Socials row `relative z-10 flex justify-center gap-2 pb-4 sm:pb-6` — three glass round buttons (`liquid-glass rounded-full p-2 text-white/80 hover:text-white hover:bg-white/5 transition-all`) wrapping `Instagram`, `Twitter`, `Globe` icons at `size={14}`.

7. Page assembly

```tsx
export default function Page() {
  return (
    <div className="landing-root font-inter min-h-screen relative overflow-x-hidden">
      <Navbar />
      <Hero />
    </div>
  );
}
```

8. Behavioral notes (must match)

- Hero video is autoplay/muted/loop/playsInline with no dark overlay.
- Z-index stack: video `z-0`, dashboard `z-10`, copy/CTA `z-20`, grass `z-30`, navbar `z-50`.
- Hero copy fades + translates up to `-60%` during scroll through the section, fully fading by 60% scroll progress.
- Dashboard parallaxes up (`-25%`); grass drifts down (`+20%`) — creates depth.
- All button labels animate with the "text slides up, replacement slides in from below" effect via `AnimatedText`.
- Inter is the global UI font; the dashboard hero `<h1>` "Built for the curious" uses Instrument Serif.

## Immersive Ocean — Hero [sites/immersive-ocean]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(14).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/immersive-ocean.webp

Create a fullscreen hero landing page for a creative studio called "Foldcraft" using React, Tailwind CSS, and Lucide React icons. The page is a single viewport-height section with a looping background video, a responsive navbar, a mobile menu, and staggered-animated hero text.

**Video Background:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_204221_5339e40b-e73d-4ab0-9c65-79c18c66fd50.mp4`
- Attributes: autoPlay, muted, loop, playsInline
- Styling: absolute positioned, full width/height, object-cover, object-position at 70% horizontal center
- The video sits behind all content (no z-index or z-0)

**Font:**
- Google Fonts: Geist (weights 300-700), loaded via `<link>` in index.html
- Tailwind config extends fontFamily with `geist: ['Geist', 'sans-serif']`
- Applied as `font-geist` on the root container
- Body CSS: `-webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale`

**Root Container:**
- `relative h-screen w-full overflow-hidden bg-black font-geist`

**Navbar (z-30):**
- Flex, space-between, padding: `px-6 py-5 md:px-12 lg:px-16`
- Left side: Logo text "Foldcraft" (`text-lg font-semibold tracking-tight text-white sm:text-xl`) followed by desktop nav links (hidden on mobile, flex on md+)
- Nav links: Home, Projects, Studio, Reach Us (`text-sm text-white/80 hover:text-white transition-colors`)
- Right side (desktop): "Let's Talk" button (`rounded-lg bg-white px-5 py-2 text-sm font-medium text-black hover:scale-105 transition-transform`)
- Right side (mobile): hamburger toggle button (40x40, z-50) with animated Menu/X icons from lucide-react. Menu rotates 90deg out and X rotates in with opacity and scale transitions (duration-300). Button has `active:scale-90`.

**Mobile Menu (z-20):**
- Absolute, `inset-x-0 top-0`, full-screen overlay with `bg-black/98 backdrop-blur-xl`
- Transition: `duration-500 ease-[cubic-bezier(0.16,1,0.3,1)]` toggling between `h-screen opacity-100` and `h-0 opacity-0 pointer-events-none`
- Inner content: centered vertically (`flex h-full flex-col justify-center px-8`), with a delayed fade + translate animation (delay-100, translate-y-8)
- Links: Home, Projects, Studio, Reach Us (`text-3xl font-medium text-white/90 hover:text-white`)
- Button: "Let's Talk" (`mt-6 rounded-full bg-white px-8 py-3.5 text-base font-medium text-black hover:scale-105`)
- All links/button call `setMobileMenuOpen(false)` on click

**Hero Content (z-10):**
- Flex column, justify-between, fills remaining height: `h-[calc(100vh-80px)]`
- Padding: `px-6 pb-10 pt-12 sm:pb-12 sm:pt-16 md:px-12 md:pb-16 md:pt-20 lg:px-16`

**Top Section (max-w-3xl):**
- Badge: "Brand & Visual Storytelling" (`text-xs sm:text-sm text-white/90`), with `animate-[fadeSlideUp_0.8s_ease_0.2s_both]`, margin-bottom 4 (sm:6)
- Heading h1: "Shaping visual / narratives, / one pixel at a time." with `<br/>` line breaks
  - Sizing: `text-3xl sm:text-5xl md:text-6xl lg:text-7xl`
  - Style: `font-medium leading-[1.1] tracking-tight text-white`
  - Animation: `animate-[fadeSlideUp_0.8s_ease_0.4s_both]`

**Bottom Section:**
- Paragraph: "Turning vision into reality through craft, motion, and an endless pursuit of beauty."
  - Style: `text-sm sm:text-base md:text-lg leading-relaxed text-white/60 max-w-sm sm:max-w-lg mb-5 sm:mb-6`
  - Animation: `animate-[fadeSlideUp_0.8s_ease_0.7s_both]`
- CTA Button: "Explore Work" with ArrowRight icon (size 16)
  - Style: `rounded-lg bg-white px-5 py-2.5 sm:px-6 sm:py-3 text-sm font-medium text-black hover:scale-105 transition-transform inline-flex items-center gap-2`
  - Animation: `animate-[fadeSlideUp_0.8s_ease_0.9s_both]`

**CSS Animation (in index.css):**
```css
@keyframes fadeSlideUp {
  from {
    opacity: 0;
    transform: translateY(24px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

**CSS Reset (in index.css):**
```css
* { margin: 0; padding: 0; box-sizing: border-box; }
```

**Dependencies:** React, lucide-react (ArrowRight, Menu, X), Tailwind CSS, Google Fonts Geist.

## Impact Ventures — Hero [sites/impact-ventures]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(67).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/impact-ventures.webp

Create a fullscreen hero landing page section for a design agency called "Atelier" using React, Tailwind CSS, and Lucide React icons. The section must be fully mobile responsive with an animated hamburger mobile menu. Here are the exact specifications:

**Fonts (Google Fonts):**
- "Instrument Serif" (regular + italic) for headings and mobile menu links
- "Inter" (weights 300, 400, 500, 600) as the sans-serif body font

Load them in index.html:
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Inter:wght@300;400;500;600&display=swap" rel="stylesheet" />
```

**Tailwind Config** - extend fontFamily:
```js
fontFamily: {
  'instrument-serif': ['"Instrument Serif"', 'serif'],
  sans: ['Inter', 'system-ui', 'sans-serif'],
}
```

**Background:**
A fullscreen looping autoplay muted video covering the entire viewport with `object-cover`. Video URL:
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_204103_f607742e-09da-4cf5-bb06-4e67b0a531de.mp4
```

**Layout:** The entire section is `w-full h-screen overflow-hidden` with the video absolutely positioned behind a `relative z-10` content layer that is `flex flex-col h-full`.

**Navbar:**
- Horizontal flex bar with padding `px-6 md:px-12 lg:px-16 py-5 md:py-6`
- Left side: Logo text "Atelier" (white, font-semibold, text-lg, tracking-tight, font-sans) followed by desktop nav links (hidden on mobile, shown md+): "Projects", "Expertise", "Studio", "Insights" - styled as `text-white/80 hover:text-white text-sm font-light transition-colors duration-200`
- Right side: "Reach Out" text link (hidden mobile) + "Let's Talk" button (white bg, black text, rounded-full, px-5 py-2, hidden mobile) + hamburger button (shown only on mobile, md:hidden)
- Hamburger: 3 lines (2px height, white, rounded-full) with the middle line shorter (w-4 vs w-6). On open, top/bottom lines rotate 45/-45 degrees and translate, middle fades out. Uses `cubic-bezier(0.76,0,0.24,1)` easing with 500ms duration.

**Mobile Menu Overlay (fixed inset-0 z-50, md:hidden):**
- Backdrop: `bg-black/90 backdrop-blur-xl` fading in with 700ms transition
- Content fades in with same 700ms cubic-bezier easing
- Header: matches navbar layout with logo + close button (X formed by rotated lines)
- Nav links: Stacked vertically, centered, `text-4xl sm:text-5xl font-instrument-serif`, white text, each with `border-b border-white/10`, `py-4`. On open they animate in with staggered delays (150ms + index*80ms), translating from `translate-y-8` to `translate-y-0`. Hover shifts text right with `hover:pl-4`
- Items: "Projects", "Expertise", "Studio", "Insights", "Reach Out"
- Footer: Full-width "Let's Talk" button (white bg, black text, rounded-full, py-4) with 550ms delay fade-in

**Hero Content (centered below navbar):**
- Container: `flex-1 flex flex-col items-center justify-start pt-4 sm:pt-6 md:pt-8 lg:pt-10 px-6 text-center`
- Heading (h1): `font-instrument-serif text-white text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl leading-[1.1] max-w-5xl`
  - Text content (with line breaks):
    ```
    UX <italic>and</italic> APP
    DESIGN <italic>for</italic> BOLD
    VENTURES
    ```
  - The italic words "and" and "for" use `italic font-instrument-serif` spans
- Subtext (p): `mt-4 md:mt-5 text-white/70 text-sm md:text-base font-light max-w-md leading-relaxed`
  - "We shape digital products that define brands" + line break (hidden sm:block) + "and unlock exponential growth."
- Buttons row: `mt-5 md:mt-6 flex flex-col sm:flex-row items-center gap-4`
  - Primary: "See Cases" with ArrowRight icon (lucide-react), white bg, black text, rounded-full, px-7 py-3, text-sm font-medium. On hover the arrow translates 0.5 right.
  - Secondary: "Watch Reel" with Play icon (lucide-react), transparent with `border border-white/40`, white text, rounded-full, px-7 py-3. On hover: `bg-white/10 border-white/60`

**Global CSS (index.css):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #root {
  height: 100%;
  width: 100%;
  overflow-x: hidden;
}
```

**Dependencies:** React, lucide-react (for ArrowRight and Play icons), Tailwind CSS. No other UI libraries.

## Innovation Studio — Hero [sites/innovation-studio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/Innovation%20Studio.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/innovation-studio.mp4

Build a full-screen hero section for a brand called "Nexformo" using React, TypeScript, Tailwind CSS, and Lucide React icons. Use Vite as the bundler.

### Font

Use Google Fonts "Geist" with weights 300, 400, 500. Load it via:
```
https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&display=swap
```
Set the body font-family to `'Geist', -apple-system, BlinkMacSystemFont, sans-serif`. Add `-webkit-font-smoothing: antialiased` and `-moz-osx-font-smoothing: grayscale` on the body. Register `font-geist` in Tailwind config.

### Tailwind Config

Extend the theme with:
- `fontFamily.geist`: `['Geist', '-apple-system', 'BlinkMacSystemFont', 'sans-serif']`
- `animation['spin-slow']`: `'spin 20s linear infinite'`

### Layout

The entire page is a single full-screen `<section>` with classes: `relative h-screen w-full overflow-hidden bg-black font-geist`.

### Background Video

A `<video>` element with `autoPlay muted loop playsInline` positioned `absolute inset-0 w-full h-full object-cover` (no opacity reduction, no overlay/gradient on top). The video source URL is:

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_135039_b04d00db-6ee2-4e2a-a7f5-b2dfd3d24fd2.mp4
```

No gradient overlay or darkening layer on the video.

### Content Layer

A `div` with `relative z-10 flex flex-col h-full px-5 sm:px-8 md:px-12 lg:px-16` sits on top of the video containing all UI.

### Navigation (Top)

A `<nav>` with `flex items-center justify-between pt-6 sm:pt-8`:

- **Logo (left):** Text "Nexform" in `text-white text-lg sm:text-xl font-medium tracking-tight`, with a superscript "o" using `<span className="text-[10px] align-super ml-0.5">o</span>`.
- **Links (center, desktop only):** Hidden on mobile (`hidden md:flex items-center gap-12`). Two links: "Studios--" and "Labs" in `text-white text-sm font-light tracking-wide hover:opacity-70 transition-opacity duration-300`. The first link has an em-dash appended.
- **Menu button (right):** A circular button `w-9 h-9 sm:w-10 sm:h-10 rounded-full border border-white/30 flex items-center justify-center text-white hover:border-white/60 transition-colors duration-300` containing a Lucide `Menu` icon at size 15.

### Main Content Area (Carousel)

Wrapper: `flex-1 flex items-center`. Inner: `w-full flex items-start justify-center md:justify-end md:mr-16 lg:mr-24 px-1 sm:px-0`.

### Rotating Circle Badge (desktop only)

Hidden on mobile (`hidden md:flex items-start mr-6 lg:mr-10 -mt-8 shrink-0`). A circle container `relative w-20 h-20 md:w-24 md:h-24 lg:w-28 lg:h-28` containing:
- A frosted circle background: `absolute inset-0 rounded-full bg-white/10 backdrop-blur-md`
- An SVG with `animate-spin-slow` (20s linear infinite), viewBox `0 0 200 200`, containing a circular text path (radius 70) with the text: `DESIGN * MODULES * DEVELOP * DEPLOY * ITERATE *` (using bullet character U+2022). Text style: `fill-white/80`, fontSize 10, fontWeight 300, letterSpacing 3.

### Text Content (Slide Carousel)

Container: `max-w-2xl relative`. Three slides that auto-rotate every 5 seconds with a crossfade transition.

Active slide: `opacity-100 translate-y-0 relative`
Inactive slide: `opacity-0 translate-y-4 absolute inset-0 pointer-events-none`
Transition: `transition-all duration-700 ease-[cubic-bezier(0.22,1,0.36,1)]`

**Slide 1:**
- Heading: "Exploration of *neural networks* for designing and rendering digital interfaces -- Algorithmic frameworks for composing and refining aesthetics."
- CTA: "Browse Projects--"

**Slide 2:**
- Heading: "Development of *spatial engines* for constructing and animating immersive experiences -- Procedural systems for generating and evolving visual forms."
- CTA: "View Case Study--"

**Slide 3:**
- Heading: "Architecture of *generative tools* for prototyping and deploying reactive layouts -- Modular pipelines for scaling and iterating design output."
- CTA: "Explore Process--"

The italicized words above (between asterisks) should have `underline underline-offset-4 decoration-white/60`. Dashes are em-dashes (`&mdash;`).

Heading classes: `text-white text-xl sm:text-2xl md:text-3xl lg:text-[2.1rem] font-light leading-[1.45] tracking-tight`
CTA classes: `inline-block mt-6 sm:mt-8 text-white text-xs sm:text-sm font-light tracking-wide hover:opacity-70 transition-opacity duration-300`

### Pagination Dots

Below the text carousel: `flex items-center gap-2 mt-8 sm:mt-10`. Three dots:
- Active: `w-1.5 h-1.5 sm:w-2 sm:h-2 rounded-full bg-white scale-100 transition-all duration-500`
- Inactive: `w-1.5 h-1.5 sm:w-2 sm:h-2 rounded-full bg-white/40 scale-90 hover:bg-white/60 transition-all duration-500`

Clicking a dot jumps to that slide and resets the 5-second timer.

### Bottom Section

Container: `pb-5 sm:pb-8`.

### Column Markers

`flex items-center justify-between mb-3 sm:mb-4` with three spans: "2", "H", "W" in `text-white/50 text-[10px] sm:text-xs font-light`.

### Footer Info

`flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4 sm:gap-0 border-t border-white/10 pt-4`

- **Left paragraph:** "Computational methods for streamlining industrial workflows and minimizing resource usage through *algorithmic refinement* as an emerging approach in interface architecture. Applications across digital infrastructure." The underlined phrase uses `underline underline-offset-2 decoration-white/30`. Text classes: `text-white/40 text-[9px] sm:text-[10px] md:text-xs font-light leading-relaxed max-w-md`. Line break between the two sentences on desktop (`<br className="hidden sm:block" />`), space on mobile.

- **Right text (sm:text-right):** Two lines: "Design Engineer" and "Dynamic Interface Engine" in `text-white/40 text-[9px] sm:text-[10px] md:text-xs font-light uppercase tracking-wider`.

### Mobile Menu Overlay

A full-screen overlay (`fixed inset-0 z-50`) toggled by the menu button. Uses `transition-all duration-500 ease-[cubic-bezier(0.22,1,0.36,1)]` for open/close.

- **Backdrop:** `absolute inset-0 bg-black/90 backdrop-blur-xl`, clicking it closes the menu.
- **Content container:** `relative z-10 flex flex-col h-full px-8 pt-8`, slides in from `-translate-y-8` when opening.
- **Header:** Same logo + a close button (Lucide `X` icon, size 16) in a `w-10 h-10 rounded-full border border-white/30` circle.
- **Links:** Four items: "Studios", "Labs", "Process", "Connect" in `text-white text-4xl sm:text-5xl font-light tracking-tight py-3 hover:opacity-60`. Each staggers in with `transitionDelay: 150 + i * 75ms`.
- **Footer:** Below a `border-t border-white/10`, showing email "hello@nexform.studio" and phone "+1 (424) 800-7700" in `text-white/40 text-xs font-light`. Appears with delay 450ms.

When menu is open, `document.body.style.overflow = 'hidden'`.

### CSS Reset (index.css)

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
```

## Integration SaaS — Hero [sites/integration-saas]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/wisdongate.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/integration-saas.mp4

Build a full-screen hero section landing page for a data platform called "DataVio" using React, Tailwind CSS, and Vite. Use the Geist font (from Google Fonts: `https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&display=swap`). Add the font-family to the Tailwind config under `fontFamily.geist`.

**Background:**
- Full-screen (`h-screen`) background video, auto-playing, muted, looped, with `playsInline`. Use this video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_151414_1020688d-fcb3-4b2a-9bc2-cd1dae8853dd.mp4`
- The video uses `object-cover` and on mobile the object-position is `70% center`, on `md:` and up it's `center`.

**Gradient Overlay (color: #B69198):**
- On mobile (`md:hidden`): a gradient from bottom to top -- solid `#B69198` from 0% to 30%, then fading to transparent at 55%.
- On desktop (`hidden md:block`): a gradient from left to right -- solid `#B69198` from 0% to 30%, then fading to transparent at 55%.
- Both overlays sit at `z-[1]` between the video and content.

**Navigation:**
- Flexbox row, justify-between. Padding: `px-5 sm:px-6 md:px-12 lg:px-16 py-4 md:py-5`.
- Left side: Brand name "DataVio" in white, `text-lg sm:text-xl md:text-2xl`, `font-semibold`, `tracking-tight`.
- Next to the brand (hidden on mobile, visible `md:flex`): links "Platform", "Pricing", "Solutions", "Connectors" in `text-white/80 hover:text-white text-sm font-medium` with `transition-colors duration-200`, spaced with `gap-6 lg:gap-8`.
- Right side (hidden on mobile): "Sign In" link, same text style as nav links.
- Mobile: A hamburger menu icon (Lucide `Menu`, size 24) that opens a slide-in panel from the right.

**Mobile Menu:**
- Overlay: fixed `inset-0`, `bg-black/60 backdrop-blur-sm z-40`, fades in/out with `transition-opacity duration-300`.
- Panel: fixed `top-0 right-0 h-full w-[280px] sm:w-[320px]`, white background, `z-50`, slides in from right using `translate-x-0` / `translate-x-full` with `duration-400 ease-[cubic-bezier(0.16,1,0.3,1)]`, `shadow-2xl`.
- Inside: close button (Lucide `X`, size 24) top-right, then nav links as stacked list items (`text-gray-800 text-lg font-medium py-3 border-b border-gray-100`), each staggered with `transitionDelay: (i+1)*60ms` on open. A "Sign In" CTA button at the bottom (`bg-gray-900 text-white rounded-lg px-6 py-3`) with `transitionDelay: 320ms`.
- Lock body scroll when menu is open (`document.body.style.overflow = 'hidden'`).

**Hero Content (z-10, flex-col, full height):**
- Layout: `flex-1 flex flex-col justify-between`, padding `px-5 sm:px-6 md:px-12 lg:px-16 py-4 sm:py-6 md:py-10`.
- **Heading** (top): `max-w-3xl`, padding-top `pt-2 sm:pt-4 md:pt-8`. Text: "One Central Hub" / "for Every Source" (line break between). Style: `text-3xl sm:text-4xl md:text-6xl lg:text-7xl font-normal text-white leading-[1.1] tracking-tight`.
- **Bottom content** (`max-w-3xl pb-2 sm:pb-4 md:pb-6`):
  - Paragraph: `text-white/75 text-sm sm:text-base md:text-lg leading-relaxed max-w-xl`. Starts with bold white span: "End fragmented pipelines." then regular text. Another bold white span: "transparency, governance, and scalable systems".
  - CTA Buttons (`mt-5 sm:mt-6 md:mt-8 flex flex-wrap gap-3 sm:gap-4`):
    - Primary: "Start Integration" -- `bg-white text-gray-900 text-sm font-medium rounded-lg px-5 sm:px-6 py-2.5 sm:py-3 hover:bg-white/90`.
    - Secondary: "Schedule Call" -- `text-white/80 text-sm font-medium hover:text-white`.
  - Social proof pill (`mt-6 sm:mt-8 md:mt-10`): `inline-flex items-center gap-3 sm:gap-4 bg-white/10 backdrop-blur-md border border-white/15 rounded-xl px-4 sm:px-5 py-3 sm:py-3.5`.
    - 3 overlapping avatar images (Pexels URLs: `774909`, `1222271`, `91227` with `?auto=compress&cs=tinysrgb&w=100`), each `w-7 h-7 sm:w-8 sm:h-8 rounded-full border-2 border-white/30 object-cover`, stacked with `-space-x-2.5`.
    - Text: `text-white/80 text-xs sm:text-sm leading-snug max-w-[220px] sm:max-w-xs`. Bold white span: "Adopted by 2,000+ companies and dev teams" followed by "who ship quicker, iterate safely, and keep full ownership of their pipelines."

**CSS (index.css):**
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
  font-family: 'Geist', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

**Tailwind Config:**
Extend `fontFamily` with `geist: ['Geist', 'system-ui', 'sans-serif']`.

**Key details:**
- No other sections -- just this single full-screen hero.
- Fully responsive from mobile to desktop.
- Uses only `lucide-react` for icons (Menu, X).
- All content is at `z-10`, overlays at `z-[1]`, video at default layer.

## IntelligentX — Hero [sites/intelligentx]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(61).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/intelligentx.webp

Prompt:
Build a modern React landing page using Vite, Tailwind CSS, and motion/react for elegant animations. The application must feature a highly polished, aesthetic hero section and a glassmorphic navigation bar.
1. Typography & Global CSS (src/index.css)
Import the fonts "Inter" and "Outfit" from Google Fonts.
Set --font-sans to Inter and --font-display to Outfit.
Set --color-brand-green to #9fff00 and --color-bg-base strictly to #EDEEF5.
Ensure the body uses @apply bg-bg-base text-zinc-900 font-sans antialiased; to carry the #EDEEF5 background throughout the entire page.
2. Component Structure (src/App.tsx)
Import Navbar and Hero.
Return a div containing the <Navbar /> and <main><Hero /></main>.
Set the wrapper container classes to min-h-screen bg-bg-base selection:bg-brand-green selection:text-black.
3. Navbar Component (src/components/Navbar.tsx)
Give it fixed styling: fixed top-0 left-0 w-full z-50 py-6 md:py-10 bg-gradient-to-b from-[#f1f1f1]/80 to-transparent backdrop-blur-[2px].
Container layout: A 12-column grid (grid-cols-12 max-w-7xl mx-auto).
Left (Cols 1-3): A geometric flower/clover SVG icon (fill: #1a1a1a) beside the brand name "mėntality" using the display font.
Center (Cols 4-9): Desktop-only hidden nav links: "service", "patient resources", "about us", "education center". Styled small and lowercase.
Right (Cols 10-12): "find help" anchor link, a black rounded button reading "get started →", and an elegant animated hamburger toggle icon for mobile.
Include an AnimatePresence and motion.div drawer that slides down for mobile with the navigation links.
4. Hero Component (src/components/Hero.tsx)
Main styling: <section className="relative min-h-[110vh] sm:min-h-[140vh] w-full flex flex-col items-center justify-start overflow-hidden bg-bg-base">
Background Video Container:
Absolute wrapper: <div className="absolute top-[15vh] sm:top-[20vh] left-0 w-full h-[95vh] sm:h-[120vh] z-0 pointer-events-none">
The video itself should be <video autoPlay loop muted playsInline className="w-full h-full object-cover opacity-100" />
Exact CloudFront URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260603_132049_036591b8-6e92-4760-b94c-a7ea6eef315c.mp4
Gradient Mask: Below the video in the wrapper, add <div className="absolute top-0 left-0 w-full h-24 sm:h-32 bg-gradient-to-b from-bg-base to-transparent"></div> to smoothly blend the video into the #EDEEF5 background.
Hero Content Alignment: Use <div className="max-w-7xl w-full mx-auto px-8 md:px-16 lg:px-20 relative z-10 grid grid-cols-12 gap-x-4 md:gap-x-8">. Place the text in col-span-12 md:col-span-10 md:col-start-2.
Hero Header (motion.h1): Needs a slide-up fade (initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.8 }}).
Exact text formatting:
[#1a1a1a] Remix: Mentality offers
[#8e8e8e] information
(line break)
[#8e8e8e] and resources to help you manage
(line break)
[#8e8e8e] your [Eye Icon Puipl UI Element] mental wellbeing.
For the Eye Icon Element between "your" and "mental", create an inline pill-shaped visual: w-[16px] md:w-[42px] lg:w-[62px] border-[2px] border-[#1a1a1a] rounded-full inline-flex items-center justify-center containing a tiny solid black dot (w-2 h-2).
Search Pill Component:
Add a delayed slide-up animation (delay: 0.15) under the header text.
Make a custom capsule <div className="bg-white rounded-[6px] border border-black/[0.05] p-1 pl-4 flex items-center shadow-sm">.
Include an <input placeholder="Ask me anything..."> with transparent background so it looks integrated.
Trailing action button: <button className="bg-[#1a1a1a] text-white w-9 h-9 rounded-full relative"> containing an SVG chevron/arrow icon.
Architectural Edge Anchors:
Absolute middle right edge: Create a glassmorphic pill button for language switching (pl — en).
Absolute bottom left corner: Place "2024" in small neat text.
Absolute bottom right corner: Place "mental health tools" in small neat text.
Ensure there are no artificial margins/padding below the video to make sure the video takes exactly 100% of the Hero viewport, while allowing the #EDEEF5 background base to anchor the entire page cleanly.

## Interactive Discovery — Hero [sites/interactive-discovery]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(7).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/interactive-discovery.webp

Build a full-screen, dark-themed hero section for a geology brand called **Lithos**, using **React 18 + TypeScript + Vite + Tailwind CSS** and **lucide-react** for icons. The signature feature is a **cursor-following spotlight that reveals a second image** through a soft circular mask on top of a base image. Match every detail below exactly.

### Fonts
Add this to the top of `src/index.css`, then `@tailwind base/components/utilities`:
```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=Playfair+Display:ital,wght@1,400;1,500;1,600&display=swap');
* { font-family: 'Inter', sans-serif; }
.font-playfair { font-family: 'Playfair Display', serif; }
```
- Body/UI font: **Inter**.
- Display/wordmark accent: **Playfair Display, italic**.

### Asset URLs (use these exactly)
- Base image (`BG_IMAGE_1`):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_195923_b0ba8ace-1d1d-4f2c-9a28-1ab84b330680.png&w=1280&q=85`
- Reveal image (`BG_IMAGE_2`):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_201152_bba90a12-bf12-459f-91f0-51f237dbaf3b.png&w=1280&q=85`

### Layout & structure
Root wrapper: `min-h-screen bg-white tracking-[-0.02em]`, inline `fontFamily: "'Inter', sans-serif"`.

**Section** (`<section>`): `relative w-full overflow-hidden h-screen bg-black`, inline `style={{ height: '100dvh' }}`. Layers, by z-index:
1. **Base image** (`z-10`): `absolute inset-0 bg-center bg-cover bg-no-repeat`, background = `BG_IMAGE_1`.
2. **Reveal layer** (`z-30`): a `RevealLayer` component (see below) showing `BG_IMAGE_2`.
3. **Heading** (`z-50`): `absolute top-[14%] left-0 right-0 flex flex-col items-center text-center px-5 pointer-events-none`. An `<h1>` with `text-white leading-[0.95]` containing two block spans:
   - Line 1: `block font-playfair italic font-normal text-5xl sm:text-7xl md:text-8xl`, inline `letterSpacing: '-0.05em'`, text **"Layers hold"**.
   - Line 2: `block font-normal text-5xl sm:text-7xl md:text-8xl -mt-1`, inline `letterSpacing: '-0.08em'`, text **"tales of time"**.
4. **Bottom-left paragraph** (`z-50`): `hidden sm:block absolute bottom-14 left-10 md:left-14 max-w-[260px]`. `<p className="text-sm text-white/80 leading-relaxed">` — "Every layer of sediment records a chapter of our planet, from ancient seabeds to drifting ash, layered across millions of years beneath us."
5. **Bottom-right block** (`z-50`): `absolute bottom-10 sm:bottom-24 left-5 right-5 sm:left-auto sm:right-10 md:right-14 max-w-full sm:max-w-[260px] flex flex-col items-start gap-4 sm:gap-5`. Contains a `<p className="text-xs sm:text-sm text-white/80 leading-relaxed">` — "Our interactive maps let you peel back the crust to trace how stones, fossils, and deep time combine to shape the ground beneath your feet." — and a **Start Digging** button: `bg-[#e8702a] hover:bg-[#d2611f] text-white text-sm font-medium px-7 py-3 rounded-full transition-all hover:scale-[1.03] active:scale-95 hover:shadow-lg hover:shadow-[#e8702a]/30`.

### The cursor spotlight reveal (core mechanic)
In the parent, define `const SPOTLIGHT_R = 260;` and track the mouse with smoothing:
- Refs: `mouse` (raw), `smooth` (eased), `rafRef`; state `cursorPos` (init `{x:-999,y:-999}`).
- `mousemove` listener stores raw `e.clientX/clientY`.
- A `requestAnimationFrame` loop lerps: `smooth.x += (mouse.x - smooth.x) * 0.1` (same for y), then `setCursorPos`. Clean up listener + cancel RAF on unmount.

`RevealLayer({ image, cursorX, cursorY })`:
- Holds a hidden `<canvas>` (`absolute inset-0 pointer-events-none`, `style={{display:'none'}}`) sized to `window.innerWidth/Height` on mount + resize.
- A reveal `<div>` (`absolute inset-0 bg-center bg-cover bg-no-repeat z-30 pointer-events-none`) with the reveal image as background.
- On every render: clear canvas, build a **radial gradient** at `(cursorX, cursorY)` from radius 0 → `SPOTLIGHT_R` with stops:
  `0 → rgba(255,255,255,1)`, `0.4 → 1`, `0.6 → 0.75`, `0.75 → 0.4`, `0.88 → 0.12`, `1 → 0`.
  Fill an arc of radius `SPOTLIGHT_R` with it. Then `canvas.toDataURL()` and apply it as `maskImage`/`webkitMaskImage` on the reveal div with `maskSize: '100% 100%'`. This makes the second image visible only inside the soft glowing circle that trails the cursor.

### Navigation (fixed, over hero)
`<nav className="fixed top-0 left-0 right-0 z-[100] flex items-center justify-between p-4 sm:p-5">`:
- **Left**: an inline SVG logo (26×26, viewBox `0 0 256 256`, `fill="#ffffff"`, path `M 256 256 L 128 256 L 0 128 L 128 128 Z M 256 128 L 128 128 L 0 0 L 128 0 Z`) + wordmark `<span className="text-white text-2xl font-playfair italic">Lithos</span>`.
- **Center pill** (`hidden md:flex absolute left-1/2 -translate-x-1/2 bg-white/20 backdrop-blur-md border border-white/30 rounded-full px-2 py-2 items-center gap-1`): buttons **Course** (active: full white text), then **Field Guides, Geology, Plans, Live Tour** (`text-white/80 ... hover:bg-white/20 hover:text-white transition-colors`, `px-4 py-1.5 rounded-full text-sm font-medium`).
- **Right (desktop)**: `hidden md:block bg-white text-gray-900 text-sm font-semibold px-6 py-2.5 rounded-full hover:bg-gray-100` — **Sign Up**.

### Animations (premium, on load)
Add to `index.css`:
```css
@keyframes heroReveal { 0%{opacity:0;transform:translateY(28px);filter:blur(12px)} 100%{opacity:1;transform:translateY(0);filter:blur(0)} }
@keyframes heroFadeUp { 0%{opacity:0;transform:translateY(20px)} 100%{opacity:1;transform:translateY(0)} }
@keyframes heroZoom { 0%{transform:scale(1.12)} 100%{transform:scale(1)} }
.hero-anim { opacity:0; animation-fill-mode:forwards; animation-timing-function:cubic-bezier(0.16,1,0.3,1); }
.hero-reveal { animation-name:heroReveal; animation-duration:1.1s; }
.hero-fade { animation-name:heroFadeUp; animation-duration:1s; }
.hero-zoom { animation:heroZoom 1.8s cubic-bezier(0.16,1,0.3,1) forwards; }
@media (prefers-reduced-motion: reduce){ .hero-anim,.hero-zoom{ animation:none; opacity:1; } }
```
Apply:
- Base image div → add `hero-zoom` (slow Ken Burns zoom-out).
- Heading line 1 → `hero-anim hero-reveal`, inline `animationDelay: '0.25s'`; line 2 → same with `'0.42s'` (blur-rise, staggered).
- Bottom-left paragraph wrapper → `hero-anim hero-fade`, `animationDelay: '0.7s'`.
- Bottom-right wrapper → `hero-anim hero-fade`, `animationDelay: '0.85s'`.

### Responsiveness
- Heading scales `text-5xl` → `sm:text-7xl` → `md:text-8xl`.
- Center nav pill and desktop Sign Up are `hidden` below `md`; the mobile hamburger is `md:hidden`.
- Bottom-left paragraph is `hidden sm:block`; bottom-right block is full-width on mobile (`left-5 right-5`) and right-anchored from `sm`.
- Use `100dvh` so mobile browser chrome doesn't clip the section.

## Interactive Portfolio — Hero [sites/interactive-portfolio]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(15).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/interactive-portfolio.webp

Build a full-viewport photography portfolio hero section in React (Vite + TypeScript + Tailwind). Use only inline styles (no Tailwind utility classes in JSX). Import the font `Inter` (weights 400, 500, 600) via Google Fonts in `index.css`:

```
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Inter+Display:wght@500;600&display=swap');
```

Set `body { margin:0; padding:0; font-family:'Inter',sans-serif; background:white; overflow:hidden; }` and `* { box-sizing: border-box; }`.

---

**LAYOUT (App component):**

The root container is `width:100%; height:100vh; overflow:hidden; position:relative; background:white`.

Inside it, layer these elements (all `position:absolute`):

1. **Background image** -- `inset:0`, uses `backgroundImage` with this URL:
   `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_151236_784929aa-a992-4292-9938-1dd9b5296a29.png&w=1920&q=85`
   `backgroundSize:'cover'`, `backgroundPosition:'center'`.

2. **Gradient overlay** -- `inset:0`, `background:'linear-gradient(180deg, rgba(84,84,84,0) 0%, rgb(0,0,0) 100%)'`, `opacity:0.4`.

3. **Bottom blur layer** -- `bottom:0; left:50%; transform:translateX(-50%); width:100%; height:47.375%`, with `backdropFilter:'blur(10px)'`, `WebkitBackdropFilter:'blur(10px)'`, masked with `maskImage:'linear-gradient(to bottom, transparent 0%, black 40%)'` (also `-webkit-mask-image`). `zIndex:1; pointerEvents:'none'`.

4. **6 draggable ProjectCard components** positioned absolutely at specific anchor percentages.

5. **Dock bar** at `bottom:64px; left:50%; transform:translateX(-50%); zIndex:4`. Flex row, `gap:16px; padding:12px; borderRadius:24px; background:'rgba(255,255,255,0.1)'; border:'1px solid rgba(255,255,255,0.2)'; backdropFilter:'blur(5px)'`.

---

**PROJECT CARDS:**

Each project has `anchorX` and `anchorY` (percentage positions). The card is `position:absolute; left: calc(anchorX% - 52px); top: calc(anchorY% - 64px)`. Uses a custom `useDraggable()` hook to allow drag-repositioning via `transform: translate(pos.x, pos.y)`. `zIndex:2; cursor:pointer; userSelect:none`.

Card structure (flex column, center-aligned, gap 8px):
- **Image wrapper**: `padding:12px; borderRadius:8px`. On hover: `border: 2px solid rgba(255,255,255,0.2)` and `background: rgba(0,0,0,0.16)`. Otherwise transparent border and background. `transition: background 0.18s ease, border-color 0.18s ease`.
  - **Thumbnail image**: `width:80px; height:auto; borderRadius:8px; border: 1px solid rgba(255,255,255,0.2); boxShadow: 0px 1px 6px 0px rgba(0,0,0,0.08)`.
- **Title label**: On hover, `background: rgb(0,102,221); padding: 4px 8px; borderRadius:4px`. Otherwise transparent with `padding: 4px 0`. `transition: background 0.18s ease, padding 0.18s ease`.
  - Text: `fontFamily:"'Inter',sans-serif"; fontWeight:400; fontSize:16px; lineHeight:1.4em; letterSpacing:-0.04em; color:rgb(247,247,247); whiteSpace:nowrap`.

Clicking a card (only if drag distance < 5px) opens that project in a modal window.

**6 Projects data:**

| # | title | anchorX | anchorY | thumbnail |
|---|-------|---------|---------|-----------|
| 1 | "La ou dort l'eau" | 42.75 | 48.5 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260606_153138_ef8b2e9b-3d18-4b75-8df7-5bc6f0f84fca.png&w=1920&q=85` |
| 2 | "Champ Silencieux" | 26 | 29.5 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_092743_5be19a2a-e188-4bca-9ed6-74049aa3d83b.png&w=1920&q=85` |
| 3 | "Lisiere" | 23.33 | 60.88 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260530_012333_aca09e65-227f-4185-a25f-85191cfac44d.png&w=1920&q=85` |
| 4 | "Elan Brut" | 68 | 62.13 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260527_084631_63ecf071-0fd9-42e3-989a-144728ce8ddb.png&w=1920&q=85` |
| 5 | "Les Silences Miroirs" | 66.08 | 19.63 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260525_053312_b4d2b145-7bb2-4755-b7a0-79a8e81b1265.png&w=1920&q=85` |
| 6 | "Revolte douce" | 73.92 | 40.75 | `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260512_012043_9764f2d0-5c6e-4faa-94a6-a8253df08c5e.png&w=1920&q=85` |

---

**useDraggable() HOOK:**

Tracks drag state in a `useRef` (`dragging, sx, sy, ox, oy, cx, cy`). On `mousedown`: records start position and current offset. Attaches `mousemove` and `mouseup` listeners to `window`. On move: calculates delta from start, updates position state. On up: removes listeners. Returns `{ pos: {x, y}, onMouseDown, isDraggingRef }`.

---

**DOCK BAR (5 icons + 1 divider):**

Each DockIcon is a flex column (centered). On hover: shows a tooltip above and scales the icon to 1.12x.

- **Tooltip**: `position:absolute; bottom: calc(100% + 12px); left:50%; transform:translateX(-50%)`. Fades in with `opacity` transition (0.15s). White pill background (`padding:6px 12px; borderRadius:64px; boxShadow:0 4px 16px rgba(0,0,0,0.12)`). Text: Inter 500, 12px, letterSpacing -0.04em, black. Below pill: a white CSS triangle (border trick, 8px).

- **Icon button**: `width:48px; height:48px; borderRadius:28%; overflow:hidden`. On hover: `transform:scale(1.12)` with `transition: transform 0.2s cubic-bezier(0.34,1.56,0.64,1)`. Image fills the button with `objectFit:cover`.

Dock contents (left to right):
1. "About Me" icon: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_151824_5f47765e-d133-4a38-b8bc-d968a07881a3.png&w=1920&q=85` -- opens About overlay
2. "Notes" icon: `https://framerusercontent.com/images/4ar8CL6aUtjymV8jTsXrcPzXCM.svg` -- opens Notes overlay
3. **Vertical divider**: `width:1px; height:48px; background:rgba(255,255,255,0.2); borderRadius:64px`
4. "Instagram" icon: `https://framerusercontent.com/images/Q0Z0p8LOZhN2hJ2arLjEtkqQD0.png` -- links to `https://www.instagram.com/`
5. "X" icon: `https://framerusercontent.com/images/vjmmhizcqEgw5ZT5SNFQMpxD00.png` -- links to `https://www.x.com/`
6. "Behance" icon: `https://framerusercontent.com/images/edJkRGfjqjPajyxmEgsUCKVgjE.png` -- links to `https://www.behance.com/`

---

**WINDOW SHELL (shared modal component):**

Centered fixed overlay (`inset:0; display:flex; alignItems:center; justifyContent:center; zIndex:50; pointerEvents:none`). Inner panel: `width:60vw (or 70vw if "wide"); maxWidth:720px (or 840px); maxHeight:70vh; borderRadius:24px; background:white; boxShadow:0 32px 80px rgba(0,0,0,0.28); pointerEvents:all`.

**Spring-in animation**: On mount, transitions from `scale(0.8) opacity:0` to `scale(1) opacity:1` using `transition: transform 0.4s cubic-bezier(0.34,1.28,0.64,1), opacity 0.3s ease`. Uses `requestAnimationFrame` + state toggle.

**Title bar** (draggable): `height:40px; padding:0 16px; borderBottom:1px solid rgb(229,229,234); cursor:grab`. Contains 3 macOS-style traffic light circles (12px diameter, colors: `rgb(253,93,92)`, `rgb(250,201,0)`, `rgb(52,199,90)`) that close the window on click. Title text: Inter 400, 16px, color `rgb(134,134,139)`, letterSpacing -0.04em.

**Scrollable body**: `flex:1; overflowY:auto; padding:16px; gap:16px; flex column`.

---

**CSS in index.css (also include):**

```css
@keyframes springIn {
  0% { opacity:0; transform:scale(0.8); }
  100% { opacity:1; transform:scale(1); }
}
.spring-in { animation: springIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) forwards; }
```

## Lead Funnel — Hero [sites/lead-funnel]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/mux3.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/lead-funnel.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Viktor Oddy - Hero Website</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter+Tight:wght@100..900&family=Fraunces:ital,opsz,wght@1,9..144,100..900&display=swap" rel="stylesheet" />
  <script src="https://cdn.tailwindcss.com"></script>
  <script>
    tailwind.config = {
      theme: {
        extend: {
          fontFamily: {
            sans: ['"Inter Tight"', 'ui-sans-serif', 'system-ui', '-apple-system', 'sans-serif'],
            serif: ['"Fraunces"', 'Georgia', 'serif'],
          },
        },
      },
    };
  </script>
  <style>
    @keyframes cursor-blink {
      0%, 100% { opacity: 1; }
      50% { opacity: 0; }
    }
    .animate-cursor-blink {
      animation: cursor-blink 1.0s step-end infinite;
    }
    html, body {
      display: grid;
      margin: 0;
      padding: 0;
      overflow: hidden;
      height: 100%;
      width: 100%;
      background: #000000;
      user-select: none;
      touch-action: none;
      font-size: clamp(9px, 0.55vw + 0.65vh + 3px, 16px);
    }
    #root {
      display: grid;
      width: 100%;
      height: 100%;
    }
    .scene, .a3d {
      display: grid;
    }
    .scene {
      overflow: hidden;
      perspective: 35em;
      width: 100vw;
      height: 100vh;
    }
    .a3d {
      place-self: end center;
      margin-bottom: 12em;
      transform-style: preserve-3d;
    }
    .card {
      --w: 24em;
      --h: calc(var(--w) * 1);
      --ba: calc(360deg / var(--n));
      grid-area: 1/ 1;
      width: var(--w);
      height: var(--h);
      object-fit: contain;
      border-radius: 12px;
      overflow: hidden;
      backface-visibility: hidden;
      transition: opacity 0.5s ease, filter 0.5s ease;
      --card-rz: clamp(-20deg, calc(var(--vel, 0) * 1.0deg), 20deg);
      --card-skew-x: clamp(-10deg, calc(var(--vel, 0) * 0.4deg), 10deg);
      --card-z-offset: calc(var(--abs-vel, 0) * -2px);
      --card-scale: calc(1 - var(--abs-vel, 0) * 0.0025);
      transform:
        rotateY(calc(var(--i) * var(--ba)))
        translateZ(calc(var(--z-trans) + var(--card-z-offset)))
        rotateZ(var(--card-rz))
        skewX(var(--card-skew-x))
        scale(var(--card-scale));
      cursor: pointer;
    }
  </style>
</head>
<body>
  <div id="root"></div>
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
  <script type="text/babel" data-type="module" data-presets="react">
    import React, { useState, useEffect, useRef, useCallback, useMemo, forwardRef, useImperativeHandle } from "https://esm.sh/react@19.0.0";
    import { createRoot } from "https://esm.sh/react-dom@19.0.0/client";
    import { motion, AnimatePresence, useReducedMotion } from "https://esm.sh/motion@12.23.24/react";

    // ===== KineticTextReveal Component =====
    function splitIntoGraphemes(value) {
      if (typeof Intl !== "undefined" && "Segmenter" in Intl) {
        const segmenter = new Intl.Segmenter("en", { granularity: "grapheme" });
        return Array.from(segmenter.segment(value), ({ segment }) => segment);
      }
      return Array.from(value);
    }

    function getSegments(text, splitBy) {
      let animatedIndex = 0;
      if (splitBy === "lines") {
        return text.split("\n").map((line) => ({
          value: line,
          animated: line.length > 0,
          index: line.length > 0 ? animatedIndex++ : -1,
        }));
      }
      if (splitBy === "characters") {
        return splitIntoGraphemes(text).map((character) => {
          const animated = !/\s/.test(character);
          return { value: character, animated, index: animated ? animatedIndex++ : -1 };
        });
      }
      return text.split(/(\s+)/).map((part) => {
        const animated = !/^\s+$/.test(part) && part.length > 0;
        return { value: part, animated, index: animated ? animatedIndex++ : -1 };
      });
    }

    function getDelay(index, total, stagger, staggerFrom) {
      if (typeof staggerFrom === "number") return Math.abs(staggerFrom - index) * stagger;
      if (staggerFrom === "end") return (total - 1 - index) * stagger;
      if (staggerFrom === "center") return Math.abs((total - 1) / 2 - index) * stagger;
      if (staggerFrom === "edges") return Math.min(index, total - 1 - index) * stagger;
      if (staggerFrom === "random") {
        const seeded = Math.abs(Math.sin(index * 12.9898) * 43758.5453) % 1;
        return Math.floor(seeded * total) * stagger;
      }
      return index * stagger;
    }

    function getOffset(direction, distance) {
      if (direction === "down") return { x: 0, y: -distance };
      if (direction === "left") return { x: distance, y: 0 };
      if (direction === "right") return { x: -distance, y: 0 };
      return { x: 0, y: distance };
    }

    function cn(...classes) { return classes.filter(Boolean).join(" "); }

    const KineticTextReveal = forwardRef(({
      text, className, segmentClassName, maskClassName,
      splitBy = "words", direction = "up", distance = 20,
      stagger = 0.075, staggerFrom = "start",
      transition = { duration: 0.72, ease: [0.22, 1, 0.36, 1] },
      blur = true, autoPlay = true, delay = 0,
      onRevealStart, onRevealComplete, ...props
    }, ref) => {
      const shouldReduceMotion = useReducedMotion();
      const [run, setRun] = useState(0);
      const [visible, setVisible] = useState(false);
      const segments = useMemo(() => getSegments(text, splitBy), [text, splitBy]);
      const animatedTotal = segments.filter((s) => s.animated).length;

      useImperativeHandle(ref, () => ({
        play: () => { setVisible(false); requestAnimationFrame(() => { setRun((c) => c + 1); setVisible(true); onRevealStart?.(); }); },
        reset: () => setVisible(false),
      }));

      useEffect(() => {
        if (!autoPlay) return;
        const timeout = window.setTimeout(() => { setRun((c) => c + 1); setVisible(true); onRevealStart?.(); }, delay * 1000);
        return () => window.clearTimeout(timeout);
      }, [autoPlay, delay, text, onRevealStart]);

      const offset = getOffset(direction, distance);
      const variants = {
        hidden: shouldReduceMotion ? { opacity: 0 } : { opacity: 0, x: offset.x, y: offset.y, filter: blur ? "blur(6px)" : "blur(0px)" },
        visible: (index) => ({
          opacity: 1, x: 0, y: 0, filter: "blur(0px)",
          transition: shouldReduceMotion ? { duration: 0.01 } : { ...transition, delay: getDelay(index, animatedTotal, stagger, staggerFrom) },
        }),
      };

      return (
        <span className={cn("inline-flex flex-wrap whitespace-pre-wrap align-baseline justify-center", splitBy === "lines" && "flex-col items-center", className)} aria-label={text} {...props}>
          <span className="sr-only">{text}</span>
          {segments.map((segment, index) => {
            if (!segment.animated) return <span key={`${run}-${index}`} aria-hidden="true" className="inline-block">{segment.value}</span>;
            return (
              <span key={`${run}-${index}`} className={cn("inline-block overflow-hidden align-baseline pb-1", maskClassName)} aria-hidden="true">
                <motion.span custom={segment.index} variants={variants} initial="hidden" animate={visible ? "visible" : "hidden"} className={cn("inline-block will-change-transform", segmentClassName)} onAnimationComplete={segment.index === animatedTotal - 1 ? onRevealComplete : undefined}>
                  {segment.value}
                </motion.span>
              </span>
            );
          })}
        </span>
      );
    });

    // ===== PagePreloader Component =====
    const PagePreloader = ({ onComplete, images }) => {
      const [progress, setProgress] = useState(1);
      const [currentImageIndex, setCurrentImageIndex] = useState(0);
      const [isFadingOut, setIsFadingOut] = useState(false);

      useEffect(() => {
        const uniqueImages = Array.from(new Set(images));
        uniqueImages.forEach((src) => { const img = new Image(); img.src = src; });
      }, [images]);

      useEffect(() => {
        const interval = setInterval(() => { setCurrentImageIndex((prev) => (prev + 1) % images.length); }, 85);
        return () => clearInterval(interval);
      }, [images]);

      useEffect(() => {
        let currentVal = 1;
        const interval = setInterval(() => {
          const isNearEnd = currentVal > 85;
          const step = isNearEnd ? Math.floor(Math.random() * 2) + 1 : Math.floor(Math.random() * 6) + 2;
          currentVal = Math.min(100, currentVal + step);
          setProgress(currentVal);
          if (currentVal >= 100) clearInterval(interval);
        }, 45);
        return () => clearInterval(interval);
      }, []);

      const onCompleteRef = useRef(onComplete);
      useEffect(() => { onCompleteRef.current = onComplete; }, [onComplete]);

      useEffect(() => {
        if (progress === 100) {
          setIsFadingOut(true);
          const fadeTimer = setTimeout(() => { onCompleteRef.current(); }, 500);
          return () => clearTimeout(fadeTimer);
        }
      }, [progress]);

      const normalizedProgress = progress / 100;
      const blurFactor = Math.pow(normalizedProgress, 4);
      const blurAmount = blurFactor * 18;

      return (
        <div className="fixed inset-0 bg-black z-[9999] flex flex-col items-center justify-center select-none overflow-hidden transition-opacity duration-500 ease-in-out" style={{ opacity: isFadingOut ? 0 : 1, pointerEvents: isFadingOut ? "none" : "auto" }}>
          <div className="relative shadow-[0_16px_48px_rgba(0,0,0,0.85)]" style={{ width: "100px", height: "100px", borderRadius: "1.5em", overflow: "hidden", filter: `blur(${blurAmount}px)`, transform: `scale(${1 - (progress / 100) * 0.1})`, transition: "filter 0.1s ease-out, transform 0.1s ease-out", willChange: "filter, transform" }}>
            <img src={images[currentImageIndex]} alt="loading preview" style={{ width: "100%", height: "100%", objectFit: "cover", borderRadius: "1.5em" }} className="select-none pointer-events-none" referrerPolicy="no-referrer" />
          </div>
          <div className="fixed bottom-8 right-8 md:bottom-12 md:right-16 z-[10000] font-sans font-[300] tracking-[-0.015em] text-[5rem] leading-none text-white select-none pointer-events-none tabular-nums" style={{ filter: `blur(${blurAmount * 0.6}px)`, transition: "filter 0.1s ease-out", willChange: "filter" }}>
            {progress}%
          </div>
        </div>
      );
    };

    // ===== Main App =====
    const DATA = [
      "https://image.mux.com/Fha8aU022LfL14z2WB1SbgIvq901NKvnl77OaQBOJXTk4/animated.webp?width=640&fps=15",
      "https://i.ibb.co/gFyGKsKC/temp-Imagel0c-NFL-heic-202606192014.jpg",
      "https://image.mux.com/3QMFUgJOJoclCn3i3dUJJQDapAuIhKin2VesnbIVThk/animated.webp?width=640&fps=15",
      "https://i.ibb.co/7JXNrt9z/temp-Imagel0c-NFL-heic-202606192011.jpg",
      "https://image.mux.com/8v3ptTfh02ifW501AE0101Oc9likenmSljCmutT2xXSEzEk/animated.webp?width=640&fps=15",
      "https://i.ibb.co/hJj4nxBT/temp-Imagel0c-NFL-heic-202606192019.jpg",
      "https://image.mux.com/rjL6oQiSOfhaxXgbslqGUsFKnaRtqLdxwurjT6Yv5PQ/animated.webp?width=640&fps=15",
      "https://i.ibb.co/gFyGKsKC/temp-Imagel0c-NFL-heic-202606192014.jpg",
      "https://image.mux.com/WuNDVUgyyrxFhrn2QxrF1LjMS3nBwrD7xjMNnIEn6nU/animated.webp?width=640&fps=15",
      "https://i.ibb.co/7JXNrt9z/temp-Imagel0c-NFL-heic-202606192011.jpg",
      "https://image.mux.com/lc4s01TqqDHxVTc01xNacwF2tHu3CdTXQflRRS8H02WYDs/animated.webp?width=640&fps=15",
      "https://i.ibb.co/hJj4nxBT/temp-Imagel0c-NFL-heic-202606192019.jpg"
    ];

    const N = DATA.length;
    const BASE_ANGLE = 360 / N;
    const CARD_HEIGHT_EM = 24;
    const BACK_COLOR = "#000000";

    const ProgressiveBlur = ({ className = "", backgroundColor = BACK_COLOR, position = "left", width = "25%", height = "100%", blurAmount = "12px" }) => {
      const isLeft = position === "left";
      const isRight = position === "right";
      const isTop = position === "top";
      const style = { position: "absolute", pointerEvents: "none", zIndex: 10, userSelect: "none", WebkitUserSelect: "none", WebkitBackdropFilter: `blur(${blurAmount})`, backdropFilter: `blur(${blurAmount})` };
      if (isLeft || isRight) {
        style.top = 0; style[isLeft ? "left" : "right"] = 0; style.width = width; style.height = "100%";
        style.background = isLeft ? `linear-gradient(to left, transparent, ${backgroundColor})` : `linear-gradient(to right, transparent, ${backgroundColor})`;
        const mask = isLeft ? `linear-gradient(to right, ${backgroundColor} 30%, transparent)` : `linear-gradient(to left, ${backgroundColor} 30%, transparent)`;
        style.maskImage = mask; style.WebkitMaskImage = mask;
      } else {
        style.left = 0; style[isTop ? "top" : "bottom"] = 0; style.width = "100%"; style.height = height;
        style.background = isTop ? `linear-gradient(to top, transparent, ${backgroundColor})` : `linear-gradient(to bottom, transparent, ${backgroundColor})`;
        const mask = isTop ? `linear-gradient(to bottom, ${backgroundColor} 30%, transparent)` : `linear-gradient(to top, ${backgroundColor} 30%, transparent)`;
        style.maskImage = mask; style.WebkitMaskImage = mask;
      }
      return <div className={`select-none pointer-events-none ${className}`} style={style} />;
    };

    const Ticker = ({ rotationY, widthClass = "w-[270px]" }) => {
      const textRef = useRef(null);
      const [textWidth, setTextWidth] = useState(0);
      useEffect(() => {
        if (!textRef.current) return;
        const observer = new ResizeObserver((entries) => {
          for (let entry of entries) {
            const fullWidth = textRef.current?.getBoundingClientRect().width || entry.contentRect.width + 48;
            if (fullWidth > 0) setTextWidth(fullWidth);
          }
        });
        observer.observe(textRef.current);
        return () => observer.disconnect();
      }, []);

      const speedScale = -1.35;
      const rawOffset = rotationY * speedScale;
      let offset = 0;
      if (textWidth > 0) { offset = rawOffset % textWidth; if (offset > 0) offset -= textWidth; }

      const tickerContent = (
        <div className="flex flex-row shrink-0 items-center pr-5 gap-x-5">
          <span className="text-white text-[12px] md:text-[13px] font-sans font-light tracking-normal select-none leading-none">We build websites that convert.</span>
          <span className="text-white/30 text-[12px] md:text-[13px] font-sans font-light tracking-normal select-none leading-none">Designed to reduce friction and maximize leads.</span>
        </div>
      );

      return (
        <div className={`relative ${widthClass} h-[18px] overflow-hidden select-none pointer-events-none flex items-center font-sans`}>
          <div className="absolute left-0 top-0 bottom-0 w-[24px] bg-gradient-to-r from-black via-black/35 to-transparent z-10 pointer-events-none" />
          <div className="absolute right-0 top-0 bottom-0 w-[24px] bg-gradient-to-l from-black via-black/35 to-transparent z-10 pointer-events-none" />
          <div className="flex flex-row whitespace-nowrap" style={{ transform: `translateX(${offset}px)`, willChange: "transform" }}>
            <div ref={textRef} className="flex flex-row shrink-0">{tickerContent}</div>
            {tickerContent}{tickerContent}{tickerContent}
          </div>
        </div>
      );
    };

    function App() {
      const [isPreloaderActive, setIsPreloaderActive] = useState(true);
      const [step, setStep] = useState(1);
      const stepRef = useRef(1); stepRef.current = step;
      const [rawAmount, setRawAmount] = useState("");
      const [rawEmail, setRawEmail] = useState("");
      const [isFocused, setIsFocused] = useState(false);
      const [isSubmitted, setIsSubmitted] = useState(false);
      const [showCookies, setShowCookies] = useState(true);
      const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
      const inputRef = useRef(null);

      const focusInput = () => { inputRef.current?.focus(); if (step === 1) inputRef.current?.select(); };

      useEffect(() => { if (step === 2) setTimeout(() => { inputRef.current?.focus(); }, 100); }, [step]);

      const handleInputChange = (e) => {
        let val = e.target.value.replace(/,/g, ".");
        if (rawAmount === "0.0" && val !== "0.0" && val !== "") {
          const added = val.replace("0.0", "");
          if (/^[0-9]$/.test(added)) val = added;
          else if (val.length === 1 && /^[0-9.]$/.test(val)) val = val;
          else if (val.startsWith("0.0") && val.length > 3) val = val.substring(3);
          else if (val.endsWith("0.0") && val.length > 3) val = val.slice(0, -3);
        }
        let cleaned = val.replace(/[^0-9.]/g, "");
        const parts = cleaned.split(".");
        if (parts.length > 2) cleaned = parts[0] + "." + parts.slice(1).join("");
        if (parts.length === 2 && parts[1].length > 2) cleaned = parts[0] + "." + parts[1].slice(0, 2);
        if (cleaned.length > 1 && cleaned.startsWith("0") && cleaned[1] !== ".") {
          cleaned = cleaned.replace(/^0+/, "");
          if (cleaned === "" || cleaned.startsWith(".")) cleaned = "0" + cleaned;
        }
        setRawAmount(cleaned);
      };

      const formatCurrency = (val) => {
        if (!val) return "";
        const parts = val.split(".");
        const integerPart = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ".");
        return parts.length > 1 ? integerPart + "," + parts[1] : integerPart;
      };

      const formattedAmount = formatCurrency(rawAmount);
      const numericValue = parseFloat(rawAmount);
      const isEmailValid = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(rawEmail);
      const showNext = !isSubmitted && (step === 1 ? (!isNaN(numericValue) && numericValue >= 1000) : rawEmail.length >= 4);

      const getScaleFactor = (text) => {
        if (step === 2) {
          if (rawEmail === "") return 1.0;
          const len = rawEmail.length;
          if (len <= 3) return 1.0;
          return Math.max(0.35, 1.0 - (len - 3) * 0.022);
        }
        if (rawAmount === "") return 1.0;
        const parts = rawAmount.split(".");
        const integerDigits = parts[0].replace(/[^0-9]/g, "");
        const digitCount = integerDigits.length;
        let scale = 1.0;
        if (digitCount <= 3) scale = 1.0;
        else if (digitCount === 4) scale = 0.92;
        else if (digitCount === 5) scale = 0.82;
        else if (digitCount === 6) scale = 0.72;
        else if (digitCount === 7) scale = 0.62;
        else if (digitCount === 8) scale = 0.52;
        else if (digitCount === 9) scale = 0.44;
        else if (digitCount === 10) scale = 0.38;
        else scale = Math.max(0.25, 0.38 - (digitCount - 10) * 0.05);
        if (parts.length > 1 && parts[1].length > 0) scale = Math.max(0.25, scale - 0.08);
        return scale;
      };

      const scaleFactor = getScaleFactor(step === 1 ? (rawAmount !== "" ? formattedAmount : "your budget") : (rawEmail !== "" ? rawEmail : "your email"));

      const handleKeyDown = (e) => {
        if (e.key === "Enter" && showNext) {
          if (step === 1) { setStep(2); velocityRef.current = 4.5 * scrollDirectionRef.current; setTimeout(() => { inputRef.current?.focus(); }, 120); }
          else { setIsSubmitted(true); velocityRef.current = 7.5 * scrollDirectionRef.current; }
        }
      };

      const [rotationY, setRotationY] = useState(0);
      const targetRotationRef = useRef(0);
      const currentRotationRef = useRef(0);
      const isDraggingRef = useRef(false);
      const lastInputTimeRef = useRef(0);
      const clickTargetRotationRef = useRef(null);
      const scrollDirectionRef = useRef(1);
      const startXRef = useRef(0);
      const startRotationYRef = useRef(0);
      const lastDragXRef = useRef(0);
      const lastDragTimeRef = useRef(0);
      const velocityRef = useRef(0);
      const prevRotationRef = useRef(0);
      const deformationVelRef = useRef(0);
      const deformationForceRef = useRef(0);

      useEffect(() => {
        let animId;
        const tick = () => {
          const target = targetRotationRef.current;
          let current = currentRotationRef.current;
          if (isDraggingRef.current) {
            currentRotationRef.current = target;
            velocityRef.current = currentRotationRef.current - prevRotationRef.current;
          } else if (clickTargetRotationRef.current !== null) {
            const diff = clickTargetRotationRef.current - current;
            if (Math.abs(diff) < 0.05) { currentRotationRef.current = clickTargetRotationRef.current; clickTargetRotationRef.current = null; velocityRef.current = 0; }
            else { const s = diff * 0.08; currentRotationRef.current += s; velocityRef.current = s; }
            targetRotationRef.current = currentRotationRef.current;
          } else {
            const autoScrollSpeed = 0.24 * scrollDirectionRef.current;
            const decayFactor = 0.982;
            velocityRef.current = autoScrollSpeed + (velocityRef.current - autoScrollSpeed) * decayFactor;
            currentRotationRef.current += velocityRef.current;
            targetRotationRef.current = currentRotationRef.current;
          }
          const instantV = currentRotationRef.current - prevRotationRef.current;
          prevRotationRef.current = currentRotationRef.current;
          const k = 0.16, c = 0.52;
          const force = -k * (deformationVelRef.current - instantV) - c * deformationForceRef.current;
          deformationForceRef.current += force;
          deformationVelRef.current += deformationForceRef.current;
          if (Math.abs(deformationVelRef.current) > 25) deformationVelRef.current = Math.sign(deformationVelRef.current) * 25;
          if (Math.abs(deformationForceRef.current) > 8) deformationForceRef.current = Math.sign(deformationForceRef.current) * 8;
          if (Math.abs(deformationVelRef.current) < 0.001 && Math.abs(instantV) < 0.001) { deformationVelRef.current = 0; deformationForceRef.current = 0; }
          setRotationY(currentRotationRef.current);
          animId = requestAnimationFrame(tick);
        };
        animId = requestAnimationFrame(tick);
        return () => cancelAnimationFrame(animId);
      }, []);

      useEffect(() => {
        const handleWheel = (e) => {
          e.preventDefault();
          clickTargetRotationRef.current = null;
          const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
          if (delta > 0) scrollDirectionRef.current = -1;
          else if (delta < 0) scrollDirectionRef.current = 1;
          const wheelVelocityImpulse = -delta * 0.052;
          const clampedImpulse = Math.max(-8, Math.min(8, wheelVelocityImpulse));
          velocityRef.current = velocityRef.current * 0.45 + clampedImpulse * 0.55;
          lastInputTimeRef.current = Date.now();
        };
        const handleKey = (e) => {
          if (e.key === "ArrowLeft" || e.key === "ArrowUp" || e.key === "PageUp") {
            e.preventDefault();
            const currentCard = Math.round(-currentRotationRef.current / BASE_ANGLE);
            const nextRotation = -(currentCard - 1) * BASE_ANGLE;
            clickTargetRotationRef.current = nextRotation; targetRotationRef.current = nextRotation;
            scrollDirectionRef.current = 1; lastInputTimeRef.current = Date.now();
          } else if (e.key === "ArrowRight" || e.key === "ArrowDown" || e.key === "PageDown") {
            e.preventDefault();
            const currentCard = Math.round(-currentRotationRef.current / BASE_ANGLE);
            const nextRotation = -(currentCard + 1) * BASE_ANGLE;
            clickTargetRotationRef.current = nextRotation; targetRotationRef.current = nextRotation;
            scrollDirectionRef.current = -1; lastInputTimeRef.current = Date.now();
          }
        };
        window.addEventListener("wheel", handleWheel, { passive: false });
        window.addEventListener("keydown", handleKey);
        return () => { window.removeEventListener("wheel", handleWheel); window.removeEventListener("keydown", handleKey); };
      }, []);

      const handlePointerDown = (e) => {
        isDraggingRef.current = true; clickTargetRotationRef.current = null;
        startXRef.current = e.clientX; startRotationYRef.current = currentRotationRef.current;
        lastDragXRef.current = e.clientX; lastDragTimeRef.current = Date.now(); velocityRef.current = 0;
        lastInputTimeRef.current = Date.now(); e.currentTarget.setPointerCapture(e.pointerId);
      };
      const handlePointerMove = (e) => {
        if (!isDraggingRef.current) return;
        const deltaX = e.clientX - startXRef.current;
        const degrees = deltaX * 0.18;
        targetRotationRef.current = startRotationYRef.current + degrees;
        lastInputTimeRef.current = Date.now();
        if (deltaX > 0) scrollDirectionRef.current = 1; else if (deltaX < 0) scrollDirectionRef.current = -1;
        const now = Date.now(); const dt = now - lastDragTimeRef.current;
        if (dt > 0) { const dx = e.clientX - lastDragXRef.current; const frameFraction = dt / 16.666; velocityRef.current = (dx * 0.18) / frameFraction; }
        lastDragXRef.current = e.clientX; lastDragTimeRef.current = now;
      };
      const handlePointerUp = (e) => {
        if (!isDraggingRef.current) return;
        isDraggingRef.current = false; e.currentTarget.releasePointerCapture(e.pointerId);
        lastInputTimeRef.current = Date.now();
        const maxVelocity = 12;
        if (Math.abs(velocityRef.current) > maxVelocity) velocityRef.current = Math.sign(velocityRef.current) * maxVelocity;
      };

      const angleRad = Math.PI / N;
      const radiusEm = (0.5 * CARD_HEIGHT_EM + 0.5) / Math.tan(angleRad);
      const zTrans = `calc(-1 * ${radiusEm}em)`;

      const handleCardClick = (idx) => {
        lastInputTimeRef.current = Date.now();
        const currentRot = currentRotationRef.current;
        const targetAngle = idx * BASE_ANGLE;
        const currentLap = Math.round(currentRot / 360) * 360;
        const candidates = [currentLap + targetAngle, currentLap - 360 + targetAngle, currentLap + 360 + targetAngle];
        let bestTarget = candidates[0], minDistance = Math.abs(candidates[0] - currentRot);
        for (let i = 1; i < candidates.length; i++) { const dist = Math.abs(candidates[i] - currentRot); if (dist < minDistance) { minDistance = dist; bestTarget = candidates[i]; } }
        if (bestTarget > currentRot) scrollDirectionRef.current = 1; else if (bestTarget < currentRot) scrollDirectionRef.current = -1;
        clickTargetRotationRef.current = bestTarget; targetRotationRef.current = bestTarget;
      };

      const handlePreloaderComplete = useCallback(() => { setIsPreloaderActive(false); }, []);

      useEffect(() => {
        if (!isPreloaderActive && step === 1) { const timer = setTimeout(() => { inputRef.current?.focus(); }, 350); return () => clearTimeout(timer); }
      }, [isPreloaderActive, step]);

      return (
        <div className="relative w-full h-full min-h-screen flex items-center justify-center overflow-hidden" style={{ background: BACK_COLOR }}>
          {isPreloaderActive ? (
            <PagePreloader images={DATA} onComplete={handlePreloaderComplete} />
          ) : (
            <>
              {/* Header */}
              <div className="absolute top-6 left-6 right-6 z-50 flex flex-col gap-4 pointer-events-none">
                <div className="flex items-center justify-between w-full pointer-events-none">
                  <div className="flex items-center gap-4 pointer-events-auto">
                    <motion.a href="https://x.com/viktoroddy" target="_blank" rel="noopener noreferrer"
                      initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }}
                      transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8, opacity: { type: "tween", ease: "easeInOut", duration: 0.8, delay: 0.3 }, y: { type: "tween", ease: "easeInOut", duration: 0.8, delay: 0.3 } }}
                      whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
                      className="flex items-center p-[6px] pr-3.5 rounded-full bg-white/10 hover:bg-white/15 active:bg-white/20 backdrop-blur-md transition-colors duration-200 cursor-pointer select-none shadow-[0_4px_16px_rgba(0,0,0,0.4)] h-[38px] gap-2.5 group flex-shrink-0">
                      <div className="w-[26px] h-[26px] rounded-full overflow-hidden flex-shrink-0 bg-white/10 relative">
                        <img src="https://pbs.twimg.com/profile_images/1941325782829113344/buT3DYqx_400x400.jpg" alt="Viktor Oddy" referrerPolicy="no-referrer" className="w-full h-full object-cover" />
                      </div>
                      <span className="font-sans font-medium text-[12px] leading-[18px] tracking-normal text-white/80 group-hover:text-white transition-colors duration-200">@viktoroddy</span>
                    </motion.a>
                    <motion.div initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8, opacity: { type: "tween", ease: "easeInOut", duration: 0.8, delay: 0.4 }, x: { type: "tween", ease: "easeInOut", duration: 0.8, delay: 0.4 } }} className="hidden sm:block flex-shrink-0">
                      <Ticker rotationY={rotationY} />
                    </motion.div>
                  </div>
                  <div className="flex items-center h-[38px] pointer-events-auto">
                    <motion.div initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.3, duration: 0.8 }} className="hidden sm:flex items-center gap-5 md:gap-7">
                      {["About", "Portfolio", "Contact"].map((label) => (
                        <a key={label} href={`#${label.toLowerCase()}`} onClick={(e) => e.preventDefault()} className="font-sans text-[12px] md:text-[13px] text-white/30 underline hover:text-white transition-colors duration-200 cursor-pointer tracking-wide">{label}</a>
                      ))}
                    </motion.div>
                    <div className="sm:hidden flex items-center">
                      <button onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)} className="flex items-center justify-center w-[38px] h-[38px] rounded-full bg-white/10 hover:bg-white/15 active:bg-white/20 backdrop-blur-md text-white/80 hover:text-white transition-colors duration-200 shadow-[0_4px_16px_rgba(0,0,0,0.4)] cursor-pointer select-none" aria-label="Toggle menu">
                        <div className="w-4 h-[10px] relative flex flex-col justify-between">
                          <motion.span animate={isMobileMenuOpen ? { rotate: 45, y: 4.25 } : { rotate: 0, y: 0 }} transition={{ type: "spring", stiffness: 300, damping: 22 }} className="absolute top-0 left-0 w-full h-[1.5px] bg-current rounded-full" />
                          <motion.span animate={isMobileMenuOpen ? { opacity: 0, scale: 0.5 } : { opacity: 1, scale: 1 }} transition={{ duration: 0.12 }} className="absolute top-[4.25px] left-0 w-full h-[1.5px] bg-current rounded-full" />
                          <motion.span animate={isMobileMenuOpen ? { rotate: -45, y: -4.25 } : { rotate: 0, y: 0 }} transition={{ type: "spring", stiffness: 300, damping: 22 }} className="absolute bottom-0 left-0 w-full h-[1.5px] bg-current rounded-full" />
                        </div>
                      </button>
                    </div>
                  </div>
                </div>
                <div className="block sm:hidden w-full pointer-events-auto h-[18px]">
                  <Ticker rotationY={rotationY} widthClass="w-full" />
                </div>
              </div>

              {/* Mobile Menu */}
              <AnimatePresence>
                {isMobileMenuOpen && (
                  <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.3, ease: "easeInOut" }} className="fixed inset-0 bg-black/95 z-40 sm:hidden flex flex-col justify-center items-end px-12 select-none">
                    <motion.div initial="hidden" animate="visible" exit="hidden" variants={{ hidden: { transition: { staggerChildren: 0.05, staggerDirection: -1 } }, visible: { transition: { staggerChildren: 0.08, delayChildren: 0.15 } } }} className="flex flex-col gap-10 text-right items-end pr-4 pointer-events-auto w-full max-w-xs mr-2">
                      {[{ label: "About", href: "#about" }, { label: "Portfolio", href: "#portfolio" }, { label: "Contact", href: "#contact" }].map((item) => (
                        <motion.div key={item.label} className="w-full text-right" variants={{ hidden: { opacity: 0, x: 40, filter: "blur(4px)" }, visible: { opacity: 1, x: 0, filter: "blur(0px)", transition: { type: "spring", stiffness: 120, damping: 20 } } }}>
                          <a href={item.href} onClick={(e) => { e.preventDefault(); setIsMobileMenuOpen(false); }} className="inline-block text-[20px] font-sans font-light tracking-wide text-white underline decoration-white decoration-[0.5px] hover:decoration-white underline-offset-[5px] transition-all duration-200">{item.label}</a>
                        </motion.div>
                      ))}
                    </motion.div>
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Headline + Input */}
              <motion.div
                initial={{ opacity: 0, scale: 0.96 }}
                animate={{ opacity: 1, scale: 1, y: showNext ? "-1.5rem" : (isSubmitted ? "5rem" : "5rem") }}
                transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }}
                className="absolute top-[calc(7.5%_+_70px)] lg:top-[7.5%] mt-[20px] lg:mt-[30px] left-0 right-0 z-20 flex flex-col items-center justify-center text-center px-4 select-none pointer-events-none">
                <motion.h1
                  animate={{
                    scale: isSubmitted ? 1.0 : step === 1 ? (rawAmount.length >= 4 ? Math.max(0.55, 1.0 - (rawAmount.length - 3) * 0.08) : 1.0) : (rawEmail.length >= 4 ? Math.max(0.55, 1.0 - (rawEmail.length - 3) * 0.08) : 1.0),
                    y: 0,
                    opacity: isSubmitted ? 1.0 : (step === 1 ? (rawAmount.length > 0 ? 0.45 : 1.0) : (rawEmail.length > 0 ? 0.45 : 1.0)),
                    filter: isSubmitted ? "blur(0px)" : step === 1 ? `blur(${Math.min(12, rawAmount.length * 1.5)}px)` : `blur(${Math.min(12, rawEmail.length * 1.5)}px)`
                  }}
                  transition={{ type: "spring", stiffness: 300, damping: 25, mass: 0.8 }}
                  style={{ transformOrigin: "bottom center" }}
                  className={isSubmitted ? "text-white w-full max-w-4xl flex items-center justify-center font-normal px-4 md:px-0" : "text-[2.5rem] tracking-[-0.01em] text-white leading-[1.05] font-sans font-normal flex flex-col items-center"}>
                  {isSubmitted ? (
                    <div className="flex flex-col items-center justify-center text-center select-none pointer-events-auto w-full max-w-xl mx-auto px-6">
                      <motion.div initial={{ opacity: 0, scale: 0.8, y: 15 }} animate={{ opacity: 1, scale: 1, y: 0 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8, delay: 0.05 }} className="text-white mb-6">
                        <svg xmlns="http://www.w3.org/2000/svg" className="w-[48px] h-[48px]" fill="currentColor" viewBox="0 0 16 16"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0m-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.06L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z"/></svg>
                      </motion.div>
                      <div className="flex flex-col items-center leading-[1.05] text-center mb-6 text-[2.5rem] tracking-[-0.01em]">
                        <KineticTextReveal text="Submission" splitBy="characters" direction="up" distance={15} stagger={0.05} delay={0.15} blur={true} segmentClassName="text-white font-sans font-light tracking-[-0.015em] pb-1 inline-block" />
                        <KineticTextReveal text="accepted." splitBy="characters" direction="up" distance={15} stagger={0.05} delay={0.3} blur={true} className="mt-[-0.06em]" segmentClassName="font-serif italic font-[280] text-neutral-100 inline-block" />
                      </div>
                      <motion.span initial={{ opacity: 0, scale: 0.96, y: 15 }} animate={{ opacity: 1, scale: 1, y: 0 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8, delay: 0.45 }} className="text-white/80 text-[1.1rem] md:text-[1.3rem] lg:text-[1.4rem] font-sans font-light tracking-normal leading-[1.2] mb-12 max-w-[24rem]">We'll be in touch shortly.</motion.span>
                      <motion.div initial={{ opacity: 0, scale: 0.96, y: 15 }} animate={{ opacity: 1, scale: 1, y: 0 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8, delay: 0.6 }} className="flex flex-col gap-2 text-center select-none items-center justify-center text-[13px] md:text-[14px]">
                        <div className="flex flex-row items-center gap-1.5 leading-none"><span className="font-sans text-white/35 lowercase font-light">budget</span><span className="font-sans text-white font-light tracking-tight">$ {formattedAmount}</span></div>
                        <div className="flex flex-row items-center gap-1.5 leading-none mt-1"><span className="font-sans text-white/35 lowercase font-light">email</span><span className="font-sans text-white font-light break-all max-w-[280px] md:max-w-md">{rawEmail}</span></div>
                      </motion.div>
                    </div>
                  ) : (
                    <>
                      <KineticTextReveal text="More High-Intent Leads." splitBy="words" direction="up" distance={15} stagger={0.08} delay={0.1} blur={true} segmentClassName="font-sans font-[300] text-white pb-1" />
                      <KineticTextReveal text="Less Friction." splitBy="words" direction="up" distance={15} stagger={0.08} delay={0.4} blur={true} className="mt-[-0.06em]" segmentClassName="font-serif italic font-[280] text-neutral-100 pb-1" />
                    </>
                  )}
                </motion.h1>

                {!isSubmitted && (
                  <div onClick={focusInput} className="mt-[2.5rem] pointer-events-auto relative cursor-text group select-none w-full max-w-[33.75rem] h-[6rem] flex items-center justify-center text-center px-4">
                    {step === 1 ? (
                      <input ref={inputRef} key="input-budget" type="text" inputMode="decimal" value={rawAmount} onChange={handleInputChange} onKeyDown={handleKeyDown} onFocus={(e) => { setIsFocused(true); e.currentTarget.select(); }} onBlur={() => { setIsFocused(false); if (rawAmount === "" || rawAmount === "0" || rawAmount === "0.0") setRawAmount(""); }} className="absolute inset-0 w-full h-full opacity-0 cursor-text z-20" aria-label="Sum Input" />
                    ) : (
                      <input ref={inputRef} key="input-email" type="email" value={rawEmail} onChange={(e) => setRawEmail(e.target.value)} onKeyDown={handleKeyDown} onFocus={() => setIsFocused(true)} onBlur={() => setIsFocused(false)} className="absolute inset-0 w-full h-full opacity-0 cursor-text z-20" aria-label="Email Input" />
                    )}
                    <AnimatePresence mode="wait">
                      {step === 1 ? (
                        <motion.div key="budget-step" initial={{ scale: 0.92, opacity: 0, y: 15 }} animate={{ scale: scaleFactor, opacity: 1, y: 0 }} exit={{ scale: 0.92, opacity: 0, y: -15 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }} className="flex items-center justify-center select-none" style={{ transformOrigin: "center center" }}>
                          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" className={`mr-3 w-[4.5rem] h-[4.5rem] select-none transition-colors duration-150 ${rawAmount !== "" ? "text-white" : "text-white/20"}`}><path d="M10.464 8.746c.227-.18.497-.311.786-.394v2.795a2.252 2.252 0 0 1-.786-.393c-.394-.313-.546-.681-.546-1.004 0-.323.152-.691.546-1.004ZM12.75 15.662v-2.824c.347.085.664.228.921.421.427.32.579.686.579.991 0 .305-.152.671-.579.991a2.534 2.534 0 0 1-.921.42Z" /><path fillRule="evenodd" d="M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25ZM12.75 6a.75.75 0 0 0-1.5 0v.816a3.836 3.836 0 0 0-1.72.756c-.712.566-1.112 1.35-1.112 2.178 0 .829.4 1.612 1.113 2.178.502.4 1.102.647 1.719.756v2.978a2.536 2.536 0 0 1-.921-.421l-.879-.66a.75.75 0 0 0-.9 1.2l.879.66c.533.4 1.169.645 1.821.75V18a.75.75 0 0 0 1.5 0v-.81a4.124 4.124 0 0 0 1.821-.749c.745-.559 1.179-1.344 1.179-2.191 0-.847-.434-1.632-1.179-2.191a4.122 4.122 0 0 0-1.821-.75V8.354c.29.082.559.213.786.393l.415.33a.75.75 0 0 0 .933-1.175l-.415-.33a3.836 3.836 0 0 0-1.719-.755V6Z" clipRule="evenodd" /></svg>
                          <span className={`font-sans font-[300] select-none tabular-nums text-[5rem] leading-none relative flex items-center tracking-[-0.015em] transition-colors duration-150 ${rawAmount !== "" ? "text-white" : "text-white/20"}`}>
                            {rawAmount !== "" ? formattedAmount : "your budget"}
                            <span className={`w-[2px] bg-white inline-block ml-1 h-[0.85em] transition-opacity duration-150 ${isFocused ? "animate-cursor-blink opacity-100" : "opacity-0 pointer-events-none"}`} />
                          </span>
                        </motion.div>
                      ) : (
                        <motion.div key="email-step" initial={{ scale: 0.92, opacity: 0, y: 15 }} animate={{ scale: scaleFactor, opacity: 1, y: 0 }} exit={{ scale: 0.92, opacity: 0, y: -15 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }} className="flex items-center justify-center select-none" style={{ transformOrigin: "center center" }}>
                          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" className={`mr-3 w-[4.5rem] h-[4.5rem] select-none transition-colors duration-150 ${rawEmail !== "" ? "text-white" : "text-white/20"}`}><path d="M1.5 8.67v8.58a3 3 0 0 0 3 3h15a3 3 0 0 0 3-3V8.67l-8.928 5.493a3 3 0 0 1-3.144 0L1.5 8.67Z" /><path d="M22.5 6.908V6.75a3 3 0 0 0-3-3h-15a3 3 0 0 0-3 3v.158l9.714 5.978a1.5 1.5 0 0 0 1.572 0L22.5 6.908Z" /></svg>
                          <span className={`font-sans font-[300] select-none text-[5rem] leading-none relative flex items-center tracking-[-0.015em] transition-colors duration-150 ${rawEmail !== "" ? "text-white" : "text-white/20"}`}>
                            {rawEmail !== "" ? rawEmail : "your email"}
                            <span className={`w-[2px] bg-white inline-block ml-1 h-[0.85em] transition-opacity duration-150 ${isFocused ? "animate-cursor-blink opacity-100" : "opacity-0 pointer-events-none"}`} />
                          </span>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </div>
                )}

                <AnimatePresence>
                  {showNext && (
                    <motion.button initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 10 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }} whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
                      className="mt-[1.5rem] pointer-events-auto flex items-center justify-center h-[50px] md:h-[60px] px-[2.1875rem] rounded-full bg-white/10 hover:bg-white/15 text-white font-sans font-normal text-[clamp(13px,0.93rem,15px)] transition-colors duration-200 cursor-pointer select-none shadow-[0_8px_32px_rgba(255,255,255,0.02)] backdrop-blur-md"
                      onClick={() => { if (step === 1) { setStep(2); velocityRef.current = 4.5 * scrollDirectionRef.current; } else { setIsSubmitted(true); velocityRef.current = 7.5 * scrollDirectionRef.current; } }}>
                      {step === 1 ? "Okay, next" : "Submit"}
                    </motion.button>
                  )}
                </AnimatePresence>
              </motion.div>

              {/* Progressive Blurs */}
              <ProgressiveBlur position="left" backgroundColor={BACK_COLOR} />
              <ProgressiveBlur position="right" backgroundColor={BACK_COLOR} />

              {/* 3D Carousel */}
              <motion.div className="scene"
                initial={{ opacity: 0, scale: 0.9, y: "16rem" }}
                animate={{ opacity: isSubmitted ? 0 : 1, scale: isSubmitted ? 0.80 : 1, y: isSubmitted ? "15rem" : "12rem" }}
                transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1] }}
                style={{ pointerEvents: isSubmitted ? "none" : "auto" }}>
                <div className="a3d cursor-grab active:cursor-grabbing select-none"
                  style={{ "--n": N, "--z-trans": zTrans, "--vel": deformationVelRef.current, "--abs-vel": Math.abs(deformationVelRef.current), transform: `rotateY(${rotationY}deg)` }}
                  onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp} onPointerCancel={handlePointerUp}>
                  {DATA.map((imgUrl, idx) => {
                    const cardAngle = idx * BASE_ANGLE;
                    let angleDiff = (cardAngle + rotationY) % 360;
                    if (angleDiff > 180) angleDiff -= 360;
                    if (angleDiff < -180) angleDiff += 360;
                    const absDiff = Math.abs(angleDiff);
                    const isOutOfView = absDiff > 90;
                    return <img key={idx} className="card" src={imgUrl} alt="pinterest image" referrerPolicy="no-referrer" style={{ "--i": idx, opacity: isOutOfView ? 0 : 1, filter: "none", pointerEvents: isOutOfView ? "none" : "auto" }} onClick={(e) => { e.stopPropagation(); handleCardClick(idx); }} />;
                  })}
                </div>
              </motion.div>

              {/* Cookie Banner */}
              <AnimatePresence>
                {showCookies && (
                  <motion.div initial={{ opacity: 0, y: 55, scale: 0.96 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: 35, scale: 0.96 }} transition={{ type: "spring", stiffness: 350, damping: 28, mass: 0.8 }}
                    className="fixed bottom-6 left-4 right-4 md:left-1/2 md:-translate-x-1/2 z-50 flex flex-col md:flex-row items-center justify-between p-2 gap-3 md:gap-8 rounded-xl md:rounded-full bg-white/10 backdrop-blur-md shadow-[0_12px_45px_rgba(0,0,0,0.85)] w-[calc(100%-2rem)] max-w-4xl md:h-[45px]">
                    <div className="flex items-center gap-2.5 w-full md:w-auto">
                      <div className="w-[29px] h-[29px] rounded-full overflow-hidden flex-shrink-0 bg-white/10 flex items-center justify-center relative">
                        <svg className="w-[15px] h-[15px] text-white" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 24 24"><path d="M15.5 2A1.5 1.5 0 1 0 15.5 5 1.5 1.5 0 1 0 15.5 2z"/><path d="M21 5A1 1 0 1 0 21 7 1 1 0 1 0 21 5z"/><path d="m21.6,11.04c-.25-.19-.56-.25-.85-.17-.29.07-.52.11-.74.11-1.65,0-3-1.35-3-2.95,0-.03.02-.13.02-.17.01-.32-.13-.62-.37-.82-.25-.2-.58-.27-.88-.19-.29.08-.53.11-.76.11-1.65,0-3-1.35-3-3.01,0-.22.03-.45.1-.72.08-.31,0-.65-.21-.89-.21-.25-.53-.37-.85-.34C5.88,2.5,2,6.79,2,11.98c0,5.53,4.49,10.02,10,10.02s10-4.5,10-10.02v-.16c-.01-.31-.16-.59-.4-.78Zm-12.6-3.04c.55,0,1,.45,1,1s-.45,1-1,1-1-.45-1-1,.45-1,1-1Zm-1.5,6c-.83,0-1.5-.67-1.5-1.5s.67-1.5,1.5-1.5,1.5.67,1.5,1.5-.67,1.5-1.5,1.5Zm3,4c-.83,0-1.5-.67-1.5-1.5s.67-1.5,1.5-1.5,1.5.67,1.5,1.5-.67,1.5-1.5,1.5Zm2-5c-.83,0-1.5-.67-1.5-1.5s.67-1.5,1.5-1.5,1.5.67,1.5,1.5-.67,1.5-1.5,1.5Zm2.5,3c-.55,0-1-.45-1-1s.45-1,1-1,1,.45,1,1-.45,1-1,1Z"/></svg>
                      </div>
                      <p className="font-sans font-light text-[12px] md:text-[13px] leading-relaxed text-white/80 tracking-normal">
                        We use cookies to understand how you use our site. Accept to help us improve.{" "}
                        <a href="/privacy" onClick={(e) => e.preventDefault()} className="text-white/30 underline hover:text-white transition-colors">Privacy Policy</a>
                      </p>
                    </div>
                    <div className="flex items-center gap-2 w-full md:w-auto justify-end">
                      <motion.button whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }} onClick={() => { localStorage.setItem("cookie_consent", "declined"); setShowCookies(false); }} className="flex-1 md:flex-none flex items-center justify-center px-4 py-1 h-[29px] min-w-[76px] rounded-full bg-white/5 hover:bg-white/10 active:bg-white/15 text-white/80 hover:text-white transition-colors duration-200 text-[12px] font-sans font-medium cursor-pointer">Decline</motion.button>
                      <motion.button whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }} transition={{ type: "spring", stiffness: 350, damping: 25, mass: 0.8 }} onClick={() => { localStorage.setItem("cookie_consent", "accepted"); setShowCookies(false); }} className="flex-1 md:flex-none flex items-center justify-center px-4 py-1 h-[29px] min-w-[76px] rounded-full bg-white hover:bg-white/90 text-black transition-colors duration-200 text-[12px] font-sans font-medium cursor-pointer">Accept</motion.button>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </>
          )}
        </div>
      );
    }

    const root = createRoot(document.getElementById("root"));
    root.render(<App />);
  </script>
</body>
</html>

## Learnly — Hero [sites/learnly]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(82).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/learnly.webp

Build a single-page hero section titled "Learnly - Professional Learning Platform" using Vite + React + TypeScript + Tailwind. There is no Framer Motion, no inline SVG, and no icon library usage — all animation is done with pure CSS transitions/keyframes (none needed) and transforms. Use Supabase as the database if persistence is ever added.

Create the following files exactly:

---

**`package.json`**
```json
{
  "name": "vite-react-typescript-starter",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "lint": "eslint .",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit -p tsconfig.app.json"
  },
  "dependencies": {
    "@supabase/supabase-js": "^2.57.4",
    "lucide-react": "^0.344.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@eslint/js": "^9.9.1",
    "@types/react": "^18.3.5",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "autoprefixer": "^10.4.18",
    "eslint": "^9.9.1",
    "eslint-plugin-react-hooks": "^5.1.0-rc.0",
    "eslint-plugin-react-refresh": "^0.4.11",
    "globals": "^15.9.0",
    "postcss": "^8.4.35",
    "tailwindcss": "^3.4.1",
    "typescript": "^5.5.3",
    "typescript-eslint": "^8.3.0",
    "vite": "^5.4.2"
  }
}
```

---

**`tailwind.config.js`**
```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [],
};
```

---

**`postcss.config.js`**
```js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

---

**`index.html`**
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet">
    <title>Learnly - Professional Learning Platform</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

---

**`src/main.tsx`**
```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App.tsx';
import './index.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
```

---

**`src/index.css`** (full file with fonts loaded via HTML `<link>`, no `@keyframes` — all animation is CSS transitions)
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --bg-soft: #f4f4f2;
  --card-bg: #ffffff;
  --text-main: #1a1e2d;
  --text-muted: #666666;
  --accent: #fdb181;
  --accent-hover: #fa9d63;
  --dark: #1a1e2d;
  --radius-lg: 40px;
  --radius-md: 20px;
  --transition: all 0.4s cubic-bezier(0.23, 1, 0.32, 1);
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: 'Outfit', sans-serif;
  background: radial-gradient(circle at top right, #fdfdfd 0%, #f4f4f2 100%);
  color: var(--text-main);
  min-height: 100vh;
  overflow-x: hidden;
}

.c6-hero {
  width: 100%;
  max-width: 1600px;
  margin: 0 auto;
  padding: 60px 100px;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  position: relative;
}

.c6-nav {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 60px;
  position: relative;
  z-index: 100;
}
.c6-logo {
  font-size: 1.8rem;
  font-weight: 700;
  letter-spacing: -0.5px;
  z-index: 101;
}
.c6-logo span { color: var(--accent); }

.c6-menu { display: flex; gap: 40px; }
.c6-menu a {
  text-decoration: none;
  font-size: 0.95rem;
  color: var(--text-muted);
  font-weight: 500;
  transition: var(--transition);
}
.c6-menu a:hover { color: var(--text-main); }

.c6-hamburger {
  display: none;
  flex-direction: column;
  gap: 6px;
  cursor: pointer;
  background: none;
  border: none;
  padding: 10px;
  z-index: 101;
}
.c6-hamburger span {
  display: block;
  width: 28px;
  height: 2.5px;
  background: var(--text-main);
  border-radius: 2px;
  transition: var(--transition);
}

.c6-mobile-nav {
  position: fixed;
  top: 0;
  right: -100%;
  width: 80%;
  height: 100vh;
  background: white;
  z-index: 99;
  display: flex;
  flex-direction: column;
  padding: 120px 40px;
  gap: 30px;
  box-shadow: -10px 0 30px rgba(0,0,0,0.05);
  transition: right 0.5s cubic-bezier(0.23, 1, 0.32, 1);
}
.c6-mobile-nav.open { right: 0; }
.c6-mobile-nav a {
  font-size: 1.5rem;
  font-weight: 600;
  text-decoration: none;
  color: var(--text-main);
}

.c6-actions { display: flex; align-items: center; gap: 25px; }
.c6-login {
  font-size: 0.95rem;
  color: var(--text-main);
  text-decoration: none;
  font-weight: 600;
}
.c6-trial {
  background: var(--dark);
  color: white;
  padding: 12px 28px;
  border-radius: 30px;
  font-size: 0.95rem;
  text-decoration: none;
  font-weight: 600;
  transition: var(--transition);
}
.c6-trial:hover { transform: translateY(-2px); box-shadow: 0 10px 20px rgba(0,0,0,0.1); }

.c6-main {
  display: grid;
  grid-template-columns: 1fr 1.5fr;
  gap: 60px;
  align-items: center;
  margin-bottom: 60px;
}

.c6-left { padding-top: 20px; }
.c6-title {
  font-size: 5.5rem;
  line-height: 1.05;
  font-weight: 600;
  color: var(--text-main);
  margin-bottom: 40px;
  letter-spacing: -2px;
}

.c6-search-container {
  position: relative;
  width: 100%;
  max-width: 500px;
  z-index: 5;
}
.c6-search {
  display: flex;
  align-items: stretch;
  gap: 0;
  background: white;
  box-shadow: 0 15px 45px rgba(0,0,0,0.08);
  border: 1px solid #eee;
  padding: 0;
}
.c6-search input {
  flex: 1;
  border: none;
  padding: 18px 25px;
  outline: none;
  font-size: 1.1rem;
  font-family: inherit;
  color: var(--text-main);
  background: transparent;
  border-radius: 0;
  min-width: 0;
}
.c6-search input::placeholder { color: #bbb; }
.c6-search button {
  background: linear-gradient(to bottom, #8BBF77 50%, var(--accent) 50%);
  background-size: 100% 200%;
  background-position: 0 100%;
  border: none;
  padding: 16px 45px;
  border-radius: 0;
  color: var(--text-main);
  font-weight: 700;
  cursor: pointer;
  transition: all 0.5s cubic-bezier(0.23, 1, 0.32, 1);
  font-size: 1.1rem;
  overflow: hidden;
  position: relative;
}
.c6-search button:hover {
  background-position: 0 0%;
  color: white;
  transform: scale(1.02);
}

.c6-right {
  display: flex;
  gap: 15px;
  height: 550px;
}
.c6-card {
  border-radius: 16px;
  overflow: hidden;
  position: relative;
  flex: 1;
  min-width: 0;
  transition: flex 0.7s cubic-bezier(0.23, 1, 0.32, 1), transform 0.4s ease;
  cursor: pointer;
}

.c6-card:first-child { flex: 2.5; }

.c6-card:hover { flex: 2.5; transform: translateY(-5px); }

.c6-right:hover .c6-card:not(:hover) { flex: 0.8; }

.c6-card img {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  height: 100%;
  width: auto;
  max-width: none;
  display: block;
}

.c6-card::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(to top, rgba(0,0,0,0.6) 0%, transparent 60%);
}

.c6-card-content {
  position: absolute;
  bottom: 30px;
  left: 30px;
  right: 30px;
  color: white;
  z-index: 2;
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  transition: all 0.4s ease;
}

.c6-card-title { font-size: 2.2rem; font-weight: 600; line-height: 1.1; white-space: nowrap; }
.c6-card-topics { text-align: right; }
.c6-card-topics .num { font-size: 2rem; font-weight: 700; display: block; line-height: 1; }
.c6-card-topics .label { font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 2px; opacity: 0.8; }

.c6-card-side-content {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  align-items: flex-start;
  padding-bottom: 30px;
  z-index: 3;
  opacity: 1;
  transition: opacity 0.4s ease;
  background: linear-gradient(to top, rgba(0,0,0,0.6) 0%, rgba(0,0,0,0.3) 30%, transparent 50%);
}
.c6-card:hover .c6-card-side-content { opacity: 0; pointer-events: none; }

.c6-card:not(:hover) .c6-card-content { opacity: 0; transform: translateY(10px); }
.c6-card:first-child .c6-card-content { opacity: 1; transform: translateY(0); }
.c6-card:first-child .c6-card-side-content { opacity: 0; }

.c6-right:hover .c6-card:first-child:not(:hover) .c6-card-content { opacity: 0; transform: translateY(10px); }
.c6-right:hover .c6-card:first-child:not(:hover) .c6-card-side-content { opacity: 1; }

.c6-vertical-text {
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  font-size: 1.8rem;
  font-weight: 600;
  white-space: nowrap;
  color: white;
  text-transform: capitalize;
  padding: 20px 10px;
  position: relative;
  background: linear-gradient(to top, transparent 50%, #1C1D2D 50%);
  border-radius: 0;
}

.c6-bottom-info {
  text-align: center;
  margin-top: auto;
  padding: 40px 0;
}
.c6-bottom-info h3 {
  font-size: 2.5rem;
  font-weight: 600;
  color: var(--text-main);
  letter-spacing: -1px;
  margin: 0;
}

@media (max-width: 1200px) {
  .c6-hero { padding: 40px 60px; }
  .c6-title { font-size: 4.5rem; }
  .c6-main { grid-template-columns: 1fr; gap: 40px; }
  .c6-right { height: 450px; }
}

@media (max-width: 768px) {
  .c6-hero {
    padding: 30px 20px;
    overflow-x: hidden;
  }
  .c6-menu, .c6-actions { display: none; }

  .c6-hamburger { display: flex; }
  .c6-hamburger.active span:nth-child(1) { transform: translateY(8.5px) rotate(45deg); }
  .c6-hamburger.active span:nth-child(2) { opacity: 0; }
  .c6-hamburger.active span:nth-child(3) { transform: translateY(-8.5px) rotate(-45deg); }

  .c6-title { font-size: 3.5rem; letter-spacing: -1px; margin-bottom: 30px; }

  .c6-right {
    height: 400px;
    width: calc(100% + 40px);
    overflow-x: auto;
    overflow-y: hidden;
    padding: 10px 0 20px 0;
    gap: 15px;
    scroll-snap-type: x mandatory;
    -webkit-overflow-scrolling: touch;
    margin: 0 -20px;
    padding-left: 20px;
    padding-right: 20px;
    display: flex;
  }
  .c6-right::-webkit-scrollbar { display: none; }

  .c6-card {
    flex: 0 0 300px;
    scroll-snap-align: center;
    height: 100%;
    transform: none !important;
  }
  .c6-card:first-child { flex: 0 0 300px; }
  .c6-right:hover .c6-card:not(:hover) { flex: 0 0 300px; }

  .c6-card-content {
    opacity: 1 !important;
    transform: translateY(0) !important;
  }
  .c6-card-side-content {
    opacity: 0 !important;
    display: none;
  }

  .c6-card:hover { flex: 0 0 300px !important; }
  .c6-right:hover .c6-card { flex: 0 0 300px !important; }
  .c6-card-title { font-size: 1.8rem; }
  .c6-card-topics .num { font-size: 1.5rem; }

  .c6-bottom-info h3 { font-size: 1.8rem; line-height: 1.2; }

  .c6-search button {
    padding: 16px 25px;
    font-size: 1rem;
  }
  .c6-search input {
    padding: 18px 15px;
    font-size: 1rem;
    min-width: 0;
  }
}
```

---

**`src/App.tsx`** (full file — no Tailwind classes, no Framer Motion, no inline SVG)
```tsx
import { useEffect, useState } from 'react';

function App() {
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    document.body.style.overflow = menuOpen ? 'hidden' : 'auto';
  }, [menuOpen]);

  useEffect(() => {
    const onResize = () => {
      if (window.innerWidth > 768) setMenuOpen(false);
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  const closeMenu = () => setMenuOpen(false);

  const cards = [
    {
      img: 'https://images.pexels.com/photos/5212675/pexels-photo-5212675.jpeg',
      alt: 'Editing Specialist',
      label: 'Editing',
      titleTop: 'Editing',
      titleBottom: 'Module',
      num: 100,
    },
    {
      img: 'https://images.pexels.com/photos/8617763/pexels-photo-8617763.jpeg',
      alt: 'Editing Primer',
      label: 'Editing',
      titleTop: 'Editing',
      titleBottom: 'Module',
      num: 45,
    },
    {
      img: 'https://images.pexels.com/photos/6333648/pexels-photo-6333648.jpeg',
      alt: 'Commerce Journey',
      label: 'Commerce',
      titleTop: 'Commerce',
      titleBottom: 'Journey',
      num: 82,
    },
  ];

  return (
    <div className="c6-hero">
      <nav className="c6-nav">
        <div className="c6-logo">
          Learnly<span>.</span>
        </div>

        <div className="c6-menu">
          <a href="#">Chase dreams</a>
          <a href="#">Collection</a>
          <a href="#">Trades</a>
          <a href="#">Students</a>
        </div>

        <div className="c6-actions">
          <a href="#" className="c6-login">Enter</a>
          <a href="#" className="c6-trial">Try It Now</a>
        </div>

        <button
          className={`c6-hamburger ${menuOpen ? 'active' : ''}`}
          onClick={() => setMenuOpen((v) => !v)}
          aria-label="Toggle menu"
        >
          <span></span>
          <span></span>
          <span></span>
        </button>

        <div className={`c6-mobile-nav ${menuOpen ? 'open' : ''}`}>
          <a href="#" onClick={closeMenu}>Chase dreams</a>
          <a href="#" onClick={closeMenu}>Collection</a>
          <a href="#" onClick={closeMenu}>Trades</a>
          <a href="#" onClick={closeMenu}>Students</a>
          <a
            href="#"
            onClick={closeMenu}
            style={{ marginTop: 20, color: 'var(--accent)' }}
          >
            Enter
          </a>
          <a
            href="#"
            onClick={closeMenu}
            className="c6-trial"
            style={{ textAlign: 'center', color: 'white' }}
          >
            Try It Now
          </a>
        </div>
      </nav>

      <main className="c6-main">
        <div className="c6-left">
          <h1 className="c6-title">
            Study.<br />Train.<br />Rise.
          </h1>
          <div className="c6-search-container">
            <div className="c6-search">
              <input type="text" placeholder="Chase your dreams" />
              <button>Up</button>
            </div>
          </div>
        </div>

        <div className="c6-right">
          {cards.map((card, i) => (
            <div className="c6-card" key={i}>
              <img src={card.img} alt={card.alt} />
              <div className="c6-card-side-content">
                <div className="c6-vertical-text">{card.label}</div>
              </div>
              <div className="c6-card-content">
                <div className="c6-card-title">
                  {card.titleTop}<br />{card.titleBottom}
                </div>
                <div className="c6-card-topics">
                  <span className="num">{card.num}</span>
                  <span className="label">Topics</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </main>

      <footer className="c6-bottom-info">
        <h3>Boundless passes to 100+ mentorships.</h3>
      </footer>
    </div>
  );
}

export default App;
```

---

**Asset URLs (verbatim, remote — do not download):**
- `https://images.pexels.com/photos/5212675/pexels-photo-5212675.jpeg`
- `https://images.pexels.com/photos/8617763/pexels-photo-8617763.jpeg`
- `https://images.pexels.com/photos/6333648/pexels-photo-6333648.jpeg`
- Font: `https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap`

**Animation values (all CSS, no Framer Motion):**
- Root transition: `all 0.4s cubic-bezier(0.23, 1, 0.32, 1)`
- Mobile nav slide: `right 0.5s cubic-bezier(0.23, 1, 0.32, 1)`
- Card flex accordion: `flex 0.7s cubic-bezier(0.23, 1, 0.32, 1), transform 0.4s ease`
- Search button gradient slide: `all 0.5s cubic-bezier(0.23, 1, 0.32, 1)` with hover `transform: scale(1.02)`
- Card content fade/translate: `all 0.4s ease`, inactive state `translateY(10px)` + `opacity 0`
- Card hover lift: `translateY(-5px)`
- Trial button hover: `translateY(-2px)` + `box-shadow: 0 10px 20px rgba(0,0,0,0.1)`
- Hamburger top/bottom bars rotate: `translateY(±8.5px) rotate(±45deg)`; middle bar `opacity: 0`

**Breakpoints:** `max-width: 1200px` and `max-width: 768px` (mobile switches cards to horizontal scroll-snap carousel with 300px card width).

## Luminara — Hero [sites/luminara]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(98).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luminara.webp

Build a full-screen hero section using React + Tailwind CSS + Vite. It must be fully mobile-responsive.

**Fonts:**
Load these Google Fonts in `index.html`:
```
Inter (weights: 400, 500, 600) — used as the body/primary font
Instrument Serif (italic) — used for the italic accent word in the heading
```
Google Fonts link: `https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Instrument+Serif:ital@0;1&display=swap`

**CSS (index.css):**
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
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.font-serif-italic {
  font-family: 'Instrument Serif', serif;
  font-style: italic;
}
```

**Layout structure (single full-viewport section):**

1. **Background video** — absolute positioned, covers the entire viewport using `object-cover`. Autoplays, muted, loops, playsInline.
   - Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260624_052448_43259007-b7c4-4269-90bd-e3ab14e80075.mp4`

2. **Navbar** — absolute top, z-10, flex row with `justify-between`, padded `px-4 sm:px-6 lg:px-8 py-4`. Contains two pill-shaped groups:

   - **Left pill** (logo + nav links): `bg-black/40 backdrop-blur-xl rounded-full px-5 py-2.5 border border-white/10`. Contains:
     - A custom SVG logo (20x20, white fill) with this path: `M 256 192 C 256 227.346 227.346 256 192 256 L 0 256 L 0 64 C 0 28.654 28.654 0 64 0 L 256 0 Z M 128 192 L 192 192 L 192 128 L 128 128 L 128 64 L 64 64 L 64 128 L 128 128 Z`
     - Nav links (hidden on mobile, `hidden sm:flex`): "Work", "Gallery", "Plans", "Story" — white text-sm font-medium with hover:text-white/80 transition

   - **Right pill** (buttons): `bg-black/40 backdrop-blur-xl rounded-full px-2 py-1.5 border border-white/10`. Contains:
     - Ghost button: "Get Free" — white text, rounded-full, hover:bg-white/10
     - Solid button: "Get a quote" — bg-white text-black, rounded-full, hover:bg-white/90

3. **Bottom gradient overlay** — absolutely positioned at bottom, `h-[60%]`, z-[5], pointer-events-none. Gradient: `bg-gradient-to-t from-black/80 via-black/40 to-transparent`. This provides text contrast.

4. **Bottom content area** — absolute bottom, z-10, padded `px-4 sm:px-6 lg:px-8 pb-8 sm:pb-12 lg:pb-16`. Flex layout: column on mobile, row on lg+ with items-end and justify-between.

   - **Left column** (max-w-3xl):
     - Badge pill: `bg-black/40 backdrop-blur-xl rounded-full px-3 py-1 mb-4 border border-white/10` containing: "Luminara * Creative Showcase" (the asterisk is a separate span in text-white/60)
     - Heading (h1): `text-2xl sm:text-3xl md:text-4xl lg:text-5xl xl:text-6xl font-medium leading-[1.05] tracking-tight text-white`
       - Text: "Make ordinary ideas into captivating "
       - Last word "narratives." is wrapped in a span with class `font-serif-italic font-normal` (renders in Instrument Serif italic)

   - **Right column** (lg:max-w-sm lg:text-right):
     - Paragraph: "Design your ideal online presence, grow your client base while crafting pieces with love and soul." — `text-white/80 text-sm sm:text-base leading-relaxed`

**Key design details:**
- All pill elements use `bg-black/40 backdrop-blur-xl border border-white/10 rounded-full`
- The gradient overlay covers the bottom 60% of the viewport and fades from solid black/80 upward to transparent
- The section is `relative w-full h-screen overflow-hidden`
- No other pages or routing needed — single section only
- Tailwind config is default with no extensions

## Luxury Hero — Hero [sites/luxury-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(8).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luxury-hero.webp

Build a luxury real estate landing page called **"Horizon Estates"** using **React + TypeScript + Vite + Tailwind CSS**. Use `lucide-react` for icons. Do NOT reset or remove any padding, margin, or spacing from elements. Every spacing value listed below is intentional and must be preserved exactly.

---

### FONTS (load in index.html `<head>`)

```html
<link href="https://db.onlinewebfonts.com/c/60323b40d418d578b0b2d55837f67ef2?family=Magical+Source+Demo" rel="stylesheet">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600&display=swap" rel="stylesheet">
```

Tailwind config extends fontFamily:
```js
heading: ['"Magical Source Demo"', 'serif'],
geist: ['Geist', 'sans-serif'],
```

---

### GLOBAL CSS (index.css) - COPY EXACTLY

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
  font-family: 'Geist', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  overflow-x: hidden;
}

.font-heading {
  font-family: 'Magical Source Demo', serif;
}

.font-geist {
  font-family: 'Geist', sans-serif;
}
```

**IMPORTANT**: The `* { margin: 0; padding: 0 }` reset applies globally. All component spacing is then set explicitly via Tailwind classes. Do NOT add additional resets or remove the padding/margin classes on components - they counteract the global reset.

---

### APP ROOT

```tsx
<div className="bg-black">
  <Navbar />
  <Hero />
</div>
```

Nothing else. Background is pure black.

---

### NAVBAR COMPONENT

The navbar is `position: fixed` and overlays the hero. It has real padding that must not be zero.

**Container `<nav>`:**
```
className="fixed top-0 left-0 right-0 z-50 px-5 md:px-12 py-4 md:py-5 flex items-center justify-between"
```
- Horizontal padding: `px-5` (20px) on mobile, `md:px-12` (48px) on desktop
- Vertical padding: `py-4` (16px) on mobile, `md:py-5` (20px) on desktop
- These paddings are CRITICAL - they space the logo, links, and button away from screen edges

**Left - Logo SVG:**
```tsx
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="none" className={className}>
  <path d="M 256 256 L 128 256 L 0 128 L 128 128 Z M 256 128 L 128 128 L 0 0 L 128 0 Z" fill="currentColor" />
</svg>
```
Classes on the logo in the navbar: `w-7 h-7 md:w-10 md:h-10 text-white relative z-50`

**Center - Nav Links (hidden mobile, visible md+):**
```
className="hidden md:flex items-center gap-8 text-white/90 text-sm font-geist font-light tracking-wide"
```
- `gap-8` (32px) between each link - this is the spacing between nav items
- Links: "Story", "Estates", "Lifestyle", "Views", "Inquire"
- Each link: `hover:text-white transition-colors`

**Right - CTA Button (hidden mobile, visible md+):**
```
className="hidden md:flex group items-center gap-2 bg-white/90 backdrop-blur-sm rounded-full pl-5 pr-1.5 py-1.5 hover:bg-white transition-all shadow-lg"
```
- `pl-5` (20px) left padding for text breathing room
- `pr-1.5` (6px) right padding - tight because the arrow circle sits there
- `py-1.5` (6px) vertical padding
- `gap-2` (8px) between text and arrow circle
- `rounded-full` makes it pill-shaped

Button text span:
```
className="text-gray-800 text-xs md:text-sm font-geist font-medium tracking-wider uppercase"
```

Arrow circle span:
```
className="flex items-center justify-center w-7 h-7 md:w-8 md:h-8 rounded-full bg-rose-200/60 group-hover:bg-rose-300/70 transition-colors"
```
Contains `<ArrowRight className="w-3.5 h-3.5 text-gray-700" />` from lucide-react.

**Mobile Hamburger (visible below md):**
```
className="relative z-50 md:hidden flex flex-col items-center justify-center w-10 h-10"
```
- The button has a tap target of `w-10 h-10` (40px square)
- Two `<span>` lines inside:
```
className="block w-5 h-[1.5px] bg-white rounded-full transition-all duration-300 ease-[cubic-bezier(0.77,0,0.18,1)]"
```
- Closed state: first span `-translate-y-[3px]`, second span `translate-y-[3px]` (6px gap between lines)
- Open state: first span `rotate-45 translate-y-[3px]`, second span `-rotate-45 -translate-y-[0px]`

**Mobile Menu Overlay:**
- Outer: `fixed inset-0 z-40 md:hidden transition-all duration-500 ease-[cubic-bezier(0.77,0,0.18,1)]`
- Background: `absolute inset-0 bg-black/95 backdrop-blur-xl transition-opacity duration-500`
- Content container: `relative h-full flex flex-col items-center justify-center px-8`
  - `px-8` (32px) side padding on the centered content
- Links list: `flex flex-col items-center gap-6`
  - `gap-6` (24px) between each mobile nav link
- Each link: `text-white text-2xl font-heading tracking-wider uppercase hover:text-white/70 transition-colors`
- Staggered animation: each item delays by 60ms starting at 100ms (`100 + i * 60`ms)
- Animation: `opacity-0 translate-y-6` to `opacity-100 translate-y-0`, duration 500ms
- Mobile CTA appears below links with `mt-10` (40px top margin), delayed 420ms
- Mobile CTA button: same styles as desktop but with `w-7 h-7` arrow circle (no md size-up)
- When menu opens: `document.body.style.overflow = 'hidden'` (prevents background scroll)

---

### HERO COMPONENT

**Outer wrapper** - creates scrollable space:
```
className="relative h-[200dvh]"
```
This makes the total height 200% of viewport - the extra 100vh below is empty scroll space.

**Sticky inner section** - stays pinned while you scroll through the outer wrapper:
```
className="sticky top-0 w-full h-[100dvh] overflow-hidden"
```

**VIDEO 1 URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260625_174131_395bc785-bb21-4e65-abf6-27c56f0764b6.mp4`

**VIDEO 2 URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260624_055914_ee2b3b56-9a58-4885-989e-5b72a68b630d.mp4`

**Video 2 (BEHIND, rendered first in DOM):**
```html
<video ref={video2Ref} muted playsInline preload="auto" src={VIDEO_2}
  className="absolute inset-0 w-full h-full object-cover" />
```
- No autoPlay, no loop
- Always in the DOM, always visible (it just sits behind Video 1)

**Video 1 (ON TOP, rendered second in DOM so it stacks above):**
```html
<video autoPlay muted loop playsInline
  className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-700 ${
    scrolled ? 'opacity-0 pointer-events-none' : 'opacity-100'
  }`}>
  <source src={VIDEO_1} type="video/mp4" />
</video>
```
- Loops forever, auto-plays on load
- Fades to invisible (700ms) when user scrolls, revealing Video 2 behind it

**Scroll logic (JavaScript):**
```ts
const [scrolled, setScrolled] = useState(false);
const video2Ref = useRef<HTMLVideoElement>(null);
const wasScrolled = useRef(false);

useEffect(() => {
  const handleScroll = () => {
    const isScrolled = window.scrollY > 0;
    setScrolled(isScrolled);
    const v = video2Ref.current;
    if (!v) return;
    if (isScrolled && !wasScrolled.current) {
      v.currentTime = 0;
      v.play().catch(() => {});
    } else if (!isScrolled && wasScrolled.current) {
      v.pause();
    }
    wasScrolled.current = isScrolled;
  };
  window.addEventListener('scroll', handleScroll, { passive: true });
  return () => window.removeEventListener('scroll', handleScroll);
}, []);
```

**Center Logo with Concentric Circles:**

Positioning container:
```
className="absolute inset-0 flex items-center justify-center pb-[25vh] sm:pb-[30vh]"
```
- `pb-[25vh]` / `sm:pb-[30vh]` pushes the logo upward from center (toward upper-third of screen)

Circle container:
```
className="relative flex items-center justify-center w-[45vw] h-[45vw] max-w-[320px] max-h-[320px] md:w-[30vw] md:h-[30vw] md:max-w-[400px] md:max-h-[400px]"
```

Three elements animate in on mount (a `visible` state flips true after 200ms setTimeout):

1. Outer circle ring:
```
className="absolute inset-0 rounded-full border border-white/35 transition-all duration-[1200ms] ease-out"
// visible: opacity-100 scale-100 | hidden: opacity-0 scale-75
// transitionDelay: 0ms
```

2. Inner circle ring:
```
className="absolute inset-[12%] rounded-full border border-white/25 transition-all duration-[1200ms] ease-out"
// visible: opacity-100 scale-100 | hidden: opacity-0 scale-75
// transitionDelay: 150ms
```

3. Logo SVG (same Logo component from Navbar):
```
className="w-12 h-12 sm:w-16 sm:h-16 md:w-24 md:h-24 text-white"
// wrapper: transition-all duration-[1000ms] ease-out
// visible: opacity-100 scale-100 | hidden: opacity-0 scale-90
// transitionDelay: 350ms
```

**Bottom Text Block:**
```
className="absolute bottom-0 left-0 right-0 pb-10 sm:pb-12 md:pb-16 px-5 sm:px-6 md:px-12 text-center"
```
- Bottom padding: `pb-10` (40px) mobile, `pb-12` (48px) sm, `pb-16` (64px) md
- Side padding: `px-5` (20px) mobile, `px-6` (24px) sm, `px-12` (48px) md

H1:
```
className="font-heading text-white text-2xl sm:text-3xl md:text-5xl lg:text-6xl leading-[1.1] tracking-wide uppercase"
// Animation: opacity-0 translate-y-8 -> opacity-100 translate-y-0
// duration-[1000ms] ease-out, delay 600ms
```
Text: `"Where the horizon meets"` then `<br />` then `"timeless elegance"`

P:
```
className="mt-3 sm:mt-4 md:mt-6 text-white/80 font-geist font-light text-xs sm:text-sm md:text-base max-w-xs sm:max-w-md mx-auto leading-relaxed"
// Animation: opacity-0 translate-y-6 -> opacity-100 translate-y-0
// duration-[1000ms] ease-out, delay 850ms
```
- `mt-3` (12px) / `mt-4` (16px) / `mt-6` (24px) gap between heading and paragraph
- `max-w-xs` (320px) / `sm:max-w-md` (448px) constrains paragraph width
- `mx-auto` centers it

Text: `"Indulge in unparalleled seaside living where sophistication meets the endless shore."`

---

### CRITICAL NOTES TO PREVENT SPACING ISSUES

1. Do NOT use `@layer base` resets that strip button/link padding beyond what is already in the global `*` reset
2. The button `pl-5 pr-1.5 py-1.5` creates asymmetric padding intentionally - the left side has more room for text, the right side is tight against the arrow circle
3. The navbar `px-5 md:px-12` keeps content away from screen edges - never set this to 0
4. `gap-8` on the nav links creates 32px between each item - this is what prevents them from being crammed together
5. The `pb-[25vh]` on the center logo container is what pushes it toward the upper third - without it the logo would be dead center
6. All `transition-all duration-[Xms]` values use Tailwind's arbitrary value syntax with square brackets
7. The `sticky top-0` + `h-[200dvh]` outer div pattern is what makes scroll detection work - without the extra height, `window.scrollY` stays at 0

## Naturecore SaaS — Hero [sites/naturecore-saas]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(53).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/naturecore-saas.webp

Build a full-screen hero landing page for a renewable energy company using React, Vite, TypeScript, Tailwind CSS, Framer Motion, and Lucide React icons. Use the Inter font from Google Fonts (weights 300-900). The page background is `#F7F7F7`.

### Dependencies

```
react, react-dom, framer-motion, lucide-react, clsx, tailwind-merge
```

### Global CSS (`index.css`)

Import Inter from Google Fonts: `https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap`

Set `font-family: 'Inter', sans-serif` on the body.

Add a `.liquid-glass` utility class:
- `background: rgba(255, 255, 255, 0.01)` with `background-blend-mode: luminosity`
- `backdrop-filter: blur(4px)` and `-webkit-backdrop-filter: blur(4px)`
- No border, `box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1)`, `position: relative`, `overflow: hidden`
- A `::before` pseudo-element with `position: absolute; inset: 0; border-radius: inherit; padding: 1.4px` and a vertical linear gradient of white at varying opacities (`rgba(255,255,255,0.45)` at 0%/100%, `rgba(255,255,255,0.15)` at 20%/80%, `rgba(255,255,255,0)` at 40%/60%`), masked with `-webkit-mask-composite: xor` / `mask-composite: exclude` to create a glass border effect.

Add a `.tracking-tight-custom` utility with `letter-spacing: -0.06em`.

Add a `@keyframes scroll` animation: `0% { transform: translateX(0) }` to `100% { transform: translateX(-50%) }`.

### Utility: `cn()` helper

A small utility combining `clsx` and `tailwind-merge`:

```ts
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

### Component: `<StaggeredFade>`

Props: `text: string`, `className?: string`, `style?: React.CSSProperties`.

Renders a `<motion.h1>` that splits the `text` into individual characters. Each character is a `<motion.span>` with a staggered fade-in animation: each letter delays by `i * 0.03` seconds with `0.3s` duration, transitioning from `opacity: 0` to `opacity: 1`. Uses `useInView` with `once: true` to trigger on scroll into view. The base className merges `'text-xl text-center sm:text-4xl font-bold tracking-tighter md:text-6xl md:leading-[4rem]'` with the passed `className` using the `cn()` helper.

### Component: `<FadeDown>`

Props: `children: React.ReactNode`, `delay?: number` (default `0`), `className?: string`.

A `<motion.div>` wrapper that animates from `{ opacity: 0, y: -20 }` to `{ opacity: 1, y: 0 }` over `0.6s` with the specified delay. Triggers once on entering the viewport via `useInView({ once: true })`.

### Component: `<BoomerangVideoBg>`

This is the key background video component that creates a forward/reverse boomerang loop effect.

**Video URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260527_061033_0f369854-8849-4214-8787-9181479c8121.mp4`

**How it works:**

1. Renders a `<video>` element (autoPlay, muted, playsInline, crossOrigin="anonymous") that plays through once (NOT looped).
2. While the video plays, it captures every frame into offscreen `<canvas>` elements using `requestVideoFrameCallback` (with a `setInterval` at 60fps fallback for unsupported browsers). Each captured frame is scaled to a max width of 960px maintaining aspect ratio.
3. When the video `ended` event fires, it sets a `ready` state to `true`.
4. Once ready, hides the `<video>` and shows a visible `<canvas>`. A `requestAnimationFrame` loop plays back the captured frames at 30fps in a boomerang pattern: forward through all frames, then reverse back to the start, repeating infinitely.
5. The outer wrapper div has classes: `absolute inset-0 w-full h-full`.
6. Both the `<video>` and `<canvas>` have classes: `w-full h-full object-cover`.
7. Visibility is toggled via `style={{ display: ready ? 'none' : 'block' }}` on the video and the inverse on the canvas.

### Main Layout (`App.tsx`)

The root is `<div className="h-screen flex flex-col bg-[#F7F7F7] relative overflow-hidden">`.

### Video Background

The `<BoomerangVideoBg>` is placed inside a container: `<div className="fixed inset-0 z-0" style={{ top: 200 }}>`. This pushes the video 200px down from the top of the viewport so it sits behind the lower portion of the hero.

### Navigation Bar

A `<nav>` with classes `flex items-center justify-between px-4 md:px-8 py-4 md:py-6 relative z-10`.

**Left side:**
- A logo image: `<img src="/image.png" alt="LGPSM" className="h-6 md:h-7" />` (user's own logo PNG)
- A language selector: `<Globe>` icon (lucide-react, `w-4 h-4`) + "En" text, `text-sm text-black`

**Center (hidden on mobile, `hidden lg:flex items-center gap-8`):**
- Nav links: "Renewables", "Strategies", "Photovoltaic", "Wind Systems", "Packages"
- Each link: `text-sm text-gray-700 hover:text-gray-900`

**Right side:**
- "Sign In" link (hidden on mobile `hidden sm:block`): `text-sm text-gray-700 hover:text-gray-900 border border-black/20 px-4 md:px-6 py-2 md:py-2.5 rounded-full transition-colors`
- "Clean Energy" button: `px-4 md:px-6 py-2 md:py-2.5 bg-black text-white text-sm rounded-full hover:bg-gray-900 transition-colors`

### Hero Content

Wrapper: `<div className="flex-1 flex flex-col items-center px-4 md:px-8 relative pt-4 md:pt-8">` with an inner `<div className="relative z-10 flex flex-col items-center">`.

**Badge pill** (top, centered):
- Classes: `mb-3 px-3 md:px-4 py-1.5 md:py-2 border border-black/20 rounded-full flex items-center gap-1.5 md:gap-2 text-xs md:text-sm`
- Content: sun emoji, arrow, globe emoji, text "Delivering power innovate" (hidden on mobile, shortened to "Power innovate" on small screens), arrow, plant emoji

**Main Heading** (using `<StaggeredFade>`):
- Text: `"Renewable Power For Tomorrow, Infinite Clean Solutions"`
- Classes: `text-3xl sm:text-4xl md:text-5xl lg:text-6xl leading-tight font-normal text-center max-w-5xl mb-3 md:mb-4 px-4`
- Color: `#31463B` (dark green) via inline style

**Subheading** (wrapped in `<FadeDown delay={0.5}>`):
- `<p>` with classes: `text-center text-gray-600 max-w-3xl mb-4 md:mb-5 text-sm md:text-base lg:text-lg px-4`
- Text: `"Sustainable Energy Platform. Engineering, deploying, and servicing solar arrays for homes, businesses, and large-scale operations worldwide."`

**CTA Buttons** (wrapped in `<FadeDown delay={0.7}>`):
- Wrapper classes: `flex flex-col sm:flex-row items-center gap-3 md:gap-4 px-4`
- **Primary button** ("Explore Options"):
  - Classes: `pl-4 md:pl-6 pr-2 py-2 bg-gradient-to-r from-[#3C684D] to-[#4A7144] text-white rounded-full flex items-center gap-2 hover:opacity-90 transition-opacity text-sm md:text-base`
  - Contains a `<Leaf>` icon (`w-4 h-4`), the text, and a circular icon container (`w-7 h-7 md:w-8 md:h-8 rounded-full`) with inline style `background: linear-gradient(59deg, #567A5E 0%, #78A873 100%)` containing a `<Play>` icon (`w-3 h-3 md:w-4 md:h-4 fill-white text-white`)
- **Secondary button** ("Start Network"):
  - Classes: `pl-4 md:pl-6 pr-2 py-2 bg-white text-gray-700 rounded-full flex items-center gap-2 hover:bg-gray-50 transition-colors text-sm md:text-base`
  - Contains the text and a circular icon container with inline style `background: linear-gradient(59deg, #EEEEEE 0%, #CBCBCB 100%)` containing an `<ArrowRight>` icon (`w-3 h-3 md:w-4 md:h-4 fill-black text-black`)

### Color Palette Summary

| Token | Value |
|---|---|
| Page background | `#F7F7F7` |
| Heading text | `#31463B` |
| Body text | Tailwind `gray-600` |
| Nav text | Tailwind `gray-700` |
| Primary CTA gradient | `#3C684D` to `#4A7144` |
| Primary CTA icon gradient | 59deg, `#567A5E` to `#78A873` |
| Secondary CTA icon gradient | 59deg, `#EEEEEE` to `#CBCBCB` |
| Nav button | `bg-black` / `text-white` |
| Sign-in border | `border-black/20` |

### Responsive Breakpoints

All elements use Tailwind's default breakpoints (`sm:`, `md:`, `lg:`). Nav links are hidden below `lg`. Sign-in button hidden below `sm`. CTA buttons stack vertically below `sm`. Font sizes scale from `text-3xl` to `lg:text-6xl`. Padding scales from `px-4` to `md:px-8`.

## Network Hero — Hero [sites/network-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(19).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/network-hero.webp

Create a single-page React + Vite landing page for "Marketeam" -- a marketing talent platform. Use Inter (400, 500, 600, 700) and Urbanist (600, 700) from Google Fonts. The page is a full-viewport hero with a header, left content area, right animated circles visualization, and a bottom logo ticker strip.

---

### Background

Full-page background image covering the entire viewport:
```
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_111401_56af5012-2263-45d3-849a-8688084d7c2a.png&w=1280&q=85
```
Applied as `background: url(...) center center / cover no-repeat` on the root `.app` container.

---

### Header

- Flexbox row, `justify-content: space-between`, padding `24px 64px`, max-width `1920px`, centered.
- **Left side**: Logo image + nav links
  - Logo: `<img>` with height 32px from: `https://polo-pecan-73837341.figma.site/_assets/v11/17ae538989a509947a8de3892c644664895e69b1.png`
  - Nav links: "Your Team", "Solutions", "Blog", "Pricing" -- color `#000000`, 15px, font-weight 400, with underline animation on hover (scaleX from 0 to 1, transform-origin left, 0.3s ease).
- **Right side**: "Log In" link + "Join Now" button
  - Log In: color `#ffffff`, 15px, weight 500, same underline hover as nav but white.
  - Join Now: pill button (border-radius 50px), black bg (`#000000`), white text, padding `12px 26px`, 15px, weight 500. On hover a `#A068FF` fill slides in from left using `::after` with `translateX(-100%)` to `translateX(0)`, cubic-bezier(0.22, 1, 0.36, 1), 0.4s. Button uses `overflow: hidden`.
  - The button is wrapped in a `.btn-border-wrap` div that has a rotating conic-gradient border using `::before` with `inset: -3px`, `padding: 3px`, mask technique for border-only effect. The gradient is: `conic-gradient(from var(--border-angle), #A068FF, #070319, #A068FF, #070319, #A068FF)`. It rotates via CSS `@property --border-angle` from `0deg` to `360deg` in 3s linear infinite.

---

### Hero Left

- `flex: 0 1 600px`, `padding-top: 40px`
- **Heading**: Typewriter effect, font Urbanist, 64px, weight 600, line-height 64px, letter-spacing -1.5px. Text: "Unlock Top Marketing Talent You Thought Was Out of Reach -- Now Just One Click Away!". The first 67 characters are colored `#000000`, the rest `#ffffff`. A blinking purple cursor (`#A068FF`) appears during typing. Typing speed: 35ms per character, starts after 400ms delay.
- **"Start Project" button**: Same pill style as Join Now but slightly larger (padding `14px 28px`, 16px), bg `#060218`. Has a right-arrow chevron SVG icon (18x18). Hover fill slides from right (`translateX(100%)` to `translateX(0)`). Also wrapped in `.btn-border-wrap` with the same rotating gradient border. Appears after typing finishes (animation-delay 3.2s).
- **Cursor element**: A purple cursor icon (SVG: pointer arrow filled `#A068FF`) + "David" label (pill badge, bg `#A068FF`, white text, 16px, weight 500, padding `8px 16px`, border-radius 20px). Positioned `margin-left: 290px`, `margin-top: 40px`. Appears with animation-delay 3.6s.

---

### Hero Right -- Circles Visualization

- Container: `720x720px`, centered.
- 4 concentric circles (orbits), each rotating slowly:
  - Orbit 1 (innermost): 353px diameter, spins left (counterclockwise) 30s
  - Orbit 2: 501px diameter, spins right 40s
  - Orbit 3: 649px diameter, spins right 50s
  - Orbit 4 (outermost): 797px diameter, spins left 60s
- Each circle has a 1px gradient border: `linear-gradient(180deg, rgba(217, 161, 255, 0) 0%, rgba(217, 161, 255, 1) 43%, rgba(217, 161, 255, 0) 100%)` applied via the mask technique.
- **Center circle (orbit-1)**: Displays an animated count-up number "20k+" (Urbanist 64px, weight 500) and "Specialists" label (Urbanist 16px, weight 600). Counter-rotates to stay upright.
- **Avatars** placed on orbits using `transform: translate(-50%, -50%) rotate(Xdeg) translate(radius) rotate(-Xdeg)`:
  - Avatar images (58px default, some 78px/88px) from these URLs:
    - `https://polo-pecan-73837341.figma.site/_assets/v11/aa51718fb3af3637e6d666b6543fc27a175fada6.png` (orbit 1, at 270deg, 177px radius, square with border-radius 20px, purple glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/ca755f7f93c1126fb8bdbf99ab364a33aa9ab272.png` (orbit 2, at 60deg, 251px, round, yellow glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/dc01064c7093dcc32674876ee3cf5e41c4a485c6.png` (orbit 2, at 180deg, 251px, 78px, pink glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/d5470a58b02388336141575048720f19a50de832.png` (orbit 2, at 300deg, 251px, square border-radius 20px, blue glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/018736aa5d0275c4ce56cfebaf2ae3007d81ca1e.png` (orbit 3, at 130deg, 325px, 88px, pink glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/c76d8a0b99676de31c014344bfaf75bad090758d.png` (orbit 4, at 30deg, 399px, purple glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/7b1b5f039de7b54cc9913e96c1923c3b15a157fa.png` (orbit 4, at 95deg, 399px, 88px, square border-radius 24px, orange glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/9ae171d8895199349755c43fbff00e122221a027.png` (orbit 4, at 220deg, 399px, 88px, square border-radius 24px, pink glow)
    - `https://polo-pecan-73837341.figma.site/_assets/v11/926c9eb7b4bc1df846fa0e39f0b0dc3fefd80671.png` (orbit 4, at 320deg, 399px, purple glow)
  - Each avatar has a staggered fly-in animation (scale 0.3 + rotate -180deg + blur -> normal), delays from 0.6s to 2.3s.

---

### Logo Ticker (Bottom)

- Horizontal infinitely scrolling strip of partner logos, `gap: 64px`, 20s animation.
- Fade masks on left/right edges (linear-gradient mask).
- 5 unique SVG logos repeated 4x for seamless loop:
  - `https://polo-pecan-73837341.figma.site/_assets/v11/1e7b0e6fcc016cd28aec5c68990118b8c54c35a5.svg`
  - `https://polo-pecan-73837341.figma.site/_assets/v11/3eac03c183db2ae080d910159211c14843398b61.svg`
  - `https://polo-pecan-73837341.figma.site/_assets/v11/17705a4c0023a0e5a99154dfb10582adbbf4260b.svg`
  - `https://polo-pecan-73837341.figma.site/_assets/v11/0e5f442b09dc5c248e3e60d40a65505fb1887228.svg`
  - `https://polo-pecan-73837341.figma.site/_assets/v11/63f99030ceb459e3c9ab9e429cfa2353491d3816.svg`
- Each logo: `width: 137px`, `height: 40px`, `object-fit: contain`.

---

### Entrance Animations

- Header: fade-down (translateY -20px to 0, 0.8s)
- Hero left: fade-up (translateY 40px to 0, 1s)
- Hero right circles: scale-in (scale 0.85 to 1 + opacity, 1.2s, delay 0.3s)
- Logos section: fade-up, delay 0.6s
- All using `cubic-bezier(0.22, 1, 0.36, 1)` easing.

---

### Responsive Breakpoints

- **1280px**: circles scale 0.85
- **1024px**: stack layout (flex-direction column), heading 48px, circles scale 0.7, nav gap shrinks
- **768px**: hide nav, heading 36px, circles scale 0.5
- **480px**: heading 28px, circles scale 0.4, smaller buttons/logos

---

### Key Colors

- Primary accent: `#A068FF`
- Background dark: `#060218` / `#070319`
- Text dark: `#000000`
- Text light: `#ffffff`
- Body bg fallback: `#0a0a0a`

---

### Technical Details

- React (useState, useEffect, useRef), Vite build
- Custom `useCountUp` hook: animates 0 to 20 over 2s with easeOutCubic, starts after 1.2s delay
- `TypewriterHeading` component: types char by char at configurable speed
- CSS `@property --border-angle` for the animated border gradient
- No external animation libraries -- pure CSS animations + JS for typewriter/counter

## Obsidian Hero — Hero [sites/obsidian-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(18).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/obsidian-hero.webp

Create a single-page architecture / design studio landing page using React, Vite, TypeScript, and Tailwind CSS. The page is a fullscreen hero with a looping background video, overlaid navigation, headline, and two staggered project cards.

---

**TECH STACK:**
- React 18 + TypeScript + Vite
- Tailwind CSS 3
- lucide-react (icons: Grid3X3, Menu, X)
- No other dependencies

---

**FONT:**
- Load "Lexend" from `https://db.onlinewebfonts.com/c/42dbf00de1681d38477679d3eadad56a?family=Lexend` via a `<link>` in index.html
- In tailwind.config.js, extend fontFamily with: `vilsuve: ['Lexend', 'sans-serif']`

---

**LOGO (inline SVG, white, 24x24):**
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor">
  <path d="M 156 0 C 211.228 0 256 44.772 256 100 L 256 256 L 100 256 C 44.772 256 0 211.228 0 156 L 0 0 Z M 80 80 C 80 133.019 122.981 176 176 176 C 176 122.981 133.019 80 80 80 Z" />
</svg>
```
Use this as a React component `<Logo className="w-6 h-6 text-white" />` (uses `fill="currentColor"`).

---

**BACKGROUND VIDEO:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260624_054830_07627566-fd88-4b82-9ee8-b5d78b1c4f36.mp4`
- Attributes: autoPlay, muted, loop, playsInline
- Positioned: absolute inset-0, w-full h-full, object-cover
- Parent container: relative h-screen w-full overflow-hidden bg-black

---

**LAYOUT STRUCTURE (all within the fullscreen container):**

1. **Navigation bar** (z-10, relative):
   - Left: Logo + desktop nav links ("Work" active/white, "Studio" and "Connect" white/70 with hover:white)
   - Right: "Enter" text (desktop only) + hamburger Menu icon
   - Padding: px-5 sm:px-6 md:px-12, py-5 md:py-6
   - Desktop nav links hidden on mobile; hamburger shows on mobile (md:hidden), also visible on desktop

2. **Main content area** (flex-1, relative, same horizontal padding):
   - **Hero text block** (top-left, pt-4 sm:pt-8 md:pt-16, max-w-lg):
     - H1: "Shape your\nbold spaces" - font-vilsuve, text-3xl sm:text-4xl md:text-6xl lg:text-7xl, text-white, leading-[0.95], tracking-tight
     - Paragraph: "Designing bold forms, sculpting purposeful structures, and building a timeless legacy for all." - text-white/60, text-xs sm:text-sm md:text-base, max-w-xs, leading-relaxed
     - Button: "Our Projects" with Grid3X3 icon - border border-white/30, rounded-full, px-5 sm:px-6 py-2.5 sm:py-3, hover:bg-white/10

   - **Two staggered cards** (absolute bottom-right):
     - Container: absolute bottom-6 sm:bottom-8 md:bottom-12 right-5 sm:right-6 md:right-12
     - Grid: grid grid-cols-2 gap-3 sm:gap-4 md:gap-5
     - Card 1: col-start-1 row-start-1, self-end, w-36 sm:w-44 md:w-52 lg:w-60
       - Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_052633_1cc62234-04fe-4fb5-905e-66254dd3c5db.png&w=1280&q=85`
       - Label: "CRAFT" (uppercase, text-white/60, text-[10px] sm:text-xs)
       - Title: "Spaces shaped with intention." (text-sm sm:text-base md:text-lg, font-semibold)
       - Grid3X3 icon bottom-right (text-white/70)
     - Card 2: col-start-2 row-start-2, w-36 sm:w-44 md:w-52 lg:w-60
       - Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_052653_54120660-1330-4f38-92ab-a6ba7ac15397.png&w=1280&q=85`
       - Label: "ARCVAULT" (same style)
       - Title: "Form in Stone" (text-base sm:text-xl md:text-2xl, font-semibold)
       - Grid3X3 icon bottom-right
     - Both cards: rounded-xl sm:rounded-2xl, aspect-square, gradient overlay (bg-gradient-to-t from-black/60 via-black/20 to-transparent), content justified to bottom

   - **Floating labels** (lg only, absolute positioned on background):
     - "Concrete Atrium / 282 M" at top-[25%] right-[30%] - small white dot + text-xs
     - "Brutalist Arc / 67%" at top-[45%] right-[15%] - same style

   - **Scroll indicator** (absolute bottom-8 left-1/2, hidden md:flex):
     - Text "Scroll" - text-white/40, text-[10px], uppercase, tracking-[0.3em], vertical writing mode (writingMode: 'vertical-rl'), rotated 180deg

3. **Mobile menu overlay** (fixed inset-0 z-50):
   - Toggled by `menuOpen` state
   - Backdrop: bg-black/90 backdrop-blur-xl, opacity transition 500ms
   - Content slides in from top (-translate-y-8 to translate-y-0), 500ms ease-out
   - Header: Logo left, X close button right
   - Links: "Work", "Studio", "Connect", "Enter" - text-4xl, font-vilsuve, font-light, staggered entrance (each link delayed 75ms apart starting at 150ms), border-b border-white/10
   - Footer: "Crafting bold spaces for an intentional world." - text-white/40 text-xs, delayed 450ms
   - pointer-events-none when closed, pointer-events-auto when open

---

**CSS (index.css):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

No additional custom CSS needed.

---

**KEY DESIGN DETAILS:**
- Entire page is black with the video covering the background
- All text is white with various opacity levels (white, white/70, white/60, white/50, white/40)
- Fully responsive: mobile-first breakpoints at sm, md, lg
- Cards use a CSS grid stagger pattern (not absolute positioning relative to each other)
- Transitions on hover states and menu animations
- No scrolling on the page (h-screen overflow-hidden)

## Organic Odyssey — Hero [sites/organic-odyssey]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(51).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/organic-odyssey.webp

Create a full-screen cinematic hero section using React, Tailwind CSS, and Framer Motion. Use Vite with TypeScript. The dependencies required are: `react`, `react-dom`, `framer-motion`, `lucide-react`, and `tailwindcss`.

**VIDEO BACKGROUND:**
- Full-screen looping background video, absolutely positioned to fill the viewport
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260619_191346_9d19d66e-86a4-47f7-8dc6-712c1788c3b2.mp4`
- Properties: autoPlay, muted, loop, playsInline, object-cover, object-center
- Page background: `#010101`, full viewport height, overflow hidden

**FONTS (load via index.html link tags):**
- Garamond from: `https://db.onlinewebfonts.com/c/2bf40ab72ea4897a3fd9b6e48b233a19?family=Garamond`
- Geist from Google Fonts: weights 300, 400, 500
- Body font: `'Geist', -apple-system, BlinkMacSystemFont, sans-serif`
- Heading font class `.font-garamond`: `'Garamond', 'Times New Roman', serif`

**NAVIGATION:**
- Relative positioned, z-20, flexbox centered on desktop, space-between on mobile
- Brand name "Organic Visions" -- white, uppercase, letter-spacing 0.25em (mobile) / 0.3em (desktop), font-light
- Desktop nav links: "Wander", "Archive", "Story", "Connect" -- white/80, uppercase, 0.2em tracking, hover to white, 300ms transition
- Mobile: hamburger toggle using lucide-react `Menu` and `X` icons (size 22)

**MOBILE MENU (hamburger dropdown):**
- Fixed position, top-16, left-4, right-4, z-50, hidden on md+
- Uses `AnimatePresence` from framer-motion for mount/unmount animation
- Animation: fade in from y:-10 to y:0, duration 0.3s, ease 'easeOut'; reverse on exit
- Each link staggers in with opacity 0 to 1, y:-8 to 0, delay 0.05 + index*0.06
- Links: white/90, 0.25em tracking, uppercase, font-light, hover to white
- Custom glass class `.mobile-menu-glass`:
  ```css
  background: rgba(10, 10, 10, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.1);
  ```
- Rounded-2xl, py-8, gap-5, flex-col centered

**HERO CONTENT:**
- Relative z-10, flex-col centered, text-center
- Padding: px-5 (sm:px-8), pt-12 (sm:pt-16, md:pt-24)
- Heading: Two lines -- "WITNESS THE" and "HIDDEN REALM"
- Font: Garamond, sizes 4xl/6xl/8xl/9xl responsive, font-normal, white, line-height 1.08, tracking-tight, mb-6 (sm:mb-8)
- Each line uses a `StaggeredFade` component that splits text into individual characters and animates each with 0.07s stagger delay (opacity 0 to 1), triggered once when in view

**STAGGERED FADE COMPONENT:**
- Accepts `text` string prop
- Splits into individual `<motion.span>` characters
- Uses `useInView` hook (once: true) to trigger animation
- Variants: hidden = opacity 0; show = opacity 1, y:0, with delay `i * 0.07` per character

**SUBTITLE:**
- Framer Motion animated paragraph, initial opacity:0 y:20, animate opacity:1 y:0, duration 0.8s, delay 1.6s
- Text: "An odyssey through delicate living forms," (line break hidden on mobile, visible sm+) "revealed by lens and curiosity."
- White/70, font-light, leading-relaxed, max-w-xs (sm:max-w-md), mb-8 (sm:mb-10)
- Responsive sizes: text-sm / text-base / text-lg

**CTA BUTTON:**
- Framer Motion animated, initial opacity:0 y:20, animate opacity:1 y:0, duration 0.8s, delay 2.0s
- Text: "Begin the Experience"
- Uses `.liquid-glass` class, rounded-full, responsive padding px-7/px-10 py-3.5/py-4
- White/90, uppercase, tracking 0.18em/0.2em responsive

**LIQUID GLASS CSS (.liquid-glass):**
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

.liquid-glass:hover {
  background: rgba(255, 255, 255, 0.04);
  box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.15);
}

.liquid-glass:active {
  transform: scale(0.98);
}
```

**GLOBAL CSS:**
- Reset: margin 0, padding 0, box-sizing border-box on all elements
- Body: antialiased font smoothing, white text, #010101 background
- Uses Tailwind directives: @tailwind base/components/utilities

**PAGE TITLE:** "Synthetic Nature"

## Portal — Hero [sites/portal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(68).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/portal.webp

Build a password manager landing page hero section using React, TypeScript, Tailwind CSS, Framer Motion, and Lucide React icons. Here is every specification:

---

### Fonts

- **Heading font:** "Helvetica Now Display Bold" -- load via this stylesheet in `index.html`:
  ```
  <link href="https://db.onlinewebfonts.com/c/04e6981992c0e2e7642af2074ebe3901?family=Helvetica+Now+Display+Bold" rel="stylesheet">
  ```
- **Body font:** "Inter" (weights 300-900) -- load via Google Fonts in `index.css`:
  ```
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap');
  ```

### CSS Variables (defined in `:root` in `index.css`)

```
--font-heading: 'Helvetica Now Display Bold', sans-serif;
--font-body: 'Inter', sans-serif;
--color-text: #192837;
--color-accent: #7342E2;
--color-login-bg: #F2F2EE;
```

Global reset: `* { box-sizing: border-box; }`, body uses `var(--font-body)`, `var(--color-text)`, margin/padding 0.

---

### Background

Full-viewport looping background video, absolutely positioned, covering the entire page with `object-cover`. URL:

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260606_131516_eca35265-ea66-4fbd-8d52-22aae6e1a503.mp4
```

Attributes: `autoPlay`, `muted`, `loop`, `playsInline`. Classes: `absolute inset-0 z-0 w-full h-full object-cover`.

---

### Logo (inline SVG component)

A custom geometric SVG logo, 32x32, viewBox `0 0 256 256`, fill `#192837`:

```
M 64 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 L 128 64 L 128 64.5 L 161 32 L 192 0 L 256 0 L 256 64 L 192 128 L 128 128 L 128 192 L 96 223 L 63.5 256 L 0 256 L 0 192 Z M 256 192 L 224 223 L 191.5 256 L 128 256 L 128 192 L 192 128 L 256 128 Z
```

---

### Navbar

- Max-width `1280px`, centered with `margin: 0 auto`.
- Padding: `px-5 sm:px-8 py-4 sm:py-5`.
- `relative z-10`, flexbox with `justify-between`, `items-center`.
- **Left:** Logo component.
- **Center (desktop, hidden on mobile `md:flex`):** 5 nav links -- "Vault", "Plans", "Install", "News", "Help". Each is `text-sm font-medium`, color `var(--color-text)`, `transition-opacity hover:opacity-70`, gap-8.
- **Right (desktop, hidden on mobile `md:flex`):** Two pill buttons with `gap-3`:
  - "Start For Free": background `#7342E2`, white text, `text-sm font-semibold px-5 py-2.5 rounded-full`, hover shadow, active scale-95.
  - "Sign In": background `#F2F2EE`, text `var(--color-text)`, same sizing/rounding.
- **Mobile (`md:hidden`):** Hamburger button using Lucide `Menu` icon (24px). Toggles to `X` icon when open.

---

### Mobile Menu (slide-in sheet)

Uses Framer Motion `AnimatePresence`. Two layers:

1. **Backdrop:** Fixed overlay, `rgba(25,40,55,0.35)` background, `backdrop-blur(4px)`. Fades in/out over 0.3s. Clicking dismisses the menu.

2. **Sheet:** Fixed, right-aligned, `width: min(88vw, 360px)`, `height: 100dvh`, background `#CFC8C5`, box-shadow `-12px 0 48px rgba(25,40,55,0.18)`. Slides in from right with custom cubic bezier `[0.22, 1, 0.36, 1]` over 0.45s; exits with `[0.55, 0, 1, 0.45]` over 0.35s.

   Contents:
   - **Header:** Logo + circular close button (40x40, background `rgba(25,40,55,0.1)`, X icon 20px), with `whileTap={{ scale: 0.9 }}`.
   - **Divider:** 1px line, `rgba(25,40,55,0.12)`, margin `0 24px`.
   - **Nav links:** Each link staggers in from right (x: 24 to 0, delay `0.18 + i * 0.07`, duration 0.4s). Font size `1.1rem`, rounded-xl, hover `bg-black/10`.
   - **CTA buttons:** Same "Start For Free" (`#7342E2`) and "Sign In" (`#F2F2EE`) as desktop, full-width, `py-3.5 rounded-full`, font size `0.95rem`.

---

### Hero Content

- Centered container, max-width `1280px`, `relative z-10`.
- Padding top: `clamp(40px, 8vw, 72px)`, bottom `48px`.
- Inner content wrapper: max-width `660px`, centered.

**Heading (`<h1>`):**
- Font: `var(--font-heading)`.
- Size: `clamp(1.65rem, 5vw, 3rem)`.
- Line-height: `1.05`, letter-spacing: `-0.01em`.
- Color: `var(--color-text)`.
- Text-align: center.
- Two lines:
  - Line 1 (nowrap): `Lock` [Zap icon 24px] `Down Your` [LockKeyhole icon 24px] `Passwords`
  - Line 2: `with Ironclad Security` [Fingerprint icon 24px]
- All inline icons: color `#192837`, `display: inline`, `verticalAlign: middle`, `position: relative`, `top: -2px`, margin `0 4px` (Fingerprint has `marginLeft: 6px` only).
- Animates: fade-up from `y: 28`, `opacity: 0`, duration 0.6s, ease `[0.22, 1, 0.36, 1]`, delay `0 * 0.15`.

**Subtext (`<p>`):**
- Font: `var(--font-body)`.
- Size: `clamp(0.9rem, 2.5vw, 1.1rem)`.
- Color: `var(--color-text)` at `opacity: 0.8`.
- Max-width: `560px`, line-height `1.65`, text-align center.
- Copy: "Zero stress, total control. Unbreakable storage, one-tap access, and pro-grade tools for your non-stop world."
- Animates: same fade-up, delay `1 * 0.15`.

**CTA Button:**
- Pill button (`borderRadius: 50px`), background `#7342E2`, white text.
- Size: `clamp(0.9rem, 2vw, 1rem)`, padding `17px 24px`, min-width `210px`.
- Box-shadow: `0 4px 24px rgba(115,66,226,0.28)`.
- Flexbox with `justify-between`, gap `32px`.
- Label: "Get It Free" with `ArrowRightCircle` icon (20px) on right.
- Hover: `scale: 1.04, brightness(1.1)`. Tap: `scale: 0.96`.
- Animates: same fade-up, delay `2 * 0.15`.

---

### Animation System (Framer Motion variants)

All hero elements use a shared `fadeUp` variant:
```
hidden: { opacity: 0, y: 28 }
visible: (i) => ({ opacity: 1, y: 0, transition: { delay: i * 0.15, duration: 0.6, ease: [0.22, 1, 0.36, 1] } })
```

---

### Dependencies

- `react`, `react-dom` (v18)
- `framer-motion`
- `lucide-react` (icons: ArrowRightCircle, Zap, LockKeyhole, Fingerprint, Menu, X)
- Tailwind CSS 3 with default config, no custom theme extensions
- Vite + TypeScript

## Prosthetics Hero — Hero [sites/prosthetics-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(49).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/prosthetics-hero.webp

Build a React + TypeScript + Tailwind CSS single-page hero section using Vite. The entire page lives in `src/App.tsx`. No extra libraries beyond `react`, `react-dom`, `lucide-react`, and Tailwind.

**Background:**
- A fullscreen autoplaying, muted, looping, `playsInline` background `<video>` element absolutely positioned `inset-0 w-full h-full object-cover`.
- Video URL (exact): `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_215831_c6a8989c-d716-4d8d-8745-e972a2eec711.mp4`
- Root wrapper: `relative min-h-screen overflow-hidden bg-[#f0f0ee]`.
- Foreground content wrapper: `relative z-10 flex flex-col min-h-screen`.

**Logo (inline SVG component):**
- `width="18" height="18"`, `viewBox="0 0 256 256"`, `fill="none"`.
- Single path with `fill="rgb(84, 84, 84)"` and `d="M 160 88 L 194 34 L 216 0 L 256 0 L 256 40 L 221.5 93.5 L 200 128 L 256 128 L 256 256 L 96 256 L 96 168 L 64.246 220 L 40 256 L 0 256 L 0 216 L 34 162 L 56 128 L 0 128 L 0 0 L 160 0 Z"`.

**Navbar (centered, pill-style, two separate pills):**
- `<nav>` classes: `flex items-center justify-center pt-4 sm:pt-6 px-4 sm:px-8 gap-2 sm:gap-3`.
- Left circular logo container: `flex items-center justify-center rounded-full w-10 h-10 sm:w-11 sm:h-11 shrink-0`, inline style `backgroundColor: '#EDEDED'`, contains the Logo.
- Right pill container: `flex items-center gap-4 sm:gap-10 rounded-xl px-4 sm:px-8 py-2.5 sm:py-3`, inline style `backgroundColor: '#EDEDED'`.
- Nav links array: `['Story', 'Products', 'Help', 'Support']`. Each anchor: `text-[12px] sm:text-[14px] font-medium text-gray-700 hover:text-gray-900 transition-colors duration-200`.

**Hero content (bottom-left aligned):**
- Outer: `flex-1 flex items-end pb-10 sm:pb-16 lg:pb-20 px-6 sm:px-12 md:px-20 lg:px-28`.
- Inner: `max-w-xs`. Four stacked elements, each with `mb-3`:

1. Badge link: `inline-flex items-center gap-1.5 text-[11.5px] font-medium text-blue-500 hover:text-blue-600 transition-colors mb-3 group`. Text: `Seen on Shark Tank in India` followed by an arrow `→` in a span with `inline-block transition-transform duration-200 group-hover:translate-x-0.5`.

2. Headline `<h1>`: `text-[1.5rem] sm:text-[1.75rem] leading-[1.15] font-medium text-gray-900 tracking-tight mb-3`. Text: `Simple, smart prosthetics made for people who keep fighting.`

3. Subtext `<p>`: `text-[13px] text-gray-400 font-normal mb-3`. Text: `Reclaim your movement now.`

4. CTA anchor: `inline-flex items-center gap-2 text-[13px] font-medium text-blue-500 border border-blue-400 rounded-full px-5 py-2.5 hover:bg-blue-500 hover:text-white hover:border-blue-500 transition-all duration-200 group`. Text: `Try a free fitting` plus arrow `→` in span with `transition-transform duration-200 group-hover:translate-x-0.5`.

**Animations / micro-interactions:**
- Arrow spans translate right by `0.5` on group hover (`group-hover:translate-x-0.5`).
- CTA fills blue on hover (bg + text + border transitions, 200ms).
- Nav links shift from gray-700 to gray-900 on hover.

**Fonts:** Default Tailwind sans-serif system font stack (no custom font). All sizes are exact pixel/rem values above (`11.5px`, `12px`, `13px`, `14px`, `1.5rem`, `1.75rem`).

**Colors:** Page background `#f0f0ee`; pill backgrounds `#EDEDED`; accent `blue-500/600/400`; text `gray-900/700/400`.

Do not add any other sections, no Supabase wiring, no routing. Only the single hero page as described.

## Retro-Futurist — Hero [sites/retro-futurist]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(46).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/retro-futurist.webp

Build a full-screen hero landing page for a creative agency called "Mainframe" using React, TypeScript, Vite, and Tailwind CSS. Here is every detail:

---

FONTS

Load two fonts in `index.html` via these stylesheet links:
- Heading: `https://db.onlinewebfonts.com/c/5ac3fe7c6abd2f62067f266d89671492?family=HelveticaNowDisplay-Medium`
- Body: `https://db.onlinewebfonts.com/c/1aa3377e489837a26d019bba501e779d?family=HelveticaNowDisplayW01-Rg`

In `index.css`, define CSS variables:
```css
:root {
  --font-heading: 'HelveticaNowDisplay-Medium', 'Helvetica Neue', Arial, sans-serif;
  --font-body: 'HelveticaNowDisplayW01-Rg', 'Helvetica Neue', Arial, sans-serif;
}
body {
  font-family: var(--font-body);
}
```

The entire page uses `var(--font-body)` except the logo text which uses `var(--font-heading)`.

---

BACKGROUND VIDEO (mouse-scrub controlled)

- A full-screen `` element is `position: fixed; inset: 0; z-index: 0; object-fit: cover; object-position: 70% center;`.
- Video source URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260530_042513_df96a13b-6155-4f6e-8b93-c9dee66fba08.mp4`
- The video is `muted`, `playsInline`, `preload="auto"`. It does NOT autoplay.
- The video scrubs forward/backward based on horizontal mouse movement. Use a `mousemove` event listener on `window`. Track `prevX`, compute `delta = currentX - prevX`, convert to a time offset: `(delta / window.innerWidth)  SENSITIVITY  video.duration` where `SENSITIVITY = 0.8`. Clamp `targetTime` between 0 and `video.duration`. Use `video.currentTime` to seek, and an `onSeeked` handler to queue the next seek if `targetTime` has moved, preventing seek-flooding.

---

**NAVBAR (fixed, z-index: 10)**

- Fixed to top, full width. Padding: `px-5 sm:px-8 py-4 sm:py-5`. Flex row, `justify-between`, `items-center`.
- **Logo (left):** Flex row with `gap-3`. Text "Mainframe(R)" (use the registered trademark symbol) at `text-[21px] sm:text-[26px]`, `tracking-tight`, black, using `var(--font-heading)`. Beside it, a decorative asterisk character `✳︎` at `text-[25px] sm:text-[30px]`, black, `select-none`, `letter-spacing: -0.02em`.
- **Desktop nav links (center, hidden below md):** Flex row, `text-[23px]`, black. Links: "Labs", "Studio", "Openings", "Shop" separated by commas rendered as `, `. Each link has `hover:opacity-60 transition-opacity`.
- **Desktop CTA (right, hidden below md):** An anchor "Get in touch" at `text-[23px]`, black, `underline underline-offset-2`, `hover:opacity-60 transition-opacity`.
- **Mobile hamburger (visible below md):** A button with 3 horizontal bars (each `w-6 h-[2px] bg-black`), spaced with `gap-[5px]`. On toggle, the top bar rotates 45deg and translates down 7px, middle bar fades to opacity 0, bottom bar rotates -45deg and translates up 7px. All transitions are `duration-300`.
- **Mobile overlay (z-index: 9):** `fixed inset-0 bg-white/95 backdrop-blur-sm`, flex column, vertically centered, left-aligned with `px-8 gap-8`. Same links at `text-[32px] font-medium`, plus "Get in touch" underlined. Fades in/out with `opacity` and `pointerEvents` toggled. Hidden on md+.

---

**HERO SECTION (z-index: 1)**

- Full `h-screen`, flex column. On mobile: `justify-end pb-12`. On `md:`: `justify-center pb-0`. Horizontal padding: `px-5 sm:px-8 md:px-10`. `overflow-hidden`.
- Content container: `max-w-xl`, `relative z-10`.

**1. Blurred intro label:**
- `pointer-events-none`, `select-none`, `mb-5 sm:mb-6`.
- Font size: `clamp(18px, 4vw, 26px)`, `line-height: 1.3`, `font-weight: 400`, `color: #000`, `filter: blur(4px)`.
- Two lines of text:
  - Line 1: "Hey there, meet A.R.I.A,"
  - Line 2: "Mainframe's Adaptive Response Interface Agent"
- Separated by a `
`.

**2. Typewriter text:**
- Text: `"Glad you stopped in. Good taste tends to find us. Now, what are we building?"`
- Custom `useTypewriter` hook: takes `text`, `speed` (default 38ms per character), `startDelay` (default 600ms). After the delay, an interval reveals one character at a time. Returns `{ displayed, done }`.
- Rendered in a `

` tag, black, `mb-5 sm:mb-6`, font size `clamp(18px, 4vw, 26px)`, `line-height: 1.35`, `font-weight: 400`, `min-height: 54px`.
- While typing, show a blinking cursor: `inline-block w-[2px] h-[1.1em] bg-black align-middle ml-[2px]` with CSS animation `blink 1s step-end infinite` (`opacity: 1 at 0%/100%, 0 at 50%`). Cursor disappears when `done` is true.

**3. Action pill buttons:**
- Appear with a fade-in + slide-up animation (`opacity 0->1`, `translateY(8px)->0`, `transition: opacity 0.4s ease, transform 0.4s ease`). They become visible 400ms after page load, independent of the typewriter animation (do NOT wait for typing to finish).
- Container: `flex flex-wrap gap-y-1`.
- **4 white pill buttons:** Labels: "Pitch us an idea", "Come work here", "Send a brief hello", "See how we operate". Each is `inline-flex items-center justify-center bg-white text-black border border-black/10 rounded-full text-[13px] sm:text-[15px] px-4 sm:px-5 py-[0.3em] mx-[0.2em] mb-[0.4em] white-space: nowrap`. Hover: `bg-black text-white`, `transition-colors duration-200`.
- **1 outline pill button:** Text "Reach us: hello@mainframe.co" (email is underlined with `underline-offset-1`), followed by a small 12x12 copy icon (inline SVG of two overlapping rectangles). Styled: `text-white bg-transparent border border-white rounded-full`, same sizing as above, with `gap-2 sm:gap-3` between text and icon. Hover: `bg-white text-black`. On click, copies "hello@mainframe.co" to clipboard via `navigator.clipboard.writeText()`.

---

DEPENDENCIES

Only React, ReactDOM, Tailwind CSS, and Vite. No other UI libraries. Lucide-react is available but not used in this component.

## Reveal Hero — Hero [sites/reveal-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(26).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/reveal-hero.webp

Build a single-page React + TypeScript + Vite + Tailwind CSS project that recreates the following hero section exactly. Use `lucide-react` for icons. Do not install any other UI or animation libraries.

### Project setup

- React 18, TypeScript, Vite, Tailwind CSS.
- Dependencies: `react`, `react-dom`, `lucide-react`, `@supabase/supabase-js`.
- File `src/index.css` must be:

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  font-family: 'Inter', sans-serif;
}
```

### Assets (use these URLs verbatim, do NOT download)

- `BG_IMAGE_1` = `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260512_012043_9764f2d0-5c6e-4faa-94a6-a8253df08c5e.png&w=1280&q=85`
- `BG_IMAGE_2` = `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260512_012949_6b24738e-6e5f-4b6f-93d7-5772f4d32285.png&w=1280&q=85`

### Constants

- `SPOTLIGHT_R = 260`
- `GRID_CELL = 48`

### Behavior / animations

1. **Grid background** — inline SVG, opacity 0.1, full-cover, absolutely positioned behind content. A `<pattern id="grid">` with `width=48`, `height=48`, `patternUnits="userSpaceOnUse"`, whose `x` and `y` are bound to a ref-driven `gridOffset`. Pattern contains a single `<path d="M 48 0 L 0 0 0 48" fill="none" stroke="#64748b" strokeWidth="0.6" />`. A `<rect width="100%" height="100%" fill="url(#grid)" />` fills it.
2. **Mouse tracking** — on `mousemove` store raw `{x, y}` in a ref. A `requestAnimationFrame` loop eases a `smooth` ref toward `mouse` with factor `0.1`. Using `smooth`, compute normalized offset `cx = (smooth.x - rect.left) / rect.width - 0.5` (same for y), then ease `gridOffset` toward `cx * 16` / `cy * 16` with factor `0.06`. Update a `cursorPos` state with the smoothed coords each frame.
3. **Reveal layer (spotlight)** — a hidden `<canvas>` sized to `window.innerWidth` × `window.innerHeight`. Each frame (in a `useEffect` that runs on every render of `RevealLayer`):
   - Clear canvas.
   - Create `createRadialGradient(cursorX, cursorY, 0, cursorX, cursorY, 260)` with stops:
     - `0` → `rgba(255,255,255,1)`
     - `0.4` → `rgba(255,255,255,1)`
     - `0.6` → `rgba(255,255,255,0.75)`
     - `0.75` → `rgba(255,255,255,0.4)`
     - `0.88` → `rgba(255,255,255,0.12)`
     - `1` → `rgba(255,255,255,0)`
   - Draw a full circle `arc(cursorX, cursorY, 260, 0, 2π)` filled with that gradient.
   - Convert canvas to `toDataURL()` and apply as `maskImage` / `webkitMaskImage` on a `<div>` sized `100% 100%` whose background is `BG_IMAGE_2` (`bg-center bg-cover bg-no-repeat`). Mask size `100% 100%`.
4. Resize listener resets canvas width/height to `window.innerWidth` / `window.innerHeight`.

### Layout / JSX structure

Top-level `App`:

- Root `<div className="min-h-screen bg-white" style={{ fontFamily: 'Inter, sans-serif' }}>`.
- Fixed nav: `<nav className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-5 sm:px-8 py-4 sm:py-5">`.

**Logo** (inline SVG, first child of nav, inside `<div className="flex items-center">`):

```html
<svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 256 256" fill="none">
  <path d="M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 96 95 L 63.5 128 L 64 128 L 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 64 L 64 0 L 192 0 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z" fill="#111111" />
</svg>
```

**Desktop pill nav** (hidden on mobile): `<div className="hidden md:flex absolute left-1/2 -translate-x-1/2 bg-gray-900 rounded-full px-2 py-1.5 items-center gap-1">` containing buttons in order:
- `Device` — active: `bg-white text-gray-900 text-sm font-medium px-4 py-1.5 rounded-full`
- `Real Stories`, `Science`, `Plans`, `Reach Us` — each: `text-gray-300 text-sm font-medium px-4 py-1.5 rounded-full hover:text-white transition-colors`

**Desktop CTA** (right): `<button className="hidden md:flex bg-gray-900 text-white text-sm font-medium px-5 py-2 rounded-full items-center gap-2 hover:bg-gray-700 transition-colors">` with green dot `<span className="w-2 h-2 rounded-full bg-green-400 inline-block"></span>` + text `Reserve Yours`.

**Mobile hamburger** (shown `md:hidden`): toggles `menuOpen` state; icon is lucide `Menu` (size 22) or `X` (size 22) when open; button classes `md:hidden text-gray-900 p-1`.

**Mobile dropdown** (when `menuOpen`): `fixed top-0 left-0 right-0 z-40 bg-white pt-16 pb-6 px-5 shadow-lg flex flex-col gap-1 md:hidden`, maps items `['Device','Real Stories','Science','Plans','Reach Us']` to `<button className="text-gray-800 text-base font-medium py-3 border-b border-gray-100 text-left hover:text-gray-500 transition-colors">`, then a `Reserve Yours` button: `mt-4 bg-gray-900 text-white text-sm font-medium px-5 py-3 rounded-full flex items-center gap-2 justify-center hover:bg-gray-700 transition-colors` with the green dot span.

**Hero section** (`<section>`): `relative w-full overflow-hidden` with inline `style={{ height: '100vh' }}`. Children in order:

1. The grid SVG described above (`opacity: 0.1`, `z-0`, pointer-events none, `absolute inset-0 w-full h-full`).
2. Base image div: `absolute inset-0 bg-center bg-cover bg-no-repeat z-10` with `backgroundImage: url('<BG_IMAGE_1>')`.
3. `<RevealLayer>` (hidden canvas + masked div with `BG_IMAGE_2`, `z-30`, pointer-events none).
4. Hero text block: `<div className="absolute bottom-12 sm:bottom-12 md:bottom-56 left-5 sm:left-8 md:left-12 max-w-[260px] sm:max-w-xs z-50">` containing:
   - `<p className="text-[10px] sm:text-[11px] font-semibold tracking-[0.18em] text-gray-600 uppercase mb-2 sm:mb-3">PureFlow One</p>`
   - `<h1 className="text-2xl sm:text-3xl md:text-4xl font-bold text-gray-900 leading-tight mb-4 sm:mb-6">Clean Air, Clear<br />Mind. Anywhere.</h1>`
   - A `<div className="flex items-center gap-3 sm:gap-4">` with:
     - `<button className="bg-gray-900 text-white text-xs sm:text-sm font-medium px-4 sm:px-6 py-2 sm:py-2.5 rounded-full hover:bg-gray-700 transition-colors">Discover</button>`
     - `<button className="flex items-center gap-2 text-gray-700 text-xs sm:text-sm font-medium hover:text-gray-900 transition-colors">` containing lucide `<Play size={12} className="fill-gray-700" />` and text `View Specs`.

### Notes

- All icons from `lucide-react`: `Play`, `Menu`, `X`.
- Font: Inter (Google Fonts) weights 300–700.
- No purple/indigo colors; neutrals + `bg-green-400` status dot only.
- Responsive: tablet (`sm`) keeps hero text at the same bottom as mobile (`bottom-12`); only desktop (`md`+) raises it (`bottom-56`).
- Use `useRef` + `requestAnimationFrame` (no external animation libs). Canvas-based radial mask reveal must update each frame.

## Solar Energy Hero — Hero [sites/solar-energy-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(35).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/solar-energy-hero.webp

Build a single-page React + TypeScript + Vite hero section for a solar energy brand called "reposit." The page features a fullscreen background image that transitions between a daytime (Morning) photo and a nighttime (Night) photo using a custom pull-down animation. The entire page uses vanilla CSS (no CSS modules) with Tailwind installed but only used minimally (the design is almost entirely custom CSS). Google Font "Outfit" is loaded. The icon library is lucide-react (only the Zap icon is used).

---

TECH STACK AND CONFIG:

- Vite 5.4.2 with @vitejs/plugin-react, React 18.3.1, TypeScript 5.5.3
- Tailwind CSS 3.4.1 via PostCSS + Autoprefixer
- lucide-react 0.344.0
- @supabase/supabase-js 2.57.4 (installed but unused in this page)
- vite.config.ts: optimizeDeps.exclude includes 'lucide-react'
- tailwind.config.js: content array is ['./index.html', './src/**/*.{js,ts,jsx,tsx}'], no theme extensions, no plugins
- postcss.config.js: plugins are tailwindcss and autoprefixer

---

INDEX.HTML (verbatim):

```
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Reposit Zero Electricity Bills Page</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

---

IMAGES:

Two images stored locally in /public/images/:
- `/images/hero-light.webp` — the daytime/morning photo. Source URL: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/hf_20260515_092045_b654224c-4741-458f-8596-fa5bfeffabbc_1_oyfhme.jpg
- `/images/hero-dark.webp` — the nighttime photo. Source URL: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/hf_20260515_092102_24e30358-d694-4b70-8a56-a4f0887cf8ae_1_ry5dvs.jpg

Download both at build time so they serve locally (no external fetching at runtime).

---

MAIN.TSX (verbatim):

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App.tsx';
import './index.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
```

---

APP.TSX (verbatim):

Single component, no router, no external state. Uses useState, useEffect, useRef from React. Imports only `{ Zap }` from lucide-react.

Constants:
- `LIGHT_IMG = '/images/hero-light.webp'`
- `DARK_IMG = '/images/hero-dark.webp'`

State:
- `isDark` (boolean, default `true`) — controls theme
- `menuOpen` (boolean, default `false`) — mobile drawer

Refs:
- `bgFrontRef` (HTMLDivElement) — the foreground background layer
- `bgBackRef` (HTMLDivElement) — the blurred background layer behind it
- `animatingRef` (boolean) — prevents double-clicks during transition

Effects:
1. When `isDark` changes: add/remove class `light-theme` on `document.body`
2. On mount: set both bgFrontRef and bgBackRef backgroundImage to `url(${DARK_IMG})`

Toggle logic (`toggleTheme(toDark: boolean)`):
1. If already in target state or animating, return early
2. Set animatingRef true
3. Set bgBack's backgroundImage to the target image
4. Add class `pull-down` to bgFront (triggers the pull-down CSS animation)
5. After 300ms timeout: set isDark state, set bgFront's backgroundImage to target image
6. After another 30ms timeout: remove `pull-down` class, set animatingRef false

JSX structure (exact nesting):
```
div.hero
  div.blur-overlay.blur-overlay-top
  div.blur-overlay.blur-overlay-bottom
  div.hero-bg-wrapper
    div[ref=bgBackRef].hero-bg.bg-back
    div[ref=bgFrontRef].hero-bg.bg-front
  nav.navbar
    div.logo-container
      <Zap className="logo" size={32} strokeWidth={2} />
      span.brand-name "reposit"
    div.nav-links (add class "active" when menuOpen)
      a[href="#"] "How It Works"
      a[href="#"] "Our Cases"
      a[href="#"] "About Us"
      a[href="#"] "Careers"
      a[href="#"] "Resources"
      a[href="#"] "Customers"
      button.cta-button.drawer-cta "Get an Instant Quote"
    button.cta-button.nav-cta "Get an Instant Quote"
    div.hamburger (add class "active" when menuOpen, onClick toggles menuOpen)
      span
      span
      span
  div.hero-content
    h1.hero-title
      "$0 Electricity Bills"
      <br/>
      span.title-accent "for the next"
      " 7 years"
    div.theme-toggle
      div.toggle-indicator [inline style: transform is 'translateX(calc(100% + 4px))' when isDark, 'translateX(0)' when light]
      button.toggle-btn (add class "active" when !isDark), onClick => toggleTheme(false)
        span.label "Morning"
        span.subtext "$0 for Electricity"
      button.toggle-btn (add class "active" when isDark), onClick => toggleTheme(true)
        span.label "Night"
        span.subtext "$0 for Electricity"
    p.hero-footer
      "Forget the energy market, weather conditions and seasons; our Smart Controller guarantees you get no electricity bill for seven years."
```

---

INDEX.CSS (verbatim, every rule):

CSS Custom Properties on :root:
- `--bg-light: #ffffff`
- `--bg-dark: #000000`
- `--text-light: #3E3424`
- `--text-dark: #E5DEC9`
- `--active-toggle: #f5f8ea`
- `--transition-speed: 0.9s`
- `--pull-easing: cubic-bezier(0.32, 0, 0.67, 0)`
- `--return-easing: cubic-bezier(0.175, 0.885, 0.32, 1.4)`

Universal reset: `* { margin:0; padding:0; box-sizing:border-box; font-family:'Outfit',sans-serif; }`

body:
- background-color: var(--bg-dark), color: var(--text-dark), overflow:hidden, transition: background-color 0.5s ease

body.light-theme:
- background-color: var(--bg-light), color: var(--text-light)

.blur-overlay:
- position:absolute, left:0, width:100%, height:10vh, z-index:2, pointer-events:none
- backdrop-filter: blur(25px) saturate(1.5), -webkit-backdrop-filter: blur(25px) saturate(1.5)

.blur-overlay-top:
- top:0
- mask-image: linear-gradient(to bottom, black 70%, transparent 100%)
- -webkit-mask-image: same

.blur-overlay-bottom:
- bottom:0
- mask-image: linear-gradient(to top, black 70%, transparent 100%)
- -webkit-mask-image: same

.hero:
- position:relative, width:100%, height:100vh, display:flex, flex-direction:column, align-items:center, justify-content:space-between, overflow:hidden
- background-image: radial-gradient(circle at center, rgba(255,255,255,0.05) 0%, transparent 100%)

body.light-theme .hero:
- background-image: radial-gradient(circle at center, rgba(0,0,0,0.02) 0%, transparent 100%)

.hero-bg-wrapper:
- position:absolute, top:0, left:0, width:100%, height:100%, z-index:1, overflow:hidden

.hero-bg:
- position:absolute, top:0, left:0, width:100%, height:100%
- background-size:cover, background-position: center 40%, background-repeat:no-repeat
- transform: scale(1.1)

.bg-front:
- z-index:2
- transition: transform 0.5s var(--return-easing), opacity 0.5s ease

.bg-back:
- z-index:1, filter: blur(40px), transform: scale(1.2)

.hero-bg::after (pseudo-element overlay):
- content:'', position:absolute, top:0, left:0, width:100%, height:100%, pointer-events:none
- background: radial-gradient(circle at center, transparent 0%, rgba(0,0,0,0.4) 100%), linear-gradient(to bottom, rgba(0,0,0,0.3) 0%, transparent 30%, transparent 70%, rgba(0,0,0,0.8) 100%)

body.light-theme .hero-bg::after:
- background: radial-gradient(circle at center, transparent 0%, rgba(255,255,255,0.2) 100%), linear-gradient(to bottom, rgba(255,255,255,0.3) 0%, transparent 30%, transparent 70%, rgba(255,255,255,0.8) 100%)

.navbar:
- width:100%, max-width:100%, padding:24px 30px, display:flex, justify-content:space-between, align-items:center, z-index:110

.hamburger:
- display:none, flex-direction:column, gap:6px, cursor:pointer, z-index:120

.hamburger span:
- display:block, width:28px, height:2px, background:currentColor, border-radius:2px, transition:0.3s

.hamburger.active span:nth-child(1): transform: translateY(8px) rotate(45deg)
.hamburger.active span:nth-child(2): opacity:0
.hamburger.active span:nth-child(3): transform: translateY(-8px) rotate(-45deg)

.logo-container: display:flex, align-items:center, gap:12px

.logo: height:32px, color:#ffffff, transition: color 0.5s ease
body.light-theme .logo: color:#000000

.brand-name: font-size:24px, font-weight:400, letter-spacing:-0.5px, color:#ffffff, transition: color 0.5s ease
body.light-theme .brand-name: color:#000000

.nav-links: display:flex, gap:32px
.nav-links a: color:inherit, text-decoration:none, font-size:14px, font-weight:500, opacity:0.7, transition: opacity 0.3s
.nav-links a:hover: opacity:1

.cta-button: background:#ffffff, color:#000000, border:none, padding:12px 24px, border-radius:8px, font-weight:600, font-size:14px, cursor:pointer, transition: transform 0.3s, background 0.3s
.drawer-cta: display:none
body.light-theme .cta-button: background:#000000, color:#ffffff
.cta-button:hover: transform: translateY(-2px), box-shadow: 0 10px 20px rgba(0,0,0,0.1)

.hero-content: flex-grow:1, display:flex, flex-direction:column, align-items:center, justify-content:flex-start, text-align:center, padding:30px 20px 0, z-index:5

.hero-title: font-size:56px, font-weight:500, line-height:1.0, max-width:1000px, margin-bottom:40px, letter-spacing:-1px, color:var(--text-dark), opacity:0.95

.title-accent: transition: color 0.5s ease
body:not(.light-theme) .title-accent: color:#10100F
body.light-theme .title-accent: color:white
body.light-theme .hero-title: color:var(--text-light), opacity:0.95

.theme-toggle: background: rgba(210,198,171,0.15), backdrop-filter: blur(20px), border:none, padding:2px 1px, border-radius:8px, display:flex, gap:4px, margin-top:auto, margin-bottom:8px, position:relative
body.light-theme .theme-toggle: background: rgba(210,198,171,0.25), border:none

.toggle-btn: padding:6px 40px, border-radius:4px, border:none, background:transparent, color:#ffffff, cursor:pointer, z-index:1, transition: color 0.3s, display:flex, flex-direction:column, align-items:center, gap:4px
.toggle-btn .label: font-weight:500, font-size:18px
.toggle-btn .subtext: font-size:11px, opacity:0.6

.toggle-indicator: position:absolute, top:2px, left:1px, width:calc(50% - 3px), height:calc(100% - 4px), background:var(--active-toggle), border-radius:4px, transition: transform 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), z-index:0, box-shadow: 0 4px 12px rgba(0,0,0,0.1)
body:not(.light-theme) .toggle-indicator: transform: translateX(calc(100% + 4px))

.toggle-btn.active: color:#3E3424 !important
.toggle-btn.active .subtext: opacity:0.8

.hero-footer: max-width:600px, margin-bottom:60px, margin-top:0, color:var(--text-dark), opacity:1, font-size:16px, font-weight:300, line-height:1.6, z-index:5
body.light-theme .hero-footer: color:var(--text-light)

.pull-down: transform: translateY(20vh) scale(1.1) !important, opacity:0.8 !important, transition: transform 0.3s var(--pull-easing), opacity 0.3s ease !important

@keyframes fadeIn: from { opacity:0; transform:translateY(20px) } to { opacity:1; transform:translateY(0) }
.hero-content > *: animation: fadeIn 1s ease forwards
.hero-title: animation-delay:0.2s
.theme-toggle: animation-delay:0.4s
.hero-footer: animation-delay:0.6s

MOBILE BREAKPOINT (@media max-width:768px):
- .hero-title: font-size:42px, margin-bottom:30px
- .navbar: padding:16px 20px
- .hero-bg: background-position: center 40%, transform: scale(1.2)
- .pull-down: transform: translateY(20vh) scale(1.2) !important
- .nav-links: display:none, position:fixed, top:0, right:0, width:100%, height:100vh, background:var(--bg-dark), flex-direction:column, justify-content:center, align-items:center, z-index:100, gap:40px, transition: transform 0.4s cubic-bezier(0.77,0,0.175,1), transform:translateX(100%)
- body.light-theme .nav-links: background:var(--bg-light)
- .nav-links.active: display:flex, transform:translateX(0)
- .nav-links a: font-size:24px, font-weight:600
- .cta-button.nav-cta: display:none
- .drawer-cta: display:block, width:200px, margin-top:20px, padding:16px
- .hamburger: display:flex !important
- .theme-toggle: flex-direction:row, width:calc(100% - 40px), max-width:400px
- .toggle-btn: padding:12px 20px, flex:1

---

ANIMATION AND TRANSITION SUMMARY:

1. Page load fadeIn: each hero-content child fades in with `animation: fadeIn 1s ease forwards`. Staggered delays: title 0.2s, toggle 0.4s, footer 0.6s. Keyframes go from opacity:0 + translateY(20px) to opacity:1 + translateY(0).

2. Theme toggle pull-down: When switching themes, the front background div gets class `pull-down` which applies `transform: translateY(20vh) scale(1.1)` with `transition: transform 0.3s cubic-bezier(0.32, 0, 0.67, 0)` and `opacity: 0.8`. After 300ms, the image source swaps and pull-down is removed. The return uses the bg-front's own transition: `transform 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.4)` (overshoot/bounce easing).

3. Toggle indicator slide: `transition: transform 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275)` — slides left/right between the two buttons with a slight overshoot.

4. Body background color: `transition: background-color 0.5s ease`

5. Logo and brand name color: `transition: color 0.5s ease`

6. CTA button hover: `transform: translateY(-2px)` with `transition: transform 0.3s`

7. Nav links opacity hover: `transition: opacity 0.3s`

8. Mobile nav drawer: `transition: transform 0.4s cubic-bezier(0.77, 0, 0.175, 1)` from translateX(100%) to translateX(0)

9. Hamburger spans: `transition: 0.3s` for the X animation

## SpeakUp Venture Hero — Hero [sites/speakup-venture-hero]

- Preview: https://stream.mux.com/a6HPc2D4wiAmo1102yX8WVFt01gzGYkOG1vPVysYykdao.m3u8
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/speakup-venture-hero.m3u8

Create a full-screen hero section (100vh, min-height 600px) for a creative agency site called **Speakup**. Build it in React + TypeScript with Tailwind CSS and use `lucide-react` for icons.

**Fonts**
- Load `Recoleta Regular` from: `<link href="https://db.onlinewebfonts.com/c/67415ab41a7350f81536b69763e6d031?family=Recoleta+Regular" rel="stylesheet">`
- Load `Inter` (weights 400, 500, 600, 700) from Google Fonts.
- Use `Recoleta Regular` only for the H1 heading. Use `Inter` for all other text (body, nav, buttons, logo wordmark).
- Extend Tailwind with `fontFamily: { recoleta: ['"Recoleta Regular"', 'serif'], inter: ['Inter', 'sans-serif'] }`.
- In `index.css`, set `html, body` to `font-family: 'Inter', sans-serif` with antialiased smoothing, and add a `.font-recoleta` utility.

**Colors (extend Tailwind)**
- `brand.green = #0E7824` (heading only)
- `brand.dark = #2D2D2F` (logo, nav text, buttons)

**Background video (no overlay, full cover)**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_150921_27df94bd-d1e3-4440-9f55-314c4611902b.mp4`
- Attributes: `autoPlay muted loop playsInline`
- Positioned `absolute inset-0 w-full h-full object-cover`.

**Logo component** (`Logo.tsx`) — accepts `className` and `color` props; renders this SVG with `fill={color}`:
```
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="none">
  <path d="M 128.005 191.173 C 128.448 156.208 156.93 128 192 128 L 192 64 L 128 64 C 128 99.346 99.346 128 64 128 L 64 192 L 128 192 Z M 192 256 L 64 256 C 28.654 256 0 227.346 0 192 L 0 64 L 64 64 L 64 0 L 192 0 C 227.346 0 256 28.654 256 64 L 256 192 L 192 192 Z" />
</svg>
```
Default color `#2D2D2F`.

**Navbar** (z-20, padding `px-6 md:px-12 lg:px-16 pt-6 md:pt-8`, flex between):
- Left: Logo (`w-8 h-8 md:w-9 md:h-9`) + wordmark `SPEAK` (bold) + `UP` (black weight), color `#2D2D2F`, tracking-tight, `text-lg md:text-xl`.
- Center (hidden below lg, gap-8 xl:gap-10): links — `Projects`, `The Team`, `Products`, `Our Story`, `Say Hello!` — text-sm, font-medium, `#2D2D2F`, hover turns `#0E7824` via `transition-colors`.
- Right (hidden below lg): pill button "Begin a venture", `rounded-full bg-[#2D2D2F] text-white text-sm font-medium px-7 py-3.5 shadow-sm hover:bg-black transition-colors`.
- Mobile: hamburger (`Menu` icon from lucide) toggles a full-screen white overlay (`fixed inset-0 z-50`) with `X` close button, stacked links `text-2xl`, and the same pill CTA.

**Hero content** (z-10, `px-6 md:px-12 lg:px-16 mt-8 md:mt-16 lg:mt-24 max-w-7xl`):
- H1 using `font-recoleta` color `#0E7824`, `leading-[1.05] tracking-tight`, sizes `text-[56px] sm:text-7xl md:text-8xl lg:text-[128px]`, content: `Crafting the` / line break / `improbable`.
- Paragraph: `mt-6 md:mt-8 text-[#2D2D2F] font-inter text-base md:text-lg max-w-md leading-[1.5]`, text: `We bring your boldest digital visions to reality.` / `<br class="hidden sm:block">` / `Because it cannot be done is where we all begin now`.
- CTA below (`mt-8 md:mt-10`): pill `rounded-full bg-[#2D2D2F] text-white text-base font-medium px-10 py-4 shadow-md hover:bg-black transition-colors`, label `Begin a venture`.

**Animations / interactions**
- Color transitions on all links and buttons via `transition-colors`.
- Hamburger state via `useState`; no overlay/tint on the video.

**Responsiveness**
- Mobile: smaller heading (56px), stacked layout, hamburger menu.
- Tablet (md): larger heading, increased spacing.
- Desktop (lg+): nav + right CTA visible, heading 128px.

**File layout**
- `index.html` with font links and `<title>Speakup</title>`.
- `tailwind.config.js` with the font and color extensions.
- `src/index.css` with Tailwind directives + body font + `.font-recoleta`.
- `src/components/Logo.tsx`, `src/components/Hero.tsx`, and `src/App.tsx` rendering `<Hero />` inside `<main>`.

## Stillmind — Hero [sites/stillmind]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/endless.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/stillmind.mp4

Create a fullscreen cinematic hero section for a mindfulness/focus app called "Lumora" using React, Tailwind CSS, and Lucide React icons.

### Font

Use **Instrument Serif** (Google Fonts, italic for the logo). Load it in index.html:
```
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet">
```

Set `font-family: 'Instrument Serif', serif` on html/body. Use `system-ui, sans-serif` inline for body text (subtext, buttons, stats, video labels).

---

### Background Video Layer

Stack 4 fullscreen looping videos absolutely positioned. Only the active one has `opacity-100`; others have `opacity-0`. Transition opacity over 1000ms ease-in-out. Videos autoPlay, muted, loop, playsInline.

**Video URLs (in order):**
1. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_081127_0992a171-d3c6-4978-8213-0ec5df8b6d63.mp4`
2. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_092026_dd05b805-ea0f-40b2-8c52-332b88502592.mp4`
3. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_081042_df7202bf-bd80-4b2b-bbc6-1f09ba2870e9.mp4`
4. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_080959_4cac5234-3573-464e-a5b7-76b94b8a7d61.mp4`

**Labels:** Golden Hour, Still Water, Deep Woods, Quiet Dawn

---

### Transparent PNG Overlay (z-index 1)

Place this image over the videos as an absolutely positioned overlay covering the full viewport:
```
https://soft-zoom-63098134.figma.site/_assets/v11/0b4a435b2df2747593c43d7a1c9b4578f7d8d90c.png
```

Apply a continuous "train-bob" animation: translateY oscillates between 0 and -6px over 3s ease-in-out infinite, with a constant scale(1.03) to prevent edges from showing during the motion.

---

### Liquid Glass Effect (CSS class `.liquid-glass`)

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

With a `::before` pseudo-element for a subtle gradient border:
```css
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

### Content Layer (z-index 2) - Flex Column Full Height

### Navigation (top)
- Left: "Lumora" in white, italic, text-xl (sm:text-2xl)
- Right (desktop md+): A `.liquid-glass` pill containing nav links ("How It Works", "Features", "Pricing", "Community") in white/90 text-sm with hover to white, plus a solid white "Get Started" button at the end
- Right (mobile): A `.liquid-glass` rounded hamburger button using Lucide `Menu`/`X` icons with a crossfade rotation animation (300ms). The Menu icon rotates out 90deg and scales to 75%; the X icon rotates in from -90deg

### Mobile Menu Overlay (fixed, z-50)
- Backdrop: `bg-black/60 backdrop-blur-sm`
- Centered fullscreen panel with staggered entrance (each link delays 50ms more: 100ms, 150ms, 200ms, 250ms, 300ms)
- Links: white text-3xl, translate-y-4 to 0 on open
- "Get Started" button at bottom with scale animation
- Cubic-bezier easing: `cubic-bezier(0.4,0,0.2,1)`, duration 500ms

### Hero Content (centered, below nav)
- **Badge**: `.liquid-glass` rounded-full pill with text "Over 10,000 minds already finding their clarity"
- **Heading**: "Clarity in an Endlessly / Noisy Universe" (line break after "Endlessly"). Sizes: text-4xl / sm:text-5xl / md:text-7xl / lg:text-[5.5rem], leading-[1.1], max-w-4xl
- **Subtext**: "Rise above the chaos of pings, infinite scrolling, and relentless demands. Discover how to protect your presence and create with intention." max-w-xl, leading-relaxed
- **Email Input**: `.liquid-glass` rounded-full pill containing a text input ("Your Best Email") and a solid white "Get Early Access" button. Max-width 320px on mobile, sm:max-w-sm
- **Video Switcher**: Row of 4 text buttons with labels. Active button has solid color + bottom border. Inactive buttons are 50% opacity with transparent border, hover to 80%

### Dark Mode for "Deep Woods" (3rd video, index 2)
When the 3rd video is active, all hero content (badge, heading, subtext, input, video switcher) transitions to dark color `#182C41` with 700ms duration. The navbar and bottom stats remain white always.

### Bottom Stats (pushed to bottom via flex-1 spacer)
- Row of stats separated by `|` dividers (hidden on mobile): "60+ Deep Sessions", "12,000+ Creators", "4.8 User Satisfaction", "Intentional-First Design"
- text-white/70, text-xs sm:text-sm, system-ui font

---

### Video Switching Logic
- Track `activeVideo` state (default 0) and `isTransitioning` boolean
- On click, if not already active and not mid-transition, set new active video and start a 1000ms cooldown (matching the CSS crossfade duration)
- During cooldown, ignore additional clicks

---

### Responsive Behavior
- Mobile: Smaller text sizes, tighter padding, hamburger nav, stats wrap naturally
- Tablet/Desktop: Larger heading, more padding, inline nav pill, stats with pipe separators

---

### Section Container
```html
<section className="relative w-full h-screen overflow-hidden bg-black">
```

Black background prevents flash before videos load. Everything is a single viewport-height section with no scroll.

---

That's the complete specification. The entire app lives in a single `App.tsx` component with the CSS in `index.css`.

## Subscription Agency — Hero [sites/subscription-agency]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/agencygradientArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/subscription-agency.mp4

**Create a single-page landing hero section for a creative agency called "Alwayzz" with a React + Vite + Tailwind CSS setup. Use custom CSS (not Tailwind utilities) for all styling. The design should be minimal, clean, black-and-white, with tight negative letter-spacing throughout.**

---

### Fonts (loaded via Google Fonts in index.html)

```
Inter: weights 400, 500, 600, 700
Source Serif 4: weights 400, 600 (both normal and italic)
```

Preconnect to `fonts.googleapis.com` and `fonts.gstatic.com`.

---

### CSS Variables

```css
--bg: #ffffff;
--text: #0a0a0a;
--muted: #6b6b6b;
--button-bg: #0a0a0a;
--button-text: #ffffff;
--border-soft: rgba(0, 0, 0, 0.08);
--green: #17c964;
```

---

### Components

**1. Navbar (fixed top, z-index 100)**
- Padding: `19px 36px`, max-width `1200px` centered.
- Left: Logo text "Alwayzz" in `Source Serif 4`, 30px, weight 600, **italic**, letter-spacing `-0.08em`, with a registered trademark symbol in Inter 14px weight 600.
- Right: "Menu" pill button (black bg, white text, rounded-full, 14px weight 500, Inter) with a `ChevronUp` icon (16px) from lucide-react.
- Full-screen drawer overlay on click: white bg, fade transition 0.4s. Nav links centered vertically at 48px weight 500, letter-spacing `-0.04em`. Links: Projects, Plans, Team, FAQs, Get in Touch. Footer with copyright.

**2. Hero Section**
- Min-height: `850px`, padding: `160px 36px`, centered flex column.
- **Background image** (via `::before` pseudo-element, covers full section):
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260626_041422_4a459e05-abce-4150-9fb7-4ededc423cd1.png&w=1280&q=85
  ```
  Background-size: cover, background-position: center.

- **Curved line animations** (decorative):
  - 20 lines on left side, 20 on right side, absolutely positioned.
  - Each line is a tall rectangle with one-sided border-radius (80%) and `2.5px solid #FCFAF8` border.
  - Left lines: no left border, radius on right. Right lines: no right border, radius on left.
  - Staggered `animationDelay: i * 0.25s`, widths from 60px increasing by 10px per line.
  - Animation: `line-pulse` 5s ease-in-out infinite (fade in to 0.9 opacity, then back to 0 with slight scale).
  - On mobile (<810px): hide side lines, show top horizontal lines instead (same animation, horizontal orientation with bottom border-radius).

- **Ticker row** (max-width 500px, height 36px):
  - Horizontal marquee scrolling left over 30s, linear, infinite.
  - Items: "Brand Identity", "App Development", "Visual Design", "Creative Video", "Iconography"
  - Each item: 13px, weight 500, color `var(--muted)`, padding `6px 14px`, rounded-full, background `rgb(251, 251, 251)`.
  - Marquee has edge fade mask: `linear-gradient(90deg, transparent 0%, black 12%, black 88%, transparent 100%)`.
  - 4x duplicated rows for seamless loop.

- **Title**:
  ```
  Premium creative <span class="serif italic">alwayzz</span><sup style registered mark> on demand.
  ```
  - Max-width: 550px, font-size: 82px, line-height: 1.03, letter-spacing: `-0.07em`, weight 600, centered.
  - The word "alwayzz" uses `Source Serif 4`, italic, weight 600, letter-spacing `-0.08em`.
  - The registered mark: Inter, 24px, weight 600, vertical-align super.

- **Subtitle**:
  ```
  A flexible design partnership for founders, brands, and agencies who want top craft delivered on their timeline.
  ```
  - Max-width: 476px, 17px, line-height 1.45, weight 400, color `var(--muted)`, centered.

- **CTA row** (flex, gap 16px, centered):
  - **Primary button** "View Plans": height 56px, padding `18px 30px`, rounded-full, black bg, white text, 15px weight 600 Inter. Hover: translateY(-1px) + box-shadow `0 4px 20px rgba(0,0,0,0.12)`.
  - **Book button** "Chat for 15 minutes": white bg, `4px solid rgb(248,248,248)` border, rounded-full, padding `8px 24px 8px 8px`. Contains:
    - Avatar image (40px circle): `https://framerusercontent.com/images/hfneFL6CHBi5BnNvCeOaqU9HqE4.png`
    - Text stack: primary "Chat for 15 minutes" (14px, weight 600, black) and secondary "Pick a slot" (12px, weight 500, `rgb(152,152,152)`) with a green dot (`rgb(29, 204, 93)`, 8px circle).

- **Progressive blur** at bottom: absolute, full width, height 178px, gradient from transparent to `rgba(255,255,255,0.4)` at 40% to solid white.

**3. TrustedBy Section**
- Padding: 36px, max-width 1200px centered.
- Left: label "Partnered with top-tier companies globally" (max-width 163px, 14px, weight 500, muted color).
- Right: horizontal marquee (30s) of company names styled as text logos (16px, weight 600, black, each with distinct font-family):
  - Airbnb (Cedarville Cursive, 700), Shopify (system-ui, 800), Notion (Georgia, 500), Linear (Inter, 600), Webflow (Inter, 700), Figma (system-ui, 600), Slack (Georgia, 700), Stripe (system-ui, 800), Vercel (Inter, 600), Framer (Source Serif 4, 600).
- Same edge-fade marquee mask as ticker.

---

### Responsive Breakpoints

- **< 1200px**: Hero padding `140px 32px`, title clamp(60px, 8vw, 72px), navbar padding 32px, drawer links 40px.
- **< 810px**: Hero min-height 760px, padding `120px 24px 96px`. Background image rotated 90deg to fill portrait viewport. Side curved lines hidden, top horizontal lines shown. Title clamp(44px, 13vw, 52px). CTA buttons stack vertically full-width (max 320px). Trusted section stacks vertically. Drawer links 32px. Navbar padding 20px.

---

### Key Animation Keyframes

```css
@keyframes marquee-left {
  from { transform: translateX(0); }
  to { transform: translateX(-50%); }
}

@keyframes line-pulse {
  0% { opacity: 0; transform: scale(1); }
  15% { opacity: 0.9; }
  70% { opacity: 0.4; }
  100% { opacity: 0; transform: scale(0.85); }
}
```

---

### CloudFront Video/Image URL

The hero background image URL (exact):
```
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260626_041422_4a459e05-abce-4150-9fb7-4ededc423cd1.png&w=1280&q=85
```

The book button avatar URL (exact):
```
https://framerusercontent.com/images/hfneFL6CHBi5BnNvCeOaqU9HqE4.png
```
]

## Tech-Forward — Hero [sites/tech-forward]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/132Area.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/tech-forward.mp4

Create a full-screen hero section landing page using React, Vite, and Framer Motion (`motion` package). Use plain CSS (no Tailwind). The font is Inter (weights 300, 400, 500, 600) from Google Fonts. The design is minimal black-and-white with a full-viewport background video.

**Stack:** React 19, Vite, `motion` (framer-motion), `lucide-react` (for the Plus icon).

**Layout:**
- Full viewport height (`min-height: 100vh`), white background, flex column with `justify-content: space-between`
- Fixed navbar at top (z-index 50, pointer-events none on the nav itself, auto on children)
- Absolutely positioned full-screen video behind everything (z-index 0)
- Footer content pinned to bottom (z-index 30) with a white gradient fade-up background

**Navbar (fixed, top):**
- Left side contains:
  1. Logo: custom SVG icon (two rotated rounded rectangles at -35deg, black fill) + brand text "NeuralKinetics" (hidden on mobile, shown on desktop 768px+)
  2. Menu button: black pill with white circle containing a Plus icon (lucide, size 12, strokeWidth 3) + "Menu" text (11px, white)
  3. Tags pill: light gray (#F4F4F6) rounded-full container with two text labels "Advanced Bionics" and "Cognitive AI" (hidden on mobile, shown on desktop)
- Right side contains:
  - A light gray pill with a black circle button (containing a 4-dot grid SVG icon) + label "Adaptive Systems" (hidden on mobile)

**Background Video:**
- URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_215831_c6a8989c-d716-4d8d-8745-e972a2eec711.mp4`
- autoPlay, muted, playsInline, object-fit: cover
- On mobile: video wrapper is 80% width and 80% height (centered)
- On desktop (768px+): video wrapper is 100% width and 100% height

**Footer content (bottom, over gradient):**
- Background: `linear-gradient(to top, #ffffff 0%, rgba(255,255,255,0.8) 50%, transparent 100%)`
- On mobile: stacks vertically. On desktop: row layout, items aligned to bottom.
- Left block:
  1. Subtitle line: small black dot (8px circle) + "Best digital banking card 2026" (13px, 55% opacity black)
  2. Heading: "One Card, Zero / Limits. Worldwide." on two lines. Font-weight 300, clamp(2rem, 8vw, 4.5rem) on mobile, clamp(2.5rem, 5.5vw, 4.5rem) on desktop, letter-spacing -0.03em, line-height 1
  3. Two buttons: "See Features" (black pill, white text, 13px) and "How It Works" (transparent with dark border rgba(0,0,0,0.35), 13px)
- Right block: Three tag pills "Neuromorphic", "AGI", "Cybernetics" (white bg, light border rgba(0,0,0,0.12), 11px, rounded-full)

**Animations (using `motion` from 'motion/react'):**
- Navbar: slides down from y:-16, opacity 0 to visible. Duration 0.8s, ease [0.16, 1, 0.3, 1]
- Video: fades in from opacity 0 + scale 1.05 to opacity 1 + scale 1. Duration 1.8s
- Footer wrapper: slides up from y:20, delay 0.5s, duration 1s
- Subtitle: slides up from y:16, delay 0.6s, duration 0.8s
- Heading: slides up from y:20, delay 0.8s, duration 0.8s
- Buttons: slides up from y:16, delay 1.0s, duration 0.8s
- All use ease: [0.16, 1, 0.3, 1]

**Responsive (mobile-first, breakpoint at 768px):**
- Mobile: navbar padding 16px, smaller buttons (28px circles), brand text hidden, tags hidden, right label hidden, footer stacks vertically, video at 80% size
- Desktop (768px+): navbar padding 24px 32px, larger buttons (32px circles), all text/tags visible, footer is row layout, video fills 100%

## Unwind Hero — Hero [sites/unwind-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(37).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/unwind-hero.webp

Recreate the WoodNest hero section exactly as a React + Tailwind CSS + Framer Motion component.

Use font family "PP Mori" for the entire hero. Define @font-face for PP Mori normal weight 400 and semibold weight 600 using the existing embedded WOFF/WOFF2 data from src/styles/fonts.css. Set base html font size to 16px.

The page is a full viewport hero:
- Root wrapper: relative, min-height: 100vh, width: 100%, overflow hidden, font PP Mori, font weight 400.
- Background: absolute full-screen video, inset 0, width/height 100%, object-cover.
- Video attributes: autoPlay, muted, playsInline. Do not loop unless explicitly requested.
- Exact video URL:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260620_140846_aef8cb19-5ec8-4b45-974b-020aed20f297.mp4

Header:
- Position relative, z-index 20.
- Display flex, align center, justify-between.
- Desktop padding: top 60px, left/right 65px.
- Tablet/mobile padding: px 32px / 16px, top 45px / 30px.
- Left: exact WoodNest SVG logo, 142px wide, 50px tall, white wordmark with warm yellow/orange mark (#FFB33F) and 0.6 opacity gradient details.
- Center/right desktop nav hidden below md.
- Nav items: Locations, Rooms, Experiences, Contact.
- Nav gap desktop 44px, text white, PP Mori normal, 16px, line-height 24px.
- Nav hover animation: opacity to 0.6, tap scale 0.96, duration 0.15.
- Book Now button: white background, black text, PP Mori semibold 16px/24px, padding 12px 32px, border-radius 10px.
- Button hover: scale 1.04 and background #f0f0f0. Tap scale 0.97. Transition duration 0.18 easeOut.
- Mobile: show Menu/X icon button in white, 24px. Open menu is absolute below header, left/right 0, margin x 16px, margin-top 8px, background #2a3349, border-radius 16px, padding 24px, flex column gap 16px, shadow-xl, z-index 50.

Hero main:
- Position relative, z-index 10.
- Padding desktop: 64px 65px 60px.
- Padding tablet/mobile: top 40px/24px, x 32px/16px, bottom 56px/40px.
- min-height: calc(100vh - 110px).
- Display flex column.
- Inner layout: flex-1, flex column on mobile, flex row on lg, lg align-stretch, gap 40px mobile, 0 desktop.

Left content:
- Flex column, justify-between, flex-1.
- H1: PP Mori normal, white, tracking-tight.
- Font size: clamp(52px, 9vw, 128px).
- Line height: 0.82.
- Three block lines:
  1. "Nature's" in white
  2. "Perfect" in white at 50% opacity
  3. "Hideaways" in white
- Animate each line with fade-up:
  initial opacity 0, y 28
  animate opacity 1, y 0
  transition duration 0.7, ease [0.22, 1, 0.36, 1]
  delays: 0.2, 0.35, 0.5

Bottom left row:
- margin-top 40px on small screens, 0 on lg.
- Flex column on mobile, row on sm/lg.
- align-start, gap 32px; desktop lg gap 0.
- Description paragraph:
  text: "Discover handpicked luxury cabins in breathtaking locations. Unplug, unwind, and reconnect with what matters most."
  PP Mori normal, color white/80, line-height 24px.
  Width 300px on sm/lg, full width on mobile.
  Font size clamp(15px, 1.3vw, 18px).
  Fade-up delay 0.65.
- Rating block:
  flex column gap 8px.
  sm margin-left 32px, lg margin-left 140px.
  First row flex align-center gap 4px.
  Star icon: 28px square, fill #FFB33F, use the same star SVG path from the existing component.
  Rating text "4.7": white, PP Mori normal, 36px, line-height 34px.
  Caption text: " from 1,800+ stays", white, PP Mori normal, 20px, line-height 24px.
  Fade-up delay 0.75.

Right reserve card:
- Wrapper animates from right:
  initial opacity 0, x 40
  animate opacity 1, x 0
  transition duration 0.8, ease [0.22, 1, 0.36, 1], delay 0.4
- Layout wrapper: flex; on lg flex-col justify-end; lg padding-left 40px, xl padding-left 64px.
- Card: relative, width 100%, max-width 410px, border-radius 35px, padding 30px, flex column, gap 30px.
- Add two absolute inset layers inside card:
  1. backdrop blur layer: inset 0, backdrop-filter blur(12.5px), border-radius 35px, pointer-events none.
  2. tint layer: inset 0, background rgba(0,0,0,0.25), mix-blend-mode soft-light, border-radius 35px, pointer-events none.

Reserve card content:
- Title row: relative flex, align-start, justify-between, gap 16px.
- Cabin title: PP Mori normal, white, 32px, line-height 36px, width 280px, two lines:
  "Evergreen "
  "Pine Family Lodge"
- Edit icon circle: background rgba(0,0,0,0.35), size 40px, border-radius 9999px, centered. Icon is 20px, stroke #BDC6C7 at 40% opacity using the existing SVG paths.
- Date row:
  Two equal pills in a flex row gap 10px.
  Each pill button: width 100%, background rgba(0,0,0,0.35), display flex, gap 6px, align center, padding 16px, border-radius 10px.
  Text: white, PP Mori normal, 16px/24px, flex-1, text-left.
  Values: "Feb 11" and "Mar 25".
  Calendar icon 20px, stroke #515C62. Chevron icon 16px, stroke #515C62.
  Hover: background rgba(0,0,0,0.5), scale 1.02. Tap scale 0.97. Duration 0.18 easeOut.
  When open, add ring 1px white/20 and rotate chevron 180deg over 0.2s.
- Dropdown calendar:
  absolute top calc(100% + 8px), left/right 0, z-index 50, border-radius 16px, overflow hidden, padding 16px.
  Background rgba(30,38,60,0.97), backdrop-filter blur(16px).
  Month label centered, white/60, 13px/20px, margin-bottom 12px, tracking-wide.
  Grid 7 columns, vertical gap 4px. Weekday labels white/30, 11px/20px.
  Day buttons 13px/20px, height 28px, border-radius 8px. Selected day white background, text #34405c, semibold.
  Calendar months: February 2025, 28 days, starts after 6 blanks; March 2025, 31 days, starts after 6 blanks.
- Time info box:
  background rgba(0,0,0,0.35), height 73px, border-radius 10px, overflow hidden, position relative.
  Left label: "Check-in", x 16px, top 16px, PP Mori normal, 14px/16px, color #bdc6c7, opacity .4.
  Left value: "After 2:00 PM", x 16px, top 34px, white, 16px/24px.
  Right label: "Check-out", right 16px, top 16px, same label style.
  Right value: "Until 12:00 PM", right 16px, top 34px, white, 16px/24px.
  Center divider: vertical line, height 44px, width 1px, background #515C62, opacity .4, centered horizontally.
- Price row:
  flex column gap 24px.
  Price/guest line: flex align-end justify-between, no wrapping.
  Price: "$359" white, 32px, line-height 28px; "/night" color #515c62, 20px.
  Guest text: "2-5 guests", white, 14px/16px.
- Reserve button:
  width 100%, height 48px, background white, border-radius 10px, centered.
  Text "Reserve", PP Mori semibold, black, 16px/24px.
  Hover scale 1.03 and background #f0f0f0. Tap scale 0.97. Duration 0.18 easeOut.

Global animation helpers:
fadeUp(delay):
initial { opacity: 0, y: 28 }
animate { opacity: 1, y: 0 }
transition { duration: 0.7, ease: [0.22, 1, 0.36, 1], delay }

fadeDown(delay):
initial { opacity: 0, y: -20 }
animate { opacity: 1, y: 0 }
transition { duration: 0.6, ease: [0.22, 1, 0.36, 1], delay }

fadeRight(delay):
initial { opacity: 0, x: 40 }
animate { opacity: 1, x: 0 }
transition { duration: 0.8, ease: [0.22, 1, 0.36, 1], delay }

At a 1545x997 viewport, the rendered desktop geometry should approximately be:
- Header: x 0, y 0, w 1545, h 110, padding 60px 65px 0.
- Main: x 0, y 110, w 1545, h 887, padding 64px 65px 60px.
- H1: x 65, y 174, w about 955, h 315. Font 128px, line-height 104.96px.
- Description: x 65, y 841, w 300, h 96.
- Rating value row: x 505, y 841.
- Reserve card: x about 1084, y 502, w about 396 rendered at this viewport, h 436, padding 30px, radius 35px.

Do not add extra overlays, gradients, cards, marketing sections, or explanatory text. The hero should be the first screen and match this exact composition.

## VaultShield — Hero [sites/vaultshield]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(64).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vaultshield.webp

Create a fullscreen hero section for a password manager app called "VaultShield" using React, TypeScript, Tailwind CSS, Framer Motion, and Lucide React icons.

---

### Fonts

- **Heading font:** `Helvetica Now Display Bold` loaded from `https://db.onlinewebfonts.com/c/04e6981992c0e2e7642af2074ebe3901?family=Helvetica+Now+Display+Bold` (add as a `<link>` in `index.html`)
- **Body font:** `Inter` (weights 300-900) loaded from Google Fonts: `https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap` (imported in CSS)

### CSS Variables

```css
:root {
  --font-heading: 'Helvetica Now Display Bold', sans-serif;
  --font-body: 'Inter', sans-serif;
  --color-text: #192837;
  --color-accent: #7342E2;
  --color-login-bg: #F2F2EE;
}
```

### Background Video

Full-screen background video covering the entire viewport (`absolute inset-0, object-cover`):

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_003132_8b7edcb6-c64d-4a52-a9ca-879942e122ad.mp4
```

Attributes: `autoPlay`, `muted`, `loop`, `playsInline`

### Layout Structure

1. **Container:** `relative w-full min-h-screen`, font-family from `--font-body`, color from `--color-text`
2. **Navbar:** max-width 1280px, centered, z-10, `px-5 sm:px-8 py-4 sm:py-5`, flex with items centered and space-between
3. **Hero content:** max-width 1280px centered container with `paddingTop: clamp(40px, 8vw, 72px)`, content block capped at `max-width: 560px`

### Logo (SVG)

Custom SVG logo, 32x32, fill `#192837`, geometric angular shape:

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" fill="none" overflow="visible" viewBox="0 0 256 256">
  <path d="M 64 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 L 128 64 L 128 64.5 L 161 32 L 192 0 L 256 0 L 256 64 L 192 128 L 128 128 L 128 192 L 96 223 L 63.5 256 L 0 256 L 0 192 Z M 256 192 L 224 223 L 191.5 256 L 128 256 L 128 192 L 192 128 L 256 128 Z" fill="#192837"/>
</svg>
```

### Navbar Elements

- **Left:** Logo
- **Center (desktop only, `hidden md:flex`):** 5 links — `['Vault', 'Plans', 'Install', 'News', 'Help']`, text-sm font-medium, opacity hover effect
- **Right (desktop only):**
  - "Start For Free" button — `background: #7342E2`, white text, rounded-full, `px-5 py-2.5`
  - "Sign In" button — `background: #F2F2EE`, dark text, rounded-full, `px-5 py-2.5`
- **Mobile:** Hamburger icon (Menu/X from lucide-react), opens a right-side slide-in sheet

### Mobile Menu Sheet (AnimatePresence + Framer Motion)

- **Backdrop:** fixed inset-0, `rgba(25,40,55,0.35)` background with `blur(4px)` backdrop-filter
- **Sheet:** fixed right-0 top-0, width `min(88vw, 360px)`, height `100dvh`, background `#CFC8C5`, box-shadow `-12px 0 48px rgba(25,40,55,0.18)`
- **Sheet animation:** slides from `x: '100%'` to `x: 0`, ease `[0.22, 1, 0.36, 1]`, duration 0.45s
- **Sheet content:** Logo + close button header, 1px divider, staggered nav links (delay `0.18 + i * 0.07`), bottom CTA buttons matching desktop style

### Hero Heading

- Font: `var(--font-heading)`
- Size: `clamp(1.65rem, 5vw, 3rem)`
- Line-height: `1.05`
- Letter-spacing: `-0.01em`
- Color: `#192837`
- Margin-bottom: `24px`
- Contains inline Lucide icons (Zap, LockKeyhole, Fingerprint) at 24px, color `#192837`, vertically aligned middle, positioned `top: -2px`
- Text: "Lock Down Your Passwords with Ironclad Security"
  - Zap icon before "Lock"
  - LockKeyhole icon between "Passwords" and "with"
  - Fingerprint icon after "Security"

### Hero Subtext

- Font: `var(--font-body)`
- Size: `clamp(0.9rem, 2.5vw, 1.1rem)`
- Line-height: `1.65`
- Opacity: `0.8`
- Max-width: `560px`
- Text: "Zero stress, total control. VaultShield keeps you covered with unbreakable storage, one-tap access, and pro-grade tools for your non-stop world."

### CTA Button

- Background: `#7342E2`
- Color: white
- Border-radius: `50px`
- Padding: `17px 24px`
- Font: `var(--font-body)`, font-weight semibold
- Size: `clamp(0.9rem, 2vw, 1rem)`
- Box-shadow: `0 4px 24px rgba(115,66,226,0.28)`
- Min-width: `210px`
- Flex with space-between, gap `32px`
- Text: "Get It Free" with ArrowRightCircle icon (20px) on the right
- Hover: `scale(1.04)` + `brightness(1.1)`
- Tap: `scale(0.96)`

### Animations (Framer Motion)

**fadeUp variant** applied to heading (delay 0), subtext (delay 0.15s), and CTA button (delay 0.30s):

```js
hidden: { opacity: 0, y: 28 }
visible: { opacity: 1, y: 0, transition: { delay: i * 0.15, duration: 0.6, ease: [0.22, 1, 0.36, 1] } }
```

### Dependencies

- `react`, `react-dom`
- `framer-motion`
- `lucide-react` (icons: ArrowRightCircle, Zap, LockKeyhole, Fingerprint, Menu, X)
- Tailwind CSS

---

That is every detail needed to reproduce the hero section exactly as built.

## Velorix IIC — Hero [sites/velorix-iic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(87).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/velorix-iic.webp

### File: `src/App.tsx`

```tsx
import { ArrowRight, Menu, X } from 'lucide-react';
import { useState, useEffect } from 'react';

const BG_VIDEO = "https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_155101_f2540600-6fe9-433e-8e48-b3f4b72f0727.mp4";

const NAV_ITEMS = ['Platform', 'How it works', 'AI Defense', 'Connections', 'Insights'];

function HamburgerButton({ open, onClick }: { open: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="lg:hidden relative w-9 h-9 flex items-center justify-center rounded-full transition-all duration-300"
      style={{ backgroundColor: open ? '#1a1a1a' : 'transparent' }}
      aria-label="Toggle menu"
    >
      <span
        className="absolute transition-all duration-300 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={{ opacity: open ? 0 : 1, transform: open ? 'rotate(-90deg) scale(0.5)' : 'rotate(0deg) scale(1)' }}
      >
        <Menu size={20} color="white" strokeWidth={1.5} />
      </span>
      <span
        className="absolute transition-all duration-300 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={{ opacity: open ? 1 : 0, transform: open ? 'rotate(0deg) scale(1)' : 'rotate(90deg) scale(0.5)' }}
      >
        <X size={20} color="white" strokeWidth={1.5} />
      </span>
    </button>
  );
}

function MobileMenu({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <>
      <div
        className="fixed inset-0 z-30 lg:hidden transition-all duration-500"
        style={{
          backdropFilter: open ? 'blur(12px)' : 'blur(0px)',
          backgroundColor: open ? 'rgba(0,0,0,0.6)' : 'rgba(0,0,0,0)',
          pointerEvents: open ? 'auto' : 'none',
        }}
        onClick={onClose}
      />

      <div
        className="fixed top-0 left-0 right-0 z-40 lg:hidden overflow-hidden"
        style={{
          maxHeight: open ? '420px' : '0px',
          transition: 'max-height 0.5s cubic-bezier(0.23, 1, 0.32, 1)',
        }}
      >
        <div
          className="pt-20 pb-6 px-5"
          style={{ backgroundColor: 'rgba(8,8,8,0.97)', borderBottom: '1px solid rgba(255,255,255,0.08)' }}
        >
          <div className="flex flex-col gap-1">
            {NAV_ITEMS.map((item, i) => (
              <a
                key={item}
                href="#"
                onClick={onClose}
                className="text-white/70 hover:text-white text-base py-3 px-3 rounded-xl hover:bg-white/5 transition-all duration-200 flex items-center justify-between group"
                style={{
                  fontFamily: 'Inter, sans-serif',
                  transitionDelay: open ? `${i * 50 + 80}ms` : '0ms',
                  opacity: open ? 1 : 0,
                  transform: open ? 'translateY(0)' : 'translateY(-8px)',
                  transition: `opacity 0.4s cubic-bezier(0.23,1,0.32,1) ${i * 50 + 80}ms, transform 0.4s cubic-bezier(0.23,1,0.32,1) ${i * 50 + 80}ms, color 0.2s, background 0.2s`,
                }}
              >
                {item}
                <ArrowRight size={14} className="opacity-0 group-hover:opacity-40 -translate-x-1 group-hover:translate-x-0 transition-all duration-200" />
              </a>
            ))}
          </div>

          <div
            className="mt-5 pt-5"
            style={{
              borderTop: '1px solid rgba(255,255,255,0.07)',
              transitionDelay: open ? '360ms' : '0ms',
              opacity: open ? 1 : 0,
              transform: open ? 'translateY(0)' : 'translateY(-8px)',
              transition: `opacity 0.4s cubic-bezier(0.23,1,0.32,1) 360ms, transform 0.4s cubic-bezier(0.23,1,0.32,1) 360ms`,
            }}
          >
            <button
              className="w-full py-3 rounded-full text-black text-sm font-medium transition-all duration-300 hover:opacity-80"
              style={{ fontFamily: 'Inter, sans-serif', backgroundColor: '#ffffff' }}
            >
              Join the wait
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

function Navbar() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return (
    <>
      <nav className="absolute top-0 left-0 right-0 z-50 flex items-center justify-between px-5 py-4 lg:px-10 lg:py-6">
        <span className="text-white text-xl font-semibold tracking-tight" style={{ fontFamily: 'Inter, sans-serif' }}>
          velorix
        </span>
        <div className="hidden lg:flex items-center gap-1 rounded-full px-2 py-1.5" style={{ backgroundColor: '#0C0C0C' }}>
          {NAV_ITEMS.map((item) => (
            <a
              key={item}
              href="#"
              className="text-white/80 hover:text-white text-sm px-4 py-1.5 rounded-full hover:bg-white/10 transition-all duration-200"
              style={{ fontFamily: 'Inter, sans-serif' }}
            >
              {item}
            </a>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <HamburgerButton open={open} onClick={() => setOpen((v) => !v)} />
          <button
            className="hidden lg:block text-sm font-medium px-5 py-2 rounded-full text-black transition-all duration-300 hover:opacity-80"
            style={{ fontFamily: 'Inter, sans-serif', backgroundColor: '#ffffff' }}
          >
            Join the wait
          </button>
        </div>
      </nav>
      <MobileMenu open={open} onClose={() => setOpen(false)} />
    </>
  );
}

export default function App() {
  return (
    <div className="relative w-full h-screen overflow-hidden bg-black" style={{ fontFamily: 'Inter, sans-serif' }}>
      <video
        className="absolute inset-0 z-0 w-full h-full object-cover"
        src={BG_VIDEO}
        autoPlay
        loop
        muted
        playsInline
      />

      <Navbar />

      <div className="relative z-20 flex flex-col items-center text-center pt-[90px] md:pt-[120px] px-5 sm:px-8">
        <h1
          className="text-white font-normal leading-[1.12] tracking-tight max-w-3xl"
          style={{
            fontFamily: 'Inter, sans-serif',
            fontSize: 'clamp(1.75rem, 5vw, 2.6rem)',
          }}
        >
          Where precision finds its edge
          <br className="hidden sm:block" />
          {' '}and vision rewrites what comes next
        </h1>

        <p
          className="mt-5 md:mt-6 text-white/60 text-sm md:text-base leading-relaxed max-w-xs sm:max-w-sm md:max-w-md"
          style={{ fontFamily: "'Courier New', Courier, monospace", letterSpacing: '0.01em' }}
        >
          a seamless bridge - where raw ambition
          <br className="hidden sm:block" />
          {' '}and machine clarity converge as one
        </p>

        <button
          className="mt-7 md:mt-8 flex items-center gap-2.5 px-5 py-2.5 rounded-full text-black text-sm font-medium transition-all duration-300 hover:opacity-80 group"
          style={{ fontFamily: 'Inter, sans-serif', backgroundColor: '#ffffff' }}
        >
          Watch it unfold
          <ArrowRight size={15} className="group-hover:translate-x-0.5 transition-transform duration-200" />
        </button>
      </div>
    </div>
  );
}
```

### Assets

**Background video URL (verbatim):**
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_155101_f2540600-6fe9-433e-8e48-b3f4b72f0727.mp4
```

### Icons (from `lucide-react`)

Used via the `<Menu>`, `<X>`, and `<ArrowRight>` components. These are imported from the `lucide-react` npm package — the SVG path data is not inlined in this codebase; it ships inside the package. The three icons are rendered with:

- `<Menu size={20} color="white" strokeWidth={1.5} />`
- `<X size={20} color="white" strokeWidth={1.5} />`
- `<ArrowRight size={15} />` (hero button) and `<ArrowRight size={14} />` (mobile menu links)

### Dependencies (`package.json`)

```json
{
  "dependencies": {
    "@supabase/supabase-js": "^2.57.4",
    "lucide-react": "^0.344.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  }
}
```

### Animation values (all CSS, no Framer Motion)

**Hamburger icon crossfade** — `duration: 0.3s`, `ease: cubic-bezier(0.23,1,0.32,1)`; Menu icon `opacity 1→0`, `transform rotate(0deg) scale(1) → rotate(-90deg) scale(0.5)`; X icon inverse.

**Mobile menu backdrop** — `duration: 0.5s`; `backdropFilter blur(0px) → blur(12px)`; `background rgba(0,0,0,0) → rgba(0,0,0,0.6)`.

**Mobile menu panel** — `max-height: 0px → 420px`, `duration: 0.5s`, `ease: cubic-bezier(0.23, 1, 0.32, 1)`.

**Mobile menu link stagger** — each item `duration: 0.4s`, `ease: cubic-bezier(0.23,1,0.32,1)`, `delay: i * 50 + 80ms` (80, 130, 180, 230, 280); `opacity 0 → 1`, `transform translateY(-8px) → translateY(0)`.

**Mobile menu CTA** — `duration: 0.4s`, `ease: cubic-bezier(0.23,1,0.32,1)`, `delay: 360ms`.

**Hero button arrow** — hover `translate-x-0.5`, `duration: 0.2s`.

No Supabase persistence is used on this marketing section — it's presentational only, and nothing on this hero is user-specific or stateful across sessions.

## Vertex Sci — Hero [sites/vertex-sci]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(23).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vertex-sci.webp

Create a full-screen cinematic hero section with a fixed navbar for a fictional scientific research lab website called "Vertex Sci." using React, Vite, TypeScript, and Tailwind CSS. The design is dark, minimal, and uses monospace typography exclusively. No colors other than black and white at various opacities. Mobile responsive.

---

### SETUP

**Tech stack:** React + Vite + TypeScript + Tailwind CSS (no additional UI libraries or icon packs needed).

**Font:** JetBrains Mono from Google Fonts. Add this to `index.html` `<head>`:

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@300;400;500;700;800&display=swap" rel="stylesheet" />
```

**Page title:** `Vertex Sci.`

**Global CSS reset in index.css:**
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
  font-family: 'JetBrains Mono', monospace;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

**Tailwind config** - extend fontFamily:
```js
fontFamily: {
  mono: ['JetBrains Mono', 'monospace'],
}
```

---

### NAVBAR (Fixed, overlays hero)

Create a `Navbar` component with these exact specs:

- `position: fixed`, `top: 0`, full width, `z-50`
- Transparent by default. On scroll > 20px, transition to `bg-black/60 backdrop-blur-md` (300ms transition)
- Height: `h-16` on mobile, `h-20` on md+
- Horizontal padding: `px-6 sm:px-10 md:px-16 lg:px-20` (same as hero content)
- Flexbox row: `items-center justify-between`

**Left - Logo:** A custom inline SVG, 32x32, white fill, geometric angular shape. SVG path:
```html
<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256" fill="none">
  <path d="M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 96 95 L 63.5 128 L 64 128 L 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 64 L 64 0 L 192 0 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z" fill="white" />
</svg>
```

**Center (desktop only, hidden below lg):** Navigation links in a flex row with `gap-8`:
- "Projects", "Facilities", "Discoveries", "Team"
- Each link: `text-white/70 text-xs uppercase tracking-[0.2em] font-light hover:text-white transition-colors duration-200`

**Right (desktop only, hidden below lg):** Two buttons with `gap-3`:
1. "Inquire" - outline style: `px-5 py-2.5 border border-white/30 text-white text-xs uppercase tracking-[0.15em] font-light hover:border-white/60 transition-all duration-200`
2. "Join Study" - solid style: `px-5 py-2.5 bg-white text-black text-xs uppercase tracking-[0.15em] font-medium hover:bg-white/90 transition-all duration-200`

**Mobile hamburger (visible below lg):**
- Three horizontal lines (`w-6 h-[1.5px] bg-white`) with `gap-1.5`
- On open: top line rotates +45deg and translates down 4.5px, middle line fades/scales to 0, bottom line rotates -45deg and translates up 4.5px. 300ms ease-out transitions.

**Mobile menu overlay (below lg):**
- Full screen fixed overlay, `z-40`
- Solid black background with opacity transition (500ms)
- Content: vertically stacked links with staggered entrance animations (each link delayed by `index * 60 + 150ms`)
- Each link is large (`text-2xl sm:text-3xl font-light tracking-tight`) with a bottom border (`border-white/10`) and a numbered indicator on the right (`01`, `02`, `03`, `04` in `text-white/30 text-xs`)
- Bottom of mobile menu: same two buttons ("Inquire" outline, "Join Study" solid) stacked full-width with `py-4`, delayed 400ms entrance
- Body scroll locked when menu is open

---

### HERO SECTION

A `<section>` that is `relative w-full h-screen overflow-hidden bg-black`.

### Layer 1: Background Video (absolute, z-auto)
```html
<video autoPlay muted loop playsInline className="absolute inset-0 w-full h-full object-cover">
  <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_202655_a7f5aca0-2f80-4bc9-bcb5-96ac95662003.mp4" type="video/mp4" />
</video>
```
- Animation class: starts at `opacity: 0; transform: scale(1.05)`, animates to `opacity: 1; transform: scale(1)` over **1.8s** with `cubic-bezier(0.16, 1, 0.3, 1)` and `forwards` fill.

### Layer 2: Gradient Overlay (z-[5])
- `absolute inset-x-0 bottom-0 h-[60%]`
- `bg-gradient-to-t from-black/80 via-black/30 to-transparent`
- `pointer-events-none`

### Layer 3: Content (z-10)
- `relative z-10 h-full flex flex-col justify-end`
- Padding: `px-6 sm:px-10 md:px-16 lg:px-20 pb-12 md:pb-16 lg:pb-20`

### A) Label (above the two-column layout)
- Text: `"Deep-Structure Lab. By Vertex Sci."`
- Classes: `text-white/50 text-[10px] sm:text-xs tracking-[0.3em] uppercase font-light mb-8 md:mb-12`
- Animation: simple `fadeIn` (opacity 0 to 1), **1s**, delayed **0.4s**, same cubic-bezier, forwards

### B) Two-column layout
- Container: `flex flex-col lg:flex-row lg:items-end lg:justify-between gap-10 lg:gap-20`

**LEFT COLUMN** (`flex-shrink-0`):

1. **Headline `<h1>`:**
   ```
   Fracture
   Pattern
   Dynamics
   ```
   Three words separated by `<br />` tags.
   - Classes: `text-white font-bold text-[clamp(2.5rem,8vw,5rem)] leading-[0.9] tracking-[-0.06em] uppercase`
   - Animation: `fadeSlideUp` (translateY(30px) + opacity 0 -> translateY(0) + opacity 1), **1s**, delayed **0.6s**, cubic-bezier(0.16, 1, 0.3, 1), forwards

2. **Meta line** (below headline):
   - Container: `mt-6 flex items-center gap-6 text-white/40 text-[10px] sm:text-xs tracking-wider uppercase font-light`
   - Content: `"Batch: KX-071243"` then a divider then `"Phase: Sigma"`
   - Divider: `<span>` with `w-8 h-[1px] bg-white/20 inline-block`, animated with `revealLine` (clip-path: inset(0 100% 0 0) -> inset(0 0% 0 0)), **1s**, delayed **0.9s**
   - Meta container animation: `fadeSlideUp`, **0.8s**, delayed **1.0s**, forwards

**RIGHT COLUMN** (`flex flex-col gap-8 lg:max-w-md`):

1. **Description paragraph:**
   ```
   Advancing sub-atomic fracture mapping across the crystalline stress interface. Photon array diagnostics revealing the most intricate deformation cycles in deep material forensics.
   ```
   - Classes: `text-white/60 text-xs sm:text-sm leading-relaxed font-light`
   - Animation: `fadeSlideUp`, **0.8s**, delayed **1.1s**, forwards

2. **Stats row:**
   - Container: `flex items-end gap-8 sm:gap-12`
   - Each stat is a vertical stack: `flex flex-col gap-1`
   - Stat values: `text-white text-2xl sm:text-3xl font-bold tracking-tight`
   - Stat labels: `text-white/40 text-[10px] sm:text-xs uppercase tracking-wider font-light`

   Three stats with staggered animations (all `fadeSlideUp`, **0.7s**, forwards):
   | Value | Label | Delay |
   |-------|-------|-------|
   | 7.91 | Ref. Index | 1.3s |
   | ULTRA | Clarity | 1.45s |
   | x500 degrees (use `&deg;` entity) | Resolution | 1.6s |

---

### ALL CSS KEYFRAMES (add to index.css)

```css
@keyframes fadeSlideUp {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes revealLine {
  from { clip-path: inset(0 100% 0 0); }
  to { clip-path: inset(0 0% 0 0); }
}

@keyframes scaleIn {
  from {
    opacity: 0;
    transform: scale(1.05);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
```

**Animation classes** (add to index.css - each element starts hidden and animates in):
```css
.animate-hero-video {
  animation: scaleIn 1.8s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  opacity: 0;
}
.animate-hero-label {
  animation: fadeIn 1s cubic-bezier(0.16, 1, 0.3, 1) 0.4s forwards;
  opacity: 0;
}
.animate-hero-title {
  animation: fadeSlideUp 1s cubic-bezier(0.16, 1, 0.3, 1) 0.6s forwards;
  opacity: 0;
}
.animate-hero-meta {
  animation: fadeSlideUp 0.8s cubic-bezier(0.16, 1, 0.3, 1) 1s forwards;
  opacity: 0;
}
.animate-hero-description {
  animation: fadeSlideUp 0.8s cubic-bezier(0.16, 1, 0.3, 1) 1.1s forwards;
  opacity: 0;
}
.animate-hero-stat-1 {
  animation: fadeSlideUp 0.7s cubic-bezier(0.16, 1, 0.3, 1) 1.3s forwards;
  opacity: 0;
}
.animate-hero-stat-2 {
  animation: fadeSlideUp 0.7s cubic-bezier(0.16, 1, 0.3, 1) 1.45s forwards;
  opacity: 0;
}
.animate-hero-stat-3 {
  animation: fadeSlideUp 0.7s cubic-bezier(0.16, 1, 0.3, 1) 1.6s forwards;
  opacity: 0;
}
.animate-hero-divider {
  animation: revealLine 1s cubic-bezier(0.16, 1, 0.3, 1) 0.9s forwards;
  clip-path: inset(0 100% 0 0);
}
```

---

### DESIGN RULES

- **ONLY black and white** - no color accents whatsoever
- White is used at opacities: /40, /50, /60, /70, /90 for hierarchy
- All text is uppercase except the description paragraph
- Tight negative letter-spacing on headline (`-0.06em`), wide tracking on labels (`0.2em` - `0.3em`)
- The animation cascade creates a cinematic reveal sequence from 0.4s to 1.6s
- All animations use the same smooth easing: `cubic-bezier(0.16, 1, 0.3, 1)`
- Fully responsive: stacks vertically on mobile, two-column on lg+
- Font sizes use clamp for fluid scaling on the headline

## Vision Reveal — Hero [sites/vision-reveal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/newpsotArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vision-reveal.mp4

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1.0"/>
<title>Creative Studio Showcase</title>
<link rel="preconnect" href="https://fonts.googleapis.com"/>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet"/>
<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
* { font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }

html, body {
  margin: 0; padding: 0;
  background: #E4E4E4;
  color: #F4F1E8;
  overflow-x: hidden;
  scroll-behavior: smooth;
}

/* ===== SPLASH ===== */
.splash {
  position: fixed; inset: 0;
  width: 100vw; height: 100vh;
  z-index: 9999;
  pointer-events: none;
  overflow: hidden;
  animation: splashHide 0.3s ease forwards;
  animation-delay: 1.35s;
}
.splash-row { display: flex; width: 100%; height: 50%; }
.splash-box { width: 20%; height: 100%; background: #75C5DE; }
.splash-row-top .splash-box { animation: splashTop 1s cubic-bezier(0.96,-0.02,0.38,1.01) forwards; }
.splash-row-bottom .splash-box { animation: splashBottom 1s cubic-bezier(0.96,-0.02,0.38,1.01) forwards; }
.splash-box:nth-child(1) { animation-delay: 0s; }
.splash-box:nth-child(2) { animation-delay: 0.05s; }
.splash-box:nth-child(3) { animation-delay: 0.1s; }
.splash-box:nth-child(4) { animation-delay: 0.15s; }
.splash-box:nth-child(5) { animation-delay: 0.2s; }

@keyframes splashTop { from { transform: translateY(0%); } to { transform: translateY(-100%); } }
@keyframes splashBottom { from { transform: translateY(0%); } to { transform: translateY(100%); } }
@keyframes splashHide { to { opacity: 0; visibility: hidden; } }

/* ===== HERO IMAGE ENTRANCE ===== */
@keyframes heroImageIn {
  from { opacity: 0; transform: scale(1.5) rotate(3deg); }
  to { opacity: 1; transform: scale(1) rotate(0deg); }
}
.hero-image-animate {
  animation: heroImageIn 1.2s cubic-bezier(0.25,0.46,0.45,0.94) forwards;
  animation-delay: 1s;
  opacity: 0;
}

/* ===== WORD REVEAL ===== */
@keyframes wordReveal {
  from { opacity: 0; transform: translateY(10px); filter: blur(10px); }
  to { opacity: 1; transform: translateY(0); filter: blur(0); }
}
.word-reveal {
  opacity: 0;
  display: inline-block;
  margin-right: 0.3em;
  animation: wordReveal 0.4s ease forwards;
}

/* ===== CTA ENTRANCE ===== */
@keyframes slideUpScale {
  from { opacity: 0; transform: translateY(60px) scale(0.4); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.cta-animate {
  opacity: 0;
  animation: slideUpScale 0.8s cubic-bezier(0.25,0.46,0.45,0.94) forwards;
  animation-delay: 1s;
}

/* ===== CTA BUTTON ===== */
.cta-btn { position: relative; overflow: hidden; display: flex; align-items: center; border: none; background: none; cursor: pointer; border-radius: 9999px; padding: 8px; gap: 12px; }
.cta-btn-bg {
  position: absolute; top: 5px; bottom: 5px; left: 8px;
  width: calc(100% - 8px - 8px - 48px - 12px);
  border-radius: 9999px; background: white; z-index: 0;
  transition: width 0.4s cubic-bezier(0.25,0.46,0.45,0.94);
}
@media (min-width: 768px) { .cta-btn-bg { width: calc(100% - 8px - 8px - 54px - 12px); } }
.cta-btn:hover .cta-btn-bg { width: calc(100% - 16px); }
.cta-btn-text { position: relative; z-index: 1; color: #111111; font-weight: 500; font-size: 16px; padding: 12px 32px; white-space: nowrap; }
@media (min-width: 768px) { .cta-btn-text { font-size: 18px; padding: 16px 40px; } }
.cta-btn-circle {
  position: relative; z-index: 1; display: flex; align-items: center; justify-content: center;
  width: 48px; height: 48px; border-radius: 50%; background: #75C5DE; flex-shrink: 0;
  transition: transform 0.4s cubic-bezier(0.25,0.46,0.45,0.94);
}
@media (min-width: 768px) { .cta-btn-circle { width: 54px; height: 54px; } }
.cta-btn:hover .cta-btn-circle { transform: translateX(-7px); }

/* ===== MENU CTA (smaller) ===== */
.menu-cta-btn { position: relative; overflow: hidden; display: flex; align-items: center; border: none; background: none; cursor: pointer; border-radius: 9999px; padding: 6px; gap: 8px; }
.menu-cta-bg {
  position: absolute; top: 5px; bottom: 5px; left: 8px;
  width: calc(100% - 8px - 8px - 38px - 8px);
  border-radius: 9999px; background: white; z-index: 0;
  transition: width 0.4s cubic-bezier(0.25,0.46,0.45,0.94);
}
.menu-cta-btn:hover .menu-cta-bg { width: calc(100% - 12px); }
.menu-cta-text { position: relative; z-index: 1; color: #111111; font-weight: 500; font-size: 14px; padding: 8px 40px; white-space: nowrap; }
.menu-cta-circle {
  position: relative; z-index: 1; display: flex; align-items: center; justify-content: center;
  width: 38px; height: 38px; border-radius: 50%; background: #75C5DE; flex-shrink: 0;
  transition: transform 0.3s ease;
}
.menu-cta-btn:hover .menu-cta-circle { transform: translateX(-4px); }

/* ===== CREATOR TEXT ===== */
@keyframes creatorSlideUp { from { transform: translateY(330px); } to { transform: translateY(0); } }
.creator-text-animate {
  transform: translateY(330px);
  animation: creatorSlideUp 1s cubic-bezier(0.16,1,0.3,1) forwards;
  animation-delay: 1.5s;
}

/* ===== NAVIGATION ===== */
.logo-wrapper {
  position: fixed; top: 30px; left: 0; width: 50%; z-index: 10;
  display: flex; justify-content: flex-start; align-items: center; mix-blend-mode: difference;
}
@media (min-width: 768px) { .logo-wrapper { top: 40px; } }
.logo-wrapper .inner { padding-left: 20px; }
@media (min-width: 768px) { .logo-wrapper .inner { padding-left: 40px; } }
.logo-wrapper img { width: 32px; height: 32px; }

.burger-wrapper {
  position: fixed; top: 16px; right: 0; width: 50%; z-index: 10;
  display: flex; justify-content: flex-end; align-items: center;
}
@media (min-width: 768px) { .burger-wrapper { top: 27px; } }
.burger-wrapper .inner { padding-right: 20px; }
@media (min-width: 768px) { .burger-wrapper .inner { padding-right: 40px; } }

.burger-btn {
  width: 59px; height: 59px; border-radius: 50%; border: none; cursor: pointer;
  display: flex; flex-direction: column; gap: 4px; align-items: center; justify-content: center;
  background: #F4F1E8; transition: background 0.4s ease;
}
.burger-btn:hover { background: #0B0B0B; }
.burger-btn .bar {
  display: block; width: 24px; height: 2px; background: #111111;
  transition: all 0.3s ease;
}
.burger-btn:hover .bar { background: #F4F1E8; }
.burger-btn.open { background: #0B0B0B; }
.burger-btn.open .bar { background: #F4F1E8; }
.burger-btn.open .bar:first-child { transform: rotate(45deg) translate(2px, 2px); }
.burger-btn.open .bar:last-child { transform: rotate(-45deg) translate(2px, -2px); }

/* ===== MENU PANEL ===== */
.menu-panel {
  position: fixed; z-index: 9;
  left: 8px; right: 8px;
  border-radius: 20px;
  background: rgba(17,17,17,0.95);
  backdrop-filter: blur(26px); -webkit-backdrop-filter: blur(26px);
  padding: 90px 32px 32px 32px;
  display: flex; flex-direction: column; justify-content: space-between;
  transition: top 0.5s cubic-bezier(0.25,0.46,0.45,0.94), opacity 0.4s ease;
  top: -600px; opacity: 0; pointer-events: none;
}
@media (min-width: 768px) {
  .menu-panel { left: auto; right: 7px; width: 420px; padding: 60px; }
}
.menu-panel.open { top: 0; opacity: 1; pointer-events: auto; }
@media (min-width: 768px) { .menu-panel.open { top: 7px; } }

.menu-panel nav { display: flex; flex-direction: column; gap: 8px; }
.menu-panel nav a {
  color: #F4F1E8; font-size: 36px; font-weight: 500; text-decoration: none;
  line-height: 130%; transition: opacity 0.3s ease;
}
@media (min-width: 768px) { .menu-panel nav a { font-size: 42px; } }
.menu-panel nav a:hover { opacity: 0.7; }

.menu-contact { display: flex; flex-direction: column; gap: 20px; margin-top: 32px; }
.menu-email { color: #9A9590; font-size: 18px; text-decoration: none; transition: color 0.3s ease; }
@media (min-width: 768px) { .menu-email { font-size: 20px; } }
.menu-email:hover { color: #F4F1E8; }
.menu-socials { display: flex; gap: 24px; }
.menu-socials a {
  color: #9A9590; font-size: 14px; text-decoration: underline;
  text-underline-offset: 2px; transition: color 0.3s ease;
}
.menu-socials a:hover { color: #F4F1E8; }

/* ===== HERO ===== */
.hero {
  position: relative; width: 100%; overflow: hidden;
  background: #E4E4E4; min-height: 100vh;
}
@media (min-width: 768px) { .hero { height: 100vh; min-height: 800px; } }

.hero-big-text {
  position: absolute; bottom: -30px; left: 0; right: 0; z-index: 2;
  pointer-events: none; width: 100%; text-align: center;
}
@media (min-width: 768px) { .hero-big-text { bottom: -40px; } }
.hero-big-text h2 {
  font-weight: 500; color: #F4F1E8; line-height: 80%;
  letter-spacing: -0.04em; white-space: nowrap;
  font-size: clamp(180px, 28vw, 560px);
}

.hero-base-img {
  position: absolute; top: 30vh; left: 0; right: 0; bottom: 0;
  background-size: cover; background-repeat: no-repeat;
  background-position: 60% center; z-index: 5;
}
@media (min-width: 768px) { .hero-base-img { top: 0; background-position: center; } }

.hero-reveal-img {
  position: absolute; top: 30vh; left: 0; right: 0; bottom: 0;
  background-size: cover; background-repeat: no-repeat;
  background-position: 60% center; z-index: 7; pointer-events: none;
}
@media (min-width: 768px) { .hero-reveal-img { top: 0; background-position: center; } }

.hero-content {
  position: relative; z-index: 8;
  display: flex; flex-direction: column; justify-content: flex-start; align-items: flex-start;
  width: 100%; max-width: 1600px; margin: 0 auto;
  padding: 110px 16px 24px 16px; pointer-events: none;
}
@media (min-width: 768px) {
  .hero-content {
    position: absolute; inset: 0;
    justify-content: space-between;
    padding: 160px 40px 100px 40px;
  }
}
.hero-content-inner { display: flex; flex-direction: column; align-items: flex-start; gap: 30px; width: 100%; pointer-events: auto; }

.hero-headline {
  font-size: 22px; font-weight: 500; line-height: 120%;
  letter-spacing: -0.02em; color: #111111; max-width: 447px;
}
@media (min-width: 768px) { .hero-headline { font-size: 28px; } }

/* ===== CANVAS (hidden) ===== */
#reveal-canvas { display: none; position: absolute; inset: 0; pointer-events: none; }

/* ===== REDUCED MOTION ===== */
@media (prefers-reduced-motion: reduce) {
  .splash { animation: splashHide 0.01s linear forwards; }
  .splash-box { animation: none !important; }
  .hero-image-animate, .word-reveal, .cta-animate, .creator-text-animate {
    animation: none !important; opacity: 1 !important;
    transform: none !important; filter: none !important; visibility: visible !important;
  }
}
</style>
</head>
<body>

<!-- SPLASH -->
<div class="splash" id="splash">
  <div class="splash-row splash-row-top">
    <div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div>
  </div>
  <div class="splash-row splash-row-bottom">
    <div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div><div class="splash-box"></div>
  </div>
</div>

<!-- LOGO -->
<div class="logo-wrapper">
  <div class="inner">
    <a href="/" aria-label="Home">
      <img src="https://framerusercontent.com/images/VMcS7YYTM5PXfXvlHc9u3hSCMM.svg" alt=""/>
    </a>
  </div>
</div>

<!-- BURGER -->
<div class="burger-wrapper">
  <div class="inner">
    <button class="burger-btn" id="burger-btn" aria-label="Open menu">
      <span class="bar"></span>
      <span class="bar"></span>
    </button>
  </div>
</div>

<!-- MENU PANEL -->
<div class="menu-panel" id="menu-panel">
  <nav>
    <a href="#work">Work</a>
    <a href="#about">About</a>
    <a href="#blog">Blog</a>
  </nav>
  <div class="menu-contact">
    <a href="mailto:studio@norakessler.com" class="menu-email">studio@norakessler.com</a>
    <div class="menu-socials">
      <a href="#">Pinterest</a>
      <a href="#">Behance</a>
      <a href="#">Letterboxd</a>
    </div>
  </div>
  <div style="margin-top:32px;">
    <button class="menu-cta-btn">
      <span class="menu-cta-bg"></span>
      <span class="menu-cta-text">Let's talk</span>
      <span class="menu-cta-circle">
        <svg width="14" height="14" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M5 13L13 5M13 5H6M13 5V12" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </span>
    </button>
  </div>
</div>

<!-- HERO -->
<main class="hero">
  <!-- Big text behind image -->
  <div class="hero-big-text creator-text-animate">
    <h2>Visuals</h2>
  </div>

  <!-- Base image -->
  <div class="hero-base-img hero-image-animate"
       style="background-image:url('https://soft-zoom-63098134.figma.site/_assets/v11/5c9f982199fde1d9b85a20e5396f0fa7bacaf9a3.png?w=2560');">
  </div>

  <!-- Reveal layer -->
  <canvas id="reveal-canvas"></canvas>
  <div class="hero-reveal-img" id="reveal-img"
       style="background-image:url('https://soft-zoom-63098134.figma.site/_assets/v11/6be2165e31648955b4e071f4cf2a50bc572b9bfd.png?w=1536');">
  </div>

  <!-- Content -->
  <div class="hero-content">
    <div class="hero-content-inner">
      <h1 class="hero-headline" id="headline"></h1>
      <button class="cta-btn cta-animate">
        <span class="cta-btn-bg"></span>
        <span class="cta-btn-text">Start a project now</span>
        <span class="cta-btn-circle">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M5 13L13 5M13 5H6M13 5V12" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
      </button>
    </div>
  </div>
</main>

<script>
(function() {
  // Word reveal
  const headline = document.getElementById('headline');
  const text = "I build compelling visual stories & motion that make ideas shine.";
  const words = text.split(' ');
  words.forEach(function(word, i) {
    const span = document.createElement('span');
    span.className = 'word-reveal';
    span.textContent = word;
    span.style.animationDelay = (1 + i * 0.05) + 's';
    headline.appendChild(span);
  });

  // Burger menu toggle
  const burgerBtn = document.getElementById('burger-btn');
  const menuPanel = document.getElementById('menu-panel');
  let menuOpen = false;
  burgerBtn.addEventListener('click', function() {
    menuOpen = !menuOpen;
    if (menuOpen) {
      burgerBtn.classList.add('open');
      menuPanel.classList.add('open');
      burgerBtn.setAttribute('aria-label', 'Close menu');
    } else {
      burgerBtn.classList.remove('open');
      menuPanel.classList.remove('open');
      burgerBtn.setAttribute('aria-label', 'Open menu');
    }
  });
  // Close menu on nav link click
  menuPanel.querySelectorAll('nav a').forEach(function(a) {
    a.addEventListener('click', function() {
      menuOpen = false;
      burgerBtn.classList.remove('open');
      menuPanel.classList.remove('open');
    });
  });

  // Spotlight reveal
  const SPOTLIGHT_R = 260;
  const canvas = document.getElementById('reveal-canvas');
  const imgLayer = document.getElementById('reveal-img');
  const ctx = canvas.getContext('2d');

  function resizeCanvas() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
  }
  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);

  const mouse = { x: -999, y: -999 };
  const smooth = { x: -999, y: -999 };

  window.addEventListener('mousemove', function(e) {
    mouse.x = e.clientX;
    mouse.y = e.clientY;
  });

  function loop() {
    smooth.x += (mouse.x - smooth.x) * 0.1;
    smooth.y += (mouse.y - smooth.y) * 0.1;

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    var grad = ctx.createRadialGradient(smooth.x, smooth.y, 0, smooth.x, smooth.y, SPOTLIGHT_R);
    grad.addColorStop(0, 'rgba(255,255,255,1)');
    grad.addColorStop(0.4, 'rgba(255,255,255,1)');
    grad.addColorStop(0.6, 'rgba(255,255,255,0.75)');
    grad.addColorStop(0.75, 'rgba(255,255,255,0.4)');
    grad.addColorStop(0.88, 'rgba(255,255,255,0.12)');
    grad.addColorStop(1, 'rgba(255,255,255,0)');

    ctx.beginPath();
    ctx.arc(smooth.x, smooth.y, SPOTLIGHT_R, 0, Math.PI * 2);
    ctx.fillStyle = grad;
    ctx.fill();

    var dataUrl = canvas.toDataURL();
    imgLayer.style.webkitMaskImage = 'url(' + dataUrl + ')';
    imgLayer.style.maskImage = 'url(' + dataUrl + ')';
    imgLayer.style.webkitMaskSize = '100% 100%';
    imgLayer.style.maskSize = '100% 100%';

    requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);
})();
</script>
</body>
</html>

## Visual Hero — Hero [sites/visual-hero]

- Preview: https://stream.mux.com/i9kUFJpB6GrWoe2UXRZG4lIP02g00LGulS1GTVrMMwZI00.m3u8
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/visual-hero.m3u8

**Build a fullscreen hero section in a Vite + React + TypeScript + Tailwind CSS project. Use `gsap` and `lucide-react`. No other UI libraries.**

### Fonts (in `src/index.css`)
Import at the top of index.css BEFORE `
@tailwind
` directives:
```css
@import
 url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap');

@font
-face {
  font-family: 'Dirtyline';
  src: url('https://fonts.cdnfonts.com/s/15011/Dirtyline36DaysofType.woff') format('woff');
  font-weight: normal;
  font-style: normal;
  font-display: swap;
}
```
Body font: `'Barlow', sans-serif`, background `#000`.

### Tailwind config (`tailwind.config.js`)
```js
theme: {
  extend: {
    fontFamily: {
      heading: ['Instrument Serif', 'serif'],
      body: ['Barlow', 'sans-serif'],
      dirtyline: ['Dirtyline', 'sans-serif'],
    },
    borderRadius: { DEFAULT: '9999px' },
  },
},
```

### CSS (append to `src/index.css`)
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
    rgba(255,255,255,0)    40%,
    rgba(255,255,255,0)    60%,
    rgba(255,255,255,0.15) 80%,
    rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}

.liquid-glass-strong {
  background: rgba(255,255,255,0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(50px);
  -webkit-backdrop-filter: blur(50px);
  border: none;
  box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15);
  position: relative;
  overflow: hidden;
}
.liquid-glass-strong::before {
  content: "";
  position: absolute; inset: 0;
  border-radius: inherit;
  padding: 1.4px;
  background: linear-gradient(180deg,
    rgba(255,255,255,0.5) 0%,
    rgba(255,255,255,0.2) 20%,
    rgba(255,255,255,0)   40%,
    rgba(255,255,255,0)   60%,
    rgba(255,255,255,0.2) 80%,
    rgba(255,255,255,0.5) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}

.hero-title {
  font-family: 'Instrument Serif', serif;
  font-style: italic;
  font-size: clamp(96px, 18vw, 280px);
  line-height: 0.92;
  letter-spacing: -0.02em;
  color: white;
  text-align: center;
}
```

### Component (`src/App.tsx`)

**Constants:**
- `NAV_LINKS = ['Gallery', 'Styles', 'API', 'Pricing', 'Blog']`
- `VIDEO_SRC = 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_080827_a9e5ad52-b6ee-4e79-b393-d936f179cfd7.mp4'`

**LogoMark** — inline SVG, 44x26, viewBox `0 0 44 26`, three white rects at x=0/16/30, y=3, widths 14/12/14, height 20, rx=3.

**State/refs:**
- `mounted` (boolean, set true in a mount effect for fade-in).
- `videoRef` (HTMLVideoElement), `videoBgRef` (HTMLDivElement), `displayCanvasRef` (HTMLCanvasElement).
- `framesReady` boolean state, `framesRef` = `useRef<HTMLCanvasElement[]>([])`.

**Effect 1 — Frame capture (boomerang setup):**
- On mount, get `videoRef.current`. Set `capturing = true`, `lastTime = -1`, `MAX_WIDTH = 960`, `frames: HTMLCanvasElement[] = []`.
- `captureFrame()`: bail if `!capturing` or `readyState < 2` or `currentTime === lastTime`. Update `lastTime`. Scale = `min(1, 960/videoWidth)`. Create offscreen canvas at scaled w/h, `ctx.drawImage(video, 0, 0, w, h)`, push to frames.
- Use `requestVideoFrameCallback` when available, else `requestAnimationFrame` fallback.
- On `loadedmetadata`: call `http://video.play().catch(()=>{})` then start the capture loop.
- On `ended`: set `capturing = false`, store frames in `framesRef.current`, `setFramesReady(true)`.
- If `readyState >= 1`, invoke `onLoaded()` immediately.
- Cleanup: cancel raf + remove listeners.

**Effect 2 — Boomerang render:**
- When `framesReady` true, grab `displayCanvasRef`, set its `width/height` from `frames[0]`.
- Variables: `index = 0`, `direction = 1`, `last = http://performance.now()`, `interval = 1000/30`.
- In an `requestAnimationFrame(render)` loop: if `now - last >= interval`, draw `frames[index]`, advance `index += direction`. When `index >= frames.length - 1`, clamp and flip to `-1`. When `index <= 0`, clamp and flip to `+1`.
- Cleanup: cancelAnimationFrame.

**Effect 3 — Parallax mouse tracking (gsap):**
- `strength = 20`. Track `targetX/Y`, smoothly lerp `currentX/Y += (target - current) * 0.06` each frame.
- On `mousemove`: `targetX = ((clientX - cx)/cx) * strength` (same for Y).
- Each frame: `gsap.set(videoBgRef.current, { x: currentX, y: currentY })`.

**JSX structure:**
Root: `<div className="min-h-screen bg-black text-white font-body overflow-x-hidden">`

1. **Video background layer:** `<div ref={videoBgRef} className="fixed top-0 left-0 w-full h-full z-0 scale-[1.08] origin-center">` containing:
   - `<video>` with `src={VIDEO_SRC}`, `muted`, `playsInline`, `preload="auto"`, `crossOrigin="anonymous"`, `className="w-full h-full object-cover"`, `style={{ display: framesReady ? 'none' : 'block' }}`.
   - `<canvas ref={displayCanvasRef} className="w-full h-full object-cover" style={{ display: framesReady ? 'block' : 'none' }}>`.

2. **Hero title:** fixed div, `left-0 right-0 z-20 w-full px-4`, `style={{ top: '126px' }}`, fades in via `transition-all duration-1000` toggling `opacity-100 translate-y-0` vs `opacity-0 translate-y-6` based on `mounted`. Inside: `<h1 className="hero-title select-none">MicroVisuals</h1>`.

3. **Nav:** `<nav className="fixed top-5 left-1/2 -translate-x-1/2 z-50 whitespace-nowrap">` containing a `liquid-glass flex items-center gap-6 rounded px-4 py-2.5` pill:
   - `<LogoMark />`
   - `<div className="flex items-center gap-5">` of `NAV_LINKS` as `<a>` with classes `text-sm font-body font-light text-white/70 hover:text-white transition-colors duration-200`.
   - Right cluster `flex items-center gap-3 ml-4`: "Sign in" link (same style), then "Try it free" with `liquid-glass-strong text-sm font-body font-medium text-white rounded px-4 py-1.5 transition-all duration-200 hover:scale-[1.04] hover:shadow-[0_0_16px_2px_rgba(255,255,255,0.12)] active:scale-[0.97]`.

4. **Bottom row:** fixed, `bottom-12 left-0 right-0 px-10 flex items-end justify-between z-20`, fade-in with `transition-all duration-1000 delay-300`.
   - Left `<p>`: `text-sm font-body font-light text-white/75 max-w-[220px] leading-relaxed`, text: "Forma's AI understands context, composition, and style like a creative director would."
   - Center absolute `left-1/2 -translate-x-1/2 bottom-0 flex items-center gap-3` with two buttons:
     - Primary: `group relative bg-white text-black text-sm font-body font-medium rounded px-6 py-3 overflow-hidden active:scale-[0.97] transition-all duration-200 shadow-[0_0_0_0_rgba(255,255,255,0)] hover:shadow-[0_0_24px_4px_rgba(255,255,255,0.25)] hover:scale-[1.03]`. Contents: `<span className="relative z-10">Start generating</span>` + overlay `<span className="absolute inset-0 bg-gradient-to-b from-white to-white/85 opacity-0 group-hover:opacity-100 transition-opacity duration-200" />`.
     - Secondary: `liquid-glass group text-white text-sm font-body font-medium rounded px-6 py-3 active:scale-[0.97] transition-all duration-200 hover:scale-[1.03] hover:shadow-[inset_0_1px_1px_rgba(255,255,255,0.2),0_0_20px_2px_rgba(255,255,255,0.07)]` — label "See templates".
   - Right `<p>`: same classes as left plus `text-right`, text: "Describe what you see in your head — get images that actually match."

### Notes
- Tailwind default border-radius is overridden to `9999px` (full pill) — every `rounded` in the markup produces pill corners.
- Do NOT use `video.currentTime` to reverse — the boomerang uses the captured `frames[]` array only.
- The video element stays mounted (hidden once `framesReady`) so the canvas keeps drawing snapshots.

## Waitlist Hero — Hero [sites/waitlist-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(74).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/waitlist-hero.webp

Build a full-viewport dark hero section as a single React + TypeScript + Tailwind CSS page (Vite). Use the Inter font family. No purple/indigo hues.

**Layout structure (top to bottom, full viewport, no scroll):**

1. **Outer page wrapper** — `w-screen h-screen overflow-hidden flex flex-col`, background `#0E1114` with a subtle dotted pattern: `radial-gradient(circle, rgba(255,255,255,0.07) 1px, transparent 1px)` at `24px 24px`.

2. **Navbar** (outside the inner card) — `flex items-center justify-between px-7 py-7`, `shrink-0`.
   - Left: wordmark `micro` in white, `font-semibold text-2xl tracking-tight`, letter-spacing `-0.02em`.
   - Right: two buttons in a `gap-3` flex row.
     - "Login": transparent, `text-white/70 text-sm font-medium px-4 py-2 rounded-full`, hover `text-white`.
     - "Join the Waitlist": black bg `#000`, white text, `1px solid #ffffff` border, `text-sm font-semibold px-5 py-2 rounded-full`, hover `opacity-90`, active `scale-95`, 200ms transitions.

3. **Inner card** — fills remaining height, `mx-2 mb-2`, `bg-#030404`, `rounded-[32px]`, `overflow-hidden`, `relative`.

   Inside the card:

   **a) Three-panel video section** (`flex-1 flex gap-2 p-2 lg:p-5 min-h-0`):
   - 3 equal-width cards (`flex-1`), each `relative overflow-hidden rounded-[22px]`. Cards 2 and 3 hidden on small screens (`hidden sm:block`).
   - Each card has a `<canvas>` absolutely covering it (`absolute inset-0 w-full h-full`).
   - A single hidden `<video>` element is the source of truth for all canvases, positioned offscreen (`position:absolute; width:1; height:1; opacity:0; left:-9999; top:-9999`), `muted playsInline preload="auto"`.
   - Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_071134_9cc2f2d8-a599-4a73-8c89-6eb4af170352.mp4` — but at build time, download this file and host it locally at `/public/boomerang.mp4` and reference it as `/boomerang.mp4`. Streaming directly from CloudFront and using `currentTime` seek-based boomerang causes lag.

   **Boomerang playback logic (critical — this is the non-obvious part):**
   - Do NOT seek via `video.currentTime` to reverse — it lags badly.
   - On mount, play the video forward once with `video.play()`. Use `requestVideoFrameCallback` (fallback to `requestAnimationFrame`) to capture every unique frame into an offscreen `<canvas>` element (scale down to max width 960 for memory). Push each captured canvas into a `frames[]` array.
   - On the `ended` event, stop capturing and store `frames` in React state.
   - Once frames are ready, run a `requestAnimationFrame` render loop at 30 FPS (1000/30 ms interval) that advances an index through `frames` with a direction variable: when index hits `frames.length - 1` flip to `-1`, when it hits `0` flip to `+1`. That's the boomerang.

   **Canvas slicing logic (one video rendered as three synced slices):**
   - Each frame of animation, for every visible canvas:
     - Resize canvas backing store to its `clientWidth`/`clientHeight` if mismatched.
     - Treat the N visible cards as one continuous `cover`-fitted surface (total width = `cw * n`, height = `ch`).
     - Compute source rect `sx, sy, sw, sh` from the frame that maintains `cover` behavior given that combined display aspect.
     - Slice width = `sw / n`; slice X = `sx + sliceW * i` for card `i`.
     - `ctx.drawImage(frame, sliceX, sy, sliceW, sh, 0, 0, cw, ch)`.

   **b) Bottom fade gradient** — absolutely positioned in the card, `bottom-0 left-0 right-0`, `height: 260px`, `z-10`, `pointer-events-none`, `background: linear-gradient(to top, rgba(3,4,4,0.88) 0%, rgba(3,4,4,0.50) 45%, transparent 100%)`.

   **c) Hero text + CTA row** — absolutely at bottom of card, `p-6 md:p-8 pb-10 md:pb-14`, `flex flex-col md:flex-row md:items-end md:justify-between gap-4 md:gap-0`, `z-20`, `pointer-events-none` (re-enable on interactive children).
   - Left column (`pointer-events-auto`):
     - Paragraph: "An all-in-one tool for email, CRM, project management and more that automatically organizes itself." — `text-white/70 text-sm leading-relaxed max-w-[280px]`.
     - Button "Join the Waitlist" — `self-start px-6 py-2.5 rounded-full text-sm font-semibold`, `bg-#ffffff text-#030404`, hover `opacity-90`, active `scale-95`.
   - Right column (`md:items-end`):
     - `<h1>`: "Organized." — `text-[clamp(52px,10vw,110px)]`, `font-weight: 600`, `line-height: 1.0`, `letter-spacing: -0.03em`, white, right-aligned on md+.
     - Italic subtitle: "So you don't have to be." — `text-white/60 text-base italic tracking-wide`.

   **d) Middle card overlay (card 2 only):**
   - Centered pill-shaped image frame, `width: 130px`, `height: 225px`, `border-radius: 999px`, `overflow: hidden`, `box-shadow: 0 0 0 1.5px rgba(255,255,255,0.10)`.
   - Inside it, an `<img>` filling the frame (`objectFit: cover`, `objectPosition: center`) with src:
     `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260507_181851_f7a6e930-087d-4ce3-978d-f982e804b7df.png&w=1280&q=85`.

   **e) Glowing Orbs** (decorative) — a reusable `<Orb>` component: `absolute pointer-events-none z-10`, `border-radius: 50%`, `background: radial-gradient(circle, {color} 0%, transparent 70%)`, `filter: blur(20px)`, `mix-blend-mode: screen`.
   - Card 1: `top:14% left:16% width:100 height:100`, color `rgba(255,255,255,0.70)`.
   - Card 2: `top:8% left:50% translateX(-50%) width:72 height:72`, color `rgba(200,215,255,0.55)`.
   - Card 3: `top:20% right:10% width:110 height:110`, color `rgba(185,210,235,0.55)`.

**Stack:** React 18, TypeScript, Tailwind, Vite, lucide-react available (not used here). Single `src/App.tsx`. All transitions 200ms. No external UI libs. Match every color, radius, and pixel value above exactly.

## Wellbeing OS — Hero [sites/wellbeing-os]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/Wellbeing%20OS.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/wellbeing-os.mp4

Create a fullscreen hero section for a SaaS product called "flowpath" using React, Tailwind CSS, and Lucide React icons. The section should be a single `<section>` filling the viewport (`h-screen w-full overflow-hidden`).

**Background:**
- A looping, muted, autoplaying `<video>` element covering the full section with `object-cover`. Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260703_053131_1ec3dd1c-d627-44fb-ab20-6e1fce41b0d5.mp4`
- A subtle dark overlay on top of the video: `bg-black/10`

**Font:**
- Use "Helvetica Now Text" as the primary font, loaded from: `https://db.onlinewebfonts.com/c/08e020de1811ec4489f82d1247a42c09?family=Helvetica+Now+Text`
- Fallback stack: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`
- Applied globally via `* { font-family: ... }` in CSS

**Navigation (top, not fixed/sticky):**
- Full-width with responsive horizontal padding (`px-5 sm:px-6 md:px-12 lg:px-16`) and vertical padding (`py-4 sm:py-5`)
- Logo: An inline SVG diamond shape (28x28) with two overlapping diamond paths at 0.9 and 0.5 opacity, followed by the text "flowpath" in white, `text-lg sm:text-xl font-medium tracking-tight`
- Desktop nav (hidden on mobile): horizontal flex with items "Product" (dropdown: Connections, Workflows, Insights), "Solutions" (dropdown: Guides, Use cases, API reference), "About" (dropdown: Our story, Open roles, Reach us), "Plans" (no dropdown)
- Nav buttons: `text-white/90 hover:text-white text-sm font-medium`, with a `ChevronDown` icon (3.5x3.5) that rotates 180 degrees when dropdown is open
- Dropdowns open on hover (onMouseEnter/onMouseLeave), positioned `absolute top-full left-0`, using a custom `.liquid-glass` class, `rounded-xl py-3 px-2 min-w-[160px] shadow-xl`. Dropdown items: `text-white/80 hover:text-white text-sm rounded-lg hover:bg-white/5`
- Desktop CTA: "Log in" link (`text-white/90 hover:text-white text-sm font-medium`) and "Try it free" button using `.liquid-glass rounded-full px-5 py-2 text-white text-sm font-medium`
- Mobile menu button: animated toggle between `Menu` and `X` icons with rotation/scale/opacity transitions (duration-300)
- Mobile menu: absolutely positioned below nav, slides in with `cubic-bezier(0.16,1,0.3,1)` easing over 400ms. Background: `bg-[#2C221C]/95 backdrop-blur-xl rounded-2xl p-6`. Shows all nav items with sub-items indented, plus a bordered footer with Log in and Try it free

**Hero Content (below nav, top-aligned, not vertically centered):**
- Container: `flex-1 flex items-start justify-center` with `pt-16 sm:pt-20 md:pt-24` for spacing from the nav
- Text wrapper: `text-center max-w-3xl`
- Heading `<h1>`: `text-white text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl leading-[1.05] tracking-[-0.02em]`
  - Content (with line breaks):
    ```
    Bridge the
    gaps. <span class="text-white/60">Ditch the</span>
    <span class="text-white/60">grindwork.</span>
    ```
- Subheading `<p>`: `text-white/80 text-sm sm:text-base md:text-lg leading-relaxed max-w-md mx-auto mt-6 sm:mt-8`
  - Text: "Flowpath unifies your complete wellness tools, so your crew spends less energy plugging gaps and more on real progress."
- Two CTA buttons side by side (`flex flex-wrap items-center justify-center gap-3 sm:gap-4 mt-6 sm:mt-8`):
  1. "Begin your journey" - solid white button: `px-5 sm:px-6 py-2.5 sm:py-3 bg-white text-gray-900 text-sm font-semibold rounded-full hover:bg-white/90`
  2. "See it live" - glass button: `px-5 sm:px-6 py-2.5 sm:py-3 liquid-glass rounded-full text-white text-sm font-semibold hover:bg-white/10`

**Custom CSS (`.liquid-glass` class):**
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

**Additional CSS utilities:**
```css
@keyframes dropdown-in {
  from { opacity: 0; transform: translateY(-4px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.animate-dropdown { animation: dropdown-in 0.2s ease-out; }
.duration-400 { transition-duration: 400ms; }
```

**Important notes:**
- Dropdown elements need `!absolute` (Tailwind important modifier) to override the `position: relative` from `.liquid-glass`
- The entire section is fully responsive with breakpoints at sm, md, lg, xl
- No external UI libraries beyond Lucide React for icons
- Tailwind config is default with no extensions

## Wellness Balance — Hero [sites/wellness-balance]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(11).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/wellness-balance.webp

Create a single-page hero landing for a wellness/supplements brand called "TerraElix" using React + Tailwind CSS + Lucide React icons. The page is a full-viewport hero with a background image, navbar, headline with word-by-word reveal animations, CTA section, and a 3-panel footer strip. It must be fully responsive (mobile, tablet, desktop).

---

### Fonts

Import from Google Fonts:
- **DM Sans** (weights 400, 500) -- used for brand name, nav links, headline, panel 1 text
- **Inter** (weights 400, 500) -- used for buttons, body text, panel 2/3 text

---

### Background

Full-screen background image covering the entire viewport:
```
url: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_110248_b62f758d-f68c-4045-a7b4-91771d6d0a0f.png&w=1280&q=85
background-size: cover; background-position: center; background-repeat: no-repeat;
```

---

### Layout Structure

```
<div> (min-h-screen, flex flex-col, relative, overflow-hidden)
  <nav> -- navbar
  <section> -- hero content (flex-1, flex col, justify-center)
  <div> -- mobile/tablet product image (visible below lg)
  <div> -- 3-panel grid footer
  <img> -- desktop floating product image (absolute, hidden below lg)
</div>
```

---

### Navbar

- **Left:** Brand name "TerraElix" -- white, DM Sans 500, 30px, letter-spacing -0.05em
- **Center (desktop only, hidden on mobile):** Nav links "About", "Products", "Promotions", "Contact" -- DM Sans 500, 18px, text-white/90, gap 10 (lg)
- **Right:** Row of icon buttons + avatar + mobile menu toggle
  - Search icon (Lucide `Search`, size 20, strokeWidth 1.5)
  - Shopping bag icon (Lucide `ShoppingBag`, size 20, strokeWidth 1.5)
  - Return icon (Lucide `CornerUpLeft`, size 20, strokeWidth 1.5)
  - Round avatar image (w-8 h-8, lg:w-10 lg:h-10, rounded-full, object-cover):
    ```
    https://polo-pecan-73837341.figma.site/_assets/v11/ca8093996e970200cbcf8bde8744175e52da5a79.png
    ```
  - Hamburger menu button (md:hidden, Lucide `Menu` / `X` toggle)

- **Mobile overlay menu:** fixed inset-0 bg-black/90 z-30 with centered nav links (text-2xl, white)

Padding: px-5 sm:px-8 lg:px-10, py-4 lg:py-5

---

### Hero Headline

Font: DM Sans, weight 400, letter-spacing -0.05em

Responsive sizes:
- Base: 48px/50px line-height
- sm: 80px/72px
- md: 110px/95px
- lg: 130px/110px
- xl: 155px/125px

Text layout (3 lines):
```
Line 1: "The" (white) "Power" (white) "of" (white/45 -- dimmed)
Line 2: "Nature" (dimmed) "in" (dimmed) "Every" (white)
Line 3: "Capsule" (white) + inline image
```

Each word is wrapped in a container with overflow-hidden, and the inner span animates with `wordReveal` (translateY 100% + blur to visible). Staggered delays: 0.3s, 0.4s, 0.5s, 0.6s, 0.7s, 0.8s, 0.9s.

**Inline image** after "Capsule" (hidden on mobile, sm:inline-block, align-middle, ml-2 lg:ml-4):
```
https://polo-pecan-73837341.figma.site/_assets/v11/6a7de4fbe9c9e2315040607320a9ff5e93117bf4.png
height: clamp(60px, 10vw, 160px); width: auto;
```

---

### CTA Section

Below the headline, mt-8 sm:mt-12 lg:mt-[75px]. Flex row on sm+, column on mobile. Gap: 5 (mobile), 8 (sm), 50px (lg).

- **Button:** "Explore Now" + ArrowUpRight icon. bg-black text-white rounded-md. Sizes: w-full sm:w-[240px] md:w-[280px] lg:w-[310px], h-14 sm:h-16 lg:h-[72px]. Font: Inter 500, responsive text (base to 2xl), letter-spacing -0.03em.
- **Paragraph:** "Discover our new plant-based supplements for daily balance and clean energy." -- white, max-w-[310px], Inter 400, text-sm sm:text-base lg:text-lg, line-height 1.45, letter-spacing -0.03em.

---

### Mobile/Tablet Product Image (lg:hidden)

Visible below lg breakpoint. Oversized, bleeding off edges:
```
https://polo-pecan-73837341.figma.site/_assets/v11/50ad042b3cd48a2e120ea3ba17c8cfeaf3cc334c.png
w-[180%] sm:w-[151%] max-w-[1296px], object-contain, mx-auto, drop-shadow-2xl
margin-bottom: -180px sm:-220px (overlaps panels below)
```

---

### Bottom 3-Panel Grid

`grid grid-cols-1 md:grid-cols-[2fr_1fr_2fr]`, relative z-10.

### Panel 1 (bg-[#ECEDEC])
- Text: "Start your personalized path to natural balance" -- DM Sans 400, text-2xl sm:text-[28px] lg:text-[35px], leading-[1.1], letter-spacing -0.05em, max-w-[350px]
- Link: "Personal Assessment" -- underline, Inter 400, text-base lg:text-lg, letter-spacing -0.03em
- Decorative image (absolute right-0 bottom-0, h-full, mix-blend-multiply):
  ```
  https://polo-pecan-73837341.figma.site/_assets/v11/6736cbe6e26afa2cd7c04a91892a79f7640785b5.png
  ```

### Panel 2 (bg-[#FEFDF9]) -- Auto-rotating card carousel
4 cards cycling every 3500ms with fade/slide transition:
1. FlaskConical icon, bg-black circle: "Experience our newly enhanced natural formula"
2. Leaf icon, bg-emerald-800 circle: "Pure organic ingredients sourced sustainably"
3. Droplets icon, bg-cyan-800 circle: "Advanced bioavailability for maximum absorption"
4. Sun icon, bg-amber-700 circle: "Clinically tested for daily energy & vitality"

Each card: icon in a 40px (sm:48px) round colored circle + text (Inter 400, text-sm sm:text-base lg:text-lg, text-black/80, line-height 1.2, letter-spacing -0.03em).

Active card: opacity-100 translate-y-0. Inactive: opacity-0 translate-y-4 absolute.

Bottom dots: 4 thin bars (h-0.5, flex-1, rounded-full). Active: bg-black. Inactive: bg-black/20.

### Panel 3 (bg-black)
- Left: Product image (w-[120px] h-[82px] sm:w-[160px] h-[110px] lg:w-[208px] h-[142px]):
  ```
  https://polo-pecan-73837341.figma.site/_assets/v11/30e8f38d1f993c357a3be2721557fc899d5640fc.png
  ```
- Right: "+14K" (white, Inter 400, text-2xl sm:text-3xl lg:text-[35px], letter-spacing -0.05em) + "People have already optimized their wellness" (text-white/60, Inter 400, text-sm sm:text-base lg:text-lg, line-height 1.2)

---

### Desktop Floating Product (lg+ only)

Same image as mobile product, but absolutely positioned for desktop:
```
https://polo-pecan-73837341.figma.site/_assets/v11/50ad042b3cd48a2e120ea3ba17c8cfeaf3cc334c.png
position: absolute; z-0; hidden lg:block;
width: clamp(600px, 80vw, 1412px); height: auto;
bottom: -10%; right: clamp(-400px, -20vw, -100px);
```

---

### Animations (CSS keyframes)

```css
@keyframes fadeUp {
  from { opacity: 0; transform: translateY(30px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes slideInLeft {
  from { opacity: 0; transform: translateX(-40px); }
  to { opacity: 1; transform: translateX(0); }
}
@keyframes slideInRight {
  from { opacity: 0; transform: translateX(40px); }
  to { opacity: 1; transform: translateX(0); }
}
@keyframes scaleIn {
  from { opacity: 0; transform: scale(0.9); }
  to { opacity: 1; transform: scale(1); }
}
@keyframes wordReveal {
  from { opacity: 0; transform: translateY(100%); filter: blur(4px); }
  to { opacity: 1; transform: translateY(0); filter: blur(0px); }
}
```

All use `cubic-bezier(0.16, 1, 0.3, 1)` easing with `both` fill mode.

**Classes and their animations:**
- `.animate-fade-up` -- fadeUp 0.8s
- `.animate-fade-in` -- fadeIn 0.7s
- `.animate-slide-left` -- slideInLeft 0.8s
- `.animate-slide-right` -- slideInRight 0.8s
- `.animate-scale-in` -- scaleIn 1s
- `.animate-word-reveal > span` -- wordReveal 0.7s

**Delay classes:** .delay-200 through .delay-1100 (increments of 0.1s)

**Animation assignments:**
- Navbar container: animate-fade-in
- Brand name: animate-slide-left delay-200
- Nav links: animate-fade-in delay-400
- Right icons: animate-slide-right delay-300
- CTA row: animate-fade-up delay-600
- Desktop product image: animate-scale-in delay-700
- Mobile product image: animate-scale-in delay-800
- Panel 1: animate-fade-up delay-900
- Panel 2: animate-fade-up delay-1000
- Panel 3: animate-fade-up delay-1100
- Inline capsule image: animate-scale-in delay-1000

## Wellness Hero — Hero [sites/wellness-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(32).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/wellness-hero.webp

Build a full-screen hero section landing page for "Aurai" - an always-on AI wellness companion. The page is a single viewport-height section with a looping background video and overlaid content.

### Video Background

- Full-screen `<video>` element with `autoPlay`, `loop`, `muted`, `playsInline` attributes
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260618_174853_aac61aa2-0f3f-4cf1-bc78-7f657dd11164.mp4`
- Video covers entire viewport with `object-cover`
- Focal point positioning:
  - Mobile (default): `object-position: 80% center`
  - Tablet (md breakpoint): `object-position: right center`
  - Desktop (lg breakpoint): `object-position: center center`

### Fonts

- **Askan Light** loaded from: `https://db.onlinewebfonts.com/c/304a6edcec9f8858eeaafc2ac18243f4?family=Askan+Light` - used for the brand name and heading
- **Inter** (weights 300, 400, 500, 600) from Google Fonts - used as the body/UI font
- Tailwind config extends fontFamily with `askan: ['"Askan Light"', 'serif']` and `inter: ['Inter', 'sans-serif']`

### Layout Structure

The content is layered on top of the video using `absolute inset-0 z-10` with a flex column layout. Padding: `px-4 sm:px-10 lg:px-12 py-4 sm:py-8`.

### Navigation (Top)

A `<nav>` with `flex items-center justify-between`:

**Left nav pill (glassmorphism):**
- `bg-black/20 backdrop-blur-md rounded-2xl border border-white/10`
- Padding: `px-4 py-2.5 sm:px-6 sm:py-4`
- Contains:
  - A custom SVG logo (4-petal pinwheel shape, `w-5 h-5 sm:w-7 sm:h-7`, white)
  - Brand text "Aurai" in `font-askan text-white text-base sm:text-xl tracking-wide`
  - Hamburger menu icon (lucide-react `Menu`/`X`) with left margin: `ml-4 sm:ml-32 md:ml-64 lg:ml-96`

**Right button (desktop only):**
- `hidden sm:block bg-white text-gray-900 font-medium text-sm px-6 py-3 rounded-full`
- Text: "Join the list"

### Mobile Menu (shown on toggle)

- `sm:hidden`, positioned `absolute top-[4.5rem] left-4 right-4`
- `bg-black/30 backdrop-blur-xl rounded-2xl p-5 border border-white/10`
- Contains links: "Story", "Benefits", "Connect" (white text) and a full-width "Join the list" button

### Main Content (Bottom-aligned)

On mobile: a spacer `flex-1 sm:hidden` pushes content to the bottom.

The content container: `flex flex-col sm:flex-1 sm:flex-row sm:items-end pb-4 sm:pb-12 lg:pb-16 sm:mt-auto`

**Left column:**

1. **Heading:** `font-askan text-white text-[2rem] sm:text-[3.5rem] md:text-[4.5rem] lg:text-[5.5rem] leading-[1.05] tracking-tight max-w-[700px]`
   - Text: "Your calm is always within."

2. **Subtitle:** `text-white/70 text-xs sm:text-base md:text-lg max-w-[520px] leading-relaxed`
   - Text: "Aurai is your always-on wellness companion. Built by leading therapists, it brings you the care and clarity right when you need it."

3. **Email form:** A rounded pill input with inline submit button
   - Container: `bg-black/30 backdrop-blur-md rounded-full border border-white/10`
   - Input: transparent background, white text, placeholder "Your email address", `px-4 sm:px-6 py-3 sm:py-4 text-sm`
   - Submit button (absolute right-1.5): `bg-white text-gray-900 text-xs sm:text-sm font-medium px-3 sm:px-6 py-2 sm:py-3 rounded-full`
   - Text: "Join the list"
   - On submit: shows alert with entered email

4. **Feature pills (mobile only):** `flex sm:hidden flex-wrap gap-2 mt-2`
   - Three pills with `bg-black/30 backdrop-blur-md text-white text-xs px-3 py-1.5 rounded-full border border-white/10`
   - Labels: "Smart Therapy", "Real-time Healing", "Insights into outcomes"

**Right column (desktop only):**
- `hidden sm:flex flex-col items-end gap-2 self-end`
- Same three feature pills as mobile but with `text-xs sm:text-sm px-4 py-2`

### Custom SVG Logo

A pinwheel/4-quadrant shape with this path:
```
M 228 0 C 172.772 0 128 44.772 128 100 L 128 0 L 0 0 L 0 28 C 0 83.228 44.772 128 100 128 L 0 128 L 0 256 L 28 256 C 83.228 256 128 211.228 128 156 L 128 256 L 256 256 L 256 228 C 256 172.772 211.228 128 156 128 L 256 128 L 256 0 Z
```
ViewBox: `0 0 256 256`, fill: `currentColor`

### Global CSS

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: 'Inter', sans-serif; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
```

### Key Design Principles

- No dark overlay on the video - content relies on glassmorphism pills and text contrast
- All interactive glass elements use `bg-black/20` or `bg-black/30` with `backdrop-blur-md` or `backdrop-blur-xl`
- Borders are `border-white/10` throughout
- White text with `/70` opacity for secondary text
- Rounded-full for buttons and inputs, rounded-2xl for containers
- Page title: "Aurai - Always-On Wellness Companion"

## Aethera Studio — Hero Section [sites/aethera-hero]

- Preview: https://motionsites.ai/assets/hero-aethera-preview-DknSlcTa.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/aethera-hero.gif

Prompt: Cinematic Hero Section with Looping Video Background

Create a fullscreen single-page hero section using React + Vite + Tailwind CSS + TypeScript with the following specifications:

Fonts:
Display text (headings, logo): Instrument Serif
Body text (navigation, descriptions): Inter
Import both fonts in /src/styles/fonts.css

Video Background:
URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_083109_283f3553-e28f-428b-a723-d639c617eb2b.mp4
Position: top: '300px' with inset: 'auto 0 0 0'
Implement custom fade-in/fade-out loop logic using React useEffect and useRef:
Use requestAnimationFrame to continuously monitor currentTime and duration
Fade in over 0.5s at the start (opacity 0 to 1)
Fade out over 0.5s before the end (opacity 1 to 0)
On ended event: set opacity to 0, wait 100ms, reset currentTime = 0, then play() again
This creates a seamless manual loop with smooth fade transitions
Add gradient overlays: absolute inset-0 bg-gradient-to-b from-background via-transparent to-background positioned over the video

Navigation Bar:
Logo: "Aethera®" (with registered trademark symbol as superscript)
Logo styling: text-3xl, tracking-tight, Instrument Serif, color #000000
Menu items: Home (color #000000), Studio, About, Journal, Reach Us (all others #6F6F6F)
Menu items: text-sm with transition-colors
CTA button: "Begin Journey", rounded-full, px-6 py-2.5, text-sm, black background (#000000), white text, hover scale 1.03
Layout: flex justify-between, px-8 py-6, max-w-7xl mx-auto

Hero Section:
Positioning: paddingTop: 'calc(8rem - 75px)', pb-40
Layout: centered (flex flex-col items-center justify-center text-center), px-6
Headline:
Text: "Beyond silence, we build the eternal."
Styling: text-5xl sm:text-7xl md:text-8xl, max-w-7xl, font-normal
Font: Instrument Serif
Line height: 0.95
Letter spacing: -2.46px
Color: #000000 for main text, #6F6F6F for italic emphasized words ("silence," and "the eternal.")
Animation: animate-fade-rise

Description:
Text: "Building platforms for brilliant minds, fearless makers, and thoughtful souls. Through the noise, we craft digital havens for deep work and pure flows."
Styling: text-base sm:text-lg, max-w-2xl, mt-8, leading-relaxed
Color: #6F6F6F
Animation: animate-fade-rise-delay

Hero CTA Button:
Text: "Begin Journey"
Styling: rounded-full, px-14 py-5, text-base, mt-12
Colors: black background (#000000), white text (#FFFFFF)
Hover: scale 1.03
Animation: animate-fade-rise-delay-2

Colors:
Background: white (#FFFFFF)
Headlines/logos/buttons: black (#000000)
Descriptions/menu items: gray (#6F6F6F)
Button text: white (#FFFFFF)

Animations (in /src/styles/theme.css):
fade-rise: opacity 0 to 1, translateY 20px to 0, duration 0.8s, ease-out
fade-rise-delay: same as fade-rise but with 0.2s delay
fade-rise-delay-2: same as fade-rise but with 0.4s delay

Layout Structure:
Container: relative min-h-screen w-full overflow-hidden
Background video layer (z-0)
Gradient overlay on video
Navigation bar (z-10)
Hero section (z-10)
All elements should be responsive and maintain the glassmorphic aesthetic with the specified padding, positioning, and smooth animations.

## Aetheris Voyage — Hero Section [sites/aetheris-voyage-hero]

- Preview: https://motionsites.ai/assets/hero-aetheris-voyage-preview-BGJn1z4t.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/aetheris-voyage-hero.gif

Build Prompt: Cinematic Space-Travel Landing Page
Build a single-page landing site with two full-height sections (Hero + Capabilities), both using looping background videos with custom JS crossfade, a shared liquid-glass design system, and Framer Motion entrance animations.

Tech stack (pinned, CDN-only)
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://unpkg.com/react@18.3.1/umd/react.development.js" integrity="sha384-hD6/rw4ppMLGNu3tX5cjIb+uRZ7UkRJ6BPkLpg4hAu/6onKUg4lLsHAs9EBPT82L" crossorigin="anonymous"></script>
<script src="https://unpkg.com/react-dom@18.3.1/umd/react-dom.development.js" integrity="sha384-u6aeetuaXnQ38mYT8rp6sbXaQe3NL9t+IBXmnYxwkUI2Hw4bsp2Wvmx4yRQF1uAm" crossorigin="anonymous"></script>
<script src="https://unpkg.com/@babel/standalone@7.29.0/babel.min.js" integrity="sha384-m08KidiNqLdpJqLq95G/LEi8Qvjl/xUYll3QILypMoQ65QorJ9Lvtp2RXYGBFj1y" crossorigin="anonymous"></script>
<script src="https://unpkg.com/framer-motion@11.11.17/dist/framer-motion.js"></script>
<script>window.Motion = window.FramerMotion;</script>
Body is bg: #000. Page is a React app mounted on #root, all components are <script type="text/babel"> files exporting via window.X = X.

Fonts
Google Fonts:

family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600
Tailwind config adds:

font-heading → 'Instrument Serif', serif (always italic in use)
font-body → 'Barlow', sans-serif
Default border radius override: DEFAULT: "9999px" (so bare rounded → pill).

Liquid-glass utilities (exact CSS, in a <style> block)
Two variants — .liquid-glass (subtle, for nav/chips/cards) and .liquid-glass-strong (heavier blur, for primary CTA):

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
.liquid-glass-strong { /* same but: */
backdrop-filter: blur(50px);
box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15);
}
.liquid-glass-strong::before { /* same but 0.5 / 0.2 / 0 / 0 / 0.2 / 0.5 stops */ }
FadingVideo component (custom JS crossfade, no CSS transitions)
Wraps a <video autoPlay muted playsInline preload="auto"> starting at opacity: 0. Behavior:

FADE_MS = 500, FADE_OUT_LEAD = 0.55 seconds.
fadeTo(target, duration) uses requestAnimationFrame; reads current opacity from http://video.style.opacity so each new fade resumes from wherever the last one left off. Each call calls cancelAnimationFrame on the previous rAF id before starting.
On loadeddata: set opacity 0, play(), fadeTo(1).
On timeupdate: if fadingOutRef not set and duration - currentTime <= 0.55 and > 0, flip the ref and fadeTo(0).
On ended: set opacity 0; after setTimeout(100ms) reset currentTime = 0, play(), clear fadingOutRef, fadeTo(1).
loop attribute is OFF (we implement looping manually via ended).
Cleanup on unmount: cancel rAF, remove listeners.
Section 1 — Hero (full viewport, black bg)
Background video (120% width/height, top-aligned, centered horizontally — focal point is the top of frame):

src: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260418_080021_d598092b-c4c2-4e53-8e46-94cf9064cd50.mp4
class: absolute left-1/2 top-0 -translate-x-1/2 object-cover object-top z-0
style: { width: "120%", height: "120%" }
No overlay. z-10 layer holds: Navbar → Hero content (flex-1, centered) → Partners.

Navbar (fixed top-4, px-8 / lg:px-16, z-50)
Left: 48×48 liquid-glass circle with italic serif lowercase "a" (Instrument Serif).
Center (desktop only): liquid-glass pill, px-1.5 py-1.5, holding 5 text links — Home, Voyages, Worlds, Innovation, Plan Launch — each px-3 py-2 text-sm font-medium text-white/90 font-body. Followed by a white pill button Claim a Spot + ArrowUpRight icon (bg-white text-black, whitespace-nowrap).
Right: 48×48 invisible spacer to balance logo.
Hero content (centered, pt-24 px-4)
All animated with Framer Motion, initial: {filter: blur(10px), opacity: 0, y: 20}, easeOut.

Badge (delay 0.4s): liquid-glass rounded-full pill. Contains white pill chip "New" (bg-white text-black px-3 py-1 text-xs font-semibold) + text "Maiden Crewed Voyage to Mars Arrives 2026" (text-sm text-white/90, pr-3).
Headline — BlurText component (word-by-word animation, see below). Text: "Venture Past Our Sky Across the Universe". Classes: text-6xl md:text-7xl lg:text-[5.5rem] font-heading italic text-white leading-[0.8] max-w-2xl justify-center tracking-[-4px].
Subheading (delay 0.8s, mt-4 text-sm md:text-base text-white max-w-2xl font-body font-light leading-tight): "Discover the universe in ways once unimaginable. Our pioneering vessels and breakthrough engineering bring deep-space exploration within reach—secure and extraordinary."
CTAs (delay 1.1s, flex items-center gap-6 mt-6):
Primary: liquid-glass-strong rounded-full px-5 py-2.5 text-sm font-medium text-white with "Start Your Voyage" + ArrowUpRight (h-5 w-5).
Secondary: bare text link, "View Liftoff" + Play icon (h-4 w-4, filled).
Stats row (delay 1.3s, flex items-stretch gap-4 mt-8): two liquid-glass cards, p-5 w-[220px] rounded-[1.25rem], each:
Top: white 28×28 outline SVG icon (clock for card 1, globe for card 2).
Bottom: large number in Instrument Serif italic white (text-4xl tracking-[-1px] leading-none): "34.5 Min" / "2.8B+". Label below (text-xs text-white font-body font-light mt-2): "Average Videos Watch Time" / "Users Across the Globe".
Partners (bottom of hero, delay 1.4s)
flex flex-col items-center gap-4 pb-8:

liquid-glass rounded-full chip (px-3.5 py-1 text-xs font-medium text-white): "Collaborating with top aerospace pioneers globally".
Row of 5 names in Instrument Serif italic white, text-2xl md:text-3xl tracking-tight, gap-12/md:gap-16: Aeon · Vela · Apex · Orbit · Zeno.
BlurText component (word-by-word blur-in)
IntersectionObserver triggers on 10% visibility. Splits text by spaces. Each word is a motion.span with:

initial: {filter: 'blur(10px)', opacity: 0, y: 50}
3-step keyframes to {filter: 'blur(5px)', opacity: 0.5, y: -5} → {filter: 'blur(0px)', opacity: 1, y: 0}
duration: 0.7 (stepDuration 0.35 × 2), times: [0, 0.5, 1], ease: easeOut
Stagger: delay = (i * 100) / 1000 seconds
display: inline-block, marginRight: 0.28em (not non-breaking-space — letter-spacing -4px eats nbsp).
Parent <p> is display: flex; flexWrap: wrap; justifyContent: center; rowGap: 0.1em.
Section 2 — Capabilities (min-h-screen, black bg)
Background video (full-bleed, no 120% scale):

src: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260418_094631_d30ab262-45ee-4b7d-99f3-5d5848c8ef13.mp4
class: absolute inset-0 w-full h-full object-cover z-0
Same FadingVideo treatment. No overlay.

Content (relative z-10 px-8 md:px-16 lg:px-20 pt-24 pb-10 flex flex-col min-h-screen):

Header (mb-auto):

Kicker: text-sm font-body text-white/80 mb-6 → // Capabilities
Heading: font-heading italic text-white text-6xl md:text-7xl lg:text-[6rem] leading-[0.9] tracking-[-3px]:
Production
evolved
(two lines, <br/> between).
Three cards (grid grid-cols-1 md:grid-cols-3 gap-6 mt-16): each is liquid-glass rounded-[1.25rem] p-6 min-h-[360px] flex flex-col.

Top row of each card (flex items-start justify-between gap-4):

Left: 44×44 nested liquid-glass square (rounded-[0.75rem]) with a white Material Icons SVG (fill currentColor, h-6 w-6 text-white). Use random Material icons — these three used:
AI Scenery: image icon — path M5 21q-.825 0-1.412-.587T3 19V5q0-.825.588-1.412T5 3h14q.825 0 1.413.588T21 5v14q0 .825-.587 1.413T19 21H5Zm1-4h12l-3.75-5-3 4L9 13l-3 4Z
Batch Production: movie icon — path M4 6.47 5.76 10H20v8H4V6.47M22 4h-4l2 4h-3l-2-4h-2l2 4h-3l-2-4H8l2 4H7L5 4H4c-1.1 0-1.99.89-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V4Z
Smart Lighting: lightbulb icon — path M9 21c0 .55.45 1 1 1h4c.55 0 1-.45 1-1v-1H9v1Zm3-19C8.14 2 5 5.14 5 9c0 2.38 1.19 4.47 3 5.74V17c0 .55.45 1 1 1h6c.55 0 1-.45 1-1v-2.26c1.81-1.27 3-3.36 3-5.74 0-3.86-3.14-7-7-7Z
Right: flex flex-wrap justify-end gap-1.5 max-w-[70%] — 4 small liquid-glass pill tags (rounded-full px-3 py-1 text-[11px] text-white/90 font-body whitespace-nowrap):
Card 1: Natural Context · Photo Realism · Infinite Settings · Eco-Vibe
Card 2: Scale Fast · Visual Consistency · Time Saver · Ready to Post
Card 3: Ray Tracing · Physical Shadows · Studio Quality · Sunlight Sync
Middle: flex-1 spacer.

Bottom of each card (mt-6):

Title h3: font-heading italic text-white text-3xl md:text-4xl tracking-[-1px] leading-none — "AI Scenery" / "Batch Production" / "Smart Lighting"
Body p (mt-3 text-sm text-white/90 font-body font-light leading-snug max-w-[32ch]):
"AI analyzes your product to create indistinguishable natural environments — from Icelandic cliffs to misty forests."
"Style your entire product line in minutes. Create a unified visual identity for catalogues and social media without weeks of retouching."
"Automatic lighting and material adjustment. Achieve flawless integration with realistic shadows and sunlight."
Icons (inline lucide-style SVGs, currentColor stroke)
ArrowUpRight: 24×24, M7 17L17 7 + M7 7h10v10, strokeWidth 2, round caps.
Play: 24×24 filled polygon 6 4 20 12 6 20 6 4.
Notes
All text white; no green, no gradient backgrounds.
No CSS transitions on the videos — fades must be rAF-driven per the FadingVideo spec.
Videos are full-bleed with no dark overlay; contrast comes from the liquid-glass chrome.
Framer Motion dev warnings about list keys can be suppressed with a console.error filter wrapper — they're benign.
The detailed prompt above captures every element, style, animation, video URL, and font to recreate the landing page exactly.

## Asme — Hero Section [sites/asme-hero]

- Preview: https://motionsites.ai/assets/hero-asme-preview-B_nGDnTP.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/asme-hero.gif

Build a single-page hero section with a full-screen looping background video, liquid glass UI elements, and a dark cinematic aesthetic. Use React, TypeScript, Tailwind CSS, and Lucide React icons. Here are the exact specifications:

Background Video:

Full-screen muted autoplaying video covering the entire viewport, positioned absolutely with object-cover
Video source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_115001_bcdaa3b4-03de-47e7-ad63-ae3e392c32d4.mp4
The video is shifted down by 17% (translate-y-[17%]) so the top portion of the video is cropped -- the interesting content is in the lower portion of the frame
The video loops seamlessly with a custom JavaScript fade system (no CSS transitions): 500ms requestAnimationFrame-based fade-in on load/loop start, 500ms fade-out when 0.55 seconds remain before the video ends. A fadingOutRef boolean prevents re-triggering the fade-out from repeated timeUpdate events. On ended, opacity is set to 0, then after 100ms the video resets to currentTime = 0, plays, and fades back in. Each new fade cancels any running animation frame to prevent competing animations. Fades resume from the current opacity rather than snapping.
The outer container is min-h-screen bg-black with overflow-hidden

Font:

Import Google Font "Instrument Serif" (both regular and italic) via CSS @import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap')
The heading uses fontFamily: "'Instrument Serif', serif" applied via inline style

Liquid Glass CSS (.liquid-glass class):

background: rgba(255, 255, 255, 0.01) with background-blend-mode: luminosity
backdrop-filter: blur(4px) and -webkit-backdrop-filter: blur(4px)
border: none
box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1)
position: relative; overflow: hidden
A ::before pseudo-element creates the glass border effect:
position: absolute; inset: 0; border-radius: inherit; padding: 1.4px
background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)
Mask trick for border-only rendering: -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0); -webkit-mask-composite: xor; mask-composite: exclude
pointer-events: none

Layout (all inside one full-screen flex column):

Navigation bar (relative z-20, padding pl-6 pr-6 py-6):
Inner container: rounded-full px-6 py-3 flex items-center justify-between max-w-5xl mx-auto
Left side: Logo area with a Globe icon (size 24) and text "Asme" in white, font-semibold text-lg, with gap-2
Next to the logo (with gap-8): three nav links ("Features", "Pricing", "About") -- hidden on mobile, shown on md: -- styled text-white/80 hover:text-white transition-colors text-sm font-medium
Right side (gap-4): "Sign Up" as plain white text button, "Login" as a liquid-glass rounded-full px-6 py-2 button

Hero content area (relative z-10 flex-1 flex flex-col items-center justify-center px-6 py-12 text-center -translate-y-[20%]):
Heading: "Built for the curious" -- text-5xl md:text-6xl lg:text-7xl text-white mb-8 tracking-tight whitespace-nowrap with Instrument Serif font
Below the heading, a max-w-xl w-full space-y-4 container:
Email input bar: liquid-glass rounded-full pl-6 pr-2 py-2 flex items-center gap-3. Inside: a transparent email input (placeholder: "Enter your email", text-white placeholder:text-white/40 text-base) and a white circular submit button (bg-white rounded-full p-3 text-black) containing an ArrowRight icon (size 20)
Subtitle text: text-white text-sm leading-relaxed px-4 -- "Stay updated with the latest news and insights. Subscribe to our newsletter today and never miss out on exciting updates."
Manifesto button: centered, liquid-glass rounded-full px-8 py-3 text-white text-sm font-medium hover:bg-white/5 transition-colors

Social icons footer (relative z-10 flex justify-center gap-4 pb-12):
Three circular icon buttons, each liquid-glass rounded-full p-4 text-white/80 hover:text-white hover:bg-white/5 transition-all
Icons: Instagram, Twitter, Globe (all size 20) from lucide-react
Each has an aria-label

Tech stack: Vite + React 18 + TypeScript, Tailwind CSS 3, lucide-react for all icons. Default Tailwind config with no extensions. No other UI libraries.

## Automation Machines — Hero Section [sites/automation-machines-hero]

- Preview: https://motionsites.ai/assets/hero-automation-machines-preview-DlTveRIN.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/automation-machines-hero.gif

PROMPT TO RECREATE THIS HERO SECTION

Build a full-screen dark hero section for a futuristic "Automation Machines" landing page using React, Vite, Tailwind CSS v4, Motion (framer-motion), Lucide React icons, and Spline 3D. The page should be a single full-viewport section with a black background, white text, and a 3D Spline scene behind all content.

DEPENDENCIES (exact versions)

react ^19.0.0
react-dom ^19.0.0
vite ^6.2.0
@vitejs/plugin-react ^5.0.4
tailwindcss ^4.1.14 (with @tailwindcss/vite ^4.1.14 plugin)
motion ^12.23.24 (import from motion/react)
lucide-react ^0.546.0
@splinetool/react-spline ^4.1.0
@splinetool/runtime ^1.12.72

FONTS (Google Fonts)

Import this exact Google Fonts URL in your CSS:
https://fonts.googleapis.com/css2?family=Orbitron:wght@400;500;600;700;800;900&family=Space+Grotesk:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&family=Instrument+Serif:ital@1&display=swap

Font assignments via Tailwind @theme:
--font-sans: "Space Grotesk" (body text default)
--font-display: "Orbitron" (main heading)
--font-mono: "JetBrains Mono" (technical specs values, pill badges)
--font-loader: "Instrument Serif" (defined but not actively used in the hero)

TAILWIND CSS v4 CONFIGURATION (in index.css)

@import url('https://fonts.googleapis.com/css2?family=Orbitron:wght@400;500;600;700;800;900&family=Space+Grotesk:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&family=Instrument+Serif:ital@1&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Space Grotesk", ui-sans-serif, system-ui, sans-serif;
  --font-display: "Orbitron", sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, monospace;
  --font-loader: "Instrument Serif", serif;
  --color-brand-orange: #F27D26;
  --color-bg: #0a0a0a;
  --color-text: #f5f5f5;
  --color-muted: #888888;
  --color-stroke: #1f1f1f;
}

@layer base {
  body {
    @apply bg-black text-white antialiased;
    font-family: var(--font-sans);
  }
}

@layer utilities {
  .text-glow {
    text-shadow: 0 0 10px rgba(255, 255, 255, 0.3);
  }
}

VITE CONFIG: Use @tailwindcss/vite plugin alongside @vitejs/plugin-react.

3D SPLINE BACKGROUND

Use @splinetool/react-spline with React lazy loading and Suspense
Scene URL: https://prod.spline.design/PIgTjpRFA03yfLyK/scene.splinecode
The Spline container is position: absolute, inset: 0, z-index: 0
It is shifted 15% to the right using inline style: transform: translateX(15%)
Suspense fallback is a full-size black div

PAGE LAYOUT

The root wrapper is: min-h-screen bg-black text-white overflow-x-hidden relative
Selection styling: selection:bg-white selection:text-black
The content sits above the Spline at z-10 with pointer-events-none (interactive elements get pointer-events-auto):
mx-auto px-4 md:px-6 pt-6 md:pt-10 min-h-screen md:h-screen flex flex-col justify-between pb-6 relative z-10 pointer-events-none

The layout uses a CSS Grid with 12 columns on desktop, 1 column on mobile.

TOP SECTION (upper-left content)

All in a single col-span-12 cell with space-y-6 md:space-y-8:

Main Heading: Font: font-display (Orbitron). Text: "Automation" on line 1, then line break, then "Machines •" (using the HTML bull entity). Size: text-[40px] sm:text-[56px] md:text-[72px]. Line height: leading-[1] md:leading-[0.9]. Weight: font-extralight. Tracking: tracking-tight. Transform: uppercase. Max width: max-w-xl. Gradient text effect: bg-gradient-to-r from-white/20 via-white/70 to-white bg-clip-text text-transparent. Animation: Motion div wrapping it, initial={{ opacity: 0, x: -20 }}, animate={{ opacity: 1, x: 0 }}, transition={{ duration: 0.8, ease: "easeOut" }}

Subtitle paragraph: Text: "Developed with high-end skills and a pixel-perfect frame for those who don't just browse the web—they build it. Code your dreams....". Classes: text-sm text-white max-w-md leading-relaxed font-light. Animation: initial={{ opacity: 0 }}, animate={{ opacity: 1 }}, transition={{ duration: 0.8, delay: 0.2 }}

Three circular icon buttons: Icons from Lucide: Snowflake, Maximize, Zap (in that order). Each in a flex row with gap-4. Each icon container: w-10 h-10 rounded-full border border-white/20 flex items-center justify-center hover:border-white/60 transition-colors cursor-pointer pointer-events-auto. Icon: size={16}, className="text-white/80". Animation: initial={{ opacity: 0 }}, animate={{ opacity: 1 }}, transition={{ duration: 0.8, delay: 0.4 }}

BOTTOM SECTION (footer area, pinned to bottom via flex justify-between)

Wrapper: flex flex-col md:flex-row justify-between items-start md:items-end gap-12 md:gap-0 mt-16 md:mt-0

Left side -- Technical Specs card: Animation: initial={{ opacity: 0, y: 40 }}, animate={{ opacity: 1, y: 0 }}, transition={{ duration: 0.8, delay: 0.8 }}. Container: p-6 md:p-8 w-full md:max-w-md pointer-events-auto. Header: text "Technical Specs", classes: text-[10px] font-mono tracking-[0.3em] uppercase text-white/60 mb-5. Four spec rows in a space-y-4 div, each row is: flex justify-between items-end border-b border-white/10 pb-3 group cursor-default. Label (left): text-xs text-white/70 group-hover:text-white transition-colors. Value (right): text-xs font-mono tracking-tight text-white. Data: Stack: "React + Node + SQL", Logic: "V8 - Runtime Logic", Uptime: "99.9% High-Avail", Scale: "Responsive Modern Layout"

Right side -- Pill badge bar: Animation: initial={{ opacity: 0 }}, animate={{ opacity: 1 }}, transition={{ duration: 1, delay: 1 }}. Outer wrapper: flex items-center w-full md:w-auto. Pill container: flex flex-wrap gap-2 bg-white/10 backdrop-blur-md rounded-2xl md:rounded-full p-2 border border-white/5 w-full md:w-auto pointer-events-auto. Four pill badges: "TS/JS" -- ACTIVE/highlighted: px-4 py-2 text-[10px] font-mono tracking-widest bg-white text-black rounded-full. "V1" -- outline: px-3 py-2 text-[10px] font-mono tracking-widest border border-white/20 rounded-full. "Full-Stack" -- outline: px-4 py-2 text-[10px] font-mono tracking-widest border border-white/20 rounded-full. "Cloud-Ready" -- outline: px-4 py-2 text-[10px] font-mono tracking-widest border border-white/20 rounded-full

ANIMATION STAGGER SEQUENCE (Motion from motion/react)

All animations use initial + animate (not scroll-triggered):
Heading: delay 0s, slides in from left (x: -20), 0.8s duration, easeOut
Subtitle: delay 0.2s, fades in, 0.8s duration
Icon buttons: delay 0.4s, fade in, 0.8s duration
Technical specs card: delay 0.8s, slides up from y: 40, 0.8s duration
Pill badge bar: delay 1.0s, fades in, 1.0s duration

KEY DESIGN DETAILS

Color palette: Pure black (#000) background, white text with various opacity levels (white/80, white/70, white/60, white/20, white/10, white/5). No navigation bar -- the hero IS the full page. The 3D scene fills the entire viewport behind the content, offset 15% to the right. All text content is left-aligned on the upper-left. The technical specs and pill badges anchor to the bottom of the viewport. On mobile, layout stacks vertically; on desktop (md breakpoint), it stretches edge-to-edge. The gradient on the heading goes from nearly invisible white (20% opacity) on the left to full white on the right, creating a reveal/fade effect. The text selection color is inverted (white background, black text). pointer-events-none on main prevents accidental interaction with the Spline scene; individual interactive elements opt back in with pointer-events-auto

## Bloom AI — Hero Section [sites/bloom-ai-hero]

- Preview: https://motionsites.ai/assets/hero-bloom-ai-preview-g6RcYLTl.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bloom-ai-hero.gif

Create a full-screen hero landing page for "Bloom" — an AI-powered plant/floral design platform. The design uses a liquid glass morphism aesthetic over a looping video background.

Background
Full-screen autoplaying, looping, muted video background: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260315_073750_51473149-4350-4920-ae24-c8214286f323.mp4
Video covers entire viewport with object-cover, sits at z-0. All content floats above at z-10.

Fonts
Display/Body: Poppins (Google Fonts) — used for headings and body text
Serif accent: Source Serif 4 (Google Fonts) — used only for italic/emphasis text inside headings (e.g., <em>, <i>, .italic inside h1-h3)
Headings use font-weight: 500

Color Palette
Strict grayscale only — all CSS variables are 0 0% X% HSL values
Text is text-white, text-white/80, text-white/60, text-white/50 for hierarchy
No colored accents whatsoever

Liquid Glass CSS (two tiers)
Define under @layer components:

.liquid-glass (light)
background: rgba(255,255,255,0.01);
background-blend-mode: luminosity;
backdrop-filter: blur(4px);
border: none;
box-shadow: inset 0 1px 1px rgba(255,255,255,0.1);
position: relative; overflow: hidden;
::before pseudo-element: gradient border using linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, transparent 40%, transparent 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%) with padding: 1.4px, masked via -webkit-mask-composite: xor; mask-composite: exclude;

.liquid-glass-strong (heavy, for CTA/panels)
Same structure but backdrop-filter: blur(50px), box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15), and ::before uses 0.5/0.2 alpha instead of 0.45/0.15.

Layout — Two-Panel Split
Flex row, min-h-screen. Left panel w-[52%], right panel w-[48%] (hidden on mobile lg:flex).

Left Panel
Has a liquid-glass-strong overlay (absolute inset-4 lg:inset-6 rounded-3xl)
Nav: Logo image (/logo.png, 32×32) + "bloom" text (semibold, 2xl, tracking-tighter, white) on left. "Menu" button with Menu icon on right, liquid-glass pill.
Hero center (flex-1, centered):
Logo image again (80×80)
h1: "Innovating the / spirit of bloom AI" — text-6xl lg:text-7xl, tracking-[-0.05em], white. The italic part uses font-serif text-white/80
CTA button: "Explore Now" with Download icon in a w-7 h-7 rounded-full bg-white/15 circle. Button is liquid-glass-strong, rounded-full, hover:scale-105 active:scale-95
Three pills: "Artistic Gallery", "AI Generation", "3D Structures" — liquid-glass, rounded-full, text-xs text-white/80
Bottom quote:
"VISIONARY DESIGN" label (text-xs tracking-widest uppercase text-white/50)
Quote: "We imagined a realm with no ending." — mixed font-display/font-serif italic spans
Author: "MARCUS AURELIO" with horizontal lines on each side

Right Panel (desktop only)
Top bar: Social icons (Twitter, LinkedIn, Instagram) in a liquid-glass pill with ArrowRight. Account button with Sparkles icon button, both liquid-glass.
Community card: Small liquid-glass card (w-56), "Enter our ecosystem" title + description
Bottom feature section (mt-auto): Outer liquid-glass container with rounded-[2.5rem]
Two side-by-side cards: "Processing" (Wand2 icon) and "Growth Archive" (BookOpen icon), each liquid-glass rounded-3xl
Bottom card: flower image thumbnail (from @/assets/hero-flowers.png, 96×64), "Advanced Plant Sculpting" title + description, and a "+" button. All liquid-glass.

Icons
All from lucide-react: Sparkles, Download, Wand2, BookOpen, ArrowRight, Twitter, Linkedin, Instagram, Menu

Key Details
All interactive elements: hover:scale-105 transition-transform
Social icon links: text-white hover:text-white/80 transition-colors
Icon containers: w-8 h-8 rounded-full bg-white/10 flex items-center justify-center
No border classes anywhere — glass effect handles all borders via ::before
border-radius token: --radius: 1rem

## Crypto Wealth — Hero Section [sites/crypto-wealth-hero]

- Preview: https://motionsites.ai/assets/hero-crypto-wealth-preview-Cv79y7eb.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/crypto-wealth-hero.gif

Recreation Prompt
Build a React + TypeScript + Vite + Tailwind CSS landing page called "ContentFlow" with two scroll-linked fullscreen sections layered over a fixed video background. Use lucide-react for icons. Use the Inter font from Google Fonts (weights 300-900).

Stack & Setup
Vite + React 18 + TypeScript
Tailwind CSS with default config
lucide-react for icons (Layers, Calendar, Lock, BarChart3, FileText)
Load Inter font via Google Fonts preconnect in index.html; set * { font-family: 'Inter', sans-serif; }
Page title: ContentFlow
Global Layout (App.tsx)
Root wrapper: <div className="min-h-[200vh]"> (creates scroll room)
Renders <VideoBackground zoom={videoZoom} /> then <Navbar /> then a <div className="relative" style={{ zIndex: 10 }}> containing <Hero> and <ShowcaseSection>
Uses a custom useScrollProgress() hook returning 0..1 where progress = min(scrollY / viewportHeight, 1) (updated via rAF on passive scroll listener)
videoZoom = 1 + scrollProgress * 0.3
Video Background (Section 1 — fixed behind everything)
src/components/VideoBackground.tsx:

Single fixed, full-viewport video background, pointer-events-none, zIndex: 0
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260417_061226_74f0749c-a22d-42b3-895e-5d6203bc741c.mp4
autoPlay loop muted playsInline, object-cover, absolutely positioned to fill
Outer wrapper fixed inset-0 overflow-hidden pointer-events-none
Inner wrapper applies transform: scale(${zoom}) with transformOrigin: 'center center' and will-change-transform
Accepts zoom?: number prop, default 1
Navbar (fixed, floating pill)
src/components/Navbar.tsx:

<nav> fixed top/left/right, z-50, centered via flex, pt-6 px-6
Inner pill: flex items-center gap-4 md:gap-8 bg-white/70 backdrop-blur-sm border border-gray-200/80 rounded-full pl-4 pr-2.5 md:pl-5 md:pr-2.5 py-2.5 shadow-sm
Left logo: inline SVG 28x28 viewBox 0 0 256 256, four rounded-corner "petal" shapes in black (path d="M 128 192 C 92.654 192 64 220.654 64 256 L 0 256 C 0 185.308 57.308 128 128 128 Z M 256 128 C 256 198.692 198.692 256 128 256 L 128 192 C 163.346 192 192 163.346 192 128 Z M 128 64 C 92.654 64 64 92.654 64 128 L 0 128 C 0 57.308 57.308 0 128 0 Z M 256 0 C 256 70.692 198.692 128 128 128 L 128 64 C 163.346 64 192 35.346 192 0 Z")
Hidden-on-mobile links (hidden md:flex items-center gap-7): Features, Workflows, Resources, Pricing — text-sm font-medium text-gray-600 hover:text-gray-900 transition-colors duration-150
"Get started" button: gradient-border-btn text-sm font-semibold text-gray-900 rounded-full px-5 py-2 hover:bg-gray-50 shadow-sm
.gradient-border-btn CSS: white background with a masked ::before pseudo-element creating a 1.5px gradient border using linear-gradient(135deg, #F59E0B, #3B82F6) via -webkit-mask + mask-composite: exclude trick
Hero Section (Section 1 content)
src/components/Hero.tsx:

Props: { scrollProgress: number }
opacity = max(1 - scrollProgress * 2.5, 0)
translateY = scrollProgress * -60 (pixels)
Root <section>: relative flex flex-col items-center justify-start px-4 sm:px-6 pt-32 sm:pt-36 md:pt-40 text-center min-h-screen will-change-transform, inline style applies opacity, transform: translateY(...), zIndex: 10, pointerEvents: opacity < 0.1 ? 'none' : 'auto'
Heading <h1> with classes text-[2.25rem] sm:text-[3rem] md:text-[3.75rem] leading-none tracking-tighter font-medium text-gray-900 max-w-2xl:
Line 1: A New Way wrapped in <span className="text-zinc-400">
Line 2: to Manage Your
Line 3: Content Flow
Lines separated with <br />
Subcopy <p className="mt-8 text-base text-gray-500 max-w-sm leading-relaxed">: Take full control of your publishing workflow <br /> with our unified content management platform.
Then renders <PopupCard />
PopupCard (animated card stack under hero)
src/components/PopupCard.tsx:

Container: flex flex-col items-center gap-3 mt-10 sm:mt-16 w-full px-4 sm:px-0
Renders 4 cards, each with a staggered animation delay of i * 150ms
Card layout: popup-card-animate flex items-center gap-3 sm:gap-4 bg-white/80 backdrop-blur-sm rounded-2xl shadow-lg border border-gray-200/60 px-4 sm:px-6 py-3 sm:py-4 w-full max-w-[380px]
Each card has: leading icon, middle text (text-xs sm:text-sm font-medium text-gray-700 flex-1 whitespace-nowrap), trailing 28x28 SVG spin ring (light gray full circle + gradient arc from 12 o'clock to 3 o'clock, strokeWidth=3, round cap; arc uses a per-card linearGradient from gradientFrom to gradientTo with coords x1=14 y1=3 x2=25 y2=14)
The trailing SVG has class spin-ring that rotates 360deg every 1.4s linear infinite
Cards:
Icon: the same black 4-petal logo SVG (24x24); text Your All-in-One Content Studio; gradient #F59E0B → #3B82F6
Icon: <Layers className="w-6 h-6 text-emerald-600 flex-shrink-0" />; text Multi-Channel Publishing Hub; gradient #10B981 → #06B6D4
Icon: <Lock className="w-6 h-6 text-blue-600 flex-shrink-0" />; text Role-Based Access & Approvals; gradient #3B82F6 → #0EA5E9
Icon: <BarChart3 className="w-6 h-6 text-amber-600 flex-shrink-0" />; text Advanced Content Analytics; gradient #F59E0B → #EF4444
Keyframes popup-card-in: 0% opacity:0; transform: translateY(16px) scale(0.96) → 100% opacity:1; transform: translateY(0) scale(1); applied via .popup-card-animate { opacity:0; animation: popup-card-in 0.6s cubic-bezier(0.16,1,0.3,1) forwards; } with per-card animation-delay
Showcase Section (Section 2)
src/components/ShowcaseSection.tsx:

Props: { scrollProgress: number }
Fade-in/scale-up mapping: fadeStart = 0.35, fadeEnd = 0.75; t = clamp((scrollProgress - fadeStart) / (fadeEnd - fadeStart), 0, 1); apply opacity = t and scale = 0.88 + t * 0.12
Root <section>: relative min-h-screen flex items-center justify-center px-6 md:px-16 lg:px-24 will-change-transform, inline style opacity, transform: scale(...), transformOrigin: 'center top', zIndex: 20
Inner card: relative w-full max-w-7xl mx-auto rounded-2xl sm:rounded-3xl overflow-hidden min-h-[480px] sm:min-h-[560px] md:min-h-[680px]
Background video inside the card (absolute inset-0 w-full h-full object-cover, autoplay/loop/muted/playsInline):
URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260415_154932_f36efd90-557d-4cfb-add6-79336918bd53.mp4
Overlay: absolute inset-0 z-[1] bg-gradient-to-t from-black/90 via-black/50 to-transparent pointer-events-none
Foreground grid (relative z-10 flex flex-col md:flex-row items-end md:items-stretch h-full min-h-[480px] sm:min-h-[560px] md:min-h-[680px]):
Left half (flex flex-col justify-end p-4 sm:p-5 md:p-8 md:w-1/2):
<h2 className="text-2xl sm:text-3xl md:text-[2.75rem] font-medium text-gray-900 md:text-white md:leading-[1.15] leading-tight tracking-tighter">Your Content Engine,<br />Faster and Clearer</h2>
<p className="mt-5 text-sm md:text-base text-gray-600 md:text-white/70 leading-relaxed max-w-md">Get live performance data, editorial analytics, and the clarity you need to publish confidently every single time.</p>
Right half (flex items-end justify-end md:w-1/2 mt-auto origin-bottom-right scale-[0.75] sm:scale-[0.85] md:scale-100): renders <Dashboard />
Dashboard Card
src/components/Dashboard.tsx:

Container: dashboard-animate bg-white rounded-tl-2xl shadow-xl border border-gray-100 p-4 sm:p-6 md:p-8 w-full max-w-xl flex flex-col gap-4 sm:gap-6
Animation .dashboard-animate: opacity:0; animation: slide-in-right 0.9s ease-out 1.2s forwards; keyframes go from opacity:0; translateX(40px) to opacity:1; translateX(0)
Top row (flex flex-col sm:flex-row items-start sm:justify-between gap-3 sm:gap-0):

Left: w-10 h-10 sm:w-12 sm:h-12 rounded-xl bg-gray-100 containing <FileText className="w-5 h-5 sm:w-6 sm:h-6 text-gray-600" />, label Total Reach (text-xs sm:text-sm text-gray-400 font-medium), big value 498,098 (text-xl sm:text-3xl font-bold text-gray-900 tracking-tight) with trailing views in small gray
Right: legend dots Published (black 2x2 dot) and In Draft (gray-300 dot), plus pill badge Monthly (text-xs font-medium text-gray-700 bg-gray-100 rounded-md px-2.5 py-1)
Chart (AnimatedChart):

SVG viewBox 0 0 400 180; weekly data points:
Mon 18000, Tue 22000, Wed 19000, Thu 25000, Fri 21000, Sat 32000, Sun 28000
padTop=10, padBottom=30, maxVal=40000; points distributed horizontally evenly
smooth() builds a path using midpoint cubic Bézier control points to produce a smooth curve
Gradient chartGrad: #1F2937 0.08 → #1F2937 0 vertically; used as area fill under the line
Y-axis labels 0, 10k, 20k, 30k, 40k drawn as text-anchor="end", fill-gray-400, fontSize=9, Inter; horizontal dashed grid lines stroke="#E5E7EB", strokeWidth=0.5, strokeDasharray="4 3"
X-axis day labels at y = height - 8, fontSize=9; highlight index 5 (Sat) is fill-gray-900 font-semibold, others fill-gray-400
Line stroke #1F2937, width 2, round caps
Animations (via refs on mount):
Line path: dasharray/dashoffset trick, transition: stroke-dashoffset 1.8s ease-out to 0
Area: opacity 0 → 1 over 1s, starting at 800ms
Highlight dot at Sat (r=5, white fill, #1F2937 stroke width 2, scale-in from 0 at 1600ms over 0.4s)
Tooltip group (rect x=hx-48, y=hy-32, 96x22, rx=6, fill #1F2937; texts 32,104 white and +6,488 in #34D399, both fontSize=9): fade + translateY(4px→0) starting 1800ms over 0.4s
Dashed vertical drop line from highlight point to baseline (stroke="#1F2937", strokeDasharray="3 2")
Top Channels block:

Heading Top Channels (text-sm font-semibold text-gray-700 mb-4)
Three rows (flex items-center justify-between):
BLG Blog Posts 12,461 +4.20% positive, color #F59E0B
SOC Social Media 8,932 -1.05% negative, color #3B82F6
NWS Newsletters 5,718 +2.87% positive, color #10B981
Each row left: colored circular badge (8×8 sm:10×10, white bold letter = first char of symbol, backgroundColor: ch.color), symbol bold + full name in gray-400 (hidden on small)
Each row right: price, change in text-emerald-500 (positive) or text-red-500 (negative), then a ProgressRing (40x40 svg) hidden on small
ProgressRing: r=16, circ = 2πr, strokeDasharray=circ, strokeDashoffset=circ - (percent/100)*circ, base stroke #E5E7EB, foreground stroke color, strokeWidth=3, round caps, rotated -90° around center. Class progress-ring-fill runs @keyframes progress-fill { 0% { stroke-dashoffset: 88; } } for 1.2s ease-out starting at 2s, both
Use 70% for positive, 30% for negative
CSS Keyframes to include (src/index.css)
spin-ring (0 → 360deg, 1.4s linear infinite)
slide-in-right used by .dashboard-animate (opacity + translateX from 40px, 0.9s ease-out 1.2s forwards)
progress-fill used by .progress-ring-fill (stroke-dashoffset from 88 to 0, 1.2s ease-out 2s both)
popup-card-in used by .popup-card-animate (0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards; per-card inline animation-delay)
.gradient-border-btn with ::before masked 1.5px gradient border using linear-gradient(135deg, #F59E0B, #3B82F6) and -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0); -webkit-mask-composite: xor; mask-composite: exclude;
Scroll Behavior Summary
Scrolling 0 → 100% of viewport height drives scrollProgress 0 → 1
Hero fades out quickly (opacity = 1 - p*2.5) and drifts up (translateY = -60px * p)
Background video zooms from 1x to 1.3x
Showcase section fades/scales in between p=0.35 and p=0.75 (opacity 0→1, scale 0.88→1.0), transform-origin center top

## DesignPro Academy — Hero Section [sites/designpro-hero]

- Preview: https://motionsites.ai/assets/hero-designpro-preview-D8c5_een.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/designpro-hero.gif

Create a full-screen hero section for a product design education platform called "DesignPro" with the following exact specifications:

Background:

Full-screen looping video background using this exact CloudFront URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_105406_16f4600d-7a92-4292-b96e-b19156c7830a.mp4

Video should autoplay, loop, be muted, and play inline

Background color: black (#000000)

Navigation Bar:

Logo: Circular design with a white border (2px), containing a smaller filled white circle inside, followed by "DesignPro" text

Navigation links in a rounded pill container with gray-700 border: Home, About Us, Courses, Instructors, Testimonials, Blog, Contact us (with arrow icon)

All nav links: white/80 opacity, hover to full white

Font size: text-sm

Mobile: Show hamburger menu icon on screens smaller than lg

Max width: 7xl container with proper padding

Content Layout:

Top Section (below nav):

Two-column layout on large screens, stacked on mobile

Left column: "We deliver transformative programs that empower emerging product designers with cutting-edge expertise and vision to thrive globally."

Right column (right-aligned on lg+): "8000+ Talented Designers Launched !"

Both paragraphs: white/80 opacity, text-sm on mobile, text-base on desktop

Hero Section (center):

Small uppercase text above heading: "Seats for Next Program Opening Soon" (white/80 opacity, text-xs on mobile, text-sm on desktop, tight tracking)

Main heading with these exact specifications:

Line 1: "Become" in white, font-medium

Line 2: "Product Leader." with animated shiny gradient effect

Font sizes: text-5xl (mobile) scaling up to text-9xl (xl screens)

Line height: 0.85

Letter spacing: tracking-tighter

ShinyText Component:

Use framer-motion for animation

Base color: #64CEFB (light blue)

Shine color: #ffffff (white)

Animation speed: 3 seconds

Gradient spread: 100 degrees

Gradient should sweep across text continuously from left to right

Use CSS gradient with backgroundClip: text and transparent text fill

CTA Button:

Text: "Apply for Next Enrollment" with arrow icon (from lucide-react)

Black background, hover: gray-900

Rounded-full shape

Padding: px-6 md:px-8, py-3 md:py-4

Arrow should translate right on hover

Group hover animation on arrow icon

Typography:

Font family: Inter (sans-serif)

All text colors: white/80 opacity for body text, full white for headings and hover states

Technical Stack:

React + TypeScript

Vite

Tailwind CSS

Framer Motion for animations

Lucide React for icons

Responsive Breakpoints:

Mobile-first design

sm: 640px

md: 768px

lg: 1024px

xl: 1280px

Key CSS Details:

Container max-width: max-w-7xl with centered margins

Section height: h-screen

Video: absolute positioning, inset-0, object-cover

Content: relative z-10 positioning to appear above video

Smooth transitions on all interactive elements

Create the complete implementation including the ShinyText component with proper framer-motion animation logic.

## Digital Epoch — Hero Section [sites/digital-epoch-hero]

- Preview: https://motionsites.ai/assets/hero-digital-epoch-preview-B85ezqXO.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/digital-epoch-hero.gif

Build a modern, high-performance landing page section using React, TypeScript, Tailwind CSS v4, and Motion. The application should match the following exact specifications:
1. Dependencies & Setup
Libraries: Install lucide-react, motion, clsx, and tailwind-merge.
Fonts & CSS: In index.css, import the Inter and Outfit fonts from Google Fonts: @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Outfit:wght@400;500;600&display=swap');
Configure the Tailwind theme in your CSS to use Inter as --font-sans and Outfit as --font-display.
The global body background should be #f9fafb.
2. Main Hero Container & Video Background
Create a hero section container with these exact classes: relative w-full max-w-[1400px] mx-auto rounded-[48px] bg-white border border-slate-200/50 shadow-[0_40px_100px_-20px_rgba(0,0,0,0.03)] overflow-hidden h-[600px] flex flex-col.
Inside, add an absolutely positioned underlying layer (absolute inset-0 pointer-events-none z-0 overflow-hidden select-none) for the background video.
The video tag must point to exactly this URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260505_101331_74f9b798-3f00-4e86-8a01-377aa16ffeaa.mp4. It must include autoPlay, loop, muted, and playsInline attributes, with the classes: w-full h-full object-cover scale-105 transition-transform duration-1000. No overlays.
3. Hero Text Content
Create a content wrapper positioned relative (z-20 flex-1 px-8 md:px-16 pt-12 md:pt-16 flex flex-col items-start).
Use motion.div from motion/react to animate the text layer in (fade in, slide up slightly).
Headline: "Foundation of the<br />new digital epoch". Should use the font-display font, sizes text-[42px] md:text-[56px], medium weight, tight tracking, color #0a1b33.
Subheadline: "Designing products, powering ecosystems and laying the foundation of a decentralized web for enterprises, builders and communities alike." Should use font-sans, sizes text-[14px] md:text-[15px], color #64748b.
Contact Button: Text "Contact Us", using a dark background (bg-[#0a152d]), white text, rounded-full, with hover scale animations via motion.button.
4. Floating Bottom Navbar
Create an absolutely positioned navbar wrapper at the bottom center of the hero: absolute bottom-10 left-1/2 -translate-x-1/2 z-30.
The nav element should use motion.nav to fade in and slide up (delayed after the text). It must have the classes: flex items-center bg-white/90 backdrop-blur-2xl px-1.5 py-1.5 rounded-full shadow-[0_12px_40px_rgba(0,0,0,0.08)] border border-slate-200/40.
Nav Elements:
A small circular logo placeholder on the left (w-9 h-9 bg-white border-slate-100 shadow-sm) containing the star character "✦".
Two standard text buttons: "Products" and "Docs" (text-[12px] font-semibold text-slate-500 hover:text-[#0a1b33]).
A "Get in touch" button on the right containing the text and a small ChevronRight (from lucide-react). Styled identically to the marquee cards: bg-white px-5 py-2 rounded-full text-[12px] font-semibold text-[#0a1b33] border border-slate-200/60 shadow-sm hover:border-slate-300 transition-all.
5. Seamless Marquee Logo Scroller Component
Below the hero container (mt-10), add a custom highly-performant Marquee Scroller component.
The scroller must use a pure CSS @keyframes animation (transform: translateX(0) to translateX(-50%)) for infinite scrolling, pausing on hover. It needs a left/right masking gradient (maskImage linear-gradient fading to transparent on the edges). No title or description text above the scroller.
The Logos List: Supply an array of 8 objects with src URLs from svgl.app, alt names, and hex gradient objects:
Procure (procure.svg, blue gradient)
Shopify (shopify.svg, yellow gradient)
Blender (blender.svg, blue gradient)
Figma (figma.svg, purple gradient)
Spotify (spotify.svg, pink/red gradient)
Lottielab (lottielab.svg, yellow/green)
Google Cloud (google-cloud.svg, light blue)
Bing (bing.svg, cyan/teal)
Render the list twice inline to ensure a seamless loop.
Card Design: Make each logo's container card exactly match the "Get in touch" navbar button's styling. The container classes must be exactly: group relative h-24 w-40 shrink-0 flex items-center justify-center rounded-full bg-white border border-slate-200/60 shadow-sm hover:border-slate-300 transition-all overflow-hidden.
Inside the card, add an absolute div using the specific gradient colors, scaled at 1.5 and 0 opacity, which drops to scale 1 and opacity 100 on group-hover.
The image tag should invert/turn black on hover (group-hover:brightness-0 group-hover:invert).

## Dot — Hero Section [sites/dot-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/dot-hero-Csf49OgS.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/dot-hero.gif

Build a React landing page exactly as specified below. Use React 19, Tailwind CSS v4, and motion/react for animations.
1. Fonts & Global CSS Setup:
In index.html, import these Google Fonts:
Instrument Serif (weights 400, italic 400)
Inter (weights 100 to 900)
In src/index.css, import this custom font for the Nokia text:
@import url('https://db.onlinewebfonts.com/c/440b53b1a1c65037f944ff19259d8014?family=Nokia+Cellphone+FC+Small');
Configure the Tailwind theme variables in index.css:
--font-instrument: "Instrument Serif", serif;
--font-serif: "Instrument Serif", serif;
--font-sans: "Inter", sans-serif;
--font-nokia: "Nokia Cellphone FC Small", monospace;
Create a @utility font-instrument { font-family: "Instrument Serif", serif; }
Set the root font-family to var(--font-sans) and apply anti-aliasing.
2. Component Structure:
Create one main App.tsx file containing 4 components: TypingMessages, Navbar, Hero, and App.
3. Navbar Component:
Container: Fixed to the top top-6, centered horizontally left-1/2 -translate-x-1/2, width 95% w-[95%] max-w-5xl. z-50, pointer-events-none.
Nav Tag: pointer-events-auto, backdrop blur, rounded full pill shape, transparent background with border border-black/10. Flex between items.
Logo: Text "dot." using font-instrument text-[28px] tracking-tight text-[#1a1a1a].
Links: "Philosophy", "Trust", "Access", "Tribe". Hidden on mobile, flex on desktop (gap-10). font-sans text-[14px] text-[#1a1a1a] with hover opacity fading.
CTA Button ("Link up"):
Background #0871E7, rounded full, white text font-sans text-[14px].
Shadow: shadow-[inset_0_-4px_4px_rgba(255,255,255,0.39)] outline-1 outline-[#0871E7] -outline-offset-1.
Add a subtle top glint effect using an absolutely positioned rectangle inside the button: w-[80%] h-4 left-[10%] top-[1px] bg-gradient-to-b from-[#DEF0FC] to-transparent rounded-[12px]. Make it scale wider on group hover (group-hover:scale-x-105).
4. Hero Component:
Container: min-h-screen bg-[#F3F4ED] pt-24 md:pt-32 flex column centered.
Video Background: Absolute positioning inset-0 z-0. Use an HTML5 <video> set to autoplay, loop, muted, playsInline, scaling with object-cover.
Video Source: EXACTLY https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_054418_a6d194f0-ac86-4df9-abe5-ded73e596d7c.mp4. Add an overlaid empty div with bg-white/5 for a slight tint.
Hero Text Container: Relative z-20, pointer-events-none, text-centered layout.
Main Headline: "Short notes. <br /> Daily calm."
Animate using motion.div (from opacity: 0, scale: 0.95 to opacity: 1, scale: 1 over 1.5s with ease [0.16, 1, 0.3, 1]).
Style: font-instrument text-[38px] md:text-[56px] lg:text-[72px] leading-[0.85] tracking-tight text-[#1a1a1a] mb-6.
Sub-headline: "Linked with a single anonymous peer. One message every day. A quiet rhythm in the digital noise."
Animate using motion.div (from opacity: 0, y: 20 to opacity: 1, y: 0 over 1.2s, delay: 0.3, ease [0.16, 1, 0.3, 1]).
Style: font-sans text-[16px] md:text-[18px] text-[#1a1a1a]/70 leading-relaxed font-normal max-w-xl mx-auto.
Include the TypingMessages component inside the hero to overlap on the phone screen in the video.
5. TypingMessages Component:
Logic: Cycle through an array of messages: ["Are you here?", "Yes, I am.", "Speak soon."].
Typing speed: 100ms. Deleting speed: 50ms. Pause before deleting: 2000ms.
Positioning: Absolute position it to sit perfectly on the phone screen inside the video:
absolute left-[48.5%] md:left-[47.5%] lg:left-[48.5%] -translate-x-1/2 bottom-[32%] z-30 w-[110px] sm:w-[130px] flex justify-start text-left.
Text Style: font-nokia text-[#2A3616] text-[10px] sm:text-[14px] leading-tight break-words min-h-[1.5em].
Cursor: Add a blinking Framer Motion cursor motion.span (w-1.5 h-3 bg-[#2A3616] ml-1 align-middle) animating opacity from 0 to 1 to 0 over 0.8s, repeating infinitely, linearly.

## Duolingo Styleguide — Hero Section [sites/duolingo-styleguide-hero]

- Preview: https://motionsites.ai/assets/hero-duolingo-styleguide-preview-1HTxQ6Tj.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/duolingo-styleguide-hero.gif

Fonts
Primary font: 'Nunito' from Google Fonts (weights: 400, 500, 600, 700, 800, 900)
Display/heading font: 'Feather Bold' from https://db.onlinewebfonts.com/c/14936bb7a4b6575fd2eee80a3ab52cc2?family=Feather+Bold
Font stack fallback: 'Nunito', 'DIN Round Pro', -apple-system, BlinkMacSystemFont, sans-serif
Color Variables (CSS custom properties)

--green: rgb(88, 204, 2)
--green-hover: rgb(75, 178, 0)
--green-shadow: #61B800
--dark-blue: rgb(16, 15, 62)
--blue: rgb(28, 176, 246)
--gray-text: rgb(75, 75, 75)
--gray-light: rgb(119, 119, 119)
--border-color: rgb(229, 229, 229)
--nav-text: rgb(175, 175, 175)
--footer-green: #4EC604
--red: #FF4B4B
--orange: #FF9600
--golden: #FFC800
Structure & Layout
Fixed Navbar (64px height, white background, bottom border)
Left side: Duolingo logo image (https://d35aaqx5ub95lt.cloudfront.net/images/splash/f92d5f2f7d56636846861c458c0d0b6c.svg, 140x33px), followed by a 1px vertical divider (24px tall), then "STYLE GUIDE" label (11px, uppercase, letter-spacing 1.5px, gray)
Right side: Horizontal nav links - "Colors", "Type", "Buttons", "Cards", "Components" (13px, bold, uppercase, 0.5px letter-spacing, gray, with green hover/active states and subtle green background on hover)
Max-width: 1440px, centered
Hero Section (centered, green-to-white gradient background)
Headline: "duolingo design" in Feather Bold font, 52px, green color (#58CC02), lowercase
Description: "A comprehensive visual reference for the Duolingo design system covering colors, typography, button variants, cards, and UI components." -- 17px, gray-light color, max-width 520px, 1.5 line-height
Two buttons below: Primary "GET STARTED" button (green, white text, 12px border-radius, 4px green box-shadow for 3D effect, uppercase bold) and Secondary "I ALREADY HAVE AN ACCOUNT" button (transparent with 2px gray border, blue text, 4px gray box-shadow for 3D effect)
Both buttons: 48px height, 24px horizontal padding, 15px font-size, 700 weight, uppercase
Buttons have active state: box-shadow removed, translateY(4px)
Padding: 56px top, 40px sides, 40px bottom
Main Grid (2-column grid, no gap, max-width 1440px)
Each panel has 36px vertical and 40px horizontal padding, bottom border and right border (border-color). Even panels have no right border.

Each panel has a section label: 11px, 800 weight, uppercase, 2px letter-spacing, gray (nav-text), with a 1px line extending to the right via ::after pseudo-element.

Panels in order (left-to-right, top-to-bottom):

Panel 1: Color Palette (light)
Grid of 12 color swatches, grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)), 12px gap. Each swatch:

Square (aspect-ratio: 1), 12px border-radius, 1px border rgba(0,0,0,0.06)
Hover: scale(1.05) with box-shadow 0 8px 24px rgba(0,0,0,0.12)
Below swatch: name (12px, bold, gray-text) and hex value (10px, gray-light, semi-bold)
Colors in order:

Green -- rgb(88, 204, 2) -- #58CC02
Green Hover -- rgb(75, 178, 0) -- #4BB200
Blue -- rgb(28, 176, 246) -- #1CB0F6
Dark Blue -- rgb(16, 15, 62) -- #100F3E
Red -- #FF4B4B
Orange -- #FF9600
Golden -- #FFC800
Footer Green -- #4EC604
Gray Text -- rgb(75, 75, 75) -- #4B4B4B
Gray Light -- rgb(119, 119, 119) -- #777777
Nav Text -- rgb(175, 175, 175) -- #AFAFAF
Border -- rgb(229, 229, 229) -- #E5E5E5
Panel 2: Typography (light)
Vertical stack with 20px gap. Each row is a flex row (baseline-aligned, 20px gap) with a meta column (80px wide, right-aligned) showing size in blue (11px bold) and weight label below (10px, nav-text color), then the sample text.

Rows:

48px / Feather Bold -- "Display" -- green color, Feather Bold font
32px / Bold 700 -- "Heading One" -- gray-text color
28px / Feather Bold -- "heading two" (lowercase) -- green color, Feather Bold font
18px / Medium 500 -- "Body text for paragraphs and descriptions with comfortable reading line-height." -- gray-light color, 1.6 line-height
14px / Bold 700 -- "CAPTION LABEL" -- uppercase, nav-text color, 0.5px letter-spacing
12px / Semi 600 -- "Small utility text for metadata and hints" -- gray-light color
Panel 3: Button Variants (light)
Vertical stack with 16px gap. Each row has an 80px label (10px, bold, uppercase, 1px letter-spacing, nav-text) then buttons with 12px gap, flex-wrap.

Rows:

"Primary" -- 3 buttons: "GET STARTED" (green bg, white text, 4px green shadow), "SMALL" (same but 36px height, 13px font, 16px padding, 10px radius, 3px shadow), "DISABLED" (same as primary but opacity 0.45, pointer-events none)
"Secondary" -- 3 buttons: "LEARN MORE" (transparent, 2px #CFCFCF border, blue text, 4px #CFCFCF shadow), "SMALL" (same sizing as small primary), "DISABLED" (opacity 0.45)
"Danger" -- 2 buttons: "DELETE" (#FF4B4B bg, white text, 4px #CC3C3C shadow), "REMOVE" (small variant)
"Ghost" -- 1 button: "VIEW ALL" (no bg/border/shadow, green text, green bg on hover at 0.08 opacity)
Panel 4: Dark Theme Buttons (dark-blue background)
Section label and ::after line use white at 35% and 10% opacity respectively.

Two rows:

"GET STARTED" primary + "TRY 1 WEEK FREE" (white bg, dark-blue text, 4px #88879F shadow, hover bg #c8f040)
Small variants of both
Panel 5: Cards (light)
2-column grid, 16px gap. Each card: white bg, 2px border (border-color), 16px border-radius. Hover: translateY(-4px), box-shadow 0 12px 32px rgba(0,0,0,0.08).

Card 1:

Image: https://images.pexels.com/photos/4145354/pexels-photo-4145354.jpeg?auto=compress&cs=tinysrgb&w=400&h=200&fit=crop (120px height, cover)
Tag: "NEW" (green text, green bg at 10% opacity, 11px, 800 weight, uppercase, 6px radius, 3px/8px padding)
Title: "Spanish for Beginners" (16px, bold, gray-text)
Description: "Start your language journey with interactive lessons designed to build fluency." (13px, gray-light, 1.5 line-height)
Footer (12px top border, 12px/16px padding): left "12 UNITS" (12px bold uppercase nav-text), right "START" (12px bold uppercase blue, hover opacity 0.7)
Card 2:

Image: https://images.pexels.com/photos/267669/pexels-photo-267669.jpeg?auto=compress&cs=tinysrgb&w=400&h=200&fit=crop
Tag: "POPULAR" (blue text, blue bg at 10% opacity)
Title: "French Conversations"
Description: "Practice real-world dialogue and improve pronunciation with native speakers."
Footer: "8 UNITS" / "CONTINUE"
Panel 6: Dark Theme Cards (dark-blue background)
2-column grid, same structure but no images. Cards have bg rgba(255,255,255,0.06), border rgba(255,255,255,0.08). Titles are white, descriptions are white at 50% opacity, footer border is white at 8% opacity, footer text is white at 30% opacity.

Card 1:

Tag: "SUPER" (golden #FFC800 text, golden bg at 15% opacity)
Title: "Unlimited Hearts"
Desc: "Keep learning without interruption with Super Duolingo benefits."
Footer: "PREMIUM" / "UPGRADE"
Card 2:

Tag: "PRO" (orange #FF9600 text, orange bg at 15% opacity)
Title: "Mastery Quizzes"
Desc: "Challenge yourself with advanced assessments to test your skill level."
Footer: "ADVANCED" / "TRY NOW"
Panel 7: Components (light)
Vertical stack with 20px gap. Each group has a label (10px bold uppercase, 1px letter-spacing, nav-text).

Badges: Flex row, 8px gap. Pill-shaped badges (4px/10px padding, 20px radius, 12px bold uppercase):

"COMPLETED" (green text, green bg 12%)
"IN PROGRESS" (blue text, blue bg 12%)
"FAILED" (red text, red bg 12%)
"STREAK" (orange text, orange bg 12%)
"PREMIUM" (golden-brown #b8920f text, golden bg 15%)
Input + Button: Flex row, 12px gap. Input (flex:1, 48px height, 16px padding, 2px border border-color, 12px radius, 15px font, 600 weight, focus border turns blue, placeholder is nav-text color 500 weight) + Primary "SUBSCRIBE" button.

Toggle: Flex row with two toggle switches. Each toggle is 48x28px. Track is border-color bg, 14px radius. Thumb is 22x22px white circle, 3px from edges, with 1px 3px rgba(0,0,0,0.15) shadow. Checked state: track turns green, thumb translates 20px right. Labels "Sound effects" and "Animations" (14px, 600 weight). First toggle is checked by default.

Progress: 3 progress bars in a column, 10px gap. Each row: flex, 12px gap, bar (flex:1, 12px height, border-color bg, 6px radius, overflow hidden), fill (6px radius, 0.6s ease width transition), value (12px bold, 32px wide, right-aligned).

85% green fill
60% blue fill
35% orange fill
Tooltips & Streak: Flex row, 16px gap, center-aligned.

Tooltip trigger: "Hover me" (13px, bold, green text, green bg 8%, 8px/16px padding, 8px radius). On hover shows tooltip bubble above (dark-blue bg, white 12px 600-weight text, 6px/12px padding, 8px radius, 5px triangle arrow pointing down via ::after border trick).
Streak counter: Inline-flex, 6px gap, 6px/14px padding, orange bg 10%, 20px radius. Fire emoji (18px) + "42" (16px, 800 weight, orange).
Panel 8: Dark Theme Components (dark-blue background)
Labels use white at 30% opacity.

Language Pills: Flex row, 8px gap. Each pill: inline-flex, 6px gap, 6px/12px padding, 2px border, 12px radius, cursor pointer, hover turns border green with subtle green bg.

"Spanish" (ACTIVE -- green border, green bg 8%, white text) with flag https://d35aaqx5ub95lt.cloudfront.net/vendor/59a90a2cedd48b751a8fd22014768fd7.svg
"French" (inactive -- white border 12%, white text 70%) with flag https://d35aaqx5ub95lt.cloudfront.net/vendor/482fda142ee4abd728ebf4ccce5d3307.svg
"German" with flag https://d35aaqx5ub95lt.cloudfront.net/vendor/c71db846ffab7e0a74bc6971e34ad82e.svg
"Japanese" with flag https://d35aaqx5ub95lt.cloudfront.net/vendor/edea4fa18ff3e7d8c0282de3f102aaed.svg
Flag images: 24x18px, object-fit contain. Pill text: 13px, bold.
Avatar Group: Flex row with overlapping circular avatars (36px, 50% radius, 2px white border, -8px margin-left except first). Images:

https://images.pexels.com/photos/774909/pexels-photo-774909.jpeg?auto=compress&cs=tinysrgb&w=80&h=80&fit=crop
https://images.pexels.com/photos/1222271/pexels-photo-1222271.jpeg?auto=compress&cs=tinysrgb&w=80&h=80&fit=crop
https://images.pexels.com/photos/733872/pexels-photo-733872.jpeg?auto=compress&cs=tinysrgb&w=80&h=80&fit=crop
Count badge "+5" (same 36px circle, #f0f0f0 bg, 11px 800 weight, gray-light)
Text next to group: "8 learners active" (13px, 600 weight, white 50% opacity)
Progress (Dark): 2 bars, track bg is white 8%, values are white 60%:

72% golden fill
45% green fill
Badges (Dark):

"MASTERED" (green bg 15%, #7ADB2E text)
"REVIEW" (blue bg 15%, #4DC4F8 text)
"CROWN" (golden bg 15%, #FFC800 text)
Responsive Breakpoints
900px and below:

Grid becomes single column, no right borders
Hero h1: 36px
Nav links hidden
Cards grid becomes single column
Hero buttons stack vertically, max-width 280px
600px and below:

Hero padding: 40px 20px 32px
Hero h1: 28px
Panel padding: 28px 20px
Color grid: 3 columns
Type meta column: hidden
Display type: 32px
Button labels: hidden
Input row: column direction

## EcoVolta — Hero Section [sites/ecovolta-hero]

- Preview: https://motionsites.ai/assets/hero-ecovolta-preview-BXrSPAWj.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ecovolta-hero.gif

PROMPT:

Build a single-page hero section for a renewable energy company called "EcoVolta" using React, TypeScript, Vite, Tailwind CSS, Framer Motion, and Lucide React icons. The page should be a full-viewport height (h-screen) layout with no scrolling (overflow-hidden).

TECH STACK:

React 18 + TypeScript
Vite
Tailwind CSS 3
Framer Motion (for animations)
Lucide React (for icons)
clsx + tailwind-merge (for className utility)
Font: Google Fonts Inter (weights 300-900)
BACKGROUND:

The entire page background color is #F5F3EE (warm off-white/cream)
A full-page background video plays on loop, autoplaying, muted, with playsInline. It is fixed, inset-0, w-full h-full object-cover at z-0
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260402_161801_19c1f902-b569-4d42-87b0-4de571a14399.mp4
NAVBAR (z-10, relative):

Flex row, space-between, padding px-4 md:px-8 py-4 md:py-6
Left side: Text logo "EcoVolta" (text-lg md:text-xl font-semibold), followed by a language selector showing a Globe icon (lucide Globe, 16x16) and "En" text
Center (hidden on smaller than lg): Navigation links -- "Renewables", "Strategies", "Photovoltaic", "Wind Systems", "Packages" -- each text-sm text-gray-700 hover:text-gray-900, spaced gap-8
Right side: "Sign In" link (hidden below sm) with a rounded-full border (border border-black/20) and padding px-4 md:px-6 py-2 md:py-2.5; "Clean Energy" button with bg-black text-white rounded-full same padding
HERO CONTENT (z-10, centered, flex-col items-center):

Container: flex-1 flex flex-col items-center px-4 md:px-8 pt-4 md:pt-8
Badge/Pill: Rounded-full pill with border border-black/20, containing emojis and text: sun -> planet earth -> "Delivering power innovate" -> seedling (exact: <span>sun emoji</span> <span>arrow</span> <span>earth globe emoji</span> <span>Delivering power innovate</span> <span>arrow</span> <span>seedling emoji</span>). On mobile, shorter text "Power innovate". Font size text-xs md:text-sm.

Main Heading: Uses a custom StaggeredFade animation component. Text: "Renewable Power For Tomorrow, Infinite Clean Solutions". Styling: text-3xl sm:text-4xl md:text-5xl lg:text-6xl leading-tight font-normal text-center max-w-5xl mb-3 md:mb-4. Color: #31463B (dark forest green). The animation reveals each letter individually with a staggered 0.03s delay per letter, 0.3s duration fade-in, triggered once when in view.

Subheading: Wrapped in FadeDown component (delay 0.5s). Text: "Sustainable Energy Platform. Engineering, deploying, and servicing solar arrays for homes, businesses, and large-scale operations worldwide." Styling: text-center text-gray-600 max-w-3xl mb-4 md:mb-5 text-sm md:text-base lg:text-lg.

CTA Buttons (FadeDown delay 0.7s): Two buttons side by side (column on mobile, row on sm+):

Primary "Explore Options": bg-gradient-to-r from-[#3C684D] to-[#4A7144] (green gradient), white text, rounded-full, contains a Leaf icon (16x16) on left, text, and a circular icon button on right (w-7 h-7 md:w-8 md:h-8) with gradient background linear-gradient(59deg, #567A5E 0%, #78A873 100%) containing a filled Play icon
Secondary "Start Network": White bg, text-gray-700, rounded-full, text on left, circular icon on right with gradient linear-gradient(59deg, #EEEEEE 0%, #CBCBCB 100%) containing a black ArrowRight icon
BOTTOM-LEFT ELEMENT (hidden below md, absolute bottom-24 left-8, z-10):

White rounded-lg box (40x40) with a MapPin icon in text-[#4A7C5A]
Below it: "4521 Sunvalley," (font-medium text-gray-900) and "Rd7, USA" (text-gray-600)
BOTTOM-CENTER ELEMENT (absolute bottom-20 md:bottom-24, z-10):

A "liquid glass" play button: 36x36 md:40x40 circle with glassmorphism effect (see CSS below), containing a filled white Play icon
Next to it: "Clean Power System" text in text-xs md:text-sm font-medium text-white
BOTTOM-RIGHT ELEMENT (hidden below lg, absolute bottom-48 right-8, z-10):

Three overlapping avatar circles (profile images):
Left: 40x40 circle, z-0, positioned left-0
Center: 64x64 circle with 4px white border, z-10, centered via left-1/2 -translate-x-1/2
Right: 40x40 circle, z-0, positioned right-0
Container: h-16 w-28 relative
Avatar image URLs:
Left: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260404_181959_c031059f-0b95-4099-89ca-105c74073dd7.png&w=1280&q=85
Center: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260404_181856_0904710c-03e6-460d-86ac-9acc0958001f.png&w=1280&q=85
Right: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260404_182114_826e4e5d-c7c6-425f-a72b-0410be243f72.png&w=1280&q=85
Text: "+ 37k Deployments" (text-sm font-medium)
Row of 5 icons below: RefreshCw, Square, PlusCircle, Grid2X2, Sparkles (all 20x20, text-black)
LOGO MARQUEE (bottom of page, z-10):

Padding pb-6 md:pb-8, overflow hidden
Infinitely scrolling list of company names: Retool, remote, ARC, Raycast, runway, ramp, HEX, Vercel, descript (duplicated for seamless loop)
Each name: text-gray-400 text-base md:text-xl font-medium whitespace-nowrap, gap gap-8 md:gap-16
CSS keyframe animation scroll translating from 0 to -50% on X axis, duration 15s on mobile / 30s on desktop, linear infinite
Left and right gradient fade overlays: w-16 md:w-32, gradient from #F5F3EE to transparent
LIQUID GLASS CSS (custom utility class .liquid-glass):


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
SCROLL KEYFRAME:


@keyframes scroll {
  0% { transform: translateX(0); }
  100% { transform: translateX(-50%); }
}
ANIMATION COMPONENTS:

StaggeredFade: Renders an <h1> using Framer Motion. Splits the text into individual letters. Each letter fades in (opacity: 0 -> 1) with a staggered delay of i * 0.03s and duration 0.3s. Triggered once when the element enters the viewport (useInView with once: true).

FadeDown: A Framer Motion wrapper div that animates from opacity: 0, y: -20 to opacity: 1, y: 0 with duration: 0.6s and a configurable delay. Triggered once when in view.

RESPONSIVE BREAKPOINTS:

Mobile-first design
sm (640px): CTA buttons go horizontal, "Sign In" visible, badge shows full text
md (768px): Larger text sizes, bottom-left address visible, spacing increases
lg (1024px): Desktop nav links visible, bottom-right avatars/deployments visible, heading reaches text-6xl

## EcoVolta V2 — Hero Section [sites/ecovolta-v2-hero]

- Preview: https://motionsites.ai/assets/hero-ecovolta-v2-preview-D8IVEFGK.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ecovolta-v2-hero.gif

Create a full-screen landing page hero section with the following exact specifications:

Video Background
HLS streaming video from Mux: https://stream.mux.com/02gzwandixH4J534bd00JsCvlFfw6ha101WQ00C9b3sGibM.m3u8
Video must autoplay, loop, be muted, and play inline
Use hls.js library for cross-browser compatibility with the following config:
capLevelToPlayerSize: false (allow quality higher than player size)
maxMaxBufferLength: 30, maxBufferLength: 20, maxBufferSize: 60MB
Switch to highest quality level once manifest loads
Native HLS support detection for Safari/iOS
Video positioned absolutely, covering full viewport with object-cover
Fonts
Import from Google Fonts:

Instrument Serif (Regular 400, Italic 400)
Manrope (Regular 400, Medium 500, SemiBold 600)
Instrument Sans (Regular 400, Medium 500, SemiBold 600)
SF Pro Display Medium for main heading
Layout & Positioning
Full viewport height container with overflow-hidden
Main content centered absolutely: left: 50%, top: calc(50% - 136.5px), transform translate(-50%, -50%)
Max width 984px with 24px horizontal padding
Navigation bar: absolute position at top, 20px from top, max-width 1110px, horizontally centered
Hero Content (Center)
Main Headline:

Text: "An AI that does your outbound while you " (regular) + "close deals." (gradient italic)
Regular portion: Instrument Serif Regular, #212121, 48px mobile / 70px desktop
"close deals" portion: Instrument Serif Italic with radial gradient text effect
Gradient: Radial from blue (#368CFB at 0%, #5CAEFE at 30%, #85BDE0 at 47.5%, #AECDC2 at 65%, #D6DCA3 at 82.5%, #FFEB85 at 100%)
SVG gradient transform: matrix(35.22 -11.4 433.41 134.85 369.8 114)
Leading: 1.1 mobile, 64px desktop
Max width: 722px
Opacity: 0.9
Subheadline:

Text: "AI sales agent that finds leads, personalizes outreach, follows up, and books meetings — automatically."
Font: Manrope Regular, 18px mobile / 20px desktop
Gradient text: from rgba(37,44,50,0.7) to rgba(55,65,74,0.7)
Letter spacing: -0.4px
Opacity: 0.7
Max width: 510px
Gap above: 24px mobile / 32px desktop
CTA Button:

Text: "Get started" (Instrument Sans Medium, 16px, white)
Size: 152px × 52px, rounded 12px
Background: Linear gradient from #444 to #292929
Border: 1px solid black
Shadows:
Outer: 0px 4px 4px rgba(0,0,0,0.25), 0px 1px 2px rgba(0,0,0,0.31)
Inner: inset 0px 2px 1px rgba(255,255,255,0.51), inset 1px 1px 0.25px rgba(255,255,255,0.3)
Hover: opacity 90%, smooth transition
Gap above: 32px mobile / 48px desktop
Navigation Bar (Top)
Logo (Left):

Icon: 23×23px SVG with radial gradient (blue to yellow: #368CFB → #5CAEFE → #FFEB85)
Text: "closer" in Instrument Serif Regular, 26px, #212121
Gap between icon and text: 6px
Nav Links (Center, hidden on mobile):

Links: "Product", "How it works", "Pricing", "Customers", "Docs"
Font: Manrope Medium, 18px
Gradient text: from rgba(37,44,50,0.7) to rgba(55,65,74,0.7)
Gap between links: 16px mobile / 26px large screens
Hidden below md breakpoint
Login Button (Right):

Text: "Login" (Instrument Sans Medium, 18px, #212121)
Size: 108px × 46px, rounded 12px
Background: white with 1px border #dde2e4
Hover: slight gray background, smooth transition
Responsive Behavior
Mobile: Single column, 48px headline, 18px body, tighter gaps (24px/32px)
Desktop: 70px headline, 20px body, wider gaps (32px/48px)
Navigation collapses on mobile (hide center nav)
Percentage-based widths with max-width constraints
Leading adjustments: tighter on mobile (1.1), fixed on desktop (64px for headline)
Technical Requirements
React + TypeScript + Tailwind CSS v4
Install: hls.js package
All elements fully responsive
Smooth hover transitions (opacity, colors)
Proper z-indexing (video behind, nav on top)
Cross-browser video compatibility with error recovery

## EMBER.dsgn — Hero Section [sites/ember-dsgn-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(85).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ember-dsgn-hero.webp

Recreation Prompt
Build a fullscreen split-panel hero section for "EMBER.dsgn" — a digital design studio. Single-page React app, no routing.

Stack & dependencies
React 19 + TypeScript + Vite 6
Tailwind CSS 4 via @tailwindcss/vite
motion (framer-motion v12 successor — import from motion/react)
lucide-react for icons (ArrowUpRight, Menu, X)
hls.js for HLS video playback
Vite dev script: vite --port=3000 --host=0.0.0.0
Global styles (src/index.css)
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800;900&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
}

body {
  @apply antialiased overflow-hidden;
}
Root container


 with: relative h-screen w-full font-sans text-white selection:bg-white/20 overflow-hidden bg-black. Three stacked layers inside (z-index order: video bg → split panels → nav → mobile menu overlay).

1. Background video (z-0)
Absolute, inset-0, full-cover
HLS stream URL: https://stream.mux.com/Q3hYHAcLU82ceOUgwDeO4HiwOc3WZn9JD02PugwzxHOI.m3u8
 attrs: muted loop playsInline autoPlay, classes absolute inset-0 w-full h-full object-cover scale-x-[-1] (horizontally mirrored)
In useEffect: if Hls.isSupported(), create new Hls(), loadSource, attachMedia, on MANIFEST_PARSED set playbackRate = 0.7 and call .play(). Cleanup with hls.destroy(). Safari fallback: set video.src directly and use loadedmetadata listener with same playback rate.
2. Split panels (z-10)
Outer wrapper: absolute inset-0 flex flex-col lg:flex-row z-10 pointer-events-none overflow-y-auto lg:overflow-hidden scrollbar-hide

Left panel — "EMBER" cutout effect
relative w-full lg:w-1/2 min-h-screen lg:h-full flex flex-col pointer-events-auto overflow-hidden border-b lg:border-b-0 lg:border-r border-white/5
Blur layer: absolute inset-0, backgroundColor: rgba(131, 131, 131, 0.3), backdropFilter: blur(20px) (+ webkit prefix), with maskImage: url(#emberMask) so the EMBER letters cut a clear hole through the blur revealing the video.
SVG mask def:  with . White rect 100%×100%, then black-fill EMBER text in two responsive variants:
Mobile: 
Desktop: 
Text element: x=0 y=115 textLength="100%" lengthAdjust="spacingAndGlyphs", classes font-[900] tracking-tighter, inline fontSize: 130px, fill="black"
Content stack (z-20, pt-[12vh] lg:pt-[8vh] px-6 md:px-12):
Spacer matching the EMBER SVG: h-[20vh] lg:h-[25vh]
Vertical line: flex-grow flex flex-col pt-4 min-h-[100px] containing w-[1px] h-full bg-white/20
Footer block (pb-12 flex flex-col gap-6 pt-4):
"ABOUT" eyebrow: text-[10px] font-bold tracking-[0.3em] uppercase text-white/40
Heading: text-xl md:text-2xl font-normal leading-[1.3] text-white/90 — copy: "We shape striking digital identities through bold contrasts and meaningful motion." 
 "Our design process transforms the primal into the powerful."
Bottom row: flex flex-col sm:flex-row justify-between items-start sm:items-end border-t border-white/10 pt-8 w-full gap-8 — three cells:
"Double Click and" caption + Explore Our Work link with 
Social links (Instagram, Telegram) — each with a w-1 h-1 bg-white rounded-full opacity-50 bullet
Address (hidden on mobile): 23 Industrial Lane, Unit 5 / London, UK, E2 8AA
All small-text uses text-[10px] font-bold uppercase tracking-widest; eyebrows use text-[9px]
Right panel — "STUDIO" word
relative w-full lg:w-1/2 min-h-[50vh] lg:h-full flex flex-col justify-end pb-8 lg:pb-2 pointer-events-auto overflow-hidden
Two concentric circles (decorative, centered, z-0): wrapper absolute inset-0 z-0 pointer-events-none flex justify-center → inner relative h-full aspect-square flex flex-col items-center containing two divs:
Circle 1: absolute top-[-10vh] lg:top-[-25vh] w-[40vh] lg:w-[60vh] h-[40vh] lg:h-[60vh] border border-white/20 lg:border-white/35 rounded-full
Circle 2: same size, top-[30vh] lg:top-[18vh]
STUDIO wordmark (z-10, relative w-full mb-1 px-6 md:px-[5%]):  with STUDIO
3. Navigation (z-50, fixed)
fixed top-0 left-0 w-full z-50 flex items-center justify-between px-6 md:px-12 py-6 lg:py-8 pointer-events-none — two pointer-events-auto groups.

Left group:

Logo: 2×2 grid of w-2 md:w-2.5 h-2 md:h-2.5 bg-[#FF5C35] squares (gap-0.5) + EMBER.dsgn text (text-lg md:text-xl font-black tracking-tighter)
Desktop nav links (hidden lg:flex items-center gap-6 text-[10.5px] uppercase font-medium tracking-[0.2em] text-white/70): WORKS, SERVICES, ABOUT, TEAM — each with a w-1 h-1 bg-white rounded-full opacity-50 bullet, hover:text-white transition-colors
Right group:

Language pill (hidden sm:flex border border-white/20 rounded-full px-4 py-1.5 text-[10.5px] font-medium tracking-widest uppercase items-center gap-3 bg-white/5 backdrop-blur-sm): EN | RU with separator at text-white/20
Mobile burger button (lg:hidden p-2) with , opens menu
Contacts pill (hidden sm:block border border-white/20 rounded-full px-6 py-2 text-[10.5px] font-medium tracking-widest uppercase hover:bg-white hover:text-black transition-all bg-white/5 backdrop-blur-sm) with bullet + CONTACTS
4. Mobile menu overlay
State: isMenuOpen (useState boolean). Wrap in ; render  with:

initial={{ opacity: 0, x: "100%" }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: "100%" }}
transition={{ type: "spring", damping: 25, stiffness: 200 }}
Classes: fixed inset-0 z-[100] bg-black pointer-events-auto lg:hidden flex flex-col
Layout:

Header row: same logo + close button ()
Center links (flex-grow flex flex-col justify-center px-12 gap-8): map ['WORKS', 'SERVICES', 'ABOUT', 'TEAM', 'CONTACTS'] to  with initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 + i * 0.05 }}, classes text-4xl font-bold tracking-tighter hover:text-[#FF5C35] transition-colors
Footer (p-12 border-t border-white/10 flex justify-between items-center): EN | RU toggle + UKRAINE / LONDON label
Color tokens
Brand orange: #FF5C35
All whites use opacity variants: white/5 white/10 white/20 white/35 white/40 white/70 white/90
Page background: pure black
Behavior summary
Video plays muted, looped, mirrored, at 0.7× speed
EMBER letters appear as a clear-glass cutout in a 20px backdrop-blur layer
STUDIO is solid white wordmark
Both wordmarks scale to fill their column via SVG textLength="100%" lengthAdjust="spacingAndGlyphs" with Inter weight 900
Mobile (

## EVR Ventures — Hero Section [sites/evr-ventures-hero]

- Preview: https://motionsites.ai/assets/hero-evr-ventures-preview-DZxeVFEX.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/evr-ventures-hero.gif

Create a hero section with the following exact specifications:

Fonts:

Body/Sans: 'Geist', sans-serif

Display: 'Gilda Display', serif

Color Scheme (dark theme — black background, white foreground):

--background: 0 0% 0% (pure black)

--foreground: 0 0% 100% (pure white)

Video Background:

Full-screen looping muted video with autoPlay, muted, loop, playsInline

URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_024928_1efd0b0d-6c02-45a8-8847-1030900c4f63.mp4

object-cover with horizontal offset object-[37%_center] — no dark overlay

Positioned absolute inset-0 with z-0

Navigation Bar (z-10, relative):

Left: A "Menu" button — rounded-full pill with a border (border-foreground/30), uppercase tracking-widest text, and two horizontal lines (hamburger icon made of two <span> bars, w-7 h-[2px] bg-foreground, gap 4px)

Center: Logo text "EVR" — absolute left-1/2 -translate-x-1/2, text-2xl font-bold tracking-wider text-foreground

Right (hidden on mobile, visible md:flex): Two pill buttons ("About Us", "Services") with border border-foreground/30 rounded-full styling, plus a "Get Started" CTA button with bg-gradient-to-r from-[hsl(220,70%,78%)] to-[hsl(40,80%,82%)], black text, rounded-full, uppercase

Full-Screen Menu Overlay (AnimatePresence + framer-motion):

Triggered by Menu button, uses useState for menuOpen

Animated with clipPath: "circle(0% at 80px 40px)" → "circle(150% at 80px 40px)" on open, reverse on close

Duration 0.7s, ease [0.76, 0, 0.24, 1]

Background: bg-foreground (white), fixed inset-0 z-50

Close button: same pill style as Menu but with X icon, text-background (black text)

Center logo "EVR" in text-background

Menu links: Home, About Us, Services, Projects, Contact — each animated with opacity: 0, x: -60 → opacity: 1, x: 0, staggered delay 0.15 + i * 0.08, ease [0.25, 1, 0.5, 1]

Each link: text-[clamp(2rem,5vw,4.5rem)] font-light -tracking-[0.06em], with ArrowRight icon on right, separated by border-b border-background/10

Hover: text shifts right 4px, arrow shifts right 2px

Bottom: "Evolve Responsible Ventures" left, "© 2026" right, both text-background/40 text-xs tracking-[0.2em] uppercase

Body scroll locked when menu is open

Main Content Area (z-10, relative):

Container: flex-1 flex flex-col with justify-start pt-6 px-6 pb-2 on mobile, justify-end pt-0 px-10 pb-16 on md:

Subheading row: ArrowRight icon (w-4 h-4) + "Evolve Responsible Ventures" in text-xs font-medium tracking-[0.25em] uppercase

Below that, a flex container: column on mobile (heading top, stats bottom), row on lg: (side by side at bottom)

Heading:

Navigating the
route to impactful
regeneration

Each line: text-[clamp(2rem,6vw,5rem)], first two lines font-light, third line font-display (Gilda Display serif)

leading-[0.9] -tracking-[0.2em] on the <h1>

Stats/Progress Circle (right side on desktop, bottom on mobile):

lg:max-w-xs lg:pb-4, with mt-8 on mobile, mt-0 on md:

SVG circular progress: 120x120 viewBox, radius 54, strokeWidth 3

Background circle: stroke="hsl(var(--foreground) / 0.15)"

Progress circle: stroke="hsl(var(--foreground))", animated to 75% on mount (500ms delay), strokeLinecap="round", transition-all duration-1000 ease-out

Center text: "75%" in text-foreground text-lg font-medium

Below circle: paragraph text-foreground/70 text-sm leading-relaxed: "Guiding organizations toward lasting environmental performance through actionable strategy and measurable outcomes"

Clients/Partners Marquee Bar (bottom, z-10):

Top row: "Our Partners" left, "Backed by 30+ global brands" right (hidden on mobile)

Both text-xs font-medium tracking-[0.2em] uppercase text-foreground

Below: border-t border-foreground/10, overflow-hidden py-5

Marquee: CSS animate-marquee (keyframe translateX(0) → translateX(-50%), 20s linear infinite)

Brand names: Opensense, DKNY, Under Armour, LIU·JO, ATOM, ECCO, ORUM — duplicated twice for seamless loop

Each name: text-foreground/50 text-lg font-medium tracking-wide, gap-16

Responsive Layout Summary:

Mobile: heading at top of content area, stats/progress at bottom, nav right buttons hidden

Tablet/Desktop (md:+): content aligned to bottom, nav right buttons visible

Large (lg:): heading and stats side by side at bottom

## Impressive Hero — Hero Section [sites/impressive-hero]

- Preview: https://motionsites.ai/assets/hero-impressive-preview-BCJtlSs2.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/impressive-hero.gif

Create a full-screen hero section in React + TypeScript + Tailwind CSS (Vite) with a fullscreen background video, a floating "liquid glass" navigation bar, an animated character-by-character heading, and a bottom liquid-glass tagline pill.

Stack / Setup:

React 18 + TypeScript + Vite
Tailwind CSS (default config, no theme extensions)
No extra libraries (no framer-motion, no icon libs needed for this section)
Font: 'Helvetica Neue', Helvetica, Arial, sans-serif, weight 400, antialiased
Global CSS (src/index.css):


@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-weight: 400;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.liquid-glass {
  background: rgba(0, 0, 0, 0.4);
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
    rgba(255,255,255,0.3) 0%, rgba(255,255,255,0.1) 20%,
    rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.1) 80%, rgba(255,255,255,0.3) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}
Components:

AnimatedHeading — splits text into characters, each <span> is inline-block with transition-all duration-500. Starts at opacity: 0, translateX(-18px) and animates to opacity: 1, translateX(0) with a per-character transitionDelay of lineIndex * line.length * charDelay + charIndex * charDelay ms. Supports multi-line via \n split, each line wrapped in <div className="flex flex-wrap justify-center">. Starts after delay ms via setTimeout. Preserves spaces using \u00A0.

FadeIn — wraps children in a div with transition-opacity, toggles opacity 0 -> 1 after delay ms, transitionDuration configurable (default 800ms).

Hero layout (App.tsx):

Root: min-h-screen bg-black text-white relative

Background <video autoPlay loop muted playsInline> absolutely positioned, inset-0 w-full h-full object-cover, with source:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_084718_72a17915-4964-4059-afcd-22d59399b72e.mp4 (type video/mp4)

Content wrapper: relative z-10 min-h-screen

Navbar (absolutely positioned top, padding px-6 md:px-12 lg:px-16 pt-6): a .liquid-glass rounded-xl bar, flex items-center justify-between px-4 py-2:

Left: <div className="text-2xl font-semibold tracking-tight">VEX</div>
Center (hidden on mobile, md:flex gap-8 text-sm): links "Story", "Investing", "Building", "Advisory" with hover:text-gray-300 transition-colors
Right: white pill button "Start a Chat" — bg-white text-black px-6 py-2 rounded-lg text-sm font-medium hover:bg-gray-100
Center block (min-h-screen px-6 md:px-12 lg:px-16 flex flex-col items-center justify-center text-center, inner w-full max-w-4xl flex flex-col items-center):

AnimatedHeading with text "Shaping tomorrow\nwith vision and action.", classes text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-normal mb-4, inline style letterSpacing: '-0.04em', delay={200}, charDelay={30}
FadeIn delay={800} duration={1000}: <p className="text-base md:text-lg text-gray-300 mb-5">We back visionaries and craft ventures that define what comes next.</p>
FadeIn delay={1200} duration={1000}: two buttons in flex flex-wrap gap-4 justify-center:
Primary: "Start a Chat" — bg-white text-black px-8 py-3 rounded-lg font-medium hover:bg-gray-100 transition-colors
Secondary: "Explore Now" — liquid-glass border border-white/20 text-white px-8 py-3 rounded-lg font-medium hover:bg-white hover:text-black transition-colors
Bottom tagline (absolutely positioned, bottom-0 left-0 right-0 px-6 md:px-12 lg:px-16 pb-12 lg:pb-16 flex justify-center): FadeIn delay={1400} duration={1000} wrapping a .liquid-glass border border-white/20 px-6 py-3 rounded-xl containing <p className="text-lg md:text-xl lg:text-2xl font-light">Investing. Building. Advisory.</p>

Animation timeline:

200ms: heading characters start animating left-to-right, 30ms per character, 500ms each
800ms: subheading fades in over 1000ms
1200ms: CTA buttons fade in over 1000ms
1400ms: bottom tagline fades in over 1000ms
Responsive breakpoints: mobile-first, nav links hidden below md, heading scales from text-4xl up to xl:text-7xl.

## Luminex — Hero Section [sites/luminex-hero]

- Preview: https://motionsites.ai/assets/hero-luminex-preview-CxOP7ce6.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luminex-hero.gif

Create a modern React landing page with a full-screen HLS video background, glassmorphic navigation header, and hero content positioned in the bottom-left corner.

## Nexar — Hero Section [sites/nexar-hero]

- Preview: https://motionsites.ai/assets/hero-nexar-preview-Dk7ThCat.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexar-hero.gif

PROMPT:

Build a project management dashboard called "Nexar" using React, TypeScript, Tailwind CSS, Vite, and Lucide React icons. The design should have a fullscreen looping background video, a white pill-shaped header, a 3-column responsive grid layout, and task cards with staggered fade-up animations. Use Google Fonts "Instrument Serif" for display/serif text.

BACKGROUND:

A fullscreen looping .mp4 video plays behind all content. The video is fixed position, covers the entire viewport via object-cover, and sits at z-index: -10.

Video URL:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260403_054410_6b17f7f9-d11e-44f1-90b0-75ee563d1971.mp4
Attributes: autoPlay, loop, muted, playsInline.

FONTS:

Load "Instrument Serif" (regular + italic) from Google Fonts in index.html:


<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet">
Define a utility class .font-serif-display in index.css with font-family: 'Instrument Serif', serif;. Use this class for the brand name "nexar", the greeting "Hey, Alex!", and the "Speak now to Nexar!" heading.

ANIMATIONS:

Custom fadeUp keyframe animation defined in index.css:

From: opacity: 0; transform: translateY(24px)
To: opacity: 1; transform: translateY(0)
Timing: 0.8s cubic-bezier(0.16, 1, 0.3, 1) forwards
Class: .animate-fade-up (starts with opacity: 0)
Each element has a staggered animationDelay via inline style prop, starting at 0s for the header and incrementing by ~0.05-0.1s per element (0s, 0.1s, 0.15s, 0.2s, 0.25s, 0.3s, 0.35s, 0.4s, 0.45s, 0.5s, 0.55s, 0.6s).
LAYOUT:

Root container: min-h-screen relative p-4 sm:p-6 lg:p-8 overflow-x-hidden
Max width wrapper: max-w-[1800px] mx-auto
Main grid: flex flex-col lg:grid lg:grid-cols-12 gap-4 sm:gap-6 lg:gap-7
Left column: lg:col-span-3
Center column: lg:col-span-6 with inner width constrained to lg:w-[85%] xl:w-[85%] 2xl:w-[60%] centered
Right column: lg:col-span-3
HEADER (white pill bar):

Full-width white rounded-full bar (bg-white rounded-full px-4 sm:px-6 py-2 sm:py-3 shadow-sm) containing:

Left: A 5x5/6x6 black rounded-md square with a 2x2 grid of tiny white rounded dots inside, followed by the word "nexar" in text-2xl sm:text-3xl font-serif-display
Center nav (hidden on mobile, hidden lg:flex): Links "Workspace" (active, black, font-medium), "Actions", "Performance", "AI Insights" (gray-500, hover gray-900)
Right: A toggle pill (bg-gray-100 rounded-full) with "Solo" (inactive, gray) and "Crew" (active, bg-black text-white rounded-full) buttons, plus a black circle notification bell icon (filled white)
SECTION HEADER ROW (below header):

A row with border-b border-black/10 pb-4 sm:pb-6 mb-4 sm:mb-6 containing:

Left (col-span-3): Orange-red gradient circle avatar (bg-gradient-to-br from-red-400 to-orange-500) with white User icon inside, plus "Hey, Alex!" in text-[28px] sm:text-[36px] lg:text-[42px] font-serif-display
Center (col-span-6): "Active Items" text in text-[20px] sm:text-[24px] lg:text-[26px] tracking-[-0.04em]
Right (col-span-3): "Crew:" label, 3 overlapping circular team member avatar images (-space-x-2), and "+9" counter
Team member avatar image URLs:


https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260403_075317_744395c6-7168-48c6-a1f6-5b9b7bd58f87.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260403_075333_2caea84e-742e-4846-9284-ed8532c44c99.png&w=1280&q=85

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260403_075354_70a33cfd-3c9c-45ef-a7bb-d371cb8aa0af.png&w=1280&q=85
LEFT COLUMN:

Project Selector Pill: Light blue (bg-[#DBECFC]) rounded-full pill with a white circle containing a yellow X icon (two crossing lines, stroke #EAB308, strokeWidth 3.5), title "Zenith Launch", subtitle "Product & Strategy", and a ChevronDown icon.

Productivity Score: Giant number "85%" in text-[80px] sm:text-[100px] lg:text-[120px] xl:text-[140px] tracking-[-0.04em] with "Current efficiency" subtitle below.

Sprint Metrics Card: Rounded card (rounded-[20px] sm:rounded-[28px]) with a background image:


https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260403_055416_630ff6c1-4b72-4cb6-a563-0c7e41124fe1.png&w=1280&q=85
(cover, top center). Contains "Sprint Metrics" header with an "Analytics" pill badge, three stat columns: "26h / Sessions", "11h / Standups", "6h / Audits". Has an absolutely positioned white floating circle button at bottom center (-bottom-4 left-1/2 -translate-x-1/2) containing a pencil/edit SVG icon.

CENTER COLUMN (3 TaskCards stacked vertically):

Create a reusable TaskCard component in src/components/TaskCard.tsx. It accepts props: icon (Lucide icon component), title, tagText, tagColor ('green'|'yellow'|'red'), details (array of {label, value}), bottomLeftContent (ReactNode), buttonText, buttonVariant ('dark'|'light'|'black'), buttonIcon (optional ReactNode).

TaskCard layout: White rounded card (bg-white rounded-[20px] sm:rounded-[28px] px-4 sm:px-6 py-4 sm:py-5 shadow-sm) with 3 rows separated by border-b border-black/10:

Row 1: Icon + title (left) and colored pill tag (right)
Row 2: 3-column detail grid (label in gray-500, value in gray-900 font-medium). Third detail column is narrower (flex-[0.5] max-w-[120px]).
Row 3: Custom bottom-left content + action button
Tag colors: green = bg-green-500 text-white, yellow = bg-yellow-400 text-gray-900, red = bg-red-500 text-white.

Button variants: dark = bg-[#ECECEC] text-gray-900, black = bg-black text-white, light = bg-gray-100 text-gray-900.

Card 1 - "Sprint Planning Call":

Icon: Phone, Tag: "Session" (green)
Details: Time: "Today: 10:00 AM", With: "Product & Growth", Alert: "15 min"
Bottom: 3 overlapping avatar images + "+7" + "Set to begin?" text
Button: "Enter session" (dark variant)
No rotation
Card 2 - "Layout Critique":

Icon: BarChart2, Tag: "Action" (yellow)
Details: Focus: "Zenith Platform", Details: "Verify the layout of landing screen", Due By: "Mar 22"
Bottom: "Assignees:" label + 2 overlapping avatar images
Button: "Let AI begin" (black variant) with Sparkles icon
Rotated 2 degrees clockwise (rotate-[2deg])
Card 3 - "Zenith Crew Check":

Icon: Phone, Tag: "Session" (green)
Details: Time: "Fri: 5:30 PM", With: "Sales Lead & Team", Alert: "10 min"
Bottom: 3 overlapping avatar images + "+5" + "Scheduled" text
Button: "Show details" (light variant)
No rotation
RIGHT COLUMN:

Fast Commands List: Title row with star emoji in white circle + "Fast commands" heading (text-[20px] sm:text-[24px] lg:text-[26px] xl:text-[30px] tracking-[-0.04em]) + "+ Add Item" button. Three list items separated by border-t border-black/10, each with description text and a filled Play icon (Lucide Play with fill-gray-700):

"Review session notes and extract key discussion insights"
"Generate PDF report with finished items from this week"
"Update timeline view based on revised action items in sprint"
Voice Input Card: Light blue (bg-[#DBECFC]) rounded card with "Audio Input" blue pill badge (bg-blue-500 text-white), "Speak now to Nexar!" heading in font-serif-display, and a waveform visualization made of 35 thin vertical bars (w-0.5 bg-blue-400 rounded-full) with varying heights: [8, 16, 12, 28, 20, 36, 42, 24, 40, 16, 44, 32, 48, 28, 20, 36, 14, 32, 22, 40, 18, 30, 12, 26, 16, 34, 20, 38, 24, 28, 16, 22, 12, 20, 8] each multiplied by 0.8 for pixel height. Has a floating white circle microphone button at bottom center (same positioning as Sprint Metrics card).

DEPENDENCIES:

react, react-dom (^18.3.1)
lucide-react (^0.344.0) -- icons used: ChevronDown, User, ChevronRight, Mic, Phone, Bell, Play, BarChart2, Sparkles
@supabase/Bolt Database-js (^2.57.4)
tailwindcss (^3.4.1), postcss, autoprefixer
Vite (^5.4.2), @vitejs/plugin-react
TypeScript
Vite config: Exclude lucide-react from optimizeDeps.

Tailwind config: Default with content scanning ./index.html and ./src/**/*.{js,ts,jsx,tsx}.

## Nexus IT Solutions — Hero Section [sites/nexus-hero]

- Preview: https://motionsites.ai/assets/hero-nexus-preview-74RfhYpA.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexus-hero.gif

Create a full-viewport (100vh) hero landing page using React, Tailwind CSS, and TypeScript. Load Google Fonts: Akshar (400–700) and Inter (400–700) via <link> in index.html.

Structure: The page is a single div with min-h-screen bg-background containing a h-screen flex flex-col relative overflow-hidden wrapper. Inside: Background video absolutely positioned behind everything, Content wrapper (relative z-10 flex flex-col flex-1) containing Navbar, Hero content (flex-1, vertically centered), Trusted By section.

Background Video: <video className="absolute inset-0 w-full h-full object-fill" src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_065810_0098b193-498c-4d26-9abd-5db8bf4fb479.mp4" autoPlay muted loop playsInline /> object-fill stretches to fill the full viewport — no cropping, no overlay.

Navbar: Container flex items-center justify-between px-8 py-5 max-w-7xl mx-auto w-full gap-12. Logo: <NEXUS> styled font-akshar text-xl font-medium tracking-wider text-foreground. Nav links: HOME, SOLUTIONS, OUR TEAM, NEWS — text-lg tracking-[0.05em] text-muted-foreground hover:text-foreground font-akshar gap-8. CTA: "GET IN TOUCH" border border-foreground/10 text-muted-foreground hover:bg-muted-foreground hover:text-background uppercase tracking-[0.05em] text-xl rounded-none font-akshar.

Hero Content: H1 "We drive companies beyond their biggest obstacles" text-4xl md:text-5xl lg:text-6xl font-normal leading-tight max-w-3xl letterSpacing -0.06em heading-gradient. Subheading: "Accelerating Growth through IT Strategy, Digital Innovation, and Custom-Built Technology Platforms" mt-6 text-muted-foreground text-lg md:text-xl max-w-xl font-akshar. CTA: NotchedButton "START YOUR JOURNEY" with corner decorations (8 spans, 10px long x 1px, inset 4px from edges).

Trusted By: "Trusted by leading innovators worldwide" uppercase tracking-[0.12em]. Brand names: FedEx, amazon, McKESSON, pitney bowes — text-lg md:text-2xl font-bold tracking-wide opacity-40.

CSS tokens: --background: 0 0% 100%; --foreground: 220 20% 20%; --primary: 212 72% 18%; --muted-foreground: 220 10% 50%; --heading-gradient-from: 212 72% 10%; --heading-gradient-to: 205 65% 48%. heading-gradient class uses linear-gradient with background-clip text.

## Portal — Hero Section [sites/portal-hero]

- Preview: https://motionsites.ai/assets/hero-portal-preview-DEscBr2T.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/portal-hero.gif

PROMPT:

Build a full-viewport cinematic movie/streaming hero section using React, Tailwind CSS, and Lucide React icons. Use the Inter font from Google Fonts. The entire page is a single full-height hero -- no scrolling, no additional sections.

BACKGROUND VIDEO:

A full-screen background video plays on loop, muted, autoplaying, covering the entire viewport with object-cover. The video is fixed-positioned behind everything at z-index 0.

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260406_094145_4a271a6c-3869-4f1c-8aa7-aeb0cb227994.mp4

BOTTOM BLUR OVERLAY (no gradient darkening):

Over the video, there is a single fixed, full-screen overlay div that applies a strong backdrop-blur-xl. This div uses a CSS mask so the blur only appears at the bottom and fades to transparent toward the middle of the screen. There is NO dark gradient overlay -- only blur.

The mask: mask-image: linear-gradient(to top, black 0%, transparent 45%) (with the -webkit- prefix too).

This overlay is pointer-events-none and sits at z-index 1.

FONT:

Import Inter from Google Fonts (weights 300-700). Set font-family: 'Inter', sans-serif on the body.

LIQUID GLASS EFFECT (used on multiple buttons):

Create a reusable .liquid-glass CSS class with these exact properties:

background: rgba(255, 255, 255, 0.01) with background-blend-mode: luminosity
backdrop-filter: blur(4px) (with -webkit- prefix)
border: none
box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1)
position: relative; overflow: hidden
A ::before pseudo-element that creates a thin glowing border effect:
position: absolute; inset: 0; border-radius: inherit; padding: 1.4px
background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)
Uses -webkit-mask with linear-gradient(#fff 0 0) content-box and linear-gradient(#fff 0 0) combined with -webkit-mask-composite: xor and mask-composite: exclude to create a border-only gradient stroke
pointer-events: none
BLUR-FADE-UP ANIMATION (used on every element with staggered delays):

Create a @keyframes blurFadeUp animation:

From: opacity: 0; filter: blur(20px); transform: translateY(40px)
To: opacity: 1; filter: blur(0); transform: translateY(0)
The .animate-blur-fade-up class applies this as animation: blurFadeUp 1s ease-out forwards with initial opacity: 0. Each element on the page gets a staggered animationDelay via inline style.

NAVBAR (z-index 50, relative positioned):

A horizontal navbar with justify-between, padding px-4 sm:px-6 md:px-12 py-4 md:py-6.

Left: A text logo (e.g. your brand name like "CINEMATIC" or similar) styled as h-8 md:h-10, with blur-fade-up animation at delay 0ms.

Center (desktop only, hidden below lg): Navigation links -- "Movies", "TV Series", "Editor's Pick", "Interviews", "User Reviews" -- each as an anchor with text-sm, hover:text-gray-300 transition-colors, and staggered blur-fade-up delays from 100ms to 300ms (50ms increments).

Right: Two buttons visible on sm and up:

A "Search" button -- rounded-full liquid-glass pill with the text "Search" and a Lucide Search icon (size 18), padding px-4 md:px-6 py-2, blur-fade-up at 350ms.
A user/profile circle button -- w-10 h-10 rounded-full liquid-glass with a Lucide User icon (size 18), blur-fade-up at 400ms.
A hamburger menu button visible only below lg -- w-10 h-10 rounded-full liquid-glass with animated icon transition between Lucide Menu and X icons. The transition uses rotate-180, opacity, and scale-50 with duration-500 ease-out. Blur-fade-up at 350ms.
MOBILE MENU (below lg breakpoint):

An absolutely positioned dropdown below the navbar (top-[72px]), z-index 40. It slides in with translate-y-0 opacity-100 when open, -translate-y-4 opacity-0 pointer-events-none when closed, duration-500 ease-out.

Background: bg-gray-900/95 backdrop-blur-lg with border-t border-b border-gray-800 shadow-2xl.
Contains the same 5 nav links, each in a column with py-3 px-3 rounded-lg, hover:bg-gray-800/50, and staggered slide-in animations (translate-x based, 50ms delay increments).
Below sm, also shows Search and Profile buttons in a bordered section at the bottom.
HERO CONTENT (bottom of viewport):

A flex container that grows to fill remaining space and aligns content to the bottom (flex-1 flex flex-col justify-end), with padding px-4 sm:px-6 md:px-12 pb-8 md:pb-16, z-index 10.

Inside, a flex-col md:flex-row items-end gap-8 layout:

Left side (flex-1):

Metadata row -- a horizontal flex-wrap row with gap-3 sm:gap-6 mb-6 md:mb-8 text-xs sm:text-sm, blur-fade-up at 300ms:

Star icon (size 16, fill-white, responsive to sm:w-5 sm:h-5) + "8.7/10 IMDB" (font-medium)
Clock icon (size 16) + "132 min"
Calendar icon (size 16) + "April, 2025"
Title -- text-3xl sm:text-5xl md:text-6xl lg:text-7xl font-normal, letter-spacing -0.04em, mb-4 md:mb-6, blur-fade-up at 400ms. Text: "Step Through. Work Smarter."

Description -- text-base sm:text-lg md:text-xl text-gray-400 mb-6 md:mb-12 max-w-2xl, blur-fade-up at 500ms. Text: "A voyage through forgotten realms, where past and future intertwine."

CTA buttons -- flex-wrap row with gap-3 sm:gap-4:

"Watch Now" -- bg-white text-black rounded-full font-medium, px-6 sm:px-8 py-2.5 sm:py-3, with a Lucide Play icon (size 18, fill-black), hover:bg-gray-200, blur-fade-up at 600ms.
"Learn More" -- rounded-full font-medium liquid-glass, same padding, blur-fade-up at 700ms.
Right side (navigation arrows):

A row of two pill buttons (md:w-auto, aligned right on desktop, left on mobile):

"Previous" button -- rounded-full liquid-glass, px-4 sm:px-6 py-2.5 sm:py-3, with Lucide ChevronLeft icon, blur-fade-up at 800ms.
"Next" button -- same styling with Lucide ChevronRight icon, blur-fade-up at 900ms.
COLOR PALETTE:

Background: pure black (bg-black)
Text: white, with text-gray-400 for the subtitle
All interactive glass elements use the .liquid-glass class (nearly transparent white with blur)
The only solid-colored element is the "Watch Now" button (white background, black text)
STAGGER TIMING SUMMARY:

Logo: 0ms
Nav links: 100ms, 150ms, 200ms, 250ms, 300ms
Search button: 350ms
User button: 400ms
Metadata row: 300ms
Title: 400ms
Description: 500ms
Watch Now: 600ms
Learn More: 700ms
Previous: 800ms
Next: 900ms
RESPONSIVE BREAKPOINTS:

Below sm (< 640px): Smaller text, tighter padding, Search/User buttons hidden (available in mobile menu)
Below lg (< 1024px): Nav links hidden, hamburger menu shown
md and up: Side-by-side layout for hero content and navigation arrows
lg and up: Full desktop navbar with all links visible

## Power AI — Hero Section [sites/power-ai-hero]

- Preview: https://motionsites.ai/assets/hero-power-ai-preview-BqpSbx41.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/power-ai-hero.gif

Create a full-screen dark hero section with a looping background video, navbar, headline, subtitle, CTA button, and a logo marquee at the bottom. Here are the exact specifications:

Theme & Colors (index.css CSS variables):
Background: 260 87% 3% (deep dark blue-purple)
Foreground: 40 6% 95% (off-white)
Hero sub text: 40 6% 82%
Body font: Geist Sans (via @fontsource/geist-sans)
Headline font: General Sans (loaded from Fontshare: https://api.fontshare.com/v2/css?f[]=general-sans@400,500,600,700&display=swap)

Background Video (Index page wrapper):
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_065045_c44942da-53c6-4804-b734-f9e07fc22e08.mp4
Positioned absolute inset-0 w-full h-full object-cover behind all content
Starts with opacity: 0
Custom JS-controlled fade loop: 0.5s fade-in at start, 0.5s fade-out at end, using requestAnimationFrame. On ended, opacity resets to 0, waits 100ms, then replays from 0
No gradient overlays on the video
The wrapper div has overflow-hidden, the hero content sits in a relative z-10 div above

Blurred overlay shape (centered behind content):
w-[984px] h-[527px] opacity-90 bg-gray-950 blur-[82px]
Absolutely positioned at top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
pointer-events-none
The hero section has overflow-visible so the blur is not clipped

Navbar:
Full width, py-5 px-8, flex row with justify-between
Left: logo image (src/assets/logo.png, height 32px)
Center: nav items — "Features" (with ChevronDown), "Solutions", "Plans", "Learning" (with ChevronDown). Each is a button with text-foreground/90 and hover transition
Right: "Sign Up" button using heroSecondary variant, rounded-full px-4 py-2
Below navbar: a 1px divider line with gradient from-transparent via-foreground/20 to-transparent, offset mt-[3px]

Hero content (vertically centered in remaining space via flex-1):
Headline: "Power AI" at text-[220px], font-normal, leading-[1.02], tracking-[-0.024em], font-family General Sans
"Power " is plain text-foreground
"AI" uses bg-clip-text text-transparent with backgroundImage: linear-gradient(to left, #6366f1, #a855f7, #fcd34d) (indigo → purple → amber)
Subtitle: "The most powerful AI ever deployed / in talent acquisition" — text-hero-sub, text-lg, leading-8, max-w-md, mt-[9px], opacity-80
CTA: "Schedule a Consult" button, heroSecondary variant, px-[29px] py-[24px], mt-[25px]

Logo marquee (pinned to bottom of hero, pb-10):
Container: max-w-5xl mx-auto
Left side: static text "Relied on by brands / across the globe" in text-foreground/50 text-sm
Right side: infinite scrolling marquee with logos: Vortex, Nimbus, Prysma, Cirrus, Kynder, Halcyn (duplicated for seamless loop)
Each logo: a liquid-glass 24x24 rounded-lg icon showing the first letter, plus the name in text-base font-semibold text-foreground
Marquee animation: translateX(0%) → translateX(-50%), 20s linear infinite
gap-16 between logos, gap-12 between text and marquee

Liquid glass utility class (in index.css):
.liquid-glass { background: rgba(255, 255, 255, 0.01); background-blend-mode: luminosity; backdrop-filter: blur(4px); border: none; box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1); position: relative; overflow: hidden; }
.liquid-glass::before { content: ""; position: absolute; inset: 0; border-radius: inherit; padding: 1.4px; background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%); -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0); -webkit-mask-composite: xor; mask-composite: exclude; pointer-events: none; }

Section structure: min-h-screen flex flex-col — navbar at top, content centered via flex-1 flex items-center justify-center, marquee at bottom.

## Prioritize — Hero Section [sites/prioritize-hero]

- Preview: https://motionsites.ai/assets/hero-prioritize-preview-DlI3SYr4.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/prioritize-hero.png

System & Tech Stack Requirements:
Build a responsive landing page hero section and navigation bar using React, Tailwind CSS, Framer Motion (import { motion } from "motion/react"), and lucide-react for icons.
1. Global Layout & Styling Setup:
Fonts: Import Google Fonts: Inter (weights: 100-500) for the UI sans-serif, and Caveat (weights: 400-700) for handwriting elements. Apply Inter to the body. Custom Tailwind theme config: --font-sans: "Inter", ... and --font-handwriting: "Caveat", cursive.
Global Layout Settings: The main application background is a very light gray #FDFDFD. Body text colors are primarily black #141414 or gray text-gray-500/400.
Brand Icon SVG: Create an SVG logo consisting of three staggered building blocks stepping up from bottom-left to top-right. Use this exact code:
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="none" className="w-8 h-8"><path d="M 64 256 L 0 256 L 0 192 L 64 192 Z M 160 256 L 96 256 L 96 160 L 0 160 L 0 96 L 160 96 Z M 256 256 L 192 256 L 192 64 L 0 64 L 0 0 L 256 0 Z" fill="#2563EB"></path></svg>
2. Component 1: Navbar (Navbar.tsx)
Layout: Fixed or relative top bar width 100%, bg-[#FDFDFD], flex justified between, padding x-6 y-6 (md:px-12), z-index 50.
Left Hand Side: The Brand SVG (above) paired with text "Prioritize" (text-xl, tracking-tight, #141414).
Center Links (Desktop only): "Features", "Solutions", "Resources", "Pricing". Text: text-sm, text-gray-500, hover:text-black.
Right Hand Side (Desktop only): Two buttons. "Sign in" (plain text, text-gray-500 hover text-black) and "Try for free" (bg-white border rounded-xl, text-sm text-[#141414] hover:bg-gray-50).
Mobile: A hamburger menu (Menu / X from lucide-react) toggling a dropdown overlay (bg-white absolute top-full left-0 full-width) that lists the nav items, a divider hr, and the two auth buttons stacked. The mobile "Try for free" button uses bg-[#2563EB] text-white with shadow-blue-500/20.
3. Component 2: Hero (Hero.tsx)
Hero Container: Wrap the entire hero in a bg-[#FDFDFD] section. Inside, place a massive "card": w-full min-h-[85vh] py-32 bg-white rounded-2xl md:rounded-[2.5rem] border border-black/10 overflow-hidden relative flex flex-col items-center justify-center text-center.
Background Noise: Inside the card, add an absolute pointer-events-none div filling the space. Use inline style for radial gradient noise: background: "#ffffff", backgroundImage: "radial-gradient(circle at 1px 1px, rgba(0, 0, 0, 0.08) 1px, transparent 0)", backgroundSize: "20px 20px".
Central Content Layer (z-10):
Icon: The Brand SVG inside a rounded-2xl white box with dropshadow (drop-shadow-[0_4px_12px_rgba(0,0,0,0.1)]). Animate Fade/slide up (y: 20 -> 0).
H1: Draft, build, and ship <br/> <span className="text-gray-500">every single idea</span>. Text sizes: text-4xl md:text-6xl lg:text-7xl tracking-tight text-[#141414]. Animate Fade/slide up (duration: 0.8, delay: 0.2).
Subtitle: "Organize your workflow and realize your creative vision." text-gray-400 text-xl. Cascade animate up (delay: 0.4).
Button: "Start free trial". bg-[#2563EB] text-white rounded-xl shadow-blue-500/20. Animate pop-in (scale 0.95 -> 1, delay 0.6) with hover scale 1.05.
4. Floating Decorative UI Blocks (Positioned absolutely around the center)
Use Framer Motion to animate the entrance. Scale them down responsive-ly so they don't break mobile view (scale-[0.4] sm:scale-50 md:scale-75 lg:scale-90).
Group 1: Top-Left Overlay (absolute top-16 -left-12)
Element 1: A yellow sticky note. bg-[#FFF188] p-7 w-56 shadow. Include a red #D32F2F pin dot at top center. Text: "Capture fleeting thoughts, organize project details, and execute with precision." Font: font-handwriting text-[21px] text-[#424242]. Animation: Fade + rotate 2deg -> 3deg (delay 0.4).
Element 2: Transparent Folder & Check. A Folder lucide icon (w-72 fill-white/60 drop-shadow). Overlaid on it, a white card containing a blue box bg-[#2563EB] with a white Check lucide icon inside, tilted -2deg. Animation: Fade + rotate 6deg -> 12deg (delay 0.6).
Group 2: Bottom-Left 'Active Sprints' Folder (absolute -bottom-20 -left-8)
Folder icon (w-[450px] fill-[#F2F3F5] text-gray-200 drop-shadow).
Content inside: Title "Active Sprints".
Two floating task cards (bg-white/95, rounded-xl p-3). Each card contains:
Left: A colored numeric badge (Task 1: Orange #FF5722 "8" / Task 2: Green #00C853 "3") + Project Name ("Design Hub", "Prod Refresh").
Right: Overlapping avatars. Avatars URLs use referrerPolicy="no-referrer":
T1 Avatars: https://api.dicebear.com/7.x/avataaars/svg?seed=Felix, ...seed=Aneka
T2 Avatars: https://api.dicebear.com/7.x/avataaars/svg?seed=Sam, ...seed=Maya
Bottom: A progress track consisting of Date badge + Progress bar line (light blue #00BFFF, T2 has red overage #FF5252) + percentage indicator.
Animation: Fade + slide up y: 50 -> 0, turn rotate: -5 -> -4 (delay 0.8).
Group 3: Bottom-Right 'Seamless Sync' Folder (absolute -bottom-24 -right-8)
Folder icon (w-[450px] fill-[#F2F3F5] text-gray-200 drop-shadow).
Content inside: Title "Seamless Sync".
Below it: 3 x white integration squared boxes floating next to each other. Icons inside: Mail (lucide, color #EA4335), Slack (lucide, color #4A154B), Calendar (lucide, color #4285F4). Add hover transition: hover:scale-105.
Animation: Fade + slide up y: 50 -> 0, turn rotate: 5 -> 4 (delay 1.0).
Group 4: Top-Right 'Deadlines' Folder (absolute -top-10 -right-32)
Folder icon (w-[420px] fill-[#F2F3F5] text-gray-200 drop-shadow).
Content inside: Title "Deadlines".
"Project Launch" Box (bg-white/90 p-5 rounded-2xl). Includes an absolute "Meetings" pill at the top right bounding-box. A subtitle "Review with design leads", and a time badge at bottom: #E1F5FE light blue background with a Clock icon and text "13:00 - 13:45" colored #03A9F4.
Floating independently atop this group: A tilted -8deg white box block holding a Timer lucide icon with a custom red line pointer overlapping it.
Timer box animation: Slide from left x: -20 scale from 0.8 (delay 1.4)
Overlay finishing touch: Add an empty transparent Folder icon on top of everything here tilted rotate-[15deg] (fill-white/60).
Overall folder animation: Rotate 4 -> 6, Slide x: 50 (delay 1.2).

## Railroad.ai — Hero Section [sites/railroad-ai-hero]

- Preview: https://motionsites.ai/assets/hero-railroad-ai-preview-CBjplU90.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/railroad-ai-hero.gif

Create a landing page hero section with the following exact specifications:

Background:

Full-screen background video covering the entire viewport with object-cover, autoplaying, looping, muted, inline playback, and preloaded
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260317_100335_dc625816-c3c1-4b00-b93e-4cb301cf5ea5.mp4
A subtle bg-black/5 overlay on top of the video

Fonts:

Import: https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap
However, the actual font-family used is the system font stack: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'SF Pro Display', system-ui, sans-serif for both heading and body

Color Tokens (HSL, in CSS variables):

--background: 213 45% 67%
--foreground: 0 0% 100% (white)
--heading: 205 52% 5% (near-black)
--description: 180 9% 33% (muted dark gray)
--primary: 0 0% 100% (white)
--primary-foreground: 0 0% 0% (black)
--radius: 9999px (fully rounded)

Navbar (fixed, top 30px):

Fixed position, top-[30px], full width, z-50, horizontal padding px-8 lg:px-16, flex row with space-between
Left: Logo text "Railroad.ai" in heading font, text-2xl, white, tight tracking
Center: A liquid-glass pill with rounded-full containing 4 nav links: "Home", "Voyages", "Worlds", "Innovation" — each text-sm font-medium text-foreground/90, hidden on mobile (hidden md:flex)
Right: "Get Started" button with ArrowUpRight icon, bg-primary text-primary-foreground rounded-full px-4 py-2 text-sm font-medium

Hero Content (centered, vertically):

Container: flex-1 flex flex-col items-center justify-center text-center px-4 pt-24 pb-[200px]
Badge: A liquid-glass rounded-full pill with bg-black/10, containing text "10K+ already subscribed" in text-sm text-foreground/90 px-3 font-body

Heading: "Focus in a Constantly Distracted World" — uses a custom BlurText component that splits text into words and animates each word individually with framer-motion: blur(10px) → blur(0px), opacity 0→1, y 50→0, duration 0.35s, staggered delay of 100ms per word. Styled as text-6xl md:text-7xl lg:text-[5.5rem] font-heading text-heading leading-[0.85] tracking-[-4px] max-w-3xl

Subheading: "Cut through the noise of notifications, endless feeds, and constant interruptions. Learn how to reclaim your attention and do meaningful work that truly matters." — framer-motion animation: blur(10px)→blur(0px), opacity 0→1, y 20→0, duration 0.6s, delay 0.8s. Styled as text-[calc(1rem+3px)] md:text-[calc(1.125rem+3px)] text-description max-w-2xl leading-tight tracking-[-0.05em]

Email Input: framer-motion animated (same blur/fade pattern, delay 1.1s). A liquid-glass rounded-full container with inline styles: backdropFilter: blur(100px), background: rgba(0,0,0,0.25), padding p-1.5 pl-6. Inside: a transparent <input> with placeholder "Enter your email" and a white rounded-full "Join Waitlist" button with ArrowUpRight icon (bg-primary text-primary-foreground rounded-full px-5 py-2.5 text-sm font-medium)

Liquid Glass CSS (critical):

.liquid-glass {
  background: rgba(255,255,255,0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
  border: none;
  box-shadow: inset 0 1px 1px rgba(255,255,255,0.1);
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

Tech Stack: React, Vite, TypeScript, Tailwind CSS, framer-motion, lucide-react (ArrowUpRight icon), shadcn/ui design tokens.

## RIVR — Hero Section [sites/rivr-hero]

- Preview: https://motionsites.ai/assets/hero-rivr-preview-DcS3pjx4.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/rivr-hero.gif

Build a Hero section for a DeFi dashboard named RIVR showcasing a sleek, glassmorphism aesthetic. Please mimic these exact specifications to ensure a premium UI.

Dependencies: 
- Use `lucide-react` for icons.
- Use `motion` (imported from `'motion/react'`) for animations.

1. Global Styles (`src/index.css`)
Import the custom 'Helvetica Regular' font, set the Tailwind theme properly, and reset the body. Exact CSS to include:
@import "tailwindcss";

@font-face {
    font-family: "Helvetica Regular";
    src: url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.eot");
    src: url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.eot?#iefix")format("embedded-opentype"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.woff2")format("woff2"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.woff")format("woff"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.ttf")format("truetype"),
    url("https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65.svg#Helvetica Regular")format("svg");
}

@theme {
  --font-helvetica: "Helvetica Regular", ui-sans-serif, system-ui, sans-serif;
}

:root {
  font-family: var(--font-helvetica);
}

body {
  margin: 0;
  overflow-x: hidden;
  background-color: #f0f0f0;
}

2. App Structure (`src/App.tsx`)
Create a single `<main className="min-h-screen bg-[#f0f0f0]">` instance that returns the `<Hero />` component.

3. Hero Component (`src/components/Hero.tsx`)
Outer wrapper: `<div className="w-full h-screen flex items-center justify-center p-3 md:p-5 bg-[#f0f0f0]">`.
Inner container: `<section className="relative w-full max-w-[1536px] h-full rounded-[1.5rem] md:rounded-[3rem] overflow-hidden shadow-none flex flex-col items-center bg-white/10 group">`
Inside the `<section>`:
- The Video Background: 
  A `<video>` element with `autoPlay muted loop playsInline`. 
  Classes: `absolute inset-0 w-full h-full object-cover object-[65%] lg:object-center z-0`. 
  Source URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260428_193507_4286c423-2fd9-4efd-92bd-91a939453fc1.mp4` (Must use exactly this URL).
- The Content Layer:
  A `<div className="relative z-10 w-full h-full flex flex-col items-center">`.
  Inside it, place: `<Navbar />`, the text container, `<BottomLeftCard />`, and `<BottomRightCorner />`.
- Text Container:
  `<div className="w-full flex flex-col items-center pt-8 px-6 text-center max-w-4xl">`. Inside it:
  - `<HeroBadge />`
  - A `<motion.h1>` with class: `text-4xl sm:text-5xl md:text-6xl lg:text-[80px] font-normal text-[#5E6470] mb-2 tracking-tight leading-[1.05]`. Text: "Fluid Asset Streams". Animation: initial={{ opacity: 0, scale: 0.98 }}, animate={{ opacity: 1, scale: 1 }}, transition={{ duration: 0.8, delay: 0.2 }}.
  - A `<motion.p>` with class: `text-sm sm:text-base md:text-lg text-[#5E6470] opacity-80 leading-relaxed max-w-xl font-normal`. Text: "Access Smart Vaults, stake RIVR, NFTs, transform rigid holdings into liquid cash instantly.". Animation: initial={{ opacity: 0 }}, animate={{ opacity: 1 }}, transition={{ duration: 0.8, delay: 0.4 }}.

4. Navbar Component (`src/components/Navbar.tsx`)
Wrapper: `<nav className="flex items-center justify-between py-6 px-6 md:px-10 w-full relative z-10">`.
- Left Side (hidden spacer for centering): `<div className="flex-1 hidden md:block" />`
- Center Menu: `<ul className="hidden md:flex items-center gap-8 text-[rgb(45,45,45)] font-normal text-sm">`. Include items: Ecosystem, Economics (hasDropdown), Developers, Governance (hasDropdown). List items need: `cursor-pointer hover:opacity-70 transition-opacity flex items-center gap-1 group`. Append a `ChevronRight` icon (classes: `w-4 h-4 transition-transform group-hover:translate-x-0.5`) if hasDropdown is true.
- Mobile Logo: `<div className="md:hidden"><span className="font-regular tracking-tighter text-xl text-[rgba(30,50,90,0.9)]">RIVR</span></div>`
- Right Button: `<div className="flex-1 flex justify-end">` wrapping a `<motion.button>` (whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}). 
  Button classes: `flex items-center bg-[rgba(30,50,90,0.8)] text-white rounded-full pl-2 pr-4 md:pr-6 py-1.5 md:py-2 gap-2 md:gap-3 hover:bg-[rgba(30,50,90,1)] transition-colors group`. Inside button: Add an icon wrapper `<div className="bg-white/20 p-1 md:p-1.5 rounded-full flex items-center justify-center">` containing `ArrowUpRight` (w-4 h-4 md:w-5 md:h-5 text-white), and a text node "Book Demo" (`text-xs md:text-sm font-normal`).

5. HeroBadge Component (`src/components/HeroBadge.tsx`)
Returns a `<motion.div>` (initial opacity 0, y 20; animate opacity 1, y 0; transition duration 0.6, ease "easeOut").
Classes: `flex items-center gap-2 px-4 py-2 rounded-full bg-white/60 backdrop-blur-md border border-white/20 mx-auto mb-3 w-fit`.
Contents: `<Sparkles className="w-4 h-4 text-[rgba(30,50,90,0.8)]" />` and text `<span className="text-[14px] font-normal text-[rgba(30,50,90,0.9)]">Fluid Staking</span>`.

6. BottomLeftCard Component (`src/components/BottomLeftCard.tsx`)
Returns a `<motion.div>` (initial x: -20, opacity: 0; animate x: 0, opacity: 1; transition: duration 0.8, delay 0.2).
Position/Styling: `absolute bottom-28 right-4 left-auto md:left-6 md:right-auto md:bottom-6 lg:bottom-10 lg:left-10 p-3 md:p-4 lg:p-5 rounded-[1.2rem] md:rounded-[1.5rem] lg:rounded-[2.2rem] bg-white/30 backdrop-blur-xl flex flex-col gap-2 lg:gap-3 min-w-[140px] md:min-w-[150px] lg:min-w-[180px] w-fit`.
- Top text block: column with "5.2K" (classes: `text-2xl md:text-3xl font-normal text-[rgba(30,50,90,0.9)] tracking-tight`) and "Active Yielders" (classes: `text-[10px] md:text-[12px] font-normal text-[rgba(30,50,90,0.6)] uppercase tracking-wider`).
- Join Discord `<motion.button>` (hover/tap scale 1.02/0.98). Classes: `flex items-center bg-white rounded-full pl-1.5 pr-5 py-1.5 gap-2 hover:bg-white/90 transition-colors self-start group`. Inside: wrap `ArrowUpRight` in `<div className="bg-[rgba(30,50,90,0.1)] p-1 rounded-full ...">` (using `text-[rgba(30,50,90,0.9)]` for icon) and append "Join Discord" text (`text-[14px] font-normal text-[rgba(30,50,90,0.9)]`).

7. BottomRightCorner Component (`src/components/BottomRightCorner.tsx`)
This requires a complex faux-cutout layout. Use a `<motion.div>` (initial y: 20, opacity: 0; animate y: 0, opacity: 1; duration: 0.8, delay: 0.4).
Classes: `absolute bottom-0 right-0 p-3 pt-5 pl-8 sm:p-4 sm:pt-6 sm:pl-10 md:p-6 md:pt-8 md:pl-14 bg-[#f0f0f0] rounded-tl-[1.5rem] sm:rounded-tl-[2rem] md:rounded-tl-[3.5rem] flex items-center gap-3 sm:gap-4 md:gap-6`.
CRITICAL corner masks to include inside this container:
- Top intersection mask: `<div className="absolute -top-[1.5rem] sm:-top-[2rem] md:-top-[3.5rem] right-0 w-[1.5rem] sm:w-[2rem] md:w-[3.5rem] h-[1.5rem] sm:h-[2rem] md:h-[3.5rem] pointer-events-none"><svg width="100%" height="100%" viewBox="0 0 56 56" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M56 56V0C56 30.9279 30.9279 56 0 56H56Z" fill="#f0f0f0"/></svg></div>`
- Left intersection mask: `<div className="absolute bottom-0 -left-[1.5rem] sm:-left-[2rem] md:-left-[3.5rem] w-[1.5rem] sm:w-[2rem] md:w-[3.5rem] h-[1.5rem] sm:h-[2rem] md:h-[3.5rem] pointer-events-none"><svg width="100%" height="100%" viewBox="0 0 56 56" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M56 56H0C30.9279 56 56 30.9279 56 0V56Z" fill="#f0f0f0"/></svg></div>`
Content: 
- Circle Icon: A div with `bg-[rgba(30,50,90,0.05)] w-10 h-10 md:w-14 md:h-14 rounded-full flex items-center justify-center border border-[rgba(30,50,90,0.1)]` using `ArrowUpRight` (`text-[rgba(30,50,90,0.8)]`).
- Info column containing title "Documentation" (`text-[16px] md:text-[20px] font-normal text-[rgba(30,50,90,0.95)]`). Below it, a line containing text "Library" and a `ChevronRight` icon wrapped in `<div className="flex items-center gap-1 text-[rgba(30,50,90,0.6)] cursor-pointer hover:text-[rgba(30,50,90,0.8)] transition-colors"><span className="text-[12px] md:text-[15px] font-normal">...`

## Sentinel AI — Hero Section [sites/sentinel-ai-hero]

- Preview: https://motionsites.ai/assets/hero-sentinel-ai-preview-BXas7Q1_.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/sentinel-ai-hero.gif

Create a full-screen dark hero landing page for a security company called "SENTINEL AI" using React, Vite, TypeScript, Tailwind CSS, shadcn/ui, and an embedded Spline 3D scene as the background. The tech stack uses @splinetool/react-spline and @splinetool/runtime for the 3D embed. Here is every detail:

FONT:
Google Fonts "Sora" with weights 300, 400, 500, 600, 700. Load it in index.html:


<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Sora:wght@300;400;500;600;700&display=swap" rel="stylesheet">
Set font-sora as the body font via Tailwind config: fontFamily: { sora: ["Sora", "sans-serif"] } and apply font-sora antialiased on body.

COLOR THEME (all HSL CSS custom properties, dark only, no light mode):

--background: 0 0% 10% (dark charcoal)
--foreground: 0 0% 96% (near-white)
--primary: 119 99% 46% (vivid green)
--primary-foreground: 0 0% 4% (near-black)
--secondary: 0 0% 18%
--secondary-foreground: 0 0% 96%
--muted: 0 0% 16%
--muted-foreground: 0 0% 60%
--accent: 119 99% 46% (same vivid green as primary)
--accent-foreground: 0 0% 4%
--destructive: 0 84% 60%
--border: 0 0% 20%
--input: 0 0% 20%
--ring: 119 99% 46%
--radius: 0.5rem
--nav-button: 0 0% 18%
--hero-bg: 0 0% 8% (the darkest background, nearly black)
Map these in Tailwind config using hsl(var(--variable)) pattern. Add custom color tokens: nav-button and hero-bg.

CUSTOM ANIMATIONS (Tailwind config keyframes + animation):

fade-up keyframe:

0%: opacity: 0, transform: translateY(20px), filter: blur(4px)
100%: opacity: 1, transform: translateY(0), filter: blur(0)
Animation: fade-up 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards
fade-in keyframe:

0%: opacity: 0
100%: opacity: 1
Animation: fade-in 0.5s ease-out forwards
NAVBAR (fixed, transparent, floating over the Spline scene):

fixed top-0 left-0 right-0 z-50, horizontal flex, justify-between, padding px-8 lg:px-16 py-5
Left: Logo text "SENTINEL" -- text-foreground text-xl font-semibold tracking-tight
Center: Nav links array: ["Services", "About Us", "Projects", "Team", "Contacts"] -- each is text-sm text-muted-foreground hover:text-foreground transition-colors uppercase tracking-widest. Links use href={#section-name}. Hidden on mobile (hidden md:flex), with gap-8.
Right: "Get Quote" button using shadcn Button with a custom navCta variant: text-foreground bg-nav-button hover:bg-nav-button/80 active:scale-[0.97] transition-all. Size lg, with classes hidden md:inline-flex rounded-lg uppercase text-xs tracking-widest px-6.
HERO SECTION (full-screen, content at bottom-left):

Structure:

Outer <section>: relative min-h-screen flex items-end bg-hero-bg overflow-hidden
Spline 3D Background (absolute, full-size): Lazy-loaded via React.lazy(() => import("@splinetool/react-spline")) wrapped in <Suspense> with a fallback <div className="absolute inset-0 bg-hero-bg" />. The Spline component uses scene="https://prod.spline.design/Slk6b8kz3LRlKiyk/scene.splinecode" and className="w-full h-full". Placed inside <div className="absolute inset-0">.
Dark overlay: <div className="absolute inset-0 bg-black/30 z-[1] pointer-events-none" />
Content container: relative z-10 pointer-events-none w-full max-w-[90%] sm:max-w-md lg:max-w-2xl px-6 md:px-10 pb-10 md:pb-10 pt-32
Content elements (all with staggered animate-fade-up, starting opacity-0):

Heading (delay 0.2s): <h1> with text "SENTINEL" in white + " AI" in primary green. Classes: text-[clamp(3rem,8vw,6rem)] font-bold leading-[1.05] tracking-[-0.05em] text-foreground mb-2 md:mb-4 uppercase. The "AI" part is wrapped in <span className="text-primary">.

Subheading (delay 0.4s): <p> -- "We implement security correctly." -- text-foreground/80 text-[clamp(1.125rem,2.5vw,1.875rem)] font-light mb-3 md:mb-6

Description (delay 0.55s): <p> -- "Enterprise security systems built in days. AI-powered surveillance deployed with zero-trust architecture. Smart access control set up for your entire facility. All of it done right, not just fast." -- text-muted-foreground text-[clamp(0.875rem,1.5vw,1.25rem)] font-light mb-4 md:mb-8

Two CTA buttons (delay 0.7s): Wrapped in flex flex-wrap gap-3 font-bold. Both are plain <button> elements (not shadcn Button) with pointer-events-auto (to re-enable clicks since parent is pointer-events-none):

"Book a Call": bg-primary text-primary-foreground px-6 py-3 md:px-8 md:py-4 text-sm rounded-sm cursor-pointer hover:brightness-110 transition-all active:scale-[0.97]
"Our Work": bg-white text-background px-6 py-3 md:px-8 md:py-4 text-sm rounded-sm cursor-pointer hover:brightness-90 transition-all active:scale-[0.97]
Trust line (delay 0.85s): <p> -- "Trusted security partner. Columbus, OH. 12 systems deployed." -- text-muted-foreground/60 text-xs font-light mt-4 md:mt-6

All animated elements use style={{ animationDelay: "Xs" }} for the stagger, combined with the opacity-0 animate-fade-up classes.

PAGE WRAPPER (Index.tsx):
Simple wrapper: <div className="bg-hero-bg min-h-screen"> containing <Navbar /> and <HeroSection />.

KEY DEPENDENCIES:

@splinetool/react-spline and @splinetool/runtime for the 3D Spline embed
tailwindcss-animate plugin
shadcn/ui Button component with custom variants (navCta, hero, heroOutline)
class-variance-authority for button variants
IMPORTANT NOTES:

The Spline scene URL is https://prod.spline.design/Slk6b8kz3LRlKiyk/scene.splinecode -- this is the exact 3D scene used
The entire content area has pointer-events-none so clicks pass through to the Spline scene, but buttons re-enable with pointer-events-auto
Responsive fluid typography uses clamp() for the heading, subheading, and description
The content is anchored to the bottom-left of the viewport (flex items-end on the section + padding-bottom on the content)
No hamburger menu on mobile -- the nav links and CTA simply hide (hidden md:flex / hidden md:inline-flex)

## Shamoni — Hero Section [sites/shamoni-hero]

- Preview: https://motionsites.ai/assets/hero-shamoni-preview-DfbPWZl9.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/shamoni-hero.gif

Build an immersive, highly interactive, scroll-driven landing page using React, Vite, Tailwind CSS (v4), and `motion/react` (Framer Motion). 

Please set up the application with the exact files, dependencies, URLs, CSS variables, and mathematical Framer Motion values provided below.

### Setup & Dependencies
Install the following libraries:
`npm install motion react react-dom lucide-react`
`npm install -D tailwindcss @tailwindcss/vite`

Ensure Tailwind V4 is correctly initialized via `@tailwindcss/vite` in `vite.config.ts`.

---

### 1. Global Styles (`src/index.css`)
Import the necessary Google Fonts and set up the Tailwind V4 `@theme` overrides:

```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Manrope:wght@300;400;500;600&family=Great+Vibes&display=swap');
@import "tailwindcss";

@theme {
  --font-serif: "Instrument Serif", serif;
  --font-sans: "Manrope", sans-serif;
  --font-script: "Great Vibes", cursive;
}
2. Orbit Images Component Styles (src/components/OrbitImages.css)
This CSS provides the absolute positioning offsets for our custom rotation gallery.

code
CSS
.orbit-container {
  position: relative;
  margin-left: auto;
  margin-right: auto;
}

.orbit-scaling-container {
  width: 100%;
  height: 100%;
  position: relative;
}

.orbit-scaling-container--responsive {
  position: absolute;
  left: 50%;
  top: 50%;
  transform-origin: center center;
}

.orbit-rotation-wrapper {
  width: 100%;
  height: 100%;
  transform-origin: center center;
  position: relative;
}

.orbit-path-svg {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.orbit-item {
  position: absolute;
  will-change: transform;
  user-select: none;
}

.orbit-center-content {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.orbit-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: 50%; 
}
3. Orbit Images React Component (src/components/OrbitImages.tsx)
Create this mathematically precise component that maps motion paths over SVG strings using offsetPath and offsetDistance. It accepts Framer Motion MotionValues as overrides to allow the parent App.tsx to infinitely control its radius, spread, item size, and rotation during scroll.

code
Tsx
// @ts-nocheck
import { useMemo, useEffect, useRef, useState } from 'react';
import { motion, useMotionValue, useTransform, animate, useMotionTemplate } from 'motion/react';
import './OrbitImages.css';

function generateEllipsePath(cx, cy, rx, ry) {
  return `M ${cx - rx} ${cy} A ${rx} ${ry} 0 1 0 ${cx + rx} ${cy} A ${rx} ${ry} 0 1 0 ${cx - rx} ${cy}`;
}

function generateCirclePath(cx, cy, r) {
  return generateEllipsePath(cx, cy, r, r);
}

function generateSquarePath(cx, cy, size) {
  const h = size / 2;
  return `M ${cx - h} ${cy - h} L ${cx + h} ${cy - h} L ${cx + h} ${cy + h} L ${cx - h} ${cy + h} Z`;
}

function generateRectanglePath(cx, cy, w, h) {
  const hw = w / 2;
  const hh = h / 2;
  return `M ${cx - hw} ${cy - hh} L ${cx + hw} ${cy - hh} L ${cx + hw} ${cy + hh} L ${cx - hw} ${cy + hh} Z`;
}

function generateTrianglePath(cx, cy, size) {
  const height = (size * Math.sqrt(3)) / 2;
  const hs = size / 2;
  return `M ${cx} ${cy - height / 1.5} L ${cx + hs} ${cy + height / 3} L ${cx - hs} ${cy + height / 3} Z`;
}

function generateStarPath(cx, cy, outerR, innerR, points) {
  const step = Math.PI / points;
  let path = '';
  for (let i = 0; i < 2 * points; i++) {
    const r = i % 2 === 0 ? outerR : innerR;
    const angle = i * step - Math.PI / 2;
    const x = cx + r * Math.cos(angle);
    const y = cy + r * Math.sin(angle);
    path += i === 0 ? `M ${x} ${y}` : ` L ${x} ${y}`;
  }
  return path + ' Z';
}

function generateHeartPath(cx, cy, size) {
  const s = size / 30;
  return `M ${cx} ${cy + 12 * s} C ${cx - 20 * s} ${cy - 5 * s}, ${cx - 12 * s} ${cy - 18 * s}, ${cx} ${cy - 8 * s} C ${cx + 12 * s} ${cy - 18 * s}, ${cx + 20 * s} ${cy - 5 * s}, ${cx} ${cy + 12 * s}`;
}

function generateInfinityPath(cx, cy, w, h) {
  const hw = w / 2;
  const hh = h / 2;
  return `M ${cx} ${cy} C ${cx + hw * 0.5} ${cy - hh}, ${cx + hw} ${cy - hh}, ${cx + hw} ${cy} C ${cx + hw} ${cy + hh}, ${cx + hw * 0.5} ${cy + hh}, ${cx} ${cy} C ${cx - hw * 0.5} ${cy + hh}, ${cx - hw} ${cy + hh}, ${cx - hw} ${cy} C ${cx - hw} ${cy - hh}, ${cx - hw * 0.5} ${cy - hh}, ${cx} ${cy}`;
}

function generateWavePath(cx, cy, w, amplitude, waves) {
  const pts = [];
  const segs = waves * 20;
  const hw = w / 2;
  for (let i = 0; i <= segs; i++) {
    const x = cx - hw + (w * i) / segs;
    const y = cy + Math.sin((i / segs) * waves * 2 * Math.PI) * amplitude;
    pts.push(i === 0 ? `M ${x} ${y}` : `L ${x} ${y}`);
  }
  for (let i = segs; i >= 0; i--) {
    const x = cx - hw + (w * i) / segs;
    const y = cy - Math.sin((i / segs) * waves * 2 * Math.PI) * amplitude;
    pts.push(`L ${x} ${y}`);
  }
  return pts.join(' ') + ' Z';
}

function OrbitItem({ item, index, totalItems, pathValue, itemSizeValue, rotationValue, progress, fill, scaleStrength, focalPoint = 50 }) {
  const itemOffset = fill ? (index / totalItems) * 100 : 0;

  const offsetPercentage = useTransform(progress, (p) => {
    return (((p + itemOffset) % 100) + 100) % 100;
  });

  const offsetDistance = useTransform(offsetPercentage, (p) => `${p}%`);

  const itemScale = useTransform(() => {
    const rawPos = offsetPercentage.get();
    const strength = scaleStrength ? scaleStrength.get() : 0;
    
    let dist = Math.abs(rawPos - focalPoint);
    if (dist > 50) dist = 100 - dist;

    let targetScale = 1;
    if (dist < 20) {
      const ratio = dist / 20;
      const cosCurve = (Math.cos(ratio * Math.PI) + 1) / 2;
      targetScale = 0.4 + (cosCurve * 0.6);
    } else {
      targetScale = 0.4;
    }

    return 1 - strength * (1 - targetScale);
  });

  const offsetPath = useMotionTemplate`path("${pathValue}")`;

  return (
    <motion.div
      className="orbit-item"
      style={{
        width: itemSizeValue,
        height: itemSizeValue,
        offsetPath,
        offsetRotate: '0deg',
        offsetAnchor: 'center center',
        offsetDistance,
        scale: itemScale,
        zIndex: useTransform(itemScale, s => Math.round(s * 100)),
        pointerEvents: 'auto'
      }}
    >
      <motion.div style={{ transform: useTransform(rotationValue, r => `rotate(${-r}deg)`), width: '100%', height: '100%' }}>{item}</motion.div>
    </motion.div>
  );
}

export default function OrbitImages({
  images = [],
  altPrefix = 'Orbiting image',
  shape = 'ellipse',
  customPath,
  baseWidth = 1400,
  radiusX = 700,
  radiusY = 170,
  radius = 300,
  starPoints = 5,
  starInnerRatio = 0.5,
  rotation = -8,
  duration = 40,
  itemSize = 64,
  direction = 'normal',
  fill = true,
  width = 100,
  height = 100,
  className = '',
  showPath = false,
  pathColor = 'rgba(0,0,0,0.1)',
  pathWidth = 2,
  easing = 'linear',
  paused = false,
  centerContent,
  responsive = false,
  progressOverride,
  radiusXOverride,
  radiusYOverride,
  itemSizeOverride,
  rotationOverride,
  translateXOverride,
  focusStrength,
}) {
  const containerRef = useRef(null);
  const [scale, setScale] = useState(1);

  const designCenterX = baseWidth / 2;
  const designCenterY = baseWidth / 2;

  const currentRadiusX = radiusXOverride || useMotionValue(radiusX);
  const currentRadiusY = radiusYOverride || useMotionValue(radiusY);
  const currentItemSize = itemSizeOverride || useMotionValue(itemSize);
  const currentRotation = rotationOverride || useMotionValue(rotation);
  const currentTranslateX = translateXOverride || useMotionValue(0);

  const pathValue = useTransform([currentRadiusX, currentRadiusY], ([rx, ry]) => {
    switch (shape) {
      case 'circle': return generateCirclePath(designCenterX, designCenterY, rx);
      case 'ellipse': return generateEllipsePath(designCenterX, designCenterY, rx, ry);
      case 'square': return generateSquarePath(designCenterX, designCenterY, rx * 2);
      case 'rectangle': return generateRectanglePath(designCenterX, designCenterY, rx * 2, ry * 2);
      case 'triangle': return generateTrianglePath(designCenterX, designCenterY, rx * 2);
      case 'star': return generateStarPath(designCenterX, designCenterY, rx, rx * starInnerRatio, starPoints);
      case 'heart': return generateHeartPath(designCenterX, designCenterY, rx * 2);
      case 'infinity': return generateInfinityPath(designCenterX, designCenterY, rx * 2, ry * 2);
      case 'wave': return generateWavePath(designCenterX, designCenterY, rx * 2, ry, 3);
      case 'custom': return customPath || generateCirclePath(designCenterX, designCenterY, rx);
      default: return generateEllipsePath(designCenterX, designCenterY, rx, ry);
    }
  });

  useEffect(() => {
    if (!responsive || !containerRef.current) return;
    const updateScale = () => {
      if (!containerRef.current) return;
      setScale(containerRef.current.clientWidth / baseWidth);
    };
    updateScale();
    const observer = new ResizeObserver(updateScale);
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [responsive, baseWidth]);

  const internalProgress = useMotionValue(0);

  useEffect(() => {
    if (paused || progressOverride) return;
    const controls = animate(internalProgress, direction === 'reverse' ? -100 : 100, {
      duration,
      ease: easing,
      repeat: Infinity,
      repeatType: 'loop',
    });
    return () => controls.stop();
  }, [internalProgress, duration, easing, direction, paused, progressOverride]);

  const activeProgress = progressOverride || internalProgress;
  const containerWidth = responsive ? '100%' : (typeof width === 'number' ? width : '100%');
  const containerHeight = responsive ? 'auto' : (typeof height === 'number' ? height : (typeof width === 'number' ? width : 'auto'));

  const items = images.map((src, index) => (
    <motion.img
      key={src}
      src={src}
      alt={`${altPrefix} ${index + 1}`}
      draggable={false}
      className="orbit-image"
      whileHover={{ scale: 1.2 }}
      transition={{ duration: 0.3 }}
      style={{ cursor: "pointer", pointerEvents: "auto" }}
    />
  ));

  return (
    <div ref={containerRef} className={`orbit-container ${className}`} style={{ width: containerWidth, height: containerHeight, aspectRatio: responsive ? '1 / 1' : undefined }} aria-hidden="true">
      <div className={responsive ? 'orbit-scaling-container orbit-scaling-container--responsive' : 'orbit-scaling-container'} style={{ width: responsive ? baseWidth : '100%', height: responsive ? baseWidth : '100%', transform: responsive ? `translate(-50%, -50%) scale(${scale})` : undefined }}>
        <motion.div className="orbit-rotation-wrapper" style={{ rotate: currentRotation, x: currentTranslateX }}>
          {showPath && (
             <svg width="100%" height="100%" viewBox={`0 0 ${baseWidth} ${baseWidth}`} className="orbit-path-svg">
              <path d={pathValue.get()} fill="none" stroke={pathColor} strokeWidth={pathWidth / scale} />
            </svg>
          )}
          {items.map((item, index) => (
            <OrbitItem key={index} item={item} index={index} totalItems={items.length} pathValue={pathValue} itemSizeValue={currentItemSize} rotationValue={currentRotation} progress={activeProgress} fill={fill} scaleStrength={focusStrength} focalPoint={50} />
          ))}
        </motion.div>
      </div>
      {centerContent && <div className="orbit-center-content">{centerContent}</div>}
    </div>
  );
}
4. Main Page App Component (src/App.tsx)
Implement the exact layout, UI timelines (scrollYProgress transforms), background <video>, typography mask, and the heavily orchestrated Framer Motion timeline values. Do not change any numbers in the arrays.

code
Tsx
import { motion, useMotionTemplate, useScroll, useTransform, useAnimationFrame, useMotionValue } from 'motion/react';
import { useRef } from 'react';
import OrbitImages from './components/OrbitImages';

const orbitImagesData = [
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/3644e7bae80f5a458c3c087d313204cc67952aff.3644e7ba.png",
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/85346ab4899007b001b3df5d5da04a9d0e4e9ea4.85346ab4.png",
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/ff5f9bb7c566be349d20a775a29eab9ff591311b.ff5f9bb7.png",
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/22e1b6bbc71c4977a49b6bbd991ed75be483cf0e.22e1b6bb.png",
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/874d9530b2ec45092a4c71a1fd74564599b7e3c8.874d9530.png",
  "https://aspect-slam-99684872.figma.site/_components/v2/79eebc3801de595030a9e7fa875de4a77ede4f07/2adc4a2c178d6aaa68dda80fc42e7628372522d1.2adc4a2c.png",
];

export default function App() {
  const containerRef = useRef<HTMLDivElement>(null);
  
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start start", "end end"]
  });

  const rx = useTransform(scrollYProgress, [0, 0.08, 1], ["0%", "55%", "55%"]);
  const ry = useTransform(scrollYProgress, [0, 0.08, 1], ["0%", "55%", "55%"]);
  const clipPath = useMotionTemplate`ellipse(${rx} ${ry} at 50% 50%)`;

  const textOpacity = useTransform(scrollYProgress, [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1], [0, 1, 1, 0, 0, 1, 1]);
  const textBlurVal = useTransform(scrollYProgress, [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1], [15, 0, 0, 15, 15, 0, 0]);
  const filterText = useMotionTemplate`blur(${textBlurVal}px)`;
  const yElement = useTransform(scrollYProgress, [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1], [20, 0, 0, 20, 20, 0, 0]);

  const targetRadius = 650;
  
  const orbitItemSize = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [80, 520, 520, 80, 80]);
  const orbitRx = useTransform(scrollYProgress,       [0.15, 0.25, 0.85, 0.95, 1], [330, targetRadius, targetRadius, 330, 330]);
  const orbitRy = useTransform(scrollYProgress,       [0.15, 0.25, 0.85, 0.95, 1], [140, targetRadius, targetRadius, 140, 140]);
  const orbitRotation = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [-15, 0, 0, -15, -15]);
  const orbitTx = useTransform(scrollYProgress,       [0.15, 0.25, 0.85, 0.95, 1], [0, -targetRadius, -targetRadius, 0, 0]);
  const focusStrength = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [0, 1, 1, 0, 0]);

  const orbitProgress = useMotionValue(0);
  const prevScroll = useRef(0);

  useAnimationFrame((time, delta) => {
     const pos = scrollYProgress.get();
     const scrollDelta = pos - prevScroll.current;
     prevScroll.current = pos;

     let frameSpeed = 0;
     if (pos > 0.15 && pos < 0.85) {
        frameSpeed = (scrollDelta * 200); 
     } else {
        frameSpeed = (delta / 1000) * 2.5; 
     }

     orbitProgress.set(orbitProgress.get() + frameSpeed);
  });

  return (
    <div ref={containerRef} className="relative w-full h-[600vh] bg-black">
      <div className="sticky top-0 w-full h-screen overflow-hidden text-white">
        
        <video autoPlay loop muted playsInline className="absolute inset-0 w-full h-full object-cover z-0">
          <source src="https://stream.mux.com/OD2Ny6q9anbQ9h7Vie3KnqDxFpzHM9sjwfhF029lfd600.m3u8" type="video/mp4" />
        </video>

        <div className="absolute inset-0 bg-black/10 z-0"></div>

        <div className="absolute z-10 w-[80vw]" style={{ left: '3vw', bottom: '3vw' }}>
          <svg viewBox="0 10 350 72" className="w-full h-auto drop-shadow-2xl overflow-visible" preserveAspectRatio="xMinYMax meet">
            <text x="-3" y="80" fontFamily="'Instrument Serif', serif" fill="#FDFFB7" className="select-none">
              <tspan fontSize="90">Shamoni</tspan>
              <tspan fontSize="28.8" dx="4" dy="-40">©</tspan>
            </text>
          </svg>
        </div>

        <motion.div 
          className="absolute z-20 flex items-center justify-center overflow-hidden"
          style={{ clipPath, rotate: -15, width: '150vw', height: '150vh', left: '-25vw', top: '-25vh' }}
        >
          <div className="absolute inset-0 bg-white" />
          <div className="relative flex flex-col items-center justify-center" style={{ width: '100vw', height: '100vh', transform: 'rotate(15deg)' }}>
            <motion.div className="w-[90vw] max-w-[1200px] aspect-square relative z-0">
              <OrbitImages
                images={orbitImagesData}
                shape="ellipse"
                direction="normal"
                duration={40}
                fill={true}
                showPath={false}
                responsive={true}
                baseWidth={800}
                progressOverride={orbitProgress}
                radiusXOverride={orbitRx}
                radiusYOverride={orbitRy}
                itemSizeOverride={orbitItemSize}
                rotationOverride={orbitRotation}
                translateXOverride={orbitTx}
                focusStrength={focusStrength}
              />
            </motion.div>
          </div>
        </motion.div>

        <div className="absolute inset-0 z-[60] pointer-events-none">
            <div className="absolute top-[48%] left-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-none z-50">
              <motion.div 
                className="flex flex-col items-center whitespace-nowrap pointer-events-auto"
                style={{ filter: filterText, opacity: textOpacity, WebkitFontSmoothing: 'antialiased', WebkitBackfaceVisibility: 'hidden', transform: 'translateZ(0)' }}
              >
                <div className="flex items-baseline text-black leading-none mb-1">
                  <span className="font-serif text-[45px] md:text-[55px] italic tracking-tight text-black">M</span>
                  <span className="font-serif text-[45px] md:text-[55px] tracking-tight text-black">aster the Elements</span>
                </div>
                <span className="font-sans text-[28px] md:text-[36px] tracking-tight text-black mt-[-5px]">embrace</span>
              </motion.div>
            </div>

            <motion.div 
              className="absolute top-32 right-[calc(6vw+150px)] md:right-[214px] flex flex-col items-start text-left pointer-events-auto cursor-text"
              style={{ y: yElement, filter: filterText, opacity: textOpacity }}
            >
              <span className="font-serif text-[40px] leading-none mb-3 text-black">2K26</span>
              <span className="font-serif text-[16px] uppercase tracking-widest text-black leading-[20px] text-left">
                JOIN AN EXCLUSIVE<br />COMMUNITY
              </span>
            </motion.div>

            <motion.div 
              className="absolute bottom-8 left-8 md:bottom-16 md:left-16 flex flex-col items-start text-black pointer-events-auto cursor-text"
              style={{ y: yElement, filter: filterText, opacity: textOpacity }}
            >
              <span className="font-serif text-[40px] leading-none mb-1 text-black">0651</span>
              <span className="font-serif text-[16px] uppercase tracking-widest text-black">COLLECTION</span>
            </motion.div>

            <div className="absolute bottom-16 right-[6vw] md:right-[10vw] flex flex-col items-start z-10 pointer-events-auto">
              <motion.p 
                className="font-serif text-[16px] uppercase tracking-widest text-black leading-[20px] mb-6 text-left w-[240px] cursor-text"
                style={{ y: yElement, filter: filterText, opacity: textOpacity }}
              >
                JOIN AN EXCLUSIVE COMMUNITY OF SAILORS. WHETHER YOU CRAVE THE THRILL OF THE OPEN
              </motion.p>
              <motion.div className="flex gap-0 pointer-events-auto items-center" style={{ y: yElement, filter: filterText, opacity: textOpacity }}>
                <button className="bg-black hover:bg-black/90 transition-colors text-white rounded-[40px] px-8 py-3.5 font-serif tracking-[0.1em] uppercase text-[12px] md:text-[14px] z-10">
                  BUY COLLECTION
                </button>
                <button className="bg-black hover:bg-black/90 transition-colors w-[46px] h-[46px] flex items-center justify-center rounded-[50%] text-white -ml-2 z-0">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="ml-1">
                    <path d="M5 12h14M12 5l7 7-7 7"/>
                  </svg>
                </button>
              </motion.div>
            </div>
        </div>

        <motion.header 
          className="fixed top-0 left-0 w-full p-6 md:p-10 flex justify-between items-start z-[100] pointer-events-none"
          style={{ opacity: textOpacity, filter: filterText }}
        >
          <div className="flex items-start text-black select-none leading-none pointer-events-auto" style={{ fontFamily: "'Instrument Serif', serif", WebkitFontSmoothing: "antialiased" }}>
            <span style={{ fontSize: '40px' }}>Shamoni</span>
            <span style={{ fontSize: '14px', marginLeft: '4px', marginTop: '4px' }}>©</span>
          </div>

          <button className="group relative flex items-center justify-center w-[72px] h-[44px] hover:scale-105 transition-transform duration-300 cursor-pointer pointer-events-auto" aria-label="Menu">
            <div className="absolute inset-0 bg-black rounded-[50%] -rotate-15"></div>
            <svg className="relative z-10" width="24" height="10" viewBox="0 0 24 10" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M1 1H23M1 9H23" stroke="white" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </motion.header>

      </div>
    </div>
  );
}

## Slam Dunk — Hero Section [sites/slam-dunk-hero]

- Preview: https://motionsites.ai/assets/hero-slam-dunk-preview-Cmg3K_S4.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/slam-dunk-hero.gif

Built with Google AI Studio. Open the live app via the Google AI Studio link to remix.

## Stellar AI — Hero Section [sites/stellar-ai-hero]

- Preview: https://motionsites.ai/assets/hero-stellar-ai-preview-D3HL6bw1.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/stellar-ai-hero.gif

Create a "Stellar.ai" landing page hero section using React, Tailwind CSS, and Lucide React icons. Use the Inter font (imported from Google Fonts). The page has a white background (bg-white), max-width max-w-7xl, and is centered with mx-auto.

Font: Import Inter (weights 400, 500, 600, 700) from Google Fonts. Set font-family: 'Inter', sans-serif on the body.

Custom CSS animations (in index.css):

@keyframes fadeInUp -- from opacity: 0; transform: translateY(30px) to opacity: 1; transform: translateY(0). Class .animate-fade-in-up uses this with 0.6s ease-out forwards.
@keyframes fadeInOverlay -- from opacity: 0 to opacity: 1. Class .animate-fade-in-overlay uses this with 0.4s ease-out forwards.
@keyframes fadeInDialog -- from opacity: 0 to opacity: 1. Class .animate-slide-up-overlay uses this with 0.5s ease-out forwards and has transform: translate(-50%, -50%).
Every major section uses .animate-fade-in-up with staggered animationDelay inline styles (starting at 0.1s and incrementing by 0.1s). Each element starts with opacity: 0 inline so the animation fills it to visible.

Tailwind config: Default config with no custom theme extensions. Uses standard content paths.

NAVIGATION (animationDelay: 0.1s):
px-6 py-4 flex items-center justify-between max-w-7xl mx-auto
Left: Lucide Star icon (w-5 h-5, fill-black) + "Stellar.ai" text (text-lg font-semibold)
Center (hidden on mobile, hidden md:flex items-center gap-8): "Solutions" with ChevronDown, "For Teams" with ChevronDown, "About Us", "Learn Hub" -- all text-sm text-gray-700 hover:text-black
Right: "Login" link (text-sm text-gray-700) + "Get started free" button (bg-black text-white px-5 py-2.5 rounded-full text-sm font-medium hover:bg-gray-800 transition-colors)

HERO SECTION (px-6 pt-24 pb-32 max-w-7xl mx-auto text-center):
Reviews Badge (delay: 0.2s): inline-flex items-center gap-2 mb-8. Contains a bordered square (w-6 h-6 border border-gray-300 rounded) with a filled Star icon inside, plus "4.9 rating from 18.3K+ users" (text-sm font-medium text-black).

Main Heading (delay: 0.3s): text-6xl md:text-7xl lg:text-[80px] font-normal leading-[1.1] tracking-tight mb-5. First line: "Work Smarter. Move Faster." Second line: "AI Powers You Up." with gradient text (bg-gradient-to-r from-black via-gray-500 to-gray-400 bg-clip-text text-transparent).

Subheading (delay: 0.4s): text-lg md:text-xl text-gray-600 mb-8 max-w-2xl mx-auto. Text: "Intelligent automation syncs with the tools you love to streamline tasks, boost output, and save time."

CTA Button (delay: 0.5s): bg-black text-white px-8 py-3 rounded-full text-base font-medium hover:bg-gray-800 transition-colors mb-12. Text: "Begin Free Trial".

Tab Bar (delay: 0.6s): Centered bg-gray-100 rounded-lg p-1 container.
Mobile (md:hidden): 2x2 grid with 4 buttons: Analyse (BarChart3), Train (BookOpen), Testing (Users), Deploy (Rocket). Active: bg-white text-black shadow-sm. Inactive: text-gray-600.
Desktop (hidden md:flex): Same 4 buttons in row with vertical dividers (w-px h-5 bg-gray-300).
Tabs auto-cycle every 4s using setInterval. State: useState('analyse').

Video + Overlay Section (delay: 0.7s):
Container: relative rounded-3xl overflow-hidden h-[400px] md:h-[500px]
Video: src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260319_165750_358b1e72-c921-48b7-aaac-f200994f32fb.mp4", autoPlay, loop, muted, playsInline, w-full h-full object-cover.

4 Conditional Overlays per tab with animate-fade-in-overlay outer and animate-slide-up-overlay inner card:
a. Analyse: "Set Up Your AI Workspace" wizard with purple progress bar at 25%, 4 steps
b. Train: "AI Model Training" with orange progress at 67%, 4 metrics
c. Testing: "Test Suite Results" with green success, 127/127 tests
d. Deploy: "Deploy to Production" with 4 checklist items, Deploy Now button

Company Logos (delay: 0.8s): mt-24 flex with INTERSCOPE, SPOTIFY, Nexera (dot grid), M3 (serif italic), LAURA COLE (LC circle), vertex (dots)

## Sync AI — Hero Section [sites/stellar-ai-v2-hero]

- Preview: https://motionsites.ai/assets/hero-stellar-ai-v2-preview-DjvxjG3C.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/stellar-ai-v2-hero.gif

Create a full-viewport hero section for a SaaS landing page called "Stellar.ai" using React, TypeScript, Tailwind CSS, and Lucide React icons. The design uses the Inter font (weights 400, 500, 600, 700) imported from Google Fonts. No other dependencies beyond lucide-react, react, and react-dom.

OVERALL STRUCTURE

The page is a full-screen (h-screen) white background container with overflow-hidden. Everything is contained in a single viewport. There is no scrolling. The layout stacks vertically: navbar at top, hero content in upper-center, and a partner logo bar pinned to the bottom.

BACKGROUND VIDEO

A looping, muted, autoplaying, inline video fills the entire viewport as an absolute-positioned background
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260330_153826_e9005cf7-a1c7-4c7d-886f-fea22d644a9c.mp4
CSS: absolute inset-0 w-full h-full object-cover
The video has top padding to push it down below the hero text: pt-[120px] on mobile, md:pt-[200px] on desktop
This creates the effect where the video content appears below the text area

VIDEO FADE-OUT OVERLAYS (White gradient masks)

Three absolute-positioned gradient overlays sit on top of the video (z-10) to fade the video into the white background at the top:

General overlay: top: 120px, height: 200px, gradient from white to transparent
Desktop-only overlay (hidden on mobile, hidden md:block): top: 200px, height: 300px, gradient from white to transparent
Mobile-only overlay (md:hidden): top: 120px, height: 200px, gradient from white to transparent
All overlays use pointer-events-none so they don't block interaction.

NAVBAR (z-20, relative)

Max width max-w-7xl, centered, horizontal padding px-4 sm:px-6, vertical padding py-4
Flex row, items-center justify-between
Animated with animate-fade-in-up at animationDelay: 0.1s, initial opacity: 0

Left side (logo):
Lucide Star icon, w-5 h-5 fill-black
Text "Stellar.ai", text-lg font-semibold

Center nav (hidden on mobile, hidden md:flex, gap-8):
"Solutions" button with ChevronDown icon (w-4 h-4), text-sm text-gray-700 hover:text-black
"For Teams" button with ChevronDown icon, same styling
"About Us" button, text-sm text-gray-700 hover:text-black
"Learn Hub" button, same styling

Right side (hidden on mobile hidden sm:flex, gap-4):
"Login" text button, text-sm text-gray-700 hover:text-black
"Get started free" button: bg-black text-white px-5 py-2.5 rounded-full text-sm font-medium hover:bg-gray-800 transition-colors

Mobile hamburger (sm:hidden):
Toggles between Menu and X icons from Lucide (w-6 h-6)

MOBILE MENU (conditionally rendered when open)

Positioned absolute top-[60px] left-0 right-0 z-30
Background: bg-white/95 backdrop-blur-md border-b border-gray-200
Animated with animate-fade-in-overlay
Contains same nav items as desktop, stacked vertically with px-6 py-4 gap-4
Login and "Get started free" buttons separated by a border-t border-gray-200 pt-4
The CTA button is full width in mobile menu

HERO CONTENT (z-20, relative)

Container: px-4 sm:px-6 pt-6 sm:pt-12 pb-16 sm:pb-32 max-w-7xl mx-auto text-center

Rating badge (animationDelay: 0.2s):
inline-flex items-center gap-2 mb-5 sm:mb-8
Small box: w-6 h-6 border border-gray-300 rounded flex items-center justify-center containing a filled Star icon (w-4 h-4 fill-black)
Text: "4.9 rating from 18.3K+ users", text-xs sm:text-sm font-medium text-black

Heading (animationDelay: 0.3s):
Font sizes: text-[38px] sm:text-6xl md:text-7xl lg:text-[80px]
font-normal leading-[1.1] tracking-tight mb-4 sm:mb-5
Mobile layout (sm:hidden): Three lines -- "Work Smarter." / "Move Faster." / "AI Powers You Up."
Desktop layout (hidden sm:inline): Two lines -- "Work Smarter. Move Faster." / "AI Powers You Up."
"AI Powers You Up." uses a gradient text effect: bg-gradient-to-r from-black via-gray-500 to-gray-400 bg-clip-text text-transparent

Subheading (animationDelay: 0.4s):
text-base sm:text-lg md:text-xl text-gray-600 mb-6 sm:mb-8 max-w-2xl mx-auto px-2
Text: "Intelligent automation syncs with the tools you love to streamline tasks, boost output, and save time."

CTA button (animationDelay: 0.5s):
bg-black text-white px-6 sm:px-8 py-3 rounded-full text-sm sm:text-base font-medium hover:bg-gray-800 transition-colors
Text: "Begin Free Trial"

BOTTOM PARTNER BAR (z-20, absolute bottom-0)

Container: absolute bottom-0 left-0 right-0 z-20 flex flex-col items-center gap-3 sm:gap-4 pb-4 sm:pb-8 px-4
Animated: animate-fade-in-up at animationDelay: 0.6s, initial opacity: 0

Glass pill badge:
rounded-full px-3 sm:px-3.5 py-1
text-[10px] sm:text-xs font-medium text-white
Frosted glass effect: backdrop-blur-md bg-white/15 border border-white/20
Text: "Collaborating with top aerospace pioneers globally"

Partner logos (text-based, no images):
Flex row: gap-5 sm:gap-12 md:gap-16 flex-wrap justify-center
Five names: "Aeon", "Vela", "Apex", "Orbit", "Zeno"
Each: text-lg sm:text-2xl md:text-3xl italic text-white tracking-tight with inline style fontFamily: 'Georgia, serif'

ANIMATIONS (defined in index.css)

@keyframes fadeInUp {
  from { opacity: 0; transform: translateY(30px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-in-up { animation: fadeInUp 0.6s ease-out forwards; }

@keyframes fadeInOverlay {
  from { opacity: 0; }
  to { opacity: 1; }
}
.animate-fade-in-overlay { animation: fadeInOverlay 0.4s ease-out forwards; }

Each element uses staggered delays (0.1s through 0.6s) applied via inline style={{ animationDelay: 'X.Xs', opacity: 0 }}.

FONT
Google Fonts import: @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
Applied globally: body { font-family: 'Inter', sans-serif; }

KEY DESIGN NOTES
The "liquid glass" effect comes from the frosted-glass partner badge using backdrop-blur-md bg-white/15 border border-white/20
The mobile menu also uses glass: bg-white/95 backdrop-blur-md
No purple/indigo colors -- entire palette is black, white, and grays
The heading gradient goes from pure black through gray-500 to gray-400
The video is visible primarily in the lower half, with white gradients dissolving it into the clean white upper section
Color palette: strictly monochrome

## Taskly — Hero Section [sites/taskly-hero]

- Preview: https://motionsites.ai/assets/hero-taskly-preview-Dq2MKaI0.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/taskly-hero.gif

System Prompt: High-Fidelity "Liquid Glass" Hero Section

Core Layout: Create a 1600px max-width landing page hero section. The background should be pure white with a subtle, layered gradient glow in the top-left (using blurred ellipses in light blue #60B1FF and #319AFF). The design must be fully responsive, transitioning from a single-column mobile view to a dual-column desktop layout.

Typography:

Headlines & Brand: Use Fustat (Bold).
Body & UI: Use Inter (Normal/Medium).
Hero Headline: "Work smarter, achieve faster" (75px, 1.05 line-height, -2px tracking).

The "Strong Liquid Glass" Navbar:

Position: Sticky at top-[30px], centered, w-fit.
Visuals: backdrop-blur-[50px], background rgba(255,255,255,0.3), rounded-[16px].
Fidelity Details:
Outer Stroke: 1px solid rgba(0,0,0,0.1).
Inner Highlight Shadow: inset 0px 4px 4px 0px rgba(255,255,255,0.25).
Items: Logo "Taskly" (Fustat), Nav links (Home, Features, Company, Pricing), and a glassy "SignUp" button with an arrow icon.

The Glassy Orb (Hero Right):

Source URL: https://future.co/images/homepage/glassy-orb/orb-purple.webm
Blending Mode: Must use mix-blend-screen to filter the black background.
Scaling: scale-125 to make it massive and bleed slightly off-center.
Exact Color Grade (CSS Filter): hue-rotate(-55deg) saturate(250%) brightness(1.2) contrast(1.1). This transforms the purple asset into a vibrant, high-end "Electric Brand Blue" that matches the primary CTA.

Hero Content (Hero Left):

Social Proof: A "Rated 4.9/5 by 2700+ customers" badge with five orange #FF801E stars.
Subheadline: "Effortlessly manage your projects, collaborate with your team, and achieve your goals with our intuitive task management tool." (18px, Inter, -1px tracking).
Primary CTA: "Get Started Now" button.
Color: rgba(0,132,255,0.8) with backdrop-blur-[2px].
Details: rounded-[16px], white text, inner highlight shadow inset 0px 4px 4px 0px rgba(255,255,255,0.35), and a white circular arrow icon.
Animation: Scale 1.02 on hover with a smooth transition.

Footer Logos: Include a "Trusted by Top-tier product companies" section at the bottom with 5 grayscale SVG logos (e.g., placeholder logos for tech companies) spaced at gap-[100px].

Key Technical Specs for the Developer:

Video Tag: autoPlay loop muted playsInline.
Container: Use a relative wrapper for the background glow and a z-10 main container for the content.
Smoothing: Apply -webkit-font-smoothing: antialiased for the sharpest typography.

## Transform Data — Hero Section [sites/transform-data-hero]

- Preview: https://motionsites.ai/assets/hero-transform-data-preview-Cx5OU29N.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/transform-data-hero.gif

HERO SECTION CREATION PROMPT

Create a modern hero section with a looping video background and the following specifications:

Video Background:

URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260329_050842_be71947f-f16e-4a14-810c-06e83d23ddb5.mp4

Size: 115% width and height

Position: Centered horizontally, anchored to top with object-top focal point

Custom JavaScript fade system (NO CSS transitions):

250ms requestAnimationFrame-based fade-in on load/loop start

250ms fade-out when 0.55 seconds remain before video end

fadingOutRef boolean prevents re-triggering fade-out from repeated timeUpdate events

On ended: opacity set to 0, 100ms delay, reset to currentTime = 0, play, fade back in

Each new fade cancels running animation frames to prevent competing animations

Fades resume from current opacity (no snapping)

Fonts Required:

Schibsted Grotesk (weights: 400, 500, 600, 700)

Inter (weights: 400, 500, 600, 700)

Noto Sans (weights: 400, 500, 600, 700)

Fustat (weights: 400, 500, 600, 700)

Navigation Bar:

Logo: "Logoipsum" (Schibsted Grotesk SemiBold, 24px, -1.44px tracking)

Menu items (Schibsted Grotesk Medium, 16px, -0.2px tracking):

Platform

Features (with dropdown chevron icon)

Projects

Community

Contact

Right side buttons:

"Sign Up" (transparent background, 82px width)

"Log In" (black background, white text, 101px width)

Padding: 120px horizontal, 16px vertical

Hero Content (moved up 50px with -mt-[50px]):

Badge Component:

Dark badge with star icon + "New" text

Light background with text: "Discover what's possible"

Font: Inter Regular, 14px

Rounded corners with subtle shadow

Main Headline:

Text: "Transform Data Quickly"

Font: Fustat Bold, 80px, -4.8px tracking, line-height: none

Color: Black, center-aligned

Subtitle:

Text: "Upload your information and get powerful insights right away. Work smarter and achieve goals effortlessly."

Font: Fustat Medium, 20px, -0.4px tracking

Color: #505050

Max-width: 736px, width: 542px

Search Input Box:

Backdrop blur with dark transparent background (rgba(0,0,0,0.24))

Dimensions: 728px max-width, 200px height, rounded 18px

Top row: Credit info

Left: "60/450 credits" with green "Upgrade" button

Right: AI icon + "Powered by GPT-4o"

Font: Schibsted Grotesk Medium, 12px, white text

Main input area:

White background, rounded 12px, shadow

Placeholder: "Type question..." (16px, rgba(0,0,0,0.6))

Black circular submit button with up arrow icon (36px size)

Bottom row:

Left: Three action buttons (gray backgrounds, rounded 6px):

"Attach" with paperclip icon

"Voice" with microphone icon

"Prompts" with search icon

Right: Character counter "0/3,000" (12px, gray)

Icons (SVG paths from imported file):

Chevron down arrow

Up arrow

Star icon

AI sparkle icon

Attach/paperclip icon

Voice/microphone icon

Search icon

Spacing:

Gap between navigation and hero: 60px

Gap between header and search box: 44px

Gap within header elements: 34px (badge to title, title to subtitle)

Hero content moved up: 50px negative margin

Horizontal padding: 120px

Color Scheme:

Black text: #000000

Gray text: #505050

Light gray backgrounds: #f8f8f8

Green upgrade button: rgba(90,225,76,0.89)

Dark badge: #0e1311

White: #ffffff

Transparent overlay: rgba(0,0,0,0.24)

Component Structure:

VideoBackground component with custom fade logic

Navigation bar (fixed spacing, horizontal layout)

Hero content container (centered, max-width constraints)

Nested components for badge, header, and search input

All elements positioned over full-screen video background

## VertexAI Hero — Hero Section [sites/vertex-ai-hero]

- Preview: https://motionsites.ai/assets/hero-vertex-ai-preview-Da80y3xa.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vertex-ai-hero.gif

Build a React + TypeScript + Vite hero section for a fictional brand "VertexAI". Render a full-viewport hero with a looping background video, a frosted-glass navbar, a centered headline using mixed sans + italic-serif typography, and a footer row with a description on the left and tag buttons on the right. Follow this spec verbatim — class names, values, copy, SVG paths, padding, opacity, and all CSS must match exactly.

Project setup
React 19 + TypeScript + Vite. Files: index.html, src/main.tsx, src/App.tsx, src/App.css, src/index.css, src/components/Navbar.tsx, src/components/Navbar.css, src/components/HeroContent.tsx, src/components/HeroContent.css, src/components/FooterElements.tsx, src/components/FooterElements.css.
main.tsx mounts <App /> inside <StrictMode> and imports ./index.css.
Body: overflow: hidden, min-height: 100vh, dark scheme.
Fonts (load in index.css)
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Cormorant+Garamond:ital,wght@1,400;1,500;1,600&display=swap');
Base font-family: 'Inter', system-ui, -apple-system, sans-serif. Background #0a0a0a, text rgba(255,255,255,0.87).

index.css (design tokens + globals)
:root {
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  line-height: 1.5;
  font-weight: 400;
  color-scheme: dark;
  color: rgba(255, 255, 255, 0.87);
  background-color: #0a0a0a;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;

  --glass-bg: rgba(255, 255, 255, 0.05);
  --glass-border: rgba(255, 255, 255, 0.1);
  --glass-blur: blur(12px);
  --primary-white: #ffffff;
  --secondary-white: rgba(255, 255, 255, 0.7);
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  margin: 0;
  display: flex;
  place-items: center;
  min-width: 320px;
  min-height: 100vh;
  background-color: #0a0a0a;
  overflow: hidden;
}

#root { width: 100%; }

.glass {
  background: rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.pill { border-radius: 16px; }

button { cursor: pointer; border: none; font-family: inherit; transition: all 0.3s ease; }
a { text-decoration: none; color: inherit; }
App.tsx
import './App.css'
import Navbar from './components/Navbar'
import HeroContent from './components/HeroContent'
import FooterElements from './components/FooterElements'

function App() {
  return (
    <main className="hero-section">
      <video
        className="hero-bg-video"
        src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260503_162107_3cd240af-dff4-4396-b8b7-22e25c9adb1c.mp4"
        autoPlay
        loop
        muted
        playsInline
      />
      <Navbar />
      <HeroContent />
      <FooterElements />
    </main>
  )
}

export default App
App.css
.hero-section {
  position: relative;
  width: 100vw;
  height: 100vh;
  padding: 20px 20px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: center;
  z-index: 1;
  overflow: hidden;
  background: #0a0a0a;
}

.hero-bg-video {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  z-index: -1;
  pointer-events: none;
}

@media (max-width: 768px) {
  .hero-section { padding: 30px 20px; }
}
The video element must include autoPlay, loop, muted, playsInline (lowercase HTML attributes) so it autoplays inline on every browser. No overlay — the video shows through directly behind everything.

Navbar.tsx
import './Navbar.css'

const Navbar = () => {
  return (
    <nav className="navbar">
      <div className="logo-container">
        <div className="logo-placeholder">
          <div className="logo-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="8" cy="8" r="4" fill="white" fillOpacity="0.8" />
              <circle cx="16" cy="8" r="4" fill="white" fillOpacity="0.8" />
              <circle cx="8" cy="16" r="4" fill="white" fillOpacity="0.8" />
              <circle cx="16" cy="16" r="4" fill="white" fillOpacity="0.8" />
            </svg>
          </div>
          <span className="brand-name"><a href="">VertexAI</a></span>
        </div>
      </div>

      <div className="nav-main glass pill">
        <div className="nav-links">
          <a href="#product" className="nav-link">Product</a>
          <a href="#platform" className="nav-link">Platform</a>
          <a href="#customers" className="nav-link">Customers</a>
          <a href="#company" className="nav-link">Company</a>
        </div>
        <button className="login-btn pill">Login</button>
      </div>
    </nav>
  )
}

export default Navbar
Navbar.css
.navbar {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 20px;
}

.logo-placeholder { display: flex; align-items: center; gap: 12px; }

.brand-name { font-size: 19px; font-weight: 600; letter-spacing: -0.5px; margin-top: -5px; }

.nav-main {
  display: flex;
  align-items: center;
  padding: 6px 6px 6px 32px;
  gap: 28px;
  background: rgba(20, 18, 16, 0.42);
  border: 1px solid rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  border-radius: 16px;
}

.nav-links { display: flex; gap: 25px; }

.nav-link {
  font-size: 13px;
  font-weight: 500;
  color: var(--secondary-white);
  transition: color 0.3s ease;
}
.nav-link:hover { color: var(--primary-white); }

.login-btn {
  background: var(--primary-white);
  color: #000;
  padding: 10px 26px;
  font-size: 14px;
  font-weight: 600;
  border-radius: 12px;
}
.login-btn:hover {
  background: rgba(255, 255, 255, 0.9);
  transform: translateY(-1px);
}

@media (max-width: 900px) {
  .nav-links { display: none; }
  .nav-main { padding: 6px; gap: 0; }
}
The navbar consists of two parts: a logo block on the left (4-circle SVG mark + brand wordmark "VertexAI") and a darkened glass pill on the right that combines the nav links and the white Login button. The pill uses warm dark fill rgba(20, 18, 16, 0.42) plus an 18px backdrop-blur and a 1px white-8% border, with a border-radius of 16px (rounded rectangle, not a true pill). The Login button uses a tighter 12px radius.

HeroContent.tsx
import './HeroContent.css'

const HeroContent = () => {
  return (
    <div className="hero-content">
      <h1 className="hero-title">
        <span className="sans-bold">Meet VertexAI.</span>
        <br />
        <span className="serif-italic">Redefine space</span>
        <span className="sans-light"> with</span>
        <br />
        <span className="sans-light">intelligent design</span>
      </h1>
      <div className="cta-container">
        <button className="cta-btn pill">Start free decoration</button>
      </div>
    </div>
  )
}

export default HeroContent
The headline must render across three visual lines with a leading space before "with":

Meet VertexAI. — Inter, regular weight
Redefine space with — "Redefine space" in italic Cormorant Garamond at 1.14em of the base headline size; "with" in regular Inter at 1em
intelligent design — Inter, regular weight
HeroContent.css
.hero-content {
  text-align: center;
  max-width: 900px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 28px;
}

.hero-title {
  font-size: clamp(36px, 4.4vw, 72px);
  line-height: 0.95;
  letter-spacing: -0.022em;
  color: var(--primary-white);
  margin-bottom: 0;
  font-weight: 400;
}

.sans-bold  { font-weight: 400; font-size: 1em; }
.sans-light { font-weight: 400; font-size: 1em; }

.serif-italic {
  font-family: 'Cormorant Garamond', serif;
  font-style: italic;
  font-weight: 400;
  font-size: 1.14em;
  letter-spacing: -0.01em;
}

.cta-btn {
  background: var(--primary-white);
  color: #000;
  padding: 15px 25px;
  font-size: 13px;
  font-weight: 600;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.18);
  border-radius: 12px;
  margin-top: 0;
}
.cta-btn:hover {
  transform: scale(1.05);
  box-shadow: 0 15px 40px rgba(255, 255, 255, 0.15);
}

@media (max-width: 768px) {
  .hero-title { letter-spacing: -1px; }
}
FooterElements.tsx
import './FooterElements.css'

const FooterElements = () => {
  return (
    <div className="footer-elements">
      <div className="footer-left">
        <p className="description">
          It helps you imagine, plan, and refine spaces<br />
          through natural conversations.<br />
          From choosing colors and layouts to suggesting<br />
          furniture and décor, it adapts to your taste.
        </p>
      </div>
      <div className="footer-right">
        <button className="tag-btn glass pill">Solutions for complex spaces</button>
        <div className="action-row">
          <button className="icon-btn glass pill">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M7 17L17 7M17 7H7M17 7V17" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </button>
          <button className="tag-btn glass pill">Conversational & Action</button>
        </div>
      </div>
    </div>
  )
}

export default FooterElements
FooterElements.css
.footer-elements {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding: 0 20px 36px;
}

.footer-left { max-width: 400px; }

.description {
  font-size: 15px;
  line-height: 1.18;
  color: white;
  font-weight: 400;
  opacity: 0.8;
}

.footer-right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 12px;
}

.action-row { display: flex; gap: 12px; }

.tag-btn {
  background: transparent;
  color: var(--primary-white);
  padding: 10px 22px;
  font-size: 13px;
  font-weight: 500;
  border: 1px solid var(--glass-border);
  border: 0.75px solid white;
  border-radius: 16px;
}
.tag-btn:hover {
  background: var(--glass-bg);
  border-color: var(--primary-white);
}

.icon-btn {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.icon-btn:hover {
  background: var(--glass-bg);
  transform: rotate(45deg);
}

@media (max-width: 900px) {
  .footer-elements { flex-direction: column; align-items: center; gap: 40px; text-align: center; }
  .footer-right { align-items: center; }
}
Animations / interactions
All buttons inherit transition: all 0.3s ease from the global button selector.
Login button: hover lifts 1px (translateY(-1px)) and dims white to 90%.
CTA "Start free decoration": hover scales to 1.05 and intensifies the white glow shadow.
Tag buttons: hover gains the --glass-bg fill and the border brightens to full white.
Icon button (arrow): hover rotates 45° and gains the glass background — the arrow appears to flip toward bottom-right.
Nav links: hover transitions text color from --secondary-white (white 70%) to --primary-white.
Background video: loops continuously, muted, autoplays inline, object-fit: cover over the full viewport.
Acceptance checklist
Hero section is exactly 100vw × 100vh, with a 20px outer padding, content distributed top/middle/bottom via flexbox.
Looping CloudFront video plays behind everything at z-index -1 with no tint or overlay.
Top row: left-side logo (4 white-80% circles SVG + "VertexAI" wordmark) and right-side dark glass nav pill containing 4 nav links + white Login button (12px radius).
Middle: three-line headline as specified, with Cormorant Garamond italic only on "Redefine space"; centered; clamp(36px, 4.4vw, 72px); line-height 0.95. White CTA "Start free decoration" with padding: 15px 25px, border-radius: 12px, soft shadow.
Bottom: 4-line description on the left at 15px / 1.18 line-height, opacity 0.8; right column has "Solutions for complex spaces" stacked over a row of [arrow icon button] + "Conversational & Action". All three buttons use a 16px-radius outlined glass treatment with a 0.75px white border. The footer row sits 36px above the bottom edge.
Below 900px viewport, nav links hide and footer stacks vertically; below 768px the section padding becomes 30px 20px and headline letter-spacing tightens to -1px.

## VEX Ventures — Hero Section [sites/vex-ventures-hero]

- Preview: https://motionsites.ai/assets/hero-vex-ventures-preview-BczMFIiw.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vex-ventures-hero.gif

Recreate this hero section exactly. Here are the complete specifications:

Video Background:

Full-screen background video, absolutely positioned, covering the entire viewport (object-cover)
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260403_050628_c4e32401-fab4-4a27-b7a8-6e9291cd5959.mp4
Autoplay, loop, muted, playsInline
NO dark overlay, NO gradient overlay, NO semi-transparent layer on top of the video. The video plays raw with no dimming whatsoever.
Typography (CRITICAL - must be applied globally):

Import the Google Font Inter via a <link> tag in index.html:

<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap" rel="stylesheet">
Set the body font-family in CSS to: 'Inter', sans-serif
Apply -webkit-font-smoothing: antialiased and -moz-osx-font-smoothing: grayscale on the body
Also extend the Tailwind config to set fontFamily: { sans: ['Inter', 'sans-serif'] } so all Tailwind font-sans usage picks up Inter automatically
Navbar:

Wrapped in horizontal page padding: px-6 md:px-12 lg:px-16 with pt-6 top padding
The navbar bar itself uses the .liquid-glass class and has rounded-xl, px-4 py-2, flex layout with items-center justify-between
Left: Logo text "VEX" - text-2xl font-semibold tracking-tight
Center (hidden on mobile, visible md+): Links "Story", "Investing", "Building", "Advisory" - text-sm, gap-8, hover transitions to gray-300
Right: "Start a Chat" button - bg-white text-black px-6 py-2 rounded-lg text-sm font-medium, hover to gray-100
Hero Content (Bottom of viewport):

Container: same horizontal padding as navbar, flex column filling remaining height, content pushed to bottom with flex-1 flex flex-col justify-end, bottom padding pb-12 lg:pb-16
On large screens: 2-column grid (lg:grid lg:grid-cols-2 lg:items-end)
Left Column - Main content:

Heading: "Shaping tomorrow\nwith vision and action." (literal line break between "tomorrow" and "with")

Responsive sizes: text-4xl md:text-5xl lg:text-6xl xl:text-7xl
font-normal, mb-4
Inline style: letterSpacing: '-0.04em'
Character-by-character entrance animation: Each character starts at opacity: 0 and translateX(-18px), then transitions to opacity: 1 and translateX(0). Each character gets a staggered delay calculated as: (lineIndex * lineLength * charDelay) + (charIndex * charDelay) where charDelay = 30ms. The whole animation starts after 200ms initial delay. Each character transition is 500ms.
Spaces render as \u00A0 (non-breaking space)
Subheading: "We back visionaries and craft ventures that define what comes next."

text-base md:text-lg text-gray-300 mb-5
Fade-in animation: starts at 800ms delay, 1000ms duration
Buttons row: flex-wrap with gap-4

"Start a Chat" - bg-white text-black px-8 py-3 rounded-lg font-medium
"Explore Now" - liquid-glass border border-white/20 text-white px-8 py-3 rounded-lg font-medium, hover transitions to white bg + black text
Fade-in animation: starts at 1200ms delay, 1000ms duration
Right Column - Tag:

Aligned to bottom-right on large screens (flex items-end justify-start lg:justify-end)
Glass card: liquid-glass border border-white/20 px-6 py-3 rounded-xl
Text: "Investing. Building. Advisory." - text-lg md:text-xl lg:text-2xl font-light
Fade-in animation: starts at 1400ms delay, 1000ms duration
Liquid Glass CSS (place in global CSS):


.liquid-glass {
  background: rgba(0, 0, 0, 0.4);
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
    rgba(255,255,255,0.3) 0%, rgba(255,255,255,0.1) 20%,
    rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.1) 80%, rgba(255,255,255,0.3) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}
FadeIn component: A wrapper that starts with opacity: 0 and transitions to opacity: 1 after a configurable delay (ms) using a setTimeout + React state. Transition duration is also configurable. Uses inline transitionDuration style and Tailwind's transition-opacity class.

AnimatedHeading component: Splits text by \n into lines, then each line into individual characters. Each character is an inline-block <span> with CSS transitions on opacity and transform (translateX). Animation triggers via React state after the initial delay.

Color scheme: Black background, white text, gray-300 for secondary text, white/20 for borders. No purple, no indigo.

Stack: React + TypeScript, Tailwind CSS, Vite. No extra UI libraries needed. Icons from lucide-react if needed (none currently used in the hero).

## WISA Space — Hero Section [sites/wisa-space-hero]

- Preview: https://motionsites.ai/assets/hero-wisa-space-preview-CAIFtU8c.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/wisa-space-hero.gif

Google AI Studio app – no prompt text

## xPortfolio Hero — Hero Section [sites/xportfolio-hero]

- Preview: https://motionsites.ai/assets/hero-xportfolio-preview-D4A8maiC.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/xportfolio-hero.gif

Build a single-page hero section for a brand design agency called "Brandly" using React, Tailwind CSS, and Lucide React icons. The entire page is one viewport-height screen with no scrolling. It uses a fullscreen background video with all content layered on top.

Fonts (loaded via Google Fonts in index.html):
Inter (weights: 300, 400, 500, 600, 700) -- used as the base font for the entire page
Anton (regular 400) -- used for all large uppercase headings

Page Container:
Full viewport height (h-screen), overflow-hidden, flex flex-col
Background color: #F5F3EE (warm off-white/cream)
Base font-family set via inline style: fontFamily: 'Inter, sans-serif'

Background Video:
A video element with autoPlay, loop, muted, playsInline
Positioned fixed top-0 left-0 w-full h-full object-cover pointer-events-none with zIndex: 0
Video source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_102305_3a7cab3b-7a86-46e8-a0f9-6937f035b087.mp4
Type: video/mp4

Header (zIndex: 10):
relative, padding px-6 lg:px-12 py-4 lg:py-6, flex-shrink-0
Nav: flex items-center justify-between
Left: Logo text "Brandly" -- text-2xl lg:text-3xl font-bold text-black
Center (hidden on mobile, shown md+): Navigation links "About", "Features", "Pricing", "FAQ", "Help" -- flex items-center gap-8 text-base lg:text-lg, color #080808
Right: Two buttons side by side (flex items-center gap-3):
"Sign Up" -- px-4 lg:px-6 py-2 text-base lg:text-lg hover:text-black transition, color #080808
"Log In" -- px-4 lg:px-6 py-2 bg-black text-white text-base lg:text-lg hover:bg-gray-800 transition

Main Content Area (zIndex: 10):
relative, padding px-6 lg:px-12 py-6 lg:py-8, flex-1 flex flex-col justify-between

Top Row -- 2 column grid (grid-cols-1 lg:grid-cols-2 gap-8 lg:gap-12):
Left column: Main heading (h1): "BUILDING / BRANDS THAT / RESONATE" in Anton font. text-5xl sm:text-6xl lg:text-6xl xl:text-7xl font-normal text-black leading-[0.80] tracking-tight mb-4 lg:mb-5.
Subheading: "Thoughtful design that captivates, empowers, and creates lasting impact." text-lg lg:text-xl mb-4 lg:mb-5 max-w-md color #080808
CTA button: "Start today" with ArrowRight icon in white circle. flex items-center gap-3 pl-8 pr-1.5 py-1.5 bg-black text-white rounded-full hover:bg-gray-800

Right column (text-right): "50+ BRANDS LAUNCHED" heading in Anton, with description paragraph below.

Middle Row -- 2 column grid:
Left: Brand designer bio paragraph with social icons (Facebook filled, Instagram, Youtube)
Right: "5+ YEARS IN THE INDUSTRY" heading in Anton with description

Bottom Row -- Brand Logo Bar:
6 brand cards in grid-cols-6: Frame Blox, Supa Blox, Hype Blox, Hype Blox, Ultra Blox, Ship Blox
Each with unique abstract icon and white bg rounded-lg card style

Key: No animations, all text black/#080808, default Tailwind config, justify-between layout distribution.
