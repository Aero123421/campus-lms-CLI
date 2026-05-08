#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { executableName, packagePlatform } = require("./platform");

const root = path.resolve(__dirname, "..");
const exe = executableName();
const platform = packagePlatform();
const overrideBinary = process.env.CAMPUS_LMS_BIN
  ? path.resolve(process.env.CAMPUS_LMS_BIN)
  : null;

if (process.env.CAMPUS_LMS_BIN && !path.isAbsolute(process.env.CAMPUS_LMS_BIN)) {
  console.error("CAMPUS_LMS_BIN must be an absolute path.");
  process.exit(1);
}

const candidates = [
  overrideBinary,
  path.join(root, "npm", "bin", exe),
  platform && path.join(root, "npm", "prebuilt", platform, exe),
  path.join(root, "target", "release", exe),
  path.join(root, "target", "debug", exe)
].filter(Boolean);

const binary = candidates.find((candidate) => fs.existsSync(candidate));

if (!binary) {
  console.error(
    "campus-lms native binary was not found. Reinstall the package, or set CAMPUS_LMS_BIN to a campus-lms binary."
  );
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
