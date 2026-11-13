import { ChoiceEditor } from "./ChoiceEditor.tsx";
import type { Variable } from "./conditions.ts";
import type { Choice, Question } from "./model.ts";
import { questionProfile } from "./question-profile.ts";

interface Props {
  question: Question;
  variables: readonly Variable[];
  onChange: (question: Question) => void;
}

export function QuestionAxes({ question, variables, onChange }: Props) {
  const profile = questionProfile(question.kind);
  const rows = question.subquestions.filter((item) => item.scale === 0);
  const columns = question.subquestions.filter((item) => item.scale === 1);
  const firstScale = question.choices.filter((item) => item.scale === 0);
  const secondScale = question.choices.filter((item) => item.scale === 1);
  function handleRows(updated: Choice[]): void {
    onChange({ ...question, subquestions: [...updated, ...columns] });
  }
  function handleColumns(updated: Choice[]): void {
    onChange({ ...question, subquestions: [...rows, ...updated] });
  }
  function handleFirstScale(updated: Choice[]): void {
    onChange({ ...question, choices: [...updated, ...secondScale] });
  }
  function handleSecondScale(updated: Choice[]): void {
    onChange({ ...question, choices: [...firstScale, ...updated] });
  }
  let rowEditor = null;
  let columnEditor = null;
  let choiceEditor = null;
  let secondEditor = null;
  if (profile.rows)
    rowEditor = (
      <ChoiceEditor
        label="Sous-questions / lignes"
        choices={rows}
        variables={variables}
        limit={200 - columns.length}
        onChange={handleRows}
      />
    );
  if (profile.columns)
    columnEditor = (
      <ChoiceEditor
        label="Colonnes"
        choices={columns}
        scale={1}
        variables={variables}
        limit={200 - rows.length}
        onChange={handleColumns}
      />
    );
  if (profile.choices)
    choiceEditor = (
      <ChoiceEditor
        label="Choix de réponse / première échelle"
        choices={firstScale}
        limit={200 - secondScale.length}
        onChange={handleFirstScale}
      />
    );
  if (profile.dualScale)
    secondEditor = (
      <ChoiceEditor
        label="Seconde échelle"
        choices={secondScale}
        scale={1}
        limit={200 - firstScale.length}
        onChange={handleSecondScale}
      />
    );
  return (
    <>
      {rowEditor}
      {columnEditor}
      {choiceEditor}
      {secondEditor}
    </>
  );
}
