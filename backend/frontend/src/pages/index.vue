<template>
  <HeaderView />
  <main class="flex w-full flex-col px-2">
    <LoadingView v-if="loading" />
    <div v-else class="flex w-full flex-col items-center gap-2">
      <DeckUser v-for="user in availableUsers" :key="user.id" :user="user" />
    </div>
  </main>
</template>

<script setup lang="ts">
import type { User } from "@/composables/use-backend-fetch";

const deck = useDeckStore();

const availableUsers = ref<User[]>([]);
const loading = ref(true);

onMounted(async () => {
  deck.reset();

  // Fetch available users
  const users = await useBackendFetch<User[]>("/users");

  availableUsers.value = users;
  loading.value = false;
});
</script>
