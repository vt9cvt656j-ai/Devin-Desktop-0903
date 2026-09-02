# Michael Design Library — sites-landing-pages

Curated UI/UX design prompts from the michael-design knowledge base (Motion Prompt Library). Each section is a complete, production-grade frontend design prompt with tech stack, styling and animation specs. 64 entries.

## Weblex Dark Hero — Landing Page [sites/11]

- Preview: https://motionsites.ai/assets/hero-weblex-preview-BoIbrUHI.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/11.gif

Create a dark, full-screen hero section with a background video and a transparent navbar.

Navbar:

Fixed at the top, fully transparent (no blur, no border, no background)

Left: Brand name "Weblex." with a green dot accent using the primary color

Center (desktop): Navigation links — Home, Features, Pricing, About — in muted foreground color, small text

Right (desktop): "Get Started" button — primary color background, dark text, rounded-full, small text

Mobile: Hamburger menu icon that toggles a dropdown with the same links and button

Hero Section:

Full viewport height (h-screen)

Background video playing on autoplay, loop, muted, playsInline, covering the full section with object-cover

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260221_085953_8463b46e-ba85-4bb7-912a-1feaf346e970.mp4

Video has a seamless loop transition: fades out to black starting 1.5 seconds before the end (reaching opacity 0 by 0.3s before the end), then fades back in over the first 1 second when the video restarts. Use requestAnimationFrame for smooth opacity updates.

No dark overlay on the video — full opacity

Content is aligned to the bottom of the screen with 100px bottom padding, centered horizontally, max-width 603px

Hero Content (bottom-aligned, centered):

Badge: A small pill/badge that says "Introducing Smart Website Builder" — styled with a border, rounded-full, small text, muted foreground color

Heading: "Turn your big idea into a stunning website" — 62px font size, font-medium, centered, line-height 1.1. Responsive: 48px on medium, 36px on small screens

Paragraph: "Fintech is its potential to promote financial inclusion. In many parts of the world, millions of people lack access to traditional banking services." — muted foreground color, centered, max-width 520px

Two buttons side by side:

"Get Started Now" — primary color background, dark text, rounded-full, with an ArrowUpRight icon on the left, 18px text, hover brightness effect

"See Pricing" — secondary (white) background, dark text, rounded-full, 18px text

Color Theme (dark mode only, HSL values in CSS variables):

--background: 240 67% 1% (near-black)

--foreground: 0 0% 100% (white)

--primary: 73 98% 57% (bright lime green)

--primary-foreground: 240 67% 1% (dark)

--secondary: 0 0% 100% (white)

--secondary-foreground: 240 67% 1% (dark)

--muted: 240 10% 12%

--muted-foreground: 0 0% 82% / 0.8

--border: 0 0% 100% / 0.1

Tech: React, TypeScript, Tailwind CSS, Lucide icons for ArrowUpRight and Menu/X icons.

## Space Voyage — Landing Page [sites/20]

- Preview: https://motionsites.ai/assets/hero-space-voyage-preview-eECLH3Yc.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/20.gif

Build a full-screen cinematic hero section for a space travel website using React, Vite, TypeScript, Tailwind CSS, and the motion/react (Framer Motion) library. Recreate every detail exactly as described below.

1. Fonts
Import Instrument Serif (italic) and Barlow (weights 300, 400, 500, 600) from Google Fonts:
@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap');
Register them in tailwind.config.ts:
fontFamily: {
  heading: ["'Instrument Serif'", "serif"],
  body: ["'Barlow'", "sans-serif"],
}
Set --radius: 9999px for fully rounded elements. Use an HSL-based color system where --background: 213 45% 67% (muted sky blue) and --foreground: 0 0% 100% (white).

2. Background Video
Use a full-screen <video> element positioned absolute inset-0 with object-cover, z-0, and these attributes: autoPlay loop muted playsInline preload="auto".
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260306_115329_5e00c9c5-4d69-49b7-94c3-9c31c60bb644.mp4
Poster image: /images/hero_bg.jpeg
Overlay: A div with absolute inset-0 bg-black/5 z-0 on top of the video.
In index.html, add preload hints in <head>:
<link rel="preload" as="image" href="/images/hero_bg.jpeg" type="image/jpeg" />
<link rel="preload" as="video" href="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260306_115329_5e00c9c5-4d69-49b7-94c3-9c31c60bb644.mp4" type="video/mp4" />

3. Liquid Glass CSS
Define two utility classes in index.css under @layer components:
.liquid-glass (light):
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
.liquid-glass-strong (heavy, for CTA buttons):
.liquid-glass-strong {
  background: rgba(255, 255, 255, 0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(50px);
  -webkit-backdrop-filter: blur(50px);
  border: none;
  box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15);
  position: relative;
  overflow: hidden;
}
Same ::before pseudo-element as .liquid-glass but with 0.5 and 0.2 alpha values instead of 0.45 and 0.15.

4. Navbar
Fixed position: fixed top-4 left-0 right-0 z-50, with px-8 lg:px-16. Contains:
Left: A logo image (h-12 w-12).
Center (desktop only): A liquid-glass rounded-full pill containing nav links: "Home", "Voyages", "Worlds", "Innovation", "Plan Launch" — each styled px-3 py-2 text-sm font-medium text-foreground/90 font-body.
Inside pill, last item: A solid white button bg-white text-black rounded-full px-3.5 py-1.5 text-sm font-medium font-body with text "Claim a Spot" and an ArrowUpRight icon (lucide-react, h-4 w-4).

5. Hero Content (centered)
Wrapper: flex-1 flex flex-col items-center justify-center text-center px-4 pt-24.
a) Badge:
A liquid-glass rounded-full px-1 py-1 container with:
A solid white pill: bg-white text-black rounded-full px-3 py-1 text-xs font-semibold font-body with text "New".
Adjacent text: text-sm text-foreground/90 pr-3 font-body — "Maiden Crewed Voyage to Mars Arrives 2026".
mb-2 bottom margin.
b) Heading:
Use a custom BlurText component (word-by-word blur-in animation from bottom). Props:
text="Venture Past Our Sky Across the Universe"
className="text-6xl md:text-7xl lg:text-[5.5rem] font-heading italic text-foreground leading-[0.8] max-w-2xl justify-center tracking-[-4px]"
delay={100}
animateBy="words"
direction="bottom"
The BlurText component splits text by words, uses IntersectionObserver to trigger, and animates each word with motion.span from {filter: 'blur(10px)', opacity: 0, y: 50} through {filter: 'blur(5px)', opacity: 0.5, y: -5} to {filter: 'blur(0px)', opacity: 1, y: 0} with stepDuration: 0.35 and staggered delay of 100ms per word.
c) Subheading:
A motion.p with classes mt-1 text-sm md:text-base text-white max-w-2xl font-body font-light leading-tight. Text: "Discover the universe in ways once unimaginable. Our pioneering vessels and breakthrough engineering bring deep-space exploration within reach—secure and extraordinary."
Animation: initial={{ filter: 'blur(10px)', opacity: 0, y: 20 }} → animate={{ filter: 'blur(0px)', opacity: 1, y: 0 }}, duration: 0.6, delay: 0.8.
d) CTA Buttons:
A motion.div with flex items-center gap-6 mt-4, same blur-in animation with delay: 1.1.
Primary: liquid-glass-strong rounded-full px-5 py-2.5 text-sm font-medium text-foreground font-body — "Start Your Voyage" + ArrowUpRight icon (h-5 w-5).
Secondary: Plain text button — "View Liftoff" + Play icon (h-4 w-4 fill-current).

6. Partners Bar (bottom)
Positioned at bottom: flex flex-col items-center gap-4 pb-8.
A liquid-glass rounded-full px-3.5 py-1 text-xs font-medium text-white font-body label: "Collaborating with top aerospace pioneers globally".
A row of 5 partner names: "Aeon", "Vela", "Apex", "Orbit", "Zeno" — each styled text-2xl md:text-3xl font-heading italic text-white tracking-tight, spaced gap-12 md:gap-16.

7. Z-Index Layering
Video + overlay: z-0
All content (navbar, hero, partners): wrapped in a relative z-10 container.
Navbar: z-50.

## 3D Story — Landing Page [sites/3d-story]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(21).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/3d-story.webp

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Veldara</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
<style>
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
html, body { overflow-x: hidden; }
body { font-family: 'Inter', sans-serif; background: #010101; color: #fff; }

.fixed { position: fixed; }
.absolute { position: absolute; }
.relative { position: relative; }
.inset-0 { top: 0; right: 0; bottom: 0; left: 0; }

/* Scroll Video */
#scroll-video-container {
  position: fixed; inset: 0; z-index: -10;
  background: #0a0a0a; top: -20%;
}
#scroll-video-container canvas,
#scroll-video-container video {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover;
}
#scroll-video-container .overlay { position: absolute; inset: 0; background: rgba(0,0,0,0.2); }

/* Particles */
#particles-canvas {
  position: fixed; inset: 0; width: 100%; height: 100%;
  pointer-events: none; z-index: 3;
}

/* Nav */
nav {
  position: fixed; top: 0; left: 0; right: 0; z-index: 50;
  display: flex; align-items: center; justify-content: space-between;
  padding: 1.25rem 2.5rem;
}
nav .logo { font-weight: 700; font-size: 1.25rem; color: #fff; letter-spacing: -0.025em; }
nav .nav-links { display: flex; align-items: center; gap: 1.5rem; }
nav .nav-links a { font-size: 0.875rem; color: #d1d5db; text-decoration: none; transition: color 0.2s; }
nav .nav-links a:hover { color: #fff; }
nav .social { display: flex; align-items: center; gap: 1rem; }
nav .social a { color: #d1d5db; transition: color 0.2s; }
nav .social a:hover { color: #fff; }
nav .social svg { width: 1.25rem; height: 1.25rem; }

/* Hero */
#hero {
  position: relative; height: 100vh; width: 100%; display: flex; flex-direction: column;
}
#hero .gradient-overlay {
  position: absolute; inset: 0;
  background: linear-gradient(to top, rgba(0,0,0,0.6), transparent, transparent);
}
#hero .content {
  position: relative; z-index: 10; flex: 1; display: flex; flex-direction: column;
  align-items: center; justify-content: flex-end; text-align: center;
  padding: 0 1.5rem 6rem;
}
#hero .subtitle { font-size: 0.875rem; color: #9ca3af; margin-bottom: 1rem; letter-spacing: 0.05em; }
#hero h1 { font-size: clamp(1.5rem, 5vw, 3.75rem); font-weight: 600; line-height: 1.15; max-width: 48rem; }
#hero h1 .underlined {
  position: relative; display: inline-block;
}
#hero h1 .underlined .line {
  position: absolute; bottom: 0.25rem; left: 0; width: 100%; height: 10px;
  background: #2C5C88; border-radius: 2px;
}
#hero h1 .underlined span { position: relative; }
#hero .ctas {
  display: flex; align-items: center; gap: 1rem; margin-top: 2.5rem; flex-wrap: wrap; justify-content: center;
}
#hero .code-box {
  display: flex; align-items: center; gap: 0.5rem;
  background: #1a1a1a; border: 1px solid rgba(55,65,81,0.5);
  border-radius: 0.5rem; padding: 0.875rem 2rem;
}
#hero .code-box .prompt { color: #2C5C88; font-family: monospace; font-size: 0.875rem; }
#hero .code-box code { font-size: 0.875rem; color: #e5e7eb; font-family: monospace; }
#hero .cta-btn {
  display: inline-flex; align-items: center; gap: 0.5rem;
  background: #2C5C88; color: #fff; font-weight: 500; border-radius: 0.5rem;
  padding: 0.875rem 2rem; font-size: 0.875rem; text-decoration: none; transition: background 0.2s;
}
#hero .cta-btn:hover { background: #3a7aad; }
#hero .bounce-arrow {
  position: relative; z-index: 10; display: flex; justify-content: center; padding-bottom: 2rem;
}
#hero .bounce-arrow svg { width: 1.5rem; height: 1.5rem; color: #6b7280; animation: bounce 1s infinite; }

@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-25%); }
}

/* Cards */
#fixed-cards {
  position: fixed; bottom: 0; left: 0; right: 0; z-index: 4;
  padding: 2rem 2.5rem; opacity: 0; pointer-events: none;
}
#fixed-cards .grid {
  max-width: 72rem; margin: 0 auto;
  display: grid; grid-template-columns: repeat(3, 1fr); gap: 2.5rem;
}
#fixed-cards .card h3 { font-size: 1.5rem; font-weight: 700; color: #fff; margin-bottom: 1rem; }
#fixed-cards .card p { color: #d1d5db; font-size: 0.875rem; line-height: 1.6; }

/* Section 3 */
#section-three {
  position: relative; min-height: 100vh; display: flex; align-items: flex-end;
  justify-content: center; padding: 0 2.5rem 8rem;
}
#section-three .inner {
  position: relative; z-index: 10; display: flex; flex-direction: column;
  align-items: center; text-align: center;
  opacity: 0; transform: translateY(32px); filter: blur(8px);
  transition: opacity 1s ease-out, transform 1s ease-out, filter 1s ease-out;
}
#section-three .inner.visible { opacity: 1; transform: translateY(0); filter: blur(0); }
#section-three .inner p { color: #d1d5db; font-size: 1rem; margin-bottom: 0.75rem; }
#section-three .inner h2 { font-size: clamp(1.875rem, 6vw, 4.5rem); font-weight: 700; }

/* Content wrapper */
#content { position: relative; z-index: 2; }

/* Responsive */
@media (max-width: 768px) {
  nav { padding: 1rem 1.5rem; }
  nav .nav-links { display: none; }
  #hero .content { padding-bottom: 5rem; }
  #hero h1 { font-size: 1.5rem; }
  #hero .ctas { flex-direction: column; }
  #fixed-cards .grid { grid-template-columns: 1fr; gap: 1.5rem; }
  #fixed-cards { padding: 1.5rem 1rem; }
  #section-three { padding-bottom: 5rem; }
}
</style>
</head>
<body>

<!-- Scroll Video Background -->
<div id="scroll-video-container">
  <canvas id="video-canvas"></canvas>
  <video id="video-fallback" muted playsinline preload="auto" crossorigin="anonymous"
    src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260616_212935_bbf608da-62d1-4f25-9be4-c346e4d09cc8.mp4"
  ></video>
  <div class="overlay"></div>
</div>

<!-- Particles -->
<canvas id="particles-canvas"></canvas>

<!-- Fixed Cards -->
<div id="fixed-cards">
  <div class="grid">
    <div class="card">
      <h3>Explore Veldara</h3>
      <p>Veldara merges the elegance of Svelte 5 with the depth of Three.js within easy reach. It's crafted to be robust and adaptable while remaining intuitive and simple to grasp.</p>
    </div>
    <div class="card">
      <h3>Unlock Three.js</h3>
      <p>The web is growing increasingly dimensional. At its heart, Veldara offers a composable declarative API for building performant Three.js experiences on the web.</p>
    </div>
    <div class="card">
      <h3>Connect Everything</h3>
      <p>Veldara ships with tooling for physics, XR, animation, layouting, model loading, and extensive utilities to make building compelling 3D apps for the web effortless.</p>
    </div>
  </div>
</div>

<!-- Navigation -->
<nav>
  <div style="display:flex;align-items:center;gap:2rem;">
    <span class="logo">veldara</span>
    <div class="nav-links">
      <a href="#">Guides</a>
      <a href="#">Journal</a>
    </div>
  </div>
  <div class="social">
    <a href="#"><svg fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg></a>
    <a href="#"><svg fill="currentColor" viewBox="0 0 24 24"><path d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286z"/></svg></a>
    <a href="#"><svg fill="currentColor" viewBox="0 0 24 24"><path d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723c-.951.555-2.005.959-3.127 1.184a4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.936 4.936 0 004.604 3.417 9.867 9.867 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.053 0 13.998-7.496 13.998-13.985 0-.21 0-.42-.015-.63A9.935 9.935 0 0024 4.59z"/></svg></a>
  </div>
</nav>

<!-- Main Content -->
<div id="content">
  <!-- Section 1: Hero -->
  <section id="hero">
    <div class="gradient-overlay"></div>
    <div class="content">
      <p class="subtitle">Our Purpose:</p>
      <h1>
        Instantly craft immersive
        <span class="underlined"><span class="line"></span><span>3D worlds</span></span>
        on the web.
      </h1>
      <div class="ctas">
        <div class="code-box">
          <span class="prompt">&gt;</span>
          <code>npm i @veldara/core</code>
        </div>
        <a href="#" class="cta-btn">Get Started <span>&rarr;</span></a>
      </div>
    </div>
    <div class="bounce-arrow">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5"/></svg>
    </div>
  </section>

  <!-- Spacer -->
  <div style="height:150vh;"></div>

  <!-- Cards Trigger Zone -->
  <div id="cards-trigger" style="height:200vh;"></div>

  <!-- Spacer -->
  <div style="height:100vh;"></div>

  <!-- Section 3 -->
  <section id="section-three">
    <div class="inner" id="section-three-inner">
      <p>Presenting</p>
      <h2>Veldara 8</h2>
    </div>
  </section>
</div>

<script>
(function() {
  // ===================== SCROLL VIDEO =====================
  const VIDEO_URL = 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260616_212935_bbf608da-62d1-4f25-9be4-c346e4d09cc8.mp4';
  const canvas = document.getElementById('video-canvas');
  const videoEl = document.getElementById('video-fallback');
  const ctx = canvas.getContext('2d');
  let frames = [];
  let framesReady = false;
  let lastFrameIndex = -1;
  let videoSeeking = false;

  function resizeCanvas() {
    const dpr = Math.min(devicePixelRatio, 2);
    const rect = canvas.getBoundingClientRect();
    const w = Math.round(rect.width * dpr);
    const h = Math.round(rect.height * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    lastFrameIndex = -1;
  }

  async function extractFrames() {
    try {
      const response = await fetch(VIDEO_URL, { mode: 'cors' });
      const blob = await response.blob();
      const objectUrl = URL.createObjectURL(blob);

      const video = document.createElement('video');
      video.muted = true;
      video.playsInline = true;
      video.crossOrigin = 'anonymous';
      video.preload = 'auto';
      video.src = objectUrl;

      await new Promise((resolve, reject) => {
        video.onloadedmetadata = () => resolve();
        video.onerror = () => reject();
        setTimeout(() => reject(), 15000);
      });

      const scale = Math.min(1, 1280 / video.videoWidth);
      const scaledWidth = Math.round(video.videoWidth * scale);
      const scaledHeight = Math.round(video.videoHeight * scale);
      const frameCount = Math.max(30, Math.min(120, Math.round(video.duration * 24)));

      for (let i = 0; i < frameCount; i++) {
        const time = (i / (frameCount - 1)) * (video.duration - 0.05);
        video.currentTime = time;
        await new Promise((resolve, reject) => {
          const onSeeked = () => { video.removeEventListener('seeked', onSeeked); resolve(); };
          video.addEventListener('seeked', onSeeked);
          setTimeout(() => { video.removeEventListener('seeked', onSeeked); reject(); }, 3000);
        });
        const bitmap = await createImageBitmap(video, { resizeWidth: scaledWidth, resizeHeight: scaledHeight });
        frames.push(bitmap);
      }

      if (frames.length > 0) {
        framesReady = true;
        canvas.style.visibility = 'visible';
        videoEl.style.display = 'none';
      }
      URL.revokeObjectURL(objectUrl);
    } catch(e) { /* fallback to video seeking */ }
  }

  function getScrollBounds() {
    const vh = window.innerHeight;
    return { start: vh * 0.5, end: document.documentElement.scrollHeight - vh };
  }

  function getProgress() {
    const { start, end } = getScrollBounds();
    const range = end - start;
    if (range <= 0) return 0;
    return Math.max(0, Math.min(1, (window.scrollY - start) / range));
  }

  function drawFrame(frame) {
    const cw = canvas.width, ch = canvas.height;
    const s = Math.max(cw / frame.width, ch / frame.height);
    const dw = frame.width * s, dh = frame.height * s;
    ctx.drawImage(frame, (cw - dw) / 2, (ch - dh) / 2, dw, dh);
  }

  function videoTick() {
    const progress = getProgress();
    if (framesReady && frames.length > 0) {
      const idx = Math.round(progress * (frames.length - 1));
      if (idx !== lastFrameIndex) {
        lastFrameIndex = idx;
        if (frames[idx]) drawFrame(frames[idx]);
      }
    } else if (videoEl.duration && isFinite(videoEl.duration) && videoEl.readyState >= 1) {
      const target = progress * videoEl.duration;
      if (!videoSeeking && Math.abs(videoEl.currentTime - target) > 0.001) {
        videoSeeking = true;
        videoEl.currentTime = target;
      }
    }
    requestAnimationFrame(videoTick);
  }

  videoEl.addEventListener('seeked', () => { videoSeeking = false; });
  videoEl.addEventListener('stalled', () => { videoSeeking = false; });
  videoEl.addEventListener('loadeddata', () => { videoEl.currentTime = 0; });
  canvas.style.visibility = 'hidden';

  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);
  requestAnimationFrame(videoTick);
  extractFrames();

  // ===================== PARTICLES =====================
  const pCanvas = document.getElementById('particles-canvas');
  const pCtx = pCanvas.getContext('2d');
  let particles = [];

  function resizeParticles() {
    pCanvas.width = window.innerWidth;
    pCanvas.height = window.innerHeight;
    createParticles();
  }

  function createParticles() {
    particles = [];
    const count = Math.floor((pCanvas.width * pCanvas.height) / 12000);
    for (let i = 0; i < count; i++) {
      particles.push({
        x: Math.random() * pCanvas.width,
        y: Math.random() * pCanvas.height,
        vx: (Math.random() - 0.5) * 0.3,
        vy: (Math.random() - 0.5) * 0.3,
        size: Math.random() * 1.5 + 0.5,
        opacity: Math.random() * 0.6 + 0.2
      });
    }
  }

  function animateParticles() {
    pCtx.clearRect(0, 0, pCanvas.width, pCanvas.height);
    for (const p of particles) {
      p.x += p.vx; p.y += p.vy;
      if (p.x < 0) p.x = pCanvas.width;
      if (p.x > pCanvas.width) p.x = 0;
      if (p.y < 0) p.y = pCanvas.height;
      if (p.y > pCanvas.height) p.y = 0;
      pCtx.beginPath();
      pCtx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
      pCtx.fillStyle = `rgba(255,255,255,${p.opacity})`;
      pCtx.fill();
    }
    requestAnimationFrame(animateParticles);
  }

  resizeParticles();
  window.addEventListener('resize', resizeParticles);
  animateParticles();

  // ===================== HERO FADE =====================
  function updateHeroOpacity() {
    const fade = Math.max(0, 1 - window.scrollY / (window.innerHeight * 0.3));
    document.getElementById('hero').style.opacity = fade;
  }
  window.addEventListener('scroll', updateHeroOpacity, { passive: true });

  // ===================== FIXED CARDS =====================
  const fixedCards = document.getElementById('fixed-cards');
  const cardsGrid = fixedCards.querySelector('.grid');

  function tickCards() {
    const trigger = document.getElementById('cards-trigger');
    const rect = trigger.getBoundingClientRect();
    const triggerTop = rect.top + window.scrollY;
    const triggerHeight = rect.height;
    const scrollY = window.scrollY;
    const vh = window.innerHeight;

    const start = triggerTop - vh * 0.5;
    const end = triggerTop + triggerHeight - vh * 0.3;
    const range = end - start;

    let progress = range > 0 ? (scrollY - start) / range : 0;
    progress = Math.max(0, Math.min(1, progress));

    const isActive = scrollY >= start - vh * 0.2 && scrollY <= end + vh * 0.3;
    const fadeIn = Math.min(1, Math.max(0, (scrollY - (start - vh * 0.2)) / (vh * 0.2)));
    const fadeOut = Math.min(1, Math.max(0, (end + vh * 0.3 - scrollY) / (vh * 0.3)));
    const containerOpacity = isActive ? Math.min(fadeIn, fadeOut) : 0;

    fixedCards.style.opacity = containerOpacity;
    fixedCards.style.pointerEvents = containerOpacity > 0.1 ? 'auto' : 'none';

    const isMobile = window.innerWidth < 768;
    const revealPct = progress * 130;
    if (isMobile) {
      cardsGrid.style.maskImage = `linear-gradient(to bottom, black ${revealPct}%, transparent ${revealPct + 20}%)`;
      cardsGrid.style.webkitMaskImage = `linear-gradient(to bottom, black ${revealPct}%, transparent ${revealPct + 20}%)`;
    } else {
      cardsGrid.style.maskImage = `linear-gradient(to right, black ${revealPct}%, transparent ${revealPct + 15}%)`;
      cardsGrid.style.webkitMaskImage = `linear-gradient(to right, black ${revealPct}%, transparent ${revealPct + 15}%)`;
    }

    requestAnimationFrame(tickCards);
  }
  requestAnimationFrame(tickCards);

  // ===================== SECTION 3 INTERSECTION =====================
  const sectionThreeInner = document.getElementById('section-three-inner');
  const observer = new IntersectionObserver(([entry]) => {
    if (entry.isIntersecting) {
      sectionThreeInner.classList.add('visible');
      observer.unobserve(sectionThreeInner);
    }
  }, { threshold: 0.15 });
  observer.observe(sectionThreeInner);
})();
</script>
</body>
</html>

## Acreage Farming — Landing Page [sites/acreage-farming-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/hero-acreage-farming-preview-DY4bc7ni.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/acreage-farming-hero.gif

Precision farming landing page with dark/light sections, hero video background, stats grid, logo marquee, and service cards.

## AeroCore — Landing Page [sites/aerocore]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(34).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/aerocore.webp

<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta
      name="description"
      content="EngineTech designs and manufactures custom propulsion systems for aerospace programs."
    />
    <title>EngineTech | Custom Aerospace Engines</title>
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/@phosphor-icons/web@2.1.1/src/regular/style.css" />
    <style>
:root {
  color-scheme: light;
  --font-sans: "Geist", "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --geist-background: #ffffff;
  --geist-foreground: #0a0a0a;
  --geist-muted: #666666;
  --hero-blue: #7191d0;
  --hero-blue-soft: #aab8d5;
  --hero-cloud: #ece9e6;
  --hero-bg-bottom: linear-gradient(180deg, var(--hero-blue) 0%, var(--hero-blue-soft) 55%, var(--hero-cloud) 100%);
  --hero-bg-top: linear-gradient(180deg, rgb(255 255 255 / 0.04), rgb(255 255 255 / 0.12));
  --hero-max-width: 1820px;
}

* {
  box-sizing: border-box;
}

html,
body {
  min-height: 100%;
}

body {
  margin: 0;
  background: var(--geist-background);
  color: var(--geist-foreground);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  text-rendering: geometricPrecision;
}

a {
  color: inherit;
  text-decoration: none;
}

.mission {
  position: relative;
  z-index: 40;
  min-height: 100vh;
  margin-top: -12vh;
  background: #ffffff;
  color: #161616;
}

.mission__inner {
  display: grid;
  grid-template-columns: minmax(240px, 0.95fr) minmax(0, 2fr);
  grid-template-rows: auto minmax(360px, 1fr);
  column-gap: clamp(56px, 8vw, 170px);
  row-gap: clamp(76px, 5vw, 104px);
  width: min(100% - 96px, var(--hero-max-width));
  min-height: 100vh;
  margin: 0 auto;
  padding: clamp(34px, 3vw, 54px) 0 clamp(32px, 4vw, 62px);
}

.mission__eyebrow {
  grid-column: 1;
  grid-row: 1;
  align-self: start;
  margin: 0;
  color: #202020;
  font-size: clamp(13px, 0.9vw, 16.8px);
  font-weight: 700;
  line-height: 1.22;
  letter-spacing: 0;
}

.mission__statement {
  grid-column: 2;
  grid-row: 1;
  align-self: start;
  max-width: 1180px;
}

.mission__statement h2 {
  margin: 0;
  color: #141414;
  font-size: clamp(29px, 1.95vw, 41.3px);
  font-weight: 260;
  line-height: 1.18;
  letter-spacing: 0;
}

.mission__button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: clamp(46px, 3.3vw, 72px);
  color: #171717;
  font-size: clamp(13.8px, 1vw, 18.3px);
  font-weight: 700;
  line-height: 1;
}

.mission__button span:last-child {
  display: inline-flex;
  align-items: center;
  min-height: clamp(58px, 3.65vw, 72px);
  padding: 0 clamp(18px, 1.35vw, 26px);
  border: 1px solid #c6c6c6;
  border-radius: 5px;
  box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.04);
}

.mission__button-icon {
  position: relative;
  display: inline-grid;
  place-items: center;
  width: clamp(58px, 3.65vw, 72px);
  aspect-ratio: 1;
  border-radius: 5px;
  background: #d8e8ff;
  transition:
    background 180ms ease,
    transform 180ms ease;
}

.mission__button-icon .ph {
  font-size: clamp(22px, 1.5vw, 28px);
  color: currentColor;
  display: block;
}

.mission__button:hover .mission__button-icon {
  background: #c7dcfb;
  transform: translate(2px, 2px);
}

.mission__support {
  grid-column: 1;
  grid-row: 2;
  align-self: start;
  max-width: 520px;
  margin: 0;
  color: #5f5f5f;
  font-size: clamp(23.7px, 1.56vw, 30.6px);
  font-weight: 370;
  line-height: 1.18;
  letter-spacing: 0;
}

.mission__media {
  grid-column: 2;
  grid-row: 2;
  align-self: start;
  width: 100%;
  aspect-ratio: 16 / 9;
  overflow: hidden;
  background: transparent;
}

.hero {
  position: relative;
  height: 180vh;
  min-height: 1238px;
  overflow: clip;
  background: var(--hero-blue);
}

.hero__background {
  position: sticky;
  top: 0;
  z-index: 0;
  height: 100vh;
  overflow: hidden;
  background:
    linear-gradient(
      180deg,
      var(--hero-top, #7191d0) 0%,
      var(--hero-mid, #aab8d5) 55%,
      var(--hero-bottom, #ece9e6) 100%
    );
}

.hero__bg-layer {
  position: absolute;
  inset: 0;
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  pointer-events: none;
}

.hero__bg-layer--bottom {
  background:
    radial-gradient(circle at 52% 26%, rgb(255 255 255 / 0.22), transparent 35%),
    linear-gradient(180deg, rgb(70 100 170 / 0.14), rgb(255 255 255 / 0));
}

.hero__bg-layer--top {
  z-index: 1;
  background: var(--hero-bg-top);
  mix-blend-mode: screen;
}

.hero__stars {
  position: absolute;
  inset: 0 0 auto;
  z-index: 2;
  height: 210px;
  pointer-events: none;
  background-image:
    radial-gradient(circle, rgb(255 255 255 / 0.78) 0 1px, transparent 1.8px),
    radial-gradient(circle, rgb(255 255 255 / 0.58) 0 1px, transparent 1.6px),
    radial-gradient(circle, rgb(255 255 255 / 0.68) 0 1px, transparent 1.7px);
  background-position:
    10% 24%,
    38% 16%,
    76% 32%;
  background-size:
    180px 94px,
    260px 120px,
    340px 150px;
  opacity: 0.45;
  animation: hero-stars-twinkle 4.8s ease-in-out infinite alternate;
}

@keyframes hero-stars-twinkle {
  0% { opacity: 0.18; filter: brightness(0.92); }
  50% { opacity: 0.58; filter: brightness(1.12); }
  100% { opacity: 0.34; filter: brightness(1); }
}

.hero__nav {
  position: fixed;
  top: 0;
  left: 50%;
  z-index: 100;
  display: grid;
  grid-template-columns: minmax(220px, 1fr) auto minmax(180px, 1fr);
  align-items: center;
  gap: 32px;
  width: min(100% - 96px, var(--hero-max-width));
  margin: 0;
  padding: 27px 16px 16px;
  color: #ffffff;
  transform: translate3d(-50%, 0, 0);
  transition:
    background-color 300ms ease,
    color 300ms ease,
    transform 300ms ease,
    box-shadow 300ms ease,
    border-color 300ms ease,
    padding 300ms ease,
    top 300ms ease;
  border: 1px solid transparent;
  border-radius: 0;
}

.hero__nav.nav--scroll-down {
  transform: translate3d(-50%, 16px, 0);
  background-color: rgba(255, 255, 255, 0.88);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  color: #111111;
  padding: 14px 24px;
  border-radius: 40px;
  border-color: rgba(0, 0, 0, 0.08);
  box-shadow:
    0 12px 30px -10px rgba(0, 0, 0, 0.08),
    0 4px 12px -5px rgba(0, 0, 0, 0.03);
}

.hero__nav.nav--scroll-down .brand__name { color: #111111; }
.hero__nav.nav--scroll-down .brand__mark { background: #111111; }
.hero__nav.nav--scroll-down .hero__links { color: rgba(17, 17, 17, 0.8); }
.hero__nav.nav--scroll-down .hero__links a { color: inherit; }
.hero__nav.nav--scroll-down .hero__links a:hover { color: #111111; }
.hero__nav.nav--scroll-down .hero__cta { background: #111111; color: #ffffff; box-shadow: none; }

.hero__nav.nav--scroll-up {
  transform: translate3d(-50%, -100px, 0);
  pointer-events: none;
}

.brand {
  display: inline-flex;
  align-items: center;
  justify-self: start;
  gap: 7px;
  min-width: 0;
}

.brand__mark {
  position: relative;
  display: grid;
  place-items: center;
  width: 29px;
  aspect-ratio: 1;
  overflow: hidden;
  border-radius: 50%;
  background: #ffffff;
  transition: background-color 300ms ease;
}

.brand__mark::before {
  content: "";
  position: absolute;
  inset: -8px;
  background: var(--hero-blue);
  clip-path: polygon(0 20%, 100% 8%, 100% 19%, 0 31%, 0 43%, 100% 31%, 100% 42%, 0 54%, 0 66%, 100% 54%, 100% 65%, 0 77%);
}

.brand__mark span { display: none; }

.brand__name {
  color: #ffffff;
  font-size: 24px;
  font-weight: 560;
  line-height: 1;
  letter-spacing: 0;
  transition: color 300ms ease;
}

.hero__links {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: clamp(22px, 2.55vw, 44px);
  color: rgb(255 255 255 / 0.9);
  font-size: 14px;
  font-weight: 600;
  line-height: 20px;
  white-space: nowrap;
  transition: color 300ms ease;
}

.hero__links a { transition: color 160ms ease; }
.hero__links a:hover { color: #ffffff; }

.hero__cta {
  justify-self: end;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 122px;
  min-height: 46px;
  padding: 0 17px;
  border-radius: 6px;
  background: rgb(233 240 255 / 0.9);
  color: #111111;
  font-size: 14px;
  font-weight: 600;
  line-height: 20px;
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.42);
  transition:
    background 160ms ease,
    transform 160ms ease,
    color 160ms ease;
}

.hero__cta:hover { background: #ffffff; transform: translateY(-1px); }

.hero__content {
  position: absolute;
  inset: 0;
  z-index: 1;
  display: grid;
  place-items: center;
  height: 100vh;
  width: min(100%, var(--hero-max-width));
  margin: 0 auto;
  pointer-events: none;
}

.hero__title {
  position: fixed;
  top: calc(50% - 56px - clamp(82px, 8vw, 126px));
  left: 4%;
  z-index: 10;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  margin: 0;
  width: 96vw;
  color: #ffffff;
  font-weight: 200;
  letter-spacing: 0;
  line-height: 0.88;
  pointer-events: none;
  transform: translate3d(0, var(--scroll-y, 0px), 0);
  will-change: transform, opacity;
}

.hero__title-line {
  display: block;
  font-size: clamp(144px, 18vw, 285px);
  white-space: nowrap;
}

.hero__title-line--one { position: relative; z-index: 10; }

.hero__title-row {
  position: fixed;
  top: calc(50% - 56px + clamp(50px, 4.5vw, 90px));
  left: 4%;
  z-index: 2;
  display: flex;
  align-items: baseline;
  gap: clamp(112px, 12vw, 224px);
  color: #ffffff;
  font-weight: 200;
  pointer-events: none;
  transform: translate3d(15vw, var(--scroll-y, 0px), 0);
  will-change: transform, opacity;
}

.hero__title-line--two,
.hero__title-line--three { position: relative; }
.hero__title-line--three { transform: translateX(112px); }

.engine-visual {
  position: fixed;
  z-index: 3;
  left: 50%;
  top: -15px;
  width: auto;
  height: calc((100% + 15px) * 1.4);
  max-width: calc(100vw - 96px);
  max-height: 1023px;
  aspect-ratio: 2 / 3;
  transform: translate3d(-50%, var(--scroll-y, 0px), 0);
  will-change: transform, opacity;
  filter: drop-shadow(0 28px 34px rgb(26 31 42 / 0.22));
}

.engine-visual__asset {
  display: block;
  width: auto;
  height: 100%;
  max-width: 100%;
  object-fit: contain;
  object-position: center bottom;
}

.hero__caption {
  position: fixed;
  z-index: 4;
  left: clamp(24px, 3.85vw, 78px);
  bottom: 28px;
  display: inline-flex;
  align-items: center;
  gap: 24px;
  max-width: min(170px, calc(50vw - 112px));
  margin: 0;
  color: rgb(42 42 42 / 0.58);
  font-size: 16px;
  font-weight: 400;
  line-height: 22px;
  letter-spacing: 0;
  transform: translate3d(0, var(--scroll-y, 0px), 0);
  will-change: transform, opacity;
}

.hero__caption::before {
  content: "";
  display: block;
  width: 1px;
  height: 44px;
  background: rgb(42 42 42 / 0.32);
}

.hero.is-past .hero__title,
.hero.is-past .hero__title-row,
.hero.is-past .hero__caption,
.hero.is-past .engine-visual {
  opacity: 0;
  pointer-events: none;
}

@media (max-width: 1180px) {
  .mission__inner {
    grid-template-columns: minmax(190px, 0.7fr) minmax(0, 1.7fr);
    column-gap: clamp(36px, 5vw, 72px);
    width: min(100% - 48px, var(--hero-max-width));
  }
  .mission__statement h2 { font-size: clamp(26px, 3.37vw, 38.2px); }
  .mission__support { font-size: clamp(20.6px, 2.3vw, 26px); }
  .hero__nav {
    grid-template-columns: auto 1fr auto;
    width: min(100% - 48px, var(--hero-max-width));
  }
  .hero__links { gap: 20px; font-size: 14px; }
  .brand__name { font-size: 22px; }
  .hero__cta { min-width: 122px; min-height: 46px; font-size: 14px; }
}

@media (max-width: 860px) {
  .mission { margin-top: -8vh; }
  .mission__inner {
    display: flex;
    flex-direction: column;
    gap: 44px;
    width: min(100% - 48px, var(--hero-max-width));
    min-height: auto;
    padding: 34px 0 40px;
  }
  .mission__statement { max-width: none; }
  .mission__statement h2 { font-size: clamp(26px, 7.65vw, 39.8px); line-height: 1.1; }
  .mission__button { margin-top: 34px; font-size: 13.8px; }
  .mission__button-icon { width: 56px; }
  .mission__support { max-width: 640px; margin: 52px 0 0; font-size: clamp(19.9px, 5.35vw, 26px); }
  .mission__media { margin-top: 8px; }
  .hero { height: 180vh; min-height: 1238px; }
  .hero__nav { grid-template-columns: 1fr auto; padding-top: 22px; }
  .hero__links { display: none; }
  .hero__content { height: 100vh; }
  .hero__title-line { font-size: clamp(102px, 31.5vw, 192px); }
  .hero__title { top: calc(50% - 56px - clamp(58px, 14vw, 90px)); left: 5%; }
  .hero__title-row { top: calc(50% - 56px + clamp(32px, 9vw, 63px)); left: 5%; transform: translate3d(10vw, var(--scroll-y, 0px), 0); }
  .engine-visual { top: -19px; height: calc((100% + 19px) * 1.4); max-height: 868px; }
  .hero__caption { right: 24px; bottom: 24px; max-width: min(170px, calc(100vw - 48px)); font-size: 16px; line-height: 22px; }
}

@media (max-width: 560px) {
  .mission__inner { width: min(100% - 32px, var(--hero-max-width)); }
  .mission__eyebrow { max-width: 240px; font-size: 12.2px; }
  .mission__statement h2 { font-size: clamp(23.7px, 7.19vw, 32.1px); }
  .mission__support { font-size: clamp(18.3px, 5.97vw, 23.7px); }
  .mission__media { aspect-ratio: 4 / 3; }
  .hero__nav { width: min(100% - 32px, var(--hero-max-width)); gap: 16px; }
  .brand__mark { width: 24px; }
  .brand__name { font-size: 17px; }
  .hero__cta { min-width: auto; min-height: 38px; padding: 0 12px; font-size: 13px; }
  .hero__title-line { font-size: clamp(111px, 38.4vw, 185px); }
  .hero__title-row { gap: clamp(72px, 20vw, 128px); transform: translate3d(10vw, var(--scroll-y, 0px), 0); }
  .hero__caption { display: none; }
}

.showcase-film {
  position: fixed;
  top: 0;
  left: 0;
  width: 1px;
  height: 1px;
  z-index: 45;
  overflow: hidden;
  background: #d7dde4;
  opacity: 0;
  pointer-events: none;
  will-change: top, left, width, height, border-radius, opacity;
}

.showcase-film__video {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center;
}

.showcase-film__overlay {
  position: absolute;
  inset: 0;
  background: #000;
  pointer-events: none;
}

.showcase {
  position: relative;
  z-index: 50;
  height: 600vh;
}

.showcase__sticky {
  position: sticky;
  top: 0;
  height: 100vh;
  background: transparent;
  overflow: visible;
}

.showcase__ui {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: end;
  padding: clamp(32px, 4vw, 72px) clamp(32px, 4.5vw, 80px);
  pointer-events: none;
  will-change: opacity;
}

.showcase__panels {
  grid-column: 1;
  position: relative;
  min-height: clamp(200px, 30vh, 400px);
  max-width: 640px;
}

.showcase__panel {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  opacity: 0;
  transform: translateY(16px);
  transition:
    opacity 460ms cubic-bezier(0.4, 0, 0.2, 1),
    transform 460ms cubic-bezier(0.4, 0, 0.2, 1);
  pointer-events: none;
}

.showcase__panel.is-active { opacity: 1; transform: none; pointer-events: auto; }

.showcase__panel-num {
  display: block;
  margin: 0 0 clamp(12px, 1.1vw, 22px);
  color: rgb(255 255 255 / 0.42);
  font-size: clamp(11px, 0.78vw, 14px);
  font-weight: 600;
  letter-spacing: 0.12em;
  line-height: 1;
  text-transform: uppercase;
}

.showcase__panel-title {
  margin: 0 0 clamp(14px, 1.3vw, 26px);
  color: #ffffff;
  font-size: clamp(38px, 4.4vw, 80px);
  font-weight: 200;
  line-height: 1.07;
  letter-spacing: -0.022em;
}

.showcase__panel-desc {
  max-width: 490px;
  margin: 0;
  color: rgb(255 255 255 / 0.58);
  font-size: clamp(14px, 1.05vw, 18px);
  font-weight: 400;
  line-height: 1.6;
}

.showcase__tabs-nav {
  grid-column: 2;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: clamp(10px, 0.9vw, 20px);
  padding-left: clamp(36px, 5vw, 120px);
  pointer-events: auto;
}

.showcase__tab {
  display: flex;
  align-items: center;
  gap: clamp(8px, 0.7vw, 14px);
  color: rgb(255 255 255 / 0.28);
  font-size: clamp(12px, 0.82vw, 15px);
  font-weight: 500;
  line-height: 1;
  white-space: nowrap;
  cursor: default;
  user-select: none;
  transition: color 320ms ease;
}

.showcase__tab.is-active { color: #ffffff; }

.showcase__tab-bar {
  display: block;
  flex-shrink: 0;
  width: 1px;
  height: 14px;
  background: currentColor;
  opacity: 0;
  transition: opacity 320ms ease;
}

.showcase__tab.is-active .showcase__tab-bar { opacity: 1; }
.showcase__tab-name { transition: color 320ms ease; }

.showcase__tab-num {
  font-weight: 600;
  font-size: clamp(11px, 0.72vw, 13px);
  color: rgb(255 255 255 / 0.38);
  transition: color 320ms ease;
}

.showcase__tab.is-active .showcase__tab-num { color: rgb(255 255 255 / 0.65); }

@media (max-width: 860px) {
  .showcase__ui { grid-template-columns: 1fr; }
  .showcase__tabs-nav { display: none; }
  .showcase__panel-title { font-size: clamp(30px, 8vw, 54px); }
}

@media (max-width: 560px) {
  .showcase__ui { padding: 28px 24px; }
  .showcase__panel-title { font-size: clamp(26px, 9vw, 42px); }
  .showcase__panel-desc { font-size: 14px; }
}

.capabilities {
  position: relative;
  z-index: 70;
  min-height: 100vh;
  padding: clamp(34px, 4vw, 72px) clamp(16px, 3.8vw, 72px);
  background: #f7f8f8;
  color: #111111;
}

.capabilities__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 32px;
  max-width: var(--hero-max-width);
  margin: 0 auto clamp(24px, 3vw, 42px);
}

.capabilities__intro { max-width: 860px; }

.capabilities__intro h2 {
  max-width: 920px;
  margin: 0;
  color: #111111;
  font-size: clamp(29px, 3.2vw, 54px);
  font-weight: 300;
  letter-spacing: 0;
  line-height: 1.08;
}

.capabilities__intro p {
  max-width: 760px;
  margin: 18px 0 0;
  color: #677070;
  font-size: clamp(14px, 1vw, 17px);
  font-weight: 400;
  line-height: 1.62;
}

.capabilities__button {
  flex: 0 0 auto;
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 48px;
  padding: 0 20px;
  border: 1px solid rgb(17 17 17 / 0.1);
  border-radius: 999px;
  background: rgb(255 255 255 / 0.78);
  color: #111111;
  font-size: 14px;
  font-weight: 700;
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.95),
    0 18px 44px rgb(31 44 44 / 0.08);
}

.capabilities__button .ph { font-size: 18px; }

.capabilities__grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr);
  gap: clamp(14px, 1.25vw, 22px);
  max-width: var(--hero-max-width);
  min-height: clamp(620px, 72vh, 780px);
  margin: 0 auto;
}

.capabilities__stack {
  display: grid;
  grid-template-rows: minmax(210px, 0.74fr) minmax(270px, 1fr);
  gap: clamp(14px, 1.25vw, 22px);
  min-width: 0;
}

.capabilities__stack--systems { grid-template-rows: minmax(420px, 1.45fr) auto; }

.cap-card {
  position: relative;
  overflow: hidden;
  border: 1px solid rgb(18 35 35 / 0.09);
  border-radius: 18px;
  background: #ffffff;
  box-shadow: 0 22px 60px rgb(21 34 34 / 0.08);
}

.cap-card--tall, .cap-card--metric, .cap-card--tools { min-height: 0; }
.cap-card--media, .cap-card--metric { color: #ffffff; background: #dce3e3; }

.cap-card__video {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center;
  transform: scale(1.02);
}

.cap-card__shade {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(180deg, rgb(5 12 14 / 0.3), transparent 34%),
    linear-gradient(0deg, rgb(5 12 14 / 0.78), transparent 48%);
}

.cap-card__light {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(135deg, rgb(255 255 255 / 0.45), rgb(255 255 255 / 0.34)),
    linear-gradient(0deg, rgb(247 248 248 / 0.36), transparent 62%);
}

.cap-card__label {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  color: rgb(255 255 255 / 0.78);
  font-size: 11px;
  font-weight: 760;
  letter-spacing: 0.18em;
  line-height: 1;
  text-transform: uppercase;
}

.cap-card__label--left { justify-content: flex-start; padding: 0; color: #758080; }

.cap-card__timeline {
  position: absolute;
  z-index: 1;
  right: 20px;
  bottom: 20px;
  left: 20px;
  display: grid;
  gap: 12px;
}

.cap-card__timeline div {
  display: grid;
  grid-template-columns: 58px 16px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  color: rgb(255 255 255 / 0.76);
  font-size: 12px;
  line-height: 1.2;
}

.cap-card__timeline b { display: block; width: 5px; height: 5px; border-radius: 50%; background: rgb(255 255 255 / 0.62); }
.cap-card__timeline strong { min-width: 0; color: #ffffff; font-size: clamp(13px, 0.95vw, 15px); font-weight: 650; }
.cap-card__timeline em { color: rgb(255 255 255 / 0.58); font-style: normal; white-space: nowrap; }

.cap-card--quote,
.cap-card--contact {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 24px;
  background:
    linear-gradient(135deg, rgb(255 255 255 / 0.72), rgb(238 244 244 / 0.86)),
    #edf2f2;
}

.cap-card--video-panel > :not(.cap-card__video, .cap-card__light, .cap-card__shade) { position: relative; z-index: 1; }

.cap-card--quote blockquote { margin: clamp(22px, 2.4vw, 34px) 0 20px; color: #263030; font-size: clamp(15px, 1vw, 18px); line-height: 1.62; }
.cap-card--quote p, .cap-card--contact p { margin: 0; color: #6b7676; font-size: 14px; line-height: 1.5; }
.cap-card--quote strong { display: block; color: #111111; font-size: 15px; }

.cap-card--metric { display: block; min-height: 320px; }

.cap-card__metric {
  position: absolute;
  inset: 0;
  z-index: 1;
  width: 100%;
  height: 100%;
  text-align: center;
  text-shadow: 0 12px 32px rgb(0 0 0 / 0.3);
}

.cap-card__metric strong {
  position: absolute;
  top: 50%;
  left: 50%;
  font-size: clamp(82px, 7.4vw, 134px);
  font-weight: 220;
  letter-spacing: 0;
  line-height: 0.9;
  transform: translate(-50%, -50%);
}

.cap-card__metric span {
  position: absolute;
  right: 24px;
  bottom: 24px;
  left: 24px;
  color: rgb(255 255 255 / 0.82);
  font-size: clamp(14px, 1.05vw, 18px);
  line-height: 1.4;
}

.cap-card--tools {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 0 0 clamp(20px, 2vw, 28px);
  background:
    linear-gradient(135deg, rgb(255 255 255 / 0.72), rgb(231 238 238 / 0.9)),
    #eef3f3;
}

.cap-card--tools .cap-card__label { color: #758080; }
.cap-card--tools-media { min-height: 420px; padding-bottom: 0; background: transparent; }
.cap-card--tools-media .cap-card__label { color: rgb(255 255 255 / 0.82); }
.cap-card--tools-media .cap-card__shade {
  background:
    linear-gradient(180deg, rgb(5 12 14 / 0.18), transparent 34%),
    linear-gradient(0deg, rgb(5 12 14 / 0.32), transparent 56%);
}

.tool-marquee {
  display: grid;
  gap: 14px;
  overflow: hidden;
  padding: 26px 0 8px;
  mask-image: linear-gradient(to right, transparent, #000 9%, #000 91%, transparent);
}

.tool-marquee__row { display: flex; width: max-content; gap: 12px; }

.tool-marquee__row span {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 54px;
  padding: 0 16px;
  border: 1px solid rgb(34 52 52 / 0.1);
  border-radius: 14px;
  background: rgb(255 255 255 / 0.78);
  color: #2c3838;
  font-size: 13px;
  font-weight: 700;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.9);
}

.cap-card--tools-media .tool-marquee__row span {
  border-color: rgb(255 255 255 / 0.2);
  background: rgb(255 255 255 / 0.18);
  color: #ffffff;
  backdrop-filter: blur(10px);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.24);
}

.tool-marquee__row .ph { font-size: 20px; }
.tool-marquee__row--left { animation: marquee-left 24s linear infinite; }
.tool-marquee__row--right { animation: marquee-right 28s linear infinite; }

@keyframes marquee-left { from { transform: translateX(0); } to { transform: translateX(-50%); } }
@keyframes marquee-right { from { transform: translateX(-50%); } to { transform: translateX(0); } }

.cap-card--contact {
  min-height: 118px;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 20px 76px 20px 24px;
}

.cap-card--contact a:not(.cap-card__icon-button) {
  display: inline-block;
  margin: 14px 0 6px;
  color: #111111;
  font-size: clamp(18px, 1.45vw, 24px);
  font-weight: 360;
  letter-spacing: 0;
  line-height: 1.05;
}

.cap-card__icon-button {
  position: absolute;
  top: 50%;
  right: 16px;
  z-index: 2;
  display: inline-grid;
  place-items: center;
  width: 42px;
  height: 42px;
  border: 1px solid rgb(17 17 17 / 0.1);
  border-radius: 50%;
  background: #111111;
  color: #ffffff;
  transform: translateY(-50%);
}

.cap-card__icon-button .ph { font-size: 19px; }

@media (max-width: 1080px) {
  .capabilities__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); min-height: auto; }
  .cap-card--tall { min-height: 620px; }
  .capabilities__stack:last-child { grid-column: 1 / -1; grid-template-columns: repeat(2, minmax(0, 1fr)); grid-template-rows: minmax(260px, 1fr); }
}

@media (max-width: 760px) {
  .capabilities__header { flex-direction: column; }
  .capabilities__button { width: 100%; }
  .capabilities__grid, .capabilities__stack, .capabilities__stack:last-child { grid-template-columns: 1fr; grid-template-rows: auto; }
  .cap-card--tall { min-height: 560px; }
  .cap-card__timeline div { grid-template-columns: 52px 14px minmax(0, 1fr); }
  .cap-card__timeline em { grid-column: 3; white-space: normal; }
}

.stats {
  position: relative;
  z-index: 80;
  min-height: 100vh;
  padding: clamp(44px, 5vw, 86px) clamp(16px, 3.8vw, 72px) clamp(54px, 5vw, 90px);
  background:
    radial-gradient(circle at 78% 18%, rgb(113 145 208 / 0.18), transparent 34%),
    radial-gradient(circle at 18% 88%, rgb(170 184 213 / 0.11), transparent 28%),
    linear-gradient(180deg, #111414 0%, #171a1a 100%);
  color: #f7f8f8;
}

.stats__header {
  display: grid;
  grid-template-columns: minmax(0, 1.08fr) minmax(320px, 0.72fr);
  gap: clamp(32px, 6vw, 120px);
  max-width: var(--hero-max-width);
  margin: 0 auto clamp(34px, 4.5vw, 72px);
}

.stats__title-wrap h2 {
  max-width: 920px;
  margin: 0;
  color: #f7f8f8;
  font-size: clamp(29px, 3.2vw, 54px);
  font-weight: 300;
  letter-spacing: 0;
  line-height: 1.08;
}

.stats__summary {
  align-self: start;
  margin: 0;
  color: rgb(247 248 248 / 0.8);
  font-size: clamp(18px, 1.65vw, 28px);
  font-weight: 360;
  line-height: 1.34;
  opacity: 0;
  transform: translateY(14px);
  transition: opacity 420ms ease, transform 420ms ease;
}

.stats__summary.is-visible { opacity: 1; transform: none; }

.stats__tabs {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0;
  max-width: var(--hero-max-width);
  margin: 0 auto;
  border-bottom: 1px solid rgb(255 255 255 / 0.14);
}

.stats__tab {
  position: relative;
  min-height: 58px;
  padding: 0 20px 18px 0;
  border: 0;
  background: transparent;
  color: rgb(247 248 248 / 0.5);
  font: inherit;
  font-size: clamp(14px, 1.22vw, 22px);
  font-weight: 430;
  letter-spacing: 0;
  text-align: left;
  cursor: pointer;
  transition: color 220ms ease;
}

.stats__tab::after {
  content: "";
  position: absolute;
  right: 16px;
  bottom: -1px;
  left: 0;
  height: 4px;
  background: linear-gradient(90deg, var(--hero-blue), #aab8d5);
  transform: scaleX(0);
  transform-origin: left;
  transition: transform 360ms cubic-bezier(0.22, 1, 0.36, 1);
}

.stats__tab.is-active { color: #ffffff; }
.stats__tab.is-active::after { transform: scaleX(1); }

.stats__chart {
  position: relative;
  max-width: var(--hero-max-width);
  min-height: clamp(520px, 58vh, 680px);
  margin: clamp(28px, 3vw, 48px) auto 0;
  padding: 0 0 22px;
  overflow: hidden;
  border: 1px solid rgb(255 255 255 / 0.08);
  border-radius: 20px;
  background-color: rgb(255 255 255 / 0.025);
  background-image:
    repeating-linear-gradient(
      to right,
      transparent 0,
      transparent calc(10% - 1px),
      rgb(255 255 255 / 0.07) calc(10% - 1px),
      rgb(255 255 255 / 0.07) 10%
    );
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.08),
    0 24px 70px rgb(0 0 0 / 0.18);
}

.stats__chart-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: clamp(18px, 2vw, 28px);
  border-bottom: 1px solid rgb(255 255 255 / 0.08);
  background: rgb(255 255 255 / 0.025);
}

.stats__chart-head span, .stats__chart-head strong { font-size: clamp(12px, 0.86vw, 14px); line-height: 1; text-transform: uppercase; }
.stats__chart-head span { color: #ffffff; font-weight: 760; letter-spacing: 0.16em; }
.stats__chart-head strong { color: rgb(247 248 248 / 0.48); font-weight: 620; letter-spacing: 0.12em; }

.stats__axis {
  display: grid;
  grid-template-columns: minmax(180px, 0.27fr) minmax(0, 1fr);
  gap: clamp(18px, 2vw, 34px);
  padding: 14px clamp(24px, 2.4vw, 42px) 0;
  color: rgb(247 248 248 / 0.42);
  font-size: clamp(11px, 0.84vw, 14px);
}

.stats__axis div { display: grid; grid-template-columns: repeat(11, minmax(0, 1fr)); }
.stats__axis div span { text-align: left; }
.stats__axis div span:last-child { text-align: right; }

.stats__bars { display: grid; gap: clamp(16px, 2vh, 26px); padding: clamp(26px, 3vw, 48px) clamp(24px, 2.4vw, 42px) 0; }

.stats__bar-row {
  display: grid;
  grid-template-columns: minmax(180px, 0.27fr) minmax(0, 1fr);
  align-items: center;
  gap: clamp(18px, 2vw, 34px);
  opacity: 0;
  transform: translateY(18px);
}

.stats__chart.is-ready .stats__bar-row { animation: stats-row-in 520ms cubic-bezier(0.22, 1, 0.36, 1) forwards; animation-delay: var(--bar-delay); }

.stats__bar-label strong, .stats__bar-label span { display: block; }
.stats__bar-label strong { color: #ffffff; font-size: clamp(15px, 1.1vw, 19px); font-weight: 680; line-height: 1.2; }
.stats__bar-label span { margin-top: 5px; color: rgb(247 248 248 / 0.48); font-size: clamp(12px, 0.86vw, 14px); line-height: 1.35; }

.stats__track {
  position: relative;
  height: clamp(48px, 5.4vh, 64px);
  overflow: hidden;
  border-radius: 0;
  background: rgb(255 255 255 / 0.055);
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.075), 0 12px 32px rgb(0 0 0 / 0.16);
}

.stats__range {
  position: absolute;
  top: 9px;
  bottom: 9px;
  left: var(--range-start);
  width: var(--range-width);
  border: 1px solid rgb(170 184 213 / 0.22);
  background: linear-gradient(90deg, rgb(113 145 208 / 0.05), rgb(170 184 213 / 0.14), rgb(113 145 208 / 0.05));
  opacity: 0;
  transform: scaleX(0.6);
  transform-origin: left;
}

.stats__chart.is-ready .stats__range { animation: stats-range-in 620ms cubic-bezier(0.22, 1, 0.36, 1) forwards; animation-delay: calc(var(--bar-delay) + 60ms); }

.stats__bar {
  position: relative;
  z-index: 1;
  width: var(--bar-value);
  height: 100%;
  background: linear-gradient(90deg, rgb(113 145 208 / 0.62) 0%, #8fb0ef 62%, #d6e3ff 100%);
  box-shadow: 0 0 34px rgb(113 145 208 / 0.24);
  transform: scaleX(0);
  transform-origin: left;
}

.stats__chart.is-ready .stats__bar { animation: stats-fill 900ms cubic-bezier(0.22, 1, 0.36, 1) forwards; animation-delay: calc(var(--bar-delay) + 110ms); }

.stats__value { position: absolute; z-index: 3; top: 50%; right: 18px; color: #ffffff; font-size: clamp(14px, 1vw, 18px); font-weight: 740; transform: translateY(-50%); }

.stats__trace { position: absolute; inset: 0; z-index: 2; pointer-events: none; }

.stats__trace i {
  position: absolute;
  top: var(--point-y);
  left: var(--point-x);
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: radial-gradient(circle, rgb(255 255 255 / 0.95) 0 8%, rgb(214 227 255 / 0.42) 9% 22%, transparent 58%);
  filter: blur(0.1px);
  opacity: 0;
  transform: translate(-50%, -50%) scale(0.2);
}

.stats__trace i::before, .stats__trace i::after {
  content: "";
  position: absolute;
  top: 50%;
  left: 50%;
  border-radius: 999px;
  background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.72), transparent);
  transform: translate(-50%, -50%) rotate(var(--spark-rotate, 0deg));
}

.stats__trace i::before { width: 24px; height: 1px; }
.stats__trace i::after { width: 1px; height: 18px; background: linear-gradient(180deg, transparent, rgb(170 184 213 / 0.62), transparent); }

.stats__spark--1 { --spark-rotate: 22deg; width: 14px; height: 14px; }
.stats__spark--2 { --spark-rotate: -18deg; width: 11px; height: 11px; }

.stats__chart.is-ready .stats__trace i { animation: stats-point-in 420ms cubic-bezier(0.22, 1, 0.36, 1) forwards; animation-delay: calc(var(--bar-delay) + 260ms + var(--point-delay)); }

@keyframes stats-row-in { to { opacity: 1; transform: none; } }
@keyframes stats-fill { to { transform: scaleX(1); } }
@keyframes stats-range-in { to { opacity: 1; transform: scaleX(1); } }
@keyframes stats-point-in { to { opacity: 0.86; transform: translate(-50%, -50%) scale(1); } }

@media (max-width: 980px) {
  .stats__header { grid-template-columns: 1fr; }
  .stats__tabs { display: flex; overflow-x: auto; }
  .stats__tab { flex: 0 0 min(260px, 76vw); }
  .stats__bar-row { grid-template-columns: 1fr; gap: 10px; }
  .stats__axis { grid-template-columns: 1fr; }
  .stats__axis > span { display: none; }
}

@media (max-width: 620px) {
  .stats__title-wrap h2 { font-size: clamp(26px, 8vw, 42px); }
  .stats__chart { min-height: auto; padding-bottom: 46px; }
  .stats__axis div { grid-template-columns: repeat(6, 1fr); }
  .stats__axis div span:nth-child(even) { display: none; }
}

.video-stories {
  position: relative;
  z-index: 90;
  min-height: 100vh;
  padding: clamp(46px, 5vw, 88px) 0 clamp(44px, 4vw, 74px);
  overflow: hidden;
  background: #f7f8f8;
  color: #111111;
}

.video-stories__header { width: min(100% - 96px, 900px); margin: 0 auto clamp(38px, 4vw, 74px); }
.video-stories__header h2 { margin: 0; color: #111111; font-size: clamp(38px, 4.4vw, 76px); font-weight: 300; letter-spacing: 0; line-height: 1.08; }
.video-stories__header p { max-width: 720px; margin: 22px 0 0; color: #697272; font-size: clamp(16px, 1.25vw, 21px); font-weight: 420; line-height: 1.55; }

.video-stories__rail {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(520px, 34vw);
  gap: clamp(28px, 3vw, 54px);
  overflow-x: auto;
  overscroll-behavior-x: contain;
  scroll-snap-type: x mandatory;
  padding: 0 max(48px, calc((100vw - var(--hero-max-width)) / 2 + 48px)) 36px;
  scrollbar-width: none;
}

.video-stories__rail::-webkit-scrollbar { display: none; }

.story-card {
  scroll-snap-align: center;
  min-width: 0;
  opacity: 0.54;
  transform: translateY(10px);
  transition: opacity 260ms ease, transform 260ms ease;
}

.story-card:hover, .story-card:focus-within { opacity: 1; transform: none; }

.story-card__media {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 16 / 9;
  border-radius: 12px;
  background: #dfe5e6;
  object-fit: cover;
  object-position: center;
  box-shadow: 0 18px 48px rgb(21 34 34 / 0.1);
}

.story-card__content { padding: 24px 28px 0; }
.story-card__content p { margin: 0 0 12px; color: #111111; font-size: 15px; font-weight: 760; line-height: 1; }
.story-card__content h3 { max-width: 680px; margin: 0; color: #252b2b; font-size: clamp(18px, 1.22vw, 24px); font-weight: 520; letter-spacing: 0; line-height: 1.38; }
.story-card__content span { display: block; margin-top: 14px; color: #858d8d; font-size: 14px; line-height: 1.4; }

.video-stories__footer { display: flex; align-items: center; gap: 8px; width: min(100% - 96px, 900px); margin: 28px auto 0; }
.video-stories__footer span { display: block; width: 56px; height: 4px; border-radius: 999px; background: #cfd4d4; }
.video-stories__footer span:nth-child(3) { width: 320px; background: #111111; }
.video-stories__footer strong { margin-left: 18px; color: #7a8282; font-size: 14px; font-weight: 650; letter-spacing: 0.02em; }

@media (max-width: 860px) {
  .video-stories__header, .video-stories__footer { width: min(100% - 48px, 900px); }
  .video-stories__rail { grid-auto-columns: minmax(320px, 82vw); padding: 0 24px 30px; }
  .story-card { opacity: 1; transform: none; }
}

@media (max-width: 560px) {
  .video-stories__header, .video-stories__footer { width: min(100% - 32px, 900px); }
  .story-card__content { padding: 18px 4px 0; }
  .video-stories__footer span:nth-child(3) { width: 150px; }
}

.site-footer { position: relative; z-index: 100; overflow: hidden; background: #000000; color: #ffffff; }

.footer-dots { position: relative; height: 120px; overflow: hidden; background: #000000; }

.footer-dots__line {
  position: absolute;
  left: 0;
  top: 50%;
  width: 200%;
  height: 70px;
  opacity: 0.75;
  background-image:
    radial-gradient(circle, rgb(255 255 255 / 0.55) 1.5px, transparent 2px),
    radial-gradient(circle, rgb(255 255 255 / 0.35) 1px, transparent 1.5px),
    radial-gradient(circle, rgb(255 255 255 / 0.45) 1.2px, transparent 1.8px);
  background-position: 0 8px, 24px 22px, 48px 14px;
  background-size: 72px 38px, 110px 44px, 160px 52px;
  animation: footerDotsMove 18s linear infinite;
  transform: translateY(-50%);
}

@keyframes footerDotsMove { from { transform: translate3d(0, -50%, 0); } to { transform: translate3d(-50%, -50%, 0); } }

.site-footer__inner { width: min(100% - 96px, var(--hero-max-width)); margin: 0 auto; padding: clamp(34px, 4vw, 66px) 0 clamp(18px, 2vw, 34px); }

.site-footer__top {
  display: grid;
  grid-template-columns: minmax(320px, 1.25fr) repeat(3, minmax(150px, 0.42fr));
  gap: clamp(28px, 4vw, 76px);
  min-height: clamp(220px, 24vw, 330px);
}

.site-footer__top h2 { max-width: 680px; margin: 0; color: #ffffff; font-size: clamp(34px, 3.5vw, 62px); font-weight: 220; letter-spacing: 0; line-height: 1.06; }

.site-footer__nav { display: flex; flex-direction: column; align-items: flex-start; gap: clamp(14px, 1.35vw, 22px); }
.site-footer__nav a { color: rgb(255 255 255 / 0.88); font-size: 16px; font-weight: 650; line-height: 1.1; transition: color 180ms ease, transform 180ms ease; }
.site-footer__nav a:hover { color: #ffffff; transform: translateX(3px); }

.site-footer__brand-row { width: 100%; margin-top: clamp(18px, 3vw, 46px); }
.site-footer__brand { display: flex; align-items: center; width: 100%; min-width: 0; color: #ffffff; }

.site-footer__mark {
  position: relative;
  flex: 0 0 clamp(58px, 6.1vw, 118px);
  aspect-ratio: 1;
  margin-right: clamp(14px, 1.6vw, 28px);
  overflow: hidden;
  border-radius: 50%;
  background: #ffffff;
}

.site-footer__mark::before {
  content: "";
  position: absolute;
  inset: -18%;
  background: #000000;
  clip-path: polygon(0 20%, 100% 8%, 100% 19%, 0 31%, 0 43%, 100% 31%, 100% 42%, 0 54%, 0 66%, 100% 54%, 100% 65%, 0 77%);
}

.site-footer__brand span:last-child { display: block; flex: 1 1 auto; min-width: 0; font-size: clamp(58px, 11.1vw, 214px); font-weight: 760; letter-spacing: -0.055em; line-height: 0.78; white-space: nowrap; }

.site-footer__legal { display: flex; flex-wrap: wrap; justify-content: flex-start; gap: 8px 18px; margin-top: clamp(14px, 1.4vw, 24px); color: rgb(255 255 255 / 0.52); font-size: 9px; line-height: 1.35; }
.site-footer__legal p { margin: 0; }
.site-footer__legal a { color: inherit; }
.site-footer__legal a:hover { color: #ffffff; }

@media (max-width: 980px) {
  .site-footer__inner { width: min(100% - 48px, var(--hero-max-width)); }
  .site-footer__top { grid-template-columns: 1fr 1fr; }
  .site-footer__top h2 { grid-column: 1 / -1; }
}

@media (max-width: 560px) {
  .site-footer__inner { width: min(100% - 32px, var(--hero-max-width)); }
  .site-footer__top { grid-template-columns: 1fr; min-height: auto; }
  .site-footer__nav a { font-size: 15px; }
  .site-footer__mark { flex-basis: clamp(38px, 12vw, 58px); }
  .site-footer__brand span:last-child { font-size: clamp(45px, 18vw, 84px); }
}
    </style>
  </head>
  <body>
    <main>
      <engine-hero></engine-hero>

      <section class="mission" id="company" aria-labelledby="mission-title" data-section="mission">
        <div class="mission__inner">
          <p class="mission__eyebrow">The Name Reflects Our Mission</p>

          <div class="mission__statement">
            <h2 id="mission-title">
              Demand for resilient propulsion is rising as aerospace programs move faster, fly farther,
              and require engines built with absolute precision.
            </h2>

            <a class="mission__button" href="#technology">
              <span class="mission__button-icon" aria-hidden="true">
                <i class="ph ph-arrow-elbow-down-right"></i>
              </span>
              <span>Discover Our Story</span>
            </a>
          </div>

          <p class="mission__support">
            Our name, EngineTech, reflects our commitment to moving advanced aircraft and spacecraft from
            ambitious concepts to dependable flight-ready power.
          </p>

          <div class="mission__media" aria-label="EngineTech propulsion systems in motion"></div>
        </div>
      </section>

      <section class="showcase" id="technology" aria-label="Technology highlights"></section>

      <section class="capabilities" id="solutions" aria-labelledby="capabilities-title">
        <div class="capabilities__header">
          <div class="capabilities__intro">
            <h2 id="capabilities-title">Propulsion programs need a partner that can move from concept to certified hardware.</h2>
            <p>
              EngineTech combines precision manufacturing, hot-fire validation, materials engineering, and mission support
              for aircraft and spacecraft programs that cannot afford uncertainty.
            </p>
          </div>

          <a class="capabilities__button" href="#contact">
            <span>Start a Program</span>
            <i class="ph ph-arrow-up-right" aria-hidden="true"></i>
          </a>
        </div>

        <div class="capabilities__grid" aria-label="EngineTech capabilities and proof points">
          <article class="cap-card cap-card--tall cap-card--media">
            <video class="cap-card__video" autoplay muted loop playsinline>
              <source src="https://assets.mixkit.co/videos/45229/45229-720.mp4" type="video/mp4" />
            </video>
            <div class="cap-card__shade" aria-hidden="true"></div>

            <div class="cap-card__label">
              <span>Program Background</span>
            </div>

            <div class="cap-card__timeline">
              <div><span>2026</span><b aria-hidden="true"></b><strong>Reusable upper-stage demonstrator</strong><em>Thermal qualification</em></div>
              <div><span>2025</span><b aria-hidden="true"></b><strong>Hybrid-electric aircraft platform</strong><em>Combustor redesign</em></div>
              <div><span>2024</span><b aria-hidden="true"></b><strong>Orbital transfer vehicle</strong><em>Flight article delivery</em></div>
            </div>
          </article>

          <div class="capabilities__stack">
            <article class="cap-card cap-card--quote">
              <div class="cap-card__label cap-card__label--left">
                <span>Mission Voice</span>
              </div>
              <blockquote>
                "EngineTech brought the discipline we needed: clear design reviews, repeatable test data, and hardware
                that arrived ready for integration."
              </blockquote>
              <p><strong>Dr. Lena Morris</strong> Propulsion Lead, Orbital Systems Group</p>
            </article>

            <article class="cap-card cap-card--metric cap-card--video-panel">
              <video class="cap-card__video" autoplay muted loop playsinline>
                <source src="https://assets.mixkit.co/videos/23211/23211-720.mp4" type="video/mp4" />
              </video>
              <div class="cap-card__shade" aria-hidden="true"></div>
              <div class="cap-card__metric">
                <strong>2K</strong>
                <span>Highly Qualified Engineers</span>
              </div>
            </article>
          </div>

          <div class="capabilities__stack capabilities__stack--systems">
            <article class="cap-card cap-card--tools cap-card--tools-media cap-card--video-panel">
              <video class="cap-card__video" autoplay muted loop playsinline>
                <source src="https://assets.mixkit.co/videos/23843/23843-720.mp4" type="video/mp4" />
              </video>
              <div class="cap-card__shade" aria-hidden="true"></div>

              <div class="cap-card__label">
                <span>Core Systems</span>
              </div>

              <div class="tool-marquee" aria-hidden="true">
                <div class="tool-marquee__row tool-marquee__row--left">
                  <span><i class="ph ph-gear-six"></i> Turbopumps</span>
                  <span><i class="ph ph-fire"></i> Hot-fire</span>
                  <span><i class="ph ph-gauge"></i> Telemetry</span>
                  <span><i class="ph ph-atom"></i> Alloys</span>
                  <span><i class="ph ph-wrench"></i> Assembly</span>
                  <span><i class="ph ph-gear-six"></i> Turbopumps</span>
                  <span><i class="ph ph-fire"></i> Hot-fire</span>
                  <span><i class="ph ph-gauge"></i> Telemetry</span>
                  <span><i class="ph ph-atom"></i> Alloys</span>
                  <span><i class="ph ph-wrench"></i> Assembly</span>
                </div>
                <div class="tool-marquee__row tool-marquee__row--right">
                  <span><i class="ph ph-cpu"></i> Controls</span>
                  <span><i class="ph ph-wave-sine"></i> Vibration</span>
                  <span><i class="ph ph-shield-check"></i> Certification</span>
                  <span><i class="ph ph-rocket-launch"></i> Launch</span>
                  <span><i class="ph ph-chart-line-up"></i> Analysis</span>
                  <span><i class="ph ph-cpu"></i> Controls</span>
                  <span><i class="ph ph-wave-sine"></i> Vibration</span>
                  <span><i class="ph ph-shield-check"></i> Certification</span>
                  <span><i class="ph ph-rocket-launch"></i> Launch</span>
                  <span><i class="ph ph-chart-line-up"></i> Analysis</span>
                </div>
              </div>
            </article>

            <article class="cap-card cap-card--contact" id="contact">
              <div>
                <div class="cap-card__label cap-card__label--left">
                  <span>Reach Engineering</span>
                </div>
                <a href="mailto:programs@enginetech.com">programs@enginetech.com</a>
                <p>+1 415 018 4270</p>
              </div>
              <a class="cap-card__icon-button" href="mailto:programs@enginetech.com" aria-label="Email EngineTech">
                <i class="ph ph-arrow-up-right" aria-hidden="true"></i>
              </a>
            </article>
          </div>
        </div>
      </section>

      <section class="stats" id="our-edge" aria-labelledby="stats-title">
        <div class="stats__header">
          <div class="stats__title-wrap">
            <h2 id="stats-title">Unmatched propulsion data across every flight-critical layer.</h2>
          </div>
          <p class="stats__summary" data-stats-summary>
            EngineTech maps thermal limits, production capacity, upstream readiness, and hydrogen pathways into
            clear decisions for ambitious aerospace programs.
          </p>
        </div>

        <div class="stats__tabs" role="tablist" aria-label="Statistics categories">
          <button class="stats__tab is-active" type="button" role="tab" aria-selected="true" data-stats-tab="cities">
            Cities & Infrastructure
          </button>
          <button class="stats__tab" type="button" role="tab" aria-selected="false" data-stats-tab="materials">
            Materials & Manufacturing
          </button>
          <button class="stats__tab" type="button" role="tab" aria-selected="false" data-stats-tab="fuels">
            Fuels & Upstream
          </button>
          <button class="stats__tab" type="button" role="tab" aria-selected="false" data-stats-tab="hydrogen">
            H2 Hydrogen
          </button>
        </div>

        <div class="stats__chart" data-stats-chart aria-live="polite"></div>
      </section>

      <section class="video-stories" id="our-team" aria-labelledby="video-stories-title">
        <div class="video-stories__header">
          <h2 id="video-stories-title">Program stories from the people building flight-ready power.</h2>
          <p>
            Short field notes from integration leads, test engineers, and manufacturing teams moving advanced
            propulsion systems from requirement reviews to repeatable flight hardware.
          </p>
        </div>

        <div class="video-stories__rail" aria-label="EngineTech video previews">
          <article class="story-card">
            <video class="story-card__media" autoplay muted loop playsinline>
              <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032431_5e054107-51c0-4162-9f0f-3a40054761ef.mp4" type="video/mp4" />
            </video>
            <div class="story-card__content">
              <p>Integration Review</p>
              <h3>How a reusable upper-stage program moved from thermal risk to stable qualification.</h3>
              <span>Reusable systems · 04:20</span>
            </div>
          </article>

          <article class="story-card">
            <video class="story-card__media" autoplay muted loop playsinline>
              <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032535_4ccc152e-0cc8-4ee5-a698-e1a98cea8a1e.mp4" type="video/mp4" />
            </video>
            <div class="story-card__content">
              <p>Hot-Fire Campaign</p>
              <h3>Inside the test cell where telemetry, vibration, and injector response converge.</h3>
              <span>Validation · 03:45</span>
            </div>
          </article>

          <article class="story-card">
            <video class="story-card__media" autoplay muted loop playsinline>
              <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_033707_b842a2ea-f223-4804-96d0-737ab67510fc.mp4" type="video/mp4" />
            </video>
            <div class="story-card__content">
              <p>Manufacturing Floor</p>
              <h3>Why sub-micron inspection changes the way aerospace teams plan reliability.</h3>
              <span>Precision build · 05:10</span>
            </div>
          </article>

          <article class="story-card">
            <video class="story-card__media" autoplay muted loop playsinline>
              <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032431_5e054107-51c0-4162-9f0f-3a40054761ef.mp4" type="video/mp4" />
            </video>
            <div class="story-card__content">
              <p>Hydrogen Pathway</p>
              <h3>Designing feed systems and ignition envelopes for hydrogen-ready propulsion.</h3>
              <span>H2 systems · 04:55</span>
            </div>
          </article>

          <article class="story-card">
            <video class="story-card__media" autoplay muted loop playsinline>
              <source src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032535_4ccc152e-0cc8-4ee5-a698-e1a98cea8a1e.mp4" type="video/mp4" />
            </video>
            <div class="story-card__content">
              <p>Mission Support</p>
              <h3>The operational cadence behind launch-window support and post-test analysis.</h3>
              <span>Field readiness · 03:30</span>
            </div>
          </article>
        </div>

        <div class="video-stories__footer" aria-hidden="true">
          <span></span>
          <span></span>
          <span></span>
          <strong>05 / 05</strong>
        </div>
      </section>
    </main>

    <footer class="site-footer">
      <div class="footer-dots" aria-hidden="true">
        <div class="footer-dots__line"></div>
      </div>

      <div class="site-footer__inner">
        <div class="site-footer__top">
          <h2>Proven Advanced Propulsion Technology</h2>

          <nav class="site-footer__nav" aria-label="Footer navigation">
            <a href="#company">Company</a>
            <a href="#technology">Technology</a>
            <a href="#solutions">Solutions</a>
            <a href="#our-edge">Our Edge</a>
            <a href="#investors">Investors</a>
          </nav>

          <nav class="site-footer__nav" aria-label="Company links">
            <a href="#our-team">Our Team</a>
            <a href="#news">News</a>
            <a href="#careers">Careers</a>
            <a href="#contact">Contact Us</a>
          </nav>

          <nav class="site-footer__nav" aria-label="Social links">
            <a href="https://www.linkedin.com" target="_blank" rel="noreferrer">LinkedIn</a>
            <a href="https://x.com" target="_blank" rel="noreferrer">Follow Us on X</a>
          </nav>
        </div>

        <div class="site-footer__brand-row">
          <a class="site-footer__brand" href="/" aria-label="EngineTech home">
            <span class="site-footer__mark" aria-hidden="true"></span>
            <span>EngineTech</span>
          </a>
        </div>

        <div class="site-footer__legal">
          <p>&copy; 2026 EngineTech. All rights reserved.</p>
          <a href="#privacy">Privacy Policy</a>
          <a href="#terms">Terms of Use</a>
        </div>
      </div>
    </footer>

    <script>
// === Hero Section ===
const navItems = ["Company", "Technology", "Solutions", "Our Edge", "Our Team", "Investors", "News"];

class EngineHero extends HTMLElement {
  scrollFrame = 0;
  lastScrollY = 0;

  connectedCallback() {
    this.innerHTML = `
      <section class="hero" id="heroScroll" aria-labelledby="hero-title">
        <div class="hero__background" aria-hidden="true">
          <div class="hero__bg-layer hero__bg-layer--bottom"></div>
          <div class="hero__stars"></div>
          <div class="hero__bg-layer hero__bg-layer--top"></div>
        </div>

        <header class="hero__nav">
          <a class="brand" href="/" aria-label="EngineTech home">
            <span class="brand__mark" aria-hidden="true">
              <span></span><span></span><span></span><span></span>
            </span>
            <span class="brand__name">EngineTech</span>
          </a>

          <nav class="hero__links" aria-label="Primary navigation">
            ${navItems.map((item) => `<a href="#${item.toLowerCase().replaceAll(" ", "-")}">${item}</a>`).join("")}
          </nav>

          <a class="hero__cta" href="#contact">Get In Touch</a>
        </header>

        <div class="hero__content">
          <h1 id="hero-title" class="hero__title" aria-label="Powering the Ship">
            <span class="hero__title-line hero__title-line--one">Powering</span>
          </h1>

          <div class="hero__title-row" aria-hidden="true">
            <span class="hero__title-line hero__title-line--two">the</span>
            <span class="hero__title-line hero__title-line--three">Ship</span>
          </div>

          <div class="engine-visual" aria-hidden="true">
            <img class="engine-visual__asset" src="https://res.cloudinary.com/dsdhxhhqh/image/upload/v1780405513/hero-engine_isebcf.png" alt="" />
          </div>
        </div>

        <p class="hero__caption">
          Precision engines for orbital-class vehicles.
        </p>
      </section>
    `;

    this.initScrollHero();
  }

  initScrollHero() {
    const hero = this.querySelector(".hero");
    const bg = this.querySelector(".hero__background");
    const title = this.querySelector(".hero__title");
    const titleRow = this.querySelector(".hero__title-row");
    const caption = this.querySelector(".hero__caption");
    const object = this.querySelector(".engine-visual");
    if (!hero || !bg || !title || !titleRow || !caption || !object) return;

    const lerp = (a, b, progress) => a + (b - a) * progress;
    const colors = {
      start: { top: [113, 145, 208], mid: [170, 184, 213], bottom: [236, 233, 230] },
      end: { top: [240, 232, 220], mid: [238, 229, 216], bottom: [236, 226, 210] },
    };

    const mixColor = (from, to, progress) => {
      const r = Math.round(lerp(from[0], to[0], progress));
      const g = Math.round(lerp(from[1], to[1], progress));
      const b = Math.round(lerp(from[2], to[2], progress));
      return `rgb(${r}, ${g}, ${b})`;
    };

    const animate = () => {
      const rect = hero.getBoundingClientRect();
      const scrollLength = Math.max(hero.offsetHeight - window.innerHeight, 1);
      const progress = Math.min(Math.max(Math.abs(rect.top) / scrollLength, 0), 1);
      const scrollProgress = Math.max(Math.abs(rect.top) / scrollLength, 0);

      const scrollY = Math.abs(rect.top);
      const fadeStart = 0.9 * window.innerHeight;
      const fadeEnd = 1.35 * window.innerHeight;
      let fade = 1;
      if (scrollY > fadeStart) {
        fade = 1 - Math.min((scrollY - fadeStart) / (fadeEnd - fadeStart), 1);
      }

      const nav = this.querySelector(".hero__nav");
      if (nav) {
        if (scrollY === 0) {
          nav.classList.add("nav--at-top");
          nav.classList.remove("nav--scroll-down", "nav--scroll-up");
        } else if (scrollY > this.lastScrollY) {
          nav.classList.add("nav--scroll-down");
          nav.classList.remove("nav--at-top", "nav--scroll-up");
        } else if (scrollY < this.lastScrollY) {
          nav.classList.add("nav--scroll-up");
          nav.classList.remove("nav--at-top", "nav--scroll-down");
        }
      }
      this.lastScrollY = scrollY;

      bg.style.setProperty("--hero-top", mixColor(colors.start.top, colors.end.top, progress));
      bg.style.setProperty("--hero-mid", mixColor(colors.start.mid, colors.end.mid, progress));
      bg.style.setProperty("--hero-bottom", mixColor(colors.start.bottom, colors.end.bottom, progress));

      title.style.setProperty("--scroll-y", `${(scrollProgress * -120).toFixed(2)}px`);
      titleRow.style.setProperty("--scroll-y", `${(scrollProgress * -120).toFixed(2)}px`);
      caption.style.setProperty("--scroll-y", `${(scrollProgress * -60).toFixed(2)}px`);
      object.style.setProperty("--scroll-y", `${(scrollProgress * -250).toFixed(2)}px`);

      title.style.opacity = fade;
      titleRow.style.opacity = fade;
      caption.style.opacity = fade;
      object.style.opacity = fade;

      hero.classList.toggle("is-past", rect.bottom <= 0);

      this.scrollFrame = requestAnimationFrame(animate);
    };

    animate();
  }

  disconnectedCallback() {
    cancelAnimationFrame(this.scrollFrame);
  }
}

customElements.define("engine-hero", EngineHero);

// === Showcase Section ===
const TABS = [
  { num: "01", label: "Precision Manufacturing", title: "Built to Sub-Micron<br>Tolerances", desc: "Every component is machined and inspected in our ISO-certified facility, achieving tolerances that exceed aerospace standards by a factor of four." },
  { num: "02", label: "Advanced Materials", title: "Engineered for<br>Extreme Environments", desc: "Proprietary titanium and nickel superalloys withstand operating temperatures exceeding 1,600\u00B0C while maintaining structural integrity across millions of thermal cycles." },
  { num: "03", label: "Thermal Testing", title: "10,000 Cycles<br>Before First Flight", desc: "Each engine variant undergoes a rigorous qualification program simulating the full range of flight conditions, from sea-level ignition to orbital thermal cycling." },
  { num: "04", label: "Mission Certified", title: "Flight-Proven<br>Propulsion", desc: "Our engines have powered missions across low-Earth orbit, polar orbit, and deep-space trajectories \u2014 delivering zero in-flight anomalies across 47 consecutive launches." },
];

const lerp = (a, b, t) => a + (b - a) * t;
const clamp = (v, lo, hi) => Math.min(Math.max(v, lo), hi);
const easeOutCubic = (t) => 1 - Math.pow(1 - t, 3);
const easeInOutCubic = (t) => t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;

class ShowcaseSection {
  frame = 0;
  startRect = null;
  isStartLocked = false;
  expandStartScrollY = 0;

  constructor() {
    this.el = document.querySelector(".showcase");
    this.missionMedia = document.querySelector(".mission__media");
    if (!this.el) return;
    this.createFilm();
    this.renderUI();
    this.loop();
  }

  createFilm() {
    this.film = document.createElement("div");
    this.film.className = "showcase-film";
    this.film.innerHTML = `
      <video class="showcase-film__video" autoplay muted loop playsinline poster="https://res.cloudinary.com/dsdhxhhqh/image/upload/v1780405513/hero-engine_isebcf.png">
        <source src="https://assets.mixkit.co/videos/6853/6853-720.mp4" type="video/mp4" />
      </video>
      <div class="showcase-film__overlay"></div>
    `;
    document.body.appendChild(this.film);
    this.filmOverlay = this.film.querySelector(".showcase-film__overlay");
  }

  renderUI() {
    this.el.innerHTML = `
      <div class="showcase__sticky">
        <div class="showcase__ui" aria-live="polite">
          <div class="showcase__panels">
            ${TABS.map((t, i) => `
              <div class="showcase__panel${i === 0 ? " is-active" : ""}" data-index="${i}" aria-hidden="${i !== 0}">
                <span class="showcase__panel-num">${t.num}</span>
                <h2 class="showcase__panel-title">${t.title}</h2>
                <p class="showcase__panel-desc">${t.desc}</p>
              </div>`).join("")}
          </div>
          <nav class="showcase__tabs-nav" aria-label="Technology sections">
            ${TABS.map((t, i) => `
              <div class="showcase__tab${i === 0 ? " is-active" : ""}" data-index="${i}" role="tab" aria-selected="${i === 0}">
                <span class="showcase__tab-bar" aria-hidden="true"></span>
                <span class="showcase__tab-name">${t.label}</span>
                <span class="showcase__tab-num">${t.num}</span>
              </div>`).join("")}
          </nav>
        </div>
      </div>
    `;
    this.ui = this.el.querySelector(".showcase__ui");
    this.panels = this.el.querySelectorAll(".showcase__panel");
    this.tabs = this.el.querySelectorAll(".showcase__tab");
  }

  cardToRect(mr) { return { top: mr.top, left: mr.left, width: mr.width, height: mr.height, radius: 0 }; }

  applyRect(r) {
    this.film.style.top = `${r.top.toFixed(2)}px`;
    this.film.style.left = `${r.left.toFixed(2)}px`;
    this.film.style.width = `${r.width.toFixed(2)}px`;
    this.film.style.height = `${r.height.toFixed(2)}px`;
    this.film.style.borderRadius = `${r.radius.toFixed(2)}px`;
  }

  loop = () => {
    const { el, missionMedia, film, filmOverlay, ui, panels, tabs } = this;
    const vh = window.innerHeight;
    const rect = el.getBoundingClientRect();
    const scrolled = -rect.top;
    const totalScroll = Math.max(el.offsetHeight - vh, 1);
    let missionMediaVisible = false;
    let missionMediaPending = false;

    if (rect.bottom <= 0) {
      film.style.opacity = "0";
      filmOverlay.style.opacity = "0";
      ui.style.opacity = "0";
      this.frame = requestAnimationFrame(this.loop);
      return;
    }

    if (missionMedia) {
      const mr = missionMedia.getBoundingClientRect();
      missionMediaVisible = mr.width > 0 && mr.height > 0 && mr.bottom > 0 && mr.top < vh;
      missionMediaPending = mr.width > 0 && mr.height > 0 && mr.top >= vh;

      if (missionMediaVisible && scrolled <= 0) {
        const mediaCenterY = mr.top + mr.height / 2;
        if (mediaCenterY > vh / 2) { this.isStartLocked = false; this.expandStartScrollY = 0; }
        if (mediaCenterY <= vh / 2 || this.isStartLocked) {
          if (!this.isStartLocked) { this.expandStartScrollY = window.scrollY; }
          this.isStartLocked = true;
          this.startRect = this.cardToRect(mr);
        } else {
          this.startRect = this.cardToRect(mr);
        }
      }
    }

    if (!this.isStartLocked) {
      if (missionMediaPending) { this.startRect = null; this.expandStartScrollY = 0; }
      if (this.startRect) { this.applyRect(this.startRect); }
      film.style.opacity = this.startRect ? "1" : "0";
      filmOverlay.style.opacity = "0";
      ui.style.opacity = "0";
      this.frame = requestAnimationFrame(this.loop);
      return;
    }

    const expandP = clamp((window.scrollY - this.expandStartScrollY) / vh, 0, 1);
    const eased = easeOutCubic(expandP);
    film.style.opacity = "1";

    const sr = this.startRect || { top: vh * 0.21, left: window.innerWidth * 0.38, width: window.innerWidth * 0.58, height: vh * 0.58, radius: 0 };
    this.applyRect({
      top: lerp(sr.top, 0, eased),
      left: lerp(sr.left, 0, eased),
      width: lerp(sr.width, window.innerWidth, eased),
      height: lerp(sr.height, vh, eased),
      radius: lerp(sr.radius, 0, eased),
    });

    filmOverlay.style.opacity = String((eased * 0.22).toFixed(3));

    if (expandP < 1) { ui.style.opacity = "0"; this.frame = requestAnimationFrame(this.loop); return; }

    const progress = clamp(scrolled / totalScroll, 0, 1);
    const uiP = clamp(progress / 0.08, 0, 1);
    ui.style.opacity = String(easeInOutCubic(uiP).toFixed(3));

    const TAB_START = 0.08;
    const tabP = clamp((progress - TAB_START) / (1 - TAB_START), 0, 1);
    const activeTab = clamp(Math.floor(tabP * TABS.length), 0, TABS.length - 1);

    panels.forEach((p, i) => { const active = i === activeTab; p.classList.toggle("is-active", active); p.setAttribute("aria-hidden", String(!active)); });
    tabs.forEach((t, i) => { const active = i === activeTab; t.classList.toggle("is-active", active); t.setAttribute("aria-selected", String(active)); });

    this.frame = requestAnimationFrame(this.loop);
  };

  destroy() { cancelAnimationFrame(this.frame); this.film?.remove(); }
}

new ShowcaseSection();

// === Stats Section ===
const DATASETS = {
  cities: {
    title: "Cities & Infrastructure",
    summary: "Distributed aerospace infrastructure needs engines that can test, relight, and recover across dense launch corridors and remote operating bases.",
    bars: [
      { label: "Mobile integration bays", value: 82, target: 88, rangeStart: 58, rangeEnd: 91, unit: "%", note: "deployment coverage", trace: [28, 42, 57, 63, 74, 82] },
      { label: "Airport-adjacent service cells", value: 68, target: 74, rangeStart: 44, rangeEnd: 79, unit: "%", note: "qualified workflows", trace: [18, 36, 41, 55, 61, 68] },
      { label: "Remote launch support", value: 54, target: 63, rangeStart: 30, rangeEnd: 70, unit: "%", note: "field readiness", trace: [14, 24, 39, 43, 48, 54] },
      { label: "Thermal recovery loops", value: 76, target: 81, rangeStart: 50, rangeEnd: 84, unit: "%", note: "heat reuse potential", trace: [26, 38, 49, 66, 72, 76] },
    ],
  },
  materials: {
    title: "Materials & Manufacturing",
    summary: "EngineTech combines high-temperature alloys, additive tooling, and inspection data to compress the path from design lock to certified hardware.",
    bars: [
      { label: "Nickel superalloy margin", value: 91, target: 94, rangeStart: 68, rangeEnd: 96, unit: "%", note: "thermal headroom", trace: [44, 61, 70, 79, 86, 91] },
      { label: "Additive chamber tooling", value: 72, target: 80, rangeStart: 48, rangeEnd: 86, unit: "%", note: "lead-time reduction", trace: [19, 34, 48, 53, 67, 72] },
      { label: "Sub-micron inspection yield", value: 96, target: 97, rangeStart: 82, rangeEnd: 99, unit: "%", note: "accepted components", trace: [71, 77, 84, 89, 94, 96] },
      { label: "Reusable test article cycles", value: 84, target: 88, rangeStart: 62, rangeEnd: 91, unit: "%", note: "qualification depth", trace: [36, 52, 64, 71, 79, 84] },
    ],
  },
  fuels: {
    title: "Fuels & Upstream",
    summary: "Fuel-path analysis links propellant availability, storage constraints, and injector behavior before a program commits to flight architecture.",
    bars: [
      { label: "Methane supply compatibility", value: 78, target: 83, rangeStart: 52, rangeEnd: 88, unit: "%", note: "regional availability", trace: [22, 31, 46, 58, 69, 78] },
      { label: "Kerosene retrofit readiness", value: 64, target: 70, rangeStart: 40, rangeEnd: 74, unit: "%", note: "legacy platforms", trace: [28, 35, 39, 52, 57, 64] },
      { label: "Cryogenic storage stability", value: 88, target: 92, rangeStart: 66, rangeEnd: 95, unit: "%", note: "validated envelopes", trace: [45, 56, 68, 74, 83, 88] },
      { label: "Injector response confidence", value: 92, target: 94, rangeStart: 70, rangeEnd: 97, unit: "%", note: "hot-fire data", trace: [48, 62, 73, 85, 89, 92] },
    ],
  },
  hydrogen: {
    title: "H2 Hydrogen",
    summary: "Hydrogen programs require tight coordination between tankage, feed systems, ignition stability, and ultra-low-temperature operations.",
    bars: [
      { label: "Hydrogen-ready turbopumps", value: 86, target: 90, rangeStart: 62, rangeEnd: 93, unit: "%", note: "design maturity", trace: [30, 46, 60, 71, 79, 86] },
      { label: "LH2 feedline conditioning", value: 74, target: 82, rangeStart: 47, rangeEnd: 86, unit: "%", note: "ground systems", trace: [18, 29, 44, 58, 66, 74] },
      { label: "Ignition stability range", value: 93, target: 95, rangeStart: 72, rangeEnd: 98, unit: "%", note: "transient control", trace: [54, 68, 75, 84, 90, 93] },
      { label: "Zero-carbon flight pathway", value: 81, target: 87, rangeStart: 56, rangeEnd: 90, unit: "%", note: "program fit", trace: [24, 39, 55, 68, 76, 81] },
    ],
  },
};

class StatsSection {
  activeKey = "cities";

  constructor() {
    this.el = document.querySelector(".stats");
    if (!this.el) return;
    this.tabs = this.el.querySelectorAll("[data-stats-tab]");
    this.summary = this.el.querySelector("[data-stats-summary]");
    this.chart = this.el.querySelector("[data-stats-chart]");
    this.tabs.forEach((tab) => { tab.addEventListener("click", () => this.setActive(tab.dataset.statsTab)); });
    this.render();
  }

  setActive(key) {
    if (!DATASETS[key] || key === this.activeKey) return;
    this.activeKey = key;
    this.tabs.forEach((tab) => { const active = tab.dataset.statsTab === key; tab.classList.toggle("is-active", active); tab.setAttribute("aria-selected", String(active)); });
    this.render();
  }

  render() {
    const data = DATASETS[this.activeKey];
    this.summary.classList.remove("is-visible");
    this.chart.classList.remove("is-ready");

    window.setTimeout(() => {
      this.summary.textContent = data.summary;
      this.chart.innerHTML = `
        <div class="stats__chart-head">
          <span>${data.title}</span>
          <strong>Operating envelope</strong>
        </div>
        <div class="stats__bars">
          ${data.bars.map((bar, index) => `
            <article class="stats__bar-row" style="--bar-value: ${bar.value}%; --range-start: ${bar.rangeStart}%; --range-width: ${bar.rangeEnd - bar.rangeStart}%; --bar-delay: ${index * 90}ms;">
              <div class="stats__bar-label">
                <strong>${bar.label}</strong>
                <span>${bar.note}</span>
              </div>
              <div class="stats__track" aria-hidden="true">
                <div class="stats__range"></div>
                <div class="stats__bar"></div>
                <span class="stats__value">${bar.value}${bar.unit}</span>
                <div class="stats__trace">
                  ${bar.trace.map((point, pointIndex) => `<i class="stats__spark stats__spark--${pointIndex % 3}" style="--point-x: ${Math.min(point, bar.value - 3)}%; --point-y: ${pointIndex % 2 === 0 ? 34 : 62}%; --point-delay: ${pointIndex * 70}ms"></i>`).join("")}
                </div>
              </div>
            </article>
          `).join("")}
        </div>
        <div class="stats__axis" aria-hidden="true">
          <span></span>
          <div>
            ${Array.from({ length: 11 }, (_, i) => `<span>${i * 10}</span>`).join("")}
          </div>
        </div>
      `;

      requestAnimationFrame(() => {
        this.summary.classList.add("is-visible");
        this.chart.classList.add("is-ready");
      });
    }, 140);
  }
}

new StatsSection();
    </script>
  </body>
</html>

## AI Automation — Landing Page [sites/ai-automation]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(83).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-automation.webp

Build a React + Vite + Tailwind CSS landing page for an AI agency called "COGNITRA". Use `framer-motion` for animations and `lucide-react` for icons. The design uses "Helvetica Now Var" font throughout. Here is the exact specification:

---

### FONT

Import via CSS:
```
@import url('https://db.onlinewebfonts.com/c/e66905e07608167a84e6ad52f638c3c6?family=Helvetica+Now+Var');
```
Apply globally: `font-family: 'Helvetica Now Var', 'Helvetica Neue', Helvetica, Arial, sans-serif;`

---

### FadeUp ANIMATION COMPONENT

Create a reusable `FadeUp` component wrapping Framer Motion with these exact values:
- Props: `children`, `delay` (default 0), `duration` (default 0.7), `y` (default 24), `className`, `style`, `as` (polymorphic: div/section/span/h1/h2/h3/p/nav), `once` (default true)
- `initial={{ opacity: 0, y }}`
- `whileInView={{ opacity: 1, y: 0 }}`
- `viewport={{ once, amount: 0.2 }}`
- `transition={{ duration, delay, ease: [0.22, 1, 0.36, 1] }}`

---

### LAYOUT STRUCTURE

The page is a single `<div style={{ position: 'relative' }}>` containing:

1. A **fixed full-viewport background video** (z-index 0)
2. A **fixed transparent navbar** (z-index 10)
3. **Section 1** -- Hero (100vh, z-index 1)
4. **Section 2** -- Statement (100vh, z-index 1, transparent bg over video)
5. **Section 3** -- Services (auto height, z-index 2, #C5C5C5 bg)
6. **Fixed scroll indicator** (bottom center, z-index 5)
7. **Fixed share/repost button** (bottom right, z-index 5)

---

### FIXED BACKGROUND VIDEO

```
position: fixed, top: 0, left: 0, width: 100%, height: 100vh, objectFit: cover, zIndex: 0
src: "https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_135830_bb6491d1-9b66-4aec-9722-13b4dfe3fb46.mp4"
autoPlay, muted, loop, playsInline
```

---

### NAVBAR (fixed, transparent)

- `position: fixed; top:0; left:0; right:0; z-index:10; background: transparent; border-bottom: 1px solid rgba(0,0,0,0.18); display:flex; align-items:center; justify-content:space-between; padding: 20px 32px;`
- **Left:** Brand "COGNITRA" -- FadeUp delay=0, fontSize 13px, fontWeight 700, letterSpacing 0.12em, uppercase, color #1a1a1a
- **Center:** Links ['MAIN', 'OFFERING', 'CASE', 'RATES'] in a flex row gap 48px. Each link wrapped in FadeUp with delay = 0.05 + i*0.05. Links: fontSize 11px, letterSpacing 0.06em, color #1a1a1a, fontWeight 400
- **Right:** Links ['CREW', 'CONNECT'] same style, FadeUp delay = 0.3 + i*0.05
- Hover on all links: opacity 0.6

---

### SECTION 1 -- HERO (100vh)

- `position: relative; zIndex: 1; height: 100vh;`
- **Top overlay div** (absolute, top:0, left:0, right:0, height: 48%, background: #C5C5C5, flex column, paddingTop: 70px)
  - Inner content area: `flex:1; display:flex; alignItems:flex-end; padding: 0 32px 24px 32px;`
  - **Hero row** (flex, stretch, width 100%, gap 48px):
    - **Left column** (width 32%, flex-column, justify space-between, gap 80px):
      - `<h1>` FadeUp as="h1" delay=0.1 -- "SCALING\nFASTER USING AI" -- fontSize clamp(26px, 3vw, 42px), fontWeight 700, lineHeight 1.05, letterSpacing -0.01em, uppercase, color #1a1a1a
      - Slide counter FadeUp delay=0.5 -- "001 / 005" -- fontSize 11px, letterSpacing 0.08em, color #666
    - **Right column** (flex:1, flex-column, justify space-between, gap 80px):
      - `<p>` FadeUp as="p" delay=0.25 -- "We engineer custom automation flows and personalized AI products for ambitious modern businesses." -- fontSize 18px, lineHeight 1.6, color #5a5a5a, maxWidth 340px
      - Buttons row (flex, gap 10px) FadeUp delay=0.4:
        - "BOOK A CALL!" -- btn-primary: bg #1a1a1a, color #fff, border 1px solid #1a1a1a, border-radius 9999px, padding 12px 36px, fontSize 11px, fontWeight 500, letterSpacing 0.08em, uppercase
        - "OUR PRODUCTS" -- btn-secondary: bg transparent, color #1a1a1a, border 1px solid #1a1a1a, same radius/padding/fontSize/weight/spacing. Hover: bg #1a1a1a, color #fff
- **Bottom-left text** (absolute, top 74%, transform translateY(-50%), left 32px, maxWidth 260px) FadeUp delay=0.6:
  - "Guiding future-minded companies forward with bespoke AI products and streamlined workflows." -- fontSize 14px, lineHeight 1.65, color rgba(255,255,255,0.9)

---

### SECTION 2 -- STATEMENT (100vh, transparent over video)

- `position:relative; zIndex:1; height:100vh; display:flex; flexDirection:column; justify-content:center; padding: 70px 32px 32px 32px;`
- Inner div: flex-column, align flex-start, maxWidth 720px, padding 80px 0
- `<h2>` -- fontSize clamp(26px, 3vw, 42px), fontWeight 700, lineHeight 1.08, letterSpacing -0.01em, uppercase, color #fff, display flex, flexWrap wrap, gap 0.25em
  - Text "WE BUILD END-TO-END AI AUTOMATION SYSTEMS." split by space, each word wrapped in FadeUp as="span" delay = 0.15 + i*0.08, y=32
- `<p>` FadeUp as="p" delay=0.9 -- "We provide all-in-one AI automation services in one place." -- marginTop 24px, fontSize 14px, lineHeight 1.65, color rgba(255,255,255,0.85), maxWidth 260px

---

### SECTION 3 -- SERVICES (gray bg)

- `position:relative; zIndex:2; background:#C5C5C5; display:flex; flexDirection:column; padding: 70px 32px 80px 32px; min-height:auto;`
- **Counter**: FadeUp delay=0 -- "003 / 005" -- fontSize 11px, letterSpacing 0.08em, color #666, marginBottom 20px
- **Head row** (flex, gap 48px, align flex-start, marginBottom 32px):
  - Left col (width 32%): `<h2>` "EXPLORE WHAT WE OFFER" -- fontSize clamp(26px, 3vw, 42px), fontWeight 700, lineHeight 1.05, letterSpacing -0.01em, uppercase, color #1a1a1a, maxWidth 320px, display flex, flexWrap wrap, gap 0.25em. Each word FadeUp as="span" delay = 0.1 + i*0.1, y=28
  - Right col (flex:1, paddingTop 8px): FadeUp as="p" delay=0.25 -- "We provide all-in-one AI automation services in one place." -- fontSize 14px, lineHeight 1.65, color #3a3a3a, maxWidth 320px
- **Cards grid** (CSS grid, 3 columns 1fr, gap 20px, grid-auto-rows 1fr):
  - 3 cards, each FadeUp delay = 0.4 + idx*0.15:
    - Card container: bg transparent, border 1px solid rgba(0,0,0,0.18), borderRadius 20px, overflow hidden, flex column, paddingTop 16px
    - Video area: width 100%, aspectRatio 4/3, position relative, overflow hidden. Video inside: absolute inset 0, objectFit cover
    - Text area: padding 24px 28px 28px 28px
      - `<h3>` fontSize 18px, fontWeight 600, color #1a1a1a, marginBottom 14px
      - `<p>` fontSize 13px, lineHeight 1.6, color #3a3a3a
  - Card data:
    1. video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_220333_48163edc-995f-4513-9f44-48dbb07a7329.mp4`, title: "Process Streamlining", text: "We automate your processes by linking together the daily tools you rely upon. Lifting throughput and improving overall output."
    2. video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_221040_e6ba7c5a-864e-46e9-871e-341a176a7e3e.mp4`, title: "Strategic advisory", text: "We craft intelligent assistants that are adaptive, grasp context, and are skilled enough to handle highly intricate customer requests."
    3. video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260513_221104_fb538584-5b87-495f-952e-09ddd5a1792a.mp4`, title: "Assistant engineering", text: "Through our knowledge, we explore deep into your business and advise you on how AI powered automations may transform your operations."

---

### FIXED SCROLL INDICATOR (bottom center)

- `position:fixed; bottom:32px; left:50%; transform:translateX(-50%); zIndex:5;`
- CSS animation `scrollBounce`: `0%,100% { transform: translateY(0); } 50% { transform: translateY(6px); }` -- 2s ease-in-out infinite
- Pill shape: width 22px, height 36px, border 1.5px solid rgba(0,0,0,0.75), borderRadius 11px, flex, justify center, paddingTop 6px
- Inner dot: width 3px, height 8px, background rgba(0,0,0,0.85), borderRadius 2px

---

### FIXED REPOST BUTTON (bottom right)

- `position:fixed; bottom:32px; right:32px; zIndex:5; display:flex; alignItems:center; gap:6px; color:rgba(0,0,0,0.8); fontSize:11px; letterSpacing:0.08em; uppercase; cursor:pointer;`
- Inline SVG (share icon), width 14, height 14, viewBox "0 0 24 24", fill none, stroke currentColor, strokeWidth 2, strokeLinecap round, strokeLinejoin round:
  ```
  <circle cx="18" cy="5" r="3"/>
  <circle cx="6" cy="12" r="3"/>
  <circle cx="18" cy="19" r="3"/>
  <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/>
  <line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/>
  ```
- Text: "REPOST"

---

### RESPONSIVE BREAKPOINTS

**@media (max-width: 900px):**
- nav padding: 16px 18px; nav-links gap: 18px; hide .nav-links-secondary
- hero-row: flex-direction column, gap 24px; hero-col-left/right: width 100%, gap 24px
- section-pad: 90px 18px 32px 18px; section-pad-lg: 90px 18px 60px 18px
- services-head-row: flex-direction column, gap 16px, marginBottom 24px; services-head-col: width 100%
- cards-grid: 1 column, gap 16px
- section-3: height auto, min-height 100vh
- hero-bottom-text: top auto, bottom 80px, transform none, left 18px, right 18px, maxWidth none
- btn-primary/secondary: padding 11px 22px, fontSize 10px

**@media (max-width: 600px):**
- nav-links gap: 14px; nav-brand fontSize: 12px
- hero-overlay height: 56%, paddingTop: 64px
- hero-buttons: flex-wrap wrap

---

### PACKAGES

- react, react-dom
- framer-motion
- lucide-react
- tailwindcss, postcss, autoprefixer
- vite, @vitejs/plugin-react

## AI Designer Agency — Landing Page [sites/ai-designer-agency]

- Preview: https://motionsites.ai/assets/hero-ai-designer-agency-preview-vrAje6Od.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-designer-agency.gif

Prompt to recreate this landing page:

Build a dark-themed, single-page landing page for an AI web design agency called "VIRALMEDIA". The design uses a pure black background (#000) with white text, a signature "liquid glass" glassmorphism effect, and two Google Fonts: Barlow (body/UI) and Instrument Serif (italic accent text). Use React, Tailwind CSS, Framer Motion, and hls.js. All buttons are rounded-full.

DESIGN SYSTEM (index.css)

Color tokens (all HSL, dark-only — no light mode):

--background: 0 0% 0% (pure black)
--foreground: 0 0% 100% (pure white)
--card: 0 0% 9%, --card-foreground: 0 0% 100%
--primary: 0 0% 97%, --primary-foreground: 0 0% 9%
--secondary: 0 0% 15%, --secondary-foreground: 0 0% 100%
--muted: 0 0% 15%, --muted-foreground: 0 0% 75%
--accent: 0 0% 15%, --accent-foreground: 0 0% 100%
--border: 0 0% 20%, --input: 0 0% 20%, --ring: 0 0% 100%
--radius: 2px
--font-body: 'Barlow', sans-serif
--font-accent: 'Instrument Serif', serif

Liquid Glass CSS classes:

.liquid-glass — subtle glassmorphism: background: rgba(255,255,255,0.01), background-blend-mode: luminosity, backdrop-filter: blur(4px), no border, box-shadow: inset 0 1px 1px rgba(255,255,255,0.1). Has a ::before pseudo-element for a gradient border effect using linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, transparent 40%, transparent 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%) with mask-composite: exclude and padding: 1.4px.

.liquid-glass-strong — stronger variant: same as above but backdrop-filter: blur(50px), box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15), and slightly higher gradient opacity values (0.5/0.2 instead of 0.45/0.15).

Tailwind config: Set fontFamily: { body: ['Barlow', 'sans-serif'], accent: ['Instrument Serif', 'serif'] } at the theme root level (not in extend).

SECTION 1: NAVBAR (fixed)

Fixed top, full-width, z-50, px-8 py-6, flex row with space-between
Left: Logo text "VIRALMEDIA" — text-xl font-semibold tracking-tight font-body text-foreground
Center (hidden on mobile): Nav links ['Work', 'Services', 'About', 'Blog', 'Contact'] — each px-4 py-2 text-sm font-medium text-foreground rounded-sm hover:bg-white/10
Right: "Get Started" button — liquid-glass-strong rounded-full px-6 py-2.5 text-sm font-medium text-foreground

SECTION 2: HERO (full viewport height)

Container: relative w-full h-screen overflow-hidden
Background video (behind everything): Absolutely positioned, object-cover object-bottom. On mobile: -translate-y-[100px], on md+: no translate. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260326_073936_8dd07fdb-4f6b-4220-a3f0-9dedfaab0c88.mp4 (autoPlay, loop, muted, playsInline)
Bottom gradient overlay: absolute inset-x-0 bottom-0 h-60 bg-gradient-to-t from-background to-transparent
Content (z-10, flex column, justify-end, pb-10 md:pb-20):
Avatar row: 3 overlapping circular avatars (pravatar.cc, w-8 h-8 rounded-full border-2 border-background, -space-x-2) + text "7,000+ brands already transformed" in text-muted-foreground text-sm
Heading: text-3xl sm:text-5xl md:text-6xl lg:text-7xl font-medium tracking-[-1px] md:tracking-[-2px] — "Build Stunning with " + <span className="font-accent italic font-normal">AI Magic</span>
Subtitle: text-sm md:text-lg text-muted-foreground whitespace-normal md:whitespace-nowrap — "AI-powered websites crafted for beauty, speed, and lasting performance."
Email form: liquid-glass rounded-full p-1.5 md:p-2 max-w-lg w-full with transparent input and a solid white bg-foreground text-background rounded-full SUBSCRIBE button. Button uses motion.button with whileHover={{ scale: 1.03 }} and whileTap={{ scale: 0.98 }}

SECTION 3: ABOUT (scroll-reveal text)

bg-background py-32 px-8, max-w-4xl mx-auto text-center
Uses a ScrollRevealText component: splits text into individual words, each wrapped in a motion.span. Uses useScroll with offset: ["start 0.9", "start 0.3"] and useTransform to animate each word's opacity from 0.15 to 1 as user scrolls through
Text: "We blend artificial intelligence with human creativity to craft digital experiences that captivate, convert, and scale — building ambitious brands that truly thrive and lead in the modern web."
Typography: text-3xl md:text-4xl lg:text-5xl font-medium tracking-[-1px] leading-relaxed font-body

SECTION 4: SELECTED WORK (2×2 project grid)

bg-background py-32 pb-16 px-8, max-w-6xl mx-auto
Header: "Selected " + italic accent "Work" — text-4xl md:text-5xl font-medium tracking-[-2px] text-center mb-4
Subtitle: "A curated collection of projects where bold design meets intelligent technology." — text-muted-foreground text-lg text-center max-w-2xl mx-auto mb-16
Grid: grid-cols-1 md:grid-cols-2 gap-6
4 project cards, each with framer-motion fade-up (y: 40→0, staggered by i * 0.1):
Image container: aspect-[4/3] liquid-glass rounded-2xl overflow-hidden
Project title: text-xl font-medium text-foreground font-body
Category: text-sm text-muted-foreground font-body mt-1
Projects data:
"Nova Finance" / "Brand & Web Design" / https://motionsites.ai/assets/hero-grow-ai-preview-BlQ8tAQ-.gif
"Pulse Health" / "AI Web Development" / https://motionsites.ai/assets/hero-evr-ventures-preview-DZxeVFEX.gif
"Drift Studios" / "Website Optimization" / https://motionsites.ai/assets/hero-wealth-preview-B70idl_u.gif
"Arc Commerce" / "Brand & Development" / https://motionsites.ai/assets/hero-neuralyn-preview-Br4FRDQA.gif

SECTION 5: VIDEO SHOWCASE (parallax overlap)

h-[650px] overflow-hidden -mt-[325px] z-0 — overlaps upward into the previous section
Full-bleed autoplay video: https://media.cleanshot.cloud/media/21620/nKosRonaEKSufJVJ4VtouFhOPkqgJ3dPoQ8ZP52S.mp4
Top & bottom gradient fades: h-32 bg-gradient-to-b/t from-background to-transparent z-10

SECTION 6: CTA (full-screen with HLS video background)

w-full h-screen overflow-hidden flex items-center justify-center z-10
HLS video background using hls.js: Stream URL https://stream.mux.com/4IMYGcL01xjs7ek5ANO17JC4VQVUTsojZlnw4fXzwSxc.m3u8 (with Safari native HLS fallback)
Top/bottom gradient fades: h-40, plus a bg-black/30 overlay
Content (z-10, centered, max-w-3xl):
Heading: "Ready to " + italic accent "Transform" + " Your Brand?" — text-4xl md:text-5xl lg:text-6xl font-medium tracking-[-2px] mb-6
Subtitle: "Let's build something extraordinary together." — text-lg text-muted-foreground mb-10
Two buttons side by side: "START A PROJECT" (solid bg-foreground text-background rounded-full px-10 py-4) and "BOOK A CALL" (liquid-glass-strong rounded-full px-10 py-4)
All elements animate in with framer-motion y: 30/20→0 staggered

SECTION 7: FOOTER

bg-background border-t border-border px-8 py-16, max-w-6xl mx-auto
4-column grid (md:grid-cols-4):
Logo "VIRALMEDIA" + description "AI-powered web design agency crafting digital experiences that convert."
Services: Brand Design, AI Web Design, AI Web Development, Optimization
Company: About, Work, Blog, Careers
Connect: Twitter, LinkedIn, Instagram, Dribbble
Bottom bar: copyright "© 2026 VIRALMEDIA. All rights reserved." + Privacy/Terms links
All links: text-muted-foreground text-sm hover:text-foreground transition-colors

KEY DEPENDENCIES

framer-motion (animations)
hls.js (HLS video streaming in CTA)
Google Fonts: Barlow (400, 500, 600) and Instrument Serif (400 italic) — load via <link> in index.html

## AI Interface — Landing Page [sites/ai-interface]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/sniamtedblueish%20site.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ai-interface.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Cortex — Mind Amplified.</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter+Tight:wght@300;400;500;600;700;900&display=swap');
  </style>
  <script src="https://unpkg.com/react@19/umd/react.production.min.js" crossorigin></script>
  <script src="https://unpkg.com/react-dom@19/umd/react-dom.production.min.js" crossorigin></script>
  <script src="https://unpkg.com/framer-motion@12.42.2/dist/framer-motion.js" crossorigin></script>
  <script src="https://cdn.tailwindcss.com"></script>
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
  <script>
    tailwind.config = {
      theme: {
        extend: {
          fontFamily: { sans: ['Inter Tight', 'sans-serif'] },
          colors: { 'brand-bg': '#122e58' }
        }
      }
    }
  </script>
  <style>
    body {
      background: linear-gradient(180deg, #020715 0%, #051329 35%, #0b264b 65%, #007bb8 88%, #00b8e6 100%) no-repeat;
      background-attachment: fixed;
      color: #ffffff;
      font-family: 'Inter Tight', sans-serif;
      font-weight: 400;
      min-height: 100vh;
      line-height: 1.4;
      overflow-x: hidden;
      -webkit-font-smoothing: antialiased;
      margin: 0;
    }
    ::selection { background: white; color: #122e58; }
    ::-webkit-scrollbar { width: 8px; }
    ::-webkit-scrollbar-track { background: #020715; }
    ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.15); border-radius: 4px; }
    ::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.25); }
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="text/babel" data-type="module">
    const { useRef, useEffect, memo } = React;
    const { motion, AnimatePresence, useScroll, useTransform, useInView } = window["framer-motion"] || FramerMotion;

    // ─── TextEffect Component ───
    const defaultStaggerTimes = { char: 0.03, word: 0.05, line: 0.1 };

    const defaultContainerVariants = {
      hidden: { opacity: 0 },
      visible: { opacity: 1, transition: { staggerChildren: 0.05 } },
      exit: { transition: { staggerChildren: 0.05, staggerDirection: -1 } },
    };

    const defaultItemVariants = {
      hidden: { opacity: 0 },
      visible: { opacity: 1 },
      exit: { opacity: 0 },
    };

    const AnimationComponent = memo(({ segment, variants, per }) => {
      if (per === 'line') {
        return React.createElement(motion.span, { variants, className: 'block' }, segment);
      } else if (per === 'word') {
        return React.createElement(motion.span, { 'aria-hidden': 'true', variants, className: 'inline-block whitespace-pre' }, segment);
      } else {
        return React.createElement(motion.span, { className: 'inline-block whitespace-pre' },
          segment.split('').map((char, i) =>
            React.createElement(motion.span, { key: `char-${i}`, 'aria-hidden': 'true', variants, className: 'inline-block whitespace-pre' }, char)
          )
        );
      }
    });

    function TextEffect({ children, per = 'word', as = 'p', variants: customVariants, className, delay = 0, trigger = true }) {
      let segments;
      if (per === 'line') segments = children.split('\n');
      else if (per === 'word') segments = children.split(/(\s+)/);
      else segments = children.split('');

      const containerVariants = customVariants?.container || defaultContainerVariants;
      const itemVariants = customVariants?.item || defaultItemVariants;
      const stagger = defaultStaggerTimes[per];

      const delayedContainerVariants = {
        hidden: containerVariants.hidden,
        visible: {
          ...containerVariants.visible,
          transition: {
            ...(containerVariants.visible?.transition || {}),
            staggerChildren: containerVariants.visible?.transition?.staggerChildren || stagger,
            delayChildren: delay,
          },
        },
        exit: containerVariants.exit,
      };

      const MotionTag = motion[as] || motion.p;

      return React.createElement(AnimatePresence, null,
        trigger && React.createElement(MotionTag, {
          initial: 'hidden',
          animate: 'visible',
          exit: 'exit',
          variants: delayedContainerVariants,
          className: `whitespace-pre-wrap ${className || ''}`,
        },
          segments.map((segment, index) =>
            React.createElement(AnimationComponent, { key: `${per}-${index}-${segment}`, segment, variants: itemVariants, per })
          )
        )
      );
    }

    // ─── SVG Arrow Icon ───
    function ArrowUpRight({ className }) {
      return React.createElement('svg', {
        xmlns: 'http://www.w3.org/2000/svg',
        width: 24, height: 24,
        viewBox: '0 0 24 24',
        fill: 'none',
        stroke: 'currentColor',
        strokeLinecap: 'round',
        strokeLinejoin: 'round',
        className
      },
        React.createElement('path', { d: 'M7 7h10v10' }),
        React.createElement('path', { d: 'M7 17 17 7' })
      );
    }

    // ─── Animation Variants ───
    const blurSlideVariants = {
      container: {
        hidden: { opacity: 0 },
        visible: { opacity: 1, transition: { staggerChildren: 0.015 } },
        exit: { opacity: 0, transition: { staggerChildren: 0.01, staggerDirection: -1 } },
      },
      item: {
        hidden: { opacity: 0, filter: 'blur(10px) brightness(0%)', y: 20 },
        visible: { opacity: 1, y: 0, filter: 'blur(0px) brightness(100%)', transition: { duration: 0.4, ease: [0.16, 1, 0.3, 1] } },
        exit: { opacity: 0, y: -20, filter: 'blur(10px) brightness(0%)', transition: { duration: 0.3, ease: [0.16, 1, 0.3, 1] } },
      },
    };

    const otherElementVariants = {
      hidden: { opacity: 0, y: 35 },
      visible: { opacity: 1, y: 0, transition: { duration: 0.9, ease: [0.16, 1, 0.3, 1] } },
      exit: { opacity: 0, y: -25, transition: { duration: 0.7, ease: [0.16, 1, 0.3, 1] } },
    };

    // ─── Main App ───
    function App() {
      const scrollContainerRef = useRef(null);
      const videoRef = useRef(null);
      const heroRef = useRef(null);
      const aboutRef = useRef(null);
      const solutionsRef = useRef(null);

      const inViewHero = useInView(heroRef, { amount: 0.15, once: false });
      const inViewAbout = useInView(aboutRef, { amount: 0.15, once: false });
      const inViewSolutions = useInView(solutionsRef, { amount: 0.1, once: false });

      const { scrollYProgress: videoScrollProgress } = useScroll({
        target: scrollContainerRef,
        offset: ["start start", "end start"]
      });
      const videoOpacity = useTransform(videoScrollProgress, [0.9, 1.0], [1, 0]);

      useEffect(() => {
        const video = videoRef.current;
        const container = scrollContainerRef.current;
        if (!video || !container) return;

        let targetProgress = 0;
        let currentProgress = 0;
        let animationFrameId;

        const handleScroll = () => {
          const rect = container.getBoundingClientRect();
          const scrollHeight = container.scrollHeight;
          if (scrollHeight <= 0) return;
          const scrolled = -rect.top;
          targetProgress = Math.max(0, Math.min(1, scrolled / scrollHeight));
        };

        const updateVideoProgress = () => {
          currentProgress += (targetProgress - currentProgress) * 0.08;
          if (Math.abs(targetProgress - currentProgress) < 0.0001) currentProgress = targetProgress;
          const duration = video.duration;
          if (duration && !isNaN(duration)) {
            const targetTime = currentProgress * duration;
            if (!video.seeking && Math.abs(video.currentTime - targetTime) > 0.02) {
              video.currentTime = targetTime;
            }
          }
          animationFrameId = requestAnimationFrame(updateVideoProgress);
        };

        handleScroll();
        currentProgress = targetProgress;
        window.addEventListener('scroll', handleScroll, { passive: true });
        animationFrameId = requestAnimationFrame(updateVideoProgress);

        const handleLoadedMetadata = () => { handleScroll(); currentProgress = targetProgress; };
        video.addEventListener('loadedmetadata', handleLoadedMetadata);

        return () => {
          cancelAnimationFrame(animationFrameId);
          window.removeEventListener('scroll', handleScroll);
          video.removeEventListener('loadedmetadata', handleLoadedMetadata);
        };
      }, []);

      const { scrollYProgress } = useScroll({ target: solutionsRef, offset: ["start start", "end end"] });
      const { scrollYProgress: heroScroll } = useScroll({ target: heroRef, offset: ["start start", "end start"] });

      const heroTitleOpacity = useTransform(heroScroll, [0, 0.45], [1, 0]);
      const heroTitleBlur = useTransform(heroScroll, [0, 0.45], ["blur(0px)", "blur(20px)"]);
      const heroTitleY = useTransform(heroScroll, [0, 0.45], [0, -60]);
      const heroOtherOpacity = useTransform(heroScroll, [0, 0.45], [1, 0]);
      const heroOtherY = useTransform(heroScroll, [0, 0.45], [0, -40]);

      const { scrollYProgress: aboutScroll } = useScroll({ target: aboutRef, offset: ["start end", "end start"] });
      const aboutTitleOpacity = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], [0, 1, 1, 0]);
      const aboutTitleBlur = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], ["blur(20px)", "blur(0px)", "blur(0px)", "blur(20px)"]);
      const aboutTitleY = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], [60, 0, 0, -60]);
      const aboutOtherOpacity = useTransform(aboutScroll, [0.15, 0.35, 0.65, 0.85], [0, 1, 1, 0]);
      const aboutOtherY = useTransform(aboutScroll, [0.15, 0.35, 0.65, 0.85], [50, 0, 0, -50]);

      const opacitySet1 = useTransform(scrollYProgress, [0, 0.05, 0.22, 0.29], [0, 1, 1, 0]);
      const blurSet1 = useTransform(scrollYProgress, [0, 0.05, 0.22, 0.29], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet1 = useTransform(scrollYProgress, [0, 0.29], ["0px", "-120px"]);
      const yBottomSet1 = useTransform(scrollYProgress, [0, 0.29], ["0px", "120px"]);

      const opacitySet2 = useTransform(scrollYProgress, [0.33, 0.40, 0.58, 0.65], [0, 1, 1, 0]);
      const blurSet2 = useTransform(scrollYProgress, [0.33, 0.40, 0.58, 0.65], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet2 = useTransform(scrollYProgress, [0.33, 0.65], ["0px", "-120px"]);
      const yBottomSet2 = useTransform(scrollYProgress, [0.33, 0.65], ["0px", "120px"]);

      const opacitySet3 = useTransform(scrollYProgress, [0.69, 0.76, 0.92, 0.99], [0, 1, 1, 0]);
      const blurSet3 = useTransform(scrollYProgress, [0.69, 0.76, 0.92, 0.99], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet3 = useTransform(scrollYProgress, [0.69, 0.99], ["0px", "-120px"]);
      const yBottomSet3 = useTransform(scrollYProgress, [0.69, 0.99], ["0px", "120px"]);

      const bgScale = useTransform(scrollYProgress, [0, 1], [1.1, 1.0]);

      return React.createElement('div', { className: 'relative w-full min-h-screen' },

        // ─── Header ───
        React.createElement('header', { className: 'fixed top-4 lg:top-5 left-1/2 -translate-x-1/2 z-50 w-[calc(100%-32px)] md:w-auto bg-slate-950/55 backdrop-blur-xl rounded-xl p-1 pl-1 pr-5 flex items-center justify-between md:gap-8 transition-all' },
          React.createElement('div', { className: 'flex items-center justify-center w-10 h-10 bg-white/10 hover:bg-white/15 rounded-lg text-white text-xl select-none leading-none cursor-pointer transition-all duration-300 hover:rotate-45 active:scale-95 shrink-0' }, '\u2733'),
          React.createElement('nav', { className: 'flex items-center gap-4 lg:gap-5' },
            React.createElement('a', { href: '#cortex', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Cortex'),
            React.createElement('a', { href: '#solutions', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Interface'),
            React.createElement('a', { href: '#developer', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Developer'),
            React.createElement('a', { href: '#support', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Support'),
          )
        ),

        // ─── Background Video ───
        React.createElement(motion.div, { style: { opacity: videoOpacity }, className: 'fixed inset-0 w-full h-full z-0 select-none pointer-events-none overflow-hidden' },
          React.createElement('video', {
            ref: videoRef,
            src: 'https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260704_111356_a61893e1-7df9-45d6-a986-a651b6cb7392.mp4',
            className: 'w-full h-full object-cover',
            muted: true,
            playsInline: true,
            preload: 'auto'
          })
        ),

        // ─── Scroll Container (Hero + About) ───
        React.createElement('div', { ref: scrollContainerRef, className: 'relative z-10 w-full bg-transparent' },

          // ─── Hero Section ───
          React.createElement('section', { ref: heroRef, className: 'relative w-full h-screen flex items-center overflow-hidden bg-transparent' },
            React.createElement('main', { className: 'relative z-10 w-full max-w-none mx-auto h-screen px-4 lg:px-[56px] pt-28 lg:pt-0 grid grid-cols-1 lg:grid-cols-12 gap-12 lg:gap-8 items-center' },
              // Left Column
              React.createElement('div', { className: 'lg:col-span-7 flex flex-col justify-center h-full lg:-translate-y-[112px] transform' },
                React.createElement(motion.div, { style: { opacity: heroTitleOpacity, filter: heroTitleBlur, y: heroTitleY } },
                  React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight mb-10 text-white flex flex-col' },
                    React.createElement('span', { className: 'block' },
                      React.createElement(TextEffect, { per: 'char', variants: blurSlideVariants, trigger: inViewHero }, 'Mind')
                    ),
                    React.createElement('span', { className: 'block' },
                      React.createElement(TextEffect, { per: 'char', variants: blurSlideVariants, trigger: inViewHero, delay: 0.15 }, 'Amplified.')
                    )
                  )
                ),
                React.createElement(motion.div, { style: { opacity: heroOtherOpacity, y: heroOtherY } },
                  React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewHero ? 'visible' : 'exit' },
                    React.createElement('a', { href: '#discover', className: 'group inline-flex items-center justify-center bg-white hover:bg-white/90 text-brand-bg rounded-full px-7 py-3.5 text-sm font-normal w-fit gap-3 shadow-none transition-all' },
                      React.createElement('span', { className: 'flex items-center justify-center w-5 h-5 rounded-full bg-brand-bg text-white transition-transform group-hover:scale-105' },
                        React.createElement(ArrowUpRight, { className: 'w-3.5 h-3.5 stroke-[2.5]' })
                      ),
                      React.createElement('span', { className: 'tracking-tight' }, 'Discover Cortex')
                    )
                  )
                )
              ),
              // Right Column
              React.createElement(motion.div, { style: { opacity: heroOtherOpacity, y: heroOtherY }, className: 'lg:col-span-4 lg:col-start-9 flex flex-col justify-center lg:self-end lg:mb-[56px] lg:justify-self-end w-full max-w-[328px]' },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewHero ? 'visible' : 'exit' },
                  React.createElement('div', { className: 'text-[11.5px] font-normal uppercase text-white/50 tracking-[0.15em] mb-3' }, '001 \u2014 Concept'),
                  React.createElement('p', { className: 'text-[14.5px] font-normal leading-relaxed text-white tracking-tight' }, 'A screen is a bottleneck. Cortex is a premium neural interface that streams your intention directly to AI, amplifying your natural mind.')
                )
              )
            )
          ),

          // ─── About Section ───
          React.createElement('section', { ref: aboutRef, className: 'w-full max-w-none mx-auto px-4 lg:px-[56px] h-screen min-h-[600px] py-[56px] flex flex-col justify-between items-start bg-transparent' },
            // Top
            React.createElement('div', { className: 'w-full flex flex-col gap-6' },
              React.createElement(motion.div, { style: { opacity: aboutOtherOpacity, y: aboutOtherY } },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewAbout ? 'visible' : 'exit' },
                  React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '002 \u2014 Neural Extension')
                )
              ),
              React.createElement('div', { className: 'w-full' },
                React.createElement(motion.div, { style: { opacity: aboutTitleOpacity, filter: aboutTitleBlur, y: aboutTitleY } },
                  React.createElement(TextEffect, { per: 'word', as: 'p', variants: blurSlideVariants, trigger: inViewAbout, className: 'text-[clamp(24px,3.2vw,40px)] font-medium leading-[1.25] tracking-tight text-white max-w-[1200px]' },
                    '\u2460 Cortex is a premium, circular neural interface that rests seamlessly on your temple, establishing a real-time thought connection that augments your cognition with advanced AI models.'
                  )
                )
              )
            ),
            // Bottom
            React.createElement('div', { className: 'grid grid-cols-1 lg:grid-cols-12 w-full gap-8' },
              React.createElement(motion.div, { style: { opacity: aboutOtherOpacity, y: aboutOtherY }, className: 'lg:col-start-9 lg:col-span-4 lg:justify-self-end flex flex-col w-full max-w-[328px]' },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewAbout ? 'visible' : 'exit', className: 'w-full' },
                  React.createElement('div', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em] mb-5' }, 'Capabilities:'),
                  React.createElement('div', { className: 'flex flex-col w-full border-b border-white/15' },
                    ['Instant Knowledge Retrieval', 'Seamless Thought Translation', 'Generative Reasoning Flow'].map((item) =>
                      React.createElement('a', { key: item, href: '#', className: 'group flex justify-between items-center py-4 border-t border-white/15 text-white transition-opacity' },
                        React.createElement('span', { className: 'text-[14.5px] font-medium tracking-tight' }, item),
                        React.createElement('span', { className: 'flex items-center justify-center w-5 h-5 rounded-full bg-white text-brand-bg transition-transform group-hover:scale-110 ml-3 shrink-0' },
                          React.createElement(ArrowUpRight, { className: 'w-3.5 h-3.5 stroke-[2.5]' })
                        )
                      )
                    )
                  )
                )
              )
            )
          )
        ),

        // ─── Solutions Section ───
        React.createElement('section', { id: 'solutions', ref: solutionsRef, className: 'w-full min-h-[350vh] bg-transparent relative' },
          React.createElement('div', { className: 'w-full h-screen sticky top-0 overflow-hidden flex flex-col justify-between' },
            // BG Image
            React.createElement('div', { className: 'absolute inset-0 w-full h-full select-none pointer-events-none z-0' },
              React.createElement(motion.img, { src: 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260706_102306_b64c5436-04bc-4cb4-83c1-8c83a83a285c.png&w=1920&q=85', alt: 'Cortex Tech Background', className: 'w-full h-full object-cover', style: { scale: bgScale } })
            ),
            // Content
            React.createElement('div', { className: 'relative z-10 w-full max-w-none mx-auto h-full px-4 lg:px-[56px] flex flex-col justify-center items-start' },
              React.createElement('div', { className: 'w-full max-w-[1000px] h-[320px] lg:h-[400px] relative flex items-center justify-start' },

                // Set 1
                React.createElement(motion.div, { style: { opacity: opacitySet1, filter: blurSet1 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet1 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '003 \u2014 Interface'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Silent thought.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet1 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Cortex.')
                  )
                ),

                // Set 2
                React.createElement(motion.div, { style: { opacity: opacitySet2, filter: blurSet2 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet2 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '004 \u2014 Performance'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Cognitive flow.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet2 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Intuition.')
                  )
                ),

                // Set 3
                React.createElement(motion.div, { style: { opacity: opacitySet3, filter: blurSet3 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet3 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '005 \u2014 Symbiosis'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Instant recall.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet3 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Insight.')
                  )
                )
              )
            )
          )
        )
      );
    }

    const root = ReactDOM.createRoot(document.getElementById('root'));
    root.render(React.createElement(App));
  </script>
</body>
</html>

## AKOR Security — Landing Page [sites/akor-security-landing]

- Preview: https://motionsites.ai/assets/hero-akor-security-preview-hRrwsPNf.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/akor-security-landing.gif

Create a dark-themed single-page landing site for "AKOR — Intelligent Security Systems" using React, Tailwind CSS, and the Sora font (Google Fonts: Sora:wght@300;400;500;600;700).

Design system (CSS custom properties, HSL):
--background: 0 0% 10%, --foreground: 0 0% 96%
--primary: 119 99% 46% (vivid green), --primary-foreground: 0 0% 4%
--muted-foreground: 0 0% 60%, --border: 0 0% 20%
--hero-bg: 0 0% 8%, --nav-button: 0 0% 18%
Body: bg-background text-foreground font-sora antialiased
Global animation: animate-fade-up — a keyframe that fades in from opacity:0 translateY(16px) to opacity:1 translateY(0) over ~600ms ease-out, with animation-fill-mode: forwards.

Section 1 — Fixed Navbar:
Fixed top, full-width, z-50, horizontal flex, px-8 lg:px-16 py-5.
Left: A small 32×32 rounded green (bg-primary) icon box with an inline SVG hexagon, plus the text "AKOR" in text-xl font-semibold tracking-tight text-foreground.
Center (hidden on mobile): Nav links — "Services", "About Us", "Projects", "Team", "Contacts" — styled text-sm text-muted-foreground uppercase tracking-widest, hover → text-foreground.
Right (hidden on mobile): "Get Quote" button with bg-nav-button text-foreground hover:bg-nav-button/80 active:scale-[0.97], rounded-lg uppercase text-xs tracking-widest px-6 h-11.

Section 2 — Hero (full viewport):
min-h-screen flex flex-col justify-end bg-hero-bg overflow-hidden.
Background: an autoplaying, looping, muted <video> covering the entire section via absolute inset-0 w-full h-full object-cover. Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260322_013248_a74099a8-be2b-4164-a823-eddd5e149fa1.mp4
Content sits at the bottom-left: relative z-10 px-8 lg:px-16 pb-20 lg:pb-28 pt-32.
H1: "Intelligent\nSecurity Systems" (line break after "Intelligent"). Styled text-5xl sm:text-6xl lg:text-[5.5rem] font-light leading-[0.95] tracking-tight text-foreground. Animates in with animate-fade-up delay 0.2s.
Subtext: "Innovative security, automation, and AI solutions for businesses and smart cities". Styled text-muted-foreground text-base lg:text-lg max-w-xl mb-10. Delay 0.45s.
Two CTAs side by side (flex flex-wrap gap-8, delay 0.65s):
"Get Consultation" — green button: bg-primary text-primary-foreground font-semibold hover:bg-primary/90 active:scale-[0.97] rounded-lg px-8 uppercase text-xs tracking-widest h-11.
"Learn More" — plain text link: uppercase text-xs tracking-widest text-foreground border-b border-primary pb-1 hover:text-primary transition-colors active:scale-[0.97].

Section 3 — Services (dark foreground background):
bg-foreground (white/light background inverted — since foreground = 0 0% 96%, this section has a near-white bg with dark text via text-background).
Top: label "Services" in text-muted-foreground/60 text-xs uppercase tracking-[0.25em] mb-8, then a full-width 1px divider bg-muted-foreground/20 mb-16.
Two-column layout: flex flex-col lg:flex-row gap-16 lg:gap-24.
Left column (38%): vertically centered heading "Security, automation, and AI, helping businesses enhance efficiency" in text-3xl sm:text-4xl leading-[1.15] tracking-tight text-background font-normal. Below it, a "Get Consultation" green button (same style as hero).
Right column (62%): 4 service cards in a 2×2 grid (grid-cols-1 sm:grid-cols-2), split into two rows with a horizontal 1px divider between them. Each card has pl-8 border-l border-border/20 and contains:
A 64×64 icon image (object-contain)
A number label in text-muted-foreground/40 text-xs
Title in text-xl font-medium leading-tight text-background whitespace-pre-line
Description in text-muted-foreground/50 text-sm leading-relaxed
Card data:
Card 1: "AI-Driven\nSecurity Solutions" — icon: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260322_015134_c80a3c98-609e-4526-b79e-94dc96cd34e8.png
Card 2: "Smart Building\nAutomation" — icon: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260322_014934_6e2804d7-d219-461d-98d5-36140fc90c4c.png&w=1280&q=85
Card 3: "AI Consulting\nand Integration" — icon: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260322_014626_97cccc38-534a-4c9d-a801-68a449da9d0c.png&w=1920&q=85
Card 4: "Training\nand Support" — icon: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260322_014849_570d4eeb-9613-4b19-a084-46c7a2665243.png&w=1280&q=85

Section 4 — About Us (black background):
bg-black pt-12 lg:pt-16 pb-24 lg:pb-32 px-8 lg:px-16.
Top: "About Us" label + divider (same style as Services).
Two-column layout (flex-col lg:flex-row gap-12 lg:gap-0 items-stretch):
Left (45%): an autoplaying, looping, muted video: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260322_011532_86f9b93a-2ffc-42fd-8735-12a4c55ab536.mp4, styled w-full h-auto rounded-sm.
Vertical 1px divider between columns (hidden on mobile): w-px bg-muted-foreground/20 mx-10 mt-8.
Right (flex-1, min-h-[500px] lg:min-h-[600px], flex flex-col justify-between):
Top: heading "AI-powered security, automation for businesses and smart infrastructures" in text-3xl sm:text-4xl leading-[1.15] tracking-tight text-foreground font-normal.
Bottom (mt-auto): paragraph about mission + "Get Quote" green button (px-10).

## Apex Pulse — Landing Page [sites/apex-pulse]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(44).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/apex-pulse.webp

Build a dark, cinematic drone/UAV technology landing page called **"AETHER_X"** using React + Vite + TypeScript + Tailwind CSS. The site has a scroll-driven video background and two content sections with reveal-on-scroll animations. Here is the exact specification:

---

### Font & Global Styles

- Load **Helvetica Neue Regular** via this stylesheet in `index.html`:
  ```
  https://db.onlinewebfonts.com/c/0d49fc455f4a8951a42daf952412a713?family=Helvetica+Neue+Regular
  ```
- Body: `font-family: 'Helvetica Neue Regular', 'Helvetica Neue', Helvetica, Arial, sans-serif`, background `#0a0a0a`, color `#fff`, antialiased.
- In `tailwind.config.js`, set `fontFamily.sans` and `fontFamily.mono` to `'"Flexo Soft Medium"', 'system-ui', 'sans-serif'`.
- Selection highlight: `rgba(255, 255, 255, 0.2)`.
- Page title: `AETHER_X — Where Sky Meets Machine Logic`

---

### Dependencies

Only: `react`, `react-dom`, `lucide-react`, `@supabase/supabase-js`. Tailwind + PostCSS + Autoprefixer + Vite for tooling.

---

### Architecture (5 components)

```
App.tsx
  -> ScrollVideo (fixed fullscreen background)
  -> Navbar (fixed top)
  -> main
       -> SectionOne (hero, bottom-aligned)
       -> spacer div (h-[80vh], aria-hidden)
       -> SectionTwo (full-height, justify-between)
```

---

### Component 1: `Reveal.tsx` (scroll-triggered animation wrapper)

- Props: `children`, `delay` (ms, default 0), `className`, `as` ('div' | 'span', default 'div')
- Uses `IntersectionObserver` with `threshold: 0.15`
- Animates from `translate-y-8 opacity-0` to `translate-y-0 opacity-100`
- Transition: `duration-700 ease-out will-change-transform`
- `transitionDelay` applied via inline style from `delay` prop

---

Component 2: ScrollVideo.tsx (frame-extracted scroll-synced video)
Video URL (CloudFront):

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_230900_ef8565a6-16eb-4fe9-98e4-4b972d3f436d.mp4
Architecture principle: The <video> element starts visible as a live fallback. The rAF loop starts immediately on mount and seeks the video on scroll. Once frame extraction finishes, the canvas becomes visible and the video is hidden -- seamless handoff with zero blank frames.
Implementation:
1. State: framesReady boolean (default false). framesRef holds extracted ImageBitmap[].
2. First useEffect (frame extraction, runs once):
    * Fetch video as blob, create object URL
    * Extract up to 120 frames at max width 1280px using createImageBitmap + sequential seek-based extraction (derive frame count from video.duration * 24, clamped between 30-120)
    * On success: store frames in framesRef.current, set framesReady = true
    * On failure or cancellation: do nothing -- the video fallback stays visible permanently
    * Cleanup: set cancelled flag, revoke object URL, close all ImageBitmaps
3. Second useEffect (rAF scroll-sync loop, deps: [framesReady]):
    * Starts immediately on mount (when framesReady is still false) -- this is critical
    * Re-runs when framesReady flips to true (cleans up old loop, starts new one with frame drawing)
    * Scroll handler: maps scrollY / (scrollHeight - innerHeight) to 0-1 progress, clamped
    * Resize handler: sets canvas dimensions to clientWidth * dpr x clientHeight * dpr (dpr capped at 2)
    * Tick function (rAF):
        * Smooth interpolation: smoothed += (targetProgress - smoothed) * 0.1
        * If frames exist (frames.length > 1): map smoothed to frame index, draw to canvas with cover-fit math
        * Else if video element exists (fallback): seek video.currentTime to smoothed * duration (with seek-lock to avoid stacking seeks)
    * Cover-fit draw: scale = max(canvasW/frameW, canvasH/frameH), center the scaled frame
4. Render structure:  <div class="fixed inset-0 -z-10 bg-[#0a0a0a]">
5.   {!framesReady && <video ... visible, muted, playsInline, preload="auto", object-cover />}
6.   <canvas ... class includes 'invisible' when !framesReady, removes it when framesReady />
7.   <div class="absolute inset-0 bg-black/20" /> (overlay)
8. </div>
9.   
Key detail: The <video> is conditionally rendered (!framesReady), so it is present and visible during loading. The canvas starts with invisible class and only becomes visible when framesReady flips. This guarantees something is always on screen -- the video during load, the canvas after extraction.



### Component 3: `Navbar.tsx`

- Fixed top, z-50, flex between, responsive padding (`px-5 py-4 sm:px-8 sm:py-5 md:px-12`)
- Left: custom SVG logo (white, 32x32, scales to 36x36 on sm+). SVG path:
  ```
  M 160 88 L 194 34 L 216 0 L 256 0 L 256 40 L 221.5 93.5 L 200 128 L 256 128 L 256 256 L 96 256 L 96 168 L 64.246 220 L 40 256 L 0 256 L 0 216 L 34 162 L 56 128 L 0 128 L 0 0 L 160 0 Z
  ```
- Right: pill button "Join The Fleet" -- `rounded-full bg-white text-black uppercase tracking-wider text-xs (sm:text-sm)`, hover: `bg-white/90 scale-105`
- Both wrapped in `<Reveal>` (logo delay 0, button delay 150)

---

### Component 4: `SectionOne.tsx` (Hero)

- Full viewport height, content at bottom (`flex-col justify-end`)
- 2-column grid on sm+, single column mobile
- **Column 1:** Large heading with 3 stacked `<Reveal as="span">` lines:
  - "Relentless." (delay 100)
  - "Sovereign." (delay 250)
  - "Unyielding." (delay 400)
  - Font size: `clamp(2.5rem, 8vw, 6rem)`, font-medium, leading-[1.05], tracking-tight, drop-shadow-lg
- **Column 2:** 3 stats in a flex-wrap row (right-aligned on sm+), delay 550:
  - `360` + degree symbol + "Sensor Array"
  - `12` + degree symbol + "Thermal Scan"
  - `98` + % + "Precision"
  - Values: `text-2xl sm:text-3xl md:text-4xl font-bold tabular-nums`
  - Labels: `text-[10px] sm:text-xs uppercase tracking-wider text-white/50`

---

### Component 5: `SectionTwo.tsx`

- Full viewport, `flex-col justify-between`, responsive padding
- **Top-right block** (self-end, max-w-sm):
  - Heading (delay 100): "Precision / built into / every line." -- `text-3xl sm:text-4xl md:text-5xl font-semibold leading-[1.1]`
  - Stat row (delay 250, mt-8): 2-column grid `[auto_1fr]`
    - Left: "99.7%" bold + label "Fleet Ready"
    - Right: paragraph "Proven in 14,000+ sorties with unmatched operational readiness and zero downtime." -- `text-xs sm:text-sm text-white/70`

- **Bottom area** (mt-auto pt-16 sm:pt-20, 2-col grid on sm+):
  - Column 1 (delay 400): 3 stats in flex row:
    - `240 km` / "Reach"
    - `85 kg` / "Capacity"
    - `42 km/h` / "Glide Speed"
  - Column 2 (delay 550, sm:ml-auto):
    - 4 stats: `grid-cols-2 sm:flex`. First stat has `border border-white/30 rounded-lg`:
      - `160 km` / "Reach"
      - `5.8 hrs` / "Endurance"
      - `52 km/h` / "Max Pace"
      - `18 kg` / "Carry"
    - 2 pill buttons below:
      - "Schedule a Call" -- outline style (`border border-white/60 rounded-full`), hover: `border-white bg-white/10`
      - "Full Details" -- solid white, text-black, hover: `bg-white/90 scale-105`

---

### Key responsive breakpoints:
- Mobile: single column, smaller text, reduced padding
- `sm` (640px): 2-column grids activate, larger text
- `md` (768px): max padding/text sizes

### Color palette:
- Background: `#0a0a0a`
- Text: white, `white/70`, `white/60`, `white/50`
- Borders: `white/30`, `white/60`
- Overlay: `black/20`

## Art Landing — Landing Page [sites/art-landing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/a/mezzanine%20(1).mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/art-landing.m3u8

Build a two-section scroll-based landing page using React 19, TypeScript, Vite, Tailwind CSS v4, and `motion/react` (Framer Motion). The page uses Manrope, Italiana, and Marck Script fonts, with a video hero and a red second section featuring a cloud transition.

### Setup

**package.json dependencies:**
- `react` ^19, `react-dom` ^19
- `motion` ^12 (for `motion/react`)
- `tailwindcss` ^4.1, `@tailwindcss/vite` ^4.1
- `vite` ^6, `@vitejs/plugin-react` ^5
- `lucide-react`, `typescript` ~5.8

**vite.config.ts:** include `@vitejs/plugin-react` and `@tailwindcss/vite` plugins.

### src/index.css

```css
@import url('https://fonts.googleapis.com/css2?family=Italiana&family=Manrope:wght@400;600&family=Marck+Script&display=swap');
@import "tailwindcss";

@theme {
  --font-manrope: "Manrope", sans-serif;
  --font-italiana: "Italiana", serif;
  --font-marck: "Marck Script", cursive;
}
```

### src/App.tsx — Structure

**Root:** `<main>` with ref `containerRef`, classes `h-screen overflow-y-auto overflow-x-hidden font-manrope bg-black relative`.

**Scroll setup:**
```tsx
const containerRef = useRef<HTMLDivElement>(null);
const { scrollY } = useScroll({ container: containerRef });
const cloudYDesktop = useTransform(scrollY, [0, 300], [0, -100]);
const cloudYMobile  = useTransform(scrollY, [0, 300], [0, -24]);
```

### Section 1 — Video Hero

`<section className="relative h-screen w-full flex-shrink-0 overflow-hidden">`

- **Background video** (absolute inset-0, z-10, `w-full h-full object-cover`, autoPlay loop muted playsInline):
  - src: `https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/baby-track-video_crqby5.mp4`
- **Overlay** `absolute inset-0 z-30 pointer-events-none`.

**Top-left logo block** (`absolute top-[24px] left-[20px] md:top-[64px] md:left-[64px] pointer-events-auto max-w-[calc(100vw-140px)] md:max-w-none`):
- Flex row, gap-[16px] md:gap-[24px], items-center.
- SVG logo, white fill, 48x48 mobile / 64x64 desktop, viewBox `0 0 120 120`, path:
  `M60 120C26.8629 120 0 93.1371 0 60V0C22.5654 0 42.2213 12.4569 52.4662 30.8691C38.4788 34.2089 28.0787 46.7902 28.0787 61.8006V63.1443C28.0787 79.9648 41.7146 93.6006 58.5353 93.6006H59.8789L59.8785 61.8006C59.8785 79.3633 74.1159 93.6006 91.6787 93.6006L91.6787 61.8006C91.6787 44.2783 77.5071 30.0661 60 30.0008L60 0H62.5352C94.2722 0 120 25.7279 120 57.4648V60C120 93.1371 93.1371 120 60 120Z`
- Tagline: white, `text-[11px] md:text-[16px] w-[112px] md:w-auto leading-[1.2] font-semibold tracking-[0.02em]`.
  - Desktop (`hidden md:block`): "Effortless Growth / Operations. We Handle All Tasks. / Stay Calm." with `<br />` after each.
  - Mobile (`block md:hidden`): "Complete Business / Automation. We Handle All / Tasks. You Relax."

**Left description** (desktop only, below logo): `hidden md:flex mt-[400px] flex-col gap-[24px] w-full max-w-[320px] text-white text-[14px] font-normal leading-relaxed`. Two paragraphs about SaaS automation.

**Top-right CTA button** (`absolute top-[24px] right-[20px] md:top-[64px] md:right-[64px]`):
`px-5 py-3 md:px-10 md:py-7 border border-white rounded-[100%] text-white text-[12px] md:text-[18px] font-italiana uppercase tracking-widest hover:bg-white/10 hover:backdrop-blur-[48px] transition-all duration-300 cursor-pointer bg-black/10 backdrop-blur-sm md:bg-transparent md:backdrop-blur-none`
Label: "Get started".

**Bottom heading container** (`absolute bottom-[32px] left-[20px] right-[20px] md:left-auto md:bottom-[64px] md:right-[64px] md:max-w-[1200px] text-left md:text-right`):
- Mobile paragraphs (`md:hidden flex flex-col gap-[16px] max-w-[280px] text-white text-[12px] font-normal mb-[32px]`).
- `<h1 className="text-white text-[36px] leading-[1.1] md:text-[96px] font-italiana md:leading-[88px]">`:
  - Desktop: "Intelligent Daily / Routine Automation / For Your Business. / You Relax".
  - Mobile (`text-[32px]`): "Intelligent Daily Routine / Automation For Your / Business. You Relax".

### Section 2 — Red Background

`<section className="relative min-h-screen w-full bg-[#FF0000] flex flex-col z-10">`

**Cloud overlays** (two `motion.div`, one desktop, one mobile, both absolute top-0 left-0 w-full z-[100] pointer-events-none `-translate-y-1/2`):
- style `y: cloudYDesktop` / `y: cloudYMobile`.
- `<img src="https://res.cloudinary.com/dsdhxhhqh/image/upload/v1781500777/cloude_vj4pjv.png" className="w-full h-auto block" referrerPolicy="no-referrer" />`

**Content wrapper:** `flex-1 flex flex-col items-center w-full pt-[100px] md:pt-[400px]`.

Inner content block (`flex flex-col items-center w-full px-8 text-center z-20 relative max-w-[900px] h-auto md:h-[620px] mx-auto`):
- Same SVG logo, 80x80, white.
- Paragraph: `text-white text-[16px] h-[100px] max-w-[400px] leading-[1.6] mb-[40px] uppercase tracking-wider mx-auto`. Text: "We built this platform with a single purpose to eliminate operational chaos and restore balance to your daily business routine".
- Signature: `font-marck text-white text-[120px] leading-none mb-[32px]` reading `S.P.D`.
- Two centered paragraphs: white, `text-[16px] w-[400px] max-w-full`, font-light, first with `mb-[24px]`, container `mb-[100px] md:mb-24`.

**Bottom video block** (`relative w-full shrink-0`):
- Top fade: `absolute top-0 left-0 w-full h-[100px] bg-gradient-to-b from-[#FF0000] to-transparent z-10 pointer-events-none`.
- Video (autoPlay loop muted playsInline, `w-full h-auto block object-contain`):
  - src: `https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/track-video_2_haxdch.mp4`

### Animations
- Cloud parallax: maps scroll 0→300px to translateY 0→-100px (desktop) and 0→-24px (mobile), via `useTransform` with the section's container scroll.
- Button hover: background fades to `white/10` with `backdrop-blur-[48px]` over 300ms.

### Notes
- Videos are Cloudinary, not CloudFront. There are no CloudFront URLs in this project.
- All assets above are the only external URLs used.

## Clarity Core — Landing Page [sites/bio-active]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(30).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bio-active.webp

Build a premium single-page landing site for "LumiDerm" — a luxury skincare/biotech brand. Use React + TypeScript + Vite + Tailwind CSS. Use lucide-react for icons. Use the font "Helvetica Now Text" loaded from this CDN:
https://db.onlinewebfonts.com/c/08e020de1811ec4489f82d1247a42c09?family=Helvetica+Now+Text

Set the page title to "LumiDerm | Advanced Skin Science".

The site has 3 components: SplashScreen, Navbar, and HeroSection. The entire page is fixed/fullscreen with overflow hidden, no scrollbar. Black background base.

---

### SECTION 1: Splash Screen (SplashScreen.tsx)

A loading intro that plays on first load:
- Full-screen overlay at z-[9999], bg-[#010101].
- Shows a progress bar at the bottom (px-12, bottom-12). Left label says "Loading" in white/40, uppercase, tracking-[0.2em], xs text. Right shows the percentage in white, sm, tabular-nums font-medium.
- The progress bar is 1px tall, bg-white/10 track, with a white fill that animates from 0% to 100% over 2400ms (20ms interval steps).
- Once 100% is reached, wait 300ms, then trigger a "curtain reveal" animation:
  - Two panels (left half and right half) slide apart: left translates -100% X, right translates +100% X.
  - Duration: 1200ms, easing: cubic-bezier(0.76, 0, 0.24, 1).
  - The loader content fades to opacity-0 with duration-300 during reveal.
- After 1200ms of the curtain animation, call onComplete to signal the main content can appear.

---

### SECTION 2: Navbar (Navbar.tsx)

A floating pill-shaped navbar, fixed at the top center:
- Position: fixed top-4 (sm:top-6), centered, z-50, px-4 on the wrapper.
- Nav pill: bg-white/10, backdrop-blur-md, border border-white/15, rounded-full, px-2 py-2.
- On mobile: full-width with justify-between. On sm+: auto width.
- Appears with a transition: opacity and translateY(-4 to 0) over 700ms with 300ms delay, triggered by `isActive` prop.

Contents:
- Logo: text-base font-medium tracking-tight. "Lumi" in font-bold, "Derm" in font-light. Both white.
- Desktop nav links (hidden md:flex): "Science", "Treatments", "Results", "Testimonials", "Connect". Each: text-white/80, text-sm, hover:text-white, hover:bg-white/10, px-4 py-1.5, rounded-full.
- CTA button (hidden sm:flex): "Book Now" with ArrowRight icon. bg-[#5794E2], text-white text-sm, px-5 py-2, rounded-full, hover:bg-[#4a84d0].
- Mobile hamburger (md:hidden): w-9 h-9 rounded-full bg-white/10, toggles between Menu and X icons.
- Mobile overlay: fixed inset-0 z-40, bg-black/90 backdrop-blur-lg. Shows the same 5 links as text-2xl font-light centered vertically with gap-6, plus the "Book Now" button. Transitions opacity over 300ms.

---

### SECTION 3: Hero Section (HeroSection.tsx)

A fullscreen video background hero with scroll-driven crossfade between two states.

**Video backgrounds:** (not looping, plays once then pause)
- Video 1 URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260619_222559_363e35af-d0bc-4650-b3cb-58bf833daa51.mp4`
- Video 2 URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260619_232048_98292efb-9b9c-4089-a587-72f33437c8f8.mp4`
- Both: absolute inset-0, w-full h-full, object-cover, muted, playsInline, preload="auto".
- Video 1 opacity = 1 - progress. Video 2 opacity = progress.


**Scroll-driven progress (0 to 1):**
- Listen to wheel events (passive: false, preventDefault).
- Delta = e.deltaY / 800. Clamp progress between 0 and 1.
- When progress crosses 0.5 threshold, pause one video and play the other from the start.
- Video 1 plays when progress < 0.5. Video 2 plays when progress >= 0.5.

**Hero State 1 (progress near 0) — Bottom Left content:**
- Opacity: max(0, 1 - progress * 2.5). TranslateY: progress * 30px.
- Position: absolute bottom-12 left-6 right-6, sm:bottom-8 sm:left-8 sm:right-auto, md:bottom-16 md:left-12. max-w-2xl.
- H1: "The Future of\nSkin Regeneration". text-3xl sm:text-4xl md:text-6xl lg:text-7xl, font-light, leading-[1.05], tracking-tight, mb-4 md:mb-6.
- P: "Heal with science, not guesswork. LumiDerm merges cellular research and bioactive formulations to unlock your skin's true radiant potential." text-xs sm:text-sm md:text-base, text-white/70, max-w-md, mb-6 md:mb-8.
- Button: "Explore Now". bg-[#5794E2], rounded-full, text-xs sm:text-sm, px-6 sm:px-8, py-3 sm:py-3.5.

**Hero State 1 — White Card (bottom right, hidden on mobile):**
- hidden sm:flex. Position: absolute bottom-8 right-8 md:bottom-16 md:right-12.
- bg-white rounded-2xl p-4 md:p-5, items-center gap-4 md:gap-5, shadow-2xl, max-w-[380px] md:max-w-[460px].
- Same opacity/transform as Hero 1 content.
- Left: thumbnail image (w-20 h-20 md:w-24 md:h-24, rounded-xl, object-cover). URL: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260619_220258_2f77857f-c799-4cce-9818-442542b98f2a.png&w=1280&q=85`
- Right: "LumiDerm" and "BioActive" (text-sm font-medium, text-[#010101]), with a blue circle button (w-9 h-9, bg-[#3b82f6]) containing ArrowRight icon. Below: "CellBoost(TM) 3.0" in text-[#666] text-xs.

**Hero State 2 (progress near 1) — Bottom Center content:**
- Opacity: max(0, (progress - 0.4) * 2.5). TranslateY: max(0, (1 - progress) * 30)px.
- Position: absolute bottom-12 sm:bottom-12 md:bottom-20, left-0 right-0, centered, px-6.
- H2: "Clinically Advanced\nSkin Science". text-2xl sm:text-3xl md:text-5xl lg:text-6xl, font-light, leading-[1.1], tracking-tight, mb-4 md:mb-5.
- P: "Our patented peptide complex penetrates at the cellular level, stimulating natural collagen and restoring youthful elasticity." text-xs sm:text-sm md:text-base, text-white/70, max-w-lg.

**Hero State 2 — Stats (mobile: horizontal below text, desktop: right side vertical):**

Mobile (flex sm:hidden, mt-6, gap-6, horizontal):
- "8M+" / "Skin Transformed"
- "96.4%" / "Visible Renewal"
- "37" / "Patents Granted"
- Numbers: text-xl font-light. Labels: text-[10px] text-white/50. Dividers: w-[1px] h-8 bg-[#5794E2]/40.

Desktop (hidden sm:flex, absolute right-8 md:right-12, top-1/2, vertical, gap-8):
- Same 3 stats. Numbers: text-3xl md:text-4xl font-light. Labels: text-xs md:text-sm, text-white/50.
- Blue separator lines: w-12 h-[1px] bg-[#5794E2]/60, mt-4 after first two.
- Transform: translateY(calc(-50% + hero2TranslateY px)).

---

### App.tsx

- State: splashComplete (boolean, starts false).
- Renders SplashScreen (only when not complete), Navbar, and HeroSection.
- Both Navbar and HeroSection receive `isActive={splashComplete}`.

### Global CSS (index.css)

- Tailwind directives (@tailwind base/components/utilities).
- Global reset: * { margin:0; padding:0; box-sizing:border-box }
- html, body, #root: width/height 100%, overflow hidden, font-family: 'Helvetica Now Text', Helvetica, Arial, sans-serif.

### Tailwind Config

- Extend fontFamily with: helvetica: ['"Helvetica Now Text"', 'Helvetica', 'Arial', 'sans-serif']

Make everything fully mobile responsive. The site should feel like an Apple-level luxury product page with smooth animations and premium typography.
```

## Bloom — Landing Page [sites/bl]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(96).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/bl.webp

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Bloom</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital,wght@0,400;1,400&family=Manrope:wght@400;500;600;700&family=Space+Grotesk:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<style>
*,*::before,*::after{box-sizing:border-box}
::-webkit-scrollbar{width:6px}
::-webkit-scrollbar-track{background:#000}
::-webkit-scrollbar-thumb{background:#1f2937;border-radius:3px}
::-webkit-scrollbar-thumb:hover{background:#374151}
body{background:#000;color:#fff;overflow-x:hidden;margin:0;padding:0;font-family:"Space Grotesk",ui-sans-serif,system-ui,sans-serif}

/* --- CONTAINER --- */
.scroll-container{position:relative;width:100%;height:500vh;background:#000}
.fixed-viewport{position:fixed;inset:0;width:100%;height:100%;overflow:hidden;display:flex;align-items:center;justify-content:center;background:#000}

/* --- BRAND HEADER --- */
.premium-brand-header{position:absolute;top:clamp(24px,3vw,48px);left:clamp(24px,3vw,48px);z-index:45;display:flex;align-items:center;gap:clamp(8px,0.8vw,14px);user-select:none;pointer-events:auto;will-change:transform,opacity,filter}
@media(max-width:768px){.premium-brand-header{top:24px;left:24px}}
.premium-brand-text{font-family:"Instrument Serif",serif;font-size:clamp(22px,2.2vw,32px);font-weight:500;color:#fff;letter-spacing:-0.03em}
.premium-brand-logo-svg{width:clamp(28px,2.6vw,44px);height:auto;opacity:0.95}

/* --- NAV HEADER --- */
.premium-header-nav{position:absolute;top:clamp(24px,3vw,48px);right:clamp(24px,3vw,48px);z-index:50;pointer-events:auto;will-change:transform,opacity,filter}
@media(max-width:768px){.premium-header-nav{top:24px;right:24px}}
.premium-nav-desktop{display:block}
.premium-nav-list{display:flex;gap:0;margin:0;padding:0;list-style:none;align-items:center}
.premium-nav-item{margin:0;padding:0}
.premium-nav-link{display:block;background:#fff;color:#111;font-family:"Manrope",sans-serif;font-size:clamp(12px,0.9vw,15px);font-weight:500;text-decoration:none;padding:clamp(8px,0.7vw,12px) clamp(16px,1.3vw,24px);border-radius:8px;transition:opacity .2s ease,transform .2s ease;white-space:nowrap}
.premium-nav-link:hover{opacity:0.9;transform:scale(1.03)}
.premium-nav-link:active{transform:scale(0.97)}

/* --- HAMBURGER --- */
.premium-hamburger{display:none}
@media(max-width:768px){
  .premium-nav-desktop{display:none}
  .premium-hamburger{display:flex;flex-direction:column;justify-content:center;align-items:center;gap:5px;width:44px;height:44px;background:rgba(255,255,255,.12);backdrop-filter:blur(24px);-webkit-backdrop-filter:blur(24px);border:1px solid rgba(255,255,255,.15);border-radius:10px;cursor:pointer;padding:0;z-index:60;position:relative}
  .premium-hamburger:active{transform:scale(0.92)}
}
.premium-hamburger-line{display:block;width:20px;height:1.5px;background:#fff;border-radius:2px;transition:transform .3s cubic-bezier(.16,1,.3,1),opacity .2s ease;transform-origin:center}
.premium-hamburger-line.top-open{transform:translateY(6.5px) rotate(45deg)}
.premium-hamburger-line.mid-open{opacity:0}
.premium-hamburger-line.bot-open{transform:translateY(-6.5px) rotate(-45deg)}

/* --- MOBILE MENU --- */
.premium-mobile-menu{position:fixed;inset:0;z-index:55;background:rgba(0,0,0,.88);backdrop-filter:blur(40px);-webkit-backdrop-filter:blur(40px);display:flex;align-items:center;justify-content:center;pointer-events:auto;opacity:0;visibility:hidden;transition:opacity .3s ease,visibility .3s ease}
.premium-mobile-menu.open{opacity:1;visibility:visible}
.premium-mobile-menu-list{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;align-items:center;gap:8px}
.premium-mobile-menu-link{font-family:"Instrument Serif",Georgia,serif;font-size:40px;font-weight:400;color:#fff;text-decoration:none;letter-spacing:-0.02em;line-height:1.3;padding:8px 24px;transition:opacity .2s ease}
.premium-mobile-menu-link:hover,.premium-mobile-menu-link:active{opacity:0.6}

/* --- BOTANICAL CARD --- */
.premium-botanical-card{position:absolute;bottom:clamp(24px,3vw,48px);left:clamp(24px,3vw,48px);width:clamp(380px,30vw,540px);max-width:90vw;background:rgba(255,255,255,.16);backdrop-filter:blur(80px);-webkit-backdrop-filter:blur(80px);border-radius:0;padding:clamp(32px,3.2vw,56px);z-index:45;pointer-events:auto;border:1px solid rgba(255,255,255,.15);box-shadow:0 30px 60px rgba(0,0,0,.3);will-change:transform,opacity,filter}
@media(max-width:768px){.premium-botanical-card{bottom:32px;left:24px;padding:32px 24px;width:calc(100vw - 48px)}}
.premium-card-title{font-family:"Instrument Serif",serif;font-size:72px;line-height:1.05;font-weight:400;color:#fff;margin:0 0 clamp(10px,1.2vw,20px) 0;letter-spacing:-0.01em;width:324px;max-width:100%}
.premium-card-title .italic{font-style:italic}
@media(max-width:768px){.premium-card-title{font-size:38px;margin-bottom:10px}}
.premium-card-subtext{font-family:"Manrope",sans-serif;font-size:clamp(12px,1vw,15px);line-height:1.6;font-weight:400;color:rgba(255,255,255,.64);letter-spacing:0.01em;margin:0;width:clamp(260px,22vw,380px);max-width:100%}
@media(max-width:768px){.premium-card-subtext{font-size:13px}}

/* --- CTA CIRCLE --- */
.premium-action-circle{position:absolute;right:calc(-1*(clamp(80px,7vw,112px)/2));bottom:clamp(20px,2.5vw,40px);width:clamp(80px,7vw,112px);height:clamp(80px,7vw,112px);border-radius:50%;background:#CB8DFF;color:#fff;border:none;cursor:pointer;display:flex;align-items:center;justify-content:center;transition:transform .25s cubic-bezier(.16,1,.3,1),background-color .2s ease;z-index:55;box-shadow:none}
.premium-action-circle:hover{transform:scale(1.08);background:#d9a8ff}
.premium-action-circle:active{transform:scale(0.94)}
.premium-action-circle svg{width:clamp(30px,2.8vw,44px);height:clamp(18px,1.8vw,28px);stroke:#fff;stroke-width:1.75;fill:none;transition:transform .25s ease}
.premium-action-circle:hover svg{transform:translateX(4px)}
@media(max-width:768px){.premium-action-circle{width:64px;height:64px;right:-16px;bottom:-32px}.premium-action-circle svg{width:38px;height:22px}}

/* --- FEATURE CARDS --- */
.premium-features-container{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);width:max-content;max-width:95%;display:grid;grid-template-columns:repeat(3,280px);justify-content:center;gap:16px;z-index:46;pointer-events:none;align-items:stretch}
.premium-feature-card{background:rgba(255,255,255,.16);backdrop-filter:blur(80px);-webkit-backdrop-filter:blur(80px);border:1px solid rgba(255,255,255,.15);border-radius:0;padding:24px;width:280px;height:440px;box-shadow:0 30px 60px rgba(0,0,0,.3);display:flex;flex-direction:column;justify-content:flex-start;align-items:flex-start;transition:border-color .3s ease;will-change:transform,opacity,filter}
.premium-feature-card:hover{border-color:rgba(255,255,255,.45)}
.premium-feature-icon-wrapper{margin-bottom:32px;color:#CB8DFF;display:flex;align-items:center;justify-content:center}
.premium-feature-icon-wrapper svg{width:clamp(36px,3.5vw,48px);height:clamp(36px,3.5vw,48px);opacity:0.95}
.premium-feature-title{font-family:"Instrument Serif",Georgia,serif;font-size:24px;font-weight:400;line-height:1.15;color:#fff;margin:auto 0 8px 0;letter-spacing:-0.01em}
.premium-feature-desc{font-family:"Manrope",sans-serif;font-size:clamp(12px,1vw,14px);line-height:20px;font-weight:400;color:rgba(255,255,255,.64);margin:0;letter-spacing:0.015em}
@media(max-width:768px){
  .premium-features-container{grid-template-columns:1fr;width:calc(100% - 48px);left:24px;transform:translate(0,-50%);top:50%;gap:16px;height:auto;overflow-y:auto;max-height:80vh}
  .premium-feature-card{padding:24px;width:100%;height:auto;min-height:240px}
  .premium-feature-icon-wrapper{margin-bottom:24px}
}

/* --- MISSION TEXT --- */
.mission-container{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;pointer-events:none;z-index:40;padding:0 24px}
@media(min-width:768px){.mission-container{padding:0 48px}}
.mission-inner{position:relative;width:100%;max-width:64rem;height:100%;display:flex;align-items:center;justify-content:center}
.mission-para{position:absolute;text-align:center;user-select:none;pointer-events:none;font-family:"Instrument Serif",Georgia,serif;font-size:1.875rem;color:rgba(255,255,255,.95);line-height:1.375;letter-spacing:-0.025em;will-change:transform,opacity,filter}
@media(min-width:768px){.mission-para{font-size:3rem}}
@media(min-width:1024px){.mission-para{font-size:3.75rem}}
.mission-pill{display:inline-block;padding:2px 16px;margin:0 6px;background:#CB8DFF;color:#fff;border-radius:9999px;vertical-align:middle;font-family:"Instrument Serif",Georgia,serif}
@media(min-width:768px){.mission-pill{padding:4px 24px;margin:0 10px}}

/* --- FEEDBACK FORM --- */
.feedback-container{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;pointer-events:none;z-index:50;padding:0 24px}
@media(min-width:768px){.feedback-container{padding:0 48px}}
.feedback-form-card{position:relative;width:100%;max-width:28rem;background:rgba(0,0,0,.45);backdrop-filter:blur(48px);-webkit-backdrop-filter:blur(48px);border:1px solid rgba(255,255,255,.1);padding:32px;text-align:center;pointer-events:auto;display:flex;flex-direction:column;align-items:center;justify-content:center;user-select:none;will-change:transform,opacity,filter}
@media(min-width:768px){.feedback-form-card{padding:48px}}
.fb-logo{width:40px;height:40px;margin-bottom:20px}
.fb-title{font-family:"Instrument Serif",Georgia,serif;font-size:1.5rem;color:#fff;font-weight:400;letter-spacing:-0.025em;margin:0 0 8px 0;line-height:1.2;white-space:nowrap}
@media(min-width:768px){.fb-title{font-size:1.875rem}}
.fb-sub{font-family:"Space Grotesk",sans-serif;font-size:10px;color:rgba(255,255,255,.5);text-transform:uppercase;letter-spacing:0.05em;margin:0 0 28px 0;line-height:1.625;max-width:20rem}
@media(min-width:768px){.fb-sub{font-size:12px}}
.fb-form{width:100%;display:flex;flex-direction:column;gap:14px}
.fb-input{width:100%;padding:12px 20px;background:rgba(255,255,255,.05);color:#fff;border:1px solid rgba(255,255,255,.1);font-family:"Space Grotesk",sans-serif;font-size:14px;letter-spacing:0.025em;text-align:center;outline:none;transition:all .3s ease}
.fb-input::placeholder{color:rgba(255,255,255,.3)}
.fb-input:hover,.fb-input:focus{background:rgba(255,255,255,.1)}
.fb-input:focus{border-color:#CB8DFF}
@media(min-width:768px){.fb-input{padding:16px 20px;font-size:16px}}
.fb-btn{width:100%;padding:14px 24px;background:#CB8DFF;color:#fff;border:none;cursor:pointer;font-family:"Space Grotesk",sans-serif;font-size:12px;font-weight:600;text-transform:uppercase;letter-spacing:0.1em;transition:all .3s ease}
.fb-btn:hover{background:#d9a8ff}
.fb-btn:active{transform:scale(0.98)}
.fb-btn.submitted{background:#059669;cursor:default}
@media(min-width:768px){.fb-btn{padding:16px 24px}}
.fb-success{margin-top:16px;font-family:"Space Grotesk",sans-serif;font-size:12px;color:#34d399;font-weight:500;letter-spacing:0.025em;line-height:1.625;display:none}
.fb-success.show{display:block;animation:fadeIn .5s ease}

/* --- VIDEO BG --- */
.bg-wrapper{position:absolute;top:-5%;left:-5%;right:-5%;bottom:-5%;width:110%;height:110%;pointer-events:none;user-select:none;transform:translate3d(0,0,0);transition:transform .4s cubic-bezier(.15,.85,.35,1)}
.bg-wrapper video{position:absolute;width:0;height:0;opacity:0}
.bg-wrapper canvas{width:100%;height:100%;display:block}

/* --- LOADING --- */
.loading-screen{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;background:#000;z-index:60;transition:opacity 1s ease;padding:24px;text-align:center}
.loading-screen.hidden{opacity:0;pointer-events:none}
.loading-spinner{width:64px;height:64px;margin-bottom:16px;animation:spin 2s linear infinite}
@keyframes spin{to{transform:rotate(360deg)}}
.loading-text{font-family:"JetBrains Mono",monospace;font-size:14px;letter-spacing:0.1em;color:#6b7280;text-transform:uppercase;animation:pulse 2s ease-in-out infinite}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:.5}}
@keyframes fadeIn{from{opacity:0}to{opacity:1}}

/* --- ERROR --- */
.error-screen{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;background:#000;z-index:55;padding:24px;text-align:center;display:none}
.error-screen.show{display:flex}
.error-card{padding:16px;border:1px solid #1f2937;border-radius:4px;background:#09090b;max-width:28rem}
.error-title{color:#ef4444;font-family:"JetBrains Mono",monospace;font-size:14px;margin:0 0 8px 0}
.error-msg{color:#9ca3af;font-family:"JetBrains Mono",monospace;font-size:12px;line-height:1.625;margin:0}
</style>
</head>
<body>

<div class="scroll-container" id="scrollContainer">
  <div class="fixed-viewport">

    <!-- Brand header -->
    <div class="premium-brand-header" id="brandHeader">
      <svg viewBox="0 0 49 48" fill="none" xmlns="http://www.w3.org/2000/svg" class="premium-brand-logo-svg">
        <g clip-path="url(#cb)">
          <path d="M48.4404 17.6588C47.7017 15.3617 46.2569 13.3584 44.3131 11.9362C42.3693 10.5141 40.0265 9.74617 37.6207 9.74268H37.0462C37.1235 10.2805 37.164 10.8231 37.1676 11.3665C37.1628 14.3622 36.0924 17.2579 34.1489 19.5323C32.2054 21.8066 29.5166 23.3102 26.5664 23.7724C26.6392 24.0729 26.7121 24.3733 26.8092 24.6655C26.9904 25.2224 27.2122 25.7651 27.4728 26.2894L27.4728 26.3543C27.5942 26.5898 27.7236 26.8252 27.8612 27.0526L27.9178 27.15C28.3242 27.8128 28.796 28.433 29.3259 29.0011L29.5687 29.2609L29.9652 29.6263C30.0866 29.74 30.2161 29.8536 30.3537 29.9592L30.7098 30.2515C30.8797 30.3814 31.0577 30.5032 31.2358 30.625L31.5595 30.8523C31.8508 31.0309 32.1502 31.2095 32.4577 31.3638C33.787 32.0468 35.2378 32.4594 36.7266 32.5778C38.2154 32.6963 39.7129 32.5182 41.1329 32.0539C41.4083 31.9667 41.6785 31.8637 41.9421 31.7454C44.6033 30.6611 46.7546 28.6033 47.961 25.9882C49.1674 23.3731 49.3387 20.3958 48.4404 17.6588Z" fill="white"/>
          <path d="M41.4966 33.1341C40.2425 33.5456 38.9316 33.7566 37.6122 33.7593C35.2977 33.7531 33.0303 33.1034 31.0617 31.8822C29.0931 30.6611 27.5005 28.9163 26.4607 26.8418C26.1937 27.0042 25.9428 27.1747 25.6515 27.3614C25.1901 27.702 24.7546 28.0765 24.3486 28.4819L24.3 28.5306C24.1058 28.7173 23.9278 28.9122 23.7497 29.1151L23.685 29.1963C23.1783 29.7906 22.7336 30.4354 22.3578 31.1206C22.3012 31.2261 22.2445 31.3235 22.196 31.4291C22.1474 31.5346 22.0341 31.7538 21.9613 31.9162C21.8885 32.0786 21.8237 32.2329 21.7671 32.3952C21.7104 32.5576 21.6538 32.6794 21.6052 32.8256C21.5567 32.9717 21.4677 33.2315 21.411 33.4345C21.3544 33.6375 21.3382 33.6699 21.3139 33.7917C21.2236 34.1255 21.1534 34.4644 21.1035 34.8066C20.8672 36.2866 20.9251 37.7989 21.2737 39.2564C21.6223 40.7139 22.2548 42.0879 23.1347 43.2992C23.2966 43.5184 23.4746 43.7376 23.6526 43.9406C25.3534 46.0837 27.7685 47.5385 30.4534 48.0372C33.1384 48.045 35.9125 48.045 38.2653 46.6547C40.6181 45.2645 42.3913 43.0685 43.2586 40.4708C44.1259 37.8732 44.029 35.0487 42.9856 32.517C42.5042 32.7574 42.0067 32.9636 41.4966 33.1341Z" fill="white"/>
          <path d="M20.0045 34.6197C20.4175 31.9862 21.6629 29.5556 23.5571 27.686C23.3224 27.4912 23.0796 27.2882 22.8287 27.1096C21.2988 25.9892 19.5119 25.2743 17.6334 25.0311H17.3987C16.9941 24.9905 16.5894 24.958 16.1686 24.958C14.3714 24.962 12.6002 25.3895 10.9978 26.206C9.39542 27.0226 8.00658 28.2053 6.9432 29.659C6.78135 29.8863 6.63568 30.1217 6.49002 30.3572C5.03583 32.6434 4.4443 35.3759 4.82225 38.0613C5.2002 40.7467 6.52271 43.208 8.55098 45.0008C10.5793 46.7937 13.1796 47.7998 15.8825 47.8376C18.5854 47.8754 21.2127 46.9424 23.29 45.207C22.9121 44.8221 22.5632 44.4096 22.2461 43.9729C21.2738 42.6404 20.575 41.1275 20.1902 39.5219C19.8054 37.9163 19.7423 36.25 20.0045 34.6197Z" fill="white"/>
          <path d="M6.01393 28.9525C7.76739 26.527 10.3287 24.8119 13.234 24.1178C16.1392 23.4238 19.1961 23.7967 21.8509 25.169C21.9723 24.8848 22.0856 24.6006 22.1827 24.3002C22.7183 22.6659 22.8815 20.9318 22.6602 19.2258V18.9903C22.6602 18.7955 22.5873 18.6006 22.5469 18.4139C22.5064 18.2271 22.4821 18.1054 22.4417 17.9592C22.4012 17.8131 22.3608 17.6426 22.3122 17.4802C22.2636 17.3178 22.1827 17.0905 22.118 16.8956C22.0532 16.7008 22.029 16.652 21.9804 16.5384C21.8586 16.2165 21.7181 15.902 21.5596 15.5966C20.8885 14.2565 19.9583 13.0639 18.8232 12.0881C17.6881 11.1124 16.3708 10.3731 14.948 9.91321C14.6972 9.83202 14.4544 9.76706 14.2035 9.71023C13.2994 9.47628 12.3696 9.35628 11.4359 9.35299C8.9636 9.30745 6.54433 10.0747 4.54726 11.5375C2.55019 13.0004 1.08498 15.0786 0.375205 17.4551C-0.334567 19.8315 -0.249922 22.3757 0.616224 24.6993C1.48237 27.023 3.08245 28.9986 5.17231 30.3246C5.42183 29.8488 5.70301 29.3904 6.01393 28.9525Z" fill="white"/>
          <path d="M22.5278 15.0688C23.735 17.4435 24.1546 20.143 23.7255 22.7738C24.025 22.7738 24.3325 22.8225 24.6481 22.8225C25.2227 22.8148 25.796 22.766 26.3637 22.6764C28.9618 22.273 31.3392 20.9756 33.0885 19.0065L33.2585 18.8198C33.4041 18.6493 33.5417 18.4707 33.6793 18.2839C33.8169 18.0972 33.833 18.0891 33.9059 17.9835C33.9787 17.878 34.1406 17.6425 34.2458 17.472C34.351 17.3015 34.4076 17.2122 34.4804 17.0823C34.5533 16.9524 34.6666 16.7575 34.7475 16.587C34.8284 16.4165 34.9013 16.2785 34.9741 16.1243C35.0469 15.97 35.1117 15.8076 35.1845 15.6452C35.2573 15.4829 35.314 15.2961 35.3706 15.1175C35.4273 14.9389 35.4839 14.8008 35.5325 14.6466C35.581 14.4923 35.6377 14.2488 35.6862 14.0458C35.7348 13.8428 35.7591 13.7535 35.7833 13.6073C35.8076 13.4612 35.8643 13.1364 35.8966 12.8929C35.929 12.6493 35.8966 12.6493 35.9452 12.5194C35.9452 12.154 36.0018 11.7887 36.0018 11.4071V10.5952C35.8453 7.88465 34.727 5.31985 32.8491 3.36487C30.9712 1.40989 28.4579 0.193859 25.7639 -0.0631835C23.0699 -0.320226 20.3731 0.398699 18.1616 1.96351C15.9501 3.52831 14.37 5.83563 13.707 8.46797C14.2332 8.56239 14.7523 8.69259 15.2608 8.85768C16.8205 9.36099 18.2656 10.1689 19.5128 11.2349C20.76 12.3008 21.7847 13.6038 22.5278 15.0688Z" fill="white"/>
        </g>
        <defs><clipPath id="cb"><rect width="49" height="48" fill="white"/></clipPath></defs>
      </svg>
      <span class="premium-brand-text">Bloom</span>
    </div>

    <!-- Navigation -->
    <header class="premium-header-nav" id="navHeader">
      <nav class="premium-nav-desktop">
        <ul class="premium-nav-list">
          <li class="premium-nav-item"><a href="#atelier" class="premium-nav-link">Atelier</a></li>
          <li class="premium-nav-item"><a href="#collections" class="premium-nav-link">Collections</a></li>
          <li class="premium-nav-item"><a href="#rituals" class="premium-nav-link">Rituals</a></li>
          <li class="premium-nav-item"><a href="#about" class="premium-nav-link">About</a></li>
          <li class="premium-nav-item"><a href="#contact" class="premium-nav-link">Contact</a></li>
        </ul>
      </nav>
      <button class="premium-hamburger" id="hamburgerBtn" aria-label="Open menu">
        <span class="premium-hamburger-line" id="hLine1"></span>
        <span class="premium-hamburger-line" id="hLine2"></span>
        <span class="premium-hamburger-line" id="hLine3"></span>
      </button>
    </header>

    <!-- Mobile menu -->
    <div class="premium-mobile-menu" id="mobileMenu">
      <nav>
        <ul class="premium-mobile-menu-list">
          <li><a href="#atelier" class="premium-mobile-menu-link mobile-link">Atelier</a></li>
          <li><a href="#collections" class="premium-mobile-menu-link mobile-link">Collections</a></li>
          <li><a href="#rituals" class="premium-mobile-menu-link mobile-link">Rituals</a></li>
          <li><a href="#about" class="premium-mobile-menu-link mobile-link">About</a></li>
          <li><a href="#contact" class="premium-mobile-menu-link mobile-link">Contact</a></li>
        </ul>
      </nav>
    </div>

    <!-- Botanical card -->
    <div class="premium-botanical-card" id="botanicalCard">
      <h2 class="premium-card-title">Merging <span class="italic">Silicon</span> With Organic <span class="italic">Life.</span></h2>
      <p class="premium-card-subtext">Developing Next-Generation Cyber-Botanical Systems Designed To Heal Ecosystems And Advance Human Tech.</p>
      <button class="premium-action-circle" id="ctaBtn" aria-label="Explore systems">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 40 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" d="M29.5 4.5L37 12m0 0l-7.5 7.5M37 12H3"/>
        </svg>
      </button>
    </div>

    <!-- Feature cards -->
    <div class="premium-features-container" id="featuresContainer" style="visibility:hidden">
      <div class="premium-feature-card" id="fCard1" style="opacity:0">
        <div class="premium-feature-icon-wrapper">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="9"/></svg>
        </div>
        <h3 class="premium-feature-title">Neural Synthesis</h3>
        <p class="premium-feature-desc">Hybrid bio-computing linking mycelium networks with logical silicon cores.</p>
      </div>
      <div class="premium-feature-card" id="fCard2" style="opacity:0">
        <div class="premium-feature-icon-wrapper">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v18m9-9H3m14.5-4.5l-11 11m11 0l-11-11"/></svg>
        </div>
        <h3 class="premium-feature-title">Ecosystem Remediation</h3>
        <p class="premium-feature-desc">Self-replicating biomechanical flora actively restoring and cleansing heavily toxic soil bases.</p>
      </div>
      <div class="premium-feature-card" id="fCard3" style="opacity:0">
        <div class="premium-feature-icon-wrapper">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M16 3L7 12h12l-6 9"/></svg>
        </div>
        <h3 class="premium-feature-title">Kinetic Transduction</h3>
        <p class="premium-feature-desc">Converting natural photosynthesis cycles into electrical energy for local grids.</p>
      </div>
    </div>

    <!-- Mission text -->
    <div class="mission-container" id="missionContainer" style="visibility:hidden">
      <div class="mission-inner">
        <div class="mission-para" id="mPara1" style="opacity:0">To gracefully cultivate a newly balanced <span class="mission-pill">ecosystem</span> we dissolve all boundaries between technology and nature.</div>
        <div class="mission-para" id="mPara2" style="opacity:0">By rewriting the biological code of our <span class="mission-pill">planet</span> we employ specialized photosynthesis to heal broken landscapes.</div>
        <div class="mission-para" id="mPara3" style="opacity:0">We believe that growing through botanical <span class="mission-pill">symbiosis</span> is the ultimate pathway to power the future of humanity.</div>
      </div>
    </div>

    <!-- Feedback form -->
    <div class="feedback-container" id="feedbackContainer" style="visibility:hidden">
      <div class="feedback-form-card" id="feedbackForm" style="opacity:0">
        <svg viewBox="0 0 49 48" fill="none" xmlns="http://www.w3.org/2000/svg" class="fb-logo">
          <g clip-path="url(#cf)">
            <path d="M48.4404 17.6588C47.7017 15.3617 46.2569 13.3584 44.3131 11.9362C42.3693 10.5141 40.0265 9.74617 37.6207 9.74268H37.0462C37.1235 10.2805 37.164 10.8231 37.1676 11.3665C37.1628 14.3622 36.0924 17.2579 34.1489 19.5323C32.2054 21.8066 29.5166 23.3102 26.5664 23.7724C26.6392 24.0729 26.7121 24.3733 26.8092 24.6655C26.9904 25.2224 27.2122 25.7651 27.4728 26.2894L27.4728 26.3543C27.5942 26.5898 27.7236 26.8252 27.8612 27.0526L27.9178 27.15C28.3242 27.8128 28.796 28.433 29.3259 29.0011L29.5687 29.2609L29.9652 29.6263C30.0866 29.74 30.2161 29.8536 30.3537 29.9592L30.7098 30.2515C30.8797 30.3814 31.0577 30.5032 31.2358 30.625L31.5595 30.8523C31.8508 31.0309 32.1502 31.2095 32.4577 31.3638C33.787 32.0468 35.2378 32.4594 36.7266 32.5778C38.2154 32.6963 39.7129 32.5182 41.1329 32.0539C41.4083 31.9667 41.6785 31.8637 41.9421 31.7454C44.6033 30.6611 46.7546 28.6033 47.961 25.9882C49.1674 23.3731 49.3387 20.3958 48.4404 17.6588Z" fill="white"/>
            <path d="M41.4966 33.1341C40.2425 33.5456 38.9316 33.7566 37.6122 33.7593C35.2977 33.7531 33.0303 33.1034 31.0617 31.8822C29.0931 30.6611 27.5005 28.9163 26.4607 26.8418C26.1937 27.0042 25.9428 27.1747 25.6515 27.3614C25.1901 27.702 24.7546 28.0765 24.3486 28.4819L24.3 28.5306C24.1058 28.7173 23.9278 28.9122 23.7497 29.1151L23.685 29.1963C23.1783 29.7906 22.7336 30.4354 22.3578 31.1206C22.3012 31.2261 22.2445 31.3235 22.196 31.4291C22.1474 31.5346 22.0341 31.7538 21.9613 31.9162C21.8885 32.0786 21.8237 32.2329 21.7671 32.3952C21.7104 32.5576 21.6538 32.6794 21.6052 32.8256C21.5567 32.9717 21.4677 33.2315 21.411 33.4345C21.3544 33.6375 21.3382 33.6699 21.3139 33.7917C21.2236 34.1255 21.1534 34.4644 21.1035 34.8066C20.8672 36.2866 20.9251 37.7989 21.2737 39.2564C21.6223 40.7139 22.2548 42.0879 23.1347 43.2992C23.2966 43.5184 23.4746 43.7376 23.6526 43.9406C25.3534 46.0837 27.7685 47.5385 30.4534 48.0372C33.1384 48.045 35.9125 48.045 38.2653 46.6547C40.6181 45.2645 42.3913 43.0685 43.2586 40.4708C44.1259 37.8732 44.029 35.0487 42.9856 32.517C42.5042 32.7574 42.0067 32.9636 41.4966 33.1341Z" fill="white"/>
            <path d="M20.0045 34.6197C20.4175 31.9862 21.6629 29.5556 23.5571 27.686C23.3224 27.4912 23.0796 27.2882 22.8287 27.1096C21.2988 25.9892 19.5119 25.2743 17.6334 25.0311H17.3987C16.9941 24.9905 16.5894 24.958 16.1686 24.958C14.3714 24.962 12.6002 25.3895 10.9978 26.206C9.39542 27.0226 8.00658 28.2053 6.9432 29.659C6.78135 29.8863 6.63568 30.1217 6.49002 30.3572C5.03583 32.6434 4.4443 35.3759 4.82225 38.0613C5.2002 40.7467 6.52271 43.208 8.55098 45.0008C10.5793 46.7937 13.1796 47.7998 15.8825 47.8376C18.5854 47.8754 21.2127 46.9424 23.29 45.207C22.9121 44.8221 22.5632 44.4096 22.2461 43.9729C21.2738 42.6404 20.575 41.1275 20.1902 39.5219C19.8054 37.9163 19.7423 36.25 20.0045 34.6197Z" fill="white"/>
            <path d="M6.01393 28.9525C7.76739 26.527 10.3287 24.8119 13.234 24.1178C16.1392 23.4238 19.1961 23.7967 21.8509 25.169C21.9723 24.8848 22.0856 24.6006 22.1827 24.3002C22.7183 22.6659 22.8815 20.9318 22.6602 19.2258V18.9903C22.6602 18.7955 22.5873 18.6006 22.5469 18.4139C22.5064 18.2271 22.4821 18.1054 22.4417 17.9592C22.4012 17.8131 22.3608 17.6426 22.3122 17.4802C22.2636 17.3178 22.1827 17.0905 22.118 16.8956C22.0532 16.7008 22.029 16.652 21.9804 16.5384C21.8586 16.2165 21.7181 15.902 21.5596 15.5966C20.8885 14.2565 19.9583 13.0639 18.8232 12.0881C17.6881 11.1124 16.3708 10.3731 14.948 9.91321C14.6972 9.83202 14.4544 9.76706 14.2035 9.71023C13.2994 9.47628 12.3696 9.35628 11.4359 9.35299C8.9636 9.30745 6.54433 10.0747 4.54726 11.5375C2.55019 13.0004 1.08498 15.0786 0.375205 17.4551C-0.334567 19.8315 -0.249922 22.3757 0.616224 24.6993C1.48237 27.023 3.08245 28.9986 5.17231 30.3246C5.42183 29.8488 5.70301 29.3904 6.01393 28.9525Z" fill="white"/>
            <path d="M22.5278 15.0688C23.735 17.4435 24.1546 20.143 23.7255 22.7738C24.025 22.7738 24.3325 22.8225 24.6481 22.8225C25.2227 22.8148 25.796 22.766 26.3637 22.6764C28.9618 22.273 31.3392 20.9756 33.0885 19.0065L33.2585 18.8198C33.4041 18.6493 33.5417 18.4707 33.6793 18.2839C33.8169 18.0972 33.833 18.0891 33.9059 17.9835C33.9787 17.878 34.1406 17.6425 34.2458 17.472C34.351 17.3015 34.4076 17.2122 34.4804 17.0823C34.5533 16.9524 34.6666 16.7575 34.7475 16.587C34.8284 16.4165 34.9013 16.2785 34.9741 16.1243C35.0469 15.97 35.1117 15.8076 35.1845 15.6452C35.2573 15.4829 35.314 15.2961 35.3706 15.1175C35.4273 14.9389 35.4839 14.8008 35.5325 14.6466C35.581 14.4923 35.6377 14.2488 35.6862 14.0458C35.7348 13.8428 35.7591 13.7535 35.7833 13.6073C35.8076 13.4612 35.8643 13.1364 35.8966 12.8929C35.929 12.6493 35.8966 12.6493 35.9452 12.5194C35.9452 12.154 36.0018 11.7887 36.0018 11.4071V10.5952C35.8453 7.88465 34.727 5.31985 32.8491 3.36487C30.9712 1.40989 28.4579 0.193859 25.7639 -0.0631835C23.0699 -0.320226 20.3731 0.398699 18.1616 1.96351C15.9501 3.52831 14.37 5.83563 13.707 8.46797C14.2332 8.56239 14.7523 8.69259 15.2608 8.85768C16.8205 9.36099 18.2656 10.1689 19.5128 11.2349C20.76 12.3008 21.7847 13.6038 22.5278 15.0688Z" fill="white"/>
          </g>
          <defs><clipPath id="cf"><rect width="49" height="48" fill="white"/></clipPath></defs>
        </svg>
        <h3 class="fb-title">Cultivate alignment</h3>
        <p class="fb-sub">Subscribe to biological updates &amp; cybernetic releases.</p>
        <form class="fb-form" id="fbForm">
          <input type="email" required placeholder="name@ecosystem.com" class="fb-input" id="fbEmail">
          <button type="submit" class="fb-btn" id="fbBtn">CONNECT</button>
        </form>
        <p class="fb-success" id="fbSuccess">Welcome to the digital flora. Check your inbox soon.</p>
      </div>
    </div>

    <!-- Video background -->
    <div class="bg-wrapper" id="bgWrapper">
      <video id="v1" src="https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/202606101700_hglz7q.mp4" preload="auto" muted playsinline></video>
      <video id="v2" src="https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/202606101702_sd50y0.mp4" preload="auto" muted playsinline></video>
      <video id="v3" src="https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/202606101703_jmidj2.mp4" preload="auto" muted playsinline></video>
      <canvas id="mainCanvas"></canvas>
    </div>

    <!-- Loading -->
    <div class="loading-screen" id="loadingScreen">
      <svg class="loading-spinner" viewBox="0 0 64 64" style="transform:rotate(-90deg)">
        <circle cx="32" cy="32" r="28" stroke="#1f2937" stroke-width="2" fill="transparent"/>
        <circle cx="32" cy="32" r="28" stroke="#fff" stroke-width="3" fill="transparent" stroke-dasharray="176" stroke-dashoffset="132" stroke-linecap="round"/>
      </svg>
      <p class="loading-text">Loading experience</p>
    </div>

    <!-- Error -->
    <div class="error-screen" id="errorScreen">
      <div class="error-card">
        <p class="error-title">Video Load Error</p>
        <p class="error-msg">Unable to load video resources. Please check your connection and try reloading.</p>
      </div>
    </div>

  </div>
</div>

<script>
(function(){
  var container = document.getElementById('scrollContainer');
  var videos = { v1: document.getElementById('v1'), v2: document.getElementById('v2'), v3: document.getElementById('v3') };
  var canvas = document.getElementById('mainCanvas');
  var bgWrapper = document.getElementById('bgWrapper');
  var brandHeader = document.getElementById('brandHeader');
  var navHeader = document.getElementById('navHeader');
  var botanicalCard = document.getElementById('botanicalCard');
  var featuresContainer = document.getElementById('featuresContainer');
  var fCard1 = document.getElementById('fCard1');
  var fCard2 = document.getElementById('fCard2');
  var fCard3 = document.getElementById('fCard3');
  var missionContainer = document.getElementById('missionContainer');
  var mPara1 = document.getElementById('mPara1');
  var mPara2 = document.getElementById('mPara2');
  var mPara3 = document.getElementById('mPara3');
  var feedbackContainer = document.getElementById('feedbackContainer');
  var feedbackForm = document.getElementById('feedbackForm');
  var loadingScreen = document.getElementById('loadingScreen');
  var errorScreen = document.getElementById('errorScreen');

  var targetProgress = 0, currentProgress = 0;
  var durations = { v1: 0, v2: 0, v3: 0 };
  var metaLoaded = { v1: false, v2: false, v3: false };
  var seeking = { v1: false, v2: false, v3: false };
  var pendingSeek = { v1: -1, v2: -1, v3: -1 };
  var offscreen = null;

  function getActiveKey(p) { return p <= 0.333 ? 'v1' : p <= 0.666 ? 'v2' : 'v3'; }

  // Canvas draw with double-buffer
  function drawFrame() {
    var p = currentProgress;
    var key = getActiveKey(p);
    var video = videos[key];
    if (!video || video.readyState < 2) return;
    var dpr = window.devicePixelRatio || 1;
    var cW = canvas.width / dpr, cH = canvas.height / dpr;
    if (cW === 0 || cH === 0) return;
    var vW = video.videoWidth || 1920, vH = video.videoHeight || 1080;
    var vA = vW / vH, cA = cW / cH;
    var dW = cW, dH = cH, oX = 0, oY = 0;
    if (vA > cA) { dW = cH * vA; oX = (cW - dW) / 2; }
    else { dH = cW / vA; oY = (cH - dH) / 2; }
    if (!offscreen) offscreen = document.createElement('canvas');
    if (offscreen.width !== canvas.width || offscreen.height !== canvas.height) {
      offscreen.width = canvas.width; offscreen.height = canvas.height;
    }
    var oc = offscreen.getContext('2d');
    if (!oc) return;
    try {
      oc.setTransform(dpr, 0, 0, dpr, 0, 0);
      oc.clearRect(0, 0, cW, cH);
      oc.drawImage(video, oX, oY, dW, dH);
      var ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.drawImage(offscreen, 0, 0);
    } catch(e) {}
  }

  function handleResize() {
    var parent = canvas.parentElement;
    if (!parent) return;
    var dpr = window.devicePixelRatio || 1;
    var w = parent.clientWidth, h = parent.clientHeight;
    canvas.width = w * dpr; canvas.height = h * dpr;
    canvas.style.width = w + 'px'; canvas.style.height = h + 'px';
    if (offscreen) { offscreen.width = canvas.width; offscreen.height = canvas.height; }
    drawFrame();
  }

  function safeSeek(key, targetTime) {
    var video = videos[key];
    if (!video) return;
    var dur = durations[key] || video.duration || 10;
    var clamped = Math.max(0, Math.min(targetTime, dur - 0.05));
    if (Math.abs(video.currentTime - clamped) < 0.01) return;
    if (seeking[key]) { pendingSeek[key] = clamped; return; }
    seeking[key] = true; pendingSeek[key] = -1;
    video.currentTime = clamped;
  }

  ['v1','v2','v3'].forEach(function(key) {
    videos[key].addEventListener('seeked', function() {
      seeking[key] = false;
      if (key === getActiveKey(currentProgress)) drawFrame();
      if (pendingSeek[key] >= 0) { var t = pendingSeek[key]; pendingSeek[key] = -1; safeSeek(key, t); }
    });
  });

  // Loading
  function checkAllReady() {
    if (metaLoaded.v1 && metaLoaded.v2 && metaLoaded.v3) {
      setTimeout(function() { loadingScreen.classList.add('hidden'); handleResize(); }, 100);
    }
  }
  ['v1','v2','v3'].forEach(function(key) {
    var el = videos[key];
    function onMeta() {
      var d = el.duration;
      if (d && !isNaN(d) && d > 0 && d !== Infinity) { durations[key] = d; metaLoaded[key] = true; checkAllReady(); }
    }
    el.addEventListener('loadedmetadata', onMeta);
    el.addEventListener('durationchange', onMeta);
    el.addEventListener('canplay', onMeta);
    el.addEventListener('error', function() { errorScreen.classList.add('show'); });
    if (el.readyState >= 1) onMeta();
  });
  setTimeout(function() {
    ['v1','v2','v3'].forEach(function(key) {
      if (!metaLoaded[key]) { durations[key] = videos[key].duration || 10; metaLoaded[key] = true; }
    });
    checkAllReady();
  }, 8000);

  window.addEventListener('resize', handleResize);

  // Cursor parallax
  window.addEventListener('mousemove', function(e) {
    var x = (e.clientX - window.innerWidth / 2) / (window.innerWidth / 2);
    var y = (e.clientY - window.innerHeight / 2) / (window.innerHeight / 2);
    bgWrapper.style.transform = 'translate3d(' + (x * -24) + 'px,' + (y * -24) + 'px,0)';
  });

  // Scroll progress
  window.addEventListener('scroll', function() {
    var scrollTop = window.pageYOffset || document.documentElement.scrollTop;
    var maxScroll = container.scrollHeight - window.innerHeight;
    targetProgress = maxScroll > 0 ? Math.max(0, Math.min(1, scrollTop / maxScroll)) : 0;
  }, { passive: true });

  // Helpers
  function setVisible(el, v) { if (!el) return; el.style.visibility = v ? 'visible' : 'hidden'; el.style.pointerEvents = v ? 'auto' : 'none'; }
  function applyStyle(el, op, tx, ty, bl) {
    if (!el) return;
    el.style.opacity = op;
    el.style.transform = 'translate3d(' + tx + 'px,' + ty + 'px,0)';
    el.style.filter = 'blur(' + bl + 'px)';
  }

  // Main rAF loop
  function tick() {
    currentProgress += (targetProgress - currentProgress) * 0.08;
    var p = currentProgress;

    var activeKey = getActiveKey(p);
    var pLocal = activeKey === 'v1' ? Math.max(0, Math.min(1, p * 3))
      : activeKey === 'v2' ? Math.max(0, Math.min(1, (p - 0.333) * 3))
      : Math.max(0, Math.min(1, (p - 0.666) * 3));
    safeSeek(activeKey, pLocal * (durations[activeKey] || 10));

    // Botanical Card
    var f = Math.min(1, Math.max(0, p / 0.15));
    applyStyle(botanicalCard, 1 - f, f * -35, f * 35, f * 16);
    botanicalCard.style.pointerEvents = f > 0.95 ? 'none' : 'auto';

    // Nav header
    f = Math.min(1, Math.max(0, (p - 0.03) / 0.15));
    applyStyle(navHeader, 1 - f, 0, f * -35, f * 14);
    navHeader.style.pointerEvents = f > 0.95 ? 'none' : 'auto';

    // Brand header
    f = Math.min(1, Math.max(0, (p - 0.06) / 0.15));
    applyStyle(brandHeader, 1 - f, f * -25, f * -35, f * 12);
    brandHeader.style.pointerEvents = f > 0.95 ? 'none' : 'auto';

    // Feature cards
    var show = p > 0.13 && p < 0.55;
    setVisible(featuresContainer, show);
    function animCard(el, enterStart, enterDur, exitStart, exitDur, tx, ty) {
      if (!el) return;
      var op = 0, bl = 16, x = tx, y = ty;
      if (p >= enterStart && p <= 0.333) {
        var r = Math.min(1, Math.max(0, (p - enterStart) / enterDur));
        op = r; bl = (1 - r) * 16; x = (1 - r) * tx; y = (1 - r) * ty;
      } else if (p > 0.333 && p <= exitStart + exitDur) {
        var r = Math.min(1, Math.max(0, (p - exitStart) / exitDur));
        op = 1 - r; bl = r * 16; x = r * tx; y = r * ty;
      }
      applyStyle(el, op, x, y, bl);
    }
    animCard(fCard1, 0.15, 0.15, 0.333, 0.12, -35, 35);
    animCard(fCard2, 0.18, 0.13, 0.333, 0.15, 0, 35);
    animCard(fCard3, 0.21, 0.11, 0.333, 0.18, 35, 35);

    // Mission paragraphs
    setVisible(missionContainer, p > 0.44);
    function animPara(el, inS, inE, holdE, outE) {
      if (!el) return;
      var op = 0, bl = 20, y = 120;
      if (p >= inS && p < inE) { var r = (p - inS) / (inE - inS); op = r; bl = (1 - r) * 20; y = (1 - r) * 120; }
      else if (p >= inE && p <= holdE) { op = 1; bl = 0; y = 0; }
      else if (p > holdE && p <= outE) { var r = (p - holdE) / (outE - holdE); op = 1 - r; bl = r * 20; y = r * -120; }
      else if (p > outE) { op = 0; bl = 20; y = -120; }
      applyStyle(el, op, 0, y, bl);
    }
    animPara(mPara1, 0.44, 0.47, 0.59, 0.62);
    animPara(mPara2, 0.62, 0.65, 0.77, 0.80);
    animPara(mPara3, 0.80, 0.83, 0.95, 0.98);

    // Feedback form
    setVisible(feedbackContainer, p > 0.93);
    if (p > 0.93) {
      var opF = 0, blF = 20, yF = 120;
      if (p >= 0.94 && p < 0.97) { var r = (p - 0.94) / 0.03; opF = r; blF = (1 - r) * 20; yF = (1 - r) * 120; }
      else if (p >= 0.97) { opF = 1; blF = 0; yF = 0; }
      applyStyle(feedbackForm, opF, 0, yF, blF);
    }

    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);

  // Hamburger menu
  var menuOpen = false;
  var hamburgerBtn = document.getElementById('hamburgerBtn');
  var mobileMenu = document.getElementById('mobileMenu');
  var hLine1 = document.getElementById('hLine1');
  var hLine2 = document.getElementById('hLine2');
  var hLine3 = document.getElementById('hLine3');
  hamburgerBtn.addEventListener('click', function() {
    menuOpen = !menuOpen;
    mobileMenu.classList.toggle('open', menuOpen);
    hLine1.classList.toggle('top-open', menuOpen);
    hLine2.classList.toggle('mid-open', menuOpen);
    hLine3.classList.toggle('bot-open', menuOpen);
    hamburgerBtn.setAttribute('aria-label', menuOpen ? 'Close menu' : 'Open menu');
  });
  document.querySelectorAll('.mobile-link').forEach(function(a) {
    a.addEventListener('click', function() {
      menuOpen = false;
      mobileMenu.classList.remove('open');
      hLine1.classList.remove('top-open');
      hLine2.classList.remove('mid-open');
      hLine3.classList.remove('bot-open');
    });
  });

  // CTA button
  document.getElementById('ctaBtn').addEventListener('click', function() {
    window.scrollTo({ top: window.innerHeight * 0.9, behavior: 'smooth' });
  });

  // Feedback form
  var fbForm = document.getElementById('fbForm');
  var fbBtn = document.getElementById('fbBtn');
  var fbSuccess = document.getElementById('fbSuccess');
  fbForm.addEventListener('submit', function(e) {
    e.preventDefault();
    var email = document.getElementById('fbEmail').value.trim();
    if (email) {
      fbBtn.textContent = 'SUBMITTED';
      fbBtn.classList.add('submitted');
      fbBtn.disabled = true;
      fbSuccess.classList.add('show');
    }
  });
})();
</script>
</body>
</html>

## Cinematic Landing Page — Landing Page [sites/cinematic-landing-page]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(66).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cinematic-landing-page.webp

Create a single-page landing website for "Bakery Facilities" — a premium B2B bakery solutions company. The site uses React 18, Vite, TypeScript, Tailwind CSS, GSAP (with ScrollTrigger and SplitText plugins), and Lottie animations. No Framer Motion is used — all animations are GSAP-powered.

---

### Tech Stack & Dependencies

```json
{
  "dependencies": {
    "@gsap/react": "^2.1.2",
    "@tanstack/react-query": "^5.83.0",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "gsap": "^3.14.2",
    "lottie-react": "^2.4.1",
    "lucide-react": "^0.462.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.30.1",
    "sonner": "^1.7.4",
    "tailwind-merge": "^2.6.0",
    "tailwindcss-animate": "^1.0.7"
  },
  "devDependencies": {
    "@vitejs/plugin-react-swc": "^3.11.0",
    "autoprefixer": "^10.4.21",
    "postcss": "^8.5.6",
    "tailwindcss": "^3.4.17",
    "typescript": "^5.8.3",
    "vite": "^5.4.19"
  }
}
```

---

### Fonts (Google Fonts)

Import URL: `https://fonts.googleapis.com/css2?family=Luxurious+Script&family=Manrope:wght@500&family=Open+Sans:wght@300;400;500;600;700&family=Instrument+Serif:ital@0;1&display=swap`

Font families:
- `font-body`: 'Open Sans', sans-serif (body text, nav, buttons)
- `font-accent`: 'Instrument Serif', serif (hero h1, section titles)
- `font-manrope`: 'Manrope', sans-serif (labels, card text)
- `font-luxurious`: 'Luxurious Script', cursive ("for Professionals" subtitle)

---

### Color System (CSS Variables)

```css
:root {
  --background: 0 0% 9%;
  --foreground: 0 0% 100%;
  --primary: 0 0% 100%;
  --primary-foreground: 0 0% 9%;
  --secondary: 0 0% 97%;
  --muted: 0 0% 20%;
  --muted-foreground: 0 0% 75%;
  --accent: 0 0% 15%;
  --border: 0 0% 20%;
  --radius: 2px;
  --hero-cta-bg: 0 0% 97.3%;
  --hero-cta-text: 0 0% 9%;
}
```

Gold accent color: `#CB9D06` (used on hover states for nav, buttons, links)

---

### Tailwind Config Specifics

- Border radius: 2px base
- Container padding: 5%
- Custom keyframes: `marquee` (translateX(0) to translateX(-50%), 60s linear infinite)
- `tailwindcss-animate` plugin

---

### SECTION 1: Hero (Full-Screen Scroll-Driven Video Slider)

**Structure:**
- Outer wrapper: `height: calc(100vh + 300vh)` (3 slides x 150vh per slide transition)
- Inner sticky section: `sticky top-0 w-full h-screen overflow-visible`
- Slides transition via scroll-driven `clip-path: ellipse()` animation

**Video URLs (CloudFront):**
1. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260515_113235_88e0d62e-8103-40c1-948e-f0a4f886ffd1.mp4`
2. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260515_114315_ee3663e6-bd79-41b4-9e5b-0fae62827eb9.mp4`
3. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260515_114559_dca18b14-90f5-47c4-8a84-3cbae9bd8a0c.mp4`

**Video element:** `<video className="w-full h-full object-cover" autoPlay loop muted playsInline />`

**Clip-path transition logic:**
- `SCROLL_PER_SLIDE_VH = 150`
- Easing: cubic ease-in-out `localProgress < 0.5 ? 4*p*p*p : 1 - Math.pow(-2*p + 2, 3) / 2`
- Clip function: `ellipse(${5 + progress * 150}% ${8 + progress * 150}% at 50% 50%)`

**H1 Text:** "THE SMART BAKERY SOLUTION"
- Desktop: `font-accent text-[9.7vw] leading-[1] whitespace-nowrap tracking-[-0.04em]`, positioned `bottom-[-26px]`
- Mobile: `text-[40px] leading-[1.1] whitespace-normal text-center`, positioned `bottom-[48px]` with `px-4`
- Animation: GSAP SplitText per-char, `from: {opacity:0, y:40}`, `to: {opacity:1, y:0}`, duration: 0.8s, delay between chars: 30ms, ease: "power3.out", autoStart: true

**Subtitle:** "for Professionals" (appears after h1 animation completes)
- Font: Luxurious Script cursive
- Desktop: `text-[3vw]`
- Mobile: `text-[12vw]`
- Position: `absolute inset-x-0 top-0`, paddingTop: `calc(80px + 60px)`
- Same GSAP SplitText animation params as h1

---

### SECTION 1 NAVBAR (Fixed, transparent -> black on scroll)

**Scroll behavior:** `scrollY > 50` triggers `bg-black/90 backdrop-blur-[80px] shadow-md py-2`, else `bg-transparent py-4`

**Layout:** `fixed top-0 left-0 right-0 z-20 flex items-center px-4 md:px-10`

**Left:** Region dropdown (Globe icon + "Hong Kong / Macau" text), regions: ["Mainland China", "Hong Kong / Macau", "Taiwan"]

**Center:** Logo (SVG) flanked by nav dropdowns

**Logo SVG (4 leaf-clover shape):**
```svg
<svg viewBox="0 0 305 304" fill="none">
  <path d="M157.135 303.572C157.135 222.53 223.131 156.832 304.174 156.832V303.572H157.135Z" fill="white"/>
  <path d="M147.039 303.572C147.039 222.53 81.0425 156.832 0 156.832V303.572H147.039Z" fill="white"/>
  <path d="M157.135 0C157.135 81.0426 223.131 146.74 304.174 146.74C304.174 65.698 238.178 0 157.135 0Z" fill="white"/>
  <path d="M147.039 0C147.039 81.0426 81.0425 146.74 0 146.74C0 65.698 65.9962 0 147.039 0Z" fill="white"/>
</svg>
```
Logo size: `h-[32px] md:h-[48px]` normal, `h-[24px] md:h-[32px]` scrolled

**Left menu items:** "About Us" (submenu: Our History, Food Service Experts, Creating unforgettable culinary experiences), "Partnering With Us" (submenu: Sourcing from trusted suppliers..., Empowering Customer Operations, Our Experts)

**Right menu items:** "Our Products" (submenu: Viennese Pastry, Bread, Dessert, Savory, Speciality Pastry, Culinary Aid, Ingredient), "Let's Connect!" (submenu: Contact, LinkedIn, WhatsApp, Newsletter, Brochure, Join Us)

**Right:** Language switcher EN/繁, active gets `bg-[#CB9D06] text-white`

**Dropdown styling:** `bg-white shadow-lg py-2`, items `px-4 py-2.5 text-[13px]`, hover: `bg-[#CB9D06] text-white`

**Mobile:** Hamburger icon opens full-screen overlay `bg-black/95 backdrop-blur-md`, accordion-style menu

---

### FLOATING NAV (Right side, desktop only)

Position: `fixed right-4 md:right-6 top-1/2 -translate-y-1/2 z-50 hidden md:flex flex-col gap-4`

3 circular buttons (48px height, rounded-full, bg-black, expand on hover to show labels):
1. Download icon + "Download Brochure"
2. LinkedIn SVG icon + "LinkedIn"
3. MessageCircle icon + "Chat With Us"

LinkedIn SVG paths:
```svg
<svg viewBox="0 0 32 32">
  <path d="M12.6186 9.69215C12.6186 10.6267 11.8085 11.3843 10.8093 11.3843C9.81004 11.3843 9 10.6267 9 9.69215C9 8.7576 9.81004 8 10.8093 8C11.8085 8 12.6186 8.7576 12.6186 9.69215Z" fill="currentColor"/>
  <path d="M9.24742 12.6281H12.3402V22H9.24742V12.6281Z" fill="currentColor"/>
  <path d="M17.3196 12.6281H14.2268V22H17.3196C17.3196 22 17.3196 19.0496 17.3196 17.2049C17.3196 16.0976 17.6977 14.9855 19.2062 14.9855C20.911 14.9855 20.9008 16.4345 20.8928 17.5571C20.8824 19.0244 20.9072 20.5219 20.9072 22H24V17.0537C23.9738 13.8954 23.1508 12.4401 20.4433 12.4401C18.8354 12.4401 17.8387 13.1701 17.3196 13.8305V12.6281Z" fill="currentColor"/>
</svg>
```

Hover: `bg-[#CB9D06]`, label slides in with max-width transition 300ms

---

### SECTION 2: Product Gallery (Masonry Grid)

Container: `bg-white py-8 md:py-16 flex justify-center`, inner: `w-[90%] md:w-[65%]`

**Grid layout:**
- Desktop (>=1000px): 4 columns. Row 1: 4 equal cards. Row 2: 3 cards where middle spans 2 columns
- Mobile: 2 columns
- Aspect ratio: 3:4 (columnWidth * 4/3)
- Row gap: 40px
- Item padding: 4px

**7 items** with labels: Viennese Pastry, Bread, Dessert, Savory, Sweet Treats, Culinary Aid, Ingredient

**Animations:**
- Entry: GSAP ScrollTrigger, start "top 85%", once: true
- Initial: `opacity:0, y: item.y+120, filter: blur(10px)`
- Animate to: `opacity:1, correct position, filter: blur(0px)`, duration: 0.8s, ease: "power3.out", stagger: 0.05s per item
- Hover: CSS `transform: scale(1.2)` on the background image with `transition: transform 6s cubic-bezier(0.22, 0.61, 0.36, 1)`

**Labels:** `text-left text-black text-sm mt-2 font-manrope font-medium`

---

### SECTION 3: About Us (Scroll Reveal Text)

Container: `bg-white py-16 md:py-32 flex flex-col items-center justify-center px-6 md:px-[18%]`

**Title:** "About us" — `font-luxurious text-[32px] text-center text-black mb-[20px]`

**Body text:** "In 1976, Mr Louis Le Duff Opened The First French Casual Food Restaurant..." (full text in code above)
- `font-accent uppercase text-[24px] leading-[36px] md:text-[40px] md:leading-[56px] text-center text-black`

**Scroll Reveal Animation (GSAP ScrollTrigger scrub):**
- Container rotation: from `baseRotation: 3` to `0`, scrub, start "top bottom", end "bottom bottom"
- Word opacity: from `0.1` to `1`, stagger 0.05, scrub, start "top bottom-=20%", end "bottom bottom"
- Word blur: from `blur(4px)` to `blur(0px)`, same trigger

**Button:** "Read more" — `px-8 py-3 bg-black text-white font-manrope text-sm tracking-wide hover:bg-[#CB9D06] transition-colors duration-300`

**Partners Logo Loop (below button, mt-16 md:mt-[140px]):**
- Infinite horizontal scroll marquee, speed: 80px/s, direction: left, gap: 48px, pauseOnHover, fadeOut with white gradient edges (80px wide)
- 12 partner names rendered as: `font-body text-[14px] tracking-[0.2em] uppercase text-black/40 whitespace-nowrap`
- Partners: Bridor de France, Traiteur de Paris, Panidor, Boncolac, Mademoiselle Desserts, Mountry, Pfalzgraf, Dolceria Alba, St Michel, Poppies Bakeries, Alysse Food, Les Delices du Chef

---

### SECTION 4: Partnering With Us

Full-width background image with overlay content.

**Title:** "Partnering With Us" — `font-accent uppercase text-[28px] md:text-[40px] leading-[1.4] text-primary`
- GSAP SplitText chars animation same params as hero

**4 cards** in a grid (`grid-cols-2 md:grid-cols-4 gap-2 md:gap-[8px]`, container `w-[90%] md:w-[64%]`):
- Each card: `bg-black px-4 md:px-6 py-6 md:py-8 flex flex-col items-center text-center gap-3 md:gap-4`
- Lottie animation icon (w-10 h-10 md:w-12 md:h-12, loop)
- Label: `text-primary font-body text-[12px] md:text-[14px] tracking-wide capitalize`
- Cards: "Trusted Sourcing", "Food Safety Standards", "Operational Efficiency", "Expert Support"
- Entry animation: GSAP fromTo `y:80 -> y:0`, duration: 0.7, ease: "power3.out", stagger delay: i*0.15, ScrollTrigger start "top 90%", once

---

### SECTION 5: Footer

`bg-white` full width.

**Top section** (`px-6 md:px-10 lg:px-16 pt-12 md:pt-20 pb-10 md:pb-16`):
- Left: Phone `+852 2407 8840` (text-[13px] text-black/40 uppercase tracking-wider) + email `orders@bakeryfacilities.com` (text-[14px] font-bold, hover gold)
- Right: "Navigate" column (About Us, Partnering With Us, Our Products, Let's Connect!) + "Social" column (WhatsApp, LinkedIn, Newsletter)
- Link styling: `text-[15px] text-black font-medium hover:text-[#CB9D06]`

**Office addresses** (4 offices in a row on desktop, stacked on mobile):
- Head Office (Hong Kong), Mainland China (Shanghai), Taiwan (New Taipei City), Macau
- Each: region title (12px uppercase tracking-wider text-black/40), company name (13px font-semibold), address (12px text-black/60), phone + email with icons

**Bottom bar:** `bg-black px-6 md:px-10 lg:px-16 py-4`
- Left: copyright `text-[12px] text-white/40`
- Right: Privacy Policy + Terms of Service links `text-[12px] text-white/40 hover:text-white`

---

### Key Animation Details (All GSAP, no Framer Motion)

| Element | from | to | duration | ease | trigger |
|---------|------|-----|----------|------|---------|
| Hero H1 chars | `{opacity:0, y:40}` | `{opacity:1, y:0}` | 0.8s | power3.out | autoStart |
| Hero subtitle chars | `{opacity:0, y:40}` | `{opacity:1, y:0}` | 0.8s | power3.out | autoStart (after h1 done) |
| Gallery items | `{opacity:0, y:+120, blur:10px}` | `{opacity:1, y:pos, blur:0}` | 0.8s | power3.out | ScrollTrigger "top 85%" once |
| Section 4 cards | `{opacity:1, y:80}` | `{opacity:1, y:0}` | 0.7s | power3.out | ScrollTrigger "top 90%" once |
| Scroll reveal words | `{opacity:0.1, blur:4px, rotate:3}` | `{opacity:1, blur:0, rotate:0}` | scrub | none | scrub scroll |

**SplitText config:** `type: splitType, smartWrap: true, charsClass: "split-char font-accent"`, char stagger: 30ms (delay/1000)

---

### CSS for split-char (global):
```css
.split-char {
  padding-top: 0 !important;
  padding-bottom: 0 !important;
  line-height: 1 !important;
}
```

---

### Responsive Breakpoints

- Mobile-first
- `md:` = 768px (Tailwind default)
- `lg:` = 1024px
- Gallery columns: 1 (<400px), 2 (400-600px), 2 (600-1000px), 4 (>=1000px)

## ClubX Investors — Landing Page [sites/clubx-hero]

- Preview: https://motionsites.ai/assets/hero-clubx-preview-CpKCe8yV.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/clubx-hero.gif

Create a full-screen hero landing page for "ClubX" — a private tech investor club. The page should have:

Background: A looping, muted, autoplaying video covering the entire viewport using object-cover, sourced from https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260323_071151_38c3924f-c312-48af-a196-3fbb80e4226f.mp4. All content layers above it with z-10.

Fonts: Import Inter (400, 500) and Instrumental Serif via Google Fonts. Body uses Inter.

Navigation: Horizontally centered max-width container (max-w-7xl). Left: a small circular logo icon (10×10 / h-10 w-10). Center (hidden on mobile): links — Home (active/dark), Studio, About, Journal, Reach Us — in text-sm text-gray-600, hover to text-gray-900. Right: a black (bg-gray-900) pill button with white text saying "Begin Journey", px-6 py-2.5 text-sm, with hover:scale-[1.03] and active:scale-[0.97].

Hero content (centered, pt-32 pb-12):

Social proof badge: A pill (rounded-full) with bg-white/20 backdrop-blur-sm border border-gray-900/10. Contains 5 overlapping circular avatars (w-8 h-8 rounded-full, -space-x-2.5) followed by text: "400+ tech investors join the club. Join us!" in text-sm text-gray-800. Margin bottom mb-8.

Headline: "Finance. Freedom. Fellows." in Inter bold, text-5xl sm:text-6xl md:text-[4.9rem], leading-[0.95] tracking-[-1.5px], text-gray-900, max-w-5xl.

Subtext: "Private club of top tech investors." in text-base sm:text-lg text-gray-800, max-w-2xl mt-6 leading-relaxed.

CTA button: "Begin Journey" — black pill, px-12 py-4 text-sm text-white bg-gray-900 mt-9, hover:scale-[1.03], rounded-full.

Stats bar: Absolutely positioned at bottom-8, centered via left-1/2 -translate-x-1/2, max-w-4xl. A rounded-3xl container with bg-white/10 backdrop-blur-sm border border-gray-900/10, px-8 py-6. Four evenly spaced stat columns, each with a large white number (text-3xl sm:text-4xl font-light tracking-tight) and a label (text-white/70 text-sm):

410+ / Tech professionals
€11M / Invested
14 / Deals made
2.5 / Years on the market

Animations: Three staggered fade-rise keyframe animations (translate 24px up + fade in, 0.8s ease-out): badge at 0s delay, headline at 0s, description at 0.2s, button at 0.4s. Define in CSS:

@keyframes fade-rise {
  from { opacity: 0; transform: translateY(24px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-rise { animation: fade-rise 0.8s ease-out both; }
.animate-fade-rise-delay { animation: fade-rise 0.8s ease-out 0.2s both; }
.animate-fade-rise-delay-2 { animation: fade-rise 0.8s ease-out 0.4s both; }

Overall: min-h-screen, overflow-hidden, dark-blue background fallback (--background: 201 100% 13%). All interactive buttons have cursor-pointer and transition-transform. Generate 5 diverse professional avatar images and a circular orange-red logo icon with a white stylized letter.

## CodeNest Coding Platform — Landing Page [sites/codenest-hero]

- Preview: https://motionsites.ai/assets/hero-codenest-preview-Cgppc2qV.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/codenest-hero.gif

Create a high-end, dark-themed hero section for a coding education platform called 'CodeNest' using React and Tailwind CSS. The design must be responsive and follow these precise specifications:

1. Background & Layout:

Background: Implement a full-screen background video using the HLS stream: https://stream.mux.com/tLkHO1qZoaaQOUeVWo8hEBeGQfySP02EPS02BmnNFyXys.m3u8. Use hls.js and set enableWorker: false to ensure stability in sandboxed environments.

Overlays: Set the video to 60% opacity. Add a dark linear gradient from the left (#070b0a to transparent) and a bottom-up gradient for readability.

Grid System: Add three thin vertical grid lines (white/10 opacity) at the 25%, 50%, and 75% marks across the screen (visible on desktop).

Central Glow: Place a large horizontal SVG ellipse glow in the center-top area with a cyan/dark green hue, using a 25px Gaussian blur filter.

2. The Liquid Glass Card:

Component: Create a 200x200px floating card positioned above the main headline, shifted exactly 50px upwards using translate-y-[-50px].

CSS Styling (Liquid Glass):

background: rgba(255, 255, 255, 0.01) with background-blend-mode: luminosity.

backdrop-filter: blur(4px).

box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.1).

Border Effect: A ::before pseudo-element with inset: 0, padding: 1.4px, and a 180-degree white linear gradient. Use -webkit-mask-composite: xor and mask-composite: exclude to create a sharp, high-end border frame.

Content: '[ 2025 ]' tag (14px), 'Taught by Industry Professionals' headline (18px, using Instrument Serif italic for 'Industry'), and a small description (11px).

3. Hero Content & Typography:

Eyebrow: 'Career-Ready Curriculum' in Plus Jakarta Sans, bold, 11px, color #5ed29c.

Main Headline: 'LAUNCH YOUR CODING CAREER.' in Inter Extra Bold, uppercase, tracking-tight. Scale from 40px (mobile) to 72px (desktop). The final period must be green (#5ed29c).

Description: 'Master in-demand coding skills...' in Inter, 14px, 70% white opacity, max-width 512px.

Primary CTA: 'Get Started' button with an ArrowRight icon. Rounded-full, background #5ed29c, text #070b0a, uppercase, bold.

4. Global Navigation:

Header: Sticky/Absolute header with a white minimalist logo.

Desktop Menu: Links for 'PROJECTS', 'BLOG', 'ABOUT', 'RESUME' in Inter, 16px. Hover state: #5ed29c.

Mobile Menu: A functional hamburger menu that toggles a full-screen dark overlay.

5. Required Imports:

Fonts: Inter, Plus Jakarta Sans, and Instrument Serif (italic).

Icons: lucide-react (ArrowRight, Menu, X).

Library: hls.js for video streaming.

## Cosmos Interface — Landing Page [sites/cosmos-interface]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/cosmos%20shtuffArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/cosmos-interface.mp4

<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Cortex — Mind Amplified.</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter+Tight:wght@300;400;500;600;700;900&display=swap');
  </style>
  <script src="https://unpkg.com/react@19/umd/react.production.min.js" crossorigin></script>
  <script src="https://unpkg.com/react-dom@19/umd/react-dom.production.min.js" crossorigin></script>
  <script src="https://unpkg.com/framer-motion@12.42.2/dist/framer-motion.js" crossorigin></script>
  <script src="https://cdn.tailwindcss.com"></script>
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
  <script>
    tailwind.config = {
      theme: {
        extend: {
          fontFamily: { sans: ['Inter Tight', 'sans-serif'] },
          colors: { 'brand-bg': '#122e58' }
        }
      }
    }
  </script>
  <style>
    body {
      background: linear-gradient(180deg, #020715 0%, #051329 35%, #0b264b 65%, #007bb8 88%, #00b8e6 100%) no-repeat;
      background-attachment: fixed;
      color: #ffffff;
      font-family: 'Inter Tight', sans-serif;
      font-weight: 400;
      min-height: 100vh;
      line-height: 1.4;
      overflow-x: hidden;
      -webkit-font-smoothing: antialiased;
      margin: 0;
    }
    ::selection { background: white; color: #122e58; }
    ::-webkit-scrollbar { width: 8px; }
    ::-webkit-scrollbar-track { background: #020715; }
    ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.15); border-radius: 4px; }
    ::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.25); }
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="text/babel" data-type="module">
    const { useRef, useEffect, memo } = React;
    const { motion, AnimatePresence, useScroll, useTransform, useInView } = window["framer-motion"] || FramerMotion;

    // ─── TextEffect Component ───
    const defaultStaggerTimes = { char: 0.03, word: 0.05, line: 0.1 };

    const defaultContainerVariants = {
      hidden: { opacity: 0 },
      visible: { opacity: 1, transition: { staggerChildren: 0.05 } },
      exit: { transition: { staggerChildren: 0.05, staggerDirection: -1 } },
    };

    const defaultItemVariants = {
      hidden: { opacity: 0 },
      visible: { opacity: 1 },
      exit: { opacity: 0 },
    };

    const AnimationComponent = memo(({ segment, variants, per }) => {
      if (per === 'line') {
        return React.createElement(motion.span, { variants, className: 'block' }, segment);
      } else if (per === 'word') {
        return React.createElement(motion.span, { 'aria-hidden': 'true', variants, className: 'inline-block whitespace-pre' }, segment);
      } else {
        return React.createElement(motion.span, { className: 'inline-block whitespace-pre' },
          segment.split('').map((char, i) =>
            React.createElement(motion.span, { key: `char-${i}`, 'aria-hidden': 'true', variants, className: 'inline-block whitespace-pre' }, char)
          )
        );
      }
    });

    function TextEffect({ children, per = 'word', as = 'p', variants: customVariants, className, delay = 0, trigger = true }) {
      let segments;
      if (per === 'line') segments = children.split('\n');
      else if (per === 'word') segments = children.split(/(\s+)/);
      else segments = children.split('');

      const containerVariants = customVariants?.container || defaultContainerVariants;
      const itemVariants = customVariants?.item || defaultItemVariants;
      const stagger = defaultStaggerTimes[per];

      const delayedContainerVariants = {
        hidden: containerVariants.hidden,
        visible: {
          ...containerVariants.visible,
          transition: {
            ...(containerVariants.visible?.transition || {}),
            staggerChildren: containerVariants.visible?.transition?.staggerChildren || stagger,
            delayChildren: delay,
          },
        },
        exit: containerVariants.exit,
      };

      const MotionTag = motion[as] || motion.p;

      return React.createElement(AnimatePresence, null,
        trigger && React.createElement(MotionTag, {
          initial: 'hidden',
          animate: 'visible',
          exit: 'exit',
          variants: delayedContainerVariants,
          className: `whitespace-pre-wrap ${className || ''}`,
        },
          segments.map((segment, index) =>
            React.createElement(AnimationComponent, { key: `${per}-${index}-${segment}`, segment, variants: itemVariants, per })
          )
        )
      );
    }

    // ─── SVG Arrow Icon ───
    function ArrowUpRight({ className }) {
      return React.createElement('svg', {
        xmlns: 'http://www.w3.org/2000/svg',
        width: 24, height: 24,
        viewBox: '0 0 24 24',
        fill: 'none',
        stroke: 'currentColor',
        strokeLinecap: 'round',
        strokeLinejoin: 'round',
        className
      },
        React.createElement('path', { d: 'M7 7h10v10' }),
        React.createElement('path', { d: 'M7 17 17 7' })
      );
    }

    // ─── Animation Variants ───
    const blurSlideVariants = {
      container: {
        hidden: { opacity: 0 },
        visible: { opacity: 1, transition: { staggerChildren: 0.015 } },
        exit: { opacity: 0, transition: { staggerChildren: 0.01, staggerDirection: -1 } },
      },
      item: {
        hidden: { opacity: 0, filter: 'blur(10px) brightness(0%)', y: 20 },
        visible: { opacity: 1, y: 0, filter: 'blur(0px) brightness(100%)', transition: { duration: 0.4, ease: [0.16, 1, 0.3, 1] } },
        exit: { opacity: 0, y: -20, filter: 'blur(10px) brightness(0%)', transition: { duration: 0.3, ease: [0.16, 1, 0.3, 1] } },
      },
    };

    const otherElementVariants = {
      hidden: { opacity: 0, y: 35 },
      visible: { opacity: 1, y: 0, transition: { duration: 0.9, ease: [0.16, 1, 0.3, 1] } },
      exit: { opacity: 0, y: -25, transition: { duration: 0.7, ease: [0.16, 1, 0.3, 1] } },
    };

    // ─── Main App ───
    function App() {
      const scrollContainerRef = useRef(null);
      const videoRef = useRef(null);
      const videoRef2 = useRef(null);
      const heroRef = useRef(null);
      const aboutRef = useRef(null);
      const solutionsRef = useRef(null);

      const inViewHero = useInView(heroRef, { amount: 0.15, once: false });
      const inViewAbout = useInView(aboutRef, { amount: 0.15, once: false });
      const inViewSolutions = useInView(solutionsRef, { amount: 0.1, once: false });

      const { scrollYProgress: videoScrollProgress } = useScroll({
        target: scrollContainerRef,
        offset: ["start start", "end start"]
      });
      const videoOpacity = useTransform(videoScrollProgress, [0.9, 1.0], [1, 0]);

      // Sync scroll position with hero video
      useEffect(() => {
        const video = videoRef.current;
        const container = scrollContainerRef.current;
        if (!video || !container) return;

        let targetProgress = 0;
        let currentProgress = 0;
        let animationFrameId;

        const handleScroll = () => {
          const rect = container.getBoundingClientRect();
          const scrollHeight = container.scrollHeight;
          if (scrollHeight <= 0) return;
          const scrolled = -rect.top;
          targetProgress = Math.max(0, Math.min(1, scrolled / scrollHeight));
        };

        const updateVideoProgress = () => {
          currentProgress += (targetProgress - currentProgress) * 0.08;
          if (Math.abs(targetProgress - currentProgress) < 0.0001) currentProgress = targetProgress;
          const duration = video.duration;
          if (duration && !isNaN(duration)) {
            const targetTime = currentProgress * duration * 0.7;
            if (!video.seeking && Math.abs(video.currentTime - targetTime) > 0.02) {
              video.currentTime = targetTime;
            }
          }
          animationFrameId = requestAnimationFrame(updateVideoProgress);
        };

        handleScroll();
        currentProgress = targetProgress;
        window.addEventListener('scroll', handleScroll, { passive: true });
        animationFrameId = requestAnimationFrame(updateVideoProgress);

        const handleLoadedMetadata = () => { handleScroll(); currentProgress = targetProgress; };
        video.addEventListener('loadedmetadata', handleLoadedMetadata);

        return () => {
          cancelAnimationFrame(animationFrameId);
          window.removeEventListener('scroll', handleScroll);
          video.removeEventListener('loadedmetadata', handleLoadedMetadata);
        };
      }, []);

      // Sync scroll position with solutions video
      useEffect(() => {
        const video = videoRef2.current;
        const container = solutionsRef.current;
        if (!video || !container) return;

        let targetProgress = 0;
        let currentProgress = 0;
        let animationFrameId;

        const handleScroll = () => {
          const rect = container.getBoundingClientRect();
          const scrollableHeight = container.scrollHeight - window.innerHeight;
          if (scrollableHeight <= 0) return;
          const scrolled = -rect.top;
          targetProgress = Math.max(0, Math.min(1, scrolled / scrollableHeight));
        };

        const updateVideoProgress = () => {
          currentProgress += (targetProgress - currentProgress) * 0.08;
          if (Math.abs(targetProgress - currentProgress) < 0.0001) currentProgress = targetProgress;
          const duration = video.duration;
          if (duration && !isNaN(duration)) {
            const targetTime = currentProgress * duration;
            if (!video.seeking && Math.abs(video.currentTime - targetTime) > 0.02) {
              video.currentTime = targetTime;
            }
          }
          animationFrameId = requestAnimationFrame(updateVideoProgress);
        };

        handleScroll();
        currentProgress = targetProgress;
        window.addEventListener('scroll', handleScroll, { passive: true });
        animationFrameId = requestAnimationFrame(updateVideoProgress);

        const handleLoadedMetadata = () => { handleScroll(); currentProgress = targetProgress; };
        video.addEventListener('loadedmetadata', handleLoadedMetadata);

        return () => {
          cancelAnimationFrame(animationFrameId);
          window.removeEventListener('scroll', handleScroll);
          video.removeEventListener('loadedmetadata', handleLoadedMetadata);
        };
      }, []);

      const { scrollYProgress } = useScroll({ target: solutionsRef, offset: ["start start", "end end"] });
      const { scrollYProgress: heroScroll } = useScroll({ target: heroRef, offset: ["start start", "end start"] });

      const heroTitleOpacity = useTransform(heroScroll, [0, 0.45], [1, 0]);
      const heroTitleBlur = useTransform(heroScroll, [0, 0.45], ["blur(0px)", "blur(20px)"]);
      const heroTitleY = useTransform(heroScroll, [0, 0.45], [0, -60]);
      const heroOtherOpacity = useTransform(heroScroll, [0, 0.45], [1, 0]);
      const heroOtherY = useTransform(heroScroll, [0, 0.45], [0, -40]);

      const { scrollYProgress: aboutScroll } = useScroll({ target: aboutRef, offset: ["start end", "end start"] });
      const aboutTitleOpacity = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], [0, 1, 1, 0]);
      const aboutTitleBlur = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], ["blur(20px)", "blur(0px)", "blur(0px)", "blur(20px)"]);
      const aboutTitleY = useTransform(aboutScroll, [0.1, 0.35, 0.65, 0.9], [60, 0, 0, -60]);
      const aboutOtherOpacity = useTransform(aboutScroll, [0.15, 0.35, 0.65, 0.85], [0, 1, 1, 0]);
      const aboutOtherY = useTransform(aboutScroll, [0.15, 0.35, 0.65, 0.85], [50, 0, 0, -50]);

      const opacitySet1 = useTransform(scrollYProgress, [0, 0.05, 0.22, 0.29], [0, 1, 1, 0]);
      const blurSet1 = useTransform(scrollYProgress, [0, 0.05, 0.22, 0.29], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet1 = useTransform(scrollYProgress, [0, 0.29], ["0px", "-120px"]);
      const yBottomSet1 = useTransform(scrollYProgress, [0, 0.29], ["0px", "120px"]);

      const opacitySet2 = useTransform(scrollYProgress, [0.33, 0.40, 0.58, 0.65], [0, 1, 1, 0]);
      const blurSet2 = useTransform(scrollYProgress, [0.33, 0.40, 0.58, 0.65], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet2 = useTransform(scrollYProgress, [0.33, 0.65], ["0px", "-120px"]);
      const yBottomSet2 = useTransform(scrollYProgress, [0.33, 0.65], ["0px", "120px"]);

      const opacitySet3 = useTransform(scrollYProgress, [0.69, 0.76, 0.92, 0.99], [0, 1, 1, 0]);
      const blurSet3 = useTransform(scrollYProgress, [0.69, 0.76, 0.92, 0.99], ["blur(15px)", "blur(0px)", "blur(0px)", "blur(15px)"]);
      const yTopSet3 = useTransform(scrollYProgress, [0.69, 0.99], ["0px", "-120px"]);
      const yBottomSet3 = useTransform(scrollYProgress, [0.69, 0.99], ["0px", "120px"]);

      return React.createElement('div', { className: 'relative w-full min-h-screen' },

        // ─── Header ───
        React.createElement('header', { className: 'fixed top-4 lg:top-5 left-1/2 -translate-x-1/2 z-50 w-[calc(100%-32px)] md:w-auto bg-slate-950/55 backdrop-blur-xl rounded-xl p-1 pl-1 pr-5 flex items-center justify-between md:gap-8 transition-all' },
          React.createElement('div', { className: 'flex items-center justify-center w-10 h-10 bg-white/10 hover:bg-white/15 rounded-lg text-white text-xl select-none leading-none cursor-pointer transition-all duration-300 hover:rotate-45 active:scale-95 shrink-0' }, '\u2733'),
          React.createElement('nav', { className: 'flex items-center gap-4 lg:gap-5' },
            React.createElement('a', { href: '#cortex', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Cortex'),
            React.createElement('a', { href: '#solutions', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Interface'),
            React.createElement('a', { href: '#developer', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Developer'),
            React.createElement('a', { href: '#support', className: 'text-white/75 hover:text-white text-xs lg:text-[13.5px] font-medium tracking-tight whitespace-nowrap transition-colors' }, 'Support'),
          )
        ),

        // ─── Background Video ───
        React.createElement(motion.div, { style: { opacity: videoOpacity }, className: 'fixed inset-0 w-full h-full z-0 select-none pointer-events-none overflow-hidden' },
          React.createElement('video', {
            ref: videoRef,
            src: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260701_091244_186b0374-b961-4059-b31d-84b819807185.mp4',
            className: 'w-full h-full object-cover',
            muted: true,
            playsInline: true,
            preload: 'auto'
          })
        ),

        // ─── Scroll Container (Hero + About) ───
        React.createElement('div', { ref: scrollContainerRef, className: 'relative z-10 w-full bg-transparent' },

          // ─── Hero Section ───
          React.createElement('section', { ref: heroRef, className: 'relative w-full h-screen flex items-center overflow-hidden bg-transparent' },
            React.createElement('main', { className: 'relative z-10 w-full max-w-none mx-auto h-screen px-4 lg:px-[56px] pt-28 lg:pt-0 grid grid-cols-1 lg:grid-cols-12 gap-12 lg:gap-8 items-center' },
              // Left Column
              React.createElement('div', { className: 'lg:col-span-7 flex flex-col justify-center h-full lg:-translate-y-[112px] transform' },
                React.createElement(motion.div, { style: { opacity: heroTitleOpacity, filter: heroTitleBlur, y: heroTitleY } },
                  React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight mb-10 text-white flex flex-col' },
                    React.createElement('span', { className: 'block' },
                      React.createElement(TextEffect, { per: 'char', variants: blurSlideVariants, trigger: inViewHero }, 'Mind')
                    ),
                    React.createElement('span', { className: 'block' },
                      React.createElement(TextEffect, { per: 'char', variants: blurSlideVariants, trigger: inViewHero, delay: 0.15 }, 'Amplified.')
                    )
                  )
                ),
                React.createElement(motion.div, { style: { opacity: heroOtherOpacity, y: heroOtherY } },
                  React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewHero ? 'visible' : 'exit' },
                    React.createElement('a', { href: '#discover', className: 'group inline-flex items-center justify-center bg-white hover:bg-white/90 text-brand-bg rounded-full px-7 py-3.5 text-sm font-normal w-fit gap-3 shadow-none transition-all' },
                      React.createElement('span', { className: 'flex items-center justify-center w-5 h-5 rounded-full bg-brand-bg text-white transition-transform group-hover:scale-105' },
                        React.createElement(ArrowUpRight, { className: 'w-3.5 h-3.5 stroke-[2.5]' })
                      ),
                      React.createElement('span', { className: 'tracking-tight' }, 'Discover Cortex')
                    )
                  )
                )
              ),
              // Right Column
              React.createElement(motion.div, { style: { opacity: heroOtherOpacity, y: heroOtherY }, className: 'lg:col-span-4 lg:col-start-9 flex flex-col justify-center lg:self-end lg:mb-[56px] lg:justify-self-end w-full max-w-[328px]' },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewHero ? 'visible' : 'exit' },
                  React.createElement('div', { className: 'text-[11.5px] font-normal uppercase text-white/50 tracking-[0.15em] mb-3' }, '001 \u2014 Concept'),
                  React.createElement('p', { className: 'text-[14.5px] font-normal leading-relaxed text-white tracking-tight' }, 'A screen is a bottleneck. Cortex is a premium neural interface that streams your intention directly to AI, amplifying your natural mind.')
                )
              )
            )
          ),

          // ─── About Section ───
          React.createElement('section', { ref: aboutRef, className: 'w-full max-w-none mx-auto px-4 lg:px-[56px] h-screen min-h-[600px] py-[56px] flex flex-col justify-between items-start bg-transparent' },
            // Top
            React.createElement('div', { className: 'w-full flex flex-col gap-6' },
              React.createElement(motion.div, { style: { opacity: aboutOtherOpacity, y: aboutOtherY } },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewAbout ? 'visible' : 'exit' },
                  React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '002 \u2014 Neural Extension')
                )
              ),
              React.createElement('div', { className: 'w-full' },
                React.createElement(motion.div, { style: { opacity: aboutTitleOpacity, filter: aboutTitleBlur, y: aboutTitleY } },
                  React.createElement(TextEffect, { per: 'word', as: 'p', variants: blurSlideVariants, trigger: inViewAbout, className: 'text-[clamp(24px,3.2vw,40px)] font-medium leading-[1.25] tracking-tight text-white max-w-[1200px]' },
                    '\u2460 Cortex is a premium, circular neural interface that rests seamlessly on your temple, establishing a real-time thought connection that augments your cognition with advanced AI models.'
                  )
                )
              )
            ),
            // Bottom
            React.createElement('div', { className: 'grid grid-cols-1 lg:grid-cols-12 w-full gap-8' },
              React.createElement(motion.div, { style: { opacity: aboutOtherOpacity, y: aboutOtherY }, className: 'lg:col-start-1 lg:col-span-4 flex flex-col w-full max-w-[328px]' },
                React.createElement(motion.div, { variants: otherElementVariants, initial: 'hidden', animate: inViewAbout ? 'visible' : 'exit', className: 'w-full' },
                  React.createElement('div', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em] mb-5' }, 'Capabilities:'),
                  React.createElement('div', { className: 'flex flex-col w-full border-b border-white/15' },
                    ['Instant Knowledge Retrieval', 'Seamless Thought Translation', 'Generative Reasoning Flow'].map((item) =>
                      React.createElement('a', { key: item, href: '#', className: 'group flex justify-between items-center py-4 border-t border-white/15 text-white transition-opacity' },
                        React.createElement('span', { className: 'text-[14.5px] font-medium tracking-tight' }, item),
                        React.createElement('span', { className: 'flex items-center justify-center w-5 h-5 rounded-full bg-white text-brand-bg transition-transform group-hover:scale-110 ml-3 shrink-0' },
                          React.createElement(ArrowUpRight, { className: 'w-3.5 h-3.5 stroke-[2.5]' })
                        )
                      )
                    )
                  )
                )
              )
            )
          )
        ),

        // ─── Solutions Section ───
        React.createElement('section', { id: 'solutions', ref: solutionsRef, className: 'w-full min-h-[350vh] bg-transparent relative' },
          React.createElement('div', { className: 'w-full h-screen sticky top-0 overflow-hidden flex flex-col justify-between' },
            // BG Video
            React.createElement('div', { className: 'absolute inset-0 w-full h-full select-none pointer-events-none z-0' },
              React.createElement('video', {
                ref: videoRef2,
                src: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260707_040817_939f16e8-836c-4249-aa1d-63f3e2978a89.mp4',
                className: 'w-full h-full object-cover',
                muted: true,
                playsInline: true,
                preload: 'auto'
              })
            ),
            // Content
            React.createElement('div', { className: 'relative z-10 w-full max-w-none mx-auto h-full px-4 lg:px-[56px] flex flex-col justify-center items-start' },
              React.createElement('div', { className: 'w-full max-w-[1000px] h-[320px] lg:h-[400px] relative flex items-center justify-start' },

                // Set 1
                React.createElement(motion.div, { style: { opacity: opacitySet1, filter: blurSet1 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet1 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '003 \u2014 Interface'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Silent thought.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet1 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Cortex.')
                  )
                ),

                // Set 2
                React.createElement(motion.div, { style: { opacity: opacitySet2, filter: blurSet2 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet2 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '004 \u2014 Performance'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Cognitive flow.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet2 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Intuition.')
                  )
                ),

                // Set 3
                React.createElement(motion.div, { style: { opacity: opacitySet3, filter: blurSet3 }, className: 'absolute inset-0 flex flex-col gap-[40px] justify-center pointer-events-none' },
                  React.createElement(motion.div, { style: { y: yTopSet3 }, className: 'w-full flex flex-col gap-6' },
                    React.createElement('span', { className: 'text-[11.5px] font-medium uppercase text-white/50 tracking-[0.15em]' }, '005 \u2014 Symbiosis'),
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Instant recall.')
                  ),
                  React.createElement(motion.div, { style: { y: yBottomSet3 }, className: 'w-full' },
                    React.createElement('h1', { className: 'text-[clamp(40px,6.5vw,105px)] font-normal leading-[0.95] tracking-tight text-white w-full' }, 'Insight.')
                  )
                )
              )
            )
          )
        )
      );
    }

    const root = ReactDOM.createRoot(document.getElementById('root'));
    root.render(React.createElement(App));
  </script>
</body>
</html>

## Creative Agency — Landing Page [sites/creative-agency]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(50).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/creative-agency.webp

**Prompt:**

Build a full-viewport hero section in React + TypeScript with Tailwind CSS. Use `lucide-react` for the `Instagram` and `Send` icons. The section must match these specs exactly.

**Fonts (load in index.css):**
```css
@import url('https://db.onlinewebfonts.com/c/38c9851a552c219fba7878035cef1a1c?family=Britanica-Black');
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
@tailwind base;
@tailwind components;
@tailwind utilities;

* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: 'Inter', sans-serif; overflow-x: hidden; }
```
Also use `Geist, sans-serif` for the giant headline (fallback to sans-serif if Geist isn't loaded).

**Background video asset (verbatim URL):**
```
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_213626_db1bde2b-521c-4b22-91f3-35c072eb8771.mp4
```

**Two inline SVG logo components (verbatim path data):**

`LogoWhite` — width 30, height 22, viewBox `0 0 30 22`, fill none:
- `<path d="M2 4C2 4 8 1 15 6C22 11 28 8 28 8" stroke="white" strokeWidth="3.5" strokeLinecap="round"/>`
- `<path d="M2 16C2 16 8 13 15 18C22 23 28 20 28 20" stroke="white" strokeWidth="3.5" strokeLinecap="round"/>`

`LogoRed` — width 26, height 20, viewBox `0 0 30 22`, fill none, same two paths but `stroke="#e02b10"`.

**Structure — one root `<div>`** with classes `relative w-full min-h-screen overflow-hidden` and inline style `backgroundColor: '#e02b10'`. It contains, in order:

1. **Background `<video>`** — classes `absolute inset-0 w-full h-full object-cover`, attributes `autoPlay muted loop playsInline`, `src` equal to the URL above.

2. **Navbar** `<nav>` — classes `absolute top-0 left-0 right-0 z-30 flex items-center justify-between px-4 sm:px-6 md:px-10 py-4 md:py-5 gap-2`.
   - Left cluster: `flex items-center gap-3 sm:gap-6` containing `<LogoWhite />` and a hidden-on-mobile group `hidden sm:flex items-center gap-2` with:
     - `HOME` button: `bg-white text-black text-xs px-5 py-2 rounded-full`, inline style `fontFamily: 'Inter, sans-serif', fontWeight: 700`.
     - `RITUALS` and `RATES` buttons mapped from array: `text-white text-xs px-5 py-2 rounded-full border border-white/60 hover:border-white transition-all`, inline style `fontFamily: 'Inter, sans-serif', fontWeight: 400`.
   - Right cluster: `flex items-center gap-3 sm:gap-4`:
     - Instagram icon button: `text-white hover:opacity-70 transition-opacity`, icon `<Instagram size={18} strokeWidth={1.5} />`.
     - Send icon button: same classes plus `hidden sm:block`, icon `<Send size={16} strokeWidth={1.5} />`.
     - `Reservations` button: `border border-white text-white text-xs px-3 sm:px-5 py-2 rounded-full hover:bg-white hover:text-red-600 transition-all whitespace-nowrap`, inline style `fontFamily: 'Inter, sans-serif'`.

3. **Content wrapper** `<div>` with `relative z-20 min-h-screen flex flex-col px-4 sm:px-6 md:px-10`, containing three rows:

   **Row 1 (navbar spacer):** `<div className="h-[72px] shrink-0" />`

   **Row 2 (middle):** `<div>` with `flex-1 flex flex-col md:flex-row md:items-center md:justify-between mx-auto gap-10 md:gap-32 py-8 md:py-0`, inline style `maxWidth: '1100px', width: '100%'`.
   - **Left block** `max-w-[260px]`:
     - `<p>` classes `text-white text-[13px] tracking-[0.22em] uppercase leading-snug mb-2`, style `fontFamily: 'Inter, sans-serif', fontWeight: 700`, text `MARKETING<br />COLLECTIVE`.
     - `<p>` classes `text-white text-[13px] leading-relaxed`, style `fontFamily: 'Inter, sans-serif', opacity: 0.8`, text `Creative growth blueprints<br />for bold brands in Web3 era`.
   - **Right block** `max-w-[260px] text-left`:
     - Row `flex justify-start mb-2` containing `<LogoRed />`.
     - `<p>` classes `text-white text-[14px] leading-relaxed mb-3`, style `fontFamily: 'Inter, sans-serif'`, text: `MetricX is the essential growth dashboard for bold agencies. Monitor reach, refine spend, steer campaigns, surface insights, delight your clients every day.`
     - `<p>` classes `text-white text-[13px] leading-loose`, style `fontFamily: 'Inter, sans-serif', opacity: 0.7`, text `Audiences Dashboards Spend<br />Performance Channels Growth`.

   **Row 3 (bottom):** `<div>` with `pb-8 flex flex-col lg:flex-row lg:items-end lg:justify-between gap-8 lg:gap-6 shrink-0`.
   - **Column 1** `flex-1 min-w-0`:
     - `<h1>` classes `text-white select-none mb-6 md:mb-10`, inline style `fontFamily: 'Geist, sans-serif', fontWeight: 600, fontSize: 'clamp(56px, 13vw, 155px)', letterSpacing: '-0.04em', lineHeight: 0.78, width: 'fit-content'`, text `creative<br />studio`.
     - Sub-row `flex flex-col sm:flex-row sm:items-center gap-4 sm:gap-6`:
       - `<p>` classes `text-white text-[14px] leading-relaxed`, style `fontFamily: 'Inter, sans-serif', minWidth: '160px'`, text `Sharp ideas only. We craft<br />brands that own Web3.`
       - `<button>` classes `bg-white text-black rounded-full hover:bg-gray-100 active:scale-95 transition-all shadow-lg w-full sm:w-auto`, inline style `fontFamily: 'Inter, sans-serif', fontWeight: 600, fontSize: '15px', whiteSpace: 'nowrap', padding: '24px 60px'`, text `begin now`.
   - **Column 2 (stat cards)** `flex gap-4 sm:gap-6`, mapping `[['80%', 'Reach uplift'], ['92%', 'Client loyalty']]`:
     - Card `<div>` classes `rounded-2xl px-5 sm:px-6 py-5 flex flex-col items-start justify-between text-left flex-1 lg:flex-initial`, inline style `minWidth: '150px', minHeight: '150px', background: 'rgba(255,255,255,0.92)', backdropFilter: 'blur(10px)'`.
       - Big number `<p>` classes `leading-none`, style `fontFamily: 'Britanica-Black, sans-serif', fontSize: 'clamp(2rem, 6vw, 2.6rem)', color: '#111'`.
       - Label `<p>` classes `text-[12px] mt-auto`, style `fontFamily: 'Inter, sans-serif', color: '#888'`.

**Animations / interactions:**
- Background video auto-loops muted.
- Nav link borders animate via `transition-all` on hover from `border-white/60` to `border-white`.
- Icon buttons fade to `opacity-70` on hover via `transition-opacity`.
- Reservations button swaps to white bg / red-600 text on hover via `transition-all`.
- `begin now` button has `hover:bg-gray-100` and `active:scale-95` via `transition-all`.
- Stat cards use `backdrop-filter: blur(10px)` over the red/video backdrop.

No other animations, keyframes, or JS state. No Supabase needed for this visual-only section.

## Digital Experiences — Landing Page [sites/digital-experiences]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(40).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/digital-experiences.webp

### Prompt

Build a single-page React + Vite + TypeScript + Tailwind CSS site with exactly two full-screen sections (Hero and Capabilities). The page is a dark, cinematic web design agency landing page with "liquid glass" morphism UI elements and smooth blur/fade animations using Framer Motion.

---

### Fonts (Google Fonts)

Load via `<link>` in `index.html`:
- **Instrument Serif** (italic) -- used for all headings (`font-heading`)
- **Barlow** (weights 300, 400, 500, 600) -- used for body text (`font-body`)

Tailwind config extends `fontFamily`:
```js
heading: ["'Instrument Serif'", 'serif'],
body: ["'Barlow'", 'sans-serif'],
```

Base CSS: `html, body { background: #000; color: #fff; font-family: 'Barlow', sans-serif; }`

---

### Liquid Glass CSS (in index.css)

Two variants defined as plain CSS classes:

**`.liquid-glass`** (subtle):
- `background: rgba(255, 255, 255, 0.01)` with `background-blend-mode: luminosity`
- `backdrop-filter: blur(4px)` / `-webkit-backdrop-filter: blur(4px)`
- No border; `box-shadow: inset 0 1px 1px rgba(255,255,255,0.1)`
- `position: relative; overflow: hidden`
- `::before` pseudo-element creates a gradient stroke border:
- `position: absolute; inset: 0; border-radius: inherit; padding: 1.4px`
- `background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)`
- Masked with `-webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0); -webkit-mask-composite: xor; mask-composite: exclude;`
- `pointer-events: none`

**`.liquid-glass-strong`** (bolder):
- Same structure but `backdrop-filter: blur(50px)`
- `box-shadow: 4px 4px 4px rgba(0,0,0,0.05), inset 0 1px 1px rgba(255,255,255,0.15)`
- `::before` gradient uses 0.5 alpha at edges, 0.2 at 20%/80%

---

### FadingVideo Component

A reusable `<video>` component accepting `src` (string or string[]), `className`, and `style`. It:
1. Starts with `opacity: 0`
2. On `loadeddata`, fades in over 500ms using `requestAnimationFrame`
3. On `timeupdate`, when remaining time <= 0.55s, fades out over 550ms
4. On `ended`, if single source: resets `currentTime` to 0, replays, fades back in. If array: advances to next index (cycling).
5. Video is `autoPlay`, `muted`, `playsInline`, `preload="auto"`

---

### BlurText Component

A word-by-word staggered blur-in animation component using Framer Motion:
- Splits `text` prop by spaces
- Each word is a `motion.span` with `display: inline-block`, `marginRight: 0.28em`
- Triggers on IntersectionObserver (threshold 0.1)
- Each word animates: `filter` from `blur(10px)` to `blur(0px)`, `opacity` 0 to 1, `y` from 50 to 0
- Duration 0.7s per word, stagger delay of 100ms per word index
- Container uses `display: flex; flexWrap: wrap; justifyContent: center; rowGap: 0.1em`

---

### Section 1: Hero

- Full viewport height (`h-screen`), `overflow-hidden`, `bg-black`
- **Background video**: Single `<FadingVideo>` with:
- `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260619_191346_9d19d66e-86a4-47f7-8dc6-712c1788c3b2.mp4"`
- Positioned: `absolute left-1/2 top-0 -translate-x-1/2 object-cover object-top z-0`
- Inline style: `width: 120%; height: 120%`

- **Content** (`relative z-10, flex flex-col h-full`):

**Navbar** (fixed, `top-4 left-0 right-0 z-50`, flex between, `px-8 lg:px-16`):
- Left: `liquid-glass` circle (h-12 w-12 rounded-full) with italic "a" in `font-heading text-2xl`
- Center (hidden on mobile, `md:flex`): `liquid-glass rounded-full px-1.5 py-1.5` pill containing links ["Work", "Studio", "Services", "Journal", "Contact"] as `px-3 py-2 text-sm font-medium text-white/90 font-body` + a white CTA button "Start a Project" with ArrowUpRight icon
- Right: empty `h-12 w-12` spacer div

**Main content** (centered, `flex-1 flex flex-col items-center justify-center pt-24 px-4 text-center`):
- **Badge** (motion.div, delay 0.4): `liquid-glass rounded-full` pill with a white "New" badge inside + text "Booking Q3 2026 engagements -- limited capacity"
- **Headline** (mt-6, max-w-3xl): `<BlurText>` with text "Crafted Digital Experiences Built to Outlast Trends", classes: `text-6xl md:text-7xl lg:text-[5.5rem] font-heading italic text-white leading-[0.8] tracking-[-4px]`
- **Subtext** (motion.p, delay 0.8, mt-4): "We are a small studio of designers and engineers shaping brand-defining websites for ambitious companies. Precise typography, cinematic motion, and code you can be proud of." -- `text-sm md:text-base text-white max-w-2xl font-body font-light leading-tight`
- **CTA buttons** (motion.div, delay 1.1, mt-6, flex gap-6): "Start a Project" in `liquid-glass-strong rounded-full px-5 py-2.5` with ArrowUpRight + "Watch Showreel" plain text with Play icon
- **Stats cards** (motion.div, delay 1.3, mt-8, flex gap-4): Two `liquid-glass p-5 w-[220px] rounded-[1.25rem]` cards:
- Card 1: ClockIcon, "6 Weeks", "Average End-to-End Launch Time"
- Card 2: GlobeIcon, "140+", "Brands Shipped Across Four Continents"
- Numbers: `text-4xl font-heading italic tracking-[-1px] leading-none mt-4`

**Bottom trust bar** (motion.div, delay 1.4, flex-col items-center gap-4 pb-8):
- `liquid-glass rounded-full` pill: "Trusted by founders, operators, and creative directors worldwide"
- Logo names in a flex row (gap-12 md:gap-16): ["Aeon", "Vela", "Apex", "Orbit", "Zeno"] each as `font-heading italic text-2xl md:text-3xl tracking-tight`

- **All motion elements** use shared initial/animate: `{ filter: 'blur(10px)', opacity: 0, y: 20 }` -> `{ filter: 'blur(0px)', opacity: 1, y: 0 }`, duration 0.8s, easeOut

---

### Section 2: Capabilities

- `min-h-screen`, `overflow-hidden`, `bg-black`, relative
- **Background video**: `<FadingVideo>` with:
- `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_093722_ccfc7ebf-182f-419f-8a62-2dc02db7dd9d.mp4"`
- `absolute inset-0 w-full h-full object-cover z-0`

- **Content** (`relative z-10 px-8 md:px-16 lg:px-20 pt-24 pb-10 flex flex-col min-h-screen`):
- **Header** (mb-auto):
- Label: `text-sm font-body text-white/80 mb-6` -- "// Capabilities"
- Heading: `font-heading italic text-6xl md:text-7xl lg:text-[6rem] leading-[0.9] tracking-[-3px]` -- "Studio craft,\nend to end"

- **Cards grid** (mt-16, `grid grid-cols-1 md:grid-cols-3 gap-6`), three cards:
1. **Design** -- Icon: ImageIcon (filled image icon), Tags: ["Brand Systems", "Art Direction", "Visual Identity", "Motion"], Body: "We shape identities and interfaces that feel unmistakably yours -- typographic systems, component libraries, and art-directed pages that scale without losing soul."
2. **Engineering** -- Icon: MovieIcon (film/clapboard), Tags: ["React", "Next.js", "Headless CMS", "Edge-Ready"], Body: "Production-grade front-ends built on modern stacks. Performant, accessible, and instrumented -- with code your team will enjoy extending long after launch."
3. **Growth** -- Icon: LightbulbIcon, Tags: ["SEO", "Analytics", "A/B Testing", "Retention"], Body: "Launch is the starting line. We partner with your team on conversion, content, and iteration loops that turn a beautiful site into a compounding asset."

- Each card: `liquid-glass rounded-[1.25rem] p-6 min-h-[360px] flex flex-col`
- Top row: icon in a nested `liquid-glass h-11 w-11 rounded-[0.75rem]` square + tags (flex-wrap, gap-1.5) right-aligned, each tag is `liquid-glass rounded-full px-3 py-1 text-[11px] text-white/90 font-body whitespace-nowrap`
- Spacer: `flex-1`
- Bottom: title in `font-heading italic text-3xl md:text-4xl tracking-[-1px] leading-none` + body in `text-sm text-white/90 font-body font-light leading-snug max-w-[32ch]`

---

### Custom SVG Icons (no external icon library needed for these)

- **ArrowUpRight**: 24x24, stroke, paths "M7 17L17 7" and "M7 7h10v10"
- **Play**: 24x24, filled polygon "6 4 20 12 6 20 6 4"
- **ClockIcon**: 24x24, stroke (1.5), circle r=9 + "M12 7v5l3 2"
- **GlobeIcon**: 24x24, stroke (1.5), circle r=9 + horizontal line + two arc paths
- **ImageIcon**: 24x24, filled Material-style image icon
- **MovieIcon**: 24x24, filled Material-style movie icon
- **LightbulbIcon**: 24x24, filled Material-style bulb icon

---

### Dependencies

- react, react-dom
- framer-motion
- tailwindcss, postcss, autoprefixer
- vite, @vitejs/plugin-react
- typescript

---

### Key Design Principles

- Everything is on a pure black (#000) background
- All text is white; subtle text uses `white/80` or `white/90`
- Liquid glass elements have near-invisible fills with gradient-stroke borders via CSS masks
- Videos cover sections as atmospheric backgrounds, fading in/out smoothly
- Typography: heading font is always italic with very tight tracking (negative), body font is light weight
- Responsive: nav links hidden on mobile, grid collapses to single column, text sizes scale with breakpoints
- Animations: staggered blur-in on load for hero content, intersection-triggered for BlurText

## Dreamcore Landing — Landing Page [sites/dreamcore-landing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(22).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/dreamcore-landing.webp

Build a single-page immersive parallax landing page in React + TypeScript + Tailwind CSS using Vite. The page has two scroll-driven scenes inside a sticky viewport. Everything lives in a single `src/App.tsx` file. Use Google Fonts: **Viaoda Libre** (serif headings) and **Imprima** (sans-serif body). No external UI libraries. Use `lucide-react` only as a dependency (it is not used in this page). Use Tailwind for responsive layout breakpoints only; all other styling is inline React `CSSProperties`.

---

### GLOBAL SETUP

**`tailwind.config.js`** -- Override the `xl` breakpoint to `1100px`:
```js
screens: { xl: '1100px' }
```

**`index.css`** -- Include Tailwind directives, global reset, dark background `#0a0608`, `font-family: 'Imprima', sans-serif`, `scrollbar-gutter: stable`, and a `@keyframes bobUp` animation that translates Y by `-6px` at 50%.

**`index.html`** -- Load Google Fonts via `<link>`:
```
https://fonts.googleapis.com/css2?family=Viaoda+Libre&family=Imprima&display=swap
```
Title: "Step Into Wonder"

---

### IMAGE ASSETS (use these exact URLs)

```
PORTAL_BG    = "https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779707217/image_1_vdzwae.png"
CURTAIN_LEFT = "https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779706559/curtain_left_znkmva.png"
CURTAIN_RIGHT= "https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779706564/curtain_right_paeyym.png"
WORLD_BG     = "https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779706392/image_2_gkcdlx.png"
BOTTOM_CLOUDS= "https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779706555/bottom_clouds_xskut6.png"

CARD_IMAGES[0] = "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260525_160507_2ccbb4eb-1469-484f-af25-59168ad9a233.png&w=1280&q=85"
CARD_IMAGES[1] = "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260525_160644_072a7f68-a101-4ded-a332-7d37707dbdd1.png&w=1280&q=85"
CARD_IMAGES[2] = "https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260525_160706_1c153d04-0dfb-4ac9-a4ef-e74f301c329c.png&w=1280&q=85"
```

---

### SCENE 2 CARD DATA (9 cards for the arc slider)

```
{ title: 'Hidden Realms',   desc: 'Luminous sanctuaries unseen by wandering eyes',  color: '#f3cdd6' }
{ title: 'Wild Solitudes',  desc: 'Dissolve into untamed horizons and deep calm',   color: '#dcedc2' }
{ title: 'Silent Havens',   desc: 'Remote escapes far beyond ordinary reach',       color: '#c3e3f4' }
{ title: 'Bespoke Quests',  desc: 'Journeys shaped around your vision and soul',    color: '#f0e4c0' }
{ title: 'Vivid Drifts',    desc: 'Surreal passages through breathtaking terrain',  color: '#dcd2f2' }
{ title: 'Mystic Crests',   desc: 'Timeless ridgelines wrapped in cloud and myth',  color: '#f3cdd6' }
{ title: 'Deep Currents',   desc: 'Glowing depths alive with uncharted wonder',     color: '#c3e3f4' }
{ title: 'Gilded Dusk',     desc: 'Amber horizons that stretch past all reason',    color: '#f0e4c0' }
{ title: 'Glassy Tides',    desc: 'Calm waters holding skies of pure stillness',    color: '#dcedc2' }
```

---

### ARCHITECTURE

The outer container is `height: 480vh; position: relative`. Inside it is a `position: sticky; top: 0; height: 100vh; overflow: hidden; background: #0a0608` viewport. All layers stack via absolute positioning and z-index.

**Scroll progress** = `window.scrollY / (container.scrollHeight - window.innerHeight)`, clamped 0-1.

**Helper functions:**
- `easeInOut(t)`: quadratic ease `t < 0.5 ? 2*t*t : -1 + (4 - 2*t)*t`
- `lerp(a, b, t)`: linear interpolation
- `clamp(val, min, max)`

**`useIsMobile()`** hook: `matchMedia('(max-width: 767px)')` -- returns boolean.

---

### LAYER STACK (bottom to top by z-index)

### Layer 1: World Background (z-index: auto/0)
- `ref={worldRef}`, absolute inset 0, `transformOrigin: '50% 50%'`
- `WORLD_BG` image, `object-fit: cover`
- Parallax: `scale(lerp(1, 1.18, ep))`, mouse offset `MAG.world = 6`

### Layer 2: Bottom Clouds (z-index: 10)
- `ref={cloudsRef}`, absolute bottom:0, left:0, right:0, `transformOrigin: '50% 100%'`
- `BOTTOM_CLOUDS` image, `width: 100%, height: auto`
- Parallax: `scale(lerp(1, 1.4, ep))`, mouse offset `MAG.clouds = 9` (Y dampened to `0.4x`)
- Opacity: fades from 0.7 to 1 in the first 5% of scroll

### Layer 2.5: Arc Card Slider (z-index: 9)
- Absolute, `bottom: 60px (mobile) / 80px (desktop)`, centered horizontally
- Opacity = `scene2Opacity`
- Contains `<ArcCardSlider>` component (details below)

### Layer 3: Portal Frame (z-index: 15)
- `ref={portalRef}`, absolute inset 0, `transformOrigin: '52% 38%'`
- `PORTAL_BG` image, `object-fit: cover`
- Parallax: `scale(lerp(1, 7.5, ep))`, mouse offset `MAG.portal = 7`
- Opacity: 1 until scroll 0.65, then fades to 0 by scroll 0.85

### Layer 3.5: Bottom Fade (z-index: 16)
- Absolute bottom, `height: 40%`, `linear-gradient(to top, rgba(0,0,0,0.45) 0%, transparent 100%)`, `pointer-events: none`

### Layer 4L: Curtain Left (z-index: 16)
- `ref={curtainLRef}`, absolute inset 0, `transformOrigin: 'left center'`
- `CURTAIN_LEFT` image, `object-fit: cover`, `object-position: right center`
- On mount (after 100ms), shifts left by `translateX(-62%)` with `transition: transform 1.8s cubic-bezier(0.16, 1, 0.3, 1)`
- On scroll: additional `translateX` via `lerp(0, 150, ep)%`, scale `lerp(1, 1.3, ep)`
- Mouse offset: `MAG.curtainL = 14` (Y dampened to `0.3x`)
- After entrance animation (2200ms), transition switches to `none` for responsive parallax

### Layer 4R: Curtain Right (z-index: 16)
- Mirror of Layer 4L but `transformOrigin: 'right center'`, `object-position: left center`
- Shifts right instead of left, `MAG.curtainR = 14`

### Top Fade Gradient (z-index: 45)
- Absolute top, `height: 42vh`, `linear-gradient(to bottom, rgba(0,0,0,0.45) 0%, transparent 100%)`, `pointer-events: none`

---

### NAVIGATION (z-index: 50)

Absolute top, full width, `display: flex, justify-content: space-between, align-items: center`.

**Nav link style:** `font-family: 'Imprima', sans-serif`, `font-size: 12px`, `letter-spacing: 0.12em`, `text-transform: uppercase`, `color: #fff`, `opacity: 0.9`, no text decoration.

**Mobile** (`padding: 18px 20px`): Three items -- "Explore" (11px) | StarLogo SVG | "Connect" (11px)

**Desktop** (`padding: 22px 48px`): Left group ["Worlds", "Atelier", "Immersions"] with `gap: 36px` | StarLogo SVG center | Right group ["Craft", "Codex", "Connect"] with `gap: 36px`

**StarLogo** -- inline SVG, 28x28, white star path + 3 small circles:
```
<path d="M14 2l2.09 6.42H23l-5.45 3.96 2.09 6.42L14 14.84l-5.64 4.06 2.09-6.42L4.96 8.42h6.95L14 2z" fill="white" opacity="0.9" />
<circle cx="14" cy="24" r="1.5" fill="white" opacity="0.6" />
<circle cx="6" cy="6" r="1" fill="white" opacity="0.4" />
<circle cx="22" cy="6" r="1" fill="white" opacity="0.4" />
```

---

### SCENE 1 UI (z-index: 20)

Opacity = `clamp(1 - scrollProgress / 0.22, 0, 1)`. Fades out in first ~22% of scroll.

Uses **three separate Tailwind-responsive layout blocks** (not JS branching for layout):

### Mobile layout (`md:hidden`)
- Centered column, `padding: 80px 24px 100px`
- Fade-in: `opacity 0.9s ease, transform 0.9s ease`, delay `0.3s`, triggers on `uiVisible`
- **Heading** (Viaoda Libre): "FALL > INTO" line (`clamp(26px, 7vw, 42px)`, `tracking-widest`, color `#3b1a0a`) then "REVERIE" (`clamp(52px, 16vw, 80px)`, `tracking-tight`, `leading-none`, color `#3b1a0a`). The ">" is a `›` character in color `#6b2e0e` at `0.8em`. "INTO" is italic.
- **Subtext** (Imprima): "Crafting boundless digital worlds where the edge between AI, vision, and living myth dissolves." -- `15px`, `leading-relaxed`, color `#5c2d0e`, `max-width: 280px`
- **Single card**: 140x140px, `border-radius: 22px`, `CARD_IMAGES[0]` as background-cover, `box-shadow: 0 8px 32px rgba(0,0,0,0.5)`. Bottom gradient overlay (60% height). Bottom-left overlay: white circle (26px) with play triangle SVG + "View Reel" text (13px, white).

### Tablet layout (`hidden md:flex xl:hidden`)
- Centered column, `gap: 28px`, `padding: 80px 32px 96px`
- Same fade-in animation as mobile
- **Heading**: same structure as mobile but dark brown text (`#3b1a0a`), sizes `clamp(28px, 5vw, 44px)` / `clamp(60px, 12vw, 86px)`
- **Subtext**: same text, `16px`, color `#5c2d0e`, `max-width: 400px`
- **Three cards in a row** (`flex gap-3.5`): each 140x140px, `border-radius: 22px`. Each has:
  - Background gradient overlay (60% height, multi-stop)
  - Backdrop blur layer (44% height, masked gradient)
  - Card 1: play button + "View Reel"
  - Card 2: number "32" (Viaoda Libre, 28px, white) + "World Patrons"
  - Card 3: play button + "View Reel"

### Desktop layout (`hidden xl:block` / `hidden xl:flex`)
- **Heading block**: absolute, `top: 46%`, `left: 60px`, `maxWidth: 440px`, `translateY(-50%)` centered
  - White text with heavy `text-shadow: 0 2px 24px rgba(0,0,0,0.7), 0 1px 4px rgba(0,0,0,0.9)`
  - "FALL > INTO": `clamp(32px, 4.5vw, 54px)`, `line-height: 1.1`, `letter-spacing: 0.04em`. The `>` is `rgba(255,220,180,0.7)`.
  - "REVERIE": `clamp(50px, 7.5vw, 88px)`, `line-height: 0.9`, `letter-spacing: -0.02em`
  - Subtext: `18px`, `line-height: 1.7`, color `rgba(255,245,235,0.88)`, `max-width: 300px`, `text-shadow: 0 1px 12px rgba(0,0,0,0.8)`
  - Fade-in: opacity+transform, delay `0.3s`

- **Cards block**: absolute, `right: 40px`, `top: 50%`, `translateY(-50%)`, `flex gap: 12px`
  - Three cards, each 158x158px, `border-radius: 28px`, `box-shadow: 0 8px 32px rgba(0,0,0,0.45)`
  - Each has: gradient overlay, backdrop blur layer (same as tablet), bottom content area at 12px inset
  - Play cards: 30px white circle + 18px "View Reel"
  - Number card: "32" at 36px Viaoda Libre + 18px "World Patrons"
  - Fade-in delay: `0.55s`

### Slider Dots (bottom of Scene 1)
- Absolute, bottom `28px (mobile, centered)` / `40px (desktop, left: 60px)`
- 4 dots: first is `28px wide`, rest `14px`, all `4px tall`, `border-radius: 2px`
- Active dot: `rgba(255,255,255,0.9)`, inactive: `rgba(255,255,255,0.35)`
- Fade-in delay: `0.8s`

### Scroll Cue (desktop only)
- Absolute `bottom: 36px`, centered
- "DESCEND" text: `10px`, `letter-spacing: 0.22em`, uppercase, `rgba(255,255,255,0.6)`
- Below: `ScrollChevron` -- 34px circle with 1.5px border `rgba(255,255,255,0.5)`, chevron SVG inside, `animation: bobUp 1.8s ease-in-out infinite`
- Fade-in delay: `0.9s`

---

### SCENE 2 UI (z-index: 46)

Opacity = `clamp((scrollProgress - 0.68) / 0.16, 0, 1)`. Fades in between scroll 68%-84%.

- Centered column
- **Heading** (Viaoda Libre): "FORGE BEYOND THE REAL" -- `clamp(28px, 8vw, 44px) mobile / clamp(38px, 6.5vw, 78px) desktop`, white, `letter-spacing: 0.03em`, `line-height: 1.05`, `text-shadow: 0 2px 20px rgba(0,0,0,0.4)`
- **Subtext** (Imprima): "Singular voyages to astonishing destinations, shaped for those who seek beauty beyond the ordinary and the known." -- `14px mobile / 20px desktop`, `line-height: 1.6`, `letter-spacing: -0.01em`, `max-width: 260px mobile / 480px desktop`, color `rgba(255,255,255,0.82)`
- Margin-top: `8vh mobile / 12vh desktop`

---

### ARC CARD SLIDER COMPONENT

Props: `cards[]`, `rotationOffset: number`, `isMobile: boolean`

**Layout math:**
- `cardSpacingDeg`: 12 (mobile) / 9 (desktop) degrees between cards
- `centerIndex`: `Math.floor(totalCards / 2)`
- `arcRadius`: 700 (mobile) / 1100 (desktop) px
- `cardW`: 160 (mobile) / 220 (desktop) px
- `cardH`: 175 (mobile) / 230 (desktop) px
- `sliderH`: 260 (mobile) / 360 (desktop) px

**`rotationOffset`** is driven by scroll: `lerp(0, arcSweepDeg, clamp((scrollProgress - 0.70) / 0.30, 0, 1))` where `arcSweepDeg = (totalCards - 1) * 10`.

**Per card positioning:**
```
baseDeg = (i - centerIndex) * cardSpacingDeg
deg     = baseDeg - rotationOffset + (centerIndex * cardSpacingDeg)
rad     = deg * PI / 180
x       = sin(rad) * arcRadius
y       = arcRadius - cos(rad) * arcRadius
```
Each card is absolutely positioned at `bottom: -y + (140 mobile / 200 desktop)px`, `left: calc(50% + x - halfW)`, `transform: rotate(deg)`, `transformOrigin: halfW arcRadius`.

**Card appearance:**
- Rounded rect (`18px mobile / 26px desktop`), background = `card.color` (pastel)
- `box-shadow: 0 8px 40px rgba(80,40,60,0.18)`
- Top-right: numbered circle (24px, `1.5px border rgba(80,50,60,0.3)`, text `rgba(80,50,60,0.6)`, 10px Imprima) showing zero-padded index
- Bottom: card title in Viaoda Libre (`22px mobile / 30px`, color `#3a2530`) + description in Imprima (`12px mobile / 15px`, color `rgba(58,37,48,0.65)`)

---

### ENTRANCE ANIMATION SEQUENCE

1. **t=100ms**: Curtains open -- `curtainsOpenRef` flips to true, causing 62% horizontal shift on each curtain with `1.8s cubic-bezier(0.16, 1, 0.3, 1)` transition
2. **t=600ms**: `uiVisible` = true -- all Scene 1 UI elements fade/slide in with staggered delays (0.3s heading, 0.55s cards, 0.8s dots, 0.9s scroll cue)
3. **t=2200ms**: `entranceDone` = true -- curtain CSS transition switches to `none` so parallax is instant

---

### MOUSE PARALLAX (desktop)

`requestAnimationFrame` loop smooths raw mouse position at `speed = 0.07` (lerp). Each layer is offset by its `MAG` value in the reverse direction of the mouse. The transforms combine mouse offset with scroll-driven scale/translate.

**MAG values:** world=6, clouds=9, portal=7, curtainL=14, curtainR=14

## E-commerce Website — Landing Page [sites/ecommerce-website-landing]

- Preview: https://motionsites.ai/assets/hero-ecommerce-website-preview-D7j_TrNR.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/ecommerce-website-landing.gif

Prompt to recreate this landing page:

Build a health/wellness e-commerce landing page using React, Vite, Tailwind CSS, TypeScript, and shadcn/ui. The design is clean, minimal, and modern with a near-white background (hsl(0 0% 98%)), near-black foreground (hsl(11 6% 11%)), and fully rounded borders (border-radius: 9999px). No custom fonts — use system defaults. The page has the following sections in order:

1. Sticky Navbar — bg-gray-50, sticky top, z-50. Left: text logo ("Logo", text-xl font-semibold). Center: horizontal nav links (Weight Loss, Strength, Anti-Aging, Hair Growth, Mood, More) hidden on mobile, styled with text-sm font-medium and hover opacity transition. Right: a solid dark "Get started" button (hidden on mobile) + a gradient-border button with a User icon and "Login" text. The gradient button has a 2px border using bg-gradient-to-r from-[#84a9fa] via-[#fb6fec] via-[#fba69e] via-[#fdd4a3] via-[#fb6fec] to-[#84a9fa] with bg-[length:200%] and a hover animation that shifts the gradient (backgroundPosition 0%→200% over 0.8s). The inner button has a white/background fill and rounded-full.

2. Hero Section — Two-column grid (lg:grid-cols-2). Left column: a rating badge (green circle with star icon, "4.5 Average Rating • 453 Reviews" in a pill with subtle shadow), a large heading ("Compounded Semaglutide for Weight Loss", text-4xl md:text-5xl lg:text-6xl font-semibold), three feature items with icons (Syringe, DollarSign, Truck from lucide-react) and text, a divider line, a pricing row ("$296/mo" bold + "*No matter the dose" subtext + "Get Started" button), and an info card (image thumbnail + "Is This Right for You?" + arrow link). Right column (hidden on mobile): two vertical auto-scrolling image marquees side by side. Each column has 4 product images duplicated for seamless looping. Uses CSS @keyframes marquee (translateY 0→-50%) and marquee-reverse (translateY -50%→0) at 30s linear infinite. Top and bottom have fade gradients (bg-gradient-to-b/t from-background to-transparent, h-32) overlaying the marquee.

3. Products Grid Section — Centered header with uppercase small text "OUR MEDS" + large heading "Medication Made Affordable Without The Insurance". Below: 3-column grid (md:grid-cols-2 lg:grid-cols-3) of product cards. Each card: square image with rounded-2xl shadow-lg hover:shadow-xl, title (text-2xl font-semibold), price with "per month" suffix, and a full-width gradient-border "Get Started" button that links to /product/{handle}.

4. Weight Loss Section — bg-gray-50, two-column grid. Left: heading "Lose weight with a plan made just for you." (text-4xl md:text-5xl lg:text-6xl font-semibold), three bullet features with Calendar, Pill, CheckCircle icons, two buttons ("Get started" solid + gradient "See if you're eligible"), and a small disclaimer. Right: a single product image with rounded-2xl.

5. Product Carousel Section — bg-gray-100, two-column grid. Left: a static full-height card (h-[32rem] sm:h-[40rem] md:h-[48rem] rounded-3xl) with background image, overlay text (price label, price, title), and gradient "Get Started" button. Right: a horizontal carousel of similar cards with left/right chevron buttons (bg-neutral-100/80 rounded-full) and pagination dots at bottom. Cards have group-hover:scale-105 on the background image.

6. Science & Nature Section — bg-gray-50 py-28, centered heading "Discover the harmony of science and nature." + two buttons. Below: 6-column grid (grid-cols-2 md:grid-cols-3 lg:grid-cols-6) of feature badges — each is a white card (rounded-2xl p-8 shadow-[2px_4px_12px_rgba(0,0,0,0.08)]) with a large icon (Rabbit, TreePine, Leaf, FlaskConical, Atom, Wheat — all w-20 h-20 strokeWidth-1.5) and label text with line breaks.

7. FAQ Section — bg-gray-50, uses Radix accordion. Each item is a white card (rounded-3xl px-14 py-8 shadow-[2px_4px_12px_rgba(0,0,0,0.08)] border-none) with text-2xl font-semibold trigger and text-lg content. Items are spaced with space-y-4. 5 FAQ items about GLP-1 programs, insurance, medications, pricing guarantee, and plan inclusions.

8. Health Guide Section — bg-gray-50 py-28, centered heading "Your guide to health and wellness starts here." + two buttons. Below: 4-column grid of guide cards. Each card: white with rounded-3xl, image at top (h-48 object-cover rounded-3xl), description text, and a pill-shaped link button with category name + chevron icon, styled with border-2 border-zinc-900/[0.13] rounded-full.

9. Footer — bg-zinc-900 text-white. Two-column layout: left has logo + email signup (input with rounded-full + submit button) + privacy text. Right has 3-column link grid (Popular, Company, Legal). Below divider: social media SVG icons (Facebook, Instagram, X, TikTok, LinkedIn, YouTube) + LegitScript badge + compounded pharmacy badge. Bottom disclaimer text.

Design system: All colors via CSS custom properties in HSL. Semantic tokens: --background, --foreground, --primary, --card, --muted, --border, etc. Shadows use shadow-[2px_4px_12px_rgba(0,0,0,0.08)]. Border radius globally set to 9999px via --radius. The gradient button is a reusable component used throughout.

## Email Landing Page — Landing page [sites/email-landing-page]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(59).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/email-landing-page.webp

Build a premium, AI-native email client landing page called "Aura" using **React 18 + TypeScript + Vite + Tailwind CSS + motion/react (framer motion) + lucide-react**. The aesthetic is dark (bg `#0c0c0c`), cinematic, glassy, with a looping fullscreen background video, a shiny gradient headline, a macOS-style menu bar, a realistic inbox mockup, and a custom "liquid-glass" card treatment.

### Stack / setup

- `package.json` dependencies: `react`, `react-dom`, `@supabase/supabase-js`, `motion` (v12+, import from `motion/react`), `lucide-react`.
- Tailwind config extends colors with `brand: '#3D81E3'` and fontFamily sans with `['Inter','system-ui','sans-serif']`.
- Font: Google Fonts Inter weights 400, 500, 600, 700, 800, 900. Import in `index.css` via `@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap');`.
- `html,body { font-family: 'Inter', system-ui, sans-serif; -webkit-font-smoothing: antialiased; }`.
- Background color base `#0c0c0c`, text white, selection `bg-brand/30`.

### Global background video (fixed, behind everything)

Inside the root wrapper (`relative min-h-screen overflow-x-hidden bg-[#0c0c0c] text-white`), render a fixed full-screen video:

```
<div className="fixed inset-0 z-0 pointer-events-none">
  <video autoPlay loop muted playsInline
    className="w-full h-full object-cover pointer-events-none"
    src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_064122_c4750c0e-7476-4b44-94a2-a85a65c63bf2.mp4" />
</div>
```

Also render two hidden-on-mobile fixed vertical guide lines at the 36rem container edges:
```
<div className="hidden md:block pointer-events-none fixed inset-y-0 left-1/2 -translate-x-[calc(50%+36rem)] w-px bg-white/10 z-[5]" />
<div className="hidden md:block pointer-events-none fixed inset-y-0 left-1/2 translate-x-[calc(-50%+36rem)] w-px bg-white/10 z-[5]" />
```

### Global SVG noise filters (two, both id `c3-noise`)

- One at root level (subtle grain, multiply blend) for the shiny headline.
- One inside the pricing section (fractal noise, overlay blend) for the watermark.

Root filter:
```
<filter id="c3-noise">
  <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" stitchTiles="stitch" />
  <feColorMatrix type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.35 0" />
  <feComposite in2="SourceGraphic" operator="in" result="noise" />
  <feBlend in="SourceGraphic" in2="noise" mode="multiply" />
</filter>
```

Pricing filter:
```
<filter id="c3-noise">
  <feTurbulence type="fractalNoise" baseFrequency="0.5" numOctaves="2" stitchTiles="stitch" />
  <feComponentTransfer><feFuncA type="linear" slope="0.075" /></feComponentTransfer>
  <feComposite in2="SourceGraphic" operator="in" result="noise" />
  <feBlend in="SourceGraphic" in2="noise" mode="overlay" />
</filter>
```

### Shared primitives

**AppleLogo** — inline SVG Apple mark, `viewBox="0 0 384 512"`, `fill="currentColor"`, default `w-4 h-4`. Path:
`M318.7 268.7c-.2-36.7 16.4-64.4 50-84.8-18.8-26.9-47.2-41.7-84.7-44.6-35.5-2.8-74.3 20.7-88.5 20.7-15 0-49.4-19.7-76.4-19.7C63.3 141.2 4 184.8 4 273.5q0 39.3 14.4 81.2c12.8 36.7 59 126.7 107.2 125.2 25.2-.6 43-17.9 75.8-17.9 31.8 0 48.3 17.9 76.4 17.9 48.6-.7 90.4-82.5 102.6-119.3-65.2-30.7-61.7-90-61.7-91.9zm-56.6-164.2c27.3-32.4 24.8-61.9 24-72.5-24.1 1.4-52 16.4-67.9 34.9-17.5 19.8-27.8 44.3-25.6 71.9 26.1 2 49.9-11.4 69.5-34.3z`.

**LogoMark** — abstract 4-quadrant curve mark, `viewBox="0 0 256 256"`, default `w-8 h-8`, white fill. Path:
`M 0 128 C 70.692 128 128 185.308 128 256 L 64 256 C 64 220.654 35.346 192 0 192 Z M 256 192 C 220.654 192 192 220.654 192 256 L 128 256 C 128 185.308 185.308 128 256 128 Z M 128 0 C 128 70.692 70.692 128 0 128 L 0 64 C 35.346 64 64 35.346 64 0 Z M 192 0 C 192 35.346 220.654 64 256 64 L 256 128 C 185.308 128 128 70.692 128 0 Z`.

**AppleButton** — rounded-full white pill, Apple logo + "Download Aura" label + ChevronRight. Chevron translates `+1px` on group hover. Classes: `group inline-flex items-center justify-center gap-2 rounded-full bg-white text-black font-medium text-sm px-5 py-3 transition-all hover:bg-white/90 active:scale-[0.98]`. Accepts `label` and `full` props.

**SectionEyebrow** — `<span className="w-1.5 h-1.5 rounded-full bg-white" />` + label, optional tag pill with `px-2 py-0.5 rounded-full border border-white/10 text-white/50`.

**gradientStyle** used on the headline word "Revitalized":
```
backgroundImage: 'linear-gradient(to right, #091020 0%, #0B2551 12.5%, #A4F4FD 32.5%, #00d2ff 50%, #0B2551 67.5%, #091020 87.5%, #091020 100%)'
backgroundSize: '200% auto'
WebkitBackgroundClip: 'text' (+ backgroundClip text)
color: 'transparent'; WebkitTextFillColor: 'transparent'
filter: 'url(#c3-noise)'
```

Shiny animation (`.animate-shiny`): 6s linear infinite, keyframes shiny `{0%: background-position: -200% center; 100%: 200% center;}`.

### Liquid-glass utility (used across cards)

```
.liquid-glass {
  background: rgba(255,255,255,0.01);
  background-blend-mode: luminosity;
  backdrop-filter: blur(4px);
  border: none;
  box-shadow: inset 0 1px 1px rgba(255,255,255,0.1);
  position: relative; overflow: hidden;
}
.liquid-glass::before {
  content: ''; position: absolute; inset: 0; border-radius: inherit;
  padding: 1.4px;
  background: linear-gradient(180deg,
    rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%,
    rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%,
    rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor; mask-composite: exclude;
  pointer-events: none;
}
```

### Section 1 — Navbar

Max-width `max-w-6xl mx-auto px-6`. Motion nav fades/slides down (opacity 0 -> 1, y -10 -> 0, 0.6s easeOut). Left: just the `LogoMark` (NO "Aura" word). Center (`hidden md:flex gap-8`): links `['Solutions','Pricing','Blog','Documentation','Careers']` each `text-white/70 text-sm font-medium hover:text-white` with staggered y animation (delay 0.1 + i*0.05). Right desktop: `<AppleButton />` default label "Download Aura". Mobile right: `w-10 h-10 rounded-full border border-white/10 bg-white/5` Menu icon button.

### Section 2 — Hero

Centered section, `pt-16 md:pt-28 pb-20 text-center flex flex-col items-center`.
Motion h1 (delay 0.3, 0.8s cubic-bezier(.22,1,.36,1)), classes `text-4xl md:text-7xl font-semibold tracking-tight leading-[0.9]`:
- Line 1: "Your email." (white)
- Line 2: "Revitalized" — apply `animate-shiny` and the `gradientStyle` inline.

Then motion paragraph (delay 0.5): `mt-8 text-white/60 max-w-md text-base leading-[1.5]`:
> "Aura is the premier inbox platform for the current era. It leverages powerful AI to organize, prioritize, and refine your messages into total clarity."

Then motion div (delay 0.7) with `<AppleButton />` and `text-xs text-white/40` "Download for Intel / Apple Silicon".

### Section 3 — macOS menu bar strip

Full-width bar `h-10 bg-black/40 backdrop-blur-md border-t border-b border-white/10`. Inside `max-w-6xl mx-auto px-6 h-full flex items-center justify-between text-xs`. Left: `AppleLogo w-3.5 h-3.5`, bold white "Aura", then menu items `['File','Edit','View','Go','Window','Help']` (progressive hiding: index>2 `hidden sm:inline`, index>3 `hidden md:inline`). Right: `Search w-3.5 h-3.5` + "Wed May 6 1:09 PM". Enters with delay 0.9.

### Section 4 — Inbox mockup

`max-w-6xl mx-auto px-6 py-16 md:py-24`. Outer container `relative rounded-2xl overflow-hidden border border-white/10 bg-[#0e1014]/90 backdrop-blur-2xl`. Motion enters from y:40 at delay 1.1.

Title bar: three traffic lights `#ff5f57`, `#febc2e`, `#28c840` (each `w-3 h-3 rounded-full`); center label "Aura — Inbox" `text-xs text-white/50`.

Body `grid grid-cols-12 h-[520px]`:

**Sidebar (col-span-3, border-r, bg-black/30, p-4):**
- White "Compose with Aura" button with `Sparkles` icon (`rounded-lg bg-white text-black text-xs font-semibold px-3 py-2`).
- Nav items (icon + label + optional count): Inbox (12, active), Starred (3), Sent, Drafts (2), Archive, Trash. Active uses `bg-white/10 text-white`, others `text-white/60 hover:bg-white/5`.
- Labels section: uppercase tracking "Labels" small title, then 4 color dots: Work `#00d2ff`, Personal `#A4F4FD`, Travel `#f59e0b`, Finance `#10b981`.

**Message list (col-span-4, border-r):**
- Search header: `Search` icon + placeholder "Search mail".
- 6 messages with name, subject, preview, time, unread/active flags:
  - Linear — "Weekly product digest" — "Your team shipped 23 issues this week..." — 9:41 AM — unread + active
  - Sophia Chen — "Re: Q3 roadmap review" — "Thanks for sending the deck over. I had a few thoughts..." — 8:12 AM — unread
  - Figma — "Marcus commented on your file" — "Love the new direction on the landing hero." — Yesterday
  - Stripe — "Payout of $12,480.00 sent" — "Your payout is on its way to your bank..." — Yesterday
  - Vercel — "Deployment ready for aura-web" — "Preview is live at aura-web-g3f.vercel.app" — Mon
  - GitHub — "[aura/core] PR #482 approved" — "david-lim approved your pull request." — Mon

**Reader (col-span-5):**
- Toolbar with Reply, Forward, Archive, Trash2 icon buttons (each `w-7 h-7 rounded-md hover:bg-white/5`) and a MoreHorizontal on the right.
- Header: "Weekly product digest"; sender avatar gradient bubble `w-7 h-7 rounded-full bg-gradient-to-br from-[#00d2ff] to-[#0B2551]` with "L"; "Linear" + "to me · 9:41 AM"; "Work" pill.
- Body:
  - Card with `Sparkles` icon (color `#A4F4FD`) labeled "Summary by Aura" and text "Your team closed 23 issues, merged 14 PRs, and shipped 2 features. Top contributor: Marcus. No action needed."
  - Paragraphs: "Hi team,", "Here is your weekly digest of everything happening across your projects. This was a strong week with significant progress on the Q3 roadmap.", "Twenty-three issues were closed, fourteen pull requests were merged, and two customer-facing features went out. The velocity trend continues to climb.", "Let me know if you would like a deeper breakdown by project or contributor.", "— The Linear team" (`text-white/50`).
  - Attachment pill with `Paperclip` icon: "digest-may-6.pdf".

### Section 5 — FeatureTriage

`max-w-6xl mx-auto px-6 py-20 md:py-28`, two-column grid `grid md:grid-cols-2 gap-10 md:gap-16 items-start`.

Left column motion (y 20 -> 0, 0.7s): `SectionEyebrow label="Triage" tag="AI-native"`, h2 `mt-5 text-3xl md:text-5xl font-semibold tracking-tight leading-[1.02]`: "Clear your inbox" <br/> "in a single pass.". Paragraph `mt-6 text-white/60 text-base leading-[1.6] max-w-md`: "Aura reads every message, understands intent, and routes the noise away from the signal. Focus on what moves your day forward — the rest handles itself." Chips row (`text-xs text-white/70 px-3 py-1.5 rounded-full border border-white/10 bg-white/[0.03]`): "Auto-categorize", "Snooze for later", "Silent newsletters", "One-tap unsubscribe".

Right column: `liquid-glass rounded-2xl p-5` card. Eyebrow text: "Today · 42 messages triaged". Four sub-cards (each `liquid-glass rounded-lg p-3`):
- Priority (4) `#ffffff` — items: "Sophia Chen — Q3 review", "David Lim — contract signoff"
- Follow-up (7) `#e5e5e5` — items: "Marcus — design review", "Figma — comment thread"
- Updates (18) `#a3a3a3` — items: "Vercel — deploy ready", "GitHub — PR #482 merged"
- Archived (13) `#525252` — items: "Stripe payout · Newsletter · Receipts"

### Section 6 — LogoCloud

`max-w-6xl mx-auto px-6 py-16 md:py-20`. Centered kicker `text-xs uppercase tracking-widest text-white/40`: "Trusted by the world's most thoughtful teams". Grid `mt-10 grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-6`, each logo name as `text-sm font-semibold tracking-tight text-white/50 hover:text-white`. Names: Linear, Vercel, Figma, Stripe, Ramp, Notion, Loom, Arc. Each fades in with stagger 0.05.

### Section 7 — Testimonials

`max-w-6xl mx-auto px-6 py-20 md:py-28 border-t border-white/10`. 3-col grid of `liquid-glass rounded-2xl p-6` figures. Each: blockquote `text-sm text-white/80 leading-[1.6]` wrapped in quotes, `figcaption mt-6 pt-5 border-t border-white/10` with name `text-sm font-semibold`, role `text-xs text-white/50`, company uppercased `text-xs text-white font-semibold tracking-wide`.
- "Aura gave our leadership team four hours of their week back. It reads like email from the future." — Parker Wilf, Group Product Manager, MERCURY
- "The command palette alone has changed how I process messages. I can't imagine going back to a traditional client." — Andrew von Rosenbach, Senior Engineering Program Manager, COHERE
- "Triage that actually understands context. Our team stopped dreading Monday morning inboxes." — Mathies Christensen, Engineering Manager, LUNAR

### Section 8 — Pricing

Uses custom CSS classes (not Tailwind) for cinematic typography.

Outer `<section className="c3-pricing-section">` with its own `<svg>` defining the `c3-noise` pricing filter described earlier.

Watermark (giant hero headline as backdrop):
```
<div className="c3-watermark-container">
  <div className="c3-watermark-main">
    <span className="c3-watermark-line-1">Your email.</span>
    <span className="c3-watermark-line-2">Revitalized</span>
  </div>
</div>
```

State: `yearly` boolean toggle. Three plans:
- **Free** — "Free" — "For creators taking their first steps with Forma." — Up to 3 projects in the cloud / Image export up to 1080p / Basic editing tools / Free templates and icons / Access via web and mobile app.
- **Standard** — monthly "$9,99/m" yearly "$99,99/y" — "For freelancers and small teams who need more freedom and flexibility." — Up to 50 projects in the cloud / Export up to 4K / Advanced editing toolkit / Team collaboration (up to 5 members) / Access to premium template library.
- **Pro** (`c3-card-pro`) — monthly "$19,99/m" yearly "$199,99/y" — "For studios, agencies, and professional creators working with brands." — Unlimited projects / Export up to 8K + animations / AI-powered content generation tools / Unlimited team members / Brand customization.

Each card renders: `c3-tier-small` (tier), `c3-tier-large` (price), `c3-desc`, `c3-list` of checkmark rows (white circle `c3-check` with white SVG check), `c3-btn` "Choose Plan".

Below: `c3-toggle-wrap` with "Yearly" label and a pill toggle (white knob black when off; when `.active`, background `rgba(255,255,255,0.2)`, knob white, translated 24px).

Pricing CSS (key values, include exactly):
- `.c3-pricing-section { position: relative; padding: 40px 20px 80px; display: flex; flex-direction: column; align-items: center; overflow-x: hidden; }`
- `.c3-watermark-container { position: relative; width: 100%; max-width: 1100px; text-align: center; margin-top: 40px; z-index: 2; }`
- `.c3-watermark-main { font-size: 9rem; font-weight: 800; line-height: 0.9; letter-spacing: -0.05em; filter: url(#c3-noise); display: flex; flex-direction: column; align-items: center; }`
- `.c3-watermark-line-1 { color: #fff; }`
- `.c3-watermark-line-2 { background: linear-gradient(to right, #091020 0%, #0B2551 25%, #A4F4FD 65%, #00d2ff 100%); -webkit-background-clip: text; background-clip: text; color: transparent; -webkit-text-fill-color: transparent; }`
- `.c3-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 24px; width: 100%; max-width: 1100px; margin-top: 60px; transform: translateX(20px); position: relative; z-index: 3; }`
- `.c3-card { background: linear-gradient(135deg, rgba(0,0,0,0.7), rgba(0,0,0,0.4)); backdrop-filter: blur(14px) brightness(0.91); border: 1px solid rgba(255,255,255,1); border-radius: 44px; padding: 50px 24px; min-height: 580px; display: flex; flex-direction: column; transition: all 0.6s cubic-bezier(.22,1,.36,1); overflow: hidden; position: relative; }`
- `.c3-card::before { content:''; position:absolute; inset:0; border-radius:inherit; background: linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0) 50%); pointer-events:none; }`
- `.c3-card:hover { background: rgba(15,15,15,0.6); border-color: rgba(34,211,238,0.7); transform: translateY(-12px) scale(1.01); }`
- `.c3-card-pro { background: linear-gradient(135deg, rgba(0,0,0,0.85), rgba(0,0,0,0.55)); }`
- `.c3-tier-small { font-size: 1.1rem; font-weight: 400; color: rgba(255,255,255,0.6); }`
- `.c3-tier-large { font-size: 2.8rem; font-weight: 500; letter-spacing: -0.02em; color: #fff; margin-top: 8px; }`
- `.c3-desc { font-size: 0.88rem; color: rgba(255,255,255,0.45); min-height: 3.2em; margin-top: 16px; margin-bottom: 40px; line-height: 1.5; }`
- `.c3-list li { display:flex; align-items:flex-start; gap: 14px; font-size: 0.92rem; color: rgba(255,255,255,0.8); margin-bottom: 18px; line-height: 1.4; }`
- `.c3-check { width:28px; height:28px; border-radius:50%; background: rgba(255,255,255,0.15); display:inline-flex; align-items:center; justify-content:center; flex-shrink:0; }`
- `.c3-btn { background:#fff; color:#000; padding: 10px 32px; border-radius: 100px; font-weight:600; font-size: 0.88rem; margin-top:auto; border:none; cursor:pointer; align-self:center; transition: all 0.3s cubic-bezier(.22,1,.36,1); }`
- `.c3-btn:hover { background:#f5f5f5; transform:scale(1.02); box-shadow: 0 8px 24px rgba(255,255,255,0.15); }`
- `.c3-toggle-wrap { display:flex; align-items:center; justify-content:flex-end; gap:12px; width:100%; max-width:1100px; margin-top:32px; padding-right:20px; }`
- `.c3-toggle { width:52px; height:28px; background:#fff; border-radius:100px; position:relative; cursor:pointer; border:none; transition: background 0.3s cubic-bezier(.4,0,.2,1); padding:0; }`
- `.c3-toggle-knob { width:20px; height:20px; background:#000; border-radius:50%; position:absolute; top:4px; left:4px; transition: all 0.3s cubic-bezier(.4,0,.2,1); }`
- `.c3-toggle.active { background: rgba(255,255,255,0.2); }`
- `.c3-toggle.active .c3-toggle-knob { transform: translateX(24px); background:#fff; }`
- Media query `(max-width:1024px)`: `.c3-watermark-main { font-size: 3.5rem; filter:none; }`, `.c3-watermark-line-2 { background:none; -webkit-text-fill-color:#00d2ff; color:#00d2ff; }`, `.c3-grid` becomes horizontal scroll-snap flex (`display:flex; overflow-x:auto; scroll-snap-type:x mandatory; transform:none; width:100vw; padding:0 20px; gap:16px; scrollbar-width:none`), cards `flex: 0 0 320px; scroll-snap-align:center`, `.c3-grid::-webkit-scrollbar{display:none}`, `.c3-toggle-wrap { justify-content:center; padding-right:0; }`.

### Section 9 — FinalCTA

`max-w-6xl mx-auto px-6 py-20 md:py-32`. Motion `liquid-glass relative overflow-hidden rounded-3xl px-8 py-16 md:py-24 text-center`. Radial glow overlay: `radial-gradient(600px circle at 50% 0%, rgba(255,255,255,0.15), transparent 70%)` at opacity 0.3.
- h2 `text-4xl md:text-6xl font-semibold tracking-tight leading-[1.02]`: "Close the tabs." / "Open your day.".
- Paragraph `mt-6 text-white/60 max-w-md mx-auto text-sm leading-[1.6]`: "Join thousands of builders, founders, and operators who treat email like a tool — not an obligation."
- Buttons: `<AppleButton label="Download Aura" />` and `rounded-full border border-white/15 text-white text-sm font-medium px-5 py-3 hover:bg-white/5` "Talk to sales" + ChevronRight.


Reproduce exactly — fonts, gradient stops, noise filters, copy strings, animation delays, and the CloudFront video URL `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_064122_c4750c0e-7476-4b44-94a2-a85a65c63bf2.mp4`.

## Financial Suite — Landing Page [sites/financial-suite]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/prompts%20(i've%20added%20them%20to%20the%20motionsites)/f4444Area.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/financial-suite.mp4

Build a single-page premium credit card landing page called **"Infinite"** using **React + Vite + TypeScript + Tailwind CSS + lucide-react**. The page background is `#0A0B11`. The page title is "Infinite - Premium Credit Card".

**Tech Stack:**
- Vite + React 18 + TypeScript
- Tailwind CSS 3.4
- lucide-react (icons: `Play`, `Menu`, `X`, `User`, `Plus`)
- No other dependencies

**Fonts:**
- Primary: **Geist** from Google Fonts (`https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&display=swap`), applied globally via `* { font-family: 'Geist', sans-serif; }`
- Secondary: **Helvetica Neue Roman** loaded locally via `@font-face` from `/fonts/HelveticaNeue-Roman.woff2` and `.woff`. Applied via a utility class `.font-helvetica-neue` (and all children) used on the hero section only.

**Tailwind config extension:**
```js
transitionDuration: { '400': '400ms' }
```

---

### GLOBAL CSS (index.css)

```css
@import url('https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600;700&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

@font-face {
  font-family: 'Helvetica Neue Roman';
  src: url('/fonts/HelveticaNeue-Roman.woff2') format('woff2'),
       url('/fonts/HelveticaNeue-Roman.woff') format('woff');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

* {
  font-family: 'Geist', sans-serif;
}

.font-helvetica-neue,
.font-helvetica-neue * {
  font-family: 'Helvetica Neue Roman', 'Helvetica Neue', Helvetica, Arial, sans-serif;
}

@keyframes fadeSlideUp {
  0% { opacity: 0; transform: translateY(24px); }
  100% { opacity: 1; transform: translateY(0); }
}

@keyframes fadeIn {
  0% { opacity: 0; }
  100% { opacity: 1; }
}

.anim-stagger {
  opacity: 0;
  transform: translateY(24px);
  animation: fadeSlideUp 0.9s cubic-bezier(0.16, 1, 0.3, 1) both;
}

.anim-fade {
  opacity: 0;
  animation: fadeIn 1s cubic-bezier(0.16, 1, 0.3, 1) both;
}
```

---

### CONSTANTS

```
SCROLL_DISTANCE = 1800 (pixels of scroll to scrub through full video)
SPOTLIGHT_R = 260 (radius of cursor spotlight reveal effect in pixels)
GRID_CELL = 48 (grid pattern cell size in pixels)
```

---

### IMAGE & VIDEO URLS

```
BG_IMAGE_1 (hero base): https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_235011_7a23734e-7fe9-4491-ac28-e46133f980c2.png&w=1280&q=85

BG_IMAGE_2 (hero spotlight reveal): https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260630_011539_f0e4cacc-143c-4bec-9db3-d3415e656a83.png&w=1280&q=85

CARD_IMAGE_1 (card section base): https://soft-zoom-63098134.figma.site/_assets/v11/47ffd9fae3c79a54bef0ff41737f6ad654c92213.png?w=1024

CARD_IMAGE_2 (card section spotlight reveal): https://soft-zoom-63098134.figma.site/_assets/v11/400e1612d64f65aee1c05735530d6f7a86ae3b8d.png?w=1024

VIDEO_1 (scroll-scrubbed video): https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260630_060707_72cd8ca2-3e4b-460c-9293-575573810866.mp4
```

---

### Z-INDEX HIERARCHY

```
z-0:  scroll spacer div
z-2:  fixed video layer
z-3:  sticky hero section
z-4:  fixed card section
z-5:  content overlays within card section
z-10: "+ More" button in card section
z-30: spotlight reveal layers
z-40: bottom gradient overlays
z-50: hero bottom content
z-54: mobile menu backdrop overlay
z-55: mobile dropdown menu
z-60: fixed navigation bar
```

---

### NAVIGATION BAR (fixed, z-60, all sections)

A `position: fixed; top: 0; left: 0; right: 0` nav with `z-[60]`, flex row, items centered, justify-between. Padding: `px-5 sm:px-8 md:px-10 py-4 sm:py-5`.

**Left: Logo + Wordmark**
- Custom SVG logo (24x24, white, viewBox 0 0 256 256) — a geometric 4-quadrant shape with cut corners:
  ```
  M 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 128 L 64 128 Z
  M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z
  M 128 64 L 128 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 Z
  M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 128 0 L 192 0 Z
  ```
- "INFINITE" text: `text-sm font-medium tracking-wide uppercase text-white`

**Center (hidden on mobile, `hidden md:flex`):** Pill navigation
- Container: `absolute left-1/2 -translate-x-1/2 bg-white/10 backdrop-blur-md rounded-full px-1.5 py-1.5 items-center gap-0.5`
- Active tab "Card": `bg-white text-gray-900 text-sm font-medium px-4 py-1.5 rounded-full`
- Inactive tabs ("Rewards", "Travel", "Plans", "Support"): `text-white/70 text-sm font-medium px-4 py-1.5 rounded-full hover:text-white transition-colors`

**Right (hidden on mobile, `hidden md:flex`):**
- Account button: pill with embedded avatar circle. `flex items-center gap-2 bg-white/10 backdrop-blur-md text-white/80 text-sm font-medium pl-1.5 pr-4 py-1.5 rounded-full hover:text-white hover:bg-white/15 transition-colors`. Inner circle: `w-7 h-7 rounded-full bg-white/20` with `User` icon (size 14, strokeWidth 1.8).
- "Get Started" button: `bg-white text-gray-900 text-sm font-medium px-5 py-2 rounded-full hover:bg-white/90 transition-colors`

**Mobile hamburger (md:hidden):**
- Button: `text-white p-2 relative w-8 h-8 flex items-center justify-center`
- Two overlapping icon spans (X and Menu) with crossfade + rotation animation: the active icon is `opacity-100 rotate-0`, the inactive is `opacity-0 rotate-90` (or `-rotate-90`). Both have `absolute transition-all duration-300`.

**Mobile menu overlay (z-54):**
- `fixed inset-0 bg-black/60 backdrop-blur-sm md:hidden transition-opacity duration-300`
- Visible when menuOpen, otherwise `opacity-0 pointer-events-none`
- Clicking closes menu

**Mobile dropdown menu (z-55):**
- `fixed top-0 left-0 right-0 bg-[#0A0B11]/98 backdrop-blur-xl pt-20 pb-8 px-6 border-b border-white/10 flex flex-col gap-0 md:hidden`
- Transform animation: `transition-all duration-400 ease-[cubic-bezier(0.16,1,0.3,1)] origin-top`
- Open state: `translate-y-0 opacity-100 scale-y-100`
- Closed state: `-translate-y-4 opacity-0 scale-y-95 pointer-events-none`
- Menu items: ['Card', 'Rewards', 'Travel', 'Plans', 'Support'] — each is `text-white/80 text-[17px] font-medium py-4 border-b border-white/[0.06] text-left hover:text-white transition-all duration-300`
- Staggered entrance: each item gets `transitionDelay: ${80 + i * 40}ms` when open, `0ms` when closed
- Bottom buttons group (Account + Get Started) with `transitionDelay: 300ms`

---

### SECTION 1: HERO SECTION (sticky, z-3)

Component: `HeroSection({ faded: boolean })`

A full-viewport section with:
- `position: sticky; top: 0; height: 100vh; overflow: clip; z-index: 3`
- Class: `font-helvetica-neue relative w-full`
- The entire inner content wrapper has `transition-opacity duration-500` and fades to `opacity: 0` when `faded` is true (when scroll begins).

**Mouse tracking system:**
- Raw mouse position tracked via `mousemove` listener
- Smoothed position via lerp in a `requestAnimationFrame` loop: `smooth += (raw - smooth) * 0.1`
- Grid offset calculated: cursor position relative to section center * 16, eased at 0.06 factor

**Layer 1: SVG Grid Pattern (z-0, opacity 8%)**
- Full absolute inset-0 SVG with a `<pattern>` element
- Cell size: 48px x 48px, `patternUnits="userSpaceOnUse"`
- Pattern x/y offset driven by the smoothed grid offset (parallax)
- Path draws an L-shape: `M 48 0 L 0 0 0 48` (top and left edges of each cell)
- Stroke: `#94a3b8`, strokeWidth: 0.5, fill: none
- A full `<rect width="100%" height="100%">` fills with the pattern

**Layer 2: Base Background Image (z-10)**
- `absolute inset-0 bg-center bg-cover bg-no-repeat`
- Uses `BG_IMAGE_1` as background-image
- Has class `anim-fade` with `animationDelay: '0.1s'`

**Layer 3: Spotlight Reveal Layer (z-30)**
- Component: `RevealLayer({ image, cursorX, cursorY })`
- A hidden `<canvas>` sized to window dimensions (resized on window resize)
- On every render (no dependency array on the useEffect), draws a radial gradient at cursor position:
  - `createRadialGradient(cursorX, cursorY, 0, cursorX, cursorY, 260)`
  - Stops: 0 -> white 100%, 0.4 -> white 100%, 0.6 -> white 75%, 0.75 -> white 40%, 0.88 -> white 12%, 1.0 -> white 0%
  - Draws filled arc circle
- Canvas is converted to `toDataURL()` and applied as CSS mask (`maskImage` + `webkitMaskImage`) to the image div
- Image div: `absolute inset-0 bg-center bg-cover bg-no-repeat z-30 pointer-events-none` with `BG_IMAGE_2`

**Layer 4: Bottom Gradient (z-40)**
- `absolute bottom-0 left-0 right-0 h-72 pointer-events-none`
- `bg-gradient-to-t from-[#0A0B11] via-[#0A0B11]/60 to-transparent`

**Layer 5: Hero Content (z-50)**
- `absolute bottom-0 left-0 right-0 px-6 sm:px-10 md:px-14 pb-12 sm:pb-16 md:pb-20`
- 12-column grid: `grid grid-cols-1 md:grid-cols-12 gap-8 md:gap-12 items-end`

**Left column (md:col-span-7 lg:col-span-8):**

1. Badge line (animationDelay: 0.3s, class: anim-stagger):
   - White circle: `w-2.5 h-2.5 rounded-full bg-white/80`
   - Text: "Best digital banking card 2026" — `text-sm sm:text-[15px] text-white/80 font-normal tracking-wide`

2. Heading (animationDelay: 0.5s, class: anim-stagger):
   - `"One Card, Zero\nLimits. Worldwide."` (line break after "Zero")
   - `text-[clamp(2.2rem,6.5vw,5rem)] font-light text-white leading-[0.95] tracking-[-0.03em] mb-8 sm:mb-10`

3. Buttons row (animationDelay: 0.7s, class: anim-stagger):
   - "See Features": `bg-white text-gray-900 text-sm font-medium px-6 sm:px-7 py-3 sm:py-3.5 rounded-full hover:bg-white/90 transition-all`
   - "How It Works": `flex items-center gap-2.5 bg-white/5 backdrop-blur-sm border border-white/10 text-white/90 text-sm font-medium px-6 sm:px-7 py-3 sm:py-3.5 rounded-full hover:bg-white/15 transition-all` with `Play` icon (size 13, `fill-white/90`)

**Right column (md:col-span-5 lg:col-span-4, animationDelay: 0.85s, class: anim-stagger):**
- Paragraph: "Infinite is a premium metal credit card built for those who move fast and spend globally. Tap anywhere, earn instantly, skip foreign fees entirely, and travel already rewarded."
- `text-[15px] sm:text-base text-white/75 leading-relaxed font-normal`

---

### SECTION 2: SCROLL-DRIVEN VIDEO LAYER (fixed, z-2)

A `position: fixed; inset: 0; z-index: 2` div containing:

- `transition-opacity duration-500 pointer-events-none`
- Opacity: 1 when videoPhase is not 'idle', 0 when 'idle'

**Video element:**
- `src={VIDEO_1}`, `muted`, `playsInline`, `preload="auto"`
- `className="w-full h-full object-cover"`
- Does NOT autoplay — `currentTime` is set programmatically

**Bottom gradient overlay:**
- `absolute bottom-0 left-0 right-0 h-72 bg-gradient-to-t from-[#0A0B11] via-[#0A0B11]/60 to-transparent`

**Scroll spacer:**
- A div with `height: 1800px; position: relative; z-index: 0` placed in the document flow after the hero section

**Video scrubbing logic (in App component useEffect with rAF loop):**
```
progress = Math.min(window.scrollY / 1800, 1)
video.currentTime = progress * video.duration
```
- Phase state machine:
  - scrollY === 0 -> 'idle' (reset currentTime to 0)
  - 0 < progress < 0.99 -> 'playing'
  - progress >= 0.99 -> 'done'
- Only sets currentTime when `!video.seeking`
- Uses a ref (`phaseRef`) to avoid unnecessary re-renders

---

### SECTION 3: CARD SECTION (fixed, z-4)

Component: `CardSection({ imagesVisible: boolean })`

A `position: fixed; inset: 0; width: 100%; height: 100%; z-index: 4` section.
- `transition-opacity duration-1000 ease-out`
- opacity: 1 when `imagesVisible` (videoPhase === 'done'), 0 otherwise
- pointerEvents: 'auto' when visible, 'none' when hidden

**Has its own independent mouse tracking + spotlight system** (same lerp technique as hero, factor 0.1, using local coordinates relative to section bounds).

**Layer 1: Base Card Image (z-1)**
- `absolute inset-0 bg-center bg-cover bg-no-repeat`
- backgroundImage: `CARD_IMAGE_1`

**Layer 2: Spotlight Reveal Card Image (z-3)**
- Hidden canvas + masked div, same technique as hero RevealLayer
- Local coordinates: `smooth.x - sectionRect.left`, `smooth.y - sectionRect.top`
- Same gradient stops (260px radius)
- backgroundImage: `CARD_IMAGE_2`

**Layer 3: Giant Background Text (z-0)**
- `absolute inset-0 flex items-center justify-center pointer-events-none select-none overflow-hidden`
- Text: "INFINITE"
- Styles: `font-medium tracking-[-0.05em] text-white/[0.04] whitespace-nowrap uppercase` with inline `fontSize: '26vw'`
- This text fills the full width of the viewport

**Content Overlays:**

**Top-left heading (z-5):**
- `relative pt-16 sm:pt-28 md:pt-32 px-5 sm:px-10 md:px-14`
- "Instantly Active" — `text-[clamp(2rem,7vw,5.5rem)] font-light text-white leading-[0.95] tracking-[-0.03em]`

**Top-right "+ More" button (z-10):**
- `absolute top-16 sm:top-28 md:top-32 right-5 sm:right-10 md:right-14`
- Button: `flex items-center gap-2 bg-white/10 backdrop-blur-sm border border-white/15 text-white/80 text-sm font-medium px-4 sm:px-5 py-2 sm:py-2.5 rounded-full hover:bg-white/15 transition-all`
- lucide `Plus` icon (size 15) + "More" text

**Bottom-right heading (z-5):**
- `absolute bottom-44 sm:bottom-40 md:bottom-44 right-5 sm:right-10 md:right-14`
- "Before You Swipe" — same typography as top-left heading, with `text-right`

**Bottom-left content (z-5):**
- `absolute bottom-6 sm:bottom-14 md:bottom-16 left-5 sm:left-10 md:left-14 right-5 sm:right-auto`
- Inner `max-w-md`:
  - Paragraph: "Get approved in seconds, receive your virtual card instantly, and arrive with cashback, travel perks, and spending insights already active." — `text-sm sm:text-[15px] md:text-base text-white/65 leading-relaxed mb-5 sm:mb-6`
  - Button "Apply now for free": `bg-white text-gray-900 text-sm font-medium px-5 sm:px-6 py-2.5 sm:py-3 rounded-full hover:bg-white/90 transition-all`

---

### KEY INTERACTION BEHAVIORS

1. **On load:** Hero is visible with staggered entrance animations (fadeSlideUp with incremental delays: 0.3s, 0.5s, 0.7s, 0.85s). Base image fades in (0.1s delay).

2. **Scrolling (0 to 1800px):** Video layer fades in (opacity 0 -> 1, 500ms transition). Video currentTime is scrubbed proportionally to scroll position. Hero content fades out simultaneously (500ms opacity transition to 0).

3. **Scroll reaches 99%+:** Video phase becomes "done". Card section fades in over 1000ms (ease-out). Video layer remains visible underneath.

4. **Spotlight reveal effect (both sections):** A 260px-radius radial spotlight follows the cursor with smooth lerp tracking (factor 0.1). The spotlight masks a second image layer, creating a reveal effect. The gradient feathers from full opacity at center to transparent at edges.

5. **Grid parallax (hero only):** The SVG grid pattern subtly shifts position based on cursor location (max offset ~16px, eased at 0.06 factor).

6. **Mobile menu:** Hamburger toggles with crossfade rotation. Menu slides down with scale-y transform, items stagger in with 40ms incremental delays.

---

### DOCUMENT STRUCTURE (render order in App)

```
<div min-h-screen bg-[#0A0B11]>
  <div fixed video layer z-2 />
  <nav fixed z-60 />
  <div mobile overlay z-54 />
  <div mobile menu z-55 />
  <HeroSection sticky z-3 />
  <div scroll-spacer 1800px z-0 />
  <CardSection fixed z-4 />
</div>
```

## FlowMate — Landing Page [sites/flowmate-landing]

- Preview: https://motionsites.ai/assets/hero-flowmate-preview-BmYI3ZvH.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/flowmate-landing.gif

Create a modern, production-ready landing page for "FlowMate" - an AI workflow automation platform. Use React, TypeScript, Vite, Tailwind CSS, Framer Motion, and Lucide React.

### Design System

**Colors:**
- Background: `#fefffc` (off-white)
- Text Primary: `#2c2c2c` (dark gray)
- Text Secondary: `#444141`
- Text Tertiary: `#646464`
- Text Muted: `#b4b8b4`
- Borders: `#dde3dd`, `#dee2de`, `#e8e8e8`
- Hover Background: `#eef1ed`
- Button Black: `black` with hover `#2c2c2c`

**Typography:**
- Custom Font: PPMondwest (serif) from URL: `https://www.generalintelligencecompany.com/_next/static/media/17330fd087386262-s.p.woff2`
- Font settings: `fontKerning: 'none'`, `letterSpacing: '-0.04em'`
- System fonts as fallback

### Layout Structure

**Desktop (1024px+):**
- Fixed sidebar: 240px width, left side
- Content area: margin-left 240px
- Fixed navbar: positioned at top of content area (left: 240px)

**Mobile/Tablet:**
- No sidebar
- Full-width navbar at top
- Stacked content

### Components

### 1. Sidebar (Desktop Only)
- Fixed position, 240px wide, full height
- Border: 2px solid `#dde3dd`
- Logo image at top: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_072635_e0ca60b6-0b6c-49a3-825d-b2b6a53dd63d.png&w=1280&q=85`
- Navigation items: Home, Video, Features, Cards
- Active state: `bg-[#eef1ed]` with `text-[#2c2c2c]`
- Inactive: `text-[#b4b8b4]` with hover effects
- Smooth scroll with IntersectionObserver tracking

### 2. Navbar
- Fixed at top, semi-transparent background: `bg-[#fefffc]/90` with `backdrop-blur-sm`
- Desktop positioning: `left-0 lg:left-[240px]`
- Logo/Brand: "FlowMate" in PPMondwest, 28px mobile, 32px desktop
- Right side items:
  - "Pricing" link (hidden on mobile)
  - "Community" link (hidden on mobile)
  - "Log in" button: white background, 2px border `#dde3dd`, rounded-full
  - "Sign up" button: black background, white text, rounded-full

### 3. Hero Section
- Padding: responsive (pt-12 to pt-20, pb-12 to pb-20)
- Heading: "Transform your workflow using plain English"
  - Font: PPMondwest
  - Size: 32px mobile → 50px tablet → 70px desktop
  - Line height: 0.95
  - Max width: 900px (700px on lg)
- Subheading: "FlowMate connects to your current apps, builds smart workflows, and manages operations. Powering the platforms you already know and trust."
  - Color: `#444141`
  - Max width: 620px (520px on lg)
- CTA button: "View our intro video" with custom arrow SVG icon
- Use TextFade animation (direction: up, stagger: 0.15s)

### 4. Video Section (Liquid Glass Effect)
**Video URL:** `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_073438_071156e5-2a7a-45d8-a8d9-c628d2144e88.mp4`

**Glass morphism overlay card:**
- Centered over video
- Typewriter effect text: "Daily check rival companies and ping me on messenger"
- Speed: 50ms per character
- Styling:
  ```css
  backdrop-filter: blur(16px)
  background-image: linear-gradient(in oklab, rgba(255, 255, 255, 0.35) 0px, rgba(255, 255, 255, 0.12) 100%)
  border: 6px solid rgba(255, 255, 255, 0.2)
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.5)
  ```
- Icons: Paperclip (lucide) and circular send button with ArrowUp
- Framer Motion spring animation on scroll

### 5. Features Grid
**Title:** "Discover what FlowMate can accomplish for your team"

**6 Feature Cards (3 columns on desktop, 2 on tablet, 1 on mobile):**

1. **Research this company (FlowMate)**
   - Description: "Execute investor-grade business analysis: generate detailed spreadsheets, gather web intel, compare rivals, and build team dossiers."
   - Icon: Generic tool icon

2. **Check the dev team's progress**
   - Description: "View a quick overview of your developer squad's activity, goals, and blockers."
   - Icons: Linear + Slack logos

3. **Build my CV from available information**
   - Description: "Generate a shareable PDF curriculum using stored facts and web sources, excluding any private contact info."
   - Icon: Generic tool icon

4. **Turn this into retro pixels**
   - Description: "Transform any photo into vintage pixelated graphics with custom resolution."
   - No icons

5. **Track Industry Sites and Send Weekly Digest Each Monday**
   - Description: "Watch leading tech and development sources for fresh content then deliver Monday briefings with main insights and URLs."
   - Icon: Generic tool icon

6. **Morning schedule digest**
   - Description: "Every AM, outline your agenda with important background and recommended preparation."
   - Icons: Gmail + Google Calendar logos

**Card Styling:**
- Border: 2px solid `#dee2de`
- Rounded: 2xl
- Hover: border color changes to `#b8beb8`
- Icons in circular gray backgrounds at bottom

### 6. Cards Carousel
**Auto-rotating carousel (4 second intervals) with 5 cards:**

**Card 1:** For Everyone
- Text: "Unleash your creative vision"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081328_19f48c5b-ea4d-4f23-8f80-7374f31015d4.png&w=1280&q=85`

**Card 2:** For Teams
- Text: "Smart helper supporting each teammate daily"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081342_ad378347-1ebd-4b17-a716-ee895bf739c0.png&w=1280&q=85`

**Card 3:** For Enterprises
- Text: "Elevate your whole organization using business AI"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081415_a6e8a76c-224e-417b-bf99-6b86d6494644.png&w=1280&q=85`

**Card 4:** Platform
- Text: "Enhanced with FlowMate"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081513_cf1cd2c1-2122-4de6-90ed-acae8bfbdb00.png&w=1280&q=85`

**Card 5:** Security
- Text: "Creating trusted and helpful AI"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_081541_9d2d28bf-d6a3-4b31-b0bb-cfc5202d4fcd.png&w=1280&q=85`

**Carousel Features:**
- Shows 3 cards at a time (1 on mobile)
- Manual navigation: Previous/Next buttons with ChevronLeft/Right icons
- Framer Motion AnimatePresence with slide animations
- Transition: `duration: 0.7, ease: [0.32, 0.72, 0, 1]`
- Cards: 500px height, gradient overlay, hover scale effect

### Animations

**TextFade Component:**
- Spring animation on scroll into view
- Stagger children with configurable delay
- Direction: up or down
- Default variants: `y: 18` offset, opacity fade

**Video Section:**
- Spring animation: `{ opacity: 1, y: 0 }` from `{ opacity: 0, y: 18 }`

**Carousel:**
- Entry: slide from right/left with scale 0.95
- Exit: slide opposite direction
- Smooth transitions with custom easing

### Technical Requirements

**Dependencies:**
- React 18.3+
- Framer Motion 12.38+
- Lucide React 0.344+
- Tailwind CSS 3.4+
- TypeScript 5.5+

**Build Setup:**
- Vite bundler
- ESLint configuration
- PostCSS with Autoprefixer

**Responsive Breakpoints:**
- Mobile: default
- Tablet: md (768px)
- Desktop: lg (1024px)

All sections have proper border separation (`border-t border-[#e8e8e8]`) and the entire page uses smooth scrolling behavior with section anchors.

## Focus AI — Landing Page [sites/focus-ai-landing]

- Preview: https://motionsites.ai/assets/hero-focus-ai-preview-Bnad3D1L.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/focus-ai-landing.gif

Build a Velorah landing page -- a premium, dark-themed single-page site for an electric RV/camper brand. Use React, TypeScript, Tailwind CSS, and the hls.js library. The page has 6 sections stacked vertically. The entire page background is pure black (hsl(0,0%,0%)). Use the font Instrument Serif (loaded from Google Fonts via <link> in index.html) for all headings and display text, and Inter for body text.

GLOBAL STYLES (index.css):

Import Google Fonts at the top:

@import
 url('https://fonts.googleapis.com/css2?family=Instrumental+Serif&family=Inter:wght@400;500&display=swap');
CSS custom properties (dark-only, no light mode):

--background: 201 100% 13%
--foreground: 0 0% 100% (white)
--card: 0 0% 6%
--card-foreground: 0 0% 100%
--primary: 0 0% 100%
--primary-foreground: 0 0% 4%
--secondary: 0 0% 10%
--secondary-foreground: 0 0% 100%
--muted: 0 0% 10%
--muted-foreground: 240 4% 66%
--accent: 0 0% 10%
--accent-foreground: 0 0% 100%
--destructive: 0 84.2% 60.2%
--destructive-foreground: 0 0% 100%
--border: 0 0% 18%
--input: 0 0% 18%
--ring: 0 0% 100%
--radius: 0.5rem
Body uses font-family: var(--font-body) which maps to Inter.

Liquid Glass CSS class (.liquid-glass):

background: rgba(255, 255, 255, 0.01) with background-blend-mode: luminosity
backdrop-filter: blur(4px) and -webkit-backdrop-filter: blur(4px)
border: none
box-shadow: inset 0 1px 1px rgba(255,255,255,0.1)
position: relative; overflow: hidden
::before pseudo-element creates a gradient border effect:
padding: 1.4px
background: linear-gradient(180deg, rgba(255,255,255,0.45) 0%, rgba(255,255,255,0.15) 20%, rgba(255,255,255,0) 40%, rgba(255,255,255,0) 60%, rgba(255,255,255,0.15) 80%, rgba(255,255,255,0.45) 100%)
Uses -webkit-mask with xor composite and mask-composite: exclude to create the border-only effect
Animations:

@keyframes
 fade-rise: from opacity:0; translateY(24px) to opacity:1; translateY(0)
.animate-fade-rise: animation: fade-rise 0.8s ease-out both
.animate-fade-rise-delay: same with 0.2s delay
.animate-fade-rise-delay-2: same with 0.4s delay
index.html:
Load Instrument Serif from Google Fonts via <link> tags:

<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet" />
HLS VIDEO COMPONENT:
Create an HlsVideo component that accepts a src prop. It uses hls.js -- if Hls.isSupported(), create an HLS instance, load the source, and attach to a <video> element. Otherwise fall back to native HLS if the browser supports application/vnd.apple.mpegurl. The video element has classes: absolute inset-0 w-full h-full object-cover z-0 and attributes: autoPlay loop muted playsInline.

VIDEO URLS (use these exact URLs):

Hero background: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_151826_c7218672-6e92-402c-9e45-f1e0f454bdc4.mp4
Feature section right card: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131748_f2ca2a28-fed7-44c8-b9a9-bd9acdd5ec31.mp4
Big Statement section (HLS stream): https://stream.mux.com/9njY8qDfS02Uvbll018C8CK39p5EksK7mn02DDC1zYvppI.m3u8
CTA/Join section: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260306_074215_04640ca7-042c-45d6-bb56-58b1e8a42489.mp4
SECTION 1 -- HERO:

Full-screen section (min-h-screen, relative, overflow-hidden)
Background: <video> tag (not HLS component) using Hero URL, with autoPlay loop muted playsInline, classes absolute inset-0 w-full h-full object-cover z-0
Bottom gradient overlay: absolute inset-x-0 bottom-0 h-[40%] bg-gradient-to-t from-black via-black/60 to-transparent z-[1]
Navbar (relative z-10, flex items-center justify-between, px-8 py-6, max-w-7xl mx-auto):
Left: Brand name "Velorah" with registered trademark superscript, text-foreground text-3xl tracking-tight, font-family 'Instrument Serif', serif
Center: Nav links (Home, Studio, About, Journal, Reach Us) -- hidden md:flex items-center gap-10 text-sm text-white. All links are text-white with hover:text-white/80 transition-colors
Right: "Begin Journey" button with liquid-glass rounded-full px-6 py-2.5 text-sm text-foreground transition-transform hover:scale-[1.03]
Hero content (relative z-10 flex flex-col items-center justify-center text-center px-6 pt-[28px] pb-40):
Heading: animate-fade-rise text-foreground text-5xl sm:text-7xl md:text-8xl leading-[0.95] tracking-[-2.46px] max-w-7xl font-normal, font-family 'Instrument Serif', serif. Text: Where dreams rise through the silence. -- the words "dreams" and "through the silence." are wrapped in <em className="not-italic text-white">
Paragraph: animate-fade-rise-delay text-white text-base sm:text-lg max-w-2xl mt-8 leading-relaxed. Text: "We're designing tools for deep thinkers, bold creators, and quiet rebels. Amid the chaos, we build digital spaces for sharp focus and inspired work."
Button: animate-fade-rise-delay-2 liquid-glass rounded-full px-14 py-5 text-base text-foreground mt-12 transition-transform hover:scale-[1.03] cursor-pointer. Text: "Begin Journey"
SECTION 2 -- TAGLINE:

flex items-center justify-center min-h-[70vh] px-6 bg-[hsl(0,0%,0%)]
Heading: text-foreground text-4xl sm:text-6xl md:text-7xl leading-[1.05] tracking-[-1.5px] text-center max-w-4xl, font-family 'Instrument Serif', serif. Text: "So you can feel at home,
anywhere."
SECTION 3 -- FEATURE SPLIT:

px-6 md:px-12 max-w-7xl mx-auto py-0
Grid: grid md:grid-cols-2 gap-4 rounded-2xl overflow-hidden min-h-[520px]
Left card (bg-card rounded-2xl p-10 md:p-14 flex flex-col justify-between):
Top: Small circle icon (inline-block w-8 h-8 rounded-full border border-border mb-8), heading "100% Electric" (text-foreground text-3xl sm:text-5xl tracking-[-1px] mb-6, Instrument Serif), paragraph "No more fossil fuels, buzzing generators, and propane tanks. Velorah has power for days." (text-muted-foreground text-sm sm:text-base leading-relaxed max-w-sm)
Bottom: Feature tabs array: [{label:"Living Electric",id:"electric"},{label:"Charge Faster",id:"charge"},{label:"Sleep Well",id:"sleep"},{label:"Acoustic Comfort",id:"acoustic"},{label:"5+ Seasons",id:"seasons"}]. Each tab is a <button> with text-xs px-4 py-2 rounded-full border transition-colors. Active state: bg-foreground text-primary-foreground border-foreground. Inactive: border-border text-muted-foreground hover:text-foreground. Use useState("electric") for active tab.
Progress bar: w-full h-0.5 bg-border rounded-full mb-6 with inner div h-full bg-foreground rounded-full at width: 35%
Button: liquid-glass rounded-full px-8 py-3 text-sm text-foreground transition-transform hover:scale-[1.03]. Text: "Explore the Velorah Flow"
Right card (relative rounded-2xl overflow-hidden min-h-[400px]): <video> using Feature section URL, absolute inset-0 w-full h-full object-cover, autoPlay loop muted playsInline
SECTION 4 -- BIG STATEMENT:

relative flex flex-col items-center justify-center min-h-[90vh] px-6 overflow-hidden
Background: <HlsVideo> component using the Mux HLS URL
Content (relative z-10 flex flex-col items-center text-center max-w-5xl):
Label: text-muted-foreground text-xs sm:text-sm tracking-[0.3em] uppercase mb-6. Text: "Intelligent Companion"
Heading: text-foreground text-4xl sm:text-6xl md:text-7xl leading-[1.05] tracking-[-1.5px], Instrument Serif. Text: "Adventure inspired.
App driven."
Paragraph: text-muted-foreground text-base sm:text-lg max-w-2xl mt-8 leading-relaxed. Text: "One app to control climate, lighting, navigation, and energy. Monitor every system in real time, automate your routines, and let Velorah learn how you live on the road."
Stats grid: grid grid-cols-2 sm:grid-cols-4 gap-8 sm:gap-12 mt-14. Four items (OTA / "Over-the-air updates", 360 degrees / "System visibility", AI / "Adaptive routines", 24/7 / "Remote monitoring"). Each stat value is text-foreground text-3xl sm:text-4xl font-light in Instrument Serif, label is text-muted-foreground text-xs sm:text-sm
Button: liquid-glass rounded-full px-10 py-4 text-sm text-foreground mt-12 transition-transform hover:scale-[1.03]. Text: "Discover the App"
SECTION 5 -- CTA / JOIN:

relative min-h-[90vh] flex flex-col items-center justify-center text-center px-6 overflow-hidden
Background: <video> using CTA URL, absolute inset-0 w-full h-full object-cover z-0, autoPlay loop muted playsInline
Content (relative z-10 flex flex-col items-center max-w-4xl):
Price label: text-muted-foreground text-xs sm:text-sm tracking-[0.3em] uppercase mb-4. Text: "Starting at $99,000"
Heading: text-foreground text-5xl sm:text-7xl md:text-8xl leading-[0.95] tracking-[-2px], Instrument Serif. Text: "Join the ride"
Paragraph: text-muted-foreground text-base sm:text-lg max-w-xl mt-6 leading-relaxed. Text: "Reserve your Velorah today with a fully refundable $500 deposit. Early adopters receive priority delivery and exclusive founding-member benefits."
Two buttons in a flex flex-col sm:flex-row items-center gap-4 mt-10:
"Preorder Now": liquid-glass rounded-full px-10 py-4 text-sm text-foreground transition-transform hover:scale-[1.03]
"Schedule a Tour": rounded-full px-10 py-4 text-sm text-muted-foreground border border-border hover:text-foreground hover:border-foreground/30 transition-colors
SECTION 6 -- FOOTER:

bg-[hsl(0,0%,0%)] border-t border-border px-6 md:px-12 py-16 max-w-7xl mx-auto
Grid: grid grid-cols-1 md:grid-cols-3 gap-12 mb-16
Col 1: Heading "Where home
meets the road." (text-foreground text-2xl sm:text-3xl leading-tight, Instrument Serif)
Col 2: Links list -- product, app, company, community, press, preorder. Each is text-sm text-muted-foreground hover:text-foreground transition-colors capitalize
Col 3: Text "Subscribe for the latest
Velorah updates." (text-sm text-muted-foreground mb-4) and a "Subscribe" button (liquid-glass rounded-full px-6 py-2.5 text-sm text-foreground transition-transform hover:scale-[1.03])
Bottom bar: flex flex-col md:flex-row items-center justify-between gap-4 pt-8 border-t border-border text-xs text-muted-foreground. Left: "Velorah" with registered trademark (text-foreground text-xl tracking-tight, Instrument Serif, <sup className="text-[8px]">). Right: "Privacy Policy" and "Terms & Conditions" links (hover:text-foreground transition-colors)
TAILWIND CONFIG: Standard shadcn/ui Tailwind config with all the HSL color variables mapped, darkMode: ["class"], tailwindcss-animate plugin, and accordion keyframes/animations.

DEPENDENCIES: React 18, react-router-dom, Tailwind CSS, shadcn/ui primitives, hls.js, lucide-react, @tanstack/react-query, tailwindcss-animate.

## Future-State — Landing Page [sites/future-state]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(20).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/future-state.webp

Build a dark, cinematic single-page landing site for an AI product called NOVA_AI using Vite + React + TypeScript + Tailwind CSS, with icons from lucide-react only. No other UI packages.

Global setup

- `index.html` title: `NOVA_AI — Today AI Aligns With Bold Dreams`. Load this font in `<head>`:

  `<link href="https://db.onlinewebfonts.com/c/4556933d6966c60eda45bebad34d9c90?family=Flexo+Soft+Medium" rel="stylesheet" />`

- `index.css`: Tailwind base/components/utilities; `body { font-family: 'Flexo Soft Medium', system-ui, sans-serif; background-color: #0a0a0a; color: #fff; -webkit-font-smoothing: antialiased; }`; `::selection { background-color: rgba(255,255,255,0.2); }`

- `tailwind.config.js`: extend `fontFamily` so BOTH `sans` and `mono` map to `['"Flexo Soft Medium"', 'system-ui', 'sans-serif']`.

- `App.tsx` renders, inside `<div className="relative">`: `<ScrollVideo />`, `<Navbar />`, then `<main>` containing `<SectionOne />`, a spacer `<div aria-hidden className="h-[80vh]" />`, and `<SectionTwo />`.

Component 1 — ScrollVideo (scroll-scrubbed video background)

Fixed full-viewport background (`fixed inset-0 -z-10 bg-[#0a0a0a]`) that scrubs a video based on total page scroll progress. Video URL (exact):

`https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260611_104107_121bfb5a-b1df-4e0d-8240-25b81f7cc85d.mp4`

Implementation details:

1. On mount, `fetch` the video as a blob, create an object URL, and pre-extract frames into `ImageBitmap[]`: create an off-DOM `<video>` (muted, playsInline, preload auto), wait for metadata, scale to max width 1280, frame count = `clamp(round(duration * 24), 30, 120)`, seek evenly across `duration - 0.05` and `createImageBitmap` each frame (with resize). Support cancellation; close bitmaps and revoke the object URL on unmount.

2. Render a `<canvas className="absolute inset-0 h-full w-full">`. While frames aren't ready, render a fallback `<video>` element (same URL, muted, playsInline, `object-cover`, absolute inset-0 full size) and scrub it by setting `currentTime` (guard with a `seeking` flag + `seeked` listener; only seek when delta > 0.001s).

3. Animation loop (`requestAnimationFrame`): target progress = `window.scrollY / (scrollHeight - innerHeight)` clamped 0–1, smoothed each tick via `smoothed += (target - smoothed) * 0.1`. Map smoothed progress to a frame index and only redraw when the index changes. Draw with "cover" math (scale = max of canvas/frame ratios, center the overflow). Canvas resolution = clientWidth/Height × devicePixelRatio capped at 2; re-size on window resize. Passive scroll listener.

4. Overlay `<div className="absolute inset-0 bg-black/20" />` for text contrast.

Component 2 — Reveal (staggered scroll animation wrapper)

Reusable component: props `children`, `delay?: number` (ms), `className?: string`, `as?: 'div' | 'span'` (default `'div'`). Uses an IntersectionObserver (threshold 0.15) and sets visible = `entry.isIntersecting` — i.e., animations REPLAY when elements leave and re-enter the viewport. Renders:

`transition-all duration-700 ease-out will-change-transform`, visible → `translate-y-0 opacity-100`, hidden → `translate-y-8 opacity-0`, plus `style={{ transitionDelay: `${delay}ms` }}` and the passed className. Disconnect observer on unmount.

Component 3 — Navbar (fixed corners)

- Top-left (`fixed left-5 top-5 z-50 sm:left-8 sm:top-7 md:left-12`): wordmark link `(NOVA_AI)` in `font-mono text-lg font-medium tracking-tight text-white drop-shadow-md sm:text-xl md:text-2xl` wrapped in `<Reveal>`; below it (delay 150) `[ v.01b ]` in `mt-6 font-mono text-[10px] text-white/60 sm:mt-8 sm:text-xs`.

- Top-right `<nav>` (`fixed right-5 top-5 z-50 sm:right-8 sm:top-7 md:right-12`): vertical right-aligned `<ul>` (`flex flex-col items-end gap-1.5 sm:gap-2`) with links `main`, `tiers`, `features`, `talk to us`. Each `<li>` wraps a `<Reveal delay={100 + i * 120}>` containing an anchor: `group flex items-center gap-1 font-mono text-xs text-white/80 drop-shadow-md transition-colors duration-300 hover:text-white sm:text-sm` plus an `ArrowUpRight` icon (size 14) that translates up-right on group hover (`group-hover:-translate-y-0.5 group-hover:translate-x-0.5`).

Component 4 — SectionOne (hero, bottom-anchored)

`<section className="relative flex min-h-screen flex-col justify-end supports-[height:100svh]:min-h-[100svh]">`. Content row: `relative flex flex-col gap-10 px-5 pb-16 sm:flex-row sm:items-end sm:justify-between sm:gap-8 sm:px-8 md:px-12 md:pb-20`.

- Left: `<h1 className="max-w-xl text-4xl font-medium uppercase leading-[1.05] tracking-tight text-white drop-shadow-lg sm:text-5xl md:text-6xl lg:text-7xl">` with four staggered lines, each a `<Reveal as="span" className="block ...">`:

  1. delay 100, `pl-6 sm:pl-12`: `Today AI`

  2. delay 220, no indent: `Aligns ` + `<span className="normal-case italic font-light">with</span>`

  3. delay 340, `pl-10 sm:pl-20`: `// Bold`

  4. delay 460, `pl-16 sm:pl-32`: `Dreams`

- Right column (`flex w-full max-w-xs flex-col items-start` — visible on ALL breakpoints, stacks under the headline on mobile):

  - Reveal delay 400: row `mb-6 flex w-full items-center justify-between font-mono text-white sm:mb-8` with `( A )` (text-lg) and `[ 001 /004 ]` (`text-xs text-white/70`).

  - Reveal delay 520: paragraph `mb-6 text-sm leading-relaxed text-white/85 drop-shadow-md sm:mb-8`: "NovaAI is where your bravest work finds its true expression. We hand you the means not only to form the future."

  - Reveal delay 640: full-width pill CTA `Begin Today`: `block w-full rounded-full border border-white/60 px-8 py-3 text-center font-mono text-xs uppercase tracking-[0.15em] text-white transition-all duration-300 hover:bg-white hover:text-black`.

- Absolute bottom-left (Reveal delay 760, `bottom-5 left-5 sm:bottom-6 sm:left-8 md:left-12`): `Share2` icon button (size 18, `text-white/80 hover:text-white`, aria-label "Share").

- Absolute bottom-center (Reveal delay 760, `bottom-5 left-1/2 -translate-x-1/2 sm:bottom-6`): `ArrowDown` size 18 with `animate-bounce text-white/80`.

Component 5 — SectionTwo

`<section className="relative flex min-h-screen flex-col supports-[height:100svh]:min-h-[100svh]">`.

- Middle row: `relative flex flex-1 flex-col justify-center gap-10 px-5 pt-24 sm:flex-row sm:items-center sm:justify-between sm:gap-8 sm:px-8 sm:pt-0 md:px-12`.

  - `<h2 className="max-w-sm text-4xl font-medium uppercase leading-[1.05] tracking-tight text-white drop-shadow-lg sm:text-5xl md:text-6xl">`, two Reveal lines: delay 100 `Learn ` + italic `<span className="normal-case italic font-light">to see</span>`; delay 220 `Brilliantly`.

  - Reveal delay 340: `flex items-center justify-between font-mono text-white sm:justify-start sm:gap-16 md:gap-24` with `( B )` (text-lg) and `[ 002 /004 ]` (`text-xs text-white/70`).

- Bottom block: `relative flex flex-col gap-10 px-5 pb-16 sm:px-8 md:px-12 md:pb-20`.

  - Reveal delay 460 paragraph (`max-w-xs text-sm leading-relaxed text-white/85 drop-shadow-md`): "Our AI doesn't just respond — it interprets, sharpens, and delivers. From outline to final render, it supplies the insight you want."

  - Reveal delay 580 CTA `Run The Demo`, in-flow full-width on mobile, absolutely bottom-centered on sm+: wrapper `w-full max-w-xs sm:absolute sm:bottom-16 sm:left-1/2 sm:w-auto sm:max-w-none sm:-translate-x-1/2 md:bottom-20`; anchor `block rounded-full border border-white/60 px-10 py-3 text-center font-mono text-xs uppercase tracking-[0.15em] text-white transition-all duration-300 hover:bg-white hover:text-black`.

- Reveal delay 700, absolute bottom-left (same classes as Section 1): `Share2` icon button.

Responsiveness rules

Mobile-first: hero and section content stack vertically (`flex-col`) and switch to side-by-side (`sm:flex-row`) at 640px; horizontal padding scales `px-5 → sm:px-8 → md:px-12`; headline sizes scale `text-4xl → sm:text-5xl → md:text-6xl (→ lg:text-7xl hero)`; headline indents shrink on mobile; Section 2 gets `pt-24` on mobile to clear the fixed nav. All text must remain readable over the video (drop shadows + black/20 overlay).

## Gateway Portal — Landing page [sites/gateway-portal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(76).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/gateway-portal.webp

Build a React + TypeScript (Vite) single-page hero called Auragate — an immersive, scroll-driven landing experience with a portal zoom-in effect, parallax mouse motion, and an arc-shaped testimonial card carousel. Use inline `style` objects (not CSS modules); Tailwind is only used for a few layout utility classes on one element. No UI libraries.

Fonts & `index.html`
In `index.html`, set `<title>Step Into Wonder</title>` and load these exact fonts in `<head>`:
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Viaoda+Libre&family=Imprima&family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet" />
<link href="https://db.onlinewebfonts.com/c/e2bba9cf49b298d6be781c2274694ea3?family=Mr+Dafoe+Regular" rel="stylesheet" />
<link href="https://db.onlinewebfonts.com/c/0976a2619014c5855690b7509fab4c6e?family=Helvetica+Now+Display" rel="stylesheet" />
```
Two font families are used throughout:
- `'Helvetica Now Display', sans-serif` — all UI text, headings, body, buttons.
- `'Mr Dafoe Regular', cursive` — decorative script accents ("Discover", "Aura" in the wordmark, the large "A.").

Global CSS (`index.css`)
Tailwind directives at top. Then: universal `box-sizing: border-box`; `html, body` with `margin/padding: 0`, `background: #0a0608`, `scroll-behavior: auto`, `overflow-x: clip`; `body` font-family `'Helvetica Now Display', 'Inter', sans-serif` with `-webkit-font-smoothing: antialiased`. Add `html { scrollbar-gutter: stable; }` to prevent scrollbar layout shift. Include a `@keyframes bobUp` (0/100% translateY(0), 50% translateY(-6px)).

Assets (use these exact URLs)
```js
const PORTAL_BG = 'https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781046673/image_1_ksxfzb.png';
const WORLD_BG  = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260609_231253_53c0854c-d13c-42c1-9fc0-17e87cd34091.png&w=1280&q=85';
```
`PORTAL_BG` is the foreground "portal" image you zoom into; `WORLD_BG` is the cloud/world background revealed behind it.

Layout structure (z-index stack on `#0a0608` root)
1. Fixed world background (`zIndex 0`): `position: fixed; inset: 0; overflow: hidden`. Inner `worldRef` div (`transformOrigin: 50% 50%`, `willChange: transform`) holding `WORLD_BG` `` at `width/height 100%`, `objectFit: cover`. Never scrolls.
2. **Fixed nav** (`zIndex 50`): see below.
3. **Intro track** (`zIndex 5`): `height: 160vh`, contains a `position: sticky; top: 0; height: 100vh; overflow: hidden` stage holding the portal + Scene 1 UI.
4. **Section 2** (`zIndex 10`): scrollable testimonial content + footer, rendered over the fixed world bg.

### Scroll & motion engine
- `useIsMobile()` hook: matchMedia `(max-width: 767px)`, initial `window.innerWidth < 768`.
- Track scroll progress 0→1 across the pinned intro: `progress = clamp(window.scrollY / (introRef.offsetHeight - innerHeight), 0, 1)`. Store in both state and a ref. Listen on `scroll` (passive) and `resize`.
- Helper math: `easeInOut(t) = t<0.5 ? 2t² : -1+(4-2t)t`, `lerp(a,b,t)`, `clamp`.
- **Mouse parallax** via `requestAnimationFrame`: read raw mouse as `(clientX/innerWidth - 0.5)2` (same for Y). Smooth toward it with `lerp(..., 0.07)` each frame. Invert (`rx=-mx, ry=-my`). Magnitudes `MAG = { world: 6, portal: 7 }`.
- Each frame, with `ep = easeInOut(scrollProgress)`:
  - World: `scale = lerp(1, 1.18, ep)`, `transform: scale(s) translate(rx6px, ry6px)`.
  - Portal: `scale = lerp(1, 7.5, ep)`, `transform: scale(s) translate(rx7px, ry*7px)`, `transformOrigin: '52% 38%'`.
- **Opacity transitions** driven by scrollProgress:
  - `portalOpacity` = 1 until 0.66, then fades to 0 over the next 0.22 of scroll.
  - `scene1Opacity` = `clamp(1 - scrollProgress/0.22, 0, 1)` (Scene 1 UI fades out in the first 22% of scroll). When `< 0.05`, set `pointerEvents: none`.
- `uiVisible` state flips true 600ms after mount; Scene 1 UI fades/slides up (`translateY(24px)→0`, `opacity 0→1`, `transition: opacity 1s ease, transform 1s ease`, `transitionDelay: 0.3s`).

### Navigation (fixed, top)
Flex row, space-between, padding `26px 40px` desktop / `18px 20px` mobile.
- **Left:** `` — inline-flex baseline-aligned: "Aura" in Mr Dafoe (36px / 30px mobile) + "gate" in Helvetica Now Display weight 500 (24px / 20px), `letterSpacing: -0.02em`, white.
- **Right:** a pill **"Watch Demo"** button (white bg, `#161616` text, weight 600, `borderRadius: 999px`, padding `11px 22px` / `9px 16px`, hover bg `#e6e6e6`) immediately followed by a round 42px (38px mobile) white hamburger button containing an SVG of two rounded horizontal lines (`stroke #161616`, width 1.6). Both transition bg `0.25s`.

### Scene 1 hero copy (bottom-anchored, fades out on scroll)
A bottom-aligned container with Tailwind classes `absolute inset-x-0 bottom-0 flex flex-col md:flex-row md:items-end md:justify-between gap-12 md:gap-20`, padding `0 44px 52px` (`0 22px 40px` mobile).
- **Column 1** (`maxWidth 560px`, left aligned): `

` Helvetica Now Display weight 500, white, `lineHeight 1.04`, `letterSpacing -0.02em`, `fontSize clamp(40px,4vw,58px)` (mobile `clamp(30px,9vw,44px)`). The word **"Discover"** is a leading span in Mr Dafoe, color `#9a9a9a`, `fontSize 1.15em`, `marginRight 0.12em`. Full text:
  > *Discover* Living \
 Digital Worlds \
 Vivid, Alive, Endless

  Below it a `

` (weight 400, `13–14px`, `lineHeight 1.6`, `color rgba(255,255,255,0.5)`, `maxWidth 340px`, `marginTop 18px`): "Experience immersive worlds with stories that blur the line between imagination, AI and living reality made for you."
- **Column 2** (hidden on mobile, flex, `gap 14px`): a large "A." in Mr Dafoe (64px, white, `lineHeight 0.8`) next to a `

` (weight 400, 11px, `color rgba(255,255,255,0.5)`, `maxWidth 150px`): "A studio is a trusted partner in your journey through wonderland."

### Section 2 — "Real wonders" + arc carousel
Centered column, `paddingTop 14vh` (12vh mobile), `paddingBottom 60px`.
- Centered `

` (weight 500, `clamp(34px,4vw,52px)`, white, `letterSpacing -0.02em`, `lineHeight 1.1`, `textShadow 0 2px 20px rgba(0,0,0,0.35)`): "Real wonders.\
Real worlds."
- Centered `

` (weight 500, 17px, `maxWidth 420px`, white, `textShadow 0 2px 16px rgba(0,0,0,0.3)`, `marginTop 16px`): "See how Auragate helps others, and find out what it can do for you."
- Below: `` with these 7 testimonial quotes (in order):
  1. "It is amazing to see and feel the worlds I am stepping into each day."
  2. "I have been feeling much more alive inside these living worlds, even on the long days."
  3. "My wonder has been growing so fast that it is hard to believe the difference. Auragate gave me exactly the vision I needed."
  4. "The first two scenes felt alive. I tried everything we dreamed up and it worked."
  5. "The wonder of it all really moved me, it even brought a tear to my eyes every time."
  6. "I finally feel immersed, like the worlds were built just for me."
  7. "Stepping into it was effortless and the worlds have been unlike anything I dreamt."

### ArcCardCarousel (the signature component)
A fanned arc of cards centered on screen. State `active` starts at `floor(total/2)`. Constants (desktop / mobile): `cardW 300/230`, `cardH 420/320`, `stepX 295/170`, `dropY 52/34`, `tilt 8/7`, `containerH 560/460`. Container `position: relative; width: 100%; height: containerH`.

For each card compute signed position `pos` relative to `active` wrapped into `[-half, +half]`; `abs = |pos|`; `isCenter = pos===0`.
- **Transform:** `translateX(posstepX) translateY(absdropY + (isCenter ? 30 : 0)) rotate(postilt deg)` — this creates the downward-curving arc with outer cards dropped and rotated. (mobile center bump = 22.)
- `opacity`: center `1`, else `max(0, 0.6 - (abs-1)0.2)`. `zIndex: 100 - abs`. `pointerEvents: isCenter ? 'auto' : 'none'`. `transition: transform 0.55s cubic-bezier(0.22,1,0.36,1), opacity 0.55s ease`.
- **Card face:** `borderRadius 28px` (22 mobile).
  - **Center card:** solid `background: rgb(247,251,255)`, `border 1px solid rgba(255,255,255,0.6)`, no backdrop filter, and `boxShadow: '0 8px 24px rgba(0,0,0,0.08), 0 0 50px rgba(255,255,255,0.55), 0 0 90px rgba(255,255,255,0.35)'` (soft dark + layered white glow). Quote text color `#2c2420`.
  - **Inactive cards:** frosted glass — `background: linear-gradient(135deg, rgba(255,255,255,0.42) 0%, rgba(255,255,255,0.24) 100%)`, `backdropFilter: blur(18px) saturate(140%)` (+ `-webkit-`), `border 1px solid rgba(255,255,255,0.28)`, `boxShadow: 'inset 0 1px 1px rgba(255,255,255,0.45)'` (inset highlight only, no drop shadow). Quote text `rgba(255,255,255,0.85)`.
  - Quote `

`: Helvetica Now Display weight 500, 17px (15 mobile), `lineHeight 1.5`, `letterSpacing -0.01em`, wrapped in typographic quotes `“…”`, centered, card uses flex center.
- **Nav buttons** (absolute, `bottom: -40px`, centered, `gap 10px`): two round 46px (42px mobile) buttons. Prev (`dir -1`): `background rgba(255,255,255,0.2)`, no shadow, white chevron-left SVG with a glowing `drop-shadow` filter (`rgba(255,255,255,0.7) 0 0 6px` + `rgba(255,255,255,0.4) 0 0 14px`). Next (`dir 1`): `background rgba(255,255,255,0.9)`, `boxShadow 0 6px 18px rgba(0,0,0,0.18)`, dark `#2c2420` chevron-right. Clicking advances `active` by `±1` with modulo wrap.

### Footer
`position: relative`, padding `160px 44px 52px` (`120px 22px 40px` mobile). CSS grid: desktop `1.4fr 1fr 1fr 1fr`, mobile `1fr 1fr` (`gap 40px` / `32px 20px`), `maxWidth 1280px`, centered. First cell (full-width on mobile) = `` + a `rgba(255,255,255,0.55)` 12px line "© 2026 Auragate". Then three columns:
- **Explore:** How it works, Features
- **Contact:** X (Twitter), hello@auragate.com
- **Legal:** Privacy Policy, Terms of Service

Each column: a `rgba(255,255,255,0.55)` weight-500 13px title (`marginBottom 18px`), then a `gap 12px` list of white weight-500 14px links (`textDecoration none`, hover `opacity 0.65`).

### Responsiveness
Everything keys off the `isMobile` boolean (767px breakpoint): reduced paddings, font clamps, smaller carousel constants, Column 2 of the hero hidden, footer grid collapses to 2 columns with the brand cell spanning full width.

Behavior summary
On load, the user sees the portal image full-screen with the hero copy at the bottom. Mouse movement gently parallaxes both layers in opposite directions. Scrolling through the 160vh pinned track zooms the portal in to 7.5× while the world background scales to 1.18×; the hero copy fades out first (by 22%), then the portal fades out (66%→88%), revealing the cloud world and the "Real wonders" testimonial carousel section, ending in the footer.

## Glitch Pulse — Landing Page [sites/glitch-pulse]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(29).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/glitch-pulse.webp

### Overview

Build a single-page React + TypeScript + Vite + Tailwind CSS v4 landing page. The page is extremely tall (1200vh) but all visual content is position-fixed to the viewport. Scrolling drives two scroll-scrubbed HLS videos, a typewriter text deletion effect, a vertically-scrolling manifesto, mouse-trail stickers, and a scroll-triggered feedback form. The aesthetic is brutalist/retro-digital with a pixel font, lime green on near-black with electric blue accents.

---

### TECH STACK (exact versions)

```json
{
  "dependencies": {
    "@tailwindcss/vite": "^4.1.14",
    "@vitejs/plugin-react": "^5.0.4",
    "express": "^4.21.2",
    "hls.js": "^1.6.16",
    "react": "^19.0.1",
    "react-dom": "^19.0.1",
    "vite": "^6.2.3"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/node": "^22.14.0",
    "esbuild": "^0.25.0",
    "tailwindcss": "^4.1.14",
    "tsx": "^4.21.0",
    "typescript": "~5.8.2"
  }
}
```

---

### ALL ASSET URLs

### HLS Videos (Mux streams):
```
VIDEO_URL_1 = "https://stream.mux.com/W2NRcV6MrewS7QyWWqAWZvJR9jrnPU5rxymlPg01gRzk.m3u8"
VIDEO_URL_2 = "https://stream.mux.com/aypDi1exkKgYKEbWme9Csi47zxIim0101hw3ghmSzQIyw.m3u8"
```

### Static Stickers (Figma CDN):
```
STICKER1 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/d9a6de619b1e7bf4b31b22e6d29324306ee68ad9.d9a6de61.png"
STICKER2 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/7d1d8f4421fc4780ec85b4153ca6605a4b90dd65.7d1d8f44.png"
STICKER3 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/80809d23ccb460d0db21f77bb3afef67d3ad1d9a.80809d23.png"
STICKER4 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/50d6c27f67bc10d6859cf37d2f017bc406ad3a0d.50d6c27f.png"
```

### Mouse Trail Stickers (Figma CDN):
```
TRAIL_STICKER1 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/b77ef81dabfca9ce4a4d1af5d553e17019a0d229.b77ef81d.png"
TRAIL_STICKER2 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/9ece3a6bf6c5cecf6c0078d022a171bc93baf9c5.9ece3a6b.png"
TRAIL_STICKER3 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/41b9f0bffb2c0b2e1d3fbe26c124ed1378970c35.41b9f0bf.png"
TRAIL_STICKER4 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/0edc0785a3e3bf26be7a494886999c4a6f1dc14c.0edc0785.png"
TRAIL_STICKER5 = "https://crow-peanut-06457083.figma.site/_components/v2/4c2b061456bbff22b92923348791b501874ded3f/d12ddf42fe4c8437df4414c883fe60fb77b20cbe.d12ddf42.png"
```

---

### FONT

Google Font: **"Press Start 2P"** -- imported via CSS `@import url(...)`. Registered as a Tailwind v4 theme token:

```css
@theme {
  --font-press-start: "Press Start 2P", system-ui, sans-serif;
}
```

Applied everywhere via class `font-press-start`.

---

### COLOR SYSTEM

| Token | Value | Usage |
|-------|-------|-------|
| Background | `slate-950` | Page bg, form bg |
| Primary text/accent | `#85D743` | All headings, buttons, cursor, manifesto text |
| Secondary accent | `#0033FF` | Marquee bg, form border, button shadows |
| Button hover | `#9eff5c` / `#9bfb4e` | Submit/reset button hover states |
| Form bg | `slate-950/95` | 95% opacity dark overlay |
| Input bg | `slate-900` | Form input fields |
| Input border | `slate-700` | Default border, `#85D743` on focus |
| Nav text | white | Navigation links |
| Selection | `emerald-500/20` | Text selection highlight |

---

### HTML SHELL (`index.html`)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Scroll Video</title>
  </head>
  <body class="bg-slate-950 overflow-x-hidden m-0 p-0 selection:bg-emerald-500/20">
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

---

### CSS (`src/index.css`) -- EXACT

```css
@import url('https://fonts.googleapis.com/css2?family=Press+Start+2P&display=swap');
@import "tailwindcss";

@theme {
  --font-press-start: "Press Start 2P", system-ui, sans-serif;
}

@keyframes marquee {
  0% { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(-50%, 0, 0); }
}

@keyframes sticker-fade-out {
  0% {
    opacity: 0;
    transform: translate(-50%, -50%) rotate(var(--rot)) scale(0);
  }
  20% {
    opacity: 1;
    transform: translate(-50%, -50%) rotate(var(--rot)) scale(1.05);
  }
  30% {
    opacity: 1;
    transform: translate(-50%, -50%) rotate(var(--rot)) scale(1.0);
  }
  85% {
    opacity: 1;
    transform: translate(-50%, -50%) rotate(var(--rot)) scale(1.0);
  }
  100% {
    opacity: 0;
    transform: translate(-50%, -50%) rotate(var(--rot)) scale(0.8);
  }
}

.animate-marquee {
  display: flex;
  width: max-content;
  animation: marquee 10s linear infinite;
}
```

---

### PAGE LAYOUT

Root container: `relative w-full bg-slate-950 min-h-[1200vh] select-none`

All visual content is `position: fixed`. The 1200vh height exists solely to create scroll distance that drives the animations.

---

### ELEMENT-BY-ELEMENT BREAKDOWN

### 1. FULL-SCREEN VIDEO BACKGROUND (fixed, z-auto)

Container: `fixed inset-0 w-full h-full overflow-hidden`

Two `<video>` elements stacked:
- Both: `absolute inset-0 w-full h-full object-cover pointer-events-none transition-opacity duration-300`
- Both: `muted playsInline autoPlay={false} preload="auto" crossOrigin="anonymous"`
- Video 1 starts at `opacity: 1`, Video 2 starts at `opacity: 0`

**HLS Setup:**
- Use `hls.js` with `{ maxBufferLength: 60 }`
- On `MANIFEST_PARSED`: call `video.play().then(() => video.pause())` to warm decoder
- Fallback: native HLS for Safari via `canPlayType("application/vnd.apple.mpegurl")`

**Scroll-Scrub Logic (requestAnimationFrame lerp loop):**
- Calculate `progress = scrollY / (documentHeight - windowHeight)` [0 to 1]
- Video 1 maps progress [0, 0.5] to its full duration
- Video 2 maps progress [0.5, 1.0] to its full duration
- Lerp factor: 0.3 (smooth interpolation per frame)
- Guard: Only update `video.currentTime` when `!video.seeking`
- Snap immediately at scroll boundaries (scrollY <= 10 for v1, at bottom for v2)

**Crossfade (no dim):**
- Progress 0-0.45: v1 opacity 1, v2 opacity 0
- Progress 0.45-0.5: v1 stays at opacity 1, v2 fades from 0 to 1 (overlaid on top)
- Progress 0.5+: v1 opacity 0 (GPU savings), v2 opacity 1

---

### 2. DIAGONAL MARQUEE BANNER (fixed, z-50)

Container: `fixed top-14 left-[-170px] w-[650px] -rotate-[30deg] z-50 bg-[#0033FF] py-[18px] overflow-hidden select-none pointer-events-none shadow-2xl`

Inner: `.animate-marquee flex whitespace-nowrap`

Two identical `<span>` elements for seamless loop:
```
font-press-start text-[16px] text-[#85D743] tracking-widest px-4
```
Content: `WARNING! WARNING! WARNING! WARNING! WARNING! WARNING! WARNING! WARNING!&nbsp;` (repeated twice)

Animation: `marquee 10s linear infinite` (translates X from 0 to -50%)

---

### 3. FLOATING STICKER - TOP LEFT (fixed, z-40)

Container: `fixed z-40 select-none pointer-events-none transition-transform duration-300`
- Inline style: `top: 232px`, `left: 120px`, `transform: rotate(32deg)`
- Image: STICKER1, `w-[100px] h-auto object-contain`, `referrerPolicy="no-referrer"`

---

### 4. FLOATING STICKER - BOTTOM RIGHT (fixed, z-40)

Container: `fixed bottom-8 right-8 z-40 select-none pointer-events-none transition-transform duration-300`
- Image: STICKER2, `w-[110px] sm:w-[150px] h-auto object-contain`, `referrerPolicy="no-referrer"`

---

### 5. NAVIGATION (fixed, z-50)

```
fixed top-10 right-8 md:right-16 z-50 flex gap-6 md:gap-10 font-press-start text-[10px] sm:text-xs md:text-sm text-white select-auto
```

Links: Projects, Expertise, About, Contact
- Each: `hover:text-[#85D743] transition-colors duration-200`
- Contact link: `onClick` prevents default, calls `window.scrollTo({ top: documentElement.scrollHeight, behavior: "smooth" })`

---

### 6. HERO TEXT - TYPEWRITER DELETION (fixed, z-30)

Container: `fixed top-0 left-0 w-full h-screen z-30 pointer-events-none flex flex-col justify-end p-8 md:p-16 pb-16 sm:pb-24`

H1: `font-press-start text-[#85D743] text-3xl sm:text-5xl md:text-6xl lg:text-7xl xl:text-8xl leading-[1.2] tracking-tight uppercase select-none max-w-5xl`

**Three lines:**

**Line 1: "PROBLEM"** -- 7 individual `<span className="inline-block relative">` characters (indices 1-7)

**Line 2: "WITH" + 2 stickers** -- wrapped in `inline-flex items-center gap-3 sm:gap-5 md:gap-7 align-middle`
- Letters W-I-T-H in nested `inline-flex items-center gap-[0.06em] sm:gap-[0.08em] md:gap-[0.1em]` (indices 8-11)
- STICKER4 image at index 12: `h-[0.85em] w-auto object-contain`, rotated `-8deg`
- STICKER3 image at index 13: `h-[0.85em] w-auto object-contain`, rotated `6deg`

**Line 3: "CREATIVE?"** -- 9 characters (indices 14-22)

**Total character count: 22**

**Cursor:** After the last visible character: `inline-block w-[0.14em] h-[0.8em] bg-[#85D743] ml-1 select-none animate-pulse align-middle`

**Deletion logic:**
- During scroll progress 0 to 0.25, characters are removed right-to-left
- `visibleCount = Math.round((1 - activeProgress) * 22)` where `activeProgress = min(progress, 0.25) / 0.25`

---

### 7. MANIFESTO - ROLLING CREDITS (fixed, z-25)

Container: `fixed top-0 left-0 w-full md:w-[70%] h-screen z-25 pointer-events-none flex flex-col justify-start p-8 md:p-16 pt-[12vh] pb-16 select-none`
- Initial inline style: `opacity: 0`, `transform: translateY(100vh)`

Text div: `font-press-start text-[#85D743] text-[20px] sm:text-[26px] md:text-[32px] leading-[1.35] tracking-tight uppercase text-left whitespace-pre-line select-none`

**Scroll behavior:**
- Begins after progress 0.25 + (200px / maxScroll) delay
- `alpha = (progress - startProgress) / (1 - startProgress)` [0 to 1]
- `opacity = min(1, alpha / 0.05)` (fast fade-in)
- `translateY = 100 - (alpha * 450)` in vh (scrolls from +100vh to -350vh)

**Exact manifesto text:**
```
LIMITLESS INPUTS OR
STANDARD LAYOUT
TEMPLATE
CONSUMERS. THIS IS
A HIGHLY SELECTIVE
ENVIRONMENT
ENGINEERED FOR
HYPER-PRODUCTIVE
CREATORS, UI/UX
VISIONARIES, AND
AI PROMPT
ARCHITECTS WHO
OPERATE AT THE
ABSOLUTE LIMITS OF
DIGITAL PRODUCT
CREATION. OUR
FRAMEWORK IS

---

WHY CHOOSE US?
1. ZERO MOCKUPS,
   ONLY REAL CODE.
2. SPEED RUNS -
   ZERO WASTED TIME.
3. DIGITAL EDGE -
   AESTHETIC
   DOMINANCE.
4. SYSTEM STATE -
   INTELLIGENT
   INTERACTION.

---

WE REJECT
THE BORING.
WE REJECT
THE STANDARD.

CHOOSE ABSOLUTE
CREATIVE EDGE.

THE FUTURE IS
NOW SECURED.
```

---

### 8. MOUSE TRAIL STICKERS (fixed, z-[60])

Container: `fixed inset-0 pointer-events-none z-[60] overflow-hidden`

**Behavior:**
- Track mouse position via `window.addEventListener("mousemove", ..., { passive: true })`
- Spawn new sticker when cursor moves > 150px from last spawn point
- 5 sticker types cycle sequentially (not randomly): trail_sticker1 through trail_sticker5
- Random rotation: `Math.random() * 40 - 20` degrees
- Max 4 stickers visible: `[...prev.slice(-3), newSticker]`
- Auto-remove after 2200ms via `setTimeout`

**Each sticker element:**
- `absolute select-none pointer-events-none flex items-center justify-center -translate-x-1/2 -translate-y-1/2`
- Inline style: `left: x`, `top: y`, `transform: translate(-50%, -50%) rotate(Xdeg)`, `--rot: Xdeg`
- `animation: sticker-fade-out 2.2s forwards cubic-bezier(0.16, 1, 0.3, 1)`
- Images: `w-[110px] sm:w-[150px] md:w-[180px] h-auto object-contain`, `referrerPolicy="no-referrer"`

---

### 9. FEEDBACK FORM (fixed, z-[55])

Container: `fixed left-1/2 z-[55] w-[92%] max-w-[460px] p-6 sm:p-8 bg-slate-950/95 border-4 border-[#0033FF] shadow-[10px_10px_0px_#85D743] select-auto transition-all duration-[900ms] pointer-events-auto`

**Position/Animation:**
- `bottom: 50%`
- When visible: `transform: translate(-50%, 50%) rotate(0deg)`
- When hidden: `transform: translate(-50%, 150vh) rotate(15deg)`
- `transitionTimingFunction: cubic-bezier(0.16, 1, 0.3, 1)`

**Triggers at:** scroll progress >= 0.95

**Close button:** `absolute top-4 right-4 font-press-start text-[14px] text-slate-500 hover:text-red-500 hover:scale-110 active:scale-95 transition-all cursor-pointer select-none border-none bg-transparent` -- content: `[X]`

**Form (default state):**
- Title: "FEEDBACK SYSTEM" -- `font-press-start text-xs sm:text-sm text-[#85D743] tracking-widest uppercase text-center mb-1`
- Name input: `type="text" required placeholder="YOUR NAME"` -- `font-mono text-xs text-white bg-slate-900 border-2 border-slate-700 focus:border-[#85D743] hover:border-slate-500 focus:outline-none p-2.5 w-full uppercase transition-all placeholder-slate-600`
- Email input: `type="email" required placeholder="NAME@DOMAIN.COM"` -- same styles minus `uppercase`
- Textarea: `required rows={3} placeholder="FEEDBACK / ARCHITECTURE IDEAS..."` -- same styles plus `resize-none`
- Submit button: "LAUNCH TRANSMISSION" -- `font-press-start text-[8px] sm:text-[9px] text-black bg-[#85D743] hover:bg-[#9bfb4e] active:translate-y-0.5 active:shadow-none border-2 border-black py-3 px-6 shadow-[4px_4px_0px_#0033FF] w-full font-bold uppercase tracking-widest cursor-pointer select-none transition-all mt-1`
- Form gap: `flex flex-col gap-4 sm:gap-5`

**Success state:**
- Symbol: `font-press-start text-[32px] text-[#85D743] mb-4 animate-bounce` -- content: `✦`
- Heading: "TRANSMISSION SUCCESS" -- `font-press-start text-xs sm:text-sm text-[#85D743] mb-3 tracking-widest uppercase`
- Subtext: "Your feedback is secured in our neural network database." -- `font-mono text-[10px] sm:text-xs text-slate-400 max-w-sm mb-6 uppercase leading-relaxed`
- Reset button: "[ NEW TRANSMISSION ]" -- `font-press-start text-[8px] sm:text-[10px] text-black bg-[#85D743] hover:bg-[#9eff5c] border-2 border-black py-2.5 px-5 font-bold uppercase tracking-wider shadow-[3px_3px_0px_#0033FF] active:translate-y-0.5 active:shadow-none transition-all cursor-pointer`

---

### SERVER (`server.ts`)

Express server on port 3000. In development: Vite middleware mode (SPA). In production: serves static `dist/` directory.

```typescript
import express from "express";
import path from "path";
import fs from "fs";
import { createServer as createViteServer } from "vite";

async function startServer() {
  const app = express();
  const PORT = 3000;
  const publicDir = path.join(process.cwd(), "public");
  if (!fs.existsSync(publicDir)) fs.mkdirSync(publicDir, { recursive: true });

  if (process.env.NODE_ENV !== "production") {
    const vite = await createViteServer({ server: { middlewareMode: true }, appType: "spa" });
    app.use(vite.middlewares);
  } else {
    const distPath = path.join(process.cwd(), "dist");
    app.use(express.static(distPath));
    app.get("*", (req, res) => res.sendFile(path.join(distPath, "index.html")));
  }

  app.listen(PORT, "0.0.0.0", () => console.log(`Server running on http://localhost:${PORT}`));
}
startServer();
```

---

### VITE CONFIG (`vite.config.ts`)

```typescript
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { defineConfig } from 'vite';

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(__dirname, '.') } },
}));
```

---

### CRITICAL IMPLEMENTATION DETAILS

1. **No auto-play videos** -- videos are paused immediately after a play() call to warm up the decoder. Scroll scrubbing sets `currentTime` directly.
2. **Seeking guard** -- `if (!video.seeking)` prevents setting new frames while browser is still rendering the previous seek.
3. **No dim on crossfade** -- Video 1 stays at full opacity while video 2 fades in on top. Only after transition completes does video 1 go to opacity 0.
4. **All images use `referrerPolicy="no-referrer"`** to avoid Figma CDN blocking.
5. **State updates use equality check** -- `setX((prev) => prev !== newVal ? newVal : prev)` to avoid unnecessary re-renders during scroll.
6. **The manifesto translates a total of 450vh** (from +100vh to -350vh) to ensure it fully scrolls off screen.
7. **Mouse trail uses sequential cycling** (`typeCounter % 5`), not random selection.

## Golden Portal — Landing Page [sites/golden-portal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(13).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/golden-portal.webp

**Build a React + Vite + TypeScript + Tailwind CSS landing page called "Digital Archive" -- a high-end art gallery/studio showcase website. It should be dark, cinematic, and editorial in style. Use ONLY `lucide-react` for icons. No other UI libraries.**

---

### FONTS

Load two fonts in `index.html`:
1. **Arsenica Trial Light** (serif) from: `https://db.onlinewebfonts.com/c/cbb3cb559d2e4387e139cfb1656e31f5?family=Arsenica+Trial+Light`
2. **Inter** (sans-serif, weights 300, 400, 500, 600) from Google Fonts: `https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap`

In CSS, define:
```css
:root {
  --font-serif: 'Arsenica Trial Light', serif;
  --font-sans: 'Inter', sans-serif;
}
body { font-family: var(--font-sans); }
.font-arsenica { font-family: var(--font-serif); }
.font-inter { font-family: var(--font-sans); }
```

---

### GLOBAL CSS (index.css)

Include these exact custom styles:

**Liquid Glass Effect:**
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

**Hero Fade-Up Animation:**
```css
@keyframes hero-fade-up {
  from { opacity: 0; transform: translateY(24px); }
  to { opacity: 1; transform: translateY(0); }
}
.hero-fade-up {
  opacity: 0;
  animation: hero-fade-up 0.9s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
```

**Scroll Reveal Animations:**
```css
@keyframes reveal-up {
  from { opacity: 0; transform: translateY(32px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes reveal-scale {
  from { opacity: 0; transform: scale(0.96); }
  to { opacity: 1; transform: scale(1); }
}
.reveal { opacity: 0; }
.reveal.revealed {
  animation: reveal-up 0.8s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
.reveal-scale { opacity: 0; }
.reveal-scale.revealed {
  animation: reveal-scale 1s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
```

---

### CUSTOM HOOK: `useScrollReveal`

A reusable IntersectionObserver hook that finds all `.reveal` and `.reveal-scale` elements within a ref, and adds `.revealed` class when they enter view (threshold default 0.15, rootMargin `0px 0px -40px 0px`). Once revealed, unobserve.

---

### SECTION 1: NAVBAR (fixed, centered, floating)

- Fixed position, centered horizontally (`left-1/2 -translate-x-1/2`), top-4 on mobile, top-6 on sm+
- Uses the `.liquid-glass` class with `rounded-full`
- Padding: `px-4 py-2.5` mobile, `px-10 py-3` on sm+
- 5 items in a row with gap-4 mobile, gap-12 on sm+:
  - Text link: "Gallery"
  - Text link: "Talents"
  - Center: Custom SVG logo (a geometric angular shape, white fill, `h-5 w-5` mobile / `h-7 w-7` sm+, with hover:scale-110 transition)
  - Text link: "Journal"
  - Text link: "Story"
- Link styles: `text-[10px]` mobile / `text-xs` sm+, uppercase, `font-medium`, `tracking-[0.15em]` mobile / `tracking-[0.2em]` sm+, `text-white/85`, hover `text-white`

**Logo SVG path:**
```
M 64 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 L 128 64 L 128 64.5 L 161 32 L 192 0 L 256 0 L 256 64 L 192 128 L 128 128 L 128 192 L 96 223 L 63.5 256 L 0 256 L 0 192 Z M 256 192 L 224 223 L 191.5 256 L 128 256 L 128 192 L 192 128 L 256 128 Z
```
viewBox `0 0 256 256`, fill white.

---

### SECTION 2: HERO (full viewport video background)

- Full `h-screen w-full` section with `overflow-hidden`
- Background: autoplay muted looping video covering the full section (`object-cover`), URL:
  `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260611_130946_e6793cc7-6b6f-4035-9852-44290b781ae6.mp4`
- Content centered vertically and horizontally, z-10, white text:
  - **Subtitle line 1** (delay 0.1s): "Studioworks" -- `text-xs` mobile / `text-sm` sm+, `font-medium`, uppercase, `tracking-[0.35em]`, `text-white/90`
  - **Subtitle line 2** (delay 0.1s): "Exhibits" -- `text-[10px]` mobile / `text-xs` sm+, `font-light`, uppercase, `tracking-[0.4em]`, `text-white/70`
  - **Heading** (delay 0.25s): Two lines:
    - "DIGITAL" in Arsenica, `text-5xl` up to `text-[7rem]`, `tracking-wide`
    - "ARCHIVE" in Inter, `font-semibold`, `text-5xl` up to `text-[7rem]`, `tracking-tight`
    - `leading-[1.05]`, `drop-shadow-[0_2px_24px_rgba(0,0,0,0.25)]`
  - **Description** (delay 0.4s): "A showcase honoring the makers, visionaries and creators who turned a hard season into something rare." -- Arsenica font, `text-sm` up to `text-xl`, `max-w-xl`, `text-white/90`
  - **CTA Button** (delay 0.55s): "Enter Gallery" -- uses `.liquid-glass` class, `rounded-[50%]` (pill), `px-10 py-5` mobile / `px-12 py-6` sm+, `text-[10px]` mobile / `text-xs` sm+, uppercase, `tracking-[0.25em]`, Inter font, hover effects: `scale-[1.03]`, `shadow-[0_0_30px_rgba(255,255,255,0.15)]`, active `scale-[0.98]`

---

### TRANSITION LAYER (between Hero and Showcase)

In the App layout, after the Hero, add a decorative cloud/fog image that overlaps:
```
<div className="relative z-20 -mt-64 sm:-mt-72 md:-mt-80 lg:-mt-96">
  <img src="https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781584857/top-bg_j88wyu.png" className="pointer-events-none w-full" />
</div>
```

---

### SECTION 3: SHOWCASE (full-screen image background with text)

- Wrapped in a container with `relative -mt-40 sm:-mt-48 md:-mt-56 lg:-mt-64` (negative margin to overlap the transition image)
- Full `min-h-screen` section with `overflow-hidden`
- Background image (absolute, object-cover):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260616_040223_98d314e9-b8b4-4218-bcbd-18ffc38032ac.png&w=1280&q=85`
- Content (centered, z-10, py-32, using `useScrollReveal`):
  - **Heading**: "Still Frame" -- Arsenica, `text-4xl` up to `text-7xl`, `tracking-wide`, `drop-shadow-[0_2px_20px_rgba(0,0,0,0.3)]`, class `.reveal`
  - **Subtext** (delay 0.15s): Three lines --
    ```
    gave the world beauty
    born from the silence
    of empty studios.
    ```
    Arsenica, `text-xl` up to `text-4xl`, `tracking-wide`, `text-white/90`, `drop-shadow-[0_2px_16px_rgba(0,0,0,0.25)]`, class `.reveal`
  - **Button** (delay 0.3s): "View Their Archive" -- rounded-[50%] pill, `border border-white/50`, transparent bg, `px-10 py-4` mobile / `px-12 py-5` sm+, Inter, `text-[10px]`/`text-xs`, uppercase, `tracking-[0.25em]`, hover: `border-white bg-white/10 scale-[1.03] shadow-[0_0_30px_rgba(255,255,255,0.1)]`, class `.reveal`
- **Bottom gradient**: absolute bottom div, `h-48 w-full`, gradient from transparent to `#410C01`
- **Dove image**: positioned absolute on the parent wrapper:
  `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781584853/dove_xpaeub.png`
  Right-aligned, offset below bottom (`-bottom-12` mobile up to standard positioning), responsive widths (`w-24` up to `w-64`), z-20

---

### SECTION 4: Q&A (dark maroon background with parallax cloud overlay)

- Background: solid `bg-[#410C01]`
- Padding: `px-4 pt-20` mobile, up to `px-28 pt-32` on lg. `paddingBottom: '50vh'` (inline style)
- **Title**: "Q & A" -- Each letter separately, Arsenica font, `text-4xl` up to `text-7xl`, centered with `flex items-baseline justify-center gap-1`. The ampersand is italic, smaller (`text-xl` to `text-4xl`), `text-white/80`. Uses `.reveal` class.
- **Two-column layout** (grid, `md:grid-cols-2`, gap-10 up to gap-20):
  - **Left column** -- 3 Q&A items
  - **Right column** -- 3 Q&A items, offset `md:mt-24`
  - Each item has staggered animation delay (starting at 0.12s, incrementing by 0.12s per item globally across both columns)
  - **Question**: Arsenica, `text-xs` up to `text-base`, uppercase, `tracking-wide`, white
  - **Answer**: Inter, `text-[11px]` up to `text-sm`, `leading-relaxed`, `text-white/60`

**Q&A Content:**
- Left column:
  1. Q: "Welcome Maren. So how did Still Frame begin its journey?" / A: "Less than a year into launching the gallery, everything shut down. I had to close our doors, cancel every exhibit, and rethink it all. But I never stopped curating because I was so determined not to let the artists' momentum die. We hit the ground running to build a digital space, and we've been evolving since."
  2. Q: "How did you know where to begin?" / A: "I didn't wait until we had the perfect platform. I saw artists struggling, isolated, uninspired, overwhelmed, and set to the task of creating ways to share their work with the world as quickly as possible."
  3. Q: "So what was the first exhibit?" / A: "We were one of the first galleries to launch a virtual exhibition after the shutdown. I think our artists were really grateful for that, they saw how hard we worked to honor their craft, and they trusted us while we continued to refine the digital experience."

- Right column:
  1. Q: "What was the initial reaction?" / A: "We had so many people writing and reaching out that the online exhibits and archived works saved them in isolation. The atmosphere was so intimate, and it was really powerful to have people connecting through art, even though we were all in our own rooms, in different cities."
  2. Q: "Where did you evolve from there?" / A: "The in-person pop-ups have been really special too, recently, now that enough people feel comfortable to gather. We had our first open-air exhibit in the courtyard last month, and I was basically in tears it was so beautiful."
  3. Q: "Do you find there's a new appreciation for art?" / A: "There's a feeling of urgency like -- this is our one life, our one chance, we don't have time to be indifferent anymore. We're gonna create like there's no tomorrow, we're gonna create for a better world, we're gonna create to reclaim our voice in this life, and we're gonna create because we deserve to feel beauty and wonder."

- **Parallax cloud overlay**: The same cloud image from earlier (`top-bg_j88wyu.png`) positioned absolute at `bottom-0 left-0 w-full z-10`. It has a parallax scroll effect: as section scrolls, the image transforms with `translateY(60 - offset%)` where offset is `progress * 30` and progress = `1 - rect.bottom / (vh + rect.height)`.

---

### SECTION 5: QUOTE BANNER (full-screen background with parallax bottom overlay)

- Full viewport height, centered content
- Background image (via inline style `backgroundImage`, with `bg-cover bg-center`):
  `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260616_042421_41f4fa0b-770c-4545-a416-73a809366e49.png&w=1280&q=85`
- On lg+, content aligns `items-start pt-[25vh]` instead of center
- **Quote** (uses `.reveal-scale` animation): `"Art, resilience and vision are more important than ever."` where "are more important than ever." is in `font-light italic`. Arsenica, `text-xl` up to `text-5xl`, `leading-snug`/`lg:leading-tight`, white, max-w `xs` up to `2xl`
- **Parallax bottom overlay**: Image positioned absolute `-bottom-16 left-0 w-full z-10`:
  `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1781584854/bottom_bg_liw6lc.png`
  Parallax: transforms with `translateY(-offset px)` where offset is `progress * 80`.

---

### SECTION 6: FOOTER (fixed bottom bar)

- Fixed at bottom (`fixed bottom-0 left-0 right-0 z-40`)
- `bg-gradient-to-t from-black/40 to-transparent`
- Flex row with `justify-between`, padding `px-3 py-2.5` mobile / `px-10 py-4` sm+
- **Left side**:
  - Facebook icon (lucide `Facebook`, `h-3.5 w-3.5` / `h-4 w-4` sm+)
  - Twitter icon (lucide `Twitter`)
  - LinkedIn icon (lucide `Linkedin`)
  - "Privacy Notice" text link (hidden on mobile, shown sm+)
- **Right side**:
  - "Terms & Policies" text link (hidden on mobile, shown sm+)
  - BarChart3 icon (lucide `BarChart3`)
  - Aperture icon (lucide `Aperture`)
- Icon links: `text-white/80`, hover `text-white`
- Text links: `text-[9px]` mobile / `text-[10px]` sm+, uppercase, `font-medium`, `tracking-[0.15em]`/`tracking-[0.25em]`, `text-white/80`, hover `text-white`

---

### APP LAYOUT ORDER

```
<Navbar />
<Hero />
<TransitionCloudImage (overlaps hero with negative margin) />
<div (negative margin wrapper)>
  <Showcase />
  <DoveImage (absolute positioned) />
</div>
<QAndA />
<QuoteBanner />
<Footer />
```

---

### TAILWIND CONFIG

Default Tailwind v3 config -- no custom theme extensions. Content: `['./index.html', './src/**/*.{js,ts,jsx,tsx}']`.

---

### KEY DESIGN DETAILS

- No background color on body -- sections provide their own
- Negative margins create seamless overlapping transitions between sections
- The "liquid glass" effect creates a frosted-glass look with a subtle gradient border pseudo-element
- All animations use `cubic-bezier(0.22, 1, 0.36, 1)` easing (smooth deceleration)
- Staggered animations via inline `animationDelay` styles
- Responsive scaling uses Tailwind breakpoints: default (mobile) -> sm -> md -> lg -> xl
- Color palette: white text on dark backgrounds, `#410C01` (deep burnt maroon) as the Q&A section background
- Typography hierarchy: Arsenica for display/headings, Inter for body/UI text

## Guardnet — Landing Page [sites/guardnet-landing]

- Preview: https://motionsites.ai/assets/hero-guardnet-preview-DAQqiNXC.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/guardnet-landing.gif

Build a single-page React + TypeScript + Vite + Tailwind CSS landing page for a security/privacy brand called guardnet. Use lucide-react only if needed; do not add other UI libraries. Place everything in src/App.tsx and src/index.css. Use the Bolt Database only if persistence is actually needed (this page needs none).

Global Styling
src/index.css:

Import the font: @import url(https://db.onlinewebfonts.com/c/e55e9079ee863276569c8a68d776ef04?family=Futura+Md+BT+Medium);
Apply Tailwind base/components/utilities.
Set html, body, #root { height: 100% }.
body uses font-family: 'Futura Md BT Medium', system-ui, -apple-system, sans-serif;, background-color: #000, color: #fff, antialiased font smoothing.
Add a .hero-title class with letter-spacing: -0.04em; line-height: 0.95;.
Tailwind config: extend fontFamily.sans to ['"Readex Pro"', 'system-ui', 'sans-serif'].

Root App layout: min-h-screen bg-black text-white flex flex-col items-center overflow-x-hidden w-full. Hero renders full-bleed; the remaining sections wrap in a w-full max-w-[1400px] container.

1. LogoMark Component
Inline SVG (viewBox 0 0 256 256, white fill) with path:
M 128 192 L 128 256 L 64.5 256 L 32 223 L 0 192 L 0 128 L 64 128 Z M 256 192 L 256 256 L 192.5 256 L 160 223 L 128 192 L 128 128 L 192 128 Z M 128 64 L 128 128 L 64.5 128 L 32 95 L 0 64 L 0 0 L 64 0 Z M 256 64 L 256 128 L 192.5 128 L 160 95 L 128 64 L 128 0 L 192 0 Z

2. Navbar (absolute, over hero)
absolute top-0 left-0 right-0 z-20 px-3 sm:px-6 md:px-10 pt-4 sm:pt-6, nav is flex items-center justify-between.

Left pill: bg-neutral-900/90 backdrop-blur rounded-full pl-3 pr-4 sm:pl-4 sm:pr-6 py-2.5 sm:py-3, contains LogoMark (h-4/5) + text "guardnet" (text-white text-xs sm:text-sm).
Center pill (hidden on mobile, hidden md:flex): bg-neutral-900/90 backdrop-blur rounded-full px-3 py-2, links array ['products','offerings','mission','contact'], each text-neutral-300 hover:text-white text-sm px-5 py-2 rounded-full.
Right button: bg-white text-black text-xs sm:text-sm rounded-full px-4 sm:px-6 py-2.5 sm:py-3 hover:bg-neutral-200 with label "start today".
3. Hero Section
section: relative min-h-screen h-screen w-full bg-black overflow-hidden.

Background video (absolute, inset-0 w-full h-full object-cover, autoPlay loop muted playsInline):
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260421_074215_f4339e1c-0b1a-4f60-98b2-90e3d7840cb7.mp4

Inside, inner wrapper relative h-full w-full max-w-[1320px] mx-auto.

Three massive h1.hero-title words (text-white font-medium text-[24vw] md:text-[18vw]), absolutely positioned:

"shelter" — left-3 sm:left-4 md:left-10 top-[20%] sm:top-[18%]
"user" — right-3 sm:right-4 md:right-10 top-[36%] sm:top-[38%]
"info" — left-[10%] sm:left-[18%] md:left-[28%] top-[56%] sm:top-[58%]
Paragraph under/between words: absolute left-4 sm:left-6 md:left-10 top-[48%] sm:top-[46%] max-w-[220px] sm:max-w-[300px] text-[13px] sm:text-[18px] leading-relaxed text-white/90 font-light — copy: "we are holding each file with supreme care, granting user with safety in all place".

Three stat blocks; each has a number text-2xl sm:text-4xl md:text-5xl font-medium tracking-tight, an angled divider hidden md:block h-px w-24 bg-white/40 rotated, and a caption text-[10px] sm:text-xs md:text-sm text-white/70 mt-1 font-light:

Bottom-left: +2.7b / "mb info was concealed" (divider rotated -20deg, on the right).
Top-right: +90k / "ventures run" (divider on left, rotated 20deg).
Bottom-right: +450k / "transfers" (divider on left, rotated -20deg).
Bottom fade: pointer-events-none absolute bottom-0 left-0 right-0 h-48 bg-gradient-to-b from-transparent to-black.

4. SecuritySection
section: relative min-h-[600px] h-screen w-full overflow-hidden bg-black.

Background video (same attrs as hero):
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260421_072418_508a7d2e-396d-4f6f-9d42-ec920fcf7755.mp4

Top fade overlay: pointer-events-none absolute top-0 left-0 right-0 h-48 bg-gradient-to-b from-black to-transparent z-10.

Inner wrapper relative h-full w-full max-w-[1100px] mx-auto.

Floating center pill at top (absolute top-6 sm:top-10 left-1/2 -translate-x-1/2 z-20 w-max max-w-[95vw]): bg-neutral-900/80 backdrop-blur rounded-full p-2 sm:p-3 containing two buttons:

Ghost: text-white/90 text-xs sm:text-sm px-4 sm:px-7 py-2 sm:py-3 rounded-full hover:text-white whitespace-nowrap — "confirm real person".
Gradient: text-black text-xs sm:text-sm px-4 sm:px-7 py-2 sm:py-3 rounded-full with inline style background: linear-gradient(90deg, #FA8453 0%, #F8C9B2 100%) — "run demo".
Two paragraphs (text-[13px] sm:text-[18px] leading-relaxed font-light):

Left: absolute left-4 sm:left-6 md:left-16 top-[62%] sm:top-[56%] max-w-[280px] sm:max-w-[440px] text-white/80 — "shielding users info with premier tech, granting them with safety in all place".
Right: absolute right-4 sm:right-6 md:right-16 top-[26%] sm:top-[34%] max-w-[280px] sm:max-w-[500px] text-white/90 — "By teaming up with a defender service, a business can dramatically improve the safeguard of its important info. This covers applying strong obfuscation protocols, gateway barriers, and observation engines to shield against unauthorized entries, info escapes, and malicious cyberhacks."
5. CompaniesSection
section: relative w-full bg-black px-4 sm:px-6 md:px-10 py-12 sm:py-20.

Grid grid grid-cols-2 md:grid-cols-4 gap-3 sm:gap-4. Four cards, each relative h-24 sm:h-32 md:h-36 rounded-2xl bg-neutral-950 overflow-hidden flex items-center justify-center. Each card hosts a soft blurred color blob (absolute ... h-40 w-40 rounded-full blur-3xl opacity-30/40) and a centered logo (relative z-10).

Logos (inline SVG h-6 w-6 sm:h-8 sm:w-8 fill white + wordmark text-white text-xl sm:text-3xl font-semibold tracking-tight):

Apex — star path M12 2l2.39 4.84L20 8l-4 3.9L17.28 18 12 15.27 6.72 18 8 11.9 4 8l5.61-1.16L12 2z. Blob: -top-24 -left-24 bg-[#1e3a8a] opacity-40.
forge — path M20.63 8.46l-4.73-2.73-.53.31 5.1 2.94v5.88l-5.1 2.94.53.3 4.73-2.72V8.46zM8.1 6.04l.53.3L3.53 9.28v5.88L8.63 18.1l-.53.3-4.73-2.72V8.46L8.1 6.04zM16.05 14.3v-4.6L12 7.4 7.95 9.7v4.6L12 16.6l4.05-2.3zm-.53-.3L12 16.02l-3.52-2.02v-4.02L12 7.96l3.52 2.02v4.02z. Two blobs: -top-24 -left-24 bg-[#FA8453] opacity-30 and -bottom-24 -right-24 bg-[#F5D547] opacity-25.
Eastern Delta — path M2 4l3 16h3l2-10 2 10h3l3-16h-3l-1.5 10L12 4h-2L8.5 14 7 4H2z, wordmark uses two lines text-lg sm:text-2xl font-semibold leading-tight. Blob: -bottom-24 -left-24 bg-[#F5D547] opacity-30.
Skybank — path M6 2l6 3.75L6 9.5 0 5.75 6 2zm12 0l6 3.75L18 9.5l-6-3.75L18 2zM0 13.25L6 9.5l6 3.75L6 17l-6-3.75zm18-3.75l6 3.75L18 17l-6-3.75 6-3.75zM6 18.25L12 14.5l6 3.75L12 22l-6-3.75z. Blob: top-1/2 -translate-y-1/2 -right-28 h-48 w-48 bg-[#1e3a8a] opacity-40.
Below the grid: mt-16 sm:mt-28 flex flex-col md:flex-row items-start md:items-center justify-between gap-6 sm:gap-8 md:w-[70%] md:ml-auto:

Paragraph: max-w-md text-[13px] sm:text-[18px] leading-relaxed text-white/70 font-light — "shielding users info with premier tech, granting them with safety in all place".
Gradient-border button: outer relative rounded-full p-[1.5px] with inline style background: linear-gradient(90deg, #FA8453 0%, #F8C9B2 100%); inner block rounded-full bg-black px-8 sm:px-10 py-2.5 sm:py-3 text-white text-sm labeled "Run Demo".
6. BenefitsSection
section: relative w-full bg-black px-4 sm:px-6 md:px-10 py-12 sm:py-20.

Heading: text-white text-3xl sm:text-4xl md:text-5xl font-light text-center mb-12 sm:mb-24 with inline letter-spacing: -0.04em, text "Key Benefits".

Grid grid grid-cols-1 md:grid-cols-3 gap-3 sm:gap-4. Each card h-[380px] sm:h-[460px] rounded-2xl bg-neutral-950 overflow-hidden.

Card 1 (padded p-6 sm:p-8): blurred blob absolute top-1/2 -translate-y-1/2 -left-[420px] h-[460px] w-[460px] rounded-full bg-[#1e3a8a] blur-3xl opacity-40. Content: h3 text-xl sm:text-2xl font-light leading-tight "Preemptive Risks / Scouting and Reactions" (br between); paragraph mt-12 sm:mt-20 text-[13px] sm:text-[14px] leading-relaxed text-white/70 font-light max-w-[280px]: "Defense platforms constantly observe bandwidth streams, record files, and machine behaviors to uncover unusual patterns or outliers that could signal a defensive failure."

Card 2 (flex flex-col): top video region w-full overflow-hidden with height: 75%:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260421_072701_f6a01abb-eb30-4559-9d6e-774362defbc3.mp4
(autoPlay loop muted playsInline, w-full h-full object-cover block). Bottom fade inside video: pointer-events-none absolute bottom-0 left-0 right-0 h-32 bg-gradient-to-b from-transparent to-neutral-950. Under video: flex-1 flex items-center justify-start p-6 sm:p-8 with h3 text-xl sm:text-2xl font-light leading-tight "Know-how and Sectoral / Awareness".

Card 3 (padded): blob absolute -top-28 -right-28 h-56 w-56 rounded-full bg-[#1e3a8a] blur-3xl opacity-40. Same heading as Card 1. Paragraph pinned to bottom (mt-auto ... max-w-[320px]) with identical body text to Card 1.

Animations & Interactions
Background videos loop and autoplay muted.
Nav links and CTA button use Tailwind transition-colors hover states.
No other JS animations; motion comes from video loops and hover color transitions.
Compose
App renders, in order: <Hero />, then inside a max-w-[1400px] wrapper: <SecuritySection />, <CompaniesSection />, <BenefitsSection />.

## USD Halo — Landing Page [sites/halo-usd-landing]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/halo-usd-hero-CtMXOklk.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/halo-usd-landing.gif

Build a premium, fintech-style landing page for a stablecoin product called "Halo / USD Halo" using React + TypeScript + Vite + Tailwind CSS, with lucide-react for icons. No other UI libraries. Background color of the page is #F5F5F5.

Global Setup
Use TT Norms Pro as the primary font, loaded via @font-face from /fonts/tt-norms-pro-regular.woff2 (weight 400) and /fonts/tt-norms-pro-semibold.woff2 (weight 600), with font-display: swap. Apply it to html, body, and inherit on *.
Tailwind base + components + utilities at the top of src/index.css.
Page wrapper: flex flex-col bg-[#F5F5F5]. The first section (Navbar + Hero) is wrapped in a h-screen flex flex-col overflow-hidden container.
Inner content max width across sections: max-w-[88rem] mx-auto.
Custom Logo Icon
Create an SVG component LogoIcon using currentColor, viewBox 0 0 256 256, with this path (a stylized "halo" mark made of two interlocking rounded squares):


M 128.005 191.173 C 128.448 156.208 156.93 128 192 128 L 192 64 L 128 64 C 128 99.346 99.346 128 64 128 L 64 192 L 128 192 Z M 192 256 L 64 256 C 28.654 256 0 227.346 0 192 L 0 64 L 64 64 L 64 0 L 192 0 C 227.346 0 256 28.654 256 64 L 256 192 L 192 192 Z
1. Navbar (absolute, transparent over hero)
nav is absolute top-0 left-0 right-0 z-20 px-6 py-5.
Inner row: flex items-center justify-between.
Left: LogoIcon (w-7 h-7, black) + word "Halo" (text-2xl font-medium tracking-tight text-black).
Center (hidden below md): links Network · Ecosystem · Rewards · Help · News, gap-8, text-base text-gray-700 hover:text-black font-medium transition-colors duration-200.
Right: black pill button "Open Wallet" — bg-black text-white text-base font-medium px-7 py-2.5 rounded-full hover:bg-gray-800 transition-colors duration-200.
2. Hero Section
Outer: flex-1 px-6 pt-20 pb-6 flex items-end.
Inner card: relative w-full rounded-2xl overflow-hidden, inline style height: calc(100vh - 96px).
Background video (autoplay, muted, loop, playsInline, object-cover absolute inset-0 w-full h-full):
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_161253_c72b1869-400f-45ed-ac0c-52f68c2ed5bd.mp4

Content overlay: relative z-10 flex flex-col items-start justify-start h-full p-12 pt-36.
h1: "Your Wealth\nWorks" (with <br/>) — text-black text-5xl md:text-6xl font-medium leading-tight max-w-xl mb-4, inline letterSpacing: '-0.04em'.
p: "An automated, reward-powered digital dollar built for native passive earnings and effortless connection into DeFi." — text-black/70 text-base md:text-lg max-w-md mb-8 leading-relaxed, inline fontFamily: "'Inter', ui-sans-serif, system-ui, sans-serif".
Pill button "Join us" with arrow circle: inline-flex items-center gap-3 bg-black text-white text-base md:text-lg font-medium pl-8 pr-2 py-2 rounded-full hover:bg-gray-800. Trailing arrow inside bg-white rounded-full p-2, using ArrowRight w-5 h-5 text-black from lucide-react.
Followed by the Brand Marquee below.
Brand Marquee (inside hero, below button)
Container: mt-24 w-full max-w-md overflow-hidden.
Inject scoped <style> with keyframes marquee translating 0 → -50%, applied to .marquee-track { display:flex; width:max-content; animation: marquee 22s linear infinite; }.
Render the brand list twice (so it loops seamlessly).
Each item: mx-7 shrink-0 text-black/60 whitespace-nowrap with these inline styles:
Stripe — Georgia serif, weight 700, letterSpacing -0.02em, fontSize 15px
Coinbase — Arial sans, weight 900, letterSpacing 0.08em, fontSize 13px, uppercase
Uniswap — Trebuchet MS, weight 600, letterSpacing 0.01em, fontSize 15px, italic
Aave — Courier New monospace, weight 700, letterSpacing 0.12em, fontSize 13px, uppercase
Compound — Palatino, Book Antiqua, weight 400, letterSpacing -0.01em, fontSize 16px
MakerDAO — Impact, Arial Narrow, weight 400, letterSpacing 0.04em, fontSize 14px
Chainlink — Verdana, weight 700, letterSpacing -0.03em, fontSize 13px
3. Info Section ("Meet USD Halo.")
section bg-[#F5F5F5] px-6 py-24.
Row 1: 2-col grid (grid-cols-1 md:grid-cols-2 gap-12 mb-16 items-start).
Left: h2 "Meet USD Halo." — text-black text-4xl md:text-5xl font-medium leading-tight mb-8, letterSpacing -0.03em. Below it, black pill "Discover it" button with white arrow circle (same pattern as "Join us" but text-base).
Right: paragraph "USD Halo is a reward-earning dollar coin that lets your savings grow while remaining tied to the U.S. dollar." — text-black/70 text-2xl md:text-3xl leading-relaxed.
Row 2 — 4-col card grid (grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4):
Card 1 (spans 2 cols on lg): rounded-2xl with background image:
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260423_164207_f243351d-ed59-48ec-83a0-a5e996bdbe3c.png&w=1280&q=85

 backgroundSize: cover; backgroundPosition: center. Inside: p-7 min-h-80 flex flex-col justify-between. Title (top): "Savings that bloom" — text-black text-2xl font-medium leading-snug letterSpacing -0.02em. Body (bottom): "Gain steady returns as your dollar tokens are routed into top-performing DeFi strategies." — text-black/70 text-base max-w-xs.
Card 2: solid #2B2644, rounded-2xl, p-7, min-h-80, flex-col-justify-between. White heading "Always fluid,\nalways pegged." text-2xl font-medium, body "Keep fully dollar-anchored with on-demand access to funds — no lockups or waits." text-white/60 text-base.
Card 3: same #2B2644 styling. Heading "Fully\nautomated". Body "Skip the task of tuning positions yourself. USD Halo runs in the background for you."
4. Backed By Section (marquee row)
section bg-[#F5F5F5] px-6 with inner max-w-[88rem] mx-auto grid grid-cols-1 md:grid-cols-4 gap-8 items-center.
Left col (1/4): text-black/70 text-base leading-relaxed — "Funded by premier partners\nand forward-thinking leaders."
Right col (3/4): infinite marquee (same pattern as hero marquee but 30s linear infinite, class .backers-track, keyframes backers-marquee). Items use mx-10 shrink-0 text-black/50 whitespace-nowrap with these inline styles:
Fundamental Labs — Times New Roman serif, 400, ls 0.02em, 14px
KUCOIN — Arial Black, 900, ls 0.08em, 16px
NGC — Impact, 700, ls 0.05em, 18px
NxGen — Georgia, 600, ls -0.02em, 17px
Matter Labs — Helvetica, 700, ls -0.01em, 15px
DEXTools — Verdana, 700, ls 0.06em, 14px, uppercase
NGRAVE — Courier New, 700, ls 0.18em, 14px
Polychain — Palatino, 500, ls 0.03em, 15px
Render brands twice for the loop.
5. Use Cases Section
section bg-[#F5F5F5] px-6 py-24. Inner: 2-col grid grid-cols-1 md:grid-cols-2 gap-8 items-start.
Left column (md:pr-12 md:pt-2):
Eyebrow: "USD Halo in Practice" — text-black/60 text-sm mb-2.
h2 "Use modes" — text-5xl md:text-6xl font-medium leading-none mb-6, ls -0.04em.
Paragraph: "USD Halo powers a wide range of modes for builders, companies and treasuries wanting safe and rewarding stablecoin integrations plus more" — text-black/60 text-base leading-relaxed max-w-sm.
Right column: large relative rounded-3xl overflow-hidden min-h-[720px] with background video (autoplay/muted/loop/playsInline, object-cover absolute inset-0):
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260423_183428_ab5e672a-f608-4dcb-b319-f3e040f02e2d.mp4

Overlay content relative z-10 p-10 md:p-12:
h3 "Commerce" — text-4xl md:text-5xl font-medium leading-tight mb-5, ls -0.03em.
Paragraph: "Lift customer retention by offering USD Halo, a trusted dollar-backed stablecoin with strong yields, letting your patrons earn with zero effort on your platform." — text-black/70 text-base max-w-md mb-8.
Inline-flex link "Know more" with leading circular icon: w-9 h-9 rounded-full bg-white/80 backdrop-blur flex items-center justify-center group-hover:bg-white transition-colors containing ArrowRight w-4 h-4 text-black.
Animations & Interactions
Two CSS keyframe marquees (22s for hero brands, 30s for backers), both translating 0 → -50% on a duplicated track for seamless looping.
All buttons use transition-colors duration-200 with hover state hover:bg-gray-800 (or hover:bg-white for the white circle).
Nav links transition on hover from text-gray-700 to text-black.
Videos autoplay muted with playsInline for mobile compatibility.
Composition
App renders, in order:

h-screen overflow-hidden wrapper containing Navbar (absolute) + HeroSection.
InfoSection
BackedBySection
UseCasesSection
All section backgrounds are #F5F5F5. All headings use negative letter-spacing for the tight, modern fintech feel. Use font-medium (600) as the heaviest weight throughout.

## Health Portal — Landing Page [sites/health-portal]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(12).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/health-portal.webp

Create a single-page dental clinic landing page using **React + Vite + TypeScript + Tailwind CSS**. No external UI libraries, no icon libraries. Everything lives in one `App.tsx` file. The page has 3 full-screen sections, a splash screen, and a fixed navbar.

---

### SETUP

**Font:** "Open Sauce One" loaded via these exact links in `index.html` `<head>`:
```html
<link href="https://db.onlinewebfonts.com/c/1cd1e7d71e048159076fd90b39846902?family=Open+Sauce+One" rel="stylesheet">
<link href="https://db.onlinewebfonts.com/c/42acf9aa4a6dc2f2886a3f682e337ead?family=Open+Sauce+One+Bold" rel="stylesheet">
```

**Title:** "Dental Health - Quality Healthcare"

**Global CSS (index.css):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html, body, #root {
    height: 100%;
    margin: 0;
    padding: 0;
  }
  body {
    font-family: 'Open Sauce One', -apple-system, BlinkMacSystemFont, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
}
```

**Tailwind config:** Default, no extensions. Content: `['./index.html', './src/**/*.{js,ts,jsx,tsx}']`.

---

### IMAGE URLS (use these EXACT URLs)

```ts
const HERO_IMAGE = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_113640_ccf3cf97-d447-425b-a134-d7b09fc743fc.png&w=1280&q=85';

const SECTION2_IMAGE = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_114219_414dfe80-f15c-4e25-bf52-b13721f4bd88.png&w=1280&q=85';

const SECTION3_IMG1 = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_115253_c19ab167-8dd5-48b4-967d-b9f0d9d6e8fb.png&w=1280&q=85';

const SECTION3_IMG2 = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_115237_fc519057-6e87-4abf-999a-9610b8b085b4.png&w=1280&q=85';

const SECTION3_BG = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_114355_752ba9e6-0942-4abb-9047-5d9bb16632e9.png&w=1280&q=85';
```

---

### DATA CONSTANTS

```ts
const featureBars = ['Advanced Dentistry', 'High Quality Equipment', 'Friendly Staff'];

const services = [
  { name: 'Dental\nVeneers', num: '01', active: true },
  { name: 'Dental\nCrowns', num: '02', active: false },
  { name: 'Teeth\nWhitening', num: '03', active: false },
  { name: 'Dental\nImplants', num: null, active: false },
];
```

---

### CORE TECHNICAL CONCEPT: "MASKED CARDS"

Sections 1 and 2 use a single large background image shared across multiple cards. Each card shows a different "window" into the same image, creating a cohesive mosaic effect. Implementation:

**`useMaskPositions` hook:**
- Takes a ref to the section container and a ref to an array of card elements.
- Uses `ResizeObserver` on the section container.
- For each card, computes `{ x, y, sw, sh }` where x/y is the card's top-left offset relative to the section, sw/sh is the section's width/height.

**`useImageWidth` hook:**
- Loads the image in a `new Image()` object.
- Calculates: `renderWidth = img.naturalWidth * (sectionHeight / img.naturalHeight)`.
- Returns how wide the image would be if scaled to fill the section height.

**`MaskedCard` component:**
- Props: `bgImage`, `position` (from useMaskPositions), `imageWidth` (from useImageWidth), `focalX` (0-1 float), `className`, `children`, `cardRef`, `style`.
- Calculates `overflow = imageWidth > position.sw ? imageWidth - position.sw : 0`, then `focalOffset = overflow * focalX`.
- Applies inline style:
  ```
  backgroundImage: url(bgImage)
  backgroundSize: auto [position.sh]px
  backgroundPosition: -[position.x + focalOffset]px -[position.y]px
  backgroundRepeat: no-repeat
  ```
- `focalX` values: Section 1 mobile=0.7, desktop=0.8. Section 2 mobile=0.65, desktop=0.8.

**`useIsMobile` hook:**
- Listens to `window.matchMedia('(max-width: 767px)')` change events.
- Returns boolean.

---

### ANIMATION: `useStaggeredReveal` hook

- Takes `count` (number of elements) and `threshold` (IntersectionObserver threshold, default 0.15).
- Returns `{ containerRef, getAnimStyle }`.
- `containerRef` is attached to the section; when it crosses the threshold, `visible` becomes true (fires once).
- `getAnimStyle(index)` returns:
  ```css
  opacity: visible ? 1 : 0
  transform: visible ? 'translateY(0)' : 'translateY(24px)'
  transition: opacity 0.6s cubic-bezier(0.16,1,0.3,1) [index*120]ms,
              transform 0.6s cubic-bezier(0.16,1,0.3,1) [index*120]ms
  ```

---

### SPLASH SCREEN

- Fixed overlay covering viewport, `z-[100]`, white background.
- Number counter displayed at **bottom-left** (`items-end justify-start`).
- Counter style: `text-7xl md:text-9xl font-bold tabular-nums p-6 md:p-10 leading-none`, black text.
- Counts from 0 to 100 over exactly 2000ms (20ms per step, 100 steps).
- After reaching 100: wait 200ms, then set `exiting=true` which triggers `opacity-0` with `transition-opacity duration-700`.
- After 900ms total from reaching 100, call `onComplete()` which removes splash from DOM.

---

### NAVBAR

**Container:** `fixed top-0 left-0 right-0 z-50`, `flex items-center justify-between`, `px-4 md:px-6 py-2 md:py-3`, `bg-white/80 backdrop-blur-md`.

**Logo (left side):**
- Two lines stacked: "Dental" and "Health"
- Wrapper: `flex flex-col`
- Text: `text-xl md:text-2xl font-extrabold uppercase tracking-tight leading-none`
- Second line has `-mt-1.5 md:-mt-2` for tight spacing
- Below logo text: "quality healthcare" in `text-[8px] md:text-[9px] font-medium leading-none mt-1.5 md:mt-2`

**Desktop nav (hidden on mobile with `hidden md:block`):**
- "Menu" button: `px-6 py-3 bg-white rounded-full border border-black text-sm font-semibold`, hover: `hover:bg-black hover:text-white transition-colors duration-200`
- "Dental Emergency" text: `text-sm font-semibold text-black`

**Mobile hamburger (visible only on mobile with `md:hidden`):**
- Container: `w-10 h-10 flex items-center justify-center`, `relative`
- 3 spans, each: `absolute h-0.5 w-6 bg-black rounded-full`
- Transition: `transition-all duration-300 ease-[cubic-bezier(0.76,0,0.24,1)]`
- Closed state: top span `-translate-y-2`, middle `opacity-100 scale-x-100`, bottom `translate-y-2`
- Open state: top `rotate-45 translate-y-0`, middle `opacity-0 scale-x-0`, bottom `-rotate-45 translate-y-0`

**Mobile menu overlay (`md:hidden`):**
- Outer: `fixed inset-0 z-40`, pointer-events toggled based on open state
- Backdrop: `absolute inset-0 bg-black/20 backdrop-blur-sm`, fades opacity. Clicking closes menu.
- Panel: `absolute top-0 right-0 h-full w-[85%] max-w-sm bg-white shadow-2xl`, slides with `translate-x-0` (open) / `translate-x-full` (closed), `duration-500 ease-[cubic-bezier(0.76,0,0.24,1)]`
- Content: `flex flex-col justify-center h-full px-8 gap-1`
- Nav links: ['Home', 'Services', 'About', 'Gallery', 'Contact']
  - Each: `text-4xl font-bold text-black hover:text-neutral-500`
  - Staggered entrance: `opacity-0 translate-x-8` -> `opacity-100 translate-x-0`, `transitionDelay: ${100 + i * 60}ms` when open
  - `transition-all duration-500 ease-[cubic-bezier(0.76,0,0.24,1)]`
- Bottom section: `mt-8 pt-8 border-t border-neutral-200`, delayed 450ms
  - "Dental Emergency" text: `text-sm font-semibold text-black mb-4`
  - Button: `w-full px-6 py-4 bg-black rounded-full text-white text-sm font-semibold hover:bg-neutral-800 transition-colors duration-200`, text "Book Appointment"
- When open: `document.body.style.overflow = 'hidden'`. Cleanup on unmount.

---

### SECTION 1 - HERO

**Container:** `<section>`, `h-screen w-full overflow-hidden flex flex-col`, `pt-24 md:pt-24 px-3 md:px-5 pb-1.5 md:pb-2 gap-1.5 md:gap-2`

Attach both `section1Ref` and `s1Reveal.containerRef` to this element.

Uses `HERO_IMAGE` as shared background via MaskedCard technique.

**3 Feature Bars** (mapped from `featureBars` array):
- Each is a `MaskedCard` with: `w-full h-14 md:h-20 shrink-0 rounded-xl md:rounded-2xl overflow-hidden relative`
- Animated with `s1Reveal.getAnimStyle(i)` for i=0,1,2
- Content: `<span>` centered vertically and horizontally (`flex items-center justify-center h-full`), `text-black text-lg md:text-3xl font-bold text-center`, `relative z-10`

**Main Hero Card** (4th card, index 3):
- `MaskedCard`: `w-full flex-1 min-h-0 rounded-xl md:rounded-2xl overflow-hidden relative`
- Animated with `s1Reveal.getAnimStyle(3)`
- **Top-left text:** `absolute top-4 left-4 md:top-7 md:left-7`, `text-black text-xs md:text-sm font-semibold leading-4 md:leading-5 max-w-[200px] md:max-w-[300px] z-10`
  - Content: "We wish to provide professional dental services" `<br/>` "that match the current technologies"
- **Bottom-left block:** `absolute bottom-5 left-3 md:bottom-8 md:left-4 z-10`
  - Label: `block text-black text-xs md:text-sm font-semibold mb-1 md:mb-2`, text "Trusted Dentist in West New York"
  - Heading: `<h1>` with `text-black text-[clamp(3rem,11vw,11rem)] font-bold leading-[0.79] tracking-tight`, content: "Dental" `<br/>` "Care"
- **Bottom-right text:** `absolute bottom-6 right-4 md:bottom-10 md:right-8`, `text-white text-xs md:text-sm font-semibold z-10`, content: "Free Consultation"

---

### SECTION 2 - SMILE GALLERY

**Container:** `<section>`, `min-h-screen md:h-screen w-full overflow-hidden flex flex-col`, `pt-1.5 md:pt-2 px-3 md:px-5 pb-1.5 md:pb-2 gap-1.5 md:gap-2`

Attach both `section2Ref` and `s2Reveal.containerRef` to this element.

Uses `SECTION2_IMAGE` as shared background via MaskedCard technique.

**Grid container:** `flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 grid-rows-[auto_auto_auto_auto] md:grid-rows-[1fr_1fr_0.8fr] gap-1.5 md:gap-2`

**Card 0 - Top Left ("Smile Gallery"):**
- `MaskedCard`: `rounded-xl md:rounded-2xl overflow-hidden relative min-h-[160px] md:min-h-0`
- Animated: `s2Reveal.getAnimStyle(0)`
- Heading: `absolute top-4 left-5 md:top-6 md:left-7`, `text-white md:text-black text-2xl md:text-3xl font-bold z-10`, text "Smile Gallery"
- Subtitle: `absolute bottom-4 left-5 md:bottom-6 md:left-7`, `text-white md:text-black text-xs md:text-sm font-semibold z-10`, text "Our cosmetic dental work"

**Card 1 - Top Right (spans 2 rows on desktop):**
- `MaskedCard`: `md:row-span-2 rounded-xl md:rounded-2xl overflow-hidden relative min-h-[200px] md:min-h-0`
- Animated: `s2Reveal.getAnimStyle(1)`
- Text: `absolute bottom-16 left-5 md:bottom-20 md:left-7`, `text-white text-xs md:text-sm font-semibold leading-4 md:leading-5 z-10`, content: "If you want a gorgeous smile," `<br/>` "call us to ask about a smile makeover."
- Button: `absolute bottom-4 right-4 md:bottom-6 md:right-6`, `px-5 py-3 md:px-8 md:py-5 bg-white rounded-full text-black text-base md:text-xl font-bold z-10 hover:scale-105 transition-transform`, text "Call Us"

**Card 2 - Bottom Left ("Smile makeover"):**
- `MaskedCard`: `rounded-xl md:rounded-2xl overflow-hidden relative min-h-[160px] md:min-h-0`
- Animated: `s2Reveal.getAnimStyle(2)`
- Heading: `absolute top-4 left-5 md:top-6 md:left-7`, `text-white md:text-black text-[clamp(3rem,7vw,6rem)] font-bold leading-[0.9] z-10`, content: "Smile" `<br/>` "makeover"

**Card 3 - Bottom Full Width (Services):**
- `MaskedCard`: `col-span-1 md:col-span-2 rounded-xl md:rounded-2xl overflow-hidden relative min-h-[200px] md:min-h-0`
- Animated: `s2Reveal.getAnimStyle(3)`
- Inner container: `absolute inset-0 z-10 flex flex-wrap md:flex-nowrap gap-1.5 md:gap-2 p-2 md:p-3`
- 4 service sub-cards mapped from `services` array:
  - Container: `flex-1 min-w-[calc(50%-4px)] md:min-w-0 rounded-xl md:rounded-2xl p-3 md:p-5 flex flex-col justify-between`
  - Active: `bg-white/90 backdrop-blur-md`
  - Inactive: `bg-white/20 backdrop-blur-xl`
  - Service name: `<h3>` with `text-xl md:text-4xl font-bold leading-[1.05] whitespace-pre-line`, color: active=`text-black`, inactive=`text-white`
  - Number badge (if `svc.num` exists): `self-end w-8 h-8 md:w-12 md:h-12 rounded-full border flex items-center justify-center text-xs md:text-sm font-semibold`
    - Active: `border-black text-black`
    - Inactive: `border-white text-white`

---

### SECTION 3 - IMPLANT DENTISTRY

**Container:** `<section>`, `min-h-screen md:h-screen w-full overflow-hidden flex flex-col`, `pt-1.5 md:pt-2 px-3 md:px-5 pb-1.5 md:pb-2 gap-1.5 md:gap-2`

Attach `s3Reveal.containerRef` to this element.

Does NOT use MaskedCard technique. Uses regular `<img>` tags and solid backgrounds.

**Grid:** `flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-1.5 md:gap-2`

### LEFT COLUMN: `flex flex-col gap-1.5 md:gap-2`

**1. Heading Card:**
- `<div>`: `rounded-xl md:rounded-2xl bg-stone-50 p-5 md:p-7 flex flex-col justify-between flex-[1.2] min-h-[180px] md:min-h-0`
- Animated: `s3Reveal.getAnimStyle(0)`
- Heading: `<h2>` with `text-[clamp(3rem,7vw,6.5rem)] font-bold leading-[0.95] text-black`, content: "Implant" `<br/>` "Dentistry"
- Subtitle: `<p>` with `text-xs md:text-sm font-semibold text-black`, text "Restore Missing Teeth"

**2. Two Image Cards (side by side):**
- Wrapper: `<div>` with `flex gap-1.5 md:gap-2 flex-1 min-h-[140px] md:min-h-0`
- Animated: `s3Reveal.getAnimStyle(1)`
- Left image: `<div className="flex-1 rounded-xl md:rounded-2xl overflow-hidden"><img src={SECTION3_IMG1} alt="Dental implant procedure" className="w-full h-full object-cover" /></div>`
- Right image: `<div className="flex-1 rounded-xl md:rounded-2xl overflow-hidden"><img src={SECTION3_IMG2} alt="Dental restoration" className="w-full h-full object-cover" /></div>`

**3. Consultation Card:**
- `<div>`: `rounded-xl md:rounded-2xl bg-zinc-200 p-5 md:p-7 flex items-end justify-between flex-[0.8] min-h-[160px] md:min-h-0`
- Animated: `s3Reveal.getAnimStyle(2)`
- Left content block:
  - Label: `<p>` with `text-xs md:text-sm font-semibold text-black mb-2 md:mb-3`, text "Consultation"
  - Heading: `<h3>` with `text-xl md:text-3xl font-bold text-black leading-6 md:leading-8`, content: "Dental" `<br/>` "Restoration" `<br/>` "Services"
- Button: `px-5 py-3 md:px-8 md:py-5 bg-white rounded-full text-black text-base md:text-xl font-bold hover:scale-105 transition-transform`, text "Book Online"

### RIGHT COLUMN: Single tall image card

- `<div>`: `rounded-xl md:rounded-2xl overflow-hidden relative min-h-[350px] md:min-h-0`
- Animated: `s3Reveal.getAnimStyle(3)`
- Background image: `<img src={SECTION3_BG} alt="Smiling patient" className="w-full h-full object-cover" />`
- **Overlay container:** `absolute bottom-3 left-3 right-3 md:bottom-5 md:left-5 md:right-5 flex gap-1.5 md:gap-2`

**Overlay Card 1 (white, left):**
- `flex-1 bg-white rounded-xl md:rounded-2xl p-3 md:p-5 flex flex-col justify-between h-36 md:h-52`
- Heading: `<h4>` with `text-lg md:text-2xl font-bold text-black leading-5 md:leading-7`, content: "The Process" `<br/>` "of Installing" `<br/>` "Implants"
- Arrow icon: `self-end w-9 h-9 md:w-12 md:h-12 rounded-full border border-black flex items-center justify-center`
  - SVG: `width="14" height="14" viewBox="0 0 14 14" fill="none"`, class `rotate-[-45deg]`
  - Path: `d="M1 7h12m0 0L8 2m5 5L8 12"` with `stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"`

**Overlay Card 2 (glass, right):**
- `flex-1 bg-white/20 backdrop-blur-xl rounded-xl md:rounded-2xl p-3 md:p-5 flex flex-col justify-between h-36 md:h-52`
- Heading: `<h4>` with `text-lg md:text-2xl font-bold text-white leading-5 md:leading-7`, content: "Caring" `<br/>` "for Dental" `<br/>` "Implants"
- Arrow icon: `self-end w-9 h-9 md:w-12 md:h-12 rounded-full border border-white flex items-center justify-center`
  - Same SVG as above but with added class `text-white`

---

### OUTER WRAPPER

The entire app is wrapped in `<div className="bg-white">` containing:
1. `{showSplash && <SplashScreen />}` (conditionally rendered)
2. `<Navbar />`
3. Section 1
4. Section 2
5. Section 3

---

### KEY DESIGN RULES

- **Spacing between sections:** Only `pb-1.5 md:pb-2` on each section and `pt-1.5 md:pt-2` on sections 2 and 3 -- virtually seamless.
- **Border radius:** All cards use `rounded-xl md:rounded-2xl` with `overflow-hidden`.
- **Color palette:** Strictly black, white, and translucent white (`bg-white/20`, `bg-white/90`) with `backdrop-blur-md` or `backdrop-blur-xl`.
- **Background fills:** `bg-stone-50` and `bg-zinc-200` for Section 3 solid cards.
- **Typography:** Heavy bold/extrabold, `clamp()` for responsive headings, extremely tight leading (0.79, 0.9, 0.95, 1.05).
- **Interactions:** `hover:scale-105 transition-transform` on CTA buttons.
- **Responsive:** Single `md:` (768px) breakpoint. Stacked on mobile, grid on desktop.
- **No external packages** beyond React and Tailwind.

## Innovation — Landing Page [sites/innovation-landing]

- Preview: https://motionsites.ai/assets/hero-innovation-preview-BerBJHh1.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/innovation-landing.gif

RECREATION PROMPT

Build a single-page landing site using React + TypeScript + Vite + Tailwind CSS + framer-motion + lucide-react. The entire page has a bg-black background. The font loaded via Google Fonts is Instrument Serif (italic and regular). Import it in index.css:


@import url('https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap');
LIQUID GLASS CSS (in index.css, inside @layer components)
Create a reusable .liquid-glass class used on every glass element:


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
SECTION 1 -- HERO (full-viewport, in Index.tsx)
Full-screen (min-h-screen) container with overflow-hidden relative flex flex-col.

Background video: absolute, covers the entire viewport (absolute inset-0 w-full h-full object-cover object-bottom). URL:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_074625_a81f018a-956b-43fb-9aee-4d1508e30e6a.mp4
Attributes: muted, autoPlay, playsInline, preload="auto". Starts at opacity: 0.

Video fade logic (vanilla JS via refs, no CSS transitions):

On canplay: play the video, then animate opacity from 0 to 1 over 500ms using requestAnimationFrame.
On timeupdate: when remaining time <= 0.55s, animate opacity from current to 0 over 500ms.
On ended: set opacity to 0, wait 100ms, reset currentTime to 0, play again, fade back to 1 over 500ms.
This creates a seamless loop with smooth crossfade to black between plays.
Navbar (relative z-20, px-6 py-6):

A liquid-glass rounded-full pill, max-w-5xl mx-auto, px-6 py-3, flex between left/right.
Left: Globe icon (24px, white) + "Asme" text (white, font-semibold, text-lg). Hidden on mobile: nav links "Features", "Pricing", "About" (text-white/80 hover:text-white text-sm font-medium, gap-8 ml-8).
Right: "Sign Up" text button (white, text-sm, font-medium) + "Login" button (liquid-glass rounded-full px-6 py-2, white text-sm font-medium).
Hero content (relative z-10, flex-1 flex flex-col items-center justify-center, px-6 py-12 text-center, -translate-y-[20%]):

Heading: text-7xl md:text-8xl lg:text-9xl, white, tracking-tight whitespace-nowrap, font-family 'Instrument Serif', serif. Text: Know it then <em className="italic">all</em>.
Email input: max-w-xl w-full. A liquid-glass rounded-full pill with pl-6 pr-2 py-2 flex items-center gap-3. Inside: transparent <input> with placeholder "Enter your email" (text-white placeholder:text-white/40). A white circular submit button (bg-white rounded-full p-3 text-black) containing ArrowRight icon (20px).
Subtitle: text-white text-sm leading-relaxed px-4. Text: "Stay updated with the latest news and insights. Subscribe to our newsletter today and never miss out on exciting updates."
Manifesto button: liquid-glass rounded-full px-8 py-3 text-white text-sm font-medium hover:bg-white/5 transition-colors.
Social icons footer (relative z-10, flex justify-center gap-4 pb-12):

Three liquid-glass rounded-full p-4 buttons for Instagram, Twitter, Globe icons (20px). text-white/80 hover:text-white hover:bg-white/5 transition-all.
SECTION 2 -- ABOUT SECTION (separate component AboutSection.tsx)
Uses framer-motion useInView (ref, { once: true, margin: "-100px" }).
bg-black pt-32 md:pt-44 pb-10 md:pb-14 px-6 overflow-hidden.
Subtle radial gradient overlay: bg-[radial-gradient(ellipse_at_top,_rgba(255,255,255,0.03)_0%,_transparent_70%)].
Label: "About Us" -- text-white/40 text-sm tracking-widest uppercase. Animates: opacity: 0, y: 20 -> opacity: 1, y: 0, duration 0.6.
Heading: text-4xl md:text-6xl lg:text-7xl text-white leading-[1.1] tracking-tight. Animates: opacity: 0, y: 40 -> opacity: 1, y: 0, duration 0.8, delay 0.1. Text structure:
Pioneering then ideas (Instrument Serif italic, text-white/60) for
Line break (hidden on mobile)
minds that then create, build, and inspire. (all Instrument Serif italic, text-white/60)
SECTION 3 -- FEATURED VIDEO (separate component FeaturedVideoSection.tsx)
bg-black pt-6 md:pt-10 pb-20 md:pb-32 px-6 overflow-hidden. Max-w-6xl.
A rounded-3xl overflow-hidden aspect-video container that animates opacity: 0, y: 60 -> opacity: 1, y: 0, duration 0.9.
Video: w-full h-full object-cover, muted, autoPlay, loop, playsInline, preload="auto". URL:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260402_054547_9875cfc5-155a-4229-8ec8-b7ba7125cbf8.mp4
Gradient overlay on video: bg-gradient-to-t from-black/60 via-transparent to-transparent.
Bottom overlay content (absolute bottom-0 left-0 right-0 p-6 md:p-10):
Flex row on desktop, column on mobile.
Left: a liquid-glass rounded-2xl p-6 md:p-8 max-w-md card. Label "Our Approach" (text-white/50 text-xs tracking-widest uppercase mb-3). Body text (text-white text-sm md:text-base leading-relaxed): "We believe in the power of curiosity-driven exploration. Every project starts with a question, and every answer opens a new door to innovation."
Right: "Explore more" button (liquid-glass rounded-full px-8 py-3, white text-sm font-medium) with whileHover={{ scale: 1.05 }} and whileTap={{ scale: 0.95 }}.
SECTION 4 -- PHILOSOPHY / INNOVATION x VISION (separate component PhilosophySection.tsx)
bg-black py-28 md:py-40 px-6 overflow-hidden. Max-w-6xl.
Heading: text-5xl md:text-7xl lg:text-8xl text-white tracking-tight mb-16 md:mb-24. Animates opacity: 0, y: 40 -> opacity: 1, y: 0, duration 0.8. Text: Innovation then x in Instrument Serif italic text-white/40, then Vision.
Two-column grid (grid-cols-1 md:grid-cols-2 gap-8 md:gap-12):
Left: Video in rounded-3xl overflow-hidden aspect-[4/3]. Animates from opacity: 0, x: -40. URL:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260307_083826_e938b29f-a43a-41ec-a153-3d4730578ab8.mp4
muted, autoPlay, loop, playsInline, preload="auto".
Right: Animates from opacity: 0, x: 40. Two text blocks separated by a w-full h-px bg-white/10 divider.
Block 1: Label "Choose your space" (text-white/40 text-xs tracking-widest uppercase mb-4). Body (text-white/70 text-base md:text-lg leading-relaxed): "Every meaningful breakthrough begins at the intersection of disciplined strategy and remarkable creative vision. We operate at that crossroads, turning bold thinking into tangible outcomes that move people and reshape industries."
Block 2: Label "Shape the future". Body: "We believe that the best work emerges when curiosity meets conviction. Our process is designed to uncover hidden opportunities and translate them into experiences that resonate long after the first impression."
SECTION 5 -- SERVICES / WHAT WE DO (separate component ServicesSection.tsx)
bg-black py-28 md:py-40 px-6 overflow-hidden. Max-w-6xl.
Subtle radial gradient: bg-[radial-gradient(ellipse_at_center,_rgba(255,255,255,0.02)_0%,_transparent_60%)].
Header row: flex between "What we do" (text-3xl md:text-5xl text-white tracking-tight) and "Our services" label (text-white/40 text-sm, hidden on mobile). Animates opacity: 0, y: 30 -> visible, duration 0.7.
Two-card grid (grid-cols-1 md:grid-cols-2 gap-6 md:gap-8):
Each card: liquid-glass rounded-3xl overflow-hidden with group class. Animates opacity: 0, y: 50 -> visible, duration 0.8, staggered by 0.15s.
Card video area: aspect-video, object-cover, transition-transform duration-700 group-hover:scale-105. Gradient overlay: bg-gradient-to-t from-black/40 to-transparent.
Card body (p-6 md:p-8): tag label (uppercase, tracking-widest, text-white/40 text-xs), ArrowUpRight icon in a liquid-glass rounded-full p-2 circle, title (text-white text-xl md:text-2xl mb-3 tracking-tight), description (text-white/50 text-sm leading-relaxed).
Card 1: Video URL:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131748_f2ca2a28-fed7-44c8-b9a9-bd9acdd5ec31.mp4
Tag: "Strategy". Title: "Research & Insight". Description: "We dig deep into data, culture, and human behavior to surface the insights that drive meaningful, lasting change."
Card 2: Video URL:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_151826_c7218672-6e92-402c-9e45-f1e0f454bdc4.mp4
Tag: "Craft". Title: "Design & Execution". Description: "From concept to launch, we obsess over every detail to deliver experiences that feel effortless and look extraordinary."

## Investment Gate — Landing Page [sites/investment-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(9).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/investment-hero.webp

Create a luxury real estate landing page called "VELORA" using React, TypeScript, Vite, and Tailwind CSS. The page has 3 main parts: a Splash Screen, a Hero Section with a morphing bottom navbar and popup menu, and a Scroll-Driven Gallery Overlay. Use `lucide-react` for icons. No other UI libraries.

---

### FONTS

Load these two fonts in `index.html` `<head>`:

1. **Haboro Norm Regular** (serif, used for brand name and headings):
   ```html
   <link href="https://db.onlinewebfonts.com/c/cc69fe194f7ed41628d4628f37a10a21?family=Haboro+Norm+Regular" rel="stylesheet">
   ```

2. **Geist** (sans-serif, weights 300/400/500/600, used for body text and UI elements):
   ```html
   <link href="https://fonts.googleapis.com/css2?family=Geist:wght@300;400;500;600&display=swap" rel="stylesheet">
   ```

Register them in `tailwind.config.js`:
```js
fontFamily: {
  haboro: ['"Haboro Norm Regular"', 'serif'],
  geist: ['Geist', 'sans-serif'],
}
```

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

body {
  font-family: 'Geist', sans-serif;
  overflow-x: hidden;
}

@keyframes menuSlideUp {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(40px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0) scale(1);
  }
}

@keyframes menuSlideDown {
  from {
    opacity: 1;
    transform: translateX(-50%) translateY(0) scale(1);
  }
  to {
    opacity: 0;
    transform: translateX(-50%) translateY(40px) scale(0.95);
  }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes fadeOut {
  from { opacity: 1; }
  to { opacity: 0; }
}

@keyframes menuItemSlide {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.menu-open {
  animation: menuSlideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

.menu-close {
  animation: menuSlideDown 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

.menu-overlay-open {
  animation: fadeIn 0.3s ease forwards;
}

.menu-overlay-close {
  animation: fadeOut 0.3s ease forwards;
}

.menu-item-enter {
  animation: menuItemSlide 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}
```

---

### SECTION 1: SPLASH SCREEN (`SplashScreen.tsx`)

A fixed full-screen black overlay (`z-[9999]`) that plays a video once. When the video ends, the screen fades out over 700ms and then unmounts.

- **Container**: `fixed inset-0 z-[9999] bg-black flex items-center`, with a `transition-opacity duration-700` that goes to `opacity-0` when fading.
- **Video**: Absolutely positioned at `top-[20%] left-[30%]` on mobile, `md:left-[55%] md:-translate-x-1/2` on desktop. Sizes: `w-[320px]` mobile, `sm:w-[390px]`, `md:w-[750px]`. `aspect-video object-contain`. Autoplay, muted, playsInline. No loop.
- **Video URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260624_205729_0e6fae4e-5fc0-42d0-b49a-b88d85ede0b2.mp4`
- **Behavior**: On `onEnded`, set fading state to true. After 800ms timeout, call `onComplete` to remove the splash.

---

### SECTION 2: HERO SECTION (in `App.tsx`)

The entire hero is a `<section>` with `fixed inset-0 h-screen w-full overflow-hidden z-0`.

### 2A. Video Background
- Full-screen looping video: `absolute inset-0 w-full h-full object-cover`
- **Video URL**: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260624_210218_173f8eba-17ff-4e27-972b-d128af25bf49.mp4`
- autoPlay, muted, loop, playsInline

### 2B. Top Navbar
- `relative z-10 flex items-center justify-between px-4 sm:px-6 md:px-12 pt-4 sm:pt-6 md:pt-8`
- Left: empty spacer `w-8 sm:w-32 md:w-40`
- Center: Brand name "VELORA" in `font-haboro text-white text-lg sm:text-xl md:text-2xl tracking-[0.2em] uppercase`
- Right: spacer same width containing a "GET IN TOUCH" button (`hidden sm:block bg-white text-black px-4 md:px-6 py-2 md:py-2.5 text-xs md:text-sm font-geist font-medium tracking-wide hover:bg-white/90 transition-colors duration-200`)

### 2C. Hero Heading (centered)
- Container: `flex-1 flex items-center justify-center px-4 sm:px-6`
- Text: `font-haboro text-white text-center text-2xl sm:text-3xl md:text-5xl lg:text-6xl xl:text-7xl leading-[1.1] tracking-wide uppercase max-w-5xl`
- Content:
  ```
  PREMIUM REAL ESTATE FOR
  INVESTORS BEYOND OWNERSHIP
  ```
  (line break between the two lines)

### 2D. Bottom Morphing Navbar
- Centered at the bottom: `flex justify-center pb-4 sm:pb-6 md:pb-10`
- The bar is a black rectangle that morphs between two states:
  - **Expanded** (menu closed): width `280px`, max-width `85vw`. Contains:
    - Left: `Home` icon from lucide-react (`w-5 h-5 text-amber-500`)
    - Center: "HOME" text (`font-geist text-white text-sm tracking-[0.15em] uppercase font-medium`)
    - Right: Hamburger button (two horizontal white lines, `w-6 h-[1.5px] bg-white`, one at `top-[8px]` and one at `top-[15px]`)
  - **Collapsed** (menu open): morphs to `56px` square showing an X close button (two `w-5 h-[1.5px] bg-white` lines rotated 45deg and -45deg)
- Transition: `transition-all duration-500 ease-[cubic-bezier(0.16,1,0.3,1)]`

### 2E. Menu Popup
- Triggered by hamburger click.
- **Overlay**: `fixed inset-0 z-20`, with `menu-overlay-open`/`menu-overlay-close` CSS animation classes.
- **Menu panel**: `fixed z-30 left-1/2 bottom-[80px] sm:bottom-[100px] md:bottom-[120px] w-[92vw] max-w-[480px] bg-black rounded-lg p-6 sm:p-8 md:p-10`
  - Uses `menu-open`/`menu-close` CSS classes (translateX(-50%) centered via those animations).
  - Header: "MENU" label (`font-geist text-neutral-400 text-xs tracking-[0.2em] uppercase mb-6`)
  - Menu items list (`space-y-1`): `['About', 'Properties', 'Work', 'Partnership', 'Contact']`
    - Each item: `font-haboro text-white text-2xl md:text-3xl lg:text-4xl hover:text-neutral-300 transition-colors duration-200 block py-1`
    - Each `<li>` has class `menu-item-enter` with staggered `animationDelay: ${i * 60 + 100}ms` and initial `opacity: 0`
  - Footer section: `mt-8 pt-6 border-t border-neutral-700/50 space-y-3`
    - "Private line" row: label `font-geist text-neutral-400 text-sm`, value `font-geist text-white text-sm` showing `+44 020 8156 7290`
    - "Email" row: same style, showing `hello@velora.com`
  - CTA button: `w-full mt-6 bg-white text-black py-3 font-geist text-sm font-medium tracking-wide hover:bg-neutral-100 transition-colors duration-200` with text "GET IN TOUCH"
- **Close behavior**: Sets `isClosing` to true, waits 300ms, then unmounts menu.

---

### SECTION 3: SCROLL-DRIVEN GALLERY (`ScrollGallery.tsx`)

The entire app container has `bg-black h-[900vh]` to create scroll space. The gallery is a `fixed inset-0 z-10 pointer-events-none` overlay that animates based on scroll progress (0 to 1).

### Scroll Phase Breakdown:

| Progress Range | What Happens |
|---|---|
| 0.00 - 0.06 | A thin black vertical line appears at center, grows from 0px to 50px wide, full height |
| 0.06 - 0.16 | The line expands width to fill the entire viewport (cubic ease-in-out) |
| 0.16 - 0.34 | Image 1 scales from center (scale 0.2 to 1.0, opacity fades in quickly) |
| 0.34 - 0.50 | Image 2 scales from center (same animation, stacked on top) |
| 0.50 - 0.64 | Image 3 scales from center (same animation, stacked on top) |
| 0.64 - 0.78 | Black text screen scales from center (same scale animation). Shows text: "Access exceptional real estate opportunities **worldwide.**" (worldwide in amber-500) |
| 0.78 - 0.90 | Background morphs from black to white (RGB interpolation). The black text fades out. |
| 0.90 - 1.00 | White screen with text: "Built on trust and **expertise.**" (expertise in amber-500) fades in |

### Image Scale Animation Formula:
- `scale = 0.2 + progress * 0.8`
- `opacity = Math.min(1, progress * 4)` (quick fade-in)

### Image Styling:
- Mobile: `w-[85vw] h-auto max-w-[900px] aspect-[16/10]` with `rounded-lg`
- Desktop (`sm:`): `w-full h-full max-w-none aspect-auto` with no rounding (`rounded-none`)
- All images: `object-cover`

### Image URLs (in order):
1. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_211418_dbb8d807-3cfb-4c26-b1df-02fb0c23cc7d.png&w=1280&q=85`
2. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_211445_ba965dcd-97d6-4644-b390-d4744078ec6c.png&w=1280&q=85`
3. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_212326_f5e78786-d7bb-40c5-abac-cd3c0be37d90.png&w=1280&q=85`

### Text Screens Typography:
- Both text screens: `font-geist font-light text-2xl sm:text-3xl md:text-4xl lg:text-5xl text-center leading-[1.3] max-w-3xl px-6`
- Black screen text color: `text-white`
- White screen text color: `text-neutral-800`
- Accent word color: `text-amber-500`

### Width Expansion Easing (Phase 2):
```js
const eased = expandProgress < 0.5
  ? 4 * expandProgress * expandProgress * expandProgress
  : 1 - Math.pow(-2 * expandProgress + 2, 3) / 2;
```

### Morph (black to white) formula:
```js
backgroundColor: `rgb(${Math.round(morphProgress * 255)}, ${Math.round(morphProgress * 255)}, ${Math.round(morphProgress * 255)})`
```

---

### PAGE STRUCTURE

```
App.tsx
  - bg-black h-[900vh]
  - SplashScreen (conditional, unmounts after video ends)
  - Hero Section (fixed, z-0)
    - Video background (looping)
    - Navbar + Heading + Bottom bar
    - Menu popup (conditional)
  - ScrollGallery (fixed, z-10, pointer-events-none)
```

---

### TECH STACK
- Vite + React 18 + TypeScript
- Tailwind CSS 3.4
- lucide-react (only the `Home` icon)
- No other dependencies needed

---

### PAGE TITLE
`VELORA - Premium Real Estate`

## Layered Depth — Landing Page [sites/layered-depth]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(43).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/layered-depth.webp

Create a React + Vite + TypeScript + Tailwind CSS landing page for an architecture studio called "Qelora". The page has exactly two sections: a Hero and a Section 2. The entire site uses inline styles (no Tailwind utility classes in JSX -- Tailwind is only used for base reset). Use only `react`, `react-dom`, and `lucide-react` as dependencies (icons are all inline SVGs here, lucide is not actually used in this page).

---

FONTS

Load these three custom fonts in `index.html` `<head>`:

```html
<link href="https://db.onlinewebfonts.com/c/076f8c5b3b67616658dd1e4e9bac62ec?family=Zimula+Trial+Med" rel="stylesheet">
<link href="https://db.onlinewebfonts.com/c/08d8ca53f66ab5b48659912fa0136b78?family=Zimula+Trial+Bd" rel="stylesheet">
```

Also import in `index.css`:
```css
@import url('https://db.onlinewebfonts.com/c/46024824a3dd3309c3a7f46f4f1283ba?family=Zimula+Trial+Reg');
```

Font usage:
- Body / default: `'Zimula Trial Med', sans-serif`
- Bold / logo / hero text: `'Zimula Trial Bd', sans-serif`
- The `Reg` import is available but Med is the primary weight used everywhere

---

GLOBAL CSS (`index.css`)

```css
@import url('https://db.onlinewebfonts.com/c/46024824a3dd3309c3a7f46f4f1283ba?family=Zimula+Trial+Reg');

@tailwind base;
@tailwind components;
@tailwind utilities;

*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html {
  scroll-behavior: smooth;
}

body {
  font-family: 'Zimula Trial Med', sans-serif;
  background: #0e0c0a;
  overflow-x: hidden;
}

::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: #0e0c0a; }
::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); border-radius: 3px; }
```

---

COLOR PALETTE

- Dark background: `#0e0c0a`
- Primary text: `#241f21`, `#282425`, `#2a2420`
- White: `#fff`
- Dark accent: `#100e0c`
- Warm transparent overlays: `rgba(235, 230, 218, 0.12)`, `rgba(242, 238, 230, 0.38)`
- Frosted glass backgrounds: `rgba(248,245,240,0.72)`, `rgba(248,245,240,0.88)`, `rgba(248,245,240,0.92)`, `rgba(248,245,240,0.96)`

---

ASSET URLs (Cloudinary, not CloudFront)

Videos:
- Background video (Hero): `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/f_auto/v1779808200/bg-video_xsmysw.mp4`
- Bird enter animation: `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/v1779808206/bird-entrada_e72qt7.webm`
- Bird idle 1: `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/v1779808282/bird-idle_fzjami.webm`
- Bird idle 2: `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/v1779808284/bird-idle2_rajmgo.webm`
- Bird leave animation: `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/v1779808286/bird-saida_ifroz1.webm`
- Background video (Section 2): `https://res.cloudinary.com/dy5er7kv5/video/upload/q_auto/f_auto/v1779835701/bg-2-video_sgbpqt.mp4`

Images:
- Q logo (unused but declared): `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779808187/q-logo_isvugc.png`
- Center sculpture/slab: `https://res.cloudinary.com/dy5er7kv5/image/upload/q_auto/f_auto/v1779854565/slab_v1_kb4vqk.png`
- CTA card photo (Pexels): `https://images.pexels.com/photos/3184465/pexels-photo-3184465.jpeg?auto=compress&cs=tinysrgb&w=400`

---

APP STRUCTURE

```
src/
  main.tsx      -> StrictMode, renders
  App.tsx       ->  then , no routing
  Hero.tsx      -> Hero section component
  Section2.tsx  -> Second section component
  index.css     -> Global styles
```

---

SECTION 1: HERO (`Hero.tsx`)

Container: `position: relative`, `width: 100%`, `minHeight: 100vh`, `overflow: visible`, `fontFamily: 'Zimula Trial Med', sans-serif`.

Responsive breakpoint: `isMobile = window.innerWidth < 768`, checked on mount and resize.

Layer 1 -- Background Video (z-index: 0)
- `` with `autoPlay muted loop playsInline`
- `position: absolute`, `inset: 0`, `width: 100%`, `height: 100vh`, `objectFit: cover`
- Source: `BG_VIDEO` URL

### Layer 2 -- Warm Overlay (z-index: 1)
- A div covering the hero with `background: rgba(235, 230, 218, 0.12)`, `height: 100vh`, `pointerEvents: none`

### Layer 3 -- Bird Animation System (z-index: 8)
- Container: `position: absolute`, `top: 0`, `left: 0`, `width: 100%`, `height: 100vh`, `pointerEvents: none`, `aria-hidden`
- Contains 4 `` elements (enter, idle1, idle2, leave), each toggled visible/hidden via `display` property
- **Desktop:** Each video is `position: absolute`, `inset: 0`, `width: 100%`, `height: 100%`, `objectFit: cover`
- **Mobile:** Each video is `position: absolute`, `top: 50%`, `left: 0`, `transform: translateY(-50%)`, `width: 100%`, `height: auto` (full width, auto height, vertically centered)
- **State machine:** Type `'enter' | 'idle1' | 'idle2' | 'leave' | 'hidden'`
  - On page load: play `enter` video
  - When `enter` ends: transition to `idle1`
  - When `idle1` ends: transition to `idle2`
  - When `idle2` ends: transition back to `idle1` (infinite loop)
  - **On scroll down** (past 10px threshold): pause all idle/enter videos, reset their `currentTime` to 0, play `leave` video
  - **On scroll back to top** (below 10px): pause leave video, reset, play `enter` video again
- Uses both React state and refs (`birdStateRef`) to avoid stale closures in scroll handlers
- All videos are preloaded with `.load()` on mount
- The `playVideo` helper sets `currentTime = 0`, checks `readyState >= 2`, then plays (or waits for `canplay` event)

### Layer 4 -- Center Brand Text "Qelora" (z-index: 5)
- Absolutely positioned container filling `100vh`, `display: flex`, `alignItems: center`, `justifyContent: center`, `pointerEvents: none`
- Text: `"Qelora"` in `'Zimula Trial Bd', sans-serif`
- Font size: mobile `26vw`, desktop `22vw`
- `letterSpacing: -0.05em`, `color: #241f21`, `lineHeight: 1`
- `marginBottom`: mobile `8vh`, desktop `12vh`

### Layer 5 -- Sculpture Image (z-index: 5)
- `` with `position: absolute`, `top: 50%`, `left: 50%`
- `transform: translateX(-50%) translateY(${-heroScroll  0.3}px)` -- parallax that moves UP as user scrolls down
- Width: mobile `220vw`, desktop `160vw`; `height: auto`
- `pointerEvents: none`, `willChange: transform`

### Layer 6 -- Fixed Navbar (z-index: 100)
- `position: fixed`, `top: 0`, full width
- Padding: mobile `16px 20px`, desktop `20px 36px`
- **Left:** Brand name "Qelora" with registered trademark superscript. Font: `'Zimula Trial Bd'`, size: mobile `20px`, desktop `24px`, `letterSpacing: -0.03em`, `color: #241f21`. The `(R)` sup has `fontSize: 0.4em`, `verticalAlign: super`
- **Right (desktop):** `NavPills` component -- a row of pill buttons for `['Projects', 'Studio', 'Responsibility', 'Archive']` plus an `EN` language selector
  - Each pill: `background: rgba(248,245,240,0.92)`, `borderRadius: 12px`, `padding: 13px 22px 8px`, `height: 40px`, `fontSize: 13px`, `textTransform: uppercase`, `letterSpacing: 0.07em`, `color: #241f21`
  - Active pill has `fontWeight: 700` and a 3px round dot at `bottom: 3px`, centered
  - Non-active: `fontWeight: 500`
  - Language pill: separate rounded capsule (`borderRadius: 100px`), `padding: 8px 14px`, `background: rgba(248,245,240,0.88)`, `backdropFilter: blur(12px)`, `boxShadow: 0 2px 20px rgba(0,0,0,0.1)`, contains "EN" text and a chevron-down SVG
- **Right (mobile):** Hamburger button, `42x42px`, `borderRadius: 100px`, same frosted glass style. Shows X icon when open, 3-line hamburger when closed

### Layer 7 -- Mobile Dropdown Menu (z-index: 99)
- `position: fixed`, `top: 70px`, `left: 16px`, `right: 16px`
- `background: rgba(248,245,240,0.96)`, `backdropFilter: blur(16px)`, `borderRadius: 18px`, `padding: 8px`, `boxShadow: 0 8px 40px rgba(0,0,0,0.14)`
- Each menu item: full-width button, `padding: 14px 20px`, `fontSize: 13px`, uppercase, `letterSpacing: 0.07em`, `borderBottom: 1px solid rgba(40,36,37,0.08)`
- Bottom: EN language selector row

### Layer 8 -- Bottom Panels (z-index: 20)
- `bottom` is calculated as: `bottomOffset + heroScroll  0.5` where `bottomOffset` is 24px on mobile, 36px on desktop. This creates a parallax push-down effect as user scrolls.

**Desktop layout (side-by-side):**

- **Bottom-left panel:** `position: absolute`, `left: 36px`, `borderRadius: 18px`, `padding: 22px 28px`, `maxWidth: 270px`
  - Headline: `"Designing places\nbeyond\nwhat's expected"` -- `fontSize: clamp(17px, 2vw, 24px)`, `lineHeight: 1.28`, `color: #282425`, `letterSpacing: -0.01em`
  - Below: 1px border-top divider (`rgba(40,36,37,0.2)`), then "EXPLORE OUR APPROACH" link with down-arrow SVG. `fontSize: 11px`, uppercase, `letterSpacing: 0.1em`

- **Bottom-right panel:** `position: absolute`, `right: 36px`, `borderRadius: 18px`, `width: clamp(210px, 21vw, 290px)`, `height: 180px`, `overflow: hidden`
  - Background: Pexels photo covering the entire card
  - Dark gradient overlay: `linear-gradient(to bottom, rgba(16,14,12,0.55) 0%, transparent 60%)`
  - Top text: `"Every lasting space begins\nwith a quiet dialogue."` -- `color: #fff`, `fontSize: 13px`, `lineHeight: 1.35`
  - Bottom: inline flex with a white circle (envelope SVG icon, 36x36px, `borderRadius: 12px`) and a white "START A PROJECT" button (`fontSize: 11px`, uppercase, `letterSpacing: 0.07em`, `fontWeight: 700`, `borderRadius: 12px`, `height: 36px`)

**Mobile layout (stacked):**
- Single flex column container, `left: 20px`, `right: 20px`, `gap: 12px`
- **Top card:** Tagline panel with `background: rgba(248,245,240,0.72)`, `backdropFilter: blur(8px)`, `borderRadius: 16px`, `padding: 18px 20px`. Same text as desktop but single line: "Designing places beyond what's expected", `fontSize: 17px`. Same divider + "Explore our approach" link below.
- **Bottom card:** CTA card, `borderRadius: 16px`, `height: 120px`. Same structure as desktop right panel but adapted for mobile (text `fontSize: 12px`, same button row).

---

### SECTION 2 (`Section2.tsx`)

**Container:** `position: relative`, `width: 100%`, `minHeight: 100vh`, `display: flex`, `flexDirection: column`, `alignItems: center`, `justifyContent: center`, `overflow: hidden`, `fontFamily: 'Zimula Trial Med', sans-serif`

### Layer 1 -- Background Video (z-index: 0)
- `` with `autoPlay muted loop playsInline`, `position: absolute`, `inset: 0`, `width: 100%`, `height: 100%`, `objectFit: cover`
- Source: `BG_VIDEO_2` URL

### Layer 2 -- Warm Overlay (z-index: 1)
- `background: rgba(242, 238, 230, 0.38)`, `position: absolute`, `inset: 0`, `pointerEvents: none`

### Layer 3 -- Center Headline (z-index: 2)
- Absolutely positioned, `inset: 0`, flex centered, `pointerEvents: none`, `textAlign: center`, `padding: 0 24px`
- Text: `"What stands the\ntest of time is all\nthat guides the\nwork."` using `
` tags
- `fontSize: clamp(32px, 5.5vw, 80px)`, `lineHeight: 1.18`, `color: #2a2420`, `maxWidth: 780px`, `letterSpacing: -0.025em`, `fontWeight: 400`

### Layer 4 -- Bottom Element (z-index: 2)
- `position: absolute`, `bottom: clamp(24px, 4vh, 48px)`, full width, flex column centered, `padding: 0 24px`
- **Vertical line:** `width: 1px`, `height: 56px`, `background: rgba(42,36,32,0.25)`
- **Below (margin-top: 22px):** flex column centered, `gap: 14px`
  - **Map pin SVG:** 24x28px outline pin icon, `stroke: #2a2420`, `strokeWidth: 1.4`
  - **Subtext:** `"Civic bodies and private clients trust us to shape resilient communities and purposeful places."` -- `fontSize: clamp(11px, 1.4vw, 13px)`, `color: #2a2420`, `letterSpacing: 0.04em`, `lineHeight: 1.6`, `maxWidth: 340px`, `opacity: 0.75`

---

### KEY BEHAVIORS SUMMARY

1. **Bird animation state machine:** enter -> idle1 <-> idle2 loop; scroll triggers leave; scroll back triggers re-enter
2. **Parallax effects:** Sculpture image moves up with `translateY(-scrollY  0.3)`. Bottom panels push down with `bottom = offset + scrollY  0.5`
3. Responsive at 768px breakpoint: Nav collapses to hamburger, panels stack vertically, bird videos switch from cover-fill to width-100%/height-auto/vertically-centered, sculpture grows from 160vw to 220vw, brand text grows from 22vw to 26vw
4. All styling is inline -- no CSS classes in JSX, no Tailwind utility classes on elements
5. No third-party animation libraries -- all animations are native video playback + scroll-driven inline style changes via React state

## Liquid Glass Agency — Landing Page [sites/liquid-glass-agency]

- Preview: https://motionsites.ai/assets/hero-liquid-glass-agency-preview-Cr5Q9-lc.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/liquid-glass-agency.gif

Build a dark, premium, single-page landing page for an AI-powered web design agency using React + Vite + Tailwind CSS + shadcn/ui + Framer Motion (motion/react). The page has a luxury editorial aesthetic -- black backgrounds, white text, liquid glass (glassmorphism) effects, and cinematic video backgrounds.

FONTS
Import from Google Fonts:

https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Barlow:wght@300;400;500;600&display=swap
* Headings: Instrument Serif (italic) -- used via Tailwind class font-heading
* Body: Barlow (weights 300, 400, 500, 600) -- used via Tailwind class font-body
Tailwind config extends fontFamily:

heading: ["'Instrument Serif'", "serif"]
body: ["'Barlow'", "sans-serif"]

COLOR THEME (CSS custom properties, HSL format)

:root {
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
}

LIQUID GLASS CSS (the core visual effect)
Two utility classes defined in index.css under @layer components:
.liquid-glass (subtle):

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
.liquid-glass-strong (more prominent, used on CTA buttons):

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
The ::before pseudo-element creates a gradient border effect using the mask-composite trick (thin glowing border that fades in the middle).

ASSETS & MEDIA URLS
Hero background video (MP4, CloudFront):

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260307_083826_e938b29f-a43a-41ec-a153-3d4730578ab8.mp4
Poster image: /images/hero_bg.jpeg (local file in public/images/)
StartSection video (HLS via Mux):

https://stream.mux.com/9JXDljEVWYwWu01PUkAemafDugK89o01BR6zqJ3aS9u00A.m3u8
Stats section video (HLS via Mux, displayed desaturated):

https://stream.mux.com/NcU3HlHeF7CUL86azTTzpy3Tlb00d6iF3BmCdFslMJYM.m3u8
CTA/Footer section video (HLS via Mux):

https://stream.mux.com/8wrHPCX2dC3msyYU9ObwqNdm00u3ViXvOSHUMRYSEe5Q.m3u8
Feature GIFs (imported from src/assets/):
* feature-1.gif -- used in FeaturesChess row 1 (right side)
* feature-2.gif -- used in FeaturesChess row 2 (left side)
Logo icon: src/assets/logo-icon.png (12x12 Tailwind = h-12 w-12)

SECTION-BY-SECTION BREAKDOWN
1. NAVBAR (fixed, floating)
* Fixed position: fixed top-4 left-0 right-0 z-50, horizontal padding px-8 lg:px-16, vertical py-3
* Left: Logo image (h-12 w-12)
* Center (desktop only, hidden md:flex): Navigation links inside a liquid-glass rounded-full px-1.5 py-1 pill container
    * Links: "Home", "Services", "Work", "Process", "Pricing"
    * Each link: px-3 py-2 text-sm font-medium text-foreground/90 font-body
    * Last item: white solid button "Get Started" with ArrowUpRight icon, bg-white text-black rounded-full px-3.5 py-1.5 text-sm
2. HERO SECTION
* Container: relative overflow-visible, fixed height 1000px
* Background video: <video> tag with autoPlay, loop, muted, playsInline. Positioned absolute left-0 w-full h-auto object-contain z-0 with top: 20%. Source is the CloudFront MP4 URL. Poster is /images/hero_bg.jpeg.
* Dark overlay: absolute inset-0 bg-black/5 z-0
* Bottom gradient fade: absolute bottom-0, height 300px, linear-gradient(to bottom, transparent, black)
* Content (z-10, centered, paddingTop: 150px):
    * Badge pill: liquid-glass rounded-full px-1 py-1 with inner white "New" badge (bg-white text-black rounded-full px-3 py-1 text-xs font-semibold) and text "Introducing AI-powered web design."
    * Heading (BlurText component): "The Website Your Brand Deserves" -- text-6xl md:text-7xl lg:text-[5.5rem] font-heading italic text-foreground leading-[0.8] max-w-2xl tracking-[-4px], animated word-by-word from bottom with blur, delay 100ms
    * Subtext (motion.p): "Stunning design. Blazing performance. Built by AI, refined by experts. This is web design, wildly reimagined." -- blur-in animation, delay 0.8s, text-sm md:text-base text-white font-body font-light leading-tight
    * CTA buttons (motion.div, delay 1.1s):
        * "Get Started" -- liquid-glass-strong rounded-full px-5 py-2.5 with ArrowUpRight icon
        * "Watch the Film" -- text-only with Play icon (filled)
    * Partners bar at bottom (mt-auto pb-8 pt-16): "Trusted by the teams behind" liquid-glass pill, then 5 partner names rendered in text-2xl md:text-3xl font-heading italic text-white with gap-12 md:gap-16: Stripe, Vercel, Linear, Notion, Figma
3. BlurText COMPONENT (custom animated text)
* Splits text by words or letters
* Uses IntersectionObserver to trigger on scroll
* Each word/letter is a <motion.span> that animates from {filter: 'blur(10px)', opacity: 0, y: 50} (when direction=bottom) through {filter: 'blur(5px)', opacity: 0.5, y: -5} to {filter: 'blur(0px)', opacity: 1, y: 0}
* Staggered by index with configurable delay (default 200ms per element)
* Step duration 0.35s per keyframe step
4. START SECTION ("How It Works")
* Full-width section with HLS video background using hls.js library
* Video: autoPlay, loop, muted, playsInline, absolute inset-0 w-full h-full object-cover
* Top and bottom gradient fades (200px each, black to transparent)
* Content centered (z-10, minHeight 500px):
    * Badge: "How It Works" in liquid-glass rounded-full px-3.5 py-1
    * Heading: "You dream it. We ship it." -- text-4xl md:text-5xl lg:text-6xl font-heading italic tracking-tight leading-[0.9]
    * Subtext: "Share your vision. Our AI handles the rest--wireframes, design, code, launch. All in days, not quarters." -- text-white/60 font-body font-light text-sm md:text-base
    * CTA: "Get Started" liquid-glass-strong rounded-full px-6 py-3
5. FEATURES CHESS (alternating rows)
* Section header: "Capabilities" badge + "Pro features. Zero complexity." heading
* Row 1 (flex, content left / image right):
    * Title: "Designed to convert. Built to perform."
    * Body: "Every pixel is intentional. Our AI studies what works across thousands of top sites--then builds yours to outperform them all."
    * Button: "Learn more" liquid-glass-strong
    * Gif: https://motionsites.ai/assets/hero-finlytic-preview-CV9g0FHP.gif download and place inside liquid-glass rounded-2xl overflow-hidden
* Row 2 (flex-row-reverse, content right / image left):
    * Title: "It gets smarter. Automatically."
    * Body: "Your site evolves on its own. AI monitors every click, scroll, and conversion--then optimizes in real time. No manual updates. Ever."
    * Button: "See how it works" liquid-glass-strong
    * gif: https://motionsites.ai/assets/hero-wealth-preview-B70idl_u.gif download and place inside liquid-glass rounded-2xl overflow-hidden
6. FEATURES GRID ("Why Us")
* Section header: "Why Us" badge + "The difference is everything." heading
* 4-column grid (grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6), each card is liquid-glass rounded-2xl p-6:
    1. Icon: Zap -- "Days, Not Months" -- "Concept to launch at a pace that redefines fast. Because waiting isn't a strategy."
    2. Icon: Palette -- "Obsessively Crafted" -- "Every detail considered. Every element refined. Design so precise, it feels inevitable."
    3. Icon: BarChart3 -- "Built to Convert" -- "Layouts informed by data. Decisions backed by performance. Results you can measure."
    4. Icon: Shield -- "Secure by Default" -- "Enterprise-grade protection comes standard. SSL, DDoS mitigation, compliance. All included."
    * Each icon sits in a liquid-glass-strong rounded-full w-10 h-10 circle
7. STATS SECTION
* HLS video background (Mux URL), displayed with filter: saturate(0) (desaturated/B&W)
* Top and bottom gradient fades (200px each)
* Content: liquid-glass rounded-3xl p-12 md:p-16 card with 4-column grid:
    * "200+" / "Sites launched"
    * "98%" / "Client satisfaction"
    * "3.2x" / "More conversions"
    * "5 days" / "Average delivery"
    * Values: text-4xl md:text-5xl lg:text-6xl font-heading italic
    * Labels: text-white/60 font-body font-light text-sm
8. TESTIMONIALS
* Section header: "What They Say" badge + "Don't take our word for it." heading
* 3-column grid (md:grid-cols-3 gap-6), each card is liquid-glass rounded-2xl p-8:
    1. "A complete rebuild in five days. The result outperformed everything we'd spent months building before." -- Sarah Chen, CEO, Luminary
    2. "Conversions up 4x. That's not a typo. The design just works differently when it's built on real data." -- Marcus Webb, Head of Growth, Arcline
    3. "They didn't just design our site. They defined our brand. World-class doesn't begin to cover it." -- Elena Voss, Brand Director, Helix
    * Quote: text-white/80 font-body font-light text-sm italic
    * Name: text-white font-body font-medium text-sm
    * Role: text-white/50 font-body font-light text-xs
9. CTA + FOOTER
* HLS video background (Mux URL)
* Top and bottom gradient fades (200px each)
* Content (z-10, centered):
    * Heading: "Your next website starts here." -- text-5xl md:text-6xl lg:text-7xl font-heading italic leading-[0.85]
    * Subtext: "Book a free strategy call. See what AI-powered design can do. No commitment, no pressure. Just possibilities."
    * Two buttons:
        * "Book a Call" -- liquid-glass-strong rounded-full px-6 py-3
        * "View Pricing" -- bg-white text-black rounded-full px-6 py-3
    * Footer bar (mt-32 pt-8 border-t border-white/10):
        * Left: "(c) 2026 Studio. All rights reserved." text-white/40 text-xs
        * Right: "Privacy", "Terms", "Contact" links text-white/40 text-xs

KEY DEPENDENCIES

{
  "motion": "^12.35.0",
  "hls.js": "^1.6.15",
  "lucide-react": "^0.462.0",
  "react-router-dom": "^6.30.1"
}
Icons used from lucide-react: ArrowUpRight, Play, Zap, Palette, BarChart3, Shield

OVERALL PAGE STRUCTURE

<div bg-black>
  <div z-10>
    <Navbar />           -- fixed floating nav
    <Hero />             -- 1000px tall, CloudFront MP4 video bg
    <div bg-black>
      <StartSection />   -- HLS video bg, "How It Works"
      <FeaturesChess />  -- alternating text/gif rows
      <FeaturesGrid />   -- 4-card grid
      <Stats />          -- HLS video bg (desaturated), stats card
      <Testimonials />   -- 3-card grid
      <CtaFooter />      -- HLS video bg, CTA + footer
    </div>
  </div>
</div>

ANIMATION PATTERNS
1. BlurText (heading): Word-by-word stagger from bottom with gaussian blur dissolve, IntersectionObserver triggered
2. Hero subtext: motion.p with filter: blur(10px) -> blur(0px), opacity: 0 -> 1, y: 20 -> 0, delay 0.8s, duration 0.6s
3. Hero CTA buttons: Same blur-in pattern, delay 1.1s
4. All video backgrounds: autoPlay, loop, muted, playsInline with top/bottom black gradient fades (200px typically, 300px on hero bottom)

DESIGN PATTERNS USED THROUGHOUT
* Every section badge: liquid-glass rounded-full px-3.5 py-1 text-xs font-medium text-white font-body
* Every section heading: text-4xl md:text-5xl lg:text-6xl font-heading italic text-white tracking-tight leading-[0.9]
* Every body text: text-white/60 or text-white/70, font-body font-light text-sm md:text-base
* Primary CTA: liquid-glass-strong rounded-full with ArrowUpRight icon
* Secondary CTA: bg-white text-black rounded-full
* Card containers: liquid-glass rounded-2xl
* Video overlay fades: always linear-gradient(to bottom/top, black, transparent) with pointer-events-none

## Luxury Botanical — Landing Page [sites/luxury-botanical]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(36).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luxury-botanical.webp

<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Bentley — Beyond The Collection</title>

<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link
  href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Manrope:wght@300;400;500;600&family=Great+Vibes&display=swap"
  rel="stylesheet"
/>

<script src="https://cdn.tailwindcss.com"></script>
<script>
  tailwind.config = {
    theme: {
      extend: {
        fontFamily: {
          serif: ['"Instrument Serif"', "serif"],
          sans: ["Manrope", "sans-serif"],
          script: ['"Great Vibes"', "cursive"],
        },
      },
    },
  };
</script>

<style>
  *, *::before, *::after { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; background: #000; }
  body {
    font-family: "Manrope", ui-sans-serif, system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
  }

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

  @keyframes scrollArrow {
    0%   { transform: translateY(-6px); opacity: 0; }
    40%  { opacity: 1; }
    100% { transform: translateY(10px); opacity: 0; }
  }
  .scroll-arrow {
    animation: scrollArrow 1.6s ease-in-out infinite;
  }
</style>

<script src="https://unpkg.com/@babel/standalone@7.29.0/babel.min.js" integrity="sha384-m08KidiNqLdpJqLq95G/LEi8Qvjl/xUYll3QILypMoQ65QorJ9Lvtp2RXYGBFj1y" crossorigin="anonymous"></script>
<script type="module">
  import React from "https://esm.sh/react@18.3.1";
  import * as ReactDOMClient from "https://esm.sh/react-dom@18.3.1/client?deps=react@18.3.1";
  import * as FM from "https://esm.sh/framer-motion@11.18.2?deps=react@18.3.1,react-dom@18.3.1";
  window.React = React;
  window.ReactDOM = ReactDOMClient;
  window.FM = FM;
  window.__depsReady = true;
  window.dispatchEvent(new Event("deps-ready"));
</script>
</head>
<body>
  <div id="root"></div>

  <script type="text/babel" data-presets="react">
(function () {
  const start = () => {
    const { useRef, useState, useEffect, useMemo } = React;
    const { createRoot } = ReactDOM;
    const {
      motion,
      useScroll,
      useTransform,
      useMotionTemplate,
      useMotionValue,
      useAnimationFrame,
      animate
    } = window.FM;

    /* ============================================================
       OrbitImages
       ============================================================ */

    function generateEllipsePath(cx, cy, rx, ry) {
      return `M ${cx - rx} ${cy} A ${rx} ${ry} 0 1 0 ${cx + rx} ${cy} A ${rx} ${ry} 0 1 0 ${cx - rx} ${cy}`;
    }

    function OrbitItem({
      item,
      title,
      desc,
      index,
      totalItems,
      pathValue,
      itemSizeValue,
      rotationValue,
      progress,
      fill,
      scaleStrength,
      focalPoint = 50
    }) {
      const itemOffset = fill ? index / totalItems * 100 : 0;

      const offsetPercentage = useTransform(progress, (p) => {
        return ((p + itemOffset) % 100 + 100) % 100;
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
          targetScale = 0.4 + cosCurve * 0.6;
        } else {
          targetScale = 0.4;
        }
        return 1 - strength * (1 - targetScale);
      });

      const offsetPath = useMotionTemplate`path("${pathValue}")`;
      const zIndexMV = useTransform(itemScale, (s) => Math.round(s * 100));
      const counterRotate = useTransform(rotationValue, (r) => `rotate(${-r}deg)`);
      const labelOpacity = useTransform(scaleStrength || useMotionValue(0), (s) => s);

      return (
        <motion.div
          className="orbit-item"
          style={{
            width: itemSizeValue,
            height: itemSizeValue,
            offsetPath,
            offsetRotate: "0deg",
            offsetAnchor: "center center",
            offsetDistance,
            scale: itemScale,
            zIndex: zIndexMV,
            pointerEvents: "auto"
          }}>
          <motion.div style={{ transform: counterRotate, width: "100%", height: "100%", position: "relative" }}>
            {item}
            {(title || desc) &&
              <motion.div
                style={{
                  position: "absolute",
                  left: "115%",
                  top: "50%",
                  transform: "translateY(-50%)",
                  width: "min(360px, 95%)",
                  color: "#000",
                  opacity: labelOpacity,
                  pointerEvents: "none",
                  fontFamily: "Manrope, ui-sans-serif, system-ui, sans-serif"
                }}>
                {title &&
                  <div style={{
                    fontFamily: "'Instrument Serif', serif",
                    fontSize: "clamp(26px, 3vw, 40px)",
                    lineHeight: 1.05,
                    letterSpacing: "-0.01em",
                    marginBottom: "14px",
                    whiteSpace: "normal"
                  }}>
                    {title}
                  </div>
                }
                {desc &&
                  <div style={{
                    fontFamily: "Manrope, ui-sans-serif, system-ui, sans-serif",
                    fontWeight: 400,
                    fontSize: "clamp(13px, 1vw, 15px)",
                    lineHeight: 1.5,
                    color: "rgba(0,0,0,0.72)"
                  }}>
                    {desc}
                  </div>
                }
              </motion.div>
            }
          </motion.div>
        </motion.div>
      );
    }

    function OrbitImages({
      images = [],
      altPrefix = "Orbiting image",
      baseWidth = 1400,
      radiusX = 700,
      radiusY = 170,
      duration = 40,
      itemSize = 64,
      direction = "normal",
      fill = true,
      width = 100,
      height = 100,
      className = "",
      showPath = false,
      pathColor = "rgba(0,0,0,0.1)",
      pathWidth = 2,
      easing = "linear",
      paused = false,
      centerContent,
      responsive = false,
      progressOverride,
      radiusXOverride,
      radiusYOverride,
      itemSizeOverride,
      rotationOverride,
      translateXOverride,
      focusStrength
    }) {
      const containerRef = useRef(null);
      const [scale, setScale] = useState(1);

      const designCenterX = baseWidth / 2;
      const designCenterY = baseWidth / 2;

      const currentRadiusX = radiusXOverride || useMotionValue(radiusX);
      const currentRadiusY = radiusYOverride || useMotionValue(radiusY);
      const currentItemSize = itemSizeOverride || useMotionValue(itemSize);
      const currentRotation = rotationOverride || useMotionValue(-8);
      const currentTranslateX = translateXOverride || useMotionValue(0);

      const pathValue = useTransform([currentRadiusX, currentRadiusY], ([rx, ry]) => {
        return generateEllipsePath(designCenterX, designCenterY, rx, ry);
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
        const controls = animate(internalProgress, direction === "reverse" ? -100 : 100, {
          duration,
          ease: easing,
          repeat: Infinity,
          repeatType: "loop"
        });
        return () => controls.stop();
      }, [internalProgress, duration, easing, direction, paused, progressOverride]);

      const activeProgress = progressOverride || internalProgress;
      const containerWidth = responsive ? "100%" : typeof width === "number" ? width : "100%";
      const containerHeight = responsive ? "auto" : typeof height === "number" ? height : typeof width === "number" ? width : "auto";

      const items = images.map((entry, index) => {
        const src = typeof entry === "string" ? entry : entry.src;
        return (
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
        );
      });

      return (
        <div
          ref={containerRef}
          className={`orbit-container ${className}`}
          style={{
            width: containerWidth,
            height: containerHeight,
            aspectRatio: responsive ? "1 / 1" : undefined
          }}
          aria-hidden="true">
          <div
            className={responsive ? "orbit-scaling-container orbit-scaling-container--responsive" : "orbit-scaling-container"}
            style={{
              width: responsive ? baseWidth : "100%",
              height: responsive ? baseWidth : "100%",
              transform: responsive ? `translate(-50%, -50%) scale(${scale})` : undefined
            }}>
            <motion.div className="orbit-rotation-wrapper" style={{ rotate: currentRotation, x: currentTranslateX }}>
              {showPath &&
                <svg width="100%" height="100%" viewBox={`0 0 ${baseWidth} ${baseWidth}`} className="orbit-path-svg">
                  <path d={pathValue.get()} fill="none" stroke={pathColor} strokeWidth={pathWidth / scale} />
                </svg>
              }
              {items.map((item, index) => {
                const entry = images[index];
                const title = typeof entry === "object" ? entry.title : null;
                const desc = typeof entry === "object" ? entry.desc : null;
                return (
                  <OrbitItem
                    key={index}
                    item={item}
                    title={title}
                    desc={desc}
                    index={index}
                    totalItems={items.length}
                    pathValue={pathValue}
                    itemSizeValue={currentItemSize}
                    rotationValue={currentRotation}
                    progress={activeProgress}
                    fill={fill}
                    scaleStrength={focusStrength}
                    focalPoint={50}
                  />
                );
              })}
            </motion.div>
          </div>
          {centerContent && <div className="orbit-center-content">{centerContent}</div>}
        </div>
      );
    }

    /* ============================================================
       StaySection
       ============================================================ */

    function StaySection() {
      const blurUp = {
        initial: { opacity: 0, y: 40, filter: "blur(20px)" },
        whileInView: { opacity: 1, y: 0, filter: "blur(0px)" },
        viewport: { once: true, amount: 0.3 },
        transition: { duration: 1, ease: "easeOut" }
      };

      return (
        <section
          className="relative w-full overflow-hidden"
          style={{ minHeight: "100vh", backgroundColor: "#ffffff" }}>
          <img
            src="https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/pasted-1779282335552-1_gmztyi.png"
            alt=""
            aria-hidden="true"
            className="absolute inset-x-0 bottom-0 w-full pointer-events-none select-none"
            style={{ objectFit: "cover", objectPosition: "center bottom" }}
          />

          <div className="relative max-w-[1480px] mx-auto px-8 md:px-16 pt-20 md:pt-24 pb-20 md:pb-24 min-h-screen flex flex-col" style={{ gap: "32px" }}>
            <motion.div {...blurUp}>
              <div style={{
                fontFamily: "'Instrument Serif', serif",
                fontSize: "clamp(60px, 11vw, 160px)",
                lineHeight: 0.95,
                letterSpacing: "-0.01em",
                color: "#000"
              }}>
                Stay <span style={{ fontStyle: "italic" }}>in</span>
              </div>
              <div style={{
                fontFamily: "Manrope, ui-sans-serif, sans-serif",
                fontWeight: 400,
                lineHeight: 0.95,
                letterSpacing: "-0.02em",
                color: "#000",
                fontSize: "64px"
              }}>
                the collection
              </div>
            </motion.div>

            <motion.div
              {...blurUp}
              transition={{ duration: 1, ease: "easeOut", delay: 0.2 }}
              className="max-w-md">
              <p className="mb-6" style={{
                fontFamily: "Manrope, ui-sans-serif, sans-serif",
                fontSize: "15px",
                lineHeight: 1.55,
                color: "rgba(0,0,0,0.78)"
              }}>
                Editions and invitations from the Bentley fragrance studio, sent twice a season.
              </p>
              <form
                className="flex items-center border-b border-black/40 pb-2 gap-3"
                onSubmit={(e) => e.preventDefault()}>
                <input
                  type="email"
                  placeholder="your@email.com"
                  className="bg-transparent flex-1 outline-none"
                  style={{ fontFamily: "Manrope, ui-sans-serif, sans-serif", fontSize: "15px", color: "#000" }}
                />
                <button type="submit" style={{
                  fontFamily: "Manrope, ui-sans-serif, sans-serif",
                  fontSize: "11px",
                  fontWeight: 500,
                  letterSpacing: "0.25em",
                  textTransform: "uppercase",
                  color: "#000"
                }}>
                  Subscribe →
                </button>
              </form>
            </motion.div>
          </div>
        </section>
      );
    }

    /* ============================================================
       Footer
       ============================================================ */

    function Footer() {
      const blurUp = {
        initial: { opacity: 0, y: 40, filter: "blur(20px)" },
        whileInView: { opacity: 1, y: 0, filter: "blur(0px)" },
        viewport: { once: true, amount: 0.3 },
        transition: { duration: 1, ease: "easeOut" }
      };

      const Column = ({ heading, items }) => (
        <div>
          <div className="mb-5 text-black/55" style={{
            fontFamily: "Manrope, ui-sans-serif, sans-serif",
            fontSize: "11px",
            fontWeight: 500,
            letterSpacing: "0.3em",
            textTransform: "uppercase"
          }}>
            {heading}
          </div>
          <ul className="space-y-3">
            {items.map((label) => (
              <li key={label}>
                <a href="#" className="hover:underline" style={{
                  fontFamily: "Manrope, ui-sans-serif, sans-serif",
                  fontSize: "15px",
                  fontWeight: 400,
                  color: "rgba(0,0,0,0.85)"
                }}>
                  {label}
                </a>
              </li>
            ))}
          </ul>
        </div>
      );

      return (
        <footer className="relative w-full text-black overflow-hidden" style={{ backgroundColor: "#f4ecdc" }}>
          <div className="relative max-w-[1480px] mx-auto px-8 md:px-16 pt-12 md:pt-14 pb-12">
            <motion.div
              {...blurUp}
              transition={{ duration: 1, ease: "easeOut", delay: 0.15 }}
              className="grid grid-cols-2 md:grid-cols-4 gap-12 md:gap-10 mb-20 md:mb-24">
              <Column heading="Discover" items={["All fragrances", "The bottle", "Sustainability", "Editions"]} />
              <Column heading="Studio" items={["Our story", "Perfumers", "Atelier visits", "Press"]} />
              <Column heading="Contact" items={["Boutiques", "Concierge", "Returns", "Care guide"]} />

              <div>
                <div className="mb-5 text-black/55" style={{
                  fontFamily: "Manrope, ui-sans-serif, sans-serif",
                  fontSize: "11px",
                  fontWeight: 500,
                  letterSpacing: "0.3em",
                  textTransform: "uppercase"
                }}>
                  Newsletter
                </div>
                <p className="mb-5 text-black/65" style={{
                  fontFamily: "Manrope, ui-sans-serif, sans-serif",
                  fontSize: "14px",
                  lineHeight: 1.5
                }}>
                  Editions and invitations, sent twice a season.
                </p>
                <form
                  className="flex items-center border-b border-black/30 pb-2 gap-3"
                  onSubmit={(e) => e.preventDefault()}>
                  <input
                    type="email"
                    placeholder="your@email.com"
                    className="bg-transparent flex-1 outline-none"
                    style={{ fontFamily: "Manrope, ui-sans-serif, sans-serif", fontSize: "14px", color: "#000" }}
                  />
                  <button type="submit" style={{
                    fontFamily: "Manrope, ui-sans-serif, sans-serif",
                    fontSize: "11px",
                    fontWeight: 500,
                    letterSpacing: "0.25em",
                    textTransform: "uppercase"
                  }}>
                    Subscribe →
                  </button>
                </form>
              </div>
            </motion.div>

            <motion.div
              {...blurUp}
              transition={{ duration: 0.9, ease: "easeOut", delay: 0.25 }}
              className="flex flex-col md:flex-row items-start md:items-center justify-between gap-6 pt-8 border-t border-black/15">
              <div className="text-black/55" style={{
                fontFamily: "Manrope, ui-sans-serif, sans-serif",
                fontSize: "11px",
                fontWeight: 500,
                letterSpacing: "0.3em",
                textTransform: "uppercase"
              }}>
                © 2026 Beyond The Collection
              </div>
              <div className="flex items-center gap-5" style={{
                fontFamily: "Manrope, ui-sans-serif, sans-serif",
                fontSize: "11px",
                fontWeight: 500,
                letterSpacing: "0.28em",
                textTransform: "uppercase"
              }}>
                <a href="#" className="hover:underline">Instagram</a>
                <span className="text-black/30">·</span>
                <a href="#" className="hover:underline">TikTok</a>
                <span className="text-black/30">·</span>
                <a href="#" className="hover:underline">Spotify</a>
              </div>
              <div className="text-black/55" style={{
                fontFamily: "Manrope, ui-sans-serif, sans-serif",
                fontSize: "11px",
                fontWeight: 500,
                letterSpacing: "0.3em",
                textTransform: "uppercase"
              }}>
                EN · USD
              </div>
            </motion.div>
          </div>
        </footer>
      );
    }

    /* ============================================================
       App
       ============================================================ */

    const orbitImagesData = [
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL1996-Beyond_wild_vetiver_Flakon_100ml_300dpi_a55ie5.webp",
        title: "Wild Vetiver",
        desc: "Smoky vetiver wrapped in saffron and leather — a grounded, untamed signature."
      },
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL2156_BEYOND_RADIANT_OSMANTHUS_1_hlc4v1.webp",
        title: "Radiant Osmanthus",
        desc: "Apricot-tinged osmanthus over soft musks. Quietly luminous, never loud."
      },
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL2156_BEYOND_RADIANT_OSMANTHUS_hoc3up.webp",
        title: "Vibrant Hibiscus",
        desc: "Bright hibiscus and pink pepper resting on creamy sandalwood."
      },
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL2157_BEYOND_VIBRANT_HIBISCUS_pgiehq.webp",
        title: "Mellow Heliotrope",
        desc: "Almond, vanilla and heliotrope petals — a powdery, hushed warmth."
      },
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL2158_BEYOND_MELLOW_HELIOTROPE_agqych.webp",
        title: "Magnetic Amber",
        desc: "Resinous amber, oud and rich woods. The collection's deepest note."
      },
      {
        src: "https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/cloudinarry%20to%20cloudflare/BL2371-BL2372-BL2373-Magnetic-Amber_web_2_dbmtpy.webp",
        title: "Crystal Edition",
        desc: "A limited cut of the bottle — etched facets, lavender pour, leather collar."
      }
    ];

    const VIDEO_SRC = "https://d8j0ntlcm91z4.cloudfront.net/user_3BA1nJibL92zfZpAJB3BLBU6tQI/hf_20260520_114550_b72cc2b7-2267-4d9e-b19f-f3bb4b0c7084.mp4";
    const TARGET_RADIUS = 650;

    function App() {
      const containerRef = useRef(null);

      const { scrollYProgress } = useScroll({
        target: containerRef,
        offset: ["start start", "end end"]
      });

      const rx = useTransform(scrollYProgress, [0, 0.08, 1], ["0%", "55%", "55%"]);
      const ry = useTransform(scrollYProgress, [0, 0.08, 1], ["0%", "55%", "55%"]);
      const clipPath = useMotionTemplate`ellipse(${rx} ${ry} at 50% 50%)`;

      const textOpacity = useTransform(
        scrollYProgress,
        [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1],
        [0, 1, 1, 0, 0, 1, 1]
      );
      const textBlurVal = useTransform(
        scrollYProgress,
        [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1],
        [15, 0, 0, 15, 15, 0, 0]
      );
      const filterText = useMotionTemplate`blur(${textBlurVal}px)`;
      const yElement = useTransform(
        scrollYProgress,
        [0.03, 0.08, 0.15, 0.22, 0.90, 0.98, 1],
        [20, 0, 0, 20, 20, 0, 0]
      );

      const scrollHintOpacity = useTransform(scrollYProgress, [0, 0.03, 0.08], [1, 1, 0]);

      const orbitItemSize = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [80, 360, 360, 80, 80]);
      const orbitRx = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [330, TARGET_RADIUS, TARGET_RADIUS, 330, 330]);
      const orbitRy = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [140, TARGET_RADIUS, TARGET_RADIUS, 140, 140]);
      const orbitRotation = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [-15, 0, 0, -15, -15]);
      const orbitTx = useTransform(
        scrollYProgress,
        [0.15, 0.25, 0.85, 0.95, 1],
        [0, -(TARGET_RADIUS + 200), -(TARGET_RADIUS + 200), 0, 0]
      );
      const focusStrength = useTransform(scrollYProgress, [0.15, 0.25, 0.85, 0.95, 1], [0, 1, 1, 0, 0]);

      const orbitProgress = useMotionValue(0);
      const prevScroll = useRef(0);

      useAnimationFrame((time, delta) => {
        const pos = scrollYProgress.get();
        const scrollDelta = pos - prevScroll.current;
        prevScroll.current = pos;

        let frameSpeed = 0;
        if (pos > 0.15 && pos < 0.85) {
          frameSpeed = scrollDelta * 200;
        } else {
          frameSpeed = delta / 1000 * 2.5;
        }

        orbitProgress.set(orbitProgress.get() + frameSpeed);
      });

      return (
        <>
          <div ref={containerRef} className="relative w-full h-[600vh] bg-black">
            <div className="sticky top-0 w-full h-screen overflow-hidden text-white">

              {/* Video background */}
              <video autoPlay loop muted playsInline className="absolute inset-0 w-full h-full object-cover z-0">
                <source src={VIDEO_SRC} type="video/mp4" />
              </video>

              {/* Top-left logo text */}
              <div
                className="absolute z-10 flex flex-col items-start text-left text-black select-none leading-[0.95]"
                style={{ top: "120px", left: "96px" }}>
                <div className="flex items-baseline">
                  <span style={{ fontFamily: "'Instrument Serif', serif", fontSize: "clamp(32px, 5vw, 64px)" }}>
                    Beyond
                  </span>
                  <span style={{
                    fontFamily: "'Instrument Serif', serif",
                    fontStyle: "italic",
                    fontSize: "clamp(32px, 5vw, 64px)",
                    marginLeft: "0.05em"
                  }}>
                    The
                  </span>
                </div>
                <span style={{
                  fontFamily: "Manrope, ui-sans-serif, system-ui, sans-serif",
                  fontWeight: 400,
                  fontSize: "clamp(28px, 4.4vw, 56px)",
                  letterSpacing: "-0.01em",
                  marginTop: "0.05em"
                }}>
                  Collection
                </span>
              </div>

              {/* Scroll hint arrow */}
              <motion.div
                className="absolute z-10 left-1/2 -translate-x-1/2 flex flex-col items-center text-white select-none pointer-events-none"
                style={{ bottom: "40px", opacity: scrollHintOpacity }}>
                <div className="relative w-[20px] h-[34px] overflow-hidden">
                  <svg
                    className="scroll-arrow absolute inset-0"
                    width="20" height="34" viewBox="0 0 20 34" fill="none"
                    xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                    <path d="M10 4 V28 M3 21 L10 28 L17 21" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </div>
              </motion.div>

              {/* Clip-path reveal with orbit */}
              <motion.div
                className="absolute z-20 flex items-center justify-center overflow-hidden"
                style={{
                  clipPath,
                  rotate: -15,
                  width: "150vw",
                  height: "150vh",
                  left: "-25vw",
                  top: "-25vh"
                }}>
                <div className="absolute inset-0 bg-white" />
                <div
                  className="relative flex flex-col items-center justify-center"
                  style={{ width: "100vw", height: "100vh", transform: "rotate(15deg)" }}>
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

              {/* Text overlays */}
              <div className="absolute inset-0 z-[60] pointer-events-none">

                {/* Center brand text */}
                <div className="absolute top-[48%] left-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-none z-50">
                  <motion.div
                    className="flex flex-col items-center whitespace-nowrap pointer-events-auto"
                    style={{
                      filter: filterText,
                      opacity: textOpacity,
                      WebkitFontSmoothing: "antialiased",
                      WebkitBackfaceVisibility: "hidden",
                      transform: "translateZ(0)"
                    }}>
                    <div className="flex items-baseline text-black leading-none mb-1">
                      <span className="font-serif text-[45px] md:text-[55px] tracking-tight text-black">Beyond </span>
                      <span className="font-serif text-[45px] md:text-[55px] italic tracking-tight text-black">The</span>
                    </div>
                    <span className="font-sans text-[28px] md:text-[36px] tracking-tight text-black mt-[-5px]">Collection</span>
                  </motion.div>
                </div>

                {/* Top-right info */}
                <motion.div
                  className="absolute top-32 right-[calc(6vw+150px)] md:right-[214px] flex flex-col items-start text-left pointer-events-auto cursor-text"
                  style={{ y: yElement, filter: filterText, opacity: textOpacity }}>
                  <span className="font-serif text-[40px] leading-none mb-3 text-black">2K26</span>
                  <span className="font-serif text-[16px] uppercase tracking-widest text-black leading-[20px] text-left">
                    JOIN AN EXCLUSIVE<br />COMMUNITY
                  </span>
                </motion.div>

                {/* Bottom-left number */}
                <motion.div
                  className="absolute bottom-8 left-8 md:bottom-16 md:left-16 flex flex-col items-start text-black pointer-events-auto cursor-text"
                  style={{ y: yElement, filter: filterText, opacity: textOpacity }}>
                  <span className="font-serif text-[40px] leading-none mb-1 text-black">0651</span>
                  <span className="font-serif text-[16px] uppercase tracking-widest text-black">COLLECTION</span>
                </motion.div>

                {/* Bottom-right CTA */}
                <div className="absolute bottom-16 right-[6vw] md:right-[10vw] flex flex-col items-start z-10 pointer-events-auto">
                  <motion.p
                    className="font-serif text-[16px] uppercase tracking-widest text-black leading-[20px] mb-6 text-left w-[240px] cursor-text"
                    style={{ y: yElement, filter: filterText, opacity: textOpacity }}>
                    JOIN AN EXCLUSIVE COMMUNITY OF SAILORS. WHETHER YOU CRAVE THE THRILL OF THE OPEN
                  </motion.p>
                  <motion.div
                    className="flex gap-0 pointer-events-auto items-center"
                    style={{ y: yElement, filter: filterText, opacity: textOpacity }}>
                    <button className="bg-black hover:bg-black/90 transition-colors text-white rounded-[40px] px-8 py-3.5 font-serif tracking-[0.1em] uppercase text-[12px] md:text-[14px] z-10">
                      BUY COLLECTION
                    </button>
                    <button className="bg-black hover:bg-black/90 transition-colors w-[46px] h-[46px] flex items-center justify-center rounded-[50%] text-white -ml-2 z-0">
                      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="ml-1">
                        <path d="M5 12h14M12 5l7 7-7 7" />
                      </svg>
                    </button>
                  </motion.div>
                </div>
              </div>

              {/* Header */}
              <motion.header
                className="absolute top-0 left-0 w-full px-6 md:px-12 py-5 md:py-6 flex justify-between items-center z-[100] pointer-events-none"
                style={{ opacity: scrollHintOpacity }}>
                <a href="#" className="flex items-center gap-3 text-black select-none pointer-events-auto" aria-label="Bentley">
                  <svg width="54" height="40" viewBox="0 0 84 60" fill="none" aria-hidden="true">
                    <g fill="currentColor">
                      <path d="M42 22 C30 22 19 16 4 12 C9 26 18 33 30 33 L42 33 Z" />
                      <path d="M42 22 C54 22 65 16 80 12 C75 26 66 33 54 33 L42 33 Z" />
                      <path d="M34 25 C36 28 39 30 42 30 C45 30 48 28 50 25 L42 22 Z" opacity="0.7" />
                    </g>
                    <text x="42" y="52" textAnchor="middle" fontFamily="'Instrument Serif', serif" fontSize="22" fontStyle="italic" fill="currentColor">B</text>
                  </svg>
                  <span style={{
                    fontFamily: "Manrope, ui-sans-serif, sans-serif",
                    fontWeight: 600,
                    fontSize: "14px",
                    letterSpacing: "0.42em",
                    textTransform: "uppercase"
                  }}>
                    Bentley
                  </span>
                </a>

                <a
                  href="#"
                  className="pointer-events-auto inline-flex items-center gap-2 bg-black text-white rounded-full pl-5 pr-2 py-2 hover:bg-black/85 transition-colors"
                  style={{
                    fontFamily: "Manrope, ui-sans-serif, sans-serif",
                    fontSize: "11px",
                    fontWeight: 500,
                    letterSpacing: "0.22em",
                    textTransform: "uppercase"
                  }}>
                  <span className="hidden sm:inline">Shop the collection</span>
                  <span className="sm:hidden">Shop</span>
                  <span className="inline-flex w-7 h-7 items-center justify-center rounded-full bg-white/15">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M5 12h14M12 5l7 7-7 7" />
                    </svg>
                  </span>
                </a>
              </motion.header>

            </div>
          </div>

          <StaySection />
          <Footer />
        </>
      );
    }

    createRoot(document.getElementById("root")).render(<App />);
  };

  if (window.__depsReady) start();
  else window.addEventListener("deps-ready", start, { once: true });
})();
  </script>
</body>
</html>

## Luxury Ecommerce Design — Landing Page [sites/luxury-editorial-ecommerce-design]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(77).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luxury-editorial-ecommerce-design.webp

Create a React + Tailwind CSS beauty/skincare brand landing page called "STRETCH" with 3 sections. Use Vite, React 18, TypeScript, Tailwind CSS, and lucide-react for icons. The system font stack is used (no custom fonts loaded). The page has smooth scroll-triggered fade-in animations using IntersectionObserver, button hover lift animations, and full responsive design with a mobile hamburger menu.

---

### SECTION 1: HERO (Full viewport height, split 50/50 on desktop, stacked on mobile)

**Announcement Bar** (absolute positioned, top of page, z-30):
- Background: `#F9F4F0`, text black
- Centered text: "free shipping for orders over 50€"
- ChevronLeft and ChevronRight icons (size 16) on each side
- Padding: `py-2.5` mobile, `py-3` desktop

**Navigation** (absolute positioned below announcement bar at `top-[38px]` mobile / `top-[42px]` desktop, z-30):
- Left: Logo text "STRETCH" — `text-lg sm:text-xl font-bold tracking-[0.2em] uppercase`
- Center (hidden on mobile, visible md+): 4 links — "shop", "learn", "journal", "theme" — `text-sm`, with an underline animation on hover (a `<span>` inside that goes from `w-0` to `w-full` on group-hover, `h-[1px] bg-white transition-all duration-300`)
- Right:
  - French flag (3 colored divs: `bg-blue-700`, white, `bg-red-600` in a `w-6 h-4` container) + "eur €" text + ChevronDown — hidden on mobile
  - Vertical divider `w-px h-5 bg-white/30 mx-2` — hidden on mobile
  - User icon (hidden below sm), Search icon, ShoppingBag icon (all size 20)
  - Menu/X hamburger toggle (visible below md)

**Mobile Menu** (fixed fullscreen overlay, z-40):
- `bg-black/95 backdrop-blur-sm`
- Centered vertically: same 4 nav links at `text-3xl font-light`
- Transition: `opacity` + `pointer-events` toggle over `duration-500`

**Hero Left Half** (`w-full lg:w-1/2`, `min-h-[60vh] lg:min-h-0`):
- Background: Full-bleed absolute image:
  ```
  https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260516_101925_8e509c31-4e75-4ae1-b164-2605265b2d47.png&w=1280&q=85
  ```
- Content (relative z-10, fade-in animation: `translate-y-8` to `translate-y-0`, `opacity-0` to `opacity-100`, `duration-1000`):
  - Heading: "ethical beauty," (line break) "sustainable impact." — `text-4xl sm:text-5xl md:text-6xl lg:text-[clamp(3.5rem,5vw,6rem)] font-light leading-[1.05] mb-6`
  - Under "impact." word: decorative SVG with 3 wavy gold lines (`stroke="#C8A45C"`, strokeWidths 2, 1.5, 1) — absolutely positioned `-bottom-1 left-0 w-full h-4`
  - Paragraph: "Committed to sustainable beauty and minimize our impact on the planet." — `text-sm md:text-base text-white/80 mb-10 max-w-md`
  - Button: "about us" — `px-10 py-4 bg-white text-black rounded-full text-sm` with `.btn-primary` class

**Hero Right Half** (`w-full lg:w-1/2`, `min-h-[40vh] lg:min-h-0`):
- Video slideshow (3 slides, auto-advances every 5000ms):
  - Video 1: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260516_112022_cddf2487-4ffe-45b6-ba4c-99ab79003cc5.mp4`
  - Video 2: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_175400_b46d1cd2-2050-45e2-9d13-b9c0bacb16b3.mp4`
  - Video 3: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_182440_671605c8-2ed8-4507-a4cb-a62a8f61316f.mp4`
  - All videos: `autoPlay loop muted playsInline`, `object-cover`, absolute `inset-0`
  - Transition between slides: `transition-opacity duration-700`
- Controls (absolute `bottom-6 right-6` z-20):
  - 3 dot indicators: `w-2 h-2 rounded-full`, active = `bg-white scale-125`, inactive = `bg-white/50`
  - Pause/Play toggle button: `w-8 h-8 rounded-full border border-white/50`, Pause/Play icon size 14

---

### SECTION 2: BEST SELLERS (Background `#F9F4F0`, text black)

- Padding: `py-12 sm:py-16 px-4 sm:px-6 lg:px-10`
- Fade-in animation on scroll (translate-y-6 to 0, opacity 0 to 1)

**Tabs:**
- Two buttons: "best sellers" and "sets"
- Text: `text-2xl sm:text-4xl md:text-5xl font-medium`
- Active tab: `text-[#1a1a1a]` with a filled dot `w-5 h-5 sm:w-6 sm:h-6 rounded-full bg-[#1a1a1a]` that has a scale-in CSS animation
- Inactive tab: `text-gray-400`, hover → `text-gray-600`

**Product Carousel** (horizontal scroll, `overflow-x-auto scrollbar-hide`):
- Vertical scroll (mouse wheel) is hijacked to scroll horizontally
- Each product card: `w-[260px] sm:w-[280px] md:w-[300px] lg:w-[calc(25%-1px)]`
- Cards have `border border-gray-200` on all 4 sides, with `-ml-[1px] first:ml-0` to collapse shared borders
- Cards fade in staggered: each card has `transitionDelay: ${200 + index * 80}ms`
- On hover: product image scales to 105% (`transition-transform duration-500`)

**7 Products (in order):**
1. Category: "ILLUMINATE" | Name: "Illuminating cleansing gel" | Price: "€36,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_193822_8c95f5ed-b142-454f-ab87-59ad1f09e758.png&w=1280&q=85`
2. Category: "UNIFY" | Subcategory: "TIGHTEN PORES" | Name: "Unifying serum spray" | Price: "€34,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194048_278bf3cc-7d1f-43c1-9dc7-73d8fcd9949c.png&w=1280&q=85`
3. Category: "NATURAL GLOW" | Name: "Super glow set" | Price: "€92,00" | Old price: "€99,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194058_d89610de-05f8-45e4-8196-0680296c565a.png&w=1280&q=85`
4. Category: "PROTECT" | Subcategory: "ILLUMINATE" | Name: "Radiance day oil" | Price: "€59,00" | Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260518_194112_1763cbb2-3171-4ad3-9f38-1b738b8f1bb6.png&w=1280&q=85`
5. Category: "HYDRATE" | Subcategory: "NOURISH" | Name: "Deep moisture cream" | Price: "€48,00" | Image: same as product 1
6. Category: "RENEW" | Name: "Night repair elixir" | Price: "€72,00" | Old price: "€79,00" | Image: same as product 2
7. Category: "SMOOTH" | Subcategory: "REFINE" | Name: "Gentle exfoliating toner" | Price: "€42,00" | Image: same as product 3

**Card layout:**
- Top: category label (`text-xs font-medium tracking-wider uppercase`) + optional subcategory (`text-xs text-gray-500 uppercase mt-0.5`) in a `px-4 h-12` container
- Middle: image in `mx-4 aspect-[3/4] rounded-lg overflow-hidden bg-[#F9F4F0]`, `object-cover`
- Bottom: product name (`text-sm`, centered) + price row (with optional strikethrough old price in `text-gray-400 line-through`)

**Scroll Progress Bar:**
- `mt-8 sm:mt-10 mx-auto max-w-[280px]`
- Track: `h-[2px] bg-gray-300 rounded-full`
- Thumb: `width: 30%`, `bg-[#1a1a1a]`, position calculated as `translateX(${scrollProgress * (100 / 0.3)}%)`

---

### SECTION 3: CATEGORIES (Background black, text white)

- 3-column grid on desktop (`grid-cols-1 md:grid-cols-3`), no gaps, no dividers between columns
- Fade-in animation on scroll (translate-y-12 to 0, opacity 0 to 1, duration-1000)

**3 Category Cards (each):**
- Min height: `min-h-[400px] sm:min-h-[500px] md:min-h-[750px]`
- Padding: `p-6 sm:p-8 md:p-12`
- Full-bleed background video (absolute, `object-cover`)
- On hover: video scales to 105% (`transition-transform duration-700`)
- Dark overlay: `bg-black/10` → hover `bg-black/20` (`transition-colors duration-500`)
- Vertical text (rotated): `writingMode: 'vertical-lr', transform: 'rotate(180deg)'` — `text-5xl sm:text-6xl md:text-7xl lg:text-8xl font-medium` — moves up 2px on hover
- Button at bottom: "shop [name]" — `px-8 py-3 bg-white text-black rounded-full text-sm` with `.btn-primary`

**Category data:**
1. Name: "face" | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203023_87a26602-2898-4acc-a396-c7a2b5ad84fd.mp4`
2. Name: "beauty tools" | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203415_b86e3f19-2aec-46cd-9a86-b64c40118e38.mp4`
3. Name: "body" | Video: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260518_203051_85fee398-ea01-4aa0-972b-137a74213be5.mp4`

---

### CSS (index.css):

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

.btn-primary {
  position: relative;
  overflow: hidden;
  transition: transform 0.3s ease, box-shadow 0.3s ease;
}

.btn-primary::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(120deg, transparent 0%, rgba(0, 0, 0, 0.05) 50%, transparent 100%);
  transform: translateX(-100%);
  transition: transform 0.5s ease;
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
}

.btn-primary:hover::before {
  transform: translateX(100%);
}

.btn-primary:active {
  transform: translateY(0);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

@keyframes scale-in {
  from {
    transform: scale(0);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

.animate-scale-in {
  animation: scale-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

---

### ANIMATIONS SUMMARY:
1. **useInView hook** — custom IntersectionObserver hook (threshold configurable, default 0.15). Once element enters viewport, sets `isVisible = true` permanently (unobserves after).
2. **Hero text** — fades in + slides up 8px over 1000ms
3. **Best sellers tabs** — fades in + slides up 6px over 800ms
4. **Product cards** — staggered fade-in (each 80ms apart, starting at 200ms delay), slides up 8px over 500ms
5. **Categories section** — fades in + slides up 12px over 1000ms
6. **Tab dot** — scale-in keyframe with bounce easing `cubic-bezier(0.34, 1.56, 0.64, 1)` over 300ms
7. **Buttons (.btn-primary)** — lift 2px + shadow on hover, light sweep effect via `::before` pseudo-element
8. **Product images** — scale to 105% on card hover over 500ms
9. **Category videos** — scale to 105% on card hover over 700ms
10. **Nav links** — underline grows from left (`w-0` to `w-full`) over 300ms on hover

---

### TECH STACK:
- Vite + React 18 + TypeScript
- Tailwind CSS 3.4
- lucide-react for icons (ChevronLeft, ChevronRight, User, Search, ShoppingBag, ChevronDown, Pause, Play, Menu, X)
- No other UI libraries

## Luxury Real Estate — Landing Page [sites/luxury-real-estate]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(38).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/luxury-real-estate.webp

Build a single-page React + TypeScript + Tailwind CSS + Vite landing page for a luxury real estate brand named "Velar.". Use only `lucide-react` for icons. The app is in `src/App.tsx`. Use the exact specifications below.

Global Setup

- Page background: `#f5f0ea` (warm off-white).
- Body wrapper: `overflow-x: clip`.
- Fonts (loaded via `@import` inside an inline `<style>` block):
  - Primary: `Syne` weights 400, 700, 800, 900 from Google Fonts.
  - Secondary: `Inter` weights 300, 400, 500, 600 from Google Fonts.
- Constants:
  - `GRASS_GREEN = '#213138'` (deep teal — used for preloader background and default logo color).
  - `FULL_TEXT = 'Velar.'`
  - `HOUSE_IMG = 'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1780471903/building_bzziky.png'`
  - `BG_IMG = 'https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260603_073200_7082add5-f1f8-4873-8696-d6f78a44089b.png&w=1920&q=85`
- Gallery videos (5, in order):
  1. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260528_154759_4cdc8175-8261-497c-b688-9477c76545d4.mp4`
  2. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260528_154751_39b1b9bb-2708-4211-b6a2-d39f93309e52.mp4`
  3. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260528_154737_eba7900c-0313-483c-a30a-632c747ccc42.mp4`
  4. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260602_144009_4348fe33-f885-4345-8e92-3fe1c2625d32.mp4`
  5. `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260602_145337_e44eaa8c-6bb1-4a6e-a70f-ed0231cbaccb.mp4`

Section 1 — Preloader / Intro Overlay

- Fixed full-viewport overlay (`z-index: 100`) filled with `#213138`, centered flex.
- Renders an animated typewriter of the word `Velar.` in Syne, font-size `2.6rem`, color white, letter-spacing `-0.02em`. Letters use weight 700 except `.` which is weight 900.
- A blinking white cursor (3px × 1.1em rounded bar, animation `blink 0.7s step-end infinite` toggling opacity 0/1) follows the last typed letter.
- Timings (using `setTimeout`):
  - `CHAR_INTERVAL = 140ms`, `TYPE_START = 600ms`.
  - Reveal letters one at a time at `TYPE_START + i * CHAR_INTERVAL`.
  - `LIFT_AT = TYPE_START + 6 * CHAR_INTERVAL + 700ms`.
  - Hide cursor at `LIFT_AT − 150ms`.
  - Start "lifting" the overlay upward at `LIFT_AT`: `transform: translateY(-100%)` with transition `transform 1.5s cubic-bezier(0.45, 0, 0.15, 1)`.
  - At `LIFT_AT + 1300ms`, fade in the hero text (`opacity 0 → 1`, `translateY(-28px) → 0`, transition `0.7s cubic-bezier(0.22, 1, 0.36, 1) 0.1s`).
  - At `LIFT_AT + 2100ms`, set `liftDone` true and disable the overlay's transition (so it stays parked off-screen).

Section 2 — Fixed Navigation

- Fixed top nav `z-50`, padding `px-6 md:px-10 lg:px-16`, `py-5 md:py-6`, flex justify-between.
- Left: word `Velar.` in Syne, `text-xl`, weight 700 for letters and 900 for `.`. Color = `navColor` (see scroll behavior).
- Right: hamburger toggle button. Two stacked 28px-wide × 1px lines, top one shrinks to `w-5` on hover. When open, swap to a Lucide `X` icon, size 24.
- Scroll behavior: track whether any "dark section" (refs to Section 4 and Section 5) currently overlaps the viewport top (`rect.top <= 0 && rect.bottom > 0`). If so, `navOnDark = true` and `navColor = '#ffffff'`. Otherwise `navColor = '#213138'`. Color transitions: `color 0.35s ease`.
- Mobile menu: when open, full-screen `#f5f0ea` overlay (`z-40`) centered with 4 vertically stacked links: `Residences`, `Story`, `Listings`, `Inquire`. Each link is Syne, `text-4xl`, `font-light`, `tracking-widest`, uppercase, black with hover `text-gray-500`. Click closes menu.

Section 3 — Hero

- `<section>` `position: relative`, `min-height: 100vh`, `overflow: visible`.
- Background: `BG_IMG` as `background-image`, `background-size: cover`, `background-position: center center`, `background-repeat: no-repeat`.
- Hero text block (`.hero-text-block`) inside, `z-index: 10`, hidden initially, fades+slides in (see preloader timings).
- Top row (`.hero-heading-top`, padded `px-6 md:px-10 lg:px-16`, flex `items-end justify-between`, `margin-bottom: -0.04em`):
  - Left: `LIVE IN` — Syne 800, uppercase, black, `letter-spacing: -0.03em`, `line-height: 1`. Size via CSS class `.hero-own-the`.
  - Right (desktop only ≥1024px, `.hero-subtitle-desktop`): two-line right-aligned paragraph in Syne 700, `clamp(10px, 0.95vw, 14px)`, max-width 300px, opacity 0.7, line-height 1.6, margin-bottom `0.2em`, letter-spacing `0.02em`:
    > Stately homes built with vision,
    > scope, and architectural finesse.
- Headline row (wrapped in `overflow: hidden`):
  - `IRREPLACEABLE` — Syne 800, uppercase, black, `letter-spacing: -0.03em`, padded `px-6 md:px-10 lg:px-16`. Size via `.hero-extraordinary`.
- Mobile/tablet subtitle (`.hero-subtitle-mobile`, padded `px-6`), Syne 600, `clamp(12px, 3vw, 15px)`, opacity 0.65, margin-top `0.9em`:
  > Premium real estate with vision,
  > depth, and architectural clarity.

Hero Responsive Type Sizes

```css
@media (max-width: 639px) {
  .hero-subtitle-desktop { display: none !important; }
  .hero-subtitle-mobile  { display: block !important; }
  .hero-text-block { padding-top: 90px !important; }
  .hero-heading-top { justify-content: flex-start !important; }
  .hero-own-the { font-size: 7.5vw !important; }
  .hero-extraordinary { font-size: 14.5vw !important; white-space: normal !important; word-break: break-word !important; line-height: 0.9 !important; }
}
@media (min-width: 640px) and (max-width: 1023px) {
  .hero-subtitle-desktop { display: none !important; }
  .hero-subtitle-mobile  { display: block !important; }
  .hero-text-block { padding-top: 110px !important; }
  .hero-heading-top { justify-content: flex-start !important; }
  .hero-own-the { font-size: 5.5vw !important; }
  .hero-extraordinary { font-size: 11vw !important; white-space: normal !important; word-break: break-word !important; line-height: 0.9 !important; }
}
@media (min-width: 1024px) {
  .hero-subtitle-desktop { display: block !important; }
  .hero-subtitle-mobile  { display: none !important; }
  .hero-text-block { padding-top: calc(28vh - 50px) !important; }
  .hero-own-the { font-size: 3vw !important; }
  .hero-extraordinary { font-size: clamp(52px, 6.5vw, 9vw) !important; white-space: nowrap !important; line-height: 0.88 !important; }
}
```

Section 4 — Scroll-Driven House Animation (the centerpiece)

- A `position: fixed` wrapper at `z-index: 22`, `pointer-events: none`, `will-change: transform`, default `bottom: 0; left: 50%; transform: translateX(-50%); width: 100%; min-width: 1400px;`.
- Inside, an inner div performs the initial "rise from below" entrance: starts at `translateY(102vh)`, transitions to `translateY(0)` with `transform 1.5s cubic-bezier(0.45, 0, 0.15, 1) 0.4s`, triggered when `lifting` becomes true. Once `liftDone` true the transition is removed so the scroll handler can take over.
- Renders `` at width 100%, aria-hidden.
- After `liftDone`, a scroll/resize listener (`updateHousePosition`) computes:
  - `baseW = max(window.innerWidth, 1400)`.
  - `triggerPoint = -(heroH  0.30)` — animation starts when 30% of hero has scrolled off.
  - `endPoint = heroRect.top - (darkRect.bottom - vh)` — ends when the bottom of Section 5 reaches viewport bottom.
  - `progress = clamp((heroRect.top − triggerPoint) / (endPoint − triggerPoint), 0, 1)`.
  - `t = smoothstep(smoothstep(progress))` where `smoothstep(t) = tt(3−2t)` (applied twice).
  - `startX = (vw − baseW) / 2`, `startY = vh − imgH` (bottom-centered).
  - `finalScale = 1.45`, `finalX = (vw − baseW  finalScale) / 2` (bottom-centered), `mobileOffset = vw < 1024 ? −250 : 4`, `finalY = darkRect.bottom − imgH * finalScale + 500 + mobileOffset`.
  - Interpolates `currentX`, `currentY`, `currentScale` linearly via `t`.
- At `progress <= 0` resets to resting (bottom-centered, scale 1). Otherwise sets `top: 0; left: 0; transform: translate(currentX, currentY) scale(currentScale); transform-origin: top left;`.

### Section 5 — Dark Statement + Stats (sticky)

- Outer wrapper: `position: relative; height: 200vh; z-index: 20`.
- Inner `

` (`s2-section`): `position: sticky; top: 0; height: 100vh; background: #1a1a1a; overflow: hidden`. Above it is a tiny `4vh` `#1a1a1a` scroll spacer.
- Content wrapper `.s2-content`: flex column, padding `px-6 md:px-10 lg:px-16`, `padding-top: clamp(30px, 4vw, 60px)`, `padding-bottom: clamp(60px, 8vw, 120px)`.
- Statement text (`.s2-statement`), Inter 300, color `#e8e4df`, letter-spacing `-0.02em`, `line-height: 1.35`, `white-space: nowrap`, font-size `clamp(22px, 2.6vw, 42px)`. Wrapper has `max-width: 1200px`, centered, `padding-left: 25%`. Lines (with hard `
`s):
  > Every estate we present is hand-chosen
  > through a frame of permanence, refinement,
  > and timeless detail. Standards are not
  > a flourish. It is our discipline.
- Stats row (`.s2-stats-row`): same max-width/centered/padding-left 25%, `margin-top: clamp(48px, 6vw, 80px)`. Three columns in a flex row, each `flex:1`, with a left border (`1px solid rgba(255,255,255,0.2)`) between items and `padding-left: clamp(20px, 2.5vw, 40px)` on items 2–3:
  1. `120+` — `Portfolio Holdings`
  2. `12` — `Global Locations`
  3. `98%` — `Patron Loyalty Rate`
  - Numbers: Inter 300, white, font-size `clamp(36px, 4.5vw, 72px)`, line-height 1.1. Use a `CountUp` component that, when the element first crosses 30% into the viewport (IntersectionObserver), animates from 0 to `end` over 2000ms with easing `1 - (1 - t)^3`, rendering `Math.round(eased * end) + suffix`.
  - Labels: Inter 400, `rgba(255,255,255,0.6)`, font-size `clamp(12px, 1.1vw, 16px)`, `margin-top: clamp(4px, 0.5vw, 8px)`, letter-spacing `0.01em`.
- Tablet/mobile rules:
  - `≤767px`: remove the 25% left padding entirely (set to 0).
  - `768–1023px`: reduce padding-left to 15%, set `min-height: 70vh` and adjust paddings.

### Section 6 — Hover-Expand Gallery (slides over Section 5)

- `

` (`s3-gallery-section`) `position: relative; z-index: 25; margin-top: -100vh; background: #1a1a1a; height: 100vh; overflow: hidden`. This makes it slide up over Section 5 as the user scrolls.
- Background ticker (`.s3-ticker-wrap`): absolutely positioned `inset:0`, flex center, `overflow: hidden`, `z-index: 0`, `pointer-events: none`. Contains a `.ticker-track` with two copies of a giant repeating string:
  > `Velar.   Velar.   Velar.   Velar.   Velar.   Velar.   Velar.   Velar.  ` (with ` ` separators)
  - Each span: Syne 800, `clamp(100px, 14vw, 220px)`, white, `white-space: nowrap`, letter-spacing `-0.02em`, `user-select: none`, `padding-right: 0.3em`. (The ticker can also be animated with a horizontal scroll keyframe — left as a static layered word-mark behind the gallery here.)
- Gallery content (`.s3-gallery-content`): z-index 1, flex center, full height, padding `clamp(24px, 4vw, 60px)`.
- Row (`.gallery-expand-row`): flex with `gap:6px`, height 70%, max-width 1200px. Each item (`.gallery-expand-item`): `flex:1 1 0%`, full height, `border-radius:12px`, `overflow:hidden`, `cursor:pointer`, transition `flex 0.5s cubic-bezier(0.4, 0, 0.2, 1)`. On hover, the hovered item grows to `flex: 4`, others shrink — classic accordion expand.
- Each item contains the corresponding video (autoplay, loop, muted, playsInline) covering the tile (`object-fit: cover`).

### Gallery Mobile/Tablet Rules (≤1023px)

```css
.s3-gallery-section { height: auto; min-height: 100vh; overflow: visible; }
.s3-ticker-wrap { position: sticky; top: 0; height: 100vh; width: 100%; margin-bottom: -100vh; }
.s3-gallery-content { height: auto; align-items: flex-start; padding: 80px 16px 60px; }
.gallery-expand-row { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; height: auto; width: 100%; max-width: 700px; }
.gallery-expand-item { flex: none; height: auto; aspect-ratio: 4/5; border-radius: 10px; transition: transform 0.3s ease; }
.gallery-expand-item:hover { flex: none; transform: scale(1.02); }
.gallery-expand-item:last-child:nth-child(odd) { grid-column: 1 / -1; max-width: calc(50% - 4px); justify-self: center; }
@media (max-width: 479px) {
  .s3-gallery-content { padding: 60px 12px 48px; }
  .gallery-expand-row { gap: 6px; }
}
```

### Behavior Recap

- Preloader types `Velar.` then slides up out of view, simultaneously revealing the hero text and rising the house image from below the viewport.
- The house image stays bottom-centered behind the hero text on initial load.
- As the user scrolls past 30% of the hero, the house begins drifting upward and scaling up to 1.45×, remaining horizontally centered, while pinning toward the bottom of the dark statement section.
- The nav logo color cross-fades to white whenever a dark section sits at the viewport top.
- Section 5 stays sticky as Section 6 (gallery) slides up over it thanks to negative `margin-top: -100vh` and higher `z-index`.
- Stat numbers count up once on scroll into view.
- Gallery tiles accordion-expand on hover (desktop) or 2-column grid (mobile/tablet).

### Tech Notes

- Use only `react`, `react-dom`, `lucide-react`, Tailwind, and Vite. No additional libraries.
- All animation logic lives inside a single `App.tsx` using `useState`, `useEffect`, `useRef`, `useCallback`, and `IntersectionObserver`.
- Use Supabase if any persistence is later needed; this page itself has no data layer.

## Mindloop Landing — Landing Page [sites/mindloop-landing]

- Preview: https://motionsites.ai/assets/hero-mindloop-landing-preview-Bqnstohr.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/mindloop-landing.gif

Build a dark monochrome landing page called Mindloop — a newsletter/content platform. Use React + Vite + TypeScript + Tailwind CSS + shadcn/ui + Framer Motion. Fonts: Inter (sans) and Instrument Serif (serif, used for italic accent words). The entire theme is pure black (#000) background with white foreground — no colors or gradients beyond monochrome. Install hls.js and framer-motion.

Design System (index.css)
All CSS variables in HSL (no hsl() wrapper in the variable, just the values):

--background: 0 0% 0%
--foreground: 0 0% 100%
--card: 0 0% 5%
--card-foreground: 0 0% 100%
--primary: 0 0% 100%
--primary-foreground: 0 0% 0%
--secondary: 0 0% 12%
--secondary-foreground: 0 0% 85%
--muted: 0 0% 15%
--muted-foreground: 0 0% 65%
--accent: 170 15% 45%
--accent-foreground: 0 0% 100%
--border: 0 0% 20%
--input: 0 0% 18%
--ring: 0 0% 40%
--hero-subtitle: 210 17% 95%
Liquid Glass Effect (global CSS class .liquid-glass)

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
Animation Pattern
All sections use a reusable fadeUp helper with staggered delays:

const fadeUp = (delay: number) => ({
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, margin: "-100px" },
  transition: { duration: 0.6, delay, ease: "easeOut" },
});

Page Structure (top to bottom)
1. Navbar (fixed, transparent)
Left: Logo (concentric circles icon — outer w-7 h-7 with border-2 border-foreground/60, inner w-3 h-3 with border border-foreground/60) + "Mindloop" bold text.
Center-left: Nav links ["Home", "How It Works", "Philosophy", "Use Cases"] separated by • dots. Links are text-muted-foreground hover:text-foreground.
Right: 3 social icons (Instagram, Linkedin, Twitter from lucide-react) in liquid-glass circular buttons (w-10 h-10 rounded-full).
No background — fully transparent, fixed top-0 z-50, padding px-8 md:px-28 py-4.

2. Hero Section (full viewport height)
Background: autoplaying looping muted MP4 video covering the entire section.
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260325_120549_0cd82c36-56b3-4dd9-b190-069cfc3a623f.mp4
Bottom gradient: h-64 bg-gradient-to-t from-background to-transparent for smooth fade to black.
Content (centered, z-10, pt-28 md:pt-32):
Avatar row: 3 overlapping circular avatars (-space-x-2, w-8 h-8 rounded-full border-2 border-background) + "7,000+ people already subscribed" in text-muted-foreground text-sm.
Heading: text-5xl md:text-7xl lg:text-8xl font-medium tracking-[-2px] — "Get Inspired with Us" where "Inspired" is font-serif italic font-normal.
Subtitle: text-lg in hsl(var(--hero-subtitle)) color — "Join our feed for meaningful updates, news around technology and a shared journey toward depth and direction."
Email form: liquid-glass rounded-full p-2 max-w-lg container with email input and a white bg-foreground text-background rounded-full px-8 py-3 "SUBSCRIBE" button with whileHover scale 1.03 and whileTap scale 0.98.

3. "Search has changed" Section
Top padding pt-52 md:pt-64, bottom padding pb-6 md:pb-9.
Heading: text-5xl md:text-7xl lg:text-8xl — "Search has changed. Have you?" with "changed." in serif italic.
Subtitle: text-muted-foreground text-lg max-w-2xl mx-auto mb-24.
3 platform cards (grid md:grid-cols-3 gap-12 md:gap-8 mb-20): Each card has a 200x200 icon image centered, platform name (font-semibold text-base), and description (text-muted-foreground text-sm).
ChatGPT icon: local asset icon-chatgpt.png
Perplexity icon: local asset icon-perplexity.png
Google AI icon: local asset icon-google.png
Bottom tagline: "If you don't answer the questions, someone else will." in text-muted-foreground text-sm text-center.

4. Mission Section
Padding pt-0 pb-32 md:pb-44.
Video: Large 800x800 looping autoplaying muted video centered.
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260325_132944_a0d124bb-eaa1-4082-aa30-2310efb42b4b.mp4
Scroll-driven word-by-word reveal using useScroll and useTransform from framer-motion:
Paragraph 1 (text-2xl md:text-4xl lg:text-5xl font-medium tracking-[-1px]): "We're building a space where curiosity meets clarity — where readers find depth, writers find reach, and every newsletter becomes a conversation worth having." Words "curiosity", "meets", "clarity" are highlighted in --foreground, rest in --hero-subtitle.
Paragraph 2 (text-xl md:text-2xl lg:text-3xl font-medium mt-10): "A platform where content, community, and insight flow together — with less noise, less friction, and more meaning for everyone involved."
Each word transitions opacity from 0.15 to 1 based on scroll progress.

5. Solution Section
Padding py-32 md:py-44, border-t border-border/30.
Label: "SOLUTION" in text-xs tracking-[3px] uppercase text-muted-foreground.
Heading: text-4xl md:text-6xl — "The platform for meaningful content" (serif italic on "meaningful").
Video: Rounded rounded-2xl, aspect-[3/1] object-cover.
Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260325_125119_8e5ae31c-0021-4396-bc08-f7aebeb877a2.mp4
4-column feature grid (md:grid-cols-4 gap-8): Curated Feed, Writer Tools, Community, Distribution — each with title (font-semibold text-base) and description (text-muted-foreground text-sm).

6. CTA Section
Padding py-32 md:py-44, border-t border-border/30, overflow-hidden.
Background video (HLS via hls.js): absolute inset-0 object-cover z-0.
HLS URL: https://stream.mux.com/8wrHPCX2dC3msyYU9ObwqNdm00u3ViXvOSHUMRYSEe5Q.m3u8
Uses Hls.isSupported() check with fallback to native HLS for Safari.
Overlay: absolute inset-0 bg-background/45 z-[1].
Content (z-10, centered):
Concentric circles logo icon (w-10 h-10 outer, w-5 h-5 inner).
Heading: "Start Your Journey" (serif italic).
Subtitle in text-muted-foreground.
Two buttons: "Subscribe Now" (bg-foreground text-background rounded-lg px-8 py-3.5) and "Start Writing" (liquid-glass rounded-lg).

7. Footer
Simple py-12 px-8 md:px-28 footer.
Left: "© 2026 Mindloop. All rights reserved." in text-muted-foreground text-sm.
Right: Privacy, Terms, Contact links in text-muted-foreground text-sm hover:text-foreground.

Key Dependencies
framer-motion for all animations
hls.js for the CTA background video streaming
@fontsource/inter (400, 500, 600, 700)
@fontsource/instrument-serif (400, 400-italic)
lucide-react for icons
tailwindcss-animate plugin

Assets Needed
3 avatar images (avatar-1.png, avatar-2.png, avatar-3.png)
3 platform icons (icon-chatgpt.png, icon-perplexity.png, icon-google.png)

## Mythic Naturecore — landing page [sites/mythic-naturecore]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(72).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/mythic-naturecore.webp

Recreate a high-fidelity, premium interactive landing page named "Reverie" using React, TypeScript, and a combination of Tailwind CSS and inline styles. The project must have a smooth, hardware-accelerated scroll-linked animation system, 3D/parallax mouse-tracking effects, responsive layouts, and elegant micro-animations.

---

1. Typography & Global Styles

- Fonts:
  - Load the following Google Fonts:
    - Headers: `'Viaoda Libre', serif` (elegant serif font).
    - Body, nav links, and captions: `'Imprima', sans-serif` (clean, sleek sans-serif font).
- Global Reset & Base CSS:
  - `html, body { margin: 0; padding: 0; background: #0a0608; scroll-behavior: auto; }`
  - Body font should default to `'Imprima', sans-serif`.
  - Add `scrollbar-gutter: stable;` to the `html` tag to prevent layout shifts.
  - Include an animation utility:
    ```css
    @keyframes bobUp {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-6px); }
    }
    ```

---

2. Assets Asset Mapping

Define these exact asset constants at the top of the file:
```typescript
const PORTAL_BG = 'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779974947/portal_bg_mu60k9.png';
const CURTAIN_LEFT = 'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975070/curtain_left_cdht6q.png';
const CURTAIN_RIGHT = 'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975071/curtain_right_a9bn3i.png';
const WORLD_BG = 'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975077/world_bg_jzzcn1.jpg';

// The cards MUST remain in this exact order (Card 3, Card 1, Card 2)
const CARD_IMAGES = [
  'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975070/card_3_nbwm25.jpg',
  'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975070/card_2_wr6al6.jpg', // Representing Card 1
  'https://res.cloudinary.com/dsdhxhhqh/image/upload/v1779975070/card_1_jz8otj.jpg', // Representing Card 2
];
```

---

3. State Management & Mathematical Helpers

- Math Utilities:
  - Easing curve: `easeInOut(t) = t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t`
  - Linear Interpolation: `lerp(a, b, t) = a + (b - a) * t`
  - Constraint: `clamp(val, min, max) = Math.max(min, Math.min(max, val))`
- Parallax Magnitudes:
  - `MAG.world = 6`, `MAG.portal = 7`, `MAG.curtainL = 14`, `MAG.curtainR = 14`
- Hook for Responsiveness:
  - Implement a `useIsMobile()` hook responding to media query max-width of `767px` to dynamically update layouts.
- Scroll Tracking:
  - The page height must be exactly `480vh`. Inside, a single sticky container spans `100vh`.
  - Calculate normalized `scrollProgress` from `0` to `1` by reading window scroll position relative to the scrollable height.
- Smooth Mouse Tracking (Parallax):
  - Normalize coordinates `rx`, `ry` between `-1` and `1` relative to the center of the viewport.
  - Implement a `requestAnimationFrame` render loop (`tick`) to smoothly interpolate current position towards target cursor position (lerp step speed: `0.07`) to eliminate frame-rate stutters.
- Entrance Animation Delays:
  - On mount, transition curtains open after `100ms`, fade UI in after `600ms`. Disable entry CSS transitions after `2200ms` so mouse movement doesn't experience lag or delay.

---

4. Animation Timelines (Scroll & Mouse Parallax)

Apply these precise styling updates in the render loop on every frame:
1. World Layer (`WORLD_BG`):
   - Scale: Lerps from `1` (at start) to `1.18` (at maximum scroll).
   - Parallax: `transform = scale(${scale}) translate3d(${rx * 6}px, ${ry * 6}px, 0)`
2. Portal Frame (`PORTAL_BG`):
   - Scale: Lerps from `1` to `7.5` (creating an immersive zoom-through effect).
   - Origin: `52% 38%`
   - Opacity: Starts at `1`, fades out after `65%` scroll: `clamp(1 - (scrollProgress - 0.65) / 0.2, 0, 1)`
   - Parallax: `transform = scale(${scale}) translate3d(${rx * 7}px, ${ry * 7}px, 0)`
3. Curtain Left (`CURTAIN_LEFT`):
   - Initial Opening Offset: `62%` shift left.
   - Scroll Offset: Moves further leftward up to `150%` as eased progress goes `0` to `1`.
   - Curtain Scroll Scale: Lerps from `1` to `1.3`.
   - Parallax & GPU Layer: `transform = translateX(calc(-${totalShift}% + ${rx * 14}px)) translateY(${ry * 14 * 0.3}px) scale(${curtainScrollScale}) translateZ(0)`
4. Curtain Right (`CURTAIN_RIGHT`):
   - Symmetrically mirrors Curtain Left.
   - Parallax & GPU Layer: `transform = translateX(calc(${totalShift}% + ${rx * 14}px)) translateY(${ry * 14 * 0.3}px) scale(${curtainScrollScale}) translateZ(0)`

---

5. Layout & Components

Navigation Bar
- Position: Absolute at the top, `zIndex: 50`. Responsive padding: `18px 20px` (mobile), `22px 48px` (desktop).
- Desktop (>=768px): Split navigation.
  - Left side: Links `Worlds`, `Atelier`, `Immersions`.
  - Center: SVG Star Logo (clean star shape in path `M14 2l2.09 6.42H23l-5.45 ...` inside a `28x28` viewport).
  - Right side: Links `Craft`, `Codex`, `Connect`.
- Mobile (<768px): Centered star logo with an `Explore` link on the left and a `Connect` link on the right.
- Link Styling: uppercase, `12px`, letter spacing `0.12em`, white color with `0.9` opacity, no text decoration.

Scene 1: Hero Section (Entrance)
- Opacity: Fades out smoothly on scroll: `clamp(1 - scrollProgress / 0.22, 0, 1)`.
- Entrance Transition: Slide upward by `20px` on mount with opacity transition `0.9s ease` delayed by `300ms`.
- Responsive Layout:
  - Mobile (<768px): Center-aligned vertical column. Text is dark brown (`#3b1a0a`). Heading: `FALL › INTO REVERIE` (Viaoda Libre). Subheading paragraph (max-width `280px`). Below it, displays a single card with image `CARD_IMAGES[0]`, showing a rounded white play button icon and "View Reel".
  - Tablet (768px - 1099px): Center-aligned column. Text is dark brown (`#3b1a0a`). Headline and paragraph (max-width `400px`). Shows all 3 cards in a horizontal row:
    - Card 3: Image `CARD_IMAGES[0]`, Play button + "View Reel"
    - Card 1: Image `CARD_IMAGES[1]`, "32 World Patrons" in large elegant text
    - Card 2: Image `CARD_IMAGES[2]`, Play button + "View Reel"
  - Desktop (>=1100px): Split-screen horizontal layout. Text is white.
    - Left Container: Aligned to the left (top `46%`, left `60px`). Title: `FALL › INTO REVERIE` (Viaoda Libre). Subheading paragraph. Max-width `440px`.
    - Right Container: Aligned to the right (top `50%`, right `40px`). Row of 3 card containers (`158px x 158px`) with rounded corners (`28px`), bottom linear gradient, glassmorphic bottom blur (`backdropFilter: 'blur(6px)'`), play icon buttons or patron metrics overlay.
- Card Interactive Styling:
  - Backdrop blur filter on bottom labels: `backdropFilter: 'blur(6px)'`, linear gradient to top `rgba(0,0,0,0.72) 0%, rgba(0,0,0,0.18) 60%, transparent 100%`.
- Slider Dots (Bottom Left):
  - Absolutely positioned at bottom left (`60px` desktop, centered mobile).
  - Renders 4 horizontal pill indicators: first indicator is wide (`28px`), other three are thin (`14px`), colored in white with opacities.
- Scroll Cue (Descend):
  - Absolutely positioned at `bottom: 36px`, centered horizontally. Hidden on mobile.
  - Text: uppercase "Descend" in `10px`, letter-spacing `0.22em`, color `rgba(255,255,255,0.6)`.
  - Icon: A chevron SVG surrounded by a `34px x 34px` round circular border animated with the `bobUp 1.8s ease-in-out infinite` bounce animation.

Scene 2: Call to Action (Forge Beyond)
- Opacity: Fades in on scroll: `clamp((scrollProgress - 0.68) / 0.16, 0, 1)`.
- Layout: Centered vertical flex container (`zIndex: 46`), active only when opacity is visible.
- Content:
  - Centered text wrapper.
  - Heading: `FORGE BEYOND THE REAL` (Viaoda Libre, size clamp `38px` to `78px`, color `#ffffff`, letter spacing `0.03em`, line-height `1.05`, elegant text shadow `0 2px 20px rgba(0,0,0,0.4)`).
  - Paragraph: `Singular voyages to astonishing destinations, shaped for those who seek beauty beyond the ordinary and the known.` (Imprima, size `20px` desktop / `14px` mobile, max-width `480px` desktop / `260px` mobile, line-height `1.6`, color `rgba(255,255,255,0.82)`).
```

## Neon Logic — Landing Page [sites/neon-logic]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(33).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/neon-logic.webp

Build a single-page landing site for "SynapseX" -- a futuristic neural-AI interface product. The entire site uses a black background with white text and full-viewport video backgrounds. The primary font is "Space Mono" (monospace) for all text. Use React + TypeScript + Vite + Tailwind CSS + Framer Motion.

### Fonts & External Assets

- **Primary font:** "Space Mono" (all weights: 400, 700, italic) from Google Fonts
- **Display font (background watermark only):** "Anton SC" from Google Fonts
- **Icons:** Bootstrap Icons CDN (`https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css`) -- used only for the Apple icon (`bi bi-apple`) in the download button
- **All Tailwind `fontFamily` keys** (`sans`, `serif`, `mono`) are overridden to `"Space Mono", monospace`

### Video URLs (CloudFront -- use exactly these)

1. **Hero (mouse-scrubbed, NOT autoplay):**
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_083515_290e5a10-0b95-41af-a5e2-32b6389baa4d.mp4`

2. **Second Section (autoplay, muted, loop):**
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_092455_089c54f8-3b03-4966-9df1-e9746063d0ef.mp4`

3. **Metrics Section (autoplay, muted, loop):**
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_095810_ecea3dd2-fc5e-4e41-8696-4219290b6589.mp4`

4. **Technology Section (autoplay, muted, loop):**
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_095750_32a52ce0-2005-45c9-9093-41f03fde9530.mp4`

5. **Footer (autoplay, muted, loop):**
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260622_080203_fd7f4f85-3a86-4837-8192-85e7bfe68e75.mp4`

### Dependencies

```json
"framer-motion": "^12.40.0",
"lucide-react": "^0.344.0",
"react": "^18.3.1",
"react-dom": "^18.3.1"
```

### Global CSS (`index.css`)

- Import Space Mono from Google Fonts
- Import Bootstrap Icons CSS
- Tailwind directives (`@tailwind base/components/utilities`)
- CSS variables: all `--font-*` set to `"Space Mono", monospace`
- Global reset: `* { margin: 0; padding: 0; box-sizing: border-box; }`
- `html, body`: `background: #000; color: #fff; overflow-x: hidden; overflow-y: auto; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;`
- Lenis smooth scroll utility classes (`.lenis.lenis-smooth`, `.lenis.lenis-stopped`, `.lenis.lenis-scrolling iframe`)

### Custom Text Animation Components

### 1. `ScrambleIn` -- entrance reveal animation
- Props: `text: string`, `delay: number` (ms before start), `triggered: boolean`
- Character set: `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+~|}{[]:;?><`
- On trigger (after delay): runs interval every 25ms, revealing characters left-to-right at 0.5 chars/frame
- Characters not yet revealed show random chars (up to 3 ahead of the reveal cursor); characters beyond that are empty
- Spaces always show as spaces
- Before triggered: renders `&nbsp;`

### 2. `ScrambleText` -- hover-driven scramble
- Props: `text: string`, `isHovered: boolean`, `className?: string`
- On hover: scrambles all chars with random chars, then reveals left-to-right at 4 frames/char, interval 25ms
- On unhover: immediately resets to original text

### Custom SVG Logo (`SynapseXLogo`)

A 4-fold rotationally symmetric abstract shape rendered in an SVG with `viewBox="-50 -50 100 100"`. Each quadrant is the same path rotated 0/90/180/270 degrees:

```
M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z
```

### Animated Hamburger (`SquashHamburger`)

- 3 horizontal bars (absolute positioned spans)
- Desktop: container 18x12px, bar height 1.5px
- Mobile: container 15x10px, bar height 1.2px
- On open: top bar rotates 45deg + translates down to center; middle bar fades/scales out; bottom bar rotates -45deg + translates up to center
- Spring animation: stiffness 300, damping 20

---

### Page Sections (in order)

---

### SECTION 1: Hero (full viewport height)

- **Background:** Video #1, `object-cover`, paused on load. Controlled by mouse-scrub: horizontal mouse movement across viewport scrubs the video timeline forward/backward. Sensitivity factor: `0.8`. Uses `seeked` event to chain seeks without dropping frames.
- **Entrance animation:** After 800ms delay, `entranceComplete` state becomes true -- all hero content fades in (opacity 0 -> 1, duration 1s).
- **Dot grid overlay:** `radial-gradient(#ffffff 1px, transparent 1px)` with 24x24px grid, opacity 0.05, pointer-events-none
- **Large background watermark text:** The word "TRANSCENDENCE" in "Anton SC" font, centered vertically (offset +50px from center), `clamp(120px, 30vw, 521px)` font size, uppercase, letter-spacing -4px. Opacity 0.10. Color achieved via `radial-gradient(circle, rgba(142,127,148,0) 0%, #8E7F94 70%)` as `background-clip: text` with transparent fill.
- **Layout:** Flexbox column, padding `px-4 sm:px-6 md:px-8 pt-20 sm:pt-24 pb-8 sm:pb-12`. Content is pushed to the bottom using `flex-1` spacer.
- **Bottom row:** `flex-col gap-6 md:flex-row md:items-end md:justify-between`
  - **Left column** (`flex flex-col gap-4`):
    - **h1** "Brain" / "And Body" (two lines via `<br>`): `text-white font-light leading-[0.95] tracking-[-0.03em] text-[clamp(40px,10vw,100px)]`. Each line uses `ScrambleIn` with delays 200ms and 500ms.
    - **Description paragraph** (motion.p): fade-up animation (y:25->0, opacity 0->1, duration 0.9s, cubic-bezier ease `[0.215, 0.610, 0.355, 1.000]`, delay 0.2s). Text: "Built at the intersection of neuroscience and artificial intelligence. SynapseX continuously maps neural pathways, cognitive load, and physiological states into a single adaptive intelligence layer." Style: `max-w-sm text-[13px] sm:text-[15px] text-white/60 leading-relaxed`
  - **Right h1** "One" / "Network": Same styling as left h1 but with `text-left md:text-right`. ScrambleIn delays: 700ms and 1000ms.

---

### NAVBAR (fixed, z-50)

- Fixed to top, height 80px (h-20), transparent background, full width
- Fades in with `entranceComplete` (opacity 0->1, duration 0.8s)

**Desktop (hidden below `sm`):**
- Left group: two pills side by side with gap-2
  - **Logo pill:** h-12, px-5, `bg-white/15 backdrop-blur-md rounded-[14px]`. Contains SynapseXLogo (18x18px white) + "SynapseX" text (16px font-medium tracking-tight white). WhileHover: scale 1.02 + bg rgba(255,255,255,0.22). WhileTap: scale 0.98. Hides on `sm` when menu open (`hidden md:flex`), shows normally otherwise.
  - **Expanding menu pill:** Animates width from 48px (closed) to 290px (open) with spring (stiffness 350, damping 28). h-12, `rounded-[14px]`, `bg-white/15 backdrop-blur-md`. Contains:
    - Hamburger button: when closed = 48x48px rounded-[14px]; when open = 36x36px rounded-[11px] with `bg-white/10 hover:bg-white/20 ml-1.5`
    - Nav links (fade in when open, offset x:15->0): "About" and "Metrics" with ScrambleText on hover. 16px font-normal text-white/85 hover:text-white. Smooth-scroll to `window.innerHeight` and `window.innerHeight * 2` respectively.
- Right: **Download button** -- `h-12 px-6 bg-white rounded-full`, black text. Apple icon + "Download" with ScrambleText on hover. WhileHover: scale 1.03 + bg #e2e2e6. WhileTap: scale 0.97.

**Mobile (visible below `sm`):**
- Scaled-down version: h-9 pills, rounded-[10px], smaller text (13px), logo pill animates to width 0 when menu open (spring stiffness 350, damping 28). Menu capsule expands to 100% width when open. Download button: h-9 px-3.5 rounded-full.

---

### SECTION 2: Cinematic Text (full viewport height)

- **Background:** Video #2, autoplay muted loop, object-cover
- **Top gradient overlay:** 180px height, linear-gradient from `#010103` to transparent, z-10
- **Content:** Centered large paragraph in a `max-w-5xl` container with 3D perspective (400px)
  - Framer Motion: `rotateX(24deg) translateY(${yScaleValue}px) translateZ(15px)` where `yScaleValue` transforms from 60 to -120 based on smooth scroll progress (spring: stiffness 15, damping 32, mass 1.8). Opacity fades in from 0 to 1 between scroll progress 0.3-0.5.
  - Text: "A neural-AI interface built on the architecture of the human nervous system. SynapseX translates synaptic activity into computational intelligence. Every signal becomes measurable, structured, and visible. It continuously reconstructs internal state as a dynamic neural map. Biological noise is filtered into actionable cognitive patterns."
  - Style: `font-sans font-normal text-[22px] sm:text-[30px] md:text-[36px] lg:text-[42px] text-white leading-[1.35] tracking-[-0.02em] select-none px-6 sm:px-12 text-center`

---

### SECTION 3: Metrics (min-h-screen)

- **Background:** Video #3, autoplay muted loop, object-cover
- **Layout:** Centered content, `pt-32 pb-32 px-6`, max-w-6xl
- **Subtitle:** "Performance Metrics" -- `text-white/40 text-[13px] sm:text-[14px] tracking-[0.2em] uppercase mb-20 text-center`. Fades in on scroll (whileInView, duration 1.2s, once, amount 0.3).
- **Metrics grid:** 3 columns on md, 1 on mobile, gap-16 md:gap-8. Each metric fades up (y:30->0, opacity, duration 0.8s, staggered 0.15s delay per item):
  - "2.4ms" -- Synaptic Latency
  - "99.7%" -- Signal Accuracy
  - "140B" -- Neural Parameters
  - Value: `text-white text-[clamp(48px,10vw,96px)] font-light tracking-[-0.04em] leading-none`
  - Label: `text-white/40 text-[13px] sm:text-[15px] mt-4 tracking-wide`

---

### SECTION 4: Technology / Adaptive Intelligence (full viewport height)

- **Background:** Video #4, autoplay muted loop, object-cover
- **Layout:** Flexbox column, `px-8 sm:px-12 md:px-16 py-12 sm:py-16`
- **Top area:** flex-col md:flex-row md:justify-between md:items-start gap-6
  - **Left heading:** "Adaptive / Intelligence" (two lines), `text-white font-light text-[clamp(36px,8vw,72px)] leading-[0.95] tracking-[-0.03em]`. Fades up (y:40->0, duration 1.0s, whileInView once amount 0.3).
  - **Right paragraph:** "The system learns your neural baseline within 72 hours. From there, every cognitive state is mapped, predicted, and optimized in real time." `text-white/50 text-[13px] sm:text-[15px] leading-relaxed max-w-xs md:text-right md:pt-2`. Fades up (y:20->0, duration 1.0s, delay 0.2s).
- **Spacer** (`flex-1`)
- **Bottom grid:** 2 cols on mobile, 4 cols on md, gap-8 md:gap-6. Fades in on scroll (duration 1.0s, delay 0.3s). Each item staggered (y:20->0, duration 0.7s, delay i*0.1):
  1. "Cortical Mapping" -- "Real-time spatial reconstruction of active neural regions."
  2. "Signal Isolation" -- "Separates cognitive intent from biological noise."
  3. "State Prediction" -- "Anticipates cognitive transitions before they occur."
  4. "Loop Feedback" -- "Closed-loop adjustment based on outcome correlation."
  - Title: `text-white text-[14px] sm:text-[16px] font-normal mb-2`
  - Desc: `text-white/40 text-[12px] sm:text-[14px] leading-relaxed`

---

### SECTION 5: Architecture (min-h-screen, pure black background, no video)

- Centered content, max-w-3xl, `px-6 py-32`
- **Heading block** (fades up y:30->0, duration 1.0s, whileInView once amount 0.4):
  - Subtitle: "Architecture" -- `text-white/40 text-[13px] sm:text-[14px] tracking-[0.2em] uppercase mb-8`
  - Heading: "Three layers. Zero friction." -- `text-white font-light text-[clamp(28px,6vw,56px)] leading-[1.15] tracking-[-0.02em] mb-10`
  - Description: "Sensor layer captures raw bioelectric signals. Processing layer isolates intent. Interface layer delivers structured output to any connected system." -- `text-white/45 text-[15px] sm:text-[17px] leading-relaxed max-w-xl mx-auto`
- **Layer cards** (fade in, duration 1.2s, delay 0.4s, whileInView once amount 0.4): 3 stacked cards, `mt-20 flex-col items-center gap-4`. Each card: `max-w-md h-[72px] border border-white/10 rounded-lg flex items-center justify-between px-6`
  - Left: "Layer 1/2/3" -- `text-white/30 text-[12px] tracking-[0.15em] uppercase`
  - Right: "Capture" / "Process" / "Interface" -- `text-white text-[16px] sm:text-[18px] font-light`

---

### FOOTER

- Black background, overflow hidden
- Two-column layout (stacked on mobile): `flex-col md:flex-row min-h-[400px]`
- **Left:** Video #5, `object-cover`, fills half width (h-[300px] on mobile, auto height on md)
- **Right:** Flex column justify-between, `p-10 sm:p-16`
  - Top: SynapseXLogo (18x18px, text-white/70) + "SynapseX" text (15px font-medium text-white/70 tracking-tight), mb-8. Below: "The next evolution of human-machine interaction. Built for those who refuse to be limited by biology alone." `text-white/40 text-[14px] sm:text-[15px] leading-relaxed max-w-sm`
  - Bottom: "(c) 2026 SynapseX Labs. All rights reserved." `text-white/25 text-[12px] mt-12`

---

### Key Technical Details

- The entire app wrapper has inline style: `fontFamily: '"Space Mono", monospace'`
- All `h-screen` elements also have `h-[100dvh]` for mobile viewport compatibility
- The hero video is NOT autoplay -- it starts paused at time 0 and is scrubbed by horizontal mouse movement (delta-based, not absolute position). The seek logic chains via `seeked` event to avoid frame-dropping.
- Framer Motion `useScroll` tracks the second section with offset `["start end", "end start"]`, piped through `useSpring` (stiffness 15, damping 32, mass 1.8) then `useTransform` and `useMotionTemplate` for the 3D text rotation effect.
- No external state management, no routing, no database -- pure single-page React app.

---

## NeoVision — Landing Page [sites/neovision-landing]

- Preview: https://motionsites.ai/assets/hero-neovision-preview-qwRNOas1.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/neovision-landing.gif

Build a modern, futuristic landing page using React + Vite + TypeScript + Tailwind CSS. Use lucide-react for icons. The page has 3 sections: a Hero, an About section, and an Insights section. The overall page background is black (bg-black). No custom fonts -- use the Tailwind default (system sans-serif).

SECTION 1: HERO (full viewport height, light background)

The hero wrapper is a relative container. On mobile it has min-height: 100vh. On desktop (md: breakpoint) it has height: 100vh and min-height: auto. It has overflow: hidden.

Background layers (stacked with z-index):

z-index 0: A solid background fill #FBFDFD covering the entire wrapper (absolute inset-0).
z-index 1: A background video positioned absolute right-0 top-0 bottom-0. On mobile it is full width with opacity-30. On desktop it is w-[55%] with full opacity. The video element has object-cover object-top. The video URL is: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_131232_feeda0b7-d00d-4bfa-a9d5-5d38648a4214.mp4 (autoPlay, loop, muted, playsInline). The video container and video both have a CSS class video-plus-darker with mix-blend-mode: normal !important.
z-index 2: The content layer. This is a relative div with min-h-screen md:h-screen flex flex-col. It contains the Navbar and HeroSection.

Navbar (inside content layer):

relative z-10, flex row, justify-between, padding px-5 py-4 md:px-12 md:py-6
Left side: Logo image (/image.png, height h-7 md:h-8), followed by hidden-on-mobile nav links: Home, About, Services, Contact. Links are text-sm text-neutral-500 hover:text-neutral-900. Gap between logo and links: gap-12.
Right side: A search input (hidden on mobile, shown on desktop). Rounded-full, w-72, placeholder "I am looking for...", with a Search icon from lucide-react positioned absolute right inside. Border border-neutral-300, text text-neutral-600.
Mobile: A hamburger button (Menu icon from lucide-react) in a 40px circle with border-neutral-300. When toggled, a dropdown shows the nav links vertically and a full-width search input.

HeroSection (inside content layer, flex-1):

relative z-10 flex-1 flex flex-col justify-between with padding px-5 pt-8 pb-20 md:px-12 md:pt-16 md:pb-36
Top: Label "Futuristic" -- text-xs font-medium tracking-[0.3em] text-neutral-500 uppercase
Main heading area: Flex row with a small "05" number on the left (text-sm text-neutral-400 mt-2 md:mt-4), then the heading: text-[2.75rem] md:text-[5.5rem] leading-[0.95] font-light tracking-tight text-neutral-900 reading "NEW DIGITAL" (line break) "UNIVERSE". Below the heading: a "Get Started" button (bg-neutral-900 text-white text-sm font-medium rounded, px-6 py-3 md:px-8 md:py-3.5) and a "Contact Us" text link.
Middle stat: "47.2%" with "Reality" label underneath. The stat group uses a custom CSS class hero-stat-group that on desktop has margin-right: 20% and justify-center. On mobile, margin-right: 0 and justify-start.
Bottom bar: Flex col on mobile, flex row on desktop with justify-between. Left side: "Trusted by Clients" label with 4 overlapping avatar circles (Pexels photos: 415829, 1222271, 1239291, 2379004 at w=100) using -space-x-2, each w-8 h-8 md:w-9 md:h-9 rounded-full border-2 border-neutral-100 object-cover, followed by "20+" text. Right side: A link icon (lucide Link) in a w-10 h-10 md:w-12 md:h-12 circle with border-neutral-300, next to a description paragraph "In this futuristic realm, users can explore hyper-realistic virtual environments, interact with AI-driven avatars." (text-xs md:text-sm text-neutral-500 max-w-[200px] md:max-w-sm). This group uses custom class hero-description-group which on desktop has margin-right: 50%.

Diagonal Section Divider (between Hero and About):

The SectionDivider is positioned absolute bottom-0 left-0 w-full with z-index: 3, inside the hero wrapper.
Contains an SVG with viewBox="0 0 1440 120", preserveAspectRatio="none", height h-[60px] md:h-[120px].
The SVG has a single polygon: points="0,0 0,120 1440,120 1440,80 920,80 680,0" filled with #0F0F0F. This creates a diagonal cut from the left side going down to the black About section.

SECTION 2: ABOUT (dark background #0F0F0F)

Full-width section, backgroundColor: '#0F0F0F'
Two-column layout on desktop (lg:flex-row), stacked on mobile. Min-height 600px (lg: 700px).
Left column: A video that fills the space (object-cover mix-blend-lighten). Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260314_132809_d6ea910f-d700-44f7-afea-27d517487177.mp4 (autoPlay, loop, muted, playsInline). On mobile the video area is h-[400px].
Right column: Padding px-8 py-16 md:px-16 lg:px-20 xl:px-28, vertically centered content with max-width max-w-lg:
Label: "About Us" (text-xs font-medium tracking-[0.3em] text-neutral-500 uppercase, mb-8 md:mb-10)
Heading: "THE DIGITAL FRONTIER" (text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-light tracking-tight text-white leading-[1.05], mb-10 md:mb-12)
Three pill tags: "Digital", "Reality", "Next" -- px-5 py-2 rounded-full border border-neutral-700 text-sm text-neutral-300 hover:border-neutral-500
Paragraph: "Step into The Digital Frontier, where the boundaries between reality and virtual innovation disappear. This is the next era of immersive technology." (text-sm md:text-base text-neutral-400 leading-relaxed max-w-md)
Actions: "Learn More" button (bg-neutral-800 text-white text-sm font-medium rounded px-7 py-3.5 hover:bg-neutral-700) and a "Watch a Video" link with a Play icon (lucide Play) inside a w-10 h-10 bordered circle.

SECTION 3: INSIGHTS (dark background #0F0F0F, tabbed content)

Same backgroundColor: '#0F0F0F'. Padding px-8 md:px-16 lg:px-20 xl:px-28 pt-24 pb-32.
Large italic heading: "LIMITLESS POSSIBILITIES WITH NEOVISION" (text-4xl sm:text-5xl md:text-6xl lg:text-7xl xl:text-[5rem] font-light italic tracking-tight text-white leading-[1.05] max-w-5xl, mb-20 md:mb-28).
Below: a flex layout (col on mobile, row on desktop).
Left: Tab buttons (vertical list) for 3 tabs: "Innovation", "Technology", "Experience". Active tab is text-white font-medium, inactive is text-neutral-500 hover:text-neutral-300. Width lg:w-[160px] xl:w-[200px].
Right: Content area with an image and text side by side on desktop.
Image: lg:w-[420px] xl:w-[480px] aspect-[4/3] rounded-2xl overflow-hidden. Images are local files /Mask_group.jpg, /Mask_group-1.jpg, /Mask_group-2.jpg for the 3 tabs respectively.
Text side: Title (text-2xl md:text-3xl font-light text-white leading-snug max-w-sm), description (text-sm md:text-base text-neutral-400 leading-relaxed max-w-sm), "Learn More" underlined link (text-sm text-white font-medium underline underline-offset-4). At the bottom: date and author separated by a border-t border-neutral-800.
Tab data:
Innovation: "How VR is Transforming Our Digital World" / "Virtual Reality (VR) is no longer a concept of the future..." / 08 February 2025 / Henry Leonardo
Technology: "The Rise of Spatial Computing in Everyday Life" / "Spatial computing is bridging the gap..." / 15 March 2025 / Sarah Mitchell
Experience: "Designing Immersive Worlds That Feel Real" / "From haptic feedback to photorealistic rendering..." / 22 April 2025 / James Park

CUSTOM CSS (index.css):

.video-plus-darker {
  mix-blend-mode: normal !important;
}

.app-hero-wrapper {
  min-height: 100vh;
  overflow: hidden;
}

@media (min-width: 768px) {
  .app-hero-wrapper {
    min-height: auto;
    height: 100vh;
  }
}

.hero-description-group {
  margin-right: 0;
}

.hero-stat-group {
  margin-right: 0;
}

@media (min-width: 768px) {
  .hero-description-group {
    margin-right: 50%;
  }

  .hero-stat-group {
    margin-right: 20%;
  }
}

KEY DETAILS:

Tech stack: React 18, Vite, TypeScript, Tailwind CSS 3, lucide-react for all icons
No custom fonts -- default Tailwind sans-serif stack
Color palette: White hero (#FBFDFD), dark sections (#0F0F0F), neutral grays from Tailwind for text
The diagonal divider SVG polygon creates an angled transition from the light hero to the dark about section. It is positioned absolutely at the bottom of the hero wrapper.
Both videos are autoPlay, loop, muted, playsInline
The hero section has pb-20 md:pb-36 bottom padding to prevent the diagonal divider from overlapping the "Trusted by Clients" content
The SectionDivider component is rendered inside the hero wrapper div (not between wrapper and AboutSection)
Logo image at /image.png in the public folder
Insight images at /Mask_group.jpg, /Mask_group-1.jpg, /Mask_group-2.jpg in the public folder

## Neural Interface — Landing Page [sites/neural-interface]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(54).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/neural-interface.webp

<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>SynapseX</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=Space+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap" rel="stylesheet" />
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css" />
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swiper@11/swiper-bundle.min.css" />
<style>
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

html, body {
  font-family: "Space Mono", monospace;
  background: #000;
  color: #fff;
  width: 100%;
  height: auto;
  overflow-x: hidden;
  overflow-y: auto;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

html.lenis, html.lenis-under-construction { height: auto; }
.lenis.lenis-smooth { scroll-behavior: auto !important; }
.lenis.lenis-smooth [data-lenis-prevent] { overscroll-behavior: contain; }
.lenis.lenis-stopped { overflow: hidden; }
.lenis.lenis-scrolling iframe { pointer-events: none; }

a { text-decoration: none; color: inherit; }
button { border: none; outline: none; background: none; font-family: inherit; cursor: pointer; }
button:focus { outline: none; }

/* ── Video Layer ── */
#video-layer {
  position: fixed; inset: 0;
  width: 100%; height: 100%;
  overflow: hidden; z-index: 1;
  background: #000;
  user-select: none;
}
#video-layer video {
  position: absolute; inset: 0;
  width: 100%; height: 100%;
  object-fit: cover;
  pointer-events: none;
  opacity: 0;
  will-change: transform, filter, opacity;
}

/* ── Progressive Bottom Blur ── */
#bottom-blur {
  position: fixed; bottom: 0; left: 0;
  width: 100%; height: 150px;
  z-index: 30;
  pointer-events: none; user-select: none;
  background: linear-gradient(to bottom, transparent, #000);
  -webkit-mask-image: linear-gradient(to top, #000 50%, transparent);
  mask-image: linear-gradient(to top, #000 50%, transparent);
  -webkit-backdrop-filter: blur(4px);
  backdrop-filter: blur(4px);
}

/* ── Header ── */
#main-header {
  position: fixed; top: 0; left: 0; right: 0;
  height: 80px; z-index: 50;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 2rem;
  opacity: 0;
  transition: opacity 0.8s ease-out;
  pointer-events: auto;
}
#main-header.visible { opacity: 1; }

.header-left { display: flex; align-items: center; gap: 8px; }

/* Logo Pill */
.logo-pill {
  height: 48px; padding: 0 20px;
  background: rgba(255,255,255,0.15);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border-radius: 14px;
  display: flex; align-items: center; gap: 10px;
  cursor: pointer;
  transition: background 0.15s, transform 0.15s;
  user-select: none;
}
.logo-pill:hover { background: rgba(255,255,255,0.22); transform: scale(1.02); }
.logo-pill:active { transform: scale(0.98); }
.logo-pill svg { width: 18px; height: 18px; color: #fff; flex-shrink: 0; }
.logo-pill span {
  font-size: 16px; font-weight: 500;
  letter-spacing: -0.02em; color: #fff; line-height: 1;
  white-space: nowrap;
}

/* Hamburger Menu Pill */
.menu-pill {
  height: 48px; width: 48px;
  background: rgba(255,255,255,0.15);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border-radius: 14px;
  display: flex; align-items: center;
  overflow: hidden;
  transition: width 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  position: relative;
}
.menu-pill.open { width: 290px; }

.hamburger-btn {
  width: 48px; height: 48px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; position: relative; z-index: 2;
  transition: all 0.2s;
}
.menu-pill.open .hamburger-btn {
  width: 36px; height: 36px;
  border-radius: 11px;
  background: rgba(255,255,255,0.1);
  margin-left: 6px;
}
.menu-pill.open .hamburger-btn:hover { background: rgba(255,255,255,0.2); }

.hamburger-icon {
  width: 18px; height: 12px; position: relative;
}
.hamburger-icon span {
  position: absolute; left: 0;
  width: 18px; height: 1.5px;
  background: #fff; border-radius: 1px;
  transition: transform 0.3s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.3s;
  transform-origin: center;
}
.hamburger-icon span:nth-child(1) { top: 0; }
.hamburger-icon span:nth-child(2) { top: 5px; }
.hamburger-icon span:nth-child(3) { top: 10px; }
.menu-pill.open .hamburger-icon span:nth-child(1) { transform: translateY(5px) rotate(45deg); }
.menu-pill.open .hamburger-icon span:nth-child(2) { opacity: 0; transform: scale(0); }
.menu-pill.open .hamburger-icon span:nth-child(3) { transform: translateY(-5px) rotate(-45deg); }

.menu-links {
  display: flex; align-items: center; gap: 24px;
  margin-left: auto; padding-right: 24px;
  opacity: 0; transform: translateX(15px);
  transition: opacity 0.15s, transform 0.15s;
  pointer-events: none; flex-shrink: 0;
  white-space: nowrap;
}
.menu-pill.open .menu-links {
  opacity: 1; transform: translateX(0);
  pointer-events: auto;
}
.menu-links span {
  font-size: 16px; font-weight: 400;
  color: rgba(255,255,255,0.85);
  cursor: pointer; transition: color 0.15s; line-height: 1;
}
.menu-links span:hover { color: #fff; }

/* Download Button */
.download-btn {
  height: 48px; padding: 0 24px;
  background: #fff; border-radius: 9999px;
  display: flex; align-items: center; gap: 10px;
  color: #000; transition: background 0.15s, transform 0.15s;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
.download-btn:hover { background: #e2e2e6; transform: scale(1.03); }
.download-btn:active { transform: scale(0.97); }
.download-btn i { font-size: 16px; color: #000; transform: translateY(-1px); }
.download-btn span { font-size: 16px; font-weight: 500; color: #000; line-height: 1; }

/* ── Main Content ── */
#main-content {
  position: relative; width: 100%;
  display: flex; flex-direction: column;
  padding: 80px 2rem 144px;
  z-index: 10; pointer-events: auto;
  opacity: 0;
  transition: opacity 1s ease-out;
}
#main-content.visible { opacity: 1; }

.dot-grid {
  position: absolute; inset: 0;
  background-image: radial-gradient(#fff 1px, transparent 1px);
  background-size: 24px 24px;
  opacity: 0.05; pointer-events: none;
}

/* ── Hero Section ── */
#hero-section {
  position: relative; width: 100%; max-width: 1280px; margin: 0 auto;
  display: flex; flex-direction: column;
  min-height: 80vh;
  justify-content: space-between;
  padding-top: 32px; padding-bottom: 64px;
  transition: opacity 0.1s linear, transform 0.1s linear;
}

.hero-grid {
  display: grid; grid-template-columns: 1fr 1fr;
  gap: 48px; width: 100%;
}
.hero-grid-bottom {
  display: grid; grid-template-columns: 1fr 1fr;
  gap: 48px; width: 100%; margin-top: auto; padding-top: 48px;
  align-items: end;
}

.hero-title {
  font-weight: 300;
  font-size: clamp(50px, 8vw, 100px);
  color: #fff; line-height: 0.95;
  letter-spacing: -0.03em;
  display: flex; flex-direction: column;
  user-select: none;
}
.hero-title.right { align-items: flex-end; text-align: right; }

.hero-desc {
  max-width: 380px;
  font-size: 15px; color: rgba(255,255,255,0.6);
  line-height: 1.625;
  transition: opacity 0.1s linear, transform 0.1s linear;
}

/* ── Cinematic Paragraph ── */
#cinematic-section {
  position: relative; width: 100%; max-width: 1024px; margin: 0 auto;
  padding: 96px 24px 128px;
  perspective: 400px;
  pointer-events: none;
}
#cinematic-inner {
  transform-style: preserve-3d;
  text-align: center;
  transition: opacity 0.05s linear;
}
#cinematic-inner h2 {
  font-weight: 400;
  font-size: clamp(22px, 3.5vw, 42px);
  color: #fff; line-height: 1.35;
  letter-spacing: -0.02em;
  user-select: none;
  padding: 0 24px;
}

/* ── Stats Section ── */
#stats-section {
  width: 100vw; position: relative;
  margin-left: calc(-50vw + 50%);
  margin-top: 64px;
  overflow: hidden;
  opacity: 0; transform: translateY(40px) scale(0.98);
  transition: opacity 0.6s ease-out, transform 0.6s ease-out;
}
#stats-section.revealed { opacity: 1; transform: translateY(0) scale(1); }

.swiper { width: 100%; height: 520px; padding-bottom: 20px !important; overflow: visible !important; }
.swiper-slide { width: 380px; max-width: 85%; height: 480px; background-position: center; background-size: cover; }

.stat-card-outer {
  padding: 6px; border-radius: 28px;
  background: rgba(255,255,255,0.04);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  display: flex; flex-direction: column;
  justify-content: space-between; height: 480px;
}
.stat-card-inner {
  background: rgba(0,0,0,0.45);
  border: 1px solid rgba(255,255,255,0.05);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border-radius: 23px; padding: 32px;
  display: flex; flex-direction: column;
  justify-content: space-between; flex: 1;
}
.stat-title {
  font-family: "Space Mono", monospace;
  font-size: 11px; font-weight: 700;
  text-transform: uppercase; letter-spacing: 0.08em;
  color: #fff; opacity: 0.8;
}
.stat-value {
  font-size: clamp(60px, 6vw, 76px);
  font-weight: 400; letter-spacing: -0.04em;
  color: #fff; line-height: 1;
  margin-top: 24px;
}
.stat-details { display: flex; flex-direction: column; gap: 8px; padding-top: 16px; }
.stat-detail {
  display: flex; align-items: flex-start; gap: 8px;
  font-size: 11px; color: rgba(255,255,255,0.6); font-weight: 500;
}
.stat-detail .dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: rgba(255,255,255,0.3);
  margin-top: 4px; flex-shrink: 0;
}
.stat-footer {
  padding: 12px 24px 10px;
  font-family: "Space Mono", monospace;
  font-size: 10px; font-weight: 500;
  color: rgba(255,255,255,0.55);
  text-transform: uppercase; letter-spacing: 0.1em;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}

/* ── Mobile ── */
@media (max-width: 639px) {
  #main-header { padding: 0 16px; }
  .desktop-header { display: none !important; }
  .mobile-header { display: flex !important; }
  #main-content { padding: 80px 16px 144px; }
  .hero-grid, .hero-grid-bottom { grid-template-columns: 1fr; }
  .hero-title.right { align-items: flex-start; text-align: left; }
  .hero-desc { font-size: 14px; }
  #cinematic-section { padding: 64px 16px 96px; }
}
@media (min-width: 640px) {
  .mobile-header { display: none !important; }
  .desktop-header { display: flex !important; }
}

/* Mobile header */
.mobile-header {
  align-items: center; justify-content: space-between;
  width: 100%; height: 100%; gap: 8px;
}
.mobile-left {
  display: flex; align-items: center; height: 36px;
  flex-grow: 1; min-width: 0; margin-right: 16px; position: relative;
}

.logo-pill-m {
  height: 36px; padding: 0 12px;
  background: rgba(255,255,255,0.15);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border-radius: 10px;
  display: flex; align-items: center; gap: 6px;
  cursor: pointer; flex-shrink: 0;
  overflow: hidden; white-space: nowrap;
  transition: width 0.35s cubic-bezier(0.22,1,0.36,1), opacity 0.35s, margin-right 0.35s, padding 0.35s;
  width: 108px; margin-right: 6px;
}
.logo-pill-m.collapsed {
  width: 0; opacity: 0; margin-right: 0;
  padding: 0; pointer-events: none;
}
.logo-pill-m svg { width: 14px; height: 14px; color: #fff; flex-shrink: 0; }
.logo-pill-m span { font-size: 13px; font-weight: 500; letter-spacing: -0.02em; color: #fff; line-height: 1; flex-shrink: 0; }

.menu-pill-m {
  height: 36px; width: 36px;
  background: rgba(255,255,255,0.15);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border-radius: 10px;
  display: flex; align-items: center;
  overflow: hidden; flex-shrink: 0;
  transition: width 0.35s cubic-bezier(0.22,1,0.36,1);
}
.menu-pill-m.open { width: 100%; }

.hamburger-btn-m {
  width: 36px; height: 36px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; z-index: 2; transition: all 0.2s;
}
.menu-pill-m.open .hamburger-btn-m {
  width: 28px; height: 28px;
  border-radius: 8px; background: rgba(255,255,255,0.1);
  margin-left: 4px;
}

.hamburger-icon-m { width: 15px; height: 10px; position: relative; }
.hamburger-icon-m span {
  position: absolute; left: 0; width: 15px; height: 1.2px;
  background: #fff; border-radius: 1px;
  transition: transform 0.3s cubic-bezier(0.22,1,0.36,1), opacity 0.3s;
  transform-origin: center;
}
.hamburger-icon-m span:nth-child(1) { top: 0; }
.hamburger-icon-m span:nth-child(2) { top: 4px; }
.hamburger-icon-m span:nth-child(3) { top: 8px; }
.menu-pill-m.open .hamburger-icon-m span:nth-child(1) { transform: translateY(4px) rotate(45deg); }
.menu-pill-m.open .hamburger-icon-m span:nth-child(2) { opacity: 0; transform: scale(0); }
.menu-pill-m.open .hamburger-icon-m span:nth-child(3) { transform: translateY(-4px) rotate(-45deg); }

.menu-links-m {
  display: flex; align-items: center; gap: 14px;
  margin-left: auto; padding-right: 14px;
  opacity: 0; transform: translateX(10px);
  transition: opacity 0.15s, transform 0.15s;
  pointer-events: none; flex-shrink: 0; white-space: nowrap;
}
.menu-pill-m.open .menu-links-m { opacity: 1; transform: translateX(0); pointer-events: auto; }
.menu-links-m span { font-size: 13px; font-weight: 400; color: rgba(255,255,255,0.85); cursor: pointer; transition: color 0.15s; line-height: 1; }
.menu-links-m span:hover { color: #fff; }

.download-btn-m {
  height: 36px; padding: 0 14px;
  background: #fff; border-radius: 9999px;
  display: flex; align-items: center; gap: 6px;
  color: #000; flex-shrink: 0;
  transition: background 0.15s, transform 0.15s;
}
.download-btn-m:hover { background: #e2e2e6; transform: scale(1.03); }
.download-btn-m:active { transform: scale(0.97); }
.download-btn-m i { font-size: 13px; color: #000; transform: translateY(-0.5px); }
.download-btn-m span { font-size: 13px; font-weight: 500; color: #000; line-height: 1; }

/* Scramble text helper */
.scramble-line { display: inline-block; }
</style>
</head>
<body>

<!-- LAYER 0: Background Video -->
<div id="video-layer">
  <video id="bg-video" loop muted playsinline preload="auto"
    src="https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260613_120544_a609e0c2-e52d-4bd5-b10f-b66ac51f1965.mp4">
  </video>
</div>

<!-- LAYER 1: Bottom Blur -->
<div id="bottom-blur"></div>

<!-- HEADER -->
<header id="main-header">
  <!-- Desktop -->
  <div class="desktop-header" style="display:flex;align-items:center;justify-content:space-between;width:100%;height:100%;">
    <div class="header-left">
      <div class="logo-pill" id="logo-pill" onclick="window.scrollTo({top:0,behavior:'smooth'})">
        <svg viewBox="-50 -50 100 100"><g fill="currentColor"><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(90)"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(180)"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(270)"/></g></svg>
        <span>SynapseX</span>
      </div>
      <div class="menu-pill" id="menu-pill">
        <button class="hamburger-btn" id="hamburger-btn" aria-label="Toggle Menu">
          <div class="hamburger-icon"><span></span><span></span><span></span></div>
        </button>
        <div class="menu-links">
          <span onclick="window.scrollTo({top:window.innerHeight,behavior:'smooth'});document.getElementById('menu-pill').classList.remove('open')">About</span>
          <span onclick="window.scrollTo({top:window.innerHeight*2,behavior:'smooth'});document.getElementById('menu-pill').classList.remove('open')">Metrics</span>
        </div>
      </div>
    </div>
    <a class="download-btn" href="https://www.instagram.com/dmitriyinin" target="_blank" rel="noopener noreferrer">
      <i class="bi bi-apple"></i>
      <span>Download</span>
    </a>
  </div>

  <!-- Mobile -->
  <div class="mobile-header">
    <div class="mobile-left">
      <div class="logo-pill-m" id="logo-pill-m" onclick="window.scrollTo({top:0,behavior:'smooth'})">
        <svg viewBox="-50 -50 100 100"><g fill="currentColor"><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(90)"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(180)"/><path d="M 1.5,23 L 1.5,33 C 1.5,38.5 6,43 11.5,43 L 16.5,43 C 22,43 26.5,38.5 26.5,33 Q 28,28 33,26.5 C 38.5,26.5 43,22 43,16.5 L 43,11.5 C 43,6 38.5,1.5 33,1.5 L 23,1.5 Q 12,12 1.5,23 Z" transform="rotate(270)"/></g></svg>
        <span>SynapseX</span>
      </div>
      <div class="menu-pill-m" id="menu-pill-m">
        <button class="hamburger-btn-m" id="hamburger-btn-m" aria-label="Toggle Menu Mobile">
          <div class="hamburger-icon-m"><span></span><span></span><span></span></div>
        </button>
        <div class="menu-links-m">
          <span onclick="window.scrollTo({top:window.innerHeight,behavior:'smooth'});closeMobileMenu()">About</span>
          <span onclick="window.scrollTo({top:window.innerHeight*2,behavior:'smooth'});closeMobileMenu()">Metrics</span>
        </div>
      </div>
    </div>
    <a class="download-btn-m" href="https://www.instagram.com/dmitriyinin" target="_blank" rel="noopener noreferrer">
      <i class="bi bi-apple"></i>
      <span>Download</span>
    </a>
  </div>
</header>

<!-- MAIN CONTENT -->
<main id="main-content">
  <div class="dot-grid"></div>

  <!-- SECTION 1: Hero -->
  <div id="hero-section">
    <div style="width:100%;flex:1;display:flex;flex-direction:column;justify-content:space-between;gap:48px;">
      <div class="hero-grid">
        <div style="text-align:left;">
          <div class="hero-title">
            <span class="scramble-line" data-scramble-in data-text="Brain" data-delay="100">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</span>
            <span class="scramble-line" data-scramble-in data-text="And Body" data-delay="300">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</span>
          </div>
        </div>
        <div></div>
      </div>

      <div class="hero-grid-bottom">
        <div class="hero-desc" id="hero-desc">
          <p>Built at the intersection of neuroscience and artificial intelligence. SynapseX continuously maps neural pathways, cognitive load, and physiological states into a single adaptive intelligence layer.</p>
        </div>
        <div style="display:flex;flex-direction:column;align-items:flex-end;text-align:right;">
          <div class="hero-title right">
            <span class="scramble-line" data-scramble-in data-text="One" data-delay="200">&nbsp;&nbsp;&nbsp;</span>
            <span class="scramble-line" data-scramble-in data-text="Network" data-delay="400">&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- SECTION 1.5: Cinematic Parallax Paragraph -->
  <div id="cinematic-section">
    <div id="cinematic-inner">
      <h2>A neural-AI interface built on the architecture of the human nervous system. SynapseX translates synaptic activity into computational intelligence. Every signal becomes measurable, structured, and visible. It continuously reconstructs internal state as a dynamic neural map. Biological noise is filtered into actionable cognitive patterns.</h2>
    </div>
  </div>

  <!-- SECTION 2: Stats Carousel -->
  <div id="stats-section">
    <div class="swiper" id="stats-swiper">
      <div class="swiper-wrapper" id="swiper-wrapper"></div>
    </div>
  </div>
</main>

<script src="https://cdn.jsdelivr.net/npm/swiper@11/swiper-bundle.min.js"></script>
<script>
(function() {
  "use strict";

  // ── Constants ──
  const GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+~|}{[]:;?><";
  const VIDEO_URL = "https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260613_120544_a609e0c2-e52d-4bd5-b10f-b66ac51f1965.mp4";

  const statsData = [
    { title: "NEURAL ACTIVITY", value: "7.2M", footer: "LIVE SIGNALS INTERPRETED", details: ["Continuous temporal synapsing","1024 parallel telemetry streams","Dynamic feed classification active"] },
    { title: "PREDICTIVE MODEL", value: "93%", footer: "FORECAST ACCURACY RATE", details: ["Reinforced gradient mapping","Low latency neural resolution","Adaptive signal feedback system"] },
    { title: "EPOCH LATENCY", value: "0.4ms", footer: "CYCLE RESPONSE SPEED", details: ["Hardware accelerated pipeline","Direct metal shader execution","Temporal synchronization loop"] },
    { title: "COGNITIVE STREAMS", value: "14.8M", footer: "REAL-TIME MODEL COHERENCE", details: ["Distributed synapse projection","High-fidelity entropy filtering","Sub-millisecond state coherence"] },
    { title: "SYNAPSE DEPTH", value: "128L", footer: "MODEL RESOLUTION DEPTH", details: ["Deep feed-forward mapping","Transformer-based neural routing","Multi-dimensional pattern projection"] },
    { title: "SIGNAL INTEGRITY", value: "99.9%", footer: "NOISE REDUCTION RATIO", details: ["Advanced wave-let filtering","Dynamic heuristic balancing","Contextual signal amplification"] }
  ];

  // ── State ──
  let scrollProgress = 0;
  let smoothScrollProgress = 0;
  let entrancePhase = "loading"; // loading | animating | complete
  let entranceStart = 0;
  let videoReady = false;

  // ── Elements ──
  const video = document.getElementById("bg-video");
  const header = document.getElementById("main-header");
  const mainContent = document.getElementById("main-content");
  const heroSection = document.getElementById("hero-section");
  const heroDesc = document.getElementById("hero-desc");
  const cinematicInner = document.getElementById("cinematic-inner");
  const statsSection = document.getElementById("stats-section");

  // ── Build Stats Cards ──
  const wrapper = document.getElementById("swiper-wrapper");
  statsData.forEach(card => {
    const slide = document.createElement("div");
    slide.className = "swiper-slide";
    slide.innerHTML = `
      <div class="stat-card-outer">
        <div class="stat-card-inner">
          <div>
            <div style="display:flex;align-items:center;justify-content:space-between;">
              <span class="stat-title">${card.title}</span>
            </div>
            <div class="stat-value">${card.value}</div>
          </div>
          <div class="stat-details">
            ${card.details.map(d => `<div class="stat-detail"><span class="dot"></span><span>${d}</span></div>`).join("")}
          </div>
        </div>
        <div class="stat-footer">${card.footer}</div>
      </div>`;
    wrapper.appendChild(slide);
  });

  // ── Swiper Init ──
  new Swiper("#stats-swiper", {
    effect: "coverflow",
    grabCursor: true,
    slidesPerView: "auto",
    centeredSlides: true,
    loop: true,
    spaceBetween: 32,
    coverflowEffect: { rotate: 30, stretch: 0, depth: 100, modifier: 1, slideShadows: false },
    observer: true,
    observeParents: true
  });

  // ── Hamburger Menus ──
  document.getElementById("hamburger-btn").addEventListener("click", () => {
    document.getElementById("menu-pill").classList.toggle("open");
  });
  document.getElementById("hamburger-btn-m").addEventListener("click", () => {
    const pill = document.getElementById("menu-pill-m");
    const logo = document.getElementById("logo-pill-m");
    pill.classList.toggle("open");
    logo.classList.toggle("collapsed", pill.classList.contains("open"));
  });
  window.closeMobileMenu = function() {
    document.getElementById("menu-pill-m").classList.remove("open");
    document.getElementById("logo-pill-m").classList.remove("collapsed");
  };

  // ── Scroll Tracking ──
  function updateScrollProgress() {
    const scrollTop = window.scrollY || document.documentElement.scrollTop;
    const scrollHeight = document.documentElement.scrollHeight - document.documentElement.clientHeight;
    if (scrollHeight <= 0) return;
    scrollProgress = scrollTop / scrollHeight;
  }
  window.addEventListener("scroll", updateScrollProgress, { passive: true });
  updateScrollProgress();

  // ── Lenis (Desktop only) ──
  const isMobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent) || window.innerWidth < 768;

  if (!isMobile) {
    const lenisScript = document.createElement("script");
    lenisScript.src = "https://unpkg.com/lenis@1.1.18/dist/lenis.min.js";
    lenisScript.onload = function() {
      const lenis = new Lenis({
        duration: 1.2,
        easing: function(t) { return Math.min(1, 1.001 - Math.pow(2, -10 * t)); },
        smoothWheel: true,
        wheelMultiplier: 1.0,
        touchMultiplier: 1.5
      });
      lenis.on("scroll", updateScrollProgress);
      function raf(time) { lenis.raf(time); requestAnimationFrame(raf); }
      requestAnimationFrame(raf);
    };
    document.head.appendChild(lenisScript);
  }

  // ── ScrambleIn System ──
  const scrambleEls = document.querySelectorAll("[data-scramble-in]");
  const scrambleStates = [];

  scrambleEls.forEach(el => {
    const text = el.getAttribute("data-text");
    const delay = parseInt(el.getAttribute("data-delay") || "0", 10);
    scrambleStates.push({
      el, text, delay,
      phase: "idle", // idle | scrambling-in | revealed | scrambling-out | hidden
      progress: 0, lastTime: 0, started: false
    });
  });

  function updateScrambles(now) {
    const scrollActive = scrollProgress > 0.015;

    scrambleStates.forEach(s => {
      if (!videoReady && s.phase === "idle") return;

      if (videoReady && s.phase === "idle" && !scrollActive && !s.started) {
        s.started = true;
        setTimeout(() => {
          s.phase = "scrambling-in";
          s.progress = 0;
          s.lastTime = now;
        }, s.delay);
        return;
      }

      if (scrollActive && (s.phase === "revealed" || s.phase === "scrambling-in")) {
        s.phase = "scrambling-out";
        s.progress = 0;
        s.lastTime = now;
      } else if (!scrollActive && (s.phase === "hidden" || s.phase === "scrambling-out")) {
        s.phase = "scrambling-in";
        s.progress = 0;
        s.lastTime = now;
      }

      if (s.phase === "scrambling-in") {
        const duration = 900;
        s.progress = Math.min(1, s.progress + (now - s.lastTime) / duration);
        s.lastTime = now;
        const t = s.progress;

        let result = "";
        for (let i = 0; i < s.text.length; i++) {
          if (s.text[i] === " ") { result += " "; continue; }
          const threshold = i / s.text.length;
          if (t >= threshold + 0.15) result += s.text[i];
          else if (t >= threshold - 0.1) result += GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
          else result += "\u00A0";
        }
        s.el.textContent = result;
        s.el.style.opacity = "1";

        if (t >= 1) { s.phase = "revealed"; s.el.textContent = s.text; }
      } else if (s.phase === "scrambling-out") {
        const duration = 700;
        s.progress = Math.min(1, s.progress + (now - s.lastTime) / duration);
        s.lastTime = now;
        const t = s.progress;

        let result = "";
        for (let i = 0; i < s.text.length; i++) {
          if (s.text[i] === " ") { result += " "; continue; }
          const threshold = i / s.text.length;
          if (t >= threshold + 0.2) result += "\u00A0";
          else if (t >= threshold - 0.05) result += GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
          else result += s.text[i];
        }
        s.el.textContent = result;
        s.el.style.opacity = String(Math.max(0, 1 - t * 1.5));

        if (t >= 1) {
          s.phase = "hidden";
          s.el.textContent = s.text.replace(/\S/g, "\u00A0");
          s.el.style.opacity = "0";
        }
      }
    });
  }

  // ── Stats Reveal on Scroll ──
  let statsRevealed = false;
  function checkStatsReveal() {
    if (statsRevealed) return;
    const rect = statsSection.getBoundingClientRect();
    if (rect.top < window.innerHeight * 0.9) {
      statsRevealed = true;
      statsSection.classList.add("revealed");
    }
  }
  window.addEventListener("scroll", checkStatsReveal, { passive: true });

  // ── Main Animation Loop ──
  let isSeeking = false;
  let nextSeekTime = null;

  video.addEventListener("seeking", () => { isSeeking = true; });
  video.addEventListener("seeked", () => {
    isSeeking = false;
    if (nextSeekTime !== null) {
      const t = nextSeekTime; nextSeekTime = null;
      if (video.readyState >= 1 && video.duration > 0) { isSeeking = true; video.currentTime = t; }
    }
  });
  video.addEventListener("loadedmetadata", () => { video.autoplay = false; video.pause(); });
  video.autoplay = false;
  video.pause();

  // Safety timeout
  setTimeout(() => {
    if (entrancePhase === "loading") {
      entrancePhase = "animating";
      entranceStart = performance.now();
    }
  }, 3500);

  function tick(now) {
    // ── Smooth scroll interpolation ──
    smoothScrollProgress += (scrollProgress - smoothScrollProgress) * 0.12;
    if (Math.abs(scrollProgress - smoothScrollProgress) < 0.0001) smoothScrollProgress = scrollProgress;

    // ── Video blur + scale ──
    const subtleBase = Math.max(0, Math.min(1, (smoothScrollProgress - 0.1) / 0.45));
    const progressive = Math.max(0, Math.min(1, (smoothScrollProgress - 0.55) / 0.4));
    const blurVal = subtleBase * 5 + progressive * 50;
    const scaleVal = 1.03 + Math.max(0, Math.min(1, (smoothScrollProgress - 0.1) / 0.9)) * 0.08;

    // ── Video entrance ──
    let entranceZoom = 1.0;
    let entranceOpacity = 1.0;

    if (entrancePhase === "loading") {
      entranceZoom = 1.12;
      entranceOpacity = 0;
      if (video.readyState >= 3) {
        entrancePhase = "animating";
        entranceStart = performance.now();
      }
    }

    if (entrancePhase === "animating") {
      const elapsed = now - entranceStart;
      const progress = Math.min(1, elapsed / 1400);
      const easeOut = 1 - Math.pow(1 - progress, 3);
      entranceZoom = 1.12 - 0.12 * easeOut;
      entranceOpacity = Math.min(1.0, elapsed / 500);

      if (progress >= 1) {
        entrancePhase = "complete";
        videoReady = true;
        header.classList.add("visible");
        mainContent.classList.add("visible");
      }
    }

    if (entrancePhase === "complete" && !videoReady) {
      videoReady = true;
      header.classList.add("visible");
      mainContent.classList.add("visible");
    }

    // Apply video styles
    video.style.filter = `blur(${blurVal}px)`;
    video.style.transform = `scale(${scaleVal * entranceZoom})`;
    video.style.opacity = String(entranceOpacity);

    // ── Video seek ──
    if (video.readyState >= 1 && video.duration > 0) {
      const targetTime = Math.max(0, Math.min(video.duration, smoothScrollProgress * video.duration));
      if (Math.abs(video.currentTime - targetTime) > 0.008) {
        if (!isSeeking && !video.seeking) { isSeeking = true; video.currentTime = targetTime; }
        else { nextSeekTime = targetTime; }
      }
    }

    // ── Hero section parallax ──
    const scrollH = document.documentElement.scrollHeight - document.documentElement.clientHeight;
    const scrollYNorm = scrollH > 0 ? (window.scrollY / scrollH) : 0;

    // Hero fade
    const heroOp = Math.max(0, Math.min(1, 1 - scrollYNorm / 0.26));
    const heroSc = 1 - (1 - 0.96) * Math.min(1, scrollYNorm / 0.26);
    heroSection.style.opacity = String(heroOp);
    heroSection.style.transform = `scale(${heroSc})`;

    // Desc fade
    const descOp = Math.max(0, Math.min(1, 1 - scrollYNorm / 0.12));
    const descYval = -30 * Math.min(1, scrollYNorm / 0.12);
    heroDesc.style.opacity = String(descOp);
    heroDesc.style.transform = `translateY(${descYval}px)`;

    // ── Cinematic paragraph ──
    const scrollPx = window.scrollY;
    const yVal = -120 * Math.min(1, scrollPx / 1000);

    // Opacity keyframes: [0.08, 0.22, 0.42, 0.65] -> [0, 1, 1, 0]
    let cinOp = 0;
    if (scrollYNorm <= 0.08) cinOp = 0;
    else if (scrollYNorm <= 0.22) cinOp = (scrollYNorm - 0.08) / (0.22 - 0.08);
    else if (scrollYNorm <= 0.42) cinOp = 1;
    else if (scrollYNorm <= 0.65) cinOp = 1 - (scrollYNorm - 0.42) / (0.65 - 0.42);
    else cinOp = 0;

    cinematicInner.style.transform = `rotateX(24deg) translateY(${yVal}px) translateZ(15px)`;
    cinematicInner.style.opacity = String(Math.max(0, Math.min(1, cinOp)));

    // ── ScrambleIn updates ──
    updateScrambles(now);

    // ── Hero desc entrance ──
    if (videoReady && !heroDesc._entered) {
      heroDesc._entered = true;
      heroDesc.style.transition = "opacity 0.9s cubic-bezier(0.215,0.61,0.355,1) 0.2s, transform 0.9s cubic-bezier(0.215,0.61,0.355,1) 0.2s";
      heroDesc.style.opacity = "1";
      heroDesc.style.transform = "translateY(0)";
    }

    requestAnimationFrame(tick);
  }

  // Set initial desc state
  heroDesc.style.opacity = "0";
  heroDesc.style.transform = "translateY(25px)";

  requestAnimationFrame(tick);
  checkStatsReveal();
})();
</script>
</body>
</html>

## NexaCore — Landing Page [sites/nexacore-hero]

- Preview: https://motionsites.ai/assets/hero-nexacore-preview-DtWEu8_f.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nexacore-hero.gif

Build a React + TypeScript + Vite + Tailwind CSS landing page for "NexaCore" — an enterprise infrastructure operations platform. Use lucide-react for icons, hls.js for HLS video streaming, and @supabase/Bolt Database-js (available but not yet wired). No other UI libraries.

Global Setup
Fonts (src/index.css)
Load "Mazzard H" custom font via @font-face before Tailwind directives:

Weight 400: https://db.onlinewebfonts.com/t/eb5b5ee332420add9a40ee988cb6ac37.woff2 (+ .woff, .ttf fallbacks same hash)
Weight 500: https://db.onlinewebfonts.com/t/875fffdfa62169a0f131e90f37f1faf4.woff2 (+ .woff, .ttf fallbacks)
Apply globally:


@layer base {
  html, body, * { font-family: 'Mazzard H', sans-serif; }
}
App Structure (src/App.tsx)

<main>
  <Navbar />
  <Hero />
  <TrustedSection />
  <FreedomSection />
  <PrecisionSection />
</main>
Brand Colors (use consistently)
Deep navy text: rgb(26, 11, 84)
Muted lavender text: rgb(169, 151, 206) / rgb(189, 174, 231) / rgb(131, 121, 158)
Accent purple: rgb(200, 111, 255) (also #c86fff)
Primary solid blue: rgb(28, 78, 255)
Gradient A (logos/buttons): linear-gradient(90deg, rgb(28,78,255), rgb(172,36,255) 50%, rgb(254,136,27))
Gradient B (headline highlights): linear-gradient(90deg, rgb(43,167,255), rgb(202,69,255) 50%, rgb(254,136,27))
Off-white chip bg: rgb(249, 249, 249)
Dark card bg: rgba(10, 5, 20, 0.88) with backdrop-filter: blur(36px)
Never use purple/indigo outside these exact gradient stops. Heading font-weight is always 500, body 400.

1. Navbar (src/components/Navbar.tsx)
Fixed floating pill-shaped navbar.

Container: fixed top-4 left-0 right-0 z-50 flex justify-center pl-4 pr-1.5
Inner <nav>: white background, rounded-2xl, shadow-lg, width transitions from max-w-6xl → max-w-3xl on scroll > 20px (transition-all duration-500 ease-in-out).
Inner row: flex items-center justify-between gap-6, padding pl-5 pr-2 py-1.5 → pl-4 pr-2 py-1.5 on scroll.
Logo: 28x28 SVG circle stroked with logoGradient linear gradient (stops: rgb(28,78,255) → rgb(172,36,255) → rgb(254,136,27)), strokeWidth 2.5. Text "NexaCore" next to it, size 22, letter-spacing -0.02em, color rgb(26,11,84), font-weight 500.
Nav links (desktop, hidden on mobile): "What We Build", "Our Method", "Who We Are", "Thinking". Links: text-sm, rounded-xl, hover:bg-gray-100, padding shrinks on scroll (px-4 py-2 → px-2 py-1.5), gap from 1 to 0.
Right cluster: Search icon button (lucide Search size 20) in a 40x40 rounded-xl hover:bg-gray-100, plus <ContactButton />.
Mobile: Menu/X toggle (lucide) reveals stacked nav links centered + search icon + ContactButton (with flex-1).
ContactButton (src/components/ContactButton.tsx)
Gradient-border pill button.

Outer <a>: relative inline-flex items-center justify-center rounded-xl p-px, background = Gradient A.
Inner <span>: rounded-[11px] px-7 py-3 text-base text-white, background = solid rgb(28,78,255) by default, switches to Gradient A on hover with transition-colors duration-300. Text: "Contact".
2. Hero (src/components/Hero.tsx)
Full-screen looping background video with centered text.

Section: relative min-h-screen flex items-center justify-center px-4 overflow-hidden
<video> with exact src: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260418_115655_b4d9cd77-feed-43cd-a198-af78ebdf1f7a.mp4 Attributes: autoPlay muted loop playsInline, absolute inset-0 w-full h-full object-cover.
Bottom fade overlay: absolute bottom-0 left-0 right-0 h-48 z-10 pointer-events-none, background linear-gradient(to bottom, transparent, #000201).
Content (z-10, max-w-2xl, centered, gap 6):
Eyebrow <span> text "Infrastructure Built to Last" — gradient text using Gradient B (background-clip: text; -webkit-text-fill-color: transparent), text-lg font-medium.
H1: "Engineer and scale with clarity." — white, font-medium leading-tight md:whitespace-nowrap, font-size: clamp(32px, 4vw, 56px).
Paragraph, color rgb(169, 151, 206), clamp(15px, 1.2vw, 20px): "NexaCore helps infrastructure owner, operator and supplier teams enforce global build standards for mission-critical systems. Align teams, regions and programs without the heavy lifting."
3. TrustedSection (src/components/TrustedSection.tsx)
Dark background-image section with four ServiceCards.

Background image (exact): https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260418_120332_3b24257a-afe6-48ca-875f-78147370f403.png&w=1280&q=85 background-size: cover; background-position: center.
Section padding: clamp(100px, 12vw, 180px) clamp(16px, 4vw, 40px) clamp(100px, 12vw, 160px), gap: 110.
Header block max-width 1200, gap 20, centered:
H2 white font-medium, clamp(32px, 4vw, 56px), line-height 1.2: "Relied on by enterprise teams" <br/> then a span with Gradient B text: "from groundbreak to go-live."
Paragraph color rgb(189, 174, 231), clamp(14px, 1.25vw, 18px): "Built for operational clarity through constant change. Proven across 530+ MW of critical infrastructure."
Grid: grid-cols-1 sm:grid-cols-2 lg:grid-cols-4, gap 12.
Bottom fade: absolute 180px tall linear-gradient(to bottom, transparent, rgb(255,255,255)).
The 4 cards (label, title, bullets, icon):
All icons are 16x16 SVGs with a base circle path + a progressively larger filled <circle> (radius 2, 3, 4) placed mirrored via transform: matrix(-1 0 0 1 x y), all filled rgb(200,111,255). Card 1 uses only the base circle path.

Planning — "Turn new programs
into structured plans without the noise." Bullets: "Embedded program leads", "Decision-ready roadmaps".
Procurement — "Source and qualify
vendors with far
less friction." Bullets: "Cross-org scope alignment", "End-to-end accountability".
Logistics — "Move the right
materials on time
without surprises." Bullets: "Spec and fit validations", "Change order ownership".
Commissioning — "Activate systems with complete context, not guesswork." Bullets: "Uninterrupted workflows", "Verified clean handoffs".
ServiceCard (src/components/ServiceCard.tsx)
Props: label, icon, title, bullets[]. useState for hover.

Container: relative flex flex-col overflow-hidden rounded-[36px] cursor-pointer, bg rgba(10,5,20,0.88), backdrop-filter: blur(36px), height clamp(320px, 32vw, 500px).
Top image layer: absolute top, height 55%, z-index 1. Image src: https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/682c7cb62b8800a7594c5abd_hover_card_img.png, object-fit: cover; object-position: top. Default translateY(-30%) opacity 0.7; on hover translateY(0) opacity 1, transition-all duration-500.
Bottom overlay: absolute bottom, height 55%, linear-gradient(to top, rgba(10,5,20,0.95) 60%, transparent). Default hidden (translateY(100%) opacity 0), slides up on hover.
Content (z-2, padding clamp(16px, 1.94vw, 32px) clamp(18px, 2.36vw, 36px)):
Badge: rgb(41,31,57) bg, rounded-full, padding clamp(6px,0.7vw,12px) clamp(10px,1.25vw,20px), white text + icon (icon sized 1.11vw / min 14px, height 17). Label text inside.
Flex-grow spacer.
Title: white font-medium, clamp(16px, 1.7vw, 24px), leading-snug. Shifts up by -8px on hover.
Bullets <ul> gap 10. Each <li> color rgb(189,174,231), clamp(12px,1vw,15px), padding-left clamp(22px,1.8vw,28px), background-image the bullet SVG at https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/683ef70a24657b10be91ef49_bullet-list.svg, size 18px, position 0% 50%.
"Learn more" button: hidden by default (max-height 0, opacity 0, translateY 20px), on hover (max-height 80, opacity 1, translateY 0). Background = Gradient A, white text, rounded-xl, padding clamp(10px,0.9vw,14px) 0, font clamp(13px,1.1vw,16px).
4. FreedomSection (src/components/FreedomSection.tsx)
White section. 3-column grid (negatives | HLS circular video | positives).

Section: bg #ffffff, padding clamp(48px,6vw,80px) clamp(16px,3vw,40px), gap 36.
Header: centered gap-9.
Chip: bg rgb(249,249,249), rounded-full, padding 0.9vw 1.25vw, color rgb(26,11,84), text-lg font-medium. Inline SVG (19x18, viewBox 0 0 17 16) of two-heart/cloud shape filled rgb(200,111,255) (exact path in code). Text: "Control".
H2 font-medium clamp(32px,4vw,56px) color rgb(26,11,84) line-height 1.15: "Stop absorbing the chaos." <br/> gradient span (Gradient B with paddingBottom: 0.3vw; display: inline-block): "Run with confidence."
Grid: flex flex-col lg:grid, grid-template-columns: 26vw 1fr 26vw, column gap 36, row gap 24, align-items start, padding 0 clamp(0px,2.92vw,40px).
Left column — negatives
Font clamp(13px,1.15vw,17px), color rgb(131,121,158), gap 12. Each row: white bg, rounded 18, padding clamp(12px,0.97vw,16px) clamp(14px,1.25vw,20px), box-shadow 0 3px 9.1px #3f4a7e0d, 0 1px 29px #3f4a7e1a. Icon (cross) src: https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/686cc0f520a992816d8b15dc_bullet-list-cross.svg width clamp(16px,1.25vw,20px).
Texts (in order):

"Reactive firefighting when foundational issues surface too late"
"Bloated coordination overhead drains bandwidth from core teams"
"Constant re-verification because source data can't be trusted"
"Fragmented vendor relations produce mismatched deliverables"
"Scattered specs and decisions buried across siloed systems"
Center column — circular HLS video
Wrapper: borderRadius: 50%; overflow: hidden; width/height: clamp(200px,22vw,400px).
<HlsVideo /> component: uses hls.js. On mount, create new Hls({ startLevel: -1, capLevelToPlayerSize: false, maxMaxBufferLength: 60, enableWorker: true }), load https://stream.mux.com/bnYL6x5cAX6WiJv2pOKpITehZd3NVdXpj3ylJFpX5Lk.m3u8, attachMedia, on MANIFEST_PARSED set hls.currentLevel = hls.levels.length - 1 and play. Native Safari HLS fallback via canPlayType('application/vnd.apple.mpegurl').
Video style: width:160%; height:160%; object-fit:cover; absolute top:50% left:50%; transform: translate(-50%,-50%). Attrs autoPlay loop muted playsInline.
Right column — positives
Same card styling as negatives, but icon (check) src: https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/686cc068490683bbb3377d04_bullet-list.svg. Inner text color rgb(26,11,84).
Texts:

"Layered dependency maps eliminate costly surprises at every phase"
"Streamlined team handoffs deliver production-ready outcomes fast"
"Live validation loops keep requirements locked across all stages"
"Unified vendor management through a single accountable contact"
"Centralized context and clear records accelerate every decision"
Center column has class order-first lg:order-none so it appears first on mobile.

5. PrecisionSection (src/components/PrecisionSection.tsx)
Light background-image section with a 4-pillar "staircase".

Background image (exact):
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260418_125638_553b96dc-a1fd-4b2b-81a9-ed7daa80006e.png&w=1280&q=85
cover / center / no-repeat. Padding clamp(48px,8vw,120px) clamp(16px,4vw,60px) clamp(48px,5.56vw,80px), gap clamp(32px,4vw,56px), centered column, text-align center.

Header:

Chip: bg rgb(249,249,249), radius 36, padding clamp(8px,0.9vw,14px) clamp(12px,1.25vw,20px), font clamp(14px,1.1vw,18px) weight 500, color rgb(26,11,84). Inline SVG (19x18, viewBox 0 0 17 16): circle stroke #c86fff at (8.5, 8) r=7, plus four tick marks at top/bottom/left/right, all filled rgb(200,111,255). Text: "Structured Delivery".
H2 max-width clamp(700px,60vw,900px), gap 22 from chip.
Block 1 (display:block, sm:whitespace-nowrap): "One integrated, end-to-end system."
Block 2 (display:block, Gradient B text, paddingBottom: 0.3vw): "Compounding operational value."
Style: clamp(28px,4vw,56px), weight 500, rgb(26,11,84), line-height 1.15.
Paragraph: clamp(15px,1.2vw,20px), rgb(169,151,206): "NexaCore teams capture, align, validate and deliver exactly what keeps your programs on track."
Desktop staircase (hidden on sm and below)
Wrapper max-width: 82.292vw. Relative block width: 82.292vw; height: 31.94vw, text color rgb(26,11,84).

Four pillars, each absolutely positioned via left: Xvw; bottom: Yvw:

Label	left	bottom	Items
Scopes	2.8vw	7vw	conditions, capacity, specs, timelines
Integrates	22.4vw	9.08vw	civil, mechanical, electrical, controls
Certifies	41.2vw	11.16vw	redundancy, testing, compliance, sign-offs
Activates	61.1vw	13.24vw	cutover, runbooks, handoff, SLAs
Each pillar = column (align center):

Chip: linear-gradient(135deg, rgb(255,255,255), rgba(255,255,255,0.6)), fontSize 18, weight 500, radius 20, padding 0.972vw 1.736vw, gap 8. Inside: logo image src https://cdn.prod.website-files.com/6720dd1ab6df0da205830ab1/6870f623cf3df417ce45df05_icon%20logo%20eternacloud.png width 1.111vw + label text.
Vertical line: 1px wide, height: 14.24vw, background = linear-gradient(rgb(28,78,255), rgb(254,136,27) 0%, rgb(172,36,255) 25%, rgb(247,159,255) 50%, rgb(255,214,0) 66%, rgb(254,136,27) 84%, rgba(254,136,27,0) 102%).
Items absolutely positioned top: 0.56vw; left: 1.94vw (right of the line), gap 4, fontSize 16. Each item padding 0.69vw 1.04vw.
Mobile layout (sm:hidden)
Same 4 pillars in a vertical column, alternating alignment: index 0,2 left-aligned; 1,3 right-aligned. Chip (smaller: fontSize 15, padding 10px 18px, logo 16px). Below chip a row flex-direction: row (or row-reverse when right): 1px gradient line (min-height 120) adjacent to items. Items fontSize 14, color rgb(100,80,160), padding 8px 0.

Build config
Vite + React 18.3 + TS 5.5. Tailwind 3.4, PostCSS, Autoprefixer.
package.json deps: @supabase/Bolt Database-js, hls.js, lucide-react, react, react-dom.
Scripts: dev, build (vite build), preview, lint, typecheck.

## Nike Premium Landing — Landing Page [sites/nike-premium-landing]

- Preview: https://motionsites.ai/assets/hero-nike-premium-landing-preview-_VyIBlIe.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nike-premium-landing.gif

Create a high-end, interactive Nike hero landing page with two scrolling sections. The app requires `react-player` and `gsap` for animations and interactive masks.

Follow these strict requirements to perfectly match the design, assets, fonts, and logic:

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
3. Bubble Menu Component (src/components/BubbleMenu.css)
code
CSS
.bubble-menu { display: flex; align-items: center; gap: 12px; z-index: 50; }
.bubble-menu.absolute { position: absolute; }
.bubble-menu.fixed { position: fixed; }
.bubble { width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; border: 1px solid rgba(255, 255, 255, 0.1); backdrop-filter: blur(8px); transition: transform 0.2s ease, background 0.3s ease; }
.bubble:hover { transform: scale(1.05); }
.menu-btn { flex-direction: column; gap: 6px; }
.menu-line { width: 20px; height: 2px; border-radius: 2px; transition: all 0.3s ease; }
.menu-line.short { width: 14px; }
.menu-btn:hover .menu-line.short { width: 20px; }
.menu-btn.open .menu-line:not(.short) { transform: translateY(4px) rotate(45deg); }
.menu-btn.open .menu-line.short { transform: translateY(-4px) rotate(-45deg); width: 20px; }
.bubble-menu-items { inset: 0; position: fixed; display: none; align-items: center; justify-content: center; z-index: 40; background: rgba(0, 0, 0, 0.7); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); }
.pill-list { list-style: none; padding: 0; margin: 0; display: flex; gap: 16px; flex-wrap: wrap; justify-content: center; max-width: 800px; }
.pill-link { display: block; padding: 16px 36px; border-radius: 9999px; background-color: var(--pill-bg); color: var(--pill-color); text-decoration: none; font-weight: 500; font-size: 24px; transform: rotate(var(--item-rot)); transition: all 0.3s ease; border: 1px solid rgba(255, 255, 255, 0.1); }
.pill-link:hover { background-color: var(--hover-bg) !important; color: var(--hover-color) !important; transform: scale(1.05) rotate(0deg); }
.pill-label { display: block; }
4. Bubble Menu Logic (src/components/BubbleMenu.tsx)
Create a GSAP-animated pill-menu component.
code
Tsx
import { useState, useRef, useEffect, ReactNode } from 'react';
import { gsap } from 'gsap';
import './BubbleMenu.css';

interface MenuItem { label: string; href: string; ariaLabel?: string; rotation?: number; hoverStyles?: { bgColor: string; textColor: string }; }
interface BubbleMenuProps { logo?: string | ReactNode; onMenuClick?: (isOpen: boolean) => void; className?: string; style?: React.CSSProperties; menuAriaLabel?: string; menuBg?: string; menuContentColor?: string; useFixedPosition?: boolean; items?: MenuItem[]; animationEase?: string; animationDuration?: number; staggerDelay?: number; }

export default function BubbleMenu({ logo, onMenuClick, className, style, menuAriaLabel = 'Toggle menu', menuBg = '#fff', menuContentColor = '#111', useFixedPosition = false, items, animationEase = 'back.out(1.5)', animationDuration = 0.5, staggerDelay = 0.12 }: BubbleMenuProps) {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [showOverlay, setShowOverlay] = useState(false);
  const overlayRef = useRef<HTMLDivElement>(null);
  const bubblesRef = useRef<(HTMLAnchorElement | null)[]>([]);
  const labelRefs = useRef<(HTMLSpanElement | null)[]>([]);

  const containerClassName = ['bubble-menu', useFixedPosition ? 'fixed' : 'absolute', className].filter(Boolean).join(' ');

  const handleToggle = () => {
    const nextState = !isMenuOpen;
    if (nextState) setShowOverlay(true);
    setIsMenuOpen(nextState);
    onMenuClick?.(nextState);
  };

  useEffect(() => {
    const overlay = overlayRef.current;
    const bubbles = bubblesRef.current.filter(Boolean);
    const labels = labelRefs.current.filter(Boolean);
    if (!overlay || !bubbles.length) return;

    if (isMenuOpen) {
      gsap.set(overlay, { display: 'flex' });
      gsap.killTweensOf([...bubbles, ...labels]);
      gsap.set(bubbles, { scale: 0, transformOrigin: '50% 50%' });
      gsap.set(labels, { y: 24, autoAlpha: 0 });

      bubbles.forEach((bubble, i) => {
        const delay = i * staggerDelay + gsap.utils.random(-0.05, 0.05);
        const tl = gsap.timeline({ delay });
        tl.to(bubble, { scale: 1, duration: animationDuration, ease: animationEase });
        if (labels[i]) tl.to(labels[i], { y: 0, autoAlpha: 1, duration: animationDuration, ease: 'power3.out' }, `-=${animationDuration * 0.9}`);
      });
    } else if (showOverlay) {
      gsap.killTweensOf([...bubbles, ...labels]);
      gsap.to(labels, { y: 24, autoAlpha: 0, duration: 0.2, ease: 'power3.in' });
      gsap.to(bubbles, { scale: 0, duration: 0.2, ease: 'power3.in', onComplete: () => { gsap.set(overlay, { display: 'none' }); setShowOverlay(false); } });
    }
  }, [isMenuOpen, showOverlay, animationEase, animationDuration, staggerDelay]);

  return (
    <>
      <nav className={containerClassName} style={style} aria-label="Main navigation">
        <button type="button" className={`bubble toggle-bubble menu-btn ${isMenuOpen ? 'open' : ''}`} onClick={handleToggle} style={{ background: menuBg }}>
          <span className="menu-line" style={{ background: menuContentColor }} />
          <span className="menu-line short" style={{ background: menuContentColor }} />
        </button>
      </nav>
      {showOverlay && (
        <div ref={overlayRef} className="bubble-menu-items fixed">
          <ul className="pill-list">
            {items?.map((item, idx) => (
              <li key={idx}>
                <a href={item.href} className="pill-link" style={{ '--item-rot': `${item.rotation ?? 0}deg`, '--pill-bg': menuBg, '--pill-color': menuContentColor, '--hover-bg': item.hoverStyles?.bgColor, '--hover-color': item.hoverStyles?.textColor } as any} ref={el => { bubblesRef.current[idx] = el; }} onClick={handleToggle}>
                  <span className="pill-label" ref={el => { labelRefs.current[idx] = el; }}>{item.label}</span>
                </a>
              </li>
            ))}
          </ul>
        </div>
      )}
    </>
  );
}
5. Spotlight Reveal Interactive Video Mask (src/components/SpotlightReveal.tsx)
code
Tsx
import { useEffect, useRef } from 'react';
import ReactPlayer from 'react-player';

interface SpotlightRevealProps { imageSrc: string; videoSrc: string; isPlaying?: boolean; baseRadius?: number; }

export default function SpotlightReveal({ imageSrc, videoSrc, isPlaying = true, baseRadius = 420 }: SpotlightRevealProps) {
  const NUM_TRAILS = 6;
  const videoRef = useRef<HTMLVideoElement>(null);
  const pointsRef = useRef(Array.from({ length: NUM_TRAILS }, () => ({ x: -1000, y: -1000 })));

  useEffect(() => {
    if (videoRef.current) { isPlaying ? videoRef.current.play() : videoRef.current.pause(); }
  }, [isPlaying]);

  useEffect(() => {
    let targetX = window.innerWidth / 2, targetY = window.innerHeight / 2;
    const handleMouseMove = (e: MouseEvent) => { targetX = e.clientX; targetY = e.clientY; };
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
        if (circle) { circle.setAttribute('cx', points[i].x.toString()); circle.setAttribute('cy', points[i].y.toString()); }
      }
      animationFrameId = requestAnimationFrame(animate);
    };
    animate();
    return () => { window.removeEventListener('mousemove', handleMouseMove); cancelAnimationFrame(animationFrameId); };
  }, []);

  return (
    <div className="absolute inset-0 w-full h-full z-0 bg-black pointer-events-none overflow-hidden flex items-center justify-center">
      <div className="absolute inset-0 w-full h-full flex items-center justify-center overflow-hidden pointer-events-none">
        <video ref={videoRef} src={videoSrc} className="absolute inset-0 w-full h-full object-cover" muted loop playsInline />
      </div>
      <svg className="absolute inset-0 w-full h-full" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <radialGradient id="holeGradient">
            <stop offset="0%" stopColor="black" stopOpacity="1" />
            <stop offset="60%" stopColor="black" stopOpacity="0.8" />
            <stop offset="100%" stopColor="black" stopOpacity="0" />
          </radialGradient>
          <mask id="spotlight-mask" maskContentUnits="userSpaceOnUse" x="0" y="0" width="100%" height="100%">
            <rect width="100%" height="100%" fill="white" />
            {Array.from({ length: NUM_TRAILS }).reverse().map((_, reversedIndex) => {
              const i = NUM_TRAILS - 1 - reversedIndex;
              return <circle key={`trail-${i}`} id={`trail-${i}`} cx="-1000" cy="-1000" r={baseRadius - i * 35} fill="url(#holeGradient)" opacity={1 - i * 0.15} />;
            })}
          </mask>
        </defs>
        <image href={imageSrc} width="100%" height="100%" preserveAspectRatio="xMidYMid slice" mask="url(#spotlight-mask)" />
      </svg>
    </div>
  );
}
6. App Layout & Data (src/App.tsx)
code
Tsx
import { useState } from 'react';
import BubbleMenu from './components/BubbleMenu';
import SpotlightReveal from './components/SpotlightReveal';

const items = [
  { label: 'Drops', href: '#', rotation: -8, hoverStyles: { bgColor: '#ef4444', textColor: '#ffffff' } },
  { label: 'Innovation', href: '#', rotation: 8, hoverStyles: { bgColor: '#3b82f6', textColor: '#ffffff' } },
  { label: 'Collections', href: '#', rotation: 8, hoverStyles: { bgColor: '#10b981', textColor: '#ffffff' } },
  { label: 'Community', href: '#', rotation: 8, hoverStyles: { bgColor: '#f59e0b', textColor: '#ffffff' } },
  { label: 'Stores', href: '#', rotation: -8, hoverStyles: { bgColor: '#8b5cf6', textColor: '#ffffff' } }
];

export default function App() {
  const [isFirstVideoPlaying, setIsFirstVideoPlaying] = useState(false);
  const [isSecondVideoPlaying, setIsSecondVideoPlaying] = useState(false);

  return (
    <div className="relative w-full flex flex-col bg-[#050505]">
      {/* First Screen */}
      <section className="sticky top-0 z-0 w-full h-[100dvh] overflow-hidden flex flex-col justify-between pointer-events-auto">
        <SpotlightReveal
          imageSrc="https://github.com/dsMagnatov/Acreage-landing-assets/blob/main/0098888.jpg?raw=true"
          videoSrc="https://pikaso.cdnpk.net/private/production/4021778466/80a7f7ef-643d-40bc-b533-1e86f159d653-0.mp4?token=exp=1777075200~hmac=91d86c3600a89e923130fce0912dcfb0de81f05f2cde5fc77c30f3e7ae094342"
          isPlaying={isFirstVideoPlaying}
        />

        <div className="absolute bottom-0 left-0 w-full h-[75%] z-20" onMouseEnter={() => setIsFirstVideoPlaying(true)} onMouseLeave={() => setIsFirstVideoPlaying(false)} />

        <header className="relative z-50 w-full flex justify-center items-start pt-[150px]">
          <svg width="120" viewBox="135.5 361.38 420.32 149.8" fill="white" xmlns="http://www.w3.org/2000/svg">
            <path d="m181.86 511.11c-12.524-0.49755-22.77-3.9244-30.782-10.289-1.529-1.2159-5.1725-4.8616-6.3949-6.3992-3.2489-4.0853-5.4578-8.0611-6.931-12.472-4.5334-13.579-2.2002-31.397 6.6737-50.953 7.5979-16.742 19.322-33.347 39.776-56.344 3.013-3.384 11.986-13.281 12.043-13.281 0.0216 0-0.46749 0.84706-1.083 1.8786-5.3183 8.9082-9.8689 19.401-12.348 28.485-3.9823 14.576-3.502 27.085 1.4068 36.784 3.3862 6.6822 9.1913 12.47 15.719 15.67 11.428 5.5993 28.159 6.0625 48.592 1.3554 1.4068-0.32599 71.116-18.831 154.91-41.123 83.794-22.294 152.36-40.52 152.37-40.505 0.0237 0.0193-194.68 83.333-295.75 126.56-16.007 6.8431-20.287 8.5715-27.812 11.214-19.236 6.7551-36.467 9.9783-50.396 9.4251z"/>
          </svg>
          <BubbleMenu items={items} className="absolute top-8 right-8 z-50" />
        </header>

        <main className="relative z-10 w-full flex-1 flex flex-col items-center justify-end pb-24 px-4 text-center text-white">
          <h1 className="font-sans font-medium leading-[1.05] tracking-tight w-full mx-auto translate-y-[50px]" style={{ fontSize: 'clamp(14px, 3vw, 51px)' }}>
            <span className="block">Pure Comfort For</span>
            <span className="block">Next-Generation Athletes. <span className="font-serif italic font-normal pr-1">We Craft</span></span>
            <span className="block font-serif italic font-normal pr-1">The Ultimate Footwear For Elite Performance,</span>
            <span className="block font-serif italic font-normal pr-1">Urban Exploration, Everyday Style.</span>
          </h1>
        </main>
      </section>

      {/* Second Screen */}
      <section className="relative z-10 w-full h-[100dvh] overflow-hidden bg-black text-white" style={{ boxShadow: '0 -20px 50px rgba(0,0,0,0.5)' }}>
        <SpotlightReveal
          imageSrc="https://github.com/dsMagnatov/Acreage-landing-assets/blob/main/02604201313.png?raw=true"
          videoSrc="https://pikaso.cdnpk.net/private/production/4024859125/d070ae9c-55df-47aa-acbe-4ee66337855c-0.mp4?token=exp=1777075200~hmac=4202c1d0ec90137eb6dffa8e0db93ed7569a68b2016165d8b1b567f888869ff5"
          isPlaying={isSecondVideoPlaying}
          baseRadius={520}
        />

        <div className="absolute right-[calc(8%+100px)] bottom-[12%] w-[calc(50%-50px)] h-[calc(50%+230px)] z-30" onMouseEnter={() => setIsSecondVideoPlaying(true)} onMouseLeave={() => setIsSecondVideoPlaying(false)} />
        <div className="absolute left-[calc(8%+200px)] top-[calc(20%+190px)] w-[calc(15%+250px)] h-[calc(22.5%+130px)] -translate-y-full z-30" onMouseEnter={() => setIsSecondVideoPlaying(true)} onMouseLeave={() => setIsSecondVideoPlaying(false)} />

        <div className="absolute left-[calc(8%+200px)] top-[20%] z-20 w-[320px] px-8 py-6 rounded-sm border border-white/10" style={{ background: 'rgba(0, 0, 0, 0.16)', backdropFilter: 'blur(80px)', WebkitBackdropFilter: 'blur(80px)' }}>
          <div className="flex items-end gap-2 mb-4">
            <span className="font-serif italic text-[#DA3A16] text-[72px] leading-[80px] tracking-tight">78%</span>
            <div className="w-[11px]">
              <svg style={{ width: '160px', height: '80px' }} viewBox="0 0 289 138" fill="none" xmlns="http://www.w3.org/2000/svg">
                <g filter="url(#filter0_d_878_28499)"><path d="M22.5 48.7306C39.7833 48.7306 49.34 54.94 63.1667 69.2965C76.9933 83.653 86.55 110.5 103.833 110.5C121.117 110.5 130.673 84.2876 144.5 59.2856C158.327 34.2837 167.883 19.5573 185.167 19.5573C202.45 19.5573 208.55 57.6673 225.833 57.6673C243.117 57.6673 249.217 19.5 266.5 19.5" stroke="#DA3A16" strokeWidth="2"/></g>
                <defs><filter id="filter0_d_878_28499" x="0" y="0" width="289" height="138" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB"><feFlood floodOpacity="0" result="BackgroundImageFix"/><feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/><feOffset dy="4"/><feGaussianBlur stdDeviation="11.25"/><feComposite in2="hardAlpha" operator="out"/><feColorMatrix type="matrix" values="0 0 0 0 0.854902 0 0 0 0 0.227451 0 0 0 0 0.0862745 0 0 0 1 0"/><feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_878_28499"/><feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_878_28499" result="shape"/></filter></defs>
              </svg>
            </div>
          </div>
          <h3 className="font-serif text-white text-[15px] tracking-[0.02em] uppercase mb-2 leading-tight">NEXT-GEN CUSHIONING ARCHITECTURE</h3>
          <p className="font-serif text-white/64 text-[13px]">Impact Absorption & Energy Return Dynamics</p>
        </div>

        <div className="absolute left-[8%] bottom-[12%] z-20 text-white max-w-[500px]">
          <h2 className="text-[44px] leading-[1.05] tracking-tight flex flex-col">
            <span className="font-sans font-medium">Bringing Aerospace-</span>
            <span className="font-sans font-medium">Grade Infrastructure</span>
            <span className="font-serif font-normal pt-1"><span className="not-italic">Directly To Your </span><span className="italic">Everyday</span></span>
            <span className="font-serif italic font-normal">Urban Exploration</span>
          </h2>
        </div>

        <div className="absolute right-[calc(8%+100px)] bottom-[12%] z-20 flex flex-col items-center">
           <div className="bg-white w-[180px] py-[6px] flex justify-center items-center">
              <span className="text-black font-serif text-[10px] uppercase font-bold tracking-[0.08em] text-center leading-[16px]">THE SCIENCE OF IMPACT CONTROL</span>
           </div>
           <div className="bg-[#DA3A16] w-[180px] h-[100px] flex justify-center items-center">
              <svg width="86" viewBox="135.5 361.38 420.32 149.8" fill="white" xmlns="http://www.w3.org/2000/svg">
                <path d="m181.86 511.11c-12.524-0.49755-22.77-3.9244-30.782-10.289-1.529-1.2159-5.1725-4.8616-6.3949-6.3992-3.2489-4.0853-5.4578-8.0611-6.931-12.472-4.5334-13.579-2.2002-31.397 6.6737-50.953 7.5979-16.742 19.322-33.347 39.776-56.344 3.013-3.384 11.986-13.281 12.043-13.281 0.0216 0-0.46749 0.84706-1.083 1.8786-5.3183 8.9082-9.8689 19.401-12.348 28.485-3.9823 14.576-3.502 27.085 1.4068 36.784 3.3862 6.6822 9.1913 12.47 15.719 15.67 11.428 5.5993 28.159 6.0625 48.592 1.3554 1.4068-0.32599 71.116-18.831 154.91-41.123 83.794-22.294 152.36-40.52 152.37-40.505 0.0237 0.0193-194.68 83.333-295.75 126.56-16.007 6.8431-20.287 8.5715-27.812 11.214-19.236 6.7551-36.467 9.9783-50.396 9.4251z"/>
              </svg>
           </div>
        </div>
      </section>
    </div>
  );
}

## Nimbus Grid — Landing Page [sites/nimbus-grid]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(63).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nimbus-grid.webp

Build a single-page marketing site called **Nimbus Grid** — a fictional secure cloud storage capacity platform. Use plain HTML, CSS, and vanilla JS (Vite project). Match every detail below exactly.

---

### Global Setup

**Fonts (Google Fonts, preconnect both gstatic + googleapis):**
- `IBM Plex Sans` weights 400, 500 — body/headings
- `IBM Plex Mono` weights 400, 500 — labels, code, nav, CTAs

**CSS variables (`:root`):**
```
--bg: #17130d
--ink: #fff4d5
--muted: #dacaa1
--line: rgba(255,240,199,0.28)
--glass: rgba(255,239,199,0.16)
--glass-strong: rgba(255,239,199,0.24)
--accent: #ead09a
--accent-2: #ffd879
--deep: #4d3f24
--radius: 8px
color-scheme: dark
```

**Body:** dark warm background `radial-gradient(circle at top left, rgba(255,216,121,0.18), transparent 28rem) + var(--bg)`, ink color `#fff4d5`, IBM Plex Sans, font-size 1rem, line-height 1.375, letter-spacing 0.0175rem, antialiased. `<meta name="theme-color" content="#17130d">`.

**Smooth scroll** on `html`. Universal `box-sizing: border-box`. Anchor links inherit color, no underline.

---

### Section 1 — Hero

Full-viewport (`min-height: 100svh`) section with:

- **Animated shader background** as an `<iframe class="shader-bg" src="https://fragcoord.xyz/embed/c6zisyc6?viewport=1422x800" allow="autoplay; fullscreen" referrerpolicy="no-referrer">` absolutely positioned, centered with `transform: translate(-50%,-50%) scale(var(--shader-scale,5))`, `z-index:-3`, pointer-events none.
- **Fallback layer** `.shader-fallback` behind it (`z-index:-4`) — radial+linear warm-gold gradient (`#846f43 → #f0d27c → #fff2be`) so the page still looks intentional if the shader fails.
- JS: on load + debounced resize (180ms), recompute viewport so shader iframe matches window aspect, capped at 1422×800, scale = max(window.innerWidth/width, (window.innerHeight+110)/height).

**Site header (`.site-header`)** — flex row, `min-height: 42px`:
- Brand `NIMBUS GRID` in a glass pill (`padding: 9px 12px`, 1px ink-translucent border, `backdrop-filter: blur(18px) saturate(1.35)`, IBM Plex Mono 12px uppercase, inset highlight + soft shadow).
- Right side: nav (`Technology`, `Security`, `Capacity`, `Operations`) — Plex Mono 12px uppercase, 0.04rem letter-spacing, ink-translucent color, hover brightens.
- `.header-cta` button "Get Started" — same glass pill style, hover lifts 1px and brightens.

**Hero layout (grid, two rows):**
- **Top-left console card (`.console-card`)** width `min(396px, 42vw)`, dark `rgba(13,16,19,0.88)` panel, 5px radius, blurred backdrop:
  - Tabs row (`grid-template-columns: repeat(3,minmax(0,77px)) 1fr auto`): `CLI`, `API`, `Console`, plus two fake window controls (small square + a wide bar). Active tab gets accent color and a 2px accent underline.
  - Three panes (only one shown):
    - **CLI:** `<pre>` showing `$ nimbus storage create \ --workspace prod-web \ --tier encrypted-fast \ --region eu-central` with `$` in accent color, then a typing-output line `storage pool web-db-test queued` in accent.
    - **API:** `POST /v1/storage/pools` JSON body `{name:"web-db-test", tier:"encrypted-fast", quota:"8 TiB"}`, output `202 accepted: provisioning policy attached`.
    - **Console:** mock form fields — `Instance name = web-db-test` (typed), `Image = ubuntu-24.04-noble`, two-column row `Memory = 8 GiB` / `CPUs = 2`. Each `console-input` is a 33px high outlined dark slot, the two select-style ones get a `▾` glyph appended.
  - Pane size `min-height:153px`, Plex Mono 11px text.
  - JS typewriter: per active pane, find `[data-typed]`, type one char every 42ms, blinking `::after` cursor (1px wide bar, `cursor-blink` 1s steps animation).

- **Hero copy** at bottom-left:
  - H1: "Cloud space that scales with your business systems." — Plex Sans 400, `clamp(29px,3.5vw,56px)`, line-height 1, max-width 18ch.
  - Paragraph: "Nimbus Grid sells secure cloud storage capacity for companies that need fast onboarding, predictable throughput, encrypted collaboration, and modern data residency controls." — `clamp(12px,1.125vw,16.5px)`, ink color, max-width 720px.
  - Add a soft dark radial blur behind the text (`::before` with blurred ellipse, filter blur 26px) so copy stays readable over the shader.

---

### Section 2 — Platform accordion (scroll-driven)

`#platform`, `min-height: 420svh`, near-black `#050604` background with subtle gold radial top-right.

- `position: sticky` inner panel (`.accordion-inner`) at `top:0`, full viewport height, two-column grid `0.22fr | 0.78fr`.
- **Left nav** (`.accordion-nav`): four pill labels in Plex Mono 11px uppercase, each prefixed by a 7px square dot:
  1. `Programmable infra`
  2. `Data residency`
  3. `Elastic scaling`
  4. `Unified visibility`
  Active tab uses accent color and shifts right 2px.
- **Right stack** (`.accordion-stack`, height `min(80svh, 820px)`): four `.accordion-card` panels stacked with `position:absolute; inset:0`. Each card is a two-column grid (copy + visual) on a black background with a 1px ink top border.
  - **Card 1 — Programmable infra:** copy + a code window:
    `01 storage_pool = { 02 name = "client-vault" 03 region = "eu-central" 04 quota = "24 TiB" 05 policy = encrypted_fast 06 }`
  - **Card 2 — Data residency:** code window with `Region policy / EU Central locked / US East allowed / AP Southeast review / Retention 7 years`.
  - **Card 3 — Elastic scaling:** `Capacity forecast / Used 18.4 TiB / Reserved 24 TiB / Burst ready / Next tier approved`.
  - **Card 4 — Unified visibility:** `Operations view / Sync health stable / Cold data 14% / Policy drift 0 / Audit export live`.
  - Each visual: warm gold gradient backdrop (`linear-gradient(135deg, rgba(234,208,154,0.92), rgba(106,91,52,0.68))` + radial highlight), centered dark code window with 3 dot-spans, 8px radius, deep shadow.

**Scroll behavior (JS):**
- Track section's `getBoundingClientRect()` → progress 0..1 over `(height - viewport)`.
- Map to active card index (rounded). Card N's translateY animates from `stackHeight + collapsedHeight` (off-bottom) up to `index * collapsedHeight` (collapsed=84px desktop / 96px mobile), clamped per segment.
- Each card sets `--card-y` (transform) and `--card-clip-bottom` (clip-path inset) so the active card fully reveals while previous cards stay as visible header strips.
- Clicking a tab smooth-scrolls window to that card's segment.

---

### Section 3 — Pricing

`#pricing`, dark olive `#11120f` with light top wash and a soft cyan radial blur (`rgba(151,211,235,0.14)`) bleeding from the top-left.

**Top grid** (max-width 1320px, two columns ~`0.38 | 0.62`):
- **Left copy:**
  - Eyebrow `Pricing` (accent, Plex Mono 16px uppercase).
  - H2: "Only pay for cloud storage your teams actually use." `clamp(34px,4vw,68px)`, line-height 1.
  - Paragraph: "Scale capacity up for active projects and cool it down when workspaces go quiet. Nimbus Grid keeps storage, transfer, and policy costs visible before they become invoices."
- **Right pricing table** (`.pricing-table`): header row "Storage costs" + a billing toggle pill (`Per month` muted, `Per GiB` active = accent pill with `#241d0f` text). Then 5 rows separated by 1px ink lines, each `1fr | auto`:
  - Encrypted active storage — `$0.021 / GiB / month`
  - Warm collaboration tier — `$0.012 / GiB / month`
  - Cold retained archive — `$0.004 / GiB / month`
  - Regional accelerated transfer — `$0.018 / GiB moved`
  - Customer-managed key vault — `included`
  Right values use Plex Mono.

**Pricing bars** — full-bleed (`width: 100vw; margin-left: calc(50% - 50vw)`), 12-column grid, `height: 480px`, bars aligned to bottom. Each bar height = `var(--bar-height) + var(--bar-morph,0px)`, min-height 120px, gold gradient (alternating "muted" variant). Heights start at 12 fixed values (66/58/50/62/45/54/48/64/72/70/78/82%). Top edge fades into the section via gradient overlay.

**JS** ties bar height to scroll position: `progress = (viewport - rect.top) / (viewport + rect.height)`, then per-bar `morph = sin(progress*2π + i*0.72)*34 + cos(progress*π + i*0.34)*14` px, written to `--bar-morph`. Transitions `height 80ms linear`.

**Plan row** below — 3 columns (Starter / Team / Enterprise), each card max 300px:
- Starter: "For small teams consolidating shared project files." CTA `Start small`.
- Team: "For departments scaling collaboration and regional transfer." CTA `Build team plan`.
- Enterprise: "For organizations prioritizing governance, residency, and support." CTA `Talk to sales`.
CTAs: 42px tall pill, Plex Mono 12px uppercase, 1px ink translucent border, glass background, hover brightens.

---

### Section 4 — Security

`#security`, `#120f0a` background with two soft radial highlights (gold top-right, warm orange bottom-left), 1320px max-width.

**Heading row** (two columns `0.58 | 0.42`):
- Left: eyebrow `Security` + H2 "Modern encryption and compliance controls without slowing the team down."
- Right paragraph: "Role-based access, customer-managed keys, immutable retention, and regional storage policies give business clients a cloud layer that can satisfy procurement, IT, and legal from the first deployment."

**Three security cards** (`grid-template-columns: repeat(3, 1fr)`, gap `clamp(16px,2vw,22px)`, each `min-height: 464px`, square corners, 1px ink border, `#0f0c08` with subtle top wash):

1. **API card — "Full policy control"** + copy "First-class API access for storage pools, keys, regions, and retention rules. No vendor lock-in to proprietary workflows."
   - Visual: a black `.api-window` (bottom-left, ~58% width, 184px tall) with three dots and pre-text:
     ```
     -> nimbus auth login
     Enter code
     VAULT-9AMP

     -> policy attach
     workspace/client-vault
     ```
   - An overlapping `.api-spec` (top-right, gold-tinted dark `rgba(64,52,30,0.86)`, accent border) showing:
     ```
     openapi: 3.0.0
     info:
       title: Nimbus API
     paths:
       /storage/pools:
       /keys:
       /regions:
       /retention:
     ```

2. **Compliance card — "Full compliance"** + copy "SOC 2, ISO 27001, and GDPR-ready controls help teams satisfy audits, procurement reviews, and data residency requirements." Below: three rows, each a 24px circular accent badge with a checkmark drawn via `::before` (rotated bottom+left borders), small label, accent strong line:
   - SOC 2 — Type II controls
   - ISO 27001 — Security management
   - GDPR — Regional data policy
   Rows are `rgba(48,39,23,0.84)` with accent-translucent borders.

3. **Economics card — "Ownership and predictable economics"** + copy "Reserved capacity, clear transfer lanes, and audit-ready billing make storage spend easy to forecast across business units."
   - Visual: `<pre class="binary-map">` of 1s and 0s drawing a small graphic (10 rows, 28 columns, see the exact pattern in the original — a small icon shape carved out of 1s).
   - Below: 3-row asset table — `Reserved tier | 24 TiB`, `Transfer lane | EU Central`, `Revision | Q603`. Mono 11px uppercase labels, mixed-case values.

---

### Section 5 — Console showcase

`#plans`, dark teal-leaning `#070a0b` with cyan radial accent. Includes a faint repeating-stripe block (decorative `::after`, top-right).

**Heading row:** H2 "The biggest forward leap in business cloud storage operations." (`clamp(25px,4vw,52px)`, color `#dff5ff`) + right paragraph "A single control plane for provisioning storage pools, reviewing policy, watching growth, and shipping audit-ready reports without asking teams to change how they work."

**Figure label:** small Plex Mono pill `Fig. 2  Nimbus Grid web console`.

**Dashboard shell** (`.dashboard-shell`):
- Full-width, 8px radius, cyan-translucent border, `rgba(5,8,10,0.9)` background, deep shadow, perspective transform.
- Topbar: 3 dots + a placeholder title bar.
- Body grid `240px | 1fr`:
  - **Sidebar** "Client Vault" + nav items: Workspaces, **Storage Pools** (active, cyan tint), Retention, Access, Transfers, Reports.
  - **Main:** title row "Storage Pools" (cyan `#97d3eb`) + `New pool` cyan-outlined button. Then a 5-column table:
    | Name | Region | Used | Policy | State |
    | finance-vault | EU Central | 18.4 TiB | 7 years | Healthy |
    | design-assets | US East | 9.8 TiB | Versioned | Syncing |
    | legal-archive | EU Central | 42.1 TiB | Immutable | Healthy |
    | migration-lane | AP South | 6.2 TiB | Temporary | Queued |
    Headers in Plex Mono uppercase, States in cyan Plex Mono uppercase.
- **Toast** absolutely positioned bottom-right: "Pool created / finance-vault ready" (cyan, dark background).
- Hover effect: shell tilts subtly (`rotateX(1deg) rotateY(-1.2deg) translateY(-8px)`), border brightens, a sheen pseudo-element sweeps left→right (`transform: translateX(-34%) → 34%`, opacity 0→1).

---

### Section 6 — Operations cube

`#operations`, `#0c0d0a` with cyan + gold radial accents; left-to-right dark gradient overlay so the copy reads cleanly.

**Two columns** `0.44 | 0.56`:

- **Left copy:** eyebrow `Operations`, H2 "A control layer for every storage move your business makes." (`clamp(34px,4.4vw,72px)`, line-height 0.98), paragraph "Route migrations, active workspaces, archives, and compliance exports through one operational grid. Nimbus Grid keeps capacity, policy, and transfer status visible before teams hit a limit." CTA button `Plan operations` — solid accent gold pill, dark `#1b160d` text, hover swaps to `--accent-2` and lifts 2px.

- **Right visual:** a 3D cube with explode-on-click animation.
  - `.modal-cube-shell` button, perspective 1000px, `transform-style: preserve-3d`.
  - `.operations-core-cube` size `clamp(142px,18vw,250px)` with 6 `.cube-face` divs (front/back/right/left/top/bottom). Each face: 18px radius, gold-blue radial gradient (`radial-gradient(circle at 48% 44%, rgba(255,216,121,0.98)…) + linear 135deg cyan→gold→dark`), inset highlights and shadows.
  - Idle: floats with `core-cube-float` 6s ease-in-out infinite (small Y bob and rotation drift).
  - On click (toggle `is-exploded`): core cube scales to 0.72; ~14 `.cube-particle` shards (10 cube fragments + 4 small `.dot` spheres) translate to randomized `--tx/--ty/--tz` offsets with `--s`, `--r`, staggered `--d` delays. Particles use `cubic-bezier(0.17,0.78,0.18,1)` 760ms transform + 420ms opacity; start blurred + dim, end sharp. Use the exact 14 particle definitions from the original (see hero-section markup pattern, ranges roughly tx: -310..330, ty: -250..225, tz: 30..210, s: 0.09..0.58).
  - JS: on `click` (also Enter/Space when focused), toggle the `is-exploded` class. Focus outline 1px ink-translucent, offset 10px.

---

### Responsive Behavior

**`@media (max-width: 820px)`:**
- Header collapses to single column, nav wraps full-width, CTA full-width.
- Hero layout stacks; console card becomes full width; the diagonal `.console-line` decoration hides.
- Console tabs become 3 equal columns (48px tall). Window controls hide. Pane min-height 200px.
- Pricing top + plan row + security grid stack to single column.
- Accordion: nav 2-column grid above the stack, stack height 78svh, cards become 1-column.
- Console showcase: heading stacks; dashboard body single column; sidebar nav 2-cols; table drops Policy + State columns; toast becomes inline at bottom.
- Operations: stacks; cube `--spread: 0.72`.

**`@media (max-width: 520px)`:**
- Hero padding 22px 18px 0; H1 `clamp(28px,10vw,48px)`; copy 15px.
- Accordion nav 1-column.
- Operations cube `--spread: 0.48`; visual min-height 360px.
- Dashboard title row stacks vertically.

---

### Animations Summary

- `cursor-blink` — 1s infinite blinking caret in console (steps(2,start)).
- `core-cube-float` — 6s infinite gentle Y bob + tiny rotation drift on idle cube.
- Bar heights — JS-driven `--bar-morph` updates on scroll, eased to height with `transition: height 80ms linear`.
- Accordion cards — JS-driven `--card-y` translate + `--card-clip-bottom` clip-path follow scroll progress.
- Dashboard shell hover — 220ms ease 3D tilt + sheen sweep (520ms ease).
- Operations CTA hover — 160ms color/transform.
- Operations cube — click toggles `.is-exploded`: core 620ms cubic-bezier transform; shards 760ms cubic-bezier transform + 420ms opacity, staggered delays.
- Header CTA / accordion-tab / nav links — 160–200ms hover transitions.
- Smooth scroll on tab → section navigation.

---

### Project structure

```
index.html         (full markup)
styles.css         (all styles + media queries)
script.js          (shader resize, console tabs typing, accordion scroll, bars, cube)
package.json       (vite ^5.4.2, type:module, scripts: dev/build/preview)
vite.config.js     (default)
```

Build with `npm run build`. The site uses no frameworks, no images — every visual is CSS/SVG/text.

## NOVA Space Systems — Landing Page [sites/nova-space-landing]

- Preview: https://motionsites.ai/assets/hero-nova-space-preview-ej0OOJ0M.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/nova-space-landing.gif

Build a NOVA Space Launch Systems landing page — a dark, minimal aerospace website using React + Vite + Tailwind CSS + TypeScript + shadcn/ui. The aesthetic is brutally minimal with a pure black/white monochrome palette, no border-radius anywhere (--radius: 0rem), and a single font: Space Grotesk (loaded from Google Fonts, weights 400–700) used for both display and body text.

🎨 Design System (index.css + tailwind.config.ts)
Color Palette (all HSL, light mode only — dark mode is unused):

--background: 0 0% 0% (pure black)
--foreground: 0 0% 100% (pure white)
--muted-foreground: 0 0% 65% (gray for body text)
--nav-border: 0 0% 35% (subtle gray divider lines)
--border: 0 0% 25%
--radius: 0rem (no rounded corners anywhere)
All other tokens (card, popover, primary, secondary, accent, destructive, sidebar) follow the same black/white monochrome scheme.
Tailwind config:

fontFamily.display and fontFamily.body both map to "Space Grotesk", sans-serif
Custom color nav-border: hsl(var(--nav-border)) for decorative divider lines
tailwindcss-animate plugin installed

📐 Page Layout (Index.tsx)
The page is a single vertical scroll with 3 sections:

Hero fills exactly 100vh including navbar

🧭 Section 1: Navbar
A custom-built navigation bar (no shadcn component). Desktop (md+): 3-column layout using flex items-stretch h-16:

Left: Links "Programs", "Systems", "Discover" — text-sm font-body tracking-wide, separated by vertical 1px bg-nav-border divider lines (with mt-3 mb-3 ml-3 insets). Below the links, a horizontal h-px bg-nav-border line with px-4 mt-1.
Center: Logo "NOVA" — text-2xl font-display font-bold tracking-widest, centered with flex-1. Below it, two side-by-side horizontal divider lines (flex gap-4, each flex-1 h-px bg-nav-border).
Right: "Search" text + Search icon + Menu icon from lucide-react. Same vertical divider treatment as left side (mt-3 mb-3 mr-3).
Mobile: Simplified — logo left, menu icon right, full-width bottom border.

🚀 Section 2: HeroSection
Full viewport height (h-[calc(100vh-theme(spacing.16))]).

Background Video: Autoplaying, muted, looping video. URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_031824_0c85e1e9-fe2b-4d52-8cde-25b0c2b5e8a2.mp4
object-cover fills the container; focal point shifts right on smaller screens (85% mobile, 75% tablet, center on desktop)
No overlay/gradient — video is fully visible

Content overlay (relative z-10 px-8 pt-12 pb-24):
H1: "ROCKETS" — text-[4.5rem] md:text-[10rem] lg:text-[12rem] font-display font-black leading-[0.85] tracking-tighter text-foreground
Right-aligned description block (flex justify-end mt-4, inner flex flex-col items-end max-w-3xl):
Paragraph: "From precise orbital insertions to deep space trajectories, NOVA's launch systems deliver unmatched performance and dependability, backed by over five decades of proven spaceflight excellence." — text-base md:text-lg font-body text-foreground/90 leading-relaxed text-right
CTA Button: "View Our Fleet" — rectangular, no border-radius, bg-foreground text-background pl-5 pr-1.5 py-1.5 font-body text-sm tracking-wide. Contains text + a square icon container (w-10 h-10 bg-background) with ArrowUpRight icon from lucide-react. hover:opacity-90 transition-opacity.

🔬 Section 3: RocketScienceSection
Title block (px-8 pt-32 pb-16 flex justify-center):
"ROCKET" on line 1, "SCIENCE" on line 2 indented with ml-[1.5em] md:ml-[3.5em]
text-[2.5rem] md:text-[6rem] lg:text-[7rem] font-display font-extralight leading-[0.9] tracking-tighter uppercase

Grid area (px-8):
Top border: 4 equal h-px bg-nav-border lines in a flex gap-4
Decorative vertical dividers on left, center, and right edges (1px lines with gap-4 between segments, inset with ml-3/mr-3 py-3)
Desktop (lg+): A CSS Grid grid-cols-[auto_1fr_auto_1fr_auto] grid-rows-3 creating a staggered 2×3 layout:

Top-left: Text card — heading "How do we get to space?" (text-2xl md:text-3xl font-display font-normal) + body paragraph (font-body text-muted-foreground leading-relaxed text-lg). Padding p-12.
Top-right spanning 2 rows: Video (p-6, w-full h-full object-cover): https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032431_5e054107-51c0-4162-9f0f-3a40054761ef.mp4
Bottom-left spanning 2 rows: Video (p-6): https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260324_032535_4ccc152e-0cc8-4ee5-a698-e1a98cea8a1e.mp4
Bottom-right: Text card — heading "Launch vehicles" + body paragraph. Same styling. p-12.
Horizontal dividers between rows (2 segments per divider, flex gap-4)

Mobile/Tablet: Stack cards vertically with horizontal dividers between each.

Tech Stack: React 18, TypeScript, Tailwind CSS v3, Vite with @vitejs/plugin-react-swc, shadcn/ui (installed but only used for design tokens, not for these sections).

Font Loading (index.html):
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&display=swap" rel="stylesheet">

## Orbis NFT — Landing Page [sites/orbis-nft-landing]

- Preview: https://motionsites.ai/assets/hero-orbis-nft-preview-C3wvh77a.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/orbis-nft-landing.gif

Create an NFT landing page called "Orbis.Nft" with 4 sections, using a dark space theme. The page uses video backgrounds served from CloudFront, a liquid glass UI effect, and a specific color/font system. Recreate it exactly as described below.

FONTS (Google Fonts)

Anton - Used for all headings and navigation text (aliased as font-grotesk in Tailwind)

Condiment - A cursive script used for accent/overlay text (aliased as font-condiment in Tailwind)

System monospace font (font-mono) - Used for body/description paragraphs

Load via Google Fonts in index.html:

https://fonts.googleapis.com/css2?family=Anton&family=Condiment&display=swap


COLOR SYSTEM (Tailwind config)

Background: #010828 (deep dark navy blue)

cream: #EFF4FF (off-white, used for all text)

neon: #6FFF00 (bright green, used for accent cursive text and underline bars)

LIQUID GLASS CSS EFFECT

Applied via a .liquid-glass class. This is used on the navbar, social icon buttons, NFT cards, and card overlays:

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


TEXTURE OVERLAY

A full-screen fixed texture overlay sits on top of everything (z-50, pointer-events-none). It uses a /texture.png image with mix-blend-mode: lighten at opacity: 0.6, covering the entire viewport with background-size: cover.

SECTION 1: HERO (Full viewport)

Background: Full-bleed looping muted autoplaying video covering the entire section with object-cover

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_045634_e1c98c76-1265-4f5c-882a-4276f2080894.mp4

Container: max-w-[1831px] centered with responsive horizontal padding

Section has rounded-b-[32px] bottom corners, clipping the video

Header:

Left: "Orbis.Nft" logo text in Anton, 16px, uppercase

Center: Navigation bar with liquid-glass effect, rounded-[28px], px-[52px] py-[24px]. Contains 5 links: Homepage, Gallery, Buy NFT, FAQ, Contact. Each link is Anton 13px uppercase. Links have hover:text-neon transition. Nav is hidden on mobile (hidden lg:block).

Hero Content:

Large heading in Anton font, responsive sizing: 40px mobile / 60px sm / 75px md / 90px lg. Uppercase. leading-[1.05] mobile, leading-[1] tablet+. Max width 780px on desktop, offset with lg:ml-32.

Text reads:

Beyond earth
and ( its ) familiar boundaries


Overlaid cursive accent text "Nft collection" in Condiment font (24px-48px responsive), positioned absolute to the right side of the heading, slightly rotated (-rotate-1), in neon green (text-neon), with mix-blend-exclusion and opacity-90.

Social Icons (Desktop):

3 square buttons (56x56px) stacked vertically in top-right corner, each with liquid-glass and rounded-[1rem]. Icons: Mail, Twitter, Github from lucide-react (20x20px). hover:bg-white/10 transition.

Social Icons (Mobile):

Same 3 buttons but centered horizontally below the heading, shown only below lg breakpoint.

SECTION 2: ABOUT / INTRO (Full viewport)

Background: Full-bleed looping muted autoplaying video with object-cover

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_151551_992053d1-3d3e-4b8c-abac-45f22158f411.mp4

Container: Same max-w-[1831px] centered, with generous vertical padding (64px-96px responsive)

Top Row (flex row on desktop, column on mobile):

Left: Heading in Anton, responsive 32px-60px, uppercase:

Hello!
I'm orbis


With an overlaid "Orbis" in Condiment cursive, neon green, mix-blend-exclusion, 36px-68px responsive, positioned absolute at bottom-right of heading, slightly rotated.

Right: Short paragraph in monospace 14px-16px, uppercase, cream color, max-width 266px: "A digital object fixed beyond time and place. An exploration of distance, form, and silence in space"

Bottom Row (flex row, space-between):

Two columns (left and right), each containing 2 identical paragraphs. Same monospace text as above but at opacity-10 (nearly invisible, decorative). Right column hidden below lg. On mobile, text uses text-[#010828] (dark) so it's effectively invisible against the video.

SECTION 3: NFT COLLECTION GRID

Background: Solid #010828 (no video)

Container: Same max-w-[1831px] centered

Header Row:

Left: Heading in Anton, 32px-60px responsive, uppercase:

Collection of
  [indented] Space objects


Where "Space" is in Condiment cursive neon green, and "objects" is in Anton. The second line is indented with ml-12 / ml-24 / ml-32 responsive.

Right: A "SEE ALL CREATORS" button. "SEE" is large (32px-60px), "ALL" and "CREATORS" are stacked smaller (20px-36px) next to it. Below the text is a neon green bar (bg-neon, height 6px-10px responsive, full width of button).

NFT Card Grid:

3-column grid on desktop (lg:grid-cols-3), 2 on tablet, 1 on mobile. Gap 24px.

Each card: liquid-glass container with rounded-[32px], padding 18px, hover:bg-white/10 transition.

Inside each card: a square video container (pb-[100%] aspect ratio trick) with rounded-[24px] overflow hidden.

Video URLs:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_053923_22c0a6a5-313c-474c-85ff-3b50d25e944a.mp4 (Score: 8.7/10)

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_054411_511c1b7a-fb2f-42ef-bf6c-32c0b1a06e79.mp4 (Score: 9/10)

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_055427_ac7035b5-9f3b-4289-86fc-941b2432317d.mp4 (Score: 8.2/10)

Each card has an overlay bar at the bottom: a liquid-glass bar with rounded-[20px], px-5 py-4, showing "RARITY SCORE:" label (11px, cream/70% opacity) and score value (16px). On the right side of the bar is a circular purple gradient button (48x48px, bg-gradient-to-br from-[#b724ff] to-[#7c3aed]) with a right-arrow chevron SVG inside, with shadow-lg shadow-purple-500/50 and hover:scale-110 transition.

SECTION 4: CTA / FINAL SECTION

Background: Full-width video (NOT object-cover, instead w-full h-auto block so it displays at native aspect ratio)

Video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260331_055729_72d66327-b59e-4ae9-bb70-de6ccb5ecdb0.mp4

Text Content (positioned absolute over the video):

Right-aligned block, offset with lg:pr-[20%] lg:pl-[15%]

Small "Go beyond" text in Condiment cursive, neon green, mix-blend-exclusion, positioned absolute at top-left of the heading block. Sizes: 17px-68px responsive.

Heading in Anton, responsive 16px-60px, uppercase:

JOIN US.
REVEAL WHAT'S HIDDEN.
DEFINE WHAT'S NEXT.
FOLLOW THE SIGNAL.


"JOIN US." has extra bottom margin (mb-4 to mb-12 responsive) before the remaining lines.

Social Icons (Bottom-left, absolute positioned):

Positioned at left-[8%], bottom-[12%] to bottom-[20%] with responsive breakpoints.

A vertical liquid-glass container with rounded-[0.5rem] to rounded-[1.25rem] responsive, containing 3 stacked icon buttons (Mail, Twitter, Github).

Buttons have responsive widths using viewport units and rem values (e.g., w-[14vw] sm:w-[14.375rem] md:w-[10.78125rem] lg:w-[16.77rem]) and similar responsive heights.

Buttons are separated by border-b border-white/10 dividers (except the last one).

KEY TECHNICAL DETAILS

Framework: React + TypeScript + Vite + Tailwind CSS

Icons: lucide-react (Mail, Twitter, Github)

No additional packages needed beyond what Vite + React + Tailwind provides

All videos: autoPlay loop muted playsInline attributes

Responsive: Mobile-first with sm:, md:, lg: breakpoints throughout

Max content width: 1831px across all sections

All text is uppercase except the Condiment cursive accents which are normal-case

## Prisma Creative Studio — Landing Page [sites/prisma-landing]

- Preview: https://motionsites.ai/assets/hero-prisma-preview-D4QeI0Bn.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/prisma-landing.gif

Create a React + Vite + TypeScript + Tailwind CSS landing page for a creative studio called "Prisma". The page has 3 sections: Hero, About, and Features. Use framer-motion for animations and lucide-react for icons. The design is dark, moody, and cinematic with a warm cream color palette.

FONTS

Load two Google Fonts in index.html:

Almarai (weights: 300, 400, 700, 800) -- used as the global default font
Instrument Serif (italic only) -- used for italic accent text in the About section
In index.css, set the global font family:


* { font-family: 'Almarai', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', sans-serif; }
In tailwind.config.js, extend:

colors.primary: #DEDBC8 (warm cream, used for all primary text and accents)
fontFamily.serif: ['"Instrument Serif"', 'serif']
COLOR SYSTEM

Background: black (#000000) globally, #101010 for the About card, #212121 for Features cards
Primary text color: #E1E0CC (applied via inline style, slightly different from Tailwind primary)
Tailwind primary: #DEDBC8 (used for utility classes like text-primary, text-primary/70)
Gray text: text-gray-400, text-gray-500
Navbar link color: rgba(225, 224, 204, 0.8) with hover: #E1E0CC
CUSTOM CSS UTILITIES (index.css)

Two SVG noise texture utilities:

.noise-overlay: fractal noise (baseFrequency: 0.85, numOctaves: 3) used as overlay on hero video
.bg-noise: fractal noise (baseFrequency: 0.9, numOctaves: 4) used as subtle background in Features section
Both use inline SVG data URIs with feTurbulence filter.

SECTION 1: HERO

Full viewport height (h-screen). The entire section has p-4 md:p-6 padding creating an inset effect. Inside is a container with rounded-2xl md:rounded-[2rem] and overflow-hidden.

Background video:

URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_170732_8a9ccda6-5cff-4628-b164-059c500a2b41.mp4
autoPlay loop muted playsInline, object-cover, fills entire container
Noise overlay on top: .noise-overlay with opacity-[0.7] mix-blend-overlay pointer-events-none
Gradient overlay: bg-gradient-to-b from-black/30 via-transparent to-black/60
Navbar:

Absolutely positioned at top center
Black background pill that hangs from top edge: bg-black rounded-b-2xl md:rounded-b-3xl px-4 py-2 md:px-8
5 nav items: "Our story", "Collective", "Workshops", "Programs", "Inquiries"
Text size: text-[10px] sm:text-xs md:text-sm
Gap between items: gap-3 sm:gap-6 md:gap-12 lg:gap-14
Link color: rgba(225, 224, 204, 0.8), hover: #E1E0CC (inline styles)
Hero Content (bottom-aligned):

Absolutely positioned at bottom: absolute bottom-0 left-0 right-0
12-column grid: left 8 columns for heading, right 4 columns for text + button
Giant heading "Prisma" using WordsPullUp component:
Responsive sizes: text-[26vw] sm:text-[24vw] md:text-[22vw] lg:text-[20vw] xl:text-[19vw] 2xl:text-[20vw]
font-medium leading-[0.85] tracking-[-0.07em]
Color: #E1E0CC
Has a superscript asterisk (*) on the final "a" of "Prisma": positioned with absolute top-[0.65em] -right-[0.3em] text-[0.31em]
Pull-up animation: each word slides up from y:20 with staggered delay of 0.08s, triggered by useInView
Description paragraph (right column):
"Prisma is a worldwide network of visual artists, filmmakers and storytellers bound not by place, status or labels but by passion and hunger to unlock potential through our unique perspectives."
text-primary/70 text-xs sm:text-sm md:text-base, line-height: 1.2
Framer motion: fade up from y:20, delay 0.5s, custom ease [0.16, 1, 0.3, 1]
CTA Button "Join the lab":
Pill shape: bg-primary rounded-full
Black text, font-medium, text-sm sm:text-base
Right side has a black circle (bg-black rounded-full w-9 h-9 sm:w-10 sm:h-10) containing a white/cream ArrowRight icon
Hover: gap increases (hover:gap-3), circle scales up (group-hover:scale-110)
Framer motion: fade up from y:20, delay 0.7s, same custom ease
SECTION 2: ABOUT

bg-black, padded section with centered content
Inner card: bg-[#101010], centered text, max-w-6xl
Top: small label "Visual arts" in text-primary, text-[10px] sm:text-xs
Main heading uses WordsPullUpMultiStyle component with 3 segments:
"I am Marcus Chen," -- font-normal (Almarai)
"a self-taught director." -- italic font-serif (Instrument Serif italic)
"I have skills in color grading, visual effects, and narrative design." -- font-normal
Container: text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl max-w-3xl mx-auto leading-[0.95] sm:leading-[0.9]
Each word animates in with pull-up effect (y:20 to y:0), staggered at 0.08s delay
Body paragraph below with scroll-linked character opacity animation:
Text: "Over the last seven years, I have worked with Parallax, a Berlin-based production house that crafts cinema, series, and Noir Studio in Paris. Together, we have created work that has earned international acclaim at several major festivals."
text-[#DEDBC8], text-xs sm:text-sm md:text-base
Each character is individually wrapped in an AnimatedLetter component
Uses useScroll with target offset ['start 0.8', 'end 0.2']
Each character's opacity transitions from 0.2 to 1 based on scroll position, creating a progressive text reveal effect
Character staggering: charProgress = index / totalChars, range [charProgress - 0.1, charProgress + 0.05]
SECTION 3: FEATURES

min-h-screen bg-black, with subtle .bg-noise overlay at opacity-[0.15]
Header text uses WordsPullUpMultiStyle:
Line 1: "Studio-grade workflows for visionary creators." in cream
Line 2: "Built for pure vision. Powered by art." in text-gray-500
Both: text-xl sm:text-2xl md:text-3xl lg:text-4xl font-normal
4-column card grid (lg:h-[480px], gap-3 sm:gap-2 md:gap-1):

Each card has staggered entrance animation: scale from 0.95 + fade in, triggered by useInView (once, margin "-100px"), staggered at 0.15s intervals with ease [0.22, 1, 0.36, 1].

Card 1 - Video card: Full video background (URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260406_133058_0504132a-0cf3-4450-a370-8ea3b05c95d4.mp4), autoPlay loop muted playsInline, object-cover. Bottom text: "Your creative canvas." in #E1E0CC.

Card 2 - "Project Storyboard." (01): bg-[#212121], small image icon at top (https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_171918_4a5edc79-d78f-4637-ac8b-53c43c220606.png&w=1280&q=85, 10x10 sm:12x12 rounded), title with number, 4 checklist items with green Check icons, "Learn more" link with rotated arrow (-45deg).

Card 3 - "Smart Critiques." (02): Same layout as Card 2. Icon: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_171741_ed9845ab-f5b2-4018-8ce7-07cc01823522.png&w=1280&q=85. 3 checklist items about AI analysis, creative notes, tool integrations.

Card 4 - "Immersion Capsule." (03): Same layout. Icon: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_171809_f56666dc-c099-4778-ad82-9ad4f209567b.png&w=1280&q=85. 3 checklist items about notification silencing, ambient soundscapes, schedule syncing.

All feature card checklist items use Check icon from lucide-react in text-primary color, with text-gray-400 description text. "Learn more" buttons use ArrowRight rotated -45deg.

SHARED ANIMATION COMPONENTS

WordsPullUp: Splits text by spaces, each word is a motion.span that slides up (y:20 to 0) with staggered delay. Uses useInView (once: true). Supports showAsterisk prop that adds a superscript * after the last character "a" of the final word.

WordsPullUpMultiStyle: Takes an array of {text, className} segments, splits all into individual words preserving per-word className. Same pull-up animation. Words are wrapped in inline-flex flex-wrap justify-center.

RESPONSIVE BREAKPOINTS

The page is fully responsive across mobile, tablet, and desktop. Cards in Features switch from 1-col (mobile) to 2-col (md) to 4-col (lg). Hero text scales from 26vw down to 19vw. Navbar items compress with smaller gaps on mobile. All padding, font sizes, and spacing use Tailwind responsive prefixes (sm/md/lg/xl/2xl).

TECH STACK

Vite + React 18 + TypeScript
Tailwind CSS 3
framer-motion (for all animations: pull-up text, fade-in, scroll-linked opacity, card entrances)
lucide-react (ArrowRight, Check icons)

## PROMPT — Landing Page [sites/prompt-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/fe42Area.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/prompt-hero.mp4

### Overview

Build a full-screen, scroll-driven fashion/archive landing page for a brand called "prmpt". The page has two main phases:

1. **Hero phase** (first 100vh of scroll): Full-viewport video background with overlaid UI (logo, nav, product info, custom cursor). A black panel slides up from below covering the video.
2. **Gallery phase** (continues scrolling): The black panel contains a scattered grid of product images that scale in/out as they enter/exit the viewport. At the end, a white overlay fades in with a "view" CTA button.

---

### Tech Stack

- **React 19** + **TypeScript**
- **Vite 6** with `@vitejs/plugin-react`
- **Tailwind CSS v4** via `@tailwindcss/vite` plugin
- **GSAP 3.15** + `@gsap/react` (ScrollTrigger)
- **Motion (Framer Motion) 12** (`motion/react`)
- **Font**: "Inter Tight" (Google Fonts, weight 500) -- loaded via `<link>` or import

---

### Asset URLs

**Videos (CloudFront):**
- LEFT video: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260625_154433_532a85d3-dabf-4265-b8bd-19ac6af31842.mp4`
- RIGHT video: `https://d8j0ntlcm91z4.cloudfront.net/user_39ca84eAE1ODL9hbR5VhoEj8tBf/hf_20260625_154401_a664f076-b971-4557-8728-40ef9ea4c49b.mp4`

**Gallery Images (10 total, in order):**
1. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_104530_521b2f85-c0f3-4d0e-9704-b578315b4cb9.png&w=1920&q=85`
2. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103711_76ccdb8b-5043-4f47-9c54-4379713393ea.png&w=1920&q=85`
3. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103728_394f6a1b-85e2-4386-a4f6-408472a0a5b7.png&w=1920&q=85`
4. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103739_86743e0e-16a7-4bee-bf38-dd67985344dc.png&w=1920&q=85`
5. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103748_b2215dc8-a3a7-470d-b19a-5b87fa7d0c37.png&w=1920&q=85`
6. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103758_e919ce72-5c9d-4b87-9be6-d7647b34825c.png&w=1920&q=85`
7. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103808_013583d0-3386-4547-9832-37c7d8edb3ac.png&w=1920&q=85`
8. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103937_a0c49d0a-33eb-4ead-aea6-c1baf241acbc.png&w=1920&q=85`
9. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_103956_d18ed8fd-7b6f-4b86-91f9-20010fe38670.png&w=1920&q=85`
10. `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260629_104034_ba5a9963-87ff-4008-a545-6bd686c088b5.png&w=1920&q=85`

---

### SECTION 1: Hero (Video Background + Overlaid UI)

### Root Container
- `id="scroll-spacer"`, `position: relative`, `user-select: none`, `background: white`
- Height is dynamically calculated (initially `500vh`, then overridden by GSAP to `vh + maxScroll + 2*vh`)
- Custom cursor hidden on desktop (`cursor: none`), default on touch devices

### 1A. Custom Cursor (Desktop Only)
- Hidden on mobile/tablet (< 1024px)
- A `fixed`, `pointer-events-none`, `z-index: 50` div that follows `mousemove`
- Positioned via direct DOM manipulation (`style.left/top = clientX/clientY`)
- `transform: translate(-50%, -50%)` to center on pointer
- `mix-blend-mode: exclusion`
- Contains a 48x48 SVG: a circle with stroke (r=22.75, strokeWidth=2.5) containing a custom Japanese/decorative glyph path, all filled white

### 1B. Logo (Top Left)
- `position: fixed`, `pointer-events-none`, `z-index: 20`
- `mix-blend-mode: exclusion`
- Responsive width: 124px (mobile < 640), 266px (tablet 640-1024), 355px (desktop)
- Position: `top: 16px, left: 16px` (mobile), `top: 32px, left: 32px` (desktop)
- Motion animation: fade in + slide up (`opacity: 0->1, y: 12->0`), duration 0.6s, ease `[0.25, 0.1, 0.25, 1]`, delay 0s
- SVG viewBox `0 0 355 110`, contains the "prmpt" wordmark + circled "R" mark, all paths filled white

### 1C. Caption (Below Logo, Left Side)
- `position: fixed`, `pointer-events-none`, `z-index: 20`
- `mix-blend-mode: exclusion`
- Position: `left: 32px` (desktop), `left: 16px` (mobile)
- Top: 244px (desktop), 180px (tablet), 118px (mobile)
- Width: 692px (desktop), `calc(50vw - 48px)` (tablet), `calc(100vw - 32px)` (mobile)
- Font: Inter Tight, weight 500, size 12px, line-height 140%, letter-spacing -0.04em, color #FFFFFF
- Motion animation: same as logo but delay 0.3s
- Text content: "When switching between videos near the center, do not reset currentTime to 0 abruptly. Add a small dead zone: if cursor is within +/-50px of center, keep both videos at currentTime = 0 and show whichever was last active."

### 1D. Header Navigation (Top Right)
- `position: fixed`, `z-index: 20`, `pointer-events-none`
- `mix-blend-mode: exclusion`
- Position: `top: 32px, right: 32px` (desktop), `top: 16px, right: 16px` (mobile)
- Width: 330px (desktop), auto (mobile)
- Height: 30px
- Flex row, justify-content: space-between, align-items: center
- Motion animation: same easing, delay 0.15s
- Contains:
  - "ABOUT" text (hidden on mobile): Inter Tight, 500, 15px, uppercase, white
  - A flex row with gap 50px (desktop) / 20px (mobile):
    - Hamburger SVG icon: viewBox `0 0 40 40`, two horizontal lines (`M0 14H40` and `M0 26H40`), stroke white, strokeWidth 2.5. Size: 30x30 (desktop), 24x24 (mobile)
    - "[ CART ]" text: Inter Tight, 500, 15px (desktop) / 13px (mobile), white

### 1E. Product Info (Bottom Right)
- `id="outro-info"`, `position: fixed`, `pointer-events-none`, `z-index: 20`
- `mix-blend-mode: exclusion`
- **Desktop**: right: 32px, bottom: 80px, width: 330px, flex-column, align center
- **Mobile**: left: 0, right: 0, bottom: 48px, flex-column, align center
- Motion animation: opacity 0->1, delay 0.45s
- `data-outro-offset`: 166 (desktop), 132 (mobile) -- used by scroll animation
- Contains:
  - Top block (flex-column, align flex-start, width 100% desktop / 252px mobile, margin-bottom 32px desktop / 12px mobile):
    - Circle icon: relative div (30x30 desktop, 20x20 mobile) containing:
      - SVG circle (cx=20, cy=20, r=18.75, stroke white, strokeWidth 2.5 desktop / 2 mobile)
      - `<span id="circle-symbol">` centered inside, shows "8" initially, changes to random symbol from `['8', '$', '^^', '%', '/']` on scroll (throttled 80ms)
      - Font: Inter Tight, 500, 15px (desktop) / 10px (mobile), letter-spacing -0.04em, uppercase, white
    - Collection label: Inter Tight, 500, 30px (desktop) / 20px (mobile), line-height 100%, text-align center, letter-spacing -0.04em, uppercase, white. Content: `ARCHIVE COLLECTION` + line break + `"PROMPT"`
  - Price: Inter Tight, 500, 80px (desktop) / 60px (mobile), line-height 100%, text-align center, letter-spacing -0.04em, white. Content: `$97,33`

### 1F. "View" Button (Bottom Right, Initially Hidden)
- `id="outro-buy"`, `position: fixed`, `pointer-events-none`, `z-index: 20`
- `mix-blend-mode: exclusion`
- **Desktop**: right: 32px, bottom: 32px, width: 330px, height: 174px
- **Mobile**: left: 16px, right: 16px, bottom: 60px, height: 100px
- `transform-origin: right bottom`, `transform: scale(0)` (starts hidden, scales to 1 via scroll)
- Background: #fff, border-radius: 1335px (pill shape)
- Flex center
- Text "view": Inter Tight, 500, 110px (desktop) / 72px (mobile), letter-spacing -0.04em, color #fff, `mix-blend-mode: exclusion`

### 1G. Video Container
- `id="main-canvas"`, `pointer-events-none`
- **Desktop**: `position: fixed, inset: 0, width: 100%, height: 100%, z-index: 0`
- **Mobile**: `position: fixed, left: 0, top: 220px, width: 100vw, height: calc(100vh - 220px), z-index: 0`
- Opacity transition: 0 -> 1 when both videos loaded (`opacity 0.3s ease`)
- `overflow: hidden`
- Contains two `<video>` elements (muted, playsInline, preload="auto"), absolutely positioned to fill container, `object-fit: cover`
- Left video starts `display: none`, right starts `display: block`


**Desktop (non-touch):**
- Videos are NOT auto-played. They are scrubbed based on cursor X position via `requestAnimationFrame`.
- Dead zone: `Math.max(30, width * 0.05)` pixels from center
- If cursor is in dead zone, keep current video at `currentTime = 0`
- If cursor moves left of dead zone: show RIGHT video, scrub it based on distance from center-left-edge to left edge
- If cursor moves right of dead zone: show LEFT video, scrub it based on distance from center+deadzone to right edge
- `activeSideRef` tracks which side was last active, only changes when cursor exceeds dead zone
- Progress calculation: `(distance from dead zone edge) / (available range)` mapped to `0...video.duration`
CRITICAL: Only update currentTime when !video.seeking -- this prevents jittery playback by waiting for the browser to finish rendering the previous seek before requesting a new one.
### 1H. Video Interaction Logic

**Mobile/Tablet (touch):**
- Videos auto-play alternately: left plays first, on `ended` event switches to right, on right `ended` switches back to left
- Respects `prefers-reduced-motion`

### 1I. White Overlay
- `id="outro-overlay"`, `position: fixed, inset: 0`, `pointer-events-none`, `z-index: 12`
- Background: #fff, opacity: 0 (controlled by scroll)

### 1J. Footer
- `id="outro-footer"`, `position: fixed`, `pointer-events-none`
- Left: 16px, bottom: 32px (desktop) / 24px (mobile)
- `mix-blend-mode: exclusion`, opacity: 0 (controlled by scroll)
- Flex row, gap: 80px (desktop) / space-between (mobile)
- Two spans: "PRMPT (R) 2026" and "PRIVACY POLICY"
- Font: Inter Tight, 500, 13px (desktop) / 11px (mobile), letter-spacing -0.02em, uppercase, white

---

### SECTION 2: Black Panel (Gallery)

### Container
- `position: fixed, inset: 0`, background: black, `z-index: 10`
- Initially translated `translateY(100vh)` (off-screen below)
- Slides up to `translateY(0)` during first 100vh of scroll via GSAP ScrollTrigger (scrub: true, ease: none)

### Inner Wrapper
- `width: 100%`, `padding-top: min(400px, 40vh)`

### Grid Layout Algorithm
- Responsive columns: 2 (< 640px), 3 (640-1024px), 4 (>= 1024px)
- Each cell has `aspect-ratio: 2/3`
- Layout function `buildLayout(count, cols)` creates rows:
  - For each row `r`, compute primary column: `a = (r * 2 + (r % 2)) % cols`
  - Place one image at column `a`
  - Every 3rd row (`r % 3 === 0`), place a second image at `b = (a + 2) % cols` (or `(a+1)%cols` if same as a)
  - Empty cells get `-1` (rendered as empty spacer divs)

### Card Behavior
- Each card has class `bp-card`, `will-change: transform`
- `transform: scale(0)` initially
- `transform-origin`: cards in left half of grid get `right bottom`, right half get `left bottom`
- Scale is computed per-frame in RAF based on card's vertical position:
  - **Enter**: `Math.min(1, (vh - top) / (vh * 0.6))` -- scales from 0 to 1 as it enters viewport
  - **Exit**: `Math.min(1, bottom / (vh * 0.4))` -- scales from 1 to 0 as it exits top
  - Final scale: `Math.min(enter, exit)`
  - If card is fully off-screen (bottom <= 0 or top >= vh): `scale(0)`

### Scroll Phases (RAF-based, NOT scroll events)
- **Phase 1** (scrollY 0 to vh): Panel slides up. Cards are computed with panelOffset = `vh - scrollY`
- **Phase 2** (scrollY > vh): Panel is fixed at top. Inner wrapper translates up: `translateY(-(scrollY - vh))`. Cards recomputed with phase2 offset.
- **Outro** (scrollY > vh + maxScroll): White overlay fades in, product info slides up by `outroOffset` px, "view" button scales from 0 to 1, footer fades in. Progress: `(scrollY - vh - maxScroll) / (vh - 100)`

### Spacer Height Calculation
- Set dynamically: `vh + maxScroll + 2 * vh` where `maxScroll = wrapScrollHeight - vh`

---

### CSS (index.css)

```css
@import "tailwindcss";

.bp-card {
  will-change: transform;
}

@media (prefers-reduced-motion: reduce) {
  .bp-card {
    will-change: auto;
  }
}
```

---

### Responsive Breakpoints
- **Mobile**: < 640px
- **Tablet**: 640px - 1024px
- **Desktop**: >= 1024px

---

### Key Design Principles
- All text overlays use `mix-blend-mode: exclusion` to remain visible against both light and dark backgrounds
- No visible scroll bar interaction -- entirely RAF-driven position tracking
- `pointer-events-none` on all overlaid UI elements
- `user-select: none` on root container
- Videos hidden (`visibility: hidden`) once scroll passes first viewport height
- Circle symbol randomizes on scroll (throttled to 80ms)
- Entry animations staggered: logo (0s), nav (0.15s), caption (0.3s), product info (0.45s)

## RIVR DeFi — Landing Page [sites/rivr-defi-landing]

- Preview: https://motionsites.ai/assets/landing-rivr-defi-preview-BPVSgEtB.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/rivr-defi-landing.gif

Build a high-performance DeFi dashboard landing page using React, Vite, Tailwind CSS v4, Framer Motion (import { motion } from 'motion/react'), and Lucide React icons. The application must match the following specification component by component, using exact styling, animations, colors, and CloudFront video URLs.
1. Global Setup (index.css & App.tsx)
CSS Setup:
Import Tailwind via @import "tailwindcss";.
Import a custom @font-face for "Helvetica Regular" using this base URL for various formats (eot, woff2, woff, ttf, svg): https://db.onlinewebfonts.com/t/a64ff11d2c24584c767f6257e880dc65
Create a theme variable --font-helvetica using ui-sans-serif, system-ui, sans-serif fallbacks.
Set :root { font-family: var(--font-helvetica); } and body { margin: 0; overflow-x: hidden; background-color: #f0f0f0; }.
App Setup:
Wrap your page components (Hero, Metrics, Features, CTA, Footer) in <main className="min-h-screen bg-[#f0f0f0]">.
2. Hero Section (Hero.tsx)
Create a full-screen wrapper: w-full h-screen flex items-center justify-center p-3 md:p-5 bg-[#f0f0f0].
Inside, a <section> container: relative w-full max-w-[1536px] h-full rounded-[1.5rem] md:rounded-[3rem] overflow-hidden flex flex-col items-center group.
Background Video: Place an absolute <video> filling the section, autoPlay muted loop playsInline. Use this URL:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260428_193507_4286c423-2fd9-4efd-92bd-91a939453fc1.mp4
Main content block: Over the video (relative, z-10), stack the Navbar, a HeroBadge, an h1 ("Fluid Asset Streams" - text color #5E6470), and a paragraph ("Access Smart Vaults, stake RIVR..."). Use subtle staggered fade-in + scale animations via Framer Motion.
3. Navbar & Floating Elements
Navbar: Hidden on desktop, mobile shows text logo "RIVR". Desktop shows centered links: "Ecosystem", "Economics" (w/ ChevronRight), "Developers", "Governance". Add a right-aligned "Book Demo" hoverable button (dark blue background, white text, inner white/20 pill with ArrowUpRight).
HeroBadge: A pill with bg-white/60 backdrop-blur-md border border-white/20, a Lucide Sparkles icon, and text "Fluid Staking".
BottomLeftCard: Absolute positioned at the bottom left. Glassmorphism card (bg-white/30 backdrop-blur-xl), containing "5.2K Active Yielders" and a "Join Discord" white pill button.
BottomRightCorner: Absolute positioned at bottom right, simulating an architectural "cut-out" merged with the container.
Background #f0f0f0, padding p-6 pt-8 pl-14, rounded top-left rounded-tl-[3.5rem].
Content: a faint circle with ArrowUpRight, and "Documentation / Library" text.
Crucial Inverted Corner SVG Trick: Include two absolutely positioned SVGs (one top, one left) measuring exactly 3.5rem to fill the gaps and make the inner curve flush: <path d="M56 56V0C56 30.9279 30.9279 56 0 56H56Z" fill="#f0f0f0"/>.
4. Metrics Section (Metrics.tsx)
Container: w-full max-w-[1536px] mx-auto px-3 md:px-5 py-6 md:py-12.
Inner box: bg-[rgba(30,50,90,0.02)] border border-[rgba(30,50,90,0.05)] rounded-[1.5rem] md:rounded-[3rem] p-8 md:p-16.
Data: A 2x4 grid separated by borders (divide-[rgba(30,50,90,0.1)]).
Items: "$2.4B" (Total Value Locked), "8.5%" (Average Realized Yield), "140K+" (Active Participants), "< 2s" (Finality Engine).
Animations: Staggered upward fade-ins using whileInView.
5. Features Section - No Background Videos Layout (Features.tsx)
Header: "Architected for high-performance DeFi", floating "Start Staking" outline button, pure white cards on #f0f0f0 background.
Grid Setup: grid grid-cols-1 md:grid-cols-3 md:grid-rows-2. All cards are white, rounded [1.5rem] md:rounded-[2rem], with hover:shadow-[0_8px_30px_rgb(0,0,0,0.04)] and overflow-hidden.
Card 1 (Tall Left): md:row-span-2 min-h-[28rem]. Title: "Unlock the liquidity of your staked assets". Bottom description. Features a massive, 2% opacity (opacity-[0.02]) background watermark of a Lucide Layers icon that scales up (group-hover:scale-110) on hover.
Card 2 (Wide Top Right): md:col-span-2. Title "Real-time Yields". Includes a scaled-up watermark of a Lucide Activity icon (opacity-[0.02]) anchored to the bottom right.
Card 3 (Bottom Right 1): Title "Bank-grade". Shows "Smart contracts audited...". Has a "View Audits" rounded outline button at the bottom.
Card 4 (Bottom Right 2): Title "Cross-Chain". Centers a gray circular button with an ArrowUpRight that triggers a scale transition on hover.
6. CTA Section (CTA.tsx)
Container layout identical to Hero (centered max-w rounded box), but uses a different background video.
Background Video: absolute inset-0 w-full h-full object-cover. URL:
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260427_104731_bfd355f7-1f84-4f81-ad88-52c2bca70bad.mp4
Content (layered above video, white text):
Headline: "Melt rigid assets into fluid yield."
Two flex-row buttons: "Launch App" (Solid white + inner icon) and "Read Docs" (bg-white/10 backdrop-blur-md).
7. Footer Section (Footer.tsx)
Simple border-top section.
Left column: The "RIVR" logo text, short description text.
Right grid: 3 columns of small, muted links ("Protocol", "Developers", "Community") transitioning to dark text on hover.

## SkyElite Private Jets — Landing Page [sites/skyelite-hero]

- Preview: https://motionsites.ai/assets/hero-skyelite-preview-DHaZIgUv.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/skyelite-hero.gif

Create a premium private jet landing page hero section with the following specifications:

Video Background:
Use this exact CloudFront video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260328_091828_e240eb17-6edc-4129-ad9d-98678e3fd238.mp4
Video should autoplay, be muted, loop continuously, and include playsInline attribute
Video covers entire viewport (100vh) using object-cover

Navigation Bar:
Brand name "SkyElite" on the left (text-2xl, font-semibold, text-gray-900)
Desktop menu items (hidden on mobile, visible md:flex): Start, Story, Rates, Benefits, FAQ
Navigation links in gray-900 with hover:text-gray-700 transition
Mobile hamburger menu button using Lucide React icons (Menu/X)
Mobile menu appears as dropdown with white/95 opacity background, backdrop blur, rounded corners, shadow
Max width 7xl, centered with px-8 py-6

Hero Content (centered, -mt-80 to pull up):
Small uppercase label: "PRIVATE JETS" (text-sm, font-semibold, gray-600, tracking-wider, mb-4)
Large two-line heading with overlapping effect:
Line 1: "Premium." (text-6xl md:text-7xl lg:text-8xl, font-normal, text-gray-500, leading-none, tracking-tighter)
Line 2: "Accessible." (same size, color: #202A36, negative margin-top: -12px for overlap)
Subtitle: "Your dedication deserves recognition." (text-lg md:text-xl, gray-600, mb-6, max-w-2xl)
Two call-to-action buttons (gap-4, centered):
"Discover" button: px-4 py-2, rounded-full, bg-gray-300, text-gray-800, font-medium, hover:bg-gray-400
"Book Now" button: px-4 py-2, rounded-full, white text, bg-color #202A36, hover color #1a2229 with smooth transitions

Typography:
Use Inter font (import from Google Fonts: 400, 500, 600, 700 weights)
Apply to entire body via CSS

Technical Setup:
React with TypeScript
Tailwind CSS for styling
Lucide React for icons
useState hook for mobile menu toggle
Full screen height container (h-screen)
Responsive breakpoints: mobile-first, md, lg
All transitions use transition-colors class

Layout Structure:
Outer container: min-h-screen, bg-gray-50
Hero section: relative, h-screen, overflow-hidden
Content wrapper: relative, h-full, flex flex-col
Main content area: flex-1, flex items-center justify-center

Make it clean, modern, and premium-looking with smooth interactions.

## Stellar Launch — Landing Page [sites/stellar-launch]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(88).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/stellar-launch.webp

Build a Launchex Awards landing page using React + Vite + Tailwind CSS + TypeScript + lucide-react. The page has 3 sections plus persistent overlay navigation elements. Use the fonts "Inter" (body) and "TT Firs Neue" (display headings). The entire page lives inside a white container with 20px padding (p-3 on mobile, p-5 on desktop) creating an inset card effect with large rounded corners (28px mobile, 36px desktop). The scrollable content lives in an absolutely-positioned div inside this container with hidden scrollbars.

---

### FONTS

Load via `<link>` in index.html:
- Google Fonts Inter: weights 300, 400, 500, 600, 700
- TT Firs Neue from: `https://db.onlinewebfonts.com/c/69f2576e7ca287875bf8d089130e292c?family=TT+Firs+Neue`

In CSS define:
```css
html, body {
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  background: #ffffff;
}
.font-firs {
  font-family: 'TT Firs Neue', 'Inter', system-ui, sans-serif;
}
.no-scrollbar {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.no-scrollbar::-webkit-scrollbar {
  display: none;
}
```

---

### COLOR PALETTE

- Primary dark: `#154359`
- Teal accent: `#066377`
- Light background: `#F0F0F0` (nominations section)
- Lighter background: `#F0F5F7` (about section)
- Gradient text: `linear-gradient(294deg, #185B7B 20%, #4BBDF0)`
- Nomination stroke: `rgba(6, 99, 119, 0.25)`

---

### OUTER SHELL STRUCTURE

```
div.h-screen.bg-white.p-3.sm:p-5
  div.relative.w-full.h-full.overflow-hidden.rounded-[28px].sm:rounded-[36px].bg-white
    div.absolute.inset-0.overflow-y-auto.overflow-x-hidden.no-scrollbar
      [SECTIONS GO HERE]
    [NAV BAR - absolute positioned]
    [BOTTOM OVERLAYS - absolute positioned]
```

---

### SECTION 1: HERO

- Full viewport height: `min-height: calc(100vh - 40px)`
- Background: autoplaying, looping, muted video filling the section with `object-cover`
  - Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260511_151648_2bdfbd1c-6bde-4f5d-a967-f57cbced97f6.mp4`
- Overlay gradient on the video: `bg-gradient-to-b from-black/10 via-transparent to-black/20`

**Top bar (z-20):** flex row, justify-between, px-4 sm:px-10, pt-5 sm:pt-8
- Left: Logo using lucide-react `Sparkles` icon (w-5 h-5 sm:w-6 sm:h-6, strokeWidth 1.5) + text "launchex" (14px sm:15px, font-semibold, tracking-tight) and "awards" below (10px sm:11px, font-light, opacity-90, -mt-0.5). All white.
- Right: CTA button "Send in your entry form" (hidden on mobile, shows "Enter" on mobile). Teal background `#066377`, white text, 10px sm:11px, uppercase, tracking-[0.14em], font-medium. Has a chamfered/clipped shape using `clipPath: polygon(10px 0, 100% 0, 100% calc(100% - 10px), calc(100% - 10px) 100%, 0 100%, 0 10px)`. Includes `ArrowUpRight` icon (w-3.5 h-3.5) that moves on hover (translate-x-0.5, -translate-y-0.5). Button has `hover:brightness-125` transition.

**Center content (z-10):** flex-col, items-center, text-center, color `#154359`, pt-32 sm:pt-40, pb-24
- Eyebrow: "Prize for ventures" - 11px sm:12px, uppercase, tracking-[0.3em], font-medium, mb-6, opacity-90
- Heading: "launchex prizes" (two lines with `<br/>`), using `.font-firs`, font-normal, tracking-[-0.04em], leading-[0.9], sizes: 48px / 76px / 100px / 120px (responsive breakpoints)
- Subtext: "Bridging visions with reality, helping ventures soar up to the stars" - 12px sm:14px, uppercase, tracking-[0.22em], font-medium, max-w-md, leading-[1.8], opacity-90, mt-8

---

### SECTION 2: SUBMISSIONS (NOMINATIONS)

- Background: `#F0F0F0`
- Padding: py-20 sm:py-28, px-6 sm:px-10
- Overflow hidden, relative positioning

**Layout:** 3-column on large (left nominations | center video | right nominations), stacked on mobile (center first, then left, then right). max-w-5xl, mx-auto, gap-10 lg:gap-12.

**Center column:**
- Header text: "[submissions]" (12px, tracking-[0.24em], uppercase) and "submissions" below (font-firs, 44px sm:54px, font-semibold, tracking-tight, uppercase). Color `#154359`.
- Video below (mt-6 sm:mt-8): 220px/380px/460px square (responsive), object-cover
  - URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260514_154120_b89bfedd-530d-4ebb-9eb7-42eeafe08667.mp4`
  - autoPlay, loop, muted, playsInline

**Left nominations (3 cards), pushed down with lg:mt-36:**
1. "Lead" / "AI venture for commerce"
2. "Emerging innovations" / "in food commerce"
3. "The finest innovations" / "for learners and young students"

**Right nominations (3 cards), pushed down with lg:mt-36:**
1. "Innovations for advanced" / "career training"
2. "The finest innovations" / "in finance"
3. "Categories" / "coming soon"

**NominationCard component:**
- `<a>` tag, max-w-[20em], h-[5em], hover:-translate-y-0.5 transition
- Contains an SVG with a chamfered rectangle (polygon points="14,0 100,0 100,86 86,100 0,100 0,14") as border - stroke `rgba(6, 99, 119, 0.25)`, strokeWidth 1, vectorEffect non-scaling-stroke, fill none, preserveAspectRatio="none", viewBox="0 0 100 100"
- Text centered inside: title in 13px font-semibold, subtitle in 12px font-normal opacity-80. Color `#154359`.

**Bottom fade gradient (pointer-events-none, absolute, bottom-0, full width, h-40 sm:h-56, z-10):**
- `linear-gradient(to bottom, rgba(240, 245, 247, 0) 0%, rgba(240, 245, 247, 0.7) 60%, #F0F5F7 100%)`

---

### SECTION 3: ABOUT THE FOUNDERS

- Background: `#F0F5F7`
- Padding: py-20 sm:py-28, px-6 sm:px-10
- max-w-7xl mx-auto

**Top row:** flex-col on mobile, flex-row on lg. Color `#154359`.
- Left: Heading "About the founders" (two lines) - font-firs, 36px/48px/54px, font-semibold, uppercase, tracking-tight, leading-[0.95]
- Right: max-w-xl column
  - Two paragraphs (17px sm:18px, leading-[1.5]):
    - "Launchex.Hub is a platform that is part of a portfolio of companies Launchex, for sourcing and showcasing groundbreaking innovations."
    - "Launchex.Hub's mission is to offer every local-language innovator the chance to reshape our world with their pioneering creation."
  - Link "Launchex.Hub website" with arrow icon (mt-6, 14px, font-medium). Arrow in a chamfered 32x32 box with border in `#154359`, clipPath `polygon(8px 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%, 0 8px)`. Hover: -translate-y-0.5. Links to `https://base.launchex.vc/`

**Stats grid (mt-14):** 1 col / 2 col md / 3 col lg, gap-5. Three cards:

Card 1: "7+ years" / "Launchex has served the market, guiding ventures and their journeys"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_154203_6c6f94dc-a07e-4ba5-8688-106f01ccd2c8.png&w=1280&q=85`
- No vertical offset
- clipPath: `polygon(64px 0, calc(100% - 14px) 0, calc(100% - 4px) 4px, 100% 14px, 100% calc(100% - 14px), calc(100% - 4px) calc(100% - 4px), calc(100% - 14px) 100%, 14px 100%, 4px calc(100% - 4px), 0 calc(100% - 14px), 0 64px)`
- Text position: left-6 right-6 bottom-6

Card 2: "15000+" / "innovation ventures moved through the Launchex pipeline"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_154151_45c62c60-3bcc-4f21-8f9d-03722ebb5df8.png&w=1280&q=85`
- Offset: lg:mt-24 (pushed down on desktop)
- clipPath: `polygon(0 14px, 4px 4px, 14px 0, calc(100% - 64px) 0, 100% 64px, 100% calc(100% - 14px), calc(100% - 4px) calc(100% - 4px), calc(100% - 14px) 100%, 64px 100%, 0 calc(100% - 64px))`
- Text position: left-6 bottom-20

Card 3: "120+" / "accelerator sessions delivered by Launchex across Eastern Europe"
- Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260514_152238_24ec8db4-d728-4739-bb30-e985533e9637.png&w=1280&q=85`
- No vertical offset
- clipPath: `polygon(0 14px, 4px 4px, 14px 0, calc(100% - 64px) 0, 100% 64px, 100% calc(100% - 64px), calc(100% - 64px) 100%, 14px 100%, 4px calc(100% - 4px), 0 calc(100% - 14px))`
- Text position: left-6 right-28 bottom-6

**Each stat card structure:**
- Outer div: w-full, h-[280px] sm:h-[340px], backgroundColor `rgba(255, 255, 255, 0.8)`, padding `1.5px` (acts as border), clipPath applied
- Inner div: w-full h-full, overflow-hidden, background-image set to the image URL, bg-cover bg-center, same clipPath applied, `mixBlendMode: 'plus-darker'`
- Text overlay (absolute positioned): value in font-firs, font-semibold, uppercase, 36px sm:52px, gradient text (`linear-gradient(294deg, #185B7B 20%, #4BBDF0)` with background-clip text, color transparent). Description in 14px, leading-[1.4], color `#154359`, mt-3. Max-width 66%.

**Bottom fade gradient (same as section 2):**
- `linear-gradient(to bottom, rgba(240, 245, 247, 0) 0%, rgba(240, 245, 247, 0.7) 60%, #F0F5F7 100%)`

---

### PERSISTENT OVERLAY ELEMENTS (inside the outer rounded container, outside the scrollable area)

**Top navigation bar:**
- Hidden on mobile (`hidden md:flex`), absolute, top-0, centered horizontally (left-1/2 -translate-x-1/2), z-40
- White background, border-bottom-left-radius and border-bottom-right-radius: 28px
- Padding: px-6 lg:px-10, py-4, gap-6 lg:gap-10
- Links: "About", "Submissions", "Venue", "Judges", "Connect" - 11px, uppercase, tracking-[0.14em], font-medium, text-neutral-800, hover:text-neutral-500
- Two decorative `<span>` elements on left (-left-6) and right (-right-6) that create inverted rounded corners using radial-gradient masks:
  - Left: `radial-gradient(circle at 0 100%, transparent 24px, black 25px)`
  - Right: `radial-gradient(circle at 100% 100%, transparent 24px, black 25px)`

**Bottom-right page indicator:**
- pointer-events-none, absolute, bottom-4 sm:bottom-6, right-4 sm:right-8, z-40
- "01" [line] "05" - flex, gap-3, text-white/80, 10px, font-medium, uppercase, tracking-[0.18em], mix-blend-difference
- Line is a span: w-8 h-px bg-white/40

**Bottom-left scroll indicator:**
- pointer-events-none, absolute, bottom-4 sm:bottom-6, left-4 sm:left-8, z-40
- "Scroll to discover" - text-white/80, 10px, font-medium, uppercase, tracking-[0.18em], mix-blend-difference

---

### KEY IMPLEMENTATION DETAILS

- All clip-paths use the `polygon()` function with pixel-based chamfers creating angular/geometric cut corners
- The page is fully responsive with sm/md/lg breakpoints
- Videos use autoPlay, loop, muted, playsInline attributes
- Use lucide-react for Sparkles and ArrowUpRight icons only
- The stat card images use `mix-blend-mode: plus-darker` for a deeper tonal effect
- No scrollbar is visible (custom CSS utility)
- All transitions are subtle: translate, color changes, brightness
- The outer container clips all content with its rounded corners - the scroll happens inside

## Synthesis — Landing Page [sites/synthesis]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(91).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/synthesis.webp

Build a premium scroll-driven landing page for "Elias Norden — Health Capital" using Vite + React + TypeScript + Tailwind CSS. Dark navy aesthetic, white text throughout.

**Fonts (load in index.html):**
- Body font: `Roobert TRIAL` via `<link href="https://db.onlinewebfonts.com/c/0ab46e1b2f236c9fad58c1e34cdecdf1?family=Roobert+TRIAL" rel="stylesheet" />`, fallback `system-ui, sans-serif`
- Accent serif: `Instrument Serif` (regular + italic) via `<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet" />`
- Tailwind config: `fontFamily: { sans: ['"Roobert TRIAL"', 'system-ui', 'sans-serif'], instrument: ['"Instrument Serif"', 'Georgia', 'serif'] }`, custom colors `navy-950: #020b1f`, `navy-900: #041536`

**Global CSS:** `html { scroll-behavior: smooth }`, body uses Roobert TRIAL, background `#020b1f`, white text, `-webkit-font-smoothing: antialiased`, `::selection { background: rgba(255,255,255,0.2) }`. Page title: "Elias Norden — Health Capital".

**Navbar (fixed, top, z-50, transparent):**
- Flex row, justify-between, padding `px-4 py-4 sm:px-6 sm:py-5 md:px-12 md:py-7`
- Left logo link: "Elias Norden" — `text-lg sm:text-xl md:text-2xl tracking-tight text-white`, where "Norden" is wrapped in `<span className="font-instrument italic">` with NO font-weight applied
- Right links: Articles, Allocations, Inquire — `text-[10px] sm:text-[11px] md:text-xs font-medium uppercase tracking-[0.14em] sm:tracking-[0.18em] text-white/80 hover:text-white transition-colors duration-300`, gaps `gap-4 sm:gap-6 md:gap-10`

**Hero section — scroll-scrubbed video (the core feature):**
- Outer `<section>` is `relative h-[700vh]`; inside it a `sticky top-0 h-screen overflow-hidden bg-navy-950 supports-[height:100svh]:h-[100svh]` viewport
- Video URL: `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260610_193933_20e7efd7-2d68-4946-a270-04cb7b9ab74b.mp4`
- On mount: fetch the video as a blob, create an object URL, then pre-extract frames into `ImageBitmap[]` by seeking through a detached `<video>` element — frame count = `clamp(round(duration * 24), 30, 110)`, frames resized to max width 1280 via `createImageBitmap(video, { resizeWidth, resizeHeight })`. Clean up bitmaps and object URL on unmount; handle cancellation
- While frames extract, render a fallback muted/playsInline `<video>` scrubbed by setting `currentTime` (guard with a `seeking` flag and `seeked` listener); once frames are ready, switch to a full-screen `<canvas>` drawn with object-cover math (`scale = max(cw/fw, ch/fh)`, centered). Canvas resizes with `devicePixelRatio` capped at 2
- Scroll logic in a single `requestAnimationFrame` loop: compute raw progress = `-section.getBoundingClientRect().top / (section.offsetHeight - window.innerHeight)` clamped 0–1, then smooth it with lerp `smoothed += (target - smoothed) * 0.1`
- Phase boundaries: `SCRUB_END = 0.55`, `FADE_END = 0.65`. Phase 1 (0–0.55): video scrubs frame-by-frame with scroll. Phase 2 (0.55–0.65): hero layer fades out while a near-black layer (`bg-[#000308]`) fades in (set opacities directly via refs, no React state). Phase 3 (0.65–1.0): three paragraphs reveal sequentially, one per third of remaining scroll
- Overlay on video: `absolute inset-0 bg-black/35`

**Hero headline (centered over video):**
- `<h1 className="max-w-5xl text-center text-[2rem] leading-[1.15] tracking-tight text-white sm:text-5xl sm:leading-[1.1] md:text-6xl lg:text-7xl">` — NO font-weight class (natural weight)
- Text: "Merging science, defi and `<br className="hidden sm:block" />` lifespans into *true wellness.*" where "true wellness." is `<span className="font-instrument italic">` with no font-weight

**Press logos strip (bottom-right of hero):**
- Label "In the news:" — `text-right text-xs sm:text-sm text-white/70`
- Right-aligned flex-wrap row, `gap-x-5 gap-y-3 sm:gap-x-8 sm:gap-y-4 md:gap-x-12`, each `text-sm sm:text-lg md:text-xl text-white/90 hover:opacity-60 transition-opacity duration-300`:
  - Praxis — `font-instrument font-bold tracking-wide`
  - VENTURE BULLETIN — `font-sans font-bold tracking-tight`
  - Blockdispatch — `font-sans font-semibold italic`
  - Healthspan.Quarterly — `font-mono font-medium tracking-tighter`
  - Vetted / TJ — `font-instrument italic tracking-wide`
  - biofuture.io — `font-sans font-light tracking-widest`
- Container padding: `px-5 pb-10 sm:px-6 sm:pb-16 md:px-12 md:pb-24`

**Navy reveal layer (fades in after scrub):**
- `absolute inset-0 bg-[#000308]`, starts `opacity-0 pointer-events-none` (pointer-events enabled when fade > 0.5), centered `max-w-4xl space-y-6 sm:space-y-10 text-center`
- Three paragraphs, each `text-lg sm:text-2xl md:text-3xl leading-relaxed md:leading-snug text-white transition-all duration-700 ease-out`, animating from `translate-y-8 opacity-0` to `translate-y-0 opacity-100` as visibleCount increments:
  1. "Elias is committed to a tomorrow where people enjoy more vibrant, rewarding decades beside loved ones."
  2. "In pursuit of this purpose, in 2021 he co-founded the Healthspan Research Alliance, a global nonprofit backing early-stage science on prolonging the healthy human lifespan."
  3. "Elias is also a managing partner and co-founder of VitalVC, a venture capital firm backing bold pioneers in biotech and lifespans."

**App structure:** `<div className="bg-navy-950"><Navbar /><Hero /></div>`. Use lucide-react for any icons. No purple hues.

## Scenic Travel — Landing Page [sites/travel-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(94).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/travel-hero.webp

Here is a complete, drop-in prompt you can hand to a fresh model to reproduce the site byte-for-byte. It captures the stack, structure, animations, every CloudFront URL, and the exact CSS quirks.

---

### Master Recreation Prompt

Build a Next.js 16.2.6 (App Router, webpack — not Turbopack) site for a luxury travel concept. Stack and conventions below are mandatory. Reproduce file paths, classNames, animation timings, and copy verbatim.

### Stack & dependencies
- `next` `16.2.6`, `react` `19.2.4`, `react-dom` `19.2.4`
- `framer-motion` `^12.38.0`
- `lucide-react` `^1.14.0`
- `gsap` `^3.15.0` (installed but unused — keep it in deps)
- Tailwind v4 (`tailwindcss` + `@tailwindcss/postcss`), TypeScript 5.9.3, ESLint 9, `eslint-config-next` 16.2.6

`package.json` scripts:
```json
"dev": "next dev --webpack",
"build": "next build --webpack",
"start": "next start",
"lint": "eslint"
```

`next.config.ts` is empty default (`{}`). No `images.remotePatterns`.

### File layout
```
app/
  layout.tsx
  page.tsx
  globals.css
  not-found.tsx
  [...catchAll]/page.tsx
  destinations/
    page.tsx
    [id]/page.tsx
  components/
    Navbar.tsx
  sections/
    HeroSection.tsx
    DestinationsSection.tsx
    TourDetailSection.tsx
  lib/
    tours.ts
public/
  img1.jpg … img10.jpg
```

### Global constants (used everywhere)
```ts
const goldEase = [0.76, 0, 0.24, 1] as const
```
Background color throughout: `#f3ebe4`. Selection: `bg-black text-white`.

### `app/layout.tsx`
- Metadata title: `"Travel — Discover the World"`
- Description: `"Escape the ordinary and find inspiration in the most breathtaking corners of the globe."`
- Loads Google Fonts Inter (weights 300, 400, 500) via `<link rel="preconnect">` + stylesheet in `<head>`
- `<body>` has `className="h-full antialiased"` and inline `style={{ fontFamily: "'Inter', sans-serif" }}`
- Renders `<Navbar />` then `{children}`. `<html lang="en" className="h-full">`.

### `app/globals.css` (exact content)
```css
@import "tailwindcss";

* { margin: 0; padding: 0; box-sizing: border-box; -webkit-font-smoothing: antialiased; }

html, body { height: 100%; background-color: #f3ebe4; font-family: 'Inter', system-ui, sans-serif; }

.destinations-page { overflow-y: auto; }

.hero-container { position: relative; width: 100vw; height: 100vh; display: flex; }
.left-bg  { width: 50vw; height: 100%; background-color: #f3ebe4; }
.right-bg { width: 50vw; height: 100%; position: relative; display: flex; justify-content: flex-end; align-items: flex-end; padding: 30px; }
.bg-image-wrapper { position: absolute; inset: 0; z-index: 0; }

.text-layer-wrapper { position: absolute; inset: 0; display: flex; flex-direction: column; justify-content: center; align-items: center; text-align: center; pointer-events: none; z-index: 20; }
.text-black-side { color: #1c1c1c; clip-path: inset(0 50% 0 0); }
.text-white-side { color: white;   clip-path: inset(0 0 0 50%); }

.gem-card { position: relative; z-index: 30; background: white; border-radius: 32px; padding: 16px; display: flex; gap: 24px; width: 100%; height: 200px; box-shadow: 0 25px 60px -15px rgba(0,0,0,0.15); }
.gem-image-box { width: 200px; height: 100%; border-radius: 20px; }
.gem-content { display: flex; flex-direction: column; justify-content: space-between; padding: 8px 0; }
#explorebtn { padding: 12px 24px; }

.footerLink { color: #000; }
.footerLink:hover { margin-left: 10px; color: #555; }

#destcontainer { padding: 30px; padding-top: 100px; }
#Popular { margin-bottom: 1rem; }
#infocard { padding: 24px; gap: 24px; }
#tourcontainer { padding: 30px; }
#bookbtn { padding: 8px; }

@media (max-width: 1000px) {
  .gem-image-box { display: none; }
  .gem-content p { font-size: 15px; }
  #destcontainer { padding: 20px; padding-top: 80px; }
}

@media (max-width: 850px) {
  #topContent { padding: 6px; gap: 10px; }
  .left-bg, .text-black-side { display: none; }
  .right-bg { width: 100vw; padding: 0; }
  .text-white-side { clip-path: none; width: 100vw; color: white; }
  .gem-card { max-width: 100%; flex-direction: column; border-radius: 40px 40px 0 0; padding: 24px; gap: 15px; position: fixed; bottom: 0; left: 0; right: 0; box-shadow: 0 -15px 50px rgba(0,0,0,0.15); }
  .gem-image-box { display: none; }
  .gem-content { width: 100%; text-align: left; }
  .gem-content p { font-size: 12px; }
  #destcontainer { padding: 10px; padding-top: 120px; }
}

@media (max-width: 500px) {
  #tourcontainer { padding: 10px; padding-top: 100px; justify-content: center; }
  #searchInput { margin-bottom: 3rem; }
}
```

### `app/components/Navbar.tsx`
Client component. Imports `useState` from react, `motion, AnimatePresence` from framer-motion, `Star, Menu, X` from lucide-react, `usePathname` from next/navigation, `Link` from next/link.

- `desktopLinks`: About `/`, Destinations `/destinations`, Booking `/booking`, FAQ `/faq`, Account `/account`
- `mobileLinks`: same, minus Account
- `pathname === '/'` → `isHome`. `pathname.startsWith('/destinations/') && pathname !== '/destinations'` → `isTourDetail`.
- **Star icon** (size 30, `fill="currentColor"`, `strokeWidth={0}`): fixed top-left at `top:30, left:30`, `z-1001`, color logic:
  - menu open → black
  - tour detail → white
  - home → `max-[850px]:text-white min-[851px]:text-black`
  - else → black
- **Hamburger** (Menu, size 32): fixed `top-7.5 right-7.5 z-300`, white on home/tour-detail else black. Hover scale-110 with 300ms ease-out.
- **Mobile overlay** (AnimatePresence): full-screen white panel slides from `y:'-100%'` to `0`, duration 0.75s, goldEase. Close X (size 32) top-right, `hover:rotate-90`. Links centered, `text-5xl md:text-7xl font-light tracking-tighter hover:italic`, each animates from `{opacity:0, y:28}` to `{0,0}` with delay `0.3 + i*0.07`, duration 0.55s. If active, prepend `<span className="mr-1">/</span>`.
- **Desktop nav** (fixed `bottom-10 left-10`, hidden by default, `min-[851px]:flex` flex-col gap-1, but only when NOT tour detail): `text-[13px] tracking-widest font-medium`, each item animates `{y:20, opacity:0}` → `{0,1}`, delay `0.4 + i*0.08`, duration 0.6s. Active link prefixed with `<span className="mr-0.5">/</span>`.

### `app/sections/HeroSection.tsx`
Client component. Imports `motion` from framer-motion, `ArrowRight` from lucide-react, `Image` from next/image, `Link` from next/link, plus `useEffect, useRef` from react.

A `HeroContent` subcomponent (used twice, mirrored via clip-path):
- Wrapper div `id="topContent"`, `flex flex-col items-center justify-center transform -translate-y-[40px] md:-translate-y-[20px] px-6`
- Two `<motion.h1>` lines wrapped in `overflow-hidden` divs:
  - `"Discover the beauty"` (initial `y:'110%'` → `y:0`, duration 1.1s, goldEase)
  - `"of the world around"` (same, but `delay: 0.08`, parent div has `mb-8`)
  - Both: `font-light leading-[1.05] tracking-[-0.04em] text-[clamp(42px,6vw,80px)]`
- A `<motion.p>` body: copy `"Escape the ordinary and find inspiration in the most breathtaking corners of the globe. We curate unique travel experiences tailored to your rhythm and spirit."`. Class `text-[clamp(14px,1vw,16px)] leading-[1.7] max-w-[550px] mx-auto opacity-80 font-light tracking-wide`. Initial `{opacity:0, y:18}` → `{1,0}`, duration 0.9s, delay 0.55s.

Default export `HeroSection`:
- `videoRef` for the hero video. `useEffect`: set `v.muted = true`, call `v.play().catch(()=>{})` immediately and on `loadeddata`. Remove listener on cleanup. (This defeats Safari/Chrome autoplay edge cases.)
- Outer div: `bg-[#f3ebe4] selection:bg-black selection:text-white min-h-screen overflow-hidden font-sans`.
- `<main className="hero-container">` with:
  1. `<div className="left-bg" />`
  2. `<div className="right-bg">`:
     - `.bg-image-wrapper` containing a `<motion.div className="relative w-full h-full">` (initial `scale:1.06` → `scale:1`, duration 2.2s, goldEase) wrapping a `<video>`:
       - `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_220929_e6719f25-1ba0-45c2-97fc-0148805d9fb9.mp4"`
       - `autoPlay loop muted playsInline preload="auto"`, `ref={videoRef}`
       - Class `absolute inset-0 w-full h-full object-cover object-left`
     - Sibling `<div className="absolute inset-0 bg-black/20 md:bg-transparent" />` (mobile darkening only)
     - **Gem card** `<motion.div className="gem-card">` (initial `{y:60, opacity:0}` → `{0,1}`, duration 1.1s, delay 0.5s):
       - `.gem-image-box relative shrink-0 overflow-hidden` containing a `<video>`:
         - `src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260509_073207_eeb9b7e5-7df4-4204-80c2-163eb46466e8.mp4"`
         - `autoPlay loop muted playsInline preload="auto"`
         - Class `absolute inset-0 w-full h-full object-cover`
       - `.gem-content gap-[20px]` with:
         - `<div className="mb-5 md:mb-0">`: `<h3 className="font-semibold text-[#1c1c1c] text-xl md:text-base mb-2">Hidden Gems</h3>` and `<p className="text-gray-500 text-xs leading-relaxed">Explore our handpicked collection of authentic stays and secluded retreats, where nature meets comfort in perfect harmony.</p>`
         - `<Link id="explorebtn" href="/destinations" className="bg-black text-white px-8 py-4 md:px-5 md:py-2.5 rounded-full text-xs flex items-center gap-2 self-start hover:bg-zinc-800 transition-all duration-300 active:scale-95 cursor-pointer">Explore more <ArrowRight size={14} /></Link>`
  3. Two text layers stacked:
     - `<div className="text-layer-wrapper text-black-side"><HeroContent /></div>`
     - `<div className="text-layer-wrapper text-white-side"><HeroContent /></div>`
     - The clip-path CSS makes the left half show black text and the right half show white — same headline, two colors, perfectly aligned.

### `app/sections/DestinationsSection.tsx`
Client component. State `query`. Filters `tours` by `name.toLowerCase().includes(query.toLowerCase())`.

- Outer: `bg-[#f3ebe4] min-h-screen font-sans selection:bg-black selection:text-white`
- `<div id="destcontainer" className="transition-all duration-500">`
  - Search row (`<motion.div>` initial `{opacity:0, y:20}` → `{1,0}`, duration 0.8s) wrapping `<input id="searchInput" placeholder="Find your tour" className="w-full max-w-2xl bg-transparent text-[clamp(24px,4vw,42px)] font-light tracking-[-0.02em] outline-none placeholder-black/20 caret-black/40 text-center" />`
  - `<motion.p id="Popular" className="text-sm font-medium tracking-widest mb-[15px]">Popular</motion.p>` (initial `opacity:0` → `0.6`, duration 0.6s, delay 0.2s)
  - `<div className="flex gap-5 overflow-x-auto pb-6 no-scrollbar">` with empty-state `<p className="text-black/40 text-sm pt-4">No tours found for "{query}"</p>` then card map. Each `<motion.div>` has `style={{ width: tour.w, flexShrink: 0 }}`, animates `{opacity:0, y:30}` → `{1,0}` with `delay: 0.1 + i*0.07`, duration 0.55s. Inside is a `<Link href={`/destinations/${tour.id}`} className="flex flex-col gap-3 group">` containing:
    - Media box `relative rounded-2xl overflow-hidden` with inline `style={{ height: tour.imgH }}`. **If `tour.video`**, render a paused `<video src={\`${tour.video}#t=0.1\`} muted playsInline preload="metadata">` with class `absolute inset-0 w-full h-full object-cover group-hover:scale-105 transition-transform duration-500 ease-out`. **Else** `<Image src={tour.image} fill alt={tour.name}>` with the same hover transform classes.
    - Caption: `<h3 className="text-sm font-medium leading-tight">{name}</h3><p className="text-sm text-black/45 mt-1">{priceDisplay} / person</p>`
- Append a `<style jsx global>` block hiding scrollbars on `.no-scrollbar`.

### `app/sections/TourDetailSection.tsx`
Client component, accepts `{ tour: Tour }`.

- Outer: `<div id="tourcontainer" className="relative min-h-screen w-full flex items-end justify-end font-sans selection:bg-black selection:text-white overflow-hidden p-4 md:p-10">`
- Background: `<div className="absolute inset-0 z-0 overflow-hidden">` containing `<motion.div className="relative w-full h-full">` (scale 1.06 → 1, duration 2.2s).
  - **If `tour.video`**: `<video src={tour.video} autoPlay loop muted playsInline preload="auto" className="absolute inset-0 w-full h-full object-cover" />`. **No overlay, no brightness filter.**
  - **Else**: `<Image src={tour.image} fill alt={tour.name} className="object-cover brightness-90" priority />` AND a sibling overlay `<div className="absolute inset-0 bg-black/10 md:bg-transparent md:bg-gradient-to-r from-black/20 to-transparent" />` (only when no video).
- Info card `<motion.div id="infocard" className="relative z-10 w-full max-w-[400px] bg-[#f3ebe4] rounded-[20px] shadow-2xl overflow-y-auto max-h-[90vh] no-scrollbar sm:gap-6 gap-2 flex flex-col">` (initial `{opacity:0, x:40}` → `{1,0}`, duration 0.8s):
  1. Header block `flex flex-col gap-[10px]`:
     - `<Link href="/destinations" className="inline-flex items-center gap-2 text-sm text-black px-4 py-2 rounded-full transition-all"><ArrowLeft size={15} />Back to explore</Link>`
     - `<h1 className="text-[20px] font-semibold text-[#1c1c1c] mb-4 tracking-tight">{name}</h1>`
  2. Description `<p className="sm:text-[15px] text-[12px] text-black/70 leading-relaxed mb-8">{description}</p>`
  3. Friends row `flex items-center gap-4 mb-10`: stacked avatar circles (3 of `tour.images`) — each `relative w-9 h-9 rounded-full overflow-hidden border-2 border-[#f3ebe4] shadow-sm`, `marginLeft: i===0 ? 0 : -12`, `zIndex: 3-i`, `<Image fill object-cover>`. Then a `+{friends-3}` chip `w-9 h-9 rounded-full bg-black text-white text-[11px] font-bold flex items-center justify-center border-2 border-[#f3ebe4]`. Trailing label `<span className="text-[13px] font-medium text-black/60">{friends} friends been there</span>`
  4. Three info rows in `space-y-4 mb-10 gap-2 flex flex-col`. Each `flex justify-between items-center pb-4 border-b border-black/10` (last row uses `pb-2` and no border):
     - `Avg cost per trip` → `priceDisplay`
     - `Best time to visit` → `bestTime`
     - `Visa` → `<span className="text-blue-600">🇪🇺</span> {visa}`. Labels: `text-[13px] text-black/40 font-medium uppercase tracking-wider`. Values: `text-sm font-bold text-black/90`.
  5. `grid grid-cols-3 gap-3 mb-8` of three thumbnails — `relative aspect-square rounded-[20px] overflow-hidden group` with `<Image fill className="object-cover transition-transform duration-500 group-hover:scale-110">`.
  6. CTA `<motion.button id="bookbtn" whileHover={{ y: -2 }} className="w-full bg-[#0f1115] text-white rounded-[24px] text-[15px] font-bold tracking-tight hover:bg-black active:scale-[0.98] transition-all duration-300">Book this tour</motion.button>`
- Append the same `<style jsx global>` no-scrollbar block.

### `app/lib/tours.ts`
```ts
export type Tour = {
  id: string
  image: string
  video?: string
  images: string[]
  name: string
  description: string
  price: number
  priceDisplay: string
  friends: number
  bestTime: string
  visa: string
  imgH: number
  w: number
}

export const tours: Tour[] = [
  {
    id: 'cold-islands-norway',
    image: '/img1.jpg',
    video: 'https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_071134_9cc2f2d8-a599-4a73-8c89-6eb4af170352.mp4',
    images: ['/img4.jpg','/img6.jpg','/img9.jpg'],
    name: 'Cold Islands Norway',
    description: "Experience the raw beauty of Norway's remote arctic islands. From dramatic fjords to the magical northern lights, Norway offers a perfect blend of wilderness and Scandinavian culture.",
    price: 1800, priceDisplay: '$1,800', friends: 8, bestTime: 'Jun - Sep', visa: 'Schengen / EU', imgH: 230, w: 200,
  },
  { id: 'serengeti-tanzania', image: '/img2.jpg', images: ['/img5.jpg','/img7.jpg','/img8.jpg'],
    name: 'Serengeti National Park, Tanzania',
    description: 'Witness the greatest wildlife spectacle on Earth. The Serengeti offers unmatched safari experiences, from the Great Migration to close encounters with the Big Five across endless golden plains.',
    price: 2400, priceDisplay: '$2,400', friends: 14, bestTime: 'Jul - Oct', visa: 'Visa on arrival', imgH: 310, w: 340 },
  { id: 'switzerland-alps', image: '/img3.jpg', images: ['/img1.jpg','/img6.jpg','/img10.jpg'],
    name: 'Switzerland',
    description: 'Experience the pinnacle of alpine serenity. From the pristine peaks of the Jungfrau region to the crystal-clear waters of Lake Brienz, Switzerland offers a perfect blend of high-end comfort and untouched nature.',
    price: 3200, priceDisplay: '$3,200', friends: 12, bestTime: 'May - Oct', visa: 'Schengen / EU', imgH: 360, w: 250 },
  { id: 'norway-coastal', image: '/img4.jpg', images: ['/img1.jpg','/img9.jpg','/img6.jpg'],
    name: 'Cold Islands Norway',
    description: "Sail through Norway's stunning coastal landscapes where turquoise waters meet towering cliffs. A journey through some of the most dramatic scenery on the planet.",
    price: 1800, priceDisplay: '$1,800', friends: 6, bestTime: 'May - Aug', visa: 'Schengen / EU', imgH: 215, w: 210 },
  { id: 'mountain-valleys-iceland', image: '/img6.jpg', images: ['/img5.jpg','/img3.jpg','/img9.jpg'],
    name: 'Mountain Valleys, Iceland',
    description: "Explore Iceland's surreal volcanic landscapes, cascading waterfalls and geothermal wonders. A land of fire and ice unlike anywhere else on Earth.",
    price: 2100, priceDisplay: '$2,100', friends: 9, bestTime: 'Jun - Aug', visa: 'Schengen / EU', imgH: 250, w: 235 },
  { id: 'hidden-coves-croatia', image: '/img7.jpg', images: ['/img8.jpg','/img2.jpg','/img5.jpg'],
    name: 'Hidden Coves, Croatia',
    description: "Discover Croatia's secluded Adriatic coastline — crystal clear waters, ancient walled cities and charming fishing villages tucked between dramatic limestone cliffs.",
    price: 1950, priceDisplay: '$1,950', friends: 11, bestTime: 'May - Sep', visa: 'Schengen / EU', imgH: 300, w: 220 },
  { id: 'desert-dunes-morocco', image: '/img8.jpg', images: ['/img2.jpg','/img7.jpg','/img10.jpg'],
    name: 'Desert Dunes, Morocco',
    description: "Journey into the Sahara's vast golden dunes, ancient medinas and vibrant souks. Morocco blends Berber, Arab and French influences into one unforgettable sensory experience.",
    price: 1600, priceDisplay: '$1,600', friends: 7, bestTime: 'Oct - Apr', visa: 'Visa free / 90 days', imgH: 240, w: 215 },
]

export function getTourById(id: string): Tour | undefined {
  return tours.find(t => t.id === id)
}
```

### Routing pages
- `app/page.tsx` → `import HeroSection from './sections/HeroSection'; export default function Home(){ return <HeroSection /> }`
- `app/destinations/page.tsx` → renders `<DestinationsSection />`
- `app/destinations/[id]/page.tsx` is `async`, awaits `params: Promise<{id:string}>`, calls `getTourById`, calls `notFound()` if missing, otherwise renders `<TourDetailSection tour={tour} />`
- `app/[...catchAll]/page.tsx` → `import { notFound } from 'next/navigation'; export default function CatchAll(){ notFound() }`

### `app/not-found.tsx`
Client component, full-screen `bg-[#f3ebe4]` flex center. Three motion blocks (all goldEase):
- Big `404`: `text-[120px] font-light leading-none tracking-[-0.04em] text-black/10 select-none`, `{opacity:0, y:30}`→`{1,0}`, duration 1s
- `Page not found`: `text-2xl font-light tracking-tight text-black mt-4 mb-3`, delay 0.15s, duration 0.8s
- `This page doesn't exist yet.`: `text-sm text-black/40 mb-10`, delay 0.25s, duration 0.7s
- `Back to home` link to `/`: `text-[13px] tracking-widest font-medium text-black border-b border-black/20 pb-0.5 hover:border-black transition-colors duration-200`, delay 0.35s

### Assets
Put any 10 photographs into `public/` named `img1.jpg` through `img10.jpg` (the gallery references all of them).

### CloudFront video URLs (use exactly these strings)
1. **Hero background video** — used in `HeroSection.tsx` only:
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_220929_e6719f25-1ba0-45c2-97fc-0148805d9fb9.mp4`
2. **Hidden Gems card video** — used inside `.gem-image-box` in `HeroSection.tsx`:
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260509_073207_eeb9b7e5-7df4-4204-80c2-163eb46466e8.mp4`
3. **First-tour video** — `tours[0].video` (Cold Islands Norway). On `/destinations` rendered paused with `#t=0.1` as a still frame. On `/destinations/cold-islands-norway` rendered as autoplaying full-bleed background, **no overlay, no brightness filter**:
   `https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260508_071134_9cc2f2d8-a599-4a73-8c89-6eb4af170352.mp4`

### Behavioral requirements (do not omit)
- All three videos: `autoPlay loop muted playsInline preload="auto"` (the destinations card uses `preload="metadata"` and no autoplay — it's a still frame via `#t=0.1`).
- Hero video focal point is left-aligned: `object-cover object-left`.
- Hero video has a defensive `useEffect` that forces `muted = true` and re-calls `play()` on `loadeddata`.
- The split-screen headline is two identical `HeroContent` components clipped via `clip-path: inset(0 50% 0 0)` (black, left half) and `clip-path: inset(0 0 0 50%)` (white, right half). Below 850px the black side is hidden and the white side fills the viewport.
- Active route in Navbar is prefixed with `/` and a small margin.
- Tour detail with a video has *no* darkening overlay and *no* `brightness-90`. Image-backed tours keep both.

## Urban Jungle — Landing Page [sites/urban-jungle-hero]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/uploaded/urbanjungleArea.mp4
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/urban-jungle-hero.mp4

Build a scroll-driven hero section landing page using React 19, Vite, Tailwind CSS v4 (using @tailwindcss/vite plugin), GSAP (with ScrollTrigger + ScrollToPlugin), hls.js, and react-router-dom (BrowserRouter). The page body is black with white text. The root container is 500vh tall.

SETUP
Dependencies (package.json):

react, react-dom, react-router-dom, gsap, hls.js, lucide-react, motion, tailwindcss v4, @tailwindcss/vite, @vitejs/plugin-react, vite
Vite config: Use @tailwindcss/vite and @vitejs/plugin-react plugins.

Entry point (main.tsx): Wrap <App /> in <StrictMode> and <BrowserRouter>.

Custom headline font: Download the font file from https://dirtylinestudio.com/wp-content/uploads/2022/05/Dirtyline-36daysoftype-2022.woff2 and save it to the public/ directory as Dirtyline-36daysoftype-2022.woff2. Then register it via @font-face in CSS.

Google Fonts (loaded via CSS @import): Manrope:wght@400;500;600;700 and Instrument+Serif:ital@0;1

Tailwind v4 theme (index.css):

@import url('https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700&family=Instrument+Serif:ital@0;1&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Manrope", ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";
  --font-serif: "Instrument Serif", ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
  --font-dirtyline: "Dirtyline36Daysoftype2022", sans-serif;
  --animate-marquee: marquee 20s linear infinite;
  @keyframes marquee {
    100% { transform: translateX(-50%); }
  }
}

@font-face {
  font-family: 'Dirtyline36Daysoftype2022';
  src: url('/Dirtyline-36daysoftype-2022.woff2') format('woff2');
  font-style: normal;
  font-weight: normal;
  text-rendering: optimizeLegibility;
  font-display: swap;
}

body {
  background-color: black;
  color: white;
}

LAYER 1: BACKGROUND -- ScrollVideo Component
A full-screen fixed video background that scrubs its playback position based on scroll progress (scroll at top = frame 0, scroll at bottom = last frame).

Video source :
https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260709_080129_da34b00e-a5db-47dd-81a1-cccad79ac1ac.mp4

Props: src (string), className (string)

Implementation:

Use hls.js. On MANIFEST_PARSED, force the highest quality level: hls.currentLevel = maxLevel; hls.startLevel = maxLevel. Config: maxBufferLength: 120, maxMaxBufferLength: 600, maxBufferSize: 200 * 1024 * 1024, startPosition: 0, capLevelToPlayerSize: false, startLevel: -1, autoStartLoad: true.
For Safari (native HLS), set video.src = src directly.
Track buffer progress via FRAG_BUFFERED event, calculating (bufferedEnd / duration) * 100.
The <video> element is rendered directly (no canvas). Classes: w-full h-full object-cover scale-[1.35]. Attributes: muted, playsInline, crossOrigin="anonymous".
Scroll-to-seek: Use GSAP ScrollTrigger.create with trigger: document.documentElement, start: 'top top', end: 'bottom bottom', scrub: true. On onUpdate, calculate targetTime = self.progress * duration. Throttle seeking: track a currentTarget variable. If video.seeking is true, set seekPending = true. On the seeked event, if seekPending, call doSeek() again with the latest currentTarget. This prevents hammering the decoder.
if (!video.seeking).
Тепер ми кажемо браузеру: "Оновлюй кадр відео ТІЛЬКИ тоді, коли ти повністю закінчив малювати попередній"."
Mouse parallax on video wrapper: On mousemove, GSAP tweens the wrapper's x/y by moveX * -30 and moveY * -30, where moveX/moveY are normalized mouse offset from center (-1 to 1). Duration: 1.5, ease: power2.out.
Loading overlay: Show a fixed, z-50, centered black overlay with "Loading... {progress}%" in white, text-2xl font-sans. Hide once canplay fires.
Wrapper div classes: fixed top-0 left-0 w-full h-full z-0 scale-[1.05] origin-center

LAYER 2: HERO TEXT -- ScrollFloat Component
A fixed overlay at z-10, positioned at the bottom of the viewport: fixed inset-0 flex flex-col justify-end p-4 md:p-8 pointer-events-none.

Text content: "Unleash The\nFull Power" (literal newline between the two lines).

ScrollFloat component implementation:

Splits the text string by \n into lines, then by spaces into words, then into individual characters.
Each line is wrapped in <span style="display: block">.
Each word is wrapped in <span style="display: inline-block; white-space: nowrap">.
Each character is wrapped in <span class="char">.
Word separators: &nbsp; between words.
Animation: Uses gsap.fromTo on all .char elements. FROM: {opacity: 1, yPercent: 0, scaleY: 1, scaleX: 1, transformOrigin: '50% 0%'}. TO: {opacity: 0, yPercent: 250, scaleY: 1.2, scaleX: 0.9}. So the text starts fully visible and animates away as you scroll down.
ScrollTrigger config: trigger: document.body, start: 'top top', end: '+=1000', scrub: 1.5.
Stagger: 0.05, ease: power2.inOut, duration: 1.
Typography: Font family: font-dirtyline (the Dirtyline custom font). Font size: clamp(4rem, 15vw, 317px). Line height: 0.85. Letter spacing: 0%. Color: white.

ScrollFloat.css:
.scroll-float-text { display: inline-block; }
.char { display: inline-block; }

LAYER 3: GLASS PANEL -- GlassPanel Component (About Us section)
Positioned absolutely at the bottom of the 500vh container: absolute bottom-0 left-0 w-full h-screen. It slides up from below as you scroll to the bottom.

Slide-up animation: gsap.fromTo on the panel wrapper: from {y: '100%'} to {y: '0%'}, ease: none. ScrollTrigger: trigger is the container div, start: 'top bottom', end: 'bottom bottom', scrub: 1.5.

Panel wrapper: w-full max-w-[1250px] h-[900px] max-h-[85vh] pointer-events-auto with perspective: 1000px inline style.

Panel itself: w-full h-full flex flex-col justify-between rounded-3xl relative overflow-hidden with inline styles:
backgroundColor: 'rgba(0, 0, 0, 0.16)'
backdropFilter: 'blur(160px)'
WebkitBackdropFilter: 'blur(160px)'
border: '1px solid rgba(255, 255, 255, 0.1)'
transformStyle: 'preserve-3d'
willChange: 'transform'

3D mouse parallax on panel: On mousemove, GSAP tweens: x: moveX * 20, y: moveY * 20, rotationY: moveX * 4, rotationX: -moveY * 4. Ease: power3.out, duration: 1.

Content (all centered text):
Subtitle: <p> with font-serif italic text-white/70 text-base md:text-lg mb-4 md:mb-6 -- text: "About Us"
Main heading: <h2> with font-serif text-white text-4xl md:text-6xl lg:text-[96px] leading-[1.1] lg:leading-[92.6px] tracking-tight w-full max-w-[1000px] mx-auto -- text: "We transform sterile concrete into thriving urban jungles. Our innovative designs bring wild nature back to modern cities. Experience the bloom" where the italic words (urban, nature, bloom) are wrapped in <span className="italic">.
All text is centered: the content area uses flex flex-col items-center justify-center px-6 md:px-12 text-center.

Bottom marquee (text-based logos, not images):
Instead of image logos, use text brand names as the marquee items. Use names like "VOICEFLOW", "ZENDESK", "PENDO", "GLIDE", "CANVA". Each name is rendered as white text, opacity-40 hover:opacity-100 transition-opacity duration-300, uppercase, font-sans font-semibold text-sm tracking-widest. The marquee row is duplicated 4x for seamless infinite scroll, using the CSS animate-marquee keyframe (translateX(-50%) over 20s linear infinite). The marquee sits at the bottom of the glass panel, separated by a border-t border-white/10 py-6.

LAYER 4: PILL NAVIGATION -- PillNav Component
Fixed at top center of viewport (position: fixed; top: 24px; left: 50%; transform: translateX(-50%); z-index: 100). Font: Manrope, 600 weight, 14px, uppercase, 0.05em letter-spacing.

Structure:
A circular black logo button (48x48px, border-radius: 50%) containing a 4-petal SVG icon (white fill, 24x24). The SVG paths:
m50,50c0,18.2,14.77,32.98,32.97,32.98,0-18.2-14.77-32.98-32.97-32.98Z
m17.02,82.98c18.2,0,32.98-14.77,32.98-32.98-18.2,0-32.98,14.77-32.98,32.98Z
m82.98,17.02c-18.2,0-32.97,14.77-32.97,32.97,18.2,0,32.97-14.77,32.97-32.97Z
m17.02,17.02c0,18.2,14.77,32.97,32.98,32.97,0-18.2-14.77-32.97-32.98-32.97Z
viewBox: 0 0 100 100. On hover, the SVG container rotates 360deg via GSAP (duration: 0.2).

Nav items container: black background, border-radius: 50px, padding: 4px, border: 2px solid #000. Contains a <ul> with flex layout, gap: 4px.

Each nav pill: padding: 8px 24px, border-radius: 50px, background-color: #f0f0f0, color: #000, font-weight: 600, font-size: 14px, letter-spacing: 0.05em, text-transform: uppercase, overflow: hidden, position: relative.

Pill hover effect (GSAP-powered liquid fill):
Each pill contains a hidden .hover-circle element (absolute, black, border-radius: 50%, scale: 0).
The circle's size is calculated dynamically: R = (w*w/4 + h*h) / (2*h), D = 2*R + 2, positioned at bottom: -delta where delta = R - sqrt(R*R - w*w/4) + 1. Transform origin: 50% ${D - delta}px.
A .label-stack contains two labels: .pill-label (dark text, visible) and .pill-label-hover (white text, hidden below).
On hover enter: a GSAP timeline plays forward -- circle scales to 3, pill-label slides up out of view, pill-label-hover slides up into view (white text over black circle). Timeline tweened to end in 0.3s.
On hover leave: timeline tweened back to 0 in 0.2s.

Nav items: HOME, ABOUT, SERVICES, CONTACT.
HOME onClick: gsap.to(window, { duration: 3, scrollTo: 0, ease: 'power3.inOut' })
ABOUT onClick: gsap.to(window, { duration: 3, scrollTo: document.body.scrollHeight, ease: 'power3.inOut' })

Initial load animation: Logo scales from 0 to 1 (duration 0.6). Nav items container width animates from 0 to auto (duration 0.6).

Responsive: At 768px breakpoint, desktop nav items are hidden and replaced with a hamburger button (two 24x2px lines, gap 4px). On toggle, lines animate to X shape (rotation +/-45deg, y +/-3px). A popover menu appears below with fade+slide animation.

PillNav.css (full):
.pill-nav-container { position: fixed; top: 24px; left: 50%; transform: translateX(-50%); z-index: 100; font-family: 'Manrope', sans-serif; }
.pill-nav { display: flex; align-items: center; background-color: transparent; padding: 0; gap: 0; }
.pill-logo { display: flex; align-items: center; justify-content: center; border-radius: 50%; background-color: #000; width: 48px; height: 48px; flex-shrink: 0; }
.logo-svg-container { display: flex; align-items: center; justify-content: center; }
.pill-nav-items { background-color: #000; border-radius: 50px; padding: 4px; border: 2px solid #000; }
.pill-list { display: flex; align-items: center; gap: 4px; list-style: none; margin: 0; padding: 0; }
.pill { position: relative; display: block; padding: 8px 24px; border-radius: 50px; text-decoration: none; color: #000; font-weight: 600; font-size: 14px; letter-spacing: 0.05em; text-transform: uppercase; overflow: hidden; background-color: #f0f0f0; transition: background-color 0.3s ease; }
.pill.is-active { background-color: #e0e0e0; }
.hover-circle { position: absolute; background-color: #000; border-radius: 50%; pointer-events: none; z-index: 0; transform: scale(0); }
.label-stack { position: relative; display: block; z-index: 1; overflow: hidden; height: 1.2em; }
.pill-label, .pill-label-hover { display: block; line-height: 1.2em; text-align: center; }
.pill-label-hover { position: absolute; top: 0; left: 0; width: 100%; color: #fff; }
.mobile-menu-button { background: none; border: none; cursor: pointer; display: flex; flex-direction: column; gap: 4px; padding: 8px; }
.hamburger-line { width: 24px; height: 2px; background-color: var(--pill-text); display: block; }
.mobile-menu-popover { position: absolute; top: 100%; left: 0; right: 0; margin-top: 8px; background-color: var(--pill-bg); border-radius: 16px; padding: 16px; visibility: hidden; }
.mobile-menu-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
.mobile-menu-link { color: var(--pill-text); text-decoration: none; font-size: 1.1rem; display: block; text-align: center; }
@media (min-width: 769px) { .mobile-only { display: none !important; } }
@media (max-width: 768px) { .desktop-only { display: none !important; } }

APP COMPONENT ASSEMBLY
<ScrollVideo src="https://stream.mux.com/43NlHXsaMrmyzWamMk87m01fNyxSTekAD669BBAPBNm00.m3u8" />
<PillNav />
<div style={{ position: "relative", height: "500vh" }}>
  <ScrollFloat>{`Unleash The\nFull Power`}</ScrollFloat>
  <GlassPanel />
</div>

## Veloce Finance — Landing Page [sites/veloce-finance-landing]

- Preview: https://motionsites.ai/assets/hero-veloce-finance-preview-DQW35gIt.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/veloce-finance-landing.gif

Build a React + TypeScript + Vite landing page for a fintech app called "veloce" using Tailwind CSS and Framer Motion. The page has 4 sections. Do not use purple/indigo colors — the gradient used is a specific brand gradient defined below.

Dependencies:

framer-motion
lucide-react
@supabase/Bolt Database-js (not used in UI but installed)
Fonts (Tailwind config + CSS):

In tailwind.config.js, extend fontFamily with:


'manrope': ['Manrope', 'sans-serif'],
'helvetica': ['Helvetica', 'Arial', 'sans-serif'],
'helvetica-neue': ['Helvetica Neue', 'Helvetica', 'Arial', 'sans-serif'],
'inter': ['Inter', 'sans-serif'],
'product-sans': ['Product Sans', 'sans-serif'],
'sf-compact': ['SF Compact Display', 'SF Compact Text', 'system-ui', 'sans-serif'],
In tailwind.config.js, extend colors with:


'sintra-dark': '#00041F',
'sintra-accent': '#B56939',
'sintra-light': '#EFF4FF',
'sintra-gray': '#49484F',
In index.css, import from Google Fonts:


@import url('https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700;800&display=swap');
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
Also add globally:


@layer base {
  * {
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
}
Brand gradient: from-[#B56939] via-[#5C3779] to-[#454BBB] (left to right)

src/components/BlurIn.tsx

A reusable wrapper component using Framer Motion's useInView. When the element enters the viewport (once), animate from filter: blur(20px), opacity: 0 to filter: blur(0px), opacity: 1 over 1.2s. Export as named export BlurIn.

src/App.tsx

The root layout is a min-h-screen div. The first child is a full-screen h-screen flex flex-col relative div that contains:

A background <video> tag (absolute, covers full area, object-cover, on mobile positioned at object-[10%_center], on desktop object-center, z-index 0, autoPlay loop muted playsInline) with this exact src:

https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_154629_a31a2372-bd54-4f7e-ac9b-21246141a664.mp4
<Header /> component
<Hero /> component
Below the hero screen, render <InsightsSection /> then <TextFillSection />.

src/components/Header.tsx

A sticky top header with justify-between items-center px-6 md:px-12 lg:px-15 py-6 z-20:

Left side: Logo text "veloce" in font-manrope font-semibold text-2xl md:text-3xl text-[#00041F], followed on desktop (lg:) by a nav with links: "Home", "About us", "Faq" — font-helvetica text-base text-[#00041F] hover:opacity-70 transition-opacity.

Right side (desktop only, lg:): "Log in" button (font-inter font-medium text-base text-[#00041F] hover:opacity-70), and a "Sign up" button (bg-[#00041F] text-[#EFF4FF] px-6 py-2.5 rounded-[40px] font-inter font-medium text-base hover:opacity-90).

Mobile hamburger: Show a Menu / X icon from lucide-react. Toggle between them with AnimatePresence + motion.div, using rotate + opacity animation (duration: 0.2). When open, show a dropdown (absolute top-24 left-0 right-0 bg-white shadow-lg mx-4 rounded-lg px-6 py-8 z-50) with staggered motion.a nav links (slide in from x: -20, stagger 0.1s) and "Log in" / "Sign up" buttons below a border separator. The mobile menu animates in with opacity: 0, y: -20 → opacity: 1, y: 0 over 0.3s easeOut.

src/components/Hero.tsx

A flex-1 flex flex-col items-center justify-between px-6 md:px-12 pb-12 md:pb-16 relative div.

At the bottom, apply a fade-out overlay: absolute inset-x-0 bottom-0 h-32 bg-gradient-to-t from-white to-transparent z-20 pointer-events-none.

Center: Wrap an <h1> in <BlurIn>. The h1 has text-center font-helvetica-neue font-medium leading-tight text-[#010828] max-w-2xl. It has two inline spans:

"Fast payments, your way at " — text-4xl md:text-6xl lg:text-7xl tracking-[-0.03em]
"lightspeed." — same sizes, but colored with the brand gradient via bg-gradient-to-r from-[#B56939] via-[#5C3779] to-[#454BBB] bg-clip-text text-transparent tracking-[-0.03em]
Bottom: A subtitle "Handle finances with ease and power" in text-[#49484F] text-base md:text-lg font-helvetica-neue, then two side-by-side store badge buttons:

Google Play badge: Border button px-3 py-2 rounded-lg border border-[#00041F] hover:bg-gray-50. Contains an <img> (w-6 h-7 object-contain) with src:

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_154325_6e98dcdb-51ba-446a-8c52-2d2f2675a575.png&w=1280&q=85
Next to it: "GET IT ON" in text-[10px] font-product-sans uppercase text-[#00041F] and "Google Play" in text-sm font-product-sans font-bold text-[#00041F].
App Store badge: Same border style. Image src:

https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260405_154356_13de5dda-6dfe-4f54-b3e2-251301254578.png&w=1280&q=85
Text: "Download on the" in text-[10px] font-sf-compact text-[#00041F] and "App Store" in text-sm font-sf-compact font-medium text-[#00041F].
src/components/InsightsSection.tsx

A white section px-6 md:px-12 lg:px-[60px] py-20 bg-white flex flex-col gap-[90px].

Top block (max-w-[517px] flex flex-col gap-10):

Heading wrapped in <BlurIn>: "Instant payment clarity counts" — text-[#00041F] text-4xl md:text-5xl lg:text-6xl font-helvetica-neue font-medium leading-[1] lg:leading-[60px] tracking-[-0.03em]
Paragraph: "Real-time data powers smarter spending choices every day" — text-[#49484F] text-base md:text-lg lg:text-xl font-helvetica-neue max-w-[361px]
Cards row (flex flex-col lg:flex-row items-stretch lg:items-end gap-5), animated with Framer Motion whileInView (once, amount: 0.2), stagger children 0.2s, each card animates from opacity: 0, y: 30 to visible over 0.6s easeOut:

Three cards, each flex-1 p-10 rounded-[40px] relative overflow-hidden flex flex-col justify-end:

Card 1 — min-h-[450px]. Video src:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_143605_bc7bd6c0-9c68-49ff-a9d3-073a10759fa4.mp4
Overlay: bg-[rgba(206,223,235,0.25)]. Stat: "1.6M", description: "Active members rely on us for effortless payment experiences" (max-w-[377px])

Card 2 — min-h-[350px]. Video src:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_145119_f4ec4d9f-3ecd-4116-baa3-26e8cf2df976.mp4
Overlay: bg-[rgba(247,236,233,0.6)]. Stat: "850К", description: "Transfers completed each day, quick and protected" (max-w-[351px])

Card 3 — min-h-[450px]. Video src:


https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260405_140728_ae719193-f10b-4105-82fc-c989610b3aa6.mp4
Overlay: bg-[rgba(218,218,218,0.2)]. Stat: "120+", description: "Nations enabled for instant checkouts and worldwide remittance" (max-w-[351px])

All three cards: stat number is text-5xl md:text-[60px] font-helvetica-neue font-medium leading-[1] md:leading-[60px] text-[#00041F]. Description is text-lg md:text-[22px] font-helvetica-neue opacity-80 text-[#49484F]. Content wrapper is relative z-10 max-w-[388px] flex flex-col gap-5. Video is absolute inset-0 w-full h-full object-cover. Overlay div is absolute inset-0.

src/components/TextFillSection.tsx

A scroll-driven text color fill animation. White background section: flex justify-center items-center px-6 md:px-16 py-24 md:py-32 bg-white mb-[30vh].

Inner wrapper: max-w-2xl w-full text-center relative.

An <h2> with text-4xl md:text-5xl lg:text-6xl font-helvetica-neue font-medium leading-tight relative tracking-[-0.03em] and two absolutely stacked spans with identical text: "Handle payments fast & sleek! Track expenses, reach targets, unlock insights to make sharper decisions, all in one app".

Bottom span (base layer): block text-[#B8B7BA] (light gray, always visible)
Top span (overlay): absolute inset-0 with the brand gradient (bg-gradient-to-r from-[#B56939] via-[#5C3779] to-[#454BBB] bg-clip-text text-transparent). Use inline style clipPath: inset(0 ${100 - fillPercentage}% 0 0) with transition: clip-path 0.1s linear to reveal from left to right.
Scroll logic (via useEffect + useRef on the section div): On scroll, get the element's getBoundingClientRect().top. Define startFill = windowHeight * 0.8 and endFill = windowHeight * 0.2. When elementTop is between endFill and startFill, compute fillPercentage = ((startFill - elementTop) / (startFill - endFill)) * 100, clamped 0–100. Below startFill → 0%, above endFill → 100%.

## Vitara — Landing Page [sites/vitara-hero]

- Preview: https://motionsites.ai/assets/hero-vitara-preview-Cjz2QYyU.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vitara-hero.gif

Create a modern healthcare AI landing page with a full-screen video background hero section with the following exact specifications:

VIDEO BACKGROUND:

Use this exact CloudFront video URL: https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260402_134434_5de46cb4-38e7-42a6-a8bc-6e62b2fd6c7b.mp4
Video should autoplay, loop, be muted, and play inline
Position: absolute, full width/height, object-fit: cover with object-position: bottom
Add a 32px high gradient overlay at the bottom that fades from transparent to #2B3534
FONTS:

Import Inria Serif (weights: 300, 400, 700) from Google Fonts in the HTML head
Body text: 'Helvetica Neue', Helvetica, Arial, sans-serif
All headings: 'Inria Serif' serif with letter-spacing: -0.07em
NAVIGATION BAR:

White background with padding
Logo: "Vitara" in 2xl font, semibold, gray-900
Menu items (desktop only, hidden on mobile): Home, Services, Team, Membership, Resources in gray-600 with hover:gray-900
Right side: "Login" text button and "Sign up" button (gray-800 bg, white text, rounded-lg)
HERO CONTENT:

Main heading (4xl on mobile, 6xl on tablet, 7xl on desktop): "Smart Care Begins with Data + Insight"
Subheading (lg to xl): "Turn medical insights into personalized wellness plans."
Both text elements use a custom AnimatedText component that:
Splits text into words
Each word animates with fadeUp animation (opacity 0 to 1, translateY 20px to 0, 0.6s ease-out)
Staggered delay of 0.1s per word
Animation fill mode: forwards
INTERACTIVE INPUT BOX:

Max width 36rem, centered
Background color: #2B3534
Rounded 2xl corners with 2xl shadow
Contains:
Textarea with placeholder: "Welcome to Vitara — your care intelligence hub!"
Transparent background, white text, gray-400 placeholder, min-height: 60px
Three action buttons with Lucide React icons:
"Start Wellness Check" (Zap icon)
"Chat with MedAI" (Stethoscope icon)
"View Insights" (BarChart3 icon)
Buttons: white border (15% opacity), rounded-full, hover effect (white bg 10% opacity)
Horizontal scroll with hidden scrollbar and right-side fade gradient
Send button: white background, gray-800 text, rounded-lg, with Send icon from Lucide React
SECOND SECTION:

Full-width background: #2B3534
Two-column grid (single column on mobile)
Left: Large heading "Your proactive shield against disease" (3xl to 6xl, white, Inria Serif font)
Right: Body text "We blend smart technology & clinical wisdom to provide tailored, preventive, & insight-rich medicine for tomorrow." (gray-300, max-width sm, right-aligned)
Both use the same AnimatedText component with word-by-word fade-up animation
CSS UTILITIES NEEDED:


.scrollbar-hide::-webkit-scrollbar { display: none; }
.scrollbar-hide { -ms-overflow-style: none; scrollbar-width: none; }

@keyframes fadeUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

.animate-fadeUp { animation: fadeUp 0.6s ease-out; }
TECH STACK:

React with TypeScript
Tailwind CSS
Lucide React for icons (Send, Zap, Stethoscope, BarChart3)
Vite build tool
RESPONSIVE DESIGN:

Mobile-first approach
Navigation menu hidden on mobile (md:flex)
Font sizes scale from mobile (text-4xl) to desktop (text-7xl)
Padding adjusts: px-6 on mobile, px-12 on tablet, px-20 on desktop
Grid becomes single column on mobile
KEY COLORS:

Primary dark: #2B3534
Text dark: gray-900
Text medium: gray-600
Text light on dark: white and gray-300
Button primary: gray-800
This creates a premium, sophisticated healthcare AI landing page with smooth animations, a cinematic video background, and clean typography using Inria Serif for headlines.

## AI Designer Portfolio — Landing Page [sites/vortex-studio-hero]

- Preview: https://motionsites.ai/assets/hero-vortex-studio-preview-BQyvwopD.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/vortex-studio-hero.gif

Create a single-page landing page for a creative design studio called "Viktor Oddy" using React, TypeScript, Vite, and Tailwind CSS. Use lucide-react for icons. The page has a white background throughout and uses two custom fonts: "PP Neue Montreal" (body text, loaded from Webflow CDN) and "PP Mondwest" (serif accent font, loaded from a local /PPMondwest-Regular.woff2 file). The body default font is PP Neue Montreal with system fallbacks.

The page consists of these sections in order:

1. HERO SECTION (centered, narrow column max-w-[440px], px-6, pt-12 md:pt-16)

Logo text: "Viktor Oddy" in PP Mondwest serif font, text-[32px] md:text-[40px] lg:text-[44px], font-semibold, color #051A24, tracking-tight, mb-4. Fades in with staggered animation (delay 0.1s).
Tagline: "The creative studio of Viktor Oddy" in monospace font (font-mono), text-xs md:text-sm, color #051A24, mb-2. Animation delay 0.2s.
Main Heading: Two lines: "Build the next wave," and "the bold way." where "next wave" and "bold way." are in PP Mondwest serif. Text is text-[32px] md:text-[40px] lg:text-[44px], leading-[1.1], color #0D212C, tracking-tight, whitespace-nowrap. Animation delay 0.3s.
Description: Three paragraphs in a flex-col gap-6 container, text-sm md:text-base, color #051A24, leading-relaxed, mt-5 md:mt-6. Animation delay 0.4s.
Paragraph 1: "I spent seven years at Apple crafting products used by over a billion people. I founded Vortex Studio to bring that same level of thinking to innovators shaping what comes next."
Paragraph 2: "The studio is deliberately small. I guide the creative vision on every project, backed by a veteran design crew that moves fast without cutting corners."
Paragraph 3: "Projects start at $5,000 per month."
Two buttons in flex-col sm:flex-row, gap-3 md:gap-4, mt-5 md:mt-6. Animation delay 0.5s:
"Start a chat" (primary: bg-[#051A24], text white, rounded-full, px-7 py-3, with a complex multi-layered box-shadow including an inset highlight)
"View projects" (secondary: bg-white, text #051A24, no border, with subtle shadow)
2. INFINITE MARQUEE (full width, mt-16 md:mt-20, mb-16)

Horizontally scrolling image strip. Uses 8 GIF images duplicated (total 16) in a flex row with animate-marquee CSS animation (translateX(0) to translateX(-50%), 30s linear infinite on desktop, 10s on mobile). Images are h-[280px] md:h-[500px], object-cover, mx-3, rounded-2xl, shadow-lg.

Image URLs (all from motionsites.ai):

https://motionsites.ai/assets/hero-space-voyage-preview-eECLH3Yc.gif
https://motionsites.ai/assets/hero-portfolio-cosmic-preview-BpvWJ3Nc.gif
https://motionsites.ai/assets/hero-velorah-preview-CJNTtbpd.gif
https://motionsites.ai/assets/hero-asme-preview-B_nGDnTP.gif
https://motionsites.ai/assets/hero-transform-data-preview-Cx5OU29N.gif
https://motionsites.ai/assets/hero-aethera-preview-DknSlcTa.gif
https://motionsites.ai/assets/hero-orbit-web3-preview-BXt4OttD.gif
https://motionsites.ai/assets/hero-nexora-preview-cx5HmUgo.gif
3. TESTIMONIAL QUOTE SECTION (py-12, px-6, max-w-2xl, centered)

A quote icon (lucide-react Quote, w-6 h-6, text-slate-900). Animation delay 0.1s.
Large quote text: 'I left Apple to build the studio I always wanted to work with' where "Apple" is in PP Mondwest serif. Text sizing: text-[32px] md:text-[40px] lg:text-[44px], leading-[1.1], color #0D212C, tracking-tight. Animation delay 0.2s.
Author: "Viktor Oddy" in italic, text-sm, color #273C46. Animation delay 0.3s.
Three company logo names displayed as text: "Apple" (80px wide, 24px font), "IDEO" (83px wide, 24px font), "Polygon" (110px wide, 24px font). Font-medium, text-slate-900. Animation delay 0.4s.
Below logos: A parallax image (scrolls with a parallax effect based on viewport position, max offset 200px). The image URL is: https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260330_103804_7aa5494f-4d5b-432e-9dc7-20715275f143.png&w=1280&q=85. Alt text "Chris Halaska". w-full max-w-xs rounded-2xl shadow-lg. Animation delay 0.5s. The parallax uses IntersectionObserver + scroll listener with requestAnimationFrame.
4. PRICING SECTION (full width, py-12, px-6)

Two cards in a grid (grid-cols-1 md:grid-cols-2, gap-8), aligned right on desktop (md:justify-end, md:max-w-4xl). Each card has rounded-[40px], pl-10 pr-10 md:pr-24 pt-3 pb-10.

Card 1 (Dark): bg-[#051A24], inset shadow. Text color #F6FCFF / #E0EBF0. Animation delay 0.1s.

Title: "Monthly Partnership" (text-[22px], font-medium)
Description: "A dedicated creative design team. / You work directly with Viktor."
Price: "$5,000" (text-2xl, color #F6FCFF), "Monthly" below
Two buttons: "Start a chat" (primary) + "How it works" (secondary), both linking to https://halaskastudio.com/./book
Card 2 (Light): bg-white, shadow-[0_4px_16px_rgba(0,0,0,0.08)]. Animation delay 0.2s.

Title: "Custom Project" (text-[22px], font-medium)
Description: "Fixed scope, fixed timeline. / Same team, same standards."
Price: "$5,000" (text-2xl, color #0D212C), "Minimum" below
One button: "Start a chat" (tertiary variant: white bg with combined shadow)
5. TESTIMONIAL CAROUSEL (full width, py-20)

Header row (md:max-w-4xl, md:ml-auto): Title "What builders say" (where "builders" is in PP Mondwest serif, same large heading size) on left. On the right: 5 filled black star icons (lucide-react Star, w-5 h-5, fill-black) + "Clutch 5/5" text.
Auto-scrolling carousel (3s interval, pauses on hover) with prev/next circular buttons (w-12 h-12 rounded-full, border border-[#0D212C]/20, lucide ChevronLeft/ChevronRight).
Cards are 427.5px wide on desktop (full width minus 48px on mobile), gap-6, with exit animation (opacity fade + scale down). Each card: bg-white, rounded-[32px] md:rounded-[40px], shadow-[0_4px_16px_rgba(0,0,0,0.08)], px-6 md:pl-10 md:pr-24 py-8.
Card content: SVG quote mark icon (custom path), quote text (text-base, color #0D212C, leading-relaxed), author row with circular avatar (w-12 h-12), name (font-semibold, text-sm), role/company with arrow prefix.
Testimonials array uses Pexels avatar images. The testimonials are tripled for infinite scroll effect. Transform uses cubic-bezier(0.4, 0, 0.2, 1) with 0.8s transition.
5 testimonials:

Marcus Anderson, CEO, Data.storage - "With very little guidance team delivered designs that were consistently spot on..."
alexwu, Founder, Nexgate - "Viktor led the creation of our best fundraising deck to date!..."
James Mitchell, VP Product, LaunchPad - "Working with Viktor transformed our product vision..."
Rachel Foster, Co-founder, Nexus Labs - "The design quality exceeded our expectations..."
David Zhang, Head of Design, Paradigm Labs - "Incredible work from start to finish..."
6. PROJECTS SECTION (max-w-[1200px], px-6, py-12)

Vertical stack of 3 project items (gap-16 md:gap-20). Each has:

Text block offset left (ml-20 md:ml-28): Project name in PP Mondwest serif (text-2xl md:text-3xl, font-semibold, color #051A24) + description (text-sm md:text-base, color #051A24/70)
Full-width image below (rounded-2xl, shadow-lg, object-cover)
Each item independently triggers fade-in animation via IntersectionObserver.
Projects:

"evr" - "From idea to millions raised for a web3 AI product" - https://motionsites.ai/assets/hero-evr-ventures-preview-DZxeVFEX.gif
"Automation Machines" - "Streamlining industrial automation processes" - https://motionsites.ai/assets/hero-automation-machines-preview-DlTveRIN.gif
"xPortfolio" - "Modern portfolio management platform" - https://motionsites.ai/assets/hero-xportfolio-preview-D4A8maiC.gif
7. PARTNER SECTION (full width, py-12, px-6)

Large white container (max-w-7xl, py-48, rounded-[40px], subtle shadow). On mouse hover, GIF thumbnails (from the marquee images array) spawn at cursor position with random rotation (-10 to +10 deg), fade out over 1000ms with scale-down, spawning every 80ms minimum. Uses requestAnimationFrame-style cleanup.

Centered heading: "Partner with us" in PP Mondwest serif, text-[48px] md:text-[64px] lg:text-[80px], color #0D212C, mb-12.
CTA button: Dark pill with circular avatar image (Pexels photo 415829, w-10 h-10 rounded-full) + "Start chat with Viktor". Same primary button shadow style.
8. FOOTER (full width, py-12, px-6, max-w-[1200px])

Flex row (md:flex-row). Left side: "Start a chat" primary button. Right side: ArrowUpRight icon (lucide-react), then two columns of links:

Column 1: Services, Work, About (anchor links)
Column 2: x.com, LinkedIn (external links, target _blank)
All links: text-base, color #051A24, hover:opacity-70 transition.

9. COPYRIGHT BAR (max-w-[1200px], px-6, py-4)

Flex row justify-between: "Vortex Studio Limited" on left, "Austin, USA" on right. Text-sm, color #051A24.

10. FIXED BOTTOM NAV (z-50, centered)

Floating pill fixed to bottom (bottom-6, centered via left-1/2 -translate-x-1/2). White bg, rounded-full, px-8 py-2, complex layered shadow. Contains: "V" letter in PP Mondwest serif (text-2xl, font-semibold, color #051A24) + "Start a chat" primary button.

ANIMATIONS:

All sections use a custom useInViewAnimation hook (IntersectionObserver with threshold 0.1, triggers once). Elements get class animate-fade-in-up when in view (otherwise opacity-0). The animation is defined in CSS:


@keyframes fadeInUp {
  0% { opacity: 0; transform: translateY(30px); }
  100% { opacity: 1; transform: translateY(0); }
}
.animate-fade-in-up {
  animation: fadeInUp 0.8s ease-out forwards;
  opacity: 0;
}
Each element within a section has staggered animationDelay values (0.1s, 0.2s, 0.3s, etc.).

COLOR PALETTE:

Primary dark: #051A24
Secondary dark: #0D212C
Light text on dark: #F6FCFF, #E0EBF0
Body text: #051A24
Muted text: #273C46
Background: white throughout
BUTTON SHADOWS (critical for the design feel):

Primary: 0_1px_2px_0_rgba(5,26,36,0.1), 0_4px_4px_0_rgba(5,26,36,0.09), 0_9px_6px_0_rgba(5,26,36,0.05), 0_17px_7px_0_rgba(5,26,36,0.01), 0_26px_7px_0_rgba(5,26,36,0), inset_0_2px_8px_0_rgba(255,255,255,0.5)
Secondary: 0_0_0_0.5px_rgba(0,0,0,0.05), 0_4px_30px_rgba(0,0,0,0.08)
FONTS (CSS):


@font-face {
  font-family: 'PP Neue Montreal';
  src: url('https://assets.website-files.com/6009ec8cda7f305645c9d91b/60176f9bb43e36419997ecfe_PPNeueMontreal-Book.otf') format('opentype');
  font-weight: 400;
  font-display: swap;
}
@font-face {
  font-family: 'PP Neue Montreal';
  src: url('https://assets.website-files.com/6009ec8cda7f305645c9d91b/60176f9b39c5673e51a86f5a_PPNeueMontreal-Medium.otf') format('opentype');
  font-weight: 500;
  font-display: swap;
}
@font-face {
  font-family: 'PP Mondwest';
  src: url('/PPMondwest-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: swap;
}
FILE STRUCTURE:

src/App.tsx - Main layout with hero, marquee, and section composition
src/components/Button.tsx - Reusable button (primary/secondary/tertiary variants)
src/components/TestimonialSection.tsx - Quote with parallax image
src/components/PricingSection.tsx - Two pricing cards
src/components/TestimonialCarousel.tsx - Auto-scrolling testimonial cards
src/components/ProjectsSection.tsx - Project showcase items
src/components/PartnerSection.tsx - Interactive mouse-trail CTA section
src/components/Footer.tsx - Footer with links
src/components/CopyrightBar.tsx - Copyright line
src/components/BottomNav.tsx - Fixed floating bottom nav
src/hooks/useInViewAnimation.ts - IntersectionObserver scroll-trigger hook
src/index.css - Font faces, marquee animation, fade-in-up animation

## Yacht Club — Landing Page [sites/yacht-club-hero]

- Preview: https://motionsites.ai/assets/hero-yacht-club-preview-BXyoIjIf.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/yacht-club-hero.gif

Core Instruction
Act as an elite, award-winning creative frontend developer. Your task is to perfectly recreate a luxury Yacht Club landing page experience with highly advanced WebGL-like DOM animations, a custom GSAP staggered menu, a Framer Motion video fleet overlay, and an interactive liquid distortion cursor trail.
Tech Stack
Framework: React 19, Vite, TypeScript
Styling: Tailwind CSS v4
Animation: motion/react (Framer Motion) and gsap
Fonts: @fontsource-variable/geist and Google Fonts (Instrument Serif)
1. Global CSS & Theming (src/index.css)
Configure the global CSS exactly as follows using Tailwind v4 syntax:
Import Instrument Serif from Google Fonts and tw-animate-css.
Define the base theme with OKLCH colors for a dark luxury aesthetic.
Set --background: oklch(0.145 0 0); and --foreground: oklch(0.985 0 0);.
Use --font-sans: "Instrument Serif", ui-serif, serif; and map --font-heading to it to force the serif font on headers. Force @layer base to have body { @apply bg-background text-foreground; }.
Ensure all text throughout the app default to font-serif uppercase unless specified otherwise.
2. Global State & App Structure (src/App.tsx)
Create a single-page app utilizing AnimatePresence. Manage two main states:
isMenuOpen: Boolean linked to the custom GSAP menu.
isFleetOpen: Boolean triggered by clicking "Our Fleet" in the menu.
Background Video:
Implement an absolute, z-0 background wrapper using <motion.div>.
Frame the following Vimeo iframe exactly:
<iframe src="https://player.vimeo.com/video/1184061018?background=1&autoplay=1&loop=1&byline=0&title=0" className="absolute top-1/2 left-1/2 w-[100vw] h-[56.25vw] min-h-[100vh] min-w-[177.77vh] -translate-x-1/2 -translate-y-1/2" />
Use motion.div to apply a dynamic CSS blur. When isFleetOpen is true, animate the filter to blur(100px) (duration 1.56s). When false, animate to blur(0px) (duration 1.3s).
3. The Interactive Liquid Cursor Trail
Build an interactive liquid ripple system inside App.tsx utilizing a hidden SVG filter and a DOM pool of 80 div elements.
The SVG Filter:
Place this SVG at the top of the app container:
<svg className="hidden"><filter id="liquid-trail"><feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="2" result="noise" /><feDisplacementMap in="SourceGraphic" in2="noise" scale="30" xChannelSelector="R" yChannelSelector="G" /></filter></svg>
The Ripple Logic:
Create an array of 80 objects with { x: 0, y: 0, age: 0, active: false } managed via useRef.
On window mousemove, calculate the distance since the last mouse position. If distance > 25px, spawn a new ripple by setting active = true, writing the X/Y coords, and incrementing a rotating current index (idx + 1) % 80.
Use requestAnimationFrame to step the rings. For each active ring, increment age += 0.012.
Ring Math: size = 20 + (age * 280). opacity = 1 - Math.pow(age, 1.2).
If age >= 1, set opacity to 0 and scale to 0.
The React DOM nodes for the ripples should be absolute empty divs with inline styles: backdropFilter: 'url(#liquid-trail) blur(1px)', WebkitBackdropFilter: 'url(#liquid-trail) blur(1px)', boxShadow: 'inset 0 0 30px rgba(255,255,255,0.1), 0 0 15px rgba(147,197,253,0.15)', willChange: 'transform, opacity, width, height, left, top'
4. Hero Content (!isFleetOpen state)
Wrap this inside <AnimatePresence> to mount/unmount seamlessly when the fleet viewer opens.
Main Heading: Positioned absolute top-[96px] left-[20px] md:left-[96px].
Use motion.div with staggered children variants mapping to: hidden: { opacity: 0, y: 40, transition: { duration: 0.48 } } and visible: { opacity: 1, y: 0, transition: { duration: 0.96 } }.
The text is: "MASTER THE" > "ELEMENTS." (italicized) > "EMBRACE THE" > "OCEAN" (italicized).
Styling: text-[64px] md:text-[140px] font-normal leading-none drop-shadow-2xl.
Subtext:
Text: "JOIN AN EXCLUSIVE COMMUNITY OF SAILORS. WHETHER YOU CRAVE THE THRILL OF THE OPEN SEA OR THE SERENITY OF A SUNSET CRUISE, YOUR NEXT GREAT ADVENTURE STARTS HERE."
Position: Pushed over slightly on desktop md:translate-x-[100px], text-[10px] md:text-xs, strictly w-[260px], tracking-widest.
Floating CTA Button: Positioned bottom-8 right-8 z-50.
Button text: "JOIN THE [italicized: CLUB]"
On desktop, if the menu is open, translate the button to the left using transform: translateX(calc(-1 * clamp(260px, 38vw, 420px))).
5. Fleet Video Overlay Modal (isFleetOpen state)
When opened, display a fixed flex container taking up the full screen (flex-col md:flex-row).
The Data: Use this exact JSON array:
code
JSON
[
  {
    "src": "https://app-uploads.krea.ai/wan-videos/08006647-1c55-4823-b35d-e40d57c66bf8.mp4",
    "title": "OCEAN\nECLIPSE",
    "specs": [
      { "label": "LENGTH", "value": "28 M (92 FT)" },
      { "label": "CRUISING SPEED", "value": "22 KNOTS" },
      { "label": "GUESTS", "value": "UP TO 12 GUESTS" },
      { "label": "CABINS", "value": "4 EN-SUITE CABINS" },
      { "label": "SPECIAL FEATURE", "value": "ADVANCED GYRO STABILIZATION" }
    ]
  },
  {
    "src": "https://app-uploads.krea.ai/wan-videos/91fd9932-6194-4d58-ada0-955692853019.mp4",
    "title": "BLACK\nSOVEREIGN",
    "specs": [
      { "label": "LENGTH", "value": "24 M (78 FT)" },
      { "label": "TOP SPEED", "value": "45 KNOTS" },
      { "label": "HULL", "value": "CARBON FIBER & KEVLAR" },
      { "label": "ENGINES", "value": "TWIN V12 2000 HP" },
      { "label": "SPECIAL FEATURE", "value": "BESPOKE DESIGN WITH GOLD DETAILING" }
    ]
  },
  {
    "src": "https://app-uploads.krea.ai/wan-videos/95fb3282-d7cf-448e-9202-ef0662541c83.mp4",
    "title": "AZURE\nHORIZON",
    "specs": [
      { "label": "LENGTH", "value": "32 M (105 FT)" },
      { "label": "RANGE", "value": "1,500 NAUTICAL MILES" },
      { "label": "GUESTS", "value": "14 GUESTS + 5 CREW" },
      { "label": "DECK", "value": "SPACIOUS SUN DECK WITH JACUZZI" },
      { "label": "SPECIAL FEATURE", "value": "FULL WATER TOYS GARAGE" }
    ]
  }
]
FleetVideo Component:
Each column maps out a <video> taking up w-full h-[85vh] md:h-full md:flex-1. Include borders (border-r-2 border-white).
Animate columns coming in from the right (x: '100vw' to x: 0) using Framer Motion with delays staggering by i * 0.1 and a duration of 1.56.
Add onMouseEnter / onMouseLeave state to trigger video .play() and .pause() wrapped safely in a promise resolver to avoid interruptions.
On hover, scale the video up to 105% with duration-700.
Show a black overlay (bg-black/20).
Hover Content: Inside <AnimatePresence>, render the title (e.g., text-5xl md:text-7xl mb-12), the mapped specs (uppercase tracking-widest text-white/70), and a "VIEW" button (bg-white/5 backdrop-blur-[120px]). Apply the specific y stagger text variants so they gracefully shift up from a clipped mask.
6. The GSAP Custom Staggered Menu (StaggeredMenu.jsx)
Create an off-canvas navigation sidebar triggered from the right using GSAP.
The Wrapper & Icon: The hamburger icon consists of a div wrapper containing two spans forming a "Plus" shape. When open, GSAP controls it: rotate the wrapper by 225deg to form an X.
Slot Machine Text Link: Next to the hamburger, display the text "MENU". When clicked, use a slot-machine-style vertical transition, cycling the words "MENU" and "CLOSE" three times before settling on the correct target word. Shift the interior wrapper yPercent: -finalShift rapidly over 0.5s easing power4 out.
Menu Colors: Set the GSAP configuration to use colors ['#1a1a1a', '#93c5fd']. On open, the menu button's color changes automatically from #ffffff to #000000.
The Panel Animation:
Utilize "prelayers", an array of div masking layers that slide in from 100vw. GSAP should .fromTo their xPercent to 0 using power4.out staggered by 0.07s.
The main panel follows right after taking 0.65 seconds.
Menu Items: The menu items MUST contain: Home, Our Fleet (triggers setIsFleetOpen(true) and closes the menu), Membership, Regattas & Events, Academy, Contact. Animate their entrance using yPercent and a 10deg rotation rotating back to flat using .stagger: { each: 0.1 } mapped directly against the items NodeList.
Wrap the StaggeredMenu over the app context, providing the right position, colors, and social handles (Instagram, Facebook, Twitter).
Ensure all component names, class names, file structures, and specific math equations are strictly ported to perfectly mimic the reference logic.

## Yoga Coach — Landing Page [sites/yoga-coach]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(39).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/yoga-coach.webp

Build a full-screen, two-section yoga coach landing page using React + TypeScript + Vite + Tailwind CSS v4 + Motion (framer-motion successor) + hls.js. The page has NO scrolling -- it is exactly viewport-sized with two states: a hero video screen and a second "collection" screen that slides up after the video ends.

---

### TECH STACK & SETUP

- **Framework:** React 19 with TypeScript
- **Bundler:** Vite 6 with `@vitejs/plugin-react` and `@tailwindcss/vite`
- **Styling:** Tailwind CSS v4 (imported via `@import "tailwindcss"` in CSS)
- **Animation:** `motion` package (the `motion/react` import path)
- **Video Streaming:** `hls.js` for HLS (.m3u8) stream playback
- **Font:** Google Fonts "Anton" (display sans-serif, all-caps condensed)

---

### FONT & THEME CONFIGURATION

**index.css:**
```css
@import url('https://fonts.googleapis.com/css2?family=Anton&display=swap');
@import "tailwindcss";

@theme {
  --font-anton: "Anton", sans-serif;
}
```

Use `font-anton` class throughout. All text is UPPERCASE.

---

### MEDIA ASSETS (EXACT URLS)

**Poster/Background Image (first screen, before video plays):**
```
https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260624_053103_a6c6fd5c-8f43-4942-a487-e81c8f3cd0c1.png&w=1920&q=85
```

**Main Background Video (HLS stream, plays on button click):**
```
https://stream.mux.com/YyFgoUXUTVMiVMMLeQoq49tP00joMyEzoRmnEnk02H5rA.m3u8
```

**Three Card Videos (HLS streams, second section, play on hover):**
```
Card 1 (Left):   https://stream.mux.com/S9BmS8DLYYowz7jr1BkN2PbAQm4bjEwijllpwmEB4xA.m3u8
Card 2 (Center): https://stream.mux.com/83kMTSKA4Xy01RTddNwDdt7OjCocQTCdKHY7AsD02Dlgc.m3u8
Card 3 (Right):  https://stream.mux.com/c4KUkE6NHGljcc4M8458iMEdHAbUvsG5MTpqefzBYvo.m3u8
```

---

### SECTION 1: HERO VIDEO SCREEN (Full Viewport)

**Container:** `w-screen h-screen bg-black overflow-hidden select-none` with `onMouseMove` handler for parallax.

### Background Video Layer
- A `<video>` element with the poster image URL as the `poster` attribute.
- HLS initialized via hls.js: create `new Hls({ enableWorker: false })`, call `hls.loadSource(url)` and `hls.attachMedia(video)`. Set `isLoaded=true` on `Hls.Events.MANIFEST_PARSED`. Fallback for Safari: set `video.src` directly (native HLS support).
- Video attributes: `muted`, `playsInline`, `preload="auto"`, no `controls`, `onEnded` triggers second screen.
- Inline style: `transform: scale(1.08) translate(${parallax.x}px, ${parallax.y}px)` -- parallax moves max 20px in each direction based on mouse position relative to window center.
- CSS: `w-full h-full object-cover transition-opacity duration-1000 ease-in-out origin-center`. Starts at `opacity-0`, transitions to `opacity-100` when loaded.
- An overlay div on top: `absolute inset-0 bg-black/10 pointer-events-none`.

### Parallax Logic
- On mousemove, compute `x = (clientX - innerWidth/2) / (innerWidth/2)` (range -1 to 1), same for y.
- Set parallax state to `{ x: x * 20, y: y * 20 }`.

### UI Overlay (absolute, full viewport, z-10, pointer-events-none)
- Padding: `p-6 sm:p-12 md:p-16`
- Layout: `flex flex-col justify-between`

### Top-Left Slogan
- Container: `max-w-xs sm:max-w-sm md:max-w-md pointer-events-auto overflow-hidden`
- Animated with Motion: when `isPlaying` is true, animates `{ y: 250, opacity: 0 }`. When false: `{ y: 0, opacity: 1 }`.
- Transition: `duration: 1, ease: [0.16, 1, 0.3, 1]` (custom cubic bezier).
- Text: `font-anton text-white text-xl sm:text-2xl md:text-3xl lg:text-[2.2rem] leading-[0.95] tracking-wide uppercase`
- Content:
  ```
  FIND YOUR FOCUS,
  ARRANGE PRIVATE
  YOGA SESSIONS, AND LIVE
  THE MINDFUL WAY
  ```

### Bottom Bar (w-full, flex col on mobile, flex row items-end justify-between on md+)

**Bottom-Left Title:**
- Container: `pointer-events-auto overflow-hidden`
- Animated: when `isPlaying`, `{ y: "150%", opacity: 0 }` else `{ y: 0, opacity: 1 }`.
- Transition: `duration: 1.1, ease: [0.16, 1, 0.3, 1]`
- Text: `font-anton text-white text-5xl sm:text-7xl md:text-8xl lg:text-[10rem] xl:text-[11rem] leading-none tracking-tight uppercase`
- Content:
  ```
  HEY, I AM
  JESSICA
  ```

**Bottom-Right Title:**
- Same animation/transition as bottom-left.
- Container: `text-left md:text-right pointer-events-auto overflow-hidden`
- Same text sizing.
- Content:
  ```
  YOGA
  COACH
  ```

**Center Circular "LET'S START" Button:**
- Position: `absolute left-1/2 bottom-[15%] md:bottom-2 -translate-x-1/2 z-20 pointer-events-auto`
- Wrapped in `<AnimatePresence>`. Only renders when `!isPlaying`.
- Exit animation: `{ scale: 0, opacity: 0 }` with `duration: 0.8, ease: [0.34, 1.56, 0.64, 1]` (bouncy overshoot bezier).
- Button classes: `w-28 h-28 sm:w-32 sm:h-32 md:w-36 md:h-36 rounded-full bg-black/90 hover:bg-black text-white font-anton text-xs sm:text-sm tracking-widest flex flex-col items-center justify-center gap-1 cursor-pointer transition-all duration-300 border border-white/20 hover:border-white/40 shadow-[0_0_50px_rgba(0,0,0,0.8)] backdrop-blur-sm relative overflow-hidden group`
- Inner radial glow: `absolute inset-0 bg-radial from-white/10 to-transparent opacity-60 group-hover:opacity-100 transition-opacity duration-300`
- Label: `relative z-10 font-bold tracking-wider` text "LET'S START"
- onClick: plays the video and sets `isPlaying=true`.

### Play/Pause Logic
- Clicking button calls `video.play()`, sets `isPlaying=true`.
- When video ends (`onEnded`): sets `isPlaying=false`, `showSecondScreen=true`.

---

### SECTION 2: COLLECTION SCREEN (Slides Up Over Hero)

**Container:** A `motion.div` that covers the full viewport.
- `initial={{ y: "100%" }}`
- `animate={showSecondScreen ? { y: 0 } : { y: "100%" }}`
- `transition={{ type: "spring", damping: 32, stiffness: 220 }}`
- Classes: `absolute inset-0 w-full h-full z-30 flex flex-col items-center justify-start p-6 pt-24 sm:p-12 md:p-16`
- Background: `bg-gradient-to-b from-[#d5effd] via-[#aedcf9] to-[#8cd0f7]` (light blue gradient)

### "BACK TO START" Button
- Position: `absolute top-[56px] left-1/2 -translate-x-1/2 z-40`
- Classes: `flex items-center font-anton text-black hover:text-black/70 text-lg sm:text-xl border border-black/20 hover:border-black/40 px-8 py-3 rounded-full bg-white/40 backdrop-blur-md transition-all duration-300 hover:scale-105 active:scale-95 cursor-pointer`
- onClick: pauses video, resets `currentTime=0`, hides second screen, resets `isPlaying`.

### Content Body
- Container: `w-full flex-1 flex flex-col items-center justify-center relative select-none mt-20 sm:mt-16`

**Background Giant Text (behind cards):**
- `font-anton select-none pointer-events-none uppercase text-[13.5vw] sm:text-[16.9vw] leading-none text-center w-full absolute z-10 whitespace-nowrap`
- Gradient text effect: `bg-gradient-to-b from-white via-white/70 to-white/0 bg-clip-text text-transparent`
- Additional: `transform translate-y-4 filter drop-shadow-[0_2px_15px_rgba(255,255,255,0.1)]`
- Content: `COLLECTION #451`

### Overlapping Card Deck
- Container: `flex items-center justify-center relative z-20 w-full max-w-5xl mt-6`
- Three `<YogaCard>` components overlapping with negative margins.

**Left Card:**
- `src`: Card 1 URL
- `initialRotation={-8}`
- `hoverOffset={{ x: -60, y: -25, rotate: -12, scale: 1.04 }}`
- `zIndexClass="z-20 hover:z-40"`
- `className="-mr-14 sm:-mr-22 md:-mr-28 lg:-mr-32"`

**Center Card:**
- `src`: Card 2 URL
- `initialRotation={0}`
- `hoverOffset={{ x: 0, y: -15, rotate: 0, scale: 1.08 }}`
- `zIndexClass="z-30 hover:z-40"`
- `className="scale-105"`

**Right Card:**
- `src`: Card 3 URL
- `initialRotation={8}`
- `hoverOffset={{ x: 60, y: -25, rotate: 12, scale: 1.04 }}`
- `zIndexClass="z-20 hover:z-40"`
- `className="-ml-14 sm:-ml-22 md:-ml-28 lg:-ml-32"`

---

### YOGACARD COMPONENT (src/components/YogaCard.tsx)

**Props:** `src: string`, `initialRotation: number`, `hoverOffset: { x, y, rotate, scale }`, `zIndexClass: string`, `className?: string`

**HLS Setup:**
- On mount, if `src.endsWith(".m3u8") && Hls.isSupported()`: create `new Hls({ enableWorker: false })`, `loadSource`, `attachMedia`. Destroy on unmount.
- Safari fallback: set `video.src` directly.

**Hover Behavior:**
- Track `isHovered` state via `onMouseEnter`/`onMouseLeave`.
- When hovered: `video.play()`. When unhovered: `video.pause()` and `video.currentTime = 0`.

**Card Styling:**
- `motion.div` with classes: `relative rounded-3xl overflow-hidden aspect-[9/16] w-72 sm:w-[21rem] md:w-[24rem] bg-white border-[6px] sm:border-[8px] border-white shadow-[0_15px_40px_rgba(0,0,0,0.22)] cursor-pointer origin-bottom select-none`
- `initial={{ rotate: initialRotation, x: 0, y: 0, scale: 1 }}`
- `whileHover` animates to `hoverOffset` values with `transition: { type: "spring", stiffness: 200, damping: 20 }`

**Video element:** `w-full h-full object-cover rounded-2xl pointer-events-none`, `muted`, `loop`, `playsInline`, `preload="auto"`.

**Inner ring overlay:** `absolute inset-0 rounded-2xl ring-1 ring-black/10 pointer-events-none`

---

### KEY ANIMATION SUMMARY

| Element | Trigger | Animation | Transition |
|---------|---------|-----------|------------|
| Top slogan | isPlaying | y: 0 to 250, opacity 1 to 0 | 1s, ease [0.16, 1, 0.3, 1] |
| Bottom titles (both) | isPlaying | y: 0 to "150%", opacity 1 to 0 | 1.1s, ease [0.16, 1, 0.3, 1] |
| Play button | isPlaying (exit) | scale 1 to 0, opacity 1 to 0 | 0.8s, ease [0.34, 1.56, 0.64, 1] |
| Second screen | showSecondScreen | y: "100%" to 0 | spring, damping 32, stiffness 220 |
| Cards hover | mouse enter/leave | rotate/x/y/scale to hoverOffset | spring, stiffness 200, damping 20 |
| Background video parallax | mousemove | translate up to 20px each axis | inline style (no transition) |
| Video opacity | loaded state | opacity 0 to 1 | CSS transition 1000ms ease-in-out |

---

### DEPENDENCIES (package.json)

```json
"dependencies": {
  "@tailwindcss/vite": "^4.1.14",
  "@vitejs/plugin-react": "^5.0.4",
  "hls.js": "^1.6.16",
  "motion": "^12.23.24",
  "react": "^19.0.1",
  "react-dom": "^19.0.1",
  "vite": "^6.2.3"
}
```

---

### VITE CONFIG

Uses `@tailwindcss/vite` plugin and `@vitejs/plugin-react`. Path alias `@` resolves to project root.

## Zenith Realty — Landing Page [sites/zenith-realty-landing]

- Preview: https://motionsites.ai/assets/landing-zenith-realty-preview-Y1uTjYYl.gif
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/zenith-realty-landing.gif

Build a luxury real estate landing page called "ZENITH REALTY" with the exact design, components, and animations described below. Do not include a footer.

**Core Setup:**
* **Font:** Use Google Font 'Lato' (weights 300, 400, 500, 700, 900). Configure it in tailwind by setting `@theme { --font-lato: "Lato", sans-serif; }` in `index.css` and apply `font-lato` globally.
* **Global Style:** The main wrapper should use `bg-[#F8F8F8]` and `text-[#141414]`.
* **Libraries:** Use `lucide-react` for icons, `motion/react` for animations, and `recharts` for charts.

**Section 1: Hero & Navbar**
* Full screen height (`h-screen`), overflow hidden, relative positioning.
* **Background Video:** Absolutely positioned, covering the full area: `<video src="https://d8j0ntlcm91z4.cloudfront.net/user_38xzZboKViGWJOttwIXH07lWA1P/hf_20260503_144509_89e2d612-8af2-45c3-90f4-4831bc60715d.mp4" autoPlay muted loop playsInline className="absolute top-0 left-0 w-full h-full object-cover z-0" />`
* **Content Overlay:** Over the video, use a relative wrapper with `z-10` and `bg-white/10`.
* **Navbar:**
  * **Logo:** Stacked text "ZENITH" over "REALTY" with `text-xl font-black leading-[0.85] tracking-tighter text-[#141414]`.
  * **Links (Desktop):** "Properties" (with `ChevronDown`), "Mortgage" (with a New badge: `bg-black text-[white] text-[9px] px-1.5 py-0.5 rounded-xs leading-none`), "Company", "Careers" (with `ChevronDown`), "Blog". Style: `text-[13px] font-medium tracking-tight text-[#141414] hover:opacity-60`.
  * **Action Button:** "Post a property" (with `Home` icon). Style: `border border-black/10 bg-white/80 backdrop-blur-md px-6 py-2.5 rounded-none text-[13px] font-medium hover:bg-white`.
  * **Mobile Menu:** Implement a functional slide-in mobile menu using `AnimatePresence` and `motion.div` from the right (`x: '100%'` to `x: 0`) containing nav links and a dark "Post a property" button at the bottom.
* **Hero Content:** Grid layout `grid-cols-1 md:grid-cols-12` in the main container.
  * **Headline:** "Discover space you truly belong in" (animate fading up `y: 30`, duration 0.9). Style: `text-4xl md:text-5xl lg:text-7xl font-medium tracking-tight leading-[1.05] text-[#141414]`.
  * **Button:** "Book a call" (animate fade in with delay 0.6). Style: `bg-[#141414] text-white px-9 py-4 text-[13px] font-medium uppercase tracking-wider shadow-2xl`.
  * **Subtext:** "Experience more than a house; find a sanctuary where your journey unfolds, rich with comfort and endless opportunities." (animate fade in with delay 0.4). Position this on the right side of the grid (`md:col-span-4 md:col-start-9`). Style: `text-[#A5A5A5] text-[15px] md:text-[18px] leading-[1.4]`.

**Section 2: Properties**
* **Header:** "Guiding you toward the residence of your dreams" (`text-3xl md:text-5xl font-medium tracking-tight leading-[1.1] text-[#141414]`). To its right (`md:col-start-9`), place subtext: "Our vision bridges balance, design, and attention so that every client resides in a space reflecting their values." (`text-[#A5A5A5] text-[14px] leading-relaxed`).
* **Grid:** 3 property cards that fade and slide up (`motion.div` with staggered delays, `viewport={{ once: true }}`). Use `bg-white`, group class, and image wrappers with `aspect-[4/3] md:aspect-square overflow-hidden group-hover:scale-105 duration-700`.
* **Property Data:**
  1. Title: "Aether Heights", Price: "$345,000", Location: "USA/California/Malibu", Stats: 300 m², 1 floor, 6 beds, 2 baths, Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260503_145701_de344c15-5eac-4c64-8bd6-19a2811bba4a.png&w=1280&q=85`
  2. Title: "Azure Sanctuary", Price: "$225,000", Location: "Caribbean/Bahamas/Bimini", Stats: 250 m², 1 floor, 4 beds, 1 bath, Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260503_145923_c1a9880c-0fab-4a76-8289-bd650d5e5dce.png&w=1280&q=85`
  3. Title: "Summit Pavilion", Price: "$510,000", Location: "USA/Colorado/Vail", Stats: 400 m², 3 floors, 6 beds, 3 baths, Image: `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260503_150022_cdda0eaa-1c17-4f59-8188-4f98b328619f.png&w=1280&q=85`
* **Card Formatting:** In each card details section, place the stats inline in a flex wrap row. Use Lucide icons: `Square` for area, `Layers` for floors, `Bed` for beds, `Bath` for baths. Stat styling: `text-[#141414] text-[11px] font-medium`, icon styling: `text-[#A5A5A5] size={13} strokeWidth={2.5}`.

**Section 3: How it Works**
* **Header:** "Explore our service and the process" with right-aligned subtext "Digital walk-throughs, select portfolios, and professional insight — all the tools to search and secure with ease."
* **Layout:** 12-column grid. Left 4 columns for a custom menu block, right 8 columns for an image (`aspect-video md:aspect-square`).
* **Content Block (Left):** White background (`bg-white p-8 md:p-16`). Include:
  * Title "Exclusive collection", desc "Consultants curate custom lists of vetted homes. Featuring media, VR walk-ins, and private physical tours."
  * Button: "Free consult" (border, transparent bg hover:bg-gray-50).
  * Navigation list at the bottom: 4 text buttons vertically stacked — "Market Analysis", "Exclusive collection" (active mode, `text-[#141414]`), "Policy Support", "Closing Deal". The inactive items should be `text-[#A5A5A5] hover:text-[#141414]`. All `text-[13px] font-medium`.
* **Image (Right):** `https://images.higgs.ai/?default=1&output=webp&url=https%3A%2F%2Fd8j0ntlcm91z4.cloudfront.net%2Fuser_38xzZboKViGWJOttwIXH07lWA1P%2Fhf_20260503_150112_2b0e700f-7af4-4459-b326-7d9e2f468daa.png&w=1280&q=85`

**Section 4: Investment / Analytics**
* **Header:** "Trusted frameworks for secure growth" with two paragraphs of right-aligned subtext: "Our holdings go beyond floor plans; they represent a vehicle for your wealth to thrive consistently." and "We meticulously vet the premier market offerings for our valued partners."
* **Charts Grid:** 3 cards side by side (`bg-white p-6 flex flex-col justify-between aspect-video md:aspect-[1.8/1]`).
  1. Title: "Annual growth", Value: "19%". Data array: `[35, 60, 45, 40, 55, 75, 60, 80, 55, 30]`.
  2. Title: "Aggregate yield profit", Value: "$820,000". Data array: `[8, 12, 18, 28, 32, 38, 55, 70, 85]`.
  3. Title: "Median returns", Value: "14%". Data array: `[10, 75, 20, 35, 30, 65, 55, 25, 40]`.
* **Chart Implementation:** Titles use `text-[#141414]/40 text-[12px] font-medium tracking-tight uppercase`. Values use `text-4xl font-medium text-[#141414]`. Below the values, create an `h-24` container with a `ResponsiveContainer` and `BarChart`. Use a custom `shape` function on the `Bar` that renders two rectangles per bar to simulate a transparent bar with a solid top line:
  * A light background `rect` where `fill="#141414"` and `fillOpacity={0.05}`.
  * A solid top cap `rect` where `height={2}` and `fill="#141414"`.

## Neo Museum — Website [sites/neo-museum]

- Preview: https://pub-86dc5b5484314368ac5436a674b0d919.r2.dev/hero%20sections/animated%20(75).webp
- Asset: https://code.mrday.one/design-assets/sites/visuals-by-id/neo-museum.webp

Project Setup

Stack: React 19 + Vite 6 + Tailwind CSS 4 + Motion (Framer Motion) + Lucide React icons + TypeScript

package.json dependencies:
- `react`, `react-dom` ^19.0.1
- `vite` ^6.2.3
- `@tailwindcss/vite` ^4.1.14, `tailwindcss` ^4.1.14
- `motion` ^12.23.24
- `lucide-react` ^0.546.0
- `@vitejs/plugin-react` ^5.0.4
- `typescript` ~5.8.2

Fonts (loaded via Google Fonts in `index.css`):
- Sans: Inter (weights: 300, 400, 500, 600)
- Mono: JetBrains Mono (weights: 400, 500)

```css
/* index.css */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');
@import "tailwindcss";

@theme {
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, monospace;
}

@layer utilities {
  .text-mega {
    font-size: 21vw;
    line-height: 0.75;
    letter-spacing: -0.04em;
  }
}
```

Global styling: Background `#fcfcfc`, text `#111`, selection color `bg-black text-white`, `overflow-x-hidden`, `font-sans` (Inter).

---

DATA

```tsx
const chaptersData = [
  { name: "Age of Dinosaurs", image: "https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624247/01_udnber.png" },
  { name: "Fossils of Ancient Life", image: "https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624374/02_pmvxxl.png" },
  { name: "Reptiles of the Mesozoic", image: "https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624236/03_hcp3jc.png" },
  { name: "Marine Fossil Gallery", image: "https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624256/04_get63z.png" },
  { name: "Prehistoric Giants", image: "https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624251/05_kz1tyu.png" }
];
```

---

STATE

```tsx
const [showVideo, setShowVideo] = useState(false);
const [activeChapter, setActiveChapter] = useState(2); // starts at "Reptiles of the Mesozoic"
const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
```

- `showVideo` flips to `true` after a 2800ms delay (setTimeout)
- `activeChapter` auto-cycles every 3500ms via setInterval, wrapping `(prev + 1) % 5`

---

ANIMATION VARIANTS

```tsx
const fadeUp = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
};

const letterBlock = {
  initial: { y: 120, opacity: 0 },
  animate: {
    y: 0,
    opacity: 1,
    transition: { duration: 1.2, ease: [0.16, 1, 0.3, 1] }
  }
};
```

---

SECTION 1: HERO (full viewport height)

Container: `relative w-full min-h-screen flex flex-col overflow-hidden`

1A. HEADER (NHM Logo)

- `motion.header` with `staggerChildren: 0.1, delayChildren: 0.1`
- Padding: `pt-6 px-6 md:px-16`, `z-20`
- The "NHM" logo is a custom inline SVG with `viewBox="0 0 840 100"`, `fill-[#111]`, full width
- The SVG is wrapped in `motion.h1` with `variants` that animate from `scale: 1.03` to `scale: 1` with `staggerChildren: 0.06, delayChildren: 0.1`
- Each polygon of each letter uses the `letterBlock` variant (slides up from `y: 120`)
- Letter N (translate 0,0): Three polygons -- left vertical `0,0 14,0 14,100 0,100`, right vertical `200,0 214,0 214,100 200,100`, diagonal `0,0 33,0 214,100 181,100`
- Letter H (translate 280,0): Three polygons -- left vertical `0,0 14,0 14,100 0,100`, right vertical `200,0 214,0 214,100 200,100`, crossbar `14,43 200,43 200,57 14,57`
- Letter M (translate 560,0): Four polygons -- left vertical `0,0 14,0 14,100 0,100`, right vertical `266,0 280,0 280,100 266,100`, left diagonal `0,0 26,0 153,100 127,100`, right diagonal `254,0 280,0 153,100 127,100`

1B. SUB-NAV BAR

- Below the SVG logo, `flex justify-between items-start mt-8`
- Font: `text-[10px] md:text-[11px] font-mono tracking-[0.2em] uppercase`
- Uses `fadeUp` variant with `duration: 0.8, ease: "easeOut"`

Left column (15% width): Three lines -- "Natura" / "History" / "Museum"

Arrow separator (5% width, hidden on mobile): `ArrowRight` from lucide, size 14, strokeWidth 1, `text-gray-400`

Center column (flex-1 on mobile, 30% on desktop): "Exploring the story of life on earth through science, discovery and wonder." -- Split differently on desktop (3 lines) vs mobile (4 lines). `text-gray-800 leading-relaxed font-mono`

Arrow separator (5% width, hidden on mobile): Same as above

Right column (15% width, hidden on mobile): Nav links list -- Visit, Exhibitions, Discover, Learn, About. `text-gray-800`, `hover:text-black hover:underline`

Hamburger button (far right, z-60): Two horizontal lines (`w-8 h-[1.5px] bg-black`), `gap-[6px]`. Hover: first line shrinks to `w-6`, second expands to `w-10`. When open: first rotates 45deg + translateY, second rotates -45deg + translateY (forming an X). Transition: `duration-300`.

1C. MOBILE MENU OVERLAY

- `AnimatePresence` wrapping a `motion.div`
- Appears below the header, slides in from `y: -20`, `opacity: 0` to `y: 0, opacity: 1`
- `bg-[#fcfcfc] border-b border-gray-200 shadow-xl`, only visible on `md:hidden`
- Contains the same nav links as the desktop version, `text-sm font-mono tracking-[0.2em] uppercase`, `space-y-6`

1D. BACKGROUND VIDEO

- Appears after 2800ms delay (controlled by `showVideo` state)
- `absolute top-0 left-0 w-full h-full pointer-events-none z-0`
- Video: `autoPlay loop muted playsInline`, `w-full h-full object-cover`
- Video URL: `https://res.cloudinary.com/dsdxaxkiz/video/upload/v1779624998/magnific_use-img-2-as-the-exact-ba_Piu3X0W42C_wnrc8f.mp4`

1E. LEFT SIDEBAR CONTENT

- `motion.div` with `staggerChildren: 0.15, delayChildren: 0.6`
- Position: `px-10 md:px-16`, `mt-20 sm:mt-28 md:mt-32`, `w-[320px]`, `z-10`

Section indicator: `01` + horizontal line (`w-16 h-[1.5px] bg-black/20`), `text-xs font-mono`

Headline: "TIMELESS WONDERS" -- `text-[3.5rem] md:text-[5rem] font-normal tracking-tight leading-[1]`. Line break between "TIMELESS" and "WONDERS".

Description: "Step into the natural world and / discover the stories written / millions of years ago." -- `text-[13px] md:text-[14px] text-gray-700 w-[240px] leading-[1.6]`

CTA Button ("Explore Now"):
- Container: `bg-[#1a1a1a] px-6 py-3.5 border border-[#1a1a1a] rounded-md shadow-sm`
- Hover: slides up 0.5px, adds `shadow-[3px_3px_0px_rgba(17,17,17,0.5)]`
- Active: resets translate and shadow
- Has a sliding background panel: `bg-[#fcfcfc]` that slides from `-translate-x-[101%]` to `translate-x-0` on hover, `duration-700 ease-[cubic-bezier(0.16,1,0.3,1)]`
- Icon: Custom SVG leaf/plant shape (4 paths forming a stylized leaf), white by default, turns `#111` on hover with `scale-110 -rotate-12 -translate-y-1` transform
- Text: "Explore Now", `text-[15px] font-medium`, white turning to `#111` on hover

1F. RIGHT SIDEBAR (hidden on mobile)

- `motion.div` with `staggerChildren: 0.15, delayChildren: 0.9`
- Position: `w-[200px] mt-12 md:mt-20`, `hidden md:flex`

Specimen info: "Tyrannosaurus Rex" heading (`text-[10px] font-bold font-mono tracking-widest uppercase`), subtext "Late Cretaceous period / 68-66 million years ago" (`text-[12px] text-gray-600 leading-[1.6]`)

Stats: "Length" label + "12.3 m" value, "Height" label + "4.0 m" value. Labels: `text-[10px] font-mono tracking-widest uppercase text-gray-500`. Values: `text-[13px] font-medium`.

View Details button: Circle (`w-10 h-10 rounded-full border border-gray-400`) with `Plus` icon (size 16, strokeWidth 1.5), text "View Details" (`text-[10px] font-mono uppercase tracking-widest font-bold`). Hover: circle gets `border-black bg-[#111]`, icon turns white.

1G. BOTTOM-LEFT "SCROLL TO EXPLORE"

- `absolute bottom-10 left-[2.5rem] md:left-[4rem]`, `hidden md:flex`
- Fade up animation: `delay: 1.2`
- Circle (`w-12 h-12 rounded-full border border-gray-300`) containing two thin vertical lines (`w-[1px] h-[12px] bg-gray-600`, `gap-[4px]`) representing a pause icon
- Text: "Scroll to explore" -- `text-[10px] font-mono tracking-widest uppercase text-gray-500 font-semibold`

---

SECTION 2: "EXPLORE OUR WORLD"

Container: `relative w-full min-h-[75vh] md:min-h-screen bg-[#fcfcfc]`, flex column centered, `pt-24 md:pt-32 pb-0 z-20`

2A. SECTION LABEL

`[ 02 ] Explore Our World` -- `text-[10px] md:text-[11px] font-mono tracking-[0.2em]`, `mb-12`. "02" in `text-gray-500`, "Explore Our World" in `text-gray-900 font-bold uppercase`.

2B. MAIN HEADING

"Unearth the stories of our planet's past through fossils, minerals, and ancient wonders." -- `text-[2.2rem] md:text-[3.5rem] lg:text-[4.2rem] leading-[1.1] font-medium tracking-tight text-[#111]`, max-width 1000px, text-center. Line break on desktop after "past". Animates with `whileInView` from `y: 40, opacity: 0` to `y: 0, opacity: 1`, `once: true`, margin `-100px`.

2C. ACTION PILLS

Five pill buttons in a flex-wrap row, `gap-3 md:gap-4`, `mb-10 md:mb-24`. Staggered reveal animation (`staggerChildren: 0.1, delayChildren: 0.3`). Each pill: `rounded-full border border-gray-300 text-[11px] font-medium uppercase tracking-wider bg-white/50 backdrop-blur-sm text-gray-800`. Hover: `border-black bg-black text-white`. Icons from lucide (size 14, strokeWidth 2):

1. `Bone` + "Dinosaurs"
2. `Dna` + "Ancient Life"
3. `Gem` + "Minerals"
4. `Leaf` + "Fossils"
5. `BookOpen` + "Learn More"

2D. SPACER

`min-h-[220px] md:min-h-[450px]` -- provides room for the pterodactyl image from Section 3 to overlap upward.

2E. BOTTOM TEXT

Absolute positioned at bottom, `px-8 md:px-16 pb-8 md:pb-12`, `pointer-events-none`. Two text elements at `justify-between`:
- Left: "WE DON'T JUST TELL STORIES."
- Right: "PALEONTOLOGY (C) 2026"
- Both: `text-[10px] font-mono tracking-widest uppercase text-gray-500 font-medium`, hidden on mobile.

---

SECTION 3: "ANCIENT COLLECTION" (Dark Section)

Container: `relative w-full bg-[#0a0a0a] text-white flex flex-col z-30`

3A. PTERODACTYL IMAGE (Overlapping)

- Absolute positioned at top, centered horizontally (`left-1/2 -translate-x-1/2`)
- Width: `w-[160vw] md:w-[1100px]`
- Image URL: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779625001/ChatGPT_Image_May_23_2026_12_24_44_PM_1_lv1dne.png`
- Animates with `whileInView` from `y: "-65%", opacity: 0` to `y: "-78%", opacity: 1`, `duration: 1.4, ease: "easeOut"`, viewport margin `100px`
- `pointer-events-none z-0`, `mix-blend` not applied here

3B. HEADING AREA

- Padding: `px-8 md:px-16 pt-32 md:pt-48 mb-16`, `z-10`
- Two-column layout on xl (`flex-col xl:flex-row justify-between`)

Left -- Main heading: "Curated from millions of years of wonder [3 circle icons] & discovery." -- `text-[1.8rem] md:text-[3rem] lg:text-[3.8rem] xl:text-[4rem] leading-[1.15] font-medium tracking-tight text-white`. The three circle icons are inline (`inline-flex gap-2 md:gap-3 align-middle mx-2 md:mx-4 translate-y-[-4px]`), each `w-10 h-10 md:w-14 md:h-14 rounded-full border border-gray-600 bg-black text-gray-400`. Hover: `bg-white text-black border-white`. Icons: `Bone`, `Dna`, `Leaf` (size 22).

Right -- Tagline + pills:
- Tagline: "WE DON'T JUST DISPLAY FOSSILS / WE SHARE EARTH'S STORY" -- `text-[9px] md:text-[10px] font-mono tracking-widest text-gray-400 uppercase mb-6 leading-relaxed`
- Three pills: "Educational", "Authentic", "Inspiring" -- `px-5 py-2 rounded-full border border-gray-600 text-[9px] font-mono tracking-widest uppercase text-gray-300`. Hover: `bg-white text-black border-white`.

3C. TWO-COLUMN PANEL

Separated by `h-[1px] bg-gray-800` line. Flex row on desktop, column on mobile.

Left panel (35% width):
- `border-r border-gray-800` on desktop, `border-b` on mobile
- `min-h-[400px] md:min-h-[500px]`
- Top: `***` text (`text-gray-500 text-xl tracking-[0.3em]`)
- Center: Chapter image using `SandTransitionImage` component (SVG filter-based sand/dissolve transition). Image: `absolute inset-0 w-[80%] h-[80%] m-auto object-contain mix-blend-lighten`. Uses `AnimatePresence mode="wait"`.
- Bottom: Chapter counter `01 / 05` style, with animated number (`motion.div` slides vertically). `text-[10px] font-mono tracking-widest text-[#888] uppercase`. Counter numeral color `#888`, divider `text-[#333]`.

Right panel (65% width):
- Top bar: "Explore the past. Understand the present." + animated "Chapter 0X" label. `border-b border-gray-800 p-8 text-[10px] font-mono text-gray-400 tracking-widest`.
- Chapter list: 5 items, each `border-b border-gray-800/80 py-8`. Active: `text-white`, inactive: `text-[#444] hover:text-[#999]`. Chapter name: `text-2xl md:text-[2rem] font-medium tracking-tight`. Active item shows `ArrowUpRight` icon (size 22, strokeWidth 1, `text-gray-400`) that animates in/out.
- Clicking a chapter sets `activeChapter`.

3D. BOTTOM FOOTER

- `h-[1px] bg-gray-800` divider
- Text: "DIGGING INTO OUR PLANET'S PAST" -- `px-8 py-8 text-[10px] font-mono tracking-widest text-gray-500 uppercase bg-[#0a0a0a]`

---

SandTransitionImage COMPONENT

A custom component that creates a sand/particle dissolve effect using SVG filters:

```tsx
function SandTransitionImage({ src, alt, className }) {
  // Uses usePresence() from motion/react for AnimatePresence awareness
  // Unique filterId per instance via useRef
  // requestAnimationFrame loop over 900ms
  // Easing: entering = quartic ease-out (1 - Math.pow(1-t, 4)), exiting = cubic (Math.pow(t, 3))
  // SVG filter chain:
  //   1. feTurbulence: fractalNoise, baseFrequency 1.8, numOctaves 4
  //   2. feDisplacementMap: scale up to 150 based on progress
  //   3. feOffset: dy up to -80 (enter) or 120 (exit), dx up to -30/+30
  //   4. feGaussianBlur: up to 6px
  //   5. feColorMatrix: opacity fades (1 - progress * 1.2)
  // Image has crossOrigin="anonymous" and referrerPolicy="no-referrer"
}
```

---

ALL EXTERNAL ASSET URLs

Video:
- `https://res.cloudinary.com/dsdxaxkiz/video/upload/v1779624998/magnific_use-img-2-as-the-exact-ba_Piu3X0W42C_wnrc8f.mp4`

Images:
- Chapter 1: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624247/01_udnber.png`
- Chapter 2: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624374/02_pmvxxl.png`
- Chapter 3: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624236/03_hcp3jc.png`
- Chapter 4: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624256/04_get63z.png`
- Chapter 5: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779624251/05_kz1tyu.png`
- Pterodactyl: `https://res.cloudinary.com/dsdxaxkiz/image/upload/v1779625001/ChatGPT_Image_May_23_2026_12_24_44_PM_1_lv1dne.png`

(Note: these are Cloudinary URLs, not CloudFront. The project uses Cloudinary for all hosted media assets.)

---

KEY DESIGN DETAILS

- Color palette: `#fcfcfc` (off-white bg), `#111` / `#1a1a1a` (near-black), `#0a0a0a` (dark section bg). Gray scale via Tailwind: `gray-300` through `gray-800`.
- No purple/indigo anywhere. Strictly monochrome black/white/gray.
- Typography hierarchy: Large display headings (3.5-5rem), mono labels (10-11px), body text (13-14px).
- Spacing: 8px base system throughout.
- Transitions: Most hover transitions 300-700ms. Button slide effect uses `cubic-bezier(0.16, 1, 0.3, 1)`. Letter animations use same cubic bezier.
- The page is entirely a single `App.tsx` component plus the `SandTransitionImage` helper function in the same file.
