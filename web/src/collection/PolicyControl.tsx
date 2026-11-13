import type { ParticipationPolicy } from "./api.ts";

export function policyChoice(value: string): ParticipationPolicy {
  if (value === "browserOnce") return { mode: "browserOnce" };
  if (value === "independent")
    return { mode: "invitation", persistence: false, allowEditAfterSubmit: false };
  if (value === "persistent" || value === "editable")
    return { mode: "invitation", persistence: true, allowEditAfterSubmit: value === "editable" };
  return { mode: "open" };
}

export function policyLabel(policy: ParticipationPolicy): string {
  if (policy.mode === "open") return "Accès public, participations multiples autorisées";
  if (policy.mode === "browserOnce") return "Accès public, limitation par cookie du navigateur";
  if (!policy.persistence) return "Invitation obligatoire, réponses indépendantes";
  if (policy.allowEditAfterSubmit)
    return "Invitation avec reprise et correction de la même réponse";
  return "Invitation avec reprise, réponse définitive après envoi";
}

export function PolicyControl({
  disabled,
  onChange,
}: {
  disabled: boolean;
  onChange: (policy: ParticipationPolicy) => void;
}) {
  function handleChange(event: Event): void {
    if (event.currentTarget instanceof HTMLSelectElement)
      onChange(policyChoice(event.currentTarget.value));
  }
  return (
    <div>
      <label>
        Accès et participations multiples
        <select onChange={handleChange} disabled={disabled}>
          <option value="open">Public — participations multiples autorisées</option>
          <option value="browserOnce">Public — limiter à un envoi par navigateur</option>
          <option value="independent">Sur invitation — réponses indépendantes</option>
          <option value="persistent">Sur invitation — reprendre la même réponse</option>
          <option value="editable">Sur invitation — reprendre et corriger après envoi</option>
        </select>
      </label>
      <p>
        Le cookie peut être effacé ou contourné avec un autre navigateur. Les invitations limitent
        les soumissions côté serveur ; leur nombre d’utilisations se règle pour chaque lien. La
        reprise par invitation crée une association entre l’invitation et la réponse.
      </p>
    </div>
  );
}
