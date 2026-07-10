// Deterministic web scaffold — the v0.dev "curated starter" advantage.
// Instead of the agent typing a bare `npm create vite`, this lays down a
// runnable Vite + Vue 3 + Tailwind project whose design-token system, font
// pairing and base component classes are already curated — so every site
// starts from the same high-quality, non-AI-slop base. The agent then builds
// pages on top (npm install + npm run dev).

use std::path::{Path, PathBuf};
use tokio::fs;

fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "my-site".to_string()
    } else {
        s
    }
}

#[tauri::command]
pub async fn web_scaffold(
    name: String,
    workspace: String,
    framework: Option<String>,
) -> Result<serde_json::Value, String> {
    let proj = slug(&name);
    let fw = framework.unwrap_or_default().trim().to_lowercase();
    // Vue is the default per the house style; react is accepted but currently
    // shares the Vite+Tailwind+token base with a React entry.
    let is_react = matches!(
        fw.as_str(),
        "react" | "reactjs" | "react.js" | "jsx" | "tsx"
    );

    let root = PathBuf::from(&workspace);
    if !root.exists() {
        return Err(format!("工作区不存在: {workspace}"));
    }
    let dir = root.join(&proj);
    if dir.exists() {
        return Err(format!("目录已存在: {proj}（换个名字或先删除）"));
    }

    // ── write helper ────────────────────────────────────────────────
    async fn put(dir: &Path, rel: &str, content: &str) -> Result<(), String> {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("建目录失败 {rel}: {e}"))?;
        }
        fs::write(&p, content)
            .await
            .map_err(|e| format!("写入失败 {rel}: {e}"))
    }

    fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("创建项目目录失败: {e}"))?;

    // Files shared by both stacks
    put(&dir, "tailwind.config.js", TAILWIND_CONFIG).await?;
    put(&dir, "postcss.config.js", POSTCSS_CONFIG).await?;
    put(&dir, "src/style.css", STYLE_CSS).await?;
    put(&dir, ".gitignore", GITIGNORE).await?;
    put(&dir, "README.md", &readme(&proj)).await?;

    let mut files: Vec<&str> = vec![
        "tailwind.config.js",
        "postcss.config.js",
        "src/style.css",
        "README.md",
        ".gitignore",
    ];

    if is_react {
        put(&dir, "package.json", &pkg_json(&proj, true)).await?;
        put(&dir, "vite.config.js", VITE_CONFIG_REACT).await?;
        put(&dir, "index.html", &index_html(&proj, true)).await?;
        put(&dir, "src/main.jsx", MAIN_JSX).await?;
        put(&dir, "src/App.jsx", APP_JSX).await?;
        files.extend([
            "package.json",
            "vite.config.js",
            "index.html",
            "src/main.jsx",
            "src/App.jsx",
        ]);
    } else {
        put(&dir, "package.json", &pkg_json(&proj, false)).await?;
        put(&dir, "vite.config.js", VITE_CONFIG_VUE).await?;
        put(&dir, "index.html", &index_html(&proj, false)).await?;
        put(&dir, "src/main.js", MAIN_JS).await?;
        put(&dir, "src/App.vue", APP_VUE).await?;
        put(&dir, "src/components/SiteHeader.vue", SITE_HEADER_VUE).await?;
        files.extend([
            "package.json",
            "vite.config.js",
            "index.html",
            "src/main.js",
            "src/App.vue",
            "src/components/SiteHeader.vue",
        ]);
    }

    Ok(serde_json::json!({
        "path": proj,
        "framework": if is_react { "react" } else { "vue" },
        "files": files,
        "next": format!("cd {proj} && npm install && npm run dev"),
        "note": "已铺好 Vite + Tailwind + 设计令牌系统(src/style.css :root) + 字体配对(Space Grotesk 标题 / Manrope 正文)。改配色只改 :root 变量；组件一律用 var(--...) / Tailwind theme，别写死数值。在此基座上建页面。"
    }))
}

// ── curated templates ──────────────────────────────────────────────

fn pkg_json(proj: &str, react: bool) -> String {
    if react {
        format!(
            r#"{{
  "name": "{proj}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{ "dev": "vite", "build": "vite build", "preview": "vite preview" }},
  "dependencies": {{ "react": "^18.3.1", "react-dom": "^18.3.1" }},
  "devDependencies": {{
    "@vitejs/plugin-react": "^4.3.4",
    "autoprefixer": "^10.4.20",
    "postcss": "^8.4.49",
    "tailwindcss": "^3.4.17",
    "vite": "^6.0.7"
  }}
}}
"#
        )
    } else {
        format!(
            r#"{{
  "name": "{proj}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{ "dev": "vite", "build": "vite build", "preview": "vite preview" }},
  "dependencies": {{ "vue": "^3.5.13" }},
  "devDependencies": {{
    "@vitejs/plugin-vue": "^5.2.1",
    "autoprefixer": "^10.4.20",
    "postcss": "^8.4.49",
    "tailwindcss": "^3.4.17",
    "vite": "^6.0.7"
  }}
}}
"#
        )
    }
}

