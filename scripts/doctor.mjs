import { spawnSync } from "node:child_process";
import { cargoEnvironment } from "./cargo-environment.mjs";
import { probeTools } from "./doctor-tools.mjs";

const environment = cargoEnvironment(process.env);
const checks = probeTools({
  platform: process.platform,
  arch: process.arch,
  environment,
  run(command, args) {
    return spawnSync(command, args, { env: environment, encoding: "utf8", timeout: 15_000 });
  },
});

let hasFailure = false;
for (const { label, output, required } of checks) {
  if (output === null) {
    console.error(`${label} : indisponible${required ? " (requis)" : " (optionnel)"}`);
    if (required) {
      hasFailure = true;
    }
    continue;
  }

  console.log(`${label} : ${output}`);
}
console.log("Versions attendues : voir rust-toolchain.toml, package.json et docs/development.md.");
process.exitCode = Number(hasFailure);
