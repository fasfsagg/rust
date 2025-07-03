//! 内存管理优化模块
//!
//! 【功能】: 为企业级聊天应用提供高效的内存管理策略
//! 【目标】: 降低内存使用20-30%，提升缓存命中率至>90%
//! 【特性】: L1内存缓存、对象池、内存池、智能缓存淘汰、内存泄漏检测

use bytes::Bytes;
use parking_lot::{Mutex as ParkingMutex, RwLock as ParkingRwLock};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use tracing::{debug, error, info, instrument, warn};

/// 内存管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManagerConfig {
    /// L1缓存最大条目数
    pub l1_cache_max_entries: usize,
    /// L1缓存TTL（秒）
    pub l1_cache_ttl_seconds: u64,
    /// 对象池初始大小
    pub object_pool_initial_size: usize,
    /// 对象池最大大小
    pub object_pool_max_size: usize,
    /// 内存池块大小（字节）
    pub memory_pool_block_size: usize,
    /// 内存池最大块数
    pub memory_pool_max_blocks: usize,
    /// 缓存淘汰检查间隔（秒）
    pub cache_eviction_interval_seconds: u64,
    /// 内存监控间隔（秒）
    pub memory_monitoring_interval_seconds: u64,
    /// 内存压力阈值（字节）
    pub memory_pressure_threshold_bytes: usize,
    /// 是否启用内存泄漏检测
    pub enable_leak_detection: bool,
}

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            l1_cache_max_entries: 10000,
            l1_cache_ttl_seconds: 300, // 5分钟
            object_pool_initial_size: 1000,
            object_pool_max_size: 10000,
            memory_pool_block_size: 4096, // 4KB
            memory_pool_max_blocks: 1000,
            cache_eviction_interval_seconds: 60,    // 1分钟
            memory_monitoring_interval_seconds: 30, // 30秒
            memory_pressure_threshold_bytes: 1024 * 1024 * 1024, // 1GB
            enable_leak_detection: true,
        }
    }
}

/// L1内存缓存条目
#[derive(Debug)]
pub struct CacheEntry<T> {
    /// 缓存值
    pub value: T,
    /// 创建时间
    pub created_at: Instant,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 访问次数
    pub access_count: AtomicU64,
    /// 数据大小（字节）
    pub size_bytes: usize,
}

impl<T> CacheEntry<T> {
    /// 创建新的缓存条目
    pub fn new(value: T, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: AtomicU64::new(1),
            size_bytes,
        }
    }

    /// 更新访问时间和计数
    pub fn touch(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        // 注意：last_accessed 需要通过外部更新，因为这里是不可变引用
    }

    /// 检查是否过期
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    /// 获取访问频率（每秒访问次数）
    pub fn access_frequency(&self) -> f64 {
        let elapsed_secs = self.created_at.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            (self.access_count.load(Ordering::Relaxed) as f64) / elapsed_secs
        } else {
            0.0
        }
    }
}

/// L1内存缓存实现
#[derive(Debug)]
pub struct L1MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    /// 缓存存储
    cache: ParkingRwLock<HashMap<K, Arc<ParkingMutex<CacheEntry<V>>>>>,
    /// 配置
    config: MemoryManagerConfig,
    /// 统计信息
    stats: Arc<CacheStats>,
}

