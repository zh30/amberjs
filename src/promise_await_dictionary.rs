//! Host polyfill for TC39 `await-dictionary` (Stage 3):
//! `Promise.allKeyed` / `Promise.allSettledKeyed`.
//!
//! V8/Node do not ship these yet. Install on the main realm and on every
//! `$262.createRealm()` context so cross-realm `other.Promise.allKeyed` works.

use anyhow::Result;
use rusty_v8 as v8;

/// JS source that defines `Promise.allKeyed` / `Promise.allSettledKeyed` on the
/// current realm's `Promise`.
pub const INSTALL_JS: &str = r#"
(function () {
  'use strict';

  if (typeof Promise !== 'function') {
    return;
  }
  if (typeof Promise.allKeyed === 'function' && typeof Promise.allSettledKeyed === 'function') {
    return;
  }

  function newPromiseCapability(C) {
    if (typeof C !== 'function') {
      throw new TypeError('Promise capability requires a constructor');
    }
    var resolve, reject;
    var promise = new C(function (resolutionFunc, rejectionFunc) {
      if (resolve !== undefined || reject !== undefined) {
        throw new TypeError('Promise executor already called');
      }
      resolve = resolutionFunc;
      reject = rejectionFunc;
    });
    if (typeof resolve !== 'function') {
      throw new TypeError('Promise resolve is not callable');
    }
    if (typeof reject !== 'function') {
      throw new TypeError('Promise reject is not callable');
    }
    return { promise: promise, resolve: resolve, reject: reject };
  }

  function getPromiseResolve(constructor) {
    var promiseResolve = constructor.resolve;
    if (typeof promiseResolve !== 'function') {
      throw new TypeError('Promise resolve is not callable');
    }
    return promiseResolve;
  }

  function createKeyedPromiseCombinatorResultObject(entries) {
    var obj = Object.create(null);
    for (var i = 0; i < entries.length; i++) {
      var entry = entries[i];
      Object.defineProperty(obj, entry.key, {
        value: entry.value,
        writable: true,
        enumerable: true,
        configurable: true
      });
    }
    return obj;
  }

  function createSettlementObject(status, field, fieldValue) {
    var obj = Object.create(Object.prototype);
    Object.defineProperty(obj, 'status', {
      value: status,
      writable: true,
      enumerable: true,
      configurable: true
    });
    Object.defineProperty(obj, field, {
      value: fieldValue,
      writable: true,
      enumerable: true,
      configurable: true
    });
    return obj;
  }

  // CreateBuiltinFunction(..., 1, "") — arrow: length 1, name "", non-constructible.
  function makeOnFulfilled(variant, entries, thisIndex, alreadyCalled, remaining, capability) {
    return (value) => {
      if (alreadyCalled.value) {
        return;
      }
      alreadyCalled.value = true;
      if (variant === 'all') {
        entries[thisIndex].value = value;
      } else {
        entries[thisIndex].value = createSettlementObject('fulfilled', 'value', value);
      }
      remaining.value = remaining.value - 1;
      if (remaining.value === 0) {
        var result = createKeyedPromiseCombinatorResultObject(entries);
        return capability.resolve(result);
      }
    };
  }

  function makeOnRejected(entries, thisIndex, alreadyCalled, remaining, capability) {
    return (error) => {
      if (alreadyCalled.value) {
        return;
      }
      alreadyCalled.value = true;
      entries[thisIndex].value = createSettlementObject('rejected', 'reason', error);
      remaining.value = remaining.value - 1;
      if (remaining.value === 0) {
        var result = createKeyedPromiseCombinatorResultObject(entries);
        return capability.resolve(result);
      }
    };
  }

  function performPromiseAllKeyed(variant, promises, ctor, capability, promiseResolve) {
    var allKeys = Reflect.ownKeys(promises);
    var entries = [];
    var remaining = { value: 1 };

    for (var i = 0; i < allKeys.length; i++) {
      var key = allKeys[i];
      var propertyDesc = Object.getOwnPropertyDescriptor(promises, key);
      if (propertyDesc !== undefined && propertyDesc.enumerable === true) {
        var propertyValue = promises[key];
        var thisIndex = entries.length;
        entries.push({ key: key, value: undefined });
        var nextPromise = promiseResolve.call(ctor, propertyValue);
        var alreadyCalled = { value: false };
        var onFulfilled = makeOnFulfilled(
          variant,
          entries,
          thisIndex,
          alreadyCalled,
          remaining,
          capability
        );
        var onRejected;
        if (variant === 'all') {
          onRejected = capability.reject;
        } else {
          onRejected = makeOnRejected(
            entries,
            thisIndex,
            alreadyCalled,
            remaining,
            capability
          );
        }
        remaining.value = remaining.value + 1;
        nextPromise.then(onFulfilled, onRejected);
      }
    }

    remaining.value = remaining.value - 1;
    if (remaining.value === 0) {
      var result = createKeyedPromiseCombinatorResultObject(entries);
      capability.resolve(result);
    }
    return capability.promise;
  }

  // Method shorthand → non-constructible; name/length match built-in descriptors.
  var allKeyed = {
    allKeyed(promises) {
      var C = this;
      var capability = newPromiseCapability(C);
      var promiseResolve;
      try {
        promiseResolve = getPromiseResolve(C);
      } catch (e) {
        capability.reject(e);
        return capability.promise;
      }
      if (promises === null || (typeof promises !== 'object' && typeof promises !== 'function')) {
        capability.reject(new TypeError('Promise.allKeyed requires an Object'));
        return capability.promise;
      }
      try {
        return performPromiseAllKeyed('all', promises, C, capability, promiseResolve);
      } catch (e) {
        capability.reject(e);
        return capability.promise;
      }
    }
  }.allKeyed;

  var allSettledKeyed = {
    allSettledKeyed(promises) {
      var C = this;
      var capability = newPromiseCapability(C);
      var promiseResolve;
      try {
        promiseResolve = getPromiseResolve(C);
      } catch (e) {
        capability.reject(e);
        return capability.promise;
      }
      if (promises === null || (typeof promises !== 'object' && typeof promises !== 'function')) {
        capability.reject(new TypeError('Promise.allSettledKeyed requires an Object'));
        return capability.promise;
      }
      try {
        return performPromiseAllKeyed('all-settled', promises, C, capability, promiseResolve);
      } catch (e) {
        capability.reject(e);
        return capability.promise;
      }
    }
  }.allSettledKeyed;

  Object.defineProperty(Promise, 'allKeyed', {
    value: allKeyed,
    writable: true,
    enumerable: false,
    configurable: true
  });
  Object.defineProperty(Promise, 'allSettledKeyed', {
    value: allSettledKeyed,
    writable: true,
    enumerable: false,
    configurable: true
  });
})();
"#;

/// Evaluate [`INSTALL_JS`] in the current context (main realm or `$262.createRealm`).
pub fn install(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let code = v8::String::new(scope, INSTALL_JS)
        .ok_or_else(|| anyhow::anyhow!("oom installing await-dictionary polyfill"))?;
    let script = v8::Script::compile(scope, code, None)
        .ok_or_else(|| anyhow::anyhow!("failed to compile await-dictionary polyfill"))?;
    script
        .run(scope)
        .ok_or_else(|| anyhow::anyhow!("await-dictionary polyfill threw"))?;
    Ok(())
}
