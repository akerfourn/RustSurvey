import { useEffect, useRef } from "preact/hooks";
import type { Snapshot, SurveyRuntime } from "../survey/engine.ts";
import { localized, plainText } from "../survey/text.ts";
import type { SurveyGroup } from "../survey/types.ts";
import type { Validation } from "../survey/validation.ts";
import { QuestionCard } from "./QuestionCard.tsx";

interface NavigationProps {
  previousDisabled: boolean;
  nextDisabled: boolean;
  nextLabel: string;
  onPrevious: () => void;
  onNext: () => void;
}

function StudyNavigation({
  previousDisabled,
  nextDisabled,
  nextLabel,
  onPrevious,
  onNext,
}: NavigationProps) {
  return (
    <nav className="viewer-navigation" aria-label="Parcours du questionnaire">
      <button type="button" className="secondary" disabled={previousDisabled} onClick={onPrevious}>
        Précédent
      </button>
      <button type="button" disabled={nextDisabled} onClick={onNext}>
        {nextLabel}
      </button>
    </nav>
  );
}

interface StudyPageProps extends NavigationProps {
  runtime: SurveyRuntime;
  snapshot: Snapshot;
  group: SurveyGroup;
  language: string;
  position: number;
  validation: Validation;
  attempted: boolean;
  readOnly: boolean;
  onAnswer: (field: string, value: string) => void;
  onClear: (fields: readonly string[]) => void;
}

export function StudyPage(props: StudyPageProps) {
  const {
    runtime,
    snapshot,
    group,
    language,
    position,
    validation,
    attempted,
    readOnly,
    onAnswer,
    onClear,
  } = props;
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    heading.current?.scrollIntoView({ block: "start" });
  }, [group.id]);
  const label = localized(group.labels, language, runtime.study.baseLanguage);
  const questions = [...runtime.questions.values()].filter(
    (question) => question.source.groupId === group.id && snapshot.visible.has(question.source.id),
  );
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
        question={question}
        answers={snapshot.answers}
        language={language}
        baseLanguage={runtime.study.baseLanguage}
        fields={runtime.fields}
        errors={errors}
        warnings={warnings}
        readOnly={readOnly}
        onAnswer={onAnswer}
        onClear={onClear}
      />
    );
  });
  let errorContent = null;
  if (attempted && validation.errors.size > 0)
    errorContent = (
      <p className="notice error" role="alert">
        Corrigez les réponses signalées avant de continuer.
      </p>
    );
  return (
    <div>
      <div className="group-heading">
        <p className="eyebrow">
          Groupe {position + 1} sur {snapshot.groups.length} · {questions.length} question(s)
          affichée(s)
        </p>
        <h3 ref={heading} tabIndex={-1}>
          {plainText(label?.name) || `Groupe ${group.id}`}
        </h3>
        <p className="muted">{plainText(label?.description)}</p>
        <progress
          value={position + 1}
          max={snapshot.groups.length}
          aria-label="Progression dans les groupes"
        />
      </div>
      <div className="navigation-top">
        <StudyNavigation {...props} />
      </div>
      {content}
      {errorContent}
      <StudyNavigation {...props} />
    </div>
  );
}
