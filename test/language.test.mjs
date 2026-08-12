import { test } from "node:test";
import assert from "node:assert/strict";
import {
  AGENT_LANG_MAP,
  langBadge,
  langKey,
  languageIdForPath,
} from "../src/agent/language.js";

test("langKey recognizes file extensions and explicit language names", () => {
  assert.equal(langKey("src/app.tsx"), "ts");
  assert.equal(langKey("Python"), "py");
  assert.equal(langKey("README.md"), "md");
  assert.equal(langKey("Makefile"), "default");
});

test("languageIdForPath keeps unknown extensions for Monaco fallback", () => {
  assert.equal(languageIdForPath("src/app.jsx"), "js");
  assert.equal(languageIdForPath("Cargo.toml"), "toml");
  assert.equal(languageIdForPath(""), "");
});

test("langBadge renders stable badge classes and escaped labels", () => {
  assert.equal(langBadge("src/app.ts"), '<span class="atc-lang-badge atc-lang-badge--ts">TS</span>');
  assert.equal(langBadge("unknown.file"), '<span class="atc-lang-badge atc-lang-badge--default">FILE</span>');

  const html = langBadge("x.foo", {
    langMap: { ...AGENT_LANG_MAP, foo: "custom" },
    labels: { custom: "<Custom>" },
  });
  assert.equal(html, '<span class="atc-lang-badge atc-lang-badge--custom">&lt;Custom&gt;</span>');
});
