import type { Component, JSX } from 'solid-js';

type Props = {
  ref?: HTMLDivElement;
  class?: string;
  isSelected?: boolean;
  onClick: () => void;
  children: JSX.Element;
};

export const NodeBaseStyle: Component<Props> = (props) => {
  return (
    <div
      class={`${
        props.class ?? ''
      } flex h-8 cursor-pointer flex-row items-center gap-1 truncate pr-2 text-neutral-400 transition-colors hover:bg-neutral-800 hover:text-neutral-300${
        props.isSelected ? ' bg-neutral-600! text-neutral-100!' : ''
      }`}
      onClick={() => props.onClick()}
      ref={props.ref}
    >
      {props.children}
    </div>
  );
};
