import { useEffect, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import { listRows, type SourceRow } from "./api.ts";

export function ImportRows({ id, onExpired }: { id: string; onExpired: () => void }) {
  const [from, setFrom] = useState(0);
  const [items, setItems] = useState<SourceRow[]>([]);
  const [more, setMore] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setBusy(true);
    async function load(): Promise<void> {
      try {
        const result = await listRows(id, from, controller.signal);
        if (!controller.signal.aborted) {
          setItems(result.items);
          setMore(result.hasMore);
          setError(null);
        }
      } catch (cause) {
        if (!controller.signal.aborted) {
          setError(message(cause));
          if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403))
            onExpired();
        }
      } finally {
        if (!controller.signal.aborted) setBusy(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [id, from, onExpired]);
  function previous(): void {
    setFrom(Math.max(0, from - 20));
  }
  function next(): void {
    const last = items.at(-1);
    if (last !== undefined) setFrom(last.ordinal + 1);
  }
  let notice = null;
  if (error !== null)
    notice = (
      <p role="alert" className="notice error">
        {error}
      </p>
    );
  const rows = items.map((item) => {
    let label = "État inconnu";
    if (item.status === "submitted") label = "Soumise";
    if (item.status === "draft") label = "Partielle";
    let source = "Identifiant source inconnu";
    if (item.sourceId !== null) source = `Identifiant source ${item.sourceId}`;
    return (
      <li key={item.ordinal}>
        {item.ordinal + 1} · {source} · {label} ·{" "}
        <a href={`/api/admin/response-imports/${id}/records/${item.ordinal}/record.json`}>
          Valeurs de la ligne (JSON)
        </a>
        {" · "}
        <a href={`/api/admin/response-imports/${id}/records/${item.ordinal}/native-cells.json`}>
          Examiner la conversion (JSON)
        </a>
      </li>
    );
  });
  return (
    <section>
      <h4>Lignes conservées</h4>
      {notice}
      <ul>{rows}</ul>
      <div className="button-row">
        <button type="button" onClick={previous} disabled={busy || from === 0}>
          Précédentes
        </button>
        <button type="button" onClick={next} disabled={busy || !more}>
          Suivantes
        </button>
      </div>
    </section>
  );
}