const VITE_CONFIG_VUE: &str = r#"import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: { host: '127.0.0.1', port: 3000 },
})
"#;

const VITE_CONFIG_REACT: &str = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: { host: '127.0.0.1', port: 3000 },
})
"#;

const POSTCSS_CONFIG: &str = r#"export default {
  plugins: { tailwindcss: {}, autoprefixer: {} },
}
"#;

// Tailwind theme is wired to the CSS-variable tokens in style.css, so Tailwind
// utilities and raw CSS share one source of truth. Change a token → both update.
const TAILWIND_CONFIG: &str = r#"/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,jsx,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: 'var(--bg)', surface: 'var(--surface)', border: 'var(--border)',
        text: 'var(--text)', muted: 'var(--text-muted)', faint: 'var(--text-faint)',
        primary: 'var(--primary)', 'primary-hover': 'var(--primary-hover)',
      },
      fontFamily: {
        sans: 'var(--font-sans)', display: 'var(--font-display)', mono: 'var(--font-mono)',
      },
      borderRadius: { sm: 'var(--radius-sm)', DEFAULT: 'var(--radius-md)', lg: 'var(--radius-lg)' },
      boxShadow: { sm: 'var(--shadow-sm)', md: 'var(--shadow-md)' },
      maxWidth: { prose: '65ch', content: '72rem' },
    },
  },
  plugins: [],
}
"#;

// The design system. Fonts default to a real pairing (NOT Inter). All values
// are tokens — components reference var(--…), never hardcode.
const STYLE_CSS: &str = r#"@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  /* fonts — a real pairing, not the AI-slop Inter default */
  --font-sans: 'Manrope', system-ui, -apple-system, sans-serif;
  --font-display: 'Space Grotesk', var(--font-sans);
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  /* spacing — 4px grid */
  --sp-1: 4px; --sp-2: 8px; --sp-3: 12px; --sp-4: 16px; --sp-6: 24px; --sp-8: 32px; --sp-12: 48px; --sp-16: 64px;
  /* color — light theme, neutral with a slight cool bias */
  --bg: #ffffff; --surface: #f8fafc; --border: #e5e7eb;
  --text: #1f2937; --text-muted: #6b7280; --text-faint: #9ca3af;
  --primary: #4f46e5; --primary-hover: #4338ca; --primary-light: #eef2ff;
  --success: #16a34a; --warning: #d97706; --danger: #dc2626;
  /* radius / shadow / motion */
  --radius-sm: 6px; --radius-md: 10px; --radius-lg: 16px; --radius-full: 9999px;
  --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.06);
  --shadow-md: 0 4px 12px -2px rgb(0 0 0 / 0.08), 0 2px 6px -2px rgb(0 0 0 / 0.05);
  --duration: 160ms; --ease: cubic-bezier(0.16, 1, 0.3, 1);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0b0e14; --surface: #141922; --border: #232a36;
    --text: #e6e9ef; --text-muted: #9aa4b2; --text-faint: #6b7480;
    --primary: #818cf8; --primary-hover: #a5b4fc; --primary-light: #1e2333;
    --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.4);
    --shadow-md: 0 8px 24px -4px rgb(0 0 0 / 0.5);
  }
}

@layer base {
  body { background: var(--bg); color: var(--text); font-family: var(--font-sans); -webkit-font-smoothing: antialiased; }
  h1, h2, h3 { font-family: var(--font-display); line-height: 1.12; letter-spacing: -0.01em; }
  p { line-height: 1.6; }
}

@layer components {
  .btn { display: inline-flex; align-items: center; gap: var(--sp-2); padding: var(--sp-3) var(--sp-6); font-size: 0.9375rem; font-weight: 600; border-radius: var(--radius-md); border: 1px solid transparent; cursor: pointer; transition: background var(--duration) var(--ease), transform var(--duration) var(--ease); }
  .btn-primary { background: var(--primary); color: #fff; }
  .btn-primary:hover { background: var(--primary-hover); transform: translateY(-1px); }
  .btn-ghost { background: transparent; color: var(--text); border-color: var(--border); }
  .btn-ghost:hover { background: var(--surface); }
  .btn:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
  .card { background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: var(--sp-8); box-shadow: var(--shadow-sm); transition: box-shadow var(--duration) var(--ease), transform var(--duration) var(--ease); }
  .card:hover { box-shadow: var(--shadow-md); transform: translateY(-2px); }
}
"#;

fn index_html(proj: &str, react: bool) -> String {
    let entry = if react {
        "/src/main.jsx"
    } else {
        "/src/main.js"
    };
    let mount = if react {
        r#"<div id="root"></div>"#
    } else {
        r#"<div id="app"></div>"#
    };
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{proj}</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700&display=swap" rel="stylesheet" />
  </head>
  <body>
    {mount}
    <script type="module" src="{entry}"></script>
  </body>
</html>
"#
    )
}

