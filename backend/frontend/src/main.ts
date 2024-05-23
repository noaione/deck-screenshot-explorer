import "./styles.css";

import { createApp } from "vue";
import { createPinia } from "pinia";
import { createHead } from "@unhead/vue";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";

import App from "./App.vue";
import router from "./plugins/router";

const app = createApp(App);
const head = createHead();
const pinia = createPinia();

pinia.use(piniaPluginPersistedstate);

app.use(router);
app.use(head);
app.use(pinia);

app.mount("#app");
