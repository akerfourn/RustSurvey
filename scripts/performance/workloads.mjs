import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";

export function identifier(serial) {
  return `00000000-0000-4000-8000-${String(serial).padStart(12, "0")}`;
}

export async function workload(name) {
  const simple = name === "small";
  const file = simple ? "collection.json" : "collection-forms.json";
  const corpus = JSON.parse(
    await readFile(new URL(`../../examples/rules/${file}`, import.meta.url), "utf8"),
  );
  const definition = corpus.definition;
  const answers = simple
    ? { GATE: "Y", AGE: "24", NOTE: "été", SCORE: "forged" }
    : corpus.baseline.answers;
  if (name === "defaults") {
    definition.schemaVersion = 4;
    for (const code of ["S", "T", "Q"]) {
      const question = definition.groups[0].questions.find((question) => question.code === code);
      let coordinate = { kind: "primary" };
      if (code === "Q")
        coordinate = { kind: "subquestion", id: question.subquestions[0].id, scale: null };
      question.defaults = {
        sameForAllLanguages: true,
        values: [
          {
            coordinate,
            role: "answer",
            language: "fr",
            value: code === "T" ? "initial {sum(K_A1, K_A2)}" : "initial",
          },
        ],
      };
    }
  }
  return {
    name,
    definition,
    answers,
    editedField: simple ? "NOTE" : "U",
    computedField: simple ? "SCORE" : "CALC",
    computedValue: simple ? "25" : "10",
    definitionSha256: createHash("sha256").update(JSON.stringify(definition)).digest("hex"),
  };
}

export async function campaign(server, fixture) {
  const study = await server.json(
    "/api/admin/studies",
    fixture.definition,
    { method: "POST" },
    201,
  );
  const run = await server.json(
    `/api/admin/studies/${study.id}/runs`,
    { expectedRevision: 1, navigation: { format: "all", showProgress: false } },
    { method: "POST" },
    201,
  );
  assert.equal(run.profile, fixture.name === "defaults" ? "native-defaults-v1" : "native-pages-v1");
  return run;
}

export async function participant(server, run, serial) {
  const id = identifier(serial);
  return server.json(
    `/api/runs/${run.id}/responses`,
    { responseId: id, language: "fr" },
    { method: "POST", participant: true },
    201,
  );
}

export function command(worker, fixture, serial) {
  return {
    expectedVersion: worker.version,
    operationId: identifier(serial),
    answers: { ...worker.answers, [fixture.editedField]: `été${serial % 2}` },
    checkpoint: { language: "fr", groupId: null, pageId: "all" },
    submit: false,
  };
}

export async function save(server, worker, input) {
  const response = await server.request(`/api/responses/${worker.id}`, input, {
    method: "PUT",
    participant: true,
  });
  const result = await response.json();
  if (response.status === 200) {
    assert.equal(result.id, worker.id);
    assert.equal(result.version, worker.version + 1);
    assert.equal(result.status, "draft");
    assert.deepEqual(result.checkpoint, input.checkpoint);
    worker.version = result.version;
    worker.answers = input.answers;
  }
  let code = "ok";
  if (response.status !== 200) {
    code = result.error?.code ?? "unknown_error";
    if (response.status !== 503 || !["collection_busy", "write_busy"].includes(code))
      throw new Error(`Unexpected benchmark response: HTTP ${response.status}, ${code}`);
  }
  return { status: response.status, code };
}

export async function verify(server, worker, fixture) {
  const stored = await server.json(`/api/responses/${worker.id}`, undefined, { participant: true });
  assert.equal(stored.version, worker.version);
  assert.equal(stored.answers[fixture.editedField], worker.answers[fixture.editedField]);
  assert.equal(stored.answers[fixture.computedField], fixture.computedValue);
  assert.deepEqual(stored.checkpoint, { language: "fr", groupId: null, pageId: "all" });
  return stored;
}
