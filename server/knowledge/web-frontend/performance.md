# Frontend Performance — Core Web Vitals Checklist

## Target Thresholds
- **LCP** (Largest Contentful Paint): < 2.5s
- **INP** (Interaction to Next Paint): < 200ms
- **CLS** (Cumulative Layout Shift): < 0.1

## Images
- All below-fold images: `loading="lazy"`
- All images MUST have explicit `width` and `height` attributes (prevents CLS)
- Use `<picture>` with `srcset` for responsive images; serve WebP/AVIF
- Hero image: `fetchpriority="high"` + preload via `<link rel="preload" as="image">`
- Use `next/image` in Next.js (auto-optimizes)

## JavaScript
- Route-level code splitting: `React.lazy()` + `<Suspense>` (React) / `defineAsyncComponent` (Vue)
- Never top-level import large libraries; use dynamic `import()`
- Third-party scripts: `<script defer>` or `<script async>`
- Tree-shake: `import { Button } from '@/components/Button'` not `import * as Components`

## CSS
- Inline critical CSS in `<head>` (above-fold styles)
- Defer non-critical CSS: `<link rel="preload" as="style" onload="this.rel='stylesheet'">`
- Prefer CSS animations over JS animations
- `will-change` sparingly — only on elements about to animate, remove after

## Fonts
- `font-display: swap` on all `@font-face`
- Preconnect: `<link rel="preconnect" href="https://fonts.googleapis.com">`
- Subset fonts to used character ranges
- Prefer system font stack for body text when possible

## Lists & Virtual Scrolling
- Lists > 100 items: use `@tanstack/virtual` or `react-window`
- Infinite scroll: load in pages of 20-50, intersection observer trigger

## Responsive
- Mobile-first CSS: start with smallest viewport, add complexity via `@media (min-width:)`
- Breakpoints: `sm: 640px, md: 768px, lg: 1024px, xl: 1280px`
- Prefer `@container` queries for component-level responsiveness
- Use `clamp()` for fluid typography: `font-size: clamp(1rem, 2.5vw, 1.5rem)`

## Verification Loop
```
1. Generate code
2. Run Lighthouse audit (via CLI or MCP)
3. Check LCP, INP, CLS against thresholds
4. If any metric fails → read specific Lighthouse recommendation → fix → re-audit
5. Repeat until all passing
```
