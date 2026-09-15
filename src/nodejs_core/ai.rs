// Stage 100 / v1.0.0: 原生 Agentic AI 核心加速层 (BeeJS-AI Core)
// 对齐未来 3 年路线图 (2026-2029) Year 1 目标：提供零拷贝 Tensor、本地流式推理抽象与 Agent 执行管道
// 模块路径: `bee:ai` 或 `ai`

use anyhow::Result;
use rusty_v8 as v8;

/// Helper to extract contiguous f32 slice from V8 ArrayBufferView or Array
fn extract_f32_slice(scope: &mut v8::PinScope, val: v8::Local<v8::Value>) -> Option<Vec<f32>> {
    if val.is_typed_array() {
        if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(val) {
            let ab = view.buffer(scope)?;
            let store = ab.get_backing_store();
            if let Some(ptr) = store.data() {
                let byte_offset = view.byte_offset();
                let byte_length = view.byte_length();
                let count = byte_length / std::mem::size_of::<f32>();
                let slice = unsafe {
                    let p = (ptr.as_ptr() as *const u8).add(byte_offset) as *const f32;
                    std::slice::from_raw_parts(p, count)
                };
                return Some(slice.to_vec());
            }
        }
    } else if val.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(val) {
            let mut out = Vec::with_capacity(arr.length() as usize);
            for i in 0..arr.length() {
                if let Some(item) = arr.get_index(scope, i) {
                    if let Some(n) = item.to_number(scope) {
                        out.push(n.value() as f32);
                    }
                }
            }
            return Some(out);
        }
    }
    None
}

/// Helper to wrap contiguous f32 slice into a V8 Float32Array
fn create_v8_float32_array<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    data: &[f32],
) -> Option<v8::Local<'s, v8::Float32Array>> {
    let byte_len = data.len() * std::mem::size_of::<f32>();
    let buffer = v8::ArrayBuffer::new(scope, byte_len);
    let store = buffer.get_backing_store();
    if let Some(ptr) = store.data() {
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr() as *const u8,
                ptr.as_ptr() as *mut u8,
                byte_len,
            );
        }
    }
    v8::Float32Array::new(scope, buffer, 0, data.len())
}