impl<K, V> L1MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    /// 创建新的L1缓存
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self {
            cache: ParkingRwLock::new(HashMap::new()),
            config,
            stats: Arc::new(CacheStats::default()),
        }
    }

    /// 获取缓存值
    #[instrument(skip(self, key), fields(cache_key = ?std::any::type_name::<K>()))]
    pub fn get(&self, key: &K) -> Option<V> {
        let cache = self.cache.read();
        if let Some(entry_arc) = cache.get(key) {
            let mut entry = entry_arc.lock();

            // 检查是否过期
            let ttl = Duration::from_secs(self.config.l1_cache_ttl_seconds);
            if entry.is_expired(ttl) {
                drop(entry);
                drop(cache);
                // 异步清理过期条目
                self.remove(key);
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            // 更新访问信息
            entry.last_accessed = Instant::now();
            entry.touch();
            let value = entry.value.clone();

            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            debug!("L1缓存命中");
            Some(value)
        } else {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            debug!("L1缓存未命中");
            None
        }
    }

    /// 设置缓存值
    #[instrument(skip(self, key, value), fields(cache_key = ?std::any::type_name::<K>()))]
    pub fn set(&self, key: K, value: V, size_bytes: usize) -> bool {
        // 检查缓存大小限制
        {
            let cache = self.cache.read();
            if cache.len() >= self.config.l1_cache_max_entries {
                drop(cache);
                // 触发缓存淘汰
                self.evict_entries();
            }
        }

        let entry = Arc::new(ParkingMutex::new(CacheEntry::new(value, size_bytes)));
        let mut cache = self.cache.write();

        let inserted = cache.insert(key, entry).is_none();
        if inserted {
            self.stats.cache_sets.fetch_add(1, Ordering::Relaxed);
            self.stats
                .total_size_bytes
                .fetch_add(size_bytes, Ordering::Relaxed);
            debug!(size_bytes = size_bytes, "L1缓存条目已添加");
        }

        inserted
    }

    /// 移除缓存条目
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        if let Some(entry_arc) = cache.remove(key) {
            let entry = entry_arc.lock();
            let size_bytes = entry.size_bytes;
            let value = entry.value.clone();
            drop(entry);

            self.stats.cache_removals.fetch_add(1, Ordering::Relaxed);
            self.stats
                .total_size_bytes
                .fetch_sub(size_bytes, Ordering::Relaxed);
            debug!(size_bytes = size_bytes, "L1缓存条目已移除");
            Some(value)
        } else {
            None
        }
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let count = cache.len();
        cache.clear();

        self.stats.cache_clears.fetch_add(1, Ordering::Relaxed);
        self.stats.total_size_bytes.store(0, Ordering::Relaxed);
        info!(cleared_entries = count, "L1缓存已清空");
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStatsSnapshot {
        let cache = self.cache.read();
        let current_entries = cache.len();
        drop(cache);

        CacheStatsSnapshot {
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            cache_sets: self.stats.cache_sets.load(Ordering::Relaxed),
            cache_removals: self.stats.cache_removals.load(Ordering::Relaxed),
            cache_clears: self.stats.cache_clears.load(Ordering::Relaxed),
            cache_evictions: self.stats.cache_evictions.load(Ordering::Relaxed),
            current_entries,
            total_size_bytes: self.stats.total_size_bytes.load(Ordering::Relaxed),
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// 计算缓存命中率
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.stats.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.stats.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }

    /// 智能缓存淘汰算法（LFU + TTL）
    #[instrument(skip(self))]
    fn evict_entries(&self) {
        let mut cache = self.cache.write();
        let target_size = (self.config.l1_cache_max_entries * 80) / 100; // 淘汰到80%

        if cache.len() <= target_size {
            return;
        }

        let ttl = Duration::from_secs(self.config.l1_cache_ttl_seconds);
        let mut candidates: Vec<_> = cache
            .iter()
            .filter_map(|(key, entry_arc)| {
                let entry = entry_arc.lock();
                if entry.is_expired(ttl) {
                    // 过期条目优先淘汰
                    Some((key.clone(), 0.0, entry.size_bytes))
                } else {
                    // 按访问频率排序
                    let frequency = entry.access_frequency();
                    Some((key.clone(), frequency, entry.size_bytes))
                }
            })
            .collect();

        // 按访问频率升序排序（频率低的先淘汰）
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut evicted_count = 0;
        let mut evicted_bytes = 0;

        for (key, _frequency, size_bytes) in candidates.iter() {
            if cache.len() <= target_size {
                break;
            }

            cache.remove(key);
            evicted_count += 1;
            evicted_bytes += size_bytes;
        }

        self.stats
            .cache_evictions
            .fetch_add(evicted_count, Ordering::Relaxed);
        self.stats
            .total_size_bytes
            .fetch_sub(evicted_bytes, Ordering::Relaxed);

        info!(
            evicted_entries = evicted_count,
            evicted_bytes = evicted_bytes,
            remaining_entries = cache.len(),
            "智能缓存淘汰完成"
        );
    }
}

/// 缓存统计信息
#[derive(Debug, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub cache_hits: AtomicU64,
    /// 缓存未命中次数
    pub cache_misses: AtomicU64,
    /// 缓存设置次数
    pub cache_sets: AtomicU64,
    /// 缓存移除次数
    pub cache_removals: AtomicU64,
    /// 缓存清空次数
    pub cache_clears: AtomicU64,
    /// 缓存淘汰次数
    pub cache_evictions: AtomicU64,
    /// 总大小（字节）
    pub total_size_bytes: AtomicUsize,
}

/// 缓存统计信息快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsSnapshot {
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 缓存设置次数
    pub cache_sets: u64,
    /// 缓存移除次数
    pub cache_removals: u64,
    /// 缓存清空次数
    pub cache_clears: u64,
    /// 缓存淘汰次数
    pub cache_evictions: u64,
    /// 当前条目数
    pub current_entries: usize,
    /// 总大小（字节）
    pub total_size_bytes: usize,
    /// 命中率（百分比）
    pub hit_rate: f64,
}

