import { Directory, DirectoryTree, File, Zip } from '../types/DirectoryTree';

export const extractFirstFiles = (entries: DirectoryTree[]): (File | Zip)[] => {
  const files = entries
    .filter((entry) => entry.type === 'File' || entry.type === 'Zip')
    .map((entry) => entry as File | Zip);
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
