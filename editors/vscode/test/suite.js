// Opens a Markdown document and waits for emdysi's findings: the spelling
// mistake at once, then the agreement error once the sentence is parsed;
// then a quick fix replaces the misspelled word.

const assert = require("assert");
const path = require("path");
const vscode = require("vscode");

function waitFor(predicate, ms) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const timer = setInterval(() => {
      const value = predicate();
      if (value) {
        clearInterval(timer);
        resolve(value);
      } else if (Date.now() - started > ms) {
        clearInterval(timer);
        reject(new Error("timed out"));
      }
    }, 200);
  });
}

async function run() {
  const folder = vscode.workspace.workspaceFolders[0].uri.fsPath;
  const uri = vscode.Uri.file(path.join(folder, "draft.md"));
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc);
  const ours = () =>
    vscode.languages.getDiagnostics(uri).filter((d) => d.source === "emdysi");
  const codes = () => ours().map((d) => String(d.code));
  // The grammar loads and the sentences are parsed: up to two minutes.
  await waitFor(() => codes().includes("core.grammar-errors"), 120000);
  assert.ok(codes().includes("core.spelling"), codes().join(", "));
  const spelling = ours().find((d) => d.code === "core.spelling");
  assert.strictEqual(doc.getText(spelling.range), "recieved");
  // The quick fix replaces it.
  const actions = await vscode.commands.executeCommand(
    "vscode.executeCodeActionProvider",
    uri,
    spelling.range,
  );
  const fix = actions.find((a) => a.title.includes("received"));
  assert.ok(fix, actions.map((a) => a.title).join(", "));
  await vscode.workspace.applyEdit(fix.edit);
  assert.ok(doc.getText().includes("We received the files."), doc.getText());
  console.log(`emdysi: passed (${codes().join(", ")})`);
}

module.exports = { run };
