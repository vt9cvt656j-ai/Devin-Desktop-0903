# Web Frontend (Vue 3 / Nuxt 3)

> Opinionated, battle-tested defaults for building Vue 3 + Nuxt 3 frontends. When in doubt, follow the RIGHT DEFAULT given here. Assume Vue 3.5+, Nuxt 4, Vite, TypeScript, and `<script setup>` everywhere unless told otherwise (Nuxt 3.1x only when you've confirmed the project is on it — Nuxt 4 moved app code under `app/`, so check the directory layout and `package.json` before writing paths).

## Composition API: ref vs reactive, computed, watch — the decision tree

Use `<script setup>` exclusively. The Options API is legacy; never mix the two in one project.

- **`ref()`** — the default for almost everything. Primitives, objects, arrays, DOM template refs. Access via `.value` in script, auto-unwrapped in templates. Use multiple `ref()` calls over one giant reactive object.
- **`reactive()`** — when you have a cohesive object where every field is always accessed together (a form model, a complex config). You lose the ability to reassign the whole object and destructuring breaks reactivity. Prefer `ref()` unless you have a clear reason.
- **`computed()`** — derived state. If a value can be calculated from other refs/reactive, use `computed()`. It caches and only re-evaluates when deps change. This is the Vue equivalent of "compute, don't store."
- **`watch(source, cb)`** — react to a specific ref/getter change. Use for side effects (API calls, logging, syncing to localStorage). Always specify `{ immediate: true }` if you need it to run on mount.
- **`watchEffect(cb)`** — auto-tracks every reactive dependency used inside the callback. Use for side effects where you don't care about old/new values. Runs immediately by default.

```vue
<script setup lang="ts">
import { ref, computed, watch, watchEffect } from 'vue'

const count = ref(0)
const user = ref<User | null>(null)
const doubled = computed(() => count.value * 2)       // derived state, cached

watch(() => user.value?.id, (id) => {                 // react to specific source
  if (id) fetchProfile(id)
}, { immediate: true })

watchEffect(() => { document.title = `Count: ${count.value}` })  // auto-tracks deps
</script>
```

`watch` vs `watchEffect`: use `watch` when you need old/new values or want to react to one specific source. Use `watchEffect` for "run this effect whenever any of its deps change."

PITFALLS: using `reactive()` for a primitive (it only works on objects); destructuring a `reactive()` object and losing reactivity (`const { name } = reactive({name: 'x'})` is dead); forgetting `.value` in script (template auto-unwraps, script does not); a `computed()` with side effects (it should be pure — use `watch` for side effects); `watchEffect` that accidentally tracks too many deps causing spurious re-runs.

## State management: ref/reactive -> Pinia -> never Vuex

Pick the SMALLEST tool that fits. Escalate only when you hit the next problem.

- **`ref()` / `reactive()`** — default for component-local state. Multiple refs over one blob.
- **Composables** — when 2-3 components share the same logic + state, extract a composable (`useSomething()`). If the state should be shared (singleton), use a module-level ref inside the composable.
- **Pinia** — when many components across the tree read/write shared mutable state and you need devtools, SSR hydration, or persistence. Pinia is the official Vue store. It replaces Vuex entirely.
- **NEVER use Vuex** in new projects. Vuex is legacy. Pinia has better TS support, simpler API, no mutations boilerplate, and is the officially recommended store for Vue 3.
- **Server state != client state.** Data from an API belongs in `useFetch`/`useAsyncData` (Nuxt) or TanStack Query (Vue SPA) — NOT in Pinia. Putting fetched entities in a global store and manually invalidating is the same amateur smell as in React.

```ts
// stores/counter.ts — Pinia setup syntax (preferred over options syntax)
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useCounterStore = defineStore('counter', () => {
  const count = ref(0)
  const doubled = computed(() => count.value * 2)
  function increment() { count.value++ }
  function reset() { count.value = 0 }
  return { count, doubled, increment, reset }
})

// In a component — use storeToRefs for reactive destructuring:
import { storeToRefs } from 'pinia'
const store = useCounterStore()
const { count, doubled } = storeToRefs(store) // reactive
const { increment, reset } = store            // methods don't need storeToRefs
```

PITFALLS: reaching for Pinia on day one for a 3-component app (a composable is simpler); destructuring a store without `storeToRefs` (loses reactivity); putting fetched data in Pinia and hand-managing cache invalidation; using Vuex in 2024+ (Pinia is strictly better).

## Props, emits, v-model on components, provide/inject

```vue
<script setup lang="ts">
// Props — type-based declaration (preferred). Use withDefaults for optional defaults.
const props = withDefaults(defineProps<{
  title: string
  count?: number
  items: Item[]
}>(), { count: 0, items: () => [] })

// Emits — type-based
const emit = defineEmits<{ update: [value: string]; delete: [id: number] }>()
</script>
```

**v-model on components** — the clean way to do two-way binding:

```vue
<!-- Parent: <SearchInput v-model="query" /> -->
<!-- is shorthand for: <SearchInput :modelValue="query" @update:modelValue="query = $event" /> -->

<!-- SearchInput.vue -->
<script setup lang="ts">
const model = defineModel<string>()  // Vue 3.4+ — the simplest way
</script>
<template>
  <input :value="model" @input="model = ($event.target as HTMLInputElement).value" />
</template>

<!-- Multiple v-model bindings: <UserForm v-model:first="first" v-model:last="last" /> -->
<script setup lang="ts">
const first = defineModel<string>('first')
const last = defineModel<string>('last')
</script>
```

**provide/inject** — dependency injection for deep trees. Use for theme, config, or service instances. NOT for general state management. Always `provide` a `ref` (not a raw value) so descendants get reactive data. Use `InjectionKey<T>` for type-safe keys across files.

PITFALLS: mutating props directly (Vue warns — emit an event or use `v-model` instead); forgetting to type emits (loses TS safety); `provide` with a raw value instead of a `ref` (descendants get a snapshot, not reactive data); using provide/inject as a state management replacement (it has no devtools, no structure).

## Component patterns: SFCs, slots, dynamic & async components

**Single-file components (SFCs)** are the unit. Always `<script setup lang="ts">` + `<template>` + optional `<style scoped>`. One component per file.

**Slots** — Vue's composition primitive (like React's `children` but more powerful):

