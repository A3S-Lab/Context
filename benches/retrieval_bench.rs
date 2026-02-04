use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let dim = 1536;
    let a: Vec<f32> = (0..dim).map(|i| (i as f32).sin()).collect();
    let b: Vec<f32> = (0..dim).map(|i| (i as f32).cos()).collect();

    c.bench_function("cosine_similarity_1536d", |bencher| {
        bencher.iter(|| cosine_similarity(black_box(&a), black_box(&b)))
    });
}

fn bench_vector_search(c: &mut Criterion) {
    let dim = 1536;
    let num_vectors = 10000;

    let query: Vec<f32> = (0..dim).map(|i| (i as f32).sin()).collect();
    let vectors: Vec<Vec<f32>> = (0..num_vectors)
        .map(|j| (0..dim).map(|i| ((i + j) as f32).cos()).collect())
        .collect();

    c.bench_function("vector_search_10k", |bencher| {
        bencher.iter(|| {
            let mut scores: Vec<(usize, f32)> = vectors
                .iter()
                .enumerate()
                .map(|(i, v)| (i, cosine_similarity(&query, v)))
                .collect();
            scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            scores.truncate(10);
            black_box(scores)
        })
    });
}

criterion_group!(benches, bench_cosine_similarity, bench_vector_search);
criterion_main!(benches);
