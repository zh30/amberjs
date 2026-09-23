/**
 * Amber VS Code Extension - Main Entry Point
 *
 * This extension provides:
 * - Language support for JavaScript/TypeScript with Amber enhancements
 * - Debugging capabilities for Amber runtime
 * - Integration with Amber CLI tools
 */

import * as vscode from 'vscode';
import { AmberLanguageService } from './language/AmberLanguageService';
import { AmberDebugAdapterDescriptorFactory } from './debug/AmberDebugAdapter';
import { AmberCommands } from './utils/AmberCommands';
import { AmberConfiguration } from './utils/AmberConfiguration';

export function activate(context: vscode.ExtensionContext) {
    // Log activation
    vscode.window.showInformationMessage('🐝 Amber Runtime Extension activated!');

    // Initialize configuration
    const config = new AmberConfiguration();

    // Register language service
    const languageService = new AmberLanguageService(context, config);
    const languageClient = languageService.initialize();

    context.subscriptions.push(languageClient);

    // Register debug adapter
    const debugAdapterFactory = new AmberDebugAdapterDescriptorFactory(config);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('amberjs', debugAdapterFactory)
    );

    // Register commands
    const commands = new AmberCommands(config);
    context.subscriptions.push(
        vscode.commands.registerCommand('amberjs.runScript', commands.runScript),
        vscode.commands.registerCommand('amberjs.debugScript', commands.debugScript),
        vscode.commands.registerCommand('amberjs.formatDocument', commands.formatDocument),
        vscode.commands.registerCommand('amberjs.exportTypes', commands.exportTypes),
        vscode.commands.registerCommand('amberjs.deploy', commands.deploy),
        vscode.commands.registerCommand('amberjs.showPerformanceReport', commands.showPerformanceReport),
        vscode.commands.registerCommand('amberjs.installRuntime', commands.installRuntime),
        vscode.commands.registerCommand('amberjs.selectRuntime', commands.selectRuntime)
    );

    // Register configuration change handler
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('amberjs')) {
                config.reload();
                vscode.window.showInformationMessage('🐝 Amber configuration updated');
            }
        })
    );

    // Show welcome message on first activation
    const beenActivated = context.globalState.get('amberjs.activated', false);
    if (!beenActivated) {
        context.globalState.update('amberjs.activated', true);
        showWelcomeMessage();
    }
}

function showWelcomeMessage() {
    const message = 'Welcome to Amber! Install the runtime to get started.';
    const action = 'Install Amber';

    vscode.window.showInformationMessage(message, action).then((selection) => {
        if (selection === action) {
            vscode.commands.executeCommand('amberjs.installRuntime');
        }
    });
}

export function deactivate(): Thenable<void> | undefined {
    // Cleanup resources
    return undefined;
}
