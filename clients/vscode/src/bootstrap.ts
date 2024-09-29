import * as vscode from "vscode";
import * as os from "os";

export async function bootstrap(
  context: vscode.ExtensionContext
): Promise<string> {
  const path = await getServer(context);
  if (!path) {
    throw new Error("fluent-bit-language-server is not available.");
  }

  console.log("Using server binary at", path);

  // TODO: check validity

  return path;
}

async function getServer(
  context: vscode.ExtensionContext
): Promise<string | undefined> {
  // check if the server path is configured explicitly
  const explicitPath = process.env["__FLB_LSP_SERVER_DEBUG"];
  if (explicitPath) {
    if (explicitPath.startsWith("~/")) {
        return os.homedir() + explicitPath.slice("~".length);
    }
    return explicitPath;
  }

  const ext = process.platform === "win32" ? ".exe" : "";
  const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `fluent-bit-language-server${ext}`);
  const bundledExists = await fileExists(bundled);

  if (!bundledExists) {
    await vscode.window.showErrorMessage(
      "Unfortunately we don't ship binaries for your platform yet. " +
      "Please build and run the server manually from the source code. " +
      "Or, please create an issue on repository."
    );
    return;
  }

  return bundled.fsPath;
}

async function fileExists(uri: vscode.Uri) {
  return await vscode.workspace.fs.stat(uri).then(
    () => true,
    () => false,
  );
}

// TODO: check availability of the server binary
