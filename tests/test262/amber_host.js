// Amber $262 host object for test262.
// Prefer native __amber_createRealm / __amber_detachArrayBuffer (AMBER_TEST262=1).
// Agent stays an honest throw-stub until multi-agent host lands.
/* eslint-disable no-var */

// test262 INTERPRETING.md: `print` must be a writable/configurable/non-enumerable
// global that exposes the string value of its first argument to the runner.
// Used by async completion and by tests that alias `print` (e.g. Array.print).
(function installPrint(global) {
  function print(value) {
    var s = arguments.length === 0 ? "" : String(value);
    if (typeof console !== "undefined" && typeof console.log === "function") {
      console.log(s);
    }
  }
  Object.defineProperty(global, "print", {
    value: print,
    writable: true,
    enumerable: false,
    configurable: true,
  });
})(typeof globalThis !== "undefined" ? globalThis : this);

var $262 = (function () {
  "use strict";

  function installPrintOnRealmGlobal(g) {
    if (!g || typeof g.print === "function") {
      return;
    }
    function print(value) {
      var s = arguments.length === 0 ? "" : String(value);
      if (typeof console !== "undefined" && typeof console.log === "function") {
        console.log(s);
      }
    }
    Object.defineProperty(g, "print", {
      value: print,
      writable: true,
      enumerable: false,
      configurable: true,
    });
  }

  function createRealm() {
    if (typeof __amber_createRealm === "function") {
      var record = __amber_createRealm();
      // Ensure nested createRealm stays on the returned $262-shaped record.
      if (record && typeof record.createRealm !== "function") {
        record.createRealm = createRealm;
      }
      // Prefer native detach on the realm record; fall back to host helper.
      if (record && typeof record.detachArrayBuffer !== "function") {
        record.detachArrayBuffer = detachArrayBuffer;
      }
      // INTERPRETING.md: createRealm defines the host API (incl. print) on the
      // new realm's global object.
      if (record && record.global) {
        installPrintOnRealmGlobal(record.global);
      }
      return record;
    }
    throw new Test262Error(
      "$262.createRealm requires AMBER_TEST262=1 (native realm host)"
    );
  }

  function detachArrayBuffer(buffer, key) {
    if (typeof __amber_detachArrayBuffer === "function") {
      if (arguments.length >= 2) {
        return __amber_detachArrayBuffer(buffer, key);
      }
      return __amber_detachArrayBuffer(buffer);
    }
    // Fallback when AMBER_TEST262 is unset: global web_api detach (no key).
    var native = globalThis.detachArrayBuffer;
    if (typeof native === "function" && native !== detachArrayBuffer) {
      return native(buffer);
    }
    throw new Test262Error(
      "$262.detachArrayBuffer requires AMBER_TEST262=1 (native detach host)"
    );
  }

  function evalScript(source) {
    // test262 `$262.evalScript` must evaluate an ECMAScript *Script* in the
    // current realm (GlobalDeclarationInstantiation: non-configurable global
    // var/function bindings, persistent global lexical `let`/`const`/`class`,
    // and lex/var collision SyntaxErrors). Indirect `eval` is Eval Code and
    // fails those cases (configurable bindings; lexical names vanish).
    // Prefer Amber's Node-compat `vm.runInThisContext` (Script semantics).
    // Only catch require() failure — never swallow Script SyntaxError/TypeError.
    var vm = null;
    if (typeof require === "function") {
      try {
        vm = require("vm");
      } catch (_err) {
        vm = null;
      }
    }
    if (vm && typeof vm.runInThisContext === "function") {
      return vm.runInThisContext(String(source));
    }
    return (0, eval)(source);
  }

  function gc() {
    // Best-effort no-op; Amber does not expose V8 gc() to scripts by default.
  }

  return {
    createRealm: createRealm,
    detachArrayBuffer: detachArrayBuffer,
    evalScript: evalScript,
    gc: gc,
    global: globalThis,
    agent: {
      start: function () {
        throw new Test262Error("$262.agent is not implemented on Amber yet");
      },
      broadcast: function () {
        throw new Test262Error("$262.agent is not implemented on Amber yet");
      },
      getReport: function () {
        throw new Test262Error("$262.agent is not implemented on Amber yet");
      },
      sleep: function () {
        throw new Test262Error("$262.agent is not implemented on Amber yet");
      },
      monotonicNow: function () {
        return Date.now();
      },
    },
  };
})();
