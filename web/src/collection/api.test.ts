import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { ApiError } from "../authoring/api.ts";
import {
  getParticipation,
  createRun,
  createImportedRun,
  invitationFragment,
  resumeFragment,
  saveParticipation,
  startParticipation,
  type SaveCommand,
} from "./api.ts";

const id = "00000000-0000-4000-8000-000000000001";
const runId = "00000000-0000-4000-8000-000000000002";
const token = "a".repeat(64);
const definition: unknown = JSON.parse(
  readFileSync(new URL("../../../examples/rules/collection.json", import.meta.url), "utf8"),
).definition;
const checkpoint = { language: "fr", groupId: "main" };
const run = {
  id: runId,
  studyId: id,
  revision: 1,
  profile: "native-simple-v1",
  isOpen: true,
  createdAt: 100,
  participationPolicy: { mode: "open" },
};
const stored = {
  id,
  run,
  definition,
  version: 0,
  status: "draft",
  answers: {},
  checkpoint,
  createdAt: 100,
  updatedAt: 100,
};

test("test_imported_campaign_should_send_the_explicit_context_and_facts_without_native_policy_overrides", async () => {
  const original = globalThis.fetch;
  const navigation = { format: "group", showProgress: true } as const;
  const facts = {
    schemaVersion: 1,
    surveyId: "701",
    hasParticipantsTable: false,
    deleteNonValues: true,
    showNoAnswer: true,
    effectiveTheme: "vanilla",
    activePlugins: 0,
    timeZone: "UTC",
    timeAdjust: "0",
  } as const;
  const controller = new AbortController();
  let body: unknown;
  globalThis.fetch = async (_input, options = {}) => {
    body = JSON.parse(String(options.body));
    assert.equal(new Headers(options.headers).get("x-csrf-token"), "synthetic-csrf");
    assert.equal(options.signal, controller.signal);
    return Response.json({ ...run, profile: "native-pages-v1", navigation });
  };
  try {
    const result = await createImportedRun(
      id,
      1,
      "synthetic-csrf",
      runId,
      facts,
      controller.signal,
    );
    assert.deepEqual(result.navigation, navigation);
    assert.deepEqual(body, { expectedRevision: 1, sourceExecution: { contextId: runId, facts } });
  } finally {
    globalThis.fetch = original;
  }
});

test("test_native_page_campaign_should_send_frozen_settings_and_decode_the_exact_question_cursor", async () => {
  const original = globalThis.fetch;
  const navigation = { format: "question", showProgress: false } as const;
  const pageRun = { ...run, profile: "native-pages-v1", navigation };
  const page = { language: "en", groupId: "details", pageId: "q:age" };
  let body: unknown;
  globalThis.fetch = async (_input, options = {}) => {
    if (options.method === "POST") {
      body = JSON.parse(String(options.body));
      return Response.json(pageRun);
    }
    return Response.json({ ...stored, run: pageRun, checkpoint: page });
  };
  try {
    assert.deepEqual(
      (await createRun(id, 1, "synthetic-csrf", { mode: "open" }, navigation)).navigation,
      navigation,
    );
    assert.deepEqual(body, {
      expectedRevision: 1,
      participationPolicy: { mode: "open" },
      navigation,
    });
    const resumed = await getParticipation(id, token);
    assert.deepEqual(resumed.checkpoint, page);
    assert.deepEqual(resumed.run.navigation, navigation);
  } finally {
    globalThis.fetch = original;
  }
});

test("test_campaign_decoder_should_refuse_unknown_profiles_or_mismatched_navigation_without_fallbacks", async () => {
  const original = globalThis.fetch;
  try {
    for (const override of [
      { profile: "native-pages-v1", navigation: null },
      { profile: "native-defaults-v1", navigation: null },
      { profile: "future-profile", navigation: null },
      { profile: "native-forms-v1", navigation: { format: "all", showProgress: true } },
      { profile: "native-pages-v1", navigation: { format: "unknown", showProgress: true } },
      {
        profile: "native-pages-v1",
        navigation: { format: "all", showProgress: true, allowPrevious: false },
      },
    ]) {
      globalThis.fetch = async () => Response.json({ ...stored, run: { ...run, ...override } });
      await assert.rejects(
        getParticipation(id, token),
        /Présentation de campagne invalide|Profil de campagne incompatible/,
      );
    }
    globalThis.fetch = async () => Response.json(stored);
    const legacy = await getParticipation(id, token);
    assert.equal(legacy.run.navigation, null);
    assert.deepEqual(legacy.checkpoint, checkpoint);
  } finally {
    globalThis.fetch = original;
  }
});

test("test_default_campaign_should_decode_its_pinned_profile_and_initialized_answers", async () => {
  const original = globalThis.fetch;
  globalThis.fetch = async () =>
    Response.json({
      ...stored,
      run: {
        ...run,
        profile: "native-defaults-v1",
        navigation: { format: "question", showProgress: true },
      },
      answers: { GATE: "Y" },
      checkpoint: { ...checkpoint, pageId: "q:gate" },
    });
  try {
    const result = await getParticipation(id, token);
    assert.equal(result.run.profile, "native-defaults-v1");
    assert.deepEqual(result.answers, { GATE: "Y" });
    assert.equal(result.checkpoint.pageId, "q:gate");
  } finally {
    globalThis.fetch = original;
  }
});

