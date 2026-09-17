/// <reference types="node" />
/// <reference types="vscode" />
/**
 * Amber Language Service
 *
 * Starts `amber lsp` via vscode-languageclient and adds a few Amber completions.
 */

import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';
import { AmberConfiguration } from '../utils/AmberConfiguration';

export class AmberLanguageService {
    private client: LanguageClient | undefined;
    private readonly diagnostics: vscode.DiagnosticCollection;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly config: AmberConfiguration
    ) {
        this.diagnostics = vscode.languages.createDiagnosticCollection('amberjs');
        this.context.subscriptions.push(this.diagnostics);
    }

    public initialize(): LanguageClient {
        const beePath = this.config.getRuntimePath() || 'amber';
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
                { scheme: 'file', language: 'amberjs' },
            ],
            initializationOptions: {
                amberjsPath: this.config.getRuntimePath(),
                enableTypeChecking: this.config.getEnableTypeChecking(),
                maxMemory: this.config.getMaxMemory(),
            },
            synchronize: {
                configurationSection: 'amberjs',
            },
        };

        this.client = new LanguageClient(
            'amberjs-language-server',
            'Amber Language Server',
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
                ['javascript', 'typescript', 'amberjs'],
                {
                    provideCompletionItems: (document: vscode.TextDocument) => {
                        const completions: vscode.CompletionItem[] = [];
                        const run = new vscode.CompletionItem('amberjs.run', vscode.CompletionItemKind.Function);
                        run.detail = 'Execute a Amber script';
                        run.insertText = new vscode.SnippetString('amberjs.run(${1:script})');
                        completions.push(run);

                        const test = new vscode.CompletionItem('amberjs.test', vscode.CompletionItemKind.Function);
                        test.detail = 'Run tests with Amber';
                        test.insertText = new vscode.SnippetString('amberjs.test(${1:pattern})');
                        completions.push(test);

                        if (document.languageId === 'typescript') {
                            const compile = new vscode.CompletionItem(
                                'amberjs.compile',
                                vscode.CompletionItemKind.Function
                            );
                            compile.detail = 'Compile TypeScript with Amber';
                            completions.push(compile);
                        }
                        return completions;
                    },
                },
                '.'
            ),
            vscode.languages.registerHoverProvider(['javascript', 'typescript', 'amberjs'], {
                provideHover: (document: vscode.TextDocument, position: vscode.Position) => {
                    const range = document.getWordRangeAtPosition(position);
                    const word = range ? document.getText(range) : '';
                    if (word === 'amberjs' || word.startsWith('amberjs')) {
                        return new vscode.Hover(
                            new vscode.MarkdownString('**Amber Runtime** — `amber run` / `amber lsp` / `amber run --inspect-brk`')
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
