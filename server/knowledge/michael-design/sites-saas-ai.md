# Michael Design Library — sites-saas-ai

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 32 entries.

## AI Image Generator UI — AI [sites/ai-image-generator-ui]

- Preview: https://res.cloudinary.com/dsdhxhhqh/image/upload/v1778221760/CleanShot_2026-05-08_at_13.28.55_2x_swwnfd.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-image-generator-ui.png

Build a "Core Features" marketing section as a single centered component with three gradient cards. Use the Inter font family (weights 400, 500, 600) loaded from Google Fonts: `https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap`.

**Page shell:**
- Body: white background `#ffffff`, 80px top/bottom + 20px left/right padding, flex centered, Inter font.
- Global reset: `* { box-sizing: border-box; margin: 0; padding: 0; }`.

**Container (`.c1-container`):** max-width 1100px, full width, text-align center.

**Header block:**
- Badge (`.c1-badge`): text "Core Features", 0.75rem, weight 600, uppercase, letter-spacing 1px, gradient text using `linear-gradient(90deg, #F5C344, #F28482, #B567C2)` with `-webkit-background-clip: text` and transparent fill. 16px bottom margin.
- Title (`.c1-title`): "Built for Speed & Quality", font-size 2.75rem, weight 500, color `#0f172a`, letter-spacing -0.02em, 12px bottom margin.
- Subtitle (`.c1-subtitle`): "Everything you need to go" + `<br>` + "from idea to image", 1.125rem, color `#64748b`, line-height 1.5, 50px bottom margin.

**Grid (`.c1-grid`):** 3 equal columns, 24px gap. Breakpoints: 2 columns under 900px, 1 column under 600px (title scales to 2.25rem).

**Card base (`.c1-card`):** 20px border-radius, height 340px, flex column justify-end, relative, overflow hidden, text-align left, background `#F4F8F9`, shadow `0 10px 30px -10px rgba(0,0,0,0.1)`. Titles inside (`h3`): 1.05rem, weight 600, color `#1e293b`, padding 24px, z-index 2.

**Card 1 — Smart Prompt Suggestions (`.c1-card-1`):**
- Background: `radial-gradient(circle at 50% 0%, #FFB347 0%, #F9ED96 30%, #F4F8F9 60%, #F4F8F9 100%)`.
- Prompt box (white, 12px radius, 16px padding, 0.8rem text, color `#475569`, line-height 1.6, shadow `0 8px 20px rgba(0,0,0,0.04)`), absolutely positioned top:30px/left:24px/right:24px. Text: "A bright, high-resolution 3D illustration of a **cheerful cartoon** of a **girl character** **centred against a** smooth blue background" — bold phrases have class `.c1-blur-text` with gradient `linear-gradient(90deg, #FFB347, #E5A1F5)` as clipped text, weight 600.
- "Add more details" pill button: absolute top:180px/left:40px, white background, 1px solid black border, 5px 14px padding, 20px radius, 0.75rem text, weight 600, color `#1e293b`, shadow `0 4px 15px rgba(0,0,0,0.08)`, includes `✦` character styled `color: #a855f7; font-size: 1rem` with 6px gap.
- Cursor SVG arrow: absolute top:205px/left:110px, 24x24, fill `#0f172a`, white stroke 1px, drop-shadow `0 4px 6px rgba(0,0,0,0.2)`, z-index 10. Path: `M4 2L20 11L11 13L9 22L4 2Z`.
- Heading: "Smart Prompt Suggestions".

**Card 2 — API Access (`.c1-card-2`):**
- Background: `radial-gradient(circle at 50% 0%, #E5A1F5 0%, #F8ACA0 30%, #F4F8F9 60%, #F4F8F9 100%)`.
- `.c1-api-visual` absolutely positioned top:0/left:0/right:0/bottom:70px, flex centered, 24px horizontal padding.
- Image (`.c1-network-img`): width 100%, height 180px, object-fit contain, margin-top 20px. Source: `https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/viktor/network.svg`.
- Heading: "API Access".

**Card 3 — Project Library (`.c1-card-3`):**
- Background: `radial-gradient(circle at 50% 0%, #F9ED96 0%, #E5A1F5 30%, #F4F8F9 60%, #F4F8F9 100%)`.
- Mesh overlay (`.c1-mesh`): absolute inset 0, background image = two linear gradients of `rgba(255,255,255,0.8) 1px, transparent 1px` (horizontal and 90deg vertical), background-size 16px 16px, masked with `radial-gradient(circle at center top, black 0%, transparent 80%)` (include `-webkit-mask-image`).
- Folder image (`.c1-folder`): absolute top:50px, horizontally centered via `left:50%; transform:translateX(-50%)`, width 170px, drop-shadow `0 15px 25px rgba(0,0,0,0.08)`. Source: `https://pub-f170a2592d2c4a1485466404c36807be.r2.dev/viktor/library%20icon.svg`.
- Search pill (`.c1-search`): absolute top:220px, centered, white background, 1px solid black, 6px 18px padding, 20px radius, 0.75rem text weight 500 color `#1e293b`, shadow `0 8px 20px rgba(0,0,0,0.06)`, white-space nowrap, 8px gap. Contains a 14x14 lucide-style search SVG (circle cx=11 cy=11 r=8, line 21,21→16.65,16.65, stroke `#64748b`, stroke-width 2, round caps/joins) followed by text "Search in library".
- Heading: "Project Library".

**Note:** No animations are defined in this component — it is purely static styling. No JavaScript behavior, no hover effects. Use Supabase if any data persistence is needed, though this section requires none.

## AI Automation Hero — AI / SaaS [sites/10]

- Preview: https://motionsites.ai/assets/hero-synapse-ai-preview-BjBuH68i.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/10.gif

Create a full-screen hero section with the following exact specifications:

Layout & Structure:
- Full viewport height (h-screen), full width, relative positioning with overflow-hidden
- Background color: #070612 (dark purple-black)
- Content aligned to the left side, vertically centered
- Max-width container (max-w-7xl) with horizontal padding (px-6 lg:px-12)

Background Video:
Video Source: HLS stream from https://stream.mux.com/s8pMcOvMQXc4GD6AX4e1o01xFogFxipmuKltNfSYza0200.m3u8
- Autoplaying, looping, muted video positioned absolutely behind content
- Video shifted 200px to the right (margin-left: 200px)
- Video scaled to 1.2x with origin-left, object-cover, full height
- Bottom fade gradient (h-40) from background color to transparent (z-10)

Badge (top element):
- Pill-shaped badge with rounded-full, border border-white/20, backdrop-blur-sm
- Contains a Sparkles icon (lucide-react, w-3 h-3, text-white/80)
- Text: "New AI Automation Ally" in text-sm font-medium text-white/80
- Animated with blur-in effect (0.6s duration, no delay)

Main Heading:
- Three lines of text:
  - Line 1: "Unlock the Power of AI" (block display)
  - Line 2: "for Your" (inline)
  - Line 3: "Business." in serif italic font (inline)
- Font sizes: text-4xl md:text-5xl lg:text-6xl
- Font weight: font-medium
- Line height: leading-tight lg:leading-[1.2]
- Color: white (text-foreground)
- Each word animates in with staggered split-text animation (0.08s delay between words, 0.6s duration, y: 40px -> 0, opacity: 0 -> 1)

Subtitle:
- Text: "Our cutting-edge AI platform automates, analyzes, and accelerates your workflows so you can focus on what really matters."
- Styling: text-white/80, text-lg, font-normal, leading-relaxed, max-w-xl
- Animated with blur-in effect (0.4s delay, 0.6s duration)

CTA Buttons (bottom):
- Two buttons side by side with gap-4, flex-wrap
- Primary button "Book A Free Call":
  - Solid white background (bg-foreground), dark text (text-background)
  - Rounded-full, px-5 py-3
  - Includes right arrow icon (ArrowRight from lucide-react)
  - Links to /book-call
- Secondary button "Learn now":
  - Semi-transparent background (bg-white/20), backdrop-blur-sm
  - Rounded-full, px-8 py-3
  - White text
- Both buttons animated with blur-in effect (0.6s delay, 0.6s duration)

Animations (using framer-motion):
- BlurIn component: opacity 0->1, blur 10px->0, y 20->0
- SplitText component: splits text by words, staggers each word's animation

Z-index layering:
- Video: z-0
- Bottom gradient: z-10
- Content: z-20

Spacing:
- 12-unit gap (gap-12) between badge/heading group and CTA buttons
- 6-unit gap (gap-6) between badge and heading, and between heading and subtitle

## AI Driving Assistant — AI SaaS Website [sites/ai-driving-assistant]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/carteslaArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-driving-assistant.mp4

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Auren — Your car. Its mind.</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter+Tight:wght@400;500;600&family=JetBrains+Mono:wght@400;500&family=Orbitron:wght@400;500;700&display=swap" rel="stylesheet" />
<script src="https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/ScrollTrigger.min.js"></script>
<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --font-sans: "Inter Tight", Arial, Helvetica, sans-serif;
  --font-orbitron: "Orbitron", sans-serif;
  --font-mono: "JetBrains Mono", monospace;

  --hero-heading-size: clamp(40px, 8.16vw, 120px);
  --hero-pad-left: clamp(20px, 2.72vw, 40px);
  --hero-pad-bottom: clamp(28px, 7.7vh, 64px);
  --hero-block-width: clamp(300px, 54.6vw, 803px);
  --hero-gap: clamp(8px, 1.1vw, 16px);

  --hero-media-w: clamp(140px, 13.13vw, 193px);
  --hero-media-aspect: 193 / 108;
  --hero-logo-w: clamp(30px, 2.72vw, 40px);
  --hero-logo-aspect: 40 / 14;
  --hero-logo-left: calc(50% - var(--hero-media-w) / 2 - var(--hero-logo-w));
  --hero-logo-top: calc(
    50% - (var(--hero-media-w) / (var(--hero-media-aspect))) / 2 -
      (var(--hero-logo-w) / (var(--hero-logo-aspect))) +
      var(--hero-center-shift, 0px)
  );
  --hero-title-offset-x: clamp(40px, 6.6vw, 97px);
  --hero-title-offset-y: clamp(28px, 6.5vh, 54px);
  --hero-title-size: clamp(30px, 5.44vw, 80px);
  --hero-title-width: clamp(220px, 34.4vw, 506px);
  --hero-desc-width: clamp(220px, 25.65vw, 377px);
  --hero-title-left: calc(50% + var(--hero-title-offset-x));
  --hero-title-top: calc(50% + var(--hero-title-offset-y));
  --hero-title-align: left;

  --header-pad-x: clamp(20px, 2.72vw, 40px);
  --header-pad-y: clamp(16px, 2.18vw, 32px);

  --story-word-size: clamp(48px, 9vw, 168px);
  --story-block-width: clamp(300px, 42vw, 560px);
}

html, body {
  margin: 0;
  padding: 0;
  background: #0c0d0f;
  color: #f0f1f3;
  font-family: var(--font-sans);
}
body { overflow-x: hidden; }

