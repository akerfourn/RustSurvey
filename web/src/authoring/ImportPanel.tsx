import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, importLss, importDefinition, message } from "./api.ts";
interface Props {
  csrf: string;
  onExpired: () => void;
  format?: "lss" | "native";
}
export function ImportPanel({ csrf, onExpired, format = "lss" }: Props) {
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => controller.current?.abort(), []);
  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    setFile(event.currentTarget.files?.[0] ?? null);
    setError(null);
  }
  async function handleImport(): Promise<void> {
    if (file === null || busy) return;
    const request = new AbortController();
    controller.current = request;
    setBusy(true);
    setError(null);
    try {
      let study;
      if (format === "native") study = await importDefinition(file, csrf, request.signal);
      else study = await importLss(file, csrf, request.signal);
      if (!request.signal.aborted) window.location.hash = `/admin/edit/${study.id}`;
    } catch (cause) {
      if (request.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
    } finally {
      if (!request.signal.aborted) setBusy(false);
    }
  }
  let notice = null;
  if (error !== null)
    notice = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let label = "Importer et enregistrer le LSS";
  let title = "Importer une étude LimeSurvey";
  let description =
    "Schéma 623, fichier de 8 Mio maximum. L’original est conservé en SQL. Les réponses restent locales.";
  let fileLabel = "Fichier LSS";
  let accept = ".lss,application/xml,text/xml";
  if (format === "native") {
    title = "Importer une définition native";
    description =
      "JSON de définition, 8 Mio maximum, importé comme nouveau questionnaire. Les médias et l’archive LSS privée ne font pas partie de ce fichier.";
    fileLabel = "Fichier JSON";
    accept = ".json,application/json";
    label = "Importer la définition";
  }
  if (busy) label = "Import et enregistrement…";
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p>{description}</p>
      <label>
        {fileLabel}
        <input type="file" accept={accept} disabled={busy} onChange={handleFile} />
      </label>
      {notice}
      <button type="button" onClick={handleImport} disabled={file === null || busy}>
        {label}
      </button>
    </section>
  );
}
