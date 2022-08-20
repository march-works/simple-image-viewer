import { invoke } from '@tauri-apps/api';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { FileEntry, readDir } from '@tauri-apps/api/fs';
import { FC, useCallback, useEffect, useRef, useState } from 'react';
import { PathSelection } from '../../features/directory-tree/routes/PathSelection';
import {
  Directory,
  DirectoryTree,
  File,
  Zip,
} from '../../features/directory-tree/types/DirectoryTree';
import { isCompressedFile } from '../../features/filepath/utils/checkers';
import { ImageCanvas } from '../../features/image/routes/ImageCanvas';
import { useDebounce } from '../../hooks/useDebounce';

type Props = {
  isActiveTab: boolean;
  path: string;
};

export const ViewerTab: FC<Props> = ({ isActiveTab, path }) => {
  const [tree, setTree] = useState<DirectoryTree[]>([]);
  const [currentDir, setCurrentDir] = useState<(File | Zip)[]>([]);
  const unlistenRef = useRef<UnlistenFn>();
  const currentDirRef = useRef<(File | Zip)[]>();
  const isActiveTabRef = useRef<boolean>();
  currentDirRef.current = currentDir;
  isActiveTabRef.current = isActiveTab;
  const [viewing, setViewing] = useState<number>(0);
  const [selected, setSelected] = useState<File | Zip>();
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
    if (isCompressedFile(path)) {
      const files = await invoke<string[]>('get_filenames_inner_zip', {
        filepath: path,
      });
      setTree(
        files.map((file) => {
          return {
            type: 'Zip',
            name: file,
            path,
          };
        })
      );
    } else {
      const entries = await readDir(path, {
        recursive: true,
      });
      setTree(entries.map(convertEntryToTree));
    }
  };

  const moveForward = () => {
    setViewing((prev) =>
      currentDirRef.current ? (prev + 1) % currentDirRef.current.length : 0
    );
  };
  const moveBackward = () => {
    setViewing((prev) =>
      currentDirRef.current
        ? (prev - 1 + currentDirRef.current.length) %
          currentDirRef.current.length
        : 0
    );
  };

  const handleOnKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!isActiveTabRef.current) return;
      if (event.key === 'ArrowLeft') moveBackward();
      else if (event.key === 'ArrowRight') moveForward();
    },
    [isActiveTab]
  );

  useEffect(() => {
    invoke('subscribe_dir_notification', { filepath: path });
    listen('directory-tree-changed', (event) => {
      if (event.payload === path) readDirAndSetTree();
    }).then((unlisten) => (unlistenRef.current = unlisten));

    readDirAndSetTree();
    document.addEventListener('keydown', handleOnKeyDown, false);
    return () => {
      unlistenRef.current && unlistenRef.current();
      document.removeEventListener('keydown', handleOnKeyDown, false);
    };
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
    setSelected(currentDir[viewing]);
  }, [currentDir, viewing]);

  const findViewingFiles = (
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

  const handleOnSelectedChanged = (path: string) => {
    const files = findViewingFiles(path, tree);
    files && setCurrentDir(files.files);
    files && setViewing(files.page);
  };

  return (
    <div className="flex flex-row bg-neutral-900 h-full">
      <ImageCanvas
        viewing={debouncedSelected}
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
