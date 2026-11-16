import { ImportDetails } from "./ImportDetails.tsx";
import { RevisionHistory } from "./RevisionHistory.tsx";
import { MediaLibrary } from "./MediaLibrary.tsx";
import { PublicationPanel } from "./PublicationPanel.tsx";
import { useCallback, useMemo, useState } from "preact/hooks";
import type { Session, ValidationIssue } from "./model.ts";
import { toViewer } from "./model.ts";
import { useEditor } from "./useEditor.ts";
import { DefinitionEditor } from "./DefinitionEditor.tsx";
import { LoginPanel } from "./LoginPanel.tsx";
import { StudyViewer } from "../components/StudyViewer.tsx";
import { ViewerBoundary } from "../components/ViewerBoundary.tsx";
import { RunManager } from "../collection/RunManager.tsx";

interface EditorScreenProps {
  id: string | null;
  session: Session;
  onSession: (session: Session) => void;
}
function issueLabel(issue: ValidationIssue): string {
  const group = /groups\[(\d+)\]/.exec(issue.path);
  const question = /questions\[(\d+)\]/.exec(issue.path);
  const choice = /choices\[(\d+)\]/.exec(issue.path);
  const parts: string[] = [];
  if (group !== null) parts.push(`Groupe ${Number(group[1]) + 1}`);
  if (question !== null) parts.push(`question ${Number(question[1]) + 1}`);
  if (choice !== null) parts.push(`choix ${Number(choice[1]) + 1}`);
  if (parts.length === 0) parts.push("Questionnaire");
  return `${parts.join(" · ")} : ${issue.message}`;
}
export function EditorScreen({ id, session, onSession }: EditorScreenProps) {
  const editor = useEditor(id, session.csrfToken);
  const [preview, setPreview] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const writing = editor.saving || editor.publishing || restoring;
  const study = useMemo(
    () => toViewer(editor.definition, id, editor.source !== null),
    [editor.definition, id, editor.source],
  );
  function handleSave(): void {
    void editor.save();
  }
  function handlePublish(): void {
    void editor.publish();
  }
  function handlePreview(): void {
    setPreview(!preview);
  }
  function handleBack(): void {
    if (!editor.dirty || window.confirm("Quitter sans enregistrer les modifications ?"))
      window.location.hash = "/admin";
  }
  function handleLogin(updated: Session): void {
    onSession(updated);
    editor.setNeedsLogin(false);
  }
  const handleExpired = useCallback(() => {
    editor.setNeedsLogin(true);
  }, [editor.setNeedsLogin]);
  let error = null;
  if (editor.error !== null)
    error = (
      <div className="notice error" role="alert">
        <p>{editor.error}</p>
        <ul>
          {editor.issues.map((issue, index) => (
            <li key={`${issue.path}-${index}`}>{issueLabel(issue)}</li>
          ))}
        </ul>
      </div>
    );
  let login = null;
  if (editor.needsLogin) login = <LoginPanel configured={true} onLogin={handleLogin} />;
  let status = "Brouillon non enregistré";
  if (editor.revision > 0) status = `Brouillon enregistré · révision ${editor.revision}`;
  if (editor.dirty) status = "Modifications non enregistrées";
  if (editor.saving) status = "Enregistrement…";
  if (restoring) status = "Restauration de la révision…";
  if (editor.publishing) status = "Publication…";
  let previewLabel = "Aperçu";
  if (preview) previewLabel = "Revenir à l’éditeur";
  let content = <p role="status">Chargement du questionnaire…</p>;
  if (editor.ready) {
    content = (
      <fieldset className="editor-body" disabled={writing}>
        <DefinitionEditor
          definition={editor.definition}
          readOnlyContent={editor.source !== null}
          onChange={editor.change}
        />
      </fieldset>
    );
    if (preview)
      content = (
        <ViewerBoundary key={`preview-${editor.revision}`}>
          <StudyViewer study={study} report={editor.source?.report} debug={true} />
        </ViewerBoundary>
      );
  }
  let reload = null;
  let media = null;
  if (id !== null && editor.ready && editor.source === null && session.csrfToken !== null)
    media = (
      <MediaLibrary
        id={id}
        csrf={session.csrfToken}
        definition={editor.definition}
        disabled={writing}
        onChange={editor.change}
        onExpired={handleExpired}
      />
    );
  let history = null;
  let collection = null;
  if (id !== null && editor.ready && session.csrfToken !== null)
    collection = (
      <RunManager
        key={id}
        studyId={id}
        revision={editor.revision}
        csrf={session.csrfToken}
        disabled={editor.dirty || writing}
        onExpired={handleExpired}
        imported={editor.source !== null}
      />
    );
  if (id !== null && editor.ready)
    history = (
      <RevisionHistory
        id={id}
        revision={editor.revision}
        csrf={session.csrfToken}
        disabled={editor.dirty || writing}
        onRestore={editor.acceptSaved}
        onBusy={setRestoring}
        report={editor.source?.report}
        onExpired={handleExpired}
      />
    );
  if (id !== null)
    reload = (
      <button type="button" className="text-button" onClick={editor.reloadSaved} disabled={writing}>
        Recharger la version enregistrée
      </button>
    );
  return (
    <div className="authoring-editor">
      <section className="panel editor-toolbar">
        <div>
          <h1>Éditeur de questionnaire</h1>
          <p role="status">{status}</p>
          <p className="muted">
            L’enregistrement modifie le brouillon privé. La publication rend cette version visible
            dans le catalogue ; les campagnes collectent séparément sur leur révision figée.
          </p>
        </div>
        <div className="button-row">
          <button type="button" className="secondary" onClick={handleBack} disabled={writing}>
            Mes questionnaires
          </button>
          <button
            type="button"
            className="secondary"
            onClick={handlePreview}
            disabled={!editor.ready || writing}
          >
            {previewLabel}
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={!editor.ready || writing || (!editor.dirty && editor.revision > 0)}
          >
            Enregistrer le brouillon
          </button>
          {reload}
        </div>
        {error}
      </section>
      {login}
      <PublicationPanel
        id={id}
        revision={editor.revision}
        publishedRevision={editor.publishedRevision}
        disabled={!editor.ready || writing || editor.dirty}
        publishing={editor.publishing}
        onPublish={handlePublish}
      />
      <ImportDetails
        id={id}
        source={editor.source}
        csrf={session.csrfToken}
        revision={editor.revision}
        onExpired={handleExpired}
      />
      {history}
      {media}
      {collection}
      {content}
    </div>
  );
}
