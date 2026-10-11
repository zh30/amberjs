// Node.js StringDecoder 模块实现
// UTF-8 (G39) + multi-encoding write/end buffering (G68)

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_string_decoder_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    let js_code = r#"
    (function() {
        const B64 =
            'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

        function normalizeEncoding(encoding) {
            let enc = String(encoding == null ? 'utf8' : encoding).toLowerCase();
            if (enc === 'utf-8') enc = 'utf8';
            else if (enc === 'ucs2' || enc === 'utf-16le') enc = 'utf16le';
            else if (enc === 'binary') enc = 'latin1';
            return enc;
        }

        function isSupportedEncoding(enc) {
            return (
                enc === 'utf8' ||
                enc === 'utf16le' ||
                enc === 'latin1' ||
                enc === 'ascii' ||
                enc === 'base64' ||
                enc === 'hex'
            );
        }

        function bytesFromInput(buffer) {
            if (typeof buffer === 'string') return null; // signal passthrough
            if (buffer instanceof Uint8Array) {
                return Array.from(buffer);
            }
            if (buffer && buffer.buffer instanceof ArrayBuffer) {
                const u8 = new Uint8Array(
                    buffer.buffer,
                    buffer.byteOffset || 0,
                    buffer.byteLength || buffer.length || 0
                );
                return Array.from(u8);
            }
            if (Array.isArray(buffer)) {
                return buffer.slice();
            }
            return [];
        }

        function decodeUtf8Complete(bytes) {
            return new TextDecoder('utf8').decode(new Uint8Array(bytes));
        }

        function utf8IncompleteTrail(allBytes) {
            let incompleteBytes = 0;
            for (let check = 1; check <= 4 && check <= allBytes.length; check++) {
                const b = allBytes[allBytes.length - check];
                if ((b & 0x80) === 0) {
                    break;
                } else if ((b & 0xC0) === 0xC0) {
                    let needed = 0;
                    if ((b & 0xE0) === 0xC0) needed = 2;
                    else if ((b & 0xF0) === 0xE0) needed = 3;
                    else if ((b & 0xF8) === 0xF0) needed = 4;

                    if (check < needed) {
                        incompleteBytes = check;
                    }
                    break;
                }
            }
            return incompleteBytes;
        }

        function decodeLatin1(bytes) {
            let out = '';
            for (let i = 0; i < bytes.length; i++) {
                out += String.fromCharCode(bytes[i] & 0xff);
            }
            return out;
        }

        function decodeAscii(bytes) {
            let out = '';
            for (let i = 0; i < bytes.length; i++) {
                out += String.fromCharCode(bytes[i] & 0x7f);
            }
            return out;
        }

        function decodeHex(bytes) {
            let out = '';
            for (let i = 0; i < bytes.length; i++) {
                const h = (bytes[i] & 0xff).toString(16);
                out += h.length === 1 ? '0' + h : h;
            }
            return out;
        }

        function encodeBase64(bytes, pad) {
            let out = '';
            let i = 0;
            const len = bytes.length;
            for (; i + 2 < len; i += 3) {
                const n = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
                out +=
                    B64[(n >> 18) & 63] +
                    B64[(n >> 12) & 63] +
                    B64[(n >> 6) & 63] +
                    B64[n & 63];
            }
            if (pad) {
                const rem = len - i;
                if (rem === 1) {
                    const n = bytes[i] << 16;
                    out += B64[(n >> 18) & 63] + B64[(n >> 12) & 63] + '==';
                } else if (rem === 2) {
                    const n = (bytes[i] << 16) | (bytes[i + 1] << 8);
                    out +=
                        B64[(n >> 18) & 63] +
                        B64[(n >> 12) & 63] +
                        B64[(n >> 6) & 63] +
                        '=';
                }
            }
            return out;
        }

        function decodeUtf16leComplete(bytes) {
            // bytes length must be even; caller holds odd trail / lone high surrogate.
            let out = '';
            for (let i = 0; i + 1 < bytes.length; i += 2) {
                const cu = bytes[i] | (bytes[i + 1] << 8);
                out += String.fromCharCode(cu);
            }
            return out;
        }

        function utf16leHoldCount(allBytes, flush) {
            if (allBytes.length === 0) return 0;
            let evenLen = allBytes.length - (allBytes.length & 1);
            if (!flush && evenLen >= 2) {
                const lo = allBytes[evenLen - 2];
                const hi = allBytes[evenLen - 1];
                const cu = lo | (hi << 8);
                // Hold incomplete surrogate pair (high without low).
                if (cu >= 0xd800 && cu <= 0xdbff) {
                    evenLen -= 2;
                }
            }
            return allBytes.length - evenLen;
        }

        class StringDecoder {
            constructor(encoding) {
                const enc = normalizeEncoding(encoding);
                if (!isSupportedEncoding(enc)) {
                    const err = new TypeError('Unknown encoding: ' + encoding);
                    err.code = 'ERR_UNKNOWN_ENCODING';
                    throw err;
                }
                this.encoding = enc;
                this._encoding = enc;
                this._buffer = [];
            }

            write(buffer) {
                if (typeof buffer === 'string') return buffer;
                const bytes = bytesFromInput(buffer);
                if (bytes === null) return buffer;
                if (!Array.isArray(bytes)) return '';

                const allBytes = this._buffer.concat(bytes);
                this._buffer = [];
                if (allBytes.length === 0) return '';

                const enc = this.encoding;
                if (enc === 'utf8') {
                    const incomplete = utf8IncompleteTrail(allBytes);
                    if (incomplete > 0) {
                        this._buffer = allBytes.slice(allBytes.length - incomplete);
                        const emitBytes = allBytes.slice(0, allBytes.length - incomplete);
                        if (emitBytes.length === 0) return '';
                        return decodeUtf8Complete(emitBytes);
                    }
                    return decodeUtf8Complete(allBytes);
                }

                if (enc === 'utf16le') {
                    const hold = utf16leHoldCount(allBytes, false);
                    if (hold > 0) {
                        this._buffer = allBytes.slice(allBytes.length - hold);
                        const emitBytes = allBytes.slice(0, allBytes.length - hold);
                        if (emitBytes.length === 0) return '';
                        return decodeUtf16leComplete(emitBytes);
                    }
                    return decodeUtf16leComplete(allBytes);
                }

                if (enc === 'base64') {
                    const rem = allBytes.length % 3;
                    if (rem !== 0) {
                        this._buffer = allBytes.slice(allBytes.length - rem);
                        const emitBytes = allBytes.slice(0, allBytes.length - rem);
                        if (emitBytes.length === 0) return '';
                        return encodeBase64(emitBytes, false);
                    }
                    return encodeBase64(allBytes, false);
                }

                if (enc === 'hex') {
                    return decodeHex(allBytes);
                }

                if (enc === 'ascii') {
                    return decodeAscii(allBytes);
                }

                // latin1 (binary normalized to latin1)
                return decodeLatin1(allBytes);
            }

            end(buffer) {
                let res = '';
                if (buffer !== undefined && buffer !== null && buffer !== '') {
                    res += this.write(buffer);
                } else if (buffer === '') {
                    // no-op write of empty string passthrough
                }

                if (this._buffer.length > 0) {
                    const held = this._buffer;
                    this._buffer = [];
                    const enc = this.encoding;
                    if (enc === 'utf8') {
                        res += decodeUtf8Complete(held);
                    } else if (enc === 'utf16le') {
                        // Odd trailing byte is discarded (Node); lone high surrogate is emitted.
                        const emitBytes = held.slice(0, held.length - (held.length & 1));
                        if (emitBytes.length > 0) {
                            res += decodeUtf16leComplete(emitBytes);
                        }
                    } else if (enc === 'base64') {
                        res += encodeBase64(held, true);
                    } else if (enc === 'hex') {
                        res += decodeHex(held);
                    } else if (enc === 'ascii') {
                        res += decodeAscii(held);
                    } else {
                        res += decodeLatin1(held);
                    }
                }
                return res;
            }
        }

        const stringDecoderModule = {
            StringDecoder,
            default: StringDecoder
        };

        globalThis.__string_decoder = stringDecoderModule;
        globalThis.string_decoder = stringDecoderModule;
    })();
    "#;

    let script_source = v8::String::new(scope, js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    let string_decoder_key = v8::String::new(scope, "__string_decoder").unwrap();
    if let Some(sd_val) = global.get(scope, string_decoder_key.into()) {
        let sd_key = v8::String::new(scope, "string_decoder").unwrap();
        global.set(scope, sd_key.into(), sd_val);
    }

    Ok(())
}