```vue
<!-- Card.vue -->
<template>
  <div class="card">
    <div class="card-header">
      <slot name="header">Default Header</slot>   <!-- named slot with fallback -->
    </div>
    <slot />                                        <!-- default slot -->
    <div class="card-footer">
      <slot name="footer" :count="items.length" /> <!-- scoped slot: passes data up -->
    </div>
  </div>
</template>

<!-- Usage -->
<Card>
  <template #header><h2>My Card</h2></template>
  <p>Body content goes in the default slot</p>
  <template #footer="{ count }">{{ count }} items</template>
</Card>
```

**Dynamic components** — render based on a variable:

```vue
<component :is="currentTab" />
<!-- currentTab is a component reference, not a string -->
```

**Async components** — code-split heavy components:

```ts
import { defineAsyncComponent } from 'vue'
const HeavyChart = defineAsyncComponent(() => import('./HeavyChart.vue'))
// Renders lazily; wrap in <Suspense> for loading state
```

PITFALLS: using `v-if`/`v-else` chains for 5+ variants when `<component :is>` is cleaner; slot prop drilling (pass data via scoped slots, not through 3 layers); forgetting fallback content in slots; not wrapping async components in `<Suspense>` (no loading UI).

## Vue Router 4: routes, guards, lazy loading, nested routes

```ts
// router/index.ts
import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: () => import('@/pages/Home.vue') },
    {
      path: '/users/:id',
      component: () => import('@/pages/UserLayout.vue'),  // lazy-loaded
      children: [
        { path: '', component: () => import('@/pages/UserProfile.vue') },
        { path: 'posts', component: () => import('@/pages/UserPosts.vue') },
      ],
    },
    { path: '/:pathMatch(.*)*', component: () => import('@/pages/NotFound.vue') },
  ],
})

// Navigation guard — runs before every route change
router.beforeEach(async (to, from) => {
  const auth = useAuthStore()
  if (to.meta.requiresAuth && !auth.isLoggedIn) {
    return { path: '/login', query: { redirect: to.fullPath } }
  }
})

export default router
```

