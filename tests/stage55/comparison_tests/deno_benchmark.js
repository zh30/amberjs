#!/usr/bin/env deno run --allow-env

/**
 * Deno 性能对比测试
 * 用于与 Amber 进行性能对比
 */

// Deno 不需要导入 fs，使用内置的 Deno.writeTextFileSync

// 测试结果收集器
class BenchmarkResult {
    constructor(name, iterations = 1000) {
        this.name = name;
        this.iterations = iterations;
        this.times = [];
        this.memoryUsage = [];
    }

    // 运行基准测试
    async run(fn) {
        // 预热
        for (let i = 0; i < 10; i++) {
            await fn();
        }

        // 正式测试
        for (let i = 0; i < this.iterations; i++) {
            const start = performance.now();
            await fn();
            const end = performance.now();

            const duration = end - start;
            this.times.push(duration);

            // 记录内存使用 (Deno 不支持 process.memoryUsage)
            // 跳过内存测试或使用估算值
        }
    }

    // 计算统计结果
    getStats() {
        const sortedTimes = [...this.times].sort((a, b) => a - b);
        const mean = this.times.reduce((a, b) => a + b, 0) / this.times.length;
        const median = sortedTimes[Math.floor(sortedTimes.length / 2)];
        const min = sortedTimes[0];
        const max = sortedTimes[sortedTimes.length - 1];

        // 计算标准差
        const variance = this.times.reduce((acc, val) => acc + Math.pow(val - mean, 2), 0) / this.times.length;
        const stdDev = Math.sqrt(variance);

        // 吞吐量 (ops/sec)
        const throughput = 1000 / mean;

        return {
            name: this.name,
            iterations: this.iterations,
            mean: Number(mean.toFixed(3)),
            median: Number(median.toFixed(3)),
            min: Number(min.toFixed(3)),
            max: Number(max.toFixed(3)),
            stdDev: Number(stdDev.toFixed(3)),
            throughput: Number(throughput.toFixed(2)),
            avgMemoryBytes: 0, // Deno 不支持内存监控
            avgMemoryMB: 0
        };
    }
}

// 测试用例定义（与 Node.js/Bun 保持一致）
const testCases = [
    {
        name: 'Startup Time',
        description: '测量运行时启动时间',
        fn: async () => {
            // 简单的启动测试
            console.log('');
        }
    },
    {
        name: 'Simple Arithmetic',
        description: '简单算术运算',
        fn: async () => {
            let sum = 0;
            for (let i = 0; i < 10000; i++) {
                sum += i * i;
            }
        }
    },
    {
        name: 'Fibonacci (n=30)',
        description: '计算 Fibonacci 数列',
        fn: async () => {
            function fib(n) {
                if (n <= 1) return n;
                return fib(n - 1) + fib(n - 2);
            }
            fib(30);
        }
    },
    {
        name: 'Array Operations',
        description: '数组操作性能',
        fn: async () => {
            const arr = new Array(10000);
            for (let i = 0; i < arr.length; i++) {
                arr[i] = i * i;
            }
            arr.sort((a, b) => a - b);
            arr.filter(x => x % 2 === 0);
            arr.map(x => x * 2);
        }
    },
    {
        name: 'Object Operations',
        description: '对象操作性能',
        fn: async () => {
            const obj = {};
            for (let i = 0; i < 5000; i++) {
                obj[`key_${i}`] = { value: i, nested: { data: i * 2 } };
            }
            Object.keys(obj);
            Object.values(obj);
        }
    },
    {
        name: 'String Operations',
        description: '字符串操作性能',
        fn: async () => {
            let str = '';
            for (let i = 0; i < 1000; i++) {
                str += `test_${i}_`;
            }
            str.split('_');
            str.toUpperCase();
            str.toLowerCase();
        }
    },
    {
        name: 'JSON Processing',
        description: 'JSON 数据处理',
        fn: async () => {
            const data = {
                users: Array.from({ length: 100 }, (_, i) => ({
                    id: i,
                    name: `User ${i}`,
                    email: `user${i}@example.com`,
                    active: i % 2 === 0,
                    tags: ['tag1', 'tag2', 'tag3']
                })),
                meta: {
                    total: 100,
                    page: 1,
                    limit: 10
                }
            };
            JSON.stringify(data);
            JSON.parse(JSON.stringify(data));
        }
    },
    {
        name: 'Async Operations',
        description: '异步操作性能',
        fn: async () => {
            await Promise.resolve(42);
            await Promise.all([
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ]);
        }
    }
];

// 主测试函数
async function runBenchmarks() {
    console.log('🚀 Starting Deno Performance Benchmark');
    console.log('='.repeat(60));
    console.log(`Deno Version: ${Deno.version.deno}`);
    console.log(`V8 Version: ${Deno.version.v8}`);
    console.log(`TypeScript Version: ${Deno.version.typescript}`);
    console.log(`Platform: ${Deno.build.os} ${Deno.build.arch}`);
    console.log('');

    const results = [];

    for (const testCase of testCases) {
        console.log(`📊 Running: ${testCase.name}`);
        console.log(`   Description: ${testCase.description}`);

        const benchmark = new BenchmarkResult(testCase.name);
        await benchmark.run(testCase.fn);
        const stats = benchmark.getStats();

        console.log(`   Mean: ${stats.mean}ms`);
        console.log(`   Throughput: ${stats.throughput} ops/sec`);
        console.log(`   Memory: N/A (Deno doesn't expose memory API)`);
        console.log('');

        results.push(stats);
    }

    // 生成报告
    const report = {
        runtime: 'Deno',
        version: Deno.version.deno,
        v8Version: Deno.version.v8,
        typescriptVersion: Deno.version.typescript,
        timestamp: new Date().toISOString(),
        platform: {
            os: Deno.build.os,
            arch: Deno.build.arch
        },
        results: results
    };

    // 保存结果
    const outputFile = 'deno_benchmark_results.json';
    Deno.writeTextFileSync(outputFile, JSON.stringify(report, null, 2));
    console.log(`✅ Results saved to ${outputFile}`);

    // 计算总体评分
    const avgThroughput = results.reduce((sum, r) => sum + r.throughput, 0) / results.length;

    console.log('');
    console.log('📈 Overall Performance:');
    console.log(`   Average Throughput: ${avgThroughput.toFixed(2)} ops/sec`);
    console.log(`   Memory: N/A`);
    console.log('');

    return report;
}

// 运行测试
if (import.meta.main) {
    runBenchmarks()
        .then(() => {
            console.log('✅ Benchmark completed successfully');
            Deno.exit(0);
        })
        .catch((error) => {
            console.error('❌ Benchmark failed:', error);
            Deno.exit(1);
        });
}
