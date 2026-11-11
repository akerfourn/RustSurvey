import type { TargetedEvent } from "preact";

interface ViewerHeaderProps {
  language: string;
  languages: readonly string[];
  onLanguage: (language: string) => void;
}

export function ViewerHeader({ language, languages, onLanguage }: ViewerHeaderProps) {
  function handleChange(event: TargetedEvent<HTMLSelectElement>): void {
    onLanguage(event.currentTarget.value);
  }
  const options = languages.map((value) => (
    <option key={value} value={value}>
      {value.toUpperCase()}
    </option>
  ));
  return (
    <>
      <div className="section-heading">
        <div>
          <p className="eyebrow">Saisie dans le navigateur</p>
          <h2 id="viewer-title">Aperçu de l’étude</h2>
        </div>
        <label className="language-control">
          Langue
          <select value={language} onChange={handleChange}>
            {options}
          </select>
        </label>
      </div>
      <p className="notice">
        Aperçu partiel, sans compte. Les réponses restent dans cet onglet et disparaissent au
        rechargement. Aucun enregistrement serveur.
      </p>
    </>
  );
}

interface ViewerActionsProps {
  onDownload: () => void;
  onReset: () => void;
}

export function ViewerActions({ onDownload, onReset }: ViewerActionsProps) {
  return (
    <div className="local-actions">
      <button type="button" className="secondary" onClick={onDownload}>
        Télécharger les réponses (JSON)
      </button>
      <button type="button" className="text-button" onClick={onReset}>
        Recommencer la saisie
      </button>
    </div>
  );
}
