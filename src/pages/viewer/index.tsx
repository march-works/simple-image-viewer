/* @refresh reload */
import { Router } from '@solidjs/router';
import { render } from 'solid-js/web';

import '../../style.css';
import Viewer from './Viewer';

render(() => (
  <Router>
    <Viewer />
  </Router>
), document.getElementById('root') as HTMLElement);
