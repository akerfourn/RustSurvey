import type { TargetedEvent } from "preact";
import { attribute, displayColumns, localizedAttribute } from "../survey/attributes.ts";
import { fixedOptions } from "../survey/question-kinds.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import type { QuestionInputProps } from "./QuestionInput.tsx";
import { TextControl } from "./TextControl.tsx";

export interface InputOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export function inputOptions(props: QuestionInputProps, scale = 0): InputOption[] {
  const { question, language, baseLanguage, answers, fields } = props;
  const fixed = fixedOptions(question.source.sourceType, language);
  if (fixed !== null) return fixed.map((item) => ({ value: item.code, label: item.label }));
  return question.choices
    .filter((choice) => choice.scale === scale)
    .map((choice) => ({
      value: choice.code,
      label: pipeText(
        plainText(localized(choice.labels, language, baseLanguage)?.text) || choice.code,
        answers,
        fields,
      ).text,
    }));
}

export function ChoiceInput(props: QuestionInputProps) {
  const { question, answers, onAnswer, language, baseLanguage } = props;
  const field = question.source.code;
  const value = answers.get(field) ?? "";
  let options = inputOptions(props);
  if (question.source.other)
    options.push({
      value: "-oth-",
      label:
        plainText(
          localizedAttribute(question.source, "other_replace_text", language, baseLanguage),
        ) || "Autre",
    });
  const activeChoices = props.activeChoices;
  if (activeChoices !== undefined) {
    const showDisabled = attribute(question.source, "array_filter_style") === "1";
    options = options
      .filter((option) => activeChoices.has(option.value) || showDisabled)
      .map((option) => ({ ...option, disabled: !activeChoices.has(option.value) }));
  }
  function handleChange(event: TargetedEvent<HTMLInputElement | HTMLSelectElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  let control = (
    <div className="choices" style={{ "--choice-columns": displayColumns(question.source) }}>
      {options.map((option) => (
        <label className="choice" key={option.value}>
          <input
            type="radio"
            name={field}
            value={option.value}
            checked={value === option.value}
            disabled={option.disabled === true}
            onChange={handleChange}
          />
          <span>{option.label}</span>
        </label>
      ))}
    </div>
  );
  if (question.source.sourceType === "!")
    control = (
      <select name={field} value={value} aria-label={field} onChange={handleChange}>
        <option value="">Choisir…</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    );
  let extra = null;
  if (question.source.other && value === "-oth-")
    extra = (
      <TextControl {...props} field={`${field}_other`} label="Précisez la réponse autre" type="S" />
    );
  if (question.source.sourceType === "O")
    extra = <TextControl {...props} field={`${field}_comment`} label="Commentaire" type="T" />;
  return (
    <div>
      {control}
      {extra}
    </div>
  );
}
