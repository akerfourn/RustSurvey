import { useEffect, useRef, useState } from "preact/hooks";
import { message } from "../authoring/api.ts";
import { credentials, getRun, resumeFragment, startParticipation, type RunEntry } from "./api.ts";

export function RunStart({
  id,
  invitationToken,
}: {
  id: string;
  invitationToken?: string | undefined;
}) {
  const [loaded, setLoaded] = useState<RunEntry | null>(null);
  const [invitation, setInvitation] = useState(invitationToken ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const identity = useRef<{ id: string; token: string } | null>(null);
  const writing = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setLoaded(null);
    setError(null);
    setBusy(false);
    writing.current = null;
    identity.current = null;
    setInvitation(invitationToken ?? "");
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
  }, [id, invitationToken]);
  async function begin(): Promise<void> {
    if (loaded === null || busy) return;
    const controller = new AbortController();
    writing.current = controller;
    setBusy(true);
    setError(null);
    try {
      identity.current ??= credentials();
      const participant = identity.current;
      const policy = loaded.run.participationPolicy;
      let grant: string | undefined;
      if (policy.mode === "invitation") {
        grant = invitation.trim();
        if (!/^[a-f0-9]{64}$/.test(grant)) throw new Error("Saisissez le code d’invitation reçu.");
        if (policy.persistence) participant.token = grant;
      }
      const started = await startParticipation(
        id,
        participant.id,
        participant.token,
        loaded.definition.language,
        controller.signal,
        grant,
      );
      if (policy.mode === "invitation" && policy.persistence) grant = undefined;
      if (!controller.signal.aborted)
        window.location.hash = resumeFragment(started.id, participant.token, grant);
    } catch (cause) {
      if (!controller.signal.aborted) setError(message(cause));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleStart(): void {
    void begin();
  }
  function handleInvitation(event: Event): void {
    if (event.currentTarget instanceof HTMLInputElement) {
      setInvitation(event.currentTarget.value);
      identity.current = null;
    }
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
    let invitationControl = null;
    if (loaded.run.participationPolicy.mode === "invitation")
      invitationControl = (
        <label>
          Code d’invitation
          <input
            type="password"
            autoComplete="off"
            value={invitation}
            onInput={handleInvitation}
            disabled={busy}
          />
        </label>
      );
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
          {invitationControl}
          <p>
            Un lien privé de reprise sera affiché après le démarrage. Conservez-le pour retrouver
            cette participation sur un autre appareil.
          </p>
          <button type="button" onClick={handleStart} disabled={busy}>
            Commencer le questionnaire
          </button>
        </>
      );
    if (loaded.browserCompleted)
      content = (
        <p className="notice">
          Ce navigateur a déjà envoyé une réponse à cette campagne. Votre lien privé permet de
          consulter la réponse existante.
        </p>
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
