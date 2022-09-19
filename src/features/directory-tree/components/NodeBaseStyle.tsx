import { forwardRef, ReactElement } from 'react';

type Props = {
  className?: string;
  isSelected?: boolean;
  onClick: () => void;
  children: ReactElement | ReactElement[];
};

export const NodeBaseStyle = forwardRef<HTMLDivElement, Props>(
  ({ className, isSelected, onClick, children }, ref) => {
    return (
      <div
        className={`${
          className ?? ''
        } flex h-8 cursor-pointer flex-row items-center gap-1 truncate pr-2 text-neutral-400 transition-colors hover:bg-neutral-800 hover:text-neutral-300${
          isSelected ? ' !bg-neutral-600 !text-neutral-100' : ''
        }`}
        onClick={onClick}
        ref={ref}
      >
        {children}
      </div>
    );
  }
);
