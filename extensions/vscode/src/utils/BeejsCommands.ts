/**
 * Beejs Commands
 *
 * VS Code commands for Beejs integration
 */

import * as vscode from 'vscode';
import { exec } from 'child_process';
import { promisify } from 'util';
import * as path from 'path';
import { BeejsConfiguration } from './BeejsConfiguration';

const execAsync = promisify(exec);

export class BeejsCommands {
    private config: BeejsConfiguration;
    private outputChannel: vscode.OutputChannel;

    constructor(config: BeejsConfiguration) {
        this.config = config;
        this.outputChannel = vscode.window.createOutputChannel('Beejs');
    }

    public async runScript(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const filePath = editor.document.uri.fsPath;

        try {
            this.outputChannel.clear();
            this.outputChannel.appendLine(`🐝 Running Beejs script: ${filePath}`);
            this.outputChannel.show();

            const { stdout, stderr } = await execAsync(
                `${this.config.getRuntimePath()} run "${filePath}"`
            );

            if (stdout) {
                this.outputChannel.appendLine(stdout);
            }
            if (stderr) {
                this.outputChannel.appendLine(`Error: ${stderr}`);
            }

            vscode.window.showInformationMessage('✅ Script executed successfully');
        } catch (error: any) {
            this.outputChannel.appendLine(`❌ Error: ${error.message}`);
            vscode.window.showErrorMessage(`Script execution failed: ${error.message}`);
        }
    }

    public async debugScript(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const filePath = editor.document.uri.fsPath;

        try {
            await this.config.validateConfiguration();

            const debugConfig: vscode.DebugConfiguration = {
                type: 'beejs',
                request: 'launch',
                name: `Debug: ${path.basename(filePath)}`,
                program: filePath,
                runtimeExecutable: this.config.getRuntimePath(),
                runtimeArgs: [
                    'run',
                    '--inspect-brk',
                    '--inspect-port',
                    this.config.getDebugPort().toString(),
                ],
                port: this.config.getDebugPort(),
                stopOnEntry: true,
                console: 'integratedTerminal',
            };

            vscode.debug.startDebugging(vscode.workspace.workspaceFolders?.[0], debugConfig);
        } catch (error: any) {
            vscode.window.showErrorMessage(`Failed to start debug session: ${error.message}`);
        }
    }

    public async showPerformanceReport(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const filePath = editor.document.uri.fsPath;

        try {
            this.outputChannel.clear();
            this.outputChannel.appendLine('📊 Generating performance report...');
            this.outputChannel.show();

            const { stdout, stderr } = await execAsync(
                `${this.config.getRuntimePath()} profile "${filePath}"`
            );

            if (stdout) {
                this.outputChannel.appendLine(stdout);
            }
            if (stderr) {
                this.outputChannel.appendLine(`Error: ${stderr}`);
            }

            vscode.window.showInformationMessage('📊 Performance report generated');
        } catch (error: any) {
            this.outputChannel.appendLine(`❌ Error: ${error.message}`);
            vscode.window.showErrorMessage(`Performance profiling failed: ${error.message}`);
        }
    }

    public async installRuntime(): Promise<void> {
        const choice = await vscode.window.showInformationMessage(
            'Install Beejs Runtime',
            'Download from GitHub',
            'Use package manager (npm)',
            'Cancel'
        );

        if (choice === 'Download from GitHub') {
            vscode.env.openExternal(
                vscode.Uri.parse('https://github.com/beejs-team/beejs/releases')
            );
        } else if (choice === 'Use package manager (npm)') {
            const terminal = vscode.window.createTerminal('Beejs Install');
            terminal.show();
            terminal.sendText('npm install -g @beejs/runtime');
        }
    }

    public async selectRuntime(): Promise<string> {
        const paths = await vscode.window.showInputBox({
            prompt: 'Enter path to Beejs executable',
            value: this.config.getRuntimePath(),
        });

        if (paths) {
            await vscode.workspace
                .getConfiguration('beejs')
                .update('runtimePath', paths, vscode.ConfigurationTarget.Global);

            this.config.reload();
            vscode.window.showInformationMessage(`🐝 Beejs runtime path updated: ${paths}`);
        }

        return this.config.getRuntimePath();
    }

    public async formatDocument(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const filePath = editor.document.uri.fsPath;
        try {
            await execAsync(`${this.config.getRuntimePath()} fmt "${filePath}"`);
            vscode.window.showInformationMessage(`✨ Formatted with OXC: ${path.basename(filePath)}`);
        } catch (error: any) {
            vscode.window.showErrorMessage(`Format failed: ${error.message}`);
        }
    }

    public async exportTypes(): Promise<void> {
        const rootPath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();
        const typesDir = path.join(rootPath, 'types');
        const typesPath = path.join(typesDir, 'beejs.d.ts');

        try {
            await execAsync(`${this.config.getRuntimePath()} types -o "${typesPath}"`);
            vscode.window.showInformationMessage(`📘 TypeScript types exported to: ${typesPath}`);
        } catch (error: any) {
            vscode.window.showErrorMessage(`Export types failed: ${error.message}`);
        }
    }

    public async deploy(): Promise<void> {
        const rootPath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();
        const target = await vscode.window.showQuickPick(
            [
                { label: 'docker', description: 'Production multi-stage Dockerfile & docker-compose.yml' },
                { label: 'standalone', description: 'Self-contained executable binary (bee compile)' },
                { label: 'k8s', description: 'Kubernetes deployment, service & HPA manifests' }
            ],
            { placeHolder: 'Select deployment target' }
        );

        if (!target) return;

        try {
            const { stdout } = await execAsync(
                `${this.config.getRuntimePath()} deploy --target ${target.label}`,
                { cwd: rootPath }
            );
            vscode.window.showInformationMessage(`🚀 Deployment generated for: ${target.label}`);
            this.outputChannel.clear();
            this.outputChannel.appendLine(stdout);
            this.outputChannel.show();
        } catch (error: any) {
            vscode.window.showErrorMessage(`Deployment scaffolding failed: ${error.message}`);
        }
    }

    public dispose(): void {
        this.outputChannel.dispose();
    }
}
