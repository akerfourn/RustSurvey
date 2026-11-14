import { useEffect, useRef } from "preact/hooks";
import type { NavigationSettings, SurveyPage } from "../survey/navigation.ts";
import { StudyGroup, type StudyGroupProps } from "./StudyGroup.tsx";

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

interface StudyPageProps extends NavigationProps, Omit<StudyGroupProps, "group" | "questionIds"> {
  page: SurveyPage;
  pageCount: number;
  format: NavigationSettings["format"];
  showProgress: boolean;
  position: number;
}

export function StudyPage(props: StudyPageProps) {
  const { runtime, page, pageCount, position, validation, attempted } = props;
  const heading = useRef<HTMLDivElement>(null);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    heading.current?.scrollIntoView({ block: "start" });
  }, [page.id]);
  const content = runtime.groups.flatMap((group) => {
    const questionIds = page.questionIds.filter(
      (id) => runtime.questions.get(id)?.source.groupId === group.id,
    );
    if (questionIds.length === 0) return [];
    return <StudyGroup key={group.id} {...props} group={group} questionIds={questionIds} />;
  });
  let emptyContent = null;
  if (content.length === 0)
    emptyContent = (
      <p className="notice">Aucune question pertinente à afficher pour les réponses actuelles.</p>
    );
  let progress = null;
  let pageLabel = "Page";
  if (props.format === "group") pageLabel = "Groupe";
  if (props.format === "question") pageLabel = "Question";
  if (props.showProgress)
    progress = (
      <div>
        <p className="eyebrow">
          {pageLabel} {position + 1} sur {pageCount} · {page.questionIds.length} question(s)
          affichée(s)
        </p>
        <progress
          value={position + 1}
          max={pageCount}
          aria-label="Progression dans le questionnaire"
        />
      </div>
    );
  let errorContent = null;
  if (attempted && validation.errors.size > 0)
    errorContent = (
      <p className="notice error" role="alert">
        Corrigez les réponses signalées avant de continuer.
      </p>
    );
  return (
    <div>
      <div className="group-heading" ref={heading} tabIndex={-1} aria-label="Page du questionnaire">
        {progress}
      </div>
      <div className="navigation-top">
        <StudyNavigation {...props} />
      </div>
      {content}
      {emptyContent}
      {errorContent}
      <StudyNavigation {...props} />
    </div>
  );
}
