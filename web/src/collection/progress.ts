import type { ViewerDraft } from "./types.ts";

export interface ProgressState extends ViewerDraft {
  editVersion: number;
  savedVersion: number;
  attemptedVersion: number;
}

export function initialProgress(progress: ViewerDraft): ProgressState {
  return { ...progress, editVersion: 0, savedVersion: 0, attemptedVersion: 0 };
}

export function editProgress(state: ProgressState, progress: ViewerDraft): ProgressState {
  return { ...state, ...progress, editVersion: state.editVersion + 1 };
}

export function beginSave(state: ProgressState): ProgressState {
  return { ...state, attemptedVersion: state.editVersion };
}

export function acknowledgeProgress(
  state: ProgressState,
  sentVersion: number,
  saved: ViewerDraft,
): ProgressState {
  if (state.editVersion !== sentVersion) return { ...state, savedVersion: sentVersion };
  return {
    ...state,
    answers: saved.answers,
    language: saved.language,
    groupId: saved.groupId,
    pageId: saved.pageId,
    savedVersion: sentVersion,
  };
}

export function hasUnsavedChanges(state: ProgressState): boolean {
  return state.editVersion !== state.savedVersion;
}
