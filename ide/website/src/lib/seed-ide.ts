/**
 * The embedded editor reads its language and theme from localStorage at boot, and the
 * iframes are same-origin, so the page must seed those keys BEFORE one mounts. Without
 * this the editor boots in its default locale (Chinese) inside an English page.
 */
export function seedIdePreferences() {
  try {
    localStorage.setItem("michael-ide-locale", "en");
    localStorage.setItem(
      "michael-ide.theme",
      document.documentElement.classList.contains("dark") ? "dark" : "light",
    );
  } catch {
    /* private mode — the editor falls back to its own defaults */
  }
}
