import { useState } from "react";
import { ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { SectionReveal } from "@/components/motion/section-reveal";
import { IdeEmbed } from "@/components/site/ide-embed";
import websiteLight from "@/assets/proj-website-light.png";
import websiteDark from "@/assets/proj-website-dark.png";
import desktopLight from "@/assets/proj-desktop-light.png";
import desktopDark from "@/assets/proj-desktop-dark.png";
import serviceLight from "@/assets/proj-service-light.png";
import serviceDark from "@/assets/proj-service-dark.png";
import mobileLight from "@/assets/proj-mobile-light.png";
import mobileDark from "@/assets/proj-mobile-dark.png";

/*
 * Showcase：四种真实工程，各自跑一份产品本体（/app 里的 ide 生产构建，?demo= 选工程）。
 * 一次只允许一个实例存活 —— Monaco 太重，四个同时跑会拖垮页面。
 */
const blocks = [
  {
    id: "website",
    demo: "website",
    stack: "HTML · CSS · JavaScript",
    eyebrow: "Building a website",
    title: "The whole front end, in one window",
    body: "Markup, stylesheet and modules side by side, with real highlighting for each. Custom properties, media queries and template literals all read the way they should — and the file tree keeps the shape of the site.",
    steps: [
      "Open assets/css/site.css and scroll the custom properties",
      "Compare index.html with assets/js/site.js in two tabs",
      "Find the double-submit bug flagged in site.js",
    ],
    link: { label: "See the agent loop", href: "#features" },
    poster: [websiteLight, websiteDark] as const,
  },
  {
    id: "desktop",
    demo: "desktop",
    stack: "Rust · TypeScript · Tauri",
    eyebrow: "Shipping a desktop app",
    title: "Two languages, one project, no context switch",
    body: "A Tauri app puts a Rust core under a TypeScript interface. Both open in the same editor with their own syntax, their own icons, and the language named in the status bar — so crossing the boundary costs nothing.",
    steps: [
      "Open src-tauri/src/main.rs — note the Rust icon and status bar",
      "Open src/App.tsx to see the other half of the same feature",
      "Follow invoke(\"read_note\") from the front end to the command",
    ],
    link: { label: "How it works", href: "#architecture" },
    poster: [desktopLight, desktopDark] as const,
  },
  {
    id: "service",
    demo: "service",
    stack: "Python · FastAPI · pytest",
    eyebrow: "Running a backend service",
    title: "Routes, models and the tests that cover them",
    body: "A service is more than its handlers. Routers, Pydantic models, a test suite and pinned requirements all sit in one tree, so a change and the test that proves it are two clicks apart.",
    steps: [
      "Open app/routers/invoices.py and read the validation",
      "Open tests/test_invoices.py beside it",
      "Look at webhooks.py — the signature check is not constant time",
    ],
    link: { label: "Security & governance", href: "#security" },
    poster: [serviceLight, serviceDark] as const,
  },
  {
    id: "mobile",
    demo: "mobile",
    stack: "Swift · SwiftUI · XCTest",
    eyebrow: "Working on a mobile app",
    title: "Swift and SwiftUI, highlighted properly",
    body: "Property wrappers, trailing closures, string interpolation with format specifiers — a Swift file reads correctly here, and its XCTest target sits in the same window.",
    steps: [
      "Open Trailhead/Views/HikeListView.swift",
      "Read the @EnvironmentObject and @State wrappers",
      "Open TrailheadTests to see the cases that cover the filter",
    ],
    link: { label: "Extensibility", href: "#extensions" },
    poster: [mobileLight, mobileDark] as const,
  },
];

export function Showcase() {
  const [live, setLive] = useState<string | null>(null);

  return (
    <div className="mt-28 space-y-24">
      {blocks.map((block, i) => (
        <SectionReveal key={block.id}>
          <div className="grid items-end gap-6 lg:grid-cols-[minmax(0,1fr)_auto] lg:gap-12">
            <div className={cn("min-w-0", i % 2 === 1 && "lg:order-2 lg:text-right")}>
              <p className="type-eyebrow mb-3">{block.eyebrow}</p>
              <h3 className="text-balance text-2xl font-semibold sm:text-3xl">{block.title}</h3>
              <p className="type-measure mt-3 text-pretty text-muted-foreground lg:inline-block">
                {block.body}
              </p>
              <p className="mt-3 font-mono text-xs text-brand">{block.stack}</p>
            </div>
            <ul
              className={cn(
                "space-y-1.5 text-sm text-muted-foreground",
                i % 2 === 1 && "lg:order-1",
              )}
            >
              {block.steps.map((s, n) => (
                <li key={s} className="flex items-start gap-2">
                  <span className="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full bg-secondary font-mono text-[10px] text-foreground">
                    {n + 1}
                  </span>
                  {s}
                </li>
              ))}
            </ul>
          </div>

          <div className="mt-6">
            <IdeEmbed
              label={block.id}
              demo={block.demo}
              posterLight={block.poster[0]}
              posterDark={block.poster[1]}
              active={live === block.id}
              onActivate={() => setLive(block.id)}
              onStop={() => setLive(null)}
            />
          </div>

          <a
            href={block.link.href}
            className="group mt-5 inline-flex items-center gap-1.5 text-sm font-medium text-brand transition-colors hover:text-brand-2"
          >
            {block.link.label}
            <ArrowRight className="size-4 transition-transform duration-200 group-hover:translate-x-0.5" />
          </a>
        </SectionReveal>
      ))}
    </div>
  );
}
