//! AST search using tree-sitter for semantic code understanding.
//!
//! Each grammar is loaded via its crate's `LANGUAGE` constant (a
//! [`tree_sitter_language::LanguageFn`]). Core `tree-sitter` and every grammar must
//! agree on the parser ABI — if a grammar is bumped past the core's supported
//! ABI, [`Parser::set_language`] returns a [`LanguageError`] and we surface it
//! loudly from [`AstSearcher::new`] rather than silently dropping it.

use super::{MatchType, SearchResult};
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_language::LanguageFn;
use walkdir::WalkDir;

/// Languages we can parse, paired with their tree-sitter grammar.
const GRAMMARS: &[(&str, LanguageFn)] = &[
    ("rust", tree_sitter_rust::LANGUAGE),
    ("javascript", tree_sitter_javascript::LANGUAGE),
    ("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
    ("python", tree_sitter_python::LANGUAGE),
    ("go", tree_sitter_go::LANGUAGE),
    ("java", tree_sitter_java::LANGUAGE),
    ("cpp", tree_sitter_cpp::LANGUAGE),
    ("c", tree_sitter_c::LANGUAGE),
];

/// AST searcher using tree-sitter.
///
/// Holds one [`Language`] per supported language. Parsers are created
/// on demand per parse so the searcher stays `&self`-shareable.
pub struct AstSearcher {
    languages: std::collections::HashMap<String, Language>,
}

impl AstSearcher {
    /// Create a new AST searcher, loading every supported grammar.
    ///
    /// Returns an error if any grammar's parser ABI is incompatible with the
    /// linked `tree-sitter` core (the failure mode an ABI-mismatched grammar
    /// bump produces). This makes such a mismatch impossible to ship silently.
    pub fn new() -> Result<Self, tree_sitter::LanguageError> {
        let mut languages = std::collections::HashMap::new();

        for (name, language_fn) in GRAMMARS {
            let language: Language = (*language_fn).into();
            // Validate the ABI now, against a throwaway parser, so a mismatch
            // is reported here instead of being swallowed at parse time.
            let mut parser = Parser::new();
            parser.set_language(&language)?;
            languages.insert((*name).to_string(), language);
        }

        Ok(Self { languages })
    }

    /// Parse `source` for `lang`, returning the syntax tree and the language.
    fn parse(&self, lang: &str, source: &str) -> Option<(tree_sitter::Tree, Language)> {
        let language = self.languages.get(lang)?.clone();
        let mut parser = Parser::new();
        // Safe to unwrap: every stored language was ABI-validated in `new`.
        parser
            .set_language(&language)
            .expect("language ABI validated in AstSearcher::new");
        let tree = parser.parse(source, None)?;
        Some((tree, language))
    }

    /// Search for AST patterns.
    pub async fn search(
        &self,
        pattern: &str,
        path: &Path,
        language: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();

            // Detect language from file extension unless one was forced.
            let lang = language.unwrap_or_else(|| detect_language(file_path));

            if !self.languages.contains_key(lang) {
                continue;
            }

            if let Ok(source) = fs::read_to_string(file_path) {
                if let Some((tree, grammar)) = self.parse(lang, &source) {
                    let file_results =
                        search_tree(&tree, &grammar, &source, pattern, file_path, lang);
                    results.extend(file_results);

                    if results.len() >= max_results {
                        break;
                    }
                }
            }
        }

        results.truncate(max_results);
        Ok(results)
    }
}

/// Search within a parsed tree.
fn search_tree(
    tree: &tree_sitter::Tree,
    grammar: &Language,
    source: &str,
    pattern: &str,
    file_path: &Path,
    language: &str,
) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let root_node = tree.root_node();

    // Build query based on pattern.
    let query_str = build_query_string(pattern, language);

    if let Ok(query) = Query::new(grammar, &query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root_node, source.as_bytes());

        // `QueryMatches` is a streaming iterator in tree-sitter >= 0.22.
        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let start = node.start_position();

                let match_text = source[node.byte_range()].to_string();
                let (context_before, context_after) = get_context(source, node, 3);

                results.push(SearchResult {
                    file_path: file_path.to_path_buf(),
                    line_number: start.row + 1,
                    column: start.column,
                    match_text,
                    context_before,
                    context_after,
                    match_type: MatchType::Ast,
                    score: 0.95,
                    node_type: Some(node.kind().to_string()),
                    semantic_context: Some(get_semantic_context(node, source)),
                });
            }
        }
    } else {
        // Fallback to pattern matching on node text.
        results.extend(search_nodes_by_text(root_node, source, pattern, file_path));
    }

    results
}

/// Detect language from file extension
fn detect_language(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => "rust",
        Some("js") | Some("mjs") => "javascript",
        Some("ts") | Some("tsx") => "typescript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("cpp") | Some("cc") | Some("cxx") => "cpp",
        Some("c") | Some("h") => "c",
        _ => "text",
    }
}

