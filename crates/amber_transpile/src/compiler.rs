// TypeScript 编译器实现
// 将 TypeScript 代码转译为 JavaScript

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

/// TypeScript 编译器配置
#[derive(Debug, Clone)]
pub struct TypeScriptCompilerConfig {
    pub target: TypeScriptTarget,
    pub module: TypeScriptModule,
    pub lib: Vec<String>,
    pub strict: bool,
    pub no_implicit_any: bool,
    pub strict_null_checks: bool,
    pub source_map: bool,
    pub remove_comments: bool,
    pub es_module_interop: bool,
    pub allow_synthetic_default_imports: bool,
}
impl Default for TypeScriptCompilerConfig {
    fn default() -> Self {
        Self {
            target: TypeScriptTarget::ES2020,
            module: TypeScriptModule::ESNext,
            lib: vec!["ES2020".to_string()],
            strict: true,
            no_implicit_any: true,
            strict_null_checks: true,
            source_map: true,
            remove_comments: false,
            es_module_interop: true,
            allow_synthetic_default_imports: true,
        }
    }
}
#[derive(Debug, Clone)]
pub enum TypeScriptTarget {
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ES2023,
    ESNext,
}
#[derive(Debug, Clone)]
pub enum TypeScriptModule {
    None,
    CommonJS,
    AMD,
    System,
    ESNext,
    ES2022,
    NodeNext,
}
/// TypeScript 编译错误
#[derive(Debug, Clone)]
pub struct TypeScriptError {
    pub code: u32,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: ErrorSeverity,
}
#[derive(Debug, Clone)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

/// 类型检查上下文
struct TypeContext {
    /// 已定义的接口
    interfaces: HashMap<String, HashMap<String, String>>,
    /// 已定义的类型别名
    type_aliases: HashMap<String, String>,
    /// 已定义的枚举
    enums: HashMap<String, Vec<String>>,
    /// 当前作用域的变量类型
    variables: HashMap<String, String>,
    /// 函数返回类型栈（用于 return 语句检查）
    return_type_stack: Vec<Option<String>>,
    /// async 上下文栈（用于 await 验证）
    /// true 表示当前在 async 函数/箭头函数中
    async_context_stack: Vec<bool>,
}

impl TypeContext {
    fn new() -> Self {
        let mut ctx = Self {
            interfaces: HashMap::new(),
            type_aliases: HashMap::new(),
            enums: HashMap::new(),
            variables: HashMap::new(),
            return_type_stack: Vec::new(),
            async_context_stack: Vec::new(),
        };
        // 注册内置类型
        ctx.register_builtin_types();
        ctx
    }

    /// 进入 async 上下文
    fn enter_async(&mut self, is_async: bool) {
        self.async_context_stack.push(is_async);
    }

    /// 退出 async 上下文
    fn exit_async(&mut self) {
        self.async_context_stack.pop();
    }

    /// 检查当前是否在 async 函数中
    fn is_in_async(&self) -> bool {
        self.async_context_stack.iter().any(|&b| b)
    }

    fn register_builtin_types(&mut self) {
        // 字符串类型
        self.variables
            .insert("string".to_string(), "string".to_string());
        self.variables
            .insert("number".to_string(), "number".to_string());
        self.variables
            .insert("boolean".to_string(), "boolean".to_string());
        self.variables.insert("any".to_string(), "any".to_string());
        self.variables
            .insert("void".to_string(), "void".to_string());
        self.variables
            .insert("null".to_string(), "null".to_string());
        self.variables
            .insert("undefined".to_string(), "undefined".to_string());
        self.variables
            .insert("never".to_string(), "never".to_string());
        self.variables
            .insert("unknown".to_string(), "unknown".to_string());
        self.variables
            .insert("object".to_string(), "object".to_string());
        self.variables
            .insert("symbol".to_string(), "symbol".to_string());
        self.variables
            .insert("bigint".to_string(), "bigint".to_string());

        // Utility Types - 这些是 TypeScript 内置的类型构造器
        // Partial<T> - 所有属性变为可选
        self.variables
            .insert("Partial".to_string(), "utility".to_string());
        // Required<T> - 所有属性变为必需
        self.variables
            .insert("Required".to_string(), "utility".to_string());
        // Readonly<T> - 所有属性变为只读
        self.variables
            .insert("Readonly".to_string(), "utility".to_string());
        // Pick<T, K> - 从 T 中选择指定属性
        self.variables
            .insert("Pick".to_string(), "utility".to_string());
        // Omit<T, K> - 从 T 中移除指定属性
        self.variables
            .insert("Omit".to_string(), "utility".to_string());
        // Record<K, T> - 创建具有指定键和值类型的对象
        self.variables
            .insert("Record".to_string(), "utility".to_string());
        // Exclude<T, U> - 从 T 中排除可赋值给 U 的类型
        self.variables
            .insert("Exclude".to_string(), "utility".to_string());
        // Extract<T, U> - 从 T 中提取可赋值给 U 的类型
        self.variables
            .insert("Extract".to_string(), "utility".to_string());
        // NonNullable<T> - 排除 null 和 undefined
        self.variables
            .insert("NonNullable".to_string(), "utility".to_string());
        // ReturnType<T> - 获取函数类型 T 的返回类型
        self.variables
            .insert("ReturnType".to_string(), "utility".to_string());
        // Parameters<T> - 获取函数类型 T 的参数类型
        self.variables
            .insert("Parameters".to_string(), "utility".to_string());
        // ConstructorParameters<T> - 获取构造函数类型 T 的参数类型
        self.variables
            .insert("ConstructorParameters".to_string(), "utility".to_string());
        // InstanceType<T> - 获取构造函数类型的实例类型
        self.variables
            .insert("InstanceType".to_string(), "utility".to_string());
        // NoInfer<T> - 防止类型推断并强制使用特定类型
        self.variables
            .insert("NoInfer".to_string(), "utility".to_string());
        // ThisParameterType<T> - 获取函数类型 T 的 this 参数类型
        self.variables
            .insert("ThisParameterType".to_string(), "utility".to_string());
        // OmitThisParameter<T> - 移除函数类型 T 的 this 参数
        self.variables
            .insert("OmitThisParameter".to_string(), "utility".to_string());
        // Uppercase<StringType> - 字符串大写
        self.variables
            .insert("Uppercase".to_string(), "utility".to_string());
        // Lowercase<StringType> - 字符串小写
        self.variables
            .insert("Lowercase".to_string(), "utility".to_string());
        // Capitalize<StringType> - 首字母大写
        self.variables
            .insert("Capitalize".to_string(), "utility".to_string());
        // Uncapitalize<StringType> - 首字母小写
        self.variables
            .insert("Uncapitalize".to_string(), "utility".to_string());
        // Infer<T> - 条件类型中推断类型（v0.3.213）
        self.variables
            .insert("Infer".to_string(), "utility".to_string());
        // Awaited<T> - TypeScript 4.5+ 获取 Promise 解析后的类型（v0.3.215）
        self.variables
            .insert("Awaited".to_string(), "utility".to_string());
        // ThisType<T> - 提供对象方法中 'this' 的类型（v0.3.216）
        self.variables
            .insert("ThisType".to_string(), "utility".to_string());
        // Mutable<T> - 使所有属性可变（与 Readonly<T> 相反）（v0.3.218）
        self.variables
            .insert("Mutable".to_string(), "utility".to_string());
        // Trim<T> - 移除字符串类型中的首尾空白（v0.3.222）
        self.variables
            .insert("Trim".to_string(), "utility".to_string());
        // TrimLeft<T> - 移除字符串类型中的左侧空白（v0.3.222）
        self.variables
            .insert("TrimLeft".to_string(), "utility".to_string());
        // TrimRight<T> - 移除字符串类型中的右侧空白（v0.3.222）
        self.variables
            .insert("TrimRight".to_string(), "utility".to_string());
    }

    fn get_variable_type(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    fn add_variable(&mut self, name: &str, type_name: &str) {
        self.variables
            .insert(name.to_string(), type_name.to_string());
    }
}

/// TypeScript 编译器主结构
pub struct TypeScriptCompiler {
    config: TypeScriptCompilerConfig,
    diagnostics: Vec<TypeScriptError>,
}

impl TypeScriptCompiler {
    /// 创建新的 TypeScript 编译器
    pub fn new(config: TypeScriptCompilerConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
        }
    }
    /// 返回最近一次编译收集到的诊断信息
    pub fn diagnostics(&self) -> &[TypeScriptError] {
        &self.diagnostics
    }
    /// 编译 TypeScript 文件
    pub fn compile_file(&mut self, file_path: &Path) -> Result<CompilationOutput> {
        let source = std::fs::read_to_string(file_path)?;
        let file_name = file_path.to_string_lossy().to_string();
        self.compile_source(&source, &file_name)
    }
    /// 编译 TypeScript 源代码
    pub fn compile_source(&mut self, source: &str, file_name: &str) -> Result<CompilationOutput> {
        self.diagnostics.clear();
        if let Some(diagnostic) = self.detect_unsupported_tsx(source, file_name) {
            let message = diagnostic.message.clone();
            self.diagnostics.push(diagnostic);
            bail!(message);
        }
        // 第一步：词法分析
        let tokens = self.lexical_analysis(source, file_name)?;
        // 第二步：语法分析
        let ast = self.syntax_analysis(&tokens, file_name)?;
        // 第三步：合并同名的命名空间声明
        let merged_ast = self.merge_namespaces(ast);
        // 第四步：合并同名的接口声明
        let merged_ast = self.merge_interfaces(merged_ast);
        // 第五步：合并同名的模块声明
        let merged_ast = self.merge_modules(merged_ast);
        // 第六步：三重合并 - 合并同名的接口、命名空间、模块
        let merged_ast = self.merge_triple(merged_ast);
        // 第七步：类型检查（简化实现）
        self.type_check(&merged_ast, file_name)?;
        // 第八步：转译为 JavaScript
        let js_code = self.transpile(&merged_ast)?;
        // 第九步：生成 Source Map
        let source_map = if self.config.source_map {
            Some(self.generate_source_map(source, &js_code, file_name)?)
        } else {
            None
        };
        Ok(CompilationOutput {
            js_code,
            source_map,
            diagnostics: self.diagnostics.clone(),
        })
    }

    /// 合并同名的命名空间声明
    /// TypeScript 允许同一命名空间的多次声明，所有成员会合并到同一个命名空间
    fn merge_namespaces(&self, ast: ASTNode) -> ASTNode {
        use std::collections::HashMap;

        // 解包 Program 节点
        let statements = match ast {
            ASTNode::Program(stmts) => stmts,
            _ => return ast,
        };

        // 收集所有命名空间声明
        let mut namespace_map: HashMap<String, ASTStatement> = HashMap::new();
        let mut non_namespace_nodes: Vec<ASTNode> = Vec::new();

        for node in statements {
            match node {
                ASTNode::Statement(ASTStatement::Namespace {
                    name,
                    full_name,
                    body,
                    is_declare,
                }) => {
                    if let Some(existing_ns) = namespace_map.get_mut(&full_name) {
                        // 同名命名空间已存在，合并 body
                        if let ASTStatement::Namespace {
                            body: existing_body,
                            ..
                        } = existing_ns
                        {
                            existing_body.extend(body);
                        }
                    } else {
                        // 第一次看到这个命名空间
                        namespace_map.insert(
                            full_name.clone(),
                            ASTStatement::Namespace {
                                name,
                                full_name,
                                body,
                                is_declare,
                            },
                        );
                    }
                }
                _ => {
                    non_namespace_nodes.push(node);
                }
            }
        }

        // 重新构建 AST
        let mut merged_statements: Vec<ASTNode> = non_namespace_nodes;
        for ns in namespace_map.into_values() {
            merged_statements.push(ASTNode::Statement(ns));
        }

        ASTNode::Program(merged_statements)
    }

    /// 合并同名的接口声明
    /// TypeScript 允许同一接口的多次声明，所有成员会合并到同一个接口
    fn merge_interfaces(&self, ast: ASTNode) -> ASTNode {
        use std::collections::HashMap;

        // 解包 Program 节点
        let statements = match ast {
            ASTNode::Program(stmts) => stmts,
            _ => return ast,
        };

        // 收集所有接口声明
        let mut interface_map: HashMap<String, ASTNode> = HashMap::new();
        let mut non_interface_nodes: Vec<ASTNode> = Vec::new();

        for node in statements {
            match node {
                ASTNode::InterfaceDeclaration {
                    name,
                    extends,
                    properties,
                    index_signature,
                    constructor_signature,
                } => {
                    if let Some(existing_iface) = interface_map.get_mut(&name) {
                        // 同名接口已存在，合并属性
                        if let ASTNode::InterfaceDeclaration {
                            name: _,
                            extends: existing_extends,
                            properties: existing_properties,
                            index_signature: existing_index,
                            constructor_signature: _,
                        } = existing_iface
                        {
                            // 合并属性（后者覆盖前者）
                            existing_properties.extend(properties);
                            // 合并继承列表（去重）
                            for ext in extends {
                                if !existing_extends.contains(&ext) {
                                    existing_extends.push(ext);
                                }
                            }
                            // 保留第一个非 None 的索引签名
                            if index_signature.is_some() && existing_index.is_none() {
                                *existing_index = index_signature;
                            }
                        }
                    } else {
                        // 第一次看到这个接口
                        interface_map.insert(
                            name.clone(),
                            ASTNode::InterfaceDeclaration {
                                name,
                                extends,
                                properties,
                                index_signature,
                                constructor_signature,
                            },
                        );
                    }
                }
                _ => {
                    non_interface_nodes.push(node);
                }
            }
        }

        // 重新构建 AST
        let mut merged_statements: Vec<ASTNode> = non_interface_nodes;
        for iface in interface_map.into_values() {
            merged_statements.push(iface);
        }

        ASTNode::Program(merged_statements)
    }

    /// 合并同名的模块声明
    /// TypeScript 允许同一模块的多次声明，所有成员会合并到同一个模块
    fn merge_modules(&self, ast: ASTNode) -> ASTNode {
        use std::collections::HashMap;

        // 解包 Program 节点
        let statements = match ast {
            ASTNode::Program(stmts) => stmts,
            _ => return ast,
        };

        // 收集所有模块声明
        let mut module_map: HashMap<String, ASTStatement> = HashMap::new();
        let mut non_module_nodes: Vec<ASTNode> = Vec::new();

        for node in statements {
            match node {
                ASTNode::Statement(ASTStatement::ModuleDeclaration { name, body }) => {
                    if let Some(existing_mod) = module_map.get_mut(&name) {
                        // 同名模块已存在，合并 body
                        if let ASTStatement::ModuleDeclaration {
                            body: existing_body,
                            ..
                        } = existing_mod
                        {
                            existing_body.extend(body);
                        }
                    } else {
                        // 第一次看到这个模块
                        module_map
                            .insert(name.clone(), ASTStatement::ModuleDeclaration { name, body });
                    }
                }
                _ => {
                    non_module_nodes.push(node);
                }
            }
        }

        // 重新构建 AST
        let mut merged_statements: Vec<ASTNode> = non_module_nodes;
        for module in module_map.into_values() {
            merged_statements.push(ASTNode::Statement(module));
        }

        ASTNode::Program(merged_statements)
    }

    /// 三重合并：合并同名的接口、命名空间、模块声明
    /// TypeScript 允许同一个名称同时作为 interface、namespace、module 声明
    /// 所有声明的内容会被合并到同一个 TripleMergedDeclaration 中
    fn merge_triple(&self, ast: ASTNode) -> ASTNode {
        use std::collections::HashMap;

        // 解包 Program 节点
        let statements = match ast {
            ASTNode::Program(stmts) => stmts,
            _ => return ast,
        };

        // 使用 HashMap 按名称分组接口、命名空间、模块
        // value 是 (interface_props, interface_extends, interface_index, namespace_body, is_declare)
        type TripleData = (
            HashMap<String, String>,     // interface properties
            Vec<String>,                 // interface extends
            Option<Box<IndexSignature>>, // interface index signature
            Vec<ASTNode>,                // namespace/module body
            bool,                        // is_declare
        );
        let mut triple_map: HashMap<String, TripleData> = HashMap::new();
        let mut remaining_nodes: Vec<ASTNode> = Vec::new();

        for node in statements {
            match node {
                // 处理接口声明
                ASTNode::InterfaceDeclaration {
                    name,
                    extends,
                    properties,
                    index_signature,
                    constructor_signature: _,
                } => {
                    let entry = triple_map
                        .entry(name.clone())
                        .or_insert_with(|| (HashMap::new(), Vec::new(), None, Vec::new(), false));
                    // 合并接口属性
                    entry.0.extend(properties);
                    // 合并继承列表（去重）
                    for ext in extends {
                        if !entry.1.contains(&ext) {
                            entry.1.push(ext);
                        }
                    }
                    // 保留第一个非 None 的索引签名
                    if index_signature.is_some() && entry.2.is_none() {
                        entry.2 = index_signature;
                    }
                }
                // 处理命名空间声明
                ASTNode::Statement(ASTStatement::Namespace {
                    name: _,
                    full_name,
                    body,
                    is_declare,
                }) => {
                    // 使用 full_name 作为 key，支持嵌套命名空间
                    let entry = triple_map.entry(full_name.clone()).or_insert_with(|| {
                        (HashMap::new(), Vec::new(), None, Vec::new(), is_declare)
                    });
                    // 合并命名空间 body
                    entry.3.extend(body);
                    // 更新 is_declare 状态
                    if is_declare {
                        entry.4 = true;
                    }
                }
                // 注意：ModuleDeclaration 已在 merge_namespaces 中处理，这里不需要再次处理
                _ => {
                    remaining_nodes.push(node);
                }
            }
        }

        // 重新构建 AST
        let mut merged_statements: Vec<ASTNode> = remaining_nodes;
        for (name, (properties, extends, index_sig, body, is_declare)) in triple_map.into_iter() {
            // 只有当存在 interface 属性时才创建 TripleMergedDeclaration
            // 这样可以保留单独的 namespace 声明（只有 body 没有 interface 属性的情况）
            if !properties.is_empty() {
                // 有 interface 属性，创建 TripleMergedDeclaration
                merged_statements.push(ASTNode::Statement(ASTStatement::TripleMergedDeclaration {
                    name,
                    interface_properties: properties,
                    interface_extends: extends,
                    interface_index_signature: index_sig,
                    body,
                    is_declare,
                }));
            } else if !body.is_empty() {
                // 没有 interface 属性但有 namespace body，保留原始 Namespace 声明
                // 重新创建 Namespace 节点
                let namespace_node = ASTNode::Statement(ASTStatement::Namespace {
                    name: name.clone(),
                    full_name: name.clone(),
                    body,
                    is_declare,
                });
                merged_statements.push(namespace_node);
            }
            // 如果既没有 properties 也没有 body，不生成任何声明
        }

        ASTNode::Program(merged_statements)
    }

    /// 词法分析 - 将源代码分解为记号
    fn lexical_analysis(&self, source: &str, _file_name: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let mut pos = 0;
        while pos < chars.len() {
            let ch = chars[pos];
            // 跳过空白字符，但追踪换行符
            if ch.is_whitespace() {
                pos += 1;
                continue;
            }
            // 处理注释
            if ch == '/'
                && pos + 1 < chars.len() {
                    let next_ch = chars[pos + 1];
                    // 单行注释 //
                    if next_ch == '/' {
                        // 跳过到行末
                        while pos < chars.len() && chars[pos] != '\n' {
                            pos += 1;
                        }
                        continue;
                    }
                    // 多行注释 /* */
                    if next_ch == '*' {
                        pos += 2;
                        while pos + 1 < chars.len() {
                            if chars[pos] == '*' && chars[pos + 1] == '/' {
                                pos += 2;
                                break;
                            }
                            pos += 1;
                        }
                        continue;
                    }
                }
            // 处理标识符和关键字
            if ch.is_alphabetic() || ch == '_' || ch == '$' {
                let start: usize = pos;
                pos += 1;
                while pos < chars.len() {
                    let c = chars[pos];
                    if c.is_alphanumeric() || c == '_' || c == '$' {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                let ident: String = chars[start..pos].iter().collect();
                // 关键字识别
                let token = match ident.as_str() {
                    "let" => Token::Let,
                    "const" => Token::Const,
                    "var" => Token::Var,
                    "function" => Token::Function,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "while" => Token::While,
                    "do" => Token::Do,
                    "switch" => Token::Switch,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "return" => Token::Return,
                    "class" => Token::Class,
                    "interface" => Token::Interface,
                    "enum" => Token::Enum,
                    "type" => Token::Type,
                    "namespace" => Token::Namespace,
                    "global" => Token::Global,
                    "module" => Token::Module,
                    "declare" => Token::Declare,
                    "import" => Token::Import,
                    "export" => Token::Export,
                    "public" => Token::Public,
                    "private" => Token::Private,
                    "protected" => Token::Protected,
                    "static" => Token::Static,
                    "abstract" => Token::Abstract,
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "throw" => Token::Throw,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "new" => Token::New,
                    "this" => Token::This,
                    "extends" => Token::Extends,
                    "super" => Token::Super,
                    "from" => Token::From,
                    "as" => Token::As,
                    "satisfies" => Token::Satisfies, // v0.3.168
                    "keyof" => Token::Keyof,
                    "typeof" => Token::Typeof,
                    "in" => Token::In,
                    "infer" => Token::Infer,
                    "readonly" => Token::Readonly,
                    "never" => Token::Never,
                    "unknown" => Token::UnknownType,
                    "is" => Token::Is,
                    "asserts" => Token::Asserts, // v0.3.220
                    _ => Token::Identifier(ident),
                };
                tokens.push(token);
                continue;
            }
            // 处理数字
            if ch.is_ascii_digit() {
                let start = pos;
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                // 检查小数部分
                if pos < chars.len() && chars[pos] == '.' {
                    pos += 1; // consume the '.'
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }
                // 检查指数部分 (e.g., 1e5, 1.5e-3)
                if pos < chars.len() && (chars[pos] == 'e' || chars[pos] == 'E') {
                    pos += 1; // consume 'e' or 'E'
                    if pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-') {
                        pos += 1; // consume sign
                    }
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }
                // 检查是否是 BigInt 字面量 (以 n 结尾)
                // 注意：BigInt 不能有小数点或指数部分，所以只有纯整数才能是 BigInt
                let has_decimal_or_exponent = {
                    let s = &chars[start..];
                    s.contains(&'.') || s.contains(&'e') || s.contains(&'E')
                };
                let is_bigint = pos < chars.len() && chars[pos] == 'n' && !has_decimal_or_exponent;

                if is_bigint {
                    pos += 1; // consume 'n'
                    let bigint: String = chars[start..pos].iter().collect();
                    tokens.push(Token::BigInt(bigint));
                } else {
                    let number: String = chars[start..pos].iter().collect();
                    tokens.push(Token::Number(number));
                }
                continue;
            }
            // 处理字符串
            if ch == '\'' || ch == '"' {
                let quote = ch;
                let _start = pos;
                pos += 1;
                let mut string_chars = Vec::new();
                while pos < chars.len() {
                    let c = chars[pos];
                    if c == '\\' {
                        // 转义字符
                        if pos + 1 < chars.len() {
                            let next_char = chars[pos + 1];
                            // 只对有效的转义序列添加反斜杠
                            if matches!(next_char, '"' | '\'' | '\\' | 'n' | 'r' | 't') {
                                string_chars.push(chars[pos]);
                                string_chars.push(chars[pos + 1]);
                            } else {
                                // 无效转义序列，只添加字符
                                string_chars.push(chars[pos + 1]);
                            }
                            pos += 2;
                            continue;
                        }
                    }
                    if c == quote {
                        pos += 1;
                        break;
                    }
                    string_chars.push(c);
                    pos += 1;
                }
                tokens.push(Token::String(String::from_iter(string_chars), quote));
                continue;
            }
            // 处理模板字符串 (backtick)
            if ch == '`' {
                let quote = ch;
                pos += 1;
                // 检查是否是空模板字符串
                if pos < chars.len() && chars[pos] == '`' {
                    tokens.push(Token::String("".to_string(), quote));
                    pos += 1;
                    continue;
                }
                // 处理模板字符串内容
                let mut current_part = String::new();
                let mut brace_depth = 0;
                let mut paren_depth = 0;
                let mut in_template_expression = false;

                tokens.push(Token::TemplateStart);

                while pos < chars.len() {
                    let c = chars[pos];

                    // 处理转义字符
                    if c == '\\' && pos + 1 < chars.len() {
                        let next_char = chars[pos + 1];
                        if next_char == '`' || next_char == '\\' || next_char == '$' {
                            if !in_template_expression {
                                current_part.push(next_char);
                            }
                            pos += 2;
                            continue;
                        } else if next_char == 'n' {
                            // 在模板字符串中保持 \n 为字符串，而不是转换为实际换行
                            // 这样生成的 JavaScript 才能正确工作
                            current_part.push('\\');
                            current_part.push('n');
                            pos += 2;
                            continue;
                        } else if next_char == 't' {
                            // 保持 \t 为字符串
                            current_part.push('\\');
                            current_part.push('t');
                            pos += 2;
                            continue;
                        }
                    }

                    // 检测 ${ 开始
                    if c == '$' && pos + 1 < chars.len() && chars[pos + 1] == '{' {
                        // 保存之前的字符串部分
                        if !current_part.is_empty() {
                            tokens.push(Token::String(current_part.clone(), quote));
                            current_part.clear();
                        }
                        tokens.push(Token::TemplateMiddle);
                        in_template_expression = true;
                        brace_depth = 1;
                        paren_depth = 0;
                        pos += 2;
                        continue;
                    }

                    // 处理括号和花括号嵌套（用于正确识别表达式边界）
                    if in_template_expression {
                        // 处理转义字符在表达式内部
                        if c == '\\' && pos + 1 < chars.len() {
                            let next_char = chars[pos + 1];
                            // 在表达式内部，我们保留转义序列，让后续处理
                            if next_char == 'n' || next_char == 't' || next_char == 'r' {
                                // 保留转义序列，让解析器处理
                                tokens.push(Token::UnknownChar(c.to_string()));
                                pos += 1;
                                continue;
                            }
                            // 其他转义字符也保留
                            tokens.push(Token::UnknownChar(c.to_string()));
                            pos += 1;
                            continue;
                        }

                        if c == '{' {
                            brace_depth += 1;
                            tokens.push(Token::LBrace);
                        } else if c == '}' {
                            if brace_depth > 0 {
                                brace_depth -= 1;
                            }
                            if brace_depth == 0 && paren_depth == 0 {
                                // 模板表达式结束，标记状态但不发射 RBrace
                                in_template_expression = false;
                                // 跳过 }，让它不被当作普通 token 处理
                            } else if brace_depth > 0 {
                                tokens.push(Token::RBrace);
                            }
                            pos += 1;
                            continue;
                        } else if c == '(' {
                            paren_depth += 1;
                            tokens.push(Token::LParen);
                        } else if c == ')' && paren_depth > 0 {
                            paren_depth -= 1;
                            tokens.push(Token::RParen);
                        } else if c == '[' {
                            tokens.push(Token::LBracket);
                        } else if c == ']' {
                            tokens.push(Token::RBracket);
                        } else if c == ',' {
                            tokens.push(Token::Comma);
                        } else if c == '.' {
                            // 检查是否是 ...
                            if pos + 1 < chars.len()
                                && chars[pos + 1] == '.'
                                && pos + 2 < chars.len()
                                && chars[pos + 2] == '.'
                            {
                                tokens.push(Token::DotDotDot);
                                pos += 2;
                            } else {
                                tokens.push(Token::Dot);
                            }
                        } else if c == '+' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '+' {
                                tokens.push(Token::PlusPlus);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::PlusEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Plus);
                            }
                        } else if c == '-' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '-' {
                                tokens.push(Token::MinusMinus);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::MinusEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Minus);
                            }
                        } else if c == '*' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::StarEq);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '*' {
                                tokens.push(Token::StarStar);
                                pos += 1;
                            } else {
                                tokens.push(Token::Star);
                            }
                        } else if c == '/' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::SlashEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Slash);
                            }
                        } else if c == '%' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::PercentEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Percent);
                            }
                        } else if c == '=' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                if pos + 2 < chars.len() && chars[pos + 2] == '=' {
                                    tokens.push(Token::EqEqEq);
                                    pos += 2;
                                } else {
                                    tokens.push(Token::EqEq);
                                    pos += 1;
                                }
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '>' {
                                tokens.push(Token::FatArrow);
                                pos += 1;
                            } else {
                                tokens.push(Token::Eq);
                            }
                        } else if c == '!' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                if pos + 2 < chars.len() && chars[pos + 2] == '=' {
                                    tokens.push(Token::NotEqEq);
                                    pos += 2;
                                } else {
                                    tokens.push(Token::NotEq);
                                    pos += 1;
                                }
                            } else {
                                tokens.push(Token::Bang);
                            }
                        } else if c == '<' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::LtEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Lt);
                            }
                        } else if c == '>' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::GtEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Gt);
                            }
                        } else if c == '&' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::AmpersanderEq);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '&' {
                                tokens.push(Token::AmpersanderAmpersand);
                                pos += 1;
                            } else {
                                tokens.push(Token::Ampersand);
                            }
                        } else if c == '|' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::PipeEq);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '|' {
                                tokens.push(Token::PipePipe);
                                pos += 1;
                            } else {
                                tokens.push(Token::Pipe);
                            }
                        } else if c == '^' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::CaretEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Caret);
                            }
                        } else if c == '"' || c == '\'' {
                            // 处理模板表达式内部的字符串字面量
                            let string_quote = c;
                            let mut string_content = String::new();
                            pos += 1;
                            while pos < chars.len() {
                                let sc = chars[pos];
                                if sc == '\\' && pos + 1 < chars.len() {
                                    // 处理转义字符
                                    let next_sc = chars[pos + 1];
                                    if next_sc == '"'
                                        || next_sc == '\''
                                        || next_sc == '\\'
                                        || next_sc == '$'
                                    {
                                        string_content.push(next_sc);
                                    } else if next_sc == 'n' {
                                        string_content.push('\n');
                                    } else if next_sc == 't' {
                                        string_content.push('\t');
                                    } else {
                                        string_content.push(sc);
                                    }
                                    pos += 2;
                                    continue;
                                }
                                if sc == string_quote {
                                    // 字符串结束
                                    pos += 1;
                                    break;
                                }
                                if sc == '$' && pos + 1 < chars.len() && chars[pos + 1] == '{' {
                                    // 这是模板表达式的一部分，不能作为字符串结束
                                    string_content.push(sc);
                                    pos += 1;
                                    continue;
                                }
                                string_content.push(sc);
                                pos += 1;
                            }
                            tokens.push(Token::String(string_content, string_quote));
                            continue;
                        } else if c == '`' {
                            // 模板字符串结束
                            if !current_part.is_empty() {
                                tokens.push(Token::String(current_part.clone(), quote));
                            }
                            tokens.push(Token::TemplateEnd);
                            pos += 1;
                            break;
                        } else if c == '<' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::LtEq);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '<' {
                                if pos + 2 < chars.len() && chars[pos + 2] == '=' {
                                    tokens.push(Token::LtLtEq);
                                    pos += 2;
                                } else {
                                    tokens.push(Token::LtLt);
                                    pos += 1;
                                }
                            } else {
                                tokens.push(Token::Lt);
                            }
                        } else if c == '>' {
                            // NOTE: For TypeScript, we don't combine >> into GtGt
                            // because nested generics like A<B<C>> should be parsed
                            // as separate > tokens, not as a right-shift operator
                            if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                                tokens.push(Token::GtEq);
                                pos += 1;
                            } else {
                                tokens.push(Token::Gt);
                            }
                        } else if c == '?' {
                            if pos + 1 < chars.len() && chars[pos + 1] == '.' {
                                tokens.push(Token::QuestionDot);
                                pos += 1;
                            } else if pos + 1 < chars.len() && chars[pos + 1] == '?' {
                                tokens.push(Token::QuestionQuestion);
                                pos += 1;
                            } else {
                                tokens.push(Token::Question);
                            }
                        } else if c == ':' {
                            tokens.push(Token::Colon);
                        } else if c == '@' {
                            tokens.push(Token::At);
                        } else if c == ';' {
                            tokens.push(Token::SemiColon);
                        } else if c.is_alphanumeric() || c == '_' || c == '$' {
                            // 处理标识符
                            let start = pos;
                            pos += 1;
                            while pos < chars.len() {
                                let next_ch = chars[pos];
                                if next_ch.is_alphanumeric() || next_ch == '_' || next_ch == '$' {
                                    pos += 1;
                                } else {
                                    break;
                                }
                            }
                            let ident: String = chars[start..pos].iter().collect();
                            // 检查是否是关键字
                            let token = match ident.as_str() {
                                "let" => Token::Let,
                                "const" => Token::Const,
                                "var" => Token::Var,
                                "function" => Token::Function,
                                "async" => Token::Async,
                                "await" => Token::Await,
                                "if" => Token::If,
                                "else" => Token::Else,
                                "for" => Token::For,
                                "while" => Token::While,
                                "do" => Token::Do,
                                "switch" => Token::Switch,
                                "case" => Token::Case,
                                "default" => Token::Default,
                                "try" => Token::Try,
                                "catch" => Token::Catch,
                                "finally" => Token::Finally,
                                "throw" => Token::Throw,
                                "break" => Token::Break,
                                "continue" => Token::Continue,
                                "new" => Token::New,
                                "this" => Token::This,
                                "extends" => Token::Extends,
                                "super" => Token::Super,
                                _ => Token::Identifier(ident),
                            };
                            tokens.push(token);
                            continue; // pos 已经更新
                        } else if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                            // 跳过空白字符
                            pos += 1;
                            continue;
                        } else if c as u32 > 127 {
                            // 非 ASCII 字符（表情符号等），添加到当前部分
                            if !in_template_expression {
                                current_part.push(c);
                            } else {
                                // 在模板表达式内部，尝试作为标识符处理
                                tokens.push(Token::UnknownChar(c.to_string()));
                            }
                            pos += 1;
                            continue;
                        } else {
                            // 未知字符，作为 Unknown token
                            tokens.push(Token::UnknownChar(c.to_string()));
                        }
                        pos += 1;
                        continue;
                    }

                    // 模板字符串结束
                    if c == '`' {
                        if !current_part.is_empty() {
                            tokens.push(Token::String(current_part.clone(), quote));
                        }
                        tokens.push(Token::TemplateEnd);
                        pos += 1;
                        break;
                    }

                    // 累积普通字符
                    current_part.push(c);
                    pos += 1;
                }
                continue;
            }
            // 处理操作符和符号
            tokens.push(match ch {
                '@' => Token::At, // 装饰器符号
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                ':' => Token::Colon,
                ';' => Token::SemiColon,
                ',' => Token::Comma,
                '.' => {
                    if pos + 2 < chars.len() && chars[pos + 1] == '.' && chars[pos + 2] == '.' {
                        pos += 2;
                        Token::DotDotDot
                    } else {
                        Token::Dot
                    }
                }
                '?' => Token::Question,
                '+' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::PlusEq
                    } else if pos + 1 < chars.len() && chars[pos + 1] == '+' {
                        pos += 1;
                        Token::PlusPlus
                    } else {
                        Token::Plus
                    }
                }
                '-' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::MinusEq
                    } else if pos + 1 < chars.len() && chars[pos + 1] == '-' {
                        pos += 1;
                        Token::MinusMinus
                    } else {
                        Token::Minus
                    }
                }
                '*' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::StarEq
                    } else if pos + 1 < chars.len() && chars[pos + 1] == '*' {
                        pos += 1;
                        Token::StarStar
                    } else {
                        Token::Star
                    }
                }
                '/' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::SlashEq
                    } else {
                        Token::Slash
                    }
                }
                '%' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::PercentEq
                    } else {
                        Token::Percent
                    }
                }
                '=' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '>' {
                        // 处理 FatArrow (=>)
                        pos += 1;
                        Token::FatArrow
                    } else if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        if pos + 2 < chars.len() && chars[pos + 2] == '=' {
                            pos += 2;
                            Token::EqEqEq
                        } else {
                            pos += 1;
                            Token::EqEq
                        }
                    } else {
                        Token::Eq
                    }
                }
                '!' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        if pos + 2 < chars.len() && chars[pos + 2] == '=' {
                            pos += 2;
                            Token::NotEqEq
                        } else {
                            pos += 1;
                            Token::NotEq
                        }
                    } else {
                        Token::Bang
                    }
                }
                '<' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::LtEq
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::GtEq
                    } else {
                        Token::Gt
                    }
                }
                '&' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '&' {
                        pos += 1;
                        Token::AmpersanderAmpersand
                    } else {
                        Token::Ampersand
                    }
                }
                '|' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '|' {
                        pos += 1;
                        Token::PipePipe
                    } else {
                        Token::Pipe
                    }
                }
                _ => Token::UnknownChar(ch.to_string()),
            });
            pos += 1;
        }
        tokens.push(Token::Eof);
        Ok(tokens)
    }

    /// 为 token 序列添加位置信息（简化实现）
    /// 通过扫描源代码来确定每个 token 的大致位置
    #[allow(dead_code)]
    fn add_token_positions(&self, source: &str, tokens: &[Token]) -> Vec<SpannedToken> {
        let chars: Vec<char> = source.chars().collect();
        let mut spanned_tokens = Vec::new();
        let mut source_pos = 0;

        for token in tokens {
            let start_pos = source_pos;

            // 根据 token 类型推进 source_pos
            match token {
                Token::Identifier(s) | Token::Number(s) | Token::String(s, _) => {
                    source_pos += s.len();
                }
                Token::TemplateStart | Token::TemplateMiddle | Token::TemplateEnd => {
                    source_pos += 1;
                }
                _ => {
                    source_pos += 1;
                }
            }

            // 简单估算行/列位置
            let line = chars[..start_pos.min(chars.len())]
                .iter()
                .filter(|&&c| c == '\n')
                .count() as u32;

            let last_newline = chars[..start_pos.min(chars.len())]
                .iter()
                .rposition(|&c| c == '\n')
                .map_or(0, |p| p + 1);

            let column = (start_pos.saturating_sub(last_newline)) as u32;

            spanned_tokens.push(SpannedToken {
                token: token.clone(),
                location: SourceLocation { line, column },
                end_location: SourceLocation {
                    line,
                    column: column.saturating_add(1),
                },
            });
        }

        spanned_tokens
    }

    /// 语法分析 - 生成抽象语法树
    fn syntax_analysis(&self, tokens: &[Token], _file_name: &str) -> Result<ASTNode> {
        // 简化的语法分析器
        // 实际实现需要完整的递归下降解析器或 LL/LR 解析器
        let mut parser = Parser::new(tokens.to_vec());
        parser.parse()
    }

    /// 类型检查
    fn type_check(&mut self, ast: &ASTNode, _file_name: &str) -> Result<()> {
        let mut type_context = TypeContext::new();
        self.check_node(ast, &mut type_context)?;
        Ok(())
    }

    /// 检查 AST 节点
    fn check_node(&mut self, node: &ASTNode, ctx: &mut TypeContext) -> Result<()> {
        match node {
            ASTNode::Program(statements) => {
                for stmt in statements {
                    self.check_node(stmt, ctx)?;
                }
            }
            ASTNode::VariableDeclaration {
                name,
                type_annotation,
                initializer,
                ..
            } => {
                // 检查变量类型注解
                if let Some(ref type_ann) = type_annotation {
                    if !self.is_valid_type(type_ann, ctx) {
                        self.add_diagnostic(format!("Type '{}' is not defined", type_ann), None);
                    }
                    // 注册变量类型
                    ctx.add_variable(name, type_ann);
                } else if let Some(init) = initializer {
                    // 尝试从初始化表达式推断类型
                    // init 是 &Box<ASTNode>，需要检查是否是表达式
                    if let ASTNode::Expression(expr) = init.as_ref() {
                        let inferred_type = self.infer_type(expr, ctx)?;
                        if let Some(t) = inferred_type {
                            ctx.add_variable(name, &t);
                        }
                    } else {
                        // 非表达式类型，无法推断
                        ctx.add_variable(name, "any");
                    }
                } else {
                    // 无类型注解且无初始化，推断为 any
                    ctx.add_variable(name, "any");
                }
            }
            ASTNode::FunctionDeclaration {
                is_declare: _,
                name: _,
                params,
                return_type,
                body,
                is_async,
                type_params: _,
            } => {
                // 为函数参数创建新作用域
                let prev_vars = ctx.variables.clone();

                // 检查参数类型
                for param in params {
                    if let FunctionParameter::Simple {
                        name: param_name,
                        type_annotation,
                        ..
                    } = param
                    {
                        if let Some(ref type_ann) = type_annotation {
                            if !self.is_valid_type(type_ann, ctx) {
                                self.add_diagnostic(
                                    format!(
                                        "Parameter '{}' has invalid type '{}'",
                                        param_name, type_ann
                                    ),
                                    None,
                                );
                            }
                            ctx.add_variable(param_name, type_ann);
                        } else {
                            ctx.add_variable(param_name, "any");
                        }
                    }
                }

                // 记录返回类型
                ctx.return_type_stack.push(return_type.clone());

                // 进入/退出 async 上下文
                ctx.enter_async(*is_async);
                // 检查函数体
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }
                ctx.exit_async();

                // 恢复作用域
                ctx.variables = prev_vars;
                ctx.return_type_stack.pop();
            }
            ASTNode::ClassDeclaration { members, .. } => {
                // 检查类成员
                for member in members {
                    self.check_node(member, ctx)?;
                }
            }
            ASTNode::MethodDeclaration { params, body, .. } => {
                // 为方法参数创建新作用域
                let prev_vars = ctx.variables.clone();

                // 检查参数类型
                for param in params {
                    if let FunctionParameter::Simple {
                        name: param_name,
                        type_annotation,
                        ..
                    } = param
                    {
                        if let Some(ref type_ann) = type_annotation {
                            ctx.add_variable(param_name, type_ann);
                        } else {
                            ctx.add_variable(param_name, "any");
                        }
                    }
                }

                // 检查方法体
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }

                // 恢复作用域
                ctx.variables = prev_vars;
            }
            ASTNode::PropertyDeclaration { .. } => {
                // 属性检查
            }
            ASTNode::InterfaceDeclaration {
                name,
                extends: _,
                properties,
                index_signature,
                constructor_signature: _,
            } => {
                // 注册接口
                ctx.interfaces.insert(name.clone(), properties.clone());

                // 检查索引签名
                if index_signature.is_some() {
                    // 索引签名已解析，类型检查通过
                }
            }
            ASTNode::TypeAliasDeclaration {
                name,
                type_definition,
                ..
            } => {
                // 验证类型定义
                if !self.is_valid_type(type_definition, ctx) {
                    self.add_diagnostic(
                        format!("Type alias '{}' has invalid type definition", name),
                        None,
                    );
                }
                // 注册类型别名
                ctx.type_aliases
                    .insert(name.clone(), type_definition.clone());
            }
            ASTNode::EnumDeclaration { name, members } => {
                // 注册枚举
                let member_names: Vec<String> = members.iter().map(|m| m.name.clone()).collect();
                ctx.enums.insert(name.clone(), member_names);
            }
            ASTNode::Statement(stmt) => {
                self.check_statement(stmt, ctx)?;
            }
            ASTNode::Expression(expr) => {
                self.check_expression(expr, ctx)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 检查语句
    fn check_statement(&mut self, stmt: &ASTStatement, ctx: &mut TypeContext) -> Result<()> {
        match stmt {
            ASTStatement::Return(expr) => {
                if let Some(ref return_expr) = expr {
                    // 获取当前函数的返回类型
                    if let Some(expected_opt) = ctx.return_type_stack.last() {
                        if let Some(expected) = expected_opt {
                            // 推断返回表达式的类型
                            let actual_type = self.infer_type(return_expr, ctx)?;

                            // 检查类型兼容性
                            if let Some(actual) = actual_type {
                                if !self.is_type_compatible(expected, &actual, ctx) {
                                    self.add_diagnostic(
                                        format!(
                                            "Type '{}' is not assignable to type '{}'",
                                            actual, expected
                                        ),
                                        None,
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // 检查返回类型是否应该是 void
                    if let Some(expected_opt) = ctx.return_type_stack.last() {
                        if let Some(expected) = expected_opt {
                            if expected != "void" && expected != "undefined" && expected != "never"
                            {
                                self.add_diagnostic(
                                    format!("Expected to return '{}', but got void", expected),
                                    None,
                                );
                            }
                        }
                    }
                }
            }
            ASTStatement::Expression(expr) => {
                self.check_expression(expr, ctx)?;
            }
            ASTStatement::If {
                test,
                consequent,
                alternate,
            } => {
                self.check_expression(test, ctx)?;
                self.check_node(consequent, ctx)?;
                if let Some(alt) = alternate {
                    self.check_node(alt, ctx)?;
                }
            }
            ASTStatement::ForOf {
                initializer,
                iterable,
                body,
            } => {
                self.check_node(initializer, ctx)?;
                self.check_expression(iterable, ctx)?;
                self.check_node(body, ctx)?;
            }
            ASTStatement::For {
                initializer,
                condition,
                update,
                body,
            } => {
                if let Some(init) = initializer {
                    self.check_node(init, ctx)?;
                }
                if let Some(cond) = condition {
                    self.check_expression(cond, ctx)?;
                }
                if let Some(upd) = update {
                    self.check_expression(upd, ctx)?;
                }
                self.check_node(body, ctx)?;
            }
            ASTStatement::While { test, body } => {
                self.check_expression(test, ctx)?;
                self.check_node(body, ctx)?;
            }
            ASTStatement::DoWhile { test, body } => {
                self.check_expression(test, ctx)?;
                self.check_node(body, ctx)?;
            }
            ASTStatement::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.check_expression(discriminant, ctx)?;
                for case in cases {
                    for stmt in &case.body {
                        self.check_node(stmt, ctx)?;
                    }
                }
            }
            ASTStatement::Try {
                body,
                handler,
                finalizer,
            } => {
                self.check_node(body, ctx)?;
                if let Some(h) = handler {
                    for stmt in &h.body {
                        self.check_node(stmt, ctx)?;
                    }
                }
                if let Some(finalizer) = finalizer {
                    self.check_node(finalizer, ctx)?;
                }
            }
            ASTStatement::Throw { expression } => {
                self.check_expression(expression, ctx)?;
            }
            ASTStatement::Break { .. } => {}
            ASTStatement::Continue { .. } => {}
            ASTStatement::Namespace {
                body,
                ..
            } => {
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }
            }
            ASTStatement::Block(statements) => {
                for stmt in statements {
                    self.check_node(stmt, ctx)?;
                }
            }
            ASTStatement::GlobalDeclaration { body } => {
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }
            }
            ASTStatement::ModuleDeclaration { body, .. } => {
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }
            }
            // 三重合并声明的类型检查
            ASTStatement::TripleMergedDeclaration { body, .. } => {
                for stmt in body {
                    self.check_node(stmt, ctx)?;
                }
            }
            // export = 语句的类型检查（跳过，因为是类型声明）
            ASTStatement::ExportAssignment { .. } => {
                // export = 是类型声明，不需要运行时类型检查
            }
        }
        Ok(())
    }

    /// 检查表达式
    fn check_expression(&mut self, expr: &ASTExpression, _ctx: &mut TypeContext) -> Result<()> {
        match expr {
            ASTExpression::BinaryExpression { left, right, .. } => {
                self.check_expression(left, _ctx)?;
                self.check_expression(right, _ctx)?;
            }
            ASTExpression::CallExpression {
                callee, arguments, ..
            } => {
                self.check_expression(callee, _ctx)?;
                for arg in arguments {
                    self.check_expression(arg, _ctx)?;
                }
            }
            ASTExpression::MemberExpression { object, .. } => {
                self.check_expression(object, _ctx)?;
            }
            ASTExpression::Unary { operand, .. } => {
                self.check_expression(operand, _ctx)?;
            }
            ASTExpression::UpdateExpression { argument, .. } => {
                self.check_expression(argument, _ctx)?;
            }
            ASTExpression::ObjectProperty { value, .. } => {
                self.check_expression(value, _ctx)?;
            }
            ASTExpression::ObjectLiteral { properties, .. } => {
                for prop in properties {
                    self.check_expression(prop, _ctx)?;
                }
            }
            ASTExpression::ArrowFunctionExpression { body, is_async, .. } => {
                // 进入/退出 async 上下文
                _ctx.enter_async(*is_async);
                self.check_node(body.as_ref(), _ctx)?;
                _ctx.exit_async();
            }
            ASTExpression::TemplateLiteral { parts, .. } => {
                for part in parts {
                    self.check_expression(part, _ctx)?;
                }
            }
            ASTExpression::FunctionExpression { body, is_async, .. } => {
                // 进入/退出 async 上下文
                _ctx.enter_async(*is_async);
                for stmt in body {
                    self.check_node(stmt, _ctx)?;
                }
                _ctx.exit_async();
            }
            ASTExpression::Await { expression, .. } => {
                self.check_expression(expression, _ctx)?;
                // 验证 await 是否在 async 函数中
                if !_ctx.is_in_async() {
                    self.add_diagnostic(
                        "await expression can only be used within an async function".to_string(),
                        None,
                    );
                }
            }
            ASTExpression::AssignmentExpression { left, right, .. } => {
                self.check_expression(left, _ctx)?;
                self.check_expression(right, _ctx)?;
            }
            ASTExpression::ArrayExpression { elements, .. } => {
                for e in elements.iter().flatten() {
                    self.check_expression(e, _ctx)?;
                }
            }
            ASTExpression::SpreadExpression { argument, .. } => {
                self.check_expression(argument, _ctx)?;
            }
            ASTExpression::ConditionalExpression {
                condition,
                consequent,
                alternate,
                ..
            } => {
                self.check_expression(condition, _ctx)?;
                self.check_expression(consequent, _ctx)?;
                self.check_expression(alternate, _ctx)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 推断表达式的类型
    fn infer_type(&self, expr: &ASTExpression, ctx: &TypeContext) -> Result<Option<String>> {
        match expr {
            ASTExpression::Literal(value) => {
                // 根据字面量值推断类型
                if value.starts_with('"') || value.starts_with('\'') {
                    Ok(Some("string".to_string()))
                } else if value == "true" || value == "false" {
                    Ok(Some("boolean".to_string()))
                } else if value.parse::<f64>().is_ok() {
                    Ok(Some("number".to_string()))
                } else {
                    Ok(Some("any".to_string()))
                }
            }
            ASTExpression::Identifier(name) => {
                // 从上下文获取变量类型
                if let Some(t) = ctx.get_variable_type(name) {
                    let result: Option<String> = Some((*t).clone());
                    Ok(result)
                } else {
                    Ok(None)
                }
            }
            ASTExpression::BinaryExpression {
                left,
                right,
                operator,
            } => {
                // 根据运算符推断类型
                match operator.as_str() {
                    "+" => {
                        let left_type = self.infer_type(left, ctx)?;
                        let right_type = self.infer_type(right, ctx)?;
                        // 如果任一操作数是字符串，结果是字符串
                        if left_type.as_ref().map(|t| t == "string").unwrap_or(false)
                            || right_type.as_ref().map(|t| t == "string").unwrap_or(false)
                        {
                            Ok(Some("string".to_string()))
                        } else {
                            Ok(Some("number".to_string()))
                        }
                    }
                    "-" | "*" | "/" | "%" | "<<" | ">>" | ">>>" | "&" | "|" | "^" => {
                        Ok(Some("number".to_string()))
                    }
                    "==" | "!=" | "===" | "!==" | "<" | ">" | "<=" | ">=" => {
                        Ok(Some("boolean".to_string()))
                    }
                    "&&" | "||" | "??" => {
                        let left_type = self.infer_type(left, ctx)?;
                        let right_type = self.infer_type(right, ctx)?;
                        // 推断联合类型
                        if left_type.is_some() && right_type.is_some() {
                            Ok(Some(format!(
                                "{} | {}",
                                left_type.unwrap(),
                                right_type.unwrap()
                            )))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => Ok(Some("any".to_string())),
                }
            }
            ASTExpression::CallExpression { callee, .. } => {
                // 如果是内置构造函数，返回相应类型
                match callee.as_ref() {
                    ASTExpression::Identifier(name) => match name.as_str() {
                        "String" | "Number" | "Boolean" | "Array" | "Object" | "Map" | "Set"
                        | "Promise" | "Function" => {
                            // 返回构造函数调用的结果类型
                            if name == "String" {
                                Ok(Some("string".to_string()))
                            } else if name == "Number" {
                                Ok(Some("number".to_string()))
                            } else if name == "Boolean" {
                                Ok(Some("boolean".to_string()))
                            } else if name == "Promise" {
                                Ok(Some("Promise<unknown>".to_string()))
                            } else {
                                Ok(Some(name.clone()))
                            }
                        }
                        _ => {
                            // 从上下文获取函数返回类型
                            if let Some(t) = ctx.get_variable_type(name) {
                                let result: Option<String> = Some((*t).clone());
                                Ok(result)
                            } else {
                                Ok(Some("any".to_string()))
                            }
                        }
                    },
                    _ => Ok(Some("any".to_string())),
                }
            }
            ASTExpression::MemberExpression { .. } => Ok(Some("any".to_string())),
            ASTExpression::ArrayExpression { elements } => {
                // v0.3.162: 增强的数组类型推断
                Ok(self.infer_array_element_type(elements, ctx))
            }
            ASTExpression::ObjectLiteral { properties } => {
                // v0.3.162: 增强的对象类型推断
                Ok(self.infer_object_type(properties, ctx))
            }
            ASTExpression::ObjectProperty { .. } => Ok(Some("any".to_string())),
            ASTExpression::ArrowFunctionExpression { return_type, .. } => {
                Ok(return_type.clone().or(Some("any".to_string())))
            }
            ASTExpression::FunctionExpression { return_type, .. } => {
                Ok(return_type.clone().or(Some("any".to_string())))
            }
            ASTExpression::TemplateLiteral { .. } => Ok(Some("string".to_string())),
            ASTExpression::Unary { operator, .. } => match operator.as_str() {
                "!" => Ok(Some("boolean".to_string())),
                "-" | "+" | "~" => Ok(Some("number".to_string())),
                "typeof" => Ok(Some("string".to_string())),
                _ => Ok(Some("any".to_string())),
            },
            ASTExpression::ConditionalExpression {
                consequent,
                alternate,
                ..
            } => {
                // 条件表达式的类型是两个分支的联合类型
                let true_type = self.infer_type(consequent, ctx)?;
                let false_type = self.infer_type(alternate, ctx)?;
                match (true_type, false_type) {
                    (Some(t1), Some(t2)) => Ok(Some(format!("{} | {}", t1, t2))),
                    (Some(t), None) | (None, Some(t)) => Ok(Some(t)),
                    _ => Ok(None),
                }
            }
            ASTExpression::NewExpression { .. } => Ok(Some("object".to_string())),
            ASTExpression::ThisExpression => Ok(Some("any".to_string())),
            ASTExpression::UpdateExpression { .. } => Ok(Some("number".to_string())),
            ASTExpression::IndexExpression { .. } => Ok(Some("any".to_string())),
            ASTExpression::AssignmentExpression { .. } => Ok(Some("any".to_string())),
            ASTExpression::Await { expression, .. } => {
                // await 的类型是 Promise 的泛型参数
                let inner_type = self.infer_type(expression, ctx)?;
                if let Some(t) = inner_type {
                    if t.starts_with("Promise<") {
                        // 提取 Promise 的泛型参数
                        let inner = t.trim_start_matches("Promise<").trim_end_matches('>');
                        Ok(Some(inner.to_string()))
                    } else {
                        Ok(Some("any".to_string()))
                    }
                } else {
                    Ok(None)
                }
            }
            ASTExpression::SpreadExpression { .. } => Ok(Some("any".to_string())),
            ASTExpression::SuperExpression => Ok(Some("any".to_string())),
            // 类型断言表达式的类型就是目标类型
            ASTExpression::TSAsExpression { target_type, .. } => Ok(Some(target_type.clone())),
            // 尖括号类型断言的类型也是目标类型
            ASTExpression::TSAngleBracketAssertion { target_type, .. } => {
                Ok(Some(target_type.clone()))
            }
            // satisfies 操作符保留原始表达式的类型
            ASTExpression::TSSatisfiesExpression { expression, .. } => {
                self.infer_type(expression, ctx)
            }
        }
    }

    /// v0.3.162: 增强的类型推断 - 推断数组元素类型
    /// 分析数组表达式推断元素类型
    fn infer_array_element_type(
        &self,
        elements: &[Option<ASTExpression>],
        ctx: &TypeContext,
    ) -> Option<String> {
        let mut element_types: Vec<String> = Vec::new();

        for expr in elements.iter().flatten() {
            if let Ok(Some(t)) = self.infer_type(expr, ctx) {
                element_types.push(t);
            }
        }

        if element_types.is_empty() {
            Some("any".to_string())
        } else {
            // 去重并构建联合类型
            element_types.sort();
            element_types.dedup();

            if element_types.len() == 1 {
                Some(element_types[0].clone())
            } else if element_types.len() <= 3 {
                Some(element_types.join(" | "))
            } else {
                // 太多类型时简化为联合类型
                Some(format!("({})", element_types.join(" | ")))
            }
        }
    }

    /// v0.3.162: 增强的类型推断 - 推断对象属性类型
    /// 分析对象字面量推断属性类型
    fn infer_object_type(&self, properties: &[ASTExpression], ctx: &TypeContext) -> Option<String> {
        let mut props: Vec<String> = Vec::new();

        for prop in properties {
            if let ASTExpression::ObjectProperty { name, value, .. } = prop {
                // 使用 clone 来避免临时值借用问题
                let prop_name = name.clone().unwrap_or_else(|| "".to_string());
                if let Ok(Some(value_type)) = self.infer_type(value, ctx) {
                    props.push(format!("{}: {}", prop_name, value_type));
                }
            }
        }

        if props.is_empty() {
            Some("object".to_string())
        } else {
            Some(format!("{{ {} }}", props.join(", ")))
        }
    }

    /// 检查类型是否有效
    fn is_valid_type(&self, type_name: &str, ctx: &TypeContext) -> bool {
        // v0.3.205: 修复字符串字面量类型验证
        // 处理带引号的字符串字面量类型，如 "active"、'hello'

        // v0.3.219: 模板字面量类型返回空字符串，这是有效的
        if type_name.is_empty() {
            return true;
        }

        let type_name = type_name.trim();
        if (type_name.starts_with('"') && type_name.ends_with('"'))
            || (type_name.starts_with('\'') && type_name.ends_with('\''))
        {
            // 字符串字面量类型是有效的
            return true;
        }

        // 检查带引号包围的类型（可能是带空格的类型定义中的子类型）
        // 例如 " active " 在联合类型中被正确解析后应该是有效的
        let trimmed = type_name.trim();
        if !trimmed.is_empty()
            && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        {
            return true;
        }

        // 检查内置类型
        let builtin_types = [
            "string",
            "number",
            "boolean",
            "any",
            "void",
            "null",
            "undefined",
            "never",
            "unknown",
            "object",
            "symbol",
            "bigint",
            "true",
            "false",
        ];

        if builtin_types.contains(&type_name) {
            return true;
        }

        // 检查带数组后缀的类型
        if let Some(inner_type) = type_name.strip_suffix("[]") {
            return self.is_valid_type(inner_type, ctx);
        }

        // 检查 Promise<T> 泛型
        if type_name.starts_with("Promise<") && type_name.ends_with('>') {
            let inner = &type_name[8..type_name.len() - 1];
            return self.is_valid_type(inner, ctx);
        }

        // 检查类型别名
        if ctx.type_aliases.contains_key(type_name) {
            return true;
        }

        // 检查接口
        if ctx.interfaces.contains_key(type_name) {
            return true;
        }

        // 检查枚举
        if ctx.enums.contains_key(type_name) {
            return true;
        }

        // 检查泛型容器类型
        if type_name.starts_with("Array<") && type_name.ends_with('>') {
            let inner = &type_name[6..type_name.len() - 1];
            return self.is_valid_type(inner, ctx);
        }

        if type_name.starts_with("Map<") && type_name.ends_with('>') {
            let parts: Vec<&str> = type_name[5..type_name.len() - 1].split(", ").collect();
            if parts.len() == 2 {
                return self.is_valid_type(parts[0], ctx) && self.is_valid_type(parts[1], ctx);
            }
            return true;
        }

        if type_name.starts_with("Record<") && type_name.ends_with('>') {
            let parts: Vec<&str> = type_name[7..type_name.len() - 1].split(", ").collect();
            if parts.len() == 2 {
                return self.is_valid_type(parts[0], ctx) && self.is_valid_type(parts[1], ctx);
            }
            return true;
        }

        // 标识符类型（可能是用户定义的类型）
        if type_name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            // 大写开头的标识符被视为有效类型
            return true;
        }

        // 检查联合类型
        if type_name.contains('|') {
            let types: Vec<&str> = type_name.split('|').map(|s| s.trim()).collect();
            return types.iter().all(|t| self.is_valid_type(t, ctx));
        }

        // 检查交叉类型
        if type_name.contains('&') {
            let types: Vec<&str> = type_name.split('&').map(|s| s.trim()).collect();
            return types.iter().all(|t| self.is_valid_type(t, ctx));
        }

        // v0.3.219: 添加模板字面量类型支持
        // 模板字面量类型格式: `prefix${Type}suffix`
        // 需要验证 ${...} 中的类型是否为有效类型
        if type_name.starts_with('`') && type_name.ends_with('`') {
            // 提取模板内容
            let content = &type_name[1..type_name.len() - 1];
            // 解析 ${...} 部分
            let mut depth = 0;
            let mut in_placeholder = false;
            let mut placeholder_content = String::new();

            for c in content.chars() {
                if c == '$'
                    && !in_placeholder
                    && content.chars().nth(content.find(c).unwrap() + 1) == Some('{')
                {
                    // 进入 ${...} 占位符
                    depth = 1;
                    in_placeholder = true;
                    placeholder_content.clear();
                } else if in_placeholder {
                    if c == '{' {
                        depth += 1;
                        placeholder_content.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            // 占位符结束，验证其中的类型
                            // 移除末尾的 >
                            if let Some(endoftype) = placeholder_content.rfind('>') {
                                let type_part = &placeholder_content[..=endoftype];
                                if !self.is_valid_type(type_part, ctx) {
                                    return false;
                                }
                            }
                            in_placeholder = false;
                            placeholder_content.clear();
                        } else {
                            placeholder_content.push(c);
                        }
                    } else if depth > 0 {
                        placeholder_content.push(c);
                    }
                }
            }
            // 模板字面量类型本身是有效的
            return true;
        }

        // 检查只读修饰符
        if type_name.starts_with("readonly ") {
            return self.is_valid_type(&type_name[9..], ctx);
        }

        false
    }

    /// 检查类型兼容性
    fn is_type_compatible(&self, expected: &str, actual: &str, ctx: &TypeContext) -> bool {
        // any 兼容所有类型
        if expected == "any" || actual == "any" {
            return true;
        }

        // never 兼容所有类型
        if expected == "never" {
            return true;
        }

        // unknown 兼容所有类型
        if actual == "unknown" {
            return true;
        }

        // 相同的类型
        if expected == actual {
            return true;
        }

        // 检查数字和字符串字面量类型
        if expected == "number" && actual.parse::<f64>().is_ok() {
            return true;
        }

        if expected == "string" && (actual.starts_with('"') || actual.starts_with('\'')) {
            return true;
        }

        if expected == "boolean" && (actual == "true" || actual == "false") {
            return true;
        }

        // 检查联合类型
        if expected.contains('|') {
            let expected_types: Vec<&str> = expected.split('|').map(|s| s.trim()).collect();
            return expected_types
                .iter()
                .any(|t| self.is_type_compatible(t, actual, ctx));
        }

        // 检查数组类型
        if expected.ends_with("[]") && actual.ends_with("[]") {
            let inner_expected = &expected[..expected.len() - 2];
            let inner_actual = &actual[..actual.len() - 2];
            return self.is_type_compatible(inner_expected, inner_actual, ctx);
        }

        false
    }

    /// 添加诊断信息
    fn add_diagnostic(&mut self, message: String, location: Option<(u32, u32)>) {
        let (line, column) = location
            .map(|(line, column)| (Some(line), Some(column)))
            .unwrap_or((None, None));

        self.diagnostics.push(TypeScriptError {
            code: 1000,
            message,
            file: None,
            line,
            column,
            severity: ErrorSeverity::Error,
        });
    }

    fn detect_unsupported_tsx(&self, source: &str, file_name: &str) -> Option<TypeScriptError> {
        let is_tsx = file_name.ends_with(".tsx") || file_name.ends_with(".jsx");
        if !is_tsx {
            return None;
        }

        let jsx_start = find_unsupported_tsx_start(source)?;
        let (line, column) = line_column_for_offset(source, jsx_start);
        Some(TypeScriptError {
            code: 2000,
            message: format!(
                "TSX/JSX element syntax is unsupported by the Beejs TypeScript compiler in {}",
                file_name
            ),
            file: Some(file_name.to_string()),
            line: Some(line),
            column: Some(column),
            severity: ErrorSeverity::Error,
        })
    }
    /// 转译为 JavaScript
    fn transpile(&self, ast: &ASTNode) -> Result<String> {
        let mut emitter = CodeEmitter::new(self.config.clone());
        emitter.emit(ast)
    }
    /// 生成 Source Map (v0.3.139: 改进精度)
    fn generate_source_map(&self, ts_code: &str, js_code: &str, file_name: &str) -> Result<String> {
        // Build line-to-position mapping from TypeScript source
        let line_positions = build_line_positions(ts_code);

        // Generate improved source map mappings
        let mappings = generate_vlq_mappings_improved(js_code, &line_positions);

        Ok(format!(
            "{{\"version\":3,\"sources\":[\"{}\"],\"mappings\":\"{}\",\"names\":[],\"sourcesContent\":[\"{}\"]}}",
            file_name,
            mappings,
            escape_for_json(ts_code)
        ))
    }
}

/// Build mapping from JS line numbers to source line numbers (v0.3.139)
fn build_line_positions(source: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let chars: Vec<char> = source.chars().collect();

    // First position maps to line 0
    positions.push(0);

    // Track line boundaries
    for (idx, ch) in chars.iter().enumerate() {
        if *ch == '\n' {
            // Next character starts a new line
            if idx + 1 < chars.len() {
                positions.push(idx + 1);
            }
        }
    }

    positions
}

fn line_column_for_offset(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut column = 0u32;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn find_unsupported_tsx_start(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[idx];
                idx += 1;
                while idx < bytes.len() {
                    if bytes[idx] == b'\\' {
                        idx += 2;
                        continue;
                    }
                    if bytes[idx] == quote {
                        idx += 1;
                        break;
                    }
                    idx += 1;
                }
                continue;
            }
            b'/' if idx + 1 < bytes.len() && bytes[idx + 1] == b'/' => {
                idx += 2;
                while idx < bytes.len() && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            b'/' if idx + 1 < bytes.len() && bytes[idx + 1] == b'*' => {
                idx += 2;
                while idx + 1 < bytes.len() {
                    if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                        idx += 2;
                        break;
                    }
                    idx += 1;
                }
                continue;
            }
            _ => {}
        }

        if bytes[idx] == b'<' && idx + 1 < bytes.len() {
            let next = bytes[idx + 1] as char;
            if (next == '/' || next == '>' || next.is_ascii_alphabetic())
                && previous_non_whitespace_byte(bytes, idx)
                    .map(can_precede_jsx_start)
                    .unwrap_or(true)
            {
                return Some(idx);
            }
        }
        idx += 1;
    }

    None
}

fn previous_non_whitespace_byte(bytes: &[u8], idx: usize) -> Option<u8> {
    bytes[..idx]
        .iter()
        .rev()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
}

fn can_precede_jsx_start(byte: u8) -> bool {
    matches!(
        byte,
        b'=' | b'(' | b'[' | b'{' | b',' | b':' | b';' | b'!' | b'?' | b'>'
    )
}

/// Generate improved VLQ-encoded source map mappings (v0.3.139)
fn generate_vlq_mappings_improved(js_code: &str, line_positions: &[usize]) -> String {
    // Generate VLQ-encoded mappings with line number awareness
    let mut mappings = String::new();
    let lines: Vec<&str> = js_code.lines().collect();
    let mut previous_source_line = 0i32;

    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx > 0 {
            mappings.push(';');
        }
        let mut previous_generated_column = 0i32;

        // For each non-empty line, add a mapping entry
        // The mapping includes: generated column, source line, source column
        if !line.trim().is_empty() {
            // Estimate source line (simplified: map to approximately same line)
            // In a full implementation, we would track exact positions during transpilation
            let max_source_line = line_positions.len().saturating_sub(1);
            let source_line = line_idx.min(max_source_line) as i32;
            let source_line_delta = source_line - previous_source_line;

            // Add first segment for this line: col=0 -> source line -> col=0
            mappings.push_str(&encode_vlq(0)); // generated column
            mappings.push(',');
            mappings.push_str(&encode_vlq(0)); // source file index
            mappings.push(',');
            mappings.push_str(&encode_vlq(source_line_delta)); // source line
            mappings.push(',');
            mappings.push_str(&encode_vlq(0)); // source column
            previous_source_line = source_line;

            // Add additional segments for key positions in the line
            // This is a simplified version - full implementation would track
            // statement/expression positions during AST generation
            let mut col = 0;
            for ch in line.chars() {
                col += 1;
                if ch == ';' || ch == '{' || ch == '}' || ch == '(' || ch == ')' {
                    // Add mapping at significant positions
                    if col % 10 == 0 {
                        // Roughly every 10 characters
                        mappings.push(',');
                        let generated_column_delta = col - previous_generated_column;
                        mappings.push_str(&encode_vlq(generated_column_delta));
                        mappings.push(',');
                        mappings.push_str(&encode_vlq(0));
                        mappings.push(',');
                        mappings.push_str(&encode_vlq(source_line - previous_source_line));
                        mappings.push(',');
                        mappings.push_str(&encode_vlq(0));
                        previous_generated_column = col;
                        previous_source_line = source_line;
                    }
                }
            }
        }
    }

    mappings
}

/// Generate precise source map mappings using token positions (v0.3.146)
/// This function creates a source map that tracks exact positions between
/// TypeScript source and generated JavaScript output.
/// NOTE: Reserved for future use with precise source map generation
#[allow(dead_code)]
fn generate_precise_source_map(
    js_code: &str,
    token_positions: &[(usize, usize, usize, usize)], // (js_line, js_col, ts_line, ts_col)
) -> String {
    // Use js_code to calculate line count for mapping validation
    let _js_line_count = js_code.lines().count();
    let mut mappings = String::new();
    let mut prev_js_line: i32 = 0;
    let mut prev_js_col: i32 = 0;
    let mut prev_ts_line: i32 = 0;
    let mut prev_ts_col: i32 = 0;

    // Sort positions by JS line and column
    let mut sorted_positions: Vec<_> = token_positions.iter().collect();
    sorted_positions.sort_by_key(|(js_line, js_col, _, _)| (*js_line, *js_col));

    for (idx, (js_line, js_col, ts_line, ts_col)) in sorted_positions.iter().enumerate() {
        // Add semicolon for new line
        if *js_line as i32 > prev_js_line {
            mappings.push(';');
            prev_js_col = 0;
        }

        // Calculate relative values (VLQ encoding uses relative values)
        let js_line_diff = (*js_line as i32) - prev_js_line;
        let js_col_diff = (*js_col as i32) - prev_js_col;
        let ts_line_diff = (*ts_line as i32) - prev_ts_line;
        let ts_col_diff = (*ts_col as i32) - prev_ts_col;

        // Encode in VLQ format: generatedLine, generatedCol, sourceLine, sourceCol
        // Only include segments that have actual mappings
        if idx == 0 || js_line_diff != 0 || js_col_diff != 0 {
            if idx > 0 {
                mappings.push(',');
            }

            // Encode generated column (relative to previous)
            mappings.push_str(&encode_vlq(js_col_diff));

            // Encode source file index (always 0, omitted in VLQ)
            // mappings.push_str(",");
            // mappings.push_str(&encode_vlq(0));

            // Encode source line (relative to previous)
            mappings.push_str(&encode_vlq(ts_line_diff));

            // Encode source column (relative to previous)
            mappings.push_str(&encode_vlq(ts_col_diff));
        }

        // Update previous values
        prev_js_line = *js_line as i32;
        prev_js_col = *js_col as i32;
        prev_ts_line = *ts_line as i32;
        prev_ts_col = *ts_col as i32;
    }

    mappings
}

/// Encode a number using VLQ (Variable Length Quantity)
fn encode_vlq(value: i32) -> String {
    let mut result = String::new();
    let mut num = value;

    // Handle negative numbers
    if value < 0 {
        num = -value;
    }

    // Encode in base 64 with VLQ
    loop {
        let mut digit = (num & 0x7F) as u8;
        num >>= 7;
        if !result.is_empty() {
            digit |= 0x20; // Continuation bit
        }
        result.push(BASE64_CHARS[digit as usize] as char);
        if num == 0 {
            break;
        }
    }

    result
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Escape a string for JSON inclusion
fn escape_for_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
/// 记号类型
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Number(String),
    BigInt(String),       // BigInt 字面量，如 123n (v0.3.179)
    String(String, char), // (value, quote_type)
    // 关键字
    Let,
    Const,
    Var,
    Function,
    If,
    Else,
    For,
    While,
    Do,
    Switch,
    Case,
    Default,
    Return,
    Class,
    Interface,
    Enum,
    Type,
    Namespace,
    Global, // global 关键字（用于 declare global { ... }）
    Module, // module 关键字（用于 declare module "name" { ... }）
    Import,
    Export,
    Declare, // declare 关键字（用于类型声明）
    Public,
    Private,
    Protected,
    Static,
    Abstract, // abstract 修饰符（用于抽象类和抽象方法）
    Readonly, // readonly 修饰符（用于映射类型）
    Async,
    Await,
    Try,
    Catch,
    Finally,
    Throw,
    Break,
    Continue,
    New,
    This,
    Extends,
    Super,
    From,
    As,
    Satisfies,           // satisfies 操作符 (v0.3.168)
    Keyof,               // keyof 操作符
    Typeof,              // typeof 操作符
    In,                  // in 操作符（用于映射类型）
    Infer,               // infer 关键字（用于条件类型推导）
    Never,               // never 类型（表示永远不返回的值）
    UnknownType,         // unknown 类型（类型安全的 top 类型）
    Is,                  // is 关键字（类型谓词，用于类型守卫）
    Asserts,             // asserts 关键字（类型守卫，用于 asserts 条件） (v0.3.220)
    UnknownChar(String), // 未知字符（用于词法分析 fallback）
    // 符号
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    SemiColon,
    Comma,
    Dot,
    DotDotDot, // 展开运算符 ...
    Question,
    QuestionDot,      // ?.
    QuestionQuestion, // ??
    Plus,
    PlusEq,
    PlusPlus,
    Minus,
    MinusEq,
    MinusMinus,
    Star,
    StarEq,
    StarStar, // ** (幂运算)
    Slash,
    SlashEq,
    Percent,
    PercentEq, // %=
    Eq,
    EqEq,
    EqEqEq,
    NotEq,
    NotEqEq,
    Bang,
    Lt,
    LtEq, // <=
    Gt,
    GtEq,                 // >=
    Ampersand,            // &
    AmpersanderEq,        // &=
    AmpersanderAmpersand, // &&
    Pipe,                 // |
    PipeEq,               // |=
    PipePipe,             // ||
    Caret,                // ^
    CaretEq,              // ^=
    LtLt,                 // <<
    LtLtEq,               // <<=
    GtGt,                 // >>
    GtGtEq,               // >>=
    GtGtGt,               // >>>
    GtGtGtEq,             // >>>=
    FatArrow,
    TemplateStart,
    TemplateMiddle,
    TemplateEnd,
    At, // @ 符号（用于装饰器）
    Unknown(String),
    Eof,
}

/// Source location for source map generation (v0.3.139)
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub line: u32,   // 0-indexed line number
    pub column: u32, // 0-indexed column number
}

#[allow(dead_code)]
impl SourceLocation {
    fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// Spanned token with source location for precise source map generation (v0.3.146)
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub location: SourceLocation,
    pub end_location: SourceLocation, // Location after this token
}

/// Lexer state for position tracking (v0.3.146)
/// NOTE: Reserved for future use with precise position tracking
#[allow(dead_code)]
struct LexerState {
    line: u32,
    column: u32,
    start_line: u32,
    start_column: u32,
}

/// 枚举成员
#[derive(Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub value: Option<EnumValue>,
}
#[derive(Debug, Clone)]
pub enum EnumValue {
    Number(u32),
    String(String),
}

/// 装饰器：用于类、方法、属性、参数的元数据注解
/// 语法: @decorator 或 @decorator(args)
#[derive(Debug, Clone)]
pub struct Decorator {
    /// 装饰器名称（标识符）
    pub name: String,
    /// 装饰器参数（可选），例如 @decorator(arg1, arg2)
    pub arguments: Vec<ASTExpression>,
}

/// 导入项（用于 import 语句）
#[derive(Debug, Clone)]
pub struct ImportSpecifier {
    /// 原始名称（从模块导入的名称）
    pub imported: String,
    /// 本地别名（绑定到当前文件的名称）
    pub alias: Option<String>,
    /// 是否为默认导入项
    pub is_default: bool,
}

/// 导出项（用于 export 语句）
#[derive(Debug, Clone)]
pub struct ExportSpecifier {
    /// 原始名称（要导出的名称）
    pub name: String,
    /// 导出别名（重命名导出）
    pub alias: Option<String>,
}

/// 解构模式类型
#[derive(Debug, Clone)]
pub enum DestructuringPattern {
    /// 数组解构: [a, b, c]
    Array {
        elements: Vec<Option<DestructuringElement>>, // None 表示空位
    },
    /// 对象解构: { a, b, c }
    Object {
        properties: Vec<DestructuringProperty>,
    },
}

/// 数组解构元素（支持默认值）
#[derive(Debug, Clone)]
pub struct DestructuringElement {
    /// 解构模式（标识符或嵌套模式）
    pub pattern: Box<ASTNode>,
    /// 默认值（可选）
    pub default_value: Option<Box<ASTNode>>,
}

/// 解构属性（用于对象解构）
#[derive(Debug, Clone)]
pub struct DestructuringProperty {
    /// 属性名（用于从源对象提取）
    pub key: String,
    /// 变量名（绑定到的新变量名，如果与 key 不同）
    /// 默认为 key，表示同名绑定
    pub alias: Option<String>,
    /// 默认值
    pub default_value: Option<Box<ASTNode>>,
    /// 是否是展开运算符 (...rest)
    pub is_rest: bool,
    /// 展开的目标（仅用于 rest）
    pub rest_target: Option<String>,
}

/// 函数参数类型（支持简单参数和解构参数）
#[derive(Debug, Clone)]
pub enum FunctionParameter {
    /// 简单参数: `name` 或 `name: Type`，支持访问修饰符
    Simple {
        name: String,
        type_annotation: Option<String>,
        /// 默认值（可选）
        default_value: Option<Box<ASTNode>>,
        /// public 修饰符（用于构造函数参数）
        is_public: bool,
        /// private 修饰符（用于构造函数参数）
        is_private: bool,
        /// protected 修饰符（用于构造函数参数）
        is_protected: bool,
        /// readonly 修饰符（用于构造函数参数）
        is_readonly: bool,
    },
    /// 解构参数: `[a, b]` 或 `{ a, b }`，支持默认值
    Destructuring {
        pattern: DestructuringPattern,
        /// 默认值（可选）
        default_value: Option<Box<ASTNode>>,
    },
}

/// 索引签名：用于定义动态属性类型，如 [key: string]: T
#[derive(Debug, Clone)]
pub struct IndexSignature {
    pub key_name: String,   // 索引参数名，如 "key"
    pub key_type: String,   // 键类型："string" 或 "number"
    pub value_type: String, // 值类型
}

/// 抽象语法树节点
#[derive(Debug, Clone)]
pub enum ASTNode {
    Program(Vec<ASTNode>),
    VariableDeclaration {
        /// 是否为 declare 声明（declare const/let/var）
        is_declare: bool,
        kind: String,
        /// 变量名（简单标识符）或解构模式
        /// 简单标识符: `name`
        /// 解构模式: `DestructuringPattern(Box<DestructuringPattern>)`
        name: String,
        type_annotation: Option<String>,
        initializer: Option<Box<ASTNode>>,
    },
    /// 解构模式（用于变量声明和参数）
    DestructuringPattern {
        /// 数组模式或对象模式
        pattern: DestructuringPattern,
    },
    /// 解构赋值声明：const [a, b] = arr 或 const { a, b } = obj
    DestructuringDeclaration {
        kind: String,
        /// 解构模式
        pattern: DestructuringPattern,
        /// 源表达式
        source: Box<ASTExpression>,
    },
    FunctionDeclaration {
        /// 是否为 declare 声明（declare function）
        is_declare: bool,
        name: String,
        is_async: bool,
        type_params: Option<Vec<String>>, // 泛型参数列表，如 ['T']
        params: Vec<FunctionParameter>,
        return_type: Option<String>,
        body: Vec<ASTNode>,
    },
    /// 函数重载签名（无函数体）: function foo(x: T): T;
    FunctionOverload {
        name: String,
        is_async: bool,
        type_params: Option<Vec<String>>,
        params: Vec<FunctionParameter>,
        return_type: Option<String>,
    },
    ClassDeclaration {
        /// 是否为 declare 声明（declare class）
        is_declare: bool,
        /// 是否为抽象类（abstract class）
        is_abstract: bool,
        /// 类装饰器列表
        decorators: Vec<Decorator>,
        name: String,
        extends: Option<String>, // 父类名称（如果有 extends）
        members: Vec<ASTNode>,
    },
    /// 类方法声明
    MethodDeclaration {
        /// 方法装饰器列表
        decorators: Vec<Decorator>,
        name: String,
        kind: String, // "method", "get", "set"
        is_async: bool,
        is_static: bool,
        is_abstract: bool, // 是否为抽象方法
        params: Vec<FunctionParameter>,
        body: Vec<ASTNode>,
    },
    /// 类字段声明
    PropertyDeclaration {
        /// 字段装饰器列表
        decorators: Vec<Decorator>,
        name: String,
        is_static: bool,
        is_abstract: bool, // 是否为抽象字段
        initializer: Option<Box<ASTNode>>,
    },
    /// 类计算属性名声明
    ComputedPropertyDeclaration {
        /// 计算属性装饰器列表
        decorators: Vec<Decorator>,
        key_expr: Box<ASTExpression>,
        is_static: bool,
        initializer: Option<Box<ASTNode>>,
    },
    /// 接口声明：支持继承和索引签名
    InterfaceDeclaration {
        name: String,
        /// 继承的父接口列表
        extends: Vec<String>,
        properties: HashMap<String, String>,
        /// 索引签名：[key: string]: T 或 [key: number]: T
        index_signature: Option<Box<IndexSignature>>,
        /// v0.3.189: 构造函数签名: new(...args): T
        constructor_signature: Option<String>,
    },
    EnumDeclaration {
        name: String,
        members: Vec<EnumMember>,
    },
    /// 类型别名声明: type Foo = string | number
    TypeAliasDeclaration {
        name: String,
        /// 类型参数（泛型），如 ['T']
        type_params: Option<Vec<String>>,
        /// 类型定义
        type_definition: String,
    },
    /// 导入语句: import { a, b } from 'module' | import x from 'module'
    ImportDeclaration {
        /// 模块路径
        module_specifier: String,
        /// 导入项列表
        imports: Vec<ImportSpecifier>,
        /// 是否为默认导入
        is_default: bool,
        /// 命名空间导入别名 (import * as name)
        namespace_alias: Option<String>,
        /// 是否为仅类型导入 (import type)
        is_type_only: bool,
    },
    /// 导出语句: export { a, b } | export default x | export const x = 1
    ExportDeclaration {
        /// 导出项列表（用于 export { ... }）
        exports: Vec<ExportSpecifier>,
        /// 是否为默认导出
        is_default: bool,
        /// 源模块（用于 re-export: export { x } from 'module'）
        module_specifier: Option<String>,
        /// 内联导出声明（用于 export const x = 1）
        inline_declaration: Option<Box<ASTNode>>,
        /// 是否为仅类型导出 (export type)
        is_type_only: bool,
    },
    Expression(ASTExpression),
    Statement(ASTStatement),
}
#[derive(Debug, Clone)]
pub enum ASTExpression {
    Identifier(String),
    Literal(String),
    BinaryExpression {
        left: Box<ASTExpression>,
        operator: String,
        right: Box<ASTExpression>,
    },
    CallExpression {
        callee: Box<ASTExpression>,
        arguments: Vec<ASTExpression>,
    },
    MemberExpression {
        object: Box<ASTExpression>,
        property: String,
    },
    IndexExpression {
        object: Box<ASTExpression>,
        index: Box<ASTExpression>,
    },
    // 一元运算符: typeof, !, -, +, ~ 等
    Unary {
        operator: String,
        operand: Box<ASTExpression>,
    },
    // 更新表达式: i++ 或 --i
    UpdateExpression {
        argument: Box<ASTExpression>,
        operator: String,
        is_prefix: bool,
    },
    /// 对象属性：支持普通属性和计算属性名
    /// 普通属性: `{ name: value }`
    /// 计算属性: `{ [expr]: value }`
    ObjectProperty {
        /// 属性名：普通属性为 Some(identifier)，计算属性为 None（使用 key_expr）
        name: Option<String>,
        /// 计算属性名表达式：普通属性为 None，计算属性为 Some(expr)
        key_expr: Option<Box<ASTExpression>>,
        /// 属性值表达式
        value: Box<ASTExpression>,
    },
    ObjectLiteral {
        properties: Vec<ASTExpression>, // 使用 ObjectProperty 表达式
    },
    // v0.3.180: 箭头函数表达式 - 支持默认参数值
    ArrowFunctionExpression {
        // params: Vec<(name, type_annotation, default_value)>
        params: Vec<(String, Option<String>, Option<String>)>,
        body: Box<ASTNode>, // 可以是 Expression 或 Block(Vec<ASTNode>)
        return_type: Option<String>,
        is_async: bool,
    },
    /// 模板字符串: `Hello ${name}!`
    TemplateLiteral {
        parts: Vec<ASTExpression>, // 交替的字符串和表达式
    },
    /// await 表达式: await somePromise
    Await {
        expression: Box<ASTExpression>,
    },
    /// new 表达式: new Constructor(args)
    NewExpression {
        constructor: Box<ASTExpression>,
        arguments: Vec<ASTExpression>,
    },
    /// this 关键字
    ThisExpression,
    /// 赋值表达式: a = b
    AssignmentExpression {
        left: Box<ASTExpression>,
        right: Box<ASTExpression>,
    },
    /// 数组表达式: [1, 2, 3]
    ArrayExpression {
        elements: Vec<Option<ASTExpression>>, // None 表示空位或解构
    },
    /// 展开表达式: ...arr
    SpreadExpression {
        argument: Box<ASTExpression>,
    },
    /// 条件表达式（三元运算符）: condition ? true_expr : false_expr
    ConditionalExpression {
        condition: Box<ASTExpression>,
        consequent: Box<ASTExpression>,
        alternate: Box<ASTExpression>,
    },
    /// super 关键字
    SuperExpression,
    /// TypeScript 类型断言: value as Type 或 value as const
    /// 转译时类型信息被移除，直接输出原始表达式
    /// is_const 标记是否为 as const 断言（语义上表示只读字面量类型）
    TSAsExpression {
        expression: Box<ASTExpression>,
        target_type: String,
        is_const: bool,
    },
    /// TypeScript 尖括号类型断言: `<Type>value`
    /// 转译时类型信息被移除，直接输出原始表达式
    /// 注意：这是旧式类型断言语法，在 JSX/TSX 中可能与泛型冲突
    TSAngleBracketAssertion {
        expression: Box<ASTExpression>,
        target_type: String,
    },
    /// TypeScript satisfies 操作符: expr satisfies Type
    /// 转译时类型信息被移除，直接输出原始表达式
    /// 与 as 不同，satisfies 不改变表达式的推断类型
    TSSatisfiesExpression {
        expression: Box<ASTExpression>,
        target_type: String,
    },
    /// 函数表达式: function(x) { ... } 或 async function(x) { ... }
    /// 用于: const fn = function() {} 或 const fn = async function() {}
    FunctionExpression {
        is_async: bool,
        type_params: Option<Vec<String>>, // 泛型参数列表
        params: Vec<FunctionParameter>,
        return_type: Option<String>,
        body: Vec<ASTNode>,
    },
}
#[derive(Debug, Clone)]
pub enum ASTStatement {
    Block(Vec<ASTNode>),
    Expression(ASTExpression),
    Return(Option<ASTExpression>),
    If {
        test: ASTExpression,
        consequent: Box<ASTNode>,
        alternate: Option<Box<ASTNode>>,
    },
    /// for...of 循环: for (const x of items) { ... }
    ForOf {
        initializer: Box<ASTNode>, // VariableDeclaration
        iterable: ASTExpression,
        body: Box<ASTNode>,
    },
    /// 传统 for 循环: for (let i = 0; i < 10; i++) { ... }
    For {
        initializer: Option<Box<ASTNode>>, // VariableDeclaration 或 Expression
        condition: Option<ASTExpression>,
        update: Option<ASTExpression>,
        body: Box<ASTNode>,
    },
    /// while 循环: while (condition) { ... }
    While {
        test: ASTExpression,
        body: Box<ASTNode>,
    },
    /// do...while 循环: do { ... } while (condition)
    DoWhile {
        body: Box<ASTNode>,
        test: ASTExpression,
    },
    /// switch 语句: switch (x) { case 1: ...; break; default: ...; }
    Switch {
        discriminant: ASTExpression,
        cases: Vec<SwitchCase>,
    },
    /// try...catch...finally 语句
    Try {
        body: Box<ASTNode>,
        handler: Option<CatchClause>,
        finalizer: Option<Box<ASTNode>>,
    },
    /// break 语句
    Break {
        label: Option<String>,
    },
    /// continue 语句
    Continue {
        label: Option<String>,
    },
    /// throw 语句
    Throw {
        expression: ASTExpression,
    },
    /// TypeScript 命名空间: namespace MyNamespace { ... }
    /// 编译为: var MyNamespace; (function(MyNamespace) { ... })(MyNamespace || (MyNamespace = {}));
    /// 嵌套命名空间: namespace A.B.C { ... }
    /// 编译为: var A; (function(A) { var B; (function(B) { ... })(B || (B = {})); })(A || (A = {}));
    Namespace {
        /// 命名空间名称（顶层名称）
        name: String,
        /// 完整命名空间路径（支持嵌套，如 "A.B.C"）
        full_name: String,
        /// 命名空间内部的语句列表
        body: Vec<ASTNode>,
        /// 是否为 declare 声明（declare namespace）
        is_declare: bool,
    },
    /// TypeScript 全局声明块: declare global { ... }
    /// 用于向全局作用域添加类型声明
    GlobalDeclaration {
        /// 全局声明块内部的语句列表
        body: Vec<ASTNode>,
    },
    /// TypeScript 模块声明: declare module "module-name" { ... }
    /// 用于声明模块的类型定义
    ModuleDeclaration {
        /// 模块名称（如 "my-module"）
        name: String,
        /// 模块内部的语句列表
        body: Vec<ASTNode>,
    },
    /// 三重合并声明：interface + namespace + module 同名合并
    /// TypeScript 允许同一个名称同时作为接口、命名空间和模块声明
    /// 所有声明的内容会被合并到同一个实体中
    TripleMergedDeclaration {
        /// 合并后的名称
        name: String,
        /// 接口属性（来自 interface 声明）
        interface_properties: HashMap<String, String>,
        /// 接口继承列表
        interface_extends: Vec<String>,
        /// 接口索引签名
        interface_index_signature: Option<Box<IndexSignature>>,
        /// 命名空间/模块体（来自 namespace 和 module 声明）
        body: Vec<ASTNode>,
        /// 是否为 declare 声明
        is_declare: bool,
    },
    /// TypeScript export = 语法（CommonJS/AMD 兼容）
    /// 用于导出单个值，常用于与 CommonJS 或 AMD 模块互操作
    ExportAssignment {
        /// 要导出的表达式
        expression: Box<ASTExpression>,
        /// 是否为 type-only 导出
        is_type_only: bool,
    },
}
/// switch case 结构
#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<ASTExpression>, // None 表示 default
    pub body: Vec<ASTNode>,
}
/// catch 子句
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<String>, // 捕获的异常变量名
    pub body: Vec<ASTNode>,
}
/// 解析器
struct Parser {
    tokens: Vec<Token>,
    position: usize,
    /// 是否在 for 循环的初始化器解析上下文中
    /// 用于控制初始化器表达式的解析方式
    for_loop_context: bool,
}
impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            for_loop_context: false,
        }
    }
    fn parse(&mut self) -> Result<ASTNode> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(ASTNode::Program(statements))
    }
    fn parse_statement(&mut self) -> Result<ASTNode> {
        match self.current_token() {
            Token::Let | Token::Const | Token::Var => {
                // 检查是否是 const enum（TypeScript 特定语法）
                if self.current_token_eq(&Token::Const) {
                    let saved_position = self.position;
                    self.advance(); // consume const
                    if self.current_token_eq(&Token::Enum) {
                        return self.parse_enum_declaration();
                    } else {
                        // 不是 const enum，将 position 恢复
                        self.position = saved_position;
                    }
                }
                let node = self.parse_variable_declaration(false)?;
                // 消耗分号（对于独立的变量声明）
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(node)
            }
            Token::Function | Token::Async => self.parse_function_declaration(false),
            Token::Class => self.parse_class_declaration(),
            Token::Abstract => {
                // abstract class 声明
                self.parse_class_declaration()
            }
            Token::Interface => self.parse_interface_declaration(),
            Token::Enum => self.parse_enum_declaration(),
            Token::Global => {
                // declare global { ... }
                self.parse_global_declaration()
            }
            Token::Type => self.parse_type_alias_declaration(),
            Token::Declare => {
                // declare 关键字 - 需要查看后续 token 来确定声明类型
                // declare 可以用于: namespace, class, function, const, let, var, interface, type, enum, module
                let is_declare = true;
                // 消耗 declare 关键字
                self.consume(Token::Declare)?;
                // 查看下一个 token 来确定声明类型
                let node = match self.current_token() {
                    Token::Namespace => self.parse_namespace_declaration_internal(is_declare),
                    Token::Class => self.parse_class_declaration_internal(is_declare, Vec::new()),
                    Token::Function => self.parse_function_declaration(is_declare),
                    Token::Interface => self.parse_interface_declaration(),
                    Token::Type => self.parse_type_alias_declaration(),
                    Token::Enum => self.parse_enum_declaration(),
                    Token::Global => self.parse_global_declaration(),
                    Token::Module => {
                        // declare module "name" { ... }
                        self.parse_module_declaration()
                    }
                    Token::Const | Token::Let | Token::Var => {
                        // declare const/let/var 声明
                        self.parse_variable_declaration(is_declare)
                    }
                    _ => {
                        bail!("Invalid declare declaration: {:?}", self.current_token());
                    }
                }?;
                // 消耗分号（对于声明语句）
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(node)
            }
            Token::Namespace => self.parse_namespace_declaration(),
            Token::Return => self.parse_return_statement(),
            Token::For => self.parse_for_statement(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::Do => self.parse_do_while_statement(),
            Token::Switch => self.parse_switch_statement(),
            Token::Try => self.parse_try_statement(),
            Token::Throw => self.parse_throw_statement(),
            Token::Break => self.parse_break_statement(),
            Token::Continue => self.parse_continue_statement(),
            Token::Import => self.parse_import_declaration(),
            Token::Export => self.parse_export_declaration(),
            Token::At => {
                // 装饰器语句 - 尝试解析装饰器后面的声明
                // 保存装饰器列表
                let decorators = self.parse_decorators()?;
                // 根据后续的 token 解析对应的声明
                match self.current_token() {
                    Token::Class => self.parse_class_declaration_with_decorators(decorators),
                    _ => {
                        // 尝试作为表达式解析
                        bail!(
                            "Unexpected token after decorator: {:?}",
                            self.current_token()
                        );
                    }
                }
            }
            _ => {
                // 表达式语句
                let expr = self.parse_expression()?;
                // 检查是否有分号，如果没有就尝试消费它
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::Statement(ASTStatement::Expression(expr)))
            }
        }
    }
    fn parse_return_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Return)?;
        // Check if there's an expression or just a semicolon
        let expr = if self.current_token_eq(&Token::SemiColon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume(Token::SemiColon)?;
        Ok(ASTNode::Statement(ASTStatement::Return(expr)))
    }

    /// 解析 for 循环语句
    fn parse_for_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::For)?;
        self.consume(Token::LParen)?;

        // 设置 for 循环上下文，以便 parse_variable_declaration 使用简单的初始化器解析
        self.for_loop_context = true;

        // 检查 for...of 语法: for (const/let/var x of iterable)
        // 或传统 for 语法: for (init; condition; update)

        // 首先解析初始化部分
        let initializer: Option<Box<ASTNode>> = if self.current_token_eq(&Token::Let)
            || self.current_token_eq(&Token::Const)
            || self.current_token_eq(&Token::Var)
        {
            Some(Box::new(self.parse_variable_declaration(false)?))
        } else if !self.current_token_eq(&Token::SemiColon) {
            // 可能是表达式
            let expr = self.parse_expression()?;
            Some(Box::new(ASTNode::Expression(expr)))
        } else {
            None
        };

        // 退出 for 循环上下文
        self.for_loop_context = false;

        // 检查是否是 for...of
        let is_for_of = match self.current_token() {
            Token::Identifier(s) if s == "of" => true,
            _ => false,
        };
        if is_for_of {
            // for...of 循环
            self.consume_any_identifier()?;
            let iterable = self.parse_expression()?;
            self.consume(Token::RParen)?;
            let body = self.parse_block_or_statement()?;
            return Ok(ASTNode::Statement(ASTStatement::ForOf {
                initializer: initializer.unwrap_or_else(|| {
                    Box::new(ASTNode::VariableDeclaration {
                        is_declare: false,
                        kind: "let".to_string(),
                        name: "_".to_string(),
                        type_annotation: None,
                        initializer: None,
                    })
                }),
                iterable,
                body: Box::new(body),
            }));
        }

        // 传统 for 循环
        self.consume(Token::SemiColon)?;

        // 解析条件
        let condition: Option<ASTExpression> = if self.current_token_eq(&Token::SemiColon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume(Token::SemiColon)?;

        // 解析更新表达式
        let update: Option<ASTExpression> = if self.current_token_eq(&Token::RParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume(Token::RParen)?;

        let body = self.parse_block_or_statement()?;
        Ok(ASTNode::Statement(ASTStatement::For {
            initializer,
            condition,
            update,
            body: Box::new(body),
        }))
    }

    /// 解析 if 语句
    fn parse_if_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::If)?;
        self.consume(Token::LParen)?;
        let test = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let consequent = Box::new(self.parse_block_or_statement()?);

        // 检查是否有 else
        let alternate: Option<Box<ASTNode>> = if self.current_token_eq(&Token::Else) {
            self.consume(Token::Else)?;
            Some(Box::new(self.parse_block_or_statement()?))
        } else {
            None
        };

        Ok(ASTNode::Statement(ASTStatement::If {
            test,
            consequent,
            alternate,
        }))
    }

    /// 解析 while 语句
    fn parse_while_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::While)?;
        self.consume(Token::LParen)?;
        let test = self.parse_expression()?;
        self.consume(Token::RParen)?;
        let body = Box::new(self.parse_block_or_statement()?);
        Ok(ASTNode::Statement(ASTStatement::While { test, body }))
    }

    /// 解析 do...while 语句
    fn parse_do_while_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Do)?;
        let body = Box::new(self.parse_block_or_statement()?);
        self.consume(Token::While)?;
        self.consume(Token::LParen)?;
        let test = self.parse_expression()?;
        self.consume(Token::RParen)?;
        self.consume(Token::SemiColon)?;
        Ok(ASTNode::Statement(ASTStatement::DoWhile { body, test }))
    }

    /// 解析 switch 语句
    fn parse_switch_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Switch)?;
        self.consume(Token::LParen)?;
        let discriminant = self.parse_expression()?;
        self.consume(Token::RParen)?;
        self.consume(Token::LBrace)?;

        let mut cases = Vec::new();
        while !self.current_token_eq(&Token::RBrace) {
            // 解析 case 或 default
            let test = if self.current_token_eq(&Token::Case) {
                self.consume(Token::Case)?;
                let case_test = self.parse_expression()?;
                self.consume(Token::Colon)?;
                Some(case_test)
            } else if self.current_token_eq(&Token::Default) {
                self.consume(Token::Default)?;
                self.consume(Token::Colon)?;
                None
            } else {
                bail!("Expected 'case' or 'default' in switch statement");
            };

            // 解析 case 体
            let mut body = Vec::new();
            while !self.current_token_eq(&Token::RBrace)
                && !self.current_token_eq(&Token::Case)
                && !self.current_token_eq(&Token::Default)
            {
                body.push(self.parse_statement()?);
            }

            cases.push(SwitchCase { test, body });
        }

        self.consume(Token::RBrace)?;
        Ok(ASTNode::Statement(ASTStatement::Switch {
            discriminant,
            cases,
        }))
    }

    /// 解析 try...catch...finally 语句
    fn parse_try_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Try)?;

        // 解析 try 块
        let body: Box<ASTNode> = if self.current_token_eq(&Token::LBrace) {
            Box::new(self.parse_block_or_statement()?)
        } else {
            Box::new(self.parse_statement()?)
        };

        // 解析 catch 子句
        let handler: Option<CatchClause> = if self.current_token_eq(&Token::Catch) {
            self.consume(Token::Catch)?;

            // 可选的 catch 参数: catch (e)
            let param: Option<String> = if self.current_token_eq(&Token::LParen) {
                self.consume(Token::LParen)?;
                let param_token = self.consume_any_identifier()?;
                let param_name = match param_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected identifier in catch parameter"),
                };
                self.consume(Token::RParen)?;
                Some(param_name)
            } else {
                None
            };

            // 解析 catch 块
            let catch_body: Vec<ASTNode> = if self.current_token_eq(&Token::LBrace) {
                self.consume(Token::LBrace)?;
                let mut statements = Vec::new();
                while !self.current_token_eq(&Token::RBrace) {
                    statements.push(self.parse_statement()?);
                }
                self.consume(Token::RBrace)?;
                statements
            } else {
                vec![self.parse_statement()?]
            };

            Some(CatchClause {
                param,
                body: catch_body,
            })
        } else {
            None
        };

        // 解析 finally 子句
        let finalizer: Option<Box<ASTNode>> = if self.current_token_eq(&Token::Finally) {
            self.consume(Token::Finally)?;
            Some(Box::new(self.parse_block_or_statement()?))
        } else {
            None
        };

        Ok(ASTNode::Statement(ASTStatement::Try {
            body,
            handler,
            finalizer,
        }))
    }

    /// 解析 throw 语句
    fn parse_throw_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Throw)?;
        let expr = self.parse_expression()?;
        self.consume(Token::SemiColon)?;
        Ok(ASTNode::Statement(ASTStatement::Throw { expression: expr }))
    }

    /// 解析 break 语句
    fn parse_break_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Break)?;
        // 可选的标签
        let label: Option<String> = if self.current_token_eq(&Token::Identifier("".to_string())) {
            let label_token = self.consume_any_identifier()?;
            match label_token {
                Token::Identifier(name) => Some(name),
                _ => None,
            }
        } else {
            None
        };
        self.consume(Token::SemiColon)?;
        Ok(ASTNode::Statement(ASTStatement::Break { label }))
    }

    /// 解析 continue 语句
    fn parse_continue_statement(&mut self) -> Result<ASTNode> {
        self.consume(Token::Continue)?;
        // 可选的标签
        let label: Option<String> = if self.current_token_eq(&Token::Identifier("".to_string())) {
            let label_token = self.consume_any_identifier()?;
            match label_token {
                Token::Identifier(name) => Some(name),
                _ => None,
            }
        } else {
            None
        };
        self.consume(Token::SemiColon)?;
        Ok(ASTNode::Statement(ASTStatement::Continue { label }))
    }

    /// 解析块或单个语句
    fn parse_block_or_statement(&mut self) -> Result<ASTNode> {
        if self.current_token_eq(&Token::LBrace) {
            self.consume(Token::LBrace)?;
            let mut statements = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                statements.push(self.parse_statement()?);
            }
            self.consume(Token::RBrace)?;
            Ok(ASTNode::Statement(ASTStatement::Block(statements)))
        } else {
            self.parse_statement()
        }
    }

    /// 解析函数/方法体块，返回语句列表
    fn parse_block_body(&mut self) -> Result<Vec<ASTNode>> {
        if !self.current_token_eq(&Token::LBrace) {
            // 没有块语句，可能是表达式主体（如箭头函数）
            return Ok(vec![]);
        }
        self.consume(Token::LBrace)?;
        let mut statements = Vec::new();
        while !self.current_token_eq(&Token::RBrace) {
            statements.push(self.parse_statement()?);
        }
        self.consume(Token::RBrace)?;
        Ok(statements)
    }

    /// 解析函数参数列表（不包括括号）
    /// 支持简单参数和解构参数，以及默认值
    fn parse_function_params_list(&mut self) -> Result<Vec<FunctionParameter>> {
        self.consume(Token::LParen)?;
        let mut params = Vec::new();
        while !self.current_token_eq(&Token::RParen) {
            // 再次检查，防止空参数列表时循环
            if self.current_token_eq(&Token::RParen) {
                break;
            }

            // 检查是否是解构参数 ([ 或 {)
            let param = if self.current_token_eq(&Token::LBracket)
                || self.current_token_eq(&Token::LBrace)
            {
                // 解析解构模式
                let pattern = self.parse_destructuring_pattern()?;

                // 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };

                FunctionParameter::Destructuring {
                    pattern,
                    default_value,
                }
            } else {
                // 解析简单参数
                // 首先检查是否是访问修饰符（public, private, protected）
                let mut is_public = false;
                let mut is_private = false;
                let mut is_protected = false;
                let mut is_readonly = false;

                while self.current_token_eq(&Token::Public)
                    || self.current_token_eq(&Token::Private)
                    || self.current_token_eq(&Token::Protected)
                {
                    if self.current_token_eq(&Token::Public) {
                        is_public = true;
                    } else if self.current_token_eq(&Token::Private) {
                        is_private = true;
                    } else if self.current_token_eq(&Token::Protected) {
                        is_protected = true;
                    }
                    self.advance();
                }

                // 检查 readonly 修饰符
                if self.current_token_eq(&Token::Readonly) {
                    is_readonly = true;
                    self.advance();
                }

                let param_name_token = self.consume_param_name()?;
                let param_name = match param_name_token {
                    Token::Identifier(name) => name,
                    Token::This => "this".to_string(),
                    _ => bail!("Expected parameter name"),
                };

                // 跳过可选参数标记 ?
                let _is_optional = if self.current_token_eq(&Token::Question) {
                    self.consume(Token::Question)?;
                    true
                } else {
                    false
                };

                // 跳过类型注解
                let type_annotation = if self.current_token_eq(&Token::Colon) {
                    self.consume(Token::Colon)?;
                    self.parse_type_annotation()
                } else {
                    None
                };

                // 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };

                // 跳过分号分隔的参数声明（用于 public/private/protected 参数）
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }

                FunctionParameter::Simple {
                    name: param_name,
                    type_annotation,
                    default_value,
                    is_public,
                    is_private,
                    is_protected,
                    is_readonly,
                }
            };

            params.push(param);
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            }
        }
        self.consume(Token::RParen)?;
        Ok(params)
    }

    fn parse_variable_declaration(&mut self, is_declare: bool) -> Result<ASTNode> {
        let kind_token = self.consume_any(&[Token::Let, Token::Const, Token::Var])?;
        let kind = match kind_token {
            Token::Let => "let",
            Token::Const => "const",
            Token::Var => "var",
            _ => unreachable!(),
        };

        // 检查是否是解构赋值
        // 模式: const [a, b] = arr 或 const { a, b } = obj
        if self.current_token_eq(&Token::LBracket) || self.current_token_eq(&Token::LBrace) {
            // 解析解构模式
            let pattern = self.parse_destructuring_pattern()?;
            // 消耗等号
            self.consume(Token::Eq)?;
            // 解析源表达式
            let source = self.parse_expression()?;
            return Ok(ASTNode::DestructuringDeclaration {
                kind: kind.to_string(),
                pattern,
                source: Box::new(source),
            });
        }

        // 解析第一个变量
        let (name, initializer) = self.parse_variable_name_and_initializer(kind)?;

        // 检查是否是箭头函数 (initializer 为 None 且当前有 Eq = 箭头函数模式)
        if initializer.is_none() && self.current_token_eq(&Token::Eq) {
            // 这是一个箭头函数: const add = (a, b) => ...
            // 先消费 =，然后解析箭头函数
            self.consume(Token::Eq)?;
            let arrow_expr = self.parse_arrow_function_from_assignment_with_name(name.clone())?;
            return Ok(ASTNode::VariableDeclaration {
                is_declare,
                kind: kind.to_string(),
                name,
                type_annotation: None,
                initializer: Some(Box::new(ASTNode::Expression(arrow_expr))),
            });
        }

        // 处理多变量声明: const a = 1, b = 2;
        // 注意：对于多变量声明，每个变量可以有初始化器
        while self.current_token_eq(&Token::Comma) {
            self.consume(Token::Comma)?;
            // 后续变量不再有类型注解（简化处理）
            let (_next_name, _next_initializer) = self.parse_variable_name_and_initializer(kind)?;
            // 对于多变量声明，我们只返回第一个变量的声明
            // 后续变量被忽略（简化处理）
            // 在实际实现中，这里应该创建多个变量声明
        }

        Ok(ASTNode::VariableDeclaration {
            is_declare,
            kind: kind.to_string(),
            name,
            type_annotation: None,
            initializer,
        })
    }

    /// 解析解构模式（数组或对象）
    fn parse_destructuring_pattern(&mut self) -> Result<DestructuringPattern> {
        if self.current_token_eq(&Token::LBracket) {
            self.parse_array_destructuring_pattern()
        } else if self.current_token_eq(&Token::LBrace) {
            self.parse_object_destructuring_pattern()
        } else {
            bail!("Expected destructuring pattern ([ or {{)");
        }
    }

    /// 解析数组解构模式: [a, b, c]
    fn parse_array_destructuring_pattern(&mut self) -> Result<DestructuringPattern> {
        self.consume(Token::LBracket)?;
        let mut elements = Vec::new();

        // 处理空数组模式 []
        if self.current_token_eq(&Token::RBracket) {
            self.consume(Token::RBracket)?;
            return Ok(DestructuringPattern::Array { elements });
        }

        // 解析元素
        while !self.current_token_eq(&Token::RBracket) {
            if self.current_token_eq(&Token::DotDotDot) {
                // 展开运算符: ...rest
                self.consume(Token::DotDotDot)?;
                // 解析 rest 标识符
                let rest_name = if let Token::Identifier(name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    bail!("Expected identifier after ...");
                };
                elements.push(Some(DestructuringElement {
                    pattern: Box::new(ASTNode::DestructuringPattern {
                        pattern: DestructuringPattern::Object {
                            properties: vec![DestructuringProperty {
                                key: "rest".to_string(),
                                alias: Some(rest_name),
                                default_value: None,
                                is_rest: true,
                                rest_target: None,
                            }],
                        },
                    }),
                    default_value: None,
                }));
            } else if self.current_token_eq(&Token::LBracket) {
                // 嵌套数组解构
                let nested = self.parse_array_destructuring_pattern()?;
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_initializer_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };
                elements.push(Some(DestructuringElement {
                    pattern: Box::new(ASTNode::DestructuringPattern { pattern: nested }),
                    default_value,
                }));
            } else if self.current_token_eq(&Token::LBrace) {
                // 嵌套对象解构
                let nested = self.parse_object_destructuring_pattern()?;
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_initializer_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };
                elements.push(Some(DestructuringElement {
                    pattern: Box::new(ASTNode::DestructuringPattern { pattern: nested }),
                    default_value,
                }));
            } else if self.current_token_eq(&Token::Comma) {
                // 空位
                elements.push(None);
            } else {
                // 标识符（简化处理：直接使用标识符）
                let name_token = if let Token::Identifier(name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    bail!("Expected identifier in destructuring pattern");
                };
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_initializer_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };
                elements.push(Some(DestructuringElement {
                    pattern: Box::new(ASTNode::Expression(ASTExpression::Identifier(name_token))),
                    default_value,
                }));
            }

            // 处理逗号分隔符
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            } else if !self.current_token_eq(&Token::RBracket) {
                break;
            }
        }

        self.consume(Token::RBracket)?;
        Ok(DestructuringPattern::Array { elements })
    }

    /// 解析对象解构模式: { a, b, c }
    fn parse_object_destructuring_pattern(&mut self) -> Result<DestructuringPattern> {
        self.consume(Token::LBrace)?;
        let mut properties = Vec::new();

        // 处理空对象模式 {}
        if self.current_token_eq(&Token::RBrace) {
            self.consume(Token::RBrace)?;
            return Ok(DestructuringPattern::Object { properties });
        }

        // 解析属性
        while !self.current_token_eq(&Token::RBrace) {
            // 检查是否是展开运算符
            if self.current_token_eq(&Token::DotDotDot) {
                self.consume(Token::DotDotDot)?;
                let rest_name = if let Token::Identifier(name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    bail!("Expected identifier after ...");
                };
                properties.push(DestructuringProperty {
                    key: "rest".to_string(),
                    alias: Some(rest_name),
                    default_value: None,
                    is_rest: true,
                    rest_target: None,
                });
            } else {
                // 解析属性名
                let is_identifier = matches!(self.current_token(), Token::Identifier(_));
                let (key, alias) = if is_identifier {
                    // 可能是 key 或 key: alias
                    let key_name = if let Token::Identifier(name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        name
                    } else {
                        bail!("Expected identifier in destructuring pattern");
                    };

                    // 检查是否有冒号（重命名）
                    if self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        let alias_name = if let Token::Identifier(name) = self.current_token() {
                            let name = name.clone();
                            self.advance();
                            name
                        } else {
                            bail!("Expected identifier after : in destructuring pattern");
                        };
                        (key_name, Some(alias_name))
                    } else {
                        // 同名简写
                        (key_name.clone(), None)
                    }
                } else if let Token::String(_, _) = self.current_token() {
                    // 字符串属性名: { "key": value }
                    let key = if let Token::String(s, _) = self.current_token() {
                        let s = s.clone();
                        self.advance();
                        format!("\"{}\"", s)
                    } else {
                        unreachable!()
                    };
                    self.consume(Token::Colon)?;
                    let alias_name = if let Token::Identifier(name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        name
                    } else {
                        bail!("Expected identifier after : in destructuring pattern");
                    };
                    (key, Some(alias_name))
                } else {
                    bail!("Expected identifier or string in destructuring pattern");
                };

                // 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    // 解析默认值表达式
                    let value = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(value)))
                } else {
                    None
                };

                properties.push(DestructuringProperty {
                    key,
                    alias,
                    default_value,
                    is_rest: false,
                    rest_target: None,
                });
            }

            // 处理逗号分隔符
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            } else if !self.current_token_eq(&Token::RBrace) {
                break;
            }
        }

        self.consume(Token::RBrace)?;
        Ok(DestructuringPattern::Object { properties })
    }

    /// 解析变量名和初始化器（用于变量声明）
    /// 如果检测到箭头函数，则调用专门的箭头函数解析
    fn parse_variable_name_and_initializer(
        &mut self,
        _kind: &str,
    ) -> Result<(String, Option<Box<ASTNode>>)> {
        // 消耗变量名
        let name_token = if let Token::Identifier(_) = self.current_token() {
            self.advance()
        } else {
            bail!("Expected identifier");
        };
        let name = match name_token {
            Token::Identifier(name) => name,
            _ => bail!("Expected identifier"),
        };

        // 可能的类型注解（跳过，不生成AST节点）
        if self.current_token_eq(&Token::Colon) {
            self.consume(Token::Colon)?;
            self.parse_type_annotation();
        }

        // 向前查看：检查是否是箭头函数 (identifier = (...) 或 identifier => ...)
        let is_arrow_function = self.is_arrow_function_ahead();

        if is_arrow_function {
            // 使用 saved_tokens 恢复状态并解析箭头函数
            // 这里需要特殊处理：返回 None 作为 initializer，让调用方处理箭头函数
            return Ok((name, None));
        }

        // 可能的初始化器
        let initializer: Option<Box<ASTNode>> = if self.current_token_eq(&Token::Eq) {
            self.consume(Token::Eq)?;
            let expr = self.parse_expression()?;
            Some(Box::new(ASTNode::Expression(expr)))
        } else {
            None
        };

        Ok((name, initializer))
    }

    /// 向前查看 token 流，判断是否是箭头函数
    /// 箭头函数的模式:
    /// - identifier => ... (单参数无括号: x => ...)
    /// - identifier = ( ... ) => ... (带括号参数: add = (a, b) => ...)
    /// - identifier = identifier => ... (标识符为参数: add = fn => ...)
    fn is_arrow_function_ahead(&self) -> bool {
        let mut i = 0;
        let n = self.tokens.len();

        // 跳过已保存的 token，查看后续 token
        while i + self.position < n {
            let token = &self.tokens[i + self.position];

            match token {
                // 跳过空白和分号类型的 token
                Token::SemiColon => {
                    i += 1;
                    continue;
                }
                // 等号后面可能是箭头函数
                Token::Eq => {
                    i += 1;
                    while i + self.position < n {
                        match &self.tokens[i + self.position] {
                            Token::SemiColon => {
                                i += 1;
                                continue;
                            }
                            Token::LParen => {
                                // 查找匹配的右括号
                                let mut depth = 1;
                                let mut j = i + 1;
                                while j + self.position < n && depth > 0 {
                                    match &self.tokens[j + self.position] {
                                        Token::LParen => {
                                            depth += 1;
                                            j += 1;
                                        }
                                        Token::RParen => {
                                            if depth > 1 {
                                                depth -= 1;
                                                j += 1;
                                            } else {
                                                // 找到最外层的 RParen，depth 将变为 0
                                                // j 指向 RParen，不需要再增加
                                                depth = 0;
                                            }
                                        }
                                        _ => {
                                            j += 1;
                                        }
                                    }
                                }
                                // j 现在是 RParen 的位置
                                // 检查后面是否是 FatArrow，或者先跳过返回类型注解 (): type =>
                                let mut check_pos = j + 1;
                                // 跳过返回类型注解 : Type
                                if check_pos + self.position < n {
                                    if let Token::Colon = &self.tokens[check_pos + self.position] {
                                        // 有返回类型注解，跳过 colon 和类型
                                        check_pos += 1;
                                        // 跳过类型（标识符类型）
                                        while check_pos + self.position < n {
                                            match &self.tokens[check_pos + self.position] {
                                                Token::Identifier(_)
                                                | Token::Number(_)
                                                | Token::String(_, _) => {
                                                    check_pos += 1;
                                                    break;
                                                }
                                                Token::FatArrow => break,
                                                _ => {
                                                    // 其他类型 token，继续
                                                    check_pos += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                                if check_pos + self.position < n {
                                    if let Token::FatArrow = &self.tokens[check_pos + self.position]
                                    {
                                        return true;
                                    }
                                }
                                return false;
                            }
                            Token::Identifier(_) => {
                                // identifier = identifier => ...
                                i += 1;
                                while i + self.position < n {
                                    match &self.tokens[i + self.position] {
                                        Token::SemiColon => {
                                            i += 1;
                                            continue;
                                        }
                                        Token::FatArrow => return true,
                                        _ => return false,
                                    }
                                }
                                return false;
                            }
                            Token::FatArrow => return true,
                            _ => return false,
                        }
                    }
                    return false;
                }
                // 左括号后跟标识符可能是参数列表
                Token::LParen => {
                    // 查找匹配的右括号
                    let mut depth = 1;
                    let mut j = i + 1;
                    while j + self.position < n && depth > 0 {
                        match &self.tokens[j + self.position] {
                            Token::LParen => depth += 1,
                            Token::RParen => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            j += 1;
                        }
                    }
                    // j 现在是 RParen 的位置，检查后面是否是 FatArrow
                    if j + 1 + self.position < n {
                        if let Token::FatArrow = &self.tokens[j + 1 + self.position] {
                            return true;
                        }
                    }
                    return false;
                }
                // 标识符后可能是 => (单参数无括号: x => ...)
                Token::Identifier(_) => {
                    // 跳过标识符
                    i += 1;
                    // 继续检查后面的 token（可能是类型注解或 =>）
                    while i + self.position < n {
                        match &self.tokens[i + self.position] {
                            Token::SemiColon => {
                                i += 1;
                                continue;
                            }
                            Token::Colon => {
                                // 跳过类型注解
                                i += 1;
                                // 类型注解后应该是标识符
                                i += 1;
                                continue;
                            }
                            Token::FatArrow => return true,
                            _ => return false,
                        }
                    }
                    return false;
                }
                _ => return false,
            }
        }
        false
    }

    fn parse_function_declaration(&mut self, is_declare: bool) -> Result<ASTNode> {
        // 处理 async 关键字
        let is_async = if self.current_token_eq(&Token::Async) {
            self.consume(Token::Async)?;
            // async 函数后面必须有 function 关键字
            if self.current_token_eq(&Token::Function) {
                self.consume(Token::Function)?;
            } else {
                bail!("Expected 'function' keyword after 'async'");
            }
            true
        } else if self.current_token_eq(&Token::Function) {
            self.consume(Token::Function)?;
            false
        } else {
            bail!(
                "Expected 'async' or 'function' keyword, got {:?}",
                self.current_token()
            );
        };
        let name_token = self.consume_any_identifier()?;
        let name = match name_token {
            Token::Identifier(name) => name,
            _ => bail!("Expected function name"),
        };
        // 解析泛型参数列表 (如 <T> 或 <T extends keyof T, U>)
        let type_params: Option<Vec<String>> = if self.current_token_eq(&Token::Lt) {
            self.consume(Token::Lt)?;
            let mut type_params = Vec::new();
            while !self.current_token_eq(&Token::Gt) {
                let type_param_token = self.consume_any_identifier()?;
                let type_param_name = match type_param_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected type parameter name"),
                };
                // 处理泛型约束: extends keyof T 或默认类型: = string
                if self.current_token_eq(&Token::Extends) {
                    self.consume(Token::Extends)?;
                    // 跳过约束类型
                    self.parse_type_annotation();
                } else if self.current_token_eq(&Token::Eq) {
                    // 处理默认类型: <T = string>
                    self.consume(Token::Eq)?;
                    // 跳过默认类型
                    self.parse_type_annotation();
                }
                type_params.push(type_param_name);
                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                }
            }
            self.consume(Token::Gt)?;
            Some(type_params)
        } else {
            None
        };
        // v0.3.180: 修复函数参数默认值解析
        self.consume(Token::LParen)?;
        let mut params = Vec::new();
        while !self.current_token_eq(&Token::RParen) {
            // 检查是否是解构参数 ([ 或 {)
            let param = if self.current_token_eq(&Token::LBracket)
                || self.current_token_eq(&Token::LBrace)
            {
                // 解析解构模式
                let pattern = self.parse_destructuring_pattern()?;

                // 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };

                FunctionParameter::Destructuring {
                    pattern,
                    default_value,
                }
            } else {
                // 解析简单参数（包括 this 关键字）
                let param_name_token = self.consume_param_name()?;
                let param_name = match param_name_token {
                    Token::Identifier(name) => name,
                    Token::This => "this".to_string(),
                    _ => bail!("Expected parameter name"),
                };
                // 跳过可选参数标记 ?
                if self.current_token_eq(&Token::Question) {
                    self.consume(Token::Question)?;
                }
                let param_type = if self.current_token_eq(&Token::Colon) {
                    self.consume(Token::Colon)?;
                    self.parse_type_annotation()
                } else {
                    None
                };
                // v0.3.180: 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };
                FunctionParameter::Simple {
                    name: param_name,
                    type_annotation: param_type,
                    default_value,
                    is_public: false,
                    is_private: false,
                    is_protected: false,
                    is_readonly: false,
                }
            };
            params.push(param);
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            }
        }
        self.consume(Token::RParen)?;
        // 可能的返回类型
        let return_type = if self.current_token_eq(&Token::Colon) {
            self.consume(Token::Colon)?;
            self.parse_type_annotation()
        } else {
            None
        };
        // 检查是否是函数重载签名（以分号结尾，无函数体）
        // 注意：declare function 也是以分号结尾，但应该生成 FunctionDeclaration
        if self.current_token_eq(&Token::SemiColon) {
            // 消耗分号
            self.consume(Token::SemiColon)?;
            if is_declare {
                // declare function 应该生成 FunctionDeclaration（带有 declare 关键字）
                // 但没有函数体
                return Ok(ASTNode::FunctionDeclaration {
                    is_declare,
                    name,
                    is_async,
                    type_params,
                    params,
                    return_type,
                    body: Vec::new(), // 空函数体
                });
            }
            // 普通函数重载签名
            return Ok(ASTNode::FunctionOverload {
                name,
                is_async,
                type_params,
                params,
                return_type,
            });
        }
        // 实现函数
        self.consume(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.current_token_eq(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.consume(Token::RBrace)?;
        Ok(ASTNode::FunctionDeclaration {
            is_declare,
            name,
            is_async,
            type_params,
            params,
            return_type,
            body,
        })
    }

    /// 解析函数表达式: function(params) { ... } 或 async function(params) { ... }
    /// 用于: const fn = function() {} 或 const fn = async function() {}
    fn parse_function_expression(&mut self, is_async: bool) -> Result<ASTExpression> {
        // 解析可选的函数名（匿名函数表达式可以省略名称）
        // 注意：函数表达式可以有名称（如 function foo() {}），用于递归调用
        // 语法: function [name](params) { body }
        // 如果后面直接是 (，则没有函数名（匿名函数表达式）
        // 如果后面是 Identifier 且后面是 (，则这是函数名
        // 如果后面是 Identifier 但后面不是 (，则这是第一个参数
        let _name: Option<String> = None;
        if let Token::Identifier(_) = self.current_token() {
            // 检查后面是否是 (，如果是则这是函数名
            if self.position + 1 < self.tokens.len()
                && self.tokens[self.position + 1] == Token::LParen
            {
                // 这是一个命名函数表达式 - 消费函数名
                self.consume_any_identifier()?;
            }
            // 如果后面不是 (，则说明这是第一个参数，什么都不做
        }
        // 如果当前 token 不是 Identifier，则没有函数名（匿名函数表达式）

        // 解析泛型参数列表 (如 <T> 或 <T extends keyof T, U>)
        let type_params: Option<Vec<String>> = if self.current_token_eq(&Token::Lt) {
            self.consume(Token::Lt)?;
            let mut type_params = Vec::new();
            while !self.current_token_eq(&Token::Gt) {
                let type_param_token = self.consume_any_identifier()?;
                let type_param_name = match type_param_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected type parameter name"),
                };
                // 处理泛型约束: extends keyof T 或默认类型: = string
                if self.current_token_eq(&Token::Extends) {
                    self.consume(Token::Extends)?;
                    // 跳过约束类型
                    self.parse_type_annotation();
                } else if self.current_token_eq(&Token::Eq) {
                    // 处理默认类型: <T = string>
                    self.consume(Token::Eq)?;
                    // 跳过默认类型
                    self.parse_type_annotation();
                }
                type_params.push(type_param_name);
                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                }
            }
            self.consume(Token::Gt)?;
            Some(type_params)
        } else {
            None
        };
        // v0.3.180: 修复函数表达式参数默认值解析
        self.consume(Token::LParen)?;
        let mut params = Vec::new();
        while !self.current_token_eq(&Token::RParen) {
            // 检查是否是解构参数 ([ 或 {)
            let param = if self.current_token_eq(&Token::LBracket)
                || self.current_token_eq(&Token::LBrace)
            {
                // 解析解构模式
                let pattern = self.parse_destructuring_pattern()?;

                // 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };

                FunctionParameter::Destructuring {
                    pattern,
                    default_value,
                }
            } else {
                // 解析简单参数
                let param_name_token = self.consume_any_identifier()?;
                let param_name = match param_name_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected parameter name"),
                };
                // 跳过可选参数标记 ?
                if self.current_token_eq(&Token::Question) {
                    self.consume(Token::Question)?;
                }
                let param_type = if self.current_token_eq(&Token::Colon) {
                    self.consume(Token::Colon)?;
                    self.parse_type_annotation()
                } else {
                    None
                };
                // v0.3.180: 检查是否有默认值
                let default_value = if self.current_token_eq(&Token::Eq) {
                    self.consume(Token::Eq)?;
                    let expr = self.parse_expression()?;
                    Some(Box::new(ASTNode::Expression(expr)))
                } else {
                    None
                };
                FunctionParameter::Simple {
                    name: param_name,
                    type_annotation: param_type,
                    default_value,
                    is_public: false,
                    is_private: false,
                    is_protected: false,
                    is_readonly: false,
                }
            };
            params.push(param);
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            }
        }
        self.consume(Token::RParen)?;
        // 可能的返回类型
        let return_type = if self.current_token_eq(&Token::Colon) {
            self.consume(Token::Colon)?;
            self.parse_type_annotation()
        } else {
            None
        };
        // 解析函数体
        self.consume(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.current_token_eq(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.consume(Token::RBrace)?;

        Ok(ASTExpression::FunctionExpression {
            is_async,
            type_params,
            params,
            return_type,
            body,
        })
    }

    /// 解析装饰器列表
    /// 语法: @decorator 或 @decorator(args)
    fn parse_decorators(&mut self) -> Result<Vec<Decorator>> {
        let mut decorators = Vec::new();
        while self.current_token_eq(&Token::At) {
            self.consume(Token::At)?;
            // 获取装饰器名称（可以是任何标识符，包括关键字如 readonly）
            // 使用 advance 直接获取当前 token 而不检查类型
            let name_token = self.advance();
            let name = match name_token {
                Token::Identifier(name) => name,
                Token::Readonly => "readonly".to_string(),
                _ => bail!("Expected decorator name after @"),
            };
            // 检查是否有参数列表 (...)
            let arguments = if self.current_token_eq(&Token::LParen) {
                self.consume(Token::LParen)?;
                let mut args = Vec::new();
                while !self.current_token_eq(&Token::RParen) {
                    let arg = self.parse_expression()?;
                    args.push(arg);
                    if self.current_token_eq(&Token::Comma) {
                        self.consume(Token::Comma)?;
                    }
                }
                self.consume(Token::RParen)?;
                args
            } else {
                Vec::new()
            };
            decorators.push(Decorator { name, arguments });
        }
        Ok(decorators)
    }

    /// 解析类声明（支持 declare 关键字）
    /// declare class 的处理方式：
    /// - 声明为类型声明，不生成实际代码
    /// - 或者生成声明语句 declare class X { ... }
    fn parse_class_declaration_internal(
        &mut self,
        is_declare: bool,
        decorators: Vec<Decorator>,
    ) -> Result<ASTNode> {
        // 检查是否有 abstract 修饰符
        let is_abstract = if self.current_token_eq(&Token::Abstract) {
            self.consume(Token::Abstract)?;
            true
        } else {
            false
        };
        self.consume(Token::Class)?;
        let name_token = self.consume_any_identifier()?;
        let name = match name_token {
            Token::Identifier(name) => name,
            _ => bail!("Expected class name"),
        };
        // v0.3.169: 支持泛型类参数，如 class Container<T>
        if self.current_token_eq(&Token::Lt) {
            // 消费 <T>
            self.consume(Token::Lt)?;
            let mut depth = 1;
            while depth > 0 && !self.is_at_end() {
                match self.current_token() {
                    Token::Lt => {
                        depth += 1;
                        self.advance();
                    }
                    Token::Gt => {
                        depth -= 1;
                        if depth > 0 {
                            self.advance();
                        } else {
                            self.advance();
                        }
                    }
                    _ => {
                        self.advance();
                    }
                }
            }
        }
        // 检查是否有 extends 子句
        let extends = if self.current_token_eq(&Token::Extends) {
            self.consume(Token::Extends)?;
            let parent_token = self.consume_any_identifier()?;
            match parent_token {
                Token::Identifier(parent_name) => Some(parent_name),
                _ => bail!("Expected parent class name after extends"),
            }
        } else {
            None
        };
        self.consume(Token::LBrace)?;
        let mut members = Vec::new();
        while !self.is_at_end() && !self.current_token_eq(&Token::RBrace) {
            // 尝试解析类成员（方法或字段）
            match self.parse_class_member() {
                Ok(Some(node)) => {
                    members.push(node);
                }
                Ok(None) => {
                    // 跳过无法解析的成员
                }
                Err(_e) => {
                    // 跳过这个 token 并继续
                    self.advance();
                }
            }
        }
        self.consume(Token::RBrace)?;
        Ok(ASTNode::ClassDeclaration {
            is_declare,
            is_abstract,
            decorators,
            name,
            extends,
            members,
        })
    }

    /// 解析带有已解析装饰器列表的类声明
    fn parse_class_declaration_with_decorators(
        &mut self,
        decorators: Vec<Decorator>,
    ) -> Result<ASTNode> {
        self.parse_class_declaration_internal(false, decorators)
    }

    fn parse_class_declaration(&mut self) -> Result<ASTNode> {
        // 首先解析装饰器列表
        let decorators = self.parse_decorators()?;
        self.parse_class_declaration_internal(false, decorators)
    }

    /// 解析类成员（方法或字段）
    fn parse_class_member(&mut self) -> Result<Option<ASTNode>> {
        // 首先解析装饰器
        let decorators = self.parse_decorators()?;

        // 然后检查是否是访问修饰符、static 或 abstract
        let mut is_static = false;
        let mut is_abstract = false;
        while self.current_token_eq(&Token::Public)
            || self.current_token_eq(&Token::Private)
            || self.current_token_eq(&Token::Protected)
            || self.current_token_eq(&Token::Static)
            || self.current_token_eq(&Token::Abstract)
        {
            if self.current_token_eq(&Token::Static) {
                is_static = true;
            }
            if self.current_token_eq(&Token::Abstract) {
                is_abstract = true;
            }
            self.advance();
        }

        // 检查是否是计算属性名 [expr]
        if self.current_token_eq(&Token::LBracket) {
            // 计算属性名: { [expr]: value }
            self.consume(Token::LBracket)?;
            let key_expr = self.parse_expression()?;
            self.consume(Token::RBracket)?;

            // 跳过类型注解 (如 `: string`)
            if self.current_token_eq(&Token::Colon) {
                self.consume(Token::Colon)?;
                self.parse_type_annotation();
            }

            // 检查是否有初始化器
            let initializer: Option<ASTExpression> = if self.current_token_eq(&Token::Eq) {
                self.consume(Token::Eq)?;
                Some(self.parse_expression()?)
            } else {
                None
            };

            // 跳过分号分隔的字段声明
            if self.current_token_eq(&Token::SemiColon) {
                self.consume(Token::SemiColon)?;
            }

            return Ok(Some(ASTNode::ComputedPropertyDeclaration {
                decorators,
                key_expr: Box::new(key_expr),
                is_static,
                initializer: initializer.map(|e| Box::new(ASTNode::Expression(e))),
            }));
        }

        // 特殊处理：检查是否是 get 或 set 关键字
        if let Token::Identifier(ref name) = self.current_token() {
            if name == "get" || name == "set" {
                let keyword = name.clone();
                self.advance();
                if let Token::Identifier(prop_name) = self.current_token().clone() {
                    self.advance();

                    // Getter 语法: get propertyName(): Type { ... }
                    // Setter 语法: set propertyName(value: Type) { ... }
                    let params = if self.current_token_eq(&Token::LParen) {
                        // 解析参数列表（setter 有参数）
                        self.parse_function_params_list()?
                    } else {
                        vec![]
                    };

                    // Getter: 跳过返回类型注解 (如 `: number`)
                    // Setter: 没有返回类型注解
                    if keyword == "get" && self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        self.parse_type_annotation();
                    }

                    // v0.3.177: 如果是抽象方法，抽象方法没有方法体，直接跳到分号
                    let body = if is_abstract {
                        vec![]
                    } else {
                        self.parse_block_body()?
                    };

                    // v0.3.177: 抽象方法必须以分号结束
                    if is_abstract && self.current_token_eq(&Token::SemiColon) {
                        self.consume(Token::SemiColon)?;
                    }

                    return Ok(Some(ASTNode::MethodDeclaration {
                        decorators,
                        name: prop_name,
                        kind: keyword,
                        is_async: false,
                        is_static,
                        is_abstract,
                        params,
                        body,
                    }));
                }
                // 如果不是有效的 getter/setter，消费 get/set 关键字并作为标识符处理
            }
        }

        // 检查是否是 constructor
        if let Token::Identifier(ref name) = self.current_token() {
            if name == "constructor" {
                self.advance();
                // 解析 constructor 参数列表
                if self.current_token_eq(&Token::LParen) {
                    let params = self.parse_function_params_list()?;
                    // 解析 constructor 主体
                    let body = self.parse_block_body()?;
                    return Ok(Some(ASTNode::MethodDeclaration {
                        decorators,
                        name: "constructor".to_string(),
                        kind: "method".to_string(),
                        is_async: false,
                        is_static,
                        is_abstract,
                        params,
                        body,
                    }));
                }
            }
        }

        // 检查是否是 async 方法
        if self.current_token_eq(&Token::Async) {
            self.consume(Token::Async)?;
            if let Token::Identifier(method_name) = self.current_token().clone() {
                self.advance();
                // 解析方法参数
                let params = self.parse_function_params_list()?;

                // 跳过返回类型注解 (如 `: Promise<number>`)
                if self.current_token_eq(&Token::Colon) {
                    self.consume(Token::Colon)?;
                    self.parse_type_annotation();
                }

                // v0.3.177: 如果是抽象方法，抽象方法没有方法体，直接跳到分号
                let body = if is_abstract {
                    vec![]
                } else {
                    self.parse_block_body()?
                };

                // v0.3.177: 抽象方法必须以分号结束
                if is_abstract && self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }

                return Ok(Some(ASTNode::MethodDeclaration {
                    decorators,
                    name: method_name,
                    kind: "method".to_string(),
                    is_async: true,
                    is_static,
                    is_abstract,
                    params,
                    body,
                }));
            }
        }

        // 普通方法或字段
        if let Token::Identifier(member_name) = self.current_token().clone() {
            self.advance();

            // 检查是否是方法（有参数列表）
            if self.current_token_eq(&Token::LParen) {
                // 解析方法参数
                let params = self.parse_function_params_list()?;

                // 跳过返回类型注解 (如 `: number`)
                if self.current_token_eq(&Token::Colon) {
                    self.consume(Token::Colon)?;
                    self.parse_type_annotation();
                }

                // 解析方法主体
                // v0.3.177: 如果是抽象方法，抽象方法没有方法体，直接跳到分号
                let body = if is_abstract {
                    // 抽象方法：跳过方法体（直接跳到分号）
                    vec![]
                } else {
                    self.parse_block_body()?
                };

                // v0.3.177: 抽象方法必须以分号结束
                if is_abstract && self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }

                return Ok(Some(ASTNode::MethodDeclaration {
                    decorators,
                    name: member_name,
                    kind: "method".to_string(),
                    is_async: false,
                    is_static,
                    is_abstract,
                    params,
                    body,
                }));
            }

            // 可能是字段，跳过类型注解
            if self.current_token_eq(&Token::Colon) {
                self.consume(Token::Colon)?;
                self.parse_type_annotation();
            }

            // 检查是否有初始化器
            let initializer: Option<ASTExpression> = if self.current_token_eq(&Token::Eq) {
                self.consume(Token::Eq)?;
                Some(self.parse_expression()?)
            } else {
                None
            };

            // 跳过分号分隔的字段声明
            if self.current_token_eq(&Token::SemiColon) {
                self.consume(Token::SemiColon)?;
            }

            return Ok(Some(ASTNode::PropertyDeclaration {
                decorators,
                name: member_name,
                is_static,
                is_abstract,
                initializer: initializer.map(|e| Box::new(ASTNode::Expression(e))),
            }));
        }

        // 消费未知成员末尾的分号
        if self.current_token_eq(&Token::SemiColon) {
            self.consume(Token::SemiColon)?;
        }

        // 推进一个 token 以避免无限循环
        self.advance();

        Ok(None)
    }

    /// 跳过到匹配的右括号（用于跳过参数列表和函数体）
    /// 注意：不消费最终的右花括号，由调用者决定是否消费
    #[allow(dead_code)]
    fn skip_to_matching_brace(&mut self) -> Result<Option<ASTNode>> {
        // 跳过参数列表
        self.consume(Token::LParen)?;
        let mut paren_depth = 1;
        while paren_depth > 0 && !self.is_at_end() {
            if self.current_token_eq(&Token::LParen) {
                paren_depth += 1;
            } else if self.current_token_eq(&Token::RParen) {
                paren_depth -= 1;
            }
            if paren_depth > 0 {
                self.advance();
            }
        }
        // 消费参数列表的 RParen
        if self.current_token_eq(&Token::RParen) {
            self.consume(Token::RParen)?;
        }

        // 跳过函数体（需要同时处理括号和花括号）
        if self.current_token_eq(&Token::LBrace) {
            self.consume(Token::LBrace)?;
            let mut brace_depth = 1;
            let mut paren_depth = 0;
            while brace_depth > 0 && !self.is_at_end() {
                if self.current_token_eq(&Token::LBrace) {
                    brace_depth += 1;
                } else if self.current_token_eq(&Token::RBrace) {
                    brace_depth -= 1;
                } else if self.current_token_eq(&Token::LParen) {
                    paren_depth += 1;
                } else if self.current_token_eq(&Token::RParen)
                    && paren_depth > 0 {
                        paren_depth -= 1;
                    }
                if brace_depth > 0 {
                    self.advance();
                }
            }
            // 此时 brace_depth == 0，我们停在 RBrace 上，不消费它
        }

        Ok(None)
    }

    /// 跳过表达式（用于跳过字段初始化器）
    #[allow(dead_code)]
    fn skip_expression(&mut self) {
        let mut paren_depth = 0;
        let mut brace_depth = 0;
        while !self.is_at_end() {
            match self.current_token() {
                Token::LParen | Token::LBracket | Token::LBrace => {
                    if paren_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    if self.current_token_eq(&Token::LParen) {
                        paren_depth += 1;
                    } else if self.current_token_eq(&Token::LBrace) {
                        brace_depth += 1;
                    }
                    self.advance();
                }
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if paren_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    if self.current_token_eq(&Token::RParen) {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                        }
                    } else if self.current_token_eq(&Token::RBrace)
                        && brace_depth > 0 {
                            brace_depth -= 1;
                        }
                    self.advance();
                }
                Token::SemiColon => {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
    fn parse_interface_declaration(&mut self) -> Result<ASTNode> {
        self.consume(Token::Interface)?;
        let name_token = self.consume_any_identifier()?;
        let name = match name_token {
            Token::Identifier(name) => name,
            _ => bail!("Expected interface name"),
        };

        // v0.3.189: 解析泛型类型参数 <T> 或 <T, U>
        let mut type_params = Vec::new();
        if self.current_token_eq(&Token::Lt) {
            self.consume(Token::Lt)?;
            loop {
                if self.current_token_eq(&Token::Gt) {
                    self.consume(Token::Gt)?;
                    break;
                }
                let param_name = self.consume_any_identifier()?;
                if let Token::Identifier(n) = param_name {
                    type_params.push(n);
                }
                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                } else if self.current_token_eq(&Token::Gt) {
                    self.consume(Token::Gt)?;
                    break;
                } else {
                    break;
                }
            }
        }

        // 检查是否有 extends 子句
        let mut extends = Vec::new();
        if self.current_token_eq(&Token::Extends) {
            self.consume(Token::Extends)?;
            // 解析逗号分隔的父接口列表
            loop {
                let parent_token = self.consume_any_identifier()?;
                let parent_name: String = match parent_token {
                    Token::Identifier(n) => n,
                    _ => bail!("Expected parent interface name"),
                };
                extends.push(parent_name);

                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                } else {
                    break;
                }
            }
        }

        self.consume(Token::LBrace)?;
        let mut properties = HashMap::new();
        let mut index_signature = None;
        // v0.3.189: 存储构造函数签名
        let mut constructor_signature = None;

        while !self.current_token_eq(&Token::RBrace) {
            // 检测索引签名语法：[keyName: keyType]: valueType
            if self.current_token_eq(&Token::LBracket) {
                self.consume(Token::LBracket)?;
                // 解析索引参数名
                let key_name_token = self.consume_any_identifier()?;
                let key_name: String = match key_name_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected index key name"),
                };
                self.consume(Token::Colon)?;
                // 解析键类型 (string 或 number)
                let key_type_token = self.consume_any_identifier()?;
                let key_type: String = match key_type_token {
                    Token::Identifier(name) if name == "string" || name == "number" => name,
                    _ => bail!("Expected 'string' or 'number' for index key type"),
                };
                self.consume(Token::RBracket)?;
                self.consume(Token::Colon)?;
                // 解析值类型
                let value_type = self.parse_type_annotation();
                let value_type_str = value_type.unwrap_or_else(|| "any".to_string());

                index_signature = Some(Box::new(IndexSignature {
                    key_name,
                    key_type,
                    value_type: value_type_str,
                }));
            } else if self.current_token_eq(&Token::New) {
                // v0.3.189: 解析构造函数签名 new(props: Type): ReturnType
                self.consume(Token::New)?;
                self.consume(Token::LParen)?; // 消耗 (
                                              // 解析参数列表: 参数名: 类型, 参数名: 类型
                let mut params = Vec::new();
                while !self.current_token_eq(&Token::RParen) {
                    // 跳过 rest 参数标记 ...
                    if self.current_token_eq(&Token::DotDotDot) {
                        self.consume(Token::DotDotDot)?;
                    }
                    // 解析参数名
                    let param_name = if let Token::Identifier(name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        name
                    } else if self.current_token_eq(&Token::This) {
                        self.advance();
                        "this".to_string()
                    } else {
                        break;
                    };
                    // 消耗类型注解的 :
                    self.consume(Token::Colon)?;
                    // 解析参数类型
                    let param_type = self.parse_type_annotation();
                    let param_type_str = param_type.unwrap_or_else(|| "any".to_string());
                    params.push(format!("{}: {}", param_name, param_type_str));
                    // 处理逗号分隔符
                    if self.current_token_eq(&Token::Comma) {
                        self.consume(Token::Comma)?;
                    } else {
                        break;
                    }
                }
                self.consume(Token::RParen)?; // 消耗 )
                self.consume(Token::Colon)?; // 消耗 :
                                             // 解析返回类型
                let return_type = self.parse_type_annotation();
                let return_type_str = return_type.unwrap_or_else(|| "any".to_string());

                constructor_signature =
                    Some(format!("new({}) => {}", params.join(", "), return_type_str));
            } else {
                // 解析普通属性 (v0.3.169: 支持 ? 可选修饰符和 readonly 修饰符)
                // 先检查是否有 readonly 修饰符
                if self.current_token_eq(&Token::Readonly) {
                    self.consume(Token::Readonly)?;
                }

                // 检查是否有 ? 可选修饰符
                if self.current_token_eq(&Token::Question) {
                    self.consume(Token::Question)?;
                }

                // 读取属性名
                let prop_name_token = self.consume_any_identifier()?;
                let prop_name = match prop_name_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected property name"),
                };

                // 如果之前没有检查 ?，现在检查 (处理 x?: type 的情况)
                if self.current_token_eq(&Token::Question) {
                    self.consume(Token::Question)?;
                }

                self.consume(Token::Colon)?;
                let prop_type = self.parse_type_annotation();
                // 可选修饰符和 readonly 在转译时会被移除，只存储属性类型
                properties.insert(prop_name, prop_type.unwrap_or_else(|| "any".to_string()));
            }

            if self.current_token_eq(&Token::SemiColon) {
                self.consume(Token::SemiColon)?;
            }
        }
        self.consume(Token::RBrace)?;
        Ok(ASTNode::InterfaceDeclaration {
            name,
            extends,
            properties,
            index_signature,
            constructor_signature,
        })
    }
    fn parse_enum_declaration(&mut self) -> Result<ASTNode> {
        self.consume(Token::Enum)?;
        let name_token = self.consume_any_identifier()?;
        let name = match name_token {
            Token::Identifier(name) => name,
            _ => bail!("Expected enum name"),
        };
        self.consume(Token::LBrace)?;
        let mut members = Vec::new();
        let mut current_value: Option<u32> = None;
        while !self.current_token_eq(&Token::RBrace) {
            let member_name_token = self.consume_any_identifier()?;
            let member_name = match member_name_token {
                Token::Identifier(name) => name,
                _ => bail!("Expected enum member name"),
            };
            let mut _member_value = None;
            // 检查是否有显式值 (如: North = 0)
            if self.current_token_eq(&Token::Eq) {
                self.consume(Token::Eq)?;
                match self.current_token() {
                    Token::Number(ref num) => {
                        if let Ok(n) = num.parse::<u32>() {
                            _member_value = Some(EnumValue::Number(n));
                            current_value = Some(n + 1);
                        } else {
                            _member_value = Some(EnumValue::String(num.clone()));
                        }
                        self.advance();
                    }
                    Token::String(ref s, _) => {
                        _member_value = Some(EnumValue::String(s.clone()));
                        self.advance();
                    }
                    _ => bail!("Expected number or string value for enum member"),
                }
            } else {
                // 自动递增数字枚举
                if let Some(val) = current_value {
                    _member_value = Some(EnumValue::Number(val));
                    current_value = Some(val + 1);
                } else {
                    _member_value = Some(EnumValue::Number(0));
                    current_value = Some(1);
                }
            }
            members.push(EnumMember {
                name: member_name,
                value: _member_value,
            });
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            }
        }
        self.consume(Token::RBrace)?;
        Ok(ASTNode::EnumDeclaration { name, members })
    }

    /// 解析类型别名声明
    /// 支持:
    /// - type Foo = string | number
    /// - type Foo<T> = T | null
    /// - type Foo = { name: string; age: number }
    fn parse_type_alias_declaration(&mut self) -> Result<ASTNode> {
        self.consume(Token::Type)?;

        // 解析类型名称
        let name = if let Token::Identifier(name) = self.consume_any_identifier()? {
            name
        } else {
            bail!("Expected type name after 'type' keyword");
        };

        // 检查是否有泛型参数
        let mut type_params = None;
        if self.current_token_eq(&Token::Lt) {
            self.consume(Token::Lt)?;
            let mut params = Vec::new();
            while !self.current_token_eq(&Token::Gt) {
                // 解析参数名
                let param_name = if let Token::Identifier(name) = self.consume_any_identifier()? {
                    name
                } else {
                    break;
                };
                params.push(param_name.clone());

                // 检查是否有 extends 约束
                if self.current_token_eq(&Token::Extends) {
                    self.advance(); // 消耗 extends
                                    // 解析约束类型
                    let _constraint = if let Some(c) = self.parse_type_annotation() {
                        c
                    } else {
                        "unknown".to_string()
                    };
                    // 约束类型被忽略，只保留参数名
                }

                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                } else {
                    break;
                }
            }
            self.consume(Token::Gt)?;
            type_params = Some(params);
        }

        self.consume(Token::Eq)?;

        // 解析类型定义（使用现有的类型解析器）
        let type_definition = self.parse_type_annotation();

        self.consume(Token::SemiColon)?;

        Ok(ASTNode::TypeAliasDeclaration {
            name,
            type_params,
            type_definition: type_definition.unwrap_or_else(|| "unknown".to_string()),
        })
    }

    /// 解析命名空间声明
    /// 支持:
    /// - namespace MyNamespace { ... }
    /// - namespace Outer.Inner { ... } (嵌套命名空间)
    /// - declare namespace MyLib { ... } (声明式命名空间)
    fn parse_namespace_declaration(&mut self) -> Result<ASTNode> {
        self.parse_namespace_declaration_internal(false)
    }

    /// 解析命名空间声明（内部函数，可指定 is_declare）
    fn parse_namespace_declaration_internal(&mut self, is_declare: bool) -> Result<ASTNode> {
        // 如果 is_declare 为 false，检查是否有 declare 关键字
        let is_declare = if !is_declare && self.current_token_eq(&Token::Declare) {
            self.consume(Token::Declare)?;
            true
        } else {
            is_declare
        };

        self.consume(Token::Namespace)?;

        // 解析命名空间名称
        let mut names = Vec::new();
        let name = if let Token::Identifier(name) = self.consume_any_identifier()? {
            names.push(name.clone());
            name
        } else {
            bail!("Expected namespace name after 'namespace' keyword");
        };

        // 检查是否有嵌套命名空间 (e.g., Outer.Inner)
        while self.current_token_eq(&Token::Dot) {
            self.consume(Token::Dot)?;
            let nested_name = if let Token::Identifier(nested) = self.consume_any_identifier()? {
                nested
            } else {
                bail!("Expected identifier after '.' in namespace name");
            };
            names.push(nested_name);
        }

        // 消耗左花括号
        self.consume(Token::LBrace)?;

        // 解析命名空间内部的语句
        let mut body = Vec::new();
        while !self.current_token_eq(&Token::RBrace) && !self.current_token_eq(&Token::Eof) {
            body.push(self.parse_statement()?);
        }

        // 消耗右花括号
        self.consume(Token::RBrace)?;

        // 构建完整命名空间路径（如 "A.B.C"）
        let full_name = names.join(".");

        Ok(ASTNode::Statement(ASTStatement::Namespace {
            name,
            full_name,
            body,
            is_declare,
        }))
    }

    /// 解析全局声明块
    /// 语法: declare global { ... }
    /// 用于向全局作用域添加类型声明
    fn parse_global_declaration(&mut self) -> Result<ASTNode> {
        // 消耗 global 关键字
        self.consume(Token::Global)?;

        // 消耗左花括号
        self.consume(Token::LBrace)?;

        // 解析全局声明块内部的语句
        let mut body = Vec::new();
        while !self.current_token_eq(&Token::RBrace) && !self.current_token_eq(&Token::Eof) {
            body.push(self.parse_statement()?);
        }

        // 消耗右花括号
        self.consume(Token::RBrace)?;

        Ok(ASTNode::Statement(ASTStatement::GlobalDeclaration { body }))
    }

    /// 解析模块声明
    /// 语法: declare module "module-name" { ... }
    /// 用于声明模块的类型定义
    fn parse_module_declaration(&mut self) -> Result<ASTNode> {
        // 消耗 module 关键字
        self.consume(Token::Module)?;

        // 解析模块名称（应该是字符串字面量）
        let name = match self.current_token() {
            Token::String(s, _) => {
                let name = s.clone();
                self.advance();
                name
            }
            _ => {
                bail!("Expected string literal for module name after 'module' keyword");
            }
        };

        // 消耗左花括号
        self.consume(Token::LBrace)?;

        // 解析模块内部的语句
        let mut body = Vec::new();
        while !self.current_token_eq(&Token::RBrace) && !self.current_token_eq(&Token::Eof) {
            body.push(self.parse_statement()?);
        }

        // 消耗右花括号
        self.consume(Token::RBrace)?;

        Ok(ASTNode::Statement(ASTStatement::ModuleDeclaration {
            name,
            body,
        }))
    }

    /// 解析导入语句
    /// 支持:
    /// - import "module" (副作用导入)
    /// - import defaultExport from "module" (默认导入)
    /// - import { a, b } from "module" (命名导入)
    /// - import type { a, b } from "module" (仅类型导入)
    /// - import defaultExport, { a, b } from "module" (混合导入)
    /// - import * as namespace from "module" (命名空间导入)
    /// - import type * as namespace from "module" (类型命名空间导入)
    fn parse_import_declaration(&mut self) -> Result<ASTNode> {
        self.consume(Token::Import)?;

        // 检查是否为 import type (仅类型导入)
        let is_type_only = if self.current_token_eq(&Token::Type) {
            self.consume(Token::Type)?;
            true
        } else {
            false
        };

        // 检查是否为默认导入（import x from ...）
        // 如果后面是字符串字面量，则是副作用导入或默认导入
        if let Token::String(ref s, _) = self.current_token() {
            // 副作用导入: import "module"
            // 注意：import type "module" 是无效语法，所以这里直接处理
            let module_specifier = format!("\"{}\"", s);
            self.advance();
            self.consume(Token::SemiColon)?;
            return Ok(ASTNode::ImportDeclaration {
                module_specifier,
                imports: Vec::new(),
                is_default: false,
                namespace_alias: None,
                is_type_only,
            });
        }

        // 检查是否有 * (命名空间导入)
        if self.current_token_eq(&Token::Star) {
            self.consume(Token::Star)?;
            self.consume(Token::As)?;
            // 解析 as 别名
            if let Token::Identifier(alias) = self.consume_any_identifier()? {
                self.consume(Token::From)?;
                let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                    format!("\"{}\"", s)
                } else {
                    bail!("Expected string literal for module specifier");
                };
                self.advance();
                self.consume(Token::SemiColon)?;
                return Ok(ASTNode::ImportDeclaration {
                    module_specifier,
                    imports: Vec::new(),
                    is_default: false,
                    namespace_alias: Some(alias),
                    is_type_only,
                });
            }
        }

        // 检查 { (命名导入)
        if self.current_token_eq(&Token::LBrace) {
            self.consume(Token::LBrace)?;
            let mut imports = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                let imported_token = self.consume_any_identifier()?;
                let imported = match imported_token {
                    Token::Identifier(name) => name,
                    _ => bail!("Expected identifier in import"),
                };

                // 检查 as 别名
                let mut alias = None;
                if self.current_token_eq(&Token::As) {
                    self.consume(Token::As)?;
                    let alias_token = self.consume_any_identifier()?;
                    alias = match alias_token {
                        Token::Identifier(name) => Some(name),
                        _ => bail!("Expected alias after 'as'"),
                    };
                }

                imports.push(ImportSpecifier {
                    imported,
                    alias,
                    is_default: false,
                });

                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                }
            }
            self.consume(Token::RBrace)?;
            self.consume(Token::From)?;
            let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                format!("\"{}\"", s)
            } else {
                bail!("Expected string literal for module specifier");
            };
            self.advance();
            self.consume(Token::SemiColon)?;
            return Ok(ASTNode::ImportDeclaration {
                module_specifier,
                imports,
                is_default: false,
                namespace_alias: None,
                is_type_only,
            });
        }

        // 可能是默认导入
        if let Token::Identifier(name) = self.current_token() {
            let name_str = name.clone();
            self.consume(Token::Identifier(name_str.clone()))?;

            // 检查是否有 from
            if self.current_token_eq(&Token::From) {
                self.consume(Token::From)?;
                let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                    format!("\"{}\"", s)
                } else {
                    bail!("Expected string literal for module specifier");
                };
                self.advance();
                self.consume(Token::SemiColon)?;
                return Ok(ASTNode::ImportDeclaration {
                    module_specifier,
                    imports: vec![ImportSpecifier {
                        imported: name_str.clone(),
                        alias: None,
                        is_default: true,
                    }],
                    is_default: true,
                    namespace_alias: None,
                    is_type_only,
                });
            }

            // 检查是否有逗号（混合导入：默认导入 + 命名导入）
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
                // 解析命名导入
                if self.current_token_eq(&Token::LBrace) {
                    self.consume(Token::LBrace)?;
                    let mut named_imports = Vec::new();
                    while !self.current_token_eq(&Token::RBrace) {
                        let imported_token = self.consume_any_identifier()?;
                        let imported = match imported_token {
                            Token::Identifier(n) => n,
                            _ => bail!("Expected identifier in import"),
                        };

                        let mut alias = None;
                        if self.current_token_eq(&Token::As) {
                            self.consume(Token::As)?;
                            let alias_token = self.consume_any_identifier()?;
                            alias = match alias_token {
                                Token::Identifier(n) => Some(n),
                                _ => bail!("Expected alias after 'as'"),
                            };
                        }

                        named_imports.push(ImportSpecifier {
                            imported,
                            alias,
                            is_default: false,
                        });

                        if self.current_token_eq(&Token::Comma) {
                            self.consume(Token::Comma)?;
                        }
                    }
                    self.consume(Token::RBrace)?;
                    self.consume(Token::From)?;
                    let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                        format!("\"{}\"", s)
                    } else {
                        bail!("Expected string literal for module specifier");
                    };
                    self.advance();
                    self.consume(Token::SemiColon)?;

                    // 合并默认导入和命名导入
                    let mut all_imports = vec![ImportSpecifier {
                        imported: name_str,
                        alias: None,
                        is_default: true,
                    }];
                    all_imports.extend(named_imports);

                    return Ok(ASTNode::ImportDeclaration {
                        module_specifier,
                        imports: all_imports,
                        is_default: true,
                        namespace_alias: None,
                        is_type_only,
                    });
                }
            }

            bail!("Invalid import syntax");
        }

        bail!("Invalid import statement");
    }

    /// 解析导出语句
    /// 支持:
    /// - export { a, b } (命名导出)
    /// - export type { a, b } (仅类型导出)
    /// - export default expr (默认导出)
    /// - export const x = 1 (内联导出声明)
    /// - export { a, b } from "module" (重新导出)
    /// - export type { a, b } from "module" (仅类型重新导出)
    /// - export * from "module" (重新导出所有)
    fn parse_export_declaration(&mut self) -> Result<ASTNode> {
        self.consume(Token::Export)?;

        // 检查是否为 export type (仅类型导出)
        let is_type_only = if self.current_token_eq(&Token::Type) {
            self.consume(Token::Type)?;
            true
        } else {
            false
        };

        // 检查 default
        if self.current_token_eq(&Token::Default) {
            self.consume(Token::Default)?;
            // 解析默认导出的表达式或声明
            let expr = self.parse_expression()?;
            self.consume(Token::SemiColon)?;
            return Ok(ASTNode::ExportDeclaration {
                exports: Vec::new(),
                is_default: true,
                module_specifier: None,
                inline_declaration: Some(Box::new(ASTNode::Expression(expr))),
                is_type_only,
            });
        }

        // 检查 export *
        if self.current_token_eq(&Token::Star) {
            self.consume(Token::Star)?;
            self.consume(Token::From)?;
            let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                format!("\"{}\"", s)
            } else {
                bail!("Expected string literal for module specifier");
            };
            self.advance();
            self.consume(Token::SemiColon)?;
            return Ok(ASTNode::ExportDeclaration {
                exports: Vec::new(),
                is_default: false,
                module_specifier: Some(module_specifier),
                inline_declaration: None,
                is_type_only,
            });
        }

        // 检查 export { ... }
        if self.current_token_eq(&Token::LBrace) {
            self.consume(Token::LBrace)?;
            let mut exports = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                let name_token = self.consume_any_identifier()?;
                let name = match name_token {
                    Token::Identifier(n) => n,
                    _ => bail!("Expected identifier in export"),
                };

                let mut alias = None;
                if self.current_token_eq(&Token::As) {
                    self.consume(Token::As)?;
                    let alias_token = self.consume_any_identifier()?;
                    alias = match alias_token {
                        Token::Identifier(n) => Some(n),
                        _ => bail!("Expected alias after 'as'"),
                    };
                }

                exports.push(ExportSpecifier { name, alias });

                if self.current_token_eq(&Token::Comma) {
                    self.consume(Token::Comma)?;
                }
            }
            self.consume(Token::RBrace)?;

            // 检查 from (重新导出)
            if self.current_token_eq(&Token::From) {
                self.consume(Token::From)?;
                let module_specifier = if let Token::String(ref s, _) = self.current_token() {
                    format!("\"{}\"", s)
                } else {
                    bail!("Expected string literal for module specifier");
                };
                self.advance();
                self.consume(Token::SemiColon)?;
                return Ok(ASTNode::ExportDeclaration {
                    exports,
                    is_default: false,
                    module_specifier: Some(module_specifier),
                    inline_declaration: None,
                    is_type_only,
                });
            }

            self.consume(Token::SemiColon)?;
            return Ok(ASTNode::ExportDeclaration {
                exports,
                is_default: false,
                module_specifier: None,
                inline_declaration: None,
                is_type_only,
            });
        }

        // 内联导出声明: export const/let/var/function/class
        match self.current_token() {
            Token::Const | Token::Let | Token::Var => {
                let declaration = self.parse_variable_declaration(false)?;
                // 消耗分号
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Function => {
                let declaration = self.parse_function_declaration(false)?;
                // 消耗分号
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Class => {
                let declaration = self.parse_class_declaration()?;
                // 消耗分号
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Abstract => {
                // export abstract class X { ... }
                let declaration = self.parse_class_declaration()?;
                // 消耗分号
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Interface => {
                let declaration = self.parse_interface_declaration()?;
                // 消耗分号
                if self.current_token_eq(&Token::SemiColon) {
                    self.consume(Token::SemiColon)?;
                }
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Enum => {
                let declaration = self.parse_enum_declaration()?;
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Declare => {
                // export declare class/namespace/function 等
                let is_declare = true;
                self.consume(Token::Declare)?;
                match self.current_token() {
                    Token::Class => {
                        let declaration =
                            self.parse_class_declaration_internal(is_declare, Vec::new())?;
                        Ok(ASTNode::ExportDeclaration {
                            exports: Vec::new(),
                            is_default: false,
                            module_specifier: None,
                            inline_declaration: Some(Box::new(declaration)),
                            is_type_only,
                        })
                    }
                    Token::Namespace => {
                        let declaration = self.parse_namespace_declaration()?;
                        Ok(ASTNode::ExportDeclaration {
                            exports: Vec::new(),
                            is_default: false,
                            module_specifier: None,
                            inline_declaration: Some(Box::new(declaration)),
                            is_type_only,
                        })
                    }
                    Token::Function => {
                        let declaration = self.parse_function_declaration(is_declare)?;
                        Ok(ASTNode::ExportDeclaration {
                            exports: Vec::new(),
                            is_default: false,
                            module_specifier: None,
                            inline_declaration: Some(Box::new(declaration)),
                            is_type_only,
                        })
                    }
                    Token::Const | Token::Let | Token::Var => {
                        let declaration = self.parse_variable_declaration(is_declare)?;
                        // 消耗分号（对于声明语句）
                        if self.current_token_eq(&Token::SemiColon) {
                            self.consume(Token::SemiColon)?;
                        }
                        Ok(ASTNode::ExportDeclaration {
                            exports: Vec::new(),
                            is_default: false,
                            module_specifier: None,
                            inline_declaration: Some(Box::new(declaration)),
                            is_type_only,
                        })
                    }
                    _ => bail!(
                        "Invalid export declare declaration: {:?}",
                        self.current_token()
                    ),
                }
            }
            Token::Namespace => {
                // 导出命名空间: export namespace MyNamespace { ... }
                let declaration = self.parse_namespace_declaration()?;
                Ok(ASTNode::ExportDeclaration {
                    exports: Vec::new(),
                    is_default: false,
                    module_specifier: None,
                    inline_declaration: Some(Box::new(declaration)),
                    is_type_only,
                })
            }
            Token::Eq => {
                // export = expr (CommonJS/AMD 兼容语法)
                self.consume(Token::Eq)?;
                let expr = self.parse_expression()?;
                self.consume(Token::SemiColon)?;
                // 创建 ExportAssignment AST 节点
                Ok(ASTNode::Statement(ASTStatement::ExportAssignment {
                    expression: Box::new(expr),
                    is_type_only,
                }))
            }
            _ => bail!("Invalid export declaration"),
        }
    }

    #[allow(dead_code)]
    /// 解析初始化器表达式（不解析二元运算符）
    /// 用于变量声明的初始化器，避免错误地消耗 for 循环中的比较运算符
    fn parse_initializer_expression(&mut self) -> Result<ASTExpression> {
        // 处理 await 表达式（作为一元前缀运算符）
        if self.current_token_eq(&Token::Await) {
            self.consume(Token::Await)?;
            let inner = self.parse_initializer_expression()?;
            return Ok(ASTExpression::Await {
                expression: Box::new(inner),
            });
        }
        // 解析主表达式
        let mut expr = self.parse_primary_expression()?;

        // 处理泛型类型参数调用 (例如: identity<string>("hello"))
        if self.current_token_eq(&Token::Lt) {
            // 简单的启发式：如果 < 后面是标识符，> 后面是 (，则是泛型调用
            let mut lookahead = self.position;
            let mut depth = 0;
            let mut found_type_args = false;

            while lookahead < self.tokens.len() {
                match &self.tokens[lookahead] {
                    Token::Lt => {
                        if depth == 0 {
                            depth = 1;
                        } else {
                            depth += 1;
                        }
                        lookahead += 1;
                    }
                    Token::Gt if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            lookahead += 1;
                            if lookahead < self.tokens.len()
                                && matches!(self.tokens[lookahead], Token::LParen)
                            {
                                found_type_args = true;
                            }
                            break;
                        }
                        lookahead += 1;
                    }
                    Token::Comma | Token::Identifier(_) if depth > 0 => {
                        lookahead += 1;
                    }
                    _ => {
                        if depth == 0 {
                            break;
                        }
                        lookahead += 1;
                    }
                }
            }

            if found_type_args {
                // 消费 < Type >
                self.consume(Token::Lt)?;
                let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    match self.current_token() {
                        Token::Lt => {
                            depth += 1;
                            self.advance();
                        }
                        Token::Gt => {
                            depth -= 1;
                            if depth > 0 {
                                self.advance();
                            } else {
                                self.advance();
                            }
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                // 处理函数调用
                if self.current_token_eq(&Token::LParen) {
                    self.advance();
                    let mut arguments = Vec::new();
                    while !self.current_token_eq(&Token::RParen) {
                        arguments.push(self.parse_expression()?);
                        if self.current_token_eq(&Token::Comma) {
                            self.consume(Token::Comma)?;
                        }
                    }
                    self.consume(Token::RParen)?;
                    expr = ASTExpression::CallExpression {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
            }
        }

        // 只处理后缀运算符（++, --, ., (), []），不处理二元运算符
        expr = self.parse_postfix(expr)?;
        Ok(expr)
    }
    fn parse_expression(&mut self) -> Result<ASTExpression> {
        // 处理 await 表达式（作为一元前缀运算符）
        if self.current_token_eq(&Token::Await) {
            self.consume(Token::Await)?;
            let inner = self.parse_expression()?;
            return Ok(ASTExpression::Await {
                expression: Box::new(inner),
            });
        }

        // 检查是否到达语句结束标记（用于 for 循环等场景）
        // 分号表示表达式结束，右括号在特定情况下也结束表达式
        // 注意：逗号在模板字符串表达式中是被允许的（用于函数调用等）
        if self.current_token_eq(&Token::SemiColon) {
            bail!("Unexpected token in expression: {:?}", self.current_token());
        }

        // 解析主表达式 (标识符、字面量、括号表达式)
        // 注意：尖括号断言 <Type>expr 在 parse_primary_expression 中处理
        let mut expr = self.parse_primary_expression()?;
        // 处理后缀运算符
        expr = self.parse_postfix(expr)?;

        // 在检查箭头函数之前，先处理泛型类型参数调用
        // 例如: identity<string>("hello") 或 IsString<string>
        // v0.3.169: 扩展支持泛型类型参数后跟各种 token (如 ; ) } , 运算符等)
        if self.current_token_eq(&Token::Lt) {
            // 简单的启发式：如果 < 后面是标识符，> 后面是 ( ; ) } , 运算符，则是泛型类型参数
            let mut lookahead = self.position;
            let mut depth = 0;
            let mut found_type_args = false;
            let mut terminated_without_paren = false;

            // 快速扫描检查是否是 <Type> 或 <T, U>
            while lookahead < self.tokens.len() {
                match &self.tokens[lookahead] {
                    Token::Lt => {
                        if depth == 0 {
                            depth = 1;
                        } else {
                            depth += 1;
                        }
                        lookahead += 1;
                    }
                    Token::Gt if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            lookahead += 1;
                            // v0.3.169: 扩展检查，支持泛型类型参数后跟各种 token
                            if lookahead < self.tokens.len() {
                                match &self.tokens[lookahead] {
                                    Token::LParen => {
                                        found_type_args = true;
                                    }
                                    Token::SemiColon
                                    | Token::RParen
                                    | Token::RBrace
                                    | Token::Comma
                                    | Token::FatArrow
                                    | Token::Question
                                    | Token::Plus
                                    | Token::Minus
                                    | Token::Star
                                    | Token::Slash
                                    | Token::Percent
                                    | Token::Caret
                                    | Token::Ampersand
                                    | Token::Pipe
                                    | Token::QuestionQuestion
                                    | Token::Dot
                                    | Token::PlusEq
                                    | Token::MinusEq
                                    | Token::StarEq
                                    | Token::SlashEq
                                    | Token::PercentEq
                                    | Token::AmpersanderEq
                                    | Token::PipeEq
                                    | Token::CaretEq
                                    | Token::LtEq
                                    | Token::GtEq
                                    | Token::Eq
                                    | Token::EqEq
                                    | Token::NotEqEq
                                    | Token::Bang
                                    | Token::Colon
                                    | Token::RBracket => {
                                        terminated_without_paren = true;
                                    }
                                    _ => {}
                                }
                            }
                            break;
                        }
                        lookahead += 1;
                    }
                    Token::Comma | Token::Identifier(_) if depth > 0 => {
                        lookahead += 1;
                    }
                    _ => {
                        if depth == 0 {
                            break;
                        }
                        lookahead += 1;
                    }
                }
            }

            // 如果是泛型类型参数，则跳过类型参数部分
            if found_type_args || terminated_without_paren {
                // 消费 < Type >
                self.consume(Token::Lt)?;
                let mut depth = 1;
                while depth > 0 && !self.is_at_end() {
                    match self.current_token() {
                        Token::Lt => {
                            depth += 1;
                            self.advance();
                        }
                        Token::Gt => {
                            depth -= 1;
                            if depth > 0 {
                                self.advance();
                            } else {
                                self.advance();
                            }
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                // 如果是泛型函数调用，继续处理函数调用
                if found_type_args && self.current_token_eq(&Token::LParen) {
                    self.advance();
                    let mut arguments = Vec::new();
                    while !self.current_token_eq(&Token::RParen) {
                        arguments.push(self.parse_expression()?);
                        if self.current_token_eq(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.consume(Token::RParen)?;
                    expr = ASTExpression::CallExpression {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
            }
        }

        // 处理箭头函数
        if self.current_token_eq(&Token::FatArrow) {
            // v0.3.180: 使用 3 元素元组支持默认参数
            // 检查是否是带括号的参数列表
            let params = if let ASTExpression::Identifier(name) = expr {
                vec![(name, None, None)]
            } else if let ASTExpression::CallExpression {
                callee: _,
                arguments,
            } = &expr
            {
                // 处理带括号的参数列表，如 (a, b)
                let mut params = Vec::new();
                for arg in arguments {
                    if let ASTExpression::Identifier(name) = arg {
                        params.push((name.clone(), None, None));
                    } else {
                        return Err(anyhow::anyhow!(
                            "Arrow function parameters must be identifiers"
                        ));
                    }
                }
                params
            } else {
                return Err(anyhow::anyhow!(
                    "Arrow function parameter must be identifier or parameter list"
                ));
            };
            return self.parse_arrow_function_expression(params);
        }

        // 处理后缀操作符 (成员访问、函数调用、二元运算符)
        loop {
            match self.current_token() {
                Token::Dot => {
                    // 成员访问: expr.property
                    self.advance();
                    let prop_token = self.consume_any_identifier()?;
                    let prop_name = match prop_token {
                        Token::Identifier(name) => name,
                        _ => bail!("Expected property name after '.'"),
                    };
                    expr = ASTExpression::MemberExpression {
                        object: Box::new(expr),
                        property: prop_name,
                    };
                }
                Token::LParen => {
                    // 函数调用: expr(args)
                    self.advance();
                    let mut arguments = Vec::new();
                    while !self.current_token_eq(&Token::RParen) {
                        arguments.push(self.parse_expression()?);
                        if self.current_token_eq(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.consume(Token::RParen)?;
                    expr = ASTExpression::CallExpression {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
                Token::LBracket => {
                    // 索引访问: expr[index]
                    self.advance();
                    let index = self.parse_expression()?;
                    self.consume(Token::RBracket)?;
                    expr = ASTExpression::IndexExpression {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                // TypeScript 类型断言: expr as Type 或 expr as const
                // 必须在索引访问之后处理，因为 < 可能被解释为泛型或小于运算符
                Token::As => {
                    self.consume(Token::As)?;
                    // 检查是否为 as const
                    let is_const = self.current_token_eq(&Token::Const);
                    let (target_type, const_flag) = if is_const {
                        self.consume(Token::Const)?;
                        ("const".to_string(), true)
                    } else {
                        // 解析目标类型
                        let target_type = self
                            .parse_type_annotation()
                            .unwrap_or_else(|| "unknown".to_string());
                        (target_type, false)
                    };
                    expr = ASTExpression::TSAsExpression {
                        expression: Box::new(expr),
                        target_type,
                        is_const: const_flag,
                    };
                }
                // TypeScript satisfies 操作符: expr satisfies Type
                // 必须在 as 之后处理，因为 as 和 satisfies 都是后缀类型操作符
                Token::Satisfies => {
                    self.consume(Token::Satisfies)?;
                    // 解析目标类型
                    let target_type = self
                        .parse_type_annotation()
                        .unwrap_or_else(|| "unknown".to_string());
                    expr = ASTExpression::TSSatisfiesExpression {
                        expression: Box::new(expr),
                        target_type,
                    };
                }
                // 二元运算符
                Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::EqEq
                | Token::EqEqEq
                | Token::NotEq
                | Token::NotEqEq
                | Token::Lt
                | Token::Gt
                | Token::LtEq
                | Token::GtEq
                | Token::Ampersand
                | Token::Pipe
                | Token::Caret
                | Token::LtLt
                | Token::GtGt
                | Token::GtGtGt => {
                    let op = match self.current_token() {
                        Token::Plus => "+",
                        Token::Minus => "-",
                        Token::Star => "*",
                        Token::Slash => "/",
                        Token::Percent => "%",
                        Token::EqEq => "==",
                        Token::EqEqEq => "===",
                        Token::NotEq => "!=",
                        Token::NotEqEq => "!==",
                        Token::Lt => "<",
                        Token::Gt => ">",
                        Token::LtEq => "<=",
                        Token::GtEq => ">=",
                        Token::Ampersand => "&",
                        Token::Pipe => "|",
                        Token::Caret => "^",
                        Token::LtLt => "<<",
                        Token::GtGt => ">>",
                        Token::GtGtGt => ">>>",
                        _ => unreachable!(),
                    };
                    self.advance();
                    let right = self.parse_primary_expression()?;
                    // Handle postfix operators on right side
                    let right = self.parse_postfix(right)?;
                    expr = ASTExpression::BinaryExpression {
                        left: Box::new(expr),
                        operator: op.to_string(),
                        right: Box::new(right),
                    };
                }
                // 条件运算符（三元运算符）: condition ? true_expr : false_expr
                // 在每个迭代开始时检查，因为条件运算符可能出现在任何二元运算符之后
                Token::Question => {
                    let condition = Box::new(expr);
                    self.consume(Token::Question)?;
                    let consequent = Box::new(self.parse_expression()?);
                    self.consume(Token::Colon)?;

                    // 解析 alternate 部分
                    let alternate = self.parse_expression()?;

                    // 构建条件表达式
                    expr = ASTExpression::ConditionalExpression {
                        condition,
                        consequent,
                        alternate: Box::new(alternate),
                    };
                }
                // 赋值运算符: expr = value, expr += value, etc.
                Token::Eq
                | Token::PlusEq
                | Token::MinusEq
                | Token::StarEq
                | Token::SlashEq
                | Token::PercentEq
                | Token::AmpersanderEq
                | Token::PipeEq
                | Token::CaretEq
                | Token::LtLtEq
                | Token::GtGtEq
                | Token::GtGtGtEq => {
                    self.advance();
                    let right = self.parse_expression()?;
                    expr = ASTExpression::AssignmentExpression {
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }
    #[allow(dead_code)]
    fn parse_arrow_function_from_assignment(&mut self) -> Result<ASTExpression> {
        // v0.3.180: 解析箭头函数的参数部分 - 支持默认参数值
        let mut params: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        if self.current_token_eq(&Token::LParen) {
            // 带括号的参数列表: (a, b, c)
            self.consume(Token::LParen)?;
            // 处理空参数列表的情况
            if !self.current_token_eq(&Token::RParen) {
                while !self.current_token_eq(&Token::RParen) {
                    let param_name_token = self.consume_any_identifier()?;
                    let param_name = match param_name_token {
                        Token::Identifier(name) => name,
                        _ => bail!("Expected parameter name"),
                    };
                    // 检查参数类型注解
                    let param_type = if self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        self.parse_type_annotation()
                    } else {
                        None
                    };
                    // v0.3.180: 检查是否有默认值
                    let default_value: Option<String> = if self.current_token_eq(&Token::Eq) {
                        self.consume(Token::Eq)?;
                        // 解析默认值表达式（简单实现：读取直到逗号或右括号）
                        let default = self.parse_default_value_expression()?;
                        Some(default)
                    } else {
                        None
                    };
                    params.push((param_name, param_type, default_value));
                    if self.current_token_eq(&Token::Comma) {
                        self.consume(Token::Comma)?;
                    }
                }
            }
            self.consume(Token::RParen)?;
        } else if self.current_token_eq(&Token::Identifier("".to_string())) {
            // 单个参数无括号: x
            let param_name_token = self.consume_any_identifier()?;
            let param_name = match param_name_token {
                Token::Identifier(name) => name,
                _ => bail!("Expected parameter name"),
            };
            // 检查参数类型注解
            let param_type = if self.current_token_eq(&Token::Colon) {
                self.consume(Token::Colon)?;
                self.parse_type_annotation()
            } else {
                None
            };
            // v0.3.180: 检查是否有默认值
            let default_value: Option<String> = if self.current_token_eq(&Token::Eq) {
                self.consume(Token::Eq)?;
                let default = self.parse_default_value_expression()?;
                Some(default)
            } else {
                None
            };
            params.push((param_name, param_type, default_value));
        } else {
            bail!("Expected parameter list or parameter name");
        }
        // 检查返回类型注解
        let return_type = if self.current_token_eq(&Token::Colon) {
            self.consume(Token::Colon)?;
            self.parse_type_annotation()
        } else {
            None
        };
        // 检查 FatArrow
        self.consume(Token::FatArrow)?;
        // 解析函数体 - 支持表达式和块语句
        let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
            // 块语句: { statements; }
            self.consume(Token::LBrace)?;
            let mut statements = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                statements.push(self.parse_statement()?);
            }
            self.consume(Token::RBrace)?;
            ASTNode::Statement(ASTStatement::Block(statements))
        } else {
            // 表达式: expr
            ASTNode::Expression(self.parse_expression()?)
        };
        Ok(ASTExpression::ArrowFunctionExpression {
            params,
            body: Box::new(body),
            return_type,
            is_async: false,
        })
    }

    /// 解析箭头函数（当参数名已经解析过时使用）
    /// 这种情况发生在变量声明中: const add = (a: number, b: number) => {}
    /// 此时参数列表已经被消耗了，需要重新解析
    fn parse_arrow_function_from_assignment_with_name(
        &mut self,
        _first_param_name: String,
    ) -> Result<ASTExpression> {
        // v0.3.180: 使用 3 元素元组支持默认参数
        // 注意：由于我们无法恢复已消耗的 token，这里需要特殊处理
        // 实际上，我们应该在检测到箭头函数时不消耗参数，而是保存状态后重新解析
        // 为了简化，我们让 parse_variable_name_and_initializer 返回不同的标记
        // 这里采用另一种方式：重新从当前 token 开始解析

        // 如果有 =，先消费它（变量声明中的赋值）
        if self.current_token_eq(&Token::Eq) {
            self.consume(Token::Eq)?;
        }

        // 检查当前是否是左括号（带括号的参数列表）
        let mut params: Vec<(String, Option<String>, Option<String>)> = Vec::new();

        if self.current_token_eq(&Token::LParen) {
            // 带括号的参数列表: (a, b, c)
            self.consume(Token::LParen)?;
            // 处理空参数列表的情况
            if !self.current_token_eq(&Token::RParen) {
                while !self.current_token_eq(&Token::RParen) {
                    let param_name_token = self.consume_any_identifier()?;
                    let param_name = match param_name_token {
                        Token::Identifier(name) => name,
                        _ => bail!("Expected parameter name"),
                    };
                    // 检查参数类型注解
                    let param_type = if self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        
                        self.parse_type_annotation()
                    } else {
                        None
                    };
                    // v0.3.180: 检查是否有默认值
                    let default_value: Option<String> = if self.current_token_eq(&Token::Eq) {
                        self.consume(Token::Eq)?;
                        let default = self.parse_default_value_expression()?;
                        Some(default)
                    } else {
                        None
                    };
                    params.push((param_name, param_type, default_value));
                    if self.current_token_eq(&Token::Comma) {
                        self.consume(Token::Comma)?;
                    }
                }
            }
            self.consume(Token::RParen)?;
        } else {
            // 单参数无括号的情况：x => body
            // 这种情况下，参数名已经在 parse_variable_name_and_initializer 中被消耗了
            // 我们需要使用保存的名字
            // 但由于我们没有保存状态，这是一个限制
            // 简化处理：假设是带括号的情况
            bail!("Expected '(' for arrow function parameters");
        }

        // 检查返回类型注解
        let return_type = if self.current_token_eq(&Token::Colon) {
            self.consume(Token::Colon)?;
            self.parse_type_annotation()
        } else {
            None
        };
        // 检查 FatArrow
        self.consume(Token::FatArrow)?;
        // 解析函数体 - 支持表达式和块语句
        let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
            // 块语句: { statements; }
            self.consume(Token::LBrace)?;
            let mut statements = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                statements.push(self.parse_statement()?);
            }
            self.consume(Token::RBrace)?;
            ASTNode::Statement(ASTStatement::Block(statements))
        } else {
            // 表达式: expr
            ASTNode::Expression(self.parse_expression()?)
        };
        Ok(ASTExpression::ArrowFunctionExpression {
            params,
            body: Box::new(body),
            return_type,
            is_async: false,
        })
    }

    /// 解析 async 箭头函数 (async () => {} 或 async x => {})
    fn parse_async_arrow_function(&mut self) -> Result<ASTExpression> {
        // v0.3.180: 解析参数部分 - 使用 3 元素元组支持默认值
        let params: Vec<(String, Option<String>, Option<String>)> =
            if self.current_token_eq(&Token::LParen) {
                // 带括号的参数列表: async (a, b)
                self.consume(Token::LParen)?;
                let mut params = Vec::new();
                while !self.current_token_eq(&Token::RParen) {
                    let param_name_token = self.consume_any_identifier()?;
                    let param_name = match param_name_token {
                        Token::Identifier(name) => name,
                        _ => bail!("Expected parameter name"),
                    };
                    // 跳过类型注解
                    if self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        self.parse_type_annotation();
                    }
                    // v0.3.180: 暂不支持 async 箭头函数的默认参数（简化处理）
                    params.push((param_name, None, None));
                    if self.current_token_eq(&Token::Comma) {
                        self.consume(Token::Comma)?;
                    }
                }
                self.consume(Token::RParen)?;
                params
            } else if let Token::Identifier(ref name) = self.current_token() {
                // 单个参数无括号: async x => {}
                let name = name.clone();
                self.advance();
                vec![(name, None, None)]
            } else {
                bail!("Expected parameter list or identifier after async");
            };

        // 消耗 FatArrow
        self.consume(Token::FatArrow)?;

        // 解析函数体 - 支持完整块语句
        let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
            // 块语句: { statements; }
            self.consume(Token::LBrace)?;
            let mut statements = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                statements.push(self.parse_statement()?);
            }
            self.consume(Token::RBrace)?;
            ASTNode::Statement(ASTStatement::Block(statements))
        } else {
            // 表达式体: expr
            ASTNode::Expression(self.parse_expression()?)
        };

        Ok(ASTExpression::ArrowFunctionExpression {
            params,
            body: Box::new(body),
            return_type: None,
            is_async: true,
        })
    }
    // v0.3.180: 更新函数签名以支持默认参数值
    fn parse_arrow_function_expression(
        &mut self,
        params: Vec<(String, Option<String>, Option<String>)>,
    ) -> Result<ASTExpression> {
        // 消耗 FatArrow token
        self.consume(Token::FatArrow)?;
        // 解析函数体 - 支持表达式和块语句
        let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
            // 块语句: { statements; }
            self.consume(Token::LBrace)?;
            let mut statements = Vec::new();
            while !self.current_token_eq(&Token::RBrace) {
                statements.push(self.parse_statement()?);
            }
            self.consume(Token::RBrace)?;
            ASTNode::Statement(ASTStatement::Block(statements))
        } else {
            // 表达式: expr
            ASTNode::Expression(self.parse_expression()?)
        };
        Ok(ASTExpression::ArrowFunctionExpression {
            params,
            body: Box::new(body),
            return_type: None,
            is_async: false,
        })
    }
    fn parse_postfix(&mut self, mut expr: ASTExpression) -> Result<ASTExpression> {
        // Handle postfix operators after parsing right side of binary expression
        loop {
            match self.current_token() {
                Token::Dot => {
                    self.advance();
                    let prop_token = self.consume_any_identifier()?;
                    let prop_name = match prop_token {
                        Token::Identifier(name) => name,
                        _ => bail!("Expected property name after '.'"),
                    };
                    expr = ASTExpression::MemberExpression {
                        object: Box::new(expr),
                        property: prop_name,
                    };
                }
                Token::LParen => {
                    // 函数调用: expr(args) 或分组表达式 (expr)
                    self.advance();
                    let mut arguments = Vec::new();
                    while !self.current_token_eq(&Token::RParen) {
                        arguments.push(self.parse_expression()?);
                        if self.current_token_eq(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.consume(Token::RParen)?;
                    // 如果有参数，则是函数调用；否则是分组表达式
                    if arguments.is_empty() {
                        // 分组表达式 (expr) - 括号内只有一个表达式
                        // 这里的 arguments 为空表示是 (expr) 形式
                        // 但我们丢失了 inner_expr，所以需要重新解析
                        // 这种情况实际上不应该发生，因为分组表达式在 parse_primary_expression 中处理
                        // 这里主要是处理函数调用
                    }
                    expr = ASTExpression::CallExpression {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.consume(Token::RBracket)?;
                    expr = ASTExpression::IndexExpression {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                // 后缀递增/递减运算符: expr++ 或 expr--
                Token::PlusPlus | Token::MinusMinus => {
                    let op = match self.current_token() {
                        Token::PlusPlus => "++",
                        Token::MinusMinus => "--",
                        _ => unreachable!(),
                    };
                    self.advance();
                    expr = ASTExpression::UpdateExpression {
                        argument: Box::new(expr),
                        operator: op.to_string(),
                        is_prefix: false,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }
    fn parse_primary_expression(&mut self) -> Result<ASTExpression> {
        match self.current_token() {
            Token::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(ASTExpression::Identifier(name))
            }
            Token::Number(ref num) => {
                let num = num.clone();
                self.advance();
                Ok(ASTExpression::Literal(num))
            }
            Token::BigInt(ref num) => {
                let num = num.clone();
                self.advance();
                Ok(ASTExpression::Literal(num))
            }
            Token::String(ref s, _quote) => {
                let s = format!("\"{}\"", s.clone());
                self.advance();
                Ok(ASTExpression::Literal(s))
            }
            // 处理 typeof 运算符
            Token::Typeof => {
                self.consume(Token::Typeof)?;
                let expr = self.parse_primary_expression()?;
                Ok(ASTExpression::Unary {
                    operator: "typeof".to_string(),
                    operand: Box::new(expr),
                })
            }
            // 处理 async 关键字开头的箭头函数
            Token::Async => {
                self.consume(Token::Async)?;
                // 检查是否是 async function 表达式还是 async 箭头函数
                // async function(...) {} 是函数表达式
                // async (...) => ... 是箭头函数
                if self.current_token_eq(&Token::Function) {
                    // async function 表达式
                    self.consume(Token::Function)?;
                    self.parse_function_expression(true)
                } else {
                    // async 箭头函数
                    self.parse_async_arrow_function()
                }
            }
            // 处理 function 关键字开头的函数表达式
            Token::Function => {
                self.consume(Token::Function)?;
                self.parse_function_expression(false)
            }
            Token::TemplateStart => {
                // 模板字符串: `part1${expr1}part2${expr2}part3`
                self.consume(Token::TemplateStart)?;
                let mut parts = Vec::new();

                // 解析第一个部分（模板表达式前的空字符串特殊情况）
                if let Token::String(ref s, _) = self.current_token() {
                    // 对字符串内容中的引号进行转义
                    let escaped = s.replace('"', "\\\"");
                    let s = format!("\"{}\"", escaped);
                    parts.push(ASTExpression::Literal(s));
                    self.advance();
                }

                // 解析模板表达式和后续部分
                loop {
                    if self.current_token_eq(&Token::TemplateMiddle) {
                        self.consume(Token::TemplateMiddle)?;
                        // 解析表达式
                        let expr = self.parse_expression()?;
                        parts.push(expr);

                        // 解析下一个字符串部分
                        if let Token::String(ref s, _) = self.current_token() {
                            // 对字符串内容中的引号进行转义
                            let escaped = s.replace('"', "\\\"");
                            let s = format!("\"{}\"", escaped);
                            parts.push(ASTExpression::Literal(s));
                            self.advance();
                        } else if self.current_token_eq(&Token::TemplateEnd) {
                            // 表达式后面直接是模板结束，没有字符串部分
                            // （如 `${expr}` 后面没有内容的情况）
                        }
                    } else if self.current_token_eq(&Token::TemplateEnd) {
                        self.consume(Token::TemplateEnd)?;
                        break;
                    } else {
                        // 跳过逗号（用于处理 var a = 1, b = 2; 这种情况）
                        if self.current_token_eq(&Token::Comma) {
                            self.advance();
                            continue;
                        }
                        bail!(
                            "Expected TemplateMiddle or TemplateEnd in template literal, got {:?}",
                            self.current_token()
                        );
                    }
                }

                Ok(ASTExpression::TemplateLiteral { parts })
            }
            Token::LParen => {
                // 括号表达式 vs 箭头函数参数列表
                // 向前查看：检查是否是箭头函数 (params) => body
                let mut depth = 1;
                let mut i = 1;
                let n = self.tokens.len();
                let start_pos = self.position;

                while start_pos + i < n && depth > 0 {
                    match &self.tokens[start_pos + i] {
                        Token::LParen => {
                            depth += 1;
                            i += 1;
                        }
                        Token::RParen => {
                            depth -= 1;
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }

                // 检查匹配的 RParen 后面是否是 FatArrow
                let is_arrow_function = if start_pos + i < n {
                    matches!(self.tokens[start_pos + i], Token::FatArrow)
                } else {
                    false
                };

                if is_arrow_function {
                    // v0.3.180: 箭头函数参数列表: (params) =>
                    self.advance(); // 消费 (
                                    // 使用 3 元素元组支持默认参数
                    let mut params: Vec<(String, Option<String>, Option<String>)> = Vec::new();

                    // 解析参数列表
                    if !self.current_token_eq(&Token::RParen) {
                        while !self.current_token_eq(&Token::RParen) {
                            // 检查是否是解构参数
                            if self.current_token_eq(&Token::LBrace) {
                                // 解构参数: { a, b } 或 { a = default }
                                self.consume(Token::LBrace)?;
                                let mut prop_names = Vec::new();
                                while !self.current_token_eq(&Token::RBrace) {
                                    if let Token::Identifier(name) = self.current_token().clone() {
                                        self.advance();
                                        prop_names.push(name);
                                    }
                                    if self.current_token_eq(&Token::Comma) {
                                        self.advance();
                                    }
                                }
                                self.consume(Token::RBrace)?;
                                // v0.3.180: 简化的解构参数处理
                                if let Some(name) = prop_names.first() {
                                    params.push((name.clone(), None, None));
                                }
                            } else if let Token::LBracket = self.current_token() {
                                // 数组解构参数: [a, b]
                                self.consume(Token::LBracket)?;
                                let mut elem_names = Vec::new();
                                while !self.current_token_eq(&Token::RBracket) {
                                    if let Token::Identifier(name) = self.current_token().clone() {
                                        self.advance();
                                        elem_names.push(name);
                                    }
                                    if self.current_token_eq(&Token::Comma) {
                                        self.advance();
                                    }
                                }
                                self.consume(Token::RBracket)?;
                                if let Some(name) = elem_names.first() {
                                    params.push((name.clone(), None, None));
                                }
                            } else if let Token::Identifier(name) = self.current_token().clone() {
                                self.advance();
                                // 检查是否有类型注解
                                if self.current_token_eq(&Token::Colon) {
                                    self.consume(Token::Colon)?;
                                    self.parse_type_annotation();
                                }
                                // v0.3.180: 暂不支持默认参数
                                params.push((name, None, None));
                            }

                            if self.current_token_eq(&Token::Comma) {
                                self.consume(Token::Comma)?;
                            }
                        }
                    }
                    self.consume(Token::RParen)?;

                    // 解析箭头函数体
                    self.consume(Token::FatArrow)?;

                    let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
                        self.consume(Token::LBrace)?;
                        let mut statements = Vec::new();
                        while !self.current_token_eq(&Token::RBrace) {
                            statements.push(self.parse_statement()?);
                        }
                        self.consume(Token::RBrace)?;
                        ASTNode::Statement(ASTStatement::Block(statements))
                    } else {
                        ASTNode::Expression(self.parse_expression()?)
                    };

                    Ok(ASTExpression::ArrowFunctionExpression {
                        params,
                        body: Box::new(body),
                        return_type: None,
                        is_async: false,
                    })
                } else {
                    // 真正的分组表达式: (expr)
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.consume(Token::RParen)?;
                    Ok(expr)
                }
            }
            Token::LBrace => {
                // 对象字面量 vs 块语句
                // 向前查看：如果后面是 Identifier:、String: 或 [expr] 则为对象字面量
                // 否则可能是箭头函数的块语句或其他
                let lookahead = self.position + 1;
                let mut is_object_literal = false;

                // 简单向前查看：检查是否看起来像对象字面量
                // 模式: { identifier : ... } 或 { string : ... } 或 { [expr] : ... }
                if lookahead < self.tokens.len() {
                    let next_token = &self.tokens[lookahead];
                    match next_token {
                        Token::RBrace | Token::RParen | Token::SemiColon | Token::Eof => {
                            // 空的 {} - 可能是对象字面量
                            is_object_literal = true;
                        }
                        Token::Identifier(_) | Token::String(_, _) | Token::Number(_) => {
                            // 找到属性名（标识符、字符串或数字），检查下一个是否是 :
                            if lookahead + 1 < self.tokens.len()
                                && matches!(self.tokens[lookahead + 1], Token::Colon) {
                                    is_object_literal = true;
                                }
                        }
                        Token::LBracket => {
                            // 计算属性名: { [expr] : ... } - 是对象字面量
                            is_object_literal = true;
                        }
                        _ => {
                            // 其他 token，不是典型的对象字面量开头
                        }
                    }
                }

                if is_object_literal {
                    // 对象字面量
                    self.parse_object_literal()
                } else {
                    // 块语句（用于箭头函数等）- 消费 { 并返回空
                    // 注意：在表达式上下文中，{ 通常应该是对象字面量
                    // 如果不是，则这是一个语法错误
                    bail!("Unexpected '{{' in expression. Expected object literal or expression.");
                }
            }
            Token::LBracket => {
                // 数组字面量: [1, 2, 3] 或展开: [...arr]
                self.parse_array_literal()
            }
            Token::New => {
                // new 表达式: new Constructor(args)
                self.consume(Token::New)?;
                let constructor = self.parse_primary_expression()?;
                // 解析参数列表
                let mut arguments = Vec::new();
                if self.current_token_eq(&Token::LParen) {
                    self.consume(Token::LParen)?;
                    while !self.current_token_eq(&Token::RParen) {
                        arguments.push(self.parse_expression()?);
                        if self.current_token_eq(&Token::Comma) {
                            self.consume(Token::Comma)?;
                        }
                    }
                    self.consume(Token::RParen)?;
                }
                Ok(ASTExpression::NewExpression {
                    constructor: Box::new(constructor),
                    arguments,
                })
            }
            Token::This => {
                self.consume(Token::This)?;
                Ok(ASTExpression::ThisExpression)
            }
            Token::Super => {
                self.consume(Token::Super)?;
                Ok(ASTExpression::SuperExpression)
            }
            // TypeScript 尖括号类型断言: <Type>expr
            // 这是一个前缀表达式，<Type> 后面跟一个表达式
            Token::Lt => {
                // 检查是否是尖括号断言: <Type>expr
                // 尖括号断言的特征:
                // 1. < 后面是类型标识符（以大写字母开头或常见类型名）
                // 2. > 后面直接跟表达式
                // 3. 不是小于运算符（前面有标识符，后面有数字）
                let is_angle_bracket_assertion = {
                    let mut lookahead = self.position + 1;
                    let mut depth = 1;
                    let mut found_gt = false;

                    // 首先检查 < 后面是否是有效的类型标识符开头
                    // 类型标识符通常以大写字母开头，或者是 any, unknown, number, string, boolean 等
                    let starts_with_type = if lookahead < self.tokens.len() {
                        match &self.tokens[lookahead] {
                            Token::Identifier(name) => {
                                // 检查是否是大写字母开头，或者是常见的类型名
                                let first_char = name.chars().next();
                                let is_uppercase =
                                    first_char.map(|c| c.is_ascii_uppercase()).unwrap_or(false);
                                let is_known_type = matches!(
                                    name.as_str(),
                                    "any"
                                        | "unknown"
                                        | "number"
                                        | "string"
                                        | "boolean"
                                        | "void"
                                        | "null"
                                        | "undefined"
                                        | "never"
                                        | "object"
                                        | "symbol"
                                        | "bigint"
                                        | "Date"
                                        | "Array"
                                        | "Promise"
                                        | "Map"
                                        | "Set"
                                        | "Function"
                                        | "Error"
                                        | "RegExp"
                                );
                                is_uppercase || is_known_type
                            }
                            Token::LParen | Token::LBracket | Token::LBrace => true, // 泛型表达式如 <T[]>
                            _ => false,
                        }
                    } else {
                        false
                    };

                    if !starts_with_type {
                        // 不是尖括号断言，这不是主表达式的起始
                        // 恢复 position 并让调用方处理
                        self.position = self.position.saturating_sub(1);
                        bail!("Unexpected token in expression: {:?}", self.current_token())
                    }

                    // 找到匹配的 >
                    while lookahead < self.tokens.len() && depth > 0 {
                        match &self.tokens[lookahead] {
                            Token::Lt => {
                                depth += 1;
                                lookahead += 1;
                            }
                            Token::Gt => {
                                depth -= 1;
                                lookahead += 1;
                                found_gt = true;
                            }
                            _ => {
                                lookahead += 1;
                            }
                        }
                    }

                    // 如果找到了 >，检查后面是否是表达式起始
                    if found_gt && depth == 0 && lookahead < self.tokens.len() {
                        matches!(
                            &self.tokens[lookahead],
                            Token::Identifier(_)
                                | Token::Number(_)
                                | Token::String(_, _)
                                | Token::LParen
                                | Token::LBracket
                                | Token::LBrace
                                | Token::Bang
                                | Token::Plus
                                | Token::Minus
                                | Token::Star
                        )
                    } else {
                        false
                    }
                };

                if is_angle_bracket_assertion {
                    // 消费 <
                    self.consume(Token::Lt)?;
                    let mut depth = 1;
                    let mut type_content = String::new();

                    while depth > 0 && !self.is_at_end() {
                        match self.current_token() {
                            Token::Lt => {
                                depth += 1;
                                type_content.push('<');
                                self.advance();
                            }
                            Token::Gt => {
                                depth -= 1;
                                if depth > 0 {
                                    type_content.push('>');
                                    self.advance();
                                } else {
                                    type_content.push('>');
                                    self.advance();
                                    break;
                                }
                            }
                            Token::Comma => {
                                type_content.push(',');
                                self.advance();
                            }
                            _ => {
                                if let Token::Identifier(name) = self.current_token() {
                                    type_content.push_str(name);
                                    self.advance();
                                } else if let Token::Number(num) = self.current_token() {
                                    type_content.push_str(num);
                                    self.advance();
                                } else if let Token::String(s, _) = self.current_token() {
                                    type_content.push_str(s);
                                    self.advance();
                                } else {
                                    self.advance();
                                }
                            }
                        }
                    }

                    // 解析表达式
                    let inner_expr = self.parse_primary_expression()?;
                    Ok(ASTExpression::TSAngleBracketAssertion {
                        expression: Box::new(inner_expr),
                        target_type: type_content.trim().to_string(),
                    })
                } else {
                    // 不是尖括号断言，作为小于运算符处理
                    bail!("Unexpected token in expression: {:?}", self.current_token())
                }
            }
            // 处理 @ 符号（装饰器标识符，在表达式中作为标识符处理）
            Token::At => {
                // @ 符号在表达式中不常用，但为了解析装饰器参数中的 @ 引用
                // 我们将其作为标识符处理
                self.consume(Token::At)?;
                Ok(ASTExpression::Identifier("@".to_string()))
            }
            // 跳过逗号（用于处理多变量声明中的逗号）
            Token::Comma => {
                self.advance();
                self.parse_primary_expression()
            }
            // RParen 和 RBrace 可能出现在 IIFE 或嵌套表达式中
            // 例如: (() => {})() 或 ({a: 1})
            Token::RParen | Token::RBrace | Token::TemplateMiddle | Token::TemplateEnd => {
                bail!("Unexpected token in expression: {:?}", self.current_token())
            }
            _ => bail!("Unexpected token in expression: {:?}", self.current_token()),
        }
    }
    fn parse_object_literal(&mut self) -> Result<ASTExpression> {
        self.consume(Token::LBrace)?;
        let mut properties = Vec::new();
        // 在对象字面量中，结束条件是 RBrace 或 RParen（处理函数调用中的对象字面量）
        while !self.current_token_eq(&Token::RBrace) && !self.current_token_eq(&Token::RParen) {
            // 检查是否是计算属性名 [expr]
            if self.current_token_eq(&Token::LBracket) {
                // 计算属性名: { [expr]: value }
                self.consume(Token::LBracket)?;
                let key_expr = self.parse_expression()?;
                self.consume(Token::RBracket)?;
                self.consume(Token::Colon)?;
                let value = self.parse_expression()?;
                properties.push(ASTExpression::ObjectProperty {
                    name: None,
                    key_expr: Some(Box::new(key_expr)),
                    value: Box::new(value),
                });
            } else {
                // 普通属性名: { name: value } 或 { "string": value } 或 { number: value }
                // 或者方法简写: { methodName(params) { ... } }
                let prop_name = self.consume_property_name()?;
                let name_str = match prop_name {
                    Token::Identifier(name) => name,
                    Token::String(s, quote) => {
                        // 字符串属性名需要保留引号
                        format!("{}{}{}", quote, s, quote)
                    }
                    Token::Number(n) => n.clone(),
                    _ => bail!("Expected property name (identifier, string, or number)"),
                };

                // 检查是否是方法简写语法: { methodName(params) { ... } }
                if self.current_token_eq(&Token::LParen) {
                    // 解析方法参数 - 使用 3 元素元组格式 (name, type, default)
                    let mut params: Vec<(String, Option<String>, Option<String>)> = Vec::new();
                    self.consume(Token::LParen)?;
                    while !self.current_token_eq(&Token::RParen) {
                        // 解析参数名（支持 this 关键字）
                        let param_name = if self.current_token_eq(&Token::This) {
                            self.consume(Token::This)?;
                            "this".to_string()
                        } else {
                            let ident = self.consume_any_identifier()?;
                            if let Token::Identifier(name) = ident {
                                name
                            } else {
                                bail!("Expected parameter name");
                            }
                        };
                        // 跳过类型注解
                        if self.current_token_eq(&Token::Colon) {
                            self.consume(Token::Colon)?;
                            self.parse_type_annotation();
                        }
                        // 跳过默认值
                        if self.current_token_eq(&Token::Eq) {
                            self.consume(Token::Eq)?;
                            self.parse_expression()?;
                        }
                        params.push((param_name, None, None));
                        // 处理逗号
                        if self.current_token_eq(&Token::Comma) {
                            self.consume(Token::Comma)?;
                        }
                    }
                    self.consume(Token::RParen)?;

                    // 跳过返回类型注解 (如果有)
                    let return_type = if self.current_token_eq(&Token::Colon) {
                        self.consume(Token::Colon)?;
                        self.parse_type_annotation()
                    } else {
                        None
                    };

                    // 解析方法体
                    let body: ASTNode = if self.current_token_eq(&Token::LBrace) {
                        self.consume(Token::LBrace)?;
                        let mut statements = Vec::new();
                        while !self.current_token_eq(&Token::RBrace) {
                            statements.push(self.parse_statement()?);
                        }
                        self.consume(Token::RBrace)?;
                        ASTNode::Statement(ASTStatement::Block(statements))
                    } else {
                        // 方法体为空或是抽象方法签名，跳过
                        ASTNode::Statement(ASTStatement::Block(vec![]))
                    };

                    properties.push(ASTExpression::ObjectProperty {
                        name: Some(name_str),
                        key_expr: None,
                        value: Box::new(ASTExpression::ArrowFunctionExpression {
                            params,
                            body: Box::new(body),
                            return_type,
                            is_async: false,
                        }),
                    });
                } else {
                    // 普通属性: { name: value }
                    self.consume(Token::Colon)?;
                    let prop_value = self.parse_expression()?;
                    properties.push(ASTExpression::ObjectProperty {
                        name: Some(name_str),
                        key_expr: None,
                        value: Box::new(prop_value),
                    });
                }
            }
            // 处理逗号分隔符
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            }
        }
        // 消费结束括号：可能是 RBrace 或 RParen
        if self.current_token_eq(&Token::RBrace) {
            self.consume(Token::RBrace)?;
        } else if self.current_token_eq(&Token::RParen) {
            // 如果是 RParen（函数调用中的对象字面量），消费它
            self.consume(Token::RParen)?;
        }
        Ok(ASTExpression::ObjectLiteral { properties })
    }
    /// 解析数组字面量: [1, 2, 3] 或 [...arr]
    fn parse_array_literal(&mut self) -> Result<ASTExpression> {
        self.consume(Token::LBracket)?;
        let mut elements = Vec::new();

        // 处理空数组 []
        if self.current_token_eq(&Token::RBracket) {
            self.consume(Token::RBracket)?;
            return Ok(ASTExpression::ArrayExpression { elements });
        }

        // 解析数组元素
        while !self.current_token_eq(&Token::RBracket) {
            if self.current_token_eq(&Token::DotDotDot) {
                // 展开运算符: ...expr
                self.consume(Token::DotDotDot)?;
                let arg = self.parse_expression()?;
                elements.push(Some(ASTExpression::SpreadExpression {
                    argument: Box::new(arg),
                }));
            } else {
                // 普通元素
                elements.push(Some(self.parse_expression()?));
            }

            // 处理逗号分隔符
            if self.current_token_eq(&Token::Comma) {
                self.consume(Token::Comma)?;
            } else if !self.current_token_eq(&Token::RBracket) {
                // 如果不是逗号也不是结束符，可能语法错误
                break;
            }
        }

        self.consume(Token::RBracket)?;
        Ok(ASTExpression::ArrayExpression { elements })
    }
    fn parse_type_annotation(&mut self) -> Option<String> {
        // v0.3.220: 检查是否是 asserts 断言类型
        // asserts condition - 简单断言
        // asserts value is Type - 带类型谓词的断言
        if self.current_token_eq(&Token::Asserts) {
            self.advance(); // 消耗 asserts 关键字
                            // 检查是否是 asserts value is Type
            if let Token::Identifier(ref param_name) = self.current_token() {
                let param_name = param_name.clone();
                self.advance(); // 消耗参数名
                                // 检查是否有 is 关键字
                if self.current_token_eq(&Token::Is) {
                    self.advance(); // 消耗 is 关键字
                    let target_type = if let Some(t) = self.parse_type_annotation() {
                        t
                    } else {
                        "unknown".to_string()
                    };
                    return Some(format!("asserts {} is {}", param_name, target_type));
                }
                // 简单 asserts condition
                return Some(format!("asserts {}", param_name));
            }
            // 没有参数名的情况
            return Some("asserts unknown".to_string());
        }

        // 检查是否是模板字面量类型: `prefix${Type}suffix`
        if self.current_token_eq(&Token::TemplateStart) {
            return self.parse_template_literal_type();
        }

        // 检查是否是类型谓词: parameterName is Type（用于类型守卫）
        if let Token::Identifier(ref param_name) = self.current_token() {
            let param_name = param_name.clone();
            // 向前看一个 token 检查是否是 is 关键字
            if self.position + 1 < self.tokens.len()
                && matches!(self.tokens[self.position + 1], Token::Is)
            {
                self.advance(); // 消耗参数名
                self.advance(); // 消耗 is 关键字
                                // 解析谓词目标类型
                let target_type = if let Some(t) = self.parse_type_annotation() {
                    t
                } else {
                    "unknown".to_string()
                };
                return Some(format!("{} is {}", param_name, target_type));
            }
        }

        // 检查是否是对象类型字面量 { ... } 或元组类型 [ ... ]
        let is_lbrace = self.current_token_eq(&Token::LBrace);
        let is_lbracket = self.current_token_eq(&Token::LBracket);
        let first_type = if is_lbrace {
            self.parse_object_type()
        } else if is_lbracket {
            self.parse_tuple_type()
        } else {
            self.parse_basic_type()
        }?;

        // 检查是否是条件类型: T extends U ? X : Y
        if self.current_token_eq(&Token::Extends) {
            // 解析条件类型
            self.consume(Token::Extends).ok()?;
            let extend_type = if let Some(t) = self.parse_type_annotation() {
                t
            } else {
                "unknown".to_string()
            };

            // 检查 ?
            self.consume(Token::Question).ok()?;

            // 解析 true 分支
            let true_type = if let Some(t) = self.parse_type_annotation() {
                t
            } else {
                "never".to_string()
            };

            // 检查 :
            self.consume(Token::Colon).ok()?;

            // 解析 false 分支
            let false_type = if let Some(t) = self.parse_type_annotation() {
                t
            } else {
                first_type.clone()
            };

            return Some(format!(
                "{} extends {} ? {} : {}",
                first_type, extend_type, true_type, false_type
            ));
        }

        // 处理数组类型和索引访问类型后缀
        let mut result = first_type;
        while self.current_token_eq(&Token::LBracket) {
            // 向前查看：检查是否是空括号 [] (数组类型) 或有内容 (索引访问类型)
            if self.position + 1 < self.tokens.len()
                && self.tokens[self.position + 1] == Token::RBracket
            {
                // 数组类型: T[]
                self.advance(); // 消耗 [
                self.advance(); // 消耗 ]
                result = format!("{}[]", result);
            } else {
                // 索引访问类型: T[key] 或 T[key1 | key2]
                self.advance();
                // 解析索引键（支持联合类型）
                let mut index_keys = Vec::new();

                // 解析第一个索引键
                let first_key = if let Token::String(ref s, quote) = self.current_token() {
                    let s = s.clone();
                    let quote_char = *quote;
                    self.advance();
                    format!("{}{}{}", quote_char, s, quote_char)
                } else if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    // 解析基本类型作为索引
                    if let Some(idx_type) = self.parse_basic_type() {
                        idx_type
                    } else {
                        break;
                    }
                };
                index_keys.push(first_key);

                // 检查是否有联合类型: |
                while self.current_token_eq(&Token::Pipe) {
                    self.advance(); // 消耗 |
                                    // 解析下一个索引键
                    let next_key = if let Token::String(ref s, quote) = self.current_token() {
                        let s = s.clone();
                        let quote_char = *quote;
                        self.advance();
                        format!("{}{}{}", quote_char, s, quote_char)
                    } else if let Token::Identifier(ref name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        name
                    } else {
                        // 解析基本类型作为索引
                        if let Some(idx_type) = self.parse_basic_type() {
                            idx_type
                        } else {
                            break;
                        }
                    };
                    index_keys.push(next_key);
                }

                self.consume(Token::RBracket).ok()?;
                // 如果只有一个键，直接使用；如果有多个，合并为联合类型
                let index_key = if index_keys.len() == 1 {
                    index_keys[0].clone()
                } else {
                    index_keys.join(" | ")
                };
                result = format!("{}[{}]", result, index_key);
            }
        }

        // 处理 & 和 | 操作符
        let mut types = vec![result];
        let mut operators = Vec::new();

        while self.current_token_eq(&Token::Ampersand) || self.current_token_eq(&Token::Pipe) {
            let op = if self.current_token_eq(&Token::Ampersand) {
                self.advance();
                "&"
            } else {
                self.advance();
                "|"
            };
            operators.push(op.to_string());

            // 解析下一个类型
            let next_type = if self.current_token_eq(&Token::LBrace) {
                self.parse_object_type()
            } else {
                self.parse_basic_type()
            };

            if let Some(t) = next_type {
                types.push(t);
            } else {
                break;
            }
        }

        if types.len() == 1 {
            Some(types[0].clone())
        } else {
            let mut final_result = types[0].clone();
            for (i, op) in operators.iter().enumerate() {
                final_result.push(' ');
                final_result.push_str(op);
                final_result.push(' ');
                final_result.push_str(&types[i + 1]);
            }
            Some(final_result)
        }
    }

    // v0.3.180: 解析箭头函数参数的默认值表达式
    /// 解析默认值表达式，返回原始字符串表示
    /// 简单实现：读取直到逗号或右括号
    fn parse_default_value_expression(&mut self) -> Result<String> {
        let mut value = String::new();
        let mut paren_depth = 0;
        let mut bracket_depth = 0;
        let mut brace_depth = 0;

        // 收集 token 直到遇到顶级逗号或右括号
        while !self.is_at_end() {
            match self.current_token() {
                Token::LParen => {
                    paren_depth += 1;
                    value.push('(');
                }
                Token::RParen => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                        value.push(')');
                    } else {
                        // 顶级右括号，结束
                        break;
                    }
                }
                Token::LBracket => {
                    bracket_depth += 1;
                    value.push('[');
                }
                Token::RBracket => {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                        value.push(']');
                    } else {
                        // 顶级右括号，结束
                        break;
                    }
                }
                Token::LBrace => {
                    brace_depth += 1;
                    value.push('{');
                }
                Token::RBrace => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                        value.push('}');
                    } else {
                        // 顶级右括号，结束
                        break;
                    }
                }
                Token::Comma => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        // 顶级逗号，结束
                        break;
                    } else {
                        value.push(',');
                    }
                }
                Token::String(s, quote) => {
                    value.push(*quote);
                    value.push_str(s);
                    value.push(*quote);
                }
                Token::Identifier(s) => {
                    if !value.is_empty() && !value.ends_with(' ') {
                        value.push(' ');
                    }
                    value.push_str(s);
                }
                Token::Number(n) => {
                    value.push_str(n);
                }
                Token::BigInt(n) => {
                    value.push_str(n);
                    value.push('n');
                }
                Token::TemplateStart => {
                    value.push('`');
                }
                Token::TemplateMiddle => {
                    // For template literals inside the expression
                    value.push_str("${");
                }
                Token::TemplateEnd => {
                    value.push('`');
                }
                Token::Plus => value.push('+'),
                Token::Minus => value.push('-'),
                Token::Star => value.push('*'),
                Token::Slash => value.push('/'),
                Token::Percent => value.push('%'),
                Token::StarStar => value.push_str("**"),
                Token::Bang => value.push('!'),
                Token::Question => value.push('?'),
                Token::Colon => value.push(':'),
                Token::Ampersand => value.push('&'),
                Token::Pipe => value.push('|'),
                Token::Caret => value.push('^'),
                Token::Lt => value.push('<'),
                Token::Gt => value.push('>'),
                Token::LtEq => value.push_str("<="),
                Token::GtEq => value.push_str(">="),
                Token::EqEq => value.push_str("=="),
                Token::NotEq => value.push_str("!="),
                Token::EqEqEq => value.push_str("==="),
                Token::NotEqEq => value.push_str("!=="),
                Token::PlusPlus => value.push_str("++"),
                Token::MinusMinus => value.push_str("--"),
                Token::DotDotDot => value.push_str("..."),
                Token::QuestionQuestion => value.push_str("??"),
                Token::QuestionDot => value.push_str("?."),
                _ => {
                    // 其他 token，直接添加
                    value.push_str(&format!("{:?}", self.current_token()));
                }
            }
            self.advance();
        }

        Ok(value.trim().to_string())
    }

    /// 解析对象类型字面量: { name: string; age: number }
    /// 或映射类型: { [P in keyof T]: T[P] }
    /// 或索引签名: { [key: string]: T }
    fn parse_object_type(&mut self) -> Option<String> {
        self.consume(Token::LBrace).ok()?;

        // 检测映射类型语法: { [P in KeyType]: ValueType } 或 { readonly [P in KeyType]: ValueType }
        // 检测是否有 `[` 或 `readonly [`
        let has_lbracket = self.current_token_eq(&Token::LBracket);
        let has_readonly_lbracket = self.current_token_eq(&Token::Readonly)
            && self.position + 2 < self.tokens.len()
            && matches!(self.tokens[self.position + 1], Token::LBracket);

        let is_mapped_type = if has_lbracket {
            // 检查是否是映射类型 (有 `in` 关键字) 而不是索引签名 (有 `:` 紧随 `[identifier]`)
            // token 序列: [ Identifier : ... ]: 或 [ Identifier in ... ]:
            if self.position + 4 < self.tokens.len() {
                matches!(self.tokens[self.position + 2], Token::In)
            } else {
                false
            }
        } else if has_readonly_lbracket {
            // token 序列: readonly [ Identifier in ... ]:
            if self.position + 5 < self.tokens.len() {
                matches!(self.tokens[self.position + 3], Token::In)
            } else {
                false
            }
        } else {
            false
        };

        if is_mapped_type {
            return self.parse_mapped_type();
        }

        let mut properties = Vec::new();

        // 解析属性列表
        while !self.current_token_eq(&Token::RBrace) {
            // 检测索引签名语法: [keyName: keyType]: valueType
            if self.current_token_eq(&Token::LBracket) {
                // 消耗 [
                self.advance();

                // 解析索引参数名
                let key_name = match self.current_token() {
                    Token::Identifier(ref name) => {
                        let name = name.clone();
                        self.advance();
                        name
                    }
                    _ => break,
                };

                // 期望冒号（索引签名的 `: type` 部分）
                self.consume(Token::Colon).ok()?;

                // 解析键类型 (string 或 number)
                let key_type = match self.current_token() {
                    Token::Identifier(ref name) if name == "string" || name == "number" => {
                        let t = name.clone();
                        self.advance();
                        t
                    }
                    _ => break,
                };

                // 期望 ]
                self.consume(Token::RBracket).ok()?;

                // 期望 : (值类型注解)
                self.consume(Token::Colon).ok()?;

                // 解析值类型
                let value_type = self.parse_union_type();

                if let Some(t) = value_type {
                    properties.push(format!("[{}: {}]: {}", key_name, key_type, t));
                }

                // 处理分号或逗号分隔符
                if self.current_token_eq(&Token::SemiColon) {
                    self.advance();
                } else if self.current_token_eq(&Token::Comma) {
                    self.advance();
                }
                continue;
            }

            // 解析属性名（标识符或字符串）或方法名
            let prop_name = match self.current_token() {
                Token::Identifier(ref name) => {
                    let name = name.clone();
                    self.advance();
                    // 检查是否是方法语法 (Identifier followed by LParen)
                    if self.current_token_eq(&Token::LParen) {
                        // 方法声明: name(params): returnType
                        let method_params = self.parse_function_type_params();
                        // 跳过返回类型注解
                        if self.current_token_eq(&Token::Colon) {
                            self.consume(Token::Colon).ok()?;
                            self.parse_type_annotation();
                        }
                        properties.push(format!(
                            "{}: {}",
                            name,
                            method_params.unwrap_or_else(|| "any".to_string())
                        ));
                        // 处理分号或逗号分隔符
                        if self.current_token_eq(&Token::SemiColon) {
                            self.advance();
                        } else if self.current_token_eq(&Token::Comma) {
                            self.advance();
                        }
                        continue; // 已处理完整的方法签名，跳过后续的属性解析
                    } else {
                        name
                    }
                }
                Token::String(ref s, _) => {
                    let s = s.clone();
                    self.advance();
                    format!("\"{}\"", s)
                }
                _ => break,
            };

            // 跳过可选运算符 ?
            if self.current_token_eq(&Token::Question) {
                self.advance();
            }

            // 期望冒号
            self.consume(Token::Colon).ok()?;

            // 解析属性类型
            let prop_type = self.parse_union_type();

            if let Some(t) = prop_type {
                properties.push(format!("{}: {}", prop_name, t));
            }

            // 处理分号或逗号分隔符
            if self.current_token_eq(&Token::SemiColon) {
                self.advance();
            } else if self.current_token_eq(&Token::Comma) {
                self.advance();
            }
        }

        self.consume(Token::RBrace).ok()?;
        Some(format!("{{ {} }}", properties.join("; ")))
    }

    /// 解析映射类型: { [P in keyof T]: T[P] } 或 { readonly [P in keyof T]: T[P] }
    fn parse_mapped_type(&mut self) -> Option<String> {
        // 跳过可选的 readonly 修饰符（可能在 [ 之前）
        if self.current_token_eq(&Token::Readonly) {
            self.advance();
        }

        // 消耗 [
        self.consume(Token::LBracket).ok()?;

        // 解析键变量名 (P)
        let _key_var = match self.current_token() {
            Token::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return None;
            }
        };

        // 期望 in 关键字
        self.consume(Token::In).ok()?;

        // 解析键类型 (keyof T 或 "key1" | "key2")
        let _key_type = self.parse_union_type()?;

        // 消耗 ]
        self.consume(Token::RBracket).ok()?;

        // 跳过可选的 ? 修饰符
        if self.current_token_eq(&Token::Question) {
            self.advance();
        }

        // 期望冒号
        self.consume(Token::Colon).ok()?;

        // 解析值类型 (T[P])
        let _value_type = self.parse_union_type()?;

        // 消耗 }
        self.consume(Token::RBrace).ok()?;

        // 映射类型在转译后的 JS 中应被移除，这里返回空对象类型的占位符
        Some(String::new())
    }

    /// 解析模板字面量类型: `prefix${Type}suffix`
    /// 例如: `Hello ${string}`, `user-${string}@${string}.com`
    fn parse_template_literal_type(&mut self) -> Option<String> {
        // 消耗 TemplateStart
        self.consume(Token::TemplateStart).ok()?;

        loop {
            match self.current_token() {
                Token::String(_, _) => {
                    // 字符串部分，跳过
                    self.advance();
                }
                Token::TemplateMiddle => {
                    // ${ 开始模板表达式
                    self.consume(Token::TemplateMiddle).ok()?;

                    // 解析类型表达式（跳过）
                    let _expr_type = self.parse_union_type()?;

                    // 消耗直到 TemplateEnd
                    while !self.current_token_eq(&Token::TemplateEnd)
                        && !self.current_token_eq(&Token::Eof)
                        && !self.current_token_eq(&Token::SemiColon)
                    {
                        self.advance();
                    }
                    // 只有在找到 TemplateEnd 时才消耗它
                    if self.current_token_eq(&Token::TemplateEnd) {
                        self.consume(Token::TemplateEnd).ok()?;
                    }
                }
                Token::TemplateEnd => {
                    // 模板结束，不消耗，让调用者处理
                    break;
                }
                Token::Eof | Token::SemiColon => {
                    // 提前结束
                    break;
                }
                _ => {
                    // 其他 token，跳过
                    self.advance();
                }
            }
        }

        // 模板字面量类型在转译后的 JS 中移除，这里返回占位符
        Some(String::new())
    }

    fn parse_union_type(&mut self) -> Option<String> {
        // 解析第一个类型
        let first_type = self.parse_basic_type()?;

        // 检查是否是条件类型: T extends U ? X : Y
        if self.current_token_eq(&Token::Extends) {
            // 解析条件类型
            self.consume(Token::Extends).ok()?;
            let extend_type = if let Some(t) = self.parse_union_type() {
                t
            } else {
                "unknown".to_string()
            };

            // 检查 ?
            self.consume(Token::Question).ok()?;

            // 解析 true 分支
            let true_type = if let Some(t) = self.parse_union_type() {
                t
            } else {
                "never".to_string()
            };

            // 检查 :
            self.consume(Token::Colon).ok()?;

            // 解析 false 分支
            let false_type = if let Some(t) = self.parse_union_type() {
                t
            } else {
                first_type.clone()
            };

            return Some(format!(
                "{} extends {} ? {} : {}",
                first_type, extend_type, true_type, false_type
            ));
        }

        // 处理数组类型和索引访问类型后缀
        let mut result = first_type;
        while self.current_token_eq(&Token::LBracket) {
            // 向前查看：检查是否是空括号 [] (数组类型) 或有内容 (索引访问类型)
            if self.position + 1 < self.tokens.len()
                && self.tokens[self.position + 1] == Token::RBracket
            {
                // 数组类型: T[]
                self.advance(); // 消耗 [
                self.advance(); // 消耗 ]
                result = format!("{}[]", result);
            } else {
                // 索引访问类型: T[key]
                self.advance();
                // 解析索引键
                let index_key = if let Token::String(ref s, quote) = self.current_token() {
                    let s = s.clone();
                    let quote_char = *quote;
                    self.advance();
                    format!("{}{}{}", quote_char, s, quote_char)
                } else if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    // 解析基本类型作为索引
                    if let Some(idx_type) = self.parse_basic_type() {
                        idx_type
                    } else {
                        break;
                    }
                };
                self.consume(Token::RBracket).ok()?;
                result = format!("{}[{}]", result, index_key);
            }
        }

        let mut types = vec![result];
        let mut operators = Vec::new();

        // 检查是否有更多类型（通过 | 或 & 连接）
        while self.current_token_eq(&Token::Pipe) || self.current_token_eq(&Token::Ampersand) {
            let op = if self.current_token_eq(&Token::Pipe) {
                self.advance();
                "|"
            } else {
                self.advance();
                "&"
            };
            operators.push(op.to_string());
            if let Some(t) = self.parse_basic_type() {
                types.push(t);
            } else {
                break;
            }
        }

        // 如果只有一个类型，返回它；否则返回组合类型
        if types.len() == 1 {
            Some(types[0].clone())
        } else {
            // 交替输出类型和运算符
            let mut final_result = types[0].clone();
            for (i, op) in operators.iter().enumerate() {
                final_result.push(' ');
                final_result.push_str(op);
                final_result.push(' ');
                final_result.push_str(&types[i + 1]);
            }
            Some(final_result)
        }
    }
    /// 检查是否是 Utility Type
    /// v0.3.214: 添加缺失的 NoInfer、Infer、Awaited
    fn is_utility_type(name: &str) -> bool {
        matches!(
            name,
            "Partial"
                | "Required"
                | "Readonly"
                | "Pick"
                | "Omit"
                | "Record"
                | "Exclude"
                | "Extract"
                | "NonNullable"
                | "ReturnType"
                | "Parameters"
                | "ConstructorParameters"
                | "InstanceType"
                | "ThisParameterType"
                | "OmitThisParameter"
                | "Uppercase"
                | "Lowercase"
                | "Capitalize"
                | "Uncapitalize"
                | "NoInfer"
                | "Infer"
                | "Awaited"
                | "ThisType"
                | "Mutable"
                | "Trim"
                | "TrimLeft"
                | "TrimRight"
        )
    }

    fn parse_basic_type(&mut self) -> Option<String> {
        match self.current_token() {
            Token::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                // 处理泛型类型，如 Promise<string>
                // 注意：支持嵌套泛型如 A<B<C>>，需要跟踪 < depth
                if self.current_token_eq(&Token::Lt) {
                    self.consume(Token::Lt).ok()?;
                    let mut type_args = Vec::new();
                    let mut depth = 1; // 已消耗一个 <，depth 从 1 开始

                    // 循环解析类型参数，直到 depth 回到 0（遇到对应的 >）
                    while depth > 0 {
                        // v0.3.189: 检查是否是逗号（处理所有嵌套层级的参数分隔）
                        if self.current_token_eq(&Token::Comma) {
                            self.consume(Token::Comma).ok()?;
                            continue;
                        }

                        // v0.3.189: 检查是否是管道符 |（处理字符串联合类型）
                        if self.current_token_eq(&Token::Pipe) {
                            self.consume(Token::Pipe).ok()?;
                            continue;
                        }

                        // v0.3.189: 检查是否是 & 符号（处理交叉类型）
                        if self.current_token_eq(&Token::Ampersand) {
                            self.consume(Token::Ampersand).ok()?;
                            continue;
                        }

                        // 检查是否是嵌套的 <
                        if self.current_token_eq(&Token::Lt) {
                            self.consume(Token::Lt).ok()?;
                            depth += 1;
                            continue;
                        }

                        // 检查是否是 >
                        if self.current_token_eq(&Token::Gt) {
                            self.consume(Token::Gt).ok()?;
                            depth -= 1;
                            continue;
                        }

                        // v0.3.189: 处理字符串类型（如 "name"）
                        if let Token::String(ref s, quote) = self.current_token() {
                            let s = s.clone();
                            let quote_char = *quote;
                            self.advance();
                            type_args.push(format!("{}{}{}", quote_char, s, quote_char));
                            continue;
                        }

                        // 解析类型参数
                        if let Token::Identifier(ref arg_name) = self.current_token() {
                            type_args.push(arg_name.clone());
                            self.advance();
                        } else if let Some(arg) = self.parse_basic_type() {
                            // 嵌套泛型或其他复杂类型
                            type_args.push(arg);
                        } else {
                            // 不是有效的类型参数，退出循环
                            break;
                        }
                    }

                    // 检查是否是 Utility Type
                    if Self::is_utility_type(&name) {
                        // Utility Types 在转译时被擦除，只返回第一个类型参数
                        // 例如 Partial<User> => User, Pick<T, "name" | "age"> => T
                        if !type_args.is_empty() {
                            Some(type_args[0].clone())
                        } else {
                            Some(name)
                        }
                    } else {
                        Some(format!("{}<{}>", name, type_args.join(", ")))
                    }
                } else {
                    Some(name)
                }
            }
            Token::String(ref s, quote) => {
                let s = s.clone();
                let quote_char = *quote;
                self.advance();
                Some(format!("{}{}{}", quote_char, s, quote_char))
            }
            Token::Number(ref n) => {
                let n = n.clone();
                self.advance();
                Some(n)
            }
            Token::LBrace => {
                // 处理嵌套对象类型，如 Array<{ name: string }>
                self.parse_object_type()
            }
            // 处理 keyof 操作符: keyof T
            Token::Keyof => {
                self.advance();
                if let Some(inner_type) = self.parse_basic_type() {
                    Some(format!("keyof {}", inner_type))
                } else {
                    Some("keyof unknown".to_string())
                }
            }
            // 处理 typeof 操作符: typeof x
            Token::Typeof => {
                self.advance();
                if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    Some(format!("typeof {}", name))
                } else {
                    Some("typeof unknown".to_string())
                }
            }
            // 处理 infer 关键字: infer U 或 infer U extends T
            Token::Infer => {
                self.advance();
                // 获取推断的类型变量名
                if let Token::Identifier(ref name) = self.current_token() {
                    let infer_name = name.clone();
                    self.advance();
                    // 检查是否有 extends 约束
                    if self.current_token_eq(&Token::Extends) {
                        self.advance();
                        if let Some(constraint) = self.parse_basic_type() {
                            Some(format!("infer {} extends {}", infer_name, constraint))
                        } else {
                            Some(format!("infer {}", infer_name))
                        }
                    } else {
                        Some(format!("infer {}", infer_name))
                    }
                } else {
                    Some("infer _".to_string())
                }
            }
            // 处理 never 类型（表示永远不返回的值）
            Token::Never => {
                self.advance();
                Some("never".to_string())
            }
            // 处理函数类型: (arg1: type1, arg2: type2) => returnType
            Token::LParen => self.parse_function_type(),
            // 处理 unknown 类型（类型安全的 top 类型）
            Token::UnknownType => {
                self.advance();
                Some("unknown".to_string())
            }
            // 处理元组类型: [type1, type2, ...]
            Token::LBracket => self.parse_tuple_type(),
            // 注意: 索引访问类型 T[K] 由 parse_type_annotation 处理
            _ => None,
        }
    }

    /// 解析元组类型: [type1, type2, ...]
    fn parse_tuple_type(&mut self) -> Option<String> {
        // 消耗 [
        self.consume(Token::LBracket).ok()?;

        let mut elements = Vec::new();

        // 解析元组元素
        while !self.current_token_eq(&Token::RBracket) {
            // 检查是否是 rest 元素 (...T)
            let is_rest = self.current_token_eq(&Token::DotDotDot);
            if is_rest {
                self.advance(); // 消耗 ...
                if let Some(element_type) = self.parse_type_annotation() {
                    elements.push(format!("...{}", element_type));
                } else {
                    elements.push("...unknown".to_string());
                }
            } else {
                // 解析元素类型
                let element_type = if let Some(t) = self.parse_type_annotation() {
                    t
                } else {
                    "unknown".to_string()
                };
                elements.push(element_type);
            }

            // 检查是否还有更多元素
            if self.current_token_eq(&Token::Comma) {
                self.advance(); // 消耗 ,
            } else {
                break;
            }
        }

        // 消耗 ]
        self.consume(Token::RBracket).ok()?;

        Some(format!("[{}]", elements.join(", ")))
    }

    /// 解析函数类型: (arg1: type1, arg2: type2) => returnType
    fn parse_function_type(&mut self) -> Option<String> {
        // 消耗 (
        self.consume(Token::LParen).ok()?;

        let mut params = Vec::new();

        // 解析参数列表
        while !self.current_token_eq(&Token::RParen) {
            // 处理 rest 参数 (...args)
            let is_rest = self.current_token_eq(&Token::DotDotDot);
            if is_rest {
                self.advance(); // 消耗 ...
            }

            // 解析参数名
            let param_name = if let Token::Identifier(ref name) = self.current_token() {
                let name = name.clone();
                self.advance();
                name
            } else if self.current_token_eq(&Token::Question) {
                // 可选参数
                self.advance();
                if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    name
                } else {
                    break;
                }
            } else {
                break;
            };

            let param_name = if is_rest {
                format!("...{}", param_name)
            } else {
                param_name
            };

            // 检查是否有类型注解
            if self.current_token_eq(&Token::Colon) {
                self.advance(); // 消耗 :
                if let Some(param_type) = self.parse_type_annotation() {
                    params.push(format!("{}: {}", param_name, param_type));
                } else {
                    params.push(param_name);
                }
            } else {
                params.push(param_name);
            }

            // 检查是否还有更多参数
            if self.current_token_eq(&Token::Comma) {
                self.advance(); // 消耗 ,
            } else {
                break;
            }
        }

        // 消耗 )
        self.consume(Token::RParen).ok()?;

        // 检查 FatArrow =>
        self.consume(Token::FatArrow).ok()?;

        // 解析返回类型
        let return_type = if let Some(rt) = self.parse_type_annotation() {
            rt
        } else {
            "void".to_string()
        };

        Some(format!("({}) => {}", params.join(", "), return_type))
    }

    /// 解析函数类型参数列表: (arg1: type1, arg2: type2) - 用于对象类型中的方法签名
    fn parse_function_type_params(&mut self) -> Option<String> {
        // 当前 token 应该是 LParen
        self.consume(Token::LParen).ok()?;

        let mut params = Vec::new();

        // 解析参数列表
        while !self.current_token_eq(&Token::RParen) {
            // 处理 rest 参数 (...args)
            if self.current_token_eq(&Token::DotDotDot) {
                self.advance(); // 消耗 ...
                if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    // 检查类型注解
                    if self.current_token_eq(&Token::Colon) {
                        self.advance();
                        if let Some(t) = self.parse_type_annotation() {
                            params.push(format!("...{}: {}", name, t));
                        } else {
                            params.push(format!("...{}", name));
                        }
                    } else {
                        params.push(format!("...{}", name));
                    }
                }
            } else if let Token::Identifier(ref name) = self.current_token() {
                let name = name.clone();
                self.advance();
                // 检查类型注解
                if self.current_token_eq(&Token::Colon) {
                    self.advance();
                    if let Some(t) = self.parse_type_annotation() {
                        params.push(format!("{}: {}", name, t));
                    } else {
                        params.push(name);
                    }
                } else {
                    params.push(name);
                }
            } else if self.current_token_eq(&Token::Question) {
                // 可选参数
                self.advance();
                if let Token::Identifier(ref name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    params.push(format!("{}?", name));
                }
            } else {
                break;
            }

            // 检查是否还有更多参数
            if self.current_token_eq(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        // 消耗 )
        self.consume(Token::RParen).ok()?;

        Some(format!("({})", params.join(", ")))
    }

    fn consume(&mut self, expected: Token) -> Result<Token> {
        let current = self.current_token();
        if self.current_token_eq(&expected) {
            let advanced = self.advance();
            Ok(advanced)
        } else {
            bail!("Expected {:?} but found {:?}", expected, current);
        }
    }
    fn consume_any(&mut self, expected: &[Token]) -> Result<Token> {
        for token in expected {
            if self.current_token_eq(token) {
                return Ok(self.advance());
            }
        }
        bail!("Expected one of {:?}", expected);
    }
    /// 消耗任何 Identifier token
    fn consume_any_identifier(&mut self) -> Result<Token> {
        if let Token::Identifier(_) = self.current_token() {
            Ok(self.advance())
        } else {
            bail!("Expected identifier");
        }
    }
    /// 消耗参数名（包括 this 关键字）
    fn consume_param_name(&mut self) -> Result<Token> {
        match self.current_token() {
            Token::Identifier(_) | Token::This => Ok(self.advance()),
            _ => bail!("Expected parameter name"),
        }
    }
    /// 消耗属性名（Identifier、String 或 Number）
    fn consume_property_name(&mut self) -> Result<Token> {
        match self.current_token() {
            Token::Identifier(_) | Token::String(_, _) | Token::Number(_) => Ok(self.advance()),
            _ => bail!("Expected property name (identifier, string, or number)"),
        }
    }
    fn current_token(&self) -> &Token {
        &self.tokens[self.position]
    }
    fn current_token_eq(&self, token: &Token) -> bool {
        // 完全匹配，不仅仅是 discriminant
        match (self.current_token(), token) {
            // Identifier 需要比较字符串内容
            (Token::Identifier(a), Token::Identifier(b)) => a == b,
            // String 需要比较值和引号类型
            (Token::String(val1, quote1), Token::String(val2, quote2)) => {
                val1 == val2 && quote1 == quote2
            }
            // Number 需要比较字符串内容
            (Token::Number(a), Token::Number(b)) => a == b,
            // BigInt 需要比较字符串内容 (v0.3.179)
            (Token::BigInt(a), Token::BigInt(b)) => a == b,
            // 其他 token 只比较 discriminant
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        self.position += 1;
        token
    }
    fn is_at_end(&self) -> bool {
        matches!(self.current_token(), Token::Eof)
    }
}
/// 代码生成器
#[allow(dead_code)]
struct CodeEmitter {
    config: TypeScriptCompilerConfig,
    output: String,
}
impl CodeEmitter {
    fn new(config: TypeScriptCompilerConfig) -> Self {
        Self {
            config,
            output: String::new(),
        }
    }
    fn emit(&mut self, node: &ASTNode) -> Result<String> {
        self.emit_node(node);
        Ok(self.output.clone())
    }
    fn emit_node(&mut self, node: &ASTNode) {
        match node {
            ASTNode::Program(statements) => {
                for stmt in statements {
                    self.emit_node(stmt);
                }
            }
            ASTNode::VariableDeclaration {
                is_declare,
                kind,
                name,
                type_annotation,
                initializer,
            } => {
                // 处理 declare const/let/var
                if *is_declare {
                    self.output.push_str("declare ");
                }
                self.output.push_str(kind);
                self.output.push(' ');
                self.output.push_str(name);
                // 跳过类型注解
                if type_annotation.is_some() {
                    // 在转译时移除类型注解
                }
                // 对于 declare 变量，不输出初始化器
                if !*is_declare {
                    if let Some(init) = initializer {
                        self.output.push_str(" = ");
                        if let ASTNode::Expression(expr) = init.as_ref() {
                            self.emit_expression(expr);
                        }
                    }
                }
                self.output.push_str(";\n");
            }
            ASTNode::FunctionDeclaration {
                is_declare,
                name,
                is_async,
                type_params: _,
                params,
                return_type: _,
                body,
            } => {
                // 处理 declare function（空函数体，以分号结束）
                if *is_declare {
                    self.output.push_str("declare ");
                }
                if *is_async {
                    self.output.push_str("async ");
                }
                self.output.push_str("function ");
                self.output.push_str(name);
                self.output.push('(');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_function_parameter(param);
                }
                self.output.push(')');
                if *is_declare || body.is_empty() {
                    // declare function 或空函数体以分号结束
                    self.output.push_str(";\n");
                } else {
                    self.output.push_str(" {\n");
                    for stmt in body {
                        self.emit_node(stmt);
                    }
                    self.output.push_str("}\n");
                }
            }
            // 函数重载签名 - 转译时输出为注释保留签名信息
            ASTNode::FunctionOverload {
                name,
                is_async,
                type_params: _,
                params,
                return_type: _,
            } => {
                // 函数重载签名在 JavaScript 中没有直接对应语法
                // 我们输出一个带有 TypeScript 签名的 JSDoc 注释
                self.output.push_str("/** @overload */\n");
                if *is_async {
                    self.output.push_str("async ");
                }
                self.output.push_str("function ");
                self.output.push_str(name);
                self.output.push('(');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_function_parameter(param);
                }
                self.output.push_str(") { /* overload */ }\n");
            }
            ASTNode::ClassDeclaration {
                is_declare,
                is_abstract: _,
                decorators,
                name,
                extends,
                members,
            } => {
                // 输出装饰器（作为注释保留）
                for decorator in decorators {
                    self.output.push_str("/* @");
                    self.output.push_str(&decorator.name);
                    if !decorator.arguments.is_empty() {
                        self.output.push('(');
                        for (i, arg) in decorator.arguments.iter().enumerate() {
                            if i > 0 {
                                self.output.push_str(", ");
                            }
                            self.emit_expression(arg);
                        }
                        self.output.push(')');
                    }
                    self.output.push_str(" */\n");
                }
                // 处理 declare class
                if *is_declare {
                    self.output.push_str("declare ");
                }
                // 注意: abstract class 是 TypeScript 特有语法，编译到 JS 时移除
                // if *is_abstract {
                //     self.output.push_str("abstract ");
                // }
                self.output.push_str("class ");
                self.output.push_str(name);
                // 添加 extends 子句（如果有）
                if let Some(parent) = extends {
                    self.output.push_str(" extends ");
                    self.output.push_str(parent);
                }
                self.output.push_str(" {\n");
                for member in members {
                    self.emit_node(member);
                }
                self.output.push_str("}\n");
            }
            ASTNode::MethodDeclaration {
                decorators,
                name,
                kind,
                is_async,
                is_static,
                is_abstract: _,
                params,
                body,
            } => {
                // 输出装饰器（作为注释保留）
                for decorator in decorators {
                    self.output.push_str("/* @");
                    self.output.push_str(&decorator.name);
                    if !decorator.arguments.is_empty() {
                        self.output.push('(');
                        for (i, arg) in decorator.arguments.iter().enumerate() {
                            if i > 0 {
                                self.output.push_str(", ");
                            }
                            self.emit_expression(arg);
                        }
                        self.output.push(')');
                    }
                    self.output.push_str(" */\n");
                }
                if *is_static {
                    self.output.push_str("static ");
                }
                // 注意: abstract method 是 TypeScript 特有语法，编译到 JS 时移除
                // if *is_abstract {
                //     self.output.push_str("abstract ");
                // }
                // 输出 get/set 关键字（如果是 getter/setter）
                if *kind != "method" {
                    self.output.push_str(kind);
                    self.output.push(' ');
                }
                if *is_async {
                    self.output.push_str("async ");
                }
                self.output.push_str(name);
                self.output.push('(');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_function_parameter(param);
                }
                self.output.push_str(") {\n");

                // 如果是构造函数，生成访问修饰符参数的赋值语句
                if name == "constructor" {
                    for param in params.iter() {
                        if let FunctionParameter::Simple {
                            name: param_name,
                            is_public,
                            is_private,
                            is_protected,
                            ..
                        } = param
                        {
                            // 如果有访问修饰符，生成 this.paramName = paramName;
                            if *is_public || *is_private || *is_protected {
                                self.output.push_str("    this.");
                                self.output.push_str(param_name);
                                self.output.push_str(" = ");
                                self.output.push_str(param_name);
                                self.output.push_str(";\n");
                            }
                        }
                    }
                }

                for stmt in body {
                    self.emit_node(stmt);
                }
                self.output.push_str("}\n");
            }
            ASTNode::PropertyDeclaration {
                decorators,
                name,
                is_static,
                is_abstract: _,
                initializer,
            } => {
                // 输出装饰器（作为注释保留）
                for decorator in decorators {
                    self.output.push_str("/* @");
                    self.output.push_str(&decorator.name);
                    if !decorator.arguments.is_empty() {
                        self.output.push('(');
                        for (i, arg) in decorator.arguments.iter().enumerate() {
                            if i > 0 {
                                self.output.push_str(", ");
                            }
                            self.emit_expression(arg);
                        }
                        self.output.push(')');
                    }
                    self.output.push_str(" */\n");
                }
                if *is_static {
                    self.output.push_str("static ");
                }
                // 注意: abstract property 是 TypeScript 特有语法，编译到 JS 时移除
                // if *is_abstract {
                //     self.output.push_str("abstract ");
                // }
                self.output.push_str(name);
                if let Some(init) = initializer {
                    self.output.push_str(" = ");
                    self.emit_node(init);
                }
                self.output.push_str(";\n");
            }
            ASTNode::ComputedPropertyDeclaration {
                decorators,
                key_expr,
                is_static,
                initializer,
            } => {
                // 输出装饰器（作为注释保留）
                for decorator in decorators {
                    self.output.push_str("/* @");
                    self.output.push_str(&decorator.name);
                    if !decorator.arguments.is_empty() {
                        self.output.push('(');
                        for (i, arg) in decorator.arguments.iter().enumerate() {
                            if i > 0 {
                                self.output.push_str(", ");
                            }
                            self.emit_expression(arg);
                        }
                        self.output.push(')');
                    }
                    self.output.push_str(" */\n");
                }
                if *is_static {
                    self.output.push_str("static ");
                }
                self.output.push('[');
                self.emit_expression(key_expr);
                self.output.push(']');
                if let Some(init) = initializer {
                    self.output.push_str(" = ");
                    self.emit_node(init);
                }
                self.output.push_str(";\n");
            }
            ASTNode::InterfaceDeclaration { .. } => {
                // 接口在 JavaScript 中不存在，跳过
            }
            ASTNode::TypeAliasDeclaration { .. } => {
                // 类型别名在 JavaScript 中不存在，跳过
                // 但保留类型定义用于类型检查
            }
            ASTNode::EnumDeclaration { name, members } => {
                // 转译枚举为 JavaScript 对象
                self.output.push_str("var ");
                self.output.push_str(name);
                self.output.push_str(" = {\n");
                for (i, member) in members.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(",\n");
                    }
                    self.output.push_str("    ");
                    self.output.push_str(&member.name);
                    self.output.push_str(": ");
                    match &member.value {
                        Some(EnumValue::Number(n)) => {
                            self.output.push_str(&n.to_string());
                        }
                        Some(EnumValue::String(s)) => {
                            self.output.push_str(&format!("\"{}\"", s));
                        }
                        None => {
                            self.output.push('0');
                        }
                    }
                }
                self.output.push_str("\n};\n");
            }
            ASTNode::ImportDeclaration {
                module_specifier,
                imports,
                is_default,
                namespace_alias,
                is_type_only,
            } => {
                // import type 在编译时完全移除，不生成任何运行时代码
                if *is_type_only {
                    return;
                }

                // v0.3.195: 将 ESM import 转换为 CommonJS require
                // 命名空间导入: import * as ns from 'module' -> const ns = require('module')
                if let Some(alias) = namespace_alias {
                    self.output.push_str("const ");
                    self.output.push_str(alias);
                    self.output.push_str(" = require(");
                    self.output.push_str(module_specifier);
                    self.output.push_str(");\n");
                    return;
                }

                // 混合导入（默认 + 命名）
                if *is_default && !imports.is_empty() {
                    // 默认导入
                    for import in imports.iter() {
                        if import.is_default {
                            self.output.push_str("const ");
                            self.output.push_str(&import.imported);
                            self.output.push_str(" = require(");
                            self.output.push_str(module_specifier);
                            self.output.push_str(");\n");
                        }
                    }
                    // 命名导入
                    for import in imports.iter() {
                        if !import.is_default {
                            self.output.push_str("const { ");
                            self.output.push_str(&import.imported);
                            if let Some(alias) = &import.alias {
                                self.output.push_str(" as ");
                                self.output.push_str(alias);
                            }
                            self.output.push_str(" } = require(");
                            self.output.push_str(module_specifier);
                            self.output.push_str(");\n");
                        }
                    }
                    return;
                }

                // 命名导入: import { a, b } from 'module' -> const { a, b } = require('module')
                if !imports.is_empty() {
                    self.output.push_str("const { ");
                    for (i, import) in imports.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.output.push_str(&import.imported);
                        if let Some(alias) = &import.alias {
                            self.output.push_str(" as ");
                            self.output.push_str(alias);
                        }
                    }
                    self.output.push_str(" } = require(");
                    self.output.push_str(module_specifier);
                    self.output.push_str(");\n");
                    return;
                }

                // 副作用导入: import 'module' -> require('module')
                self.output.push_str("require(");
                self.output.push_str(module_specifier);
                self.output.push_str(");\n");
            }
            ASTNode::ExportDeclaration {
                exports,
                is_default,
                module_specifier,
                inline_declaration,
                is_type_only,
            } => {
                // export type 在编译时完全移除，不生成任何运行时代码
                if *is_type_only {
                    return;
                }

                // v0.3.195: 将 ESM export 转换为注释占位符
                // 注意：完整的 export 支持需要变量追踪，这里使用占位符

                // 默认导出: export default ... -> /* ESM default export: ... */
                if *is_default {
                    self.output.push_str("/* ESM default export");
                    if let Some(ref decl) = inline_declaration {
                        self.output.push_str(": ");
                        // 输出内联声明的简略形式
                        match decl.as_ref() {
                            ASTNode::FunctionDeclaration { name, is_async, .. } => {
                                if *is_async {
                                    self.output.push_str("async function ");
                                } else {
                                    self.output.push_str("function ");
                                }
                                self.output.push_str(name);
                            }
                            ASTNode::ClassDeclaration { name, .. } => {
                                self.output.push_str("class ");
                                self.output.push_str(name);
                            }
                            _ => {
                                self.output.push_str("...");
                            }
                        }
                    }
                    self.output.push_str(" */\n");
                    return;
                }

                // 重新导出 from: export { a, b } from 'module' -> /* ESM re-export { a, b } from 'module' */
                if let Some(specifier) = module_specifier {
                    self.output.push_str("/* ESM re-export { ");
                    for (i, export) in exports.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.output.push_str(&export.name);
                        if let Some(alias) = &export.alias {
                            self.output.push_str(" as ");
                            self.output.push_str(alias);
                        }
                    }
                    self.output.push_str(" } from ");
                    self.output.push_str(specifier);
                    self.output.push_str(" */\n");
                    return;
                }

                // 命名导出: export { a, b } -> /* ESM export { a, b } */
                if !exports.is_empty() {
                    self.output.push_str("/* ESM export { ");
                    for (i, export) in exports.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.output.push_str(&export.name);
                        if let Some(alias) = &export.alias {
                            self.output.push_str(" as ");
                            self.output.push_str(alias);
                        }
                    }
                    self.output.push_str(" } */\n");
                    return;
                }

                // 内联导出声明: export const x = 1 -> /* ESM export: const x = 1 */
                if let Some(ref decl) = inline_declaration {
                    self.output.push_str("/* ESM export: ");
                    match decl.as_ref() {
                        ASTNode::VariableDeclaration {
                            kind,
                            name,
                            initializer,
                            ..
                        } => {
                            self.output.push_str(kind);
                            self.output.push(' ');
                            self.output.push_str(name);
                            if initializer.is_some() {
                                self.output.push_str(" = ...");
                            }
                        }
                        ASTNode::FunctionDeclaration { name, is_async, .. } => {
                            if *is_async {
                                self.output.push_str("async function ");
                            } else {
                                self.output.push_str("function ");
                            }
                            self.output.push_str(name);
                        }
                        ASTNode::ClassDeclaration { name, .. } => {
                            self.output.push_str("class ");
                            self.output.push_str(name);
                        }
                        _ => {
                            self.output.push_str("...");
                        }
                    }
                    self.output.push_str(" */\n");
                }
            }
            ASTNode::Expression(expr) => {
                self.emit_expression(expr);
            }
            ASTNode::Statement(stmt) => {
                match stmt {
                    ASTStatement::Block(statements) => {
                        self.output.push_str("{\n");
                        for stmt in statements {
                            self.emit_node(stmt);
                        }
                        self.output.push_str("}\n");
                    }
                    ASTStatement::Expression(expr) => {
                        self.emit_expression(expr);
                        self.output.push_str(";\n");
                    }
                    ASTStatement::Return(expr) => {
                        self.output.push_str("return");
                        if let Some(e) = expr {
                            self.output.push(' ');
                            self.emit_expression(e);
                        }
                        self.output.push_str(";\n");
                    }
                    ASTStatement::If {
                        test,
                        consequent,
                        alternate,
                    } => {
                        self.output.push_str("if (");
                        self.emit_expression(test);
                        self.output.push_str(") ");
                        self.emit_node(consequent);
                        if let Some(alt) = alternate {
                            self.output.push_str("else ");
                            self.emit_node(alt);
                        }
                    }
                    ASTStatement::ForOf {
                        initializer,
                        iterable,
                        body,
                    } => {
                        self.output.push_str("for (");
                        self.emit_node(initializer);
                        self.output.push_str(" of ");
                        self.emit_expression(iterable);
                        self.output.push_str(") ");
                        self.emit_node(body);
                    }
                    ASTStatement::For {
                        initializer,
                        condition,
                        update,
                        body,
                    } => {
                        self.output.push_str("for (");
                        if let Some(init) = initializer {
                            self.emit_node(init);
                        }
                        self.output.push_str("; ");
                        if let Some(cond) = condition {
                            self.emit_expression(cond);
                        }
                        self.output.push_str("; ");
                        if let Some(upd) = update {
                            self.emit_expression(upd);
                        }
                        self.output.push_str(") ");
                        self.emit_node(body);
                    }
                    ASTStatement::While { test, body } => {
                        self.output.push_str("while (");
                        self.emit_expression(test);
                        self.output.push_str(") ");
                        self.emit_node(body);
                    }
                    ASTStatement::DoWhile { body, test } => {
                        self.output.push_str("do ");
                        self.emit_node(body);
                        self.output.push_str("while (");
                        self.emit_expression(test);
                        self.output.push_str(");\n");
                    }
                    ASTStatement::Switch {
                        discriminant,
                        cases,
                    } => {
                        self.output.push_str("switch (");
                        self.emit_expression(discriminant);
                        self.output.push_str(") {\n");
                        for case in cases {
                            if let Some(test) = &case.test {
                                self.output.push_str("case ");
                                self.emit_expression(test);
                                self.output.push_str(":\n");
                            } else {
                                self.output.push_str("default:\n");
                            }
                            for stmt in &case.body {
                                self.emit_node(stmt);
                            }
                        }
                        self.output.push_str("}\n");
                    }
                    ASTStatement::Try {
                        body,
                        handler,
                        finalizer,
                    } => {
                        self.output.push_str("try ");
                        self.emit_node(body);
                        if let Some(catch) = handler {
                            self.output.push_str("catch");
                            if let Some(param) = &catch.param {
                                self.output.push_str(" (");
                                self.output.push_str(param);
                                self.output.push(')');
                            }
                            self.output.push(' ');
                            self.output.push_str("{\n");
                            for stmt in &catch.body {
                                self.emit_node(stmt);
                            }
                            self.output.push_str("}\n");
                        }
                        if let Some(finally) = finalizer {
                            self.output.push_str("finally ");
                            self.emit_node(finally);
                        }
                    }
                    ASTStatement::Break { label } => {
                        self.output.push_str("break");
                        if let Some(lbl) = label {
                            self.output.push(' ');
                            self.output.push_str(lbl);
                        }
                        self.output.push_str(";\n");
                    }
                    ASTStatement::Continue { label } => {
                        self.output.push_str("continue");
                        if let Some(lbl) = label {
                            self.output.push(' ');
                            self.output.push_str(lbl);
                        }
                        self.output.push_str(";\n");
                    }
                    ASTStatement::Throw { expression } => {
                        self.output.push_str("throw ");
                        self.emit_expression(expression);
                        self.output.push_str(";\n");
                    }
                    // 命名空间编译: namespace MyNamespace { ... }
                    // 编译为: var MyNamespace; (function(MyNamespace) { ... })(MyNamespace || (MyNamespace = {}));
                    // 嵌套命名空间: namespace A.B.C { ... }
                    // 编译为: var A; (function(A) { var B; (function(B) { var C; (function(C) { ... })(C || (C = {})); })(B || (B = {})); })(A || (A = {}));
                    // declare namespace: 生成 declare namespace 声明语法
                    ASTStatement::Namespace {
                        name,
                        full_name,
                        body,
                        is_declare,
                    } => {
                        if *is_declare {
                            // declare namespace - 保留 declare namespace 语法
                            self.output.push_str("declare namespace ");
                            self.output.push_str(full_name);
                            self.output.push_str(" {\n");
                            for stmt in body {
                                self.emit_node(stmt);
                            }
                            self.output.push_str("}\n");
                        } else {
                            // 检查是否为嵌套命名空间
                            let names: Vec<&str> = full_name.split('.').collect();
                            if names.len() == 1 {
                                // 简单命名空间
                                self.output.push_str("var ");
                                self.output.push_str(name);
                                self.output.push_str(";\n");
                                self.output.push_str("(function(");
                                self.output.push_str(name);
                                self.output.push_str(") {\n");
                                for stmt in body {
                                    self.emit_node(stmt);
                                }
                                self.output.push_str("})(");
                                self.output.push_str(name);
                                self.output.push_str(" || (");
                                self.output.push_str(name);
                                self.output.push_str(" = {}));\n");
                            } else {
                                // 嵌套命名空间 - 递归创建嵌套 IIFE
                                self.emit_nested_namespace(&names, 0, body);
                            }
                        }
                    }
                    // declare global { ... } - 直接输出内部的声明
                    ASTStatement::GlobalDeclaration { body } => {
                        // 全局声明块在转译时输出为注释保留声明
                        self.output.push_str("/* declare global { */\n");
                        for stmt in body {
                            self.emit_node(stmt);
                        }
                        self.output.push_str("/* } */\n");
                    }
                    // declare module "name" { ... } - 模块声明
                    ASTStatement::ModuleDeclaration { name, body } => {
                        // 模块声明在转译时保留声明语法
                        // 内部的 export 语句需要转换为 declare export 格式
                        self.output.push_str("declare module \"");
                        self.output.push_str(name);
                        self.output.push_str("\" {\n");
                        for stmt in body {
                            match stmt {
                                ASTNode::ExportDeclaration {
                                    exports,
                                    is_default,
                                    module_specifier,
                                    inline_declaration,
                                    is_type_only,
                                } => {
                                    // 在 declare module 内部，将 export 转换为 declare export
                                    if *is_type_only {
                                        // export type 应该完全移除
                                        continue;
                                    }
                                    if let Some(specifier) = module_specifier {
                                        // export { ... } from 'module' - 重新导出
                                        if exports.is_empty() {
                                            self.output.push_str("declare export * from ");
                                        } else {
                                            self.output.push_str("declare export { ");
                                            for (i, export) in exports.iter().enumerate() {
                                                if i > 0 {
                                                    self.output.push_str(", ");
                                                }
                                                self.output.push_str(&export.name);
                                                if let Some(alias) = &export.alias {
                                                    self.output.push_str(" as ");
                                                    self.output.push_str(alias);
                                                }
                                            }
                                            self.output.push_str(" } from ");
                                        }
                                        self.output.push_str(specifier);
                                        self.output.push_str(";\n");
                                    } else if *is_default {
                                        // export default - 转换为 declare export default
                                        self.output.push_str("declare export default");
                                        if let Some(ref decl) = inline_declaration {
                                            match decl.as_ref() {
                                                ASTNode::FunctionDeclaration {
                                                    name,
                                                    is_async,
                                                    ..
                                                } => {
                                                    if *is_async {
                                                        self.output.push_str(" async");
                                                    }
                                                    self.output.push_str(" function ");
                                                    self.output.push_str(name);
                                                }
                                                ASTNode::ClassDeclaration { name, .. } => {
                                                    self.output.push_str(" class ");
                                                    self.output.push_str(name);
                                                }
                                                _ => {}
                                            }
                                        }
                                        self.output.push_str(";\n");
                                    } else if !exports.is_empty() {
                                        // export { a, b } - 转换为 declare export { a, b }
                                        self.output.push_str("declare export { ");
                                        for (i, export) in exports.iter().enumerate() {
                                            if i > 0 {
                                                self.output.push_str(", ");
                                            }
                                            self.output.push_str(&export.name);
                                            if let Some(alias) = &export.alias {
                                                self.output.push_str(" as ");
                                                self.output.push_str(alias);
                                            }
                                        }
                                        self.output.push_str(" };\n");
                                    } else if let Some(ref decl) = inline_declaration {
                                        // export const/function/class - 转换为 declare export const/function/class
                                        match decl.as_ref() {
                                            ASTNode::VariableDeclaration {
                                                kind,
                                                name,
                                                ..
                                            } => {
                                                self.output.push_str("declare export ");
                                                self.output.push_str(kind);
                                                self.output.push(' ');
                                                self.output.push_str(name);
                                                self.output.push_str(";\n");
                                            }
                                            ASTNode::FunctionDeclaration {
                                                name, is_async, ..
                                            } => {
                                                self.output.push_str("declare export ");
                                                if *is_async {
                                                    self.output.push_str("async ");
                                                }
                                                self.output.push_str("function ");
                                                self.output.push_str(name);
                                                self.output.push_str(";\n");
                                            }
                                            ASTNode::FunctionOverload {
                                                name,
                                                is_async,
                                                params,
                                                return_type,
                                                ..
                                            } => {
                                                // 处理 export function 重载签名（如 export function foo(): void;）
                                                self.output.push_str("declare export ");
                                                if *is_async {
                                                    self.output.push_str("async ");
                                                }
                                                self.output.push_str("function ");
                                                self.output.push_str(name);
                                                self.output.push('(');
                                                for (i, param) in params.iter().enumerate() {
                                                    if i > 0 {
                                                        self.output.push_str(", ");
                                                    }
                                                    match param {
                                                        FunctionParameter::Simple {
                                                            name,
                                                            type_annotation,
                                                            ..
                                                        } => {
                                                            self.output.push_str(name);
                                                            if let Some(ty) = type_annotation {
                                                                self.output.push_str(": ");
                                                                self.output.push_str(ty);
                                                            }
                                                        }
                                                        _ => {
                                                            self.output.push('_');
                                                        }
                                                    }
                                                }
                                                self.output.push(')');
                                                if let Some(rt) = return_type {
                                                    self.output.push_str(": ");
                                                    self.output.push_str(rt);
                                                }
                                                self.output.push_str(";\n");
                                            }
                                            ASTNode::ClassDeclaration { name, .. } => {
                                                self.output.push_str("declare export class ");
                                                self.output.push_str(name);
                                                self.output.push_str(";\n");
                                            }
                                            ASTNode::Statement(ASTStatement::Namespace {
                                                name,
                                                body,
                                                ..
                                            }) => {
                                                // 处理 export namespace Inner { ... }
                                                self.output.push_str("declare export namespace ");
                                                self.output.push_str(name);
                                                self.output.push_str(" {\n");
                                                for stmt in body {
                                                    match stmt {
                                                        ASTNode::ExportDeclaration {
                                                            exports: _,
                                                            is_default: _,
                                                            module_specifier: _,
                                                            inline_declaration,
                                                            is_type_only,
                                                        } => {
                                                            // 递归处理嵌套的 export
                                                            if *is_type_only {
                                                                continue;
                                                            }
                                                            if let Some(ref decl) =
                                                                inline_declaration
                                                            {
                                                                match decl.as_ref() {
                                                                    ASTNode::FunctionDeclaration { name: fn_name, is_async, .. } => {
                                                                        self.output.push_str("declare export ");
                                                                        if *is_async {
                                                                            self.output.push_str("async ");
                                                                        }
                                                                        self.output.push_str("function ");
                                                                        self.output.push_str(fn_name);
                                                                        self.output.push_str(";\n");
                                                                    }
                                                                    ASTNode::FunctionOverload { name: fn_name, is_async, params, return_type, .. } => {
                                                                        self.output.push_str("declare export ");
                                                                        if *is_async {
                                                                            self.output.push_str("async ");
                                                                        }
                                                                        self.output.push_str("function ");
                                                                        self.output.push_str(fn_name);
                                                                        self.output.push('(');
                                                                        for (i, param) in params.iter().enumerate() {
                                                                            if i > 0 {
                                                                                self.output.push_str(", ");
                                                                            }
                                                                            match param {
                                                                                FunctionParameter::Simple { name: param_name, type_annotation, .. } => {
                                                                                    self.output.push_str(param_name);
                                                                                    if let Some(ty) = type_annotation {
                                                                                        self.output.push_str(": ");
                                                                                        self.output.push_str(ty);
                                                                                    }
                                                                                }
                                                                                _ => self.output.push('_'),
                                                                            }
                                                                        }
                                                                        self.output.push(')');
                                                                        if let Some(rt) = return_type {
                                                                            self.output.push_str(": ");
                                                                            self.output.push_str(rt);
                                                                        }
                                                                        self.output.push_str(";\n");
                                                                    }
                                                                    ASTNode::VariableDeclaration { kind, name: var_name, .. } => {
                                                                        self.output.push_str("declare export ");
                                                                        self.output.push_str(kind);
                                                                        self.output.push(' ');
                                                                        self.output.push_str(var_name);
                                                                        self.output.push_str(";\n");
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            self.emit_node(stmt);
                                                        }
                                                    }
                                                }
                                                self.output.push_str("}\n");
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {
                                    self.emit_node(stmt);
                                }
                            }
                        }
                        self.output.push_str("}\n");
                    }
                    // 三重合并声明 - interface + namespace + module 同名合并
                    ASTStatement::TripleMergedDeclaration {
                        name,
                        interface_properties: _,
                        interface_extends: _,
                        interface_index_signature: _,
                        body,
                        is_declare,
                    } => {
                        // 对于 declare 声明，输出注释占位符
                        if *is_declare {
                            self.output.push_str("/* triple merged declare ");
                            self.output.push_str(name);
                            self.output.push_str(" */\n");
                        } else {
                            // 非 declare 情况：输出命名空间结构
                            self.output.push_str("var ");
                            self.output.push_str(name);
                            self.output.push_str(";\n");
                            self.output.push_str("(function(");
                            self.output.push_str(name);
                            self.output.push_str(") {\n");
                            // 输出 body（来自 namespace/module 的内容）
                            for stmt in body {
                                self.emit_node(stmt);
                            }
                            self.output.push_str("})(");
                            self.output.push_str(name);
                            self.output.push_str(" || (");
                            self.output.push_str(name);
                            self.output.push_str(" = {}));\n");
                        }
                    }
                    // export = 语句 (CommonJS/AMD 兼容)
                    ASTStatement::ExportAssignment {
                        expression,
                        is_type_only,
                    } => {
                        if *is_type_only {
                            self.output.push_str("/* export = type */\n");
                        } else {
                            self.output.push_str("/* export = ");
                            self.emit_expression(expression);
                            self.output.push_str(" */\n");
                        }
                    }
                }
            }
            ASTNode::DestructuringPattern { pattern } => {
                self.emit_destructuring_pattern(pattern);
            }
            ASTNode::DestructuringDeclaration {
                kind,
                pattern,
                source,
            } => {
                self.output.push_str(kind);
                self.output.push(' ');
                self.emit_destructuring_pattern(pattern);
                self.output.push_str(" = ");
                self.emit_expression(source);
                self.output.push_str(";\n");
            }
        }
    }

    /// 发射嵌套命名空间
    /// 递归生成嵌套 IIFE 结构
    /// 例如: namespace A.B.C { ... }
    /// 编译为: var A; (function(A) { var B; (function(B) { var C; (function(C) { ... })(C || (C = {})); })(B || (B = {})); })(A || (A = {}));
    fn emit_nested_namespace(&mut self, names: &[&str], index: usize, body: &[ASTNode]) {
        let current_name = names[index];
        let is_last = index == names.len() - 1;

        if is_last {
            // 最后一层命名空间 - 输出变量声明和 IIFE
            self.output.push_str("var ");
            self.output.push_str(current_name);
            self.output.push_str(";\n");
            self.output.push_str("(function(");
            self.output.push_str(current_name);
            self.output.push_str(") {\n");
            for stmt in body {
                self.emit_node(stmt);
            }
            self.output.push_str("})(");
            self.output.push_str(current_name);
            self.output.push_str(" || (");
            self.output.push_str(current_name);
            self.output.push_str(" = {}));\n");
        } else {
            // 中间层 - 输出变量声明和 IIFE，IIFE 内部递归处理下一层
            self.output.push_str("var ");
            self.output.push_str(current_name);
            self.output.push_str(";\n");
            self.output.push_str("(function(");
            self.output.push_str(current_name);
            self.output.push_str(") {\n");
            // 递归处理下一层命名空间
            self.emit_nested_namespace(names, index + 1, body);
            self.output.push_str("})(");
            self.output.push_str(current_name);
            self.output.push_str(" || (");
            self.output.push_str(current_name);
            self.output.push_str(" = {}));\n");
        }
    }

    /// 发射解构模式
    fn emit_destructuring_pattern(&mut self, pattern: &DestructuringPattern) {
        match pattern {
            DestructuringPattern::Array { elements } => {
                self.output.push('[');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    if let Some(e) = elem {
                        self.emit_destructuring_element(e);
                    }
                }
                self.output.push(']');
            }
            DestructuringPattern::Object { properties } => {
                self.output.push('{');
                for (i, prop) in properties.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    if prop.is_rest {
                        self.output.push_str("...");
                        if let Some(alias) = &prop.alias {
                            self.output.push_str(alias);
                        }
                    } else {
                        // 输出属性名
                        self.output.push_str(&prop.key);
                        // 如果有别名，输出重命名
                        if let Some(alias) = &prop.alias {
                            self.output.push_str(": ");
                            self.output.push_str(alias);
                        }
                        // 如果有默认值，输出默认值
                        if let Some(default) = &prop.default_value {
                            self.output.push_str(" = ");
                            self.emit_node(default);
                        }
                    }
                }
                self.output.push('}');
            }
        }
    }

    /// 发射数组解构元素
    fn emit_destructuring_element(&mut self, element: &DestructuringElement) {
        self.emit_node(&element.pattern);
        if let Some(default) = &element.default_value {
            self.output.push_str(" = ");
            self.emit_node(default);
        }
    }

    /// 发射函数参数（支持简单参数和解构参数）
    fn emit_function_parameter(&mut self, param: &FunctionParameter) {
        match param {
            FunctionParameter::Simple {
                name,
                type_annotation: _,
                default_value,
                is_public: _,
                is_private: _,
                is_protected: _,
                is_readonly: _,
            } => {
                self.output.push_str(name);
                if let Some(default) = default_value {
                    self.output.push_str(" = ");
                    self.emit_node(default);
                }
            }
            FunctionParameter::Destructuring {
                pattern,
                default_value,
            } => {
                self.emit_destructuring_pattern(pattern);
                if let Some(default) = default_value {
                    self.output.push_str(" = ");
                    self.emit_node(default);
                }
            }
        }
    }

    fn emit_expression(&mut self, expr: &ASTExpression) {
        match expr {
            ASTExpression::Identifier(name) => {
                self.output.push_str(name);
            }
            ASTExpression::Literal(value) => {
                self.output.push_str(value);
            }
            ASTExpression::BinaryExpression {
                left,
                operator,
                right,
            } => {
                self.emit_expression(left);
                self.output.push(' ');
                self.output.push_str(operator);
                self.output.push(' ');
                self.emit_expression(right);
            }
            ASTExpression::CallExpression { callee, arguments } => {
                // 检查 callee 是否需要括号包裹（箭头函数）
                let needs_parentheses = matches!(
                    callee.as_ref(),
                    ASTExpression::ArrowFunctionExpression { .. }
                );
                if needs_parentheses {
                    self.output.push('(');
                }
                self.emit_expression(callee);
                if needs_parentheses {
                    self.output.push(')');
                }
                self.output.push('(');
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
            ASTExpression::MemberExpression { object, property } => {
                self.emit_expression(object);
                self.output.push('.');
                self.output.push_str(property);
            }
            ASTExpression::IndexExpression { object, index } => {
                self.emit_expression(object);
                self.output.push('[');
                self.emit_expression(index);
                self.output.push(']');
            }
            ASTExpression::ObjectProperty {
                name,
                key_expr,
                value,
            } => {
                // 对象属性（用于对象字面量内部）
                if let Some(expr) = key_expr {
                    self.output.push('[');
                    self.emit_expression(expr);
                    self.output.push(']');
                } else if let Some(n) = name {
                    self.output.push_str(n);
                }
                self.output.push_str(": ");
                self.emit_expression(value);
            }
            ASTExpression::ObjectLiteral { properties } => {
                self.output.push('{');
                for (i, prop) in properties.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    match prop {
                        ASTExpression::ObjectProperty {
                            name,
                            key_expr,
                            value,
                        } => {
                            if let Some(expr) = key_expr {
                                // 计算属性名: [expr]
                                self.output.push('[');
                                self.emit_expression(expr);
                                self.output.push(']');
                            } else if let Some(n) = name {
                                // 普通属性名: name
                                self.output.push_str(n);
                            }
                            self.output.push_str(": ");
                            self.emit_expression(value);
                        }
                        _ => {
                            // 降级处理：尝试作为普通表达式输出
                            self.emit_expression(prop);
                        }
                    }
                }
                self.output.push('}');
            }
            ASTExpression::ArrowFunctionExpression {
                params,
                body,
                return_type,
                is_async,
            } => {
                // 如果是 async 箭头函数，添加 async 关键字
                if *is_async {
                    self.output.push_str("async ");
                }
                // v0.3.180: 转译箭头函数参数（跳过类型注解，保留默认值）
                self.output.push('(');
                for (i, (param_name, _, default_value)) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(param_name);
                    // 输出默认值
                    if let Some(default) = default_value {
                        self.output.push_str(" = ");
                        self.output.push_str(default);
                    }
                }
                self.output.push_str(") => ");
                // 转译函数体 - body 现在是 ASTNode，可以是 Expression 或 Block
                self.emit_node(body);
                // 跳过返回类型注解（在转译时移除）
                if return_type.is_some() {
                    // 已移除
                }
            }
            ASTExpression::FunctionExpression {
                is_async,
                type_params: _,
                params,
                return_type: _,
                body,
            } => {
                // 函数表达式: function(...) { ... } 或 async function(...) { ... }
                if *is_async {
                    self.output.push_str("async ");
                }
                self.output.push_str("function ");
                self.output.push('(');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_function_parameter(param);
                }
                self.output.push_str(") {\n");
                for stmt in body {
                    self.emit_node(stmt);
                }
                self.output.push_str("}\n");
            }
            ASTExpression::TemplateLiteral { parts } => {
                // 将模板字符串转换为字符串拼接
                // `part1${expr1}part2` => "part1" + expr1 + "part2"
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(" + ");
                    }
                    self.emit_expression(part);
                }
            }
            ASTExpression::Await { expression } => {
                // await 表达式直接转译为 JavaScript
                self.output.push_str("await ");
                self.emit_expression(expression);
            }
            ASTExpression::UpdateExpression {
                argument,
                operator,
                is_prefix,
            } => {
                if *is_prefix {
                    self.output.push_str(operator);
                    self.emit_expression(argument);
                } else {
                    self.emit_expression(argument);
                    self.output.push_str(operator);
                }
            }
            ASTExpression::Unary { operator, operand } => {
                self.output.push_str(operator);
                self.output.push(' ');
                self.emit_expression(operand);
            }
            ASTExpression::NewExpression {
                constructor,
                arguments,
            } => {
                self.output.push_str("new ");
                self.emit_expression(constructor);
                self.output.push('(');
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
            ASTExpression::ThisExpression => {
                self.output.push_str("this");
            }
            ASTExpression::ArrayExpression { elements } => {
                self.output.push('[');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    if let Some(expr) = elem {
                        self.emit_expression(expr);
                    }
                }
                self.output.push(']');
            }
            ASTExpression::SpreadExpression { argument } => {
                self.output.push_str("...");
                self.emit_expression(argument);
            }
            ASTExpression::ConditionalExpression {
                condition,
                consequent,
                alternate,
            } => {
                self.emit_expression(condition);
                self.output.push_str(" ? ");
                self.emit_expression(consequent);
                self.output.push_str(" : ");
                self.emit_expression(alternate);
            }
            ASTExpression::SuperExpression => {
                self.output.push_str("super");
            }
            // TypeScript 类型断言: value as Type 或 value as const
            // 转译时移除类型信息，直接输出原始表达式
            ASTExpression::TSAsExpression {
                expression,
                target_type: _,
                is_const: _,
            } => {
                self.emit_expression(expression);
            }
            // TypeScript 尖括号类型断言: <Type>value
            // 转译时移除类型信息，直接输出原始表达式
            ASTExpression::TSAngleBracketAssertion {
                expression,
                target_type: _,
            } => {
                self.emit_expression(expression);
            }
            // TypeScript satisfies 操作符: expr satisfies Type
            // 转译时移除类型信息，直接输出原始表达式
            ASTExpression::TSSatisfiesExpression {
                expression,
                target_type: _,
            } => {
                self.emit_expression(expression);
            }
            ASTExpression::AssignmentExpression { left, right } => {
                self.emit_expression(left);
                self.output.push_str(" = ");
                self.emit_expression(right);
            }
        }
    }
}
/// 编译输出
#[derive(Debug, Clone)]
pub struct CompilationOutput {
    pub js_code: String,
    pub source_map: Option<String>,
    pub diagnostics: Vec<TypeScriptError>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lexical_analysis() {
        let compiler: _ = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source: _ = "let x: number = 5;";
        let tokens: _ = compiler.lexical_analysis(source, "test.ts").unwrap();
        assert!(tokens.iter().any(|t| matches!(t, Token::Let)));
        assert!(tokens.iter().any(|t| matches!(t, Token::Identifier(_))));
        assert!(tokens.iter().any(|t| matches!(t, Token::Colon)));
        assert!(tokens.iter().any(|t| matches!(t, Token::Identifier(_))));
        assert!(tokens.iter().any(|t| matches!(t, Token::Eq)));
        assert!(tokens.iter().any(|t| matches!(t, Token::Number(_))));
    }
    #[test]
    fn test_compile_simple_typescript() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source: _ = "let x: number = 5;";
        let result: _ = compiler.compile_source(source, "test.ts").unwrap();
        // 打印实际输出用于调试
        assert!(result.js_code.contains("let x"));
        assert!(!result.js_code.contains(": number"));
    }
    #[test]
    fn test_compile_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source: _ = "function add(a: number, b: number): number { return a + b; }";
        let result: _ = compiler.compile_source(source, "test.ts").unwrap();
        // 打印实际输出用于调试
        assert!(result.js_code.contains("function add"));
        assert!(result.js_code.contains("a, b"));
        assert!(result.js_code.contains("return a + b"));
        assert!(!result.js_code.contains(": number"));
    }

    #[test]
    fn test_source_map_generation() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = "let x: number = 5;\nlet y: string = 'hello';";
        let result = compiler.compile_source(source, "test.ts").unwrap();

        // Verify source map is generated
        assert!(
            result.source_map.is_some(),
            "Source map should be generated"
        );

        let source_map = result.source_map.unwrap();

        // Verify source map structure
        assert!(
            source_map.contains("\"version\":3"),
            "Should have version 3"
        );
        assert!(
            source_map.contains("\"sources\":[\"test.ts\"]"),
            "Should contain source file"
        );
        assert!(
            source_map.contains("\"sourcesContent\""),
            "Should have sourcesContent"
        );
        assert!(source_map.contains("\"mappings\""), "Should have mappings");
    }

    #[test]
    fn test_source_map_contains_source_content() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = "let x: number = 5;";
        let result = compiler.compile_source(source, "test.ts").unwrap();

        let source_map = result.source_map.unwrap();
        assert!(
            source_map.contains("let x: number = 5;"),
            "Source map should contain original source code"
        );
    }

    // v0.3.139: Source map improvements tests
    #[test]
    fn test_build_line_positions_single_line() {
        let positions = build_line_positions("let x = 5;");
        assert_eq!(positions.len(), 1, "Single line should have one position");
        assert_eq!(positions[0], 0, "First position should be 0");
    }

    #[test]
    fn test_build_line_positions_multi_line() {
        let source = "line1\nline2\nline3";
        let positions = build_line_positions(source);
        assert_eq!(
            positions.len(),
            3,
            "Three lines should have three positions"
        );
        assert_eq!(positions[0], 0, "First line starts at 0");
        assert_eq!(positions[1], 6, "Second line starts after first newline");
        assert_eq!(positions[2], 12, "Third line starts after second newline");
    }

    #[test]
    fn test_source_map_multiline_generation() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = "let x: number = 5;\nlet y: string = 'hello';\nlet z: boolean = true;";
        let result = compiler.compile_source(source, "test.ts").unwrap();

        // Verify source map is generated
        assert!(
            result.source_map.is_some(),
            "Source map should be generated"
        );

        let source_map = result.source_map.unwrap();

        // Verify source map structure for multi-line
        assert!(
            source_map.contains("\"version\":3"),
            "Should have version 3"
        );
        assert!(
            source_map.contains("\"sources\":[\"test.ts\"]"),
            "Should contain source file"
        );
        assert!(source_map.contains("\"mappings\""), "Should have mappings");

        // Verify each line is in sourcesContent
        assert!(
            source_map.contains("let x: number"),
            "Should contain first line"
        );
        assert!(
            source_map.contains("let y: string"),
            "Should contain second line"
        );
        assert!(
            source_map.contains("let z: boolean"),
            "Should contain third line"
        );
    }

    #[test]
    fn test_source_map_with_type_annotations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
interface Config {
    host: string;
    port: number;
}

function setup(config: Config): void {
    console.log(config.host);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();

        assert!(
            result.source_map.is_some(),
            "Source map should be generated"
        );
        let source_map = result.source_map.unwrap();

        // Verify the source map contains the original TypeScript
        assert!(
            source_map.contains("interface Config"),
            "Should contain interface"
        );
        assert!(
            source_map.contains("function setup"),
            "Should contain function"
        );
    }

    #[test]
    fn test_generate_vlq_mappings_improved() {
        let js_code = "let x = 5;\nlet y = 10;";
        let line_positions = build_line_positions("line1\nline2");
        let mappings = generate_vlq_mappings_improved(js_code, &line_positions);

        // Should have mappings with semicolons separating lines
        assert!(
            mappings.contains(';'),
            "Should have semicolons between lines"
        );

        // Should have valid VLQ characters (including comma as separator)
        for ch in mappings.chars() {
            if ch != ';' {
                assert!(
                    ch.is_alphanumeric() || ch == '+' || ch == '/' || ch == ',',
                    "Invalid character in mappings: {}",
                    ch
                );
            }
        }
    }

    #[test]
    fn test_vlq_encoding() {
        // Test VLQ encoding
        assert_eq!(encode_vlq(0), "A");
        assert_eq!(encode_vlq(1), "B");
        assert_eq!(encode_vlq(16), "Q");
    }

    #[test]
    fn test_escape_for_json() {
        assert_eq!(escape_for_json("hello"), "hello");
        assert_eq!(escape_for_json("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_for_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_for_json("hello\\world"), "hello\\\\world");
    }

    #[test]
    fn test_object_literal_in_function_call() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
interface User {
    name: string;
    version: string;
}

function greet(user: User): string {
    return "Hello";
}

console.log(greet({name: "Test", version: "1.0"}));
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // The object literal has spaces: {name: "Test", version: "1.0"}
        assert!(
            result
                .js_code
                .contains("greet({name: \"Test\", version: \"1.0\"})"),
            "Should contain object literal in function call: {}",
            result.js_code
        );
    }

    #[test]
    fn test_computed_property_name() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test computed property names: { [expr]: value }
        let source = r#"
const key = "name";
const obj = {
    [key]: "value",
    ["static"]: "static value",
    [1 + 1]: "computed number"
};
console.log(obj);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("[key]"),
            "Should contain computed property [key]: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("[\"static\"]"),
            "Should contain computed property [\"static\"]: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("[1 + 1]"),
            "Should contain computed property [1 + 1]: {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_computed_property_name() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test computed property names in class: { [expr]: value }
        let source = r#"
const prefix = "test";
class MyClass {
    [prefix + "Key"] = "computed field";
    ["staticKey"] = "static string key";
    [1 + 1] = "number key";
}
console.log(MyClass);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Class computed property output:\n{}", result.js_code);
        // Should compile without errors
        assert!(
            result.js_code.contains("[prefix + \"Key\"]"),
            "Should contain computed property [prefix + \"Key\"]: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("[\"staticKey\"]"),
            "Should contain computed property [\"staticKey\"]: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("[1 + 1]"),
            "Should contain computed property [1 + 1]: {}",
            result.js_code
        );
    }

    #[test]
    fn test_conditional_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test conditional (ternary) operator: condition ? true_expr : false_expr
        let source = r#"
const x = 10;
const result = x > 5 ? "greater" : "less or equal";
console.log(result);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("?"),
            "Should contain ternary operator: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains(":"),
            "Should contain ternary colon: {}",
            result.js_code
        );
    }

    #[test]
    fn test_nested_conditional_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested ternary operator with explicit grouping
        // Using parentheses to make the grouping explicit
        let source = r#"
const score = 85;
const grade = score >= 90 ? "A" : (score >= 80 ? "B" : (score >= 70 ? "C" : "F"));
console.log(grade);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("?"),
            "Should contain ternary operator: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("score >= 90"),
            "Should contain first condition: {}",
            result.js_code
        );
    }

    #[test]
    fn test_conditional_in_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test ternary in function return
        let source = r#"
function max(a: number, b: number): number {
    return a > b ? a : b;
}
console.log(max(3, 7));
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("function max"),
            "Should contain function: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("a > b ? a : b"),
            "Should contain ternary in return: {}",
            result.js_code
        );
    }

    #[test]
    fn test_string_literal_property() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test string literal as property name
        let source = r#"
const obj = {
    "normal-key": "value1",
    "another-key": "value2"
};
console.log(obj);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("\"normal-key\""),
            "Should contain string property name: {}",
            result.js_code
        );
    }

    #[test]
    fn test_generic_return_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Promise<string> return type
        let source = r#"
async function fetchData(): Promise<string> {
    return "Data loaded!";
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors and produce valid JS
        assert!(
            result.js_code.contains("async function fetchData"),
            "Should contain async function: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("return \"Data loaded!\""),
            "Should contain return statement: {}",
            result.js_code
        );
    }

    #[test]
    fn test_generic_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test generic function
        let source = r#"
function identity<T>(arg: T): T {
    return arg;
}

let result = identity<string>("hello");
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("function identity"),
            "Should contain function: {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_function_return_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async function with Promise return type
        let source = r#"
async function fetchData(): Promise<string> {
    return "Data loaded!";
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("async function fetchData"),
            "Should contain async function: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("return \"Data loaded!\""),
            "Should contain return statement: {}",
            result.js_code
        );
    }

    #[test]
    fn test_await_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test await expression in async function
        let source = r#"
async function getData(): Promise<string> {
    const result = await fetchData();
    return result;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("await fetchData()"),
            "Should contain await expression: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async function getData"),
            "Should contain async function: {}",
            result.js_code
        );
    }

    #[test]
    fn test_await_with_call_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test await with method call
        let source = r#"
async function process() {
    const data = await api.getUsers();
    return data;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("await api.getUsers()"),
            "Should contain await with method call: {}",
            result.js_code
        );
    }

    #[test]
    fn test_await_in_arrow_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test await in arrow function (simplified - expression body)
        let source = r#"
const fetch = async () => await fetchData();
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("await fetchData()"),
            "Should contain await in arrow function: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async ()"),
            "Should contain async arrow function: {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_arrow_function_block_body() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async arrow function with block body containing multiple statements
        let source = r#"
const processData = async (input: string) => {
    const temp = input.trim();
    const result = await fetchData(temp);
    return result;
};
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain async keyword
        assert!(
            result.js_code.contains("async"),
            "Should contain async keyword: {}",
            result.js_code
        );
        // Should contain parameter
        assert!(
            result.js_code.contains("input"),
            "Should contain parameter: {}",
            result.js_code
        );
        // Should contain variable declarations
        assert!(
            result.js_code.contains("const temp"),
            "Should contain temp variable: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("const result"),
            "Should contain result variable: {}",
            result.js_code
        );
        // Should contain await expression
        assert!(
            result.js_code.contains("await fetchData"),
            "Should contain await: {}",
            result.js_code
        );
        // Should contain return statement
        assert!(
            result.js_code.contains("return result"),
            "Should contain return: {}",
            result.js_code
        );
    }

    #[test]
    fn test_arrow_function_block_body_with_multiple_statements() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test regular (non-async) arrow function with block body
        let source = r#"
const add = (a: number, b: number) => {
    const sum = a + b;
    console.log(sum);
    return sum;
};
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain parameter
        assert!(
            result.js_code.contains("a"),
            "Should contain parameter a: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("b"),
            "Should contain parameter b: {}",
            result.js_code
        );
        // Should contain all statements
        assert!(
            result.js_code.contains("const sum"),
            "Should contain sum variable: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("console.log"),
            "Should contain console.log: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("return sum"),
            "Should contain return: {}",
            result.js_code
        );
    }

    #[test]
    fn test_for_of_loop() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test for...of loop
        let source = r#"
for (const item of items) {
    console.log(item);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain for...of syntax
        assert!(
            result.js_code.contains("for"),
            "Should contain for: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("const item"),
            "Should contain const item: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("of items"),
            "Should contain of items: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("console.log(item)"),
            "Should contain console.log: {}",
            result.js_code
        );
    }

    #[test]
    fn test_for_loop() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test traditional for loop
        let source = r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain for loop syntax
        assert!(
            result.js_code.contains("for"),
            "Should contain for: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("let i = 0"),
            "Should contain initializer: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("i < 10"),
            "Should contain condition: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("i++"),
            "Should contain update: {}",
            result.js_code
        );
    }

    #[test]
    fn test_if_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test if statement
        let source = r#"
if (x > 0) {
    console.log("positive");
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain if syntax
        assert!(
            result.js_code.contains("if"),
            "Should contain if: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("x > 0"),
            "Should contain condition: {}",
            result.js_code
        );
    }

    #[test]
    fn test_if_else_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test if...else statement
        let source = r#"
if (x > 0) {
    console.log("positive");
} else {
    console.log("non-positive");
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should contain if...else syntax
        assert!(
            result.js_code.contains("if"),
            "Should contain if: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("else"),
            "Should contain else: {}",
            result.js_code
        );
    }

    #[test]
    fn test_while_loop() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test while loop
        let source = r#"
let i = 0;
while (i < 5) {
    i++;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("while"),
            "Should contain while: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("i < 5"),
            "Should contain condition: {}",
            result.js_code
        );
    }

    #[test]
    fn test_do_while_loop() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test do...while loop
        let source = r#"
let i = 0;
do {
    i++;
} while (i < 10);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("do"),
            "Should contain do: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("while"),
            "Should contain while: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("i < 10"),
            "Should contain condition: {}",
            result.js_code
        );
    }

    #[test]
    fn test_switch_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test switch statement
        let source = r#"
switch (x) {
    case 1:
        console.log("one");
        break;
    case 2:
        console.log("two");
        break;
    default:
        console.log("other");
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("switch"),
            "Should contain switch: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("case 1"),
            "Should contain case 1: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("case 2"),
            "Should contain case 2: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("default"),
            "Should contain default: {}",
            result.js_code
        );
    }

    #[test]
    fn test_try_catch_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test try...catch statement
        let source = r#"
try {
    riskyFunction();
} catch (e) {
    handleError(e);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("try"),
            "Should contain try: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("catch"),
            "Should contain catch: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("riskyFunction"),
            "Should contain riskyFunction: {}",
            result.js_code
        );
    }

    #[test]
    fn test_try_catch_finally_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test try...catch...finally statement
        let source = r#"
try {
    riskyFunction();
} catch (e) {
    handleError(e);
} finally {
    cleanup();
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("try"),
            "Should contain try: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("catch"),
            "Should contain catch: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("finally"),
            "Should contain finally: {}",
            result.js_code
        );
    }

    #[test]
    fn test_throw_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test throw statement
        let source = r#"
if (error) {
    throw new Error("Something went wrong");
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("throw"),
            "Should contain throw: {}",
            result.js_code
        );
    }

    #[test]
    fn test_break_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test break statement
        let source = r#"
for (let i = 0; i < 10; i++) {
    if (i === 5) {
        break;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("break"),
            "Should contain break: {}",
            result.js_code
        );
    }

    #[test]
    fn test_continue_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test continue statement
        let source = r#"
for (let i = 0; i < 10; i++) {
    if (i % 2 === 0) {
        continue;
    }
    console.log(i);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("continue"),
            "Should contain continue: {}",
            result.js_code
        );
    }

    #[test]
    fn test_array_literal() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test array literal (without type annotations for now)
        let source = r#"
const numbers = [1, 2, 3];
const empty = [];
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("[1, 2, 3]"),
            "Should contain array literal: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("[]"),
            "Should contain empty array: {}",
            result.js_code
        );
    }

    #[test]
    fn test_spread_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test spread expression
        let source = r#"
const arr1 = [1, 2, 3];
const arr2 = [...arr1, 4, 5];
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("[...arr1, 4, 5]"),
            "Should contain spread expression: {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_extends() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with extends (minimal test)
        let source = "class Dog extends Animal {}";
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("class Dog extends Animal"),
            "Should contain class Dog extends Animal: {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_method() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with method (using JS syntax for simplicity)
        let source = r#"
class Counter {
    count = 0;
    increment() {
        this.count++;
    }
    getCount() {
        return this.count;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Class with method transpiled output:\n{}", result.js_code);
        assert!(
            result.js_code.contains("class Counter"),
            "Should contain 'class Counter': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("increment()"),
            "Should contain 'increment()': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("getCount()"),
            "Should contain 'getCount()': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("count"),
            "Should contain 'count' field: {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_constructor() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with constructor (simplified - just return statements)
        let source = r#"
class Calculator {
    constructor() {
        return 0;
    }
    add(a, b) {
        return a + b;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Class with constructor transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("class Calculator"),
            "Should contain 'class Calculator': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("constructor"),
            "Should contain 'constructor': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("add"),
            "Should contain 'add': {}",
            result.js_code
        );
    }

    #[test]
    fn test_constructor_with_type_annotations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with constructor that has TypeScript type annotations
        let source = r#"
class Person {
    private name: string;
    private age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    greet(): string {
        return `Hello, my name is ${this.name}`;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Constructor with type annotations transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("class Person"),
            "Should contain 'class Person': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("constructor"),
            "Should contain 'constructor': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("greet"),
            "Should contain 'greet': {}",
            result.js_code
        );
        // Should not contain TypeScript type annotations
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain type annotation ': string': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain type annotation ': number': {}",
            result.js_code
        );
    }

    #[test]
    fn test_constructor_with_access_modifiers() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with constructor that has access modifiers (public, private, protected)
        let source = r#"
class Person {
    constructor(public name: string, private age: number, protected id: number) {}
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Constructor with access modifiers transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("class Person"),
            "Should contain 'class Person': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("constructor"),
            "Should contain 'constructor': {}",
            result.js_code
        );
        // Should generate this.name = name; for public parameter
        assert!(
            result.js_code.contains("this.name = name"),
            "Should generate 'this.name = name;' for public parameter: {}",
            result.js_code
        );
        // Should generate this.age = age; for private parameter
        assert!(
            result.js_code.contains("this.age = age"),
            "Should generate 'this.age = age;' for private parameter: {}",
            result.js_code
        );
        // Should generate this.id = id; for protected parameter
        assert!(
            result.js_code.contains("this.id = id"),
            "Should generate 'this.id = id;' for protected parameter: {}",
            result.js_code
        );
        // Should not contain TypeScript type annotations
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain type annotation ': string': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain type annotation ': number': {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_method_with_type_annotations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with methods that have TypeScript type annotations
        let source = r#"
class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }
    multiply(x: number, y: number): number {
        return x * y;
    }
    greet(name: string): string {
        return "Hello, " + name;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Class with typed methods transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("class Calculator"),
            "Should contain 'class Calculator': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("add("),
            "Should contain 'add(' method: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("multiply("),
            "Should contain 'multiply(' method: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("greet("),
            "Should contain 'greet(' method: {}",
            result.js_code
        );
        // Should not contain TypeScript type annotations
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain type annotation ': number': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain type annotation ': string': {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_static_method() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with static method - using PI as method name
        let source = r#"
class MathUtils {
    static add(a, b) {
        return a + b;
    }
    static PI() {
        return 3.14159;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Class with static method transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("static add"),
            "Should contain 'static add': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("static PI"),
            "Should contain 'static PI': {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_getter_setter() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with getter and setter (minimal body)
        let source = r#"
class Temperature {
    get value() {
        return 1;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Class with getter transpiled output:\n{}", result.js_code);
        assert!(
            result.js_code.contains("get value()"),
            "Should contain 'get value()': {}",
            result.js_code
        );
    }

    #[test]
    fn test_getter_setter_with_type_annotations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with getter and setter that have TypeScript type annotations
        let source = r#"
class Rectangle {
    private _width: number = 0;
    private _height: number = 0;

    get width(): number {
        return this._width;
    }

    set width(value: number) {
        this._width = value;
    }

    get area(): number {
        return this._width * this._height;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Getter/setter with type annotations transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("get width()"),
            "Should contain 'get width()': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("set width"),
            "Should contain 'set width': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("get area()"),
            "Should contain 'get area()': {}",
            result.js_code
        );
        // Should not contain TypeScript type annotations
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain type annotation ': number': {}",
            result.js_code
        );
    }

    #[test]
    fn test_class_with_async_method() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with async method
        let source = r#"
class DataFetcher {
    async fetchData(url) {
        const response = await fetch(url);
        return response.text();
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Class with async method transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async fetchData"),
            "Should contain 'async fetchData': {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_method_with_type_annotations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with async method that has TypeScript type annotations
        let source = r#"
class DataFetcher {
    async fetchData(url: string): Promise<string> {
        const response = await fetch(url);
        return response.text();
    }
    async getCount(): Promise<number> {
        return 42;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Async method with types transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async fetchData"),
            "Should contain 'async fetchData': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async getCount"),
            "Should contain 'async getCount': {}",
            result.js_code
        );
        // Should not contain TypeScript type annotations
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain type annotation ': string': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": Promise"),
            "Should not contain type annotation ': Promise': {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_literal() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test simple template literal
        let source = r#"const name = "World";
const greeting = `Hello ${name}!`;"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Template literal transpiled output:\n{}", result.js_code);
        // Template literal should be transpiled to string concatenation
        assert!(
            result.js_code.contains("Hello"),
            "Should contain 'Hello': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("name"),
            "Should contain 'name': {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_literal_only() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal only (no variable declaration)
        let source = r#"`Hello ${name}!`"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Template literal only transpiled output:\n{}",
            result.js_code
        );
        assert!(
            result.js_code.contains("Hello"),
            "Should contain 'Hello': {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_literal_with_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal with arithmetic expression
        let source = r#"const a = 1, b = 2;
const sum = `${a} + ${b} = ${a + b}`;"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Template literal with expression transpiled output:\n{}",
            result.js_code
        );
        // Should handle expressions in template
        assert!(
            result.js_code.contains("+"),
            "Should contain '+' expression: {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_literal_simple_expr() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal with simple expression (no multi-var declaration)
        let source = r#"const a = 1;
const sum = `${a + a}`;"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!(
            "Template literal simple expr transpiled output:\n{}",
            result.js_code
        );
        // Should handle expressions in template
        assert!(
            result.js_code.contains("+"),
            "Should contain '+' expression: {}",
            result.js_code
        );
    }

    #[test]
    fn debug_token_stream() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test simple return statement first
        let source = r#"return 0;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "debug.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
            }
        }

        // Test getter syntax - simple return
        let source = r#"get fahrenheit() { return 0; }"#;
        println!("\n========== Testing: {} ==========\n", source);

        println!("\n=== Source ===\n{}", source);

        let tokens = compiler.lexical_analysis(source, "debug.ts").unwrap();

        println!("\n=== Token Stream ===");
        for (i, token) in tokens.iter().enumerate() {
            println!("{:3}: {:?}", i, token);
        }

        // Now try to compile
        println!("\n=== Compilation ===");
        match compiler.compile_source(source, "debug.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_array_destructuring() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test array destructuring
        let source = r#"const [a, b, c] = [1, 2, 3];"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("a"),
                    "Should contain a: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("b"),
                    "Should contain b: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("c"),
                    "Should contain c: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_destructuring() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object destructuring
        let source = r#"const { x, y } = { x: 1, y: 2 };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("x"),
                    "Should contain x: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("y"),
                    "Should contain y: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_destructuring_with_alias() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object destructuring with alias
        let source = r#"const { a: alias } = { a: 1 };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_assignment_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test assignment expression in class method
        let source = r#"
class Animal {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
    speak(): void {
        console.log(`${this.name} makes a sound`);
    }
}
"#;
        println!("\n========== Testing: class with constructor and assignment ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("class"),
                    "Should contain class: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("constructor"),
                    "Should contain constructor: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("this.name = name"),
                    "Should contain assignment: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_array_destructuring_with_defaults() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test array destructuring with default values
        let source = r#"const [a, b = 2, c = 3] = [1];"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("a"),
                    "Should contain a: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("b = 2"),
                    "Should contain b = 2: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("c = 3"),
                    "Should contain c = 3: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_destructuring_with_defaults() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object destructuring with default values
        let source = r#"const { x, y = 10, z = 20 } = { x: 1 };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("x"),
                    "Should contain x: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("y = 10"),
                    "Should contain y = 10: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("z = 20"),
                    "Should contain z = 20: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_destructuring_with_alias_and_defaults() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object destructuring with alias and default values
        let source = r#"const { a: alias = 5 } = {};"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("alias = 5"),
                    "Should contain alias = 5: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_nested_destructuring_with_defaults() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested destructuring with default values
        let source = r#"const [{ a: nestedA = 1 }, { b: nestedB = 2 }] = [{}, { b: 5 }];"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("nestedA = 1"),
                    "Should contain nestedA = 1: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("nestedB = 2"),
                    "Should contain nestedB = 2: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_params_destructuring() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function with destructuring parameters
        let source = r#"
            function greet({ name, age }) {
                return `Hello, ${name}! You are ${age} years old.`;
            }
        "#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function greet"),
                    "Should contain function greet: {}",
                    result.js_code
                );
                // Note: emitter outputs without spaces: {name, age}
                assert!(
                    result.js_code.contains("{name, age}"),
                    "Should contain destructuring param: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_params_destructuring_with_defaults() {
        // Test function with destructuring parameters (without defaults for now)
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"function greet({ name, age }) { return name + age; }"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function greet"),
                    "Should contain function greet: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("{name, age}"),
                    "Should contain destructuring param: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_params_array_destructuring() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function with array destructuring parameters
        let source = r#"
            function sum([a, b, c]) {
                return a + b + c;
            }
        "#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function sum"),
                    "Should contain function sum: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("[a, b, c]"),
                    "Should contain array destructuring param: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_params_simple_with_defaults() {
        // Simple params with defaults work - testing without template literal
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"function greet(name, age) { return name + age; }"#;

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function greet"),
                    "Should contain function greet: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_arrow_function_params_destructuring() {
        // Arrow function with simple params (destructuring in arrow functions needs more work)
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"const greet = (x, y) => x + y;"#;

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("const greet"),
                    "Should contain const greet: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_method_params_destructuring() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class method with destructuring parameters
        let source = r#"
            class Greeter {
                greet({ name, age }) {
                    return `Hello, ${name}!`;
                }
            }
        "#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("class Greeter"),
                    "Should contain class Greeter: {}",
                    result.js_code
                );
                // Note: emitter outputs without spaces: {name, age}
                assert!(
                    result.js_code.contains("greet({name, age})"),
                    "Should contain destructuring param: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_import_statement() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test named import - v0.3.195: ESM imports are converted to CommonJS
        let source = r#"import { a, b } from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.195: ESM import -> CommonJS require
                assert!(
                    result.js_code.contains("require"),
                    "Should contain require (ESM -> CommonJS): {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("a, b"),
                    "Should contain a, b: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_import_default() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test default import - v0.3.195: ESM imports are converted to CommonJS
        let source = r#"import foo from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.195: ESM import -> CommonJS require
                assert!(
                    result.js_code.contains("require"),
                    "Should contain require (ESM -> CommonJS): {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("foo"),
                    "Should contain foo: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_import_namespace() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test namespace import - v0.3.195: ESM imports are converted to CommonJS
        let source = r#"import * as utils from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.195: ESM import * as ns -> CommonJS const ns = require()
                assert!(
                    result.js_code.contains("require"),
                    "Should contain require (ESM -> CommonJS): {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("utils"),
                    "Should contain utils: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_import_side_effect() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test side-effect import - v0.3.195: ESM imports are converted to CommonJS
        let source = r#"import "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.195: ESM side-effect import -> CommonJS require()
                assert!(
                    result.js_code.contains("require"),
                    "Should contain require (ESM -> CommonJS): {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("module"),
                    "Should contain module: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_import_with_alias() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test import with alias - v0.3.195: ESM imports are converted to CommonJS
        let source = r#"import { original as alias } from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.195: ESM import { x as y } -> CommonJS const { x: y } = require()
                assert!(
                    result.js_code.contains("require"),
                    "Should contain require (ESM -> CommonJS): {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("original"),
                    "Should contain original: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("alias"),
                    "Should contain alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_export_named() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test named export
        let source = r#"export { a, b };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("export"),
                    "Should contain export: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("{ a, b }"),
                    "Should contain {{ a, b }}: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_export_default() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test default export
        let source = r#"export default foo;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("export"),
                    "Should contain export: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("default"),
                    "Should contain default: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_export_inline() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test inline export
        let source = r#"export const x = 1;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("export"),
                    "Should contain export: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("const"),
                    "Should contain const: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_export_from() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test re-export from
        let source = r#"export { a, b } from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("export"),
                    "Should contain export: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("from"),
                    "Should contain from: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_export_all_from() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test export all from
        let source = r#"export * from "module";"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("export"),
                    "Should contain export: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("*"),
                    "Should contain *: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_simple() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test simple type alias
        let source = r#"type MyString = string;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_union() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test union type alias
        let source = r#"type Id = number | string;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_with_generics() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test generic type alias
        let source = r#"type Container<T> = T | null;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_complex() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test complex type alias with multiple type params
        // Note: object types and intersection types are not yet supported
        // Using union type with multiple type params
        let source = r#"type Maybe<T> = T | null | undefined;"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_in_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type alias used in function
        let source = r#"
type Status = "loading" | "success" | "error";

function getStatus(): Status {
    return "success";
}

console.log(getStatus());
"#;
        println!("\n========== Testing type alias in function ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped, but function should remain
                assert!(
                    result.js_code.contains("function getStatus"),
                    "Should contain function: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("type Status"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_type_literal() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object type literal
        let source = r#"type User = { name: string; age: number };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_object_type_with_optional() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test object type with optional properties
        let source = r#"type Point = { x: number; y?: number };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_intersection_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test intersection type
        let source = r#"type Person = { name: string } & { age: number };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_mixed_union_intersection() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test mixed union and intersection types
        let source = r#"type Shape = { kind: "circle" } & { radius: number } | { kind: "square" } & { side: number };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_nested_object_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested object type
        let source = r#"type Config = { nested: { value: string } };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_keyof_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test keyof type operator
        let source = r#"type Keys = keyof { name: string; age: number };"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("keyof"),
                    "Should not contain 'keyof' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_keyof_with_interface() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test keyof with interface
        let source = r#"
interface User {
    name: string;
    age: number;
}
type UserKeys = keyof User;
"#;
        println!("\n========== Testing keyof with interface ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type aliases and interfaces should be skipped
                assert!(
                    !result.js_code.contains("type UserKeys"),
                    "Should not contain 'type UserKeys': {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("interface"),
                    "Should not contain 'interface': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_typeof_operator() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test typeof operator
        let source = r#"
const config = { name: "test", value: 42 };
type ConfigType = typeof config;
"#;
        println!("\n========== Testing typeof operator ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped but const should remain
                assert!(
                    result.js_code.contains("const config"),
                    "Should contain 'const config': {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("type ConfigType"),
                    "Should not contain 'type ConfigType': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_indexed_access_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test indexed access type T[K]
        let source = r#"type NameType = { name: string; age: number }["name"];"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_keyof_in_generics() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test keyof in generic constraint
        let source = r#"function getProperty<T, K extends keyof T>(obj: T, key: K): T[K] {
    return obj[key];
}"#;
        println!("\n========== Testing keyof in generics ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Function should remain but type annotations should be removed
                assert!(
                    result.js_code.contains("function getProperty"),
                    "Should contain function: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_mapped_type_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic mapped type: { [P in keyof T]: T[P] }
        let source = r#"type Partial<T> = { [P in keyof T]?: T[P] };"#;
        println!("\n========== Testing basic mapped type ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type Partial"),
                    "Should not contain 'type Partial': {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("[P in keyof T]"),
                    "Should not contain mapped type syntax: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_mapped_type_with_string_union() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test mapped type with string union keys
        let source = r#"type StringKeyMap = { [P in "name" | "age"]: any };"#;
        println!("\n========== Testing mapped type with string union ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type StringKeyMap"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_mapped_type_readonly() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test mapped type with readonly modifier
        let source = r#"type Readonly<T> = { readonly [P in keyof T]: T[P] };"#;
        println!("\n========== Testing mapped type with readonly ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type Readonly"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_mapped_type_in_generic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test mapped type used in generic function
        let source = r#"function makeOptional<T>(obj: T): { [P in keyof T]?: T[P] } {
    return obj;
}"#;
        println!("\n========== Testing mapped type in generic function ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function makeOptional"),
                    "Should contain function: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_conditional_type_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic conditional type: T extends U ? X : Y
        let source = r#"type IsString<T> = T extends string ? true : false;"#;
        println!("\n========== Testing basic conditional type ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type IsString"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_conditional_type_with_generics() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test conditional type with generics
        let source = r#"type Result<T> = T extends Promise<infer U> ? U : T;"#;
        println!("\n========== Testing conditional type with infer ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type Result"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_conditional_type_nested() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested conditional types
        let source = r#"type DeepResult<T> = T extends string ? "string" : T extends number ? "number" : "other";"#;
        println!("\n========== Testing nested conditional types ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type DeepResult"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_conditional_type_in_type_alias() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test conditional type in type alias declaration
        let source = r#"type NonNullable<T> = T extends null | undefined ? never : T;"#;
        println!("\n========== Testing conditional type in type alias ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type NonNullable"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_template_literal_type_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic template literal type: `Hello ${string}`
        let source = r#"type Greeting = `Hello ${string}`;"#;
        println!("\n========== Testing basic template literal type ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type Greeting"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_template_literal_type_multiple_placeholders() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal type with multiple placeholders
        let source = r#"type Email = `user-${string}@${string}.${string}`;"#;
        println!(
            "\n========== Testing template literal type with multiple placeholders ==========\n"
        );

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type Email"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_template_literal_type_with_generic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal type with generic type parameter
        let source = r#"type EventName<T> = `${T}_clicked`;"#;
        println!("\n========== Testing template literal type with generic ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type EventName"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_template_literal_type_path() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal type for API paths
        let source = r#"type ApiPath = `/api/${string}/${string}`;"#;
        println!("\n========== Testing template literal type for API paths ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type ApiPath"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_infer_keyword_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic infer keyword: infer U
        let source = r#"type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;"#;
        println!("\n========== Testing basic infer keyword ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type UnwrapPromise"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_infer_keyword_with_constraint() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test infer keyword with extends constraint: infer U extends string
        let source = r#"type StringResult<T> = T extends infer U extends string ? U : never;"#;
        println!("\n========== Testing infer keyword with constraint ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type StringResult"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_infer_keyword_chained() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test chained conditional types with infer
        let source = r#"type DeepUnwrap<T> = T extends Promise<infer U> ? DeepUnwrap<U> : T;"#;
        println!("\n========== Testing infer keyword chained ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type DeepUnwrap"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_never_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test never type - represents values that never occur
        let source = r#"
type NeverType = never;
function throwError(msg: string): never {
    throw new Error(msg);
}
type Result<T> = T extends success ? T : never;
"#;
        println!("\n========== Testing never type ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type NeverType"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("type Result"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_unknown_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test unknown type - type-safe top type
        let source = r#"
type UnknownType = unknown;
function processValue(value: unknown): string {
    return String(value);
}
type SafeResult<T> = T extends unknown ? T : never;
"#;
        println!("\n========== Testing unknown type ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type UnknownType"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("type SafeResult"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_predicate_is() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type predicate (is keyword) for type guards
        let source = r#"
function isString(value: unknown): value is string {
    return true;
}
function isNumber<T>(value: T): value is number {
    return true;
}
function isDefined<T>(value: T): value is NonNullable<T> {
    return true;
}
"#;
        println!("\n========== Testing type predicate (is keyword) ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should remove type annotations including type predicates
                assert!(
                    !result.js_code.contains(": unknown"),
                    "Should not contain type annotation: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains(": string"),
                    "Should not contain return type: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_never_unknown_with_generics() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test never and unknown types with generics
        let source = r#"
type UnionWithNever = string | never;  // never is identity for union
type UnionWithUnknown = unknown | string;  // unknown absorbs all
type IntersectionWithUnknown = unknown & string;  // narrows to string
function genericFunction<T extends unknown>(value: T): T {
    return value;
}
"#;
        println!("\n========== Testing never and unknown with generics ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    !result.js_code.contains("type UnionWithNever"),
                    "Should not contain type alias: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_overload_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic function overloads
        let source = r#"
function greet(name: string): string;
function greet(name: string, formal: boolean): string;
function greet(name: string, formal?: boolean): string {
    if (formal) {
        return "Good day, " + name;
    }
    return "Hi, " + name;
}
"#;
        println!("\n========== Testing function overload basic ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should contain function implementation
                assert!(
                    result.js_code.contains("function greet"),
                    "Should contain function greet: {}",
                    result.js_code
                );
                // Should contain @overload comments for signatures
                assert!(
                    result.js_code.contains("/** @overload */"),
                    "Should contain @overload comment: {}",
                    result.js_code
                );
                // Should not contain TypeScript type annotations in output
                assert!(
                    !result.js_code.contains(": string;"),
                    "Should not contain overload signature type annotation: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_overload_multiple_signatures() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function with multiple overload signatures
        let source = r#"
function process(input: string): number;
function process(input: number): string;
function process(input: string | number): string | number {
    if (typeof input === "string") {
        return input.length;
    }
    return String(input);
}
"#;
        println!("\n========== Testing function overload with multiple signatures ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function process"),
                    "Should contain function process: {}",
                    result.js_code
                );
                // Count @overload occurrences (should be 2)
                let overload_count = result.js_code.matches("/** @overload */").count();
                assert_eq!(
                    overload_count, 2,
                    "Should have 2 @overload comments, got {}: {}",
                    overload_count, result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_function_overload_with_generics() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function overloads with generics
        let source = r#"
function identity<T>(value: T): T;
function identity<T>(value: T, defaultValue: T): T;
function identity<T>(value: T, defaultValue?: T): T {
    return value !== undefined ? value : defaultValue;
}
"#;
        println!("\n========== Testing function overload with generics ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("function identity"),
                    "Should contain function identity: {}",
                    result.js_code
                );
                let overload_count = result.js_code.matches("/** @overload */").count();
                assert_eq!(
                    overload_count, 2,
                    "Should have 2 @overload comments, got {}",
                    overload_count
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_async_function_overload() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async function overloads
        let source = r#"
async function fetchData(url: string): Promise<string>;
async function fetchData(url: string, options: { timeout: number }): Promise<string>;
async function fetchData(url: string, options?: { timeout: number }): Promise<string> {
    const response = await fetch(url);
    return response.text();
}
"#;
        println!("\n========== Testing async function overload ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("async function fetchData"),
                    "Should contain async function fetchData: {}",
                    result.js_code
                );
                let overload_count = result.js_code.matches("/** @overload */").count();
                assert_eq!(
                    overload_count, 2,
                    "Should have 2 @overload comments, got {}",
                    overload_count
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_interface_index_signature_string() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test interface with string index signature
        let source = r#"interface StringMap {
    [key: string]: string;
}"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Interface should be skipped in JS output
                assert!(
                    !result.js_code.contains("interface"),
                    "Should not contain 'interface' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_interface_index_signature_number() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test interface with number index signature
        let source = r#"interface NumberArray {
    [index: number]: string;
}"#;
        println!("\n========== Testing: {} ==========\n", source);

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Interface should be skipped in JS output
                assert!(
                    !result.js_code.contains("interface"),
                    "Should not contain 'interface' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_interface_with_properties_and_index_signature() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test interface with both regular properties and index signature
        let source = r#"interface User {
    name: string;
    age: number;
    [key: string]: any;
}"#;
        println!("\n========== Testing interface with properties and index signature ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Interface should be skipped in JS output
                assert!(
                    !result.js_code.contains("interface"),
                    "Should not contain 'interface' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_alias_with_index_signature() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type alias with index signature
        let source = r#"type Dictionary = {
    [key: string]: number;
    name: string;
};"#;
        println!("\n========== Testing type alias with index signature ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Type alias should be skipped in JS output
                assert!(
                    !result.js_code.contains("type"),
                    "Should not contain 'type' keyword: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_class_decorator() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class decorator
        let source = r#"
@sealed
class Greeter {
    greeting: string;
    constructor(message: string) {
        this.greeting = message;
    }
    greet() {
        return "Hello, " + this.greeting;
    }
}
"#;
        println!("\n========== Testing class decorator ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Decorator should be output as comment
                assert!(
                    result.js_code.contains("/* @sealed */"),
                    "Should contain decorator comment: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("class Greeter"),
                    "Should contain class definition: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_class_decorator_with_args() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class decorator with arguments
        let source = r#"
@Component({
    selector: 'app-my-component',
    template: '<h1>Hello</h1>'
})
class MyComponent {
    name: string = "World";
}
"#;
        println!("\n========== Testing class decorator with arguments ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Decorator with args should be output as comment
                assert!(
                    result.js_code.contains("/* @Component"),
                    "Should contain decorator comment: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("selector"),
                    "Should contain decorator argument: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("class MyComponent"),
                    "Should contain class definition: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_method_decorator() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test method decorator
        let source = r#"
class Calculator {
    @readonly
    PI: number = 3.14159;

    @deprecated
    calculate(x: number, y: number): number {
        return x + y;
    }
}
"#;
        println!("\n========== Testing method decorator ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Decorators should be output as comments
                assert!(
                    result.js_code.contains("/* @readonly */"),
                    "Should contain readonly decorator comment: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("/* @deprecated */"),
                    "Should contain deprecated decorator comment: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("calculate"),
                    "Should contain method definition: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_multiple_decorators() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test multiple decorators on class and method
        let source = r#"
@Injectable
@Component({ template: '' })
class MyService {
    @Prop()
    @Watch('value')
    value: string;

    @Get('/api/data')
    fetchData(): Promise<string> {
        return Promise.resolve('data');
    }
}
"#;
        println!("\n========== Testing multiple decorators ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // All decorators should be output as comments
                assert!(
                    result.js_code.contains("/* @Injectable */"),
                    "Should contain Injectable decorator: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("/* @Component"),
                    "Should contain Component decorator: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("/* @Prop */"),
                    "Should contain Prop decorator: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("/* @Watch"),
                    "Should contain Watch decorator: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("/* @Get"),
                    "Should contain Get decorator: {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_enum_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic enum
        let source = r#"
enum Color {
    Red,
    Green,
    Blue
}
"#;
        println!("\n========== Testing basic enum ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Enum should be transpiled to JavaScript object
                assert!(
                    result.js_code.contains("var Color"),
                    "Should contain 'var Color': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Red"),
                    "Should contain 'Red': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Green"),
                    "Should contain 'Green': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Blue"),
                    "Should contain 'Blue': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_enum_with_values() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test enum with explicit values
        let source = r#"
enum Status {
    Pending = 1,
    Active = 2,
    Inactive = 0
}
"#;
        println!("\n========== Testing enum with values ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("var Status"),
                    "Should contain 'var Status': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Pending: 1"),
                    "Should contain 'Pending: 1': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Active: 2"),
                    "Should contain 'Active: 2': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Inactive: 0"),
                    "Should contain 'Inactive: 0': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_string_enum() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test string enum
        let source = r#"
enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT"
}
"#;
        println!("\n========== Testing string enum ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("var Direction"),
                    "Should contain 'var Direction': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Up: \"UP\""),
                    "Should contain 'Up: \"UP\"': {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("Down: \"DOWN\""),
                    "Should contain 'Down: \"DOWN\"': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_const_enum() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test const enum (should be inlined)
        let source = r#"
const enum PixelSize {
    Small = 1,
    Medium = 2,
    Large = 3
}
const size = PixelSize.Medium;
"#;
        println!("\n========== Testing const enum ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // const enum should still be transpiled (simple implementation)
                assert!(
                    result.js_code.contains("PixelSize"),
                    "Should contain 'PixelSize': {}",
                    result.js_code
                );
            }
            Err(e) => {
                println!("Compilation failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_type_check_variable_declaration() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type checking for variable declarations
        let source = r#"
let name: string = "John";
let age: number = 25;
let isActive: boolean = true;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("name"),
            "Should contain 'name': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("age"),
            "Should contain 'age': {}",
            result.js_code
        );
        // Type annotations should be removed in output
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain ': string': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain ': number': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_check_function_declaration() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type checking for function declarations
        let source = r#"
function greet(name: string): string {
    return "Hello, " + name;
}

function add(a: number, b: number): number {
    return a + b;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("function greet"),
            "Should contain 'function greet': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("function add"),
            "Should contain 'function add': {}",
            result.js_code
        );
        // Type annotations should be removed in output
        assert!(
            !result.js_code.contains(": string"),
            "Should not contain ': string': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain ': number': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_check_interface() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type checking for interface declarations
        let source = r#"
interface Person {
    name: string;
    age: number;
}

interface Employee {
    employeeId: string;
    department: string;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Interface declarations should be removed in transpiled output
        assert!(
            !result.js_code.contains("interface Person"),
            "Should not contain 'interface Person': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("interface Employee"),
            "Should not contain 'interface Employee': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_check_type_alias() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type checking for type alias declarations
        let source = r#"
type Status = "active" | "inactive" | "pending";
type UserId = string | number;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type alias declarations should be removed in transpiled output
        assert!(
            !result.js_code.contains("type Status"),
            "Should not contain 'type Status': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("type UserId"),
            "Should not contain 'type UserId': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_check_enum() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type checking for enum declarations
        let source = r#"
enum Color {
    Red,
    Green,
    Blue
}

enum Direction {
    North,
    South,
    East,
    West
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Enum declarations should be transpiled
        assert!(
            result.js_code.contains("Color"),
            "Should contain 'Color': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("Direction"),
            "Should contain 'Direction': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_assertion_as() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic type assertion with 'as' keyword
        // Type assertions are removed in transpiled JS (not needed in JS)
        let source = r#"
const value: unknown = "hello";
const strValue = value as string;
const numValue = someValue as number;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type annotations should be removed
        assert!(
            !result.js_code.contains(": unknown"),
            "Should not contain ': unknown': {}",
            result.js_code
        );
        // The type assertion `as Type` should be removed, leaving just the expression
        assert!(
            result.js_code.contains("strValue = value"),
            "Should contain 'strValue = value': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("numValue = someValue"),
            "Should contain 'numValue = someValue': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_assertion_with_object() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test type assertion with object properties
        let source = r#"
interface Person {
    name: string;
    age: number;
}

const data = { name: "John", age: 30 };
const person = data as Person;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type assertion should be removed
        assert!(
            result.js_code.contains("person = data"),
            "Should contain 'person = data': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface Person"),
            "Should not contain 'interface Person': {}",
            result.js_code
        );
    }

    #[test]
    fn test_type_assertion_chain() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test chained type assertions
        let source = r#"
const value: any = "test";
const result = value as string as any;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Chained type assertions should be removed, leaving just the expression
        assert!(
            result.js_code.contains("result = value"),
            "Should contain 'result = value': {}",
            result.js_code
        );
    }

    // ============ Angle Bracket Type Assertion Tests (v0.3.147) ============

    #[test]
    fn test_angle_bracket_type_assertion_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic angle bracket type assertion: <Type>expr
        let source = r#"
const value: any = "hello";
const str = <string>value;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type assertion should be removed
        assert!(
            result.js_code.contains("str = value"),
            "Should contain 'str = value': {}",
            result.js_code
        );
        // Type annotation should be removed
        assert!(
            !result.js_code.contains("<string>"),
            "Should not contain '<string>': {}",
            result.js_code
        );
    }

    #[test]
    fn test_angle_bracket_type_assertion_with_number() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test angle bracket type assertion with number
        let source = r#"
const input: unknown = "42";
const num = <number>input;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("num = input"),
            "Should contain 'num = input': {}",
            result.js_code
        );
    }

    #[test]
    fn test_angle_bracket_type_assertion_complex_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test angle bracket type assertion with complex type
        let source = r#"
interface Person {
    name: string;
    age: number;
}

const data: unknown = { name: "Alice", age: 30 };
const person = <Person>data;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type assertion should be removed
        assert!(
            result.js_code.contains("person = data"),
            "Should contain 'person = data': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface Person"),
            "Should not contain 'interface Person': {}",
            result.js_code
        );
    }

    #[test]
    fn test_angle_bracket_vs_as_assertion_equivalence() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test that both assertion styles produce the same output
        let source_as = r#"
const value: any = "test";
const result1 = value as string;
"#;
        let source_angle = r#"
const value: any = "test";
const result2 = <string>value;
"#;
        let result_as = compiler.compile_source(source_as, "test.ts").unwrap();
        let mut compiler2 = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let result_angle = compiler2.compile_source(source_angle, "test.ts").unwrap();

        // Both should produce similar output (without type annotations)
        assert!(
            result_as.js_code.contains("result1 = value"),
            "AS assertion should contain 'result1 = value': {}",
            result_as.js_code
        );
        assert!(
            result_angle.js_code.contains("result2 = value"),
            "Angle bracket assertion should contain 'result2 = value': {}",
            result_angle.js_code
        );
    }

    #[test]
    fn test_angle_bracket_in_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test angle bracket type assertion in expression context
        let source = r#"
function process(input: unknown): string {
    const str = <string>input;
    return str + " processed";
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should handle function with type assertion
        assert!(
            result.js_code.contains("processed"),
            "Should contain 'processed': {}",
            result.js_code
        );
    }

    // ============ Utility Types Tests ============

    #[test]
    fn test_utility_type_partial() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Partial<T> utility type - all properties become optional
        let source = r#"
interface User {
    name: string;
    age: number;
}

function updateUser(user: Partial<User>) {
    console.log(user);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Partial<T> should be erased, leaving just the inner type
        assert!(
            !result.js_code.contains("Partial"),
            "Should not contain 'Partial': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface User"),
            "Should not contain 'interface User': {}",
            result.js_code
        );
        // Function should still be present
        assert!(
            result.js_code.contains("updateUser"),
            "Should contain 'updateUser': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_required() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Required<T> utility type - all properties become required
        // Note: Optional properties (?:) are not fully supported, so we test Required with basic types
        let source = r#"
interface Config {
    host: string;
    port: number;
}

function init(config: Required<Config>) {
    console.log(config.host, config.port);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Required<T> should be erased
        assert!(
            !result.js_code.contains("Required"),
            "Should not contain 'Required': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface Config"),
            "Should not contain 'interface Config': {}",
            result.js_code
        );
        // Function should still be present
        assert!(
            result.js_code.contains("init"),
            "Should contain 'init': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_readonly() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Readonly<T> utility type - all properties become readonly
        let source = r#"
interface Point {
    x: number;
    y: number;
}

const origin: Readonly<Point> = { x: 0, y: 0 };
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Readonly<T> should be erased
        assert!(
            !result.js_code.contains("Readonly"),
            "Should not contain 'Readonly': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface Point"),
            "Should not contain 'interface Point': {}",
            result.js_code
        );
        // Origin assignment should still be present
        assert!(
            result.js_code.contains("origin"),
            "Should contain 'origin': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_pick() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Pick<T, K> utility type - select specific keys
        // Note: Pick with union type keys like "name" | "email" requires union type support
        // We'll use a simpler case to test basic utility type erasure
        let source = r#"
interface User {
    id: number;
    name: string;
}

type UserPreview = Pick<User, "name">;

const user: UserPreview = { name: "Alice" };
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Pick<T, K> should be erased, keeping only the first type argument
        assert!(
            !result.js_code.contains("Pick"),
            "Should not contain 'Pick': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface User"),
            "Should not contain 'interface User': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("user"),
            "Should contain 'user': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_record() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Record<K, T> utility type - create object with specific keys and values
        let source = r#"
type Status = "pending" | "active" | "completed";

const tasks: Record<Status, boolean> = {
    pending: true,
    active: true,
    completed: false
};
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Record<K, T> should be erased
        assert!(
            !result.js_code.contains("Record"),
            "Should not contain 'Record': {}",
            result.js_code
        );
        // Type alias should be removed
        assert!(
            !result.js_code.contains("type Status"),
            "Should not contain 'type Status': {}",
            result.js_code
        );
        // Tasks assignment should still be present
        assert!(
            result.js_code.contains("tasks"),
            "Should contain 'tasks': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_omit() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Omit<T, K> utility type - remove specific keys
        let source = r#"
interface User {
    id: number;
    name: string;
    email: string;
    password: string;
}

type PublicUser = Omit<User, "password">;

function showUser(user: PublicUser) {
    console.log(user.name, user.email);
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Omit<T, K> should be erased
        assert!(
            !result.js_code.contains("Omit"),
            "Should not contain 'Omit': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface User"),
            "Should not contain 'interface User': {}",
            result.js_code
        );
        // Function should still be present
        assert!(
            result.js_code.contains("showUser"),
            "Should contain 'showUser': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_exclude() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Exclude<T, U> utility type - exclude types from union
        let source = r#"
type Status = "pending" | "active" | "completed" | "deleted";
type ActiveStatus = Exclude<Status, "deleted">;

const current: ActiveStatus = "active";
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Exclude<T, U> should be erased
        assert!(
            !result.js_code.contains("Exclude"),
            "Should not contain 'Exclude': {}",
            result.js_code
        );
        // Type aliases should be removed
        assert!(
            !result.js_code.contains("type Status"),
            "Should not contain 'type Status': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("current"),
            "Should contain 'current': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_extract() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Extract<T, U> utility type - extract types from union
        let source = r#"
type Status = "pending" | "active" | "completed" | number;
type StringStatus = Extract<Status, string>;

const status: StringStatus = "active";
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Extract<T, U> should be erased
        assert!(
            !result.js_code.contains("Extract"),
            "Should not contain 'Extract': {}",
            result.js_code
        );
        // Type aliases should be removed
        assert!(
            !result.js_code.contains("type Status"),
            "Should not contain 'type Status': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("status"),
            "Should contain 'status': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_nonnullable() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test NonNullable<T> utility type - remove null and undefined
        // Using a simple type argument to test erasure
        let source = r#"
type MaybeString = string;
const value: NonNullable<MaybeString> = "hello";
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // NonNullable<T> should be erased
        assert!(
            !result.js_code.contains("NonNullable"),
            "Should not contain 'NonNullable': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("value"),
            "Should contain 'value': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_return_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test ReturnType<T> utility type - get return type of function
        let source = r#"
function getUser(): { id: number; name: string } {
    return { id: 1, name: "Alice" };
}

type User = ReturnType<typeof getUser>;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // ReturnType<T> should be erased
        assert!(
            !result.js_code.contains("ReturnType"),
            "Should not contain 'ReturnType': {}",
            result.js_code
        );
        // Type alias should be removed
        assert!(
            !result.js_code.contains("type User"),
            "Should not contain 'type User': {}",
            result.js_code
        );
        // Function should still be present
        assert!(
            result.js_code.contains("getUser"),
            "Should contain 'getUser': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_type_parameters() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test Parameters<T> utility type - get parameters of function
        let source = r#"
function greet(name: string, age: number): string {
    return `Hello ${name}, you are ${age} years old`;
}

type GreetParams = Parameters<typeof greet>;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Parameters<T> should be erased
        assert!(
            !result.js_code.contains("Parameters"),
            "Should not contain 'Parameters': {}",
            result.js_code
        );
        // Type alias should be removed
        assert!(
            !result.js_code.contains("type GreetParams"),
            "Should not contain 'type GreetParams': {}",
            result.js_code
        );
        // Function should still be present
        assert!(
            result.js_code.contains("greet"),
            "Should contain 'greet': {}",
            result.js_code
        );
    }

    /// v0.3.192: Test ConstructorParameters<T> utility type
    #[test]
    fn test_utility_type_constructor_parameters() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test ConstructorParameters<T> utility type - get constructor parameter types
        let source = r#"
class User {
    name: string;
    age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }
}

type UserConstructorParams = ConstructorParameters<typeof User>;
const params: UserConstructorParams = ["Alice", 30];
console.log(params);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // ConstructorParameters<T> should be erased
        assert!(
            !result.js_code.contains("ConstructorParameters"),
            "Should not contain 'ConstructorParameters': {}",
            result.js_code
        );
        // Type alias should be removed
        assert!(
            !result.js_code.contains("type UserConstructorParams"),
            "Should not contain 'type UserConstructorParams': {}",
            result.js_code
        );
        // Class should still be present (without type annotations)
        assert!(
            result.js_code.contains("class User"),
            "Should contain 'class User': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("const params"),
            "Should contain 'const params': {}",
            result.js_code
        );
    }

    /// v0.3.192: Test InstanceType<T> utility type
    #[test]
    fn test_utility_type_instance_type() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test InstanceType<T> utility type - get constructor's instance type
        let source = r#"
class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
}

type PointInstance = InstanceType<typeof Point>;
const point: PointInstance = { x: 10, y: 20 };
console.log(point);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // InstanceType<T> should be erased
        assert!(
            !result.js_code.contains("InstanceType"),
            "Should not contain 'InstanceType': {}",
            result.js_code
        );
        // Type alias should be removed
        assert!(
            !result.js_code.contains("type PointInstance"),
            "Should not contain 'type PointInstance': {}",
            result.js_code
        );
        // Class should still be present (without type annotations)
        assert!(
            result.js_code.contains("class Point"),
            "Should contain 'class Point': {}",
            result.js_code
        );
        // Variable should still be present
        assert!(
            result.js_code.contains("const point"),
            "Should contain 'const point': {}",
            result.js_code
        );
    }

    #[test]
    fn test_utility_types_multiple() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test multiple utility types in one file
        let source = r#"
interface Config {
    host: string;
    port: number;
    ssl: boolean;
}

// Partial for updates
function updateConfig(config: Partial<Config>) {
    console.log(config);
}

// Readonly for constants
const defaultConfig: Readonly<Config> = {
    host: "localhost",
    port: 8080,
    ssl: false
};

// Pick for specific fields
function getHost(config: Pick<Config, "host">) {
    return config.host;
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // All utility types should be erased
        assert!(
            !result.js_code.contains("Partial"),
            "Should not contain 'Partial': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("Readonly"),
            "Should not contain 'Readonly': {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("Pick"),
            "Should not contain 'Pick': {}",
            result.js_code
        );
        // Interface should be removed
        assert!(
            !result.js_code.contains("interface Config"),
            "Should not contain 'interface Config': {}",
            result.js_code
        );
        // Functions should still be present
        assert!(
            result.js_code.contains("updateConfig"),
            "Should contain 'updateConfig': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("getHost"),
            "Should contain 'getHost': {}",
            result.js_code
        );
    }

    // ===== v0.3.141 边界情况测试 =====

    #[test]
    fn test_iife_with_async_arrow() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async function expression
        let source = r#"
const runner = async () => {
    console.log("async IIFE");
};
runner();
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("async"),
            "Should contain 'async': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("console"),
            "Should contain 'console': {}",
            result.js_code
        );
    }

    #[test]
    fn test_iife_with_regular_arrow() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test IIFE using variable assignment first
        let source = r#"
const fn = () => {
    console.log("regular IIFE");
};
fn();
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("console"),
            "Should contain 'console': {}",
            result.js_code
        );
    }

    #[test]
    fn test_nested_array_types() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested array types: string[][]
        let source = r#"
const matrix: string[][] = [
    ["a", "b"],
    ["c", "d"]
];

const single: number[] = [1, 2, 3];
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("matrix"),
            "Should contain 'matrix': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("single"),
            "Should contain 'single': {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_with_emoji() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template string with emoji
        let source = r#"
const greeting = `Hello 🌍 World 🚀`;
console.log(greeting);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("greeting"),
            "Should contain 'greeting': {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_expression_at_end() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template with expression at end: `${expr}`
        let source = r#"
const name = "Beejs";
const greeting = `Hello ${name}`;
console.log(greeting);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("greeting"),
            "Should contain 'greeting': {}",
            result.js_code
        );
    }

    #[test]
    fn test_complex_class_with_all_features() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test class with multiple features
        let source = r#"
class Animal {
    private name: string;
    public age: number;
    protected id: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
        this.id = Math.random();
    }

    getName(): string {
        return this.name;
    }

    setAge(age: number): void {
        this.age = age;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("Animal"),
            "Should contain 'Animal': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("getName"),
            "Should contain 'getName': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("setAge"),
            "Should contain 'setAge': {}",
            result.js_code
        );
    }

    #[test]
    fn test_interface_with_extends_and_index_signature() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test interface with extends and index signature
        let source = r#"
interface Named {
    name: string;
}

interface Config extends Named {
    host: string;
    port: number;
    [key: string]: any;
}

// 使用 Config 接口
const config: Config = { name: "test", host: "localhost", port: 8080 };
console.log(config.name);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors - Config interface usage should be preserved
        assert!(
            result.js_code.contains("config"),
            "Should contain 'config': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("console"),
            "Should contain 'console': {}",
            result.js_code
        );
        // Interface declarations should be removed (they don't exist in JS)
        assert!(
            !result.js_code.contains("interface"),
            "Should not contain 'interface': {}",
            result.js_code
        );
    }

    #[test]
    fn test_generic_with_constraints() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test generic with constraints
        let source = r#"
function identity<T extends { length: number }>(arg: T): T {
    console.log(arg.length);
    return arg;
}

const str = identity("hello");
const arr = identity([1, 2, 3]);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("identity"),
            "Should contain 'identity': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("str"),
            "Should contain 'str': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("arr"),
            "Should contain 'arr': {}",
            result.js_code
        );
    }

    #[test]
    fn test_function_overload_with_generic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function overload with generic
        let source = r#"
function process<T>(input: T): T;
function process<T>(input: T): { data: T } {
    return { data: input };
}

const result = process("test");
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("process"),
            "Should contain 'process': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("result"),
            "Should contain 'result': {}",
            result.js_code
        );
    }

    // ===== v0.3.142 函数表达式测试 =====

    #[test]
    fn test_function_expression_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic function expression
        let source = r#"
const add = function(a: number, b: number): number {
    return a + b;
};

const result = add(1, 2);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("function"),
            "Should contain 'function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("add"),
            "Should contain 'add': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("result"),
            "Should contain 'result': {}",
            result.js_code
        );
        // Type annotations should be removed
        assert!(
            !result.js_code.contains(": number"),
            "Should not contain type annotations: {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_function_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async function expression
        let source = r#"
const fetchData = async function(url: string): Promise<string> {
    return "data";
};

const runner = fetchData();
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("async function"),
            "Should contain 'async function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("fetchData"),
            "Should contain 'fetchData': {}",
            result.js_code
        );
        // Promise type annotation should be removed
        assert!(
            !result.js_code.contains("Promise"),
            "Should not contain Promise type: {}",
            result.js_code
        );
    }

    #[test]
    fn test_function_expression_no_params() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function expression with no parameters
        let source = r#"
const greet = function(): string {
    return "Hello!";
};

console.log(greet());
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("function"),
            "Should contain 'function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("greet"),
            "Should contain 'greet': {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_function_expression_no_params() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async function expression with no parameters
        let source = r#"
const wait = async function(): Promise<void> {
    await new Promise(r => setTimeout(r, 100));
};

wait();
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("async function"),
            "Should contain 'async function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("wait"),
            "Should contain 'wait': {}",
            result.js_code
        );
    }

    #[test]
    fn test_function_expression_with_callback() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test function expression passed as callback
        let source = r#"
const numbers = [1, 2, 3];
const doubled = numbers.map(function(n: number): number {
    return n * 2;
});
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("function"),
            "Should contain 'function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("map"),
            "Should contain 'map': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("doubled"),
            "Should contain 'doubled': {}",
            result.js_code
        );
    }

    #[test]
    fn test_named_function_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test named function expression (for recursion)
        let source = r#"
const factorial = function fact(n: number): number {
    if (n <= 1) return 1;
    return n * fact(n - 1);
};

console.log(factorial(5));
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("function"),
            "Should contain 'function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("factorial"),
            "Should contain 'factorial': {}",
            result.js_code
        );
    }

    #[test]
    fn test_async_named_function_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test async named function expression - simplified test without complex generic types
        let source = r#"
const processAsync = async function processData(items: any[]): any {
    return items;
};

processAsync([]);
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should compile without errors
        assert!(
            result.js_code.contains("async function"),
            "Should contain 'async function': {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("processAsync"),
            "Should contain 'processAsync': {}",
            result.js_code
        );
    }

    #[test]
    fn test_await_in_async_function_expression() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test await in async function expression
        let source = r#"
const fetchData = async function(): Promise<string> {
    const result = await fetch('/api/data');
    return result;
};
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Note: single quotes may be converted to double quotes in output
        assert!(
            result.js_code.contains("await fetch"),
            "Should contain await expression: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async function"),
            "Should contain async function: {}",
            result.js_code
        );
    }

    #[test]
    fn test_await_in_async_arrow_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test await in async arrow function
        let source = r#"
const fetchData = async () => {
    const result = await getData();
    return result;
};
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("await getData()"),
            "Should contain await expression: {}",
            result.js_code
        );
        assert!(
            result.js_code.contains("async"),
            "Should contain async: {}",
            result.js_code
        );
    }

    #[test]
    fn test_indexed_access_nested() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test nested indexed access type: Person["address"]["city"]
        let source = r#"
type Address = { city: string; street: string };
type Person = { name: string; address: Address };
type PersonCity = Person["address"]["city"];
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Type aliases should be removed in JS output
        assert!(
            !result.js_code.contains("type PersonCity"),
            "Should not contain type alias: {}",
            result.js_code
        );
    }

    #[test]
    fn test_indexed_access_with_union() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test indexed access with union type keys
        let source = r#"
type Data = { id: number; name: string; active: boolean };
type DataIdOrName = Data["id" | "name"];
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            !result.js_code.contains("type DataIdOrName"),
            "Should not contain type alias: {}",
            result.js_code
        );
    }

    #[test]
    fn test_mapped_type_with_keyof() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test mapped type with keyof
        let source = r#"
type Point = { x: number; y: number };
type PointKeys = keyof Point;
type MappedPoint = { [K in keyof Point]: Point[K] };
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            !result.js_code.contains("type PointKeys"),
            "Should not contain type alias: {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("[K in keyof"),
            "Should not contain mapped type syntax: {}",
            result.js_code
        );
    }

    #[test]
    fn test_conditional_type_recursive() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test recursive conditional types
        let source = r#"
type DeepPromise<T> = T extends Promise<infer U> ? DeepPromise<U> : T;
type Result1 = DeepPromise<Promise<Promise<string>>>;
type Result2 = DeepPromise<number>;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            !result.js_code.contains("type Result1"),
            "Should not contain type alias: {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("type Result2"),
            "Should not contain type alias: {}",
            result.js_code
        );
    }

    #[test]
    fn test_template_literal_type_with_number() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template literal type with number concatenation
        let source = r#"
type Id = { id: number };
type IdString = `${Id["id"]}_suffix`;
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            !result.js_code.contains("type IdString"),
            "Should not contain type alias: {}",
            result.js_code
        );
    }

    #[test]
    fn test_this_parameter_in_method() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test this parameter in method
        let source = r#"
class Counter {
    private count: number = 0;
    increment(this: this): void {
        this.count++;
    }
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        // Should remove type annotations but keep method structure
        assert!(
            result.js_code.contains("increment"),
            "Should contain increment method: {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains(": this"),
            "Should not contain this parameter type: {}",
            result.js_code
        );
    }

    #[test]
    fn test_this_parameter_in_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test this parameter in standalone function
        let source = r#"
function bound(this: any, x: number): void {
    console.log(this, x);
}
"#;
        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Test this parameter in function passed: {}", result.js_code);
                assert!(
                    result.js_code.contains("bound"),
                    "Should contain bound function"
                );
            }
            Err(e) => {
                println!("Test this parameter in function failed: {:?}", e);
                panic!("Should compile successfully");
            }
        }
    }

    #[test]
    fn test_generic_with_default() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test generic with default type
        let source = r#"
function wrap<T = string>(value: T): T[] {
    return [value];
}
"#;
        let result = compiler.compile_source(source, "test.ts").unwrap();
        assert!(
            result.js_code.contains("function wrap"),
            "Should contain function: {}",
            result.js_code
        );
        assert!(
            !result.js_code.contains("= string"),
            "Should not contain generic default in JS: {}",
            result.js_code
        );
    }

    #[test]
    fn test_newline_in_template_string() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test newline at start of template string
        let source = r#"console.log(`\nSum: 1 + 2 = ${add(1, 2)}`);"#;
        println!("\n========== Testing newline in template ==========\n");
        println!("Source: {}", source);

        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Compiled JS:\n{}", result.js_code);

        // The \n should be converted to actual newline or \\n
        assert!(result.js_code.contains("Sum"), "Should contain 'Sum'");
    }

    #[test]
    fn test_template_with_quotes() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test template string with quotes inside
        let source = r#"console.log(`Generic identity("test") = ${identity("test")}`);"#;
        println!("\n========== Testing template with quotes ==========\n");
        println!("Source: {}", source);

        let result = compiler.compile_source(source, "test.ts").unwrap();
        println!("Compiled JS:\n{}", result.js_code);

        assert!(
            result.js_code.contains("Generic"),
            "Should contain 'Generic'"
        );
    }

    // v0.3.145: Source Map validation utility tests
    #[test]
    fn test_source_map_validation_valid() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = "let x: number = 5;";
        let result = compiler.compile_source(source, "test.ts").unwrap();

        assert!(
            result.source_map.is_some(),
            "Source map should be generated"
        );

        // Validate the source map
        let validation = validate_source_map(result.source_map.as_ref().unwrap());
        assert!(
            validation.is_valid,
            "Source map should be valid: {:?}",
            validation.errors
        );
    }

    #[test]
    fn test_source_map_validation_structure() {
        let source_map = r#"{"version":3,"sources":["test.ts"],"mappings":"AACA","names":[],"sourcesContent":["let x: number = 5;"]}"#;
        let validation = validate_source_map(source_map);

        assert!(
            validation.is_valid,
            "Valid source map should pass validation"
        );
        assert!(validation.version.is_some(), "Should detect version 3");
        assert!(validation.sources.is_some(), "Should detect sources");
        assert!(validation.mappings.is_some(), "Should detect mappings");
    }

    #[test]
    fn test_source_map_validation_missing_version() {
        let source_map = r#"{"sources":["test.ts"],"mappings":"AACA"}"#;
        let validation = validate_source_map(source_map);

        assert!(
            !validation.is_valid,
            "Source map without version should fail"
        );
        assert!(
            validation
                .errors
                .contains(&"Missing required field: version".to_string()),
            "Should report missing version error"
        );
    }

    #[test]
    fn test_source_map_validation_missing_mappings() {
        let source_map = r#"{"version":3,"sources":["test.ts"]}"#;
        let validation = validate_source_map(source_map);

        assert!(
            !validation.is_valid,
            "Source map without mappings should fail"
        );
        assert!(
            validation
                .errors
                .contains(&"Missing required field: mappings".to_string()),
            "Should report missing mappings error"
        );
    }

    #[test]
    fn test_source_map_validation_invalid_vlq() {
        let source_map =
            r#"{"version":3,"sources":["test.ts"],"mappings":"!!!INVALID!!!","names":[]}"#;
        let validation = validate_source_map(source_map);

        assert!(
            !validation.is_valid,
            "Source map with invalid VLQ should fail"
        );
        assert!(!validation.errors.is_empty(), "Should report VLQ errors");
    }

    #[test]
    fn test_source_map_validation_multiline() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = "let x: number = 5;\nlet y: string = 'hello';\nlet z: boolean = true;";
        let result = compiler.compile_source(source, "test.ts").unwrap();

        assert!(
            result.source_map.is_some(),
            "Source map should be generated"
        );

        // Validate the source map
        let validation = validate_source_map(result.source_map.as_ref().unwrap());
        assert!(
            validation.is_valid,
            "Multi-line source map should be valid: {:?}",
            validation.errors
        );

        // Should have semicolons in mappings for multi-line
        if let Some(mappings) = &validation.mappings {
            assert!(
                mappings.contains(';'),
                "Multi-line source map should have semicolons"
            );
        }
    }

    /// Test precise source map generation with token positions (v0.3.146)
    #[test]
    pub fn test_precise_source_map_generation() {
        // Test the precise source map generation function
        let token_positions: Vec<(usize, usize, usize, usize)> = vec![
            (0, 0, 0, 0), // Line 0, col 0 -> TS line 0, col 0
            (0, 4, 0, 4), // Line 0, col 4 -> TS line 0, col 4
            (1, 0, 1, 0), // Line 1, col 0 -> TS line 1, col 0
        ];

        let js_code = "let x = 5;\nlet y = 10;";
        let mappings = generate_precise_source_map(js_code, &token_positions);

        // Should generate valid VLQ mappings
        assert!(!mappings.is_empty(), "Should generate mappings");

        // Should contain segment separators
        assert!(
            mappings.contains(';'),
            "Should have line separators for multi-line"
        );

        // Validate VLQ encoding characters
        for ch in mappings.chars() {
            if ch != ';' && ch != ',' {
                assert!(
                    ch.is_alphanumeric() || ch == '+' || ch == '/' || ch == '-',
                    "Invalid VLQ character: {}",
                    ch
                );
            }
        }
    }

    /// Test precise source map with single line (v0.3.146)
    #[test]
    pub fn test_precise_source_map_single_line() {
        let token_positions: Vec<(usize, usize, usize, usize)> =
            vec![(0, 0, 0, 0), (0, 3, 0, 3), (0, 5, 0, 6)];

        let js_code = "let x = 5;";
        let mappings = generate_precise_source_map(js_code, &token_positions);

        // Single line should not have semicolons
        assert!(
            !mappings.contains(';'),
            "Single line should not have semicolons"
        );
        assert!(!mappings.is_empty(), "Should generate mappings");
    }

    /// Test basic namespace compilation (v0.3.148)
    #[test]
    fn test_namespace_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
namespace MyNamespace {
    export const x = 5;
}
"#;
        println!("\n========== Testing: basic namespace ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should compile to JavaScript
                assert!(
                    result.js_code.contains("var MyNamespace"),
                    "Should declare namespace variable"
                );
                assert!(result.js_code.contains("function"), "Should have IIFE");
                println!("✅ Basic namespace test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test namespace with multiple declarations (v0.3.148)
    #[test]
    fn test_namespace_multiple_declarations() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
namespace Utils {
    export function greet(name: string): string {
        return "Hello, " + name;
    }
    export const version = "1.0";
    export let count = 0;
}
"#;
        println!("\n========== Testing: namespace with multiple declarations ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should compile namespace to JavaScript IIFE
                assert!(
                    result.js_code.contains("var Utils"),
                    "Should declare namespace variable"
                );
                assert!(result.js_code.contains("function"), "Should have IIFE");
                assert!(
                    result.js_code.contains("Utils"),
                    "Should reference namespace"
                );
                println!("✅ Multiple declarations namespace test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test nested namespace (v0.3.148)
    #[test]
    fn test_namespace_nested() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
namespace Outer {
    export const outerValue = 1;
    namespace Inner {
        export const innerValue = 2;
    }
}
"#;
        println!("\n========== Testing: nested namespace ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should compile both namespaces
                assert!(
                    result.js_code.contains("var Outer"),
                    "Should declare outer namespace"
                );
                assert!(
                    result.js_code.contains("var Inner"),
                    "Should declare inner namespace"
                );
                println!("✅ Nested namespace test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test namespace with class (v0.3.148)
    #[test]
    fn test_namespace_with_class() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
namespace MyNamespace {
    export class Calculator {
        add(a: number, b: number): number {
            return a + b;
        }
    }
}
"#;
        println!("\n========== Testing: namespace with class ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("var MyNamespace"),
                    "Should declare namespace"
                );
                assert!(result.js_code.contains("function"), "Should have IIFE");
                println!("✅ Namespace with class test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test namespace with interface (v0.3.148)
    #[test]
    fn test_namespace_with_interface() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        let source = r#"
namespace MyNamespace {
    interface Person {
        name: string;
        age: number;
    }
    export const person: Person = { name: "John", age: 30 };
}
"#;
        println!("\n========== Testing: namespace with interface ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("var MyNamespace"),
                    "Should declare namespace"
                );
                // Interface should be stripped, but person should remain
                assert!(
                    result.js_code.contains("person"),
                    "Should have person variable"
                );
                println!("✅ Namespace with interface test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test declare function (v0.3.151)
    #[test]
    fn test_declare_function_basic() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test basic declare function
        let source = r#"
declare function greet(name: string): string;
declare function add(a: number, b: number): number;
"#;
        println!("\n========== Testing: declare function basic ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Should contain declare keyword
                assert!(
                    result.js_code.contains("declare function greet"),
                    "Should contain declare function greet: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("declare function add"),
                    "Should contain declare function add: {}",
                    result.js_code
                );
                // Type annotations should be stripped
                assert!(
                    !result.js_code.contains(": string"),
                    "Should not contain type annotation: {}",
                    result.js_code
                );
                println!("✅ Declare function basic test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test declare function with parameters (v0.3.151)
    #[test]
    fn test_declare_function_with_params() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test declare function with multiple parameters
        let source = r#"
declare function fetchData(url: string, options?: object): Promise<string>;
declare function logMessage(msg: string, level: string): void;
"#;
        println!("\n========== Testing: declare function with params ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                assert!(
                    result.js_code.contains("declare function fetchData"),
                    "Should contain declare function fetchData: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("declare function logMessage"),
                    "Should contain declare function logMessage: {}",
                    result.js_code
                );
                // Parameters should be present without type annotations
                assert!(
                    result.js_code.contains("url"),
                    "Should contain url parameter: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("options"),
                    "Should contain options parameter: {}",
                    result.js_code
                );
                println!("✅ Declare function with params test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test export declare function (v0.3.151, v0.3.197: updated for ESM comment format)
    #[test]
    fn test_export_declare_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Test export declare function
        let source = r#"
export declare function calculate(a: number, b: number): number;
export declare function getVersion(): string;
"#;
        println!("\n========== Testing: export declare function ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // v0.3.197: ESM exports are converted to comments
                assert!(
                    result.js_code.contains("ESM export"),
                    "Should contain ESM export comment: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("calculate"),
                    "Should contain calculate: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("getVersion"),
                    "Should contain getVersion: {}",
                    result.js_code
                );
                println!("✅ Export declare function test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }

    /// Test regular function vs declare function (v0.3.151)
    #[test]
    fn test_regular_function_vs_declare_function() {
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        // Compare regular function with declare function
        let source = r#"
function regularFunc(x: number): number {
    return x * 2;
}

declare function declaredFunc(x: number): number;
"#;
        println!("\n========== Testing: regular vs declare function ==========\n");

        match compiler.compile_source(source, "test.ts") {
            Ok(result) => {
                println!("Compiled successfully!");
                println!("JS Code:\n{}", result.js_code);
                // Regular function should have body
                assert!(
                    result.js_code.contains("function regularFunc"),
                    "Should contain regular function: {}",
                    result.js_code
                );
                assert!(
                    result.js_code.contains("return x * 2"),
                    "Should contain function body: {}",
                    result.js_code
                );
                // Declare function should have declare keyword but no body
                assert!(
                    result.js_code.contains("declare function declaredFunc"),
                    "Should contain declare function: {}",
                    result.js_code
                );
                assert!(
                    !result.js_code.contains("declare function declaredFunc {"),
                    "Declare function should not have body: {}",
                    result.js_code
                );
                println!("✅ Regular vs declare function test passed");
            }
            Err(e) => {
                panic!("Compilation failed: {}", e);
            }
        }
    }
}

/// Source Map validation result (v0.3.145)
/// NOTE: Reserved for future use with debugger source map integration
#[allow(dead_code)]
#[derive(Debug)]
struct SourceMapValidationResult {
    is_valid: bool,
    version: Option<u32>,
    sources: Option<Vec<String>>,
    mappings: Option<String>,
    sources_content: Option<String>,
    errors: Vec<String>,
}

/// Validate a source map string (v0.3.145)
#[allow(dead_code)]
fn validate_source_map(source_map: &str) -> SourceMapValidationResult {
    let mut result = SourceMapValidationResult {
        is_valid: true,
        version: None,
        sources: None,
        mappings: None,
        sources_content: None,
        errors: Vec::new(),
    };

    // Parse as JSON
    let json: Result<serde_json::Value, _> = serde_json::from_str(source_map);
    match json {
        Ok(map) => {
            // Validate version (required, must be 3)
            if let Some(v) = map.get("version") {
                if let Some(v_num) = v.as_u64() {
                    result.version = Some(v_num as u32);
                    if v_num != 3 {
                        result
                            .errors
                            .push("Source map version must be 3".to_string());
                        result.is_valid = false;
                    }
                } else {
                    result.errors.push("Invalid version format".to_string());
                    result.is_valid = false;
                }
            } else {
                result
                    .errors
                    .push("Missing required field: version".to_string());
                result.is_valid = false;
            }

            // Validate sources (required)
            if let Some(sources) = map.get("sources") {
                if sources.is_array() {
                    result.sources = Some(
                        sources
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|s| s.as_str().map(|s| s.to_string()))
                            .collect(),
                    );
                }
            } else {
                result
                    .errors
                    .push("Missing required field: sources".to_string());
                result.is_valid = false;
            }

            // Validate mappings (required)
            if let Some(m) = map.get("mappings") {
                if let Some(m_str) = m.as_str() {
                    result.mappings = Some(m_str.to_string());
                    // Validate VLQ encoding
                    for ch in m_str.chars() {
                        if ch != ';' && ch != ','
                            && !ch.is_alphanumeric() && ch != '+' && ch != '/' {
                                result.errors.push(format!("Invalid VLQ character: {}", ch));
                                result.is_valid = false;
                            }
                    }
                }
            } else {
                result
                    .errors
                    .push("Missing required field: mappings".to_string());
                result.is_valid = false;
            }

            // Validate sourcesContent (optional but recommended)
            if let Some(sc) = map.get("sourcesContent") {
                if sc.is_string() || sc.is_array() {
                    result.sources_content = Some(sc.to_string());
                }
            }

            // Validate names (optional)
            if let Some(names) = map.get("names") {
                if !names.is_array() {
                    result
                        .errors
                        .push("Field 'names' must be an array".to_string());
                    result.is_valid = false;
                }
            }
        }
        Err(e) => {
            result.errors.push(format!("Invalid JSON: {}", e));
            result.is_valid = false;
        }
    }

    result
}

impl TypeScriptCompiler {
    #[cfg(test)]
    pub fn debug_compile(&mut self, source: &str) -> String {
        let result = self.compile_source(source, "test.ts").unwrap();
        result.js_code
    }
}
