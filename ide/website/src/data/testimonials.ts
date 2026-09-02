/**
 * Social proof — real posts and quotes about the product.
 *
 * The section renders ONLY entries with `placeholder` removed or set to false, and it
 * disappears entirely while every entry is still a template. Nothing here may be
 * invented: each card names a real person and should link to the post it came from,
 * so any visitor can check it.
 *
 * To add one: copy the person's own words verbatim, set their name and handle, paste
 * the permalink into `url`, point `avatar` at their public profile picture, and delete
 * `placeholder`.
 */
export type Testimonial = {
  id: string;
  /** The person's own words, verbatim. Never paraphrased, never written for them. */
  quote: string;
  name: string;
  /** Handle without the @, e.g. "sethtjf". */
  handle?: string;
  role?: string;
  company?: string;
  /** Where it was posted — drives the icon in the corner. */
  platform?: "x" | "linkedin" | "github";
  /** Permalink to the original post, so the claim is checkable. */
  url?: string;
  /** Their own profile picture URL. */
  avatar?: string;
  /** Short verifiable outcome, e.g. "4 days → 6 hours". */
  metric?: string;
  /** Remove once the entry holds a real, sourced quote. */
  placeholder?: boolean;
};

export const testimonials: Testimonial[] = [
  {
    id: "one",
    quote: "Paste a real post about Mr. Day One here, word for word.",
    name: "Their name",
    handle: "theirhandle",
    platform: "x",
    url: "",
    placeholder: true,
  },
  {
    id: "two",
    quote: "A second real post — ideally one that names a specific thing it did for them.",
    name: "Their name",
    handle: "theirhandle",
    platform: "x",
    url: "",
    placeholder: true,
  },
  {
    id: "three",
    quote: "A quote from a customer conversation works too — with their permission.",
    name: "Their name",
    role: "Their role",
    company: "Their company",
    platform: "linkedin",
    placeholder: true,
  },
  {
    id: "four",
    quote: "If someone opened an issue or a PR saying something useful, that counts as well.",
    name: "Their name",
    handle: "theirhandle",
    platform: "github",
    placeholder: true,
  },
];

/** True once at least one entry is a real, sourced quote. Drives both the section
 *  and the "Customers" nav link, so a link can never point at a hidden section. */
export const hasRealTestimonials = testimonials.some((t) => !t.placeholder);
