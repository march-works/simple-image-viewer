import type { Component } from 'solid-js';

type Props = {
  size?: 'sm' | 'md' | 'lg';
};

const sizeClasses = {
  sm: 'h-8 w-8',
  md: 'h-16 w-16',
  lg: 'h-32 w-32',
};

export const Loading: Component<Props> = (props) => {
  const size = () => props.size ?? 'lg';

  return (
    <div class="flex flex-1 items-center justify-center">
      <div
        class={`animate-spin rounded-full border-b-2 border-neutral-500 ${sizeClasses[size()]}`}
      />
    </div>
  );
};
