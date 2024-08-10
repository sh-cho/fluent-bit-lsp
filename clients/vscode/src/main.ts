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

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
  // const traceOutputChannel = window.createOutputChannel("fluent-bit language server trace");
  const command = process.env.SERVER_PATH || "fluent-bit-language-server";
  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
        RUST_LOG: "debug",
      },
    },
  };
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

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