test("test_start_and_resume_should_keep_the_secret_in_authorization_and_out_of_http_urls_and_bodies", async () => {
  const original = globalThis.fetch;
  const calls: { url: string; options: RequestInit }[] = [];
  globalThis.fetch = async (input, options = {}) => {
    calls.push({ url: String(input), options });
    return Response.json(stored);
  };
  try {
    await startParticipation(runId, id, token, "fr");
    await getParticipation(id, token);
    assert.equal(calls.length, 2);
    for (const call of calls) {
      assert.ok(!call.url.includes(token));
      assert.ok(!String(call.options.body).includes(token));
      assert.equal(new Headers(call.options.headers).get("Authorization"), `Bearer ${token}`);
      assert.equal(call.options.cache, "no-store");
    }
    assert.equal(resumeFragment(id, token), `#/responses/${id}/${token}`);
  } finally {
    globalThis.fetch = original;
  }
});

test("test_save_retry_should_reuse_the_exact_operation_and_expected_version_after_a_lost_acknowledgement", async () => {
  const original = globalThis.fetch;
  const bodies: string[] = [];
  const command: SaveCommand = {
    expectedVersion: 0,
    operationId: id,
    answers: { GATE: "N" },
    checkpoint,
    submit: true,
  };
  globalThis.fetch = async (_input, options = {}) => {
    bodies.push(String(options.body));
    if (bodies.length === 1) throw new TypeError("synthetic connection failure");
    return Response.json({
      id,
      version: 1,
      status: "submitted",
      checkpoint,
      updatedAt: 101,
      warnings: [],
    });
  };
  try {
    await assert.rejects(saveParticipation(id, token, command), {
      name: "TypeError",
      message: "synthetic connection failure",
    });
    const acknowledgement = await saveParticipation(id, token, command);
    assert.deepEqual(bodies, [JSON.stringify(command), JSON.stringify(command)]);
    assert.equal(acknowledgement.status, "submitted");
    assert.equal(acknowledgement.version, 1);
  } finally {
    globalThis.fetch = original;
  }
});

test("test_save_should_surface_server_validation_and_reject_unknown_response_states", async () => {
  const original = globalThis.fetch;
  const command: SaveCommand = {
    expectedVersion: 0,
    operationId: id,
    answers: {},
    checkpoint,
    submit: true,
  };
  try {
    globalThis.fetch = async () =>
      Response.json(
        {
          error: {
            message: "Corrigez les réponses.",
            issues: [{ path: "gate", message: "Cette question est obligatoire." }],
          },
        },
        { status: 422 },
      );
    await assert.rejects(
      saveParticipation(id, token, command),
      (error: unknown) =>
        error instanceof ApiError &&
        error.status === 422 &&
        error.issues[0]?.message === "Cette question est obligatoire.",
    );
    globalThis.fetch = async () => Response.json({ ...stored, status: "unknown" });
    await assert.rejects(getParticipation(id, token), {
      name: "Error",
      message: "État de réponse invalide.",
    });
  } finally {
    globalThis.fetch = original;
  }
});

test("test_invited_participation_should_keep_both_secrets_out_of_http_urls_and_response_bodies", async () => {
  const original = globalThis.fetch;
  const invitation = "b".repeat(64);
  const calls: { url: string; options: RequestInit }[] = [];
  globalThis.fetch = async (input, options = {}) => {
    calls.push({ url: String(input), options });
    if (options.method === "PUT")
      return Response.json({
        id,
        version: 1,
        status: "submitted",
        checkpoint,
        updatedAt: 101,
        warnings: [],
      });
    return Response.json({
      ...stored,
      run: {
        ...run,
        participationPolicy: {
          mode: "invitation",
          persistence: false,
          allowEditAfterSubmit: false,
        },
      },
    });
  };
  try {
    await startParticipation(runId, id, token, "fr", undefined, invitation);
    await saveParticipation(
      id,
      token,
      { expectedVersion: 0, operationId: id, answers: {}, checkpoint, submit: true },
      undefined,
      invitation,
    );
    for (const call of calls) {
      const headers = new Headers(call.options.headers);
      assert.equal(headers.get("Authorization"), `Bearer ${token}`);
      assert.equal(headers.get("X-Participation-Token"), invitation);
      for (const secret of [token, invitation]) {
        assert.ok(!call.url.includes(secret));
        assert.ok(!String(call.options.body).includes(secret));
      }
    }
    assert.equal(
      resumeFragment(id, token, invitation),
      `#/responses/${id}/${token}/invite/${invitation}`,
    );
    assert.equal(invitationFragment(runId, invitation), `#/runs/${runId}/invite/${invitation}`);
  } finally {
    globalThis.fetch = original;
  }
});
