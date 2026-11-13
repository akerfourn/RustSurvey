import { win32 } from "node:path";

function output(result, { preferStderr = false } = {}) {
  if (result.error !== undefined || result.status !== 0) return null;
  const streams = [result.stdout ?? "", result.stderr ?? ""];
  if (preferStderr) streams.reverse();
  const text = streams.find((stream) => stream.trim() !== "")?.trim() ?? "";
  if (text === "") return null;
  return text.split(/\r?\n/)[0];
}

function windowsCompiler(run, environment, arch) {
  const direct = output(run("cl.exe", ["/help"]), { preferStderr: true });
  if (direct !== null) return direct;
  const programFiles = environment["ProgramFiles(x86)"] ?? environment.ProgramFiles;
  if (programFiles === undefined) return null;
  const locator = win32.join(programFiles, "Microsoft Visual Studio", "Installer", "vswhere.exe");
  let host = "Hostx64";
  let target = "x64";
  if (arch === "arm64") {
    host = "Hostarm64";
    target = "arm64";
  }
  const found = run(locator, [
    "-latest",
    "-products",
    "*",
    "-requires",
    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    "-find",
    `VC\\Tools\\MSVC\\*\\bin\\${host}\\${target}\\cl.exe`,
  ]);
  if (found.error !== undefined || found.status !== 0) return null;
  const path = found.stdout?.split(/\r?\n/).find((line) => line.trim() !== "");
  if (path === undefined) return null;
  return output(run(path.trim(), ["/help"]), { preferStderr: true });
}

/** Probe executable availability; compilation is verified separately by Cargo.
 * @param {{ platform: NodeJS.Platform, arch: string, environment: NodeJS.ProcessEnv,
 * run: (command: string, args: string[]) => {status: number | null, error?: Error, stdout?: string, stderr?: string} }} options
 */
export function probeTools({ platform, arch, environment, run }) {
  const checks = [
    { label: "Node.js", command: process.execPath, args: ["--version"], required: true },
    { label: "npm", command: "npm", args: ["--version"], required: true },
    { label: "Rust", command: "rustc", args: ["--version"], required: true },
    { label: "Cargo", command: "cargo", args: ["--version"], required: true },
    { label: "rustfmt", command: "cargo", args: ["fmt", "--version"], required: true },
    { label: "Clippy", command: "cargo", args: ["clippy", "--version"], required: true },
    { label: "Compilateur C", command: "cc", args: ["--version"], required: true },
    { label: "Docker", command: "docker", args: ["--version"], required: false },
    { label: "Docker Compose", command: "docker", args: ["compose", "version"], required: false },
  ];
  return checks.map((check) => {
    let result;
    if (platform === "win32" && check.command === "npm")
      result = output(
        run(environment.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm.cmd --version"]),
      );
    else if (platform === "win32" && check.command === "cc")
      result = windowsCompiler(run, environment, arch);
    else result = output(run(check.command, check.args));
    return { label: check.label, required: check.required, output: result };
  });
}
