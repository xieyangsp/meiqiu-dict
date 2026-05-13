import { createApp, type Component } from 'vue';
import { createPinia } from 'pinia';
import './assets/styles.css';

// Pick which App to mount based on the ?win= query parameter.
const winName = new URLSearchParams(window.location.search).get('win') ?? 'main';

const loaders: Record<string, () => Promise<{ default: Component }>> = {
  main: () => import('./apps/MainApp.vue'),
  floater: () => import('./apps/FloaterApp.vue'),
  popup: () => import('./apps/PopupApp.vue'),
};

const load = loaders[winName] ?? loaders.main;

load().then(({ default: Root }) => {
  const app = createApp(Root);
  app.use(createPinia());
  app.mount('#app');
});
