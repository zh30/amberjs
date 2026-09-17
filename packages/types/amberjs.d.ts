/**
 * Amberjs Runtime Type Definitions
 * Version: 1.0.0
 *
 * Comprehensive TypeScript declarations for Beejs runtime built-in APIs,
 * global objects, and native agentic AI modules.
 */

declare module "bee:ai" {
  /**
   * High-performance, zero-copy n-dimensional Tensor backed by TypedArray.
   */
  export class Tensor {
    readonly shape: readonly number[];
    readonly strides: readonly number[];
    readonly data: Float32Array;
    readonly length: number;

    constructor(data: ArrayLike<number> | Float32Array, shape?: number[]);

    /**
     * Creates a tensor filled with zeros.
     */
    static zeros(shape: number[]): Tensor;

    /**
     * Creates a tensor filled with ones.
     */
    static ones(shape: number[]): Tensor;

    /**
     * Matrix multiplication with another 2D tensor.
     */
    matmul(other: Tensor): Tensor;

    /**
     * Vector dot product.
     */
    dot(other: Tensor): number;

    /**
     * Computes L2 Frobenius norm.
     */
    norm(): number;

    /**
     * Applies softmax normalization along the last dimension.
     */
    softmax(): Tensor;

    /**
     * Computes cosine similarity between two vectors.
     */
    cosineSimilarity(other: Tensor): number;

    /**
     * Element-wise addition.
     */
    add(other: Tensor | number): Tensor;

    /**
     * Element-wise subtraction.
     */
    sub(other: Tensor | number): Tensor;

    /**
     * Element-wise multiplication.
     */
    mul(other: Tensor | number): Tensor;

    /**
     * Element-wise division.
     */
    div(other: Tensor | number): Tensor;

    /**
     * Flattens tensor data into a standard JavaScript array.
     */
    toArray(): number[];

    /**
     * Slices tensor along first dimension.
     */
    slice(start: number, end?: number): Tensor;
  }

  export interface LLMGenerateOptions {
    prompt: string;
    maxTokens?: number;
    temperature?: number;
    topP?: number;
    stop?: string[];
  }

  export interface LLMResponse {
    text: string;
    tokensUsed: number;
    finishReason: "stop" | "length" | "timeout";
  }

  /**
   * Local streaming LLM inference engine.
   */
  export class LLM {
    readonly modelPath: string;

    constructor(modelPath?: string);

    /**
     * Loads a local weights file or GGUF/ONNX model into memory.
     */
    static load(modelPath: string): Promise<LLM>;

    /**
     * Generates a complete response synchronously or asynchronously.
     */
    generate(options: LLMGenerateOptions): Promise<LLMResponse>;

    /**
     * Streams generated tokens sequentially.
     */
    generateStream(
      options: LLMGenerateOptions
    ): AsyncIterableIterator<string>;

    /**
     * Computes dense vector embeddings for input text.
     */
    embed(text: string, options?: EmbedOptions): Promise<Tensor | Float32Array>;
  }

  export interface EmbedOptions {
    dimensions?: 64 | 128 | 384 | number;
    asTensor?: boolean;
  }

  /**
   * Native zero-dependency deterministic text embedding function.
   */
  export function embed(text: string, options?: EmbedOptions): Float32Array | Tensor;

  /**
   * Batch native text embedding generator.
   */
  export function embedBatch(texts: string[], options?: EmbedOptions): Float32Array[] | Tensor[];

  /**
   * Semantic cosine similarity between two vector representations.
   */
  export function cosineSimilarity(a: Float32Array | number[] | Tensor, b: Float32Array | number[] | Tensor): number;

  export interface EdgeGenerateOptions {
    maxTokens?: number;
    temperature?: number;
    topP?: number;
    schema?: Record<string, unknown>;
    responseFormat?: "text" | "json_object";
    stopSequences?: string[];
    model?: string;
  }

  export interface EdgeGenerateResult {
    text: string;
    tokens: number;
    finishReason: string;
    schemaValid?: boolean;
    model: string;
  }

  /**
   * Native Edge SLM text generation with optional constrained JSON schema decoding.
   */
  export function generate(prompt: string, options?: EdgeGenerateOptions): Promise<EdgeGenerateResult>;

  /**
   * Native Edge SLM token streaming generator.
   */
  export function generateStream(prompt: string, options?: EdgeGenerateOptions): AsyncIterableIterator<string>;

  export interface AgentTool {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
    execute: (args: Record<string, unknown>) => Promise<unknown> | unknown;
  }

  /**
   * Deterministic pipeline orchestrator for Agent tools.
   */
  export class AgentPipeline {
    readonly tools: Map<string, AgentTool>;

    constructor();

    /**
     * Registers an executable tool into the pipeline.
     */
    registerTool(tool: AgentTool): this;

    /**
     * Executes a tool invocation plan deterministically.
     */
    execute(plan: { tool: string; args: Record<string, unknown> }): Promise<unknown>;
  }
}

