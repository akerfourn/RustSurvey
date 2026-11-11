import type { TargetedEvent } from "preact";
import { attribute, localizedAttribute } from "../survey/attributes.ts";
import { fieldName } from "../survey/cells.ts";
import { matrixOptions } from "../survey/matrix-options.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import { inputOptions } from "./ChoiceInput.tsx";
import type { QuestionInputProps } from "./QuestionInput.tsx";
import { TextControl } from "./TextControl.tsx";

export function subquestionLabel(id: string, props: QuestionInputProps): string {
  const row = props.question.subquestions.find((item) => item.id === id);
  if (row === undefined) return "";
  return pipeText(
    plainText(localized(row.labels, props.language, props.baseLanguage)?.text) || row.code,
    props.answers,
    props.fields,
  ).text;
}

export function GridInput(props: QuestionInputProps) {
  const { question, answers, onAnswer, activeFields, language, baseLanguage } = props;
  const dual = question.source.sourceType === "1";
  const rows = question.subquestions.filter((row) => row.scale === 0);
  const columns = question.subquestions.filter((row) => row.scale === 1);
  function handleSelect(event: TargetedEvent<HTMLSelectElement>): void {
    onAnswer(event.currentTarget.name, event.currentTarget.value);
  }
  const columnLabels = columns.map((column) => ({
    id: column.id,
    label: subquestionLabel(column.id, props),
  }));
  if (dual)
    for (const [index, name] of ["dualscale_headerA", "dualscale_headerB"].entries())
      columnLabels.push({
        id: String(index),
        label:
          plainText(localizedAttribute(question.source, name, language, baseLanguage)) ||
          `Échelle ${index + 1}`,
      });
  let numbers: string[] = [];
  if (question.source.sourceType === ":" && attribute(question.source, "input_boxes") !== "1") {
    try {
      numbers = matrixOptions(question.source, question.cells.length);
    } catch {
      numbers = [];
    }
  }
  const headings = columnLabels.map((column) => (
    <th key={column.id} scope="col">
      {column.label}
    </th>
  ));
  const content = rows.map((row) => {
    const cells = columnLabels.map((column) => {
      let field = `${fieldName(question.source, row)}_${column.id}`;
      if (!dual) {
        const sourceColumn = columns.find((candidate) => candidate.id === column.id);
        if (sourceColumn === undefined) return null;
        field = fieldName(question.source, row, sourceColumn);
      }
      const label = `${subquestionLabel(row.id, props)} — ${column.label}`;
      let input = (
        <TextControl
          {...props}
          field={field}
          label={label}
          type={question.source.sourceType}
          disabled={!activeFields.has(field)}
        />
      );
      if (
        dual ||
        (question.source.sourceType === ":" && attribute(question.source, "input_boxes") !== "1")
      ) {
        let options = numbers.map((value) => (
          <option key={value} value={value}>
            {value}
          </option>
        ));
        if (dual)
          options = inputOptions(props, Number(column.id)).map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ));
        input = (
          <select
            name={field}
            value={answers.get(field) ?? ""}
            aria-label={label}
            disabled={!activeFields.has(field)}
            onChange={handleSelect}
          >
            <option value="">Sans réponse</option>
            {options}
          </select>
        );
      }
      return <td key={column.id}>{input}</td>;
    });
    return (
      <tr key={row.id}>
        <th scope="row">{subquestionLabel(row.id, props)}</th>
        {cells}
      </tr>
    );
  });
  return (
    <div
      className="array-scroll"
      role="region"
      aria-label={`Tableau ${question.source.code}`}
      tabIndex={0}
    >
      <table className="array-table">
        <caption>Une réponse par cellule.</caption>
        <thead>
          <tr>
            <th scope="col">Questions</th>
            {headings}
          </tr>
        </thead>
        <tbody>{content}</tbody>
      </table>
    </div>
  );
}
