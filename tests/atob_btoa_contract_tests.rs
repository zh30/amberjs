//! Pins docs/ATOB_BTOA_CONTRACT.md (G29). Real Latin-1 + standard Base64; not HTML ForgivingBase64.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_js(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("execution failed: {error}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn btoa_atob_basic_empty_and_round_trip() {
    let output = run_js(
        r#"
        const hi = btoa("Hello");
        const back = atob(hi);
        const empty = btoa("") === "" && atob("") === "";
        const special = btoa("+/)=") === "Ky8pPQ==" && atob("Ky8pPQ==") === "+/)=";
        const latin1 = String.fromCharCode(0, 255, 128);
        const round = atob(btoa(latin1)) === latin1;
        [
            typeof atob,
            typeof btoa,
            hi,
            back,
            empty,
            special,
            round
        ].join("|");
        "#,
    );
    assert_eq!(
        output,
        "function|function|SGVsbG8=|Hello|true|true|true",
        "got {output}"
    );
}

#[test]
#[serial]
fn coercion_and_required_argument() {
    let output = run_js(
        r#"
        const nullEnc = btoa(null);
        const numEnc = btoa(123);
        // ToString(null) === "null", which is four valid Base64 chars and decodes.
        const nullDec = atob(null);
        const missingBtoa = (() => {
            try { btoa(); return "ok"; } catch (e) {
                return (e instanceof Error) + ":" + e.message;
            }
        })();
        const missingAtob = (() => {
            try { atob(); return "ok"; } catch (e) {
                return (e instanceof Error) + ":" + e.message;
            }
        })();
        const undefBtoa = (() => {
            try { btoa(undefined); return "ok"; } catch (e) {
                return e.message;
            }
        })();
        [
            nullEnc,
            numEnc,
            nullDec.length,
            [...nullDec].map((c) => c.charCodeAt(0)).join(","),
            missingBtoa,
            missingAtob,
            undefBtoa
        ].join("|");
        "#,
    );
    assert_eq!(
        output,
        "bnVsbA==|MTIz|3|158,233,101|true:btoa: input is required|true:atob: input is required|btoa: input is required",
        "got {output}"
    );
}

#[test]
#[serial]
fn latin1_rejection_and_invalid_base64() {
    let output = run_js(
        r#"
        const unicode = (() => {
            try {
                btoa("你好");
                return "accepted";
            } catch (e) {
                return (e instanceof Error) + ":" + e.message.includes("Latin-1");
            }
        })();
        const bad = (() => {
            try {
                atob("!!!invalid!!!");
                return "accepted";
            } catch (e) {
                return (e instanceof Error) + ":" + e.message.includes("invalid base64");
            }
        })();
        const whitespace = (() => {
            try {
                atob("SGVs bG8=");
                return "accepted";
            } catch (e) {
                return e.message.includes("invalid base64") ? "strict" : e.message;
            }
        })();
        [unicode, bad, whitespace].join("|");
        "#,
    );
    assert_eq!(output, "true:true|true:true|strict", "got {output}");
}
