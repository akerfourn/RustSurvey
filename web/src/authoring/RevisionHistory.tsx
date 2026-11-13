import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import {
  getRevision,
  ApiError,
  listRevisions,
  message,
  restoreRevision,
  type DefinitionRevision,
  type RevisionSummary,
} from "./api.ts";
import { toViewer, type StoredStudy } from "./model.ts";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";
import type { InspectionReport } from "../survey/types.ts";

interface Props {
  id: string;
  revision: number;
  csrf: string | null;
  disabled: boolean;
  onRestore: (study: StoredStudy) => void;
  onBusy: (busy: boolean) => void;
  report: InspectionReport | undefined;
  onExpired: () => void;
}

function HistoryBody({
  id,
  revision,
  csrf,
  disabled,
  onRestore,
  onBusy,
  report,
  onExpired,
}: Props) {
  function reportError(cause: unknown): void {
    setError(message(cause));
    if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
  }
  const [before, setBefore] = useState(1_000_000_000);
  const [items, setItems] = useState<RevisionSummary[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState<DefinitionRevision | null>(null);
  const request = useRef<AbortController | null>(null);
  useEffect(
    () => () => {
      request.current?.abort();
      onBusy(false);
    },
    [onBusy],
  );
  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    async function load(): Promise<void> {
      try {
        const page = await listRevisions(id, before, controller.signal);
        if (!controller.signal.aborted) {
          setItems(page.items);
          setHasMore(page.hasMore);
        }
      } catch (cause) {
        if (!controller.signal.aborted) reportError(cause);
      }
    }
    void load();
    return () => controller.abort();
  }, [id, revision, before, csrf, onExpired]);
  function handleMore(): void {
    const last = items.at(-1);
    if (last !== undefined) setBefore(last.revision);
  }
  function handleLatest(): void {
    setBefore(1_000_000_000);
  }
  async function inspect(number: number): Promise<void> {
    const controller = new AbortController();
    request.current?.abort();
    request.current = controller;
    setBusy(true);
    setError(null);
    try {
      const saved = await getRevision(id, number, controller.signal);
      if (!controller.signal.aborted) setSelected(saved);
    } catch (cause) {
      if (!controller.signal.aborted) reportError(cause);
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  async function restore(number: number): Promise<void> {
    if (csrf === null || disabled) return;
    const controller = new AbortController();
    request.current?.abort();
    request.current = controller;
    setBusy(true);
    onBusy(true);
    setError(null);
    try {
      const saved = await restoreRevision(id, number, revision, csrf, controller.signal);
      if (!controller.signal.aborted) {
        setSelected(null);
        setBefore(1_000_000_000);
        onRestore(saved);
      }
    } catch (cause) {
      if (!controller.signal.aborted) reportError(cause);
    } finally {
      if (!controller.signal.aborted) {
        setBusy(false);
        onBusy(false);
      }
    }
  }
  const rows = items.map((item) => {
    function handleInspect(): void {
      void inspect(item.revision);
    }
    function handleRestore(): void {
      void restore(item.revision);
    }
    return (
      <li key={item.revision}>
        <span>
          Révision {item.revision} · {new Date(item.createdAt * 1000).toLocaleString()}
        </span>
        <div className="button-row">
          <button type="button" className="text-button" onClick={handleInspect} disabled={busy}>
            Aperçu
          </button>
          <a
            href={`/api/admin/studies/${encodeURIComponent(id)}/revisions/${item.revision}/definition.json`}
          >
            Définition JSON
          </a>
          <button
            type="button"
            className="secondary"
            onClick={handleRestore}
            disabled={disabled || busy || item.revision === revision}
          >
            Restaurer
          </button>
        </div>
      </li>
    );
  });
  let notice = null;
  let preview = null;
  if (error !== null)
    notice = (
      <p role="alert" className="notice error">
        {error}
      </p>
    );
  if (selected !== null)
    preview = (
      <section>
        <h3>Aperçu de la révision {selected.revision}</h3>
        <ViewerBoundary key={`history-${selected.revision}`}>
          <StudyViewer study={toViewer(selected.definition, id)} report={report} debug={true} />
        </ViewerBoundary>
      </section>
    );
  return (
    <div>
      <p>
        Restaurer crée une nouvelle révision. Les versions précédentes restent disponibles.
        Enregistrez les modifications en cours avant de restaurer.
      </p>
      <p className="muted">
        Le JSON contient la définition native ; il ne remplace pas l’archive LSS originale et ses
        données non interprétées.
      </p>
      {notice}
      <ul className="revision-list">{rows}</ul>
      <div className="button-row">
        <button
          type="button"
          className="text-button"
          onClick={handleLatest}
          disabled={before === 1_000_000_000}
        >
          Plus récentes
        </button>
        <button type="button" className="text-button" onClick={handleMore} disabled={!hasMore}>
          Plus anciennes
        </button>
      </div>
      {preview}
    </div>
  );
}

export function RevisionHistory(props: Props) {
  const [open, setOpen] = useState(false);
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  let content = null;
  if (open) content = <HistoryBody {...props} />;
  return (
    <details className="panel" onToggle={handleToggle}>
      <summary>Historique des révisions</summary>
      {content}
    </details>
  );
}
