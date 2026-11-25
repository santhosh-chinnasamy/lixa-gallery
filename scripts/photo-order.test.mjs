import assert from 'node:assert/strict';
import test from 'node:test';
import { patchByPath, rebuildPathIndex } from '../src/lib/photoListPatch.ts';
import { projectPhotoOrder, remapOrder } from '../src/lib/photoOrder.ts';

function photo(path, name, extra = {}) {
  return {
    path,
    thumbnail_path: '',
    metadata: {
      name,
      modified: 1,
      created: 1,
      size: extra.size ?? 10,
    },
    exif: extra.exif,
    ...extra.fields,
  };
}

test('path index patches one slot and keeps the list and the other rows', () => {
  const list = [
    photo('/a.jpg', 'a'),
    photo('/b.jpg', 'b'),
    photo('/c.jpg', 'c'),
  ];
  const index = new Map();
  rebuildPathIndex(list, index);
  const before = list;
  const untouched = list[0];
  const changed = patchByPath(
    list,
    index,
    [photo('/b.jpg', 'b', { fields: { thumbnail_path: '/cache/b.webp' } })],
    (existing, update) => ({ ...existing, ...update }),
  );
  assert.equal(changed, true);
  assert.equal(list, before);
  assert.equal(list[0], untouched);
  assert.equal(list[1].thumbnail_path, '/cache/b.webp');
  assert.equal(list[2].path, '/c.jpg');
  assert.equal(index.get('/b.jpg'), 1);
});

test('unknown paths and an empty patch do not rewrite the list', () => {
  const list = [photo('/a.jpg', 'a')];
  const index = new Map();
  rebuildPathIndex(list, index);
  assert.equal(
    patchByPath(
      list,
      index,
      [photo('/missing.jpg', 'm')],
      (existing) => existing,
    ),
    false,
  );
  assert.equal(list[0].path, '/a.jpg');
});

test('thumbnail updates keep sort order and skip a new sort', () => {
  const incoming = [
    photo('/b.jpg', 'b'),
    photo('/a.jpg', 'a'),
    photo('/c.jpg', 'c'),
  ];
  const sorted = projectPhotoOrder(null, incoming, 'name', 'asc', null);
  assert.deepEqual(
    sorted.map((item) => item.path),
    ['/a.jpg', '/b.jpg', '/c.jpg'],
  );

  const patched = incoming.map((item) =>
    item.path === '/b.jpg'
      ? { ...item, thumbnail_path: '/cache/b.webp' }
      : item,
  );
  // Deliver the batch in a different physical order than the grid.
  const reshuffled = [patched[2], patched[0], patched[1]];
  const next = projectPhotoOrder(sorted, reshuffled, 'name', 'asc', {
    sortBy: 'name',
    sortOrder: 'asc',
  });
  assert.notEqual(next, sorted);
  assert.deepEqual(
    next.map((item) => item.path),
    ['/a.jpg', '/b.jpg', '/c.jpg'],
  );
  assert.equal(next[0], sorted[0]);
  assert.equal(next[1].thumbnail_path, '/cache/b.webp');
  assert.equal(next[2], sorted[2]);

  const again = projectPhotoOrder(next, reshuffled, 'name', 'asc', {
    sortBy: 'name',
    sortOrder: 'asc',
  });
  assert.equal(again, next);
});

test('a sort-key change and a sort-mode change both reorder', () => {
  const rows = [
    photo('/a.jpg', 'a', { size: 3 }),
    photo('/b.jpg', 'b', { size: 1 }),
    photo('/c.jpg', 'c', { size: 2 }),
  ];
  const byName = projectPhotoOrder(null, rows, 'name', 'asc', null);
  const renamed = rows.map((item) =>
    item.path === '/a.jpg'
      ? { ...item, metadata: { ...item.metadata, name: 'z' } }
      : item,
  );
  const resorted = projectPhotoOrder(byName, renamed, 'name', 'asc', {
    sortBy: 'name',
    sortOrder: 'asc',
  });
  assert.deepEqual(
    resorted.map((item) => item.metadata.name),
    ['b', 'c', 'z'],
  );

  const bySize = projectPhotoOrder(resorted, renamed, 'size', 'asc', {
    sortBy: 'name',
    sortOrder: 'asc',
  });
  assert.deepEqual(
    bySize.map((item) => item.path),
    ['/b.jpg', '/c.jpg', '/a.jpg'],
  );
});

test('filling in a capture time reorders a date sort and nothing else', () => {
  const rows = [photo('/a.jpg', 'a'), photo('/b.jpg', 'b')];
  rows[0].metadata.created = 20;
  rows[1].metadata.created = 10;
  const byDate = projectPhotoOrder(null, rows, 'date', 'asc', null);
  assert.deepEqual(
    byDate.map((item) => item.path),
    ['/b.jpg', '/a.jpg'],
  );

  const withExif = rows.map((item) =>
    item.path === '/b.jpg'
      ? { ...item, exif: { captured_at: 50, captured_subsec: 0 } }
      : item,
  );
  const resorted = projectPhotoOrder(byDate, withExif, 'date', 'asc', {
    sortBy: 'date',
    sortOrder: 'asc',
  });
  assert.deepEqual(
    resorted.map((item) => item.path),
    ['/a.jpg', '/b.jpg'],
  );

  const byName = projectPhotoOrder(byDate, withExif, 'name', 'asc', {
    sortBy: 'date',
    sortOrder: 'asc',
  });
  assert.deepEqual(
    byName.map((item) => item.path),
    ['/a.jpg', '/b.jpg'],
  );
});

test('remapping after a thumbnail patch stays in display order', () => {
  // The backend delivers rows in folder order; the grid displays them sorted.
  const incoming = [
    photo('/b.jpg', 'b'),
    photo('/a.jpg', 'a'),
    photo('/c.jpg', 'c'),
  ];
  const displayed = projectPhotoOrder(null, incoming, 'name', 'asc', null);
  const incomingIndex = new Map(incoming.map((item, at) => [item.path, at]));

  // Patch the 'a' row in place, exactly like mergePhotoUpdates does.
  const aAt = incomingIndex.get('/a.jpg');
  incoming[aAt] = { ...incoming[aAt], thumbnail_path: '/cache/a.webp' };

  const remapped = remapOrder(displayed, incoming, incomingIndex);
  assert.notEqual(remapped, displayed);
  assert.deepEqual(
    remapped.map((item) => item.path),
    ['/a.jpg', '/b.jpg', '/c.jpg'],
  );
  assert.equal(remapped[0].thumbnail_path, '/cache/a.webp');
  assert.equal(remapped[1], displayed[1], 'untouched rows keep their objects');
});

test('remapping returns the same array when no row changed', () => {
  const incoming = [photo('/a.jpg', 'a'), photo('/b.jpg', 'b')];
  const displayed = projectPhotoOrder(null, incoming, 'name', 'asc', null);
  const incomingIndex = new Map(incoming.map((item, at) => [item.path, at]));
  assert.equal(remapOrder(displayed, incoming, incomingIndex), displayed);
});
