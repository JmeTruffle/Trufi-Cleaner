// Stamps the release version and update endpoint into tauri.conf.json.
// Usage: node scripts/stamp-release.mjs <patch> <owner/repo>
// The version is <major>.<minor> from the config plus the given patch, so
// every push to main becomes a new release (0.1.1, 0.1.2, ...). Bump the
// minor or major in tauri.conf.json by hand for bigger milestones.

import { readFileSync, writeFileSync, appendFileSync } from "node:fs";

const [patch, releasesRepo] = process.argv.slice(2);
if (!/^\d+$/.test(patch ?? "") || !/^[\w.-]+\/[\w.-]+$/.test(releasesRepo ?? "")) {
  console.error("usage: stamp-release.mjs <patch> <owner/repo>");
  process.exit(1);
}

const path = "apps/desktop/src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(path, "utf8"));
const [major, minor] = conf.version.split(".");
conf.version = `${major}.${minor}.${patch}`;
conf.plugins.updater.endpoints = [
  `https://github.com/${releasesRepo}/releases/latest/download/latest.json`,
];
writeFileSync(path, JSON.stringify(conf, null, 2) + "\n");

if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${conf.version}\n`);
console.log(`version ${conf.version}, updates from ${releasesRepo}`);
