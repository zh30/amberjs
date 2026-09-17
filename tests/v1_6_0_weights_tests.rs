// Tests for Amber v1.6.0 Native GGUF & SafeTensors Model Weights Loader (`amber:weights`)

use std::fs::File;
use std::io::{Seek, Write};
use tempfile::tempdir;

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn create_mock_gguf_file(path: &std::path::Path) {
    let mut file = File::create(path).expect("Failed to create GGUF test file");

    // 1. Header
    file.write_all(b"GGUF").unwrap(); // magic
    file.write_all(&3u32.to_le_bytes()).unwrap(); // version = 3
    file.write_all(&1u64.to_le_bytes()).unwrap(); // tensor_count = 1
    file.write_all(&2u64.to_le_bytes()).unwrap(); // metadata_kv_count = 2

    // 2. Metadata KV 1: "general.architecture" -> STRING "llama"
    let key1 = "general.architecture";
    file.write_all(&(key1.len() as u64).to_le_bytes()).unwrap();
    file.write_all(key1.as_bytes()).unwrap();
    file.write_all(&8u32.to_le_bytes()).unwrap(); // type 8 = STRING
    let val1 = "llama";
    file.write_all(&(val1.len() as u64).to_le_bytes()).unwrap();
    file.write_all(val1.as_bytes()).unwrap();

    // 3. Metadata KV 2: "general.alignment" -> UINT32 32
    let key2 = "general.alignment";
    file.write_all(&(key2.len() as u64).to_le_bytes()).unwrap();
    file.write_all(key2.as_bytes()).unwrap();
    file.write_all(&4u32.to_le_bytes()).unwrap(); // type 4 = UINT32
    file.write_all(&32u32.to_le_bytes()).unwrap();

    // 4. Tensor 1: "token_embd.weight" (shape: [4, 2], F32, offset: 0)
    let tensor_name = "token_embd.weight";
    file.write_all(&(tensor_name.len() as u64).to_le_bytes())
        .unwrap();
    file.write_all(tensor_name.as_bytes()).unwrap();
    file.write_all(&2u32.to_le_bytes()).unwrap(); // 2 dimensions
    file.write_all(&4u64.to_le_bytes()).unwrap(); // dim 0 = 4
    file.write_all(&2u64.to_le_bytes()).unwrap(); // dim 1 = 2
    file.write_all(&0u32.to_le_bytes()).unwrap(); // dtype 0 = F32
    file.write_all(&0u64.to_le_bytes()).unwrap(); // offset = 0

    // 5. Align to 32 bytes
    let pos = file.stream_position().unwrap();
    let rem = pos % 32;
    if rem != 0 {
        let pad = 32 - rem;
        file.write_all(&vec![0u8; pad as usize]).unwrap();
    }

    // 6. Tensor data (8 * 4 = 32 bytes of float32 values)
    let values: [f32; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    for v in values {
        file.write_all(&v.to_le_bytes()).unwrap();
    }
}

fn create_mock_safetensors_file(path: &std::path::Path) {
    let mut file = File::create(path).expect("Failed to create SafeTensors test file");

    let header_json = r#"{"__metadata__":{"format":"pt"},"layer1.weight":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]}}"#;
    let header_bytes = header_json.as_bytes();
    let header_len = header_bytes.len() as u64;

    file.write_all(&header_len.to_le_bytes()).unwrap();
    file.write_all(header_bytes).unwrap();

    let values: [f32; 4] = [1.5, 2.5, 3.5, 4.5];
    for v in values {
        file.write_all(&v.to_le_bytes()).unwrap();
    }
}

#[test]
#[serial]
fn test_gguf_metadata_parsing_and_tensor_loading() {
    let dir = tempdir().expect("tempdir");
    let gguf_path = dir.path().join("model.gguf");
    create_mock_gguf_file(&gguf_path);

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
        const weights = require('amber:weights');
        const weightsAlias = require('weights');
        if (weights !== weightsAlias) throw new Error('Alias mismatch');

        const meta = weights.readGGUFMetadata("{path}");
        if (meta.version !== 3) throw new Error('Expected version 3, got ' + meta.version);
        if (meta.tensor_count !== 1) throw new Error('Expected 1 tensor, got ' + meta.tensor_count);
        if (meta.metadata['general.architecture'] !== 'llama') {{
            throw new Error('Architecture mismatch');
        }}

        const tensorInfo = meta.tensors[0];
        if (tensorInfo.name !== 'token_embd.weight') throw new Error('Name mismatch');
        if (tensorInfo.dtype !== 'F32') throw new Error('Dtype mismatch');

        // Load tensor
        const tensor = weights.loadTensor("{path}", "token_embd.weight");
        if (tensor.name !== 'token_embd.weight') throw new Error('Loaded name mismatch');
        if (tensor.byteLength !== 32) throw new Error('Byte length mismatch: ' + tensor.byteLength);

        // Verify float values
        const floatView = new Float32Array(tensor.buffer);
        if (floatView.length !== 8) throw new Error('Float view length mismatch: ' + floatView.length);
        if (floatView[0] !== 1.0 || floatView[7] !== 8.0) {{
            throw new Error('Values mismatch: ' + floatView.join(', '));
        }}

        JSON.stringify({{ success: true, count: floatView.length }});
        "#,
        path = gguf_path.to_string_lossy().replace('\\', "\\\\")
    );

    let res = runtime.execute_code(&code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_safetensors_metadata_and_tensor_loading() {
    let dir = tempdir().expect("tempdir");
    let st_path = dir.path().join("model.safetensors");
    create_mock_safetensors_file(&st_path);

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
        const weights = require('amber:weights');
        const ai = require('amber:ai');

        const meta = weights.readSafeTensorsMetadata("{path}");
        if (meta.metadata.format !== 'pt') throw new Error('Format mismatch');
        if (meta.tensors.length !== 1) throw new Error('Tensors count mismatch');

        const item = meta.tensors[0];
        if (item.name !== 'layer1.weight') throw new Error('Tensor name mismatch');
        if (item.size_bytes !== 16) throw new Error('Size mismatch');

        // Load tensor
        const loaded = weights.loadTensor("{path}", "layer1.weight");
        const f32 = new Float32Array(loaded.buffer);
        if (f32[0] !== 1.5 || f32[3] !== 4.5) throw new Error('Data mismatch');

        // Use Tensor.fromBuffer
        const tensorObj = ai.Tensor.fromBuffer(loaded.buffer, loaded.shape, 'float32');
        if (tensorObj.size !== 4) throw new Error('Tensor.fromBuffer size mismatch: ' + tensorObj.size);

        JSON.stringify({{ success: true, shape: tensorObj.shape }});
        "#,
        path = st_path.to_string_lossy().replace('\\', "\\\\")
    );

    let res = runtime.execute_code(&code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
