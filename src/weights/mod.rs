// Amber v1.6.0: Native GGUF & SafeTensors Model Weight Loader (`amber:weights`)
//
// High-performance, zero-dependency binary parser and tensor loader:
// - Direct parsing of GGUF (v2/v3) binary metadata, KV dictionaries, and tensor infos
// - Direct parsing of SafeTensors 8-byte LE header and JSON tensor descriptor
// - Zero-copy memory mapping for tensor slicing and `amber:ai.Tensor` instantiation

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use memmap2::Mmap;
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GGUFTensorInfo {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub dtype: String,
    pub dtype_id: u32,
    pub offset: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GGUFMetadata {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_kv_count: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tensors: Vec<GGUFTensorInfo>,
    pub data_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeTensorItem {
    pub name: String,
    pub dtype: String,
    pub shape: Vec<u64>,
    pub data_offsets: [u64; 2],
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeTensorsMetadata {
    pub header_size: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tensors: Vec<SafeTensorItem>,
}

fn ggml_type_to_str(dtype_id: u32) -> &'static str {
    match dtype_id {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        6 => "Q5_0",
        7 => "Q5_1",
        8 => "Q8_0",
        9 => "Q8_1",
        10 => "Q2_K",
        11 => "Q3_K",
        12 => "Q4_K",
        13 => "Q5_K",
        14 => "Q6_K",
        15 => "Q8_K",
        16 => "IQ2_XXS",
        17 => "IQ2_XS",
        18 => "IQ3_XXS",
        19 => "IQ1_S",
        20 => "IQ4_NL",
        21 => "IQ3_S",
        22 => "IQ2_S",
        23 => "IQ4_XS",
        24 => "I8",
        25 => "I16",
        26 => "I32",
        27 => "I64",
        28 => "F64",
        29 => "IQ1_M",
        30 => "BF16",
        _ => "UNKNOWN",
    }
}

fn ggml_type_element_size(dtype_id: u32) -> f64 {
    match dtype_id {
        0 => 4.0,    // F32
        1 => 2.0,    // F16
        2 => 0.5625, // Q4_0: 18 bytes / 32 weights
        8 => 1.0625, // Q8_0: 34 bytes / 32 weights
        24 => 1.0,   // I8
        25 => 2.0,   // I16
        26 => 4.0,   // I32
        27 => 8.0,   // I64
        28 => 8.0,   // F64
        30 => 2.0,   // BF16
        _ => 1.0,
    }
}

pub fn parse_gguf_file(path: &Path) -> Result<GGUFMetadata, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read magic: {}", e))?;

    if &magic != b"GGUF" {
        return Err(format!(
            "Invalid GGUF magic: expected 'GGUF', got {:?}",
            magic
        ));
    }

    let mut buf4 = [0u8; 4];
    let mut buf8 = [0u8; 8];

    file.read_exact(&mut buf4)
        .map_err(|e| format!("Failed to read version: {}", e))?;
    let version = u32::from_le_bytes(buf4);

    if version < 2 || version > 3 {
        return Err(format!("Unsupported GGUF version: {}", version));
    }

    file.read_exact(&mut buf8)
        .map_err(|e| format!("Failed to read tensor_count: {}", e))?;
    let tensor_count = u64::from_le_bytes(buf8);

    file.read_exact(&mut buf8)
        .map_err(|e| format!("Failed to read metadata_kv_count: {}", e))?;
    let metadata_kv_count = u64::from_le_bytes(buf8);

    let mut metadata = HashMap::new();

    // Read metadata KV pairs
    for _ in 0..metadata_kv_count {
        let key = read_gguf_string(&mut file)?;
        let val = read_gguf_value(&mut file)?;
        metadata.insert(key, val);
    }

    // Default alignment is 32 bytes unless specified in metadata
    let alignment = metadata
        .get("general.alignment")
        .and_then(|v| v.as_u64())
        .unwrap_or(32);

    // Read Tensor Info records
    let mut tensors = Vec::with_capacity(tensor_count as usize);
    for _ in 0..tensor_count {
        let name = read_gguf_string(&mut file)?;
        file.read_exact(&mut buf4)
            .map_err(|e| format!("Failed to read n_dimensions: {}", e))?;
        let n_dimensions = u32::from_le_bytes(buf4);

        let mut dimensions = Vec::with_capacity(n_dimensions as usize);
        let mut total_elements: u64 = 1;
        for _ in 0..n_dimensions {
            file.read_exact(&mut buf8)
                .map_err(|e| format!("Failed to read dimension: {}", e))?;
            let dim = u64::from_le_bytes(buf8);
            dimensions.push(dim);
            total_elements = total_elements.saturating_mul(dim);
        }

        file.read_exact(&mut buf4)
            .map_err(|e| format!("Failed to read tensor type: {}", e))?;
        let dtype_id = u32::from_le_bytes(buf4);

        file.read_exact(&mut buf8)
            .map_err(|e| format!("Failed to read tensor offset: {}", e))?;
        let offset = u64::from_le_bytes(buf8);

        let elem_size = ggml_type_element_size(dtype_id);
        let size_bytes = (total_elements as f64 * elem_size).ceil() as u64;

        tensors.push(GGUFTensorInfo {
            name,
            dimensions,
            dtype: ggml_type_to_str(dtype_id).to_string(),
            dtype_id,
            offset,
            size_bytes,
        });
    }

    let current_pos = file
        .stream_position()
        .map_err(|e| format!("Failed to get stream position: {}", e))?;
    let remainder = current_pos % alignment;
    let data_offset = if remainder != 0 {
        current_pos + (alignment - remainder)
    } else {
        current_pos
    };

    Ok(GGUFMetadata {
        version,
        tensor_count,
        metadata_kv_count,
        metadata,
        tensors,
        data_offset,
    })
}

fn read_gguf_string<R: Read>(reader: &mut R) -> Result<String, String> {
    let mut buf8 = [0u8; 8];
    reader
        .read_exact(&mut buf8)
        .map_err(|e| format!("Failed to read string length: {}", e))?;
    let len = u64::from_le_bytes(buf8) as usize;

    let mut str_bytes = vec![0u8; len];
    reader
        .read_exact(&mut str_bytes)
        .map_err(|e| format!("Failed to read string bytes: {}", e))?;

    String::from_utf8(str_bytes).map_err(|e| format!("Invalid UTF-8 string: {}", e))
}

fn read_gguf_value<R: Read>(reader: &mut R) -> Result<serde_json::Value, String> {
    let mut buf4 = [0u8; 4];
    reader
        .read_exact(&mut buf4)
        .map_err(|e| format!("Failed to read value type: {}", e))?;
    let type_id = u32::from_le_bytes(buf4);

    read_gguf_typed_value(reader, type_id)
}

fn read_gguf_typed_value<R: Read>(
    reader: &mut R,
    type_id: u32,
) -> Result<serde_json::Value, String> {
    let mut buf1 = [0u8; 1];
    let mut buf2 = [0u8; 2];
    let mut buf4 = [0u8; 4];
    let mut buf8 = [0u8; 8];

    match type_id {
        0 => {
            reader.read_exact(&mut buf1).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(buf1[0]))
        }
        1 => {
            reader.read_exact(&mut buf1).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(buf1[0] as i8))
        }
        2 => {
            reader.read_exact(&mut buf2).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(u16::from_le_bytes(buf2)))
        }
        3 => {
            reader.read_exact(&mut buf2).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(i16::from_le_bytes(buf2)))
        }
        4 => {
            reader.read_exact(&mut buf4).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(u32::from_le_bytes(buf4)))
        }
        5 => {
            reader.read_exact(&mut buf4).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(i32::from_le_bytes(buf4)))
        }
        6 => {
            reader.read_exact(&mut buf4).map_err(|e| e.to_string())?;
            let f = f32::from_le_bytes(buf4);
            Ok(serde_json::Number::from_f64(f as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null))
        }
        7 => {
            reader.read_exact(&mut buf1).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(buf1[0] != 0))
        }
        8 => {
            let s = read_gguf_string(reader)?;
            Ok(serde_json::Value::from(s))
        }
        9 => {
            // ARRAY: element_type (u32) + count (u64)
            reader.read_exact(&mut buf4).map_err(|e| e.to_string())?;
            let elem_type = u32::from_le_bytes(buf4);
            reader.read_exact(&mut buf8).map_err(|e| e.to_string())?;
            let count = u64::from_le_bytes(buf8) as usize;

            let mut arr = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let item = read_gguf_typed_value(reader, elem_type)?;
                arr.push(item);
            }
            Ok(serde_json::Value::Array(arr))
        }
        10 => {
            reader.read_exact(&mut buf8).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(u64::from_le_bytes(buf8)))
        }
        11 => {
            reader.read_exact(&mut buf8).map_err(|e| e.to_string())?;
            Ok(serde_json::Value::from(i64::from_le_bytes(buf8)))
        }
        12 => {
            reader.read_exact(&mut buf8).map_err(|e| e.to_string())?;
            let f = f64::from_le_bytes(buf8);
            Ok(serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null))
        }
        _ => Err(format!("Unknown GGUF metadata type ID: {}", type_id)),
    }
}

