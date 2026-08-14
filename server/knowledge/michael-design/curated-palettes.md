# Michael Design Library — curated-palettes

Operator-verified palettes (业主实测过的配色). These are not theory: each one has been used to build real sites with this agent and produced results the operator kept. When no category blueprint pins a palette, pick one of these by character instead of inventing colours — an invented palette is the single most reliable way to make a page look AI-generated.

Every palette below gives: the four values that matter (page background / ink / primary / surface), the Tailwind step that matches, what it is for, and what ruins it. Keep ≥90% of the page in background/ink/surface; the primary appears only on the main CTA, links, selected state, focus ring, and at most one emphasis per screen.

## Mono Ink — near-black on white, one hairline [palettes/mono-ink]

The safest palette in existence and the hardest to make look cheap. Editorial, confident, never dated. This is the correct default when the user has not expressed a colour preference.

- background `#FFFFFF` (`white`) · ink `#111111` (`neutral-900`) · primary `#111111` (ink itself) · surface `#FAFAFA` (`neutral-50`) · hairline `#E5E5E5` (`neutral-200`)
- Dark variant: background `#0A0A0A` (`neutral-950`) · ink `#FAFAFA` · surface `#171717` (`neutral-900`) · hairline `rgb(255 255 255 / 0.10)`
- **Primary is the ink.** A black button on white with white text. No colour anywhere until the design genuinely needs a signal.
- Use for: portfolio, agency, editorial, docs, developer tools, luxury, anything where "expensive" beats "friendly".
- Ruined by: adding a second neutral family (mixing zinc with slate reads as sloppy), grey-on-grey text, borders everywhere instead of whitespace, and pure `#000` — near-black is softer and looks intentional.

## Paper Warm — off-white paper, warm ink [palettes/paper-warm]

Mono Ink with the temperature turned up. Reads as crafted and human rather than clinical; the small warm shift in the background is what separates it from a default white page.

- background `#FBFAF8` (`stone-50`) · ink `#1C1917` (`stone-900`) · primary `#1C1917` · surface `#FFFFFF` (cards sit *lighter* than the page) · hairline `#E7E5E4` (`stone-200`)
- Optional accent when one is needed: terracotta `#B45309` (`amber-700`) or clay `#9A3412` (`orange-800`)
- Use for: bakery, café, wellness, ceramics, florist, wedding, bookshop, slow-living brands, anything with food or craft.
- Ruined by: turning the whole page amber. The warmth belongs in the background and one accent — the moment section backgrounds become orange, it stops looking warm and starts looking like a template.

## Google — white, generous space, one confident blue [palettes/google-material]

The most legible palette on this list. Big white areas, strong type hierarchy, colour used as function rather than decoration. Best when the product must feel trustworthy and mainstream.

- background `#FFFFFF` · ink `#202124` · primary `#1A73E8` · surface `#F8F9FA` · hairline `#DADCE0`
- Functional colours (only for real states): success `#188038` · warning `#F9AB00` · danger `#D93025`
- Type: one sans family, weights 400/500/700 only; headings large and tight, body 14–16px with generous line height.
- Pills and 8px radii; buttons are solid primary or plain text — no gradients, no shadows heavier than `0 1px 3px rgb(60 64 67 / .3)`.
- Use for: dashboards, admin, SaaS, education, public services, forms-heavy products.
- Ruined by: using the four brand colours decoratively. Red/yellow/green mean error/warning/success and nothing else.

## Apple — near-white, huge type, one blue [palettes/apple-cupertino]

Space and typography do the work; colour barely participates. Requires real images and real copy — it has nowhere to hide.

