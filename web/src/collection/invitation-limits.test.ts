import assert from "node:assert/strict";
import { test } from "node:test";
import { invitationLimits } from "./invitation-limits.ts";

test("test_invitation_limits_should_distinguish_one_response_many_responses_and_unlimited_responses", () => {
  const fields = { maximum: "3", unlimited: false, validFrom: "", validUntil: "" };
  assert.deepEqual(invitationLimits(fields, false), {
    maxUses: 3,
    validFrom: null,
    validUntil: null,
  });
  assert.equal(invitationLimits({ ...fields, unlimited: true }, false).maxUses, null);
  assert.equal(invitationLimits({ ...fields, unlimited: true }, true).maxUses, 1);
  assert.throws(() => invitationLimits({ ...fields, maximum: "0" }, false), {
    message: "Indiquez entre 1 et 100 000 soumissions autorisées.",
  });
});

test("test_invitation_dates_should_keep_explicit_timezone_instants_and_reject_an_inverted_interval", () => {
  const fields = {
    maximum: "1",
    unlimited: false,
    validFrom: "2026-01-01T10:00:00Z",
    validUntil: "2026-01-01T11:00:00Z",
  };
  const limits = invitationLimits(fields, false);
  assert.equal(limits.validUntil! - limits.validFrom!, 3600);
  assert.throws(() => invitationLimits({ ...fields, validUntil: fields.validFrom }, false), {
    message: "La fin de validité doit être postérieure au début.",
  });
  assert.throws(() => invitationLimits({ ...fields, validFrom: "invalid" }, false), {
    message: "Date de validité invalide.",
  });
});
