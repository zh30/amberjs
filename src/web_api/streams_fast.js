(() => {
    function ReadableStream(underlyingSource) {
        this._queue = [];
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
            },
            error(err) {
                self._errored = err || new Error("ReadableStream error");
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
                if (stream._queue.length > 0) {
                    return Promise.resolve({ value: stream._queue.shift(), done: false });
                }
                if (stream._closed) {
                    return Promise.resolve({ value: undefined, done: true });
                }
                if (typeof stream._source.pull === "function") {
                    var pulled = stream._source.pull(stream._controller);
                    if (pulled && typeof pulled.then === "function") {
                        return pulled.then(() => {
                            if (stream._errored) return Promise.reject(stream._errored);
                            if (stream._queue.length > 0) {
                                return { value: stream._queue.shift(), done: false };
                            }
                            if (stream._closed) return { value: undefined, done: true };
                            return { value: undefined, done: true };
                        });
                    }
                    if (stream._queue.length > 0) {
                        return Promise.resolve({ value: stream._queue.shift(), done: false });
                    }
                    if (stream._closed) return Promise.resolve({ value: undefined, done: true });
                }
                return Promise.resolve({ value: undefined, done: true });
            },
            cancel(reason) {
                stream._closed = true;
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
                return stream._closed ? Promise.resolve() : new Promise(() => {});
            },
        };
    };
    ReadableStream.prototype.cancel = function (reason) {
        this._closed = true;
        this._queue.length = 0;
        if (typeof this._source.cancel === "function") {
            return Promise.resolve(this._source.cancel(reason));
        }
        return Promise.resolve();
    };
    ReadableStream.prototype.pipeThrough = function (transform) {
        if (!transform || !transform.writable || !transform.readable) {
            throw new TypeError("pipeThrough requires { writable, readable }");
        }
        this.pipeTo(transform.writable);
        return transform.readable;
    };
    ReadableStream.prototype.pipeTo = function (writable) {
        var reader = this.getReader();
        if (!writable || typeof writable.getWriter !== "function") {
            return Promise.reject(new TypeError("pipeTo requires a WritableStream"));
        }
        var writer = writable.getWriter();
        function pump() {
            return reader.read().then((result) => {
                if (result.done) {
                    if (writer.close) return writer.close();
                    return undefined;
                }
                return Promise.resolve(writer.write(result.value)).then(pump);
            });
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
