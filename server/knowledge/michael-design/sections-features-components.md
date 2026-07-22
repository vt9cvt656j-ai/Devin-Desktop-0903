# Michael Design Library — sections-features-components

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 35 entries.

## Apex Program Accordion — Accordion [sections/apex-program-accordion]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/apex-program-accordion.webp

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

## Guardnet Benefits — Benefits [sections/guardnet-benefits]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/guardnet-benefits.webp

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

## Kova Features — Benefits [sections/kova-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/kova-features.webp

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

## Bento Grid Stats — Bento [sections/bento-grid-stats]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/bento-grid-stats.webp

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

## Cognitra Offer — Cards [sections/cognitra-offer]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/cognitra-offer.webp

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

## Nimbus Security — Cards [sections/nimbus-security]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nimbus-security.webp

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

## Nimbus Sticky Cards — Cards [sections/nimbus-sticky-cards]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nimbus-sticky-cards.webp

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

## Orbis Cards — Cards [sections/orbis-cards]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/orbis-cards.webp

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

## Veloce Cards — Cards [sections/veloce-cards]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/veloce-cards.webp

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

## FlowMate Carousal — Carousal [sections/flowmate-carousal]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/flowmate-carousal.webp

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

## Animated Cards — Component [sections/animated-cards]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/animated-cards.webp

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

## Cognitra Feature — Feature [sections/cognitra-feature]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/cognitra-feature.webp

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

## Capabilities Overview — Features [sections/features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/features.webp

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

## Interior Features — Features [sections/interior-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/interior-features.webp

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

## LaunchEx Submissions — Features [sections/launchex-submissions]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/launchex-submissions.webp

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

## Liquid Glass Features — Features [sections/liquid-glass-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/liquid-glass-features.webp

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

## Max Reed Portfolio — Features [sections/max-reed-portfolio]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/max-reed-portfolio.webp

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

## NexaCore Control — Features [sections/nexacore-control]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nexacore-control.webp

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

## NexaCore Results — Features [sections/nexacore-results]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nexacore-results.webp

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

## Nike Hover — Features [sections/nike-hover]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nike-hover.webp

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

## Benefits Features — Features Section [sections/benefits-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/benefits-features.gif

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

## Glow Features — Features Section [sections/glow-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/glow-features.png

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

## Keep Ahead Features — Features Section [sections/keep-ahead-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/keep-ahead-features.png

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

## Nexora Features — Features Section [sections/nexora-features]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nexora-features.gif

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

## Guardnet Demo — Info [sections/guardnet-demo]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/guardnet-demo.webp

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

## Scroll Marquee — Marquee [sections/scroll-marquee]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/scroll-marquee.webp

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

## NexaCore Process — Process [sections/nexacore-process]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nexacore-process.webp

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

## Feedback Slider — Slider [sections/feedback-slider]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/feedback-slider.webp

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

## Media Card Carousel — Slider [sections/media-card-carousel]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/media-card-carousel.webp

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

## Arceage Stats — Stats [sections/arceage-stats]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/arceage-stats.webp

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

## Glassmorphic Feature Tabs — Tabs [sections/glassmorphic-feature-tabs]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/glassmorphic-feature-tabs.webp

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

## Technical Specifications — Tabs [sections/technical-specifications]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/technical-specifications.webp

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

## Halo Use Case — Use Case [sections/halo-use-case]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/halo-use-case.webp

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

## Halo Benefits — Why Us [sections/halo-benefits]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/halo-benefits.webp

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

## Nexar — Hero Section [sections/nexar-hero]

- Asset: https://code.mrday.one/design-assets/sections/visuals-by-id/nexar-hero.gif

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
