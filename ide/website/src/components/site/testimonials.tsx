import type { PointerEvent } from "react";
import { Github, Linkedin, MessageSquarePlus } from "lucide-react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { SectionReveal } from "@/components/motion/section-reveal";
import { CodeReview } from "@/components/site/code-review";
import { hasRealTestimonials, testimonials, type Testimonial } from "@/data/testimonials";

/* 聚光位置写进 CSS 变量，光斑本体在 index.css .spotlight-card（仅 hover 设备启用） */
function trackSpotlight(event: PointerEvent<HTMLDivElement>) {
  const rect = event.currentTarget.getBoundingClientRect();
  event.currentTarget.style.setProperty("--spot-x", `${event.clientX - rect.left}px`);
  event.currentTarget.style.setProperty("--spot-y", `${event.clientY - rect.top}px`);
}

function initialsOf(name: string) {
  return (
    name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0]?.toUpperCase() ?? "")
      .join("") || "—"
  );
}

/** X has no lucide glyph; this is the mark itself. */
function XMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}

function PlatformMark({ platform }: { platform?: Testimonial["platform"] }) {
  const cls = "size-4 shrink-0 text-muted-foreground";
  if (platform === "linkedin") return <Linkedin className={cls} />;
  if (platform === "github") return <Github className={cls} />;
  return <XMark className={cls} />;
}

/**
 * A post card: avatar, name, handle, source mark, then the person's own words.
 * The whole card links to the original when a permalink is present, so anything
 * claimed on this page can be checked against its source.
 */
function PostCard({ t }: { t: Testimonial }) {
  const body = (
    <Card
      className="spotlight-card h-full transition-all duration-200 hover:-translate-y-1 hover:shadow-lg"
      onPointerMove={trackSpotlight}
    >
      <CardContent className="p-6">
        <div className="flex items-start gap-3">
          <Avatar className="size-11">
            {t.avatar && <AvatarImage src={t.avatar} alt="" />}
            <AvatarFallback>{initialsOf(t.name)}</AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1">
            <p className="truncate font-semibold leading-tight">{t.name}</p>
            <p className="truncate text-sm text-muted-foreground">
              {t.handle ? `@${t.handle}` : [t.role, t.company].filter(Boolean).join(" · ")}
            </p>
          </div>
          <PlatformMark platform={t.platform} />
        </div>

        <blockquote className="mt-4 text-pretty text-[15px] leading-relaxed text-foreground">
          {t.quote}
        </blockquote>

        {t.metric && (
          <Badge variant="secondary" className="mt-4 font-normal">
            {t.metric}
          </Badge>
        )}
      </CardContent>
    </Card>
  );

  return t.url ? (
    <a
      href={t.url}
      target="_blank"
      rel="noreferrer"
      className="block h-full rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
    >
      {body}
    </a>
  ) : (
    body
  );
}

/** Facts anyone can check, so the section says something true before quotes exist. */
const PROOF = [
  ["157", "tools in the agent's catalog"],
  ["665", "automated tests, green"],
  ["80+", "languages highlighted"],
  ["8", "languages in the interface"],
];

/*
 * The two buttons here used to point at the GitHub repo and its discussions board. The
 * repo is private, so both returned 404 — an invitation to talk to us that led nowhere.
 * Until there is a public place to post, the invitation stands on its own text and the
 * button goes somewhere that works.
 */
const SIGN_UP = "https://code.mrday.one/gate";

/** Shown until the first real quote lands — an invitation, not invented praise. */
function CollectingState() {
  return (
    <Card className="overflow-hidden">
      <CardContent className="grid items-center gap-8 p-8 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)] lg:p-10">
        <div>
          <h3 className="text-balance text-xl font-semibold sm:text-2xl">
            We would rather show you real ones
          </h3>
          <p className="mt-3 text-pretty text-muted-foreground">
            This is where quotes from people using Mr. Day One will go — their words, linked to
            the post they came from. We are collecting them now rather than writing them
            ourselves. If it has changed how you work, we would like to hear it.
          </p>
          <div className="mt-6 flex flex-wrap gap-3">
            <Button asChild>
              <a href={SIGN_UP}>
                <MessageSquarePlus /> Try it and tell us
              </a>
            </Button>
          </div>
        </div>

        <dl className="grid grid-cols-2 gap-x-6 gap-y-5">
          {PROOF.map(([value, label]) => (
            <div key={label}>
              <dt className="type-metric text-2xl leading-none">{value}</dt>
              <dd className="mt-1.5 text-sm leading-snug text-muted-foreground">{label}</dd>
            </div>
          ))}
        </dl>
      </CardContent>
    </Card>
  );
}

export function Testimonials() {
  // Real quotes render as cards. Until there is one, the slot carries an honest
  // invitation plus checkable facts — never invented praise, but never a hole either.
  const real = testimonials.filter((t) => !t.placeholder);

  return (
    <section id="customers" className="border-t border-border bg-background py-24">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <SectionReveal className="mb-12 max-w-2xl">
          <p className="type-eyebrow mb-3">Reviews</p>
          <h2 className="text-balance text-3xl font-semibold sm:text-4xl">
            An outside read of the code
          </h2>
          {hasRealTestimonials && (
            <p className="mt-4 text-muted-foreground">Every card links to the post it came from.</p>
          )}
        </SectionReveal>

        <CodeReview />

        {hasRealTestimonials ? (
          /* Masonry so posts of different lengths sit without ragged gaps. */
          <div className="mt-6 columns-1 gap-5 sm:columns-2 lg:columns-3 [&>*]:mb-5 [&>*]:break-inside-avoid">
            {real.map((t, i) => (
              <SectionReveal key={t.id} delay={(i % 3) * 70}>
                <PostCard t={t} />
              </SectionReveal>
            ))}
          </div>
        ) : (
          <SectionReveal>
            <div className="mt-6">
              <CollectingState />
            </div>
          </SectionReveal>
        )}
      </div>
    </section>
  );
}
