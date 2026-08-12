/**
 * The shape of a changelog entry. The entries themselves are not here.
 *
 * They used to be: a hand-edited array in this file, which meant publishing one cost a
 * rebuild and a deploy. They now live in the gateway's `changelog_entries` table and are
 * written from the admin console, so the page fetches them at runtime and there is exactly
 * one place an entry is created or removed.
 *
 * Only the types remain, because the page still needs to know what it is rendering.
 * If a fourth `kind` is ever added, it has to be added in three places at once — here, the
 * icon table in `changelog-page.tsx`, and the `KINDS` guard in the gateway's changelog.rs
 * — or the server will reject entries the page could have drawn, which is the failure that
 * guard exists to make loud rather than silent.
 */

export type ChangeKind = "added" | "fixed" | "changed";

export type ChangelogEntry = {
  /** ISO date, "2026-08-10". Rendered from its parts to avoid a UTC day shift. */
  date: string;
  /** Which surface changed: IDE, Console, Website, Gateway. */
  product: string;
  title: string;
  /** Empty when the change does not ride a release. */
  version?: string;
  changes: { kind: ChangeKind; text: string }[];
};
