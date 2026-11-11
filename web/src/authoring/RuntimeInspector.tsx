import type { Snapshot, SurveyRuntime } from "../survey/engine.ts";
export function RuntimeInspector({
  runtime,
  snapshot,
}: {
  runtime: SurveyRuntime;
  snapshot: Snapshot;
}) {
  const rows = [...runtime.questions.values()].slice(0, 200).map((question) => {
    let state = "Masquée";
    if (snapshot.visible.has(question.source.id)) state = "Visible";
    const fields = question.fields
      .map((field) => `${field} = ${snapshot.answers.get(field) ?? "(vide)"}`)
      .join(" ; ");
    const group = runtime.groups.find((group) => group.id === question.source.groupId);
    return (
      <tr key={question.source.id}>
        <td>
          <code>{question.source.code}</code>
        </td>
        <td>{state}</td>
        <td>
          <code>
            {group?.relevance ?? "1"} / {question.source.relevance}
          </code>
        </td>
        <td>{fields}</td>
      </tr>
    );
  });
  return (
    <details className="panel runtime-inspector">
      <summary>Inspecter les conditions et valeurs de cet aperçu</summary>
      <p>
        État local, jusqu’à 200 questions. Les conditions de groupe et de question doivent toutes
        deux être vraies.
      </p>
      <table>
        <thead>
          <tr>
            <th>Question</th>
            <th>Affichage</th>
            <th>Conditions groupe / question</th>
            <th>Valeurs</th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </details>
  );
}
