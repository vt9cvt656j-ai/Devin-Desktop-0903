/*
 * The real marks, drawn as paths.
 *
 * lucide has a GitHub glyph but nothing for GitLab, so the pair was GitHub's cat next to
 * a generic branch icon — which reads as "GitHub, and some other thing". A brand is how
 * people recognise where they are about to send their credentials, so both are the
 * actual logos.
 *
 * `currentColor` throughout rather than the brand palette: these sit in a monochrome
 * console and have to work on light and dark alike. The GitLab tanuki is normally four
 * oranges; flattening it to one colour keeps the silhouette, which is the recognisable
 * part.
 */

export function GitHubMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" aria-hidden className={className} fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

export function GitLabMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M23.955 13.587l-1.342-4.135-2.664-8.189a.455.455 0 0 0-.867 0L16.418 9.45H7.582L4.918 1.263a.455.455 0 0 0-.867 0L1.387 9.452.045 13.587a.924.924 0 0 0 .331 1.03L12 23.054l11.624-8.436a.92.92 0 0 0 .331-1.031" />
    </svg>
  );
}

/** Keyed by the provider string the gateway sends. */
export const BRAND_MARKS: Record<string, (p: { className?: string }) => React.ReactElement> = {
  github: GitHubMark,
  gitlab: GitLabMark,
};
