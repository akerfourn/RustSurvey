import type { InvitationLimits } from "./invitations-api.ts";

export interface InvitationFields {
  maximum: string;
  unlimited: boolean;
  validFrom: string;
  validUntil: string;
}

function timestamp(value: string): number | null {
  if (value === "") return null;
  const milliseconds = Date.parse(value);
  if (!Number.isFinite(milliseconds)) throw new Error("Date de validité invalide.");
  return Math.floor(milliseconds / 1000);
}

export function invitationLimits(fields: InvitationFields, persistent: boolean): InvitationLimits {
  let maxUses: number | null = 1;
  if (!persistent) {
    maxUses = Number(fields.maximum);
    if (fields.unlimited) maxUses = null;
    else if (!Number.isInteger(maxUses) || maxUses < 1 || maxUses > 100_000)
      throw new Error("Indiquez entre 1 et 100 000 soumissions autorisées.");
  }
  const validFrom = timestamp(fields.validFrom);
  const validUntil = timestamp(fields.validUntil);
  if (validFrom !== null && validUntil !== null && validFrom >= validUntil)
    throw new Error("La fin de validité doit être postérieure au début.");
  return { maxUses, validFrom, validUntil };
}
