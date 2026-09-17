---
title: "WICG 导入映射与原生插件 (Import Maps & Native Addons)"
subtitle: "符合标准规范的裸模块导入重映射与 Node-API C/C++ 动态链接原生扩展支持"
group: "Agent 与高级特性"
id: "import-maps-native"
---

随着前端现代化的推进以及跨平台原生 C/C++ 扩展的需求，Beejs 提供了对 **WICG 导入映射 (Import Maps)** 与 **Node-API 原生扩展动态加载 (Native Addons)** 的深度支持。

---

## 1. WICG 导入映射 (`--import-map`)

### 1.1 什么是 Import Maps？
WICG Import Maps 是浏览器和现代 JavaScript 运行时的一项标准规范，允许开发者重映射模块导入说明符（Bare Specifiers），无需构建工具即可直接在代码中使用裸模块名称。

### 1.2 编写 `import_map.json`

```json
{
  "imports": {
    "lodash": "./vendor/lodash.js",
    "components/": "./src/components/",
    "chalk": "https://esm.sh/chalk@5"
  }
}
```

### 1.3 运行与打包支持

在执行脚本或进行模块打包时传入 `--import-map`：

```bash
# 运行时直接根据映射表解析导入
$ bee run --import-map import_map.json app.ts

# 打包时自动应用重映射
$ bee bundle app.ts -o dist/bundle.js --import-map import_map.json
```

在你的代码中：
```typescript
// 直接导入裸模块名称，Beejs 会自动将其重映射到 ./vendor/lodash.js
import _ from 'lodash';
import Button from 'components/Button.tsx';
```

---

## 2. Node-API 与 C/C++ 原生扩展 (`process.dlopen`)

在高性能科学计算、硬件交互或底层系统集成场景中，许多 npm 依赖包包含预编译的 `.node` C/C++ 原生共享库。

### 2.1 原生扩展支持机制
Beejs 在底层实现了与 Node.js 一致的 `process.dlopen` 接口：

```typescript
// 直接调用底层动态库加载
const addonModule = { exports: {} };
process.dlopen(addonModule, './build/Release/my_native_addon.node');
console.log(addonModule.exports.calculate());
```

### 2.2 CommonJS 自动派发
当在代码中使用 `require()` 引用 `.node` 文件时，Beejs 的模块解析器会自动拦截并将其委派给动态链接器（Unix `libc::dlopen` / `libc::dlsym`）：

```javascript
// 自动调用 process.dlopen 加载 native addon
const nativeBinding = require('./addon.node');
```

---

## 3. 最佳实践
- **微服务与无 node_modules 架构**：通过 Import Maps 直接指定本地共享包路径，彻底摆脱嵌套深层 `node_modules` 的解析开销；
- **平台动态兼容**：支持根据系统平台派发不同架构的 `.node` 预编译二进制文件。
