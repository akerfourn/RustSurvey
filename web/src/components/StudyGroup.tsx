import type { Snapshot, SurveyRuntime } from "../survey/engine.ts";
import { localized, pipeText, plainText } from "../survey/text.ts";
import type { SurveyGroup } from "../survey/types.ts";
import type { Validation } from "../survey/validation.ts";
import { QuestionCard } from "./QuestionCard.tsx";

export interface StudyGroupProps {
  runtime: SurveyRuntime;
  snapshot: Snapshot;
  group: SurveyGroup;
  questionIds: string[];
  language: string;
  validation: Validation;
  attempted: boolean;
  readOnly: boolean;
  onAnswer: (field: string, value: string) => void;
  onClear: (fields: readonly string[]) => void;
  onLanguage: (language: string) => void;
  mediaBaseUrl: string | null;
}

export function StudyGroup(props: StudyGroupProps) {
  const { runtime, snapshot, group, questionIds, language, validation, attempted } = props;
  const label = localized(group.labels, language, runtime.study.baseLanguage);
  const groupTitle = pipeText(plainText(label?.name), snapshot.answers, runtime.fields).text;
  const groupDescription = pipeText(
    plainText(label?.description),
    snapshot.answers,
    runtime.fields,
  ).text;
  const questions = questionIds
    .map((id) => runtime.questions.get(id))
    .filter((question) => question !== undefined);
  const content = questions.map((question) => {
    let errors: string[] = [];
    let warnings: string[] = [];
    if (attempted) {
      errors = validation.errors.get(question.source.id) ?? [];
      warnings = validation.warnings.get(question.source.id) ?? [];
    }
    return (
      <QuestionCard
        key={question.source.id}
        question={{
          ...question,
          subquestions: question.subquestions.filter((row) =>
            snapshot.visibleSubquestions.has(row.id),
          ),
        }}
        answers={snapshot.answers}
        language={language}
        baseLanguage={runtime.study.baseLanguage}
        fields={runtime.fields}
        activeFields={snapshot.activeFields}
        activeChoices={snapshot.activeChoices.get(question.source.id)}
        otherVisible={snapshot.visibleOther.has(question.source.id)}
        languages={runtime.study.languages}
        onLanguage={props.onLanguage}
        errors={errors}
        warnings={warnings}
        readOnly={props.readOnly}
        onAnswer={props.onAnswer}
        onClear={props.onClear}
        mediaBaseUrl={props.mediaBaseUrl}
      />
    );
  });
  return (
    <div>
      <h3>{groupTitle || `Groupe ${group.id}`}</h3>
      <p className="muted">{groupDescription}</p>
      {content}
    </div>
  );
}