**Always lazy-load route components** with `() => import(...)`. Only the landing page might be eagerly loaded. This is free code splitting.

**Typed routes** — use `unplugin-vue-router` for auto-generated typed route names and params. Prevents typos in `router.push({ name: 'usr-profile' })`.

**In-component guards** with `<script setup>`:

```vue
<script setup lang="ts">
import { onBeforeRouteLeave } from 'vue-router'
onBeforeRouteLeave((to, from) => {
  if (hasUnsavedChanges.value) return confirm('Discard changes?')
})
</script>
```

PITFALLS: eagerly importing all route components (bloats initial bundle); putting auth logic in every component instead of a global `beforeEach` guard; forgetting the catch-all 404 route; accessing `route.params` without typing (stringly-typed params cause runtime bugs).

## Nuxt 3: auto-imports, file routing, server routes, data fetching

Nuxt 3 is the right default for SSR Vue apps. It auto-imports Vue APIs, composables, and components — no manual imports needed.

**File-based routing** — files in `pages/` become routes automatically:

```
pages/
  index.vue          -> /
  about.vue          -> /about
  users/
    index.vue        -> /users
    [id].vue         -> /users/:id
  [...slug].vue      -> catch-all
```

**Server routes** — files in `server/api/` become API endpoints:

```ts
// server/api/users/[id].get.ts — GET /api/users/:id
export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, 'id')
  const user = await db.user.findUnique({ where: { id } })
  if (!user) throw createError({ statusCode: 404, message: 'User not found' })
  return user
})

// server/api/users.post.ts — POST /api/users
export default defineEventHandler(async (event) => {
  const body = await readBody(event)
  return db.user.create({ data: body })
})
```

**Data fetching** — `useFetch` and `useAsyncData` are the core:

```vue
<script setup lang="ts">
// useFetch — shorthand for useAsyncData + $fetch
const { data: users, status, error, refresh } = await useFetch('/api/users')

// useAsyncData — when you need more control or non-fetch sources
const { data: user } = await useAsyncData(
  `user-${id}`,                                  // unique key for dedup + caching
  () => $fetch(`/api/users/${id}`)
)

// Lazy variant — doesn't block navigation (loads after page renders)
const { data, pending } = await useLazyFetch('/api/heavy-data')
</script>
<template>
  <div v-if="status === 'pending'">Loading...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else-if="!users?.length">No users found</div>
  <ul v-else>
    <li v-for="user in users" :key="user.id">{{ user.name }}</li>
  </ul>
</template>
```

**Middleware** — route guards in `middleware/`. Named: `middleware/auth.ts` + `definePageMeta({ middleware: 'auth' })`. Global: suffix with `.global.ts`. Return `navigateTo('/login')` to redirect.

**Layouts** — files in `layouts/` wrap pages. `layouts/default.vue` uses `<slot />` for page content. Switch layouts per page: `definePageMeta({ layout: 'admin' })`.

**Error handling** — `error.vue` at project root catches fatal errors (receives `error` prop with `statusCode`/`message`, call `clearError()` to recover). For non-fatal component errors, use `<NuxtErrorBoundary>` with `#error="{ error, clearError }"` scoped slot.

**SSR vs CSR:** Nuxt defaults to universal (SSR + client hydration). For specific pages that must be client-only: `definePageMeta({ ssr: false })`. For the entire app as an SPA: `nuxt.config.ts` with `ssr: false`. Prefer SSR for SEO, first-paint, and data fetching; use CSR for dashboards or auth-only pages.

