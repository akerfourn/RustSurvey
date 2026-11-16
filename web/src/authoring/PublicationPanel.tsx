import { publicStudyHash } from "./publication-links.ts";

interface Props {
  id: string | null;
  revision: number;
  publishedRevision: number | null;
  disabled: boolean;
  publishing: boolean;
  onPublish: () => void;
}

export function PublicationPanel({
  id,
  revision,
  publishedRevision,
  disabled,
  publishing,
  onPublish,
}: Props) {
  let status = "Ce questionnaire est un brouillon privé. Enregistrez-le avant de publier.";
  if (revision > 0) status = `Brouillon r${revision} enregistré, aucune révision publiée.`;
  if (publishedRevision !== null) status = `Révision publique : r${publishedRevision}.`;
  let label = "Publier cette révision";
  if (publishing) label = "Publication…";
  let link = null;
  if (id !== null && publishedRevision !== null) {
    link = (
      <a
        className="button-link secondary"
        href={publicStudyHash(id)}
        target="_blank"
        rel="noreferrer"
      >
        Ouvrir le questionnaire public
      </a>
    );
  }
  return (
    <section className="panel">
      <h2>Publication</h2>
      <p role="status">{status}</p>
      <p className="muted">
        Enregistrez le brouillon puis publiez pour les nouveaux arrivants. Les parcours déjà ouverts
        conservent leur version, sans interruption de l’étude.
      </p>
      <div className="button-row">
        <button
          type="button"
          onClick={onPublish}
          disabled={disabled || id === null || revision === publishedRevision}
        >
          {label}
        </button>
        {link}
      </div>
    </section>
  );
}
