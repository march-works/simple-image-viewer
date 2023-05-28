import { FileEntry } from '@tauri-apps/api/fs';
import { isImageFile, isVideoFile } from '../../filepath/utils/checkers';
import { DirectoryTree } from '../types/DirectoryTree';

export const convertEntryToTree = (
  entry: FileEntry
): DirectoryTree | undefined => {
  if (entry.children === null || entry.children === undefined) {
    return isImageFile(entry.name ?? '')
      ? {
          type: 'Image',
          name: entry.name ?? '',
          path: entry.path,
        }
      : isVideoFile(entry.name ?? '')
      ? {
          type: 'Video',
          name: entry.name ?? '',
          path: entry.path,
        }
      : undefined;
  }
  return {
    type: 'Directory',
    name: entry.name ?? '',
    path: entry.path,
    children: entry.children
      .map(convertEntryToTree)
      .filter((v): v is DirectoryTree => !!v)
      .sort((a, b) =>
        a.name.localeCompare(
          b.name,
          navigator.languages[0] || navigator.language,
          { numeric: true, ignorePunctuation: true }
        )
      ),
  };
};
