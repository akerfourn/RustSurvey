import type { Snapshot } from "./engine.ts";
import type { SurveyStudy } from "./types.ts";

export function downloadAnswers(
  study: SurveyStudy,
  language: string,
  snapshot: Snapshot,
  complete: boolean,
): void {
  const content = JSON.stringify(
    {
      format: "rustsurvey-local-preview-v1",
      surveyId: study.surveyId,
      language,
      complete,
      answers: Object.fromEntries(snapshot.answers),
    },
    null,
    2,
  );
  const url = URL.createObjectURL(new Blob([content], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "rustsurvey-local-answers.json";
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
