import type { TargetedEvent } from "preact";
import { inputOptions } from "./ChoiceInput.tsx";
import type { QuestionInputProps } from "./QuestionInput.tsx";

export function RankingInput(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const options = inputOptions(props);
  const selected = new Set(
    question.fields.map((field) => answers.get(field) ?? "").filter((value) => value !== ""),
  );
  function handleChange(event: TargetedEvent<HTMLSelectElement>): void {
    onAnswer(event.currentTarget.name, event.currentTarget.value);
  }
  const positions = question.cells.map((cell, index) => {
    const value = answers.get(cell.key) ?? "";
    const choices = options.map((option) => (
      <option
        key={option.value}
        value={option.value}
        disabled={selected.has(option.value) && value !== option.value}
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
