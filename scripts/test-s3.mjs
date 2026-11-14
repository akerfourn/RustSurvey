import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { setTimeout } from "node:timers/promises";

const image =
  "dxflrs/garage:v2.5.0@sha256:677e5b758e4425a325b811c425c026bf35a4bf42f7ddbaa773de3f4fd779a5a5";
const id = randomUUID();
const container = `rustsurvey-s3-${id}`;
const network = `rustsurvey-s3-${id}`;
const temporary = await mkdtemp(join(tmpdir(), "rustsurvey-s3-"));
// Public synthetic credentials, used only by this disposable, loopback fixture.
const access = "GK00000000000000000000000000000001";
const secret = "0000000000000000000000000000000000000000000000000000000000000001";
let interrupted = false;
let active = null;
let networkCreated = false;
let containerCreated = false;
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => {
    interrupted = true;
    active?.kill(signal);
  });
function run(command, args, { visible = false, env = process.env, cleanup = false } = {}) {
  if (interrupted && !cleanup) return Promise.reject(new Error("S3 checks interrupted."));
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      env,
      stdio: visible ? "inherit" : ["ignore", "pipe", "pipe"],
    });
    active = child;
    let output = "";
    if (!visible)
      child.stdout.on("data", (chunk) => {
        output += chunk.toString();
      });
    if (!visible) child.stderr.on("data", () => {});
    child.on("error", reject);
    child.on("close", (code) => {
      if (active === child) active = null;
      if (code !== 0) return reject(new Error(`${command} failed (${code}).`));
      resolve(output.trim());
    });
  });
}
try {
  await run("docker", ["image", "inspect", image]).catch(() => {
    throw new Error(`Missing local image. Explicitly prepare it with: docker pull ${image}`);
  });
  const config = join(temporary, "garage.toml");
  await writeFile(
    config,
    `metadata_dir = "/var/lib/garage/meta"
data_dir = "/var/lib/garage/data"
db_engine = "sqlite"
replication_factor = 1
block_ram_buffer_max = "16MiB"
rpc_bind_addr = "127.0.0.1:3901"
rpc_public_addr = "127.0.0.1:3901"
rpc_secret = "${"0".repeat(63)}1"
allow_world_readable_secrets = true
[s3_api]
s3_region = "garage"
api_bind_addr = "0.0.0.0:3900"
root_domain = ".s3.garage.localhost"
`,
    { mode: 0o600 },
  );
  await run("docker", ["network", "create", network]);
  networkCreated = true;
  await run("docker", [
    "run",
    "--detach",
    "--rm",
    "--pull=never",
    "--name",
    container,
    "--network",
    network,
    "--memory=384m",
    "--cpus=2",
    "--pids-limit=128",
    "--read-only",
    "--cap-drop=ALL",
    "--tmpfs",
    "/var/lib/garage:rw,nosuid,noexec,size=192m",
    "--publish",
    "127.0.0.1::3900",
    "--mount",
    `type=bind,source=${config},target=/etc/garage.toml,readonly`,
    "--env",
    "RUST_LOG=error",
    "--env",
    `GARAGE_DEFAULT_ACCESS_KEY=${access}`,
    "--env",
    `GARAGE_DEFAULT_SECRET_KEY=${secret}`,
    "--env",
    "GARAGE_DEFAULT_BUCKET=rustsurvey-test",
    image,
    "/garage",
    "server",
    "--single-node",
    "--default-bucket",
  ]);
  containerCreated = true;
  const ports = JSON.parse(
    await run("docker", ["inspect", "--format", "{{json .NetworkSettings.Ports}}", container]),
  );
  assert.ok(ports["3900/tcp"]?.[0]?.HostPort, "S3 fixture must publish its loopback port");
  const endpoint = `http://127.0.0.1:${ports["3900/tcp"][0].HostPort}`;
  let ready = false;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(endpoint, {
        signal: AbortSignal.timeout(1000),
        redirect: "error",
      });
      await response.body?.cancel();
      if (response.status === 403 || response.status === 400) {
        ready = true;
        break;
      }
    } catch {
      if (interrupted) throw new Error("S3 checks interrupted.");
    }
    await setTimeout(250);
  }
  assert.ok(ready, "disposable S3 server must become ready");
  const environment = {
    ...process.env,
    RUSTSURVEY_TEST_S3_ENDPOINT: endpoint,
    RUSTSURVEY_TEST_S3_ACCESS_KEY_ID: access,
    RUSTSURVEY_TEST_S3_SECRET_ACCESS_KEY: secret,
  };
  console.log("S3: checking the shared contract against isolated Garage 2.5.0.");
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-object-store",
      "--locked",
      "--test",
      "s3",
      "test_external_s3_should_verify_signatures_and_respect_the_shared_immutable_contract",
      "--",
      "--ignored",
      "--exact",
    ],
    { visible: true, env: environment },
  );
  await run(
    "cargo",
    [
      "test",
      "-p",
      "rustsurvey-server",
      "--locked",
      "--test",
      "workspace",
      "s3_storage_tests::test_external_s3_should_preserve_sql_history_media_access_and_response_imports",
      "--",
      "--ignored",
      "--exact",
    ],
    { visible: true, env: environment },
  );
  await run("cargo", ["build", "-p", "rustsurvey-server", "--locked"], {
    visible: true,
    env: environment,
  });
  await run(process.execPath, ["scripts/test-local-storage.mjs"], {
    visible: true,
    env: environment,
  });
  console.log("S3: signatures, immutability, integrity and object operations verified.");
} catch (error) {
  if (containerCreated)
    await run("docker", ["logs", container], { visible: true, cleanup: true }).catch(() => {});
  throw error;
} finally {
  if (containerCreated)
    await run("docker", ["rm", "--force", container], { cleanup: true }).catch(() =>
      console.error(`Verify cleanup of ${container}.`),
    );
  if (networkCreated)
    await run("docker", ["network", "rm", network], { cleanup: true }).catch(() =>
      console.error(`Verify cleanup of ${network}.`),
    );
  const resolved = await realpath(temporary);
  assert.equal(dirname(resolved).toLowerCase(), (await realpath(tmpdir())).toLowerCase());
  assert.ok(basename(resolved).startsWith("rustsurvey-s3-"));
  await rm(resolved, { recursive: true, force: true });
}
