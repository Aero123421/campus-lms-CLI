#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const file = process.argv[2];

if (!file) {
  console.error("usage: node npm/sha256-file.js <file>");
  process.exit(2);
}

const hash = crypto.createHash("sha256");
const input = fs.createReadStream(file);

input.on("data", (chunk) => hash.update(chunk));
input.on("error", (error) => {
  console.error(error.message);
  process.exit(1);
});
input.on("end", () => {
  process.stdout.write(`${hash.digest("hex")}  ${path.basename(file)}\n`);
});
