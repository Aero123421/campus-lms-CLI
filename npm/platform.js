// SPDX-License-Identifier: Apache-2.0

function executableName() {
  return process.platform === "win32" ? "campus-lms.exe" : "campus-lms";
}

function packagePlatform() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    win32: "windows",
    darwin: "macos",
    linux: "linux"
  };

  const archMap = {
    x64: "x64",
    arm64: "arm64"
  };

  const mappedPlatform = platformMap[platform];
  const mappedArch = archMap[arch];

  if (!mappedPlatform || !mappedArch) {
    return null;
  }

  return `${mappedPlatform}-${mappedArch}`;
}

function artifactName(version) {
  const platform = packagePlatform();
  if (!platform) {
    return null;
  }
  const ext = process.platform === "win32" ? ".exe" : "";
  return `campus-lms-v${version}-${platform}${ext}`;
}

function checksumArtifactName(version) {
  const artifact = artifactName(version);
  return artifact ? `${artifact}.sha256` : null;
}

module.exports = {
  artifactName,
  checksumArtifactName,
  executableName,
  packagePlatform
};
