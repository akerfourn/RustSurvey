import { useEffect, useRef, useState } from "preact/hooks";
import { ApiError, getStudy, message, publishStudy, saveStudy } from "./api.ts";
import {
  createDefinition,
  type Definition,
  type ImportInfo,
  type ValidationIssue,
  type StoredStudy,
} from "./model.ts";

export function useEditor(id: string | null, csrf: string | null) {
  const [definition, setDefinition] = useState<Definition>(() => createDefinition());
  const [source, setSource] = useState<ImportInfo | null>(null);
  const [revision, setRevision] = useState(0);
  const [publishedRevision, setPublishedRevision] = useState<number | null>(null);
  const [ready, setReady] = useState(id === null);
  const [saving, setSaving] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [needsLogin, setNeedsLogin] = useState(false);
  const [reload, setReload] = useState(0);
  const saveRequest = useRef<AbortController | null>(null);
  useEffect(() => () => saveRequest.current?.abort(), []);
  useEffect(() => {
    if (id === null) return;
    const controller = new AbortController();
    setReady(false);
    setError(null);
    async function load(): Promise<void> {
      try {
        const study = await getStudy(id!, controller.signal);
        if (!controller.signal.aborted) {
          setDefinition(study.definition);
          setSource(study.source);
          setRevision(study.revision);
          setPublishedRevision(study.publishedRevision);
          setReady(true);
          setDirty(false);
        }
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      }
    }
    void load();
    return () => controller.abort();
  }, [id, reload]);
  useEffect(() => {
    if (!dirty) return;
    function handleUnload(event: BeforeUnloadEvent): void {
      event.preventDefault();
    }
    window.addEventListener("beforeunload", handleUnload);
    return () => window.removeEventListener("beforeunload", handleUnload);
  }, [dirty]);
  function change(updated: Definition): void {
    setDefinition(updated);
    setDirty(true);
  }
  function acceptSaved(study: StoredStudy): void {
    setDefinition(study.definition);
    setSource(study.source);
    setRevision(study.revision);
    setPublishedRevision(study.publishedRevision);
    setDirty(false);
    setError(null);
    setIssues([]);
  }
  function reloadSaved(): void {
    if (dirty && !window.confirm("Remplacer le brouillon par la dernière version enregistrée ?"))
      return;
    setReload((value) => value + 1);
  }
  async function save(): Promise<void> {
    if (csrf === null || saving || publishing || !ready) return;
    const controller = new AbortController();
    saveRequest.current = controller;
    setSaving(true);
    setError(null);
    setIssues([]);
    try {
      const study = await saveStudy(id, revision, definition, csrf, controller.signal);
      if (controller.signal.aborted) return;
      acceptSaved(study);
      if (id === null) window.location.hash = `/admin/edit/${study.id}`;
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError) {
        setIssues(cause.issues);
        if (cause.status === 401 || cause.status === 403) setNeedsLogin(true);
      }
    } finally {
      if (!controller.signal.aborted) setSaving(false);
    }
  }
  async function publish(): Promise<void> {
    if (id === null || csrf === null || dirty || saving || publishing || !ready) return;
    const controller = new AbortController();
    saveRequest.current = controller;
    setPublishing(true);
    setError(null);
    try {
      const published = await publishStudy(id, revision, csrf, controller.signal);
      if (!controller.signal.aborted) setPublishedRevision(published);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) {
        setNeedsLogin(true);
      }
    } finally {
      if (!controller.signal.aborted) setPublishing(false);
    }
  }
  return {
    definition,
    source,
    revision,
    publishedRevision,
    ready,
    saving,
    publishing,
    dirty,
    error,
    issues,
    needsLogin,
    setNeedsLogin,
    change,
    acceptSaved,
    reloadSaved,
    save,
    publish,
  };
}
