use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    File,
    Function,
    Method,
    Class,
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Interface,
    TypeAlias,
    ArrowFunction,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeContext {
    pub path: PathBuf,
    pub language: String,
    pub kind: ScopeKind,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub cursor_line: Option<usize>,
}

impl ScopeContext {
    pub fn key(&self) -> String {
        format!("{}::{}", self.kind_label(), self.name)
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            ScopeKind::File => "file",
            ScopeKind::Function => "function",
            ScopeKind::Method => "method",
            ScopeKind::Class => "class",
            ScopeKind::Module => "module",
            ScopeKind::Struct => "struct",
            ScopeKind::Enum => "enum",
            ScopeKind::Trait => "trait",
            ScopeKind::Impl => "impl",
            ScopeKind::Interface => "interface",
            ScopeKind::TypeAlias => "type_alias",
            ScopeKind::ArrowFunction => "arrow_function",
            ScopeKind::Unknown => "unknown",
        }
    }
}

pub fn detect_scope(path: impl AsRef<Path>, cursor_line: Option<usize>) -> Result<ScopeContext> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source file {}", path.display()))?;

    match path.extension().and_then(|value| value.to_str()).map(|value| value.to_ascii_lowercase()).as_deref() {
        Some("rs") => detect_scoped_source(
            path,
            &source,
            cursor_line,
            "rust",
            tree_sitter_rust::language(),
            rust_is_scope_node,
            rust_scope_kind,
            rust_scope_name,
        ),
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => detect_scoped_source(
            path,
            &source,
            cursor_line,
            "javascript",
            tree_sitter_javascript::language(),
            javascript_is_scope_node,
            javascript_scope_kind,
            javascript_scope_name,
        ),
        Some("ts") | Some("mts") | Some("cts") => detect_scoped_source(
            path,
            &source,
            cursor_line,
            "typescript",
            tree_sitter_typescript::language_typescript(),
            typescript_is_scope_node,
            typescript_scope_kind,
            typescript_scope_name,
        ),
        Some("tsx") => detect_scoped_source(
            path,
            &source,
            cursor_line,
            "typescript",
            tree_sitter_typescript::language_tsx(),
            typescript_is_scope_node,
            typescript_scope_kind,
            typescript_scope_name,
        ),
        _ => Ok(file_scope(path, cursor_line, inferred_language_label(path))),
    }
}

fn detect_scoped_source(
    path: &Path,
    source: &str,
    cursor_line: Option<usize>,
    language_label: &str,
    language: tree_sitter::Language,
    scope_matcher: fn(&str) -> bool,
    kind_mapper: fn(&str) -> ScopeKind,
    name_resolver: fn(Node<'_>, &str) -> String,
) -> Result<ScopeContext> {
    let mut parser = Parser::new();
    parser.set_language(language).context("failed to load tree-sitter grammar")?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("failed to parse source with tree-sitter"))?;

    let root = tree.root_node();
    let line = cursor_line.unwrap_or(1).max(1);

    if let Some(scope_node) = best_scope_node(root, line, scope_matcher) {
        return Ok(scope_context_from_node(
            path,
            source,
            scope_node,
            cursor_line,
            language_label,
            kind_mapper,
            name_resolver,
        ));
    }

    Ok(file_scope(path, cursor_line, language_label))
}

fn best_scope_node<'tree>(node: Node<'tree>, line: usize, scope_matcher: fn(&str) -> bool) -> Option<Node<'tree>> {
    let mut best: Option<Node<'tree>> = None;
    best_scope_node_recursive(node, line, scope_matcher, &mut best);
    best
}

fn best_scope_node_recursive<'tree>(
    node: Node<'tree>,
    line: usize,
    scope_matcher: fn(&str) -> bool,
    best: &mut Option<Node<'tree>>,
) {
    if scope_matcher(node.kind()) && node_contains_line(node, line) {
        let replace = match *best {
            Some(current) => node_span(node) < node_span(current),
            None => true,
        };

        if replace {
            *best = Some(node);
        }
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            best_scope_node_recursive(child, line, scope_matcher, best);
        }
    }
}

fn node_contains_line(node: Node<'_>, line: usize) -> bool {
    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;
    start <= line && line <= end
}

fn node_span(node: Node<'_>) -> usize {
    let start = node.start_position().row;
    let end = node.end_position().row;
    end.saturating_sub(start)
}

fn rust_is_scope_node(kind: &str) -> bool {
    matches!(
        kind,
        "function_item" | "mod_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item"
    )
}

fn javascript_is_scope_node(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "generator_function_declaration"
            | "arrow_function"
            | "method_definition"
            | "class_declaration"
    )
}

fn typescript_is_scope_node(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "arrow_function"
            | "method_definition"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
    )
}

