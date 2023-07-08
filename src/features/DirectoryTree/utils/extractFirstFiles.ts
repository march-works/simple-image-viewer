import { Directory, DirectoryTree, File } from '../types/DirectoryTree';

export const extractFirstFiles = (entries: DirectoryTree[]): File[] => {
  const files = entries
    .filter((entry) => entry.type !== 'Directory')
    .map((entry) => entry as File);
  if (files.length) {
    return files;
  }

  const dirs = entries
    .filter((entry) => entry.type === 'Directory')
    .map((entry) => entry as Directory);
  for (const dir of dirs) {
    const files = extractFirstFiles(dir.children);
    if (files.length) return files;
  }
  return [];
};
