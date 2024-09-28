import { ExtensionContext } from "vscode";
import * as vscode from "vscode";

import {
  createServerSocketTransport,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand("stork.restart-lsp", restart)
  );
  start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

export function restart(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return (async () => {
    if (client.isRunning()) {
      await client.stop();
    }
    start();
  })();
}

function start() {
  let serverOptions: ServerOptions = async () => {
    let [reader, writer] = createServerSocketTransport(50022, "utf-8");
    return { reader, writer };
  };

  let clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "stork" }],
  };

  client = new LanguageClient(
    "stork-lsp",
    "Stork Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}
