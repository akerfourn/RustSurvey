import { useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import type { Question } from "./model.ts";
import type { Variable } from "./conditions.ts";
import { ExpressionField } from "./ExpressionField.tsx";
import { questionProfile } from "./question-profile.ts";
import {
  questionSettings,
  settingValue,
  setSetting,
  type QuestionSetting,
} from "./question-settings.ts";

interface Props {
  question: Question;
  variables: readonly Variable[];
  onChange: (question: Question) => void;
}

function SettingField({
  question,
  setting,
  variables,
  onChange,
}: Props & { setting: QuestionSetting }) {
  const value = settingValue(question, setting.name);
  function handleValue(updated: string): void {
    onChange(setSetting(question, setting.name, updated));
  }
  function handleInput(event: TargetedEvent<HTMLInputElement>): void {
    handleValue(event.currentTarget.value);
  }
  function handleBoolean(event: TargetedEvent<HTMLSelectElement>): void {
    handleValue(event.currentTarget.value);
  }
  let hint = null;
  if (setting.hint !== undefined) hint = <small>{setting.hint}</small>;
  if (setting.kind === "expression")
    return (
      <ExpressionField
        value={value}
        variables={variables}
        label={setting.label}
        hint={
          setting.hint ??
          "Nombre ou expression utilisant les variables du questionnaire. Vide : aucune contrainte."
        }
        onChange={handleValue}
      />
    );
  if (setting.kind === "boolean")
    return (
      <label>
        {setting.label}
        <select value={value} onChange={handleBoolean}>
          <option value="">Défaut du moteur</option>
          <option value="1">Oui</option>
          <option value="0">Non</option>
        </select>
        {hint}
      </label>
    );
  let inputMode: "decimal" | undefined;
  if (setting.kind === "number") inputMode = "decimal";
  return (
    <label>
      {setting.label}
      <input value={value} inputMode={inputMode} maxLength={4096} onInput={handleInput} />
      {hint}
    </label>
  );
}

export function QuestionSettings({ question, variables, onChange }: Props) {
  const [open, setOpen] = useState(false);
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  function handleOther(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...question, other: event.currentTarget.checked });
  }
  let other = null;
  if (questionProfile(question.kind).other)
    other = (
      <label className="checkbox-label">
        <input type="checkbox" checked={question.other} onChange={handleOther} />
        Proposer une réponse « autre »
      </label>
    );
  let content = null;
  if (open)
    content = (
      <div className="editor-fields">
        {other}
        {questionSettings(question.kind).map((setting) => (
          <SettingField
            key={setting.name}
            question={question}
            variables={variables}
            setting={setting}
            onChange={onChange}
          />
        ))}
      </div>
    );
  return (
    <details className="question-settings" onToggle={handleToggle}>
      <summary>Calculs, validation et présentation</summary>
      {content}
    </details>
  );
}
