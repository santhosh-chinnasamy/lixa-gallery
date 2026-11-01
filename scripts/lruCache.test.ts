import assert from 'node:assert/strict';
import { test } from 'node:test';
import { LruCache } from '../src/lib/lruCache.ts';

test('evicts the least recently used entry past the limit', () => {
  const evicted: string[] = [];
  const cache = new LruCache<string>(2, (value) => evicted.push(value));

  cache.set('a', 'a');
  cache.set('b', 'b');
  assert.equal(cache.has('a'), true);
  cache.set('c', 'c');

  assert.equal(cache.size, 2);
  assert.equal(cache.has('b'), false);
  assert.equal(cache.has('a'), true);
  assert.equal(cache.has('c'), true);
  assert.deepEqual(evicted, ['b']);
});

test('stays capped while stepping through a long sequence', () => {
  const cache = new LruCache<number>(8);
  for (let i = 0; i < 500; i++) {
    cache.set(String(i), i);
    assert.ok(cache.size <= 8);
  }
  assert.equal(cache.size, 8);
  assert.equal(cache.has('499'), true);
  assert.equal(cache.has('492'), true);
  assert.equal(cache.has('491'), false);
});

test('clear drops every entry and reports them', () => {
  const evicted: string[] = [];
  const cache = new LruCache<string>(8, (value) => evicted.push(value));
  cache.set('a', 'a');
  cache.set('b', 'b');
  cache.clear();
  assert.equal(cache.size, 0);
  assert.equal(cache.has('a'), false);
  assert.deepEqual(evicted.sort(), ['a', 'b']);
});

test('replacing a key does not grow the cache', () => {
  const evicted: string[] = [];
  const cache = new LruCache<string>(1, (value) => evicted.push(value));
  cache.set('a', 'first');
  cache.set('a', 'second');
  assert.equal(cache.size, 1);
  assert.deepEqual(evicted, ['first']);
});
