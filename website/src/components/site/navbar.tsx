import { useState } from "react";
import { LogIn, Menu, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ThemeToggle } from "@/components/theme-toggle";
import { cn } from "@/lib/utils";

/** The gateway's own sign-in page, which also handles registration. */
const LOGIN_URL = "https://code.mrday.one/gate";

const links = [
  { href: "#features", label: "Product" },
  { href: "#architecture", label: "How it works" },
  { href: "#extensions", label: "Extensibility" },
  { href: "#customers", label: "Customers" },
];

export function Navbar() {
  const [open, setOpen] = useState(false);

  return (
    <header className="sticky top-0 z-50 border-b border-border/60 bg-background/80 backdrop-blur-xl">
      <nav className="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6">
        {/* 标题栏式左组：窗口控制点 → 日/夜切换 → 品牌 */}
        <div className="flex items-center gap-3">
          <ThemeToggle />
          <a href="#" className="flex items-center gap-2.5 font-display text-lg font-semibold">
            <img src="/logo.png" alt="" className="size-8" />
            Mr.day One
          </a>
        </div>

        <div className="hidden items-center gap-1 md:flex">
          {links.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            >
              {link.label}
            </a>
          ))}
        </div>

        <div className="hidden items-center gap-2 md:flex">
          <Button variant="ghost" size="sm" asChild>
            <a href={LOGIN_URL}>
              <LogIn /> Log in
            </a>
          </Button>
          <Button size="sm" asChild>
            <a href="#download">Download</a>
          </Button>
        </div>

        <div className="flex items-center gap-1 md:hidden">
          <Button
            variant="ghost"
            size="icon"
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X /> : <Menu />}
          </Button>
        </div>
      </nav>

      <div className={cn("border-t border-border/60 px-4 pb-4 md:hidden", open ? "block" : "hidden")}>
        {links.map((link) => (
          <a
            key={link.href}
            href={link.href}
            onClick={() => setOpen(false)}
            className="block rounded-lg px-3 py-2.5 text-sm font-medium text-muted-foreground hover:bg-secondary hover:text-foreground"
          >
            {link.label}
          </a>
        ))}
        <Button variant="outline" className="mt-2 w-full" asChild>
          <a href={LOGIN_URL} onClick={() => setOpen(false)}>
            <LogIn /> Log in
          </a>
        </Button>
        <Button className="mt-2 w-full" asChild>
          <a href="#download" onClick={() => setOpen(false)}>
            Download
          </a>
        </Button>
      </div>
    </header>
  );
}
