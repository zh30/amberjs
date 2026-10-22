// Amber $262 host object for test262.
// Prefer native __amber_createRealm (AMBER_TEST262=1) for real cross-realm
// builtins. Agent / detachArrayBuffer stay honest throw-stubs.
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

  function detachArrayBuffer(_buffer) {
    throw new Test262Error("$262.detachArrayBuffer is not implemented on Amber yet");
  }

  function evalScript(source) {
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
