// SPDX-License-Identifier: Apache-2.0

const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
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
fs.writeFileSync(`${target}.sha256`, `${sha256(target)}  ${path.basename(target)}\n`);

console.log(`Prepared prebuilt binary: ${target}`);

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}