/// Build tree-sitter query string from pattern
fn build_query_string(pattern: &str, language: &str) -> String {
    // Check for common patterns and convert to tree-sitter queries
    if pattern.starts_with("function ") {
        let name = pattern.trim_start_matches("function ").trim();
        match language {
            "rust" => format!(
                "(function_item name: (identifier) @fn (#eq? @fn \"{}\"))",
                name
            ),
            "javascript" | "typescript" => {
                format!(
                    "(function_declaration name: (identifier) @fn (#eq? @fn \"{}\"))",
                    name
                )
            }
            "python" => format!(
                "(function_definition name: (identifier) @fn (#eq? @fn \"{}\"))",
                name
            ),
            _ => format!("(identifier) @id (#eq? @id \"{}\")", name),
        }
    } else if pattern.starts_with("class ") {
        let name = pattern.trim_start_matches("class ").trim();
        match language {
            "rust" => format!(
                "(struct_item name: (type_identifier) @struct (#eq? @struct \"{}\"))",
                name
            ),
            "javascript" | "typescript" => {
                format!(
                    "(class_declaration name: (identifier) @class (#eq? @class \"{}\"))",
                    name
                )
            }
            "python" => format!(
                "(class_definition name: (identifier) @class (#eq? @class \"{}\"))",
                name
            ),
            _ => format!("(identifier) @id (#eq? @id \"{}\")", name),
        }
    } else {
        // Generic identifier search
        format!("(identifier) @id (#match? @id \"{}\")", pattern)
    }
}

/// Search nodes by text content
fn search_nodes_by_text(
    node: Node,
    source: &str,
    pattern: &str,
    file_path: &Path,
) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut cursor = node.walk();

    // Visit all nodes
    loop {
        let node = cursor.node();
        let node_text = source[node.byte_range()].to_string();

        // Check if node text contains pattern
        if node_text.contains(pattern) {
            let start = node.start_position();

            results.push(SearchResult {
                file_path: file_path.to_path_buf(),
                line_number: start.row + 1,
                column: start.column,
                match_text: node_text.clone(),
                context_before: vec![],
                context_after: vec![],
                match_type: MatchType::Ast,
                score: 0.9,
                node_type: Some(node.kind().to_string()),
                semantic_context: Some(get_semantic_context(node, source)),
            });
        }

        // Traverse tree
        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }

        loop {
            if !cursor.goto_parent() {
                return results;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Get context lines around a node
fn get_context(source: &str, node: Node, context_lines: usize) -> (Vec<String>, Vec<String>) {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    let before_start = start_line.saturating_sub(context_lines);
    let after_end = std::cmp::min(end_line + context_lines + 1, lines.len());

    let context_before = lines[before_start..start_line]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let context_after = lines[(end_line + 1)..after_end]
        .iter()
        .map(|s| s.to_string())
        .collect();

    (context_before, context_after)
}

/// Get semantic context for a node
fn get_semantic_context(node: Node, source: &str) -> String {
    let mut context = format!(
        "{} at {}:{}",
        node.kind(),
        node.start_position().row + 1,
        node.start_position().column
    );

    // Find parent function or class
    if let Some(parent) = find_parent_context(node) {
        let parent_text = source[parent.byte_range()].lines().next().unwrap_or("");
        context.push_str(&format!(" in {}", parent_text));
    }

    context
}

/// Find parent function or class context
fn find_parent_context(mut node: Node) -> Option<Node> {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "function_item"
            | "function_declaration"
            | "function_definition"
            | "method_definition"
            | "struct_item"
            | "class_declaration"
            | "class_definition"
            | "impl_item" => {
                return Some(parent);
            }
            _ => node = parent,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_searcher_loads_all_grammars() {
        // Construction fails loudly if any grammar's parser ABI is
        // incompatible with the linked tree-sitter core.
        let searcher = AstSearcher::new().expect("all grammars must load (ABI must match core)");
        assert_eq!(searcher.languages.len(), GRAMMARS.len());
    }

    /// Parse a tiny snippet for `lang` and assert the tree is sane:
    /// a non-empty root whose kind matches `expected_root`, with no error nodes.
    fn assert_parses_clean(searcher: &AstSearcher, lang: &str, source: &str, expected_root: &str) {
        let (tree, _grammar) = searcher
            .parse(lang, source)
            .unwrap_or_else(|| panic!("{lang}: parser produced no tree"));
        let root = tree.root_node();
        assert_eq!(root.kind(), expected_root, "{lang}: unexpected root kind");
        assert!(root.child_count() > 0, "{lang}: root has no children");
        assert!(
            !root.has_error(),
            "{lang}: tree contains error nodes (ABI/grammar mismatch?)"
        );
        assert_eq!(
            root.start_byte(),
            0,
            "{lang}: root does not start at byte 0"
        );
        assert_eq!(
            root.end_byte(),
            source.len(),
            "{lang}: root does not cover whole source"
        );
    }

    #[test]
    fn test_parse_c_snippet() {
        let searcher = AstSearcher::new().expect("grammars must load");
        assert_parses_clean(
            &searcher,
            "c",
            "int add(int a, int b) { return a + b; }\n",
            "translation_unit",
        );
    }

    #[test]
    fn test_parse_rust_snippet() {
        let searcher = AstSearcher::new().expect("grammars must load");
        assert_parses_clean(
            &searcher,
            "rust",
            "fn add(a: i32, b: i32) -> i32 { a + b }\n",
            "source_file",
        );
    }

    #[tokio::test]
    async fn test_ast_search() {
        let searcher = AstSearcher::new().expect("grammars must load");
        let results = searcher
            .search("function test", Path::new("."), Some("rust"), 10)
            .await;

        assert!(results.is_ok());
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.rs")), "rust");
        assert_eq!(detect_language(Path::new("test.js")), "javascript");
        assert_eq!(detect_language(Path::new("test.ts")), "typescript");
        assert_eq!(detect_language(Path::new("test.py")), "python");
        assert_eq!(detect_language(Path::new("test.go")), "go");
    }
}
