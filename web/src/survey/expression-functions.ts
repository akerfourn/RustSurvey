import { compare, numeric, stringValue, truth, type Scalar } from "./scalars.ts";

interface FunctionDefinition {
  minimum: number;
  maximum: number;
  evaluate: (values: Scalar[]) => Scalar;
}
export const FUNCTIONS = new Map<string, FunctionDefinition>();
const MAX_STRING_LENGTH = 65536;

function register(
  name: string,
  minimum: number,
  maximum: number,
  evaluate: FunctionDefinition["evaluate"],
): void {
  FUNCTIONS.set(name, { minimum, maximum, evaluate });
}

const unary: Record<string, (value: number) => number> = {
  abs: Math.abs,
  acos: Math.acos,
  asin: Math.asin,
  atan: Math.atan,
  ceil: Math.ceil,
  cos: Math.cos,
  exp: Math.exp,
  floor: Math.floor,
  sin: Math.sin,
  sqrt: Math.sqrt,
  tan: Math.tan,
};
for (const [name, operation] of Object.entries(unary))
  register(name, 1, 1, (values) => operation(numeric(values[0] ?? null)));
register("atan2", 2, 2, (values) =>
  Math.atan2(numeric(values[0] ?? null), numeric(values[1] ?? null)),
);
register("pow", 2, 2, (values) => Math.pow(numeric(values[0] ?? null), numeric(values[1] ?? null)));
register("pi", 0, 0, () => Math.PI);
register("log", 1, 2, (values) => {
  const value = Math.log(numeric(values[0] ?? null));
  if (values.length === 1) return value;
  return value / Math.log(numeric(values[1] ?? null));
});
register("if", 2, 3, () => {
  throw new Error("Une condition doit être évaluée paresseusement.");
});
register("is_empty", 1, 1, (values) => values[0] === null || values[0] === "");
register("is_null", 1, 1, (values) => values[0] === null);
register("is_string", 1, 1, (values) => typeof values[0] === "string");
register("is_numeric", 1, 1, (values) => {
  const value = values[0];
  if (value === "" || value === null || typeof value === "boolean") return false;
  try {
    numeric(value ?? null);
    return true;
  } catch {
    return false;
  }
});
register(
  "count",
  0,
  512,
  (values) => values.filter((value) => value !== null && value !== "").length,
);
register("sum", 1, 512, (values) => values.reduce<number>((sum, value) => sum + numeric(value), 0));
for (const name of ["min", "max"])
  register(name, 1, 512, (values) => {
    let best = values[0] ?? null;
    for (const value of values.slice(1)) {
      const order = compare(value, best);
      if ((name === "min" && order < 0) || (name === "max" && order > 0)) best = value;
    }
    return best;
  });
register("round", 1, 2, (values) => {
  const precision = numeric(values[1] ?? 0);
  if (!Number.isInteger(precision) || Math.abs(precision) > 15)
    throw new Error("Précision d’arrondi limitée à 15 chiffres.");
  const value = numeric(values[0] ?? null);
  const scale = Math.pow(10, precision);
  return (Math.sign(value) * Math.round(Math.abs(value) * scale + Number.EPSILON)) / scale;
});
register("strlen", 1, 1, (values) => [...stringValue(values[0] ?? null)].length);
register("strtolower", 1, 1, (values) => stringValue(values[0] ?? null).toLowerCase());
register("strtoupper", 1, 1, (values) => stringValue(values[0] ?? null).toUpperCase());
for (const name of ["trim", "ltrim", "rtrim"])
  register(name, 1, 1, (values) => {
    const value = stringValue(values[0] ?? null);
    if (name === "ltrim") return value.trimStart();
    if (name === "rtrim") return value.trimEnd();
    return value.trim();
  });
register("substr", 2, 3, (values) => {
  const characters = [...stringValue(values[0] ?? null)];
  const offset = numeric(values[1] ?? null);
  let start = Math.trunc(offset);
  if (start < 0) start = Math.max(0, characters.length + start);
  if (values.length === 2) return characters.slice(start).join("");
  const length = Math.trunc(numeric(values[2] ?? null));
  if (length < 0)
    return characters.slice(start, Math.max(start, characters.length + length)).join("");
  return characters.slice(start, start + length).join("");
});
register("strrev", 1, 1, (values) => {
  const value = stringValue(values[0] ?? null);
  if ([...value].some((character) => character.charCodeAt(0) > 127))
    throw new Error("strrev est limité aux caractères ASCII dans cet aperçu.");
  return [...value].reverse().join("");
});
register("str_replace", 3, 3, (values) => {
  const search = stringValue(values[0] ?? null);
  if (search === "") return stringValue(values[2] ?? null);
  const parts = stringValue(values[2] ?? null).split(search);
  const replacement = stringValue(values[1] ?? null);
  const size =
    parts.reduce((total, part) => total + part.length, 0) + (parts.length - 1) * replacement.length;
  if (size > MAX_STRING_LENGTH) throw new Error("Texte calculé trop long.");
  return parts.join(replacement);
});
for (const name of ["strcmp", "strcasecmp"])
  register(name, 2, 2, (values) => {
    let left = stringValue(values[0] ?? null);
    let right = stringValue(values[1] ?? null);
    if (name === "strcasecmp") {
      left = left.toLowerCase();
      right = right.toLowerCase();
    }
    if (left < right) return -1;
    if (left > right) return 1;
    return 0;
  });
