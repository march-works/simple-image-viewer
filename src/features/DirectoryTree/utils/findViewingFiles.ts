import { match } from 'ts-pattern';
import { DirectoryTree, File } from '../types/DirectoryTree';

export const findViewingFiles = (
  path: string,
  dirs: DirectoryTree[]
):
  | {
      page: number;
      files: File[];
    }
  | undefined => {
  const files = dirs.filter((dir): dir is File => dir.type !== 'Directory');
  const found = files.findIndex((dir) =>
    match(dir)
      .with({ type: 'Image' }, () => path === dir.path)
      .with({ type: 'Video' }, () => path === dir.path)
      .with({ type: 'Zip' }, () => dir.path + dir.name === path)
      .exhaustive()
  );
  if (found !== -1) {
    return {
      page: found,
      files: files.map((dir) => dir),
    };
  }
  for (const dir of dirs) {
    const files =
      dir.type === 'Directory' && findViewingFiles(path, dir.children);
    if (files) return files;
  }
  return undefined;
};
