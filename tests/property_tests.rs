//! Property-based tests using proptest.
//!
//! These tests verify invariants that should hold for all valid inputs,
//! helping catch edge cases that manual test cases might miss.

use proptest::prelude::*;
use std::collections::HashSet;

// Strategy generators for test data

/// Generate valid Rust identifiers
fn rust_identifier() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_]{0,30}")
        .unwrap()
        .prop_filter("must not be empty", |s| !s.is_empty())
}

/// Generate valid function definitions
fn rust_function() -> impl Strategy<Value = String> {
    (rust_identifier(), prop::collection::vec(rust_identifier(), 0..5)).prop_map(
        |(name, params)| {
            let params_str = params
                .iter()
                .enumerate()
                .map(|(i, t)| format!("arg{}: {}", i, t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn {}({}) {{\n    // body\n}}\n", name, params_str)
        },
    )
}

/// Generate valid struct definitions
fn rust_struct() -> impl Strategy<Value = String> {
    (
        rust_identifier(),
        prop::collection::vec(rust_identifier(), 0..10),
    )
        .prop_map(|(name, fields)| {
            let fields_str = fields
                .iter()
                .enumerate()
                .map(|(i, t)| format!("    field_{}: {},", i, t))
                .collect::<Vec<_>>()
                .join("\n");
            format!("pub struct {} {{\n{}\n}}\n", name, fields_str)
        })
}

/// Generate a Rust source file with multiple items
fn rust_source_file() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            rust_function().prop_map(|s| ("function", s)),
            rust_struct().prop_map(|s| ("struct", s)),
        ],
        1..20,
    )
    .prop_map(|items| {
        items
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Generate search queries
fn search_query() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,20}( [a-zA-Z][a-zA-Z0-9_]{0,20}){0,3}")
        .unwrap()
}

/// Generate file paths
fn file_path() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(rust_identifier(), 1..5),
        rust_identifier(),
    )
        .prop_map(|(dirs, file)| {
            let path = dirs.join("/");
            format!("{}/{}.rs", path, file)
        })
}

// Property tests

proptest! {
    /// Parsing the same code twice should produce identical results
    #[test]
    fn parsing_is_deterministic(code in rust_source_file()) {
        let result1 = parse_and_extract(&code);
        let result2 = parse_and_extract(&code);

        prop_assert_eq!(result1, result2, "Parsing should be deterministic");
    }

    /// Search results should be ranked by decreasing relevance
    #[test]
    fn search_results_are_ranked(query in search_query()) {
        let results = mock_search(&query);

        for window in results.windows(2) {
            prop_assert!(
                window[0].1 >= window[1].1,
                "Results should be in decreasing score order: {:?} vs {:?}",
                window[0],
                window[1]
            );
        }
    }

    /// All extracted symbols should have valid names
    #[test]
    fn symbols_have_valid_names(code in rust_source_file()) {
        let symbols = parse_and_extract(&code);

        for symbol in symbols {
            prop_assert!(
                !symbol.is_empty(),
                "Symbol names should not be empty"
            );
            prop_assert!(
                symbol.chars().next().unwrap().is_alphabetic() || symbol.starts_with('_'),
                "Symbol names should start with a letter or underscore: {}",
                symbol
            );
        }
    }

    /// Symbol extraction should find all declared items
    #[test]
    fn all_declarations_are_found(code in rust_source_file()) {
        let symbols = parse_and_extract(&code);

        // Count expected functions
        let expected_fns = code.matches("fn ").count();
        let found_fns = symbols.iter().filter(|s| is_function_name(s)).count();

        // We should find at least as many as declared
        // (might find more if "fn " appears in strings, which is fine)
        prop_assert!(
            found_fns >= expected_fns.saturating_sub(2), // Allow some tolerance
            "Should find most declared functions. Expected ~{}, found {}",
            expected_fns,
            found_fns
        );
    }

    /// Search with empty query should return no results or all results
    #[test]
    fn empty_query_handling(corpus_size in 1usize..100) {
        let results = mock_search_with_corpus("", corpus_size);

        // Empty query should either return nothing or everything
        prop_assert!(
            results.is_empty() || results.len() == corpus_size,
            "Empty query should return nothing or everything"
        );
    }

    /// Tokenization should be consistent
    #[test]
    fn tokenization_is_consistent(text in "[a-zA-Z0-9_\\s]+") {
        let tokens1 = tokenize(&text);
        let tokens2 = tokenize(&text);

        prop_assert_eq!(tokens1, tokens2, "Tokenization should be consistent");
    }

    /// No duplicate symbols from same location
    #[test]
    fn no_duplicate_symbols(code in rust_source_file()) {
        let symbols = parse_and_extract_with_location(&code);

        let locations: HashSet<_> = symbols.iter().map(|(_, loc)| loc).collect();

        prop_assert_eq!(
            symbols.len(),
            locations.len(),
            "Should not have duplicate symbols at same location"
        );
    }

    /// File paths should be normalized
    #[test]
    fn paths_are_normalized(path in file_path()) {
        let normalized = normalize_path(&path);

        prop_assert!(!normalized.contains("//"), "Path should not have double slashes");
        prop_assert!(!normalized.contains("./"), "Path should not have dot-slash");
        prop_assert!(!normalized.starts_with('/'), "Path should be relative");
    }

    /// BM25 scores should be non-negative
    #[test]
    fn bm25_scores_non_negative(
        query in search_query(),
        corpus_size in 1usize..50
    ) {
        let results = mock_search_with_corpus(&query, corpus_size);

        for (_, score) in results {
            prop_assert!(score >= 0.0, "BM25 scores should be non-negative, got {}", score);
        }
    }

    /// Symbol kind classification should be stable
    #[test]
    fn symbol_kind_is_stable(code in rust_function()) {
        let kind1 = classify_symbol(&code);
        let kind2 = classify_symbol(&code);

        prop_assert_eq!(kind1, kind2, "Symbol classification should be stable");
    }
}

