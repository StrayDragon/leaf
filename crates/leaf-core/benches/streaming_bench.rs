use leaf_core::streaming::StreamingRenderer;
use leaf_core::MarkdownRenderer;
use std::time::{Duration, Instant};

fn generate_markdown(size_kb: usize) -> String {
    let block = "# Heading\n\nThis is a paragraph with **bold**, *italic*, and `code`.\n\n\
        - Item 1\n- Item 2\n- Item 3\n\n\
        ```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n\
        > Blockquote with some content\n\n\
        | Col A | Col B |\n|-------|-------|\n| 1     | 2     |\n\n";
    let repeat = (size_kb * 1024) / block.len() + 1;
    block.repeat(repeat)
}

fn bench_full_reparse(source: &str, width: usize) -> Duration {
    let renderer = MarkdownRenderer::new();
    let start = Instant::now();
    let _output = renderer.render(source, width);
    start.elapsed()
}

fn bench_streaming_tick(source: &str, width: usize, chunk_size: usize) -> (Duration, usize) {
    let renderer = MarkdownRenderer::new();
    let mut stream = StreamingRenderer::new(renderer, width)
        .with_debounce(Duration::ZERO);

    let mut total_ticks = 0usize;
    let start = Instant::now();

    let mut offset = 0;
    while offset < source.len() {
        let end = (offset + chunk_size).min(source.len());
        let boundary = source[offset..end]
            .char_indices()
            .last()
            .map(|(i, c)| offset + i + c.len_utf8())
            .unwrap_or(end);
        stream.push(&source[offset..boundary]);
        if stream.tick().is_some() {
            total_ticks += 1;
        }
        offset = boundary;
    }

    let _ = stream.finish();
    total_ticks += 1;

    (start.elapsed(), total_ticks)
}

fn bench_streaming_debounced(source: &str, width: usize, chunk_size: usize) -> (Duration, usize) {
    let renderer = MarkdownRenderer::new();
    let mut stream = StreamingRenderer::new(renderer, width)
        .with_debounce(Duration::from_millis(150));

    let mut total_ticks = 0usize;
    let start = Instant::now();

    let mut offset = 0;
    while offset < source.len() {
        let end = (offset + chunk_size).min(source.len());
        let boundary = source[offset..end]
            .char_indices()
            .last()
            .map(|(i, c)| offset + i + c.len_utf8())
            .unwrap_or(end);
        stream.push(&source[offset..boundary]);
        if stream.tick().is_some() {
            total_ticks += 1;
        }
        offset = boundary;
    }

    let _ = stream.finish();
    total_ticks += 1;

    (start.elapsed(), total_ticks)
}

fn main() {
    println!("leaf-core Streaming Benchmark");
    println!("=============================\n");

    let sizes = [1, 5, 10, 25, 50, 100];
    let width = 80;

    println!("## Full Reparse (one-shot)\n");
    println!("| Size | Time | Throughput |");
    println!("|------|------|------------|");
    for &size in &sizes {
        let source = generate_markdown(size);
        let actual_kb = source.len() / 1024;

        let mut times = Vec::new();
        for _ in 0..5 {
            times.push(bench_full_reparse(&source, width));
        }
        times.sort();
        let median = times[2];

        let throughput_mb = (source.len() as f64 / 1024.0 / 1024.0) / median.as_secs_f64();
        println!(
            "| ~{}KB ({} bytes) | {:.2}ms | {:.1} MB/s |",
            actual_kb,
            source.len(),
            median.as_secs_f64() * 1000.0,
            throughput_mb,
        );
    }

    println!("\n## Streaming (no debounce, ~50 byte chunks)\n");
    println!("| Size | Total Time | Ticks | Avg per tick |");
    println!("|------|------------|-------|--------------|");
    for &size in &sizes {
        let source = generate_markdown(size);
        let actual_kb = source.len() / 1024;

        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(bench_streaming_tick(&source, width, 50));
        }
        results.sort_by_key(|(d, _)| *d);
        let (median_time, ticks) = results[1];
        let avg_per_tick = if ticks > 0 {
            median_time.as_secs_f64() * 1000.0 / ticks as f64
        } else {
            0.0
        };
        println!(
            "| ~{}KB | {:.1}ms | {} | {:.2}ms |",
            actual_kb,
            median_time.as_secs_f64() * 1000.0,
            ticks,
            avg_per_tick,
        );
    }

    println!("\n## Streaming (150ms debounce, ~50 byte chunks)\n");
    println!("| Size | Total Time | Actual Reparses | Skipped |");
    println!("|------|------------|-----------------|---------|");
    for &size in &[1, 5, 10] {
        let source = generate_markdown(size);
        let actual_kb = source.len() / 1024;
        let total_chunks = source.len() / 50 + 1;

        let (time, ticks) = bench_streaming_debounced(&source, width, 50);
        let skipped = total_chunks.saturating_sub(ticks);
        println!(
            "| ~{}KB | {:.1}ms | {} of {} | {} |",
            actual_kb,
            time.as_secs_f64() * 1000.0,
            ticks,
            total_chunks,
            skipped,
        );
    }
}
