# Amber V8 0.20 兼容性修复报告

## 修复日期
2025-12-17

## 概述
修复了 Amber 高性能 JavaScript/TypeScript 运行时中 V8 引擎的兼容性问题和编译错误。

## 已修复的问题

### 1. 字符串字面量错误 ✅
- **问题**: 第 425 行使用单引号而不是双引号
- **修复**: 将 `'./'` 改为 `"./"`，`'../'` 改为 `"../"`

### 2. 变量名错误 ✅
- **问题**: 第 290 行变量 `_scope` 不存在，应该是 `scope`
- **修复**: 将 `_scope` 改为 `scope`

### 3. 生命周期规范 ✅
- **问题**: 多个函数缺少生命周期参数
- **修复**: 为以下函数添加了生命周期参数：
  - `create_process_object<'a>()`
  - `load_and_execute_module<'a>()`
  - `get_builtin_module<'a>()`
  - `setup_nodejs_apis<'a>()`
  - `setup_process<'a>()`
  - `setup_path<'a>()`
  - `setup_fs<'a>()`
  - `setup_module_system<'a>()`

### 4. 未使用的导入 ✅
- **问题**: 导入了未使用的 `Arc` 和 `Lazy`
- **修复**: 移除了 `std::sync::Arc` 和 `once_cell::sync::Lazy`

### 5. 线程安全 ✅
- **问题**: `v8::Global` 不能在线程间发送，导致 `MODULE_CACHE` 编译失败
- **修复**: 将全局缓存改为线程本地存储：
  ```rust
  std::thread_local! {
      static MODULE_CACHE: std::cell::RefCell<HashMap<String, v8::Global<v8::Object>>> =
          std::cell::RefCell::new(HashMap::new());
  }
  ```
- **更新**: 修改了所有缓存访问点使用 `MODULE_CACHE.with()`

### 6. V8 平台初始化 ✅
- **问题**: V8 0.20 中 `new_default_platform()` API 变化
- **修复**:
  - 移除了参数：`v8::new_default_platform(0, true)` → `v8::  - 添加了new_default_platform()`
 `.make_shared()`: `v8::new_default_platform().make_shared()`

### 7. V8 API 返回类型 ✅
- **问题**: `global.set()` 和 `process.set()` 返回 `Option` 而不是 `Result`
- **修复**: 添加了 `.ok_or_else()` 转换：
  ```rust
  global.set(scope, key.into(), value.into())
      .ok_or_else(|| anyhow!("Failed to set on global"))?;
  ```

### 8. V8 数组 API ✅
- **问题**: `Array::new_with_length` API 变化
- **修复**: `v8::Array::new(scope, 2)` （保留长度参数）

### 9. V8 Context API ✅
- **问题**: 需要显式传递 context 参数
- **修复**: 更新了所有函数签名以接收 context，并在 lib.rs 中传递

### 10. V8 Null 值 ✅
- **问题**: `set_undefined()` 方法不存在
- **修复**: 使用 `v8::null(scope)` 创建 null 值

### 11. 字符串到 Local 的转换 ✅
- **问题**: V8 0.20 中字符串字面量不能直接转换为 `Local<Value>`
- **修复**: 使用 `v8::String::new(scope, "string").unwrap().into()`

## 待修复的问题

### 1. 剩余的 V8 API 兼容性问题
- `v8::Array::new_with_length` 在某些位置仍然存在
- 字符串字面量转换问题在多个位置仍然存在
- 需要系统性地更新所有 V8 API 调用

### 2. 修复策略
1. 批量替换所有字符串字面量为 `v8::String::new()`
2. 修复所有 `process.set()` 调用
3. 统一使用 V8 0.20 的 API

## 测试状态
- 编译错误数量：从 77 个减少到约 20 个
- 主要障碍：V8 0.20 API 的系统性变化
- 下一步：继续修复剩余的 API 兼容性问题

## 经验教训
1. **V8 版本升级影响巨大**: V8 0.20 引入了很多 API 变化
2. **生命周期管理复杂**: V8 的生命周期管理需要精细控制
3. **类型安全严格**: V8 Rust 绑定对类型要求很严格
4. **文档重要性**: 需要参考最新的 V8 文档进行适配

## 技术决策
1. **保留 V8 0.20**: 尽管有兼容性问题，但新版本性能更好
2. **渐进式修复**: 逐步修复每个错误而不是一次性全部重写
3. **测试驱动**: 通过编译错误指导修复过程

---
*本报告记录了 Amber 运行时 V8 兼容性修复的过程，为后续开发提供参考。*
