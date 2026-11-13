import { homedir } from "node:os";
import { posix, win32 } from "node:path";

/**
 * Preserve PATH priority, then expose the standard rustup binaries to child processes.
 * @param {NodeJS.ProcessEnv} environment
 * @param {{ platform?: NodeJS.Platform, homeDirectory?: string }} [options]
 * @returns {NodeJS.ProcessEnv}
 */
export function cargoEnvironment(
  environment,
  { platform = process.platform, homeDirectory = homedir() } = {},
) {
  let paths = posix;
  if (platform === "win32") paths = win32;

  const result = { ...environment };
  let inheritedPath = environment.PATH;
  if (platform === "win32") {
    for (const key of Object.keys(environment)) {
      if (key.toLowerCase() !== "path") continue;
      inheritedPath ??= environment[key];
      delete result[key];
    }
  }

  const directories = [];
  if (inheritedPath !== undefined && inheritedPath !== "") directories.push(inheritedPath);
  if (environment.CARGO_HOME !== undefined && environment.CARGO_HOME !== "")
    directories.push(paths.join(environment.CARGO_HOME, "bin"));
  directories.push(paths.join(homeDirectory, ".cargo", "bin"));
  result.PATH = directories.join(paths.delimiter);
  return result;
}
