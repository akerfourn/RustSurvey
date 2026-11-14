import type { NavigationSettings } from "../survey/navigation.ts";

interface NavigationControlProps {
  settings: NavigationSettings;
  disabled: boolean;
  onChange: (settings: NavigationSettings) => void;
}

export function NavigationControl({ settings, disabled, onChange }: NavigationControlProps) {
  function handleFormat(event: Event): void {
    if (!(event.currentTarget instanceof HTMLSelectElement)) return;
    const format = event.currentTarget.value;
    if (format === "group" || format === "question" || format === "all")
      onChange({ ...settings, format });
  }
  function handleProgress(event: Event): void {
    if (event.currentTarget instanceof HTMLInputElement)
      onChange({ ...settings, showProgress: event.currentTarget.checked });
  }
  return (
    <fieldset disabled={disabled}>
      <legend>Présentation de la campagne</legend>
      <label>
        Découpage des pages
        <select value={settings.format} onChange={handleFormat}>
          <option value="group">Un groupe par page</option>
          <option value="question">Une question par page</option>
          <option value="all">Tout sur une page</option>
        </select>
      </label>
      <label>
        <input type="checkbox" checked={settings.showProgress} onChange={handleProgress} />
        Afficher la progression
      </label>
      <p>
        Ces choix sont figés à l’ouverture de la campagne. Le retour aux pages précédentes est
        autorisé.
      </p>
    </fieldset>
  );
}
