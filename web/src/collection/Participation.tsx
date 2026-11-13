import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import { toViewer, type ValidationIssue } from "../authoring/model.ts";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";
import {
  getParticipation,
  resumeFragment,
  saveParticipation,
  type Participation as StoredParticipation,
  type SaveCommand,
} from "./api.ts";
import type { ViewerDraft } from "./types.ts";

export function Participation({ id, token }: { id: string; token: string }) {
  const [stored, setStored] = useState<StoredParticipation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [pending, setPending] = useState<SaveCommand | null>(null);
  const [busy, setBusy] = useState(false);
  const [reload, setReload] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const writing = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setStored(null);
    setPending(null);
    setError(null);
    setIssues([]);
    setBusy(false);
    writing.current = null;
    async function load(): Promise<void> {
      try {
        const result = await getParticipation(id, token, controller.signal);
        if (!controller.signal.aborted) setStored(result);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => {
      controller.abort();
      writing.current?.abort();
    };
  }, [id, token, reload]);
  const study = useMemo(() => {
    if (stored === null) return null;
    return toViewer(stored.definition, stored.run.studyId);
  }, [stored]);
  async function attempt(command: SaveCommand): Promise<void> {
    if (stored === null || busy || writing.current !== null) return;
    const controller = new AbortController();
    writing.current = controller;
    setBusy(true);
    setError(null);
    setIssues([]);
    try {
      const result = await saveParticipation(id, token, command, controller.signal);
      if (result.id !== id || result.version !== command.expectedVersion + 1)
        throw new Error("Accusé d’enregistrement incohérent. Rechargez la participation.");
      if (controller.signal.aborted) return;
      setStored({
        ...stored,
        answers: command.answers,
        version: result.version,
        status: result.status,
        checkpoint: result.checkpoint,
        updatedAt: result.updatedAt,
      });
      setPending(null);
      setNotice("Le dernier enregistrement a été confirmé par le serveur.");
      setIssues(result.warnings);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError) {
        setIssues(cause.issues);
        if (cause.status >= 400 && cause.status < 500 && cause.status !== 409) setPending(null);
      }
    } finally {
      if (writing.current === controller) writing.current = null;
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleSave(draft: ViewerDraft, submit: boolean): void {
    if (stored === null || pending !== null || busy || writing.current !== null) return;
    const command: SaveCommand = {
      expectedVersion: stored.version,
      operationId: crypto.randomUUID(),
      answers: Object.fromEntries(draft.answers),
      checkpoint: { language: draft.language, groupId: draft.groupId },
      submit,
    };
    setPending(command);
    setNotice(null);
    void attempt(command);
  }
  function handleRetry(): void {
    if (pending !== null) void attempt(pending);
  }
  function handleReload(): void {
    if (
      window.confirm(
        "Recharger remplacera les modifications non enregistrées par la réponse conservée sur le serveur.",
      )
    ) {
      writing.current?.abort();
      setReload((value) => value + 1);
    }
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let noticeContent = null;
  if (notice !== null)
    noticeContent = (
      <p className="notice" role="status">
        {notice}
      </p>
    );
  let retry = null;
  if (pending !== null)
    retry = (
      <button type="button" onClick={handleRetry} disabled={busy}>
        Réessayer le même enregistrement
      </button>
    );
  let stateNotice = null;
  if (stored?.status === "submitted")
    stateNotice = (
      <p className="notice">Vos réponses ont été envoyées. Elles ne peuvent plus être modifiées.</p>
    );
  else if (stored?.run.isOpen === false)
    stateNotice = (
      <p className="notice">
        La campagne est fermée. Vos réponses enregistrées restent consultables.
      </p>
    );
  let content = <p role="status">Chargement de votre participation…</p>;
  if (stored !== null && study !== null)
    content = (
      <ViewerBoundary key={`${id}:${stored.version}:${reload}`}>
        <StudyViewer
          study={study}
          persistence={{
            answers: new Map(Object.entries(stored.answers)),
            language: stored.checkpoint.language,
            groupId: stored.checkpoint.groupId,
            submitted: stored.status === "submitted",
            hasProgress: stored.version > 0 || !stored.run.isOpen,
            readOnly: stored.status === "submitted" || !stored.run.isOpen,
            disabled: busy || pending !== null,
            onSave: handleSave,
          }}
        />
      </ViewerBoundary>
    );
  const resume = `${window.location.origin}${window.location.pathname}${resumeFragment(id, token)}`;
  const issueItems = issues.map((issue, index) => (
    <li key={`${index}:${issue.path}`}>{issue.message}</li>
  ));
  let title = "Votre participation";
  if (stored !== null) title = stored.definition.title;
  return (
    <div>
      <section className="panel">
        <h1>{title}</h1>
        <label>
          Lien privé de reprise, à conserver
          <input type="url" readOnly value={resume} />
        </label>
        <p>
          Ce lien donne accès à vos réponses. Partagez uniquement le lien public de la campagne pour
          inviter d’autres personnes.
        </p>
        {stateNotice}
        {noticeContent}
        {errorContent}
        <ul>{issueItems}</ul>
        <div className="button-row">
          {retry}
          <button type="button" className="secondary" onClick={handleReload} disabled={busy}>
            Recharger la réponse enregistrée
          </button>
        </div>
      </section>
      {content}
    </div>
  );
}
