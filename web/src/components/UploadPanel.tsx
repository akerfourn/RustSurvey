import type { TargetedEvent } from "preact";

interface UploadPanelProps {
  file: File | null;
  loading: boolean;
  error: string | null;
  onFile: (file: File | null) => void;
  onInspect: () => void;
  onExample: () => void;
  onControls: () => void;
}

export function UploadPanel({
  file,
  loading,
  error,
  onFile,
  onInspect,
  onExample,
  onControls,
}: UploadPanelProps) {
  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    onFile(event.currentTarget.files?.[0] ?? null);
  }
  let statusContent = null;
  if (loading)
    statusContent = (
      <p role="status" className="muted">
        Transfert et analyse du questionnaire…
      </p>
    );
  if (error !== null)
    statusContent = (
      <p role="alert" className="notice error">
        {error}
      </p>
    );
  let fileContent = "Fichier XML .lss · 8 Mio maximum";
  if (file !== null) fileContent = `${file.name} · ${Math.ceil(file.size / 1024)} Kio`;
  return (
    <section className="panel upload-panel" aria-labelledby="upload-title" aria-busy={loading}>
      <div>
        <p className="eyebrow">01 · Importer</p>
        <h2 id="upload-title">Ouvrir un questionnaire</h2>
        <p className="muted">
          Sélectionnez un export de structure LimeSurvey pour consulter l’étude et essayer la
          saisie.
        </p>
      </div>
      <div className="upload-controls">
        <label className="file-label" htmlFor="survey-file">
          Fichier de l’étude
          <input
            id="survey-file"
            type="file"
            accept=".lss,application/xml,text/xml"
            disabled={loading}
            onChange={handleFile}
          />
        </label>
        <p className="file-detail">{fileContent}</p>
        <div className="button-row">
          <button type="button" disabled={file === null || loading} onClick={onInspect}>
            Analyser le fichier
          </button>
          <button type="button" className="secondary" disabled={loading} onClick={onExample}>
            Essayer l’exemple
          </button>
          <button type="button" className="secondary" disabled={loading} onClick={onControls}>
            Essayer les 28 types
          </button>
        </div>
        <a className="small-link" href="/api/surveys/example.lss" download="rustsurvey-demo.lss">
          Télécharger le LSS d’exemple
        </a>
      </div>
      {statusContent}
    </section>
  );
}