PITFALLS: calling `useFetch` inside an event handler or `onMounted` (it must be top-level in `<script setup>` for SSR to work); forgetting the unique key in `useAsyncData` (causes cache collisions); not handling all 4 states (pending/error/empty/data); using `$fetch` directly in components instead of `useFetch` (loses SSR data serialization, causes double-fetch on hydration); putting secrets in `runtimeConfig.public` instead of `runtimeConfig` (public ships to client).

## Forms & validation: VeeValidate + zod

Default stack: **VeeValidate** for form state + **zod** for schema validation. VeeValidate integrates deeply with Vue's reactivity and handles dirty/touched/errors per field.

```vue
<script setup lang="ts">
import { useForm } from 'vee-validate'
import { toTypedSchema } from '@vee-validate/zod'
import { z } from 'zod'

const schema = toTypedSchema(z.object({
  email: z.string().email('Enter a valid email'),
  age: z.coerce.number().int().min(18, 'Must be 18+'),
}))

const { handleSubmit, errors, isSubmitting, defineField } = useForm({
  validationSchema: schema,
})

const [email, emailAttrs] = defineField('email')
const [age, ageAttrs] = defineField('age')

const onSubmit = handleSubmit(async (values) => {
  // values is typed as { email: string; age: number }
  await createUser(values)
})
</script>

<template>
  <form @submit="onSubmit" novalidate>
    <label for="email">Email</label>
    <input id="email" type="email" v-model="email" v-bind="emailAttrs"
           :aria-invalid="!!errors.email" />
    <span v-if="errors.email" role="alert">{{ errors.email }}</span>

    <label for="age">Age</label>
    <input id="age" type="number" v-model="age" v-bind="ageAttrs" />
    <span v-if="errors.age" role="alert">{{ errors.age }}</span>

    <button type="submit" :disabled="isSubmitting">
      {{ isSubmitting ? 'Creating...' : 'Sign up' }}
    </button>
  </form>
</template>
```

For simpler forms (login, search), raw `v-model` + manual validation is fine. Reach for VeeValidate when you have 3+ fields, cross-field validation, or dynamic field arrays.

**Validate on the server too** (Nuxt server route) with the same zod schema. Client validation is UX, not security.

PITFALLS: a `ref()` per field + manual `v-if` error checks when VeeValidate exists; not disabling submit during `isSubmitting` (double-submit); trusting client-only validation; using Yup over zod (zod has better TS inference and is lighter).

## CSS with Vue: scoped styles, modules, Tailwind, dynamic classes

**Scoped styles** are the default in SFCs. They add a `data-v-*` attribute so CSS only applies to that component:

```vue
<style scoped>
.card { padding: 1rem; }
/* Deep selector — reach into child component DOM: */
.card :deep(.child-class) { color: red; }
/* Slotted content: */
.card :slotted(.from-parent) { font-weight: bold; }
</style>
```

**CSS Modules** — `<style module>` gives guaranteed unique class names via `$style.card`. Use when scoped styles aren't strict enough.

**Tailwind + Vue** — the right default for styling:

```vue
<template>
  <button :class="[
    'px-4 py-2 rounded text-sm font-medium',
    { 'bg-blue-600 text-white': isPrimary, 'bg-gray-100': !isPrimary },
    disabled && 'opacity-50 cursor-not-allowed'
  ]">{{ label }}</button>
</template>
```

Use array + object syntax for dynamic classes (not template literal concatenation). For complex logic, use a `computed` returning an array of classes.

Use `clsx` or `tailwind-merge` when class conflict resolution matters (same as React). Extract repeated class combinations into Vue components, not `@apply` soup.

PITFALLS: using `:deep()` excessively (breaks encapsulation — redesign the component boundary); scoped styles that don't reach into third-party component DOM (you need `:deep()`); dynamic classes built with template literal string concatenation instead of array/object syntax (fragile and Tailwind purge can miss them); `@apply` for everything instead of extracting a component.

## Performance: async components, keep-alive, v-once, tree-shaking

**`defineAsyncComponent`** — lazy-load heavy components:

