export interface FolderTreeNode {
  name: string;
  path: string;
  children: FolderTreeNode[];
}

export interface SubfolderEntry {
  name: string;
  path: string;
}

export function normalizeFolderPath(path: string): string {
  const unified = path.replace(/\\/g, '/').replace(/\/+$/, '');
  return unified.length === 0 ? path : unified;
}

function findNode(
  node: FolderTreeNode,
  normalizedPath: string,
): FolderTreeNode | null {
  if (normalizeFolderPath(node.path) === normalizedPath) return node;
  for (const child of node.children) {
    const found = findNode(child, normalizedPath);
    if (found) return found;
  }
  return null;
}

/** Immediate child folders of `currentPath`, or an empty list when unknown. */
export function immediateSubfolders(
  tree: FolderTreeNode | null | undefined,
  currentPath: string | null | undefined,
): SubfolderEntry[] {
  if (!tree || !currentPath) return [];
  const node = findNode(tree, normalizeFolderPath(currentPath));
  if (!node) return [];
  return node.children
    .map((child) => ({ name: child.name, path: child.path }))
    .sort((a, b) =>
      a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
    );
}
