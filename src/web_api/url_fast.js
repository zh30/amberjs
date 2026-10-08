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
    URLSearchParams.prototype.delete = function (name, value) {
        name = String(name);
        var matchValue = arguments.length > 1 && value !== undefined;
        if (matchValue) value = String(value);
        var list = this._list;
        var next = [];
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] !== name || (matchValue && list[i][1] !== value)) {
                next.push(list[i]);
            }
        }
        this._list = next;
        if (this._sync) this._sync();
    };
    URLSearchParams.prototype.has = function (name, value) {
        name = String(name);
        if (arguments.length < 2 || value === undefined) return this.get(name) !== null;
        value = String(value);
        var list = this._list;
        for (var i = 0; i < list.length; i++) {
            if (list[i][0] === name && list[i][1] === value) return true;
        }
        return false;
    };
    Object.defineProperty(URLSearchParams.prototype, "size", {
        get: function () {
            return this._list.length;
        },
        configurable: true,
    });
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

    function isSpecialProtocol(protocol) {
        return (
            protocol === "http:" ||
            protocol === "https:" ||
            protocol === "ws:" ||
            protocol === "wss:" ||
            protocol === "ftp:" ||
            protocol === "file:"
        );
    }

    function defaultPort(protocol) {
        if (protocol === "http:" || protocol === "ws:") return "80";
        if (protocol === "https:" || protocol === "wss:") return "443";
        if (protocol === "ftp:") return "21";
        return "";
    }

    function leadingScheme(input) {
        var match = /^([A-Za-z][A-Za-z0-9+.-]*):/.exec(input);
        if (!match || match[1].length < 2) return "";
        return match[1].toLowerCase() + ":";
    }

    function cleanUrl(input) {
        return String(input)
            .replace(/[\t\n\r]/g, "")
            .replace(/^[\u0000-\u001F ]+|[\u0000-\u001F ]+$/g, "");
    }

    function earlierIndex(left, right) {
        if (left < 0) return right;
        if (right < 0) return left;
        return left < right ? left : right;
    }

    function encodeUser(value) {
        return encodeURIComponent(value);
    }

    function decodeUser(value) {
        try {
            return decodeURIComponent(value);
        } catch (_error) {
            return value;
        }
    }

    function encodePath(path) {
        var out = "";
        for (var i = 0; i < path.length; i++) {
            var code = path.charCodeAt(i);
            if (
                code <= 0x20 ||
                code === 0x7f ||
                code === 0x22 ||
                code === 0x3c ||
                code === 0x3e ||
                code === 0x5c ||
                code === 0x5e ||
                code === 0x60 ||
                code === 0x7b ||
                code === 0x7c ||
                code === 0x7d
            ) {
                var hex = code.toString(16).toUpperCase();
                if (hex.length < 2) hex = "0" + hex;
                out += "%" + hex;
            } else {
                out += path.charAt(i);
            }
        }
        return out;
    }

    function removeDotSegments(pathname) {
        var absolute = pathname.charCodeAt(0) === 47;
        var parts = pathname.split("/");
        var out = [];
        for (var i = 0; i < parts.length; i++) {
            var part = parts[i];
            if (part === ".") continue;
            if (part === "..") {
                if (out.length > 1 || (out.length === 1 && out[0] !== "")) out.pop();
                continue;
            }
            out.push(part);
        }
        var result = out.join("/");
        if (absolute && (result === "" || result.charCodeAt(0) !== 47)) result = "/" + result;
        if (result === "") return absolute ? "/" : "";
        return result;
    }

    function normalizePath(pathname, protocol) {
        var path = String(pathname);
        if (isSpecialProtocol(protocol)) path = path.replace(/\\/g, "/");
        path = path.replace(/%2e/gi, ".");
        return removeDotSegments(encodePath(path));
    }

    function parseAbs(input, base) {
        input = cleanUrl(input);
        if (base != null && String(base) !== "" && !leadingScheme(input)) {
            input = resolveRelative(input, cleanUrl(base));
        }
        var protocol = leadingScheme(input);
        if (!protocol) throw new TypeError("Invalid URL");
        var hierarchicalAt = input.indexOf("://");
        var special = isSpecialProtocol(protocol);
        if (special && hierarchicalAt !== protocol.length - 1) {
            throw new TypeError("Invalid URL");
        }
        if (special) input = input.replace(/\\/g, "/");
        if (!special && hierarchicalAt !== protocol.length - 1) {
            var opaqueRest = input.slice(protocol.length);
            var opaqueHash = opaqueRest.indexOf("#");
            var hash = "";
            if (opaqueHash >= 0) {
                hash = opaqueRest.slice(opaqueHash);
                opaqueRest = opaqueRest.slice(0, opaqueHash);
            }
            var opaqueQuery = opaqueRest.indexOf("?");
            var search = "";
            var pathname = opaqueRest;
            if (opaqueQuery >= 0) {
                search = opaqueRest.slice(opaqueQuery).replace(/ /g, "%20");
                pathname = opaqueRest.slice(0, opaqueQuery);
            }
            if (hash) hash = hash.replace(/ /g, "%20");
            return {
                protocol: protocol,
                hostname: "",
                port: "",
                pathname: pathname,
                search: search,
                hash: hash,
                username: "",
                password: "",
                opaque: true,
            };
        }

        var afterHost = protocol.length + 2;
        var pathStart = input.indexOf("/", afterHost);
        var qIdx = input.indexOf("?", afterHost);
        var hashIdx = input.indexOf("#", afterHost);
        var endPath = earlierIndex(qIdx, hashIdx);
        if (endPath < 0) endPath = input.length;
        var pathname =
            pathStart >= 0 && pathStart <= endPath ? input.slice(pathStart, endPath) : "/";
        var search = qIdx >= 0 ? input.slice(qIdx, hashIdx >= 0 ? hashIdx : input.length) : "";
        var hash = hashIdx >= 0 ? input.slice(hashIdx) : "";
        search = search.replace(/ /g, "%20");
        hash = hash.replace(/ /g, "%20");
        var hostEnd = pathStart >= 0 && pathStart <= endPath ? pathStart : endPath;
        var hostport = input.slice(afterHost, hostEnd);
        var username = "";
        var password = "";
        var at = hostport.lastIndexOf("@");
        if (at >= 0) {
            var info = hostport.slice(0, at);
            hostport = hostport.slice(at + 1);
            var infoColon = info.indexOf(":");
            if (infoColon >= 0) {
                username = decodeUser(info.slice(0, infoColon));
                password = decodeUser(info.slice(infoColon + 1));
            } else {
                username = decodeUser(info);
            }
        }
        var hostname;
        var port = "";
        if (hostport.charCodeAt(0) === 91) {
            var end = hostport.indexOf("]");
            if (end < 0) throw new TypeError("Invalid URL");
            hostname = hostport.slice(0, end + 1);
            if (end + 1 < hostport.length) {
                if (hostport.charCodeAt(end + 1) !== 58) throw new TypeError("Invalid URL");
                port = hostport.slice(end + 2);
            }
        } else {
            var colon = hostport.lastIndexOf(":");
            if (colon >= 0) {
                hostname = hostport.slice(0, colon);
                port = hostport.slice(colon + 1);
            } else {
                hostname = hostport;
            }
            hostname = hostname.toLowerCase();
        }
        if (port) {
            if (!/^[0-9]+$/.test(port) || Number(port) > 65535) throw new TypeError("Invalid URL");
            port = String(Number(port));
            if (port === defaultPort(protocol)) port = "";
        }
        if (protocol !== "file:" && !hostname) throw new TypeError("Invalid URL");
        return {
            protocol: protocol,
            hostname: hostname,
            port: port,
            pathname: normalizePath(pathname, protocol),
            search: search,
            hash: hash,
            username: username,
            password: password,
            opaque: false,
        };
    }

    function resolveRelative(input, base) {
        input = cleanUrl(input);
        base = cleanUrl(base);
        if (leadingScheme(input)) return input;
        var protocol = leadingScheme(base);
        if (!protocol || base.indexOf("://") !== protocol.length - 1) {
            throw new TypeError("Invalid URL");
        }
        var after = protocol.length + 2;
        var pathStart = base.indexOf("/", after);
        var qBase = base.indexOf("?", after);
        var hBase = base.indexOf("#", after);
        var originEnd = pathStart >= 0 ? pathStart : earlierIndex(qBase, hBase);
        if (originEnd < 0) originEnd = base.length;
        var origin = base.slice(0, originEnd);
        var basePathEnd = earlierIndex(qBase, hBase);
        if (basePathEnd < 0) basePathEnd = base.length;
        var basePath = pathStart >= 0 ? base.slice(pathStart, basePathEnd) : "/";
        if (!input) {
            var keptSearch = qBase >= 0 ? base.slice(qBase, hBase >= 0 ? hBase : base.length) : "";
            return origin + basePath + keptSearch;
        }
        var c0 = input.charCodeAt(0);
        var c1 = input.length > 1 ? input.charCodeAt(1) : 0;
        if (c0 === 47 && c1 === 47) return base.slice(0, protocol.length) + input;
        if (c0 === 47) return origin + input;
        if (c0 === 63) return origin + basePath + input;
        if (c0 === 35) {
            var search = qBase >= 0 ? base.slice(qBase, hBase >= 0 ? hBase : base.length) : "";
            return origin + basePath + search + input;
        }
        var slash = basePath.lastIndexOf("/");
        return origin + basePath.slice(0, slash + 1) + input;
    }

    function hostOf(url) {
        return url._port ? url._hostname + ":" + url._port : url._hostname;
    }

    function rebuildHref(url) {
        if (url._opaque) {
            url._href = url._protocol + url._pathname + url._search + url._hash;
            return;
        }
        var user = "";
        if (url._username || url._password) {
            user = encodeUser(url._username);
            if (url._password) user += ":" + encodeUser(url._password);
            user += "@";
        }
        url._href =
            url._protocol +
            "//" +
            user +
            hostOf(url) +
            url._pathname +
            url._search +
            url._hash;
    }

    function assignParsed(url, parsed) {
        url._protocol = parsed.protocol;
        url._hostname = parsed.hostname;
        url._port = parsed.port;
        url._pathname = parsed.pathname;
        url._hash = parsed.hash;
        url._username = parsed.username;
        url._password = parsed.password;
        url._opaque = parsed.opaque;
        url._search = parsed.search;
        if (url._sp) url._sp._list = parseQuery(url._search);
        rebuildHref(url);
    }

    function URL(input, base) {
        if (!(this instanceof URL)) return new URL(input, base);
        assignParsed(this, parseAbs(input, base));
        this._sp = null;
    }

    Object.defineProperty(URL.prototype, "href", {
        get() {
            return this._href;
        },
        set(value) {
            assignParsed(this, parseAbs(value, null));
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "protocol", {
        get() {
            return this._protocol;
        },
        set(value) {
            var protocol = String(value).toLowerCase();
            if (protocol.charCodeAt(protocol.length - 1) !== 58) protocol += ":";
            if (!/^[a-z][a-z0-9+.-]*:$/.test(protocol)) return;
            if (isSpecialProtocol(protocol) && this._opaque) return;
            this._protocol = protocol;
            if (this._port === defaultPort(protocol)) this._port = "";
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "username", {
        get() {
            return this._username;
        },
        set(value) {
            if (this._opaque) return;
            this._username = String(value);
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "password", {
        get() {
            return this._password;
        },
        set(value) {
            if (this._opaque) return;
            this._password = String(value);
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "hostname", {
        get() {
            return this._hostname;
        },
        set(value) {
            if (this._opaque) return;
            var hostname = String(value);
            if (!hostname || /[\/?#]/.test(hostname)) return;
            if (hostname.charCodeAt(0) !== 91) hostname = hostname.toLowerCase();
            this._hostname = hostname;
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "port", {
        get() {
            return this._port;
        },
        set(value) {
            if (this._opaque) return;
            var port = String(value);
            if (port === "") {
                this._port = "";
                rebuildHref(this);
                return;
            }
            if (!/^[0-9]+$/.test(port) || Number(port) > 65535) return;
            port = String(Number(port));
            this._port = port === defaultPort(this._protocol) ? "" : port;
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "host", {
        get() {
            return this._opaque ? "" : hostOf(this);
        },
        set(value) {
            if (this._opaque) return;
            var host = String(value);
            if (!host || /[\/?#@]/.test(host)) return;
            var hostname;
            var port = "";
            if (host.charCodeAt(0) === 91) {
                var end = host.indexOf("]");
                if (end < 0) return;
                hostname = host.slice(0, end + 1);
                if (end + 1 < host.length) {
                    if (host.charCodeAt(end + 1) !== 58) return;
                    port = host.slice(end + 2);
                }
            } else {
                var colon = host.lastIndexOf(":");
                if (colon >= 0) {
                    hostname = host.slice(0, colon).toLowerCase();
                    port = host.slice(colon + 1);
                } else {
                    hostname = host.toLowerCase();
                }
            }
            if (port && (!/^[0-9]+$/.test(port) || Number(port) > 65535)) return;
            if (port) {
                port = String(Number(port));
                if (port === defaultPort(this._protocol)) port = "";
            }
            this._hostname = hostname;
            this._port = port;
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "pathname", {
        get() {
            return this._pathname;
        },
        set(value) {
            var path = String(value);
            if (!this._opaque && path.charCodeAt(0) !== 47) path = "/" + path;
            this._pathname = this._opaque ? path : normalizePath(path, this._protocol);
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "hash", {
        get() {
            return this._hash;
        },
        set(value) {
            var hash = String(value == null ? "" : value);
            if (!hash) this._hash = "";
            else this._hash = hash.charCodeAt(0) === 35 ? hash : "#" + hash;
            rebuildHref(this);
        },
        configurable: true,
    });
    Object.defineProperty(URL.prototype, "origin", {
        get() {
            if (this._opaque || !isSpecialProtocol(this._protocol) || this._protocol === "file:") {
                return "null";
            }
            var scheme = this._protocol;
            if (scheme === "ws:") scheme = "http:";
            if (scheme === "wss:") scheme = "https:";
            return scheme + "//" + hostOf(this);
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
