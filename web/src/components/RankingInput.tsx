import type { TargetedEvent } from "preact";
import { inputOptions } from "./ChoiceInput.tsx";
import type { QuestionInputProps } from "./QuestionInput.tsx";
import { attribute } from "../survey/attributes.ts";

export function RankingInput(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const showDisabled = attribute(question.source, "array_filter_style") === "1";
  const options = inputOptions(props).filter(
    (option) =>
      props.activeChoices === undefined || props.activeChoices.has(option.value) || showDisabled,
  );
  const selected = new Set(
    question.fields.map((field) => answers.get(field) ?? "").filter((value) => value !== ""),
  );
  function handleChange(event: TargetedEvent<HTMLSelectElement>): void {
    onAnswer(event.currentTarget.name, event.currentTarget.value);
  }
  const cells = question.cells.filter((cell) => props.activeFields.has(cell.key));
  const positions = cells.map((cell, index) => {
    const value = answers.get(cell.key) ?? "";
    const choices = options.map((option) => (
      <option
        key={option.value}
        value={option.value}
        disabled={
          (props.activeChoices !== undefined && !props.activeChoices.has(option.value)) ||
          (selected.has(option.value) && value !== option.value)
        }
      >
        {option.label}
      </option>
    ));
    return (
      <label className="ranking-position" key={cell.key}>
        Position {index + 1}
        <select name={cell.key} value={value} onChange={handleChange}>
          <option value="">Non classé</option>
          {choices}
        </select>
      </label>
    );
  });
  return <div className="ranking-input">{positions}</div>;
}