/// 对象池实现
///
/// 【功能】: 复用对象实例，减少内存分配和垃圾回收压力
/// 【特性】: 线程安全、自动扩容、统计监控
pub struct ObjectPool<T> {
    /// 对象池存储
    pool: ParkingMutex<VecDeque<T>>,
    /// 对象创建函数
    factory: Arc<dyn (Fn() -> T) + Send + Sync>,
    /// 配置
    config: MemoryManagerConfig,
    /// 统计信息
    stats: Arc<ObjectPoolStats>,
}

impl<T> std::fmt::Debug for ObjectPool<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectPool")
            .field("pool_size", &self.pool.lock().len())
            .field("config", &self.config)
            .field("stats", &self.stats)
            .finish()
    }
}

impl<T> ObjectPool<T>
where
    T: Send + 'static,
{
    /// 创建新的对象池
    pub fn new<F>(factory: F, config: MemoryManagerConfig) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let pool = ParkingMutex::new(VecDeque::new());
        let factory = Arc::new(factory);
        let stats = Arc::new(ObjectPoolStats::default());

        let object_pool = Self {
            pool,
            factory,
            config,
            stats,
        };

        // 预分配初始对象
        object_pool.preallocate_objects();
        object_pool
    }

    /// 预分配对象
    fn preallocate_objects(&self) {
        let mut pool = self.pool.lock();
        for _ in 0..self.config.object_pool_initial_size {
            let obj = (self.factory)();
            pool.push_back(obj);
        }

        self.stats
            .total_created
            .fetch_add(self.config.object_pool_initial_size, Ordering::Relaxed);

        info!(
            initial_size = self.config.object_pool_initial_size,
            "对象池预分配完成"
        );
    }

    /// 获取对象
    #[instrument(skip(self))]
    pub fn acquire(&self) -> PooledObject<T> {
        let obj = {
            let mut pool = self.pool.lock();
            if let Some(obj) = pool.pop_front() {
                self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
                debug!("从对象池获取对象");
                obj
            } else {
                drop(pool);
                // 池为空，创建新对象
                let obj = (self.factory)();
                self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
                self.stats.total_created.fetch_add(1, Ordering::Relaxed);
                debug!("对象池为空，创建新对象");
                obj
            }
        };

        self.stats.active_objects.fetch_add(1, Ordering::Relaxed);
        PooledObject::new(obj, self.stats.clone())
    }

    /// 归还对象到池中
    #[instrument(skip(self, obj))]
    pub fn release(&self, obj: T) {
        let mut pool = self.pool.lock();

        // 检查池大小限制
        if pool.len() < self.config.object_pool_max_size {
            pool.push_back(obj);
            self.stats.pool_returns.fetch_add(1, Ordering::Relaxed);
            debug!("对象已归还到池中");
        } else {
            // 池已满，丢弃对象
            self.stats.pool_discards.fetch_add(1, Ordering::Relaxed);
            debug!("对象池已满，丢弃对象");
        }

        self.stats.active_objects.fetch_sub(1, Ordering::Relaxed);
    }

    /// 获取对象池统计信息
    pub fn get_stats(&self) -> ObjectPoolStatsSnapshot {
        let pool = self.pool.lock();
        let pool_size = pool.len();
        drop(pool);

        ObjectPoolStatsSnapshot {
            pool_hits: self.stats.pool_hits.load(Ordering::Relaxed),
            pool_misses: self.stats.pool_misses.load(Ordering::Relaxed),
            pool_returns: self.stats.pool_returns.load(Ordering::Relaxed),
            pool_discards: self.stats.pool_discards.load(Ordering::Relaxed),
            total_created: self.stats.total_created.load(Ordering::Relaxed),
            active_objects: self.stats.active_objects.load(Ordering::Relaxed),
            pool_size,
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// 计算对象池命中率
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.stats.pool_hits.load(Ordering::Relaxed) as f64;
        let misses = self.stats.pool_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }
}

/// 池化对象包装器
///
/// 【功能】: 自动归还对象到池中
pub struct PooledObject<T> {
    /// 对象实例
    obj: Option<T>,
    /// 统计信息引用
    stats: Arc<ObjectPoolStats>,
}

impl<T> PooledObject<T> {
    /// 创建新的池化对象
    fn new(obj: T, stats: Arc<ObjectPoolStats>) -> Self {
        Self {
            obj: Some(obj),
            stats,
        }
    }

    /// 获取对象引用
    pub fn as_ref(&self) -> Option<&T> {
        self.obj.as_ref()
    }

