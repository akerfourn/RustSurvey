export type Scalar = string | number | boolean | null;
export type Lookup = (name: string) => Scalar;
export interface Expression {
  variables: ReadonlySet<string>;
  evaluate: (lookup: Lookup) => Scalar;
}

interface Token {
  kind: "number" | "string" | "name" | "operator" | "symbol" | "end";
  text: string;
}
type Node =
  | { kind: "literal"; value: Scalar }
  | { kind: "variable"; name: string }
  | { kind: "unary"; operator: string; child: Node }
  | { kind: "binary"; operator: string; left: Node; right: Node }
  | { kind: "call"; name: string; arguments: Node[] };

const FUNCTIONS = new Set([
  "if",
  "is_empty",
  "sum",
  "count",
  "min",
  "max",
  "strlen",
  "abs",
  "round",
]);
const PRECEDENCE = new Map([
  ["or", 1],
  ["||", 1],
  ["and", 2],
  ["&&", 2],
  ["==", 3],
  ["!=", 3],
  ["<", 3],
  ["<=", 3],
  [">", 3],
  [">=", 3],
  ["+", 4],
  ["-", 4],
  ["*", 5],
  ["/", 5],
  ["%", 5],
]);

function tokenize(source: string): Token[] {
  if (source.length > 4096) throw new Error("Expression trop longue.");
  const tokens: Token[] = [];
  let position = 0;
  while (position < source.length) {
    const character = source[position];
    if (character === undefined) break;
    if (/\s/.test(character)) {
      position += 1;
      continue;
    }
    const remaining = source.slice(position);
    if (character === "'" || character === '"') {
      const quoted = readString(source, position, character);
      tokens.push({ kind: "string", text: quoted.value });
      position = quoted.end;
    } else {
      const number = /^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/.exec(remaining)?.[0];
      const name = /^[A-Za-z_][A-Za-z_0-9]*(?:\.[A-Za-z_][A-Za-z_0-9]*)*/.exec(remaining)?.[0];
      const operator = /^(?:==|!=|<=|>=|&&|\|\||[+\-*/%<>!])/.exec(remaining)?.[0];
      if (number !== undefined) {
        tokens.push({ kind: "number", text: number });
        position += number.length;
      } else if (name !== undefined) {
        const lower = name.toLowerCase();
        let kind: Token["kind"] = "name";
        if (["and", "or", "not"].includes(lower)) kind = "operator";
        let text = name;
        if (kind === "operator") text = lower;
        tokens.push({ kind, text });
        position += name.length;
      } else if (operator !== undefined) {
        tokens.push({ kind: "operator", text: operator });
        position += operator.length;
      } else if (["(", ")", ","].includes(character)) {
        tokens.push({ kind: "symbol", text: character });
        position += 1;
      } else throw new Error(`Syntaxe d’expression non prise en charge à la position ${position}.`);
    }
    if (tokens.length > 512) throw new Error("Expression trop complexe.");
  }
  tokens.push({ kind: "end", text: "" });
  return tokens;
}

function readString(source: string, start: number, quote: string): { value: string; end: number } {
  let value = "";
  let position = start + 1;
  const escapes = new Map([
    ["n", "\n"],
    ["r", "\r"],
    ["t", "\t"],
    ["\\", "\\"],
    ["'", "'"],
    ['"', '"'],
  ]);
  while (position < source.length) {
    const character = source[position];
    position += 1;
    if (character === quote) return { value, end: position };
    if (character === "\\") {
      const escaped = escapes.get(source[position] ?? "");
      if (escaped === undefined) throw new Error("Échappement de chaîne non pris en charge.");
      value += escaped;
      position += 1;
    } else value += character;
  }
  throw new Error("Chaîne d’expression non terminée.");
}

