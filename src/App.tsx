import { useRef, useState } from 'react';
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

const App = () => {
  const [activeKey, setActiveKey] = useState<string>();
  const [panes, setPanes] = useState<
    {
      title: string;
      key: string;
      path: string;
    }[]
  >([]);
  const newTabIndex = useRef(0);

  const onChange = (newActiveKey: string) => {
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

    const newActiveKey = `newTab${newTabIndex.current++}`;
    const newPanes = [...panes];
    const title = isImageFile(dir)
      ? getParentDirectoryName(dir)
      : getFileNameWithoutExtension(dir);
    const path = isCompressedFile(dir) ? dir : getParentDirectoryPath(dir);
    newPanes.push({
      title: title,
      key: newActiveKey,
      path: path,
    });
    setPanes(newPanes);
    setActiveKey(newActiveKey);
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
    <div className="App h-full w-full flex bg-neutral-900">
      <Tabs
        viewing={activeKey}
        tabs={panes}
        handleOnClick={onChange}
        handleOnClose={remove}
        handleOnAdd={add}
      >
        {panes.map((pane) => (
          <ViewerTab key={pane.key} path={pane.path} />
        ))}
      </Tabs>
    </div>
  );
};

export default App;
