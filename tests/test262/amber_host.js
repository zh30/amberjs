// Amber $262 host object for test262.
// Minimal surface: enough for most language tests. Agent/Atomics helpers
// and detachArrayBuffer are stubs that throw until Amber grows real support —
// those tests FAIL honestly rather than silently pass.
/* eslint-disable no-var */
var $262 = (function () {
  "use strict";

  function createRealm() {
    throw new Test262Error("$262.createRealm is not implemented on Amber yet");
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
