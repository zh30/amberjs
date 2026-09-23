/**
 * Amber 测试框架 - 快照测试示例
 *
 * 快照测试非常适合测试数据结构、API 响应、配置对象等
 * 避免手动编写大量断言
 */

describe("快照测试示例", () => {
    test("用户对象快照", () => {
        const user = {
            id: 1,
            name: "Alice",
            email: "alice@example.com",
            profile: {
                age: 30,
                city: "Beijing",
                interests: ["coding", "reading", "music"]
            },
            createdAt: new Date("2025-01-01").toISOString()
        };

        expect(user).toMatchSnapshot();
    });

    test("API 响应快照", async () => {
        const apiResponse = {
            status: "success",
            data: {
                users: [
                    {
                        id: 1,
                        name: "John Doe",
                        email: "john@example.com"
                    },
                    {
                        id: 2,
                        name: "Jane Smith",
                        email: "jane@example.com"
                    }
                ],
                pagination: {
                    page: 1,
                    perPage: 10,
                    total: 100
                }
            },
            timestamp: "2025-12-22T08:00:00.000Z"
        };

        expect(apiResponse).toMatchSnapshot();
    });

    test("复杂配置对象快照", () => {
        const config = {
            app: {
                name: "Amber App",
                version: "1.0.0",
                debug: false
            },
            database: {
                host: "localhost",
                port: 5432,
                name: "amberjs_db",
                options: {
                    ssl: false,
                    maxConnections: 100,
                    timeout: 30000
                }
            },
            features: {
                enableAI: true,
                enableAnalytics: true,
                enableCaching: true
            }
        };

        expect(config).toMatchSnapshot();
    });

    test("数组快照", () => {
        const products = [
            {
                id: 1,
                name: "Laptop",
                price: 1200,
                category: "Electronics",
                tags: ["computer", "portable"]
            },
            {
                id: 2,
                name: "Mouse",
                price: 25,
                category: "Electronics",
                tags: ["wireless", "accessory"]
            },
            {
                id: 3,
                name: "Desk",
                price: 300,
                category: "Furniture",
                tags: ["office", "wood"]
            }
        ];

        expect(products).toMatchSnapshot();
    });

    test("嵌套对象快照", () => {
        const nestedData = {
            level1: {
                level2: {
                    level3: {
                        level4: {
                            value: "deeply nested"
                        }
                    }
                }
            }
        };

        expect(nestedData).toMatchSnapshot();
    });
});

describe("快照测试 - 更新快照", () => {
    test("动态数据快照", () => {
        const dynamicData = {
            timestamp: Date.now(),
            randomId: Math.random().toString(36).substring(7),
            sessionData: {
                startTime: new Date().toISOString(),
                userAgent: "Amber Test Runner"
            }
        };

        // 对于包含动态数据的测试，可以只快照稳定部分
        const stableData = {
            hasTimestamp: typeof dynamicData.timestamp === 'number',
            hasRandomId: typeof dynamicData.randomId === 'string',
            hasSessionData: !!dynamicData.sessionData
        };

        expect(stableData).toMatchSnapshot();
    });
});

describe("快照测试 - 大对象", () => {
    test("大数组快照", () => {
        const largeArray = Array.from({ length: 1000 }, (_, i) => ({
            id: i,
            name: `Item ${i}`,
            value: i * Math.random(),
            metadata: {
                created: i < 500 ? "old" : "new",
                category: i % 3 === 0 ? "A" : i % 3 === 1 ? "B" : "C"
            }
        }));

        // 可以只快照数组的结构，而不包含所有数据
        expect(largeArray).toHaveLength(1000);
        expect(largeArray[0]).toHaveProperty('id');
        expect(largeArray[0]).toHaveProperty('name');
        expect(largeArray[0]).toHaveProperty('metadata');
    });
});

/**
 * 快照测试使用说明
 *
 * 1. 首次运行测试时，Amber 会创建快照文件
 * 2. 快照文件保存在 __snapshots__ 目录中
 * 3. 后续运行会与快照进行比较
 * 4. 如果需要更新快照，使用 --updateSnapshot 参数
 *
 * 命令:
 * amber test examples/testing/snapshot_test.test.js
 * amber test examples/testing/snapshot_test.test.js --update-snapshot
 *
 * 快照文件:
 * __snapshots__/snapshot_test.test.js.snap
 */