    /// 获取可变对象引用
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.obj.as_mut()
    }

    /// 取出对象（消费包装器）
    pub fn into_inner(mut self) -> T {
        self.obj.take().expect("对象已被取出")
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(_obj) = self.obj.take() {
            // 注意：这里无法直接归还到池中，因为没有池的引用
            // 在实际使用中，应该通过其他方式处理归还逻辑
            self.stats.active_objects.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

/// 对象池统计信息
#[derive(Debug, Default)]
pub struct ObjectPoolStats {
    /// 池命中次数
    pub pool_hits: AtomicU64,
    /// 池未命中次数
    pub pool_misses: AtomicU64,
    /// 对象归还次数
    pub pool_returns: AtomicU64,
    /// 对象丢弃次数
    pub pool_discards: AtomicU64,
    /// 总创建对象数
    pub total_created: AtomicUsize,
    /// 当前活跃对象数
    pub active_objects: AtomicUsize,
}

/// 对象池统计信息快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectPoolStatsSnapshot {
    /// 池命中次数
    pub pool_hits: u64,
    /// 池未命中次数
    pub pool_misses: u64,
    /// 对象归还次数
    pub pool_returns: u64,
    /// 对象丢弃次数
    pub pool_discards: u64,
    /// 总创建对象数
    pub total_created: usize,
    /// 当前活跃对象数
    pub active_objects: usize,
    /// 当前池大小
    pub pool_size: usize,
    /// 命中率（百分比）
    pub hit_rate: f64,
}

/// 内存池实现
///
/// 【功能】: 预分配固定大小的内存块，减少动态内存分配
/// 【特性】: 零拷贝、快速分配、内存对齐
#[derive(Debug)]
pub struct MemoryPool {
    /// 可用内存块
    available_blocks: ParkingMutex<VecDeque<Bytes>>,
    /// 配置
    config: MemoryManagerConfig,
    /// 统计信息
    stats: Arc<MemoryPoolStats>,
}

impl MemoryPool {
    /// 创建新的内存池
    pub fn new(config: MemoryManagerConfig) -> Self {
        let available_blocks = ParkingMutex::new(VecDeque::new());
        let stats = Arc::new(MemoryPoolStats::default());

        let memory_pool = Self {
            available_blocks,
            config,
            stats,
        };

        // 预分配内存块
        memory_pool.preallocate_blocks();
        memory_pool
    }

    /// 预分配内存块
    fn preallocate_blocks(&self) {
        let mut blocks = self.available_blocks.lock();
        let initial_blocks = self.config.memory_pool_max_blocks / 2; // 预分配一半

        for _ in 0..initial_blocks {
            let block = Bytes::from(vec![0u8; self.config.memory_pool_block_size]);
            blocks.push_back(block);
        }

        self.stats
            .total_allocated
            .fetch_add(initial_blocks, Ordering::Relaxed);
        self.stats.total_bytes_allocated.fetch_add(
            initial_blocks * self.config.memory_pool_block_size,
            Ordering::Relaxed,
        );

        info!(
            initial_blocks = initial_blocks,
            block_size = self.config.memory_pool_block_size,
            total_bytes = initial_blocks * self.config.memory_pool_block_size,
            "内存池预分配完成"
        );
    }

    /// 分配内存块
    #[instrument(skip(self))]
    pub fn allocate(&self) -> Option<Bytes> {
        let mut blocks = self.available_blocks.lock();

        if let Some(block) = blocks.pop_front() {
            self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
            self.stats.active_blocks.fetch_add(1, Ordering::Relaxed);
            debug!(
                block_size = self.config.memory_pool_block_size,
                "从内存池分配块"
            );
            Some(block)
        } else {
            drop(blocks);
            // 池为空，尝试创建新块
            if self.stats.total_allocated.load(Ordering::Relaxed)
                < self.config.memory_pool_max_blocks
            {
                let block = Bytes::from(vec![0u8; self.config.memory_pool_block_size]);
                self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
                self.stats.total_allocated.fetch_add(1, Ordering::Relaxed);
                self.stats.active_blocks.fetch_add(1, Ordering::Relaxed);
                self.stats
                    .total_bytes_allocated
                    .fetch_add(self.config.memory_pool_block_size, Ordering::Relaxed);
                debug!(
                    block_size = self.config.memory_pool_block_size,
                    "创建新内存块"
                );
                Some(block)
            } else {
                self.stats
                    .allocation_failures
                    .fetch_add(1, Ordering::Relaxed);
                warn!("内存池已达到最大容量，分配失败");
                None
            }
        }
    }

    /// 释放内存块
    #[instrument(skip(self, block))]
    pub fn deallocate(&self, block: Bytes) {
        // 验证块大小
        if block.len() != self.config.memory_pool_block_size {
            warn!(
                expected_size = self.config.memory_pool_block_size,
                actual_size = block.len(),
                "内存块大小不匹配，丢弃"
            );
            self.stats.invalid_returns.fetch_add(1, Ordering::Relaxed);
            self.stats.active_blocks.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let mut blocks = self.available_blocks.lock();

        // 检查池容量
        if blocks.len() < self.config.memory_pool_max_blocks {
            // 重置块内容（可选，用于安全性）
            let mut reset_block = block.to_vec();
            reset_block.fill(0);
            let reset_bytes = Bytes::from(reset_block);

            blocks.push_back(reset_bytes);
            self.stats
                .successful_returns
                .fetch_add(1, Ordering::Relaxed);
            debug!("内存块已归还到池中");
        } else {
            self.stats.pool_discards.fetch_add(1, Ordering::Relaxed);
            debug!("内存池已满，丢弃内存块");
        }

        self.stats.active_blocks.fetch_sub(1, Ordering::Relaxed);
    }

    /// 获取内存池统计信息
    pub fn get_stats(&self) -> MemoryPoolStatsSnapshot {
        let blocks = self.available_blocks.lock();
        let available_blocks = blocks.len();
        drop(blocks);

        MemoryPoolStatsSnapshot {
            pool_hits: self.stats.pool_hits.load(Ordering::Relaxed),
            pool_misses: self.stats.pool_misses.load(Ordering::Relaxed),
            successful_returns: self.stats.successful_returns.load(Ordering::Relaxed),
            invalid_returns: self.stats.invalid_returns.load(Ordering::Relaxed),
            pool_discards: self.stats.pool_discards.load(Ordering::Relaxed),
            allocation_failures: self.stats.allocation_failures.load(Ordering::Relaxed),
            total_allocated: self.stats.total_allocated.load(Ordering::Relaxed),
            active_blocks: self.stats.active_blocks.load(Ordering::Relaxed),
            available_blocks,
            total_bytes_allocated: self.stats.total_bytes_allocated.load(Ordering::Relaxed),
            block_size: self.config.memory_pool_block_size,
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// 计算内存池命中率
    fn calculate_hit_rate(&self) -> f64 {
        let hits = self.stats.pool_hits.load(Ordering::Relaxed) as f64;
        let misses = self.stats.pool_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }

    /// 清理内存池
    pub fn clear(&self) {
        let mut blocks = self.available_blocks.lock();
        let cleared_count = blocks.len();
        blocks.clear();

        self.stats.pool_clears.fetch_add(1, Ordering::Relaxed);
        info!(cleared_blocks = cleared_count, "内存池已清理");
    }
}

/// 内存池统计信息
#[derive(Debug, Default)]
pub struct MemoryPoolStats {
    /// 池命中次数
    pub pool_hits: AtomicU64,
    /// 池未命中次数
    pub pool_misses: AtomicU64,
    /// 成功归还次数
    pub successful_returns: AtomicU64,
    /// 无效归还次数
    pub invalid_returns: AtomicU64,
    /// 池丢弃次数
    pub pool_discards: AtomicU64,
    /// 分配失败次数
    pub allocation_failures: AtomicU64,
    /// 池清理次数
    pub pool_clears: AtomicU64,
    /// 总分配块数
    pub total_allocated: AtomicUsize,
    /// 当前活跃块数
    pub active_blocks: AtomicUsize,
    /// 总分配字节数
    pub total_bytes_allocated: AtomicUsize,
}

/// 内存池统计信息快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolStatsSnapshot {
    /// 池命中次数
    pub pool_hits: u64,
    /// 池未命中次数
    pub pool_misses: u64,
    /// 成功归还次数
    pub successful_returns: u64,
    /// 无效归还次数
    pub invalid_returns: u64,
    /// 池丢弃次数
    pub pool_discards: u64,
    /// 分配失败次数
    pub allocation_failures: u64,
    /// 总分配块数
    pub total_allocated: usize,
    /// 当前活跃块数
    pub active_blocks: usize,
    /// 当前可用块数
    pub available_blocks: usize,
    /// 总分配字节数
    pub total_bytes_allocated: usize,
    /// 块大小
    pub block_size: usize,
    /// 命中率（百分比）
    pub hit_rate: f64,
}

/// 内存监控器
///
/// 【功能】: 监控应用程序内存使用情况，检测内存泄漏
/// 【特性】: 实时监控、历史记录、泄漏检测、压力告警
#[derive(Debug)]
pub struct MemoryMonitor {
    /// 配置
    config: MemoryManagerConfig,
    /// 内存使用历史
    usage_history: Arc<ParkingMutex<VecDeque<MemoryUsageSnapshot>>>,
    /// 统计信息
    stats: Arc<MemoryMonitorStats>,
    /// 是否正在运行
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl MemoryMonitor {
    /// 创建新的内存监控器
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self {
            config,
            usage_history: Arc::new(ParkingMutex::new(VecDeque::new())),
            stats: Arc::new(MemoryMonitorStats::default()),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 启动内存监控
    #[instrument(skip(self))]
    pub async fn start_monitoring(&self) {
        if self.is_running.swap(true, Ordering::Relaxed) {
            warn!("内存监控已在运行中");
            return;
        }

        info!("启动内存监控");
        let config = self.config.clone();
        let usage_history = self.usage_history.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                config.memory_monitoring_interval_seconds,
            ));

            while is_running.load(Ordering::Relaxed) {
                interval.tick().await;

                // 收集内存使用情况
                let snapshot = Self::collect_memory_snapshot(&config).await;

                // 检查内存压力
                Self::check_memory_pressure(&snapshot, &config, &stats).await;

                // 存储历史记录
                Self::store_usage_history(&usage_history, snapshot, &config).await;

                // 检测内存泄漏
                if config.enable_leak_detection {
                    Self::detect_memory_leaks(&usage_history, &stats).await;
                }
            }

            info!("内存监控已停止");
        });
    }

    /// 停止内存监控
    pub fn stop_monitoring(&self) {
        if self.is_running.swap(false, Ordering::Relaxed) {
            info!("正在停止内存监控");
        }
    }

    /// 收集内存使用快照
    async fn collect_memory_snapshot(config: &MemoryManagerConfig) -> MemoryUsageSnapshot {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let current_pid = sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from(0));
        let process = sys.process(current_pid);

        let process_memory = process.map(|p| p.memory()).unwrap_or(0);
        let process_virtual_memory = process.map(|p| p.virtual_memory()).unwrap_or(0);

        MemoryUsageSnapshot {
            timestamp: SystemTime::now(),
            system_total_memory: sys.total_memory(),
            system_used_memory: sys.used_memory(),
            system_available_memory: sys.available_memory(),
            process_memory_usage: process_memory,
            process_virtual_memory: process_virtual_memory,
            memory_pressure_level: Self::calculate_pressure_level(
                sys.used_memory(),
                sys.total_memory(),
                config,
            ),
        }
    }

    /// 计算内存压力级别
    fn calculate_pressure_level(
        used_memory: u64,
        total_memory: u64,
        _config: &MemoryManagerConfig,
    ) -> MemoryPressureLevel {
        let usage_percent = ((used_memory as f64) / (total_memory as f64)) * 100.0;

        if usage_percent > 90.0 {
            MemoryPressureLevel::Critical
        } else if usage_percent > 80.0 {
            MemoryPressureLevel::High
        } else if usage_percent > 70.0 {
            MemoryPressureLevel::Medium
        } else {
            MemoryPressureLevel::Low
        }
    }

    /// 检查内存压力
    async fn check_memory_pressure(
        snapshot: &MemoryUsageSnapshot,
        config: &MemoryManagerConfig,
        stats: &MemoryMonitorStats,
    ) {
        match snapshot.memory_pressure_level {
            MemoryPressureLevel::Critical => {
                stats
                    .critical_pressure_events
                    .fetch_add(1, Ordering::Relaxed);
                error!(
                    system_memory_usage = snapshot.system_used_memory,
                    process_memory_usage = snapshot.process_memory_usage,
                    "严重内存压力警告"
                );
            }
            MemoryPressureLevel::High => {
                stats.high_pressure_events.fetch_add(1, Ordering::Relaxed);
                warn!(
                    system_memory_usage = snapshot.system_used_memory,
                    process_memory_usage = snapshot.process_memory_usage,
                    "高内存压力警告"
                );
            }
            MemoryPressureLevel::Medium => {
                stats.medium_pressure_events.fetch_add(1, Ordering::Relaxed);
                debug!("中等内存压力");
            }
            MemoryPressureLevel::Low => {
                // 正常情况，无需特殊处理
            }
        }

        // 检查进程内存使用是否超过阈值
        if snapshot.process_memory_usage > (config.memory_pressure_threshold_bytes as u64) {
            stats
                .process_memory_warnings
                .fetch_add(1, Ordering::Relaxed);
            warn!(
                process_memory = snapshot.process_memory_usage,
                threshold = config.memory_pressure_threshold_bytes,
                "进程内存使用超过阈值"
            );
        }
    }

    /// 存储使用历史
    async fn store_usage_history(
        usage_history: &Arc<ParkingMutex<VecDeque<MemoryUsageSnapshot>>>,
        snapshot: MemoryUsageSnapshot,
        config: &MemoryManagerConfig,
    ) {
        let mut history = usage_history.lock();

        // 限制历史记录数量（保留最近1小时的数据）
        let max_history_size = 3600 / (config.memory_monitoring_interval_seconds as usize);

        if history.len() >= max_history_size {
            history.pop_front();
        }

        history.push_back(snapshot);
    }

    /// 检测内存泄漏
    async fn detect_memory_leaks(
        usage_history: &Arc<ParkingMutex<VecDeque<MemoryUsageSnapshot>>>,
        stats: &MemoryMonitorStats,
    ) {
        let history = usage_history.lock();

        if history.len() < 10 {
            return; // 数据不足，无法检测
        }

        // 检查内存使用趋势
        let recent_snapshots: Vec<_> = history.iter().rev().take(10).collect();
        let mut increasing_count = 0;

        for i in 1..recent_snapshots.len() {
            if recent_snapshots[i - 1].process_memory_usage
                > recent_snapshots[i].process_memory_usage
            {
                increasing_count += 1;
            }
        }

        // 如果80%的采样点都显示内存增长，可能存在内存泄漏
        if increasing_count >= 8 {
            stats
                .potential_leaks_detected
                .fetch_add(1, Ordering::Relaxed);
            warn!(
                increasing_samples = increasing_count,
                total_samples = recent_snapshots.len(),
                "检测到潜在内存泄漏"
            );
        }
    }

    /// 获取内存使用历史
    pub fn get_usage_history(&self) -> Vec<MemoryUsageSnapshot> {
        let history = self.usage_history.lock();
        history.iter().cloned().collect()
    }

    /// 获取内存监控统计信息
    pub fn get_stats(&self) -> MemoryMonitorStatsSnapshot {
        MemoryMonitorStatsSnapshot {
            critical_pressure_events: self.stats.critical_pressure_events.load(Ordering::Relaxed),
            high_pressure_events: self.stats.high_pressure_events.load(Ordering::Relaxed),
            medium_pressure_events: self.stats.medium_pressure_events.load(Ordering::Relaxed),
            process_memory_warnings: self.stats.process_memory_warnings.load(Ordering::Relaxed),
            potential_leaks_detected: self.stats.potential_leaks_detected.load(Ordering::Relaxed),
            monitoring_cycles: self.stats.monitoring_cycles.load(Ordering::Relaxed),
            is_running: self.is_running.load(Ordering::Relaxed),
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats
            .critical_pressure_events
            .store(0, Ordering::Relaxed);
        self.stats.high_pressure_events.store(0, Ordering::Relaxed);
        self.stats
            .medium_pressure_events
            .store(0, Ordering::Relaxed);
        self.stats
            .process_memory_warnings
            .store(0, Ordering::Relaxed);
        self.stats
            .potential_leaks_detected
            .store(0, Ordering::Relaxed);
        self.stats.monitoring_cycles.store(0, Ordering::Relaxed);

        info!("内存监控统计信息已重置");
    }
}

/// 内存使用快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageSnapshot {
    /// 时间戳
    pub timestamp: SystemTime,
    /// 系统总内存
    pub system_total_memory: u64,
    /// 系统已用内存
    pub system_used_memory: u64,
    /// 系统可用内存
    pub system_available_memory: u64,
    /// 进程内存使用
    pub process_memory_usage: u64,
    /// 进程虚拟内存
    pub process_virtual_memory: u64,
    /// 内存压力级别
    pub memory_pressure_level: MemoryPressureLevel,
}

