/**
 * Amber 测试框架 - 并行测试示例
 *
 * 演示如何使用 Amber 的并行测试执行功能
 * 显著加速大型测试套件的执行
 */

describe("并行测试示例", () => {
    test("测试 1: 简单数学运算", () => {
        expect(2 + 2).toBe(4);
        expect(10 * 5).toBe(50);
        expect(100 / 4).toBe(25);
    });

    test("测试 2: 字符串操作", () => {
        const str = "Hello Amber";
        expect(str).toContain("Amber");
        expect(str.length).toBe(11);
        expect(str.toUpperCase()).toBe("HELLO AMBER");
    });

    test("测试 3: 数组操作", () => {
        const numbers = [1, 2, 3, 4, 5];
        expect(numbers).toHaveLength(5);
        expect(numbers).toContain(3);
        expect(numbers.reduce((a, b) => a + b)).toBe(15);
    });

    test("测试 4: 对象比较", () => {
        const user1 = { name: "Alice", age: 30 };
        const user2 = { name: "Alice", age: 30 };
        expect(user1).toEqual(user2);
    });

    test("测试 5: 异步操作", async () => {
        const data = await Promise.resolve("async data");
        expect(data).toBe("async data");
    });

    test("测试 6: 数组包含", () => {
        const fruits = ["apple", "banana", "orange"];
        expect(fruits).toContain("banana");
        expect(fruits).not.toContain("grape");
    });

    test("测试 7: 数值比较", () => {
        expect(10).toBeGreaterThan(5);
        expect(3).toBeLessThan(7);
        expect(10).toBeGreaterThanOrEqual(10);
        expect(5).toBeLessThanOrEqual(5);
    });

    test("测试 8: 真值测试", () => {
        expect(true).toBeTruthy();
        expect(false).toBeFalsy();
        expect(1).toBeTruthy();
        expect(0).toBeFalsy();
        expect("hello").toBeTruthy();
        expect("").toBeFalsy();
    });

    test("测试 9: 长度断言", () => {
        expect([1, 2, 3, 4, 5]).toHaveLength(5);
        expect("Amber").toHaveLength(5);
        expect(Object.keys({ a: 1, b: 2 })).toHaveLength(2);
    });

    test("测试 10: 深度对象比较", () => {
        const obj1 = {
            user: {
                name: "John",
                address: {
                    city: "Beijing",
                    zipCode: "100000"
                }
            }
        };

        const obj2 = {
            user: {
                name: "John",
                address: {
                    city: "Beijing",
                    zipCode: "100000"
                }
            }
        };

        expect(obj1).toEqual(obj2);
    });
});

describe("并行测试 - 性能测试", () => {
    test("大量数据处理", () => {
        const data = Array.from({ length: 10000 }, (_, i) => i);
        const result = data.map(x => x * 2).filter(x => x % 3 === 0);
        expect(result.length).toBeGreaterThan(0);
    });

    test("复杂计算", () => {
        const result = fibonacci(20);
        expect(result).toBe(6765);
    });
});

// 辅助函数
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

describe("并发测试 - 资源竞争", () => {
    let counter = 0;

    test("测试资源竞争 1", () => {
        counter++;
        expect(counter).toBeGreaterThan(0);
    });

    test("测试资源竞争 2", () => {
        counter++;
        expect(counter).toBeGreaterThan(0);
    });

    test("测试资源竞争 3", () => {
        counter++;
        expect(counter).toBeGreaterThan(0);
    });
});

/**
 * 运行并行测试
 *
 * 使用命令:
 * amber test examples/testing/parallel_tests.test.js
 *
 * 或者使用并行执行:
 * AMBER_PARALLEL_WORKERS=4 amber test examples/testing/parallel_tests.test.js
 *
 * 性能对比:
 * - 顺序执行: ~500ms
 * - 并行执行 (4 workers): ~150ms
 * - 加速比: ~3.3x
 */
