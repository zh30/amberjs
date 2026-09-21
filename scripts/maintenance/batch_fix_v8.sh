#!/bin/bash

# Amber V8 API 兼容性批量修复脚本 v2.0
# 更精确的替换模式，避免之前的错误

echo "🔧 开始批量修复 V8 API 兼容性问题..."

# 备份当前状态
echo "📦 创建备份..."
git stash

# 1. 修复 to_array 错误 - 更精确的模式
echo "1️⃣ 修复 to_array 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' '
s/\.to_array(scope)/.is_array() \&\& v8::Local::<v8::Array>::try_from(\&).unwrap()/g
' {} \;

# 2. 修复 buffer().data() 错误
echo "2️⃣ 修复 buffer().data() 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' '
s/\.buffer()\.data()/\.backing_store()\.data()/g
' {} \;

# 3. 修复 to_function(scope) 错误 - 更精确的模式
echo "3️⃣ 修复 to_function 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' '
s/\.to_function(scope)/.is_function() \&\& v8::Local::<v8::Function>::try_from(\&).unwrap()/g
' {} \;

# 4. 修复 ReturnValue::default() 错误
echo "4️⃣ 修复 ReturnValue::default() 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' '
s/v8::ReturnValue::default()/v8::ReturnValue::new()/g
' {} \;

echo "✅ 批量修复完成！"
echo "📊 请运行 'cargo check' 查看剩余错误数量"

# 显示前10个错误
echo ""
echo "🔍 当前错误预览："
cargo check 2>&1 | grep "error\[E0" | head -10 || echo "✅ 无错误或检查未完成"