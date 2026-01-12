import { ViewerTabs } from './ViewerTabs';
import { ViewerTab, TabState } from './ViewerTab';
import { getMatches } from '@tauri-apps/plugin-cli';
import { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { createSignal, onCleanup } from 'solid-js';
const appWindow = getCurrentWebviewWindow();

type ViewerState = {
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
    invoke('change_active_viewer_tab', {
      key: newActiveKey,
      label: appWindow.label,
    });
  };

  const handleOnFocus = () => {
    invoke('change_active_viewer');
  };

  // Use appWindow.listen to only receive events targeted at this window
  appWindow
    .listen('viewer-state-changed', (event) => {
      const { active, tabs } = event.payload as ViewerState;
      setPanes(tabs);
      setActiveKey(active?.key);
      appWindow.setFocus();
    })
    .then((unListen) => (unListenRef = unListen));

  window.addEventListener('focus', handleOnFocus, false);
  handleOnFocus();

  invoke('request_restore_viewer_state', { label: appWindow.label });

  getMatches().then((matches) => {
    const filepath = matches.args.filepath?.value;
    typeof filepath === 'string' &&
      invoke('open_new_viewer_tab', { path: filepath });
  });

  onCleanup(async () => {
    window.removeEventListener('focus', handleOnFocus, false);
    unListenRef && unListenRef();
  });

  const add = async () => {
    invoke('open_image_dialog');
  };

  const remove = (targetKey: string) => {
    invoke('remove_viewer_tab', { key: targetKey, label: appWindow.label });
  };

  const openExplorer = () => {
    invoke('open_new_explorer');
  };

  return (
    <div class="App flex h-screen w-screen select-none bg-neutral-900 text-neutral-100">
      <ViewerTabs
        viewing={activeKey()}
        tabs={panes()}
        intoContent={(info) => (
          <ViewerTab
            isActiveTab={info.key === activeKey()}
            initialPath={info.path}
            initialTabKey={info.key}
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