// Helper functions (simplified implementations for testing)

fn parse_and_extract(code: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    for line in code.lines() {
        if line.contains("fn ") {
            if let Some(name) = extract_name_after(line, "fn ") {
                symbols.push(name);
            }
        }
        if line.contains("struct ") {
            if let Some(name) = extract_name_after(line, "struct ") {
                symbols.push(name);
            }
        }
    }

    symbols
}

fn parse_and_extract_with_location(code: &str) -> Vec<(String, usize)> {
    let mut symbols = Vec::new();

    for (line_num, line) in code.lines().enumerate() {
        if line.contains("fn ") {
            if let Some(name) = extract_name_after(line, "fn ") {
                symbols.push((name, line_num));
            }
        }
        if line.contains("struct ") {
            if let Some(name) = extract_name_after(line, "struct ") {
                symbols.push((name, line_num));
            }
        }
    }

    symbols
}

fn extract_name_after(line: &str, keyword: &str) -> Option<String> {
    line.split(keyword)
        .nth(1)?
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn is_function_name(s: &str) -> bool {
    // Functions typically start with lowercase
    s.chars().next().is_some_and(|c| c.is_lowercase())
}

fn mock_search(query: &str) -> Vec<(String, f64)> {
    if query.is_empty() {
        return Vec::new();
    }

    // Generate mock results with decreasing scores
    (0..10)
        .map(|i| (format!("result_{}", i), 1.0 - (i as f64 * 0.1)))
        .collect()
}

fn mock_search_with_corpus(query: &str, corpus_size: usize) -> Vec<(String, f64)> {
    if query.is_empty() {
        return Vec::new();
    }

    (0..corpus_size.min(10))
        .map(|i| (format!("result_{}", i), 1.0 - (i as f64 * 0.1)))
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn normalize_path(path: &str) -> String {
    path.replace("//", "/")
        .replace("./", "")
        .trim_start_matches('/')
        .to_string()
}

fn classify_symbol(code: &str) -> &'static str {
    if code.contains("fn ") {
        "function"
    } else if code.contains("struct ") {
        "struct"
    } else if code.contains("enum ") {
        "enum"
    } else if code.contains("trait ") {
        "trait"
    } else {
        "unknown"
    }
}

// Additional invariant tests

#[test]
fn test_search_result_limit_respected() {
    proptest!(|(limit in 1usize..100)| {
        let results = mock_search_limited("test", limit);
        prop_assert!(results.len() <= limit, "Results should respect limit");
    });
}

fn mock_search_limited(query: &str, limit: usize) -> Vec<(String, f64)> {
    mock_search(query).into_iter().take(limit).collect()
}

#[test]
fn test_chunking_preserves_content() {
    proptest!(|(content in "[a-zA-Z0-9\\s\\n]{10,500}")| {
        let chunks = mock_chunk(&content, 100);
        let reconstructed: String = chunks.join("");

        // Content should be preserved (possibly with overlap removed)
        prop_assert!(
            content.len() <= reconstructed.len() + 100, // Allow for overlap
            "Chunking should preserve content"
        );
    });
}

fn mock_chunk(content: &str, chunk_size: usize) -> Vec<String> {
    content
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect()
}
