import { ConditionEditor } from "./ConditionEditor.tsx";
import type { Variable } from "./conditions.ts";
import type { TargetedEvent } from "preact";
import { moveItem, type Group, type Question } from "./model.ts";
import { QuestionEditor } from "./QuestionEditor.tsx";
import { pruneQuestionDefaults } from "./default-values.ts";

interface GroupEditorProps {
  group: Group;
  baseLanguage: string;
  languages: readonly string[];
  variables: readonly Variable[];
  readOnlyContent: boolean;
  index: number;
  count: number;
  canAddQuestion: boolean;
  onChange: (group: Group) => void;
  onMove: (direction: -1 | 1) => void;
  onRemove: () => void;
  onAddQuestion: () => void;
}
export function GroupEditor({
  group,
  baseLanguage,
  languages,
  variables,
  readOnlyContent,
  index,
  count,
  canAddQuestion,
  onChange,
  onMove,
  onRemove,
  onAddQuestion,
}: GroupEditorProps) {
  function handleCondition(relevance: string): void {
    onChange({ ...group, relevance });
  }
  function handleTitle(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...group, title: event.currentTarget.value });
  }
  function handleDescription(event: TargetedEvent<HTMLTextAreaElement>): void {
    onChange({ ...group, description: event.currentTarget.value });
  }
  function handleUp(): void {
    onMove(-1);
  }
  function handleDown(): void {
    onMove(1);
  }
  const questions = group.questions.map((question, questionIndex) => {
    function handleChange(updated: Question): void {
      const updatedQuestion = pruneQuestionDefaults(updated, question);
      onChange({
        ...group,
        questions: group.questions.map((item) =>
          item.id === updatedQuestion.id ? updatedQuestion : item,
        ),
      });
    }
    function handleMove(direction: -1 | 1): void {
      onChange({ ...group, questions: moveItem(group.questions, questionIndex, direction) });
    }
    function handleRemove(): void {
      if (window.confirm("Retirer cette question du brouillon ?"))
        onChange({
          ...group,
          questions: group.questions.filter((item) => item.id !== question.id),
        });
    }
    return (
      <QuestionEditor
        key={question.id}
        variables={variables}
        readOnlyContent={readOnlyContent}
        question={question}
        baseLanguage={baseLanguage}
        languages={languages}
        index={questionIndex}
        count={group.questions.length}
        onChange={handleChange}
        onMove={handleMove}
        onRemove={handleRemove}
      />
    );
  });
  return (
    <section className="panel group-editor" id={`edit-${group.id}`}>
      <div className="section-heading">
        <h2>Groupe {index + 1}</h2>
        <div className="button-row">
          <button
            type="button"
            className="text-button"
            onClick={handleUp}
            disabled={readOnlyContent || index === 0}
          >
            Monter
          </button>
          <button
            type="button"
            className="text-button"
            onClick={handleDown}
            disabled={readOnlyContent || index === count - 1}
          >
            Descendre
          </button>
          <button
            type="button"
            className="text-button"
            onClick={onRemove}
            disabled={readOnlyContent}
          >
            Retirer le groupe
          </button>
        </div>
      </div>
      <fieldset className="editor-fields" disabled={readOnlyContent}>
        <label>
          Titre du groupe
          <input value={group.title} maxLength={500} onInput={handleTitle} />
        </label>
        <label>
          Description
          <textarea rows={2} value={group.description} onInput={handleDescription} />
        </label>
      </fieldset>
      <ConditionEditor value={group.relevance} variables={variables} onChange={handleCondition} />
      {questions}
      <button
        type="button"
        className="secondary"
        onClick={onAddQuestion}
        disabled={readOnlyContent || !canAddQuestion}
      >
        Ajouter une question
      </button>
    </section>
  );
}
