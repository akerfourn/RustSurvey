import { useEffect, useRef, useState } from "preact/hooks";
import { ApiError, message } from "./api.ts";
import { checkCollection, type CollectionCheck as Check } from "./collection-check-api.ts";
import { GroupSettings } from "./GroupSettings.tsx";
import { SourceContexts } from "./SourceContexts.tsx";

interface CollectionCheckProps {
  id: string;
  revision: number;
  onExpired: () => void;
  csrf: string | null;
}

export function CollectionCheck({ id, revision, onExpired, csrf }: CollectionCheckProps) {
  const [report, setReport] = useState<Check | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const active = useRef<AbortController | null>(null);
  const [contextId, setContextId] = useState<string | null>(null);
  useEffect(() => {
    setReport(null);
    setError(null);
    setBusy(false);
    return () => active.current?.abort();
  }, [id, revision]);

  async function verify(): Promise<void> {
    if (busy) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    try {
      const result = await checkCollection(id, revision, controller.signal, contextId);
      if (!controller.signal.aborted) setReport(result);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleVerify(): void {
    void verify();
  }
  function handleContext(id: string | null): void {
    active.current?.abort();
    setBusy(false);
    setReport(null);
    setError(null);
    setContextId(id);
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let reportContent = null;
  if (report !== null) reportContent = <CollectionCheckReport report={report} />;
  return (
    <details>
      <summary>Inventaire de compatibilité et de migration</summary>
      <p>
        Le bilan examine la révision enregistrée {revision}. Les modifications non enregistrées ne
        sont pas évaluées. L’ouverture d’une nouvelle campagne est vérifiée séparément dans
        Collecter des réponses.
      </p>
      <SourceContexts
        id={id}
        revision={revision}
        csrf={csrf}
        selected={contextId}
        disabled={busy}
        onSelect={handleContext}
        onExpired={onExpired}
      />
      <button type="button" onClick={handleVerify} disabled={busy}>
        Vérifier les prérequis de collecte
      </button>
      {errorContent}
      {reportContent}
    </details>
  );
}

function CollectionCheckReport({ report }: { report: Check }) {
  let runtime = "Les questions et règles dépassent le moteur de collecte natif actuel.";
  if (report.runtime === "supported")
    runtime =
      "Les questions et règles entrent dans le moteur natif actuel. Leur équivalence avec le parcours LimeSurvey reste à vérifier.";
  let mapping = "Les correspondances des cellules sont partielles.";
  if (report.mapping === "complete")
    mapping = `${report.mappedCells} cellules de réponse possèdent une correspondance source.`;
  const diagnostics = report.issues.map((issue, index) => (
    <li key={`${index}:${issue.path}`}>
      <code>{issue.path}</code> : {issue.message}
    </li>
  ));
  const omitted = report.issueCount - report.issues.length;
  let omittedContent = null;
  if (omitted > 0) omittedContent = <p>{omitted} autre(s) point(s) non affiché(s).</p>;
  return (
    <div>
      <p className="notice">
        La reprise des anciennes participations reste indisponible. Cet inventaire ne qualifie pas
        l’ouverture d’une nouvelle campagne, vérifiée séparément dans Collecter des réponses.
      </p>
      <p>
        {runtime} {mapping}
      </p>
      <p>
        {report.unprojectedSections} section(s) et {report.unprojectedColumns} champ(s) source
        restent à interpréter. {report.missingSettings} réglage(s) opérationnel(s) ne fournissent
        pas de valeur effective dans le LSS.
      </p>
      <GroupSettings settings={report.groupSettings} />
      <ul>{diagnostics}</ul>
      {omittedContent}
    </div>
  );
}
