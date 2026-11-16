import assert from "node:assert/strict";
import { test } from "node:test";
import { ApiError } from "../authoring/api.ts";
import type { Acknowledgement, SaveCommand } from "./api.ts";
import type { ViewerDraft } from "./types.ts";
import { createResponseWriter } from "./writer.ts";

function progress(value: string): ViewerDraft {
  return { answers: new Map([["NAME", value]]), language: "fr", groupId: "g1", pageId: "q:one" };
}
function acknowledge(command: SaveCommand): Acknowledgement {
  let status: Acknowledgement["status"] = "draft";
  if (command.submit) status = "submitted";
  return {
    id: "response-1",
    version: command.expectedVersion + 1,
    status,
    checkpoint: command.checkpoint,
    updatedAt: 2,
    warnings: [],
  };
}

test("test_writer_should_retry_the_exact_command_after_a_lost_acknowledgement", async () => {
  const commands: SaveCommand[] = [];
  let serial = 0;
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      commands.push(command);
      if (commands.length === 1) throw new TypeError("Synthetic connection lost after commit");
      return acknowledge(command);
    },
    () => `operation-${++serial}`,
  );
  const signal = new AbortController().signal;
  await assert.rejects(writer.save(progress("01"), true, signal), /connection lost/);
  assert.equal(writer.uncertain(), true);
  const saved = await writer.save(progress("01"), false, signal);
  assert.equal(saved.acknowledgement.status, "submitted");
  assert.equal(saved.acknowledgement.version, 1);
  assert.equal(saved.answers.get("NAME"), "01");
  assert.deepEqual(commands[0], commands[1]);
  assert.equal(serial, 1, "resolving a lost submission must not start another draft");
  assert.equal(writer.uncertain(), false);
});

test("test_writer_should_settle_an_uncertain_save_before_sending_newer_values", async () => {
  const commands: SaveCommand[] = [];
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      commands.push(command);
      if (commands.length === 1) throw new TypeError("Synthetic connection lost");
      return acknowledge(command);
    },
    () => `operation-${commands.length}`,
  );
  const signal = new AbortController().signal;
  await assert.rejects(writer.save(progress("first"), false, signal), /connection lost/);
  const saved = await writer.save(progress("newer"), false, signal);
  assert.equal(saved.acknowledgement.version, 2);
  assert.equal(saved.answers.get("NAME"), "newer");
  assert.deepEqual(commands[0], commands[1]);
  assert.equal(commands[2]!.expectedVersion, 1);
  assert.notEqual(commands[2]!.operationId, commands[1]!.operationId);
});

test("test_writer_should_allow_correcting_rejected_data_without_advancing_the_version", async () => {
  const commands: SaveCommand[] = [];
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      commands.push(command);
      if (commands.length === 1) throw new ApiError(422, "Synthetic validation rejection");
      return acknowledge(command);
    },
    () => `operation-${commands.length}`,
  );
  const signal = new AbortController().signal;
  await assert.rejects(writer.save(progress("invalid"), false, signal), /validation rejection/);
  assert.equal(writer.uncertain(), false);
  const saved = await writer.save(progress("valid"), false, signal);
  assert.equal(saved.acknowledgement.version, 1);
  assert.notEqual(commands[0]!.operationId, commands[1]!.operationId);
});

test("test_writer_should_preserve_the_campaign_api_for_authorized_post_submission_corrections", async () => {
  const commands: SaveCommand[] = [];
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      commands.push(command);
      return acknowledge(command);
    },
    () => `operation-${commands.length}`,
  );
  const signal = new AbortController().signal;
  await writer.save(progress("first submission"), true, signal);
  const corrected = await writer.save(progress("authorized correction"), true, signal);
  assert.equal(corrected.acknowledgement.version, 2);
  assert.equal(corrected.acknowledgement.status, "submitted");
  assert.equal(commands[1]!.expectedVersion, 1);
  assert.equal(corrected.answers.get("NAME"), "authorized correction");
});

test("test_writer_should_leave_conflict_resolution_to_reload_instead_of_replaying_an_obsolete_command", async () => {
  const writer = createResponseWriter(
    "response-1",
    0,
    async () => {
      throw new ApiError(409, "Response changed or run closed");
    },
    () => "operation-1",
  );
  await assert.rejects(
    writer.save(progress("local"), false, new AbortController().signal),
    /run closed/,
  );
  assert.equal(writer.uncertain(), false);
});

test("test_writer_should_keep_its_pending_command_when_the_acknowledgement_is_incoherent", async () => {
  const commands: SaveCommand[] = [];
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      commands.push(command);
      if (commands.length === 1) return { ...acknowledge(command), id: "another-response" };
      return acknowledge(command);
    },
    () => "operation-1",
  );
  const signal = new AbortController().signal;
  await assert.rejects(writer.save(progress("local"), false, signal), /incohérent/);
  assert.equal(writer.uncertain(), true);
  const saved = await writer.save(progress("local"), false, signal);
  assert.equal(saved.acknowledgement.version, 1);
  assert.deepEqual(commands[0], commands[1]);
});
