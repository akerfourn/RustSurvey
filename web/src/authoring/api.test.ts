import assert from "node:assert/strict";
import { test } from "node:test";
import {
  ApiError,
  decodeDefinition,
  downloadLss,
  getPublicStudy,
  getStudy,
  importDefinition,
  listStudies,
  publishStudy,
  saveStudy,
} from "./api.ts";
import { publicMediaPath, publicStudyHash } from "./publication-links.ts";
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
  assert.throws(() => decodeDefinition({ ...definition, schemaVersion: 5 }), /Version/);
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
      JSON.stringify({
        id: "study-1",
        revision: 4,
        publishedRevision: 2,
        createdAt: 1,
        updatedAt: 2,
        definition,
      }),
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
  assert.equal(upgraded.schemaVersion, 4);
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

test("test_public_visit_should_request_its_pinned_revision_and_never_fall_back_to_the_latest", async (context) => {
  const paths: string[] = [];
  const definition = createDefinition(() => "g1");
  context.mock.method(globalThis, "fetch", async (path: string) => {
    paths.push(path);
    if (path.endsWith("/revisions/3")) {
      return new Response(JSON.stringify({ error: { message: "Révision non publiée." } }), {
        status: 404,
      });
    }
    let revision = 2;
    if (path.endsWith("/revisions/1")) revision = 1;
    return new Response(
      JSON.stringify({
        id: "s1",
        revision,
        publishedRevision: revision,
        createdAt: 1,
        updatedAt: 1,
        definition,
      }),
    );
  });
  const signal = new AbortController().signal;
  assert.equal((await getPublicStudy("s1", null, signal)).revision, 2);
  assert.equal((await getPublicStudy("s1", 1, signal)).revision, 1);
  await assert.rejects(getPublicStudy("s1", 3, signal), /Révision non publiée/);
  assert.deepEqual(paths, [
    "/api/studies/s1",
    "/api/studies/s1/revisions/1",
    "/api/studies/s1/revisions/3",
  ]);
  assert.equal(publicStudyHash("s1"), "#/studies/s1");
  assert.equal(publicStudyHash("s1", 1), "#/studies/s1/revisions/1");
  assert.equal(publicMediaPath("s1", 1), "/api/studies/s1/revisions/1/media");
});

test("test_administrator_should_load_private_drafts_and_publish_only_the_reviewed_revision", async (context) => {
  const paths: string[] = [];
  const signal = new AbortController().signal;
  context.mock.method(globalThis, "fetch", async (path: string, options: RequestInit) => {
    paths.push(path);
    if (path.endsWith("/publish")) {
      assert.equal(options.method, "POST");
      assert.equal(new Headers(options.headers).get("X-CSRF-Token"), "test-csrf");
      assert.deepEqual(JSON.parse(String(options.body)), { expectedRevision: 4 });
      return new Response(JSON.stringify({ publishedRevision: 4 }));
    }
    if (path.includes("?offset="))
      return new Response(JSON.stringify({ items: [], hasMore: false }));
    return new Response(
      JSON.stringify({
        id: "s1",
        revision: 4,
        publishedRevision: 2,
        createdAt: 1,
        updatedAt: 1,
        definition: createDefinition(() => "g1"),
      }),
    );
  });
  const draft = await getStudy("s1", signal);
  assert.equal(draft.revision, 4);
  assert.equal(draft.publishedRevision, 2);
  await listStudies(0, signal, true);
  assert.equal(await publishStudy("s1", 4, "test-csrf", signal), 4);
  assert.deepEqual(paths, [
    "/api/admin/studies/s1",
    "/api/admin/studies?offset=0",
    "/api/admin/studies/s1/publish",
  ]);
});
