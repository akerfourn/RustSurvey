import type { ImportInfo } from "./model.ts";
export function ImportDetails({ id, source }: { id: string | null; source: ImportInfo | null }) {
  if (source === null || id === null) return null;
  const diagnostics = source.report.diagnostics.map((item, index) => (
    <li key={`${index}-${item.code}-${item.location}`}>
      {item.code} · {item.location} : {item.message}
    </li>
  ));
  return (
    <section className="panel import-details">
      <h2>Étude importée depuis un LSS</h2>
      <p>
        Les conditions des groupes et questions sont éditables. Les textes, types, traductions et
        autres options importées sont conservés et protégés dans ce lot.
      </p>
      <a
        className="button-link secondary"
        href={`/api/admin/studies/${encodeURIComponent(id)}/original.lss`}
      >
        Télécharger le LSS original inchangé
      </a>
      <p className="muted">
        L’original ne contient pas les modifications faites ici. Le réexport LSS modifié reste à
        réaliser.
      </p>
      <details>
        <summary>Rapport d’import · {source.report.warnings} avertissement(s)</summary>
        <p>
          Schéma {source.report.sourceSchema}. Empreinte SHA-256 : <code>{source.sha256}</code>
        </p>
        <ul>{diagnostics}</ul>
      </details>
    </section>
  );
}
