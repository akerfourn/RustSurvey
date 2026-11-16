import { ApiError } from "../authoring/api.ts";
import type { Acknowledgement, SaveCommand } from "./api.ts";
import type { SavedDraft, ViewerDraft } from "./types.ts";

interface Pending {
  command: SaveCommand;
  signature: string;
}

export interface ResponseWriter {
  save: (draft: ViewerDraft, submit: boolean, signal: AbortSignal) => Promise<SavedDraft>;
  uncertain: () => boolean;
}

function compareFields([left]: [string, string], [right]: [string, string]): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

export function createResponseWriter(
  id: string,
  initialVersion: number,
  send: (command: SaveCommand, signal: AbortSignal) => Promise<Acknowledgement>,
  operationId: () => string = () => crypto.randomUUID(),
): ResponseWriter {
  let version = initialVersion;
  let pending: Pending | null = null;

  async function flush(signal: AbortSignal): Promise<SavedDraft> {
    if (pending === null) throw new Error("Aucun enregistrement en attente.");
    const command = pending.command;
    try {
      const acknowledgement = await send(command, signal);
      if (acknowledgement.id !== id || acknowledgement.version !== command.expectedVersion + 1)
        throw new Error("Accusé d’enregistrement incohérent. Rechargez la participation.");
      version = acknowledgement.version;
      pending = null;
      return {
        answers: new Map(Object.entries(command.answers)),
        ...acknowledgement.checkpoint,
        acknowledgement,
      };
    } catch (cause) {
      if (
        cause instanceof ApiError &&
        [400, 401, 403, 404, 409, 413, 415, 422].includes(cause.status)
      )
        pending = null;
      throw cause;
    }
  }

  async function save(
    draft: ViewerDraft,
    submit: boolean,
    signal: AbortSignal,
  ): Promise<SavedDraft> {
    const checkpoint: SaveCommand["checkpoint"] = {
      language: draft.language,
      groupId: draft.groupId,
    };
    if (draft.pageId !== undefined) checkpoint.pageId = draft.pageId;
    const values = {
      answers: Object.fromEntries([...draft.answers].sort(compareFields)),
      checkpoint,
      submit,
    };
    const signature = JSON.stringify(values);
    if (pending !== null) {
      const matching = pending.signature === signature;
      const completing = pending.command.submit;
      const previous = await flush(signal);
      if (matching || (completing && previous.acknowledgement.status === "submitted"))
        return previous;
    }
    pending = {
      command: { ...values, expectedVersion: version, operationId: operationId() },
      signature,
    };
    return flush(signal);
  }
  return { save, uncertain: () => pending !== null };
}
