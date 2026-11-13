import type { TargetedEvent } from "preact";
import { createChoice, moveItem, type Choice } from "./model.ts";
import { ConditionEditor } from "./ConditionEditor.tsx";
import type { Variable } from "./conditions.ts";

interface ChoiceRowProps {
  choice: Choice;
  index: number;
  onChange: (choice: Choice) => void;
  onRemove: () => void;
  onMove: (direction: -1 | 1) => void;
  count: number;
  variables: readonly Variable[] | null;
}
function ChoiceRow({
  choice,
  index,
  count,
  variables,
  onChange,
  onRemove,
  onMove,
}: ChoiceRowProps) {
  function handleCode(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...choice, code: event.currentTarget.value });
  }
  function handleLabel(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...choice, label: event.currentTarget.value });
  }
  function handleCondition(relevance: string): void {
    onChange({ ...choice, relevance });
  }
  function handleUp(): void {
    onMove(-1);
  }
  function handleDown(): void {
    onMove(1);
  }
  let condition = null;
  if (variables !== null)
    condition = (
      <ConditionEditor value={choice.relevance} variables={variables} onChange={handleCondition} />
    );
  return (
    <div className="choice-editor-item">
      <div className="choice-editor-row">
        <label>
          Code du choix {index + 1}
          <input value={choice.code} maxLength={128} onInput={handleCode} />
        </label>
        <div className="button-row">
          <button
            type="button"
            className="text-button"
            onClick={handleUp}
            disabled={index === 0}
            aria-label={`Monter le choix ${index + 1}`}
          >
            ↑
          </button>
          <button
            type="button"
            className="text-button"
            onClick={handleDown}
            disabled={index === count - 1}
            aria-label={`Descendre le choix ${index + 1}`}
          >
            ↓
          </button>
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
      </div>
      {condition}
    </div>
  );
}
export function ChoiceEditor({
  choices,
  onChange,
  label = "Choix de réponse",
  scale = 0,
  limit = 200,
  variables = null,
}: {
  choices: Choice[];
  onChange: (choices: Choice[]) => void;
  label?: string;
  scale?: number;
  limit?: number;
  variables?: readonly Variable[] | null;
}) {
  function handleAdd(): void {
    let number = choices.length + 1;
    const codes = new Set(choices.map((choice) => choice.code));
    while (codes.has(`A${number}`)) number += 1;
    onChange([...choices, { ...createChoice(`A${number}`, "Nouveau choix"), scale }]);
  }
  const rows = choices.map((choice, index) => {
    function handleChange(updated: Choice): void {
      onChange(choices.map((item) => (item.id === choice.id ? updated : item)));
    }
    function handleRemove(): void {
      onChange(choices.filter((item) => item.id !== choice.id));
    }
    function handleMove(direction: -1 | 1): void {
      onChange(moveItem(choices, index, direction));
    }
    return (
      <ChoiceRow
        key={choice.id}
        choice={choice}
        index={index}
        count={choices.length}
        variables={variables}
        onMove={handleMove}
        onChange={handleChange}
        onRemove={handleRemove}
      />
    );
  });
  return (
    <fieldset className="choice-editor">
      <legend>{label}</legend>
      {rows}
      <button
        type="button"
        className="secondary"
        onClick={handleAdd}
        disabled={choices.length >= limit}
      >
        Ajouter un choix
      </button>
    </fieldset>
  );
}
