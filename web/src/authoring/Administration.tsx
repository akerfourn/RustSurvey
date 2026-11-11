import { useEffect, useState } from "preact/hooks";
import { getSession, logout, message } from "./api.ts";
import type { Session } from "./model.ts";
import { Catalogue } from "./Catalogue.tsx";
import { EditorScreen } from "./EditorScreen.tsx";
import { LoginPanel } from "./LoginPanel.tsx";

const EMPTY_SESSION: Session = {
  authenticated: false,
  configured: true,
  username: null,
  csrfToken: null,
};
export function Administration({ id, editing }: { id: string | null; editing: boolean }) {
  const [session, setSession] = useState(EMPTY_SESSION);
  const [pending, setPending] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    async function load(): Promise<void> {
      try {
        const current = await getSession(controller.signal);
        if (!controller.signal.aborted) setSession(current);
      } catch (cause) {
        if (!controller.signal.aborted) setError(message(cause));
      } finally {
        if (!controller.signal.aborted) setPending(false);
      }
    }
    void load();
    return () => controller.abort();
  }, []);
  async function handleLogout(): Promise<void> {
    if (session.csrfToken === null) return;
    setBusy(true);
    setError(null);
    try {
      setSession(await logout(session.csrfToken));
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  let content = (
    <p className="panel" role="status">
      Vérification de la session…
    </p>
  );
  if (!pending) {
    content = <LoginPanel configured={session.configured} onLogin={setSession} />;
    if (session.authenticated) {
      let workspace = <Catalogue administrator={true} />;
      if (editing)
        workspace = (
          <EditorScreen key={id ?? "new"} id={id} session={session} onSession={setSession} />
        );
      content = (
        <div>
          <div className="admin-bar">
            <span>Administration · {session.username}</span>
            <button type="button" className="secondary" onClick={handleLogout} disabled={busy}>
              Se déconnecter
            </button>
          </div>
          {workspace}
        </div>
      );
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
      {notice}
      {content}
    </div>
  );
}