class Parser {
  private readonly tokens: Token[];
  private position = 0;
  readonly variables = new Set<string>();

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }
  private peek(): Token {
    return this.tokens[this.position] ?? { kind: "end", text: "" };
  }
  private take(): Token {
    const token = this.peek();
    this.position += 1;
    return token;
  }

  parse(): Node {
    const node = this.binary(0, 0);
    if (this.peek().kind !== "end") throw new Error("Fin d’expression inattendue.");
    return node;
  }

  private binary(minimum: number, depth: number): Node {
    if (depth > 32) throw new Error("Expression trop imbriquée.");
    let left = this.atom(depth + 1);
    while (true) {
      const operator = this.peek().text;
      const precedence = PRECEDENCE.get(operator);
      if (precedence === undefined || precedence < minimum) break;
      this.take();
      const right = this.binary(precedence + 1, depth + 1);
      left = { kind: "binary", operator, left, right };
    }
    return left;
  }

  private atom(depth: number): Node {
    if (depth > 32) throw new Error("Expression trop imbriquée.");
    const token = this.take();
    if (["!", "not", "-", "+"].includes(token.text) && token.kind === "operator") {
      return { kind: "unary", operator: token.text, child: this.atom(depth + 1) };
    }
    if (token.kind === "number") {
      const value = Number(token.text);
      if (!Number.isFinite(value)) throw new Error("Nombre non fini dans l’expression.");
      return { kind: "literal", value };
    }
    if (token.kind === "string") return { kind: "literal", value: token.text };
    if (token.text === "(") {
      const node = this.binary(0, depth + 1);
      if (this.take().text !== ")") throw new Error("Parenthèse fermante manquante.");
      return node;
    }
    if (token.kind !== "name") throw new Error("Valeur attendue dans l’expression.");
    const lower = token.text.toLowerCase();
    if (lower === "true") return { kind: "literal", value: true };
    if (lower === "false") return { kind: "literal", value: false };
    if (lower === "null") return { kind: "literal", value: null };
    if (this.peek().text === "(") return this.call(lower, depth + 1);
    this.variables.add(token.text);
    return { kind: "variable", name: token.text };
  }

  private call(name: string, depth: number): Node {
    if (!FUNCTIONS.has(name)) throw new Error(`Fonction non prise en charge : ${name}.`);
    this.take();
    const arguments_: Node[] = [];
    if (this.peek().text !== ")") {
      while (true) {
        arguments_.push(this.binary(0, depth + 1));
        if (this.peek().text !== ",") break;
        this.take();
      }
    }
    if (this.take().text !== ")") throw new Error("Parenthèse de fonction manquante.");
    if (
      (name === "if" && arguments_.length !== 3) ||
      (["is_empty", "strlen", "abs", "round"].includes(name) && arguments_.length !== 1) ||
      (["min", "max"].includes(name) && arguments_.length === 0)
    ) {
      throw new Error(`Nombre d’arguments incorrect pour ${name}.`);
    }
    return { kind: "call", name, arguments: arguments_ };
  }
}

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

function equal(left: Scalar, right: Scalar): boolean {
  if (typeof left === "boolean" || typeof right === "boolean") return truth(left) === truth(right);
  if (left === right) return true;
  if (
    (typeof left === "number" && typeof right === "string" && right.trim() !== "") ||
    (typeof right === "number" && typeof left === "string" && left.trim() !== "")
  ) {
    try {
      return numeric(left) === numeric(right);
    } catch {
      return false;
    }
  }
  return false;
}

function binary(operator: string, left: Scalar, right: Scalar): Scalar {
  if (operator === "==") return equal(left, right);
  if (operator === "!=") return !equal(left, right);
  if (["<", "<=", ">", ">="].includes(operator)) {
    const a = numeric(left);
    const b = numeric(right);
    if (operator === "<") return a < b;
    if (operator === "<=") return a <= b;
    if (operator === ">") return a > b;
    return a >= b;
  }
  const a = numeric(left);
  const b = numeric(right);
  if (operator === "+") return a + b;
  if (operator === "-") return a - b;
  if (operator === "*") return a * b;
  if (b === 0) throw new Error("Division par zéro dans une règle.");
  if (operator === "/") return a / b;
  return a % b;
}

function evaluate(node: Node, lookup: Lookup): Scalar {
  if (node.kind === "literal") return node.value;
  if (node.kind === "variable") return lookup(node.name);
  if (node.kind === "unary") {
    const value = evaluate(node.child, lookup);
    if (["!", "not"].includes(node.operator)) return !truth(value);
    if (node.operator === "-") return -numeric(value);
    return numeric(value);
  }
  if (node.kind === "binary") {
    const left = evaluate(node.left, lookup);
    if (["and", "&&"].includes(node.operator))
      return truth(left) && truth(evaluate(node.right, lookup));
    if (["or", "||"].includes(node.operator))
      return truth(left) || truth(evaluate(node.right, lookup));
    return binary(node.operator, left, evaluate(node.right, lookup));
  }
  if (node.name === "if") {
    const [condition, yes, no] = node.arguments;
    if (condition === undefined || yes === undefined || no === undefined)
      throw new Error("Condition invalide.");
    if (truth(evaluate(condition, lookup))) return evaluate(yes, lookup);
    return evaluate(no, lookup);
  }
  const values = node.arguments.map((argument) => evaluate(argument, lookup));
  const first = values[0] ?? null;
  if (node.name === "is_empty") return first === null || first === "";
  if (node.name === "strlen") return [...String(first ?? "")].length;
  if (node.name === "count") return values.filter((value) => value !== null && value !== "").length;
  const numbers = values.map(numeric);
  if (node.name === "sum") return numbers.reduce((sum, value) => sum + value, 0);
  if (node.name === "min") return Math.min(...numbers);
  if (node.name === "max") return Math.max(...numbers);
  if (node.name === "abs") return Math.abs(numeric(first));
  const number = numeric(first);
  return Math.sign(number) * Math.round(Math.abs(number));
}

export function compileExpression(source: string): Expression {
  const parser = new Parser(tokenize(source.trim()));
  const node = parser.parse();
  return {
    variables: parser.variables,
    evaluate(lookup: Lookup): Scalar {
      const value = evaluate(node, lookup);
      if (typeof value === "number" && !Number.isFinite(value))
        throw new Error("Résultat numérique non fini.");
      return value;
    },
  };
}
