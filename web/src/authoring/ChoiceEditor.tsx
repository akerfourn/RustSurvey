import type { TargetedEvent } from "preact";
import { createChoice, type Choice } from "./model.ts";

interface ChoiceRowProps {
  choice: Choice;
  index: number;
  onChange: (choice: Choice) => void;
  onRemove: () => void;
}
function ChoiceRow({ choice, index, onChange, onRemove }: ChoiceRowProps) {
  function handleCode(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...choice, code: event.currentTarget.value });
  }
  function handleLabel(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...choice, label: event.currentTarget.value });
  }
  return (
    <div className="choice-editor-row">
      <label>
        Code du choix {index + 1}
        <input value={choice.code} maxLength={32} onInput={handleCode} />
      </label>
      <label>
        Libellé
        <input value={choice.label} onInput={handleLabel} />
      </label>
      <button
        type="button"
        className="text-button"
        onClick={onRemove}
        aria-label={`Retirer le choix ${index + 1}`}
      >
        Retirer
      </button>
    </div>
  );
}
export function ChoiceEditor({
  choices,
  onChange,
}: {
  choices: Choice[];
  onChange: (choices: Choice[]) => void;
}) {
  function handleAdd(): void {
    let number = choices.length + 1;
    const codes = new Set(choices.map((choice) => choice.code));
    while (codes.has(`A${number}`)) number += 1;
    onChange([...choices, createChoice(`A${number}`, "Nouveau choix")]);
  }
  const rows = choices.map((choice, index) => {
    function handleChange(updated: Choice): void {
      onChange(choices.map((item) => (item.id === choice.id ? updated : item)));
    }
    function handleRemove(): void {
      onChange(choices.filter((item) => item.id !== choice.id));
    }
    return (
      <ChoiceRow
        key={choice.id}
        choice={choice}
        index={index}
        onChange={handleChange}
        onRemove={handleRemove}
      />
    );
  });
  return (
    <fieldset className="choice-editor">
      <legend>Choix de réponse</legend>
      {rows}
      <button
        type="button"
        className="secondary"
        onClick={handleAdd}
        disabled={choices.length >= 200}
      >
        Ajouter un choix
      </button>
    </fieldset>
  );
}
