import { useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import type { Variable } from "./conditions.ts";
export function VariablePanel({ variables }: { variables: readonly Variable[] }) {
  const [search, setSearch] = useState("");
  function handleSearch(event: TargetedEvent<HTMLInputElement>): void {
    setSearch(event.currentTarget.value);
  }
  const filtered = variables.filter((variable) =>
    variable.name.toLowerCase().includes(search.toLowerCase()),
  );
  const rows = filtered.slice(0, 200).map((variable) => (
    <tr key={variable.name}>
      <td>
        <code>{variable.name}</code>
      </td>
      <td>{variable.kind}</td>
      <td>{variable.label}</td>
    </tr>
  ));
  return (
    <details className="panel variable-panel">
      <summary>Variables du questionnaire · {variables.length}</summary>
      <label>
        Rechercher une variable
        <input value={search} onInput={handleSearch} />
      </label>
      <p>{filtered.length} résultat(s), jusqu’à 200 affichés.</p>
      <table>
        <thead>
          <tr>
            <th>Code</th>
            <th>Type de cellule</th>
            <th>Question</th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </details>
  );
}
