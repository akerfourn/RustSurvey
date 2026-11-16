import { LssPreview } from "./LssPreview.tsx";
import { Catalogue } from "./authoring/Catalogue.tsx";
import { Administration } from "./authoring/Administration.tsx";
import { PublicStudy } from "./authoring/PublicStudy.tsx";
import { RunStart } from "./collection/RunStart.tsx";
import { Participation } from "./collection/Participation.tsx";
import { NavigationContext } from "./navigation/context.ts";
import { useHashRoute } from "./navigation/useHashRoute.ts";

export function App() {
  const { route, navigation } = useHashRoute();
  if (route === "#/import")
    return (
      <NavigationContext.Provider value={navigation}>
        <LssPreview />
      </NavigationContext.Provider>
    );
  let content = (
    <section className="panel">
      <h1>Page introuvable</h1>
      <a href="#/">Retour aux questionnaires</a>
    </section>
  );
  const publicStudy = /^#\/studies\/([a-f0-9-]{36})(?:\/revisions\/([1-9][0-9]{0,8}))?$/.exec(
    route,
  );
  const editStudy = /^#\/admin\/edit\/([a-f0-9-]{36})$/.exec(route);
  const run = /^#\/runs\/([a-f0-9-]{36})(?:\/invite\/([a-f0-9]{64}))?$/.exec(route);
  const response =
    /^#\/responses\/([a-f0-9-]{36})\/([a-f0-9]{64})(?:\/invite\/([a-f0-9]{64}))?$/.exec(route);
  if (route === "#/") content = <Catalogue />;
  else if (publicStudy !== null) {
    let revision: number | null = null;
    if (publicStudy[2] !== undefined) revision = Number(publicStudy[2]);
    content = <PublicStudy key={route} id={publicStudy[1]!} revision={revision} />;
  } else if (route === "#/admin") content = <Administration id={null} editing={false} />;
  else if (route === "#/admin/new") content = <Administration id={null} editing={true} />;
  else if (editStudy !== null) content = <Administration id={editStudy[1]!} editing={true} />;
  else if (run !== null) content = <RunStart id={run[1]!} invitationToken={run[2]} />;
  else if (response !== null)
    content = (
      <Participation
        key={route}
        id={response[1]!}
        token={response[2]!}
        invitationToken={response[3]}
      />
    );
  return (
    <NavigationContext.Provider value={navigation}>
      <header className="site-header">
        <a className="brand" href="#/">
          RustSurvey
        </a>
        <nav className="workspace-nav" aria-label="Navigation principale">
          <a href="#/">Questionnaires</a>
          <a href="#/import">Essayer un LSS</a>
          <a href="#/admin">Administration</a>
        </nav>
      </header>
      <main>{content}</main>
      <footer>RustSurvey · Éditeur et questionnaires publics</footer>
    </NavigationContext.Provider>
  );
}
