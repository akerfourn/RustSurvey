import type { InspectionReport } from "./types.ts";

export function sourceBehaviorIssues(
  report: Pick<InspectionReport, "sections" | "defaultProjection">,
): string[] {
  const issues: string[] = [];
  for (const section of report.sections) {
    if (section.rows === 0) continue;
    if (section.name === "defaultvalues" && report.defaultProjection?.status === "complete")
      continue;
    if (
      ["conditions", "defaultvalues", "quota", "quota_members", "quota_languagesettings"].includes(
        section.name,
      )
    )
      issues.push(`Section ${section.name} : comportement non pris en charge par cet aperçu.`);
  }
  for (const issue of report.defaultProjection?.issues ?? [])
    issues.push(`${issue.path} : ${issue.message}`);
  return issues;
}
