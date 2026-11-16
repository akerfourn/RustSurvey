import type { TargetedEvent } from "preact";

interface ViewerHeaderProps {
  language: string;
  languages: readonly string[];
  onLanguage: (language: string) => void;
  durable?: boolean;
  disabled?: boolean;
}

export function ViewerHeader({
  language,
  languages,
  onLanguage,
  durable = false,
  disabled = false,
}: ViewerHeaderProps) {
  function handleChange(event: TargetedEvent<HTMLSelectElement>): void {
    onLanguage(event.currentTarget.value);
  }
  const options = languages.map((value) => (
    <option key={value} value={value}>
      {value.toUpperCase()}
    </option>
  ));
  let eyebrow = "Saisie dans le navigateur";
  let title = "Aperçu de l’étude";
  let notice =
    "Aperçu partiel, sans compte. Les réponses restent dans cet onglet et disparaissent au rechargement. Aucun enregistrement serveur.";
  if (durable) {
    eyebrow = "Participation au questionnaire";
    title = "Vos réponses";
    notice =
      "La progression est enregistrée automatiquement après une pause, aux changements de page ou avec le bouton d’enregistrement. Le bouton final envoie vos réponses.";
  }
  return (
    <>
      <div className="section-heading">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2 id="viewer-title">{title}</h2>
        </div>
        <label className="language-control">
          Langue
          <select value={language} onChange={handleChange} disabled={disabled}>
            {options}
          </select>
        </label>
      </div>
      <p className="notice">{notice}</p>
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
