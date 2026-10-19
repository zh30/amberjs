//! Pins docs/NODE_STREAM_CONTRACT.md G71 — Transform cb/push, pipe/unpipe
//! forwarding, end/_flush 收尾与 stream.pipeline 完成回调。
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn eval(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("{code} should evaluate: {error}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn transform_cb_pushes_output_and_write_cb_fires() {
    // transform(chunk, enc, cb) 的 cb(null, out) → out 进入 readable 侧
    // ('data' 监听)；write 回调在 transform 消费完该 chunk 后触发。
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform({
              transform(chunk, enc, cb) {
                got.push('ENC=' + enc);
                cb(null, String(chunk).toUpperCase());
              }
            });
            t.on('data', (c) => got.push('D=' + c));
            t.write('ab');
            t.write('cd', 'utf8', () => got.push('WCB'));
            got.join('|');
            "#
        ),
        "ENC=utf8|D=AB|ENC=utf8|D=CD|WCB"
    );
}

#[test]
#[serial]
fn transform_write_with_cb_as_second_arg() {
    // write(chunk, cb) —— arg1 为函数时按 cb 处理，enc 默认 'utf8'
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform({
              transform(chunk, enc, cb) { got.push(enc); cb(null, chunk); }
            });
            t.on('data', () => {});
            t.write('x', () => got.push('CB'));
            got.join('|');
            "#
        ),
        "utf8|CB"
    );
}

#[test]
#[serial]
fn transform_cb_error_goes_to_error_listener() {
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform({
              transform(chunk, enc, cb) { cb(new Error('boom')); }
            });
            t.on('data', () => got.push('D'));
            t.on('error', (e) => got.push('E=' + e.message));
            t.write('x');
            got.join('|');
            "#
        ),
        "E=boom"
    );
}

#[test]
#[serial]
fn transform_end_runs_final_chunk_flush_and_events() {
    // end('tail') → 末块仍走 _transform；_flush 可再 push 尾数据；
    // 收尾 = readable 'end' + writable 'finish' + 用户 cb。
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform({
              transform(chunk, enc, cb) { cb(null, '[' + chunk + ']'); },
              flush(cb) { cb(null, 'TAIL'); }
            });
            t.on('data', (c) => got.push('D=' + c));
            t.on('end', () => got.push('END'));
            t.on('finish', () => got.push('FIN'));
            t.end('last', () => got.push('CB'));
            got.join('|');
            "#
        ),
        "D=[last]|D=TAIL|END|FIN|CB"
    );
}

#[test]
#[serial]
fn transform_push_null_emits_end() {
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform();
            t.on('data', (c) => got.push('D=' + c));
            t.on('end', () => got.push('END'));
            t.push('a');
            t.push(null);
            t.push('b');
            got.join('|');
            "#
        ),
        "D=a|END"
    );
}

#[test]
#[serial]
fn transform_pipe_forwards_data_and_end() {
    // t.pipe(w)：transform 输出经 dest.write 到达；source end → dest.end()
    // → dest 'finish'。pipe 返回 dest（可继续链）。
    assert_eq!(
        eval(
            r#"
            const { Transform, Writable } = require('stream');
            const got = [];
            const t = new Transform({
              transform(c, e, cb) { cb(null, '<' + c + '>'); }
            });
            const w = new Writable({
              write(c, e, cb) { got.push('W=' + c); cb(); }
            });
            w.on('finish', () => got.push('WFIN'));
            const ret = t.pipe(w);
            got.push('RET=' + (ret === w));
            t.write('a');
            t.write('b');
            t.end('c');
            got.join('|');
            "#
        ),
        "RET=true|W=<a>|W=<b>|W=<c>|WFIN"
    );
}

#[test]
#[serial]
fn transform_pipe_chain_transform_to_transform() {
    // t1.pipe(t2).pipe(w)：Transform 作为 pipe 目的端再接一条链
    assert_eq!(
        eval(
            r#"
            const { Transform, Writable } = require('stream');
            const got = [];
            const t1 = new Transform({ transform(c, e, cb) { cb(null, c + '1'); } });
            const t2 = new Transform({ transform(c, e, cb) { cb(null, c + '2'); } });
            const w = new Writable({ write(c, e, cb) { got.push(c); cb(); } });
            t1.pipe(t2).pipe(w);
            t1.write('x');
            t1.end();
            got.join('|');
            "#
        ),
        "x12"
    );
}

#[test]
#[serial]
fn transform_unpipe_stops_forwarding() {
    assert_eq!(
        eval(
            r#"
            const { Transform, Writable } = require('stream');
            const got = [];
            const t = new Transform({ transform(c, e, cb) { cb(null, c); } });
            const w = new Writable({ write(c, e, cb) { got.push(c); cb(); } });
            t.pipe(w);
            t.write('a');
            const ret = t.unpipe();
            got.push('RET=' + (ret === t));
            t.write('b');
            t.end('c');
            got.join('|');
            "#
        ),
        "a|RET=true"
    );
}

