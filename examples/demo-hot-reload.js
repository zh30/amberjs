/**
 * Amber Hot Reload Demo
 *
 * This file demonstrates the hot reload functionality.
 * Run with: amber run demo.js --watch
 *
 * The browser will automatically reload when this file changes.
 */

console.log('[Demo] Amber Hot Reload Demo Started');
console.log('[Demo] Current time:', new Date().toISOString());

// Simple counter to verify reload
if (typeof window !== 'undefined' && !window.reloadCount) {
  window.reloadCount = 0;
}
if (typeof window !== 'undefined') {
  window.reloadCount++;
  console.log('[Demo] Page reload count:', window.reloadCount);
}

// Demonstrate reactivity
const message = 'Hello from Amber with hot reload!';
console.log('[Demo]', message);

// Calculate Fibonacci for demonstration
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log('[Demo] Fibonacci(10) =', fibonacci(10));
console.log('[Demo] Demo complete! Try editing this file to see hot reload in action.');
