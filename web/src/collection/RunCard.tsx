import { useEffect, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import { closeRun, listResponses, type ResponseSummary, type Run } from "./api.ts";
import { Invitations } from "./Invitations.tsx";
import { policyLabel } from "./PolicyControl.tsx";

interface RunCardProps {
  run: Run;
  csrf: string;
  onChange: (run: Run) => void;
  onExpired: () => void;
}

export function RunCard({ run, csrf, onChange, onExpired }: RunCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [page, setPage] = useState(0);
  const [refresh, setRefresh] = useState(0);
  const [items, setItems] = useState<ResponseSummary[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (!expanded) return;
    const controller = new AbortController();
    setLoading(true);
    async function load(): Promise<void> {
      try {
        const result = await listResponses(run.id, page * 20, controller.signal);
        if (!controller.signal.aborted) {
          setItems(result.items);
          setHasMore(result.hasMore);
          setError(null);
        }
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      } finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [run.id, expanded, page, refresh]);
  async function close(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      onChange(await closeRun(run.id, csrf));
    } catch (cause) {
      setError(message(cause));
      if (cause instanceof ApiError && cause.status === 401) onExpired();
    } finally {
      setBusy(false);
    }
  }
  function handleClose(): void {
    void close();
  }
  function handleToggle(): void {
    setExpanded(!expanded);
  }
  function handlePrevious(): void {
    setPage((value) => Math.max(0, value - 1));
  }
  function handleNext(): void {
    setPage((value) => value + 1);
  }
  function handleRefresh(): void {
    setRefresh((value) => value + 1);
  }
  let status = "Fermée";
  let closeButton = null;
  if (run.isOpen) {
    status = "Ouverte";
    closeButton = (
      <button type="button" className="secondary" onClick={handleClose} disabled={busy}>
        Fermer la campagne
      </button>
    );
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  const entries = items.map((item) => {
    let label = "Brouillon";
    if (item.status === "submitted") label = "Réponse envoyée";
    return (
      <li key={item.id}>
        {label} · {new Date(item.updatedAt * 1000).toLocaleString()} ·{" "}
        <a href={`/api/admin/runs/${run.id}/responses/${item.id}/response.json`}>
          Télécharger les réponses JSON
        </a>
      </li>
    );
  });
  let responses = null;
  let invitations = null;
  let participationLink = "Ouvrir le lien public de participation";
  if (run.participationPolicy.mode === "invitation") {
    participationLink = "Ouvrir l’accueil de participation (invitation requise)";
    if (expanded) invitations = <Invitations run={run} csrf={csrf} onExpired={onExpired} />;
  }
  if (expanded)
    responses = (
      <div>
        <p>Les téléchargements restent réservés à l’administration.</p>
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
      </div>
    );
  return (
    <article className="notice">
      <h3>
        {status} · version {run.revision} · {new Date(run.createdAt * 1000).toLocaleString()}
      </h3>
      <p>{policyLabel(run.participationPolicy)}</p>
      <p>
        <a href={`#/runs/${run.id}`} target="_blank" rel="noreferrer">
          {participationLink}
        </a>
      </p>
      <div className="button-row">
        {closeButton}
        <button type="button" className="secondary" onClick={handleToggle}>
          Consulter les participations
        </button>
      </div>
      {errorContent}
      {responses}
      {invitations}
    </article>
  );
}
