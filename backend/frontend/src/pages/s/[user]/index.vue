<template>
  <HeaderView />
  <main class="flex w-full flex-col px-2 py-4">
    <div class="flex px-2">
      <RouterLink to="/" class="flex flex-row gap-2 transition-opacity hover:opacity-80">
        <i-mdi-arrow-left class="h-6 w-6" />
        Go Back
      </RouterLink>
    </div>
    <LoadingView v-if="loading" />
    <div v-else class="mt-4 flex w-full flex-col gap-2 md:flex-row md:flex-wrap md:justify-center">
      <DeckApp v-for="app in availableApps" :key="app.id" :userId="$route.params.user" :app="app" />
    </div>
  </main>
</template>

<script setup lang="ts">
import type { AppInfo } from "@/composables/use-backend-fetch";
import { watch } from "vue";

const availableApps = ref<AppInfo[]>([]);
const loading = ref(true);
const route = useRoute();

watch(
  () => route.params.user,
  async (newId, _) => {
    loading.value = true;

    const users = await useBackendFetch<AppInfo[]>(`/users/${newId}`);

    availableApps.value = users;
    loading.value = false;
  },
  {
    immediate: true,
  }
);
</script>
