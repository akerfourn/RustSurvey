import { useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import { credentials, invitationFragment, type Run } from "./api.ts";
import { createInvitation, type InvitationCommand } from "./invitations-api.ts";
import { invitationLimits, type InvitationFields } from "./invitation-limits.ts";

interface InvitationCreationProps {
  run: Run;
  csrf: string;
  onCreated: () => void;
  onExpired: () => void;
}

export function InvitationCreation({ run, csrf, onCreated, onExpired }: InvitationCreationProps) {
  const [fields, setFields] = useState<InvitationFields>({
    maximum: "1",
    unlimited: false,
    validFrom: "",
    validUntil: "",
  });
  const [pending, setPending] = useState<InvitationCommand | null>(null);
  const [link, setLink] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const persistent =
    run.participationPolicy.mode === "invitation" && run.participationPolicy.persistence;
  const disabled = busy || pending !== null;

  function handleField(event: Event): void {
    if (!(event.currentTarget instanceof HTMLInputElement)) return;
    const input = event.currentTarget;
    if (input.name === "unlimited") setFields({ ...fields, unlimited: input.checked });
    else setFields({ ...fields, [input.name]: input.value });
  }

  async function create(): Promise<void> {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      let command = pending;
      if (command === null) {
        command = { ...credentials(), ...invitationLimits(fields, persistent) };
        setPending(command);
      }
      await createInvitation(run.id, command, csrf);
      setLink(
        `${window.location.origin}${window.location.pathname}${invitationFragment(run.id, command.token)}`,
      );
      setPending(null);
      onCreated();
    } catch (cause) {
      setError(message(cause));
      if (cause instanceof ApiError && cause.status >= 400 && cause.status < 500) {
        setPending(null);
        if (cause.status === 401) onExpired();
      }
    } finally {
      setBusy(false);
    }
  }
  function handleCreate(): void {
    void create();
  }
  let limits = (
    <p>
      Cette invitation retrouve une seule réponse ; ses corrections ne consomment pas d’utilisation
      supplémentaire.
    </p>
  );
  if (!persistent)
    limits = (
      <>
        <label>
          Nombre maximal de réponses envoyées avec ce lien
          <input
            type="number"
            name="maximum"
            min="1"
            max="100000"
            value={fields.maximum}
            onInput={handleField}
            disabled={disabled || fields.unlimited}
          />
        </label>
        <label className="checkbox-label">
          <input
            type="checkbox"
            name="unlimited"
            checked={fields.unlimited}
            onChange={handleField}
            disabled={disabled}
          />
          Autoriser un nombre illimité de réponses
        </label>
      </>
    );
  let linkContent = null;
  if (link !== null)
    linkContent = (
      <label>
        Nouveau lien privé d’invitation — à copier maintenant
        <input type="url" readOnly value={link} />
      </label>
    );
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let label = "Créer un lien d’invitation";
  if (pending !== null) label = "Réessayer la même création";
  return (
    <section>
      <h4>Nouvelle invitation</h4>
      <p>
        Le secret du lien n’est pas conservé en clair. Copiez-le avant de quitter cette page. Aucun
        courriel n’est envoyé.
      </p>
      {limits}
      <div className="form-grid">
        <label>
          Valide à partir du (heure locale, facultatif)
          <input
            type="datetime-local"
            name="validFrom"
            value={fields.validFrom}
            onInput={handleField}
            disabled={disabled}
          />
        </label>
        <label>
          Valide jusqu’au (heure locale, facultatif)
          <input
            type="datetime-local"
            name="validUntil"
            value={fields.validUntil}
            onInput={handleField}
            disabled={disabled}
          />
        </label>
      </div>
      <button type="button" onClick={handleCreate} disabled={busy || !run.isOpen}>
        {label}
      </button>
      {errorContent}
      {linkContent}
    </section>
  );
}