- background `#F5F5F7` · ink `#1D1D1F` · primary `#0071E3` · surface `#FFFFFF` · hairline `rgb(0 0 0 / 0.08)`
- Dark variant: background `#000000` · ink `#F5F5F7` · surface `#1D1D1F` · primary `#2997FF`
- Type is the design: hero 48–80px, weight 600, tracking tight (`-0.02em`); body 17–21px, muted `#6E6E73`. Sections are tall with very large vertical rhythm (`py-32`).
- Buttons are small blue pills with generous padding; links are blue with a chevron, not underlined.
- Use for: product launches, hardware, premium consumer apps, anything with strong photography.
- Ruined by: small type and dense sections. This palette only works with air; cramming it makes it look like a bad clone.

## Ink & Signal — near-black canvas, one saturated accent [palettes/ink-signal]

The modern developer-product look (Linear / Vercel lineage). Dark, quiet, with one colour that appears rarely enough to still mean something.

- background `#0A0A0A` (`neutral-950`) · ink `#EDEDED` · surface `#141414` · hairline `rgb(255 255 255 / 0.08)`
- Accent: pick exactly one — indigo `#6366F1`, emerald `#10B981`, sky `#38BDF8`, or violet `#8B5CF6`. Use its 400 step for text on dark, 500 for fills.
- Cards must be **lighter** than the canvas, with a 1px top inner highlight `inset 0 1px 0 rgb(255 255 255 / 0.06)` — flat grey cards on a dark page is the clearest "AI wrote this" tell.
- Use for: developer tools, infrastructure, AI products, crypto, analytics.
- Ruined by: glow. No neon text, no coloured outer shadows, no full-page gradient. Depth comes from surface steps, not luminance.

## Nordic Calm — cool stone, deep teal [palettes/nordic-calm]

Quiet and grown-up without being monochrome. A good escape hatch when black-and-white feels too severe but a bright brand colour would be wrong.

- background `#F8FAFC` (`slate-50`) · ink `#0F172A` (`slate-900`) · primary `#0F766E` (`teal-700`) · surface `#FFFFFF` · hairline `#E2E8F0` (`slate-200`)
- Use for: healthcare, finance, legal, consulting, B2B services, anything that must feel steady.
- Ruined by: pairing cool slate with warm accents (amber, terracotta) — pick one temperature and stay there.

## Forest — off-white, deep green [palettes/forest-organic]

Natural without the clichéd "eco = bright green". The green is deep enough to serve as ink-adjacent, so the page stays calm.

- background `#FCFCF9` · ink `#1A2E1A` · primary `#166534` (`green-800`) · surface `#FFFFFF` · hairline `#E3E8E3`
- Use for: outdoor, agriculture, sustainability, tea, garden, veterinary, non-profit.
- Ruined by: `green-500`. Bright green reads as a template; the deep step is what makes it look considered.

## Midnight Gold — deep navy, restrained gold [palettes/midnight-gold]

For work that must look established. Gold only as a hairline, a small icon, or a single number — never as a fill.

- background `#0B1220` · ink `#E8EDF5` · surface `#131D31` · primary `#C9A227` · hairline `rgb(201 162 39 / 0.25)`
- Light variant: background `#FFFFFF` · ink `#0B1220` · primary `#0B1220` with gold reserved for accents.
- Use for: finance, law, luxury hospitality, private clubs, awards.
- Ruined by: gold gradients and gold text at body size. Gold is a highlight, not a colour scheme.

## How to choose, and how to stop [palettes/choosing]

1. If a category blueprint from this library already pins a palette, use that — it beats everything here.
2. Otherwise pick by **character**, not by taste: what should a first-time visitor feel in one second? Serious → Mono Ink or Nordic. Warm/craft → Paper Warm. Trustworthy/utility → Google. Premium consumer → Apple. Technical → Ink & Signal.
3. If the user names a colour or shows a reference, that wins outright.
4. Map the four values onto the project's own token names (`background/foreground/primary/card/muted/border`). Business components consume tokens, never raw hex.
5. Stop at one primary. A second colour needs a stated job (a real success/warning/danger state, or a distinct product line) — "it looked plain" is not a job.
