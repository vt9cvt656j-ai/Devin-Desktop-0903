# Frontend Component Patterns — Best Practices

## Framework Selection Impact on LLM Quality
- Zustand > Redux (55% vs 45% pass rate — simpler API = fewer errors)
- Tailwind > custom CSS (constrained vocabulary limits output to reasonable values)
- Svelte/Vue > Angular (Angular's complex grammar causes more errors)
- Next.js: always specify App Router vs Pages Router; include "use client"/"use server" rules

## React Patterns

### State Management
```jsx
// Client state → Zustand (NOT Redux)
const useStore = create((set) => ({
  items: [],
  addItem: (item) => set((s) => ({ items: [...s.items, item] })),
  removeItem: (id) => set((s) => ({ items: s.items.filter(i => i.id !== id) })),
}))

// Server state → TanStack Query (handles ~80% of app data)
const { data, isLoading } = useQuery({
  queryKey: ['users', userId],
  queryFn: () => fetchUser(userId),
})
```

### Component Structure
- One component per file, named export matching filename
- Props: destructure with defaults, TypeScript interface above component
- Keep components < 150 lines; extract hooks for logic > 20 lines
- `useMemo`/`useCallback` only when profiler shows re-render cost

### Error Boundaries
```jsx
<ErrorBoundary fallback={<ErrorUI />}>
  <Suspense fallback={<Skeleton />}>
    <AsyncComponent />
  </Suspense>
</ErrorBoundary>
```

## Vue Patterns
- Composition API (`<script setup>`) over Options API
- State: Pinia with Composition API syntax
- Computed > watchers; watchers only for side effects

## Design-to-Code Pipeline
```
1. Get design spec (Figma MCP / screenshot / description)
2. Decompose into sections; generate each independently
3. Map to existing components first (Code Connect pattern)
4. Generate only unmapped elements
5. Evaluate at section level: Text Accuracy, Layout, Spacing, Media Position
6. Fix sections scoring below threshold; recompose
```

## Styling Rules (Tailwind-First)
- Default to Tailwind utility classes
- shadcn/ui for common components (Button, Input, Card, Dialog, etc.)
- Project theme via `tailwind.config` — include in context
- Dark mode: `dark:` variant classes, system preference via `class` strategy
- Custom CSS only for: complex animations, pseudo-elements, third-party overrides

## Testing
- Component tests: Testing Library (`render` + `screen.getByRole` + `userEvent`)
- E2E: Playwright with test agents (Planner → Generator → Healer)
- Visual: screenshot at 375px, 768px, 1440px → compare → fix
- Three iterations is the sweet spot — diminishing returns after

## Anti-Patterns to Avoid
- No placeholder content — all data must be real or fetched
- No `any` type in TypeScript
- No inline styles (use Tailwind classes)
- No `document.querySelector` in React/Vue (use refs)
- No prop drilling past 2 levels (use context or state management)
- No `useEffect` for derived state (use `useMemo`)
