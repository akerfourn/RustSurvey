import { useId, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import type { Variable } from "./conditions.ts";
interface Props {
  value: string;
  variables: readonly Variable[];
  onChange: (value: string) => void;
}
export function ExpressionField({ value, variables, onChange }: Props) {
  const id = useId();
  const textarea = useRef<HTMLTextAreaElement>(null);
  const [search, setSearch] = useState("");
  function handleExpression(event: TargetedEvent<HTMLTextAreaElement>): void {
    onChange(event.currentTarget.value);
  }
  function handleSearch(event: TargetedEvent<HTMLInputElement>): void {
    setSearch(event.currentTarget.value);
  }
  function handleInsert(): void {
    if (!variables.some((variable) => variable.name === search)) return;
    const start = textarea.current?.selectionStart ?? value.length;
    const end = textarea.current?.selectionEnd ?? start;
    onChange(value.slice(0, start) + search + value.slice(end));
    textarea.current?.focus();
    setSearch("");
  }
  const matches = variables
    .filter((variable) => variable.name.toLowerCase().includes(search.toLowerCase()))
    .slice(0, 100);
  const options = matches.map((variable) => (
    <option key={variable.name} value={variable.name}>
      {variable.kind}
    </option>
  ));
  return (
    <div>
      <label>
        Expression d’affichage
        <textarea
          ref={textarea}
          rows={3}
          value={value}
          maxLength={4096}
          onInput={handleExpression}
          aria-describedby={id}
        />
      </label>
      <p id={id} className="muted">
        Exemple : AGE &gt;= 18 &amp;&amp; CONSENT == 'Y'. Expressions interprétées par le moteur.
      </p>
      <div className="button-row">
        <label>
          Insérer une variable
          <input
            list={`${id}-variables`}
            value={search}
            onInput={handleSearch}
            placeholder="Rechercher un code"
          />
        </label>
        <datalist id={`${id}-variables`}>{options}</datalist>
        <button
          type="button"
          className="secondary"
          onClick={handleInsert}
          disabled={!variables.some((variable) => variable.name === search)}
        >
          Insérer
        </button>
      </div>
    </div>
  );
}
