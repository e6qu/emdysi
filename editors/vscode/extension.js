// The emdysi extension: starts `en lsp` for Markdown and plain-text
// documents. Findings appear as diagnostics, fixes as quick fixes.

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function start() {
  const config = vscode.workspace.getConfiguration("emdysi");
  const args = ["lsp", ...config.get("args", [])];
  const min = config.get("minSeverity", "suggestion");
  if (min !== "suggestion") {
    args.push("--min-severity", min);
  }
  const command = config.get("path", "en");
  client = new LanguageClient(
    "emdysi",
    "emdysi",
    { command, args },
    {
      documentSelector: [
        { scheme: "file", language: "markdown" },
        { scheme: "file", language: "plaintext" },
        { scheme: "untitled", language: "markdown" },
        { scheme: "untitled", language: "plaintext" },
      ],
    },
  );
  return client.start().catch((e) => {
    vscode.window.showErrorMessage(
      `emdysi: could not start '${command} lsp' (${e.message}). Install en ` +
        "(https://github.com/e6qu/emdysi#install) or set emdysi.path.",
    );
  });
}

function activate(context) {
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration("emdysi") && client) {
        await client.stop();
        await start();
      }
    }),
  );
  return start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
