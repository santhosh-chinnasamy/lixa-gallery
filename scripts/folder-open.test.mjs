import assert from 'node:assert/strict';
import test from 'node:test';
import {
  errorText,
  favouriteToggleError,
  folderOpenError,
} from '../src/lib/errorText.ts';
import { immediateSubfolders } from '../src/lib/folderEmptyState.ts';

test('creation-time style failures stay human and include the folder', () => {
  const message = folderOpenError(
    '/Volumes/CARD',
    'IO error: Permission denied (os error 13)',
  );
  assert.equal(
    message,
    "Couldn't open /Volumes/CARD. IO error: Permission denied (os error 13)",
  );
});

test('favourite toggle errors explain the failed write', () => {
  assert.equal(
    favouriteToggleError(new Error('database is locked')),
    "Couldn't update favourite. database is locked",
  );
  assert.equal(errorText({ message: '  disk full  ' }), 'disk full');
  assert.equal(errorText(null), 'Something went wrong.');
});

test('a folder with only subfolders lists those children', () => {
  const tree = {
    name: 'CARD',
    path: '/Volumes/CARD',
    children: [
      {
        name: 'PRIVATE',
        path: '/Volumes/CARD/PRIVATE',
        children: [],
      },
      {
        name: 'DCIM',
        path: '/Volumes/CARD/DCIM',
        children: [
          {
            name: '100CANON',
            path: '/Volumes/CARD/DCIM/100CANON',
            children: [],
          },
        ],
      },
    ],
  };

  assert.deepEqual(immediateSubfolders(tree, '/Volumes/CARD'), [
    { name: 'DCIM', path: '/Volumes/CARD/DCIM' },
    { name: 'PRIVATE', path: '/Volumes/CARD/PRIVATE' },
  ]);

  assert.deepEqual(immediateSubfolders(tree, '/Volumes/CARD/DCIM/'), [
    { name: '100CANON', path: '/Volumes/CARD/DCIM/100CANON' },
  ]);

  assert.deepEqual(immediateSubfolders(tree, '/Volumes/OTHER'), []);
  assert.deepEqual(immediateSubfolders(null, '/Volumes/CARD'), []);
  assert.deepEqual(
    immediateSubfolders(
      { name: 'empty', path: '/empty', children: [] },
      '/empty',
    ),
    [],
  );
});