declare module "bee:bench" {
  export interface BenchOptions {
    warmup?: number;
    iterations?: number;
  }

  export function bench(name: string, fn: () => void | Promise<void>, options?: BenchOptions): void;
}

/**
 * Beejs Runtime Global Namespace
 */
declare namespace bee {
  export const version: string;

  /**
   * Registers a microbenchmark.
   */
  export function bench(name: string, fn: () => void | Promise<void>): void;

  /**
   * Active sandbox permissions.
   */
  export const permissions: {
    readonly denyFs: boolean;
    readonly denyNet: boolean;
    readonly denyEnv: boolean;
    readonly denyRun: boolean;
    readonly isSandbox: boolean;
  };
}

declare module "bee:db" {
  export interface DatabaseOptions {
    readonly?: boolean;
  }

  export interface RunResult {
    changes: number;
    lastInsertRowid: number;
  }

  export interface PreparedStatement {
    all<T = Record<string, unknown>>(...params: unknown[]): T[];
    get<T = Record<string, unknown>>(...params: unknown[]): T | null;
    run(...params: unknown[]): RunResult;
  }

  export class Database {
    readonly filename: string;
    readonly readonly: boolean;

    constructor(filename?: string, options?: DatabaseOptions);
    exec(sql: string): this;
    prepare(sql: string): PreparedStatement;
    query<T = Record<string, unknown>>(sql: string, ...params: unknown[]): T[];
    run(sql: string, ...params: unknown[]): RunResult;
    transaction<T>(fn: () => T): T;
    close(): void;
  }

  export { VectorDB } from "bee:vector";
}

declare module "bee:sqlite" {
  export * from "bee:db";
}

declare module "bee:vector" {
  export type VectorMetric = "cosine" | "euclidean" | "dot";

  export interface VectorDBOptions {
    dimensions?: number;
    metric?: VectorMetric;
  }

  export interface SearchOptions {
    topK?: number;
    threshold?: number;
    filter?: (metadata: Record<string, unknown>) => boolean;
  }

  export interface SearchResult {
    id: string;
    score: number;
    metadata: Record<string, unknown>;
    vector: Float32Array;
  }

  export class VectorDB {
    dimensions: number;
    metric: VectorMetric;
    readonly size: number;

    constructor(options?: VectorDBOptions);
    insert(
      id: string | number,
      vector: number[] | Float32Array | { data: Float32Array },
      metadata?: Record<string, unknown>
    ): this;
    delete(id: string | number): boolean;
    get(id: string | number): { id: string; vector: Float32Array; metadata: Record<string, unknown> } | null;
    search(
      queryVector: number[] | Float32Array | { data: Float32Array },
      options?: SearchOptions
    ): SearchResult[];
    toJSON(): Record<string, unknown>;
    static fromJSON(data: Record<string, unknown>): VectorDB;
  }

  export function cosineSimilarity(a: Float32Array | number[], b: Float32Array | number[]): number;
  export function euclideanDistance(a: Float32Array | number[], b: Float32Array | number[]): number;
  export function dotProduct(a: Float32Array | number[], b: Float32Array | number[]): number;
}

declare module "bee:std/dotenv" {
  export interface DotenvConfigOptions {
    path?: string;
    override?: boolean;
  }

  export interface DotenvResult {
    parsed: Record<string, string>;
    error?: Error;
  }

  export function parse(src: string): Record<string, string>;
  export function config(options?: DotenvConfigOptions): DotenvResult;
}

declare module "bee:std/cli" {
  export const colors: {
    reset: (s: string) => string;
    bold: (s: string) => string;
    dim: (s: string) => string;
    red: (s: string) => string;
    green: (s: string) => string;
    yellow: (s: string) => string;
    blue: (s: string) => string;
    magenta: (s: string) => string;
    cyan: (s: string) => string;
    gray: (s: string) => string;
    bgRed: (s: string) => string;
    bgGreen: (s: string) => string;
    [key: string]: (s: string) => string;
  };

  export function table(data: Record<string, unknown>[], columns?: string[]): string;

  export interface ProgressBar {
    tick(delta?: number): void;
    render(): void;
    complete(): void;
  }

  export function progressBar(options?: { total?: number; width?: number }): ProgressBar;
  export function prompt(question: string, defaultValue?: string): Promise<string>;
}

declare module "bee:std/fs" {
  export interface FileEntry {
    path: string;
    name: string;
    isFile: boolean;
    isDirectory: boolean;
    size: number;
  }

  export interface WalkDirOptions {
    maxDepth?: number;
    exts?: string[];
  }

  export function walkDir(dir?: string, options?: WalkDirOptions): FileEntry[];
  export function copyDir(src: string, dest: string): void;
  export function emptyDir(dir: string): void;
  export function ensureDir(dir: string): void;
  export function ensureFile(file: string): void;
}

declare module "bee:std/crypto" {
  export interface JWTSignOptions {
    expiresIn?: number | string;
    algorithm?: "HS256";
  }

