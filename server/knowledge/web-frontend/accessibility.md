# Web Accessibility (WCAG 2.2 AA) — Agent Checklist

## Mandatory Rules (Every UI Component)

1. **Semantic HTML first** — `<button>` not `<div onClick>`, `<nav>` not `<div class="nav">`, `<main>/<aside>/<header>/<footer>` for landmarks
2. **All images need alt text** — decorative = `alt=""`, informative = describe content, functional = describe action
3. **Color contrast** — normal text ≥ 4.5:1, large text (≥18px bold / ≥24px) ≥ 3:1, UI controls ≥ 3:1
4. **Keyboard navigation** — every interactive element reachable via Tab, operable via Enter/Space, visible focus indicator (min 2px outline, 3:1 contrast)
5. **Skip navigation** — first focusable element = `<a href="#main-content" class="sr-only focus:not-sr-only">Skip to content</a>`
6. **Form labels** — every `<input>` has `<label for="">` or `aria-label`; errors announced via `aria-describedby` + `aria-invalid="true"`
7. **Dynamic content** — updates announced via `aria-live="polite"` (non-urgent) or `aria-live="assertive"` (errors)
8. **Modals** — trap focus inside, ESC to close, return focus to trigger on close, `role="dialog" aria-modal="true" aria-labelledby="title-id"`

## Verification Loop (Run After Every UI Generation)

```
1. Render component in headless browser
2. Run axe-core → collect violations by severity
3. If Critical/Serious violations → fix in order of severity → re-render → re-check
4. Repeat up to 3 times
5. Few-shot prompting for a11y is WORSE than feedback loops — always use verify-fix loop
```

## Common Failures (89% of axe failures are these)

| Issue | Fix |
|-------|-----|
| Color contrast | Use design tokens with pre-verified contrast ratios |
| Missing alt text | Add descriptive alt; decorative images get `alt=""` |
| Missing form labels | Add `<label>` or `aria-label` |
| Missing landmark regions | Wrap in `<main>`, `<nav>`, `<header>`, `<footer>` |
| No skip navigation | Add skip link as first focusable element |
| Focus not visible | `outline: 2px solid var(--primary); outline-offset: 2px` |

## ARIA Patterns for Common Components

- **Tabs**: `role="tablist"` → `role="tab" aria-selected` → `role="tabpanel" aria-labelledby`
- **Accordion**: `<button aria-expanded aria-controls="panel-id">` → `<div id="panel-id" role="region">`
- **Dropdown**: `role="listbox"` + `role="option"` + `aria-activedescendant`
- **Toast/Notification**: `role="status" aria-live="polite"` (info), `role="alert"` (errors)
- **Loading**: `aria-busy="true"` on container + `role="status" aria-live="polite"` for status text
