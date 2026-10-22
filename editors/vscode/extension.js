// The emdysi extension: starts `en lsp` for Markdown and plain-text
// documents. Findings appear as diagnostics, fixes as quick fixes.

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;
// One output channel for every server started, so restarts do not add
// more: the server's messages and errors go there.
let output;

// Stops the running server, if any (one that failed to start cannot be
// stopped, and need not be).
async function stop() {
  if (client) {
    await client.stop().catch(() => {});
    client = undefined;
  }
}

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
      outputChannel: output,
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
  output = vscode.window.createOutputChannel("emdysi");
  context.subscriptions.push(output);
  context.subscriptions.push(
    vscode.commands.registerCommand("emdysi.restart", async () => {
      await stop();
      await start();
    }),
    vscode.commands.registerCommand("emdysi.showOutput", () => output.show(true)),
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration("emdysi")) {
        await stop();
        await start();
      }
    }),
  );
  return start();
}

function deactivate() {
  return stop();
}

module.exports = { activate, deactivate };