  export const jwt: {
    sign(payload: Record<string, unknown>, secret: string, options?: JWTSignOptions): string;
    verify<T = Record<string, unknown>>(token: string, secret: string): T;
  };

  export function hash(algorithm: "sha256" | "sha512" | "md5", data: string | Uint8Array, encoding?: "hex" | "base64"): string;
  export function uuid(): string;
  export function uuidv7(): string;
}

declare module "bee:std/assert" {
  export function assert(condition: unknown, message?: string): asserts condition;
  export function assertEquals<T>(actual: unknown, expected: T, message?: string): asserts actual is T;
  export function assertNotEquals(actual: unknown, expected: unknown, message?: string): void;
  export function assertThrows(fn: () => unknown, expectedError?: string | RegExp, message?: string): void;
}

declare module "bee:std" {
  export * as dotenv from "bee:std/dotenv";
  export * as cli from "bee:std/cli";
  export * as fs from "bee:std/fs";
  export * as crypto from "bee:std/crypto";
  export { assert, assertEquals, assertNotEquals, assertThrows } from "bee:std/assert";
}

declare module "bee:mcp" {
  export interface ToolDefinition {
    name: string;
    description?: string;
    inputSchema?: Record<string, unknown>;
  }

  export interface ResourceDefinition {
    uri: string;
    name: string;
    description?: string;
    mimeType?: string;
  }

  export interface PromptDefinition {
    name: string;
    description?: string;
    arguments?: Array<{ name: string; description?: string; required?: boolean }>;
  }

  export interface McpServerOptions {
    name: string;
    version: string;
  }

  export class McpServer {
    readonly name: string;
    readonly version: string;

    constructor(options?: McpServerOptions);

    tool(
      name: string,
      descriptionOrSchema: string | Record<string, unknown>,
      schemaOrHandler?: Record<string, unknown> | ((params: Record<string, unknown>) => unknown),
      handler?: (params: Record<string, unknown>) => unknown
    ): this;

    resource(
      uri: string,
      name: string,
      handler: (uri: string) => unknown,
      mimeType?: string
    ): this;

    prompt(
      name: string,
      description: string,
      args: Array<{ name: string; description?: string; required?: boolean }>,
      handler: (params: Record<string, unknown>) => unknown
    ): this;

    handleMessage(message: Record<string, unknown> | string): Promise<Record<string, unknown>>;
    connectLocal(): McpClient;
    startStdio(): void;
  }

  export class McpClient {
    constructor();
    ping(): Promise<boolean>;
    listTools(): Promise<ToolDefinition[]>;
    callTool(name: string, args?: Record<string, unknown>): Promise<unknown>;
    listResources(): Promise<ResourceDefinition[]>;
    readResource(uri: string): Promise<unknown>;
    listPrompts(): Promise<PromptDefinition[]>;
    getPrompt(name: string, args?: Record<string, unknown>): Promise<unknown>;
    handleMessage(message: Record<string, unknown> | string): Promise<Record<string, unknown>>;
  }
}

declare module "bee:vfs" {
  export function isEnabled(): boolean;
  export function isCow(): boolean;
  export function enable(cow?: boolean): void;
  export function disable(): void;
  export function reset(): void;
  export function listFiles(): string[];
  export function snapshot(): { enabled: boolean; cow: boolean; files: Record<string, string> };
}

declare module "bee:sandbox" {
  export * from "bee:vfs";
}

declare module "bee:ffi" {
  export type FFIType =
    | "void"
    | "bool"
    | "u8"
    | "i8"
    | "u16"
    | "i16"
    | "u32"
    | "i32"
    | "u64"
    | "i64"
    | "usize"
    | "isize"
    | "f32"
    | "f64"
    | "ptr"
    | "pointer"
    | "cstring"
    | "string";

  export interface FFIFunctionOptions {
    args: FFIType[];
    returns: FFIType;
  }

  export interface DynamicLibraryDefinition {
    [symbol: string]: FFIFunctionOptions;
  }

  export interface DynamicLibrary<T extends DynamicLibraryDefinition = DynamicLibraryDefinition> {
    readonly symbols: {
      [K in keyof T]: (...args: any[]) => any;
    };
    close(): void;
  }

  /**
   * Loads dynamic library with C ABI symbol bindings.
   */
  export function dlopen<T extends DynamicLibraryDefinition>(
    path: string,
    symbols: T
  ): DynamicLibrary<T>;

  /**
   * Retrieves raw memory address pointer of ArrayBuffer or TypedArray.
   */
  export function ptr(bufferOrView: ArrayBuffer | ArrayBufferView): bigint;

  /**
   * Reads primitive type value directly from memory address.
   */
  export function read(
    pointer: bigint | number,
    offset: number,
    type: string
  ): number | bigint | null | undefined;

  /**
   * Writes primitive type value directly into memory address.
   */
  export function write(
    pointer: bigint | number,
    offset: number,
    type: string,
    value: number | bigint
  ): boolean;

  /**
   * Reads null-terminated C string from memory pointer.
   */
  export function readCString(
    pointer: bigint | number,
    maxLen?: number
  ): string | null;