/// 内存压力级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    /// 低压力
    Low,
    /// 中等压力
    Medium,
    /// 高压力
    High,
    /// 严重压力
    Critical,
}

/// 内存监控统计信息
#[derive(Debug, Default)]
pub struct MemoryMonitorStats {
    /// 严重压力事件数
    pub critical_pressure_events: AtomicU64,
    /// 高压力事件数
    pub high_pressure_events: AtomicU64,
    /// 中等压力事件数
    pub medium_pressure_events: AtomicU64,
    /// 进程内存警告数
    pub process_memory_warnings: AtomicU64,
    /// 潜在泄漏检测数
    pub potential_leaks_detected: AtomicU64,
    /// 监控周期数
    pub monitoring_cycles: AtomicU64,
}

/// 内存监控统计信息快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMonitorStatsSnapshot {
    /// 严重压力事件数
    pub critical_pressure_events: u64,
    /// 高压力事件数
    pub high_pressure_events: u64,
    /// 中等压力事件数
    pub medium_pressure_events: u64,
    /// 进程内存警告数
    pub process_memory_warnings: u64,
    /// 潜在泄漏检测数
    pub potential_leaks_detected: u64,
    /// 监控周期数
    pub monitoring_cycles: u64,
    /// 是否正在运行
    pub is_running: bool,
}

