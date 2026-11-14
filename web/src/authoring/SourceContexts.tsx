import { useEffect, useRef, useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { ApiError, message } from "./api.ts";
import {
  listSourceContexts,
  uploadSourceContext,
  type SourceContext,
} from "./source-context-api.ts";

interface Props {
  id: string;
  revision: number;
  csrf: string | null;
  selected: string | null;
  disabled: boolean;
  onSelect: (id: string | null) => void;
  onExpired: () => void;
  purpose?: "check" | "collection";
}

export function SourceContexts({
  id,
  revision,
  csrf,
  selected,
  disabled,
  onSelect,
  onExpired,
  purpose = "check",
}: Props) {
  const [items, setItems] = useState<SourceContext[]>([]);
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = useRef<AbortController | null>(null);
  const fileInput = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    async function load(): Promise<void> {
      try {
        const result = await listSourceContexts(id, revision, controller.signal);
        if (!controller.signal.aborted)
          setItems((previous) =>
            [
              ...previous,
              ...result.filter((item) => !previous.some((saved) => saved.id === item.id)),
            ].slice(0, 32),
          );
      } catch (cause) {
        if (controller.signal.aborted) return;
        setError(message(cause));
        if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403))
          onExpired();
      }
    }
    void load();
    return () => {
      controller.abort();
      active.current?.abort();
    };
  }, [id, revision, onExpired]);

  async function save(): Promise<void> {
    if (busy || disabled || file === null || csrf === null) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError(null);
    try {
      const context = await uploadSourceContext(id, revision, file, csrf, controller.signal);
      if (controller.signal.aborted) return;
      setItems((previous) =>
        [context, ...previous.filter((item) => item.id !== context.id)].slice(0, 32),
      );
      setFile(null);
      if (fileInput.current !== null) fileInput.current.value = "";
      onSelect(context.id);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function handleFile(event: TargetedEvent<HTMLInputElement>): void {
    setFile(event.currentTarget.files?.[0] ?? null);
  }
  function handleSave(): void {
    void save();
  }
  function handleSelected(event: TargetedEvent<HTMLSelectElement>): void {
    let next: string | null = event.currentTarget.value;
    if (next === "") next = null;
    onSelect(next);
  }
  const options = items.map((item) => (
    <option key={item.id} value={item.id}>
      Contexte {item.sha256.slice(0, 12)} · {new Date(item.createdAt * 1000).toLocaleString()}
    </option>
  ));
  let selectedContent = null;
  if (selected !== null)
    selectedContent = (
      <a href={`/api/admin/source-settings-contexts/${encodeURIComponent(selected)}/original.json`}>
        Télécharger le contexte original choisi
      </a>
    );
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  let explanation =
    "Choisir un contexte pour ce bilan ne modifie pas le questionnaire ni ses participations.";
  let label = "Contexte utilisé pour le prochain bilan";
  let emptyLabel = "LSS seul, sans contexte complémentaire";
  if (purpose === "collection") {
    explanation = "La nouvelle campagne conservera les réglages du contexte choisi.";
    label = "Contexte de la nouvelle campagne";
    emptyLabel = "Choisir un contexte source";
  }
  return (
    <fieldset disabled={busy || disabled}>
      <legend>Contexte des réglages source</legend>
      <p>
        Chaque fichier reste privé et lié à la révision {revision}. {explanation}
      </p>
      <label>
        {label}
        <select value={selected ?? ""} onChange={handleSelected}>
          <option value="">{emptyLabel}</option>
          {options}
        </select>
      </label>
      {selectedContent}
      <label>
        Ajouter un contexte JSON · 64 Kio maximum
        <input ref={fileInput} type="file" accept=".json,application/json" onChange={handleFile} />
      </label>
      <button type="button" onClick={handleSave} disabled={file === null || csrf === null}>
        Enregistrer et sélectionner ce contexte
      </button>
      {errorContent}
    </fieldset>
  );
}
