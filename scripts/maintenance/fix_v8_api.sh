#!/bin/bash

# Amber V8 API 兼容性问题批量修复脚本
# 用于修复 rusty_v8 0.22 -> 0.32 的 API 变更

echo "正在修复 V8 API 兼容性问题..."

# 修复 to_array 错误
echo "修复 to_array 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/\.to_array(scope)/is_array() \&\& v8::Local::<v8::Array>::try_from(\0).unwrap()/g' {} \;
find src/ -name "*.rs" -type f -exec sed -i '' 's/if let Some(arr) = \(.*\)\.to_array(scope)/if \1.is_array() {\n        let arr = v8::Local::<v8::Array>::try_from(\1).unwrap()/g' {} \;
find src/ -name "*.rs" -type f -exec sed -i '' 's/\.to_function(scope)/is_function() \&\& v8::Local::<v8::Function>::try_from(\0).unwrap()/g' {} \;

# 修复 buffer().data() 错误
echo "修复 buffer().data() 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/\.buffer()\.data()/\.backing_store()\.data()/g' {} \;

# 修复 FunctionCallbackArguments::new 错误
echo "修复 FunctionCallbackArguments::new 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/v8::FunctionCallbackArguments::new(scope, &\[\])/v8::FunctionCallbackArguments::from_function_args(scope, \&[])/g' {} \;

# 修复 ReturnValue::default() 错误
echo "修复 ReturnValue::default() 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/v8::ReturnValue::default()/v8::ReturnValue::new()/g' {} \;

# 修复 set_prototype_property_initializer_callback 错误 (如果存在)
echo "修复 set_prototype_property_initializer_callback 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/set_prototype_property_initializer_callback/set_access_check_callback/g' {} \;

# 修复 PropertyAttribute::None 错误
echo "修复 PropertyAttribute::None 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/PropertyAttribute::None/PropertyAttribute::EMPTY/g' {} \;

# 修复 set_on_instance 错误
echo "修复 set_on_instance 错误..."
find src/ -name "*.rs" -type f -exec sed -i '' 's/set_on_instance/set_accessor/g' {} \;

echo "V8 API 兼容性问题修复完成！"