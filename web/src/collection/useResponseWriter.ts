import { useLayoutEffect, useRef, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import { saveParticipation, type Participation } from "./api.ts";
import { createResponseWriter } from "./writer.ts";
import type { SavedDraft, ViewerDraft } from "./types.ts";
import type { ValidationIssue } from "../authoring/model.ts";

export function useResponseWriter(initial: Participation, token: string, invitationToken?: string) {
  const [saved, setSaved] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [backgroundSaving, setBackgroundSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [blocked, setBlocked] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const controller = useRef<AbortController | null>(null);
  const inFlight = useRef(false);
  const [writer] = useState(() =>
    createResponseWriter(initial.id, initial.version, (command, signal) =>
      saveParticipation(initial.id, token, command, signal, invitationToken),
    ),
  );
  useLayoutEffect(() => () => controller.current?.abort(), []);

  async function save(
    draft: ViewerDraft,
    submit: boolean,
    background: boolean,
  ): Promise<SavedDraft | null> {
    if (inFlight.current || blocked) return null;
    const request = new AbortController();
    controller.current = request;
    inFlight.current = true;
    setBusy(true);
    setBackgroundSaving(background);
    setError(null);
    setIssues([]);
    try {
      const result = await writer.save(draft, submit, request.signal);
      if (request.signal.aborted) return null;
      const acknowledgement = result.acknowledgement;
      setSaved((previous) => ({
        ...previous,
        answers: Object.fromEntries(result.answers),
        version: acknowledgement.version,
        status: acknowledgement.status,
        checkpoint: acknowledgement.checkpoint,
        updatedAt: acknowledgement.updatedAt,
      }));
      setIssues(acknowledgement.warnings);
      return result;
    } catch (cause) {
      if (request.signal.aborted) return null;
      setError(message(cause));
      if (cause instanceof ApiError) {
        setIssues(cause.issues);
        if ([401, 403, 404, 409].includes(cause.status)) setBlocked(true);
      }
      return null;
    } finally {
      inFlight.current = false;
      if (!request.signal.aborted) {
        setBusy(false);
        setBackgroundSaving(false);
        setUncertain(writer.uncertain());
      }
    }
  }
  return { saved, busy, backgroundSaving, error, issues, blocked, uncertain, save };
}
