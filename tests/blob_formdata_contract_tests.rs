//! Pins docs/BLOB_FORMDATA_CONTRACT.md.
//! Blob, File, and FormData on the default runtime, plus FormData as a fetch body.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

fn run(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

fn http_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(b"\r\n\r\n".len())
        .position(|window| window == b"\r\n\r\n")
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut request = Vec::new();
    let mut expected_len = None;
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read_len) => {
                request.extend_from_slice(&buffer[..read_len]);
                if let Some(header_end) = http_header_end(&request) {
                    if expected_len.is_none() {
                        let headers = String::from_utf8_lossy(&request[..header_end]);
                        expected_len = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        });
                    }
                    if let Some(content_len) = expected_len {
                        if request.len() >= header_end + b"\r\n\r\n".len() + content_len {
                            break;
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
    request
}

fn header_value<'a>(request: &'a str, name: &str) -> &'a str {
    request
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.eq_ignore_ascii_case(name) {
                Some(value.trim())
            } else {
                None
            }
        })
        .unwrap_or("")
}

fn disposition_line<'a>(request: &'a str, field: &str) -> &'a str {
    let needle = format!("name=\"{field}\"");
    request
        .lines()
        .find(|line| line.contains(&needle))
        .unwrap_or("")
}

#[test]
#[serial]
fn blob_constructor_slice_text_and_array_buffer() {
    let output = run(r#"
        const empty = new Blob();
        const fromNull = new Blob(null);
        const bare = new Blob('ab');
        const typed = new Blob([], { type: 'Text/Plain' });
        const mixed = new Blob([
            'ab',
            new Uint8Array([9, 8, 7, 6]).subarray(1, 3),
            new Uint8Array([1, 2]).buffer,
            1,
            { skipped: true }
        ], { type: 'text/plain' });
        const nested = new Blob([
            new Blob(['ab']),
            'c',
            new File(['d'], 'f.txt'),
            new Uint8Array([101])
        ]);
        const binary = new Blob([new Uint8Array([0xff, 0xfe])]);
        const binaryBytes = Array.from(new Uint8Array(binary.arrayBuffer())).join(',');
        const endings = new Blob(['a\r\nb'], { endings: 'native' });
        const source = new Blob(['Hello, World!'], { type: 'text/plain' });
        source.type = 'Text/Custom';
        source.size = 1;
        const sliced = source.slice(1, 3);
        const overridden = new Blob(['Hello, World!'], { type: 'text/plain' }).slice(0, 5, 'text/html');
        const cleared = new Blob(['Hello'], { type: 'text/plain' }).slice(0, 1, 5);
        const tail = new Blob(['Hello, World!']).slice(-6);
        const emptySlice = new Blob(['Hello, World!']).slice(5, 2);
        const kept = Array.from(new Uint8Array(source.arrayBuffer())).length;
        [
            empty.size,
            fromNull.size,
            bare.text(),
            bare.size,
            typed.type,
            mixed.size,
            Array.from(new Uint8Array(mixed.arrayBuffer())).join(','),
            nested.text(),
            nested.size,
            nested instanceof Blob,
            binary.size,
            binaryBytes,
            binary.text().length,
            binary.text().charCodeAt(0),
            binary.text().charCodeAt(1),
            endings.text() === 'a\r\nb',
            typeof source.text(),
            source.text() instanceof Promise,
            source.arrayBuffer() instanceof ArrayBuffer,
            kept,
            source.text(),
            sliced.text(),
            sliced.type,
            sliced instanceof Blob,
            overridden.type,
            overridden.text(),
            cleared.type,
            tail.text(),
            emptySlice.size
        ].join('|');
        "#);
    assert_eq!(
        output,
        "0|0|ab|2|Text/Plain|6|97,98,8,7,1,2|abcde|5|true|2|255,254|2|65533|65533|true|string|false|true|13|Hello, World!|el|Text/Custom|true|text/html|Hello||World!|0",
        "blob constructor, slice, text, and arrayBuffer: {output}"
    );
}

#[test]
#[serial]
fn file_constructor_and_slice() {
    let output = run(r#"
        const bare = new File('ab', 'ignored.txt');
        const missingName = new File(['x']);
        const file = new File(['file'], 'doc.txt', { type: 'Text/Plain', lastModified: 0 });
        const now = new File(['x'], 'now.txt');
        const sliced = file.slice(0, 2);
        [
            bare.size,
            missingName.name,
            file.name,
            file.size,
            file.type,
            file.lastModified,
            file instanceof File,
            file instanceof Blob,
            typeof file.text,
            file.text(),
            now.lastModified > 1e12,
            sliced instanceof File,
            sliced instanceof Blob,
            sliced.text(),
            sliced.type,
            'name' in sliced,
            'lastModified' in sliced
        ].join('|');
        "#);
    assert_eq!(
        output,
        "0||doc.txt|4|Text/Plain|0|true|true|function|file|true|true|true|fi|Text/Plain|false|false",
        "File constructor and slice: {output}"
    );
}

#[test]
#[serial]
fn form_data_append_get_delete_set_and_entries() {
    let output = run(r#"
        const ignored = new FormData('nope');
        const fd = new FormData();
        fd.append('tag', 'a');
        fd.append('tag', 'b');
        fd.append('name', 'amber');
        fd.append('n', 12);
        fd.append('flag', true);
        fd.append('blank', '');
        fd.append('skip', null);
        fd.append('skip2', undefined);
        fd.append('bytes', new Uint8Array([1, 2]));
        fd.delete('tag');
        const order = new FormData();
        order.append('a', '1');
        order.append('b', '2');
        order.append('a', '3');
        order.set('a', '9');
        const iter = new FormData();
        iter.append('tag', 'a');
        iter.append('tag', 'b');
        iter.append('name', 'amber');
        const keys = iter.keys();
        const firstKey = keys.next();
        const secondKey = keys.next();
        const thirdKey = keys.next();
        const doneKey = keys.next();
        const values = Array.from(iter.values()).join(',');
        const entries = Array.from(iter.entries()).map(([name, value]) => name + '=' + value).join(',');
        const direct = Array.from(iter).map(([name, value]) => name + '=' + value).join(',');
        const seen = [];
        iter.forEach(function(value, name, owner) {
            seen.push(this.label + ':' + name + '=' + value + ':' + (owner === iter));
        }, { label: 'ctx' });
        [
            ignored instanceof FormData,
            ignored.has('nope'),
            fd instanceof FormData,
            fd.has('tag'),
            fd.has('name'),
            fd.has('skip'),
            fd.has('skip2'),
            fd.get('name'),
            fd.get('missing') === null,
            JSON.stringify(fd.getAll('name')),
            fd.getAll('missing').length,
            fd.get('n'),
            fd.get('flag'),
            fd.get('blank'),
            fd.get('bytes'),
            Array.from(order.entries()).map(([name, value]) => name + '=' + value).join(','),
            order.getAll('a').length,
            typeof keys.next,
            firstKey.value,
            secondKey.value,
            thirdKey.value,
            doneKey.done,
            values,
            entries,
            direct,
            seen.join(',')
        ].join('|');
        "#);
    assert_eq!(
        output,
        "true|false|true|false|true|false|false|amber|true|[\"amber\"]|0|12|true||1,2|b=2,a=9|1|function|tag|tag|name|true|a,b,amber|tag=a,tag=b,name=amber|tag=a,tag=b,name=amber|ctx:tag=a:true,ctx:tag=b:true,ctx:name=amber:true",
        "FormData append/get/delete/set/entries: {output}"
    );
}

#[test]
#[serial]
fn form_data_get_returns_text_for_blob_and_file() {
    let output = run(r#"
        const fd = new FormData();
        fd.append('blobField', new Blob(['blob payload'], { type: 'text/plain' }), 'blob.txt');
        fd.append('fileField', new File(['file payload'], 'file.txt', { type: 'text/custom' }));
        fd.append('bin', new Blob([new Uint8Array([0xff, 0xfe])]));
        const bin = fd.get('bin');
        [
            typeof fd.get('blobField'),
            fd.get('blobField') instanceof Blob,
            fd.get('blobField'),
            fd.get('fileField') instanceof File,
            fd.get('fileField'),
            JSON.stringify(fd.getAll('blobField')),
            bin.length,
            bin.charCodeAt(0),
            bin.charCodeAt(1),
            Array.from(fd.entries()).map(([name, value]) => name + ':' + typeof value).join(',')
        ].join('|');
        "#);
    assert_eq!(
        output,
        "string|false|blob payload|false|file payload|[\"blob payload\"]|2|65533|65533|blobField:string,fileField:string,bin:string",
        "FormData get returns text for Blob and File: {output}"
    );
}

#[test]
#[serial]
fn fetch_form_data_body_is_multipart() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let request_bytes = read_http_request(&mut stream);
        let request = String::from_utf8_lossy(&request_bytes);
        let content_type = header_value(&request, "content-type");
        let note = disposition_line(&request, "note");
        let named = disposition_line(&request, "named");
        let blob_field = disposition_line(&request, "blobField");
        let file_field = disposition_line(&request, "fileField");
        let bytes_field = disposition_line(&request, "bytesField");
        let plain_blob = disposition_line(&request, "plainBlob");
        let empty_file = disposition_line(&request, "emptyFile");
        let has_binary = request_bytes
            .windows(3)
            .any(|window| window == [0, 255, 65]);
        let ok = content_type.starts_with("multipart/form-data; boundary=----AmberFormBoundary")
            && note.contains("name=\"note\"")
            && !note.contains("filename=")
            && request.contains("Content-Type: text/plain")
            && request.contains("alpha")
            && named.contains("filename=\"x.txt\"")
            && blob_field.contains("filename=\"blob.txt\"")
            && request.contains("blob payload")
            && file_field.contains("filename=\"file.txt\"")
            && request.contains("Content-Type: text/custom")
            && request.contains("file payload")
            && bytes_field.contains("filename=\"bytes.bin\"")
            && request.contains("Content-Type: application/octet-stream")
            && has_binary
            && plain_blob.contains("filename=\"blob\"")
            && empty_file.contains("name=\"emptyFile\"")
            && !empty_file.contains("filename=")
            && request.contains("\r\n--")
            && request.contains("--\r\n");
        let body = if ok {
            "ok".to_string()
        } else {
            format!("bad multipart content-type={content_type}; note={note}; named={named}; blob={blob_field}; file={file_field}; bytes={bytes_field}; plain={plain_blob}; empty={empty_file}; binary={has_binary}; request={request}")
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });

    let url = format!("http://{address}");
    let output = run(&format!(
        r#"
        const formData = new FormData();
        formData.append('note', 'alpha');
        formData.append('named', 'body', 'x.txt');
        formData.append('blobField', new Blob(['blob payload'], {{ type: 'text/plain' }}), 'blob.txt');
        formData.append('fileField', new File(['file payload'], 'file.txt', {{ type: 'text/custom' }}));
        formData.append('bytesField', new Blob([new Uint8Array([0, 255, 65])], {{ type: 'application/octet-stream' }}), 'bytes.bin');
        formData.append('plainBlob', new Blob(['p']));
        formData.append('emptyFile', new File(['z'], ''));
        const response = fetch({url:?}, {{ method: 'POST', body: formData }});
        response.text();
        "#
    ));
    assert_eq!(output, "ok", "FormData fetch body: {output}");
}

#[test]
#[serial]
fn fetch_form_data_keeps_caller_content_type() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let request_bytes = read_http_request(&mut stream);
        let request = String::from_utf8_lossy(&request_bytes);
        let content_type = header_value(&request, "content-type");
        let multipart_body = request.contains("----AmberFormBoundary")
            && request.contains("name=\"note\"")
            && request.contains("alpha");
        let body = format!("{content_type}|{multipart_body}");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });

    let url = format!("http://{address}");
    let output = run(&format!(
        r#"
        const formData = new FormData();
        formData.append('note', 'alpha');
        const response = fetch({url:?}, {{
            method: 'POST',
            headers: {{ 'Content-Type': 'text/plain' }},
            body: formData
        }});
        response.text();
        "#
    ));
    assert_eq!(
        output, "text/plain|true",
        "caller Content-Type with a FormData body: {output}"
    );
}
