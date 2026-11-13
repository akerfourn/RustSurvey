import { useEffect, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { message } from "./api.ts";
import type { Definition } from "./model.ts";
import { activeLanguages, changeBaseLanguage, parseLanguages } from "./translations.ts";

export function LanguageSettings({
  definition,
  onChange,
}: {
  definition: Definition;
  onChange: (definition: Definition) => void;
}) {
  const [value, setValue] = useState(activeLanguages(definition).join(", "));
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setValue(activeLanguages(definition).join(", "));
  }, [definition.language, definition.languages]);
  function handleInput(event: TargetedEvent<HTMLInputElement>): void {
    setValue(event.currentTarget.value);
  }
  function handleApply(): void {
    try {
      const languages = parseLanguages(value, definition.language);
      onChange({ ...definition, languages });
      setValue(languages.join(", "));
      setError(null);
    } catch (cause) {
      setError(message(cause));
    }
  }
  function handleBase(event: TargetedEvent<HTMLSelectElement>): void {
    onChange(changeBaseLanguage(definition, event.currentTarget.value));
  }
  let notice = null;
  if (error !== null)
    notice = (
      <p role="alert" className="error">
        {error}
      </p>
    );
  return (
    <fieldset className="editor-fields">
      <legend>Langues du questionnaire</legend>
      <label>
        Langues proposées
        <input value={value} onInput={handleInput} placeholder="fr, en" maxLength={1024} />
      </label>
      <button type="button" className="secondary" onClick={handleApply}>
        Appliquer les langues
      </button>
      {notice}
      <label>
        Langue principale
        <select value={definition.language} onChange={handleBase}>
          {activeLanguages(definition).map((language) => (
            <option key={language} value={language}>
              {language}
            </option>
          ))}
        </select>
      </label>
      <p className="muted">
        Les traductions restent conservées si une langue est retirée du parcours. Changer la langue
        principale conserve les textes de la précédente.
      </p>
    </fieldset>
  );
}
