import type { Definition } from "./model.ts";
function NodeLink({ id, label }: { id: string; label: string }) {
  function handleSelect(): void {
    document.getElementById(`edit-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
  return (
    <button type="button" className="text-button" onClick={handleSelect}>
      {label}
    </button>
  );
}
export function DefinitionOutline({ definition }: { definition: Definition }) {
  const groups = definition.groups.map((group) => (
    <li key={group.id}>
      <NodeLink id={group.id} label={group.title} />
      <ul>
        {group.questions.map((question) => (
          <li key={question.id}>
            <NodeLink
              id={question.id}
              label={`${question.code} · ${question.title.slice(0, 90)}`}
            />
          </li>
        ))}
      </ul>
    </li>
  ));
  return (
    <details className="panel editor-outline">
      <summary>Plan du questionnaire</summary>
      <nav aria-label="Groupes et questions">
        <ul>{groups}</ul>
      </nav>
    </details>
  );
}
