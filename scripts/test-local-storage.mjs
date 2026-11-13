import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, readdir, rename, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";

// All credentials and data below belong to disposable local fixtures.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? "target");
let interrupted = false;
const temporary = await mkdtemp(join(tmpdir(), "rustsurvey-local-http-"));
const image = await readFile(join(root, "crates/server/tests/fixtures/pixel.png"));
const media = ["00000000-0000-4000-8000-000000000010", "00000000-0000-4000-8000-000000000011"];
let child;
let base;
let cookie = "";
let csrf = "";
function interrupt() {
  interrupted = true;
  child?.kill("SIGTERM");
}
process.on("SIGINT", interrupt);
process.on("SIGTERM", interrupt);
async function start(backend) {
  if (interrupted) {
    throw new Error("Local storage checks interrupted");
  }
  const environment = {
    ...process.env,
    BIND_ADDRESS: "127.0.0.1:0",
    RUSTSURVEY_DEV_READY: "1",
    OBJECT_STORE_BACKEND: backend,
    OBJECT_STORE_DIRECTORY: ".data/media",
    ADMIN_USERNAME: "admin",
    ADMIN_PASSWORD: "synthetic-admin-password",
    ADMIN_COOKIE_SECURE: "false",
    DEV_ALLOW_WEAK_PASSWORDS: "false",
  };
  delete environment.DATABASE_URL;
  child = spawn(
    resolve(
      target,
      "debug",
      process.platform === "win32" ? "rustsurvey-server.exe" : "rustsurvey-server",
    ),
    [],
    { cwd: temporary, env: environment, stdio: ["ignore", "pipe", "pipe"] },
  );
  child.stderr.on("data", () => {});
  base = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("HTTP startup timed out")), 15000);
    const lines = createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      if (line.startsWith("RUSTSURVEY_DEV_API_URL=")) {
        clearTimeout(timer);
        resolve(line.slice("RUSTSURVEY_DEV_API_URL=".length));
      }
    });
    child.once("error", () => {
      clearTimeout(timer);
      reject(new Error("Cannot launch the local test server"));
    });
    child.once("exit", () => {
      clearTimeout(timer);
      reject(new Error("HTTP exited before readiness"));
    });
  });
}
async function stop() {
  if (
    child === undefined ||
    child.pid === undefined ||
    child.exitCode !== null ||
    child.signalCode !== null
  ) {
    return;
  }
  await new Promise((resolve) => {
    const timer = setTimeout(() => child.kill("SIGKILL"), 3000);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill("SIGTERM");
  });
}
async function request(path, options = {}, status = 200) {
  if (interrupted) {
    throw new Error("Local storage checks interrupted");
  }
  const response = await fetch(`${base}${path}`, {
    ...options,
    signal: AbortSignal.timeout(10000),
    headers: {
      Cookie: cookie,
      "X-Requested-With": "RustSurvey",
      "X-CSRF-Token": csrf,
      ...options.headers,
    },
  });
  assert.equal(response.status, status, `${options.method ?? "GET"} ${path}`);
  return response;
}
async function json(path, method = "GET", body, status = 200) {
  const options = { method };
  if (body !== undefined) {
    options.body = JSON.stringify(body);
    options.headers = { "Content-Type": "application/json" };
  }
  return (await request(path, options, status)).json();
}
async function collectionFixture() {
  const fixture = JSON.parse(await readFile(join(root, "examples/rules/collection.json"), "utf8"));
  const study = await json("/api/admin/studies", "POST", fixture.definition, 201);
  const run = await json(
    `/api/admin/studies/${study.id}/runs`,
    "POST",
    { expectedRevision: 1 },
    201,
  );
  const id = "00000000-0000-4000-8000-000000000100";
  const headers = {
    Authorization: `Bearer ${"a".repeat(64)}`,
    Cookie: "",
    "Content-Type": "application/json",
  };
  await request(
    `/api/runs/${run.id}/responses`,
    { method: "POST", headers, body: JSON.stringify({ responseId: id, language: "fr" }) },
    201,
  );
  const command = {
    expectedVersion: 0,
    operationId: "00000000-0000-4000-8000-000000000101",
    answers: { GATE: "Y", AGE: "24", NOTE: "été", SCORE: "forged" },
    checkpoint: { language: "en", groupId: "details" },
    submit: false,
  };
  const options = { method: "PUT", headers, body: JSON.stringify(command) };
  const acknowledgement = await (await request(`/api/responses/${id}`, options)).json();
  const invitedRun = await json(
    `/api/admin/studies/${study.id}/runs`,
    "POST",
    {
      expectedRevision: 1,
      participationPolicy: { mode: "invitation", persistence: true, allowEditAfterSubmit: true },
    },
    201,
  );
  const invitationId = "00000000-0000-4000-8000-000000000300";
  const invitedId = "00000000-0000-4000-8000-000000000301";
  const token = "d".repeat(64);
  await json(
    `/api/admin/runs/${invitedRun.id}/invitations`,
    "POST",
    { id: invitationId, token, maxUses: 1 },
    201,
  );
  const invitedHeaders = {
    ...headers,
    Authorization: `Bearer ${token}`,
    "X-Participation-Token": token,
  };
  await request(
    `/api/runs/${invitedRun.id}/responses`,
    {
      method: "POST",
      headers: invitedHeaders,
      body: JSON.stringify({ responseId: invitedId, language: "fr" }),
    },
    201,
  );
  const invitedOptions = {
    method: "PUT",
    headers: invitedHeaders,
    body: JSON.stringify({ ...command, answers: { GATE: "N" }, submit: true }),
  };
  const invitedAcknowledgement = await (
    await request(`/api/responses/${invitedId}`, invitedOptions)
  ).json();
  const invited = {
    runId: invitedRun.id,
    id: invitedId,
    invitationId,
    headers: invitedHeaders,
    options: invitedOptions,
    acknowledgement: invitedAcknowledgement,
  };
  const forms = await formsFixture(headers);
  return { id, headers, options, acknowledgement, invited, forms };
}
async function formsFixture(headers) {
  const corpus = JSON.parse(
    await readFile(join(root, "examples/rules/collection-forms.json"), "utf8"),
  );
  const study = await json("/api/admin/studies", "POST", corpus.definition, 201);
  const run = await json(
    `/api/admin/studies/${study.id}/runs`,
    "POST",
    { expectedRevision: 1 },
    201,
  );
  const id = "00000000-0000-4000-8000-000000000400";
  await request(
    `/api/runs/${run.id}/responses`,
    {
      method: "POST",
      headers,
      body: JSON.stringify({ responseId: id, language: "fr" }),
    },
    201,
  );
  const options = {
    method: "PUT",
    headers,
    body: JSON.stringify({
      expectedVersion: 0,
      operationId: "00000000-0000-4000-8000-000000000401",
      answers: corpus.baseline.answers,
      checkpoint: { language: "fr", groupId: "forms" },
      submit: true,
    }),
  };
  const acknowledgement = await (await request(`/api/responses/${id}`, options)).json();
  return { id, headers, options, acknowledgement, expected: corpus.baseline.expected };
}
async function verifyCollection(fixture) {
  const stored = await (
    await request(`/api/responses/${fixture.id}`, { headers: fixture.headers })
  ).json();
  assert.equal(stored.version, 1);
  assert.deepEqual(stored.answers, { GATE: "Y", AGE: "24", NOTE: "été", SCORE: "25" });
  assert.deepEqual(stored.checkpoint, { language: "en", groupId: "details" });
  assert.deepEqual(
    await (await request(`/api/responses/${fixture.id}`, fixture.options)).json(),
    fixture.acknowledgement,
  );
  const forms = fixture.forms;
  const complete = await (
    await request(`/api/responses/${forms.id}`, { headers: forms.headers })
  ).json();
  assert.equal(complete.run.profile, "native-forms-v1");
  assert.deepEqual(complete.answers, forms.expected);
  assert.deepEqual(
    await (await request(`/api/responses/${forms.id}`, forms.options)).json(),
    forms.acknowledgement,
  );
  const invited = fixture.invited;
  const resumed = await (
    await request(
      `/api/runs/${invited.runId}/responses`,
      {
        method: "POST",
        headers: invited.headers,
        body: JSON.stringify({
          responseId: "00000000-0000-4000-8000-000000000302",
          language: "fr",
        }),
      },
      201,
    )
  ).json();
  assert.equal(resumed.id, invited.id);
  assert.equal(resumed.status, "submitted");
  assert.deepEqual(resumed.answers, { GATE: "N" });
  assert.deepEqual(
    await (await request(`/api/responses/${invited.id}`, invited.options)).json(),
    invited.acknowledgement,
  );
  const invitations = await json(`/api/admin/runs/${invited.runId}/invitations`);
  assert.equal(invitations.items[0].uses, 1);
}
try {
  await start("sql");
  assert.ok((await stat(join(temporary, ".data/rustsurvey.db"))).isFile());
  const authentication = await request("/api/admin/login", {
    method: "POST",
    body: JSON.stringify({ username: "admin", password: "synthetic-admin-password" }),
    headers: { "Content-Type": "application/json" },
  });
  cookie = authentication.headers.getSetCookie()[0].split(";")[0];
  csrf = (await authentication.json()).csrfToken;
  const collection = await collectionFixture();
  const definition = {
    schemaVersion: 3,
    title: "Local storage check",
    language: "fr",
    welcome: "",
    end: "",
    groups: [
      {
        id: "g1",
        title: "Group",
        description: "",
        questions: [
          {
            id: "q1",
            code: "Q1",
            title: "Question",
            help: "",
            kind: "short_text",
            mandatory: "no",
            choices: [],
          },
        ],
      },
    ],
  };
  let study = await json("/api/admin/studies", "POST", definition, 201);
  await (
    await request(`/api/admin/studies/${study.id}/media/${media[0]}?name=pixel.png`, {
      method: "PUT",
      body: image,
    })
  ).arrayBuffer();
  study.definition.groups[0].questions[0].media = [
    { id: media[0], alt: "Synthetic SQL image", language: null },
  ];
  study = await json(`/api/admin/studies/${study.id}`, "PUT", {
    expectedRevision: 1,
    definition: study.definition,
  });
  await stop();
  await start("file");
  await verifyCollection(collection);
  assert.equal((await json("/api/admin/session")).authenticated, true);
  assert.deepEqual(
    Buffer.from(await (await request(`/api/studies/${study.id}/media/${media[0]}`)).arrayBuffer()),
    image,
  );
  assert.equal(
    (await readdir(join(temporary, ".data/media"))).length,
    0,
    "legacy SQL content remains readable without copying it",
  );
  await (
    await request(`/api/admin/studies/${study.id}/media/${media[1]}?name=pixel.png`, {
      method: "PUT",
      body: image,
    })
  ).arrayBuffer();
  const keys = await readdir(join(temporary, ".data/media"));
  assert.equal(keys.length, 1);
  assert.match(keys[0], /^[a-f0-9]{64}$/);
  assert.deepEqual(await readFile(join(temporary, ".data/media", keys[0], "content")), image);
  study.definition.groups[0].questions[0].media.push({
    id: media[1],
    alt: "Synthetic file image",
    language: null,
  });
  study = await json(`/api/admin/studies/${study.id}`, "PUT", {
    expectedRevision: 2,
    definition: study.definition,
  });
  const partial = await request(
    `/api/studies/${study.id}/media/${media[1]}`,
    { headers: { Range: "bytes=0-7" } },
    206,
  );
  assert.deepEqual(Buffer.from(await partial.arrayBuffer()), image.subarray(0, 8));
  await stop();
  await start("file");
  assert.equal((await json(`/api/studies/${study.id}`)).revision, 3);
  assert.deepEqual(
    Buffer.from(await (await request(`/api/studies/${study.id}/media/${media[1]}`)).arrayBuffer()),
    image,
  );
  await stop();
  // Simulate a legacy layout using only this disposable database, after a clean shutdown.
  for (const suffix of ["", "-wal", "-shm"]) {
    try {
      await rename(
        join(temporary, `.data/rustsurvey.db${suffix}`),
        join(temporary, `rustsurvey.db${suffix}`),
      );
    } catch (error) {
      if (error.code !== "ENOENT") {
        throw error;
      }
    }
  }
  await start("file");
  assert.equal((await json("/api/admin/session")).authenticated, true);
  assert.equal((await json(`/api/studies/${study.id}`)).revision, 3);
  for (const id of media) {
    assert.deepEqual(
      Buffer.from(await (await request(`/api/studies/${study.id}/media/${id}`)).arrayBuffer()),
      image,
    );
  }
  await verifyCollection(collection);
  console.log(
    "HTTP réel : stockage local, médias, sessions, collecte, invitations, reprise privée et réessai idempotent vérifiés après redémarrage, y compris avec une ancienne base racine.",
  );
} finally {
  await stop();
  await rm(temporary, { recursive: true, force: true });
}