/* --- Header --- */
.header {
  position: fixed;
  inset: 0 0 auto 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--header-pad-y) var(--header-pad-x) 0;
}
.glass-pill {
  border-radius: 8px;
  background: rgba(240,241,243,0.15);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
}
.header-logo { display: flex; align-items: center; padding: 4px 16px; }
.header-right { display: flex; align-items: center; gap: 16px; }
.header-nav { display: flex; align-items: center; gap: 12px; padding: 9px 16px; }
.header-nav span {
  font-size: 14px; font-weight: 400; line-height: 1;
  letter-spacing: -0.02em; color: #F0F1F3; text-align: center;
}
.header-menu { display: flex; align-items: center; padding: 4px 16px; cursor: pointer; border: none; background: none; }
.header-login {
  display: flex; align-items: center; border-radius: 8px;
  background: #F0F1F3; padding: 9px 16px; border: none; cursor: pointer;
}
.header-login span { font-size: 14px; line-height: 1; letter-spacing: -0.02em; color: #0C0D0F; }

@media (max-width: 640px) {
  .header-nav { display: none; }
}

/* --- Hero --- */
.hero {
  position: relative; width: 100%; overflow: hidden;
  height: 100dvh;
  background: linear-gradient(180deg, #03070A 0%, #AAC2CE 100%);
}

.hero-corner-logo {
  position: absolute; z-index: 10; pointer-events: none;
  width: var(--hero-logo-w);
  aspect-ratio: var(--hero-logo-aspect);
  left: var(--hero-logo-left);
  top: var(--hero-logo-top);
  transform-origin: center;
  will-change: transform, opacity, filter;
}
.hero-corner-logo svg { width: 100%; height: 100%; }

.hero-textblock { display: contents; }

.hero-title {
  position: absolute; z-index: 10; pointer-events: none; margin: 0;
  font-family: var(--font-orbitron); font-weight: 400;
  text-transform: uppercase; line-height: 100%; letter-spacing: -0.04em;
  color: #F0F1F3; font-size: var(--hero-title-size);
  left: var(--hero-title-left); top: var(--hero-title-top);
  width: var(--hero-title-width);
  text-align: var(--hero-title-align);
  transform-origin: center;
  will-change: transform, opacity, filter;
}

.hero-description {
  position: absolute; z-index: 10; pointer-events: none;
  font-size: 14px; font-weight: 500; line-height: 1.3;
  letter-spacing: -0.02em; color: #F0F1F3;
  left: var(--hero-pad-left); top: var(--hero-logo-top);
  width: var(--hero-desc-width);
  will-change: opacity;
}

.hero-email {
  position: absolute; z-index: 10;
  display: flex; flex-direction: column; gap: 13px;
  left: var(--hero-pad-left); bottom: var(--header-pad-y);
  will-change: opacity;
}
.hero-email form { display: flex; align-items: center; }
.hero-email input {
  height: 32px; width: 185px; border-radius: 8px;
  background: rgba(240,241,243,0.15); backdrop-filter: blur(10px);
  padding: 9px 16px; font-size: 14px; line-height: 1;
  letter-spacing: -0.02em; color: #F0F1F3; border: none; outline: none;
}
.hero-email input::placeholder { color: rgba(240,241,243,0.7); }
.hero-email button[type="submit"] {
  height: 32px; display: flex; align-items: center;
  border-radius: 8px; background: #F0F1F3; padding: 9px 16px;
  font-size: 14px; line-height: 1; letter-spacing: -0.02em;
  color: #0C0D0F; border: none; cursor: pointer;
  transition: transform 0.15s ease-out;
}
.hero-email button[type="submit"]:active { transform: scale(0.97); }
.hero-email .caption {
  max-width: 628px; font-size: 14px; line-height: 1.3;
  letter-spacing: -0.02em; color: rgba(240,241,243,0.8);
}

.hero-video-slot {
  position: absolute; pointer-events: none; visibility: hidden;
  width: var(--hero-media-w); aspect-ratio: var(--hero-media-aspect);
  left: calc(50% - var(--hero-media-w) / 2);
  top: calc(50% - (var(--hero-media-w) / (var(--hero-media-aspect))) / 2 + var(--hero-center-shift, 0px));
}

.hero-media-wrapper {
  position: absolute; z-index: 20; pointer-events: none;
  overflow: hidden; background: rgba(0,0,0,0.4);
  top: 0; left: 0;
  width: var(--hero-media-w); aspect-ratio: var(--hero-media-aspect);
  border-radius: 12px; opacity: 0;
  will-change: top, left, width, height, border-radius;
}
.hero-media-wrapper video { width: 100%; height: 100%; object-fit: cover; }

.story-scrim {
  position: absolute; inset: 0; z-index: 25;
  pointer-events: none; opacity: 0;
  background: linear-gradient(180deg, rgba(12,13,15,0.72) 0%, rgba(12,13,15,0.15) 32%, rgba(12,13,15,0.15) 62%, rgba(12,13,15,0.78) 100%);
}

.story-beat {
  position: absolute; inset: 0; z-index: 30;
  pointer-events: none; display: flex;
  align-items: center; justify-content: center;
  perspective: 1000px;
}
.story-word {
  font-family: var(--font-orbitron); font-weight: 400;
  text-align: center; text-transform: uppercase;
  line-height: 0.9; letter-spacing: -0.04em;
  color: #F0F1F3; font-size: var(--story-word-size);
  opacity: 0; will-change: transform, opacity, filter;
}

.feature-caption {
  position: absolute; z-index: 30; pointer-events: none;
  display: flex; flex-direction: column; align-items: flex-start;
  gap: 19px; opacity: 0;
  left: var(--hero-pad-left); bottom: 40px;
  width: min(377px, calc(100% - 2 * var(--hero-pad-left)));
  will-change: transform, opacity;
}
.feature-caption .dot {
  width: 16px; height: 16px; border-radius: 50%; background: #fff;
}
.feature-caption p {
  width: 100%; font-size: 14px; font-weight: 500;
  line-height: 1.3; letter-spacing: -0.02em; color: #fff;
}

.waveform-container, .route-container {
  position: absolute; inset: 0; z-index: 30;
  pointer-events: none; display: flex;
  align-items: center; justify-content: center;
  opacity: 0; will-change: transform, opacity;
}
.waveform-container svg, .route-container svg {
  height: auto; width: var(--story-block-width);
}

/* --- Mobile --- */
@media (max-width: 640px) {
  :root {
    --hero-pad-left: 16px;
    --header-pad-x: 16px;
    --hero-title-align: center;
    --hero-center-shift: -80px;
    --hero-title-width: 100%;
    --hero-desc-width: 100%;
    --story-word-size: clamp(30px, 11vw, 60px);
    --story-block-width: min(560px, calc(100vw - 32px));
  }
  .hero-textblock {
    position: absolute;
    top: calc(50% + (var(--hero-media-w) / (var(--hero-media-aspect))) / 2 + 32px + var(--hero-center-shift, 0px));
    left: 16px; right: 16px; z-index: 10;
    display: flex; flex-direction: column; gap: 20px;
  }
  .hero-textblock > * { position: static; inset: auto; }
  .hero-email { right: 16px; }
  .hero-email input { width: auto; flex: 1; }
}
</style>
</head>
<body>

<!-- ===== HEADER ===== -->
<header class="header">
  <div class="glass-pill header-logo">
    <svg width="98" height="24" viewBox="0 0 98 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path fill-rule="evenodd" clip-rule="evenodd" d="M27 5C34.5425 5 38.3141 4.99987 40.6572 5.81997C43.0004 6.64007 43 7.96014 43 10.6V13.4C43 16.0399 43.0004 17.3599 40.6572 18.18C38.3141 19.0001 34.5425 19 27 19H19C11.4575 19 7.68592 19.0001 5.34277 18.18C2.99963 17.3599 3 16.0399 3 13.4V10.6C3 7.96014 2.99963 6.64007 5.34277 5.81997C7.68592 4.99987 11.4575 5 19 5H27ZM23 9.9C19.6863 9.9 17 10.8402 17 12C17 13.1598 19.6863 14.1 23 14.1C26.3137 14.1 29 13.1598 29 12C29 10.8402 26.3137 9.9 23 9.9Z" fill="#F0F1F3"/>
      <path d="M50 17.04V8.85C50 8.50467 50.084 8.18733 50.252 7.898C50.42 7.60867 50.6487 7.38 50.938 7.212C51.2273 7.044 51.5447 6.96 51.89 6.96H58.19C58.5353 6.96 58.848 7.044 59.128 7.212C59.4173 7.38 59.646 7.60867 59.814 7.898C59.9913 8.18733 60.08 8.50467 60.08 8.85V17.04H58.386V13.582H51.68V17.04H50ZM51.68 11.902H58.386V8.906C58.386 8.83133 58.358 8.77067 58.302 8.724C58.2553 8.67733 58.1993 8.654 58.134 8.654H51.932C51.8667 8.654 51.806 8.67733 51.75 8.724C51.7033 8.77067 51.68 8.83133 51.68 8.906V11.902Z" fill="#F0F1F3"/>
      <path d="M62.9491 17.04C62.6038 17.04 62.2865 16.956 61.9971 16.788C61.7171 16.6107 61.4931 16.382 61.3251 16.102C61.1571 15.822 61.0731 15.5093 61.0731 15.164V8.92H62.7391V15.108C62.7391 15.1827 62.7625 15.248 62.8091 15.304C62.8651 15.3507 62.9305 15.374 63.0051 15.374H67.3871C67.4618 15.374 67.5225 15.3507 67.5691 15.304C67.6251 15.248 67.6531 15.1827 67.6531 15.108V8.92H69.3191V15.164C69.3191 15.5093 69.2351 15.822 69.0671 16.102C68.8991 16.382 68.6751 16.6107 68.3951 16.788C68.1151 16.956 67.7978 17.04 67.4431 17.04H62.9491Z" fill="#F0F1F3"/>
      <path d="M70.2335 17.04V10.796C70.2335 10.4507 70.3175 10.138 70.4855 9.858C70.6628 9.578 70.8915 9.354 71.1715 9.186C71.4608 9.00867 71.7735 8.92 72.1095 8.92H76.6315V10.586H72.1655C72.0908 10.586 72.0255 10.614 71.9695 10.67C71.9228 10.7167 71.8995 10.7773 71.8995 10.852V17.04H70.2335Z" fill="#F0F1F3"/>
      <path d="M78.8636 17.04C78.5183 17.04 78.2056 16.956 77.9256 16.788C77.6456 16.6107 77.417 16.382 77.2396 16.102C77.0716 15.822 76.9876 15.5093 76.9876 15.164V10.796C76.9876 10.4507 77.0716 10.138 77.2396 9.858C77.417 9.578 77.6456 9.354 77.9256 9.186C78.2056 9.00867 78.5183 8.92 78.8636 8.92H83.3576C83.7123 8.92 84.0296 9.00867 84.3096 9.186C84.5896 9.354 84.8136 9.578 84.9816 9.858C85.1496 10.138 85.2336 10.4507 85.2336 10.796V13.82H78.6536V15.108C78.6536 15.1827 78.677 15.248 78.7236 15.304C78.7796 15.3507 78.845 15.374 78.9196 15.374H85.2336V17.04H78.8636ZM78.6536 12.294H83.5676V10.852C83.5676 10.7773 83.5396 10.7167 83.4836 10.67C83.437 10.614 83.3763 10.586 83.3016 10.586H78.9196C78.845 10.586 78.7796 10.614 78.7236 10.67C78.677 10.7167 78.6536 10.7773 78.6536 10.852V12.294Z" fill="#F0F1F3"/>
      <path d="M86.163 17.04V8.92H92.547C92.883 8.92 93.191 9.00867 93.471 9.186C93.7603 9.354 93.989 9.578 94.157 9.858C94.325 10.138 94.409 10.4507 94.409 10.796V17.04H92.743V10.852C92.743 10.7773 92.715 10.7167 92.659 10.67C92.6123 10.614 92.5563 10.586 92.491 10.586H88.095C88.0296 10.586 87.969 10.614 87.913 10.67C87.857 10.7167 87.829 10.7773 87.829 10.852V17.04H86.163Z" fill="#F0F1F3"/>
    </svg>
  </div>
  <div class="header-right">
    <div style="display:flex;align-items:center">
      <div class="glass-pill header-nav">
        <span>Service</span><span>About</span><span>Contact</span>
      </div>
      <button class="glass-pill header-menu" aria-label="Open menu">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M7.5 9H21.75V10.5H7.5V9Z" fill="#F0F1F3"/>
          <path d="M2.25 13.5H21.75V15H2.25V13.5Z" fill="#F0F1F3"/>
        </svg>
      </button>
    </div>
    <button class="header-login"><span>Log In</span></button>
  </div>
</header>

<!-- ===== HERO ===== -->
<section class="hero" id="hero">
  <!-- Corner Logo -->
  <div class="hero-corner-logo" id="cornerLogo" aria-hidden="true">
    <svg viewBox="0 0 40 14" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path fill-rule="evenodd" clip-rule="evenodd" d="M24 0C31.5425 0 35.3141 -0.000130236 37.6572 0.819971C40.0004 1.64007 40 2.96014 40 5.6V8.4C40 11.0399 40.0004 12.3599 37.6572 13.18C35.3141 14.0001 31.5425 14 24 14H16C8.45753 14 4.68592 14.0001 2.34277 13.18C-0.000371695 12.3599 0 11.0399 0 8.4V5.6C0 2.96014 -0.000371933 1.64007 2.34277 0.819971C4.68592 -0.000130117 8.45753 0 16 0H24ZM20 4.9C16.6863 4.9 14 5.8402 14 7C14 8.1598 16.6863 9.1 20 9.1C23.3137 9.1 26 8.1598 26 7C26 5.8402 23.3137 4.9 20 4.9Z" fill="#F0F1F3"/>
    </svg>
  </div>

  <!-- Text Block (title + description) -->
  <div class="hero-textblock">
    <h1 class="hero-title" id="heroTitle">Hands<br>Off.</h1>
    <p class="hero-description" id="heroDesc">A private AI driver that learns the way you move — your routes, your routine, your time of day. It settles into the car you already drive and quietly takes the wheel, so less of every trip depends on you.</p>
  </div>

  <!-- Email Capture -->
  <div class="hero-email" id="heroEmail">
    <form onsubmit="event.preventDefault()">
      <input type="email" placeholder="Email" aria-label="Email" />
      <button type="submit">Send</button>
    </form>
    <p class="caption">Join the waitlist and be first to let your car do the driving. No spam, just the road ahead.</p>
  </div>

  <!-- Feature Captions -->
  <div class="feature-caption" id="caption1">
    <span class="dot"></span>
    <p>Just speak. Ask for a warmer cabin, a quieter route, a stop for coffee — the car listens and takes care of the rest while you ride.</p>
  </div>
  <div class="feature-caption" id="caption2">
    <span class="dot"></span>
    <p>Drop a few points on the map and it strings them together — every turn, every stop, driven in order while you sit back and watch.</p>
  </div>

  <!-- Video Slot (invisible measurement) -->
  <div class="hero-video-slot" id="videoSlot" aria-hidden="true"></div>

  <!-- Video -->
  <div class="hero-media-wrapper" id="mediaWrapper">
    <video id="heroVideo" muted playsinline preload="auto" crossorigin="anonymous"
      src="https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260709_102332_2d8c4e02-313c-4362-aaa7-4c907cfc4f79.mp4">
    </video>
  </div>

  <!-- Scroll Story -->
  <div class="story-scrim" data-story="scrim" aria-hidden="true"></div>
  <div class="story-beat"><span class="story-word" data-story="beat1-word">Effortless.</span></div>
  <div class="story-beat"><span class="story-word" data-story="beat2-word">Anywhere.</span></div>

  <!-- Voice Waveform -->
  <div class="waveform-container" id="waveform" aria-hidden="true"></div>

  <!-- Route Checkpoints -->
  <div class="route-container" id="routeViz" aria-hidden="true">
    <svg viewBox="0 0 620 240" fill="none">
      <path data-route-path d="M 40 170 C 160 170 210 84 310 84 C 410 84 460 170 580 170" stroke="#F0F1F3" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
      <g data-checkpoint style="opacity:0">
        <circle cx="40" cy="170" r="26" fill="#F0F1F3"/>
        <path d="M 30 171 L 37 178 L 51 163" stroke="#0C0D0F" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
      </g>
      <g data-checkpoint style="opacity:0">
        <circle cx="310" cy="84" r="26" fill="#F0F1F3"/>
        <path d="M 300 85 L 307 92 L 321 77" stroke="#0C0D0F" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
      </g>
      <g data-checkpoint style="opacity:0">
        <circle cx="580" cy="170" r="26" fill="#F0F1F3"/>
        <path d="M 570 171 L 577 178 L 591 163" stroke="#0C0D0F" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
      </g>
    </svg>
  </div>
</section>

<script>
(function() {
  // Generate waveform SVG
  var BAR_COUNT = 27, VIEW_W = 600, VIEW_H = 200, BAR_W = 8;
  var GAP = (VIEW_W - BAR_COUNT * BAR_W) / (BAR_COUNT - 1);
  var MAX_BAR_H = VIEW_H * 0.9;
  var ns = "http://www.w3.org/2000/svg";
  var svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 " + VIEW_W + " " + VIEW_H);
  svg.setAttribute("fill", "none");
  for (var i = 0; i < BAR_COUNT; i++) {
    var t = i / (BAR_COUNT - 1);
    var bell = Math.sin(t * Math.PI);
    var h = (0.18 + 0.82 * bell) * MAX_BAR_H;
    var x = i * (BAR_W + GAP);
    var y = (VIEW_H - h) / 2;
    var rect = document.createElementNS(ns, "rect");
    rect.setAttribute("data-bar", "");
    rect.setAttribute("x", x);
    rect.setAttribute("y", y);
    rect.setAttribute("width", BAR_W);
    rect.setAttribute("height", h);
    rect.setAttribute("rx", BAR_W / 2);
    rect.setAttribute("fill", "#F0F1F3");
    rect.style.transformBox = "fill-box";
    rect.style.transformOrigin = "center";
    rect.style.transform = "scaleY(0)";
    svg.appendChild(rect);
  }
  document.getElementById("waveform").appendChild(svg);

  // Refs
  var section = document.getElementById("hero");
  var media = document.getElementById("mediaWrapper");
  var title = document.getElementById("heroTitle");
  var cornerLogo = document.getElementById("cornerLogo");
  var description = document.getElementById("heroDesc");
  var emailBlock = document.getElementById("heroEmail");
  var caption1 = document.getElementById("caption1");
  var caption2 = document.getElementById("caption2");
  var waveform = document.getElementById("waveform");
  var route = document.getElementById("routeViz");
  var video = document.getElementById("heroVideo");
  var slot = document.getElementById("videoSlot");

  // Show first frame
  function showFirstFrame() {
    try { video.pause(); if (video.currentTime < 0.001) video.currentTime = 0.001; } catch(e) {}
  }
  video.addEventListener("loadedmetadata", showFirstFrame);
  if (video.readyState >= 1) showFirstFrame();

  // Constants
  var EXPAND_VH = 160, CONTENT_VH = 1040, MAX_BLUR = 24;
  var FULLSCREEN_INSET = 0, FULLSCREEN_RADIUS = 16;

  var expandFraction = EXPAND_VH / (EXPAND_VH + CONTENT_VH);
  var B = 1 - expandFraction;
  function at(local) { return expandFraction + local * B; }
  function dur(span) { return span * B; }

  function startRect() {
    var s = section.getBoundingClientRect();
    var r = slot.getBoundingClientRect();
    return { top: r.top - s.top, left: r.left - s.left, width: r.width, height: r.height };
  }
  function endRect() {
    var s = section.getBoundingClientRect();
    var i = FULLSCREEN_INSET;
    return { top: i, left: i, width: s.width - i * 2, height: s.height - i * 2 };
  }

  gsap.registerPlugin(ScrollTrigger);

  var mm = gsap.matchMedia(section);
  mm.add(
    { motionOK: "(prefers-reduced-motion: no-preference)", reduced: "(prefers-reduced-motion: reduce)" },
    function(context) {
      var reduced = !!context.conditions.reduced;
      var blurProxy = { v: 0 };

      var scrim = section.querySelector('[data-story="scrim"]');
      var word1El = section.querySelector('[data-story="beat1-word"]');
      var word2El = section.querySelector('[data-story="beat2-word"]');
      var bars = Array.from(section.querySelectorAll("[data-bar]"));
      var routePath = section.querySelector("[data-route-path]");
      var checkpoints = Array.from(section.querySelectorAll("[data-checkpoint]"));

      var tl = gsap.timeline({
        scrollTrigger: {
          trigger: section,
          start: "top top",
          end: "+=" + (EXPAND_VH + CONTENT_VH) + "%",
          scrub: 0.6,
          pin: true,
          anticipatePin: 1,
          invalidateOnRefresh: true,
          onLeave: function() {
            gsap.set([media, title, cornerLogo, description, emailBlock, word1El, word2El, caption1, caption2, waveform, route], { willChange: "auto" });
          },
          onLeaveBack: function() {
            gsap.set([media, title, cornerLogo, description, emailBlock, word1El, word2El, caption1, caption2, waveform, route], { willChange: "auto" });
          }
        }
      });

      // Phase A: expand
      tl.fromTo(media,
        { top: function(){return startRect().top}, left: function(){return startRect().left}, width: function(){return startRect().width}, height: function(){return startRect().height}, borderRadius: 12, opacity: 1 },
        { top: function(){return endRect().top}, left: function(){return endRect().left}, width: function(){return endRect().width}, height: function(){return endRect().height}, borderRadius: FULLSCREEN_RADIUS, opacity: 1, ease: "none", duration: expandFraction, immediateRender: true },
        0
      );

      gsap.set([title, cornerLogo], { transformOrigin: "center" });
      tl.to([title, cornerLogo], { opacity: 0, scale: reduced ? 1 : 0.82, ease: "none", duration: expandFraction }, 0);
      tl.to(blurProxy, {
        v: reduced ? 0 : MAX_BLUR, ease: "none", duration: expandFraction,
        onUpdate: function() {
          var b = "blur(" + blurProxy.v + "px)";
          title.style.filter = b;
          cornerLogo.style.filter = b;
        }
      }, 0);
      tl.to([description, emailBlock], { opacity: 0, ease: "none", duration: expandFraction }, 0);

      // Phase B: video scrub
      tl.to({}, {
        duration: B, ease: "none",
        onUpdate: function() {
          var p = this.progress();
          var d = video.duration;
          if (!d || isNaN(d)) return;
          if (video.readyState >= 2) {
            if (!video.paused) video.pause();
            var target = p * d;
            if (Math.abs(video.currentTime - target) > 0.03) video.currentTime = target;
          } else if (video.paused) {
            video.play().catch(function(){});
          }
        }
      }, expandFraction);

      // Scrim
      tl.to(scrim, { opacity: 1, ease: "power1.out", duration: dur(0.06) }, at(0));

      // Flyby helper
      function addFlyby(el, enterStart, enterDur, exitDur) {
        el.style.filter = "none";
        gsap.set(el, { opacity: 0, scale: reduced ? 1 : 0.9, z: reduced ? 0 : -120 });
        tl.to(el, { opacity: 1, scale: 1, z: 0, ease: "power1.out", duration: dur(enterDur) }, at(enterStart));
        tl.to(el, { opacity: 0, z: reduced ? 0 : 200, ease: "power1.in", duration: dur(exitDur) }, at(enterStart + enterDur));
      }

      // Caption helpers
      function captionIn(el, start, span) {
        gsap.set(el, { opacity: 0, y: reduced ? 0 : 20 });
        tl.to(el, { opacity: 1, y: 0, ease: "power1.out", duration: dur(span) }, at(start));
      }
      function captionOut(el, start, span) {
        tl.to(el, { opacity: 0, y: reduced ? 0 : 20, ease: "power1.in", duration: dur(span) }, at(start));
      }

      // Beat 1 - Voice
      addFlyby(word1El, 0.02, 0.1, 0.1);

      var waveProxy = { p: 0 };
      var EDGE = 0.22;
      function envelope(p) { return p < EDGE ? p / EDGE : p > 1 - EDGE ? (1 - p) / EDGE : 1; }
      gsap.set(waveform, { opacity: 0 });
      bars.forEach(function(b) { b.style.transform = "scaleY(0)"; });
      tl.to(waveProxy, {
        p: 1, ease: "none", duration: dur(0.28),
        onUpdate: function() {
          var e = envelope(waveProxy.p);
          waveform.style.opacity = String(Math.min(1, e * 1.6));
          for (var i = 0; i < bars.length; i++) {
            var dance = reduced ? 1 : 0.5 + 0.5 * Math.abs(Math.sin(waveProxy.p * Math.PI * 3 + i * 0.5));
            bars[i].style.transform = "scaleY(" + (e * dance).toFixed(4) + ")";
          }
        }
      }, at(0.16));

      captionIn(caption1, 0.2, 0.08);
      captionOut(caption1, 0.38, 0.08);

      // Beat 2 - Route
      addFlyby(word2El, 0.52, 0.1, 0.1);

      gsap.set(route, { opacity: 0 });
      tl.to(route, { opacity: 1, ease: "power1.out", duration: dur(0.08) }, at(0.68));

      if (routePath) {
        var len = routePath.getTotalLength();
        gsap.set(routePath, { strokeDasharray: len, strokeDashoffset: reduced ? 0 : len });
        if (!reduced) {
          tl.to(routePath, { strokeDashoffset: 0, ease: "none", duration: dur(0.28) }, at(0.7));
        }
      }

      gsap.set(checkpoints, { opacity: 0, scale: reduced ? 1 : 0, transformOrigin: "50% 50%" });
      tl.to(checkpoints, { opacity: 1, scale: 1, ease: reduced ? "none" : "back.out(1.7)", duration: dur(0.05), stagger: dur(0.1) }, at(0.75));

      captionIn(caption2, 0.72, 0.08);

      return function() {};
    }
  );

  // Refresh after fonts load
  document.fonts && document.fonts.ready.then(function() { ScrollTrigger.refresh(); });
})();
</script>
</body>
</html>

## ADHD Planner — App [sites/adhd-planner]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/the%20frisArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/adhd-planner.mp4

Build a single-page landing site for "Drift" -- a calm, ADHD-friendly planner app. Use React + Vite + TypeScript + Tailwind CSS + lucide-react for icons. No other UI libraries.

### Fonts

Import via Google Fonts in `index.css`:
- **Inter** (weights 400, 500, 600) -- used as base body font
- **Instrument Serif** (italic only) -- used for the italic word "the stress" in the hero heading

```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@1&family=Inter:wght@400;500;600&display=swap');
```

Body: `font-family: 'Inter', sans-serif;` with antialiased rendering. `overflow-x: clip` on body.

### Color Palette

- Hero overlay: `bg-black/20`
- About section background: `#F6E4CF`
- Dark text / icons in About: `#321C04`
- Light cream (button backgrounds in About): `#FFF9F2`
- Muted accent (divider, secondary button bg): `#D9C4AA`
- Secondary button hover: `#CEBA9E`
- Dark button hover: `#1F1003`

### Tailwind Config

Add a custom keyframe `fade-in-down` (0%: opacity 0, translateY -8px; 100%: opacity 1, translateY 0) with 0.2s ease-out animation.

---

### SECTION 1: HERO (full viewport height)

- Full-screen section (`h-screen`, `overflow-hidden`, `mb-[-25px]` negative bottom margin so the next section overlaps it slightly)
- **Background video** (autoPlay, muted, loop, playsInline, object-cover, absolute inset-0):
  ```
  https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260711_090308_1dd0cea7-f9ba-4db4-8147-c7d746061c9e.mp4
  ```
- **Semi-transparent overlay**: `absolute inset-0 bg-black/20`

### Navbar (centered floating pill)
- Positioned `absolute top-6 left-1/2 -translate-x-1/2 z-50`
- White rounded-full pill with shadow-lg containing:
  - Brand text "Drift." (text-lg, font-bold, tracking-tight, text-black)
  - Animated hamburger icon (two lines that animate to an X on click using rotate-45/-rotate-45 with cubic-bezier(0.77,0,0.175,1) easing, 300ms)
- Dropdown (below the pill): white rounded-2xl container with links "Features", "Drift AI", "FAQ". Animated with opacity/scale/translate transitions. Hidden by default, pointer-events-none when closed.

### Hero Content (bottom-aligned)
- Flex column, `justify-end`, padding bottom 12 (md:16)
- **Heading** (centered):
  - Line 1: "Own your time" -- text-5xl / sm:text-7xl / md:text-8xl / lg:text-[96px], font-normal, text-white, leading-[1.1], tracking-tight
  - Line 2: "without *the stress*" -- same sizing. The words "the stress" are rendered in Instrument Serif italic via inline style `fontFamily: "'Instrument Serif', serif", fontStyle: 'italic'` inside an `<em>` tag with className `not-italic`
- **Subtitle**: "Drift is a calm, ADHD-friendly planner that turns scattered ideas into a clear path" -- text-white/80, text-sm md:text-base, font-medium, max-w-[420px], centered
- **CTA Bar** (centered below heading with gap):
  - Container: `bg-black/25 backdrop-blur-md rounded-xl`, flex row, items-center, pl-6 pr-1 py-1
  - Desktop text: "No noise. No complicated systems. Just your day, gently sorted." (text-white, text-sm, font-medium, hidden on mobile)
  - Mobile text (sm:hidden): "No noise. Just your day, gently sorted."
  - Button: "Start for free" -- bg-white, text-black, text-sm, font-medium, px-5, py-2.5, rounded-xl, hover:bg-white/90

---

### SECTION 2: ABOUT SECTION

- Background: `bg-[#F6E4CF]`
- **Rounded top corners**: `rounded-t-[25px]` with `relative z-10` (overlaps hero by 25px)
- Padding: py-20 md:py-32, px-6

### Top Area (centered, max-w-3xl)
- Paragraph: "We craft tools that move with your rhythm, not over it. Designed for ease, presence, and flow." -- text-[#321C04], text-base md:text-lg, text-center, leading-relaxed, max-w-lg
- Two buttons (flex-wrap, centered, gap-4):
  1. **"Say hello"** -- dark pill button (`bg-[#321C04]`, `text-[#FFF9F2]`, rounded-full). Has a white circle on the left containing a Mail icon (lucide-react, size 16). Text is uppercase, tracking-wide, font-medium.
  2. **"Stay informed"** -- muted pill button (`bg-[#D9C4AA]`, `text-[#321C04]`, rounded-full). Has a white circle on the left containing a Plus icon. Same text styling.

### Decorative Divider
- Full-width flex row with: small circle (w-2 h-2 rounded-full bg-[#D9C4AA]) + 2px gap + horizontal line (flex-1 h-[2px] bg-[#D9C4AA]) + 2px gap + small circle

### Bottom Area (max-w-6xl, flex-col md:flex-row)
- Left: Custom SVG logo (40x40, viewBox 0 0 256 256, abstract geometric shape with rounded quadrants, fill #321C04) + label "Calm / Amplified" (text-xs, uppercase, tracking-widest, font-semibold, line break between words)
- Right: Large paragraph: "We make AI tools and assistants. But, most importantly, we help you remember what gentle productivity looks like when software moves with you, not over you. We create systems that carry the cognitive weight, so you can attend to what truly counts." -- text-2xl / sm:text-3xl / md:text-4xl / lg:text-[42px], leading-[1.3], font-normal, text-[#321C04]

---

### SECTION 3: FEATURES SECTION (scroll-driven cards)

- **Fixed background image** (behind content, -z-10):
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260709_082449_46df5cc4-ad98-4541-9236-a2659c1478a4.png&w=1920&q=85
  ```
- Padding: px-5 md:px-10 lg:px-16, py-20 md:py-40 lg:py-48

### Layout: CSS Grid on lg+ (400px / xl:460px left column, 1fr right column, gap-24 / xl:gap-48)

### Left Column (sticky on desktop)
- `lg:sticky lg:top-0 lg:h-screen lg:flex lg:flex-col lg:justify-between lg:py-32`
- Heading: "Software that flows with your mind, not over it" -- text-white, text-2xl / sm:text-3xl / lg:text-[46px], leading-[1.2], font-normal
- Feature nav buttons (hidden below lg): list of feature titles as buttons. Active state: `bg-black/20 text-white`. Inactive: `bg-black/20 text-white/40`. Clicking scrolls to card (smooth, block: center).
- Bottom CTA (hidden below lg): "No noise. No complicated systems. Just your day, gently sorted." + "Start for free" button (same style as hero)

### Right Column (scrolling cards)
- 3 feature cards with IntersectionObserver:
  - **Active detection** (threshold 0.6): highlights corresponding nav button
  - **Reveal animation** (threshold 0.15): cards slide in from right (translate-x-16 to translate-x-0, opacity 0 to 1, duration-700, ease-out). Once revealed, stays visible.

Each card (`bg-black/20 backdrop-blur-sm rounded-3xl p-6 md:p-10`):
- Same SVG logo (40x40, fill rgba(255,255,255,0.8))
- Title (text-white, text-xl md:text-2xl, font-medium)
- Video (aspect-video, rounded-2xl, overflow-hidden, bg-black/30, autoPlay/muted/loop/playsInline)
- Description (text-white/60, font-medium, text-sm md:text-base, leading-relaxed)

**Feature data:**

1. Title: "Built for ease, not urgency"
   Description: "Drift strips away the noise that makes organizing feel draining. Every surface is made to be soft, quiet, and intuitive so you can move forward, not get stuck decoding."
   Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260702_102608_5fa1187d-9ac6-44fb-82ab-54376200abc0.mp4`

2. Title: "The gentlest way to start"
   Description: "Beginning your day should feel natural, not daunting. Drift eases you into motion with subtle cues and a quiet view of what deserves your energy right now."
   Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260625_174131_395bc785-bb21-4e65-abf6-27c56f0764b6.mp4`

3. Title: "Deep, undivided focus"
   Description: "No interruptions, no clutter. Drift holds you in the present task with a stripped-back layout that softens all else until you are truly ready to shift."
   Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260525_052706_d2e390fd-1846-4fe7-a4d8-8d2f1c875358.mp4`

---

### SVG Logo (used in About + Feature cards)

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 256 256" fill="none">
  <path d="M 256 256 L 178 256 C 150.386 256 128 233.614 128 206 L 128 256 L 0 256 L 0 192 C 0 156.654 28.654 128 64 128 C 99.346 128 128 156.654 128 192 L 128 128 L 256 128 Z M 78 0 C 105.614 0 128 22.386 128 50 L 128 0 L 256 0 L 256 64 C 256 99.346 227.346 128 192 128 C 156.654 128 128 99.346 128 64 L 128 128 L 0 128 L 0 0 Z" fill="#321C04" />
</svg>
```

---

### File Structure

```
src/
  App.tsx          -- Hero section + renders AboutSection + FeaturesSection
  main.tsx         -- ReactDOM render
  index.css        -- Font imports + Tailwind directives + body styles
  components/
    Navbar.tsx     -- Floating pill navbar with animated hamburger
    AboutSection.tsx -- Cream-colored about section
    FeaturesSection.tsx -- Dark features with sticky left + scrolling cards
```

## Targo Logistics Hero — SaaS [sites/12]

- Preview: https://motionsites.ai/assets/hero-targo-preview-BF9qQyMr.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/12.gif

Design Prompt: Targo Hero Section

Brand Identity: Create a high-end, dark-themed hero section for a logistics brand called "targo". Use a color palette of deep black (#000000), a vibrant brand red (#EE3F2C), and crisp white for primary text. The typography should use the Rubik font family, with headlines in bold, uppercase, and slightly tight letter-spacing (approx. -4%).

Layout & Positioning:

Header: A clean top navigation bar with a white SVG logo (abstract symbol + "targo" wordmark) on the left. Include "Home", "About", and "Contact Us" links, plus a small red "Contact Us" button with clipped corners on the right.

Main Hero: The headline "Swift and Simple Transport" and a "Get Started" button should be left-aligned and positioned in the upper-third of the section (aligned toward the top rather than centered).

Bottom Widget: A "Book a Free Consultation" card positioned at the bottom-left.

Key Design Elements:

Video Background: An auto-looping, muted background video using URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260227_042027_c4b2f2ea-1c7c-4d6e-9e3d-81a78063703f.mp4. Ensure it has 100% opacity with no dark overlay.

Clipped-Corner Buttons: All primary buttons must feature a custom geometric shape using CSS clip-path (a 10-12px diagonal cut on the top-right and bottom-left corners). Use the brand red for "Get Started" and solid white for "Book a Call".

Liquid Glass Effect: The consultation card must use advanced glassmorphism: backdrop-filter: blur(40px) saturate(180%), a 1px white border with 12% opacity, a subtle diagonal white-to-transparent shine gradient across the surface, and an inner box-shadow for depth.

Scaled Proportions: The layout should feel refined and compact. Headlines should be roughly 64px on desktop, and the overall spacing should avoid excessive padding to maintain a "scaled-down" professional look.

Technical Details:

Frameworks: React & Tailwind CSS.

Icons: Use the Phone icon from lucide-react inside the consultation button.

Responsiveness: Ensure the headline scales down to ~42px on mobile and the padding adjusts from 64px (desktop) to 32px (mobile).

## HR SaaS Hero — SaaS [sites/16]

- Preview: https://motionsites.ai/assets/hero-hr-saas-preview-Cf365Y1O.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/16.gif

Create a minimalist, high-end React hero section using Tailwind CSS v4 and the Motion library.

Layout & Spacing:

The section should have a min-h-screen height and be centered.

Apply a heavy top padding of exactly 290px to the main content container to create an editorial, spacious feel.

The content container should have a max-w-[1200px] and a vertical gap of 32px between elements.

Background:

Use this background video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260302_085640_276ea93b-d7da-4418-a09b-2aa5b490e838.mp4.

Critical: The video must be vertically flipped using scaleY(-1) and set to object-cover.

Apply a white gradient overlay on top of the video: from-[26.416%] from-[rgba(255,255,255,0)] to-[66.943%] to-white to seamlessly blend the video into the background.

Typography (Geist & Instrument Serif):

Main Heading: Use the 'Geist' font, medium weight, tracking -0.04em.

Text Content: 'Simple [management] for your remote team'.

Sizes: The main heading should be 80px (desktop), while the word 'management' should be in 'Instrument Serif' italic at 100px.

Description: Geist font, 18px, 80% opacity, slate color (#373a46), max-width 554px.

Interactive Components:

Email Navbar: Create a rounded (40px) input container with bg-[#fcfcfc], a thin border, and a soft shadow (0px 10px 40px 5px rgba(194,194,194,0.25)).

CTA Button: A dark, multi-layered gradient button ('Create Free Account') with a complex inner shadow for a high-gloss tactile effect: shadow-[inset_-4px_-6px_25px_0px_rgba(201,201,201,0.08),inset_4px_4px_10px_0px_rgba(29,29,29,0.24)].

Social Proof: Below the input, add a '1,020+ Reviews' badge with a row of star/brand icons.

Animations:

Use Motion to staggered 'fade and slide up' the heading, description, and the email input block for a smooth entrance.

Key Technical Specs for Implementation:

Video Class: className="w-full h-full object-cover [transform:scaleY(-1)]"

Gradient Class: className="absolute inset-0 bg-gradient-to-b from-[26.416%] from-[rgba(255,255,255,0)] to-[66.943%] to-white"

Button Shadow: shadow-[inset_-4px_-6px_25px_0px_rgba(201,201,201,0.08),inset_4px_4px_10px_0px_rgba(29,29,29,0.24)]

## Taskora SaaS Hero — SaaS [sites/2]

- Preview: https://motionsites.ai/assets/hero-taskora-preview-BlRBv8IU.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/2.gif

Build a high-fidelity, responsive, dark-themed hero section for a SaaS product called "Taskora" using React, Tailwind CSS, and Framer Motion (for entrance animations).

1. Visual Style & Assets
Theme: Dark mode base (#050505) with white text.
Background Video: Use this video URL as a full-screen background loop. Set it to opacity-50 and add a gradient overlay (black/60 to #050505) so it fades seamlessly into the background at the bottom: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260201_052917_7fc4e418-3123-40bf-b5ba-394c28eb4b3a.mp4
Typography: Import and use these specific Google Fonts:
Instrument Serif (Italic) → Strictly for the word "Workflow" in the headline.
Manrope → For the "Trusted by" badge and subheadlines.
Cabin → For the main CTA button.
Inter / Inter Tight → For the Dashboard UI and Navbar links.

2. Component Layout
A. Floating Navbar
Create a fixed, floating "pill-shaped" navbar with a glassmorphism effect (bg-white/10 backdrop-blur-md).
Desktop: Logo on left, Links centered (Home, Features, Company, Contact), Auth buttons (Sign Up, White "Sign In" button) on right.
Mobile: Collapse links into a hamburger menu that opens a glassmorphism dropdown.

B. Hero Content (Centered)
Badge: A pill-shaped badge reading "Trusted by +30.000 of clients globally". Include a star icon with a blue gradient fill.
Headline: Massive scale (up to text-[80px] on desktop). Text: "Simplify Your Workflow. Stay Focused." (Italicize "Workflow" using the Serif font).
Subhead: Gray text (text-gray-400): "Taskora helps teams manage projects, tasks, and deadlines with clarity."
CTA: A large white button with black text: "Book a Free Demo". Add a subtle hover scale and shadow effect.

C. Dashboard Preview (The "Product Shot")
Build a detailed, non-functional mock dashboard interface container placed below the CTA.
Visuals: Light mode dashboard (bg-[#F9F9FA]) to contrast with the dark hero background.
Sidebar: Thin vertical rail with navigation icons (Home, Users, etc.).
Content Area:
Stats Cards: 3 cards (Total Sales, Operating Expenses, Gross Profit) showing a value, a percentage trend (green/red), and a mini bar chart at the bottom.
Revenue Chart: A section showing a bar chart visualization.
Deals Table: A detailed data table showing rows with "Deal Name", "Company" (Amazon.com with logo), "Amount", "Date", "Owner" (avatar), and "Stage" (New tag).
Header: Search bar, Notification bell, and User profile pictures.

3. Responsiveness
Ensure the Typography scales down significantly for mobile (text-5xl for headline).
The Dashboard preview should preserve its layout but become scrollable or stack vertically on smaller screens.
Navbar transforms from a horizontal row to a mobile drawer.

## ClearInvoice SaaS Hero — SaaS [sites/3]

- Preview: https://motionsites.ai/assets/hero-clearinvoice-preview-l3q8sam6.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/3.gif

Create a high-fidelity, dark-mode Hero section for a SaaS product called "ClearInvoice" using React and Tailwind CSS.

Tech Stack:
Framework: React (Vite)
Styling: Tailwind CSS
Animation: motion/react (Framer Motion)
Icons: lucide-react
Video: Native HTML5 <video> with hls.js for streaming (Do NOT use react-player).

1. Background Video (Crucial):
Source: https://stream.mux.com/hUT6X11m1Vkw1QMxPOLgI761x2cfpi9bHFbi5cNg4014.m3u8
Behavior: Autoplay, Loop, Muted, PlaysInline.
Opacity: 100% (No dark overlay).
Implementation: Create a memoized BackgroundVideo component using hls.js to handle the .m3u8 stream natively. Ensure it cleans up properly on unmount to prevent "AbortError".
Z-Index: It must sit behind all content (-z-10).

2. Layout & Styling:
Font Family:
Headings: "Switzer" (Medium weight, tight tracking).
Body: "Geist" (Clean, legible).

Top Bar: A 5px high gradient bar at the very top: from-[#ccf] via-[#e7d04c] to-[#31fb78].
Navbar:
Logo on left.
Links (Features, Pricing, Reviews) centered.
Auth buttons (Sign In, Sign Up) on right.
Mobile: Hamburger menu that opens a full-width dropdown.

3. Hero Content:
Headline: "Manage your online store while save 3x operating cost" (Large text: text-6xl, tight leading).
Subhead: "ClearInvoice takes the hassle out of billing with easy-to-use tools." (White/90).
Animations: Use motion/react to stagger the entrance of the Text, Buttons, and Social Proof (Fade Up + Slide).

4. Button Styles (Exact Recreation):
Primary Button:
Background: Gradient from-[#FF3300] to-[#EE7926].
Glow: An absolute positioned div behind the button with bg-orange-600 blur-lg opacity-20.
Inner Stroke: A 1.5px border overlay (border-white/20) inside the button for a "glassy" edge.
Hover: scale: 1.05, glow increases to opacity-60, and an Arrow icon slides in from the left.

Secondary Button:
Background: bg-white/90 backdrop blur.
Inner Stroke: 1.5px border (border-black/5).
Hover: scale: 1.05, background becomes solid white.

5. Social Proof:
Row of 3 user avatars (overlapping borders).
Text: "Trusted by 210k+ stores worldwide".

## Datacore SaaS Hero — SaaS [sites/4]

- Preview: https://motionsites.ai/assets/hero-datacore-preview-DWeq7Ls3.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/4.gif

Build a high-fidelity, production-ready Hero section for a SaaS product called "Datacore" using React, Tailwind CSS, and Lucide Icons.

### Design Style
- **Theme:** Dark mode, modern, clean, "Linear-style" aesthetic.
- **Background:** Full-screen background video with a black overlay (`bg-black/60`) for text readability.
- **Responsiveness:** Fully responsive for mobile (with a hamburger menu) and desktop.

### Tech Stack Requirements
- Use **React** with **Tailwind CSS**.
- Use **lucide-react** for icons.
- Use **hls.js** to handle the background video streaming (.m3u8) to ensure it works on Chrome/Firefox, while using native HLS for Safari.
- Import these Google Fonts via CSS: 'Inter', 'Manrope', 'Cabin', and 'Instrument Serif'.

### Assets
- **Video Source:** `https://stream.mux.com/4IMYGcL01xjs7ek5ANO17JC4VQVUTsojZlnw4fXzwSxc.m3u8`
- **Video Poster/Thumbnail:** `https://customer-cbeadsgr09pnsezs.cloudflarestream.com/257c7359efd4b4aaebcc03aa8fc78a36/thumbnails/thumbnail.jpg`

### Typography & Colors
- **Global Font:** 'Manrope'
- **Headings:** 'Inter'
- **Buttons/Badges:** 'Cabin'
- **Italic Accent:** 'Instrument Serif'
- **Colors:**
  - Primary Purple: `#7b39fc` (Hover: `#6a2ce0`)
  - Secondary Dark: `#2b2344` (Hover: `#352b54`)
  - Accent Orange: `#f87b52`
  - Glass Border: `rgba(164,132,215,0.5)`
  - Glass Background: `rgba(85,80,110,0.4)`

### Layout & Components

1. **Background Video Component:**
   - Create a robust component that handles HLS streams.
   - It must auto-play, loop, mute, and play inline.
   - It must handle the poster image fading out once the video actually starts playing to prevent black flashes.

2. **Navbar:**
   - **Left:** Logo (Use a white square container with the `Command` icon inside).
   - **Center (Desktop):** Links for "Home", "Services" (with a `ChevronDown` icon), "Reviews", "Contact us".
   - **Right (Desktop):** "Sign In" (Glass effect button) and "Get Started" (Purple button).
   - **Mobile:** Show a Menu toggle button that opens a full-screen black overlay with vertical links.

3. **Hero Content (Centered):**
   - **Badge:** A glassmorphism pill shape containing:
     - A small orange tag: "New"
     - Text: "Say Hello to Datacore v3.2"
   - **Headline (Large, ~76px on desktop):**
     - Line 1: "Your Networks."
     - Line 2: "One Rapid [Italic Serif Font: Interface]."
   - **Subtext:** "Platform helps admins control access, logs, and servers with purpose."
   - **CTA Buttons:**
     - Primary: "Book a Free Demo" (Purple)
     - Secondary: "Get Started Now" (Dark Navy)

Please ensure the code is production-ready, clean, and handles the video loading state gracefully.

## Synapse Dark Hero — SaaS [sites/7]

- Preview: https://motionsites.ai/assets/hero-synapse-preview-CP83ds5W.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/7.gif

Build a high-fidelity, dark-themed Hero Section using React, Tailwind CSS, and Framer Motion. The background should be solid black (#000000).

1. Structure & Layout:

Navbar: Fixed at the top with a blurred glass effect.

Logo: Text "Synapse" (font-medium, tracking-tight, white).

Links: Features (active state with gradient border), Insights, About, Case Studies (strikethrough style), Contact.

CTA: "Get Started for Free" (White/Gray gradient button).

Hero Content: Centered text container (z-10, relative).

Badges: Row of 3 glass-effect badges "Integrated with" + Icon.

Headline: "Where Innovation Meets Execution" (Large ~80px font, tight tracking, fade-in animation).

Subtext: 2-line description about testing and deployment.

Buttons:

"Get Started for Free" (Solid Black background, White border).

"Let's Get Connected" (Transparent glass style).

Logo Marquee: A static row of grayscale, 40% opacity logos (use placeholder SVGs) at the bottom.

2. Background Video (Crucial):

Source: https://stream.mux.com/9JXDljEVWYwWu01PUkAemafDugK89o01BR6zqJ3aS9u00A.m3u8

Implementation: Create a memoized VideoPlayer component using hls.js to handle the .m3u8 stream. Ensure proper cleanup on unmount.

Styling: 100% Opacity (no dark overlays), playing in loop/muted/autoplay.

Positioning: The video container should have a height of 80vh and be positioned absolute bottom-[35vh], sitting effectively "floating" behind the text content but pushed up from the bottom edge.

3. Animations:

Use motion/react to apply staggered fade-in-up animations to the badges, headline, subtitle, and buttons on load.

## AI Meeting Notes — SaaS [sites/ai-meeting-notes]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/ailight123Area.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-meeting-notes.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Livo AI - AI Analysis for Real-Time Discussions</title>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
  <style>
    *, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
    html { scroll-behavior: smooth; }
    body { font-family: 'Inter', sans-serif; overflow-x: hidden; }
    a { text-decoration: none; color: inherit; }
    button { border: none; background: none; font-family: inherit; }
    img { display: block; max-width: 100%; }

    /* ===== ANIMATIONS ===== */
    @keyframes spin-slow { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
    @keyframes ticker-scroll { from { transform: translateX(0); } to { transform: translateX(-33.333%); } }
    @keyframes fadeInUp { from { opacity: 0; transform: translateY(12px); filter: blur(8px); } to { opacity: 1; transform: translateY(0); filter: blur(0); } }
    @keyframes fadeInRight { from { opacity: 0; transform: translateX(40px); } to { opacity: 1; transform: translateX(0); } }
    @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
    @keyframes slideInFromRight { from { opacity: 0; transform: translateX(80px) scale(0.97); } to { opacity: 1; transform: translateX(0) scale(1); } }

    .animate-word { display: inline-block; opacity: 0; transform: translateY(10px); filter: blur(10px); animation: fadeInUp 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards; }
    .animate-fade-in-up { opacity: 0; transform: translateY(24px); animation: fadeInUp 0.7s cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards; }
    .animate-fade-in-right { opacity: 0; animation: fadeInRight 0.7s cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards; animation-delay: 0.1s; }
    .animate-fade-in { opacity: 0; animation: fadeIn 0.5s ease forwards; }

    /* ===== SECTION 1: HERO ===== */
    .hero-section { position: relative; width: 100%; min-height: 100vh; overflow: hidden; display: flex; flex-direction: column; align-items: center; background-color: rgb(254, 241, 238); }
    .hero-inner { width: 100%; display: flex; flex-direction: column; align-items: center; padding-top: 32px; flex: 1; }

    /* Navbar */
    .navbar { width: 100%; max-width: 1440px; margin: 0 auto; display: flex; align-items: center; justify-content: space-between; padding: 0 64px; }
    .navbar-logo { width: 60px; height: 60px; border-radius: 20px; overflow: hidden; flex-shrink: 0; box-shadow: 15px 25px 45px rgba(0,0,0,0.25); }
    .navbar-logo img { width: 100%; height: 100%; object-fit: cover; }
    .navbar-links { display: flex; align-items: center; gap: 54px; }
    .navbar-links a { font-size: 16px; line-height: 16px; font-weight: 500; letter-spacing: 0em; color: #000; transition: opacity 0.2s; }
    .navbar-links a:hover { opacity: 0.7; }
    .navbar-actions { display: flex; align-items: center; gap: 12px; }
    .navbar-btn { width: 50px; height: 50px; border-radius: 16px; border: 1px solid rgba(0,0,0,0.18); display: flex; align-items: center; justify-content: center; cursor: pointer; transition: background 0.2s; }
    .navbar-btn:hover { background: rgba(0,0,0,0.05); }
    .navbar-btn svg { width: 24px; height: 24px; stroke: #000; fill: none; stroke-width: 1.5; }

    /* Hero Content */
    .hero-content { width: 100%; max-width: 1440px; margin: 0 auto; display: flex; flex-direction: row; align-items: center; justify-content: space-between; padding: 100px 64px 80px; gap: 32px; flex: 1; }
    .hero-left { max-width: 520px; display: flex; flex-direction: column; gap: 40px; }
    .hero-heading { font-size: 82px; line-height: 1.1; font-weight: 500; letter-spacing: -0.05em; }
    .hero-heading-line1 { display: block; position: relative; z-index: 1; background: linear-gradient(100deg, rgb(115,34,237) 0%, rgb(253,135,61) 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
    .hero-heading-line2 { display: block; color: #000; }
    .hero-paragraph { font-size: 18px; line-height: 28px; font-weight: 500; letter-spacing: -0.02em; color: #000; text-wrap: balance; }
    .hero-ctas { display: flex; align-items: center; gap: 14px; }

    /* Primary CTA */
    .cta-primary-wrapper { position: relative; padding: 5px; }
    .cta-primary-border { position: absolute; inset: 0; border-radius: 23px; overflow: hidden; }
    .cta-primary-border-inner { position: absolute; top: -200%; left: -250%; width: 600%; height: 600%; background: conic-gradient(from 0deg, rgb(122,50,227) 0%, rgb(253,135,61) 25%, rgb(236,72,153) 50%, rgb(122,50,227) 75%, rgb(253,135,61) 100%); animation: spin-slow 3s linear infinite; }
    .cta-primary-bg { position: absolute; inset: 2px; border-radius: 21px; background: rgb(254,241,238); }
    .cta-primary { position: relative; height: 60px; padding: 0 30px; border-radius: 18px; color: #fff; font-weight: 500; font-size: 18px; line-height: 18px; letter-spacing: -0.02em; cursor: pointer; overflow: hidden; display: flex; align-items: center; gap: 12px; background: linear-gradient(135deg, rgba(122,50,227,1) 0%, rgba(236,72,153,0.9) 50%, rgba(253,135,61,1) 100%); transition: transform 0.2s, box-shadow 0.3s; }
    .cta-primary:hover { transform: scale(1.03); box-shadow: 0 8px 32px rgba(122,50,227,0.35), 0 4px 16px rgba(253,135,61,0.25); }
    .cta-primary:active { transform: scale(0.97); }
    .cta-primary-circle { width: 28px; height: 28px; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.8); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
    .cta-primary-circle svg { width: 14px; height: 14px; margin-left: 1px; }

    /* Secondary CTA */
    .cta-secondary { height: 60px; padding: 0 32px; border-radius: 20px; border: 1px solid rgba(0,0,0,0.25); background: transparent; color: #000; font-weight: 500; font-size: 18px; line-height: 18px; letter-spacing: -0.02em; cursor: pointer; transition: all 0.3s; }
    .cta-secondary:hover { background: #fff; border-color: rgba(0,0,0,0.1); box-shadow: 0 4px 20px rgba(0,0,0,0.08); transform: scale(1.03); }
    .cta-secondary:active { transform: scale(0.97); }

    /* Trust Notes */
    .trust-notes { display: flex; flex-wrap: wrap; align-items: center; gap: 24px; }
    .trust-notes span { font-size: 14px; line-height: 14px; font-weight: 500; letter-spacing: -0.02em; color: #000; white-space: nowrap; }

    /* Hero Image */
    .hero-right { width: 788px; flex-shrink: 0; }
    .hero-image-container { width: 100%; aspect-ratio: 3/2; border-radius: 16px; overflow: hidden; }
    .hero-image-container img { width: 100%; height: 100%; object-fit: cover; }

    /* Offer Strip */
    .offer-strip { width: 100%; background: rgba(255,255,255,0.82); display: flex; flex-direction: column; align-items: center; gap: 18px; padding: 52px 40px 32px; }
    .offer-text { display: flex; align-items: center; gap: 18px; text-align: center; }
    .offer-text span { font-size: 16px; line-height: 16px; font-weight: 500; letter-spacing: -0.02em; color: #000; }
    .offer-text a { font-size: 16px; line-height: 16px; font-weight: 500; letter-spacing: -0.02em; color: rgb(122,50,227); transition: opacity 0.2s; }
    .offer-text a:hover { opacity: 0.7; }

    /* Logo Ticker */
    .logo-ticker { width: 100%; height: 85px; overflow: hidden; position: relative; }
    .logo-ticker-mask { position: absolute; inset: 0; z-index: 10; pointer-events: none; mask-image: linear-gradient(270deg, transparent 0%, black 4.7%, black 95.3%, transparent 100%); -webkit-mask-image: linear-gradient(270deg, transparent 0%, black 4.7%, black 95.3%, transparent 100%); }
    .logo-ticker-track { display: flex; align-items: center; gap: 30px; height: 100%; position: absolute; animation: ticker-scroll 15s linear infinite; }
    .logo-ticker-track img { height: 70px; flex-shrink: 0; object-fit: contain; }

    /* ===== SECTION 2: SCENARIOS ===== */
    .scenarios-section { width: 100%; min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 48px 64px; background-color: rgb(254, 241, 238); }
    .scenarios-inner { width: 100%; max-width: 1440px; margin: 0 auto; display: flex; flex-direction: row; align-items: flex-start; gap: 60px; }
    .scenarios-left { width: 540px; flex-shrink: 0; display: flex; flex-direction: column; justify-content: space-between; height: 830px; }
    .scenarios-left-content { display: flex; flex-direction: column; gap: 36px; }
    .scenarios-label { font-size: 28px; font-weight: 500; line-height: 28px; letter-spacing: -0.05em; background: linear-gradient(to right, #9333ea, #ec4899); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
    .scenarios-heading-container { height: 130px; overflow: visible; }
    .scenarios-heading { font-size: 78px; font-weight: 500; line-height: 1.05; letter-spacing: -0.05em; color: #000; transition: opacity 0.5s, transform 0.5s; }
    .scenarios-paragraph-container { height: 130px; }
    .scenarios-paragraph { font-size: 21px; line-height: 1.55; letter-spacing: -0.02em; color: rgba(0,0,0,0.75); max-width: 560px; transition: opacity 0.45s, transform 0.45s; }
    .scenarios-features { height: 248px; display: flex; flex-direction: column; gap: 16px; margin-top: 4px; }
    .scenario-feature-card { display: flex; align-items: center; gap: 16px; background: #fff; border-radius: 18px; height: 72px; padding: 0 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.04); opacity: 0; transform: translateY(14px); transition: opacity 0.5s, transform 0.5s; }
    .scenario-feature-card.visible { opacity: 1; transform: translateY(0); }
    .scenario-feature-icon { width: 36px; height: 36px; border-radius: 50%; background: linear-gradient(to bottom right, #f3e8ff, #fce7f3); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
    .scenario-feature-icon svg { width: 18px; height: 18px; stroke: #9333ea; stroke-width: 2.5; fill: none; }
    .scenario-feature-text { font-size: 18px; font-weight: 500; line-height: 22px; color: rgba(0,0,0,0.8); }

    /* Pagination */
    .scenarios-pagination { display: flex; align-items: center; gap: 12px; }
    .pagination-btn { position: relative; width: 44px; height: 44px; cursor: pointer; border-radius: 50%; }
    .pagination-btn-inactive { width: 100%; height: 100%; border-radius: 50%; border: 1px solid rgba(0,0,0,0.2); display: flex; align-items: center; justify-content: center; transition: border-color 0.2s; }
    .pagination-btn-inactive:hover { border-color: rgba(0,0,0,0.4); }
    .pagination-btn-inactive span { font-size: 16px; font-weight: 500; color: rgba(0,0,0,0.3); letter-spacing: -0.05em; }
    .pagination-btn-active { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
    .pagination-btn-active span { font-size: 16px; font-weight: 600; color: #000; letter-spacing: -0.05em; z-index: 1; }
    .pagination-btn svg { position: absolute; inset: 0; width: 100%; height: 100%; transform: rotate(-90deg); }
    .pagination-btn circle.bg { fill: white; stroke: rgba(0,0,0,0.06); stroke-width: 2.5; }
    .pagination-btn circle.progress { fill: none; stroke: url(#progressGrad); stroke-width: 2.5; stroke-linecap: round; transition: stroke-dasharray 0.03s linear; }

    /* Scenarios Right */
    .scenarios-right { flex: 1; flex-shrink: 0; display: flex; align-items: center; height: 830px; }
    .scenarios-image-container { position: relative; width: 100%; height: 100%; overflow: hidden; border-radius: 16px; }
    .scenarios-image { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; transition: opacity 0.7s cubic-bezier(0.44,0,0.56,1), transform 0.7s cubic-bezier(0.44,0,0.56,1); }
    .scenarios-image img { height: 100%; width: auto; max-width: none; }
    .scenarios-image.entering { opacity: 0; transform: translateX(80px) scale(0.97); }
    .scenarios-image.active { opacity: 1; transform: translateX(0) scale(1); }
    .scenarios-image.exiting { opacity: 0; transform: translateX(-50px) scale(0.97); }

    /* ===== SECTION 3: APP ADVERT ===== */
    .app-advert-section { position: relative; width: 100%; height: 100vh; overflow: hidden; display: flex; align-items: center; }
    .app-advert-bg { position: absolute; inset: 0; background-size: cover; background-position: center; transition: opacity 0.6s ease-in-out; }
    .app-advert-overlay { position: absolute; inset: 0; background: linear-gradient(to right, rgba(0,0,0,0.6), rgba(0,0,0,0.3), transparent); }
    .app-advert-content { position: relative; z-index: 10; width: 100%; height: 100%; display: flex; align-items: center; padding: 64px; }
    .app-advert-inner { width: 100%; max-width: 1440px; margin: 0 auto; }
    .app-advert-text { display: flex; flex-direction: column; gap: 48px; max-width: 580px; }
    .app-advert-icon { width: 110px; height: 110px; border-radius: 28px; overflow: hidden; box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25); }
    .app-advert-icon img { width: 100%; height: 100%; object-fit: cover; }
    .app-advert-heading-group { display: flex; flex-direction: column; gap: 36px; }
    .app-advert-heading { font-size: 78px; line-height: 82px; font-weight: 500; letter-spacing: -0.05em; color: #fff; transition: opacity 0.6s, transform 0.6s; }
    .app-advert-paragraph { font-size: 22px; line-height: 30px; font-weight: 500; letter-spacing: -0.03em; color: rgba(255,255,255,0.9); transition: opacity 0.5s 0.1s, transform 0.5s 0.1s; }
    .app-advert-cta { display: flex; align-items: center; gap: 8px; height: 66px; padding: 0 28px; border-radius: 20px; width: fit-content; cursor: pointer; background: linear-gradient(165deg, rgba(122,50,227,1) 0%, rgba(253,135,61,1) 100%); transition: transform 0.2s, box-shadow 0.3s; }
    .app-advert-cta:hover { transform: scale(1.04); box-shadow: 0 8px 30px rgba(122,50,227,0.3); }
    .app-advert-cta:active { transform: scale(0.97); }
    .app-advert-cta svg { width: 32px; height: 32px; stroke: #fff; fill: none; stroke-width: 1.5; }
    .app-advert-cta span { font-size: 19px; line-height: 19px; font-weight: 500; letter-spacing: -0.02em; color: #fff; }

    /* ===== SECTION 4: PRICING ===== */
    .pricing-section { width: 100%; min-height: 100vh; background: #fff; display: flex; align-items: center; padding: 80px 0; }
    .pricing-inner { width: 100%; max-width: 1440px; margin: 0 auto; padding: 0 64px; }
    .pricing-header { display: flex; flex-direction: column; align-items: center; gap: 40px; }
    .pricing-title { font-size: 58px; font-weight: 500; line-height: 1; letter-spacing: -0.05em; color: #000; text-align: center; }

    /* Billing Toggle */
    .billing-toggle { display: flex; align-items: center; gap: 20px; justify-content: center; }
    .billing-toggle-label { font-size: 16px; font-weight: 500; line-height: 16px; color: #000; transition: opacity 0.3s; }
    .billing-toggle-label.active { opacity: 1; }
    .billing-toggle-label.inactive { opacity: 0.4; }
    .billing-toggle-track { position: relative; width: 52px; height: 32px; border-radius: 999px; padding: 3px; cursor: pointer; flex-shrink: 0; background: linear-gradient(135deg, rgba(122,50,227,1) 0%, rgba(253,135,61,1) 100%); transition: box-shadow 0.2s; }
    .billing-toggle-track:hover { box-shadow: 0 2px 8px rgba(0,0,0,0.15); }
    .billing-toggle-thumb { width: 26px; height: 26px; border-radius: 50%; background: #fff; box-shadow: 0 1px 3px rgba(0,0,0,0.1); transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1); }
    .billing-toggle-thumb.monthly { transform: translateX(20px); }

    /* Pricing Cards */
    .pricing-cards { margin-top: 72px; display: flex; flex-direction: row; gap: 24px; align-items: stretch; }
    .pricing-card { flex: 1; border-radius: 30px; overflow: hidden; display: flex; flex-direction: column; opacity: 0; transform: translateY(36px); transition: opacity 0.6s cubic-bezier(0.25,0.46,0.45,0.94), transform 0.6s cubic-bezier(0.25,0.46,0.45,0.94); }
    .pricing-card.visible { opacity: 1; transform: translateY(0); }
    .pricing-card-top { position: relative; display: flex; flex-direction: column; align-items: center; padding: 64px 48px 40px; gap: 28px; min-height: 400px; }
    .pricing-card-top.light { background-color: rgb(254, 241, 238); }
    .pricing-card-top.dark { background-color: #000; }
    .pricing-card-bottom { display: flex; flex-direction: column; padding: 40px 48px; flex: 1; }
    .pricing-card-bottom.light { background-color: rgb(252, 225, 224); }
    .pricing-card-bottom.dark { background-color: rgb(24, 24, 24); }

    /* Ribbon */
    .ribbon { position: absolute; top: 24px; right: -32px; z-index: 10; transform: rotate(45deg); }
    .ribbon-inner { padding: 7px 40px; text-align: center; background: linear-gradient(135deg, rgba(122,50,227,1) 0%, rgba(253,135,61,1) 100%); }
    .ribbon-inner span { font-size: 13px; font-weight: 500; color: #fff; white-space: nowrap; }

    .pricing-plan-name { font-size: 56px; font-weight: 500; line-height: 1; letter-spacing: -0.05em; text-align: center; }
    .pricing-plan-name.light { color: #000; }
    .pricing-plan-name.dark { color: #fff; }
    .pricing-description { font-size: 18px; font-weight: 500; line-height: 22px; letter-spacing: -0.02em; text-align: center; }
    .pricing-description.light { color: rgba(0,0,0,0.8); }
    .pricing-description.dark { color: rgba(255,255,255,0.8); }

    .pricing-price { display: flex; align-items: center; gap: 12px; }
    .pricing-price-old { font-size: 38px; font-weight: 500; line-height: 30px; letter-spacing: -0.05em; text-decoration: line-through; }
    .pricing-price-old.dark { color: rgba(255,255,255,0.5); }
    .pricing-price-current { font-size: 38px; font-weight: 500; line-height: 30px; letter-spacing: -0.05em; transition: opacity 0.4s, transform 0.4s; }
    .pricing-price-current.light { color: rgba(0,0,0,0.5); }
    .pricing-price-current.light.contact { color: #000; }
    .pricing-price-current.dark { color: #fff; }
    .pricing-price-current.dark.discounted { color: #fff; }
    .pricing-price-current.dark.no-discount { color: rgba(255,255,255,0.5); }

    /* Pricing CTA buttons */
    .pricing-cta { display: flex; align-items: center; justify-content: center; gap: 10px; height: 66px; width: 100%; border-radius: 20px; padding: 0 28px; cursor: pointer; transition: transform 0.2s, box-shadow 0.3s; }
    .pricing-cta:hover { transform: scale(1.03); }
    .pricing-cta:active { transform: scale(0.97); }
    .pricing-cta.black { background: #000; }
    .pricing-cta.black:hover { box-shadow: 0 8px 24px rgba(0,0,0,0.2); }
    .pricing-cta.gradient { background: linear-gradient(165deg, rgba(122,50,227,1) 0%, rgba(253,135,61,1) 100%); }
    .pricing-cta.gradient:hover { box-shadow: 0 8px 30px rgba(122,50,227,0.25); }
    .pricing-cta-circle { width: 30px; height: 30px; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.6); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
    .pricing-cta-circle svg { width: 15px; height: 15px; stroke: #fff; stroke-width: 2; fill: none; }
    .pricing-cta span { font-size: 19px; font-weight: 500; line-height: 19px; letter-spacing: -0.02em; color: #fff; }

    .pricing-subtext { font-size: 14px; font-weight: 500; line-height: 14px; text-align: center; transition: opacity 0.4s; }
    .pricing-subtext.light { color: rgba(0,0,0,0.5); }
    .pricing-subtext.dark { color: rgba(255,255,255,0.5); }

    /* Features list */
    .feature-list { display: flex; flex-direction: column; gap: 16px; }
    .feature-item { display: flex; align-items: center; gap: 14px; }
    .feature-check { width: 28px; height: 28px; border-radius: 50%; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
    .feature-check.light { background: rgba(0,0,0,0.05); }
    .feature-check.dark { background: rgba(255,255,255,0.1); }
    .feature-check svg { width: 16px; height: 16px; stroke-width: 2.5; fill: none; }
    .feature-check.light svg { stroke: rgba(0,0,0,0.7); }
    .feature-check.dark svg { stroke: #fff; }
    .feature-text { font-size: 18px; font-weight: 500; line-height: 22px; letter-spacing: -0.02em; }
    .feature-text.light { color: rgba(0,0,0,0.8); }
    .feature-text.dark { color: #fff; }

    /* ===== RESPONSIVE ===== */
    @media (max-width: 1024px) {
      .navbar { padding: 0 40px; }
      .navbar-links { display: none; }
      .navbar-actions { display: none; }
      .hero-content { flex-direction: column; padding: 80px 40px 80px; gap: 48px; }
      .hero-left { max-width: 100%; }
      .hero-heading { font-size: 64px; }
      .hero-right { width: 100%; }
      .scenarios-section { padding: 48px 40px; }
      .scenarios-inner { flex-direction: column; }
      .scenarios-left { width: 100%; height: auto; }
      .scenarios-right { height: 440px; width: 100%; }
      .scenarios-heading { font-size: 62px; }
      .scenarios-heading-container { height: 100px; }
      .scenarios-paragraph { font-size: 19px; }
      .scenarios-paragraph-container { height: 100px; }
      .app-advert-content { padding: 40px; }
      .app-advert-heading { font-size: 66px; line-height: 70px; }
      .app-advert-paragraph { font-size: 19px; line-height: 27px; }
      .pricing-inner { padding: 0 40px; }
      .pricing-cards { flex-direction: column; }
    }
    @media (max-width: 768px) {
      .navbar { padding: 0 20px; }
      .navbar-logo { width: 50px; height: 50px; border-radius: 16px; }
      .hero-content { padding: 64px 20px 64px; }
      .hero-heading { font-size: 44px; }
      .hero-paragraph { font-size: 16px; line-height: 24px; }
      .hero-ctas { flex-direction: column; align-items: flex-start; gap: 12px; }
      .cta-primary { height: 56px; padding: 0 24px; font-size: 17px; }
      .cta-secondary { height: 56px; padding: 0 28px; font-size: 17px; }
      .offer-strip { padding: 32px 20px; }
      .offer-text { flex-direction: column; gap: 12px; }
      .scenarios-section { padding: 40px 20px; }
      .scenarios-heading { font-size: 44px; }
      .scenarios-heading-container { height: 110px; }
      .scenarios-paragraph { font-size: 17px; }
      .scenarios-paragraph-container { height: 120px; }
      .scenarios-right { height: 320px; }
      .app-advert-content { padding: 24px; }
      .app-advert-icon { width: 80px; height: 80px; border-radius: 22px; }
      .app-advert-heading { font-size: 52px; line-height: 56px; }
      .app-advert-paragraph { font-size: 17px; line-height: 24px; }
      .app-advert-cta { height: 56px; padding: 0 24px; }
      .app-advert-cta svg { width: 28px; height: 28px; }
      .app-advert-cta span { font-size: 17px; }
      .pricing-inner { padding: 0 20px; }
      .pricing-title { font-size: 46px; }
      .pricing-card-top { padding: 56px 28px 40px; }
      .pricing-card-bottom { padding: 36px 28px; }
      .pricing-plan-name { font-size: 42px; }
    }
  </style>
</head>
<body>

  <!-- ===== SECTION 1: HERO ===== -->
  <section class="hero-section">
    <div class="hero-inner">
      <!-- Navbar -->
      <nav class="navbar">
        <div class="navbar-logo">
          <img src="https://framerusercontent.com/images/sXJWPys5DXyez95t6axrD3kbJkc.png" alt="Livo" />
        </div>
        <div class="navbar-links">
          <a href="#">How it works</a>
          <a href="#">Use cases</a>
          <a href="#">Features</a>
          <a href="#">Pricing</a>
          <a href="#">FAQ</a>
        </div>
        <div class="navbar-actions">
          <button class="navbar-btn">
            <svg viewBox="0 0 24 24"><path d="M5 12h14m0 0l-4-4m4 4l-4 4" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
          <button class="navbar-btn">
            <svg viewBox="0 0 24 24"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
        </div>
      </nav>

      <!-- Hero Content -->
      <div class="hero-content">
        <div class="hero-left animate-fade-in-up">
          <h1 class="hero-heading">
            <span class="hero-heading-line1" id="heroLine1"></span>
            <span class="hero-heading-line2" id="heroLine2"></span>
          </h1>
          <p class="hero-paragraph" id="heroParagraph"></p>
          <div class="hero-ctas">
            <div class="cta-primary-wrapper">
              <div class="cta-primary-border"><div class="cta-primary-border-inner"></div></div>
              <div class="cta-primary-bg"></div>
              <button class="cta-primary">
                <span class="cta-primary-circle">
                  <svg viewBox="0 0 14 14" fill="none"><path d="M3 7h8m0 0L8 4m3 3L8 10" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </span>
                <span>Start for free</span>
              </button>
            </div>
            <button class="cta-secondary">Contact us</button>
          </div>
          <div class="trust-notes">
            <span>&bull; 31-day free trial</span>
            <span>&bull; No credit card required</span>
            <span>&bull; Cancel anytime</span>
          </div>
        </div>
        <div class="hero-right animate-fade-in-right">
          <div class="hero-image-container">
            <img src="https://framerusercontent.com/images/b4pOG23X1MeuH63d5Dmm4HFLVA.png" alt="Livo AI interface" />
          </div>
        </div>
      </div>

      <!-- Offer Strip -->
      <div class="offer-strip">
        <div class="offer-text">
          <span>Enjoy 50% off premium features for first 3 months — 21 days remaining</span>
          <a href="#">Start 14 days trial</a>
        </div>
        <div class="logo-ticker">
          <div class="logo-ticker-mask"></div>
          <div class="logo-ticker-track" id="logoTrack"></div>
        </div>
      </div>
    </div>
  </section>

  <!-- ===== SECTION 2: SCENARIOS ===== -->
  <section class="scenarios-section" id="scenariosSection">
    <div class="scenarios-inner">
      <div class="scenarios-left">
        <div class="scenarios-left-content">
          <span class="scenarios-label animate-fade-in">Scenarios</span>
          <div class="scenarios-heading-container">
            <h2 class="scenarios-heading" id="scenariosHeading"></h2>
          </div>
          <div class="scenarios-paragraph-container">
            <p class="scenarios-paragraph" id="scenariosParagraph"></p>
          </div>
          <div class="scenarios-features" id="scenariosFeatures"></div>
        </div>
        <div class="scenarios-pagination" id="scenariosPagination"></div>
      </div>
      <div class="scenarios-right">
        <div class="scenarios-image-container" id="scenariosImageContainer"></div>
      </div>
    </div>
  </section>

  <!-- ===== SECTION 3: APP ADVERT ===== -->
  <section class="app-advert-section" id="appAdvertSection">
    <div class="app-advert-bg" id="appAdvertBg1" style="background-image: url('https://framerusercontent.com/images/qnyDJGivgHQMm5JaWxQxdKn3q0.png'); opacity: 1;"></div>
    <div class="app-advert-bg" id="appAdvertBg2" style="background-image: url('https://polo-pecan-73837341.figma.site/_assets/v11/f71ca5dd250ff31df02f32da412dc606df352cc5.png?w=2191'); opacity: 0;"></div>
    <div class="app-advert-overlay"></div>
    <div class="app-advert-content">
      <div class="app-advert-inner">
        <div class="app-advert-text">
          <div class="app-advert-icon">
            <img src="https://framerusercontent.com/images/sXJWPys5DXyez95t6axrD3kbJkc.png" alt="Livo" />
          </div>
          <div class="app-advert-heading-group">
            <h2 class="app-advert-heading" id="appAdvertHeading">AI analysis</h2>
            <p class="app-advert-paragraph" id="appAdvertParagraph">Record meetings wherever you are with the Livo AI mobile app. From video calls to in-person conversations, effortlessly capture audio, generate live transcripts, and review actionable insights right from your phone.</p>
          </div>
          <button class="app-advert-cta">
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M8 12l4 4 4-4M12 8v8" stroke-linecap="round" stroke-linejoin="round"/></svg>
            <span>Download App</span>
          </button>
        </div>
      </div>
    </div>
  </section>

  <!-- ===== SECTION 4: PRICING ===== -->
  <section class="pricing-section" id="pricingSection">
    <div class="pricing-inner">
      <div class="pricing-header">
        <h2 class="pricing-title">Plans</h2>
        <div class="billing-toggle">
          <span class="billing-toggle-label active" id="yearlyLabel">Yearly</span>
          <div class="billing-toggle-track" id="billingToggle" onclick="toggleBilling()">
            <div class="billing-toggle-thumb" id="billingThumb"></div>
          </div>
          <span class="billing-toggle-label inactive" id="monthlyLabel">Monthly</span>
        </div>
      </div>
      <div class="pricing-cards" id="pricingCards"></div>
    </div>
  </section>

  <!-- SVG Defs for gradients -->
  <svg width="0" height="0" style="position:absolute">
    <defs>
      <linearGradient id="progressGrad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="#9333ea"/>
        <stop offset="100%" stop-color="#ec4899"/>
      </linearGradient>
    </defs>
  </svg>

  <script>
    // ===== ANIMATED WORDS HELPER =====
    function animateWords(container, text, baseDelay = 0, staggerDelay = 0.05) {
      container.innerHTML = '';
      const words = text.split(' ');
      words.forEach((word, i) => {
        const span = document.createElement('span');
        span.className = 'animate-word';
        span.textContent = word + (i < words.length - 1 ? '\u00A0' : '');
        span.style.animationDelay = (baseDelay + i * staggerDelay) + 's';
        container.appendChild(span);
      });
    }

    // ===== HERO ANIMATED TEXT =====
    animateWords(document.getElementById('heroLine1'), 'AI analysis', 0.1, 0.05);
    animateWords(document.getElementById('heroLine2'), 'for real-time discussions', 0.25, 0.05);
    animateWords(document.getElementById('heroParagraph'), "Livo AI records your meetings, recognizes who's speaking, and provides real-time insights and live recommendations — all without taking manual notes.", 0.5, 0.02);

    // ===== LOGO TICKER =====
    const LOGOS = [
      { src: 'https://framerusercontent.com/images/Qo4XNTbEsI5VNAtleec5o3fWg.png', width: 210 },
      { src: 'https://framerusercontent.com/images/t6BIfZjwwbbizLquISVq96n6EGc.png', width: 210 },
      { src: 'https://framerusercontent.com/images/blfT46mvLdPrSwL7JUMxh1mUVI.png', width: 210 },
      { src: 'https://framerusercontent.com/images/zDwbpG7hVs2UTJsIh3Fwr3eX4E.png', width: 164 },
      { src: 'https://framerusercontent.com/images/F2VMPPEvVSp3zSIiTC7dXDzw.png', width: 210 },
    ];
    const logoTrack = document.getElementById('logoTrack');
    for (let r = 0; r < 3; r++) {
      LOGOS.forEach(logo => {
        const img = document.createElement('img');
        img.src = logo.src;
        img.style.width = logo.width + 'px';
        img.alt = '';
        logoTrack.appendChild(img);
      });
    }

    // ===== SECTION 2: SCENARIOS SLIDER =====
    const SCENARIO_SLIDES = [
      {
        heading: 'Talent & Hiring',
        paragraph: 'Run interviews more efficiently by capturing candidate answers automatically and to turn them into clear, useful insights.',
        features: ['Structured interviews with standardized questions', 'Clear insights and hiring recommendations', 'Automatic recording and transcription'],
        image: 'https://framerusercontent.com/images/J8MYD2sMAbepbr2MiuyxCmAYEgk.png',
      },
      {
        heading: 'Commercial Teams',
        paragraph: 'Record all customer interactions to ensure precise follow-ups and use insights to accelerate deal progression.',
        features: ['Track every customer touchpoint', 'Leverage insights to drive deals', 'Enable accurate follow-ups'],
        image: 'https://polo-pecan-73837341.figma.site/_assets/v11/56974c2f2a0bcc77e6331ef7df0ebdd1d7d4d377.png',
      },
      {
        heading: 'Management & Teams',
        paragraph: 'Capture decisions with speaker-tagged transcripts and receive automated summaries. Broadcast live sessions so everyone stays updated in real time.',
        features: ['Generate instant reports from the transcripts.', 'Keep the team updated in real time.'],
        image: 'https://polo-pecan-73837341.figma.site/_assets/v11/b1bded3078219cd54a5a2fef5cb4919c68e7c261.png',
      },
      {
        heading: 'Remote Team Members',
        paragraph: 'Remain focused during meetings as Livo AI provides real-time transcription. Quickly act on key points with clear summaries.',
        features: ['Capture every word instantly without manual note-taking.', 'Get concise, actionable insights from discussions.'],
        image: 'https://polo-pecan-73837341.figma.site/_assets/v11/1c14b7ac2dcdea9ad19040e054b91f33dd4ed3ec.png',
      },
    ];

    let scenarioActive = 0;
    let scenarioProgress = 0;
    let scenarioStartTime = Date.now();
    const SCENARIO_DURATION = 5000;

    function renderScenarioSlide(index) {
      const slide = SCENARIO_SLIDES[index];
      const headingEl = document.getElementById('scenariosHeading');
      const paragraphEl = document.getElementById('scenariosParagraph');
      const featuresEl = document.getElementById('scenariosFeatures');
      const imageContainer = document.getElementById('scenariosImageContainer');

      headingEl.style.opacity = '0';
      headingEl.style.transform = 'translateY(20px)';
      paragraphEl.style.opacity = '0';
      paragraphEl.style.transform = 'translateY(14px)';

      setTimeout(() => {
        headingEl.textContent = slide.heading;
        headingEl.style.opacity = '1';
        headingEl.style.transform = 'translateY(0)';
      }, 50);

      setTimeout(() => {
        paragraphEl.textContent = slide.paragraph;
        paragraphEl.style.opacity = '1';
        paragraphEl.style.transform = 'translateY(0)';
      }, 130);

      featuresEl.innerHTML = '';
      slide.features.forEach((feature, i) => {
        const card = document.createElement('div');
        card.className = 'scenario-feature-card';
        card.innerHTML = `
          <div class="scenario-feature-icon">
            <svg viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </div>
          <span class="scenario-feature-text">${feature}</span>
        `;
        featuresEl.appendChild(card);
        setTimeout(() => card.classList.add('visible'), 200 + i * 90);
      });

      // Image transition
      const existingImg = imageContainer.querySelector('.scenarios-image');
      if (existingImg) {
        existingImg.classList.remove('active');
        existingImg.classList.add('exiting');
        setTimeout(() => existingImg.remove(), 700);
      }
      const newImg = document.createElement('div');
      newImg.className = 'scenarios-image entering';
      newImg.innerHTML = `<img src="${slide.image}" alt="${slide.heading}" />`;
      imageContainer.appendChild(newImg);
      setTimeout(() => { newImg.classList.remove('entering'); newImg.classList.add('active'); }, 50);
    }

    function renderScenarioPagination() {
      const container = document.getElementById('scenariosPagination');
      container.innerHTML = '';
      SCENARIO_SLIDES.forEach((_, i) => {
        const btn = document.createElement('button');
        btn.className = 'pagination-btn';
        const label = String(i + 1).padStart(2, '0');
        if (i === scenarioActive) {
          btn.innerHTML = `
            <svg viewBox="0 0 44 44">
              <circle class="bg" cx="22" cy="22" r="20"/>
              <circle class="progress" cx="22" cy="22" r="20" stroke-dasharray="0 125.66" id="progressCircle${i}"/>
            </svg>
            <div class="pagination-btn-active"><span>${label}</span></div>
          `;
        } else {
          btn.innerHTML = `<div class="pagination-btn-inactive"><span>${label}</span></div>`;
        }
        btn.onclick = () => goToScenarioSlide(i);
        container.appendChild(btn);
      });
    }

    function goToScenarioSlide(index) {
      scenarioActive = index;
      scenarioProgress = 0;
      scenarioStartTime = Date.now();
      renderScenarioSlide(index);
      renderScenarioPagination();
    }

    // Init scenarios
    renderScenarioSlide(0);
    renderScenarioPagination();

    // Scenarios timer
    setInterval(() => {
      const elapsed = Date.now() - scenarioStartTime;
      scenarioProgress = Math.min(elapsed / SCENARIO_DURATION, 1);
      const circle = document.getElementById('progressCircle' + scenarioActive);
      if (circle) {
        circle.setAttribute('stroke-dasharray', `${scenarioProgress * 125.66} 125.66`);
      }
      if (scenarioProgress >= 1) {
        goToScenarioSlide((scenarioActive + 1) % SCENARIO_SLIDES.length);
      }
    }, 30);

    // ===== SECTION 3: APP ADVERT SLIDER =====
    const APP_ADVERT_SLIDES = [
      {
        bg: 'https://framerusercontent.com/images/qnyDJGivgHQMm5JaWxQxdKn3q0.png',
        heading: 'AI analysis',
        paragraph: 'Record meetings wherever you are with the Livo AI mobile app. From video calls to in-person conversations, effortlessly capture audio, generate live transcripts, and review actionable insights right from your phone.',
      },
      {
        bg: 'https://polo-pecan-73837341.figma.site/_assets/v11/f71ca5dd250ff31df02f32da412dc606df352cc5.png?w=2191',
        heading: 'In real-time mode',
        paragraph: 'Start using Livo AI to seamlessly capture your meetings, turn conversations into clear understanding, and easily share key takeaways with your team.',
      },
    ];

    let appAdvertActive = 0;
    const bg1 = document.getElementById('appAdvertBg1');
    const bg2 = document.getElementById('appAdvertBg2');
    const advertHeading = document.getElementById('appAdvertHeading');
    const advertParagraph = document.getElementById('appAdvertParagraph');

    function switchAppAdvertSlide() {
      appAdvertActive = (appAdvertActive + 1) % APP_ADVERT_SLIDES.length;
      const slide = APP_ADVERT_SLIDES[appAdvertActive];

      if (appAdvertActive === 0) {
        bg1.style.opacity = '1';
        bg2.style.opacity = '0';
      } else {
        bg1.style.opacity = '0';
        bg2.style.opacity = '1';
      }

      advertHeading.style.opacity = '0';
      advertHeading.style.transform = 'translateY(20px)';
      advertParagraph.style.opacity = '0';
      advertParagraph.style.transform = 'translateY(14px)';

      setTimeout(() => {
        advertHeading.textContent = slide.heading;
        advertHeading.style.opacity = '1';
        advertHeading.style.transform = 'translateY(0)';
      }, 150);
      setTimeout(() => {
        advertParagraph.textContent = slide.paragraph;
        advertParagraph.style.opacity = '1';
        advertParagraph.style.transform = 'translateY(0)';
      }, 250);
    }
    setInterval(switchAppAdvertSlide, 7000);

    // ===== SECTION 4: PRICING =====
    let isYearly = true;

    const PLANS = [
      {
        name: 'Individual',
        description: 'Well suited for beginning without any expenses.',
        price: '0\u20AC', monthlyPrice: '0\u20AC',
        oldPrice: null,
        cta: 'Free forever', variant: 'black',
        subtext: '0\u20AC per month', monthlySubtext: '0\u20AC per month',
        features: ['Mobile application', 'Auto transcript', 'Unlimited sessions', 'Unlimited contacts'],
        dark: false, ribbon: false,
      },
      {
        name: 'Advanced',
        description: 'Unlimited premium tools',
        price: '132\u20AC', monthlyPrice: '165\u20AC',
        oldPrice: '165\u20AC',
        cta: 'Free 14 days trial', variant: 'gradient',
        subtext: '132\u20AC per month, paid annually', monthlySubtext: '165\u20AC per month',
        features: ['Prompt modes', 'Team collaboration', 'Advanced summaries', 'Live time history'],
        dark: true, ribbon: true,
      },
      {
        name: 'Business',
        description: 'Tailored or self-managed solutions',
        price: 'Contact us', monthlyPrice: 'Contact us',
        oldPrice: null,
        cta: 'Request pricing', variant: 'black',
        subtext: "Schedule a short call \u2014 we'll set everything up for you.",
        monthlySubtext: "Schedule a short call \u2014 we'll set everything up for you.",
        features: ['Self-hosted options', 'HIPAA support', 'Custom LLMs', 'Priority support'],
        dark: false, ribbon: false,
      },
    ];

    function renderPricingCards() {
      const container = document.getElementById('pricingCards');
      container.innerHTML = '';
      PLANS.forEach((plan, i) => {
        const theme = plan.dark ? 'dark' : 'light';
        const displayPrice = isYearly ? plan.price : plan.monthlyPrice;
        const displaySubtext = isYearly ? plan.subtext : plan.monthlySubtext;
        const showOldPrice = isYearly && plan.oldPrice;

        let priceClass = `pricing-price-current ${theme}`;
        if (plan.dark && showOldPrice) priceClass += ' discounted';
        else if (plan.dark && !showOldPrice) priceClass += ' no-discount';
        if (!plan.dark && plan.price === 'Contact us') priceClass += ' contact';

        const card = document.createElement('div');
        card.className = 'pricing-card';
        card.style.transitionDelay = (i * 0.1) + 's';
        card.innerHTML = `
          <div class="pricing-card-top ${theme}">
            ${plan.ribbon ? '<div class="ribbon"><div class="ribbon-inner"><span>Top choice</span></div></div>' : ''}
            <h3 class="pricing-plan-name ${theme}">${plan.name}</h3>
            <p class="pricing-description ${theme}">${plan.description}</p>
            <div class="pricing-price">
              ${showOldPrice ? `<span class="pricing-price-old ${theme}">${plan.oldPrice}</span>` : ''}
              <span class="${priceClass}">${displayPrice}</span>
            </div>
            <button class="pricing-cta ${plan.variant}">
              <div class="pricing-cta-circle">
                <svg viewBox="0 0 24 24"><path d="M7 17L17 7M17 7H7M17 7V17" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </div>
              <span>${plan.cta}</span>
            </button>
            <span class="pricing-subtext ${theme}">${displaySubtext}</span>
          </div>
          <div class="pricing-card-bottom ${theme}">
            <div class="feature-list">
              ${plan.features.map(f => `
                <div class="feature-item">
                  <div class="feature-check ${theme}">
                    <svg viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </div>
                  <span class="feature-text ${theme}">${f}</span>
                </div>
              `).join('')}
            </div>
          </div>
        `;
        container.appendChild(card);
        setTimeout(() => card.classList.add('visible'), 100 + i * 100);
      });
    }

    function toggleBilling() {
      isYearly = !isYearly;
      const thumb = document.getElementById('billingThumb');
      const yearlyLabel = document.getElementById('yearlyLabel');
      const monthlyLabel = document.getElementById('monthlyLabel');

      if (isYearly) {
        thumb.classList.remove('monthly');
        yearlyLabel.classList.add('active');
        yearlyLabel.classList.remove('inactive');
        monthlyLabel.classList.add('inactive');
        monthlyLabel.classList.remove('active');
      } else {
        thumb.classList.add('monthly');
        yearlyLabel.classList.add('inactive');
        yearlyLabel.classList.remove('active');
        monthlyLabel.classList.add('active');
        monthlyLabel.classList.remove('inactive');
      }
      renderPricingCards();
    }

    renderPricingCards();

    // ===== INTERSECTION OBSERVER FOR SCROLL ANIMATIONS =====
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
        }
      });
    }, { threshold: 0.1, rootMargin: '-40px' });

    document.querySelectorAll('.pricing-card').forEach(el => observer.observe(el));
  </script>
</body>
</html>

## Apex SaaS — SaaS [sites/apex-saas-hero]

- Preview: https://motionsites.ai/assets/hero-apex-saas-preview-CbnBKSPv.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/apex-saas-hero.gif

Build a dark SaaS landing page with three sections: a floating glassmorphic navbar, a hero section, and a social proof section with a background video. Use React + Tailwind CSS + TypeScript.

Font: Install @fontsource/geist-sans (weights 400, 500, 600, 700). Set body font to 'Geist Sans', 'Inter', system-ui, sans-serif.

Color System (HSL, CSS variables in :root):

--background: 260 87% 3%
--foreground: 40 6% 95%
--primary: 262 83% 58%
--primary-foreground: 0 0% 100%
--secondary: 240 4% 16%
--secondary-foreground: 40 6% 95%
--muted: 240 4% 16%
--muted-foreground: 240 5% 65%
--border: 240 4% 20%
--hero-heading: 40 10% 96%
--hero-sub: 40 6% 82%
--card: 240 6% 9%
--ring: 262 83% 58%
--radius: 0.75rem
Register hero.heading and hero.sub as Tailwind colors mapped to these variables.

Liquid Glass Utility Class (.liquid-glass):

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

Section 1 — Navbar: A horizontally centered nav with py-5. Inside, a liquid-glass rounded-3xl p-2 container, max-w-[850px] w-full, using flex items-center justify-between gap-6. Contains:

Logo (left): A w-7 h-7 rounded-lg div with bg-gradient-to-b from-secondary to-muted border border-border, containing an inline SVG (circle + 4 crosshair lines, white strokes). Next to it, the text "APEX" styled text-foreground text-xl font-semibold tracking-tight.
Nav links (center): Four buttons — "Features" (with ChevronDown icon), "Solutions", "Plans", "Learning" (with ChevronDown icon). Each: px-3 py-2 text-foreground/90 text-base hover:text-foreground transition-colors.
CTA (right): A rounded-xl px-4 py-2 button using bg-primary text-primary-foreground, size sm, text "Sign Up".

Section 2 — Hero: A section with bg-background relative overflow-hidden. Contains the Navbar, then a flex flex-col items-center pt-20 px-4 div with:

Badge: A liquid-glass rounded-full pl-3 pr-1 py-1 flex items-center gap-2 mb-8 pill. Left text: "Nova+ Launched!" (text-foreground text-xs). Right: a nested rounded-full bg-white/5 px-3 py-0.5 span with "Explore" + a ChevronRight icon (w-3 h-3).
Heading: h1 with text-hero-heading text-center text-4xl sm:text-6xl lg:text-7xl font-semibold leading-[1.05] tracking-tight max-w-5xl. Text: "Accelerate Your" + br + "Revenue Growth Now".
Subheading: p with text-hero-sub text-center text-lg leading-8 max-w-md mt-4 opacity-80. Text: "Drive your funnel forward with clever workflows, analytics, and seamless lead management."
CTA Buttons: flex items-center gap-4 mt-8. Two buttons:
Primary: bg-primary text-primary-foreground rounded-full px-6 py-3 text-base font-medium — "Start Free Right Now"
Secondary: liquid-glass text-foreground rounded-full px-6 py-3 text-base font-normal hover:bg-white/5 — "Schedule a Consult"

Section 3 — Social Proof: A section with relative w-full overflow-hidden. Contains:

Background Video: <video> element: autoPlay muted playsInline, absolute inset-0 w-full h-full object-cover, initial style={{ opacity: 0 }}. Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260308_114720_3dabeb9e-2c39-4907-b747-bc3544e2d5b7.mp4
Gradient overlay: absolute inset-0 bg-gradient-to-b from-background via-transparent to-background.
Content: relative z-10 flex flex-col items-center pt-16 pb-24 px-4 gap-20:
Stats row: flex items-center gap-8 flex-wrap justify-center. Three items: Clock/"3-5 week turnround", DollarSign/"Upfront cost clarity", ShieldCheck/"Full refund assurance". Each: flex items-center gap-3 text-foreground/80 text-sm with icon in a liquid-glass w-8 h-8 rounded-lg container, icon w-4 h-4 text-foreground/70, label font-medium.
Spacer: div for video visibility.
Logo Marquee: w-full max-w-5xl. Layout: flex items-center gap-12. Left: text-foreground/50 text-sm paragraph "Relied on by brands / across the globe" (with br), whitespace-nowrap shrink-0. Right: relative overflow-hidden flex-1 div containing a flex animate-marquee gap-16 items-center div. Logos: Vortex, Nimbus, Prysma, Cirrus, Kynder, Halcyn — duplicated for seamless loop. Each logo: shrink-0 flex items-center gap-2 with a liquid-glass w-6 h-6 rounded-lg icon showing the first letter (text-xs font-bold text-foreground/70), and the name (text-base font-semibold whitespace-nowrap text-foreground).

Marquee Animation (Tailwind config):

keyframes: {
  marquee: {
    "0%": { transform: "translateX(0%)" },
    "100%": { transform: "translateX(-50%)" },
  },
},
animation: {
  marquee: "marquee 20s linear infinite",
},

Icons: All from lucide-react — ChevronRight, ChevronDown, Clock, DollarSign, ShieldCheck.

## AuraMail — SaaS [sites/auramail]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(56).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/auramail.webp

### Full Recreation Prompt — Aura Hero Section

Build a premium, Apple-inspired dark landing page hero for "Aura" — an AI email app — using **React 18 + TypeScript + Vite + Tailwind CSS + `motion/react` (Framer Motion) + `lucide-react`**.

### 1. Dependencies (package.json)

```json
{
  "dependencies": {
    "@supabase/supabase-js": "^2.57.4",
    "lucide-react": "^0.344.0",
    "motion": "^12.38.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  }
}
```

### 2. Global CSS (`src/index.css`)

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

html,
body {
  font-family: 'Inter', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
}

@layer utilities {
  .animate-shiny {
    animation: shiny 6s linear infinite;
  }
}

@keyframes shiny {
  0% {
    background-position: -200% center;
  }
  100% {
    background-position: 200% center;
  }
}
```

### 3. Tailwind config (`tailwind.config.js`)

```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: '#3D81E3',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
```

### 4. Imports (top of `App.tsx`)

```tsx
import { useState } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { Menu, X, ChevronRight, Search } from 'lucide-react';
```

### 5. SVG Assets — exact inline markup

### Apple Logo component

```tsx
function AppleLogo({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 384 512"
      className={className}
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M318.7 268.7c-.2-36.7 16.4-64.4 50-84.8-18.8-26.9-47.2-41.7-84.7-44.6-35.5-2.8-74.3 20.7-88.5 20.7-15 0-49.4-19.7-76.4-19.7C63.3 141.2 4 184.8 4 273.5q0 39.3 14.4 81.2c12.8 36.7 59 126.7 107.2 125.2 25.2-.6 43-17.9 75.8-17.9 31.8 0 48.3 17.9 76.4 17.9 48.6-.7 90.4-82.5 102.6-119.3-65.2-30.7-61.7-90-61.7-91.9zm-56.6-164.2c27.3-32.4 24.8-61.9 24-72.5-24.1 1.4-52 16.4-67.9 34.9-17.5 19.8-27.8 44.3-25.6 71.9 26.1 2 49.9-11.4 69.5-34.3z" />
    </svg>
  );
}
```

### Aura LogoMark (four-sparkle/concave-square mark)

```tsx
function LogoMark({ className = 'w-8 h-8' }: { className?: string }) {
  return (
    <svg viewBox="0 0 256 256" className={className} fill="none" aria-hidden="true">
      <path
        d="M 0 128 C 70.692 128 128 185.308 128 256 L 64 256 C 64 220.654 35.346 192 0 192 Z M 256 192 C 220.654 192 192 220.654 192 256 L 128 256 C 128 185.308 185.308 128 256 128 Z M 128 0 C 128 70.692 70.692 128 0 128 L 0 64 C 35.346 64 64 35.346 64 0 Z M 192 0 C 192 35.346 220.654 64 256 64 L 256 128 C 185.308 128 128 70.692 128 0 Z"
        fill="white"
      />
    </svg>
  );
}
```

### 6. Constants

```tsx
const navLinks = ['Solutions', 'Pricing', 'Blog', 'Documentation', 'Careers'];
const menuItems = ['File', 'Edit', 'View', 'Go', 'Window', 'Help'];

const gradientStyle: React.CSSProperties = {
  backgroundImage:
    'linear-gradient(110deg, #3D81E3, 20%, #AE9AE6, 40%, #F8D8D5, 60%, #FEEFDB, 80%, #3D81E3)',
  backgroundSize: '200% auto',
  WebkitBackgroundClip: 'text',
  backgroundClip: 'text',
  color: 'transparent',
  WebkitTextFillColor: 'transparent',
};
```

### 7. AppleButton component

```tsx
function AppleButton({
  label = 'Download Aura',
  full = false,
}: {
  label?: string;
  full?: boolean;
}) {
  return (
    <button
      className={`group inline-flex items-center justify-center gap-2 rounded-full bg-white text-black font-medium text-sm px-5 py-3 transition-all hover:bg-white/90 active:scale-[0.98] ${
        full ? 'w-full' : ''
      }`}
    >
      <AppleLogo className="w-4 h-4" />
      <span>{label}</span>
      <ChevronRight className="w-4 h-4 transition-transform duration-200 group-hover:translate-x-[1px]" />
    </button>
  );
}
```

### 8. Root Wrapper

```tsx
<div className="relative min-h-screen overflow-x-hidden bg-[#0c0c0c] text-white selection:bg-brand/30">
```

### 9. Background Video (no overlay)

```tsx
<div className="fixed inset-0 z-0 pointer-events-none">
  <video
    autoPlay
    loop
    muted
    playsInline
    className="w-full h-full object-cover pointer-events-none"
    src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260506_124911_3ed2d5b7-1604-4a4d-acbf-a28fd6c79348.mp4"
  />
</div>
```

### 10. Vertical Guide Lines (desktop only)

```tsx
<div className="hidden md:block pointer-events-none fixed inset-y-0 left-1/2 -translate-x-[calc(50%+36rem)] w-px bg-white/10 z-[5]" />
<div className="hidden md:block pointer-events-none fixed inset-y-0 left-1/2 translate-x-[calc(-50%+36rem)] w-px bg-white/10 z-[5]" />
```

### 11. Foreground Wrapper

```tsx
<div className="relative z-10">
  <div className="max-w-6xl mx-auto px-6">
```

### 12. Top Navigation

```tsx
<motion.nav
  initial={{ opacity: 0, y: -10 }}
  animate={{ opacity: 1, y: 0 }}
  transition={{ duration: 0.6, ease: 'easeOut' }}
  className="flex items-center justify-between py-5"
>
  <div className="flex items-center gap-2">
    <LogoMark />
    <span className="font-semibold tracking-tight text-white">Aura</span>
  </div>

  <ul className="hidden md:flex items-center gap-8">
    {navLinks.map((link, i) => (
      <motion.li
        key={link}
        initial={{ opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.1 + i * 0.05, ease: 'easeOut' }}
      >
        <a
          href="#"
          className="text-white/70 text-sm font-medium hover:text-white transition-colors"
        >
          {link}
        </a>
      </motion.li>
    ))}
  </ul>

  <div className="hidden md:block">
    <AppleButton />
  </div>

  <button
    onClick={() => setMenuOpen(true)}
    className="md:hidden w-10 h-10 inline-flex items-center justify-center rounded-full border border-white/10 bg-white/5"
    aria-label="Open menu"
  >
    <Menu className="w-5 h-5" />
  </button>
</motion.nav>
```

### 13. Hero Section

```tsx
<section className="pt-16 md:pt-28 pb-20 text-center flex flex-col items-center">
  <motion.h1
    initial={{ opacity: 0, y: 20 }}
    animate={{ opacity: 1, y: 0 }}
    transition={{ duration: 0.8, delay: 0.3, ease: [0.22, 1, 0.36, 1] }}
    className="text-4xl md:text-7xl font-semibold tracking-tight leading-[0.9]"
  >
    <span className="block text-white">Your email.</span>
    <span className="block animate-shiny" style={gradientStyle}>
      Revitalized
    </span>
  </motion.h1>

  <motion.p
    initial={{ opacity: 0, y: 20 }}
    animate={{ opacity: 1, y: 0 }}
    transition={{ duration: 0.8, delay: 0.5, ease: [0.22, 1, 0.36, 1] }}
    className="mt-8 text-white/60 max-w-md text-base leading-[1.5]"
  >
    Aura is the premier inbox platform for the current era. It leverages powerful
    AI to organize, prioritize, and refine your messages into total clarity.
  </motion.p>

  <motion.div
    initial={{ opacity: 0, y: 20 }}
    animate={{ opacity: 1, y: 0 }}
    transition={{ duration: 0.8, delay: 0.7, ease: [0.22, 1, 0.36, 1] }}
    className="mt-10 flex flex-col items-center gap-3"
  >
    <AppleButton />
    <p className="text-xs text-white/40">Download for Intel / Apple Silicon</p>
  </motion.div>
</section>
```

### 14. Mac Menu Bar (below hero, full-bleed)

```tsx
<motion.div
  initial={{ opacity: 0, y: 20 }}
  animate={{ opacity: 1, y: 0 }}
  transition={{ duration: 0.8, delay: 0.9, ease: [0.22, 1, 0.36, 1] }}
  className="h-10 bg-black/40 backdrop-blur-md border-t border-b border-white/10"
>
  <div className="max-w-6xl mx-auto px-6 h-full flex items-center justify-between text-xs">
    <div className="flex items-center gap-5">
      <AppleLogo className="w-3.5 h-3.5 text-white" />
      <span className="font-bold text-white">Aura</span>
      {menuItems.map((item, i) => (
        <span
          key={item}
          className={`text-white/80 ${i > 2 ? 'hidden sm:inline' : ''} ${
            i > 3 ? 'hidden md:inline' : ''
          }`}
        >
          {item}
        </span>
      ))}
    </div>
    <div className="flex items-center gap-3 text-white/80">
      <Search className="w-3.5 h-3.5" />
      <span>Wed May 6 1:09 PM</span>
    </div>
  </div>
</motion.div>
```

### 15. Mobile Drawer (AnimatePresence)

```tsx
<AnimatePresence>
  {menuOpen && (
    <motion.div
      initial={{ x: '100%' }}
      animate={{ x: 0 }}
      exit={{ x: '100%' }}
      transition={{ type: 'tween', duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
      className="fixed inset-0 z-[100] bg-black flex flex-col"
    >
      <div className="flex items-center justify-between px-6 py-5">
        <div className="flex items-center gap-2">
          <LogoMark />
          <span className="font-semibold">Aura</span>
        </div>
        <button
          onClick={() => setMenuOpen(false)}
          className="w-10 h-10 inline-flex items-center justify-center rounded-full border border-white/10 bg-white/5"
          aria-label="Close menu"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      <div className="flex-1 flex flex-col justify-center px-8 gap-6">
        {navLinks.map((link, i) => (
          <motion.a
            key={link}
            href="#"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.15 + i * 0.08 }}
            className="text-3xl font-semibold tracking-tight text-white"
          >
            {link}
          </motion.a>
        ))}
      </div>

      <div className="px-6 pb-10">
        <AppleButton full />
      </div>
    </motion.div>
  )}
</AnimatePresence>
```

### 16. Design rules

- **No overlay** on the video — text sits directly over it.
- **Color palette**: background `#0c0c0c`, brand `#3D81E3`, white text with `/60`, `/70`, `/40` opacity variants. No purple/indigo anywhere except as a transient stop inside the "Revitalized" text gradient.
- **Typography**: Inter only, weights 500/600/700 used. Headline is `font-semibold` (600), `tracking-tight`, `leading-[0.9]`.
- **Motion easing** for hero staggers: `[0.22, 1, 0.36, 1]` (cubic ease-out). Nav uses `'easeOut'`. Stagger delays: nav 0.1 + i*0.05; hero h1 0.3, p 0.5, CTA 0.7, menu bar 0.9 — all `duration: 0.8` except nav (0.6) and nav-link items (0.5).
- **Shiny gradient text**: 110° linear gradient blue → lilac → blush → cream → blue, `background-size: 200% auto`, animated via the `shiny` keyframe panning background-position from `-200%` to `200%` over 6s linear infinite, clipped to text.
- **Guide lines**: 1px white `/10` verticals at `±36rem` from center on md+.
- **Menu bar**: 40px tall, translucent black `/40`, backdrop-blur-md, bordered top and bottom with white `/10`.
- **Responsive**: hamburger on mobile, full drawer; menu items progressively hide past index 2 / 3 at smaller breakpoints.

## Bionova Biotech — SaaS [sites/bionova-hero]

- Preview: https://motionsites.ai/assets/hero-bionova-preview-Sk76d0_D.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bionova-hero.gif

Create a "BIONOVA" biotech consulting hero section that spans exactly 100vh on desktop (scrollable on mobile). Use Poppins font (imported from Google Fonts) as the heading font family. Install hls.js for video streaming.

Design system:

Background: white (hsl(0 0% 100%))

Foreground/text: dark gray (hsl(0 0% 17%))

Hero button color: soft blue (hsl(213 90% 78%))

Font: Poppins (400, 500, 600, 700 weights)

Custom animation fade-up: translateY(20px) + opacity 0 + blur(4px) → translateY(0) + opacity 1 + blur(0), 700ms with cubic-bezier(0.16, 1, 0.3, 1) easing, forwards fill

Navigation:

Logo text "BIONOVA" (bold, tracking-tight, xl) on the left

Center links (hidden on mobile): About, Offerings, Pricing, Blog — semibold, text-sm, 70% opacity, hover to full

Right side (hidden on mobile): "Log in" text link + "Request a call" rounded-full pill button in hero-btn blue with white text

Layout:

Full section: min-h-screen lg:h-screen, flex column, lg:overflow-hidden

Content area: px-5 lg:px-16, pb-8 lg:pb-[82px], flex-1

Two-column grid (lg:grid-cols-2), items-stretch, gap-8

Left column (flex col, justify-between, fade-up animation):

Top group:

H1 with text sizes: text-[2rem] sm:text-5xl lg:text-[3.5rem] xl:text-7xl, leading-[1.08], tracking-tight, font-normal

First line: small rounded-full image pill (w-20 h-10 / sm:w-24 sm:h-12, bg-cover) inline next to "World-class"

Second line: "consultants that"

Third line: "empower" followed by an inline pill button with Play icon + "How do we work" (border-2, rounded-full, smaller text on mobile)

Fourth line: "biotech leaders"

CTAs directly under headline (pt-6): "Contact us" blue pill button with ArrowUpRight icon + "Request a call" underlined text link

Bottom group (hidden on mobile, hidden lg:block):

Description paragraph (text-sm, max-w-md)

Logo bar: "Headway", "brightline", "hazel", "G&STC" — bold text-2xl, flex-wrap

Right column (flex col, gap-4, fade-up with 150ms delay):

Card 1 (top, larger): rounded-[1.5rem] lg:rounded-[2.5rem], bg-black, flex-1, min-h-[200px] on mobile

Background: autoplay muted looping HLS video — https://stream.mux.com/1RdbcBtpEUK6501pc6yaIvwo9UfSnOg02k1uHxat00xR3w.m3u8 — object-cover, full size

White heading (text-2xl lg:text-3xl): "If you're ready to build your bioventure, let's get in touch."

Bottom: description text (white/85) + white circle arrow button (bg-background)

Cards 2 & 3 (bottom row): grid-cols-2, gap-3 lg:gap-4, flex-1

Card 2: rounded-[1.5rem] lg:rounded-[2.5rem], bg-black, p-5 lg:p-8, min-h-[180px]

Background: HLS video https://stream.mux.com/t1TbTB8M1VYHkhxBuap4A8Vm1x015HTHyuQxqchDBago.m3u8 — scaled to 150%, centered with top-1/2 left-1/2 -translate-x/y-1/2

Top: "locations" white pill badge + arrow circle button (both bg-background)

Bottom: "United bio-entrepreneurs" heading (text-lg lg:text-2xl, white) + description (text-xs lg:text-sm, white/80)

Card 3: same card style as Card 2

Background: HLS video https://stream.mux.com/6yvj9SR5bjmXq9N3ak7gy427RwUs8R2ZoH4ndA7Q1018.m3u8 — scaled to 280%, centered same way

Top: "scientists" white pill badge

Bottom: large "34" number (text-4xl lg:text-7xl, white) + description (text-xs lg:text-sm, white/80)

HLS video implementation:

Use hls.js library with useRef for each video element

Loop through all 3 streams in a single useEffect

Handle native HLS (Safari) with canPlayType fallback

All videos: autoPlay, muted, loop, playsInline, preload="auto"

All card content uses relative z-10 to sit above the video

Responsive (mobile only):

Section becomes min-h-screen (scrollable) instead of fixed h-screen

Nav links and right-side buttons hidden below md

Smaller padding (px-5), smaller border radius (rounded-[1.5rem]), smaller card padding (p-5/p-6)

Bottom description/logo bar hidden below lg

Headline button smaller on mobile (px-4 py-1.5 text-sm)

Card text sizes scale down on mobile

## BookedUp — SaaS [sites/bookedup]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(58).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bookedup.webp

Create a modern Hero Section web application using React (TypeScript), Tailwind CSS v4, motion/react, and lucide-react for icons. Implement the exact layout, CSS variables, mock data, and components as described below.
1. Global CSS & Typography (index.css)
Configure Tailwind CSS and import two specific fonts:
Inter: https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap
SK Reykjavik Rounded Regular: https://db.onlinewebfonts.com/c/16e715573b2b3072037cf4ab26fc8bb8?family=SK+Reykjavik+Rounded+Regular
Define standard CSS configurations:
Make a @theme block defining --font-heading (SK Reykjavik) and --font-sans (Inter).
Add a body block in @layer base that applies: font-sans antialiased text-[#202020] bg-[#F7F7F7].
2. Mock Data & Types
Define the following TypeScript interfaces and constants at the top of your App.tsx:
An array of 4 Unsplash avatar URLs (AVATARS):
https://images.unsplash.com/photo-1534528741775-53994a69daeb?auto=format&fit=crop&q=80&w=100&h=100
https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?auto=format&fit=crop&q=80&w=100&h=100
https://images.unsplash.com/photo-1494790108377-be9c29b29330?auto=format&fit=crop&q=80&w=100&h=100
https://images.unsplash.com/photo-1500648767791-00dcc994a43e?auto=format&fit=crop&q=80&w=100&h=100
Types: CallItem (name, avatar, time, duration) and ClientCall (id, day, date, items: CallItem[]).
CLIENT_CALLS Array: Create an array of 5 call agenda items (e.g., MON 18 JUL, TUE 19 JUL, THU 25 JUL, MON 25 JUL, WED 20 JUL). Assign 1 or 2 consultation items per day using the avatar array and mock times (e.g., "9:30 AM - 10:30 AM", "1hr").
3. Custom Deep UI Components
Create the following custom React components exactly:
DeepShadowIcon: An inline div containing a background blur glowing layer (absolute opacity-60 bg-[#F25C40] blur-[18px] scale-90 translate-y-1.5) sitting underneath an inner bg-[#F25C40] box. Add exact shadows onto the top box: shadow-[0px_0px_5px_rgba(255,255,255,0.5)_inset,0px_8px_20px_rgba(242,92,64,0.35)]. Add p-2 rounded-2xl.
DeepShadowButton: A button wrapped in a group div containing an outer glowing shadow (absolute opacity-40 bg-[#212121] blur-[25px]) that animates scaling up slightly to scale-110 and translate-y-1 on hover. The button itself uses bg-[#202020] and inner shadows: shadow-[0px_0px_4px_rgba(255,255,255,0.25)_inset,0px_6px_19px_rgba(0,0,0,0.25)]. Text inside is white.
DeepShadowAvatar: Avatar image stacked on a blur shadow (bg-black blur-[12px] opacity-20). Provide a hasGlow prop; if true, the background glow is orange (bg-[#F25C40] opacity-60 blur-[18px]). Accept size props for sm, md, lg.
AvatarWithShadow: Small functional image component wrapped in a div with an absolute bottom backdrop of bg-black opacity-25 blur-[10px].
ClientCard: Accepts index and a single ClientCall. Wraps content in a motion.div that fades & scales up in on load. Rotates the card alternately rotate-3 or -rotate-3 depending on if the index is even/odd. Background applies bg-white and an outer shadow shadow-[0_4px_20px_rgba(0,0,0,0.03)]. Card header displays Day on left, Date on right. Iterates over call.items dividing the avatar, names, and limits with dotted bottom borders.
4. The Hero Layout (App.tsx > App)
Construct the main app structure over a bg-[#F7F7F7] global page:
Use a main grid container: max-w-[1440px] lg:grid-cols-[1.2fr_0.8fr] gap-12.
Left Side (Value Prop):
Pulse Badge: A small pill with rounded borders border-[#D1F2D1]. Inside is a green pulsing dot bg-[#52D352] animate-pulse shadow-[0_0_8px_rgba(82,211,82,0.6)] and uppercase text "BOOKING FOR SUMMER" in tracking wide #52D352.
Main Headline: h1 using the font-heading style. Text #202020, negative letter spacing (tracking-tight / tracking-[-0.02em]). Text says: "Expanding" [insert inline DeepShadowIcon here wrapping a white lucide TrendingUp icon rotated -rotate-6] "reach <br/> with every lead".
Subheadline: Paragraph using text-neutral-500. Text: "Automating lead systems and funnels, we design scalable growth engines for your next venture."
CTA Row: A layout using gap-4 sm:gap-6. Uses your DeepShadowButton with "Scale revenue now", sitting next to a standard white button (with border, grey text, and a Lucide Play icon) saying "Start Here".
Social Proof Section: Side-by-side layout separated by a .w-px dividing line on desktop.
Left half: "VERIFIED CLIENTS" over a row of 4 DeepShadowAvatar elements. Make the 2nd one uniquely use hasGlow={true}. Make sizes alternate between md and lg.
Right half: "TOP TIER QUALITY 5/5" over a row of 5 Lucide Star icons styled with text-[#FFB648] fill-[#FFB648].
Right Side (The Infinite Client Marquee):
Wrap the column in a container with relative overflow-hidden lg:h-full (height 600px on mobile).
Add top and bottom fade masks: absolute divs with bg-gradient-to-b and bg-gradient-to-t (from-[#F7F7F7] to-transparent) spanning h-20 sm:h-40 for scroll falloff.
Use a dual-column vertical scrolling tracks configuration (grid grid-cols-1 md:grid-cols-2 lg:grid-cols-1).
Framer Motion Tracks: Apply motion.div scrolling tracks wrapped in an infinite repeating frame moving its Y axis: animate={{ y: [0, -1200] }} transition={{ repeat: Infinity, duration: 40, ease: "linear" }}.
Provide continuous looping visual depth by iterating through [...CLIENT_CALLS, ...CLIENT_CALLS, ...CLIENT_CALLS] printing out copies of the ClientCard components onto the tracks. Create a hidden second track for tablets using animate={{ y: [-600, -1800] }}.

## CoderCrest — SaaS [sites/codercrest-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/codercrest-hero-CoycO52t.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/codercrest-hero.gif

Create a React + TypeScript component named HeroSection in src/components/HeroSection.tsx using Tailwind CSS and the hls.js npm package (install it: npm install hls.js).

Layout & Background:

A <section> that is 100vh tall, position: relative, overflow: hidden, flex column centered, with background: #000.
A fullscreen HLS video background using this Mux stream URL: 
https://stream.mux.com/tLkHO1qZoaaQOUeVWo8hEBeGQfySP02EPS02BmnNFyXys.m3u8

The video is <video autoPlay loop muted playsInline> with classes absolute inset-0 w-full h-full object-cover and zIndex: 0. Play it through hls.js: if Hls.isSupported(), create an Hls({ autoStartLoad: true }) instance, loadSource, attachMedia, and play on MANIFEST_PARSED. Else, fall back to native application/vnd.apple.mpegurl support. Clean up the Hls instance on unmount. No overlay over the video — full opacity.
Content container:

A div with classes relative z-10 flex flex-col items-center text-center px-4 max-w-5xl mx-auto and inline style marginTop: 380 (pushes content down 380px).
Headline (<h1>):

Font: 'YDYoonche L', 'YDYoonche M', sans-serif
fontSize: clamp(2.2rem, 7vw, 6.5rem), color: #fff, fontWeight: 300, letterSpacing: -0.01em, lineHeight: 1.1, className="leading-tight".
Three lines:
"The vision" — gradient text using background: linear-gradient(90deg, #666666 0%, #d0d0d0 50%, #666666 100%) with WebkitBackgroundClip: text, WebkitTextFillColor: transparent, backgroundClip: text, display: block, lineHeight: 1.1, marginBottom: -0.22em.
"of engineering" — same gradient styling as line 1.
A flex line flex items-center justify-center gap-3 flex-wrap with white text containing in order:
<span style={{color:'#999'}}>is</span>
A circular video icon (see below) playing the human clip
<span>human</span>
<span style={{color:'#999', position:'relative', top:'0.15em', marginLeft:'0.25em'}}>+</span>
A circular video icon playing the AI clip
<span>AI</span>
VideoIcon component:

Outer <span> with classes inline-block align-middle rounded-full overflow-hidden, sized via inline style width/height: clamp(48px, 10vw, ${size}px) (default size=72, but the hero passes size={110} for both icons), flexShrink: 0.
Inner <video autoPlay loop muted playsInline> with width: 100%, height: 100%, objectFit: cover, display: block. Call videoRef.current.play().catch(() => {}) in a useEffect.
Two CloudFront MP4 sources:
VIDEO_HUMAN: 
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260424_090051_64ea5059-da6b-492b-a171-aa7ecc767dc3.mp4

VIDEO_AI: 
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260424_093237_ff0ddc63-c068-4e29-96da-fdd0e40af133.mp4

Subheading (<p>):

Classes mt-4 max-w-xl text-center px-2.
fontSize: clamp(0.95rem, 2.2vw, 1.2rem), color: #ccc, lineHeight: 1.4, fontWeight: 400.
Text: "We help you map the talent you need, track the talent you have, and close your gaps to thrive in a GenAI world."
CTA Button:

Classes: mt-6 transition-all duration-300 hover:scale-[1.03] hover:shadow-[0px_6px_32px_8px_rgba(39,243,169,0.22)] active:scale-[0.98]
Inline style: padding: '12px 28px', background: '#000', boxShadow: '0px 6px 24px 6px rgba(39, 243, 169, 0.15)', borderRadius: 8, outline: '1px solid #30463C', outlineOffset: -1, border: 'none', cursor: 'pointer', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', gap: 10.
Inner <span> with color: '#fff', fontSize: 14, fontWeight: 400, text: "Join The Movement!".
Animations / interactions:

All three videos auto-play, loop, muted, inline.
Button has a 300ms transition: scales to 1.03 and gains a brighter green glow on hover, scales to 0.98 on active.
Fonts:

The headline expects 'YDYoonche L' / 'YDYoonche M' to be loaded globally (e.g., via index.css or an external font provider). It falls back to sans-serif.

## SAAS Software — SaaS [sites/convix-software-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/convix-software-hero-B6-tdnN6.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/convix-software-hero.gif

Build a fully responsive, full-viewport hero section for a PR-agency SaaS called "Convix Software" with these exact specs:

Page Frame
Outer wrapper: min-h-screen w-full bg-[#ededed] p-3 sm:p-4, font-family Inter
Hero container (clips everything inside): relative w-full h-[calc(100vh-24px)] sm:h-[calc(100vh-32px)] overflow-hidden bg-[#d9d9d9] rounded-2xl sm:rounded-3xl
Background Video
URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260424_064411_9e9d7f84-9277-41f4-ab10-59172d89e6be.mp4
Absolutely positioned, inset-0 w-full h-full object-cover pointer-events-none
Attributes: autoPlay, loop, muted, playsInline, preload="auto", disableRemotePlayback, webkit-playsinline="true", x5-playsinline="true"
Poster fallback: https://images.unsplash.com/photo-1557683316-973673baf926?w=1600&q=60
Above the video: absolute inset-0 bg-white/10 overlay
Foreground content wrapper: relative z-10
Fonts (/src/styles/fonts.css)
Import from Google Fonts:

Inter weights 400, 500, 600, 700
Instrument Serif regular + italic
Navbar (floating pill, responsive with hamburger)
Wrapper: flex justify-center pt-4 sm:pt-6 px-3 sm:px-4
Pill: bg-white rounded-full shadow-sm border border-neutral-200 pl-2 pr-2 py-2 w-full max-w-[760px] relative
Logo (left, shrink-0): orange #ef4d23 8-petal flower SVG — 8 circles at radius 10 around center (16,16) plus center circle, all r=3.5, viewBox 32×32, rendered w-7 h-7 sm:w-8 sm:h-8
Desktop links (hidden md:flex, gap-6, 14px): "Home" (with 1.5px black dot), "Features", "About", "Pages" (#ef4d23 + ChevronDown 3.5)
Right cluster (ml-auto): ShoppingCart icon (hidden on mobile), then orange #ef4d23 rounded-full button "Get early access" (desktop) / "Early access" (mobile) with white/20 inner circle holding ChevronRight
Mobile-only Menu (lucide) hamburger button (md:hidden)
When open: dropdown panel absolute top-full left-2 right-2 mt-2 bg-white rounded-2xl shadow-lg border border-neutral-200 p-3 z-20 listing the same nav items vertically
useState open toggles the menu
Hero Content (centered)
flex flex-col items-center px-4 pt-10 sm:pt-16 pb-8 sm:pb-12 text-center
Badge: inline-flex items-center gap-2 bg-white rounded-full px-4 py-1.5 shadow-sm, 13px — orange dot + "Convix Software"
Headline <h1> with inline style fontSize: clamp(36px, 8vw, 72px); lineHeight: 1.05; fontWeight: 500; letterSpacing: -0.02em, mt-5 sm:mt-6 max-w-4xl:
"Shaping " + <span style={{fontFamily:"'Instrument Serif', serif", fontStyle:"italic", fontWeight:400}}>Agencies</span> + <br> + "of tomorrow"
Subtitle <p> mt-4 sm:mt-6 text-neutral-700 px-2, fontSize: clamp(13px, 3.5vw, 16px): "The All-In-One Software Powering the Future of PR Agencies"
CTA button mt-6 sm:mt-8 inline-flex items-center gap-3 bg-[#0b0f1a] text-white rounded-full pl-6 sm:pl-7 pr-2 py-2 sm:py-2.5, 14px: "Get Started" + w-6 h-6 sm:w-7 sm:h-7 rounded-full bg-white/15 containing ChevronRight (4×4)
Dashboard Preview
Wrapper: bg-[#f5f2ee] rounded-3xl p-4 sm:p-6 w-full max-w-[880px] mx-auto
Grid: grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4
Outer container around it: px-3 sm:px-4
Card 1 — Clicks (white, rounded-2xl, p-5)
Header: orange "Clicks" + neutral "This Month" (13px)
Big number "6,896" (28px, weight 600) + red pill bg-red-50 text-red-600 rounded-full px-2 py-0.5 with TrendingDown icon "-3,382 (33%)" (11px)
Small caption "Compared to yesterday"
Centered "Month Target achieved" label
Gauge at 92% in #ef4d23, with end labels "389K" / "425K"
Toggle pill bottom: bg-neutral-100 rounded-full p-1 flex — "Impressions" active (white card + shadow) / "Clicks" inactive
Card 2 — Form (white, rounded-2xl, p-5, flex flex-col gap-3)
Two label+dropdown groups (label 12px neutral-700, button bordered rounded-lg px-3 py-2 with ChevronDown):
"Show figures for" → "This month"
"Compare period by" → "Month-to-date (MTD)"
Two label+input groups with # prefix:
"Ste targets (This month)" → 10
"Ste targets (This year)" → 100
Footer: orange #ef4d23 "Save" button (rounded-lg px-5 py-2), underlined "Cancel", X icon pushed to right (ml-auto)
Card 3 — Video Starts (white, rounded-2xl, p-5)
Header: orange "Video Starts" + "today"
Big "0" + neutral pill with TrendingUp + "0"
"Compared to yesterday"
Gauge at 68% in #9ca3af (no end labels)
Toggle pill: "Video Clicks" active / "Video Starts"
Gauge Component (reusable)
Props: value, color="#ef4d23", showLabels, min, max
SVG viewBox 0 0 200 120, max-width 260px
40 tick marks spanning a 180° arc (start at angle π, sweep to 2π); active count = round(value/100 * 40)
Each tick: <line> from radius (r-10) to r=80 around center (100,100), strokeWidth=2.5, strokeLinecap="round", active uses color, inactive #d4d4d8
Center text: <text x=100 y=105 textAnchor="middle">{value}%</text>, fontSize 22, fontWeight 600
If showLabels: small flex row below SVG, 11px neutral-500, justify-between, showing min and max
Colors
Primary orange: #ef4d23
Dark CTA: #0b0f1a
Page bg: #ededed; hero bg: #d9d9d9; dashboard tray: #f5f2ee
Icons (lucide-react)
ChevronDown, ChevronRight, ShoppingCart, Menu, TrendingDown, TrendingUp, X

File Structure
src/app/App.tsx
src/app/components/Navbar.tsx
src/app/components/DashboardPreview.tsx
src/app/components/Gauge.tsx
src/styles/fonts.css
Behavior
No custom animations; only the native looping muted background video
Entire hero (video + content + dashboard) is clipped together by the rounded container, so the dashboard cards bleed off the bottom edge
Fully responsive: navbar collapses to hamburger under md, headline/CTA scale via clamp(), dashboard grid steps from 1 → 2 → 3 columns

## Datacore Booking — SaaS [sites/datacore-booking-hero]

- Preview: https://motionsites.ai/assets/hero-datacore-booking-preview-B3t9SRK6.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/datacore-booking-hero.gif

Create a responsive, full-screen hero section for a web application using React and Tailwind CSS.

Design System & Assets:

Fonts: Load and use 'Manrope' (for UI/Nav), 'Cabin' (for buttons/tags), 'Instrument Serif' (for headlines), and 'Inter' (for body text).

Primary Color: Purple #7b39fc
Secondary/Dark Color: Dark Purple #2b2344
Background: Use a full-screen, absolute-positioned HTML5 video background. The video should autoplay, loop, mute, and play inline. Ensure it covers the viewport (min-h-screen, object-cover) without an overlay (keep it opaque).

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260210_031346_d87182fb-b0af-4273-84d1-c6fd17d6bf0f.mp4

1. Navbar Component (Top Overlay)

Layout: Full width, transparent background, z-20 relative positioning.
Padding: px-6 (mobile) to px-[120px] (desktop), py-[16px].

Logo (Left): Use this specific SVG path filled with white:
Path: M1.04356 6.35771L13.6437 0.666504... (Future logo shape).

Navigation Links (Center-Left, Desktop Only):
Items: "Home", "Services" (with a ChevronDown icon), "Reviews", "Contact us".
Style: Manrope font, Medium weight, 14px size, White.
Hover effect: opacity-80.

Action Buttons (Right, Desktop Only):
Sign In: White background, thin gray border (#d4d4d4), rounded 8px, Black text (#171717), Manrope Semibold 14px.
Get Started: Primary Purple background (#7b39fc), rounded 8px, White text (#fafafa), Manrope Semibold 14px, subtle shadow.

Mobile: Hide links/buttons and show a White Menu icon (hamburger) that toggles a full-screen black overlay menu.

2. Hero Content (Centered)

Container: Centered vertically and horizontally (flex-col items-center text-center), z-10 relative, top margin mt-32.

Tagline Pill:
Style: Glassmorphism effect (bg-[rgba(85,80,110,0.4)], backdrop-blur, border rgba(164,132,215,0.5)).
Shape: Rounded 10px, Height 38px.
Content: A small inner badge (#7b39fc bg, rounded 6px) saying "New" followed by text "Say Hello to Datacore v3.2".
Font: Cabin, Medium, 14px, White.

Headline:
Text: "Book your perfect stay instantly and hassle-free".
Typography: Instrument Serif font, White.
Size: 5xl (mobile) to 96px (desktop).
Styling: Line-height 1.1. The word "and" should be italicized with specific spacing.

Subtext:
Text: "Discover handpicked hotels, resorts, and stays across your favorite destinations. Enjoy exclusive deals, fast booking, and 24/7 support."
Typography: Inter font, Normal weight, 18px.
Color: White with 70% opacity (text-white/70).
Width: Max width 662px.

Call to Action Buttons (Row):
Button 1: "Book a Free Demo" — Primary Purple (#7b39fc), rounded 10px, Cabin Medium 16px, White.
Button 2: "Get Started Now" — Dark Purple (#2b2344), rounded 10px, Cabin Medium 16px, Off-white (#f6f7f9).
Hover effects: Slightly lighten backgrounds on hover.

## Digitwist AI Builder — SaaS [sites/digitwist-hero]

- Preview: https://motionsites.ai/assets/hero-digitwist-preview-s2pJetjQ.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/digitwist-hero.gif

Create a dark mode hero section for an AI website builder with the following exact specifications:

### Technical Setup

### Required Packages
Install these packages:
- `motion` (version 12.23.24 or later) - for animations
- `hls.js` (version 1.6.15 or later) - for video streaming
- `lucide-react` (version 0.487.0 or later) - for icons

### Fonts
Import these Google Fonts:
```css
@import url('https://fonts.googleapis.com/css2?family=Instrument+Sans:ital,wght@0,400..700;1,400..700&family=Instrument+Serif:ital@0;1&display=swap');
```

### Layout Structure

### Navbar Component
Create a fixed, transparent navbar with:

**Position & Styling:**
- Fixed to top, full width, z-index 50
- Background: fully transparent (bg-transparent)
- Padding: px-6 py-4
- Flexbox layout: items-center justify-between

**Left Section:**
- Sunburst icon (24x24px SVG) in white color

**Center Section** (hidden on mobile, visible md:flex):
- Navigation links: "Products" (with ChevronDown icon), "Customer Stories", "Resources", "Pricing"
- Font: Instrument Sans, text-sm, font-medium
- Color: text-white/80, hover:text-white
- Gap: gap-8

**Right Section:**
- "Book A Demo" link (hidden on small screens, sm:block)
- "Get Started" button: white background, black text, rounded-full, px-5 py-2.5, font-semibold

### Hero Section Component

**Container:**
- Relative positioning, full width, min-h-screen
- Background color: #000000 (pure black)
- Text color: white
- Overflow hidden

**Background Video Layer:**
- Video URL: https://stream.mux.com/T6oQJQ02cQ6N01TR6iHwZkKFkbepS34dkkIc9iukgy400g.m3u8
- Video implementation using HLS.js with Safari fallback
- Video properties: muted, loop, playsInline
- Object-fit: cover, opacity: 60%
- Poster image fallback: https://images.unsplash.com/photo-1647356191320-d7a1f80ca777?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w3Nzg4Nzd8MHwxfHNlYXJjaHwxfHxhYnN0cmFjdCUyMGRhcmslMjB0ZWNobm9sb2d5JTIwbmV1cmFsJTIwbmV0d29ya3xlbnwxfHx8fDE3Njg5NzIyNTV8MA&ixlib=rb-4.1.0&q=80&w=1080

**Video Overlay:**
- Black overlay: bg-black/60 with backdrop-blur-[2px]

**Decorative Gradients:**
- Top-left gradient: position top-[-20%] left-[20%], size 600x600px, bg-blue-900/20, blur-[120px], mix-blend-screen
- Bottom-right gradient: position bottom-[-10%] right-[20%], size 500x500px, bg-indigo-900/20, blur-[120px], mix-blend-screen

**Content Container:**
- Max-width: 5xl (max-w-5xl)
- Center aligned (mx-auto, items-center, text-center)
- Z-index: 10, top margin: mt-20
- Vertical spacing: space-y-12

**Pre-headline:**
- Text: "Design at the speed of thought"
- Font: Instrument Serif
- Size: text-3xl (mobile), sm:text-5xl, lg:text-[48px]
- Line height: leading-[1.1]
- Color: white
- Animation: Motion fade up (opacity 0→1, y 20→0, duration 0.6s)

**Main Headline:**
- Text: "Build Faster"
- Font: Instrument Sans, font-semibold
- Size: text-6xl (mobile), sm:text-8xl, lg:text-[136px]
- Line height: leading-[0.9], letter spacing: tracking-tighter
- Gradient: bg-gradient-to-b from-white via-white to-[#b4c0ff]
- Text effect: bg-clip-text text-transparent
- Animation: Motion scale (opacity 0→1, scale 0.9→1, delay 0.2s, duration 0.6s)

**Subheadline:**
- Text: "Create fully functional, SEO-optimized websites in seconds with our advanced AI engine."
- Font: Instrument Sans
- Size: text-lg (mobile), sm:text-[20px]
- Line height: leading-[1.65]
- Color: white, opacity-70
- Max width: max-w-xl
- Animation: Motion fade (opacity 0→0.7, delay 0.4s, duration 0.6s)

**CTA Buttons:**

Primary Button:
- Style: White pill-shaped with blue arrow
- Layout: pl-6 pr-2 py-2, rounded-full
- Background: white
- Text: "Start Building Free" (font-medium, text-lg, Instrument Sans, color #0a0400)
- Arrow container: 40x40px circle, bg-[#3054ff], hover:bg-[#2040e0]
- Icon: ArrowRight (lucide-react), white, 20x20px
- Hover effect: shadow-[0_0_20px_rgba(255,255,255,0.3)], scale-105

Secondary Button:
- Text: "See Examples"
- Style: text link with arrow
- Color: text-white/70, hover:text-white
- Background: backdrop-blur-sm, hover:bg-white/5
- Padding: px-4 py-2, rounded-lg
- Icon: ArrowRight with group-hover:translate-x-1 transition

Button Container:
- Layout: flex-col (mobile), sm:flex-row
- Gap: gap-6, items centered
- Animation: Motion fade up (opacity 0→1, y 20→0, delay 0.6s, duration 0.5s)

### HLS.js Video Implementation
```tsx
import { useEffect, useRef } from "react";
import Hls from "hls.js";

const videoRef = useRef<HTMLVideoElement>(null);
const videoSrc = "https://stream.mux.com/T6oQJQ02cQ6N01TR6iHwZkKFkbepS34dkkIc9iukgy400g.m3u8";

useEffect(() => {
  const video = videoRef.current;
  if (!video) return;

  if (Hls.isSupported()) {
    const hls = new Hls();
    hls.loadSource(videoSrc);
    hls.attachMedia(video);
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      video.play().catch((e) => console.log("Auto-play prevented:", e));
    });
    return () => {
      hls.destroy();
    };
  } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
    video.src = videoSrc;
    video.addEventListener("loadedmetadata", () => {
      video.play().catch((e) => console.log("Auto-play prevented:", e));
    });
  }
}, []);
```

### Motion Animations
Import: `import { motion } from "motion/react"`

- Pre-headline: initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.6 }}
- Main headline: initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }} transition={{ delay: 0.2, duration: 0.6 }}
- Subheadline: initial={{ opacity: 0 }} animate={{ opacity: 0.7 }} transition={{ delay: 0.4, duration: 0.6 }}
- Buttons: initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.6, duration: 0.5 }}

### Color Palette
- Background: #000000
- Primary text: white
- Secondary text: white/80, white/70
- Primary button background: white
- Primary button text: #0a0400
- Primary button accent: #3054ff, hover #2040e0
- Gradient end color: #b4c0ff
- Decorative gradients: blue-900/20, indigo-900/20

## Finlytic AI Agent — SaaS [sites/finlytic-hero]

- Preview: https://motionsites.ai/assets/hero-finlytic-preview-CV9g0FHP.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/finlytic-hero.gif

Build a hero section with the following exact specifications:

Overall Layout:

Full-width section with background: #000000 (pure black)
Overflow hidden
Background video playing behind all content (details below)

Background Video:

Source: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260215_121759_424f8e9c-d8bd-4974-9567-52709dfb6842.mp4
Autoplay, loop, muted, playsInline
Scaled to 120% of the container (width and height both 120%)
Horizontally centered, focal point anchored to the bottom
Sits behind all content (lowest z-index)

Blurred Background Element:

Absolute positioned, horizontally centered, top offset ~215px
Size: 801px wide × 384px tall, fully rounded (pill shape)
Color: pure black #000000
Blur: 77.5px
z-index: 1 (above video, below content)

All text and UI content sits at z-index: 2 (above everything)

Navbar (top):

Max width: 1440px, centered horizontally
Horizontal padding: 120px, vertical padding: 16px, height: 102px
Flexbox row, space-between alignment

Left side: Logo + nav links with 80px gap between them
Logo: "LOGOIPSUM" SVG mark, 134px × 25px, white fill
Nav links in a row with 10px gap between items
Each link: font Manrope, medium weight, 14px size, 22px line-height, white color, padding 10px horizontal / 4px vertical
Items: "Home", "Services" (with a 24×24 white chevron-down icon to the right, 3px gap), "Reviews", "Contact us"

Right side: Two buttons with 12px gap
"Sign In" button: white background, 16px horizontal / 8px vertical padding, 8px border-radius, Manrope semibold 14px/22px, color #171717, with a 1px #d4d4d4 border overlay
"Get Started" button: background #7b39fc (purple), 16px/8px padding, 8px border-radius, Manrope semibold 14px/22px, color #fafafa, subtle box-shadow 0px 4px 16px rgba(23,23,23,0.04)

Hero Content (centered below navbar):

Flex column, centered, max-width 871px, top margin 162px
24px gap between heading block and buttons

Heading block: flex column, 10px gap, center-aligned text
Line 1: "Automate repetitive." — font Inter, medium weight, 76px, white, letter-spacing -2px, line-height 1.15
Line 2: "Focus on growth." — font Instrument Serif, italic, 76px, white, letter-spacing -2px, line-height 1.15
Subtitle: "The next-generation AI agent platform that handles lead generation, customer support, and data entry while you build." — font Manrope, regular weight, 18px, 26px line-height, color #f6f7f9, opacity 90%, max-width 613px

CTA Buttons: flex row, 22px gap, vertically centered
"Get Started Free": background #7b39fc, padding 24px horizontal / 14px vertical, 10px border-radius, font Cabin medium 16px, line-height 1.7, white text
"Watch 2min Demo": background #2b2344 (dark purple), same padding/radius/font specs, color #f6f7f9

Dashboard Image (below hero content):

Centered, top margin 80px, bottom padding 40px
Outer container: 1163px wide (max 90% of viewport), 24px border-radius, backdrop-blur 10px, background rgba(255,255,255,0.05) (glassmorphic), transparent border 1.5px
Inner padding: 22.5px all sides
Image inside: full width, auto height, 8px border-radius, object-fit cover

## Grow AI Talent Platform — SaaS [sites/grow-ai-hero]

- Preview: https://motionsites.ai/assets/hero-grow-ai-preview-BlQ8tAQ-.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/grow-ai-hero.gif

Build a dark-themed landing page hero section with a navbar, headline, CTA button, background video with fade-in/out loop, and a logo marquee. Use React + Vite + Tailwind CSS + TypeScript with shadcn/ui. Install @fontsource/geist-sans.

1. Theme & Design Tokens (index.css)
Set up a single dark theme (no light mode toggle). All colors in HSL:
:root {
  --background: 260 87% 3%;
  --foreground: 40 6% 95%;
  --card: 240 6% 9%;
  --card-foreground: 40 6% 95%;
  --popover: 240 6% 9%;
  --popover-foreground: 40 6% 95%;
  --primary: 262 83% 58%;
  --primary-foreground: 0 0% 100%;
  --secondary: 240 4% 16%;
  --secondary-foreground: 40 6% 95%;
  --muted: 240 4% 16%;
  --muted-foreground: 240 5% 65%;
  --accent: 262 83% 58%;
  --accent-foreground: 0 0% 100%;
  --destructive: 0 84.2% 60.2%;
  --destructive-foreground: 0 0% 100%;
  --border: 240 4% 20%;
  --input: 240 4% 20%;
  --ring: 262 83% 58%;
  --radius: 0.75rem;
  --hero-heading: 40 10% 96%;
  --hero-sub: 40 6% 82%;
}

Body font: 'Geist Sans', 'Inter', system-ui, sans-serif
Import these font weights:
@import "@fontsource/geist-sans/400.css";
@import "@fontsource/geist-sans/500.css";
@import "@fontsource/geist-sans/600.css";
@import "@fontsource/geist-sans/700.css";

2. Liquid Glass Utility (index.css)
Add a .liquid-glass utility class in @layer utilities:
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

3. Tailwind Config
Add these to tailwind.config.ts:
All the semantic color tokens mapped to hsl(var(--token))
A hero color group: hero.heading and hero.sub
A marquee keyframe: 0% { transform: translateX(0%) } → 100% { transform: translateX(-50%) }
Animation: marquee: "marquee 20s linear infinite"

4. Button Variants
In the shadcn button.tsx, add two custom variants:
hero: "bg-primary text-primary-foreground rounded-full px-6 py-3 text-base font-medium hover:bg-primary/90"
heroSecondary: "liquid-glass text-foreground rounded-full px-6 py-3 text-base font-normal hover:bg-white/5"

5. Navbar Component
Full-width, py-5 px-8, flex row, justify-between
Left: A logo image (32px height). Use a logo.png from src/assets/logo.png
Center: Nav items as plain buttons: "Features" (with ChevronDown icon), "Solutions", "Plans", "Learning" (with ChevronDown icon). Text is text-foreground/90 text-base, gap-1 between items
Right: "Sign Up" button using heroSecondary variant, size="sm", rounded-full px-4 py-2
Below the navbar, add a full-width 1px gradient divider: mt-[3px] w-full h-px bg-gradient-to-r from-transparent via-foreground/20 to-transparent

6. Hero Section
Section with bg-background relative overflow-hidden
Contains the Navbar at the top
Below navbar + divider, centered content with pt-20 px-4
Headline "Grow": text-[230px] font-normal leading-[1.02] tracking-[-0.024em], font-family 'General Sans', sans-serif, bg-clip-text text-transparent with background-image: linear-gradient(223deg, #E8E8E9 0%, #3A7BBF 104.15%)
Subtext: text-hero-sub text-center text-lg leading-8 max-w-md mt-4 opacity-80, two lines: "The most powerful AI ever deployed" / "in talent acquisition" (split with <br/>)
CTA Button: heroSecondary variant, text "Schedule a Consult", px-[29px] py-[24px], wrapped in a div with mt-8 mb-[66px]

7. Social Proof / Video Section
Immediately below the hero, a separate <section> with relative w-full overflow-hidden.
Background Video: <video> element: autoPlay muted playsInline, absolute inset-0 w-full h-full object-cover, initial style={{ opacity: 0 }}
Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260308_114720_3dabeb9e-2c39-4907-b747-bc3544e2d5b7.mp4
Fade logic (JavaScript): Use requestAnimationFrame to continuously read currentTime and duration. Fade in over 0.5s at the start, fade out over 0.5s at the end. On ended, set opacity to 0, wait 100ms, reset currentTime = 0, and play() again. This creates a seamless manual loop with fade transitions.
Gradient overlays: absolute inset-0 bg-gradient-to-b from-background via-transparent to-background
Content (z-10): flex flex-col items-center pt-16 pb-24 px-4 gap-20
A h-40 spacer div for video visibility

Logo Marquee at max-w-5xl:
Left side: text "Relied on by brands / across the globe" in text-foreground/50 text-sm, with <br/>, whitespace-nowrap shrink-0
Right side: horizontally scrolling marquee using animate-marquee (the 20s infinite animation)
Logos are placeholder brands: Vortex, Nimbus, Prysma, Cirrus, Kynder, Halcyn — duplicated for seamless loop
Each logo: a small liquid-glass w-6 h-6 rounded-lg square with the first letter, plus the brand name in text-base font-semibold text-foreground
Gap between logos: gap-16

8. Page Composition
The Index page simply renders <HeroSection /> then <SocialProofSection /> sequentially with no wrapper styling.

## Mindloop — SaaS [sites/mindloop-hero]

- Preview: https://motionsites.ai/assets/hero-mindloop-preview-BR8xW6xW.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/mindloop-hero.gif

Create a full-screen hero section with a background video, navbar, and centered content. Use a dark theme with all white text.

Background Video:

Full-screen <video> element with autoPlay loop muted playsInline
Positioned absolute inset-0 w-full h-full object-cover z-0
Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_151826_c7218672-6e92-402c-9e45-f1e0f454bdc4.mp4
Font:

Import Google Font: Instrument Serif (display) and Inter (body)
All headings use font-family: 'Instrument Serif', serif
Body text uses Inter, sans-serif
Navbar (relative z-10):

Flex row, justify-between, px-8 py-6, max-w-7xl mx-auto
Left: Brand name "Velorah®" — text-3xl tracking-tight, white, Instrument Serif. The ® is wrapped in <sup className="text-xs">
Center: Hidden on mobile (hidden md:flex), links: Home, Studio, About, Journal, Reach Us — text-sm text-white, gap-10, hover:opacity-80 transition-opacity
Right: "Begin Journey" button with liquid-glass effect, rounded-full px-6 py-2.5 text-sm, hover:scale-[1.03]
Hero Content (relative z-10):

Flex column, centered (items-center justify-center text-center), px-6 pt-32 pb-40
H1: "Focus in a Distracted World" — text-5xl sm:text-7xl md:text-8xl, leading-[0.95], tracking-[-2.46px], max-w-7xl, white, Instrument Serif, animate-fade-rise
Paragraph: "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work." — text-base sm:text-lg, max-w-2xl mt-8 leading-relaxed, white, animate-fade-rise-delay
CTA Button: "Begin Journey" — liquid-glass, rounded-full px-14 py-5 text-base, white, mt-12, hover:scale-[1.03], animate-fade-rise-delay-2
Liquid Glass CSS (.liquid-glass):

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
Animations:

@keyframes fade-rise {
  from { opacity: 0; transform: translateY(24px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-rise { animation: fade-rise 0.8s ease-out both; }
.animate-fade-rise-delay { animation: fade-rise 0.8s ease-out 0.2s both; }
.animate-fade-rise-delay-2 { animation: fade-rise 0.8s ease-out 0.4s both; }
Page background: bg-black (hsl(0,0%,0%)), section is min-h-screen overflow-hidden.

## Minimal Workflow SaaS — SaaS [sites/minimal-workflow-saas]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(86).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/minimal-workflow-saas.webp

React 19 + TypeScript + Vite 6
Tailwind CSS v4 (via @tailwindcss/vite plugin, NOT PostCSS)
motion v12+ (import from "motion/react", NOT "framer-motion")
lucide-react (for ChevronRight icon)
Font: Google Inter (weights 400, 500, 600, 700)
```

### CloudFront Video URL

```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260525_064035_ff2947db-c2f5-47e4-818d-0e985c6ea0fc.mp4
```

---

### FILE: index.css

```css
@import "tailwindcss";
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
}

:root {
  font-family: var(--font-sans);
}

body {
  background-color: #f8fafc;
  color: #1e293b;
}
```

---

### FILE: App.tsx -- Root layout with full-bleed background video

The root div is `relative min-h-screen bg-[#f8fafc] selection:bg-slate-200 overflow-x-hidden flex flex-col justify-between`.

Inside it, TWO sibling layers:

**Layer 1 -- Background video (absolute, z-0):**
- Container: `absolute inset-x-0 top-0 bottom-0 z-0 overflow-hidden pointer-events-none`
- `` element with attributes: `autoPlay`, `muted`, `loop`, `playsInline`
  - src = the CloudFront URL above
  - className: `w-full h-full object-cover object-bottom opacity-[0.98]`
- Overlay div on top of video: `absolute inset-0 bg-white/[0.05] backdrop-blur-[2px]` (extremely subtle white wash + micro blur)

**Layer 2 -- Content (relative, z-10):**
- Container: `relative z-10 flex-grow flex flex-col`
- Contains `` directly
- Contains `` wrapping ``

---

### FILE: Navbar.tsx -- Minimal top navigation

Container: `` with `relative z-50 flex items-center justify-between px-6 py-5 max-w-7xl mx-auto w-full select-none`

**Left -- Brand logo:**
- Text "Script" in `font-bold text-[21px] tracking-tight text-[#0f172a]`
- Custom 3-bar icon next to it, rotated -15deg:
  - Wrapper: `flex flex-col gap-[2.5px] rotate-[-15deg] ml-1.5 translate-y-[1px]`
  - Bar 1: `w-3.5 h-[1.5px] bg-[#0f172a] rounded-full`
  - Bar 2: `w-2.5 h-[1.5px] bg-[#0f172a] rounded-full translate-x-[2px]`
  - Bar 3: `w-3 h-[1.5px] bg-[#64748b] rounded-full translate-x-[4px]` (lighter gray, staggered right)

**Center -- Nav links (absolute centered, hidden on mobile):**
- Container: `hidden md:flex absolute left-1/2 -translate-x-1/2 items-center gap-8 text-[13px] font-medium text-slate-600`
- 5 links: "Resources", "Service", "Support", "Developers", "Updates"
- Each: `hover:text-slate-900 transition-colors`

**Right -- CTA button:**
- Text "Join us"
- Classes: `px-4.5 py-1.5 text-xs font-medium border border-slate-200 rounded-full hover:bg-white/85 bg-white/30 backdrop-blur-sm transition-all shadow-[0_1px_2px_rgba(0,0,0,0.02)] text-slate-800`

---

### FILE: Hero.tsx -- Main hero content

Imports: `motion` from `motion/react`, `AnimatedTaskList` component, `ChevronRight` from `lucide-react` (also imports `ArrowRight` but it is unused).

Section container: `relative pt-10 pb-6 flex flex-col justify-center items-center w-full select-none`
Inner container: `relative z-10 max-w-7xl mx-auto px-6 text-center flex flex-col items-center`

**Element 1 -- Headline (motion.h1):**
- Classes: `text-4xl md:text-[45px] tracking-tight text-slate-900 mb-5 max-w-4xl mx-auto leading-[1.12]`
- Animation: `initial={{ opacity: 0, y: 20 }}` -> `animate={{ opacity: 1, y: 0 }}`, `transition={{ duration: 0.8, ease: "easeOut" }}`
- Content (3 lines separated by `
`):
  - Line 1: `Guide everyone on teams`
  - Line 2: `tech manuals`
  - Line 3: `— with a total ease of mind` (note: em dash character)

**Element 2 -- Subtext (motion.p):**
- Classes: `text-xs md:text-[13px] text-slate-500 max-w-xl mx-auto mb-6 leading-relaxed font-normal`
- Animation: same fade-up, `delay: 0.2`
- Content: "Script offers the best path to register your workflow steps" + `
` + "and optimize training on your setup systems"

**Element 3 -- CTA Button (motion.div wrapper):**
- Wrapper: `mb-14`, animation same fade-up, `delay: 0.4`
- Button classes: `bg-gradient-to-b from-[#252a38] to-[#1a1e29] hover:from-[#1d212c] hover:to-[#12151e] text-white px-5 py-2 rounded-lg text-xs font-semibold flex items-center gap-1 mx-auto transition-all shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_1px_2px_rgba(0,0,0,0.15)] border border-slate-900/80 active:scale-95 duration-150`
- Content: "Register Now!" followed by ``

**Element 4 -- Animated Task List area:**
- Outer div: `relative w-full flex flex-col items-center max-w-sm`
- AnimatedTaskList wrapper: `` with `initial={{ opacity: 0, scale: 0.95 }}`, `animate={{ opacity: 1, scale: 1 }}`, `transition={{ duration: 1, delay: 0.6 }}`, className `relative z-20 w-full`
- Below it, tagline: `` with `initial={{ opacity: 0 }}`, `animate={{ opacity: 1 }}`, `transition={{ delay: 1, duration: 1 }}`, className `mt-14 text-[10px] font-medium tracking-wide text-white/50`, text: "All people aligned."

---

### FILE: AnimatedTaskList.tsx -- Infinite auto-scrolling task queue with glass card

Imports: `React`, `useState`, `useEffect` from react; `motion` from `motion/react`.

**Task data (9 items):**
```
"How to code an app in Python"
"How to build charts with data in Excel"
"How to edit profile of users on GitHub"
"How to set up a custom task rule in Asana"
"How to design a form in Sheets"
"How to build a custom webhook in Slack"
"How to sync a dashboard in Excel"
"How to create a team member in Canva"
"How to link a custom project page in Jira"
```

`N = tasks.length` (9). `duplicatedTasks = [...tasks, ...tasks, ...tasks]` (27 items, tripled for infinite loop).

**State:**
- `index` starts at `N` (9)
- `animate` starts at `true`

**Scroll logic (3 useEffects):**

1. `setInterval` every 4500ms: increments `index` by 1 if `< N  2` (18)
2. When `index === N  2`: after 1000ms timeout, sets `animate = false` and `index = N` (silent teleport back)
3. When `index === N && !animate`: after 50ms timeout, sets `animate = true` (re-enables animation)

**Outer container:** `relative w-full max-w-[340px] md:max-w-[420px] h-[220px] select-none mx-auto text-left font-sans overflow-hidden`

**Glass highlight card (static, z-0):**
- Position: `absolute top-0 left-0 w-full h-[54px]`
- Style: `rounded-xl bg-white/[0.08] backdrop-blur-md border border-white/20 shadow-[inset_0_1px_1px_rgba(255,255,255,0.15)]`
- Layout: `flex items-center px-4 pointer-events-none`
- Contains a white icon square: `w-[30px] h-[30px] bg-white rounded-lg flex items-center justify-center shrink-0 shadow-sm border border-white/40`
  - Inside: 3-bar mini logo rotated -15deg:
    - Wrapper: `flex flex-col gap-[1.5px] rotate-[-15deg]`
    - Bar 1: `w-2.5 h-[1.5px] bg-[#0c101d] rounded-full`
    - Bar 2: `w-1.8 h-[1.5px] bg-[#0c101d] rounded-full translate-x-[0.8px]`
    - Bar 3: `w-2.2 h-[1.5px] bg-[#475569] rounded-full translate-x-[1.6px]`

**Task items layer (absolute, z-10):**
- Container: `absolute inset-0 w-full h-full z-10 pointer-events-none`
- Maps over `duplicatedTasks` (27 items). For each item at index `i`, computes `distance = i - index`:

**Position/opacity rules based on distance:**

| distance | y | height | opacity | blur |
|----------|-----|--------|---------|------|
| 0 (active) | 0 | 54px | 1.0 | 0px |
| < 0 (past) | -35 | 30px | 0.0 | 0px |
| 1 | 68px | 22px | 0.55 | 0.2px |
| 2 | 90px | 22px | 0.36 | 0.4px |
| 3 | 112px | 22px | 0.22 | 0.6px |
| 4 | 134px | 22px | 0.11 | 0.8px |
| 5 | 156px | 22px | 0.04 | 1.1px |
| 6+ | formula | 22px | 0.0 | 0px |

Formula for inactive y: `68 + (distance - 1) * 22`

**Each motion.div item:**
- Classes: `absolute left-0 w-full flex items-center select-none justify-start`
- `animate={{ y, opacity }}`, `style={{ height, filter: filterBlur }}`
- Transition: when `animate=true`: `{ duration: 1.0, ease: [0.16, 1, 0.3, 1] }` (custom spring-like bezier). When `animate=false`: `{ duration: 0 }` (instant, no animation for teleport)

**Active item rendering (distance === 0):**
- Container: `pl-[58px] flex flex-col justify-center text-left`
- Label: `text-[7.5px] text-white/50 font-bold uppercase tracking-wider leading-none mb-1`, text: "Learn the step"
- Task text: `text-[12.5px] md:text-[13px] font-medium tracking-tight text-white leading-none`

**Inactive item rendering (distance !== 0):**
- Container: `pl-[58px] flex items-center text-left`
- Task text: `text-[11.5px] md:text-[12px] font-normal tracking-tight text-white/70 leading-none`

---

### FILE: LogoCloud.tsx -- Brand logo strip (NOT displayed in current App.tsx but exists as component)

Container: `w-full bg-white border-t border-slate-100 py-7 select-none relative z-20`
Grid: `grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 items-center justify-center gap-y-8 gap-x-6`

8 brand logos, all built with inline SVGs and styled text:
1. **Mercedes-Benz** -- circle + 3-spoke SVG, `text-[10px] font-medium tracking-wider uppercase text-slate-700`
2. **Certainty** -- circle + checkmark SVG (emerald-600), `text-[13px] font-bold tracking-tight text-slate-800`
3. **STAR MOUNTAIN CAPITAL** -- 3 overlapping mountain peaks SVG, `text-[7px] font-black tracking-[0.16em]` + `text-[5px] font-semibold tracking-[0.25em] scale-90`
4. **Paige** -- dark circle with pie chart SVG, `text-[14px] font-bold tracking-tight text-slate-900`
5. **ALARIS** -- text only, `text-[13px] font-light tracking-[0.3em] uppercase`
6. **raft** -- text only, `text-[15px] font-bold tracking-tighter lowercase`
7. **Foobar** -- split weight: "Foo" `font-black text-slate-900` + "bar" `font-semibold text-slate-400`, `text-[14px]`
8. **Alph4** -- triangle SVG with internal lines, `text-[8px] font-bold tracking-widest text-slate-600 scale-95`

---

### Key Design Specifications

- **Color palette**: Entirely slate, white, charcoal-navy (#0f172a, #252a38, #1a1e29). NO purple/indigo anywhere.
- **Video background**: Covers entire viewport, `object-cover object-bottom`, 98% opacity, with a `bg-white/[0.05] backdrop-blur-[2px]` overlay
- **Glass card effect**: `bg-white/[0.08] backdrop-blur-md border border-white/20` with inset highlight shadow
- **CTA button**: Dark gradient with inset white highlight: `shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_1px_2px_rgba(0,0,0,0.15)]`
- **Animation sequence**: Staggered fade-up (0s, 0.2s, 0.4s for headline/subtext/button), then scale-in at 0.6s for task list, then fade-in at 1s for tagline
- **Task list animation curve**: Custom cubic-bezier `[0.16, 1, 0.3, 1]` (fast start, very smooth deceleration)
- **Task list cycle**: 4.5s interval, 1.0s slide duration, silent instant teleport back when exhausted
- **Text on dark video**: White with varying opacity (1.0, /70, /50) for hierarchy
- **select-none**: Applied to navbar, hero section, and task list to prevent text selection on decorative elements
- **Responsive**: Nav links hidden on mobile (`hidden md:flex`), task list width `max-w-[340px] md:max-w-[420px]`, headline `text-4xl md:text-[45px]`

## Neuralyn — SaaS [sites/neuralyn-hero]

- Preview: https://motionsites.ai/assets/hero-neuralyn-preview-Br4FRDQA.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/neuralyn-hero.gif

Create a dark landing page for "Neuralyn" — an analytics dashboard SaaS. Use React + Vite + Tailwind CSS + TypeScript + Framer Motion + shadcn/ui.

Fonts
Inter (400, 500, 600, 700) for body/UI via @fontsource/inter
Instrument Serif (400, 400-italic) for the italic accent word via @fontsource/instrument-serif

Color Theme (all HSL, dark mode by default in :root)
Background: 0 0% 0% (pure black)
Foreground: 0 0% 100% (pure white)
Muted foreground: 0 0% 65%
Card: 0 0% 5%
Border: 0 0% 20%
Hero subtitle: 210 17% 95%

Page Structure
Section 1: Hero (full viewport height, overflow-hidden)

Navbar — horizontal, padded px-8 md:px-28 py-4:

Left: Logo image + "Neuralyn" text (text-xl font-bold tracking-tight) + nav links (Home, Services with ChevronDown icon, Reviews, Contact us) — links hidden on mobile, gap-1 between links, gap-12 md:gap-20 between logo and links
Right: "Sign In" button — solid white background (bg-foreground), black text (text-background), rounded-lg text-sm font-semibold, hover opacity transition

Hero Content — centered column, mt-16 md:mt-20 px-4:

Tag pill: A "liquid glass" styled pill (liquid-glass class) with inner "New" badge (white bg, black text, rounded-md text-sm font-medium px-2 py-0.5) + "Say Hello to Corewave v3.2" in text-sm font-medium text-muted-foreground. Pill has px-3 py-2 rounded-lg mb-6.
Title: text-5xl md:text-7xl, tracking-[-2px], font-medium, leading-tight md:leading-[1.15] mb-3. Text: "Your Insights." / "One Clear Overview." — the word "Overview" is in Instrument Serif italic (font-serif italic font-normal)
Subtitle: text-lg font-normal leading-6 opacity-90 mb-8, color uses CSS variable --hero-subtitle. Text: "Neuralyn helps teams track metrics, goals, and progress with precision." with a <br/> after "goals,"
CTA Button: "Get Started for Free" — solid white (bg-foreground text-background), rounded-full px-8 py-3.5 text-base font-medium, whileHover: scale 1.03, whileTap: scale 0.98

Dashboard + Video Area — full viewport width using w-screen with marginLeft: calc(-50vw + 50%) trick, aspect-ratio: 16/9, positioned relative:

Background video: <video>, absolutely positioned inset-0 w-full h-full object-cover. URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260307_083826_e938b29f-a43a-41ec-a153-3d4730578ab8.mp4
Dashboard image: Absolutely positioned, centered, max-w-5xl w-[90%] rounded-2xl, mixBlendMode: "luminosity". Has parallax scroll (y: 0→-250).
Bottom gradient fade: Absolutely positioned at bottom of section, h-40, gradient from background to transparent, z-30, pointer-events-none.

Parallax Scroll Effects (Framer Motion useScroll({ target: sectionRef, offset: ["start start", "end start"] }) + useTransform):

Hero text content group: y: [0, -200] and opacity: [1, 0] (fades over first 50% of scroll)
Dashboard image: y: [0, -250]

Entrance Animations: Staggered initial={{ opacity: 0, y }} / animate={{ opacity: 1, y: 0 }}:

Tag pill: y: 10, duration 0.5s, delay 0
Title: y: 20, duration 0.6s, delay 0.1
Subtitle: y: 20, duration 0.6s, delay 0.2
CTA: y: 20, duration 0.6s, delay 0.3
Dashboard area: y: 40, duration 0.8s, delay 0.4

Liquid Glass CSS

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

Section 2: Testimonial (min-h-screen, centered, py-24 md:py-32 px-8 md:px-28)

Quote symbol image (w-14 h-10 object-contain)
Testimonial text (text-4xl md:text-5xl font-medium leading-[1.2], wrapped in flex flex-wrap): "Neuralyn revolutionized how we handle financial insights using smart analytics. We are now driving better outcomes quicker than we ever imagined! Neuralyn revolutionized how we handle financial insights using smart analytics."
Scroll-driven word reveal: Each word is a <motion.span> with mr-[0.3em]. Uses useScroll({ target: containerRef, offset: ["start end", "end center"] }). Each word maps to a sequential range [i/total, (i+1)/total] → opacity: [0.2, 1] and color: ["hsl(0 0% 35%)", "hsl(0 0% 100%)"].
Closing " quotation mark in text-muted-foreground ml-2
Author row (flex items-center gap-4): Avatar image (w-14 h-14 rounded-full border-[3px] border-foreground object-cover) + name "Brooklyn Simmons" (text-base font-semibold leading-7 text-foreground) + role "Product Manager" (text-sm font-normal leading-5 text-muted-foreground)
Layout: max-w-3xl mx-auto, content left-aligned (items-start), gap-10 between elements

Assets needed:
logo.png — small logo icon
hero-dashboard.png — dashboard screenshot
quote-symbol.png — decorative quote mark
testimonial-avatar.png — circular headshot

## Nexora Automation — SaaS [sites/nexora-hero]

- Preview: https://motionsites.ai/assets/hero-nexora-preview-cx5HmUgo.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexora-hero.gif

Create a SaaS landing page hero section with the following exact specifications:

Page Layout

The entire page is h-screen flex flex-col bg-background overflow-hidden — the Navbar + Hero fill exactly 100vh with no scroll.
The page uses two Google Fonts imported via CSS: Instrument Serif (display/headings, including italic) and Inter (body text).
Fonts & Design Tokens (index.css)

Import fonts:

@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Inter:wght@400;500;600&display=swap');
CSS variables (:root):

--background: 0 0% 100% (white)
--foreground: 210 14% 17% (dark charcoal)
--primary: 210 14% 17% / --primary-foreground: 0 0% 100%
--secondary: 0 0% 96% / --secondary-foreground: 0 0% 9%
--muted: 0 0% 96% / --muted-foreground: 184 5% 55%
--accent: 239 84% 67% (indigo/blue) / --accent-foreground: 0 0% 100%
--border: 0 0% 90%
--ring: 239 84% 67%
--radius: 0.5rem
--font-display: 'Instrument Serif', serif
--font-body: 'Inter', sans-serif
--shadow-dashboard: 0 25px 80px -12px rgba(0, 0, 0, 0.08), 0 0 0 1px rgba(0, 0, 0, 0.06)
Tailwind config extends fontFamily with display and body mapped to the CSS vars. All colors use hsl(var(--token)) pattern.

Navbar

flex items-center justify-between px-6 md:px-12 lg:px-20 py-5 font-body
Left: Logo text ✦ Nexora — text-xl font-semibold tracking-tight text-foreground
Right (hidden on mobile): Nav links "Home", "Pricing", "About", "Contact" — text-sm text-muted-foreground hover:text-foreground with gap-8
CTA button: rounded-full px-5 text-sm font-medium using primary styling
Hero Section




Background Video: Fullscreen muted autoplay loop video, absolute inset-0 w-full h-full object-cover z-0
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260319_015952_e1deeb12-8fb7-4071-a42a-60779fc64ab6.mp4
All content wrapped in relative z-10 flex flex-col items-center w-full
1. Badge (top)

Framer Motion: fade up from y:10, duration 0.5s
inline-flex items-center gap-1.5 rounded-full border border-border bg-background px-4 py-1.5 text-sm text-muted-foreground font-body
Text: "Now with GPT-5 support ✨"
mb-6
2. Headline

Framer Motion: fade up from y:16, duration 0.6s, delay 0.1s
text-center font-display text-5xl md:text-6xl lg:text-[5rem] leading-[0.95] tracking-tight text-foreground max-w-xl
Content: The Future of Smarter Automation — the word "Smarter" renders in Instrument Serif italic
3. Subheadline

Framer Motion: fade up from y:16, duration 0.6s, delay 0.2s
mt-4 text-center text-base md:text-lg text-muted-foreground max-w-[650px] leading-relaxed font-body
Text: "Automate your busywork with intelligent agents that learn, adapt, and execute—so your team can focus on what matters most."
4. CTA Buttons

Framer Motion: fade up from y:16, duration 0.6s, delay 0.3s
mt-5 flex items-center gap-3
Primary button: rounded-full px-6 py-5 text-sm font-medium font-body — text "Book a demo"
Play button: ghost variant, h-11 w-11 rounded-full border-0 bg-background shadow-[0_2px_12px_rgba(0,0,0,0.08)] hover:bg-background/80 with a Play icon (lucide) h-4 w-4 fill-foreground
5. Dashboard Preview (custom coded, NOT an image)

Framer Motion: fade up from y:30, duration 0.8s, delay 0.5s
Container: mt-8 w-full max-w-5xl
Frosted glass wrapper: rounded-2xl overflow-hidden p-3 md:p-4 with inline styles:
background: rgba(255, 255, 255, 0.4)
border: 1px solid rgba(255, 255, 255, 0.5)
boxShadow: var(--shadow-dashboard)
Dashboard internals (all coded in React, text-[11px], select-none pointer-events-none):

Top bar: Logo "N" in rounded box + "Nexora" + chevron | Search bar with ⌘K shortcut | "Move Money" + bell + avatar "JB"
Sidebar (w-40): Items — Home (active), Tasks (badge "10"), Transactions, Payments (chevron), Cards, Capital, Accounts (chevron). Section "Workflows": Trake rutes, Payments, Notifications, Settings
Main content (bg-secondary/30):
Greeting: "Welcome, Jane" — text-sm font-semibold
Action buttons row: Send (primary/accent), Request, Transfer, Deposit, Pay Bill, Create Invoice — rounded-full pill buttons text-[10px], + "Customize" text
Two equal-width cards (flex-1 basis-0) side by side:
Balance card: "Mercury Balance" with checkmark, amount $8,450,190.32 (cents in text-xs text-muted-foreground), stats (Last 30 Days, +$1.8M green, -$900K red), SVG area chart (h-20) with smooth cubic Bézier curve, linear gradient fill from accent at 15% opacity to transparent, stroke in accent color strokeWidth="1.5"
Accounts card: Header "Accounts" with + and ⋮ icons. Three rows (py-3, no dividers, text-xs, justify-between): Credit $98,125.50, Treasury $6,750,200.00, Operations $1,592,864.82
Transactions table: "Recent Transactions" heading, table with columns Date/Description/Amount/Status. 4 rows: AWS -$5,200 Pending (amber), Client Payment +$125,000 Completed (green), Payroll -$85,450 Completed, Office Supplies -$1,200 Completed
Dependencies

framer-motion for all animations
lucide-react for all icons
shadcn/ui Button component
Tailwind CSS with tailwindcss-animate plugin
Key Design Decisions

The dashboard overflows toward the bottom of the viewport and is clipped by overflow-hidden on the parent
No dark mode — light only
All colors use semantic Tailwind tokens, never raw color values in components
The SVG chart uses a hand-crafted cubic Bézier path, not a charting library

## Nickel Payments — SaaS [sites/nickel-hero]

- Preview: https://motionsites.ai/assets/hero-nickel-preview-CnRoBZt5.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nickel-hero.gif

Create a landing page hero section with a floating navbar. Use React, Tailwind CSS, and shadcn/ui.

Design System (index.css):

Background: 249 18% 95% (warm off-white)

Foreground: 240 10% 10% (near-black)

Primary: 24 90% 55% (warm orange)

Primary foreground: 0 0% 100% (white)

Secondary: 0 0% 100% (white)

Secondary foreground: 240 10% 10%

Muted foreground: 240 5% 46%

Border: 240 10% 88%

Nav background: 0 0% 100% (white)

Font family: Inter, system-ui, sans-serif

Navbar:

Full-width outer wrapper with px-6 lg:px-8 pt-4

Inner <nav> is max-w-7xl mx-auto, white background (bg-nav), rounded-xl, shadow-sm

Inner padding: px-8 py-5, flex row, items centered, space-between

Logo (left): Link with text-2xl font-bold tracking-tight. Icon is a w-7 h-7 black circle (bg-foreground rounded-full) containing a w-3 h-3 white rounded square (bg-white rounded-sm). Text: "nickel"

Center links (hidden on mobile, md:flex gap-6): "Products" and "Company" are buttons with a ChevronDown icon (h-3.5 w-3.5). "Pricing" and "For Accountants" are plain links. All use text-base font-medium text-foreground/80 hover:text-foreground transition-colors

Right side: "Log in" link (same style as nav links, hidden on sm down). "Get started" button using a hero variant with default size

Hero Button Variants (in button.tsx):

hero: bg-gradient-to-b from-[hsl(24,100%,72%)] to-[hsl(18,98%,53%)] text-primary-foreground hover:opacity-90 rounded-lg text-lg font-medium

hero-outline: bg-secondary text-secondary-foreground hover:bg-muted rounded-lg text-lg font-medium

Size xl: h-14 px-10 py-4

Hero Section:

<section> with bg-background min-h-[calc(100vh-4rem)] relative overflow-hidden

Content container: max-w-7xl mx-auto px-6 lg:px-8 min-h-[calc(100vh-4rem)] flex items-center w-full relative z-10

Text block: max-w-xl

H1: text-5xl sm:text-6xl lg:text-7xl font-medium tracking-tight text-foreground leading-[1.05] — "Unlock growth with every payment"

Paragraph: mt-6 text-lg sm:text-xl text-muted-foreground max-w-xl leading-relaxed — "Run payments, extend net terms and automate collections compliance."

Buttons row: mt-10 flex flex-wrap gap-4 — "Get started" (variant="hero" size="xl") and "Talk to a human" (variant="hero-outline" size="xl")

Video (right side): Absolutely positioned absolute top-0 right-0 w-[55%] h-full hidden lg:block. Video element: w-full h-full object-cover rounded-bl-2xl, autoPlay, loop, muted, playsInline. Source URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260319_192508_4eecde4c-f835-4f4b-b255-eafd1156da99.mp4

Page layout: min-h-screen bg-background, renders <Navbar /> then <HeroSection />

## Planet Orbit — SaaS [sites/planet-orbit-hero]

- Preview: https://motionsites.ai/assets/hero-planet-orbit-preview-DWAP8Z1P.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/planet-orbit-hero.gif

Create a dark SaaS landing page hero section with the following exact specifications:

Font: Geist Sans (400, 500, 600, 700 weights) via @fontsource/geist-sans

Color System (HSL):

Background: 260 87% 3% (near-black with slight purple)
Foreground: 40 6% 95% (warm off-white)
Primary/Accent: 121 95% 76% (#87FB89 green)
Primary foreground: 0 0% 5% (dark text on green buttons)
Hero heading: 40 10% 96%
Hero sub: 40 6% 82%
Secondary/Muted: 240 4% 16%
Border: 240 4% 20%

Background Video: Full-screen background video covering the entire section (navbar through social proof). URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260309_042944_4a2205b7-b061-490a-852b-92d9e9955ce9.mp4 Set to autoPlay, loop, muted, playsInline with object-cover and absolute inset-0.

Liquid Glass Effect (reusable utility class .liquid-glass):

background: rgba(255, 255, 255, 0.01);
background-blend-mode: luminosity;
backdrop-filter: blur(4px);
border: none;
box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1);
overflow: hidden;
Plus a ::before pseudo-element with a vertical gradient border using mask-composite:

padding: 1.4px;
background: linear-gradient(180deg,
  rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
  rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
  rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
-webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
-webkit-mask-composite: xor;
mask-composite: exclude;

Layout (top to bottom, all centered over the video):

Navbar — Centered liquid-glass pill (rounded-3xl, max-w-[850px]) containing:

Logo: Small rounded-lg gradient square with a crosshair SVG icon + "APEX" text (xl, semibold)
Nav items: "Features" (with chevron-down), "Solutions", "Plans", "Learning" (with chevron-down) — text-base, foreground/90 opacity
CTA button: "Sign Up" — green primary, rounded-xl, small size

Announcement Badge — liquid-glass rounded-full pill: "Nova+ Launched!" text + "Explore" chip with ChevronRight icon, nested pill with bg-white/5

Heading — text-4xl sm:text-6xl lg:text-7xl, font-semibold, tracking-tight, leading-[1.05], max-w-5xl:

Accelerate Your
Revenue Growth Now

Subheading — text-lg, max-w-md, opacity-80, hero-sub color: "Drive your funnel forward with clever workflows, analytics, and seamless lead management."

Two CTA Buttons side by side:

"Start Free Right Now" — green primary, rounded-full, px-6 py-3
"Schedule a Consult" — liquid-glass, rounded-full, px-6 py-3, hover:bg-white/5

Social Proof Marquee at the bottom — "Relied on by brands across the globe" label (foreground/50, text-sm) on the left, then a horizontally scrolling marquee of brand names: Vortex, Nimbus, Prysma, Cirrus, Kynder, Halcyn. Each has a small liquid-glass rounded-lg icon square with the first letter + the brand name (text-base, font-semibold). Duplicated array for seamless loop. Animation: translateX(0%) → translateX(-50%) over 20s linear infinite.

Button Variants (class-variance-authority):

hero: bg-primary text-primary-foreground rounded-full px-6 py-3 text-base font-medium hover:bg-primary/90
heroSecondary: liquid-glass text-foreground rounded-full px-6 py-3 text-base font-normal hover:bg-white/5

## Price Calculator — SaaS [sites/price-calculator]

- Preview: https://motionsites.ai/assets/hero-price-calculator-preview-Dak8DDgY.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/price-calculator.gif

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

## SaaS Value — SaaS [sites/saas-value]

- Preview: https://res.cloudinary.com/dsdhxhhqh/image/upload/v1781539562/CleanShot_2026-06-15_at_15.31.34_2x_ckhjmj.png
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/saas-value.png

Build a full-viewport hero section for a SaaS landing page called "Questly" using React, TypeScript, Tailwind CSS 3, and Vite. Use `lucide-react` for all icons. No other UI libraries.

---

FONT

Use the font "Nimbus Sans TW01" loaded from this stylesheet in `index.html`:

```
https://db.onlinewebfonts.com/c/bb5de19d87c09a95216dc6ccd96e37c6?family=Nimbus+Sans+TW01
```

Set the font stack in both `tailwind.config.js` and `index.css`:

```
'Nimbus Sans TW01', 'Helvetica Neue', Helvetica, Arial, sans-serif
```

Enable `-webkit-font-smoothing: antialiased` and `-moz-osx-font-smoothing: grayscale` on `html`.

---

BACKGROUND IMAGE

The full hero section uses this image as a `background-image` (cover, centered):

```
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260611_133301_d5f2a94a-b22e-4e4a-a6b6-eacdddf1f5b0.png&w=1280&q=85
```

Applied via inline `style={{ backgroundImage: url(...) }}` on the `

`. The section is `relative min-h-[100svh] overflow-hidden bg-cover bg-center flex flex-col`.

---

GRASS OVERLAY

An absolutely positioned grass PNG sits at the bottom of the section, full width, `z-10`, pointer-events-none, select-none:

```
https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781191264/grass_eam204.png
```

Classes: `pointer-events-none absolute bottom-0 left-0 z-10 w-full select-none`

---

LOGO (SVG Component)

A custom SVG logo component used in the navbar and dashboard sidebar. It uses `currentColor` for fill so it inherits text color. ViewBox: `0 0 256 256`. Path data:

```
M 144 256 L 27.598 256 L 144 139.598 Z M 256 207.5 L 200 256 L 200 56 L 0 56 L 48 0 L 256 0 Z M 0 204.402 L 0 112 L 92.402 112 Z
```

---

NAVBAR

- Positioned with `animate-fade-down relative z-20`
- Flex row: logo left, nav links center, CTA + hamburger right
- Horizontal padding: `px-5 sm:px-8 lg:px-10`, vertical: `py-4 sm:py-5`
- Logo: `text-gray-900`, icon sized `w-5 h-5 sm:w-6 sm:h-6`
- Desktop nav links (hidden below `md`): `text-[13px] text-gray-700`, hover `text-gray-900`, gap-8. Items: "Toolkit" (with `ChevronDown` icon `w-3.5 h-3.5`), "Plans", "News"
- CTA button: `bg-gray-900 text-white text-[13px] font-medium px-4 sm:px-5 py-2 rounded-full hover:bg-gray-800`
- Hamburger (md:hidden): `w-9 h-9 rounded-full text-gray-900 hover:bg-gray-900/10`, toggles `Menu`/`X` icons (`w-5 h-5`)
- Mobile dropdown (when open): `absolute left-4 right-4 top-full rounded-2xl bg-white/80 backdrop-blur-xl ring-1 ring-gray-200 px-5 py-3 animate-fade-up`. Links: `text-[15px] text-gray-700 hover:text-gray-900 border-b border-gray-200 last:border-b-0`

---

HERO CONTENT (centered, text-center)

Spacing between navbar and content uses a flex spacer: `flex-1 min-h-8 sm:min-h-12 lg:min-h-16 shrink-0`

Headline (h1)
- `text-gray-900 font-normal leading-[1.05] tracking-tight`
- Sizes: `text-[40px] min-[400px]:text-[44px] sm:text-6xl lg:text-7xl xl:text-[80px]`
- Two lines, each a `` with staggered `animate-fade-up`:
  - Line 1: "Get cited." (no delay)
  - Line 2: "Effortlessly." (`[animation-delay:100ms]`)

### Search Bar (form)
- `animate-fade-up [animation-delay:220ms] mt-5 sm:mt-6 w-full max-w-xl`
- Pill container: `flex items-center gap-3 rounded-full bg-white/60 backdrop-blur-md ring-1 ring-gray-200 pl-5 pr-1.5 py-1.5`
- Input: `flex-1 bg-transparent text-sm sm:text-base text-gray-900 placeholder-gray-500 outline-none py-2`, placeholder: "What makes content rank in AI search?"
- Submit button: `w-9 h-9 sm:w-10 sm:h-10 rounded-full bg-gray-900 text-white hover:scale-105 active:scale-95 transition-transform shrink-0`, contains `ArrowUp` icon `w-4 h-4 sm:w-[18px] sm:h-[18px]`

### Description
- `animate-fade-up [animation-delay:340ms] mt-4 sm:mt-5 text-gray-600 text-sm sm:text-base lg:text-lg leading-relaxed max-w-md`
- Text: "Ship articles that answer actual customer questions -- and be seen on [Sparkles icon] ChatGPT"
- Line break `
` before the dash
- `Sparkles` icon: `inline w-4 h-4 -mt-1`

### CTA Buttons
- `animate-fade-up [animation-delay:460ms] mt-4 sm:mt-5 flex flex-wrap items-center justify-center gap-3`
- **Primary**: `bg-gray-900 text-white text-sm font-medium px-6 py-2.5 rounded-full hover:bg-gray-800 hover:shadow-lg transition-all` -- "Try It Free"
- **Secondary**: `text-gray-700 text-sm font-medium px-6 py-2.5 rounded-full ring-1 ring-gray-300 hover:bg-gray-100 transition-colors` -- "Talk to sales"

---

### DASHBOARD MOCKUP (below the hero content)

Another flex spacer (`flex-1 min-h-10 sm:min-h-12 lg:min-h-16 shrink-0`) separates the content from the dashboard.

### Container
- `animate-hero-rise [animation-delay:620ms] relative z-0 w-[92%] sm:w-[84%] lg:w-[72%] max-w-4xl mx-auto shrink-0 -mb-10 sm:-mb-20 lg:-mb-32`
- Uses a **ScaledDashboard** wrapper: a `ResizeObserver`-based component that renders the mockup at a fixed design width of **896px** and scales it down via CSS `transform: scale()` to fit its container, with `transformOrigin: 'top left'`. The outer div's height is set to `inner.offsetHeight * scale` to prevent layout overflow.

### Mockup Chrome
- Outer: `rounded-t-2xl overflow-hidden bg-[#1a1a1c] shadow-[0_-20px_80px_rgba(0,0,0,0.35)] ring-1 ring-white/10 text-left`
- **Title bar**: `bg-[#242427] border-b border-white/5 px-4 py-2.5`
  - Traffic lights: three spans `w-2.5 h-2.5 rounded-full` colored `#ff5f57`, `#febc2e`, `#28c840`
  - Icons (all `w-3.5 h-3.5 text-white/40`): `PanelLeft`, `ChevronLeft`, `ChevronRight` (text-white/25)
  - Center URL bar: `bg-[#1a1a1c] rounded-md px-6 py-1 text-[10px] text-white/60` with `Monitor` icon -- text "questly.ai"
  - Right icons: `RotateCw`, `Share`, `Plus`, `Copy`

### Sidebar (22% width)
- `border-r border-white/5 bg-[#1e1e21] px-3 py-3.5`
- Logo icon `w-4 h-4 text-white/70` + `Grid` icon `w-3.5 h-3.5 text-white/30`
- Workspace badge: `w-4 h-4 rounded bg-[#e8553f]` with "C" letter, label "CareNest" `text-[10px] text-white/80`
- Nav items: Compass/Uncover, Layers/Subjects, ListTodo/Inbox -- `text-[10px] text-white/60`
- Recent articles list with "Ready to Release" green dots `text-[#28c840]/70`

### Main Content Area
- Header: workspace icon (larger `w-9 h-9 rounded-lg bg-[#e8553f]`), "CareNest" `text-sm font-medium text-white`, subtitle `text-[10px] text-white/45`, and a "Generate" button with `Sparkles` icon
- **Stats grid** (4 columns): `grid-cols-4 divide-x divide-white/5 rounded-xl bg-white/[0.03] ring-1 ring-white/5`
  - RELEASED: 62 / Posts indexed
  - BREADTH: 12 / Subject groups
  - REMAINING: 412 / Ready to draft
  - MAX REACH: 3,156,200 / Searches a month
  - Values: `text-xl font-medium text-white`, labels: `text-[8px] tracking-wider text-white/35`
- **Subject cards** (3 columns): Elder Care, Mobility, Home Safety -- `rounded-lg bg-white/[0.03] ring-1 ring-white/5`
- **Drafting inbox** table: 5 rows with question, volume, difficulty, status columns. "Drafting" status colored `text-[#febc2e]/80`

---

### ANIMATIONS (defined in index.css)

```css
@keyframes fade-up {
  from { opacity: 0; transform: translateY(24px); filter: blur(6px); }
  to { opacity: 1; transform: translateY(0); filter: blur(0); }
}

@keyframes fade-down {
  from { opacity: 0; transform: translateY(-16px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes hero-rise {
  from { opacity: 0; transform: translateY(64px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

.animate-fade-up { animation: fade-up 0.9s cubic-bezier(0.22, 1, 0.36, 1) both; }
.animate-fade-down { animation: fade-down 0.7s cubic-bezier(0.22, 1, 0.36, 1) both; }
.animate-hero-rise { animation: hero-rise 1.1s cubic-bezier(0.22, 1, 0.36, 1) both; }
```

Staggered delays applied via inline `[animation-delay:Xms]` Tailwind arbitrary values. Respect `prefers-reduced-motion: reduce` by disabling all three animations.

---

RESPONSIVE BREAKPOINTS SUMMARY

| Element | Mobile (<640) | SM (640+) | MD (768+) | LG (1024+) | XL (1280+) |
|---|---|---|---|---|---|
| Headline | 40px / 44px@400 | 60px | -- | 70px | 80px |
| Nav links | Hidden (hamburger) | -- | Visible | -- | -- |
| Search bar width | full | -- | -- | -- | max-w-xl |
| Dashboard width | 92% | 84% | -- | 72% | -- |
| Dashboard bottom overlap | -mb-10 | -mb-20 | -- | -mb-32 | -- |

---

FILE STRUCTURE

```
src/
  App.tsx            -- renders <Hero />
  main.tsx           -- ReactDOM.createRoot
  index.css          -- Tailwind directives + custom keyframes
  components/
    Hero.tsx          -- main section with bg image, content, ScaledDashboard, grass overlay
    Navbar.tsx        -- top nav with mobile drawer
    Logo.tsx          -- SVG logo component
    DashboardMockup.tsx -- full browser-chrome dashboard mockup
```

## Securify Data Security — SaaS [sites/securify-hero]

- Preview: https://motionsites.ai/assets/hero-securify-preview-DQSYrftH.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/securify-hero.gif

Build a full-screen hero section for a data-security SaaS landing page called "securify" using React + TypeScript + Tailwind CSS, with a looping fullscreen background video, a floating pill-shaped navbar, and large staggered typography.

Fonts & Global Styles

Load Google font "Readex Pro" weights 300, 400, 500, 600, 700.
Set body font-family: 'Readex Pro', system-ui, -apple-system, sans-serif;, background #000, color #fff, antialiased.
Make html, body, #root height 100%.
Add a .hero-title class with letter-spacing: -0.04em; line-height: 0.95;.
Section container

A <section> with classes: relative h-screen w-full overflow-hidden bg-black.
Background video

<video> with className="absolute inset-0 w-full h-full object-cover", autoPlay loop muted playsInline, and src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260418_063509_7d167302-4fd4-480b-8260-18ab572333d4.mp4".
Navbar (absolute, z-20, px-6 md:px-10 pt-6, top-0 left-0 right-0)

A <nav> with flex items-center justify-between gap-4.
Left pill: flex items-center gap-2 bg-neutral-900/90 backdrop-blur rounded-full pl-4 pr-6 py-3 containing:
A custom white SVG logo (viewBox 0 0 256 256, class h-5 w-5) with path: M 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 128 L 64 128 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z M 128 64 L 128 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 Z M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 128 0 L 192 0 Z filled #ffffff.
Brand text "securify" (text-white text-sm font-normal tracking-tight).
Center pill (hidden on mobile): hidden md:flex items-center gap-1 bg-neutral-900/90 backdrop-blur rounded-full px-3 py-2 with four anchor links: "platform", "solutions", "company", "support" — each text-neutral-300 hover:text-white transition-colors text-sm px-5 py-2 rounded-full.
Right button: "get started" — bg-white text-black text-sm font-normal rounded-full px-6 py-3 hover:bg-neutral-200 transition-colors.
Foreground content wrapper: relative h-full w-full (rendered after Navbar, above the video).

Three giant staggered headline words (each an <h1> with class hero-title absolute text-white font-medium text-[14vw] md:text-[13vw]):

"protect" — left-4 md:left-10 top-[18%]
"your" — right-4 md:right-10 top-[38%]
"data" — left-[18%] md:left-[28%] top-[58%]
All lowercase.

Description paragraph (absolute, left-6 md:left-10 top-[46%], max-w-[240px] text-[15px] leading-snug text-white/90):

"we can guarding your data with utmost care, empowering you with privacy everywhere"

Stat block — top-right (absolute right-6 md:right-24 top-[14%]):

Row: flex items-center gap-3 justify-end — a diagonal divider (hidden md:block h-px w-24 bg-white/40 rotate-[20deg]) then number "+65k" (text-4xl md:text-5xl font-medium tracking-tight).
Sublabel: "startups use" (text-xs md:text-sm text-white/70 mt-1 text-right).
Bottom gradient overlay: pointer-events-none absolute bottom-0 left-0 right-0 h-48 bg-gradient-to-b from-transparent to-black.

Stat block — bottom-left (absolute left-6 md:left-20 bottom-20 md:bottom-24):

Row: number "+1.5b" then divider hidden md:block h-px w-24 bg-white/40 rotate-[-20deg].
Sublabel: "gb data was protected" (text-xs md:text-sm text-white/70 mt-1).
Stat block — bottom-right (absolute right-6 md:right-20 bottom-16 md:bottom-20):

Row: diagonal divider rotate-[-20deg] then "+300k".
Sublabel: "downloads" (right-aligned, text-white/70).
Notes

All text is lowercase.
Navbar pills use bg-neutral-900/90 backdrop-blur.
Only transitions: hover:text-white on nav links, hover:bg-neutral-200 on the button.
No purple/indigo anywhere; palette is pure black, white, neutral-900, and white opacity variants (white/40, white/70, white/90).
Responsive: mobile hides nav links and diagonal dividers; typography scales via vw units.

## Slate — SaaS [sites/slate-hero]

- Preview: https://motionsites.ai/assets/slate-hero-BY-9TCfd.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/slate-hero.gif

Build a full-screen hero landing page for a productivity SaaS called "Slate" with the exact specs below.

Tech Stack
Vite + React + TypeScript
Tailwind CSS
lucide-react for icons
gsap + @gsap/react for the heading text animation (via a SplitText component)
Background
Use a fullscreen autoplay/muted/loop/playsInline <video> absolutely positioned to cover the viewport (absolute inset-0 w-full h-full object-cover). Source:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260425_081506_cfddbdab-90d5-49b8-aa1a-8f52de33d335.mp4
A relative z-10 container holds all foreground content with flex flex-col min-h-screen lg:h-screen lg:overflow-hidden.

Typography (global)
In index.css, apply globally:


* {
  font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-weight: 200;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
h1, h2, h3, h4, h5, h6 { font-weight: inherit; }
Liquid Glass Effect
Two reusable CSS classes:


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
Navbar
Padding: px-4 sm:px-6 lg:px-10 py-4 lg:py-5, flex with justify-between.
Left: a 22x22 white inline SVG logo (four-square chevron mark) and the wordmark "Slate" (text-white font-light text-lg tracking-tight).
Logo path: M 256 256 L 128 256 L 0 128 L 128 128 Z M 256 128 L 128 128 L 0 0 L 128 0 Z in viewBox 0 0 256 256.
Center (hidden on <lg): nav links Our Approach, Products, Story, Resources, Billing (text-white/70 hover:text-white text-sm, gap-8).
Right: a liquid-glass pill button "Start today", rounded-xl px-4 sm:px-5 py-2 text-white/90 text-xs sm:text-sm font-light, with inline background: rgba(255,255,255,0.22).
Hero Block
Centered column: flex flex-col items-center text-center px-4 sm:px-6 pt-8 sm:pt-12 lg:pt-16 pb-8 sm:pb-10 lg:pb-12.

Badge (top)
A liquid-glass rounded-full px-3 sm:px-4 py-1.5 mb-4 pill. Three children separated by spacing/divider:

Welcome to Slate 2.4! (text-white/80 text-xs sm:text-sm)
| divider (text-white/50)
Read Guide + <ArrowRight size={13} /> from lucide-react (text-white/70 hover:text-white)
Heading (animated)
A flex-column wrapper:


style={{
  fontSize: 'clamp(36px, 8vw, 76px)',
  letterSpacing: '-1.5px',
  lineHeight: 1.1,
  fontWeight: 200,
  textShadow: '0 2px 20px rgba(0,0,0,0.3)',
}}
Two stacked SplitText components (the second wrapped in <div style={{ marginTop: '-0.15em' }}>):

Line 1: "Grow Your Team"
Line 2: "Thriving"
SplitText props for both:


tag="h1"
delay={60}
duration={0.8}
ease="power3.out"
splitType="chars"
from={{ opacity: 0, y: 40 }}
to={{ opacity: 1, y: 0 }}
threshold={0.1}
rootMargin="0px"
textAlign="center"
Implementation: a GSAP-based component that splits text into characters, animates them on scroll-trigger entrance with a per-char stagger, and respects all those props.

Subtext (staggered fade-in)

<p
  className="text-white/65 max-w-md mb-6 leading-relaxed px-2 hero-fade-up"
  style={{ fontSize: 'clamp(13px, 1.5vw, 17px)', lineHeight: 1.6, animationDelay: '0.6s' }}
>
  Build a thriving hub where your smartest ideas grow<br className="hidden sm:block" />
  smoothly, and your dreams arrive sooner than ever.
</p>
CTA Buttons (staggered fade-in)
Container: flex flex-col sm:flex-row items-stretch sm:items-center gap-3 sm:gap-4 w-full sm:w-auto max-w-xs sm:max-w-none.

Primary (animationDelay: '0.85s', inline background: rgba(255,255,255,0.22)):

liquid-glass rounded-xl px-6 sm:px-7 py-2.5 text-white font-light flex items-center justify-center gap-2.5 transition-all duration-200 group hero-fade-up
Label "Start today" (fontSize: 15)
<ArrowUpRight size={18} className="group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
Secondary (animationDelay: '1.0s'):

Same classes minus the inline background, plus hover:bg-white/8
Label "Watch demo" (fontSize: 15), no icon
Fade-up animation CSS

@keyframes heroFadeUp {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: translateY(0); }
}
.hero-fade-up {
  opacity: 0;
  animation: heroFadeUp 0.9s cubic-bezier(0.22, 1, 0.36, 1) forwards;
  will-change: transform, opacity;
}
Constraints
No purple/indigo hues. White text on the video, glass surfaces use rgba(255,255,255,*) only.
Fully responsive across mobile, tablet, desktop.
No drop shadows on glass surfaces other than the inner highlight already specified.
All icons from lucide-react only.

## Terra Geo Map — SaaS [sites/terra-hero]

- Preview: https://motionsites.ai/assets/hero-terra-preview-BFjrCr7T.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/terra-hero.gif

Create a hero section for a geo-mapping SaaS landing page called "Terra" with these exact specifications:

Font: Inter (weights: 400, 500, 700)

Primary blue color: #2E7DF3 (HSL: 217 90% 57%)

Navigation bar:
- Logo: 32px circle with gradient from-primary to-blue-400, containing 🌍 emoji, followed by "Terra" in bold
- Desktop nav (visible at lg: breakpoint and up): "Product", "Solutions", "Resources" with dropdown chevron SVG arrows, "Examples", "Pricing" — all in muted-foreground, hover to foreground
- Right side: "Login" button (rounded-full, border, ghost style) and "Sign Up" button (rounded-full, primary bg)
- Mobile/tablet (< lg): Hamburger menu using lucide-react Menu/X icons, toggling a dropdown with all nav items and buttons

Hero content (centered, flex column):

1. Product Hunt badge — mt-10 top spacing, rounded-lg border with border-red-200 bg-red-50/50, contains 🏆 emoji, "PRODUCT HUNT" label (10px, uppercase, tracking-wider, red-400) and "#1 Product of the Day" (14px, semibold, red-500)

2. Heading — font-medium, letter-spacing: -0.2em (inline style), sizes text-5xl md:text-7xl:
   - Line 1: "The ultimate geo" in primary color
   - Line 2: "map " in primary color, followed by "builder" with:
     - Gradient text: background-image: linear-gradient(135deg, #767676 0%, #D3D3D3 100%) with bg-clip-text text-transparent
     - Dotted selection box SVG absolutely positioned around the word (-inset-3 md:-inset-4), rotated -0.5deg, containing:
       - An irregular quadrilateral path: M5 5 L195 5 L195 88 L5 72 Z (bottom-right corner drops lower than bottom-left) — stroke #B0B0B0, strokeWidth 1.2, strokeDasharray 6 4
       - 4 corner dots: circles at (5,5), (195,5), (5,72), (195,88) — radius 3.5, fill #B0B0B0
       - 4 midpoint dots: circles at (100,5), (100,80), (5,38.5), (195,46.5) — radius 3, fill #B0B0B0
       - SVG viewBox: 0 0 200 95, preserveAspectRatio="none"

3. Subtext — mt-8, muted-foreground, text-base md:text-lg, max-w-lg, centered:
   Terra is how teams build maps and
   run spatial intelligence together.
   Design, collaborate, share — all in one place.

4. CTA button — mt-8, px-10 py-4, primary bg, primary-foreground text, font-semibold, rounded-full, shadow-lg shadow-primary/20

5. Video — mt-12, max-w-5xl, rounded-xl overflow-hidden, no drop shadow:
   src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260325_092310_5c71bab5-63cd-4a95-9390-cc6a1189d553.mp4"
   muted autoPlay loop playsInline

Layout: min-h-screen flex flex-col bg-background, hero content area is flex-1 flex flex-col items-center justify-center px-4 pt-8
