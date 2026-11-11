import { useId } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { OPERATORS, type ConditionRow as Row, type Variable } from "./conditions.ts";

interface Props {
  row: Row;
  variables: readonly Variable[];
  onChange: (row: Row) => void;
  onRemove: () => void;
}
export function ConditionRow({ row, variables, onChange, onRemove }: Props) {
  const listId = useId();
  function handleVariable(event: TargetedEvent<HTMLInputElement>): void {
    const variable = variables.find((item) => item.name === event.currentTarget.value);
    let literalType: Row["literalType"] = "string";
    if (variable?.kind === "number") literalType = "number";
    onChange({ ...row, variable: event.currentTarget.value, literalType });
  }
  function handleOperator(event: TargetedEvent<HTMLSelectElement>): void {
    onChange({ ...row, operator: event.currentTarget.value });
  }
  function handleValue(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...row, value: event.currentTarget.value });
  }
  const filtered = variables
    .filter((variable) => variable.name.toLowerCase().includes(row.variable.toLowerCase()))
    .slice(0, 100);
  const options = filtered.map((variable) => (
    <option key={variable.name} value={variable.name}>
      {variable.kind}
    </option>
  ));
  const variable = variables.find(
    (variable) => variable.name === row.variable.replace(/\.NAOK$/, ""),
  );
  const values = variable?.choices.map((choice) => (
    <option key={choice.code} value={choice.code}>
      {choice.label}
    </option>
  ));
  const operators = [...OPERATORS].map(([value, label]) => (
    <option key={value} value={value}>
      {label}
    </option>
  ));
  let value = null;
  if (row.operator !== "empty" && row.operator !== "answered")
    value = (
      <label>
        Valeur
        <input value={row.value} onInput={handleValue} list={`${listId}-values`} />
        <datalist id={`${listId}-values`}>{values}</datalist>
      </label>
    );
  return (
    <div className="condition-row">
      <label>
        Variable
        <input value={row.variable} list={listId} onInput={handleVariable} />
        <datalist id={listId}>{options}</datalist>
      </label>
      <label>
        Comparaison
        <select value={row.operator} onChange={handleOperator}>
          {operators}
        </select>
      </label>
      {value}
      <button type="button" className="text-button" onClick={onRemove}>
        Retirer la règle
      </button>
    </div>
  );
}