  export const FFIType: Record<string, string>;
}

declare module "ffi" {
  export * from "bee:ffi";
}

declare module "bee:pool" {
  export interface IsolatePoolOptions {
    minIsolates?: number;
    maxIsolates?: number;
    maxMemoryMb?: number;
    timeoutMs?: number;
  }

  export interface PoolStats {
    active: number;
    tasksCompleted: number;
    tasksFailed: number;
    totalCreated: number;
  }

  /**
   * Multi-tenant high-density Isolate pool for sandboxed agent & microservice execution.
   */
  export class IsolatePool {
    constructor(options?: IsolatePoolOptions);
    run<T = any>(code: string, timeoutMs?: number): Promise<T>;
    runFile<T = any>(filePath: string, timeoutMs?: number): Promise<T>;
    stats(): PoolStats;
    destroy(): void;
  }
}

declare module "pool" {
  export * from "bee:pool";
}

declare module "bee:wasm" {
  export interface MemoryViewOptions {
    target: any;
    byteLength?: number;
    byteOffset?: number;
  }

  export class MemoryView {
    readonly ptr: bigint;
    readonly byteLength: number;
    constructor(target: any, byteLength?: number, byteOffset?: number);

    getUint8(offset: number): number;
    setUint8(offset: number, val: number): void;
    getInt8(offset: number): number;
    setInt8(offset: number, val: number): void;
    getInt32(offset: number): number;
    setInt32(offset: number, val: number): void;
    getUint32(offset: number): number;
    setUint32(offset: number, val: number): void;
    getFloat32(offset: number): number;
    setFloat32(offset: number, val: number): void;
    getFloat64(offset: number): number;
    setFloat64(offset: number, val: number): void;
    getCString(offset?: number): string;
    getString(offset: number, length: number): string;
    setString(offset: number, str: string): number;
    copyFrom(srcPtr: bigint | number | any, length: number, dstOffset?: number): boolean;
    copyTo(dstPtr: bigint | number | any, length: number, srcOffset?: number): boolean;
    fill(value: number, offset?: number, length?: number | null): boolean;
  }

  export function ptr(target: any): bigint;
  export function copyMemory(srcPtr: bigint | number, dstPtr: bigint | number, length: number): boolean;
  export function fillMemory(ptr: bigint | number, value: number, length: number): boolean;
  export function compareMemory(ptr1: bigint | number, ptr2: bigint | number, length: number): number;
  export function read(ptr: bigint | number, type?: string): number | bigint | null;
  export function write(ptr: bigint | number, val: number | bigint, type?: string): boolean;
  export function readCString(ptr: bigint | number): string | null;
  export function readString(ptr: bigint | number, length: number): string | null;
  export function writeString(ptr: bigint | number, str: string): number;
  export function loadModuleMmap(filePath: string): Promise<WebAssembly.Module>;
  export function createSharedMemory(options?: { initial?: number; maximum?: number }): WebAssembly.Memory;
  export function wrapPointer(ptr: bigint | number, byteLength: number, type?: string): any;
  export function linkTensor(tensor: any, memory: WebAssembly.Memory, byteOffset?: number): any;
  export function createTensorFromMemory(memory: WebAssembly.Memory, byteOffset: number, shape: number[], dtype?: string): any;

  export const version: string;
}

declare module "wasm" {
  export * from "bee:wasm";
}

declare module "bee:replay" {
  export interface RecordOptions {
    script?: string;
    outputPath?: string;
  }

  export interface TraceStats {
    version: number;
    script: string;
    totalEvents: number;
    startTime: number;
    endTime: number;
    durationMs: number;
    mode: "idle" | "recording" | "replaying";
  }

  export function startRecording(opts?: RecordOptions | string): boolean;
  export function stopRecording(outputPath?: string): any;
  export function loadTrace(traceOrPath: string | object): boolean;
  export function step<T = any>(name: string, input: any, outputOrFn?: T | ((input: any) => T)): T;
  export function isRecording(): boolean;
  export function isReplaying(): boolean;
  export function getTraceStats(): TraceStats;
  export function reset(): boolean;
}

declare module "replay" {
  export * from "bee:replay";
}

declare module "bee:weights" {
  export interface GGUFTensorInfo {
    name: string;
    shape: number[];
    dtype: string;
    offset: number;
    size_bytes: number;
  }

  export interface GGUFMetadata {
    version: number;
    tensor_count: number;
    kv_count: number;
    metadata: Record<string, unknown>;
    tensors: GGUFTensorInfo[];
  }

  export interface SafeTensorItem {
    name: string;
    dtype: string;
    shape: number[];
    data_offsets: [number, number];
    size_bytes: number;
  }

  export interface SafeTensorsMetadata {
    header_size: number;
    metadata: Record<string, unknown>;
    tensors: SafeTensorItem[];
  }

  export interface LoadedTensor {
    name: string;
    dtype: string;
    shape: number[];
    buffer: ArrayBuffer;
    byteLength: number;
  }

