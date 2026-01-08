import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const config = vscode.workspace.getConfiguration('topos');

    // Get server path from configuration
    const serverPath = config.get<string>('server.path', 'topos');
    const serverArgs = config.get<string[]>('server.args', ['lsp']);

    // Server options - spawn the topos lsp process
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: serverArgs,
            transport: TransportKind.stdio,
        },
        debug: {
            command: serverPath,
            args: serverArgs,
            transport: TransportKind.stdio,
        },
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'topos' },
            { scheme: 'untitled', language: 'topos' },
        ],
        synchronize: {
            // Watch for .tps and .topos files
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{tps,topos}'),
        },
        outputChannelName: 'Topos Language Server',
        traceOutputChannel: vscode.window.createOutputChannel('Topos LSP Trace'),
    };

    // Create the language client
    client = new LanguageClient(
        'topos',
        'Topos Language Server',
        serverOptions,
        clientOptions
    );

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('topos.restartServer', async () => {
            if (client) {
                await client.stop();
                await client.start();
                vscode.window.showInformationMessage('Topos language server restarted');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('topos.showTraceReport', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'topos') {
                vscode.window.showWarningMessage('Open a Topos file first');
                return;
            }

            const terminal = vscode.window.createTerminal('Topos Trace');
            terminal.sendText(`topos trace "${editor.document.fileName}"`);
            terminal.show();
        })
    );

    // Start the client
    try {
        await client.start();
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        vscode.window.showErrorMessage(
            `Failed to start Topos language server: ${message}. ` +
            `Make sure 'topos' is installed and in your PATH.`
        );
    }
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}
