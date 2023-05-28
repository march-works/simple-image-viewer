import { debounce } from '@solid-primitives/scheduled';
import { invoke } from '@tauri-apps/api';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { readDir } from '@tauri-apps/api/fs';
import {
  Component,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from 'solid-js';
import { PathSelection } from '../../features/DirectoryTree/routes/PathSelection';
import {
  DirectoryTree,
  File,
  Zip,
} from '../../features/DirectoryTree/types/DirectoryTree';
import { convertEntryToTree } from '../../features/DirectoryTree/utils/convertEntryToTree';
import { extractFirstFiles } from '../../features/DirectoryTree/utils/extractFirstFiles';
import { filterNonImageFiles } from '../../features/DirectoryTree/utils/filterNonImageFiles';
import { findViewingFiles } from '../../features/DirectoryTree/utils/findViewingFiles';
import {
  isCompressedFile,
  isImageFile,
} from '../../features/FilePath/utils/checkers';
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
  const [imageScale, setImageScale] = createSignal<number>(1);
  const [position, setPosition] = createSignal({ x: 0, y: 0 });

  const readDirAndSetTree = async () => {
    if (isCompressedFile(props.path)) {
      const files = await invoke<string[]>('get_filenames_inner_zip', {
        filepath: props.path,
      });
      setTree(() =>
        files
          .filter((file) => isImageFile(file))
          .sort((a, b) =>
            a.localeCompare(b, navigator.languages[0] || navigator.language, {
              numeric: true,
              ignorePunctuation: true,
            })
          )
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
    resetStatus();
  };

  const moveBackward = () => {
    setViewing((prev) =>
      currentDir().length
        ? (prev - 1 + currentDir().length) % currentDir().length
        : 0
    );
    resetStatus();
  };

  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    setImageScale((prev) =>
      Math.min(Math.max(0.1, prev + (e.deltaY > 0 ? -0.1 : 0.1)), 3)
    );
  };

  const zoomIn = () => {
    setImageScale((prev) => Math.min(Math.max(0.1, prev + 0.1), 3));
  };

  const zoomOut = () => {
    setImageScale((prev) => Math.min(Math.max(0.1, prev - 0.1), 3));
  };

  const handleOnKeyDown = (event: KeyboardEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.key === 'ArrowLeft') moveBackward();
    else if (event.key === 'ArrowRight') moveForward();
    else if (event.ctrlKey && event.key === 'i') zoomIn();
    else if (event.ctrlKey && event.key === 'o') zoomOut();
  };

  const handleOnButtonDown = (event: MouseEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.button === 3) moveBackward();
    else if (event.button === 4) moveForward();
  };

  const handlePositionChange = (newPosition: { x: number; y: number }) => {
    setPosition(newPosition);
  };

  const resetStatus = () => {
    setImageScale(1);
    setPosition({ x: 0, y: 0 });
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

  const handleOnSelectedChanged = (path: string) => {
    const files = findViewingFiles(path, tree());
    files && setCurrentDir(files.files);
    files && setViewing(files.page);
    resetStatus();
  };

  return (
    <div class="flex h-full flex-row">
      <ImageCanvas
        viewing={selected()}
        moveForward={moveForward}
        moveBackward={moveBackward}
        zoomIn={zoomIn}
        zoomOut={zoomOut}
        imageScale={imageScale()}
        position={position()}
        onPositionChange={handlePositionChange}
        handleWheel={handleWheel}
      />
      <PathSelection
        selected={currentDir()[viewing()]}
        tree={tree()}
        onSelectedChanged={handleOnSelectedChanged}
      />
    </div>
  );
};
