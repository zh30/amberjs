/**
 * Amber 测试框架 - 性能测试示例
 *
 * 使用 Amber 内置的性能测试框架进行基准测试
 * 自动检测性能回归
 */

describe("性能测试 - 数学运算", () => {
    benchmark("简单加法", () => {
        let result = 0;
        for (let i = 0; i < 1000; i++) {
            result += i;
        }
        return result;
    });

    benchmark("复杂计算 - 质数检测", () => {
        function isPrime(num) {
            if (num <= 1) return false;
            if (num <= 3) return true;
            if (num % 2 === 0 || num % 3 === 0) return false;
            for (let i = 5; i * i <= num; i += 6) {
                if (num % i === 0 || num % (i + 2) === 0) return false;
            }
            return true;
        }
        return isPrime(997);
    });

    benchmark("字符串操作", () => {
        let str = "Amber";
        for (let i = 0; i < 100; i++) {
            str = str.toUpperCase().toLowerCase().replace(/j/g, 'J');
        }
        return str;
    });

    benchmark("数组操作", () => {
        const arr = Array.from({ length: 1000 }, (_, i) => i);
        return arr
            .filter(x => x % 2 === 0)
            .map(x => x * 2)
            .reduce((a, b) => a + b, 0);
    });
});

describe("性能测试 - 数据结构", () => {
    benchmark("对象创建", () => {
        const obj = {
            id: Math.random(),
            name: "test",
            value: Date.now(),
            nested: {
                a: 1,
                b: 2,
                c: 3
            }
        };
        return obj;
    });

    benchmark("Map 操作", () => {
        const map = new Map();
        for (let i = 0; i < 100; i++) {
            map.set(`key${i}`, `value${i}`);
        }
        let sum = 0;
        for (let [key, value] of map) {
            sum += key.length + value.length;
        }
        return sum;
    });

    benchmark("Set 操作", () => {
        const set = new Set();
        for (let i = 0; i < 1000; i++) {
            set.add(Math.random());
        }
        return set.size;
    });
});

describe("性能测试 - 算法", () => {
    benchmark("排序算法 - 快速排序", () => {
        function quickSort(arr) {
            if (arr.length <= 1) return arr;
            const pivot = arr[Math.floor(arr.length / 2)];
            const left = arr.filter(x => x < pivot);
            const right = arr.filter(x => x > pivot);
            const middle = arr.filter(x => x === pivot);
            return [
                ...quickSort(left),
                ...middle,
                ...quickSort(right)
            ];
        }
        const arr = Array.from({ length: 100 }, () => Math.random());
        return quickSort(arr);
    });

    benchmark("搜索算法 - 二分搜索", () => {
        function binarySearch(arr, target) {
            let left = 0;
            let right = arr.length - 1;
            while (left <= right) {
                const mid = Math.floor((left + right) / 2);
                if (arr[mid] === target) return mid;
                if (arr[mid] < target) left = mid + 1;
                else right = mid - 1;
            }
            return -1;
        }
        const arr = Array.from({ length: 100 }, (_, i) => i);
        return binarySearch(arr, 50);
    });

    benchmark("斐波那契数列 - 递归", () => {
        function fibonacci(n) {
            if (n <= 1) return n;
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
        return fibonacci(10);
    });

    benchmark("斐波那契数列 - 迭代", () => {
        function fibonacciIterative(n) {
            if (n <= 1) return n;
            let a = 0, b = 1;
            for (let i = 2; i <= n; i++) {
                const c = a + b;
                a = b;
                b = c;
            }
            return b;
        }
        return fibonacciIterative(10);
    });
});

describe("性能测试 - 内存分配", () => {
    benchmark("大量对象创建", () => {
        const objects = [];
        for (let i = 0; i < 10000; i++) {
            objects.push({
                id: i,
                data: new Array(100).fill(i)
            });
        }
        return objects.length;
    });

    benchmark("字符串拼接", () => {
        let result = "";
        for (let i = 0; i < 1000; i++) {
            result += `Item ${i}, `;
        }
        return result.length;
    });

    benchmark("模板字符串", () => {
        let result = "";
        for (let i = 0; i < 1000; i++) {
            result = `${result}Item ${i}, `;
        }
        return result.length;
    });
});

describe("性能测试 - 并发操作", () => {
    benchmark("Promise 并发执行", async () => {
        const promises = Array.from({ length: 100 }, (_, i) =>
            Promise.resolve(i * 2)
        );
        const results = await Promise.all(promises);
        return results.length;
    });

    benchmark("异步等待", async () => {
        let counter = 0;
        for (let i = 0; i < 100; i++) {
            await Promise.resolve();
            counter++;
        }
        return counter;
    });
});

/**
 * 性能测试运行
 *
 * 命令:
 * amber test examples/testing/perf_test.test.js
 *
 * 输出示例:
 * Performance Test Results:
 * 简单加法: 1000000 ops/sec (基准: 800000 ops/sec)
 * 复杂计算: 50000 ops/sec (基准: 45000 ops/sec)
 * 字符串操作: 100000 ops/sec (基准: 120000 ops/sec) ⚠️
 *
 * 回归检测:
 * - 性能下降 > 20%: ❌ 失败
 * - 性能下降 10-20%: ⚠️ 警告
 * - 性能变化 < 10%: ✅ 通过
 *
 * 基准存储:
 * 性能基准保存在 .amberjs/benchmarks/ 目录中
 */
