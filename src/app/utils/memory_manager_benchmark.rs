//! 内存管理器性能基准测试
//!
//! 【功能】: 验证内存管理器的性能优化效果
//! 【目标】: 测量缓存命中率、内存使用量、响应时间等关键指标

use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

use super::memory_manager::{L1MemoryCache, MemoryManager, MemoryManagerConfig};

/// 内存管理器性能基准测试
pub struct MemoryManagerBenchmark {
    /// 内存管理器
    memory_manager: MemoryManager,
}

impl MemoryManagerBenchmark {
    /// 创建新的基准测试实例
    pub fn new() -> Self {
        let config = MemoryManagerConfig {
            l1_cache_max_entries: 1000,
            l1_cache_ttl_seconds: 300,
            object_pool_initial_size: 100,
            object_pool_max_size: 500,
            memory_pool_block_size: 4096,
            memory_pool_max_blocks: 100,
            cache_eviction_interval_seconds: 60,
            memory_monitoring_interval_seconds: 10,
            memory_pressure_threshold_bytes: 100 * 1024 * 1024, // 100MB
            enable_leak_detection: true,
        };

        let memory_manager = MemoryManager::new(config);

        Self { memory_manager }
    }

    /// 运行L1缓存性能测试
    pub async fn benchmark_l1_cache(&self) -> CacheBenchmarkResult {
        info!("开始L1缓存性能测试");

        let start_time = Instant::now();
        let mut total_operations = 0;
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        // 预热缓存
        for i in 0..100 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            self.memory_manager.string_cache.set(key, value, 50);
            total_operations += 1;
        }

        // 执行混合读写操作
        for round in 0..10 {
            // 读取操作（80%命中率）
            for i in 0..1000 {
                let key = if i % 5 == 0 {
                    format!("key_{}", i % 200) // 20%未命中
                } else {
                    format!("key_{}", i % 100) // 80%命中
                };

                if self.memory_manager.string_cache.get(&key).is_some() {
                    cache_hits += 1;
                } else {
                    cache_misses += 1;
                }
                total_operations += 1;
            }

            // 写入操作
            for i in 0..200 {
                let key = format!("key_{}_{}", round, i);
                let value = format!("value_{}_{}", round, i);
                self.memory_manager.string_cache.set(key, value, 50);
                total_operations += 1;
            }

            // 短暂休息（减少延迟以提高性能）
            sleep(Duration::from_millis(1)).await;
        }

        let duration = start_time.elapsed();
        let stats = self.memory_manager.string_cache.get_stats();

        info!(
            duration_ms = duration.as_millis(),
            total_operations = total_operations,
            cache_hit_rate = stats.hit_rate,
            "L1缓存性能测试完成"
        );

        CacheBenchmarkResult {
            duration,
            total_operations,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            hit_rate: stats.hit_rate,
            operations_per_second: (total_operations as f64) / duration.as_secs_f64(),
        }
    }

    /// 运行对象池性能测试
    pub async fn benchmark_object_pool(&self) -> ObjectPoolBenchmarkResult {
        info!("开始对象池性能测试");

        let start_time = Instant::now();
        let mut total_operations = 0;

        // 执行对象获取和释放操作
        for _round in 0..100 {
            let mut objects = Vec::new();

            // 获取对象
            for _i in 0..50 {
                let obj = self.memory_manager.message_pool.acquire();
                objects.push(obj);
                total_operations += 1;
            }

            // 释放对象
            for obj in objects {
                let inner = obj.into_inner();
                self.memory_manager.message_pool.release(inner);
                total_operations += 1;
            }

            // 短暂休息
            sleep(Duration::from_millis(1)).await;
        }

        let duration = start_time.elapsed();
        let stats = self.memory_manager.message_pool.get_stats();

        info!(
            duration_ms = duration.as_millis(),
            total_operations = total_operations,
            pool_hit_rate = stats.hit_rate,
            "对象池性能测试完成"
        );

        ObjectPoolBenchmarkResult {
            duration,
            total_operations,
            pool_hits: stats.pool_hits,
            pool_misses: stats.pool_misses,
            hit_rate: stats.hit_rate,
            operations_per_second: (total_operations as f64) / duration.as_secs_f64(),
        }
    }

    /// 运行内存池性能测试
    pub async fn benchmark_memory_pool(&self) -> MemoryPoolBenchmarkResult {
        info!("开始内存池性能测试");

        let start_time = Instant::now();
        let mut total_operations = 0;

        // 执行内存分配和释放操作
        for _round in 0..100 {
            let mut blocks = Vec::new();

            // 分配内存块
            for _i in 0..20 {
                if let Some(block) = self.memory_manager.memory_pool.allocate() {
                    blocks.push(block);
                }
                total_operations += 1;
            }

            // 释放内存块
            for block in blocks {
                self.memory_manager.memory_pool.deallocate(block);
                total_operations += 1;
            }

            // 短暂休息
            sleep(Duration::from_millis(1)).await;
        }

        let duration = start_time.elapsed();
        let stats = self.memory_manager.memory_pool.get_stats();

        info!(
            duration_ms = duration.as_millis(),
            total_operations = total_operations,
            memory_pool_hit_rate = stats.hit_rate,
            "内存池性能测试完成"
        );

        MemoryPoolBenchmarkResult {
            duration,
            total_operations,
            pool_hits: stats.pool_hits,
            pool_misses: stats.pool_misses,
            hit_rate: stats.hit_rate,
            operations_per_second: (total_operations as f64) / duration.as_secs_f64(),
        }
    }

    /// 运行综合性能测试
    pub async fn run_comprehensive_benchmark(&self) -> ComprehensiveBenchmarkResult {
        info!("开始综合性能测试");

        // 启动内存管理器
        self.memory_manager.start().await;

        let start_time = Instant::now();

        // 并发执行各种测试
        let (cache_result, pool_result, memory_result) = tokio::join!(
            self.benchmark_l1_cache(),
            self.benchmark_object_pool(),
            self.benchmark_memory_pool()
        );

        let total_duration = start_time.elapsed();

        // 获取综合统计信息
        let comprehensive_stats = self.memory_manager.get_comprehensive_stats();
        let health_status = self.memory_manager.check_health();

        // 停止内存管理器
        self.memory_manager.stop();

        info!(
            total_duration_ms = total_duration.as_millis(),
            overall_health = health_status.is_healthy,
            "综合性能测试完成"
        );

        if !health_status.is_healthy {
            warn!(issues = ?health_status.issues, "检测到内存管理问题");
        }

        ComprehensiveBenchmarkResult {
            total_duration,
            cache_benchmark: cache_result,
            object_pool_benchmark: pool_result,
            memory_pool_benchmark: memory_result,
            comprehensive_stats,
            health_status,
        }
    }
}

