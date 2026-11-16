import assert from "node:assert/strict";
import { test } from "node:test";
import type { Acknowledgement, SaveCommand } from "./api.ts";
import type { ViewerDraft } from "./types.ts";
import {
  acknowledgeProgress,
  beginSave,
  editProgress,
  hasUnsavedChanges,
  initialProgress,
} from "./progress.ts";
import { createResponseWriter } from "./writer.ts";

function progress(value: string, language = "fr", pageId = "q:one"): ViewerDraft {
  return { answers: new Map([["NAME", value]]), language, groupId: "g1", pageId };
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

test("test_progress_should_apply_the_acknowledged_snapshot_when_there_are_no_newer_edits", () => {
  const edited = editProgress(initialProgress(progress("")), progress("sent"));
  const saving = beginSave(edited);
  const saved = acknowledgeProgress(saving, saving.editVersion, progress("sent", "en", "q:next"));
  assert.equal(hasUnsavedChanges(saving), true);
  assert.equal(hasUnsavedChanges(saved), false);
  assert.equal(saved.answers.get("NAME"), "sent");
  assert.equal(saved.language, "en");
  assert.equal(saved.pageId, "q:next");
  assert.equal(edited.language, "fr", "acknowledgement must not mutate the sent snapshot");
});

test("test_progress_should_keep_newer_answers_language_and_page_after_a_delayed_acknowledgement", () => {
  const saving = beginSave(editProgress(initialProgress(progress("")), progress("first")));
  const edited = editProgress(saving, { ...progress("newer", "en", "q:three"), groupId: "g3" });
  const confirmed = acknowledgeProgress(
    edited,
    saving.editVersion,
    progress("first", "fr", "q:two"),
  );
  assert.equal(confirmed.answers.get("NAME"), "newer");
  assert.equal(confirmed.language, "en");
  assert.equal(confirmed.groupId, "g3");
  assert.equal(confirmed.pageId, "q:three");
  assert.equal(hasUnsavedChanges(confirmed), true);
  const saved = acknowledgeProgress(beginSave(confirmed), confirmed.editVersion, confirmed);
  assert.equal(hasUnsavedChanges(saved), false);
});

test("test_progress_should_remain_dirty_after_a_rejected_save", () => {
  const saving = beginSave(editProgress(initialProgress(progress("")), progress("invalid")));
  assert.equal(hasUnsavedChanges(saving), true);
  assert.equal(saving.attemptedVersion, saving.editVersion);
  assert.equal(saving.savedVersion, 0);
});

test("test_progress_should_save_newer_edits_with_the_next_campaign_response_version", async () => {
  let complete: (value: Acknowledgement) => void = () => {
    throw new Error("No pending write");
  };
  const sent: SaveCommand[] = [];
  const writer = createResponseWriter(
    "response-1",
    0,
    async (command) => {
      sent.push(command);
      if (command.expectedVersion === 0)
        return new Promise((resolve) => {
          complete = resolve;
        });
      return acknowledge(command);
    },
    () => `operation-${sent.length}`,
  );
  const signal = new AbortController().signal;
  let local = beginSave(editProgress(initialProgress(progress("")), progress("first")));
  const sentVersion = local.editVersion;
  const pending = writer.save(local, false, signal);
  local = editProgress(local, progress("second", "en", "q:second"));
  complete(acknowledge(sent[0]!));
  local = acknowledgeProgress(local, sentVersion, await pending);
  assert.equal(local.answers.get("NAME"), "second");
  assert.equal(hasUnsavedChanges(local), true);
  const saved = await writer.save(local, false, signal);
  assert.equal(saved.acknowledgement.version, 2);
  assert.equal(sent[1]!.checkpoint.pageId, "q:second");
  assert.equal(sent[1]!.checkpoint.language, "en");
  assert.equal(hasUnsavedChanges(acknowledgeProgress(local, local.editVersion, saved)), false);
});