  export function readGGUFMetadata(filePath: string): GGUFMetadata;
  export function readSafeTensorsMetadata(filePath: string): SafeTensorsMetadata;
  export function loadTensor(filePath: string, tensorName: string): LoadedTensor;
}

declare module "weights" {
  export * from "bee:weights";
}

declare module "bee:security" {
  export interface PermissionDescriptor {
    name: "read" | "write" | "net" | "listen" | "env" | "run";
    path?: string;
    host?: string;
    varName?: string;
    command?: string;
  }

  export interface PermissionStatus {
    state: "granted" | "denied";
    name: string;
  }

  export interface PermissionRuleItem {
    kind: string;
    action: string;
    resource: string;
  }

  export interface PermissionListResult {
    allow: PermissionRuleItem[];
    deny: PermissionRuleItem[];
  }

  export interface SandboxPolicyRules {
    allowRead?: string[];
    allowWrite?: string[];
    allowNet?: string[];
    allowListen?: string[];
    allowEnv?: string[];
    allowRun?: string[];
    denyFs?: boolean;
    denyNet?: boolean;
    denyEnv?: boolean;
    denyRun?: boolean;
  }

  export interface SandboxPolicy {
    readonly allowRead: string[];
    readonly allowWrite: string[];
    readonly allowNet: string[];
    readonly allowListen: string[];
    readonly allowEnv: string[];
    readonly allowRun: string[];
    readonly denyFs: boolean;
    readonly denyNet: boolean;
    readonly denyEnv: boolean;
    readonly denyRun: boolean;
    readonly isSandbox: boolean;
  }

  export namespace permissions {
    export function query(descriptor: PermissionDescriptor): PermissionStatus;
    export function has(descriptor: PermissionDescriptor): boolean;
    export function list(): PermissionListResult;
    export function revoke(descriptor: PermissionDescriptor): boolean;
  }

  export function createSandboxPolicy(rules?: SandboxPolicyRules): SandboxPolicy;
  export function attenuate(basePolicy: SandboxPolicy, restPolicy: SandboxPolicy): SandboxPolicy;
}

declare module "security" {
  export * from "bee:security";
}

declare module "bee:permissions" {
  import { PermissionDescriptor, PermissionStatus, PermissionListResult } from "bee:security";
  export function query(descriptor: PermissionDescriptor): PermissionStatus;
  export function has(descriptor: PermissionDescriptor): boolean;
  export function list(): PermissionListResult;
  export function revoke(descriptor: PermissionDescriptor): boolean;
}

declare module "permissions" {
  export * from "bee:permissions";
}

declare module "bee:kv" {
  export interface KVSetOptions {
    ttlMs?: number;
  }

  export interface KVScanOptions {
    prefix?: string;
    limit?: number;
  }

  export interface KVBatchOperation<T = any> {
    type: "put" | "set" | "del" | "delete";
    key: string;
    value?: T;
    ttlMs?: number;
  }

  export class KVStore {
    readonly path: string | null;
    readonly isClosed: boolean;

    static open(path: string): KVStore;
    static openMemory(): KVStore;

    get<T = any>(key: string): T | undefined;
    set<T = any>(key: string, value: T, options?: KVSetOptions | number): this;
    delete(key: string): boolean;
    has(key: string): boolean;
    keys(prefix?: string | null): string[];
    values<T = any>(): T[];
    entries<T = any>(prefix?: string | null): Array<[string, T]>;
    scan<T = any>(options?: KVScanOptions | string): Array<[string, T]>;
    incr(key: string, delta?: number): number;
    batch(operations: KVBatchOperation[]): this;
    clear(): this;
    compact(): boolean;
    flush(): boolean;
    close(): void;
  }

  export function open(path: string): KVStore;
  export function openMemory(): KVStore;
}

declare module "kv" {
  export * from "bee:kv";
}

declare module "bee:tools" {
  export interface JSONSchemaProperty {
    type: "string" | "number" | "integer" | "boolean" | "array" | "object";
    description?: string;
    enum?: any[];
    default?: any;
    items?: JSONSchemaProperty;
    properties?: Record<string, JSONSchemaProperty>;
    required?: string[];
  }

  export interface JSONSchemaDefinition {
    type: "object";
    properties: Record<string, JSONSchemaProperty>;
    required?: string[];
  }

  export interface ToolDefinition<TArgs = any, TResult = any> {
    name: string;
    description?: string;
    parameters?: JSONSchemaDefinition;
    execute: (args: TArgs) => Promise<TResult> | TResult;
  }

  export interface ToolCall {
    name: string;
    arguments: Record<string, unknown>;
  }

  export class AgentTool<TArgs = any, TResult = any> {
    readonly name: string;
    readonly description: string;
    readonly parameters: JSONSchemaDefinition;
    constructor(def: ToolDefinition<TArgs, TResult>);
    execute(args?: TArgs): Promise<TResult>;
    toJSON(): {
      type: "function";
      function: {
        name: string;
        description: string;
        parameters: JSONSchemaDefinition;
      };
    };
  }

