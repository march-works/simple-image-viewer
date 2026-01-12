/* @refresh reload */
import { render } from 'solid-js/web';

import '../../style.css';
import Viewer from './Viewer';

render(() => <Viewer />, document.getElementById('root') as HTMLElement);
