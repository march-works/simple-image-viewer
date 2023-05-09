import { DirectoryTree, File, Zip } from '../types/DirectoryTree';

export const findViewingFiles = (
  path: string,
  dirs: DirectoryTree[]
):
  | {
      page: number;
      files: (File | Zip)[];
    }
  | undefined => {
  const validFiles = dirs.filter(
    (dir) => dir.type === 'File' || dir.type === 'Zip'
  );
  const found = validFiles.findIndex((dir) =>
    dir.type === 'File'
      ? dir.path === path
      : dir.type === 'Zip'
      ? dir.path + dir.name === path
      : false
  );
  if (found !== -1) {
    return {
      page: found,
      files: validFiles.map((dir) => dir as File | Zip),
    };
  }
  for (const dir of dirs) {
    const files =
      dir.type === 'Directory' && findViewingFiles(path, dir.children);
    if (files) return files;
  }
  return undefined;
};
