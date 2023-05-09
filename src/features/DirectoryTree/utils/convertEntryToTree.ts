import { FileEntry } from "@tauri-apps/api/fs";
import { DirectoryTree } from "../types/DirectoryTree";

export const convertEntryToTree = (entry: FileEntry): DirectoryTree => {
  if (entry.children === null || entry.children === undefined) {
    return {
      type: 'File',
      name: entry.name ?? '',
      path: entry.path,
    };
  }
  return {
    type: 'Directory',
    name: entry.name ?? '',
    path: entry.path,
    children: entry.children
      .map(convertEntryToTree)
      .sort((a, b) =>
        a.name.localeCompare(
          b.name,
          navigator.languages[0] || navigator.language,
          { numeric: true, ignorePunctuation: true }
        )
      ),
  };
};
