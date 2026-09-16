(() => {
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
                if (eq < 0 || eq > amp) out.push([s.slice(i, amp), ""]);
                else out.push([s.slice(i, eq), s.slice(eq + 1, amp)]);
            }
            if (amp === len) break;
            i = amp + 1;
        }
        return out;
    }
    function URLSearchParams(init) {
        if (!(this instanceof URLSearchParams)) return new URLSearchParams(init);
        this._list = [];
        if (init == null || init === "") return;
        if (typeof init === "string") this._list = parseQuery(init);
        else if (Array.isArray(init)) {
            for (var i = 0; i < init.length; i++) {
                this._list.push([String(init[i][0]), String(init[i][1])]);
            }
        } else if (typeof init === "object") {
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
    URLSearchParams.prototype.set = function (name, value) {
        name = String(name);
        value = String(value);
        var list = this._list;
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] === name) {
                list[i][1] = value;
                var j = i + 1;
                while (j < list.length) {
                    if (list[j][0] === name) list.splice(j, 1);
                    else j++;
                }
                return;
            }
        }
        list.push([name, value]);
    };
    URLSearchParams.prototype.append = function (name, value) {
        this._list.push([String(name), String(value)]);
    };
    URLSearchParams.prototype.delete = function (name) {
        name = String(name);
        var list = this._list;
        var next = [];
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] !== name) next.push(list[i]);
        }
        this._list = next;
    };
    URLSearchParams.prototype.has = function (name) {
        return this.get(name) !== null;
    };
    URLSearchParams.prototype.toString = function () {
        var list = this._list;
        var s = "";
        for (var i = 0; i < list.length; i++) {
            if (i) s += "&";
            s += list[i][0] + "=" + list[i][1];
        }
        return s;
    };
    URLSearchParams.prototype.forEach = function (cb, thisArg) {
        var list = this._list;
        for (var i = 0; i < list.length; i++) cb.call(thisArg, list[i][1], list[i][0], this);
    };
    function resolveRelative(input, base) {
        var colon = base.indexOf("://");
        if (colon < 0) throw new TypeError("Invalid URL");
        var pathStart = base.indexOf("/", colon + 3);
        var qBase = base.indexOf("?");
        var hBase = base.indexOf("#");
        var originEnd = pathStart >= 0 ? pathStart : qBase >= 0 ? qBase : hBase >= 0 ? hBase : base.length;
        var origin = base.slice(0, originEnd);
        var basePath =
            pathStart >= 0
                ? base.slice(pathStart, qBase >= 0 ? qBase : hBase >= 0 ? hBase : base.length)
                : "/";
        var c0 = input.charCodeAt(0);
        if (c0 === 47) return origin + input;
        if (c0 === 63) return origin + basePath + input;
        if (c0 === 35) {
            return origin + basePath + (qBase >= 0 ? base.slice(qBase, hBase >= 0 ? hBase : base.length) : "") + input;
        }
        var slash = basePath.lastIndexOf("/");
        return origin + basePath.slice(0, slash + 1) + input;
    }
    function URL(input, base) {
        input = String(input);
        if (base != null && input.indexOf("://") < 0) input = resolveRelative(input, String(base));
        var scheme = input.indexOf("://");
        if (scheme <= 0) throw new TypeError("Invalid URL");
        var afterHost = scheme + 3;
        var pathStart = input.indexOf("/", afterHost);
        var qIdx = input.indexOf("?", afterHost);
        var hashIdx = input.indexOf("#", afterHost);
        var endPath = qIdx >= 0 && (hashIdx < 0 || qIdx < hashIdx) ? qIdx : hashIdx >= 0 ? hashIdx : input.length;
        this.pathname = pathStart >= 0 && pathStart <= endPath ? input.slice(pathStart, endPath) : "/";
        this.search = qIdx >= 0 ? input.slice(qIdx, hashIdx >= 0 ? hashIdx : input.length) : "";
        this.hash = hashIdx >= 0 ? input.slice(hashIdx) : "";
        this.href = input;
        this.protocol = input.slice(0, scheme + 1);
        var hostEnd = pathStart >= 0 ? pathStart : endPath;
        var hostport = input.slice(afterHost, hostEnd);
        this.host = hostport;
        if (hostport.charCodeAt(0) !== 91) {
            var colon = hostport.lastIndexOf(":");
            if (colon >= 0) {
                this.hostname = hostport.slice(0, colon);
                this.port = hostport.slice(colon + 1);
            } else {
                this.hostname = hostport;
                this.port = "";
            }
        } else {
            this.hostname = hostport;
            this.port = "";
        }
        this.origin = this.protocol + "//" + hostport;
        this.username = "";
        this.password = "";
    }
    Object.defineProperty(URL.prototype, "searchParams", {
        get() {
            if (!this._sp) this._sp = new URLSearchParams(this.search);
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
    URL.canParse = (input, base) => {
        try {
            parseAbs(input, base);
            return true;
        } catch {
            return false;
        }
    };
    globalThis.URL = URL;
    globalThis.URLSearchParams = URLSearchParams;
})();
