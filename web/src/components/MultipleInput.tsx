import type { TargetedEvent } from "preact";
import { attribute, displayColumns, localizedAttribute } from "../survey/attributes.ts";
import { fieldName } from "../survey/cells.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import type { QuestionInputProps } from "./QuestionInput.tsx";
import { TextControl } from "./TextControl.tsx";

function SliderInput(props: QuestionInputProps & { field: string; label: string }) {
  const { field, label, answers, onAnswer, question } = props;
  const min = attribute(question.source, "slider_min") || "0";
  const max = attribute(question.source, "slider_max") || "100";
  const step = attribute(question.source, "slider_accuracy") || "1";
  const value = answers.get(field) ?? "";
  let displayed = value;
  if (displayed === "") displayed = String((Number(min) + Number(max)) / 2);
  function handleInput(event: TargetedEvent<HTMLInputElement>): void {
    onAnswer(field, event.currentTarget.value);
  }
  function handleReset(): void {
    onAnswer(field, "");
  }
  let reset = null;
  if (attribute(question.source, "slider_reset") === "1")
    reset = (
      <button type="button" className="text-button" onClick={handleReset}>
        Effacer
      </button>
    );
  let bounds = null;
  if (attribute(question.source, "slider_showminmax") === "1")
    bounds = (
      <span>
        {min} – {max}
      </span>
    );
  return (
    <div className="slider-control">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={displayed}
        aria-label={label}
        disabled={!props.activeFields.has(field)}
        onInput={handleInput}
      />
      <output>{value || "Non renseigné"}</output>
      {bounds}
      {reset}
    </div>
  );
}

export function MultipleInput(props: QuestionInputProps) {
  const { question, answers, onAnswer, language, baseLanguage, activeFields, fields } = props;
  function handleCheckbox(event: TargetedEvent<HTMLInputElement>): void {
    const input = event.currentTarget;
    let value = "";
    if (input.checked) value = "Y";
    onAnswer(input.name, value);
  }
  const inputs = question.subquestions
    .filter((row) => row.scale === 0)
    .map((row) => {
      const field = fieldName(question.source, row);
      const label = pipeText(
        plainText(localized(row.labels, language, baseLanguage)?.text) || row.code,
        answers,
        fields,
      ).text;
      let content = (
        <TextControl {...props} field={field} label={label} disabled={!activeFields.has(field)} />
      );
      if (["M", "P"].includes(question.source.sourceType)) {
        let comment = null;
        if (question.source.sourceType === "P" && activeFields.has(`${field}comment`))
          comment = (
            <TextControl
              {...props}
              field={`${field}comment`}
              label={`Commentaire : ${label}`}
              type="S"
            />
          );
        content = (
          <div>
            <label className="choice">
              <input
                type="checkbox"
                name={field}
                checked={answers.get(field) === "Y"}
                disabled={!activeFields.has(field)}
                onChange={handleCheckbox}
              />
              <span>{label}</span>
            </label>
            {comment}
          </div>
        );
      } else if (
        question.source.sourceType === "K" &&
        attribute(question.source, "slider_layout") === "1"
      )
        content = <SliderInput {...props} field={field} label={label} />;
      return (
        <div className="subquestion" key={row.id}>
          <label htmlFor={`field-${field}`}>{label}</label>
          {content}
        </div>
      );
    });
  let other = null;
  if (question.source.other) {
    const field = `${question.source.code}_other`;
    const label =
      plainText(
        localizedAttribute(question.source, "other_replace_text", language, baseLanguage),
      ) || "Autre";
    let comment = null;
    if (question.source.sourceType === "P" && activeFields.has(`${field}comment`))
      comment = (
        <TextControl {...props} field={`${field}comment`} label="Commentaire pour autre" type="S" />
      );
    other = (
      <div className="subquestion">
        <label htmlFor={`field-${field}`}>{label}</label>
        <TextControl {...props} field={field} label={label} type="S" />
        {comment}
      </div>
    );
  }
  return (
    <div className="choices" style={{ "--choice-columns": displayColumns(question.source) }}>
      {inputs}
      {other}
    </div>
  );
}
