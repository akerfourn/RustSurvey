import { useEffect, useMemo, useState } from "preact/hooks";
import { getStudy, message } from "./api.ts";
import { toViewer, type StoredStudy } from "./model.ts";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";

export function PublicStudy({ id }: { id: string }) {
  const [stored, setStored] = useState<StoredStudy | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setStored(null);
    setError(null);
    async function load(): Promise<void> {
      try {
        const study = await getStudy(id, controller.signal);
        if (!controller.signal.aborted) setStored(study);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => controller.abort();
  }, [id]);
  const study = useMemo(
    () => (stored === null ? null : toViewer(stored.definition, stored.id, stored.source !== null)),
    [stored],
  );
  let content = (
    <p className="panel" role="status">
      Chargement du questionnaire…
    </p>
  );
  if (error !== null)
    content = (
      <p className="panel notice error" role="alert">
        {error}
      </p>
    );
  else if (study !== null && stored !== null)
    content = (
      <ViewerBoundary key={`${stored.id}:${stored.revision}`}>
        <section className="panel">
          <h1>{stored.definition.title}</h1>
        </section>
        <StudyViewer study={study} report={stored.source?.report} />
      </ViewerBoundary>
    );
  return (
    <div>
      <a className="back-link" href="#/">
        Retour aux questionnaires
      </a>
      {content}
    </div>
  );
}
