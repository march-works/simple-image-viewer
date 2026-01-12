import { createEffect } from 'solid-js';
import type { Component } from 'solid-js';
import type { File } from '../../../pages/viewer/ViewerTab';
import { NodeBaseStyle } from './NodeBaseStyle';
import { FaSolidImage } from 'solid-icons/fa';

type Props = {
  node: File;
  isSelected: boolean;
  onClick?: () => void;
};

export const ZipNode: Component<Props> = (props) => {
  let nodeRef!: HTMLDivElement;

  createEffect(() => {
    props.isSelected &&
      nodeRef &&
      nodeRef.scrollIntoView({
        behavior: 'smooth',
        block: 'center',
        inline: 'center',
      });
  });

  return (
    <NodeBaseStyle
      class="pl-3"
      ref={nodeRef}
      isSelected={props.isSelected}
      onClick={() => props.onClick && props.onClick()}
    >
      <FaSolidImage />
      <div class="hidden lg:block">{props.node.name}</div>
    </NodeBaseStyle>
  );
};
