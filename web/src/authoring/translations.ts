import type { Choice, Definition, Group, Question, Translation } from "./model.ts";

export interface TranslationTarget {
  id: string;
  label: string;
  text: string;
  help: string;
  translations: Translation[];
}
export function activeLanguages(definition: Definition): string[] {
  return [...new Set([definition.language, ...definition.languages])];
}

export function parseLanguages(source: string, base: string): string[] {
  const languages = [
    ...new Set([base, ...source.split(/[,;\s]+/).filter((value) => value !== "")]),
  ];
  if (
    languages.length > 32 ||
    languages.some(
      (language) => !/^[A-Za-z]+(?:-[A-Za-z]+)*$/.test(language) || language.length > 32,
    )
  )
    throw new Error("Prévoir au plus 32 codes de langue : fr, en, de, pt-BR…");
  return languages;
}

function upsert(
  translations: Translation[],
  language: string,
  text: string,
  help: string,
): Translation[] {
  return [...translations.filter((item) => item.language !== language), { language, text, help }];
}
function swapTexts(
  translations: Translation[],
  oldLanguage: string,
  language: string,
  text: string,
  help: string,
) {
  const saved = upsert(translations, oldLanguage, text, help);
  const translated = saved.find((item) => item.language === language);
  return { text: translated?.text ?? text, help: translated?.help ?? help, translations: saved };
}
function swapChoice(choice: Choice, oldLanguage: string, language: string): Choice {
  const swapped = swapTexts(choice.translations, oldLanguage, language, choice.label, choice.help);
  return { ...choice, label: swapped.text, help: swapped.help, translations: swapped.translations };
}
function swapQuestion(question: Question, oldLanguage: string, language: string): Question {
  const swapped = swapTexts(
    question.translations,
    oldLanguage,
    language,
    question.title,
    question.help,
  );
  return {
    ...question,
    title: swapped.text,
    help: swapped.help,
    translations: swapped.translations,
    choices: question.choices.map((choice) => swapChoice(choice, oldLanguage, language)),
    subquestions: question.subquestions.map((choice) => swapChoice(choice, oldLanguage, language)),
  };
}
function swapGroup(group: Group, oldLanguage: string, language: string): Group {
  const swapped = swapTexts(
    group.translations,
    oldLanguage,
    language,
    group.title,
    group.description,
  );
  return {
    ...group,
    title: swapped.text,
    description: swapped.help,
    translations: swapped.translations,
    questions: group.questions.map((question) => swapQuestion(question, oldLanguage, language)),
  };
}

export function changeBaseLanguage(definition: Definition, language: string): Definition {
  if (language === definition.language) return definition;
  const oldLanguage = definition.language;
  const translations = [
    ...definition.translations.filter((item) => item.language !== oldLanguage),
    {
      language: oldLanguage,
      title: definition.title,
      welcome: definition.welcome,
      end: definition.end,
    },
  ];
  const translated = translations.find((item) => item.language === language);
  return {
    ...definition,
    language,
    languages: [...new Set([language, ...activeLanguages(definition)])],
    title: translated?.title ?? definition.title,
    welcome: translated?.welcome ?? definition.welcome,
    end: translated?.end ?? definition.end,
    translations,
    groups: definition.groups.map((group) => swapGroup(group, oldLanguage, language)),
  };
}

export function translationTargets(definition: Definition): TranslationTarget[] {
  const targets: TranslationTarget[] = [];
  for (const group of definition.groups) {
    targets.push({
      id: group.id,
      label: `Groupe · ${group.title}`,
      text: group.title,
      help: group.description,
      translations: group.translations,
    });
    for (const question of group.questions) {
      targets.push({
        id: question.id,
        label: `${question.code} · ${question.title}`,
        text: question.title,
        help: question.help,
        translations: question.translations,
      });
      for (const choice of [...question.choices, ...question.subquestions])
        targets.push({
          id: choice.id,
          label: `${question.code} / ${choice.code} · ${choice.label}`,
          text: choice.label,
          help: choice.help,
          translations: choice.translations,
        });
    }
  }
  return targets;
}

export function translateTarget(
  definition: Definition,
  id: string,
  language: string,
  text: string,
  help: string,
): Definition {
  function translateChoice(choice: Choice): Choice {
    if (choice.id !== id) return choice;
    return { ...choice, translations: upsert(choice.translations, language, text, help) };
  }
  return {
    ...definition,
    groups: definition.groups.map((group) => {
      const questions = group.questions.map((question) => {
        let translations = question.translations;
        if (question.id === id) translations = upsert(translations, language, text, help);
        return {
          ...question,
          translations,
          choices: question.choices.map(translateChoice),
          subquestions: question.subquestions.map(translateChoice),
        };
      });
      let translations = group.translations;
      if (group.id === id) translations = upsert(translations, language, text, help);
      return { ...group, translations, questions };
    }),
  };
}
