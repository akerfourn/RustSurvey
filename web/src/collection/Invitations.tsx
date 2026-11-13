import { useEffect, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import type { Run } from "./api.ts";
import { InvitationCreation } from "./InvitationCreation.tsx";
import { listInvitations, revokeInvitation, type Invitation } from "./invitations-api.ts";

interface InvitationsProps {
  run: Run;
  csrf: string;
  onExpired: () => void;
}

function InvitationEntry({
  item,
  busy,
  onRevoke,
}: {
  item: Invitation;
  busy: boolean;
  onRevoke: (id: string) => void;
}) {
  function handleRevoke(): void {
    onRevoke(item.id);
  }
  let maximum = "illimitées";
  if (item.maxUses !== null) maximum = String(item.maxUses);
  let action = (
    <button type="button" className="secondary" onClick={handleRevoke} disabled={busy}>
      Révoquer
    </button>
  );
  if (item.revoked) action = <span>Révoquée</span>;
  let validity = "Sans limite de dates";
  if (item.validFrom !== null || item.validUntil !== null) {
    let from = "maintenant";
    let until = "sans échéance";
    if (item.validFrom !== null) from = new Date(item.validFrom * 1000).toLocaleString();
    if (item.validUntil !== null) until = new Date(item.validUntil * 1000).toLocaleString();
    validity = `${from} → ${until}`;
  }
  return (
    <li>
      <code>{item.id}</code> · {item.uses} / {maximum} soumissions · {validity} {action}
    </li>
  );
}

export function Invitations({ run, csrf, onExpired }: InvitationsProps) {
  const [items, setItems] = useState<Invitation[]>([]);
  const [page, setPage] = useState(0);
  const [refresh, setRefresh] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    async function load(): Promise<void> {
      try {
        const result = await listInvitations(run.id, page * 50, controller.signal);
        if (!controller.signal.aborted) {
          setItems(result.items);
          setHasMore(result.hasMore);
        }
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      } finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [run.id, page, refresh]);
  async function revoke(id: string): Promise<void> {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await revokeInvitation(run.id, id, csrf);
      setRefresh((value) => value + 1);
    } catch (cause) {
      setError(message(cause));
      if (cause instanceof ApiError && cause.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  function handleRevoke(id: string): void {
    void revoke(id);
  }
  function handleRefresh(): void {
    setRefresh((value) => value + 1);
  }
  function handlePrevious(): void {
    setPage((value) => Math.max(0, value - 1));
  }
  function handleNext(): void {
    setPage((value) => value + 1);
  }
  const entries = items.map((item) => (
    <InvitationEntry key={item.id} item={item} busy={busy} onRevoke={handleRevoke} />
  ));
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  return (
    <section>
      <h3>Invitations privées</h3>
      <p>
        Un lien peut être partagé : la limite porte sur le jeton, sans vérifier l’identité de la
        personne.
      </p>
      <InvitationCreation run={run} csrf={csrf} onCreated={handleRefresh} onExpired={onExpired} />
      {errorContent}
      <ul>{entries}</ul>
      <div className="button-row">
        <button type="button" onClick={handlePrevious} disabled={page === 0 || loading}>
          Précédentes
        </button>
        <button type="button" onClick={handleNext} disabled={!hasMore || loading}>
          Suivantes
        </button>
        <button type="button" onClick={handleRefresh} disabled={loading}>
          Actualiser
        </button>
      </div>
    </section>
  );
}