  export function compileSchemaTool<TArgs = any, TResult = any>(
    def: ToolDefinition<TArgs, TResult>
  ): AgentTool<TArgs, TResult>;

  export interface OpenAPIOptions {
    baseUrl?: string;
    headers?: Record<string, string>;
  }

  export type OpenAPIToolList = Array<AgentTool> & {
    map: Record<string, AgentTool>;
  };

  export function fromOpenAPI(
    specOrJson: string | Record<string, unknown>,
    options?: OpenAPIOptions
  ): OpenAPIToolList;

  export function parseToolCalls(llmOutput: string | object): ToolCall[];

  export function registerTools(pipeline: any, tools: AgentTool[] | Record<string, AgentTool>): any;
}

declare module "tools" {
  export * from "bee:tools";
}

declare module "bee:sandbox" {
  export interface EnclaveOptions {
    context?: Record<string, unknown>;
  }

  export function isEnabled(): boolean;
  export function isCow(): boolean;
  export function enable(cow?: boolean): void;
  export function disable(): void;
  export function reset(): void;
  export function listFiles(): string[];
  export function snapshot(): any;
  export function startAuditLog(path: string): boolean;
  export function stopAuditLog(): boolean;
  export function getAuditLogPath(): string | null;
  export function createEnclave<T = any>(
    codeOrFn: string | (() => T),
    options?: EnclaveOptions
  ): T;
}

declare module "sandbox" {
  export * from "bee:sandbox";
}

declare module "bee:bus" {
  export interface Message<TPayload = any> {
    id: string;
    topic: string;
    payload: TPayload;
    headers?: Record<string, string>;
    timestamp: number;
    priority?: number;
    replyTo?: string;
    correlationId?: string;
    reply(responsePayload: any): Message;
  }

  export interface Subscription {
    id: string;
    pattern: string;
    priority: number;
    once: boolean;
    unsubscribe(): void;
  }

  export interface SubscribeOptions {
    priority?: number;
    once?: boolean;
  }

  export interface PublishOptions {
    id?: string;
    headers?: Record<string, string>;
    priority?: number;
    replyTo?: string;
    correlationId?: string;
  }

  export interface RequestOptions extends PublishOptions {
    timeoutMs?: number;
  }

  export interface BusMetrics {
    published_count: number;
    delivered_count: number;
    dead_letter_count: number;
    active_subscriptions: number;
  }

  export class MessageBus {
    readonly id: number;
    constructor(id?: number);
    subscribe<T = any>(
      pattern: string,
      handler: (message: Message<T>) => void | Promise<void>,
      options?: SubscribeOptions
    ): Subscription;
    once<T = any>(
      pattern: string,
      handler: (message: Message<T>) => void | Promise<void>,
      options?: SubscribeOptions
    ): Subscription;
    unsubscribe(subId: string | Subscription): boolean;
    use(middleware: (msg: Message, next: () => void) => void | boolean): this;
    publish<T = any>(topic: string, payload: T, options?: PublishOptions): Message<T>;
    broadcast<T = any>(topic: string, payload: T, options?: PublishOptions): Message<T>;
    request<TResponse = any, TPayload = any>(
      topic: string,
      payload: TPayload,
      options?: RequestOptions
    ): Promise<TResponse>;
    reply(originalMessage: Message, responsePayload: any): Message;
    getMetrics(): BusMetrics;
    getDeadLetters(): Message[];
    clearDeadLetters(): boolean;
    getTopics(): string[];
    clear(): void;
  }

  export function createBus(id?: number): MessageBus;
  export function getDefaultBus(): MessageBus;
  export function subscribe<T = any>(
    pattern: string,
    handler: (message: Message<T>) => void | Promise<void>,
    options?: SubscribeOptions
  ): Subscription;
  export function once<T = any>(
    pattern: string,
    handler: (message: Message<T>) => void | Promise<void>,
    options?: SubscribeOptions
  ): Subscription;
  export function unsubscribe(subId: string | Subscription): boolean;
  export function publish<T = any>(topic: string, payload: T, options?: PublishOptions): Message<T>;
  export function broadcast<T = any>(topic: string, payload: T, options?: PublishOptions): Message<T>;
  export function request<TResponse = any, TPayload = any>(
    topic: string,
    payload: TPayload,
    options?: RequestOptions
  ): Promise<TResponse>;
  export function reply(originalMessage: Message, responsePayload: any): Message;
  export function use(middleware: (msg: Message, next: () => void) => void | boolean): MessageBus;
  export function getMetrics(): BusMetrics;
  export function getDeadLetters(): Message[];
  export function clearDeadLetters(): boolean;
  export function getTopics(): string[];
  export function topicMatches(pattern: string, topic: string): boolean;
}

declare module "bus" {
  export * from "bee:bus";
}

declare module "bee:grammar" {
  export interface SSEMessage {
    event: string;
    data: string;
    id?: string;
    retry?: number;
    json<T = any>(): T;
  }

