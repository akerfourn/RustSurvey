import { useEffect, useRef, useState } from "preact/hooks";
import { message } from "../authoring/api.ts";
import {
  credentials,
  getRun,
  resumeFragment,
  startParticipation,
  type RunDefinition,
} from "./api.ts";

export function RunStart({ id }: { id: string }) {
  const [loaded, setLoaded] = useState<RunDefinition | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const identity = useRef<{ id: string; token: string } | null>(null);
  const writing = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setLoaded(null);
    setError(null);
    identity.current = null;
    async function load(): Promise<void> {
      try {
        const result = await getRun(id, controller.signal);
        if (!controller.signal.aborted) setLoaded(result);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => {
      controller.abort();
      writing.current?.abort();
    };
  }, [id]);
  async function begin(): Promise<void> {
    if (loaded === null || busy) return;
    const controller = new AbortController();
    writing.current = controller;
    setBusy(true);
    setError(null);
    try {
      identity.current ??= credentials();
      const participant = identity.current;
      await startParticipation(
        id,
        participant.id,
        participant.token,
        loaded.definition.language,
        controller.signal,
      );
      if (!controller.signal.aborted)
        window.location.hash = resumeFragment(participant.id, participant.token);
    } catch (cause) {
      if (!controller.signal.aborted) setError(message(cause));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleStart(): void {
    void begin();
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let content = <p role="status">Chargement de la campagne…</p>;
  if (loaded !== null) {
    content = (
      <p className="notice">
        Cette campagne est fermée. Aucune nouvelle participation ne peut commencer.
      </p>
    );
    if (loaded.run.isOpen)
      content = (
        <>
          <p>
            Vos réponses pourront être enregistrées pour reprendre plus tard, puis envoyées à la fin
            du questionnaire.
          </p>
          <p>
            Un lien privé de reprise sera affiché après le démarrage. Conservez-le pour retrouver
            cette participation sur un autre appareil.
          </p>
          <button type="button" onClick={handleStart} disabled={busy}>
            Commencer le questionnaire
          </button>
        </>
      );
  }
  let title = "Participation";
  if (loaded !== null) title = loaded.definition.title;
  return (
    <section className="panel">
      <h1>{title}</h1>
      {errorContent}
      {content}
    </section>
  );
}
