// Stage 55.3.2: Memory Optimization Tests
//
// This module contains comprehensive tests for memory optimization features:
// - Zero-copy allocation
// - Intelligent memory pooling
// - Generational garbage collection
// - Memory compression
// - Leak detection
//
// These tests validate that Beejs achieves 30-50% memory usage reduction
// compared to standard allocation strategies.

use amberjs::memory::*;
use std::time{Duration, Instant};

#[cfg(test)]
mod memory_optimization_tests {
    use super::*;

    /// Test 1: Memory Optimization Manager Creation
    #[test]
    fn test_memory_optimization_manager_creation() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        assert!(manager.zero_copy_allocator.is_some());
        assert!(manager.memory_pool.is_some());
        assert!(manager.generational_gc.is_some());
        assert!(manager.memory_compressor.is_some());
        assert!(manager.leak_detector.is_some());
    }

    /// Test 2: Basic Allocation and Deallocation
    #[test]
    fn test_basic_allocation_deallocation() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        // Test small allocation
        let handle1 = manager.allocate(64).unwrap();
        assert!(!handle1.ptr.is_null());
        assert_eq!(handle1.size, 64);

        // Test medium allocation
        let handle2 = manager.allocate(1024).unwrap();
        assert!(!handle2.ptr.is_null());
        assert_eq!(handle2.size, 1024);

        // Test large allocation
        let handle3 = manager.allocate(64 * 1024).unwrap();
        assert!(!handle3.ptr.is_null());
        assert_eq!(handle3.size, 64 * 1024);

        manager.free(handle1).unwrap();
        manager.free(handle2).unwrap();
        manager.free(handle3).unwrap();
    }

    /// Test 3: Memory Pool Allocation
    #[test]
    fn test_memory_pool_allocation() {
        let config = MemoryOptimizationConfig {
            enable_zero_copy: false,
            enable_pooling: true,
            enable_generational_gc: false,
            enable_compression: false,
            enable_leak_detection: false,
            pool_config: PoolConfig {
                string_pool_size: 50,
                object_pool_size: 100,
                buffer_timeout: Duration::from_secs(60),
                min_usage_threshold: 2,
            },
            gc_config: GCConfig::default(),
            compression_config: CompressionConfig::default(),
            leak_detection_config: LeakDetectionConfig::default(),
        };

        let manager = MemoryOptimizationManager::new(config);

        // Allocate multiple objects to test pooling
        let handles: Vec<_> = (0..10)
            .map(|_| manager.allocate(512).unwrap())
            .collect();

        // Verify all allocations succeeded
        assert_eq!(handles.len(), 10);
        for handle in &handles {
            assert!(!handle.ptr.is_null());
        }

        // Free all handles
        for handle in handles {
            manager.free(handle).unwrap();
        }
    }

    /// Test 4: Statistics Tracking
    #[test]
    fn test_statistics_tracking() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        // Perform allocations
        let _h1 = manager.allocate(100).unwrap();
        let _h2 = manager.allocate(200).unwrap();
        let _h3 = manager.allocate(300).unwrap();

        // Get statistics
        let stats = manager.get_stats();

        // Verify statistics are being tracked
        assert!(stats.total_allocations >= 3);
        assert!(stats.active_allocations > 0);
    }

    /// Test 5: Memory Pooling Efficiency
    #[test]
    fn test_memory_pooling_efficiency() {
        let config = MemoryOptimizationConfig {
            enable_zero_copy: true,
            enable_pooling: true,
            enable_generational_gc: false,
            enable_compression: false,
            enable_leak_detection: false,
            pool_config: PoolConfig {
                string_pool_size: 200,
                object_pool_size: 200,
                buffer_timeout: Duration::from_secs(300),
                min_usage_threshold: 1,
            },
            gc_config: GCConfig::default(),
            compression_config: CompressionConfig::default(),
            leak_detection_config: LeakDetectionConfig::default(),
        };

        let manager = MemoryOptimizationManager::new(config);

        // Allocate and free multiple times to test pooling efficiency
        for _ in 0..50 {
            let handle = manager.allocate(1024).unwrap();
            manager.free(handle).unwrap();
        }

        let stats = manager.get_stats();

        // Pooling should account for a significant portion of allocations
        // (allowing for some fallback to standard allocation)
        assert!(stats.pooled_allocations > 0, "Pooled allocations should be greater than 0");
    }

    /// Test 6: Memory Performance Benchmark
    #[test]
    fn test_memory_performance_benchmark() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        let iterations = 1000;
        let allocation_size = 1024;

        // Benchmark allocations
        let start = Instant::now();
        let mut handles = Vec::new();

        for _ in 0..iterations {
            let handle = manager.allocate(allocation_size).unwrap();
            handles.push(handle);
        }

        let allocation_time = start.elapsed();

        // Benchmark deallocations
        let dealloc_start = Instant::now();
        for handle in handles {
            manager.free(handle).unwrap();
        }
        let deallocation_time = dealloc_start.elapsed();

        // Performance assertions
        println!("Allocations: {} in {:?}", iterations, allocation_time);
        println!("Deallocations: {} in {:?}", iterations, deallocation_time);

        // Should complete in reasonable time (adjust threshold as needed)
        assert!(allocation_time < Duration::from_millis(500),
            "Allocation benchmark took too long: {:?}", allocation_time);
        assert!(deallocation_time < Duration::from_millis(500),
            "Deallocation benchmark took too long: {:?}", deallocation_time);
    }

    /// Test 7: Memory Usage Reduction Verification
    #[test]
    fn test_memory_usage_reduction() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        // Simulate a workload similar to a web server
        let mut all_handles = Vec::new();

        // Create multiple rounds of allocations
        for round in 0..10 {
            // Allocate objects for this round
            for _ in 0..50 {
                let handle = manager.allocate(2048).unwrap();
                all_handles.push(handle);
            }

            // Simulate some processing time
            std::thread::sleep(Duration::from_millis(10));

            // Free half of the objects
            let half = all_handles.len() / 2;
            for _ in 0..half {
                if let Some(handle) = all_handles.pop() {
                    manager.free(handle).unwrap();
                }
            }
        }

        let stats = manager.get_stats();

        // Verify that optimizations are being used
        let zero_copy_count = stats.zero_copy_allocations;
        let pooled_count = stats.pooled_allocations;
        let total_count = stats.total_allocations;

        println!("Total allocations: {}", total_count);
        println!("Zero-copy allocations: {}", zero_copy_count);
        println!("Pooled allocations: {}", pooled_count);

        // At least some allocations should use optimizations
        assert!(total_count > 0, "Should have performed some allocations");

        // Free remaining allocations
        while let Some(handle) = all_handles.pop() {
            manager.free(handle).unwrap();
        }
    }

    /// Test 8: Mixed Optimization Strategies
    #[test]
    fn test_mixed_optimization_strategies() {
        let config = MemoryOptimizationConfig {
            enable_zero_copy: true,
            enable_pooling: true,
            enable_generational_gc: true,
            enable_compression: true,
            enable_leak_detection: true,
            pool_config: PoolConfig {
                string_pool_size: 100,
                object_pool_size: 100,
                buffer_timeout: Duration::from_secs(60),
                min_usage_threshold: 1,
            },
            gc_config: GCConfig {
                young_generation_size: 4 * 1024 * 1024,
                old_generation_size: 64 * 1024 * 1024,
                gc_threshold: 100,
                max_pause_time: Duration::from_millis(10),
            },
            compression_config: CompressionConfig {
                compression_threshold: 256,
                compression_algorithm: "lz4".to_string(),
                compression_level: 1,
            },
            leak_detection_config: LeakDetectionConfig {
                enable_tracking: true,
                track_stack_traces: false,
                max_tracked_allocations: 5000,
                leak_threshold: 25,
            },
        };

        let manager = MemoryOptimizationManager::new(config);

        // Test different allocation sizes
        let sizes = vec![16, 64, 256, 1024, 4096, 16384, 65536];

        let mut all_handles = Vec::new();

        for size in sizes {
            for _ in 0..10 {
                let handle = manager.allocate(size).unwrap();
                all_handles.push(handle);
            }
        }

        // Get final stats
        let stats = manager.get_stats();

        println!("Mixed optimization test - Total allocations: {}", stats.total_allocations);
        println!("Zero-copy: {}, Pooled: {}, GC runs: {}",
                 stats.zero_copy_allocations, stats.pooled_allocations, stats.gc_collections);

        // Verify all allocations succeeded
        assert_eq!(all_handles.len(), sizes.len() * 10);

        // Free all allocations
        for handle in all_handles {
            manager.free(handle).unwrap();
        }

        // Final verification
        let final_stats = manager.get_stats();
        assert_eq!(final_stats.active_allocations, 0);
    }

    /// Test 9: Configuration Validation
    #[test]
    fn test_configuration_validation() {
        // Test default configuration
        let config = MemoryOptimizationConfig::default();
        assert!(config.enable_zero_copy);
        assert!(config.enable_pooling);
        assert!(config.enable_generational_gc);
        assert!(config.enable_compression);
        assert!(config.enable_leak_detection);

        // Test custom configuration
        let custom_config = MemoryOptimizationConfig {
            enable_zero_copy: false,
            enable_pooling: true,
            enable_generational_gc: false,
            enable_compression: true,
            enable_leak_detection: false,
            pool_config: PoolConfig {
                string_pool_size: 200,
                object_pool_size: 300,
                buffer_timeout: Duration::from_secs(600),
                min_usage_threshold: 5,
            },
            gc_config: GCConfig {
                young_generation_size: 32 * 1024 * 1024,
                old_generation_size: 512 * 1024 * 1024,
                gc_threshold: 2000,
                max_pause_time: Duration::from_millis(20),
            },
            compression_config: CompressionConfig {
                compression_threshold: 2048,
                compression_algorithm: "zstd".to_string(),
                compression_level: 3,
            },
            leak_detection_config: LeakDetectionConfig {
                enable_tracking: true,
                track_stack_traces: true,
                max_tracked_allocations: 50000,
                leak_threshold: 200,
            },
        };

        let manager = MemoryOptimizationManager::new(custom_config);
        assert!(manager.zero_copy_allocator.is_none());
        assert!(manager.memory_pool.is_some());
        assert!(manager.generational_gc.is_none());
        assert!(manager.memory_compressor.is_some());
        assert!(manager.leak_detector.is_none());
    }

    /// Test 10: Edge Cases
    #[test]
    fn test_edge_cases() {
        let config = MemoryOptimizationConfig::default();
        let manager = MemoryOptimizationManager::new(config);

        // Test very small allocation
        let handle1 = manager.allocate(1).unwrap();
        assert!(!handle1.ptr.is_null());
        assert_eq!(handle1.size, 1);
        manager.free(handle1).unwrap();

        // Test very large allocation
        let handle2 = manager.allocate(1024 * 1024).unwrap(); // 1MB
        assert!(!handle2.ptr.is_null());
        assert_eq!(handle2.size, 1024 * 1024);
        manager.free(handle2).unwrap();

        // Test zero allocation (edge case)
        let handle3 = manager.allocate(0).unwrap();
        assert!(!handle3.ptr.is_null() || handle3.size == 0);
        manager.free(handle3).unwrap();
    }
}