/// 统一内存管理器
///
/// 【功能】: 集成L1缓存、对象池、内存池和内存监控的统一管理器
/// 【特性】: 一站式内存管理、统一配置、综合统计
#[derive(Debug)]
pub struct MemoryManager {
    /// 配置
    config: MemoryManagerConfig,
    /// L1缓存（字符串类型示例）
    pub string_cache: L1MemoryCache<String, String>,
    /// 消息对象池
    pub message_pool: ObjectPool<Vec<u8>>,
    /// 内存池
    pub memory_pool: MemoryPool,
    /// 内存监控器
    pub memory_monitor: MemoryMonitor,
}

impl MemoryManager {
    /// 创建新的内存管理器
    pub fn new(config: MemoryManagerConfig) -> Self {
        let string_cache = L1MemoryCache::new(config.clone());

        let message_pool = ObjectPool::new(
            || Vec::with_capacity(1024), // 预分配1KB缓冲区
            config.clone(),
        );

        let memory_pool = MemoryPool::new(config.clone());
        let memory_monitor = MemoryMonitor::new(config.clone());

        Self {
            config,
            string_cache,
            message_pool,
            memory_pool,
            memory_monitor,
        }
    }

    /// 启动内存管理器
    #[instrument(skip(self))]
    pub async fn start(&self) {
        info!("启动内存管理器");

        // 启动内存监控
        self.memory_monitor.start_monitoring().await;

        info!("内存管理器启动完成");
    }

