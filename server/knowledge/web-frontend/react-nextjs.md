# Web Frontend (React / Next.js)

> Opinionated, battle-tested defaults for building React + Next.js (App Router) frontends. When in doubt, follow the RIGHT DEFAULT given here. Assume React 19 and Next.js 15/16 App Router unless told otherwise (React 18 / Next 14 only when you've confirmed the project is on them — check `package.json` before using version-gated APIs).

## React state: useState vs useReducer vs Context vs a store (the decision tree)

Pick the SMALLEST tool that fits. Escalate only when you hit the next problem.

- **`useState`** — default for almost everything. Independent values, booleans, inputs, toggles, a single object that updates in simple ways. Use multiple `useState` calls over one giant state object.
- **`useReducer`** — when the next state depends on the previous state AND there are many related transitions (e.g. a wizard, a cart, a complex form with `add/remove/reset/validate`). Rule of thumb: 3+ related `setX` calls that always move together → one reducer.
- **React Context** — for LOW-FREQUENCY, app-wide values that rarely change: theme, current user, locale, feature flags. NOT for fast-changing state (mouse position, form keystrokes) — every consumer re-renders on every change.
- **A store (Zustand)** — when many components across the tree read/write the SAME mutable state and you want surgical re-renders without prop drilling or a Provider wrapping everything. Zustand is the right default external store in 2024+. Reach for Redux Toolkit only on large teams that need its devtools/middleware conventions, or an existing Redux codebase.
- **Server state ≠ client state.** Data from an API (lists, entities, anything fetched) belongs in **TanStack Query (React Query)** or framework data loaders — NOT `useState` + `useEffect`, and NOT a global store. Mixing server cache into Redux/Zustand by hand is a classic amateur smell.

```ts
// Zustand: define once, select narrowly so only readers of `count` re-render.
import { create } from 'zustand'
type CounterState = { count: number; inc: () => void; reset: () => void }
export const useCounter = create<CounterState>((set) => ({
  count: 0,
  inc: () => set((s) => ({ count: s.count + 1 })),
  reset: () => set({ count: 0 }),
}))
// In a component — select a SLICE, not the whole store:
const count = useCounter((s) => s.count)        // re-renders only when count changes
// DON'T: const { count } = useCounter()        // re-renders on ANY store change
```

PITFALLS: reaching for Context/Redux on day one for a 3-component app; putting fetched data in a global store; selecting the whole Zustand store instead of a slice; one mega `useState({...})` object that forces spread-merge on every update.

## Lift state up, avoid prop drilling, and never duplicate state

- **Lift state up** to the *closest common ancestor* of the components that need it — and no higher. If two siblings must share state, it lives in their parent. Don't hoist state to the app root "just in case"; that creates re-render storms and prop drilling.
- **Single source of truth.** Each piece of state lives in exactly ONE place. If component B needs A's value, pass it down (or read from a shared store) — do NOT copy it into B's own `useState`.
- **Prop drilling** (passing a prop through 3+ layers that don't use it) is a signal, not a sin. Fix it by, in order of preference: (1) **component composition** — pass JSX as `children`/render props so the data and its consumer stay close; (2) Context for truly global, low-frequency values; (3) a store for shared mutable state.

```tsx
// Composition kills drilling: Layout doesn't need `user`, so don't thread it through.
function Page({ user }: { user: User }) {
  return <Layout sidebar={<Profile user={user} />} />   // user goes straight to its consumer
}
// vs DON'T: <Layout user={user} /> -> <Sidebar user={user} /> -> <Profile user={user} />
```

PITFALLS: copying a prop into local state and then they drift out of sync; "controlled-ish" components that store a mirror of their prop; adding Context for something only two adjacent components use (composition is simpler).

## Derived state is an anti-pattern — compute, don't store

If a value can be calculated from existing props/state, **calculate it during render**. Do NOT store it in `useState` and sync it with `useEffect`. Storing derived state is the #1 cause of stale-data bugs.

```tsx
// WRONG — derived state synced via effect (stale, extra render, bug factory):
const [items, setItems] = useState<Item[]>([])
const [total, setTotal] = useState(0)
useEffect(() => { setTotal(items.reduce((s, i) => s + i.price, 0)) }, [items])

// RIGHT — derive during render:
const [items, setItems] = useState<Item[]>([])
const total = items.reduce((s, i) => s + i.price, 0)
// If the computation is genuinely expensive AND profiled as a bottleneck, wrap it:
const total = useMemo(() => items.reduce((s, i) => s + i.price, 0), [items])
```

- Same rule for "filtered list", "is form valid", "selected item object from selectedId". Compute inline.
- The only time you "store" derived-looking state is when you intentionally want a SNAPSHOT that should NOT update when the source changes (rare — name it like a snapshot).

PITFALLS: `useEffect` whose only job is to call `setState` from other state/props — delete it and compute inline. Two pieces of state that must "stay in sync" — collapse them into one + a derivation.

## useEffect: dependency arrays, cleanup, and not lying to React

- **The dependency array must list EVERY reactive value used inside the effect** (props, state, context, and functions/objects defined in the component). Don't silence the `react-hooks/exhaustive-deps` lint by omitting deps — that hides real bugs. Keep the lint ON.
- `[]` = run once after mount (and cleanup on unmount). No array = run after *every* render (almost always wrong).
- **Always return a cleanup function** for anything that outlives the render: subscriptions, timers, event listeners, websockets, `AbortController`. React calls cleanup before re-running the effect and on unmount.

```tsx
useEffect(() => {
  const id = setInterval(() => tick(), 1000)
  return () => clearInterval(id)          // cleanup — prevents leaks & double timers
}, [tick])

useEffect(() => {
  const ctrl = new AbortController()
  fetch(`/api/users/${userId}`, { signal: ctrl.signal })
    .then((r) => r.json()).then(setUser)
    .catch((e) => { if (e.name !== 'AbortError') setError(e) })
  return () => ctrl.abort()               // cancel in-flight request on userId change/unmount
}, [userId])
```

- **Infinite loops** come from: setting state in an effect with no/incorrect deps; depending on an object/array/function that's re-created every render. Fix the dep, don't remove it. Memoize the dependency (`useMemo`/`useCallback`) or move it out of the component, or hoist the value out of state.
- **React 18 StrictMode runs effects twice in dev** on purpose — to surface missing cleanup. If your effect breaks when run twice, the effect is buggy. Don't "fix" it by disabling StrictMode.

PITFALLS: missing cleanup (duplicate listeners, leaked timers, race conditions); object/function deps causing loops; using a ref to "cheat" the deps instead of fixing the real dependency.

## When NOT to use useEffect (the #1 React mistake)

Most `useEffect`s are wrong. Effects are for synchronizing with EXTERNAL systems (DOM APIs, network, timers, non-React libs). They are NOT for reacting to renders. Before writing an effect, check this list:

- **Transforming data for render?** → compute during render. No effect.
- **Caching an expensive calc?** → `useMemo`. No effect.
- **Resetting state when a prop changes?** → give the component a `key` so React remounts it, or compute during render. Usually no effect.
- **Updating state in response to a user event?** → do it in the event handler, not an effect. (e.g. POST on submit belongs in `onSubmit`, not in an effect watching form state.)
- **Fetching data?** → in App Router use Server Components or a route loader; on the client use TanStack Query. Raw `useEffect` fetching is the fallback of last resort and must handle race conditions + cleanup.
- **Sharing logic between handlers?** → extract a plain function and call it from each handler.

```tsx
// WRONG — event logic shoved into an effect:
useEffect(() => { if (submitted) postOrder(order) }, [submitted])
// RIGHT — do it where the event happens:
function handleSubmit() { postOrder(order) }

// WRONG — reset on prop change via effect:
useEffect(() => { setComment('') }, [postId])
// RIGHT — remount with a key (state resets automatically):
<CommentBox key={postId} />
```

PITFALLS: "chains" of effects that each set state and trigger the next — collapse into one render-time computation or one event handler. Effects that mirror props into state. Effects with a setState and the source value in deps.

## Data fetching patterns (client) — don't hand-roll useEffect fetching

Default to **TanStack Query** for client-side server state. It gives you caching, dedup, loading/error states, retries, refetch-on-focus, and pagination for free — all the things people get wrong by hand.

```tsx
import { useQuery } from '@tanstack/react-query'
function UserCard({ id }: { id: string }) {
  const { data, isPending, isError, error } = useQuery({
    queryKey: ['user', id],
    queryFn: ({ signal }) => fetch(`/api/users/${id}`, { signal }).then((r) => {
      if (!r.ok) throw new Error('Failed to load user'); return r.json()
    }),
  })
  if (isPending) return <Spinner />
  if (isError) return <ErrorBox message={error.message} />
  return <div>{data.name}</div>
}
```

If you MUST use raw `useEffect` (no library available): always (1) track loading + error + data, (2) use `AbortController` to cancel, (3) guard against out-of-order responses (a stale request resolving after a newer one). The `key`/`signal` pattern above handles this.

PITFALLS: no loading state (UI flashes empty then pops), no error state (silent failure), not cancelling on unmount (setState-on-unmounted warning + race), refetching the same data in 5 components instead of sharing one query, putting fetched data into a global store and manually invalidating it.

## Next.js App Router: Server vs Client Components

**Default to Server Components.** In the App Router every component is a Server Component unless the file (or an imported file) starts with `"use client"`. Server Components render on the server, ship ZERO JS for themselves, can be `async`, and can read the DB/secrets directly.

Add `"use client"` ONLY when the component needs:
- interactivity / event handlers (`onClick`, `onChange`)
- React state or lifecycle (`useState`, `useEffect`, `useReducer`)
- browser-only APIs (`window`, `localStorage`, `IntersectionObserver`)
- Context providers/consumers, or client-only libs

```tsx
// app/users/page.tsx — Server Component (no "use client"): fetch on the server, no client JS.
export default async function UsersPage() {
  const users = await db.user.findMany()      // direct DB access, runs on server only
  return <UserList users={users} />            // pass plain data down
}

// app/users/like-button.tsx
'use client'                                   // needed: it has state + onClick
import { useState } from 'react'
export function LikeButton() {
  const [liked, setLiked] = useState(false)
  return <button onClick={() => setLiked((v) => !v)}>{liked ? '♥' : '♡'}</button>
}
```

- **Push `"use client"` DOWN the tree (to the leaves).** Don't mark a whole page client just to make one button interactive. A Server Component can import and render Client Components, but a Client Component cannot import a Server Component (it can receive one via `children`).
- **Never put secrets / API keys / DB calls in a `"use client"` file** — that code ships to the browser. Server-only secrets stay in Server Components, Route Handlers, or Server Actions.
- You **cannot** pass functions (non-Server-Action), class instances, or Dates-with-methods as props from Server → Client. Pass serializable data (strings, numbers, plain objects, arrays).
- Hooks (`useState`/`useEffect`) and `onClick` in a file with no `"use client"` → build error. That error means "add the directive or move the interactivity to a leaf."

## Next.js App Router: data fetching, caching, and Route Handlers

- **Fetch in Server Components with `await fetch(...)`** (or your DB client). Next dedupes identical `fetch` calls in one render pass automatically.
- **Control caching explicitly** so you don't serve stale or accidentally-dynamic data:
  - `fetch(url)` — cached by default in older defaults; in Next 15 fetches are uncached by default. Be explicit.
  - `fetch(url, { cache: 'force-cache' })` — static, cached.
  - `fetch(url, { cache: 'no-store' })` — always fresh, per request (user-specific data, dashboards).
  - `fetch(url, { next: { revalidate: 60 } })` — ISR: cache for 60s then revalidate.
  - `next: { tags: ['posts'] }` + `revalidateTag('posts')` in a mutation — on-demand invalidation.

```tsx
// Parallel fetches: kick both off, then await — don't await sequentially (waterfall).
export default async function Dashboard() {
  const usersP = fetch('/api/users', { next: { revalidate: 60 } }).then((r) => r.json())
  const statsP = fetch('/api/stats', { cache: 'no-store' }).then((r) => r.json())
  const [users, stats] = await Promise.all([usersP, statsP])   // RIGHT
  return <Layout users={users} stats={stats} />
}
```

- **Route Handlers** (`app/api/.../route.ts`) — build a JSON API or webhook when an *external* client (mobile app, third party) needs it, or for client-side `fetch` targets. For internal Server→Server data you usually DON'T need a route handler — call the DB directly in the Server Component.
- **Mutations** (form submit, create/update/delete) → prefer **Server Actions** over hand-written API routes for app-internal writes; they're typesafe and co-located.

```ts
// app/api/posts/route.ts
import { NextResponse } from 'next/server'
export async function GET() {
  const posts = await db.post.findMany()
  return NextResponse.json(posts)
}
export async function POST(req: Request) {
  const body = await req.json()
  const post = await db.post.create({ data: body })
  return NextResponse.json(post, { status: 201 })
}
```

PITFALLS: sequential `await` waterfalls; forgetting `cache: 'no-store'` on per-user data (users see each other's data); building API routes for internal calls the server could make directly; doing DB work in a Client Component via an exposed key.

## Next.js App Router: layout, loading, error, and not-found files

These special files in `app/` are the idiomatic way to handle shell, suspense, and errors — use them instead of ad-hoc state.

- **`layout.tsx`** — shared shell (nav, footer) that WRAPS children and **persists across navigation without re-rendering**. The root layout must render `<html>` and `<body>`. Don't put per-page data that should refresh here.
- **`page.tsx`** — the route's unique UI.
- **`loading.tsx`** — instant loading UI shown via Suspense while the segment's Server Component awaits data. Free streaming skeletons. Use this instead of manual `isLoading` for route-level fetches.
- **`error.tsx`** — MUST be `"use client"`; receives `{ error, reset }`. Catches render/runtime errors in that segment and below. Provide a retry via `reset()`.
- **`not-found.tsx`** — rendered when you call `notFound()` or hit an unmatched route.
- **`template.tsx`** — like layout but re-mounts on navigation (use when you need enter animations or per-nav state reset).

```tsx
// app/dashboard/loading.tsx — shown automatically while page.tsx awaits.
export default function Loading() { return <DashboardSkeleton /> }

// app/dashboard/error.tsx
'use client'
export default function Error({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <div role="alert">
      <p>Something went wrong: {error.message}</p>
      <button onClick={reset}>Try again</button>
    </div>
  )
}
```

PITFALLS: forgetting `"use client"` on `error.tsx` (build error); putting refreshable data in `layout.tsx` and wondering why it's stale; not adding `loading.tsx` so the whole route blocks instead of streaming; missing a root-level `error.tsx`/`not-found.tsx` so failures show an ugly default.

## Performance: keys in lists (do this right or get subtle bugs)

- **Every item in a `.map()` needs a stable, unique `key`.** Use a real ID from the data (`item.id`), not the array index.
- **Index as key is a real bug**, not a style nit: when the list reorders, inserts, or deletes, React reuses the wrong DOM/state. Inputs keep the previous row's value, animations glitch, checkboxes check the wrong item.
- Keys must be **unique among siblings** and **stable across renders**. Never use `Math.random()` or `Date.now()` as a key — it forces a full remount every render (kills perf + loses focus/state).
- Index-as-key is only acceptable when the list is static, never reordered, never filtered, and items have no state. When unsure, use a stable id.

```tsx
{users.map((u) => <Row key={u.id} user={u} />)}          // RIGHT
{users.map((u, i) => <Row key={i} user={u} />)}          // WRONG if list ever changes order/size
{users.map((u) => <Row key={Math.random()} user={u} />)} // NEVER
```

## Performance: memo / useMemo / useCallback — when they help vs cargo-cult

Don't wrap everything. Memoization has a cost (memory + comparison) and most components are fast. Reach for it deliberately:

- **`React.memo(Component)`** — skip re-render when props are shallow-equal. Helps ONLY if the component is expensive to render OR re-renders often with the same props. Useless if its props are new objects/functions every time (see below).
- **`useMemo(fn, deps)`** — cache an expensive computation, or a referential value (object/array) you pass to a memoized child or a hook dep array. Don't memo cheap arithmetic.
- **`useCallback(fn, deps)`** — cache a function identity so a `React.memo` child or an effect dep doesn't see a "new" function each render. Pointless unless something downstream depends on that identity.

```tsx
// memo child + stable callback/value = child actually skips re-renders.
const Child = React.memo(function Child({ data, onPick }: Props) { /* expensive */ })
function Parent({ rows }: { rows: Row[] }) {
  const sorted = useMemo(() => [...rows].sort(byName), [rows])     // stable array identity
  const onPick = useCallback((id: string) => select(id), [])      // stable fn identity
  return <Child data={sorted} onPick={onPick} />
}
// Without the memo/useCallback above, Child gets new props every render and React.memo does nothing.
```

- **React 19 + the React Compiler** auto-memoizes; if it's enabled, hand-written `useMemo`/`useCallback` are largely unnecessary. Don't fight the compiler.
- The cure for re-render storms is usually **better state placement** (move state down, split components, select store slices) — not blanket memoization.

PITFALLS: `useCallback`/`useMemo` everywhere "for performance" with no measurement (adds noise + cost); `React.memo` on a component whose props are fresh objects each render (does nothing); memoizing to "fix" a bug that's really a missing/incorrect dep.

## Performance: code splitting, dynamic import, and image optimization

- **Code-split heavy / below-the-fold / rarely-used components** with `next/dynamic` (or `React.lazy` + `Suspense`). Charts, rich text editors, modals, maps are prime candidates.
- Use `ssr: false` for client-only libs (those touching `window`).

```tsx
import dynamic from 'next/dynamic'
const Chart = dynamic(() => import('@/components/Chart'), {
  loading: () => <Skeleton className="h-64" />,
  ssr: false,                    // only if the lib needs the browser
})
```

- **Images: use `next/image`**, not raw `<img>`. It lazy-loads, sizes responsively, and prevents layout shift. Always set `width`/`height` (or `fill` + a sized parent) and a meaningful `alt`. Use `priority` on the LCP/hero image only.

```tsx
import Image from 'next/image'
<Image src="/hero.jpg" alt="Product hero" width={1200} height={630} priority />
```

- **Avoid re-render storms:** colocate state with its users, split big components, pass JSX as `children` (children don't re-render when the parent's state changes), and select narrow store slices.
- Defer non-critical work: `next/font` for fonts (no layout shift), don't import a 200KB icon set to use 3 icons (import individually / tree-shakeable set).

PITFALLS: raw `<img>` (no lazy-load, CLS); no `width`/`height` (layout shift tanks CLS); `priority` on every image (defeats lazy-load); shipping a whole charting lib in the initial bundle.

## Forms: controlled vs uncontrolled, and react-hook-form + zod (the default)

- **Controlled** = React state is the source of truth (`value` + `onChange`). Use for live validation, dependent fields, formatting-as-you-type. Cost: a render per keystroke (fine for small forms).
- **Uncontrolled** = the DOM holds the value; you read it on submit (via refs / `FormData`). Cheaper, fewer renders. Good for simple forms and file inputs (file inputs are *always* uncontrolled).
- **Default for any non-trivial form: `react-hook-form` + `zod`.** RHF keeps inputs largely uncontrolled (fast, fewer renders) and zod gives you one schema for validation + inferred TypeScript types.

```tsx
'use client'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'

const schema = z.object({
  email: z.string().email('Enter a valid email'),
  age: z.coerce.number().int().min(18, 'Must be 18+'),
})
type FormData = z.infer<typeof schema>

export function SignupForm() {
  const { register, handleSubmit, formState: { errors, isSubmitting } } =
    useForm<FormData>({ resolver: zodResolver(schema) })

  async function onSubmit(data: FormData) { await createUser(data) }

  return (
    <form onSubmit={handleSubmit(onSubmit)} noValidate>
      <label htmlFor="email">Email</label>
      <input id="email" type="email" {...register('email')}
             aria-invalid={!!errors.email} aria-describedby="email-err" />
      {errors.email && <p id="email-err" role="alert">{errors.email.message}</p>}

      <button type="submit" disabled={isSubmitting}>
        {isSubmitting ? 'Creating…' : 'Sign up'}
      </button>
    </form>
  )
}
```

- **Validate on the server too** (Server Action / route handler) with the *same* zod schema — client validation is UX, never trust it for security.
- Disable the submit button while `isSubmitting` to prevent double-submits.

PITFALLS: a `useState` per field + manual validation (verbose, buggy) when RHF+zod exists; controlling a giant form (perf); trusting client validation as the only check; `value` without `onChange` (React warns: read-only input).

## Forms & accessibility basics (do these every time)

- **Every input has a label.** Use `<label htmlFor="id">` tied to the input's `id`, OR wrap the input in the `<label>`. A `placeholder` is NOT a label (it disappears on type, fails a11y).
- **Use the right element:** `<button>` for actions (never a clickable `<div>`), `<a href>` for navigation, real `<input type=...>` for inputs. Native elements are keyboard- and screen-reader-accessible for free.
- **Buttons inside forms:** set `type="submit"` for the submit button and `type="button"` for everything else (a bare `<button>` in a form defaults to submit and will reload/submit unexpectedly).
- **Error messaging:** mark invalid fields with `aria-invalid`, link the message with `aria-describedby`, and give the error `role="alert"` so screen readers announce it.
- **Focus & keyboard:** don't remove focus outlines without replacing them (`focus-visible:` styles). Ensure Tab order is logical. Interactive things must be reachable and operable by keyboard.
- **Images:** meaningful `alt`; decorative images get `alt=""`.
- **Color contrast** ≥ 4.5:1 for body text. Never convey state by color alone (add an icon/text).

PITFALLS: `<div onClick>` instead of `<button>` (no keyboard, no role); placeholder-as-label; `outline: none` with no replacement; missing `htmlFor`/`id` pairing; icon-only buttons with no `aria-label`.

## CSS: Tailwind idioms (utility-first, done cleanly)

- **Compose utilities in `className`; don't write custom CSS** until you genuinely need it. Order roughly: layout → box model → typography → color → state. Keep it scannable.
- **Use the design scale**, not magic numbers: `p-4`, `gap-6`, `text-sm`. Avoid arbitrary values like `mt-[13px]` unless matching a fixed asset — arbitrary values everywhere = you've abandoned the system.
- **Extract a COMPONENT, not a `@apply` soup.** Repeated class strings → make a React component (`<Button variant="primary">`), optionally with `cva` (class-variance-authority) for variants. `@apply` is a last resort.
- **Merge conditional classes** with `clsx`/`cn` (and `tailwind-merge` to resolve conflicts) — don't build className strings with template literals + ternaries by hand.

```tsx
import { clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'
const cn = (...a: any[]) => twMerge(clsx(a))

function Button({ variant = 'primary', className, ...props }: Props) {
  return (
    <button
      className={cn(
        'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium',
        'focus-visible:outline-none focus-visible:ring-2 disabled:opacity-50',
        variant === 'primary' && 'bg-blue-600 text-white hover:bg-blue-700',
        variant === 'ghost' && 'bg-transparent hover:bg-gray-100',
        className,                      // let callers override; twMerge resolves conflicts
      )}
      {...props}
    />
  )
}
```

PITFALLS: arbitrary-value soup (`p-[7px] mt-[13px]`); duplicating long class strings across files instead of a component; string-concatenating classNames (conflicts win unpredictably without `tailwind-merge`); inline `style={{}}` for things Tailwind already covers.

## CSS: Flexbox vs Grid, and centering without magic numbers

- **Flexbox = one dimension** (a row OR a column). Use for: navbars, toolbars, button groups, stacking items, distributing space along one axis. `flex`, `items-center`, `justify-between`, `gap-4`.
- **Grid = two dimensions** (rows AND columns together) or any defined layout. Use for: page layouts, card galleries, dashboards, form grids, anything where you want columns to line up. `grid grid-cols-3 gap-4`.
- **Centering:** flex parent → `flex items-center justify-center`. Grid → `grid place-items-center`. Don't reach for absolute positioning + negative margins/transforms to center; that's the old hack.
- **Gaps over margins between siblings:** use `gap-*` on the flex/grid container instead of `mr-*`/`mb-*` on each child (no leftover trailing margin, cleaner).
- Responsive columns the idiomatic way: `grid-cols-1 md:grid-cols-2 lg:grid-cols-3`. For auto-fitting cards without breakpoints: `grid-cols-[repeat(auto-fill,minmax(16rem,1fr))]`.

```tsx
// Card grid: responsive, gap-based, no magic numbers.
<div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
  {cards.map((c) => <Card key={c.id} {...c} />)}
</div>
// Toolbar: one-dimensional → flexbox.
<div className="flex items-center justify-between gap-4">
  <h1 className="text-lg font-semibold">Title</h1>
  <Button>Action</Button>
</div>
```

PITFALLS: using flexbox to fake a grid (wrapping + manual widths) when CSS Grid is one line; absolute-positioning for layout that flow/grid handles; per-child margins instead of container `gap`; fixed pixel widths that break on mobile (use `min()`/`max()`/`fr`/percentages).

## CSS: responsive breakpoints, mobile-first, and dark mode

- **Mobile-first:** unprefixed utilities are the base (smallest screen); add `sm: md: lg: xl: 2xl:` to layer on LARGER screens. `min-width` breakpoints stack upward. Don't write desktop styles then undo them for mobile.
- Default Tailwind breakpoints: `sm 640`, `md 768`, `lg 1024`, `xl 1280`, `2xl 1536`. Use them; don't invent random ones unless a design demands it.
- **Test the real small viewport** (~360–390px). Common breaks: fixed widths, long unbroken strings (`break-words`/`truncate`), horizontal overflow, tap targets < 44px.

```tsx
<div className="flex flex-col gap-4 md:flex-row md:items-center">…</div>  {/* stack on mobile, row on md+ */}
<h1 className="text-2xl md:text-4xl">…</h1>
```

- **Dark mode:** add `dark:` variants and toggle the `dark` class on `<html>` so YOU control it (theme switch + system default), not just `prefers-color-scheme`. How you enable that toggle depends on the Tailwind major, and the wrong one fails silently (no build error — `dark:` simply never matches):
  - **Tailwind v4** — there is no `darkMode` option and `tailwind.config.js` is not read by default. Declare the variant in your CSS next to `@import "tailwindcss"`: `@custom-variant dark (&:where(.dark, .dark *));`. Theme tokens go in an `@theme { … }` block in the same file, not in a JS config.
  - **Tailwind v3 (legacy projects)** — `darkMode: 'class'` (or `'selector'`) in `tailwind.config.js`, tokens under `theme.extend`.
  - Tell which you're on before writing either: `@import "tailwindcss"` in the CSS ⇒ v4; `@tailwind base;` ⇒ v3. Check `package.json` if the CSS is ambiguous.
- Theme via design tokens: prefer semantic colors (`bg-background text-foreground`, often CSS variables) over hardcoding `bg-white dark:bg-gray-900` in 50 places. shadcn/ui sets this up well.

```tsx
<div className="bg-white text-gray-900 dark:bg-gray-900 dark:text-gray-100">…</div>
// Better at scale: <div className="bg-background text-foreground"> with tokens for both themes.
```

PITFALLS: desktop-first then fighting it with `max-` overrides; hardcoding light colors everywhere then sprinkling `dark:` retroactively; forgetting to flip `dark` on `<html>` (so `dark:` never triggers); a dark-mode flash on load (set the class before paint via an inline script or framework theme provider).

## Loading, error, empty, and optimistic UI states (never ship the happy path only)

Amateur UIs render only the success case. Every async surface needs FOUR states. Design them explicitly.

- **Loading:** skeletons (preferred — preserve layout, no shift) or a spinner. In App Router use `loading.tsx`/`<Suspense>`; on the client use the lib's `isPending`.
- **Error:** show a human message + a retry. Use `error.tsx` for route errors; an Error Boundary for client subtrees; `isError` for queries. Never swallow errors silently.
- **Empty:** when data loads but is empty, render a real empty state (icon + message + primary action), not a blank screen or a perpetual spinner.
- **Optimistic UI:** for mutations that almost always succeed (like, toggle, add-to-list), update the UI immediately and roll back on error. Use `useOptimistic` (React 19), TanStack Query's optimistic updates, or manual rollback.

```tsx
// React 19 optimistic add — UI updates instantly, reconciles when the server responds.
'use client'
import { useOptimistic } from 'react'
function Todos({ todos, addTodo }: Props) {
  const [optimistic, addOptimistic] = useOptimistic(
    todos, (state, newText: string) => [...state, { id: 'temp', text: newText, pending: true }],
  )
  async function action(formData: FormData) {
    const text = formData.get('text') as string
    addOptimistic(text)              // instant
    await addTodo(text)              // server; list reconciles on success/error
  }
  return <form action={action}>{/* … */}{optimistic.map(/* … */)}</form>
}
```

PITFALLS: showing nothing while loading (looks broken/blank); a spinner that spins forever on error; treating empty as loading; mutations that feel laggy because the UI waits for the round-trip; optimistic updates with no rollback on failure (UI lies).

## Amateur-code checklist (review every diff against this)

The fastest tells that code was written by a weak model. Fix all of these:

- **Index as `key`** in a dynamic list → use a stable `item.id`. `Math.random()`/`Date.now()` keys → never.
- **Mutating state directly:** `state.push(x)`, `obj.foo = 1`, `arr.sort()` on state, then `setState(arr)`. React state is immutable — create new references: `setItems([...items, x])`, `setUser({ ...user, name })`, `setItems(items.map(...))`, `setItems(items.filter(...))`. Mutation → stale renders + skipped updates.
- **Fetching in render / unconditionally re-fetching:** calling `fetch` directly in the component body, or an effect with bad deps that loops. Use Server Components / TanStack Query.
- **Missing loading & error states** (and missing empty state) — see the four-states section.
- **No optimistic UI** on common mutations → laggy feel.
- **Hardcoded values** that should be config/props/env: API base URLs, magic numbers, copy strings, colors. Use `process.env.NEXT_PUBLIC_*` for public config, props for variants, design tokens for colors.
- **Derived state in `useState`** synced by `useEffect` → compute during render.
- **`useEffect` for event logic** → move to the handler. **`useEffect` with missing deps / lint disabled** → fix the deps.
- **Secrets in client code** (`"use client"` files, `NEXT_PUBLIC_` for private keys) → server-only.
- **Marking a whole page `"use client"`** to make one button interactive → push the directive to the leaf.
- **`<div onClick>`** instead of `<button>`; inputs with no `<label>`; removed focus outlines.
- **Raw `<img>`** instead of `next/image`; missing `width`/`height` (layout shift).
- **`any` everywhere / ignored TS errors** → type props and API responses; infer form types from zod.
- **Cargo-cult `useMemo`/`useCallback`/`memo`** with no measurement, or where props are fresh each render so memo does nothing.
- **`async` directly on a Client Component function** (`'use client'` + `export default async function`) → not allowed; fetch in a Server Component or use a query hook.
- **Sequential `await` waterfalls** when fetches are independent → `Promise.all`.
- **Forgetting `cache: 'no-store'`** on per-user server fetches → cross-user data leaks.
