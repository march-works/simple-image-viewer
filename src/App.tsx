import { useEffect, useRef, useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { Tabs } from './components/Tab/Tabs';
import { ViewerTab } from './pages/viewer/ViewerTab';
import { ImageExtensions } from './features/filepath/consts/images';
import { CompressedExtensions } from './features/filepath/consts/compressed';
import {
  isCompressedFile,
  isImageFile,
} from './features/filepath/utils/checkers';
import {
  getFileNameWithoutExtension,
  getParentDirectoryName,
  getParentDirectoryPath,
} from './features/filepath/utils/converters';
import { getMatches } from '@tauri-apps/api/cli';
import { BaseDirectory, readTextFile, writeTextFile } from '@tauri-apps/api/fs';

type TabState = {
  title: string;
  key: string;
  path: string;
  initFilePath?: string;
}[];

const App = () => {
  const savePath = 'tabstate.sav';
  const [activeKey, setActiveKey] = useState<string>();
  const [panes, setPanes] = useState<TabState>([]);
  const newTabIndex = useRef(0);

  const onChange = (newActiveKey: string) => {
    setActiveKey(newActiveKey);
  };

  useEffect(() => {
    readTextFile(savePath, { dir: BaseDirectory.LocalData }).then(
      (tabState) => {
        const state = JSON.parse(tabState) as { title: string; path: string }[];
        restoreTab(state);
      }
    );
    getMatches().then((matches) => {
      const filepath = matches.args.filepath.value;
      typeof filepath === 'string' && createNewTab(filepath);
    });
  }, []);

  useEffect(() => {
    writeTextFile(
      savePath,
      JSON.stringify(
        panes.map((pane) => {
          return {
            title: pane.title,
            path: pane.path,
          };
        })
      ),
      { dir: BaseDirectory.LocalData }
    );
  }, [panes]);

  const restoreTab = (dirs: { title: string; path: string }[]) => {
    const restored = dirs.map(({ title, path }) => {
      const newActiveKey = `newTab${newTabIndex.current++}`;
      return {
        title,
        key: newActiveKey,
        path,
        initFilePath: undefined,
      };
    });
    setPanes((prevPanes) => {
      const newPanes = [...prevPanes, ...restored];
      return newPanes;
    });
    restored.length && setActiveKey(restored[restored.length - 1].key);
  };

  const createNewTab = (dir: string) => {
    const newActiveKey = `newTab${newTabIndex.current++}`;
    setPanes((prevPanes) => {
      const newPanes = [...prevPanes];
      const title = isImageFile(dir)
        ? getParentDirectoryName(dir)
        : getFileNameWithoutExtension(dir);
      const path = isCompressedFile(dir) ? dir : getParentDirectoryPath(dir);
      newPanes.push({
        title: title,
        key: newActiveKey,
        path: path,
        initFilePath: isImageFile(dir) ? dir : undefined,
      });
      return newPanes;
    });
    setActiveKey(newActiveKey);
  };

  const add = async () => {
    const dir = await open({
      filters: [
        {
          name: 'Image',
          extensions: [...ImageExtensions, ...CompressedExtensions],
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
    let newActiveKey = activeKey;
    let lastIndex = -1;
    panes.forEach((pane, i) => {
      if (pane.key === targetKey) {
        lastIndex = i - 1;
      }
    });
    const newPanes = panes.filter((pane) => pane.key !== targetKey);
    if (newPanes.length && newActiveKey === targetKey) {
      if (lastIndex >= 0) {
        newActiveKey = newPanes[lastIndex].key;
      } else {
        newActiveKey = newPanes[0].key;
      }
    }
    setPanes(newPanes);
    setActiveKey(newActiveKey);
  };

  return (
    <div className="App h-screen w-screen flex bg-neutral-900 text-neutral-100 select-none">
      <Tabs
        viewing={activeKey}
        tabs={panes}
        handleOnClick={onChange}
        handleOnClose={remove}
        handleOnAdd={add}
      >
        {panes.map((pane) => (
          <ViewerTab
            key={pane.key}
            isActiveTab={pane.key === activeKey}
            path={pane.path}
            initFilePath={pane.initFilePath}
          />
        ))}
      </Tabs>
    </div>
  );
};

export default App;