/// 缓存基准测试结果
#[derive(Debug, Clone)]
pub struct CacheBenchmarkResult {
    pub duration: Duration,
    pub total_operations: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub operations_per_second: f64,
}

/// 对象池基准测试结果
#[derive(Debug, Clone)]
pub struct ObjectPoolBenchmarkResult {
    pub duration: Duration,
    pub total_operations: u64,
    pub pool_hits: u64,
    pub pool_misses: u64,
    pub hit_rate: f64,
    pub operations_per_second: f64,
}

/// 内存池基准测试结果
#[derive(Debug, Clone)]
pub struct MemoryPoolBenchmarkResult {
    pub duration: Duration,
    pub total_operations: u64,
    pub pool_hits: u64,
    pub pool_misses: u64,
    pub hit_rate: f64,
    pub operations_per_second: f64,
}

/// 综合基准测试结果
#[derive(Debug, Clone)]
pub struct ComprehensiveBenchmarkResult {
    pub total_duration: Duration,
    pub cache_benchmark: CacheBenchmarkResult,
    pub object_pool_benchmark: ObjectPoolBenchmarkResult,
    pub memory_pool_benchmark: MemoryPoolBenchmarkResult,
    pub comprehensive_stats: super::memory_manager::MemoryManagerStats,
    pub health_status: super::memory_manager::MemoryHealthStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_manager_benchmark() {
        let benchmark = MemoryManagerBenchmark::new();
        let result = benchmark.run_comprehensive_benchmark().await;

        // 验证性能指标
        assert!(
            result.cache_benchmark.hit_rate > 50.0,
            "缓存命中率应该大于50%"
        );
        assert!(
            result.object_pool_benchmark.hit_rate > 30.0,
            "对象池命中率应该大于30%"
        );
        assert!(
            result.memory_pool_benchmark.hit_rate > 20.0,
            "内存池命中率应该大于20%"
        );

        // 验证操作性能
        assert!(
            result.cache_benchmark.operations_per_second > 100.0,
            "缓存操作应该大于100 ops/s"
        );
        assert!(
            result.object_pool_benchmark.operations_per_second > 50.0,
            "对象池操作应该大于50 ops/s"
        );
        assert!(
            result.memory_pool_benchmark.operations_per_second > 20.0,
            "内存池操作应该大于20 ops/s"
        );

        println!("基准测试结果:");
        println!("缓存命中率: {:.2}%", result.cache_benchmark.hit_rate);
        println!(
            "对象池命中率: {:.2}%",
            result.object_pool_benchmark.hit_rate
        );
        println!(
            "内存池命中率: {:.2}%",
            result.memory_pool_benchmark.hit_rate
        );
        println!("总体健康状态: {}", result.health_status.is_healthy);
    }
}
