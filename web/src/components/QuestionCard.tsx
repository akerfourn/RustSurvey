import type { RuntimeQuestion } from "../survey/engine.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import { QuestionInput, type QuestionInputProps } from "./QuestionInput.tsx";

interface QuestionCardProps extends QuestionInputProps {
  fields: ReadonlyMap<string, string>;
  errors: string[];
  warnings: string[];
  readOnly: boolean;
}

function requirement(question: RuntimeQuestion): string {
  if (question.source.sourceType === "X") return "Information";
  if (question.source.mandatory === "strict") return "Obligatoire";
  if (question.source.mandatory === "soft") return "Recommandé";
  return "Facultatif";
}

export function QuestionCard(props: QuestionCardProps) {
  const {
    question,
    language,
    baseLanguage,
    answers,
    fields,
    errors,
    warnings,
    readOnly,
    onAnswer,
  } = props;
  const label = localized(question.source.labels, language, baseLanguage);
  const title = pipeText(plainText(label?.text) || question.source.code, answers, fields);
  const help = pipeText(plainText(label?.help), answers, fields);
  const issues = [...question.issues, ...title.issues, ...help.issues];
  const issueContent = issues.map((issue, index) => (
    <p className="warning" key={`${index}-${issue}`}>
      {issue}
    </p>
  ));
  const errorContent = errors.map((error) => (
    <p className="error" key={error}>
      {error}
    </p>
  ));
  const warningContent = warnings.map((warning) => (
    <p className="warning" key={warning}>
      {warning}
    </p>
  ));
  let helpContent = null;
  if (help.text !== "")
    helpContent = (
      <p className="question-help" id={`help-${question.source.id}`}>
        {help.text}
      </p>
    );
  function handleClear(): void {
    for (const field of question.fields) onAnswer(field, "");
  }
  if (question.source.sourceType === "X") {
    return (
      <section className="question information">
        <p className="question-meta">{question.source.code} · Information</p>
        <p className="question-title">{title.text}</p>
        {helpContent}
        {issueContent}
      </section>
    );
  }
  let clearContent = null;
  if (question.issues.length === 0 && !readOnly)
    clearContent = (
      <button type="button" className="text-button" onClick={handleClear}>
        Effacer la réponse
      </button>
    );
  return (
    <fieldset
      className="question"
      disabled={readOnly || question.issues.length > 0}
      aria-describedby={`help-${question.source.id}`}
    >
      <legend>
        <span className="question-meta">
          {question.source.code} · {requirement(question)}
        </span>
        <span className="question-title">{title.text}</span>
      </legend>
      {helpContent}
      {issueContent}
      <QuestionInput {...props} />
      <div aria-live="polite">
        {errorContent}
        {warningContent}
      </div>
      {clearContent}
    </fieldset>
  );
}
