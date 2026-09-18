(() => {
    function abortError() {
        var err = new Error("Aborted");
        err.name = "AbortError";
        return err;
    }

    function takeQueuedChunk(stream) {
        if (stream._readIndex < stream._queue.length) {
            var value = stream._queue[stream._readIndex];
            stream._readIndex += 1;
            return { value: value, done: false };
        }
        return null;
    }

    function ReadableStream(underlyingSource) {
        this._queue = [];
        this._readIndex = 0;
        this._state = 0;
        this._closed = false;
        this._errored = null;
        this._locked = false;
        this._source = underlyingSource || {};
        var self = this;
        this._controller = {
            enqueue(chunk) {
                if (self._closed || self._errored) return;
                self._queue.push(chunk);
            },
            close() {
                self._closed = true;
                self._state = 1;
            },
            error(err) {
                self._errored = err || new Error("ReadableStream error");
                self._state = 2;
            },
            get desiredSize() {
                return 1;
            },
        };
        if (typeof this._source.start === "function") {
            var started = this._source.start(this._controller);
            if (started && typeof started.then === "function") {
                started.catch((err) => {
                    self._errored = err;
                    self._state = 2;
                });
            }
        }
    }
    ReadableStream.prototype.getReader = function () {
        if (this._locked) throw new TypeError("ReadableStream is locked");
        this._locked = true;
        var stream = this;
        return {
            read() {
                if (stream._errored) return Promise.reject(stream._errored);
                var queued = takeQueuedChunk(stream);
                if (queued) return Promise.resolve(queued);
                if (stream._closed) {
                    return Promise.resolve({ value: undefined, done: true });
                }
                if (typeof stream._source.pull === "function") {
                    var pulled = stream._source.pull(stream._controller);
                    if (pulled && typeof pulled.then === "function") {
                        return pulled.then(() => {
                            if (stream._errored)
                                return Promise.reject(stream._errored);
                            var afterPull = takeQueuedChunk(stream);
                            if (afterPull) return afterPull;
                            if (stream._closed)
                                return { value: undefined, done: true };
                            return { value: undefined, done: true };
                        });
                    }
                    queued = takeQueuedChunk(stream);
                    if (queued) return Promise.resolve(queued);
                    if (stream._closed)
                        return Promise.resolve({
                            value: undefined,
                            done: true,
                        });
                }
                return Promise.resolve({ value: undefined, done: true });
            },
            cancel(reason) {
                stream._closed = true;
                stream._state = 1;
                stream._queue.length = 0;
                stream._locked = false;
                if (typeof stream._source.cancel === "function") {
                    return Promise.resolve(stream._source.cancel(reason));
                }
                return Promise.resolve();
            },
            releaseLock() {
                stream._locked = false;
            },
            get closed() {
                return stream._closed
                    ? Promise.resolve()
                    : new Promise(() => {});
            },
        };
    };
    ReadableStream.prototype.cancel = function (reason) {
        this._closed = true;
        this._state = 1;
        this._queue.length = 0;
        if (typeof this._source.cancel === "function") {
            return Promise.resolve(this._source.cancel(reason));
        }
        return Promise.resolve();
    };
    ReadableStream.prototype.pipeThrough = function (transform, options) {
        if (!transform || !transform.writable || !transform.readable) {
            throw new TypeError("pipeThrough requires { writable, readable }");
        }
        // Amber's pipeThrough returns the readable side and also exposes
        // `.readable` / `.writable` on that object so callers can treat the
        // result as either the output stream or the transform pair.
        var readable = transform.readable;
        if (readable && typeof readable === "object") {
            readable.readable = readable;
            readable.writable = transform.writable;
        }
        this.pipeTo(transform.writable, options);
        return readable;
    };
    ReadableStream.prototype.pipeTo = function (writable, options) {
        options = options || {};
        var preventClose = !!options.preventClose;
        var preventAbort = !!options.preventAbort;
        var signal = options.signal;

        if (signal && signal.aborted) {
            return Promise.reject(abortError());
        }
        if (!writable || typeof writable.getWriter !== "function") {
            return Promise.reject(
                new TypeError("pipeTo requires a WritableStream"),
            );
        }

        var reader = this.getReader();
        var writer = writable.getWriter();
        var rejected = false;

        function rejectPipe(err) {
            if (rejected) return Promise.reject(err);
            rejected = true;
            if (preventAbort) return Promise.reject(err);
            if (writer && typeof writer.abort === "function") {
                return Promise.resolve(writer.abort(err)).then(
                    function () {
                        return Promise.reject(err);
                    },
                    function () {
                        return Promise.reject(err);
                    },
                );
            }
            return Promise.reject(err);
        }

        if (signal && typeof signal.addEventListener === "function") {
            signal.addEventListener("abort", function () {
                rejectPipe(abortError());
            });
        }

        function pump() {
            if (rejected) return Promise.reject(abortError());
            return reader.read().then(function (result) {
                if (rejected) return Promise.reject(abortError());
                if (result.done) {
                    if (preventClose) return undefined;
                    if (writer.close) return writer.close();
                    return undefined;
                }
                return Promise.resolve(writer.write(result.value)).then(
                    pump,
                    rejectPipe,
                );
            }, rejectPipe);
        }
        return pump();
    };
    Object.defineProperty(ReadableStream.prototype, "locked", {
        get() {
            return this._locked;
        },
        configurable: true,
    });
    globalThis.ReadableStream = ReadableStream;
})();
