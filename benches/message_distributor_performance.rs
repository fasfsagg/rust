// 【性能基准测试】消息分发器性能优化基准测试
//
// 【功能】: 测试消息批处理优化的性能改进效果
// 【目标】: 验证零拷贝、压缩和动态批处理的性能提升
// 【基准】: 对比优化前后的吞吐量和延迟

use axum_tutorial::app::service::message_distributor::{
    CompressionConfig, CompressionType, DynamicBatchConfig,
};
use axum_tutorial::app::service::{ConnectionManager, MessageDistributor};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// 创建测试用的消息分发器（未优化版本）
async fn create_basic_distributor() -> MessageDistributor {
    let connection_manager = Arc::new(ConnectionManager::new());
    MessageDistributor::new(connection_manager, Some(100), Some(4))
}

/// 创建高性能优化的消息分发器
async fn create_optimized_distributor() -> MessageDistributor {
    let connection_manager = Arc::new(ConnectionManager::new());

    let compression_config = CompressionConfig {
        enabled: true,
        level: 6,
        min_size_threshold: 512, // 512字节阈值
        compression_type: CompressionType::Gzip,
    };

    let dynamic_batch_config = DynamicBatchConfig {
        min_batch_size: 50,
        max_batch_size: 2000,
        current_batch_size: std::sync::atomic::AtomicUsize::new(200),
        load_threshold: 1000,
        adjustment_factor: 2.0,
    };

    MessageDistributor::new_high_performance(
        connection_manager,
        compression_config,
        dynamic_batch_config,
        8, // 更多工作线程
    )
}

/// 生成测试消息
fn generate_test_messages(count: usize, size: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("Test message {} - {}", i, "A".repeat(size)))
        .collect()
}

/// 基准测试：零拷贝消息优化
fn bench_zero_copy_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("zero_copy_optimization");

    for message_size in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("basic", message_size),
            message_size,
            |b, &size| {
                let _distributor = rt.block_on(create_basic_distributor());
                let message = "A".repeat(size);

                b.iter(|| {
                    // 模拟基础消息处理
                    black_box(message.as_bytes().to_vec())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized", message_size),
            message_size,
            |b, &size| {
                let distributor = rt.block_on(create_optimized_distributor());
                let message = "A".repeat(size);

                b.iter(|| {
                    // 使用零拷贝优化
                    black_box(distributor.optimize_message(&message).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：动态批处理大小调整
fn bench_dynamic_batch_sizing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dynamic_batch_sizing");

    for queue_length in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("static_batch", queue_length),
            queue_length,
            |b, &length| {
                let _distributor = rt.block_on(create_basic_distributor());

                b.iter(|| {
                    // 静态批处理大小
                    black_box(std::cmp::min(100, length))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("dynamic_batch", queue_length),
            queue_length,
            |b, &length| {
                let distributor = rt.block_on(create_optimized_distributor());

                b.iter(|| {
                    // 动态批处理大小调整
                    black_box(distributor.adjust_batch_size(length))
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：消息压缩性能
fn bench_message_compression(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_compression");

    for message_size in [512, 1024, 2048, 4096].iter() {
        let message = "Hello World! ".repeat(*message_size / 13); // 创建可压缩的重复内容

        group.bench_with_input(
            BenchmarkId::new("no_compression", message_size),
            &message,
            |b, msg| {
                b.iter(|| {
                    // 无压缩处理
                    black_box(msg.as_bytes().to_vec())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_compression", message_size),
            &message,
            |b, msg| {
                let distributor = rt.block_on(create_optimized_distributor());

                b.iter(|| {
                    // 带压缩的优化处理
                    black_box(distributor.optimize_message(msg).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：批量消息处理吞吐量
fn bench_batch_processing_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_processing_throughput");
    group.sample_size(10); // 减少样本数量，因为这是较重的测试

    for batch_size in [50, 100, 200, 500].iter() {
        let messages = generate_test_messages(*batch_size, 200);

        group.bench_with_input(
            BenchmarkId::new("basic_processing", batch_size),
            &messages,
            |b, msgs| {
                let _distributor = rt.block_on(create_basic_distributor());

                b.iter(|| {
                    // 基础批处理
                    for msg in msgs {
                        black_box(msg.as_bytes().to_vec());
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_processing", batch_size),
            &messages,
            |b, msgs| {
                let distributor = rt.block_on(create_optimized_distributor());

                b.iter(|| {
                    // 优化的批处理
                    for msg in msgs {
                        black_box(distributor.optimize_message(msg).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：性能指标收集开销
fn bench_performance_metrics_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("performance_metrics_overhead");

    group.bench_function("without_metrics", |b| {
        b.iter(|| {
            // 无性能指标收集
            black_box("test message".as_bytes().to_vec())
        });
    });

    group.bench_function("with_metrics", |b| {
        let distributor = rt.block_on(create_optimized_distributor());

        b.iter(|| {
            // 带性能指标收集
            black_box(distributor.optimize_message("test message").unwrap());
            black_box(distributor.get_performance_metrics());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_zero_copy_optimization,
    bench_dynamic_batch_sizing,
    bench_message_compression,
    bench_batch_processing_throughput,
    bench_performance_metrics_overhead
);

criterion_main!(benches);
