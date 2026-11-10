import { spawnSync } from "node:child_process";

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const checks = [
  { label: "Node.js", command: process.execPath, args: ["--version"], required: true },
  { label: "npm", command: npmCommand, args: ["--version"], required: true },
  { label: "Rust", command: "rustc", args: ["--version"], required: true },
  { label: "Cargo", command: "cargo", args: ["--version"], required: true },
  { label: "rustfmt", command: "cargo", args: ["fmt", "--version"], required: true },
  { label: "Clippy", command: "cargo", args: ["clippy", "--version"], required: true },
  { label: "Compilateur C", command: "cc", args: ["--version"], required: true },
  { label: "Docker", command: "docker", args: ["--version"], required: false },
  { label: "Docker Compose", command: "docker", args: ["compose", "version"], required: false },
];

let hasFailure = false;
for (const { label, command, args, required } of checks) {
  const result = spawnSync(command, args, { encoding: "utf8", timeout: 15_000 });
  if (result.error !== undefined || result.status !== 0) {
    console.error(`${label} : indisponible${required ? " (requis)" : " (optionnel)"}`);
    if (required) {
      hasFailure = true;
    }
    continue;
  }

  const output = result.stdout.trim();
  console.log(`${label} : ${output.split("\n")[0]}`);
}
console.log("Versions attendues : voir rust-toolchain.toml, package.json et docs/development.md.");
process.exitCode = Number(hasFailure);
