import type { ImportInfo } from "./model.ts";
import { LssDownload } from "./LssDownload.tsx";
export function ImportDetails({
  id,
  source,
  revision,
  onExpired,
}: {
  id: string | null;
  source: ImportInfo | null;
  revision: number;
  onExpired: () => void;
}) {
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
      <LssDownload
        id={id}
        revision={null}
        label="Télécharger le LSS original inchangé"
        onExpired={onExpired}
      />
      <p className="muted">
        L’original ne contient pas les modifications faites ici. Le LSS modifié reprend les
        conditions de la révision enregistrée ; les autres octets sont conservés. Les anciennes
        conditions non interprétées peuvent bloquer ce réexport.
      </p>
      <LssDownload
        id={id}
        revision={revision}
        label={`Télécharger le LSS modifié · révision ${revision}`}
        onExpired={onExpired}
      />
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
