/**
 * Beejs Debug Adapter
 *
 * Launches `bee run --inspect-brk --inspect-port <port>` and speaks DAP
 * to VS Code. Chrome DevTools can also attach to the same CDP port.
 */

import * as vscode from 'vscode';
import { spawn, ChildProcess } from 'child_process';
import { Buffer } from 'buffer';
import { BeejsConfiguration } from '../utils/BeejsConfiguration';

interface DapRequest {
    type: 'request';
    seq: number;
    command: string;
    arguments?: {
        program?: string;
        port?: number;
    };
}

export class BeejsDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    constructor(private readonly config: BeejsConfiguration) {}

    public createDebugAdapterDescriptor(
        _session: vscode.DebugSession,
        _executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        return new vscode.DebugAdapterInlineImplementation(new BeejsDebugAdapter(this.config));
    }
}

class BeejsDebugAdapter implements vscode.DebugAdapter {
    private readonly sendEmitter = new vscode.EventEmitter<vscode.DebugProtocolMessage>();
    readonly onDidSendMessage: vscode.Event<vscode.DebugProtocolMessage> = this.sendEmitter.event;
    private child: ChildProcess | undefined;
    private seq = 0;
    private readonly outputChannel: vscode.OutputChannel;

    constructor(private readonly config: BeejsConfiguration) {
        this.outputChannel = vscode.window.createOutputChannel('Beejs Debug');
    }

    public handleMessage(message: vscode.DebugProtocolMessage): void {
        const msg = message as DapRequest;
        if (msg.type !== 'request') {
            return;
        }
        switch (msg.command) {
            case 'initialize':
                this.respond(msg, {
                    supportsConfigurationDoneRequest: true,
                    supportsTerminateRequest: true,
                });
                this.emitEvent('initialized');
                break;
            case 'launch':
                this.handleLaunch(msg);
                break;
            case 'attach':
                this.handleAttach(msg);
                break;
            case 'disconnect':
            case 'terminate':
                this.stopChild();
                this.respond(msg);
                this.emitEvent('terminated');
                break;
            case 'configurationDone':
            case 'threads':
            case 'stackTrace':
            case 'scopes':
            case 'variables':
            case 'continue':
                this.respond(msg, msg.command === 'threads' ? { threads: [{ id: 1, name: 'bee' }] } : {});
                break;
            default:
                this.respond(msg);
                break;
        }
    }

    private handleLaunch(request: DapRequest): void {
        const program = request.arguments?.program;
        if (!program) {
            this.respondError(request, 'No program specified');
            return;
        }
        const beejsPath = this.config.getRuntimePath();
        const debugPort = this.config.getDebugPort();
        this.outputChannel.appendLine(
            `Launching: ${beejsPath} run --inspect-brk --inspect-port ${debugPort} ${program}`
        );
        this.child = spawn(beejsPath, [
            'run',
            '--inspect-brk',
            '--inspect-port',
            String(debugPort),
            program,
        ]);
        this.child.stdout?.on('data', (data: Buffer) => {
            this.outputChannel.append(data.toString());
        });
        this.child.stderr?.on('data', (data: Buffer) => {
            this.outputChannel.append(data.toString());
        });
        this.child.on('exit', (code: number | null) => {
            this.outputChannel.appendLine(`Process exited with code ${code}`);
            this.emitEvent('terminated');
        });
        this.respond(request);
        this.emitEvent('stopped', { reason: 'entry', threadId: 1 });
    }

    private handleAttach(request: DapRequest): void {
        const port = request.arguments?.port ?? this.config.getDebugPort();
        this.outputChannel.appendLine(`Attach to bee inspector on port ${port} (bee run --inspect)`);
        this.respond(request);
    }

    private respond(request: DapRequest, body?: Record<string, unknown>): void {
        this.sendEmitter.fire({
            type: 'response',
            request_seq: request.seq,
            success: true,
            command: request.command,
            seq: ++this.seq,
            body,
        } as vscode.DebugProtocolMessage);
    }

    private respondError(request: DapRequest, message: string): void {
        this.sendEmitter.fire({
            type: 'response',
            request_seq: request.seq,
            success: false,
            command: request.command,
            seq: ++this.seq,
            message,
        } as vscode.DebugProtocolMessage);
    }

    private emitEvent(event: string, body?: Record<string, unknown>): void {
        this.sendEmitter.fire({
            type: 'event',
            event,
            seq: ++this.seq,
            body,
        } as vscode.DebugProtocolMessage);
    }

    private stopChild(): void {
        if (this.child) {
            this.child.kill();
            this.child = undefined;
        }
    }

    public dispose(): void {
        this.stopChild();
        this.sendEmitter.dispose();
        this.outputChannel.dispose();
    }
}