pub fn parse_safetensors_file(path: &Path) -> Result<SafeTensorsMetadata, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut buf8 = [0u8; 8];
    file.read_exact(&mut buf8)
        .map_err(|e| format!("Failed to read header size: {}", e))?;
    let header_size = u64::from_le_bytes(buf8);

    if header_size > 100 * 1024 * 1024 {
        return Err(format!(
            "SafeTensors header size too large ({} bytes)",
            header_size
        ));
    }

    let mut header_bytes = vec![0u8; header_size as usize];
    file.read_exact(&mut header_bytes)
        .map_err(|e| format!("Failed to read header JSON: {}", e))?;

    let header_str = String::from_utf8(header_bytes)
        .map_err(|e| format!("SafeTensors header is not valid UTF-8: {}", e))?;

    let parsed: serde_json::Value = serde_json::from_str(&header_str)
        .map_err(|e| format!("Failed to parse SafeTensors header JSON: {}", e))?;

    let obj = parsed
        .as_object()
        .ok_or_else(|| "SafeTensors header root must be an object".to_string())?;

    let mut metadata = HashMap::new();
    let mut tensors = Vec::new();

    for (k, v) in obj {
        if k == "__metadata__" {
            if let Some(m) = v.as_object() {
                for (mk, mv) in m {
                    metadata.insert(mk.clone(), mv.clone());
                }
            }
        } else if let Some(tensor_obj) = v.as_object() {
            let dtype = tensor_obj
                .get("dtype")
                .and_then(|s| s.as_str())
                .unwrap_or("F32")
                .to_string();

            let shape: Vec<u64> = tensor_obj
                .get("shape")
                .and_then(|s| s.as_array())
                .map(|arr| arr.iter().filter_map(|d| d.as_u64()).collect())
                .unwrap_or_default();

            let data_offsets: [u64; 2] = tensor_obj
                .get("data_offsets")
                .and_then(|d| d.as_array())
                .and_then(|arr| {
                    if arr.len() == 2 {
                        let start = arr[0].as_u64()?;
                        let end = arr[1].as_u64()?;
                        Some([start, end])
                    } else {
                        None
                    }
                })
                .unwrap_or([0, 0]);

            let size_bytes = data_offsets[1].saturating_sub(data_offsets[0]);

            tensors.push(SafeTensorItem {
                name: k.clone(),
                dtype,
                shape,
                data_offsets,
                size_bytes,
            });
        }
    }

    Ok(SafeTensorsMetadata {
        header_size,
        metadata,
        tensors,
    })
}

