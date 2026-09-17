// Microbenchmark example for `amber bench`

bench("Array.prototype.push (1000 items)", () => {
    const arr = [];
    for (let i = 0; i < 1000; i++) {
        arr.push(i);
    }
});

bench("JSON.parse & JSON.stringify", () => {
    const obj = { id: 1, name: "amberjs", active: true, scores: [10, 20, 30] };
    const str = JSON.stringify(obj);
    const parsed = JSON.parse(str);
});

bench("Math.sqrt in loop", () => {
    let sum = 0;
    for (let i = 0; i < 1000; i++) {
        sum += Math.sqrt(i);
    }
});
