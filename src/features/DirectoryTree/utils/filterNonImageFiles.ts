import { isImageFile } from '../../filepath/utils/checkers';
import { DirectoryTree } from '../types/DirectoryTree';

export const filterNonImageFiles = (tree: DirectoryTree[]): DirectoryTree[] => {
  return tree
    .map((file) =>
      file.type === 'Directory'
        ? {
            ...file,
            children: filterNonImageFiles(file.children),
          }
        : file
    )
    .filter((file) => file.type === 'Directory' || isImageFile(file.name));
};
