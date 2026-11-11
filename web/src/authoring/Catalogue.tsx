import { useEffect, useState } from "preact/hooks";
import { listStudies, message } from "./api.ts";
import type { StudySummary } from "./model.ts";

interface CatalogueProps {
  administrator?: boolean;
}
export function Catalogue({ administrator = false }: CatalogueProps) {
  const [items, setItems] = useState<StudySummary[]>([]);
  const [offset, setOffset] = useState(0);
  const [more, setMore] = useState(false);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setBusy(true);
    setError(null);
    async function load(): Promise<void> {
      try {
        const page = await listStudies(offset, controller.signal);
        if (!controller.signal.aborted) {
          setItems(page.items);
          setMore(page.hasMore);
        }
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      } finally {
        if (!controller.signal.aborted) setBusy(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [offset]);
  function handlePrevious(): void {
    setOffset(Math.max(0, offset - 50));
  }
  function handleNext(): void {
    setOffset(offset + 50);
  }
  let title = "Questionnaires publics";
  let create = null;
  if (administrator) {
    title = "Mes questionnaires";
    create = (
      <a className="button-link" href="#/admin/new">
        Créer un questionnaire
      </a>
    );
  }
  let content = <p>Aucun questionnaire enregistré pour le moment.</p>;
  if (busy) content = <p role="status">Chargement des questionnaires…</p>;
  else if (error !== null)
    content = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  else if (items.length > 0)
    content = (
      <ul className="study-list">
        {items.map((study) => {
          let edit = null;
          if (administrator)
            edit = (
              <a className="button-link secondary" href={`#/admin/edit/${study.id}`}>
                Modifier
              </a>
            );
          return (
            <li key={study.id}>
              <div>
                <h2>{study.title}</h2>
                <p className="muted">Version {study.revision}</p>
              </div>
              <div className="button-row">
                <a className="button-link secondary" href={`#/studies/${study.id}`}>
                  Ouvrir
                </a>
                {edit}
              </div>
            </li>
          );
        })}
      </ul>
    );
  return (
    <section className="panel">
      <div className="section-heading">
        <h1>{title}</h1>
        {create}
      </div>
      {content}
      <nav className="button-row" aria-label="Pages de questionnaires">
        <button
          type="button"
          className="secondary"
          onClick={handlePrevious}
          disabled={offset === 0 || busy}
        >
          Précédent
        </button>
        <button type="button" className="secondary" onClick={handleNext} disabled={!more || busy}>
          Suivant
        </button>
      </nav>
    </section>
  );
}
