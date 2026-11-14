import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { copyFile, cp, mkdir, mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import { arch, platform } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cargoEnvironment } from "./cargo-environment.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? "target");
const environment = cargoEnvironment(process.env);
const npm = process.env.npm_execpath;
if (npm === undefined || process.argv.length !== 2 || process.env.CARGO_BUILD_TARGET)
  throw new Error(
    "Use npm run package:pilot on the target operating system without cross compilation.",
  );
let active = null;
let interrupted = false;
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => {
    interrupted = true;
    active?.kill(signal);
  });
function run(command, args, { visible = true, env = environment } = {}) {
  if (interrupted) return Promise.reject(new Error("Pilot packaging interrupted."));
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      env,
      windowsHide: true,
      stdio: visible ? "inherit" : ["ignore", "pipe", "pipe"],
    });
    active = child;
    let output = "";
    if (!visible)
      child.stdout.on("data", (chunk) => {
        output += chunk.toString();
      });
    if (!visible) child.stderr.resume();
    child.on("error", reject);
    child.on("close", (code) => {
      if (active === child) active = null;
      if (code !== 0)
        return reject(new Error(`${command} failed (${code}); pilot verification is incomplete.`));
      resolve(output.trim());
    });
  });
}
async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}
async function fileHashes(directory, prefix = "") {
  const files = {};
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    const name = `${prefix}${entry.name}`;
    if (entry.isDirectory()) Object.assign(files, await fileHashes(path, `${name}/`));
    else if (entry.isFile()) files[name] = await sha256(path);
    else throw new Error("Pilot packages may contain only regular files and directories.");
  }
  return files;
}

console.log("Building the pilot client and release server with all three SQL drivers.");
await run(process.execPath, [npm, "run", "build"]);
await run("cargo", ["build", "-p", "rustsurvey-server", "--release", "--all-features", "--locked"]);
const packages = join(target, "pilot");
await mkdir(packages, { recursive: true });
const output = await mkdtemp(join(packages, `rustsurvey-${platform()}-${arch()}-`));
const executable = platform() === "win32" ? "rustsurvey-server.exe" : "rustsurvey-server";
await copyFile(join(target, "release", executable), join(output, executable));
await cp(join(root, "web/dist"), join(output, "web"), {
  recursive: true,
  errorOnExist: true,
  force: false,
});
await copyFile(join(root, "docs/pilot.md"), join(output, "LIRE-MOI.md"));
await writeFile(
  join(output, ".env.example"),
  `# LOCAL PILOT. Set a private password before starting.
BIND_ADDRESS=127.0.0.1:3000
STATIC_ASSETS_DIRECTORY=web
DATABASE_URL=sqlite://.data/rustsurvey.db?mode=rwc
OBJECT_STORE_BACKEND=file
OBJECT_STORE_DIRECTORY=.data/media
ADMIN_USERNAME=admin
ADMIN_PASSWORD=
ADMIN_COOKIE_SECURE=false
DEV_ALLOW_WEAK_PASSWORDS=false
COLLECTION_WRITE_QUEUE_CAPACITY=8
`,
);
await mkdir(join(output, "examples"));
for (const [source, name] of [
  ["collection.json", "collecte-simple.json"],
  ["collection-forms.json", "28-types.json"],
]) {
  const corpus = JSON.parse(await readFile(join(root, "examples/rules", source), "utf8"));
  await writeFile(
    join(output, "examples", name),
    `${JSON.stringify(corpus.definition, null, 2)}\n`,
  );
}
const testing = {
  ...environment,
  RUSTSURVEY_TEST_BINARY: join(output, executable),
  RUSTSURVEY_TEST_WEB_DIRECTORY: join(output, "web"),
};
for (const name of Object.keys(testing))
  if (name.startsWith("RUSTSURVEY_TEST_S3_")) delete testing[name];
console.log("Checking the packaged binary and web build on isolated synthetic data.");
await run(process.execPath, ["scripts/test-local-storage.mjs"], { env: testing });
const [head, status, rustc] = await Promise.all([
  run("git", ["rev-parse", "HEAD"], { visible: false }),
  run("git", ["status", "--porcelain"], { visible: false }),
  run("rustc", ["--version"], { visible: false }),
]);
const manifest = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  platform: platform(),
  architecture: arch(),
  source: { head, dirty: status !== "", rustc },
  verification: { packagedHttpWorkflow: "passed", visualBrowserReview: "not-performed" },
  files: await fileHashes(output),
};
await writeFile(join(output, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Pilot package verified: ${output}`);
