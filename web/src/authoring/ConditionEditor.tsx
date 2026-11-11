import { ExpressionField } from "./ExpressionField.tsx";
import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ConditionRow } from "./ConditionRow.tsx";
import {
  conditionExpression,
  expressionIssues,
  visualCondition,
  type ConditionRow as Row,
  type Variable,
  type VisualCondition,
} from "./conditions.ts";

interface Draft extends Row {
  id: number;
}
interface Props {
  value: string;
  variables: readonly Variable[];
  onChange: (value: string) => void;
}
export function ConditionEditor(props: Props) {
  const [open, setOpen] = useState(false);
  function handleToggle(event: TargetedEvent<HTMLDetailsElement>): void {
    setOpen(event.currentTarget.open);
  }
  let editor = null;
  if (open) editor = <ConditionFields {...props} />;
  return (
    <details className="condition-editor" onToggle={handleToggle}>
      <summary>Conditions d’affichage</summary>
      {editor}
      <code className="condition-expression">{props.value || "1"}</code>
    </details>
  );
}
function ConditionFields({ value, variables, onChange }: Props) {
  const initial = visualCondition(value);
  const [advanced, setAdvanced] = useState(initial === null);
  const [join, setJoin] = useState<VisualCondition["join"]>(initial?.join ?? "all");
  const [rows, setRows] = useState<Draft[]>(
    () => initial?.rows.map((row, id) => ({ ...row, id })) ?? [],
  );
  const serial = useRef(rows.length);
  const lastValue = useRef(value);
  useEffect(() => {
    if (value === lastValue.current) return;
    lastValue.current = value;
    const parsed = visualCondition(value);
    setAdvanced(parsed === null);
    setJoin(parsed?.join ?? "all");
    setRows(parsed?.rows.map((row) => ({ ...row, id: serial.current++ })) ?? []);
  }, [value]);
  function emit(updated: Draft[], nextJoin: VisualCondition["join"]): void {
    setRows(updated);
    setJoin(nextJoin);
    const expression = conditionExpression({ join: nextJoin, rows: updated });
    lastValue.current = expression;
    onChange(expression);
  }
  function handleAdd(): void {
    const variable = variables[0];
    if (variable === undefined) return;
    let literalType: Row["literalType"] = "string";
    if (variable.kind === "number") literalType = "number";
    emit(
      [
        ...rows,
        {
          id: serial.current++,
          variable: variable.name,
          operator: "==",
          value: "",
          literalType,
        },
      ],
      join,
    );
  }
  function handleJoin(event: TargetedEvent<HTMLSelectElement>): void {
    emit(rows, event.currentTarget.value as VisualCondition["join"]);
  }
  function handleExpression(value: string): void {
    lastValue.current = value;
    onChange(value);
  }
  function handleAdvanced(): void {
    setAdvanced(true);
  }
  function handleVisual(): void {
    const parsed = visualCondition(value);
    if (parsed === null) return;
    setJoin(parsed.join);
    setRows(parsed.rows.map((row) => ({ ...row, id: serial.current++ })));
    setAdvanced(false);
  }
  const errors = expressionIssues(value, variables);
  const notices = errors.map((error) => <li key={error}>{error}</li>);
  let content = (
    <div>
      <label>
        Afficher si
        <select value={join} onChange={handleJoin}>
          <option value="all">Toutes les règles sont vraies (ET)</option>
          <option value="any">Au moins une règle est vraie (OU)</option>
        </select>
      </label>
      <p className="muted">Sans règle, cet élément est toujours affiché.</p>
      {rows.map((row) => {
        function handleChange(updated: Row): void {
          emit(
            rows.map((item) => {
              if (item.id === row.id) return { ...updated, id: row.id };
              return item;
            }),
            join,
          );
        }
        function handleRemove(): void {
          emit(
            rows.filter((item) => item.id !== row.id),
            join,
          );
        }
        return (
          <ConditionRow
            key={row.id}
            row={row}
            variables={variables}
            onChange={handleChange}
            onRemove={handleRemove}
          />
        );
      })}
      <button
        type="button"
        className="secondary"
        onClick={handleAdd}
        disabled={variables.length === 0 || rows.length >= 20}
      >
        Ajouter une règle
      </button>
      <button type="button" className="text-button" onClick={handleAdvanced}>
        Mode expression
      </button>
    </div>
  );
  if (advanced)
    content = (
      <div>
        <ExpressionField value={value} variables={variables} onChange={handleExpression} />
        <button
          type="button"
          className="text-button"
          onClick={handleVisual}
          disabled={visualCondition(value) === null}
        >
          Mode visuel
        </button>
        <p className="muted">
          Les expressions complexes restent dans ce mode, sans conversion forcée.
        </p>
      </div>
    );
  let issues = null;
  if (errors.length > 0)
    issues = (
      <ul className="notice warning" role="status">
        {notices}
      </ul>
    );
  return (
    <div>
      {content}
      {issues}
    </div>
  );
}
