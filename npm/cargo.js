// SPDX-License-Identifier: Apache-2.0

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const cargo = findCargo();

if (!cargo) {
  console.error("cargo was not found. Install Rust or add cargo to PATH.");
  process.exit(1);
}

const args = process.argv.slice(2);
const result = spawnSync(cargo, args, {
  cwd: root,
  stdio: "inherit",
  windowsHide: false
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);

function findCargo() {
  const cargoName = process.platform === "win32" ? "cargo.exe" : "cargo";
  const pathDirs = (process.env.PATH || "").split(path.delimiter).filter(Boolean);
  const extraDirs = [];

  if (process.env.CARGO_HOME) {
    extraDirs.push(path.join(process.env.CARGO_HOME, "bin"));
  }
  if (process.env.USERPROFILE) {
    extraDirs.push(path.join(process.env.USERPROFILE, ".cargo", "bin"));
  }
  if (process.env.HOME) {
    extraDirs.push(path.join(process.env.HOME, ".cargo", "bin"));
  }

  for (const dir of [...pathDirs, ...extraDirs]) {
    const candidate = path.join(dir, cargoName);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return null;
}
