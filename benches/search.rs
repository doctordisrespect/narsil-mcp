//! Benchmarks for search performance.

#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use tempfile::TempDir;

/// Benchmark BM25 search with varying corpus sizes
fn bench_bm25_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_search");

    for num_files in [100, 500, 1000, 5000].iter() {
        let corpus = create_search_corpus(*num_files);

        group.throughput(Throughput::Elements(*num_files as u64));
        group.bench_with_input(
            BenchmarkId::new("corpus_size", num_files),
            &corpus,
            |b, corpus| {
                b.iter(|| bm25_search(black_box(corpus), black_box("function process")));
            },
        );
    }

    group.finish();
}

/// Benchmark TF-IDF similarity search
fn bench_tfidf_search(c: &mut Criterion) {
    let corpus = create_search_corpus(1000);
    let query_code = r#"
        fn process_data(input: &str) -> Result<String> {
            let parsed = parse(input)?;
            transform(parsed)
        }
    "#;

    c.bench_function("tfidf_search_1000_docs", |b| {
        b.iter(|| tfidf_similarity_search(black_box(&corpus), black_box(query_code)));
    });
}

/// Benchmark hybrid search (BM25 + TF-IDF fusion)
fn bench_hybrid_search(c: &mut Criterion) {
    let corpus = create_search_corpus(1000);

    let mut group = c.benchmark_group("hybrid_search");

    group.bench_function("bm25_only", |b| {
        b.iter(|| bm25_search(black_box(&corpus), black_box("authentication user login")));
    });

    group.bench_function("tfidf_only", |b| {
        b.iter(|| {
            tfidf_similarity_search(black_box(&corpus), black_box("fn authenticate(user: &User)"))
        });
    });

    group.bench_function("hybrid_rrf", |b| {
        b.iter(|| {
            hybrid_search(
                black_box(&corpus),
                black_box("authentication user login"),
                black_box(60.0), // RRF k parameter
            )
        });
    });

    group.finish();
}

/// Benchmark symbol search
fn bench_symbol_search(c: &mut Criterion) {
    let temp_dir = create_test_repo(500);
    let symbols = extract_symbols(temp_dir.path());

    let mut group = c.benchmark_group("symbol_search");

    group.bench_function("exact_match", |b| {
        b.iter(|| find_symbol_exact(black_box(&symbols), black_box("Struct100")));
    });

    group.bench_function("prefix_match", |b| {
        b.iter(|| find_symbol_prefix(black_box(&symbols), black_box("Struct1")));
    });

    group.bench_function("fuzzy_match", |b| {
        b.iter(|| find_symbol_fuzzy(black_box(&symbols), black_box("Strct100")));
    });

    group.finish();
}

/// Benchmark filtering during search
fn bench_filtered_search(c: &mut Criterion) {
    let corpus = create_search_corpus(2000);

    let mut group = c.benchmark_group("filtered_search");

    group.bench_function("no_filter", |b| {
        b.iter(|| bm25_search(black_box(&corpus), black_box("function")));
    });

    group.bench_function("language_filter_rust", |b| {
        b.iter(|| bm25_search_filtered(black_box(&corpus), black_box("function"), "rust"));
    });

    group.bench_function("path_pattern_filter", |b| {
        b.iter(|| {
            bm25_search_path_filtered(black_box(&corpus), black_box("function"), "src/**/*.rs")
        });
    });

    group.finish();
}

// Data structures and helpers

struct SearchDocument {
    id: String,
    content: String,
    path: String,
    language: String,
    tokens: Vec<String>,
    term_freqs: HashMap<String, usize>,
}

struct SearchCorpus {
    documents: Vec<SearchDocument>,
    term_document_freqs: HashMap<String, usize>,
}

fn create_search_corpus(num_docs: usize) -> SearchCorpus {
    let mut documents = Vec::with_capacity(num_docs);
    let mut term_document_freqs: HashMap<String, usize> = HashMap::new();

    for i in 0..num_docs {
        let content = generate_document_content(i);
        let tokens = tokenize(&content);
        let term_freqs = compute_term_freqs(&tokens);

        for term in term_freqs.keys() {
            *term_document_freqs.entry(term.clone()).or_default() += 1;
        }

        documents.push(SearchDocument {
            id: format!("doc_{}", i),
            content,
            path: format!("src/module_{}.rs", i),
            language: "rust".to_string(),
            tokens,
            term_freqs,
        });
    }

    SearchCorpus {
        documents,
        term_document_freqs,
    }
}

fn generate_document_content(index: usize) -> String {
    let topics = ["authentication", "database", "network", "process", "cache"];
    let topic = topics[index % topics.len()];

    format!(
        r#"/// Module for {topic} handling
pub fn {topic}_handler_{index}(input: &str) -> Result<Output, Error> {{
    let parsed = parse_{topic}(input)?;
    validate_{topic}(&parsed)?;
    process_{topic}(parsed)
}}

pub struct {topic}Config_{index} {{
    pub enabled: bool,
    pub timeout: Duration,
    pub retries: u32,
}}

impl {topic}Config_{index} {{
    pub fn new() -> Self {{
        Self {{
            enabled: true,
            timeout: Duration::from_secs(30),
            retries: 3,
        }}
    }}
}}
"#
    )
}

