import { Component, createEffect } from 'solid-js';
import { Image } from '../types/DirectoryTree';
import { NodeBaseStyle } from './NodeBaseStyle';
import { FaSolidImage } from 'solid-icons/fa';

type Props = {
  node: Image;
  isSelected: boolean;
  onClick?: (path: string) => void;
};

export const ImageNode: Component<Props> = (props) => {
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
      <FaSolidImage />
      <div class="hidden lg:block">{props.node.name}</div>
    </NodeBaseStyle>
  );
};
