import { useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { defaultTargetKey } from "../survey/defaults.ts";
import type { Question } from "./model.ts";
import { defaultTargets, setDefaultValue, type DefaultTarget } from "./default-values.ts";

interface Props {
  question: Question;
  baseLanguage: string;
  languages: readonly string[];
  onChange: (question: Question) => void;
}

function DefaultValueField({
  question,
  target,
  language,
  onChange,
}: {
  question: Question;
  target: DefaultTarget;
  language: string;
  onChange: Props["onChange"];
}) {
  const current = question.defaults?.values.find(
    (entry) =>
      entry.language === language && defaultTargetKey(entry.coordinate, entry.role) === target.key,
  );
  function handleEnabled(event: TargetedEvent<HTMLInputElement>): void {
    let value: string | null = null;
    if (event.currentTarget.checked) value = "";
    onChange(setDefaultValue(question, target, language, value));
  }
  function handleValue(event: TargetedEvent<HTMLInputElement>): void {
    onChange(setDefaultValue(question, target, language, event.currentTarget.value));
  }
  return (
    <fieldset>
      <legend>{target.label}</legend>
      <label className="checkbox-label">
        <input type="checkbox" checked={current !== undefined} onChange={handleEnabled} />
        Définir une valeur pour cette langue
      </label>
      <label>
        Valeur (code de réponse ou texte)
        <input
          value={current?.value ?? ""}
          disabled={current === undefined}
          maxLength={4096}
          onInput={handleValue}
        />
      </label>
    </fieldset>
  );
}

export function DefaultValues({ question, baseLanguage, languages, onChange }: Props) {
  const [selectedLanguage, setSelectedLanguage] = useState(baseLanguage);
  const [open, setOpen] = useState(false);
  const targets = defaultTargets(question);
  if (targets.length === 0) return null;
  const sameForAllLanguages = question.defaults?.sameForAllLanguages ?? false;
  let language = selectedLanguage;
  if (sameForAllLanguages || !languages.includes(language)) language = baseLanguage;
  function handleShared(event: TargetedEvent<HTMLInputElement>): void {
    onChange({
      ...question,
      defaults: {
        sameForAllLanguages: event.currentTarget.checked,
        values: question.defaults?.values ?? [],
      },
    });
  }
  function handleLanguage(event: TargetedEvent<HTMLSelectElement>): void {
    setSelectedLanguage(event.currentTarget.value);
  }
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  let content = null;
  if (open) {
    const fields = targets.map((target) => (
      <DefaultValueField
        key={target.key}
        question={question}
        target={target}
        language={language}
        onChange={onChange}
      />
    ));
    content = (
      <div className="editor-fields">
        <p className="notice">
          Ces valeurs initialisent les champs absents à l’entrée de leur page. Effacer une réponse
          conserve un champ vide, y compris après sauvegarde et reprise.
        </p>
        <label className="checkbox-label">
          <input type="checkbox" checked={sameForAllLanguages} onChange={handleShared} />
          Utiliser les valeurs de la langue principale pour toutes les langues
        </label>
        <label>
          Langue des valeurs
          <select value={language} disabled={sameForAllLanguages} onChange={handleLanguage}>
            {languages.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>
        <small>
          Une case cochée avec un texte vide définit une valeur vide. Une case décochée laisse le
          défaut absent. Les traductions restent conservées lors du changement de mode.
        </small>
        {fields}
      </div>
    );
  }
  return (
    <details className="question-settings" onToggle={handleToggle}>
      <summary>Valeurs par défaut</summary>
      {content}
    </details>
  );
}