```ts
const HeavyEditor = defineAsyncComponent({
  loader: () => import('./HeavyEditor.vue'),
  loadingComponent: LoadingSpinner,
  delay: 200,     // show spinner after 200ms (avoids flash for fast loads)
  timeout: 10000,
})
```

**`<KeepAlive>`** — cache component instances when toggling with `v-if` or `<component :is>`. Use for tabs where you want to preserve scroll position and form state:

```vue
<KeepAlive :max="5">       <!-- cache at most 5 instances -->
  <component :is="currentTab" />
</KeepAlive>
```

**`v-once`** — render once, skip all future diffs. **`v-memo="[deps]"`** — memoize a template subtree (like React.memo for a DOM chunk). **Virtual scrolling** for 1000+ items: use `@tanstack/vue-virtual`. Never render 5000 DOM nodes.

**Tree-shaking:** Vue 3 is tree-shakeable by design. Unused APIs (`KeepAlive`, `Teleport`, `Transition`) are dropped if not imported. Use `manualChunks` in `vite.config.ts` to split vendor bundles. Nuxt handles Vite config and chunking automatically.

PITFALLS: `<KeepAlive>` without `:max` (memory leak — caches every instance forever); `v-once` on dynamic content (data never updates); rendering 5000 list items without virtual scrolling; importing the entire icon library instead of individual icons.

## Composables: extracting and sharing logic

A composable is a function that uses Vue's Composition API and follows the `use*` naming convention. It is the primary way to reuse stateful logic.

```ts
// composables/useMouse.ts
import { ref, onMounted, onUnmounted } from 'vue'

export function useMouse() {
  const x = ref(0)
  const y = ref(0)

  function update(e: MouseEvent) {
    x.value = e.pageX
    y.value = e.pageY
  }

  onMounted(() => window.addEventListener('mousemove', update))
  onUnmounted(() => window.removeEventListener('mousemove', update))

  return { x, y }  // always return refs, not raw values
}
```

**Singleton composable** — shared state across all callers by placing state at module level:

```ts
// composables/useAuth.ts
const currentUser = ref<User | null>(null)  // module-level = singleton
export function useAuth() {
  async function login(creds: Credentials) {
    currentUser.value = await $fetch('/api/login', { method: 'POST', body: creds })
  }
  const isLoggedIn = computed(() => currentUser.value !== null)
  return { currentUser: readonly(currentUser), isLoggedIn, login }
}
```

**Naming:** always `use` prefix. File names match: `composables/useAuth.ts` exports `useAuth()`. In Nuxt, composables in `composables/` are auto-imported.

**Accept `MaybeRef<T>`** for input params so callers can pass either a raw value or a ref. Use `toValue()` inside the composable to unwrap.