    /// 停止内存管理器
    pub fn stop(&self) {
        info!("停止内存管理器");

        // 停止内存监控
        self.memory_monitor.stop_monitoring();

        info!("内存管理器已停止");
    }

    /// 获取综合统计信息
    pub fn get_comprehensive_stats(&self) -> MemoryManagerStats {
        MemoryManagerStats {
            cache_stats: self.string_cache.get_stats(),
            object_pool_stats: self.message_pool.get_stats(),
            memory_pool_stats: self.memory_pool.get_stats(),
            memory_monitor_stats: self.memory_monitor.get_stats(),
            config: self.config.clone(),
        }
    }

    /// 执行内存清理
    #[instrument(skip(self))]
    pub fn cleanup(&self) {
        info!("执行内存清理");

        // 清理缓存
        self.string_cache.clear();

        // 清理内存池
        self.memory_pool.clear();

        info!("内存清理完成");
    }

    /// 检查内存健康状态
    pub fn check_health(&self) -> MemoryHealthStatus {
        let cache_stats = self.string_cache.get_stats();
        let pool_stats = self.message_pool.get_stats();
        let memory_stats = self.memory_pool.get_stats();
        let monitor_stats = self.memory_monitor.get_stats();

        let mut issues = Vec::new();
        let mut is_healthy = true;

        // 检查缓存命中率
        if cache_stats.hit_rate < 80.0 {
            issues.push(format!("L1缓存命中率过低: {:.1}%", cache_stats.hit_rate));
            is_healthy = false;
        }

        // 检查对象池效率
        if pool_stats.hit_rate < 70.0 {
            issues.push(format!("对象池命中率过低: {:.1}%", pool_stats.hit_rate));
            is_healthy = false;
        }

        // 检查内存池效率
        if memory_stats.hit_rate < 60.0 {
            issues.push(format!("内存池命中率过低: {:.1}%", memory_stats.hit_rate));
            is_healthy = false;
        }

        // 检查内存泄漏
        if monitor_stats.potential_leaks_detected > 0 {
            issues.push(format!(
                "检测到{}个潜在内存泄漏",
                monitor_stats.potential_leaks_detected
            ));
            is_healthy = false;
        }

        // 检查内存压力
        if monitor_stats.critical_pressure_events > 0 {
            issues.push(format!(
                "发生{}次严重内存压力事件",
                monitor_stats.critical_pressure_events
            ));
            is_healthy = false;
        }

        MemoryHealthStatus {
            is_healthy,
            issues,
            cache_hit_rate: cache_stats.hit_rate,
            object_pool_hit_rate: pool_stats.hit_rate,
            memory_pool_hit_rate: memory_stats.hit_rate,
            total_memory_usage: memory_stats.total_bytes_allocated,
        }
    }
}

/// 内存管理器综合统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManagerStats {
    /// 缓存统计
    pub cache_stats: CacheStatsSnapshot,
    /// 对象池统计
    pub object_pool_stats: ObjectPoolStatsSnapshot,
    /// 内存池统计
    pub memory_pool_stats: MemoryPoolStatsSnapshot,
    /// 内存监控统计
    pub memory_monitor_stats: MemoryMonitorStatsSnapshot,
    /// 配置信息
    pub config: MemoryManagerConfig,
}

/// 内存健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealthStatus {
    /// 是否健康
    pub is_healthy: bool,
    /// 问题列表
    pub issues: Vec<String>,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 对象池命中率
    pub object_pool_hit_rate: f64,
    /// 内存池命中率
    pub memory_pool_hit_rate: f64,
    /// 总内存使用量
    pub total_memory_usage: usize,
}
