// SPDX-License-Identifier: Apache-2.0

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const https = require("node:https");
const path = require("node:path");
const crypto = require("node:crypto");
const { artifactName, checksumArtifactName, executableName, packagePlatform } = require("./platform");

if (process.env.CAMPUS_LMS_SKIP_DOWNLOAD === "1") {
  console.log("Skipping campus-lms binary install because CAMPUS_LMS_SKIP_DOWNLOAD=1.");
  process.exit(0);
}

const root = path.resolve(__dirname, "..");
const pkg = require(path.join(root, "package.json"));
const exe = executableName();
const platform = packagePlatform();
const prebuiltBinary = platform ? path.join(root, "npm", "prebuilt", platform, exe) : null;
const installedBinary = path.join(root, "npm", "bin", exe);
const releaseBinary = path.join(root, "target", "release", exe);
const MAX_DOWNLOAD_BYTES = 100 * 1024 * 1024;
const MAX_REDIRECTS = 5;

if (prebuiltBinary && fs.existsSync(prebuiltBinary)) {
  fs.mkdirSync(path.dirname(installedBinary), { recursive: true });
  fs.copyFileSync(prebuiltBinary, installedBinary);
  markExecutable(installedBinary);
  process.exit(0);
}

if (fs.existsSync(releaseBinary)) {
  fs.mkdirSync(path.dirname(installedBinary), { recursive: true });
  fs.copyFileSync(releaseBinary, installedBinary);
  markExecutable(installedBinary);
  process.exit(0);
}

installDownloadedBinary()
  .then((installed) => {
    if (installed) {
      process.exit(0);
    }
    if (process.env.CAMPUS_LMS_BUILD_FROM_SOURCE === "1") {
      buildFromSource();
      process.exit(0);
    }
    console.error(
      "No campus-lms prebuilt binary was available for this platform. " +
        "Set CAMPUS_LMS_DOWNLOAD_BASE_URL to a release directory, or set CAMPUS_LMS_BUILD_FROM_SOURCE=1 to build with Rust."
    );
    process.exit(1);
  })
  .catch((error) => {
    console.error(error.message);
    process.exit(1);
  });

async function installDownloadedBinary() {
  const name = artifactName(pkg.version);
  const checksumName = checksumArtifactName(pkg.version);
  if (!name) {
    return false;
  }
  const baseUrl = binaryBaseUrl(pkg);
  if (!baseUrl) {
    return false;
  }

  const normalizedBaseUrl = baseUrl.replace(/\/$/, "");
  const url = `${normalizedBaseUrl}/${name}`;
  const checksumUrl = `${normalizedBaseUrl}/${checksumName}`;
  const checksumTarget = `${installedBinary}.sha256`;
  fs.mkdirSync(path.dirname(installedBinary), { recursive: true });
  try {
    await download(checksumUrl, checksumTarget);
    await download(url, installedBinary);
    verifyChecksum(installedBinary, checksumTarget, name);
  } finally {
    fs.rmSync(checksumTarget, { force: true });
  }
  markExecutable(installedBinary);
  return true;
}

function binaryBaseUrl(pkg) {
  if (process.env.CAMPUS_LMS_DOWNLOAD_BASE_URL) {
    return process.env.CAMPUS_LMS_DOWNLOAD_BASE_URL;
  }
  if (pkg.campusLms && pkg.campusLms.binaryBaseUrl) {
    return pkg.campusLms.binaryBaseUrl.replace("{version}", `v${pkg.version}`);
  }
  if (pkg.repository && typeof pkg.repository.url === "string") {
    const match = pkg.repository.url.match(/github\.com[:/](.+?\/.+?)(?:\.git)?$/);
    if (match) {
      return `https://github.com/${match[1]}/releases/download/v${pkg.version}`;
    }
  }
  return null;
}

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

function buildFromSource() {
  const cargo = findCargo();
  if (!cargo) {
    console.error("cargo was not found. Install Rust or use a prebuilt campus-lms npm package.");
    process.exit(1);
  }

  const result = spawnSync(cargo, ["build", "--release"], {
    cwd: root,
    stdio: "inherit",
    windowsHide: false
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  fs.mkdirSync(path.dirname(installedBinary), { recursive: true });
  fs.copyFileSync(releaseBinary, installedBinary);
  markExecutable(installedBinary);
}

function download(url, target, redirects = 0) {
  return new Promise((resolve, reject) => {
    let parsed;
    try {
      parsed = new URL(url);
    } catch (error) {
      reject(new Error(`Invalid download URL ${url}: ${error.message}`));
      return;
    }
    if (parsed.protocol !== "https:") {
      reject(new Error(`Refusing non-HTTPS download URL: ${url}`));
      return;
    }
    let settled = false;
    const fail = (error) => {
      if (settled) {
        return;
      }
      settled = true;
      fs.rmSync(target, { force: true });
      reject(error);
    };
    const request = https.get(
      parsed,
      {
        headers: {
          "User-Agent": `campus-lms-cli/${pkg.version}`
        }
      },
      (response) => {
        if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
          response.resume();
          if (redirects >= MAX_REDIRECTS) {
            fail(new Error(`Too many redirects while downloading ${url}`));
            return;
          }
          if (!response.headers.location) {
            fail(new Error(`Redirect from ${url} did not include a Location header.`));
            return;
          }
          const nextUrl = new URL(response.headers.location, parsed).toString();
          download(nextUrl, target, redirects + 1).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          response.resume();
          fail(new Error(`Failed to download ${url}: HTTP ${response.statusCode}`));
          return;
        }
        const contentLength = Number(response.headers["content-length"] || "0");
        if (contentLength > MAX_DOWNLOAD_BYTES) {
          response.resume();
          fail(new Error(`Download is too large: ${url}`));
          return;
        }
        let received = 0;
        const file = fs.createWriteStream(target);
        response.on("data", (chunk) => {
          received += chunk.length;
          if (received > MAX_DOWNLOAD_BYTES) {
            response.destroy();
            file.destroy();
            fail(new Error(`Download exceeded ${MAX_DOWNLOAD_BYTES} bytes: ${url}`));
          }
        });
        response.pipe(file);
        file.on("finish", () =>
          file.close(() => {
            if (!settled) {
              settled = true;
              resolve();
            }
          })
        );
        file.on("error", fail);
      }
    );
    request.on("error", fail);
  });
}

function verifyChecksum(file, checksumFile, artifact) {
  const checksumText = fs.readFileSync(checksumFile, "utf8").trim();
  const expected = parseChecksum(checksumText, artifact);
  const actual = sha256(file);

  if (actual !== expected) {
    fs.rmSync(file, { force: true });
    throw new Error(
      `Checksum verification failed for ${artifact}. Expected ${expected}, got ${actual}.`
    );
  }
}

function parseChecksum(text, artifact) {
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  for (const line of lines) {
    const match = line.match(/^([a-fA-F0-9]{64})(?:\s+\*?(.+))?$/);
    if (!match) {
      continue;
    }
    const fileName = match[2] ? path.basename(match[2].trim()) : null;
    if (!fileName || fileName === artifact) {
      return match[1].toLowerCase();
    }
  }
  throw new Error(`No SHA256 checksum for ${artifact} was found.`);
}

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}

function markExecutable(file) {
  if (process.platform !== "win32") {
    fs.chmodSync(file, 0o755);
  }
}
