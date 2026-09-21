fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Linux ELF will not resolve hello.node's undefined napi_* against the
    // main executable unless those symbols are in the dynamic symbol table.
    // A global `-rdynamic` rustflag previously broke rust-crypto, so this is
    // scoped to the `amber` binary only.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-bin=amber=-Wl,--export-dynamic");
        println!("cargo:rustc-link-arg-bin=amber=-Wl,--export-dynamic");
    }
}
