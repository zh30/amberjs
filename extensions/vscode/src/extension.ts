/**
 * Beejs VS Code Extension - Main Entry Point
 *
 * This extension provides:
 * - Language support for JavaScript/TypeScript with Beejs enhancements
 * - Debugging capabilities for Beejs runtime
 * - Integration with Beejs CLI tools
 */

import * as vscode from 'vscode';
import { BeejsLanguageService } from './language/BeejsLanguageService';
import { BeejsDebugAdapterDescriptorFactory } from './debug/BeejsDebugAdapter';
import { BeejsCommands } from './utils/BeejsCommands';
import { BeejsConfiguration } from './utils/BeejsConfiguration';

export function activate(context: vscode.ExtensionContext) {
    // Log activation
    vscode.window.showInformationMessage('🐝 Beejs Runtime Extension activated!');

    // Initialize configuration
    const config = new BeejsConfiguration();

    // Register language service
    const languageService = new BeejsLanguageService(context, config);
    const languageClient = languageService.initialize();

    context.subscriptions.push(languageClient);

    // Register debug adapter
    const debugAdapterFactory = new BeejsDebugAdapterDescriptorFactory(config);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('beejs', debugAdapterFactory)
    );

    // Register commands
    const commands = new BeejsCommands(config);
    context.subscriptions.push(
        vscode.commands.registerCommand('beejs.runScript', commands.runScript),
        vscode.commands.registerCommand('beejs.debugScript', commands.debugScript),
        vscode.commands.registerCommand('beejs.formatDocument', commands.formatDocument),
        vscode.commands.registerCommand('beejs.exportTypes', commands.exportTypes),
        vscode.commands.registerCommand('beejs.deploy', commands.deploy),
        vscode.commands.registerCommand('beejs.showPerformanceReport', commands.showPerformanceReport),
        vscode.commands.registerCommand('beejs.installRuntime', commands.installRuntime),
        vscode.commands.registerCommand('beejs.selectRuntime', commands.selectRuntime)
    );

    // Register configuration change handler
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('beejs')) {
                config.reload();
                vscode.window.showInformationMessage('🐝 Beejs configuration updated');
            }
        })
    );

    // Show welcome message on first activation
    const beenActivated = context.globalState.get('beejs.activated', false);
    if (!beenActivated) {
        context.globalState.update('beejs.activated', true);
        showWelcomeMessage();
    }
}

function showWelcomeMessage() {
    const message = 'Welcome to Beejs! Install the runtime to get started.';
    const action = 'Install Beejs';

    vscode.window.showInformationMessage(message, action).then((selection) => {
        if (selection === action) {
            vscode.commands.executeCommand('beejs.installRuntime');
        }
    });
}

export function deactivate(): Thenable<void> | undefined {
    // Cleanup resources
    return undefined;
}
