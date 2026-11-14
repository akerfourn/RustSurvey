import { useEffect, useRef, useState } from "preact/hooks";
import { ApiError, downloadLss, message } from "./api.ts";

interface Props {
  id: string;
  revision: number | null;
  label: string;
  onExpired: () => void;
  native?: boolean;
}
export function LssDownload({ id, revision, label, onExpired, native = false }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef<AbortController | null>(null);
  useEffect(() => () => request.current?.abort(), []);
  async function handleDownload(): Promise<void> {
    if (busy) return;
    const controller = new AbortController();
    request.current = controller;
    setBusy(true);
    setError(null);
    try {
      const blob = await downloadLss(id, revision, controller.signal, native);
      if (controller.signal.aborted) return;
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `original-${id}.lss`;
      if (revision !== null) link.download = `modified-${id}-r${revision}.lss`;
      if (native) link.download = `survey-${id}-r${revision}.lss`;
      document.body.append(link);
      link.click();
      link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(message(cause));
      if (cause instanceof ApiError && (cause.status === 401 || cause.status === 403)) onExpired();
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  let notice = null;
  if (error !== null)
    notice = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  return (
    <div>
      <button type="button" className="secondary" onClick={handleDownload} disabled={busy}>
        {label}
      </button>
      {notice}
    </div>
  );
}