#[test]
#[serial]
fn pipeline_runs_completion_callback_on_finish() {
    // pipeline(src, ..., dest, cb)：cb 在末个 dest 'finish' 时触发；
    // 返回值是末个可写流。
    assert_eq!(
        eval(
            r#"
            const { PassThrough, Transform, Writable, pipeline } = require('stream');
            const got = [];
            const src = new PassThrough();
            const t = new Transform({ transform(c, e, cb) { cb(null, c + '!'); } });
            const w = new Writable({ write(c, e, cb) { got.push(c); cb(); } });
            const ret = pipeline(src, t, w, () => got.push('DONE'));
            got.push('RET=' + (ret === w));
            src.write('q');
            src.end();
            got.join('|');
            "#
        ),
        "RET=true|q!|DONE"
    );
}

#[test]
#[serial]
fn transform_without_user_fn_passes_through() {
    // 无 transform 选项时恒等透传（write → push 原 chunk）
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform();
            t.on('data', (c) => got.push(c));
            t.write('same');
            t.end();
            got.join('|');
            "#
        ),
        "same"
    );
}

#[test]
#[serial]
fn transform_on_end_does_not_clobber_end_method() {
    // 'end' 既是方法名又是事件名：on('end') 不得覆盖 end() 方法
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const t = new Transform();
            t.on('end', () => {});
            [typeof t.end, typeof t.on].join('|');
            "#
        ),
        "function|function"
    );
}

#[test]
#[serial]
fn transform_end_waits_for_async_transform_cb() {
    // G71: end(chunk) 的收尾必须由末块 _write 的写回调驱动——
    // transform 把 cb 存起来异步调用时，finish 不得先于其输出到达。
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform({
              transform(chunk, enc, cb) { globalThis._cb = cb; }
            });
            t.on('data', (c) => got.push('D=' + c));
            t.on('finish', () => got.push('FIN'));
            t.end('x', () => got.push('ENDCB'));
            // transform cb 尚未调用：end 已经返回，但收尾不该发生
            got.push('AFTER=' + got.length);
            globalThis._cb(null, 'LATE');
            got.join('|');
            "#
        ),
        "AFTER=0|D=LATE|FIN|ENDCB"
    );
}

#[test]
#[serial]
fn transform_push_after_eof_is_ignored() {
    // G71: push(null) 之后的 push 静默失败——不发 data、不重发 end。
    assert_eq!(
        eval(
            r#"
            const { Transform } = require('stream');
            const got = [];
            const t = new Transform();
            t.on('data', (c) => got.push('D=' + c));
            t.on('end', () => got.push('END'));
            const r1 = t.push('a');
            const r2 = t.push(null);
            const r3 = t.push('b');
            const r4 = t.push(null);
            got.push('R=' + [r1, r2, r3, r4].join(','));
            got.join('|');
            "#
        ),
        "D=a|END|R=true,false,false,false"
    );
}

#[test]
#[serial]
fn pipeline_completion_cb_fires_for_fresh_transform_dest() {
    // G71: 末段是没注册过任何监听的 Transform/PassThrough 时，
    // pipeline 也要挂到 _events.finish（fresh 流没有 _events 对象）。
    assert_eq!(
        eval(
            r#"
            const { PassThrough, Transform, pipeline } = require('stream');
            const got = [];
            const src = new PassThrough();
            const mid = new Transform({ transform(c, e, cb) { cb(null, c + '!'); } });
            const dst = new Transform({ transform(c, e, cb) { got.push('D=' + c); cb(); } });
            pipeline(src, mid, dst, () => got.push('DONE'));
            src.write('q');
            src.end();
            got.join('|');
            "#
        ),
        "D=q!|DONE"
    );
}

#[test]
#[serial]
fn pipeline_completion_cb_runs_once_on_double_end() {
    // G71: dest.end() 被调用两次（链上级联 + 用户直调）时 cb 只跑一次。
    assert_eq!(
        eval(
            r#"
            const { PassThrough, Writable, pipeline } = require('stream');
            const got = [];
            const src = new PassThrough();
            const w = new Writable({ write(c, e, cb) { cb(); } });
            pipeline(src, w, () => got.push('DONE'));
            src.write('q');
            src.end();
            w.end(); // 二次 end：finish 重发，cb 不得重复
            got.join('|');
            "#
        ),
        "DONE"
    );
}

#[test]
#[serial]
fn pipeline_completion_cb_fires_for_pre_ended_source() {
    // G71: 已 ended 的 Readable 进 pipeline——'end' 在挂监听瞬间即发，
    // 完成回调必须先于接管挂好、且下游先连才不丢收尾。
    assert_eq!(
        eval(
            r#"
            const { Readable, Writable, pipeline } = require('stream');
            const got = [];
            const r = new Readable({ read() {} });
            r.push('x');
            r.push(null); // 先结束再进 pipeline
            const w = new Writable({ write(c, e, cb) { got.push('W=' + c); cb(); } });
            pipeline(r, w, () => got.push('DONE'));
            got.join('|');
            "#
        ),
        "DONE"
    );
}
