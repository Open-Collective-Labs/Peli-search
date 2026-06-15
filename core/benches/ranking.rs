use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use pelisearch_core::index::InvertedIndex;
use pelisearch_core::ranking::statistics::CollectionStats;
use pelisearch_core::search;

const DICTIONARY: &[&str] = &[
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
    "hello", "world", "rust", "programming", "language", "systems",
    "performance", "safety", "concurrency", "memory", "management",
    "zero", "cost", "abstraction", "trait", "generic", "lifetime",
    "async", "await", "future", "stream", "iterator", "closure",
    "macro", "module", "crate", "package", "dependency", "ecosystem",
    "compiler", "optimization", "release", "debug", "build", "test",
    "benchmark", "documentation", "example", "guide", "tutorial",
    "electric", "bike", "review", "commuting", "walking", "park",
    "search", "index", "tokenize", "rank", "score", "document",
    "collection", "statistics", "frequency", "inverse", "term",
];

struct BenchData {
    index: InvertedIndex,
    stats: CollectionStats,
    single_term: String,
    multi_term: String,
    match_count: usize,
}

fn build_data(num_docs: usize) -> BenchData {
    let mut index = InvertedIndex::new();
    let mut stats = CollectionStats::new();

    for i in 0..num_docs {
        let doc_id = format!("doc_{}", i);
        let num_words = 5 + (i % 20); // 5..24 words per doc
        let mut words = Vec::with_capacity(num_words);
        for j in 0..num_words {
            let idx = (i * 31 + j * 17) % DICTIONARY.len();
            words.push(DICTIONARY[idx]);
        }
        let text = words.join(" ");
        index.add_document(&doc_id, &text).unwrap();
        stats.update_document(&doc_id, &text);
    }

    // Count documents matching the single term query to report throughput
    let single = DICTIONARY[0];
    let match_count = index
        .get_postings(single)
        .map_or(0, |p| p.len());

    BenchData {
        index,
        stats,
        single_term: single.to_string(),
        multi_term: format!(
            "{} {} {}",
            DICTIONARY[0], DICTIONARY[1], DICTIONARY[2]
        ),
        match_count,
    }
}

fn bench_ranking(c: &mut Criterion) {
    let sizes = [100usize, 10_000, 100_000];

    for &size in &sizes {
        let data = build_data(size);
        let group_name = format!("{}_docs", size);

        let mut latency_group = c.benchmark_group(format!("{}/latency", group_name));
        latency_group.throughput(Throughput::Elements(1));

        latency_group.bench_with_input(
            BenchmarkId::new("single_term", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.single_term),
                    )
                });
            },
        );

        latency_group.bench_with_input(
            BenchmarkId::new("multi_term", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.multi_term),
                    )
                });
            },
        );

        latency_group.bench_with_input(
            BenchmarkId::new("with_explanations", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search_with_explanations(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.single_term),
                    )
                });
            },
        );

        latency_group.finish();

        let mut throughput_group =
            c.benchmark_group(format!("{}/throughput", group_name));
        throughput_group.throughput(Throughput::Elements(data.match_count as u64));

        throughput_group.bench_with_input(
            BenchmarkId::new("docs_scored_per_second", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.single_term),
                    )
                });
            },
        );

        throughput_group.finish();
    }
}

fn bench_and_vs_or_multi_term(c: &mut Criterion) {
    let sizes = [100usize, 10_000, 100_000];

    for &size in &sizes {
        let data = build_data(size);
        let group_name = format!("and_vs_or_{}_docs", size);

        let mut group = c.benchmark_group(&group_name);
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::new("search_any_OR", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search_any(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.multi_term),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("search_AND_optimized", size),
            &data,
            |b, d| {
                b.iter(|| {
                    search::search(
                        black_box(&d.index),
                        black_box(&d.stats),
                        black_box(&d.multi_term),
                    )
                });
            },
        );

        group.finish();

        // Verify the optimization produces fewer scored docs
        let or_results = search::search_any(&data.index, &data.stats, &data.multi_term);
        let and_results = search::search(&data.index, &data.stats, &data.multi_term);
        let or_count = or_results.len();
        let and_count = and_results.len();

        // Print for visual verification
        eprintln!(
            "  {group_name}: OR={or_count} docs, AND={and_count} docs, ratio={:.2}%",
            if or_count > 0 {
                and_count as f64 / or_count as f64 * 100.0
            } else {
                0.0
            }
        );
    }
}

criterion_group!(
    benches,
    bench_ranking,
    bench_and_vs_or_multi_term
);
criterion_main!(benches);