for (const name of ["strpos", "stripos"])
  register(name, 2, 3, (values) => {
    let haystack = stringValue(values[0] ?? null);
    let needle = stringValue(values[1] ?? null);
    if (name === "stripos") {
      haystack = haystack.toLowerCase();
      needle = needle.toLowerCase();
    }
    let offset = Math.trunc(numeric(values[2] ?? 0));
    const characters = [...haystack];
    if (offset < 0) offset = characters.length + offset;
    if (offset < 0 || offset > characters.length) return false;
    const start = characters.slice(0, offset).join("").length;
    const position = haystack.indexOf(needle, start);
    if (position < 0) return false;
    return [...haystack.slice(0, position)].length;
  });
for (const name of ["join", "implode", "list"])
  register(name, 1, 512, (values) => {
    let separator = "";
    let items = values;
    if (name === "implode") {
      separator = stringValue(values[0] ?? null);
      items = values.slice(1);
    }
    if (name === "list") {
      separator = ", ";
      items = items.filter((value) => value !== "" && value !== null);
    }
    const strings = items.map(stringValue);
    if (
      strings.reduce((sum, item) => sum + item.length, 0) +
        separator.length * Math.max(0, strings.length - 1) >
      MAX_STRING_LENGTH
    )
      throw new Error("Texte calculé trop long.");
    return strings.join(separator);
  });
register("str_repeat", 2, 2, (values) => {
  const value = stringValue(values[0] ?? null);
  const count = numeric(values[1] ?? null);
  if (
    !Number.isSafeInteger(count) ||
    count < 0 ||
    count > MAX_STRING_LENGTH ||
    value.length * count > MAX_STRING_LENGTH
  )
    throw new Error("Répétition de texte trop grande.");
  return value.repeat(count);
});
register("unique", 1, 512, (values) => {
  const answered = values.filter((value) => value !== "" && value !== null).map(stringValue);
  return new Set(answered).size === answered.length;
});

export function validateArguments(name: string, count: number): void {
  const definition = FUNCTIONS.get(name);
  if (definition === undefined || count < definition.minimum || count > definition.maximum)
    throw new Error(`Nombre d’arguments incorrect pour ${name}.`);
}

export function evaluateFunction(name: string, values: Scalar[]): Scalar {
  const definition = FUNCTIONS.get(name);
  if (definition === undefined) throw new Error(`Fonction non prise en charge : ${name}.`);
  const result = definition.evaluate(values);
  if (typeof result === "number" && !Number.isFinite(result))
    throw new Error("Résultat numérique non fini.");
  if (typeof result === "string" && result.length > MAX_STRING_LENGTH)
    throw new Error("Texte calculé trop long.");
  return result;
}

function matches(operator: string, left: Scalar, right: Scalar): boolean {
  const order = compare(left, right);
  if (["==", "eq"].includes(operator)) return order === 0;
  if (["!=", "ne"].includes(operator)) return order !== 0;
  if (["<", "lt"].includes(operator)) return order < 0;
  if (["<=", "le"].includes(operator)) return order <= 0;
  if ([">", "gt"].includes(operator)) return order > 0;
  if ([">=", "ge"].includes(operator)) return order >= 0;
  throw new Error("Comparateur de fonction non pris en charge.");
}
register(
  "countif",
  2,
  512,
  (values) => values.slice(1).filter((value) => matches("==", value, values[0] ?? null)).length,
);
for (const name of ["countifop", "sumifop"])
  register(name, 3, 512, (values) => {
    const operator = stringValue(values[0] ?? null);
    const matching = values
      .slice(2)
      .filter(
        (value) => value !== "" && value !== null && matches(operator, value, values[1] ?? null),
      );
    if (name === "countifop") return matching.length;
    return matching.reduce<number>((sum, value) => sum + numeric(value), 0);
  });
register("strstr", 2, 3, (values) => {
  const value = stringValue(values[0] ?? null);
  const position = value.indexOf(stringValue(values[1] ?? null));
  if (position < 0) return false;
  if (truth(values[2] ?? false)) return value.slice(0, position);
  return value.slice(position);
});
