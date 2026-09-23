/**
 * Amber Debug Adapter
 * Implements the Debug Adapter Protocol for VS Code integration
 */

import { DebugAdapter, LoggingDebugSession, InitializedEvent, TerminatedEvent, StoppedEvent, BreakpointEvent, OutputEvent } from '@vscode/debugadapter';
import { DebugProtocol } from '@vscode/debugprotocol';
import { Subject } from 'await-notify';

export class AmberDebugAdapter extends LoggingDebugSession {

    private _configurationDone = new Subject();
    private breakpoints: Map<string, DebugProtocol.Breakpoint[]> = new Map();

    public constructor() {
        super('amberjs-debug.log');
    }

    protected initializeRequest(response: DebugProtocol.InitializeResponse, args: DebugProtocol.InitializeRequestArguments): void {
        response.body = response.body || {};
        response.body.supportsConfigurationDoneRequest = true;
        response.body.supportsEvaluateForHovers = true;
        response.body.supportsStepBack = true;
        response.body.supportsSetVariable = true;

        this.sendResponse(response);
        this.sendEvent(new InitializedEvent());
    }

    protected configurationDoneRequest(response: DebugProtocol.ConfigurationDoneResponse, args: DebugProtocol.ConfigurationDoneArguments): void {
        this._configurationDone.notify();
        this.sendResponse(response);
    }

    protected launchRequest(response: DebugProtocol.LaunchResponse, args: DebugProtocol.LaunchRequestArguments): void {
        // TODO: Launch Amber runtime with debug mode
        this.sendResponse(response);
    }

    protected setBreakPointsRequest(response: DebugProtocol.SetBreakpointsResponse, args: DebugProtocol.SetBreakpointsArguments): void {
        const path = args.source.path || '';
        const clientLines = args.lines || [];

        // TODO: Set breakpoints in Amber runtime
        const breakpoints: DebugProtocol.Breakpoint[] = clientLines.map(line => ({
            id: 1,
            verified: true,
            line: line,
            source: args.source
        }));

        this.breakpoints.set(path, breakpoints);
        response.body = { breakpoints };

        this.sendResponse(response);
    }

    protected threadsRequest(response: DebugProtocol.ThreadsResponse): void {
        response.body = {
            threads: [
                { id: 1, name: 'Main Thread' }
            ]
        };
        this.sendResponse(response);
    }

    protected stackTraceRequest(response: DebugProtocol.StackTraceResponse, args: DebugProtocol.StackTraceArguments): void {
        // TODO: Get stack trace from Amber runtime
        response.body = {
            stackFrames: [
                {
                    id: 1,
                    name: 'main',
                    source: { path: 'test.js' },
                    line: 10,
                    column: 0
                }
            ]
        };
        this.sendResponse(response);
    }

    protected scopesRequest(response: DebugProtocol.ScopesResponse, args: DebugProtocol.ScopesArguments): void {
        response.body = {
            scopes: [
                { id: 1, name: 'Locals', line: 10, column: 0, source: { path: 'test.js' } }
            ]
        };
        this.sendResponse(response);
    }

    protected variablesRequest(response: DebugProtocol.VariablesResponse, args: DebugProtocol.VariablesArguments): void {
        // TODO: Get variables from Amber runtime
        response.body = {
            variables: [
                { name: 'count', value: '42', type: 'number', variablesReference: 0 }
            ]
        };
        this.sendResponse(response);
    }

    protected continueRequest(response: DebugProtocol.ContinueResponse, args: DebugProtocol.ContinueArguments): void {
        // TODO: Continue execution
        this.sendResponse(response);
    }

    protected nextRequest(response: DebugProtocol.NextResponse, args: DebugProtocol.NextArguments): void {
        // TODO: Step next
        this.sendResponse(response);
    }

    protected stepInRequest(response: DebugProtocol.StepInResponse, args: DebugProtocol.StepInArguments): void {
        // TODO: Step in
        this.sendResponse(response);
    }

    protected stepOutRequest(response: DebugProtocol.StepOutResponse, args: DebugProtocol.StepOutArguments): void {
        // TODO: Step out
        this.sendResponse(response);
    }

    protected evaluateRequest(response: DebugProtocol.EvaluateResponse, args: DebugProtocol.EvaluateArguments): void {
        // TODO: Evaluate expression in Amber runtime
        response.body = {
            result: '42',
            type: 'number'
        };
        this.sendResponse(response);
    }
}

DebugAdapter.run(AmberDebugAdapter);
