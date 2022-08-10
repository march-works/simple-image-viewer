import { FileEntry, readDir } from '@tauri-apps/api/fs';
import { FC, useCallback, useEffect, useState } from 'react';
import { PathSelection } from '../../features/directory-tree/routes/PathSelection';
import {
  Directory,
  DirectoryTree,
  File,
} from '../../features/directory-tree/types/DirectoryTree';
import { ImageCanvas } from '../../features/image/routes/ImageCanvas';
import { useDebounce } from '../../hooks/useDebounce';

type Props = {
  path: string;
};

export const ViewerTab: FC<Props> = ({ path }) => {
  const [tree, setTree] = useState<DirectoryTree[]>([]);
  const [currentDir, setCurrentDir] = useState<File[]>([]);
  const [viewing, setViewing] = useState<number>(0);
  const [selected, setSelected] = useState<string>('');
  const debouncedSelected = useDebounce(selected, 200);

  const convertEntryToTree = (entry: FileEntry): DirectoryTree => {
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
      children: entry.children.map(convertEntryToTree),
    };
  };

  const readDirAndSetTree = async () => {
    const entries = await readDir(path, {
      recursive: true,
    });
    setTree(entries.map(convertEntryToTree));
  };

  useEffect(() => {
    readDirAndSetTree();
  }, []);

  const extractFirstFiles = useCallback((entries: DirectoryTree[]): File[] => {
    const files = entries
      .filter((entry) => entry.type === 'File')
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
  }, []);

  useEffect(() => {
    const entry = extractFirstFiles(tree);
    entry && setCurrentDir(entry);
    entry && setViewing(0);
  }, [tree, extractFirstFiles]);

  useEffect(() => {
    setSelected(currentDir[viewing]?.path ?? '');
  }, [currentDir, viewing]);

  const findViewingFiles = (
    path: string,
    dirs: DirectoryTree[]
  ):
    | {
        page: number;
        files: File[];
      }
    | undefined => {
    const found = dirs.findIndex((dir) => dir.path === path);
    if (found !== -1) {
      return {
        page: found,
        files: dirs
          .filter((dir) => dir.type === 'File')
          .map((dir) => dir as File),
      };
    }
    for (const dir of dirs) {
      const files =
        dir.type === 'Directory' && findViewingFiles(path, dir.children);
      if (files) return files;
    }
    return undefined;
  };

  const handleOnSelectedChanged = (path: string) => {
    const files = findViewingFiles(path, tree);
    files && setCurrentDir(files.files);
    files && setViewing(files.page);
  };

  const moveForward = () => {
    setViewing((prev) => (prev + 1) % currentDir.length);
  };
  const moveBackward = () => {
    setViewing((prev) => (prev - 1 + currentDir.length) % currentDir.length);
  };

  return (
    <div
      className="flex flex-row bg-neutral-900 h-full"
      tabIndex={0}
      onKeyDown={(e) =>
        e.key === 'ArrowLeft'
          ? moveBackward()
          : e.key === 'ArrowRight'
          ? moveForward()
          : undefined
      }
    >
      <ImageCanvas
        path={debouncedSelected}
        moveForward={moveForward}
        moveBackward={moveBackward}
      />
      <PathSelection
        selected={selected}
        tree={tree}
        onSelectedChanged={handleOnSelectedChanged}
      />
    </div>
  );
};
