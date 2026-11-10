import { localized, plainText } from "../survey/text.ts";
import type { Inspection } from "../survey/types.ts";
import { StudyViewer } from "./StudyViewer.tsx";

interface InspectionViewProps {
  inspection: Inspection;
  filename: string;
}

export function InspectionView({ inspection, filename }: InspectionViewProps) {
  const { metadata, study, report } = inspection;
  const title =
    plainText(
      localized(metadata.titles, metadata.baseLanguage ?? "fr", metadata.baseLanguage)?.value,
    ) || "Étude sans titre";
  let viewerContent = (
    <p className="notice warning">
      Le schéma {report.sourceSchema} est lisible, mais seul le schéma 623 est actuellement projeté
      pour le viewer. Les métadonnées extraites sont affichées ci-dessus.
    </p>
  );
  let studyContent = null;
  if (study !== null) {
    viewerContent = <StudyViewer study={study} inspection={inspection} />;
    studyContent = (
      <div className="stat-grid">
        <div>
          <strong>{study.groups.length}</strong>
          <span>groupes</span>
        </div>
        <div>
          <strong>{study.questions.length}</strong>
          <span>questions</span>
        </div>
        <div>
          <strong>{study.subquestions.length}</strong>
          <span>sous-questions</span>
        </div>
        <div>
          <strong>{study.answers.length}</strong>
          <span>options</span>
        </div>
      </div>
    );
  }
  const diagnostics = report.diagnostics.map((item, index) => (
    <li key={`${index}-${item.location}`} data-severity={item.severity}>
      <strong>{item.code}</strong>
      <code>{item.location}</code>
      <span>{item.message}</span>
    </li>
  ));
  const sections = report.sections.map((section) => (
    <li key={section.name}>
      <code>{section.name}</code>
      <span>{section.rows} ligne(s)</span>
    </li>
  ));
  let omittedContent = null;
  if (report.omittedDiagnostics > 0)
    omittedContent = (
      <p>{report.omittedDiagnostics} diagnostic(s) supplémentaire(s) non affiché(s).</p>
    );
  return (
    <div className="inspection">
      <section className="panel" aria-labelledby="study-title">
        <p className="eyebrow">02 · Explorer</p>
        <h2 id="study-title">{title}</h2>
        <p className="muted imported-file">{filename}</p>
        <div className="metadata-row">
          <span>Étude {metadata.surveyId ?? "inconnue"}</span>
          <span>Schéma {report.sourceSchema}</span>
          <span>Langues : {metadata.languages.join(", ")}</span>
        </div>
        {studyContent}
        <details className="technical-details">
          <summary>
            Rapport d’import · {report.errors} erreur(s), {report.warnings} avertissement(s)
          </summary>
          <p>Ces contrôles de structure ne certifient pas l’exécution complète dans LimeSurvey.</p>
          <ul className="diagnostics">{diagnostics}</ul>
          {omittedContent}
          <h3>Sections présentes</h3>
          <ul className="section-list">{sections}</ul>
        </details>
      </section>
      {viewerContent}
    </div>
  );
}
