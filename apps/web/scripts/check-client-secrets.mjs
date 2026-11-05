// Builds with sentinel values for server-only secrets and fails if any of
// them appears in the client bundle (.next/static). Threat model item W1.
import { execSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const SENTINELS = { PINATA_JWT: "ORBIT_SECRET_SENTINEL_9f1c2e" };

execSync("npx next build", {
  stdio: "inherit",
  env: { ...process.env, ...SENTINELS, NEXT_TELEMETRY_DISABLED: "1" },
});

function* files(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* files(path);
    else yield path;
  }
}

const leaks = [];
for (const file of files(".next/static")) {
  const text = readFileSync(file, "utf8");
  for (const [name, value] of Object.entries(SENTINELS)) {
    if (text.includes(value)) leaks.push(`${name} in ${file}`);
  }
}
if (leaks.length) {
  console.error(`Server secrets found in client assets:\n${leaks.join("\n")}`);
  process.exit(1);
}
console.log("No server-only secrets in client assets.");
