// Downloads VS Code, installs this extension in it, and runs test/suite.js
// against a real `en`: EMDYSI_EN names the executable (default: the one
// built in this repository).

const path = require("path");
const fs = require("fs");
const os = require("os");
const { runTests } = require("@vscode/test-electron");

async function main() {
  const en =
    process.env.EMDYSI_EN ||
    path.resolve(__dirname, "../../../target/release/en" + (process.platform === "win32" ? ".exe" : ""));
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "emdysi-vscode-"));
  fs.mkdirSync(path.join(workspace, ".vscode"));
  fs.writeFileSync(
    path.join(workspace, ".vscode", "settings.json"),
    JSON.stringify({ "emdysi.path": en }),
  );
  fs.writeFileSync(
    path.join(workspace, "draft.md"),
    "# Draft\n\nWe recieved the files. The new version are faster.\n",
  );
  await runTests({
    extensionDevelopmentPath: path.resolve(__dirname, ".."),
    extensionTestsPath: path.resolve(__dirname, "suite.js"),
    launchArgs: [workspace, "--disable-extensions", "--disable-workspace-trust"],
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
