// Shared accessors for the src/main.js source text used by the source-assertion tests.
//
// Why this exists: dozens of tests assert `assert.match(SRC, /…/)` to prove that a
// contract (a routing hint, a prompt line, a guard) is really present in the shipped
// code. Raw source text also contains COMMENTS, so a contract that was deleted from the
// code but left behind in a comment keeps every one of those assertions green. That trap
// has already fired here: five tool-contract assertions stayed green off a comment line
// that literally labelled itself "兼容旧提示契约" after the text had been removed from
// the model-visible channel.
//
// `CODE` is the same file with every comment blanked out — byte offsets and line numbers
// are preserved, so failure output still points at the right line. String and template
// literals and regex literals are untouched, so prompt text living inside a template
// literal still matches. Positive assertions ("this contract must exist") belong on CODE.
// `SRC` stays available for the handful of assertions that deliberately inspect comments.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import * as acorn from "acorn";

const HERE = dirname(fileURLToPath(import.meta.url));
export const MAIN_PATH = join(HERE, "../../src/main.js");

/** Raw main.js text, comments included. Only for assertions that target comments. */
export const SRC = readFileSync(MAIN_PATH, "utf8");

/**
 * Blank out every comment in `source`, preserving length and line breaks so offsets and
 * line numbers stay identical to the original.
 */
export function stripComments(source) {
  const ranges = [];
  acorn.parse(source, {
    ecmaVersion: "latest",
    sourceType: "module",
    allowAwaitOutsideFunction: true,
    allowHashBang: true,
    onComment(_block, _text, start, end) { ranges.push([start, end]); },
  });
  if (!ranges.length) return source;
  const out = source.split("");
  for (const [start, end] of ranges) {
    for (let i = start; i < end; i++) {
      if (out[i] !== "\n" && out[i] !== "\r") out[i] = " ";
    }
  }
  return out.join("");
}

/** main.js with all comments blanked out. Use this for positive source assertions. */
export const CODE = stripComments(SRC);