fn scope_context_from_node(
    path: &Path,
    source: &str,
    node: Node<'_>,
    cursor_line: Option<usize>,
    language: &str,
    kind_mapper: fn(&str) -> ScopeKind,
    name_resolver: fn(Node<'_>, &str) -> String,
) -> ScopeContext {
    ScopeContext {
        path: path.to_path_buf(),
        language: language.to_string(),
        kind: kind_mapper(node.kind()),
        name: name_resolver(node, source),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        cursor_line,
    }
}

fn rust_scope_kind(kind: &str) -> ScopeKind {
    match kind {
        "function_item" => ScopeKind::Function,
        "mod_item" => ScopeKind::Module,
        "struct_item" => ScopeKind::Struct,
        "enum_item" => ScopeKind::Enum,
        "trait_item" => ScopeKind::Trait,
        "impl_item" => ScopeKind::Impl,
        _ => ScopeKind::Unknown,
    }
}

fn javascript_scope_kind(kind: &str) -> ScopeKind {
    match kind {
        "function_declaration" | "generator_function_declaration" => ScopeKind::Function,
        "arrow_function" => ScopeKind::ArrowFunction,
        "method_definition" => ScopeKind::Method,
        "class_declaration" => ScopeKind::Class,
        _ => ScopeKind::Unknown,
    }
}

fn typescript_scope_kind(kind: &str) -> ScopeKind {
    match kind {
        "function_declaration" => ScopeKind::Function,
        "arrow_function" => ScopeKind::ArrowFunction,
        "method_definition" => ScopeKind::Method,
        "class_declaration" => ScopeKind::Class,
        "interface_declaration" => ScopeKind::Interface,
        "type_alias_declaration" => ScopeKind::TypeAlias,
        _ => ScopeKind::Unknown,
    }
}

fn rust_scope_name(node: Node<'_>, source: &str) -> String {
    match node.kind() {
        "function_item" | "struct_item" | "enum_item" | "trait_item" | "mod_item" => {
            node.child_by_field_name("name")
                .and_then(|child| node_text(child, source))
                .unwrap_or_else(|| node.kind().to_string())
        }
        "impl_item" => {
            let target = node
                .child_by_field_name("type")
                .and_then(|child| node_text(child, source))
                .unwrap_or_else(|| "unknown".to_string());
            format!("impl {target}")
        }
        _ => node.kind().to_string(),
    }
}

fn javascript_scope_name(node: Node<'_>, source: &str) -> String {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "class_declaration" => {
            node.child_by_field_name("name")
                .and_then(|child| node_text(child, source))
                .unwrap_or_else(|| node.kind().to_string())
        }
        "method_definition" => node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
            .unwrap_or_else(|| node.kind().to_string()),
        "arrow_function" => ancestor_name(node, source).unwrap_or_else(|| "arrow_function".to_string()),
        _ => node.kind().to_string(),
    }
}

fn typescript_scope_name(node: Node<'_>, source: &str) -> String {
    match node.kind() {
        "function_declaration" | "class_declaration" | "interface_declaration" | "type_alias_declaration" => {
            node.child_by_field_name("name")
                .and_then(|child| node_text(child, source))
                .unwrap_or_else(|| node.kind().to_string())
        }
        "method_definition" => node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
            .unwrap_or_else(|| node.kind().to_string()),
        "arrow_function" => ancestor_name(node, source).unwrap_or_else(|| "arrow_function".to_string()),
        _ => node.kind().to_string(),
    }
}

fn ancestor_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut current = node.parent();

    while let Some(parent) = current {
        if parent.kind() == "variable_declarator" {
            if let Some(name_node) = parent.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, source) {
                    return Some(name);
                }
            }
        }

        current = parent.parent();
    }

    None
}

fn node_text(node: Node<'_>, source: &str) -> Option<String> {
    source.get(node.byte_range()).map(|text| text.trim().to_string())
}

fn file_scope(path: &Path, cursor_line: Option<usize>, language: impl Into<String>) -> ScopeContext {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file")
        .to_string();

    ScopeContext {
        path: path.to_path_buf(),
        language: language.into(),
        kind: ScopeKind::File,
        name,
        start_line: 1,
        end_line: cursor_line.unwrap_or(1),
        cursor_line,
    }
}

fn inferred_language_label(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("rs") => "rust".to_string(),
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => "javascript".to_string(),
        Some("ts") | Some("tsx") | Some("mts") | Some("cts") => "typescript".to_string(),
        Some(other) => other.to_string(),
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn detects_rust_function_scope() {
        let root = std::env::temp_dir().join(format!("git-lore-parser-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("lib.rs");
        fs::write(
            &file,
            r#"
pub fn outer() {
    fn inner() {
        println!("hi");
    }
}
"#,
        )
        .unwrap();

        let scope = detect_scope(&file, Some(3)).unwrap();
        assert_eq!(scope.kind, ScopeKind::Function);
        assert_eq!(scope.name, "inner");
        assert_eq!(scope.language, "rust");
    }

    #[test]
    fn falls_back_to_file_scope_for_non_rust() {
        let root = std::env::temp_dir().join(format!("git-lore-parser-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        fs::write(&file, "hello\nworld\n").unwrap();

        let scope = detect_scope(&file, Some(2)).unwrap();
        assert_eq!(scope.kind, ScopeKind::File);
        assert_eq!(scope.name, "notes");
    }

        #[test]
        fn detects_javascript_function_scope() {
                let root = std::env::temp_dir().join(format!("git-lore-parser-js-test-{}", Uuid::new_v4()));
                fs::create_dir_all(&root).unwrap();
                let file = root.join("index.js");
                fs::write(
                        &file,
                        r#"
function outer() {
    function inner() {
        return 1;
    }
}
"#,
                )
                .unwrap();

                let scope = detect_scope(&file, Some(3)).unwrap();
                assert_eq!(scope.language, "javascript");
                assert_eq!(scope.kind, ScopeKind::Function);
                assert_eq!(scope.name, "inner");
        }

        #[test]
        fn detects_typescript_class_scope() {
                let root = std::env::temp_dir().join(format!("git-lore-parser-ts-test-{}", Uuid::new_v4()));
                fs::create_dir_all(&root).unwrap();
                let file = root.join("service.ts");
                fs::write(
                        &file,
                        r#"
class Service {
    run(): void {
        return;
    }
}
"#,
                )
                .unwrap();

                let scope = detect_scope(&file, Some(3)).unwrap();
                assert_eq!(scope.language, "typescript");
                assert_eq!(scope.kind, ScopeKind::Method);
                assert_eq!(scope.name, "run");
        }
}
