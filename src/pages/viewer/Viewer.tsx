import { open } from '@tauri-apps/api/dialog';
import { Tabs } from '../../components/Tab/Tabs';
import { ViewerTab } from './ViewerTab';
import { ImageExtensions } from '../../features/filepath/consts/images';
import { CompressedExtensions } from '../../features/filepath/consts/compressed';
import {
  getFileNameWithoutExtension,
  getParentDirectoryName,
  getParentDirectoryPath,
} from '../../features/FilePath/utils/converters';
import { getMatches } from '@tauri-apps/api/cli';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { appWindow, WebviewWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api';
import { createSignal, onCleanup, onMount } from 'solid-js';
import { VideoExtensions } from '../../features/filepath/consts/videos';
import {
  isExecutableFile,
  isCompressedFile,
} from '../../features/filepath/utils/checkers';

type TabState = {
  title: string;
  key: string;
  path: string;
  initFilePath?: string;
}[];

const Viewer = () => {
  const [activeKey, setActiveKey] = createSignal<string>();
  const [panes, setPanes] = createSignal<TabState>([]);
  let newTabIndex = 0;
  let unListenRef: UnlistenFn | undefined = undefined;

  const onChange = (newActiveKey: string) => {
    setActiveKey(newActiveKey);
  };

  const handleOnFocus = () => {
    invoke('change_active_window');
  };

  onMount(() => {
    listen('image-file-opened', (event) => {
      createNewTab(event.payload as string);
      appWindow.setFocus();
    }).then((unListen) => (unListenRef = unListen));

    window.addEventListener('focus', handleOnFocus, false);
    handleOnFocus();

    getMatches().then((matches) => {
      const filepath = matches.args.filepath.value;
      typeof filepath === 'string' &&
        appWindow.label === 'main' &&
        createNewTab(filepath);
    });
  });

  onCleanup(() => {
    window.removeEventListener('focus', handleOnFocus, false);
    unListenRef && unListenRef();
  });

  const createNewTab = (dir: string) => {
    const newActiveKey = `newTab${newTabIndex++}`;
    setPanes((prevPanes) => {
      const newPanes = [...prevPanes];
      const title = isExecutableFile(dir)
        ? getParentDirectoryName(dir)
        : getFileNameWithoutExtension(dir);
      const path = isCompressedFile(dir) ? dir : getParentDirectoryPath(dir);
      newPanes.push({
        title: title,
        key: newActiveKey,
        path: path,
        initFilePath: isExecutableFile(dir) ? dir : undefined,
      });
      return newPanes;
    });
    setActiveKey(newActiveKey);
  };

  const add = async () => {
    const dir = await open({
      filters: [
        {
          name: 'File',
          extensions: [
            ...ImageExtensions,
            ...VideoExtensions,
            ...CompressedExtensions,
          ],
        },
      ],
    });
    if (Array.isArray(dir)) {
      return;
    }
    if (!dir) {
      return;
    }
    createNewTab(dir);
  };

  const remove = (targetKey: string) => {
    let newActiveKey = activeKey();
    let lastIndex = -1;
    panes().forEach((pane, i) => {
      if (pane.key === targetKey) {
        lastIndex = i - 1;
      }
    });
    const newPanes = panes().filter((pane) => pane.key !== targetKey);
    if (newPanes.length && newActiveKey === targetKey) {
      if (lastIndex >= 0) {
        newActiveKey = newPanes[lastIndex].key;
      } else {
        newActiveKey = newPanes[0].key;
      }
    }
    setPanes(() => newPanes);
    setActiveKey(() => newActiveKey);
  };

  const openExplorer = () => {
    new WebviewWindow('explorer', {
      url: '../../../explorer.html',
      title: 'Image Explorer',
    });
  };

  return (
    <div class="App flex h-screen w-screen select-none bg-neutral-900 text-neutral-100">
      <Tabs
        viewing={activeKey()}
        tabs={panes()}
        intoContent={(info) => (
          <ViewerTab
            isActiveTab={info.key === activeKey()}
            path={info.path}
            initFilePath={info.initFilePath}
          />
        )}
        handleOnClick={onChange}
        handleOnClose={remove}
        handleOnAdd={add}
        handleOnOpenExplorer={openExplorer}
      />
    </div>
  );
};

export default Viewer;
