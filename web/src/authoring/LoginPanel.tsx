import { useState } from "preact/hooks";
import type { TargetedEvent } from "preact";
import { login, message } from "./api.ts";
import type { Session } from "./model.ts";

interface LoginPanelProps {
  configured: boolean;
  onLogin: (session: Session) => void;
}
export function LoginPanel({ configured, onLogin }: LoginPanelProps) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  function handleUsername(event: TargetedEvent<HTMLInputElement>): void {
    setUsername(event.currentTarget.value);
  }
  function handlePassword(event: TargetedEvent<HTMLInputElement>): void {
    setPassword(event.currentTarget.value);
  }
  async function handleSubmit(event: TargetedEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const session = await login(username, password);
      setPassword("");
      onLogin(session);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  let notice = null;
  if (!configured)
    notice = (
      <p className="notice">
        Le compte initial doit être configuré au démarrage du serveur. Les instructions sont dans le
        README du projet.
      </p>
    );
  let errorContent = null;
  if (error !== null)
    errorContent = (
      <p className="notice error" role="alert">
        {error}
      </p>
    );
  return (
    <section className="panel login-panel">
      <h1>Administration</h1>
      <p>Connectez-vous pour créer et modifier les questionnaires.</p>
      {notice}
      <form onSubmit={handleSubmit} className="editor-fields">
        <label>
          Identifiant
          <input
            autoComplete="username"
            value={username}
            maxLength={64}
            required
            disabled={busy || !configured}
            onInput={handleUsername}
          />
        </label>
        <label>
          Mot de passe
          <input
            type="password"
            autoComplete="current-password"
            value={password}
            maxLength={1024}
            required
            disabled={busy || !configured}
            onInput={handlePassword}
          />
        </label>
        {errorContent}
        <button type="submit" disabled={busy || !configured}>
          Se connecter
        </button>
      </form>
    </section>
  );
}
