import type { TargetedEvent } from "preact";
import { attribute, localizedAttribute } from "../survey/attributes.ts";
import { pipeText, plainText } from "../survey/text.ts";
import type { QuestionInputProps } from "./QuestionInput.tsx";

interface TextControlProps extends QuestionInputProps {
  field: string;
  label: string;
  type?: string;
  disabled?: boolean;
}

function boundedInteger(value: string, fallback: number, maximum: number): number {
  if (!/^\d+$/.test(value)) return fallback;
  return Math.max(1, Math.min(maximum, Number(value)));
}

export function TextControl(props: TextControlProps) {
  const {
    question,
    field,
    label,
    answers,
    onAnswer,
    language,
    baseLanguage,
    disabled = false,
  } = props;
  const type = props.type ?? question.source.sourceType;
  const source = question.source;
  const value = answers.get(field) ?? "";
  function setting(name: string): string {
    return pipeText(
      plainText(localizedAttribute(source, name, language, baseLanguage)),
      answers,
      props.fields,
    ).text;
  }
  const placeholder = setting("placeholder");
  const prefix = setting("prefix");
  const suffix = setting("suffix");
  const size = boundedInteger(attribute(source, "input_size"), 30, 200);
  let inputWidth: string | undefined;
  if (attribute(source, "input_size") !== "") inputWidth = `${size}ch`;
  const rows = boundedInteger(attribute(source, "display_rows"), 4, 50);
  let inputMode: "text" | "decimal" = "text";
  if (["N", "K", ":"].includes(type) || attribute(source, "numbers_only") === "1")
    inputMode = "decimal";
  function handleInput(event: TargetedEvent<HTMLInputElement | HTMLTextAreaElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  let control = (
    <input
      id={`field-${field}`}
      name={field}
      type="text"
      inputMode={inputMode}
      size={size}
      style={{ maxWidth: inputWidth }}
      value={value}
      placeholder={placeholder}
      aria-label={label}
      disabled={disabled}
      onInput={handleInput}
    />
  );
  if (type === "D") {
    control = (
      <input
        id={`field-${field}`}
        name={field}
        type="date"
        value={value}
        aria-label={label}
        disabled={disabled}
        onInput={handleInput}
      />
    );
  } else if (["T", "U"].includes(type)) {
    control = (
      <textarea
        id={`field-${field}`}
        name={field}
        rows={rows}
        value={value}
        placeholder={placeholder}
        aria-label={label}
        disabled={disabled}
        onInput={handleInput}
      />
    );
  }
  return (
    <span className="text-control">
      <span>{prefix}</span>
      {control}
      <span>{suffix}</span>
    </span>
  );
}
