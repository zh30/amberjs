/// <reference types="node" />
/// <reference types="vscode" />
/**
 * Amber Configuration Manager
 *
 * Manages VS Code settings for Amber runtime integration
 */

import * as vscode from 'vscode';

export class AmberConfiguration {
    private runtimePath: string = 'amber';
    private debugPort: number = 9229;
    private enableTypeChecking: boolean = true;
    private maxMemory: string = '512m';
    private enableExperimentalFeatures: boolean = false;

    constructor() {
        this.reload();
    }

    public reload(): void {
        const config = vscode.workspace.getConfiguration('amberjs');

        this.runtimePath = config.get('runtimePath', 'amber');
        this.debugPort = config.get('debugPort', 9229);
        this.enableTypeChecking = config.get('enableTypeChecking', true);
        this.maxMemory = config.get('maxMemory', '512m');
        this.enableExperimentalFeatures = config.get('enableExperimentalFeatures', false);
    }

    public getRuntimePath(): string {
        return this.runtimePath;
    }

    public getDebugPort(): number {
        return this.debugPort;
    }

    public getEnableTypeChecking(): boolean {
        return this.enableTypeChecking;
    }

    public getMaxMemory(): string {
        return this.maxMemory;
    }

    public getEnableExperimentalFeatures(): boolean {
        return this.enableExperimentalFeatures;
    }

    public async validateConfiguration(): Promise<boolean> {
        try {
            // Check if Amber executable exists
            const fs = require('fs');
            const path = require('path');

            if (this.runtimePath !== 'amber') {
                // Check absolute path
                if (!fs.existsSync(this.runtimePath)) {
                    vscode.window.showErrorMessage(
                        `Amber runtime not found at: ${this.runtimePath}`
                    );
                    return false;
                }
            }

            // Validate memory setting
            const memoryRegex = /^\d+(\.\d+)?(m|M|g|G)?$/;
            if (!memoryRegex.test(this.maxMemory)) {
                vscode.window.showErrorMessage(
                    `Invalid memory format: ${this.maxMemory}. Use format like '512m' or '1g'`
                );
                return false;
            }

            // Validate port range
            if (this.debugPort < 1024 || this.debugPort > 65535) {
                vscode.window.showErrorMessage(
                    `Debug port out of range: ${this.debugPort}. Use port 1024-65535`
                );
                return false;
            }

            return true;
        } catch (error) {
            vscode.window.showErrorMessage(`Configuration validation failed: ${error}`);
            return false;
        }
    }
}
