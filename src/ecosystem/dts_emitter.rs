// .d.ts 文件发射器
// Stage 91 Phase 3.2.1 - 类型定义文件输出
//
/// 将类型分析结果转换为 TypeScript 声明文件
use super::*;
use std::collections::HashMap;
use std::collections::{BTreeMap};
use std::path::Path;
/// .d.ts 文件发射器
#[derive(Debug)]
pub struct DtsEmitter {
    config: EmitterConfig,
    indent_level: usize,
}
impl DtsEmitter {
    /// 创建新的发射器
    pub fn new() -> Self {
        Self {
            config: EmitterConfig::default(),
            indent_level: 0,
        }
    }
    /// 发射类型定义
    pub fn emit_types(&self, types: &HashMap<String, TypeDefinition>, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();
        // 添加文件头
        output.push_str(&format!("// Type definitions for {}\n", filename));
        output.push_str("// Project: Amber Runtime\n");
        output.push_str("// Definitions by: Amber Type Generator\n\n");
        // 添加模块声明
        output.push_str(&format!("declare module '{}' {{\n", self.get_module_name(filename));
        self.indent();
        // 发射所有类型
        for (_name, type_def) in types {
            let type_output: _ = self.emit_type_definition(type_def)?;
            output.push_str(&type_output);
            output.push('\n');
        }
        self.dedent();
        output.push_str("}\n");
        Ok(output)
    }
    /// 发射单个类型定义
    pub fn emit_type_definition(&self, type_def: &TypeDefinition) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();
        // 添加 JSDoc 注释
        if let Some(ref js_doc) = type_def.js_doc {
            for line in js_doc.lines() {
                output.push_str(&format!("{}{}\n", self.get_indent(), line.trim());
            }
        }
        match type_def.kind {
            TypeKind::Interface => {
                output.push_str(&format!("{}export interface {}", self.get_indent(), type_def.name));
                // 类型参数
                if !type_def.type_params.is_empty() {
                    output.push_str(&format!("<{}>", type_def.type_params.join(", "));
                }
                // 继承
                if !type_def.extends.is_empty() {
                    output.push_str(&format!(" extends {}", type_def.extends.join(", "));
                }
                output.push_str(" {\n");
                self.indent();
                // 成员
                for (_member_name, member) in &type_def.members {
                    let member_output: _ = self.emit_type_member(member)?;
                    output.push_str(&member_output);
                    output.push('\n');
                }
                self.dedent();
                output.push_str(&format!("{}}}\n", self.get_indent());
            }
            TypeKind::Class => {
                output.push_str(&format!("{}export class {}", self.get_indent(), type_def.name));
                // 类型参数
                if !type_def.type_params.is_empty() {
                    output.push_str(&format!("<{}>", type_def.type_params.join(", "));
                }
                // 继承
                if let Some(_extends) = type_def.extends.first() {
                    // output.push_str(&format!(" extends {}", extends));
                }
                output.push_str(" {\n");
                self.indent();
                // 构造函数
                output.push_str(&format!("{}constructor();\n", self.get_indent());
                // 成员
                for (_member_name, member) in &type_def.members {
                    let member_output: _ = self.emit_type_member(member)?;
                    output.push_str(&member_output);
                    output.push('\n');
                }
                self.dedent();
                output.push_str(&format!("{}}}\n", self.get_indent());
            }
            TypeKind::TypeAlias => {
                output.push_str(&format!("{}export type {} = ", self.get_indent(), type_def.name));
                // 类型参数
                if !type_def.type_params.is_empty() {
                    output.push_str(&format!("<{}> = ", type_def.type_params.join(", "));
                }
                output.push_str("unknown;\n");
            }
            TypeKind::Function => {
                output.push_str(&format!("{}export function {}(): void;\n", self.get_indent(), type_def.name));
            }
            _ => {
                output.push_str(&format!("{}export const {}: any;\n", self.get_indent(), type_def.name));
            }
        }
        Ok(output)
    }
    /// 发射类型成员
    pub fn emit_type_member(&self, member: &TypeMember) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();
        // JSDoc
        if let Some(ref js_doc) = member.js_doc {
            for line in js_doc.lines() {
                output.push_str(&format!("{}{}\n", self.get_indent(), line.trim());
            }
        }
        match &member.member_type {
            MemberType::Property(prop_type) => {
                output.push_str(&format!("{}{}", self.get_indent(), member.name));
                if member.optional {
                    output.push('?');
                }
                output.push_str(&format!(": {}", self.emit_type(prop_type)?));
                output.push(';');
            }
            MemberType::Method(return_type) => {
                output.push_str(&format!("{}{}", self.get_indent(), member.name));
                output.push_str("(");
                output.push_str("): ");
                output.push_str(&self.emit_type(return_type)?);
                output.push(';');
            }
            MemberType::Indexer(indexer_type) => {
                output.push_str(&format!("{}[key: {}]: ", self.get_indent(), self.emit_type(indexer_type)?));
                output.push_str("any;");
            }
        }
        Ok(output)
    }
    /// 发射类型
    pub fn emit_type(&self, type_info: &Type) -> Result<String, Box<dyn std::error::Error>> {
        match type_info {
            Type::Primitive(primitive) => match primitive {
                PrimitiveType::String => Ok("string".to_string()),
                PrimitiveType::Number => Ok("number".to_string()),
                PrimitiveType::Boolean => Ok("boolean".to_string()),
                PrimitiveType::Undefined => Ok("undefined".to_string()),
                PrimitiveType::Null => Ok("null".to_string()),
                PrimitiveType::Void => Ok("void".to_string()),
                PrimitiveType::Any => Ok("any".to_string()),
                PrimitiveType::Never => Ok("never".to_string()),
            },
            Type::Array(element_type) => {
                let inner: _ = self.emit_type(element_type)?;
                Ok(format!("{}[]", inner))
            }
            Type::Union(types) => {
                let type_strs: Result<Vec<String>, _> = types.iter().map(|t| self.emit_type(t)).collect();
                Ok(type_strs?.join(" | "))
            }
            Type::Intersection(types) => {
                let type_strs: Result<Vec<String>, _> = types.iter().map(|t| self.emit_type(t)).collect();
                Ok(type_strs?.join(" & "))
            }
            Type::Object(properties) => {
                let mut output = String::new();
                output.push_str("{\n");
                self.indent();
                for (prop_name, prop_type) in properties {
                    output.push_str(&format!("{}{}: {},\n", self.get_indent(), prop_name, self.emit_type(prop_type)?));
                }
                self.dedent();
                output.push_str(&format!("{}}}", self.get_indent());
                Ok(output)
            }
            Type::Function {
                params,
                return_type,
                type_params,
            } => {
                let mut output = String::new();
                if !type_params.is_empty() {
                    output.push_str(&format!("<{}>", type_params.join(", "));
                }
                output.push('(');
                let param_strs: Result<Vec<String>, _> = params.iter().map(|p| self.emit_type(p)).collect();
                output.push_str(&param_strs?.join(", "));
                output.push_str(") => ");
                output.push_str(&self.emit_type(return_type)?);
                Ok(output)
            }
            Type::TypeRef(type_name) => Ok(type_name.clone()),
            Type::Generic { base, args } => {
                let base_str: _ = self.emit_type(base)?;
                let arg_strs: Result<Vec<String>, _> = args.iter().map(|a| self.emit_type(a)).collect();
                Ok(format!("{}<{}>", base_str, arg_strs?.join(", "))
            }
            Type::Unknown => Ok("unknown".to_string()),
        }
    }
    /// 发射项目范围的类型定义
    pub fn emit_project_types(&self, project_info: &ProjectTypeInfo) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();
        // 文件头
        output.push_str("// Type definitions for Amber Project\n");
        output.push_str("// Generated automatically by Amber Type Generator\n\n");
        // 全局类型
        if !project_info.globals.is_empty() {
            output.push_str("declare global {\n");
            self.indent();
            for (_name, type_def) in &project_info.globals {
                let type_output: _ = self.emit_type_definition(type_def)?;
                output.push_str(&type_output);
                output.push('\n');
            }
            self.dedent();
            output.push_str("}\n\n");
        }
        // 模块类型
        for (_module_name, module_info) in &project_info.modules {
            output.push_str(&format!("declare module '{}' {{\n", module_info.name));
            self.indent();
            for (_export_name, type_def) in &module_info.exports {
                let type_output: _ = self.emit_type_definition(type_def)?;
                output.push_str(&type_output);
                output.push('\n');
            }
            self.dedent();
            output.push_str("}\n\n");
        }
        Ok(output)
    }
    /// 发射索引文件
    pub fn emit_index_file(&self, project_info: &ProjectTypeInfo) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();
        output.push_str("// Type definition index\n");
        output.push_str("// Auto-generated by Amber Type Generator\n\n");
        // 导出所有类型
        for (file_path, file_info) in &project_info.files {
            output.push_str(&format!("// {}\n", file_path));
            for (export_name, _type_def) in &file_info.exports {
                output.push_str(&format!(
                    "export {{ {} }} from './{}';\n",
                    export_name,
                    file_path.trim_end_matches(".ts").trim_end_matches(".js"));
            }
            output.push('\n');
        }
        Ok(output)
    }
    /// 获取缩进字符串
    fn get_indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }
    /// 增加缩进级别
    fn indent(&mut self) {
        self.indent_level += 1;
    }
    /// 减少缩进级别
    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
    /// 获取模块名
    fn get_module_name(&self, filename: &str) -> String {
        // 简化实现 - 从文件名提取模块名
        let path: _ = std::path::Path::new(filename);
        if let Some(stem) = path.file_stem() {
            stem.to_string_lossy().to_string()
        } else {
            "unknown".to_string()
        }
    }
}
/// 发射器配置
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    pub include_comments: bool,
    pub pretty_print: bool,
    pub sort_members: bool,
    pub include_source_map: bool,
}
impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            include_comments: true,
            pretty_print: true,
            sort_members: true,
            include_source_map: false,
        }
    }
}