/* @refresh reload */
import { render } from 'solid-js/web';

import '../../style.css';
import Explorer from './Explorer';

render(() => <Explorer />, document.getElementById('root') as HTMLElement);
