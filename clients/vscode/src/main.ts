import {
  workspace,
  ExtensionContext,
} from "vscode";

import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import * as vscode from "vscode";

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
  // TODO: bootstrap debug, release
  // const traceOutputChannel = window.createOutputChannel("fluent-bit language server trace");
  // const command = process.env.SERVER_PATH || "fluent-bit-language-server";

  // Use bundled server only for now
  const ext = process.platform === "win32" ? ".exe" : "";
  const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `fluent-bit-language-server${ext}`);
  const bundledExists = await fileExists(bundled);

  if (!bundledExists) {
    await vscode.window.showErrorMessage(
      "Unfortunately we don't ship binaries for your platform yet." +
      "Please build and run the server manually from the source code." +
      "Or, please create an issue on repository."
    );
    return;
  }

  const run: Executable = {
    command: bundled.fsPath,
    options: {
      env: {
        ...process.env
      },
    }
  }
  const serverOptions: ServerOptions = {
    run,
    debug: run,
  };

  let clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [
      { scheme: "file", language: "fluent-bit" },
      // { scheme: "file", pattern: "**/*.conf" },
      // { scheme: "file", language: "plaintext" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/.clientrc"),
    },
    // traceOutputChannel,
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "fluentbitLanguageServer",
    "fluent-bit language server",
    serverOptions,
    clientOptions
  );

  console.log("Running fluent-bit extension");
  await client.start();
}

async function fileExists(uri: vscode.Uri) {
  return await vscode.workspace.fs.stat(uri).then(
    () => true,
    () => false,
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