const MAIN_JS: &str = r#"import { createApp } from 'vue'
import './style.css'
import App from './App.vue'

createApp(App).mount('#app')
"#;

const APP_VUE: &str = r##"<script setup>
import SiteHeader from './components/SiteHeader.vue'

const features = [
  { title: 'Token-driven', body: '一套 CSS 变量当单一真源，改主题只改 :root。' },
  { title: 'Real type', body: 'Space Grotesk 标题 + Manrope 正文，不是满屏 Inter。' },
  { title: 'Accessible', body: '状态齐全、对比达标、prefers-reduced-motion 兜底。' },
]
</script>

<template>
  <SiteHeader />
  <main class="mx-auto max-w-content px-6">
    <section class="py-24 text-center">
      <p class="mb-4 text-sm font-semibold uppercase tracking-widest text-primary">Starter</p>
      <h1 class="mx-auto max-w-3xl text-5xl font-bold text-text">在一套精选设计系统上开始搭建</h1>
      <p class="mx-auto mt-6 max-w-prose text-lg text-muted">
        Vite + Vue + Tailwind，设计令牌 / 字体配对 / 基础组件已就位。把内容换成这个产品自己的东西。
      </p>
      <div class="mt-8 flex justify-center gap-4">
        <a class="btn btn-primary" href="#">开始</a>
        <a class="btn btn-ghost" href="#">了解更多</a>
      </div>
    </section>
    <section class="grid gap-6 pb-24 sm:grid-cols-3">
      <article v-for="f in features" :key="f.title" class="card">
        <h3 class="text-lg font-semibold text-text">{{ f.title }}</h3>
        <p class="mt-2 text-muted">{{ f.body }}</p>
      </article>
    </section>
  </main>
</template>
"##;

const SITE_HEADER_VUE: &str = r##"<template>
  <header class="sticky top-0 z-10 border-b border-border bg-bg/80 backdrop-blur">
    <div class="mx-auto flex max-w-content items-center justify-between px-6 py-4">
      <span class="font-display text-lg font-bold text-text">Brand</span>
      <nav class="hidden items-center gap-8 text-sm text-muted sm:flex">
        <a class="transition-colors hover:text-text" href="#">Features</a>
        <a class="transition-colors hover:text-text" href="#">Pricing</a>
        <a class="transition-colors hover:text-text" href="#">Docs</a>
      </nav>
      <a class="btn btn-primary" href="#">Get started</a>
    </div>
  </header>
</template>
"##;

const MAIN_JSX: &str = r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import './style.css'
import App from './App.jsx'

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
"#;

const APP_JSX: &str = r##"import './style.css'

const features = [
  { title: 'Token-driven', body: '一套 CSS 变量当单一真源，改主题只改 :root。' },
  { title: 'Real type', body: 'Space Grotesk 标题 + Manrope 正文，不是满屏 Inter。' },
  { title: 'Accessible', body: '状态齐全、对比达标、prefers-reduced-motion 兜底。' },
]

export default function App() {
  return (
    <>
      <header className="sticky top-0 z-10 border-b border-border bg-bg/80 backdrop-blur">
        <div className="mx-auto flex max-w-content items-center justify-between px-6 py-4">
          <span className="font-display text-lg font-bold text-text">Brand</span>
          <a className="btn btn-primary" href="#">Get started</a>
        </div>
      </header>
      <main className="mx-auto max-w-content px-6">
        <section className="py-24 text-center">
          <p className="mb-4 text-sm font-semibold uppercase tracking-widest text-primary">Starter</p>
          <h1 className="mx-auto max-w-3xl text-5xl font-bold text-text">在一套精选设计系统上开始搭建</h1>
          <p className="mx-auto mt-6 max-w-prose text-lg text-muted">
            Vite + React + Tailwind，设计令牌 / 字体配对 / 基础组件已就位。
          </p>
          <div className="mt-8 flex justify-center gap-4">
            <a className="btn btn-primary" href="#">开始</a>
            <a className="btn btn-ghost" href="#">了解更多</a>
          </div>
        </section>
        <section className="grid gap-6 pb-24 sm:grid-cols-3">
          {features.map((f) => (
            <article key={f.title} className="card">
              <h3 className="text-lg font-semibold text-text">{f.title}</h3>
              <p className="mt-2 text-muted">{f.body}</p>
            </article>
          ))}
        </section>
      </main>
    </>
  )
}
"##;

const GITIGNORE: &str = "node_modules\ndist\n.DS_Store\n*.local\n";

fn readme(proj: &str) -> String {
    format!(
        r#"# {proj}

Vite + Tailwind starter with a curated design-token system.

```bash
npm install
npm run dev      # http://127.0.0.1:3000
```

- **Design tokens** live in `src/style.css` `:root` — change the theme there, never hardcode values.
- **Fonts**: Space Grotesk (display) + Manrope (body), loaded in `index.html`.
- **Base components**: `.btn`, `.btn-primary`, `.btn-ghost`, `.card` (+ Tailwind theme wired to the tokens).
"#
    )
}
