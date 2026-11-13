import assert from "node:assert/strict";
import { test } from "node:test";
import { ApiError, decodeDefinition, downloadLss, importDefinition, saveStudy } from "./api.ts";
import { createDefinition, createQuestion } from "./model.ts";

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
      assert.match(error.message, /8 Mio/);
      return true;
    },
  );
});

test("test_decoder_should_reject_future_versions_and_unknown_question_types", () => {
  const definition = createDefinition(() => "group-1");
  assert.deepEqual(decodeDefinition(definition), definition);
  assert.throws(() => decodeDefinition({ ...definition, schemaVersion: 4 }), /Version/);
  definition.groups[0]!.questions.push(createQuestion(definition, () => "question-1"));
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

test("test_native_import_should_preserve_unknown_fields_for_server_rejection_instead_of_losing_them", async (context) => {
  const definition = { ...createDefinition(() => "g1"), futureRule: "must not disappear" };
  let body: unknown;
  context.mock.method(globalThis, "fetch", async (_url: string, options: RequestInit) => {
    body = JSON.parse(String(options.body));
    return new Response(JSON.stringify({ error: { message: "Unknown field.", issues: [] } }), {
      status: 422,
    });
  });
  const file = new File([JSON.stringify(definition)], "definition.json");
  await assert.rejects(
    importDefinition(file, "csrf", new AbortController().signal),
    /Unknown field/,
  );
  assert.deepEqual(body, definition);
});

test("test_v2_decoder_should_upgrade_without_losing_text_or_source_aliases", () => {
  const definition = createDefinition(() => "g1");
  const question = createQuestion(definition, () => "q1");
  question.source = { id: "1234", kind: "S" };
  definition.groups[0]!.questions.push(question);
  const old = { ...definition, schemaVersion: 2 };
  const upgraded = decodeDefinition(old);
  assert.equal(upgraded.schemaVersion, 3);
  assert.deepEqual(upgraded.groups[0]!.questions[0]!.source, question.source);
  assert.deepEqual(upgraded.groups[0]!.questions[0]!.media, []);
});

test("test_lss_download_should_show_export_blockers_without_navigating_away_from_the_editor", async (context) => {
  context.mock.method(
    globalThis,
    "fetch",
    async () =>
      new Response(
        JSON.stringify({ error: { message: "Anciennes conditions : réexport bloqué." } }),
        { status: 422 },
      ),
  );
  await assert.rejects(downloadLss("study-1", 2, new AbortController().signal), /réexport bloqué/);
});
