import type { TargetedEvent } from "preact";
import {
  QUESTION_KINDS,
  changeKind,
  type Mandatory,
  type Question,
  type QuestionKind,
  type Choice,
} from "./model.ts";
import { ChoiceEditor } from "./ChoiceEditor.tsx";

interface QuestionEditorProps {
  question: Question;
  index: number;
  count: number;
  onChange: (question: Question) => void;
  onMove: (direction: -1 | 1) => void;
  onRemove: () => void;
}
export function QuestionEditor({
  question,
  index,
  count,
  onChange,
  onMove,
  onRemove,
}: QuestionEditorProps) {
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
  function handleChoices(choices: Choice[]): void {
    onChange({ ...question, choices });
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
  let choices = null;
  if (question.kind === "single_choice" || question.kind === "multiple_choice")
    choices = <ChoiceEditor choices={question.choices} onChange={handleChoices} />;
  return (
    <section className="question-editor">
      <div className="section-heading">
        <h3>Question {index + 1}</h3>
        <div className="button-row">
          <button type="button" className="text-button" onClick={handleUp} disabled={index === 0}>
            Monter
          </button>
          <button
            type="button"
            className="text-button"
            onClick={handleDown}
            disabled={index === count - 1}
          >
            Descendre
          </button>
          <button type="button" className="text-button" onClick={onRemove}>
            Retirer
          </button>
        </div>
      </div>
      <div className="editor-fields">
        <label>
          Intitulé
          <input value={question.title} onInput={handleTitle} />
        </label>
        <div className="editor-columns">
          <label>
            Type
            <select value={question.kind} onChange={handleKind}>
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
              disabled={question.kind === "information"}
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
        {choices}
      </div>
    </section>
  );
}
