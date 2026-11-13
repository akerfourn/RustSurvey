import { object, request, write } from "../authoring/api.ts";

export interface InvitationLimits {
  maxUses: number | null;
  validFrom: number | null;
  validUntil: number | null;
}

export interface Invitation extends InvitationLimits {
  id: string;
  runId: string;
  uses: number;
  revoked: boolean;
  createdAt: number;
}

export interface InvitationCommand extends InvitationLimits {
  id: string;
  token: string;
}

function integer(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value))
    throw new Error("Compteur d'invitation invalide.");
  return value;
}

function optionalInteger(value: unknown): number | null {
  if (value === null) return null;
  return integer(value);
}

function invitation(value: unknown): Invitation {
  const item = object(value);
  if (
    typeof item.id !== "string" ||
    typeof item.runId !== "string" ||
    typeof item.revoked !== "boolean"
  )
    throw new Error("Invitation invalide.");
  return {
    id: item.id,
    runId: item.runId,
    revoked: item.revoked,
    maxUses: optionalInteger(item.maxUses),
    validFrom: optionalInteger(item.validFrom),
    validUntil: optionalInteger(item.validUntil),
    uses: integer(item.uses),
    createdAt: integer(item.createdAt),
  };
}

export async function createInvitation(
  runId: string,
  command: InvitationCommand,
  csrf: string,
): Promise<Invitation> {
  return invitation(
    await request(`/api/admin/runs/${encodeURIComponent(runId)}/invitations`, {
      ...write("POST", command, csrf),
      signal: AbortSignal.timeout(20_000),
    }),
  );
}

export async function listInvitations(
  runId: string,
  offset: number,
  signal?: AbortSignal,
): Promise<{ items: Invitation[]; hasMore: boolean }> {
  const body = object(
    await request(`/api/admin/runs/${encodeURIComponent(runId)}/invitations?offset=${offset}`, {
      signal: signal ?? null,
    }),
  );
  if (!Array.isArray(body.items) || typeof body.hasMore !== "boolean")
    throw new Error("Liste d'invitations invalide.");
  return { items: body.items.map(invitation), hasMore: body.hasMore };
}

export async function revokeInvitation(
  runId: string,
  id: string,
  csrf: string,
): Promise<Invitation> {
  return invitation(
    await request(
      `/api/admin/runs/${encodeURIComponent(runId)}/invitations/${encodeURIComponent(id)}/revoke`,
      { ...write("POST", {}, csrf), signal: AbortSignal.timeout(20_000) },
    ),
  );
}
