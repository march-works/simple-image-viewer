import { debounce } from '@solid-primitives/scheduled';
import { invoke } from '@tauri-apps/api';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { FileEntry, readDir } from '@tauri-apps/api/fs';
import {
  Component,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from 'solid-js';
import { PathSelection } from '../../features/DirectoryTree/routes/PathSelection';
import {
  Directory,
  DirectoryTree,
  File,
  Zip,
} from '../../features/DirectoryTree/types/DirectoryTree';
import {
  isCompressedFile,
  isImageFile,
} from '../../features/FilePath/utils/checkers.js';
import { ImageCanvas } from '../../features/Image/routes/ImageCanvas';

type Props = {
  isActiveTab: boolean;
  path: string;
  initFilePath?: string;
};

export const ViewerTab: Component<Props> = (props) => {
  const [tree, setTree] = createSignal<DirectoryTree[]>([]);
  const [currentDir, setCurrentDir] = createSignal<(File | Zip)[]>([]);
  let unListenRef: UnlistenFn | undefined = undefined;
  const [viewing, setViewing] = createSignal<number>(0);
  const [selected, setSelected] = createSignal<File | Zip>();
  const trigger = debounce((path: File | Zip) => setSelected(path), 100);

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

  const filterNonImageFiles = (tree: DirectoryTree[]): DirectoryTree[] => {
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

  const readDirAndSetTree = async () => {
    if (isCompressedFile(props.path)) {
      const files = await invoke<string[]>('get_filenames_inner_zip', {
        filepath: props.path,
      });
      setTree(() =>
        files
          .filter((file) => isImageFile(file))
          .map((file) => {
            return {
              type: 'Zip',
              name: file,
              path: props.path,
            };
          })
      );
    } else {
      const entries = await readDir(props.path, {
        recursive: true,
      });
      setTree(filterNonImageFiles(entries.map(convertEntryToTree)));
    }
  };

  const moveForward = () => {
    setViewing((prev) =>
      currentDir().length ? (prev + 1) % currentDir().length : 0
    );
  };
  const moveBackward = () => {
    setViewing((prev) =>
      currentDir().length
        ? (prev - 1 + currentDir().length) % currentDir().length
        : 0
    );
  };

  const handleOnKeyDown = (event: KeyboardEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.key === 'ArrowLeft') moveBackward();
    else if (event.key === 'ArrowRight') moveForward();
  };

  const handleOnButtonDown = (event: MouseEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.button === 3) moveBackward();
    else if (event.button === 4) moveForward();
  };

  onMount(() => {
    invoke('subscribe_dir_notification', { filepath: props.path });
    listen('directory-tree-changed', (event) => {
      if (event.payload === props.path) readDirAndSetTree();
    }).then((unListen) => (unListenRef = unListen));

    readDirAndSetTree();
    document.addEventListener('keydown', handleOnKeyDown, false);
    document.addEventListener('mouseup', handleOnButtonDown, false);
  });

  onCleanup(() => {
    unListenRef && unListenRef();
    document.removeEventListener('keydown', handleOnKeyDown, false);
    document.removeEventListener('mouseup', handleOnButtonDown, false);
  });

  const extractFirstFiles = (entries: DirectoryTree[]): (File | Zip)[] => {
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

  createEffect(() => {
    if (props.initFilePath) {
      handleOnSelectedChanged(props.initFilePath);
    } else {
      const entry = extractFirstFiles(tree());
      entry && setCurrentDir(entry);
      entry && setViewing(0);
    }
  });

  createEffect(() => {
    trigger(currentDir()[viewing()]);
  });

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
    const files = findViewingFiles(path, tree());
    files && setCurrentDir(files.files);
    files && setViewing(files.page);
  };

  return (
    <div class="flex h-full flex-row">
      <ImageCanvas
        viewing={selected()}
        moveForward={moveForward}
        moveBackward={moveBackward}
      />
      <PathSelection
        selected={currentDir()[viewing()]}
        tree={tree()}
        onSelectedChanged={handleOnSelectedChanged}
      />
    </div>
  );
};
