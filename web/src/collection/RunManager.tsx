import { useEffect, useState } from "preact/hooks";
import { ApiError, message } from "../authoring/api.ts";
import type { ValidationIssue } from "../authoring/model.ts";
import { createRun, listRuns, type ParticipationPolicy, type Run } from "./api.ts";
import { RunCard } from "./RunCard.tsx";
import { PolicyControl } from "./PolicyControl.tsx";

interface RunManagerProps {
  studyId: string;
  revision: number;
  csrf: string;
  disabled: boolean;
  onExpired: () => void;
}

export function RunManager({ studyId, revision, csrf, disabled, onExpired }: RunManagerProps) {
  const [runs, setRuns] = useState<Run[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [policy, setPolicy] = useState<ParticipationPolicy>({ mode: "open" });
  useEffect(() => {
    const controller = new AbortController();
    async function load(): Promise<void> {
      try {
        const result = await listRuns(studyId, controller.signal);
        if (!controller.signal.aborted) setRuns(result);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => controller.abort();
  }, [studyId]);
  async function create(): Promise<void> {
    if (busy || disabled) return;
    setBusy(true);
    setError(null);
    setIssues([]);
    try {
      const run = await createRun(studyId, revision, csrf, policy);
      setRuns((previous) => [run, ...previous].slice(0, 50));
    } catch (cause) {
      setError(message(cause));
      if (cause instanceof ApiError) {
        setIssues(cause.issues);
        if (cause.status === 401) onExpired();
      }
    } finally {
      setBusy(false);
    }
  }
  function handleCreate(): void {
    void create();
  }
  function handleChanged(run: Run): void {
    setRuns((previous) =>
      previous.map((item) => {
        if (item.id === run.id) return run;
        return item;
      }),
    );
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  const cards = runs.map((run) => (
    <RunCard key={run.id} run={run} csrf={csrf} onChange={handleChanged} onExpired={onExpired} />
  ));
  const issueItems = issues.map((issue, index) => (
    <li key={`${index}:${issue.path}`}>{issue.message}</li>
  ));
  return (
    <section className="panel">
      <h2>Collecter des réponses</h2>
      <p>
        Une campagne conserve la version enregistrée du questionnaire. Les modifications ultérieures
        de l’éditeur ne changent pas les participations en cours.
      </p>
      <p>
        Les 28 types natifs sans fichiers sont disponibles, avec tableaux, classements, filtres et
        calculs. Les médias, quotas et règles non pris en charge bloquent l’ouverture avec un
        diagnostic.
      </p>
      <PolicyControl disabled={disabled || busy} onChange={setPolicy} />
      <button type="button" onClick={handleCreate} disabled={disabled || busy}>
        Ouvrir une campagne sur la version enregistrée
      </button>
      {errorContent}
      <ul>{issueItems}</ul>
      {cards}
    </section>
  );
}