PITFALLS: returning raw values instead of refs (caller loses reactivity); side effects in composables without cleanup (`onUnmounted`); calling composables outside of `setup()` context (lifecycle hooks won't work); a composable that does too much (keep them focused — compose multiple small composables).

## TypeScript in Vue: typing props, emits, refs, generics

```vue
<script setup lang="ts">
// Typed props with defaults (factory fn for non-primitives)
const props = withDefaults(defineProps<{
  title: string; count?: number; items: Item[]
}>(), { count: 0, items: () => [] })

// Typed emits
const emit = defineEmits<{ submit: [data: FormData]; 'update:modelValue': [value: string] }>()

// Typed refs
const el = ref<HTMLInputElement | null>(null)  // template ref — always null-check
const users = ref<User[]>([])
</script>

<!-- Generic components (Vue 3.3+) -->
<script setup lang="ts" generic="T extends { id: string }">
defineProps<{ items: T[]; selected?: T }>()
defineEmits<{ select: [item: T] }>()
</script>
```

**Typing provide/inject** — use `InjectionKey<T>` from `vue` for type-safe keys: `export const ThemeKey: InjectionKey<Ref<'light'|'dark'>> = Symbol('theme')`. Provider calls `provide(ThemeKey, theme)`, consumer calls `inject(ThemeKey)` with automatic type inference.

PITFALLS: using runtime prop validation (`defineProps({ title: String })`) instead of type-based in TS projects (loses full type inference); forgetting factory functions for array/object defaults (`default: []` is shared across instances — use `() => []`); not null-checking template refs; `any` on event payloads.

## Testing: Vitest + Vue Test Utils

Default to **Vitest** (Vite-native, fast, Jest-compatible API) + **@vue/test-utils** for component tests.

```ts
// components/__tests__/Counter.test.ts
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import Counter from '../Counter.vue'

describe('Counter', () => {
  it('increments on click', async () => {
    const wrapper = mount(Counter, { props: { initial: 0 } })
    expect(wrapper.text()).toContain('0')
    await wrapper.find('button').trigger('click')
    expect(wrapper.text()).toContain('1')
  })

  it('emits update event', async () => {
    const wrapper = mount(Counter)
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('update')).toHaveLength(1)
    expect(wrapper.emitted('update')![0]).toEqual([1])
  })
})
```

**Testing Pinia stores:** call `setActivePinia(createPinia())` in `beforeEach`, then use the store directly. **Testing Nuxt pages:** use `@nuxt/test-utils` with `mountSuspended` for components with async setup.

PITFALLS: testing implementation details (checking internal refs instead of rendered output); not `await`-ing DOM updates after trigger (Vue batches updates); missing Pinia setup in tests (store won't work without `setActivePinia`); snapshot testing entire components (brittle, hard to review).

## Amateur-code checklist (review every diff against this)

The fastest tells that code was written by a weak model or junior dev. Fix all of these:

- **Options API in a Vue 3 project** — use `<script setup>` Composition API exclusively. No `data()`, `methods`, `computed:` as options.
- **Mutating props** — never `props.x = y`. Emit an event or use `defineModel`. Vue warns in dev but the code is wrong.
- **`reactive()` for a primitive** — `reactive(0)` doesn't work. Use `ref()`.
- **Destructuring `reactive()` and losing reactivity** — `const { name } = reactive(state)` is a dead snapshot. Use `toRefs()` or stick with `ref()`.
- **`v-if` + `v-for` on the same element** — `v-if` has higher priority in Vue 3 and won't access the `v-for` variable. Wrap with `<template v-for>` and put `v-if` inside.
- **Index as `:key`** in a dynamic list — use `item.id`. Same as React: index keys cause DOM reuse bugs on reorder/filter.
- **Missing `:key` on `v-for`** — always provide it. No key = Vue can't track items efficiently.
- **`this.` in `<script setup>`** — there is no `this`. It's a syntax error in mindset, not just code.
- **Vuex in a new Vue 3 project** — use Pinia.
- **`$fetch` in components instead of `useFetch`** (Nuxt) — causes double-fetch on SSR hydration. Use `useFetch` / `useAsyncData` at the top level of `<script setup>`.
- **`useFetch` inside `onMounted` or event handlers** — it must be called at the top level of `setup()` for SSR to work. Use `$fetch` for imperative calls in handlers.
- **Missing loading/error/empty states** — always handle all 4 (pending, error, empty, data). A `v-if="data"` with nothing for the other cases is incomplete.
- **Huge monolithic component (300+ lines in `<script setup>`)** — break it into composables and child components. Vue's strength is composition.
- **Not using auto-imports in Nuxt** — manually importing `ref`, `computed`, `useFetch` etc. from `vue` and `nuxt` in a Nuxt project. Nuxt auto-imports them.
- **Watchers that should be computed** — if you `watch(x, () => y.value = x.value * 2)`, that's a computed: `const y = computed(() => x.value * 2)`.
- **`any` everywhere / no TypeScript** — type your props, emits, store state, and API responses. Use `defineProps<T>()` not `defineProps({})`.
- **Global CSS without scoping** — use `<style scoped>`, CSS modules, or Tailwind. Unscoped global CSS leaks across components.
- **Sequential awaits in server routes** when fetches are independent — use `Promise.all`.
- **Secrets in `runtimeConfig.public`** — anything in `public` ships to the client. Private keys go in `runtimeConfig` (server-only).
