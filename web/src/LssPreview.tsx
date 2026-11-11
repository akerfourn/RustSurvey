import { useEffect, useRef, useState } from "preact/hooks";
import { inspectFile, loadExample } from "./api/inspection.ts";
import { InspectionView } from "./components/InspectionView.tsx";
import { UploadPanel } from "./components/UploadPanel.tsx";
import { ViewerBoundary } from "./components/ViewerBoundary.tsx";
import type { Inspection } from "./survey/types.ts";

interface ImportedStudy {
  inspection: Inspection;
  filename: string;
  id: number;
}

export function LssPreview() {
  const [file, setFile] = useState<File | null>(null);
  const [imported, setImported] = useState<ImportedStudy | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef<AbortController | null>(null);
  const sequence = useRef(0);
  useEffect(() => () => request.current?.abort(), []);

  async function analyze(example: "example" | "controls" | null): Promise<void> {
    request.current?.abort();
    const controller = new AbortController();
    request.current = controller;
    setLoading(true);
    setError(null);
    try {
      let selected = file;
      if (example !== null) selected = await loadExample(controller.signal, example);
      if (selected === null) throw new Error("Sélectionnez un fichier LSS.");
      const inspection = await inspectFile(selected, controller.signal);
      if (controller.signal.aborted) return;
      sequence.current += 1;
      setImported({ inspection, filename: selected.name, id: sequence.current });
    } catch (cause) {
      if (controller.signal.aborted) return;
      let message = "Le backend est injoignable. Vérifiez qu’il est démarré puis réessayez.";
      if (cause instanceof Error && !(cause instanceof TypeError)) message = cause.message;
      setError(message);
    } finally {
      if (!controller.signal.aborted) setLoading(false);
    }
  }
  function handleFile(selected: File | null): void {
    setFile(selected);
    setError(null);
  }
  function handleInspect(): void {
    void analyze(null);
  }
  function handleExample(): void {
    void analyze("example");
  }
  function handleControls(): void {
    void analyze("controls");
  }
  let resultContent = (
    <section className="empty-state">
      <span className="empty-icon" aria-hidden="true">
        ↳
      </span>
      <h2>Votre étude, directement dans le navigateur</h2>
      <p>
        Importez un LSS ou ouvrez l’exemple pour parcourir les groupes, répondre aux questions et
        essayer les conditions.
      </p>
    </section>
  );
  if (imported !== null)
    resultContent = (
      <ViewerBoundary key={imported.id}>
        <InspectionView inspection={imported.inspection} filename={imported.filename} />
      </ViewerBoundary>
    );
  return (
    <>
      <header className="site-header">
        <a className="brand" href="#/">
          RustSurvey<span className="version-tag">Aperçu local</span>
        </a>
        <span className="target-label">LimeSurvey 6.5.14 · Schéma 623</span>
      </header>
      <main>
        <div className="hero">
          <p className="eyebrow">Du fichier au questionnaire</p>
          <h1>
            Explorez votre étude.
            <br />
            Essayez son parcours.
          </h1>
          <p>
            Un import LSS, un viewer interactif et des réponses locales. Le serveur analyse la
            structure ; le navigateur fait tourner l’aperçu.
          </p>
        </div>
        <UploadPanel
          file={file}
          loading={loading}
          error={error}
          onFile={handleFile}
          onInspect={handleInspect}
          onExample={handleExample}
          onControls={handleControls}
        />
        {resultContent}
      </main>
      <footer>
        RustSurvey · Prototype de moteur de questionnaires ·{" "}
        <a href="/api/health">État du backend</a>
      </footer>
    </>
  );
}
