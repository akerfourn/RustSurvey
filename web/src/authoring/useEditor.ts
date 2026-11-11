import { useEffect, useRef, useState } from "preact/hooks";
import { ApiError, getStudy, message, saveStudy } from "./api.ts";
import { createDefinition, type Definition, type ValidationIssue } from "./model.ts";

export function useEditor(id: string | null, csrf: string | null) {
  const [definition, setDefinition] = useState<Definition>(() => createDefinition());
  const [revision, setRevision] = useState(0);
  const [ready, setReady] = useState(id === null);
  const [saving, setSaving] = useState(false);
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
          setRevision(study.revision);
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
  function reloadSaved(): void {
    if (dirty && !window.confirm("Remplacer le brouillon par la dernière version enregistrée ?"))
      return;
    setReload((value) => value + 1);
  }
  async function save(): Promise<void> {
    if (csrf === null || saving || !ready) return;
    const controller = new AbortController();
    saveRequest.current = controller;
    setSaving(true);
    setError(null);
    setIssues([]);
    try {
      const study = await saveStudy(id, revision, definition, csrf, controller.signal);
      if (controller.signal.aborted) return;
      setDefinition(study.definition);
      setRevision(study.revision);
      setDirty(false);
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
  return {
    definition,
    revision,
    ready,
    saving,
    dirty,
    error,
    issues,
    needsLogin,
    setNeedsLogin,
    change,
    reloadSaved,
    save,
  };
}
