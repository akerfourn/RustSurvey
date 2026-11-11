export type Scalar = string | number | boolean | null;

export function truth(value: Scalar): boolean {
  if (value === null) return false;
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return Number.isFinite(value) && value !== 0;
  return value !== "" && value !== "0";
}

export function numeric(value: Scalar): number {
  if (value === null || value === "") return 0;
  if (typeof value === "boolean") return Number(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("Résultat numérique non fini.");
    return value;
  }
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value.trim())) {
    throw new Error("Une règle attend un nombre.");
  }
  const result = Number(value);
  if (!Number.isFinite(result)) throw new Error("Résultat numérique non fini.");
  return result;
}

export function stringValue(value: Scalar): string {
  if (value === null || value === false) return "";
  if (value === true) return "1";
  return String(value);
}

export function compare(left: Scalar, right: Scalar): number {
  try {
    const a = numeric(left);
    const b = numeric(right);
    if (a < b) return -1;
    if (a > b) return 1;
    return 0;
  } catch {
    const a = stringValue(left);
    const b = stringValue(right);
    if (a < b) return -1;
    if (a > b) return 1;
    return 0;
  }
}
