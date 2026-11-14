import type { ImportDetails } from "./api.ts";
import { ImportRows } from "./ImportRows.tsx";

export function ImportReport({
  batch,
  onExpired,
}: {
  batch: ImportDetails;
  onExpired: () => void;
}) {
  const report = batch.report;
  let progress = "Préparation en cours";
  if (batch.state === "prepared") progress = "Données conservées";
  const issues = report.issues.map((issue, index) => (
    <li key={`${index}-${issue.path}`}>
      {issue.path} : {issue.message}
    </li>
  ));
  let diagnostics = null;
  if (report.issueCount > 0)
    diagnostics = (
      <details>
        <summary>
          {report.issueCount} diagnostic(s) · {report.issues.length} affiché(s)
        </summary>
        <ul>{issues}</ul>
      </details>
    );
  let rows = null;
  if (batch.state === "prepared")
    rows = <ImportRows key={batch.id} id={batch.id} onExpired={onExpired} />;
  return (
    <section className="notice">
      <h3>
        {progress} · révision {batch.revision}
      </h3>
      <p role="status">
        {batch.loadedRows} / {batch.rowCount} lignes conservées.
      </p>
      <p>
        {report.submitted} soumise(s), {report.drafts} partielle(s), {report.unknownStatus} d'état
        inconnu.
      </p>
      <p>
        {report.mappedCells} cellule(s) reconnue(s), {report.unknownColumnCount} colonne(s) sans
        correspondance, {report.missingColumnCount} colonne(s) attendue(s) absente(s).
      </p>
      <p>
        Cette préparation ne permet pas encore de poursuivre la collecte ou les reprises LimeSurvey.
      </p>
      <div className="button-row">
        <a href={`/api/admin/response-imports/${batch.id}/original.lsr`}>Fichier original LSR</a>
        <a href={`/api/admin/response-imports/${batch.id}/mapping.json`}>
          Correspondances des colonnes (JSON)
        </a>
      </div>
      {diagnostics}
      {rows}
    </section>
  );
}
