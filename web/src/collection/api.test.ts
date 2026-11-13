import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { ApiError } from "../authoring/api.ts";
import {
  getParticipation,
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
