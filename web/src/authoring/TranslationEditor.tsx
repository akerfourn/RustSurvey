import { useMemo, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import type { Definition } from "./model.ts";
import { activeLanguages, translateTarget, translationTargets } from "./translations.ts";

interface Props {
  definition: Definition;
  onChange: (definition: Definition) => void;
}
function TranslationBody({ definition, onChange }: Props) {
  const languages = activeLanguages(definition).filter(
    (language) => language !== definition.language,
  );
  const [selectedLanguage, setSelectedLanguage] = useState(languages[0] ?? "");
  const [selectedTarget, setSelectedTarget] = useState("@survey");
  const [search, setSearch] = useState("");
  const targets = useMemo(() => translationTargets(definition), [definition]);
  const language = languages.find((item) => item === selectedLanguage) ?? languages[0] ?? "";
  function handleLanguage(event: TargetedEvent<HTMLSelectElement>): void {
    setSelectedLanguage(event.currentTarget.value);
  }
  function handleTarget(event: TargetedEvent<HTMLSelectElement>): void {
    setSelectedTarget(event.currentTarget.value);
  }
  function handleSearch(event: TargetedEvent<HTMLInputElement>): void {
    setSearch(event.currentTarget.value);
  }
  const target = targets.find((item) => item.id === selectedTarget);
  const translated = target?.translations.find((item) => item.language === language);
  const surveyTranslation = definition.translations.find((item) => item.language === language);
  function handleText(event: TargetedEvent<HTMLTextAreaElement>): void {
    if (target !== undefined)
      onChange(
        translateTarget(
          definition,
          target.id,
          language,
          event.currentTarget.value,
          translated?.help ?? target.help,
        ),
      );
  }
  function handleHelp(event: TargetedEvent<HTMLTextAreaElement>): void {
    if (target !== undefined)
      onChange(
        translateTarget(
          definition,
          target.id,
          language,
          translated?.text ?? target.text,
          event.currentTarget.value,
        ),
      );
  }
  function updateSurvey(field: "title" | "welcome" | "end", value: string): void {
    const translation = {
      language,
      title: surveyTranslation?.title ?? definition.title,
      welcome: surveyTranslation?.welcome ?? definition.welcome,
      end: surveyTranslation?.end ?? definition.end,
      [field]: value,
    };
    onChange({
      ...definition,
      translations: [
        ...definition.translations.filter((item) => item.language !== language),
        translation,
      ],
    });
  }
  function handleSurveyTitle(event: TargetedEvent<HTMLInputElement>): void {
    updateSurvey("title", event.currentTarget.value);
  }
  function handleWelcome(event: TargetedEvent<HTMLTextAreaElement>): void {
    updateSurvey("welcome", event.currentTarget.value);
  }
  function handleEnd(event: TargetedEvent<HTMLTextAreaElement>): void {
    updateSurvey("end", event.currentTarget.value);
  }
  if (languages.length === 0)
    return <p>Ajouter une langue supplémentaire dans les réglages du questionnaire.</p>;
  const matches = [
    ...targets.filter((item) => item.id === selectedTarget),
    ...targets
      .filter(
        (item) =>
          item.id !== selectedTarget && item.label.toLowerCase().includes(search.toLowerCase()),
      )
      .slice(0, 99),
  ];
  let content = null;
  if (selectedTarget === "@survey")
    content = (
      <div className="editor-fields">
        <label>
          Titre traduit
          <input
            value={surveyTranslation?.title ?? definition.title}
            maxLength={200}
            onInput={handleSurveyTitle}
          />
        </label>
        <label>
          Accueil traduit
          <textarea
            value={surveyTranslation?.welcome ?? definition.welcome}
            rows={3}
            onInput={handleWelcome}
          />
        </label>
        <label>
          Fin traduite
          <textarea value={surveyTranslation?.end ?? definition.end} rows={3} onInput={handleEnd} />
        </label>
      </div>
    );
  else if (target !== undefined)
    content = (
      <div className="editor-fields">
        <p className="muted">Texte de base : {target.text}</p>
        <label>
          Intitulé / libellé traduit
          <textarea value={translated?.text ?? target.text} rows={3} onInput={handleText} />
        </label>
        <label>
          Aide / description traduite
          <textarea value={translated?.help ?? target.help} rows={2} onInput={handleHelp} />
        </label>
      </div>
    );
  return (
    <div className="editor-fields">
      <p>
        Les nouvelles traductions partent des textes de base. Enregistrer puis choisir la langue
        dans l’aperçu pour les vérifier.
      </p>
      <label>
        Langue à traduire
        <select value={language} onChange={handleLanguage}>
          {languages.map((item) => (
            <option key={item} value={item}>
              {item}
            </option>
          ))}
        </select>
      </label>
      <label>
        Rechercher un élément
        <input value={search} onInput={handleSearch} />
      </label>
      <label>
        Élément à traduire
        <select value={selectedTarget} onChange={handleTarget}>
          <option value="@survey">Titre, accueil et fin du questionnaire</option>
          {matches.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label.slice(0, 120)}
            </option>
          ))}
        </select>
      </label>
      {content}
    </div>
  );
}
export function TranslationEditor(props: Props) {
  const [open, setOpen] = useState(false);
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  let content = null;
  if (open) content = <TranslationBody {...props} />;
  return (
    <details className="panel" onToggle={handleToggle}>
      <summary>Traductions des textes et des choix</summary>
      {content}
    </details>
  );
}