fn tokenize(content: &str) -> Vec<String> {
    content
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_lowercase())
        .collect()
}

fn compute_term_freqs(tokens: &[String]) -> HashMap<String, usize> {
    let mut freqs = HashMap::new();
    for token in tokens {
        *freqs.entry(token.clone()).or_default() += 1;
    }
    freqs
}

fn create_test_repo(num_files: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    for i in 0..num_files {
        let content = generate_document_content(i);
        std::fs::write(src_dir.join(format!("module_{}.rs", i)), content).unwrap();
    }

    temp_dir
}

// Stub search functions - would be replaced with actual implementations

fn bm25_search(corpus: &SearchCorpus, query: &str) -> Vec<(String, f64)> {
    let query_tokens: Vec<String> = tokenize(query);
    let n = corpus.documents.len() as f64;
    let k1 = 1.2;
    let b = 0.75;
    let avg_dl = corpus.documents.iter().map(|d| d.tokens.len()).sum::<usize>() as f64 / n;

    let mut scores: Vec<(String, f64)> = corpus
        .documents
        .iter()
        .map(|doc| {
            let dl = doc.tokens.len() as f64;
            let score: f64 = query_tokens
                .iter()
                .map(|term| {
                    let tf = *doc.term_freqs.get(term).unwrap_or(&0) as f64;
                    let df = *corpus.term_document_freqs.get(term).unwrap_or(&0) as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let tf_component = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (dl / avg_dl)));
                    idf * tf_component
                })
                .sum();
            (doc.id.clone(), score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scores.truncate(10);
    scores
}

fn bm25_search_filtered(corpus: &SearchCorpus, query: &str, language: &str) -> Vec<(String, f64)> {
    // Simplified - would filter before scoring
    bm25_search(corpus, query)
        .into_iter()
        .filter(|_| language == "rust")
        .collect()
}

fn bm25_search_path_filtered(
    corpus: &SearchCorpus,
    query: &str,
    _pattern: &str,
) -> Vec<(String, f64)> {
    // Simplified - would use glob pattern matching
    bm25_search(corpus, query)
}

fn tfidf_similarity_search(corpus: &SearchCorpus, query_code: &str) -> Vec<(String, f64)> {
    let query_tokens = tokenize(query_code);
    let query_freqs = compute_term_freqs(&query_tokens);

    let mut scores: Vec<(String, f64)> = corpus
        .documents
        .iter()
        .map(|doc| {
            let score: f64 = query_freqs
                .iter()
                .map(|(term, &qf)| {
                    let df = *doc.term_freqs.get(term).unwrap_or(&0) as f64;
                    qf as f64 * df
                })
                .sum();
            (doc.id.clone(), score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scores.truncate(10);
    scores
}

fn hybrid_search(corpus: &SearchCorpus, query: &str, rrf_k: f64) -> Vec<(String, f64)> {
    let bm25_results = bm25_search(corpus, query);
    let tfidf_results = tfidf_similarity_search(corpus, query);

    // Reciprocal Rank Fusion
    let mut combined_scores: HashMap<String, f64> = HashMap::new();

    for (rank, (id, _)) in bm25_results.iter().enumerate() {
        *combined_scores.entry(id.clone()).or_default() += 1.0 / (rrf_k + rank as f64 + 1.0);
    }

    for (rank, (id, _)) in tfidf_results.iter().enumerate() {
        *combined_scores.entry(id.clone()).or_default() += 1.0 / (rrf_k + rank as f64 + 1.0);
    }

    let mut results: Vec<(String, f64)> = combined_scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results.truncate(10);
    results
}

struct Symbol {
    name: String,
    kind: String,
    path: String,
}

fn extract_symbols(path: &std::path::Path) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            // Simplified symbol extraction
            for line in content.lines() {
                if line.contains("fn ") {
                    if let Some(name) = extract_name(line, "fn ") {
                        symbols.push(Symbol {
                            name,
                            kind: "function".to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                        });
                    }
                }
                if line.contains("struct ") {
                    if let Some(name) = extract_name(line, "struct ") {
                        symbols.push(Symbol {
                            name,
                            kind: "struct".to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }
    symbols
}

fn extract_name(line: &str, keyword: &str) -> Option<String> {
    line.split(keyword)
        .nth(1)?
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn find_symbol_exact<'a>(symbols: &'a [Symbol], name: &str) -> Option<&'a Symbol> {
    symbols.iter().find(|s| s.name == name)
}

fn find_symbol_prefix<'a>(symbols: &'a [Symbol], prefix: &str) -> Vec<&'a Symbol> {
    symbols.iter().filter(|s| s.name.starts_with(prefix)).collect()
}

fn find_symbol_fuzzy<'a>(symbols: &'a [Symbol], query: &str) -> Vec<&'a Symbol> {
    // Simplified fuzzy matching - just check if all chars appear in order
    symbols
        .iter()
        .filter(|s| {
            let mut query_chars = query.chars().peekable();
            for c in s.name.chars() {
                if query_chars.peek() == Some(&c) {
                    query_chars.next();
                }
            }
            query_chars.peek().is_none()
        })
        .collect()
}

criterion_group!(
    benches,
    bench_bm25_search,
    bench_tfidf_search,
    bench_hybrid_search,
    bench_symbol_search,
    bench_filtered_search,
);
criterion_main!(benches);