  export interface GrammarResult {
    valid: boolean;
    completed: boolean;
    error?: string;
    [key: string]: any;
  }

  export class Grammar {
    readonly type: string;
    validate(text: string): GrammarResult;
    accept(prefix: string, nextToken: string): boolean;
  }

  export interface StreamDecoder<T = any> {
    push(chunk: string): T;
    finish(): T;
    readonly current: T;
    readonly raw: string;
    reset(): void;
  }

  export interface StreamDecoderOptions<T = any> {
    onChunk?: (parsed: T, isComplete: boolean) => void;
  }

  export function parsePartialJSON<T = any>(input: string): T;
  export function createStreamDecoder<T = any>(
    options?: StreamDecoderOptions<T>
  ): StreamDecoder<T>;
  export function parseSSEChunk(chunk: string): SSEMessage[];
  export function createChoiceGrammar(choices: string[]): Grammar;
  export function createRegexGrammar(pattern: string | RegExp): Grammar;
  export function createJSONGrammar(schema?: any): Grammar;
  export function createGrammar(spec: {
    choices?: string[];
    regex?: string | RegExp;
    pattern?: string | RegExp;
    schema?: any;
    type?: string;
  }): Grammar;
}

declare module "grammar" {
  export * from "bee:grammar";
}

declare module "bee:checkpoint" {
  export interface Checkpoint<TState = any> {
    id: string;
    parent_id?: string;
    branch: string;
    timestamp: number;
    state: TState;
    metadata?: Record<string, unknown>;
  }

  export interface ValueDiff {
    from: any;
    to: any;
  }

  export interface StateDiff {
    added: Record<string, unknown>;
    modified: Record<string, ValueDiff>;
    deleted: string[];
  }

  export interface SaveCheckpointOptions<TState = any> {
    id?: string;
    state?: TState;
    branch?: string;
    metadata?: Record<string, unknown>;
  }

  export interface ListCheckpointOptions {
    branch?: string;
  }

  export class CheckpointManager {
    readonly id: number;
    readonly currentBranch: string;
    constructor(id?: number | null, branch?: string);
    save<T = any>(
      idOrOptions: string | SaveCheckpointOptions<T> | T,
      state?: T,
      metadata?: Record<string, unknown>
    ): Checkpoint<T>;
    get<T = any>(id: string): Checkpoint<T> | undefined;
    restore<T = any>(id: string): T;
    list(options?: ListCheckpointOptions): Checkpoint[];
    diff(fromId: string, toId: string): StateDiff;
    fork(fromId: string, branchName: string): CheckpointManager;
    delete(id: string): boolean;
    clear(): boolean;
    persist(kvStore: any, prefix?: string): number;
    restoreFromKV(kvStore: any, prefix?: string): number;
  }

  export function createCheckpointManager(
    id?: number | null,
    branch?: string
  ): CheckpointManager;
  export function getDefaultManager(): CheckpointManager;
  export function save<T = any>(
    idOrOptions: string | SaveCheckpointOptions<T> | T,
    state?: T,
    metadata?: Record<string, unknown>
  ): Checkpoint<T>;
  export function restore<T = any>(id: string): T;
  export function get<T = any>(id: string): Checkpoint<T> | undefined;
  export function list(options?: ListCheckpointOptions): Checkpoint[];
  export function diff(fromId: string, toId: string): StateDiff;
  export function fork(fromId: string, branchName: string): CheckpointManager;
  export function clear(): boolean;
}

declare module "checkpoint" {
  export * from "bee:checkpoint";
}

/**
 * WinterTC Sockets API (TC55 proposal-sockets-api)
 */
declare module "bee:sockets" {
  export interface SocketInfo {
    remoteAddress: string;
    localAddress: string;
    alpn: string | null;
  }

  export interface SocketOptions {
    secureTransport?: "off" | "on" | "starttls";
    allowHalfOpen?: boolean;
    sni?: string;
    alpn?: string[];
  }

  export interface SocketAddress {
    hostname: string;
    port: number;
  }

  export class Socket {
    readonly readable: ReadableStream<Uint8Array>;
    readonly writable: WritableStream<Uint8Array>;
    readonly opened: Promise<SocketInfo>;
    readonly closed: Promise<void>;
    readonly upgraded: boolean;

    constructor(address: string | SocketAddress, options?: SocketOptions);
    close(reason?: any): Promise<void>;
    startTls(): Socket;
  }

  export function connect(
    address: string | SocketAddress,
    options?: SocketOptions
  ): Socket;
}

declare module "sockets" {
  export * from "bee:sockets";
}

declare module "wintertc:sockets" {
  export * from "bee:sockets";
}

// Global WinterTC Web APIs
declare class DOMException extends Error {
  readonly name: string;
  readonly message: string;
  readonly code: number;

