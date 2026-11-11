import type { TargetedEvent } from "preact";
import { createGroup, createQuestion, moveItem, type Definition, type Group } from "./model.ts";
import { GroupEditor } from "./GroupEditor.tsx";

export function DefinitionEditor({
  definition,
  onChange,
}: {
  definition: Definition;
  onChange: (definition: Definition) => void;
}) {
  function handleTitle(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...definition, title: event.currentTarget.value });
  }
  function handleLanguage(event: TargetedEvent<HTMLSelectElement>): void {
    onChange({ ...definition, language: event.currentTarget.value });
  }
  function handleWelcome(event: TargetedEvent<HTMLTextAreaElement>): void {
    onChange({ ...definition, welcome: event.currentTarget.value });
  }
  function handleEnd(event: TargetedEvent<HTMLTextAreaElement>): void {
    onChange({ ...definition, end: event.currentTarget.value });
  }
  function handleAddGroup(): void {
    onChange({ ...definition, groups: [...definition.groups, createGroup()] });
  }
  const questionCount = definition.groups.reduce(
    (count, group) => count + group.questions.length,
    0,
  );
  const groups = definition.groups.map((group, index) => {
    function handleChange(updated: Group): void {
      onChange({
        ...definition,
        groups: definition.groups.map((item) => (item.id === group.id ? updated : item)),
      });
    }
    function handleMove(direction: -1 | 1): void {
      onChange({ ...definition, groups: moveItem(definition.groups, index, direction) });
    }
    function handleRemove(): void {
      if (window.confirm("Retirer ce groupe et ses questions du brouillon ?"))
        onChange({
          ...definition,
          groups: definition.groups.filter((item) => item.id !== group.id),
        });
    }
    function handleAddQuestion(): void {
      handleChange({ ...group, questions: [...group.questions, createQuestion(definition)] });
    }
    return (
      <GroupEditor
        key={group.id}
        group={group}
        index={index}
        count={definition.groups.length}
        canAddQuestion={questionCount < 500}
        onChange={handleChange}
        onMove={handleMove}
        onRemove={handleRemove}
        onAddQuestion={handleAddQuestion}
      />
    );
  });
  return (
    <div>
      <section className="panel editor-fields">
        <label>
          Titre du questionnaire
          <input value={definition.title} maxLength={200} onInput={handleTitle} />
        </label>
        <label>
          Langue
          <select value={definition.language} onChange={handleLanguage}>
            <option value="fr">Français</option>
            <option value="en">English</option>
          </select>
        </label>
        <label>
          Texte d’accueil
          <textarea value={definition.welcome} rows={3} onInput={handleWelcome} />
        </label>
        <label>
          Texte de fin
          <textarea value={definition.end} rows={3} onInput={handleEnd} />
        </label>
      </section>
      {groups}
      <button
        type="button"
        className="secondary"
        disabled={definition.groups.length >= 50}
        onClick={handleAddGroup}
      >
        Ajouter un groupe
      </button>
    </div>
  );
}
