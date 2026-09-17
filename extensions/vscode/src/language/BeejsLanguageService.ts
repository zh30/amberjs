/**
 * Beejs Language Service
 *
 * Starts `bee lsp` via vscode-languageclient and adds a few Beejs completions.
 */

import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';
import { BeejsConfiguration } from '../utils/BeejsConfiguration';

export class BeejsLanguageService {
    private client: LanguageClient | undefined;
    private readonly diagnostics: vscode.DiagnosticCollection;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly config: BeejsConfiguration
    ) {
        this.diagnostics = vscode.languages.createDiagnosticCollection('beejs');
        this.context.subscriptions.push(this.diagnostics);
    }

    public initialize(): LanguageClient {
        const beePath = this.config.getRuntimePath() || 'bee';
        const serverExecutable = {
            command: beePath,
            args: ['lsp'],
        };

        const serverOptions: ServerOptions = {
            run: serverExecutable,
            debug: serverExecutable,
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'javascript' },
                { scheme: 'file', language: 'typescript' },
                { scheme: 'file', language: 'beejs' },
            ],
            initializationOptions: {
                beejsPath: this.config.getRuntimePath(),
                enableTypeChecking: this.config.getEnableTypeChecking(),
                maxMemory: this.config.getMaxMemory(),
            },
            synchronize: {
                configurationSection: 'beejs',
            },
        };

        this.client = new LanguageClient(
            'beejs-language-server',
            'Beejs Language Server',
            serverOptions,
            clientOptions
        );

        void this.client.start();
        this.registerProviders();
        return this.client;
    }

    private registerProviders(): void {
        this.context.subscriptions.push(
            vscode.languages.registerCompletionItemProvider(
                ['javascript', 'typescript', 'beejs'],
                {
                    provideCompletionItems: (document: vscode.TextDocument) => {
                        const completions: vscode.CompletionItem[] = [];
                        const run = new vscode.CompletionItem('beejs.run', vscode.CompletionItemKind.Function);
                        run.detail = 'Execute a Beejs script';
                        run.insertText = new vscode.SnippetString('beejs.run(${1:script})');
                        completions.push(run);

                        const test = new vscode.CompletionItem('beejs.test', vscode.CompletionItemKind.Function);
                        test.detail = 'Run tests with Beejs';
                        test.insertText = new vscode.SnippetString('beejs.test(${1:pattern})');
                        completions.push(test);

                        if (document.languageId === 'typescript') {
                            const compile = new vscode.CompletionItem(
                                'beejs.compile',
                                vscode.CompletionItemKind.Function
                            );
                            compile.detail = 'Compile TypeScript with Beejs';
                            completions.push(compile);
                        }
                        return completions;
                    },
                },
                '.'
            ),
            vscode.languages.registerHoverProvider(['javascript', 'typescript', 'beejs'], {
                provideHover: (document: vscode.TextDocument, position: vscode.Position) => {
                    const range = document.getWordRangeAtPosition(position);
                    const word = range ? document.getText(range) : '';
                    if (word === 'beejs' || word.startsWith('beejs')) {
                        return new vscode.Hover(
                            new vscode.MarkdownString('**Beejs Runtime** — `bee run` / `bee lsp` / `bee run --inspect-brk`')
                        );
                    }
                    return undefined;
                },
            })
        );
    }

    public dispose(): Thenable<void> | undefined {
        return this.client?.stop();
    }
}
