import type { TargetedEvent } from "preact";
import { fieldName, type RuntimeQuestion } from "../survey/engine.ts";
import { localized, plainText } from "../survey/text.ts";
import type { Answers } from "../survey/types.ts";

export interface QuestionInputProps {
  question: RuntimeQuestion;
  answers: Answers;
  language: string;
  baseLanguage: string | null;
  onAnswer: (field: string, value: string) => void;
}

interface TextInputProps {
  field: string;
  value: string;
  label: string;
  type: string;
  onAnswer: (field: string, value: string) => void;
}

function TextInput({ field, value, label, type, onAnswer }: TextInputProps) {
  function handleInput(event: TargetedEvent<HTMLInputElement | HTMLTextAreaElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  let control = (
    <input
      id={`field-${field}`}
      name={field}
      type="text"
      value={value}
      aria-label={label}
      onInput={handleInput}
    />
  );
  if (["T", "U"].includes(type)) {
    control = (
      <textarea
        id={`field-${field}`}
        name={field}
        rows={4}
        value={value}
        aria-label={label}
        onInput={handleInput}
      />
    );
  } else if (["N", "K"].includes(type)) {
    control = (
      <input
        id={`field-${field}`}
        name={field}
        type="text"
        inputMode="decimal"
        value={value}
        aria-label={label}
        onInput={handleInput}
      />
    );
  } else if (type === "D") {
    control = (
      <input
        id={`field-${field}`}
        name={field}
        type="date"
        value={value}
        aria-label={label}
        onInput={handleInput}
      />
    );
  }
  return control;
}

interface Option {
  value: string;
  label: string;
}

function choices({ question, language, baseLanguage }: QuestionInputProps): Option[] {
  const type = question.source.sourceType;
  if (type === "5")
    return [1, 2, 3, 4, 5].map((value) => ({ value: String(value), label: String(value) }));
  if (type === "Y") {
    if (language === "fr")
      return [
        { value: "Y", label: "Oui" },
        { value: "N", label: "Non" },
      ];
    return [
      { value: "Y", label: "Yes" },
      { value: "N", label: "No" },
    ];
  }
  if (type === "G") {
    if (language === "fr")
      return [
        { value: "F", label: "Féminin" },
        { value: "M", label: "Masculin" },
      ];
    return [
      { value: "F", label: "Female" },
      { value: "M", label: "Male" },
    ];
  }
  return question.choices.map((choice) => ({
    value: choice.code,
    label: plainText(localized(choice.labels, language, baseLanguage)?.text) || choice.code,
  }));
}

function SingleChoice(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const field = question.source.code;
  const value = answers.get(field) ?? "";
  const options = choices(props);
  function handleChoice(event: TargetedEvent<HTMLInputElement | HTMLSelectElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  const optionContent = options.map((option) => (
    <option key={option.value} value={option.value}>
      {option.label}
    </option>
  ));
  if (question.source.sourceType === "!") {
    return (
      <select name={field} value={value} aria-label={field} onChange={handleChoice}>
        <option value="">Choisir…</option>
        {optionContent}
      </select>
    );
  }
  const radios = options.map((option) => (
    <label className="choice" key={option.value}>
      <input
        type="radio"
        name={field}
        value={option.value}
        checked={value === option.value}
        onChange={handleChoice}
      />
      <span>{option.label}</span>
    </label>
  ));
  return <div className="choices">{radios}</div>;
}

function MultipleInput({
  question,
  answers,
  language,
  baseLanguage,
  onAnswer,
}: QuestionInputProps) {
  function handleCheckbox(event: TargetedEvent<HTMLInputElement>): void {
    const input = event.currentTarget;
    let value = "";
    if (input.checked) value = "Y";
    onAnswer(input.name, value);
  }
  const inputs = question.subquestions.map((subquestion) => {
    const field = fieldName(question.source, subquestion);
    const label =
      plainText(localized(subquestion.labels, language, baseLanguage)?.text) || subquestion.code;
    if (question.source.sourceType === "M") {
      return (
        <label className="choice" key={subquestion.id}>
          <input
            type="checkbox"
            name={field}
            checked={answers.get(field) === "Y"}
            onChange={handleCheckbox}
          />
          <span>{label}</span>
        </label>
      );
    }
    return (
      <div className="subquestion" key={subquestion.id}>
        <label htmlFor={`field-${field}`}>{label}</label>
        <TextInput
          field={field}
          value={answers.get(field) ?? ""}
          label={label}
          type={question.source.sourceType}
          onAnswer={onAnswer}
        />
      </div>
    );
  });
  return <div className="choices">{inputs}</div>;
}

export function QuestionInput(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const type = question.source.sourceType;
  if (["L", "!", "5", "Y", "G"].includes(type)) return <SingleChoice {...props} />;
  if (["M", "Q", "K"].includes(type)) return <MultipleInput {...props} />;
  return (
    <TextInput
      field={question.source.code}
      value={answers.get(question.source.code) ?? ""}
      label={question.source.code}
      type={type}
      onAnswer={onAnswer}
    />
  );
}