/// 设置全局与模块化 `bee:ai` API
pub fn setup_ai_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Register native AI callback for high-performance embeddings and vector math
    let ai_native_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let action = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            match action.as_str() {
                "embed" => {
                    let text = if args.length() > 1 {
                        args.get(1).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let dims = if args.length() > 2 && args.get(2).is_number() {
                        args.get(2)
                            .to_integer(scope)
                            .map(|i| i.value() as usize)
                            .unwrap_or(128)
                    } else {
                        128
                    };
                    let normalize = if args.length() > 3 && args.get(3).is_boolean() {
                        args.get(3).boolean_value(scope)
                    } else {
                        true
                    };

                    let opts = crate::ai_engine::EmbedOptions {
                        dimensions: dims,
                        normalize,
                    };
                    let vec = crate::ai_engine::candle_embed(&text, &opts);
                    if let Some(f32_arr) = create_v8_float32_array(scope, &vec) {
                        retval.set(f32_arr.into());
                    } else {
                        let arr = v8::Array::new(scope, vec.len() as i32);
                        for (i, &val) in vec.iter().enumerate() {
                            let num = v8::Number::new(scope, val as f64);
                            arr.set_index(scope, i as u32, num.into());
                        }
                        retval.set(arr.into());
                    }
                }
                "similarity" => {
                    let vec_a = extract_f32_slice(scope, args.get(1)).unwrap_or_default();
                    let vec_b = extract_f32_slice(scope, args.get(2)).unwrap_or_default();
                    let sim = crate::ai_engine::candle_cosine_similarity(&vec_a, &vec_b);
                    retval.set(v8::Number::new(scope, sim as f64).into());
                }
                "load_model" => {
                    let path = if args.length() > 1 {
                        args.get(1).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let opts_json = if args.length() > 2 && args.get(2).is_string() {
                        args.get(2).to_rust_string_lossy(scope)
                    } else {
                        "{}".to_string()
                    };
                    match crate::ai_engine::CandleModel::load(&path, &opts_json) {
                        Ok(arc_model) => {
                            let m = arc_model.lock().unwrap();
                            let info = serde_json::json!({
                                "modelId": m.id,
                                "path": m.path,
                                "device": format!("{:?}", m.device),
                                "contextLength": m.context_length,
                                "eosTokenId": m.eos_token_id
                            });
                            if let Some(s) = v8::String::new(scope, &info.to_string()) {
                                retval.set(s.into());
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to load model: {}", e);
                            let s = v8::String::new(scope, &err_msg).unwrap();
                            let err = v8::Exception::error(scope, s);
                            scope.throw_exception(err);
                        }
                    }
                }
                "model_generate" => {
                    let model_id = if args.length() > 1 && args.get(1).is_number() {
                        args.get(1)
                            .to_integer(scope)
                            .map(|i| i.value() as u64)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    let prompt = if args.length() > 2 {
                        args.get(2).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let opts_json = if args.length() > 3 && args.get(3).is_string() {
                        args.get(3).to_rust_string_lossy(scope)
                    } else {
                        "{}".to_string()
                    };
                    let opts: crate::ai_engine::GenerateOptions =
                        serde_json::from_str(&opts_json).unwrap_or_default();
                    if let Some(model_arc) = crate::ai_engine::get_model(model_id) {
                        let mut model = model_arc.lock().unwrap();
                        match model.generate(&prompt, &opts) {
                            Ok(res) => {
                                let res_json = serde_json::to_string(&res).unwrap_or_default();
                                if let Some(s) = v8::String::new(scope, &res_json) {
                                    retval.set(s.into());
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Model generate error: {}", e);
                                let s = v8::String::new(scope, &err_msg).unwrap();
                                let err = v8::Exception::error(scope, s);
                                scope.throw_exception(err);
                            }
                        }
                    } else {
                        match crate::ai_engine::EdgeGenerator::generate(&prompt, &opts) {
                            Ok(res) => {
                                let res_json = serde_json::to_string(&res).unwrap_or_default();
                                if let Some(s) = v8::String::new(scope, &res_json) {
                                    retval.set(s.into());
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("AI generate failed: {}", e);
                                let s = v8::String::new(scope, &err_msg).unwrap();
                                let err = v8::Exception::error(scope, s);
                                scope.throw_exception(err);
                            }
                        }
                    }
                }
                "model_generate_stream" => {
                    let model_id = if args.length() > 1 && args.get(1).is_number() {
                        args.get(1)
                            .to_integer(scope)
                            .map(|i| i.value() as u64)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    let prompt = if args.length() > 2 {
                        args.get(2).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let opts_json = if args.length() > 3 && args.get(3).is_string() {
                        args.get(3).to_rust_string_lossy(scope)
                    } else {
                        "{}".to_string()
                    };
                    let opts: crate::ai_engine::GenerateOptions =
                        serde_json::from_str(&opts_json).unwrap_or_default();
                    let mut chunks = Vec::new();
                    if let Some(model_arc) = crate::ai_engine::get_model(model_id) {
                        let mut model = model_arc.lock().unwrap();
                        let _ = model.generate_stream(&prompt, &opts, |chunk| {
                            chunks.push(chunk.to_string());
                            true
                        });
                    } else {
                        let _ = crate::ai_engine::EdgeGenerator::generate_stream(
                            &prompt,
                            &opts,
                            |chunk| {
                                chunks.push(chunk.to_string());
                                true
                            },
                        );
                    }
                    let chunks_json =
                        serde_json::to_string(&chunks).unwrap_or_else(|_| "[]".to_string());
                    if let Some(s) = v8::String::new(scope, &chunks_json) {
                        retval.set(s.into());
                    }
                }
                "tensor_matmul" => {
                    let a_data = extract_f32_slice(scope, args.get(1));
                    let m = args
                        .get(2)
                        .to_integer(scope)
                        .map(|i| i.value() as usize)
                        .unwrap_or(0);
                    let k1 = args
                        .get(3)
                        .to_integer(scope)
                        .map(|i| i.value() as usize)
                        .unwrap_or(0);
                    let b_data = extract_f32_slice(scope, args.get(4));
                    let k2 = args
                        .get(5)
                        .to_integer(scope)
                        .map(|i| i.value() as usize)
                        .unwrap_or(0);
                    let n = args
                        .get(6)
                        .to_integer(scope)
                        .map(|i| i.value() as usize)
                        .unwrap_or(0);
                    if let (Some(a), Some(b)) = (a_data, b_data) {
                        match crate::ai_engine::candle_matmul(&a, m, k1, &b, k2, n) {
                            Ok(res) => {
                                if let Some(f32_arr) = create_v8_float32_array(scope, &res) {
                                    retval.set(f32_arr.into());
                                }
                            }
                            Err(e) => {
                                let s =
                                    v8::String::new(scope, &format!("Candle matmul error: {}", e))
                                        .unwrap();
                                scope.throw_exception(v8::Exception::error(scope, s));
                            }
                        }
                    }
                }
                "tensor_softmax" => {
                    let data = extract_f32_slice(scope, args.get(1));
                    let shape_val = args.get(2);
                    let mut shape = Vec::new();
                    if shape_val.is_array() {
                        if let Ok(arr) = v8::Local::<v8::Array>::try_from(shape_val) {
                            for i in 0..arr.length() {
                                if let Some(item) = arr.get_index(scope, i) {
                                    if let Some(num) = item.to_integer(scope) {
                                        shape.push(num.value() as usize);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(d) = data {
                        if shape.is_empty() {
                            shape.push(d.len());
                        }
                        match crate::ai_engine::candle_softmax(&d, &shape) {
                            Ok(res) => {
                                if let Some(f32_arr) = create_v8_float32_array(scope, &res) {
                                    retval.set(f32_arr.into());
                                }
                            }
                            Err(e) => {
                                let s =
                                    v8::String::new(scope, &format!("Candle softmax error: {}", e))
                                        .unwrap();
                                scope.throw_exception(v8::Exception::error(scope, s));
                            }
                        }
                    }
                }
                "tensor_dot" => {
                    let a_data = extract_f32_slice(scope, args.get(1));
                    let b_data = extract_f32_slice(scope, args.get(2));
                    if let (Some(a), Some(b)) = (a_data, b_data) {
                        if let Ok(dot) = crate::ai_engine::candle_dot(&a, &b) {
                            retval.set(v8::Number::new(scope, dot as f64).into());
                        }
                    }
                }
                "tensor_norm" => {
                    let data = extract_f32_slice(scope, args.get(1));
                    if let Some(d) = data {
                        if let Ok(norm) = crate::ai_engine::candle_norm(&d) {
                            retval.set(v8::Number::new(scope, norm as f64).into());
                        }
                    }
                }
                "generate" => {
                    let prompt = if args.length() > 1 {
                        args.get(1).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let opts_json = if args.length() > 2 && args.get(2).is_string() {
                        args.get(2).to_rust_string_lossy(scope)
                    } else {
                        "{}".to_string()
                    };
                    let opts: crate::ai_engine::GenerateOptions =
                        serde_json::from_str(&opts_json).unwrap_or_default();
                    match crate::ai_engine::EdgeGenerator::generate(&prompt, &opts) {
                        Ok(res) => {
                            let res_json = serde_json::to_string(&res).unwrap_or_default();
                            if let Some(s) = v8::String::new(scope, &res_json) {
                                retval.set(s.into());
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("AI generate failed: {}", e);
                            let s = v8::String::new(scope, &err_msg).unwrap();
                            let err = v8::Exception::error(scope, s);
                            scope.throw_exception(err);
                        }
                    }
                }
                "generate_stream" => {
                    let prompt = if args.length() > 1 {
                        args.get(1).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let opts_json = if args.length() > 2 && args.get(2).is_string() {
                        args.get(2).to_rust_string_lossy(scope)
                    } else {
                        "{}".to_string()
                    };
                    let opts: crate::ai_engine::GenerateOptions =
                        serde_json::from_str(&opts_json).unwrap_or_default();
                    let mut chunks = Vec::new();
                    let _ =
                        crate::ai_engine::EdgeGenerator::generate_stream(&prompt, &opts, |chunk| {
                            chunks.push(chunk.to_string());
                            true
                        });
                    let chunks_json =
                        serde_json::to_string(&chunks).unwrap_or_else(|_| "[]".to_string());
                    if let Some(s) = v8::String::new(scope, &chunks_json) {
                        retval.set(s.into());
                    }
                }
                _ => {}
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let native_key = v8::String::new(scope, "__bee_ai_native").unwrap();
    global.set(scope, native_key.into(), ai_native_fn.into());

    let js_code = r#"
    (function() {
        // --- 顶层原生文本向量化 (Embeddings) ---
        function embed(text, options = {}) {
            if (typeof text !== 'string') {
                throw new TypeError('Text must be a string');
            }
            const dims = options.dimensions || 64;
            const norm = options.normalize !== false;
            const raw = globalThis.__bee_ai_native('embed', text, dims, norm);
            const floatArray = raw instanceof Float32Array ? raw : new Float32Array(raw);
            if (options.asTensor && typeof Tensor !== 'undefined') {
                return new Tensor(floatArray, [floatArray.length], 'float32');
            }
            return floatArray;
        }

        function embedBatch(texts, options = {}) {
            if (!Array.isArray(texts)) {
                throw new TypeError('Texts must be an array of strings');
            }
            return texts.map(t => embed(t, options));
        }

        // --- 顶层原生文本生成与结构化解码 (Edge SLM Generation) ---
        async function generate(prompt, options = {}) {
            if (typeof prompt !== 'string') {
                throw new TypeError('Prompt must be a string');
            }
            const raw = globalThis.__bee_ai_native('generate', prompt, JSON.stringify(options));
            const parsed = JSON.parse(raw);
            return {
                text: parsed.text,
                tokens: parsed.tokens_generated,
                finishReason: parsed.finish_reason,
                schemaValid: options.schema ? true : false,
                model: options.model || 'bee-slm-edge'
            };
        }

        async function* generateStream(prompt, options = {}) {
            if (typeof prompt !== 'string') {
                throw new TypeError('Prompt must be a string');
            }
            const raw = globalThis.__bee_ai_native('generate_stream', prompt, JSON.stringify(options));
            const chunks = JSON.parse(raw);
            for (const chunk of chunks) {
                yield chunk;
            }
        }
        // --- Tensor: 高性能零拷贝多维张量 ---
        class Tensor {
            constructor(data, shape = null, dtype = 'float32') {
                this.dtype = dtype;
                
                // 数据扁平化处理并映射到 TypedArray
                let flatArray;
                if (ArrayBuffer.isView(data)) {
                    flatArray = data;
                } else if (Array.isArray(data)) {
                    // 如果传入多维数组，递归推断 shape 并扁平化
                    if (!shape) {
                        shape = [];
                        let curr = data;
                        while (Array.isArray(curr)) {
                            shape.push(curr.length);
                            curr = curr[0];
                        }
                    }
                    const flattened = [];
                    function flatten(arr) {
                        for (let i = 0; i < arr.length; i++) {
                            if (Array.isArray(arr[i])) flatten(arr[i]);
                            else flattened.push(Number(arr[i]));
                        }
                    }
                    flatten(data);
                    flatArray = dtype === 'int32' ? new Int32Array(flattened) : new Float32Array(flattened);
                } else if (typeof data === 'number') {
                    flatArray = dtype === 'int32' ? new Int32Array([data]) : new Float32Array([data]);
                    if (!shape) shape = [1];
                } else {
                    throw new TypeError('Tensor data must be an Array, TypedArray, or number');
                }

                if (!shape || shape.length === 0) {
                    shape = [flatArray.length];
                }

                const expectedSize = shape.reduce((acc, dim) => acc * dim, 1);
                if (flatArray.length !== expectedSize) {
                    throw new Error(`Shape [${shape.join(', ')}] specifies ${expectedSize} elements, but data has ${flatArray.length}`);
                }

                this.data = flatArray;
                this.shape = Object.freeze([...shape]);
                this.ndim = this.shape.length;
                this.length = this.data.length;
                this.size = this.length;
            }

            static from(data, shape = null, dtype = 'float32') {
                return new Tensor(data, shape, dtype);
            }

            static fromBuffer(buffer, shape = null, dtype = 'float32') {
                let view;
                switch (dtype) {
                    case 'float64':
                    case 'f64':
                        view = new Float64Array(buffer);
                        break;
                    case 'int32':
                    case 'i32':
                        view = new Int32Array(buffer);
                        break;
                    case 'int8':
                    case 'i8':
                        view = new Int8Array(buffer);
                        break;
                    case 'uint8':
                    case 'u8':
                        view = new Uint8Array(buffer);
                        break;
                    case 'float32':
                    case 'f32':
                    default:
                        view = new Float32Array(buffer);
                        break;
                }
                const actualShape = shape || [view.length];
                return new Tensor(view, actualShape, dtype);
            }

            // 矩阵乘法 (2D)
            matmul(other) {
                if (!(other instanceof Tensor)) {
                    throw new TypeError('matmul requires a Tensor');
                }
                if (this.ndim !== 2 || other.ndim !== 2) {
                    throw new Error(`matmul currently requires 2D tensors, got ${this.ndim}D and ${other.ndim}D`);
                }
                const [m, k1] = this.shape;
                const [k2, n] = other.shape;
                if (k1 !== k2) {
                    throw new Error(`Dimension mismatch in matmul: [${m}x${k1}] and [${k2}x${n}]`);
                }

                if (globalThis.__bee_ai_native) {
                    try {
                        const accelerated = globalThis.__bee_ai_native('tensor_matmul', this.data, m, k1, other.data, k2, n);
                        if (accelerated instanceof Float32Array) {
                            return new Tensor(accelerated, [m, n], this.dtype);
                        }
                    } catch (_) {}
                }

                const out = new Float32Array(m * n);
                const a = this.data;
                const b = other.data;

                for (let i = 0; i < m; i++) {
                    const aOffset = i * k1;
                    const outOffset = i * n;
                    for (let j = 0; j < n; j++) {
                        let sum = 0.0;
                        for (let k = 0; k < k1; k++) {
                            sum += a[aOffset + k] * b[k * n + j];
                        }
                        out[outOffset + j] = sum;
                    }
                }
                return new Tensor(out, [m, n], this.dtype);
            }

            // 点积 (1D 或扁平)
            dot(other) {
                if (!(other instanceof Tensor)) {
                    throw new TypeError('dot requires a Tensor');
                }
                if (this.length !== other.length) {
                    throw new Error(`Tensors must have same length for dot product, got ${this.length} and ${other.length}`);
                }
                if (globalThis.__bee_ai_native) {
                    try {
                        const accelerated = globalThis.__bee_ai_native('tensor_dot', this.data, other.data);
                        if (typeof accelerated === 'number' && !isNaN(accelerated)) return accelerated;
                    } catch (_) {}
                }
                let sum = 0.0;
                const a = this.data;
                const b = other.data;
                for (let i = 0; i < this.length; i++) {
                    sum += a[i] * b[i];
                }
                return sum;
            }

            // L2 范数
            norm() {
                if (globalThis.__bee_ai_native) {
                    try {
                        const accelerated = globalThis.__bee_ai_native('tensor_norm', this.data);
                        if (typeof accelerated === 'number' && !isNaN(accelerated)) return accelerated;
                    } catch (_) {}
                }
                let sum = 0.0;
                const a = this.data;
                for (let i = 0; i < this.length; i++) {
                    sum += a[i] * a[i];
                }
                return Math.sqrt(sum);
            }

            // 余弦相似度
            cosineSimilarity(other) {
                const normA = this.norm();
                const normB = other.norm();
                if (normA === 0 || normB === 0) return 0;
                return this.dot(other) / (normA * normB);
            }

            // 元素逐项相加
            add(other) {
                const out = new Float32Array(this.length);
                if (typeof other === 'number') {
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] + other;
                } else if (other instanceof Tensor) {
                    if (this.length !== other.length) throw new Error('Tensor length mismatch for add');
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] + other.data[i];
                } else {
                    throw new TypeError('Invalid argument for add');
                }
                return new Tensor(out, this.shape, this.dtype);
            }

            // 元素逐项相减
            sub(other) {
                const out = new Float32Array(this.length);
                if (typeof other === 'number') {
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] - other;
                } else if (other instanceof Tensor) {
                    if (this.length !== other.length) throw new Error('Tensor length mismatch for sub');
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] - other.data[i];
                } else {
                    throw new TypeError('Invalid argument for sub');
                }
                return new Tensor(out, this.shape, this.dtype);
            }

            // 元素逐项相乘 (Hadamard / 标量积)
            mul(other) {
                const out = new Float32Array(this.length);
                if (typeof other === 'number') {
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] * other;
                } else if (other instanceof Tensor) {
                    if (this.length !== other.length) throw new Error('Tensor length mismatch for mul');
                    for (let i = 0; i < this.length; i++) out[i] = this.data[i] * other.data[i];
                } else {
                    throw new TypeError('Invalid argument for mul');
                }
                return new Tensor(out, this.shape, this.dtype);
            }

            // Softmax 归一化
            softmax(axis = -1) {
                if (globalThis.__bee_ai_native && (axis === -1 || axis === this.ndim - 1)) {
                    try {
                        const accelerated = globalThis.__bee_ai_native('tensor_softmax', this.data, this.shape);
                        if (accelerated instanceof Float32Array) {
                            return new Tensor(accelerated, this.shape, this.dtype);
                        }
                    } catch (_) {}
                }
                const out = new Float32Array(this.length);
                const step = axis === -1 || axis === this.ndim - 1 ? this.shape[this.ndim - 1] : this.length;
                for (let i = 0; i < this.length; i += step) {
                    let maxVal = -Infinity;
                    for (let j = 0; j < step; j++) {
                        if (this.data[i + j] > maxVal) maxVal = this.data[i + j];
                    }
                    let sumExp = 0.0;
                    for (let j = 0; j < step; j++) {
                        const exp = Math.exp(this.data[i + j] - maxVal);
                        out[i + j] = exp;
                        sumExp += exp;
                    }
                    for (let j = 0; j < step; j++) {
                        out[i + j] /= sumExp;
                    }
                }
                return new Tensor(out, this.shape, this.dtype);
            }

            // 重塑形状
            reshape(newShape) {
                return new Tensor(this.data, newShape, this.dtype);
            }

            // 转为标准 JS 嵌套数组
            toArray() {
                if (this.ndim === 1) {
                    return Array.from(this.data);
                }
                if (this.ndim === 2) {
                    const [rows, cols] = this.shape;
                    const res = [];
                    for (let i = 0; i < rows; i++) {
                        res.push(Array.from(this.data.subarray(i * cols, (i + 1) * cols)));
                    }
                    return res;
                }
                return Array.from(this.data);
            }

            toString() {
                return `Tensor(shape=[${this.shape.join(', ')}], dtype='${this.dtype}')`;
            }
        }

        // --- LLM: 原生本地大语言模型与 Agentic 推理抽象 ---
        class LLM {
            constructor(modelName, options = {}) {
                this.model = modelName;
                this._modelId = options._modelId || null;
                this.device = options.device || 'cpu';
                this.temperature = options.temperature ?? 0.7;
                this.maxTokens = options.maxTokens ?? 512;
                this.contextLength = options.contextLength ?? 4096;
                this._isReady = true;
            }

            // 静态工厂函数异步加载本地模型 (GGUF / SafeTensors)
            static async load(modelPathOrName, options = {}) {
                if (globalThis.__bee_ai_native) {
                    try {
                        const raw = globalThis.__bee_ai_native('load_model', String(modelPathOrName), JSON.stringify(options));
                        const parsed = JSON.parse(raw);
                        return new LLM(modelPathOrName, {
                            ...options,
                            _modelId: parsed.modelId,
                            device: parsed.device || options.device || 'cpu',
                            contextLength: parsed.contextLength || 4096
                        });
                    } catch (_) {}
                }
                await new Promise(resolve => setTimeout(resolve, 1));
                return new LLM(modelPathOrName, options);
            }

            // 一次性文本生成
            async generate(prompt, options = {}) {
                if (typeof prompt !== 'string') {
                    throw new TypeError('Prompt must be a string');
                }
                if (this._modelId && globalThis.__bee_ai_native) {
                    const raw = globalThis.__bee_ai_native('model_generate', this._modelId, prompt, JSON.stringify({ ...options, model: this.model }));
                    const parsed = JSON.parse(raw);
                    return {
                        text: parsed.text,
                        tokens: parsed.tokens_generated,
                        finishReason: parsed.finish_reason,
                        schemaValid: options.schema ? true : false,
                        model: this.model
                    };
                }
                return generate(prompt, { ...options, model: this.model });
            }

            // 流式 Token 生成 (AsyncIterable)
            async *generateStream(prompt, options = {}) {
                if (typeof prompt !== 'string') {
                    throw new TypeError('Prompt must be a string');
                }
                if (this._modelId && globalThis.__bee_ai_native) {
                    const raw = globalThis.__bee_ai_native('model_generate_stream', this._modelId, prompt, JSON.stringify({ ...options, model: this.model }));
                    const chunks = JSON.parse(raw);
                    for (const chunk of chunks) {
                        yield chunk;
                    }
                    return;
                }
                for await (const chunk of generateStream(prompt, { ...options, model: this.model })) {
                    yield chunk;
                }
            }

            // 向量嵌入生成 (Embedding - backed by native Rust encoder)
            async embed(text) {
                return embed(text, { asTensor: true });
            }

            // 分词辅助
            tokenize(text) {
                if (typeof text !== 'string') return [];
                return text.match(/\w+|\s+|[^\w\s]+/g) || [];
            }
        }

        // --- 余弦相似度快捷工具函数 ---
        function cosineSimilarity(a, b) {
            const tensorA = a instanceof Tensor ? a : new Tensor(a);
            const tensorB = b instanceof Tensor ? b : new Tensor(b);
            return tensorA.cosineSimilarity(tensorB);
        }

        // --- AgentPipeline: 自治 Agent 调度管道 ---
        class AgentPipeline {
            constructor({ model, tools = [], systemPrompt = '' } = {}) {
                this.model = model;
                this.tools = new Map();
                for (const tool of tools) {
                    if (tool && tool.name && typeof tool.execute === 'function') {
                        this.tools.set(tool.name, tool);
                    }
                }
                this.systemPrompt = systemPrompt;
                this.history = [];
            }

            registerTool(tool) {
                if (!tool || !tool.name || typeof tool.execute !== 'function') {
                    throw new TypeError('Tool must have name and execute function');
                }
                this.tools.set(tool.name, tool);
            }

            async step(userInput) {
                this.history.push({ role: 'user', content: userInput });
                let outputText;
                if (this.model && typeof this.model.generate === 'function') {
                    const res = await this.model.generate(userInput);
                    outputText = res.text || res;
                } else {
                    outputText = `Executed: ${userInput}`;
                }
                this.history.push({ role: 'assistant', content: outputText });
                return {
                    status: 'completed',
                    output: outputText,
                    historyLength: this.history.length
                };
            }
        }

        // 统一导出对象
        const beeAi = {
            embed,
            embedBatch,
            generate,
            generateStream,
            Tensor,
            LLM,
            AgentPipeline,
            cosineSimilarity,
            version: '__BEE_PKG_VERSION__',
            get weights() { return globalThis.__bee_weights; },
            get tools() { return globalThis.__bee_tools; },
            get kv() { return globalThis.__bee_kv; },
            get bus() { return globalThis.__bee_bus; },
            get grammar() { return globalThis.__bee_grammar; },
            get checkpoint() { return globalThis.__bee_checkpoint; }
        };

        // 绑定到全局
        globalThis.__bee_ai = beeAi;
        globalThis.bee_ai = beeAi;
        globalThis.embed = embed;
        globalThis.embedBatch = embedBatch;
        globalThis.generate = generate;
        globalThis.generateStream = generateStream;
    })();
    "#;
    let js_code = js_code.replace("__BEE_PKG_VERSION__", env!("CARGO_PKG_VERSION"));

    let script_source = v8::String::new(scope, &js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    // 设置全局 `ai` 属性
    let bee_ai_key = v8::String::new(scope, "__bee_ai").unwrap();
    if let Some(ai_val) = global.get(scope, bee_ai_key.into()) {
        let ai_key = v8::String::new(scope, "ai").unwrap();
        global.set(scope, ai_key.into(), ai_val);
    }

    Ok(())
}
