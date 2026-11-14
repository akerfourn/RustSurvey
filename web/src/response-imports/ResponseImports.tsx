import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, message } from "../authoring/api.ts";
import {
  complete,
  getImport,
  listImports,
  upload,
  type ImportDetails,
  type ImportSummary,
} from "./api.ts";
import { ImportReport } from "./ImportReport.tsx";

interface Props {
  id: string;
  revision: number;
  csrf: string | null;
  onExpired: () => void;
}

export function ResponseImports({ id, revision, csrf, onExpired }: Props) {
  const [items, setItems] = useState<ImportSummary[]>([]);
  const [file, setFile] = useState<File | null>(null);
  const [selected, setSelected] = useState<ImportDetails | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => controller.current?.abort(), []);
  useEffect(() => {
    const request = new AbortController();
    async function load(): Promise<void> {
      try {
        const values = await listImports(id, request.signal);
        if (!request.signal.aborted) setItems(values);
      } catch (cause) {
        if (!request.signal.aborted) {
          setError(message(cause));
          if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403))
            onExpired();
        }
      }
    }
    void load();
    return () => request.abort();
  }, [id, refresh, onExpired]);
  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    setFile(event.currentTarget.files?.[0] ?? null);
    setError(null);
  }
  async function run(
    action: (signal: AbortSignal) => Promise<ImportDetails>,
    resume: boolean,
  ): Promise<void> {
    if (busy) return;
    const request = new AbortController();
    controller.current?.abort();
    controller.current = request;
    setBusy(true);
    setError(null);
    try {
      const batch = await action(request.signal);
      if (request.signal.aborted) return;
      setSelected(batch);
      if (resume && csrf !== null)
        await complete(batch, csrf, request.signal, (next) => {
          if (!request.signal.aborted) setSelected(next);
        });
    } catch (cause) {
      if (!request.signal.aborted) {
        setError(message(cause));
        if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403))
          onExpired();
      }
    } finally {
      if (!request.signal.aborted) {
        setBusy(false);
        setRefresh((value) => value + 1);
      }
    }
  }
  function handleUpload(): void {
    if (file === null || csrf === null) return;
    const token = csrf;
    const input = file;
    void run((signal) => upload(id, revision, input, token, signal), true);
  }
  function handleRefresh(): void {
    setRefresh((value) => value + 1);
  }
  const entries = items.map((item) => {
    function inspect(): void {
      void run((signal) => getImport(item.id, signal), false);
    }
    function resume(): void {
      void run((signal) => getImport(item.id, signal), true);
    }
    let resumeButton = null;
    if (item.state !== "prepared")
      resumeButton = (
        <button type="button" onClick={resume} disabled={busy || csrf === null}>
          Reprendre la préparation
        </button>
      );
    return (
      <li key={item.id}>
        Révision {item.revision} · {item.loadedRows} / {item.rowCount} lignes ·{" "}
        {new Date(item.createdAt * 1000).toLocaleString()}
        <div className="button-row">
          <button type="button" onClick={inspect} disabled={busy}>
            Voir le bilan
          </button>
          {resumeButton}
        </div>
      </li>
    );
  });
  let notice = null;
  if (error !== null)
    notice = (
      <p role="alert" className="notice error">
        {error}
      </p>
    );
  let report = null;
  if (selected !== null) report = <ImportReport batch={selected} onExpired={onExpired} />;
  let uploadLabel = "Importer les réponses";
  if (busy) uploadLabel = "Préparation…";
  return (
    <details className="panel">
      <summary>Importer des réponses LimeSurvey</summary>
      <p>
        Les réponses sont conservées à part pour préparer leur migration. Cet import n'ouvre pas de
        collecte et ne remplace aucune réponse existante.
      </p>
      <p>
        Fichier LSR du schéma 623, 8 Mio maximum. La préparation utilise la révision enregistrée{" "}
        {revision}.
      </p>
      <label>
        Fichier de réponses (.lsr)
        <input
          type="file"
          accept=".lsr,.xml,application/xml,text/xml"
          onChange={handleFile}
          disabled={busy}
        />
      </label>
      <button
        type="button"
        onClick={handleUpload}
        disabled={busy || csrf === null || file === null}
      >
        {uploadLabel}
      </button>
      {notice}
      {report}
      <h3>Préparations récentes</h3>
      <ul>{entries}</ul>
      <button type="button" onClick={handleRefresh} disabled={busy}>
        Actualiser
      </button>
    </details>
  );
}
