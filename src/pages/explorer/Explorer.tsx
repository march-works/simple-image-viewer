import { createSignal, onCleanup } from 'solid-js';
import { ExplorerTabs } from './ExplorerTabs';
import { ExplorerTab, TabState } from './ExplorerTab';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api';

type ExplorerState = {
  active?: {
    key: string;
  };
  tabs: TabState[];
};

const Explorer = () => {
  const [activeKey, setActiveKey] = createSignal<string>();
  const [panes, setPanes] = createSignal<TabState[]>([]);
  let unListenRef: UnlistenFn | undefined = undefined;

  listen('explorer-state-changed', (event) => {
    const { active, tabs } = event.payload as ExplorerState;
    setPanes(tabs);
    setActiveKey(active?.key);
    appWindow.setFocus();
  }).then((unListen) => (unListenRef = unListen));

  invoke('request_restore_explorer_state', { label: appWindow.label });

  onCleanup(async () => {
    unListenRef && unListenRef();
  });

  const onChange = (newActiveKey: string) => {
    invoke('change_active_explorer_tab', {
      key: newActiveKey,
      label: appWindow.label,
    });
  };

  const add = async () => {
    invoke('open_new_explorer_tab', { label: appWindow.label });
  };

  const remove = (targetKey: string) => {
    invoke('remove_explorer_tab', { key: targetKey, label: appWindow.label });
  };

  return (
    <div class="App flex h-screen w-screen select-none bg-neutral-900 text-neutral-100 overflow-hidden">
      <ExplorerTabs
        viewing={activeKey()}
        tabs={panes()}
        handleOnClick={onChange}
        handleOnClose={remove}
        handleOnAdd={add}
        intoContent={(info) => (
          <ExplorerTab
            tabKey={info.key}
            path={info.path}
            transferPath={info.transferPath}
          />
        )}
      />
    </div>
  );
};

export default Explorer;
