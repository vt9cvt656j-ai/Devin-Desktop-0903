import { cn } from "./lib/cn.js";

/**
 * The `/` command palette above the prompt box.
 *
 * Built in shadcn's Command language rather than the old `.atmenu` markup it shared with the
 * @-file picker. That sharing is why it looked the way it did: the @-menu's proportions
 * (18px container radius, 12px rows, 9px/12px padding) are right for file rows with icons and
 * directory paths, and far too loose for two words of text. It also inherited the @-menu's blue
 * `is-active` wash, which made every command read as a link.
 *
 * The command is rendered without its leading `/` — the slash is already visible in the input
 * the user just typed, so repeating it in every row is noise.
 *
 * Colour comes from the theme bridge in tailwind.css (bg-popover / text-foreground /
 * bg-accent / text-muted-foreground), so this follows light and dark with no variant of its own.
 */
export function SlashMenu({ items = [], activeIndex = 0, onPick, onHover }) {
  if (!items.length) return null;
  return (
    <div
      role="listbox"
      aria-label="Commands"
      className="overflow-hidden rounded-lg border border-border bg-popover p-1 shadow-lg"
    >
      {items.map((item, i) => {
        const active = i === activeIndex;
        return (
          <div
            key={item.cmd}
            role="option"
            aria-selected={active}
            // mousedown, not click: the prompt's blur handler closes this menu, and blur lands
            // before click would. preventDefault keeps focus in the textarea.
            onMouseDown={(event) => { event.preventDefault(); onPick?.(i); }}
            onMouseEnter={() => onHover?.(i)}
            className={cn(
              "flex cursor-default select-none items-baseline gap-2 rounded-md px-2 py-1.5",
              "text-[13px] leading-5 transition-colors",
              active ? "bg-accent text-accent-foreground" : "text-foreground",
            )}
          >
            {/* The command name is a literal you TYPE, not UI copy. The auto-localizer would
                otherwise render `sessions` as 会话 — which cannot be typed to run anything.
                Same category as the model picker's entries in AUTO_I18N_SKIP_SELECTOR, and
                excluded the same way. The description beside it is copy and still translates. */}
            <span className="shrink-0 font-medium" data-i18n-skip>{item.cmd}</span>
            <span className="min-w-0 truncate text-[12px] text-muted-foreground">{item.desc}</span>
          </div>
        );
      })}
    </div>
  );
}
