import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, readdir, realpath, rename, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";

// All credentials and data below belong to disposable local fixtures.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? "target");
const binary =
  process.env.RUSTSURVEY_TEST_BINARY ??
  resolve(
    target,
    "debug",
    process.platform === "win32" ? "rustsurvey-server.exe" : "rustsurvey-server",
  );
const webDirectory = process.env.RUSTSURVEY_TEST_WEB_DIRECTORY;
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
  if (webDirectory === undefined) delete environment.STATIC_ASSETS_DIRECTORY;
  else environment.STATIC_ASSETS_DIRECTORY = webDirectory;
  if (backend === "s3") {
    Object.assign(environment, {
      S3_ENDPOINT: process.env.RUSTSURVEY_TEST_S3_ENDPOINT,
      S3_ACCESS_KEY_ID: process.env.RUSTSURVEY_TEST_S3_ACCESS_KEY_ID,
      S3_SECRET_ACCESS_KEY: process.env.RUSTSURVEY_TEST_S3_SECRET_ACCESS_KEY,
      S3_BUCKET: "rustsurvey-test",
      S3_REGION: "garage",
      S3_PREFIX: "http/v1",
      S3_ALLOW_HTTP_LOOPBACK: "true",
    });
  }
  delete environment.DATABASE_URL;
  child = spawn(binary, [], {
    cwd: temporary,
    env: environment,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
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
  if (webDirectory !== undefined) await verifyPublicBuild();
}
async function verifyPublicBuild() {
  const response = await request("/");
  assert.match(response.headers.get("content-type"), /^text\/html/);
  assert.equal(response.headers.get("cache-control"), "no-cache");
  assert.ok(response.headers.get("content-security-policy").includes("script-src 'self'"));
  assert.equal(response.headers.get("referrer-policy"), "no-referrer");
  const html = await response.text();
  const paths = [...html.matchAll(/(?:src|href)="(\/assets\/[^"?]+)"/g)].map((match) => match[1]);
  assert.ok(paths.some((path) => path.endsWith(".js")));
  for (const path of paths) {
    const asset = await request(path);
    assert.ok((await asset.arrayBuffer()).byteLength > 0);
    assert.equal(asset.headers.get("x-content-type-options"), "nosniff");
    const etag = asset.headers.get("etag");
    assert.ok(etag !== null);
    await request(path, { headers: { "If-None-Match": etag } }, 304);
    const head = await request(path, { method: "HEAD" });
    assert.equal((await head.arrayBuffer()).byteLength, 0);
  }
  for (const path of ["/.env", "/.data/rustsurvey.db", "/src/main.tsx", "/api/missing"]) {
    const missing = await request(path, {}, 404);
    assert.equal((await missing.arrayBuffer()).byteLength, 0);
  }
  assert.equal(await (await request("/api/health")).text(), "ok");
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
    { expectedRevision: 1, navigation: { format: "question", showProgress: false } },
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
    checkpoint: { language: "en", groupId: "details", pageId: "q:age" },
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
      navigation: { format: "question", showProgress: false },
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
  const defaults = await defaultsFixture(headers);
  return { id, headers, options, acknowledgement, invited, forms, defaults };
}
async function defaultsFixture(headers) {
  const corpus = JSON.parse(
    await readFile(join(root, "examples/rules/collection-forms.json"), "utf8"),
  );
  const definition = corpus.definition;
  definition.schemaVersion = 4;
  definition.languages = ["fr", "en"];
  definition.groups[0].questions = definition.groups[0].questions.filter((question) =>
    ["S", "T"].includes(question.code),
  );
  for (const question of definition.groups[0].questions) {
    question.relevance = "1";
    question.defaults = {
      sameForAllLanguages: false,
      values: [
        { coordinate: { kind: "primary" }, role: "answer", language: "fr", value: "défaut" },
        { coordinate: { kind: "primary" }, role: "answer", language: "en", value: "default" },
      ],
    };
    if (question.code === "T") question.defaults.values[0].value = "depuis {S}";
  }
  const [first, second] = definition.groups[0].questions;
  const study = await json("/api/admin/studies", "POST", definition, 201);
  const run = await json(
    `/api/admin/studies/${study.id}/runs`,
    "POST",
    {
      expectedRevision: 1,
      navigation: { format: "question", showProgress: true },
    },
    201,
  );
  assert.equal(run.profile, "native-defaults-v1");
  const id = "00000000-0000-4000-8000-000000000600";
  const started = await (
    await request(
      `/api/runs/${run.id}/responses`,
      {
        method: "POST",
        headers,
        body: JSON.stringify({ responseId: id, language: "fr" }),
      },
      201,
    )
  ).json();
  assert.deepEqual(started.answers, { S: "défaut" });
  assert.equal(started.checkpoint.pageId, `q:${first.id}`);
  await request(`/api/responses/${id}`, {
    method: "PUT",
    headers,
    body: JSON.stringify({
      expectedVersion: 0,
      operationId: "00000000-0000-4000-8000-000000000601",
      answers: { S: "modifié" },
      checkpoint: { language: "fr", groupId: definition.groups[0].id, pageId: `q:${second.id}` },
      submit: false,
    }),
  });
  const initialized = await (await request(`/api/responses/${id}`, { headers })).json();
  assert.deepEqual(initialized.answers, { S: "modifié", T: "depuis modifié" });
  const options = {
    method: "PUT",
    headers,
    body: JSON.stringify({
      expectedVersion: 1,
      operationId: "00000000-0000-4000-8000-000000000602",
      answers: { S: "modifié", T: "" },
      checkpoint: { language: "en", groupId: definition.groups[0].id, pageId: `q:${second.id}` },
      submit: false,
    }),
  };
  const acknowledgement = await (await request(`/api/responses/${id}`, options)).json();
  const original = Buffer.from(
    await (
      await request(`/api/admin/studies/${study.id}/revisions/1/definition.lss`)
    ).arrayBuffer(),
  );
  const imported = await (
    await request(
      "/api/admin/imports/lss",
      {
        method: "POST",
        headers: { "Content-Type": "application/xml" },
        body: original,
      },
      201,
    )
  ).json();
  assert.equal(imported.source.report.defaultProjection.status, "complete");
  assert.equal(imported.source.report.defaultProjection.projectedValues, 4);
  for (const question of imported.definition.groups[0].questions) {
    const expected = definition.groups[0].questions.find((source) => source.code === question.code);
    assert.equal(question.defaults.sameForAllLanguages, expected.defaults.sameForAllLanguages);
    assert.deepEqual(
      question.defaults.values.toSorted((left, right) =>
        left.language.localeCompare(right.language),
      ),
      expected.defaults.values.toSorted((left, right) =>
        left.language.localeCompare(right.language),
      ),
    );
  }
  const source = await importedCampaignFixture(study.id, headers);
  return { id, options, acknowledgement, imported, original, source };
}

async function importedCampaignFixture(studyId, headers) {
  const configuration = JSON.parse(
    await readFile(join(root, "examples/rules/import-execution.json"), "utf8"),
  );
  let xml = await (
    await request(`/api/admin/studies/${studyId}/revisions/1/definition.lss?surveyId=701`)
  ).text();
  for (const [name, value] of Object.entries(configuration.surveyOverrides)) {
    const field = `<fieldname>${name}</fieldname>`;
    if (xml.includes(field)) {
      assert.equal(name, "datestamp", "fixture overrides only the existing timestamp option");
      assert.ok(xml.includes("<datestamp>N</datestamp>"));
      xml = xml.replace("<datestamp>N</datestamp>", "<datestamp>Y</datestamp>");
      continue;
    }
    xml = xml.replace("<fieldname>sid</fieldname>", `<fieldname>sid</fieldname>${field}`);
    if (value !== null)
      xml = xml.replace("<sid>701</sid>", `<sid>701</sid><${name}>${value}</${name}>`);
  }
  const study = await (
    await request(
      "/api/admin/imports/lss",
      { method: "POST", headers: { "Content-Type": "application/xml" }, body: xml },
      201,
    )
  ).json();
  const context = await json(
    `/api/admin/studies/${study.id}/revisions/1/settings-contexts`,
    "POST",
    configuration.context,
    201,
  );
  const run = await json(
    `/api/admin/studies/${study.id}/runs`,
    "POST",
    { expectedRevision: 1, sourceExecution: { contextId: context.id, facts: configuration.facts } },
    201,
  );
  assert.equal(run.profile, "native-defaults-v1");
  const id = "00000000-0000-4000-8000-000000006600";
  const started = await (
    await request(
      `/api/runs/${run.id}/responses`,
      { method: "POST", headers, body: JSON.stringify({ responseId: id, language: "fr" }) },
      201,
    )
  ).json();
  assert.deepEqual(started.answers, { S: "défaut", T: "depuis défaut" });
  await request(`/api/responses/${id}`, {
    method: "PUT",
    headers,
    body: JSON.stringify({
      expectedVersion: 0,
      operationId: "00000000-0000-4000-8000-000000006601",
      answers: { S: "avant relance", T: "conservé" },
      checkpoint: started.checkpoint,
      submit: false,
    }),
  });
  const options = {
    method: "PUT",
    headers,
    body: JSON.stringify({
      expectedVersion: 1,
      operationId: "00000000-0000-4000-8000-000000006602",
      answers: { S: "après relance", T: "" },
      checkpoint: { ...started.checkpoint, language: "en" },
      submit: false,
    }),
  };
  return { id, run, options, definition: started.definition, acknowledgement: null };
}
async function formsFixture(headers) {
  const corpus = JSON.parse(
    await readFile(join(root, "examples/rules/collection-forms.json"), "utf8"),
  );
  const exclusive = corpus.definition.groups[0].questions.find((question) => question.code === "M");
  exclusive.attributes.push({ name: "exclude_all_others", value: "A2", language: null });
  corpus.baseline.answers.M_A2 = "Y";
  delete corpus.baseline.expected.M_A1;
  corpus.baseline.expected.M_A2 = "Y";
  const filtered = corpus.definition.groups[0].questions.find((question) => question.code === "F");
  filtered.attributes.push({ name: "array_filter", value: "P", language: null });
  const otherRow = structuredClone(filtered.subquestions[0]);
  otherRow.id = "restart-filter-other";
  otherRow.code = "other";
  filtered.subquestions.push(otherRow);
  corpus.baseline.answers.F_other = "02";
  corpus.baseline.answers.P_other = "";
  corpus.baseline.expected.P_other = "";
  delete corpus.baseline.expected.P_othercomment;
  delete corpus.baseline.expected.F_A2;
  for (const code of ["L", "R"]) {
    const target = corpus.definition.groups[0].questions.find((question) => question.code === code);
    target.attributes.push({ name: "array_filter", value: "M", language: null });
    if (code === "L") target.mandatory = "no";
    for (const [index, choice] of target.choices.entries()) choice.code = `A${index + 1}`;
  }
  corpus.baseline.answers.L = "A1";
  delete corpus.baseline.expected.L;
  corpus.baseline.answers.R_1 = "A2";
  corpus.baseline.answers.R_2 = "A1";
  corpus.baseline.expected.R_1 = "A2";
  delete corpus.baseline.expected.R_2;
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
  return {
    id,
    runId: run.id,
    studyId: study.id,
    headers,
    options,
    acknowledgement,
    expected: corpus.baseline.expected,
  };
}
async function verifyCollection(fixture) {
  const defaults = fixture.defaults;
  const source = defaults.source;
  const sourceBefore = await (
    await request(`/api/responses/${source.id}`, { headers: fixture.headers })
  ).json();
  assert.deepEqual(sourceBefore.definition, source.definition);
  const acknowledgement = await (
    await request(`/api/responses/${source.id}`, source.options)
  ).json();
  if (source.acknowledgement === null) source.acknowledgement = acknowledgement;
  assert.deepEqual(acknowledgement, source.acknowledgement);
  const resumedSource = await (
    await request(`/api/responses/${source.id}`, { headers: fixture.headers })
  ).json();
  assert.equal(resumedSource.version, 2);
  assert.deepEqual(resumedSource.answers, { S: "après relance", T: "" });
  assert.equal(resumedSource.checkpoint.language, "en");
  const imported = await json(`/api/studies/${defaults.imported.id}`);
  assert.deepEqual(imported.definition, defaults.imported.definition);
  assert.deepEqual(
    imported.source.report.defaultProjection,
    defaults.imported.source.report.defaultProjection,
  );
  const original = await request(`/api/admin/studies/${imported.id}/original.lss`);
  assert.deepEqual(Buffer.from(await original.arrayBuffer()), defaults.original);
  const defaultResponse = await (
    await request(`/api/responses/${defaults.id}`, { headers: fixture.headers })
  ).json();
  assert.equal(defaultResponse.run.profile, "native-defaults-v1");
  assert.equal(defaultResponse.version, 2);
  assert.equal(defaultResponse.checkpoint.language, "en");
  assert.deepEqual(defaultResponse.answers, { S: "modifié", T: "" });
  assert.deepEqual(
    await (await request(`/api/responses/${defaults.id}`, defaults.options)).json(),
    defaults.acknowledgement,
  );
  const stored = await (
    await request(`/api/responses/${fixture.id}`, { headers: fixture.headers })
  ).json();
  assert.equal(stored.version, 1);
  assert.deepEqual(stored.answers, { GATE: "Y", AGE: "24", NOTE: "été", SCORE: "25" });
  assert.deepEqual(stored.checkpoint, { language: "en", groupId: "details", pageId: "q:age" });
  assert.equal(stored.run.profile, "native-pages-v1");
  assert.deepEqual(stored.run.navigation, { format: "question", showProgress: false });
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

async function sourceImportFixture() {
  const lss = await readFile(join(root, "crates/limesurvey/tests/fixtures/minimal-623.lss"));
  const study = await (
    await request(
      "/api/admin/imports/lss",
      { method: "POST", body: lss, headers: { "Content-Type": "application/xml" } },
      201,
    )
  ).json();
  const fields = ["id", "submitdate", "701X20X101", "701X20X102SQ001"];
  const rows = Array.from(
    { length: 520 },
    (_, index) =>
      `<row><id>${index + 1}</id><_701X20X101>01</_701X20X101><_701X20X102SQ001/></row>`,
  ).join("");
  const xml = `<?xml version="1.0" encoding="UTF-8"?><document><LimeSurveyDocType>Responses</LimeSurveyDocType><DBVersion>623</DBVersion><languages><language>fr</language></languages><responses><fields>${fields.map((field) => `<fieldname>${field}</fieldname>`).join("")}</fields><rows>${rows}</rows></responses></document>`;
  const bytes = Buffer.from(xml);
  const batch = await (
    await request(
      `/api/admin/studies/${study.id}/response-imports?revision=1`,
      { method: "POST", body: bytes, headers: { "Content-Type": "application/xml" } },
      202,
    )
  ).json();
  assert.equal(batch.loadedRows, 500);
  const sourceContext = Buffer.from(
    JSON.stringify({
      schemaVersion: 1,
      surveyId: "701",
      surveyGroupId: "2",
      groups: [{ id: "2", parentId: null, values: { showwelcome: "N" } }],
      global: {},
    }),
  );
  const context = await (
    await request(
      `/api/admin/studies/${study.id}/revisions/1/settings-contexts`,
      {
        method: "POST",
        body: sourceContext,
        headers: { "Content-Type": "application/json" },
      },
      201,
    )
  ).json();
  return { studyId: study.id, batchId: batch.id, bytes, contextId: context.id, sourceContext };
}

async function verifySourceImport(fixture) {
  const original = await request(`/api/admin/response-imports/${fixture.batchId}/original.lsr`);
  assert.equal(original.headers.get("cache-control"), "no-store");
  assert.deepEqual(Buffer.from(await original.arrayBuffer()), fixture.bytes);
  const batch = await json(`/api/admin/response-imports/${fixture.batchId}/continue`, "POST", {});
  assert.equal(batch.state, "prepared");
  assert.equal(batch.loadedRows, 520);
  const last = await json(`/api/admin/response-imports/${fixture.batchId}/records/519/record.json`);
  assert.equal(last.id, "520");
  assert.equal(last["701X20X101"], "01");
  assert.equal(last["701X20X102SQ001"], "");
  assert.ok(!Object.hasOwn(last, "submitdate"));
  const converted = await json(
    `/api/admin/response-imports/${fixture.batchId}/records/519/native-cells.json`,
  );
  const mapping = await json(`/api/admin/response-imports/${fixture.batchId}/mapping.json`);
  const primary = mapping.bindings.find((field) => field.column === "701X20X101");
  const detail = mapping.bindings.find((field) => field.column === "701X20X102SQ001");
  assert.equal(converted.projection.answers[primary.runtimeKey], "01");
  assert.equal(converted.projection.answers[detail.runtimeKey], "");
  assert.equal(converted.projection.metadata.submitdate, null);
  assert.ok(!Object.hasOwn(converted.projection.metadata, "startlanguage"));
  assert.equal(converted.execution, "blocked");
  assert.equal(converted.continuity, "unavailable");
  const context = await request(
    `/api/admin/source-settings-contexts/${fixture.contextId}/original.json`,
  );
  assert.equal(context.headers.get("cache-control"), "no-store");
  assert.deepEqual(Buffer.from(await context.arrayBuffer()), fixture.sourceContext);
  const report = await json(
    `/api/admin/studies/${fixture.studyId}/revisions/1/collection-check?contextId=${fixture.contextId}`,
  );
  assert.equal(report.sourceContext.id, fixture.contextId);
  assert.equal(
    report.groupSettings.settings.find((setting) => setting.name === "showwelcome").value,
    false,
  );
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
  const sourceImport = await sourceImportFixture();
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
  await verifySourceImport(sourceImport);
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
  await json(`/api/admin/runs/${collection.forms.runId}/close`, "POST", {});
  const exported = await request(`/api/admin/runs/${collection.forms.runId}/responses.vv`);
  assert.equal(exported.headers.get("cache-control"), "no-store");
  assert.equal(exported.headers.get("transfer-encoding"), "chunked");
  const vv = await exported.text();
  const lines = vv.trimEnd().split("\n");
  assert.equal(lines.length, 3);
  const values = Object.fromEntries(
    lines[1].split("\t").map((key, index) => [key, lines[2].split("\t")[index]]),
  );
  assert.equal(values.L, "{question_not_shown}");
  assert.equal(values.R_1, "A2");
  assert.equal(values.R_2, "{question_not_shown}");
  assert.equal(values.DUAL_A1_1, "10");
  assert.equal(values.CALC, "10");
  assert.equal(values.startlanguage, "fr");
  assert.ok(!vv.includes(collection.forms.id));
  const lss = await request(
    `/api/admin/studies/${collection.forms.studyId}/revisions/1/definition.lss`,
  );
  assert.match(await lss.text(), /<DBVersion>623<\/DBVersion>/);
  await verifySourceImport(sourceImport);
  const fileSourceImport = await (
    await request(
      `/api/admin/studies/${sourceImport.studyId}/response-imports?revision=1`,
      {
        method: "POST",
        body: Buffer.from(sourceImport.bytes.toString().replace("<id>1</id>", "<id>7001</id>")),
        headers: { "Content-Type": "application/xml" },
      },
      202,
    )
  ).json();
  const fileFixture = {
    ...sourceImport,
    batchId: fileSourceImport.id,
    bytes: Buffer.from(sourceImport.bytes.toString().replace("<id>1</id>", "<id>7001</id>")),
  };
  await stop();
  await start("file");
  await verifySourceImport(fileFixture);
  await verifyCollection(collection);
  const source = collection.defaults.source;
  await json(`/api/admin/runs/${source.run.id}/close`, "POST", {});
  const sourceVv = await (await request(`/api/admin/runs/${source.run.id}/responses.vv`)).text();
  assert.ok(sourceVv.includes("après relance"));
  const sourceLss = await (await request(`/api/admin/runs/${source.run.id}/definition.lss`)).text();
  assert.match(sourceLss, /<sid>701<\/sid>/);
  if (process.env.RUSTSURVEY_TEST_S3_ENDPOINT !== undefined) {
    assert.ok(process.env.RUSTSURVEY_TEST_S3_ENDPOINT.startsWith("http://127.0.0.1:"));
    await stop();
    await start("s3");
    const s3Media = "00000000-0000-4000-8000-000000000012";
    const uploadPath = `/api/admin/studies/${study.id}/media/${s3Media}?name=s3.png`;
    const options = {
      method: "PUT",
      body: image,
      headers: { "Content-Type": "application/octet-stream" },
    };
    const uploaded = await (await request(uploadPath, options)).json();
    assert.equal(uploaded.state, "ready");
    const s3Bytes = Buffer.from(
      sourceImport.bytes.toString().replace("<id>1</id>", "<id>8001</id>"),
    );
    const s3Import = await (
      await request(
        `/api/admin/studies/${sourceImport.studyId}/response-imports?revision=1`,
        { method: "POST", body: s3Bytes, headers: { "Content-Type": "application/xml" } },
        202,
      )
    ).json();
    await stop();
    await start("s3");
    assert.deepEqual(await (await request(uploadPath, options)).json(), uploaded);
    const read = await request(`/api/admin/studies/${study.id}/media/${s3Media}`);
    assert.deepEqual(Buffer.from(await read.arrayBuffer()), image);
    const original = await request(`/api/admin/response-imports/${s3Import.id}/original.lsr`);
    assert.deepEqual(Buffer.from(await original.arrayBuffer()), s3Bytes);
    console.log(
      "HTTP réel S3 : image privée, accusé répété et original LSR préservés après arrêt complet du serveur.",
    );
  }
  console.log(
    "HTTP réel : stockage, médias, collecte native/importée, invitations, LSS/VV et préparation LSR vérifiés après arrêt et relance en modes SQL/fichiers.",
  );
} finally {
  await stop();
  const resolvedTemporary = await realpath(temporary);
  assert.equal(dirname(resolvedTemporary).toLowerCase(), (await realpath(tmpdir())).toLowerCase());
  assert.ok(basename(resolvedTemporary).startsWith("rustsurvey-local-http-"));
  await rm(resolvedTemporary, { recursive: true, force: true });
}
