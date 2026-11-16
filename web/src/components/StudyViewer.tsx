import { RuntimeInspector } from "../authoring/RuntimeInspector.tsx";
import { StudyPage } from "./StudyPage.tsx";
import { ViewerActions, ViewerHeader } from "./ViewerControls.tsx";
import { SurveyMessage } from "./SurveyMessage.tsx";
import { useStudySession, type StudyViewerProps } from "./useStudySession.ts";

export function StudyViewer(props: StudyViewerProps) {
  const { study, persistence, debug = false } = props;
  const {
    runtime,
    mediaBaseUrl,
    snapshot,
    language,
    currentPage,
    pages,
    position,
    currentValidation,
    welcome,
    ending,
    hasWelcome,
    issues,
    notices,
    invalidStructure,
    readOnly,
    cannotComplete,
    started,
    attempted,
    complete,
    handleAnswer,
    handleClear,
    handleLanguageCode,
    handleStart,
    handleEdit,
    handlePrevious,
    handleNext,
    handleSaveDraft,
    handleDownload,
    handleReset,
    navigation,
    dirty,
    navigationDisabled,
  } = useStudySession(props);
  const issueItems = issues.map((issue, index) => <li key={`${index}-${issue}`}>{issue}</li>);
  let issueContent = null;
  if (issues.length > 0)
    issueContent = (
      <details className="notice warning">
        <summary>{issues.length} limite(s) du moteur d’aperçu</summary>
        <ul>{issueItems}</ul>
        <p>Les questions concernées restent consultables. La validation finale est désactivée.</p>
      </details>
    );
  let noticeContent = null;
  if (notices.length > 0)
    noticeContent = (
      <details className="notice">
        <summary>{notices.length} différence(s) de présentation</summary>
        <ul>
          {notices.map((notice) => (
            <li key={notice}>{notice}</li>
          ))}
        </ul>
      </details>
    );
  let readOnlyContent = null;
  if (invalidStructure)
    readOnlyContent = (
      <p className="notice error">
        La structure contient des erreurs ou dépasse les limites de l’aperçu. La saisie est
        désactivée.
      </p>
    );
  let nextLabel = "Groupe suivant";
  if (navigation?.format === "question") nextLabel = "Question suivante";
  if (position === pages.length - 1) nextLabel = "Valider la saisie locale";
  if (position === pages.length - 1 && persistence !== undefined)
    nextLabel = "Envoyer mes réponses";
  if (position === pages.length - 1 && persistence?.readOnly === true)
    nextLabel = "Terminer la consultation";
  if (
    !readOnly &&
    attempted &&
    currentValidation.errors.size === 0 &&
    currentValidation.warnings.size > 0
  )
    nextLabel = "Continuer sans les réponses recommandées";
  let pageContent = (
    <section>
      <p className="notice">Aucune question pertinente à afficher pour les réponses actuelles.</p>
      <button type="button" onClick={handleNext} disabled={navigationDisabled || cannotComplete}>
        {nextLabel}
      </button>
    </section>
  );
  let messageMode: "local" | "submitted" | "closed" = "local";
  if (persistence?.submitted === true) messageMode = "submitted";
  else if (persistence?.readOnly === true) messageMode = "closed";
  let nextDisabled = navigationDisabled;
  if (persistence?.readOnly !== true && position === pages.length - 1 && cannotComplete)
    nextDisabled = true;
  if (hasWelcome && !started)
    pageContent = <SurveyMessage message={welcome} completed={false} onContinue={handleStart} />;
  else if (complete)
    pageContent = (
      <SurveyMessage message={ending} completed={true} mode={messageMode} onContinue={handleEdit} />
    );
  else if (currentPage !== undefined)
    pageContent = (
      <StudyPage
        runtime={runtime}
        mediaBaseUrl={mediaBaseUrl}
        snapshot={snapshot}
        page={currentPage}
        pageCount={pages.length}
        format={navigation?.format ?? "group"}
        showProgress={navigation?.showProgress ?? true}
        language={language}
        position={position}
        validation={currentValidation}
        attempted={attempted}
        readOnly={readOnly}
        onAnswer={handleAnswer}
        onClear={handleClear}
        onLanguage={handleLanguageCode}
        previousDisabled={(position <= 0 && !hasWelcome) || navigationDisabled}
        nextDisabled={nextDisabled}
        nextLabel={nextLabel}
        onPrevious={handlePrevious}
        onNext={handleNext}
      />
    );
  let inspector = null;
  if (debug) inspector = <RuntimeInspector runtime={runtime} snapshot={snapshot} />;
  let actions = <ViewerActions onDownload={handleDownload} onReset={handleReset} />;
  if (persistence !== undefined) {
    let saveLabel = "Enregistrer pour reprendre plus tard";
    let status = "Progression enregistrée.";
    if (dirty) status = "Modifications non enregistrées. Sauvegarde automatique après une pause.";
    if (persistence.saveFailed) status = "Échec de sauvegarde. Corrigez la saisie ou réessayez.";
    if (persistence.saving) status = "Enregistrement…";
    if (persistence.uncertain) {
      saveLabel = "Réessayer le même enregistrement";
      status = "Enregistrement non confirmé. Réessayez avant de poursuivre.";
    }
    if (persistence.blocked) status = "Enregistrement bloqué. Consultez le message d’erreur.";
    actions = (
      <div className="local-actions">
        <button
          type="button"
          onClick={handleSaveDraft}
          disabled={
            persistence.readOnly ||
            persistence.submitted ||
            persistence.saving ||
            persistence.blocked
          }
        >
          {saveLabel}
        </button>
        <span role="status">{status}</span>
        <button type="button" className="secondary" onClick={handleDownload}>
          Télécharger une copie JSON
        </button>
      </div>
    );
  }
  return (
    <section className="viewer panel" aria-labelledby="viewer-title">
      {inspector}
      <ViewerHeader
        language={language}
        languages={study.languages}
        onLanguage={handleLanguageCode}
        durable={persistence !== undefined}
        disabled={readOnly}
      />
      {readOnlyContent}
      {issueContent}
      {noticeContent}
      {pageContent}
      {actions}
    </section>
  );
}
