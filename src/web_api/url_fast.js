(() => {
    function decodeForm(value) {
        try {
            return decodeURIComponent(String(value).replace(/\+/g, " "));
        } catch (_error) {
            return String(value);
        }
    }

    function encodeForm(value) {
        return encodeURIComponent(String(value)).replace(/%20/g, "+");
    }

    function parseQuery(qs) {
        var out = [];
        if (qs == null || qs === "") return out;
        var s = String(qs);
        var start = s.charCodeAt(0) === 63 ? 1 : 0;
        var len = s.length;
        if (start >= len) return out;
        var i = start;
        while (i <= len) {
            var amp = s.indexOf("&", i);
            if (amp < 0) amp = len;
            if (amp > i) {
                var eq = s.indexOf("=", i);
                if (eq < 0 || eq > amp) {
                    out.push([decodeForm(s.slice(i, amp)), ""]);
                } else {
                    out.push([
                        decodeForm(s.slice(i, eq)),
                        decodeForm(s.slice(eq + 1, amp)),
                    ]);
                }
            }
            if (amp === len) break;
            i = amp + 1;
        }
        return out;
    }

    function isIterable(value) {
        return (
            value != null &&
            typeof value !== "string" &&
            typeof value[Symbol.iterator] === "function"
        );
    }

    function readSequencePair(item) {
        if (item == null || typeof item[Symbol.iterator] !== "function") {
            throw new TypeError("URLSearchParams sequence pair must be an array");
        }
        var values = [];
        var iterator = item[Symbol.iterator]();
        for (;;) {
            var step = iterator.next();
            if (step.done) break;
            values.push(step.value);
        }
        if (values.length !== 2) {
            throw new TypeError(
                "URLSearchParams sequence pair must contain exactly two items",
            );
        }
        return [String(values[0]), String(values[1])];
    }

    function makeIterator(items) {
        var index = 0;
        var iterator = {
            next: function () {
                if (index < items.length) {
                    return { value: items[index++], done: false };
                }
                return { value: undefined, done: true };
            },
        };
        iterator[Symbol.iterator] = function () {
            return this;
        };
        return iterator;
    }

    function URLSearchParams(init) {
        if (!(this instanceof URLSearchParams)) return new URLSearchParams(init);
        this._list = [];
        this._sync = null;
        if (init == null || init === "") return;
        if (init instanceof URLSearchParams) {
            this._list = init._list.map(function (pair) {
                return [pair[0], pair[1]];
            });
            return;
        }
        if (typeof init === "string") {
            this._list = parseQuery(init);
            return;
        }
        if (isIterable(init)) {
            var iterator = init[Symbol.iterator]();
            for (;;) {
                var step = iterator.next();
                if (step.done) break;
                this._list.push(readSequencePair(step.value));
            }
            return;
        }
        if (typeof init === "object") {
            var keys = Object.keys(init);
            for (var k = 0; k < keys.length; k++) {
                this._list.push([keys[k], String(init[keys[k]])]);
            }
        }
    }

    URLSearchParams.prototype.get = function (name) {
        name = String(name);
        var list = this._list;
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] === name) return list[i][1];
        }
        return null;
    };
    URLSearchParams.prototype.getAll = function (name) {
        name = String(name);
        var values = [];
        var list = this._list;
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] === name) values.push(list[i][1]);
        }
        return values;
    };
    URLSearchParams.prototype.set = function (name, value) {
        name = String(name);
        value = String(value);
        var list = this._list;
        var found = false;
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] === name) {
                if (!found) {
                    list[i][1] = value;
                    found = true;
                } else {
                    list.splice(i, 1);
                    i--;
                }
            }
        }
        if (!found) list.push([name, value]);
        if (this._sync) this._sync();
    };
    URLSearchParams.prototype.append = function (name, value) {
        this._list.push([String(name), String(value)]);
        if (this._sync) this._sync();
    };
    URLSearchParams.prototype.delete = function (name) {
        name = String(name);
        var list = this._list;
        var next = [];
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] !== name) next.push(list[i]);
        }
        this._list = next;
        if (this._sync) this._sync();
    };
    URLSearchParams.prototype.has = function (name) {
        return this.get(name) !== null;
    };
    URLSearchParams.prototype.sort = function () {
        this._list.sort(function (a, b) {
            if (a[0] < b[0]) return -1;
            if (a[0] > b[0]) return 1;
            return 0;
        });
        if (this._sync) this._sync();
    };
    URLSearchParams.prototype.toString = function () {
        var list = this._list;
        var parts = [];
        for (var i = 0; i < list.length; i++) {
            parts.push(encodeForm(list[i][0]) + "=" + encodeForm(list[i][1]));
        }
        return parts.join("&");
    };
    URLSearchParams.prototype.forEach = function (cb, thisArg) {
        var list = this._list.slice();
        for (var i = 0; i < list.length; i++) {
            cb.call(thisArg, list[i][1], list[i][0], this);
        }
    };
    URLSearchParams.prototype.entries = function () {
        return makeIterator(
            this._list.map(function (pair) {
                return [pair[0], pair[1]];
            }),
        );
    };
    URLSearchParams.prototype.keys = function () {
        return makeIterator(
            this._list.map(function (pair) {
                return pair[0];
            }),
        );
    };
    URLSearchParams.prototype.values = function () {
        return makeIterator(
            this._list.map(function (pair) {
                return pair[1];
            }),
        );
    };
    URLSearchParams.prototype[Symbol.iterator] = URLSearchParams.prototype.entries;

    function parseAbs(input, base) {
        input = String(input);
        if (base != null && input.indexOf("://") < 0) {
            input = resolveRelative(input, String(base));
        }
        var scheme = input.indexOf("://");
        if (scheme <= 0) throw new TypeError("Invalid URL");
        var afterHost = scheme + 3;
        var pathStart = input.indexOf("/", afterHost);
        var qIdx = input.indexOf("?", afterHost);
        var hashIdx = input.indexOf("#", afterHost);
        var endPath =
            qIdx >= 0 && (hashIdx < 0 || qIdx < hashIdx)
                ? qIdx
                : hashIdx >= 0
                  ? hashIdx
                  : input.length;
        var pathname =
            pathStart >= 0 && pathStart <= endPath
                ? input.slice(pathStart, endPath)
                : "/";
        var search =
            qIdx >= 0
                ? input.slice(qIdx, hashIdx >= 0 ? hashIdx : input.length)
                : "";
        var hash = hashIdx >= 0 ? input.slice(hashIdx) : "";
        var protocol = input.slice(0, scheme + 1);
        var hostEnd = pathStart >= 0 ? pathStart : endPath;
        var hostport = input.slice(afterHost, hostEnd);
        var hostname;
        var port = "";
        if (hostport.charCodeAt(0) === 91) {
            hostname = hostport;
        } else {
            var colon = hostport.lastIndexOf(":");
            if (colon >= 0) {
                hostname = hostport.slice(0, colon);
                port = hostport.slice(colon + 1);
            } else {
                hostname = hostport;
            }
        }
        return {
            protocol: protocol,
            hostname: hostname,
            port: port,
            host: hostport,
            pathname: pathname,
            search: search,
            hash: hash,
            origin: protocol + "//" + hostport,
        };
    }

    function resolveRelative(input, base) {
        var colon = base.indexOf("://");
        if (colon < 0) throw new TypeError("Invalid URL");
        var pathStart = base.indexOf("/", colon + 3);
        var qBase = base.indexOf("?");
        var hBase = base.indexOf("#");
        var originEnd =
            pathStart >= 0
                ? pathStart
                : qBase >= 0
                  ? qBase
                  : hBase >= 0
                    ? hBase
                    : base.length;
        var origin = base.slice(0, originEnd);
        var basePath =
            pathStart >= 0
                ? base.slice(
                      pathStart,
                      qBase >= 0 ? qBase : hBase >= 0 ? hBase : base.length,
                  )
                : "/";
        var c0 = input.charCodeAt(0);
        if (c0 === 47) return origin + input;
        if (c0 === 63) return origin + basePath + input;
        if (c0 === 35) {
            return (
                origin +
                basePath +
                (qBase >= 0
                    ? base.slice(qBase, hBase >= 0 ? hBase : base.length)
                    : "") +
                input
            );
        }
        var slash = basePath.lastIndexOf("/");
        return origin + basePath.slice(0, slash + 1) + input;
    }

    function rebuildHref(url) {
        url._href = url.origin + url.pathname + url._search + url.hash;
    }

    function URL(input, base) {
        if (!(this instanceof URL)) return new URL(input, base);
        var parsed = parseAbs(input, base);
        this.protocol = parsed.protocol;
        this.hostname = parsed.hostname;
        this.port = parsed.port;
        this.host = parsed.host;
        this.pathname = parsed.pathname;
        this.hash = parsed.hash;
        this.origin = parsed.origin;
        this.username = "";
        this.password = "";
        this._search = parsed.search;
        this._sp = null;
        rebuildHref(this);
    }

    Object.defineProperty(URL.prototype, "href", {
        get() {
            return this._href;
        },
        set(value) {
            var parsed = parseAbs(value, null);
            this.protocol = parsed.protocol;
            this.hostname = parsed.hostname;
            this.port = parsed.port;
            this.host = parsed.host;
            this.pathname = parsed.pathname;
            this.hash = parsed.hash;
            this.origin = parsed.origin;
            this._search = parsed.search;
            if (this._sp) this._sp._list = parseQuery(this._search);
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "search", {
        get() {
            return this._search;
        },
        set(value) {
            var search = String(value == null ? "" : value);
            if (search && search.charCodeAt(0) !== 63) search = "?" + search;
            if (search === "?") search = "";
            this._search = search;
            if (this._sp) this._sp._list = parseQuery(search);
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "searchParams", {
        get() {
            if (!this._sp) {
                var url = this;
                this._sp = new URLSearchParams(this._search);
                this._sp._sync = function () {
                    var query = url._sp.toString();
                    url._search = query ? "?" + query : "";
                    rebuildHref(url);
                };
            }
            return this._sp;
        },
        configurable: true,
    });
    URL.prototype.toString = function () {
        return this.href;
    };
    URL.prototype.toJSON = function () {
        return this.href;
    };
    URL.canParse = function (input, base) {
        try {
            parseAbs(input, base);
            return true;
        } catch (_error) {
            return false;
        }
    };

    globalThis.URL = URL;
    globalThis.URLSearchParams = URLSearchParams;
})();
