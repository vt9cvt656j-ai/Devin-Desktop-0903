import { Check, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { SectionReveal } from "@/components/motion/section-reveal";
import { review } from "@/data/review";

/**
 * An attributed code review. Deliberately NOT styled as a customer testimonial:
 * the badge names what it is, the author is the model that wrote it, and the
 * criticisms sit at the same weight as the praise.
 */
export function CodeReview() {
  return (
    <SectionReveal>
      <Card className="overflow-hidden">
        <CardContent className="p-8 lg:p-10">
          <div className="flex flex-wrap items-center gap-3">
            {/*
              Anthropic's own published mark, saved locally rather than redrawn or
              hotlinked — a logo copied by eye is both wrong and unmistakably wrong.

              It is here to attribute the review to the model that actually wrote it, and
              a vendor mark on a product page does lean towards implying endorsement. That
              is why the badge beside it and the footnote below stay exactly as they are:
              "Code review · not a customer", and "an assessment of the code, not an
              endorsement from a user". If the criticisms ever come off this card, the
              logo has to come off with them — see the note in data/review.ts.
            */}
            <img
              src="/anthropic-mark.png"
              alt="Anthropic"
              className="size-11 shrink-0 rounded-full object-cover"
            />
            <div className="min-w-0 flex-1">
              <p className="font-semibold leading-tight">{review.author}</p>
              <p className="text-sm text-muted-foreground">{review.context}</p>
            </div>
            <Badge variant="outline" className="shrink-0">
              Code review · not a customer
            </Badge>
          </div>

          <blockquote className="mt-6 text-balance font-display text-xl leading-snug sm:text-2xl">
            “{review.verdict}”
          </blockquote>

          <Separator className="my-8" />

          <div className="grid gap-8 lg:grid-cols-2">
            <div>
              <p className="type-eyebrow mb-4">What holds up</p>
              <ul className="space-y-3">
                {review.strengths.map((s) => (
                  <li key={s} className="flex gap-2.5 text-sm leading-relaxed text-muted-foreground">
                    <Check className="mt-0.5 size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
                    <span>{s}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div>
              <p className="type-eyebrow mb-4">What needs work</p>
              <ul className="space-y-3">
                {review.criticisms.map((c) => (
                  <li key={c} className="flex gap-2.5 text-sm leading-relaxed text-muted-foreground">
                    <TriangleAlert className="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
                    <span>{c}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>

          <p className="mt-8 text-xs text-muted-foreground">
            Published in full, including the criticisms. {review.affiliation}'s model wrote this
            after working inside the repository — it is an assessment of the code, not an
            endorsement from a user.
          </p>
        </CardContent>
      </Card>
    </SectionReveal>
  );
}
