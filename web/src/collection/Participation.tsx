import { useEffect, useState } from "preact/hooks";
import { message } from "../authoring/api.ts";
import { getParticipation, type Participation as StoredParticipation } from "./api.ts";
import { ParticipationForm } from "./ParticipationForm.tsx";

export function Participation({
  id,
  token,
  invitationToken,
}: {
  id: string;
  token: string;
  invitationToken?: string | undefined;
}) {
  const [stored, setStored] = useState<StoredParticipation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setStored(null);
    setError(null);
    async function load(): Promise<void> {
      try {
        const result = await getParticipation(id, token, controller.signal);
        if (!controller.signal.aborted) setStored(result);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => controller.abort();
  }, [id, token, invitationToken, reload]);
  function handleReload(): void {
    setReload((value) => value + 1);
  }
  if (stored !== null)
    return (
      <ParticipationForm
        key={`${stored.id}:${reload}`}
        initial={stored}
        token={token}
        invitationToken={invitationToken}
        onReload={handleReload}
      />
    );
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <section className="notice error" role="alert">
        <p>{error}</p>
        <button type="button" onClick={handleReload}>
          Réessayer
        </button>
      </section>
    );
  return (
    <section className="panel">
      <p role="status">Chargement de votre participation…</p>
      {errorContent}
    </section>
  );
}
