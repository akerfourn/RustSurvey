import { ConditionEditor } from "./ConditionEditor.tsx";
import type { Variable } from "./conditions.ts";
import type { TargetedEvent } from "preact";
import {
  QUESTION_KINDS,
  changeKind,
  type Mandatory,
  type Question,
  type QuestionKind,
} from "./model.ts";
import { QuestionAxes } from "./QuestionAxes.tsx";
import { QuestionSettings } from "./QuestionSettings.tsx";
import { QuestionMediaEditor } from "./QuestionMediaEditor.tsx";

interface QuestionEditorProps {
  question: Question;
  variables: readonly Variable[];
  readOnlyContent: boolean;
  index: number;
  count: number;
  onChange: (question: Question) => void;
  onMove: (direction: -1 | 1) => void;
  onRemove: () => void;
}
export function QuestionEditor({
  question,
  variables,
  readOnlyContent,
  index,
  count,
  onChange,
  onMove,
  onRemove,
}: QuestionEditorProps) {
  function handleCondition(relevance: string): void {
    onChange({ ...question, relevance });
  }
  function handleTitle(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...question, title: event.currentTarget.value });
  }
  function handleHelp(event: TargetedEvent<HTMLTextAreaElement>): void {
    onChange({ ...question, help: event.currentTarget.value });
  }
  function handleCode(event: TargetedEvent<HTMLInputElement>): void {
    onChange({ ...question, code: event.currentTarget.value });
  }
  function handleKind(event: TargetedEvent<HTMLSelectElement>): void {
    onChange(changeKind(question, event.currentTarget.value as QuestionKind));
  }
  function handleMandatory(event: TargetedEvent<HTMLSelectElement>): void {
    onChange({ ...question, mandatory: event.currentTarget.value as Mandatory });
  }
  function handleUp(): void {
    onMove(-1);
  }
  function handleDown(): void {
    onMove(1);
  }
  const kinds = [...QUESTION_KINDS].map(([kind, label]) => (
    <option key={kind} value={kind}>
      {label}
    </option>
  ));
  let importedType = null;
  if (!QUESTION_KINDS.has(question.kind))
    importedType = (
      <option value={question.kind}>{question.source?.kind ?? question.kind} · type importé</option>
    );
  return (
    <section className="question-editor" id={`edit-${question.id}`}>
      <div className="section-heading">
        <h3>Question {index + 1}</h3>
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
            Retirer
          </button>
        </div>
      </div>
      <fieldset className="editor-fields" disabled={readOnlyContent}>
        <label>
          Intitulé
          <input value={question.title} onInput={handleTitle} />
        </label>
        <div className="editor-columns">
          <label>
            Type
            <select value={question.kind} onChange={handleKind}>
              {importedType}
              {kinds}
            </select>
          </label>
          <label>
            Code
            <input value={question.code} maxLength={64} onInput={handleCode} />
          </label>
          <label>
            Réponse
            <select
              value={question.mandatory}
              onChange={handleMandatory}
              disabled={question.kind === "information" || question.kind === "equation"}
            >
              <option value="no">Facultative</option>
              <option value="soft">Recommandée</option>
              <option value="strict">Obligatoire</option>
            </select>
          </label>
        </div>
        <label>
          Aide
          <textarea rows={2} value={question.help} onInput={handleHelp} />
        </label>
        <QuestionAxes question={question} variables={variables} onChange={onChange} />
        <QuestionSettings question={question} variables={variables} onChange={onChange} />
        <QuestionMediaEditor question={question} onChange={onChange} />
      </fieldset>
      <ConditionEditor
        value={question.relevance}
        variables={variables}
        onChange={handleCondition}
      />
    </section>
  );
}
