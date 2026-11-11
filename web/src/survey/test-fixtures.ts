import type { SurveyQuestion, SurveyStudy, SurveySubquestion } from "./types.ts";

export function makeQuestion(
  id: string,
  code: string,
  overrides: Partial<SurveyQuestion> = {},
): SurveyQuestion {
  return {
    id,
    groupId: "10",
    code,
    sourceType: "S",
    family: "text",
    mandatory: "no",
    order: 0,
    relevance: "1",
    randomizationGroup: "",
    validation: "",
    other: false,
    labels: [],
    attributes: [],
    ...overrides,
  };
}

export function makeStudy(questions: SurveyQuestion[]): SurveyStudy {
  return {
    surveyId: "1",
    baseLanguage: "fr",
    languages: ["fr"],
    titles: [],
    showWelcome: false,
    messages: [],
    groups: [{ id: "10", order: 0, relevance: "1", randomizationGroup: "", labels: [] }],
    questions,
    subquestions: [],
    answers: [],
  };
}

export function makeSubquestion(
  id: string,
  parentId: string,
  code: string,
  scale = 0,
): SurveySubquestion {
  return {
    id,
    parentId,
    groupId: "10",
    code,
    scale,
    order: Number(id),
    relevance: "1",
    labels: [],
  };
}

export function makeChoiceArray(type: "H" | "F"): SurveyStudy {
  const study = makeStudy([
    makeQuestion("1", "GATE", { sourceType: "Y" }),
    makeQuestion("2", "GRID", { sourceType: type, relevance: "GATE == 'Y'" }),
    makeQuestion("3", "DETAIL", { relevance: "GRID_after == '02'" }),
    makeQuestion("4", "FOLLOWUP", { relevance: "DETAIL == 'go'" }),
  ]);
  study.subquestions = [makeSubquestion("21", "2", "before"), makeSubquestion("22", "2", "after")];
  study.answers = [
    {
      id: "31",
      questionId: "2",
      code: "01",
      order: 0,
      scale: 0,
      labels: [{ language: "fr", text: "Option A" }],
    },
    {
      id: "32",
      questionId: "2",
      code: "02",
      order: 1,
      scale: 0,
      labels: [{ language: "fr", text: "Option B" }],
    },
  ];
  return study;
}

export function makeCheckboxMatrix(): SurveyStudy {
  const study = makeStudy([
    makeQuestion("1", "GATE", { sourceType: "Y" }),
    makeQuestion("2", "MATRIX", {
      sourceType: ":",
      relevance: "GATE == 'Y'",
      attributes: [{ name: "multiflexible_checkbox", value: "1", language: null }],
    }),
    makeQuestion("3", "DETAIL", { relevance: "MATRIX_R1_C1 == 1" }),
  ]);
  study.subquestions = [
    makeSubquestion("21", "2", "R1"),
    makeSubquestion("22", "2", "R10"),
    makeSubquestion("23", "2", "C1", 1),
    makeSubquestion("24", "2", "C2", 1),
  ];
  return study;
}
