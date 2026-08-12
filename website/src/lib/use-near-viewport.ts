import { useEffect, useState, type RefObject } from "react";

/**
 * True once the element is near the viewport — and never before the page has loaded.
 *
 * Used to decide when the embedded editors may boot. Two properties matter, and the
 * obvious `IntersectionObserver` version had neither:
 *
 * **It must not fire early.** The posters above these sections have no intrinsic size
 * until they load, so during the first moments the document is a fraction of its final
 * height and a section that really sits 4,000px down measures as almost on screen. An
 * observer took that at face value, booted the editor at the top of the page, and the
 * editor's focus dragged the reader down to it — the "site opens part-way down" bug.
 * Waiting for `load` means every measurement is taken against the settled layout.
 *
 * **It must be observable.** Intersection callbacks do not run when a page is not being
 * painted, which made the behaviour impossible to verify in a headless browser and easy
 * to get wrong twice. Measuring on scroll is plain arithmetic that runs anywhere.
 */
export function useNearViewport(
  ref: RefObject<Element | null>,
  margin = 300,
): boolean {
  const [near, setNear] = useState(false);

  useEffect(() => {
    if (near) return;

    const check = () => {
      const el = ref.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.top < window.innerHeight + margin && rect.bottom > -margin) setNear(true);
    };

    const begin = () => {
      // Check once on arrival, for a reader who reloaded already scrolled down.
      check();
      window.addEventListener("scroll", check, { passive: true });
      window.addEventListener("resize", check);
    };

    if (document.readyState === "complete") begin();
    else window.addEventListener("load", begin, { once: true });

    return () => {
      window.removeEventListener("load", begin);
      window.removeEventListener("scroll", check);
      window.removeEventListener("resize", check);
    };
  }, [ref, margin, near]);

  return near;
}
