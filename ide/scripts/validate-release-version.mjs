import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync(new URL("../package.json", import.meta.url))).version;
const tauriVersion = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url))).version;
const cargo = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
const versions = new Map([
  ["package.json", packageVersion],
  ["src-tauri/tauri.conf.json", tauriVersion],
  ["Cargo.toml", cargoVersion],
]);

for (const [file, version] of versions) {
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(String(version || ""))) {
    throw new Error(`${file} has an invalid release version: ${version || "missing"}`);
  }
}

if (new Set(versions.values()).size !== 1) {
  throw new Error(`Michael IDE versions do not match: ${[...versions].map(([file, version]) => `${file}=${version}`).join(", ")}`);
}

const releaseTag = String(process.env.MICHAEL_RELEASE_TAG || "").trim();
if (releaseTag && releaseTag !== `v${packageVersion}`) {
  throw new Error(`release tag ${releaseTag} does not match Michael IDE v${packageVersion}`);
}

console.log(`Michael IDE release version verified: v${packageVersion}`);