  static readonly INDEX_SIZE_ERR: 1;
  static readonly DOMSTRING_SIZE_ERR: 2;
  static readonly HIERARCHY_REQUEST_ERR: 3;
  static readonly WRONG_DOCUMENT_ERR: 4;
  static readonly INVALID_CHARACTER_ERR: 5;
  static readonly NO_DATA_ALLOWED_ERR: 6;
  static readonly NO_MODIFICATION_ALLOWED_ERR: 7;
  static readonly NOT_FOUND_ERR: 8;
  static readonly NOT_SUPPORTED_ERR: 9;
  static readonly INUSE_ATTRIBUTE_ERR: 10;
  static readonly INVALID_STATE_ERR: 11;
  static readonly SYNTAX_ERR: 12;
  static readonly INVALID_MODIFICATION_ERR: 13;
  static readonly NAMESPACE_ERR: 14;
  static readonly INVALID_ACCESS_ERR: 15;
  static readonly VALIDATION_ERR: 16;
  static readonly TYPE_MISMATCH_ERR: 17;
  static readonly SECURITY_ERR: 18;
  static readonly NETWORK_ERR: 19;
  static readonly ABORT_ERR: 20;
  static readonly URL_MISMATCH_ERR: 21;
  static readonly QUOTA_EXCEEDED_ERR: 22;
  static readonly TIMEOUT_ERR: 23;
  static readonly INVALID_NODE_TYPE_ERR: 24;
  static readonly DATA_CLONE_ERR: 25;

  constructor(message?: string, name?: string);
}

declare interface URLPatternInit {
  protocol?: string;
  username?: string;
  password?: string;
  hostname?: string;
  port?: string;
  pathname?: string;
  search?: string;
  hash?: string;
  baseURL?: string;
}

declare interface URLPatternComponentResult {
  input: string;
  groups: Record<string, string>;
}

declare interface URLPatternResult {
  inputs: [string | URLPatternInit, string?];
  protocol: URLPatternComponentResult;
  username: URLPatternComponentResult;
  password: URLPatternComponentResult;
  hostname: URLPatternComponentResult;
  port: URLPatternComponentResult;
  pathname: URLPatternComponentResult;
  search: URLPatternComponentResult;
  hash: URLPatternComponentResult;
}

declare class URLPattern {
  readonly protocol: string;
  readonly username: string;
  readonly password: string;
  readonly hostname: string;
  readonly port: string;
  readonly pathname: string;
  readonly search: string;
  readonly hash: string;
  readonly hasRegExpGroups: boolean;

  constructor(input: string | URLPatternInit, baseURL?: string);
  test(input: string | URLPatternInit, baseURL?: string): boolean;
  exec(input: string | URLPatternInit, baseURL?: string): URLPatternResult | null;
}

declare class ByteLengthQueuingStrategy {
  readonly highWaterMark: number;
  constructor(options: { highWaterMark: number });
  size(chunk?: any): number;
}

declare class CountQueuingStrategy {
  readonly highWaterMark: number;
  constructor(options: { highWaterMark: number });
  size(chunk?: any): number;
}

declare function connect(
  address: string | import("bee:sockets").SocketAddress,
  options?: import("bee:sockets").SocketOptions
): import("bee:sockets").Socket;

declare function reportError(error: any): void;


/** Amber module aliases */
declare module "amber:ai" {
  export * from "bee:ai";
}
declare module "amber:bench" {
  export * from "bee:bench";
}
declare module "amber:bus" {
  export * from "bee:bus";
}
declare module "amber:checkpoint" {
  export * from "bee:checkpoint";
}
declare module "amber:db" {
  export * from "bee:db";
}
declare module "amber:ffi" {
  export * from "bee:ffi";
}
declare module "amber:grammar" {
  export * from "bee:grammar";
}
declare module "amber:kv" {
  export * from "bee:kv";
}
declare module "amber:mcp" {
  export * from "bee:mcp";
}
declare module "amber:permissions" {
  export * from "bee:permissions";
}
declare module "amber:pool" {
  export * from "bee:pool";
}
declare module "amber:replay" {
  export * from "bee:replay";
}
declare module "amber:sandbox" {
  export * from "bee:sandbox";
}
declare module "amber:security" {
  export * from "bee:security";
}
declare module "amber:sockets" {
  export * from "bee:sockets";
}
declare module "amber:sqlite" {
  export * from "bee:sqlite";
}
declare module "amber:std" {
  export * from "bee:std";
}
declare module "amber:std/assert" {
  export * from "bee:std/assert";
}
declare module "amber:std/cli" {
  export * from "bee:std/cli";
}
declare module "amber:std/crypto" {
  export * from "bee:std/crypto";
}
declare module "amber:std/dotenv" {
  export * from "bee:std/dotenv";
}
declare module "amber:std/fs" {
  export * from "bee:std/fs";
}
declare module "amber:tools" {
  export * from "bee:tools";
}
declare module "amber:vector" {
  export * from "bee:vector";
}
declare module "amber:vfs" {
  export * from "bee:vfs";
}
declare module "amber:wasm" {
  export * from "bee:wasm";
}
declare module "amber:weights" {
  export * from "bee:weights";
}
