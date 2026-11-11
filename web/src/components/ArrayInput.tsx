import type { TargetedEvent } from "preact";
import { fieldName } from "../survey/cells.ts";
import { localized, plainText } from "../survey/text.ts";
import type { SurveySubquestion } from "../survey/types.ts";
import type { QuestionInputProps } from "./QuestionInput.tsx";

interface RadioCellProps {
  field: string;
  value: string;
  checked: boolean;
  labelledBy: string;
  onAnswer: QuestionInputProps["onAnswer"];
}

function RadioCell({ field, value, checked, labelledBy, onAnswer }: RadioCellProps) {
  function handleChange(event: TargetedEvent<HTMLInputElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  return (
    <td>
      <label className="array-option">
        <input
          type="radio"
          name={field}
          value={value}
          checked={checked}
          aria-labelledby={labelledBy}
          onChange={handleChange}
        />
      </label>
    </td>
  );
}

interface ArrayOption {
  id: string;
  code: string;
  label: string;
}

function options({ question, language, baseLanguage }: QuestionInputProps): ArrayOption[] {
  const choices = question.choices.map((choice) => ({
    id: `array-${question.source.id}-answer-${choice.id}`,
    code: choice.code,
    label: plainText(localized(choice.labels, language, baseLanguage)?.text) || choice.code,
  }));
  if (question.source.mandatory === "no")
    choices.push({ id: `array-${question.source.id}-empty`, code: "", label: "Sans réponse" });
  return choices;
}

function subquestionLabel(subquestion: SurveySubquestion, props: QuestionInputProps): string {
  return (
    plainText(localized(subquestion.labels, props.language, props.baseLanguage)?.text) ||
    subquestion.code
  );
}

function subquestionHeaderId(questionId: string, subquestionId: string): string {
  return `array-${questionId}-subquestion-${subquestionId}`;
}

function ChoiceArray(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const choices = options(props);
  const subquestions = question.subquestions.filter((item) => item.scale === 0);
  const transpose = question.source.sourceType === "H";
  let headers = choices.map((choice) => (
    <th key={choice.id} id={choice.id} scope="col">
      {choice.label}
    </th>
  ));
  let rows = subquestions.map((subquestion) => {
    const field = fieldName(question.source, subquestion);
    const headerId = subquestionHeaderId(question.source.id, subquestion.id);
    const cells = choices.map((choice) => (
      <RadioCell
        key={choice.id}
        field={field}
        value={choice.code}
        checked={(answers.get(field) ?? "") === choice.code}
        labelledBy={`${headerId} ${choice.id}`}
        onAnswer={onAnswer}
      />
    ));
    return (
      <tr key={subquestion.id}>
        <th id={headerId} scope="row">
          {subquestionLabel(subquestion, props)}
        </th>
        {cells}
      </tr>
    );
  });
  let instruction = "Un choix par ligne.";
  if (transpose) {
    instruction = "Un choix par colonne.";
    headers = subquestions.map((subquestion) => (
      <th
        key={subquestion.id}
        id={subquestionHeaderId(question.source.id, subquestion.id)}
        scope="col"
      >
        {subquestionLabel(subquestion, props)}
      </th>
    ));
    rows = choices.map((choice) => {
      const cells = subquestions.map((subquestion) => {
        const field = fieldName(question.source, subquestion);
        return (
          <RadioCell
            key={subquestion.id}
            field={field}
            value={choice.code}
            checked={(answers.get(field) ?? "") === choice.code}
            labelledBy={`${subquestionHeaderId(question.source.id, subquestion.id)} ${choice.id}`}
            onAnswer={onAnswer}
          />
        );
      });
      return (
        <tr key={choice.id}>
          <th id={choice.id} scope="row">
            {choice.label}
          </th>
          {cells}
        </tr>
      );
    });
  }
  return (
    <div
      className="array-scroll"
      role="region"
      aria-label={`Tableau ${question.source.code}`}
      tabIndex={0}
    >
      <table className="array-table">
        <caption>{instruction}</caption>
        <thead>
          <tr>
            <th scope="col">Réponses</th>
            {headers}
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}

function CheckboxMatrix(props: QuestionInputProps) {
  const { question, answers, onAnswer } = props;
  const rowAxis = question.subquestions.filter((item) => item.scale === 0);
  const columnAxis = question.subquestions.filter((item) => item.scale === 1);
  function handleCheckbox(event: TargetedEvent<HTMLInputElement>): void {
    const input = event.currentTarget;
    let value = "0";
    if (input.checked) value = "1";
    onAnswer(input.name, value);
  }
  const headers = columnAxis.map((column) => (
    <th key={column.id} id={subquestionHeaderId(question.source.id, column.id)} scope="col">
      {subquestionLabel(column, props)}
    </th>
  ));
  const rows = rowAxis.map((row) => {
    const rowHeader = subquestionHeaderId(question.source.id, row.id);
    const cells = columnAxis.map((column) => {
      const field = fieldName(question.source, row, column);
      return (
        <td key={column.id}>
          <label className="array-option">
            <input
              type="checkbox"
              name={field}
              checked={answers.get(field) === "1"}
              aria-labelledby={`${rowHeader} ${subquestionHeaderId(question.source.id, column.id)}`}
              onChange={handleCheckbox}
            />
          </label>
        </td>
      );
    });
    return (
      <tr key={row.id}>
        <th id={rowHeader} scope="row">
          {subquestionLabel(row, props)}
        </th>
        {cells}
      </tr>
    );
  });
  return (
    <div
      className="array-scroll"
      role="region"
      aria-label={`Matrice ${question.source.code}`}
      tabIndex={0}
    >
      <table className="array-table">
        <caption>Plusieurs cases peuvent être cochées par ligne.</caption>
        <thead>
          <tr>
            <th scope="col">Combinaisons</th>
            {headers}
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}

export function ArrayInput(props: QuestionInputProps) {
  if (props.question.source.sourceType === ":") return <CheckboxMatrix {...props} />;
  return <ChoiceArray {...props} />;
}
