import { Tabs } from '../../components/Tab/Tabs';
import { ViewerTab, TabState } from './ViewerTab';
import { getMatches } from '@tauri-apps/api/cli';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { appWindow, WebviewWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api';
import { createSignal, onCleanup, onMount } from 'solid-js';

type WindowState = {
  active?: {
    key: string;
  };
  tabs: TabState[];
};

const Viewer = () => {
  const [activeKey, setActiveKey] = createSignal<string>();
  const [panes, setPanes] = createSignal<TabState[]>([]);
  let unListenRef: UnlistenFn | undefined = undefined;

  const onChange = (newActiveKey: string) => {
    invoke('change_active_tab', { key: newActiveKey, label: appWindow.label });
  };

  const handleOnFocus = () => {
    invoke('change_active_window');
  };

  onMount(() => {
    listen('window-state-changed', (event) => {
      const { active, tabs } = event.payload as WindowState;
      setPanes(tabs);
      setActiveKey(active?.key);
      appWindow.setFocus();
    }).then((unListen) => (unListenRef = unListen));

    window.addEventListener('focus', handleOnFocus, false);
    handleOnFocus();

    invoke('request_restore_state', { label: appWindow.label });

    getMatches().then((matches) => {
      const filepath = matches.args.filepath.value;
      typeof filepath === 'string' &&
        invoke('open_new_tab', { path: filepath });
    });
  });

  onCleanup(async () => {
    window.removeEventListener('focus', handleOnFocus, false);
    unListenRef && unListenRef();
  });

  const add = async () => {
    invoke('open_dialog');
  };

  const remove = (targetKey: string) => {
    invoke('remove_tab', { key: targetKey, label: appWindow.label });
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
            tabKey={info.key}
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