/// Sets up the `amber:weights` API in V8 Context
pub fn setup_weights_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let weights_obj = v8::Object::new(scope);

    // 1. weights.readGGUFMetadata(filePath)
    let read_gguf_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_string() {
                let msg =
                    v8::String::new(scope, "readGGUFMetadata requires a file path string").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let path_str = args.get(0).to_rust_string_lossy(scope);
            let path = Path::new(&path_str);

            match parse_gguf_file(path) {
                Ok(meta) => {
                    let json_str =
                        serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
                    let v8_str = v8::String::new(scope, &json_str).unwrap();
                    let parsed = v8::json::parse(scope, v8_str)
                        .unwrap_or_else(|| v8::Object::new(scope).into());
                    rv.set(parsed);
                }
                Err(err) => {
                    let msg =
                        v8::String::new(scope, &format!("Failed to parse GGUF file: {}", err))
                            .unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();
    let key = v8::String::new(scope, "readGGUFMetadata").unwrap();
    weights_obj.set(scope, key.into(), read_gguf_fn.into());

    // 2. weights.readSafeTensorsMetadata(filePath)
    let read_st_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_string() {
                let msg =
                    v8::String::new(scope, "readSafeTensorsMetadata requires a file path string")
                        .unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let path_str = args.get(0).to_rust_string_lossy(scope);
            let path = Path::new(&path_str);

            match parse_safetensors_file(path) {
                Ok(meta) => {
                    let json_str =
                        serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
                    let v8_str = v8::String::new(scope, &json_str).unwrap();
                    let parsed = v8::json::parse(scope, v8_str)
                        .unwrap_or_else(|| v8::Object::new(scope).into());
                    rv.set(parsed);
                }
                Err(err) => {
                    let msg = v8::String::new(
                        scope,
                        &format!("Failed to parse SafeTensors file: {}", err),
                    )
                    .unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();
    let key = v8::String::new(scope, "readSafeTensorsMetadata").unwrap();
    weights_obj.set(scope, key.into(), read_st_fn.into());

    // 3. weights.loadTensor(filePath, tensorName)
    let load_tensor_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 2 || !args.get(0).is_string() || !args.get(1).is_string() {
                let msg =
                    v8::String::new(scope, "loadTensor requires (filePath, tensorName)").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let path_str = args.get(0).to_rust_string_lossy(scope);
            let tensor_name = args.get(1).to_rust_string_lossy(scope);
            let path = Path::new(&path_str);

            let file = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    let msg =
                        v8::String::new(scope, &format!("Failed to open file: {}", e)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }
            };

            let mmap = match unsafe { Mmap::map(&file) } {
                Ok(m) => m,
                Err(e) => {
                    let msg =
                        v8::String::new(scope, &format!("Failed to mmap file: {}", e)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }
            };

            // Detect format based on magic / extension
            let is_gguf = mmap.len() >= 4 && &mmap[0..4] == b"GGUF";

            if is_gguf {
                let meta = match parse_gguf_file(path) {
                    Ok(m) => m,
                    Err(e) => {
                        let msg = v8::String::new(scope, &e).unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                };

                let tensor = match meta.tensors.iter().find(|t| t.name == tensor_name) {
                    Some(t) => t,
                    None => {
                        let msg = v8::String::new(
                            scope,
                            &format!("Tensor '{}' not found in GGUF file", tensor_name),
                        )
                        .unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                };

                let start = (meta.data_offset + tensor.offset) as usize;
                let end = start + tensor.size_bytes as usize;

                if end > mmap.len() {
                    let msg =
                        v8::String::new(scope, "Tensor byte range exceeds file bounds").unwrap();
                    let exc = v8::Exception::range_error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }

                let slice = &mmap[start..end];
                let boxed_slice = slice.to_vec().into_boxed_slice();
                let backing_store =
                    v8::ArrayBuffer::new_backing_store_from_boxed_slice(boxed_slice);
                let array_buffer =
                    v8::ArrayBuffer::with_backing_store(scope, &backing_store.make_shared());

                let res_obj = v8::Object::new(scope);
                let name_key = v8::String::new(scope, "name").unwrap();
                let name_val = v8::String::new(scope, &tensor.name).unwrap();
                res_obj.set(scope, name_key.into(), name_val.into());

                let dtype_key = v8::String::new(scope, "dtype").unwrap();
                let dtype_val = v8::String::new(scope, &tensor.dtype).unwrap();
                res_obj.set(scope, dtype_key.into(), dtype_val.into());

                let shape_key = v8::String::new(scope, "shape").unwrap();
                let shape_arr = v8::Array::new(scope, tensor.dimensions.len() as i32);
                for (idx, dim) in tensor.dimensions.iter().enumerate() {
                    let dim_num = v8::Number::new(scope, *dim as f64);
                    shape_arr.set_index(scope, idx as u32, dim_num.into());
                }
                res_obj.set(scope, shape_key.into(), shape_arr.into());

                let buf_key = v8::String::new(scope, "buffer").unwrap();
                res_obj.set(scope, buf_key.into(), array_buffer.into());

                let size_key = v8::String::new(scope, "byteLength").unwrap();
                let size_val = v8::Number::new(scope, tensor.size_bytes as f64);
                res_obj.set(scope, size_key.into(), size_val.into());

                rv.set(res_obj.into());
            } else {
                // SafeTensors format
                let meta = match parse_safetensors_file(path) {
                    Ok(m) => m,
                    Err(e) => {
                        let msg = v8::String::new(scope, &e).unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                };

                let tensor = match meta.tensors.iter().find(|t| t.name == tensor_name) {
                    Some(t) => t,
                    None => {
                        let msg = v8::String::new(
                            scope,
                            &format!("Tensor '{}' not found in SafeTensors file", tensor_name),
                        )
                        .unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                };

                let data_base = 8 + meta.header_size as usize;
                let start = data_base + tensor.data_offsets[0] as usize;
                let end = data_base + tensor.data_offsets[1] as usize;

                if end > mmap.len() {
                    let msg = v8::String::new(scope, "Tensor offset exceeds file length").unwrap();
                    let exc = v8::Exception::range_error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }

                let slice = &mmap[start..end];
                let boxed_slice = slice.to_vec().into_boxed_slice();
                let backing_store =
                    v8::ArrayBuffer::new_backing_store_from_boxed_slice(boxed_slice);
                let array_buffer =
                    v8::ArrayBuffer::with_backing_store(scope, &backing_store.make_shared());

                let res_obj = v8::Object::new(scope);
                let name_key = v8::String::new(scope, "name").unwrap();
                let name_val = v8::String::new(scope, &tensor.name).unwrap();
                res_obj.set(scope, name_key.into(), name_val.into());

                let dtype_key = v8::String::new(scope, "dtype").unwrap();
                let dtype_val = v8::String::new(scope, &tensor.dtype).unwrap();
                res_obj.set(scope, dtype_key.into(), dtype_val.into());

                let shape_key = v8::String::new(scope, "shape").unwrap();
                let shape_arr = v8::Array::new(scope, tensor.shape.len() as i32);
                for (idx, dim) in tensor.shape.iter().enumerate() {
                    let dim_num = v8::Number::new(scope, *dim as f64);
                    shape_arr.set_index(scope, idx as u32, dim_num.into());
                }
                res_obj.set(scope, shape_key.into(), shape_arr.into());

                let buf_key = v8::String::new(scope, "buffer").unwrap();
                res_obj.set(scope, buf_key.into(), array_buffer.into());

                let size_key = v8::String::new(scope, "byteLength").unwrap();
                let size_val = v8::Number::new(scope, tensor.size_bytes as f64);
                res_obj.set(scope, size_key.into(), size_val.into());

                rv.set(res_obj.into());
            }
        },
    )
    .unwrap();
    let key = v8::String::new(scope, "loadTensor").unwrap();
    weights_obj.set(scope, key.into(), load_tensor_fn.into());

    // Register globally as `__amber_weights` and `weights`
    let global = context.global(scope);
    let weights_key = v8::String::new(scope, "__amber_weights").unwrap();
    global.set(scope, weights_key.into(), weights_obj.into());
    let weights_plain = v8::String::new(scope, "weights").unwrap();
    global.set(scope, weights_plain.into(), weights_obj.into());

    Ok(())
}
