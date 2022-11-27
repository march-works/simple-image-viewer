import { Component, createEffect } from 'solid-js';
import { File } from '../types/DirectoryTree';
import { NodeBaseStyle } from './NodeBaseStyle';

type Props = {
  node: File;
  isSelected: boolean;
  onClick?: (path: string) => void;
};

export const FileNode: Component<Props> = (props) => {
  let nodeRef: HTMLDivElement | undefined = undefined;

  createEffect(() => {
    props.isSelected &&
      nodeRef &&
      (() => {
        nodeRef.scrollIntoView({
          behavior: 'smooth',
          block: 'center',
          inline: 'center',
        });
      })();
  });
  return (
    <NodeBaseStyle
      class="pl-3"
      ref={nodeRef}
      isSelected={props.isSelected}
      onClick={() => props.onClick && props.onClick(props.node.path)}
    >
      <i class="fa-solid fa-image" />
      <div class="hidden lg:block">{props.node.name}</div>
    </NodeBaseStyle>
  );
};
