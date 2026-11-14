import type { GroupSetting } from "./group-settings.ts";

function SettingRow({ setting }: { setting: GroupSetting }) {
  let value = "À résoudre";
  if (setting.status === "invalid") value = "Valeur non prise en charge";
  else if (setting.value === true) value = "Oui";
  else if (setting.value === false) value = "Non";
  else if (setting.value !== null) value = String(setting.value);
  return (
    <tr>
      <td>
        <code>{setting.name}</code>
      </td>
      <td>{value}</td>
      <td>{setting.origin}</td>
    </tr>
  );
}

export function GroupSettings({ settings }: { settings: GroupSetting[] }) {
  const rows = settings.map((setting) => <SettingRow key={setting.name} setting={setting} />);
  const resolved = settings.filter((setting) => setting.status === "resolved").length;
  return (
    <details>
      <summary>
        Réglages héritables · {resolved}/{settings.length} résolus
      </summary>
      <p>
        Les valeurs héritées demandent les groupes et réglages globaux de l'installation source. Ce
        bilan ne remplace pas la vérification du parcours de collecte.
      </p>
      <table>
        <thead>
          <tr>
            <th>Réglage</th>
            <th>Valeur</th>
            <th>Origine</th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </details>
  );
}
