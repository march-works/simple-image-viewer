import { CloseOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { FC, ReactElement } from 'react';

type TabInfo = {
  key: string;
  title: string;
};

type Props = {
  viewing?: string;
  tabs: TabInfo[];
  handleOnClick: (key: string) => void;
  handleOnClose: (key: string) => void;
  handleOnAdd: () => void;
  children?: ReactElement | ReactElement[];
};

export const Tabs: FC<Props> = ({
  viewing,
  tabs,
  handleOnClick,
  handleOnClose,
  handleOnAdd,
  children,
}) => {
  return (
    <div className="relative flex w-full flex-1 flex-col">
      <div className="flex h-8 w-full flex-none flex-row bg-neutral-800 align-baseline">
        {tabs.map((tab) => (
          <div
            key={tab.key}
            className={
              'flex w-48 min-w-0 flex-row justify-between rounded-t-md border-2 border-b-0 border-neutral-500 p-1 transition-colors' +
              (tab.key === viewing
                ? ' bg-gradient-to-b from-neutral-500 to-neutral-900 text-neutral-100'
                : ' bg-neutral-900 text-neutral-400 hover:bg-gradient-to-b hover:from-neutral-600 hover:to-neutral-900 hover:text-neutral-300')
            }
            onMouseDown={(e) => e.button === 1 && handleOnClose(tab.key)}
          >
            <div
              className="flex-1 self-center truncate"
              onClick={() => handleOnClick(tab.key)}
            >
              {tab.title}
            </div>
            <div
              className="flex h-5 w-5 justify-center rounded-full text-neutral-100 transition-colors hover:bg-neutral-500"
              onClick={() => handleOnClose(tab.key)}
            >
              <CloseOutlined className="text-xs" />
            </div>
          </div>
        ))}
        <div className="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300">
          <FolderOpenOutlined
            className="ml-px text-xl"
            style={{ lineHeight: '1rem', cursor: 'default' }}
            onClick={handleOnAdd}
          />
        </div>
      </div>
      <div className="relative" style={{ height: 'calc(100% - 2rem)' }}>
        {Array.isArray(children)
          ? children.map((child) => (
              <div
                key={child.key}
                className={`w-full h-full${
                  child.key === viewing ? '' : ' hidden'
                }`}
              >
                {child}
              </div>
            ))
          : children}
      </div>
    </div>
  );
};
