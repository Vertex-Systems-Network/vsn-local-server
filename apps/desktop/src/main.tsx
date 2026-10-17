import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { ToastProvider } from './components/Toast';
import { initTheme } from './theme/theme';
import { applyPrefs, readPrefs } from './theme/prefs';
import './styles.css';
import './accessibility.css';
initTheme();
applyPrefs(readPrefs());
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ToastProvider>
      <App />
    </ToastProvider>
  </React.StrictMode>,
);
