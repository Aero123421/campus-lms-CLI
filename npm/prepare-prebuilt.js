// SPDX-License-Identifier: Apache-2.0

const fs = require("node:fs");
const path = require("node:path");
const { executableName, packagePlatform } = require("./platform");

const root = path.resolve(__dirname, "..");
const platform = packagePlatform();

if (!platform) {
  console.error(`Unsupported platform for prebuilt packaging: ${process.platform}/${process.arch}`);
  process.exit(1);
}

const source = path.join(root, "target", "release", executableName());
if (!fs.existsSync(source)) {
  console.error(`Release binary not found at ${source}. Run npm run build:native first.`);
  process.exit(1);
}

const targetDir = path.join(root, "npm", "prebuilt", platform);
fs.mkdirSync(targetDir, { recursive: true });

const target = path.join(targetDir, executableName());
fs.copyFileSync(source, target);
if (process.platform !== "win32") {
  fs.chmodSync(target, 0o755);
}

console.log(`Prepared prebuilt binary: ${target}`);
