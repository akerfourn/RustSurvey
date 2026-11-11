import assert from "node:assert/strict";
import { test } from "node:test";
import { ApiError, decodeDefinition, saveStudy } from "./api.ts";
import { createDefinition } from "./model.ts";

test("test_save_should_explain_oversized_payload_rejections_even_without_json", async (context) => {
  context.mock.method(globalThis, "fetch", async () => new Response("too large", { status: 413 }));
  await assert.rejects(
    saveStudy(
      null,
      0,
      createDefinition(() => "g1"),
      "csrf",
    ),
    (error: unknown) => {
      assert.ok(error instanceof ApiError);
      assert.equal(error.status, 413);
      assert.match(error.message, /1 Mio/);
      return true;
    },
  );
});

test("test_decoder_should_reject_future_versions_and_unknown_question_types", () => {
  const definition = createDefinition(() => "group-1");
  assert.deepEqual(decodeDefinition(definition), definition);
  assert.throws(() => decodeDefinition({ ...definition, schemaVersion: 2 }), /Version/);
  definition.groups[0]!.questions.push({
    id: "question-1",
    code: "Q1",
    title: "Question",
    help: "",
    kind: "short_text",
    mandatory: "no",
    choices: [],
  });
  const invalid: unknown = {
    ...definition,
    groups: [
      {
        ...definition.groups[0],
        questions: [{ ...definition.groups[0]!.questions[0], kind: "future" }],
      },
    ],
  };
  assert.throws(() => decodeDefinition(invalid), /Type de question inconnu/);
});

test("test_save_should_send_csrf_and_expected_revision_without_persisting_credentials_in_the_document", async (context) => {
  const definition = createDefinition(() => "group-1");
  const controller = new AbortController();
  let sentUrl: string | null = null;
  let sentOptions: RequestInit | undefined;
  context.mock.method(globalThis, "fetch", async (url: string, options: RequestInit) => {
    sentUrl = url;
    sentOptions = options;
    return new Response(
      JSON.stringify({ id: "study-1", revision: 4, createdAt: 1, updatedAt: 2, definition }),
    );
  });
  const stored = await saveStudy("study-1", 3, definition, "csrf-for-test", controller.signal);
  assert.equal(stored.revision, 4);
  assert.equal(sentUrl, "/api/admin/studies/study-1");
  assert.equal(sentOptions?.credentials, "same-origin");
  assert.equal(sentOptions?.signal, controller.signal);
  assert.equal(new Headers(sentOptions?.headers).get("X-CSRF-Token"), "csrf-for-test");
  assert.deepEqual(JSON.parse(String(sentOptions?.body)), { expectedRevision: 3, definition });
});

test("test_save_should_surface_conflicts_and_server_validation_paths_to_the_editor", async (context) => {
  const definition = createDefinition(() => "group-1");
  context.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(
        JSON.stringify({
          error: {
            code: "revision_conflict",
            message: "Rechargez la version.",
            issues: [{ path: "title", message: "Titre requis." }],
          },
        }),
        { status: 409 },
      ),
  );
  await assert.rejects(saveStudy("study-1", 1, definition, "csrf"), (error: unknown) => {
    assert.ok(error instanceof ApiError);
    assert.equal(error.status, 409);
    assert.equal(error.message, "Rechargez la version.");
    assert.deepEqual(error.issues, [{ path: "title", message: "Titre requis." }]);
    return true;
  });
});
