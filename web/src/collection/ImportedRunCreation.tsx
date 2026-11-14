import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, message } from "../authoring/api.ts";
import { SourceContexts } from "../authoring/SourceContexts.tsx";
import type { ValidationIssue } from "../authoring/model.ts";
import { createImportedRun, type Run } from "./api.ts";
import { readRuntimeFacts } from "./source-runtime.ts";

interface Props {
  studyId: string;
  revision: number;
  csrf: string;
  disabled: boolean;
  onCreated: (run: Run) => void;
  onExpired: () => void;
}

export function ImportedRunCreation({
  studyId,
  revision,
  csrf,
  disabled,
  onCreated,
  onExpired,
}: Props) {
  const [contextId, setContextId] = useState<string | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);

  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    setFile(event.currentTarget.files?.[0] ?? null);
    setError(null);
    setIssues([]);
  }
  async function create(): Promise<void> {
    if (busy || disabled || contextId === null || file === null) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    setIssues([]);
    try {
      const facts = await readRuntimeFacts(file);
      if (controller.signal.aborted) return;
      const run = await createImportedRun(
        studyId,
        revision,
        csrf,
        contextId,
        facts,
        controller.signal,
      );
      if (!controller.signal.aborted) onCreated(run);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError) {
        setIssues(cause.issues);
        if (cause.status === 401 || cause.status === 403) onExpired();
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleCreate(): void {
    void create();
  }
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  const diagnostics = issues.map((issue, index) => (
    <li key={`${index}:${issue.path}`}>
      <code>{issue.path}</code> : {issue.message}
    </li>
  ));
  return (
    <div>
      <p>
        La présentation et les accès seront repris des réglages source pris en charge. Les fonctions
        incompatibles empêchent l’ouverture et sont signalées.
      </p>
      <p>
        Cette action ouvre une nouvelle campagne. Les anciennes réponses et leurs liens de reprise
        ne sont pas transférés.
      </p>
      <SourceContexts
        id={studyId}
        revision={revision}
        csrf={csrf}
        selected={contextId}
        onSelect={setContextId}
        disabled={disabled || busy}
        onExpired={onExpired}
        purpose="collection"
      />
      <fieldset disabled={disabled || busy}>
        <legend>Paramètres d’exécution source</legend>
        <p>
          Choisir le fichier produit par l’outil d’extraction LimeSurvey décrit dans la
          documentation.
        </p>
        <label>
          Fichier des paramètres d’exécution · 64 Kio maximum
          <input type="file" accept=".json,application/json" onChange={handleFile} />
        </label>
        <button type="button" onClick={handleCreate} disabled={contextId === null || file === null}>
          Ouvrir une nouvelle campagne sur la version enregistrée
        </button>
      </fieldset>
      {errorContent}
      <ul>{diagnostics}</ul>
    </div>
  );
}
