import { useRef, useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { Tabs } from './components/Tab/Tabs';
import { ViewerTab } from './pages/viewer/ViewerTab';

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
      directory: true,
    });
    if (Array.isArray(dir)) {
      return;
    }
    if (!dir) {
      return;
    }

    const newActiveKey = `newTab${newTabIndex.current++}`;
    const newPanes = [...panes];
    newPanes.push({
      title: dir.split('\\').pop()?.split('/').pop() ?? '',
      key: newActiveKey,
      path: dir,
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
    <div className="App h-screen w-screen flex bg-neutral-900">
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
